use crate::bounded::BoundedOutcome;
use crate::buchi::{
    check_buchi_with_product_limits, BoundedBuchiResult, BuchiAutomaton, BuchiCounterexample,
    BuchiError, BuchiProductState, BuchiStatus, FiniteRunPolicy,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::fairness::{weakly_fair_cycle, WeakFairness};
use crate::graph::{
    capture_reachable_graph, induced_graph, shortest_path, ReachableGraph, SnapshotEdge,
};
use crate::model::TransitionSystem;
use crate::product::{build_action_product_with_limits, BoundedActionProduct};
use crate::recurrence::{component_is_cyclic, strongly_connected_components, RecurrenceError};
use std::collections::HashMap;
use std::hash::Hash;

/// Verify generalized Buchi acceptance under exact-action weak fairness while
/// bounding only product construction after complete model capture.
///
/// The complete model snapshot remains the authority for action enablement.
/// Therefore a product cutoff that omits an enabled fair-action edge cannot
/// fabricate a state where that action appears disabled. A retained closed
/// acceptance-avoiding cycle is conclusive only when it is weakly fair against
/// the complete model-side enablement relation. If no such witness is retained,
/// an incomplete product remains `INCONCLUSIVE`.
///
/// An empty fairness set delegates to the existing bounded Buchi backend and is
/// therefore an exact compatibility path, including deterministic witnesses and
/// accounting.
pub fn check_buchi_with_weak_fairness_and_product_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedBuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_buchi_with_product_limits(model, automaton, limits);
    }

    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let BoundedActionProduct {
        graph: product,
        checked_states,
        explored_transitions,
        max_depth_reached,
        completion,
        known_terminal,
    } = build_action_product_with_limits(
        &captured.graph,
        automaton.initial(),
        |state, action| automaton.advance(state, action),
        |state, automaton| BuchiProductState { state, automaton },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let enablement = complete_model_enablement_graph(&captured.graph, &product)?;
    let counterexample = find_bounded_fair_counterexample(
        &enablement,
        &product,
        &known_terminal,
        automaton,
        fairness,
    )?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    } else {
        match completion {
            BoundedOutcome::Conclusive(()) => BoundedOutcome::Conclusive(BuchiStatus::Satisfied),
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedBuchiResult {
        automaton: automaton.name().to_owned(),
        outcome,
        finite_policy: automaton.finite_policy(),
        acceptance_sets: automaton.acceptance_sets().len(),
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        checked_product_states: checked_states,
        explored_product_transitions: explored_transitions,
        retained_product_transitions,
        max_product_depth_reached: max_depth_reached,
        counterexample,
    })
}

/// Build an action-enable snapshot with exactly the product state's id space.
/// Targets are intentionally self references: only action-label presence is
/// consumed by `weakly_fair_cycle`; recurrent taken edges still come from the
/// retained product residual itself.
fn complete_model_enablement_graph<S, A>(
    model_graph: &ReachableGraph<S>,
    product: &ReachableGraph<BuchiProductState<S, A>>,
) -> Result<ReachableGraph<BuchiProductState<S, A>>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone,
{
    let model_ids = model_graph
        .states
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, state)| (state, id))
        .collect::<HashMap<_, _>>();

    let outgoing = product
        .states
        .iter()
        .enumerate()
        .map(|(product_id, state)| {
            let model_id = model_ids
                .get(&state.state)
                .copied()
                .ok_or(BuchiError::MissingWitness)?;
            Ok(model_graph.outgoing[model_id]
                .iter()
                .map(|edge| SnapshotEdge {
                    action: edge.action.clone(),
                    target: product_id,
                })
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>, BuchiError>>()?;

    Ok(ReachableGraph {
        states: product.states.clone(),
        outgoing,
        initial_ids: product.initial_ids.clone(),
    })
}

fn find_bounded_fair_counterexample<S, A>(
    enablement: &ReachableGraph<BuchiProductState<S, A>>,
    product: &ReachableGraph<BuchiProductState<S, A>>,
    known_terminal: &[bool],
    automaton: &BuchiAutomaton<A>,
    fairness: &WeakFairness,
) -> Result<Option<BuchiCounterexample<S, A>>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if automaton.finite_policy() == FiniteRunPolicy::RequireAcceptingTerminal {
        for (product_id, state) in product.states.iter().enumerate() {
            if !known_terminal[product_id] {
                continue;
            }
            let Some(set) = automaton
                .acceptance_sets()
                .iter()
                .find(|set| !set.contains(&state.automaton))
            else {
                continue;
            };
            let trace = shortest_path(product, &product.initial_ids, product_id, None)
                .ok_or(BuchiError::MissingWitness)?;
            return Ok(Some(BuchiCounterexample::FiniteTerminal {
                missing_acceptance: set.name().to_owned(),
                trace,
            }));
        }
    }

    let mut best: Option<BoundedFairCandidate<S, A>> = None;
    for (acceptance_index, set) in automaton.acceptance_sets().iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| !set.contains(&state.automaton))
            .collect::<Vec<_>>();
        let old_ids = included
            .iter()
            .enumerate()
            .filter_map(|(id, included)| included.then_some(id))
            .collect::<Vec<_>>();
        if old_ids.is_empty() {
            continue;
        }

        let residual = induced_graph(product, &included);
        for component in strongly_connected_components(&residual) {
            if !component_is_cyclic(&residual, &component) {
                continue;
            }
            let Some(cycle) = weakly_fair_cycle(
                enablement,
                &residual,
                &old_ids,
                &component,
                fairness,
            )?
            else {
                continue;
            };
            let entry = *component.first().ok_or(BuchiError::MissingWitness)?;
            let product_entry = old_ids[entry];
            let stem = shortest_path(product, &product.initial_ids, product_entry, None)
                .ok_or(BuchiError::MissingWitness)?;
            let candidate = BoundedFairCandidate {
                acceptance_index,
                product_entry,
                stem,
                cycle,
            };
            if best.as_ref().is_none_or(|current| {
                bounded_fair_candidate_key(&candidate) < bounded_fair_candidate_key(current)
            }) {
                best = Some(candidate);
            }
        }
    }

    Ok(
        best.map(|candidate| BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance: automaton.acceptance_sets()[candidate.acceptance_index]
                .name()
                .to_owned(),
            stem: candidate.stem,
            cycle: candidate.cycle,
        }),
    )
}

struct BoundedFairCandidate<S, A> {
    acceptance_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<BuchiProductState<S, A>>>,
    cycle: Vec<TraceStep<BuchiProductState<S, A>>>,
}

fn bounded_fair_candidate_key<S, A>(
    candidate: &BoundedFairCandidate<S, A>,
) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.acceptance_index,
        candidate.product_entry,
    )
}

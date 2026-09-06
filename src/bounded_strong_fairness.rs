use crate::bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
use crate::buchi::{
    check_buchi_with_limits, check_buchi_with_product_limits, AnalysisBuchiResult,
    BoundedBuchiResult, BuchiAutomaton, BuchiError, BuchiProductState, BuchiStatus,
};
use crate::checker::ExplorationLimits;
use crate::fair_enablement::{
    bounded_enablement_graph_for_actions, complete_enablement_graph,
};
use crate::graph::{
    capture_reachable_graph, capture_reachable_graph_with_limits, GraphCaptureCompletion,
};
use crate::model::TransitionSystem;
use crate::product::{
    build_action_product_from_prefix_with_limits, build_action_product_with_limits,
    BoundedActionProduct,
};
use crate::recurrence::RecurrenceError;
use crate::strong_fairness::{find_strong_fair_buchi_counterexample, StrongFairness};
use std::hash::Hash;

/// Verify generalized Büchi acceptance under exact-action strong fairness while
/// bounding only product construction after complete model capture.
///
/// Complete model capture remains the authority for action enablement. A
/// product cutoff can therefore remove executable product edges without turning
/// those missing edges into evidence that a strong-fair action is disabled.
/// A retained acceptance-avoiding recurrent witness is conclusive only when its
/// repeated component satisfies every strong-fair enabled/taken pair against
/// that complete enablement authority. Without such a witness, an incomplete
/// product remains `INCONCLUSIVE`.
///
/// Empty strong fairness delegates exactly to the historical bounded Büchi
/// backend, including accounting and deterministic witnesses.
pub fn check_buchi_with_strong_fairness_and_product_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    fairness: &StrongFairness,
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
    let enablement = complete_enablement_graph(&captured.graph, &product, |state| &state.state)
        .ok_or(BuchiError::MissingWitness)?;
    let counterexample = find_strong_fair_buchi_counterexample(
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

/// Verify generalized Büchi acceptance under exact-action strong fairness with
/// independent deterministic model-capture and product-construction budgets.
///
/// For a model state whose complete successor vector was captured, exact action
/// enablement is projected into product state ids. If a model cutoff leaves that
/// successor vector unknown, every configured strong-fair action is
/// conservatively represented as enabled. Such uncertainty can therefore never
/// be misread as a disabled-action fact. A conclusive recurrent counterexample
/// still needs real retained product edges that take every strong-fair action
/// enabled on its repeated component.
///
/// Finite terminal counterexamples keep their historical precedence. If no
/// conclusive violation survives, model-stage incompleteness takes precedence
/// over product-stage incompleteness, matching the existing staged analysis
/// contract. Empty strong fairness delegates exactly to historical staged
/// Büchi verification.
pub fn check_buchi_with_strong_fairness_and_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    fairness: &StrongFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisBuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_buchi_with_limits(model, automaton, limits);
    }

    let captured =
        capture_reachable_graph_with_limits(model, limits.model).map_err(RecurrenceError::from)?;
    let model_retained_transitions = captured.graph.outgoing.iter().map(Vec::len).sum();
    let model_completion = match captured.completion {
        GraphCaptureCompletion::Complete => BoundedOutcome::Conclusive(()),
        GraphCaptureCompletion::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };
    let BoundedActionProduct {
        graph: product,
        checked_states: checked_product_states,
        explored_transitions: explored_product_transitions,
        max_depth_reached: max_product_depth_reached,
        completion: product_completion,
        known_terminal,
    } = build_action_product_from_prefix_with_limits(
        &captured.graph,
        &captured.known_terminal,
        automaton.initial(),
        |state, action| automaton.advance(state, action),
        |state, automaton| BuchiProductState { state, automaton },
        limits.product,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let enablement = bounded_enablement_graph_for_actions(
        &captured.graph,
        &captured.complete_enabled_actions,
        &product,
        fairness.actions(),
        |state| &state.state,
    )
    .ok_or(BuchiError::MissingWitness)?;
    let counterexample = find_strong_fair_buchi_counterexample(
        &enablement,
        &product,
        &known_terminal,
        automaton,
        fairness,
    )?;
    let outcome = staged_strong_fair_outcome(
        counterexample.is_some(),
        &model_completion,
        &product_completion,
    );

    Ok(AnalysisBuchiResult {
        automaton: automaton.name().to_owned(),
        outcome,
        finite_policy: automaton.finite_policy(),
        acceptance_sets: automaton.acceptance_sets().len(),
        model_completion,
        product_completion,
        model_states: captured.discovered_states,
        checked_model_states: captured.checked_states,
        explored_model_transitions: captured.explored_transitions,
        retained_model_transitions: model_retained_transitions,
        max_model_depth_reached: captured.max_depth_reached,
        product_states: product.states.len(),
        checked_product_states,
        explored_product_transitions,
        retained_product_transitions,
        max_product_depth_reached,
        counterexample,
    })
}

fn staged_strong_fair_outcome(
    violated: bool,
    model_completion: &BoundedOutcome<()>,
    product_completion: &BoundedOutcome<()>,
) -> AnalysisOutcome<BuchiStatus> {
    if violated {
        return AnalysisOutcome::Conclusive(BuchiStatus::Violated);
    }
    if let BoundedOutcome::Inconclusive(reason) = model_completion {
        return AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: *reason,
        });
    }
    if let BoundedOutcome::Inconclusive(reason) = product_completion {
        return AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Product,
            reason: *reason,
        });
    }
    AnalysisOutcome::Conclusive(BuchiStatus::Satisfied)
}

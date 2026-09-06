use crate::buchi::{
    BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState, BuchiResult, BuchiStatus,
    FiniteRunPolicy,
};
use crate::checker::TraceStep;
use crate::graph::{capture_reachable_graph, induced_graph, shortest_path, ReachableGraph};
use crate::model::TransitionSystem;
use crate::product::build_action_product;
use crate::recurrence::{component_is_cyclic, strongly_connected_components, RecurrenceError};
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;

/// Exact-action strong-fairness assumptions for infinite executions.
///
/// For every configured action `a`, an admitted infinite execution must take
/// `a` infinitely often whenever `a` is enabled infinitely often. This is
/// strictly stronger than weak fairness: intermittent enablement still creates
/// an obligation when it recurs forever.
///
/// The empty set preserves the repository's historical no-fairness semantics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StrongFairness {
    actions: Vec<String>,
}

impl StrongFairness {
    pub fn new<I, T>(actions: I) -> Result<Self, StrongFairnessError>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();
        for action in actions {
            let action = action.into();
            if action.trim().is_empty() {
                return Err(StrongFairnessError::EmptyActionName);
            }
            if !seen.insert(action.clone()) {
                return Err(StrongFairnessError::DuplicateAction { action });
            }
            ordered.push(action);
        }
        Ok(Self { actions: ordered })
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrongFairnessError {
    EmptyActionName,
    DuplicateAction { action: String },
}

impl fmt::Display for StrongFairnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActionName => write!(f, "strong-fair action name must not be empty"),
            Self::DuplicateAction { action } => {
                write!(f, "duplicate strong-fair action '{action}'")
            }
        }
    }
}

impl std::error::Error for StrongFairnessError {}

/// Universally verify one generalized Büchi automaton while quantifying only
/// over infinite executions that satisfy the configured exact-action strong
/// fairness assumptions. Finite terminal policy is unchanged by fairness.
///
/// For a strong-fair action `a`, recurrent admissibility is the Streett pair
/// `(enabled(a), taken(a))`: if a recurrent set visits any state enabling `a`,
/// the repeated witness must contain an internal `a` edge. A candidate SCC that
/// has enabled states but no such edge is not rejected wholesale; those enabled
/// states are removed and SCC decomposition is repeated so a smaller fair
/// recurrent subset can survive.
pub fn check_buchi_with_strong_fairness<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    fairness: &StrongFairness,
) -> Result<BuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return crate::buchi::check_buchi(model, automaton);
    }

    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let product = build_action_product(
        &captured.graph,
        automaton.initial(),
        |state, action| automaton.advance(state, action),
        |state, automaton| BuchiProductState { state, automaton },
    );
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let known_terminal = product
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let counterexample =
        find_strong_fair_buchi_counterexample(&product, &known_terminal, automaton, fairness)?;

    Ok(BuchiResult {
        automaton: automaton.name().to_owned(),
        status: if counterexample.is_some() {
            BuchiStatus::Violated
        } else {
            BuchiStatus::Satisfied
        },
        finite_policy: automaton.finite_policy(),
        acceptance_sets: automaton.acceptance_sets().len(),
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample,
    })
}

fn find_strong_fair_buchi_counterexample<S, A>(
    product: &ReachableGraph<BuchiProductState<S, A>>,
    known_terminal: &[bool],
    automaton: &BuchiAutomaton<A>,
    fairness: &StrongFairness,
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

    let mut best: Option<StrongFairCandidate<S, A>> = None;
    for (acceptance_index, set) in automaton.acceptance_sets().iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| !set.contains(&state.automaton))
            .collect::<Vec<_>>();
        let residual_to_product = included
            .iter()
            .enumerate()
            .filter_map(|(id, included)| included.then_some(id))
            .collect::<Vec<_>>();
        if residual_to_product.is_empty() {
            continue;
        }

        let residual = induced_graph(product, &included);
        for component in
            strongly_fair_components(product, &residual, &residual_to_product, fairness)
        {
            let Some((entry, product_entry, stem)) =
                nearest_component_entry(product, &residual_to_product, &component)
            else {
                return Err(BuchiError::MissingWitness);
            };
            let cycle = strong_fair_cycle(
                product,
                &residual,
                &residual_to_product,
                &component,
                entry,
                fairness,
            )?;
            let candidate = StrongFairCandidate {
                acceptance_index,
                product_entry,
                stem,
                cycle,
            };
            if best.as_ref().is_none_or(|current| {
                strong_fair_candidate_key(&candidate) < strong_fair_candidate_key(current)
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

struct StrongFairCandidate<S, A> {
    acceptance_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<BuchiProductState<S, A>>>,
    cycle: Vec<TraceStep<BuchiProductState<S, A>>>,
}

fn strong_fair_candidate_key<S, A>(candidate: &StrongFairCandidate<S, A>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.acceptance_index,
        candidate.product_entry,
    )
}

/// Return every cyclic recurrent component that survives all strong-fair
/// Streett pairs. Component ids refer to `residual`.
fn strongly_fair_components<S>(
    full_graph: &ReachableGraph<S>,
    residual: &ReachableGraph<S>,
    residual_to_full: &[usize],
    fairness: &StrongFairness,
) -> Vec<Vec<usize>>
where
    S: Clone,
{
    let mut pending = VecDeque::new();
    for component in strongly_connected_components(residual) {
        if component_is_cyclic(residual, &component) {
            pending.push_back(component);
        }
    }

    let mut fair = Vec::new();
    while let Some(component) = pending.pop_front() {
        let members = component.iter().copied().collect::<HashSet<_>>();
        let mut split = false;

        for action in fairness.actions() {
            let enabled = component
                .iter()
                .copied()
                .filter(|node| {
                    full_graph.outgoing[residual_to_full[*node]]
                        .iter()
                        .any(|edge| edge.action == *action)
                })
                .collect::<Vec<_>>();
            if enabled.is_empty() {
                continue;
            }

            let internal_take = component.iter().copied().any(|source| {
                residual.outgoing[source]
                    .iter()
                    .any(|edge| edge.action == *action && members.contains(&edge.target))
            });
            if internal_take {
                continue;
            }

            let removed = enabled.into_iter().collect::<HashSet<_>>();
            for subcomponent in cyclic_subcomponents_after_removal(residual, &component, &removed) {
                pending.push_back(subcomponent);
            }
            split = true;
            break;
        }

        if !split {
            fair.push(component);
        }
    }

    fair.sort_by_key(|component| component.first().copied().unwrap_or(usize::MAX));
    fair
}

fn cyclic_subcomponents_after_removal<S: Clone>(
    residual: &ReachableGraph<S>,
    component: &[usize],
    removed: &HashSet<usize>,
) -> Vec<Vec<usize>> {
    let component_members = component.iter().copied().collect::<HashSet<_>>();
    let included = (0..residual.states.len())
        .map(|node| component_members.contains(&node) && !removed.contains(&node))
        .collect::<Vec<_>>();
    let dense_to_residual = included
        .iter()
        .enumerate()
        .filter_map(|(id, included)| included.then_some(id))
        .collect::<Vec<_>>();
    if dense_to_residual.is_empty() {
        return Vec::new();
    }

    let dense = induced_graph(residual, &included);
    strongly_connected_components(&dense)
        .into_iter()
        .filter(|candidate| component_is_cyclic(&dense, candidate))
        .map(|candidate| {
            candidate
                .into_iter()
                .map(|dense_id| dense_to_residual[dense_id])
                .collect::<Vec<_>>()
        })
        .collect()
}

fn nearest_component_entry<S: Clone>(
    full_graph: &ReachableGraph<S>,
    residual_to_full: &[usize],
    component: &[usize],
) -> Option<(usize, usize, Vec<TraceStep<S>>)> {
    component
        .iter()
        .copied()
        .filter_map(|entry| {
            let full_entry = residual_to_full[entry];
            shortest_path(full_graph, &full_graph.initial_ids, full_entry, None).map(|stem| {
                let distance = stem.len().saturating_sub(1);
                (distance, full_entry, entry, stem)
            })
        })
        .min_by_key(|(distance, full_entry, _, _)| (*distance, *full_entry))
        .map(|(_, full_entry, entry, stem)| (entry, full_entry, stem))
}

fn strong_fair_cycle<S>(
    full_graph: &ReachableGraph<S>,
    residual: &ReachableGraph<S>,
    residual_to_full: &[usize],
    component: &[usize],
    entry: usize,
    fairness: &StrongFairness,
) -> Result<Vec<TraceStep<S>>, RecurrenceError>
where
    S: Clone + Eq,
{
    let members = component.iter().copied().collect::<HashSet<_>>();
    if !members.contains(&entry) || residual_to_full.len() != residual.states.len() {
        return Err(RecurrenceError::CycleWitnessMissing);
    }

    let mut cycle = vec![TraceStep {
        action: None,
        state: residual.states[entry].clone(),
    }];
    let mut current = entry;

    for action in fairness.actions() {
        let enabled = component.iter().copied().any(|node| {
            full_graph.outgoing[residual_to_full[node]]
                .iter()
                .any(|edge| edge.action == *action)
        });
        if !enabled {
            continue;
        }

        let internal = component.iter().copied().find_map(|source| {
            residual.outgoing[source]
                .iter()
                .find(|edge| edge.action == *action && members.contains(&edge.target))
                .map(|edge| (source, edge))
        });
        let Some((source, edge)) = internal else {
            return Err(RecurrenceError::CycleWitnessMissing);
        };

        append_path(residual, &members, current, source, &mut cycle)?;
        cycle.push(TraceStep {
            action: Some(edge.action.clone()),
            state: residual.states[edge.target].clone(),
        });
        current = edge.target;
    }

    if cycle.len() == 1 {
        let edge = residual.outgoing[entry]
            .iter()
            .find(|edge| members.contains(&edge.target))
            .ok_or(RecurrenceError::CycleWitnessMissing)?;
        cycle.push(TraceStep {
            action: Some(edge.action.clone()),
            state: residual.states[edge.target].clone(),
        });
        current = edge.target;
    }

    append_path(residual, &members, current, entry, &mut cycle)?;
    if cycle.len() < 2
        || cycle.first().map(|step| &step.state) != cycle.last().map(|step| &step.state)
    {
        return Err(RecurrenceError::CycleWitnessMissing);
    }
    Ok(cycle)
}

fn append_path<S: Clone>(
    graph: &ReachableGraph<S>,
    members: &HashSet<usize>,
    from: usize,
    to: usize,
    output: &mut Vec<TraceStep<S>>,
) -> Result<(), RecurrenceError> {
    let path = shortest_path(graph, &[from], to, Some(members))
        .ok_or(RecurrenceError::CycleWitnessMissing)?;
    output.extend(path.into_iter().skip(1));
    Ok(())
}

use crate::buchi::{
    BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState, BuchiResult, BuchiStatus,
    FiniteRunPolicy,
};
use crate::checker::TraceStep;
use crate::fairness::{check_buchi_with_weak_fairness, FairnessError, WeakFairness};
use crate::graph::{capture_reachable_graph, induced_graph, shortest_path, ReachableGraph};
use crate::model::TransitionSystem;
use crate::product::build_action_product;
use crate::recurrence::{component_is_cyclic, strongly_connected_components, RecurrenceError};
use crate::strong_fairness::{
    check_buchi_with_strong_fairness, StrongFairness, StrongFairnessError,
};
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;

/// One validated profile containing exact-action weak- and strong-fairness
/// assumptions for infinite executions.
///
/// Ordering is preserved independently inside each class. If an action appears
/// in both classes, the stronger obligation subsumes the weak one, so the
/// canonical profile keeps that action only in `strong` while preserving the
/// relative order of every remaining weak action. Finite executions are never
/// filtered by this profile.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FairnessProfile {
    weak: WeakFairness,
    strong: StrongFairness,
}

impl FairnessProfile {
    pub fn new<WI, WT, SI, ST>(
        weak_actions: WI,
        strong_actions: SI,
    ) -> Result<Self, FairnessProfileError>
    where
        WI: IntoIterator<Item = WT>,
        WT: Into<String>,
        SI: IntoIterator<Item = ST>,
        ST: Into<String>,
    {
        let weak = WeakFairness::new(weak_actions).map_err(FairnessProfileError::Weak)?;
        let strong =
            StrongFairness::new(strong_actions).map_err(FairnessProfileError::Strong)?;
        let weak_only = weak
            .actions()
            .iter()
            .filter(|action| !strong.actions().contains(*action))
            .cloned()
            .collect::<Vec<_>>();
        let weak = WeakFairness::new(weak_only).map_err(FairnessProfileError::Weak)?;
        Ok(Self { weak, strong })
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn weak(&self) -> &WeakFairness {
        &self.weak
    }

    pub fn strong(&self) -> &StrongFairness {
        &self.strong
    }

    pub fn weak_actions(&self) -> &[String] {
        self.weak.actions()
    }

    pub fn strong_actions(&self) -> &[String] {
        self.strong.actions()
    }

    pub fn is_empty(&self) -> bool {
        self.weak.is_empty() && self.strong.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FairnessProfileError {
    Weak(FairnessError),
    Strong(StrongFairnessError),
}

impl fmt::Display for FairnessProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Weak(error) => write!(f, "invalid weak-fair assumption: {error}"),
            Self::Strong(error) => write!(f, "invalid strong-fair assumption: {error}"),
        }
    }
}

impl std::error::Error for FairnessProfileError {}

impl From<FairnessError> for FairnessProfileError {
    fn from(value: FairnessError) -> Self {
        Self::Weak(value)
    }
}

impl From<StrongFairnessError> for FairnessProfileError {
    fn from(value: StrongFairnessError) -> Self {
        Self::Strong(value)
    }
}

/// Universally verify one generalized Büchi automaton while admitting only
/// infinite executions satisfying every weak and strong obligation in one
/// canonical fairness profile.
///
/// Compatibility paths are exact: an empty profile delegates to the historical
/// no-fair backend, weak-only profiles delegate to the sealed weak-fair backend,
/// and strong-only profiles delegate to the sealed strong-fair backend. Mixed
/// profiles use the same complete model capture, deterministic action product,
/// SCC substrate, and finite-terminal precedence as those authorities.
pub fn check_buchi_with_fairness_profile<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    profile: &FairnessProfile,
) -> Result<BuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if profile.is_empty() {
        return crate::buchi::check_buchi(model, automaton);
    }
    if profile.strong.is_empty() {
        return check_buchi_with_weak_fairness(model, automaton, &profile.weak);
    }
    if profile.weak.is_empty() {
        return check_buchi_with_strong_fairness(model, automaton, &profile.strong);
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
    let counterexample = find_profile_buchi_counterexample(
        &product,
        &product,
        &known_terminal,
        automaton,
        profile,
    )?;

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

/// Find one finite or combined-fair infinite Büchi counterexample.
///
/// `enablement` and `product` share state ids. The distinction is retained so
/// later bounded/staged composition can provide conservative complete-model
/// enablement while witness edges remain restricted to a justified product
/// prefix. M45 itself invokes this over the complete product.
pub(crate) fn find_profile_buchi_counterexample<S, A>(
    enablement: &ReachableGraph<BuchiProductState<S, A>>,
    product: &ReachableGraph<BuchiProductState<S, A>>,
    known_terminal: &[bool],
    automaton: &BuchiAutomaton<A>,
    profile: &FairnessProfile,
) -> Result<Option<BuchiCounterexample<S, A>>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if enablement.states.len() != product.states.len()
        || known_terminal.len() != product.states.len()
    {
        return Err(BuchiError::MissingWitness);
    }

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

    let mut best: Option<ProfileCandidate<S, A>> = None;
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
        for component in strong_fair_components(
            enablement,
            &residual,
            &residual_to_product,
            &profile.strong,
        ) {
            if !weak_component_is_admissible(
                enablement,
                &residual,
                &residual_to_product,
                &component,
                &profile.weak,
            ) {
                continue;
            }
            let Some((entry, product_entry, stem)) =
                nearest_component_entry(product, &residual_to_product, &component)
            else {
                return Err(BuchiError::MissingWitness);
            };
            let cycle = combined_fair_cycle(
                enablement,
                &residual,
                &residual_to_product,
                &component,
                entry,
                profile,
            )?;
            let candidate = ProfileCandidate {
                acceptance_index,
                product_entry,
                stem,
                cycle,
            };
            if best.as_ref().is_none_or(|current| {
                profile_candidate_key(&candidate) < profile_candidate_key(current)
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

struct ProfileCandidate<S, A> {
    acceptance_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<BuchiProductState<S, A>>>,
    cycle: Vec<TraceStep<BuchiProductState<S, A>>>,
}

fn profile_candidate_key<S, A>(candidate: &ProfileCandidate<S, A>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.acceptance_index,
        candidate.product_entry,
    )
}

/// Strong fairness is Streett-style: if an action is enabled at any state that
/// recurs infinitely often, the recurrent execution must take that action.
/// When a candidate component has enabled states but no internal take edge,
/// those enabled states are removed and SCC decomposition repeats so a smaller
/// recurrent subset can still survive.
fn strong_fair_components<S>(
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

fn weak_component_is_admissible<S>(
    full_graph: &ReachableGraph<S>,
    residual: &ReachableGraph<S>,
    residual_to_full: &[usize],
    component: &[usize],
    fairness: &WeakFairness,
) -> bool {
    let members = component.iter().copied().collect::<HashSet<_>>();
    fairness.actions().iter().all(|action| {
        let some_disabled = component.iter().copied().any(|node| {
            !full_graph.outgoing[residual_to_full[node]]
                .iter()
                .any(|edge| edge.action == *action)
        });
        let internal_take = component.iter().copied().any(|source| {
            residual.outgoing[source]
                .iter()
                .any(|edge| edge.action == *action && members.contains(&edge.target))
        });
        some_disabled || internal_take
    })
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

/// Construct one deterministic closed walk satisfying every obligation in the
/// already-admissible recurrent component. Repeating this walk forever gives a
/// concrete mixed-fair execution: each weak action either visits a disabling
/// state or is taken, while each recurrently enabled strong action is taken.
fn combined_fair_cycle<S>(
    full_graph: &ReachableGraph<S>,
    residual: &ReachableGraph<S>,
    residual_to_full: &[usize],
    component: &[usize],
    entry: usize,
    profile: &FairnessProfile,
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

    for action in profile.weak.actions() {
        if let Some(disabled) = component.iter().copied().find(|node| {
            !full_graph.outgoing[residual_to_full[*node]]
                .iter()
                .any(|edge| edge.action == *action)
        }) {
            append_path(residual, &members, current, disabled, &mut cycle)?;
            current = disabled;
            continue;
        }

        let Some((source, edge)) = component.iter().copied().find_map(|source| {
            residual.outgoing[source]
                .iter()
                .find(|edge| edge.action == *action && members.contains(&edge.target))
                .map(|edge| (source, edge))
        }) else {
            return Err(RecurrenceError::CycleWitnessMissing);
        };
        append_path(residual, &members, current, source, &mut cycle)?;
        cycle.push(TraceStep {
            action: Some(edge.action.clone()),
            state: residual.states[edge.target].clone(),
        });
        current = edge.target;
    }

    for action in profile.strong.actions() {
        let enabled = component.iter().copied().any(|node| {
            full_graph.outgoing[residual_to_full[node]]
                .iter()
                .any(|edge| edge.action == *action)
        });
        if !enabled {
            continue;
        }

        let Some((source, edge)) = component.iter().copied().find_map(|source| {
            residual.outgoing[source]
                .iter()
                .find(|edge| edge.action == *action && members.contains(&edge.target))
                .map(|edge| (source, edge))
        }) else {
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

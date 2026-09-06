use crate::bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::graph::{capture_reachable_graph, induced_graph, shortest_path, ReachableGraph};
use crate::model::TransitionSystem;
use crate::product::{
    build_action_product, build_action_product_with_analysis_limits,
    build_action_product_with_limits, BoundedActionProduct, StagedActionProduct,
};
use crate::recurrence::{
    component_is_cyclic, cycle_witness, strongly_connected_components, RecurrenceError,
};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

type AutomatonStep<A> = Arc<dyn Fn(&A, &str) -> A + Send + Sync>;
type AutomatonPredicate<A> = Arc<dyn Fn(&A) -> bool + Send + Sync>;

/// One named generalized Büchi acceptance set over automaton states.
pub struct AcceptanceSet<A> {
    name: String,
    predicate: AutomatonPredicate<A>,
}

impl<A> Clone for AcceptanceSet<A> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            predicate: Arc::clone(&self.predicate),
        }
    }
}

impl<A> fmt::Debug for AcceptanceSet<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AcceptanceSet")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<A> AcceptanceSet<A> {
    pub fn new(
        name: impl Into<String>,
        predicate: impl Fn(&A) -> bool + Send + Sync + 'static,
    ) -> Result<Self, BuchiError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(BuchiError::EmptyAcceptanceName);
        }
        Ok(Self {
            name,
            predicate: Arc::new(predicate),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Policy for finite maximal model executions.
///
/// Büchi acceptance is inherently an infinite-run condition, so finite paths
/// must not be assigned an implicit meaning. `IgnoreTerminals` checks only
/// infinite executions. `RequireAcceptingTerminal` additionally requires every
/// finite product terminal to satisfy every named acceptance set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteRunPolicy {
    IgnoreTerminals,
    RequireAcceptingTerminal,
}

/// A deterministic finite automaton equipped with generalized Büchi acceptance
/// sets and an explicit finite-run policy.
pub struct BuchiAutomaton<A> {
    name: String,
    initial: A,
    step: AutomatonStep<A>,
    acceptance: Vec<AcceptanceSet<A>>,
    finite_policy: FiniteRunPolicy,
}

impl<A: Clone> Clone for BuchiAutomaton<A> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            initial: self.initial.clone(),
            step: Arc::clone(&self.step),
            acceptance: self.acceptance.clone(),
            finite_policy: self.finite_policy,
        }
    }
}

impl<A: fmt::Debug> fmt::Debug for BuchiAutomaton<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuchiAutomaton")
            .field("name", &self.name)
            .field("initial", &self.initial)
            .field(
                "acceptance",
                &self
                    .acceptance
                    .iter()
                    .map(|set| &set.name)
                    .collect::<Vec<_>>(),
            )
            .field("finite_policy", &self.finite_policy)
            .finish_non_exhaustive()
    }
}

impl<A> BuchiAutomaton<A> {
    pub fn new(
        name: impl Into<String>,
        initial: A,
        step: impl Fn(&A, &str) -> A + Send + Sync + 'static,
        acceptance: Vec<AcceptanceSet<A>>,
        finite_policy: FiniteRunPolicy,
    ) -> Result<Self, BuchiError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(BuchiError::EmptyAutomatonName);
        }
        if acceptance.is_empty() {
            return Err(BuchiError::NoAcceptanceSets);
        }

        let mut names = HashSet::new();
        for set in &acceptance {
            if !names.insert(set.name.clone()) {
                return Err(BuchiError::DuplicateAcceptanceName {
                    name: set.name.clone(),
                });
            }
        }

        Ok(Self {
            name,
            initial,
            step: Arc::new(step),
            acceptance,
            finite_policy,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initial(&self) -> &A {
        &self.initial
    }

    pub fn acceptance_sets(&self) -> &[AcceptanceSet<A>] {
        &self.acceptance
    }

    pub fn finite_policy(&self) -> FiniteRunPolicy {
        self.finite_policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuchiProductState<S, A> {
    pub state: S,
    pub automaton: A,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuchiStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuchiCounterexample<S, A> {
    /// A finite maximal run violates the explicit strict terminal policy.
    FiniteTerminal {
        missing_acceptance: String,
        trace: Vec<TraceStep<BuchiProductState<S, A>>>,
    },
    /// An infinite lasso can remain forever outside one acceptance set.
    AcceptanceAvoidingCycle {
        acceptance: String,
        stem: Vec<TraceStep<BuchiProductState<S, A>>>,
        cycle: Vec<TraceStep<BuchiProductState<S, A>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuchiResult<S, A> {
    pub automaton: String,
    pub status: BuchiStatus,
    pub finite_policy: FiniteRunPolicy,
    pub acceptance_sets: usize,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub counterexample: Option<BuchiCounterexample<S, A>>,
}

/// Generalized Büchi verification under deterministic product-space limits.
///
/// Model capture remains exhaustive; these counters describe only the bounded
/// action-product phase. A retained real terminal/lasso may prove violation
/// before a later cutoff, while satisfaction still requires product completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedBuchiResult<S, A> {
    pub automaton: String,
    pub outcome: BoundedOutcome<BuchiStatus>,
    pub finite_policy: FiniteRunPolicy,
    pub acceptance_sets: usize,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub counterexample: Option<BuchiCounterexample<S, A>>,
}

/// Generalized Büchi verification under independent model and product budgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisBuchiResult<S, A> {
    pub automaton: String,
    pub outcome: AnalysisOutcome<BuchiStatus>,
    pub finite_policy: FiniteRunPolicy,
    pub acceptance_sets: usize,
    pub model_completion: BoundedOutcome<()>,
    pub product_completion: BoundedOutcome<()>,
    pub model_states: usize,
    pub checked_model_states: usize,
    pub explored_model_transitions: usize,
    pub retained_model_transitions: usize,
    pub max_model_depth_reached: Option<usize>,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub counterexample: Option<BuchiCounterexample<S, A>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuchiError {
    EmptyAutomatonName,
    NoAcceptanceSets,
    EmptyAcceptanceName,
    DuplicateAcceptanceName { name: String },
    Graph(RecurrenceError),
    MissingWitness,
}

impl fmt::Display for BuchiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAutomatonName => write!(f, "Buchi automaton name must not be empty"),
            Self::NoAcceptanceSets => {
                write!(f, "Buchi automaton requires at least one acceptance set")
            }
            Self::EmptyAcceptanceName => write!(f, "Buchi acceptance-set name must not be empty"),
            Self::DuplicateAcceptanceName { name } => {
                write!(f, "duplicate Buchi acceptance-set name '{name}'")
            }
            Self::Graph(error) => write!(f, "Buchi product analysis failed: {error}"),
            Self::MissingWitness => write!(f, "Buchi violation did not yield a witness"),
        }
    }
}

impl std::error::Error for BuchiError {}

impl From<RecurrenceError> for BuchiError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

/// Universally verify generalized Büchi acceptance over all infinite maximal
/// executions of a finite model, with explicit handling for finite terminals.
pub fn check_buchi<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
) -> Result<BuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let product = build_action_product(
        &captured.graph,
        &automaton.initial,
        |state, action| (automaton.step)(state, action),
        |state, automaton| BuchiProductState { state, automaton },
    );
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let known_terminal = product
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let counterexample = find_counterexample(&product, &known_terminal, automaton)?;

    Ok(BuchiResult {
        automaton: automaton.name.clone(),
        status: if counterexample.is_some() {
            BuchiStatus::Violated
        } else {
            BuchiStatus::Satisfied
        },
        finite_policy: automaton.finite_policy,
        acceptance_sets: automaton.acceptance.len(),
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample,
    })
}

/// Verify generalized Büchi semantics while bounding only deterministic action-
/// product construction after complete model capture.
pub fn check_buchi_with_product_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    limits: ExplorationLimits,
) -> Result<BoundedBuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
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
        &automaton.initial,
        |state, action| (automaton.step)(state, action),
        |state, automaton| BuchiProductState { state, automaton },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let counterexample = find_counterexample(&product, &known_terminal, automaton)?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    } else {
        match completion {
            BoundedOutcome::Conclusive(()) => BoundedOutcome::Conclusive(BuchiStatus::Satisfied),
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedBuchiResult {
        automaton: automaton.name.clone(),
        outcome,
        finite_policy: automaton.finite_policy,
        acceptance_sets: automaton.acceptance.len(),
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

/// Verify generalized Büchi semantics under a staged whole-analysis envelope.
pub fn check_buchi_with_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    limits: AnalysisLimits,
) -> Result<AnalysisBuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    let StagedActionProduct {
        product:
            BoundedActionProduct {
                graph: product,
                checked_states: checked_product_states,
                explored_transitions: explored_product_transitions,
                max_depth_reached: max_product_depth_reached,
                completion: product_completion,
                known_terminal,
            },
        model_discovered_states,
        model_checked_states,
        model_explored_transitions,
        model_retained_transitions,
        model_max_depth_reached,
        model_completion,
    } = build_action_product_with_analysis_limits(
        model,
        &automaton.initial,
        |state, action| (automaton.step)(state, action),
        |state, automaton| BuchiProductState { state, automaton },
        limits,
    )
    .map_err(RecurrenceError::from)?;

    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let counterexample = find_counterexample(&product, &known_terminal, automaton)?;
    let outcome = staged_buchi_outcome(
        counterexample.is_some(),
        &model_completion,
        &product_completion,
    );

    Ok(AnalysisBuchiResult {
        automaton: automaton.name.clone(),
        outcome,
        finite_policy: automaton.finite_policy,
        acceptance_sets: automaton.acceptance.len(),
        model_completion,
        product_completion,
        model_states: model_discovered_states,
        checked_model_states: model_checked_states,
        explored_model_transitions: model_explored_transitions,
        retained_model_transitions: model_retained_transitions,
        max_model_depth_reached: model_max_depth_reached,
        product_states: product.states.len(),
        checked_product_states,
        explored_product_transitions,
        retained_product_transitions,
        max_product_depth_reached,
        counterexample,
    })
}

fn staged_buchi_outcome(
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

fn find_counterexample<S, A>(
    product: &ReachableGraph<BuchiProductState<S, A>>,
    known_terminal: &[bool],
    automaton: &BuchiAutomaton<A>,
) -> Result<Option<BuchiCounterexample<S, A>>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if automaton.finite_policy == FiniteRunPolicy::RequireAcceptingTerminal {
        for (product_id, state) in product.states.iter().enumerate() {
            if !known_terminal[product_id] {
                continue;
            }
            let Some(set) = automaton
                .acceptance
                .iter()
                .find(|set| !(set.predicate)(&state.automaton))
            else {
                continue;
            };
            let trace = shortest_path(product, &product.initial_ids, product_id, None)
                .ok_or(BuchiError::MissingWitness)?;
            return Ok(Some(BuchiCounterexample::FiniteTerminal {
                missing_acceptance: set.name.clone(),
                trace,
            }));
        }
    }

    let mut best: Option<CycleCandidate<S, A>> = None;
    for (acceptance_index, set) in automaton.acceptance.iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| !(set.predicate)(&state.automaton))
            .collect::<Vec<_>>();
        let old_ids = included
            .iter()
            .enumerate()
            .filter(|(_, included)| **included)
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        if old_ids.is_empty() {
            continue;
        }

        let residual = induced_graph(product, &included);
        let components = strongly_connected_components(&residual);
        for (component_index, component) in components.iter().enumerate() {
            if !component_is_cyclic(&residual, component) {
                continue;
            }
            let entry = *component.first().ok_or(BuchiError::MissingWitness)?;
            let product_entry = old_ids[entry];
            let stem = shortest_path(product, &product.initial_ids, product_entry, None)
                .ok_or(BuchiError::MissingWitness)?;
            let candidate = CycleCandidate {
                acceptance_index,
                product_entry,
                stem,
                residual: residual.clone(),
                component_index,
                component: component.clone(),
            };
            if best
                .as_ref()
                .is_none_or(|current| candidate_key(&candidate) < candidate_key(current))
            {
                best = Some(candidate);
            }
        }
    }

    if let Some(mut candidate) = best {
        let entry = *candidate
            .component
            .first()
            .ok_or(BuchiError::MissingWitness)?;
        candidate.residual.initial_ids = vec![entry];
        let witness = cycle_witness(
            &candidate.residual,
            candidate.component_index,
            &candidate.component,
        )?
        .ok_or(BuchiError::MissingWitness)?;
        return Ok(Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance: automaton.acceptance[candidate.acceptance_index]
                .name
                .clone(),
            stem: candidate.stem,
            cycle: witness.cycle,
        }));
    }

    Ok(None)
}

struct CycleCandidate<S, A> {
    acceptance_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<BuchiProductState<S, A>>>,
    residual: ReachableGraph<BuchiProductState<S, A>>,
    component_index: usize,
    component: Vec<usize>,
}

fn candidate_key<S, A>(candidate: &CycleCandidate<S, A>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.acceptance_index,
        candidate.product_entry,
    )
}

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

type MonitorStep<M> = Arc<dyn Fn(&M, &str) -> M + Send + Sync>;
type MonitorPredicate<M> = Arc<dyn Fn(&M) -> bool + Send + Sync>;

/// A named monitor-state predicate that represents an immediate violation.
pub struct RejectCondition<M> {
    name: String,
    predicate: MonitorPredicate<M>,
}

impl<M> Clone for RejectCondition<M> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            predicate: Arc::clone(&self.predicate),
        }
    }
}

impl<M> fmt::Debug for RejectCondition<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RejectCondition")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<M> RejectCondition<M> {
    pub fn new(
        name: impl Into<String>,
        predicate: impl Fn(&M) -> bool + Send + Sync + 'static,
    ) -> Result<Self, MonitorError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MonitorError::EmptyConditionName);
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

/// A named monitor region that may not contain a maximal suffix forever.
pub struct ProgressCondition<M> {
    name: String,
    active: MonitorPredicate<M>,
}

impl<M> Clone for ProgressCondition<M> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            active: Arc::clone(&self.active),
        }
    }
}

impl<M> fmt::Debug for ProgressCondition<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressCondition")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<M> ProgressCondition<M> {
    pub fn new(
        name: impl Into<String>,
        active: impl Fn(&M) -> bool + Send + Sync + 'static,
    ) -> Result<Self, MonitorError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MonitorError::EmptyConditionName);
        }
        Ok(Self {
            name,
            active: Arc::new(active),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// A deterministic action-driven finite monitor plus verification conditions.
pub struct FiniteMonitor<M> {
    name: String,
    initial: M,
    step: MonitorStep<M>,
    rejecting: Vec<RejectCondition<M>>,
    progress: Vec<ProgressCondition<M>>,
}

impl<M: Clone> Clone for FiniteMonitor<M> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            initial: self.initial.clone(),
            step: Arc::clone(&self.step),
            rejecting: self.rejecting.clone(),
            progress: self.progress.clone(),
        }
    }
}

impl<M: fmt::Debug> fmt::Debug for FiniteMonitor<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FiniteMonitor")
            .field("name", &self.name)
            .field("initial", &self.initial)
            .field(
                "rejecting",
                &self
                    .rejecting
                    .iter()
                    .map(|item| &item.name)
                    .collect::<Vec<_>>(),
            )
            .field(
                "progress",
                &self
                    .progress
                    .iter()
                    .map(|item| &item.name)
                    .collect::<Vec<_>>(),
            )
            .finish_non_exhaustive()
    }
}

impl<M> FiniteMonitor<M> {
    pub fn new(
        name: impl Into<String>,
        initial: M,
        step: impl Fn(&M, &str) -> M + Send + Sync + 'static,
        rejecting: Vec<RejectCondition<M>>,
        progress: Vec<ProgressCondition<M>>,
    ) -> Result<Self, MonitorError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MonitorError::EmptyMonitorName);
        }
        if rejecting.is_empty() && progress.is_empty() {
            return Err(MonitorError::NoConditions);
        }

        let mut names = HashSet::new();
        for condition in &rejecting {
            if !names.insert(condition.name.clone()) {
                return Err(MonitorError::DuplicateConditionName {
                    name: condition.name.clone(),
                });
            }
        }
        for condition in &progress {
            if !names.insert(condition.name.clone()) {
                return Err(MonitorError::DuplicateConditionName {
                    name: condition.name.clone(),
                });
            }
        }

        Ok(Self {
            name,
            initial,
            step: Arc::new(step),
            rejecting,
            progress,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initial(&self) -> &M {
        &self.initial
    }

    pub fn rejecting(&self) -> &[RejectCondition<M>] {
        &self.rejecting
    }

    pub fn progress(&self) -> &[ProgressCondition<M>] {
        &self.progress
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MonitorProductState<S, M> {
    pub state: S,
    pub monitor: M,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorCounterexample<S, M> {
    Rejecting {
        condition: String,
        trace: Vec<TraceStep<MonitorProductState<S, M>>>,
    },
    ProgressTerminal {
        condition: String,
        trace: Vec<TraceStep<MonitorProductState<S, M>>>,
    },
    ProgressCycle {
        condition: String,
        stem: Vec<TraceStep<MonitorProductState<S, M>>>,
        cycle: Vec<TraceStep<MonitorProductState<S, M>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorResult<S, M> {
    pub monitor: String,
    pub status: MonitorStatus,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub counterexample: Option<MonitorCounterexample<S, M>>,
}

/// Finite-monitor verification under deterministic product-space limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMonitorResult<S, M> {
    pub monitor: String,
    pub outcome: BoundedOutcome<MonitorStatus>,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub counterexample: Option<MonitorCounterexample<S, M>>,
}

/// Finite-monitor verification under independent model and product budgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisMonitorResult<S, M> {
    pub monitor: String,
    pub outcome: AnalysisOutcome<MonitorStatus>,
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
    pub counterexample: Option<MonitorCounterexample<S, M>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorError {
    EmptyMonitorName,
    NoConditions,
    EmptyConditionName,
    DuplicateConditionName { name: String },
    Graph(RecurrenceError),
    MissingWitness,
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMonitorName => write!(f, "finite monitor name must not be empty"),
            Self::NoConditions => write!(
                f,
                "finite monitor requires at least one rejecting or progress condition"
            ),
            Self::EmptyConditionName => write!(f, "monitor condition name must not be empty"),
            Self::DuplicateConditionName { name } => {
                write!(f, "duplicate monitor condition name '{name}'")
            }
            Self::Graph(error) => write!(f, "monitor product analysis failed: {error}"),
            Self::MissingWitness => write!(f, "monitor violation did not yield a witness"),
        }
    }
}

impl std::error::Error for MonitorError {}

impl From<RecurrenceError> for MonitorError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

pub fn check_monitor<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
) -> Result<MonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let product = build_action_product(
        &captured.graph,
        &monitor.initial,
        |state, action| (monitor.step)(state, action),
        |state, monitor| MonitorProductState { state, monitor },
    );
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let known_terminal = product
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let counterexample = find_counterexample(&product, &known_terminal, monitor)?;

    Ok(MonitorResult {
        monitor: monitor.name.clone(),
        status: if counterexample.is_some() {
            MonitorStatus::Violated
        } else {
            MonitorStatus::Satisfied
        },
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample,
    })
}

pub fn check_monitor_with_product_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    limits: ExplorationLimits,
) -> Result<BoundedMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
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
        &monitor.initial,
        |state, action| (monitor.step)(state, action),
        |state, monitor| MonitorProductState { state, monitor },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let counterexample = find_counterexample(&product, &known_terminal, monitor)?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(MonitorStatus::Violated)
    } else {
        match completion {
            BoundedOutcome::Conclusive(()) => BoundedOutcome::Conclusive(MonitorStatus::Satisfied),
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedMonitorResult {
        monitor: monitor.name.clone(),
        outcome,
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

/// Verify finite-monitor semantics under a staged whole-analysis envelope.
pub fn check_monitor_with_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    limits: AnalysisLimits,
) -> Result<AnalysisMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
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
        &monitor.initial,
        |state, action| (monitor.step)(state, action),
        |state, monitor| MonitorProductState { state, monitor },
        limits,
    )
    .map_err(RecurrenceError::from)?;

    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let counterexample = find_counterexample(&product, &known_terminal, monitor)?;
    let outcome = staged_monitor_outcome(
        counterexample.is_some(),
        &model_completion,
        &product_completion,
    );

    Ok(AnalysisMonitorResult {
        monitor: monitor.name.clone(),
        outcome,
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

fn staged_monitor_outcome(
    violated: bool,
    model_completion: &BoundedOutcome<()>,
    product_completion: &BoundedOutcome<()>,
) -> AnalysisOutcome<MonitorStatus> {
    if violated {
        return AnalysisOutcome::Conclusive(MonitorStatus::Violated);
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
    AnalysisOutcome::Conclusive(MonitorStatus::Satisfied)
}

fn find_counterexample<S, M>(
    product: &ReachableGraph<MonitorProductState<S, M>>,
    known_terminal: &[bool],
    monitor: &FiniteMonitor<M>,
) -> Result<Option<MonitorCounterexample<S, M>>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
{
    for (product_id, state) in product.states.iter().enumerate() {
        for condition in &monitor.rejecting {
            if (condition.predicate)(&state.monitor) {
                let trace = shortest_path(product, &product.initial_ids, product_id, None)
                    .ok_or(MonitorError::MissingWitness)?;
                return Ok(Some(MonitorCounterexample::Rejecting {
                    condition: condition.name.clone(),
                    trace,
                }));
            }
        }
    }

    for (product_id, state) in product.states.iter().enumerate() {
        if !known_terminal[product_id] {
            continue;
        }
        for condition in &monitor.progress {
            if (condition.active)(&state.monitor) {
                let trace = shortest_path(product, &product.initial_ids, product_id, None)
                    .ok_or(MonitorError::MissingWitness)?;
                return Ok(Some(MonitorCounterexample::ProgressTerminal {
                    condition: condition.name.clone(),
                    trace,
                }));
            }
        }
    }

    let mut best: Option<ProgressCandidate<S, M>> = None;
    for (condition_index, condition) in monitor.progress.iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| (condition.active)(&state.monitor))
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
            let entry = *component.first().ok_or(MonitorError::MissingWitness)?;
            let product_entry = old_ids[entry];
            let stem = shortest_path(product, &product.initial_ids, product_entry, None)
                .ok_or(MonitorError::MissingWitness)?;
            let candidate = ProgressCandidate {
                condition_index,
                product_entry,
                stem,
                residual: residual.clone(),
                component_index,
                component: component.clone(),
            };
            let replace = best
                .as_ref()
                .is_none_or(|current| candidate_key(&candidate) < candidate_key(current));
            if replace {
                best = Some(candidate);
            }
        }
    }

    if let Some(mut candidate) = best {
        let entry = *candidate
            .component
            .first()
            .ok_or(MonitorError::MissingWitness)?;
        candidate.residual.initial_ids = vec![entry];
        let witness = cycle_witness(
            &candidate.residual,
            candidate.component_index,
            &candidate.component,
        )?
        .ok_or(MonitorError::MissingWitness)?;
        return Ok(Some(MonitorCounterexample::ProgressCycle {
            condition: monitor.progress[candidate.condition_index].name.clone(),
            stem: candidate.stem,
            cycle: witness.cycle,
        }));
    }

    Ok(None)
}

struct ProgressCandidate<S, M> {
    condition_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<MonitorProductState<S, M>>>,
    residual: ReachableGraph<MonitorProductState<S, M>>,
    component_index: usize,
    component: Vec<usize>,
}

fn candidate_key<S, M>(candidate: &ProgressCandidate<S, M>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.condition_index,
        candidate.product_entry,
    )
}

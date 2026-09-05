use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, TraceStep};
use crate::graph::{
    capture_reachable_graph, capture_reachable_graph_with_limits, induced_graph, shortest_path,
    GraphCaptureCompletion, ReachableGraph,
};
use crate::model::TransitionSystem;
use crate::recurrence::{
    component_is_cyclic, cycle_witness, strongly_connected_components, RecurrenceError,
};
use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

/// A named universal eventuality query over maximal executions of the finite
/// transition graph: every execution must eventually reach a target state.
pub struct EventualityProperty<S> {
    name: String,
    target: Arc<dyn Fn(&S) -> bool + Send + Sync>,
}

impl<S> Clone for EventualityProperty<S> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            target: Arc::clone(&self.target),
        }
    }
}

impl<S> fmt::Debug for EventualityProperty<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventualityProperty")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<S> EventualityProperty<S> {
    pub fn new(
        name: impl Into<String>,
        target: impl Fn(&S) -> bool + Send + Sync + 'static,
    ) -> Result<Self, EventualityError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(EventualityError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            target: Arc::new(target),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventualityStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventualityCounterexample<S> {
    /// A maximal finite execution terminates before the target is reached.
    Finite { trace: Vec<TraceStep<S>> },
    /// An execution can remain forever in a target-free recurrent region.
    Infinite {
        stem: Vec<TraceStep<S>>,
        cycle: Vec<TraceStep<S>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventualityResult<S> {
    pub property: String,
    pub status: EventualityStatus,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub counterexample: Option<EventualityCounterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedEventualityResult<S> {
    pub property: String,
    pub outcome: BoundedOutcome<EventualityStatus>,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub counterexample: Option<EventualityCounterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventualityError {
    EmptyPropertyName,
    Graph(RecurrenceError),
    MissingFiniteWitness,
    MissingCycleWitness,
}

impl fmt::Display for EventualityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "eventuality property name must not be empty"),
            Self::Graph(error) => write!(f, "eventuality graph analysis failed: {error}"),
            Self::MissingFiniteWitness => write!(
                f,
                "target-free terminal state did not yield a finite counterexample trace"
            ),
            Self::MissingCycleWitness => write!(
                f,
                "target-free cyclic component did not yield a stem-plus-cycle counterexample"
            ),
        }
    }
}

impl std::error::Error for EventualityError {}

impl From<RecurrenceError> for EventualityError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

/// Check the finite-graph universal eventuality property "on every maximal
/// execution, eventually target" without a fairness assumption.
///
/// The model is explored exactly once by the canonical BFS substrate. Target
/// states are absorbing for this property: after an execution reaches target,
/// later behavior is irrelevant. Analysis therefore computes the subgraph
/// reachable from non-target initial states without crossing any target state.
/// A violation is witnessed either by a true terminal state in that residual
/// graph or by a residual cyclic SCC. Terminal counterexamples take precedence
/// over cycle counterexamples; within each class canonical discovery order is
/// deterministic.
pub fn check_eventuality<S>(
    model: &TransitionSystem<S>,
    property: &EventualityProperty<S>,
) -> Result<EventualityResult<S>, EventualityError>
where
    S: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let known_terminal = captured
        .graph
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let counterexample = find_counterexample(&captured.graph, &known_terminal, property)?;

    Ok(EventualityResult {
        property: property.name.clone(),
        status: if counterexample.is_some() {
            EventualityStatus::Violated
        } else {
            EventualityStatus::Satisfied
        },
        discovered_states: captured.discovered_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        counterexample,
    })
}

/// Check universal eventuality under deterministic resource limits.
///
/// A real target-free terminal or closed target-free cycle already present in
/// the justified graph prefix is a conclusive violation even if exploration
/// later hits a resource limit. In contrast, satisfaction requires complete
/// exploration of the relevant finite graph; an incomplete prefix with no
/// counterexample is therefore `Inconclusive`.
pub fn check_eventuality_with_limits<S>(
    model: &TransitionSystem<S>,
    property: &EventualityProperty<S>,
    limits: ExplorationLimits,
) -> Result<BoundedEventualityResult<S>, EventualityError>
where
    S: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph_with_limits(model, limits)
        .map_err(RecurrenceError::from)?;
    let counterexample = find_counterexample(&captured.graph, &captured.known_terminal, property)?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(EventualityStatus::Violated)
    } else {
        match captured.completion {
            GraphCaptureCompletion::Complete => {
                BoundedOutcome::Conclusive(EventualityStatus::Satisfied)
            }
            GraphCaptureCompletion::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedEventualityResult {
        property: property.name.clone(),
        outcome,
        discovered_states: captured.discovered_states,
        checked_states: captured.checked_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        counterexample,
    })
}

fn find_counterexample<S>(
    graph: &ReachableGraph<S>,
    known_terminal: &[bool],
    property: &EventualityProperty<S>,
) -> Result<Option<EventualityCounterexample<S>>, EventualityError>
where
    S: Clone + Eq + Hash,
{
    if graph.states.is_empty() {
        return Ok(None);
    }
    debug_assert_eq!(known_terminal.len(), graph.states.len());

    let is_target = graph
        .states
        .iter()
        .map(|state| (property.target)(state))
        .collect::<Vec<_>>();

    let mut residual_reachable = vec![false; graph.states.len()];
    let mut queue = VecDeque::new();
    for &initial in &graph.initial_ids {
        if !is_target[initial] && !residual_reachable[initial] {
            residual_reachable[initial] = true;
            queue.push_back(initial);
        }
    }

    while let Some(node) = queue.pop_front() {
        for edge in &graph.outgoing[node] {
            if !is_target[edge.target] && !residual_reachable[edge.target] {
                residual_reachable[edge.target] = true;
                queue.push_back(edge.target);
            }
        }
    }

    let residual_ids = residual_reachable
        .iter()
        .enumerate()
        .filter(|(_, reachable)| **reachable)
        .map(|(id, _)| id)
        .collect::<HashSet<_>>();

    if let Some(terminal) = (0..graph.states.len())
        .find(|id| residual_reachable[*id] && known_terminal[*id])
    {
        let trace = shortest_path(graph, &graph.initial_ids, terminal, Some(&residual_ids))
            .ok_or(EventualityError::MissingFiniteWitness)?;
        return Ok(Some(EventualityCounterexample::Finite { trace }));
    }

    let residual = induced_graph(graph, &residual_reachable);
    let components = strongly_connected_components(&residual);
    if let Some((component_index, component)) = components
        .iter()
        .enumerate()
        .find(|(_, component)| component_is_cyclic(&residual, component))
    {
        let witness = cycle_witness(&residual, component_index, component)?
            .ok_or(EventualityError::MissingCycleWitness)?;
        return Ok(Some(EventualityCounterexample::Infinite {
            stem: witness.stem,
            cycle: witness.cycle,
        }));
    }

    Ok(None)
}

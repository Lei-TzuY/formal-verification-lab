use crate::checker::TraceStep;
use crate::graph::{capture_reachable_graph, shortest_path, GraphCaptureError, ReachableGraph};
use crate::model::{ModelError, TransitionSystem};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

/// One reachable strongly connected component, with states ordered by the
/// canonical BFS discovery order used to build the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StronglyConnectedComponent<S> {
    pub states: Vec<S>,
    /// A component is cyclic when it contains more than one state or its
    /// singleton state has a self-loop.
    pub cyclic: bool,
}

/// A deterministic witness for the first cyclic SCC in canonical ordering.
///
/// `stem` is a shortest transition-count path from an initial state to the
/// component entry. `cycle` starts and ends at that same entry. The selected
/// cycle is deterministic but is not claimed to be globally shortest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleWitness<S> {
    pub component_index: usize,
    pub stem: Vec<TraceStep<S>>,
    pub cycle: Vec<TraceStep<S>>,
}

/// Deterministic SCC/recurrent-cycle analysis over the exhaustively reachable
/// finite graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceAnalysis<S> {
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub components: Vec<StronglyConnectedComponent<S>>,
    pub first_cycle: Option<CycleWitness<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrenceError {
    Model(ModelError),
    UnexpectedInconclusive,
    SnapshotTargetMissing,
    CycleWitnessMissing,
}

impl fmt::Display for RecurrenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "recurrence exploration failed: {error}"),
            Self::UnexpectedInconclusive => write!(
                f,
                "unbounded canonical recurrence exploration unexpectedly became inconclusive"
            ),
            Self::SnapshotTargetMissing => write!(
                f,
                "captured transition points to a state missing from the exhausted graph snapshot"
            ),
            Self::CycleWitnessMissing => write!(
                f,
                "cyclic strongly connected component did not yield a closed cycle witness"
            ),
        }
    }
}

impl std::error::Error for RecurrenceError {}

impl From<ModelError> for RecurrenceError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

impl From<GraphCaptureError> for RecurrenceError {
    fn from(value: GraphCaptureError) -> Self {
        match value {
            GraphCaptureError::Model(error) => Self::Model(error),
            GraphCaptureError::UnexpectedInconclusive => Self::UnexpectedInconclusive,
            GraphCaptureError::SnapshotTargetMissing => Self::SnapshotTargetMissing,
        }
    }
}

/// Exhaustively materialize the reachable transition graph once through the
/// neutral graph substrate, then analyze SCC structure without invoking the
/// model transition relation again.
pub fn analyze_recurrence<S>(
    model: &TransitionSystem<S>,
) -> Result<RecurrenceAnalysis<S>, RecurrenceError>
where
    S: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let component_ids = strongly_connected_components(&captured.graph);
    let components = component_ids
        .iter()
        .map(|ids| StronglyConnectedComponent {
            states: ids
                .iter()
                .map(|id| captured.graph.states[*id].clone())
                .collect(),
            cyclic: component_is_cyclic(&captured.graph, ids),
        })
        .collect::<Vec<_>>();

    let first_cycle = component_ids
        .iter()
        .enumerate()
        .find(|(_, ids)| component_is_cyclic(&captured.graph, ids))
        .map(|(component_index, ids)| cycle_witness(&captured.graph, component_index, ids))
        .transpose()?
        .flatten();

    Ok(RecurrenceAnalysis {
        discovered_states: captured.discovered_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        components,
        first_cycle,
    })
}

pub(crate) fn strongly_connected_components<S>(graph: &ReachableGraph<S>) -> Vec<Vec<usize>> {
    struct TarjanState {
        next_index: usize,
        index: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        components: Vec<Vec<usize>>,
    }

    fn visit<S>(node: usize, graph: &ReachableGraph<S>, state: &mut TarjanState) {
        let node_index = state.next_index;
        state.next_index += 1;
        state.index[node] = Some(node_index);
        state.lowlink[node] = node_index;
        state.stack.push(node);
        state.on_stack[node] = true;

        for edge in &graph.outgoing[node] {
            let target = edge.target;
            if state.index[target].is_none() {
                visit(target, graph, state);
                state.lowlink[node] = state.lowlink[node].min(state.lowlink[target]);
            } else if state.on_stack[target] {
                state.lowlink[node] = state.lowlink[node].min(
                    state.index[target].expect("a node on the Tarjan stack has an assigned index"),
                );
            }
        }

        if state.lowlink[node] == node_index {
            let mut component = Vec::new();
            loop {
                let member = state
                    .stack
                    .pop()
                    .expect("Tarjan root has itself on the stack");
                state.on_stack[member] = false;
                component.push(member);
                if member == node {
                    break;
                }
            }
            component.sort_unstable();
            state.components.push(component);
        }
    }

    let count = graph.states.len();
    let mut state = TarjanState {
        next_index: 0,
        index: vec![None; count],
        lowlink: vec![0; count],
        stack: Vec::new(),
        on_stack: vec![false; count],
        components: Vec::new(),
    };

    for node in 0..count {
        if state.index[node].is_none() {
            visit(node, graph, &mut state);
        }
    }

    state
        .components
        .sort_by_key(|component| component.first().copied().unwrap_or(usize::MAX));
    state.components
}

pub(crate) fn component_is_cyclic<S>(graph: &ReachableGraph<S>, component: &[usize]) -> bool {
    component.len() > 1
        || component.first().is_some_and(|node| {
            graph.outgoing[*node]
                .iter()
                .any(|edge| edge.target == *node)
        })
}

pub(crate) fn cycle_witness<S: Clone + Eq>(
    graph: &ReachableGraph<S>,
    component_index: usize,
    component: &[usize],
) -> Result<Option<CycleWitness<S>>, RecurrenceError> {
    let Some(&entry) = component.first() else {
        return Ok(None);
    };
    let members = component.iter().copied().collect::<HashSet<_>>();
    let stem = shortest_path(graph, &graph.initial_ids, entry, None)
        .ok_or(RecurrenceError::CycleWitnessMissing)?;

    let first_internal_edge = graph.outgoing[entry]
        .iter()
        .find(|edge| members.contains(&edge.target))
        .ok_or(RecurrenceError::CycleWitnessMissing)?;

    let mut cycle = vec![TraceStep {
        action: None,
        state: graph.states[entry].clone(),
    }];
    cycle.push(TraceStep {
        action: Some(first_internal_edge.action.clone()),
        state: graph.states[first_internal_edge.target].clone(),
    });

    if first_internal_edge.target != entry {
        let return_path =
            shortest_path(graph, &[first_internal_edge.target], entry, Some(&members))
                .ok_or(RecurrenceError::CycleWitnessMissing)?;
        cycle.extend(return_path.into_iter().skip(1));
    }

    if cycle.len() < 2
        || cycle.first().map(|step| &step.state) != cycle.last().map(|step| &step.state)
    {
        return Err(RecurrenceError::CycleWitnessMissing);
    }

    Ok(Some(CycleWitness {
        component_index,
        stem,
        cycle,
    }))
}

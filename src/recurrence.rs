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

    #[derive(Debug, Clone, Copy)]
    struct DfsFrame {
        node: usize,
        next_edge: usize,
    }

    fn enter_node(node: usize, state: &mut TarjanState, frames: &mut Vec<DfsFrame>) {
        let node_index = state.next_index;
        state.next_index += 1;
        state.index[node] = Some(node_index);
        state.lowlink[node] = node_index;
        state.stack.push(node);
        state.on_stack[node] = true;
        frames.push(DfsFrame { node, next_edge: 0 });
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
    let mut frames = Vec::new();

    for root in 0..count {
        if state.index[root].is_some() {
            continue;
        }
        enter_node(root, &mut state, &mut frames);

        while let Some(frame) = frames.last_mut() {
            let node = frame.node;
            if frame.next_edge < graph.outgoing[node].len() {
                let target = graph.outgoing[node][frame.next_edge].target;
                frame.next_edge += 1;

                if state.index[target].is_none() {
                    enter_node(target, &mut state, &mut frames);
                } else if state.on_stack[target] {
                    state.lowlink[node] = state.lowlink[node].min(
                        state.index[target]
                            .expect("a node on the Tarjan stack has an assigned index"),
                    );
                }
                continue;
            }

            let finished = frames
                .pop()
                .expect("the active Tarjan frame exists while finishing a node");
            let node = finished.node;
            let node_index =
                state.index[node].expect("an active Tarjan frame has an assigned discovery index");

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

            if let Some(parent) = frames.last() {
                let parent = parent.node;
                state.lowlink[parent] = state.lowlink[parent].min(state.lowlink[node]);
            }
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

#[cfg(test)]
mod tests {
    use super::strongly_connected_components;
    use crate::graph::{ReachableGraph, SnapshotEdge};

    const SMALL_N: usize = 3;
    const SMALL_EDGE_COUNT: usize = SMALL_N * SMALL_N;
    const DEEP_N: usize = 50_000;

    fn graph_from_mask(mask: usize) -> ReachableGraph<usize> {
        let mut outgoing = vec![Vec::new(); SMALL_N];
        for (from, edges) in outgoing.iter_mut().enumerate() {
            for to in 0..SMALL_N {
                if mask & (1usize << (from * SMALL_N + to)) != 0 {
                    edges.push(SnapshotEdge {
                        action: format!("{from}->{to}"),
                        target: to,
                    });
                }
            }
        }
        ReachableGraph {
            states: (0..SMALL_N).collect(),
            outgoing,
            initial_ids: vec![0],
        }
    }

    fn recursive_reference<S>(graph: &ReachableGraph<S>) -> Vec<Vec<usize>> {
        struct State {
            next_index: usize,
            index: Vec<Option<usize>>,
            lowlink: Vec<usize>,
            stack: Vec<usize>,
            on_stack: Vec<bool>,
            components: Vec<Vec<usize>>,
        }

        fn visit<S>(node: usize, graph: &ReachableGraph<S>, state: &mut State) {
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
                        state.index[target]
                            .expect("a node on the reference Tarjan stack has an index"),
                    );
                }
            }

            if state.lowlink[node] == node_index {
                let mut component = Vec::new();
                loop {
                    let member = state
                        .stack
                        .pop()
                        .expect("reference Tarjan root remains on its stack");
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
        let mut state = State {
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

    #[test]
    fn iterative_tarjan_matches_recursive_order_on_all_three_node_graphs() {
        for mask in 0..(1usize << SMALL_EDGE_COUNT) {
            let graph = graph_from_mask(mask);
            assert_eq!(
                strongly_connected_components(&graph),
                recursive_reference(&graph),
                "mask={mask}"
            );
        }
    }

    #[test]
    fn iterative_tarjan_handles_deep_chain_without_recursive_dfs() {
        let mut outgoing = vec![Vec::new(); DEEP_N];
        for (node, edges) in outgoing.iter_mut().enumerate().take(DEEP_N - 1) {
            edges.push(SnapshotEdge {
                action: String::new(),
                target: node + 1,
            });
        }
        let graph = ReachableGraph {
            states: (0..DEEP_N).collect::<Vec<_>>(),
            outgoing,
            initial_ids: vec![0],
        };

        let components = strongly_connected_components(&graph);
        assert_eq!(components.len(), DEEP_N);
        assert_eq!(components.first(), Some(&vec![0]));
        assert_eq!(components.last(), Some(&vec![DEEP_N - 1]));
    }

    #[test]
    fn iterative_tarjan_handles_deep_cycle_without_recursive_dfs() {
        let mut outgoing = vec![Vec::new(); DEEP_N];
        for (node, edges) in outgoing.iter_mut().enumerate() {
            edges.push(SnapshotEdge {
                action: String::new(),
                target: (node + 1) % DEEP_N,
            });
        }
        let graph = ReachableGraph {
            states: (0..DEEP_N).collect::<Vec<_>>(),
            outgoing,
            initial_ids: vec![0],
        };

        let components = strongly_connected_components(&graph);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), DEEP_N);
        assert_eq!(components[0].first(), Some(&0));
        assert_eq!(components[0].last(), Some(&(DEEP_N - 1)));
    }
}

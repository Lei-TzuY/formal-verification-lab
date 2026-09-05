use crate::checker::{search_with_probes, ExplorationLimits, GraphSearchOutcome, TraceStep};
use crate::model::{ModelError, Transition, TransitionSystem};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub(crate) struct SnapshotEdge {
    pub(crate) action: String,
    pub(crate) target: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ReachableGraph<S> {
    pub(crate) states: Vec<S>,
    pub(crate) outgoing: Vec<Vec<SnapshotEdge>>,
    pub(crate) initial_ids: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct CapturedReachableGraph<S> {
    pub(crate) graph: ReachableGraph<S>,
    pub(crate) discovered_states: usize,
    pub(crate) explored_transitions: usize,
    pub(crate) max_depth_reached: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GraphCaptureError {
    Model(ModelError),
    UnexpectedInconclusive,
    SnapshotTargetMissing,
}

impl From<ModelError> for GraphCaptureError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Exhaustively materialize the reachable transition graph once through the
/// canonical BFS substrate. Higher-level graph analyses reuse this snapshot so
/// the model transition relation is never re-invoked for the same analysis.
pub(crate) fn capture_reachable_graph<S>(
    model: &TransitionSystem<S>,
) -> Result<CapturedReachableGraph<S>, GraphCaptureError>
where
    S: Clone + Eq + Hash,
{
    let mut captured: Vec<(S, Vec<Transition<S>>)> = Vec::new();
    let search = search_with_probes(
        model,
        ExplorationLimits::unbounded(),
        |_state| None,
        |state, transitions| {
            captured.push((state.clone(), transitions.to_vec()));
            None
        },
    )?;

    if !matches!(search.outcome, GraphSearchOutcome::Exhausted) {
        return Err(GraphCaptureError::UnexpectedInconclusive);
    }

    Ok(CapturedReachableGraph {
        graph: build_graph(model, captured)?,
        discovered_states: search.discovered_states,
        explored_transitions: search.explored_transitions,
        max_depth_reached: search.max_depth_reached,
    })
}

fn build_graph<S>(
    model: &TransitionSystem<S>,
    captured: Vec<(S, Vec<Transition<S>>)>,
) -> Result<ReachableGraph<S>, GraphCaptureError>
where
    S: Clone + Eq + Hash,
{
    let states = captured
        .iter()
        .map(|(state, _)| state.clone())
        .collect::<Vec<_>>();
    let state_to_id = states
        .iter()
        .cloned()
        .enumerate()
        .map(|(id, state)| (state, id))
        .collect::<HashMap<_, _>>();

    let outgoing = captured
        .into_iter()
        .map(|(_, transitions)| {
            transitions
                .into_iter()
                .map(|transition| {
                    let target = state_to_id
                        .get(&transition.next)
                        .copied()
                        .ok_or(GraphCaptureError::SnapshotTargetMissing)?;
                    Ok(SnapshotEdge {
                        action: transition.action,
                        target,
                    })
                })
                .collect::<Result<Vec<_>, GraphCaptureError>>()
        })
        .collect::<Result<Vec<_>, GraphCaptureError>>()?;

    let mut initial_ids = Vec::new();
    for initial in model.initial_states() {
        let id = state_to_id
            .get(initial)
            .copied()
            .ok_or(GraphCaptureError::SnapshotTargetMissing)?;
        if !initial_ids.contains(&id) {
            initial_ids.push(id);
        }
    }

    Ok(ReachableGraph {
        states,
        outgoing,
        initial_ids,
    })
}

/// Build a dense graph induced by `included`, preserving original discovery
/// order for both states and initial roots. Callers are responsible for making
/// `included` represent the desired reachable subgraph.
pub(crate) fn induced_graph<S: Clone>(
    graph: &ReachableGraph<S>,
    included: &[bool],
) -> ReachableGraph<S> {
    assert_eq!(included.len(), graph.states.len());

    let mut old_to_new = vec![None; graph.states.len()];
    let mut states = Vec::new();
    for (old_id, state) in graph.states.iter().enumerate() {
        if included[old_id] {
            let new_id = states.len();
            old_to_new[old_id] = Some(new_id);
            states.push(state.clone());
        }
    }

    let outgoing = graph
        .outgoing
        .iter()
        .enumerate()
        .filter(|(old_id, _)| included[*old_id])
        .map(|(_, edges)| {
            edges
                .iter()
                .filter_map(|edge| {
                    old_to_new[edge.target].map(|target| SnapshotEdge {
                        action: edge.action.clone(),
                        target,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let initial_ids = graph
        .initial_ids
        .iter()
        .filter_map(|old_id| old_to_new[*old_id])
        .collect();

    ReachableGraph {
        states,
        outgoing,
        initial_ids,
    }
}

pub(crate) fn shortest_path<S: Clone>(
    graph: &ReachableGraph<S>,
    starts: &[usize],
    goal: usize,
    allowed: Option<&HashSet<usize>>,
) -> Option<Vec<TraceStep<S>>> {
    let count = graph.states.len();
    let mut seen = vec![false; count];
    let mut root = vec![false; count];
    let mut predecessor: Vec<Option<(usize, String)>> = vec![None; count];
    let mut queue = VecDeque::new();

    for &start in starts {
        if allowed.is_some_and(|members| !members.contains(&start)) || seen[start] {
            continue;
        }
        seen[start] = true;
        root[start] = true;
        queue.push_back(start);
    }

    while let Some(node) = queue.pop_front() {
        if node == goal {
            break;
        }
        for edge in &graph.outgoing[node] {
            if allowed.is_some_and(|members| !members.contains(&edge.target)) || seen[edge.target] {
                continue;
            }
            seen[edge.target] = true;
            predecessor[edge.target] = Some((node, edge.action.clone()));
            queue.push_back(edge.target);
        }
    }

    if !seen.get(goal).copied().unwrap_or(false) {
        return None;
    }

    let mut reversed = Vec::new();
    let mut node = goal;
    loop {
        if root[node] {
            reversed.push(TraceStep {
                action: None,
                state: graph.states[node].clone(),
            });
            break;
        }
        let (previous, action) = predecessor[node].clone()?;
        reversed.push(TraceStep {
            action: Some(action),
            state: graph.states[node].clone(),
        });
        node = previous;
    }
    reversed.reverse();
    Some(reversed)
}

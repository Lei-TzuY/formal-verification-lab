use crate::checker::{
    search_with_probes, ExplorationLimits, GraphSearchOutcome, InconclusiveReason, TraceStep,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphCaptureCompletion {
    Complete,
    Inconclusive(InconclusiveReason),
}

/// A reachable-graph prefix produced by the canonical bounded BFS.
///
/// `graph` contains only transitions that were actually counted and whose
/// targets were retained by the search. `known_terminal[id]` is true only when
/// the model's full successor vector for that retained state was evaluated and
/// proved empty. This lets higher-level properties use real finite/cycle
/// counterexamples without mistaking an exploration cutoff for a terminal.
#[derive(Debug, Clone)]
pub(crate) struct BoundedCapturedReachableGraph<S> {
    pub(crate) graph: ReachableGraph<S>,
    pub(crate) discovered_states: usize,
    pub(crate) checked_states: usize,
    pub(crate) explored_transitions: usize,
    pub(crate) max_depth_reached: Option<usize>,
    pub(crate) completion: GraphCaptureCompletion,
    pub(crate) known_terminal: Vec<bool>,
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

/// Materialize the portion of the reachable graph that is justified by one
/// canonical bounded BFS. The transition function is still called at most once
/// per checked state. A cutoff never fabricates an absent edge or terminal.
pub(crate) fn capture_reachable_graph_with_limits<S>(
    model: &TransitionSystem<S>,
    limits: ExplorationLimits,
) -> Result<BoundedCapturedReachableGraph<S>, GraphCaptureError>
where
    S: Clone + Eq + Hash,
{
    let mut captured: Vec<(S, Vec<Transition<S>>)> = Vec::new();
    let search = search_with_probes(
        model,
        limits,
        |_state| None,
        |state, transitions| {
            captured.push((state.clone(), transitions.to_vec()));
            None
        },
    )?;

    let completion = match search.outcome {
        GraphSearchOutcome::Exhausted => GraphCaptureCompletion::Complete,
        GraphSearchOutcome::Inconclusive(reason) => GraphCaptureCompletion::Inconclusive(reason),
        GraphSearchOutcome::Match { .. } => return Err(GraphCaptureError::UnexpectedInconclusive),
    };
    let (graph, known_terminal) = build_bounded_graph(
        model,
        captured,
        search.discovered_states,
        search.explored_transitions,
        limits,
    )?;

    Ok(BoundedCapturedReachableGraph {
        graph,
        discovered_states: search.discovered_states,
        checked_states: search.checked_states,
        explored_transitions: search.explored_transitions,
        max_depth_reached: search.max_depth_reached,
        completion,
        known_terminal,
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

fn build_bounded_graph<S>(
    model: &TransitionSystem<S>,
    captured: Vec<(S, Vec<Transition<S>>)>,
    discovered_states: usize,
    explored_transitions: usize,
    limits: ExplorationLimits,
) -> Result<(ReachableGraph<S>, Vec<bool>), GraphCaptureError>
where
    S: Clone + Eq + Hash,
{
    let mut states = Vec::new();
    let mut state_to_id = HashMap::new();
    let mut depths = Vec::new();
    let mut outgoing: Vec<Vec<SnapshotEdge>> = Vec::new();
    let mut known_terminal = Vec::new();
    let mut initial_ids = Vec::new();

    for initial in model.initial_states() {
        if state_to_id.contains_key(initial) {
            continue;
        }
        if states.len() >= discovered_states {
            break;
        }
        let id = states.len();
        states.push(initial.clone());
        state_to_id.insert(initial.clone(), id);
        depths.push(0_usize);
        outgoing.push(Vec::new());
        known_terminal.push(false);
        initial_ids.push(id);
    }

    let mut remaining_transitions = explored_transitions;
    for (state, transitions) in captured {
        let source = state_to_id
            .get(&state)
            .copied()
            .ok_or(GraphCaptureError::SnapshotTargetMissing)?;
        if transitions.is_empty() {
            known_terminal[source] = true;
        }

        for transition in transitions {
            if remaining_transitions == 0 {
                break;
            }
            remaining_transitions -= 1;

            let target = if let Some(target) = state_to_id.get(&transition.next).copied() {
                Some(target)
            } else {
                let blocked_by_depth = limits
                    .max_depth
                    .is_some_and(|limit| depths[source] >= limit);
                if blocked_by_depth || states.len() >= discovered_states {
                    None
                } else {
                    let id = states.len();
                    states.push(transition.next.clone());
                    state_to_id.insert(transition.next.clone(), id);
                    depths.push(depths[source] + 1);
                    outgoing.push(Vec::new());
                    known_terminal.push(false);
                    Some(id)
                }
            };

            if let Some(target) = target {
                outgoing[source].push(SnapshotEdge {
                    action: transition.action,
                    target,
                });
            }
        }
    }

    if remaining_transitions != 0 || states.len() != discovered_states {
        return Err(GraphCaptureError::SnapshotTargetMissing);
    }

    Ok((
        ReachableGraph {
            states,
            outgoing,
            initial_ids,
        },
        known_terminal,
    ))
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

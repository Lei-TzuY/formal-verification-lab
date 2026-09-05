use crate::checker::{search_with_probes, ExplorationLimits, GraphSearchOutcome, TraceStep};
use crate::model::{ModelError, Transition, TransitionSystem};
use std::collections::{HashMap, HashSet, VecDeque};
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

#[derive(Debug, Clone)]
struct SnapshotEdge {
    action: String,
    target: usize,
}

#[derive(Debug, Clone)]
struct ReachableSnapshot<S> {
    states: Vec<S>,
    outgoing: Vec<Vec<SnapshotEdge>>,
    initial_ids: Vec<usize>,
}

/// Exhaustively materialize the reachable transition graph once through the
/// canonical BFS substrate, then analyze SCC structure without invoking the
/// model transition relation again.
pub fn analyze_recurrence<S>(
    model: &TransitionSystem<S>,
) -> Result<RecurrenceAnalysis<S>, RecurrenceError>
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
        return Err(RecurrenceError::UnexpectedInconclusive);
    }

    let snapshot = build_snapshot(model, captured)?;
    let component_ids = strongly_connected_components(&snapshot);
    let components = component_ids
        .iter()
        .map(|ids| StronglyConnectedComponent {
            states: ids.iter().map(|id| snapshot.states[*id].clone()).collect(),
            cyclic: component_is_cyclic(&snapshot, ids),
        })
        .collect::<Vec<_>>();

    let first_cycle = component_ids
        .iter()
        .enumerate()
        .find(|(_, ids)| component_is_cyclic(&snapshot, ids))
        .map(|(component_index, ids)| cycle_witness(&snapshot, component_index, ids))
        .transpose()?
        .flatten();

    Ok(RecurrenceAnalysis {
        discovered_states: search.discovered_states,
        explored_transitions: search.explored_transitions,
        max_depth_reached: search.max_depth_reached,
        components,
        first_cycle,
    })
}

fn build_snapshot<S>(
    model: &TransitionSystem<S>,
    captured: Vec<(S, Vec<Transition<S>>)>,
) -> Result<ReachableSnapshot<S>, RecurrenceError>
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
                        .ok_or(RecurrenceError::SnapshotTargetMissing)?;
                    Ok(SnapshotEdge {
                        action: transition.action,
                        target,
                    })
                })
                .collect::<Result<Vec<_>, RecurrenceError>>()
        })
        .collect::<Result<Vec<_>, RecurrenceError>>()?;

    let mut initial_ids = Vec::new();
    for initial in model.initial_states() {
        let id = state_to_id
            .get(initial)
            .copied()
            .ok_or(RecurrenceError::SnapshotTargetMissing)?;
        if !initial_ids.contains(&id) {
            initial_ids.push(id);
        }
    }

    Ok(ReachableSnapshot {
        states,
        outgoing,
        initial_ids,
    })
}

fn strongly_connected_components<S>(snapshot: &ReachableSnapshot<S>) -> Vec<Vec<usize>> {
    struct TarjanState {
        next_index: usize,
        index: Vec<Option<usize>>,
        lowlink: Vec<usize>,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        components: Vec<Vec<usize>>,
    }

    fn visit<S>(node: usize, snapshot: &ReachableSnapshot<S>, state: &mut TarjanState) {
        let node_index = state.next_index;
        state.next_index += 1;
        state.index[node] = Some(node_index);
        state.lowlink[node] = node_index;
        state.stack.push(node);
        state.on_stack[node] = true;

        for edge in &snapshot.outgoing[node] {
            let target = edge.target;
            if state.index[target].is_none() {
                visit(target, snapshot, state);
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

    let count = snapshot.states.len();
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
            visit(node, snapshot, &mut state);
        }
    }

    state
        .components
        .sort_by_key(|component| component.first().copied().unwrap_or(usize::MAX));
    state.components
}

fn component_is_cyclic<S>(snapshot: &ReachableSnapshot<S>, component: &[usize]) -> bool {
    component.len() > 1
        || component.first().is_some_and(|node| {
            snapshot.outgoing[*node]
                .iter()
                .any(|edge| edge.target == *node)
        })
}

fn cycle_witness<S: Clone + Eq>(
    snapshot: &ReachableSnapshot<S>,
    component_index: usize,
    component: &[usize],
) -> Result<Option<CycleWitness<S>>, RecurrenceError> {
    let Some(&entry) = component.first() else {
        return Ok(None);
    };
    let members = component.iter().copied().collect::<HashSet<_>>();
    let stem = shortest_path(snapshot, &snapshot.initial_ids, entry, None)
        .ok_or(RecurrenceError::CycleWitnessMissing)?;

    let first_internal_edge = snapshot.outgoing[entry]
        .iter()
        .find(|edge| members.contains(&edge.target))
        .ok_or(RecurrenceError::CycleWitnessMissing)?;

    let mut cycle = vec![TraceStep {
        action: None,
        state: snapshot.states[entry].clone(),
    }];
    cycle.push(TraceStep {
        action: Some(first_internal_edge.action.clone()),
        state: snapshot.states[first_internal_edge.target].clone(),
    });

    if first_internal_edge.target != entry {
        let return_path = shortest_path(
            snapshot,
            &[first_internal_edge.target],
            entry,
            Some(&members),
        )
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

fn shortest_path<S: Clone>(
    snapshot: &ReachableSnapshot<S>,
    starts: &[usize],
    goal: usize,
    allowed: Option<&HashSet<usize>>,
) -> Option<Vec<TraceStep<S>>> {
    let count = snapshot.states.len();
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
        for edge in &snapshot.outgoing[node] {
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
                state: snapshot.states[node].clone(),
            });
            break;
        }
        let (previous, action) = predecessor[node].clone()?;
        reversed.push(TraceStep {
            action: Some(action),
            state: snapshot.states[node].clone(),
        });
        node = previous;
    }
    reversed.reverse();
    Some(reversed)
}

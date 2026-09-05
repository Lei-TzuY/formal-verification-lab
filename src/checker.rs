use crate::model::{ModelError, TransitionSystem};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;

/// Whether all discovered reachable states satisfied all safety invariants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    Safe,
    Violated,
}

/// One state in a reconstructed path. `action` is the transition taken from
/// the previous state; it is `None` for an initial state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceStep<S> {
    pub action: Option<String>,
    pub state: S,
}

/// The first invariant violation encountered by deterministic BFS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterexample<S> {
    pub invariant: String,
    pub trace: Vec<TraceStep<S>>,
}

/// Summary returned by the semantic checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult<S> {
    pub status: VerificationStatus,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub counterexample: Option<Counterexample<S>>,
}

#[derive(Debug)]
struct Node<S> {
    state: S,
    predecessor: Option<usize>,
    action: Option<String>,
}

/// Explore the reachable state graph with deterministic breadth-first search.
///
/// BFS plus predecessor links guarantees that the first reported violating
/// state has a shortest transition-count path from any initial state. The
/// ordering of equally short traces follows initial-state order, invariant
/// order, and successor order supplied by the model.
pub fn check<S>(model: &TransitionSystem<S>) -> Result<CheckResult<S>, ModelError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    let mut nodes: Vec<Node<S>> = Vec::new();
    let mut node_by_state: HashMap<S, usize> = HashMap::new();
    let mut queue = VecDeque::new();

    for initial in model.initial_states() {
        if node_by_state.contains_key(initial) {
            continue;
        }
        let id = nodes.len();
        nodes.push(Node {
            state: initial.clone(),
            predecessor: None,
            action: None,
        });
        node_by_state.insert(initial.clone(), id);
        queue.push_back(id);
    }

    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(node_id) = queue.pop_front() {
        checked_states += 1;
        let state = &nodes[node_id].state;

        for invariant in model.invariants() {
            if !invariant.holds(state) {
                return Ok(CheckResult {
                    status: VerificationStatus::Violated,
                    discovered_states: nodes.len(),
                    checked_states,
                    explored_transitions,
                    counterexample: Some(Counterexample {
                        invariant: invariant.name().to_owned(),
                        trace: reconstruct_trace(&nodes, node_id),
                    }),
                });
            }
        }

        let transitions = model.successors(state)?;
        explored_transitions += transitions.len();

        for transition in transitions {
            if node_by_state.contains_key(&transition.next) {
                continue;
            }

            let id = nodes.len();
            node_by_state.insert(transition.next.clone(), id);
            nodes.push(Node {
                state: transition.next,
                predecessor: Some(node_id),
                action: Some(transition.action),
            });
            queue.push_back(id);
        }
    }

    Ok(CheckResult {
        status: VerificationStatus::Safe,
        discovered_states: nodes.len(),
        checked_states,
        explored_transitions,
        counterexample: None,
    })
}

fn reconstruct_trace<S: Clone>(nodes: &[Node<S>], mut node_id: usize) -> Vec<TraceStep<S>> {
    let mut reversed = Vec::new();

    loop {
        let node = &nodes[node_id];
        reversed.push(TraceStep {
            action: node.action.clone(),
            state: node.state.clone(),
        });

        match node.predecessor {
            Some(predecessor) => node_id = predecessor,
            None => break,
        }
    }

    reversed.reverse();
    reversed
}

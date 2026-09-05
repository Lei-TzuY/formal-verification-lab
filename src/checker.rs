use crate::model::{ModelError, TransitionSystem};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;

/// Whether exploration proved safety, found a violation, or stopped before a
/// proof was complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    Safe,
    Violated,
    Inconclusive,
}

/// Resource boundary that prevented exhaustive exploration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InconclusiveReason {
    StateLimitReached { limit: usize },
    TransitionLimitReached { limit: usize },
    DepthLimitReached { limit: usize },
}

/// Optional deterministic bounds for explicit-state exploration.
///
/// A limit is only reported when it actually prevents required work. Reaching
/// an exact bound is therefore still compatible with `Safe` when the reachable
/// graph is fully exhausted at that bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExplorationLimits {
    pub max_states: Option<usize>,
    pub max_transitions: Option<usize>,
    pub max_depth: Option<usize>,
}

impl ExplorationLimits {
    pub const fn unbounded() -> Self {
        Self {
            max_states: None,
            max_transitions: None,
            max_depth: None,
        }
    }
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
    /// Greatest BFS depth among states actually retained by the search.
    /// `None` is possible only when a zero-state budget prevents loading even
    /// the first initial state.
    pub max_depth_reached: Option<usize>,
    /// Number of examined transition edges grouped by action label. The map is
    /// ordered so reports and downstream consumers receive deterministic data.
    /// Its values always sum to `explored_transitions`.
    pub transitions_by_action: BTreeMap<String, usize>,
    pub counterexample: Option<Counterexample<S>>,
    pub inconclusive_reason: Option<InconclusiveReason>,
}

#[derive(Debug)]
struct Node<S> {
    state: S,
    predecessor: Option<usize>,
    action: Option<String>,
    depth: usize,
}

/// Explore the reachable state graph without resource bounds.
pub fn check<S>(model: &TransitionSystem<S>) -> Result<CheckResult<S>, ModelError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    check_with_limits(model, ExplorationLimits::unbounded())
}

/// Explore the reachable state graph with deterministic breadth-first search
/// and optional state, transition, and depth bounds.
///
/// BFS plus predecessor links guarantees that the first reported violating
/// state has a shortest transition-count path from any initial state. The
/// ordering of equally short traces follows initial-state order, invariant
/// order, and successor order supplied by the model.
///
/// Bounds never imply safety. If a bound prevents examining a required state
/// or transition, the result is `Inconclusive`. A bound that is reached exactly
/// after the complete reachable graph has been exhausted still permits `Safe`.
pub fn check_with_limits<S>(
    model: &TransitionSystem<S>,
    limits: ExplorationLimits,
) -> Result<CheckResult<S>, ModelError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    let mut nodes: Vec<Node<S>> = Vec::new();
    let mut node_by_state: HashMap<S, usize> = HashMap::new();
    let mut queue = VecDeque::new();
    let mut max_depth_reached = None;
    let mut transitions_by_action = BTreeMap::new();

    for initial in model.initial_states() {
        if node_by_state.contains_key(initial) {
            continue;
        }
        if let Some(limit) = limits.max_states.filter(|limit| nodes.len() >= *limit) {
            return Ok(inconclusive_result(
                nodes.len(),
                0,
                0,
                max_depth_reached,
                transitions_by_action,
                InconclusiveReason::StateLimitReached { limit },
            ));
        }

        let id = nodes.len();
        nodes.push(Node {
            state: initial.clone(),
            predecessor: None,
            action: None,
            depth: 0,
        });
        max_depth_reached = Some(0);
        node_by_state.insert(initial.clone(), id);
        queue.push_back(id);
    }

    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(node_id) = queue.pop_front() {
        checked_states += 1;
        let depth = nodes[node_id].depth;
        let state = &nodes[node_id].state;

        for invariant in model.invariants() {
            if !invariant.holds(state) {
                return Ok(CheckResult {
                    status: VerificationStatus::Violated,
                    discovered_states: nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    transitions_by_action,
                    counterexample: Some(Counterexample {
                        invariant: invariant.name().to_owned(),
                        trace: reconstruct_trace(&nodes, node_id),
                    }),
                    inconclusive_reason: None,
                });
            }
        }

        let transitions = model.successors(state)?;

        for transition in transitions {
            if let Some(limit) = limits
                .max_transitions
                .filter(|limit| explored_transitions >= *limit)
            {
                return Ok(inconclusive_result(
                    nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    transitions_by_action,
                    InconclusiveReason::TransitionLimitReached { limit },
                ));
            }
            explored_transitions += 1;
            *transitions_by_action
                .entry(transition.action.clone())
                .or_insert(0) += 1;

            if node_by_state.contains_key(&transition.next) {
                continue;
            }

            if let Some(limit) = limits.max_depth.filter(|limit| depth >= *limit) {
                return Ok(inconclusive_result(
                    nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    transitions_by_action,
                    InconclusiveReason::DepthLimitReached { limit },
                ));
            }

            if let Some(limit) = limits.max_states.filter(|limit| nodes.len() >= *limit) {
                return Ok(inconclusive_result(
                    nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    transitions_by_action,
                    InconclusiveReason::StateLimitReached { limit },
                ));
            }

            let id = nodes.len();
            let next_depth = depth + 1;
            node_by_state.insert(transition.next.clone(), id);
            nodes.push(Node {
                state: transition.next,
                predecessor: Some(node_id),
                action: Some(transition.action),
                depth: next_depth,
            });
            max_depth_reached = Some(max_depth_reached.map_or(next_depth, |max| max.max(next_depth)));
            queue.push_back(id);
        }
    }

    Ok(CheckResult {
        status: VerificationStatus::Safe,
        discovered_states: nodes.len(),
        checked_states,
        explored_transitions,
        max_depth_reached,
        transitions_by_action,
        counterexample: None,
        inconclusive_reason: None,
    })
}

fn inconclusive_result<S>(
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
    transitions_by_action: BTreeMap<String, usize>,
    reason: InconclusiveReason,
) -> CheckResult<S> {
    CheckResult {
        status: VerificationStatus::Inconclusive,
        discovered_states,
        checked_states,
        explored_transitions,
        max_depth_reached,
        transitions_by_action,
        counterexample: None,
        inconclusive_reason: Some(reason),
    }
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

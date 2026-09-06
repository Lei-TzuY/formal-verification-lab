use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, InconclusiveReason};
use crate::graph::{ReachableGraph, SnapshotEdge};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

#[derive(Debug, Clone)]
pub(crate) struct BoundedActionProduct<P> {
    pub(crate) graph: ReachableGraph<P>,
    pub(crate) checked_states: usize,
    pub(crate) explored_transitions: usize,
    pub(crate) max_depth_reached: Option<usize>,
    pub(crate) completion: BoundedOutcome<()>,
    /// True only for retained product states whose underlying model state is a
    /// real terminal in the complete captured model graph.
    pub(crate) known_terminal: Vec<bool>,
}

/// Build the reachable synchronous product of a captured labeled model graph
/// and one deterministic action-driven control state machine.
///
/// Product discovery is deterministic BFS. Initial model states keep their
/// captured order, outgoing model edges keep their captured order, and product
/// states are deduplicated by `(model_state_id, control_state)`. The model
/// transition relation is never invoked here; callers supply an already
/// captured graph and consume only its action labels.
///
/// `lift` preserves each caller's public product-state representation while the
/// shared substrate owns graph construction and ordering semantics.
pub(crate) fn build_action_product<S, C, P, Step, Lift>(
    graph: &ReachableGraph<S>,
    initial_control: &C,
    step: Step,
    lift: Lift,
) -> ReachableGraph<P>
where
    S: Clone,
    C: Clone + Eq + Hash,
    Step: Fn(&C, &str) -> C,
    Lift: Fn(S, C) -> P,
{
    let bounded = build_action_product_with_limits(
        graph,
        initial_control,
        step,
        lift,
        ExplorationLimits::unbounded(),
    );
    let BoundedActionProduct {
        graph, completion, ..
    } = bounded;
    match completion {
        BoundedOutcome::Conclusive(()) => graph,
        BoundedOutcome::Inconclusive(_) => {
            unreachable!("unbounded action-product construction cannot be inconclusive")
        }
    }
}

/// Materialize a deterministic prefix of an action product under product-space
/// state, transition, and depth limits.
///
/// The supplied model graph is already complete. Limits therefore apply only
/// to product construction, not to model capture. A counted transition whose
/// previously unseen target is blocked by a state/depth limit is not inserted
/// into the retained graph. `known_terminal` is derived from the complete model
/// graph, so a cutoff can never turn a partially expanded product state into a
/// fabricated finite terminal.
pub(crate) fn build_action_product_with_limits<S, C, P, Step, Lift>(
    graph: &ReachableGraph<S>,
    initial_control: &C,
    step: Step,
    lift: Lift,
    limits: ExplorationLimits,
) -> BoundedActionProduct<P>
where
    S: Clone,
    C: Clone + Eq + Hash,
    Step: Fn(&C, &str) -> C,
    Lift: Fn(S, C) -> P,
{
    let mut states = Vec::new();
    let mut outgoing: Vec<Vec<SnapshotEdge>> = Vec::new();
    let mut initial_ids = Vec::new();
    let mut known_terminal = Vec::new();
    let mut ids: HashMap<(usize, C), usize> = HashMap::new();
    let mut model_ids = Vec::new();
    let mut controls = Vec::new();
    let mut depths = Vec::new();
    let mut queue = VecDeque::new();
    let mut max_depth_reached = None;

    for &model_id in &graph.initial_ids {
        let control = initial_control.clone();
        let key = (model_id, control.clone());
        let product_id = if let Some(id) = ids.get(&key).copied() {
            id
        } else {
            if let Some(limit) = limits.max_states.filter(|limit| states.len() >= *limit) {
                return finish_product(
                    states,
                    outgoing,
                    initial_ids,
                    known_terminal,
                    0,
                    0,
                    max_depth_reached,
                    Some(InconclusiveReason::StateLimitReached { limit }),
                );
            }

            let id = states.len();
            ids.insert(key, id);
            states.push(lift(graph.states[model_id].clone(), control.clone()));
            outgoing.push(Vec::new());
            known_terminal.push(graph.outgoing[model_id].is_empty());
            model_ids.push(model_id);
            controls.push(control);
            depths.push(0_usize);
            queue.push_back(id);
            max_depth_reached = Some(0);
            id
        };
        if !initial_ids.contains(&product_id) {
            initial_ids.push(product_id);
        }
    }

    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(source) = queue.pop_front() {
        checked_states += 1;
        let model_id = model_ids[source];
        let depth = depths[source];

        for edge in &graph.outgoing[model_id] {
            if let Some(limit) = limits
                .max_transitions
                .filter(|limit| explored_transitions >= *limit)
            {
                return finish_product(
                    states,
                    outgoing,
                    initial_ids,
                    known_terminal,
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    Some(InconclusiveReason::TransitionLimitReached { limit }),
                );
            }
            explored_transitions += 1;

            let next_control = step(&controls[source], &edge.action);
            let key = (edge.target, next_control.clone());
            let target = if let Some(id) = ids.get(&key).copied() {
                id
            } else {
                if let Some(limit) = limits.max_depth.filter(|limit| depth >= *limit) {
                    return finish_product(
                        states,
                        outgoing,
                        initial_ids,
                        known_terminal,
                        checked_states,
                        explored_transitions,
                        max_depth_reached,
                        Some(InconclusiveReason::DepthLimitReached { limit }),
                    );
                }
                if let Some(limit) = limits.max_states.filter(|limit| states.len() >= *limit) {
                    return finish_product(
                        states,
                        outgoing,
                        initial_ids,
                        known_terminal,
                        checked_states,
                        explored_transitions,
                        max_depth_reached,
                        Some(InconclusiveReason::StateLimitReached { limit }),
                    );
                }

                let id = states.len();
                ids.insert(key, id);
                states.push(lift(
                    graph.states[edge.target].clone(),
                    next_control.clone(),
                ));
                outgoing.push(Vec::new());
                known_terminal.push(graph.outgoing[edge.target].is_empty());
                model_ids.push(edge.target);
                controls.push(next_control);
                depths.push(depth + 1);
                queue.push_back(id);
                max_depth_reached = Some(max_depth_reached.unwrap_or(0).max(depth + 1));
                id
            };

            outgoing[source].push(SnapshotEdge {
                action: edge.action.clone(),
                target,
            });
        }
    }

    finish_product(
        states,
        outgoing,
        initial_ids,
        known_terminal,
        checked_states,
        explored_transitions,
        max_depth_reached,
        None,
    )
}

fn finish_product<P>(
    states: Vec<P>,
    outgoing: Vec<Vec<SnapshotEdge>>,
    initial_ids: Vec<usize>,
    known_terminal: Vec<bool>,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
    reason: Option<InconclusiveReason>,
) -> BoundedActionProduct<P> {
    BoundedActionProduct {
        graph: ReachableGraph {
            states,
            outgoing,
            initial_ids,
        },
        checked_states,
        explored_transitions,
        max_depth_reached,
        completion: match reason {
            Some(reason) => BoundedOutcome::Inconclusive(reason),
            None => BoundedOutcome::Conclusive(()),
        },
        known_terminal,
    }
}

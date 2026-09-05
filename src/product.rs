use crate::recurrence::{ReachableGraph, SnapshotEdge};
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

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
    let mut states = Vec::new();
    let mut outgoing: Vec<Vec<SnapshotEdge>> = Vec::new();
    let mut initial_ids = Vec::new();
    let mut ids: HashMap<(usize, C), usize> = HashMap::new();
    let mut queue = VecDeque::new();

    for &model_id in &graph.initial_ids {
        let control = initial_control.clone();
        let key = (model_id, control.clone());
        let product_id = if let Some(id) = ids.get(&key).copied() {
            id
        } else {
            let id = states.len();
            ids.insert(key.clone(), id);
            states.push(lift(graph.states[model_id].clone(), control));
            outgoing.push(Vec::new());
            queue.push_back(key);
            id
        };
        if !initial_ids.contains(&product_id) {
            initial_ids.push(product_id);
        }
    }

    while let Some((model_id, control)) = queue.pop_front() {
        let source = ids[&(model_id, control.clone())];
        for edge in &graph.outgoing[model_id] {
            let next_control = step(&control, &edge.action);
            let key = (edge.target, next_control.clone());
            let target = if let Some(id) = ids.get(&key).copied() {
                id
            } else {
                let id = states.len();
                ids.insert(key.clone(), id);
                states.push(lift(graph.states[edge.target].clone(), next_control));
                outgoing.push(Vec::new());
                queue.push_back(key);
                id
            };
            outgoing[source].push(SnapshotEdge {
                action: edge.action.clone(),
                target,
            });
        }
    }

    ReachableGraph {
        states,
        outgoing,
        initial_ids,
    }
}

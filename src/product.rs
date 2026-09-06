use crate::bounded::{AnalysisLimits, BoundedOutcome};
use crate::checker::{ExplorationLimits, InconclusiveReason};
use crate::graph::{
    capture_reachable_graph_with_limits, GraphCaptureCompletion, GraphCaptureError, ReachableGraph,
    SnapshotEdge,
};
use crate::model::TransitionSystem;
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
    /// terminal justified by the model snapshot used to construct the product.
    pub(crate) known_terminal: Vec<bool>,
}

/// Shared result of staged bounded model capture followed by bounded product
/// construction. This is the M28 trust boundary reused by temporal consumers.
#[derive(Debug, Clone)]
pub(crate) struct StagedActionProduct<P> {
    pub(crate) product: BoundedActionProduct<P>,
    pub(crate) model_discovered_states: usize,
    pub(crate) model_checked_states: usize,
    pub(crate) model_explored_transitions: usize,
    pub(crate) model_retained_transitions: usize,
    pub(crate) model_max_depth_reached: Option<usize>,
    pub(crate) model_completion: BoundedOutcome<()>,
}

#[derive(Debug, Clone, Copy)]
struct ProductAccounting {
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
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
/// The supplied model graph is complete. Limits therefore apply only to product
/// construction. Terminal knowledge is derived from that complete graph.
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
    let known_terminal = graph.outgoing.iter().map(Vec::is_empty).collect::<Vec<_>>();
    build_action_product_from_prefix_with_limits(
        graph,
        &known_terminal,
        initial_control,
        step,
        lift,
        limits,
    )
}

/// Compose canonical bounded model capture with bounded action-product
/// construction without pretending that the captured model prefix is complete.
///
/// Real retained model edges may still support conclusive counterexamples.
/// Terminal facts are propagated explicitly from bounded model capture so a
/// missing edge caused by a model cutoff can never fabricate a product terminal.
pub(crate) fn build_action_product_with_analysis_limits<S, C, P, Step, Lift>(
    model: &TransitionSystem<S>,
    initial_control: &C,
    step: Step,
    lift: Lift,
    limits: AnalysisLimits,
) -> Result<StagedActionProduct<P>, GraphCaptureError>
where
    S: Clone + Eq + Hash,
    C: Clone + Eq + Hash,
    Step: Fn(&C, &str) -> C,
    Lift: Fn(S, C) -> P,
{
    let captured = capture_reachable_graph_with_limits(model, limits.model)?;
    let model_retained_transitions = captured.graph.outgoing.iter().map(Vec::len).sum();
    let model_completion = match captured.completion {
        GraphCaptureCompletion::Complete => BoundedOutcome::Conclusive(()),
        GraphCaptureCompletion::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };
    let product = build_action_product_from_prefix_with_limits(
        &captured.graph,
        &captured.known_terminal,
        initial_control,
        step,
        lift,
        limits.product,
    );

    Ok(StagedActionProduct {
        product,
        model_discovered_states: captured.discovered_states,
        model_checked_states: captured.checked_states,
        model_explored_transitions: captured.explored_transitions,
        model_retained_transitions,
        model_max_depth_reached: captured.max_depth_reached,
        model_completion,
    })
}

/// Build a product over a justified model prefix whose terminal knowledge is
/// supplied separately from the prefix's outgoing adjacency.
fn build_action_product_from_prefix_with_limits<S, C, P, Step, Lift>(
    graph: &ReachableGraph<S>,
    model_known_terminal: &[bool],
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
    assert_eq!(model_known_terminal.len(), graph.states.len());

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
                    ProductAccounting {
                        checked_states: 0,
                        explored_transitions: 0,
                        max_depth_reached,
                    },
                    Some(InconclusiveReason::StateLimitReached { limit }),
                );
            }

            let id = states.len();
            ids.insert(key, id);
            states.push(lift(graph.states[model_id].clone(), control.clone()));
            outgoing.push(Vec::new());
            known_terminal.push(model_known_terminal[model_id]);
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
                    ProductAccounting {
                        checked_states,
                        explored_transitions,
                        max_depth_reached,
                    },
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
                        ProductAccounting {
                            checked_states,
                            explored_transitions,
                            max_depth_reached,
                        },
                        Some(InconclusiveReason::DepthLimitReached { limit }),
                    );
                }
                if let Some(limit) = limits.max_states.filter(|limit| states.len() >= *limit) {
                    return finish_product(
                        states,
                        outgoing,
                        initial_ids,
                        known_terminal,
                        ProductAccounting {
                            checked_states,
                            explored_transitions,
                            max_depth_reached,
                        },
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
                known_terminal.push(model_known_terminal[edge.target]);
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
        ProductAccounting {
            checked_states,
            explored_transitions,
            max_depth_reached,
        },
        None,
    )
}

fn finish_product<P>(
    states: Vec<P>,
    outgoing: Vec<Vec<SnapshotEdge>>,
    initial_ids: Vec<usize>,
    known_terminal: Vec<bool>,
    accounting: ProductAccounting,
    reason: Option<InconclusiveReason>,
) -> BoundedActionProduct<P> {
    BoundedActionProduct {
        graph: ReachableGraph {
            states,
            outgoing,
            initial_ids,
        },
        checked_states: accounting.checked_states,
        explored_transitions: accounting.explored_transitions,
        max_depth_reached: accounting.max_depth_reached,
        completion: match reason {
            Some(reason) => BoundedOutcome::Inconclusive(reason),
            None => BoundedOutcome::Conclusive(()),
        },
        known_terminal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_graph() -> ReachableGraph<usize> {
        ReachableGraph {
            states: vec![0, 1],
            outgoing: vec![
                vec![SnapshotEdge {
                    action: "advance".to_owned(),
                    target: 1,
                }],
                Vec::new(),
            ],
            initial_ids: vec![0],
        }
    }

    fn self_loop_graph() -> ReachableGraph<usize> {
        ReachableGraph {
            states: vec![0],
            outgoing: vec![vec![SnapshotEdge {
                action: "tick".to_owned(),
                target: 0,
            }]],
            initial_ids: vec![0],
        }
    }

    fn build(
        graph: &ReachableGraph<usize>,
        limits: ExplorationLimits,
    ) -> BoundedActionProduct<(usize, bool)> {
        build_action_product_with_limits(
            graph,
            &false,
            |control, _action| *control,
            |state, control| (state, control),
            limits,
        )
    }

    #[test]
    fn zero_state_budget_blocks_the_first_initial_product_state() {
        let result = build(
            &chain_graph(),
            ExplorationLimits {
                max_states: Some(0),
                max_transitions: None,
                max_depth: None,
            },
        );

        assert_eq!(
            result.completion,
            BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 0 })
        );
        assert!(result.graph.states.is_empty());
        assert!(result.graph.initial_ids.is_empty());
        assert_eq!(result.checked_states, 0);
        assert_eq!(result.explored_transitions, 0);
        assert_eq!(result.max_depth_reached, None);
        assert!(result.known_terminal.is_empty());
    }

    #[test]
    fn transition_budget_stops_before_the_blocked_edge_is_counted_or_retained() {
        let result = build(
            &chain_graph(),
            ExplorationLimits {
                max_states: None,
                max_transitions: Some(0),
                max_depth: None,
            },
        );

        assert_eq!(
            result.completion,
            BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
        );
        assert_eq!(result.graph.states, vec![(0, false)]);
        assert!(result.graph.outgoing[0].is_empty());
        assert_eq!(result.checked_states, 1);
        assert_eq!(result.explored_transitions, 0);
        assert_eq!(result.max_depth_reached, Some(0));
        assert_eq!(result.known_terminal, vec![false]);
    }

    #[test]
    fn depth_budget_counts_the_edge_but_does_not_retain_its_unseen_target() {
        let result = build(
            &chain_graph(),
            ExplorationLimits {
                max_states: None,
                max_transitions: None,
                max_depth: Some(0),
            },
        );

        assert_eq!(
            result.completion,
            BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 0 })
        );
        assert_eq!(result.graph.states, vec![(0, false)]);
        assert!(result.graph.outgoing[0].is_empty());
        assert_eq!(result.checked_states, 1);
        assert_eq!(result.explored_transitions, 1);
        assert_eq!(result.max_depth_reached, Some(0));
        assert_eq!(result.known_terminal, vec![false]);
    }

    #[test]
    fn depth_budget_does_not_block_a_real_edge_to_an_existing_product_state() {
        let result = build(
            &self_loop_graph(),
            ExplorationLimits {
                max_states: Some(1),
                max_transitions: Some(1),
                max_depth: Some(0),
            },
        );

        assert_eq!(result.completion, BoundedOutcome::Conclusive(()));
        assert_eq!(result.graph.states, vec![(0, false)]);
        assert_eq!(result.graph.outgoing[0].len(), 1);
        assert_eq!(result.graph.outgoing[0][0].action, "tick");
        assert_eq!(result.graph.outgoing[0][0].target, 0);
        assert_eq!(result.checked_states, 1);
        assert_eq!(result.explored_transitions, 1);
        assert_eq!(result.max_depth_reached, Some(0));
        assert_eq!(result.known_terminal, vec![false]);
    }

    #[test]
    fn exact_limits_that_do_not_prevent_work_still_complete_the_product() {
        let result = build(
            &chain_graph(),
            ExplorationLimits {
                max_states: Some(2),
                max_transitions: Some(1),
                max_depth: Some(1),
            },
        );

        assert_eq!(result.completion, BoundedOutcome::Conclusive(()));
        assert_eq!(result.graph.states, vec![(0, false), (1, false)]);
        assert_eq!(result.graph.outgoing[0].len(), 1);
        assert!(result.graph.outgoing[1].is_empty());
        assert_eq!(result.checked_states, 2);
        assert_eq!(result.explored_transitions, 1);
        assert_eq!(result.max_depth_reached, Some(1));
        assert_eq!(result.known_terminal, vec![false, true]);
    }
}

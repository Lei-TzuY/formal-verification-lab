use formal_verification_lab::property::check_deadlock_with_limits;
use formal_verification_lab::{
    BoundedOutcome, DeadlockProperty, DeadlockStatus, ExplorationLimits, InconclusiveReason,
    Invariant, StateVariable, Transition, TransitionSystem,
};
use std::collections::{HashMap, VecDeque};

const N: usize = 3;
const EDGE_COUNT: usize = N * N;

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << (from * N + to)) != 0
}

fn graph_model(mask: usize) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("bounded-deadlock-{mask}"),
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        move |state| {
            let mut successors = Vec::new();
            for to in 0..N {
                if has_edge(mask, *state, to) {
                    successors.push(Transition::new(format!("{}->{to}", *state), to));
                }
            }
            Ok(successors)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleResult {
    outcome: BoundedOutcome<DeadlockStatus>,
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
    witness: Option<Vec<(Option<String>, usize)>>,
}

#[derive(Debug, Clone)]
struct OracleNode {
    state: usize,
    predecessor: Option<usize>,
    action: Option<String>,
    depth: usize,
}

fn oracle(
    graph_mask: usize,
    allowed_terminal_mask: usize,
    limits: ExplorationLimits,
) -> OracleResult {
    let mut nodes = Vec::<OracleNode>::new();
    let mut by_state = HashMap::<usize, usize>::new();
    let mut queue = VecDeque::new();
    let mut max_depth_reached = None;

    if let Some(limit) = limits.max_states.filter(|limit| nodes.len() >= *limit) {
        return OracleResult {
            outcome: BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached {
                limit,
            }),
            discovered_states: 0,
            checked_states: 0,
            explored_transitions: 0,
            max_depth_reached,
            witness: None,
        };
    }

    nodes.push(OracleNode {
        state: 0,
        predecessor: None,
        action: None,
        depth: 0,
    });
    by_state.insert(0, 0);
    queue.push_back(0);
    max_depth_reached = Some(0);

    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(source) = queue.pop_front() {
        checked_states += 1;
        let state = nodes[source].state;
        let depth = nodes[source].depth;
        let successors = (0..N)
            .filter(|to| has_edge(graph_mask, state, *to))
            .collect::<Vec<_>>();

        let allowed_terminal = allowed_terminal_mask & (1usize << state) != 0;
        if successors.is_empty() && !allowed_terminal {
            return OracleResult {
                outcome: BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFound),
                discovered_states: nodes.len(),
                checked_states,
                explored_transitions,
                max_depth_reached,
                witness: Some(reconstruct(&nodes, source)),
            };
        }

        for target_state in successors {
            if let Some(limit) = limits
                .max_transitions
                .filter(|limit| explored_transitions >= *limit)
            {
                return OracleResult {
                    outcome: BoundedOutcome::Inconclusive(
                        InconclusiveReason::TransitionLimitReached { limit },
                    ),
                    discovered_states: nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    witness: None,
                };
            }
            explored_transitions += 1;

            if by_state.contains_key(&target_state) {
                continue;
            }

            if let Some(limit) = limits.max_depth.filter(|limit| depth >= *limit) {
                return OracleResult {
                    outcome: BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached {
                        limit,
                    }),
                    discovered_states: nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    witness: None,
                };
            }

            if let Some(limit) = limits.max_states.filter(|limit| nodes.len() >= *limit) {
                return OracleResult {
                    outcome: BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached {
                        limit,
                    }),
                    discovered_states: nodes.len(),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                    witness: None,
                };
            }

            let id = nodes.len();
            nodes.push(OracleNode {
                state: target_state,
                predecessor: Some(source),
                action: Some(format!("{state}->{target_state}")),
                depth: depth + 1,
            });
            by_state.insert(target_state, id);
            queue.push_back(id);
            max_depth_reached = Some(max_depth_reached.unwrap_or(0).max(depth + 1));
        }
    }

    OracleResult {
        outcome: BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFree),
        discovered_states: nodes.len(),
        checked_states,
        explored_transitions,
        max_depth_reached,
        witness: None,
    }
}

fn reconstruct(nodes: &[OracleNode], mut id: usize) -> Vec<(Option<String>, usize)> {
    let mut reversed = Vec::new();
    loop {
        let node = &nodes[id];
        reversed.push((node.action.clone(), node.state));
        let Some(predecessor) = node.predecessor else {
            break;
        };
        id = predecessor;
    }
    reversed.reverse();
    reversed
}

fn profiles() -> [ExplorationLimits; 9] {
    [
        ExplorationLimits::unbounded(),
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_states: Some(1),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_transitions: Some(1),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_depth: Some(0),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_depth: Some(1),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(2),
            max_depth: Some(1),
        },
    ]
}

#[test]
fn unexpected_initial_terminal_is_conclusive_even_with_zero_transition_budget() {
    let model = graph_model(0);
    let property = DeadlockProperty::new("strict", |_state: &usize| false).unwrap();
    let result = check_deadlock_with_limits(
        &model,
        &property,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFound)
    );
    assert_eq!(result.explored_transitions, 0);
    assert_eq!(result.witness.unwrap().len(), 1);
}

#[test]
fn cutoff_during_nonterminal_expansion_does_not_fabricate_deadlock() {
    let model = graph_model(1usize << 1); // 0 -> 1; node 0 is not a terminal.
    let property = DeadlockProperty::new("strict", |_state: &usize| false).unwrap();
    let result = check_deadlock_with_limits(
        &model,
        &property,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
    );
    assert!(result.witness.is_none());
}

#[test]
fn exact_limits_can_still_prove_deadlock_free() {
    let model = graph_model(1usize << 0); // reachable graph is one self-loop state.
    let property = DeadlockProperty::new("strict", |_state: &usize| false).unwrap();
    let result = check_deadlock_with_limits(
        &model,
        &property,
        ExplorationLimits {
            max_states: Some(1),
            max_transitions: Some(1),
            max_depth: Some(0),
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFree)
    );
    assert_eq!(result.discovered_states, 1);
    assert_eq!(result.explored_transitions, 1);
}

#[test]
fn all_three_node_graphs_terminal_policies_and_limits_match_independent_oracle() {
    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for allowed_terminal_mask in 0..(1usize << N) {
            let model = graph_model(graph_mask);
            let property = DeadlockProperty::new(
                "generated-policy",
                move |state: &usize| allowed_terminal_mask & (1usize << *state) != 0,
            )
            .unwrap();

            for limits in profiles() {
                let expected = oracle(graph_mask, allowed_terminal_mask, limits);
                let first = check_deadlock_with_limits(&model, &property, limits).unwrap();
                let second = check_deadlock_with_limits(&model, &property, limits).unwrap();
                let context = format!(
                    "graph={graph_mask} allowed={allowed_terminal_mask} limits={limits:?}"
                );

                assert_eq!(first, second, "determinism {context}");
                assert_eq!(first.outcome, expected.outcome, "outcome {context}");
                assert_eq!(
                    first.discovered_states, expected.discovered_states,
                    "discovered {context}"
                );
                assert_eq!(first.checked_states, expected.checked_states, "checked {context}");
                assert_eq!(
                    first.explored_transitions, expected.explored_transitions,
                    "transitions {context}"
                );
                assert_eq!(
                    first.max_depth_reached, expected.max_depth_reached,
                    "depth {context}"
                );

                let actual_witness = first.witness.as_ref().map(|trace| {
                    trace
                        .iter()
                        .map(|step| (step.action.clone(), step.state))
                        .collect::<Vec<_>>()
                });
                assert_eq!(actual_witness, expected.witness, "witness {context}");

                if let Some(trace) = &first.witness {
                    let terminal = trace.last().unwrap().state;
                    assert!(
                        (0..N).all(|to| !has_edge(graph_mask, terminal, to)),
                        "reported state must be a real terminal {context}"
                    );
                    assert_eq!(
                        allowed_terminal_mask & (1usize << terminal),
                        0,
                        "reported state must violate policy {context}"
                    );
                }
            }
        }
    }
}

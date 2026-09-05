use formal_verification_lab::checker::{check, check_with_limits, ExplorationLimits, VerificationStatus};
use formal_verification_lab::model::{Invariant, StateVariable, Transition, TransitionSystem};
use std::collections::BTreeMap;

const N: usize = 3;
const EDGE_COUNT: usize = N * N;
const INF: usize = usize::MAX / 4;

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << (from * N + to)) != 0
}

fn all_pairs_shortest_paths(mask: usize) -> [[usize; N]; N] {
    let mut distance = [[INF; N]; N];

    for (node, row) in distance.iter_mut().enumerate() {
        row[node] = 0;
    }
    for (from, row) in distance.iter_mut().enumerate() {
        for (to, value) in row.iter_mut().enumerate() {
            if has_edge(mask, from, to) {
                *value = (*value).min(1);
            }
        }
    }

    for via in 0..N {
        for from in 0..N {
            for to in 0..N {
                let through = distance[from][via].saturating_add(distance[via][to]);
                distance[from][to] = distance[from][to].min(through);
            }
        }
    }

    distance
}

fn graph_model(mask: usize, invariant: Invariant<usize>) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("graph-{mask}"),
        vec![StateVariable::new("node", "current graph node")],
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
        vec![invariant],
    )
    .unwrap()
}

#[test]
fn exhaustive_three_node_graphs_match_independent_reachability_and_accounting_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let distances = all_pairs_shortest_paths(mask);
        let model = graph_model(mask, Invariant::new("always", |_state: &usize| true));
        let result = check(&model).unwrap();

        assert_eq!(result.status, VerificationStatus::Safe, "mask={mask}");

        let reachable: Vec<_> = (0..N).filter(|node| distances[0][*node] < INF).collect();
        assert_eq!(result.discovered_states, reachable.len(), "mask={mask}");
        assert_eq!(result.checked_states, reachable.len(), "mask={mask}");

        let expected_depth = reachable
            .iter()
            .map(|node| distances[0][*node])
            .max()
            .unwrap();
        assert_eq!(result.max_depth_reached, Some(expected_depth), "mask={mask}");

        let mut expected_actions = BTreeMap::new();
        for from in &reachable {
            for to in 0..N {
                if has_edge(mask, *from, to) {
                    expected_actions.insert(format!("{from}->{to}"), 1usize);
                }
            }
        }
        let expected_edges: usize = expected_actions.values().sum();
        assert_eq!(result.explored_transitions, expected_edges, "mask={mask}");
        assert_eq!(result.transitions_by_action, expected_actions, "mask={mask}");
        assert_eq!(
            result.transitions_by_action.values().sum::<usize>(),
            result.explored_transitions,
            "mask={mask}"
        );
    }
}

#[test]
fn exhaustive_three_node_graphs_match_independent_shortest_counterexample_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let distances = all_pairs_shortest_paths(mask);
        let model = graph_model(
            mask,
            Invariant::new("avoid-node-two", |state: &usize| *state != 2),
        );
        let first = check(&model).unwrap();
        let second = check(&model).unwrap();

        assert_eq!(first, second, "determinism failed for mask={mask}");

        if distances[0][2] < INF {
            assert_eq!(first.status, VerificationStatus::Violated, "mask={mask}");
            let trace = first.counterexample.expect("reachable target must violate");
            assert_eq!(trace.invariant, "avoid-node-two", "mask={mask}");
            assert_eq!(trace.trace.len() - 1, distances[0][2], "mask={mask}");
            assert_eq!(trace.trace.last().unwrap().state, 2, "mask={mask}");

            for pair in trace.trace.windows(2) {
                let from = pair[0].state;
                let to = pair[1].state;
                assert!(has_edge(mask, from, to), "trace used missing edge for mask={mask}");
                let expected_action = format!("{from}->{to}");
                assert_eq!(
                    pair[1].action.as_deref(),
                    Some(expected_action.as_str()),
                    "mask={mask}"
                );
            }
        } else {
            assert_eq!(first.status, VerificationStatus::Safe, "mask={mask}");
            assert!(first.counterexample.is_none(), "mask={mask}");
        }
    }
}

#[test]
fn diagnostic_accounting_respects_resource_boundaries() {
    let model = TransitionSystem::new(
        "bounded-edge",
        vec![StateVariable::new("value", "small counter")],
        vec![0usize],
        |state| {
            if *state < 2 {
                Ok(vec![Transition::new("increment", *state + 1)])
            } else {
                Ok(Vec::new())
            }
        },
        vec![Invariant::new("bounded", |state: &usize| *state <= 2)],
    )
    .unwrap();

    let no_initial = check_with_limits(
        &model,
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();
    assert_eq!(no_initial.status, VerificationStatus::Inconclusive);
    assert_eq!(no_initial.discovered_states, 0);
    assert_eq!(no_initial.max_depth_reached, None);
    assert!(no_initial.transitions_by_action.is_empty());

    let no_edge = check_with_limits(
        &model,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();
    assert_eq!(no_edge.status, VerificationStatus::Inconclusive);
    assert_eq!(no_edge.discovered_states, 1);
    assert_eq!(no_edge.max_depth_reached, Some(0));
    assert_eq!(no_edge.explored_transitions, 0);
    assert!(no_edge.transitions_by_action.is_empty());

    let blocked_successor = check_with_limits(
        &model,
        ExplorationLimits {
            max_depth: Some(0),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();
    assert_eq!(blocked_successor.status, VerificationStatus::Inconclusive);
    assert_eq!(blocked_successor.discovered_states, 1);
    assert_eq!(blocked_successor.max_depth_reached, Some(0));
    assert_eq!(blocked_successor.explored_transitions, 1);
    assert_eq!(
        blocked_successor.transitions_by_action.get("increment"),
        Some(&1)
    );
}

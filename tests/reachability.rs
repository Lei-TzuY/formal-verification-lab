use formal_verification_lab::examples::{bounded_counter, CounterState};
use formal_verification_lab::{
    check_reachability, Invariant, ReachabilityError, ReachabilityProperty, ReachabilityStatus,
    StateVariable, Transition, TransitionSystem,
};

const N: usize = 3;
const EDGE_COUNT: usize = N * N;
const INF: usize = usize::MAX / 4;

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << (from * N + to)) != 0
}

fn shortest_paths_from_zero(mask: usize) -> [usize; N] {
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
    distance[0]
}

fn graph_model(mask: usize) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("reachability-graph-{mask}"),
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

#[test]
fn reachability_property_names_are_validated() {
    assert!(matches!(
        ReachabilityProperty::<usize>::new("  ", |_| true),
        Err(ReachabilityError::EmptyPropertyName)
    ));
}

#[test]
fn bounded_counter_returns_shortest_reachability_witness() {
    let model = bounded_counter().unwrap();
    let property = ReachabilityProperty::new("reaches-three", |state: &CounterState| {
        state.value == 3
    })
    .unwrap();
    let result = check_reachability(&model, &property).unwrap();

    assert_eq!(result.status, ReachabilityStatus::Reachable);
    assert_eq!(result.property, "reaches-three");
    assert_eq!(result.discovered_states, 4);
    assert_eq!(result.checked_states, 4);
    assert_eq!(result.explored_transitions, 3);
    assert_eq!(result.max_depth_reached, Some(3));

    let witness = result.witness.unwrap();
    assert_eq!(witness.len(), 4);
    assert_eq!(witness.first().unwrap().action, None);
    assert_eq!(witness.last().unwrap().state.value, 3);
    assert_eq!(
        witness
            .iter()
            .skip(1)
            .map(|step| step.action.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("increment"), Some("increment"), Some("increment")]
    );
}

#[test]
fn initial_target_has_zero_transition_witness() {
    let model = bounded_counter().unwrap();
    let property = ReachabilityProperty::new("starts-at-zero", |state: &CounterState| {
        state.value == 0
    })
    .unwrap();
    let result = check_reachability(&model, &property).unwrap();

    assert_eq!(result.status, ReachabilityStatus::Reachable);
    assert_eq!(result.checked_states, 1);
    assert_eq!(result.explored_transitions, 0);
    let witness = result.witness.unwrap();
    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0].state.value, 0);
    assert!(witness[0].action.is_none());
}

#[test]
fn unreachable_target_requires_exhausting_reachable_graph() {
    let model = bounded_counter().unwrap();
    let property = ReachabilityProperty::new("reaches-four", |state: &CounterState| {
        state.value == 4
    })
    .unwrap();
    let result = check_reachability(&model, &property).unwrap();

    assert_eq!(result.status, ReachabilityStatus::Unreachable);
    assert_eq!(result.discovered_states, 4);
    assert_eq!(result.checked_states, 4);
    assert_eq!(result.explored_transitions, 3);
    assert_eq!(result.max_depth_reached, Some(3));
    assert!(result.witness.is_none());
}

#[test]
fn reachability_query_does_not_abort_on_models_original_safety_invariants() {
    let model = TransitionSystem::new(
        "query-semantics",
        vec![StateVariable::new("value", "query state")],
        vec![0u8],
        |state| {
            if *state == 0 {
                Ok(vec![Transition::new("advance", 1)])
            } else {
                Ok(Vec::new())
            }
        },
        vec![Invariant::new("deliberately-false-at-zero", |state: &u8| {
            *state != 0
        })],
    )
    .unwrap();
    let property = ReachabilityProperty::new("reaches-one", |state: &u8| *state == 1).unwrap();
    let result = check_reachability(&model, &property).unwrap();

    assert_eq!(result.status, ReachabilityStatus::Reachable);
    assert_eq!(result.witness.unwrap().len(), 2);
}

#[test]
fn all_three_node_graphs_match_independent_reachability_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let distances = shortest_paths_from_zero(mask);
        let model = graph_model(mask);
        let property = ReachabilityProperty::new("reach-node-two", |state: &usize| *state == 2)
            .unwrap();
        let first = check_reachability(&model, &property).unwrap();
        let second = check_reachability(&model, &property).unwrap();

        assert_eq!(first, second, "determinism failed for mask={mask}");

        if distances[2] < INF {
            assert_eq!(
                first.status,
                ReachabilityStatus::Reachable,
                "mask={mask}"
            );
            let witness = first.witness.expect("reachable target needs witness");
            assert_eq!(witness.len() - 1, distances[2], "mask={mask}");
            assert_eq!(witness.last().unwrap().state, 2, "mask={mask}");
            for pair in witness.windows(2) {
                let from = pair[0].state;
                let to = pair[1].state;
                assert!(has_edge(mask, from, to), "missing witness edge mask={mask}");
            }
        } else {
            assert_eq!(
                first.status,
                ReachabilityStatus::Unreachable,
                "mask={mask}"
            );
            assert!(first.witness.is_none(), "mask={mask}");
            let reachable_count = distances.iter().filter(|distance| **distance < INF).count();
            assert_eq!(first.discovered_states, reachable_count, "mask={mask}");
            assert_eq!(first.checked_states, reachable_count, "mask={mask}");
        }
    }
}

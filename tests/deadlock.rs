use formal_verification_lab::examples::{bounded_counter, CounterState};
use formal_verification_lab::{
    check_deadlock, DeadlockError, DeadlockProperty, DeadlockStatus, Invariant, StateVariable,
    Transition, TransitionSystem,
};

const N: usize = 3;
const EDGE_COUNT: usize = N * N;
const INF: usize = usize::MAX / 4;

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << (from * N + to)) != 0
}

fn out_degree(mask: usize, node: usize) -> usize {
    (0..N).filter(|to| has_edge(mask, node, *to)).count()
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
        format!("deadlock-graph-{mask}"),
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
fn deadlock_property_names_are_validated() {
    assert!(matches!(
        DeadlockProperty::<usize>::new("  ", |_| false),
        Err(DeadlockError::EmptyPropertyName)
    ));
}

#[test]
fn bounded_counter_legitimate_terminal_is_deadlock_free() {
    let model = bounded_counter().unwrap();
    let property = DeadlockProperty::new("counter-completion-is-terminal", |state: &CounterState| {
        state.value == 3
    })
    .unwrap();
    let result = check_deadlock(&model, &property).unwrap();

    assert_eq!(result.status, DeadlockStatus::DeadlockFree);
    assert_eq!(result.property, "counter-completion-is-terminal");
    assert_eq!(result.discovered_states, 4);
    assert_eq!(result.checked_states, 4);
    assert_eq!(result.explored_transitions, 3);
    assert_eq!(result.max_depth_reached, Some(3));
    assert!(result.witness.is_none());
}

#[test]
fn bounded_counter_strict_policy_returns_shortest_deadlock_witness() {
    let model = bounded_counter().unwrap();
    let property = DeadlockProperty::new("no-terminal-state-is-allowed", |_state: &CounterState| false)
        .unwrap();
    let result = check_deadlock(&model, &property).unwrap();

    assert_eq!(result.status, DeadlockStatus::DeadlockFound);
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
fn initial_unexpected_terminal_has_zero_transition_witness() {
    let model = TransitionSystem::new(
        "initial-deadlock",
        vec![StateVariable::new("state", "single terminal state")],
        vec![0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("valid", |state: &u8| *state == 0)],
    )
    .unwrap();
    let property = DeadlockProperty::new("no-terminal", |_state: &u8| false).unwrap();
    let result = check_deadlock(&model, &property).unwrap();

    assert_eq!(result.status, DeadlockStatus::DeadlockFound);
    assert_eq!(result.discovered_states, 1);
    assert_eq!(result.checked_states, 1);
    assert_eq!(result.explored_transitions, 0);
    assert_eq!(result.max_depth_reached, Some(0));
    let witness = result.witness.unwrap();
    assert_eq!(witness.len(), 1);
    assert_eq!(witness[0].state, 0);
    assert!(witness[0].action.is_none());
}

#[test]
fn deadlock_query_does_not_abort_on_models_original_safety_invariants() {
    let model = TransitionSystem::new(
        "deadlock-query-semantics",
        vec![StateVariable::new("value", "query state")],
        vec![0u8],
        |state| {
            if *state == 0 {
                Ok(vec![Transition::new("advance", 1)])
            } else {
                Ok(Vec::new())
            }
        },
        vec![Invariant::new(
            "deliberately-false-at-zero",
            |state: &u8| *state != 0,
        )],
    )
    .unwrap();
    let property = DeadlockProperty::new("find-terminal", |_state: &u8| false).unwrap();
    let result = check_deadlock(&model, &property).unwrap();

    assert_eq!(result.status, DeadlockStatus::DeadlockFound);
    let witness = result.witness.unwrap();
    assert_eq!(witness.len(), 2);
    assert_eq!(witness.last().unwrap().state, 1);
}

#[test]
fn all_three_node_graphs_match_independent_deadlock_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let distances = shortest_paths_from_zero(mask);
        let model = graph_model(mask);
        let property = DeadlockProperty::new("no-terminal", |_state: &usize| false).unwrap();
        let first = check_deadlock(&model, &property).unwrap();
        let second = check_deadlock(&model, &property).unwrap();

        assert_eq!(first, second, "determinism failed for mask={mask}");

        let expected_distance = (0..N)
            .filter(|node| distances[*node] < INF && out_degree(mask, *node) == 0)
            .map(|node| distances[node])
            .min();

        match expected_distance {
            Some(distance) => {
                assert_eq!(first.status, DeadlockStatus::DeadlockFound, "mask={mask}");
                let witness = first.witness.expect("reachable deadlock needs witness");
                assert_eq!(witness.len() - 1, distance, "mask={mask}");
                let terminal = witness.last().unwrap().state;
                assert_eq!(out_degree(mask, terminal), 0, "mask={mask}");
                for pair in witness.windows(2) {
                    let from = pair[0].state;
                    let to = pair[1].state;
                    assert!(has_edge(mask, from, to), "missing witness edge mask={mask}");
                }
            }
            None => {
                assert_eq!(first.status, DeadlockStatus::DeadlockFree, "mask={mask}");
                assert!(first.witness.is_none(), "mask={mask}");
                let reachable_count = distances.iter().filter(|distance| **distance < INF).count();
                let reachable_edges = (0..N)
                    .filter(|node| distances[*node] < INF)
                    .map(|node| out_degree(mask, node))
                    .sum::<usize>();
                assert_eq!(first.discovered_states, reachable_count, "mask={mask}");
                assert_eq!(first.checked_states, reachable_count, "mask={mask}");
                assert_eq!(first.explored_transitions, reachable_edges, "mask={mask}");
            }
        }
    }
}

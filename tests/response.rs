use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol,
};
use formal_verification_lab::{
    check_response, Invariant, ObligationState, ResponseCounterexample, ResponseProperty,
    ResponseStatus, StateVariable, TraceStep, Transition, TransitionSystem,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const PRODUCT_N: usize = N * 2;
const INF: usize = usize::MAX / 4;

fn edge_index(from: usize, to: usize) -> usize {
    from * N + to
}

fn has_bit(mask: usize, bit: usize) -> bool {
    mask & (1usize << bit) != 0
}

fn action_index(action: &str) -> usize {
    action
        .strip_prefix('e')
        .expect("generated action uses eN form")
        .parse()
        .expect("generated action index is numeric")
}

fn graph_model(graph_mask: usize) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("response-graph-{graph_mask}"),
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                let edge = edge_index(*state, to);
                if has_bit(graph_mask, edge) {
                    next.push(Transition::new(format!("e{edge}"), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

fn product_index(node: usize, pending: bool) -> usize {
    node * 2 + usize::from(pending)
}

fn decode_product(index: usize) -> (usize, bool) {
    (index / 2, index % 2 == 1)
}

fn monitor_next(pending: bool, trigger: bool, response: bool) -> bool {
    if response {
        false
    } else {
        pending || trigger
    }
}

fn oracle_adjacency(
    graph_mask: usize,
    trigger_mask: usize,
    response_mask: usize,
) -> [[bool; PRODUCT_N]; PRODUCT_N] {
    let mut adjacency = [[false; PRODUCT_N]; PRODUCT_N];
    for node in 0..N {
        for pending in [false, true] {
            let from = product_index(node, pending);
            for to in 0..N {
                let edge = edge_index(node, to);
                if !has_bit(graph_mask, edge) {
                    continue;
                }
                let next_pending = monitor_next(
                    pending,
                    has_bit(trigger_mask, edge),
                    has_bit(response_mask, edge),
                );
                adjacency[from][product_index(to, next_pending)] = true;
            }
        }
    }
    adjacency
}

fn floyd(adjacency: &[[bool; PRODUCT_N]; PRODUCT_N]) -> [[usize; PRODUCT_N]; PRODUCT_N] {
    let mut distance = [[INF; PRODUCT_N]; PRODUCT_N];
    for (node, row) in distance.iter_mut().enumerate() {
        row[node] = 0;
    }
    for (from, row) in adjacency.iter().enumerate() {
        for (to, edge) in row.iter().enumerate() {
            if *edge {
                distance[from][to] = 1;
            }
        }
    }
    for via in 0..PRODUCT_N {
        for from in 0..PRODUCT_N {
            for to in 0..PRODUCT_N {
                let through = distance[from][via].saturating_add(distance[via][to]);
                distance[from][to] = distance[from][to].min(through);
            }
        }
    }
    distance
}

fn pending_floyd(
    adjacency: &[[bool; PRODUCT_N]; PRODUCT_N],
) -> [[usize; PRODUCT_N]; PRODUCT_N] {
    let mut distance = [[INF; PRODUCT_N]; PRODUCT_N];
    for index in 0..PRODUCT_N {
        let (_, pending) = decode_product(index);
        if pending {
            distance[index][index] = 0;
        }
    }
    for from in 0..PRODUCT_N {
        let (_, from_pending) = decode_product(from);
        if !from_pending {
            continue;
        }
        for to in 0..PRODUCT_N {
            let (_, to_pending) = decode_product(to);
            if to_pending && adjacency[from][to] {
                distance[from][to] = distance[from][to].min(1);
            }
        }
    }
    for via in 0..PRODUCT_N {
        for from in 0..PRODUCT_N {
            for to in 0..PRODUCT_N {
                let through = distance[from][via].saturating_add(distance[via][to]);
                distance[from][to] = distance[from][to].min(through);
            }
        }
    }
    distance
}

#[test]
fn deterministic_request_grant_protocol_satisfies_response() {
    let model = request_grant_protocol().unwrap();
    let property = ResponseProperty::new(
        "request-eventually-grant",
        |action| action == "request",
        |action| action == "grant",
    )
    .unwrap();

    let result = check_response(&model, &property).unwrap();
    assert_eq!(result.status, ResponseStatus::Satisfied);
    assert!(result.counterexample.is_none());
}

#[test]
fn enabled_grant_does_not_save_unfair_wait_cycle() {
    let model = unfair_request_grant_protocol().unwrap();
    let property = ResponseProperty::new(
        "request-eventually-grant",
        |action| action == "request",
        |action| action == "grant",
    )
    .unwrap();

    let result = check_response(&model, &property).unwrap();
    assert_eq!(result.status, ResponseStatus::Violated);
    let ResponseCounterexample::Infinite { stem, cycle } = result.counterexample.unwrap() else {
        panic!("expected pending cycle counterexample");
    };
    assert!(stem.last().unwrap().state.pending);
    assert!(cycle.iter().all(|step| step.state.pending));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle.iter().skip(1).all(|step| step.action.as_deref() == Some("wait")));
}

#[test]
fn unanswered_request_can_terminate_finitely() {
    let model = TransitionSystem::new(
        "finite-unanswered-request",
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("request", 1)]),
            1 => Ok(Vec::new()),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let property = ResponseProperty::new(
        "request-eventually-grant",
        |action| action == "request",
        |action| action == "grant",
    )
    .unwrap();

    let result = check_response(&model, &property).unwrap();
    let ResponseCounterexample::Finite { trace } = result.counterexample.unwrap() else {
        panic!("expected finite pending counterexample");
    };
    assert_eq!(trace.len(), 2);
    assert!(!trace[0].state.pending);
    assert!(trace[1].state.pending);
}

#[test]
fn response_wins_when_one_action_is_both_trigger_and_response() {
    let model = TransitionSystem::new(
        "instant-response",
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("request-and-grant", 1)]),
            1 => Ok(Vec::new()),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let property = ResponseProperty::new(
        "instant-response",
        |action| action == "request-and-grant",
        |action| action == "request-and-grant",
    )
    .unwrap();

    let result = check_response(&model, &property).unwrap();
    assert_eq!(result.status, ResponseStatus::Satisfied);
}

#[test]
fn response_property_names_are_validated() {
    assert!(ResponseProperty::new("   ", |_| true, |_| false).is_err());
}

#[test]
fn all_two_node_graph_and_action_classifications_match_product_oracle() {
    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for trigger_mask in 0..(1usize << EDGE_COUNT) {
            for response_mask in 0..(1usize << EDGE_COUNT) {
                let model = graph_model(graph_mask);
                let property = ResponseProperty::new(
                    format!("response-{trigger_mask}-{response_mask}"),
                    move |action| has_bit(trigger_mask, action_index(action)),
                    move |action| has_bit(response_mask, action_index(action)),
                )
                .unwrap();
                let first = check_response(&model, &property).unwrap();
                let second = check_response(&model, &property).unwrap();
                assert_eq!(
                    first, second,
                    "determinism graph={graph_mask} trigger={trigger_mask} response={response_mask}"
                );

                let adjacency = oracle_adjacency(graph_mask, trigger_mask, response_mask);
                let distance = floyd(&adjacency);
                let pending_distance = pending_floyd(&adjacency);
                let initial = product_index(0, false);

                let has_pending_terminal = (0..PRODUCT_N).any(|product| {
                    let (node, pending) = decode_product(product);
                    pending
                        && distance[initial][product] < INF
                        && (0..N).all(|to| !has_bit(graph_mask, edge_index(node, to)))
                });

                let has_pending_cycle = (0..PRODUCT_N).any(|product| {
                    let (_, pending) = decode_product(product);
                    if !pending || distance[initial][product] >= INF {
                        return false;
                    }
                    adjacency[product][product]
                        || (0..PRODUCT_N).any(|other| {
                            other != product
                                && decode_product(other).1
                                && distance[initial][other] < INF
                                && pending_distance[product][other] < INF
                                && pending_distance[other][product] < INF
                        })
                });

                let expected_violation = has_pending_terminal || has_pending_cycle;
                assert_eq!(
                    first.status == ResponseStatus::Violated,
                    expected_violation,
                    "status graph={graph_mask} trigger={trigger_mask} response={response_mask}"
                );

                match first.counterexample {
                    None => assert!(!expected_violation),
                    Some(ResponseCounterexample::Finite { trace }) => {
                        assert!(has_pending_terminal);
                        validate_product_trace(
                            graph_mask,
                            trigger_mask,
                            response_mask,
                            &trace,
                        );
                        let end = &trace.last().unwrap().state;
                        assert!(end.pending);
                        assert!((0..N).all(|to| {
                            !has_bit(graph_mask, edge_index(end.state, to))
                        }));
                        let product = product_index(end.state, end.pending);
                        assert_eq!(trace.len() - 1, distance[initial][product]);
                    }
                    Some(ResponseCounterexample::Infinite { stem, cycle }) => {
                        assert!(!has_pending_terminal, "finite failures take precedence");
                        assert!(has_pending_cycle);
                        validate_product_trace(
                            graph_mask,
                            trigger_mask,
                            response_mask,
                            &stem,
                        );
                        validate_product_trace(
                            graph_mask,
                            trigger_mask,
                            response_mask,
                            &cycle,
                        );
                        assert!(cycle.iter().all(|step| step.state.pending));
                        assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
                        assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
                        let entry = &cycle.first().unwrap().state;
                        let product = product_index(entry.state, entry.pending);
                        assert_eq!(stem.len() - 1, distance[initial][product]);
                    }
                }
            }
        }
    }
}

fn validate_product_trace(
    graph_mask: usize,
    trigger_mask: usize,
    response_mask: usize,
    trace: &[TraceStep<ObligationState<usize>>],
) {
    assert!(!trace.is_empty());
    for pair in trace.windows(2) {
        let from = &pair[0].state;
        let to = &pair[1].state;
        let action = pair[1].action.as_deref().expect("non-root trace step has action");
        let edge = action_index(action);
        assert_eq!(edge, edge_index(from.state, to.state));
        assert!(has_bit(graph_mask, edge));
        assert_eq!(
            to.pending,
            monitor_next(
                from.pending,
                has_bit(trigger_mask, edge),
                has_bit(response_mask, edge)
            )
        );
    }
}

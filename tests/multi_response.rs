use formal_verification_lab::multi_response_examples::{
    dual_response_protocol, unfair_dual_response_protocol,
};
use formal_verification_lab::{
    check_multi_response, Invariant, MultiObligationState, MultiResponseCounterexample,
    MultiResponseProperty, MultiResponseStatus, ResponseClause, StateVariable, TraceStep,
    Transition, TransitionSystem,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const PENDING_VALUES: usize = 4;
const PRODUCT_N: usize = N * PENDING_VALUES;
const CODE_COUNT: usize = 7;
const ASSIGNMENT_COUNT: usize = CODE_COUNT.pow(EDGE_COUNT as u32);
const INF: usize = usize::MAX / 4;

fn dual_property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "dual-request-response",
        vec![
            ResponseClause::new(
                "class-a",
                |action| action == "request-a",
                |action| action == "grant-a",
            )
            .unwrap(),
            ResponseClause::new(
                "class-b",
                |action| action == "request-b",
                |action| action == "grant-b",
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn edge_index(from: usize, to: usize) -> usize {
    from * N + to
}

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << edge_index(from, to)) != 0
}

fn decode_assignment(mut assignment: usize) -> [u8; EDGE_COUNT] {
    let mut codes = [0u8; EDGE_COUNT];
    for code in &mut codes {
        *code = (assignment % CODE_COUNT) as u8;
        assignment /= CODE_COUNT;
    }
    codes
}

fn parse_edge(action: &str) -> usize {
    action
        .strip_prefix('e')
        .and_then(|rest| rest.split(':').next())
        .expect("generated action uses eN:cM form")
        .parse()
        .expect("generated edge index is numeric")
}

fn triggers_a(code: u8) -> bool {
    matches!(code, 1 | 6)
}

fn responds_a(code: u8) -> bool {
    matches!(code, 2 | 5)
}

fn triggers_b(code: u8) -> bool {
    matches!(code, 3 | 5)
}

fn responds_b(code: u8) -> bool {
    matches!(code, 4 | 6)
}

fn graph_model(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("multi-response-graph-{graph_mask}"),
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                let edge = edge_index(*state, to);
                if has_edge(graph_mask, *state, to) {
                    next.push(Transition::new(format!("e{edge}:c{}", codes[edge]), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

fn generated_property(codes: [u8; EDGE_COUNT]) -> MultiResponseProperty {
    let a_trigger_codes = codes;
    let a_response_codes = codes;
    let b_trigger_codes = codes;
    let b_response_codes = codes;
    MultiResponseProperty::new(
        "generated-two-class-response",
        vec![
            ResponseClause::new(
                "class-a",
                move |action| triggers_a(a_trigger_codes[parse_edge(action)]),
                move |action| responds_a(a_response_codes[parse_edge(action)]),
            )
            .unwrap(),
            ResponseClause::new(
                "class-b",
                move |action| triggers_b(b_trigger_codes[parse_edge(action)]),
                move |action| responds_b(b_response_codes[parse_edge(action)]),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn product_index(node: usize, pending_mask: usize) -> usize {
    node * PENDING_VALUES + pending_mask
}

fn decode_product(index: usize) -> (usize, usize) {
    (index / PENDING_VALUES, index % PENDING_VALUES)
}

fn update_bit(mask: usize, bit: usize, trigger: bool, response: bool) -> usize {
    if response {
        mask & !(1usize << bit)
    } else if trigger {
        mask | (1usize << bit)
    } else {
        mask
    }
}

fn monitor_next(mask: usize, code: u8) -> usize {
    let after_a = update_bit(mask, 0, triggers_a(code), responds_a(code));
    update_bit(after_a, 1, triggers_b(code), responds_b(code))
}

fn oracle_adjacency(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> [[bool; PRODUCT_N]; PRODUCT_N] {
    let mut adjacency = [[false; PRODUCT_N]; PRODUCT_N];
    for node in 0..N {
        for pending_mask in 0..PENDING_VALUES {
            let from = product_index(node, pending_mask);
            for to in 0..N {
                if !has_edge(graph_mask, node, to) {
                    continue;
                }
                let edge = edge_index(node, to);
                let next_mask = monitor_next(pending_mask, codes[edge]);
                adjacency[from][product_index(to, next_mask)] = true;
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

fn clause_floyd(
    adjacency: &[[bool; PRODUCT_N]; PRODUCT_N],
    bit: usize,
) -> [[usize; PRODUCT_N]; PRODUCT_N] {
    let mut distance = [[INF; PRODUCT_N]; PRODUCT_N];
    for (index, row) in distance.iter_mut().enumerate() {
        let (_, mask) = decode_product(index);
        if mask & (1usize << bit) != 0 {
            row[index] = 0;
        }
    }
    for (from, (adjacency_row, distance_row)) in
        adjacency.iter().zip(distance.iter_mut()).enumerate()
    {
        let (_, from_mask) = decode_product(from);
        if from_mask & (1usize << bit) == 0 {
            continue;
        }
        for (to, edge) in adjacency_row.iter().enumerate() {
            let (_, to_mask) = decode_product(to);
            if *edge && to_mask & (1usize << bit) != 0 {
                distance_row[to] = 1;
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
fn dual_protocol_satisfies_both_response_classes() {
    let model = dual_response_protocol().unwrap();
    let result = check_multi_response(&model, &dual_property()).unwrap();

    assert_eq!(result.status, MultiResponseStatus::Satisfied);
    assert_eq!(result.clause_count, 2);
    assert!(result.counterexample.is_none());
}

#[test]
fn unfair_second_class_reports_that_specific_clause() {
    let model = unfair_dual_response_protocol().unwrap();
    let result = check_multi_response(&model, &dual_property()).unwrap();

    assert_eq!(result.status, MultiResponseStatus::Violated);
    let MultiResponseCounterexample::Infinite {
        clause,
        stem,
        cycle,
    } = result.counterexample.unwrap()
    else {
        panic!("expected infinite class-B counterexample");
    };
    assert_eq!(clause, "class-b");
    assert!(stem.last().unwrap().state.pending[1]);
    assert!(cycle.iter().all(|step| step.state.pending[1]));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn alternating_pending_classes_are_not_a_false_cycle_violation() {
    let model = TransitionSystem::new(
        "alternating-obligations",
        vec![StateVariable::new("node", "protocol point")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("request-a", 1)]),
            1 => Ok(vec![Transition::new("grant-a-request-b", 2)]),
            2 => Ok(vec![Transition::new("grant-b-request-a", 1)]),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 2)],
    )
    .unwrap();
    let property = MultiResponseProperty::new(
        "alternating-response",
        vec![
            ResponseClause::new(
                "class-a",
                |action| matches!(action, "request-a" | "grant-b-request-a"),
                |action| action == "grant-a-request-b",
            )
            .unwrap(),
            ResponseClause::new(
                "class-b",
                |action| action == "grant-a-request-b",
                |action| action == "grant-b-request-a",
            )
            .unwrap(),
        ],
    )
    .unwrap();

    let result = check_multi_response(&model, &property).unwrap();
    assert_eq!(result.status, MultiResponseStatus::Satisfied);
    assert!(result.counterexample.is_none());
}

#[test]
fn finite_terminal_identifies_first_pending_clause() {
    let model = TransitionSystem::new(
        "finite-multi-response",
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("request-b", 1)]),
            1 => Ok(Vec::new()),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let result = check_multi_response(&model, &dual_property()).unwrap();

    let MultiResponseCounterexample::Finite { clause, trace } = result.counterexample.unwrap()
    else {
        panic!("expected finite response counterexample");
    };
    assert_eq!(clause, "class-b");
    assert_eq!(trace.len(), 2);
    assert!(trace.last().unwrap().state.pending[1]);
}

#[test]
fn multi_response_metadata_is_validated() {
    assert!(MultiResponseProperty::new("empty", Vec::new()).is_err());
    assert!(ResponseClause::new("   ", |_| true, |_| false).is_err());

    let first = ResponseClause::new("same", |_| true, |_| false).unwrap();
    let second = ResponseClause::new("same", |_| false, |_| true).unwrap();
    assert!(MultiResponseProperty::new("duplicate", vec![first, second]).is_err());
}

#[test]
fn all_two_node_graphs_and_semantic_assignments_match_independent_oracle() {
    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            let model = graph_model(graph_mask, codes);
            let property = generated_property(codes);
            let first = check_multi_response(&model, &property).unwrap();
            let second = check_multi_response(&model, &property).unwrap();
            assert_eq!(
                first, second,
                "determinism graph={graph_mask} assignment={assignment}"
            );

            let adjacency = oracle_adjacency(graph_mask, codes);
            let distance = floyd(&adjacency);
            let initial = product_index(0, 0);
            let clause_distance = [clause_floyd(&adjacency, 0), clause_floyd(&adjacency, 1)];

            let has_pending_terminal = (0..PRODUCT_N).any(|product| {
                let (node, mask) = decode_product(product);
                mask != 0
                    && distance[initial][product] < INF
                    && (0..N).all(|to| !has_edge(graph_mask, node, to))
            });

            let clause_has_cycle = |bit: usize| {
                (0..PRODUCT_N).any(|product| {
                    let (_, mask) = decode_product(product);
                    if mask & (1usize << bit) == 0 || distance[initial][product] >= INF {
                        return false;
                    }
                    adjacency[product][product]
                        || (0..PRODUCT_N).any(|other| {
                            other != product
                                && decode_product(other).1 & (1usize << bit) != 0
                                && distance[initial][other] < INF
                                && clause_distance[bit][product][other] < INF
                                && clause_distance[bit][other][product] < INF
                        })
                })
            };
            let has_pending_cycle = clause_has_cycle(0) || clause_has_cycle(1);
            let expected_violation = has_pending_terminal || has_pending_cycle;

            assert_eq!(
                first.status == MultiResponseStatus::Violated,
                expected_violation,
                "status graph={graph_mask} assignment={assignment}"
            );

            let reachable_products = (0..PRODUCT_N)
                .filter(|product| distance[initial][*product] < INF)
                .count();
            let reachable_edges = (0..PRODUCT_N)
                .filter(|from| distance[initial][*from] < INF)
                .map(|from| adjacency[from].iter().filter(|edge| **edge).count())
                .sum::<usize>();
            assert_eq!(first.product_states, reachable_products);
            assert_eq!(first.product_transitions, reachable_edges);

            match first.counterexample {
                None => assert!(!expected_violation),
                Some(MultiResponseCounterexample::Finite { clause, trace }) => {
                    assert!(
                        has_pending_terminal,
                        "finite witness without terminal oracle"
                    );
                    validate_trace(graph_mask, codes, &trace);
                    let end = &trace.last().unwrap().state;
                    let bit = clause_bit(&clause);
                    assert!(end.pending[bit]);
                    assert!((0..N).all(|to| !has_edge(graph_mask, end.state, to)));
                    let product = product_index(end.state, pending_mask(&end.pending));
                    assert_eq!(trace.len() - 1, distance[initial][product]);
                }
                Some(MultiResponseCounterexample::Infinite {
                    clause,
                    stem,
                    cycle,
                }) => {
                    assert!(!has_pending_terminal, "finite failures take precedence");
                    let bit = clause_bit(&clause);
                    assert!(clause_has_cycle(bit));
                    validate_trace(graph_mask, codes, &stem);
                    validate_trace(graph_mask, codes, &cycle);
                    assert!(cycle.iter().all(|step| step.state.pending[bit]));
                    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
                    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
                    let entry = &cycle.first().unwrap().state;
                    let product = product_index(entry.state, pending_mask(&entry.pending));
                    assert_eq!(stem.len() - 1, distance[initial][product]);
                }
            }
        }
    }
}

fn clause_bit(clause: &str) -> usize {
    match clause {
        "class-a" => 0,
        "class-b" => 1,
        other => panic!("unexpected generated clause {other}"),
    }
}

fn pending_mask(pending: &[bool]) -> usize {
    pending.iter().enumerate().fold(
        0usize,
        |mask, (bit, value)| {
            if *value {
                mask | (1usize << bit)
            } else {
                mask
            }
        },
    )
}

fn validate_trace(
    graph_mask: usize,
    codes: [u8; EDGE_COUNT],
    trace: &[TraceStep<MultiObligationState<usize>>],
) {
    assert!(!trace.is_empty());
    assert_eq!(trace[0].state.pending.len(), 2);
    for pair in trace.windows(2) {
        let from = &pair[0].state;
        let to = &pair[1].state;
        let action = pair[1]
            .action
            .as_deref()
            .expect("non-root trace step has action");
        let edge = parse_edge(action);
        assert_eq!(edge, edge_index(from.state, to.state));
        assert!(has_edge(graph_mask, from.state, to.state));
        let expected = monitor_next(pending_mask(&from.pending), codes[edge]);
        assert_eq!(pending_mask(&to.pending), expected);
    }
}

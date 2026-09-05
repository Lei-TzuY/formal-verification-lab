use formal_verification_lab::monitor_examples::{
    invalid_double_open_protocol, session_monitor, session_protocol, stuck_committed_protocol,
};
use formal_verification_lab::{
    check_monitor, FiniteMonitor, Invariant, MonitorCounterexample, MonitorProductState,
    MonitorStatus, ProgressCondition, RejectCondition, StateVariable, TraceStep, Transition,
    TransitionSystem,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const MONITOR_STATES: usize = 4;
const PRODUCT_N: usize = N * MONITOR_STATES;
const ACTION_COUNT: usize = 4;
const ASSIGNMENT_COUNT: usize = ACTION_COUNT.pow(EDGE_COUNT as u32);
const INF: usize = usize::MAX / 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OracleMonitor {
    Idle,
    Open,
    Committed,
    Rejected,
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
        *code = (assignment % ACTION_COUNT) as u8;
        assignment /= ACTION_COUNT;
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

fn parse_code(action: &str) -> u8 {
    action
        .split(":c")
        .nth(1)
        .expect("generated action contains code")
        .parse()
        .expect("generated action code is numeric")
}

fn oracle_step(state: OracleMonitor, code: u8) -> OracleMonitor {
    use OracleMonitor::{Committed, Idle, Open, Rejected};

    if state == Rejected {
        return Rejected;
    }
    match (state, code) {
        (Idle, 0) => Idle,
        (Idle, 1) => Open,
        (Open, 0) => Open,
        (Open, 2) => Committed,
        (Committed, 0) => Committed,
        (Committed, 3) => Idle,
        _ => Rejected,
    }
}

fn monitor_index(state: OracleMonitor) -> usize {
    match state {
        OracleMonitor::Idle => 0,
        OracleMonitor::Open => 1,
        OracleMonitor::Committed => 2,
        OracleMonitor::Rejected => 3,
    }
}

fn decode_monitor(index: usize) -> OracleMonitor {
    match index {
        0 => OracleMonitor::Idle,
        1 => OracleMonitor::Open,
        2 => OracleMonitor::Committed,
        3 => OracleMonitor::Rejected,
        _ => unreachable!(),
    }
}

fn active(state: OracleMonitor) -> bool {
    matches!(state, OracleMonitor::Open | OracleMonitor::Committed)
}

fn product_index(node: usize, monitor: OracleMonitor) -> usize {
    node * MONITOR_STATES + monitor_index(monitor)
}

fn decode_product(index: usize) -> (usize, OracleMonitor) {
    (index / MONITOR_STATES, decode_monitor(index % MONITOR_STATES))
}

fn graph_model(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("monitor-graph-{graph_mask}"),
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

fn generated_monitor() -> FiniteMonitor<OracleMonitor> {
    FiniteMonitor::new(
        "generated-session-monitor",
        OracleMonitor::Idle,
        |state, action| oracle_step(*state, parse_code(action)),
        vec![RejectCondition::new("legal-order", |state| {
            *state == OracleMonitor::Rejected
        })
        .unwrap()],
        vec![ProgressCondition::new("eventually-idle", |state| active(*state)).unwrap()],
    )
    .unwrap()
}

fn oracle_adjacency(
    graph_mask: usize,
    codes: [u8; EDGE_COUNT],
) -> [[bool; PRODUCT_N]; PRODUCT_N] {
    let mut adjacency = [[false; PRODUCT_N]; PRODUCT_N];
    for node in 0..N {
        for monitor_index_value in 0..MONITOR_STATES {
            let monitor = decode_monitor(monitor_index_value);
            let from = product_index(node, monitor);
            for to in 0..N {
                if !has_edge(graph_mask, node, to) {
                    continue;
                }
                let edge = edge_index(node, to);
                let next = oracle_step(monitor, codes[edge]);
                adjacency[from][product_index(to, next)] = true;
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

fn active_floyd(
    adjacency: &[[bool; PRODUCT_N]; PRODUCT_N],
) -> [[usize; PRODUCT_N]; PRODUCT_N] {
    let mut distance = [[INF; PRODUCT_N]; PRODUCT_N];
    for (index, row) in distance.iter_mut().enumerate() {
        if active(decode_product(index).1) {
            row[index] = 0;
        }
    }
    for (from, (adjacency_row, distance_row)) in
        adjacency.iter().zip(distance.iter_mut()).enumerate()
    {
        if !active(decode_product(from).1) {
            continue;
        }
        for (to, edge) in adjacency_row.iter().enumerate() {
            if *edge && active(decode_product(to).1) {
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
fn ordered_session_satisfies_monitor() {
    let result = check_monitor(&session_protocol().unwrap(), &session_monitor().unwrap()).unwrap();
    assert_eq!(result.status, MonitorStatus::Satisfied);
    assert!(result.counterexample.is_none());
}

#[test]
fn double_open_is_immediate_rejecting_violation() {
    let result = check_monitor(
        &invalid_double_open_protocol().unwrap(),
        &session_monitor().unwrap(),
    )
    .unwrap();
    let MonitorCounterexample::Rejecting { condition, trace } = result.counterexample.unwrap()
    else {
        panic!("expected rejecting-state witness");
    };
    assert_eq!(condition, "legal-action-order");
    assert_eq!(trace.len(), 3);
}

#[test]
fn stuck_committed_state_is_progress_cycle() {
    let result = check_monitor(
        &stuck_committed_protocol().unwrap(),
        &session_monitor().unwrap(),
    )
    .unwrap();
    let MonitorCounterexample::ProgressCycle {
        condition,
        stem,
        cycle,
    } = result.counterexample.unwrap()
    else {
        panic!("expected progress-cycle witness");
    };
    assert_eq!(condition, "opened-session-eventually-closes");
    assert_eq!(stem.len(), 3);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn active_terminal_is_progress_terminal() {
    let model = TransitionSystem::new(
        "open-and-stop",
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("open", 1)]),
            1 => Ok(Vec::new()),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let result = check_monitor(&model, &session_monitor().unwrap()).unwrap();
    let MonitorCounterexample::ProgressTerminal { condition, trace } = result.counterexample.unwrap()
    else {
        panic!("expected progress-terminal witness");
    };
    assert_eq!(condition, "opened-session-eventually-closes");
    assert_eq!(trace.len(), 2);
}

#[test]
fn monitor_metadata_is_validated() {
    assert!(FiniteMonitor::new(
        "   ",
        OracleMonitor::Idle,
        |state, _| *state,
        vec![RejectCondition::new("reject", |_| false).unwrap()],
        Vec::new(),
    )
    .is_err());
    assert!(FiniteMonitor::new(
        "empty",
        OracleMonitor::Idle,
        |state, _| *state,
        Vec::new(),
        Vec::new(),
    )
    .is_err());

    let reject = RejectCondition::new("same", |_| false).unwrap();
    let progress = ProgressCondition::new("same", |_| false).unwrap();
    assert!(FiniteMonitor::new(
        "duplicate",
        OracleMonitor::Idle,
        |state, _| *state,
        vec![reject],
        vec![progress],
    )
    .is_err());
}

#[test]
fn all_two_node_graphs_and_action_assignments_match_independent_monitor_oracle() {
    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            let model = graph_model(graph_mask, codes);
            let monitor = generated_monitor();
            let first = check_monitor(&model, &monitor).unwrap();
            let second = check_monitor(&model, &monitor).unwrap();
            assert_eq!(
                first, second,
                "determinism graph={graph_mask} assignment={assignment}"
            );

            let adjacency = oracle_adjacency(graph_mask, codes);
            let distance = floyd(&adjacency);
            let active_distance = active_floyd(&adjacency);
            let initial = product_index(0, OracleMonitor::Idle);

            let reachable_reject = (0..PRODUCT_N).any(|product| {
                distance[initial][product] < INF
                    && decode_product(product).1 == OracleMonitor::Rejected
            });
            let active_terminal = (0..PRODUCT_N).any(|product| {
                let (node, monitor) = decode_product(product);
                distance[initial][product] < INF
                    && active(monitor)
                    && (0..N).all(|to| !has_edge(graph_mask, node, to))
            });
            let active_cycle = (0..PRODUCT_N).any(|product| {
                if distance[initial][product] >= INF || !active(decode_product(product).1) {
                    return false;
                }
                adjacency[product][product]
                    || (0..PRODUCT_N).any(|other| {
                        other != product
                            && distance[initial][other] < INF
                            && active(decode_product(other).1)
                            && active_distance[product][other] < INF
                            && active_distance[other][product] < INF
                    })
            });
            let expected_violation = reachable_reject || active_terminal || active_cycle;
            assert_eq!(
                first.status == MonitorStatus::Violated,
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
                Some(MonitorCounterexample::Rejecting { condition, trace }) => {
                    assert!(reachable_reject);
                    assert_eq!(condition, "legal-order");
                    validate_trace(graph_mask, codes, &trace);
                    let end = &trace.last().unwrap().state;
                    assert_eq!(end.monitor, OracleMonitor::Rejected);
                    let end_product = product_index(end.state, end.monitor);
                    let min_reject_distance = (0..PRODUCT_N)
                        .filter(|product| decode_product(*product).1 == OracleMonitor::Rejected)
                        .map(|product| distance[initial][product])
                        .min()
                        .unwrap();
                    assert_eq!(trace.len() - 1, distance[initial][end_product]);
                    assert_eq!(trace.len() - 1, min_reject_distance);
                }
                Some(MonitorCounterexample::ProgressTerminal { condition, trace }) => {
                    assert!(!reachable_reject, "rejecting failure takes precedence");
                    assert!(active_terminal);
                    assert_eq!(condition, "eventually-idle");
                    validate_trace(graph_mask, codes, &trace);
                    let end = &trace.last().unwrap().state;
                    assert!(active(end.monitor));
                    assert!((0..N).all(|to| !has_edge(graph_mask, end.state, to)));
                }
                Some(MonitorCounterexample::ProgressCycle {
                    condition,
                    stem,
                    cycle,
                }) => {
                    assert!(!reachable_reject, "rejecting failure takes precedence");
                    assert!(!active_terminal, "progress terminal takes precedence");
                    assert!(active_cycle);
                    assert_eq!(condition, "eventually-idle");
                    validate_trace(graph_mask, codes, &stem);
                    validate_trace(graph_mask, codes, &cycle);
                    assert!(cycle.iter().all(|step| active(step.state.monitor)));
                    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
                    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
                    let entry = &cycle.first().unwrap().state;
                    let product = product_index(entry.state, entry.monitor);
                    assert_eq!(stem.len() - 1, distance[initial][product]);
                }
            }
        }
    }
}

fn validate_trace(
    graph_mask: usize,
    codes: [u8; EDGE_COUNT],
    trace: &[TraceStep<MonitorProductState<usize, OracleMonitor>>],
) {
    assert!(!trace.is_empty());
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
        assert_eq!(to.monitor, oracle_step(from.monitor, codes[edge]));
    }
}

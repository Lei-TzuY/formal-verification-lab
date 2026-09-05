use formal_verification_lab::buchi_examples::{
    alternating_pulses, finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::{
    check_buchi, AcceptanceSet, BuchiAutomaton, BuchiCounterexample, BuchiProductState,
    BuchiStatus, FiniteRunPolicy, Invariant, StateVariable, TraceStep, Transition,
    TransitionSystem,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const AUTOMATON_STATES: usize = 4;
const PRODUCT_N: usize = N * AUTOMATON_STATES;
const ACTION_COUNT: usize = 4;
const ASSIGNMENT_COUNT: usize = ACTION_COUNT.pow(EDGE_COUNT as u32);
const INF: usize = usize::MAX / 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OracleState {
    None,
    A,
    B,
    Both,
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

fn oracle_step(code: u8) -> OracleState {
    match code {
        0 => OracleState::None,
        1 => OracleState::A,
        2 => OracleState::B,
        3 => OracleState::Both,
        _ => unreachable!(),
    }
}

fn state_index(state: OracleState) -> usize {
    match state {
        OracleState::None => 0,
        OracleState::A => 1,
        OracleState::B => 2,
        OracleState::Both => 3,
    }
}

fn decode_state(index: usize) -> OracleState {
    match index {
        0 => OracleState::None,
        1 => OracleState::A,
        2 => OracleState::B,
        3 => OracleState::Both,
        _ => unreachable!(),
    }
}

fn accepts(state: OracleState, set: usize) -> bool {
    match set {
        0 => matches!(state, OracleState::A | OracleState::Both),
        1 => matches!(state, OracleState::B | OracleState::Both),
        _ => unreachable!(),
    }
}

fn acceptance_name(set: usize) -> &'static str {
    match set {
        0 => "set-a",
        1 => "set-b",
        _ => unreachable!(),
    }
}

fn product_index(node: usize, automaton: OracleState) -> usize {
    node * AUTOMATON_STATES + state_index(automaton)
}

fn decode_product(index: usize) -> (usize, OracleState) {
    (
        index / AUTOMATON_STATES,
        decode_state(index % AUTOMATON_STATES),
    )
}

fn graph_model(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("buchi-graph-{graph_mask}"),
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

fn generated_automaton(policy: FiniteRunPolicy) -> BuchiAutomaton<OracleState> {
    BuchiAutomaton::new(
        "generated-generalized-buchi",
        OracleState::None,
        |_state, action| oracle_step(parse_code(action)),
        vec![
            AcceptanceSet::new("set-a", |state| accepts(*state, 0)).unwrap(),
            AcceptanceSet::new("set-b", |state| accepts(*state, 1)).unwrap(),
        ],
        policy,
    )
    .unwrap()
}

fn oracle_adjacency(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> [[bool; PRODUCT_N]; PRODUCT_N] {
    let mut adjacency = [[false; PRODUCT_N]; PRODUCT_N];
    for node in 0..N {
        for state_index_value in 0..AUTOMATON_STATES {
            let automaton = decode_state(state_index_value);
            let from = product_index(node, automaton);
            for to in 0..N {
                if !has_edge(graph_mask, node, to) {
                    continue;
                }
                let edge = edge_index(node, to);
                adjacency[from][product_index(to, oracle_step(codes[edge]))] = true;
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

fn avoiding_floyd(
    adjacency: &[[bool; PRODUCT_N]; PRODUCT_N],
    set: usize,
) -> [[usize; PRODUCT_N]; PRODUCT_N] {
    let mut distance = [[INF; PRODUCT_N]; PRODUCT_N];
    for (index, row) in distance.iter_mut().enumerate() {
        if !accepts(decode_product(index).1, set) {
            row[index] = 0;
        }
    }
    for (from, (adjacency_row, distance_row)) in
        adjacency.iter().zip(distance.iter_mut()).enumerate()
    {
        if accepts(decode_product(from).1, set) {
            continue;
        }
        for (to, edge) in adjacency_row.iter().enumerate() {
            if *edge && !accepts(decode_product(to).1, set) {
                distance_row[to] = distance_row[to].min(1);
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

fn on_avoiding_cycle(
    product: usize,
    set: usize,
    adjacency: &[[bool; PRODUCT_N]; PRODUCT_N],
    avoiding: &[[usize; PRODUCT_N]; PRODUCT_N],
) -> bool {
    if accepts(decode_product(product).1, set) {
        return false;
    }
    adjacency[product][product]
        || (0..PRODUCT_N).any(|other| {
            other != product
                && !accepts(decode_product(other).1, set)
                && avoiding[product][other] < INF
                && avoiding[other][product] < INF
        })
}

#[test]
fn alternating_pulses_satisfy_both_acceptance_sets() {
    let result = check_buchi(
        &alternating_pulses().unwrap(),
        &pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap(),
    )
    .unwrap();
    assert_eq!(result.status, BuchiStatus::Satisfied);
    assert_eq!(result.acceptance_sets, 2);
    assert!(result.counterexample.is_none());
}

#[test]
fn unfair_execution_reports_second_acceptance_set() {
    let result = check_buchi(
        &unfair_second_pulse().unwrap(),
        &pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap(),
    )
    .unwrap();
    let BuchiCounterexample::AcceptanceAvoidingCycle {
        acceptance,
        stem,
        cycle,
    } = result.counterexample.unwrap()
    else {
        panic!("expected acceptance-avoiding lasso");
    };
    assert_eq!(acceptance, "pulse-b-observed");
    assert!(stem
        .iter()
        .any(|step| step.action.as_deref() == Some("pulse-a")));
    assert!(cycle.iter().all(|step| {
        !matches!(
            step.state.automaton,
            formal_verification_lab::buchi_examples::PulseAutomatonState::B
                | formal_verification_lab::buchi_examples::PulseAutomatonState::Both
        )
    }));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn finite_terminal_policy_is_explicit() {
    let model = finite_quiet_run().unwrap();
    let ignored = check_buchi(
        &model,
        &pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap(),
    )
    .unwrap();
    assert_eq!(ignored.status, BuchiStatus::Satisfied);

    let strict = check_buchi(
        &model,
        &pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap(),
    )
    .unwrap();
    let BuchiCounterexample::FiniteTerminal {
        missing_acceptance,
        trace,
    } = strict.counterexample.unwrap()
    else {
        panic!("expected strict finite-terminal failure");
    };
    assert_eq!(missing_acceptance, "pulse-a-observed");
    assert_eq!(trace.len(), 2);
}

#[test]
fn strict_terminal_can_satisfy_all_acceptance_sets() {
    let model = TransitionSystem::new(
        "finite-both",
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("pulse-both", 1)]),
            1 => Ok(Vec::new()),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let result = check_buchi(
        &model,
        &pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap(),
    )
    .unwrap();
    assert_eq!(result.status, BuchiStatus::Satisfied);
}

#[test]
fn buchi_metadata_is_validated() {
    assert!(BuchiAutomaton::new(
        "   ",
        OracleState::None,
        |state, _| *state,
        vec![AcceptanceSet::new("set", |_| true).unwrap()],
        FiniteRunPolicy::IgnoreTerminals,
    )
    .is_err());
    assert!(BuchiAutomaton::new(
        "none",
        OracleState::None,
        |state, _| *state,
        Vec::new(),
        FiniteRunPolicy::IgnoreTerminals,
    )
    .is_err());

    let first = AcceptanceSet::new("same", |_| true).unwrap();
    let second = AcceptanceSet::new("same", |_| false).unwrap();
    assert!(BuchiAutomaton::new(
        "duplicate",
        OracleState::None,
        |state, _| *state,
        vec![first, second],
        FiniteRunPolicy::IgnoreTerminals,
    )
    .is_err());
}

#[test]
fn oracle_shortest_paths_keep_zero_distance_on_self_loops() {
    let mut adjacency = [[false; PRODUCT_N]; PRODUCT_N];
    let initial = product_index(0, OracleState::None);
    adjacency[initial][initial] = true;

    let distance = floyd(&adjacency);
    assert_eq!(distance[initial][initial], 0);

    let avoiding = avoiding_floyd(&adjacency, 0);
    assert_eq!(avoiding[initial][initial], 0);
    assert!(on_avoiding_cycle(initial, 0, &adjacency, &avoiding));
}

#[test]
fn all_two_node_graphs_actions_and_finite_policies_match_independent_oracle() {
    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            let model = graph_model(graph_mask, codes);
            for policy in [
                FiniteRunPolicy::IgnoreTerminals,
                FiniteRunPolicy::RequireAcceptingTerminal,
            ] {
                let automaton = generated_automaton(policy);
                let first = check_buchi(&model, &automaton).unwrap();
                let second = check_buchi(&model, &automaton).unwrap();
                assert_eq!(
                    first, second,
                    "determinism graph={graph_mask} assignment={assignment} policy={policy:?}"
                );

                let adjacency = oracle_adjacency(graph_mask, codes);
                let distance = floyd(&adjacency);
                let avoiding = [avoiding_floyd(&adjacency, 0), avoiding_floyd(&adjacency, 1)];
                let initial = product_index(0, OracleState::None);

                let strict_terminal = policy == FiniteRunPolicy::RequireAcceptingTerminal
                    && (0..PRODUCT_N).any(|product| {
                        let (node, state) = decode_product(product);
                        distance[initial][product] < INF
                            && (0..N).all(|to| !has_edge(graph_mask, node, to))
                            && (!accepts(state, 0) || !accepts(state, 1))
                    });

                let mut best_avoiding: Option<(usize, usize)> = None;
                for set in 0..2 {
                    for product in 0..PRODUCT_N {
                        if distance[initial][product] >= INF
                            || !on_avoiding_cycle(product, set, &adjacency, &avoiding[set])
                        {
                            continue;
                        }
                        let candidate = (distance[initial][product], set);
                        if best_avoiding.is_none_or(|current| candidate < current) {
                            best_avoiding = Some(candidate);
                        }
                    }
                }
                let avoiding_cycle = best_avoiding.is_some();
                let expected_violation = strict_terminal || avoiding_cycle;
                assert_eq!(
                    first.status == BuchiStatus::Violated,
                    expected_violation,
                    "status graph={graph_mask} assignment={assignment} policy={policy:?}"
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
                    Some(BuchiCounterexample::FiniteTerminal {
                        missing_acceptance,
                        trace,
                    }) => {
                        assert!(strict_terminal);
                        validate_trace(graph_mask, codes, &trace);
                        let end = &trace.last().unwrap().state;
                        let (node, state) = (end.state, end.automaton);
                        assert!((0..N).all(|to| !has_edge(graph_mask, node, to)));
                        let missing = if !accepts(state, 0) { 0 } else { 1 };
                        assert_eq!(missing_acceptance, acceptance_name(missing));
                        let min_distance = (0..PRODUCT_N)
                            .filter(|product| {
                                let (candidate_node, candidate_state) = decode_product(*product);
                                distance[initial][*product] < INF
                                    && (0..N).all(|to| !has_edge(graph_mask, candidate_node, to))
                                    && (!accepts(candidate_state, 0)
                                        || !accepts(candidate_state, 1))
                            })
                            .map(|product| distance[initial][product])
                            .min()
                            .unwrap();
                        assert_eq!(trace.len() - 1, min_distance);
                    }
                    Some(BuchiCounterexample::AcceptanceAvoidingCycle {
                        acceptance,
                        stem,
                        cycle,
                    }) => {
                        assert!(!strict_terminal, "strict finite terminal takes precedence");
                        let set = match acceptance.as_str() {
                            "set-a" => 0,
                            "set-b" => 1,
                            other => panic!("unexpected acceptance set {other}"),
                        };
                        let (best_distance, best_set) =
                            best_avoiding.expect("cycle counterexample has an oracle candidate");
                        assert_eq!(set, best_set);
                        assert_eq!(stem.len() - 1, best_distance);
                        validate_trace(graph_mask, codes, &stem);
                        validate_trace(graph_mask, codes, &cycle);
                        assert!(cycle.iter().all(|step| !accepts(step.state.automaton, set)));
                        assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
                        assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
                        let entry = &cycle.first().unwrap().state;
                        let product = product_index(entry.state, entry.automaton);
                        assert!(on_avoiding_cycle(product, set, &adjacency, &avoiding[set]));
                        assert_eq!(stem.len() - 1, distance[initial][product]);
                    }
                }
            }
        }
    }
}

fn validate_trace(
    graph_mask: usize,
    codes: [u8; EDGE_COUNT],
    trace: &[TraceStep<BuchiProductState<usize, OracleState>>],
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
        assert_eq!(to.automaton, oracle_step(codes[edge]));
    }
}

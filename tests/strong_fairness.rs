use formal_verification_lab::buchi::{
    check_buchi, AcceptanceSet, BuchiAutomaton, BuchiCounterexample, BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::pulse_automaton;
use formal_verification_lab::fairness::{check_buchi_with_weak_fairness, WeakFairness};
use formal_verification_lab::strong_fairness::{
    check_buchi_with_strong_fairness, StrongFairness, StrongFairnessError,
};
use formal_verification_lab::{Invariant, StateVariable, Transition, TransitionSystem};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const ACTION_COUNT: usize = 4;

fn sticky_action_automaton(
    name: &str,
    action: &'static str,
    policy: FiniteRunPolicy,
) -> BuchiAutomaton<bool> {
    BuchiAutomaton::new(
        name,
        false,
        move |seen, observed| *seen || observed == action,
        vec![AcceptanceSet::new("observed", |seen: &bool| *seen).unwrap()],
        policy,
    )
    .unwrap()
}

fn two_state_intermittent_enablement_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "intermittent-serve",
        vec![StateVariable::new("node", "scheduler point")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![
                Transition::new("step", 1usize),
                Transition::new("serve", 1usize),
            ]),
            1 => Ok(vec![Transition::new("step", 0usize)]),
            _ => unreachable!("model state is in range"),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < 2)],
    )
    .unwrap()
}

#[test]
fn strong_fairness_excludes_intermittently_enabled_starvation_that_weak_fairness_allows() {
    let model = two_state_intermittent_enablement_model();
    let automaton =
        sticky_action_automaton("serve-observed", "serve", FiniteRunPolicy::IgnoreTerminals);
    let weak = WeakFairness::new(["serve"]).unwrap();
    let strong = StrongFairness::new(["serve"]).unwrap();

    let weak_result = check_buchi_with_weak_fairness(&model, &automaton, &weak).unwrap();
    assert_eq!(weak_result.status, BuchiStatus::Violated);
    let BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. } =
        weak_result.counterexample.unwrap()
    else {
        panic!("weak fairness should retain the intermittent starvation lasso");
    };
    assert!(cycle
        .iter()
        .all(|step| step.action.as_deref() != Some("serve")));

    let strong_result = check_buchi_with_strong_fairness(&model, &automaton, &strong).unwrap();
    assert_eq!(strong_result.status, BuchiStatus::Satisfied);
    assert!(strong_result.counterexample.is_none());
}

#[test]
fn streett_pruning_keeps_a_fair_subcycle_after_removing_bad_enabled_states() {
    let model = TransitionSystem::new(
        "strong-fair-pruning",
        vec![StateVariable::new("node", "pruning point")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![
                Transition::new("go", 1usize),
                Transition::new("serve", 1usize),
            ]),
            1 => Ok(vec![
                Transition::new("back", 0usize),
                Transition::new("idle", 1usize),
            ]),
            _ => unreachable!("model state is in range"),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < 2)],
    )
    .unwrap();
    let automaton =
        sticky_action_automaton("serve-observed", "serve", FiniteRunPolicy::IgnoreTerminals);
    let fairness = StrongFairness::new(["serve"]).unwrap();

    let result = check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
    assert_eq!(result.status, BuchiStatus::Violated);
    let BuchiCounterexample::AcceptanceAvoidingCycle {
        acceptance,
        stem,
        cycle,
    } = result.counterexample.unwrap()
    else {
        panic!("the safe-node idle subcycle remains a strong-fair violation");
    };
    assert_eq!(acceptance, "observed");
    assert_eq!(stem.last().unwrap().state.state, 1);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle.iter().all(|step| step.state.state == 1));
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("idle")));
}

#[test]
fn strong_fairness_never_changes_strict_finite_terminal_failures() {
    let model = TransitionSystem::new(
        "strict-terminal",
        vec![StateVariable::new("node", "terminal")],
        vec![0usize],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("well-formed", |_state: &usize| true)],
    )
    .unwrap();
    let automaton = sticky_action_automaton(
        "serve-observed",
        "serve",
        FiniteRunPolicy::RequireAcceptingTerminal,
    );
    let fairness = StrongFairness::new(["serve"]).unwrap();

    let ordinary = check_buchi(&model, &automaton).unwrap();
    let strong = check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
    assert_eq!(strong, ordinary);
    assert!(matches!(
        strong.counterexample,
        Some(BuchiCounterexample::FiniteTerminal { .. })
    ));
}

#[test]
fn strong_fairness_validation_preserves_order_and_fails_closed() {
    let fairness = StrongFairness::new(["a", "b"]).unwrap();
    assert_eq!(fairness.actions(), &["a".to_owned(), "b".to_owned()]);
    assert_eq!(StrongFairness::none().actions(), &[] as &[String]);
    assert_eq!(
        StrongFairness::new([""]).unwrap_err(),
        StrongFairnessError::EmptyActionName
    );
    assert_eq!(
        StrongFairness::new(["a", "a"]).unwrap_err(),
        StrongFairnessError::DuplicateAction {
            action: "a".to_owned()
        }
    );
}

fn edge_index(from: usize, to: usize) -> usize {
    from * N + to
}

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << edge_index(from, to)) != 0
}

fn decode_actions(mut assignment: usize) -> [u8; EDGE_COUNT] {
    let mut actions = [0u8; EDGE_COUNT];
    for action in &mut actions {
        *action = (assignment % ACTION_COUNT) as u8;
        assignment /= ACTION_COUNT;
    }
    actions
}

fn action_label(code: u8) -> &'static str {
    match code {
        0 => "quiet",
        1 => "fair-a",
        2 => "fair-b",
        3 => "other",
        _ => unreachable!("generated action code is in range"),
    }
}

fn generated_model(mask: usize, actions: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("strong-fair-oracle-{mask}"),
        vec![StateVariable::new("node", "current graph node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                if has_edge(mask, *state, to) {
                    let edge = edge_index(*state, to);
                    next.push(Transition::new(action_label(actions[edge]), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

fn never_accepting_automaton() -> BuchiAutomaton<()> {
    BuchiAutomaton::new(
        "never-accepting",
        (),
        |_state, _action| (),
        vec![AcceptanceSet::new("never", |_state: &()| false).unwrap()],
        FiniteRunPolicy::IgnoreTerminals,
    )
    .unwrap()
}

fn reachable_nodes(mask: usize) -> [bool; N] {
    let mut reachable = [false; N];
    reachable[0] = true;
    for _ in 0..N {
        for from in 0..N {
            if !reachable[from] {
                continue;
            }
            for (to, target_reachable) in reachable.iter_mut().enumerate() {
                if has_edge(mask, from, to) {
                    *target_reachable = true;
                }
            }
        }
    }
    reachable
}

fn subset_contains(subset: usize, node: usize) -> bool {
    subset & (1usize << node) != 0
}

fn restricted_reachable(mask: usize, subset: usize, start: usize, goal: usize) -> bool {
    if !subset_contains(subset, start) || !subset_contains(subset, goal) {
        return false;
    }
    let mut seen = [false; N];
    seen[start] = true;
    for _ in 0..N {
        for from in 0..N {
            if !seen[from] || !subset_contains(subset, from) {
                continue;
            }
            for (to, target_seen) in seen.iter_mut().enumerate() {
                if subset_contains(subset, to) && has_edge(mask, from, to) {
                    *target_seen = true;
                }
            }
        }
    }
    seen[goal]
}

fn subset_is_cyclic_strongly_connected(mask: usize, subset: usize) -> bool {
    let members = (0..N)
        .filter(|node| subset_contains(subset, *node))
        .collect::<Vec<_>>();
    if members.is_empty() {
        return false;
    }
    if members.len() == 1 {
        let node = members[0];
        if !has_edge(mask, node, node) {
            return false;
        }
    }
    members.iter().all(|from| {
        members
            .iter()
            .all(|to| restricted_reachable(mask, subset, *from, *to))
    })
}

fn subset_satisfies_strong_fairness(mask: usize, actions: [u8; EDGE_COUNT], subset: usize) -> bool {
    [1u8, 2u8].into_iter().all(|fair_code| {
        let enabled = (0..N).any(|from| {
            subset_contains(subset, from)
                && (0..N).any(|to| {
                    has_edge(mask, from, to) && actions[edge_index(from, to)] == fair_code
                })
        });
        let internally_taken = (0..N).any(|from| {
            subset_contains(subset, from)
                && (0..N).any(|to| {
                    subset_contains(subset, to)
                        && has_edge(mask, from, to)
                        && actions[edge_index(from, to)] == fair_code
                })
        });
        !enabled || internally_taken
    })
}

fn oracle_has_strong_fair_infinite_run(mask: usize, actions: [u8; EDGE_COUNT]) -> bool {
    let reachable = reachable_nodes(mask);
    (1usize..(1usize << N)).any(|subset| {
        (0..N).any(|node| subset_contains(subset, node) && reachable[node])
            && subset_is_cyclic_strongly_connected(mask, subset)
            && subset_satisfies_strong_fairness(mask, actions, subset)
    })
}

fn assert_real_strong_fair_cycle(
    mask: usize,
    actions: [u8; EDGE_COUNT],
    cycle: &[formal_verification_lab::TraceStep<
        formal_verification_lab::BuchiProductState<usize, ()>,
    >],
) {
    assert!(cycle.len() >= 2);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);

    for pair in cycle.windows(2) {
        let from = pair[0].state.state;
        let to = pair[1].state.state;
        let action = pair[1]
            .action
            .as_deref()
            .expect("non-root cycle step has an action");
        let edge = edge_index(from, to);
        assert!(has_edge(mask, from, to));
        assert_eq!(action, action_label(actions[edge]));
    }

    for fair_code in [1u8, 2u8] {
        let fair_action = action_label(fair_code);
        let enabled_on_cycle = cycle.iter().any(|step| {
            let from = step.state.state;
            (0..N).any(|to| has_edge(mask, from, to) && actions[edge_index(from, to)] == fair_code)
        });
        let taken_on_cycle = cycle
            .iter()
            .any(|step| step.action.as_deref() == Some(fair_action));
        assert!(!enabled_on_cycle || taken_on_cycle);
    }
}

#[test]
fn all_two_node_graph_action_assignments_match_independent_strong_fair_oracle() {
    let fairness = StrongFairness::new(["fair-a", "fair-b"]).unwrap();
    let automaton = never_accepting_automaton();

    for graph_mask in 0usize..(1usize << EDGE_COUNT) {
        for assignment in 0usize..ACTION_COUNT.pow(EDGE_COUNT as u32) {
            let actions = decode_actions(assignment);
            let expected_violation = oracle_has_strong_fair_infinite_run(graph_mask, actions);
            let model = generated_model(graph_mask, actions);
            let result = check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
            let repeated = check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
            assert_eq!(
                result, repeated,
                "determinism graph={graph_mask} assignment={assignment}"
            );
            assert_eq!(
                result.status == BuchiStatus::Violated,
                expected_violation,
                "graph={graph_mask} assignment={assignment}"
            );

            match result.counterexample {
                Some(BuchiCounterexample::AcceptanceAvoidingCycle {
                    acceptance,
                    stem,
                    cycle,
                }) => {
                    assert!(expected_violation);
                    assert_eq!(acceptance, "never");
                    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
                    assert_real_strong_fair_cycle(graph_mask, actions, &cycle);
                }
                None => assert!(!expected_violation),
                Some(BuchiCounterexample::FiniteTerminal { .. }) => {
                    panic!("ignore-terminal oracle cannot return a finite violation")
                }
            }
        }
    }
}

fn pulse_action_label(code: u8) -> &'static str {
    match code {
        0 => "quiet",
        1 => "pulse-a",
        2 => "pulse-b",
        3 => "pulse-both",
        _ => unreachable!("generated pulse action code is in range"),
    }
}

fn pulse_graph_model(mask: usize, actions: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("empty-strong-fairness-compat-{mask}"),
        vec![StateVariable::new("node", "current graph node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                if has_edge(mask, *state, to) {
                    let edge = edge_index(*state, to);
                    next.push(Transition::new(pulse_action_label(actions[edge]), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

#[test]
fn empty_strong_fairness_exactly_matches_existing_buchi_on_all_m13_two_node_products() {
    let fairness = StrongFairness::none();
    for graph_mask in 0usize..(1usize << EDGE_COUNT) {
        for assignment in 0usize..ACTION_COUNT.pow(EDGE_COUNT as u32) {
            let actions = decode_actions(assignment);
            for policy in [
                FiniteRunPolicy::IgnoreTerminals,
                FiniteRunPolicy::RequireAcceptingTerminal,
            ] {
                let model = pulse_graph_model(graph_mask, actions);
                let automaton = pulse_automaton(policy).unwrap();
                let ordinary = check_buchi(&model, &automaton).unwrap();
                let strong =
                    check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
                assert_eq!(
                    strong, ordinary,
                    "graph={graph_mask} assignment={assignment} policy={policy:?}"
                );
            }
        }
    }
}

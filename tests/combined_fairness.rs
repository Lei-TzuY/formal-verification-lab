use formal_verification_lab::buchi_examples::{
    finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::{
    check_buchi, check_buchi_with_fairness_profile, check_buchi_with_strong_fairness,
    check_buchi_with_weak_fairness, AcceptanceSet, BuchiAutomaton, BuchiCounterexample,
    BuchiProductState, BuchiStatus, FairnessProfile, FairnessProfileError, FiniteRunPolicy,
    Invariant, StateVariable, StrongFairness, TraceStep, Transition, TransitionSystem, WeakFairness,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const CODE_COUNT: usize = 4; // absent, weak action, strong action, other
const ASSIGNMENT_COUNT: usize = CODE_COUNT.pow(EDGE_COUNT as u32);

fn decode(mut assignment: usize) -> [u8; EDGE_COUNT] {
    let mut codes = [0; EDGE_COUNT];
    for code in &mut codes {
        *code = (assignment % CODE_COUNT) as u8;
        assignment /= CODE_COUNT;
    }
    codes
}

fn action(code: u8) -> &'static str {
    match code {
        1 => "w",
        2 => "s",
        3 => "other",
        _ => unreachable!("absent edges have no action"),
    }
}

fn action_code(label: &str) -> u8 {
    match label {
        "w" => 1,
        "s" => 2,
        "other" => 3,
        _ => panic!("unexpected generated action {label}"),
    }
}

fn graph_model(codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        "combined-fairness-generated",
        vec![StateVariable::new("node", "generated graph node")],
        vec![0],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                let code = codes[*state * N + to];
                if code != 0 {
                    next.push(Transition::new(action(code), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

fn reject_all_automaton() -> BuchiAutomaton<()> {
    BuchiAutomaton::new(
        "reject-every-infinite-run",
        (),
        |_state, _action| (),
        vec![AcceptanceSet::new("never", |_state| false).unwrap()],
        FiniteRunPolicy::IgnoreTerminals,
    )
    .unwrap()
}

fn full_reachability(codes: [u8; EDGE_COUNT]) -> [[bool; N]; N] {
    let mut reach = [[false; N]; N];
    for (node, row) in reach.iter_mut().enumerate() {
        row[node] = true;
    }
    for from in 0..N {
        for to in 0..N {
            if codes[from * N + to] != 0 {
                reach[from][to] = true;
            }
        }
    }
    for via in 0..N {
        for from in 0..N {
            for to in 0..N {
                reach[from][to] |= reach[from][via] && reach[via][to];
            }
        }
    }
    reach
}

fn recurrent_subset(codes: [u8; EDGE_COUNT], mask: usize) -> bool {
    let nodes = (0..N)
        .filter(|node| mask & (1 << node) != 0)
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return false;
    }

    let full = full_reachability(codes);
    if !nodes.iter().any(|node| full[0][*node]) {
        return false;
    }

    let mut reach = [[false; N]; N];
    for node in &nodes {
        reach[*node][*node] = true;
    }
    for from in &nodes {
        for to in &nodes {
            if codes[*from * N + *to] != 0 {
                reach[*from][*to] = true;
            }
        }
    }
    for via in &nodes {
        for from in &nodes {
            for to in &nodes {
                reach[*from][*to] |= reach[*from][*via] && reach[*via][*to];
            }
        }
    }
    if nodes
        .iter()
        .any(|from| nodes.iter().any(|to| !reach[*from][*to]))
    {
        return false;
    }

    nodes.len() > 1 || codes[nodes[0] * N + nodes[0]] != 0
}

fn enabled(codes: [u8; EDGE_COUNT], state: usize, code: u8) -> bool {
    (0..N).any(|to| codes[state * N + to] == code)
}

fn internal_take(codes: [u8; EDGE_COUNT], mask: usize, code: u8) -> bool {
    (0..N).any(|from| {
        mask & (1 << from) != 0
            && (0..N).any(|to| {
                mask & (1 << to) != 0 && codes[from * N + to] == code
            })
    })
}

fn oracle_exists_combined_fair_run(
    codes: [u8; EDGE_COUNT],
    weak_codes: &[u8],
    strong_codes: &[u8],
) -> bool {
    (1..(1 << N)).any(|mask| {
        if !recurrent_subset(codes, mask) {
            return false;
        }

        let weak_ok = weak_codes.iter().all(|code| {
            let some_disabled =
                (0..N).any(|node| mask & (1 << node) != 0 && !enabled(codes, node, *code));
            some_disabled || internal_take(codes, mask, *code)
        });
        let strong_ok = strong_codes.iter().all(|code| {
            let some_enabled =
                (0..N).any(|node| mask & (1 << node) != 0 && enabled(codes, node, *code));
            !some_enabled || internal_take(codes, mask, *code)
        });
        weak_ok && strong_ok
    })
}

fn assert_real_combined_fair_cycle(
    codes: [u8; EDGE_COUNT],
    cycle: &[TraceStep<BuchiProductState<usize, ()>>],
    weak_codes: &[u8],
    strong_codes: &[u8],
) {
    assert!(cycle.len() >= 2);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);

    for pair in cycle.windows(2) {
        let action = pair[1].action.as_deref().expect("cycle edge action");
        let from = pair[0].state.state;
        let to = pair[1].state.state;
        assert_eq!(codes[from * N + to], action_code(action));
    }

    for code in weak_codes {
        let taken = cycle
            .iter()
            .skip(1)
            .filter_map(|step| step.action.as_deref())
            .any(|label| action_code(label) == *code);
        let disabled = cycle.iter().take(cycle.len() - 1).any(|step| {
            !enabled(codes, step.state.state, *code)
        });
        assert!(taken || disabled, "weak fairness obligation was not witnessed");
    }

    for code in strong_codes {
        let enabled_infinitely_often = cycle
            .iter()
            .take(cycle.len() - 1)
            .any(|step| enabled(codes, step.state.state, *code));
        let taken = cycle
            .iter()
            .skip(1)
            .filter_map(|step| step.action.as_deref())
            .any(|label| action_code(label) == *code);
        assert!(
            !enabled_infinitely_often || taken,
            "strong fairness obligation was not witnessed"
        );
    }
}

#[test]
fn profile_validates_each_class_and_canonicalizes_overlap_to_strong() {
    let profile = FairnessProfile::new(["a", "b"], ["b", "c"]).unwrap();
    assert_eq!(profile.weak_actions(), &["a"]);
    assert_eq!(profile.strong_actions(), &["b", "c"]);

    assert!(matches!(
        FairnessProfile::new([""], Vec::<&str>::new()),
        Err(FairnessProfileError::Weak(_))
    ));
    assert!(matches!(
        FairnessProfile::new(["a", "a"], Vec::<&str>::new()),
        Err(FairnessProfileError::Weak(_))
    ));
    assert!(matches!(
        FairnessProfile::new(Vec::<&str>::new(), [""]),
        Err(FairnessProfileError::Strong(_))
    ));
    assert!(matches!(
        FairnessProfile::new(Vec::<&str>::new(), ["a", "a"]),
        Err(FairnessProfileError::Strong(_))
    ));
}

#[test]
fn intermittent_enablement_separates_weak_from_strong_fairness() {
    let model = TransitionSystem::new(
        "intermittent-enable",
        vec![StateVariable::new("node", "protocol point")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("tick", 1), Transition::new("s", 2)]),
            1 => Ok(vec![Transition::new("tick", 0)]),
            2 => Ok(Vec::new()),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < 3)],
    )
    .unwrap();
    let automaton = reject_all_automaton();

    let weak = FairnessProfile::new(["s"], Vec::<&str>::new()).unwrap();
    let strong = FairnessProfile::new(Vec::<&str>::new(), ["s"]).unwrap();

    assert_eq!(
        check_buchi_with_fairness_profile(&model, &automaton, &weak)
            .unwrap()
            .status,
        BuchiStatus::Violated
    );
    assert_eq!(
        check_buchi_with_fairness_profile(&model, &automaton, &strong)
            .unwrap()
            .status,
        BuchiStatus::Satisfied
    );
}

#[test]
fn mixed_witness_satisfies_weak_and_strong_obligations_on_one_closed_walk() {
    let model = TransitionSystem::new(
        "mixed-fair-cycle",
        vec![StateVariable::new("node", "protocol point")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("w", 0), Transition::new("s", 1)]),
            1 => Ok(vec![Transition::new("w", 0)]),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < 2)],
    )
    .unwrap();
    let automaton = reject_all_automaton();
    let profile = FairnessProfile::new(["w"], ["s"]).unwrap();
    let result = check_buchi_with_fairness_profile(&model, &automaton, &profile).unwrap();

    assert_eq!(result.status, BuchiStatus::Violated);
    let BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. } =
        result.counterexample.expect("mixed fair lasso")
    else {
        panic!("expected infinite mixed-fair counterexample");
    };
    let actions = cycle
        .iter()
        .filter_map(|step| step.action.as_deref())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"w"));
    assert!(actions.contains(&"s"));
}

#[test]
fn finite_terminal_policy_is_unchanged_by_combined_fairness() {
    let model = finite_quiet_run().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();
    let profile = FairnessProfile::new(["pulse-a"], ["pulse-b"]).unwrap();
    let result = check_buchi_with_fairness_profile(&model, &automaton, &profile).unwrap();

    assert_eq!(result.status, BuchiStatus::Violated);
    assert!(matches!(
        result.counterexample,
        Some(BuchiCounterexample::FiniteTerminal { .. })
    ));
}

#[test]
fn empty_and_single_class_profiles_preserve_existing_backends_exactly() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();

    let none = FairnessProfile::none();
    assert_eq!(
        check_buchi_with_fairness_profile(&model, &automaton, &none).unwrap(),
        check_buchi(&model, &automaton).unwrap()
    );

    let weak_profile = FairnessProfile::new(["pulse-b"], Vec::<&str>::new()).unwrap();
    let weak = WeakFairness::new(["pulse-b"]).unwrap();
    assert_eq!(
        check_buchi_with_fairness_profile(&model, &automaton, &weak_profile).unwrap(),
        check_buchi_with_weak_fairness(&model, &automaton, &weak).unwrap()
    );

    let strong_profile = FairnessProfile::new(Vec::<&str>::new(), ["pulse-b"]).unwrap();
    let strong = StrongFairness::new(["pulse-b"]).unwrap();
    assert_eq!(
        check_buchi_with_fairness_profile(&model, &automaton, &strong_profile).unwrap(),
        check_buchi_with_strong_fairness(&model, &automaton, &strong).unwrap()
    );
}

#[test]
fn all_two_node_action_graphs_match_independent_mixed_and_overlap_oracles() {
    let automaton = reject_all_automaton();
    let mixed = FairnessProfile::new(["w"], ["s"]).unwrap();
    let overlap = FairnessProfile::new(["w"], ["w"]).unwrap();

    for assignment in 0..ASSIGNMENT_COUNT {
        let codes = decode(assignment);
        let model = graph_model(codes);

        for (profile, weak_codes, strong_codes, label) in [
            (&mixed, &[1u8][..], &[2u8][..], "mixed"),
            (&overlap, &[][..], &[1u8][..], "overlap"),
        ] {
            let expected_violation =
                oracle_exists_combined_fair_run(codes, weak_codes, strong_codes);
            let first = check_buchi_with_fairness_profile(&model, &automaton, profile).unwrap();
            let second = check_buchi_with_fairness_profile(&model, &automaton, profile).unwrap();
            assert_eq!(first, second, "determinism: {label} assignment={assignment}");
            assert_eq!(
                first.status,
                if expected_violation {
                    BuchiStatus::Violated
                } else {
                    BuchiStatus::Satisfied
                },
                "status: {label} assignment={assignment} codes={codes:?}"
            );

            if expected_violation {
                let Some(BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. }) =
                    first.counterexample.as_ref()
                else {
                    panic!("expected lasso: {label} assignment={assignment}");
                };
                assert_real_combined_fair_cycle(codes, cycle, weak_codes, strong_codes);
            } else {
                assert!(first.counterexample.is_none());
            }
        }
    }
}

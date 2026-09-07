use formal_verification_lab::buchi::{
    check_buchi_with_limits, check_buchi_with_product_limits, AcceptanceSet, BuchiAutomaton,
    BuchiCounterexample, BuchiProductState, BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{
    finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::{
    check_buchi_with_fairness_profile, check_buchi_with_fairness_profile_and_limits,
    check_buchi_with_fairness_profile_and_product_limits,
    check_buchi_with_strong_fairness_and_limits,
    check_buchi_with_strong_fairness_and_product_limits, check_buchi_with_weak_fairness_and_limits,
    check_buchi_with_weak_fairness_and_product_limits, AnalysisInconclusiveReason, AnalysisLimits,
    AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits, FairnessProfile,
    InconclusiveReason, Invariant, StateVariable, StrongFairness, TraceStep, Transition,
    TransitionSystem, WeakFairness,
};
use std::collections::VecDeque;

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const CODE_COUNT: usize = 4; // absent, weak action, strong action, other

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

fn decode(mut assignment: usize) -> [u8; EDGE_COUNT] {
    let mut codes = [0u8; EDGE_COUNT];
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
        "bounded-combined-fairness-generated",
        vec![StateVariable::new("node", "generated graph node")],
        vec![0usize],
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

#[derive(Debug, Clone, Copy)]
struct OraclePrefix {
    discovered: [bool; N],
    checked: [bool; N],
    retained: [[bool; N]; N],
    complete: bool,
    checked_states: usize,
    explored_transitions: usize,
}

fn oracle_prefix(codes: [u8; EDGE_COUNT], limit: usize) -> OraclePrefix {
    let mut discovered = [false; N];
    let mut checked = [false; N];
    let mut retained = [[false; N]; N];
    let mut queue = VecDeque::new();
    discovered[0] = true;
    queue.push_back(0usize);
    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(from) = queue.pop_front() {
        checked_states += 1;
        checked[from] = true;
        for to in 0..N {
            if codes[from * N + to] == 0 {
                continue;
            }
            if explored_transitions >= limit {
                return OraclePrefix {
                    discovered,
                    checked,
                    retained,
                    complete: false,
                    checked_states,
                    explored_transitions,
                };
            }
            explored_transitions += 1;
            retained[from][to] = true;
            if !discovered[to] {
                discovered[to] = true;
                queue.push_back(to);
            }
        }
    }

    OraclePrefix {
        discovered,
        checked,
        retained,
        complete: true,
        checked_states,
        explored_transitions,
    }
}

fn subset_contains(subset: usize, node: usize) -> bool {
    subset & (1usize << node) != 0
}

fn reachable_from_initial(retained: &[[bool; N]; N], goal: usize) -> bool {
    let mut seen = [false; N];
    seen[0] = true;
    for _ in 0..N {
        for from in 0..N {
            if !seen[from] {
                continue;
            }
            for (to, target_seen) in seen.iter_mut().enumerate() {
                if retained[from][to] {
                    *target_seen = true;
                }
            }
        }
    }
    seen[goal]
}

fn restricted_reachable(
    retained: &[[bool; N]; N],
    subset: usize,
    start: usize,
    goal: usize,
) -> bool {
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
                if subset_contains(subset, to) && retained[from][to] {
                    *target_seen = true;
                }
            }
        }
    }
    seen[goal]
}

fn recurrent_subset(prefix: OraclePrefix, subset: usize) -> bool {
    let members = (0..N)
        .filter(|node| subset_contains(subset, *node))
        .collect::<Vec<_>>();
    if members.is_empty()
        || members.iter().any(|node| !prefix.discovered[*node])
        || !members
            .iter()
            .any(|node| reachable_from_initial(&prefix.retained, *node))
    {
        return false;
    }
    if members.len() == 1 && !prefix.retained[members[0]][members[0]] {
        return false;
    }
    members.iter().all(|from| {
        members
            .iter()
            .all(|to| restricted_reachable(&prefix.retained, subset, *from, *to))
    })
}

fn full_enabled(codes: [u8; EDGE_COUNT], state: usize, code: u8) -> bool {
    (0..N).any(|to| codes[state * N + to] == code)
}

fn staged_enabled(prefix: OraclePrefix, codes: [u8; EDGE_COUNT], state: usize, code: u8) -> bool {
    if prefix.checked[state] {
        full_enabled(codes, state, code)
    } else {
        true
    }
}

fn internal_take(
    codes: [u8; EDGE_COUNT],
    retained: &[[bool; N]; N],
    subset: usize,
    code: u8,
) -> bool {
    (0..N).any(|from| {
        subset_contains(subset, from)
            && (0..N).any(|to| {
                subset_contains(subset, to) && retained[from][to] && codes[from * N + to] == code
            })
    })
}

fn subset_is_fair(
    codes: [u8; EDGE_COUNT],
    prefix: OraclePrefix,
    subset: usize,
    weak_codes: &[u8],
    strong_codes: &[u8],
    staged: bool,
) -> bool {
    let enabled = |state, code| {
        if staged {
            staged_enabled(prefix, codes, state, code)
        } else {
            full_enabled(codes, state, code)
        }
    };

    let weak_ok = weak_codes.iter().all(|code| {
        let some_disabled =
            (0..N).any(|node| subset_contains(subset, node) && !enabled(node, *code));
        some_disabled || internal_take(codes, &prefix.retained, subset, *code)
    });
    let strong_ok = strong_codes.iter().all(|code| {
        let some_enabled = (0..N).any(|node| subset_contains(subset, node) && enabled(node, *code));
        !some_enabled || internal_take(codes, &prefix.retained, subset, *code)
    });
    weak_ok && strong_ok
}

fn oracle_has_fair_cycle(
    codes: [u8; EDGE_COUNT],
    prefix: OraclePrefix,
    weak_codes: &[u8],
    strong_codes: &[u8],
    staged: bool,
) -> bool {
    (1usize..(1usize << N)).any(|subset| {
        recurrent_subset(prefix, subset)
            && subset_is_fair(codes, prefix, subset, weak_codes, strong_codes, staged)
    })
}

fn assert_cycle_uses_retained_edges_and_is_fair(
    codes: [u8; EDGE_COUNT],
    prefix: OraclePrefix,
    cycle: &[TraceStep<BuchiProductState<usize, ()>>],
    weak_codes: &[u8],
    strong_codes: &[u8],
    staged: bool,
) {
    assert!(cycle.len() >= 2);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);

    for pair in cycle.windows(2) {
        let from = pair[0].state.state;
        let to = pair[1].state.state;
        let edge_action = pair[1].action.as_deref().expect("cycle edge action");
        assert!(prefix.retained[from][to]);
        assert_eq!(codes[from * N + to], action_code(edge_action));
    }

    let enabled = |state, code| {
        if staged {
            staged_enabled(prefix, codes, state, code)
        } else {
            full_enabled(codes, state, code)
        }
    };
    for code in weak_codes {
        let taken = cycle
            .iter()
            .skip(1)
            .filter_map(|step| step.action.as_deref())
            .any(|label| action_code(label) == *code);
        let disabled = cycle
            .iter()
            .take(cycle.len() - 1)
            .any(|step| !enabled(step.state.state, *code));
        assert!(taken || disabled, "weak obligation missing from cycle");
    }
    for code in strong_codes {
        let enabled_infinitely_often = cycle
            .iter()
            .take(cycle.len() - 1)
            .any(|step| enabled(step.state.state, *code));
        let taken = cycle
            .iter()
            .skip(1)
            .filter_map(|step| step.action.as_deref())
            .any(|label| action_code(label) == *code);
        assert!(
            !enabled_infinitely_often || taken,
            "strong obligation missing from cycle"
        );
    }
}

#[test]
fn empty_and_single_class_profiles_delegate_exactly_under_bounds() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let product_limits = transition_limit(2);
    let staged_limits = AnalysisLimits::new(transition_limit(2), transition_limit(2));

    let none = FairnessProfile::none();
    assert_eq!(
        check_buchi_with_fairness_profile_and_product_limits(
            &model,
            &automaton,
            &none,
            product_limits,
        )
        .unwrap(),
        check_buchi_with_product_limits(&model, &automaton, product_limits).unwrap()
    );
    assert_eq!(
        check_buchi_with_fairness_profile_and_limits(&model, &automaton, &none, staged_limits)
            .unwrap(),
        check_buchi_with_limits(&model, &automaton, staged_limits).unwrap()
    );

    let weak_profile = FairnessProfile::new(["pulse-b"], Vec::<&str>::new()).unwrap();
    let weak = WeakFairness::new(["pulse-b"]).unwrap();
    assert_eq!(
        check_buchi_with_fairness_profile_and_product_limits(
            &model,
            &automaton,
            &weak_profile,
            product_limits,
        )
        .unwrap(),
        check_buchi_with_weak_fairness_and_product_limits(
            &model,
            &automaton,
            &weak,
            product_limits,
        )
        .unwrap()
    );
    assert_eq!(
        check_buchi_with_fairness_profile_and_limits(
            &model,
            &automaton,
            &weak_profile,
            staged_limits,
        )
        .unwrap(),
        check_buchi_with_weak_fairness_and_limits(&model, &automaton, &weak, staged_limits)
            .unwrap()
    );

    let strong_profile = FairnessProfile::new(Vec::<&str>::new(), ["pulse-b"]).unwrap();
    let strong = StrongFairness::new(["pulse-b"]).unwrap();
    assert_eq!(
        check_buchi_with_fairness_profile_and_product_limits(
            &model,
            &automaton,
            &strong_profile,
            product_limits,
        )
        .unwrap(),
        check_buchi_with_strong_fairness_and_product_limits(
            &model,
            &automaton,
            &strong,
            product_limits,
        )
        .unwrap()
    );
    assert_eq!(
        check_buchi_with_fairness_profile_and_limits(
            &model,
            &automaton,
            &strong_profile,
            staged_limits,
        )
        .unwrap(),
        check_buchi_with_strong_fairness_and_limits(&model, &automaton, &strong, staged_limits)
            .unwrap()
    );
}

#[test]
fn generous_limits_preserve_unbounded_mixed_result_and_evidence() {
    let model = graph_model([1, 2, 1, 0]);
    let automaton = reject_all_automaton();
    let profile = FairnessProfile::new(["w"], ["s"]).unwrap();
    let unbounded = check_buchi_with_fairness_profile(&model, &automaton, &profile).unwrap();

    let product = check_buchi_with_fairness_profile_and_product_limits(
        &model,
        &automaton,
        &profile,
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        product.outcome,
        BoundedOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(product.model_states, unbounded.model_states);
    assert_eq!(product.model_transitions, unbounded.model_transitions);
    assert_eq!(product.product_states, unbounded.product_states);
    assert_eq!(
        product.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(product.counterexample, unbounded.counterexample);

    let staged = check_buchi_with_fairness_profile_and_limits(
        &model,
        &automaton,
        &profile,
        AnalysisLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(staged.model_states, unbounded.model_states);
    assert_eq!(staged.product_states, unbounded.product_states);
    assert_eq!(
        staged.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(staged.counterexample, unbounded.counterexample);
}

#[test]
fn mixed_fairness_never_filters_strict_finite_terminal_failure() {
    let model = finite_quiet_run().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();
    let profile = FairnessProfile::new(["pulse-a"], ["pulse-b"]).unwrap();

    let product = check_buchi_with_fairness_profile_and_product_limits(
        &model,
        &automaton,
        &profile,
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        product.outcome,
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert!(matches!(
        product.counterexample,
        Some(BuchiCounterexample::FiniteTerminal { .. })
    ));

    let staged = check_buchi_with_fairness_profile_and_limits(
        &model,
        &automaton,
        &profile,
        AnalysisLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert!(matches!(
        staged.counterexample,
        Some(BuchiCounterexample::FiniteTerminal { .. })
    ));
}

#[test]
fn staged_product_cutoff_is_reported_at_product_stage() {
    let model = graph_model([0, 3, 0, 3]);
    let automaton = reject_all_automaton();
    let profile = FairnessProfile::new(["w"], ["s"]).unwrap();
    let limits = AnalysisLimits::new(ExplorationLimits::unbounded(), transition_limit(0));

    let result =
        check_buchi_with_fairness_profile_and_limits(&model, &automaton, &profile, limits).unwrap();
    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Product,
            reason: InconclusiveReason::TransitionLimitReached { limit: 0 },
        })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn product_and_staged_transition_limits_match_independent_mixed_oracle() {
    let automaton = reject_all_automaton();
    let mixed = FairnessProfile::new(["w"], ["s"]).unwrap();
    let overlap = FairnessProfile::new(["w"], ["w"]).unwrap();

    for assignment in 0usize..CODE_COUNT.pow(EDGE_COUNT as u32) {
        let codes = decode(assignment);
        let full = oracle_prefix(codes, usize::MAX);
        let model = graph_model(codes);

        for (profile, weak_codes, strong_codes, label) in [
            (&mixed, &[1u8][..], &[2u8][..], "mixed"),
            (&overlap, &[][..], &[1u8][..], "overlap"),
        ] {
            for limit in 0usize..=EDGE_COUNT {
                let prefix = oracle_prefix(codes, limit);

                let product_expected =
                    oracle_has_fair_cycle(codes, prefix, weak_codes, strong_codes, false);
                let product = check_buchi_with_fairness_profile_and_product_limits(
                    &model,
                    &automaton,
                    profile,
                    transition_limit(limit),
                )
                .unwrap();
                assert_eq!(
                    product.model_states,
                    full.discovered.iter().filter(|seen| **seen).count(),
                    "product model states: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    product.model_transitions, full.explored_transitions,
                    "product model transitions: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    product.product_states,
                    prefix.discovered.iter().filter(|seen| **seen).count(),
                    "product states: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    product.checked_product_states, prefix.checked_states,
                    "product checked: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    product.explored_product_transitions, prefix.explored_transitions,
                    "product explored: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    product.retained_product_transitions, prefix.explored_transitions,
                    "product retained: {label} assignment={assignment} limit={limit}"
                );
                if product_expected {
                    assert_eq!(
                        product.outcome,
                        BoundedOutcome::Conclusive(BuchiStatus::Violated),
                        "product violation: {label} assignment={assignment} limit={limit}"
                    );
                    let Some(BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. }) =
                        product.counterexample.as_ref()
                    else {
                        panic!(
                            "expected product cycle: {label} assignment={assignment} limit={limit}"
                        );
                    };
                    assert_cycle_uses_retained_edges_and_is_fair(
                        codes,
                        prefix,
                        cycle,
                        weak_codes,
                        strong_codes,
                        false,
                    );
                } else if prefix.complete {
                    assert_eq!(
                        product.outcome,
                        BoundedOutcome::Conclusive(BuchiStatus::Satisfied),
                        "product satisfied: {label} assignment={assignment} limit={limit}"
                    );
                    assert!(product.counterexample.is_none());
                } else {
                    assert_eq!(
                        product.outcome,
                        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached {
                            limit
                        }),
                        "product inconclusive: {label} assignment={assignment} limit={limit}"
                    );
                    assert!(product.counterexample.is_none());
                }

                let staged_expected =
                    oracle_has_fair_cycle(codes, prefix, weak_codes, strong_codes, true);
                let staged = check_buchi_with_fairness_profile_and_limits(
                    &model,
                    &automaton,
                    profile,
                    AnalysisLimits::new(transition_limit(limit), ExplorationLimits::unbounded()),
                )
                .unwrap();
                assert_eq!(
                    staged.model_states,
                    prefix.discovered.iter().filter(|seen| **seen).count(),
                    "staged model states: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    staged.checked_model_states, prefix.checked_states,
                    "staged model checked: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    staged.explored_model_transitions, prefix.explored_transitions,
                    "staged model explored: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    staged.retained_model_transitions, prefix.explored_transitions,
                    "staged model retained: {label} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    staged.product_completion,
                    BoundedOutcome::Conclusive(()),
                    "staged product completion: {label} assignment={assignment} limit={limit}"
                );
                if staged_expected {
                    assert_eq!(
                        staged.outcome,
                        AnalysisOutcome::Conclusive(BuchiStatus::Violated),
                        "staged violation: {label} assignment={assignment} limit={limit}"
                    );
                    let Some(BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. }) =
                        staged.counterexample.as_ref()
                    else {
                        panic!(
                            "expected staged cycle: {label} assignment={assignment} limit={limit}"
                        );
                    };
                    assert_cycle_uses_retained_edges_and_is_fair(
                        codes,
                        prefix,
                        cycle,
                        weak_codes,
                        strong_codes,
                        true,
                    );
                } else if prefix.complete {
                    assert_eq!(
                        staged.outcome,
                        AnalysisOutcome::Conclusive(BuchiStatus::Satisfied),
                        "staged satisfied: {label} assignment={assignment} limit={limit}"
                    );
                    assert!(staged.counterexample.is_none());
                } else {
                    assert_eq!(
                        staged.outcome,
                        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
                            stage: AnalysisStage::Model,
                            reason: InconclusiveReason::TransitionLimitReached { limit },
                        }),
                        "staged inconclusive: {label} assignment={assignment} limit={limit}"
                    );
                    assert!(staged.counterexample.is_none());
                }
            }
        }
    }
}

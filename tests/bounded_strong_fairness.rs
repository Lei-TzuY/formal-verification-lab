use formal_verification_lab::buchi::{
    check_buchi_with_limits, check_buchi_with_product_limits, AcceptanceSet, BuchiAutomaton,
    BuchiCounterexample, BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{
    finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::{
    check_buchi_with_strong_fairness, check_buchi_with_strong_fairness_and_limits,
    check_buchi_with_strong_fairness_and_product_limits, AnalysisInconclusiveReason,
    AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits,
    InconclusiveReason, Invariant, StateVariable, StrongFairness, TraceStep, Transition,
    TransitionSystem,
};
use std::collections::VecDeque;

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const ACTION_COUNT: usize = 4;

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

fn staged_model_transition_limit(limit: usize) -> AnalysisLimits {
    AnalysisLimits::new(transition_limit(limit), ExplorationLimits::unbounded())
}

#[test]
fn empty_strong_fairness_is_exact_bounded_buchi_compatibility() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let limits = transition_limit(2);

    let expected = check_buchi_with_product_limits(&model, &automaton, limits).unwrap();
    let actual = check_buchi_with_strong_fairness_and_product_limits(
        &model,
        &automaton,
        &StrongFairness::none(),
        limits,
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn product_cutoff_does_not_hide_enabled_strong_fair_action() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["pulse-b"]).unwrap();

    // The prefix retains First --pulse-a--> Second and the Second pulse-a
    // self-loop, then stops before retaining the enabled pulse-b edge. Complete
    // model capture still proves pulse-b enabled at Second, so the self-loop is
    // not a strong-fair counterexample and the cutoff stays inconclusive.
    let result = check_buchi_with_strong_fairness_and_product_limits(
        &model,
        &automaton,
        &fairness,
        transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert!(result.counterexample.is_none());
    assert_eq!(result.explored_product_transitions, 2);
    assert_eq!(result.retained_product_transitions, 2);
}

#[test]
fn retained_taken_strong_fair_edge_is_conclusive_before_product_cutoff() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["pulse-a"]).unwrap();

    let result = check_buchi_with_strong_fairness_and_product_limits(
        &model,
        &automaton,
        &fairness,
        transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    );
    let Some(BuchiCounterexample::AcceptanceAvoidingCycle {
        acceptance, cycle, ..
    }) = result.counterexample
    else {
        panic!("expected retained strong-fair acceptance-avoiding cycle");
    };
    assert_eq!(acceptance, "pulse-b-observed");
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("pulse-a")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn generous_product_budget_matches_unbounded_strong_fair_result() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["pulse-b"]).unwrap();

    let unbounded = check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
    let bounded = check_buchi_with_strong_fairness_and_product_limits(
        &model,
        &automaton,
        &fairness,
        ExplorationLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(
        bounded.outcome,
        BoundedOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(bounded.model_states, unbounded.model_states);
    assert_eq!(bounded.model_transitions, unbounded.model_transitions);
    assert_eq!(bounded.product_states, unbounded.product_states);
    assert_eq!(
        bounded.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(bounded.counterexample, unbounded.counterexample);
}

#[test]
fn strong_fairness_never_changes_strict_finite_terminal_failure_under_bounds() {
    let model = finite_quiet_run().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();
    let fairness = StrongFairness::new(["pulse-b"]).unwrap();

    let result = check_buchi_with_strong_fairness_and_product_limits(
        &model,
        &automaton,
        &fairness,
        ExplorationLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(BuchiCounterexample::FiniteTerminal { .. })
    ));
}

#[test]
fn empty_strong_fairness_is_exact_staged_buchi_compatibility() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let limits = staged_model_transition_limit(2);

    let expected = check_buchi_with_limits(&model, &automaton, limits).unwrap();
    let actual = check_buchi_with_strong_fairness_and_limits(
        &model,
        &automaton,
        &StrongFairness::none(),
        limits,
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn staged_model_cutoff_does_not_hide_enabled_strong_fair_action() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["pulse-b"]).unwrap();

    // Second's full successor vector is evaluated before the transition budget
    // prevents retaining pulse-b. Its enablement therefore remains authoritative.
    let result = check_buchi_with_strong_fairness_and_limits(
        &model,
        &automaton,
        &fairness,
        staged_model_transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::TransitionLimitReached { limit: 2 },
        })
    );
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert_eq!(result.product_completion, BoundedOutcome::Conclusive(()));
    assert!(result.counterexample.is_none());
    assert_eq!(result.retained_model_transitions, 2);
}

#[test]
fn staged_taken_strong_fair_edge_remains_conclusive_before_model_cutoff() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["pulse-a"]).unwrap();

    let result = check_buchi_with_strong_fairness_and_limits(
        &model,
        &automaton,
        &fairness,
        staged_model_transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(BuchiStatus::Violated)
    );
    let Some(BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. }) = result.counterexample
    else {
        panic!("expected retained strong-fair cycle");
    };
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("pulse-a")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn staged_proven_disabled_action_remains_conclusive_before_model_cutoff() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["never-enabled"]).unwrap();

    let result = check_buchi_with_strong_fairness_and_limits(
        &model,
        &automaton,
        &fairness,
        staged_model_transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(BuchiCounterexample::AcceptanceAvoidingCycle { .. })
    ));
}

#[test]
fn generous_staged_budget_matches_unbounded_strong_fair_result() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = StrongFairness::new(["pulse-b"]).unwrap();

    let unbounded = check_buchi_with_strong_fairness(&model, &automaton, &fairness).unwrap();
    let staged = check_buchi_with_strong_fairness_and_limits(
        &model,
        &automaton,
        &fairness,
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
        format!("bounded-strong-fair-oracle-{mask}"),
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

#[derive(Debug, Clone, Copy)]
struct OraclePrefix {
    discovered: [bool; N],
    retained: [[bool; N]; N],
    complete: bool,
    checked_states: usize,
    explored_transitions: usize,
}

fn oracle_prefix(mask: usize, transition_limit: usize) -> OraclePrefix {
    let mut discovered = [false; N];
    let mut retained = [[false; N]; N];
    let mut queue = VecDeque::new();
    discovered[0] = true;
    queue.push_back(0usize);
    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(from) = queue.pop_front() {
        checked_states += 1;
        for to in 0..N {
            if !has_edge(mask, from, to) {
                continue;
            }
            if explored_transitions >= transition_limit {
                return OraclePrefix {
                    discovered,
                    retained,
                    complete: false,
                    checked_states,
                    explored_transitions,
                };
            }
            explored_transitions += 1;
            if !discovered[to] {
                discovered[to] = true;
                queue.push_back(to);
            }
            retained[from][to] = true;
        }
    }

    OraclePrefix {
        discovered,
        retained,
        complete: true,
        checked_states,
        explored_transitions,
    }
}

fn subset_contains(subset: usize, node: usize) -> bool {
    subset & (1usize << node) != 0
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

fn subset_is_cyclic_strongly_connected(retained: &[[bool; N]; N], subset: usize) -> bool {
    let members = (0..N)
        .filter(|node| subset_contains(subset, *node))
        .collect::<Vec<_>>();
    if members.is_empty() {
        return false;
    }
    if members.len() == 1 && !retained[members[0]][members[0]] {
        return false;
    }
    members.iter().all(|from| {
        members
            .iter()
            .all(|to| restricted_reachable(retained, subset, *from, *to))
    })
}

fn subset_satisfies_strong_fairness(
    mask: usize,
    actions: [u8; EDGE_COUNT],
    retained: &[[bool; N]; N],
    subset: usize,
) -> bool {
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
                        && retained[from][to]
                        && actions[edge_index(from, to)] == fair_code
                })
        });
        !enabled || internally_taken
    })
}

fn oracle_has_retained_strong_fair_cycle(
    mask: usize,
    actions: [u8; EDGE_COUNT],
    prefix: OraclePrefix,
) -> bool {
    (1usize..(1usize << N)).any(|subset| {
        (0..N).all(|node| !subset_contains(subset, node) || prefix.discovered[node])
            && subset_is_cyclic_strongly_connected(&prefix.retained, subset)
            && subset_satisfies_strong_fairness(mask, actions, &prefix.retained, subset)
    })
}

fn assert_cycle_uses_retained_edges(
    actions: [u8; EDGE_COUNT],
    prefix: OraclePrefix,
    cycle: &[TraceStep<formal_verification_lab::BuchiProductState<usize, ()>>],
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
        assert!(prefix.retained[from][to]);
        assert_eq!(action, action_label(actions[edge_index(from, to)]));
    }
}

#[test]
fn two_node_graph_actions_and_transition_limits_match_independent_prefix_oracle() {
    let fairness = StrongFairness::new(["fair-a", "fair-b"]).unwrap();
    let automaton = never_accepting_automaton();

    for graph_mask in 0usize..(1usize << EDGE_COUNT) {
        for assignment in 0usize..ACTION_COUNT.pow(EDGE_COUNT as u32) {
            let actions = decode_actions(assignment);
            for limit in 0usize..=EDGE_COUNT {
                let prefix = oracle_prefix(graph_mask, limit);
                let expected_violation =
                    oracle_has_retained_strong_fair_cycle(graph_mask, actions, prefix);
                let model = generated_model(graph_mask, actions);
                let result = check_buchi_with_strong_fairness_and_product_limits(
                    &model,
                    &automaton,
                    &fairness,
                    transition_limit(limit),
                )
                .unwrap();
                let repeated = check_buchi_with_strong_fairness_and_product_limits(
                    &model,
                    &automaton,
                    &fairness,
                    transition_limit(limit),
                )
                .unwrap();

                assert_eq!(
                    result, repeated,
                    "determinism graph={graph_mask} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    result.product_states,
                    prefix.discovered.iter().filter(|seen| **seen).count(),
                    "states graph={graph_mask} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    result.checked_product_states, prefix.checked_states,
                    "checked graph={graph_mask} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    result.explored_product_transitions, prefix.explored_transitions,
                    "transitions graph={graph_mask} assignment={assignment} limit={limit}"
                );
                assert_eq!(
                    result.retained_product_transitions, prefix.explored_transitions,
                    "retained graph={graph_mask} assignment={assignment} limit={limit}"
                );

                if expected_violation {
                    assert_eq!(
                        result.outcome,
                        BoundedOutcome::Conclusive(BuchiStatus::Violated),
                        "violation graph={graph_mask} assignment={assignment} limit={limit}"
                    );
                    let Some(BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. }) =
                        result.counterexample
                    else {
                        panic!(
                            "expected cycle graph={graph_mask} assignment={assignment} limit={limit}"
                        );
                    };
                    assert_cycle_uses_retained_edges(actions, prefix, &cycle);
                } else if prefix.complete {
                    assert_eq!(
                        result.outcome,
                        BoundedOutcome::Conclusive(BuchiStatus::Satisfied),
                        "satisfied graph={graph_mask} assignment={assignment} limit={limit}"
                    );
                    assert!(result.counterexample.is_none());
                } else {
                    assert_eq!(
                        result.outcome,
                        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached {
                            limit
                        }),
                        "inconclusive graph={graph_mask} assignment={assignment} limit={limit}"
                    );
                    assert!(result.counterexample.is_none());
                }
            }
        }
    }
}

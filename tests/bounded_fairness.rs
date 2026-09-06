use formal_verification_lab::buchi::{
    check_buchi_with_limits, check_buchi_with_product_limits, BuchiCounterexample, BuchiStatus,
    FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{
    finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::{
    check_buchi_with_weak_fairness, check_buchi_with_weak_fairness_and_limits,
    check_buchi_with_weak_fairness_and_product_limits, AnalysisInconclusiveReason, AnalysisLimits,
    AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits, InconclusiveReason,
    WeakFairness,
};

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
fn empty_fairness_is_exact_bounded_buchi_compatibility() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let limits = transition_limit(2);

    let expected = check_buchi_with_product_limits(&model, &automaton, limits).unwrap();
    let actual = check_buchi_with_weak_fairness_and_product_limits(
        &model,
        &automaton,
        &WeakFairness::none(),
        limits,
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn hidden_enabled_fair_edge_does_not_become_a_disabled_state() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    // Product exploration retains First --pulse-a--> Second and the
    // Second --pulse-a--> Second self-loop, then cuts off immediately before
    // the enabled pulse-b edge. The retained self-loop is not weakly fair.
    let result = check_buchi_with_weak_fairness_and_product_limits(
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
fn unrelated_fairness_keeps_a_real_retained_cycle_conclusive() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-a"]).unwrap();

    let result = check_buchi_with_weak_fairness_and_product_limits(
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
        panic!("expected retained acceptance-avoiding cycle");
    };
    assert_eq!(acceptance, "pulse-b-observed");
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("pulse-a")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn generous_product_budget_matches_unbounded_weak_fair_result() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    let unbounded = check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap();
    let bounded = check_buchi_with_weak_fairness_and_product_limits(
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
fn weak_fairness_does_not_change_strict_finite_terminal_failure() {
    let model = finite_quiet_run().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    let result = check_buchi_with_weak_fairness_and_product_limits(
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
fn empty_fairness_is_exact_staged_buchi_compatibility() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let limits = staged_model_transition_limit(2);

    let expected = check_buchi_with_limits(&model, &automaton, limits).unwrap();
    let actual = check_buchi_with_weak_fairness_and_limits(
        &model,
        &automaton,
        &WeakFairness::none(),
        limits,
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn staged_model_cutoff_does_not_hide_enabled_fair_action() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    // The model prefix retains First --pulse-a--> Second and the Second
    // --pulse-a--> Second self-loop, then reaches the transition limit before
    // retaining Second --pulse-b--> First. The successor vector for Second was
    // already evaluated in full, so pulse-b is proven enabled and the retained
    // self-loop cannot be admitted as a weakly-fair counterexample.
    let result = check_buchi_with_weak_fairness_and_limits(
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
fn staged_taken_fair_edge_remains_conclusive_before_model_cutoff() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-a"]).unwrap();

    let result = check_buchi_with_weak_fairness_and_limits(
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
        panic!("expected retained weakly-fair cycle");
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
    let fairness = WeakFairness::new(["never-enabled"]).unwrap();

    let result = check_buchi_with_weak_fairness_and_limits(
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
fn generous_staged_budget_matches_unbounded_weak_fair_result() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    let unbounded = check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap();
    let staged = check_buchi_with_weak_fairness_and_limits(
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

#[test]
fn staged_weak_fairness_preserves_strict_finite_terminal_failure() {
    let model = finite_quiet_run().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    let result = check_buchi_with_weak_fairness_and_limits(
        &model,
        &automaton,
        &fairness,
        AnalysisLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(BuchiCounterexample::FiniteTerminal { .. })
    ));
}

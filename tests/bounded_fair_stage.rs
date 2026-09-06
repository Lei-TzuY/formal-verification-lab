use formal_verification_lab::buchi::{BuchiCounterexample, BuchiStatus, FiniteRunPolicy};
use formal_verification_lab::buchi_examples::{pulse_automaton, unfair_second_pulse};
use formal_verification_lab::{
    check_buchi_with_weak_fairness_and_limits, AnalysisInconclusiveReason, AnalysisLimits,
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

fn state_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: Some(limit),
        max_transitions: None,
        max_depth: None,
    }
}

#[test]
fn staged_product_cutoff_reports_product_stage_under_weak_fairness() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();
    let limits = AnalysisLimits::new(ExplorationLimits::unbounded(), transition_limit(2));

    let result =
        check_buchi_with_weak_fairness_and_limits(&model, &automaton, &fairness, limits).unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Product,
            reason: InconclusiveReason::TransitionLimitReached { limit: 2 },
        })
    );
    assert_eq!(result.model_completion, BoundedOutcome::Conclusive(()));
    assert_eq!(
        result.product_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn staged_product_cutoff_preserves_a_retained_weakly_fair_cycle() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-a"]).unwrap();
    let limits = AnalysisLimits::new(ExplorationLimits::unbounded(), transition_limit(2));

    let result =
        check_buchi_with_weak_fairness_and_limits(&model, &automaton, &fairness, limits).unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert_eq!(result.model_completion, BoundedOutcome::Conclusive(()));
    assert_eq!(
        result.product_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
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
fn simultaneous_staged_cutoffs_keep_model_stage_precedence() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();
    let limits = AnalysisLimits::new(transition_limit(2), state_limit(1));

    let result =
        check_buchi_with_weak_fairness_and_limits(&model, &automaton, &fairness, limits).unwrap();

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
    assert_eq!(
        result.product_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
    assert!(result.counterexample.is_none());
}

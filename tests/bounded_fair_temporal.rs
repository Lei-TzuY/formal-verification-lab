use formal_verification_lab::buchi_examples::unfair_second_pulse;
use formal_verification_lab::{
    check_action_temporal_with_limits, check_action_temporal_with_product_limits,
    check_action_temporal_with_weak_fairness, check_action_temporal_with_weak_fairness_and_limits,
    check_action_temporal_with_weak_fairness_and_product_limits, ActionAtom, ActionTemporalSpec,
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
    ExplorationLimits, InconclusiveReason, TemporalError, TemporalObligation, TemporalStatus,
    WeakFairness,
};

fn recurring_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::all_infinitely_often(
        "both-pulses",
        vec![
            ActionAtom::exact("pulse-a").unwrap(),
            ActionAtom::exact("pulse-b").unwrap(),
        ],
    )
    .unwrap()
}

fn response_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::response(
        "respond",
        ActionAtom::exact("request").unwrap(),
        ActionAtom::exact("grant").unwrap(),
    )
    .unwrap()
}

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

#[test]
fn empty_fairness_is_exact_product_bounded_temporal_compatibility() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let limits = transition_limit(2);

    let expected = check_action_temporal_with_product_limits(&model, &spec, limits).unwrap();
    let actual = check_action_temporal_with_weak_fairness_and_product_limits(
        &model,
        &spec,
        &WeakFairness::none(),
        limits,
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn product_cutoff_hidden_fair_edge_stays_inconclusive_through_temporal_frontend() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    let result = check_action_temporal_with_weak_fairness_and_product_limits(
        &model,
        &spec,
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
fn retained_taken_fair_cycle_stays_conclusive_through_temporal_frontend() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let fairness = WeakFairness::new(["pulse-a"]).unwrap();

    let result = check_action_temporal_with_weak_fairness_and_product_limits(
        &model,
        &spec,
        &fairness,
        transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(TemporalStatus::Violated)
    );
    let counterexample = result.counterexample.expect("real retained lasso");
    let formal_verification_lab::TemporalCounterexample::Infinite {
        obligation,
        cycle,
        ..
    } = counterexample
    else {
        panic!("expected infinite temporal counterexample");
    };
    assert_eq!(
        obligation,
        TemporalObligation::InfinitelyOftenAction("pulse-b".to_owned())
    );
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("pulse-a")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn empty_fairness_is_exact_staged_temporal_compatibility() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let limits = AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded());

    let expected = check_action_temporal_with_limits(&model, &spec, limits).unwrap();
    let actual = check_action_temporal_with_weak_fairness_and_limits(
        &model,
        &spec,
        &WeakFairness::none(),
        limits,
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn staged_model_cutoff_hidden_fair_edge_stays_model_inconclusive() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();
    let limits = AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded());

    let result =
        check_action_temporal_with_weak_fairness_and_limits(&model, &spec, &fairness, limits)
            .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::TransitionLimitReached { limit: 2 },
        })
    );
    assert!(result.counterexample.is_none());
    assert_eq!(result.retained_model_transitions, 2);
}

#[test]
fn generous_staged_limits_match_unbounded_weak_fair_temporal_result() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let fairness = WeakFairness::new(["pulse-b"]).unwrap();

    let unbounded = check_action_temporal_with_weak_fairness(&model, &spec, &fairness).unwrap();
    let staged = check_action_temporal_with_weak_fairness_and_limits(
        &model,
        &spec,
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
fn response_fairness_still_fails_closed_under_both_limit_surfaces() {
    let model = unfair_second_pulse().unwrap();
    let spec = response_spec();
    let fairness = WeakFairness::new(["grant"]).unwrap();

    let product = check_action_temporal_with_weak_fairness_and_product_limits(
        &model,
        &spec,
        &fairness,
        ExplorationLimits::unbounded(),
    );
    assert!(matches!(
        product,
        Err(TemporalError::WeakFairnessUnsupportedForResponse)
    ));

    let staged = check_action_temporal_with_weak_fairness_and_limits(
        &model,
        &spec,
        &fairness,
        AnalysisLimits::unbounded(),
    );
    assert!(matches!(
        staged,
        Err(TemporalError::WeakFairnessUnsupportedForResponse)
    ));
}

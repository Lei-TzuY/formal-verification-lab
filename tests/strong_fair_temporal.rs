use formal_verification_lab::{
    check_action_temporal, check_action_temporal_with_limits,
    check_action_temporal_with_product_limits, check_action_temporal_with_strong_fairness,
    check_action_temporal_with_strong_fairness_and_limits,
    check_action_temporal_with_strong_fairness_and_product_limits,
    check_action_temporal_with_weak_fairness, ActionAtom, ActionTemporalSpec, AnalysisLimits,
    AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits, InconclusiveReason,
    Invariant, StateVariable, StrongFairness, TemporalBackend, TemporalObligation, TemporalStatus,
    Transition, TransitionSystem, WeakFairness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Phase {
    Idle,
    Offer,
    Away,
}

fn intermittent_grant_model() -> TransitionSystem<Phase> {
    TransitionSystem::new(
        "intermittent-grant-temporal",
        vec![StateVariable::new("phase", "protocol phase")],
        vec![Phase::Idle],
        |state| match state {
            Phase::Idle => Ok(vec![Transition::new("request", Phase::Offer)]),
            Phase::Offer => Ok(vec![
                Transition::new("grant", Phase::Idle),
                Transition::new("defer", Phase::Away),
            ]),
            Phase::Away => Ok(vec![Transition::new("return", Phase::Offer)]),
        },
        vec![Invariant::new("recognized-phase", |_state| true)],
    )
    .unwrap()
}

fn response_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::response(
        "request-eventually-grant",
        ActionAtom::exact("request").unwrap(),
        ActionAtom::exact("grant").unwrap(),
    )
    .unwrap()
}

fn recurring_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::all_infinitely_often(
        "grant-infinitely-often",
        vec![ActionAtom::exact("grant").unwrap()],
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
fn strong_fair_response_frontend_distinguishes_intermittent_enablement_from_weak_fairness() {
    let model = intermittent_grant_model();
    let spec = response_spec();

    let baseline = check_action_temporal(&model, &spec).unwrap();
    assert_eq!(baseline.backend, TemporalBackend::Response);
    assert_eq!(baseline.status, TemporalStatus::Violated);

    let weak = check_action_temporal_with_weak_fairness(
        &model,
        &spec,
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(weak.backend, TemporalBackend::Response);
    assert_eq!(weak.status, TemporalStatus::Violated);

    let strong = check_action_temporal_with_strong_fairness(
        &model,
        &spec,
        &StrongFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(strong.backend, TemporalBackend::Response);
    assert_eq!(strong.status, TemporalStatus::Satisfied);
    assert!(strong.counterexample.is_none());
}

#[test]
fn strong_fair_recurring_frontend_reuses_buchi_backend() {
    let model = intermittent_grant_model();
    let spec = recurring_spec();

    let weak = check_action_temporal_with_weak_fairness(
        &model,
        &spec,
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(weak.backend, TemporalBackend::Buchi);
    assert_eq!(weak.status, TemporalStatus::Violated);

    let strong = check_action_temporal_with_strong_fairness(
        &model,
        &spec,
        &StrongFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(strong.backend, TemporalBackend::Buchi);
    assert_eq!(strong.status, TemporalStatus::Satisfied);
    assert!(strong.counterexample.is_none());
}

#[test]
fn product_cutoff_propagates_through_strong_fair_response_frontend() {
    let result = check_action_temporal_with_strong_fairness_and_product_limits(
        &intermittent_grant_model(),
        &response_spec(),
        &StrongFairness::new(["grant"]).unwrap(),
        transition_limit(2),
    )
    .unwrap();

    assert_eq!(result.backend, TemporalBackend::Response);
    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn staged_model_cutoff_keeps_stage_provenance_through_temporal_frontend() {
    let result = check_action_temporal_with_strong_fairness_and_limits(
        &intermittent_grant_model(),
        &response_spec(),
        &StrongFairness::new(["grant"]).unwrap(),
        AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(result.backend, TemporalBackend::Response);
    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(formal_verification_lab::AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::TransitionLimitReached { limit: 2 },
        })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn empty_strong_fairness_exactly_preserves_both_typed_temporal_backends() {
    let model = intermittent_grant_model();
    let fairness = StrongFairness::none();

    for spec in [response_spec(), recurring_spec()] {
        assert_eq!(
            check_action_temporal_with_strong_fairness(&model, &spec, &fairness).unwrap(),
            check_action_temporal(&model, &spec).unwrap()
        );

        let product_limits = transition_limit(2);
        assert_eq!(
            check_action_temporal_with_strong_fairness_and_product_limits(
                &model,
                &spec,
                &fairness,
                product_limits,
            )
            .unwrap(),
            check_action_temporal_with_product_limits(&model, &spec, product_limits).unwrap()
        );

        let staged = AnalysisLimits::new(transition_limit(2), transition_limit(2));
        assert_eq!(
            check_action_temporal_with_strong_fairness_and_limits(
                &model, &spec, &fairness, staged,
            )
            .unwrap(),
            check_action_temporal_with_limits(&model, &spec, staged).unwrap()
        );
    }
}

#[test]
fn generous_limits_preserve_strong_fair_temporal_results_for_both_backends() {
    let model = intermittent_grant_model();
    let fairness = StrongFairness::new(["grant"]).unwrap();

    for spec in [response_spec(), recurring_spec()] {
        let unbounded =
            check_action_temporal_with_strong_fairness(&model, &spec, &fairness).unwrap();
        let product = check_action_temporal_with_strong_fairness_and_product_limits(
            &model,
            &spec,
            &fairness,
            ExplorationLimits::unbounded(),
        )
        .unwrap();
        assert_eq!(
            product.outcome,
            BoundedOutcome::Conclusive(unbounded.status)
        );
        assert_eq!(product.backend, unbounded.backend);
        assert_eq!(product.model_states, unbounded.model_states);
        assert_eq!(product.model_transitions, unbounded.model_transitions);
        assert_eq!(product.product_states, unbounded.product_states);
        assert_eq!(
            product.retained_product_transitions,
            unbounded.product_transitions
        );
        assert_eq!(product.counterexample, unbounded.counterexample);

        let staged = check_action_temporal_with_strong_fairness_and_limits(
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
        assert_eq!(staged.backend, unbounded.backend);
        assert_eq!(staged.model_states, unbounded.model_states);
        assert_eq!(staged.product_states, unbounded.product_states);
        assert_eq!(
            staged.retained_product_transitions,
            unbounded.product_transitions
        );
        assert_eq!(staged.counterexample, unbounded.counterexample);
    }
}

#[test]
fn strong_fair_response_violation_remains_normalized_to_response_obligation() {
    let result = check_action_temporal_with_strong_fairness(
        &intermittent_grant_model(),
        &response_spec(),
        &StrongFairness::new(["defer"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.backend, TemporalBackend::Response);
    assert_eq!(result.status, TemporalStatus::Violated);
    let Some(formal_verification_lab::TemporalCounterexample::Infinite {
        obligation,
        cycle,
        ..
    }) = result.counterexample
    else {
        panic!("expected normalized response lasso");
    };
    assert_eq!(obligation, TemporalObligation::Response);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

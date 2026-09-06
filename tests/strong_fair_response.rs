use formal_verification_lab::{
    check_response, check_response_with_limits, check_response_with_product_limits,
    check_response_with_strong_fairness, check_response_with_strong_fairness_and_limits,
    check_response_with_strong_fairness_and_product_limits, check_response_with_weak_fairness,
    AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits,
    InconclusiveReason, Invariant, ResponseCounterexample, ResponseProperty, ResponseStatus,
    StateVariable, StrongFairness, Transition, TransitionSystem, WeakFairness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Phase {
    Idle,
    Offer,
    Away,
}

fn intermittent_grant_model() -> TransitionSystem<Phase> {
    TransitionSystem::new(
        "intermittent-grant",
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

fn finite_pending_model() -> TransitionSystem<u8> {
    TransitionSystem::new(
        "finite-pending",
        vec![StateVariable::new("phase", "protocol phase")],
        vec![0_u8],
        |state| match state {
            0 => Ok(vec![Transition::new("request", 1_u8)]),
            1 => Ok(Vec::new()),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("recognized-phase", |state: &u8| *state <= 1)],
    )
    .unwrap()
}

fn property() -> ResponseProperty {
    ResponseProperty::new(
        "request-eventually-grant",
        |action| action == "request",
        |action| action == "grant",
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
fn strong_fairness_eliminates_intermittently_enabled_grant_starvation() {
    let model = intermittent_grant_model();
    let property = property();

    let baseline = check_response(&model, &property).unwrap();
    assert_eq!(baseline.status, ResponseStatus::Violated);

    let weak = check_response_with_weak_fairness(
        &model,
        &property,
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(weak.status, ResponseStatus::Violated);

    let strong = check_response_with_strong_fairness(
        &model,
        &property,
        &StrongFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(strong.status, ResponseStatus::Satisfied);
    assert!(strong.counterexample.is_none());
}

#[test]
fn strong_fairness_for_taken_defer_preserves_the_real_response_violation() {
    let model = intermittent_grant_model();
    let result = check_response_with_strong_fairness(
        &model,
        &property(),
        &StrongFairness::new(["defer"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, ResponseStatus::Violated);
    let Some(ResponseCounterexample::Infinite { stem, cycle }) = result.counterexample else {
        panic!("expected infinite pending response counterexample");
    };
    assert!(stem.last().unwrap().state.pending);
    assert!(cycle.iter().all(|step| step.state.pending));
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("defer")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn strong_fairness_never_removes_a_finite_pending_terminal() {
    let result = check_response_with_strong_fairness(
        &finite_pending_model(),
        &property(),
        &StrongFairness::new(["grant"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, ResponseStatus::Violated);
    let Some(ResponseCounterexample::Finite { trace }) = result.counterexample else {
        panic!("expected finite pending terminal");
    };
    assert!(trace.last().unwrap().state.pending);
}

#[test]
fn empty_strong_fairness_is_exact_historical_compatibility_in_all_budget_modes() {
    let model = intermittent_grant_model();
    let property = property();
    let fairness = StrongFairness::none();

    assert_eq!(
        check_response_with_strong_fairness(&model, &property, &fairness).unwrap(),
        check_response(&model, &property).unwrap()
    );

    let product_limits = transition_limit(2);
    assert_eq!(
        check_response_with_strong_fairness_and_product_limits(
            &model,
            &property,
            &fairness,
            product_limits,
        )
        .unwrap(),
        check_response_with_product_limits(&model, &property, product_limits).unwrap()
    );

    let staged = AnalysisLimits::new(transition_limit(2), transition_limit(2));
    assert_eq!(
        check_response_with_strong_fairness_and_limits(&model, &property, &fairness, staged)
            .unwrap(),
        check_response_with_limits(&model, &property, staged).unwrap()
    );
}

#[test]
fn product_cutoff_cannot_turn_hidden_strong_fair_behavior_into_proof() {
    let result = check_response_with_strong_fairness_and_product_limits(
        &intermittent_grant_model(),
        &property(),
        &StrongFairness::new(["grant"]).unwrap(),
        transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn staged_model_cutoff_preserves_model_stage_provenance() {
    let result = check_response_with_strong_fairness_and_limits(
        &intermittent_grant_model(),
        &property(),
        &StrongFairness::new(["grant"]).unwrap(),
        AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded()),
    )
    .unwrap();

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
fn generous_product_and_staged_limits_match_unbounded_strong_fair_response() {
    let model = intermittent_grant_model();
    let property = property();
    let fairness = StrongFairness::new(["grant"]).unwrap();
    let unbounded = check_response_with_strong_fairness(&model, &property, &fairness).unwrap();

    let product = check_response_with_strong_fairness_and_product_limits(
        &model,
        &property,
        &fairness,
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

    let staged = check_response_with_strong_fairness_and_limits(
        &model,
        &property,
        &fairness,
        AnalysisLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(staged.model_states, unbounded.model_states);
    assert_eq!(
        staged.explored_model_transitions,
        unbounded.model_transitions
    );
    assert_eq!(staged.product_states, unbounded.product_states);
    assert_eq!(
        staged.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(staged.counterexample, unbounded.counterexample);
}

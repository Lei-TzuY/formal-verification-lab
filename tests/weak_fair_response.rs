use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol, RequestGrantState, RequestPhase,
};
use formal_verification_lab::{
    check_response, check_response_with_limits, check_response_with_product_limits,
    check_response_with_weak_fairness, check_response_with_weak_fairness_and_limits,
    check_response_with_weak_fairness_and_product_limits, AnalysisLimits, AnalysisOutcome,
    AnalysisStage, BoundedOutcome, ExplorationLimits, InconclusiveReason, Invariant,
    ResponseCounterexample, ResponseProperty, ResponseStatus, StateVariable, Transition,
    TransitionSystem, WeakFairness,
};

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
fn grant_weak_fairness_excludes_the_unfair_wait_cycle() {
    let model = unfair_request_grant_protocol().unwrap();
    let property = property();

    let baseline = check_response(&model, &property).unwrap();
    assert_eq!(baseline.status, ResponseStatus::Violated);

    let fair = check_response_with_weak_fairness(
        &model,
        &property,
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(fair.status, ResponseStatus::Satisfied);
    assert!(fair.counterexample.is_none());
}

#[test]
fn fairness_for_the_taken_wait_action_preserves_the_real_violation() {
    let model = unfair_request_grant_protocol().unwrap();
    let result = check_response_with_weak_fairness(
        &model,
        &property(),
        &WeakFairness::new(["wait"]).unwrap(),
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
        .any(|step| step.action.as_deref() == Some("wait")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn weak_fairness_does_not_remove_a_finite_pending_terminal() {
    let model = TransitionSystem::new(
        "finite-pending-response",
        vec![StateVariable::new("phase", "request phase")],
        vec![RequestGrantState {
            phase: RequestPhase::Idle,
        }],
        |state| match state.phase {
            RequestPhase::Idle => Ok(vec![Transition::new(
                "request",
                RequestGrantState {
                    phase: RequestPhase::Waiting,
                },
            )]),
            RequestPhase::Waiting => Ok(Vec::new()),
        },
        vec![Invariant::new("recognized-phase", |_state| true)],
    )
    .unwrap();

    let result = check_response_with_weak_fairness(
        &model,
        &property(),
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, ResponseStatus::Violated);
    let Some(ResponseCounterexample::Finite { trace }) = result.counterexample else {
        panic!("expected finite pending terminal");
    };
    assert!(trace.last().unwrap().state.pending);
}

#[test]
fn empty_fairness_is_exactly_the_existing_response_path_for_all_budget_modes() {
    let model = unfair_request_grant_protocol().unwrap();
    let property = property();
    let fairness = WeakFairness::none();

    assert_eq!(
        check_response_with_weak_fairness(&model, &property, &fairness).unwrap(),
        check_response(&model, &property).unwrap()
    );

    let product_limits = transition_limit(2);
    assert_eq!(
        check_response_with_weak_fairness_and_product_limits(
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
        check_response_with_weak_fairness_and_limits(&model, &property, &fairness, staged).unwrap(),
        check_response_with_limits(&model, &property, staged).unwrap()
    );
}

#[test]
fn product_cutoff_cannot_hide_an_enabled_fair_grant() {
    let model = unfair_request_grant_protocol().unwrap();
    let result = check_response_with_weak_fairness_and_product_limits(
        &model,
        &property(),
        &WeakFairness::new(["grant"]).unwrap(),
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
fn product_cutoff_keeps_a_retained_weakly_fair_wait_cycle_conclusive() {
    let model = unfair_request_grant_protocol().unwrap();
    let result = check_response_with_weak_fairness_and_product_limits(
        &model,
        &property(),
        &WeakFairness::new(["wait"]).unwrap(),
        transition_limit(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(ResponseStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(ResponseCounterexample::Infinite { .. })
    ));
}

#[test]
fn staged_model_cutoff_uses_complete_enablement_provenance_for_fair_grant() {
    let model = unfair_request_grant_protocol().unwrap();
    let result = check_response_with_weak_fairness_and_limits(
        &model,
        &property(),
        &WeakFairness::new(["grant"]).unwrap(),
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
fn staged_model_cutoff_preserves_a_retained_weakly_fair_wait_cycle() {
    let model = unfair_request_grant_protocol().unwrap();
    let result = check_response_with_weak_fairness_and_limits(
        &model,
        &property(),
        &WeakFairness::new(["wait"]).unwrap(),
        AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(ResponseStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(ResponseCounterexample::Infinite { .. })
    ));
}

#[test]
fn fair_response_still_satisfies_the_deterministic_protocol() {
    let model = request_grant_protocol().unwrap();
    let result = check_response_with_weak_fairness(
        &model,
        &property(),
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, ResponseStatus::Satisfied);
    assert!(result.counterexample.is_none());
}

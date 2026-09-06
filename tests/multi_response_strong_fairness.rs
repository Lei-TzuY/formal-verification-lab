use formal_verification_lab::multi_response::{
    check_multi_response_with_strong_fairness,
    check_multi_response_with_strong_fairness_and_limits,
    check_multi_response_with_strong_fairness_and_product_limits,
    check_multi_response_with_weak_fairness,
};
use formal_verification_lab::{
    check_multi_response, check_multi_response_with_limits,
    check_multi_response_with_product_limits, AnalysisLimits, AnalysisOutcome, AnalysisStage,
    BoundedOutcome, ExplorationLimits, Invariant, MultiResponseCounterexample,
    MultiResponseProperty, MultiResponseStatus, ResponseClause, StateVariable, StrongFairness,
    Transition, TransitionSystem, WeakFairness,
};

fn dual_property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "dual-request-response",
        vec![
            ResponseClause::new(
                "class-a",
                |action| action == "request-a",
                |action| action == "grant-a",
            )
            .unwrap(),
            ResponseClause::new(
                "class-b",
                |action| action == "request-b",
                |action| action == "grant-b",
            )
            .unwrap(),
        ],
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

fn generous_limits() -> ExplorationLimits {
    ExplorationLimits {
        max_states: Some(64),
        max_transitions: Some(64),
        max_depth: Some(64),
    }
}

fn intermittent_grant_b_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "intermittent-grant-b",
        vec![StateVariable::new("state", "protocol state")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("request-b", 1)]),
            1 => Ok(vec![
                Transition::new("grant-b", 0),
                Transition::new("defer-b", 2),
            ]),
            2 => Ok(vec![Transition::new("return-b", 1)]),
            _ => Ok(vec![]),
        },
        vec![Invariant::new("known-state", |state: &usize| *state <= 2)],
    )
    .unwrap()
}

fn finite_pending_b_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "finite-pending-b",
        vec![StateVariable::new("state", "protocol state")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("request-b", 1)]),
            1 => Ok(vec![]),
            _ => Ok(vec![]),
        },
        vec![Invariant::new("known-state", |state: &usize| *state <= 1)],
    )
    .unwrap()
}

fn taken_fair_action_pending_b_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "taken-strong-fair-action-pending-b",
        vec![StateVariable::new("state", "protocol state")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("request-b", 1)]),
            1 => Ok(vec![Transition::new("tick", 1)]),
            _ => Ok(vec![]),
        },
        vec![Invariant::new("known-state", |state: &usize| *state <= 1)],
    )
    .unwrap()
}

#[test]
fn strong_fairness_eliminates_intermittently_enabled_class_b_lasso_that_weak_keeps() {
    let model = intermittent_grant_b_model();
    let property = dual_property();

    let baseline = check_multi_response(&model, &property).unwrap();
    assert_eq!(baseline.status, MultiResponseStatus::Violated);

    let weak = check_multi_response_with_weak_fairness(
        &model,
        &property,
        &WeakFairness::new(["grant-b"]).unwrap(),
    )
    .unwrap();
    assert_eq!(weak.status, MultiResponseStatus::Violated);
    assert!(matches!(
        weak.counterexample,
        Some(MultiResponseCounterexample::Infinite { ref clause, .. }) if clause == "class-b"
    ));

    let strong = check_multi_response_with_strong_fairness(
        &model,
        &property,
        &StrongFairness::new(["grant-b"]).unwrap(),
    )
    .unwrap();
    assert_eq!(strong.status, MultiResponseStatus::Satisfied);
    assert!(strong.counterexample.is_none());
}

#[test]
fn unrelated_strong_fairness_preserves_the_exact_pending_clause() {
    let model = intermittent_grant_b_model();
    let property = dual_property();
    let result = check_multi_response_with_strong_fairness(
        &model,
        &property,
        &StrongFairness::new(["grant-a"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Infinite {
        clause,
        stem,
        cycle,
    }) = result.counterexample
    else {
        panic!("expected class-B infinite counterexample");
    };
    assert_eq!(clause, "class-b");
    assert!(stem.last().unwrap().state.pending[1]);
    assert!(cycle.iter().all(|step| step.state.pending[1]));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn finite_pending_terminal_is_not_erased_by_strong_fairness() {
    let model = finite_pending_b_model();
    let property = dual_property();
    let result = check_multi_response_with_strong_fairness(
        &model,
        &property,
        &StrongFairness::new(["grant-b"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Finite { clause, trace }) = result.counterexample else {
        panic!("expected finite class-B counterexample");
    };
    assert_eq!(clause, "class-b");
    assert_eq!(trace.last().unwrap().state.state, 1);
    assert!(trace.last().unwrap().state.pending[1]);
}

#[test]
fn taken_strong_fair_action_does_not_hide_a_real_pending_clause_cycle() {
    let model = taken_fair_action_pending_b_model();
    let property = dual_property();
    let result = check_multi_response_with_strong_fairness(
        &model,
        &property,
        &StrongFairness::new(["tick"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Infinite { clause, cycle, .. }) = result.counterexample
    else {
        panic!("expected infinite class-B counterexample");
    };
    assert_eq!(clause, "class-b");
    assert!(cycle.iter().all(|step| step.state.pending[1]));
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("tick")));
}

#[test]
fn empty_strong_fairness_is_exact_compatibility_for_all_limit_surfaces() {
    let model = intermittent_grant_b_model();
    let property = dual_property();
    let fairness = StrongFairness::none();

    assert_eq!(
        check_multi_response_with_strong_fairness(&model, &property, &fairness).unwrap(),
        check_multi_response(&model, &property).unwrap()
    );

    let product_limits = transition_limit(2);
    assert_eq!(
        check_multi_response_with_strong_fairness_and_product_limits(
            &model,
            &property,
            &fairness,
            product_limits,
        )
        .unwrap(),
        check_multi_response_with_product_limits(&model, &property, product_limits).unwrap()
    );

    let staged = AnalysisLimits::new(transition_limit(2), transition_limit(2));
    assert_eq!(
        check_multi_response_with_strong_fairness_and_limits(
            &model, &property, &fairness, staged,
        )
        .unwrap(),
        check_multi_response_with_limits(&model, &property, staged).unwrap()
    );
}

#[test]
fn strong_fair_product_cutoff_remains_inconclusive_when_satisfaction_is_unresolved() {
    let result = check_multi_response_with_strong_fairness_and_product_limits(
        &intermittent_grant_b_model(),
        &dual_property(),
        &StrongFairness::new(["grant-b"]).unwrap(),
        transition_limit(1),
    )
    .unwrap();

    assert!(matches!(result.outcome, BoundedOutcome::Inconclusive(_)));
    assert!(result.counterexample.is_none());
}

#[test]
fn strong_fair_staged_model_cutoff_preserves_model_stage_provenance() {
    let result = check_multi_response_with_strong_fairness_and_limits(
        &intermittent_grant_b_model(),
        &dual_property(),
        &StrongFairness::new(["grant-b"]).unwrap(),
        AnalysisLimits::new(transition_limit(1), ExplorationLimits::unbounded()),
    )
    .unwrap();

    let AnalysisOutcome::Inconclusive(reason) = result.outcome else {
        panic!("expected staged model cutoff");
    };
    assert_eq!(reason.stage, AnalysisStage::Model);
    assert!(result.counterexample.is_none());
}

#[test]
fn generous_limits_preserve_unbounded_strong_fair_result_and_evidence() {
    let model = taken_fair_action_pending_b_model();
    let property = dual_property();
    let fairness = StrongFairness::new(["tick"]).unwrap();
    let unbounded =
        check_multi_response_with_strong_fairness(&model, &property, &fairness).unwrap();

    let bounded = check_multi_response_with_strong_fairness_and_product_limits(
        &model,
        &property,
        &fairness,
        generous_limits(),
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

    let staged = check_multi_response_with_strong_fairness_and_limits(
        &model,
        &property,
        &fairness,
        AnalysisLimits::new(generous_limits(), generous_limits()),
    )
    .unwrap();
    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(staged.counterexample, unbounded.counterexample);
}

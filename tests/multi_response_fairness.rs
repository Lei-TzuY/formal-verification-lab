use formal_verification_lab::multi_response::{
    check_multi_response_with_weak_fairness, check_multi_response_with_weak_fairness_and_limits,
    check_multi_response_with_weak_fairness_and_product_limits,
};
use formal_verification_lab::multi_response_examples::unfair_dual_response_protocol;
use formal_verification_lab::{
    check_multi_response, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
    ExplorationLimits, Invariant, MultiResponseCounterexample, MultiResponseProperty,
    MultiResponseStatus, ResponseClause, StateVariable, Transition, TransitionSystem, WeakFairness,
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

fn limits(max_transitions: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(max_transitions),
        max_depth: None,
    }
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
        "taken-fair-action-pending-b",
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
fn matching_response_fairness_eliminates_only_the_unfair_class_b_lasso() {
    let model = unfair_dual_response_protocol().unwrap();
    let property = dual_property();

    let no_fairness = check_multi_response(&model, &property).unwrap();
    assert_eq!(no_fairness.status, MultiResponseStatus::Violated);
    assert!(matches!(
        no_fairness.counterexample,
        Some(MultiResponseCounterexample::Infinite { ref clause, .. }) if clause == "class-b"
    ));

    let fair_b = WeakFairness::new(["grant-b"]).unwrap();
    let fair_result = check_multi_response_with_weak_fairness(&model, &property, &fair_b).unwrap();
    assert_eq!(fair_result.status, MultiResponseStatus::Satisfied);
    assert!(fair_result.counterexample.is_none());

    let unrelated = WeakFairness::new(["grant-a"]).unwrap();
    let unrelated_result =
        check_multi_response_with_weak_fairness(&model, &property, &unrelated).unwrap();
    assert_eq!(unrelated_result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Infinite {
        clause,
        stem,
        cycle,
    }) = unrelated_result.counterexample
    else {
        panic!("expected class-B infinite counterexample");
    };
    assert_eq!(clause, "class-b");
    assert!(stem.last().unwrap().state.pending[1]);
    assert!(cycle.iter().all(|step| step.state.pending[1]));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn empty_fairness_is_exact_unbounded_compatibility() {
    let model = unfair_dual_response_protocol().unwrap();
    let property = dual_property();

    let historical = check_multi_response(&model, &property).unwrap();
    let fair_api =
        check_multi_response_with_weak_fairness(&model, &property, &WeakFairness::none()).unwrap();

    assert_eq!(fair_api, historical);
}

#[test]
fn finite_pending_terminal_is_not_erased_by_weak_fairness() {
    let model = finite_pending_b_model();
    let property = dual_property();
    let fairness = WeakFairness::new(["grant-b"]).unwrap();

    let result = check_multi_response_with_weak_fairness(&model, &property, &fairness).unwrap();
    assert_eq!(result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Finite { clause, trace }) = result.counterexample else {
        panic!("expected finite class-B counterexample");
    };
    assert_eq!(clause, "class-b");
    assert_eq!(trace.last().unwrap().state.state, 1);
    assert!(trace.last().unwrap().state.pending[1]);
}

#[test]
fn taken_fair_action_does_not_hide_a_real_pending_clause_cycle() {
    let model = taken_fair_action_pending_b_model();
    let property = dual_property();
    let fairness = WeakFairness::new(["tick"]).unwrap();

    let result = check_multi_response_with_weak_fairness(&model, &property, &fairness).unwrap();
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
fn product_cutoff_remains_inconclusive_when_fair_satisfaction_is_unresolved() {
    let model = unfair_dual_response_protocol().unwrap();
    let property = dual_property();
    let fairness = WeakFairness::new(["grant-b"]).unwrap();

    let result = check_multi_response_with_weak_fairness_and_product_limits(
        &model,
        &property,
        &fairness,
        limits(1),
    )
    .unwrap();

    assert!(matches!(result.outcome, BoundedOutcome::Inconclusive(_)));
    assert!(result.counterexample.is_none());
}

#[test]
fn staged_model_cutoff_reports_model_stage_under_weak_fairness() {
    let model = unfair_dual_response_protocol().unwrap();
    let property = dual_property();
    let fairness = WeakFairness::new(["grant-b"]).unwrap();
    let staged = AnalysisLimits::new(limits(1), ExplorationLimits::unbounded());

    let result =
        check_multi_response_with_weak_fairness_and_limits(&model, &property, &fairness, staged)
            .unwrap();

    let AnalysisOutcome::Inconclusive(reason) = result.outcome else {
        panic!("expected staged model cutoff");
    };
    assert_eq!(reason.stage, AnalysisStage::Model);
    assert!(result.counterexample.is_none());
}

#[test]
fn generous_product_limits_preserve_unbounded_fair_result_and_accounting() {
    let model = taken_fair_action_pending_b_model();
    let property = dual_property();
    let fairness = WeakFairness::new(["tick"]).unwrap();
    let unbounded = check_multi_response_with_weak_fairness(&model, &property, &fairness).unwrap();
    let generous = ExplorationLimits {
        max_states: Some(64),
        max_transitions: Some(64),
        max_depth: Some(64),
    };

    let bounded = check_multi_response_with_weak_fairness_and_product_limits(
        &model, &property, &fairness, generous,
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

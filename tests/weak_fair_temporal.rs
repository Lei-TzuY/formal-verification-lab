use formal_verification_lab::buchi_examples::unfair_second_pulse;
use formal_verification_lab::response_examples::unfair_request_grant_protocol;
use formal_verification_lab::{
    check_action_temporal, check_action_temporal_with_weak_fairness, ActionAtom,
    ActionTemporalSpec, TemporalBackend, TemporalCounterexample, TemporalObligation,
    TemporalStatus, WeakFairness,
};

fn pulse_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::all_infinitely_often(
        "infinitely-often-a-and-b",
        vec![
            ActionAtom::exact("pulse-a").unwrap(),
            ActionAtom::exact("pulse-b").unwrap(),
        ],
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

#[test]
fn empty_weak_fairness_exactly_preserves_recurring_frontend_result() {
    let model = unfair_second_pulse().unwrap();
    let spec = pulse_spec();
    assert_eq!(
        check_action_temporal_with_weak_fairness(&model, &spec, &WeakFairness::none()).unwrap(),
        check_action_temporal(&model, &spec).unwrap()
    );
}

#[test]
fn weak_fair_pulse_b_excludes_unfair_recurring_counterexample() {
    let model = unfair_second_pulse().unwrap();
    let spec = pulse_spec();
    assert_eq!(
        check_action_temporal(&model, &spec).unwrap().status,
        TemporalStatus::Violated
    );

    let fairness = WeakFairness::new(["pulse-b"]).unwrap();
    let result = check_action_temporal_with_weak_fairness(&model, &spec, &fairness).unwrap();
    assert_eq!(result.status, TemporalStatus::Satisfied);
    assert!(result.counterexample.is_none());
}

#[test]
fn weak_fair_counterexample_retains_required_taken_action() {
    let model = unfair_second_pulse().unwrap();
    let spec = pulse_spec();
    let fairness = WeakFairness::new(["pulse-a"]).unwrap();
    let result = check_action_temporal_with_weak_fairness(&model, &spec, &fairness).unwrap();
    assert_eq!(result.status, TemporalStatus::Violated);

    let Some(TemporalCounterexample::Infinite { cycle, .. }) = result.counterexample else {
        panic!("expected recurring-action lasso");
    };
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("pulse-a")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn weak_fair_response_routes_through_the_response_backend() {
    let model = unfair_request_grant_protocol().unwrap();
    let spec = response_spec();

    let baseline = check_action_temporal(&model, &spec).unwrap();
    assert_eq!(baseline.backend, TemporalBackend::Response);
    assert_eq!(baseline.status, TemporalStatus::Violated);

    let fair_grant = check_action_temporal_with_weak_fairness(
        &model,
        &spec,
        &WeakFairness::new(["grant"]).unwrap(),
    )
    .unwrap();
    assert_eq!(fair_grant.backend, TemporalBackend::Response);
    assert_eq!(fair_grant.status, TemporalStatus::Satisfied);
    assert!(fair_grant.counterexample.is_none());

    let fair_wait = check_action_temporal_with_weak_fairness(
        &model,
        &spec,
        &WeakFairness::new(["wait"]).unwrap(),
    )
    .unwrap();
    assert_eq!(fair_wait.status, TemporalStatus::Violated);
    let Some(TemporalCounterexample::Infinite {
        obligation, cycle, ..
    }) = fair_wait.counterexample
    else {
        panic!("expected weakly fair response lasso");
    };
    assert_eq!(obligation, TemporalObligation::Response);
    assert!(cycle
        .iter()
        .skip(1)
        .any(|step| step.action.as_deref() == Some("wait")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

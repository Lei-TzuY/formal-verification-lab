use formal_verification_lab::buchi_examples::{alternating_pulses, unfair_second_pulse};
use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol,
};
use formal_verification_lab::{
    check_action_temporal, check_action_temporal_with_product_limits, ActionAtom,
    ActionTemporalSpec, BoundedOutcome, ExplorationLimits, InconclusiveReason, TemporalBackend,
    TemporalCounterexample, TemporalObligation, TemporalStatus,
};

fn response_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::response(
        "request-eventually-grant",
        ActionAtom::exact("request").unwrap(),
        ActionAtom::exact("grant").unwrap(),
    )
    .unwrap()
}

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

fn max_product_states(limit: usize) -> ExplorationLimits {
    let mut limits = ExplorationLimits::unbounded();
    limits.max_states = Some(limit);
    limits
}

fn max_product_transitions(limit: usize) -> ExplorationLimits {
    let mut limits = ExplorationLimits::unbounded();
    limits.max_transitions = Some(limit);
    limits
}

fn generous_limits() -> ExplorationLimits {
    ExplorationLimits {
        max_states: Some(32),
        max_transitions: Some(64),
        max_depth: Some(16),
    }
}

#[test]
fn bounded_response_frontend_reports_product_inconclusive_without_false_proof() {
    let model = request_grant_protocol().unwrap();
    let result =
        check_action_temporal_with_product_limits(&model, &response_spec(), max_product_states(1))
            .unwrap();

    assert_eq!(result.backend, TemporalBackend::Response);
    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
    assert_eq!(result.model_states, 2);
    assert_eq!(result.model_transitions, 2);
    assert_eq!(result.product_states, 1);
    assert_eq!(result.checked_product_states, 1);
    assert_eq!(result.explored_product_transitions, 1);
    assert_eq!(result.retained_product_transitions, 0);
    assert_eq!(result.max_product_depth_reached, Some(0));
    assert!(result.counterexample.is_none());
}

#[test]
fn bounded_response_frontend_preserves_real_cycle_before_later_cutoff() {
    let model = unfair_request_grant_protocol().unwrap();
    let result = check_action_temporal_with_product_limits(
        &model,
        &response_spec(),
        max_product_transitions(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(TemporalStatus::Violated)
    );
    assert_eq!(result.backend, TemporalBackend::Response);
    let Some(TemporalCounterexample::Infinite {
        obligation,
        stem,
        cycle,
    }) = result.counterexample
    else {
        panic!("expected normalized response lasso");
    };
    assert_eq!(obligation, TemporalObligation::Response);
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("wait")));
}

#[test]
fn bounded_recurring_frontend_reports_product_inconclusive_without_false_satisfaction() {
    let model = alternating_pulses().unwrap();
    let result =
        check_action_temporal_with_product_limits(&model, &pulse_spec(), max_product_states(1))
            .unwrap();

    assert_eq!(result.backend, TemporalBackend::Buchi);
    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
    assert_eq!(result.product_states, 1);
    assert_eq!(result.explored_product_transitions, 1);
    assert_eq!(result.retained_product_transitions, 0);
    assert!(result.counterexample.is_none());
}

#[test]
fn bounded_recurring_frontend_preserves_real_acceptance_avoiding_cycle() {
    let model = unfair_second_pulse().unwrap();
    let result = check_action_temporal_with_product_limits(
        &model,
        &pulse_spec(),
        max_product_transitions(2),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(TemporalStatus::Violated)
    );
    assert_eq!(result.backend, TemporalBackend::Buchi);
    let Some(TemporalCounterexample::Infinite {
        obligation,
        stem,
        cycle,
    }) = result.counterexample
    else {
        panic!("expected normalized Buchi lasso");
    };
    assert_eq!(
        obligation,
        TemporalObligation::InfinitelyOftenAction("pulse-b".to_owned())
    );
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .all(|step| step.action.as_deref() != Some("pulse-b")));
}

#[test]
fn generous_product_limits_match_existing_unbounded_frontend_results() {
    let response_model = unfair_request_grant_protocol().unwrap();
    let response_spec = response_spec();
    let response_unbounded = check_action_temporal(&response_model, &response_spec).unwrap();
    let response_bounded = check_action_temporal_with_product_limits(
        &response_model,
        &response_spec,
        generous_limits(),
    )
    .unwrap();
    assert_eq!(
        response_bounded.outcome,
        BoundedOutcome::Conclusive(response_unbounded.status)
    );
    assert_eq!(response_bounded.backend, response_unbounded.backend);
    assert_eq!(response_bounded.property, response_unbounded.property);
    assert_eq!(
        response_bounded.model_states,
        response_unbounded.model_states
    );
    assert_eq!(
        response_bounded.model_transitions,
        response_unbounded.model_transitions
    );
    assert_eq!(
        response_bounded.product_states,
        response_unbounded.product_states
    );
    assert_eq!(
        response_bounded.retained_product_transitions,
        response_unbounded.product_transitions
    );
    assert_eq!(
        response_bounded.counterexample,
        response_unbounded.counterexample
    );

    let pulse_model = unfair_second_pulse().unwrap();
    let pulse_spec = pulse_spec();
    let pulse_unbounded = check_action_temporal(&pulse_model, &pulse_spec).unwrap();
    let pulse_bounded =
        check_action_temporal_with_product_limits(&pulse_model, &pulse_spec, generous_limits())
            .unwrap();
    assert_eq!(
        pulse_bounded.outcome,
        BoundedOutcome::Conclusive(pulse_unbounded.status)
    );
    assert_eq!(pulse_bounded.backend, pulse_unbounded.backend);
    assert_eq!(pulse_bounded.property, pulse_unbounded.property);
    assert_eq!(pulse_bounded.model_states, pulse_unbounded.model_states);
    assert_eq!(
        pulse_bounded.model_transitions,
        pulse_unbounded.model_transitions
    );
    assert_eq!(pulse_bounded.product_states, pulse_unbounded.product_states);
    assert_eq!(
        pulse_bounded.retained_product_transitions,
        pulse_unbounded.product_transitions
    );
    assert_eq!(pulse_bounded.counterexample, pulse_unbounded.counterexample);
}

#[test]
fn bounded_temporal_frontend_is_deterministic() {
    let model = unfair_second_pulse().unwrap();
    let spec = pulse_spec();
    let limits = max_product_transitions(2);
    let first = check_action_temporal_with_product_limits(&model, &spec, limits).unwrap();
    let second = check_action_temporal_with_product_limits(&model, &spec, limits).unwrap();
    assert_eq!(first, second);
}

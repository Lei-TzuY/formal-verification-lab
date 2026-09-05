use formal_verification_lab::buchi_examples::{
    alternating_pulses, finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol,
};
use formal_verification_lab::{
    check_action_temporal, check_buchi, check_response, ActionAtom, ActionTemporalSpec,
    BuchiStatus, FiniteRunPolicy, ResponseProperty, ResponseStatus, TemporalBackend,
    TemporalCounterexample, TemporalError, TemporalObligation, TemporalStatus,
};
use std::process::Command;

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

#[test]
fn typed_response_matches_direct_response_backend() {
    for model in [
        request_grant_protocol().unwrap(),
        unfair_request_grant_protocol().unwrap(),
    ] {
        let direct_property = ResponseProperty::new(
            "request-eventually-grant",
            |action| action == "request",
            |action| action == "grant",
        )
        .unwrap();
        let direct = check_response(&model, &direct_property).unwrap();
        let typed = check_action_temporal(&model, &response_spec()).unwrap();

        assert_eq!(typed.backend, TemporalBackend::Response);
        assert_eq!(
            typed.status == TemporalStatus::Satisfied,
            direct.status == ResponseStatus::Satisfied
        );
        assert_eq!(typed.model_states, direct.model_states);
        assert_eq!(typed.model_transitions, direct.model_transitions);
        assert_eq!(typed.product_states, direct.product_states);
        assert_eq!(typed.product_transitions, direct.product_transitions);
        assert_eq!(
            typed.counterexample.is_some(),
            direct.counterexample.is_some()
        );
    }
}

#[test]
fn typed_infinitely_often_matches_hand_built_buchi_examples() {
    for model in [
        alternating_pulses().unwrap(),
        unfair_second_pulse().unwrap(),
        finite_quiet_run().unwrap(),
    ] {
        let direct = check_buchi(
            &model,
            &pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap(),
        )
        .unwrap();
        let typed = check_action_temporal(&model, &pulse_spec()).unwrap();

        assert_eq!(typed.backend, TemporalBackend::Buchi);
        assert_eq!(
            typed.status == TemporalStatus::Satisfied,
            direct.status == BuchiStatus::Satisfied
        );
        assert_eq!(typed.model_states, direct.model_states);
        assert_eq!(typed.model_transitions, direct.model_transitions);
        assert_eq!(typed.product_states, direct.product_states);
        assert_eq!(typed.product_transitions, direct.product_transitions);
        assert_eq!(
            typed.counterexample.is_some(),
            direct.counterexample.is_some()
        );
    }
}

#[test]
fn typed_frontend_normalizes_backend_control_state_out_of_witnesses() {
    let response =
        check_action_temporal(&unfair_request_grant_protocol().unwrap(), &response_spec()).unwrap();
    let Some(TemporalCounterexample::Infinite {
        obligation,
        stem,
        cycle,
    }) = response.counterexample
    else {
        panic!("expected normalized response lasso");
    };
    assert_eq!(obligation, TemporalObligation::Response);
    assert!(stem
        .iter()
        .any(|step| step.action.as_deref() == Some("request")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);

    let recurring = check_action_temporal(&unfair_second_pulse().unwrap(), &pulse_spec()).unwrap();
    let Some(TemporalCounterexample::Infinite {
        obligation,
        stem: _,
        cycle,
    }) = recurring.counterexample
    else {
        panic!("expected normalized recurring-action lasso");
    };
    assert_eq!(
        obligation,
        TemporalObligation::InfinitelyOftenAction("pulse-b".to_owned())
    );
    assert!(cycle
        .iter()
        .all(|step| step.action.as_deref() != Some("pulse-b")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn typed_frontend_rejects_malformed_metadata_deterministically() {
    assert_eq!(
        ActionAtom::exact("   ").unwrap_err(),
        TemporalError::EmptyActionName
    );
    assert_eq!(
        ActionTemporalSpec::all_infinitely_often("empty", Vec::new()).unwrap_err(),
        TemporalError::NoRecurringActions
    );
    assert_eq!(
        ActionTemporalSpec::all_infinitely_often(
            "duplicate",
            vec![
                ActionAtom::exact("pulse-a").unwrap(),
                ActionAtom::exact("pulse-a").unwrap(),
            ],
        )
        .unwrap_err(),
        TemporalError::DuplicateRecurringAction {
            action: "pulse-a".to_owned()
        }
    );
    assert_eq!(
        ActionTemporalSpec::response(
            "   ",
            ActionAtom::exact("request").unwrap(),
            ActionAtom::exact("grant").unwrap(),
        )
        .unwrap_err(),
        TemporalError::EmptyPropertyName
    );
}

#[test]
fn temporal_cli_reaches_existing_backends() {
    let binary = env!("CARGO_BIN_EXE_fvlab");

    let response_ok = Command::new(binary)
        .args(["temporal", "request-grant"])
        .output()
        .unwrap();
    assert!(response_ok.status.success());
    let response_stdout = String::from_utf8(response_ok.stdout).unwrap();
    assert!(response_stdout.contains("backend: RESPONSE"));
    assert!(response_stdout.contains("temporal: SATISFIED"));

    let response_bad = Command::new(binary)
        .args(["temporal", "request-grant-unfair"])
        .output()
        .unwrap();
    assert_eq!(response_bad.status.code(), Some(10));
    let response_bad_stdout = String::from_utf8(response_bad.stdout).unwrap();
    assert!(response_bad_stdout.contains("backend: RESPONSE"));
    assert!(response_bad_stdout.contains("counterexample: INFINITE"));

    let buchi_ok = Command::new(binary)
        .args(["temporal", "pulses"])
        .output()
        .unwrap();
    assert!(buchi_ok.status.success());
    let buchi_stdout = String::from_utf8(buchi_ok.stdout).unwrap();
    assert!(buchi_stdout.contains("backend: BUCHI"));
    assert!(buchi_stdout.contains("temporal: SATISFIED"));

    let buchi_bad = Command::new(binary)
        .args(["temporal", "pulses-unfair"])
        .output()
        .unwrap();
    assert_eq!(buchi_bad.status.code(), Some(10));
    let buchi_bad_stdout = String::from_utf8(buchi_bad.stdout).unwrap();
    assert!(buchi_bad_stdout.contains("backend: BUCHI"));
    assert!(buchi_bad_stdout.contains("obligation: infinitely-often action 'pulse-b'"));
}

use formal_verification_lab::buchi_examples::{alternating_pulses, unfair_second_pulse};
use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol,
};
use formal_verification_lab::{
    check_action_temporal, parse_action_temporal, ActionAtom, ActionTemporalSpec,
    TemporalParseErrorKind,
};
use std::process::Command;

fn typed_response(name: &str) -> ActionTemporalSpec {
    ActionTemporalSpec::response(
        name,
        ActionAtom::exact("request").unwrap(),
        ActionAtom::exact("grant").unwrap(),
    )
    .unwrap()
}

fn typed_pulses(name: &str) -> ActionTemporalSpec {
    ActionTemporalSpec::all_infinitely_often(
        name,
        vec![
            ActionAtom::exact("pulse-a").unwrap(),
            ActionAtom::exact("pulse-b").unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn parser_produces_the_same_typed_specs_as_direct_construction() {
    assert_eq!(
        parse_action_temporal("response-spec", "response(\"request\", \"grant\")").unwrap(),
        typed_response("response-spec")
    );
    assert_eq!(
        parse_action_temporal(
            "pulse-spec",
            " infinitely-often ( \"pulse-a\" , \"pulse-b\" ) "
        )
        .unwrap(),
        typed_pulses("pulse-spec")
    );
}

#[test]
fn parsed_specs_are_differentially_identical_to_direct_typed_specs() {
    for model in [
        request_grant_protocol().unwrap(),
        unfair_request_grant_protocol().unwrap(),
    ] {
        let parsed = parse_action_temporal(
            "request-eventually-grant",
            "response(\"request\",\"grant\")",
        )
        .unwrap();
        assert_eq!(
            check_action_temporal(&model, &parsed).unwrap(),
            check_action_temporal(&model, &typed_response("request-eventually-grant")).unwrap()
        );
    }

    for model in [
        alternating_pulses().unwrap(),
        unfair_second_pulse().unwrap(),
    ] {
        let parsed = parse_action_temporal(
            "infinitely-often-a-and-b",
            "infinitely-often(\"pulse-a\",\"pulse-b\")",
        )
        .unwrap();
        assert_eq!(
            check_action_temporal(&model, &parsed).unwrap(),
            check_action_temporal(&model, &typed_pulses("infinitely-often-a-and-b")).unwrap()
        );
    }
}

#[test]
fn canonical_expression_round_trips_quoted_action_labels() {
    let original = ActionTemporalSpec::response(
        "escaped",
        ActionAtom::exact("say\"hello").unwrap(),
        ActionAtom::exact("path\\done\nnext").unwrap(),
    )
    .unwrap();
    let rendered = original.canonical_expression();
    assert_eq!(
        rendered,
        "response(\"say\\\"hello\",\"path\\\\done\\nnext\")"
    );
    assert_eq!(
        parse_action_temporal("escaped", &rendered).unwrap(),
        original
    );
}

#[test]
fn parser_reports_deterministic_position_aware_errors() {
    let unknown = parse_action_temporal("x", "eventually(\"tick\")").unwrap_err();
    assert_eq!(unknown.position(), 0);
    assert_eq!(
        unknown.kind(),
        &TemporalParseErrorKind::UnknownOperator {
            operator: "eventually".to_owned()
        }
    );

    let missing_quote = parse_action_temporal("x", "response(request,\"grant\")").unwrap_err();
    assert_eq!(missing_quote.position(), 9);
    assert_eq!(
        missing_quote.kind(),
        &TemporalParseErrorKind::ExpectedString
    );

    let unterminated = parse_action_temporal("x", "response(\"request)").unwrap_err();
    assert_eq!(unterminated.position(), 9);
    assert_eq!(
        unterminated.kind(),
        &TemporalParseErrorKind::UnterminatedString
    );

    let wrong_arity = parse_action_temporal("x", "response(\"request\")").unwrap_err();
    assert_eq!(wrong_arity.position(), 18);
    assert_eq!(
        wrong_arity.kind(),
        &TemporalParseErrorKind::WrongArity {
            operator: "response".to_owned(),
            expected: 2,
            actual: 1
        }
    );

    let trailing =
        parse_action_temporal("x", "response(\"request\",\"grant\") trailing").unwrap_err();
    assert_eq!(trailing.position(), 28);
    assert_eq!(trailing.kind(), &TemporalParseErrorKind::TrailingInput);
}

#[test]
fn parser_reuses_typed_semantic_validation() {
    let empty = parse_action_temporal("x", "infinitely-often()").unwrap_err();
    assert!(matches!(empty.kind(), TemporalParseErrorKind::Semantic(_)));

    let duplicate = parse_action_temporal("x", "infinitely-often(\"tick\",\"tick\")").unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        TemporalParseErrorKind::Semantic(_)
    ));
}

#[test]
fn temporal_cli_accepts_user_supplied_formulas() {
    let binary = env!("CARGO_BIN_EXE_fvlab");

    let response_ok = Command::new(binary)
        .args([
            "temporal",
            "check",
            "request-grant",
            "response(\"request\",\"grant\")",
        ])
        .output()
        .unwrap();
    assert!(response_ok.status.success());
    let stdout = String::from_utf8(response_ok.stdout).unwrap();
    assert!(stdout.contains("temporal: SATISFIED"));
    assert!(stdout.contains("backend: RESPONSE"));

    let recurring_bad = Command::new(binary)
        .args([
            "temporal",
            "check",
            "pulses-unfair",
            "infinitely-often(\"pulse-a\",\"pulse-b\")",
        ])
        .output()
        .unwrap();
    assert_eq!(recurring_bad.status.code(), Some(10));
    let stdout = String::from_utf8(recurring_bad.stdout).unwrap();
    assert!(stdout.contains("obligation: infinitely-often action 'pulse-b'"));

    let parse_bad = Command::new(binary)
        .args(["temporal", "check", "pulses", "eventually(\"pulse-a\")"])
        .output()
        .unwrap();
    assert_eq!(parse_bad.status.code(), Some(2));
    let stderr = String::from_utf8(parse_bad.stderr).unwrap();
    assert!(stderr.contains("byte 0"));
    assert!(stderr.contains("unsupported temporal operator 'eventually'"));
}

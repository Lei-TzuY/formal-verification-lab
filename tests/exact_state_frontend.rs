use formal_verification_lab::{
    check_eventuality, check_exact_state_property, check_reachability, parse_declarative_model,
    parse_exact_state_property, EventualityCounterexample, EventualityProperty, EventualityStatus,
    ExactStateBackend, ExactStateEvidence, ExactStateParseErrorKind, ExactStatePropertySpec,
    ExactStateStatus, ReachabilityProperty, ReachabilityStatus,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CHAIN: &str = r#"
model "chain"
state "start"
state "middle"
state "done"
initial "start"
edge "start" "advance" "middle"
edge "middle" "finish" "done"
"#;

const AVOIDING_CYCLE: &str = r#"
model "avoiding-cycle"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "enter" "loop"
edge "loop" "spin" "loop"
edge "loop" "finish" "done"
"#;

fn reachable_spec(name: &str, target: &str) -> ExactStatePropertySpec {
    ExactStatePropertySpec::reachable(name, target).unwrap()
}

fn all_eventually_spec(name: &str, target: &str) -> ExactStatePropertySpec {
    ExactStatePropertySpec::all_eventually(name, target).unwrap()
}

#[test]
fn parser_matches_direct_typed_exact_state_specs_and_round_trips() {
    assert_eq!(
        parse_exact_state_property("reach", r#"reachable("done")"#).unwrap(),
        reachable_spec("reach", "done")
    );
    assert_eq!(
        parse_exact_state_property("eventual", r#" all-eventually ( "done" ) "#).unwrap(),
        all_eventually_spec("eventual", "done")
    );

    let escaped = ExactStatePropertySpec::reachable("escaped", "say\"done\\next\nline").unwrap();
    let rendered = escaped.canonical_expression();
    assert_eq!(rendered, r#"reachable("say\"done\\next\nline")"#);
    assert_eq!(
        parse_exact_state_property("escaped", &rendered).unwrap(),
        escaped
    );
}

#[test]
fn reachable_frontend_matches_direct_backend_and_preserves_shortest_witness() {
    let model = parse_declarative_model(CHAIN).unwrap();
    let target = "done".to_owned();
    let direct_property =
        ReachabilityProperty::new("reach-done", move |state: &String| state == &target).unwrap();
    let direct = check_reachability(&model, &direct_property).unwrap();
    let frontend =
        check_exact_state_property(&model, &reachable_spec("reach-done", "done")).unwrap();

    assert_eq!(direct.status, ReachabilityStatus::Reachable);
    assert_eq!(frontend.backend, ExactStateBackend::Reachability);
    assert_eq!(frontend.status, ExactStateStatus::Satisfied);
    assert_eq!(frontend.discovered_states, direct.discovered_states);
    assert_eq!(frontend.explored_transitions, direct.explored_transitions);
    assert_eq!(frontend.max_depth_reached, direct.max_depth_reached);
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::ReachabilityWitness {
            trace: direct.witness.unwrap()
        })
    );
}

#[test]
fn unreachable_exact_state_is_a_violation_without_fabricated_evidence() {
    let model = parse_declarative_model(CHAIN).unwrap();
    let target = "missing".to_owned();
    let direct_property =
        ReachabilityProperty::new("reach-missing", move |state: &String| state == &target).unwrap();
    let direct = check_reachability(&model, &direct_property).unwrap();
    let frontend =
        check_exact_state_property(&model, &reachable_spec("reach-missing", "missing")).unwrap();

    assert_eq!(direct.status, ReachabilityStatus::Unreachable);
    assert_eq!(frontend.status, ExactStateStatus::Violated);
    assert_eq!(frontend.discovered_states, direct.discovered_states);
    assert_eq!(frontend.explored_transitions, direct.explored_transitions);
    assert_eq!(frontend.max_depth_reached, direct.max_depth_reached);
    assert!(frontend.evidence.is_none());
}

#[test]
fn all_eventually_frontend_matches_direct_backend_for_success_and_lasso_failure() {
    let chain = parse_declarative_model(CHAIN).unwrap();
    let direct_target = "done".to_owned();
    let direct_property = EventualityProperty::new("eventually-done", move |state: &String| {
        state == &direct_target
    })
    .unwrap();
    let direct = check_eventuality(&chain, &direct_property).unwrap();
    let frontend =
        check_exact_state_property(&chain, &all_eventually_spec("eventually-done", "done"))
            .unwrap();

    assert_eq!(direct.status, EventualityStatus::Satisfied);
    assert_eq!(frontend.backend, ExactStateBackend::Eventuality);
    assert_eq!(frontend.status, ExactStateStatus::Satisfied);
    assert_eq!(frontend.discovered_states, direct.discovered_states);
    assert_eq!(frontend.explored_transitions, direct.explored_transitions);
    assert_eq!(frontend.max_depth_reached, direct.max_depth_reached);
    assert!(frontend.evidence.is_none());

    let cyclic = parse_declarative_model(AVOIDING_CYCLE).unwrap();
    let direct_target = "done".to_owned();
    let direct_property = EventualityProperty::new("eventually-done", move |state: &String| {
        state == &direct_target
    })
    .unwrap();
    let direct = check_eventuality(&cyclic, &direct_property).unwrap();
    let frontend =
        check_exact_state_property(&cyclic, &all_eventually_spec("eventually-done", "done"))
            .unwrap();

    assert_eq!(direct.status, EventualityStatus::Violated);
    let Some(EventualityCounterexample::Infinite { stem, cycle }) = direct.counterexample else {
        panic!("expected direct lasso counterexample");
    };
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle })
    );
    assert_eq!(frontend.status, ExactStateStatus::Violated);
}

#[test]
fn all_eventually_frontend_preserves_finite_terminal_counterexample() {
    let model = parse_declarative_model(
        r#"
model "finite-failure"
state "start"
state "done"
initial "start"
"#,
    )
    .unwrap();
    let target = "done".to_owned();
    let direct_property =
        EventualityProperty::new("eventually-done", move |state: &String| state == &target)
            .unwrap();
    let direct = check_eventuality(&model, &direct_property).unwrap();
    let frontend =
        check_exact_state_property(&model, &all_eventually_spec("eventually-done", "done"))
            .unwrap();

    let Some(EventualityCounterexample::Finite { trace }) = direct.counterexample else {
        panic!("expected direct finite counterexample");
    };
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::EventualityFiniteCounterexample { trace })
    );
}

#[test]
fn exact_state_parser_rejects_malformed_syntax_and_empty_targets_deterministically() {
    let unknown = parse_exact_state_property("x", r#"eventually("done")"#).unwrap_err();
    assert_eq!(unknown.position(), 0);
    assert_eq!(
        unknown.kind(),
        &ExactStateParseErrorKind::UnknownOperator {
            operator: "eventually".to_owned()
        }
    );

    let arity = parse_exact_state_property("x", r#"reachable("a","b")"#).unwrap_err();
    assert!(matches!(
        arity.kind(),
        ExactStateParseErrorKind::WrongArity {
            expected: 1,
            actual: 2,
            ..
        }
    ));

    let empty = parse_exact_state_property("x", r#"reachable("")"#).unwrap_err();
    assert!(matches!(
        empty.kind(),
        ExactStateParseErrorKind::Semantic(_)
    ));

    let trailing = parse_exact_state_property("x", r#"reachable("done") extra"#).unwrap_err();
    assert_eq!(trailing.kind(), &ExactStateParseErrorKind::TrailingInput);
}

fn temp_model_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fvlab-state-{label}-{}-{nonce}.model",
        std::process::id()
    ))
}

#[test]
fn state_file_cli_runs_reachability_and_eventuality_against_external_models() {
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let chain_path = temp_model_path("chain");
    let cycle_path = temp_model_path("cycle");
    fs::write(&chain_path, CHAIN).unwrap();
    fs::write(&cycle_path, AVOIDING_CYCLE).unwrap();

    let reachable = Command::new(binary)
        .args([
            "state",
            "file",
            chain_path.to_str().unwrap(),
            r#"reachable("done")"#,
        ])
        .output()
        .unwrap();
    assert!(reachable.status.success());
    let stdout = String::from_utf8(reachable.stdout).unwrap();
    assert!(stdout.contains("backend: REACHABILITY"));
    assert!(stdout.contains("state property: SATISFIED"));
    assert!(stdout.contains("evidence: REACHABILITY_WITNESS"));
    assert!(stdout.contains("--finish--> \"done\""));

    let missing = Command::new(binary)
        .args([
            "state",
            "file",
            chain_path.to_str().unwrap(),
            r#"reachable("missing")"#,
        ])
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(11));
    let stdout = String::from_utf8(missing.stdout).unwrap();
    assert!(stdout.contains("state property: VIOLATED"));
    assert!(stdout.contains("evidence: none"));

    let eventuality = Command::new(binary)
        .args([
            "state",
            "file",
            cycle_path.to_str().unwrap(),
            r#"all-eventually("done")"#,
        ])
        .output()
        .unwrap();
    assert_eq!(eventuality.status.code(), Some(11));
    let stdout = String::from_utf8(eventuality.stdout).unwrap();
    assert!(stdout.contains("backend: EVENTUALITY"));
    assert!(stdout.contains("evidence: EVENTUALITY_INFINITE_COUNTEREXAMPLE"));
    assert!(stdout.contains("--spin-->"));

    fs::remove_file(chain_path).unwrap();
    fs::remove_file(cycle_path).unwrap();
}

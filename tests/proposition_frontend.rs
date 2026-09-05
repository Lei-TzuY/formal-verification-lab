use formal_verification_lab::{
    check_eventuality, check_proposition_property, check_reachability, parse_declarative_document,
    parse_declarative_model, DeclarativeModelError, EventualityCounterexample, EventualityProperty,
    EventualityStatus, ExactStateBackend, ExactStateEvidence, ExactStateStatus, PropositionError,
    PropositionPropertySpec, ReachabilityProperty, ReachabilityStatus,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const LABELED_GRAPH: &str = r#"
model "labeled-graph"
state "start"
state "critical-a"
state "critical-b"
state "done"
initial "start"
edge "start" "choose-a" "critical-a"
edge "start" "choose-b" "critical-b"
edge "critical-a" "finish-a" "done"
edge "critical-b" "finish-b" "done"
label "critical-a" "critical"
label "critical-b" "critical"
label "done" "complete"
"#;

const AVOIDING_GRAPH: &str = r#"
model "avoiding-proposition"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "enter" "loop"
edge "loop" "spin" "loop"
edge "loop" "finish" "done"
label "done" "complete"
"#;

#[test]
fn declarative_document_preserves_graph_and_proposition_membership_order() {
    let document = parse_declarative_document(LABELED_GRAPH).unwrap();

    assert_eq!(document.model().name(), "labeled-graph");
    assert_eq!(
        document.proposition_states("critical").unwrap(),
        &["critical-a".to_owned(), "critical-b".to_owned()]
    );
    assert_eq!(
        document.proposition_states("complete").unwrap(),
        &["done".to_owned()]
    );
    assert!(document.state_has_proposition("critical-a", "critical"));
    assert!(!document.state_has_proposition("start", "critical"));

    let legacy = parse_declarative_model(LABELED_GRAPH).unwrap();
    assert_eq!(legacy.initial_states(), document.model().initial_states());
    assert_eq!(
        legacy.successors(&"start".to_owned()).unwrap(),
        document.model().successors(&"start".to_owned()).unwrap()
    );
}

#[test]
fn proposition_metadata_rejects_unknown_states_duplicates_and_empty_names() {
    let unknown = parse_declarative_document(
        r#"
model "unknown-label-state"
state "a"
initial "a"
label "missing" "critical"
"#,
    )
    .unwrap_err();
    assert_eq!(
        unknown,
        DeclarativeModelError::UnknownLabelState {
            line: 5,
            state: "missing".to_owned(),
        }
    );

    let duplicate = parse_declarative_document(
        r#"
model "duplicate-label"
state "a"
initial "a"
label "a" "critical"
label "a" "critical"
"#,
    )
    .unwrap_err();
    assert_eq!(
        duplicate,
        DeclarativeModelError::DuplicateLabel {
            line: 6,
            state: "a".to_owned(),
            proposition: "critical".to_owned(),
        }
    );

    let empty = parse_declarative_document(
        r#"
model "empty-proposition"
state "a"
initial "a"
label "a" ""
"#,
    )
    .unwrap_err();
    assert_eq!(empty, DeclarativeModelError::EmptyProposition { line: 5 });
}

#[test]
fn proposition_reachability_matches_direct_multi_state_predicate() {
    let document = parse_declarative_document(LABELED_GRAPH).unwrap();
    let direct = ReachabilityProperty::new("reach-critical", |state: &String| {
        matches!(state.as_str(), "critical-a" | "critical-b")
    })
    .unwrap();
    let direct_result = check_reachability(document.model(), &direct).unwrap();
    let frontend = check_proposition_property(
        &document,
        &PropositionPropertySpec::reachable("reach-critical", "critical").unwrap(),
    )
    .unwrap();

    assert_eq!(direct_result.status, ReachabilityStatus::Reachable);
    assert_eq!(frontend.backend, ExactStateBackend::Reachability);
    assert_eq!(frontend.status, ExactStateStatus::Satisfied);
    assert_eq!(frontend.discovered_states, direct_result.discovered_states);
    assert_eq!(
        frontend.explored_transitions,
        direct_result.explored_transitions
    );
    assert_eq!(frontend.max_depth_reached, direct_result.max_depth_reached);
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::ReachabilityWitness {
            trace: direct_result.witness.unwrap(),
        })
    );
    let Some(ExactStateEvidence::ReachabilityWitness { trace }) = frontend.evidence else {
        panic!("expected reachability witness");
    };
    assert_eq!(trace.last().unwrap().state, "critical-a");
}

#[test]
fn proposition_eventuality_matches_direct_predicate_for_success_and_lasso_failure() {
    let document = parse_declarative_document(LABELED_GRAPH).unwrap();
    let direct =
        EventualityProperty::new("eventually-complete", |state: &String| state == "done").unwrap();
    let direct_result = check_eventuality(document.model(), &direct).unwrap();
    let frontend = check_proposition_property(
        &document,
        &PropositionPropertySpec::all_eventually("eventually-complete", "complete").unwrap(),
    )
    .unwrap();

    assert_eq!(direct_result.status, EventualityStatus::Satisfied);
    assert_eq!(frontend.backend, ExactStateBackend::Eventuality);
    assert_eq!(frontend.status, ExactStateStatus::Satisfied);
    assert_eq!(frontend.discovered_states, direct_result.discovered_states);
    assert_eq!(
        frontend.explored_transitions,
        direct_result.explored_transitions
    );
    assert_eq!(frontend.max_depth_reached, direct_result.max_depth_reached);
    assert!(frontend.evidence.is_none());

    let avoiding = parse_declarative_document(AVOIDING_GRAPH).unwrap();
    let direct =
        EventualityProperty::new("eventually-complete", |state: &String| state == "done").unwrap();
    let direct_result = check_eventuality(avoiding.model(), &direct).unwrap();
    let frontend = check_proposition_property(
        &avoiding,
        &PropositionPropertySpec::all_eventually("eventually-complete", "complete").unwrap(),
    )
    .unwrap();

    assert_eq!(direct_result.status, EventualityStatus::Violated);
    let Some(EventualityCounterexample::Infinite { stem, cycle }) = direct_result.counterexample
    else {
        panic!("expected direct lasso counterexample");
    };
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle })
    );
    assert_eq!(frontend.status, ExactStateStatus::Violated);
}

#[test]
fn unknown_proposition_fails_closed_before_backend_execution() {
    let document = parse_declarative_document(LABELED_GRAPH).unwrap();
    let error = check_proposition_property(
        &document,
        &PropositionPropertySpec::reachable("reach-unknown", "missing").unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        error,
        PropositionError::UnknownProposition {
            proposition: "missing".to_owned(),
        }
    );
}

fn temp_model_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fvlab-proposition-{label}-{}-{nonce}.model",
        std::process::id()
    ))
}

#[test]
fn proposition_file_cli_runs_existing_backends_and_preserves_state_file_compatibility() {
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let labeled_path = temp_model_path("labeled");
    let avoiding_path = temp_model_path("avoiding");
    fs::write(&labeled_path, LABELED_GRAPH).unwrap();
    fs::write(&avoiding_path, AVOIDING_GRAPH).unwrap();

    let reach = Command::new(binary)
        .args([
            "proposition",
            "file",
            labeled_path.to_str().unwrap(),
            "reachable",
            "critical",
        ])
        .output()
        .unwrap();
    assert!(reach.status.success());
    let stdout = String::from_utf8(reach.stdout).unwrap();
    assert!(stdout.contains("proposition: \"critical\""));
    assert!(stdout.contains("backend: REACHABILITY"));
    assert!(stdout.contains("state property: SATISFIED"));
    assert!(stdout.contains("--choose-a--> \"critical-a\""));

    let eventuality = Command::new(binary)
        .args([
            "proposition",
            "file",
            avoiding_path.to_str().unwrap(),
            "all-eventually",
            "complete",
        ])
        .output()
        .unwrap();
    assert_eq!(eventuality.status.code(), Some(11));
    let stdout = String::from_utf8(eventuality.stdout).unwrap();
    assert!(stdout.contains("backend: EVENTUALITY"));
    assert!(stdout.contains("state property: VIOLATED"));
    assert!(stdout.contains("evidence: EVENTUALITY_INFINITE_COUNTEREXAMPLE"));
    assert!(stdout.contains("--spin-->"));

    let unknown = Command::new(binary)
        .args([
            "proposition",
            "file",
            labeled_path.to_str().unwrap(),
            "reachable",
            "missing",
        ])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(stderr.contains("unknown proposition 'missing'"));

    let exact_state = Command::new(binary)
        .args([
            "state",
            "file",
            labeled_path.to_str().unwrap(),
            r#"reachable("done")"#,
        ])
        .output()
        .unwrap();
    assert!(exact_state.status.success());

    fs::remove_file(labeled_path).unwrap();
    fs::remove_file(avoiding_path).unwrap();
}

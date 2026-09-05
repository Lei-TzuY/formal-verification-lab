use formal_verification_lab::{
    check_action_temporal, parse_action_temporal, parse_declarative_model, DeclarativeModelError,
    Invariant, StateVariable, Transition, TransitionSystem,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const FAIR_MODEL: &str = r#"
model "request-grant-file"
state "idle"
state "waiting"
initial "idle"
edge "idle" "request" "waiting"
edge "waiting" "grant" "idle"
"#;

const UNFAIR_MODEL: &str = r#"
model "request-grant-file-unfair"
state "idle"
state "waiting"
initial "idle"
edge "idle" "request" "waiting"
edge "waiting" "wait" "waiting"
edge "waiting" "grant" "idle"
"#;

fn direct_request_grant(name: &str, unfair: bool) -> TransitionSystem<String> {
    TransitionSystem::new(
        name,
        vec![StateVariable::new("state", "declarative state id")],
        vec!["idle".to_owned()],
        move |state: &String| match state.as_str() {
            "idle" => Ok(vec![Transition::new("request", "waiting".to_owned())]),
            "waiting" => {
                let mut next = Vec::new();
                if unfair {
                    next.push(Transition::new("wait", "waiting".to_owned()));
                }
                next.push(Transition::new("grant", "idle".to_owned()));
                Ok(next)
            }
            other => Err(formal_verification_lab::ModelError::TransitionGeneration {
                message: format!("unexpected state {other}"),
            }),
        },
        vec![Invariant::new("declared-state-domain", |state: &String| {
            matches!(state.as_str(), "idle" | "waiting")
        })],
    )
    .unwrap()
}

fn response_spec() -> formal_verification_lab::ActionTemporalSpec {
    parse_action_temporal(
        "request-eventually-grant",
        r#"response("request","grant")"#,
    )
    .unwrap()
}

#[test]
fn declarative_model_preserves_initial_and_edge_order() {
    let input = r#"
model "ordered"
state "a"
state "b"
state "c"
initial "b"
initial "a"
edge "a" "second" "c"
edge "a" "first" "b"
"#;
    let model = parse_declarative_model(input).unwrap();

    assert_eq!(model.name(), "ordered");
    assert_eq!(model.initial_states(), &["b".to_owned(), "a".to_owned()]);
    assert_eq!(
        model.successors(&"a".to_owned()).unwrap(),
        vec![
            Transition::new("second", "c".to_owned()),
            Transition::new("first", "b".to_owned()),
        ]
    );
    assert!(model.successors(&"c".to_owned()).unwrap().is_empty());
}

#[test]
fn parsed_models_are_differentially_identical_to_direct_models() {
    for (text, direct) in [
        (FAIR_MODEL, direct_request_grant("direct-fair", false)),
        (
            UNFAIR_MODEL,
            direct_request_grant("direct-unfair", true),
        ),
    ] {
        let parsed = parse_declarative_model(text).unwrap();
        assert_eq!(
            check_action_temporal(&parsed, &response_spec()).unwrap(),
            check_action_temporal(&direct, &response_spec()).unwrap()
        );
    }
}

#[test]
fn declarative_validation_rejects_duplicates_and_unknown_references() {
    let duplicate_state = parse_declarative_model(
        r#"
model "duplicate"
state "a"
state "a"
initial "a"
"#,
    )
    .unwrap_err();
    assert_eq!(
        duplicate_state,
        DeclarativeModelError::DuplicateState {
            line: 4,
            state: "a".to_owned(),
        }
    );

    let duplicate_initial = parse_declarative_model(
        r#"
model "duplicate-initial"
state "a"
initial "a"
initial "a"
"#,
    )
    .unwrap_err();
    assert_eq!(
        duplicate_initial,
        DeclarativeModelError::DuplicateInitial {
            line: 5,
            state: "a".to_owned(),
        }
    );

    let unknown_target = parse_declarative_model(
        r#"
model "bad-edge"
state "a"
initial "a"
edge "a" "go" "missing"
"#,
    )
    .unwrap_err();
    assert_eq!(
        unknown_target,
        DeclarativeModelError::UnknownEdgeTarget {
            line: 5,
            state: "missing".to_owned(),
        }
    );

    let duplicate_edge = parse_declarative_model(
        r#"
model "duplicate-edge"
state "a"
state "b"
initial "a"
edge "a" "go" "b"
edge "a" "go" "b"
"#,
    )
    .unwrap_err();
    assert_eq!(
        duplicate_edge,
        DeclarativeModelError::DuplicateEdge {
            line: 7,
            from: "a".to_owned(),
            action: "go".to_owned(),
            to: "b".to_owned(),
        }
    );
}

#[test]
fn declarative_syntax_errors_are_position_aware_and_semantic_names_are_checked() {
    let unterminated = parse_declarative_model("model \"oops\n").unwrap_err();
    assert_eq!(
        unterminated,
        DeclarativeModelError::UnterminatedString { line: 1, column: 7 }
    );

    let invalid_escape = parse_declarative_model(
        "model \"m\"\nstate \"a\\q\"\ninitial \"a\"\n",
    )
    .unwrap_err();
    assert_eq!(
        invalid_escape,
        DeclarativeModelError::InvalidEscape {
            line: 2,
            column: 9,
            escape: "q".to_owned(),
        }
    );

    let empty_action = parse_declarative_model(
        r#"
model "empty-action"
state "a"
initial "a"
edge "a" "" "a"
"#,
    )
    .unwrap_err();
    assert_eq!(empty_action, DeclarativeModelError::EmptyAction { line: 5 });
}

fn temp_model_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("fvlab-{label}-{}-{nonce}.model", std::process::id()))
}

#[test]
fn temporal_cli_loads_external_models_and_user_expressions() {
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let fair_path = temp_model_path("fair");
    let unfair_path = temp_model_path("unfair");
    fs::write(&fair_path, FAIR_MODEL).unwrap();
    fs::write(&unfair_path, UNFAIR_MODEL).unwrap();

    let expression = r#"response("request","grant")"#;
    let fair = Command::new(binary)
        .args([
            "temporal",
            "file",
            fair_path.to_str().unwrap(),
            expression,
        ])
        .output()
        .unwrap();
    assert!(fair.status.success());
    let fair_stdout = String::from_utf8(fair.stdout).unwrap();
    assert!(fair_stdout.contains("model: request-grant-file"));
    assert!(fair_stdout.contains("temporal: SATISFIED"));

    let unfair = Command::new(binary)
        .args([
            "temporal",
            "file",
            unfair_path.to_str().unwrap(),
            expression,
        ])
        .output()
        .unwrap();
    assert_eq!(unfair.status.code(), Some(10));
    let unfair_stdout = String::from_utf8(unfair.stdout).unwrap();
    assert!(unfair_stdout.contains("temporal: VIOLATED"));
    assert!(unfair_stdout.contains("--wait-->"));

    fs::remove_file(fair_path).unwrap();
    fs::remove_file(unfair_path).unwrap();
}

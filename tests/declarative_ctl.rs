use formal_verification_lab::{
    check_declarative_ctl, check_declarative_ctl_text, evaluate_ctl, parse_ctl_formula,
    parse_declarative_document, render_ctl_formula, CtlEvidence, CtlEvidenceAction, CtlFormula,
    CtlParseErrorKind, DeclarativeCtlError, DeclarativeCtlStatus,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const BRANCHING_MODEL: &str = r#"
model "ctl-branching"
state "start"
state "left"
state "right"
state "done"
initial "start"
edge "start" "choose-left" "left"
edge "start" "choose-right" "right"
edge "left" "finish" "done"
edge "right" "spin" "right"
label "start" "p"
label "left" "p"
label "right" "p"
label "done" "q"
"#;

#[test]
fn parser_respects_precedence_until_grouping_and_canonical_round_trip() {
    let parsed = parse_ctl_formula(r#"AG "p" or EF "q" and EX not "bad""#).unwrap();
    let expected = CtlFormula::or(
        CtlFormula::ag(CtlFormula::atom("p".to_owned())),
        CtlFormula::and(
            CtlFormula::ef(CtlFormula::atom("q".to_owned())),
            CtlFormula::ex(CtlFormula::negate(CtlFormula::atom("bad".to_owned()))),
        ),
    );
    assert_eq!(parsed, expected);

    let until = parse_ctl_formula(r#"E[("p" or "q") U AF "q"]"#).unwrap();
    let canonical = render_ctl_formula(&until);
    assert_eq!(parse_ctl_formula(&canonical).unwrap(), until);

    let universal_until = parse_ctl_formula(r#"A["p" U "q"]"#).unwrap();
    assert!(matches!(universal_until, CtlFormula::Au(_, _)));
}

#[test]
fn parser_reports_position_aware_errors_and_rejects_empty_atoms() {
    let missing = parse_ctl_formula("EF (").unwrap_err();
    assert!(matches!(
        missing.kind(),
        CtlParseErrorKind::ExpectedExpression
    ));

    let missing_until = parse_ctl_formula(r#"E["p" "q"]"#).unwrap_err();
    assert!(matches!(
        missing_until.kind(),
        CtlParseErrorKind::ExpectedUntil
    ));

    let invalid_escape = parse_ctl_formula(r#""p\z""#).unwrap_err();
    assert!(matches!(
        invalid_escape.kind(),
        CtlParseErrorKind::InvalidEscape { escape } if escape == "z"
    ));

    let empty = parse_ctl_formula(r#""""#).unwrap_err();
    assert!(matches!(empty.kind(), CtlParseErrorKind::EmptyProposition));
}

#[test]
fn declarative_binding_rejects_unknown_atoms_before_backend_execution() {
    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
    let formula = parse_ctl_formula(r#"EF "missing""#).unwrap();

    assert_eq!(
        check_declarative_ctl(&document, &formula).unwrap_err(),
        DeclarativeCtlError::UnknownProposition {
            proposition: "missing".to_owned(),
        }
    );
}

#[test]
fn declarative_frontend_delegates_exactly_to_typed_ctl_authority() {
    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
    let formula = parse_ctl_formula(r#"AF "q" or EG "p""#).unwrap();

    let direct = evaluate_ctl(document.model(), &formula, |atom, state| {
        document.state_has_proposition(state, atom)
    })
    .unwrap();
    let frontend = check_declarative_ctl(&document, &formula).unwrap();

    assert_eq!(frontend.evaluation, direct);
    assert_eq!(frontend.formula, render_ctl_formula(&formula));
    assert_eq!(frontend.status, DeclarativeCtlStatus::Satisfied);
}

#[test]
fn branching_semantics_distinguish_existential_and_universal_eventuality() {
    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();

    let ef = check_declarative_ctl_text(&document, r#"EF "q""#).unwrap();
    let af = check_declarative_ctl_text(&document, r#"AF "q""#).unwrap();

    assert_eq!(ef.status, DeclarativeCtlStatus::Satisfied);
    assert_eq!(af.status, DeclarativeCtlStatus::Violated);
    assert!(matches!(
        ef.evaluation.initial[0].evidence,
        Some(CtlEvidence::Finite { .. })
    ));
    assert!(matches!(
        af.evaluation.initial[0].evidence,
        Some(CtlEvidence::Lasso { .. })
    ));
}

#[test]
fn terminal_self_loop_provenance_survives_declarative_frontend() {
    let document = parse_declarative_document(
        r#"
model "terminal"
state "done"
initial "done"
label "done" "complete"
"#,
    )
    .unwrap();

    let result = check_declarative_ctl_text(&document, r#"EX "complete""#).unwrap();
    assert_eq!(result.status, DeclarativeCtlStatus::Satisfied);

    match result.evaluation.initial[0].evidence.as_ref().unwrap() {
        CtlEvidence::Finite { trace } => {
            assert_eq!(trace.len(), 2);
            assert_eq!(
                trace[1].action,
                Some(CtlEvidenceAction::TerminalSelfLoop)
            );
            assert_eq!(trace[1].state, "done");
        }
        other => panic!("expected finite terminal-loop evidence, got {other:?}"),
    }
}

#[test]
fn nested_until_and_global_formulas_preserve_m69_semantics() {
    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();

    let exists_until = check_declarative_ctl_text(&document, r#"E["p" U "q"]"#).unwrap();
    let all_until = check_declarative_ctl_text(&document, r#"A["p" U "q"]"#).unwrap();
    let nested = check_declarative_ctl_text(&document, r#"AG ("p" or "q")"#).unwrap();

    assert_eq!(exists_until.status, DeclarativeCtlStatus::Satisfied);
    assert_eq!(all_until.status, DeclarativeCtlStatus::Violated);
    assert_eq!(nested.status, DeclarativeCtlStatus::Satisfied);
}

#[test]
fn built_binary_ctl_file_frontend_covers_success_violation_and_fail_closed_errors() {
    let root = fixture_dir("cli");
    let path = root.join("model.fvl");
    fs::write(
        &path,
        r#"
model "ctl-cli"
state "start"
state "done"
initial "start"
edge "start" "finish" "done"
label "start" "ready"
label "done" "complete"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_fvlab");

    let satisfied = Command::new(binary)
        .args(["ctl", "file", path.to_str().unwrap(), r#"EF "complete""#])
        .output()
        .unwrap();
    assert!(satisfied.status.success());
    let stdout = String::from_utf8(satisfied.stdout).unwrap();
    assert!(stdout.contains("CTL: SATISFIED"));
    assert!(stdout.contains("formula: EF (\"complete\")"));

    let violated = Command::new(binary)
        .args(["ctl", "file", path.to_str().unwrap(), r#"AG "ready""#])
        .output()
        .unwrap();
    assert_eq!(violated.status.code(), Some(14));
    let stdout = String::from_utf8(violated.stdout).unwrap();
    assert!(stdout.contains("CTL: VIOLATED"));

    let unknown = Command::new(binary)
        .args(["ctl", "file", path.to_str().unwrap(), r#"EF "missing""#])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(stderr.contains("unknown CTL proposition 'missing'"));

    let malformed = Command::new(binary)
        .args(["ctl", "file", path.to_str().unwrap(), "EF ("])
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8(malformed.stderr).unwrap();
    assert!(stderr.contains("CTL parse error"));

    fs::remove_dir_all(root).unwrap();
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m70-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

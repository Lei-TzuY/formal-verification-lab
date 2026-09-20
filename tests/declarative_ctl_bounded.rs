use formal_verification_lab::{
    check_declarative_ctl_text_with_limits, check_declarative_ctl_with_limits,
    evaluate_ctl_with_limits, parse_ctl_formula, parse_declarative_document,
    render_bounded_declarative_ctl_report, BoundedCtlStatus, BoundedCtlTruth, BoundedOutcome,
    CtlEvidence, CtlEvidenceAction, DeclarativeCtlError, ExplorationLimits, InconclusiveReason,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "bounded-ctl-file"
state "start"
state "done"
state "other"
initial "start"
edge "start" "finish" "done"
edge "start" "branch" "other"
label "start" "ready"
label "done" "complete"
label "other" "ready"
"#;

#[test]
fn bounded_declarative_adapter_matches_m71_typed_authority() {
    let document = parse_declarative_document(MODEL).unwrap();
    let formula = parse_ctl_formula(r#"EF "complete""#).unwrap();
    let limits = ExplorationLimits {
        max_states: Some(2),
        ..ExplorationLimits::unbounded()
    };

    let direct = evaluate_ctl_with_limits(
        document.model(),
        &formula,
        |atom, state| document.state_has_proposition(state, atom),
        limits,
    )
    .unwrap();
    let frontend = check_declarative_ctl_with_limits(&document, &formula, limits).unwrap();

    assert_eq!(frontend.evaluation, direct);
}

#[test]
fn exact_bound_completion_remains_conclusive() {
    let document = parse_declarative_document(
        r#"
model "exact-bound"
state "start"
state "done"
initial "start"
edge "start" "finish" "done"
label "done" "complete"
"#,
    )
    .unwrap();
    let limits = ExplorationLimits {
        max_states: Some(2),
        max_transitions: Some(1),
        ..ExplorationLimits::unbounded()
    };

    let result =
        check_declarative_ctl_text_with_limits(&document, r#"EF "complete""#, limits).unwrap();
    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    );
    assert_eq!(result.evaluation.initial[0].truth, BoundedCtlTruth::True);
}

#[test]
fn each_cutoff_class_is_reported_without_false_terminalization() {
    let document = parse_declarative_document(MODEL).unwrap();
    let cases = [
        (
            ExplorationLimits {
                max_states: Some(1),
                ..ExplorationLimits::unbounded()
            },
            InconclusiveReason::StateLimitReached { limit: 1 },
        ),
        (
            ExplorationLimits {
                max_transitions: Some(0),
                ..ExplorationLimits::unbounded()
            },
            InconclusiveReason::TransitionLimitReached { limit: 0 },
        ),
        (
            ExplorationLimits {
                max_depth: Some(0),
                ..ExplorationLimits::unbounded()
            },
            InconclusiveReason::DepthLimitReached { limit: 0 },
        ),
    ];

    for (limits, reason) in cases {
        let result =
            check_declarative_ctl_text_with_limits(&document, r#"EF "complete""#, limits).unwrap();
        assert_eq!(result.evaluation.outcome, BoundedOutcome::Inconclusive(reason));
        assert_eq!(result.evaluation.initial[0].truth, BoundedCtlTruth::Unknown);
        assert!(result.evaluation.initial[0].evidence.is_none());

        let report = render_bounded_declarative_ctl_report(document.model().name(), &result);
        assert!(report.contains("CTL: INCONCLUSIVE"));
        assert!(report.contains("initial state 0: UNKNOWN"));
    }
}

#[test]
fn retained_witness_can_be_conclusive_before_state_cutoff() {
    let document = parse_declarative_document(MODEL).unwrap();
    let result = check_declarative_ctl_text_with_limits(
        &document,
        r#"EF "complete""#,
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    );
    assert!(matches!(
        result.evaluation.initial[0].evidence,
        Some(CtlEvidence::Finite { .. })
    ));
}

#[test]
fn unknown_propositions_fail_before_bounded_backend_execution() {
    let document = parse_declarative_document(MODEL).unwrap();
    let result = check_declarative_ctl_text_with_limits(
        &document,
        r#"EF "missing""#,
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::unbounded()
        },
    );

    assert_eq!(
        result.unwrap_err(),
        DeclarativeCtlError::UnknownProposition {
            proposition: "missing".to_owned(),
        }
    );
}

#[test]
fn proven_terminal_self_loop_survives_bounded_reporting() {
    let document = parse_declarative_document(
        r#"
model "terminal"
state "done"
initial "done"
label "done" "complete"
"#,
    )
    .unwrap();
    let result = check_declarative_ctl_text_with_limits(
        &document,
        r#"EX "complete""#,
        ExplorationLimits {
            max_states: Some(1),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    );
    match result.evaluation.initial[0].evidence.as_ref().unwrap() {
        CtlEvidence::Finite { trace } => {
            assert_eq!(
                trace[1].action,
                Some(CtlEvidenceAction::TerminalSelfLoop)
            );
        }
        other => panic!("expected finite terminal-loop evidence, got {other:?}"),
    }

    let report = render_bounded_declarative_ctl_report(document.model().name(), &result);
    assert!(report.contains("--<terminal-self-loop>-->"));
}

#[test]
fn built_binary_bounded_ctl_covers_conclusive_inconclusive_and_errors() {
    let root = fixture_dir("cli");
    let path = root.join("model.fvl");
    fs::write(&path, MODEL).unwrap();
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = path.to_str().unwrap();

    let satisfied = Command::new(binary)
        .args([
            "ctl",
            "file",
            path,
            r#"EF "complete""#,
            "--max-states",
            "2",
        ])
        .output()
        .unwrap();
    assert_eq!(satisfied.status.code(), Some(0));
    let stdout = String::from_utf8(satisfied.stdout).unwrap();
    assert!(stdout.contains("CTL: SATISFIED"));
    assert!(stdout.contains("definitely satisfying states:"));

    let violated = Command::new(binary)
        .args([
            "ctl",
            "file",
            path,
            r#"AG "ready""#,
            "--max-states",
            "2",
        ])
        .output()
        .unwrap();
    assert_eq!(violated.status.code(), Some(14));

    let inconclusive = Command::new(binary)
        .args([
            "ctl",
            "file",
            path,
            r#"EF "complete""#,
            "--max-states",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(inconclusive.status.code(), Some(3));
    let stdout = String::from_utf8(inconclusive.stdout).unwrap();
    assert!(stdout.contains("CTL: INCONCLUSIVE"));
    assert!(stdout.contains("state limit reached (max 1)"));

    let malformed = Command::new(binary)
        .args([
            "ctl",
            "file",
            path,
            r#"EF "complete""#,
            "--max-states",
            "nope",
        ])
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));

    let unknown = Command::new(binary)
        .args([
            "ctl",
            "file",
            path,
            r#"EF "missing""#,
            "--max-states",
            "0",
        ])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(stderr.contains("unknown CTL proposition 'missing'"));

    fs::remove_dir_all(root).unwrap();
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m72-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

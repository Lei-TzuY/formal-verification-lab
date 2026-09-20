use formal_verification_lab::{
    check_declarative_mu_text, check_declarative_mu_text_with_limits,
    check_declarative_mu_with_limits, evaluate_mu_with_limits, parse_declarative_document,
    parse_mu_formula, render_bounded_declarative_mu_report, BoundedMuStatus, BoundedMuTruth,
    BoundedOutcome, DeclarativeMuError, DeclarativeMuStatus, ExplorationLimits, InconclusiveReason,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "bounded-mu-file"
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
fn bounded_declarative_adapter_matches_m77_typed_authority() {
    let document = parse_declarative_document(MODEL).unwrap();
    let formula = parse_mu_formula(r#"mu X. "complete" or diamond $X"#).unwrap();
    let limits = ExplorationLimits {
        max_states: Some(2),
        ..ExplorationLimits::unbounded()
    };

    let direct = evaluate_mu_with_limits(
        document.model(),
        &formula,
        |atom, state| document.state_has_proposition(state, atom),
        limits,
    )
    .unwrap();
    let frontend = check_declarative_mu_with_limits(&document, &formula, limits).unwrap();

    assert_eq!(frontend.evaluation, direct);
}

#[test]
fn unbounded_bounded_frontend_collapses_to_complete_m76_behavior() {
    let document = parse_declarative_document(MODEL).unwrap();
    let expression = r#"mu X. "complete" or diamond $X"#;

    let complete = check_declarative_mu_text(&document, expression).unwrap();
    let bounded = check_declarative_mu_text_with_limits(
        &document,
        expression,
        ExplorationLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(complete.status, DeclarativeMuStatus::Satisfied);
    assert_eq!(
        bounded.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    );
    assert!(bounded.evaluation.initial_states_complete);
    assert_eq!(
        bounded.evaluation.definitely_satisfying_state_indices,
        complete.evaluation.satisfying_state_indices
    );
    assert_eq!(
        bounded.evaluation.possibly_satisfying_state_indices,
        complete.evaluation.satisfying_state_indices
    );
    assert_eq!(
        bounded.evaluation.discovered_states,
        complete.evaluation.discovered_states
    );
    assert_eq!(
        bounded.evaluation.explored_transitions,
        complete.evaluation.explored_transitions
    );
    assert_eq!(
        bounded.evaluation.fixpoint_iterations,
        complete.evaluation.fixpoint_iterations
    );
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

    let result = check_declarative_mu_text_with_limits(
        &document,
        r#"mu X. "complete" or diamond $X"#,
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(1),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    );
    assert_eq!(result.evaluation.initial[0].truth, BoundedMuTruth::True);
}

#[test]
fn each_cutoff_class_is_reported_as_unknown_when_successors_are_unresolved() {
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
        let result = check_declarative_mu_text_with_limits(
            &document,
            r#"mu X. "complete" or diamond $X"#,
            limits,
        )
        .unwrap();

        assert_eq!(
            result.evaluation.outcome,
            BoundedOutcome::Inconclusive(reason)
        );
        assert_eq!(result.evaluation.initial[0].truth, BoundedMuTruth::Unknown);

        let report = render_bounded_declarative_mu_report(document.model().name(), &result);
        assert!(report.contains("MU: INCONCLUSIVE"));
        assert!(report.contains("initial state 0: UNKNOWN"));
        assert!(report.contains("initial states complete: true"));
    }
}

#[test]
fn retained_existential_result_can_be_conclusive_before_state_cutoff() {
    let document = parse_declarative_document(MODEL).unwrap();
    let result = check_declarative_mu_text_with_limits(
        &document,
        r#"mu X. "complete" or diamond $X"#,
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    );
    assert_eq!(result.evaluation.initial[0].truth, BoundedMuTruth::True);
}

#[test]
fn unknown_propositions_fail_before_bounded_graph_capture_even_at_zero_budget() {
    let document = parse_declarative_document(MODEL).unwrap();
    let result = check_declarative_mu_text_with_limits(
        &document,
        r#"mu X. "missing" or diamond $X"#,
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::unbounded()
        },
    );

    assert!(matches!(
        result.unwrap_err(),
        DeclarativeMuError::UnknownProposition { proposition }
            if proposition == "missing"
    ));
}

#[test]
fn proven_terminal_self_loop_remains_conclusive_in_bounded_frontend() {
    let document = parse_declarative_document(
        r#"
model "terminal"
state "done"
initial "done"
label "done" "complete"
"#,
    )
    .unwrap();
    let result = check_declarative_mu_text_with_limits(
        &document,
        r#"diamond "complete""#,
        ExplorationLimits {
            max_states: Some(1),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    );
    assert_eq!(result.evaluation.initial[0].truth, BoundedMuTruth::True);

    let report = render_bounded_declarative_mu_report(document.model().name(), &result);
    assert!(report.contains("terminal policy: totalize proven terminals"));
}

#[test]
fn zero_state_budget_is_inconclusive_not_vacuously_satisfied() {
    let document = parse_declarative_document(MODEL).unwrap();
    let result = check_declarative_mu_text_with_limits(
        &document,
        "true",
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert!(!result.evaluation.initial_states_complete);
    assert!(result.evaluation.initial.is_empty());
    assert_eq!(
        result.evaluation.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 0 })
    );
}

#[test]
fn built_binary_mu_file_preserves_complete_path_and_bounded_exit_contract() {
    let root = fixture_dir("cli");
    let path = root.join("model.fvl");
    fs::write(&path, MODEL).unwrap();
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = path.to_str().unwrap();
    let expression = r#"mu X. "complete" or diamond $X"#;

    let complete = Command::new(binary)
        .args(["mu", "file", path, expression])
        .output()
        .unwrap();
    assert_eq!(complete.status.code(), Some(0));
    let complete_stdout = String::from_utf8(complete.stdout).unwrap();
    assert!(complete_stdout.contains("MU: SATISFIED"));
    assert!(complete_stdout.contains("satisfying states:"));
    assert!(!complete_stdout.contains("possibly satisfying states:"));

    let retained = Command::new(binary)
        .args(["mu", "file", path, expression, "--max-states", "2"])
        .output()
        .unwrap();
    assert_eq!(retained.status.code(), Some(0));
    let retained_stdout = String::from_utf8(retained.stdout).unwrap();
    assert!(retained_stdout.contains("MU: SATISFIED"));
    assert!(retained_stdout.contains("definitely satisfying states:"));
    assert!(retained_stdout.contains("possibly satisfying states:"));

    let violated = Command::new(binary)
        .args([
            "mu",
            "file",
            path,
            r#"nu X. "complete" and box $X"#,
            "--max-states",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(violated.status.code(), Some(15));

    let inconclusive = Command::new(binary)
        .args(["mu", "file", path, expression, "--max-states", "1"])
        .output()
        .unwrap();
    assert_eq!(inconclusive.status.code(), Some(3));
    let inconclusive_stdout = String::from_utf8(inconclusive.stdout).unwrap();
    assert!(inconclusive_stdout.contains("MU: INCONCLUSIVE"));
    assert!(inconclusive_stdout.contains("state limit reached (max 1)"));
    assert!(inconclusive_stdout.contains("initial state 0: UNKNOWN"));

    let zero = Command::new(binary)
        .args(["mu", "file", path, "true", "--max-states", "0"])
        .output()
        .unwrap();
    assert_eq!(zero.status.code(), Some(3));
    let zero_stdout = String::from_utf8(zero.stdout).unwrap();
    assert!(zero_stdout.contains("initial states complete: false"));

    let unknown = Command::new(binary)
        .args(["mu", "file", path, r#""missing""#, "--max-states", "0"])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    let unknown_stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(unknown_stderr.contains("unknown mu-calculus proposition 'missing'"));

    let malformed_limit = Command::new(binary)
        .args(["mu", "file", path, expression, "--max-depth", "nope"])
        .output()
        .unwrap();
    assert_eq!(malformed_limit.status.code(), Some(2));

    fs::remove_dir_all(root).unwrap();
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m78-bounded-mu-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

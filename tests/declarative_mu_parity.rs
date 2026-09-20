use formal_verification_lab::{
    check_declarative_mu_text, check_declarative_mu_text_via_parity, parse_declarative_document,
    DeclarativeMuError, DeclarativeMuStatus, MuTerminalPolicy, MuValidationError,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "mu-parity-file"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "cycle" "loop"
edge "start" "finish" "done"
edge "loop" "back" "start"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#;

#[test]
fn declarative_parity_backend_matches_sealed_fixpoint_frontend() {
    let document = parse_declarative_document(MODEL).unwrap();
    let formulas = [
        "true",
        r#""complete""#,
        r#"not "complete""#,
        r#"diamond "complete""#,
        r#"box "ready""#,
        r#"mu X. "complete" or diamond $X"#,
        r#"nu X. "ready" and diamond $X"#,
        r#"not (mu X. "complete" or diamond $X)"#,
        r#"mu X. "complete" or diamond (nu Y. ("ready" and diamond $Y) or diamond $X)"#,
        r#"nu X. "ready" and box (mu Y. "complete" or diamond $Y or box $X)"#,
    ];

    for formula in formulas {
        let fixpoint = check_declarative_mu_text(&document, formula).unwrap();
        let parity = check_declarative_mu_text_via_parity(&document, formula).unwrap();

        assert_eq!(parity.formula, fixpoint.formula, "formula={formula}");
        assert_eq!(parity.status, fixpoint.status, "formula={formula}");
        assert_eq!(
            parity.evaluation.terminal_policy,
            MuTerminalPolicy::TotalizeWithSelfLoop
        );
        assert_eq!(
            parity.evaluation.reachable_states,
            fixpoint.evaluation.reachable_states,
            "formula={formula}"
        );
        assert_eq!(
            parity.evaluation.satisfying_state_indices,
            fixpoint.evaluation.satisfying_state_indices,
            "formula={formula}"
        );
        assert_eq!(
            parity.evaluation.initial,
            fixpoint.evaluation.initial,
            "formula={formula}"
        );
        assert_eq!(
            parity.evaluation.discovered_states,
            fixpoint.evaluation.discovered_states
        );
        assert_eq!(
            parity.evaluation.explored_transitions,
            fixpoint.evaluation.explored_transitions
        );
        assert_eq!(
            parity.evaluation.max_depth_reached,
            fixpoint.evaluation.max_depth_reached
        );
        assert!(parity.evaluation.parity_game_vertices >= parity.evaluation.reachable_states.len());
    }
}

#[test]
fn declarative_parity_preserves_terminal_totalization() {
    let document = parse_declarative_document(
        r#"
model "terminal-parity"
state "done"
initial "done"
label "done" "complete"
"#,
    )
    .unwrap();

    for formula in [
        r#"diamond "complete""#,
        r#"nu X. "complete" and diamond $X"#,
    ] {
        let parity = check_declarative_mu_text_via_parity(&document, formula).unwrap();
        assert_eq!(parity.status, DeclarativeMuStatus::Satisfied);
        assert_eq!(parity.evaluation.satisfying_state_indices, vec![0]);
    }

    let violated = check_declarative_mu_text_via_parity(&document, "mu X. diamond $X").unwrap();
    assert_eq!(violated.status, DeclarativeMuStatus::Violated);
}

#[test]
fn declarative_parity_keeps_validation_and_atom_resolution_fail_closed() {
    let document = parse_declarative_document(MODEL).unwrap();

    let unbound = check_declarative_mu_text_via_parity(&document, "diamond $X").unwrap_err();
    assert!(matches!(
        unbound,
        DeclarativeMuError::Validation(MuValidationError::UnboundVariable { variable })
            if variable == "X"
    ));

    let non_monotone =
        check_declarative_mu_text_via_parity(&document, "mu X. not $X").unwrap_err();
    assert!(matches!(
        non_monotone,
        DeclarativeMuError::Validation(MuValidationError::NonMonotoneVariable { variable })
            if variable == "X"
    ));

    let unknown =
        check_declarative_mu_text_via_parity(&document, r#"mu X. "missing" or diamond $X"#)
            .unwrap_err();
    assert!(matches!(
        unknown,
        DeclarativeMuError::UnknownProposition { proposition }
            if proposition == "missing"
    ));
}

#[test]
fn built_cli_parity_is_opt_in_and_default_fixpoint_output_is_unchanged() {
    let root = fixture_dir("cli");
    let path = root.join("model.fvl");
    fs::write(&path, MODEL).unwrap();
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = path.to_str().unwrap();
    let formula = r#"mu X. "complete" or diamond $X"#;

    let default = Command::new(binary)
        .args(["mu", "file", path, formula])
        .output()
        .unwrap();
    assert_eq!(default.status.code(), Some(0));

    let explicit_fixpoint = Command::new(binary)
        .args(["mu", "file", path, formula, "--backend", "fixpoint"])
        .output()
        .unwrap();
    assert_eq!(explicit_fixpoint.status.code(), Some(0));
    assert_eq!(explicit_fixpoint.stdout, default.stdout);

    let parity = Command::new(binary)
        .args(["mu", "file", path, formula, "--backend", "parity"])
        .output()
        .unwrap();
    assert_eq!(parity.status.code(), Some(0));
    let stdout = String::from_utf8(parity.stdout).unwrap();
    assert!(stdout.contains("backend: parity-game"));
    assert!(stdout.contains("MU: SATISFIED"));
    assert!(stdout.contains("parity game vertices:"));
    assert!(stdout.contains("max parity priority:"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_cli_parity_rejects_bounds_and_invalid_backend_configuration() {
    let root = fixture_dir("fail-closed");
    let path = root.join("model.fvl");
    fs::write(&path, MODEL).unwrap();
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = path.to_str().unwrap();
    let formula = r#"mu X. "complete" or diamond $X"#;

    let bounded = Command::new(binary)
        .args([
            "mu",
            "file",
            path,
            formula,
            "--backend",
            "parity",
            "--max-states",
            "1",
        ])
        .output()
        .unwrap();
    assert_eq!(bounded.status.code(), Some(2));
    let stderr = String::from_utf8(bounded.stderr).unwrap();
    assert!(stderr.contains("parity backend does not support model-space limits"));
    assert!(stderr.contains("--backend fixpoint"));

    let unknown = Command::new(binary)
        .args(["mu", "file", path, formula, "--backend", "unknown"])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(stderr.contains("unknown mu-calculus backend 'unknown'"));

    let duplicate = Command::new(binary)
        .args([
            "mu",
            "file",
            path,
            formula,
            "--backend",
            "parity",
            "--backend",
            "fixpoint",
        ])
        .output()
        .unwrap();
    assert_eq!(duplicate.status.code(), Some(2));
    let stderr = String::from_utf8(duplicate.stderr).unwrap();
    assert!(stderr.contains("duplicate option '--backend'"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_cli_parity_preserves_formula_and_atom_errors() {
    let root = fixture_dir("errors");
    let path = root.join("model.fvl");
    fs::write(&path, MODEL).unwrap();
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = path.to_str().unwrap();

    for (formula, needle) in [
        ("diamond $X", "unbound mu-calculus variable"),
        ("mu X. not $X", "occurs negatively"),
        (r#""missing""#, "unknown mu-calculus proposition 'missing'"),
        ("mu X true", "mu-calculus parse error"),
    ] {
        let output = Command::new(binary)
            .args(["mu", "file", path, formula, "--backend", "parity"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "formula={formula}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(needle), "formula={formula}: {stderr}");
    }

    fs::remove_dir_all(root).unwrap();
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m82-mu-parity-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

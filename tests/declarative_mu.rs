use formal_verification_lab::{
    check_declarative_ctl_text, check_declarative_mu, check_declarative_mu_text, evaluate_mu,
    parse_declarative_document, parse_mu_formula, render_mu_formula, DeclarativeMuError,
    DeclarativeMuStatus, MuFormula, MuParseErrorKind, MuTerminalPolicy, MuValidationError,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "mu-file"
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
fn parser_precedence_binders_shadowing_and_canonical_round_trip() {
    let parsed = parse_mu_formula(
        r#"nu X. "ready" and diamond (mu X. "complete" or diamond $X) and diamond $X"#,
    )
    .unwrap();

    let expected = MuFormula::nu(
        "X".to_owned(),
        MuFormula::and(
            MuFormula::and(
                MuFormula::atom("ready".to_owned()),
                MuFormula::diamond(MuFormula::mu(
                    "X".to_owned(),
                    MuFormula::or(
                        MuFormula::atom("complete".to_owned()),
                        MuFormula::diamond(MuFormula::var("X".to_owned())),
                    ),
                )),
            ),
            MuFormula::diamond(MuFormula::var("X".to_owned())),
        ),
    );
    assert_eq!(parsed, expected);

    let canonical = render_mu_formula(&parsed);
    assert_eq!(parse_mu_formula(&canonical).unwrap(), parsed);
}

#[test]
fn parser_reports_structural_errors_and_empty_atoms() {
    let missing_variable = parse_mu_formula("mu . true").unwrap_err();
    assert!(matches!(
        missing_variable.kind(),
        MuParseErrorKind::ExpectedVariable
    ));

    let missing_dot = parse_mu_formula("nu X true").unwrap_err();
    assert!(matches!(missing_dot.kind(), MuParseErrorKind::ExpectedDot));

    let missing_close = parse_mu_formula("(diamond true").unwrap_err();
    assert!(matches!(
        missing_close.kind(),
        MuParseErrorKind::ExpectedCloseParen
    ));

    let invalid_escape = parse_mu_formula(r#""p\z""#).unwrap_err();
    assert!(matches!(
        invalid_escape.kind(),
        MuParseErrorKind::InvalidEscape { escape } if escape == "z"
    ));

    let empty = parse_mu_formula(r#""""#).unwrap_err();
    assert!(matches!(empty.kind(), MuParseErrorKind::EmptyProposition));
}

#[test]
fn declarative_frontend_validates_binding_and_monotonicity_before_execution() {
    let document = parse_declarative_document(MODEL).unwrap();

    let unbound = check_declarative_mu_text(&document, "diamond $X").unwrap_err();
    assert!(matches!(
        unbound,
        DeclarativeMuError::Validation(MuValidationError::UnboundVariable { variable })
            if variable == "X"
    ));

    let non_monotone = check_declarative_mu_text(&document, "mu X. not $X").unwrap_err();
    assert!(matches!(
        non_monotone,
        DeclarativeMuError::Validation(MuValidationError::NonMonotoneVariable { variable })
            if variable == "X"
    ));

    let unknown =
        check_declarative_mu_text(&document, r#"mu X. "missing" or diamond $X"#).unwrap_err();
    assert!(matches!(
        unknown,
        DeclarativeMuError::UnknownProposition { proposition }
            if proposition == "missing"
    ));
}

#[test]
fn declarative_frontend_delegates_exactly_to_m75_kernel() {
    let document = parse_declarative_document(MODEL).unwrap();
    let formula = parse_mu_formula(r#"nu X. "ready" and diamond $X"#).unwrap();

    let direct = evaluate_mu(document.model(), &formula, |atom, state| {
        document.state_has_proposition(state, atom)
    })
    .unwrap();
    let frontend = check_declarative_mu(&document, &formula).unwrap();

    assert_eq!(frontend.evaluation, direct);
    assert_eq!(frontend.formula, render_mu_formula(&formula));
    assert_eq!(frontend.status, DeclarativeMuStatus::Satisfied);
    assert_eq!(
        frontend.evaluation.terminal_policy,
        MuTerminalPolicy::TotalizeWithSelfLoop
    );
}

#[test]
fn textual_ctl_compatible_fixpoints_match_sealed_ctl_frontend() {
    let document = parse_declarative_document(MODEL).unwrap();
    let cases = [
        (r#"mu X. "complete" or diamond $X"#, r#"EF "complete""#),
        (r#"nu X. "ready" and box $X"#, r#"AG "ready""#),
        (
            r#"mu X. "complete" or ("ready" and diamond $X)"#,
            r#"E["ready" U "complete"]"#,
        ),
    ];

    for (mu_text, ctl_text) in cases {
        let mu = check_declarative_mu_text(&document, mu_text).unwrap();
        let ctl = check_declarative_ctl_text(&document, ctl_text).unwrap();
        assert_eq!(
            mu.evaluation.satisfying_state_indices, ctl.evaluation.satisfying_state_indices,
            "textual mu/CTL subset mismatch: {mu_text} vs {ctl_text}"
        );
    }
}

#[test]
fn terminal_modal_semantics_match_m75_totalization() {
    let document = parse_declarative_document(
        r#"
model "terminal"
state "done"
initial "done"
label "done" "complete"
"#,
    )
    .unwrap();

    let diamond = check_declarative_mu_text(&document, r#"diamond "complete""#).unwrap();
    let forever =
        check_declarative_mu_text(&document, r#"nu X. "complete" and diamond $X"#).unwrap();

    assert_eq!(diamond.status, DeclarativeMuStatus::Satisfied);
    assert_eq!(forever.status, DeclarativeMuStatus::Satisfied);
}

#[test]
fn built_binary_mu_file_covers_satisfaction_violation_and_fail_closed_errors() {
    let root = fixture_dir("cli");
    let path = root.join("model.fvl");
    fs::write(&path, MODEL).unwrap();
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = path.to_str().unwrap();

    let satisfied = Command::new(binary)
        .args(["mu", "file", path, r#"mu X. "complete" or diamond $X"#])
        .output()
        .unwrap();
    assert_eq!(satisfied.status.code(), Some(0));
    let stdout = String::from_utf8(satisfied.stdout).unwrap();
    assert!(stdout.contains("MU: SATISFIED"));
    assert!(stdout.contains("fixpoint iterations:"));
    assert!(stdout.contains("terminal policy: totalize reachable terminals"));

    let violated = Command::new(binary)
        .args(["mu", "file", path, r#"nu X. "complete" and box $X"#])
        .output()
        .unwrap();
    assert_eq!(violated.status.code(), Some(15));
    let stdout = String::from_utf8(violated.stdout).unwrap();
    assert!(stdout.contains("MU: VIOLATED"));

    let unbound = Command::new(binary)
        .args(["mu", "file", path, "diamond $X"])
        .output()
        .unwrap();
    assert_eq!(unbound.status.code(), Some(2));
    let stderr = String::from_utf8(unbound.stderr).unwrap();
    assert!(stderr.contains("unbound mu-calculus variable"));

    let non_monotone = Command::new(binary)
        .args(["mu", "file", path, "mu X. not $X"])
        .output()
        .unwrap();
    assert_eq!(non_monotone.status.code(), Some(2));
    let stderr = String::from_utf8(non_monotone.stderr).unwrap();
    assert!(stderr.contains("occurs negatively"));

    let unknown = Command::new(binary)
        .args(["mu", "file", path, r#""missing""#])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));
    let stderr = String::from_utf8(unknown.stderr).unwrap();
    assert!(stderr.contains("unknown mu-calculus proposition 'missing'"));

    let malformed = Command::new(binary)
        .args(["mu", "file", path, "mu X true"])
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8(malformed.stderr).unwrap();
    assert!(stderr.contains("mu-calculus parse error"));

    fs::remove_dir_all(root).unwrap();
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("fvlab-m76-mu-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

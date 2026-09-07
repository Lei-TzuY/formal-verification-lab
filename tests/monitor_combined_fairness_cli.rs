use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("CLI stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("CLI stderr should be UTF-8")
}

#[test]
fn mixed_monitor_fairness_is_accepted_reported_and_canonicalizes_overlap() {
    let mixed = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert!(mixed.status.success(), "{}", stderr(&mixed));
    let text = stdout(&mixed);
    assert!(text.contains("monitor verification: SATISFIED"));
    assert!(text.contains("weak fairness actions: 1"));
    assert!(text.contains("weak-fair action: \"close\""));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"unrelated\""));

    let overlap = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "close",
    ]);
    assert!(overlap.status.success(), "{}", stderr(&overlap));
    let overlap_text = stdout(&overlap);
    assert!(overlap_text.contains("weak fairness actions: 0"));
    assert!(overlap_text.contains("strong fairness actions: 1"));
    assert!(overlap_text.contains("strong-fair action: \"close\""));
}

#[test]
fn mixed_monitor_fairness_preserves_finite_and_rejecting_precedence() {
    let rejecting = run(&[
        "monitor",
        "session-double-open",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "commit",
    ]);
    assert_eq!(rejecting.status.code(), Some(8));
    let rejecting_text = stdout(&rejecting);
    assert!(rejecting_text.contains("counterexample: REJECTING_STATE"));
    assert!(rejecting_text.contains("weak-fair action: \"close\""));
    assert!(rejecting_text.contains("strong-fair action: \"commit\""));

    let terminal = run(&[
        "monitor",
        "session-open-terminal",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(terminal.status.code(), Some(8));
    let terminal_text = stdout(&terminal);
    assert!(terminal_text.contains("counterexample: PROGRESS_TERMINAL"));
    assert!(terminal_text.contains("weak-fair action: \"close\""));
    assert!(terminal_text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn mixed_monitor_fairness_preserves_product_and_model_cutoff_provenance() {
    let product = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
        "--max-product-transitions",
        "3",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_text = stdout(&product);
    assert!(product_text.contains("monitor verification: INCONCLUSIVE"));
    assert!(product_text.contains("product inconclusive reason: transition limit reached (max 3)"));
    assert!(product_text.contains("weak-fair action: \"close\""));
    assert!(product_text.contains("strong-fair action: \"unrelated\""));

    let staged = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(staged.status.code(), Some(3));
    let staged_text = stdout(&staged);
    assert!(staged_text.contains("analysis inconclusive stage: model"));
    assert!(staged_text.contains("analysis inconclusive reason: transition limit reached (max 2)"));
    assert!(staged_text.contains("weak-fair action: \"close\""));
    assert!(staged_text.contains("strong-fair action: \"unrelated\""));
}

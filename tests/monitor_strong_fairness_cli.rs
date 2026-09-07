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
fn strong_fair_close_filters_only_the_unfair_monitor_progress_lasso() {
    let historical = run(&["monitor", "session-unfair-close"]);
    assert_eq!(historical.status.code(), Some(8));
    let historical_text = stdout(&historical);
    assert!(historical_text.contains("monitor verification: VIOLATED"));
    assert!(historical_text.contains("counterexample: PROGRESS_CYCLE"));
    assert!(!historical_text.contains("strong fairness actions:"));

    let fair = run(&[
        "monitor",
        "session-unfair-close",
        "--strong-fair-action",
        "close",
    ]);
    assert!(fair.status.success(), "{}", stderr(&fair));
    let fair_text = stdout(&fair);
    assert!(fair_text.contains("model: session-unfair-close-enabled"));
    assert!(fair_text.contains("monitor verification: SATISFIED"));
    assert!(fair_text.contains("counterexample: none (monitor conditions hold)"));
    assert!(fair_text.contains("strong fairness actions: 1"));
    assert!(fair_text.contains("strong-fair action: \"close\""));
    assert!(!fair_text.contains("weak fairness actions:"));

    let unrelated = run(&[
        "monitor",
        "session-unfair-close",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(unrelated.status.code(), Some(8));
    let unrelated_text = stdout(&unrelated);
    assert!(unrelated_text.contains("monitor verification: VIOLATED"));
    assert!(unrelated_text.contains("counterexample: PROGRESS_CYCLE"));
    assert!(unrelated_text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn taken_strong_fair_action_does_not_hide_a_real_progress_cycle() {
    let output = run(&[
        "monitor",
        "session-stuck",
        "--strong-fair-action",
        "tick",
    ]);
    assert_eq!(output.status.code(), Some(8));
    let text = stdout(&output);
    assert!(text.contains("counterexample: PROGRESS_CYCLE"));
    assert!(text.contains("--tick-->"));
    assert!(text.contains("strong-fair action: \"tick\""));
}

#[test]
fn strong_fairness_never_excuses_rejecting_or_finite_terminal_monitor_violations() {
    let rejecting = run(&[
        "monitor",
        "session-double-open",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(rejecting.status.code(), Some(8));
    let rejecting_text = stdout(&rejecting);
    assert!(rejecting_text.contains("counterexample: REJECTING_STATE"));
    assert!(rejecting_text.contains("violated condition: legal-action-order"));
    assert!(rejecting_text.contains("strong-fair action: \"close\""));

    let terminal = run(&[
        "monitor",
        "session-open-terminal",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(terminal.status.code(), Some(8));
    let terminal_text = stdout(&terminal);
    assert!(terminal_text.contains("model: session-open-terminal"));
    assert!(terminal_text.contains("counterexample: PROGRESS_TERMINAL"));
    assert!(terminal_text.contains("violated condition: opened-session-eventually-closes"));
    assert!(terminal_text.contains("strong-fair action: \"close\""));
}

#[test]
fn bounded_strong_fair_monitor_cli_preserves_cutoff_honesty_and_exit_three() {
    let product = run(&[
        "monitor",
        "session-unfair-close",
        "--max-product-transitions",
        "3",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_text = stdout(&product);
    assert!(product_text.contains("monitor verification: INCONCLUSIVE"));
    assert!(product_text.contains("product inconclusive reason:"));
    assert!(product_text.contains("strong-fair action: \"close\""));
    assert!(!product_text.contains("counterexample: PROGRESS_CYCLE"));

    let staged = run(&[
        "monitor",
        "session-unfair-close",
        "--strong-fair-action",
        "close",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(staged.status.code(), Some(3));
    let staged_text = stdout(&staged);
    assert!(staged_text.contains("monitor verification: INCONCLUSIVE"));
    assert!(staged_text.contains("analysis inconclusive stage: model"));
    assert!(staged_text.contains("strong-fair action: \"close\""));
    assert!(!staged_text.contains("counterexample: PROGRESS_CYCLE"));
}

#[test]
fn monitor_strong_fairness_options_fail_closed_on_duplicate_empty_missing_or_mixed_actions() {
    let duplicate = run(&[
        "monitor",
        "session-unfair-close",
        "--strong-fair-action",
        "close",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate strong-fair action 'close'"));

    let empty = run(&["monitor", "session-unfair-close", "--strong-fair-action", ""]);
    assert_eq!(empty.status.code(), Some(2));
    assert!(stderr(&empty).contains("strong-fair action name must not be empty"));

    let missing = run(&["monitor", "session-unfair-close", "--strong-fair-action"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("option '--strong-fair-action' requires an action value"));

    let mixed = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(mixed.status.code(), Some(2));
    assert!(stderr(&mixed).contains("cannot combine weak and strong fairness assumptions"));
}

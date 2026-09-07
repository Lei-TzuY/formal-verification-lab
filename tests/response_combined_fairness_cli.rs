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
fn weak_strong_and_mixed_response_fairness_route_through_direct_cli() {
    let weak = run(&[
        "respond",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
    ]);
    assert!(weak.status.success(), "{}", stderr(&weak));
    let weak_text = stdout(&weak);
    assert!(weak_text.contains("response: SATISFIED"));
    assert!(weak_text.contains("weak fairness actions: 1"));
    assert!(weak_text.contains("weak-fair action: \"grant\""));
    assert!(weak_text.contains("strong fairness actions: 0"));

    let strong = run(&[
        "respond",
        "request-grant-unfair",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(strong.status.success(), "{}", stderr(&strong));
    let strong_text = stdout(&strong);
    assert!(strong_text.contains("response: SATISFIED"));
    assert!(strong_text.contains("weak fairness actions: 0"));
    assert!(strong_text.contains("strong fairness actions: 1"));
    assert!(strong_text.contains("strong-fair action: \"grant\""));

    let mixed = run(&[
        "respond",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert!(mixed.status.success(), "{}", stderr(&mixed));
    let mixed_text = stdout(&mixed);
    assert!(mixed_text.contains("response: SATISFIED"));
    assert!(mixed_text.contains("weak-fair action: \"grant\""));
    assert!(mixed_text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn overlapping_response_fairness_is_canonicalized_to_strong() {
    let output = run(&[
        "respond",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("weak fairness actions: 0"));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"grant\""));
}

#[test]
fn unrelated_mixed_fairness_preserves_real_pending_lasso() {
    let output = run(&[
        "respond",
        "request-grant-unfair",
        "--weak-fair-action",
        "unrelated-weak",
        "--strong-fair-action",
        "unrelated-strong",
    ]);
    assert_eq!(output.status.code(), Some(7));
    let text = stdout(&output);
    assert!(text.contains("response: VIOLATED"));
    assert!(text.contains("counterexample: PENDING_CYCLE"));
    assert!(text.contains("--wait-->"));
    assert!(text.contains("weak-fair action: \"unrelated-weak\""));
    assert!(text.contains("strong-fair action: \"unrelated-strong\""));
}

#[test]
fn fairness_never_excuses_a_finite_pending_response_terminal() {
    let output = run(&[
        "respond",
        "request-grant-terminal",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(output.status.code(), Some(7));
    let text = stdout(&output);
    assert!(text.contains("model: request-grant-terminal"));
    assert!(text.contains("response: VIOLATED"));
    assert!(text.contains("counterexample: PENDING_TERMINAL"));
    assert!(text.contains("--request-->"));
}

#[test]
fn fair_response_product_cutoff_is_inconclusive_before_terminal_and_conclusive_after() {
    let before = run(&[
        "respond",
        "request-grant-terminal",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "unrelated",
        "--max-product-states",
        "1",
    ]);
    assert_eq!(before.status.code(), Some(3));
    let before_text = stdout(&before);
    assert!(before_text.contains("response: INCONCLUSIVE"));
    assert!(before_text.contains("product inconclusive reason: state limit reached (max 1)"));
    assert!(before_text.contains("counterexample: none (product exploration incomplete)"));

    let after = run(&[
        "respond",
        "request-grant-terminal",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "unrelated",
        "--max-product-states",
        "2",
    ]);
    assert_eq!(after.status.code(), Some(7));
    let after_text = stdout(&after);
    assert!(after_text.contains("response: VIOLATED"));
    assert!(after_text.contains("counterexample: PENDING_TERMINAL"));
}

#[test]
fn staged_response_preserves_model_before_product_cutoff_provenance() {
    let output = run(&[
        "respond",
        "request-grant-terminal",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "unrelated",
        "--max-model-states",
        "1",
        "--max-product-states",
        "0",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let text = stdout(&output);
    assert!(text.contains("response: INCONCLUSIVE"));
    assert!(text.contains("analysis inconclusive stage: model"));
    assert!(text.contains("analysis inconclusive reason: state limit reached (max 1)"));
    assert!(text.contains("model completion: INCONCLUSIVE"));
}

#[test]
fn response_fairness_validation_and_historical_no_option_path_are_explicit() {
    let duplicate = run(&[
        "respond",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--weak-fair-action",
        "grant",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate weak-fair action 'grant'"));

    let missing = run(&["respond", "request-grant-unfair", "--strong-fair-action"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("option '--strong-fair-action' requires an action value"));

    let historical = run(&["respond", "request-grant-unfair"]);
    assert_eq!(historical.status.code(), Some(7));
    let historical_text = stdout(&historical);
    assert!(historical_text.contains("response: VIOLATED"));
    assert!(historical_text.contains("counterexample: PENDING_CYCLE"));
    assert!(!historical_text.contains("weak fairness actions:"));
    assert!(!historical_text.contains("strong fairness actions:"));

    let usage = run(&[
        "respond",
        "request-grant-unfair",
        "--unknown-option",
        "1",
    ]);
    assert_eq!(usage.status.code(), Some(2));
    let usage_text = stderr(&usage);
    assert!(usage_text.contains(
        "respond <request-grant|request-grant-unfair|request-grant-terminal>"
    ));
    assert!(usage_text.contains("[--weak-fair-action ACTION]..."));
    assert!(usage_text.contains("[--strong-fair-action ACTION]..."));
}

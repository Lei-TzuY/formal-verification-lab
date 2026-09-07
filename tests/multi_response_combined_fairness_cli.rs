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
fn mixed_multi_response_fairness_routes_and_reports_both_classes() {
    let output = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("multi-response: SATISFIED"));
    assert!(text.contains("weak fairness actions: 1"));
    assert!(text.contains("weak-fair action: \"grant-b\""));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn weak_only_and_strong_only_multi_response_routes_preserve_compatibility() {
    let weak = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
    ]);
    assert!(weak.status.success(), "{}", stderr(&weak));
    let weak_text = stdout(&weak);
    assert!(weak_text.contains("multi-response: SATISFIED"));
    assert!(weak_text.contains("weak fairness actions: 1"));
    assert!(weak_text.contains("weak-fair action: \"grant-b\""));
    assert!(weak_text.contains("strong fairness actions: 0"));

    let strong = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--strong-fair-action",
        "grant-b",
    ]);
    assert!(strong.status.success(), "{}", stderr(&strong));
    let strong_text = stdout(&strong);
    assert!(strong_text.contains("multi-response: SATISFIED"));
    assert!(strong_text.contains("weak fairness actions: 0"));
    assert!(strong_text.contains("strong fairness actions: 1"));
    assert!(strong_text.contains("strong-fair action: \"grant-b\""));

    let unrelated = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(unrelated.status.code(), Some(7));
    let unrelated_text = stdout(&unrelated);
    assert!(unrelated_text.contains("multi-response: VIOLATED"));
    assert!(unrelated_text.contains("violated clause: class-b"));
}

#[test]
fn overlapping_multi_response_fairness_is_canonicalized_to_strong() {
    let output = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "grant-b",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("weak fairness actions: 0"));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"grant-b\""));
}

#[test]
fn fairness_never_excuses_a_finite_pending_multi_response_terminal() {
    let output = run(&[
        "respond",
        "dual-grant-terminal-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(output.status.code(), Some(7));
    let text = stdout(&output);
    assert!(text.contains("multi-response: VIOLATED"));
    assert!(text.contains("violated clause: class-b"));
    assert!(text.contains("counterexample: PENDING_TERMINAL"));
    assert!(text.contains("weak-fair action: \"grant-b\""));
    assert!(text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn mixed_multi_response_product_and_model_cutoffs_remain_inconclusive() {
    let product = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
        "--max-product-transitions",
        "3",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_text = stdout(&product);
    assert!(product_text.contains("multi-response: INCONCLUSIVE"));
    assert!(product_text.contains("product inconclusive reason: transition limit reached (max 3)"));
    assert!(product_text.contains("weak-fair action: \"grant-b\""));
    assert!(product_text.contains("strong-fair action: \"unrelated\""));

    let model = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(model.status.code(), Some(3));
    let model_text = stdout(&model);
    assert!(model_text.contains("analysis inconclusive stage: model"));
    assert!(model_text.contains("analysis inconclusive reason: transition limit reached (max 2)"));
    assert!(model_text.contains("weak-fair action: \"grant-b\""));
    assert!(model_text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn multi_response_fairness_validation_fails_closed() {
    let duplicate = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--weak-fair-action",
        "grant-b",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate weak-fair action 'grant-b'"));

    let missing = run(&["respond", "dual-grant-unfair-b", "--strong-fair-action"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("option '--strong-fair-action' requires an action value"));
}

#[test]
fn historical_no_option_multi_response_path_and_usage_remain_explicit() {
    let output = run(&["respond", "dual-grant"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("multi-response: SATISFIED"));
    assert!(!text.contains("weak fairness actions:"));
    assert!(!text.contains("strong fairness actions:"));

    let usage = run(&["respond", "dual-grant", "--unknown-option", "1"]);
    assert_eq!(usage.status.code(), Some(2));
    let usage_text = stderr(&usage);
    assert!(usage_text.contains("respond <dual-grant|dual-grant-unfair-b|dual-grant-terminal-b>"));
    assert!(usage_text.contains("[--weak-fair-action ACTION]..."));
    assert!(usage_text.contains("[--strong-fair-action ACTION]..."));
}

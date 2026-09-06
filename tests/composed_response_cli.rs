use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

#[test]
fn model_stage_cutoff_is_reported_with_stage_provenance() {
    let output = run(&[
        "respond",
        "request-grant",
        "--max-model-depth",
        "0",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let output = stdout(&output);

    assert!(output.contains("response: INCONCLUSIVE"));
    assert!(output.contains("analysis inconclusive stage: model"));
    assert!(output.contains("analysis inconclusive reason: depth limit reached (max 0)"));
    assert!(output.contains("model completion: INCONCLUSIVE"));
    assert!(output.contains("model inconclusive reason: depth limit reached (max 0)"));
    assert!(output.contains("product completion: COMPLETE"));
    assert!(output.contains("counterexample: none (analysis incomplete)"));
    assert!(!output.contains("counterexample: none (product exploration incomplete)"));
}

#[test]
fn model_stage_has_cli_precedence_when_both_stages_cut_off() {
    let output = run(&[
        "respond",
        "request-grant",
        "--max-model-depth",
        "0",
        "--max-product-states",
        "0",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let output = stdout(&output);

    assert!(output.contains("analysis inconclusive stage: model"));
    assert!(output.contains("analysis inconclusive reason: depth limit reached (max 0)"));
    assert!(output.contains("model completion: INCONCLUSIVE"));
    assert!(output.contains("product completion: INCONCLUSIVE"));
    assert!(output.contains("product inconclusive reason: state limit reached (max 0)"));
}

#[test]
fn retained_pending_cycle_is_conclusive_despite_later_model_cutoff() {
    let output = run(&[
        "respond",
        "request-grant-unfair",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(output.status.code(), Some(7));
    let output = stdout(&output);

    assert!(output.contains("response: VIOLATED"));
    assert!(output.contains("model completion: INCONCLUSIVE"));
    assert!(output.contains("model inconclusive reason: transition limit reached (max 2)"));
    assert!(output.contains("product completion: COMPLETE"));
    assert!(output.contains("counterexample: PENDING_CYCLE"));
    assert!(output.contains("--wait-->"));
    assert!(!output.contains("response: INCONCLUSIVE"));
}

#[test]
fn generous_independent_budgets_can_prove_satisfaction() {
    let output = run(&[
        "respond",
        "request-grant",
        "--max-model-states",
        "2",
        "--max-model-transitions",
        "2",
        "--max-model-depth",
        "1",
        "--max-product-states",
        "2",
        "--max-product-transitions",
        "2",
        "--max-product-depth",
        "1",
    ]);
    assert_eq!(output.status.code(), Some(0));
    let output = stdout(&output);

    assert!(output.contains("response: SATISFIED"));
    assert!(output.contains("model completion: COMPLETE"));
    assert!(output.contains("product completion: COMPLETE"));
    assert!(output.contains("model states: 2"));
    assert!(output.contains("product states: 2"));
    assert!(!output.contains("analysis inconclusive stage:"));
}

#[test]
fn multi_response_uses_the_same_staged_cli_contract() {
    let output = run(&[
        "respond",
        "dual-grant",
        "--max-model-depth",
        "0",
        "--max-product-states",
        "1",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let output = stdout(&output);

    assert!(output.contains("multi-response: INCONCLUSIVE"));
    assert!(output.contains("clauses: 2"));
    assert!(output.contains("analysis inconclusive stage: model"));
    assert!(output.contains("model completion: INCONCLUSIVE"));
    assert!(output.contains("product completion: COMPLETE"));
    assert!(output.contains("counterexample: none (analysis incomplete)"));
}

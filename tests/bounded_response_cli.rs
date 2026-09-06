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

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

#[test]
fn response_cli_exposes_honest_product_space_limits() {
    let satisfied = run(&[
        "respond",
        "request-grant",
        "--max-product-states",
        "2",
        "--max-product-transitions",
        "2",
        "--max-product-depth",
        "1",
    ]);
    assert_eq!(satisfied.status.code(), Some(0));
    let satisfied_stdout = stdout(&satisfied);
    assert!(satisfied_stdout.contains("response: SATISFIED"));
    assert!(satisfied_stdout.contains("product states: 2"));
    assert!(satisfied_stdout.contains("checked product states: 2"));
    assert!(satisfied_stdout.contains("explored product transitions: 2"));
    assert!(satisfied_stdout.contains("retained product transitions: 2"));
    assert!(satisfied_stdout.contains("max product depth reached: 1"));

    let inconclusive = run(&[
        "respond",
        "request-grant",
        "--max-product-depth",
        "0",
    ]);
    assert_eq!(inconclusive.status.code(), Some(3));
    let inconclusive_stdout = stdout(&inconclusive);
    assert!(inconclusive_stdout.contains("response: INCONCLUSIVE"));
    assert!(inconclusive_stdout.contains("product inconclusive reason: depth limit reached (max 0)"));
    assert!(inconclusive_stdout.contains("model states: 2"));
    assert!(inconclusive_stdout.contains("model transitions: 2"));
    assert!(inconclusive_stdout.contains("product states: 1"));
    assert!(inconclusive_stdout.contains("explored product transitions: 1"));
    assert!(inconclusive_stdout.contains("retained product transitions: 0"));
    assert!(inconclusive_stdout.contains("counterexample: none (product exploration incomplete)"));
    assert!(!inconclusive_stdout.contains("response: SATISFIED"));

    let conclusive_cycle = run(&[
        "respond",
        "request-grant-unfair",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(conclusive_cycle.status.code(), Some(7));
    let cycle_stdout = stdout(&conclusive_cycle);
    assert!(cycle_stdout.contains("response: VIOLATED"));
    assert!(cycle_stdout.contains("counterexample: PENDING_CYCLE"));
    assert!(cycle_stdout.contains("explored product transitions: 2"));
    assert!(cycle_stdout.contains("retained product transitions: 2"));
    assert!(cycle_stdout.contains("--wait-->"));
    assert!(!cycle_stdout.contains("response: INCONCLUSIVE"));
}

#[test]
fn multi_response_cli_uses_the_same_product_limit_contract() {
    let output = run(&[
        "respond",
        "dual-grant",
        "--max-product-states",
        "1",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("multi-response: INCONCLUSIVE"));
    assert!(output_stdout.contains("clauses: 2"));
    assert!(output_stdout.contains("product inconclusive reason: state limit reached (max 1)"));
    assert!(output_stdout.contains("model states: 4"));
    assert!(output_stdout.contains("model transitions: 4"));
    assert!(output_stdout.contains("counterexample: none (product exploration incomplete)"));
}

#[test]
fn response_cli_rejects_model_limit_flags_and_malformed_product_limits() {
    let wrong_namespace = run(&["respond", "request-grant", "--max-states", "1"]);
    assert_eq!(wrong_namespace.status.code(), Some(2));
    assert!(stderr(&wrong_namespace).contains("unknown option '--max-states'"));

    let missing_value = run(&["respond", "request-grant", "--max-product-depth"]);
    assert_eq!(missing_value.status.code(), Some(2));
    assert!(stderr(&missing_value)
        .contains("option '--max-product-depth' requires an integer value"));

    let duplicate = run(&[
        "respond",
        "request-grant",
        "--max-product-states",
        "1",
        "--max-product-states",
        "2",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate option '--max-product-states'"));
}

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
fn monitor_cli_exposes_honest_product_space_inconclusive() {
    let output = run(&["monitor", "session-ok", "--max-product-states", "1"]);
    assert_eq!(output.status.code(), Some(3));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("monitor verification: INCONCLUSIVE"));
    assert!(output_stdout.contains("product inconclusive reason: state limit reached (max 1)"));
    assert!(output_stdout.contains("model states: 3"));
    assert!(output_stdout.contains("model transitions: 3"));
    assert!(output_stdout.contains("product states: 1"));
    assert!(output_stdout.contains("counterexample: none (product exploration incomplete)"));
    assert!(!output_stdout.contains("monitor verification: SATISFIED"));
}

#[test]
fn bounded_monitor_cli_preserves_real_rejecting_and_cycle_violations() {
    let rejecting = run(&[
        "monitor",
        "session-double-open",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(rejecting.status.code(), Some(8));
    let rejecting_stdout = stdout(&rejecting);
    assert!(rejecting_stdout.contains("monitor verification: VIOLATED"));
    assert!(rejecting_stdout.contains("counterexample: REJECTING_STATE"));
    assert!(rejecting_stdout.contains("violated condition: legal-action-order"));
    assert!(rejecting_stdout.contains("explored product transitions: 2"));

    let cycle = run(&["monitor", "session-stuck", "--max-product-transitions", "3"]);
    assert_eq!(cycle.status.code(), Some(8));
    let cycle_stdout = stdout(&cycle);
    assert!(cycle_stdout.contains("monitor verification: VIOLATED"));
    assert!(cycle_stdout.contains("counterexample: PROGRESS_CYCLE"));
    assert!(cycle_stdout.contains("violated condition: opened-session-eventually-closes"));
    assert!(cycle_stdout.contains("--tick-->"));
}

#[test]
fn bounded_monitor_cli_can_establish_satisfaction_and_validates_flag_namespace() {
    let satisfied = run(&[
        "monitor",
        "session-ok",
        "--max-product-states",
        "10",
        "--max-product-transitions",
        "10",
        "--max-product-depth",
        "10",
    ]);
    assert_eq!(satisfied.status.code(), Some(0));
    let satisfied_stdout = stdout(&satisfied);
    assert!(satisfied_stdout.contains("monitor verification: SATISFIED"));
    assert!(satisfied_stdout.contains("counterexample: none (monitor conditions hold)"));

    let wrong_namespace = run(&["monitor", "session-ok", "--max-states", "1"]);
    assert_eq!(wrong_namespace.status.code(), Some(2));
    assert!(stderr(&wrong_namespace).contains("unknown option '--max-states'"));
}

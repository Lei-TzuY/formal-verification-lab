use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should run")
}

#[test]
fn bounded_buchi_cli_reports_inconclusive_product_cutoff() {
    let output = run(&["buchi", "pulses", "--max-product-states", "1"]);

    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Buchi verification: INCONCLUSIVE"));
    assert!(stdout.contains("product inconclusive reason: state limit reached (max 1)"));
    assert!(stdout.contains("product states: 1"));
    assert!(stdout.contains("counterexample: none (product exploration incomplete)"));
    assert!(!stdout.contains("Buchi verification: SATISFIED"));
}

#[test]
fn bounded_buchi_cli_preserves_real_cycle_violation_before_cutoff() {
    let output = run(&["buchi", "pulses-unfair", "--max-product-transitions", "2"]);

    assert_eq!(output.status.code(), Some(9));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Buchi verification: VIOLATED"));
    assert!(stdout.contains("counterexample: ACCEPTANCE_AVOIDING_CYCLE"));
    assert!(stdout.contains("avoided acceptance set: pulse-b-observed"));
    assert!(stdout.contains("explored product transitions: 2"));
    assert!(stdout.contains("--pulse-a-->"));
}

#[test]
fn bounded_buchi_cli_supports_generous_limits_and_rejects_model_limit_namespace() {
    let satisfied = run(&[
        "buchi",
        "pulses",
        "--max-product-states",
        "10",
        "--max-product-transitions",
        "10",
        "--max-product-depth",
        "10",
    ]);
    assert_eq!(satisfied.status.code(), Some(0));
    let stdout = String::from_utf8(satisfied.stdout).unwrap();
    assert!(stdout.contains("Buchi verification: SATISFIED"));
    assert!(stdout.contains("counterexample: none (all configured acceptance obligations hold)"));

    let wrong_namespace = run(&["buchi", "pulses", "--max-states", "1"]);
    assert_eq!(wrong_namespace.status.code(), Some(2));
    let stderr = String::from_utf8(wrong_namespace.stderr).unwrap();
    assert!(stderr.contains("unknown option '--max-states'"));
}

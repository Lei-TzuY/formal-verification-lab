use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary executes")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

#[test]
fn textual_staged_product_cutoff_reports_product_provenance_with_fairness() {
    let expression = "infinitely-often(\"pulse-a\",\"pulse-b\")";
    let output = run(&[
        "temporal",
        "check",
        "pulses-unfair",
        expression,
        "--weak-fair-action",
        "pulse-b",
        "--max-model-transitions",
        "16",
        "--max-product-transitions",
        "2",
    ]);

    assert_eq!(output.status.code(), Some(3));
    let out = stdout(&output);
    assert!(out.contains("temporal: INCONCLUSIVE"));
    assert!(out.contains("analysis inconclusive stage: product"));
    assert!(out.contains("model completion: COMPLETE"));
    assert!(out.contains("product completion: INCONCLUSIVE"));
    assert!(out.contains("weak-fair action: \"pulse-b\""));
    assert!(out.contains("counterexample: none (analysis incomplete)"));
}

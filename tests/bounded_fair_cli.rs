use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary executes")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn temp_model_path() -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fvlab-bounded-fair-cli-{}-{id}.fvl",
        std::process::id()
    ))
}

fn write_unfair_pulse_model() -> PathBuf {
    let path = temp_model_path();
    fs::write(
        &path,
        concat!(
            "model \"bounded-unfair-pulses\"\n",
            "state \"first\"\n",
            "state \"second\"\n",
            "initial \"first\"\n",
            "edge \"first\" \"pulse-a\" \"second\"\n",
            "edge \"second\" \"pulse-a\" \"second\"\n",
            "edge \"second\" \"pulse-b\" \"first\"\n",
        ),
    )
    .expect("temporary declarative model is written");
    path
}

#[test]
fn fixed_product_cutoff_hidden_fair_edge_is_inconclusive() {
    let output = run(&[
        "temporal",
        "pulses-unfair",
        "--max-product-transitions",
        "2",
        "--weak-fair-action",
        "pulse-b",
    ]);

    assert_eq!(output.status.code(), Some(3));
    let out = stdout(&output);
    assert!(out.contains("temporal: INCONCLUSIVE"));
    assert!(out.contains("product inconclusive reason:"));
    assert!(out.contains("weak-fair action: \"pulse-b\""));
    assert!(out.contains("counterexample: none (product exploration incomplete)"));
}

#[test]
fn textual_staged_model_cutoff_preserves_model_stage_provenance() {
    let expression = "infinitely-often(\"pulse-a\",\"pulse-b\")";
    let output = run(&[
        "temporal",
        "check",
        "pulses-unfair",
        expression,
        "--weak-fair-action",
        "pulse-b",
        "--max-model-transitions",
        "2",
    ]);

    assert_eq!(output.status.code(), Some(3));
    let out = stdout(&output);
    assert!(out.contains("temporal: INCONCLUSIVE"));
    assert!(out.contains("analysis inconclusive stage: model"));
    assert!(out.contains("model completion: INCONCLUSIVE"));
    assert!(out.contains("weak-fair action: \"pulse-b\""));
    assert!(out.contains("counterexample: none (analysis incomplete)"));
}

#[test]
fn declarative_retained_weakly_fair_cycle_remains_a_real_violation() {
    let path = write_unfair_pulse_model();
    let expression = "infinitely-often(\"pulse-a\",\"pulse-b\")";
    let output = run(&[
        "temporal",
        "file",
        path.to_str().expect("temporary path is utf-8"),
        expression,
        "--max-product-transitions",
        "2",
        "--weak-fair-action",
        "pulse-a",
    ]);
    let _ = fs::remove_file(&path);

    assert_eq!(output.status.code(), Some(10));
    let out = stdout(&output);
    assert!(out.contains("model: bounded-unfair-pulses"));
    assert!(out.contains("temporal: VIOLATED"));
    assert!(out.contains("counterexample: INFINITE"));
    assert!(out.contains("obligation: infinitely-often action 'pulse-b'"));
    assert!(out.contains("weak-fair action: \"pulse-a\""));
}

#[test]
fn generous_product_budget_matches_unbounded_matching_fairness() {
    let bounded = run(&[
        "temporal",
        "pulses-unfair",
        "--weak-fair-action",
        "pulse-b",
        "--max-product-transitions",
        "16",
    ]);
    assert_eq!(bounded.status.code(), Some(0));
    let bounded_stdout = stdout(&bounded);
    assert!(bounded_stdout.contains("temporal: SATISFIED"));
    assert!(bounded_stdout.contains("weak-fair action: \"pulse-b\""));

    let unbounded = run(&["temporal", "pulses-unfair", "--weak-fair-action", "pulse-b"]);
    assert_eq!(unbounded.status.code(), Some(0));
}

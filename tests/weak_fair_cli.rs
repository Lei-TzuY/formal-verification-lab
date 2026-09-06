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

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

fn temp_model_path() -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fvlab-weak-fair-cli-{}-{id}.fvl",
        std::process::id()
    ))
}

fn write_unfair_request_grant_model() -> PathBuf {
    let path = temp_model_path();
    fs::write(
        &path,
        concat!(
            "model \"unfair-request-grant-file\"\n",
            "state \"idle\"\n",
            "state \"waiting\"\n",
            "initial \"idle\"\n",
            "edge \"idle\" \"request\" \"waiting\"\n",
            "edge \"waiting\" \"wait\" \"waiting\"\n",
            "edge \"waiting\" \"grant\" \"idle\"\n",
        ),
    )
    .expect("temporary declarative model is written");
    path
}

#[test]
fn fixed_temporal_weak_fairness_excludes_only_the_matching_unfair_lasso() {
    let baseline = run(&["temporal", "pulses-unfair"]);
    assert_eq!(baseline.status.code(), Some(10));

    let fair_b = run(&["temporal", "pulses-unfair", "--weak-fair-action", "pulse-b"]);
    assert_eq!(fair_b.status.code(), Some(0));
    let fair_b_stdout = stdout(&fair_b);
    assert!(fair_b_stdout.contains("temporal: SATISFIED"));
    assert!(fair_b_stdout.contains("weak fairness actions: 1"));
    assert!(fair_b_stdout.contains("weak-fair action: \"pulse-b\""));

    let fair_a = run(&["temporal", "pulses-unfair", "--weak-fair-action", "pulse-a"]);
    assert_eq!(fair_a.status.code(), Some(10));
    let fair_a_stdout = stdout(&fair_a);
    assert!(fair_a_stdout.contains("temporal: VIOLATED"));
    assert!(fair_a_stdout.contains("weak-fair action: \"pulse-a\""));
    assert!(fair_a_stdout.contains("counterexample: INFINITE"));
}

#[test]
fn textual_and_declarative_temporal_paths_share_the_same_fairness_surface() {
    let expression = "infinitely-often(\"pulse-a\",\"pulse-b\")";
    let textual = run(&[
        "temporal",
        "check",
        "pulses-unfair",
        expression,
        "--weak-fair-action",
        "pulse-b",
    ]);
    assert_eq!(textual.status.code(), Some(0));
    assert!(stdout(&textual).contains("weak-fair action: \"pulse-b\""));

    let path = temp_model_path();
    fs::write(
        &path,
        concat!(
            "model \"unfair-pulses-file\"\n",
            "state \"first\"\n",
            "state \"second\"\n",
            "initial \"first\"\n",
            "edge \"first\" \"pulse-a\" \"second\"\n",
            "edge \"second\" \"pulse-a\" \"second\"\n",
            "edge \"second\" \"pulse-b\" \"first\"\n",
        ),
    )
    .expect("temporary declarative model is written");

    let declarative = run(&[
        "temporal",
        "file",
        path.to_str().expect("temporary path is utf-8"),
        expression,
        "--weak-fair-action",
        "pulse-b",
    ]);
    let _ = fs::remove_file(&path);

    assert_eq!(declarative.status.code(), Some(0));
    let declarative_stdout = stdout(&declarative);
    assert!(declarative_stdout.contains("model: unfair-pulses-file"));
    assert!(declarative_stdout.contains("temporal: SATISFIED"));
    assert!(declarative_stdout.contains("weak-fair action: \"pulse-b\""));
}

#[test]
fn fairness_declarations_preserve_order_and_fail_closed_on_duplicates() {
    let ordered = run(&[
        "temporal",
        "pulses",
        "--weak-fair-action",
        "pulse-b",
        "--weak-fair-action",
        "pulse-a",
    ]);
    assert_eq!(ordered.status.code(), Some(0));
    let ordered_stdout = stdout(&ordered);
    let b = ordered_stdout
        .find("weak-fair action: \"pulse-b\"")
        .expect("pulse-b fairness declaration is rendered");
    let a = ordered_stdout
        .find("weak-fair action: \"pulse-a\"")
        .expect("pulse-a fairness declaration is rendered");
    assert!(b < a, "fairness declarations preserve input order");

    let duplicate = run(&[
        "temporal",
        "pulses",
        "--weak-fair-action",
        "pulse-b",
        "--weak-fair-action",
        "pulse-b",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate weak-fair action 'pulse-b'"));
}

#[test]
fn response_fairness_routes_fixed_textual_and_declarative_paths() {
    let baseline = run(&["temporal", "request-grant-unfair"]);
    assert_eq!(baseline.status.code(), Some(10));

    let fixed = run(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
    ]);
    assert_eq!(fixed.status.code(), Some(0));
    let fixed_out = stdout(&fixed);
    assert!(fixed_out.contains("backend: RESPONSE"));
    assert!(fixed_out.contains("temporal: SATISFIED"));
    assert!(fixed_out.contains("weak-fair action: \"grant\""));

    let expression = "response(\"request\",\"grant\")";
    let textual = run(&[
        "temporal",
        "check",
        "request-grant-unfair",
        expression,
        "--weak-fair-action",
        "grant",
    ]);
    assert_eq!(textual.status.code(), Some(0));
    assert!(stdout(&textual).contains("backend: RESPONSE"));

    let path = write_unfair_request_grant_model();
    let declarative = run(&[
        "temporal",
        "file",
        path.to_str().expect("temporary path is utf-8"),
        expression,
        "--weak-fair-action",
        "grant",
    ]);
    let _ = fs::remove_file(&path);
    assert_eq!(declarative.status.code(), Some(0));
    let declarative_out = stdout(&declarative);
    assert!(declarative_out.contains("model: unfair-request-grant-file"));
    assert!(declarative_out.contains("backend: RESPONSE"));
    assert!(declarative_out.contains("temporal: SATISFIED"));

    let fair_wait = run(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "wait",
    ]);
    assert_eq!(fair_wait.status.code(), Some(10));
    let fair_wait_out = stdout(&fair_wait);
    assert!(fair_wait_out.contains("temporal: VIOLATED"));
    assert!(fair_wait_out.contains("obligation: response"));
    assert!(fair_wait_out.contains("counterexample: INFINITE"));
}

#[test]
fn response_fairness_preserves_product_and_model_cutoff_honesty() {
    let product = run(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_out = stdout(&product);
    assert!(product_out.contains("temporal: INCONCLUSIVE"));
    assert!(product_out.contains("product inconclusive reason:"));
    assert!(product_out.contains("weak-fair action: \"grant\""));
    assert!(product_out.contains("counterexample: none (product exploration incomplete)"));

    let staged = run(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(staged.status.code(), Some(3));
    let staged_out = stdout(&staged);
    assert!(staged_out.contains("temporal: INCONCLUSIVE"));
    assert!(staged_out.contains("analysis inconclusive stage: model"));
    assert!(staged_out.contains("model completion: INCONCLUSIVE"));
    assert!(staged_out.contains("weak-fair action: \"grant\""));
}

#[test]
fn fairness_option_requires_an_action_value() {
    let output = run(&["temporal", "pulses", "--weak-fair-action"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("option '--weak-fair-action' requires an action value"));
}

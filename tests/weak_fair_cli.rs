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

#[test]
fn fixed_temporal_weak_fairness_excludes_only_the_matching_unfair_lasso() {
    let baseline = run(&["temporal", "pulses-unfair"]);
    assert_eq!(baseline.status.code(), Some(10));

    let fair_b = run(&[
        "temporal",
        "pulses-unfair",
        "--weak-fair-action",
        "pulse-b",
    ]);
    assert_eq!(fair_b.status.code(), Some(0));
    let fair_b_stdout = stdout(&fair_b);
    assert!(fair_b_stdout.contains("temporal: SATISFIED"));
    assert!(fair_b_stdout.contains("weak fairness actions: 1"));
    assert!(fair_b_stdout.contains("weak-fair action: \"pulse-b\""));

    let fair_a = run(&[
        "temporal",
        "pulses-unfair",
        "--weak-fair-action",
        "pulse-a",
    ]);
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
fn fairness_rejects_response_and_bounded_or_staged_combinations() {
    let response = run(&[
        "temporal",
        "request-grant",
        "--weak-fair-action",
        "grant",
    ]);
    assert_eq!(response.status.code(), Some(2));
    assert!(stderr(&response).contains(
        "weak fairness is currently supported only for infinitely-often temporal specifications"
    ));

    for bounded_args in [
        vec!["--max-product-states", "2"],
        vec!["--max-model-states", "2"],
    ] {
        let mut args = vec![
            "temporal",
            "pulses-unfair",
            "--weak-fair-action",
            "pulse-b",
        ];
        args.extend(bounded_args);
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(stderr(&output).contains(
            "weak fairness cannot be combined with bounded or staged temporal limits"
        ));
    }
}

#[test]
fn fairness_option_requires_an_action_value() {
    let output = run(&["temporal", "pulses", "--weak-fair-action"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("option '--weak-fair-action' requires an action value"));
}

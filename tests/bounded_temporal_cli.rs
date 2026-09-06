use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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
fn fixed_temporal_examples_expose_product_limits_and_preserve_violation_exit() {
    let inconclusive = run(&[
        "temporal",
        "request-grant",
        "--max-product-states",
        "1",
    ]);
    assert_eq!(inconclusive.status.code(), Some(3));
    let inconclusive_stdout = stdout(&inconclusive);
    assert!(inconclusive_stdout.contains("backend: RESPONSE"));
    assert!(inconclusive_stdout.contains("temporal: INCONCLUSIVE"));
    assert!(inconclusive_stdout.contains("product inconclusive reason: state limit reached (max 1)"));
    assert!(inconclusive_stdout.contains("counterexample: none (product exploration incomplete)"));

    let violated = run(&[
        "temporal",
        "request-grant-unfair",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(violated.status.code(), Some(10));
    let violated_stdout = stdout(&violated);
    assert!(violated_stdout.contains("backend: RESPONSE"));
    assert!(violated_stdout.contains("temporal: VIOLATED"));
    assert!(violated_stdout.contains("obligation: response"));
    assert!(violated_stdout.contains("--wait-->"));

    let satisfied = run(&[
        "temporal",
        "request-grant",
        "--max-product-states",
        "2",
        "--max-product-transitions",
        "2",
        "--max-product-depth",
        "1",
    ]);
    assert_eq!(satisfied.status.code(), Some(0));
    assert!(stdout(&satisfied).contains("temporal: SATISFIED"));
}

#[test]
fn textual_temporal_check_routes_bounded_recurring_properties_to_buchi() {
    let violated = run(&[
        "temporal",
        "check",
        "pulses-unfair",
        "infinitely-often(\"pulse-a\",\"pulse-b\")",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(violated.status.code(), Some(10));
    let violated_stdout = stdout(&violated);
    assert!(violated_stdout.contains("backend: BUCHI"));
    assert!(violated_stdout.contains("temporal: VIOLATED"));
    assert!(violated_stdout.contains("obligation: infinitely-often action 'pulse-b'"));

    let inconclusive = run(&[
        "temporal",
        "check",
        "pulses",
        "infinitely-often(\"pulse-a\",\"pulse-b\")",
        "--max-product-states",
        "1",
    ]);
    assert_eq!(inconclusive.status.code(), Some(3));
    assert!(stdout(&inconclusive).contains("temporal: INCONCLUSIVE"));
}

#[test]
fn declarative_temporal_file_accepts_the_same_product_limit_namespace() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fvlab-m27-{nonce}.fvl"));
    fs::write(
        &path,
        "model \"request-grant-file\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n",
    )
    .unwrap();

    let path_string = path.to_string_lossy().into_owned();
    let output = run(&[
        "temporal",
        "file",
        &path_string,
        "response(\"request\",\"grant\")",
        "--max-product-states",
        "1",
    ]);
    fs::remove_file(path).ok();

    assert_eq!(output.status.code(), Some(3));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("model: request-grant-file"));
    assert!(output_stdout.contains("backend: RESPONSE"));
    assert!(output_stdout.contains("temporal: INCONCLUSIVE"));
}

#[test]
fn temporal_cli_rejects_model_limit_flags_in_the_product_limit_surface() {
    let wrong_namespace = run(&["temporal", "request-grant", "--max-states", "1"]);
    assert_eq!(wrong_namespace.status.code(), Some(2));
    assert!(stderr(&wrong_namespace).contains("unknown option '--max-states'"));

    let malformed = run(&["temporal", "pulses", "--max-product-depth"]);
    assert_eq!(malformed.status.code(), Some(2));
    assert!(
        stderr(&malformed).contains("option '--max-product-depth' requires an integer value")
    );
}

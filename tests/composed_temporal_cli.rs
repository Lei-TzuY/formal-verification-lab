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

#[test]
fn monitor_and_buchi_cli_report_model_stage_cutoffs() {
    let monitor = run(&["monitor", "session-ok", "--max-model-states", "1"]);
    assert_eq!(monitor.status.code(), Some(3));
    let monitor_stdout = stdout(&monitor);
    assert!(monitor_stdout.contains("monitor verification: INCONCLUSIVE"));
    assert!(monitor_stdout.contains("analysis inconclusive stage: model"));
    assert!(monitor_stdout.contains("model completion: INCONCLUSIVE"));
    assert!(monitor_stdout.contains("model inconclusive reason: state limit reached (max 1)"));
    assert!(monitor_stdout.contains("counterexample: none (analysis incomplete)"));

    let buchi = run(&["buchi", "pulses", "--max-model-states", "1"]);
    assert_eq!(buchi.status.code(), Some(3));
    let buchi_stdout = stdout(&buchi);
    assert!(buchi_stdout.contains("Buchi verification: INCONCLUSIVE"));
    assert!(buchi_stdout.contains("analysis inconclusive stage: model"));
    assert!(buchi_stdout.contains("model completion: INCONCLUSIVE"));
    assert!(buchi_stdout.contains("model inconclusive reason: state limit reached (max 1)"));
    assert!(buchi_stdout.contains("counterexample: none (analysis incomplete)"));
}

#[test]
fn fixed_and_textual_temporal_cli_route_model_limits_through_staged_backends() {
    let response = run(&["temporal", "request-grant", "--max-model-states", "1"]);
    assert_eq!(response.status.code(), Some(3));
    let response_stdout = stdout(&response);
    assert!(response_stdout.contains("backend: RESPONSE"));
    assert!(response_stdout.contains("temporal: INCONCLUSIVE"));
    assert!(response_stdout.contains("analysis inconclusive stage: model"));
    assert!(response_stdout.contains("model completion: INCONCLUSIVE"));

    let recurring = run(&[
        "temporal",
        "check",
        "pulses",
        "infinitely-often(\"pulse-a\",\"pulse-b\")",
        "--max-model-states",
        "1",
    ]);
    assert_eq!(recurring.status.code(), Some(3));
    let recurring_stdout = stdout(&recurring);
    assert!(recurring_stdout.contains("backend: BUCHI"));
    assert!(recurring_stdout.contains("temporal: INCONCLUSIVE"));
    assert!(recurring_stdout.contains("analysis inconclusive stage: model"));
}

#[test]
fn declarative_temporal_file_accepts_staged_model_limits() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fvlab-m29-{nonce}.fvl"));
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
        "--max-model-states",
        "1",
    ]);
    fs::remove_file(path).ok();

    assert_eq!(output.status.code(), Some(3));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("model: request-grant-file"));
    assert!(output_stdout.contains("backend: RESPONSE"));
    assert!(output_stdout.contains("temporal: INCONCLUSIVE"));
    assert!(output_stdout.contains("analysis inconclusive stage: model"));
}

#[test]
fn product_only_temporal_cli_keeps_the_existing_report_contract() {
    let output = run(&[
        "temporal",
        "request-grant",
        "--max-product-states",
        "1",
    ]);
    assert_eq!(output.status.code(), Some(3));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("product inconclusive reason: state limit reached (max 1)"));
    assert!(output_stdout.contains("counterexample: none (product exploration incomplete)"));
    assert!(!output_stdout.contains("analysis inconclusive stage:"));
}

#[test]
fn response_cycle_remains_conclusive_before_a_later_model_transition_cutoff() {
    let output = run(&[
        "temporal",
        "request-grant-unfair",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(output.status.code(), Some(10));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("backend: RESPONSE"));
    assert!(output_stdout.contains("temporal: VIOLATED"));
    assert!(output_stdout.contains("model completion: INCONCLUSIVE"));
    assert!(output_stdout.contains("model inconclusive reason: transition limit reached (max 2)"));
    assert!(output_stdout.contains("counterexample: INFINITE"));
    assert!(output_stdout.contains("obligation: response"));
    assert!(output_stdout.contains("--wait-->"));
}

#[test]
fn buchi_cycle_remains_conclusive_before_a_later_model_transition_cutoff() {
    let output = run(&[
        "buchi",
        "pulses-unfair",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(output.status.code(), Some(9));
    let output_stdout = stdout(&output);
    assert!(output_stdout.contains("Buchi verification: VIOLATED"));
    assert!(output_stdout.contains("model completion: INCONCLUSIVE"));
    assert!(output_stdout.contains("model inconclusive reason: transition limit reached (max 2)"));
    assert!(output_stdout.contains("counterexample: ACCEPTANCE_AVOIDING_CYCLE"));
    assert!(output_stdout.contains("avoided acceptance set: pulse-b-observed"));
    assert!(output_stdout.contains("--pulse-a-->"));
}

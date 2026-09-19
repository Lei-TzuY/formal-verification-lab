use formal_verification_lab::{
    run_structural_suite_expectations_json, run_structural_suite_json,
    STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const ACYCLIC: &str = r#"model "acyclic-cli"
state "a"
state "b"
initial "a"
edge "a" "step" "b"
"#;

const CYCLIC: &str = r#"model "cycle-cli"
state "a"
initial "a"
edge "a" "loop" "a"
"#;

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("fvlab-m67-cli-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(root.join("jobs")).unwrap();
    root
}

fn write_job(root: &Path, name: &str, model: &str) {
    let jobs = root.join("jobs");
    fs::write(jobs.join(format!("{name}.fvl")), model).unwrap();
    fs::write(
        jobs.join(format!("{name}.fvstruct")),
        format!("analysis \"recurrence\"\nmodel \"{name}.fvl\""),
    )
    .unwrap();
}

fn run_cli(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-structural-suite"))
        .args(args)
        .output()
        .expect("fvlab-structural-suite should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

#[test]
fn raw_cli_matches_library_runner_for_manifest_relative_jobs() {
    let root = fixture_dir("raw");
    write_job(&root, "cycle", CYCLIC);
    write_job(&root, "acyclic", ACYCLIC);
    let suite = root.join("suite.fvss");
    fs::write(
        &suite,
        "suite \"raw\"\njob \"jobs/cycle.fvstruct\"\njob \"jobs/acyclic.fvstruct\"\n",
    )
    .unwrap();

    let direct = run_structural_suite_json(&suite);
    let output = run_cli(&[
        suite.display().to_string(),
        "--format".into(),
        "json".into(),
    ]);

    assert_eq!(output.status.code(), Some(direct.exit_code as i32));
    assert_eq!(stdout(&output).trim_end(), direct.to_json());
    assert!(direct.to_json().contains("\"outcome\":\"complete\""));
    assert!(direct.to_json().contains("\"outcome\":\"cycle_found\""));
    assert!(direct.to_json().contains("\"outcome\":\"acyclic\""));

    fs::remove_dir_all(root).ok();
}

#[test]
fn expectation_cli_matches_library_runner_for_match_and_mismatch() {
    let root = fixture_dir("expect");
    write_job(&root, "cycle", CYCLIC);
    write_job(&root, "acyclic", ACYCLIC);

    let matched_path = root.join("matched.fvss");
    fs::write(
        &matched_path,
        "suite \"matched\"\n\
job \"jobs/cycle.fvstruct\" expect \"cycle_found\"\n\
job \"jobs/acyclic.fvstruct\" expect \"acyclic\"\n",
    )
    .unwrap();
    let matched_direct = run_structural_suite_expectations_json(&matched_path);
    let matched = run_cli(&[
        matched_path.display().to_string(),
        "--check-expectations".into(),
        "--format".into(),
        "json".into(),
    ]);
    assert_eq!(matched.status.code(), Some(0));
    assert_eq!(stdout(&matched).trim_end(), matched_direct.to_json());
    assert!(matched_direct.to_json().contains("\"outcome\":\"matched\""));

    let mismatch_path = root.join("mismatch.fvss");
    fs::write(
        &mismatch_path,
        "suite \"mismatch\"\njob \"jobs/cycle.fvstruct\" expect \"acyclic\"\n",
    )
    .unwrap();
    let mismatch_direct = run_structural_suite_expectations_json(&mismatch_path);
    let mismatch = run_cli(&[
        mismatch_path.display().to_string(),
        "--check-expectations".into(),
        "--format".into(),
        "json".into(),
    ]);
    assert_eq!(
        mismatch.status.code(),
        Some(STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE as i32)
    );
    assert_eq!(stdout(&mismatch).trim_end(), mismatch_direct.to_json());
    assert!(mismatch_direct
        .to_json()
        .contains("\"outcome\":\"mismatched\""));

    fs::remove_dir_all(root).ok();
}

#[test]
fn malformed_suite_and_unsupported_format_fail_closed() {
    let root = fixture_dir("errors");
    let suite = root.join("bad.fvss");
    fs::write(&suite, "suite \"bad\"\n").unwrap();

    let malformed = run_cli(&[
        suite.display().to_string(),
        "--format".into(),
        "json".into(),
    ]);
    assert_eq!(malformed.status.code(), Some(2));
    assert!(stdout(&malformed).contains("\"outcome\":\"error\""));
    assert!(stdout(&malformed).contains("requires at least one job"));

    let format = run_cli(&[
        suite.display().to_string(),
        "--format".into(),
        "yaml".into(),
    ]);
    assert_eq!(format.status.code(), Some(2));
    assert!(stdout(&format).is_empty());
    assert!(stderr(&format).contains("expected json"));

    let usage = run_cli(&[]);
    assert_eq!(usage.status.code(), Some(2));
    assert!(stderr(&usage).contains("usage: fvlab-structural-suite"));

    fs::remove_dir_all(root).ok();
}

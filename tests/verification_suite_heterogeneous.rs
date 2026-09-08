use formal_verification_lab::{
    run_verification_job_json, run_verification_suite_expectations_json,
    run_verification_suite_json, VerificationJobOutcome, VerificationRegressionSuiteOutcome,
    VerificationSuiteOutcome, VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m59-mixed-suite-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn temporal_model() -> &'static str {
    "model \"temporal-ok\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn safety_violated_model() -> &'static str {
    "model \"safety-bad\"\nstate \"start\"\nstate \"bad\"\ninitial \"start\"\nedge \"start\" \"break\" \"bad\"\nlabel \"start\" \"ok\"\n"
}

fn safety_safe_model() -> &'static str {
    "model \"safety-safe\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"finish\" \"done\"\nlabel \"start\" \"ok\"\nlabel \"done\" \"ok\"\n"
}

fn write_temporal_job(root: &Path) -> PathBuf {
    fs::write(root.join("temporal.fvl"), temporal_model()).unwrap();
    fs::write(
        root.join("temporal.fvt"),
        "response(\"request\",\"request\",\"grant\")\n",
    )
    .unwrap();
    let manifest = root.join("temporal.fvj");
    fs::write(
        &manifest,
        "model \"temporal.fvl\"\nproperty \"temporal.fvt\"\n",
    )
    .unwrap();
    manifest
}

fn write_safety_job(root: &Path, name: &str, model: &str, tail: &str) -> PathBuf {
    fs::write(root.join(format!("{name}.fvl")), model).unwrap();
    fs::write(root.join(format!("{name}.fvp")), "\"ok\"\n").unwrap();
    let manifest = root.join(format!("{name}.fvj"));
    fs::write(
        &manifest,
        format!("analysis \"safety\"\nmodel \"{name}.fvl\"\nproperty \"{name}.fvp\"\n{tail}"),
    )
    .unwrap();
    manifest
}

fn run_suite(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-suite"))
        .args(args)
        .output()
        .expect("fvlab-suite binary should execute")
}

fn run_job(manifest: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args([
            "temporal",
            "job",
            manifest.to_str().expect("fixture path should be UTF-8"),
            "--format",
            "json",
        ])
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn raw_args(suite: &Path) -> Vec<String> {
    vec![
        suite.to_string_lossy().into_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ]
}

fn expectation_args(suite: &Path) -> Vec<String> {
    vec![
        suite.to_string_lossy().into_owned(),
        "--check-expectations".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ]
}

#[test]
fn mixed_family_suite_preserves_each_direct_envelope_and_raw_precedence() {
    let root = fixture_dir("raw");
    let temporal = write_temporal_job(&root);
    let safety_bad = write_safety_job(&root, "safety-bad", safety_violated_model(), "");
    let safety_cutoff = write_safety_job(
        &root,
        "safety-cutoff",
        safety_safe_model(),
        "max-model-transitions 0\n",
    );
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"mixed-raw\"\njob \"temporal.fvj\"\njob \"safety-bad.fvj\"\njob \"safety-cutoff.fvj\"\n",
    )
    .unwrap();

    let run = run_verification_suite_json(&suite);
    assert_eq!(run.exit_code, 7);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Violated);
    assert_eq!(run.envelope.jobs.len(), 3);

    let direct = [temporal, safety_bad, safety_cutoff]
        .iter()
        .map(run_verification_job_json)
        .collect::<Vec<_>>();
    for (entry, expected) in run.envelope.jobs.iter().zip(direct.iter()) {
        assert_eq!(entry.result, expected.envelope);
    }

    assert_eq!(
        run.envelope.jobs[0].result.schema_version,
        VERIFICATION_JOB_RESULT_SCHEMA_VERSION
    );
    assert_eq!(run.envelope.jobs[0].result.analysis, None);
    assert_eq!(
        run.envelope.jobs[0].result.outcome,
        VerificationJobOutcome::Satisfied
    );

    assert_eq!(
        run.envelope.jobs[1].result.schema_version,
        VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.jobs[1].result.analysis.as_deref(),
        Some("safety")
    );
    assert_eq!(
        run.envelope.jobs[1].result.outcome,
        VerificationJobOutcome::Violated
    );
    assert_eq!(
        run.envelope.jobs[2].result.outcome,
        VerificationJobOutcome::Inconclusive
    );
    assert_eq!(
        run.to_json(),
        run_verification_suite_json(&suite).to_json(),
        "heterogeneous suite JSON must remain deterministic"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn mixed_family_expectations_match_and_mismatch_without_reinterpreting_job_results() {
    let root = fixture_dir("expectations");
    write_temporal_job(&root);
    write_safety_job(&root, "safety-bad", safety_violated_model(), "");
    write_safety_job(
        &root,
        "safety-cutoff",
        safety_safe_model(),
        "max-model-transitions 0\n",
    );

    let matched_suite = root.join("matched.fvs");
    fs::write(
        &matched_suite,
        "suite \"mixed-matched\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety-bad.fvj\" expect \"violated\"\njob \"safety-cutoff.fvj\" expect \"inconclusive\"\n",
    )
    .unwrap();
    let matched = run_verification_suite_expectations_json(&matched_suite);
    assert_eq!(matched.exit_code, 0);
    assert_eq!(
        matched.envelope.outcome,
        VerificationRegressionSuiteOutcome::Matched
    );
    assert!(matched.envelope.jobs.iter().all(|entry| entry.matched));
    assert_eq!(
        matched.envelope.jobs[1].result.analysis.as_deref(),
        Some("safety")
    );
    assert_eq!(matched.envelope.jobs[1].result.schema_version, 2);

    let mismatch_suite = root.join("mismatch.fvs");
    fs::write(
        &mismatch_suite,
        "suite \"mixed-mismatch\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety-bad.fvj\" expect \"satisfied\"\njob \"safety-cutoff.fvj\" expect \"inconclusive\"\n",
    )
    .unwrap();
    let mismatch = run_verification_suite_expectations_json(&mismatch_suite);
    assert_eq!(mismatch.exit_code, 13);
    assert_eq!(
        mismatch.envelope.outcome,
        VerificationRegressionSuiteOutcome::Mismatched
    );
    assert!(mismatch.envelope.jobs[0].matched);
    assert!(!mismatch.envelope.jobs[1].matched);
    assert!(mismatch.envelope.jobs[2].matched);
    assert_eq!(
        mismatch.envelope.jobs[1].observed,
        VerificationJobOutcome::Violated
    );
    assert_eq!(
        mismatch.envelope.jobs[1].result.analysis.as_deref(),
        Some("safety")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn built_binaries_preserve_heterogeneous_direct_job_envelopes_inside_suites() {
    let root = fixture_dir("cli");
    let temporal = write_temporal_job(&root);
    let safety_bad = write_safety_job(&root, "safety-bad", safety_violated_model(), "");
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"mixed-cli\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety-bad.fvj\" expect \"violated\"\n",
    )
    .unwrap();

    let temporal_direct = run_job(&temporal);
    assert_eq!(temporal_direct.status.code(), Some(0));
    assert!(stderr(&temporal_direct).is_empty());
    let temporal_json = stdout(&temporal_direct).trim_end().to_owned();
    assert!(temporal_json.starts_with("{\"schema_version\":1,\"outcome\":\"satisfied\""));

    let safety_direct = run_job(&safety_bad);
    assert_eq!(safety_direct.status.code(), Some(12));
    assert!(stderr(&safety_direct).is_empty());
    let safety_json = stdout(&safety_direct).trim_end().to_owned();
    assert!(safety_json
        .starts_with("{\"schema_version\":2,\"analysis\":\"safety\",\"outcome\":\"violated\""));
    assert!(safety_json.contains("\"kind\":\"safety\""));
    assert!(!safety_json.contains("\"pending\""));

    let raw = run_suite(&raw_args(&suite));
    assert_eq!(raw.status.code(), Some(7));
    assert!(stderr(&raw).is_empty());
    let raw_json = stdout(&raw);
    assert!(raw_json.contains(&format!(
        "{{\"manifest\":\"temporal.fvj\",\"result\":{temporal_json}}}"
    )));
    assert!(raw_json.contains(&format!(
        "{{\"manifest\":\"safety-bad.fvj\",\"result\":{safety_json}}}"
    )));

    let checked = run_suite(&expectation_args(&suite));
    assert_eq!(checked.status.code(), Some(0));
    assert!(stderr(&checked).is_empty());
    let checked_json = stdout(&checked);
    assert!(checked_json.contains(&format!(
        "\"manifest\":\"temporal.fvj\",\"expected\":\"satisfied\",\"observed\":\"satisfied\",\"matched\":true,\"result\":{temporal_json}"
    )));
    assert!(checked_json.contains(&format!(
        "\"manifest\":\"safety-bad.fvj\",\"expected\":\"violated\",\"observed\":\"violated\",\"matched\":true,\"result\":{safety_json}"
    )));

    let _ = fs::remove_dir_all(root);
}

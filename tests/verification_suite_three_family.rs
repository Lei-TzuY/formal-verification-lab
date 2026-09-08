use formal_verification_lab::{
    run_verification_job_json, run_verification_suite_expectations_json,
    run_verification_suite_json, VerificationJobOutcome, VerificationRegressionSuiteOutcome,
    VerificationSuiteOutcome, VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_RESULT_SCHEMA_VERSION, VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m60-three-family-suite-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_temporal_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("temporal.fvl"),
        "model \"temporal-ok\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n",
    )
    .unwrap();
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

fn write_safety_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("safety.fvl"),
        "model \"safety-bad\"\nstate \"start\"\nstate \"bad\"\ninitial \"start\"\nedge \"start\" \"break\" \"bad\"\nlabel \"start\" \"ok\"\n",
    )
    .unwrap();
    fs::write(root.join("safety.fvp"), "\"ok\"\n").unwrap();
    let manifest = root.join("safety.fvj");
    fs::write(
        &manifest,
        "analysis \"safety\"\nmodel \"safety.fvl\"\nproperty \"safety.fvp\"\n",
    )
    .unwrap();
    manifest
}

fn write_exact_state_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("state.fvl"),
        "model \"state-chain\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"finish\" \"done\"\n",
    )
    .unwrap();
    fs::write(root.join("state.fvp"), "reachable(\"done\")\n").unwrap();
    let manifest = root.join("state.fvj");
    fs::write(
        &manifest,
        "analysis \"exact-state\"\nmodel \"state.fvl\"\nproperty \"state.fvp\"\nmax-model-transitions 0\n",
    )
    .unwrap();
    manifest
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

fn run_suite(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-suite"))
        .args(args)
        .output()
        .expect("fvlab-suite binary should execute")
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
fn raw_three_family_suite_preserves_direct_envelopes_and_aggregate_precedence() {
    let root = fixture_dir("raw");
    let temporal = write_temporal_job(&root);
    let safety = write_safety_job(&root);
    let exact_state = write_exact_state_job(&root);
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"three-family-raw\"\njob \"temporal.fvj\"\njob \"safety.fvj\"\njob \"state.fvj\"\n",
    )
    .unwrap();

    let run = run_verification_suite_json(&suite);
    assert_eq!(run.exit_code, 7);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Violated);
    assert_eq!(run.envelope.jobs.len(), 3);

    let direct = [temporal, safety, exact_state]
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
        run.envelope.jobs[2].result.schema_version,
        VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.jobs[2].result.analysis.as_deref(),
        Some("exact-state")
    );
    assert_eq!(
        run.envelope.jobs[2].result.outcome,
        VerificationJobOutcome::Inconclusive
    );
    assert_eq!(run.envelope.jobs[2].result.accounting.product_states, None);
    assert!(run.envelope.jobs[2].result.weak_fair_actions.is_empty());
    assert!(run.envelope.jobs[2].result.strong_fair_actions.is_empty());

    assert_eq!(
        run.to_json(),
        run_verification_suite_json(&suite).to_json(),
        "three-family suite JSON must remain deterministic"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn three_family_expectations_match_and_exact_state_mismatch_without_reinterpretation() {
    let root = fixture_dir("expectations");
    write_temporal_job(&root);
    write_safety_job(&root);
    write_exact_state_job(&root);

    let matched_suite = root.join("matched.fvs");
    fs::write(
        &matched_suite,
        "suite \"three-family-matched\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"inconclusive\"\n",
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
        matched.envelope.jobs[2].result.analysis.as_deref(),
        Some("exact-state")
    );
    assert_eq!(
        matched.envelope.jobs[2].observed,
        VerificationJobOutcome::Inconclusive
    );

    let mismatch_suite = root.join("mismatch.fvs");
    fs::write(
        &mismatch_suite,
        "suite \"three-family-mismatch\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();
    let mismatch = run_verification_suite_expectations_json(&mismatch_suite);
    assert_eq!(mismatch.exit_code, 13);
    assert_eq!(
        mismatch.envelope.outcome,
        VerificationRegressionSuiteOutcome::Mismatched
    );
    assert!(mismatch.envelope.jobs[0].matched);
    assert!(mismatch.envelope.jobs[1].matched);
    assert!(!mismatch.envelope.jobs[2].matched);
    assert_eq!(
        mismatch.envelope.jobs[2].observed,
        VerificationJobOutcome::Inconclusive
    );
    assert_eq!(
        mismatch.envelope.jobs[2].result.analysis.as_deref(),
        Some("exact-state")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn built_binaries_preserve_all_three_direct_job_envelopes_inside_suites() {
    let root = fixture_dir("cli");
    let temporal = write_temporal_job(&root);
    let safety = write_safety_job(&root);
    let exact_state = write_exact_state_job(&root);
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"three-family-cli\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"inconclusive\"\n",
    )
    .unwrap();

    let temporal_direct = run_job(&temporal);
    assert_eq!(temporal_direct.status.code(), Some(0));
    assert!(stderr(&temporal_direct).is_empty());
    let temporal_json = stdout(&temporal_direct).trim_end().to_owned();
    assert!(temporal_json.starts_with("{\"schema_version\":1,\"outcome\":\"satisfied\""));

    let safety_direct = run_job(&safety);
    assert_eq!(safety_direct.status.code(), Some(12));
    assert!(stderr(&safety_direct).is_empty());
    let safety_json = stdout(&safety_direct).trim_end().to_owned();
    assert!(safety_json
        .starts_with("{\"schema_version\":2,\"analysis\":\"safety\",\"outcome\":\"violated\""));

    let exact_state_direct = run_job(&exact_state);
    assert_eq!(exact_state_direct.status.code(), Some(3));
    assert!(stderr(&exact_state_direct).is_empty());
    let exact_state_json = stdout(&exact_state_direct).trim_end().to_owned();
    assert!(exact_state_json.starts_with(
        "{\"schema_version\":2,\"analysis\":\"exact-state\",\"outcome\":\"inconclusive\""
    ));
    assert!(exact_state_json
        .contains("\"cutoff\":{\"stage\":\"model\",\"kind\":\"transition_limit\",\"limit\":0}"));
    assert!(exact_state_json.contains("\"product_states\":null"));
    assert!(!exact_state_json.contains("\"pending\""));

    let raw = run_suite(&raw_args(&suite));
    assert_eq!(raw.status.code(), Some(7));
    assert!(stderr(&raw).is_empty());
    let raw_json = stdout(&raw);
    assert!(raw_json.contains(&format!(
        "{{\"manifest\":\"temporal.fvj\",\"result\":{temporal_json}}}"
    )));
    assert!(raw_json.contains(&format!(
        "{{\"manifest\":\"safety.fvj\",\"result\":{safety_json}}}"
    )));
    assert!(raw_json.contains(&format!(
        "{{\"manifest\":\"state.fvj\",\"result\":{exact_state_json}}}"
    )));

    let checked = run_suite(&expectation_args(&suite));
    assert_eq!(checked.status.code(), Some(0));
    assert!(stderr(&checked).is_empty());
    let checked_json = stdout(&checked);
    assert!(checked_json.contains(&format!(
        "\"manifest\":\"temporal.fvj\",\"expected\":\"satisfied\",\"observed\":\"satisfied\",\"matched\":true,\"result\":{temporal_json}"
    )));
    assert!(checked_json.contains(&format!(
        "\"manifest\":\"safety.fvj\",\"expected\":\"violated\",\"observed\":\"violated\",\"matched\":true,\"result\":{safety_json}"
    )));
    assert!(checked_json.contains(&format!(
        "\"manifest\":\"state.fvj\",\"expected\":\"inconclusive\",\"observed\":\"inconclusive\",\"matched\":true,\"result\":{exact_state_json}"
    )));

    let _ = fs::remove_dir_all(root);
}

use formal_verification_lab::{
    run_verification_job_json, run_verification_suite_expectations_json,
    run_verification_suite_json, VerificationJobOutcome, VerificationRegressionSuiteOutcome,
    VerificationSuiteOutcome, VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION, VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
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
        "fvlab-m61-four-family-suite-{kind}-{}-{id}",
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

fn write_proposition_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("proposition.fvl"),
        "model \"proposition-chain\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"finish\" \"done\"\nlabel \"start\" \"entry\"\nlabel \"done\" \"complete\"\n",
    )
    .unwrap();
    fs::write(root.join("proposition.fvp"), "reachable \"complete\"\n").unwrap();
    let manifest = root.join("proposition.fvj");
    fs::write(
        &manifest,
        "analysis \"proposition-expression\"\nmodel \"proposition.fvl\"\nproperty \"proposition.fvp\"\n",
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
fn raw_four_family_suite_preserves_direct_envelopes_order_and_precedence() {
    let root = fixture_dir("raw");
    let temporal = write_temporal_job(&root);
    let safety = write_safety_job(&root);
    let exact_state = write_exact_state_job(&root);
    let proposition = write_proposition_job(&root);
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"four-family-raw\"\njob \"temporal.fvj\"\njob \"safety.fvj\"\njob \"state.fvj\"\njob \"proposition.fvj\"\n",
    )
    .unwrap();

    let run = run_verification_suite_json(&suite);
    assert_eq!(run.exit_code, 7);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Violated);
    assert_eq!(run.envelope.jobs.len(), 4);

    let direct = [temporal, safety, exact_state, proposition]
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
        run.envelope.jobs[1].result.schema_version,
        VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.jobs[1].result.analysis.as_deref(),
        Some("safety")
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
    assert_eq!(
        run.envelope.jobs[3].result.schema_version,
        VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.jobs[3].result.analysis.as_deref(),
        Some("proposition-expression")
    );
    assert_eq!(
        run.envelope.jobs[3].result.outcome,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(run.envelope.jobs[3].result.accounting.product_states, None);
    assert!(run.envelope.jobs[3].result.weak_fair_actions.is_empty());
    assert!(run.envelope.jobs[3].result.strong_fair_actions.is_empty());

    assert_eq!(
        run.to_json(),
        run_verification_suite_json(&suite).to_json(),
        "four-family raw suite output must remain deterministic"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn four_family_expectations_match_and_proposition_mismatch_without_reinterpretation() {
    let root = fixture_dir("expectations");
    write_temporal_job(&root);
    write_safety_job(&root);
    write_exact_state_job(&root);
    write_proposition_job(&root);

    let matched_suite = root.join("matched.fvs");
    fs::write(
        &matched_suite,
        "suite \"four-family-matched\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"inconclusive\"\njob \"proposition.fvj\" expect \"satisfied\"\n",
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
        matched.envelope.jobs[3].result.analysis.as_deref(),
        Some("proposition-expression")
    );

    let mismatch_suite = root.join("mismatch.fvs");
    fs::write(
        &mismatch_suite,
        "suite \"four-family-mismatch\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"inconclusive\"\njob \"proposition.fvj\" expect \"violated\"\n",
    )
    .unwrap();
    let mismatch = run_verification_suite_expectations_json(&mismatch_suite);
    assert_eq!(mismatch.exit_code, 13);
    assert_eq!(
        mismatch.envelope.outcome,
        VerificationRegressionSuiteOutcome::Mismatched
    );
    assert!(mismatch.envelope.jobs[..3]
        .iter()
        .all(|entry| entry.matched));
    assert!(!mismatch.envelope.jobs[3].matched);
    assert_eq!(
        mismatch.envelope.jobs[3].observed,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        mismatch.envelope.jobs[3].result.analysis.as_deref(),
        Some("proposition-expression")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn built_binaries_preserve_all_four_direct_envelopes_inside_raw_and_expectation_suites() {
    let root = fixture_dir("cli");
    let temporal = write_temporal_job(&root);
    let safety = write_safety_job(&root);
    let exact_state = write_exact_state_job(&root);
    let proposition = write_proposition_job(&root);
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"four-family-cli\"\njob \"temporal.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"inconclusive\"\njob \"proposition.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let direct = [
        ("temporal.fvj", temporal, 0),
        ("safety.fvj", safety, 12),
        ("state.fvj", exact_state, 3),
        ("proposition.fvj", proposition, 0),
    ]
    .into_iter()
    .map(|(name, manifest, code)| {
        let output = run_job(&manifest);
        assert_eq!(output.status.code(), Some(code));
        assert!(stderr(&output).is_empty());
        (name, stdout(&output).trim_end().to_owned())
    })
    .collect::<Vec<_>>();

    assert!(direct[0]
        .1
        .starts_with("{\"schema_version\":1,\"outcome\":\"satisfied\""));
    assert!(direct[1]
        .1
        .starts_with("{\"schema_version\":2,\"analysis\":\"safety\",\"outcome\":\"violated\""));
    assert!(direct[2].1.starts_with(
        "{\"schema_version\":2,\"analysis\":\"exact-state\",\"outcome\":\"inconclusive\""
    ));
    assert!(direct[3].1.starts_with(
        "{\"schema_version\":2,\"analysis\":\"proposition-expression\",\"outcome\":\"satisfied\""
    ));
    assert!(direct[3].1.contains("\"kind\":\"reachability_witness\""));
    assert!(direct[3].1.contains("\"product_states\":null"));
    assert!(!direct[3].1.contains("\"pending\""));

    let raw = run_suite(&raw_args(&suite));
    assert_eq!(raw.status.code(), Some(7));
    assert!(stderr(&raw).is_empty());
    let raw_json = stdout(&raw);
    for (manifest, json) in &direct {
        assert!(raw_json.contains(&format!(
            "{{\"manifest\":\"{manifest}\",\"result\":{json}}}"
        )));
    }

    let checked = run_suite(&expectation_args(&suite));
    assert_eq!(checked.status.code(), Some(0));
    assert!(stderr(&checked).is_empty());
    let checked_json = stdout(&checked);
    for ((manifest, json), expected) in
        direct
            .iter()
            .zip(["satisfied", "violated", "inconclusive", "satisfied"])
    {
        assert!(checked_json.contains(&format!(
            "\"manifest\":\"{manifest}\",\"expected\":\"{expected}\",\"observed\":\"{expected}\",\"matched\":true,\"result\":{json}"
        )));
    }

    let _ = fs::remove_dir_all(root);
}

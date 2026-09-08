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
        "fvlab-m62-five-family-suite-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_legacy_multi_response_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("legacy.fvl"),
        "model \"legacy-ok\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n",
    )
    .unwrap();
    fs::write(
        root.join("legacy.fvt"),
        "response(\"request\",\"request\",\"grant\")\n",
    )
    .unwrap();
    let manifest = root.join("legacy.fvj");
    fs::write(&manifest, "model \"legacy.fvl\"\nproperty \"legacy.fvt\"\n").unwrap();
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

fn write_action_temporal_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("action.fvl"),
        "model \"action-temporal-ok\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n",
    )
    .unwrap();
    fs::write(root.join("action.fvp"), "response(\"request\",\"grant\")\n").unwrap();
    let manifest = root.join("action.fvj");
    fs::write(
        &manifest,
        "analysis \"action-temporal\"\nmodel \"action.fvl\"\nproperty \"action.fvp\"\n",
    )
    .unwrap();
    manifest
}

fn write_suite(
    root: &Path,
    name: &str,
    with_expectations: bool,
    action_expectation: &str,
) -> PathBuf {
    let suite = root.join(format!("{name}.fvs"));
    let body = if with_expectations {
        format!(
            "suite \"{name}\"\njob \"legacy.fvj\" expect \"satisfied\"\njob \"safety.fvj\" expect \"violated\"\njob \"state.fvj\" expect \"inconclusive\"\njob \"proposition.fvj\" expect \"satisfied\"\njob \"action.fvj\" expect \"{action_expectation}\"\n"
        )
    } else {
        format!(
            "suite \"{name}\"\njob \"legacy.fvj\"\njob \"safety.fvj\"\njob \"state.fvj\"\njob \"proposition.fvj\"\njob \"action.fvj\"\n"
        )
    };
    fs::write(&suite, body).unwrap();
    suite
}

fn create_jobs(root: &Path) -> [PathBuf; 5] {
    [
        write_legacy_multi_response_job(root),
        write_safety_job(root),
        write_exact_state_job(root),
        write_proposition_job(root),
        write_action_temporal_job(root),
    ]
}

fn run_job_binary(manifest: &Path) -> Output {
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

fn run_suite_binary(suite: &Path, expectations: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fvlab-suite"));
    command.arg(suite);
    if expectations {
        command.arg("--check-expectations");
    }
    command
        .args(["--format", "json"])
        .output()
        .expect("fvlab-suite binary should execute")
}

#[test]
fn raw_five_family_suite_preserves_direct_envelopes_order_and_aggregate_precedence() {
    let root = fixture_dir("raw");
    let jobs = create_jobs(&root);
    let suite = write_suite(&root, "five-family-raw", false, "satisfied");

    let run = run_verification_suite_json(&suite);
    assert_eq!(run.exit_code, 7);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Violated);
    assert_eq!(run.envelope.jobs.len(), 5);

    let direct = jobs
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
        run.envelope.jobs[2].result.schema_version,
        VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION
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
        run.envelope.jobs[4].result.schema_version,
        VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.jobs[4].result.analysis.as_deref(),
        Some("action-temporal")
    );
    assert_eq!(
        run.envelope.jobs[4].result.backend.as_deref(),
        Some("response")
    );
    assert_eq!(
        run.to_json(),
        run_verification_suite_json(&suite).to_json(),
        "five-family raw suite output must remain deterministic"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn five_family_expectations_match_and_action_temporal_mismatch_is_not_reinterpreted() {
    let root = fixture_dir("expectations");
    create_jobs(&root);

    let matched_suite = write_suite(&root, "five-family-matched", true, "satisfied");
    let matched = run_verification_suite_expectations_json(&matched_suite);
    assert_eq!(matched.exit_code, 0);
    assert_eq!(
        matched.envelope.outcome,
        VerificationRegressionSuiteOutcome::Matched
    );
    assert_eq!(matched.envelope.jobs.len(), 5);
    assert!(matched.envelope.jobs.iter().all(|entry| entry.matched));
    assert_eq!(
        matched.envelope.jobs[4].result.analysis.as_deref(),
        Some("action-temporal")
    );

    let mismatch_suite = write_suite(&root, "five-family-mismatch", true, "violated");
    let mismatch = run_verification_suite_expectations_json(&mismatch_suite);
    assert_eq!(mismatch.exit_code, 13);
    assert_eq!(
        mismatch.envelope.outcome,
        VerificationRegressionSuiteOutcome::Mismatched
    );
    assert!(mismatch.envelope.jobs[..4]
        .iter()
        .all(|entry| entry.matched));
    assert!(!mismatch.envelope.jobs[4].matched);
    assert_eq!(
        mismatch.envelope.jobs[4].observed,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        mismatch.envelope.jobs[4].result.analysis.as_deref(),
        Some("action-temporal")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn built_binaries_preserve_all_five_direct_envelopes_in_raw_and_expectation_suites() {
    let root = fixture_dir("cli");
    let jobs = create_jobs(&root);
    let suite = write_suite(&root, "five-family-cli", true, "satisfied");
    let names = [
        "legacy.fvj",
        "safety.fvj",
        "state.fvj",
        "proposition.fvj",
        "action.fvj",
    ];
    let codes = [0, 12, 3, 0, 0];

    let direct = jobs
        .iter()
        .zip(names)
        .zip(codes)
        .map(|((manifest, name), code)| {
            let output = run_job_binary(manifest);
            assert_eq!(output.status.code(), Some(code));
            assert!(output.stderr.is_empty());
            (
                name,
                String::from_utf8(output.stdout)
                    .unwrap()
                    .trim_end()
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();

    assert!(direct[4].1.starts_with(
        "{\"schema_version\":2,\"analysis\":\"action-temporal\",\"backend\":\"response\",\"outcome\":\"satisfied\""
    ));

    let raw = run_suite_binary(&suite, false);
    assert_eq!(raw.status.code(), Some(7));
    assert!(raw.stderr.is_empty());
    let raw_json = String::from_utf8(raw.stdout).unwrap();
    for (manifest, json) in &direct {
        assert!(raw_json.contains(&format!(
            "{{\"manifest\":\"{manifest}\",\"result\":{json}}}"
        )));
    }

    let checked = run_suite_binary(&suite, true);
    assert_eq!(checked.status.code(), Some(0));
    assert!(checked.stderr.is_empty());
    let checked_json = String::from_utf8(checked.stdout).unwrap();
    for ((manifest, json), expected) in direct.iter().zip([
        "satisfied",
        "violated",
        "inconclusive",
        "satisfied",
        "satisfied",
    ]) {
        assert!(checked_json.contains(&format!(
            "\"manifest\":\"{manifest}\",\"expected\":\"{expected}\",\"observed\":\"{expected}\",\"matched\":true,\"result\":{json}"
        )));
    }

    let _ = fs::remove_dir_all(root);
}

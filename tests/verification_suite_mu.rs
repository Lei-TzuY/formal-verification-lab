use formal_verification_lab::{
    run_verification_job_json, run_verification_suite_expectations_json,
    run_verification_suite_json, VerificationJobMuTruth, VerificationJobOutcome,
    VerificationRegressionSuiteOutcome, VerificationSuiteOutcome,
    VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MU_MODEL: &str = r#"
model "suite-mu"
state "start"
state "done"
state "loop"
initial "start"
edge "start" "finish" "done"
edge "start" "branch" "loop"
edge "loop" "spin" "loop"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#;

const REACH_COMPLETE: &str = r#"mu X. "complete" or diamond $X"#;

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m80-mu-suite-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_mu_job(root: &Path, name: &str, property: &str, tail: &str) -> PathBuf {
    fs::write(root.join(format!("{name}.fvl")), MU_MODEL).unwrap();
    fs::write(root.join(format!("{name}.mu")), property).unwrap();
    let manifest = root.join(format!("{name}.fvj"));
    fs::write(
        &manifest,
        format!(
            "analysis \"mu-calculus\"\nmodel \"{name}.fvl\"\nproperty \"{name}.mu\"\n{tail}"
        ),
    )
    .unwrap();
    manifest
}

fn write_safety_job(root: &Path) -> PathBuf {
    fs::write(
        root.join("safe.fvl"),
        "model \"safe\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"finish\" \"done\"\nlabel \"start\" \"ok\"\nlabel \"done\" \"ok\"\n",
    )
    .unwrap();
    fs::write(root.join("safe.fvp"), "\"ok\"\n").unwrap();
    let manifest = root.join("safe.fvj");
    fs::write(
        &manifest,
        "analysis \"safety\"\nmodel \"safe.fvl\"\nproperty \"safe.fvp\"\n",
    )
    .unwrap();
    manifest
}

fn run_suite_binary(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-suite"))
        .args(args)
        .output()
        .expect("fvlab-suite binary should execute")
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
fn generic_suite_preserves_schema_v4_mu_envelopes_and_heterogeneous_order() {
    let root = fixture_dir("raw");
    let satisfied = write_mu_job(&root, "mu-satisfied", REACH_COMPLETE, "");
    let inconclusive = write_mu_job(
        &root,
        "mu-cutoff",
        REACH_COMPLETE,
        "max-model-states 1\n",
    );
    let safety = write_safety_job(&root);
    let malformed = write_mu_job(&root, "mu-error", "mu X. (", "");

    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"mu-mixed\"\njob \"mu-satisfied.fvj\"\njob \"mu-cutoff.fvj\"\njob \"safe.fvj\"\njob \"mu-error.fvj\"\n",
    )
    .unwrap();

    let run = run_verification_suite_json(&suite);
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Error);
    assert_eq!(run.envelope.jobs.len(), 4);

    let direct = [satisfied, inconclusive, safety, malformed]
        .iter()
        .map(run_verification_job_json)
        .collect::<Vec<_>>();
    for (entry, direct) in run.envelope.jobs.iter().zip(&direct) {
        assert_eq!(entry.result, direct.envelope);
    }

    assert_eq!(
        run.envelope.jobs[0].result.schema_version,
        VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.jobs[0].result.outcome,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        run.envelope.jobs[1].result.outcome,
        VerificationJobOutcome::Inconclusive
    );
    assert_eq!(
        run.envelope.jobs[2].result.analysis.as_deref(),
        Some("safety")
    );
    assert_eq!(
        run.envelope.jobs[3].result.outcome,
        VerificationJobOutcome::Error
    );

    let mu = run.envelope.jobs[1].result.mu.as_ref().unwrap();
    assert_eq!(mu.initial[0].truth, VerificationJobMuTruth::Unknown);
    let json = run.to_json();
    assert!(json.contains("\"schema_version\":4,\"analysis\":\"mu-calculus\""));
    assert!(json.contains("\"fixpoint_iterations\":"));
    assert!(json.contains("\"kind\":\"state_limit\""));
    assert!(json.contains("\"truth\":\"unknown\""));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn expectation_suite_compares_only_outcomes_and_keeps_full_mu_details() {
    let root = fixture_dir("expectations");
    let satisfied = write_mu_job(&root, "mu-satisfied", REACH_COMPLETE, "");
    let violated = write_mu_job(&root, "mu-violated", r#""complete""#, "");
    let cutoff = write_mu_job(
        &root,
        "mu-cutoff",
        REACH_COMPLETE,
        "max-model-states 1\n",
    );

    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"mu-expectations\"\njob \"mu-satisfied.fvj\" expect \"satisfied\"\njob \"mu-violated.fvj\" expect \"violated\"\njob \"mu-cutoff.fvj\" expect \"inconclusive\"\n",
    )
    .unwrap();

    let run = run_verification_suite_expectations_json(&suite);
    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.outcome,
        VerificationRegressionSuiteOutcome::Matched
    );
    assert!(run.envelope.jobs.iter().all(|entry| entry.matched));

    let direct = [satisfied, violated, cutoff]
        .iter()
        .map(run_verification_job_json)
        .collect::<Vec<_>>();
    for (entry, direct) in run.envelope.jobs.iter().zip(&direct) {
        assert_eq!(entry.result, direct.envelope);
    }

    assert_eq!(
        run.envelope.jobs[0].result.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::True
    );
    assert_eq!(
        run.envelope.jobs[1].result.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::False
    );
    assert_eq!(
        run.envelope.jobs[2].result.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::Unknown
    );
    assert!(run.envelope.jobs[2]
        .result
        .mu
        .as_ref()
        .unwrap()
        .fixpoint_iterations
        > 0);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_suite_raw_and_expectation_json_embed_direct_mu_payloads() {
    let root = fixture_dir("binary");
    let reach = write_mu_job(&root, "mu-reach", REACH_COMPLETE, "");
    let truth = write_mu_job(&root, "mu-true", "true", "");
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"mu-binary\"\njob \"mu-reach.fvj\" expect \"satisfied\"\njob \"mu-true.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let raw = run_suite_binary(&raw_args(&suite));
    assert_eq!(raw.status.code(), Some(0));
    let raw_json = String::from_utf8(raw.stdout).unwrap();
    assert!(raw_json.contains(&run_verification_job_json(&reach).to_json()));
    assert!(raw_json.contains(&run_verification_job_json(&truth).to_json()));
    assert!(raw_json.contains("\"analysis\":\"mu-calculus\""));
    assert!(raw_json.contains("\"mu\":{"));

    let checked = run_suite_binary(&expectation_args(&suite));
    assert_eq!(checked.status.code(), Some(0));
    let checked_json = String::from_utf8(checked.stdout).unwrap();
    assert!(checked_json.contains("\"outcome\":\"matched\""));
    assert!(checked_json.contains("\"expected\":\"satisfied\""));
    assert!(checked_json.contains("\"schema_version\":4"));
    assert!(checked_json.contains("\"mu\":{"));

    fs::remove_dir_all(root).unwrap();
}

use formal_verification_lab::{
    parse_verification_suite, run_verification_job_json, run_verification_suite_expectations_json,
    run_verification_suite_json, VerificationExpectedOutcome, VerificationJobOutcome,
    VerificationRegressionSuiteOutcome, VerificationSuiteParseErrorKind,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("fvlab-m58-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn property_source() -> &'static str {
    "response(\"request\",\"request\",\"grant\")\n"
}

fn satisfied_model_source() -> &'static str {
    "model \"satisfied\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn unfair_model_source() -> &'static str {
    "model \"unfair\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"wait\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn terminal_model_source() -> &'static str {
    "model \"terminal\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\n"
}

fn write_job(root: &Path, name: &str, model_source: &str, tail: &str) -> PathBuf {
    let model = root.join(format!("{name}.fvl"));
    let property = root.join(format!("{name}.fvt"));
    let job = root.join(format!("{name}.fvj"));
    fs::write(&model, model_source).unwrap();
    fs::write(&property, property_source()).unwrap();
    fs::write(
        &job,
        format!("model \"{name}.fvl\"\nproperty \"{name}.fvt\"\n{tail}"),
    )
    .unwrap();
    job
}

#[test]
fn expectation_syntax_is_optional_and_canonical_without_changing_old_documents() {
    let old = parse_verification_suite("suite \"raw\"\njob \"a.fvj\"\n").unwrap();
    assert_eq!(old.canonical_document(), "suite \"raw\"\njob \"a.fvj\"");
    assert_eq!(old.expected_outcomes(), [None]);

    let input = "suite \"regression\"\njob \"a.fvj\" expect \"violated\"\njob \"b.fvj\" expect \"inconclusive\"\n";
    let suite = parse_verification_suite(input).unwrap();
    assert_eq!(suite.job_paths(), ["a.fvj", "b.fvj"]);
    assert_eq!(
        suite.expected_outcomes(),
        [
            Some(VerificationExpectedOutcome::Violated),
            Some(VerificationExpectedOutcome::Inconclusive),
        ]
    );
    let canonical = suite.canonical_document();
    assert_eq!(parse_verification_suite(&canonical).unwrap(), suite);
    assert_eq!(canonical, suite.canonical_document());
}

#[test]
fn expectation_parser_rejects_unknown_outcomes_fail_closed() {
    let error = parse_verification_suite("suite \"bad\"\njob \"a.fvj\" expect \"sometimes\"\n")
        .unwrap_err();
    assert!(matches!(
        error.kind(),
        VerificationSuiteParseErrorKind::InvalidExpectedOutcome { outcome }
            if outcome == "sometimes"
    ));
}

#[test]
fn expectation_check_requires_every_job_to_declare_an_expectation() {
    let root = fixture_dir("missing-expectation");
    write_job(&root, "satisfied", satisfied_model_source(), "");
    let suite_path = root.join("suite.fvs");
    fs::write(&suite_path, "suite \"missing\"\njob \"satisfied.fvj\"\n").unwrap();

    let run = run_verification_suite_expectations_json(&suite_path);
    assert_eq!(run.exit_code, 2);
    assert_eq!(
        run.envelope.outcome,
        VerificationRegressionSuiteOutcome::Error
    );
    assert!(run.envelope.jobs.is_empty());
    assert!(run
        .to_json()
        .contains("expectation check requires every verification suite job"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn matched_negative_and_positive_outcomes_form_one_passing_regression_suite() {
    let root = fixture_dir("matched");
    let satisfied = write_job(&root, "satisfied", satisfied_model_source(), "");
    let finite = write_job(&root, "finite", terminal_model_source(), "");
    let lasso = write_job(&root, "lasso", unfair_model_source(), "");
    let cutoff = write_job(
        &root,
        "cutoff",
        unfair_model_source(),
        "max-product-transitions 1\n",
    );
    let malformed = root.join("malformed.fvj");
    fs::write(&malformed, "model \"missing-property.fvl\"\n").unwrap();

    let suite_path = root.join("suite.fvs");
    fs::write(
        &suite_path,
        "suite \"regression\"\n\
job \"satisfied.fvj\" expect \"satisfied\"\n\
job \"finite.fvj\" expect \"violated\"\n\
job \"lasso.fvj\" expect \"violated\"\n\
job \"cutoff.fvj\" expect \"inconclusive\"\n\
job \"malformed.fvj\" expect \"error\"\n",
    )
    .unwrap();

    let run = run_verification_suite_expectations_json(&suite_path);
    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.outcome,
        VerificationRegressionSuiteOutcome::Matched
    );
    assert_eq!(run.envelope.jobs.len(), 5);
    assert!(run.envelope.jobs.iter().all(|entry| entry.matched));

    let direct = [satisfied, finite, lasso, cutoff, malformed]
        .iter()
        .map(run_verification_job_json)
        .collect::<Vec<_>>();
    for (entry, expected) in run.envelope.jobs.iter().zip(direct.iter()) {
        assert_eq!(entry.result, expected.envelope);
        assert_eq!(entry.observed, expected.envelope.outcome);
    }
    assert_eq!(
        run.envelope.jobs[0].observed,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        run.envelope.jobs[1].observed,
        VerificationJobOutcome::Violated
    );
    assert_eq!(
        run.envelope.jobs[2].observed,
        VerificationJobOutcome::Violated
    );
    assert_eq!(
        run.envelope.jobs[3].observed,
        VerificationJobOutcome::Inconclusive
    );
    assert_eq!(run.envelope.jobs[4].observed, VerificationJobOutcome::Error);
    assert_eq!(
        run.to_json(),
        run_verification_suite_expectations_json(&suite_path).to_json()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn mismatches_are_all_retained_in_order_and_use_dedicated_exit_code() {
    let root = fixture_dir("mismatch");
    write_job(&root, "satisfied", satisfied_model_source(), "");
    write_job(&root, "violated", unfair_model_source(), "");
    let suite_path = root.join("suite.fvs");
    fs::write(
        &suite_path,
        "suite \"mismatch\"\n\
job \"satisfied.fvj\" expect \"violated\"\n\
job \"violated.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let run = run_verification_suite_expectations_json(&suite_path);
    assert_eq!(run.exit_code, 13);
    assert_eq!(
        run.envelope.outcome,
        VerificationRegressionSuiteOutcome::Mismatched
    );
    assert_eq!(run.envelope.jobs.len(), 2);
    assert_eq!(run.envelope.jobs[0].manifest, "satisfied.fvj");
    assert_eq!(run.envelope.jobs[1].manifest, "violated.fvj");
    assert!(run.envelope.jobs.iter().all(|entry| !entry.matched));
    assert_eq!(
        run.envelope.jobs[0].expected,
        VerificationExpectedOutcome::Violated
    );
    assert_eq!(
        run.envelope.jobs[0].observed,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        run.envelope.jobs[1].expected,
        VerificationExpectedOutcome::Satisfied
    );
    assert_eq!(
        run.envelope.jobs[1].observed,
        VerificationJobOutcome::Violated
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn raw_m57_suite_execution_ignores_expectations_and_keeps_observed_exit_policy() {
    let root = fixture_dir("raw-compat");
    write_job(&root, "violated", unfair_model_source(), "");
    let suite_path = root.join("suite.fvs");
    fs::write(
        &suite_path,
        "suite \"raw\"\njob \"violated.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let raw = run_verification_suite_json(&suite_path);
    assert_eq!(raw.exit_code, 7);
    assert_eq!(raw.envelope.jobs.len(), 1);
    assert_eq!(
        raw.envelope.jobs[0].result.outcome,
        VerificationJobOutcome::Violated
    );
    assert!(!raw.to_json().contains("expected"));
    assert!(!raw.to_json().contains("matched"));

    let _ = fs::remove_dir_all(root);
}

use formal_verification_lab::{
    parse_verification_suite, run_verification_job_json, run_verification_suite_json,
    VerificationJobOutcome, VerificationSuiteOutcome, VerificationSuiteParseErrorKind,
    MAX_VERIFICATION_SUITE_JOBS,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("fvlab-m57-{kind}-{}-{id}", std::process::id()));
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
fn suite_parser_preserves_order_and_canonical_round_trip() {
    let input =
        "# ordered batch\nsuite \"nightly \\\"core\\\"\"\njob \"a.fvj\"\njob \"nested/b.fvj\"\n";
    let suite = parse_verification_suite(input).unwrap();
    assert_eq!(suite.name(), "nightly \"core\"");
    assert_eq!(suite.job_paths(), ["a.fvj", "nested/b.fvj"]);

    let canonical = suite.canonical_document();
    let reparsed = parse_verification_suite(&canonical).unwrap();
    assert_eq!(reparsed, suite);
    assert_eq!(reparsed.canonical_document(), canonical);
}

#[test]
fn suite_parser_rejects_duplicate_jobs_and_bounded_overflow() {
    let duplicate =
        parse_verification_suite("suite \"duplicate\"\njob \"same.fvj\"\njob \"same.fvj\"\n")
            .unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        VerificationSuiteParseErrorKind::DuplicateJob { path } if path == "same.fvj"
    ));

    let mut oversized = String::from("suite \"too-many\"\n");
    for index in 0..=MAX_VERIFICATION_SUITE_JOBS {
        oversized.push_str(&format!("job \"job-{index}.fvj\"\n"));
    }
    let error = parse_verification_suite(&oversized).unwrap_err();
    assert!(matches!(
        error.kind(),
        VerificationSuiteParseErrorKind::TooManyJobs { limit }
            if *limit == MAX_VERIFICATION_SUITE_JOBS
    ));
}

#[test]
fn suite_execution_matches_single_job_envelopes_and_aggregate_precedence() {
    let root = fixture_dir("aggregate");
    let satisfied = write_job(&root, "satisfied", satisfied_model_source(), "");
    let inconclusive = write_job(
        &root,
        "inconclusive",
        unfair_model_source(),
        "max-product-transitions 1\n",
    );
    let violated = write_job(&root, "violated", unfair_model_source(), "");
    let suite_path = root.join("suite.fvs");
    fs::write(
        &suite_path,
        "suite \"aggregate\"\njob \"satisfied.fvj\"\njob \"inconclusive.fvj\"\njob \"violated.fvj\"\n",
    )
    .unwrap();

    let run = run_verification_suite_json(&suite_path);
    assert_eq!(run.exit_code, 7);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Violated);
    assert_eq!(run.envelope.jobs.len(), 3);
    assert_eq!(run.envelope.jobs[0].manifest, "satisfied.fvj");
    assert_eq!(run.envelope.jobs[1].manifest, "inconclusive.fvj");
    assert_eq!(run.envelope.jobs[2].manifest, "violated.fvj");

    let direct = [satisfied, inconclusive, violated]
        .iter()
        .map(run_verification_job_json)
        .collect::<Vec<_>>();
    for (entry, expected) in run.envelope.jobs.iter().zip(direct.iter()) {
        assert_eq!(entry.result, expected.envelope);
    }
    assert_eq!(
        run.envelope.jobs[0].result.outcome,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        run.envelope.jobs[1].result.outcome,
        VerificationJobOutcome::Inconclusive
    );
    assert_eq!(
        run.envelope.jobs[2].result.outcome,
        VerificationJobOutcome::Violated
    );
    assert_eq!(
        run.to_json(),
        run_verification_suite_json(&suite_path).to_json()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn per_job_error_has_aggregate_precedence_without_suppressing_later_results() {
    let root = fixture_dir("job-error");
    let malformed = root.join("bad.fvj");
    fs::write(&malformed, "model \"missing-property.fvl\"\n").unwrap();
    write_job(&root, "satisfied", satisfied_model_source(), "");
    let suite_path = root.join("suite.fvs");
    fs::write(
        &suite_path,
        "suite \"error-precedence\"\njob \"bad.fvj\"\njob \"satisfied.fvj\"\n",
    )
    .unwrap();

    let run = run_verification_suite_json(&suite_path);
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Error);
    assert_eq!(run.envelope.jobs.len(), 2);
    assert_eq!(
        run.envelope.jobs[0].result.outcome,
        VerificationJobOutcome::Error
    );
    assert_eq!(
        run.envelope.jobs[1].result.outcome,
        VerificationJobOutcome::Satisfied
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn suite_level_parse_error_is_structured_and_has_no_job_results() {
    let root = fixture_dir("suite-error");
    let suite_path = root.join("suite.fvs");
    fs::write(&suite_path, "suite \"empty\"\n").unwrap();

    let run = run_verification_suite_json(&suite_path);
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.outcome, VerificationSuiteOutcome::Error);
    assert!(run.envelope.jobs.is_empty());
    assert!(run.envelope.error.is_some());
    assert!(run
        .to_json()
        .contains("verification suite requires at least one job"));

    let _ = fs::remove_dir_all(root);
}

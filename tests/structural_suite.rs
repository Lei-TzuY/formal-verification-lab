use formal_verification_lab::{
    parse_structural_suite, run_structural_job_json, run_structural_suite_expectations_json,
    run_structural_suite_json, StructuralExpectedOutcome, StructuralJobOutcome,
    StructuralRegressionSuiteOutcome, StructuralSuiteOutcome, StructuralSuiteParseErrorKind,
    MAX_STRUCTURAL_SUITE_JOBS, STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const ACYCLIC: &str = r#"model "acyclic"
state "a"
state "b"
initial "a"
edge "a" "step" "b"
"#;

const CYCLIC: &str = r#"model "cyclic"
state "a"
initial "a"
edge "a" "loop" "a"
"#;

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("fvlab-m67-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(root.join("jobs")).unwrap();
    root
}

fn write_job(root: &Path, name: &str, model_source: &str, tail: &str) -> PathBuf {
    let jobs = root.join("jobs");
    fs::write(jobs.join(format!("{name}.fvl")), model_source).unwrap();
    let job = jobs.join(format!("{name}.fvstruct"));
    let suffix = if tail.is_empty() {
        String::new()
    } else {
        format!("\n{tail}")
    };
    fs::write(
        &job,
        format!("analysis \"recurrence\"\nmodel \"{name}.fvl\"{suffix}"),
    )
    .unwrap();
    job
}

fn write_error_job(root: &Path, name: &str) -> PathBuf {
    let job = root.join("jobs").join(format!("{name}.fvstruct"));
    fs::write(&job, "analysis \"recurrence\"\n").unwrap();
    job
}

fn write_suite(root: &Path, source: &str) -> PathBuf {
    let path = root.join("suite.fvss");
    fs::write(&path, source).unwrap();
    path
}

#[test]
fn structural_suite_round_trips_canonically_with_optional_expectations() {
    let input = r#"
suite "nightly"
job "jobs/cycle.fvstruct" expect "cycle_found"
job "jobs/acyclic.fvstruct"
job "jobs/cutoff.fvstruct" expect "inconclusive"
"#;
    let suite = parse_structural_suite(input).unwrap();

    assert_eq!(suite.name(), "nightly");
    assert_eq!(
        suite.job_paths(),
        [
            "jobs/cycle.fvstruct",
            "jobs/acyclic.fvstruct",
            "jobs/cutoff.fvstruct",
        ]
    );
    assert_eq!(
        suite.expected_outcomes(),
        [
            Some(StructuralExpectedOutcome::CycleFound),
            None,
            Some(StructuralExpectedOutcome::Inconclusive),
        ]
    );

    let canonical = suite.canonical_document();
    assert_eq!(parse_structural_suite(&canonical).unwrap(), suite);
    assert_eq!(canonical, suite.canonical_document());
}

#[test]
fn structural_suite_parser_fails_closed_on_invalid_metadata_and_size() {
    let invalid =
        parse_structural_suite("suite \"bad\"\njob \"a\" expect \"violated\"").unwrap_err();
    assert!(matches!(
        invalid.kind(),
        StructuralSuiteParseErrorKind::InvalidExpectedOutcome { outcome }
            if outcome == "violated"
    ));

    let duplicate = parse_structural_suite("suite \"dup\"\njob \"a\"\njob \"a\"").unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        StructuralSuiteParseErrorKind::DuplicateJob { path } if path == "a"
    ));

    let no_jobs = parse_structural_suite("suite \"empty\"").unwrap_err();
    assert!(matches!(
        no_jobs.kind(),
        StructuralSuiteParseErrorKind::NoJobs
    ));

    let mut oversized = String::from("suite \"oversized\"\n");
    for index in 0..=MAX_STRUCTURAL_SUITE_JOBS {
        oversized.push_str(&format!("job \"job-{index}\"\n"));
    }
    let too_many = parse_structural_suite(&oversized).unwrap_err();
    assert!(matches!(
        too_many.kind(),
        StructuralSuiteParseErrorKind::TooManyJobs { limit }
            if *limit == MAX_STRUCTURAL_SUITE_JOBS
    ));
}

#[test]
fn raw_suite_keeps_cycle_and_acyclic_results_neutral_and_ordered() {
    let root = fixture_dir("raw-complete");
    let cycle = write_job(&root, "cycle", CYCLIC, "");
    let acyclic = write_job(&root, "acyclic", ACYCLIC, "");
    let suite = write_suite(
        &root,
        "suite \"mixed\"\njob \"jobs/cycle.fvstruct\"\njob \"jobs/acyclic.fvstruct\"\n",
    );

    let run = run_structural_suite_json(&suite);
    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.outcome, StructuralSuiteOutcome::Complete);
    assert_eq!(run.envelope.jobs.len(), 2);
    assert_eq!(run.envelope.jobs[0].manifest, "jobs/cycle.fvstruct");
    assert_eq!(run.envelope.jobs[1].manifest, "jobs/acyclic.fvstruct");
    assert_eq!(
        run.envelope.jobs[0].result,
        run_structural_job_json(&cycle).envelope
    );
    assert_eq!(
        run.envelope.jobs[1].result,
        run_structural_job_json(&acyclic).envelope
    );
    assert_eq!(
        run.envelope.jobs[0].result.outcome,
        StructuralJobOutcome::CycleFound
    );
    assert_eq!(
        run.envelope.jobs[1].result.outcome,
        StructuralJobOutcome::Acyclic
    );
    assert!(!run.to_json().contains("satisfied"));
    assert!(!run.to_json().contains("violated"));

    fs::remove_dir_all(root).ok();
}

#[test]
fn raw_suite_uses_error_over_inconclusive_precedence_without_dropping_entries() {
    let root = fixture_dir("raw-precedence");
    write_job(&root, "cutoff", ACYCLIC, "max-transitions 0");
    write_error_job(&root, "broken");

    let inconclusive_suite = write_suite(&root, "suite \"cutoff\"\njob \"jobs/cutoff.fvstruct\"\n");
    let inconclusive = run_structural_suite_json(&inconclusive_suite);
    assert_eq!(inconclusive.exit_code, 3);
    assert_eq!(
        inconclusive.envelope.outcome,
        StructuralSuiteOutcome::Inconclusive
    );

    let error_suite = write_suite(
        &root,
        "suite \"mixed-error\"\njob \"jobs/cutoff.fvstruct\"\njob \"jobs/broken.fvstruct\"\n",
    );
    let error = run_structural_suite_json(&error_suite);
    assert_eq!(error.exit_code, 2);
    assert_eq!(error.envelope.outcome, StructuralSuiteOutcome::Error);
    assert_eq!(error.envelope.jobs.len(), 2);
    assert_eq!(
        error.envelope.jobs[0].result.outcome,
        StructuralJobOutcome::Inconclusive
    );
    assert_eq!(
        error.envelope.jobs[1].result.outcome,
        StructuralJobOutcome::Error
    );

    fs::remove_dir_all(root).ok();
}

#[test]
fn expectation_mode_matches_all_four_structural_outcomes() {
    let root = fixture_dir("expectations");
    write_job(&root, "cycle", CYCLIC, "");
    write_job(&root, "acyclic", ACYCLIC, "");
    write_job(&root, "cutoff", ACYCLIC, "max-transitions 0");
    write_error_job(&root, "broken");
    let suite = write_suite(
        &root,
        "suite \"expected\"\n\
job \"jobs/cycle.fvstruct\" expect \"cycle_found\"\n\
job \"jobs/acyclic.fvstruct\" expect \"acyclic\"\n\
job \"jobs/cutoff.fvstruct\" expect \"inconclusive\"\n\
job \"jobs/broken.fvstruct\" expect \"error\"\n",
    );

    let run = run_structural_suite_expectations_json(&suite);
    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.outcome,
        StructuralRegressionSuiteOutcome::Matched
    );
    assert_eq!(run.envelope.jobs.len(), 4);
    assert!(run.envelope.jobs.iter().all(|entry| entry.matched));
    assert_eq!(
        run.envelope
            .jobs
            .iter()
            .map(|entry| entry.observed)
            .collect::<Vec<_>>(),
        vec![
            StructuralJobOutcome::CycleFound,
            StructuralJobOutcome::Acyclic,
            StructuralJobOutcome::Inconclusive,
            StructuralJobOutcome::Error,
        ]
    );
    assert_eq!(
        run.to_json(),
        run_structural_suite_expectations_json(&suite).to_json()
    );

    fs::remove_dir_all(root).ok();
}

#[test]
fn expectation_mismatches_are_retained_in_order_with_dedicated_exit() {
    let root = fixture_dir("mismatch");
    write_job(&root, "cycle", CYCLIC, "");
    write_job(&root, "acyclic", ACYCLIC, "");
    let suite = write_suite(
        &root,
        "suite \"mismatch\"\n\
job \"jobs/cycle.fvstruct\" expect \"acyclic\"\n\
job \"jobs/acyclic.fvstruct\" expect \"cycle_found\"\n",
    );

    let run = run_structural_suite_expectations_json(&suite);
    assert_eq!(run.exit_code, STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE);
    assert_eq!(
        run.envelope.outcome,
        StructuralRegressionSuiteOutcome::Mismatched
    );
    assert_eq!(run.envelope.jobs.len(), 2);
    assert!(run.envelope.jobs.iter().all(|entry| !entry.matched));
    assert_eq!(run.envelope.jobs[0].manifest, "jobs/cycle.fvstruct");
    assert_eq!(run.envelope.jobs[1].manifest, "jobs/acyclic.fvstruct");
    assert_eq!(
        run.envelope.jobs[0].observed,
        StructuralJobOutcome::CycleFound
    );
    assert_eq!(run.envelope.jobs[1].observed, StructuralJobOutcome::Acyclic);

    let raw = run_structural_suite_json(&suite);
    assert_eq!(raw.exit_code, 0);
    assert_eq!(raw.envelope.outcome, StructuralSuiteOutcome::Complete);
    assert!(!raw.to_json().contains("\"expected\""));
    assert!(!raw.to_json().contains("\"matched\""));

    fs::remove_dir_all(root).ok();
}

#[test]
fn expectation_check_requires_every_job_to_declare_expectation() {
    let root = fixture_dir("missing-expectation");
    write_job(&root, "acyclic", ACYCLIC, "");
    let suite = write_suite(&root, "suite \"missing\"\njob \"jobs/acyclic.fvstruct\"\n");

    let run = run_structural_suite_expectations_json(&suite);
    assert_eq!(run.exit_code, 2);
    assert_eq!(
        run.envelope.outcome,
        StructuralRegressionSuiteOutcome::Error
    );
    assert!(run.envelope.jobs.is_empty());
    assert!(run
        .to_json()
        .contains("expectation check requires every structural suite job"));

    fs::remove_dir_all(root).ok();
}

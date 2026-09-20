use formal_verification_lab::{
    create_declarative_mu_parity_certificate, parse_declarative_document,
    parse_orchestration_suite, render_declarative_mu_parity_certificate,
    run_certificate_verification_job_json, run_orchestration_suite_expectations_json,
    run_orchestration_suite_json, run_structural_job_json, run_verification_job_json,
    CertificateVerificationJobOutcome, OrchestrationJobFamily, OrchestrationNestedResult,
    OrchestrationRegressionOutcome, OrchestrationSuiteParseErrorKind, OrchestrationSuiteStatus,
    ORCHESTRATION_ATTENTION_EXIT_CODE, ORCHESTRATION_REGRESSION_MISMATCH_EXIT_CODE,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "orchestration"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "cycle" "loop"
edge "start" "finish" "done"
edge "loop" "back" "start"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#;

const FORMULA: &str = r#"mu X. "complete" or diamond $X"#;

#[test]
fn parser_round_trips_family_tagged_entries_and_rejects_cross_family_expectations() {
    let source = r#"
# mixed job families
suite "mixed"
job "verification" "jobs/verify.job" expect "satisfied"
job "structural" "jobs/structure.job" expect "cycle_found"
job "certificate-verification" "jobs/certificate.job" expect "verified"
"#;
    let suite = parse_orchestration_suite(source).unwrap();
    assert_eq!(suite.name(), "mixed");
    assert_eq!(suite.entries().len(), 3);
    assert_eq!(
        suite.entries()[0].family(),
        OrchestrationJobFamily::Verification
    );
    assert_eq!(
        suite.entries()[1].family(),
        OrchestrationJobFamily::Structural
    );
    assert_eq!(
        suite.entries()[2].family(),
        OrchestrationJobFamily::CertificateVerification
    );

    let canonical = suite.canonical_document();
    assert_eq!(
        canonical,
        "suite \"mixed\"\njob \"verification\" \"jobs/verify.job\" expect \"satisfied\"\njob \"structural\" \"jobs/structure.job\" expect \"cycle_found\"\njob \"certificate-verification\" \"jobs/certificate.job\" expect \"verified\""
    );
    assert_eq!(parse_orchestration_suite(&canonical).unwrap(), suite);

    let invalid = parse_orchestration_suite(
        "suite \"bad\"\njob \"verification\" \"a.job\" expect \"verified\"",
    )
    .unwrap_err();
    assert!(matches!(
        invalid.kind(),
        OrchestrationSuiteParseErrorKind::InvalidExpectedOutcome {
            family: OrchestrationJobFamily::Verification,
            outcome,
        } if outcome == "verified"
    ));

    let duplicate = parse_orchestration_suite(
        "suite \"bad\"\njob \"structural\" \"same.job\"\njob \"structural\" \"same.job\"",
    )
    .unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        OrchestrationSuiteParseErrorKind::DuplicateJob {
            family: OrchestrationJobFamily::Structural,
            path,
        } if path == "same.job"
    ));

    let same_path_different_family = parse_orchestration_suite(
        "suite \"ok\"\njob \"verification\" \"same.job\"\njob \"structural\" \"same.job\"",
    )
    .unwrap();
    assert_eq!(same_path_different_family.entries().len(), 2);
}

#[test]
fn raw_orchestration_preserves_direct_family_payloads_and_order() {
    let root = fixture_dir("raw");
    let suite_path = write_fixture(&root, true);

    let run = run_orchestration_suite_json(&suite_path);
    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.status, OrchestrationSuiteStatus::Complete);
    assert_eq!(run.envelope.suite.as_deref(), Some("mixed"));
    assert_eq!(run.envelope.jobs.len(), 3);

    let verification_path = root.join("jobs/verify.job");
    let structural_path = root.join("jobs/structure.job");
    let certificate_path = root.join("jobs/certificate.job");
    let verification = run_verification_job_json(&verification_path);
    let structural = run_structural_job_json(&structural_path);
    let certificate = run_certificate_verification_job_json(&certificate_path);

    match &run.envelope.jobs[0].result {
        OrchestrationNestedResult::Verification(result) => {
            assert_eq!(result.as_ref(), &verification.envelope);
        }
        other => panic!("expected verification result, got {other:?}"),
    }
    match &run.envelope.jobs[1].result {
        OrchestrationNestedResult::Structural(result) => {
            assert_eq!(result.as_ref(), &structural.envelope);
        }
        other => panic!("expected structural result, got {other:?}"),
    }
    match &run.envelope.jobs[2].result {
        OrchestrationNestedResult::CertificateVerification(result) => {
            assert_eq!(result.as_ref(), &certificate.envelope);
        }
        other => panic!("expected certificate result, got {other:?}"),
    }

    let json = run.to_json();
    assert!(json.contains(&verification.to_json()));
    assert!(json.contains(&structural.to_json()));
    assert!(json.contains(&certificate.to_json()));
    assert!(!json.contains(root.to_str().unwrap()));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verification_violation_and_structural_cycle_do_not_become_orchestration_failures() {
    let root = fixture_dir("domain-outcomes");
    let suite_path = write_fixture(&root, false);

    fs::write(root.join("properties/false.mu"), "false").unwrap();
    fs::write(
        root.join("jobs/verify.job"),
        "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/false.mu\"\n",
    )
    .unwrap();

    let run = run_orchestration_suite_json(&suite_path);
    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.status, OrchestrationSuiteStatus::Complete);
    assert_eq!(run.envelope.jobs[0].result.outcome_str(), "violated");
    assert_eq!(run.envelope.jobs[1].result.outcome_str(), "cycle_found");
    assert_eq!(run.envelope.jobs[2].result.outcome_str(), "verified");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn raw_status_distinguishes_certificate_rejection_from_execution_error() {
    let root = fixture_dir("status");
    let suite_path = write_fixture(&root, false);
    let certificate = root.join("certificates/reach.mupc");

    let original = fs::read_to_string(&certificate).unwrap();
    let truncated = original
        .lines()
        .filter(|line| *line != "end")
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&certificate, truncated).unwrap();

    let rejected = run_orchestration_suite_json(&suite_path);
    assert_eq!(rejected.exit_code, ORCHESTRATION_ATTENTION_EXIT_CODE);
    assert_eq!(
        rejected.envelope.status,
        OrchestrationSuiteStatus::Attention
    );
    assert_eq!(rejected.envelope.jobs[2].result.outcome_str(), "rejected");

    fs::remove_file(root.join("models/system.fvl")).unwrap();
    let failed = run_orchestration_suite_json(&suite_path);
    assert_eq!(failed.exit_code, 2);
    assert_eq!(failed.envelope.status, OrchestrationSuiteStatus::Error);
    assert!(failed
        .envelope
        .jobs
        .iter()
        .any(|job| job.result.outcome_str() == "error"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn expectation_mode_keeps_operational_status_separate_from_regression_match() {
    let root = fixture_dir("expectations");
    let suite_path = write_fixture(&root, true);

    let matched = run_orchestration_suite_expectations_json(&suite_path);
    assert_eq!(matched.exit_code, 0);
    assert_eq!(
        matched.envelope.outcome,
        OrchestrationRegressionOutcome::Matched
    );
    assert_eq!(
        matched.envelope.execution_status,
        OrchestrationSuiteStatus::Complete
    );
    assert!(matched.envelope.jobs.iter().all(|job| job.matched));

    let certificate = root.join("certificates/reach.mupc");
    let original = fs::read_to_string(&certificate).unwrap();
    fs::write(
        &certificate,
        original
            .lines()
            .filter(|line| *line != "end")
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    rewrite_certificate_expectation(&suite_path, "rejected");

    let expected_rejection = run_orchestration_suite_expectations_json(&suite_path);
    assert_eq!(expected_rejection.exit_code, 0);
    assert_eq!(
        expected_rejection.envelope.outcome,
        OrchestrationRegressionOutcome::Matched
    );
    assert_eq!(
        expected_rejection.envelope.execution_status,
        OrchestrationSuiteStatus::Attention
    );
    assert_eq!(
        expected_rejection.envelope.jobs[2].result.outcome_str(),
        "rejected"
    );

    rewrite_certificate_expectation(&suite_path, "verified");
    let mismatch = run_orchestration_suite_expectations_json(&suite_path);
    assert_eq!(
        mismatch.exit_code,
        ORCHESTRATION_REGRESSION_MISMATCH_EXIT_CODE
    );
    assert_eq!(
        mismatch.envelope.outcome,
        OrchestrationRegressionOutcome::Mismatched
    );
    assert!(!mismatch.envelope.jobs[2].matched);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn expectation_mode_requires_complete_expectations() {
    let root = fixture_dir("missing-expectation");
    let suite_path = write_fixture(&root, false);
    let run = run_orchestration_suite_expectations_json(&suite_path);
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.outcome, OrchestrationRegressionOutcome::Error);
    assert!(run
        .envelope
        .error
        .as_deref()
        .is_some_and(|message| message.contains("requires every orchestration suite job")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_orchestration_cli_matches_direct_raw_and_expectation_runs() {
    let root = fixture_dir("cli");
    let suite_path = write_fixture(&root, true);

    let raw = run_orchestration_suite_json(&suite_path);
    let raw_cli = run_binary(&[suite_path.to_str().unwrap(), "--format", "json"]);
    assert_eq!(raw_cli.status.code(), Some(raw.exit_code as i32));
    assert_eq!(
        String::from_utf8(raw_cli.stdout).unwrap(),
        format!("{}\n", raw.to_json())
    );
    assert!(raw_cli.stderr.is_empty());

    let expected = run_orchestration_suite_expectations_json(&suite_path);
    let expected_cli = run_binary(&[
        suite_path.to_str().unwrap(),
        "--check-expectations",
        "--format",
        "json",
    ]);
    assert_eq!(expected_cli.status.code(), Some(expected.exit_code as i32));
    assert_eq!(
        String::from_utf8(expected_cli.stdout).unwrap(),
        format!("{}\n", expected.to_json())
    );
    assert!(expected_cli.stderr.is_empty());

    let certificate = root.join("certificates/reach.mupc");
    let original = fs::read_to_string(&certificate).unwrap();
    fs::write(
        &certificate,
        original
            .lines()
            .filter(|line| *line != "end")
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    rewrite_certificate_expectation(&suite_path, "rejected");

    let rejected = run_orchestration_suite_expectations_json(&suite_path);
    assert_eq!(
        rejected.envelope.jobs[2].result.outcome_str(),
        CertificateVerificationJobOutcome::Rejected.as_str()
    );
    let rejected_cli = run_binary(&[
        suite_path.to_str().unwrap(),
        "--check-expectations",
        "--format",
        "json",
    ]);
    assert_eq!(
        String::from_utf8(rejected_cli.stdout).unwrap(),
        format!("{}\n", rejected.to_json())
    );
    assert_eq!(rejected_cli.status.code(), Some(0));

    fs::remove_dir_all(root).unwrap();
}

fn write_fixture(root: &Path, expectations: bool) -> PathBuf {
    fs::create_dir_all(root.join("models")).unwrap();
    fs::create_dir_all(root.join("properties")).unwrap();
    fs::create_dir_all(root.join("certificates")).unwrap();
    fs::create_dir_all(root.join("jobs")).unwrap();

    fs::write(root.join("models/system.fvl"), MODEL).unwrap();
    fs::write(root.join("properties/reach.mu"), FORMULA).unwrap();

    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();
    fs::write(
        root.join("certificates/reach.mupc"),
        render_declarative_mu_parity_certificate(&certificate),
    )
    .unwrap();

    fs::write(
        root.join("jobs/verify.job"),
        "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/reach.mu\"\n",
    )
    .unwrap();
    fs::write(
        root.join("jobs/structure.job"),
        "analysis \"recurrence\"\nmodel \"../models/system.fvl\"\n",
    )
    .unwrap();
    fs::write(
        root.join("jobs/certificate.job"),
        "model \"../models/system.fvl\"\nproperty \"../properties/reach.mu\"\ncertificate \"../certificates/reach.mupc\"\n",
    )
    .unwrap();

    let suite_path = root.join("mixed.suite");
    let suffix = if expectations {
        [
            "job \"verification\" \"jobs/verify.job\" expect \"satisfied\"",
            "job \"structural\" \"jobs/structure.job\" expect \"cycle_found\"",
            "job \"certificate-verification\" \"jobs/certificate.job\" expect \"verified\"",
        ]
        .join("\n")
    } else {
        [
            "job \"verification\" \"jobs/verify.job\"",
            "job \"structural\" \"jobs/structure.job\"",
            "job \"certificate-verification\" \"jobs/certificate.job\"",
        ]
        .join("\n")
    };
    fs::write(&suite_path, format!("suite \"mixed\"\n{suffix}\n")).unwrap();
    suite_path
}

fn rewrite_certificate_expectation(path: &Path, expected: &str) {
    let input = fs::read_to_string(path).unwrap();
    let mut lines = Vec::new();
    for line in input.lines() {
        if line.starts_with("job \"certificate-verification\"") {
            let base = line.split(" expect ").next().unwrap();
            lines.push(format!("{base} expect \"{expected}\""));
        } else {
            lines.push(line.to_owned());
        }
    }
    fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}

fn run_binary(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-orchestrate"))
        .args(args)
        .output()
        .expect("fvlab-orchestrate binary should execute")
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m88-orchestration-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

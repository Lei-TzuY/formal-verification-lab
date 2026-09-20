use formal_verification_lab::{
    check_declarative_mu_text_via_parity, parse_declarative_document, parse_verification_job,
    run_verification_job_json, run_verification_suite_expectations_json, run_verification_suite_json,
    VerificationJobMuBackend, VerificationJobMuTruth, VerificationJobOutcome,
    VerificationJobParseErrorKind, VerificationRegressionSuiteOutcome, VerificationSuiteOutcome,
    VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "mu-parity-job"
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
        "fvlab-m83-mu-parity-job-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_mu_job(
    root: &Path,
    name: &str,
    backend: Option<&str>,
    property: &str,
    tail: &str,
) -> PathBuf {
    fs::write(root.join(format!("{name}.fvl")), MODEL).unwrap();
    fs::write(root.join(format!("{name}.mu")), property).unwrap();
    let manifest = root.join(format!("{name}.fvj"));
    let backend = backend
        .map(|backend| format!("backend \"{backend}\"\n"))
        .unwrap_or_default();
    fs::write(
        &manifest,
        format!(
            "analysis \"mu-calculus\"\n{backend}model \"{name}.fvl\"\nproperty \"{name}.mu\"\n{tail}"
        ),
    )
    .unwrap();
    manifest
}

fn run_job_binary(manifest: &Path, cwd: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fvlab"));
    command.args([
        "temporal",
        "job",
        manifest.to_str().expect("fixture path should be UTF-8"),
        "--format",
        "json",
    ]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.output().unwrap()
}

#[test]
fn mu_backend_manifest_is_canonical_and_absent_backend_is_legacy_compatible() {
    let legacy =
        "analysis \"mu-calculus\"\nmodel \"models/system.fvl\"\nproperty \"properties/query.mu\"";
    let legacy_job = parse_verification_job(legacy).unwrap();
    assert_eq!(legacy_job.declared_mu_backend(), None);
    assert_eq!(legacy_job.mu_backend(), VerificationJobMuBackend::Fixpoint);
    assert_eq!(legacy_job.canonical_document(), legacy);

    let parity =
        "analysis \"mu-calculus\"\nbackend \"parity\"\nmodel \"models/system.fvl\"\nproperty \"properties/query.mu\"";
    let parity_job = parse_verification_job(parity).unwrap();
    assert_eq!(
        parity_job.declared_mu_backend(),
        Some(VerificationJobMuBackend::Parity)
    );
    assert_eq!(parity_job.mu_backend(), VerificationJobMuBackend::Parity);
    assert_eq!(parity_job.canonical_document(), parity);
    assert_eq!(
        parse_verification_job(&parity_job.canonical_document()).unwrap(),
        parity_job
    );

    let explicit_fixpoint =
        "analysis \"mu-calculus\"\nbackend \"fixpoint\"\nmodel \"m.fvl\"\nproperty \"p.mu\"";
    assert_eq!(
        parse_verification_job(explicit_fixpoint)
            .unwrap()
            .mu_backend(),
        VerificationJobMuBackend::Fixpoint
    );
}

#[test]
fn backend_directive_fails_closed_for_invalid_value_and_non_mu_family() {
    let invalid = parse_verification_job(
        "analysis \"mu-calculus\"\nbackend \"other\"\nmodel \"m\"\nproperty \"p\"",
    )
    .unwrap_err();
    assert!(matches!(
        invalid.kind(),
        VerificationJobParseErrorKind::InvalidBackend { backend } if backend == "other"
    ));

    for source in [
        "analysis \"safety\"\nbackend \"parity\"\nmodel \"m\"\nproperty \"p\"",
        "backend \"parity\"\nmodel \"m\"\nproperty \"p\"",
    ] {
        let error = parse_verification_job(source).unwrap_err();
        assert!(matches!(
            error.kind(),
            VerificationJobParseErrorKind::BackendRequiresMuCalculus { .. }
        ));
    }
}

#[test]
fn parity_job_matches_m82_frontend_and_preserves_backend_specific_details() {
    let root = fixture_dir("direct");
    let manifest = write_mu_job(&root, "parity", Some("parity"), REACH_COMPLETE, "");
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.schema_version, VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION);
    assert_eq!(run.envelope.analysis.as_deref(), Some("mu-calculus"));
    assert_eq!(run.envelope.backend.as_deref(), Some("mu-parity"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(run.envelope.cutoff, None);

    let document = parse_declarative_document(MODEL).unwrap();
    let direct = check_declarative_mu_text_via_parity(&document, REACH_COMPLETE).unwrap();
    let mu = run.envelope.mu.as_ref().unwrap();
    assert!(mu.initial_states_complete);
    assert_eq!(mu.retained_states, direct.evaluation.reachable_states.len());
    assert_eq!(
        mu.definitely_satisfying_states,
        direct.evaluation.satisfying_state_indices.len()
    );
    assert_eq!(
        mu.possibly_satisfying_states,
        direct.evaluation.satisfying_state_indices.len()
    );
    assert_eq!(mu.fixpoint_iterations, None);
    assert_eq!(
        mu.parity_game_vertices,
        Some(direct.evaluation.parity_game_vertices)
    );
    assert_eq!(
        mu.max_parity_priority,
        Some(direct.evaluation.max_priority)
    );
    assert_eq!(mu.initial[0].truth, VerificationJobMuTruth::True);
    assert_eq!(
        run.envelope.accounting.model_states,
        Some(direct.evaluation.discovered_states)
    );
    assert_eq!(
        run.envelope.accounting.explored_model_transitions,
        Some(direct.evaluation.explored_transitions)
    );

    fs::write(root.join("parity.mu"), r#""complete""#).unwrap();
    let violated = run_verification_job_json(&manifest);
    assert_eq!(violated.exit_code, 15);
    assert_eq!(violated.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(violated.envelope.backend.as_deref(), Some("mu-parity"));
    assert_eq!(
        violated.envelope.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::False
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parity_and_fixpoint_complete_jobs_agree_semantically_without_faking_iterations() {
    let root = fixture_dir("differential");
    let parity = write_mu_job(&root, "parity", Some("parity"), REACH_COMPLETE, "");
    let fixpoint = write_mu_job(&root, "fixpoint", Some("fixpoint"), REACH_COMPLETE, "");

    let parity_run = run_verification_job_json(&parity);
    let fixpoint_run = run_verification_job_json(&fixpoint);
    assert_eq!(parity_run.exit_code, fixpoint_run.exit_code);
    assert_eq!(parity_run.envelope.outcome, fixpoint_run.envelope.outcome);
    assert_eq!(parity_run.envelope.property, fixpoint_run.envelope.property);
    assert_eq!(parity_run.envelope.accounting, fixpoint_run.envelope.accounting);

    let parity_mu = parity_run.envelope.mu.as_ref().unwrap();
    let fixpoint_mu = fixpoint_run.envelope.mu.as_ref().unwrap();
    assert_eq!(
        parity_mu.initial_states_complete,
        fixpoint_mu.initial_states_complete
    );
    assert_eq!(parity_mu.retained_states, fixpoint_mu.retained_states);
    assert_eq!(
        parity_mu.definitely_satisfying_states,
        fixpoint_mu.definitely_satisfying_states
    );
    assert_eq!(
        parity_mu.possibly_satisfying_states,
        fixpoint_mu.possibly_satisfying_states
    );
    assert_eq!(parity_mu.initial, fixpoint_mu.initial);
    assert_eq!(parity_mu.fixpoint_iterations, None);
    assert!(fixpoint_mu
        .fixpoint_iterations
        .is_some_and(|iterations| iterations > 0));
    assert!(parity_mu.parity_game_vertices.is_some());
    assert_eq!(fixpoint_mu.parity_game_vertices, None);
    assert_eq!(fixpoint_mu.max_parity_priority, None);

    let fixpoint_json = fixpoint_run.to_json();
    assert!(fixpoint_json.contains("\"fixpoint_iterations\":"));
    assert!(!fixpoint_json.contains("\"parity_game_vertices\""));
    assert!(!fixpoint_json.contains("\"max_parity_priority\""));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parity_job_rejects_model_limits_before_referenced_file_io() {
    let root = fixture_dir("bounded-reject");
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        "analysis \"mu-calculus\"\nbackend \"parity\"\nmodel \"missing.fvl\"\nproperty \"missing.mu\"\nmax-model-states 1\n",
    )
    .unwrap();

    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
    assert_eq!(run.envelope.backend.as_deref(), Some("mu-parity"));
    let error = run.envelope.error.as_deref().unwrap();
    assert!(error.contains("parity verification jobs do not support max-model-* limits"));
    assert!(!error.contains("failed to read"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parity_job_errors_retain_parity_backend_identity() {
    let root = fixture_dir("errors");
    let manifest = write_mu_job(&root, "parity", Some("parity"), "mu X. (", "");

    let malformed = run_verification_job_json(&manifest);
    assert_eq!(malformed.exit_code, 2);
    assert_eq!(malformed.envelope.backend.as_deref(), Some("mu-parity"));
    assert!(malformed
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("mu-calculus parse error"));

    fs::write(root.join("parity.mu"), r#""missing""#).unwrap();
    let unknown = run_verification_job_json(&manifest);
    assert_eq!(unknown.exit_code, 2);
    assert_eq!(unknown.envelope.backend.as_deref(), Some("mu-parity"));
    assert!(unknown
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("unknown mu-calculus proposition 'missing'"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_binary_parity_job_is_manifest_relative_and_emits_stable_schema_v4_json() {
    let root = fixture_dir("binary");
    let manifest = write_mu_job(&root, "parity", Some("parity"), REACH_COMPLETE, "");
    let unrelated = root.join("unrelated-cwd");
    fs::create_dir_all(&unrelated).unwrap();

    let output = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"schema_version\":4"));
    assert!(stdout.contains("\"analysis\":\"mu-calculus\""));
    assert!(stdout.contains("\"backend\":\"mu-parity\""));
    assert!(stdout.contains("\"outcome\":\"satisfied\""));
    assert!(stdout.contains("\"fixpoint_iterations\":null"));
    assert!(stdout.contains("\"parity_game_vertices\":"));
    assert!(stdout.contains("\"max_parity_priority\":"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn generic_suites_preserve_mixed_parity_and_fixpoint_mu_envelopes() {
    let root = fixture_dir("suite");
    let parity = write_mu_job(&root, "parity", Some("parity"), REACH_COMPLETE, "");
    let fixpoint = write_mu_job(&root, "fixpoint", None, REACH_COMPLETE, "");

    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"mu-backends\"\njob \"parity.fvj\" expect \"satisfied\"\njob \"fixpoint.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let raw = run_verification_suite_json(&suite);
    assert_eq!(raw.exit_code, 0);
    assert_eq!(raw.envelope.outcome, VerificationSuiteOutcome::Satisfied);
    assert_eq!(raw.envelope.jobs.len(), 2);

    let direct = [&parity, &fixpoint]
        .into_iter()
        .map(run_verification_job_json)
        .collect::<Vec<_>>();
    for (entry, direct) in raw.envelope.jobs.iter().zip(&direct) {
        assert_eq!(entry.result, direct.envelope);
    }
    assert_eq!(raw.envelope.jobs[0].result.backend.as_deref(), Some("mu-parity"));
    assert_eq!(
        raw.envelope.jobs[1].result.backend.as_deref(),
        Some("mu-fixpoint")
    );

    let checked = run_verification_suite_expectations_json(&suite);
    assert_eq!(checked.exit_code, 0);
    assert_eq!(
        checked.envelope.outcome,
        VerificationRegressionSuiteOutcome::Matched
    );
    assert!(checked.envelope.jobs.iter().all(|entry| entry.matched));
    assert_eq!(
        checked.envelope.jobs[0].result,
        run_verification_job_json(&parity).envelope
    );
    assert_eq!(
        checked.envelope.jobs[1].result,
        run_verification_job_json(&fixpoint).envelope
    );

    fs::remove_dir_all(root).unwrap();
}

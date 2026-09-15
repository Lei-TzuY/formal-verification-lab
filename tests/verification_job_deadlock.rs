use formal_verification_lab::{
    check_declarative_deadlock_with_limits, parse_declarative_deadlock_spec,
    parse_declarative_document, parse_verification_job, run_verification_job_json, BoundedOutcome,
    DeadlockStatus, ExplorationLimits, VerificationJobAnalysis, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobOutcome,
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m63-deadlock-job-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn deadlock_free_model() -> &'static str {
    "model \"deadlock-free\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"finish\" \"done\"\nlabel \"done\" \"legitimate\"\n"
}

fn deadlock_found_model() -> &'static str {
    "model \"deadlock-found\"\nstate \"start\"\nstate \"bad\"\nstate \"declared-legitimate\"\ninitial \"start\"\nedge \"start\" \"stop\" \"bad\"\nlabel \"declared-legitimate\" \"legitimate\"\n"
}

fn write_deadlock_job(root: &Path, model: &str, property: &str, tail: &str) -> PathBuf {
    fs::write(root.join("model.fvl"), model).unwrap();
    fs::write(root.join("property.fvp"), property).unwrap();
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        format!(
            "analysis \"deadlock\"\nmodel \"model.fvl\"\nproperty \"property.fvp\"\n{tail}"
        ),
    )
    .unwrap();
    manifest
}

#[test]
fn deadlock_analysis_round_trips_through_verification_job_manifest() {
    let source = "analysis \"deadlock\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"\nmax-model-depth 3";
    let job = parse_verification_job(source).unwrap();

    assert_eq!(job.analysis(), VerificationJobAnalysis::Deadlock);
    assert_eq!(job.declared_analysis(), Some(VerificationJobAnalysis::Deadlock));
    assert_eq!(job.canonical_document(), source);
    assert_eq!(parse_verification_job(&job.canonical_document()).unwrap(), job);
}

#[test]
fn deadlock_jobs_preserve_native_status_shortest_evidence_and_model_only_accounting() {
    let found_root = fixture_dir("found");
    let found_manifest = write_deadlock_job(
        &found_root,
        deadlock_found_model(),
        "\"legitimate\"\n",
        "",
    );
    let found = run_verification_job_json(&found_manifest);

    assert_eq!(found.exit_code, 5);
    assert_eq!(
        found.envelope.schema_version,
        VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION
    );
    assert_eq!(found.envelope.analysis.as_deref(), Some("deadlock"));
    assert_eq!(found.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(found.envelope.model.as_deref(), Some("deadlock-found"));
    assert_eq!(found.envelope.property.as_deref(), Some("\"legitimate\""));
    assert!(found.envelope.weak_fair_actions.is_empty());
    assert!(found.envelope.strong_fair_actions.is_empty());
    assert_eq!(found.envelope.accounting.product_states, None);
    assert_eq!(found.envelope.cutoff, None);
    let Some(VerificationJobEvidence::Deadlock { trace }) = &found.envelope.evidence else {
        panic!("expected native deadlock witness");
    };
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].action, None);
    assert_eq!(trace[0].state, "start");
    assert_eq!(trace[1].action.as_deref(), Some("stop"));
    assert_eq!(trace[1].state, "bad");
    assert!(found.to_json().starts_with(
        "{\"schema_version\":2,\"analysis\":\"deadlock\",\"outcome\":\"violated\",\"status\":\"DEADLOCK_FOUND\""
    ));
    assert!(found.to_json().contains("\"kind\":\"deadlock\""));
    assert!(!found.to_json().contains("\"pending\""));

    let found_document = parse_declarative_document(deadlock_found_model()).unwrap();
    let found_spec =
        parse_declarative_deadlock_spec("verification-job-deadlock", "\"legitimate\"").unwrap();
    let found_direct = check_declarative_deadlock_with_limits(
        &found_document,
        &found_spec,
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        found_direct.outcome,
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFound)
    );
    assert_eq!(
        found.envelope.accounting.model_states,
        Some(found_direct.discovered_states)
    );
    assert_eq!(
        found.envelope.accounting.checked_model_states,
        Some(found_direct.checked_states)
    );
    assert_eq!(
        found.envelope.accounting.explored_model_transitions,
        Some(found_direct.explored_transitions)
    );

    let free_root = fixture_dir("free");
    let free_manifest = write_deadlock_job(
        &free_root,
        deadlock_free_model(),
        "\"legitimate\"\n",
        "",
    );
    let free = run_verification_job_json(&free_manifest);
    assert_eq!(free.exit_code, 0);
    assert_eq!(free.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(free.envelope.evidence, None);
    assert!(free.to_json().contains("\"status\":\"DEADLOCK_FREE\""));

    let _ = fs::remove_dir_all(found_root);
    let _ = fs::remove_dir_all(free_root);
}

#[test]
fn deadlock_job_reports_model_cutoff_without_fabricating_terminal_evidence() {
    let root = fixture_dir("cutoff");
    let manifest = write_deadlock_job(
        &root,
        deadlock_free_model(),
        "\"legitimate\"\n",
        "max-model-transitions 0\n",
    );
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 3);
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Inconclusive);
    let cutoff = run.envelope.cutoff.expect("bounded run should expose cutoff");
    assert_eq!(cutoff.stage, VerificationJobCutoffStage::Model);
    assert_eq!(cutoff.kind, VerificationJobCutoffKind::TransitionLimit);
    assert_eq!(cutoff.limit, 0);
    assert_eq!(run.envelope.accounting.explored_model_transitions, Some(0));
    assert_eq!(run.envelope.accounting.product_states, None);
    assert_eq!(run.envelope.evidence, None);
    assert!(run.to_json().contains("\"status\":\"INCONCLUSIVE\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn deadlock_jobs_reject_temporal_only_configuration_before_file_io() {
    for (kind, tail, expected_message) in [
        (
            "fairness",
            "strong-fair-action \"tick\"\n",
            "deadlock verification jobs do not support weak-fair-action or strong-fair-action directives",
        ),
        (
            "product-limit",
            "max-product-depth 1\n",
            "deadlock verification jobs do not support max-product-* limits",
        ),
    ] {
        let root = fixture_dir(kind);
        let manifest = root.join("job.fvj");
        fs::write(
            &manifest,
            format!(
                "analysis \"deadlock\"\nmodel \"missing-model.fvl\"\nproperty \"missing-property.fvp\"\n{tail}"
            ),
        )
        .unwrap();

        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 2);
        assert_eq!(run.envelope.analysis.as_deref(), Some("deadlock"));
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
        assert_eq!(run.envelope.error.as_deref(), Some(expected_message));
        assert!(run.envelope.model.is_none());
        assert!(run.envelope.property.is_none());

        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn deadlock_job_resolves_all_proposition_references_before_backend_execution() {
    let root = fixture_dir("unknown-proposition");
    let manifest = write_deadlock_job(&root, deadlock_free_model(), "\"missing\"\n", "");
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.analysis.as_deref(), Some("deadlock"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
    assert!(run
        .envelope
        .error
        .as_deref()
        .is_some_and(|message| message.contains("missing")));
    assert_eq!(run.envelope.evidence, None);

    let _ = fs::remove_dir_all(root);
}

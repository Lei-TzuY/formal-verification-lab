use formal_verification_lab::{
    check_action_temporal_with_fairness_profile_and_limits, parse_action_temporal,
    parse_declarative_model, parse_verification_job, run_verification_job_json, AnalysisLimits,
    AnalysisOutcome, AnalysisStage, FairnessProfile, TemporalBackend, TemporalCounterexample,
    TemporalStatus, VerificationJobAnalysis, VerificationJobCutoffKind, VerificationJobCutoffStage,
    VerificationJobEvidence, VerificationJobOutcome,
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m62-action-temporal-job-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn request_grant_model() -> &'static str {
    "model \"request-grant\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn unfair_request_grant_model() -> &'static str {
    "model \"request-grant-unfair\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"wait\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn pulse_model() -> &'static str {
    "model \"pulses\"\nstate \"first\"\nstate \"second\"\ninitial \"first\"\nedge \"first\" \"pulse-a\" \"second\"\nedge \"second\" \"pulse-b\" \"first\"\n"
}

fn write_job(root: &Path, model: &str, property: &str, tail: &str) -> PathBuf {
    fs::write(root.join("model.fvl"), model).unwrap();
    fs::write(root.join("property.fvp"), property).unwrap();
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        format!(
            "analysis \"action-temporal\"\nmodel \"model.fvl\"\nproperty \"property.fvp\"\n{tail}"
        ),
    )
    .unwrap();
    manifest
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

#[test]
fn action_temporal_analysis_round_trips_in_job_manifest() {
    let source = "analysis \"action-temporal\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"";
    let job = parse_verification_job(source).unwrap();
    assert_eq!(job.analysis(), VerificationJobAnalysis::ActionTemporal);
    assert_eq!(
        job.declared_analysis(),
        Some(VerificationJobAnalysis::ActionTemporal)
    );
    assert_eq!(job.canonical_document(), source);
    assert_eq!(
        parse_verification_job(&job.canonical_document()).unwrap(),
        job
    );
}

#[test]
fn response_job_matches_direct_staged_frontend_and_exposes_backend() {
    let root = fixture_dir("response-direct");
    let manifest = write_job(
        &root,
        request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "",
    );
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.schema_version,
        VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION
    );
    assert_eq!(run.envelope.analysis.as_deref(), Some("action-temporal"));
    assert_eq!(run.envelope.backend.as_deref(), Some("response"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(
        run.envelope.property.as_deref(),
        Some("response(\"request\",\"grant\")")
    );

    let model = parse_declarative_model(request_grant_model()).unwrap();
    let spec = parse_action_temporal(
        "verification-job-action-temporal",
        "response(\"request\",\"grant\")",
    )
    .unwrap();
    let direct = check_action_temporal_with_fairness_profile_and_limits(
        &model,
        &spec,
        &FairnessProfile::none(),
        AnalysisLimits::default(),
    )
    .unwrap();
    assert_eq!(direct.backend, TemporalBackend::Response);
    assert_eq!(
        direct.outcome,
        AnalysisOutcome::Conclusive(TemporalStatus::Satisfied)
    );
    assert_eq!(
        run.envelope.accounting.model_states,
        Some(direct.model_states)
    );
    assert_eq!(
        run.envelope.accounting.checked_model_states,
        Some(direct.checked_model_states)
    );
    assert_eq!(
        run.envelope.accounting.explored_model_transitions,
        Some(direct.explored_model_transitions)
    );
    assert_eq!(
        run.envelope.accounting.product_states,
        Some(direct.product_states)
    );
    assert_eq!(
        run.envelope.accounting.checked_product_states,
        Some(direct.checked_product_states)
    );
    assert_eq!(
        run.envelope.accounting.explored_product_transitions,
        Some(direct.explored_product_transitions)
    );
    assert!(run.envelope.evidence.is_none());
    assert!(run.to_json().contains("\"backend\":\"response\""));
}

#[test]
fn recurring_job_uses_buchi_backend_and_canonical_property() {
    let root = fixture_dir("recurring");
    let manifest = write_job(
        &root,
        pulse_model(),
        " infinitely-often( \"pulse-a\" , \"pulse-b\" ) \n",
        "",
    );
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.backend.as_deref(), Some("buchi"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(
        run.envelope.property.as_deref(),
        Some("infinitely-often(\"pulse-a\",\"pulse-b\")")
    );
}

#[test]
fn no_fair_response_violation_preserves_frontend_lasso_only() {
    let root = fixture_dir("response-lasso");
    let manifest = write_job(
        &root,
        unfair_request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "",
    );
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 10);
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Violated);
    let Some(VerificationJobEvidence::ActionTemporalInfinite {
        obligation,
        stem,
        cycle,
    }) = &run.envelope.evidence
    else {
        panic!("expected normalized action-temporal lasso");
    };
    assert_eq!(obligation, "response");
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("wait")));
    assert!(!run.to_json().contains("pending"));
}

#[test]
fn weak_fair_grant_excludes_unfair_response_cycle() {
    let root = fixture_dir("response-fair");
    let manifest = write_job(
        &root,
        unfair_request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "weak-fair-action \"grant\"\n",
    );
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(run.envelope.weak_fair_actions, vec!["grant"]);
    assert!(run.envelope.strong_fair_actions.is_empty());
}

#[test]
fn fairness_overlap_is_canonicalized_to_strong_in_machine_result() {
    let root = fixture_dir("fairness-overlap");
    let manifest = write_job(
        &root,
        unfair_request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "weak-fair-action \"grant\"\nstrong-fair-action \"grant\"\n",
    );
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 0);
    assert!(run.envelope.weak_fair_actions.is_empty());
    assert_eq!(run.envelope.strong_fair_actions, vec!["grant"]);
}

#[test]
fn staged_model_and_product_cutoffs_keep_exact_stage_provenance() {
    let model_root = fixture_dir("model-cutoff");
    let model_manifest = write_job(
        &model_root,
        request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "max-model-transitions 0\n",
    );
    let model_run = run_verification_job_json(&model_manifest);
    assert_eq!(model_run.exit_code, 3);
    assert_eq!(
        model_run.envelope.outcome,
        VerificationJobOutcome::Inconclusive
    );
    let model_cutoff = model_run.envelope.cutoff.expect("model cutoff");
    assert_eq!(model_cutoff.stage, VerificationJobCutoffStage::Model);
    assert_eq!(
        model_cutoff.kind,
        VerificationJobCutoffKind::TransitionLimit
    );
    assert_eq!(model_cutoff.limit, 0);

    let product_root = fixture_dir("product-cutoff");
    let product_manifest = write_job(
        &product_root,
        request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "max-product-states 1\n",
    );
    let product_run = run_verification_job_json(&product_manifest);
    assert_eq!(product_run.exit_code, 3);
    assert_eq!(
        product_run.envelope.outcome,
        VerificationJobOutcome::Inconclusive
    );
    let product_cutoff = product_run.envelope.cutoff.expect("product cutoff");
    assert_eq!(product_cutoff.stage, VerificationJobCutoffStage::Product);
    assert_eq!(product_cutoff.kind, VerificationJobCutoffKind::StateLimit);
    assert_eq!(product_cutoff.limit, 1);

    let model = parse_declarative_model(request_grant_model()).unwrap();
    let spec = parse_action_temporal(
        "verification-job-action-temporal",
        "response(\"request\",\"grant\")",
    )
    .unwrap();
    let direct = check_action_temporal_with_fairness_profile_and_limits(
        &model,
        &spec,
        &FairnessProfile::none(),
        AnalysisLimits {
            model: Default::default(),
            product: formal_verification_lab::ExplorationLimits {
                max_states: Some(1),
                ..Default::default()
            },
        },
    )
    .unwrap();
    let AnalysisOutcome::Inconclusive(reason) = direct.outcome else {
        panic!("direct frontend should be product-inconclusive");
    };
    assert_eq!(reason.stage, AnalysisStage::Product);
    assert_eq!(
        product_run.envelope.accounting.explored_product_transitions,
        Some(direct.explored_product_transitions)
    );
}

#[test]
fn malformed_temporal_property_fails_closed_as_action_temporal_error() {
    let root = fixture_dir("malformed");
    let manifest = write_job(&root, request_grant_model(), "always(\"grant\")\n", "");
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.analysis.as_deref(), Some("action-temporal"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
    assert!(run.envelope.backend.is_none());
    assert!(run.envelope.error.unwrap().contains("unsupported"));
}

#[test]
fn built_binary_emits_action_temporal_schema_and_temporal_exit_code() {
    let root = fixture_dir("binary");
    let manifest = write_job(
        &root,
        unfair_request_grant_model(),
        "response(\"request\",\"grant\")\n",
        "",
    );
    let output = run_job_binary(&manifest);
    assert_eq!(output.status.code(), Some(10));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"analysis\":\"action-temporal\""));
    assert!(stdout.contains("\"backend\":\"response\""));
    assert!(stdout.contains("\"kind\":\"temporal_lasso\""));
    assert!(!stdout.contains("pending"));
}

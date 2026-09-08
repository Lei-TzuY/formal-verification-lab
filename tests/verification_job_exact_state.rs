use formal_verification_lab::{
    check_exact_state_property_with_limits, parse_declarative_model, parse_exact_state_property,
    parse_verification_job, run_verification_job_json, BoundedOutcome, ExactStateEvidence,
    ExactStateStatus, ExplorationLimits, VerificationJobAnalysis, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobOutcome,
    VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m60-exact-state-job-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn chain_model() -> &'static str {
    "model \"chain\"\nstate \"start\"\nstate \"middle\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"advance\" \"middle\"\nedge \"middle\" \"finish\" \"done\"\n"
}

fn avoiding_cycle_model() -> &'static str {
    "model \"avoiding-cycle\"\nstate \"start\"\nstate \"loop\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"enter\" \"loop\"\nedge \"loop\" \"spin\" \"loop\"\nedge \"loop\" \"finish\" \"done\"\n"
}

fn finite_failure_model() -> &'static str {
    "model \"finite-failure\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\n"
}

fn write_exact_state_job(root: &Path, model: &str, property: &str, tail: &str) -> PathBuf {
    fs::write(root.join("model.fvl"), model).unwrap();
    fs::write(root.join("property.fvp"), property).unwrap();
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        format!(
            "analysis \"exact-state\"\nmodel \"model.fvl\"\nproperty \"property.fvp\"\n{tail}"
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
fn exact_state_analysis_round_trips_in_job_manifest() {
    let source = "analysis \"exact-state\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"";
    let job = parse_verification_job(source).unwrap();

    assert_eq!(job.analysis(), VerificationJobAnalysis::ExactState);
    assert_eq!(
        job.declared_analysis(),
        Some(VerificationJobAnalysis::ExactState)
    );
    assert_eq!(job.canonical_document(), source);
    assert_eq!(parse_verification_job(&job.canonical_document()).unwrap(), job);
}

#[test]
fn reachable_job_matches_direct_bounded_exact_state_backend_and_preserves_witness() {
    let root = fixture_dir("reachable");
    let manifest = write_exact_state_job(&root, chain_model(), "reachable(\"done\")\n", "");
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.schema_version,
        VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION
    );
    assert_eq!(run.envelope.analysis.as_deref(), Some("exact-state"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(run.envelope.model.as_deref(), Some("chain"));
    assert_eq!(run.envelope.property.as_deref(), Some("reachable(\"done\")"));
    assert!(run.envelope.weak_fair_actions.is_empty());
    assert!(run.envelope.strong_fair_actions.is_empty());
    assert_eq!(run.envelope.accounting.product_states, None);
    assert_eq!(run.envelope.cutoff, None);

    let model = parse_declarative_model(chain_model()).unwrap();
    let spec = parse_exact_state_property("verification-job-exact-state", "reachable(\"done\")")
        .unwrap();
    let direct = check_exact_state_property_with_limits(
        &model,
        &spec,
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        direct.outcome,
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied)
    );
    assert_eq!(
        run.envelope.accounting.model_states,
        Some(direct.discovered_states)
    );
    assert_eq!(
        run.envelope.accounting.checked_model_states,
        Some(direct.checked_states)
    );
    assert_eq!(
        run.envelope.accounting.explored_model_transitions,
        Some(direct.explored_transitions)
    );

    let Some(ExactStateEvidence::ReachabilityWitness { trace: direct_trace }) = direct.evidence
    else {
        panic!("direct backend should return reachability witness");
    };
    let Some(VerificationJobEvidence::ExactStateReachability { trace }) = run.envelope.evidence
    else {
        panic!("job envelope should preserve reachability witness");
    };
    assert_eq!(trace.len(), direct_trace.len());
    for (actual, expected) in trace.iter().zip(direct_trace.iter()) {
        assert_eq!(actual.action, expected.action);
        assert_eq!(actual.state, expected.state);
    }
    assert!(!run.to_json().contains("\"pending\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn unreachable_and_resource_cutoff_remain_distinct_violation_and_inconclusive_results() {
    let missing_root = fixture_dir("unreachable");
    let missing_manifest =
        write_exact_state_job(&missing_root, chain_model(), "reachable(\"missing\")\n", "");
    let missing = run_verification_job_json(&missing_manifest);
    assert_eq!(missing.exit_code, 11);
    assert_eq!(missing.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(missing.envelope.cutoff, None);
    assert_eq!(missing.envelope.evidence, None);

    let cutoff_root = fixture_dir("cutoff");
    let cutoff_manifest = write_exact_state_job(
        &cutoff_root,
        chain_model(),
        "reachable(\"done\")\n",
        "max-model-transitions 0\n",
    );
    let cutoff = run_verification_job_json(&cutoff_manifest);
    assert_eq!(cutoff.exit_code, 3);
    assert_eq!(
        cutoff.envelope.outcome,
        VerificationJobOutcome::Inconclusive
    );
    let cutoff_reason = cutoff
        .envelope
        .cutoff
        .expect("bounded exact-state job should expose cutoff");
    assert_eq!(cutoff_reason.stage, VerificationJobCutoffStage::Model);
    assert_eq!(
        cutoff_reason.kind,
        VerificationJobCutoffKind::TransitionLimit
    );
    assert_eq!(cutoff_reason.limit, 0);
    assert_eq!(cutoff.envelope.accounting.explored_model_transitions, Some(0));
    assert_eq!(cutoff.envelope.accounting.product_states, None);
    assert_eq!(cutoff.envelope.evidence, None);

    let _ = fs::remove_dir_all(missing_root);
    let _ = fs::remove_dir_all(cutoff_root);
}

#[test]
fn initial_reachability_witness_is_conclusive_before_zero_transition_cutoff() {
    let root = fixture_dir("initial-witness");
    let manifest = write_exact_state_job(
        &root,
        chain_model(),
        "reachable(\"start\")\n",
        "max-model-transitions 0\n",
    );
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 0);
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(run.envelope.cutoff, None);
    assert_eq!(run.envelope.accounting.explored_model_transitions, Some(0));
    let Some(VerificationJobEvidence::ExactStateReachability { trace }) = run.envelope.evidence
    else {
        panic!("expected initial-state reachability witness");
    };
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].action, None);
    assert_eq!(trace[0].state, "start");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn eventuality_jobs_preserve_finite_and_lasso_counterexamples() {
    let finite_root = fixture_dir("finite-eventuality");
    let finite_manifest = write_exact_state_job(
        &finite_root,
        finite_failure_model(),
        "all-eventually(\"done\")\n",
        "",
    );
    let finite = run_verification_job_json(&finite_manifest);
    assert_eq!(finite.exit_code, 11);
    assert_eq!(finite.envelope.outcome, VerificationJobOutcome::Violated);
    let Some(VerificationJobEvidence::ExactStateEventualityFinite { trace }) =
        finite.envelope.evidence
    else {
        panic!("expected finite eventuality counterexample");
    };
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].state, "start");

    let cycle_root = fixture_dir("cycle-eventuality");
    let cycle_manifest = write_exact_state_job(
        &cycle_root,
        avoiding_cycle_model(),
        "all-eventually(\"done\")\n",
        "",
    );
    let cycle = run_verification_job_json(&cycle_manifest);
    assert_eq!(cycle.exit_code, 11);
    assert_eq!(cycle.envelope.outcome, VerificationJobOutcome::Violated);
    let Some(VerificationJobEvidence::ExactStateEventualityInfinite { stem, cycle }) =
        cycle.envelope.evidence
    else {
        panic!("expected lasso eventuality counterexample");
    };
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert!(cycle.iter().any(|step| step.action.as_deref() == Some("spin")));

    let _ = fs::remove_dir_all(finite_root);
    let _ = fs::remove_dir_all(cycle_root);
}

#[test]
fn exact_state_jobs_reject_temporal_only_configuration_before_reading_inputs() {
    for (kind, tail, expected_message) in [
        (
            "fairness",
            "strong-fair-action \"tick\"\n",
            "exact-state verification jobs do not support weak-fair-action or strong-fair-action directives",
        ),
        (
            "product-limit",
            "max-product-depth 1\n",
            "exact-state verification jobs do not support max-product-* limits",
        ),
    ] {
        let root = fixture_dir(kind);
        let manifest = root.join("job.fvj");
        fs::write(
            &manifest,
            format!(
                "analysis \"exact-state\"\nmodel \"missing-model.fvl\"\nproperty \"missing-property.fvp\"\n{tail}"
            ),
        )
        .unwrap();

        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 2);
        assert_eq!(
            run.envelope.schema_version,
            VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION
        );
        assert_eq!(run.envelope.analysis.as_deref(), Some("exact-state"));
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
        assert_eq!(run.envelope.error.as_deref(), Some(expected_message));
        assert_eq!(run.envelope.model, None);
        assert_eq!(run.envelope.property, None);

        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn built_binary_emits_exact_state_schema_and_native_exit_codes() {
    let root = fixture_dir("cli");
    let manifest = write_exact_state_job(&root, chain_model(), "reachable(\"done\")\n", "");
    let output = run_job_binary(&manifest);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let json = String::from_utf8(output.stdout).unwrap();
    assert!(json.starts_with(
        "{\"schema_version\":2,\"analysis\":\"exact-state\",\"outcome\":\"satisfied\",\"status\":\"SATISFIED\""
    ));
    assert!(json.contains("\"kind\":\"reachability_witness\""));
    assert!(json.contains("\"action\":\"finish\",\"state\":\"done\""));
    assert!(!json.contains("\"pending\""));

    let violated = write_exact_state_job(
        &root,
        chain_model(),
        "reachable(\"missing\")\n",
        "",
    );
    let output = run_job_binary(&violated);
    assert_eq!(output.status.code(), Some(11));
    assert!(output.stderr.is_empty());
    let json = String::from_utf8(output.stdout).unwrap();
    assert!(json.contains("\"outcome\":\"violated\""));
    assert!(json.contains("\"evidence\":null"));

    let _ = fs::remove_dir_all(root);
}

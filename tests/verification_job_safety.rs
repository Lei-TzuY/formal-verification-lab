use formal_verification_lab::{
    check_safety_assertion_with_limits, parse_declarative_document, parse_proposition_expression,
    parse_verification_job, run_verification_job_json, BoundedOutcome, ExplorationLimits,
    PropositionSafetySpec, SafetyStatus, VerificationJobAnalysis, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobOutcome,
    VerificationJobParseErrorKind, VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("fvlab-m59-safety-job-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn safe_model() -> &'static str {
    "model \"safe\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"finish\" \"done\"\nlabel \"start\" \"ok\"\nlabel \"done\" \"ok\"\n"
}

fn violated_model() -> &'static str {
    "model \"violated\"\nstate \"start\"\nstate \"bad\"\ninitial \"start\"\nedge \"start\" \"break\" \"bad\"\nlabel \"start\" \"ok\"\n"
}

fn initial_violation_model() -> &'static str {
    "model \"initial-bad\"\nstate \"bad\"\nstate \"later\"\ninitial \"bad\"\nedge \"bad\" \"next\" \"later\"\nlabel \"later\" \"ok\"\n"
}

fn write_safety_job(root: &Path, model: &str, tail: &str) -> PathBuf {
    fs::write(root.join("model.fvl"), model).unwrap();
    fs::write(root.join("property.fvp"), "\"ok\"").unwrap();
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        format!("analysis \"safety\"\nmodel \"model.fvl\"\nproperty \"property.fvp\"\n{tail}"),
    )
    .unwrap();
    manifest
}

#[test]
fn historical_manifest_remains_implicit_multi_response_and_canonicalizes_unchanged() {
    let source = "model \"models/protocol.fvl\"\nproperty \"properties/response.fvt\"";
    let job = parse_verification_job(source).unwrap();

    assert_eq!(job.analysis(), VerificationJobAnalysis::MultiResponse);
    assert_eq!(job.declared_analysis(), None);
    assert_eq!(job.canonical_document(), source);
}

#[test]
fn explicit_analysis_round_trips_and_unknown_or_duplicate_family_fails_closed() {
    for (name, expected) in [
        ("multi-response", VerificationJobAnalysis::MultiResponse),
        ("safety", VerificationJobAnalysis::Safety),
    ] {
        let source = format!("analysis \"{name}\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"");
        let job = parse_verification_job(&source).unwrap();
        assert_eq!(job.analysis(), expected);
        assert_eq!(job.declared_analysis(), Some(expected));
        assert_eq!(job.canonical_document(), source);
        assert_eq!(
            parse_verification_job(&job.canonical_document()).unwrap(),
            job
        );
    }

    let unknown = parse_verification_job("analysis \"ctl\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"\n")
        .unwrap_err();
    assert_eq!(unknown.line(), 1);
    assert_eq!(
        unknown.kind(),
        &VerificationJobParseErrorKind::InvalidAnalysis {
            analysis: "ctl".to_owned(),
        }
    );

    let duplicate = parse_verification_job(
        "analysis \"safety\"\nanalysis \"multi-response\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"\n",
    )
    .unwrap_err();
    assert_eq!(duplicate.line(), 2);
    assert_eq!(
        duplicate.kind(),
        &VerificationJobParseErrorKind::DuplicateDirective {
            directive: "analysis".to_owned(),
        }
    );
}

#[test]
fn safety_job_matches_direct_m24_backend_for_safe_violated_and_inconclusive_cases() {
    let safe_root = fixture_dir("safe");
    let safe_manifest = write_safety_job(&safe_root, safe_model(), "");
    let safe_run = run_verification_job_json(&safe_manifest);
    assert_eq!(safe_run.exit_code, 0);
    assert_eq!(
        safe_run.envelope.schema_version,
        VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION
    );
    assert_eq!(safe_run.envelope.analysis.as_deref(), Some("safety"));
    assert_eq!(safe_run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(safe_run.envelope.model.as_deref(), Some("safe"));
    assert_eq!(safe_run.envelope.property.as_deref(), Some("\"ok\""));
    assert!(safe_run.envelope.weak_fair_actions.is_empty());
    assert!(safe_run.envelope.strong_fair_actions.is_empty());
    assert_eq!(safe_run.envelope.accounting.product_states, None);
    assert_eq!(safe_run.envelope.cutoff, None);
    assert_eq!(safe_run.envelope.evidence, None);
    assert!(safe_run.to_json().starts_with(
        "{\"schema_version\":2,\"analysis\":\"safety\",\"outcome\":\"satisfied\",\"status\":\"SAFE\""
    ));

    let safe_document = parse_declarative_document(safe_model()).unwrap();
    let safe_spec = PropositionSafetySpec::always(
        "verification-job-safety",
        parse_proposition_expression("\"ok\"").unwrap(),
    )
    .unwrap();
    let safe_direct = check_safety_assertion_with_limits(
        &safe_document,
        &safe_spec,
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        safe_direct.outcome,
        BoundedOutcome::Conclusive(SafetyStatus::Safe)
    );
    assert_eq!(
        safe_run.envelope.accounting.model_states,
        Some(safe_direct.discovered_states)
    );
    assert_eq!(
        safe_run.envelope.accounting.checked_model_states,
        Some(safe_direct.checked_states)
    );
    assert_eq!(
        safe_run.envelope.accounting.explored_model_transitions,
        Some(safe_direct.explored_transitions)
    );

    let violated_root = fixture_dir("violated");
    let violated_manifest = write_safety_job(&violated_root, violated_model(), "");
    let violated_run = run_verification_job_json(&violated_manifest);
    assert_eq!(violated_run.exit_code, 12);
    assert_eq!(
        violated_run.envelope.outcome,
        VerificationJobOutcome::Violated
    );
    let Some(VerificationJobEvidence::Safety { trace }) = &violated_run.envelope.evidence else {
        panic!("expected structured safety trace");
    };
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].action, None);
    assert_eq!(trace[0].state, "start");
    assert_eq!(trace[1].action.as_deref(), Some("break"));
    assert_eq!(trace[1].state, "bad");
    assert!(!violated_run.to_json().contains("\"pending\""));

    let bounded_root = fixture_dir("inconclusive");
    let bounded_manifest =
        write_safety_job(&bounded_root, safe_model(), "max-model-transitions 0\n");
    let bounded_run = run_verification_job_json(&bounded_manifest);
    assert_eq!(bounded_run.exit_code, 3);
    assert_eq!(
        bounded_run.envelope.outcome,
        VerificationJobOutcome::Inconclusive
    );
    let cutoff = bounded_run
        .envelope
        .cutoff
        .expect("bounded run should expose cutoff");
    assert_eq!(cutoff.stage, VerificationJobCutoffStage::Model);
    assert_eq!(cutoff.kind, VerificationJobCutoffKind::TransitionLimit);
    assert_eq!(cutoff.limit, 0);
    assert_eq!(bounded_run.envelope.accounting.product_states, None);
    assert_eq!(bounded_run.envelope.evidence, None);

    let bounded_document = parse_declarative_document(safe_model()).unwrap();
    let bounded_spec = PropositionSafetySpec::always(
        "verification-job-safety",
        parse_proposition_expression("\"ok\"").unwrap(),
    )
    .unwrap();
    let bounded_direct = check_safety_assertion_with_limits(
        &bounded_document,
        &bounded_spec,
        ExplorationLimits {
            max_states: None,
            max_transitions: Some(0),
            max_depth: None,
        },
    )
    .unwrap();
    assert_eq!(
        bounded_run.envelope.accounting.model_states,
        Some(bounded_direct.discovered_states)
    );
    assert_eq!(
        bounded_run.envelope.accounting.checked_model_states,
        Some(bounded_direct.checked_states)
    );
    assert_eq!(
        bounded_run.envelope.accounting.explored_model_transitions,
        Some(bounded_direct.explored_transitions)
    );

    let _ = fs::remove_dir_all(safe_root);
    let _ = fs::remove_dir_all(violated_root);
    let _ = fs::remove_dir_all(bounded_root);
}

#[test]
fn initial_safety_counterexample_remains_conclusive_before_zero_transition_cutoff() {
    let root = fixture_dir("initial-violation");
    let manifest = write_safety_job(
        &root,
        initial_violation_model(),
        "max-model-transitions 0\n",
    );
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 12);
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(run.envelope.cutoff, None);
    let Some(VerificationJobEvidence::Safety { trace }) = run.envelope.evidence else {
        panic!("expected initial-state safety trace");
    };
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].action, None);
    assert_eq!(trace[0].state, "bad");
    assert_eq!(run.envelope.accounting.explored_model_transitions, Some(0));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn safety_jobs_reject_temporal_only_fairness_and_product_limits_before_execution() {
    for (kind, tail, expected_message) in [
        (
            "fairness",
            "weak-fair-action \"grant\"\n",
            "safety verification jobs do not support weak-fair-action or strong-fair-action directives",
        ),
        (
            "product-limit",
            "max-product-states 1\n",
            "safety verification jobs do not support max-product-* limits",
        ),
    ] {
        let root = fixture_dir(kind);
        let manifest = root.join("job.fvj");
        fs::write(
            &manifest,
            format!(
                "analysis \"safety\"\nmodel \"missing-model.fvl\"\nproperty \"missing-property.fvp\"\n{tail}"
            ),
        )
        .unwrap();

        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 2);
        assert_eq!(
            run.envelope.schema_version,
            VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION
        );
        assert_eq!(run.envelope.analysis.as_deref(), Some("safety"));
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
        assert_eq!(run.envelope.error.as_deref(), Some(expected_message));
        assert!(run.envelope.model.is_none());
        assert!(run.envelope.property.is_none());

        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn explicit_multi_response_keeps_schema_v1_and_historical_json_shape() {
    let root = fixture_dir("explicit-temporal");
    fs::write(
        root.join("model.fvl"),
        "model \"temporal\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n",
    )
    .unwrap();
    fs::write(
        root.join("property.fvt"),
        "response(\"request\",\"request\",\"grant\")\n",
    )
    .unwrap();
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        "analysis \"multi-response\"\nmodel \"model.fvl\"\nproperty \"property.fvt\"\n",
    )
    .unwrap();

    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.schema_version,
        VERIFICATION_JOB_RESULT_SCHEMA_VERSION
    );
    assert_eq!(run.envelope.analysis, None);
    assert!(run
        .to_json()
        .starts_with("{\"schema_version\":1,\"outcome\":\"satisfied\",\"status\":\"SATISFIED\""));
    assert!(!run.to_json().contains("\"analysis\""));

    let _ = fs::remove_dir_all(root);
}

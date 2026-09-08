use formal_verification_lab::{
    check_proposition_expression_property_with_limits, parse_declarative_document,
    parse_proposition_expression, parse_verification_job, run_verification_job_json,
    BoundedOutcome, ExactStateEvidence, ExactStateStatus, ExplorationLimits,
    PropositionExpressionPropertySpec, VerificationJobAnalysis, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobOutcome,
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
        "fvlab-m61-proposition-job-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn proposition_model() -> &'static str {
    "model \"proposition-chain\"\nstate \"start\"\nstate \"middle\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"advance\" \"middle\"\nedge \"middle\" \"finish\" \"done\"\nlabel \"start\" \"entry\"\nlabel \"start\" \"safe\"\nlabel \"middle\" \"work\"\nlabel \"middle\" \"safe\"\nlabel \"done\" \"complete\"\nlabel \"done\" \"safe\"\n"
}

fn finite_failure_model() -> &'static str {
    "model \"finite-proposition-failure\"\nstate \"start\"\nstate \"done\"\ninitial \"start\"\nlabel \"start\" \"entry\"\nlabel \"done\" \"complete\"\n"
}

fn avoiding_cycle_model() -> &'static str {
    "model \"proposition-cycle\"\nstate \"start\"\nstate \"loop\"\nstate \"done\"\ninitial \"start\"\nedge \"start\" \"enter\" \"loop\"\nedge \"loop\" \"spin\" \"loop\"\nedge \"loop\" \"finish\" \"done\"\nlabel \"start\" \"entry\"\nlabel \"loop\" \"work\"\nlabel \"done\" \"complete\"\n"
}

fn write_job(root: &Path, model: &str, property: &str, tail: &str) -> PathBuf {
    fs::write(root.join("model.fvl"), model).unwrap();
    fs::write(root.join("property.fvp"), property).unwrap();
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        format!(
            "analysis \"proposition-expression\"\nmodel \"model.fvl\"\nproperty \"property.fvp\"\n{tail}"
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

fn direct_spec(mode: &str, expression: &str) -> PropositionExpressionPropertySpec {
    let expression = parse_proposition_expression(expression).unwrap();
    match mode {
        "reachable" => PropositionExpressionPropertySpec::reachable(
            "verification-job-proposition-expression",
            expression,
        )
        .unwrap(),
        "all-eventually" => PropositionExpressionPropertySpec::all_eventually(
            "verification-job-proposition-expression",
            expression,
        )
        .unwrap(),
        _ => panic!("test mode is supported"),
    }
}

fn assert_evidence_matches(job: &Option<VerificationJobEvidence>, direct: &Option<ExactStateEvidence>) {
    match (job, direct) {
        (None, None) => {}
        (
            Some(VerificationJobEvidence::ExactStateReachability { trace: actual }),
            Some(ExactStateEvidence::ReachabilityWitness { trace: expected }),
        )
        | (
            Some(VerificationJobEvidence::ExactStateEventualityFinite { trace: actual }),
            Some(ExactStateEvidence::EventualityFiniteCounterexample { trace: expected }),
        ) => {
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert_eq!(actual.action, expected.action);
                assert_eq!(actual.state, expected.state);
            }
        }
        (
            Some(VerificationJobEvidence::ExactStateEventualityInfinite {
                stem: actual_stem,
                cycle: actual_cycle,
            }),
            Some(ExactStateEvidence::EventualityInfiniteCounterexample {
                stem: expected_stem,
                cycle: expected_cycle,
            }),
        ) => {
            assert_eq!(actual_stem.len(), expected_stem.len());
            assert_eq!(actual_cycle.len(), expected_cycle.len());
            for (actual, expected) in actual_stem.iter().zip(expected_stem.iter()) {
                assert_eq!(actual.action, expected.action);
                assert_eq!(actual.state, expected.state);
            }
            for (actual, expected) in actual_cycle.iter().zip(expected_cycle.iter()) {
                assert_eq!(actual.action, expected.action);
                assert_eq!(actual.state, expected.state);
            }
        }
        other => panic!("job/direct evidence mismatch: {other:?}"),
    }
}

#[test]
fn proposition_expression_analysis_round_trips_in_job_manifest() {
    let source =
        "analysis \"proposition-expression\"\nmodel \"m.fvl\"\nproperty \"p.fvp\"";
    let job = parse_verification_job(source).unwrap();

    assert_eq!(job.analysis(), VerificationJobAnalysis::PropositionExpression);
    assert_eq!(
        job.declared_analysis(),
        Some(VerificationJobAnalysis::PropositionExpression)
    );
    assert_eq!(job.canonical_document(), source);
    assert_eq!(
        parse_verification_job(&job.canonical_document()).unwrap(),
        job
    );
}

#[test]
fn reachable_boolean_job_matches_direct_m24_backend_and_preserves_shortest_witness() {
    let root = fixture_dir("reachable");
    let manifest = write_job(
        &root,
        proposition_model(),
        "reachable \"work\" or \"complete\"\n",
        "",
    );
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.schema_version,
        VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        run.envelope.analysis.as_deref(),
        Some("proposition-expression")
    );
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(run.envelope.model.as_deref(), Some("proposition-chain"));
    assert_eq!(
        run.envelope.property.as_deref(),
        Some("reachable (\"work\" or \"complete\")")
    );
    assert!(run.envelope.weak_fair_actions.is_empty());
    assert!(run.envelope.strong_fair_actions.is_empty());
    assert_eq!(run.envelope.accounting.product_states, None);
    assert_eq!(run.envelope.cutoff, None);

    let document = parse_declarative_document(proposition_model()).unwrap();
    let direct = check_proposition_expression_property_with_limits(
        &document,
        &direct_spec("reachable", "\"work\" or \"complete\""),
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        direct.outcome,
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied)
    );
    assert_eq!(run.envelope.accounting.model_states, Some(direct.discovered_states));
    assert_eq!(
        run.envelope.accounting.checked_model_states,
        Some(direct.checked_states)
    );
    assert_eq!(
        run.envelope.accounting.explored_model_transitions,
        Some(direct.explored_transitions)
    );
    assert_evidence_matches(&run.envelope.evidence, &direct.evidence);

    let Some(VerificationJobEvidence::ExactStateReachability { trace }) = run.envelope.evidence
    else {
        panic!("expected reachability witness");
    };
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].state, "start");
    assert_eq!(trace[1].action.as_deref(), Some("advance"));
    assert_eq!(trace[1].state, "middle");
    assert!(!run.to_json().contains("\"pending\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn false_reachability_cutoff_and_initial_witness_remain_proof_honest() {
    let false_root = fixture_dir("false");
    let false_manifest = write_job(
        &false_root,
        proposition_model(),
        "reachable \"work\" and \"complete\"\n",
        "",
    );
    let false_run = run_verification_job_json(&false_manifest);
    assert_eq!(false_run.exit_code, 11);
    assert_eq!(false_run.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(false_run.envelope.evidence, None);
    assert_eq!(false_run.envelope.cutoff, None);

    let cutoff_root = fixture_dir("cutoff");
    let cutoff_manifest = write_job(
        &cutoff_root,
        proposition_model(),
        "reachable \"complete\"\n",
        "max-model-transitions 0\n",
    );
    let cutoff = run_verification_job_json(&cutoff_manifest);
    assert_eq!(cutoff.exit_code, 3);
    assert_eq!(cutoff.envelope.outcome, VerificationJobOutcome::Inconclusive);
    let cutoff_reason = cutoff.envelope.cutoff.expect("expected model cutoff");
    assert_eq!(cutoff_reason.stage, VerificationJobCutoffStage::Model);
    assert_eq!(
        cutoff_reason.kind,
        VerificationJobCutoffKind::TransitionLimit
    );
    assert_eq!(cutoff_reason.limit, 0);
    assert_eq!(cutoff.envelope.accounting.explored_model_transitions, Some(0));
    assert_eq!(cutoff.envelope.evidence, None);

    let initial_root = fixture_dir("initial");
    let initial_manifest = write_job(
        &initial_root,
        proposition_model(),
        "reachable \"entry\"\n",
        "max-model-transitions 0\n",
    );
    let initial = run_verification_job_json(&initial_manifest);
    assert_eq!(initial.exit_code, 0);
    assert_eq!(initial.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(initial.envelope.cutoff, None);
    let Some(VerificationJobEvidence::ExactStateReachability { trace }) = initial.envelope.evidence
    else {
        panic!("initial target should conclude with a witness");
    };
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].action, None);
    assert_eq!(trace[0].state, "start");

    let _ = fs::remove_dir_all(false_root);
    let _ = fs::remove_dir_all(cutoff_root);
    let _ = fs::remove_dir_all(initial_root);
}

#[test]
fn all_eventually_boolean_jobs_preserve_finite_and_lasso_counterexamples() {
    let finite_root = fixture_dir("finite");
    let finite_manifest = write_job(
        &finite_root,
        finite_failure_model(),
        "all-eventually \"complete\"\n",
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

    let cycle_root = fixture_dir("lasso");
    let cycle_manifest = write_job(
        &cycle_root,
        avoiding_cycle_model(),
        "all-eventually \"complete\"\n",
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
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("spin")));

    let _ = fs::remove_dir_all(finite_root);
    let _ = fs::remove_dir_all(cycle_root);
}

#[test]
fn malformed_mode_expression_unknown_reference_and_temporal_configuration_fail_closed() {
    for (kind, property, expected) in [
        (
            "mode",
            "eventually \"complete\"\n",
            "unsupported proposition-expression property mode 'eventually'",
        ),
        (
            "expression",
            "reachable \"complete\" and\n",
            "Boolean proposition parse error",
        ),
        (
            "unknown",
            "reachable \"ghost\"\n",
            "unknown proposition 'ghost'",
        ),
    ] {
        let root = fixture_dir(kind);
        let manifest = write_job(&root, proposition_model(), property, "");
        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 2);
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
        assert_eq!(
            run.envelope.analysis.as_deref(),
            Some("proposition-expression")
        );
        assert!(run.envelope.error.as_deref().unwrap().contains(expected));
        let _ = fs::remove_dir_all(root);
    }

    for (kind, tail, expected) in [
        (
            "fairness",
            "strong-fair-action \"tick\"\n",
            "proposition-expression verification jobs do not support weak-fair-action or strong-fair-action directives",
        ),
        (
            "product-limit",
            "max-product-depth 1\n",
            "proposition-expression verification jobs do not support max-product-* limits",
        ),
    ] {
        let root = fixture_dir(kind);
        let manifest = root.join("job.fvj");
        fs::write(
            &manifest,
            format!(
                "analysis \"proposition-expression\"\nmodel \"missing-model.fvl\"\nproperty \"missing-property.fvp\"\n{tail}"
            ),
        )
        .unwrap();
        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 2);
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Error);
        assert_eq!(run.envelope.model, None);
        assert_eq!(run.envelope.property, None);
        assert_eq!(run.envelope.error.as_deref(), Some(expected));
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn representative_boolean_expressions_and_limits_differentially_match_direct_backend() {
    let expressions = [
        "\"entry\"",
        "\"complete\"",
        "\"work\" or \"complete\"",
        "not \"complete\"",
        "\"safe\" and not \"complete\"",
    ];
    let limits = [
        ("", ExplorationLimits::unbounded()),
        (
            "max-model-transitions 0\n",
            ExplorationLimits {
                max_states: None,
                max_transitions: Some(0),
                max_depth: None,
            },
        ),
        (
            "max-model-depth 0\n",
            ExplorationLimits {
                max_states: None,
                max_transitions: None,
                max_depth: Some(0),
            },
        ),
        (
            "max-model-states 1\n",
            ExplorationLimits {
                max_states: Some(1),
                max_transitions: None,
                max_depth: None,
            },
        ),
    ];
    let document = parse_declarative_document(proposition_model()).unwrap();

    for mode in ["reachable", "all-eventually"] {
        for expression in expressions {
            for (tail, limit) in limits {
                let root = fixture_dir("differential");
                let property = format!("{mode} {expression}\n");
                let manifest = write_job(&root, proposition_model(), &property, tail);
                let run = run_verification_job_json(&manifest);
                let direct = check_proposition_expression_property_with_limits(
                    &document,
                    &direct_spec(mode, expression),
                    limit,
                )
                .unwrap();

                assert_eq!(
                    run.envelope.outcome,
                    match direct.outcome {
                        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied) => {
                            VerificationJobOutcome::Satisfied
                        }
                        BoundedOutcome::Conclusive(ExactStateStatus::Violated) => {
                            VerificationJobOutcome::Violated
                        }
                        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
                    },
                    "mode={mode} expression={expression} limits={limit:?}"
                );
                assert_eq!(run.envelope.accounting.model_states, Some(direct.discovered_states));
                assert_eq!(
                    run.envelope.accounting.checked_model_states,
                    Some(direct.checked_states)
                );
                assert_eq!(
                    run.envelope.accounting.explored_model_transitions,
                    Some(direct.explored_transitions)
                );
                assert_evidence_matches(&run.envelope.evidence, &direct.evidence);
                assert_eq!(
                    run.envelope.cutoff.as_ref().map(|cutoff| cutoff.limit),
                    direct
                        .outcome
                        .inconclusive_reason()
                        .map(|reason| match reason {
                            formal_verification_lab::InconclusiveReason::StateLimitReached {
                                limit,
                            }
                            | formal_verification_lab::InconclusiveReason::TransitionLimitReached {
                                limit,
                            }
                            | formal_verification_lab::InconclusiveReason::DepthLimitReached {
                                limit,
                            } => limit,
                        })
                );

                let _ = fs::remove_dir_all(root);
            }
        }
    }
}

#[test]
fn built_binary_emits_schema_v2_proposition_results_and_native_exit_codes() {
    let root = fixture_dir("cli");
    let manifest = write_job(
        &root,
        proposition_model(),
        "reachable \"complete\"\n",
        "",
    );
    let output = run_job_binary(&manifest);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let json = String::from_utf8(output.stdout).unwrap();
    assert!(json.starts_with(
        "{\"schema_version\":2,\"analysis\":\"proposition-expression\",\"outcome\":\"satisfied\",\"status\":\"SATISFIED\""
    ));
    assert!(json.contains("\"kind\":\"reachability_witness\""));
    assert!(json.contains("\"action\":\"finish\",\"state\":\"done\""));
    assert!(json.contains("\"product_states\":null"));
    assert!(!json.contains("\"pending\""));

    let violated = write_job(
        &root,
        proposition_model(),
        "reachable \"work\" and \"complete\"\n",
        "",
    );
    let output = run_job_binary(&violated);
    assert_eq!(output.status.code(), Some(11));
    let json = String::from_utf8(output.stdout).unwrap();
    assert!(json.contains("\"outcome\":\"violated\""));
    assert!(json.contains("\"evidence\":null"));

    let _ = fs::remove_dir_all(root);
}

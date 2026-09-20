use formal_verification_lab::{
    check_declarative_ctl_text, check_declarative_ctl_text_with_limits, parse_declarative_document,
    parse_verification_job, run_verification_job_json, BoundedCtlStatus, BoundedCtlTruth,
    BoundedOutcome, ExplorationLimits, VerificationJobAnalysis, VerificationJobCtlAction,
    VerificationJobCtlEvidence, VerificationJobCtlTruth, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobOutcome, VERIFICATION_JOB_CTL_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const BRANCHING_MODEL: &str = r#"
model "ctl-job"
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

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("fvlab-m73-ctl-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_job(root: &Path, model: &str, property: &str, tail: &str) -> PathBuf {
    fs::create_dir_all(root.join("portable")).unwrap();
    fs::write(root.join("portable/model.fvl"), model).unwrap();
    fs::write(root.join("portable/property.ctl"), property).unwrap();
    let manifest = root.join("portable/job.fvj");
    fs::write(
        &manifest,
        format!("analysis \"ctl\"\nmodel \"model.fvl\"\nproperty \"property.ctl\"\n{tail}"),
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
fn ctl_analysis_round_trips_canonically() {
    let source =
        "analysis \"ctl\"\nmodel \"models/system.fvl\"\nproperty \"properties/query.ctl\"\nmax-model-depth 4";
    let job = parse_verification_job(source).unwrap();

    assert_eq!(job.analysis(), VerificationJobAnalysis::Ctl);
    assert_eq!(job.declared_analysis(), Some(VerificationJobAnalysis::Ctl));
    assert_eq!(job.canonical_document(), source);
    assert_eq!(
        parse_verification_job(&job.canonical_document()).unwrap(),
        job
    );
}

#[test]
fn complete_ctl_jobs_match_direct_frontend_for_satisfaction_and_violation() {
    let root = fixture_dir("complete");
    let satisfied_manifest = write_job(&root, BRANCHING_MODEL, r#"EF "complete""#, "");
    let satisfied = run_verification_job_json(&satisfied_manifest);
    assert_eq!(satisfied.exit_code, 0);
    assert_eq!(
        satisfied.envelope.schema_version,
        VERIFICATION_JOB_CTL_RESULT_SCHEMA_VERSION
    );
    assert_eq!(satisfied.envelope.analysis.as_deref(), Some("ctl"));
    assert_eq!(satisfied.envelope.backend.as_deref(), Some("ctl-fixpoint"));
    assert_eq!(
        satisfied.envelope.outcome,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(satisfied.envelope.model.as_deref(), Some("ctl-job"));
    assert_eq!(satisfied.envelope.cutoff, None);
    assert_eq!(satisfied.envelope.evidence, None);

    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
    let direct = check_declarative_ctl_text(&document, r#"EF "complete""#).unwrap();
    let ctl = satisfied.envelope.ctl.as_ref().unwrap();
    assert!(ctl.initial_states_complete);
    assert_eq!(
        ctl.retained_states,
        direct.evaluation.reachable_states.len()
    );
    assert_eq!(
        ctl.definitely_satisfying_states,
        direct.evaluation.satisfying_state_indices.len()
    );
    assert_eq!(
        ctl.possibly_satisfying_states,
        direct.evaluation.satisfying_state_indices.len()
    );
    assert_eq!(ctl.initial[0].truth, VerificationJobCtlTruth::True);
    assert!(matches!(
        ctl.initial[0].evidence,
        Some(VerificationJobCtlEvidence::Finite { .. })
    ));

    fs::write(root.join("portable/property.ctl"), r#"AF "complete""#).unwrap();
    let violated = run_verification_job_json(&satisfied_manifest);
    assert_eq!(violated.exit_code, 14);
    assert_eq!(violated.envelope.outcome, VerificationJobOutcome::Violated);
    let ctl = violated.envelope.ctl.as_ref().unwrap();
    assert_eq!(ctl.initial[0].truth, VerificationJobCtlTruth::False);
    assert!(matches!(
        ctl.initial[0].evidence,
        Some(VerificationJobCtlEvidence::Lasso { .. })
    ));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bounded_ctl_jobs_match_m72_and_preserve_cutoff_provenance() {
    let cases = [
        (
            "state",
            "max-model-states 1\n",
            ExplorationLimits {
                max_states: Some(1),
                ..ExplorationLimits::unbounded()
            },
            VerificationJobCutoffKind::StateLimit,
            1,
        ),
        (
            "transition",
            "max-model-transitions 0\n",
            ExplorationLimits {
                max_transitions: Some(0),
                ..ExplorationLimits::unbounded()
            },
            VerificationJobCutoffKind::TransitionLimit,
            0,
        ),
        (
            "depth",
            "max-model-depth 0\n",
            ExplorationLimits {
                max_depth: Some(0),
                ..ExplorationLimits::unbounded()
            },
            VerificationJobCutoffKind::DepthLimit,
            0,
        ),
    ];

    for (kind, tail, limits, cutoff_kind, cutoff_limit) in cases {
        let root = fixture_dir(kind);
        let manifest = write_job(&root, BRANCHING_MODEL, r#"EF "complete""#, tail);
        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 3);
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Inconclusive);

        let cutoff = run.envelope.cutoff.unwrap();
        assert_eq!(cutoff.stage, VerificationJobCutoffStage::Model);
        assert_eq!(cutoff.kind, cutoff_kind);
        assert_eq!(cutoff.limit, cutoff_limit);

        let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
        let direct =
            check_declarative_ctl_text_with_limits(&document, r#"EF "complete""#, limits).unwrap();
        assert!(matches!(
            direct.evaluation.outcome,
            BoundedOutcome::Inconclusive(_)
        ));
        let ctl = run.envelope.ctl.as_ref().unwrap();
        assert_eq!(
            ctl.retained_states,
            direct.evaluation.reachable_states.len()
        );
        assert_eq!(
            ctl.definitely_satisfying_states,
            direct.evaluation.definitely_satisfying_state_indices.len()
        );
        assert_eq!(
            ctl.possibly_satisfying_states,
            direct.evaluation.possibly_satisfying_state_indices.len()
        );
        assert_eq!(ctl.initial[0].truth, VerificationJobCtlTruth::Unknown);
        assert_eq!(direct.evaluation.initial[0].truth, BoundedCtlTruth::Unknown);

        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn bounded_ctl_job_preserves_early_conclusive_witness_and_counterexample() {
    let root = fixture_dir("early-conclusive");
    let manifest = write_job(
        &root,
        BRANCHING_MODEL,
        r#"EF "complete""#,
        "max-model-states 2\n",
    );
    let satisfied = run_verification_job_json(&manifest);
    assert_eq!(satisfied.exit_code, 0);
    assert_eq!(satisfied.envelope.outcome, VerificationJobOutcome::Satisfied);
    let ctl = satisfied.envelope.ctl.as_ref().unwrap();
    assert_eq!(ctl.initial[0].truth, VerificationJobCtlTruth::True);
    assert!(matches!(
        ctl.initial[0].evidence,
        Some(VerificationJobCtlEvidence::Finite { .. })
    ));

    fs::write(root.join("portable/property.ctl"), r#"AG "ready""#).unwrap();
    let violated = run_verification_job_json(&manifest);
    assert_eq!(violated.exit_code, 14);
    assert_eq!(violated.envelope.outcome, VerificationJobOutcome::Violated);
    let ctl = violated.envelope.ctl.as_ref().unwrap();
    assert_eq!(ctl.initial[0].truth, VerificationJobCtlTruth::False);
    assert!(matches!(
        ctl.initial[0].evidence,
        Some(VerificationJobCtlEvidence::Finite { .. })
    ));

    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
    let direct = check_declarative_ctl_text_with_limits(
        &document,
        r#"AG "ready""#,
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();
    assert_eq!(
        direct.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Violated)
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ctl_job_normalizes_terminal_self_loop_evidence_without_faking_model_action() {
    let root = fixture_dir("terminal");
    let manifest = write_job(
        &root,
        r#"
model "terminal-job"
state "done"
initial "done"
label "done" "complete"
"#,
        r#"EX "complete""#,
        "",
    );
    let run = run_verification_job_json(&manifest);
    assert_eq!(run.exit_code, 0);

    let ctl = run.envelope.ctl.as_ref().unwrap();
    let Some(VerificationJobCtlEvidence::Finite { trace }) = &ctl.initial[0].evidence else {
        panic!("expected finite CTL evidence");
    };
    assert_eq!(trace.len(), 2);
    assert_eq!(
        trace[1].action,
        Some(VerificationJobCtlAction::TerminalSelfLoop)
    );

    let json = run.to_json();
    assert!(json.contains("\"schema_version\":3"));
    assert!(json.contains("\"analysis\":\"ctl\""));
    assert!(json.contains("\"backend\":\"ctl-fixpoint\""));
    assert!(json.contains("\"action\":\"<terminal-self-loop>\""));
    assert!(json.contains("\"ctl\":{"));
    assert!(json.contains("\"evidence\":null"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ctl_jobs_fail_closed_on_formula_metadata_and_temporal_only_configuration() {
    let malformed_root = fixture_dir("malformed");
    let malformed = write_job(&malformed_root, BRANCHING_MODEL, "EF (", "");
    let malformed_run = run_verification_job_json(&malformed);
    assert_eq!(malformed_run.exit_code, 2);
    assert_eq!(
        malformed_run.envelope.outcome,
        VerificationJobOutcome::Error
    );
    assert_eq!(malformed_run.envelope.analysis.as_deref(), Some("ctl"));
    assert_eq!(
        malformed_run.envelope.schema_version,
        VERIFICATION_JOB_CTL_RESULT_SCHEMA_VERSION
    );
    assert!(malformed_run
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("CTL parse error"));

    fs::write(
        malformed_root.join("portable/property.ctl"),
        r#"EF "missing""#,
    )
    .unwrap();
    let unknown_run = run_verification_job_json(&malformed);
    assert_eq!(unknown_run.exit_code, 2);
    assert!(unknown_run
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("unknown CTL proposition 'missing'"));

    for (kind, tail, needle) in [
        (
            "fairness",
            "weak-fair-action \"tick\"\n",
            "do not support weak-fair-action or strong-fair-action",
        ),
        (
            "product-limit",
            "max-product-states 1\n",
            "do not support max-product-* limits",
        ),
    ] {
        let root = fixture_dir(kind);
        let manifest = root.join("job.fvj");
        fs::write(
            &manifest,
            format!("analysis \"ctl\"\nmodel \"missing.fvl\"\nproperty \"missing.ctl\"\n{tail}"),
        )
        .unwrap();
        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 2);
        assert!(run.envelope.error.as_deref().unwrap().contains(needle));
        fs::remove_dir_all(root).unwrap();
    }

    fs::remove_dir_all(malformed_root).unwrap();
}

#[test]
fn built_binary_ctl_job_is_manifest_relative_and_emits_stable_json_statuses() {
    let root = fixture_dir("binary");
    let manifest = write_job(&root, BRANCHING_MODEL, r#"EF "complete""#, "");
    let unrelated = root.join("unrelated-cwd");
    fs::create_dir_all(&unrelated).unwrap();

    let satisfied = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(satisfied.status.code(), Some(0));
    let stdout = String::from_utf8(satisfied.stdout).unwrap();
    assert!(stdout.contains("\"schema_version\":3"));
    assert!(stdout.contains("\"analysis\":\"ctl\""));
    assert!(stdout.contains("\"outcome\":\"satisfied\""));
    assert!(stdout.contains("\"status\":\"SATISFIED\""));

    fs::write(
        root.join("portable/job.fvj"),
        "analysis \"ctl\"\nmodel \"model.fvl\"\nproperty \"property.ctl\"\nmax-model-states 1\n",
    )
    .unwrap();
    let inconclusive = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(inconclusive.status.code(), Some(3));
    let stdout = String::from_utf8(inconclusive.stdout).unwrap();
    assert!(stdout.contains("\"outcome\":\"inconclusive\""));
    assert!(stdout.contains("\"truth\":\"unknown\""));
    assert!(stdout.contains("\"kind\":\"state_limit\""));

    fs::write(root.join("portable/property.ctl"), "EF (").unwrap();
    let malformed = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(malformed.status.code(), Some(2));
    let stdout = String::from_utf8(malformed.stdout).unwrap();
    assert!(stdout.contains("\"outcome\":\"error\""));
    assert!(stdout.contains("\"analysis\":\"ctl\""));

    fs::remove_dir_all(root).unwrap();
}

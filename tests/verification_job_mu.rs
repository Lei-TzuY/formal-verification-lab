use formal_verification_lab::{
    check_declarative_mu_text, check_declarative_mu_text_with_limits, parse_declarative_document,
    parse_verification_job, run_verification_job_json, BoundedMuStatus, BoundedMuTruth,
    BoundedOutcome, ExplorationLimits, VerificationJobAnalysis, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobMuTruth, VerificationJobOutcome,
    VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const BRANCHING_MODEL: &str = r#"
model "mu-job"
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
    let root =
        std::env::temp_dir().join(format!("fvlab-m79-mu-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_job(root: &Path, model: &str, property: &str, tail: &str) -> PathBuf {
    fs::create_dir_all(root.join("portable")).unwrap();
    fs::write(root.join("portable/model.fvl"), model).unwrap();
    fs::write(root.join("portable/property.mu"), property).unwrap();
    let manifest = root.join("portable/job.fvj");
    fs::write(
        &manifest,
        format!("analysis \"mu-calculus\"\nmodel \"model.fvl\"\nproperty \"property.mu\"\n{tail}"),
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
fn mu_analysis_round_trips_canonically() {
    let source =
        "analysis \"mu-calculus\"\nmodel \"models/system.fvl\"\nproperty \"properties/query.mu\"\nmax-model-depth 4";
    let job = parse_verification_job(source).unwrap();

    assert_eq!(job.analysis(), VerificationJobAnalysis::MuCalculus);
    assert_eq!(
        job.declared_analysis(),
        Some(VerificationJobAnalysis::MuCalculus)
    );
    assert_eq!(job.canonical_document(), source);
    assert_eq!(
        parse_verification_job(&job.canonical_document()).unwrap(),
        job
    );
}

#[test]
fn complete_mu_jobs_match_direct_frontend_and_emit_two_valued_schema_v4() {
    let root = fixture_dir("complete");
    let manifest = write_job(&root, BRANCHING_MODEL, REACH_COMPLETE, "");
    let run = run_verification_job_json(&manifest);

    assert_eq!(run.exit_code, 0);
    assert_eq!(
        run.envelope.schema_version,
        VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION
    );
    assert_eq!(run.envelope.analysis.as_deref(), Some("mu-calculus"));
    assert_eq!(run.envelope.backend.as_deref(), Some("mu-fixpoint"));
    assert_eq!(run.envelope.outcome, VerificationJobOutcome::Satisfied);
    assert_eq!(run.envelope.model.as_deref(), Some("mu-job"));
    assert_eq!(run.envelope.cutoff, None);
    assert_eq!(run.envelope.evidence, None);
    assert!(run.envelope.ctl.is_none());

    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
    let direct = check_declarative_mu_text(&document, REACH_COMPLETE).unwrap();
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
    assert_eq!(
        mu.fixpoint_iterations,
        direct.evaluation.fixpoint_iterations
    );
    assert_eq!(mu.initial[0].truth, VerificationJobMuTruth::True);

    fs::write(root.join("portable/property.mu"), r#""complete""#).unwrap();
    let violated = run_verification_job_json(&manifest);
    assert_eq!(violated.exit_code, 15);
    assert_eq!(violated.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(
        violated.envelope.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::False
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn bounded_mu_jobs_match_m77_and_preserve_all_cutoff_classes() {
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
        let manifest = write_job(&root, BRANCHING_MODEL, REACH_COMPLETE, tail);
        let run = run_verification_job_json(&manifest);
        assert_eq!(run.exit_code, 3);
        assert_eq!(run.envelope.outcome, VerificationJobOutcome::Inconclusive);

        let cutoff = run.envelope.cutoff.unwrap();
        assert_eq!(cutoff.stage, VerificationJobCutoffStage::Model);
        assert_eq!(cutoff.kind, cutoff_kind);
        assert_eq!(cutoff.limit, cutoff_limit);

        let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
        let direct =
            check_declarative_mu_text_with_limits(&document, REACH_COMPLETE, limits).unwrap();
        assert!(matches!(
            direct.evaluation.outcome,
            BoundedOutcome::Inconclusive(_)
        ));
        let mu = run.envelope.mu.as_ref().unwrap();
        assert_eq!(mu.retained_states, direct.evaluation.reachable_states.len());
        assert_eq!(
            mu.definitely_satisfying_states,
            direct.evaluation.definitely_satisfying_state_indices.len()
        );
        assert_eq!(
            mu.possibly_satisfying_states,
            direct.evaluation.possibly_satisfying_state_indices.len()
        );
        assert_eq!(
            mu.fixpoint_iterations,
            direct.evaluation.fixpoint_iterations
        );
        assert_eq!(mu.initial[0].truth, VerificationJobMuTruth::Unknown);
        assert_eq!(direct.evaluation.initial[0].truth, BoundedMuTruth::Unknown);

        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn bounded_mu_jobs_preserve_early_conclusions_and_zero_budget_honesty() {
    let root = fixture_dir("early");
    let manifest = write_job(
        &root,
        BRANCHING_MODEL,
        REACH_COMPLETE,
        "max-model-states 2\n",
    );
    let satisfied = run_verification_job_json(&manifest);
    assert_eq!(satisfied.exit_code, 0);
    assert_eq!(
        satisfied.envelope.outcome,
        VerificationJobOutcome::Satisfied
    );
    assert_eq!(
        satisfied.envelope.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::True
    );

    fs::write(root.join("portable/property.mu"), r#""complete""#).unwrap();
    let violated = run_verification_job_json(&manifest);
    assert_eq!(violated.exit_code, 15);
    assert_eq!(violated.envelope.outcome, VerificationJobOutcome::Violated);
    assert_eq!(
        violated.envelope.mu.as_ref().unwrap().initial[0].truth,
        VerificationJobMuTruth::False
    );

    let zero = fixture_dir("zero");
    let zero_manifest = write_job(
        &zero,
        BRANCHING_MODEL,
        REACH_COMPLETE,
        "max-model-states 0\n",
    );
    let zero_run = run_verification_job_json(&zero_manifest);
    assert_eq!(zero_run.exit_code, 3);
    assert_eq!(
        zero_run.envelope.outcome,
        VerificationJobOutcome::Inconclusive
    );
    let mu = zero_run.envelope.mu.as_ref().unwrap();
    assert!(!mu.initial_states_complete);
    assert_eq!(mu.retained_states, 0);
    assert!(mu.initial.is_empty());

    let document = parse_declarative_document(BRANCHING_MODEL).unwrap();
    let direct = check_declarative_mu_text_with_limits(
        &document,
        REACH_COMPLETE,
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();
    assert_eq!(
        direct.evaluation.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(zero).unwrap();
}

#[test]
fn mu_jobs_fail_closed_on_formula_metadata_limits_and_temporal_only_configuration() {
    let root = fixture_dir("fail-closed");
    let manifest = write_job(&root, BRANCHING_MODEL, "mu X. (", "");
    let malformed = run_verification_job_json(&manifest);
    assert_eq!(malformed.exit_code, 2);
    assert_eq!(malformed.envelope.outcome, VerificationJobOutcome::Error);
    assert_eq!(malformed.envelope.analysis.as_deref(), Some("mu-calculus"));
    assert_eq!(
        malformed.envelope.schema_version,
        VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION
    );

    fs::write(root.join("portable/property.mu"), r#""missing""#).unwrap();
    let unknown = run_verification_job_json(&manifest);
    assert_eq!(unknown.exit_code, 2);
    assert!(unknown
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("unknown modal mu-calculus proposition 'missing'"));

    fs::write(root.join("portable/property.mu"), "mu X. not $X").unwrap();
    let non_monotone = run_verification_job_json(&manifest);
    assert_eq!(non_monotone.exit_code, 2);
    assert!(non_monotone
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("negative"));

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
        let case_root = fixture_dir(kind);
        let bad_manifest = case_root.join("job.fvj");
        fs::write(
            &bad_manifest,
            format!(
                "analysis \"mu-calculus\"\nmodel \"missing.fvl\"\nproperty \"missing.mu\"\n{tail}"
            ),
        )
        .unwrap();
        let run = run_verification_job_json(&bad_manifest);
        assert_eq!(run.exit_code, 2);
        assert!(run.envelope.error.as_deref().unwrap().contains(needle));
        fs::remove_dir_all(case_root).unwrap();
    }

    let bad_limit = root.join("bad-limit.fvj");
    fs::write(
        &bad_limit,
        "analysis \"mu-calculus\"\nmodel \"model.fvl\"\nproperty \"property.mu\"\nmax-model-states nope\n",
    )
    .unwrap();
    let invalid = run_verification_job_json(&bad_limit);
    assert_eq!(invalid.exit_code, 2);
    assert_eq!(invalid.envelope.outcome, VerificationJobOutcome::Error);
    assert!(invalid
        .envelope
        .error
        .as_deref()
        .unwrap()
        .contains("invalid non-negative decimal integer"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_binary_mu_job_is_manifest_relative_and_json_is_deterministic() {
    let root = fixture_dir("binary");
    let manifest = write_job(&root, BRANCHING_MODEL, REACH_COMPLETE, "");
    let unrelated = root.join("unrelated-cwd");
    fs::create_dir_all(&unrelated).unwrap();

    let satisfied = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(satisfied.status.code(), Some(0));
    let stdout = String::from_utf8(satisfied.stdout).unwrap();
    assert!(stdout.contains("\"schema_version\":4"));
    assert!(stdout.contains("\"analysis\":\"mu-calculus\""));
    assert!(stdout.contains("\"backend\":\"mu-fixpoint\""));
    assert!(stdout.contains("\"outcome\":\"satisfied\""));
    assert!(stdout.contains("\"mu\":{"));
    assert!(stdout.contains("\"fixpoint_iterations\":"));

    let direct_json = run_verification_job_json(&manifest).to_json();
    assert_eq!(direct_json, run_verification_job_json(&manifest).to_json());

    fs::write(
        root.join("portable/job.fvj"),
        "analysis \"mu-calculus\"\nmodel \"model.fvl\"\nproperty \"property.mu\"\nmax-model-states 1\n",
    )
    .unwrap();
    let inconclusive = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(inconclusive.status.code(), Some(3));
    let stdout = String::from_utf8(inconclusive.stdout).unwrap();
    assert!(stdout.contains("\"outcome\":\"inconclusive\""));
    assert!(stdout.contains("\"truth\":\"unknown\""));
    assert!(stdout.contains("\"kind\":\"state_limit\""));

    fs::write(root.join("portable/property.mu"), "mu X. (").unwrap();
    let malformed = run_job_binary(&manifest, Some(&unrelated));
    assert_eq!(malformed.status.code(), Some(2));
    let stdout = String::from_utf8(malformed.stdout).unwrap();
    assert!(stdout.contains("\"outcome\":\"error\""));
    assert!(stdout.contains("\"analysis\":\"mu-calculus\""));

    fs::remove_dir_all(root).unwrap();
}

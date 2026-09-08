use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, InconclusiveReason};
use crate::exact_state::{
    check_exact_state_property_with_limits, parse_exact_state_property, ExactStateEvidence,
    ExactStateStatus,
};
use crate::multi_response::{MultiResponseProperty, MultiResponseStatus};
use crate::multi_temporal::parse_multi_response_temporal;
use crate::proposition_expr::{
    check_proposition_expression_property_with_limits, BoundedPropositionExpressionResult,
    PropositionExpressionPropertySpec,
};
use crate::safety::{check_safety_assertion_with_limits, PropositionSafetySpec, SafetyStatus};
use crate::verification_execution::{
    execute_multi_response, MultiResponseExecutionConfig, MultiResponseExecutionResult,
};
use crate::verification_job::{parse_verification_job, VerificationJob, VerificationJobAnalysis};
use crate::verification_result::{
    VerificationJobAccounting, VerificationJobCutoff, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobOutcome,
    VerificationJobResultEnvelope, VerificationJobStateTraceStep,
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
};
use crate::{
    parse_declarative_document, parse_declarative_model, parse_proposition_expression,
    TransitionSystem,
};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Historical typed loader for the M55 multi-response job family.
///
/// M59+ keeps this public shape stable. Heterogeneous execution is exposed by
/// `run_verification_job_json`; callers that need typed safety, exact-state, or
/// proposition-expression inputs should use the existing dedicated APIs directly.
pub struct LoadedVerificationJob {
    pub job: VerificationJob,
    pub model: TransitionSystem<String>,
    pub property: MultiResponseProperty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobLoadError {
    message: String,
}

impl VerificationJobLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VerificationJobLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for VerificationJobLoadError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobJsonRun {
    pub envelope: VerificationJobResultEnvelope,
    pub exit_code: u8,
}

impl VerificationJobJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

pub fn load_verification_job(
    manifest_path: impl AsRef<Path>,
) -> Result<LoadedVerificationJob, VerificationJobLoadError> {
    let manifest_path = manifest_path.as_ref();
    let input = read_job_manifest(manifest_path)?;
    let job = parse_verification_job(&input)
        .map_err(|error| VerificationJobLoadError::new(error.to_string()))?;
    if job.analysis() != VerificationJobAnalysis::MultiResponse {
        return Err(VerificationJobLoadError::new(
            "typed load_verification_job supports only multi-response jobs; use run_verification_job_json for heterogeneous execution",
        ));
    }
    load_multi_response_job(manifest_path, job)
}

pub fn run_verification_job_json(manifest_path: impl AsRef<Path>) -> VerificationJobJsonRun {
    let manifest_path = manifest_path.as_ref();
    let input = match read_job_manifest(manifest_path) {
        Ok(input) => input,
        Err(error) => return error_run(VerificationJobResultEnvelope::error(error.to_string())),
    };
    let job = match parse_verification_job(&input) {
        Ok(job) => job,
        Err(error) => return error_run(VerificationJobResultEnvelope::error(error.to_string())),
    };

    match job.analysis() {
        VerificationJobAnalysis::MultiResponse => {
            match run_multi_response_job_json(manifest_path, job) {
                Ok(run) => run,
                Err(error) => error_run(VerificationJobResultEnvelope::error(error)),
            }
        }
        VerificationJobAnalysis::Safety => match run_safety_job_json(manifest_path, job) {
            Ok(run) => run,
            Err(error) => error_run(VerificationJobResultEnvelope::safety_error(error)),
        },
        VerificationJobAnalysis::ExactState => match run_exact_state_job_json(manifest_path, job) {
            Ok(run) => run,
            Err(error) => error_run(VerificationJobResultEnvelope::exact_state_error(error)),
        },
        VerificationJobAnalysis::PropositionExpression => {
            match run_proposition_expression_job_json(manifest_path, job) {
                Ok(run) => run,
                Err(error) => error_run(proposition_expression_error(error)),
            }
        }
    }
}

fn run_multi_response_job_json(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<VerificationJobJsonRun, String> {
    let loaded = load_multi_response_job(manifest_path, job).map_err(|error| error.to_string())?;
    let config =
        MultiResponseExecutionConfig::from_job(&loaded.job).map_err(|error| error.to_string())?;
    let result = execute_multi_response(&loaded.model, &loaded.property, &config)
        .map_err(|error| error.to_string())?;
    let weak = config.fairness().weak_actions();
    let strong = config.fairness().strong_actions();
    let model_name = loaded.model.name().to_owned();
    let envelope = match &result {
        MultiResponseExecutionResult::Unbounded(result) => {
            VerificationJobResultEnvelope::from_unbounded(
                model_name,
                weak,
                strong,
                config.model_limits(),
                config.product_limits(),
                result,
            )
        }
        MultiResponseExecutionResult::ProductBounded(result) => {
            VerificationJobResultEnvelope::from_product_bounded(
                model_name,
                weak,
                strong,
                config.model_limits(),
                config.product_limits(),
                result,
            )
        }
        MultiResponseExecutionResult::Staged(result) => VerificationJobResultEnvelope::from_staged(
            model_name,
            weak,
            strong,
            config.model_limits(),
            config.product_limits(),
            result,
        ),
    };
    let exit_code = match envelope.outcome {
        VerificationJobOutcome::Satisfied => 0,
        VerificationJobOutcome::Violated => 7,
        VerificationJobOutcome::Inconclusive => 3,
        VerificationJobOutcome::Error => 2,
    };
    debug_assert_eq!(
        execution_outcome(&result),
        envelope.outcome,
        "structured envelope must preserve canonical execution status"
    );
    Ok(VerificationJobJsonRun {
        envelope,
        exit_code,
    })
}

fn run_safety_job_json(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<VerificationJobJsonRun, String> {
    validate_model_only_job(&job, "safety")?;
    let (model_path, property_path) = resolve_job_paths(manifest_path, &job);

    let model_input = fs::read_to_string(&model_path).map_err(|error| {
        format!(
            "failed to read declarative model '{}': {error}",
            model_path.display()
        )
    })?;
    let document = parse_declarative_document(&model_input).map_err(|error| error.to_string())?;

    let property_input = fs::read_to_string(&property_path).map_err(|error| {
        format!(
            "failed to read safety property '{}': {error}",
            property_path.display()
        )
    })?;
    let expression =
        parse_proposition_expression(&property_input).map_err(|error| error.to_string())?;
    let spec = PropositionSafetySpec::always("verification-job-safety", expression)
        .map_err(|error| error.to_string())?;
    let result = check_safety_assertion_with_limits(&document, &spec, job.model_limits())
        .map_err(|error| error.to_string())?;
    let envelope = VerificationJobResultEnvelope::from_safety(
        document.model().name().to_owned(),
        job.model_limits(),
        &result,
    );
    let exit_code = match result.outcome {
        BoundedOutcome::Conclusive(SafetyStatus::Safe) => 0,
        BoundedOutcome::Conclusive(SafetyStatus::Violated) => 12,
        BoundedOutcome::Inconclusive(_) => 3,
    };
    debug_assert_eq!(safety_execution_outcome(&result.outcome), envelope.outcome);
    Ok(VerificationJobJsonRun {
        envelope,
        exit_code,
    })
}

fn run_exact_state_job_json(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<VerificationJobJsonRun, String> {
    validate_model_only_job(&job, "exact-state")?;
    let (model_path, property_path) = resolve_job_paths(manifest_path, &job);

    let model_input = fs::read_to_string(&model_path).map_err(|error| {
        format!(
            "failed to read declarative model '{}': {error}",
            model_path.display()
        )
    })?;
    let model = parse_declarative_model(&model_input).map_err(|error| error.to_string())?;

    let property_input = fs::read_to_string(&property_path).map_err(|error| {
        format!(
            "failed to read exact-state property '{}': {error}",
            property_path.display()
        )
    })?;
    let spec = parse_exact_state_property("verification-job-exact-state", &property_input)
        .map_err(|error| error.to_string())?;
    let canonical_property = spec.canonical_expression();
    let result = check_exact_state_property_with_limits(&model, &spec, job.model_limits())
        .map_err(|error| error.to_string())?;
    let envelope = VerificationJobResultEnvelope::from_exact_state(
        model.name().to_owned(),
        canonical_property,
        job.model_limits(),
        &result,
    );
    let exit_code = match result.outcome {
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied) => 0,
        BoundedOutcome::Conclusive(ExactStateStatus::Violated) => 11,
        BoundedOutcome::Inconclusive(_) => 3,
    };
    debug_assert_eq!(
        exact_state_execution_outcome(&result.outcome),
        envelope.outcome
    );
    Ok(VerificationJobJsonRun {
        envelope,
        exit_code,
    })
}

fn run_proposition_expression_job_json(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<VerificationJobJsonRun, String> {
    validate_model_only_job(&job, "proposition-expression")?;
    let (model_path, property_path) = resolve_job_paths(manifest_path, &job);

    let model_input = fs::read_to_string(&model_path).map_err(|error| {
        format!(
            "failed to read declarative model '{}': {error}",
            model_path.display()
        )
    })?;
    let document = parse_declarative_document(&model_input).map_err(|error| error.to_string())?;

    let property_input = fs::read_to_string(&property_path).map_err(|error| {
        format!(
            "failed to read proposition-expression property '{}': {error}",
            property_path.display()
        )
    })?;
    let (canonical_property, spec) = parse_proposition_job_property(&property_input)?;
    let result = check_proposition_expression_property_with_limits(
        &document,
        &spec,
        job.model_limits(),
    )
    .map_err(|error| error.to_string())?;
    let envelope = proposition_expression_envelope(
        document.model().name().to_owned(),
        canonical_property,
        job.model_limits(),
        &result,
    );
    let exit_code = match result.outcome {
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied) => 0,
        BoundedOutcome::Conclusive(ExactStateStatus::Violated) => 11,
        BoundedOutcome::Inconclusive(_) => 3,
    };
    debug_assert_eq!(
        exact_state_execution_outcome(&result.outcome),
        envelope.outcome
    );
    Ok(VerificationJobJsonRun {
        envelope,
        exit_code,
    })
}

fn parse_proposition_job_property(
    input: &str,
) -> Result<(String, PropositionExpressionPropertySpec), String> {
    let trimmed = input.trim_matches(|ch: char| ch.is_ascii_whitespace());
    if trimmed.is_empty() {
        return Err(
            "proposition-expression property must be 'reachable <expression>' or 'all-eventually <expression>'"
                .to_owned(),
        );
    }

    let Some(separator) = trimmed.find(|ch: char| ch.is_ascii_whitespace()) else {
        return Err(format!(
            "proposition-expression property mode '{}' requires a Boolean expression",
            trimmed
        ));
    };
    let mode = &trimmed[..separator];
    let expression_input = trimmed[separator..]
        .trim_matches(|ch: char| ch.is_ascii_whitespace());
    if expression_input.is_empty() {
        return Err(format!(
            "proposition-expression property mode '{mode}' requires a Boolean expression"
        ));
    }

    let expression =
        parse_proposition_expression(expression_input).map_err(|error| error.to_string())?;
    let canonical_expression = expression.canonical_expression();
    let spec = match mode {
        "reachable" => PropositionExpressionPropertySpec::reachable(
            "verification-job-proposition-expression",
            expression,
        ),
        "all-eventually" => PropositionExpressionPropertySpec::all_eventually(
            "verification-job-proposition-expression",
            expression,
        ),
        _ => {
            return Err(format!(
                "unsupported proposition-expression property mode '{mode}'; expected reachable or all-eventually"
            ));
        }
    }
    .map_err(|error| error.to_string())?;

    Ok((format!("{mode} {canonical_expression}"), spec))
}

fn proposition_expression_envelope(
    model: String,
    property: String,
    model_limits: ExplorationLimits,
    result: &BoundedPropositionExpressionResult,
) -> VerificationJobResultEnvelope {
    VerificationJobResultEnvelope {
        schema_version: VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
        analysis: Some("proposition-expression".to_owned()),
        outcome: exact_state_execution_outcome(&result.outcome),
        model: Some(model),
        property: Some(property),
        weak_fair_actions: Vec::new(),
        strong_fair_actions: Vec::new(),
        model_limits: model_limits.into(),
        product_limits: ExplorationLimits::unbounded().into(),
        accounting: VerificationJobAccounting {
            model_states: Some(result.discovered_states),
            checked_model_states: Some(result.checked_states),
            explored_model_transitions: Some(result.explored_transitions),
            retained_model_transitions: Some(result.explored_transitions),
            max_model_depth_reached: result.max_depth_reached,
            product_states: None,
            checked_product_states: None,
            explored_product_transitions: None,
            retained_product_transitions: None,
            max_product_depth_reached: None,
        },
        cutoff: result
            .outcome
            .inconclusive_reason()
            .map(model_cutoff),
        evidence: result.evidence.as_ref().map(convert_state_property_evidence),
        error: None,
    }
}

fn proposition_expression_error(message: impl Into<String>) -> VerificationJobResultEnvelope {
    let mut envelope = VerificationJobResultEnvelope::error(message);
    envelope.schema_version = VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION;
    envelope.analysis = Some("proposition-expression".to_owned());
    envelope
}

fn model_cutoff(reason: InconclusiveReason) -> VerificationJobCutoff {
    match reason {
        InconclusiveReason::StateLimitReached { limit } => VerificationJobCutoff {
            stage: VerificationJobCutoffStage::Model,
            kind: VerificationJobCutoffKind::StateLimit,
            limit,
        },
        InconclusiveReason::TransitionLimitReached { limit } => VerificationJobCutoff {
            stage: VerificationJobCutoffStage::Model,
            kind: VerificationJobCutoffKind::TransitionLimit,
            limit,
        },
        InconclusiveReason::DepthLimitReached { limit } => VerificationJobCutoff {
            stage: VerificationJobCutoffStage::Model,
            kind: VerificationJobCutoffKind::DepthLimit,
            limit,
        },
    }
}

fn convert_state_property_evidence(evidence: &ExactStateEvidence) -> VerificationJobEvidence {
    match evidence {
        ExactStateEvidence::ReachabilityWitness { trace } => {
            VerificationJobEvidence::ExactStateReachability {
                trace: trace.iter().map(convert_state_step).collect(),
            }
        }
        ExactStateEvidence::EventualityFiniteCounterexample { trace } => {
            VerificationJobEvidence::ExactStateEventualityFinite {
                trace: trace.iter().map(convert_state_step).collect(),
            }
        }
        ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle } => {
            VerificationJobEvidence::ExactStateEventualityInfinite {
                stem: stem.iter().map(convert_state_step).collect(),
                cycle: cycle.iter().map(convert_state_step).collect(),
            }
        }
    }
}

fn convert_state_step(step: &crate::checker::TraceStep<String>) -> VerificationJobStateTraceStep {
    VerificationJobStateTraceStep {
        action: step.action.clone(),
        state: step.state.clone(),
    }
}

fn load_multi_response_job(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<LoadedVerificationJob, VerificationJobLoadError> {
    let (model_path, property_path) = resolve_job_paths(manifest_path, &job);

    let model_input = fs::read_to_string(&model_path).map_err(|error| {
        VerificationJobLoadError::new(format!(
            "failed to read declarative model '{}': {error}",
            model_path.display()
        ))
    })?;
    let model = parse_declarative_model(&model_input)
        .map_err(|error| VerificationJobLoadError::new(error.to_string()))?;

    let property_input = fs::read_to_string(&property_path).map_err(|error| {
        VerificationJobLoadError::new(format!(
            "failed to read multi-response property '{}': {error}",
            property_path.display()
        ))
    })?;
    let spec = parse_multi_response_temporal("cli-multi-response", &property_input)
        .map_err(|error| VerificationJobLoadError::new(error.to_string()))?;
    let property = spec
        .to_property()
        .map_err(|error| VerificationJobLoadError::new(error.to_string()))?;

    Ok(LoadedVerificationJob {
        job,
        model,
        property,
    })
}

fn read_job_manifest(manifest_path: &Path) -> Result<String, VerificationJobLoadError> {
    fs::read_to_string(manifest_path).map_err(|error| {
        VerificationJobLoadError::new(format!(
            "failed to read verification job '{}': {error}",
            manifest_path.display()
        ))
    })
}

fn validate_model_only_job(job: &VerificationJob, family: &str) -> Result<(), String> {
    if !job.weak_fair_actions().is_empty() || !job.strong_fair_actions().is_empty() {
        return Err(format!(
            "{family} verification jobs do not support weak-fair-action or strong-fair-action directives"
        ));
    }
    let product_limits = job.product_limits();
    if product_limits.max_states.is_some()
        || product_limits.max_transitions.is_some()
        || product_limits.max_depth.is_some()
    {
        return Err(format!(
            "{family} verification jobs do not support max-product-* limits"
        ));
    }
    Ok(())
}

fn error_run(envelope: VerificationJobResultEnvelope) -> VerificationJobJsonRun {
    VerificationJobJsonRun {
        envelope,
        exit_code: 2,
    }
}

fn resolve_job_paths(manifest_path: &Path, job: &VerificationJob) -> (PathBuf, PathBuf) {
    let base = manifest_path.parent().unwrap_or_else(|| Path::new(""));
    (
        resolve_path(base, Path::new(job.model_path())),
        resolve_path(base, Path::new(job.property_path())),
    )
}

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn execution_outcome(result: &MultiResponseExecutionResult<String>) -> VerificationJobOutcome {
    match result.status() {
        BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied) => {
            VerificationJobOutcome::Satisfied
        }
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated) => {
            VerificationJobOutcome::Violated
        }
        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn safety_execution_outcome(outcome: &BoundedOutcome<SafetyStatus>) -> VerificationJobOutcome {
    match outcome {
        BoundedOutcome::Conclusive(SafetyStatus::Safe) => VerificationJobOutcome::Satisfied,
        BoundedOutcome::Conclusive(SafetyStatus::Violated) => VerificationJobOutcome::Violated,
        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn exact_state_execution_outcome(
    outcome: &BoundedOutcome<ExactStateStatus>,
) -> VerificationJobOutcome {
    match outcome {
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied) => {
            VerificationJobOutcome::Satisfied
        }
        BoundedOutcome::Conclusive(ExactStateStatus::Violated) => VerificationJobOutcome::Violated,
        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

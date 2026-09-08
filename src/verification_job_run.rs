use crate::bounded::BoundedOutcome;
use crate::multi_response::{MultiResponseProperty, MultiResponseStatus};
use crate::multi_temporal::parse_multi_response_temporal;
use crate::safety::{check_safety_assertion_with_limits, PropositionSafetySpec, SafetyStatus};
use crate::verification_execution::{
    execute_multi_response, MultiResponseExecutionConfig, MultiResponseExecutionResult,
};
use crate::verification_job::{parse_verification_job, VerificationJob, VerificationJobAnalysis};
use crate::verification_result::{VerificationJobOutcome, VerificationJobResultEnvelope};
use crate::{
    parse_declarative_document, parse_declarative_model, parse_proposition_expression,
    TransitionSystem,
};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Historical typed loader for the M55 multi-response job family.
///
/// M59 keeps this public shape stable. Heterogeneous execution is exposed by
/// `run_verification_job_json`; callers that need typed safety inputs should use
/// the existing declarative/safety APIs directly.
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
    validate_safety_job(&job)?;
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

fn validate_safety_job(job: &VerificationJob) -> Result<(), String> {
    if !job.weak_fair_actions().is_empty() || !job.strong_fair_actions().is_empty() {
        return Err(
            "safety verification jobs do not support weak-fair-action or strong-fair-action directives"
                .to_owned(),
        );
    }
    let product_limits = job.product_limits();
    if product_limits.max_states.is_some()
        || product_limits.max_transitions.is_some()
        || product_limits.max_depth.is_some()
    {
        return Err("safety verification jobs do not support max-product-* limits".to_owned());
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

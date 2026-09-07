use crate::multi_response::{MultiResponseProperty, MultiResponseStatus};
use crate::multi_temporal::parse_multi_response_temporal;
use crate::verification_execution::{
    execute_multi_response, MultiResponseExecutionConfig, MultiResponseExecutionResult,
};
use crate::verification_job::{parse_verification_job, VerificationJob};
use crate::verification_result::{VerificationJobOutcome, VerificationJobResultEnvelope};
use crate::{parse_declarative_model, TransitionSystem};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
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
    let input = fs::read_to_string(manifest_path).map_err(|error| {
        VerificationJobLoadError::new(format!(
            "failed to read verification job '{}': {error}",
            manifest_path.display()
        ))
    })?;
    let job = parse_verification_job(&input)
        .map_err(|error| VerificationJobLoadError::new(error.to_string()))?;
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

pub fn run_verification_job_json(manifest_path: impl AsRef<Path>) -> VerificationJobJsonRun {
    match run_verification_job_json_inner(manifest_path.as_ref()) {
        Ok(run) => run,
        Err(error) => VerificationJobJsonRun {
            envelope: VerificationJobResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

fn run_verification_job_json_inner(manifest_path: &Path) -> Result<VerificationJobJsonRun, String> {
    let loaded = load_verification_job(manifest_path).map_err(|error| error.to_string())?;
    let config = MultiResponseExecutionConfig::from_job(&loaded.job)
        .map_err(|error| error.to_string())?;
    let result = execute_multi_response(&loaded.model, &loaded.property, &config)
        .map_err(|error| error.to_string())?;
    let weak = config.fairness().weak_actions();
    let strong = config.fairness().strong_actions();
    let model_name = loaded.model.name().to_owned();
    let envelope = match &result {
        MultiResponseExecutionResult::Unbounded(result) => VerificationJobResultEnvelope::from_unbounded(
            model_name,
            weak,
            strong,
            config.model_limits(),
            config.product_limits(),
            result,
        ),
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
        crate::BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied) => {
            VerificationJobOutcome::Satisfied
        }
        crate::BoundedOutcome::Conclusive(MultiResponseStatus::Violated) => {
            VerificationJobOutcome::Violated
        }
        crate::BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

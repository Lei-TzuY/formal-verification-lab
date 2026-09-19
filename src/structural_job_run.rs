use crate::bounded::BoundedOutcome;
use crate::parse_declarative_model;
use crate::recurrence::{analyze_recurrence_with_limits, RecurrenceStatus};
use crate::structural_job::{parse_structural_job, StructuralJobAnalysis};
use crate::structural_result::{
    StructuralJobOutcome, StructuralJobResultEnvelope,
};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobJsonRun {
    pub envelope: StructuralJobResultEnvelope,
    pub exit_code: u8,
}

impl StructuralJobJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

pub fn run_structural_job_json(manifest_path: impl AsRef<Path>) -> StructuralJobJsonRun {
    let manifest_path = manifest_path.as_ref();
    let input = match fs::read_to_string(manifest_path) {
        Ok(input) => input,
        Err(error) => {
            return error_run(format!(
                "failed to read structural job manifest '{}': {error}",
                manifest_path.display()
            ));
        }
    };

    let job = match parse_structural_job(&input) {
        Ok(job) => job,
        Err(error) => return error_run(error.to_string()),
    };

    match job.analysis() {
        StructuralJobAnalysis::Recurrence => run_recurrence_job(manifest_path, &job),
    }
}

fn run_recurrence_job(
    manifest_path: &Path,
    job: &crate::structural_job::StructuralJob,
) -> StructuralJobJsonRun {
    let model_path = resolve_path(manifest_path, job.model_path());
    let model_input = match fs::read_to_string(&model_path) {
        Ok(input) => input,
        Err(error) => {
            return error_run(format!(
                "failed to read declarative model '{}': {error}",
                model_path.display()
            ));
        }
    };
    let model = match parse_declarative_model(&model_input) {
        Ok(model) => model,
        Err(error) => return error_run(error.to_string()),
    };

    let limits = job.model_limits();
    let result = match analyze_recurrence_with_limits(&model, limits) {
        Ok(result) => result,
        Err(error) => return error_run(error.to_string()),
    };
    let envelope = StructuralJobResultEnvelope::from_recurrence(
        model.name().to_owned(),
        limits,
        &result,
    );
    let exit_code = match result.outcome {
        BoundedOutcome::Conclusive(RecurrenceStatus::CycleFound)
        | BoundedOutcome::Conclusive(RecurrenceStatus::Acyclic) => 0,
        BoundedOutcome::Inconclusive(_) => 3,
    };
    debug_assert_eq!(
        envelope.outcome,
        match result.outcome {
            BoundedOutcome::Conclusive(RecurrenceStatus::CycleFound) => {
                StructuralJobOutcome::CycleFound
            }
            BoundedOutcome::Conclusive(RecurrenceStatus::Acyclic) => StructuralJobOutcome::Acyclic,
            BoundedOutcome::Inconclusive(_) => StructuralJobOutcome::Inconclusive,
        }
    );

    StructuralJobJsonRun {
        envelope,
        exit_code,
    }
}

fn resolve_path(manifest_path: &Path, declared: &str) -> PathBuf {
    let declared = Path::new(declared);
    if declared.is_absolute() {
        return declared.to_path_buf();
    }
    manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(declared)
}

fn error_run(message: impl Into<String>) -> StructuralJobJsonRun {
    StructuralJobJsonRun {
        envelope: StructuralJobResultEnvelope::error(message),
        exit_code: 2,
    }
}

use crate::bounded::BoundedOutcome;
use crate::parse_declarative_model;
use crate::recurrence::{analyze_recurrence_with_limits, RecurrenceStatus};
use crate::structural_job::{parse_structural_job, StructuralJobAnalysis};
use crate::structural_result::{StructuralJobOutcome, StructuralJobResultEnvelope};
use crate::text_source::{
    path_source_id, resolve_source_id, FileSystemTextSourceProvider, TextSourceProvider,
};
use std::path::Path;

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
    let manifest_source_id = match path_source_id(manifest_path.as_ref()) {
        Ok(source_id) => source_id,
        Err(error) => return error_run(error.to_string()),
    };
    run_structural_job_json_with_provider(&FileSystemTextSourceProvider, &manifest_source_id)
}

pub fn run_structural_job_json_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> StructuralJobJsonRun {
    let input = match provider.read_text(manifest_source_id) {
        Ok(input) => input,
        Err(error) => {
            return error_run(format!(
                "failed to read structural job manifest '{}': {}",
                manifest_source_id,
                error.kind().as_str()
            ));
        }
    };

    let job = match parse_structural_job(&input) {
        Ok(job) => job,
        Err(error) => return error_run(error.to_string()),
    };

    match job.analysis() {
        StructuralJobAnalysis::Recurrence => {
            run_recurrence_job_with_provider(provider, manifest_source_id, &job)
        }
    }
}

fn run_recurrence_job_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
    job: &crate::structural_job::StructuralJob,
) -> StructuralJobJsonRun {
    let model_source_id = match resolve_source_id(manifest_source_id, job.model_path()) {
        Ok(source_id) => source_id,
        Err(error) => return error_run(error.to_string()),
    };
    let model_input = match provider.read_text(&model_source_id) {
        Ok(input) => input,
        Err(error) => {
            return error_run(format!(
                "failed to read declarative model '{}': {}",
                model_source_id,
                error.kind().as_str()
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
    let envelope =
        StructuralJobResultEnvelope::from_recurrence(model.name().to_owned(), limits, &result);
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

fn error_run(message: impl Into<String>) -> StructuralJobJsonRun {
    StructuralJobJsonRun {
        envelope: StructuralJobResultEnvelope::error(message),
        exit_code: 2,
    }
}

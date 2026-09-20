use crate::bounded::BoundedOutcome;
use crate::ctl_bounded::BoundedCtlStatus;
use crate::declarative_ctl::{
    check_declarative_ctl_text, check_declarative_ctl_text_with_limits, DeclarativeCtlStatus,
};
use crate::parse_declarative_document;
use crate::verification_job::VerificationJob;
use crate::verification_job_run::VerificationJobJsonRun;
use crate::verification_result::{VerificationJobOutcome, VerificationJobResultEnvelope};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn run_ctl_job_json(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<VerificationJobJsonRun, String> {
    validate_ctl_job(&job)?;
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
            "failed to read CTL property '{}': {error}",
            property_path.display()
        )
    })?;

    let model_limits = job.model_limits();
    let (envelope, exit_code) = if has_model_limits(model_limits) {
        let result =
            check_declarative_ctl_text_with_limits(&document, &property_input, model_limits)
                .map_err(|error| error.to_string())?;
        let exit_code = match result.evaluation.outcome {
            BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied) => 0,
            BoundedOutcome::Conclusive(BoundedCtlStatus::Violated) => 14,
            BoundedOutcome::Inconclusive(_) => 3,
        };
        let envelope = VerificationJobResultEnvelope::from_ctl_bounded(
            document.model().name().to_owned(),
            model_limits,
            &result,
        );
        (envelope, exit_code)
    } else {
        let result = check_declarative_ctl_text(&document, &property_input)
            .map_err(|error| error.to_string())?;
        let exit_code = match result.status {
            DeclarativeCtlStatus::Satisfied => 0,
            DeclarativeCtlStatus::Violated => 14,
        };
        let envelope = VerificationJobResultEnvelope::from_ctl_complete(
            document.model().name().to_owned(),
            &result,
        );
        (envelope, exit_code)
    };

    debug_assert_eq!(
        match exit_code {
            0 => VerificationJobOutcome::Satisfied,
            14 => VerificationJobOutcome::Violated,
            3 => VerificationJobOutcome::Inconclusive,
            _ => unreachable!("CTL job runner uses only stable CTL result exit codes"),
        },
        envelope.outcome
    );

    Ok(VerificationJobJsonRun {
        envelope,
        exit_code,
    })
}

pub(crate) fn ctl_error(message: impl Into<String>) -> VerificationJobResultEnvelope {
    VerificationJobResultEnvelope::ctl_error(message)
}

fn has_model_limits(limits: crate::checker::ExplorationLimits) -> bool {
    limits.max_states.is_some() || limits.max_transitions.is_some() || limits.max_depth.is_some()
}

fn validate_ctl_job(job: &VerificationJob) -> Result<(), String> {
    if !job.weak_fair_actions().is_empty() || !job.strong_fair_actions().is_empty() {
        return Err(
            "ctl verification jobs do not support weak-fair-action or strong-fair-action directives"
                .to_owned(),
        );
    }
    let product_limits = job.product_limits();
    if product_limits.max_states.is_some()
        || product_limits.max_transitions.is_some()
        || product_limits.max_depth.is_some()
    {
        return Err("ctl verification jobs do not support max-product-* limits".to_owned());
    }
    Ok(())
}

fn resolve_job_paths(manifest_path: &Path, job: &VerificationJob) -> (PathBuf, PathBuf) {
    let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
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

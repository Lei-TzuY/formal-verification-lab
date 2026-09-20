use crate::bounded::BoundedOutcome;
use crate::ctl_bounded::BoundedCtlStatus;
use crate::declarative_ctl::{
    check_declarative_ctl_text, check_declarative_ctl_text_with_limits, DeclarativeCtlStatus,
};
use crate::parse_declarative_document;
use crate::verification_job::VerificationJob;
use crate::verification_job_run::VerificationJobJsonRun;
use crate::verification_result::{VerificationJobOutcome, VerificationJobResultEnvelope};

pub(crate) fn run_ctl_job_json_from_text(
    job: VerificationJob,
    model_input: &str,
    property_input: &str,
) -> Result<VerificationJobJsonRun, String> {
    validate_ctl_job(&job)?;
    let document = parse_declarative_document(model_input).map_err(|error| error.to_string())?;

    let model_limits = job.model_limits();
    let (envelope, exit_code) = if has_model_limits(model_limits) {
        let result =
            check_declarative_ctl_text_with_limits(&document, property_input, model_limits)
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
        let result =
            check_declarative_ctl_text(&document, property_input).map_err(|error| error.to_string())?;
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

pub(crate) fn validate_ctl_job(job: &VerificationJob) -> Result<(), String> {
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

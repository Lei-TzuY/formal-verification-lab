use crate::bounded::{AnalysisLimits, AnalysisOutcome, AnalysisStage};
use crate::combined_fairness::FairnessProfile;
use crate::temporal::{
    check_action_temporal_with_fairness_profile_and_limits, AnalysisTemporalResult,
    TemporalBackend, TemporalCounterexample, TemporalObligation, TemporalStatus,
};
use crate::temporal_parse::parse_action_temporal;
use crate::verification_job::VerificationJob;
use crate::verification_job_run::VerificationJobJsonRun;
use crate::verification_result::{
    VerificationJobAccounting, VerificationJobCutoff, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobOutcome,
    VerificationJobResultEnvelope, VerificationJobStateTraceStep,
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
};
use crate::{parse_declarative_model, InconclusiveReason, TraceStep};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn run_action_temporal_job_json(
    manifest_path: &Path,
    job: VerificationJob,
) -> Result<VerificationJobJsonRun, String> {
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
            "failed to read action-temporal property '{}': {error}",
            property_path.display()
        )
    })?;
    let spec = parse_action_temporal("verification-job-action-temporal", &property_input)
        .map_err(|error| error.to_string())?;
    let canonical_property = spec.canonical_expression();

    let fairness = FairnessProfile::new(
        job.weak_fair_actions().iter().cloned(),
        job.strong_fair_actions().iter().cloned(),
    )
    .map_err(|error| error.to_string())?;
    let limits = AnalysisLimits {
        model: job.model_limits(),
        product: job.product_limits(),
    };
    let result = check_action_temporal_with_fairness_profile_and_limits(
        &model,
        &spec,
        &fairness,
        limits,
    )
    .map_err(|error| error.to_string())?;

    let envelope = action_temporal_envelope(
        model.name().to_owned(),
        canonical_property,
        &fairness,
        limits,
        &result,
    );
    let exit_code = match result.outcome {
        AnalysisOutcome::Conclusive(TemporalStatus::Satisfied) => 0,
        AnalysisOutcome::Conclusive(TemporalStatus::Violated) => 10,
        AnalysisOutcome::Inconclusive(_) => 3,
    };
    Ok(VerificationJobJsonRun {
        envelope,
        exit_code,
    })
}

pub(crate) fn action_temporal_error(message: impl Into<String>) -> VerificationJobResultEnvelope {
    let mut envelope = VerificationJobResultEnvelope::error(message);
    envelope.schema_version = VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION;
    envelope.analysis = Some("action-temporal".to_owned());
    envelope
}

fn action_temporal_envelope(
    model: String,
    property: String,
    fairness: &FairnessProfile,
    limits: AnalysisLimits,
    result: &AnalysisTemporalResult<String>,
) -> VerificationJobResultEnvelope {
    VerificationJobResultEnvelope {
        schema_version: VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
        analysis: Some("action-temporal".to_owned()),
        backend: Some(backend_name(result.backend).to_owned()),
        outcome: temporal_outcome(&result.outcome),
        model: Some(model),
        property: Some(property),
        weak_fair_actions: fairness.weak_actions().to_vec(),
        strong_fair_actions: fairness.strong_actions().to_vec(),
        model_limits: limits.model.into(),
        product_limits: limits.product.into(),
        accounting: VerificationJobAccounting {
            model_states: Some(result.model_states),
            checked_model_states: Some(result.checked_model_states),
            explored_model_transitions: Some(result.explored_model_transitions),
            retained_model_transitions: Some(result.retained_model_transitions),
            max_model_depth_reached: result.max_model_depth_reached,
            product_states: Some(result.product_states),
            checked_product_states: Some(result.checked_product_states),
            explored_product_transitions: Some(result.explored_product_transitions),
            retained_product_transitions: Some(result.retained_product_transitions),
            max_product_depth_reached: result.max_product_depth_reached,
        },
        cutoff: result
            .outcome
            .inconclusive_reason()
            .map(|reason| analysis_cutoff(reason.stage, reason.reason)),
        evidence: result.counterexample.as_ref().map(convert_evidence),
        error: None,
    }
}

fn temporal_outcome(outcome: &AnalysisOutcome<TemporalStatus>) -> VerificationJobOutcome {
    match outcome {
        AnalysisOutcome::Conclusive(TemporalStatus::Satisfied) => VerificationJobOutcome::Satisfied,
        AnalysisOutcome::Conclusive(TemporalStatus::Violated) => VerificationJobOutcome::Violated,
        AnalysisOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn backend_name(backend: TemporalBackend) -> &'static str {
    match backend {
        TemporalBackend::Response => "response",
        TemporalBackend::Buchi => "buchi",
    }
}

fn convert_evidence(counterexample: &TemporalCounterexample<String>) -> VerificationJobEvidence {
    match counterexample {
        TemporalCounterexample::Finite { obligation, trace } => {
            VerificationJobEvidence::ActionTemporalFinite {
                obligation: obligation_name(obligation),
                trace: trace.iter().map(convert_step).collect(),
            }
        }
        TemporalCounterexample::Infinite {
            obligation,
            stem,
            cycle,
        } => VerificationJobEvidence::ActionTemporalInfinite {
            obligation: obligation_name(obligation),
            stem: stem.iter().map(convert_step).collect(),
            cycle: cycle.iter().map(convert_step).collect(),
        },
    }
}

fn obligation_name(obligation: &TemporalObligation) -> String {
    match obligation {
        TemporalObligation::Response => "response".to_owned(),
        TemporalObligation::InfinitelyOftenAction(action) => {
            format!("infinitely-often:{action}")
        }
    }
}

fn convert_step(step: &TraceStep<String>) -> VerificationJobStateTraceStep {
    VerificationJobStateTraceStep {
        action: step.action.clone(),
        state: step.state.clone(),
    }
}

fn analysis_cutoff(stage: AnalysisStage, reason: InconclusiveReason) -> VerificationJobCutoff {
    let stage = match stage {
        AnalysisStage::Model => VerificationJobCutoffStage::Model,
        AnalysisStage::Product => VerificationJobCutoffStage::Product,
    };
    match reason {
        InconclusiveReason::StateLimitReached { limit } => VerificationJobCutoff {
            stage,
            kind: VerificationJobCutoffKind::StateLimit,
            limit,
        },
        InconclusiveReason::TransitionLimitReached { limit } => VerificationJobCutoff {
            stage,
            kind: VerificationJobCutoffKind::TransitionLimit,
            limit,
        },
        InconclusiveReason::DepthLimitReached { limit } => VerificationJobCutoff {
            stage,
            kind: VerificationJobCutoffKind::DepthLimit,
            limit,
        },
    }
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

use crate::bounded::{AnalysisLimits, AnalysisOutcome, BoundedOutcome};
use crate::combined_fairness::{FairnessProfile, FairnessProfileError};
use crate::multi_response::{
    check_multi_response, check_multi_response_with_fairness_profile,
    check_multi_response_with_fairness_profile_and_limits,
    check_multi_response_with_fairness_profile_and_product_limits,
    check_multi_response_with_limits, check_multi_response_with_product_limits,
    AnalysisMultiResponseResult, BoundedMultiResponseResult, MultiResponseError,
    MultiResponseProperty, MultiResponseResult, MultiResponseStatus,
};
use crate::{ExplorationLimits, TransitionSystem, VerificationJob};
use std::hash::Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiResponseExecutionConfig {
    fairness: FairnessProfile,
    model_limits: ExplorationLimits,
    product_limits: ExplorationLimits,
    has_model_limits: bool,
    has_product_limits: bool,
}

impl MultiResponseExecutionConfig {
    pub fn from_job(job: &VerificationJob) -> Result<Self, FairnessProfileError> {
        let model_limits = job.model_limits();
        let product_limits = job.product_limits();
        Ok(Self {
            fairness: FairnessProfile::new(
                job.weak_fair_actions().iter().cloned(),
                job.strong_fair_actions().iter().cloned(),
            )?,
            model_limits,
            product_limits,
            has_model_limits: has_any_limit(model_limits),
            has_product_limits: has_any_limit(product_limits),
        })
    }

    pub fn fairness(&self) -> &FairnessProfile {
        &self.fairness
    }

    pub fn model_limits(&self) -> ExplorationLimits {
        self.model_limits
    }

    pub fn product_limits(&self) -> ExplorationLimits {
        self.product_limits
    }

    pub fn has_model_limits(&self) -> bool {
        self.has_model_limits
    }

    pub fn has_product_limits(&self) -> bool {
        self.has_product_limits
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiResponseExecutionResult<S> {
    Unbounded(MultiResponseResult<S>),
    ProductBounded(BoundedMultiResponseResult<S>),
    Staged(AnalysisMultiResponseResult<S>),
}

impl<S> MultiResponseExecutionResult<S> {
    pub fn status(&self) -> BoundedOutcome<MultiResponseStatus> {
        match self {
            Self::Unbounded(result) => BoundedOutcome::Conclusive(result.status),
            Self::ProductBounded(result) => result.outcome.clone(),
            Self::Staged(result) => match &result.outcome {
                AnalysisOutcome::Conclusive(status) => BoundedOutcome::Conclusive(*status),
                AnalysisOutcome::Inconclusive(reason) => {
                    BoundedOutcome::Inconclusive(reason.reason)
                }
            },
        }
    }
}

pub fn execute_multi_response<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    config: &MultiResponseExecutionConfig,
) -> Result<MultiResponseExecutionResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if config.has_model_limits {
        let limits = AnalysisLimits::new(config.model_limits, config.product_limits);
        let result = if config.fairness.is_empty() {
            check_multi_response_with_limits(model, property, limits)?
        } else {
            check_multi_response_with_fairness_profile_and_limits(
                model,
                property,
                &config.fairness,
                limits,
            )?
        };
        return Ok(MultiResponseExecutionResult::Staged(result));
    }

    if config.has_product_limits {
        let result = if config.fairness.is_empty() {
            check_multi_response_with_product_limits(model, property, config.product_limits)?
        } else {
            check_multi_response_with_fairness_profile_and_product_limits(
                model,
                property,
                &config.fairness,
                config.product_limits,
            )?
        };
        return Ok(MultiResponseExecutionResult::ProductBounded(result));
    }

    let result = if config.fairness.is_empty() {
        check_multi_response(model, property)?
    } else {
        check_multi_response_with_fairness_profile(model, property, &config.fairness)?
    };
    Ok(MultiResponseExecutionResult::Unbounded(result))
}

fn has_any_limit(limits: ExplorationLimits) -> bool {
    limits.max_states.is_some() || limits.max_transitions.is_some() || limits.max_depth.is_some()
}

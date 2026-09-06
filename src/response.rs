use crate::bounded::{AnalysisLimits, AnalysisOutcome, BoundedOutcome};
use crate::bounded_fairness::{
    check_buchi_with_weak_fairness_and_limits, check_buchi_with_weak_fairness_and_product_limits,
};
use crate::buchi::{
    AcceptanceSet, AnalysisBuchiResult, BoundedBuchiResult, BuchiAutomaton, BuchiCounterexample,
    BuchiError, BuchiProductState, BuchiResult, BuchiStatus, FiniteRunPolicy,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::fairness::{check_buchi_with_weak_fairness, WeakFairness};
use crate::model::TransitionSystem;
use crate::multi_response::{
    check_multi_response, check_multi_response_with_limits,
    check_multi_response_with_product_limits, AnalysisMultiResponseResult,
    BoundedMultiResponseResult, MultiObligationState, MultiResponseCounterexample,
    MultiResponseError, MultiResponseProperty, MultiResponseStatus,
};
use crate::recurrence::RecurrenceError;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

/// A named action-level response property:
/// every trigger action must eventually be followed by a response action.
pub struct ResponseProperty {
    name: String,
    trigger: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    response: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl Clone for ResponseProperty {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            trigger: Arc::clone(&self.trigger),
            response: Arc::clone(&self.response),
        }
    }
}

impl fmt::Debug for ResponseProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseProperty")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl ResponseProperty {
    pub fn new(
        name: impl Into<String>,
        trigger: impl Fn(&str) -> bool + Send + Sync + 'static,
        response: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Result<Self, ResponseError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ResponseError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            trigger: Arc::new(trigger),
            response: Arc::new(response),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Product state for the public single-clause response compatibility API.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObligationState<S> {
    pub state: S,
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseCounterexample<S> {
    /// A maximal finite execution ends while a trigger remains unanswered.
    Finite {
        trace: Vec<TraceStep<ObligationState<S>>>,
    },
    /// A reachable execution can remain forever with an unanswered trigger.
    Infinite {
        stem: Vec<TraceStep<ObligationState<S>>>,
        cycle: Vec<TraceStep<ObligationState<S>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseResult<S> {
    pub property: String,
    pub status: ResponseStatus,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub counterexample: Option<ResponseCounterexample<S>>,
}

/// Single-clause response result when only action-product construction is
/// resource bounded. Model capture remains exhaustive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedResponseResult<S> {
    pub property: String,
    pub outcome: BoundedOutcome<ResponseStatus>,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub counterexample: Option<ResponseCounterexample<S>>,
}

/// Single-clause response result under independently configured model and
/// product exploration budgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResponseResult<S> {
    pub property: String,
    pub outcome: AnalysisOutcome<ResponseStatus>,
    pub model_completion: BoundedOutcome<()>,
    pub product_completion: BoundedOutcome<()>,
    pub model_states: usize,
    pub checked_model_states: usize,
    pub explored_model_transitions: usize,
    pub retained_model_transitions: usize,
    pub max_model_depth_reached: Option<usize>,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub counterexample: Option<ResponseCounterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseError {
    EmptyPropertyName,
    Graph(RecurrenceError),
    Buchi(BuchiError),
    MissingFiniteWitness,
    MissingCycleWitness,
    AdapterInvariant,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "response property name must not be empty"),
            Self::Graph(error) => write!(f, "response graph analysis failed: {error}"),
            Self::Buchi(error) => write!(f, "response fairness analysis failed: {error}"),
            Self::MissingFiniteWitness => write!(
                f,
                "pending terminal product state did not yield a counterexample trace"
            ),
            Self::MissingCycleWitness => write!(
                f,
                "pending cyclic product component did not yield a stem-plus-cycle counterexample"
            ),
            Self::AdapterInvariant => write!(
                f,
                "single-clause response adapter observed an invalid multi-clause monitor state"
            ),
        }
    }
}

impl std::error::Error for ResponseError {}

impl From<RecurrenceError> for ResponseError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

impl From<BuchiError> for ResponseError {
    fn from(value: BuchiError) -> Self {
        Self::Buchi(value)
    }
}

/// Verify `trigger -> eventually response` over every maximal execution of the
/// finite model, without a fairness assumption.
///
/// Milestone 11 makes the multi-clause monitor the canonical response engine;
/// this function is now a compatibility adapter that constructs one clause and
/// maps the resulting one-bit vector state back to `ObligationState<S>`.
pub fn check_response<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
) -> Result<ResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    let multi_property = single_multi_property(property);
    let result = check_multi_response(model, &multi_property).map_err(map_multi_error)?;
    let counterexample = collapse_counterexample(result.counterexample)?;

    Ok(ResponseResult {
        property: result.property,
        status: map_status(result.status),
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        counterexample,
    })
}

/// Verify one response obligation only over infinite executions admitted by the
/// exact-action weak-fairness contract.
///
/// For non-empty fairness, the response monitor is compiled to one deterministic
/// pending-bit generalized Buchi automaton. Its acceptance set is `!pending`
/// and strict finite-terminal policy preserves the ordinary response rule that
/// a terminal execution with an unanswered trigger is a violation. A response
/// action wins when it also matches the trigger, exactly as in M10/M11.
///
/// Empty fairness deliberately delegates to the canonical M10/M11 adapter so
/// all historical accounting and witness tie-breaking remain exactly unchanged.
pub fn check_response_with_weak_fairness<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
    fairness: &WeakFairness,
) -> Result<ResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.actions().is_empty() {
        return check_response(model, property);
    }

    let automaton = response_buchi_automaton(property)?;
    let result = check_buchi_with_weak_fairness(model, &automaton, fairness)?;
    normalize_fair_buchi_result(property, result)
}

/// Verify a single response obligation while bounding only shared action-product
/// construction. A real finite/cyclic violation in the retained product prefix
/// remains conclusive; satisfaction requires complete product construction.
pub fn check_response_with_product_limits<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
    limits: ExplorationLimits,
) -> Result<BoundedResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    let multi_property = single_multi_property(property);
    let result = check_multi_response_with_product_limits(model, &multi_property, limits)
        .map_err(map_multi_error)?;
    normalize_bounded_result(result)
}

/// Verify weak-fair single-response semantics while bounding only product
/// construction after complete model capture. Enablement authority and retained
/// fair-cycle honesty are inherited from the M33 bounded weak-fair Buchi path.
pub fn check_response_with_weak_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.actions().is_empty() {
        return check_response_with_product_limits(model, property, limits);
    }

    let automaton = response_buchi_automaton(property)?;
    let result =
        check_buchi_with_weak_fairness_and_product_limits(model, &automaton, fairness, limits)?;
    normalize_fair_bounded_buchi_result(property, result)
}

/// Verify a single response obligation under independent model-capture and
/// product-construction limits. A justified finite/cyclic violation remains
/// conclusive from a prefix; satisfaction requires both stages to complete.
pub fn check_response_with_limits<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
    limits: AnalysisLimits,
) -> Result<AnalysisResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    let multi_property = single_multi_property(property);
    let result = check_multi_response_with_limits(model, &multi_property, limits)
        .map_err(map_multi_error)?;
    normalize_analysis_result(result)
}

/// Verify weak-fair single-response semantics under independent model/product
/// budgets. Unknown model-prefix action enablement remains fail-closed through
/// the M33 provenance-aware Buchi backend, while real retained fair cycles and
/// pending terminals may still prove a violation before a later cutoff.
pub fn check_response_with_weak_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
    fairness: &WeakFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.actions().is_empty() {
        return check_response_with_limits(model, property, limits);
    }

    let automaton = response_buchi_automaton(property)?;
    let result = check_buchi_with_weak_fairness_and_limits(model, &automaton, fairness, limits)?;
    normalize_fair_analysis_buchi_result(property, result)
}

fn response_buchi_automaton(
    property: &ResponseProperty,
) -> Result<BuchiAutomaton<bool>, ResponseError> {
    let trigger = Arc::clone(&property.trigger);
    let response = Arc::clone(&property.response);
    let acceptance = AcceptanceSet::new("response-discharged", |pending: &bool| !*pending)?;
    Ok(BuchiAutomaton::new(
        format!("{}-weak-fair-response", property.name),
        false,
        move |pending, action| {
            if response(action) {
                false
            } else if trigger(action) {
                true
            } else {
                *pending
            }
        },
        vec![acceptance],
        FiniteRunPolicy::RequireAcceptingTerminal,
    )?)
}

fn single_multi_property(property: &ResponseProperty) -> MultiResponseProperty {
    MultiResponseProperty::from_single_shared(
        property.name.clone(),
        Arc::clone(&property.trigger),
        Arc::clone(&property.response),
    )
}

fn normalize_fair_buchi_result<S>(
    property: &ResponseProperty,
    result: BuchiResult<S, bool>,
) -> Result<ResponseResult<S>, ResponseError> {
    Ok(ResponseResult {
        property: property.name.clone(),
        status: map_buchi_status(result.status),
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        counterexample: collapse_buchi_counterexample(result.counterexample),
    })
}

fn normalize_fair_bounded_buchi_result<S>(
    property: &ResponseProperty,
    result: BoundedBuchiResult<S, bool>,
) -> Result<BoundedResponseResult<S>, ResponseError> {
    Ok(BoundedResponseResult {
        property: property.name.clone(),
        outcome: match result.outcome {
            BoundedOutcome::Conclusive(status) => {
                BoundedOutcome::Conclusive(map_buchi_status(status))
            }
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        },
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        checked_product_states: result.checked_product_states,
        explored_product_transitions: result.explored_product_transitions,
        retained_product_transitions: result.retained_product_transitions,
        max_product_depth_reached: result.max_product_depth_reached,
        counterexample: collapse_buchi_counterexample(result.counterexample),
    })
}

fn normalize_fair_analysis_buchi_result<S>(
    property: &ResponseProperty,
    result: AnalysisBuchiResult<S, bool>,
) -> Result<AnalysisResponseResult<S>, ResponseError> {
    Ok(AnalysisResponseResult {
        property: property.name.clone(),
        outcome: match result.outcome {
            AnalysisOutcome::Conclusive(status) => {
                AnalysisOutcome::Conclusive(map_buchi_status(status))
            }
            AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(reason),
        },
        model_completion: result.model_completion,
        product_completion: result.product_completion,
        model_states: result.model_states,
        checked_model_states: result.checked_model_states,
        explored_model_transitions: result.explored_model_transitions,
        retained_model_transitions: result.retained_model_transitions,
        max_model_depth_reached: result.max_model_depth_reached,
        product_states: result.product_states,
        checked_product_states: result.checked_product_states,
        explored_product_transitions: result.explored_product_transitions,
        retained_product_transitions: result.retained_product_transitions,
        max_product_depth_reached: result.max_product_depth_reached,
        counterexample: collapse_buchi_counterexample(result.counterexample),
    })
}

fn normalize_bounded_result<S>(
    result: BoundedMultiResponseResult<S>,
) -> Result<BoundedResponseResult<S>, ResponseError> {
    let counterexample = collapse_counterexample(result.counterexample)?;
    let outcome = match result.outcome {
        BoundedOutcome::Conclusive(status) => BoundedOutcome::Conclusive(map_status(status)),
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };

    Ok(BoundedResponseResult {
        property: result.property,
        outcome,
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        checked_product_states: result.checked_product_states,
        explored_product_transitions: result.explored_product_transitions,
        retained_product_transitions: result.retained_product_transitions,
        max_product_depth_reached: result.max_product_depth_reached,
        counterexample,
    })
}

fn normalize_analysis_result<S>(
    result: AnalysisMultiResponseResult<S>,
) -> Result<AnalysisResponseResult<S>, ResponseError> {
    let counterexample = collapse_counterexample(result.counterexample)?;
    let outcome = match result.outcome {
        AnalysisOutcome::Conclusive(status) => AnalysisOutcome::Conclusive(map_status(status)),
        AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(reason),
    };

    Ok(AnalysisResponseResult {
        property: result.property,
        outcome,
        model_completion: result.model_completion,
        product_completion: result.product_completion,
        model_states: result.model_states,
        checked_model_states: result.checked_model_states,
        explored_model_transitions: result.explored_model_transitions,
        retained_model_transitions: result.retained_model_transitions,
        max_model_depth_reached: result.max_model_depth_reached,
        product_states: result.product_states,
        checked_product_states: result.checked_product_states,
        explored_product_transitions: result.explored_product_transitions,
        retained_product_transitions: result.retained_product_transitions,
        max_product_depth_reached: result.max_product_depth_reached,
        counterexample,
    })
}

fn map_status(status: MultiResponseStatus) -> ResponseStatus {
    match status {
        MultiResponseStatus::Satisfied => ResponseStatus::Satisfied,
        MultiResponseStatus::Violated => ResponseStatus::Violated,
    }
}

fn map_buchi_status(status: BuchiStatus) -> ResponseStatus {
    match status {
        BuchiStatus::Satisfied => ResponseStatus::Satisfied,
        BuchiStatus::Violated => ResponseStatus::Violated,
    }
}

fn collapse_buchi_counterexample<S>(
    counterexample: Option<BuchiCounterexample<S, bool>>,
) -> Option<ResponseCounterexample<S>> {
    match counterexample {
        None => None,
        Some(BuchiCounterexample::FiniteTerminal { trace, .. }) => {
            Some(ResponseCounterexample::Finite {
                trace: collapse_buchi_trace(trace),
            })
        }
        Some(BuchiCounterexample::AcceptanceAvoidingCycle { stem, cycle, .. }) => {
            Some(ResponseCounterexample::Infinite {
                stem: collapse_buchi_trace(stem),
                cycle: collapse_buchi_trace(cycle),
            })
        }
    }
}

fn collapse_buchi_trace<S>(
    trace: Vec<TraceStep<BuchiProductState<S, bool>>>,
) -> Vec<TraceStep<ObligationState<S>>> {
    trace
        .into_iter()
        .map(|step| TraceStep {
            action: step.action,
            state: ObligationState {
                state: step.state.state,
                pending: step.state.automaton,
            },
        })
        .collect()
}

fn collapse_counterexample<S>(
    counterexample: Option<MultiResponseCounterexample<S>>,
) -> Result<Option<ResponseCounterexample<S>>, ResponseError> {
    match counterexample {
        None => Ok(None),
        Some(MultiResponseCounterexample::Finite { trace, .. }) => {
            Ok(Some(ResponseCounterexample::Finite {
                trace: collapse_trace(trace)?,
            }))
        }
        Some(MultiResponseCounterexample::Infinite { stem, cycle, .. }) => {
            Ok(Some(ResponseCounterexample::Infinite {
                stem: collapse_trace(stem)?,
                cycle: collapse_trace(cycle)?,
            }))
        }
    }
}

fn collapse_trace<S>(
    trace: Vec<TraceStep<MultiObligationState<S>>>,
) -> Result<Vec<TraceStep<ObligationState<S>>>, ResponseError> {
    trace
        .into_iter()
        .map(|step| {
            if step.state.pending.len() != 1 {
                return Err(ResponseError::AdapterInvariant);
            }
            Ok(TraceStep {
                action: step.action,
                state: ObligationState {
                    state: step.state.state,
                    pending: step.state.pending[0],
                },
            })
        })
        .collect()
}

fn map_multi_error(error: MultiResponseError) -> ResponseError {
    match error {
        MultiResponseError::Graph(error) => ResponseError::Graph(error),
        MultiResponseError::Buchi(error) => ResponseError::Buchi(error),
        MultiResponseError::MissingFiniteWitness => ResponseError::MissingFiniteWitness,
        MultiResponseError::MissingCycleWitness => ResponseError::MissingCycleWitness,
        MultiResponseError::FairnessAdapterInvariant
        | MultiResponseError::EmptyPropertyName
        | MultiResponseError::NoClauses
        | MultiResponseError::EmptyClauseName
        | MultiResponseError::DuplicateClauseName { .. } => ResponseError::AdapterInvariant,
    }
}

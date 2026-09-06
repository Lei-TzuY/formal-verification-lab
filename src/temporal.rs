use crate::bounded::{AnalysisLimits, AnalysisOutcome, BoundedOutcome};
use crate::bounded_fairness::{
    check_buchi_with_weak_fairness_and_limits, check_buchi_with_weak_fairness_and_product_limits,
};
use crate::bounded_strong_fairness::{
    check_buchi_with_strong_fairness_and_limits,
    check_buchi_with_strong_fairness_and_product_limits,
};
use crate::buchi::{
    check_buchi, check_buchi_with_limits, check_buchi_with_product_limits, AcceptanceSet,
    BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState, BuchiStatus,
    FiniteRunPolicy,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::fairness::{check_buchi_with_weak_fairness, WeakFairness};
use crate::model::TransitionSystem;
use crate::response::{
    check_response, check_response_with_limits, check_response_with_product_limits,
    check_response_with_strong_fairness, check_response_with_strong_fairness_and_limits,
    check_response_with_strong_fairness_and_product_limits, check_response_with_weak_fairness,
    check_response_with_weak_fairness_and_limits,
    check_response_with_weak_fairness_and_product_limits, ObligationState, ResponseCounterexample,
    ResponseError, ResponseProperty, ResponseStatus,
};
use crate::strong_fairness::{check_buchi_with_strong_fairness, StrongFairness};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

/// One exact action-label atom used by the typed temporal frontend.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionAtom(String);

impl ActionAtom {
    pub fn exact(action: impl Into<String>) -> Result<Self, TemporalError> {
        let action = action.into();
        if action.trim().is_empty() {
            return Err(TemporalError::EmptyActionName);
        }
        Ok(Self(action))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActionTemporalKind {
    Response {
        trigger: ActionAtom,
        response: ActionAtom,
    },
    AllInfinitelyOften {
        actions: Vec<ActionAtom>,
    },
}

/// A deliberately small typed action-temporal specification.
///
/// This is not a general LTL/CTL AST. The supported forms compile only to
/// already validated explicit-state backends:
/// - exact-action response: every trigger is eventually followed by response;
/// - exact actions that must each occur infinitely often on every infinite run.
///
/// `AllInfinitelyOften` deliberately ignores finite terminal executions because
/// its obligation is defined only over infinite runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionTemporalSpec {
    name: String,
    kind: ActionTemporalKind,
}

impl ActionTemporalSpec {
    pub fn response(
        name: impl Into<String>,
        trigger: ActionAtom,
        response: ActionAtom,
    ) -> Result<Self, TemporalError> {
        Ok(Self {
            name: validate_property_name(name)?,
            kind: ActionTemporalKind::Response { trigger, response },
        })
    }

    pub fn all_infinitely_often(
        name: impl Into<String>,
        actions: Vec<ActionAtom>,
    ) -> Result<Self, TemporalError> {
        let name = validate_property_name(name)?;
        if actions.is_empty() {
            return Err(TemporalError::NoRecurringActions);
        }
        let mut seen = HashSet::new();
        for action in &actions {
            if !seen.insert(action.0.clone()) {
                return Err(TemporalError::DuplicateRecurringAction {
                    action: action.0.clone(),
                });
            }
        }
        Ok(Self {
            name,
            kind: ActionTemporalKind::AllInfinitelyOften { actions },
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Render the canonical textual form consumed by the M18 parser.
    pub fn canonical_expression(&self) -> String {
        match &self.kind {
            ActionTemporalKind::Response { trigger, response } => format!(
                "response({},{})",
                quote_action(trigger.as_str()),
                quote_action(response.as_str())
            ),
            ActionTemporalKind::AllInfinitelyOften { actions } => format!(
                "infinitely-often({})",
                actions
                    .iter()
                    .map(|action| quote_action(action.as_str()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

fn quote_action(action: &str) -> String {
    let mut output = String::from("\"");
    for ch in action.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

fn validate_property_name(name: impl Into<String>) -> Result<String, TemporalError> {
    let name = name.into();
    if name.trim().is_empty() {
        Err(TemporalError::EmptyPropertyName)
    } else {
        Ok(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalBackend {
    Response,
    Buchi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalObligation {
    Response,
    InfinitelyOftenAction(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalCounterexample<S> {
    Finite {
        obligation: TemporalObligation,
        trace: Vec<TraceStep<S>>,
    },
    Infinite {
        obligation: TemporalObligation,
        stem: Vec<TraceStep<S>>,
        cycle: Vec<TraceStep<S>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalResult<S> {
    pub property: String,
    pub backend: TemporalBackend,
    pub status: TemporalStatus,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub counterexample: Option<TemporalCounterexample<S>>,
}

/// Typed temporal result when only deterministic action-product construction is
/// resource bounded. Model capture remains exhaustive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedTemporalResult<S> {
    pub property: String,
    pub backend: TemporalBackend,
    pub outcome: BoundedOutcome<TemporalStatus>,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub counterexample: Option<TemporalCounterexample<S>>,
}

/// Typed temporal result under independent model-capture and product budgets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisTemporalResult<S> {
    pub property: String,
    pub backend: TemporalBackend,
    pub outcome: AnalysisOutcome<TemporalStatus>,
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
    pub counterexample: Option<TemporalCounterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalError {
    EmptyPropertyName,
    EmptyActionName,
    NoRecurringActions,
    DuplicateRecurringAction {
        action: String,
    },
    /// Legacy compatibility marker retained for callers that matched this
    /// variant before weak-fair response semantics were implemented.
    WeakFairnessUnsupportedForResponse,
    Response(ResponseError),
    Buchi(BuchiError),
    UnexpectedFiniteBuchiCounterexample,
}

impl fmt::Display for TemporalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "temporal property name must not be empty"),
            Self::EmptyActionName => write!(f, "temporal action name must not be empty"),
            Self::NoRecurringActions => write!(
                f,
                "all-infinitely-often temporal property requires at least one action"
            ),
            Self::DuplicateRecurringAction { action } => {
                write!(f, "duplicate infinitely-often action '{action}'")
            }
            Self::WeakFairnessUnsupportedForResponse => write!(
                f,
                "legacy weak-fair response unsupported marker is no longer emitted"
            ),
            Self::Response(error) => write!(f, "response backend failed: {error}"),
            Self::Buchi(error) => write!(f, "Buchi backend failed: {error}"),
            Self::UnexpectedFiniteBuchiCounterexample => write!(
                f,
                "infinite-run-only temporal property unexpectedly produced a finite Buchi counterexample"
            ),
        }
    }
}

impl std::error::Error for TemporalError {}

impl From<ResponseError> for TemporalError {
    fn from(value: ResponseError) -> Self {
        Self::Response(value)
    }
}

impl From<BuchiError> for TemporalError {
    fn from(value: BuchiError) -> Self {
        Self::Buchi(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct LastObservedAction(Option<usize>);

/// Compile and verify one typed action-temporal specification through an
/// existing validated backend, then erase backend control state from witnesses.
pub fn check_action_temporal<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec(model, spec, trigger, response)
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec(model, spec, actions)
        }
    }
}

/// Verify a typed action-temporal specification while quantifying only over
/// executions admitted by explicit exact-action weak fairness.
///
/// Response specifications route through the M34 pending-obligation backend;
/// recurring-action specifications route through the generalized Büchi backend.
/// Empty fairness preserves each corresponding historical no-fairness path.
pub fn check_action_temporal_with_weak_fairness<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    fairness: &WeakFairness,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_weak_fairness(model, spec, trigger, response, fairness)
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_weak_fairness(model, spec, actions, fairness)
        }
    }
}

/// Verify a typed action-temporal specification over executions admitted by
/// exact-action strong fairness. Response properties reuse the M40 response
/// adapter; recurring-action properties reuse the M38 generalized Büchi path.
pub fn check_action_temporal_with_strong_fairness<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    fairness: &StrongFairness,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_strong_fairness(model, spec, trigger, response, fairness)
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_strong_fairness(model, spec, actions, fairness)
        }
    }
}

/// Verify exact-action weak fairness while bounding only action-product
/// construction after complete model capture.
pub fn check_action_temporal_with_weak_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_weak_fairness_and_product_limits(
                model, spec, trigger, response, fairness, limits,
            )
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_weak_fairness_and_product_limits(
                model, spec, actions, fairness, limits,
            )
        }
    }
}

/// Verify exact-action strong fairness while bounding only action-product
/// construction after complete model capture, preserving M39 enablement
/// authority and product-cutoff incompleteness.
pub fn check_action_temporal_with_strong_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    fairness: &StrongFairness,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_strong_fairness_and_product_limits(
                model, spec, trigger, response, fairness, limits,
            )
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_strong_fairness_and_product_limits(
                model, spec, actions, fairness, limits,
            )
        }
    }
}

/// Verify exact-action weak fairness under independent deterministic model and
/// product budgets, preserving stage-qualified incompleteness.
pub fn check_action_temporal_with_weak_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    fairness: &WeakFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_weak_fairness_and_limits(
                model, spec, trigger, response, fairness, limits,
            )
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_weak_fairness_and_limits(
                model, spec, actions, fairness, limits,
            )
        }
    }
}

/// Verify exact-action strong fairness under independent model/product budgets,
/// preserving M39 stage-qualified incompleteness and conservative enablement
/// provenance.
pub fn check_action_temporal_with_strong_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    fairness: &StrongFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_strong_fairness_and_limits(
                model, spec, trigger, response, fairness, limits,
            )
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_strong_fairness_and_limits(
                model, spec, actions, fairness, limits,
            )
        }
    }
}

/// Compile and verify one typed action-temporal specification while bounding
/// only the shared action-product phase of the selected backend.
pub fn check_action_temporal_with_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_product_limits(model, spec, trigger, response, limits)
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_product_limits(model, spec, actions, limits)
        }
    }
}

/// Compile and verify one typed action-temporal specification under independent
/// model-capture and product-construction budgets.
pub fn check_action_temporal_with_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    match &spec.kind {
        ActionTemporalKind::Response { trigger, response } => {
            check_response_spec_with_limits(model, spec, trigger, response, limits)
        }
        ActionTemporalKind::AllInfinitelyOften { actions } => {
            check_recurring_spec_with_limits(model, spec, actions, limits)
        }
    }
}

fn check_response_spec<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result = check_response(model, &property)?;
    temporal_result_from_response(result)
}

fn check_response_spec_with_weak_fairness<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    fairness: &WeakFairness,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result = check_response_with_weak_fairness(model, &property, fairness)?;
    temporal_result_from_response(result)
}

fn check_response_spec_with_strong_fairness<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    fairness: &StrongFairness,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result = check_response_with_strong_fairness(model, &property, fairness)?;
    temporal_result_from_response(result)
}

fn temporal_result_from_response<S>(
    result: crate::response::ResponseResult<S>,
) -> Result<TemporalResult<S>, TemporalError> {
    let counterexample = normalize_response_counterexample(result.counterexample);
    Ok(TemporalResult {
        property: result.property,
        backend: TemporalBackend::Response,
        status: map_response_status(result.status),
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        counterexample,
    })
}

fn check_response_spec_with_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result = check_response_with_product_limits(model, &property, limits)?;
    bounded_temporal_result_from_response(result)
}

fn check_response_spec_with_weak_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result =
        check_response_with_weak_fairness_and_product_limits(model, &property, fairness, limits)?;
    bounded_temporal_result_from_response(result)
}

fn check_response_spec_with_strong_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    fairness: &StrongFairness,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result =
        check_response_with_strong_fairness_and_product_limits(model, &property, fairness, limits)?;
    bounded_temporal_result_from_response(result)
}

fn bounded_temporal_result_from_response<S>(
    result: crate::response::BoundedResponseResult<S>,
) -> Result<BoundedTemporalResult<S>, TemporalError> {
    let counterexample = normalize_response_counterexample(result.counterexample);
    Ok(BoundedTemporalResult {
        property: result.property,
        backend: TemporalBackend::Response,
        outcome: map_bounded_response_outcome(result.outcome),
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

fn check_response_spec_with_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result = check_response_with_limits(model, &property, limits)?;
    analysis_temporal_result_from_response(result)
}

fn check_response_spec_with_weak_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    fairness: &WeakFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result = check_response_with_weak_fairness_and_limits(model, &property, fairness, limits)?;
    analysis_temporal_result_from_response(result)
}

fn check_response_spec_with_strong_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
    fairness: &StrongFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let property = response_property(spec, trigger, response)?;
    let result =
        check_response_with_strong_fairness_and_limits(model, &property, fairness, limits)?;
    analysis_temporal_result_from_response(result)
}

fn analysis_temporal_result_from_response<S>(
    result: crate::response::AnalysisResponseResult<S>,
) -> Result<AnalysisTemporalResult<S>, TemporalError> {
    let counterexample = normalize_response_counterexample(result.counterexample);
    Ok(AnalysisTemporalResult {
        property: result.property,
        backend: TemporalBackend::Response,
        outcome: map_analysis_response_outcome(result.outcome),
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

fn response_property(
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
) -> Result<ResponseProperty, TemporalError> {
    let trigger = trigger.0.clone();
    let response = response.0.clone();
    Ok(ResponseProperty::new(
        spec.name.clone(),
        move |action| action == trigger,
        move |action| action == response,
    )?)
}

fn normalize_response_counterexample<S>(
    counterexample: Option<ResponseCounterexample<S>>,
) -> Option<TemporalCounterexample<S>> {
    match counterexample {
        None => None,
        Some(ResponseCounterexample::Finite { trace }) => Some(TemporalCounterexample::Finite {
            obligation: TemporalObligation::Response,
            trace: strip_response_trace(trace),
        }),
        Some(ResponseCounterexample::Infinite { stem, cycle }) => {
            Some(TemporalCounterexample::Infinite {
                obligation: TemporalObligation::Response,
                stem: strip_response_trace(stem),
                cycle: strip_response_trace(cycle),
            })
        }
    }
}

fn map_response_status(status: ResponseStatus) -> TemporalStatus {
    match status {
        ResponseStatus::Satisfied => TemporalStatus::Satisfied,
        ResponseStatus::Violated => TemporalStatus::Violated,
    }
}

fn map_bounded_response_outcome(
    outcome: BoundedOutcome<ResponseStatus>,
) -> BoundedOutcome<TemporalStatus> {
    match outcome {
        BoundedOutcome::Conclusive(status) => {
            BoundedOutcome::Conclusive(map_response_status(status))
        }
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    }
}

fn map_analysis_response_outcome(
    outcome: AnalysisOutcome<ResponseStatus>,
) -> AnalysisOutcome<TemporalStatus> {
    match outcome {
        AnalysisOutcome::Conclusive(status) => {
            AnalysisOutcome::Conclusive(map_response_status(status))
        }
        AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(reason),
    }
}

fn check_recurring_spec<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi(model, &automaton)?;
    temporal_result_from_buchi(result)
}

fn check_recurring_spec_with_weak_fairness<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    fairness: &WeakFairness,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi_with_weak_fairness(model, &automaton, fairness)?;
    temporal_result_from_buchi(result)
}

fn check_recurring_spec_with_strong_fairness<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    fairness: &StrongFairness,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi_with_strong_fairness(model, &automaton, fairness)?;
    temporal_result_from_buchi(result)
}

fn check_recurring_spec_with_weak_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result =
        check_buchi_with_weak_fairness_and_product_limits(model, &automaton, fairness, limits)?;
    bounded_temporal_result_from_buchi(result)
}

fn check_recurring_spec_with_strong_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    fairness: &StrongFairness,
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result =
        check_buchi_with_strong_fairness_and_product_limits(model, &automaton, fairness, limits)?;
    bounded_temporal_result_from_buchi(result)
}

fn check_recurring_spec_with_weak_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    fairness: &WeakFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi_with_weak_fairness_and_limits(model, &automaton, fairness, limits)?;
    analysis_temporal_result_from_buchi(result)
}

fn check_recurring_spec_with_strong_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    fairness: &StrongFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi_with_strong_fairness_and_limits(model, &automaton, fairness, limits)?;
    analysis_temporal_result_from_buchi(result)
}

fn temporal_result_from_buchi<S>(
    result: crate::buchi::BuchiResult<S, LastObservedAction>,
) -> Result<TemporalResult<S>, TemporalError> {
    let counterexample = normalize_buchi_counterexample(result.counterexample)?;
    Ok(TemporalResult {
        property: result.automaton,
        backend: TemporalBackend::Buchi,
        status: map_buchi_status(result.status),
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        counterexample,
    })
}

fn check_recurring_spec_with_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    limits: ExplorationLimits,
) -> Result<BoundedTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi_with_product_limits(model, &automaton, limits)?;
    bounded_temporal_result_from_buchi(result)
}

fn bounded_temporal_result_from_buchi<S>(
    result: crate::buchi::BoundedBuchiResult<S, LastObservedAction>,
) -> Result<BoundedTemporalResult<S>, TemporalError> {
    let counterexample = normalize_buchi_counterexample(result.counterexample)?;
    Ok(BoundedTemporalResult {
        property: result.automaton,
        backend: TemporalBackend::Buchi,
        outcome: map_bounded_buchi_outcome(result.outcome),
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

fn check_recurring_spec_with_limits<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
    limits: AnalysisLimits,
) -> Result<AnalysisTemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let automaton = recurring_automaton(spec, actions)?;
    let result = check_buchi_with_limits(model, &automaton, limits)?;
    analysis_temporal_result_from_buchi(result)
}

fn analysis_temporal_result_from_buchi<S>(
    result: crate::buchi::AnalysisBuchiResult<S, LastObservedAction>,
) -> Result<AnalysisTemporalResult<S>, TemporalError> {
    let counterexample = normalize_buchi_counterexample(result.counterexample)?;
    Ok(AnalysisTemporalResult {
        property: result.automaton,
        backend: TemporalBackend::Buchi,
        outcome: map_analysis_buchi_outcome(result.outcome),
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

fn recurring_automaton(
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
) -> Result<BuchiAutomaton<LastObservedAction>, TemporalError> {
    let names = actions
        .iter()
        .map(|action| action.0.clone())
        .collect::<Vec<_>>();
    let step_names = names.clone();
    let acceptance = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            AcceptanceSet::new(name.clone(), move |state: &LastObservedAction| {
                state.0 == Some(index)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BuchiAutomaton::new(
        spec.name.clone(),
        LastObservedAction(None),
        move |_state, action| {
            LastObservedAction(step_names.iter().position(|candidate| candidate == action))
        },
        acceptance,
        FiniteRunPolicy::IgnoreTerminals,
    )?)
}

fn normalize_buchi_counterexample<S>(
    counterexample: Option<BuchiCounterexample<S, LastObservedAction>>,
) -> Result<Option<TemporalCounterexample<S>>, TemporalError> {
    match counterexample {
        None => Ok(None),
        Some(BuchiCounterexample::FiniteTerminal { .. }) => {
            Err(TemporalError::UnexpectedFiniteBuchiCounterexample)
        }
        Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        }) => Ok(Some(TemporalCounterexample::Infinite {
            obligation: TemporalObligation::InfinitelyOftenAction(acceptance),
            stem: strip_buchi_trace(stem),
            cycle: strip_buchi_trace(cycle),
        })),
    }
}

fn map_buchi_status(status: BuchiStatus) -> TemporalStatus {
    match status {
        BuchiStatus::Satisfied => TemporalStatus::Satisfied,
        BuchiStatus::Violated => TemporalStatus::Violated,
    }
}

fn map_bounded_buchi_outcome(
    outcome: BoundedOutcome<BuchiStatus>,
) -> BoundedOutcome<TemporalStatus> {
    match outcome {
        BoundedOutcome::Conclusive(status) => BoundedOutcome::Conclusive(map_buchi_status(status)),
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    }
}

fn map_analysis_buchi_outcome(
    outcome: AnalysisOutcome<BuchiStatus>,
) -> AnalysisOutcome<TemporalStatus> {
    match outcome {
        AnalysisOutcome::Conclusive(status) => {
            AnalysisOutcome::Conclusive(map_buchi_status(status))
        }
        AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(reason),
    }
}

fn strip_response_trace<S>(trace: Vec<TraceStep<ObligationState<S>>>) -> Vec<TraceStep<S>> {
    trace
        .into_iter()
        .map(|step| TraceStep {
            action: step.action,
            state: step.state.state,
        })
        .collect()
}

fn strip_buchi_trace<S, A>(trace: Vec<TraceStep<BuchiProductState<S, A>>>) -> Vec<TraceStep<S>> {
    trace
        .into_iter()
        .map(|step| TraceStep {
            action: step.action,
            state: step.state.state,
        })
        .collect()
}

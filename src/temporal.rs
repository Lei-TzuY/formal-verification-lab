use crate::buchi::{
    check_buchi, AcceptanceSet, BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState,
    BuchiStatus, FiniteRunPolicy,
};
use crate::checker::TraceStep;
use crate::model::TransitionSystem;
use crate::response::{
    check_response, ObligationState, ResponseCounterexample, ResponseError, ResponseProperty,
    ResponseStatus,
};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalError {
    EmptyPropertyName,
    EmptyActionName,
    NoRecurringActions,
    DuplicateRecurringAction { action: String },
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

fn check_response_spec<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    trigger: &ActionAtom,
    response: &ActionAtom,
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
    let trigger = trigger.0.clone();
    let response = response.0.clone();
    let property = ResponseProperty::new(
        spec.name.clone(),
        move |action| action == trigger,
        move |action| action == response,
    )?;
    let result = check_response(model, &property)?;
    let counterexample = match result.counterexample {
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
    };

    Ok(TemporalResult {
        property: result.property,
        backend: TemporalBackend::Response,
        status: match result.status {
            ResponseStatus::Satisfied => TemporalStatus::Satisfied,
            ResponseStatus::Violated => TemporalStatus::Violated,
        },
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        counterexample,
    })
}

fn check_recurring_spec<S>(
    model: &TransitionSystem<S>,
    spec: &ActionTemporalSpec,
    actions: &[ActionAtom],
) -> Result<TemporalResult<S>, TemporalError>
where
    S: Clone + Eq + Hash,
{
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
    let automaton = BuchiAutomaton::new(
        spec.name.clone(),
        LastObservedAction(None),
        move |_state, action| {
            LastObservedAction(step_names.iter().position(|candidate| candidate == action))
        },
        acceptance,
        FiniteRunPolicy::IgnoreTerminals,
    )?;
    let result = check_buchi(model, &automaton)?;
    let counterexample = match result.counterexample {
        None => None,
        Some(BuchiCounterexample::FiniteTerminal { .. }) => {
            return Err(TemporalError::UnexpectedFiniteBuchiCounterexample);
        }
        Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        }) => Some(TemporalCounterexample::Infinite {
            obligation: TemporalObligation::InfinitelyOftenAction(acceptance),
            stem: strip_buchi_trace(stem),
            cycle: strip_buchi_trace(cycle),
        }),
    };

    Ok(TemporalResult {
        property: result.automaton,
        backend: TemporalBackend::Buchi,
        status: match result.status {
            BuchiStatus::Satisfied => TemporalStatus::Satisfied,
            BuchiStatus::Violated => TemporalStatus::Violated,
        },
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        counterexample,
    })
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

use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, TraceStep};
use crate::declarative::DeclarativeDocument;
use crate::property::{
    check_reachability, check_reachability_with_limits, ReachabilityError, ReachabilityProperty,
    ReachabilityStatus,
};
use crate::proposition_expr::{
    PropositionExpression, PropositionExpressionError, ResolvedPropositionExpression,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionSafetySpec {
    name: String,
    expression: PropositionExpression,
}

impl PropositionSafetySpec {
    pub fn always(
        name: impl Into<String>,
        expression: PropositionExpression,
    ) -> Result<Self, SafetyError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SafetyError::EmptyPropertyName);
        }
        Ok(Self { name, expression })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn expression(&self) -> &PropositionExpression {
        &self.expression
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyStatus {
    Safe,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyResult {
    pub property: String,
    pub expression: String,
    pub status: SafetyStatus,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub counterexample: Option<Vec<TraceStep<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedSafetyResult {
    pub property: String,
    pub expression: String,
    pub outcome: BoundedOutcome<SafetyStatus>,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub counterexample: Option<Vec<TraceStep<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyError {
    EmptyPropertyName,
    PropositionExpression(PropositionExpressionError),
    Reachability(ReachabilityError),
}

impl fmt::Display for SafetyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "safety assertion name must not be empty"),
            Self::PropositionExpression(error) => {
                write!(f, "safety proposition expression failed: {error}")
            }
            Self::Reachability(error) => write!(f, "reachability backend failed: {error}"),
        }
    }
}

impl std::error::Error for SafetyError {}

impl From<PropositionExpressionError> for SafetyError {
    fn from(value: PropositionExpressionError) -> Self {
        Self::PropositionExpression(value)
    }
}

impl From<ReachabilityError> for SafetyError {
    fn from(value: ReachabilityError) -> Self {
        Self::Reachability(value)
    }
}

/// Verify that every reachable state satisfies a Boolean proposition expression.
///
/// This is intentionally a query-time safety property rather than a model
/// invariant mutation. The complete proposition expression is resolved before
/// exploration; verification then reuses canonical existential reachability on
/// the predicate complement. A reachable complement state is therefore the
/// deterministic shortest safety counterexample.
pub fn check_safety_assertion(
    document: &DeclarativeDocument,
    spec: &PropositionSafetySpec,
) -> Result<SafetyResult, SafetyError> {
    let resolved = ResolvedPropositionExpression::resolve(document, &spec.expression)?;
    let property = ReachabilityProperty::new(spec.name.clone(), move |state: &String| {
        !resolved.evaluate(state)
    })?;
    let result = check_reachability(document.model(), &property)?;

    Ok(SafetyResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        status: match result.status {
            ReachabilityStatus::Reachable => SafetyStatus::Violated,
            ReachabilityStatus::Unreachable => SafetyStatus::Safe,
        },
        discovered_states: result.discovered_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        counterexample: result.witness,
    })
}

/// Verify a declarative Boolean safety assertion under deterministic resource
/// limits. A falsifying state found before the cutoff remains a conclusive
/// shortest counterexample; proving safety requires exhaustive completion.
pub fn check_safety_assertion_with_limits(
    document: &DeclarativeDocument,
    spec: &PropositionSafetySpec,
    limits: ExplorationLimits,
) -> Result<BoundedSafetyResult, SafetyError> {
    let resolved = ResolvedPropositionExpression::resolve(document, &spec.expression)?;
    let property = ReachabilityProperty::new(spec.name.clone(), move |state: &String| {
        !resolved.evaluate(state)
    })?;
    let result = check_reachability_with_limits(document.model(), &property, limits)?;
    let outcome = match result.outcome {
        BoundedOutcome::Conclusive(ReachabilityStatus::Reachable) => {
            BoundedOutcome::Conclusive(SafetyStatus::Violated)
        }
        BoundedOutcome::Conclusive(ReachabilityStatus::Unreachable) => {
            BoundedOutcome::Conclusive(SafetyStatus::Safe)
        }
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };

    Ok(BoundedSafetyResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        outcome,
        discovered_states: result.discovered_states,
        checked_states: result.checked_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        counterexample: result.witness,
    })
}

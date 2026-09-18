use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, TraceStep};
use crate::declarative::DeclarativeDocument;
use crate::property::{
    check_deadlock_with_limits, BoundedDeadlockResult, DeadlockError, DeadlockProperty,
    DeadlockStatus,
};
use crate::proposition_expr::{
    parse_proposition_expression, PropositionExpression, PropositionExpressionError,
    PropositionExpressionParseError, ResolvedPropositionExpression,
};
use std::fmt;

/// Declarative legitimate-terminal policy over the M22 Boolean proposition
/// language. The expression is true exactly for terminal states that are
/// allowed to end a maximal execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeDeadlockSpec {
    name: String,
    expression: PropositionExpression,
}

impl DeclarativeDeadlockSpec {
    pub fn new(
        name: impl Into<String>,
        expression: PropositionExpression,
    ) -> Result<Self, DeclarativeDeadlockError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DeclarativeDeadlockError::EmptyPropertyName);
        }
        Ok(Self { name, expression })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn expression(&self) -> &PropositionExpression {
        &self.expression
    }

    pub fn canonical_expression(&self) -> String {
        self.expression.canonical_expression()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedDeclarativeDeadlockResult {
    pub property: String,
    pub expression: String,
    pub outcome: BoundedOutcome<DeadlockStatus>,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub witness: Option<Vec<TraceStep<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarativeDeadlockError {
    EmptyPropertyName,
    Parse(PropositionExpressionParseError),
    PropositionExpression(PropositionExpressionError),
    Deadlock(DeadlockError),
}

impl fmt::Display for DeclarativeDeadlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "deadlock policy name must not be empty"),
            Self::Parse(error) => write!(f, "deadlock policy parse failed: {error}"),
            Self::PropositionExpression(error) => {
                write!(f, "deadlock proposition expression failed: {error}")
            }
            Self::Deadlock(error) => write!(f, "deadlock backend failed: {error}"),
        }
    }
}

impl std::error::Error for DeclarativeDeadlockError {}

impl From<PropositionExpressionParseError> for DeclarativeDeadlockError {
    fn from(value: PropositionExpressionParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<PropositionExpressionError> for DeclarativeDeadlockError {
    fn from(value: PropositionExpressionError) -> Self {
        Self::PropositionExpression(value)
    }
}

impl From<DeadlockError> for DeclarativeDeadlockError {
    fn from(value: DeadlockError) -> Self {
        Self::Deadlock(value)
    }
}

/// Parse exactly the existing M22 Boolean proposition-expression grammar into
/// a typed legitimate-terminal policy. No deadlock-specific Boolean grammar or
/// evaluator is introduced.
pub fn parse_declarative_deadlock_spec(
    name: impl Into<String>,
    input: &str,
) -> Result<DeclarativeDeadlockSpec, DeclarativeDeadlockError> {
    DeclarativeDeadlockSpec::new(name, parse_proposition_expression(input)?)
}

/// Verify a declarative legitimate-terminal policy under deterministic
/// model-space limits.
///
/// Every proposition reference is resolved before backend execution. The
/// resulting owned predicate is passed to the canonical bounded deadlock
/// backend, preserving shortest unexpected-terminal witnesses and honest
/// incompleteness without reimplementing graph traversal or Boolean semantics.
pub fn check_declarative_deadlock_with_limits(
    document: &DeclarativeDocument,
    spec: &DeclarativeDeadlockSpec,
    limits: ExplorationLimits,
) -> Result<BoundedDeclarativeDeadlockResult, DeclarativeDeadlockError> {
    let resolved = ResolvedPropositionExpression::resolve(document, &spec.expression)?;
    let property = DeadlockProperty::new(spec.name.clone(), move |state: &String| {
        resolved.evaluate(state)
    })?;
    let result = check_deadlock_with_limits(document.model(), &property, limits)?;
    Ok(normalize_result(spec, result))
}

fn normalize_result(
    spec: &DeclarativeDeadlockSpec,
    result: BoundedDeadlockResult<String>,
) -> BoundedDeclarativeDeadlockResult {
    BoundedDeclarativeDeadlockResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        outcome: result.outcome,
        discovered_states: result.discovered_states,
        checked_states: result.checked_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        witness: result.witness,
    }
}

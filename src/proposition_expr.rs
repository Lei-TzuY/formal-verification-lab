use crate::bounded::BoundedOutcome;
use crate::checker::ExplorationLimits;
use crate::declarative::DeclarativeDocument;
use crate::eventuality::{
    check_eventuality, check_eventuality_with_limits, EventualityCounterexample, EventualityError,
    EventualityProperty, EventualityStatus,
};
use crate::exact_state::{ExactStateBackend, ExactStateEvidence, ExactStateStatus};
use crate::property::{
    check_reachability, check_reachability_with_limits, ReachabilityError, ReachabilityProperty,
    ReachabilityStatus,
};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropositionExpression {
    Atom(String),
    Not(Box<PropositionExpression>),
    And(Box<PropositionExpression>, Box<PropositionExpression>),
    Or(Box<PropositionExpression>, Box<PropositionExpression>),
}

impl PropositionExpression {
    pub fn atom(proposition: impl Into<String>) -> Result<Self, PropositionExpressionError> {
        let proposition = proposition.into();
        if proposition.trim().is_empty() {
            return Err(PropositionExpressionError::EmptyProposition);
        }
        Ok(Self::Atom(proposition))
    }

    pub fn negate(expression: Self) -> Self {
        Self::Not(Box::new(expression))
    }

    pub fn and(left: Self, right: Self) -> Self {
        Self::And(Box::new(left), Box::new(right))
    }

    pub fn or(left: Self, right: Self) -> Self {
        Self::Or(Box::new(left), Box::new(right))
    }

    pub fn canonical_expression(&self) -> String {
        match self {
            Self::Atom(proposition) => quote_string(proposition),
            Self::Not(inner) => format!("not ({})", inner.canonical_expression()),
            Self::And(left, right) => format!(
                "({} and {})",
                left.canonical_expression(),
                right.canonical_expression()
            ),
            Self::Or(left, right) => format!(
                "({} or {})",
                left.canonical_expression(),
                right.canonical_expression()
            ),
        }
    }

    pub fn evaluate(
        &self,
        document: &DeclarativeDocument,
        state: &str,
    ) -> Result<bool, PropositionExpressionError> {
        validate_expression(document, self)?;
        Ok(evaluate_document(self, document, state))
    }

    fn collect_references<'a>(&'a self, references: &mut Vec<&'a str>) {
        match self {
            Self::Atom(proposition) => references.push(proposition),
            Self::Not(inner) => inner.collect_references(references),
            Self::And(left, right) | Self::Or(left, right) => {
                left.collect_references(references);
                right.collect_references(references);
            }
        }
    }

    fn evaluate_resolved(&self, members: &HashMap<String, HashSet<String>>, state: &str) -> bool {
        match self {
            Self::Atom(proposition) => members
                .get(proposition)
                .expect("all proposition references are resolved before backend execution")
                .contains(state),
            Self::Not(inner) => !inner.evaluate_resolved(members, state),
            Self::And(left, right) => {
                left.evaluate_resolved(members, state) && right.evaluate_resolved(members, state)
            }
            Self::Or(left, right) => {
                left.evaluate_resolved(members, state) || right.evaluate_resolved(members, state)
            }
        }
    }
}

fn evaluate_document(
    expression: &PropositionExpression,
    document: &DeclarativeDocument,
    state: &str,
) -> bool {
    match expression {
        PropositionExpression::Atom(proposition) => {
            document.state_has_proposition(state, proposition)
        }
        PropositionExpression::Not(inner) => !evaluate_document(inner, document, state),
        PropositionExpression::And(left, right) => {
            evaluate_document(left, document, state) && evaluate_document(right, document, state)
        }
        PropositionExpression::Or(left, right) => {
            evaluate_document(left, document, state) || evaluate_document(right, document, state)
        }
    }
}

fn validate_expression(
    document: &DeclarativeDocument,
    expression: &PropositionExpression,
) -> Result<(), PropositionExpressionError> {
    let mut references = Vec::new();
    expression.collect_references(&mut references);
    let mut seen = HashSet::new();
    for proposition in references {
        if !seen.insert(proposition) {
            continue;
        }
        if document.proposition_states(proposition).is_none() {
            return Err(PropositionExpressionError::UnknownProposition {
                proposition: proposition.to_owned(),
            });
        }
    }
    Ok(())
}

fn resolve_expression(
    document: &DeclarativeDocument,
    expression: &PropositionExpression,
) -> Result<HashMap<String, HashSet<String>>, PropositionExpressionError> {
    let mut references = Vec::new();
    expression.collect_references(&mut references);
    let mut seen = HashSet::new();
    let mut members = HashMap::new();
    for proposition in references {
        if !seen.insert(proposition) {
            continue;
        }
        let states = document.proposition_states(proposition).ok_or_else(|| {
            PropositionExpressionError::UnknownProposition {
                proposition: proposition.to_owned(),
            }
        })?;
        members.insert(proposition.to_owned(), states.iter().cloned().collect());
    }
    Ok(members)
}

fn quote_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropositionExpressionPropertyKind {
    Reachable,
    AllEventually,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionExpressionPropertySpec {
    name: String,
    expression: PropositionExpression,
    kind: PropositionExpressionPropertyKind,
}

impl PropositionExpressionPropertySpec {
    pub fn reachable(
        name: impl Into<String>,
        expression: PropositionExpression,
    ) -> Result<Self, PropositionExpressionError> {
        Self::new(
            name,
            expression,
            PropositionExpressionPropertyKind::Reachable,
        )
    }

    pub fn all_eventually(
        name: impl Into<String>,
        expression: PropositionExpression,
    ) -> Result<Self, PropositionExpressionError> {
        Self::new(
            name,
            expression,
            PropositionExpressionPropertyKind::AllEventually,
        )
    }

    fn new(
        name: impl Into<String>,
        expression: PropositionExpression,
        kind: PropositionExpressionPropertyKind,
    ) -> Result<Self, PropositionExpressionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(PropositionExpressionError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            expression,
            kind,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn expression(&self) -> &PropositionExpression {
        &self.expression
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionExpressionResult {
    pub property: String,
    pub expression: String,
    pub backend: ExactStateBackend,
    pub status: ExactStateStatus,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub evidence: Option<ExactStateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedPropositionExpressionResult {
    pub property: String,
    pub expression: String,
    pub backend: ExactStateBackend,
    pub outcome: BoundedOutcome<ExactStateStatus>,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub evidence: Option<ExactStateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropositionExpressionError {
    EmptyPropertyName,
    EmptyProposition,
    UnknownProposition { proposition: String },
    Reachability(ReachabilityError),
    Eventuality(EventualityError),
}

impl fmt::Display for PropositionExpressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => {
                write!(f, "Boolean proposition property name must not be empty")
            }
            Self::EmptyProposition => write!(f, "proposition name must not be empty"),
            Self::UnknownProposition { proposition } => {
                write!(f, "unknown proposition '{proposition}'")
            }
            Self::Reachability(error) => write!(f, "reachability backend failed: {error}"),
            Self::Eventuality(error) => write!(f, "eventuality backend failed: {error}"),
        }
    }
}

impl std::error::Error for PropositionExpressionError {}

impl From<ReachabilityError> for PropositionExpressionError {
    fn from(value: ReachabilityError) -> Self {
        Self::Reachability(value)
    }
}

impl From<EventualityError> for PropositionExpressionError {
    fn from(value: EventualityError) -> Self {
        Self::Eventuality(value)
    }
}

pub fn check_proposition_expression_property(
    document: &DeclarativeDocument,
    spec: &PropositionExpressionPropertySpec,
) -> Result<PropositionExpressionResult, PropositionExpressionError> {
    let members = resolve_expression(document, &spec.expression)?;
    match spec.kind {
        PropositionExpressionPropertyKind::Reachable => check_reachable(document, spec, members),
        PropositionExpressionPropertyKind::AllEventually => {
            check_all_eventually(document, spec, members)
        }
    }
}

pub fn check_proposition_expression_property_with_limits(
    document: &DeclarativeDocument,
    spec: &PropositionExpressionPropertySpec,
    limits: ExplorationLimits,
) -> Result<BoundedPropositionExpressionResult, PropositionExpressionError> {
    let members = resolve_expression(document, &spec.expression)?;
    match spec.kind {
        PropositionExpressionPropertyKind::Reachable => {
            check_reachable_with_limits(document, spec, members, limits)
        }
        PropositionExpressionPropertyKind::AllEventually => {
            check_all_eventually_with_limits(document, spec, members, limits)
        }
    }
}

fn check_reachable(
    document: &DeclarativeDocument,
    spec: &PropositionExpressionPropertySpec,
    members: HashMap<String, HashSet<String>>,
) -> Result<PropositionExpressionResult, PropositionExpressionError> {
    let expression = spec.expression.clone();
    let property = ReachabilityProperty::new(spec.name.clone(), move |state: &String| {
        expression.evaluate_resolved(&members, state)
    })?;
    let result = check_reachability(document.model(), &property)?;
    let evidence = result
        .witness
        .map(|trace| ExactStateEvidence::ReachabilityWitness { trace });

    Ok(PropositionExpressionResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        backend: ExactStateBackend::Reachability,
        status: match result.status {
            ReachabilityStatus::Reachable => ExactStateStatus::Satisfied,
            ReachabilityStatus::Unreachable => ExactStateStatus::Violated,
        },
        discovered_states: result.discovered_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        evidence,
    })
}

fn check_reachable_with_limits(
    document: &DeclarativeDocument,
    spec: &PropositionExpressionPropertySpec,
    members: HashMap<String, HashSet<String>>,
    limits: ExplorationLimits,
) -> Result<BoundedPropositionExpressionResult, PropositionExpressionError> {
    let expression = spec.expression.clone();
    let property = ReachabilityProperty::new(spec.name.clone(), move |state: &String| {
        expression.evaluate_resolved(&members, state)
    })?;
    let result = check_reachability_with_limits(document.model(), &property, limits)?;
    let evidence = result
        .witness
        .map(|trace| ExactStateEvidence::ReachabilityWitness { trace });
    let outcome = match result.outcome {
        BoundedOutcome::Conclusive(ReachabilityStatus::Reachable) => {
            BoundedOutcome::Conclusive(ExactStateStatus::Satisfied)
        }
        BoundedOutcome::Conclusive(ReachabilityStatus::Unreachable) => {
            BoundedOutcome::Conclusive(ExactStateStatus::Violated)
        }
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };

    Ok(BoundedPropositionExpressionResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        backend: ExactStateBackend::Reachability,
        outcome,
        discovered_states: result.discovered_states,
        checked_states: result.checked_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        evidence,
    })
}

fn check_all_eventually(
    document: &DeclarativeDocument,
    spec: &PropositionExpressionPropertySpec,
    members: HashMap<String, HashSet<String>>,
) -> Result<PropositionExpressionResult, PropositionExpressionError> {
    let expression = spec.expression.clone();
    let property = EventualityProperty::new(spec.name.clone(), move |state: &String| {
        expression.evaluate_resolved(&members, state)
    })?;
    let result = check_eventuality(document.model(), &property)?;
    let evidence = normalize_eventuality_evidence(result.counterexample);

    Ok(PropositionExpressionResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        backend: ExactStateBackend::Eventuality,
        status: match result.status {
            EventualityStatus::Satisfied => ExactStateStatus::Satisfied,
            EventualityStatus::Violated => ExactStateStatus::Violated,
        },
        discovered_states: result.discovered_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        evidence,
    })
}

fn check_all_eventually_with_limits(
    document: &DeclarativeDocument,
    spec: &PropositionExpressionPropertySpec,
    members: HashMap<String, HashSet<String>>,
    limits: ExplorationLimits,
) -> Result<BoundedPropositionExpressionResult, PropositionExpressionError> {
    let expression = spec.expression.clone();
    let property = EventualityProperty::new(spec.name.clone(), move |state: &String| {
        expression.evaluate_resolved(&members, state)
    })?;
    let result = check_eventuality_with_limits(document.model(), &property, limits)?;
    let evidence = normalize_eventuality_evidence(result.counterexample);
    let outcome = match result.outcome {
        BoundedOutcome::Conclusive(EventualityStatus::Satisfied) => {
            BoundedOutcome::Conclusive(ExactStateStatus::Satisfied)
        }
        BoundedOutcome::Conclusive(EventualityStatus::Violated) => {
            BoundedOutcome::Conclusive(ExactStateStatus::Violated)
        }
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };

    Ok(BoundedPropositionExpressionResult {
        property: result.property,
        expression: spec.expression.canonical_expression(),
        backend: ExactStateBackend::Eventuality,
        outcome,
        discovered_states: result.discovered_states,
        checked_states: result.checked_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        evidence,
    })
}

fn normalize_eventuality_evidence(
    counterexample: Option<EventualityCounterexample<String>>,
) -> Option<ExactStateEvidence> {
    match counterexample {
        None => None,
        Some(EventualityCounterexample::Finite { trace }) => {
            Some(ExactStateEvidence::EventualityFiniteCounterexample { trace })
        }
        Some(EventualityCounterexample::Infinite { stem, cycle }) => {
            Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropositionExpressionParseErrorKind {
    ExpectedExpression,
    ExpectedCloseParen,
    UnterminatedString,
    InvalidEscape { escape: String },
    TrailingInput,
    Semantic(PropositionExpressionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionExpressionParseError {
    position: usize,
    kind: PropositionExpressionParseErrorKind,
}

impl PropositionExpressionParseError {
    fn new(position: usize, kind: PropositionExpressionParseErrorKind) -> Self {
        Self { position, kind }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn kind(&self) -> &PropositionExpressionParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for PropositionExpressionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "proposition expression parse error at byte {}: ",
            self.position
        )?;
        match &self.kind {
            PropositionExpressionParseErrorKind::ExpectedExpression => {
                write!(f, "expected a quoted proposition, 'not', or '('")
            }
            PropositionExpressionParseErrorKind::ExpectedCloseParen => write!(f, "expected ')'"),
            PropositionExpressionParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted proposition")
            }
            PropositionExpressionParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            PropositionExpressionParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after proposition expression")
            }
            PropositionExpressionParseErrorKind::Semantic(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for PropositionExpressionParseError {}

pub fn parse_proposition_expression(
    input: &str,
) -> Result<PropositionExpression, PropositionExpressionParseError> {
    let mut parser = PropositionExpressionParser::new(input);
    parser.skip_whitespace();
    let expression = parser.parse_or()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(PropositionExpressionParseError::new(
            parser.position,
            PropositionExpressionParseErrorKind::TrailingInput,
        ));
    }
    Ok(expression)
}

struct PropositionExpressionParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> PropositionExpressionParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn is_eof(&self) -> bool {
        self.position == self.input.len()
    }

    fn current_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            self.position += 1;
        }
    }

    fn parse_or(&mut self) -> Result<PropositionExpression, PropositionExpressionParseError> {
        let mut expression = self.parse_and()?;
        loop {
            self.skip_whitespace();
            if !self.consume_keyword("or") {
                break;
            }
            let right = self.parse_and()?;
            expression = PropositionExpression::or(expression, right);
        }
        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<PropositionExpression, PropositionExpressionParseError> {
        let mut expression = self.parse_unary()?;
        loop {
            self.skip_whitespace();
            if !self.consume_keyword("and") {
                break;
            }
            let right = self.parse_unary()?;
            expression = PropositionExpression::and(expression, right);
        }
        Ok(expression)
    }

    fn parse_unary(&mut self) -> Result<PropositionExpression, PropositionExpressionParseError> {
        self.skip_whitespace();
        if self.consume_keyword("not") {
            return Ok(PropositionExpression::negate(self.parse_unary()?));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<PropositionExpression, PropositionExpressionParseError> {
        self.skip_whitespace();
        let start = self.position;
        match self.current_byte() {
            Some(b'"') => {
                let proposition = self.parse_string()?;
                PropositionExpression::atom(proposition).map_err(|error| {
                    PropositionExpressionParseError::new(
                        start,
                        PropositionExpressionParseErrorKind::Semantic(error),
                    )
                })
            }
            Some(b'(') => {
                self.position += 1;
                let expression = self.parse_or()?;
                self.skip_whitespace();
                if self.current_byte() != Some(b')') {
                    return Err(PropositionExpressionParseError::new(
                        self.position,
                        PropositionExpressionParseErrorKind::ExpectedCloseParen,
                    ));
                }
                self.position += 1;
                Ok(expression)
            }
            _ => Err(PropositionExpressionParseError::new(
                start,
                PropositionExpressionParseErrorKind::ExpectedExpression,
            )),
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        let remaining = &self.input[self.position..];
        if !remaining.starts_with(keyword) {
            return false;
        }
        let end = self.position + keyword.len();
        let boundary_ok = self
            .input
            .as_bytes()
            .get(end)
            .is_none_or(|byte| !is_identifier_byte(*byte));
        if boundary_ok {
            self.position = end;
            true
        } else {
            false
        }
    }

    fn parse_string(&mut self) -> Result<String, PropositionExpressionParseError> {
        let start = self.position;
        self.position += 1;
        let mut output = String::new();

        while !self.is_eof() {
            let ch = self.input[self.position..]
                .chars()
                .next()
                .expect("non-empty UTF-8 suffix has a character");
            self.position += ch.len_utf8();
            match ch {
                '"' => return Ok(output),
                '\\' => {
                    if self.is_eof() {
                        return Err(PropositionExpressionParseError::new(
                            start,
                            PropositionExpressionParseErrorKind::UnterminatedString,
                        ));
                    }
                    let escape_position = self.position - 1;
                    let escape = self.input[self.position..]
                        .chars()
                        .next()
                        .expect("non-empty UTF-8 suffix has an escape character");
                    self.position += escape.len_utf8();
                    match escape {
                        '\\' => output.push('\\'),
                        '"' => output.push('"'),
                        'n' => output.push('\n'),
                        'r' => output.push('\r'),
                        't' => output.push('\t'),
                        _ => {
                            return Err(PropositionExpressionParseError::new(
                                escape_position,
                                PropositionExpressionParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }

        Err(PropositionExpressionParseError::new(
            start,
            PropositionExpressionParseErrorKind::UnterminatedString,
        ))
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

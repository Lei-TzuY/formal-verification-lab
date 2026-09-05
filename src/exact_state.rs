use crate::checker::TraceStep;
use crate::eventuality::{
    check_eventuality, EventualityCounterexample, EventualityError, EventualityProperty,
    EventualityStatus,
};
use crate::model::TransitionSystem;
use crate::property::{
    check_reachability, ReachabilityError, ReachabilityProperty, ReachabilityStatus,
};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExactStatePropertyKind {
    Reachable,
    AllEventually,
}

/// A deliberately narrow exact-state property over `TransitionSystem<String>`.
///
/// This frontend does not introduce graph-search semantics. `reachable` routes
/// to the existing existential reachability backend and `all-eventually` routes
/// to the existing universal eventuality backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactStatePropertySpec {
    name: String,
    target: String,
    kind: ExactStatePropertyKind,
}

impl ExactStatePropertySpec {
    pub fn reachable(
        name: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, ExactStateError> {
        Self::new(name, target, ExactStatePropertyKind::Reachable)
    }

    pub fn all_eventually(
        name: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<Self, ExactStateError> {
        Self::new(name, target, ExactStatePropertyKind::AllEventually)
    }

    fn new(
        name: impl Into<String>,
        target: impl Into<String>,
        kind: ExactStatePropertyKind,
    ) -> Result<Self, ExactStateError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ExactStateError::EmptyPropertyName);
        }
        let target = target.into();
        if target.trim().is_empty() {
            return Err(ExactStateError::EmptyTargetState);
        }
        Ok(Self { name, target, kind })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    /// Render one deterministic textual expression accepted by
    /// `parse_exact_state_property`.
    pub fn canonical_expression(&self) -> String {
        let operator = match self.kind {
            ExactStatePropertyKind::Reachable => "reachable",
            ExactStatePropertyKind::AllEventually => "all-eventually",
        };
        format!("{operator}({})", quote_string(&self.target))
    }
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
pub enum ExactStateBackend {
    Reachability,
    Eventuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactStateStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactStateEvidence {
    ReachabilityWitness {
        trace: Vec<TraceStep<String>>,
    },
    EventualityFiniteCounterexample {
        trace: Vec<TraceStep<String>>,
    },
    EventualityInfiniteCounterexample {
        stem: Vec<TraceStep<String>>,
        cycle: Vec<TraceStep<String>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactStateResult {
    pub property: String,
    pub target: String,
    pub backend: ExactStateBackend,
    pub status: ExactStateStatus,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub evidence: Option<ExactStateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactStateError {
    EmptyPropertyName,
    EmptyTargetState,
    Reachability(ReachabilityError),
    Eventuality(EventualityError),
}

impl fmt::Display for ExactStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "exact-state property name must not be empty"),
            Self::EmptyTargetState => write!(f, "exact-state target must not be empty"),
            Self::Reachability(error) => write!(f, "reachability backend failed: {error}"),
            Self::Eventuality(error) => write!(f, "eventuality backend failed: {error}"),
        }
    }
}

impl std::error::Error for ExactStateError {}

impl From<ReachabilityError> for ExactStateError {
    fn from(value: ReachabilityError) -> Self {
        Self::Reachability(value)
    }
}

impl From<EventualityError> for ExactStateError {
    fn from(value: EventualityError) -> Self {
        Self::Eventuality(value)
    }
}

/// Verify one exact-state property by delegating to an existing validated
/// backend. An undeclared target string is still a well-formed mathematical
/// query: existential reachability exhausts to false, while universal
/// eventuality produces the backend's normal finite/lasso counterexample.
pub fn check_exact_state_property(
    model: &TransitionSystem<String>,
    spec: &ExactStatePropertySpec,
) -> Result<ExactStateResult, ExactStateError> {
    match spec.kind {
        ExactStatePropertyKind::Reachable => check_reachable(model, spec),
        ExactStatePropertyKind::AllEventually => check_all_eventually(model, spec),
    }
}

fn check_reachable(
    model: &TransitionSystem<String>,
    spec: &ExactStatePropertySpec,
) -> Result<ExactStateResult, ExactStateError> {
    let target = spec.target.clone();
    let property =
        ReachabilityProperty::new(spec.name.clone(), move |state: &String| state == &target)?;
    let result = check_reachability(model, &property)?;
    let evidence = result
        .witness
        .map(|trace| ExactStateEvidence::ReachabilityWitness { trace });

    Ok(ExactStateResult {
        property: result.property,
        target: spec.target.clone(),
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

fn check_all_eventually(
    model: &TransitionSystem<String>,
    spec: &ExactStatePropertySpec,
) -> Result<ExactStateResult, ExactStateError> {
    let target = spec.target.clone();
    let property =
        EventualityProperty::new(spec.name.clone(), move |state: &String| state == &target)?;
    let result = check_eventuality(model, &property)?;
    let evidence = match result.counterexample {
        None => None,
        Some(EventualityCounterexample::Finite { trace }) => {
            Some(ExactStateEvidence::EventualityFiniteCounterexample { trace })
        }
        Some(EventualityCounterexample::Infinite { stem, cycle }) => {
            Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle })
        }
    };

    Ok(ExactStateResult {
        property: result.property,
        target: spec.target.clone(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactStateParseErrorKind {
    ExpectedOperator,
    UnknownOperator {
        operator: String,
    },
    ExpectedOpenParen,
    ExpectedString,
    UnterminatedString,
    InvalidEscape {
        escape: String,
    },
    ExpectedCommaOrClose,
    WrongArity {
        operator: String,
        expected: usize,
        actual: usize,
    },
    TrailingInput,
    Semantic(ExactStateError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactStateParseError {
    position: usize,
    kind: ExactStateParseErrorKind,
}

impl ExactStateParseError {
    fn new(position: usize, kind: ExactStateParseErrorKind) -> Self {
        Self { position, kind }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn kind(&self) -> &ExactStateParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for ExactStateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exact-state parse error at byte {}: ", self.position)?;
        match &self.kind {
            ExactStateParseErrorKind::ExpectedOperator => {
                write!(f, "expected 'reachable' or 'all-eventually'")
            }
            ExactStateParseErrorKind::UnknownOperator { operator } => {
                write!(f, "unsupported exact-state operator '{operator}'")
            }
            ExactStateParseErrorKind::ExpectedOpenParen => write!(f, "expected '('") ,
            ExactStateParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted exact state id")
            }
            ExactStateParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted state id")
            }
            ExactStateParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            ExactStateParseErrorKind::ExpectedCommaOrClose => write!(f, "expected ',' or ')'"),
            ExactStateParseErrorKind::WrongArity {
                operator,
                expected,
                actual,
            } => write!(
                f,
                "operator '{operator}' expects {expected} arguments but received {actual}"
            ),
            ExactStateParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after exact-state expression")
            }
            ExactStateParseErrorKind::Semantic(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ExactStateParseError {}

/// Parse exactly the two state-property forms implemented by this frontend:
/// `reachable("state")` and `all-eventually("state")`.
pub fn parse_exact_state_property(
    name: impl Into<String>,
    input: &str,
) -> Result<ExactStatePropertySpec, ExactStateParseError> {
    let mut parser = ExactStateParser::new(input);
    parser.skip_whitespace();
    let operator_start = parser.position;
    let operator = parser.parse_operator()?;
    if operator != "reachable" && operator != "all-eventually" {
        return Err(ExactStateParseError::new(
            operator_start,
            ExactStateParseErrorKind::UnknownOperator { operator },
        ));
    }

    parser.skip_whitespace();
    parser.expect_byte(b'(', ExactStateParseErrorKind::ExpectedOpenParen)?;
    let (arguments, close_position) = parser.parse_arguments()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(ExactStateParseError::new(
            parser.position,
            ExactStateParseErrorKind::TrailingInput,
        ));
    }
    if arguments.len() != 1 {
        return Err(ExactStateParseError::new(
            close_position,
            ExactStateParseErrorKind::WrongArity {
                operator,
                expected: 1,
                actual: arguments.len(),
            },
        ));
    }

    let name = name.into();
    let target = arguments.into_iter().next().expect("arity was checked").0;
    let result = match operator.as_str() {
        "reachable" => ExactStatePropertySpec::reachable(name, target),
        "all-eventually" => ExactStatePropertySpec::all_eventually(name, target),
        _ => unreachable!("operator was validated before dispatch"),
    };
    result.map_err(|error| {
        ExactStateParseError::new(operator_start, ExactStateParseErrorKind::Semantic(error))
    })
}

struct ExactStateParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> ExactStateParser<'a> {
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

    fn parse_operator(&mut self) -> Result<String, ExactStateParseError> {
        let start = self.position;
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(ExactStateParseError::new(
                start,
                ExactStateParseErrorKind::ExpectedOperator,
            ));
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn expect_byte(
        &mut self,
        expected: u8,
        kind: ExactStateParseErrorKind,
    ) -> Result<(), ExactStateParseError> {
        if self.current_byte() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(ExactStateParseError::new(self.position, kind))
        }
    }

    fn parse_arguments(&mut self) -> Result<(Vec<(String, usize)>, usize), ExactStateParseError> {
        let mut arguments = Vec::new();
        self.skip_whitespace();
        if self.current_byte() == Some(b')') {
            let close = self.position;
            self.position += 1;
            return Ok((arguments, close));
        }
        loop {
            self.skip_whitespace();
            arguments.push(self.parse_string()?);
            self.skip_whitespace();
            match self.current_byte() {
                Some(b',') => self.position += 1,
                Some(b')') => {
                    let close = self.position;
                    self.position += 1;
                    return Ok((arguments, close));
                }
                _ => {
                    return Err(ExactStateParseError::new(
                        self.position,
                        ExactStateParseErrorKind::ExpectedCommaOrClose,
                    ));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<(String, usize), ExactStateParseError> {
        let start = self.position;
        if self.current_byte() != Some(b'"') {
            return Err(ExactStateParseError::new(
                start,
                ExactStateParseErrorKind::ExpectedString,
            ));
        }
        self.position += 1;
        let mut output = String::new();

        while !self.is_eof() {
            let ch = self.input[self.position..]
                .chars()
                .next()
                .expect("non-empty UTF-8 suffix has a character");
            self.position += ch.len_utf8();
            match ch {
                '"' => return Ok((output, start)),
                '\\' => {
                    if self.is_eof() {
                        return Err(ExactStateParseError::new(
                            start,
                            ExactStateParseErrorKind::UnterminatedString,
                        ));
                    }
                    let escape = self.input[self.position..]
                        .chars()
                        .next()
                        .expect("non-empty UTF-8 suffix has an escape character");
                    let escape_position = self.position - 1;
                    self.position += escape.len_utf8();
                    match escape {
                        '\\' => output.push('\\'),
                        '"' => output.push('"'),
                        'n' => output.push('\n'),
                        'r' => output.push('\r'),
                        't' => output.push('\t'),
                        _ => {
                            return Err(ExactStateParseError::new(
                                escape_position,
                                ExactStateParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }

        Err(ExactStateParseError::new(
            start,
            ExactStateParseErrorKind::UnterminatedString,
        ))
    }
}

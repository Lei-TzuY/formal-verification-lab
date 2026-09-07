use crate::bounded::AnalysisLimits;
use crate::checker::ExplorationLimits;
use crate::combined_fairness::FairnessProfile;
use crate::model::TransitionSystem;
use crate::multi_response::{
    check_multi_response, check_multi_response_with_fairness_profile,
    check_multi_response_with_fairness_profile_and_limits,
    check_multi_response_with_fairness_profile_and_product_limits,
    check_multi_response_with_limits, check_multi_response_with_product_limits,
    AnalysisMultiResponseResult, BoundedMultiResponseResult, MultiResponseError,
    MultiResponseProperty, MultiResponseResult, ResponseClause,
};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseActionRole {
    Trigger,
    Response,
}

impl fmt::Display for ResponseActionRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trigger => write!(f, "trigger"),
            Self::Response => write!(f, "response"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiResponseTemporalSpecError {
    EmptyPropertyName,
    NoClauses,
    EmptyClauseName,
    EmptyActionName {
        clause: String,
        role: ResponseActionRole,
    },
    DuplicateClauseName {
        name: String,
    },
}

impl fmt::Display for MultiResponseTemporalSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => {
                write!(f, "multi-response temporal property name must not be empty")
            }
            Self::NoClauses => write!(
                f,
                "multi-response temporal property requires at least one response clause"
            ),
            Self::EmptyClauseName => write!(f, "response clause name must not be empty"),
            Self::EmptyActionName { clause, role } => {
                write!(f, "response clause '{clause}' has an empty {role} action")
            }
            Self::DuplicateClauseName { name } => {
                write!(f, "duplicate response clause name '{name}'")
            }
        }
    }
}

impl std::error::Error for MultiResponseTemporalSpecError {}

/// One named exact-action response clause used by the textual multi-response
/// frontend. The backend still owns response semantics; this type only captures
/// validated external metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactResponseClause {
    name: String,
    trigger: String,
    response: String,
}

impl ExactResponseClause {
    pub fn new(
        name: impl Into<String>,
        trigger: impl Into<String>,
        response: impl Into<String>,
    ) -> Result<Self, MultiResponseTemporalSpecError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MultiResponseTemporalSpecError::EmptyClauseName);
        }
        let trigger = trigger.into();
        if trigger.trim().is_empty() {
            return Err(MultiResponseTemporalSpecError::EmptyActionName {
                clause: name,
                role: ResponseActionRole::Trigger,
            });
        }
        let response = response.into();
        if response.trim().is_empty() {
            return Err(MultiResponseTemporalSpecError::EmptyActionName {
                clause: name,
                role: ResponseActionRole::Response,
            });
        }
        Ok(Self {
            name,
            trigger,
            response,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn trigger(&self) -> &str {
        &self.trigger
    }

    pub fn response(&self) -> &str {
        &self.response
    }
}

/// A conjunction of named exact-action response clauses supplied through an
/// external textual frontend.
///
/// This is intentionally not another temporal-logic engine. Compilation creates
/// the canonical `MultiResponseProperty`, so no-fair, combined-fair, bounded,
/// and staged verification all reuse the already audited response backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiResponseTemporalSpec {
    name: String,
    clauses: Vec<ExactResponseClause>,
}

impl MultiResponseTemporalSpec {
    pub fn new(
        name: impl Into<String>,
        clauses: Vec<ExactResponseClause>,
    ) -> Result<Self, MultiResponseTemporalSpecError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MultiResponseTemporalSpecError::EmptyPropertyName);
        }
        if clauses.is_empty() {
            return Err(MultiResponseTemporalSpecError::NoClauses);
        }
        let mut names = HashSet::new();
        for clause in &clauses {
            if !names.insert(clause.name.clone()) {
                return Err(MultiResponseTemporalSpecError::DuplicateClauseName {
                    name: clause.name.clone(),
                });
            }
        }
        Ok(Self { name, clauses })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn clauses(&self) -> &[ExactResponseClause] {
        &self.clauses
    }

    /// Render the deterministic line-oriented syntax accepted by
    /// `parse_multi_response_temporal`.
    pub fn canonical_document(&self) -> String {
        self.clauses
            .iter()
            .map(|clause| {
                format!(
                    "response({},{},{})",
                    quote(clause.name()),
                    quote(clause.trigger()),
                    quote(clause.response())
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn to_property(&self) -> Result<MultiResponseProperty, MultiResponseError> {
        let clauses = self
            .clauses
            .iter()
            .map(|clause| {
                let trigger = clause.trigger.clone();
                let response = clause.response.clone();
                ResponseClause::new(
                    clause.name.clone(),
                    move |action| action == trigger,
                    move |action| action == response,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        MultiResponseProperty::new(self.name.clone(), clauses)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiResponseTemporalParseErrorKind {
    ExpectedDirective,
    UnknownDirective { directive: String },
    ExpectedOpenParen,
    ExpectedString,
    UnterminatedString,
    InvalidEscape { escape: String },
    ExpectedCommaOrClose,
    WrongArity { expected: usize, actual: usize },
    TrailingInput,
    Semantic(MultiResponseTemporalSpecError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiResponseTemporalParseError {
    line: usize,
    column: usize,
    kind: MultiResponseTemporalParseErrorKind,
}

impl MultiResponseTemporalParseError {
    fn new(line: usize, column: usize, kind: MultiResponseTemporalParseErrorKind) -> Self {
        Self { line, column, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn kind(&self) -> &MultiResponseTemporalParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for MultiResponseTemporalParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "multi-response temporal parse error at line {}, byte column {}: ",
            self.line, self.column
        )?;
        match &self.kind {
            MultiResponseTemporalParseErrorKind::ExpectedDirective => {
                write!(f, "expected 'response' directive")
            }
            MultiResponseTemporalParseErrorKind::UnknownDirective { directive } => {
                write!(f, "unsupported directive '{directive}'")
            }
            MultiResponseTemporalParseErrorKind::ExpectedOpenParen => {
                write!(f, "expected '('")
            }
            MultiResponseTemporalParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted string")
            }
            MultiResponseTemporalParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted string")
            }
            MultiResponseTemporalParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            MultiResponseTemporalParseErrorKind::ExpectedCommaOrClose => {
                write!(f, "expected ',' or ')'")
            }
            MultiResponseTemporalParseErrorKind::WrongArity { expected, actual } => write!(
                f,
                "response directive expects {expected} arguments but received {actual}"
            ),
            MultiResponseTemporalParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after response directive")
            }
            MultiResponseTemporalParseErrorKind::Semantic(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for MultiResponseTemporalParseError {}

/// Parse a conjunction of named response clauses from a line-oriented document.
///
/// Each non-empty, non-comment line has exactly this form:
///
/// ```text
/// response("clause-name","trigger-action","response-action")
/// ```
///
/// Blank lines and full-line `#` comments are ignored. Strings support `\\`,
/// `\"`, `\n`, `\r`, and `\t`; columns are one-based UTF-8 byte columns.
pub fn parse_multi_response_temporal(
    name: impl Into<String>,
    input: &str,
) -> Result<MultiResponseTemporalSpec, MultiResponseTemporalParseError> {
    let name = name.into();
    let mut clauses = Vec::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = raw_line.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let leading = raw_line.len() - trimmed.len();
        let clause = parse_clause_line(line_number, leading, trimmed)?;
        clauses.push(clause);
    }

    MultiResponseTemporalSpec::new(name, clauses).map_err(|error| {
        MultiResponseTemporalParseError::new(
            1,
            1,
            MultiResponseTemporalParseErrorKind::Semantic(error),
        )
    })
}

fn parse_clause_line(
    line: usize,
    leading: usize,
    input: &str,
) -> Result<ExactResponseClause, MultiResponseTemporalParseError> {
    let mut parser = LineParser::new(input);
    parser.skip_whitespace();
    let directive_start = parser.position;
    let directive = parser.parse_directive().map_err(|kind| {
        MultiResponseTemporalParseError::new(line, leading + directive_start + 1, kind)
    })?;
    if directive != "response" {
        return Err(MultiResponseTemporalParseError::new(
            line,
            leading + directive_start + 1,
            MultiResponseTemporalParseErrorKind::UnknownDirective { directive },
        ));
    }

    parser.skip_whitespace();
    parser
        .expect_byte(b'(', MultiResponseTemporalParseErrorKind::ExpectedOpenParen)
        .map_err(|(position, kind)| {
            MultiResponseTemporalParseError::new(line, leading + position + 1, kind)
        })?;
    let (arguments, close_position) = parser.parse_arguments().map_err(|(position, kind)| {
        MultiResponseTemporalParseError::new(line, leading + position + 1, kind)
    })?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(MultiResponseTemporalParseError::new(
            line,
            leading + parser.position + 1,
            MultiResponseTemporalParseErrorKind::TrailingInput,
        ));
    }
    if arguments.len() != 3 {
        return Err(MultiResponseTemporalParseError::new(
            line,
            leading + close_position + 1,
            MultiResponseTemporalParseErrorKind::WrongArity {
                expected: 3,
                actual: arguments.len(),
            },
        ));
    }

    ExactResponseClause::new(
        arguments[0].0.clone(),
        arguments[1].0.clone(),
        arguments[2].0.clone(),
    )
    .map_err(|error| {
        let column = match &error {
            MultiResponseTemporalSpecError::EmptyClauseName => arguments[0].1,
            MultiResponseTemporalSpecError::EmptyActionName {
                role: ResponseActionRole::Trigger,
                ..
            } => arguments[1].1,
            MultiResponseTemporalSpecError::EmptyActionName {
                role: ResponseActionRole::Response,
                ..
            } => arguments[2].1,
            _ => directive_start,
        };
        MultiResponseTemporalParseError::new(
            line,
            leading + column + 1,
            MultiResponseTemporalParseErrorKind::Semantic(error),
        )
    })
}

struct LineParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> LineParser<'a> {
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

    fn parse_directive(&mut self) -> Result<String, MultiResponseTemporalParseErrorKind> {
        let start = self.position;
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(MultiResponseTemporalParseErrorKind::ExpectedDirective);
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn expect_byte(
        &mut self,
        expected: u8,
        kind: MultiResponseTemporalParseErrorKind,
    ) -> Result<(), (usize, MultiResponseTemporalParseErrorKind)> {
        if self.current_byte() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err((self.position, kind))
        }
    }

    fn parse_arguments(
        &mut self,
    ) -> Result<(Vec<(String, usize)>, usize), (usize, MultiResponseTemporalParseErrorKind)> {
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
                Some(b',') => {
                    self.position += 1;
                }
                Some(b')') => {
                    let close = self.position;
                    self.position += 1;
                    return Ok((arguments, close));
                }
                _ => {
                    return Err((
                        self.position,
                        MultiResponseTemporalParseErrorKind::ExpectedCommaOrClose,
                    ));
                }
            }
        }
    }

    fn parse_string(
        &mut self,
    ) -> Result<(String, usize), (usize, MultiResponseTemporalParseErrorKind)> {
        let start = self.position;
        if self.current_byte() != Some(b'"') {
            return Err((start, MultiResponseTemporalParseErrorKind::ExpectedString));
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
                        return Err((
                            start,
                            MultiResponseTemporalParseErrorKind::UnterminatedString,
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
                            return Err((
                                escape_position,
                                MultiResponseTemporalParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }

        Err((
            start,
            MultiResponseTemporalParseErrorKind::UnterminatedString,
        ))
    }
}

fn quote(value: &str) -> String {
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

pub fn check_multi_response_temporal<S>(
    model: &TransitionSystem<S>,
    spec: &MultiResponseTemporalSpec,
) -> Result<MultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let property = spec.to_property()?;
    check_multi_response(model, &property)
}

pub fn check_multi_response_temporal_with_fairness_profile<S>(
    model: &TransitionSystem<S>,
    spec: &MultiResponseTemporalSpec,
    profile: &FairnessProfile,
) -> Result<MultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let property = spec.to_property()?;
    check_multi_response_with_fairness_profile(model, &property, profile)
}

pub fn check_multi_response_temporal_with_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &MultiResponseTemporalSpec,
    limits: ExplorationLimits,
) -> Result<BoundedMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let property = spec.to_property()?;
    check_multi_response_with_product_limits(model, &property, limits)
}

pub fn check_multi_response_temporal_with_fairness_profile_and_product_limits<S>(
    model: &TransitionSystem<S>,
    spec: &MultiResponseTemporalSpec,
    profile: &FairnessProfile,
    limits: ExplorationLimits,
) -> Result<BoundedMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let property = spec.to_property()?;
    check_multi_response_with_fairness_profile_and_product_limits(model, &property, profile, limits)
}

pub fn check_multi_response_temporal_with_limits<S>(
    model: &TransitionSystem<S>,
    spec: &MultiResponseTemporalSpec,
    limits: AnalysisLimits,
) -> Result<AnalysisMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let property = spec.to_property()?;
    check_multi_response_with_limits(model, &property, limits)
}

pub fn check_multi_response_temporal_with_fairness_profile_and_limits<S>(
    model: &TransitionSystem<S>,
    spec: &MultiResponseTemporalSpec,
    profile: &FairnessProfile,
    limits: AnalysisLimits,
) -> Result<AnalysisMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let property = spec.to_property()?;
    check_multi_response_with_fairness_profile_and_limits(model, &property, profile, limits)
}

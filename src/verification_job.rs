use crate::checker::ExplorationLimits;
use std::collections::HashSet;
use std::fmt;

const MODEL: &str = "model";
const PROPERTY: &str = "property";
const WEAK_FAIR_ACTION: &str = "weak-fair-action";
const STRONG_FAIR_ACTION: &str = "strong-fair-action";
const MAX_MODEL_STATES: &str = "max-model-states";
const MAX_MODEL_TRANSITIONS: &str = "max-model-transitions";
const MAX_MODEL_DEPTH: &str = "max-model-depth";
const MAX_PRODUCT_STATES: &str = "max-product-states";
const MAX_PRODUCT_TRANSITIONS: &str = "max-product-transitions";
const MAX_PRODUCT_DEPTH: &str = "max-product-depth";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJob {
    model_path: String,
    property_path: String,
    weak_fair_actions: Vec<String>,
    strong_fair_actions: Vec<String>,
    model_limits: ExplorationLimits,
    product_limits: ExplorationLimits,
}

impl VerificationJob {
    pub fn model_path(&self) -> &str {
        &self.model_path
    }

    pub fn property_path(&self) -> &str {
        &self.property_path
    }

    pub fn weak_fair_actions(&self) -> &[String] {
        &self.weak_fair_actions
    }

    pub fn strong_fair_actions(&self) -> &[String] {
        &self.strong_fair_actions
    }

    pub fn model_limits(&self) -> ExplorationLimits {
        self.model_limits
    }

    pub fn product_limits(&self) -> ExplorationLimits {
        self.product_limits
    }

    /// Compile the manifest's execution assumptions and resource budgets to the
    /// existing M54 CLI option surface. Semantic validation of fairness sets and
    /// the execution behavior of the limits remain owned by that canonical path.
    pub fn option_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        for action in &self.weak_fair_actions {
            args.push("--weak-fair-action".to_owned());
            args.push(action.clone());
        }
        for action in &self.strong_fair_actions {
            args.push("--strong-fair-action".to_owned());
            args.push(action.clone());
        }
        push_limit(
            &mut args,
            "--max-model-states",
            self.model_limits.max_states,
        );
        push_limit(
            &mut args,
            "--max-model-transitions",
            self.model_limits.max_transitions,
        );
        push_limit(
            &mut args,
            "--max-model-depth",
            self.model_limits.max_depth,
        );
        push_limit(
            &mut args,
            "--max-product-states",
            self.product_limits.max_states,
        );
        push_limit(
            &mut args,
            "--max-product-transitions",
            self.product_limits.max_transitions,
        );
        push_limit(
            &mut args,
            "--max-product-depth",
            self.product_limits.max_depth,
        );
        args
    }

    pub fn canonical_document(&self) -> String {
        let mut lines = vec![
            format!("model {}", quote(&self.model_path)),
            format!("property {}", quote(&self.property_path)),
        ];
        lines.extend(
            self.weak_fair_actions
                .iter()
                .map(|action| format!("weak-fair-action {}", quote(action))),
        );
        lines.extend(
            self.strong_fair_actions
                .iter()
                .map(|action| format!("strong-fair-action {}", quote(action))),
        );
        push_limit_line(&mut lines, MAX_MODEL_STATES, self.model_limits.max_states);
        push_limit_line(
            &mut lines,
            MAX_MODEL_TRANSITIONS,
            self.model_limits.max_transitions,
        );
        push_limit_line(&mut lines, MAX_MODEL_DEPTH, self.model_limits.max_depth);
        push_limit_line(
            &mut lines,
            MAX_PRODUCT_STATES,
            self.product_limits.max_states,
        );
        push_limit_line(
            &mut lines,
            MAX_PRODUCT_TRANSITIONS,
            self.product_limits.max_transitions,
        );
        push_limit_line(
            &mut lines,
            MAX_PRODUCT_DEPTH,
            self.product_limits.max_depth,
        );
        lines.join("\n")
    }
}

fn push_limit(args: &mut Vec<String>, flag: &str, value: Option<usize>) {
    if let Some(value) = value {
        args.push(flag.to_owned());
        args.push(value.to_string());
    }
}

fn push_limit_line(lines: &mut Vec<String>, directive: &str, value: Option<usize>) {
    if let Some(value) = value {
        lines.push(format!("{directive} {value}"));
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationJobParseErrorKind {
    ExpectedDirective,
    UnknownDirective { directive: String },
    ExpectedString,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyPath { directive: String },
    ExpectedNumber,
    InvalidNumber { value: String },
    TrailingInput,
    DuplicateDirective { directive: String },
    MissingDirective { directive: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobParseError {
    line: usize,
    column: usize,
    kind: VerificationJobParseErrorKind,
}

impl VerificationJobParseError {
    fn new(line: usize, column: usize, kind: VerificationJobParseErrorKind) -> Self {
        Self { line, column, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn kind(&self) -> &VerificationJobParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for VerificationJobParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "verification job parse error at line {}, byte column {}: ",
            self.line, self.column
        )?;
        match &self.kind {
            VerificationJobParseErrorKind::ExpectedDirective => write!(f, "expected a directive"),
            VerificationJobParseErrorKind::UnknownDirective { directive } => {
                write!(f, "unsupported directive '{directive}'")
            }
            VerificationJobParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted string")
            }
            VerificationJobParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted string")
            }
            VerificationJobParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            VerificationJobParseErrorKind::EmptyPath { directive } => {
                write!(f, "'{directive}' path must not be empty")
            }
            VerificationJobParseErrorKind::ExpectedNumber => {
                write!(f, "expected a non-negative decimal integer")
            }
            VerificationJobParseErrorKind::InvalidNumber { value } => {
                write!(f, "invalid non-negative decimal integer '{value}'")
            }
            VerificationJobParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after directive value")
            }
            VerificationJobParseErrorKind::DuplicateDirective { directive } => {
                write!(f, "duplicate singleton directive '{directive}'")
            }
            VerificationJobParseErrorKind::MissingDirective { directive } => {
                write!(f, "missing required directive '{directive}'")
            }
        }
    }
}

impl std::error::Error for VerificationJobParseError {}

pub fn parse_verification_job(input: &str) -> Result<VerificationJob, VerificationJobParseError> {
    let mut model_path = None;
    let mut property_path = None;
    let mut weak_fair_actions = Vec::new();
    let mut strong_fair_actions = Vec::new();
    let mut model_limits = ExplorationLimits::unbounded();
    let mut product_limits = ExplorationLimits::unbounded();
    let mut seen_singletons = HashSet::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = raw_line.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let leading = raw_line.len() - trimmed.len();
        let mut parser = LineParser::new(trimmed);
        let directive_start = parser.position;
        let directive = parser.parse_directive().map_err(|kind| {
            VerificationJobParseError::new(line_number, leading + directive_start + 1, kind)
        })?;
        parser.skip_whitespace();

        match directive.as_str() {
            MODEL | PROPERTY => {
                require_singleton(
                    &mut seen_singletons,
                    &directive,
                    line_number,
                    leading + directive_start + 1,
                )?;
                let value = parser.parse_string().map_err(|(position, kind)| {
                    VerificationJobParseError::new(line_number, leading + position + 1, kind)
                })?;
                if value.is_empty() {
                    return Err(VerificationJobParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        VerificationJobParseErrorKind::EmptyPath {
                            directive: directive.clone(),
                        },
                    ));
                }
                finish_line(&mut parser, line_number, leading)?;
                if directive == MODEL {
                    model_path = Some(value);
                } else {
                    property_path = Some(value);
                }
            }
            WEAK_FAIR_ACTION | STRONG_FAIR_ACTION => {
                let value = parser.parse_string().map_err(|(position, kind)| {
                    VerificationJobParseError::new(line_number, leading + position + 1, kind)
                })?;
                finish_line(&mut parser, line_number, leading)?;
                if directive == WEAK_FAIR_ACTION {
                    weak_fair_actions.push(value);
                } else {
                    strong_fair_actions.push(value);
                }
            }
            MAX_MODEL_STATES | MAX_MODEL_TRANSITIONS | MAX_MODEL_DEPTH | MAX_PRODUCT_STATES
            | MAX_PRODUCT_TRANSITIONS | MAX_PRODUCT_DEPTH => {
                require_singleton(
                    &mut seen_singletons,
                    &directive,
                    line_number,
                    leading + directive_start + 1,
                )?;
                let value = parser.parse_number().map_err(|(position, kind)| {
                    VerificationJobParseError::new(line_number, leading + position + 1, kind)
                })?;
                finish_line(&mut parser, line_number, leading)?;
                match directive.as_str() {
                    MAX_MODEL_STATES => model_limits.max_states = Some(value),
                    MAX_MODEL_TRANSITIONS => model_limits.max_transitions = Some(value),
                    MAX_MODEL_DEPTH => model_limits.max_depth = Some(value),
                    MAX_PRODUCT_STATES => product_limits.max_states = Some(value),
                    MAX_PRODUCT_TRANSITIONS => product_limits.max_transitions = Some(value),
                    MAX_PRODUCT_DEPTH => product_limits.max_depth = Some(value),
                    _ => unreachable!("matched budget directive"),
                }
            }
            _ => {
                return Err(VerificationJobParseError::new(
                    line_number,
                    leading + directive_start + 1,
                    VerificationJobParseErrorKind::UnknownDirective { directive },
                ));
            }
        }
    }

    let model_path = model_path.ok_or_else(|| {
        VerificationJobParseError::new(
            1,
            1,
            VerificationJobParseErrorKind::MissingDirective {
                directive: MODEL.to_owned(),
            },
        )
    })?;
    let property_path = property_path.ok_or_else(|| {
        VerificationJobParseError::new(
            1,
            1,
            VerificationJobParseErrorKind::MissingDirective {
                directive: PROPERTY.to_owned(),
            },
        )
    })?;

    Ok(VerificationJob {
        model_path,
        property_path,
        weak_fair_actions,
        strong_fair_actions,
        model_limits,
        product_limits,
    })
}

fn require_singleton(
    seen: &mut HashSet<String>,
    directive: &str,
    line: usize,
    column: usize,
) -> Result<(), VerificationJobParseError> {
    if seen.insert(directive.to_owned()) {
        Ok(())
    } else {
        Err(VerificationJobParseError::new(
            line,
            column,
            VerificationJobParseErrorKind::DuplicateDirective {
                directive: directive.to_owned(),
            },
        ))
    }
}

fn finish_line(
    parser: &mut LineParser<'_>,
    line: usize,
    leading: usize,
) -> Result<(), VerificationJobParseError> {
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(())
    } else {
        Err(VerificationJobParseError::new(
            line,
            leading + parser.position + 1,
            VerificationJobParseErrorKind::TrailingInput,
        ))
    }
}

type ParseError = (usize, VerificationJobParseErrorKind);

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

    fn parse_directive(&mut self) -> Result<String, VerificationJobParseErrorKind> {
        let start = self.position;
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(VerificationJobParseErrorKind::ExpectedDirective);
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        let start = self.position;
        if self.current_byte() != Some(b'"') {
            return Err((start, VerificationJobParseErrorKind::ExpectedString));
        }
        self.position += 1;
        let mut output = String::new();

        while !self.is_eof() {
            let ch = self.input[self.position..]
                .chars()
                .next()
                .expect("non-empty UTF-8 suffix has a character");
            if ch == '"' {
                self.position += 1;
                return Ok(output);
            }
            if ch == '\\' {
                self.position += 1;
                let escape_position = self.position;
                let Some(escape) = self.input[self.position..].chars().next() else {
                    return Err((
                        escape_position,
                        VerificationJobParseErrorKind::UnterminatedString,
                    ));
                };
                match escape {
                    '\\' => output.push('\\'),
                    '"' => output.push('"'),
                    'n' => output.push('\n'),
                    'r' => output.push('\r'),
                    't' => output.push('\t'),
                    _ => {
                        return Err((
                            escape_position,
                            VerificationJobParseErrorKind::InvalidEscape {
                                escape: escape.to_string(),
                            },
                        ));
                    }
                }
                self.position += escape.len_utf8();
            } else {
                output.push(ch);
                self.position += ch.len_utf8();
            }
        }

        Err((start, VerificationJobParseErrorKind::UnterminatedString))
    }

    fn parse_number(&mut self) -> Result<usize, ParseError> {
        let start = self.position;
        while self.current_byte().is_some_and(|byte| byte.is_ascii_digit()) {
            self.position += 1;
        }
        if self.position == start {
            return Err((start, VerificationJobParseErrorKind::ExpectedNumber));
        }
        let value = &self.input[start..self.position];
        value.parse::<usize>().map_err(|_| {
            (
                start,
                VerificationJobParseErrorKind::InvalidNumber {
                    value: value.to_owned(),
                },
            )
        })
    }
}

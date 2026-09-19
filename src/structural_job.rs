use crate::checker::ExplorationLimits;
use std::collections::HashSet;
use std::fmt;

const ANALYSIS: &str = "analysis";
const MODEL: &str = "model";
const MAX_STATES: &str = "max-states";
const MAX_TRANSITIONS: &str = "max-transitions";
const MAX_DEPTH: &str = "max-depth";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralJobAnalysis {
    Recurrence,
}

impl StructuralJobAnalysis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recurrence => "recurrence",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "recurrence" => Some(Self::Recurrence),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJob {
    analysis: StructuralJobAnalysis,
    model_path: String,
    model_limits: ExplorationLimits,
}

impl StructuralJob {
    pub fn analysis(&self) -> StructuralJobAnalysis {
        self.analysis
    }

    pub fn model_path(&self) -> &str {
        &self.model_path
    }

    pub fn model_limits(&self) -> ExplorationLimits {
        self.model_limits
    }

    pub fn canonical_document(&self) -> String {
        let mut lines = vec![
            format!("analysis {}", quote(self.analysis.as_str())),
            format!("model {}", quote(&self.model_path)),
        ];
        push_limit_line(&mut lines, MAX_STATES, self.model_limits.max_states);
        push_limit_line(
            &mut lines,
            MAX_TRANSITIONS,
            self.model_limits.max_transitions,
        );
        push_limit_line(&mut lines, MAX_DEPTH, self.model_limits.max_depth);
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralJobParseErrorKind {
    ExpectedDirective,
    UnknownDirective { directive: String },
    ExpectedString,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyModelPath,
    InvalidAnalysis { analysis: String },
    ExpectedNumber,
    InvalidNumber { value: String },
    TrailingInput,
    DuplicateDirective { directive: String },
    MissingDirective { directive: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobParseError {
    line: usize,
    column: usize,
    kind: StructuralJobParseErrorKind,
}

impl StructuralJobParseError {
    fn new(line: usize, column: usize, kind: StructuralJobParseErrorKind) -> Self {
        Self { line, column, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn kind(&self) -> &StructuralJobParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for StructuralJobParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "structural job parse error at line {}, byte column {}: ",
            self.line, self.column
        )?;
        match &self.kind {
            StructuralJobParseErrorKind::ExpectedDirective => write!(f, "expected a directive"),
            StructuralJobParseErrorKind::UnknownDirective { directive } => {
                write!(f, "unsupported directive '{directive}'")
            }
            StructuralJobParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted string")
            }
            StructuralJobParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted string")
            }
            StructuralJobParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            StructuralJobParseErrorKind::EmptyModelPath => {
                write!(f, "'model' path must not be empty")
            }
            StructuralJobParseErrorKind::InvalidAnalysis { analysis } => write!(
                f,
                "unsupported structural analysis '{analysis}'; expected recurrence"
            ),
            StructuralJobParseErrorKind::ExpectedNumber => {
                write!(f, "expected a non-negative decimal integer")
            }
            StructuralJobParseErrorKind::InvalidNumber { value } => {
                write!(f, "invalid non-negative decimal integer '{value}'")
            }
            StructuralJobParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after directive value")
            }
            StructuralJobParseErrorKind::DuplicateDirective { directive } => {
                write!(f, "duplicate singleton directive '{directive}'")
            }
            StructuralJobParseErrorKind::MissingDirective { directive } => {
                write!(f, "missing required directive '{directive}'")
            }
        }
    }
}

impl std::error::Error for StructuralJobParseError {}

pub fn parse_structural_job(input: &str) -> Result<StructuralJob, StructuralJobParseError> {
    let mut analysis = None;
    let mut model_path = None;
    let mut limits = ExplorationLimits::unbounded();
    let mut seen = HashSet::new();

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
            StructuralJobParseError::new(line_number, leading + directive_start + 1, kind)
        })?;
        require_singleton(
            &mut seen,
            &directive,
            line_number,
            leading + directive_start + 1,
        )?;
        parser.skip_whitespace();

        match directive.as_str() {
            ANALYSIS => {
                let value = parser.parse_string().map_err(|(position, kind)| {
                    StructuralJobParseError::new(line_number, leading + position + 1, kind)
                })?;
                finish_line(&mut parser, line_number, leading)?;
                analysis = Some(StructuralJobAnalysis::parse(&value).ok_or_else(|| {
                    StructuralJobParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        StructuralJobParseErrorKind::InvalidAnalysis { analysis: value },
                    )
                })?);
            }
            MODEL => {
                let value_start = parser.position;
                let value = parser.parse_string().map_err(|(position, kind)| {
                    StructuralJobParseError::new(line_number, leading + position + 1, kind)
                })?;
                finish_line(&mut parser, line_number, leading)?;
                if value.is_empty() {
                    return Err(StructuralJobParseError::new(
                        line_number,
                        leading + value_start + 1,
                        StructuralJobParseErrorKind::EmptyModelPath,
                    ));
                }
                model_path = Some(value);
            }
            MAX_STATES | MAX_TRANSITIONS | MAX_DEPTH => {
                let value = parser.parse_number().map_err(|(position, kind)| {
                    StructuralJobParseError::new(line_number, leading + position + 1, kind)
                })?;
                finish_line(&mut parser, line_number, leading)?;
                match directive.as_str() {
                    MAX_STATES => limits.max_states = Some(value),
                    MAX_TRANSITIONS => limits.max_transitions = Some(value),
                    MAX_DEPTH => limits.max_depth = Some(value),
                    _ => unreachable!(),
                }
            }
            _ => {
                return Err(StructuralJobParseError::new(
                    line_number,
                    leading + directive_start + 1,
                    StructuralJobParseErrorKind::UnknownDirective { directive },
                ));
            }
        }
    }

    Ok(StructuralJob {
        analysis: analysis.ok_or_else(|| {
            StructuralJobParseError::new(
                1,
                1,
                StructuralJobParseErrorKind::MissingDirective {
                    directive: ANALYSIS.to_owned(),
                },
            )
        })?,
        model_path: model_path.ok_or_else(|| {
            StructuralJobParseError::new(
                1,
                1,
                StructuralJobParseErrorKind::MissingDirective {
                    directive: MODEL.to_owned(),
                },
            )
        })?,
        model_limits: limits,
    })
}

fn require_singleton(
    seen: &mut HashSet<String>,
    directive: &str,
    line: usize,
    column: usize,
) -> Result<(), StructuralJobParseError> {
    if !seen.insert(directive.to_owned()) {
        return Err(StructuralJobParseError::new(
            line,
            column,
            StructuralJobParseErrorKind::DuplicateDirective {
                directive: directive.to_owned(),
            },
        ));
    }
    Ok(())
}

fn finish_line(
    parser: &mut LineParser<'_>,
    line: usize,
    leading: usize,
) -> Result<(), StructuralJobParseError> {
    parser.skip_whitespace();
    if parser.position != parser.input.len() {
        return Err(StructuralJobParseError::new(
            line,
            leading + parser.position + 1,
            StructuralJobParseErrorKind::TrailingInput,
        ));
    }
    Ok(())
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

struct LineParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> LineParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            self.position += ch.len_utf8();
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn parse_directive(&mut self) -> Result<String, StructuralJobParseErrorKind> {
        let start = self.position;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                break;
            }
            self.position += ch.len_utf8();
        }
        if self.position == start {
            return Err(StructuralJobParseErrorKind::ExpectedDirective);
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_string(
        &mut self,
    ) -> Result<String, (usize, StructuralJobParseErrorKind)> {
        let start = self.position;
        if self.peek() != Some('"') {
            return Err((start, StructuralJobParseErrorKind::ExpectedString));
        }
        self.position += 1;
        let mut output = String::new();
        loop {
            let Some(ch) = self.peek() else {
                return Err((start, StructuralJobParseErrorKind::UnterminatedString));
            };
            self.position += ch.len_utf8();
            match ch {
                '"' => return Ok(output),
                '\\' => {
                    let escape_position = self.position;
                    let Some(escape) = self.peek() else {
                        return Err((start, StructuralJobParseErrorKind::UnterminatedString));
                    };
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
                                StructuralJobParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }
    }

    fn parse_number(
        &mut self,
    ) -> Result<usize, (usize, StructuralJobParseErrorKind)> {
        let start = self.position;
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_digit() {
                break;
            }
            self.position += 1;
        }
        if self.position == start {
            return Err((start, StructuralJobParseErrorKind::ExpectedNumber));
        }
        let value = &self.input[start..self.position];
        value.parse::<usize>().map_err(|_| {
            (
                start,
                StructuralJobParseErrorKind::InvalidNumber {
                    value: value.to_owned(),
                },
            )
        })
    }
}

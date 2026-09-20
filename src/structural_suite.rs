use std::collections::HashSet;
use std::fmt;

const SUITE: &str = "suite";
const JOB: &str = "job";
const EXPECT: &str = "expect";

pub const MAX_STRUCTURAL_SUITE_JOBS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralExpectedOutcome {
    CycleFound,
    Acyclic,
    Inconclusive,
    Error,
}

impl StructuralExpectedOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CycleFound => "cycle_found",
            Self::Acyclic => "acyclic",
            Self::Inconclusive => "inconclusive",
            Self::Error => "error",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "cycle_found" => Some(Self::CycleFound),
            "acyclic" => Some(Self::Acyclic),
            "inconclusive" => Some(Self::Inconclusive),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralSuite {
    name: String,
    job_paths: Vec<String>,
    expected_outcomes: Vec<Option<StructuralExpectedOutcome>>,
}

impl StructuralSuite {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn job_paths(&self) -> &[String] {
        &self.job_paths
    }

    pub fn expected_outcomes(&self) -> &[Option<StructuralExpectedOutcome>] {
        &self.expected_outcomes
    }

    pub fn canonical_document(&self) -> String {
        let mut lines = vec![format!("suite {}", quote(&self.name))];
        lines.extend(
            self.job_paths
                .iter()
                .zip(&self.expected_outcomes)
                .map(|(path, expected)| match expected {
                    Some(expected) => format!(
                        "job {} expect {}",
                        quote(path),
                        quote(expected.as_str())
                    ),
                    None => format!("job {}", quote(path)),
                }),
        );
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralSuiteParseErrorKind {
    ExpectedDirective,
    UnknownDirective { directive: String },
    ExpectedString,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyName,
    EmptyPath,
    InvalidExpectedOutcome { outcome: String },
    TrailingInput,
    DuplicateSuiteDirective,
    DuplicateJob { path: String },
    MissingSuiteDirective,
    NoJobs,
    TooManyJobs { limit: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralSuiteParseError {
    line: usize,
    column: usize,
    kind: StructuralSuiteParseErrorKind,
}

impl StructuralSuiteParseError {
    fn new(line: usize, column: usize, kind: StructuralSuiteParseErrorKind) -> Self {
        Self { line, column, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn kind(&self) -> &StructuralSuiteParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for StructuralSuiteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "structural suite parse error at line {}, byte column {}: ",
            self.line, self.column
        )?;
        match &self.kind {
            StructuralSuiteParseErrorKind::ExpectedDirective => write!(f, "expected a directive"),
            StructuralSuiteParseErrorKind::UnknownDirective { directive } => {
                write!(f, "unsupported directive '{directive}'")
            }
            StructuralSuiteParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted string")
            }
            StructuralSuiteParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted string")
            }
            StructuralSuiteParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            StructuralSuiteParseErrorKind::EmptyName => {
                write!(f, "suite name must not be empty")
            }
            StructuralSuiteParseErrorKind::EmptyPath => {
                write!(f, "structural job path must not be empty")
            }
            StructuralSuiteParseErrorKind::InvalidExpectedOutcome { outcome } => write!(
                f,
                "unsupported expected structural outcome '{outcome}'; expected cycle_found, acyclic, inconclusive, or error"
            ),
            StructuralSuiteParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after directive value")
            }
            StructuralSuiteParseErrorKind::DuplicateSuiteDirective => {
                write!(f, "duplicate singleton directive 'suite'")
            }
            StructuralSuiteParseErrorKind::DuplicateJob { path } => {
                write!(f, "duplicate structural job path '{path}'")
            }
            StructuralSuiteParseErrorKind::MissingSuiteDirective => {
                write!(f, "missing required directive 'suite'")
            }
            StructuralSuiteParseErrorKind::NoJobs => {
                write!(f, "structural suite requires at least one job")
            }
            StructuralSuiteParseErrorKind::TooManyJobs { limit } => {
                write!(f, "structural suite exceeds maximum of {limit} jobs")
            }
        }
    }
}

impl std::error::Error for StructuralSuiteParseError {}

pub fn parse_structural_suite(
    input: &str,
) -> Result<StructuralSuite, StructuralSuiteParseError> {
    let mut name = None;
    let mut job_paths = Vec::new();
    let mut expected_outcomes = Vec::new();
    let mut seen_jobs = HashSet::new();

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
            StructuralSuiteParseError::new(line_number, leading + directive_start + 1, kind)
        })?;
        parser.skip_whitespace();
        let value = parser.parse_string().map_err(|(position, kind)| {
            StructuralSuiteParseError::new(line_number, leading + position + 1, kind)
        })?;

        match directive.as_str() {
            SUITE => {
                finish_line(&mut parser, line_number, leading)?;
                if name.is_some() {
                    return Err(StructuralSuiteParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        StructuralSuiteParseErrorKind::DuplicateSuiteDirective,
                    ));
                }
                if value.is_empty() {
                    return Err(StructuralSuiteParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        StructuralSuiteParseErrorKind::EmptyName,
                    ));
                }
                name = Some(value);
            }
            JOB => {
                if value.is_empty() {
                    return Err(StructuralSuiteParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        StructuralSuiteParseErrorKind::EmptyPath,
                    ));
                }

                let expected = parse_optional_expectation(&mut parser, line_number, leading)?;
                if !seen_jobs.insert(value.clone()) {
                    return Err(StructuralSuiteParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        StructuralSuiteParseErrorKind::DuplicateJob { path: value },
                    ));
                }
                if job_paths.len() >= MAX_STRUCTURAL_SUITE_JOBS {
                    return Err(StructuralSuiteParseError::new(
                        line_number,
                        leading + directive_start + 1,
                        StructuralSuiteParseErrorKind::TooManyJobs {
                            limit: MAX_STRUCTURAL_SUITE_JOBS,
                        },
                    ));
                }
                job_paths.push(value);
                expected_outcomes.push(expected);
            }
            _ => {
                return Err(StructuralSuiteParseError::new(
                    line_number,
                    leading + directive_start + 1,
                    StructuralSuiteParseErrorKind::UnknownDirective { directive },
                ));
            }
        }
    }

    let name = name.ok_or_else(|| {
        StructuralSuiteParseError::new(
            1,
            1,
            StructuralSuiteParseErrorKind::MissingSuiteDirective,
        )
    })?;
    if job_paths.is_empty() {
        return Err(StructuralSuiteParseError::new(
            1,
            1,
            StructuralSuiteParseErrorKind::NoJobs,
        ));
    }

    Ok(StructuralSuite {
        name,
        job_paths,
        expected_outcomes,
    })
}

fn parse_optional_expectation(
    parser: &mut LineParser<'_>,
    line: usize,
    leading: usize,
) -> Result<Option<StructuralExpectedOutcome>, StructuralSuiteParseError> {
    parser.skip_whitespace();
    if parser.is_eof() {
        return Ok(None);
    }

    let directive_start = parser.position;
    let directive = parser.parse_directive().map_err(|kind| {
        StructuralSuiteParseError::new(line, leading + directive_start + 1, kind)
    })?;
    if directive != EXPECT {
        return Err(StructuralSuiteParseError::new(
            line,
            leading + directive_start + 1,
            StructuralSuiteParseErrorKind::UnknownDirective { directive },
        ));
    }

    parser.skip_whitespace();
    let outcome_start = parser.position;
    let outcome = parser.parse_string().map_err(|(position, kind)| {
        StructuralSuiteParseError::new(line, leading + position + 1, kind)
    })?;
    let expected = StructuralExpectedOutcome::parse(&outcome).ok_or_else(|| {
        StructuralSuiteParseError::new(
            line,
            leading + outcome_start + 1,
            StructuralSuiteParseErrorKind::InvalidExpectedOutcome { outcome },
        )
    })?;
    finish_line(parser, line, leading)?;
    Ok(Some(expected))
}

fn finish_line(
    parser: &mut LineParser<'_>,
    line: usize,
    leading: usize,
) -> Result<(), StructuralSuiteParseError> {
    parser.skip_whitespace();
    if parser.is_eof() {
        Ok(())
    } else {
        Err(StructuralSuiteParseError::new(
            line,
            leading + parser.position + 1,
            StructuralSuiteParseErrorKind::TrailingInput,
        ))
    }
}

fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

type ParseError = (usize, StructuralSuiteParseErrorKind);

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

    fn parse_directive(&mut self) -> Result<String, StructuralSuiteParseErrorKind> {
        let start = self.position;
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(StructuralSuiteParseErrorKind::ExpectedDirective);
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        let start = self.position;
        if self.current_byte() != Some(b'"') {
            return Err((start, StructuralSuiteParseErrorKind::ExpectedString));
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
                        StructuralSuiteParseErrorKind::UnterminatedString,
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
                            StructuralSuiteParseErrorKind::InvalidEscape {
                                escape: escape.to_string(),
                            },
                        ));
                    }
                }
                self.position += escape.len_utf8();
                continue;
            }
            output.push(ch);
            self.position += ch.len_utf8();
        }

        Err((
            start,
            StructuralSuiteParseErrorKind::UnterminatedString,
        ))
    }
}

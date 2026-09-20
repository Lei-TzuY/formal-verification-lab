use crate::certificate_verification_result::CertificateVerificationJobOutcome;
use crate::structural_result::StructuralJobOutcome;
use crate::verification_result::VerificationJobOutcome;
use std::collections::HashSet;
use std::fmt;

const SUITE: &str = "suite";
const JOB: &str = "job";
const EXPECT: &str = "expect";

pub const MAX_ORCHESTRATION_SUITE_JOBS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrchestrationJobFamily {
    Verification,
    Structural,
    CertificateVerification,
}

impl OrchestrationJobFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verification => "verification",
            Self::Structural => "structural",
            Self::CertificateVerification => "certificate-verification",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "verification" => Some(Self::Verification),
            "structural" => Some(Self::Structural),
            "certificate-verification" => Some(Self::CertificateVerification),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestrationExpectedOutcome {
    Verification(VerificationJobOutcome),
    Structural(StructuralJobOutcome),
    CertificateVerification(CertificateVerificationJobOutcome),
}

impl OrchestrationExpectedOutcome {
    pub fn family(self) -> OrchestrationJobFamily {
        match self {
            Self::Verification(_) => OrchestrationJobFamily::Verification,
            Self::Structural(_) => OrchestrationJobFamily::Structural,
            Self::CertificateVerification(_) => OrchestrationJobFamily::CertificateVerification,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verification(VerificationJobOutcome::Satisfied) => "satisfied",
            Self::Verification(VerificationJobOutcome::Violated) => "violated",
            Self::Verification(VerificationJobOutcome::Inconclusive) => "inconclusive",
            Self::Verification(VerificationJobOutcome::Error) => "error",
            Self::Structural(StructuralJobOutcome::CycleFound) => "cycle_found",
            Self::Structural(StructuralJobOutcome::Acyclic) => "acyclic",
            Self::Structural(StructuralJobOutcome::Inconclusive) => "inconclusive",
            Self::Structural(StructuralJobOutcome::Error) => "error",
            Self::CertificateVerification(CertificateVerificationJobOutcome::Verified) => {
                "verified"
            }
            Self::CertificateVerification(CertificateVerificationJobOutcome::Rejected) => {
                "rejected"
            }
            Self::CertificateVerification(CertificateVerificationJobOutcome::Error) => "error",
        }
    }

    fn parse(
        family: OrchestrationJobFamily,
        value: &str,
    ) -> Option<OrchestrationExpectedOutcome> {
        match family {
            OrchestrationJobFamily::Verification => match value {
                "satisfied" => Some(Self::Verification(VerificationJobOutcome::Satisfied)),
                "violated" => Some(Self::Verification(VerificationJobOutcome::Violated)),
                "inconclusive" => Some(Self::Verification(VerificationJobOutcome::Inconclusive)),
                "error" => Some(Self::Verification(VerificationJobOutcome::Error)),
                _ => None,
            },
            OrchestrationJobFamily::Structural => match value {
                "cycle_found" => Some(Self::Structural(StructuralJobOutcome::CycleFound)),
                "acyclic" => Some(Self::Structural(StructuralJobOutcome::Acyclic)),
                "inconclusive" => Some(Self::Structural(StructuralJobOutcome::Inconclusive)),
                "error" => Some(Self::Structural(StructuralJobOutcome::Error)),
                _ => None,
            },
            OrchestrationJobFamily::CertificateVerification => match value {
                "verified" => Some(Self::CertificateVerification(
                    CertificateVerificationJobOutcome::Verified,
                )),
                "rejected" => Some(Self::CertificateVerification(
                    CertificateVerificationJobOutcome::Rejected,
                )),
                "error" => Some(Self::CertificateVerification(
                    CertificateVerificationJobOutcome::Error,
                )),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuiteEntry {
    family: OrchestrationJobFamily,
    job_path: String,
    expected: Option<OrchestrationExpectedOutcome>,
}

impl OrchestrationSuiteEntry {
    pub fn family(&self) -> OrchestrationJobFamily {
        self.family
    }

    pub fn job_path(&self) -> &str {
        &self.job_path
    }

    pub fn expected(&self) -> Option<OrchestrationExpectedOutcome> {
        self.expected
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuite {
    name: String,
    entries: Vec<OrchestrationSuiteEntry>,
}

impl OrchestrationSuite {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn entries(&self) -> &[OrchestrationSuiteEntry] {
        &self.entries
    }

    pub fn canonical_document(&self) -> String {
        let mut lines = vec![format!("suite {}", quote(&self.name))];
        for entry in &self.entries {
            let mut line = format!(
                "job {} {}",
                quote(entry.family.as_str()),
                quote(&entry.job_path)
            );
            if let Some(expected) = entry.expected {
                line.push_str(" expect ");
                line.push_str(&quote(expected.as_str()));
            }
            lines.push(line);
        }
        lines.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestrationSuiteParseErrorKind {
    ExpectedDirective,
    UnknownDirective { directive: String },
    ExpectedString,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyName,
    EmptyPath,
    InvalidFamily { family: String },
    InvalidExpectedOutcome {
        family: OrchestrationJobFamily,
        outcome: String,
    },
    ExpectedExpectKeyword { keyword: String },
    TrailingInput,
    DuplicateSuiteDirective,
    DuplicateJob {
        family: OrchestrationJobFamily,
        path: String,
    },
    MissingSuiteDirective,
    NoJobs,
    TooManyJobs { limit: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuiteParseError {
    line: usize,
    column: usize,
    kind: OrchestrationSuiteParseErrorKind,
}

impl OrchestrationSuiteParseError {
    fn new(line: usize, column: usize, kind: OrchestrationSuiteParseErrorKind) -> Self {
        Self { line, column, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn kind(&self) -> &OrchestrationSuiteParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for OrchestrationSuiteParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "orchestration suite parse error at line {}, byte column {}: ",
            self.line, self.column
        )?;
        match &self.kind {
            OrchestrationSuiteParseErrorKind::ExpectedDirective => {
                write!(f, "expected a directive")
            }
            OrchestrationSuiteParseErrorKind::UnknownDirective { directive } => {
                write!(f, "unsupported directive '{directive}'")
            }
            OrchestrationSuiteParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted string")
            }
            OrchestrationSuiteParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted string")
            }
            OrchestrationSuiteParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            OrchestrationSuiteParseErrorKind::EmptyName => {
                write!(f, "suite name must not be empty")
            }
            OrchestrationSuiteParseErrorKind::EmptyPath => {
                write!(f, "job path must not be empty")
            }
            OrchestrationSuiteParseErrorKind::InvalidFamily { family } => write!(
                f,
                "unsupported job family '{family}'; expected verification, structural, or certificate-verification"
            ),
            OrchestrationSuiteParseErrorKind::InvalidExpectedOutcome { family, outcome } => write!(
                f,
                "unsupported expected outcome '{outcome}' for job family '{}'",
                family.as_str()
            ),
            OrchestrationSuiteParseErrorKind::ExpectedExpectKeyword { keyword } => {
                write!(f, "expected 'expect' after job path, found '{keyword}'")
            }
            OrchestrationSuiteParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input")
            }
            OrchestrationSuiteParseErrorKind::DuplicateSuiteDirective => {
                write!(f, "duplicate singleton directive 'suite'")
            }
            OrchestrationSuiteParseErrorKind::DuplicateJob { family, path } => write!(
                f,
                "duplicate orchestration job '{}:{}'",
                family.as_str(),
                path
            ),
            OrchestrationSuiteParseErrorKind::MissingSuiteDirective => {
                write!(f, "missing required directive 'suite'")
            }
            OrchestrationSuiteParseErrorKind::NoJobs => {
                write!(f, "orchestration suite requires at least one job")
            }
            OrchestrationSuiteParseErrorKind::TooManyJobs { limit } => {
                write!(f, "orchestration suite exceeds maximum of {limit} jobs")
            }
        }
    }
}

impl std::error::Error for OrchestrationSuiteParseError {}

pub fn parse_orchestration_suite(
    input: &str,
) -> Result<OrchestrationSuite, OrchestrationSuiteParseError> {
    let mut name = None;
    let mut entries = Vec::new();
    let mut seen_jobs = HashSet::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = line_index + 1;
        let trimmed = raw_line.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let leading = raw_line.len() - trimmed.len();
        let mut parser = LineParser::new(trimmed);
        let directive_start = parser.position;
        let directive = parser.parse_word().map_err(|kind| {
            OrchestrationSuiteParseError::new(line, leading + directive_start + 1, kind)
        })?;
        parser.skip_whitespace();

        match directive.as_str() {
            SUITE => {
                if name.is_some() {
                    return Err(OrchestrationSuiteParseError::new(
                        line,
                        leading + directive_start + 1,
                        OrchestrationSuiteParseErrorKind::DuplicateSuiteDirective,
                    ));
                }
                let value = parser.parse_string().map_err(|(position, kind)| {
                    OrchestrationSuiteParseError::new(line, leading + position + 1, kind)
                })?;
                parser.finish(line, leading)?;
                if value.is_empty() {
                    return Err(OrchestrationSuiteParseError::new(
                        line,
                        leading + directive_start + 1,
                        OrchestrationSuiteParseErrorKind::EmptyName,
                    ));
                }
                name = Some(value);
            }
            JOB => {
                let family_start = parser.position;
                let family_value = parser.parse_string().map_err(|(position, kind)| {
                    OrchestrationSuiteParseError::new(line, leading + position + 1, kind)
                })?;
                let family = OrchestrationJobFamily::parse(&family_value).ok_or_else(|| {
                    OrchestrationSuiteParseError::new(
                        line,
                        leading + family_start + 1,
                        OrchestrationSuiteParseErrorKind::InvalidFamily {
                            family: family_value.clone(),
                        },
                    )
                })?;

                parser.skip_whitespace();
                let path_start = parser.position;
                let path = parser.parse_string().map_err(|(position, kind)| {
                    OrchestrationSuiteParseError::new(line, leading + position + 1, kind)
                })?;
                if path.is_empty() {
                    return Err(OrchestrationSuiteParseError::new(
                        line,
                        leading + path_start + 1,
                        OrchestrationSuiteParseErrorKind::EmptyPath,
                    ));
                }

                parser.skip_whitespace();
                let expected = if parser.is_eof() {
                    None
                } else {
                    let keyword_start = parser.position;
                    let keyword = parser.parse_word().map_err(|kind| {
                        OrchestrationSuiteParseError::new(
                            line,
                            leading + keyword_start + 1,
                            kind,
                        )
                    })?;
                    if keyword != EXPECT {
                        return Err(OrchestrationSuiteParseError::new(
                            line,
                            leading + keyword_start + 1,
                            OrchestrationSuiteParseErrorKind::ExpectedExpectKeyword { keyword },
                        ));
                    }
                    parser.skip_whitespace();
                    let outcome_start = parser.position;
                    let outcome = parser.parse_string().map_err(|(position, kind)| {
                        OrchestrationSuiteParseError::new(line, leading + position + 1, kind)
                    })?;
                    let expected = OrchestrationExpectedOutcome::parse(family, &outcome)
                        .ok_or_else(|| {
                            OrchestrationSuiteParseError::new(
                                line,
                                leading + outcome_start + 1,
                                OrchestrationSuiteParseErrorKind::InvalidExpectedOutcome {
                                    family,
                                    outcome,
                                },
                            )
                        })?;
                    parser.finish(line, leading)?;
                    Some(expected)
                };

                if !seen_jobs.insert((family, path.clone())) {
                    return Err(OrchestrationSuiteParseError::new(
                        line,
                        leading + directive_start + 1,
                        OrchestrationSuiteParseErrorKind::DuplicateJob { family, path },
                    ));
                }
                if entries.len() >= MAX_ORCHESTRATION_SUITE_JOBS {
                    return Err(OrchestrationSuiteParseError::new(
                        line,
                        leading + directive_start + 1,
                        OrchestrationSuiteParseErrorKind::TooManyJobs {
                            limit: MAX_ORCHESTRATION_SUITE_JOBS,
                        },
                    ));
                }
                entries.push(OrchestrationSuiteEntry {
                    family,
                    job_path: path,
                    expected,
                });
            }
            _ => {
                return Err(OrchestrationSuiteParseError::new(
                    line,
                    leading + directive_start + 1,
                    OrchestrationSuiteParseErrorKind::UnknownDirective { directive },
                ));
            }
        }
    }

    let name = name.ok_or_else(|| {
        OrchestrationSuiteParseError::new(
            1,
            1,
            OrchestrationSuiteParseErrorKind::MissingSuiteDirective,
        )
    })?;
    if entries.is_empty() {
        return Err(OrchestrationSuiteParseError::new(
            1,
            1,
            OrchestrationSuiteParseErrorKind::NoJobs,
        ));
    }

    Ok(OrchestrationSuite { name, entries })
}

fn quote(value: &str) -> String {
    let mut output = String::from(""");
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

type ParseError = (usize, OrchestrationSuiteParseErrorKind);

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

    fn peek(&self) -> Option<char> {
        self.input[self.position..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            self.position += ch.len_utf8();
        }
    }

    fn finish(
        &mut self,
        line: usize,
        leading: usize,
    ) -> Result<(), OrchestrationSuiteParseError> {
        self.skip_whitespace();
        if self.is_eof() {
            Ok(())
        } else {
            Err(OrchestrationSuiteParseError::new(
                line,
                leading + self.position + 1,
                OrchestrationSuiteParseErrorKind::TrailingInput,
            ))
        }
    }

    fn parse_word(&mut self) -> Result<String, OrchestrationSuiteParseErrorKind> {
        let start = self.position;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                break;
            }
            self.position += ch.len_utf8();
        }
        if self.position == start {
            return Err(OrchestrationSuiteParseErrorKind::ExpectedDirective);
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        let start = self.position;
        if self.peek() != Some('"') {
            return Err((start, OrchestrationSuiteParseErrorKind::ExpectedString));
        }
        self.position += 1;
        let mut output = String::new();

        loop {
            let Some(ch) = self.peek() else {
                return Err((
                    start,
                    OrchestrationSuiteParseErrorKind::UnterminatedString,
                ));
            };
            self.position += ch.len_utf8();
            match ch {
                '"' => return Ok(output),
                '\\' => {
                    let escape_position = self.position;
                    let Some(escape) = self.peek() else {
                        return Err((
                            start,
                            OrchestrationSuiteParseErrorKind::UnterminatedString,
                        ));
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
                                OrchestrationSuiteParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                other => output.push(other),
            }
        }
    }
}

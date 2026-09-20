use std::collections::HashSet;
use std::fmt;

const MODEL: &str = "model";
const PROPERTY: &str = "property";
const CERTIFICATE: &str = "certificate";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateVerificationJob {
    model_path: String,
    property_path: String,
    certificate_path: String,
}

impl CertificateVerificationJob {
    pub fn model_path(&self) -> &str {
        &self.model_path
    }

    pub fn property_path(&self) -> &str {
        &self.property_path
    }

    pub fn certificate_path(&self) -> &str {
        &self.certificate_path
    }

    pub fn canonical_document(&self) -> String {
        [
            format!("model {}", quote(&self.model_path)),
            format!("property {}", quote(&self.property_path)),
            format!("certificate {}", quote(&self.certificate_path)),
        ]
        .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CertificateVerificationJobParseErrorKind {
    ExpectedDirective,
    UnknownDirective { directive: String },
    ExpectedString,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyPath { directive: String },
    TrailingInput,
    DuplicateDirective { directive: String },
    MissingDirective { directive: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateVerificationJobParseError {
    line: usize,
    column: usize,
    kind: CertificateVerificationJobParseErrorKind,
}

impl CertificateVerificationJobParseError {
    fn new(line: usize, column: usize, kind: CertificateVerificationJobParseErrorKind) -> Self {
        Self { line, column, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn column(&self) -> usize {
        self.column
    }

    pub fn kind(&self) -> &CertificateVerificationJobParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for CertificateVerificationJobParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "certificate verification job parse error at line {}, byte column {}: ",
            self.line, self.column
        )?;
        match &self.kind {
            CertificateVerificationJobParseErrorKind::ExpectedDirective => {
                write!(f, "expected a directive")
            }
            CertificateVerificationJobParseErrorKind::UnknownDirective { directive } => {
                write!(f, "unsupported directive '{directive}'")
            }
            CertificateVerificationJobParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted string")
            }
            CertificateVerificationJobParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted string")
            }
            CertificateVerificationJobParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            CertificateVerificationJobParseErrorKind::EmptyPath { directive } => {
                write!(f, "'{directive}' path must not be empty")
            }
            CertificateVerificationJobParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after directive value")
            }
            CertificateVerificationJobParseErrorKind::DuplicateDirective { directive } => {
                write!(f, "duplicate singleton directive '{directive}'")
            }
            CertificateVerificationJobParseErrorKind::MissingDirective { directive } => {
                write!(f, "missing required directive '{directive}'")
            }
        }
    }
}

impl std::error::Error for CertificateVerificationJobParseError {}

pub fn parse_certificate_verification_job(
    input: &str,
) -> Result<CertificateVerificationJob, CertificateVerificationJobParseError> {
    let mut model_path = None;
    let mut property_path = None;
    let mut certificate_path = None;
    let mut seen = HashSet::new();

    for (line_index, raw_line) in input.lines().enumerate() {
        let line = line_index + 1;
        let trimmed = raw_line.trim_start_matches(|ch: char| ch.is_ascii_whitespace());
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let leading = raw_line.len() - trimmed.len();
        let mut parser = LineParser::new(trimmed);
        let directive_start = parser.position;
        let directive = parser.parse_directive().map_err(|kind| {
            CertificateVerificationJobParseError::new(line, leading + directive_start + 1, kind)
        })?;
        if !matches!(directive.as_str(), MODEL | PROPERTY | CERTIFICATE) {
            return Err(CertificateVerificationJobParseError::new(
                line,
                leading + directive_start + 1,
                CertificateVerificationJobParseErrorKind::UnknownDirective { directive },
            ));
        }
        if !seen.insert(directive.clone()) {
            return Err(CertificateVerificationJobParseError::new(
                line,
                leading + directive_start + 1,
                CertificateVerificationJobParseErrorKind::DuplicateDirective { directive },
            ));
        }

        parser.skip_whitespace();
        let value_start = parser.position;
        let value = parser.parse_string().map_err(|(position, kind)| {
            CertificateVerificationJobParseError::new(line, leading + position + 1, kind)
        })?;
        parser.skip_whitespace();
        if !parser.is_eof() {
            return Err(CertificateVerificationJobParseError::new(
                line,
                leading + parser.position + 1,
                CertificateVerificationJobParseErrorKind::TrailingInput,
            ));
        }
        if value.is_empty() {
            return Err(CertificateVerificationJobParseError::new(
                line,
                leading + value_start + 1,
                CertificateVerificationJobParseErrorKind::EmptyPath {
                    directive: directive.clone(),
                },
            ));
        }

        match directive.as_str() {
            MODEL => model_path = Some(value),
            PROPERTY => property_path = Some(value),
            CERTIFICATE => certificate_path = Some(value),
            _ => unreachable!("directive was validated above"),
        }
    }

    Ok(CertificateVerificationJob {
        model_path: required(model_path, MODEL)?,
        property_path: required(property_path, PROPERTY)?,
        certificate_path: required(certificate_path, CERTIFICATE)?,
    })
}

fn required(
    value: Option<String>,
    directive: &str,
) -> Result<String, CertificateVerificationJobParseError> {
    value.ok_or_else(|| {
        CertificateVerificationJobParseError::new(
            1,
            1,
            CertificateVerificationJobParseErrorKind::MissingDirective {
                directive: directive.to_owned(),
            },
        )
    })
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
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

type ParseError = (usize, CertificateVerificationJobParseErrorKind);

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
        while self.peek().is_some_and(|ch| ch.is_ascii_whitespace()) {
            let ch = self.peek().expect("peeked character exists");
            self.position += ch.len_utf8();
        }
    }

    fn parse_directive(&mut self) -> Result<String, CertificateVerificationJobParseErrorKind> {
        let start = self.position;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_whitespace() {
                break;
            }
            self.position += ch.len_utf8();
        }
        if self.position == start {
            return Err(CertificateVerificationJobParseErrorKind::ExpectedDirective);
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        let start = self.position;
        if self.peek() != Some('"') {
            return Err((
                start,
                CertificateVerificationJobParseErrorKind::ExpectedString,
            ));
        }
        self.position += 1;
        let mut output = String::new();

        loop {
            let Some(ch) = self.peek() else {
                return Err((
                    start,
                    CertificateVerificationJobParseErrorKind::UnterminatedString,
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
                            CertificateVerificationJobParseErrorKind::UnterminatedString,
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
                                CertificateVerificationJobParseErrorKind::InvalidEscape {
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

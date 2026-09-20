use crate::ctl::CtlFormula;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtlParseErrorKind {
    ExpectedExpression,
    ExpectedCloseParen,
    ExpectedOpenBracket { quantifier: char },
    ExpectedUntil,
    ExpectedCloseBracket,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyProposition,
    TrailingInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtlParseError {
    position: usize,
    kind: CtlParseErrorKind,
}

impl CtlParseError {
    fn new(position: usize, kind: CtlParseErrorKind) -> Self {
        Self { position, kind }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn kind(&self) -> &CtlParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for CtlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CTL parse error at byte {}: ", self.position)?;
        match &self.kind {
            CtlParseErrorKind::ExpectedExpression => write!(
                f,
                "expected a quoted proposition, true, false, not, CTL unary operator, until formula, or '('"
            ),
            CtlParseErrorKind::ExpectedCloseParen => write!(f, "expected ')'"),
            CtlParseErrorKind::ExpectedOpenBracket { quantifier } => {
                write!(f, "expected '[' after '{quantifier}'")
            }
            CtlParseErrorKind::ExpectedUntil => write!(f, "expected 'U' in CTL until formula"),
            CtlParseErrorKind::ExpectedCloseBracket => write!(f, "expected ']'"),
            CtlParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted proposition")
            }
            CtlParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            CtlParseErrorKind::EmptyProposition => {
                write!(f, "proposition name must not be empty")
            }
            CtlParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after CTL expression")
            }
        }
    }
}

impl std::error::Error for CtlParseError {}

/// Parse the deterministic textual CTL surface.
///
/// Precedence, from tightest to loosest:
///
/// - unary: `not`, `EX`, `AX`, `EF`, `AF`, `EG`, `AG`;
/// - `and`;
/// - `or`.
///
/// Until is explicit and bracketed: `E[phi U psi]` or `A[phi U psi]`.
/// Atoms are double-quoted proposition names. ASCII whitespace is allowed
/// between tokens.
pub fn parse_ctl_formula(input: &str) -> Result<CtlFormula<String>, CtlParseError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let formula = parser.parse_or()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(CtlParseError::new(
            parser.position,
            CtlParseErrorKind::TrailingInput,
        ));
    }
    Ok(formula)
}

/// Render a stable, fully-parenthesized CTL expression that round-trips through
/// `parse_ctl_formula`.
pub fn render_ctl_formula(formula: &CtlFormula<String>) -> String {
    match formula {
        CtlFormula::True => "true".to_owned(),
        CtlFormula::False => "false".to_owned(),
        CtlFormula::Atom(atom) => quote_string(atom),
        CtlFormula::Not(inner) => format!("not ({})", render_ctl_formula(inner)),
        CtlFormula::And(left, right) => format!(
            "({} and {})",
            render_ctl_formula(left),
            render_ctl_formula(right)
        ),
        CtlFormula::Or(left, right) => format!(
            "({} or {})",
            render_ctl_formula(left),
            render_ctl_formula(right)
        ),
        CtlFormula::Ex(inner) => format!("EX ({})", render_ctl_formula(inner)),
        CtlFormula::Ax(inner) => format!("AX ({})", render_ctl_formula(inner)),
        CtlFormula::Ef(inner) => format!("EF ({})", render_ctl_formula(inner)),
        CtlFormula::Af(inner) => format!("AF ({})", render_ctl_formula(inner)),
        CtlFormula::Eg(inner) => format!("EG ({})", render_ctl_formula(inner)),
        CtlFormula::Ag(inner) => format!("AG ({})", render_ctl_formula(inner)),
        CtlFormula::Eu(left, right) => format!(
            "E[({}) U ({})]",
            render_ctl_formula(left),
            render_ctl_formula(right)
        ),
        CtlFormula::Au(left, right) => format!(
            "A[({}) U ({})]",
            render_ctl_formula(left),
            render_ctl_formula(right)
        ),
    }
}

pub(crate) fn collect_ctl_atoms<'a>(
    formula: &'a CtlFormula<String>,
    atoms: &mut Vec<&'a str>,
) {
    match formula {
        CtlFormula::Atom(atom) => atoms.push(atom),
        CtlFormula::Not(inner)
        | CtlFormula::Ex(inner)
        | CtlFormula::Ax(inner)
        | CtlFormula::Ef(inner)
        | CtlFormula::Af(inner)
        | CtlFormula::Eg(inner)
        | CtlFormula::Ag(inner) => collect_ctl_atoms(inner, atoms),
        CtlFormula::And(left, right)
        | CtlFormula::Or(left, right)
        | CtlFormula::Eu(left, right)
        | CtlFormula::Au(left, right) => {
            collect_ctl_atoms(left, atoms);
            collect_ctl_atoms(right, atoms);
        }
        CtlFormula::True | CtlFormula::False => {}
    }
}

struct Parser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Parser<'a> {
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

    fn parse_or(&mut self) -> Result<CtlFormula<String>, CtlParseError> {
        let mut formula = self.parse_and()?;
        loop {
            self.skip_whitespace();
            if !self.consume_keyword("or") {
                break;
            }
            let right = self.parse_and()?;
            formula = CtlFormula::or(formula, right);
        }
        Ok(formula)
    }

    fn parse_and(&mut self) -> Result<CtlFormula<String>, CtlParseError> {
        let mut formula = self.parse_unary()?;
        loop {
            self.skip_whitespace();
            if !self.consume_keyword("and") {
                break;
            }
            let right = self.parse_unary()?;
            formula = CtlFormula::and(formula, right);
        }
        Ok(formula)
    }

    fn parse_unary(&mut self) -> Result<CtlFormula<String>, CtlParseError> {
        self.skip_whitespace();

        if self.consume_keyword("not") {
            return Ok(CtlFormula::negate(self.parse_unary()?));
        }
        if self.consume_keyword("EX") {
            return Ok(CtlFormula::ex(self.parse_unary()?));
        }
        if self.consume_keyword("AX") {
            return Ok(CtlFormula::ax(self.parse_unary()?));
        }
        if self.consume_keyword("EF") {
            return Ok(CtlFormula::ef(self.parse_unary()?));
        }
        if self.consume_keyword("AF") {
            return Ok(CtlFormula::af(self.parse_unary()?));
        }
        if self.consume_keyword("EG") {
            return Ok(CtlFormula::eg(self.parse_unary()?));
        }
        if self.consume_keyword("AG") {
            return Ok(CtlFormula::ag(self.parse_unary()?));
        }
        if self.consume_keyword("E") {
            return self.parse_until('E');
        }
        if self.consume_keyword("A") {
            return self.parse_until('A');
        }

        self.parse_primary()
    }

    fn parse_until(&mut self, quantifier: char) -> Result<CtlFormula<String>, CtlParseError> {
        self.skip_whitespace();
        if self.current_byte() != Some(b'[') {
            return Err(CtlParseError::new(
                self.position,
                CtlParseErrorKind::ExpectedOpenBracket { quantifier },
            ));
        }
        self.position += 1;

        let left = self.parse_or()?;
        self.skip_whitespace();
        if !self.consume_keyword("U") {
            return Err(CtlParseError::new(
                self.position,
                CtlParseErrorKind::ExpectedUntil,
            ));
        }
        let right = self.parse_or()?;
        self.skip_whitespace();
        if self.current_byte() != Some(b']') {
            return Err(CtlParseError::new(
                self.position,
                CtlParseErrorKind::ExpectedCloseBracket,
            ));
        }
        self.position += 1;

        Ok(match quantifier {
            'E' => CtlFormula::eu(left, right),
            'A' => CtlFormula::au(left, right),
            _ => unreachable!("only E/A dispatch reaches parse_until"),
        })
    }

    fn parse_primary(&mut self) -> Result<CtlFormula<String>, CtlParseError> {
        self.skip_whitespace();
        let start = self.position;

        if self.consume_keyword("true") {
            return Ok(CtlFormula::True);
        }
        if self.consume_keyword("false") {
            return Ok(CtlFormula::False);
        }

        match self.current_byte() {
            Some(b'"') => {
                let atom = self.parse_string()?;
                if atom.trim().is_empty() {
                    return Err(CtlParseError::new(
                        start,
                        CtlParseErrorKind::EmptyProposition,
                    ));
                }
                Ok(CtlFormula::atom(atom))
            }
            Some(b'(') => {
                self.position += 1;
                let formula = self.parse_or()?;
                self.skip_whitespace();
                if self.current_byte() != Some(b')') {
                    return Err(CtlParseError::new(
                        self.position,
                        CtlParseErrorKind::ExpectedCloseParen,
                    ));
                }
                self.position += 1;
                Ok(formula)
            }
            _ => Err(CtlParseError::new(
                start,
                CtlParseErrorKind::ExpectedExpression,
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

    fn parse_string(&mut self) -> Result<String, CtlParseError> {
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
                        return Err(CtlParseError::new(
                            start,
                            CtlParseErrorKind::UnterminatedString,
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
                            return Err(CtlParseError::new(
                                escape_position,
                                CtlParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }

        Err(CtlParseError::new(
            start,
            CtlParseErrorKind::UnterminatedString,
        ))
    }
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
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

use crate::mu_calculus::MuFormula;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuParseErrorKind {
    ExpectedExpression,
    ExpectedCloseParen,
    ExpectedVariable,
    ExpectedDot,
    UnterminatedString,
    InvalidEscape { escape: String },
    EmptyProposition,
    TrailingInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParseError {
    position: usize,
    kind: MuParseErrorKind,
}

impl MuParseError {
    fn new(position: usize, kind: MuParseErrorKind) -> Self {
        Self { position, kind }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn kind(&self) -> &MuParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for MuParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mu-calculus parse error at byte {}: ", self.position)?;
        match &self.kind {
            MuParseErrorKind::ExpectedExpression => write!(
                f,
                "expected a quoted proposition, $variable, true, false, not, diamond, box, mu/nu binder, or '('"
            ),
            MuParseErrorKind::ExpectedCloseParen => write!(f, "expected ')'"),
            MuParseErrorKind::ExpectedVariable => write!(f, "expected a fixpoint variable identifier"),
            MuParseErrorKind::ExpectedDot => write!(f, "expected '.' after fixpoint binder"),
            MuParseErrorKind::UnterminatedString => write!(f, "unterminated double-quoted proposition"),
            MuParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            MuParseErrorKind::EmptyProposition => write!(f, "proposition name must not be empty"),
            MuParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after mu-calculus expression")
            }
        }
    }
}

impl std::error::Error for MuParseError {}

pub fn parse_mu_formula(input: &str) -> Result<MuFormula<String, String>, MuParseError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let formula = parser.parse_or()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(MuParseError::new(
            parser.position,
            MuParseErrorKind::TrailingInput,
        ));
    }
    Ok(formula)
}

pub fn render_mu_formula(formula: &MuFormula<String, String>) -> String {
    match formula {
        MuFormula::True => "true".to_owned(),
        MuFormula::False => "false".to_owned(),
        MuFormula::Atom(atom) => quote_string(atom),
        MuFormula::Var(variable) => format!("${variable}"),
        MuFormula::Not(inner) => format!("not ({})", render_mu_formula(inner)),
        MuFormula::And(left, right) => format!(
            "({} and {})",
            render_mu_formula(left),
            render_mu_formula(right)
        ),
        MuFormula::Or(left, right) => format!(
            "({} or {})",
            render_mu_formula(left),
            render_mu_formula(right)
        ),
        MuFormula::Diamond(inner) => format!("diamond ({})", render_mu_formula(inner)),
        MuFormula::Box(inner) => format!("box ({})", render_mu_formula(inner)),
        MuFormula::Mu { variable, body } => {
            format!("mu {variable}. ({})", render_mu_formula(body))
        }
        MuFormula::Nu { variable, body } => {
            format!("nu {variable}. ({})", render_mu_formula(body))
        }
    }
}

pub(crate) fn collect_mu_atoms<'a>(
    formula: &'a MuFormula<String, String>,
    atoms: &mut Vec<&'a str>,
) {
    match formula {
        MuFormula::Atom(atom) => atoms.push(atom),
        MuFormula::Not(inner) | MuFormula::Diamond(inner) | MuFormula::Box(inner) => {
            collect_mu_atoms(inner, atoms)
        }
        MuFormula::And(left, right) | MuFormula::Or(left, right) => {
            collect_mu_atoms(left, atoms);
            collect_mu_atoms(right, atoms);
        }
        MuFormula::Mu { body, .. } | MuFormula::Nu { body, .. } => collect_mu_atoms(body, atoms),
        MuFormula::True | MuFormula::False | MuFormula::Var(_) => {}
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

    fn parse_or(&mut self) -> Result<MuFormula<String, String>, MuParseError> {
        let mut formula = self.parse_and()?;
        loop {
            self.skip_whitespace();
            if !self.consume_keyword("or") {
                break;
            }
            let right = self.parse_and()?;
            formula = MuFormula::or(formula, right);
        }
        Ok(formula)
    }

    fn parse_and(&mut self) -> Result<MuFormula<String, String>, MuParseError> {
        let mut formula = self.parse_unary()?;
        loop {
            self.skip_whitespace();
            if !self.consume_keyword("and") {
                break;
            }
            let right = self.parse_unary()?;
            formula = MuFormula::and(formula, right);
        }
        Ok(formula)
    }

    fn parse_unary(&mut self) -> Result<MuFormula<String, String>, MuParseError> {
        self.skip_whitespace();
        if self.consume_keyword("not") {
            return Ok(MuFormula::negate(self.parse_unary()?));
        }
        if self.consume_keyword("diamond") {
            return Ok(MuFormula::diamond(self.parse_unary()?));
        }
        if self.consume_keyword("box") {
            return Ok(MuFormula::boxed(self.parse_unary()?));
        }
        if self.consume_keyword("mu") {
            return self.parse_binder(false);
        }
        if self.consume_keyword("nu") {
            return self.parse_binder(true);
        }
        self.parse_primary()
    }

    fn parse_binder(&mut self, greatest: bool) -> Result<MuFormula<String, String>, MuParseError> {
        self.skip_whitespace();
        let variable = self.parse_identifier()?;
        self.skip_whitespace();
        if self.current_byte() != Some(b'.') {
            return Err(MuParseError::new(
                self.position,
                MuParseErrorKind::ExpectedDot,
            ));
        }
        self.position += 1;
        let body = self.parse_or()?;
        Ok(if greatest {
            MuFormula::nu(variable, body)
        } else {
            MuFormula::mu(variable, body)
        })
    }

    fn parse_primary(&mut self) -> Result<MuFormula<String, String>, MuParseError> {
        self.skip_whitespace();
        let start = self.position;
        if self.consume_keyword("true") {
            return Ok(MuFormula::True);
        }
        if self.consume_keyword("false") {
            return Ok(MuFormula::False);
        }
        match self.current_byte() {
            Some(b'"') => {
                let atom = self.parse_string()?;
                if atom.trim().is_empty() {
                    return Err(MuParseError::new(start, MuParseErrorKind::EmptyProposition));
                }
                Ok(MuFormula::atom(atom))
            }
            Some(b'$') => {
                self.position += 1;
                Ok(MuFormula::var(self.parse_identifier()?))
            }
            Some(b'(') => {
                self.position += 1;
                let formula = self.parse_or()?;
                self.skip_whitespace();
                if self.current_byte() != Some(b')') {
                    return Err(MuParseError::new(
                        self.position,
                        MuParseErrorKind::ExpectedCloseParen,
                    ));
                }
                self.position += 1;
                Ok(formula)
            }
            _ => Err(MuParseError::new(
                start,
                MuParseErrorKind::ExpectedExpression,
            )),
        }
    }

    fn parse_identifier(&mut self) -> Result<String, MuParseError> {
        self.skip_whitespace();
        let start = self.position;
        let Some(first) = self.current_byte() else {
            return Err(MuParseError::new(start, MuParseErrorKind::ExpectedVariable));
        };
        if !is_identifier_start(first) {
            return Err(MuParseError::new(start, MuParseErrorKind::ExpectedVariable));
        }
        self.position += 1;
        while self.current_byte().is_some_and(is_identifier_byte) {
            self.position += 1;
        }
        Ok(self.input[start..self.position].to_owned())
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

    fn parse_string(&mut self) -> Result<String, MuParseError> {
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
                        return Err(MuParseError::new(
                            start,
                            MuParseErrorKind::UnterminatedString,
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
                            return Err(MuParseError::new(
                                escape_position,
                                MuParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }
        Err(MuParseError::new(
            start,
            MuParseErrorKind::UnterminatedString,
        ))
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
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

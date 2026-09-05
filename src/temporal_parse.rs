use crate::temporal::{ActionAtom, ActionTemporalSpec, TemporalError};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalParseErrorKind {
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
    Semantic(TemporalError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalParseError {
    position: usize,
    kind: TemporalParseErrorKind,
}

impl TemporalParseError {
    fn new(position: usize, kind: TemporalParseErrorKind) -> Self {
        Self { position, kind }
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn kind(&self) -> &TemporalParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for TemporalParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "temporal parse error at byte {}: ", self.position)?;
        match &self.kind {
            TemporalParseErrorKind::ExpectedOperator => write!(
                f,
                "expected temporal operator 'response' or 'infinitely-often'"
            ),
            TemporalParseErrorKind::UnknownOperator { operator } => {
                write!(f, "unsupported temporal operator '{operator}'")
            }
            TemporalParseErrorKind::ExpectedOpenParen => write!(f, "expected '('") ,
            TemporalParseErrorKind::ExpectedString => {
                write!(f, "expected a double-quoted exact action label")
            }
            TemporalParseErrorKind::UnterminatedString => {
                write!(f, "unterminated double-quoted action label")
            }
            TemporalParseErrorKind::InvalidEscape { escape } => {
                write!(f, "unsupported string escape '\\{escape}'")
            }
            TemporalParseErrorKind::ExpectedCommaOrClose => write!(f, "expected ',' or ')'") ,
            TemporalParseErrorKind::WrongArity {
                operator,
                expected,
                actual,
            } => write!(
                f,
                "operator '{operator}' expects {expected} arguments but received {actual}"
            ),
            TemporalParseErrorKind::TrailingInput => {
                write!(f, "unexpected trailing input after temporal expression")
            }
            TemporalParseErrorKind::Semantic(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for TemporalParseError {}

/// Parse exactly the typed action-temporal subset implemented by M17.
///
/// Grammar, with ASCII whitespace allowed between tokens:
///
/// ```text
/// response("trigger","response")
/// infinitely-often("action"[,"action"...])
/// ```
///
/// String literals support `\\`, `\"`, `\n`, `\r`, and `\t`. Positions in
/// parse errors are UTF-8 byte offsets. Parsed metadata is routed through the
/// typed M17 constructors so semantic validation remains single-sourced.
pub fn parse_action_temporal(
    name: impl Into<String>,
    input: &str,
) -> Result<ActionTemporalSpec, TemporalParseError> {
    let mut parser = Parser::new(input);
    parser.skip_whitespace();
    let operator_start = parser.position;
    let operator = parser.parse_operator()?;
    if operator != "response" && operator != "infinitely-often" {
        return Err(TemporalParseError::new(
            operator_start,
            TemporalParseErrorKind::UnknownOperator { operator },
        ));
    }

    parser.skip_whitespace();
    parser.expect_byte(b'(', TemporalParseErrorKind::ExpectedOpenParen)?;
    let (arguments, close_position) = parser.parse_arguments()?;
    parser.skip_whitespace();
    if !parser.is_eof() {
        return Err(TemporalParseError::new(
            parser.position,
            TemporalParseErrorKind::TrailingInput,
        ));
    }

    let name = name.into();
    match operator.as_str() {
        "response" => {
            if arguments.len() != 2 {
                return Err(TemporalParseError::new(
                    close_position,
                    TemporalParseErrorKind::WrongArity {
                        operator,
                        expected: 2,
                        actual: arguments.len(),
                    },
                ));
            }
            let trigger = parse_atom(&arguments[0])?;
            let response = parse_atom(&arguments[1])?;
            ActionTemporalSpec::response(name, trigger, response).map_err(|error| {
                TemporalParseError::new(operator_start, TemporalParseErrorKind::Semantic(error))
            })
        }
        "infinitely-often" => {
            let actions = arguments
                .iter()
                .map(parse_atom)
                .collect::<Result<Vec<_>, _>>()?;
            ActionTemporalSpec::all_infinitely_often(name, actions).map_err(|error| {
                TemporalParseError::new(operator_start, TemporalParseErrorKind::Semantic(error))
            })
        }
        _ => unreachable!("operator was validated before parsing arguments"),
    }
}

fn parse_atom(argument: &(String, usize)) -> Result<ActionAtom, TemporalParseError> {
    ActionAtom::exact(argument.0.clone()).map_err(|error| {
        TemporalParseError::new(argument.1, TemporalParseErrorKind::Semantic(error))
    })
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

    fn parse_operator(&mut self) -> Result<String, TemporalParseError> {
        let start = self.position;
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(TemporalParseError::new(
                start,
                TemporalParseErrorKind::ExpectedOperator,
            ));
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn expect_byte(
        &mut self,
        expected: u8,
        kind: TemporalParseErrorKind,
    ) -> Result<(), TemporalParseError> {
        if self.current_byte() == Some(expected) {
            self.position += 1;
            Ok(())
        } else {
            Err(TemporalParseError::new(self.position, kind))
        }
    }

    fn parse_arguments(&mut self) -> Result<(Vec<(String, usize)>, usize), TemporalParseError> {
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
                    return Err(TemporalParseError::new(
                        self.position,
                        TemporalParseErrorKind::ExpectedCommaOrClose,
                    ));
                }
            }
        }
    }

    fn parse_string(&mut self) -> Result<(String, usize), TemporalParseError> {
        let start = self.position;
        if self.current_byte() != Some(b'"') {
            return Err(TemporalParseError::new(
                start,
                TemporalParseErrorKind::ExpectedString,
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
                        return Err(TemporalParseError::new(
                            start,
                            TemporalParseErrorKind::UnterminatedString,
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
                            return Err(TemporalParseError::new(
                                escape_position,
                                TemporalParseErrorKind::InvalidEscape {
                                    escape: escape.to_string(),
                                },
                            ));
                        }
                    }
                }
                _ => output.push(ch),
            }
        }

        Err(TemporalParseError::new(
            start,
            TemporalParseErrorKind::UnterminatedString,
        ))
    }
}

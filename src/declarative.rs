use crate::model::{Invariant, ModelError, StateVariable, Transition, TransitionSystem};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarativeModelError {
    MissingModel,
    DuplicateModel {
        line: usize,
    },
    EmptyModelName {
        line: usize,
    },
    NoStates,
    EmptyStateName {
        line: usize,
    },
    DuplicateState {
        line: usize,
        state: String,
    },
    NoInitialStates,
    DuplicateInitial {
        line: usize,
        state: String,
    },
    UnknownInitialState {
        line: usize,
        state: String,
    },
    EmptyAction {
        line: usize,
    },
    UnknownEdgeSource {
        line: usize,
        state: String,
    },
    UnknownEdgeTarget {
        line: usize,
        state: String,
    },
    DuplicateEdge {
        line: usize,
        from: String,
        action: String,
        to: String,
    },
    EmptyProposition {
        line: usize,
    },
    UnknownLabelState {
        line: usize,
        state: String,
    },
    DuplicateLabel {
        line: usize,
        state: String,
        proposition: String,
    },
    ExpectedDirective {
        line: usize,
        column: usize,
    },
    UnknownDirective {
        line: usize,
        directive: String,
    },
    ExpectedString {
        line: usize,
        column: usize,
    },
    UnterminatedString {
        line: usize,
        column: usize,
    },
    InvalidEscape {
        line: usize,
        column: usize,
        escape: String,
    },
    WrongArity {
        line: usize,
        directive: String,
        expected: usize,
        actual: usize,
    },
    Model(ModelError),
}

impl fmt::Display for DeclarativeModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingModel => write!(f, "declarative model is missing a model directive"),
            Self::DuplicateModel { line } => {
                write!(f, "duplicate model directive at line {line}")
            }
            Self::EmptyModelName { line } => write!(f, "empty model name at line {line}"),
            Self::NoStates => write!(f, "declarative model must declare at least one state"),
            Self::EmptyStateName { line } => write!(f, "empty state name at line {line}"),
            Self::DuplicateState { line, state } => {
                write!(f, "duplicate state '{state}' at line {line}")
            }
            Self::NoInitialStates => {
                write!(f, "declarative model must declare at least one initial state")
            }
            Self::DuplicateInitial { line, state } => {
                write!(f, "duplicate initial state '{state}' at line {line}")
            }
            Self::UnknownInitialState { line, state } => {
                write!(f, "initial state '{state}' at line {line} is not declared")
            }
            Self::EmptyAction { line } => write!(f, "empty edge action at line {line}"),
            Self::UnknownEdgeSource { line, state } => {
                write!(f, "edge source '{state}' at line {line} is not declared")
            }
            Self::UnknownEdgeTarget { line, state } => {
                write!(f, "edge target '{state}' at line {line} is not declared")
            }
            Self::DuplicateEdge {
                line,
                from,
                action,
                to,
            } => write!(
                f,
                "duplicate edge '{from}' --{action}--> '{to}' at line {line}"
            ),
            Self::EmptyProposition { line } => {
                write!(f, "empty proposition name at line {line}")
            }
            Self::UnknownLabelState { line, state } => {
                write!(f, "label state '{state}' at line {line} is not declared")
            }
            Self::DuplicateLabel {
                line,
                state,
                proposition,
            } => write!(
                f,
                "duplicate label '{proposition}' on state '{state}' at line {line}"
            ),
            Self::ExpectedDirective { line, column } => {
                write!(f, "expected directive at line {line}, column {column}")
            }
            Self::UnknownDirective { line, directive } => {
                write!(f, "unknown directive '{directive}' at line {line}")
            }
            Self::ExpectedString { line, column } => write!(
                f,
                "expected a double-quoted string at line {line}, column {column}"
            ),
            Self::UnterminatedString { line, column } => write!(
                f,
                "unterminated double-quoted string at line {line}, column {column}"
            ),
            Self::InvalidEscape {
                line,
                column,
                escape,
            } => write!(
                f,
                "unsupported string escape '\\{escape}' at line {line}, column {column}"
            ),
            Self::WrongArity {
                line,
                directive,
                expected,
                actual,
            } => write!(
                f,
                "directive '{directive}' at line {line} expects {expected} arguments but received {actual}"
            ),
            Self::Model(error) => write!(f, "canonical model construction failed: {error}"),
        }
    }
}

impl std::error::Error for DeclarativeModelError {}

impl From<ModelError> for DeclarativeModelError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

#[derive(Debug, Clone)]
struct EdgeDecl {
    line: usize,
    from: String,
    action: String,
    to: String,
}

#[derive(Debug, Clone)]
struct LabelDecl {
    line: usize,
    state: String,
    proposition: String,
}

/// A parsed declarative document owns proposition metadata alongside the same
/// canonical transition system used by every verification backend.
///
/// Propositions are ingestion metadata only. They do not become safety
/// invariants, do not alter the transition relation, and do not introduce a
/// second graph representation.
#[derive(Debug)]
pub struct DeclarativeDocument {
    model: TransitionSystem<String>,
    propositions: BTreeMap<String, Vec<String>>,
}

impl DeclarativeDocument {
    pub fn model(&self) -> &TransitionSystem<String> {
        &self.model
    }

    pub fn into_model(self) -> TransitionSystem<String> {
        self.model
    }

    /// Return proposition members in label declaration order.
    pub fn proposition_states(&self, proposition: &str) -> Option<&[String]> {
        self.propositions.get(proposition).map(Vec::as_slice)
    }

    pub fn state_has_proposition(&self, state: &str, proposition: &str) -> bool {
        self.proposition_states(proposition)
            .is_some_and(|states| states.iter().any(|candidate| candidate == state))
    }
}

/// Parse a deterministic line-oriented finite labeled-graph document with
/// optional named state propositions.
///
/// Supported directives are:
///
/// ```text
/// model "name"
/// state "state-id"
/// initial "state-id"
/// edge "from" "action" "to"
/// label "state-id" "proposition"
/// ```
///
/// Blank lines and lines whose first non-whitespace byte is `#` are ignored.
/// Strings support `\\`, `\"`, `\n`, `\r`, and `\t`. Initial-state,
/// per-source edge, and per-proposition member ordering follows declaration
/// order for deterministic witnesses and metadata inspection.
pub fn parse_declarative_document(
    input: &str,
) -> Result<DeclarativeDocument, DeclarativeModelError> {
    let mut model_name: Option<String> = None;
    let mut states = Vec::new();
    let mut state_set = HashSet::new();
    let mut initials: Vec<(usize, String)> = Vec::new();
    let mut initial_set = HashSet::new();
    let mut edges = Vec::new();
    let mut edge_set: HashSet<(String, String, String)> = HashSet::new();
    let mut labels = Vec::new();
    let mut label_set: HashSet<(String, String)> = HashSet::new();

    for (line_index, text) in input.lines().enumerate() {
        let line = line_index + 1;
        let mut parser = LineParser::new(text, line);
        parser.skip_whitespace();
        if parser.is_eof() || parser.current_byte() == Some(b'#') {
            continue;
        }

        let directive = parser.parse_directive()?;
        let arguments = parser.parse_arguments()?;
        let expected = match directive.as_str() {
            "model" | "state" | "initial" => 1,
            "label" => 2,
            "edge" => 3,
            _ => {
                return Err(DeclarativeModelError::UnknownDirective { line, directive });
            }
        };
        if arguments.len() != expected {
            return Err(DeclarativeModelError::WrongArity {
                line,
                directive,
                expected,
                actual: arguments.len(),
            });
        }

        match directive.as_str() {
            "model" => {
                if model_name.is_some() {
                    return Err(DeclarativeModelError::DuplicateModel { line });
                }
                if arguments[0].trim().is_empty() {
                    return Err(DeclarativeModelError::EmptyModelName { line });
                }
                model_name = Some(arguments[0].clone());
            }
            "state" => {
                let state = arguments[0].clone();
                if state.trim().is_empty() {
                    return Err(DeclarativeModelError::EmptyStateName { line });
                }
                if !state_set.insert(state.clone()) {
                    return Err(DeclarativeModelError::DuplicateState { line, state });
                }
                states.push(state);
            }
            "initial" => {
                let state = arguments[0].clone();
                if !initial_set.insert(state.clone()) {
                    return Err(DeclarativeModelError::DuplicateInitial { line, state });
                }
                initials.push((line, state));
            }
            "edge" => {
                let from = arguments[0].clone();
                let action = arguments[1].clone();
                let to = arguments[2].clone();
                if action.trim().is_empty() {
                    return Err(DeclarativeModelError::EmptyAction { line });
                }
                if !edge_set.insert((from.clone(), action.clone(), to.clone())) {
                    return Err(DeclarativeModelError::DuplicateEdge {
                        line,
                        from,
                        action,
                        to,
                    });
                }
                edges.push(EdgeDecl {
                    line,
                    from,
                    action,
                    to,
                });
            }
            "label" => {
                let state = arguments[0].clone();
                let proposition = arguments[1].clone();
                if proposition.trim().is_empty() {
                    return Err(DeclarativeModelError::EmptyProposition { line });
                }
                if !label_set.insert((state.clone(), proposition.clone())) {
                    return Err(DeclarativeModelError::DuplicateLabel {
                        line,
                        state,
                        proposition,
                    });
                }
                labels.push(LabelDecl {
                    line,
                    state,
                    proposition,
                });
            }
            _ => unreachable!("directive was validated before dispatch"),
        }
    }

    let name = model_name.ok_or(DeclarativeModelError::MissingModel)?;
    if states.is_empty() {
        return Err(DeclarativeModelError::NoStates);
    }
    if initials.is_empty() {
        return Err(DeclarativeModelError::NoInitialStates);
    }

    for (line, state) in &initials {
        if !state_set.contains(state) {
            return Err(DeclarativeModelError::UnknownInitialState {
                line: *line,
                state: state.clone(),
            });
        }
    }
    for edge in &edges {
        if !state_set.contains(&edge.from) {
            return Err(DeclarativeModelError::UnknownEdgeSource {
                line: edge.line,
                state: edge.from.clone(),
            });
        }
        if !state_set.contains(&edge.to) {
            return Err(DeclarativeModelError::UnknownEdgeTarget {
                line: edge.line,
                state: edge.to.clone(),
            });
        }
    }
    for label in &labels {
        if !state_set.contains(&label.state) {
            return Err(DeclarativeModelError::UnknownLabelState {
                line: label.line,
                state: label.state.clone(),
            });
        }
    }

    let mut adjacency: HashMap<String, Vec<Transition<String>>> = states
        .iter()
        .cloned()
        .map(|state| (state, Vec::new()))
        .collect();
    for edge in edges {
        adjacency
            .get_mut(&edge.from)
            .expect("edge source was validated against declared states")
            .push(Transition::new(edge.action, edge.to));
    }

    let mut propositions: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for label in labels {
        propositions
            .entry(label.proposition)
            .or_default()
            .push(label.state);
    }

    let initial_states = initials.into_iter().map(|(_, state)| state).collect();
    let invariant_domain = state_set.clone();
    let model = TransitionSystem::new(
        name,
        vec![StateVariable::new("state", "declarative state id")],
        initial_states,
        move |state: &String| {
            adjacency
                .get(state)
                .cloned()
                .ok_or_else(|| ModelError::TransitionGeneration {
                    message: format!("state '{state}' is outside the declared model domain"),
                })
        },
        vec![Invariant::new(
            "declared-state-domain",
            move |state: &String| invariant_domain.contains(state),
        )],
    )
    .map_err(DeclarativeModelError::Model)?;

    Ok(DeclarativeDocument {
        model,
        propositions,
    })
}

/// Backward-compatible M19 graph loader. Proposition metadata is accepted but
/// deliberately discarded when only the canonical transition system is
/// requested.
pub fn parse_declarative_model(
    input: &str,
) -> Result<TransitionSystem<String>, DeclarativeModelError> {
    Ok(parse_declarative_document(input)?.into_model())
}

struct LineParser<'a> {
    input: &'a str,
    line: usize,
    position: usize,
}

impl<'a> LineParser<'a> {
    fn new(input: &'a str, line: usize) -> Self {
        Self {
            input,
            line,
            position: 0,
        }
    }

    fn is_eof(&self) -> bool {
        self.position == self.input.len()
    }

    fn current_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    fn column(&self) -> usize {
        self.position + 1
    }

    fn skip_whitespace(&mut self) {
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            self.position += 1;
        }
    }

    fn parse_directive(&mut self) -> Result<String, DeclarativeModelError> {
        let start = self.position;
        while self
            .current_byte()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'-')
        {
            self.position += 1;
        }
        if self.position == start {
            return Err(DeclarativeModelError::ExpectedDirective {
                line: self.line,
                column: self.column(),
            });
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_arguments(&mut self) -> Result<Vec<String>, DeclarativeModelError> {
        let mut arguments = Vec::new();
        loop {
            self.skip_whitespace();
            if self.is_eof() {
                return Ok(arguments);
            }
            arguments.push(self.parse_string()?);
        }
    }

    fn parse_string(&mut self) -> Result<String, DeclarativeModelError> {
        let start = self.position;
        if self.current_byte() != Some(b'"') {
            return Err(DeclarativeModelError::ExpectedString {
                line: self.line,
                column: self.column(),
            });
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
                '"' => return Ok(output),
                '\\' => {
                    let escape_column = self.position;
                    if self.is_eof() {
                        return Err(DeclarativeModelError::UnterminatedString {
                            line: self.line,
                            column: start + 1,
                        });
                    }
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
                            return Err(DeclarativeModelError::InvalidEscape {
                                line: self.line,
                                column: escape_column,
                                escape: escape.to_string(),
                            });
                        }
                    }
                }
                _ => output.push(ch),
            }
        }

        Err(DeclarativeModelError::UnterminatedString {
            line: self.line,
            column: start + 1,
        })
    }
}

use crate::declarative::DeclarativeDocument;
use crate::declarative_mu::{
    check_declarative_mu_text_via_parity, validate_declarative_mu_formula, DeclarativeMuError,
};
use crate::graph::capture_reachable_graph;
use crate::mu_calculus::{MuInitialEvaluation, MuTerminalPolicy};
use crate::mu_parity::{
    verify_mu_parity_evidence, MuParityEvaluation, MuParityEvidenceError, MuParityFixpointKind,
    MuParityInitialEvidence, MuParityMove, MuParityPosition, MuParityPositionKind,
    MuParityStrategyChoice, MuParityStrategyEvidence,
};
use crate::mu_parse::{parse_mu_formula, render_mu_formula, MuParseError};
use crate::parity_game::ParityPlayer;
use std::fmt;
use std::fmt::Write as _;

pub const DECLARATIVE_MU_PARITY_CERTIFICATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityCertificateChoice {
    pub from_vertex: usize,
    pub to_vertex: usize,
    pub semantic_move: MuParityMove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityCertificateInitial {
    pub state_index: usize,
    pub satisfied: bool,
    pub winner: ParityPlayer,
    pub root_vertex: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMuParityCertificate {
    pub schema_version: u32,
    pub model_binding: String,
    pub formula: String,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub parity_game_vertices: usize,
    pub max_priority: usize,
    pub satisfying_state_indices: Vec<usize>,
    pub positions: Vec<MuParityPosition>,
    pub even_winning_vertices: Vec<usize>,
    pub odd_winning_vertices: Vec<usize>,
    pub even_choices: Vec<MuParityCertificateChoice>,
    pub odd_choices: Vec<MuParityCertificateChoice>,
    pub initial: Vec<MuParityCertificateInitial>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuParityCertificateParseError {
    Line {
        line: usize,
        message: String,
    },
    Missing {
        directive: &'static str,
    },
    UnsupportedVersion {
        version: u32,
    },
    CountMismatch {
        section: &'static str,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for MuParityCertificateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Line { line, message } => {
                write!(f, "mu parity certificate parse error at line {line}: {message}")
            }
            Self::Missing { directive } => {
                write!(f, "mu parity certificate is missing '{directive}'")
            }
            Self::UnsupportedVersion { version } => write!(
                f,
                "unsupported mu parity certificate schema version {version}; expected {}",
                DECLARATIVE_MU_PARITY_CERTIFICATE_SCHEMA_VERSION
            ),
            Self::CountMismatch {
                section,
                expected,
                actual,
            } => write!(
                f,
                "mu parity certificate {section} count mismatch: expected {expected}, parsed {actual}"
            ),
        }
    }
}

impl std::error::Error for MuParityCertificateParseError {}

#[derive(Debug)]
pub enum DeclarativeMuParityCertificateError {
    Frontend(DeclarativeMuError),
    FormulaParse(MuParseError),
    ModelBindingMismatch,
    FormulaBindingMismatch {
        certificate: String,
        requested: String,
    },
    CanonicalGraphCapture,
    MalformedEvidence {
        message: String,
    },
    Evidence(MuParityEvidenceError<String>),
}

impl fmt::Display for DeclarativeMuParityCertificateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frontend(error) => write!(f, "{error}"),
            Self::FormulaParse(error) => write!(f, "{error}"),
            Self::ModelBindingMismatch => write!(
                f,
                "mu parity certificate is bound to a different declarative model"
            ),
            Self::FormulaBindingMismatch {
                certificate,
                requested,
            } => write!(
                f,
                "mu parity certificate formula binding mismatch: certificate={certificate:?}, requested={requested:?}"
            ),
            Self::CanonicalGraphCapture => {
                write!(f, "mu parity certificate canonical graph capture failed")
            }
            Self::MalformedEvidence { message } => {
                write!(f, "mu parity certificate evidence is malformed: {message}")
            }
            Self::Evidence(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DeclarativeMuParityCertificateError {}

impl From<DeclarativeMuError> for DeclarativeMuParityCertificateError {
    fn from(value: DeclarativeMuError) -> Self {
        Self::Frontend(value)
    }
}

impl From<MuParseError> for DeclarativeMuParityCertificateError {
    fn from(value: MuParseError) -> Self {
        Self::FormulaParse(value)
    }
}

pub fn create_declarative_mu_parity_certificate(
    document: &DeclarativeDocument,
    expression: &str,
) -> Result<DeclarativeMuParityCertificate, DeclarativeMuParityCertificateError> {
    let result = check_declarative_mu_text_via_parity(document, expression)?;
    let evaluation = &result.evaluation;
    let node_count = formula_node_count(evaluation)?;

    Ok(DeclarativeMuParityCertificate {
        schema_version: DECLARATIVE_MU_PARITY_CERTIFICATE_SCHEMA_VERSION,
        model_binding: document.canonical_identity().to_owned(),
        formula: result.formula,
        discovered_states: evaluation.discovered_states,
        explored_transitions: evaluation.explored_transitions,
        max_depth_reached: evaluation.max_depth_reached,
        parity_game_vertices: evaluation.parity_game_vertices,
        max_priority: evaluation.max_priority,
        satisfying_state_indices: evaluation.satisfying_state_indices.clone(),
        positions: evaluation.positions.clone(),
        even_winning_vertices: encode_winning_vertices(
            evaluation,
            &evaluation.even_strategy,
            node_count,
        )?,
        odd_winning_vertices: encode_winning_vertices(
            evaluation,
            &evaluation.odd_strategy,
            node_count,
        )?,
        even_choices: encode_choices(evaluation, &evaluation.even_strategy, node_count)?,
        odd_choices: encode_choices(evaluation, &evaluation.odd_strategy, node_count)?,
        initial: evaluation
            .initial_evidence
            .iter()
            .map(|entry| {
                Ok(MuParityCertificateInitial {
                    state_index: entry.state_index,
                    satisfied: entry.satisfied,
                    winner: entry.winner,
                    root_vertex: canonical_vertex(evaluation, &entry.root_position, node_count)?,
                })
            })
            .collect::<Result<Vec<_>, DeclarativeMuParityCertificateError>>()?,
    })
}

pub fn verify_declarative_mu_parity_certificate(
    document: &DeclarativeDocument,
    expression: &str,
    certificate: &DeclarativeMuParityCertificate,
) -> Result<(), DeclarativeMuParityCertificateError> {
    if certificate.schema_version != DECLARATIVE_MU_PARITY_CERTIFICATE_SCHEMA_VERSION {
        return Err(DeclarativeMuParityCertificateError::MalformedEvidence {
            message: format!(
                "schema version {} is not supported",
                certificate.schema_version
            ),
        });
    }
    if certificate.model_binding != document.canonical_identity() {
        return Err(DeclarativeMuParityCertificateError::ModelBindingMismatch);
    }

    let formula = parse_mu_formula(expression)?;
    validate_declarative_mu_formula(document, &formula)?;
    let requested_formula = render_mu_formula(&formula);
    if certificate.formula != requested_formula {
        return Err(
            DeclarativeMuParityCertificateError::FormulaBindingMismatch {
                certificate: certificate.formula.clone(),
                requested: requested_formula,
            },
        );
    }

    let captured = capture_reachable_graph(document.model())
        .map_err(|_| DeclarativeMuParityCertificateError::CanonicalGraphCapture)?;
    if certificate.parity_game_vertices != certificate.positions.len() {
        return Err(DeclarativeMuParityCertificateError::MalformedEvidence {
            message: format!(
                "position count {} does not match parity_game_vertices {}",
                certificate.positions.len(),
                certificate.parity_game_vertices
            ),
        });
    }

    let even_strategy = decode_strategy(
        ParityPlayer::Even,
        &certificate.positions,
        &certificate.even_winning_vertices,
        &certificate.even_choices,
    )?;
    let odd_strategy = decode_strategy(
        ParityPlayer::Odd,
        &certificate.positions,
        &certificate.odd_winning_vertices,
        &certificate.odd_choices,
    )?;

    let mut initial = Vec::with_capacity(certificate.initial.len());
    let mut initial_evidence = Vec::with_capacity(certificate.initial.len());
    for entry in &certificate.initial {
        let state = captured
            .graph
            .states
            .get(entry.state_index)
            .cloned()
            .ok_or_else(|| DeclarativeMuParityCertificateError::MalformedEvidence {
                message: format!("initial state index {} is out of range", entry.state_index),
            })?;
        let root_position = certificate
            .positions
            .get(entry.root_vertex)
            .cloned()
            .ok_or_else(|| DeclarativeMuParityCertificateError::MalformedEvidence {
                message: format!("initial root vertex {} is out of range", entry.root_vertex),
            })?;
        initial.push(MuInitialEvaluation {
            state_index: entry.state_index,
            state,
            satisfied: entry.satisfied,
        });
        initial_evidence.push(MuParityInitialEvidence {
            state_index: entry.state_index,
            satisfied: entry.satisfied,
            winner: entry.winner,
            root_position,
        });
    }

    let evaluation = MuParityEvaluation {
        terminal_policy: MuTerminalPolicy::TotalizeWithSelfLoop,
        reachable_states: captured.graph.states.clone(),
        satisfying_state_indices: certificate.satisfying_state_indices.clone(),
        initial,
        discovered_states: certificate.discovered_states,
        explored_transitions: certificate.explored_transitions,
        max_depth_reached: certificate.max_depth_reached,
        parity_game_vertices: certificate.parity_game_vertices,
        max_priority: certificate.max_priority,
        positions: certificate.positions.clone(),
        even_strategy,
        odd_strategy,
        initial_evidence,
    };

    verify_mu_parity_evidence(
        document.model(),
        &formula,
        |atom, state| document.state_has_proposition(state, atom),
        &evaluation,
    )
    .map_err(DeclarativeMuParityCertificateError::Evidence)
}

pub fn render_declarative_mu_parity_certificate(
    certificate: &DeclarativeMuParityCertificate,
) -> String {
    let mut output = String::new();
    writeln!(
        &mut output,
        "fvlab-mu-parity-certificate {}",
        certificate.schema_version
    )
    .expect("writing to String cannot fail");
    write_quoted_directive(&mut output, "model-binding", &certificate.model_binding);
    write_quoted_directive(&mut output, "formula", &certificate.formula);
    writeln!(
        &mut output,
        "accounting {} {} {} {} {}",
        certificate.discovered_states,
        certificate.explored_transitions,
        certificate
            .max_depth_reached
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned()),
        certificate.parity_game_vertices,
        certificate.max_priority
    )
    .expect("writing to String cannot fail");
    write_index_list(
        &mut output,
        "satisfying",
        &certificate.satisfying_state_indices,
    );
    writeln!(&mut output, "positions {}", certificate.positions.len())
        .expect("writing to String cannot fail");
    for (vertex, position) in certificate.positions.iter().enumerate() {
        writeln!(
            &mut output,
            "position {vertex} {} {} {} {} {}",
            position.state_index,
            position.formula_node,
            render_position_kind(&position.kind),
            render_player(position.owner),
            position.priority
        )
        .expect("writing to String cannot fail");
    }
    write_winning(
        &mut output,
        ParityPlayer::Even,
        &certificate.even_winning_vertices,
    );
    write_winning(
        &mut output,
        ParityPlayer::Odd,
        &certificate.odd_winning_vertices,
    );
    write_choices(&mut output, ParityPlayer::Even, &certificate.even_choices);
    write_choices(&mut output, ParityPlayer::Odd, &certificate.odd_choices);
    writeln!(&mut output, "initials {}", certificate.initial.len())
        .expect("writing to String cannot fail");
    for entry in &certificate.initial {
        writeln!(
            &mut output,
            "initial {} {} {} {}",
            entry.state_index,
            if entry.satisfied { "true" } else { "false" },
            render_player(entry.winner),
            entry.root_vertex
        )
        .expect("writing to String cannot fail");
    }
    output.push_str("end\n");
    output
}

pub fn parse_declarative_mu_parity_certificate(
    input: &str,
) -> Result<DeclarativeMuParityCertificate, MuParityCertificateParseError> {
    let mut schema_version = None;
    let mut model_binding = None;
    let mut formula = None;
    let mut accounting = None;
    let mut satisfying = None;
    let mut expected_positions = None;
    let mut positions: Vec<Option<MuParityPosition>> = Vec::new();
    let mut winning_even = None;
    let mut winning_odd = None;
    let mut expected_even_choices = None;
    let mut expected_odd_choices = None;
    let mut even_choices = Vec::new();
    let mut odd_choices = Vec::new();
    let mut expected_initials = None;
    let mut initial = Vec::new();
    let mut saw_end = false;

    for (offset, raw) in input.lines().enumerate() {
        let line = offset + 1;
        let text = raw.trim();
        if text.is_empty() || text.starts_with('#') {
            continue;
        }
        if saw_end {
            return line_error(line, "content appears after end marker");
        }

        if text.starts_with("model-binding") {
            ensure_none(&model_binding, line, "model-binding")?;
            model_binding = Some(parse_quoted_directive(text, "model-binding", line)?);
            continue;
        }
        if text.starts_with("formula") {
            ensure_none(&formula, line, "formula")?;
            formula = Some(parse_quoted_directive(text, "formula", line)?);
            continue;
        }

        let tokens = text.split_whitespace().collect::<Vec<_>>();
        let directive = tokens[0];
        match directive {
            "fvlab-mu-parity-certificate" => {
                ensure_none(&schema_version, line, directive)?;
                require_arity(&tokens, 2, line)?;
                let version = parse_u32(tokens[1], line, "schema version")?;
                if version != DECLARATIVE_MU_PARITY_CERTIFICATE_SCHEMA_VERSION {
                    return Err(MuParityCertificateParseError::UnsupportedVersion { version });
                }
                schema_version = Some(version);
            }
            "accounting" => {
                ensure_none(&accounting, line, directive)?;
                require_arity(&tokens, 6, line)?;
                accounting = Some((
                    parse_usize(tokens[1], line, "discovered states")?,
                    parse_usize(tokens[2], line, "explored transitions")?,
                    parse_optional_usize(tokens[3], line, "max depth")?,
                    parse_usize(tokens[4], line, "parity game vertices")?,
                    parse_usize(tokens[5], line, "max priority")?,
                ));
            }
            "satisfying" => {
                ensure_none(&satisfying, line, directive)?;
                satisfying = Some(parse_counted_indices(&tokens, line, "satisfying")?);
            }
            "positions" => {
                ensure_none(&expected_positions, line, directive)?;
                require_arity(&tokens, 2, line)?;
                let count = parse_usize(tokens[1], line, "position count")?;
                expected_positions = Some(count);
                positions.resize(count, None);
            }
            "position" => {
                require_arity(&tokens, 7, line)?;
                let expected =
                    expected_positions.ok_or_else(|| MuParityCertificateParseError::Line {
                        line,
                        message: "position appears before positions count".to_owned(),
                    })?;
                let vertex = parse_usize(tokens[1], line, "position vertex")?;
                if vertex >= expected {
                    return line_error(line, format!("position vertex {vertex} is out of range"));
                }
                if positions[vertex].is_some() {
                    return line_error(line, format!("duplicate position vertex {vertex}"));
                }
                positions[vertex] = Some(MuParityPosition {
                    state_index: parse_usize(tokens[2], line, "state index")?,
                    formula_node: parse_usize(tokens[3], line, "formula node")?,
                    kind: parse_position_kind(tokens[4], line)?,
                    owner: parse_player(tokens[5], line)?,
                    priority: parse_usize(tokens[6], line, "priority")?,
                });
            }
            "winning" => {
                if tokens.len() < 3 {
                    return line_error(line, "winning directive requires player and count");
                }
                let player = parse_player(tokens[1], line)?;
                let parsed = parse_counted_tail(&tokens, 2, line, "winning")?;
                match player {
                    ParityPlayer::Even => {
                        ensure_none(&winning_even, line, "winning even")?;
                        winning_even = Some(parsed);
                    }
                    ParityPlayer::Odd => {
                        ensure_none(&winning_odd, line, "winning odd")?;
                        winning_odd = Some(parsed);
                    }
                }
            }
            "choices" => {
                require_arity(&tokens, 3, line)?;
                let player = parse_player(tokens[1], line)?;
                let count = parse_usize(tokens[2], line, "choice count")?;
                match player {
                    ParityPlayer::Even => {
                        ensure_none(&expected_even_choices, line, "choices even")?;
                        expected_even_choices = Some(count);
                    }
                    ParityPlayer::Odd => {
                        ensure_none(&expected_odd_choices, line, "choices odd")?;
                        expected_odd_choices = Some(count);
                    }
                }
            }
            "choice" => {
                require_arity(&tokens, 5, line)?;
                let player = parse_player(tokens[1], line)?;
                let choice = MuParityCertificateChoice {
                    from_vertex: parse_usize(tokens[2], line, "choice source")?,
                    to_vertex: parse_usize(tokens[3], line, "choice target")?,
                    semantic_move: parse_move(tokens[4], line)?,
                };
                match player {
                    ParityPlayer::Even => even_choices.push(choice),
                    ParityPlayer::Odd => odd_choices.push(choice),
                }
            }
            "initials" => {
                ensure_none(&expected_initials, line, directive)?;
                require_arity(&tokens, 2, line)?;
                expected_initials = Some(parse_usize(tokens[1], line, "initial count")?);
            }
            "initial" => {
                require_arity(&tokens, 5, line)?;
                initial.push(MuParityCertificateInitial {
                    state_index: parse_usize(tokens[1], line, "initial state")?,
                    satisfied: parse_bool(tokens[2], line)?,
                    winner: parse_player(tokens[3], line)?,
                    root_vertex: parse_usize(tokens[4], line, "initial root vertex")?,
                });
            }
            "end" => {
                require_arity(&tokens, 1, line)?;
                saw_end = true;
            }
            other => return line_error(line, format!("unknown directive '{other}'")),
        }
    }

    if !saw_end {
        return Err(MuParityCertificateParseError::Missing { directive: "end" });
    }
    let schema_version = schema_version.ok_or(MuParityCertificateParseError::Missing {
        directive: "fvlab-mu-parity-certificate",
    })?;
    let model_binding = model_binding.ok_or(MuParityCertificateParseError::Missing {
        directive: "model-binding",
    })?;
    let formula = formula.ok_or(MuParityCertificateParseError::Missing {
        directive: "formula",
    })?;
    let (
        discovered_states,
        explored_transitions,
        max_depth_reached,
        parity_game_vertices,
        max_priority,
    ) = accounting.ok_or(MuParityCertificateParseError::Missing {
        directive: "accounting",
    })?;
    let satisfying_state_indices = satisfying.ok_or(MuParityCertificateParseError::Missing {
        directive: "satisfying",
    })?;
    let expected_positions = expected_positions.ok_or(MuParityCertificateParseError::Missing {
        directive: "positions",
    })?;
    let actual_positions = positions.iter().filter(|entry| entry.is_some()).count();
    if actual_positions != expected_positions {
        return Err(MuParityCertificateParseError::CountMismatch {
            section: "positions",
            expected: expected_positions,
            actual: actual_positions,
        });
    }
    let positions = positions
        .into_iter()
        .map(|entry| entry.expect("position count was validated"))
        .collect();

    let even_winning_vertices = winning_even.ok_or(MuParityCertificateParseError::Missing {
        directive: "winning even",
    })?;
    let odd_winning_vertices = winning_odd.ok_or(MuParityCertificateParseError::Missing {
        directive: "winning odd",
    })?;
    validate_count(
        "even choices",
        expected_even_choices.ok_or(MuParityCertificateParseError::Missing {
            directive: "choices even",
        })?,
        even_choices.len(),
    )?;
    validate_count(
        "odd choices",
        expected_odd_choices.ok_or(MuParityCertificateParseError::Missing {
            directive: "choices odd",
        })?,
        odd_choices.len(),
    )?;
    validate_count(
        "initials",
        expected_initials.ok_or(MuParityCertificateParseError::Missing {
            directive: "initials",
        })?,
        initial.len(),
    )?;

    Ok(DeclarativeMuParityCertificate {
        schema_version,
        model_binding,
        formula,
        discovered_states,
        explored_transitions,
        max_depth_reached,
        parity_game_vertices,
        max_priority,
        satisfying_state_indices,
        positions,
        even_winning_vertices,
        odd_winning_vertices,
        even_choices,
        odd_choices,
        initial,
    })
}

fn formula_node_count(
    evaluation: &MuParityEvaluation<String>,
) -> Result<usize, DeclarativeMuParityCertificateError> {
    if evaluation.reachable_states.is_empty() {
        return Err(DeclarativeMuParityCertificateError::MalformedEvidence {
            message: "reachable state set is empty".to_owned(),
        });
    }
    if !evaluation
        .positions
        .len()
        .is_multiple_of(evaluation.reachable_states.len())
    {
        return Err(DeclarativeMuParityCertificateError::MalformedEvidence {
            message: "position count is not divisible by reachable state count".to_owned(),
        });
    }
    Ok(evaluation.positions.len() / evaluation.reachable_states.len())
}

fn canonical_vertex(
    evaluation: &MuParityEvaluation<String>,
    position: &MuParityPosition,
    node_count: usize,
) -> Result<usize, DeclarativeMuParityCertificateError> {
    let vertex = position
        .state_index
        .checked_mul(node_count)
        .and_then(|base| base.checked_add(position.formula_node))
        .ok_or_else(|| DeclarativeMuParityCertificateError::MalformedEvidence {
            message: "position index overflow".to_owned(),
        })?;
    if evaluation.positions.get(vertex) != Some(position) {
        return Err(DeclarativeMuParityCertificateError::MalformedEvidence {
            message: format!("typed position does not match canonical vertex {vertex}"),
        });
    }
    Ok(vertex)
}

fn encode_winning_vertices(
    evaluation: &MuParityEvaluation<String>,
    strategy: &MuParityStrategyEvidence,
    node_count: usize,
) -> Result<Vec<usize>, DeclarativeMuParityCertificateError> {
    strategy
        .winning_positions
        .iter()
        .map(|position| canonical_vertex(evaluation, position, node_count))
        .collect()
}

fn encode_choices(
    evaluation: &MuParityEvaluation<String>,
    strategy: &MuParityStrategyEvidence,
    node_count: usize,
) -> Result<Vec<MuParityCertificateChoice>, DeclarativeMuParityCertificateError> {
    strategy
        .choices
        .iter()
        .map(|choice| {
            Ok(MuParityCertificateChoice {
                from_vertex: canonical_vertex(evaluation, &choice.from, node_count)?,
                to_vertex: canonical_vertex(evaluation, &choice.to, node_count)?,
                semantic_move: choice.semantic_move.clone(),
            })
        })
        .collect()
}

fn decode_strategy(
    player: ParityPlayer,
    positions: &[MuParityPosition],
    winning_vertices: &[usize],
    choices: &[MuParityCertificateChoice],
) -> Result<MuParityStrategyEvidence, DeclarativeMuParityCertificateError> {
    let winning_positions = winning_vertices
        .iter()
        .map(|&vertex| {
            positions.get(vertex).cloned().ok_or_else(|| {
                DeclarativeMuParityCertificateError::MalformedEvidence {
                    message: format!("{player:?} winning vertex {vertex} is out of range"),
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let choices = choices
        .iter()
        .map(|choice| {
            let from = positions.get(choice.from_vertex).cloned().ok_or_else(|| {
                DeclarativeMuParityCertificateError::MalformedEvidence {
                    message: format!(
                        "{player:?} choice source {} is out of range",
                        choice.from_vertex
                    ),
                }
            })?;
            let to = positions.get(choice.to_vertex).cloned().ok_or_else(|| {
                DeclarativeMuParityCertificateError::MalformedEvidence {
                    message: format!(
                        "{player:?} choice target {} is out of range",
                        choice.to_vertex
                    ),
                }
            })?;
            Ok(MuParityStrategyChoice {
                from,
                to,
                semantic_move: choice.semantic_move.clone(),
            })
        })
        .collect::<Result<Vec<_>, DeclarativeMuParityCertificateError>>()?;

    Ok(MuParityStrategyEvidence {
        player,
        winning_positions,
        choices,
    })
}

fn render_position_kind(kind: &MuParityPositionKind) -> String {
    match kind {
        MuParityPositionKind::True => "true".to_owned(),
        MuParityPositionKind::False => "false".to_owned(),
        MuParityPositionKind::Literal { positive: true } => "literal-positive".to_owned(),
        MuParityPositionKind::Literal { positive: false } => "literal-negative".to_owned(),
        MuParityPositionKind::Variable {
            binder_formula_node,
        } => format!("variable:{binder_formula_node}"),
        MuParityPositionKind::And => "and".to_owned(),
        MuParityPositionKind::Or => "or".to_owned(),
        MuParityPositionKind::Diamond => "diamond".to_owned(),
        MuParityPositionKind::Box => "box".to_owned(),
        MuParityPositionKind::Fixpoint {
            kind: MuParityFixpointKind::Mu,
            alternation_level,
        } => format!("fixpoint-mu:{alternation_level}"),
        MuParityPositionKind::Fixpoint {
            kind: MuParityFixpointKind::Nu,
            alternation_level,
        } => format!("fixpoint-nu:{alternation_level}"),
    }
}

fn parse_position_kind(
    token: &str,
    line: usize,
) -> Result<MuParityPositionKind, MuParityCertificateParseError> {
    Ok(match token {
        "true" => MuParityPositionKind::True,
        "false" => MuParityPositionKind::False,
        "literal-positive" => MuParityPositionKind::Literal { positive: true },
        "literal-negative" => MuParityPositionKind::Literal { positive: false },
        "and" => MuParityPositionKind::And,
        "or" => MuParityPositionKind::Or,
        "diamond" => MuParityPositionKind::Diamond,
        "box" => MuParityPositionKind::Box,
        _ if token.starts_with("variable:") => MuParityPositionKind::Variable {
            binder_formula_node: parse_usize(&token["variable:".len()..], line, "variable binder")?,
        },
        _ if token.starts_with("fixpoint-mu:") => MuParityPositionKind::Fixpoint {
            kind: MuParityFixpointKind::Mu,
            alternation_level: parse_usize(
                &token["fixpoint-mu:".len()..],
                line,
                "mu alternation level",
            )?,
        },
        _ if token.starts_with("fixpoint-nu:") => MuParityPositionKind::Fixpoint {
            kind: MuParityFixpointKind::Nu,
            alternation_level: parse_usize(
                &token["fixpoint-nu:".len()..],
                line,
                "nu alternation level",
            )?,
        },
        _ => return line_error(line, format!("invalid position kind '{token}'")),
    })
}

fn render_move(value: &MuParityMove) -> String {
    match value {
        MuParityMove::OutcomeSelfLoop => "outcome-self-loop".to_owned(),
        MuParityMove::BooleanLeft => "boolean-left".to_owned(),
        MuParityMove::BooleanRight => "boolean-right".to_owned(),
        MuParityMove::ModalSuccessor { target_state_index } => {
            format!("modal-successor:{target_state_index}")
        }
        MuParityMove::ModalTerminalSelfLoop => "modal-terminal-self-loop".to_owned(),
        MuParityMove::FixpointBody => "fixpoint-body".to_owned(),
        MuParityMove::VariableReturn {
            binder_formula_node,
        } => format!("variable-return:{binder_formula_node}"),
    }
}

fn parse_move(token: &str, line: usize) -> Result<MuParityMove, MuParityCertificateParseError> {
    Ok(match token {
        "outcome-self-loop" => MuParityMove::OutcomeSelfLoop,
        "boolean-left" => MuParityMove::BooleanLeft,
        "boolean-right" => MuParityMove::BooleanRight,
        "modal-terminal-self-loop" => MuParityMove::ModalTerminalSelfLoop,
        "fixpoint-body" => MuParityMove::FixpointBody,
        _ if token.starts_with("modal-successor:") => MuParityMove::ModalSuccessor {
            target_state_index: parse_usize(
                &token["modal-successor:".len()..],
                line,
                "modal successor state",
            )?,
        },
        _ if token.starts_with("variable-return:") => MuParityMove::VariableReturn {
            binder_formula_node: parse_usize(
                &token["variable-return:".len()..],
                line,
                "variable return binder",
            )?,
        },
        _ => return line_error(line, format!("invalid semantic move '{token}'")),
    })
}

fn render_player(player: ParityPlayer) -> &'static str {
    match player {
        ParityPlayer::Even => "even",
        ParityPlayer::Odd => "odd",
    }
}

fn parse_player(token: &str, line: usize) -> Result<ParityPlayer, MuParityCertificateParseError> {
    match token {
        "even" => Ok(ParityPlayer::Even),
        "odd" => Ok(ParityPlayer::Odd),
        _ => line_error(line, format!("invalid parity player '{token}'")),
    }
}

fn write_quoted_directive(output: &mut String, directive: &str, value: &str) {
    output.push_str(directive);
    output.push(' ');
    output.push('"');
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
    output.push_str("\"\n");
}

fn parse_quoted_directive(
    text: &str,
    directive: &str,
    line: usize,
) -> Result<String, MuParityCertificateParseError> {
    let rest = text
        .strip_prefix(directive)
        .ok_or_else(|| MuParityCertificateParseError::Line {
            line,
            message: format!("expected directive '{directive}'"),
        })?
        .trim_start();
    if !rest.starts_with('"') {
        return line_error(
            line,
            format!("directive '{directive}' expects one quoted string"),
        );
    }

    let mut chars = rest.char_indices();
    chars.next();
    let mut output = String::new();
    let mut escaped = false;
    let mut closing = None;
    for (offset, ch) in chars {
        if escaped {
            match ch {
                '\\' => output.push('\\'),
                '"' => output.push('"'),
                'n' => output.push('\n'),
                'r' => output.push('\r'),
                't' => output.push('\t'),
                _ => return line_error(line, format!("invalid string escape '\\{ch}'")),
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                closing = Some(offset + ch.len_utf8());
                break;
            }
            other => output.push(other),
        }
    }
    if escaped {
        return line_error(line, "trailing string escape");
    }
    let closing = closing.ok_or_else(|| MuParityCertificateParseError::Line {
        line,
        message: "unterminated quoted string".to_owned(),
    })?;
    if !rest[closing..].trim().is_empty() {
        return line_error(line, "unexpected trailing content after quoted string");
    }
    Ok(output)
}

fn write_index_list(output: &mut String, directive: &str, values: &[usize]) {
    write!(output, "{directive} {}", values.len()).expect("writing to String cannot fail");
    for value in values {
        write!(output, " {value}").expect("writing to String cannot fail");
    }
    output.push('\n');
}

fn write_winning(output: &mut String, player: ParityPlayer, values: &[usize]) {
    write!(output, "winning {} {}", render_player(player), values.len())
        .expect("writing to String cannot fail");
    for value in values {
        write!(output, " {value}").expect("writing to String cannot fail");
    }
    output.push('\n');
}

fn write_choices(output: &mut String, player: ParityPlayer, choices: &[MuParityCertificateChoice]) {
    writeln!(
        output,
        "choices {} {}",
        render_player(player),
        choices.len()
    )
    .expect("writing to String cannot fail");
    for choice in choices {
        writeln!(
            output,
            "choice {} {} {} {}",
            render_player(player),
            choice.from_vertex,
            choice.to_vertex,
            render_move(&choice.semantic_move)
        )
        .expect("writing to String cannot fail");
    }
}

fn parse_counted_indices(
    tokens: &[&str],
    line: usize,
    section: &'static str,
) -> Result<Vec<usize>, MuParityCertificateParseError> {
    parse_counted_tail(tokens, 1, line, section)
}

fn parse_counted_tail(
    tokens: &[&str],
    count_index: usize,
    line: usize,
    section: &'static str,
) -> Result<Vec<usize>, MuParityCertificateParseError> {
    if tokens.len() <= count_index {
        return line_error(line, format!("{section} directive is missing a count"));
    }
    let expected = parse_usize(tokens[count_index], line, "count")?;
    let values = tokens[count_index + 1..]
        .iter()
        .map(|value| parse_usize(value, line, section))
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != expected {
        return Err(MuParityCertificateParseError::CountMismatch {
            section,
            expected,
            actual: values.len(),
        });
    }
    Ok(values)
}

fn parse_bool(token: &str, line: usize) -> Result<bool, MuParityCertificateParseError> {
    match token {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => line_error(line, format!("invalid boolean '{token}'")),
    }
}

fn parse_usize(
    token: &str,
    line: usize,
    field: &str,
) -> Result<usize, MuParityCertificateParseError> {
    token
        .parse::<usize>()
        .map_err(|_| MuParityCertificateParseError::Line {
            line,
            message: format!("{field} must be a non-negative integer"),
        })
}

fn parse_u32(token: &str, line: usize, field: &str) -> Result<u32, MuParityCertificateParseError> {
    token
        .parse::<u32>()
        .map_err(|_| MuParityCertificateParseError::Line {
            line,
            message: format!("{field} must be a non-negative integer"),
        })
}

fn parse_optional_usize(
    token: &str,
    line: usize,
    field: &str,
) -> Result<Option<usize>, MuParityCertificateParseError> {
    if token == "none" {
        Ok(None)
    } else {
        parse_usize(token, line, field).map(Some)
    }
}

fn require_arity(
    tokens: &[&str],
    expected: usize,
    line: usize,
) -> Result<(), MuParityCertificateParseError> {
    if tokens.len() != expected {
        return line_error(
            line,
            format!(
                "directive '{}' expects {} tokens but received {}",
                tokens[0],
                expected,
                tokens.len()
            ),
        );
    }
    Ok(())
}

fn ensure_none<T>(
    value: &Option<T>,
    line: usize,
    directive: &str,
) -> Result<(), MuParityCertificateParseError> {
    if value.is_some() {
        return line_error(line, format!("duplicate directive '{directive}'"));
    }
    Ok(())
}

fn validate_count(
    section: &'static str,
    expected: usize,
    actual: usize,
) -> Result<(), MuParityCertificateParseError> {
    if expected != actual {
        return Err(MuParityCertificateParseError::CountMismatch {
            section,
            expected,
            actual,
        });
    }
    Ok(())
}

fn line_error<T>(
    line: usize,
    message: impl Into<String>,
) -> Result<T, MuParityCertificateParseError> {
    Err(MuParityCertificateParseError::Line {
        line,
        message: message.into(),
    })
}

use crate::checker::ExplorationLimits;
use crate::ctl::{evaluate_ctl, CtlError, CtlEvaluation, CtlFormula};
use crate::ctl_bounded::{
    evaluate_ctl_with_limits, BoundedCtlError, BoundedCtlEvaluation,
};
use crate::ctl_parse::{collect_ctl_atoms, parse_ctl_formula, render_ctl_formula, CtlParseError};
use crate::declarative::DeclarativeDocument;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarativeCtlStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeCtlResult {
    pub formula: String,
    pub status: DeclarativeCtlStatus,
    pub evaluation: CtlEvaluation<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedDeclarativeCtlResult {
    pub formula: String,
    pub evaluation: BoundedCtlEvaluation<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarativeCtlError {
    Parse(CtlParseError),
    UnknownProposition { proposition: String },
    Backend(CtlError),
    BoundedBackend(BoundedCtlError),
}

impl fmt::Display for DeclarativeCtlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::UnknownProposition { proposition } => {
                write!(f, "unknown CTL proposition '{proposition}'")
            }
            Self::Backend(error) => write!(f, "CTL backend failed: {error}"),
            Self::BoundedBackend(error) => write!(f, "bounded CTL backend failed: {error}"),
        }
    }
}

impl std::error::Error for DeclarativeCtlError {}

impl From<CtlParseError> for DeclarativeCtlError {
    fn from(value: CtlParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<CtlError> for DeclarativeCtlError {
    fn from(value: CtlError) -> Self {
        Self::Backend(value)
    }
}

impl From<BoundedCtlError> for DeclarativeCtlError {
    fn from(value: BoundedCtlError) -> Self {
        Self::BoundedBackend(value)
    }
}

/// Evaluate one typed CTL formula against the existing declarative proposition
/// metadata.
///
/// All atom references are resolved before the M69 backend captures or explores
/// the model. The semantic authority remains `evaluate_ctl`; this adapter only
/// binds named propositions to the already-parsed declarative document and
/// defines whole-query success as all initial states satisfying the formula.
pub fn check_declarative_ctl(
    document: &DeclarativeDocument,
    formula: &CtlFormula<String>,
) -> Result<DeclarativeCtlResult, DeclarativeCtlError> {
    validate_atoms(document, formula)?;

    let evaluation = evaluate_ctl(document.model(), formula, |atom, state| {
        document.state_has_proposition(state, atom)
    })?;
    let status = if evaluation.all_initial_states_satisfy() {
        DeclarativeCtlStatus::Satisfied
    } else {
        DeclarativeCtlStatus::Violated
    };

    Ok(DeclarativeCtlResult {
        formula: render_ctl_formula(formula),
        status,
        evaluation,
    })
}

/// Parse and evaluate one textual CTL formula through the M69 authority.
pub fn check_declarative_ctl_text(
    document: &DeclarativeDocument,
    input: &str,
) -> Result<DeclarativeCtlResult, DeclarativeCtlError> {
    let formula = parse_ctl_formula(input)?;
    check_declarative_ctl(document, &formula)
}


/// Evaluate one typed CTL formula through the proof-honest M71 bounded
/// authority after resolving every named proposition against the declarative
/// document.
pub fn check_declarative_ctl_with_limits(
    document: &DeclarativeDocument,
    formula: &CtlFormula<String>,
    limits: ExplorationLimits,
) -> Result<BoundedDeclarativeCtlResult, DeclarativeCtlError> {
    validate_atoms(document, formula)?;

    let evaluation = evaluate_ctl_with_limits(
        document.model(),
        formula,
        |atom, state| document.state_has_proposition(state, atom),
        limits,
    )?;

    Ok(BoundedDeclarativeCtlResult {
        formula: render_ctl_formula(formula),
        evaluation,
    })
}

/// Parse and evaluate one textual CTL formula through the bounded M71
/// authority.
pub fn check_declarative_ctl_text_with_limits(
    document: &DeclarativeDocument,
    input: &str,
    limits: ExplorationLimits,
) -> Result<BoundedDeclarativeCtlResult, DeclarativeCtlError> {
    let formula = parse_ctl_formula(input)?;
    check_declarative_ctl_with_limits(document, &formula, limits)
}

fn validate_atoms(
    document: &DeclarativeDocument,
    formula: &CtlFormula<String>,
) -> Result<(), DeclarativeCtlError> {
    let mut atoms = Vec::new();
    collect_ctl_atoms(formula, &mut atoms);
    let mut seen = HashSet::new();

    for atom in atoms {
        if !seen.insert(atom) {
            continue;
        }
        if document.proposition_states(atom).is_none() {
            return Err(DeclarativeCtlError::UnknownProposition {
                proposition: atom.to_owned(),
            });
        }
    }

    Ok(())
}

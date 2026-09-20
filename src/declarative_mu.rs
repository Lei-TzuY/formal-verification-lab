use crate::checker::ExplorationLimits;
use crate::declarative::DeclarativeDocument;
use crate::mu_bounded::{evaluate_mu_with_limits, BoundedMuError, BoundedMuEvaluation};
use crate::mu_calculus::{
    evaluate_mu, validate_mu_formula, MuError, MuEvaluation, MuFormula, MuValidationError,
};
use crate::mu_parity::{evaluate_mu_via_parity, MuParityError, MuParityEvaluation};
use crate::mu_parse::{collect_mu_atoms, parse_mu_formula, render_mu_formula, MuParseError};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclarativeMuStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMuResult {
    pub formula: String,
    pub status: DeclarativeMuStatus,
    pub evaluation: MuEvaluation<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedDeclarativeMuResult {
    pub formula: String,
    pub evaluation: BoundedMuEvaluation<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarativeMuParityResult {
    pub formula: String,
    pub status: DeclarativeMuStatus,
    pub evaluation: MuParityEvaluation<String>,
}

#[derive(Debug)]
pub enum DeclarativeMuError {
    Parse(MuParseError),
    Validation(MuValidationError<String>),
    UnknownProposition { proposition: String },
    Backend(MuError<String>),
    BoundedBackend(BoundedMuError<String>),
    ParityBackend(MuParityError<String>),
}

impl fmt::Display for DeclarativeMuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "{error}"),
            Self::Validation(error) => write!(f, "{error}"),
            Self::UnknownProposition { proposition } => {
                write!(f, "unknown mu-calculus proposition '{proposition}'")
            }
            Self::Backend(error) => write!(f, "mu-calculus backend failed: {error}"),
            Self::BoundedBackend(error) => {
                write!(f, "bounded mu-calculus backend failed: {error}")
            }
            Self::ParityBackend(error) => {
                write!(f, "mu-calculus parity backend failed: {error}")
            }
        }
    }
}

impl std::error::Error for DeclarativeMuError {}

impl From<MuParseError> for DeclarativeMuError {
    fn from(value: MuParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<MuValidationError<String>> for DeclarativeMuError {
    fn from(value: MuValidationError<String>) -> Self {
        Self::Validation(value)
    }
}

impl From<MuError<String>> for DeclarativeMuError {
    fn from(value: MuError<String>) -> Self {
        Self::Backend(value)
    }
}

impl From<BoundedMuError<String>> for DeclarativeMuError {
    fn from(value: BoundedMuError<String>) -> Self {
        Self::BoundedBackend(value)
    }
}

impl From<MuParityError<String>> for DeclarativeMuError {
    fn from(value: MuParityError<String>) -> Self {
        Self::ParityBackend(value)
    }
}

/// Validate a typed textual-surface formula, resolve all named propositions,
/// then delegate execution exclusively to the M75 modal mu-calculus authority.
pub fn check_declarative_mu(
    document: &DeclarativeDocument,
    formula: &MuFormula<String, String>,
) -> Result<DeclarativeMuResult, DeclarativeMuError> {
    validate_declarative_mu_formula(document, formula)?;

    let evaluation = evaluate_mu(document.model(), formula, |atom, state| {
        document.state_has_proposition(state, atom)
    })?;
    let status = if evaluation.all_initial_states_satisfy() {
        DeclarativeMuStatus::Satisfied
    } else {
        DeclarativeMuStatus::Violated
    };

    Ok(DeclarativeMuResult {
        formula: render_mu_formula(formula),
        status,
        evaluation,
    })
}

pub fn check_declarative_mu_text(
    document: &DeclarativeDocument,
    input: &str,
) -> Result<DeclarativeMuResult, DeclarativeMuError> {
    let formula = parse_mu_formula(input)?;
    check_declarative_mu(document, &formula)
}

pub fn check_declarative_mu_via_parity(
    document: &DeclarativeDocument,
    formula: &MuFormula<String, String>,
) -> Result<DeclarativeMuParityResult, DeclarativeMuError> {
    validate_declarative_mu_formula(document, formula)?;

    let evaluation = evaluate_mu_via_parity(document.model(), formula, |atom, state| {
        document.state_has_proposition(state, atom)
    })?;
    let status = if evaluation.all_initial_states_satisfy() {
        DeclarativeMuStatus::Satisfied
    } else {
        DeclarativeMuStatus::Violated
    };

    Ok(DeclarativeMuParityResult {
        formula: render_mu_formula(formula),
        status,
        evaluation,
    })
}

pub fn check_declarative_mu_text_via_parity(
    document: &DeclarativeDocument,
    input: &str,
) -> Result<DeclarativeMuParityResult, DeclarativeMuError> {
    let formula = parse_mu_formula(input)?;
    check_declarative_mu_via_parity(document, &formula)
}

/// Validate, resolve, and evaluate one typed textual-surface formula through
/// the proof-honest M77 bounded modal mu-calculus authority.
pub fn check_declarative_mu_with_limits(
    document: &DeclarativeDocument,
    formula: &MuFormula<String, String>,
    limits: ExplorationLimits,
) -> Result<BoundedDeclarativeMuResult, DeclarativeMuError> {
    validate_declarative_mu_formula(document, formula)?;

    let evaluation = evaluate_mu_with_limits(
        document.model(),
        formula,
        |atom, state| document.state_has_proposition(state, atom),
        limits,
    )?;

    Ok(BoundedDeclarativeMuResult {
        formula: render_mu_formula(formula),
        evaluation,
    })
}

/// Parse and evaluate one textual modal mu-calculus formula through M77.
pub fn check_declarative_mu_text_with_limits(
    document: &DeclarativeDocument,
    input: &str,
    limits: ExplorationLimits,
) -> Result<BoundedDeclarativeMuResult, DeclarativeMuError> {
    let formula = parse_mu_formula(input)?;
    check_declarative_mu_with_limits(document, &formula, limits)
}

pub(crate) fn validate_declarative_mu_formula(
    document: &DeclarativeDocument,
    formula: &MuFormula<String, String>,
) -> Result<(), DeclarativeMuError> {
    validate_mu_formula(formula)?;
    let mut atoms = Vec::new();
    collect_mu_atoms(formula, &mut atoms);
    let mut seen = HashSet::new();

    for atom in atoms {
        if !seen.insert(atom) {
            continue;
        }
        if document.proposition_states(atom).is_none() {
            return Err(DeclarativeMuError::UnknownProposition {
                proposition: atom.to_owned(),
            });
        }
    }
    Ok(())
}

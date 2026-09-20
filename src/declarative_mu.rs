use crate::declarative::DeclarativeDocument;
use crate::mu_calculus::{
    evaluate_mu, validate_mu_formula, MuError, MuEvaluation, MuFormula, MuValidationError,
};
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

#[derive(Debug)]
pub enum DeclarativeMuError {
    Parse(MuParseError),
    Validation(MuValidationError<String>),
    UnknownProposition { proposition: String },
    Backend(MuError<String>),
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

/// Validate a typed textual-surface formula, resolve all named propositions,
/// then delegate execution exclusively to the M75 modal mu-calculus authority.
pub fn check_declarative_mu(
    document: &DeclarativeDocument,
    formula: &MuFormula<String, String>,
) -> Result<DeclarativeMuResult, DeclarativeMuError> {
    validate_mu_formula(formula)?;
    validate_atoms(document, formula)?;

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

fn validate_atoms(
    document: &DeclarativeDocument,
    formula: &MuFormula<String, String>,
) -> Result<(), DeclarativeMuError> {
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

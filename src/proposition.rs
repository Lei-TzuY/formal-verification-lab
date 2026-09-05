use crate::declarative::DeclarativeDocument;
use crate::eventuality::{
    check_eventuality, EventualityCounterexample, EventualityError, EventualityProperty,
    EventualityStatus,
};
use crate::exact_state::{ExactStateBackend, ExactStateEvidence, ExactStateStatus};
use crate::property::{
    check_reachability, ReachabilityError, ReachabilityProperty, ReachabilityStatus,
};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropositionPropertyKind {
    Reachable,
    AllEventually,
}

/// A named state-proposition property over a declarative document.
///
/// The proposition selects a set of explicit string states. Verification still
/// delegates to the existing reachability/eventuality predicate backends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionPropertySpec {
    name: String,
    proposition: String,
    kind: PropositionPropertyKind,
}

impl PropositionPropertySpec {
    pub fn reachable(
        name: impl Into<String>,
        proposition: impl Into<String>,
    ) -> Result<Self, PropositionError> {
        Self::new(name, proposition, PropositionPropertyKind::Reachable)
    }

    pub fn all_eventually(
        name: impl Into<String>,
        proposition: impl Into<String>,
    ) -> Result<Self, PropositionError> {
        Self::new(name, proposition, PropositionPropertyKind::AllEventually)
    }

    fn new(
        name: impl Into<String>,
        proposition: impl Into<String>,
        kind: PropositionPropertyKind,
    ) -> Result<Self, PropositionError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(PropositionError::EmptyPropertyName);
        }
        let proposition = proposition.into();
        if proposition.trim().is_empty() {
            return Err(PropositionError::EmptyProposition);
        }
        Ok(Self {
            name,
            proposition,
            kind,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn proposition(&self) -> &str {
        &self.proposition
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionResult {
    pub property: String,
    pub proposition: String,
    pub backend: ExactStateBackend,
    pub status: ExactStateStatus,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub evidence: Option<ExactStateEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropositionError {
    EmptyPropertyName,
    EmptyProposition,
    UnknownProposition { proposition: String },
    Reachability(ReachabilityError),
    Eventuality(EventualityError),
}

impl fmt::Display for PropositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "proposition property name must not be empty"),
            Self::EmptyProposition => write!(f, "proposition name must not be empty"),
            Self::UnknownProposition { proposition } => {
                write!(f, "unknown proposition '{proposition}'")
            }
            Self::Reachability(error) => write!(f, "reachability backend failed: {error}"),
            Self::Eventuality(error) => write!(f, "eventuality backend failed: {error}"),
        }
    }
}

impl std::error::Error for PropositionError {}

impl From<ReachabilityError> for PropositionError {
    fn from(value: ReachabilityError) -> Self {
        Self::Reachability(value)
    }
}

impl From<EventualityError> for PropositionError {
    fn from(value: EventualityError) -> Self {
        Self::Eventuality(value)
    }
}

/// Check one proposition property without introducing a new graph traversal.
/// Unknown proposition names fail closed before either backend is invoked.
pub fn check_proposition_property(
    document: &DeclarativeDocument,
    spec: &PropositionPropertySpec,
) -> Result<PropositionResult, PropositionError> {
    let members = document
        .proposition_states(&spec.proposition)
        .ok_or_else(|| PropositionError::UnknownProposition {
            proposition: spec.proposition.clone(),
        })?
        .iter()
        .cloned()
        .collect::<HashSet<_>>();

    match spec.kind {
        PropositionPropertyKind::Reachable => check_reachable(document, spec, members),
        PropositionPropertyKind::AllEventually => check_all_eventually(document, spec, members),
    }
}

fn check_reachable(
    document: &DeclarativeDocument,
    spec: &PropositionPropertySpec,
    members: HashSet<String>,
) -> Result<PropositionResult, PropositionError> {
    let property = ReachabilityProperty::new(spec.name.clone(), move |state: &String| {
        members.contains(state)
    })?;
    let result = check_reachability(document.model(), &property)?;
    let evidence = result
        .witness
        .map(|trace| ExactStateEvidence::ReachabilityWitness { trace });

    Ok(PropositionResult {
        property: result.property,
        proposition: spec.proposition.clone(),
        backend: ExactStateBackend::Reachability,
        status: match result.status {
            ReachabilityStatus::Reachable => ExactStateStatus::Satisfied,
            ReachabilityStatus::Unreachable => ExactStateStatus::Violated,
        },
        discovered_states: result.discovered_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        evidence,
    })
}

fn check_all_eventually(
    document: &DeclarativeDocument,
    spec: &PropositionPropertySpec,
    members: HashSet<String>,
) -> Result<PropositionResult, PropositionError> {
    let property = EventualityProperty::new(spec.name.clone(), move |state: &String| {
        members.contains(state)
    })?;
    let result = check_eventuality(document.model(), &property)?;
    let evidence = match result.counterexample {
        None => None,
        Some(EventualityCounterexample::Finite { trace }) => {
            Some(ExactStateEvidence::EventualityFiniteCounterexample { trace })
        }
        Some(EventualityCounterexample::Infinite { stem, cycle }) => {
            Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle })
        }
    };

    Ok(PropositionResult {
        property: result.property,
        proposition: spec.proposition.clone(),
        backend: ExactStateBackend::Eventuality,
        status: match result.status {
            EventualityStatus::Satisfied => ExactStateStatus::Satisfied,
            EventualityStatus::Violated => ExactStateStatus::Violated,
        },
        discovered_states: result.discovered_states,
        explored_transitions: result.explored_transitions,
        max_depth_reached: result.max_depth_reached,
        evidence,
    })
}

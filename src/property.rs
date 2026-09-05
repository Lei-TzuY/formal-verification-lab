use crate::checker::{check, TraceStep, VerificationStatus};
use crate::model::{Invariant, ModelError, TransitionSystem};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

/// A named existential reachability query: does at least one reachable state
/// satisfy the supplied target predicate?
pub struct ReachabilityProperty<S> {
    name: String,
    target: Arc<dyn Fn(&S) -> bool + Send + Sync>,
}

impl<S> Clone for ReachabilityProperty<S> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            target: Arc::clone(&self.target),
        }
    }
}

impl<S> fmt::Debug for ReachabilityProperty<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReachabilityProperty")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachabilityStatus {
    Reachable,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityResult<S> {
    pub property: String,
    pub status: ReachabilityStatus,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    /// A shortest transition-count path to a target state when reachable.
    /// The first step has no action because it is an initial state.
    pub witness: Option<Vec<TraceStep<S>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReachabilityError {
    EmptyPropertyName,
    Model(ModelError),
    UnexpectedInconclusive,
    MissingWitness,
}

impl fmt::Display for ReachabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "reachability property name must not be empty"),
            Self::Model(error) => write!(f, "reachability exploration failed: {error}"),
            Self::UnexpectedInconclusive => write!(
                f,
                "unbounded canonical reachability exploration unexpectedly became inconclusive"
            ),
            Self::MissingWitness => write!(
                f,
                "canonical checker reported target reachability without a witness trace"
            ),
        }
    }
}

impl std::error::Error for ReachabilityError {}

impl From<ModelError> for ReachabilityError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

impl<S> ReachabilityProperty<S> {
    pub fn new(
        name: impl Into<String>,
        target: impl Fn(&S) -> bool + Send + Sync + 'static,
    ) -> Result<Self, ReachabilityError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ReachabilityError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            target: Arc::new(target),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Evaluate an existential reachability property with the canonical BFS
/// checker. The query is encoded internally as a sentinel safety invariant
/// that holds until the target predicate becomes true. Consequently a target
/// hit is the canonical checker's first invariant violation and inherits its
/// deterministic shortest-trace guarantee.
///
/// `Unreachable` is returned only after the unbounded canonical checker has
/// exhausted the finite reachable graph. This is not a liveness or universal
/// eventuality claim.
pub fn check_reachability<S>(
    model: &TransitionSystem<S>,
    property: &ReachabilityProperty<S>,
) -> Result<ReachabilityResult<S>, ReachabilityError>
where
    S: Clone + Eq + Hash + fmt::Debug + 'static,
{
    let target = Arc::clone(&property.target);
    let sentinel = Invariant::new("__reachability_target_not_seen", move |state: &S| {
        !(target)(state)
    });
    let derived = model.with_replaced_invariants(
        format!("{}::reachability::{}", model.name(), property.name),
        vec![sentinel],
    )?;
    let result = check(&derived)?;

    match result.status {
        VerificationStatus::Violated => {
            let counterexample = result
                .counterexample
                .ok_or(ReachabilityError::MissingWitness)?;
            Ok(ReachabilityResult {
                property: property.name.clone(),
                status: ReachabilityStatus::Reachable,
                discovered_states: result.discovered_states,
                checked_states: result.checked_states,
                explored_transitions: result.explored_transitions,
                max_depth_reached: result.max_depth_reached,
                witness: Some(counterexample.trace),
            })
        }
        VerificationStatus::Safe => Ok(ReachabilityResult {
            property: property.name.clone(),
            status: ReachabilityStatus::Unreachable,
            discovered_states: result.discovered_states,
            checked_states: result.checked_states,
            explored_transitions: result.explored_transitions,
            max_depth_reached: result.max_depth_reached,
            witness: None,
        }),
        VerificationStatus::Inconclusive => Err(ReachabilityError::UnexpectedInconclusive),
    }
}

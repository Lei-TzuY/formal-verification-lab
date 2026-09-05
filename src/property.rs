use crate::checker::{
    check, search_with_probes, ExplorationLimits, GraphSearchOutcome, TraceStep, VerificationStatus,
};
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

/// A named policy that distinguishes legitimate terminal states from
/// unexpected terminal states. A reachable state is considered a deadlock iff
/// it has no outgoing transitions and this predicate returns false.
pub struct DeadlockProperty<S> {
    name: String,
    allowed_terminal: Arc<dyn Fn(&S) -> bool + Send + Sync>,
}

impl<S> Clone for DeadlockProperty<S> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            allowed_terminal: Arc::clone(&self.allowed_terminal),
        }
    }
}

impl<S> fmt::Debug for DeadlockProperty<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeadlockProperty")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<S> DeadlockProperty<S> {
    pub fn new(
        name: impl Into<String>,
        allowed_terminal: impl Fn(&S) -> bool + Send + Sync + 'static,
    ) -> Result<Self, DeadlockError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DeadlockError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            allowed_terminal: Arc::new(allowed_terminal),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlockStatus {
    DeadlockFound,
    DeadlockFree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlockResult<S> {
    pub property: String,
    pub status: DeadlockStatus,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    /// A shortest transition-count path to an unexpected terminal state when
    /// one is reachable. The first step is an initial state with no action.
    pub witness: Option<Vec<TraceStep<S>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlockError {
    EmptyPropertyName,
    Model(ModelError),
    UnexpectedInconclusive,
    MissingWitness,
}

impl fmt::Display for DeadlockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "deadlock property name must not be empty"),
            Self::Model(error) => write!(f, "deadlock exploration failed: {error}"),
            Self::UnexpectedInconclusive => write!(
                f,
                "unbounded canonical deadlock exploration unexpectedly became inconclusive"
            ),
            Self::MissingWitness => write!(
                f,
                "canonical search reported a reachable deadlock without a witness trace"
            ),
        }
    }
}

impl std::error::Error for DeadlockError {}

impl From<ModelError> for DeadlockError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Detect reachable unexpected terminal states using the same canonical BFS
/// substrate as safety checking. The transition relation is evaluated exactly
/// once for each checked state. Original safety invariants are intentionally
/// not part of this graph property.
///
/// `DeadlockFree` is returned only after exhaustive unbounded exploration of
/// the finite reachable graph. A found deadlock inherits deterministic shortest
/// transition-count witness semantics from canonical BFS ordering.
pub fn check_deadlock<S>(
    model: &TransitionSystem<S>,
    property: &DeadlockProperty<S>,
) -> Result<DeadlockResult<S>, DeadlockError>
where
    S: Clone + Eq + Hash,
{
    let search = search_with_probes(
        model,
        ExplorationLimits::unbounded(),
        |_state| None,
        |state, transitions| {
            (transitions.is_empty() && !(property.allowed_terminal)(state))
                .then(|| "unexpected-terminal".to_owned())
        },
    )?;

    let property_name = property.name.clone();
    let discovered_states = search.discovered_states;
    let checked_states = search.checked_states;
    let explored_transitions = search.explored_transitions;
    let max_depth_reached = search.max_depth_reached;

    match search.outcome {
        GraphSearchOutcome::Match { trace, .. } => {
            if trace.is_empty() {
                return Err(DeadlockError::MissingWitness);
            }
            Ok(DeadlockResult {
                property: property_name,
                status: DeadlockStatus::DeadlockFound,
                discovered_states,
                checked_states,
                explored_transitions,
                max_depth_reached,
                witness: Some(trace),
            })
        }
        GraphSearchOutcome::Exhausted => Ok(DeadlockResult {
            property: property_name,
            status: DeadlockStatus::DeadlockFree,
            discovered_states,
            checked_states,
            explored_transitions,
            max_depth_reached,
            witness: None,
        }),
        GraphSearchOutcome::Inconclusive(_) => Err(DeadlockError::UnexpectedInconclusive),
    }
}

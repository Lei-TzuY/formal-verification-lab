use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

/// Metadata describing one logical state variable in a finite-state model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateVariable {
    pub name: String,
    pub description: String,
}

impl StateVariable {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// A named safety invariant checked in every reachable state.
pub struct Invariant<S> {
    name: String,
    predicate: Arc<dyn Fn(&S) -> bool + Send + Sync>,
}

impl<S> Clone for Invariant<S> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            predicate: Arc::clone(&self.predicate),
        }
    }
}

impl<S> fmt::Debug for Invariant<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Invariant")
            .field("name", &self.name)
            .finish()
    }
}

impl<S> Invariant<S> {
    pub fn new(
        name: impl Into<String>,
        predicate: impl Fn(&S) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            predicate: Arc::new(predicate),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn holds(&self, state: &S) -> bool {
        (self.predicate)(state)
    }
}

/// One labeled edge in the transition relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition<S> {
    pub action: String,
    pub next: S,
}

impl<S> Transition<S> {
    pub fn new(action: impl Into<String>, next: S) -> Self {
        Self {
            action: action.into(),
            next,
        }
    }
}

/// Validation or transition-generation error for a model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    EmptyModelName,
    NoStateVariables,
    EmptyStateVariableName { index: usize },
    DuplicateStateVariable { name: String },
    NoInitialStates,
    NoInvariants,
    EmptyInvariantName { index: usize },
    DuplicateInvariant { name: String },
    EmptyTransitionAction,
    TransitionGeneration { message: String },
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyModelName => write!(f, "model name must not be empty"),
            Self::NoStateVariables => write!(f, "model must declare at least one state variable"),
            Self::EmptyStateVariableName { index } => {
                write!(f, "state variable at index {index} has an empty name")
            }
            Self::DuplicateStateVariable { name } => {
                write!(f, "duplicate state variable name: {name}")
            }
            Self::NoInitialStates => write!(f, "model must contain at least one initial state"),
            Self::NoInvariants => write!(f, "model must contain at least one safety invariant"),
            Self::EmptyInvariantName { index } => {
                write!(f, "invariant at index {index} has an empty name")
            }
            Self::DuplicateInvariant { name } => write!(f, "duplicate invariant name: {name}"),
            Self::EmptyTransitionAction => write!(f, "transition action labels must not be empty"),
            Self::TransitionGeneration { message } => {
                write!(f, "transition generation failed: {message}")
            }
        }
    }
}

impl std::error::Error for ModelError {}

type TransitionFn<S> = dyn Fn(&S) -> Result<Vec<Transition<S>>, ModelError> + Send + Sync;

/// A finite-state transition system with named variables, initial states,
/// transition relation, and safety invariants.
///
/// The checker requires `S` to have stable equality and hashing. The model is
/// responsible for returning successors in a deterministic order when
/// reproducible traces are desired.
pub struct TransitionSystem<S> {
    name: String,
    state_variables: Vec<StateVariable>,
    initial_states: Vec<S>,
    transition_relation: Arc<TransitionFn<S>>,
    invariants: Vec<Invariant<S>>,
}

impl<S> fmt::Debug for TransitionSystem<S>
where
    S: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitionSystem")
            .field("name", &self.name)
            .field("state_variables", &self.state_variables)
            .field("initial_states", &self.initial_states)
            .field("invariants", &self.invariants)
            .finish_non_exhaustive()
    }
}

impl<S> TransitionSystem<S>
where
    S: Clone + Eq + Hash,
{
    pub fn new(
        name: impl Into<String>,
        state_variables: Vec<StateVariable>,
        initial_states: Vec<S>,
        transition_relation: impl Fn(&S) -> Result<Vec<Transition<S>>, ModelError>
            + Send
            + Sync
            + 'static,
        invariants: Vec<Invariant<S>>,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        validate_metadata(&name, &state_variables, &initial_states, &invariants)?;

        Ok(Self {
            name,
            state_variables,
            initial_states,
            transition_relation: Arc::new(transition_relation),
            invariants,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn state_variables(&self) -> &[StateVariable] {
        &self.state_variables
    }

    pub fn initial_states(&self) -> &[S] {
        &self.initial_states
    }

    pub fn invariants(&self) -> &[Invariant<S>] {
        &self.invariants
    }

    /// Build an internal view over the same transition graph with a different
    /// invariant set. Property engines use this to reuse the canonical BFS
    /// checker instead of implementing a second traversal semantics.
    pub(crate) fn with_replaced_invariants(
        &self,
        name: impl Into<String>,
        invariants: Vec<Invariant<S>>,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        validate_metadata(
            &name,
            &self.state_variables,
            &self.initial_states,
            &invariants,
        )?;

        Ok(Self {
            name,
            state_variables: self.state_variables.clone(),
            initial_states: self.initial_states.clone(),
            transition_relation: Arc::clone(&self.transition_relation),
            invariants,
        })
    }

    pub fn successors(&self, state: &S) -> Result<Vec<Transition<S>>, ModelError> {
        let transitions = (self.transition_relation)(state)?;
        if transitions
            .iter()
            .any(|transition| transition.action.trim().is_empty())
        {
            return Err(ModelError::EmptyTransitionAction);
        }
        Ok(transitions)
    }
}

fn validate_metadata<S>(
    name: &str,
    state_variables: &[StateVariable],
    initial_states: &[S],
    invariants: &[Invariant<S>],
) -> Result<(), ModelError> {
    if name.trim().is_empty() {
        return Err(ModelError::EmptyModelName);
    }
    if state_variables.is_empty() {
        return Err(ModelError::NoStateVariables);
    }
    if initial_states.is_empty() {
        return Err(ModelError::NoInitialStates);
    }
    if invariants.is_empty() {
        return Err(ModelError::NoInvariants);
    }

    let mut variable_names = HashSet::new();
    for (index, variable) in state_variables.iter().enumerate() {
        let trimmed = variable.name.trim();
        if trimmed.is_empty() {
            return Err(ModelError::EmptyStateVariableName { index });
        }
        if !variable_names.insert(trimmed.to_owned()) {
            return Err(ModelError::DuplicateStateVariable {
                name: trimmed.to_owned(),
            });
        }
    }

    let mut invariant_names = HashSet::new();
    for (index, invariant) in invariants.iter().enumerate() {
        let trimmed = invariant.name().trim();
        if trimmed.is_empty() {
            return Err(ModelError::EmptyInvariantName { index });
        }
        if !invariant_names.insert(trimmed.to_owned()) {
            return Err(ModelError::DuplicateInvariant {
                name: trimmed.to_owned(),
            });
        }
    }

    Ok(())
}

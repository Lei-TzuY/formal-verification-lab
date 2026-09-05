use crate::model::{Invariant, ModelError, StateVariable, Transition, TransitionSystem};
use std::hash::Hash;
use std::sync::Arc;

type TransitionFn<S> = dyn Fn(&S) -> Result<Vec<Transition<S>>, ModelError> + Send + Sync;

/// Fluent construction layer for typed Rust transition systems.
///
/// The builder is intentionally thin: `build` delegates to
/// [`TransitionSystem::new`], so model validation and checker semantics have a
/// single source of truth. It is not a separate DSL or execution engine.
pub struct TransitionSystemBuilder<S> {
    name: String,
    state_variables: Vec<StateVariable>,
    initial_states: Vec<S>,
    transition_relation: Arc<TransitionFn<S>>,
    invariants: Vec<Invariant<S>>,
}

impl<S> TransitionSystemBuilder<S>
where
    S: Clone + Eq + Hash + 'static,
{
    /// Start a typed model with its single transition relation.
    pub fn new(
        name: impl Into<String>,
        transition_relation: impl Fn(&S) -> Result<Vec<Transition<S>>, ModelError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            state_variables: Vec::new(),
            initial_states: Vec::new(),
            transition_relation: Arc::new(transition_relation),
            invariants: Vec::new(),
        }
    }

    pub fn state_variable(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.state_variables
            .push(StateVariable::new(name, description));
        self
    }

    pub fn initial_state(mut self, state: S) -> Self {
        self.initial_states.push(state);
        self
    }

    pub fn safety_invariant(
        mut self,
        name: impl Into<String>,
        predicate: impl Fn(&S) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.invariants.push(Invariant::new(name, predicate));
        self
    }

    /// Materialize the canonical transition system and run its normal model
    /// validation. Invalid builder input therefore fails exactly like direct
    /// `TransitionSystem::new` construction.
    pub fn build(self) -> Result<TransitionSystem<S>, ModelError> {
        let transition_relation = self.transition_relation;
        TransitionSystem::new(
            self.name,
            self.state_variables,
            self.initial_states,
            move |state| transition_relation(state),
            self.invariants,
        )
    }
}

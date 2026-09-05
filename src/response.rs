use crate::checker::TraceStep;
use crate::model::TransitionSystem;
use crate::recurrence::{
    capture_reachable_graph, component_is_cyclic, cycle_witness, induced_graph, shortest_path,
    strongly_connected_components, ReachableGraph, RecurrenceError, SnapshotEdge,
};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

/// A named action-level response property:
/// every trigger action must eventually be followed by a response action.
pub struct ResponseProperty {
    name: String,
    trigger: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    response: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl Clone for ResponseProperty {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            trigger: Arc::clone(&self.trigger),
            response: Arc::clone(&self.response),
        }
    }
}

impl fmt::Debug for ResponseProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseProperty")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl ResponseProperty {
    pub fn new(
        name: impl Into<String>,
        trigger: impl Fn(&str) -> bool + Send + Sync + 'static,
        response: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Result<Self, ResponseError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ResponseError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            trigger: Arc::new(trigger),
            response: Arc::new(response),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Product state for the explicit one-bit response monitor.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObligationState<S> {
    pub state: S,
    pub pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseCounterexample<S> {
    /// A maximal finite execution ends while a trigger remains unanswered.
    Finite {
        trace: Vec<TraceStep<ObligationState<S>>>,
    },
    /// A reachable execution can remain forever with an unanswered trigger.
    Infinite {
        stem: Vec<TraceStep<ObligationState<S>>>,
        cycle: Vec<TraceStep<ObligationState<S>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseResult<S> {
    pub property: String,
    pub status: ResponseStatus,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub counterexample: Option<ResponseCounterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseError {
    EmptyPropertyName,
    Graph(RecurrenceError),
    MissingFiniteWitness,
    MissingCycleWitness,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "response property name must not be empty"),
            Self::Graph(error) => write!(f, "response graph analysis failed: {error}"),
            Self::MissingFiniteWitness => write!(
                f,
                "pending terminal product state did not yield a counterexample trace"
            ),
            Self::MissingCycleWitness => write!(
                f,
                "pending cyclic product component did not yield a stem-plus-cycle counterexample"
            ),
        }
    }
}

impl std::error::Error for ResponseError {}

impl From<RecurrenceError> for ResponseError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

/// Verify `trigger -> eventually response` over every maximal execution of the
/// finite model, without a fairness assumption.
///
/// The original model transition relation is evaluated exactly once. Analysis
/// then constructs a deterministic product graph over the captured snapshot
/// and a one-bit pending-obligation monitor. A response action clears the bit;
/// otherwise a trigger action sets it. If an action matches both predicates,
/// the response wins, so that action satisfies its trigger immediately.
///
/// A violation is a reachable pending product state that can either terminate
/// or remain forever inside a pending cyclic SCC. Finite terminals take
/// precedence; otherwise the first cyclic SCC in canonical product discovery
/// order yields a deterministic stem-plus-cycle witness.
pub fn check_response<S>(
    model: &TransitionSystem<S>,
    property: &ResponseProperty,
) -> Result<ResponseResult<S>, ResponseError>
where
    S: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model)?;
    let product = build_product_graph(&captured.graph, property);
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();

    if let Some(terminal) = product
        .states
        .iter()
        .enumerate()
        .find(|(id, state)| state.pending && product.outgoing[*id].is_empty())
        .map(|(id, _)| id)
    {
        let trace = shortest_path(&product, &product.initial_ids, terminal, None)
            .ok_or(ResponseError::MissingFiniteWitness)?;
        return Ok(ResponseResult {
            property: property.name.clone(),
            status: ResponseStatus::Violated,
            model_states: captured.discovered_states,
            model_transitions: captured.explored_transitions,
            product_states: product.states.len(),
            product_transitions,
            counterexample: Some(ResponseCounterexample::Finite { trace }),
        });
    }

    let pending = product
        .states
        .iter()
        .map(|state| state.pending)
        .collect::<Vec<_>>();
    let pending_old_ids = pending
        .iter()
        .enumerate()
        .filter(|(_, included)| **included)
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    let mut residual = induced_graph(&product, &pending);
    let components = strongly_connected_components(&residual);

    if let Some((component_index, component)) = components
        .iter()
        .enumerate()
        .find(|(_, component)| component_is_cyclic(&residual, component))
    {
        let entry = *component
            .first()
            .ok_or(ResponseError::MissingCycleWitness)?;
        let product_entry = pending_old_ids[entry];
        let stem = shortest_path(&product, &product.initial_ids, product_entry, None)
            .ok_or(ResponseError::MissingCycleWitness)?;

        // `cycle_witness` only needs a root to reconstruct its local cycle.
        // The externally reported stem is the global product path above.
        residual.initial_ids = vec![entry];
        let local = cycle_witness(&residual, component_index, component)?
            .ok_or(ResponseError::MissingCycleWitness)?;

        return Ok(ResponseResult {
            property: property.name.clone(),
            status: ResponseStatus::Violated,
            model_states: captured.discovered_states,
            model_transitions: captured.explored_transitions,
            product_states: product.states.len(),
            product_transitions,
            counterexample: Some(ResponseCounterexample::Infinite {
                stem,
                cycle: local.cycle,
            }),
        });
    }

    Ok(ResponseResult {
        property: property.name.clone(),
        status: ResponseStatus::Satisfied,
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample: None,
    })
}

fn build_product_graph<S: Clone>(
    graph: &ReachableGraph<S>,
    property: &ResponseProperty,
) -> ReachableGraph<ObligationState<S>> {
    let mut states = Vec::new();
    let mut outgoing: Vec<Vec<SnapshotEdge>> = Vec::new();
    let mut initial_ids = Vec::new();
    let mut ids: HashMap<(usize, bool), usize> = HashMap::new();
    let mut queue = VecDeque::new();

    for &model_id in &graph.initial_ids {
        let key = (model_id, false);
        let product_id = if let Some(id) = ids.get(&key).copied() {
            id
        } else {
            let id = states.len();
            ids.insert(key, id);
            states.push(ObligationState {
                state: graph.states[model_id].clone(),
                pending: false,
            });
            outgoing.push(Vec::new());
            queue.push_back(key);
            id
        };
        if !initial_ids.contains(&product_id) {
            initial_ids.push(product_id);
        }
    }

    while let Some((model_id, pending)) = queue.pop_front() {
        let source = ids[&(model_id, pending)];
        for edge in &graph.outgoing[model_id] {
            let is_trigger = (property.trigger)(&edge.action);
            let is_response = (property.response)(&edge.action);
            let next_pending = if is_response {
                false
            } else {
                pending || is_trigger
            };
            let key = (edge.target, next_pending);
            let target = if let Some(id) = ids.get(&key).copied() {
                id
            } else {
                let id = states.len();
                ids.insert(key, id);
                states.push(ObligationState {
                    state: graph.states[edge.target].clone(),
                    pending: next_pending,
                });
                outgoing.push(Vec::new());
                queue.push_back(key);
                id
            };
            outgoing[source].push(SnapshotEdge {
                action: edge.action.clone(),
                target,
            });
        }
    }

    ReachableGraph {
        states,
        outgoing,
        initial_ids,
    }
}

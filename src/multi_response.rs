use crate::checker::TraceStep;
use crate::model::TransitionSystem;
use crate::recurrence::{
    capture_reachable_graph, component_is_cyclic, cycle_witness, induced_graph, shortest_path,
    strongly_connected_components, ReachableGraph, RecurrenceError, SnapshotEdge,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

pub(crate) type ActionPredicate = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// One independently tracked action-response obligation class.
pub struct ResponseClause {
    name: String,
    trigger: ActionPredicate,
    response: ActionPredicate,
}

impl Clone for ResponseClause {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            trigger: Arc::clone(&self.trigger),
            response: Arc::clone(&self.response),
        }
    }
}

impl fmt::Debug for ResponseClause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseClause")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl ResponseClause {
    pub fn new(
        name: impl Into<String>,
        trigger: impl Fn(&str) -> bool + Send + Sync + 'static,
        response: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Result<Self, MultiResponseError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MultiResponseError::EmptyClauseName);
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

    pub(crate) fn from_shared(
        name: String,
        trigger: ActionPredicate,
        response: ActionPredicate,
    ) -> Self {
        Self {
            name,
            trigger,
            response,
        }
    }
}

/// A conjunction of independently tracked response clauses.
pub struct MultiResponseProperty {
    name: String,
    clauses: Vec<ResponseClause>,
}

impl Clone for MultiResponseProperty {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            clauses: self.clauses.clone(),
        }
    }
}

impl fmt::Debug for MultiResponseProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MultiResponseProperty")
            .field("name", &self.name)
            .field(
                "clauses",
                &self.clauses.iter().map(|clause| &clause.name).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl MultiResponseProperty {
    pub fn new(
        name: impl Into<String>,
        clauses: Vec<ResponseClause>,
    ) -> Result<Self, MultiResponseError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(MultiResponseError::EmptyPropertyName);
        }
        if clauses.is_empty() {
            return Err(MultiResponseError::NoClauses);
        }

        let mut names = HashSet::new();
        for clause in &clauses {
            if !names.insert(clause.name.clone()) {
                return Err(MultiResponseError::DuplicateClauseName {
                    name: clause.name.clone(),
                });
            }
        }

        Ok(Self { name, clauses })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn clauses(&self) -> &[ResponseClause] {
        &self.clauses
    }

    pub(crate) fn from_single_shared(
        name: String,
        trigger: ActionPredicate,
        response: ActionPredicate,
    ) -> Self {
        let clause = ResponseClause::from_shared(name.clone(), trigger, response);
        Self {
            name,
            clauses: vec![clause],
        }
    }
}

/// Explicit product state: model state plus one pending bit per response clause.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiObligationState<S> {
    pub state: S,
    pub pending: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiResponseStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiResponseCounterexample<S> {
    Finite {
        clause: String,
        trace: Vec<TraceStep<MultiObligationState<S>>>,
    },
    Infinite {
        clause: String,
        stem: Vec<TraceStep<MultiObligationState<S>>>,
        cycle: Vec<TraceStep<MultiObligationState<S>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiResponseResult<S> {
    pub property: String,
    pub status: MultiResponseStatus,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub clause_count: usize,
    pub counterexample: Option<MultiResponseCounterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiResponseError {
    EmptyPropertyName,
    NoClauses,
    EmptyClauseName,
    DuplicateClauseName { name: String },
    Graph(RecurrenceError),
    MissingFiniteWitness,
    MissingCycleWitness,
}

impl fmt::Display for MultiResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPropertyName => write!(f, "multi-response property name must not be empty"),
            Self::NoClauses => write!(f, "multi-response property requires at least one clause"),
            Self::EmptyClauseName => write!(f, "response clause name must not be empty"),
            Self::DuplicateClauseName { name } => {
                write!(f, "duplicate response clause name '{name}'")
            }
            Self::Graph(error) => write!(f, "multi-response graph analysis failed: {error}"),
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

impl std::error::Error for MultiResponseError {}

impl From<RecurrenceError> for MultiResponseError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

/// Verify a conjunction of action-response clauses over every maximal execution.
///
/// The model is captured once. The analyzer then explores a deterministic
/// product graph `(model_state, pending_bits)`, where each clause has one bit.
/// Clause `i` updates independently on every action: a response clears bit `i`;
/// otherwise a trigger sets it. Thus one action may affect several clauses.
///
/// A finite terminal violates the first pending clause at that terminal. For an
/// infinite violation, each clause is analyzed separately over the subgraph in
/// which its own bit remains pending. This distinction is essential: a cycle in
/// which different clauses alternate being pending is not itself a violation if
/// every individual clause is repeatedly discharged.
///
/// There is no fairness assumption. Among infinite violations, the witness with
/// the shortest global stem is selected; ties use clause declaration order and
/// then canonical product discovery order. The selected closed cycle is
/// deterministic but is not claimed globally shortest.
pub fn check_multi_response<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
) -> Result<MultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model)?;
    let product = build_product_graph(&captured.graph, property);
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();

    for (terminal, state) in product.states.iter().enumerate() {
        if !product.outgoing[terminal].is_empty() {
            continue;
        }
        let Some(clause_index) = state.pending.iter().position(|pending| *pending) else {
            continue;
        };
        let trace = shortest_path(&product, &product.initial_ids, terminal, None)
            .ok_or(MultiResponseError::MissingFiniteWitness)?;
        return Ok(MultiResponseResult {
            property: property.name.clone(),
            status: MultiResponseStatus::Violated,
            model_states: captured.discovered_states,
            model_transitions: captured.explored_transitions,
            product_states: product.states.len(),
            product_transitions,
            clause_count: property.clauses.len(),
            counterexample: Some(MultiResponseCounterexample::Finite {
                clause: property.clauses[clause_index].name.clone(),
                trace,
            }),
        });
    }

    let mut best: Option<InfiniteCandidate<S>> = None;
    for clause_index in 0..property.clauses.len() {
        let included = product
            .states
            .iter()
            .map(|state| state.pending[clause_index])
            .collect::<Vec<_>>();
        let old_ids = included
            .iter()
            .enumerate()
            .filter(|(_, included)| **included)
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        if old_ids.is_empty() {
            continue;
        }

        let residual = induced_graph(&product, &included);
        let components = strongly_connected_components(&residual);
        let Some((component_index, component)) = components
            .iter()
            .enumerate()
            .find(|(_, component)| component_is_cyclic(&residual, component))
        else {
            continue;
        };
        let entry = *component
            .first()
            .ok_or(MultiResponseError::MissingCycleWitness)?;
        let product_entry = old_ids[entry];
        let stem = shortest_path(&product, &product.initial_ids, product_entry, None)
            .ok_or(MultiResponseError::MissingCycleWitness)?;
        let candidate = InfiniteCandidate {
            clause_index,
            product_entry,
            stem,
            residual,
            component_index,
            component: component.clone(),
        };

        let replace = best.as_ref().is_none_or(|current| {
            candidate_key(&candidate) < candidate_key(current)
        });
        if replace {
            best = Some(candidate);
        }
    }

    if let Some(mut candidate) = best {
        let entry = *candidate
            .component
            .first()
            .ok_or(MultiResponseError::MissingCycleWitness)?;
        candidate.residual.initial_ids = vec![entry];
        let local = cycle_witness(
            &candidate.residual,
            candidate.component_index,
            &candidate.component,
        )?
        .ok_or(MultiResponseError::MissingCycleWitness)?;

        return Ok(MultiResponseResult {
            property: property.name.clone(),
            status: MultiResponseStatus::Violated,
            model_states: captured.discovered_states,
            model_transitions: captured.explored_transitions,
            product_states: product.states.len(),
            product_transitions,
            clause_count: property.clauses.len(),
            counterexample: Some(MultiResponseCounterexample::Infinite {
                clause: property.clauses[candidate.clause_index].name.clone(),
                stem: candidate.stem,
                cycle: local.cycle,
            }),
        });
    }

    Ok(MultiResponseResult {
        property: property.name.clone(),
        status: MultiResponseStatus::Satisfied,
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        clause_count: property.clauses.len(),
        counterexample: None,
    })
}

struct InfiniteCandidate<S> {
    clause_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<MultiObligationState<S>>>,
    residual: ReachableGraph<MultiObligationState<S>>,
    component_index: usize,
    component: Vec<usize>,
}

fn candidate_key<S>(candidate: &InfiniteCandidate<S>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.clause_index,
        candidate.product_entry,
    )
}

fn build_product_graph<S: Clone>(
    graph: &ReachableGraph<S>,
    property: &MultiResponseProperty,
) -> ReachableGraph<MultiObligationState<S>> {
    let clause_count = property.clauses.len();
    let mut states = Vec::new();
    let mut outgoing: Vec<Vec<SnapshotEdge>> = Vec::new();
    let mut initial_ids = Vec::new();
    let mut ids: HashMap<(usize, Vec<bool>), usize> = HashMap::new();
    let mut queue = VecDeque::new();

    for &model_id in &graph.initial_ids {
        let pending = vec![false; clause_count];
        let key = (model_id, pending.clone());
        let product_id = if let Some(id) = ids.get(&key).copied() {
            id
        } else {
            let id = states.len();
            ids.insert(key.clone(), id);
            states.push(MultiObligationState {
                state: graph.states[model_id].clone(),
                pending,
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
        let source = ids[&(model_id, pending.clone())];
        for edge in &graph.outgoing[model_id] {
            let mut next_pending = pending.clone();
            for (index, clause) in property.clauses.iter().enumerate() {
                if (clause.response)(&edge.action) {
                    next_pending[index] = false;
                } else if (clause.trigger)(&edge.action) {
                    next_pending[index] = true;
                }
            }

            let key = (edge.target, next_pending.clone());
            let target = if let Some(id) = ids.get(&key).copied() {
                id
            } else {
                let id = states.len();
                ids.insert(key.clone(), id);
                states.push(MultiObligationState {
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

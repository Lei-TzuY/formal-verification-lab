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

type AutomatonStep<A> = Arc<dyn Fn(&A, &str) -> A + Send + Sync>;
type AutomatonPredicate<A> = Arc<dyn Fn(&A) -> bool + Send + Sync>;

/// One named generalized Büchi acceptance set over automaton states.
pub struct AcceptanceSet<A> {
    name: String,
    predicate: AutomatonPredicate<A>,
}

impl<A> Clone for AcceptanceSet<A> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            predicate: Arc::clone(&self.predicate),
        }
    }
}

impl<A> fmt::Debug for AcceptanceSet<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AcceptanceSet")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<A> AcceptanceSet<A> {
    pub fn new(
        name: impl Into<String>,
        predicate: impl Fn(&A) -> bool + Send + Sync + 'static,
    ) -> Result<Self, BuchiError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(BuchiError::EmptyAcceptanceName);
        }
        Ok(Self {
            name,
            predicate: Arc::new(predicate),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Policy for finite maximal model executions.
///
/// Büchi acceptance is inherently an infinite-run condition, so finite paths
/// must not be assigned an implicit meaning. `IgnoreTerminals` checks only
/// infinite executions. `RequireAcceptingTerminal` additionally requires every
/// finite product terminal to satisfy every named acceptance set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteRunPolicy {
    IgnoreTerminals,
    RequireAcceptingTerminal,
}

/// A deterministic finite automaton equipped with generalized Büchi acceptance
/// sets and an explicit finite-run policy.
pub struct BuchiAutomaton<A> {
    name: String,
    initial: A,
    step: AutomatonStep<A>,
    acceptance: Vec<AcceptanceSet<A>>,
    finite_policy: FiniteRunPolicy,
}

impl<A: Clone> Clone for BuchiAutomaton<A> {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            initial: self.initial.clone(),
            step: Arc::clone(&self.step),
            acceptance: self.acceptance.clone(),
            finite_policy: self.finite_policy,
        }
    }
}

impl<A: fmt::Debug> fmt::Debug for BuchiAutomaton<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuchiAutomaton")
            .field("name", &self.name)
            .field("initial", &self.initial)
            .field(
                "acceptance",
                &self
                    .acceptance
                    .iter()
                    .map(|set| &set.name)
                    .collect::<Vec<_>>(),
            )
            .field("finite_policy", &self.finite_policy)
            .finish_non_exhaustive()
    }
}

impl<A> BuchiAutomaton<A> {
    pub fn new(
        name: impl Into<String>,
        initial: A,
        step: impl Fn(&A, &str) -> A + Send + Sync + 'static,
        acceptance: Vec<AcceptanceSet<A>>,
        finite_policy: FiniteRunPolicy,
    ) -> Result<Self, BuchiError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(BuchiError::EmptyAutomatonName);
        }
        if acceptance.is_empty() {
            return Err(BuchiError::NoAcceptanceSets);
        }

        let mut names = HashSet::new();
        for set in &acceptance {
            if !names.insert(set.name.clone()) {
                return Err(BuchiError::DuplicateAcceptanceName {
                    name: set.name.clone(),
                });
            }
        }

        Ok(Self {
            name,
            initial,
            step: Arc::new(step),
            acceptance,
            finite_policy,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn initial(&self) -> &A {
        &self.initial
    }

    pub fn acceptance_sets(&self) -> &[AcceptanceSet<A>] {
        &self.acceptance
    }

    pub fn finite_policy(&self) -> FiniteRunPolicy {
        self.finite_policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuchiProductState<S, A> {
    pub state: S,
    pub automaton: A,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuchiStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuchiCounterexample<S, A> {
    /// A finite maximal run violates the explicit strict terminal policy.
    FiniteTerminal {
        missing_acceptance: String,
        trace: Vec<TraceStep<BuchiProductState<S, A>>>,
    },
    /// An infinite lasso can remain forever outside one acceptance set.
    AcceptanceAvoidingCycle {
        acceptance: String,
        stem: Vec<TraceStep<BuchiProductState<S, A>>>,
        cycle: Vec<TraceStep<BuchiProductState<S, A>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuchiResult<S, A> {
    pub automaton: String,
    pub status: BuchiStatus,
    pub finite_policy: FiniteRunPolicy,
    pub acceptance_sets: usize,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub product_transitions: usize,
    pub counterexample: Option<BuchiCounterexample<S, A>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuchiError {
    EmptyAutomatonName,
    NoAcceptanceSets,
    EmptyAcceptanceName,
    DuplicateAcceptanceName { name: String },
    Graph(RecurrenceError),
    MissingWitness,
}

impl fmt::Display for BuchiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAutomatonName => write!(f, "Buchi automaton name must not be empty"),
            Self::NoAcceptanceSets => {
                write!(f, "Buchi automaton requires at least one acceptance set")
            }
            Self::EmptyAcceptanceName => write!(f, "Buchi acceptance-set name must not be empty"),
            Self::DuplicateAcceptanceName { name } => {
                write!(f, "duplicate Buchi acceptance-set name '{name}'")
            }
            Self::Graph(error) => write!(f, "Buchi product analysis failed: {error}"),
            Self::MissingWitness => write!(f, "Buchi violation did not yield a witness"),
        }
    }
}

impl std::error::Error for BuchiError {}

impl From<RecurrenceError> for BuchiError {
    fn from(value: RecurrenceError) -> Self {
        Self::Graph(value)
    }
}

/// Universally verify generalized Büchi acceptance over all infinite maximal
/// executions of a finite model, with explicit handling for finite terminals.
///
/// The model transition relation is evaluated exactly once. The deterministic
/// automaton then consumes captured action labels to form a finite product.
/// Every infinite maximal execution must visit every named acceptance set
/// infinitely often. For acceptance set `F_i`, an infinite violation therefore
/// exists exactly when a reachable product cycle can remain entirely in
/// `not F_i` after some finite stem.
///
/// Finite terminal violations, when enabled by the finite-run policy, take
/// precedence over infinite lassos. Infinite witnesses are selected by shortest
/// global stem, then acceptance-set declaration order, then product discovery
/// order. The returned closed cycle is deterministic but is not claimed globally
/// shortest. No fairness assumption is applied.
pub fn check_buchi<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
) -> Result<BuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model)?;
    let product = build_product_graph(&captured.graph, automaton);
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();

    if automaton.finite_policy == FiniteRunPolicy::RequireAcceptingTerminal {
        for (product_id, state) in product.states.iter().enumerate() {
            if !product.outgoing[product_id].is_empty() {
                continue;
            }
            let Some(set) = automaton
                .acceptance
                .iter()
                .find(|set| !(set.predicate)(&state.automaton))
            else {
                continue;
            };
            let trace = shortest_path(&product, &product.initial_ids, product_id, None)
                .ok_or(BuchiError::MissingWitness)?;
            return Ok(violation_result(
                automaton,
                captured.discovered_states,
                captured.explored_transitions,
                &product,
                product_transitions,
                BuchiCounterexample::FiniteTerminal {
                    missing_acceptance: set.name.clone(),
                    trace,
                },
            ));
        }
    }

    let mut best: Option<CycleCandidate<S, A>> = None;
    for (acceptance_index, set) in automaton.acceptance.iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| !(set.predicate)(&state.automaton))
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
        for (component_index, component) in components.iter().enumerate() {
            if !component_is_cyclic(&residual, component) {
                continue;
            }
            let entry = *component.first().ok_or(BuchiError::MissingWitness)?;
            let product_entry = old_ids[entry];
            let stem = shortest_path(&product, &product.initial_ids, product_entry, None)
                .ok_or(BuchiError::MissingWitness)?;
            let candidate = CycleCandidate {
                acceptance_index,
                product_entry,
                stem,
                residual: residual.clone(),
                component_index,
                component: component.clone(),
            };
            if best
                .as_ref()
                .is_none_or(|current| candidate_key(&candidate) < candidate_key(current))
            {
                best = Some(candidate);
            }
        }
    }

    if let Some(mut candidate) = best {
        let entry = *candidate
            .component
            .first()
            .ok_or(BuchiError::MissingWitness)?;
        candidate.residual.initial_ids = vec![entry];
        let witness = cycle_witness(
            &candidate.residual,
            candidate.component_index,
            &candidate.component,
        )?
        .ok_or(BuchiError::MissingWitness)?;
        return Ok(violation_result(
            automaton,
            captured.discovered_states,
            captured.explored_transitions,
            &product,
            product_transitions,
            BuchiCounterexample::AcceptanceAvoidingCycle {
                acceptance: automaton.acceptance[candidate.acceptance_index]
                    .name
                    .clone(),
                stem: candidate.stem,
                cycle: witness.cycle,
            },
        ));
    }

    Ok(BuchiResult {
        automaton: automaton.name.clone(),
        status: BuchiStatus::Satisfied,
        finite_policy: automaton.finite_policy,
        acceptance_sets: automaton.acceptance.len(),
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample: None,
    })
}

fn violation_result<S, A>(
    automaton: &BuchiAutomaton<A>,
    model_states: usize,
    model_transitions: usize,
    product: &ReachableGraph<BuchiProductState<S, A>>,
    product_transitions: usize,
    counterexample: BuchiCounterexample<S, A>,
) -> BuchiResult<S, A> {
    BuchiResult {
        automaton: automaton.name.clone(),
        status: BuchiStatus::Violated,
        finite_policy: automaton.finite_policy,
        acceptance_sets: automaton.acceptance.len(),
        model_states,
        model_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample: Some(counterexample),
    }
}

struct CycleCandidate<S, A> {
    acceptance_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<BuchiProductState<S, A>>>,
    residual: ReachableGraph<BuchiProductState<S, A>>,
    component_index: usize,
    component: Vec<usize>,
}

fn candidate_key<S, A>(candidate: &CycleCandidate<S, A>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.acceptance_index,
        candidate.product_entry,
    )
}

fn build_product_graph<S, A>(
    graph: &ReachableGraph<S>,
    automaton: &BuchiAutomaton<A>,
) -> ReachableGraph<BuchiProductState<S, A>>
where
    S: Clone,
    A: Clone + Eq + Hash,
{
    let mut states = Vec::new();
    let mut outgoing: Vec<Vec<SnapshotEdge>> = Vec::new();
    let mut initial_ids = Vec::new();
    let mut ids: HashMap<(usize, A), usize> = HashMap::new();
    let mut queue = VecDeque::new();

    for &model_id in &graph.initial_ids {
        let automaton_state = automaton.initial.clone();
        let key = (model_id, automaton_state.clone());
        let product_id = if let Some(id) = ids.get(&key).copied() {
            id
        } else {
            let id = states.len();
            ids.insert(key.clone(), id);
            states.push(BuchiProductState {
                state: graph.states[model_id].clone(),
                automaton: automaton_state,
            });
            outgoing.push(Vec::new());
            queue.push_back(key);
            id
        };
        if !initial_ids.contains(&product_id) {
            initial_ids.push(product_id);
        }
    }

    while let Some((model_id, automaton_state)) = queue.pop_front() {
        let source = ids[&(model_id, automaton_state.clone())];
        for edge in &graph.outgoing[model_id] {
            let next_automaton = (automaton.step)(&automaton_state, &edge.action);
            let key = (edge.target, next_automaton.clone());
            let target = if let Some(id) = ids.get(&key).copied() {
                id
            } else {
                let id = states.len();
                ids.insert(key.clone(), id);
                states.push(BuchiProductState {
                    state: graph.states[edge.target].clone(),
                    automaton: next_automaton,
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

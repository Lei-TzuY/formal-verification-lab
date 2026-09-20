use crate::bounded::BoundedOutcome;
use crate::checker::ExplorationLimits;
use crate::graph::{
    capture_reachable_graph_with_limits, BoundedCapturedReachableGraph, CapturedReachableGraph,
    GraphCaptureCompletion, GraphCaptureError, ReachableGraph,
};
use crate::model::{ModelError, TransitionSystem};
use crate::mu_calculus::{evaluate_captured_mu, validate_mu_formula, MuFormula, MuValidationError};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedMuStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedMuTruth {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMuInitialEvaluation<S> {
    pub state_index: usize,
    pub state: S,
    pub truth: BoundedMuTruth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMuEvaluation<S> {
    pub outcome: BoundedOutcome<BoundedMuStatus>,
    pub reachable_states: Vec<S>,
    pub definitely_satisfying_state_indices: Vec<usize>,
    pub possibly_satisfying_state_indices: Vec<usize>,
    pub initial: Vec<BoundedMuInitialEvaluation<S>>,
    pub initial_states_complete: bool,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub fixpoint_iterations: usize,
}

#[derive(Debug)]
pub enum BoundedMuError<V> {
    Validation(MuValidationError<V>),
    Model(ModelError),
    SnapshotInvariant,
}

impl<V: fmt::Debug> fmt::Display for BoundedMuError<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(f, "{error}"),
            Self::Model(error) => write!(f, "bounded mu-calculus model capture failed: {error}"),
            Self::SnapshotInvariant => {
                write!(
                    f,
                    "bounded mu-calculus reachable-graph snapshot invariant failed"
                )
            }
        }
    }
}

impl<V: fmt::Debug> std::error::Error for BoundedMuError<V> {}

impl<V> From<MuValidationError<V>> for BoundedMuError<V> {
    fn from(value: MuValidationError<V>) -> Self {
        Self::Validation(value)
    }
}

impl<V> From<ModelError> for BoundedMuError<V> {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Evaluate a validated modal mu-calculus formula over one canonical bounded
/// reachable-graph capture.
///
/// Every retained state receives a lower/upper satisfaction pair. Known
/// terminals retain the complete-graph synthetic self-loop policy; states whose
/// outgoing vector is incomplete remain semantically open. Complete captures
/// delegate back to the sealed M75 evaluator.
pub fn evaluate_mu_with_limits<S, A, V, F>(
    model: &TransitionSystem<S>,
    formula: &MuFormula<A, V>,
    atom_holds: F,
    limits: ExplorationLimits,
) -> Result<BoundedMuEvaluation<S>, BoundedMuError<V>>
where
    S: Clone + Eq + Hash,
    A: Clone,
    V: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    validate_mu_formula(formula)?;
    let expected_initials = model.initial_states().iter().collect::<HashSet<_>>().len();
    let captured = capture_reachable_graph_with_limits(model, limits).map_err(map_capture_error)?;
    let initial_states_complete = captured.graph.initial_ids.len() == expected_initials;

    if matches!(captured.completion, GraphCaptureCompletion::Complete) {
        return Ok(exact_complete_result(
            captured,
            initial_states_complete,
            formula,
            &atom_holds,
        ));
    }

    Ok(approximate_incomplete_result(
        captured,
        initial_states_complete,
        formula,
        &atom_holds,
    ))
}

fn exact_complete_result<S, A, V, F>(
    captured: BoundedCapturedReachableGraph<S>,
    initial_states_complete: bool,
    formula: &MuFormula<A, V>,
    atom_holds: &F,
) -> BoundedMuEvaluation<S>
where
    S: Clone,
    A: Clone,
    V: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    let checked_states = captured.checked_states;
    let exact = evaluate_captured_mu(
        CapturedReachableGraph {
            graph: captured.graph,
            discovered_states: captured.discovered_states,
            explored_transitions: captured.explored_transitions,
            max_depth_reached: captured.max_depth_reached,
        },
        formula,
        atom_holds,
    );

    let initial = exact
        .initial
        .iter()
        .map(|entry| BoundedMuInitialEvaluation {
            state_index: entry.state_index,
            state: entry.state.clone(),
            truth: if entry.satisfied {
                BoundedMuTruth::True
            } else {
                BoundedMuTruth::False
            },
        })
        .collect::<Vec<_>>();
    let outcome = if exact.all_initial_states_satisfy() {
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    } else {
        BoundedOutcome::Conclusive(BoundedMuStatus::Violated)
    };

    BoundedMuEvaluation {
        outcome,
        reachable_states: exact.reachable_states,
        definitely_satisfying_state_indices: exact.satisfying_state_indices.clone(),
        possibly_satisfying_state_indices: exact.satisfying_state_indices,
        initial,
        initial_states_complete,
        discovered_states: exact.discovered_states,
        checked_states,
        explored_transitions: exact.explored_transitions,
        max_depth_reached: exact.max_depth_reached,
        fixpoint_iterations: exact.fixpoint_iterations,
    }
}

fn approximate_incomplete_result<S, A, V, F>(
    captured: BoundedCapturedReachableGraph<S>,
    initial_states_complete: bool,
    formula: &MuFormula<A, V>,
    atom_holds: &F,
) -> BoundedMuEvaluation<S>
where
    S: Clone,
    A: Clone,
    V: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    let reason = match captured.completion {
        GraphCaptureCompletion::Inconclusive(reason) => reason,
        GraphCaptureCompletion::Complete => {
            unreachable!("complete bounded mu snapshots use exact_complete_result")
        }
    };
    let successor_complete = successor_completeness(&captured);
    let graph = &captured.graph;
    let mut evaluator = ApproxEvaluator {
        graph,
        known_terminal: &captured.known_terminal,
        successor_complete: &successor_complete,
        atom_holds,
        environment: HashMap::new(),
        fixpoint_iterations: 0,
        _atom: std::marker::PhantomData,
    };
    let top = evaluator.eval(formula);
    let definitely_satisfying_state_indices = indices_where(&top.lower);
    let possibly_satisfying_state_indices = indices_where(&top.upper);

    let initial = graph
        .initial_ids
        .iter()
        .map(|&state_index| BoundedMuInitialEvaluation {
            state_index,
            state: graph.states[state_index].clone(),
            truth: classify(top.lower[state_index], top.upper[state_index]),
        })
        .collect::<Vec<_>>();

    let any_definitely_false = graph.initial_ids.iter().any(|&state| !top.upper[state]);
    let all_definitely_true =
        initial_states_complete && graph.initial_ids.iter().all(|&state| top.lower[state]);
    let outcome = if any_definitely_false {
        BoundedOutcome::Conclusive(BoundedMuStatus::Violated)
    } else if all_definitely_true {
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    } else {
        BoundedOutcome::Inconclusive(reason)
    };
    let fixpoint_iterations = evaluator.fixpoint_iterations;

    BoundedMuEvaluation {
        outcome,
        reachable_states: captured.graph.states,
        definitely_satisfying_state_indices,
        possibly_satisfying_state_indices,
        initial,
        initial_states_complete,
        discovered_states: captured.discovered_states,
        checked_states: captured.checked_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        fixpoint_iterations,
    }
}

fn map_capture_error<V>(error: GraphCaptureError) -> BoundedMuError<V> {
    match error {
        GraphCaptureError::Model(error) => BoundedMuError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            BoundedMuError::SnapshotInvariant
        }
    }
}

fn successor_completeness<S>(captured: &BoundedCapturedReachableGraph<S>) -> Vec<bool> {
    captured
        .complete_enabled_actions
        .iter()
        .enumerate()
        .map(|(state, actions)| {
            captured.known_terminal[state]
                || actions
                    .as_ref()
                    .is_some_and(|actions| actions.len() == captured.graph.outgoing[state].len())
        })
        .collect()
}

fn indices_where(values: &[bool]) -> Vec<usize> {
    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect()
}

fn classify(lower: bool, upper: bool) -> BoundedMuTruth {
    if lower {
        BoundedMuTruth::True
    } else if !upper {
        BoundedMuTruth::False
    } else {
        BoundedMuTruth::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Approx {
    lower: Vec<bool>,
    upper: Vec<bool>,
}

struct ApproxEvaluator<'a, S, A, V, F> {
    graph: &'a ReachableGraph<S>,
    known_terminal: &'a [bool],
    successor_complete: &'a [bool],
    atom_holds: &'a F,
    environment: HashMap<V, Approx>,
    fixpoint_iterations: usize,
    _atom: std::marker::PhantomData<A>,
}

impl<S, A, V, F> ApproxEvaluator<'_, S, A, V, F>
where
    A: Clone,
    V: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    fn eval(&mut self, formula: &MuFormula<A, V>) -> Approx {
        let values = match formula {
            MuFormula::True => self.exact_constant(true),
            MuFormula::False => self.exact_constant(false),
            MuFormula::Atom(atom) => {
                let values = self
                    .graph
                    .states
                    .iter()
                    .map(|state| (self.atom_holds)(atom, state))
                    .collect::<Vec<_>>();
                Approx {
                    lower: values.clone(),
                    upper: values,
                }
            }
            MuFormula::Var(variable) => self
                .environment
                .get(variable)
                .expect("validated bounded mu variables are bound")
                .clone(),
            MuFormula::Not(inner) => {
                let inner = self.eval(inner);
                Approx {
                    lower: complement(&inner.upper),
                    upper: complement(&inner.lower),
                }
            }
            MuFormula::And(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                Approx {
                    lower: intersect(&left.lower, &right.lower),
                    upper: intersect(&left.upper, &right.upper),
                }
            }
            MuFormula::Or(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                Approx {
                    lower: union(&left.lower, &right.lower),
                    upper: union(&left.upper, &right.upper),
                }
            }
            MuFormula::Diamond(inner) => {
                let inner = self.eval(inner);
                self.pre_exists(&inner)
            }
            MuFormula::Box(inner) => {
                let inner = self.eval(inner);
                self.pre_all(&inner)
            }
            MuFormula::Mu { variable, body } => self.eval_fixpoint(variable, body, false),
            MuFormula::Nu { variable, body } => self.eval_fixpoint(variable, body, true),
        };

        debug_assert!(values
            .lower
            .iter()
            .zip(&values.upper)
            .all(|(lower, upper)| !*lower || *upper));
        values
    }

    fn exact_constant(&self, value: bool) -> Approx {
        Approx {
            lower: vec![value; self.graph.states.len()],
            upper: vec![value; self.graph.states.len()],
        }
    }

    fn eval_fixpoint(&mut self, variable: &V, body: &MuFormula<A, V>, greatest: bool) -> Approx {
        let previous_binding = self.environment.get(variable).cloned();
        let mut current = self.exact_constant(greatest);

        loop {
            self.environment.insert(variable.clone(), current.clone());
            self.fixpoint_iterations += 1;
            let next = self.eval(body);
            if next == current {
                match previous_binding {
                    Some(previous) => {
                        self.environment.insert(variable.clone(), previous);
                    }
                    None => {
                        self.environment.remove(variable);
                    }
                }
                return current;
            }
            current = next;
        }
    }

    fn pre_exists(&self, target: &Approx) -> Approx {
        let mut lower = vec![false; self.graph.states.len()];
        let mut upper = vec![false; self.graph.states.len()];

        for state in 0..self.graph.states.len() {
            if self.known_terminal[state] {
                lower[state] = target.lower[state];
                upper[state] = target.upper[state];
                continue;
            }

            lower[state] = self.graph.outgoing[state]
                .iter()
                .any(|edge| target.lower[edge.target]);
            upper[state] = self.graph.outgoing[state]
                .iter()
                .any(|edge| target.upper[edge.target])
                || !self.successor_complete[state];
        }

        Approx { lower, upper }
    }

    fn pre_all(&self, target: &Approx) -> Approx {
        let mut lower = vec![false; self.graph.states.len()];
        let mut upper = vec![false; self.graph.states.len()];

        for state in 0..self.graph.states.len() {
            if self.known_terminal[state] {
                lower[state] = target.lower[state];
                upper[state] = target.upper[state];
                continue;
            }

            lower[state] = self.successor_complete[state]
                && self.graph.outgoing[state]
                    .iter()
                    .all(|edge| target.lower[edge.target]);
            upper[state] = self.graph.outgoing[state]
                .iter()
                .all(|edge| target.upper[edge.target]);
        }

        Approx { lower, upper }
    }
}

fn complement(values: &[bool]) -> Vec<bool> {
    values.iter().map(|value| !value).collect()
}

fn intersect(left: &[bool], right: &[bool]) -> Vec<bool> {
    left.iter()
        .zip(right)
        .map(|(left, right)| *left && *right)
        .collect()
}

fn union(left: &[bool], right: &[bool]) -> Vec<bool> {
    left.iter()
        .zip(right)
        .map(|(left, right)| *left || *right)
        .collect()
}

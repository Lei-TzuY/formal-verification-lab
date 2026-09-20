use crate::ctl::CtlFormula;
use crate::graph::{capture_reachable_graph, GraphCaptureError, ReachableGraph};
use crate::model::{ModelError, TransitionSystem};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// Typed modal mu-calculus formula.
///
/// General negation is supported, but fixpoint variables must occur positively
/// relative to their nearest binder. The validator enforces this monotonicity
/// requirement before model exploration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MuFormula<A, V> {
    True,
    False,
    Atom(A),
    Var(V),
    Not(Box<MuFormula<A, V>>),
    And(Box<MuFormula<A, V>>, Box<MuFormula<A, V>>),
    Or(Box<MuFormula<A, V>>, Box<MuFormula<A, V>>),
    Diamond(Box<MuFormula<A, V>>),
    Box(Box<MuFormula<A, V>>),
    Mu {
        variable: V,
        body: Box<MuFormula<A, V>>,
    },
    Nu {
        variable: V,
        body: Box<MuFormula<A, V>>,
    },
}

impl<A, V> MuFormula<A, V> {
    pub fn atom(atom: A) -> Self {
        Self::Atom(atom)
    }

    pub fn var(variable: V) -> Self {
        Self::Var(variable)
    }

    pub fn negate(inner: Self) -> Self {
        Self::Not(Box::new(inner))
    }

    pub fn and(left: Self, right: Self) -> Self {
        Self::And(Box::new(left), Box::new(right))
    }

    pub fn or(left: Self, right: Self) -> Self {
        Self::Or(Box::new(left), Box::new(right))
    }

    pub fn diamond(inner: Self) -> Self {
        Self::Diamond(Box::new(inner))
    }

    pub fn boxed(inner: Self) -> Self {
        Self::Box(Box::new(inner))
    }

    pub fn mu(variable: V, body: Self) -> Self {
        Self::Mu {
            variable,
            body: Box::new(body),
        }
    }

    pub fn nu(variable: V, body: Self) -> Self {
        Self::Nu {
            variable,
            body: Box::new(body),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuTerminalPolicy {
    TotalizeWithSelfLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuInitialEvaluation<S> {
    pub state_index: usize,
    pub state: S,
    pub satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuEvaluation<S> {
    pub terminal_policy: MuTerminalPolicy,
    pub reachable_states: Vec<S>,
    pub satisfying_state_indices: Vec<usize>,
    pub initial: Vec<MuInitialEvaluation<S>>,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    /// Total number of least/greatest fixpoint body evaluations.
    pub fixpoint_iterations: usize,
}

impl<S> MuEvaluation<S> {
    pub fn all_initial_states_satisfy(&self) -> bool {
        self.initial.iter().all(|entry| entry.satisfied)
    }

    pub fn any_initial_state_satisfies(&self) -> bool {
        self.initial.iter().any(|entry| entry.satisfied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuValidationError<V> {
    UnboundVariable { variable: V },
    NonMonotoneVariable { variable: V },
}

impl<V: fmt::Debug> fmt::Display for MuValidationError<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnboundVariable { variable } => {
                write!(f, "unbound mu-calculus variable {variable:?}")
            }
            Self::NonMonotoneVariable { variable } => {
                write!(
                    f,
                    "mu-calculus variable {variable:?} occurs negatively relative to its fixpoint binder"
                )
            }
        }
    }
}

impl<V: fmt::Debug> std::error::Error for MuValidationError<V> {}

#[derive(Debug)]
pub enum MuError<V> {
    Validation(MuValidationError<V>),
    Model(ModelError),
    SnapshotInvariant,
}

impl<V: fmt::Debug> fmt::Display for MuError<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(f, "{error}"),
            Self::Model(error) => write!(f, "mu-calculus model capture failed: {error}"),
            Self::SnapshotInvariant => {
                write!(f, "mu-calculus reachable-graph snapshot invariant failed")
            }
        }
    }
}

impl<V: fmt::Debug> std::error::Error for MuError<V> {}

impl<V> From<MuValidationError<V>> for MuError<V> {
    fn from(value: MuValidationError<V>) -> Self {
        Self::Validation(value)
    }
}

impl<V> From<ModelError> for MuError<V> {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Validate lexical binding and fixpoint-variable positivity.
///
/// A variable occurrence is positive when the negation parity at the
/// occurrence matches the parity at its nearest binder. Shadowing is lexical:
/// the nearest binder with the same variable name wins.
pub fn validate_mu_formula<A, V>(formula: &MuFormula<A, V>) -> Result<(), MuValidationError<V>>
where
    V: Clone + Eq,
{
    let mut bindings = Vec::new();
    validate_formula_inner(formula, true, &mut bindings)
}

fn validate_formula_inner<A, V>(
    formula: &MuFormula<A, V>,
    positive: bool,
    bindings: &mut Vec<(V, bool)>,
) -> Result<(), MuValidationError<V>>
where
    V: Clone + Eq,
{
    match formula {
        MuFormula::True | MuFormula::False | MuFormula::Atom(_) => Ok(()),
        MuFormula::Var(variable) => {
            let Some((_, binder_polarity)) =
                bindings.iter().rev().find(|(bound, _)| bound == variable)
            else {
                return Err(MuValidationError::UnboundVariable {
                    variable: variable.clone(),
                });
            };
            if *binder_polarity == positive {
                Ok(())
            } else {
                Err(MuValidationError::NonMonotoneVariable {
                    variable: variable.clone(),
                })
            }
        }
        MuFormula::Not(inner) => validate_formula_inner(inner, !positive, bindings),
        MuFormula::And(left, right) | MuFormula::Or(left, right) => {
            validate_formula_inner(left, positive, bindings)?;
            validate_formula_inner(right, positive, bindings)
        }
        MuFormula::Diamond(inner) | MuFormula::Box(inner) => {
            validate_formula_inner(inner, positive, bindings)
        }
        MuFormula::Mu { variable, body } | MuFormula::Nu { variable, body } => {
            bindings.push((variable.clone(), positive));
            let result = validate_formula_inner(body, positive, bindings);
            bindings.pop();
            result
        }
    }
}

/// Evaluate a validated typed modal mu-calculus formula over one complete
/// canonical reachable graph.
///
/// Reachable terminal states are totalized with one synthetic self-loop for
/// modal predecessor semantics, matching the M69 CTL kernel's Kripke policy.
pub fn evaluate_mu<S, A, V, F>(
    model: &TransitionSystem<S>,
    formula: &MuFormula<A, V>,
    atom_holds: F,
) -> Result<MuEvaluation<S>, MuError<V>>
where
    S: Clone + Eq + Hash,
    A: Clone,
    V: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    validate_mu_formula(formula)?;
    let captured = capture_reachable_graph(model).map_err(map_capture_error)?;
    let graph = captured.graph;

    let mut evaluator = Evaluator {
        graph: &graph,
        atom_holds: &atom_holds,
        environment: HashMap::new(),
        fixpoint_iterations: 0,
        _atom: std::marker::PhantomData,
    };
    let satisfying = evaluator.eval(formula);
    let fixpoint_iterations = evaluator.fixpoint_iterations;

    let satisfying_state_indices = satisfying
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect::<Vec<_>>();
    let initial = graph
        .initial_ids
        .iter()
        .map(|&state_index| MuInitialEvaluation {
            state_index,
            state: graph.states[state_index].clone(),
            satisfied: satisfying[state_index],
        })
        .collect();

    Ok(MuEvaluation {
        terminal_policy: MuTerminalPolicy::TotalizeWithSelfLoop,
        reachable_states: graph.states,
        satisfying_state_indices,
        initial,
        discovered_states: captured.discovered_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        fixpoint_iterations,
    })
}

fn map_capture_error<V>(error: GraphCaptureError) -> MuError<V> {
    match error {
        GraphCaptureError::Model(error) => MuError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            MuError::SnapshotInvariant
        }
    }
}

struct Evaluator<'a, S, A, V, F> {
    graph: &'a ReachableGraph<S>,
    atom_holds: &'a F,
    environment: HashMap<V, Vec<bool>>,
    fixpoint_iterations: usize,
    _atom: std::marker::PhantomData<A>,
}

impl<S, A, V, F> Evaluator<'_, S, A, V, F>
where
    A: Clone,
    V: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    fn eval(&mut self, formula: &MuFormula<A, V>) -> Vec<bool> {
        match formula {
            MuFormula::True => vec![true; self.graph.states.len()],
            MuFormula::False => vec![false; self.graph.states.len()],
            MuFormula::Atom(atom) => self
                .graph
                .states
                .iter()
                .map(|state| (self.atom_holds)(atom, state))
                .collect(),
            MuFormula::Var(variable) => self
                .environment
                .get(variable)
                .expect("validated mu-calculus variables are bound")
                .clone(),
            MuFormula::Not(inner) => complement(&self.eval(inner)),
            MuFormula::And(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                intersect(&left, &right)
            }
            MuFormula::Or(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                union(&left, &right)
            }
            MuFormula::Diamond(inner) => {
                let inner = self.eval(inner);
                pre_exists(self.graph, &inner)
            }
            MuFormula::Box(inner) => {
                let inner = self.eval(inner);
                pre_all(self.graph, &inner)
            }
            MuFormula::Mu { variable, body } => self.eval_fixpoint(variable, body, false),
            MuFormula::Nu { variable, body } => self.eval_fixpoint(variable, body, true),
        }
    }

    fn eval_fixpoint(&mut self, variable: &V, body: &MuFormula<A, V>, greatest: bool) -> Vec<bool> {
        let previous_binding = self.environment.get(variable).cloned();
        let mut current = vec![greatest; self.graph.states.len()];

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

fn pre_exists<S>(graph: &ReachableGraph<S>, target: &[bool]) -> Vec<bool> {
    (0..graph.states.len())
        .map(|state| {
            if graph.outgoing[state].is_empty() {
                target[state]
            } else {
                graph.outgoing[state].iter().any(|edge| target[edge.target])
            }
        })
        .collect()
}

fn pre_all<S>(graph: &ReachableGraph<S>, target: &[bool]) -> Vec<bool> {
    (0..graph.states.len())
        .map(|state| {
            if graph.outgoing[state].is_empty() {
                target[state]
            } else {
                graph.outgoing[state].iter().all(|edge| target[edge.target])
            }
        })
        .collect()
}

/// Compile the complete M69 typed CTL surface into the modal mu-calculus.
///
/// Fresh numeric variables are internal to the returned formula. This function
/// is semantics-preserving over the shared complete-graph terminal policy and
/// provides an independent cross-layer differential target for CTL.
pub fn compile_ctl_to_mu<A: Clone>(formula: &CtlFormula<A>) -> MuFormula<A, usize> {
    let mut next_variable = 0usize;
    compile_ctl_inner(formula, &mut next_variable)
}

fn compile_ctl_inner<A: Clone>(
    formula: &CtlFormula<A>,
    next_variable: &mut usize,
) -> MuFormula<A, usize> {
    match formula {
        CtlFormula::True => MuFormula::True,
        CtlFormula::False => MuFormula::False,
        CtlFormula::Atom(atom) => MuFormula::Atom(atom.clone()),
        CtlFormula::Not(inner) => MuFormula::negate(compile_ctl_inner(inner, next_variable)),
        CtlFormula::And(left, right) => MuFormula::and(
            compile_ctl_inner(left, next_variable),
            compile_ctl_inner(right, next_variable),
        ),
        CtlFormula::Or(left, right) => MuFormula::or(
            compile_ctl_inner(left, next_variable),
            compile_ctl_inner(right, next_variable),
        ),
        CtlFormula::Ex(inner) => MuFormula::diamond(compile_ctl_inner(inner, next_variable)),
        CtlFormula::Ax(inner) => MuFormula::boxed(compile_ctl_inner(inner, next_variable)),
        CtlFormula::Ef(inner) => {
            let variable = fresh_variable(next_variable);
            MuFormula::mu(
                variable,
                MuFormula::or(
                    compile_ctl_inner(inner, next_variable),
                    MuFormula::diamond(MuFormula::var(variable)),
                ),
            )
        }
        CtlFormula::Af(inner) => {
            let variable = fresh_variable(next_variable);
            MuFormula::mu(
                variable,
                MuFormula::or(
                    compile_ctl_inner(inner, next_variable),
                    MuFormula::boxed(MuFormula::var(variable)),
                ),
            )
        }
        CtlFormula::Eg(inner) => {
            let variable = fresh_variable(next_variable);
            MuFormula::nu(
                variable,
                MuFormula::and(
                    compile_ctl_inner(inner, next_variable),
                    MuFormula::diamond(MuFormula::var(variable)),
                ),
            )
        }
        CtlFormula::Ag(inner) => {
            let variable = fresh_variable(next_variable);
            MuFormula::nu(
                variable,
                MuFormula::and(
                    compile_ctl_inner(inner, next_variable),
                    MuFormula::boxed(MuFormula::var(variable)),
                ),
            )
        }
        CtlFormula::Eu(left, right) => {
            let variable = fresh_variable(next_variable);
            MuFormula::mu(
                variable,
                MuFormula::or(
                    compile_ctl_inner(right, next_variable),
                    MuFormula::and(
                        compile_ctl_inner(left, next_variable),
                        MuFormula::diamond(MuFormula::var(variable)),
                    ),
                ),
            )
        }
        CtlFormula::Au(left, right) => {
            let variable = fresh_variable(next_variable);
            MuFormula::mu(
                variable,
                MuFormula::or(
                    compile_ctl_inner(right, next_variable),
                    MuFormula::and(
                        compile_ctl_inner(left, next_variable),
                        MuFormula::boxed(MuFormula::var(variable)),
                    ),
                ),
            )
        }
    }
}

fn fresh_variable(next_variable: &mut usize) -> usize {
    let variable = *next_variable;
    *next_variable += 1;
    variable
}

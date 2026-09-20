use crate::graph::{capture_reachable_graph, GraphCaptureError, ReachableGraph};
use crate::model::{ModelError, TransitionSystem};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;

/// Typed CTL state formulas.
///
/// The kernel is intentionally parser-free. Atoms are caller-defined typed
/// values and are interpreted by the evaluator callback supplied to
/// `evaluate_ctl`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CtlFormula<A> {
    True,
    False,
    Atom(A),
    Not(Box<CtlFormula<A>>),
    And(Box<CtlFormula<A>>, Box<CtlFormula<A>>),
    Or(Box<CtlFormula<A>>, Box<CtlFormula<A>>),
    Ex(Box<CtlFormula<A>>),
    Ax(Box<CtlFormula<A>>),
    Ef(Box<CtlFormula<A>>),
    Af(Box<CtlFormula<A>>),
    Eg(Box<CtlFormula<A>>),
    Ag(Box<CtlFormula<A>>),
    Eu(Box<CtlFormula<A>>, Box<CtlFormula<A>>),
    Au(Box<CtlFormula<A>>, Box<CtlFormula<A>>),
}

impl<A> CtlFormula<A> {
    pub fn atom(atom: A) -> Self {
        Self::Atom(atom)
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

    pub fn ex(inner: Self) -> Self {
        Self::Ex(Box::new(inner))
    }

    pub fn ax(inner: Self) -> Self {
        Self::Ax(Box::new(inner))
    }

    pub fn ef(inner: Self) -> Self {
        Self::Ef(Box::new(inner))
    }

    pub fn af(inner: Self) -> Self {
        Self::Af(Box::new(inner))
    }

    pub fn eg(inner: Self) -> Self {
        Self::Eg(Box::new(inner))
    }

    pub fn ag(inner: Self) -> Self {
        Self::Ag(Box::new(inner))
    }

    pub fn eu(left: Self, right: Self) -> Self {
        Self::Eu(Box::new(left), Box::new(right))
    }

    pub fn au(left: Self, right: Self) -> Self {
        Self::Au(Box::new(left), Box::new(right))
    }
}

/// CTL is evaluated on a totalized finite transition system.
///
/// A reachable terminal state receives one synthetic self-loop. This makes
/// every path infinite, matching the classic Kripke-structure CTL semantics,
/// while keeping the original model graph unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CtlTerminalPolicy {
    TotalizeWithSelfLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtlEvidenceAction {
    Model(String),
    TerminalSelfLoop,
}

/// One state in deterministic CTL evidence. The first step has `action=None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtlEvidenceStep<S> {
    pub action: Option<CtlEvidenceAction>,
    pub state: S,
}

/// Evidence is emitted only for top-level cases where this first authority
/// slice can construct it directly and soundly.
///
/// A lasso stores the repeated closing state as the last trace element.
/// `cycle_start` indexes the earlier occurrence of that same state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtlEvidence<S> {
    Finite {
        trace: Vec<CtlEvidenceStep<S>>,
    },
    Lasso {
        trace: Vec<CtlEvidenceStep<S>>,
        cycle_start: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtlInitialEvaluation<S> {
    pub state_index: usize,
    pub state: S,
    pub satisfied: bool,
    pub evidence: Option<CtlEvidence<S>>,
}

/// Complete CTL state-set result over one canonical reachable-graph snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtlEvaluation<S> {
    pub terminal_policy: CtlTerminalPolicy,
    pub reachable_states: Vec<S>,
    pub satisfying_state_indices: Vec<usize>,
    pub initial: Vec<CtlInitialEvaluation<S>>,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    /// Number of distinct typed subformulas whose state sets were materialized.
    pub memoized_subformulas: usize,
}

impl<S> CtlEvaluation<S> {
    pub fn all_initial_states_satisfy(&self) -> bool {
        self.initial.iter().all(|entry| entry.satisfied)
    }

    pub fn any_initial_state_satisfies(&self) -> bool {
        self.initial.iter().any(|entry| entry.satisfied)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtlError {
    Model(ModelError),
    SnapshotInvariant,
}

impl fmt::Display for CtlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "CTL model capture failed: {error}"),
            Self::SnapshotInvariant => {
                write!(f, "CTL reachable-graph snapshot invariant failed")
            }
        }
    }
}

impl std::error::Error for CtlError {}

impl From<ModelError> for CtlError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Evaluate one typed CTL formula over exactly one complete reachable graph.
///
/// The model transition function is invoked only by the initial canonical graph
/// capture. Every Boolean/temporal subformula is then evaluated over that
/// immutable snapshot and memoized as a state set.
pub fn evaluate_ctl<S, A, F>(
    model: &TransitionSystem<S>,
    formula: &CtlFormula<A>,
    atom_holds: F,
) -> Result<CtlEvaluation<S>, CtlError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    let captured = capture_reachable_graph(model).map_err(map_capture_error)?;
    let graph = captured.graph;
    let mut evaluator = Evaluator {
        graph: &graph,
        atom_holds: &atom_holds,
        memo: HashMap::new(),
    };

    let satisfying = evaluator.eval(formula);
    let satisfying_state_indices = satisfying
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect::<Vec<_>>();

    let mut initial = Vec::with_capacity(graph.initial_ids.len());
    for &state_index in &graph.initial_ids {
        let satisfied = satisfying[state_index];
        let evidence = explain_top_level(
            formula,
            state_index,
            satisfied,
            &satisfying,
            &mut evaluator,
        );
        initial.push(CtlInitialEvaluation {
            state_index,
            state: graph.states[state_index].clone(),
            satisfied,
            evidence,
        });
    }

    Ok(CtlEvaluation {
        terminal_policy: CtlTerminalPolicy::TotalizeWithSelfLoop,
        reachable_states: graph.states,
        satisfying_state_indices,
        initial,
        discovered_states: captured.discovered_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        memoized_subformulas: evaluator.memo.len(),
    })
}

fn map_capture_error(error: GraphCaptureError) -> CtlError {
    match error {
        GraphCaptureError::Model(error) => CtlError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            CtlError::SnapshotInvariant
        }
    }
}

struct Evaluator<'a, S, A, F> {
    graph: &'a ReachableGraph<S>,
    atom_holds: &'a F,
    memo: HashMap<CtlFormula<A>, Vec<bool>>,
}

impl<S, A, F> Evaluator<'_, S, A, F>
where
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    fn eval(&mut self, formula: &CtlFormula<A>) -> Vec<bool> {
        if let Some(cached) = self.memo.get(formula) {
            return cached.clone();
        }

        let values = match formula {
            CtlFormula::True => vec![true; self.graph.states.len()],
            CtlFormula::False => vec![false; self.graph.states.len()],
            CtlFormula::Atom(atom) => self
                .graph
                .states
                .iter()
                .map(|state| (self.atom_holds)(atom, state))
                .collect(),
            CtlFormula::Not(inner) => complement(&self.eval(inner)),
            CtlFormula::And(left, right) => {
                intersect(&self.eval(left), &self.eval(right))
            }
            CtlFormula::Or(left, right) => union(&self.eval(left), &self.eval(right)),
            CtlFormula::Ex(inner) => {
                let inner = self.eval(inner);
                pre_exists(self.graph, &inner)
            }
            CtlFormula::Ax(inner) => {
                let inner = self.eval(inner);
                pre_all(self.graph, &inner)
            }
            CtlFormula::Ef(inner) => {
                let inner = self.eval(inner);
                least_fixpoint_exists(self.graph, &inner)
            }
            CtlFormula::Af(inner) => {
                let inner = self.eval(inner);
                least_fixpoint_all(self.graph, &inner)
            }
            CtlFormula::Eg(inner) => {
                let inner = self.eval(inner);
                greatest_fixpoint_exists(self.graph, &inner)
            }
            CtlFormula::Ag(inner) => {
                let inner = self.eval(inner);
                greatest_fixpoint_all(self.graph, &inner)
            }
            CtlFormula::Eu(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                least_until_exists(self.graph, &left, &right)
            }
            CtlFormula::Au(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                least_until_all(self.graph, &left, &right)
            }
        };

        self.memo.insert(formula.clone(), values.clone());
        values
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

fn least_fixpoint_exists<S>(graph: &ReachableGraph<S>, seed: &[bool]) -> Vec<bool> {
    let mut current = seed.to_vec();
    loop {
        let previous = current.clone();
        let pre = pre_exists(graph, &previous);
        for state in 0..current.len() {
            current[state] = seed[state] || pre[state];
        }
        if current == previous {
            return current;
        }
    }
}

fn least_fixpoint_all<S>(graph: &ReachableGraph<S>, seed: &[bool]) -> Vec<bool> {
    let mut current = seed.to_vec();
    loop {
        let previous = current.clone();
        let pre = pre_all(graph, &previous);
        for state in 0..current.len() {
            current[state] = seed[state] || pre[state];
        }
        if current == previous {
            return current;
        }
    }
}

fn greatest_fixpoint_exists<S>(graph: &ReachableGraph<S>, guard: &[bool]) -> Vec<bool> {
    let mut current = vec![true; graph.states.len()];
    loop {
        let previous = current.clone();
        let pre = pre_exists(graph, &previous);
        for state in 0..current.len() {
            current[state] = guard[state] && pre[state];
        }
        if current == previous {
            return current;
        }
    }
}

fn greatest_fixpoint_all<S>(graph: &ReachableGraph<S>, guard: &[bool]) -> Vec<bool> {
    let mut current = vec![true; graph.states.len()];
    loop {
        let previous = current.clone();
        let pre = pre_all(graph, &previous);
        for state in 0..current.len() {
            current[state] = guard[state] && pre[state];
        }
        if current == previous {
            return current;
        }
    }
}

fn least_until_exists<S>(
    graph: &ReachableGraph<S>,
    left: &[bool],
    right: &[bool],
) -> Vec<bool> {
    let mut current = right.to_vec();
    loop {
        let previous = current.clone();
        let pre = pre_exists(graph, &previous);
        for state in 0..current.len() {
            current[state] = right[state] || (left[state] && pre[state]);
        }
        if current == previous {
            return current;
        }
    }
}

fn least_until_all<S>(
    graph: &ReachableGraph<S>,
    left: &[bool],
    right: &[bool],
) -> Vec<bool> {
    let mut current = right.to_vec();
    loop {
        let previous = current.clone();
        let pre = pre_all(graph, &previous);
        for state in 0..current.len() {
            current[state] = right[state] || (left[state] && pre[state]);
        }
        if current == previous {
            return current;
        }
    }
}

fn explain_top_level<S, A, F>(
    formula: &CtlFormula<A>,
    state: usize,
    satisfied: bool,
    top_values: &[bool],
    evaluator: &mut Evaluator<'_, S, A, F>,
) -> Option<CtlEvidence<S>>
where
    S: Clone,
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    match formula {
        CtlFormula::Ex(inner) if satisfied => {
            let inner = evaluator.eval(inner);
            one_step_evidence(evaluator.graph, state, &inner)
        }
        CtlFormula::Ef(inner) if satisfied => {
            let target = evaluator.eval(inner);
            finite_path_evidence(evaluator.graph, state, &vec![true; target.len()], &target)
        }
        CtlFormula::Eg(_inner) if satisfied => {
            lasso_evidence(evaluator.graph, state, top_values)
        }
        CtlFormula::Eu(left, right) if satisfied => {
            let left = evaluator.eval(left);
            let right = evaluator.eval(right);
            finite_path_evidence(evaluator.graph, state, &left, &right)
        }
        CtlFormula::Ax(inner) if !satisfied => {
            let inner = evaluator.eval(inner);
            one_step_evidence(evaluator.graph, state, &complement(&inner))
        }
        CtlFormula::Af(inner) if !satisfied => {
            let inner = evaluator.eval(inner);
            let never = greatest_fixpoint_exists(evaluator.graph, &complement(&inner));
            lasso_evidence(evaluator.graph, state, &never)
        }
        CtlFormula::Ag(inner) if !satisfied => {
            let inner = evaluator.eval(inner);
            let bad = complement(&inner);
            finite_path_evidence(
                evaluator.graph,
                state,
                &vec![true; bad.len()],
                &bad,
            )
        }
        CtlFormula::Au(left, right) if !satisfied => {
            let left = evaluator.eval(left);
            let right = evaluator.eval(right);
            let not_right = complement(&right);
            let not_left = complement(&left);
            let bad = intersect(&not_right, &not_left);
            let finite_bad = least_until_exists(evaluator.graph, &not_right, &bad);
            if finite_bad[state] {
                finite_path_evidence(evaluator.graph, state, &not_right, &bad)
            } else {
                let forever_not_right =
                    greatest_fixpoint_exists(evaluator.graph, &not_right);
                lasso_evidence(evaluator.graph, state, &forever_not_right)
            }
        }
        _ => None,
    }
}

fn one_step_evidence<S: Clone>(
    graph: &ReachableGraph<S>,
    source: usize,
    target: &[bool],
) -> Option<CtlEvidence<S>> {
    if graph.outgoing[source].is_empty() {
        if !target[source] {
            return None;
        }
        return Some(CtlEvidence::Finite {
            trace: vec![
                CtlEvidenceStep {
                    action: None,
                    state: graph.states[source].clone(),
                },
                CtlEvidenceStep {
                    action: Some(CtlEvidenceAction::TerminalSelfLoop),
                    state: graph.states[source].clone(),
                },
            ],
        });
    }

    let edge = graph.outgoing[source]
        .iter()
        .find(|edge| target[edge.target])?;
    Some(CtlEvidence::Finite {
        trace: vec![
            CtlEvidenceStep {
                action: None,
                state: graph.states[source].clone(),
            },
            CtlEvidenceStep {
                action: Some(CtlEvidenceAction::Model(edge.action.clone())),
                state: graph.states[edge.target].clone(),
            },
        ],
    })
}

#[derive(Debug, Clone)]
struct Predecessor {
    state: usize,
    action: CtlEvidenceAction,
}

fn finite_path_evidence<S: Clone>(
    graph: &ReachableGraph<S>,
    source: usize,
    allowed_before_target: &[bool],
    target: &[bool],
) -> Option<CtlEvidence<S>> {
    if target[source] {
        return Some(CtlEvidence::Finite {
            trace: vec![CtlEvidenceStep {
                action: None,
                state: graph.states[source].clone(),
            }],
        });
    }
    if !allowed_before_target[source] {
        return None;
    }

    let mut queue = VecDeque::from([source]);
    let mut seen = vec![false; graph.states.len()];
    let mut predecessor = vec![None::<Predecessor>; graph.states.len()];
    seen[source] = true;

    while let Some(state) = queue.pop_front() {
        if graph.outgoing[state].is_empty() {
            continue;
        }

        for edge in &graph.outgoing[state] {
            let next = edge.target;
            if seen[next] {
                continue;
            }
            if !target[next] && !allowed_before_target[next] {
                continue;
            }

            seen[next] = true;
            predecessor[next] = Some(Predecessor {
                state,
                action: CtlEvidenceAction::Model(edge.action.clone()),
            });

            if target[next] {
                return Some(CtlEvidence::Finite {
                    trace: reconstruct_finite_trace(graph, source, next, &predecessor),
                });
            }
            queue.push_back(next);
        }
    }

    None
}

fn reconstruct_finite_trace<S: Clone>(
    graph: &ReachableGraph<S>,
    source: usize,
    mut target: usize,
    predecessor: &[Option<Predecessor>],
) -> Vec<CtlEvidenceStep<S>> {
    let mut reversed = Vec::new();
    while target != source {
        let previous = predecessor[target]
            .as_ref()
            .expect("every retained evidence node except the source has a predecessor");
        reversed.push(CtlEvidenceStep {
            action: Some(previous.action.clone()),
            state: graph.states[target].clone(),
        });
        target = previous.state;
    }
    reversed.push(CtlEvidenceStep {
        action: None,
        state: graph.states[source].clone(),
    });
    reversed.reverse();
    reversed
}

fn lasso_evidence<S: Clone>(
    graph: &ReachableGraph<S>,
    source: usize,
    allowed: &[bool],
) -> Option<CtlEvidence<S>> {
    if !allowed[source] {
        return None;
    }

    let mut trace = vec![CtlEvidenceStep {
        action: None,
        state: graph.states[source].clone(),
    }];
    let mut positions = HashMap::new();
    positions.insert(source, 0usize);
    let mut current = source;

    loop {
        let (next, action) = if graph.outgoing[current].is_empty() {
            (current, CtlEvidenceAction::TerminalSelfLoop)
        } else {
            let edge = graph.outgoing[current]
                .iter()
                .find(|edge| allowed[edge.target])?;
            (
                edge.target,
                CtlEvidenceAction::Model(edge.action.clone()),
            )
        };

        trace.push(CtlEvidenceStep {
            action: Some(action),
            state: graph.states[next].clone(),
        });

        if let Some(&cycle_start) = positions.get(&next) {
            return Some(CtlEvidence::Lasso { trace, cycle_start });
        }

        positions.insert(next, trace.len() - 1);
        current = next;
    }
}

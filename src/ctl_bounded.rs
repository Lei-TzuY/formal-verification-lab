use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, InconclusiveReason};
use crate::ctl::{
    evaluate_captured_ctl, CtlEvidence, CtlEvidenceAction, CtlEvidenceStep, CtlFormula,
};
use crate::graph::{
    capture_reachable_graph_with_limits, BoundedCapturedReachableGraph, CapturedReachableGraph,
    GraphCaptureCompletion, GraphCaptureError, ReachableGraph,
};
use crate::model::{ModelError, TransitionSystem};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedCtlStatus {
    Satisfied,
    Violated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundedCtlTruth {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCtlInitialEvaluation<S> {
    pub state_index: usize,
    pub state: S,
    pub truth: BoundedCtlTruth,
    pub evidence: Option<CtlEvidence<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedCtlEvaluation<S> {
    pub outcome: BoundedOutcome<BoundedCtlStatus>,
    pub reachable_states: Vec<S>,
    pub definitely_satisfying_state_indices: Vec<usize>,
    pub possibly_satisfying_state_indices: Vec<usize>,
    pub initial: Vec<BoundedCtlInitialEvaluation<S>>,
    pub initial_states_complete: bool,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub memoized_subformulas: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedCtlError {
    Model(ModelError),
    SnapshotInvariant,
}

impl fmt::Display for BoundedCtlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "bounded CTL model capture failed: {error}"),
            Self::SnapshotInvariant => {
                write!(f, "bounded CTL reachable-graph snapshot invariant failed")
            }
        }
    }
}

impl std::error::Error for BoundedCtlError {}

impl From<ModelError> for BoundedCtlError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Evaluate CTL over one canonical bounded reachable-graph capture.
///
/// Incomplete successor sets are interpreted conservatively through lower/upper
/// satisfaction sets. A retained state is definitely true when it belongs to
/// the lower set, definitely false when it is absent from the upper set, and
/// otherwise unknown. Known terminals retain the M69 synthetic self-loop
/// policy; cut states with unknown outgoing behavior are never totalized.
pub fn evaluate_ctl_with_limits<S, A, F>(
    model: &TransitionSystem<S>,
    formula: &CtlFormula<A>,
    atom_holds: F,
    limits: ExplorationLimits,
) -> Result<BoundedCtlEvaluation<S>, BoundedCtlError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
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

fn exact_complete_result<S, A, F>(
    captured: BoundedCapturedReachableGraph<S>,
    initial_states_complete: bool,
    formula: &CtlFormula<A>,
    atom_holds: &F,
) -> BoundedCtlEvaluation<S>
where
    S: Clone,
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    let checked_states = captured.checked_states;
    let exact = evaluate_captured_ctl(
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
        .map(|entry| BoundedCtlInitialEvaluation {
            state_index: entry.state_index,
            state: entry.state.clone(),
            truth: if entry.satisfied {
                BoundedCtlTruth::True
            } else {
                BoundedCtlTruth::False
            },
            evidence: entry.evidence.clone(),
        })
        .collect::<Vec<_>>();
    let outcome = if exact.all_initial_states_satisfy() {
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    } else {
        BoundedOutcome::Conclusive(BoundedCtlStatus::Violated)
    };

    BoundedCtlEvaluation {
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
        memoized_subformulas: exact.memoized_subformulas,
    }
}

fn approximate_incomplete_result<S, A, F>(
    captured: BoundedCapturedReachableGraph<S>,
    initial_states_complete: bool,
    formula: &CtlFormula<A>,
    atom_holds: &F,
) -> BoundedCtlEvaluation<S>
where
    S: Clone,
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    let reason = match captured.completion {
        GraphCaptureCompletion::Inconclusive(reason) => reason,
        GraphCaptureCompletion::Complete => {
            unreachable!("complete bounded CTL snapshots use exact_complete_result")
        }
    };
    let successor_complete = successor_completeness(&captured);
    let graph = &captured.graph;
    let known_terminal = &captured.known_terminal;
    let mut evaluator = ApproxEvaluator {
        graph,
        known_terminal,
        successor_complete: &successor_complete,
        atom_holds,
        memo: HashMap::new(),
    };

    let top = evaluator.eval(formula);
    let definitely_satisfying_state_indices = indices_where(&top.lower);
    let possibly_satisfying_state_indices = indices_where(&top.upper);

    let mut initial = Vec::with_capacity(graph.initial_ids.len());
    for &state_index in &graph.initial_ids {
        let truth = classify(top.lower[state_index], top.upper[state_index]);
        let evidence = explain_bounded_top_level(formula, state_index, truth, &top, &mut evaluator);
        initial.push(BoundedCtlInitialEvaluation {
            state_index,
            state: graph.states[state_index].clone(),
            truth,
            evidence,
        });
    }

    let any_definitely_false = graph.initial_ids.iter().any(|&state| !top.upper[state]);
    let all_definitely_true =
        initial_states_complete && graph.initial_ids.iter().all(|&state| top.lower[state]);
    let outcome = if any_definitely_false {
        BoundedOutcome::Conclusive(BoundedCtlStatus::Violated)
    } else if all_definitely_true {
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    } else {
        BoundedOutcome::Inconclusive(reason)
    };

    let memoized_subformulas = evaluator.memo.len();

    BoundedCtlEvaluation {
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
        memoized_subformulas,
    }
}

fn map_capture_error(error: GraphCaptureError) -> BoundedCtlError {
    match error {
        GraphCaptureError::Model(error) => BoundedCtlError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            BoundedCtlError::SnapshotInvariant
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

fn classify(lower: bool, upper: bool) -> BoundedCtlTruth {
    if lower {
        BoundedCtlTruth::True
    } else if !upper {
        BoundedCtlTruth::False
    } else {
        BoundedCtlTruth::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Approx {
    lower: Vec<bool>,
    upper: Vec<bool>,
}

struct ApproxEvaluator<'a, S, A, F> {
    graph: &'a ReachableGraph<S>,
    known_terminal: &'a [bool],
    successor_complete: &'a [bool],
    atom_holds: &'a F,
    memo: HashMap<CtlFormula<A>, Approx>,
}

impl<S, A, F> ApproxEvaluator<'_, S, A, F>
where
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    fn eval(&mut self, formula: &CtlFormula<A>) -> Approx {
        if let Some(cached) = self.memo.get(formula) {
            return cached.clone();
        }

        let values = match formula {
            CtlFormula::True => self.exact_constant(true),
            CtlFormula::False => self.exact_constant(false),
            CtlFormula::Atom(atom) => {
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
            CtlFormula::Not(inner) => {
                let inner = self.eval(inner);
                Approx {
                    lower: complement(&inner.upper),
                    upper: complement(&inner.lower),
                }
            }
            CtlFormula::And(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                Approx {
                    lower: intersect(&left.lower, &right.lower),
                    upper: intersect(&left.upper, &right.upper),
                }
            }
            CtlFormula::Or(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                Approx {
                    lower: union(&left.lower, &right.lower),
                    upper: union(&left.upper, &right.upper),
                }
            }
            CtlFormula::Ex(inner) => {
                let inner = self.eval(inner);
                self.pre_exists(&inner)
            }
            CtlFormula::Ax(inner) => {
                let inner = self.eval(inner);
                self.pre_all(&inner)
            }
            CtlFormula::Ef(inner) => {
                let seed = self.eval(inner);
                self.least_fixpoint_exists(&seed)
            }
            CtlFormula::Af(inner) => {
                let seed = self.eval(inner);
                self.least_fixpoint_all(&seed)
            }
            CtlFormula::Eg(inner) => {
                let guard = self.eval(inner);
                self.greatest_fixpoint_exists(&guard)
            }
            CtlFormula::Ag(inner) => {
                let guard = self.eval(inner);
                self.greatest_fixpoint_all(&guard)
            }
            CtlFormula::Eu(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                self.least_until_exists(&left, &right)
            }
            CtlFormula::Au(left, right) => {
                let left = self.eval(left);
                let right = self.eval(right);
                self.least_until_all(&left, &right)
            }
        };

        debug_assert!(values
            .lower
            .iter()
            .zip(&values.upper)
            .all(|(lower, upper)| !*lower || *upper));
        self.memo.insert(formula.clone(), values.clone());
        values
    }

    fn exact_constant(&self, value: bool) -> Approx {
        Approx {
            lower: vec![value; self.graph.states.len()],
            upper: vec![value; self.graph.states.len()],
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

    fn least_fixpoint_exists(&self, seed: &Approx) -> Approx {
        let mut current = seed.clone();
        loop {
            let previous = current.clone();
            let pre = self.pre_exists(&previous);
            current = Approx {
                lower: union(&seed.lower, &pre.lower),
                upper: union(&seed.upper, &pre.upper),
            };
            if current == previous {
                return current;
            }
        }
    }

    fn least_fixpoint_all(&self, seed: &Approx) -> Approx {
        let mut current = seed.clone();
        loop {
            let previous = current.clone();
            let pre = self.pre_all(&previous);
            current = Approx {
                lower: union(&seed.lower, &pre.lower),
                upper: union(&seed.upper, &pre.upper),
            };
            if current == previous {
                return current;
            }
        }
    }

    fn greatest_fixpoint_exists(&self, guard: &Approx) -> Approx {
        let mut current = self.exact_constant(true);
        loop {
            let previous = current.clone();
            let pre = self.pre_exists(&previous);
            current = Approx {
                lower: intersect(&guard.lower, &pre.lower),
                upper: intersect(&guard.upper, &pre.upper),
            };
            if current == previous {
                return current;
            }
        }
    }

    fn greatest_fixpoint_all(&self, guard: &Approx) -> Approx {
        let mut current = self.exact_constant(true);
        loop {
            let previous = current.clone();
            let pre = self.pre_all(&previous);
            current = Approx {
                lower: intersect(&guard.lower, &pre.lower),
                upper: intersect(&guard.upper, &pre.upper),
            };
            if current == previous {
                return current;
            }
        }
    }

    fn least_until_exists(&self, left: &Approx, right: &Approx) -> Approx {
        let mut current = right.clone();
        loop {
            let previous = current.clone();
            let pre = self.pre_exists(&previous);
            current = Approx {
                lower: union(&right.lower, &intersect(&left.lower, &pre.lower)),
                upper: union(&right.upper, &intersect(&left.upper, &pre.upper)),
            };
            if current == previous {
                return current;
            }
        }
    }

    fn least_until_all(&self, left: &Approx, right: &Approx) -> Approx {
        let mut current = right.clone();
        loop {
            let previous = current.clone();
            let pre = self.pre_all(&previous);
            current = Approx {
                lower: union(&right.lower, &intersect(&left.lower, &pre.lower)),
                upper: union(&right.upper, &intersect(&left.upper, &pre.upper)),
            };
            if current == previous {
                return current;
            }
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

fn explain_bounded_top_level<S, A, F>(
    formula: &CtlFormula<A>,
    state: usize,
    truth: BoundedCtlTruth,
    top: &Approx,
    evaluator: &mut ApproxEvaluator<'_, S, A, F>,
) -> Option<CtlEvidence<S>>
where
    S: Clone,
    A: Clone + Eq + Hash,
    F: Fn(&A, &S) -> bool,
{
    match formula {
        CtlFormula::Ex(inner) if truth == BoundedCtlTruth::True => {
            let inner = evaluator.eval(inner);
            one_step_evidence(
                evaluator.graph,
                evaluator.known_terminal,
                state,
                &inner.lower,
            )
        }
        CtlFormula::Ef(inner) if truth == BoundedCtlTruth::True => {
            let inner = evaluator.eval(inner);
            let all = vec![true; inner.lower.len()];
            finite_path_evidence(evaluator.graph, state, &all, &inner.lower)
        }
        CtlFormula::Eg(_) if truth == BoundedCtlTruth::True => {
            lasso_evidence(evaluator.graph, evaluator.known_terminal, state, &top.lower)
        }
        CtlFormula::Eu(left, right) if truth == BoundedCtlTruth::True => {
            let left = evaluator.eval(left);
            let right = evaluator.eval(right);
            finite_path_evidence(evaluator.graph, state, &left.lower, &right.lower)
        }
        CtlFormula::Ax(inner) if truth == BoundedCtlTruth::False => {
            let not_inner = evaluator.eval(&CtlFormula::negate((**inner).clone()));
            one_step_evidence(
                evaluator.graph,
                evaluator.known_terminal,
                state,
                &not_inner.lower,
            )
        }
        CtlFormula::Af(inner) if truth == BoundedCtlTruth::False => {
            let dual = CtlFormula::eg(CtlFormula::negate((**inner).clone()));
            let dual = evaluator.eval(&dual);
            lasso_evidence(
                evaluator.graph,
                evaluator.known_terminal,
                state,
                &dual.lower,
            )
        }
        CtlFormula::Ag(inner) if truth == BoundedCtlTruth::False => {
            let not_inner = evaluator.eval(&CtlFormula::negate((**inner).clone()));
            let all = vec![true; not_inner.lower.len()];
            finite_path_evidence(evaluator.graph, state, &all, &not_inner.lower)
        }
        CtlFormula::Au(left, right) if truth == BoundedCtlTruth::False => {
            let not_right_formula = CtlFormula::negate((**right).clone());
            let not_left_formula = CtlFormula::negate((**left).clone());
            let bad_formula = CtlFormula::and(not_left_formula, not_right_formula.clone());
            let finite_formula = CtlFormula::eu(not_right_formula.clone(), bad_formula.clone());
            let finite = evaluator.eval(&finite_formula);
            if finite.lower[state] {
                let allowed = evaluator.eval(&not_right_formula);
                let bad = evaluator.eval(&bad_formula);
                finite_path_evidence(evaluator.graph, state, &allowed.lower, &bad.lower)
            } else {
                let infinite_formula = CtlFormula::eg(not_right_formula);
                let infinite = evaluator.eval(&infinite_formula);
                lasso_evidence(
                    evaluator.graph,
                    evaluator.known_terminal,
                    state,
                    &infinite.lower,
                )
            }
        }
        _ => None,
    }
}

fn one_step_evidence<S: Clone>(
    graph: &ReachableGraph<S>,
    known_terminal: &[bool],
    source: usize,
    target: &[bool],
) -> Option<CtlEvidence<S>> {
    if known_terminal[source] {
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
        for edge in &graph.outgoing[state] {
            let next = edge.target;
            if seen[next] || (!target[next] && !allowed_before_target[next]) {
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
            .expect("every retained bounded CTL evidence node has a predecessor");
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
    known_terminal: &[bool],
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
        let (next, action) = if known_terminal[current] {
            (current, CtlEvidenceAction::TerminalSelfLoop)
        } else {
            let edge = graph.outgoing[current]
                .iter()
                .find(|edge| allowed[edge.target])?;
            (edge.target, CtlEvidenceAction::Model(edge.action.clone()))
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

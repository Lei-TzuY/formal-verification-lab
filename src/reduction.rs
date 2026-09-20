use crate::checker::{check, CheckResult, Counterexample, TraceStep, VerificationStatus};
use crate::graph::{capture_reachable_graph, GraphCaptureError, ReachableGraph, SnapshotEdge};
use crate::model::{ModelError, TransitionSystem};
use std::collections::{BTreeSet, HashSet};
use std::fmt;
use std::hash::Hash;

/// Explicit, symmetric declaration of action pairs that a model author claims
/// are independent for a reduction experiment.
///
/// Raw declarations are not proof evidence. Use `validate_independence` to bind
/// a declaration to one complete reachable graph and its safety observations
/// before using the standalone reduced checker.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndependenceRelation {
    pairs: BTreeSet<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndependenceError {
    EmptyAction,
    ReflexiveAction { action: String },
}

impl fmt::Display for IndependenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAction => write!(f, "independence action labels must not be empty"),
            Self::ReflexiveAction { action } => {
                write!(
                    f,
                    "action '{action}' cannot be declared independent from itself"
                )
            }
        }
    }
}

impl std::error::Error for IndependenceError {}

impl IndependenceRelation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pair(
        mut self,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Result<Self, IndependenceError> {
        self.insert(left, right)?;
        Ok(self)
    }

    pub fn insert(
        &mut self,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Result<(), IndependenceError> {
        let left = left.into();
        let right = right.into();
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err(IndependenceError::EmptyAction);
        }
        if left == right {
            return Err(IndependenceError::ReflexiveAction { action: left });
        }

        let pair = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        self.pairs.insert(pair);
        Ok(())
    }

    pub fn independent(&self, left: &str, right: &str) -> bool {
        if left == right {
            return false;
        }
        let pair = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        self.pairs.contains(&(pair.0.to_owned(), pair.1.to_owned()))
    }

    pub fn pair_count(&self) -> usize {
        self.pairs.len()
    }
}

/// A proof-oriented independence certificate bound to the exact reachable
/// snapshot and invariant observations that were validated.
///
/// Fields are deliberately private: callers can only obtain this type through
/// `validate_independence`, so an unchecked `IndependenceRelation` cannot be
/// passed to the standalone reduced checker.
#[derive(Debug, Clone)]
pub struct ValidatedIndependenceRelation<S> {
    relation: IndependenceRelation,
    graph: ReachableGraph<S>,
    first_violations: Vec<Option<String>>,
    validation_explored_transitions: usize,
}

impl<S> ValidatedIndependenceRelation<S> {
    pub fn pair_count(&self) -> usize {
        self.relation.pair_count()
    }

    pub fn validation_states(&self) -> usize {
        self.graph.states.len()
    }

    pub fn validation_transitions(&self) -> usize {
        self.validation_explored_transitions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndependenceValidationError {
    Model(ModelError),
    SnapshotInvariant,
    AmbiguousAction {
        state_index: usize,
        action: String,
        successors: usize,
    },
    EnablednessChanged {
        state_index: usize,
        action: String,
        by_action: String,
        after_state_index: usize,
        expected_enabled: bool,
        observed_enabled: bool,
    },
    NonCommuting {
        state_index: usize,
        left: String,
        right: String,
        left_then_right: usize,
        right_then_left: usize,
    },
    InvariantObservationMismatch {
        state_index: usize,
        left: String,
        right: String,
        invariant: String,
        left_state_index: usize,
        right_state_index: usize,
    },
}

impl fmt::Display for IndependenceValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "model exploration failed: {error}"),
            Self::SnapshotInvariant => {
                write!(f, "canonical reachable-graph snapshot invariant failed")
            }
            Self::AmbiguousAction {
                state_index,
                action,
                successors,
            } => write!(
                f,
                "cannot validate action '{action}' at reachable state {state_index}: expected at most one successor, found {successors}"
            ),
            Self::EnablednessChanged {
                state_index,
                action,
                by_action,
                after_state_index,
                expected_enabled,
                observed_enabled,
            } => write!(
                f,
                "independence enabledness mismatch at reachable state {state_index}: action '{by_action}' leads to state {after_state_index}, where '{action}' enabled={observed_enabled} but source enabled={expected_enabled}"
            ),
            Self::NonCommuting {
                state_index,
                left,
                right,
                left_then_right,
                right_then_left,
            } => write!(
                f,
                "actions '{left}' and '{right}' do not commute at reachable state {state_index}: left-then-right reaches {left_then_right}, right-then-left reaches {right_then_left}"
            ),
            Self::InvariantObservationMismatch {
                state_index,
                left,
                right,
                invariant,
                left_state_index,
                right_state_index,
            } => write!(
                f,
                "actions '{left}' and '{right}' change safety observation order at reachable state {state_index}: invariant '{invariant}' differs between intermediate states {left_state_index} and {right_state_index}"
            ),
        }
    }
}

impl std::error::Error for IndependenceValidationError {}

impl From<ModelError> for IndependenceValidationError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

/// Validate exact action-label independence against the complete reachable
/// graph before the relation can be used as standalone reduction evidence.
///
/// This intentionally uses conservative sufficient conditions for the current
/// safety-only reducer:
/// - a configured action has at most one successor at every reachable state;
/// - either action preserves whether the other is enabled;
/// - when both are enabled, both orders reach the same state; and
/// - the two intermediate states agree on every safety invariant.
///
/// Failing any condition rejects the declaration rather than weakening the
/// proof boundary.
pub fn validate_independence<S>(
    model: &TransitionSystem<S>,
    relation: &IndependenceRelation,
) -> Result<ValidatedIndependenceRelation<S>, IndependenceValidationError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    let captured = capture_reachable_graph(model).map_err(map_validation_capture_error)?;
    for (left, right) in &relation.pairs {
        validate_pair(model, &captured.graph, left, right)?;
    }

    let first_violations = first_invariant_violations(model, &captured.graph.states);
    Ok(ValidatedIndependenceRelation {
        relation: relation.clone(),
        graph: captured.graph,
        first_violations,
        validation_explored_transitions: captured.explored_transitions,
    })
}

fn map_validation_capture_error(error: GraphCaptureError) -> IndependenceValidationError {
    match error {
        GraphCaptureError::Model(error) => IndependenceValidationError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            IndependenceValidationError::SnapshotInvariant
        }
    }
}

fn validate_pair<S>(
    model: &TransitionSystem<S>,
    graph: &ReachableGraph<S>,
    left: &str,
    right: &str,
) -> Result<(), IndependenceValidationError> {
    for state_index in 0..graph.states.len() {
        let left_target = unique_action_target(&graph.outgoing[state_index], state_index, left)?;
        let right_target = unique_action_target(&graph.outgoing[state_index], state_index, right)?;

        let right_after_left = if let Some(left_state) = left_target {
            let observed =
                unique_action_target(&graph.outgoing[left_state], left_state, right)?;
            if right_target.is_some() != observed.is_some() {
                return Err(IndependenceValidationError::EnablednessChanged {
                    state_index,
                    action: right.to_owned(),
                    by_action: left.to_owned(),
                    after_state_index: left_state,
                    expected_enabled: right_target.is_some(),
                    observed_enabled: observed.is_some(),
                });
            }
            observed
        } else {
            None
        };

        let left_after_right = if let Some(right_state) = right_target {
            let observed =
                unique_action_target(&graph.outgoing[right_state], right_state, left)?;
            if left_target.is_some() != observed.is_some() {
                return Err(IndependenceValidationError::EnablednessChanged {
                    state_index,
                    action: left.to_owned(),
                    by_action: right.to_owned(),
                    after_state_index: right_state,
                    expected_enabled: left_target.is_some(),
                    observed_enabled: observed.is_some(),
                });
            }
            observed
        } else {
            None
        };

        if let (Some(left_state), Some(right_state)) = (left_target, right_target) {
            let left_then_right = right_after_left
                .expect("enabledness preservation requires right after left");
            let right_then_left = left_after_right
                .expect("enabledness preservation requires left after right");
            if left_then_right != right_then_left {
                return Err(IndependenceValidationError::NonCommuting {
                    state_index,
                    left: left.to_owned(),
                    right: right.to_owned(),
                    left_then_right,
                    right_then_left,
                });
            }

            for invariant in model.invariants() {
                if invariant.holds(&graph.states[left_state])
                    != invariant.holds(&graph.states[right_state])
                {
                    return Err(
                        IndependenceValidationError::InvariantObservationMismatch {
                            state_index,
                            left: left.to_owned(),
                            right: right.to_owned(),
                            invariant: invariant.name().to_owned(),
                            left_state_index: left_state,
                            right_state_index: right_state,
                        },
                    );
                }
            }
        }
    }
    Ok(())
}

fn unique_action_target(
    edges: &[SnapshotEdge],
    state_index: usize,
    action: &str,
) -> Result<Option<usize>, IndependenceValidationError> {
    let mut target = None;
    let mut successors = 0usize;
    for edge in edges.iter().filter(|edge| edge.action == action) {
        successors += 1;
        if target.is_none() {
            target = Some(edge.target);
        }
    }
    if successors > 1 {
        return Err(IndependenceValidationError::AmbiguousAction {
            state_index,
            action: action.to_owned(),
            successors,
        });
    }
    Ok(target)
}

fn first_invariant_violations<S>(
    model: &TransitionSystem<S>,
    states: &[S],
) -> Vec<Option<String>> {
    states
        .iter()
        .map(|state| {
            model.invariants().iter().find_map(|invariant| {
                (!invariant.holds(state)).then(|| invariant.name().to_owned())
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReducedExploration<S> {
    pub status: VerificationStatus,
    pub discovered_states: usize,
    pub checked_states: usize,
    pub explored_transitions: usize,
    pub pruned_transitions: usize,
    pub counterexample: Option<Counterexample<S>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReductionAudit<S> {
    pub exhaustive: CheckResult<S>,
    pub reduced: ReducedExploration<S>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReductionAuditError {
    Model(ModelError),
    SemanticMismatch {
        exhaustive: VerificationStatus,
        reduced: VerificationStatus,
    },
}

impl fmt::Display for ReductionAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "model exploration failed: {error}"),
            Self::SemanticMismatch {
                exhaustive,
                reduced,
            } => write!(
                f,
                "reduction semantic mismatch: exhaustive={exhaustive:?}, reduced={reduced:?}"
            ),
        }
    }
}

impl std::error::Error for ReductionAuditError {}

impl From<ModelError> for ReductionAuditError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

#[derive(Debug)]
struct ReducedNode<S> {
    state: S,
    predecessor: Option<usize>,
    action: Option<String>,
}

struct ReducedContext<'a, S> {
    model: &'a TransitionSystem<S>,
    relation: &'a IndependenceRelation,
    nodes: Vec<ReducedNode<S>>,
    seen_contexts: HashSet<(S, Vec<String>)>,
    discovered_states: HashSet<S>,
    checked_states: usize,
    explored_transitions: usize,
    pruned_transitions: usize,
}

/// Run the historical experimental reduction and compare its safety status
/// with the canonical exhaustive checker.
///
/// Raw independence declarations remain untrusted here. The comparison is kept
/// for backward compatibility and as a differential audit. New proof-producing
/// callers should use `validate_independence` followed by
/// `check_validated_sleep_set_reduction`.
pub fn audit_sleep_set_reduction<S>(
    model: &TransitionSystem<S>,
    relation: &IndependenceRelation,
) -> Result<ReductionAudit<S>, ReductionAuditError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    let exhaustive = check(model)?;
    let reduced = sleep_set_exploration(model, relation)?;

    if exhaustive.status != reduced.status {
        return Err(ReductionAuditError::SemanticMismatch {
            exhaustive: exhaustive.status,
            reduced: reduced.status,
        });
    }

    Ok(ReductionAudit {
        exhaustive,
        reduced,
    })
}

/// Run safety reduction from a validated certificate without performing a
/// second exhaustive status comparison.
///
/// The certificate owns the exact reachable snapshot and invariant
/// observations used during validation, so it cannot accidentally be reused
/// as evidence for a different transition system.
pub fn check_validated_sleep_set_reduction<S>(
    validated: &ValidatedIndependenceRelation<S>,
) -> ReducedExploration<S>
where
    S: Clone,
{
    reduce_validated_snapshot(validated)
}

fn sleep_set_exploration<S>(
    model: &TransitionSystem<S>,
    relation: &IndependenceRelation,
) -> Result<ReducedExploration<S>, ModelError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    let mut context = ReducedContext {
        model,
        relation,
        nodes: Vec::new(),
        seen_contexts: HashSet::new(),
        discovered_states: HashSet::new(),
        checked_states: 0,
        explored_transitions: 0,
        pruned_transitions: 0,
    };

    for initial in model.initial_states() {
        let sleep = BTreeSet::new();
        let key = (initial.clone(), sleep_key(&sleep));
        if !context.seen_contexts.insert(key) {
            continue;
        }

        let node_id = context.nodes.len();
        context.nodes.push(ReducedNode {
            state: initial.clone(),
            predecessor: None,
            action: None,
        });
        context.discovered_states.insert(initial.clone());

        if let Some(counterexample) = explore_reduced(node_id, &sleep, &mut context)? {
            return Ok(reduced_violation(
                context.discovered_states.len(),
                context.checked_states,
                context.explored_transitions,
                context.pruned_transitions,
                counterexample,
            ));
        }
    }

    Ok(ReducedExploration {
        status: VerificationStatus::Safe,
        discovered_states: context.discovered_states.len(),
        checked_states: context.checked_states,
        explored_transitions: context.explored_transitions,
        pruned_transitions: context.pruned_transitions,
        counterexample: None,
    })
}

fn explore_reduced<S>(
    node_id: usize,
    sleep: &BTreeSet<String>,
    context: &mut ReducedContext<'_, S>,
) -> Result<Option<Counterexample<S>>, ModelError>
where
    S: Clone + Eq + Hash + fmt::Debug,
{
    context.checked_states += 1;

    for invariant in context.model.invariants() {
        if !invariant.holds(&context.nodes[node_id].state) {
            return Ok(Some(Counterexample {
                invariant: invariant.name().to_owned(),
                trace: reconstruct_reduced_trace(&context.nodes, node_id),
            }));
        }
    }

    let transitions = context.model.successors(&context.nodes[node_id].state)?;
    let mut earlier_enabled = Vec::<String>::new();

    for transition in transitions {
        let action = transition.action.clone();
        if sleep.contains(&action) {
            context.pruned_transitions += 1;
            earlier_enabled.push(action);
            continue;
        }

        context.explored_transitions += 1;
        let next_sleep = successor_sleep_set(sleep, &earlier_enabled, &action, context.relation);
        let key = (transition.next.clone(), sleep_key(&next_sleep));
        if !context.seen_contexts.insert(key) {
            earlier_enabled.push(action);
            continue;
        }

        let next_id = context.nodes.len();
        context.nodes.push(ReducedNode {
            state: transition.next.clone(),
            predecessor: Some(node_id),
            action: Some(action.clone()),
        });
        context.discovered_states.insert(transition.next);

        if let Some(counterexample) = explore_reduced(next_id, &next_sleep, context)? {
            return Ok(Some(counterexample));
        }

        earlier_enabled.push(action);
    }

    Ok(None)
}

#[derive(Debug)]
struct SnapshotReducedNode {
    state_id: usize,
    predecessor: Option<usize>,
    action: Option<String>,
}

struct SnapshotReducedContext<'a, S> {
    validated: &'a ValidatedIndependenceRelation<S>,
    nodes: Vec<SnapshotReducedNode>,
    seen_contexts: HashSet<(usize, Vec<String>)>,
    discovered_states: HashSet<usize>,
    checked_states: usize,
    explored_transitions: usize,
    pruned_transitions: usize,
}

fn reduce_validated_snapshot<S>(
    validated: &ValidatedIndependenceRelation<S>,
) -> ReducedExploration<S>
where
    S: Clone,
{
    let mut context = SnapshotReducedContext {
        validated,
        nodes: Vec::new(),
        seen_contexts: HashSet::new(),
        discovered_states: HashSet::new(),
        checked_states: 0,
        explored_transitions: 0,
        pruned_transitions: 0,
    };

    for &initial_id in &validated.graph.initial_ids {
        let sleep = BTreeSet::new();
        if !context
            .seen_contexts
            .insert((initial_id, sleep_key(&sleep)))
        {
            continue;
        }

        let node_id = context.nodes.len();
        context.nodes.push(SnapshotReducedNode {
            state_id: initial_id,
            predecessor: None,
            action: None,
        });
        context.discovered_states.insert(initial_id);

        if let Some(counterexample) = explore_validated_snapshot(node_id, &sleep, &mut context) {
            return reduced_violation(
                context.discovered_states.len(),
                context.checked_states,
                context.explored_transitions,
                context.pruned_transitions,
                counterexample,
            );
        }
    }

    ReducedExploration {
        status: VerificationStatus::Safe,
        discovered_states: context.discovered_states.len(),
        checked_states: context.checked_states,
        explored_transitions: context.explored_transitions,
        pruned_transitions: context.pruned_transitions,
        counterexample: None,
    }
}

fn explore_validated_snapshot<S>(
    node_id: usize,
    sleep: &BTreeSet<String>,
    context: &mut SnapshotReducedContext<'_, S>,
) -> Option<Counterexample<S>>
where
    S: Clone,
{
    context.checked_states += 1;
    let state_id = context.nodes[node_id].state_id;

    if let Some(invariant) = &context.validated.first_violations[state_id] {
        return Some(Counterexample {
            invariant: invariant.clone(),
            trace: reconstruct_snapshot_trace(
                &context.nodes,
                &context.validated.graph.states,
                node_id,
            ),
        });
    }

    let transitions = context.validated.graph.outgoing[state_id].clone();
    let mut earlier_enabled = Vec::<String>::new();

    for transition in transitions {
        let action = transition.action.clone();
        if sleep.contains(&action) {
            context.pruned_transitions += 1;
            earlier_enabled.push(action);
            continue;
        }

        context.explored_transitions += 1;
        let next_sleep = successor_sleep_set(
            sleep,
            &earlier_enabled,
            &action,
            &context.validated.relation,
        );
        if !context
            .seen_contexts
            .insert((transition.target, sleep_key(&next_sleep)))
        {
            earlier_enabled.push(action);
            continue;
        }

        let next_id = context.nodes.len();
        context.nodes.push(SnapshotReducedNode {
            state_id: transition.target,
            predecessor: Some(node_id),
            action: Some(action.clone()),
        });
        context.discovered_states.insert(transition.target);

        if let Some(counterexample) =
            explore_validated_snapshot(next_id, &next_sleep, context)
        {
            return Some(counterexample);
        }

        earlier_enabled.push(action);
    }

    None
}

fn successor_sleep_set(
    sleep: &BTreeSet<String>,
    earlier_enabled: &[String],
    action: &str,
    relation: &IndependenceRelation,
) -> BTreeSet<String> {
    let mut next_sleep = BTreeSet::new();
    for sleeping_action in sleep {
        if relation.independent(action, sleeping_action) {
            next_sleep.insert(sleeping_action.clone());
        }
    }
    for earlier_action in earlier_enabled {
        if relation.independent(action, earlier_action) {
            next_sleep.insert(earlier_action.clone());
        }
    }
    next_sleep
}

fn sleep_key(sleep: &BTreeSet<String>) -> Vec<String> {
    sleep.iter().cloned().collect()
}

fn reduced_violation<S>(
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    pruned_transitions: usize,
    counterexample: Counterexample<S>,
) -> ReducedExploration<S> {
    ReducedExploration {
        status: VerificationStatus::Violated,
        discovered_states,
        checked_states,
        explored_transitions,
        pruned_transitions,
        counterexample: Some(counterexample),
    }
}

fn reconstruct_reduced_trace<S: Clone>(
    nodes: &[ReducedNode<S>],
    mut node_id: usize,
) -> Vec<TraceStep<S>> {
    let mut reversed = Vec::new();
    loop {
        let node = &nodes[node_id];
        reversed.push(TraceStep {
            action: node.action.clone(),
            state: node.state.clone(),
        });
        match node.predecessor {
            Some(predecessor) => node_id = predecessor,
            None => break,
        }
    }
    reversed.reverse();
    reversed
}

fn reconstruct_snapshot_trace<S: Clone>(
    nodes: &[SnapshotReducedNode],
    states: &[S],
    mut node_id: usize,
) -> Vec<TraceStep<S>> {
    let mut reversed = Vec::new();
    loop {
        let node = &nodes[node_id];
        reversed.push(TraceStep {
            action: node.action.clone(),
            state: states[node.state_id].clone(),
        });
        match node.predecessor {
            Some(predecessor) => node_id = predecessor,
            None => break,
        }
    }
    reversed.reverse();
    reversed
}

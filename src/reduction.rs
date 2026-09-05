use crate::checker::{check, CheckResult, Counterexample, TraceStep, VerificationStatus};
use crate::model::{ModelError, TransitionSystem};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::hash::Hash;

/// Explicit, symmetric declaration of action pairs that a model author claims
/// are independent for a reduction experiment.
///
/// The relation is never trusted as proof evidence by itself. The public audit
/// API always compares reduced exploration with the canonical exhaustive
/// checker and fails closed on a semantic mismatch.
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
    node_by_state: HashMap<S, usize>,
    checked_states: usize,
    explored_transitions: usize,
    pruned_transitions: usize,
}

/// Run an experimental sleep-set reduction and compare its safety status with
/// the canonical exhaustive checker.
///
/// This API is intentionally an audit, not a standalone proof backend. A bad
/// independence declaration can make a reduction unsound; when the reduced
/// status differs from exhaustive exploration this function returns
/// `SemanticMismatch` instead of exposing the reduced result as a proof.
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
        node_by_state: HashMap::new(),
        checked_states: 0,
        explored_transitions: 0,
        pruned_transitions: 0,
    };

    for initial in model.initial_states() {
        if context.node_by_state.contains_key(initial) {
            continue;
        }
        let node_id = context.nodes.len();
        context.nodes.push(ReducedNode {
            state: initial.clone(),
            predecessor: None,
            action: None,
        });
        context.node_by_state.insert(initial.clone(), node_id);

        if let Some(counterexample) = explore_reduced(node_id, &BTreeSet::new(), &mut context)? {
            return Ok(ReducedExploration {
                status: VerificationStatus::Violated,
                discovered_states: context.nodes.len(),
                checked_states: context.checked_states,
                explored_transitions: context.explored_transitions,
                pruned_transitions: context.pruned_transitions,
                counterexample: Some(counterexample),
            });
        }
    }

    Ok(ReducedExploration {
        status: VerificationStatus::Safe,
        discovered_states: context.nodes.len(),
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
        if context.node_by_state.contains_key(&transition.next) {
            earlier_enabled.push(action);
            continue;
        }

        let mut next_sleep = BTreeSet::new();
        for sleeping_action in sleep {
            if context.relation.independent(&action, sleeping_action) {
                next_sleep.insert(sleeping_action.clone());
            }
        }
        for earlier_action in &earlier_enabled {
            if context.relation.independent(&action, earlier_action) {
                next_sleep.insert(earlier_action.clone());
            }
        }

        let next_id = context.nodes.len();
        context.nodes.push(ReducedNode {
            state: transition.next.clone(),
            predecessor: Some(node_id),
            action: Some(action.clone()),
        });
        context.node_by_state.insert(transition.next, next_id);

        if let Some(counterexample) = explore_reduced(next_id, &next_sleep, context)? {
            return Ok(Some(counterexample));
        }

        earlier_enabled.push(action);
    }

    Ok(None)
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

use crate::bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
use crate::bounded_fairness::{
    check_buchi_with_weak_fairness_and_limits, check_buchi_with_weak_fairness_and_product_limits,
};
use crate::bounded_strong_fairness::{
    check_buchi_with_strong_fairness_and_limits,
    check_buchi_with_strong_fairness_and_product_limits,
};
use crate::buchi::{
    AcceptanceSet, AnalysisBuchiResult, BoundedBuchiResult, BuchiAutomaton, BuchiCounterexample,
    BuchiError, BuchiProductState, BuchiResult, BuchiStatus, FiniteRunPolicy,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::fairness::{check_buchi_with_weak_fairness, WeakFairness};
use crate::graph::{capture_reachable_graph, induced_graph, shortest_path, ReachableGraph};
use crate::model::TransitionSystem;
use crate::product::{
    build_action_product, build_action_product_with_analysis_limits,
    build_action_product_with_limits, BoundedActionProduct, StagedActionProduct,
};
use crate::recurrence::{
    component_is_cyclic, cycle_witness, strongly_connected_components, RecurrenceError,
};
use crate::strong_fairness::{check_buchi_with_strong_fairness, StrongFairness};
use std::collections::HashSet;
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
                &self
                    .clauses
                    .iter()
                    .map(|clause| &clause.name)
                    .collect::<Vec<_>>(),
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

/// Multi-response verification under deterministic product-space limits.
///
/// The model graph is captured exhaustively before these limits are applied;
/// the accounting below therefore describes only action-product construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedMultiResponseResult<S> {
    pub property: String,
    pub outcome: BoundedOutcome<MultiResponseStatus>,
    pub model_states: usize,
    pub model_transitions: usize,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
    pub clause_count: usize,
    pub counterexample: Option<MultiResponseCounterexample<S>>,
}

/// Multi-response verification under independently configured model-capture and
/// product-construction limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisMultiResponseResult<S> {
    pub property: String,
    pub outcome: AnalysisOutcome<MultiResponseStatus>,
    pub model_completion: BoundedOutcome<()>,
    pub product_completion: BoundedOutcome<()>,
    pub model_states: usize,
    pub checked_model_states: usize,
    pub explored_model_transitions: usize,
    pub retained_model_transitions: usize,
    pub max_model_depth_reached: Option<usize>,
    pub product_states: usize,
    pub checked_product_states: usize,
    pub explored_product_transitions: usize,
    pub retained_product_transitions: usize,
    pub max_product_depth_reached: Option<usize>,
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
    Buchi(BuchiError),
    MissingFiniteWitness,
    MissingCycleWitness,
    FairnessAdapterInvariant,
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
            Self::Buchi(error) => write!(f, "fair multi-response analysis failed: {error}"),
            Self::MissingFiniteWitness => write!(
                f,
                "pending terminal product state did not yield a counterexample trace"
            ),
            Self::MissingCycleWitness => write!(
                f,
                "pending cyclic product component did not yield a stem-plus-cycle counterexample"
            ),
            Self::FairnessAdapterInvariant => write!(
                f,
                "fair multi-response adapter observed inconsistent clause state"
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

impl From<BuchiError> for MultiResponseError {
    fn from(value: BuchiError) -> Self {
        Self::Buchi(value)
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
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let initial_pending = vec![false; property.clauses.len()];
    let product = build_action_product(
        &captured.graph,
        &initial_pending,
        |pending, action| next_pending(property, pending, action),
        |state, pending| MultiObligationState { state, pending },
    );
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let known_terminal = product
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let counterexample = find_counterexample(&product, &known_terminal, property)?;

    Ok(MultiResponseResult {
        property: property.name.clone(),
        status: if counterexample.is_some() {
            MultiResponseStatus::Violated
        } else {
            MultiResponseStatus::Satisfied
        },
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        clause_count: property.clauses.len(),
        counterexample,
    })
}

/// Verify multi-response semantics while bounding only deterministic action-
/// product construction.
///
/// A real pending terminal or closed pending cycle contained in the retained
/// prefix is sufficient to prove `Violated`, even if the product build later
/// reaches a resource limit. If no such witness is established, satisfaction is
/// returned only when product construction completes; otherwise the exact
/// product-space cutoff is returned as `Inconclusive`.
///
/// Because model capture remains exhaustive in this milestone slice, these
/// limits must not be described as whole-analysis resource bounds.
pub fn check_multi_response_with_product_limits<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    limits: ExplorationLimits,
) -> Result<BoundedMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let initial_pending = vec![false; property.clauses.len()];
    let BoundedActionProduct {
        graph: product,
        checked_states,
        explored_transitions,
        max_depth_reached,
        completion,
        known_terminal,
    } = build_action_product_with_limits(
        &captured.graph,
        &initial_pending,
        |pending, action| next_pending(property, pending, action),
        |state, pending| MultiObligationState { state, pending },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let counterexample = find_counterexample(&product, &known_terminal, property)?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated)
    } else {
        match completion {
            BoundedOutcome::Conclusive(()) => {
                BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied)
            }
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedMultiResponseResult {
        property: property.name.clone(),
        outcome,
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        checked_product_states: checked_states,
        explored_product_transitions: explored_transitions,
        retained_product_transitions,
        max_product_depth_reached: max_depth_reached,
        clause_count: property.clauses.len(),
        counterexample,
    })
}

/// Verify multi-response semantics under a staged whole-analysis envelope.
///
/// Model limits are applied first by the canonical bounded BFS; product limits
/// then apply to the justified retained model prefix. A real pending terminal or
/// real closed pending cycle is conclusive even if either stage is incomplete.
/// Without such a witness, `SATISFIED` requires both stages to complete.
///
/// If both stages are incomplete and no conclusive witness exists, the overall
/// reason deterministically reports the earlier model stage. Both stage-specific
/// completions remain available in the returned result.
pub fn check_multi_response_with_limits<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    limits: AnalysisLimits,
) -> Result<AnalysisMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    let initial_pending = vec![false; property.clauses.len()];
    let StagedActionProduct {
        product:
            BoundedActionProduct {
                graph: product,
                checked_states: checked_product_states,
                explored_transitions: explored_product_transitions,
                max_depth_reached: max_product_depth_reached,
                completion: product_completion,
                known_terminal,
            },
        model_discovered_states,
        model_checked_states,
        model_explored_transitions,
        model_retained_transitions,
        model_max_depth_reached,
        model_completion,
    } = build_action_product_with_analysis_limits(
        model,
        &initial_pending,
        |pending, action| next_pending(property, pending, action),
        |state, pending| MultiObligationState { state, pending },
        limits,
    )
    .map_err(RecurrenceError::from)?;

    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let counterexample = find_counterexample(&product, &known_terminal, property)?;
    let outcome = analysis_outcome(
        counterexample.is_some(),
        &model_completion,
        &product_completion,
    );

    Ok(AnalysisMultiResponseResult {
        property: property.name.clone(),
        outcome,
        model_completion,
        product_completion,
        model_states: model_discovered_states,
        checked_model_states: model_checked_states,
        explored_model_transitions: model_explored_transitions,
        retained_model_transitions: model_retained_transitions,
        max_model_depth_reached: model_max_depth_reached,
        product_states: product.states.len(),
        checked_product_states,
        explored_product_transitions,
        retained_product_transitions,
        max_product_depth_reached,
        clause_count: property.clauses.len(),
        counterexample,
    })
}

/// Verify the conjunction of response clauses over executions admitted by an
/// explicit exact-action weak-fairness contract.
///
/// Non-empty fairness is compiled to one generalized Buchi acceptance set per
/// clause: clause `i` accepts exactly when its pending bit is false. This keeps
/// infinite failure per clause rather than collapsing all pending bits into one
/// condition. Finite terminals use the strict accepting-terminal policy, so a
/// finite unanswered obligation remains a violation. Empty fairness delegates
/// exactly to the historical M11 engine.
pub fn check_multi_response_with_weak_fairness<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    fairness: &WeakFairness,
) -> Result<MultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_multi_response(model, property);
    }

    let automaton = fair_multi_response_automaton(property)?;
    let result = check_buchi_with_weak_fairness(model, &automaton, fairness)?;
    normalize_fair_buchi_result(property, result)
}

/// Verify the conjunction of response clauses over executions admitted by an
/// explicit exact-action strong-fairness contract.
///
/// Clause obligations retain one generalized Buchi acceptance set each. Strong
/// fairness constrains only infinite executions, so strict accepting-terminal
/// semantics continue to report any real finite pending terminal as a violation.
/// Empty strong fairness delegates exactly to the historical M11 engine.
pub fn check_multi_response_with_strong_fairness<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    fairness: &StrongFairness,
) -> Result<MultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_multi_response(model, property);
    }

    let automaton = fair_multi_response_automaton(property)?;
    let result = check_buchi_with_strong_fairness(model, &automaton, fairness)?;
    normalize_fair_buchi_result(property, result)
}

/// Product-bounded weak-fair multi-response verification after complete model
/// capture. Unknown product work remains `INCONCLUSIVE` unless a real weakly
/// fair violating terminal/cycle is already justified by the retained prefix.
pub fn check_multi_response_with_weak_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_multi_response_with_product_limits(model, property, limits);
    }

    let automaton = fair_multi_response_automaton(property)?;
    let result =
        check_buchi_with_weak_fairness_and_product_limits(model, &automaton, fairness, limits)?;
    normalize_fair_bounded_buchi_result(property, result)
}

/// Product-bounded strong-fair multi-response verification after complete model
/// capture. Strong-fair enablement authority and retained-cycle honesty are
/// inherited from the M39 bounded strong-fair Buchi engine.
pub fn check_multi_response_with_strong_fairness_and_product_limits<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    fairness: &StrongFairness,
    limits: ExplorationLimits,
) -> Result<BoundedMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_multi_response_with_product_limits(model, property, limits);
    }

    let automaton = fair_multi_response_automaton(property)?;
    let result =
        check_buchi_with_strong_fairness_and_product_limits(model, &automaton, fairness, limits)?;
    normalize_fair_bounded_buchi_result(property, result)
}

/// Staged model/product-bounded weak-fair multi-response verification. Fairness
/// enablement provenance is inherited from the M33 bounded fair Buchi engine;
/// unknown enablement can never be treated as proof that a fair action is
/// disabled.
pub fn check_multi_response_with_weak_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    fairness: &WeakFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_multi_response_with_limits(model, property, limits);
    }

    let automaton = fair_multi_response_automaton(property)?;
    let result = check_buchi_with_weak_fairness_and_limits(model, &automaton, fairness, limits)?;
    normalize_fair_analysis_buchi_result(property, result)
}

/// Staged model/product-bounded strong-fair multi-response verification. M39's
/// conservative enablement provenance is retained, so missing prefix edges can
/// never prove that an intermittently enabled strong-fair action is disabled.
pub fn check_multi_response_with_strong_fairness_and_limits<S>(
    model: &TransitionSystem<S>,
    property: &MultiResponseProperty,
    fairness: &StrongFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisMultiResponseResult<S>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_multi_response_with_limits(model, property, limits);
    }

    let automaton = fair_multi_response_automaton(property)?;
    let result = check_buchi_with_strong_fairness_and_limits(model, &automaton, fairness, limits)?;
    normalize_fair_analysis_buchi_result(property, result)
}

fn fair_multi_response_automaton(
    property: &MultiResponseProperty,
) -> Result<BuchiAutomaton<Vec<bool>>, MultiResponseError> {
    let mut acceptance = Vec::with_capacity(property.clauses.len());
    for (index, clause) in property.clauses.iter().enumerate() {
        acceptance.push(AcceptanceSet::new(
            clause.name.clone(),
            move |pending: &Vec<bool>| !pending[index],
        )?);
    }

    let name = format!("{}-fair-response", property.name);
    let initial = vec![false; property.clauses.len()];
    let step_property = property.clone();
    Ok(BuchiAutomaton::new(
        name,
        initial,
        move |pending, action| next_pending(&step_property, pending, action),
        acceptance,
        FiniteRunPolicy::RequireAcceptingTerminal,
    )?)
}

fn normalize_fair_buchi_result<S>(
    property: &MultiResponseProperty,
    result: BuchiResult<S, Vec<bool>>,
) -> Result<MultiResponseResult<S>, MultiResponseError> {
    Ok(MultiResponseResult {
        property: property.name.clone(),
        status: map_buchi_status(result.status),
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        product_transitions: result.product_transitions,
        clause_count: property.clauses.len(),
        counterexample: collapse_fair_buchi_counterexample(property, result.counterexample)?,
    })
}

fn normalize_fair_bounded_buchi_result<S>(
    property: &MultiResponseProperty,
    result: BoundedBuchiResult<S, Vec<bool>>,
) -> Result<BoundedMultiResponseResult<S>, MultiResponseError> {
    Ok(BoundedMultiResponseResult {
        property: property.name.clone(),
        outcome: match result.outcome {
            BoundedOutcome::Conclusive(status) => {
                BoundedOutcome::Conclusive(map_buchi_status(status))
            }
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        },
        model_states: result.model_states,
        model_transitions: result.model_transitions,
        product_states: result.product_states,
        checked_product_states: result.checked_product_states,
        explored_product_transitions: result.explored_product_transitions,
        retained_product_transitions: result.retained_product_transitions,
        max_product_depth_reached: result.max_product_depth_reached,
        clause_count: property.clauses.len(),
        counterexample: collapse_fair_buchi_counterexample(property, result.counterexample)?,
    })
}

fn normalize_fair_analysis_buchi_result<S>(
    property: &MultiResponseProperty,
    result: AnalysisBuchiResult<S, Vec<bool>>,
) -> Result<AnalysisMultiResponseResult<S>, MultiResponseError> {
    Ok(AnalysisMultiResponseResult {
        property: property.name.clone(),
        outcome: match result.outcome {
            AnalysisOutcome::Conclusive(status) => {
                AnalysisOutcome::Conclusive(map_buchi_status(status))
            }
            AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(reason),
        },
        model_completion: result.model_completion,
        product_completion: result.product_completion,
        model_states: result.model_states,
        checked_model_states: result.checked_model_states,
        explored_model_transitions: result.explored_model_transitions,
        retained_model_transitions: result.retained_model_transitions,
        max_model_depth_reached: result.max_model_depth_reached,
        product_states: result.product_states,
        checked_product_states: result.checked_product_states,
        explored_product_transitions: result.explored_product_transitions,
        retained_product_transitions: result.retained_product_transitions,
        max_product_depth_reached: result.max_product_depth_reached,
        clause_count: property.clauses.len(),
        counterexample: collapse_fair_buchi_counterexample(property, result.counterexample)?,
    })
}

fn map_buchi_status(status: BuchiStatus) -> MultiResponseStatus {
    match status {
        BuchiStatus::Satisfied => MultiResponseStatus::Satisfied,
        BuchiStatus::Violated => MultiResponseStatus::Violated,
    }
}

fn collapse_fair_buchi_counterexample<S>(
    property: &MultiResponseProperty,
    counterexample: Option<BuchiCounterexample<S, Vec<bool>>>,
) -> Result<Option<MultiResponseCounterexample<S>>, MultiResponseError> {
    match counterexample {
        None => Ok(None),
        Some(BuchiCounterexample::FiniteTerminal {
            missing_acceptance,
            trace,
        }) => Ok(Some(MultiResponseCounterexample::Finite {
            clause: fair_clause_name(property, &missing_acceptance)?,
            trace: collapse_fair_buchi_trace(trace, property.clauses.len())?,
        })),
        Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        }) => Ok(Some(MultiResponseCounterexample::Infinite {
            clause: fair_clause_name(property, &acceptance)?,
            stem: collapse_fair_buchi_trace(stem, property.clauses.len())?,
            cycle: collapse_fair_buchi_trace(cycle, property.clauses.len())?,
        })),
    }
}

fn fair_clause_name(
    property: &MultiResponseProperty,
    acceptance: &str,
) -> Result<String, MultiResponseError> {
    property
        .clauses
        .iter()
        .find(|clause| clause.name == acceptance)
        .map(|clause| clause.name.clone())
        .ok_or(MultiResponseError::FairnessAdapterInvariant)
}

fn collapse_fair_buchi_trace<S>(
    trace: Vec<TraceStep<BuchiProductState<S, Vec<bool>>>>,
    clause_count: usize,
) -> Result<Vec<TraceStep<MultiObligationState<S>>>, MultiResponseError> {
    trace
        .into_iter()
        .map(|step| {
            if step.state.automaton.len() != clause_count {
                return Err(MultiResponseError::FairnessAdapterInvariant);
            }
            Ok(TraceStep {
                action: step.action,
                state: MultiObligationState {
                    state: step.state.state,
                    pending: step.state.automaton,
                },
            })
        })
        .collect()
}

fn analysis_outcome(
    violated: bool,
    model_completion: &BoundedOutcome<()>,
    product_completion: &BoundedOutcome<()>,
) -> AnalysisOutcome<MultiResponseStatus> {
    if violated {
        return AnalysisOutcome::Conclusive(MultiResponseStatus::Violated);
    }
    if let BoundedOutcome::Inconclusive(reason) = model_completion {
        return AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: *reason,
        });
    }
    if let BoundedOutcome::Inconclusive(reason) = product_completion {
        return AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Product,
            reason: *reason,
        });
    }
    AnalysisOutcome::Conclusive(MultiResponseStatus::Satisfied)
}

fn next_pending(property: &MultiResponseProperty, pending: &[bool], action: &str) -> Vec<bool> {
    let mut next_pending = pending.to_vec();
    for (index, clause) in property.clauses.iter().enumerate() {
        if (clause.response)(action) {
            next_pending[index] = false;
        } else if (clause.trigger)(action) {
            next_pending[index] = true;
        }
    }
    next_pending
}

fn find_counterexample<S>(
    product: &ReachableGraph<MultiObligationState<S>>,
    known_terminal: &[bool],
    property: &MultiResponseProperty,
) -> Result<Option<MultiResponseCounterexample<S>>, MultiResponseError>
where
    S: Clone + Eq + Hash,
{
    for (terminal, state) in product.states.iter().enumerate() {
        if !known_terminal[terminal] {
            continue;
        }
        let Some(clause_index) = state.pending.iter().position(|pending| *pending) else {
            continue;
        };
        let trace = shortest_path(product, &product.initial_ids, terminal, None)
            .ok_or(MultiResponseError::MissingFiniteWitness)?;
        return Ok(Some(MultiResponseCounterexample::Finite {
            clause: property.clauses[clause_index].name.clone(),
            trace,
        }));
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

        let residual = induced_graph(product, &included);
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
        let stem = shortest_path(product, &product.initial_ids, product_entry, None)
            .ok_or(MultiResponseError::MissingCycleWitness)?;
        let candidate = InfiniteCandidate {
            clause_index,
            product_entry,
            stem,
            residual,
            component_index,
            component: component.clone(),
        };

        let replace = best
            .as_ref()
            .is_none_or(|current| candidate_key(&candidate) < candidate_key(current));
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

        return Ok(Some(MultiResponseCounterexample::Infinite {
            clause: property.clauses[candidate.clause_index].name.clone(),
            stem: candidate.stem,
            cycle: local.cycle,
        }));
    }

    Ok(None)
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

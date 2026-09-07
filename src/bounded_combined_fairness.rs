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
    check_buchi_with_limits, check_buchi_with_product_limits, AnalysisBuchiResult,
    BoundedBuchiResult, BuchiAutomaton, BuchiError, BuchiProductState, BuchiStatus,
};
use crate::checker::ExplorationLimits;
use crate::combined_fairness::{find_profile_buchi_counterexample, FairnessProfile};
use crate::fair_enablement::{bounded_enablement_graph_for_actions, complete_enablement_graph};
use crate::graph::{
    capture_reachable_graph, capture_reachable_graph_with_limits, GraphCaptureCompletion,
};
use crate::model::TransitionSystem;
use crate::product::{
    build_action_product_from_prefix_with_limits, build_action_product_with_limits,
    BoundedActionProduct,
};
use crate::recurrence::RecurrenceError;
use std::hash::Hash;

/// Verify generalized Buchi acceptance under one combined weak/strong fairness
/// profile while bounding only product construction after complete model
/// capture.
///
/// Complete model capture remains authoritative for action enablement. Missing
/// product-prefix edges therefore cannot make a configured weak action appear
/// disabled or a configured strong action appear absent. A retained finite or
/// mixed-fair recurrent counterexample is conclusive; otherwise an incomplete
/// product remains `INCONCLUSIVE`.
///
/// Empty, weak-only, and strong-only profiles delegate exactly to their sealed
/// historical bounded backends.
pub fn check_buchi_with_fairness_profile_and_product_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    profile: &FairnessProfile,
    limits: ExplorationLimits,
) -> Result<BoundedBuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if profile.is_empty() {
        return check_buchi_with_product_limits(model, automaton, limits);
    }
    if profile.strong().is_empty() {
        return check_buchi_with_weak_fairness_and_product_limits(
            model,
            automaton,
            profile.weak(),
            limits,
        );
    }
    if profile.weak().is_empty() {
        return check_buchi_with_strong_fairness_and_product_limits(
            model,
            automaton,
            profile.strong(),
            limits,
        );
    }

    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let BoundedActionProduct {
        graph: product,
        checked_states,
        explored_transitions,
        max_depth_reached,
        completion,
        known_terminal,
    } = build_action_product_with_limits(
        &captured.graph,
        automaton.initial(),
        |state, action| automaton.advance(state, action),
        |state, automaton| BuchiProductState { state, automaton },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let enablement = complete_enablement_graph(&captured.graph, &product, |state| &state.state)
        .ok_or(BuchiError::MissingWitness)?;
    let counterexample = find_profile_buchi_counterexample(
        &enablement,
        &product,
        &known_terminal,
        automaton,
        profile,
    )?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    } else {
        match completion {
            BoundedOutcome::Conclusive(()) => BoundedOutcome::Conclusive(BuchiStatus::Satisfied),
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedBuchiResult {
        automaton: automaton.name().to_owned(),
        outcome,
        finite_policy: automaton.finite_policy(),
        acceptance_sets: automaton.acceptance_sets().len(),
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        checked_product_states: checked_states,
        explored_product_transitions: explored_transitions,
        retained_product_transitions,
        max_product_depth_reached: max_depth_reached,
        counterexample,
    })
}

/// Verify generalized Buchi acceptance under one combined weak/strong fairness
/// profile with independent deterministic model-capture and product budgets.
///
/// If bounded model capture has not evaluated a retained state's complete
/// successor vector, every configured weak or strong action is conservatively
/// represented as possibly enabled at that state. Missing prefix edges can
/// therefore never become false disabled/not-enabled evidence. A conclusive
/// retained violation still needs real product edges satisfying the complete
/// mixed-fair witness contract.
///
/// When no violation is already justified, model-stage incompleteness takes
/// precedence over product-stage incompleteness. Empty, weak-only, and
/// strong-only profiles delegate exactly to their sealed staged backends.
pub fn check_buchi_with_fairness_profile_and_limits<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    profile: &FairnessProfile,
    limits: AnalysisLimits,
) -> Result<AnalysisBuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if profile.is_empty() {
        return check_buchi_with_limits(model, automaton, limits);
    }
    if profile.strong().is_empty() {
        return check_buchi_with_weak_fairness_and_limits(model, automaton, profile.weak(), limits);
    }
    if profile.weak().is_empty() {
        return check_buchi_with_strong_fairness_and_limits(
            model,
            automaton,
            profile.strong(),
            limits,
        );
    }

    let captured =
        capture_reachable_graph_with_limits(model, limits.model).map_err(RecurrenceError::from)?;
    let retained_model_transitions = captured.graph.outgoing.iter().map(Vec::len).sum();
    let model_completion = match captured.completion {
        GraphCaptureCompletion::Complete => BoundedOutcome::Conclusive(()),
        GraphCaptureCompletion::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
    };
    let BoundedActionProduct {
        graph: product,
        checked_states: checked_product_states,
        explored_transitions: explored_product_transitions,
        max_depth_reached: max_product_depth_reached,
        completion: product_completion,
        known_terminal,
    } = build_action_product_from_prefix_with_limits(
        &captured.graph,
        &captured.known_terminal,
        automaton.initial(),
        |state, action| automaton.advance(state, action),
        |state, automaton| BuchiProductState { state, automaton },
        limits.product,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let fair_actions = profile_actions(profile);
    let enablement = bounded_enablement_graph_for_actions(
        &captured.graph,
        &captured.complete_enabled_actions,
        &product,
        &fair_actions,
        |state| &state.state,
    )
    .ok_or(BuchiError::MissingWitness)?;
    let counterexample = find_profile_buchi_counterexample(
        &enablement,
        &product,
        &known_terminal,
        automaton,
        profile,
    )?;
    let outcome = staged_profile_outcome(
        counterexample.is_some(),
        &model_completion,
        &product_completion,
    );

    Ok(AnalysisBuchiResult {
        automaton: automaton.name().to_owned(),
        outcome,
        finite_policy: automaton.finite_policy(),
        acceptance_sets: automaton.acceptance_sets().len(),
        model_completion,
        product_completion,
        model_states: captured.discovered_states,
        checked_model_states: captured.checked_states,
        explored_model_transitions: captured.explored_transitions,
        retained_model_transitions,
        max_model_depth_reached: captured.max_depth_reached,
        product_states: product.states.len(),
        checked_product_states,
        explored_product_transitions,
        retained_product_transitions,
        max_product_depth_reached,
        counterexample,
    })
}

fn profile_actions(profile: &FairnessProfile) -> Vec<String> {
    profile
        .weak_actions()
        .iter()
        .chain(profile.strong_actions())
        .cloned()
        .collect()
}

fn staged_profile_outcome(
    violated: bool,
    model_completion: &BoundedOutcome<()>,
    product_completion: &BoundedOutcome<()>,
) -> AnalysisOutcome<BuchiStatus> {
    if violated {
        return AnalysisOutcome::Conclusive(BuchiStatus::Violated);
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
    AnalysisOutcome::Conclusive(BuchiStatus::Satisfied)
}

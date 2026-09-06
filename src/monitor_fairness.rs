use crate::bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::fair_enablement::{bounded_enablement_graph, complete_enablement_graph};
use crate::fairness::{weakly_fair_cycle, WeakFairness};
use crate::graph::{
    capture_reachable_graph, capture_reachable_graph_with_limits, induced_graph, shortest_path,
    GraphCaptureCompletion, ReachableGraph,
};
use crate::model::TransitionSystem;
use crate::monitor::{
    check_monitor, check_monitor_with_limits, check_monitor_with_product_limits,
    AnalysisMonitorResult, BoundedMonitorResult, FiniteMonitor, MonitorCounterexample, MonitorError,
    MonitorProductState, MonitorResult, MonitorStatus,
};
use crate::product::{
    build_action_product, build_action_product_from_prefix_with_limits,
    build_action_product_with_limits, BoundedActionProduct,
};
use crate::recurrence::{component_is_cyclic, strongly_connected_components, RecurrenceError};
use std::hash::Hash;

/// Verify a finite monitor while quantifying infinite progress obligations only
/// over executions admitted by explicit exact-action weak fairness.
///
/// Fairness never changes immediate rejecting-state semantics or finite
/// progress-terminal semantics. It filters only recurrent progress cycles. The
/// empty fairness set delegates exactly to the historical monitor engine.
pub fn check_monitor_with_weak_fairness<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    fairness: &WeakFairness,
) -> Result<MonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_monitor(model, monitor);
    }

    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let product = build_action_product(
        &captured.graph,
        monitor.initial(),
        |state, action| monitor.advance(state, action),
        |state, monitor| MonitorProductState { state, monitor },
    );
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let known_terminal = product
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let enablement = complete_enablement_graph(&captured.graph, &product, |state| &state.state)
        .ok_or(MonitorError::MissingWitness)?;
    let counterexample =
        find_fair_counterexample(&enablement, &product, &known_terminal, monitor, fairness)?;

    Ok(MonitorResult {
        monitor: monitor.name().to_owned(),
        status: if counterexample.is_some() {
            MonitorStatus::Violated
        } else {
            MonitorStatus::Satisfied
        },
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample,
    })
}

/// Product-bounded weak-fair monitor verification after complete model capture.
///
/// A retained rejecting state, true active terminal, or weakly-fair active
/// cycle is conclusive even if later product work is cut off. If no such
/// witness is justified, an incomplete product remains `INCONCLUSIVE`.
pub fn check_monitor_with_weak_fairness_and_product_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    fairness: &WeakFairness,
    limits: ExplorationLimits,
) -> Result<BoundedMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_monitor_with_product_limits(model, monitor, limits);
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
        monitor.initial(),
        |state, action| monitor.advance(state, action),
        |state, monitor| MonitorProductState { state, monitor },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let enablement = complete_enablement_graph(&captured.graph, &product, |state| &state.state)
        .ok_or(MonitorError::MissingWitness)?;
    let counterexample =
        find_fair_counterexample(&enablement, &product, &known_terminal, monitor, fairness)?;
    let outcome = if counterexample.is_some() {
        BoundedOutcome::Conclusive(MonitorStatus::Violated)
    } else {
        match completion {
            BoundedOutcome::Conclusive(()) => BoundedOutcome::Conclusive(MonitorStatus::Satisfied),
            BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(reason),
        }
    };

    Ok(BoundedMonitorResult {
        monitor: monitor.name().to_owned(),
        outcome,
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

/// Staged model/product-bounded weak-fair monitor verification.
///
/// Bounded model capture supplies exact enablement only for states whose full
/// successor vector was evaluated. Unknown fair-action enablement is treated
/// conservatively as enabled, so a cutoff can never fabricate a disabled state
/// and thereby certify an unfair recurrent cycle as fair.
pub fn check_monitor_with_weak_fairness_and_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    fairness: &WeakFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
{
    if fairness.is_empty() {
        return check_monitor_with_limits(model, monitor, limits);
    }

    let captured =
        capture_reachable_graph_with_limits(model, limits.model).map_err(RecurrenceError::from)?;
    let model_retained_transitions = captured.graph.outgoing.iter().map(Vec::len).sum();
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
        monitor.initial(),
        |state, action| monitor.advance(state, action),
        |state, monitor| MonitorProductState { state, monitor },
        limits.product,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let enablement = bounded_enablement_graph(
        &captured.graph,
        &captured.complete_enabled_actions,
        &product,
        fairness,
        |state| &state.state,
    )
    .ok_or(MonitorError::MissingWitness)?;
    let counterexample =
        find_fair_counterexample(&enablement, &product, &known_terminal, monitor, fairness)?;
    let outcome = staged_outcome(
        counterexample.is_some(),
        &model_completion,
        &product_completion,
    );

    Ok(AnalysisMonitorResult {
        monitor: monitor.name().to_owned(),
        outcome,
        model_completion,
        product_completion,
        model_states: captured.discovered_states,
        checked_model_states: captured.checked_states,
        explored_model_transitions: captured.explored_transitions,
        retained_model_transitions: model_retained_transitions,
        max_model_depth_reached: captured.max_depth_reached,
        product_states: product.states.len(),
        checked_product_states,
        explored_product_transitions,
        retained_product_transitions,
        max_product_depth_reached,
        counterexample,
    })
}

fn staged_outcome(
    violated: bool,
    model_completion: &BoundedOutcome<()>,
    product_completion: &BoundedOutcome<()>,
) -> AnalysisOutcome<MonitorStatus> {
    if violated {
        return AnalysisOutcome::Conclusive(MonitorStatus::Violated);
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
    AnalysisOutcome::Conclusive(MonitorStatus::Satisfied)
}

fn find_fair_counterexample<S, M>(
    enablement: &ReachableGraph<MonitorProductState<S, M>>,
    product: &ReachableGraph<MonitorProductState<S, M>>,
    known_terminal: &[bool],
    monitor: &FiniteMonitor<M>,
    fairness: &WeakFairness,
) -> Result<Option<MonitorCounterexample<S, M>>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash,
{
    // Rejecting states are safety violations. Fairness constrains only infinite
    // scheduling and must never erase or delay this precedence tier.
    for (product_id, state) in product.states.iter().enumerate() {
        for condition in monitor.rejecting() {
            if condition.matches(&state.monitor) {
                let trace = shortest_path(product, &product.initial_ids, product_id, None)
                    .ok_or(MonitorError::MissingWitness)?;
                return Ok(Some(MonitorCounterexample::Rejecting {
                    condition: condition.name().to_owned(),
                    trace,
                }));
            }
        }
    }

    // Weak fairness says nothing about finite executions. A justified terminal
    // inside an active progress region therefore remains a violation.
    for (product_id, state) in product.states.iter().enumerate() {
        if !known_terminal[product_id] {
            continue;
        }
        for condition in monitor.progress() {
            if condition.is_active(&state.monitor) {
                let trace = shortest_path(product, &product.initial_ids, product_id, None)
                    .ok_or(MonitorError::MissingWitness)?;
                return Ok(Some(MonitorCounterexample::ProgressTerminal {
                    condition: condition.name().to_owned(),
                    trace,
                }));
            }
        }
    }

    let mut best: Option<FairProgressCandidate<S, M>> = None;
    for (condition_index, condition) in monitor.progress().iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| condition.is_active(&state.monitor))
            .collect::<Vec<_>>();
        let old_ids = included
            .iter()
            .enumerate()
            .filter_map(|(id, included)| included.then_some(id))
            .collect::<Vec<_>>();
        if old_ids.is_empty() {
            continue;
        }

        let residual = induced_graph(product, &included);
        for component in strongly_connected_components(&residual) {
            if !component_is_cyclic(&residual, &component) {
                continue;
            }
            let Some(cycle) =
                weakly_fair_cycle(enablement, &residual, &old_ids, &component, fairness)?
            else {
                continue;
            };
            let entry = *component.first().ok_or(MonitorError::MissingWitness)?;
            let product_entry = old_ids[entry];
            let stem = shortest_path(product, &product.initial_ids, product_entry, None)
                .ok_or(MonitorError::MissingWitness)?;
            let candidate = FairProgressCandidate {
                condition_index,
                product_entry,
                stem,
                cycle,
            };
            if best.as_ref().is_none_or(|current| {
                candidate_key(&candidate) < candidate_key(current)
            }) {
                best = Some(candidate);
            }
        }
    }

    Ok(best.map(|candidate| MonitorCounterexample::ProgressCycle {
        condition: monitor.progress()[candidate.condition_index]
            .name()
            .to_owned(),
        stem: candidate.stem,
        cycle: candidate.cycle,
    }))
}

struct FairProgressCandidate<S, M> {
    condition_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<MonitorProductState<S, M>>>,
    cycle: Vec<TraceStep<MonitorProductState<S, M>>>,
}

fn candidate_key<S, M>(candidate: &FairProgressCandidate<S, M>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.condition_index,
        candidate.product_entry,
    )
}

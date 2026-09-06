use crate::bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
use crate::buchi::{
    AcceptanceSet, BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState,
    FiniteRunPolicy,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::fair_enablement::{bounded_enablement_graph_for_actions, complete_enablement_graph};
use crate::graph::{
    capture_reachable_graph, capture_reachable_graph_with_limits, shortest_path,
    GraphCaptureCompletion, ReachableGraph,
};
use crate::model::TransitionSystem;
use crate::monitor::{
    check_monitor, check_monitor_with_limits, check_monitor_with_product_limits,
    AnalysisMonitorResult, BoundedMonitorResult, FiniteMonitor, MonitorCounterexample,
    MonitorError, MonitorProductState, MonitorResult, MonitorStatus,
};
use crate::product::{
    build_action_product, build_action_product_from_prefix_with_limits,
    build_action_product_with_limits, BoundedActionProduct,
};
use crate::recurrence::RecurrenceError;
use crate::strong_fairness::{find_strong_fair_buchi_counterexample, StrongFairness};
use std::hash::Hash;

/// Verify a finite monitor while quantifying infinite progress obligations only
/// over executions admitted by explicit exact-action strong fairness.
///
/// Rejecting monitor states keep global precedence over every progress failure.
/// Strong fairness constrains only infinite executions, so a real finite
/// terminal inside an active progress region remains a violation. Only active
/// recurrent progress cycles are filtered through strong fairness.
pub fn check_monitor_with_strong_fairness<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    fairness: &StrongFairness,
) -> Result<MonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
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
    let counterexample = find_strong_fair_monitor_counterexample(
        &enablement,
        &product,
        &known_terminal,
        monitor,
        fairness,
    )?;

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

/// Product-bounded strong-fair monitor verification after complete model
/// capture.
///
/// A retained rejecting state, true active terminal, or justified strong-fair
/// active recurrent cycle is conclusive even if later product work is cut off.
/// Without such evidence, an incomplete product remains `INCONCLUSIVE`.
pub fn check_monitor_with_strong_fairness_and_product_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    fairness: &StrongFairness,
    limits: ExplorationLimits,
) -> Result<BoundedMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
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
    let counterexample = find_strong_fair_monitor_counterexample(
        &enablement,
        &product,
        &known_terminal,
        monitor,
        fairness,
    )?;
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

/// Staged model/product-bounded strong-fair monitor verification.
///
/// Bounded model capture supplies exact enablement only for states whose full
/// successor vector was evaluated. Unknown enablement is conservatively treated
/// as possible for every configured strong-fair action, so missing prefix edges
/// can never certify a strong-fair recurrent witness by pretending an action is
/// disabled.
pub fn check_monitor_with_strong_fairness_and_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    fairness: &StrongFairness,
    limits: AnalysisLimits,
) -> Result<AnalysisMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
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
    let enablement = bounded_enablement_graph_for_actions(
        &captured.graph,
        &captured.complete_enabled_actions,
        &product,
        fairness.actions(),
        |state| &state.state,
    )
    .ok_or(MonitorError::MissingWitness)?;
    let counterexample = find_strong_fair_monitor_counterexample(
        &enablement,
        &product,
        &known_terminal,
        monitor,
        fairness,
    )?;
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

fn find_strong_fair_monitor_counterexample<S, M>(
    enablement: &ReachableGraph<MonitorProductState<S, M>>,
    product: &ReachableGraph<MonitorProductState<S, M>>,
    known_terminal: &[bool],
    monitor: &FiniteMonitor<M>,
    fairness: &StrongFairness,
) -> Result<Option<MonitorCounterexample<S, M>>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
{
    if enablement.states.len() != product.states.len()
        || known_terminal.len() != product.states.len()
    {
        return Err(MonitorError::MissingWitness);
    }

    // Rejecting monitor states are safety-style failures. They globally precede
    // progress-terminal and progress-cycle failures, and fairness must never
    // erase or delay them.
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

    if monitor.progress().is_empty() {
        return Ok(None);
    }

    let automaton = progress_automaton(monitor)?;
    let buchi_product = as_buchi_graph(product);
    let buchi_enablement = as_buchi_graph(enablement);
    let counterexample = find_strong_fair_buchi_counterexample(
        &buchi_enablement,
        &buchi_product,
        known_terminal,
        &automaton,
        fairness,
    )
    .map_err(map_buchi_error)?;

    Ok(counterexample.map(map_buchi_counterexample))
}

fn progress_automaton<M>(monitor: &FiniteMonitor<M>) -> Result<BuchiAutomaton<M>, MonitorError>
where
    M: Clone + 'static,
{
    let acceptance = monitor
        .progress()
        .iter()
        .cloned()
        .map(|condition| {
            let name = condition.name().to_owned();
            AcceptanceSet::new(name, move |state| !condition.is_active(state))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_buchi_error)?;

    BuchiAutomaton::new(
        format!("{}-strong-fair-progress", monitor.name()),
        monitor.initial().clone(),
        |state: &M, _action| state.clone(),
        acceptance,
        FiniteRunPolicy::RequireAcceptingTerminal,
    )
    .map_err(map_buchi_error)
}

fn map_buchi_error(error: BuchiError) -> MonitorError {
    match error {
        BuchiError::Graph(error) => MonitorError::Graph(error),
        BuchiError::EmptyAutomatonName
        | BuchiError::NoAcceptanceSets
        | BuchiError::EmptyAcceptanceName
        | BuchiError::DuplicateAcceptanceName { .. }
        | BuchiError::MissingWitness => MonitorError::MissingWitness,
    }
}

fn as_buchi_graph<S, M>(
    graph: &ReachableGraph<MonitorProductState<S, M>>,
) -> ReachableGraph<BuchiProductState<S, M>>
where
    S: Clone,
    M: Clone,
{
    ReachableGraph {
        states: graph
            .states
            .iter()
            .map(|state| BuchiProductState {
                state: state.state.clone(),
                automaton: state.monitor.clone(),
            })
            .collect(),
        outgoing: graph.outgoing.clone(),
        initial_ids: graph.initial_ids.clone(),
    }
}

fn map_buchi_counterexample<S, M>(
    counterexample: BuchiCounterexample<S, M>,
) -> MonitorCounterexample<S, M> {
    match counterexample {
        BuchiCounterexample::FiniteTerminal {
            missing_acceptance,
            trace,
        } => MonitorCounterexample::ProgressTerminal {
            condition: missing_acceptance,
            trace: map_trace(trace),
        },
        BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        } => MonitorCounterexample::ProgressCycle {
            condition: acceptance,
            stem: map_trace(stem),
            cycle: map_trace(cycle),
        },
    }
}

fn map_trace<S, M>(
    trace: Vec<TraceStep<BuchiProductState<S, M>>>,
) -> Vec<TraceStep<MonitorProductState<S, M>>> {
    trace
        .into_iter()
        .map(|step| TraceStep {
            action: step.action,
            state: MonitorProductState {
                state: step.state.state,
                monitor: step.state.automaton,
            },
        })
        .collect()
}

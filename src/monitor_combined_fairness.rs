use crate::bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
use crate::buchi::{
    AcceptanceSet, BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState,
    FiniteRunPolicy,
};
use crate::checker::{ExplorationLimits, TraceStep};
use crate::combined_fairness::{find_profile_buchi_counterexample, FairnessProfile};
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
use crate::monitor_fairness::{
    check_monitor_with_weak_fairness, check_monitor_with_weak_fairness_and_limits,
    check_monitor_with_weak_fairness_and_product_limits,
};
use crate::monitor_strong_fairness::{
    check_monitor_with_strong_fairness, check_monitor_with_strong_fairness_and_limits,
    check_monitor_with_strong_fairness_and_product_limits,
};
use crate::product::{
    build_action_product, build_action_product_from_prefix_with_limits,
    build_action_product_with_limits, BoundedActionProduct,
};
use crate::recurrence::RecurrenceError;
use std::hash::Hash;

/// Verify finite-monitor rejection/progress semantics under one canonical
/// combined weak/strong fairness profile.
///
/// Fairness filters only infinite progress executions. Reachable rejecting
/// states retain global precedence, and finite active terminals remain real
/// violations. Empty, weak-only, and strong-only profiles delegate exactly to
/// their already sealed monitor backends; only genuinely mixed profiles use the
/// combined-fair Büchi recurrent engine.
pub fn check_monitor_with_fairness_profile<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    profile: &FairnessProfile,
) -> Result<MonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
{
    if profile.is_empty() {
        return check_monitor(model, monitor);
    }
    if profile.strong().is_empty() {
        return check_monitor_with_weak_fairness(model, monitor, profile.weak());
    }
    if profile.weak().is_empty() {
        return check_monitor_with_strong_fairness(model, monitor, profile.strong());
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
    let counterexample = find_profile_monitor_counterexample(
        &enablement,
        &product,
        &known_terminal,
        monitor,
        profile,
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

/// Product-bounded combined-fair monitor verification after complete model
/// capture. A retained rejecting state, true active terminal, or justified
/// mixed-fair active recurrent cycle is conclusive; otherwise an incomplete
/// product remains `INCONCLUSIVE`.
pub fn check_monitor_with_fairness_profile_and_product_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    profile: &FairnessProfile,
    limits: ExplorationLimits,
) -> Result<BoundedMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
{
    if profile.is_empty() {
        return check_monitor_with_product_limits(model, monitor, limits);
    }
    if profile.strong().is_empty() {
        return check_monitor_with_weak_fairness_and_product_limits(
            model,
            monitor,
            profile.weak(),
            limits,
        );
    }
    if profile.weak().is_empty() {
        return check_monitor_with_strong_fairness_and_product_limits(
            model,
            monitor,
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
        monitor.initial(),
        |state, action| monitor.advance(state, action),
        |state, monitor| MonitorProductState { state, monitor },
        limits,
    );
    let retained_product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let enablement = complete_enablement_graph(&captured.graph, &product, |state| &state.state)
        .ok_or(MonitorError::MissingWitness)?;
    let counterexample = find_profile_monitor_counterexample(
        &enablement,
        &product,
        &known_terminal,
        monitor,
        profile,
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

/// Staged combined-fair monitor verification with independent model and product
/// budgets. Unknown model-side enablement stays conservative for every weak or
/// strong fair action. When no retained violation is already conclusive,
/// model-stage incompleteness precedes product-stage incompleteness.
pub fn check_monitor_with_fairness_profile_and_limits<S, M>(
    model: &TransitionSystem<S>,
    monitor: &FiniteMonitor<M>,
    profile: &FairnessProfile,
    limits: AnalysisLimits,
) -> Result<AnalysisMonitorResult<S, M>, MonitorError>
where
    S: Clone + Eq + Hash,
    M: Clone + Eq + Hash + 'static,
{
    if profile.is_empty() {
        return check_monitor_with_limits(model, monitor, limits);
    }
    if profile.strong().is_empty() {
        return check_monitor_with_weak_fairness_and_limits(model, monitor, profile.weak(), limits);
    }
    if profile.weak().is_empty() {
        return check_monitor_with_strong_fairness_and_limits(
            model,
            monitor,
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
        monitor.initial(),
        |state, action| monitor.advance(state, action),
        |state, monitor| MonitorProductState { state, monitor },
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
    .ok_or(MonitorError::MissingWitness)?;
    let counterexample = find_profile_monitor_counterexample(
        &enablement,
        &product,
        &known_terminal,
        monitor,
        profile,
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

fn find_profile_monitor_counterexample<S, M>(
    enablement: &ReachableGraph<MonitorProductState<S, M>>,
    product: &ReachableGraph<MonitorProductState<S, M>>,
    known_terminal: &[bool],
    monitor: &FiniteMonitor<M>,
    profile: &FairnessProfile,
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
    let counterexample = find_profile_buchi_counterexample(
        &buchi_enablement,
        &buchi_product,
        known_terminal,
        &automaton,
        profile,
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
        format!("{}-combined-fair-progress", monitor.name()),
        monitor.initial().clone(),
        |state: &M, _action| state.clone(),
        acceptance,
        FiniteRunPolicy::RequireAcceptingTerminal,
    )
    .map_err(map_buchi_error)
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

fn profile_actions(profile: &FairnessProfile) -> Vec<String> {
    profile
        .weak_actions()
        .iter()
        .chain(profile.strong_actions())
        .cloned()
        .collect()
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

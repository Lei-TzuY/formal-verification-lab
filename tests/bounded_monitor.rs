use formal_verification_lab::monitor::{
    check_monitor, check_monitor_with_product_limits, FiniteMonitor, MonitorCounterexample,
    MonitorStatus, ProgressCondition, RejectCondition,
};
use formal_verification_lab::monitor_examples::{
    session_monitor, session_protocol, stuck_committed_protocol,
};
use formal_verification_lab::{
    BoundedOutcome, ExplorationLimits, InconclusiveReason, Invariant, StateVariable, Transition,
    TransitionSystem,
};

fn model_with_edges(
    name: &'static str,
    successors: impl Fn(&u8) -> Vec<Transition<u8>> + Send + Sync + 'static,
) -> TransitionSystem<u8> {
    TransitionSystem::new(
        name,
        vec![StateVariable::new("node", "bounded monitor test node")],
        vec![0_u8],
        move |state| Ok(successors(state)),
        vec![Invariant::new("well-formed", |state: &u8| *state < 4)],
    )
    .unwrap()
}

fn limits(
    max_states: Option<usize>,
    max_transitions: Option<usize>,
    max_depth: Option<usize>,
) -> ExplorationLimits {
    ExplorationLimits {
        max_states,
        max_transitions,
        max_depth,
    }
}

fn always_active_monitor() -> FiniteMonitor<bool> {
    FiniteMonitor::new(
        "always-active",
        true,
        |state, _action| *state,
        Vec::new(),
        vec![ProgressCondition::new("must-not-stay-active", |state| *state).unwrap()],
    )
    .unwrap()
}

#[test]
fn partial_nonterminal_is_not_fabricated_as_progress_terminal() {
    let model = model_with_edges("nonterminal-prefix", |state| match state {
        0 => vec![Transition::new("advance", 1)],
        1 => vec![Transition::new("finish", 2)],
        _ => Vec::new(),
    });

    let result = check_monitor_with_product_limits(
        &model,
        &always_active_monitor(),
        limits(None, Some(0), None),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
    );
    assert_eq!(result.product_states, 1);
    assert_eq!(result.checked_product_states, 1);
    assert_eq!(result.explored_product_transitions, 0);
    assert_eq!(result.retained_product_transitions, 0);
    assert!(result.counterexample.is_none());
}

#[test]
fn rejecting_state_found_before_later_cutoff_is_conclusive() {
    let model = model_with_edges("reject-before-cutoff", |state| match state {
        0 => vec![Transition::new("bad", 1), Transition::new("other", 2)],
        _ => Vec::new(),
    });
    let monitor = FiniteMonitor::new(
        "reject-bad",
        false,
        |state, action| *state || action == "bad",
        vec![RejectCondition::new("bad-observed", |state| *state).unwrap()],
        Vec::new(),
    )
    .unwrap();

    let result =
        check_monitor_with_product_limits(&model, &monitor, limits(None, Some(1), None)).unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(MonitorStatus::Violated)
    );
    assert_eq!(result.explored_product_transitions, 1);
    assert_eq!(result.retained_product_transitions, 1);
    let MonitorCounterexample::Rejecting { condition, trace } = result.counterexample.unwrap()
    else {
        panic!("expected rejecting-state witness");
    };
    assert_eq!(condition, "bad-observed");
    assert_eq!(trace.len(), 2);
    assert_eq!(trace.last().unwrap().action.as_deref(), Some("bad"));
}

#[test]
fn true_active_terminal_found_before_cutoff_is_conclusive() {
    let model = model_with_edges("terminal-before-cutoff", |state| match state {
        0 => vec![
            Transition::new("go-terminal", 1),
            Transition::new("go-other", 2),
        ],
        2 => vec![Transition::new("tick", 2)],
        _ => Vec::new(),
    });
    let monitor = FiniteMonitor::new(
        "terminal-progress",
        false,
        |state, action| *state || action == "go-terminal",
        Vec::new(),
        vec![ProgressCondition::new("eventually-inactive", |state| *state).unwrap()],
    )
    .unwrap();

    let result =
        check_monitor_with_product_limits(&model, &monitor, limits(None, Some(1), None)).unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(MonitorStatus::Violated)
    );
    let MonitorCounterexample::ProgressTerminal { condition, trace } =
        result.counterexample.unwrap()
    else {
        panic!("expected true active terminal witness");
    };
    assert_eq!(condition, "eventually-inactive");
    assert_eq!(trace.len(), 2);
    assert_eq!(trace.last().unwrap().state.state, 1);
}

#[test]
fn active_cycle_found_before_cutoff_is_conclusive() {
    let model = model_with_edges("cycle-before-cutoff", |state| match state {
        0 => vec![Transition::new("tick", 0), Transition::new("escape", 1)],
        _ => Vec::new(),
    });

    let result = check_monitor_with_product_limits(
        &model,
        &always_active_monitor(),
        limits(None, Some(1), None),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(MonitorStatus::Violated)
    );
    let MonitorCounterexample::ProgressCycle {
        condition,
        stem,
        cycle,
    } = result.counterexample.unwrap()
    else {
        panic!("expected active-cycle witness");
    };
    assert_eq!(condition, "must-not-stay-active");
    assert_eq!(stem.len(), 1);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("tick")));
}

#[test]
fn zero_product_state_budget_is_inconclusive() {
    let result = check_monitor_with_product_limits(
        &session_protocol().unwrap(),
        &session_monitor().unwrap(),
        limits(Some(0), None, None),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 0 })
    );
    assert_eq!(result.product_states, 0);
    assert_eq!(result.checked_product_states, 0);
    assert_eq!(result.max_product_depth_reached, None);
    assert!(result.counterexample.is_none());
}

#[test]
fn generous_product_limits_match_unbounded_monitor_result() {
    let model = stuck_committed_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let unbounded = check_monitor(&model, &monitor).unwrap();
    let bounded =
        check_monitor_with_product_limits(&model, &monitor, limits(Some(32), Some(64), Some(32)))
            .unwrap();

    assert_eq!(
        bounded.outcome,
        BoundedOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(bounded.model_states, unbounded.model_states);
    assert_eq!(bounded.model_transitions, unbounded.model_transitions);
    assert_eq!(bounded.product_states, unbounded.product_states);
    assert_eq!(
        bounded.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(bounded.counterexample, unbounded.counterexample);
}

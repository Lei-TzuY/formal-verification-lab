use formal_verification_lab::monitor::{
    check_monitor, check_monitor_with_limits, FiniteMonitor, MonitorCounterexample, MonitorStatus,
    ProgressCondition, RejectCondition,
};
use formal_verification_lab::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
    ExplorationLimits, InconclusiveReason, Invariant, StateVariable, Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Control {
    Idle,
    Active,
    Rejected,
}

fn monitor() -> FiniteMonitor<Control> {
    FiniteMonitor::new(
        "staged-monitor",
        Control::Idle,
        |state, action| match (*state, action) {
            (Control::Idle, "start") => Control::Active,
            (Control::Active, "finish") => Control::Idle,
            (Control::Active, "bad") => Control::Rejected,
            (Control::Active, "wait") => Control::Active,
            (Control::Rejected, _) => Control::Rejected,
            (state, _) => state,
        },
        vec![RejectCondition::new("not-rejected", |state| {
            *state == Control::Rejected
        })
        .unwrap()],
        vec![ProgressCondition::new("active-eventually-clears", |state| {
            *state == Control::Active
        })
        .unwrap()],
    )
    .unwrap()
}

fn model(
    name: &'static str,
    transitions: impl Fn(&usize) -> Vec<Transition<usize>> + Send + Sync + 'static,
) -> TransitionSystem<usize> {
    TransitionSystem::new(
        name,
        vec![StateVariable::new("node", "protocol state")],
        vec![0usize],
        move |state| Ok(transitions(state)),
        vec![Invariant::new("known-node", |state: &usize| *state < 5)],
    )
    .unwrap()
}

fn limits(
    states: Option<usize>,
    transitions: Option<usize>,
    depth: Option<usize>,
) -> ExplorationLimits {
    ExplorationLimits {
        max_states: states,
        max_transitions: transitions,
        max_depth: depth,
    }
}

#[test]
fn model_cutoff_does_not_fabricate_progress_terminal() {
    let model = model("prefix-nonterminal", |state| match *state {
        0 => vec![Transition::new("start", 1)],
        1 => vec![Transition::new("finish", 2)],
        _ => Vec::new(),
    });

    let result = check_monitor_with_limits(
        &model,
        &monitor(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::DepthLimitReached { limit: 1 },
        })
    );
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    assert_eq!(result.product_completion, BoundedOutcome::Conclusive(()));
    assert!(result.counterexample.is_none());
}

#[test]
fn rejecting_state_is_conclusive_before_later_model_cutoff() {
    let model = model("reject-before-cutoff", |state| match *state {
        0 => vec![Transition::new("start", 1), Transition::new("branch", 2)],
        1 => vec![Transition::new("bad", 3)],
        2 => vec![Transition::new("later", 4)],
        _ => Vec::new(),
    });

    let result = check_monitor_with_limits(
        &model,
        &monitor(),
        AnalysisLimits::new(limits(None, None, Some(2)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(MonitorStatus::Violated)
    );
    let Some(MonitorCounterexample::Rejecting { trace, .. }) = result.counterexample else {
        panic!("expected rejecting witness");
    };
    assert_eq!(trace.last().unwrap().state.state, 3);
    assert_eq!(trace.last().unwrap().state.monitor, Control::Rejected);
}

#[test]
fn retained_progress_cycle_is_conclusive_before_later_model_cutoff() {
    let model = model("cycle-before-cutoff", |state| match *state {
        0 => vec![Transition::new("start", 1), Transition::new("branch", 2)],
        1 => vec![Transition::new("wait", 1)],
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });

    let result = check_monitor_with_limits(
        &model,
        &monitor(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(MonitorStatus::Violated)
    );
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    let Some(MonitorCounterexample::ProgressCycle { cycle, .. }) = result.counterexample else {
        panic!("expected progress-cycle witness");
    };
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle.iter().all(|step| step.state.monitor == Control::Active));
}

#[test]
fn model_stage_precedes_product_stage_when_both_are_incomplete() {
    let model = model("both-cutoffs", |state| match *state {
        0 => vec![Transition::new("start", 1)],
        1 => vec![Transition::new("finish", 2)],
        _ => Vec::new(),
    });

    let result = check_monitor_with_limits(
        &model,
        &monitor(),
        AnalysisLimits::new(limits(None, None, Some(1)), limits(Some(1), None, None)),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::DepthLimitReached { limit: 1 },
        })
    );
    assert_eq!(
        result.product_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
}

#[test]
fn fully_unbounded_staged_monitor_matches_legacy_result() {
    let model = model("legacy-equivalence", |state| match *state {
        0 => vec![Transition::new("start", 1)],
        1 => vec![Transition::new("wait", 1), Transition::new("finish", 0)],
        _ => Vec::new(),
    });
    let monitor = monitor();
    let legacy = check_monitor(&model, &monitor).unwrap();
    let staged = check_monitor_with_limits(&model, &monitor, AnalysisLimits::unbounded()).unwrap();

    assert_eq!(staged.outcome, AnalysisOutcome::Conclusive(legacy.status));
    assert_eq!(staged.model_states, legacy.model_states);
    assert_eq!(staged.explored_model_transitions, legacy.model_transitions);
    assert_eq!(staged.product_states, legacy.product_states);
    assert_eq!(
        staged.explored_product_transitions,
        legacy.product_transitions
    );
    assert_eq!(staged.counterexample, legacy.counterexample);
}

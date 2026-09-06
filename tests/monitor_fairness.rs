use formal_verification_lab::monitor_examples::{
    invalid_double_open_protocol, session_monitor, stuck_committed_protocol,
};
use formal_verification_lab::{
    check_monitor, check_monitor_with_limits, check_monitor_with_product_limits,
    check_monitor_with_weak_fairness, check_monitor_with_weak_fairness_and_limits,
    check_monitor_with_weak_fairness_and_product_limits, AnalysisLimits, AnalysisOutcome,
    AnalysisStage, BoundedOutcome, ExplorationLimits, FiniteMonitor, InconclusiveReason, Invariant,
    MonitorCounterexample, MonitorStatus, ProgressCondition, RejectCondition, StateVariable,
    Transition, TransitionSystem, WeakFairness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    Start,
    Active,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Control {
    Idle,
    Active,
}

fn progress_monitor() -> FiniteMonitor<Control> {
    FiniteMonitor::new(
        "fair-progress-monitor",
        Control::Idle,
        |state, action| match (*state, action) {
            (Control::Idle, "open") => Control::Active,
            (Control::Active, "finish") => Control::Idle,
            (Control::Active, "tick" | "fair") => Control::Active,
            _ => *state,
        },
        Vec::<RejectCondition<Control>>::new(),
        vec![ProgressCondition::new("active-eventually-clears", |state| {
            *state == Control::Active
        })
        .unwrap()],
    )
    .unwrap()
}

fn exit_enabled_model() -> TransitionSystem<Node> {
    TransitionSystem::new(
        "fair-exit-enabled",
        vec![StateVariable::new("node", "progress point")],
        vec![Node::Start],
        |state| match state {
            Node::Start => Ok(vec![Transition::new("open", Node::Active)]),
            Node::Active => Ok(vec![
                Transition::new("tick", Node::Active),
                Transition::new("finish", Node::Done),
            ]),
            Node::Done => Ok(Vec::new()),
        },
        vec![Invariant::new("recognized-node", |_state: &Node| true)],
    )
    .unwrap()
}

fn fair_cycle_model() -> TransitionSystem<Node> {
    TransitionSystem::new(
        "fair-cycle-remains-active",
        vec![StateVariable::new("node", "progress point")],
        vec![Node::Start],
        |state| match state {
            Node::Start => Ok(vec![Transition::new("open", Node::Active)]),
            Node::Active => Ok(vec![Transition::new("fair", Node::Active)]),
            Node::Done => Ok(Vec::new()),
        },
        vec![Invariant::new("recognized-node", |_state: &Node| true)],
    )
    .unwrap()
}

fn active_terminal_model() -> TransitionSystem<Node> {
    TransitionSystem::new(
        "active-terminal",
        vec![StateVariable::new("node", "progress point")],
        vec![Node::Start],
        |state| match state {
            Node::Start => Ok(vec![Transition::new("open", Node::Active)]),
            Node::Active | Node::Done => Ok(Vec::new()),
        },
        vec![Invariant::new("recognized-node", |_state: &Node| true)],
    )
    .unwrap()
}

#[test]
fn matching_fair_exit_eliminates_the_unfair_progress_lasso() {
    let model = exit_enabled_model();
    let monitor = progress_monitor();
    let historical = check_monitor(&model, &monitor).unwrap();
    assert!(matches!(
        historical.counterexample,
        Some(MonitorCounterexample::ProgressCycle { .. })
    ));

    let fair =
        check_monitor_with_weak_fairness(&model, &monitor, &WeakFairness::new(["finish"]).unwrap())
            .unwrap();
    assert_eq!(fair.status, MonitorStatus::Satisfied);
    assert!(fair.counterexample.is_none());
}

#[test]
fn unrelated_fairness_does_not_hide_a_progress_cycle() {
    let result = check_monitor_with_weak_fairness(
        &exit_enabled_model(),
        &progress_monitor(),
        &WeakFairness::new(["unrelated"]).unwrap(),
    )
    .unwrap();

    let Some(MonitorCounterexample::ProgressCycle {
        condition, cycle, ..
    }) = result.counterexample
    else {
        panic!("expected progress-cycle violation");
    };
    assert_eq!(condition, "active-eventually-clears");
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn a_cycle_that_executes_the_fair_action_is_still_a_real_violation() {
    let result = check_monitor_with_weak_fairness(
        &fair_cycle_model(),
        &progress_monitor(),
        &WeakFairness::new(["fair"]).unwrap(),
    )
    .unwrap();

    let Some(MonitorCounterexample::ProgressCycle { cycle, .. }) = result.counterexample else {
        panic!("expected fair progress-cycle violation");
    };
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("fair")));
}

#[test]
fn rejecting_state_precedence_is_unchanged_by_fairness() {
    let model = invalid_double_open_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let historical = check_monitor(&model, &monitor).unwrap();
    let fair =
        check_monitor_with_weak_fairness(&model, &monitor, &WeakFairness::new(["close"]).unwrap())
            .unwrap();

    assert_eq!(fair, historical);
    assert!(matches!(
        fair.counterexample,
        Some(MonitorCounterexample::Rejecting { .. })
    ));
}

#[test]
fn finite_active_terminal_is_unchanged_by_fairness() {
    let model = active_terminal_model();
    let monitor = progress_monitor();
    let historical = check_monitor(&model, &monitor).unwrap();
    let fair =
        check_monitor_with_weak_fairness(&model, &monitor, &WeakFairness::new(["finish"]).unwrap())
            .unwrap();

    assert_eq!(fair, historical);
    assert!(matches!(
        fair.counterexample,
        Some(MonitorCounterexample::ProgressTerminal { .. })
    ));
}

#[test]
fn empty_fairness_is_exact_compatibility_for_all_limit_surfaces() {
    let model = stuck_committed_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let fairness = WeakFairness::none();
    let product_limits = ExplorationLimits {
        max_states: Some(8),
        max_transitions: Some(8),
        max_depth: Some(8),
    };
    let analysis_limits = AnalysisLimits::new(product_limits, product_limits);

    assert_eq!(
        check_monitor_with_weak_fairness(&model, &monitor, &fairness).unwrap(),
        check_monitor(&model, &monitor).unwrap()
    );
    assert_eq!(
        check_monitor_with_weak_fairness_and_product_limits(
            &model,
            &monitor,
            &fairness,
            product_limits,
        )
        .unwrap(),
        check_monitor_with_product_limits(&model, &monitor, product_limits).unwrap()
    );
    assert_eq!(
        check_monitor_with_weak_fairness_and_limits(&model, &monitor, &fairness, analysis_limits,)
            .unwrap(),
        check_monitor_with_limits(&model, &monitor, analysis_limits).unwrap()
    );
}

#[test]
fn product_cutoff_stays_inconclusive_when_only_an_unfair_cycle_is_retained() {
    let result = check_monitor_with_weak_fairness_and_product_limits(
        &exit_enabled_model(),
        &progress_monitor(),
        &WeakFairness::new(["finish"]).unwrap(),
        ExplorationLimits {
            max_states: None,
            max_transitions: Some(2),
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn staged_model_cutoff_reports_model_stage_without_fabricating_fair_progress() {
    let result = check_monitor_with_weak_fairness_and_limits(
        &exit_enabled_model(),
        &progress_monitor(),
        &WeakFairness::new(["finish"]).unwrap(),
        AnalysisLimits::new(
            ExplorationLimits {
                max_states: None,
                max_transitions: Some(1),
                max_depth: None,
            },
            ExplorationLimits::unbounded(),
        ),
    )
    .unwrap();

    let AnalysisOutcome::Inconclusive(reason) = result.outcome else {
        panic!("expected staged inconclusive result");
    };
    assert_eq!(reason.stage, AnalysisStage::Model);
    assert_eq!(
        reason.reason,
        InconclusiveReason::TransitionLimitReached { limit: 1 }
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn unbounded_staged_fair_analysis_matches_unbounded_fair_result_and_evidence() {
    let model = fair_cycle_model();
    let monitor = progress_monitor();
    let fairness = WeakFairness::new(["fair"]).unwrap();
    let unbounded = check_monitor_with_weak_fairness(&model, &monitor, &fairness).unwrap();
    let staged = check_monitor_with_weak_fairness_and_limits(
        &model,
        &monitor,
        &fairness,
        AnalysisLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(staged.model_states, unbounded.model_states);
    assert_eq!(
        staged.explored_model_transitions,
        unbounded.model_transitions
    );
    assert_eq!(staged.product_states, unbounded.product_states);
    assert_eq!(
        staged.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(staged.counterexample, unbounded.counterexample);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PairProgress {
    Both,
    OnlyB,
}

fn independent_progress_monitor() -> FiniteMonitor<PairProgress> {
    FiniteMonitor::new(
        "independent-progress",
        PairProgress::Both,
        |state, action| match (*state, action) {
            (PairProgress::Both, "fair-a") => PairProgress::OnlyB,
            _ => *state,
        },
        Vec::<RejectCondition<PairProgress>>::new(),
        vec![
            ProgressCondition::new("a-progress", |state| *state == PairProgress::Both).unwrap(),
            ProgressCondition::new("b-progress", |_state| true).unwrap(),
        ],
    )
    .unwrap()
}

fn independent_progress_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "independent-progress-model",
        vec![StateVariable::new("node", "single recurrent node")],
        vec![0usize],
        |_state| {
            Ok(vec![
                Transition::new("other", 0usize),
                Transition::new("fair-a", 0usize),
            ])
        },
        vec![Invariant::new("single-node", |state: &usize| *state == 0)],
    )
    .unwrap()
}

#[test]
fn fairness_can_discharge_one_progress_region_without_hiding_another() {
    let result = check_monitor_with_weak_fairness(
        &independent_progress_model(),
        &independent_progress_monitor(),
        &WeakFairness::new(["fair-a"]).unwrap(),
    )
    .unwrap();

    let Some(MonitorCounterexample::ProgressCycle {
        condition, cycle, ..
    }) = result.counterexample
    else {
        panic!("expected remaining progress-cycle violation");
    };
    assert_eq!(condition, "b-progress");
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("fair-a")));
}

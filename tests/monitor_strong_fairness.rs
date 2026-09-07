use formal_verification_lab::monitor_examples::{
    invalid_double_open_protocol, session_monitor, stuck_committed_protocol,
};
use formal_verification_lab::{
    check_monitor, check_monitor_with_limits, check_monitor_with_product_limits,
    check_monitor_with_strong_fairness, check_monitor_with_strong_fairness_and_limits,
    check_monitor_with_strong_fairness_and_product_limits, check_monitor_with_weak_fairness,
    AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits,
    FiniteMonitor, InconclusiveReason, Invariant, MonitorCounterexample, MonitorStatus,
    ProgressCondition, RejectCondition, StateVariable, StrongFairness, Transition,
    TransitionSystem, WeakFairness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum IntermittentNode {
    A,
    B,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ProgressState {
    Active,
    Cleared,
}

fn intermittent_monitor() -> FiniteMonitor<ProgressState> {
    FiniteMonitor::new(
        "intermittent-progress",
        ProgressState::Active,
        |state, action| match (*state, action) {
            (ProgressState::Active, "close") => ProgressState::Cleared,
            _ => *state,
        },
        Vec::<RejectCondition<ProgressState>>::new(),
        vec![ProgressCondition::new("active-eventually-clears", |state| {
            *state == ProgressState::Active
        })
        .unwrap()],
    )
    .unwrap()
}

fn intermittent_exit_model() -> TransitionSystem<IntermittentNode> {
    TransitionSystem::new(
        "intermittent-strong-fair-exit",
        vec![StateVariable::new("node", "progress point")],
        vec![IntermittentNode::A],
        |state| match state {
            IntermittentNode::A => Ok(vec![
                Transition::new("to-b", IntermittentNode::B),
                Transition::new("close", IntermittentNode::Done),
            ]),
            IntermittentNode::B => Ok(vec![Transition::new("to-a", IntermittentNode::A)]),
            IntermittentNode::Done => Ok(Vec::new()),
        },
        vec![Invariant::new(
            "recognized-node",
            |_state: &IntermittentNode| true,
        )],
    )
    .unwrap()
}

fn taken_fair_cycle_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "taken-strong-fair-cycle",
        vec![StateVariable::new("node", "single recurrent node")],
        vec![0usize],
        |_state| Ok(vec![Transition::new("fair", 0usize)]),
        vec![Invariant::new("single-node", |state: &usize| *state == 0)],
    )
    .unwrap()
}

fn always_active_monitor() -> FiniteMonitor<bool> {
    FiniteMonitor::new(
        "always-active",
        true,
        |state, _action| *state,
        Vec::<RejectCondition<bool>>::new(),
        vec![ProgressCondition::new("must-not-stay-active", |state| *state).unwrap()],
    )
    .unwrap()
}

fn active_terminal_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "strong-fair-active-terminal",
        vec![StateVariable::new("node", "terminal point")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("advance", 1usize)]),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("bounded-node", |state: &usize| *state <= 1)],
    )
    .unwrap()
}

#[test]
fn intermittent_enablement_distinguishes_strong_from_weak_fairness() {
    let model = intermittent_exit_model();
    let monitor = intermittent_monitor();

    let historical = check_monitor(&model, &monitor).unwrap();
    assert!(matches!(
        historical.counterexample,
        Some(MonitorCounterexample::ProgressCycle { .. })
    ));

    let weak =
        check_monitor_with_weak_fairness(&model, &monitor, &WeakFairness::new(["close"]).unwrap())
            .unwrap();
    assert_eq!(weak.status, MonitorStatus::Violated);
    assert!(matches!(
        weak.counterexample,
        Some(MonitorCounterexample::ProgressCycle { .. })
    ));

    let strong = check_monitor_with_strong_fairness(
        &model,
        &monitor,
        &StrongFairness::new(["close"]).unwrap(),
    )
    .unwrap();
    assert_eq!(strong.status, MonitorStatus::Satisfied);
    assert!(strong.counterexample.is_none());
}

#[test]
fn unrelated_strong_fairness_preserves_the_progress_cycle() {
    let result = check_monitor_with_strong_fairness(
        &intermittent_exit_model(),
        &intermittent_monitor(),
        &StrongFairness::new(["unrelated"]).unwrap(),
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
fn taking_the_strong_fair_action_does_not_hide_a_real_progress_cycle() {
    let result = check_monitor_with_strong_fairness(
        &taken_fair_cycle_model(),
        &always_active_monitor(),
        &StrongFairness::new(["fair"]).unwrap(),
    )
    .unwrap();

    let Some(MonitorCounterexample::ProgressCycle { cycle, .. }) = result.counterexample else {
        panic!("expected strong-fair progress-cycle violation");
    };
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("fair")));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn rejecting_state_precedes_progress_and_is_unchanged_by_strong_fairness() {
    let model = invalid_double_open_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let historical = check_monitor(&model, &monitor).unwrap();
    let strong = check_monitor_with_strong_fairness(
        &model,
        &monitor,
        &StrongFairness::new(["close"]).unwrap(),
    )
    .unwrap();

    assert_eq!(strong, historical);
    assert!(matches!(
        strong.counterexample,
        Some(MonitorCounterexample::Rejecting { .. })
    ));
}

#[test]
fn finite_active_terminal_is_not_excused_by_strong_fairness() {
    let model = active_terminal_model();
    let monitor = always_active_monitor();
    let result = check_monitor_with_strong_fairness(
        &model,
        &monitor,
        &StrongFairness::new(["advance"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, MonitorStatus::Violated);
    assert!(matches!(
        result.counterexample,
        Some(MonitorCounterexample::ProgressTerminal { .. })
    ));
}

#[test]
fn empty_strong_fairness_is_exact_compatibility_for_all_limit_surfaces() {
    let model = stuck_committed_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let fairness = StrongFairness::none();
    let product_limits = ExplorationLimits {
        max_states: Some(8),
        max_transitions: Some(8),
        max_depth: Some(8),
    };
    let analysis_limits = AnalysisLimits::new(product_limits, product_limits);

    assert_eq!(
        check_monitor_with_strong_fairness(&model, &monitor, &fairness).unwrap(),
        check_monitor(&model, &monitor).unwrap()
    );
    assert_eq!(
        check_monitor_with_strong_fairness_and_product_limits(
            &model,
            &monitor,
            &fairness,
            product_limits,
        )
        .unwrap(),
        check_monitor_with_product_limits(&model, &monitor, product_limits).unwrap()
    );
    assert_eq!(
        check_monitor_with_strong_fairness_and_limits(
            &model,
            &monitor,
            &fairness,
            analysis_limits,
        )
        .unwrap(),
        check_monitor_with_limits(&model, &monitor, analysis_limits).unwrap()
    );
}

#[test]
fn product_cutoff_stays_inconclusive_before_a_strong_fair_proof_is_complete() {
    let result = check_monitor_with_strong_fairness_and_product_limits(
        &intermittent_exit_model(),
        &intermittent_monitor(),
        &StrongFairness::new(["close"]).unwrap(),
        ExplorationLimits {
            max_states: None,
            max_transitions: Some(1),
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 1 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn staged_model_cutoff_reports_model_stage_without_fabricating_strong_fairness() {
    let result = check_monitor_with_strong_fairness_and_limits(
        &intermittent_exit_model(),
        &intermittent_monitor(),
        &StrongFairness::new(["close"]).unwrap(),
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
fn generous_staged_limits_preserve_unbounded_strong_fair_result_and_evidence() {
    let model = taken_fair_cycle_model();
    let monitor = always_active_monitor();
    let fairness = StrongFairness::new(["fair"]).unwrap();
    let unbounded = check_monitor_with_strong_fairness(&model, &monitor, &fairness).unwrap();
    let staged = check_monitor_with_strong_fairness_and_limits(
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
        "strong-independent-progress",
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
        "strong-independent-progress-model",
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
fn strong_fairness_can_discharge_one_progress_region_without_hiding_another() {
    let result = check_monitor_with_strong_fairness(
        &independent_progress_model(),
        &independent_progress_monitor(),
        &StrongFairness::new(["fair-a"]).unwrap(),
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

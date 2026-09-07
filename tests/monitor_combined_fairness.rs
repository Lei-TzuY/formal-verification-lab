use formal_verification_lab::monitor_examples::{
    invalid_double_open_protocol, session_monitor, stuck_committed_protocol,
};
use formal_verification_lab::{
    check_monitor, check_monitor_with_fairness_profile,
    check_monitor_with_fairness_profile_and_limits,
    check_monitor_with_fairness_profile_and_product_limits, check_monitor_with_limits,
    check_monitor_with_product_limits, check_monitor_with_strong_fairness,
    check_monitor_with_strong_fairness_and_limits,
    check_monitor_with_strong_fairness_and_product_limits, check_monitor_with_weak_fairness,
    check_monitor_with_weak_fairness_and_limits,
    check_monitor_with_weak_fairness_and_product_limits, AnalysisLimits, AnalysisOutcome,
    AnalysisStage, BoundedOutcome, ExplorationLimits, FairnessProfile, FiniteMonitor,
    InconclusiveReason, Invariant, MonitorCounterexample, MonitorStatus, ProgressCondition,
    RejectCondition, StateVariable, StrongFairness, Transition, TransitionSystem, WeakFairness,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MixedNode {
    Init,
    WeakLoop,
    StrongA,
    StrongB,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ProgressState {
    Active,
    Cleared,
}

fn mixed_fairness_model() -> TransitionSystem<MixedNode> {
    TransitionSystem::new(
        "mixed-fair-progress",
        vec![StateVariable::new("node", "fairness branch")],
        vec![MixedNode::Init],
        |state| match state {
            MixedNode::Init => Ok(vec![
                Transition::new("to-weak", MixedNode::WeakLoop),
                Transition::new("to-strong", MixedNode::StrongA),
            ]),
            MixedNode::WeakLoop => Ok(vec![
                Transition::new("spin-weak", MixedNode::WeakLoop),
                Transition::new("weak-exit", MixedNode::Done),
            ]),
            MixedNode::StrongA => Ok(vec![
                Transition::new("strong-cycle-a", MixedNode::StrongB),
                Transition::new("strong-exit", MixedNode::Done),
            ]),
            MixedNode::StrongB => Ok(vec![Transition::new("strong-cycle-b", MixedNode::StrongA)]),
            MixedNode::Done => Ok(Vec::new()),
        },
        vec![Invariant::new("recognized-node", |_state: &MixedNode| true)],
    )
    .unwrap()
}

fn mixed_progress_monitor() -> FiniteMonitor<ProgressState> {
    FiniteMonitor::new(
        "mixed-fair-progress-monitor",
        ProgressState::Active,
        |state, action| match action {
            "weak-exit" | "strong-exit" => ProgressState::Cleared,
            _ => *state,
        },
        Vec::<RejectCondition<ProgressState>>::new(),
        vec![ProgressCondition::new("eventually-cleared", |state| {
            *state == ProgressState::Active
        })
        .unwrap()],
    )
    .unwrap()
}

fn mixed_profile() -> FairnessProfile {
    FairnessProfile::new(["weak-exit"], ["strong-exit"]).unwrap()
}

fn active_terminal_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "combined-fair-active-terminal",
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

fn always_active_monitor() -> FiniteMonitor<bool> {
    FiniteMonitor::new(
        "combined-always-active",
        true,
        |state, _action| *state,
        Vec::<RejectCondition<bool>>::new(),
        vec![ProgressCondition::new("must-not-stay-active", |state| *state).unwrap()],
    )
    .unwrap()
}

#[test]
fn mixed_profile_is_the_intersection_of_weak_and_strong_execution_filters() {
    let model = mixed_fairness_model();
    let monitor = mixed_progress_monitor();

    let historical = check_monitor(&model, &monitor).unwrap();
    assert_eq!(historical.status, MonitorStatus::Violated);

    let weak_only = check_monitor_with_fairness_profile(
        &model,
        &monitor,
        &FairnessProfile::new(["weak-exit"], std::iter::empty::<&str>()).unwrap(),
    )
    .unwrap();
    assert_eq!(weak_only.status, MonitorStatus::Violated);

    let strong_only = check_monitor_with_fairness_profile(
        &model,
        &monitor,
        &FairnessProfile::new(std::iter::empty::<&str>(), ["strong-exit"]).unwrap(),
    )
    .unwrap();
    assert_eq!(strong_only.status, MonitorStatus::Violated);

    let mixed = check_monitor_with_fairness_profile(&model, &monitor, &mixed_profile()).unwrap();
    assert_eq!(mixed.status, MonitorStatus::Satisfied);
    assert!(mixed.counterexample.is_none());
}

#[test]
fn rejecting_state_keeps_global_precedence_under_a_mixed_profile() {
    let model = invalid_double_open_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let historical = check_monitor(&model, &monitor).unwrap();
    let mixed = check_monitor_with_fairness_profile(
        &model,
        &monitor,
        &FairnessProfile::new(["close"], ["commit"]).unwrap(),
    )
    .unwrap();

    assert_eq!(mixed, historical);
    assert!(matches!(
        mixed.counterexample,
        Some(MonitorCounterexample::Rejecting { .. })
    ));
}

#[test]
fn finite_active_terminal_is_not_excused_by_a_mixed_profile() {
    let result = check_monitor_with_fairness_profile(
        &active_terminal_model(),
        &always_active_monitor(),
        &FairnessProfile::new(["advance"], ["unrelated"]).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, MonitorStatus::Violated);
    assert!(matches!(
        result.counterexample,
        Some(MonitorCounterexample::ProgressTerminal { .. })
    ));
}

#[test]
fn empty_weak_only_and_strong_only_profiles_delegate_exactly() {
    let model = stuck_committed_protocol().unwrap();
    let monitor = session_monitor().unwrap();
    let product_limits = ExplorationLimits {
        max_states: Some(8),
        max_transitions: Some(8),
        max_depth: Some(8),
    };
    let analysis_limits = AnalysisLimits::new(product_limits, product_limits);

    let none = FairnessProfile::none();
    assert_eq!(
        check_monitor_with_fairness_profile(&model, &monitor, &none).unwrap(),
        check_monitor(&model, &monitor).unwrap()
    );
    assert_eq!(
        check_monitor_with_fairness_profile_and_product_limits(
            &model,
            &monitor,
            &none,
            product_limits,
        )
        .unwrap(),
        check_monitor_with_product_limits(&model, &monitor, product_limits).unwrap()
    );
    assert_eq!(
        check_monitor_with_fairness_profile_and_limits(&model, &monitor, &none, analysis_limits)
            .unwrap(),
        check_monitor_with_limits(&model, &monitor, analysis_limits).unwrap()
    );

    let weak = WeakFairness::new(["close"]).unwrap();
    let weak_profile = FairnessProfile::new(["close"], std::iter::empty::<&str>()).unwrap();
    assert_eq!(
        check_monitor_with_fairness_profile(&model, &monitor, &weak_profile).unwrap(),
        check_monitor_with_weak_fairness(&model, &monitor, &weak).unwrap()
    );
    assert_eq!(
        check_monitor_with_fairness_profile_and_product_limits(
            &model,
            &monitor,
            &weak_profile,
            product_limits,
        )
        .unwrap(),
        check_monitor_with_weak_fairness_and_product_limits(
            &model,
            &monitor,
            &weak,
            product_limits,
        )
        .unwrap()
    );
    assert_eq!(
        check_monitor_with_fairness_profile_and_limits(
            &model,
            &monitor,
            &weak_profile,
            analysis_limits,
        )
        .unwrap(),
        check_monitor_with_weak_fairness_and_limits(&model, &monitor, &weak, analysis_limits)
            .unwrap()
    );

    let strong = StrongFairness::new(["close"]).unwrap();
    let strong_profile = FairnessProfile::new(std::iter::empty::<&str>(), ["close"]).unwrap();
    assert_eq!(
        check_monitor_with_fairness_profile(&model, &monitor, &strong_profile).unwrap(),
        check_monitor_with_strong_fairness(&model, &monitor, &strong).unwrap()
    );
    assert_eq!(
        check_monitor_with_fairness_profile_and_product_limits(
            &model,
            &monitor,
            &strong_profile,
            product_limits,
        )
        .unwrap(),
        check_monitor_with_strong_fairness_and_product_limits(
            &model,
            &monitor,
            &strong,
            product_limits,
        )
        .unwrap()
    );
    assert_eq!(
        check_monitor_with_fairness_profile_and_limits(
            &model,
            &monitor,
            &strong_profile,
            analysis_limits,
        )
        .unwrap(),
        check_monitor_with_strong_fairness_and_limits(&model, &monitor, &strong, analysis_limits)
            .unwrap()
    );
}

#[test]
fn product_cutoff_stays_inconclusive_before_mixed_fair_progress_is_resolved() {
    let result = check_monitor_with_fairness_profile_and_product_limits(
        &mixed_fairness_model(),
        &mixed_progress_monitor(),
        &mixed_profile(),
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
fn staged_model_cutoff_reports_model_stage_under_a_mixed_profile() {
    let result = check_monitor_with_fairness_profile_and_limits(
        &mixed_fairness_model(),
        &mixed_progress_monitor(),
        &mixed_profile(),
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
fn generous_staged_limits_preserve_unbounded_mixed_profile_result() {
    let model = mixed_fairness_model();
    let monitor = mixed_progress_monitor();
    let profile = mixed_profile();
    let unbounded = check_monitor_with_fairness_profile(&model, &monitor, &profile).unwrap();
    let staged = check_monitor_with_fairness_profile_and_limits(
        &model,
        &monitor,
        &profile,
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

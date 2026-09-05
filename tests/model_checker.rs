use formal_verification_lab::checker::{
    check, check_with_limits, ExplorationLimits, InconclusiveReason, VerificationStatus,
};
use formal_verification_lab::examples::{
    bounded_counter, buggy_mutex, traffic_light, MutexState, Phase,
};
use formal_verification_lab::model::{
    Invariant, ModelError, StateVariable, Transition, TransitionSystem,
};
use formal_verification_lab::report::render_report;

#[test]
fn enumerates_all_reachable_counter_states() {
    let model = bounded_counter().unwrap();
    let result = check(&model).unwrap();

    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(result.discovered_states, 4);
    assert_eq!(result.checked_states, 4);
    assert_eq!(result.explored_transitions, 3);
    assert!(result.counterexample.is_none());
    assert!(result.inconclusive_reason.is_none());
}

#[test]
fn detects_invariant_failure() {
    let model = buggy_mutex().unwrap();
    let result = check(&model).unwrap();

    assert_eq!(result.status, VerificationStatus::Violated);
    assert!(result.inconclusive_reason.is_none());
    let counterexample = result.counterexample.expect("bug must be reachable");
    assert_eq!(counterexample.invariant, "mutual-exclusion");
    assert_eq!(
        counterexample.trace.last().unwrap().state,
        MutexState {
            p1: Phase::Critical,
            p2: Phase::Critical,
        }
    );
}

#[test]
fn reconstructs_a_shortest_counterexample() {
    let model = buggy_mutex().unwrap();
    let result = check(&model).unwrap();
    let trace = result.counterexample.unwrap().trace;

    assert_eq!(
        trace.len(),
        5,
        "four transitions are necessary to violate mutex"
    );
    let actions: Vec<_> = trace
        .iter()
        .filter_map(|step| step.action.as_deref())
        .collect();
    assert_eq!(
        actions,
        vec!["p1:request", "p1:enter", "p2:request", "p2:enter"]
    );
}

#[test]
fn terminates_on_cycles_and_deduplicates_states() {
    let model = traffic_light().unwrap();
    let result = check(&model).unwrap();

    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(result.discovered_states, 3);
    assert_eq!(result.checked_states, 3);
    assert_eq!(result.explored_transitions, 3);
}

#[test]
fn supports_multiple_initial_states_in_declared_order() {
    let model = TransitionSystem::new(
        "multiple-initial",
        vec![StateVariable::new("value", "small integer")],
        vec![0u8, 10u8, 0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("at-most-ten", |state: &u8| *state <= 10)],
    )
    .unwrap();

    let result = check(&model).unwrap();
    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(
        result.discovered_states, 2,
        "duplicate initials are visited once"
    );
    assert_eq!(result.checked_states, 2);
}

#[test]
fn output_is_deterministic() {
    let model = buggy_mutex().unwrap();
    let first = check(&model).unwrap();
    let second = check(&model).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        render_report(model.name(), &first),
        render_report(model.name(), &second)
    );
}

#[test]
fn rejects_malformed_model_metadata() {
    let duplicate_variable = TransitionSystem::new(
        "bad",
        vec![
            StateVariable::new("x", "first"),
            StateVariable::new("x", "second"),
        ],
        vec![0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("ok", |_state: &u8| true)],
    );
    assert!(matches!(
        duplicate_variable,
        Err(ModelError::DuplicateStateVariable { .. })
    ));

    let no_initial = TransitionSystem::new(
        "bad",
        vec![StateVariable::new("x", "value")],
        Vec::<u8>::new(),
        |_state| Ok(Vec::new()),
        vec![Invariant::new("ok", |_state: &u8| true)],
    );
    assert_eq!(no_initial.unwrap_err(), ModelError::NoInitialStates);

    let no_invariants = TransitionSystem::new(
        "bad",
        vec![StateVariable::new("x", "value")],
        vec![0u8],
        |_state| Ok(Vec::new()),
        Vec::new(),
    );
    assert_eq!(no_invariants.unwrap_err(), ModelError::NoInvariants);
}

#[test]
fn rejects_empty_transition_labels_during_exploration() {
    let model = TransitionSystem::new(
        "bad-edge",
        vec![StateVariable::new("x", "value")],
        vec![0u8],
        |state| Ok(vec![Transition::new("", *state + 1)]),
        vec![Invariant::new("ok", |_state: &u8| true)],
    )
    .unwrap();

    assert_eq!(
        check(&model).unwrap_err(),
        ModelError::EmptyTransitionAction
    );
}

#[test]
fn finds_initial_state_violation_with_zero_length_path() {
    let model = TransitionSystem::new(
        "bad-initial",
        vec![StateVariable::new("x", "value")],
        vec![7u8, 0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("must-be-zero", |state: &u8| *state == 0)],
    )
    .unwrap();

    let result = check(&model).unwrap();
    let counterexample = result.counterexample.unwrap();
    assert_eq!(counterexample.trace.len(), 1);
    assert_eq!(counterexample.trace[0].state, 7);
    assert!(counterexample.trace[0].action.is_none());
}

#[test]
fn state_limit_returns_inconclusive_instead_of_safe() {
    let model = bounded_counter().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_states: Some(3),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Inconclusive);
    assert_eq!(
        result.inconclusive_reason,
        Some(InconclusiveReason::StateLimitReached { limit: 3 })
    );
    assert_eq!(result.discovered_states, 3);
    assert_eq!(result.checked_states, 3);
    assert_eq!(result.explored_transitions, 3);
    assert!(result.counterexample.is_none());
}

#[test]
fn transition_limit_returns_inconclusive_instead_of_safe() {
    let model = bounded_counter().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_transitions: Some(2),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Inconclusive);
    assert_eq!(
        result.inconclusive_reason,
        Some(InconclusiveReason::TransitionLimitReached { limit: 2 })
    );
    assert_eq!(result.discovered_states, 3);
    assert_eq!(result.checked_states, 3);
    assert_eq!(result.explored_transitions, 2);
}

#[test]
fn depth_limit_returns_inconclusive_before_hidden_violation() {
    let model = buggy_mutex().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_depth: Some(3),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Inconclusive);
    assert_eq!(
        result.inconclusive_reason,
        Some(InconclusiveReason::DepthLimitReached { limit: 3 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn exact_resource_limits_can_still_prove_safety() {
    let model = bounded_counter().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_states: Some(4),
            max_transitions: Some(3),
            max_depth: Some(3),
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Safe);
    assert!(result.inconclusive_reason.is_none());
    assert_eq!(result.discovered_states, 4);
    assert_eq!(result.explored_transitions, 3);
}

#[test]
fn depth_boundary_can_prove_a_closed_cycle() {
    let model = traffic_light().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_depth: Some(2),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(result.discovered_states, 3);
    assert_eq!(result.explored_transitions, 3);
}

#[test]
fn sufficient_depth_preserves_shortest_counterexample() {
    let model = buggy_mutex().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_depth: Some(4),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Violated);
    let trace = result.counterexample.unwrap().trace;
    assert_eq!(trace.len(), 5);
    assert_eq!(
        trace.last().unwrap().state,
        MutexState {
            p1: Phase::Critical,
            p2: Phase::Critical,
        }
    );
}

#[test]
fn state_budget_must_cover_all_unique_initial_states() {
    let model = TransitionSystem::new(
        "multiple-initial-budget",
        vec![StateVariable::new("value", "small integer")],
        vec![0u8, 1u8, 0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("small", |state: &u8| *state <= 1)],
    )
    .unwrap();

    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_states: Some(1),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Inconclusive);
    assert_eq!(result.discovered_states, 1);
    assert_eq!(result.checked_states, 0);
    assert_eq!(
        result.inconclusive_reason,
        Some(InconclusiveReason::StateLimitReached { limit: 1 })
    );
}

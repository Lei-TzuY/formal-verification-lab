use formal_verification_lab::checker::{check, VerificationStatus};
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
}

#[test]
fn detects_invariant_failure() {
    let model = buggy_mutex().unwrap();
    let result = check(&model).unwrap();

    assert_eq!(result.status, VerificationStatus::Violated);
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

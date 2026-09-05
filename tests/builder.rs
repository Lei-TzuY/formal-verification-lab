use formal_verification_lab::{
    check, ModelError, Transition, TransitionSystemBuilder, VerificationStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TinyState(u8);

#[test]
fn builder_materializes_an_executable_transition_system() {
    let model = TransitionSystemBuilder::new("builder-counter", |state: &TinyState| {
        if state.0 < 2 {
            Ok(vec![Transition::new("increment", TinyState(state.0 + 1))])
        } else {
            Ok(Vec::new())
        }
    })
    .state_variable("value", "small counter")
    .initial_state(TinyState(0))
    .safety_invariant("bounded", |state: &TinyState| state.0 <= 2)
    .build()
    .unwrap();

    let result = check(&model).unwrap();
    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(result.discovered_states, 3);
    assert_eq!(result.checked_states, 3);
    assert_eq!(result.explored_transitions, 2);
}

#[test]
fn builder_delegates_to_canonical_model_validation() {
    let result =
        TransitionSystemBuilder::new("invalid-builder", |_state: &TinyState| Ok(Vec::new()))
            .state_variable("value", "first declaration")
            .state_variable("value", "duplicate declaration")
            .initial_state(TinyState(0))
            .safety_invariant("ok", |_state: &TinyState| true)
            .build();

    assert!(matches!(
        result,
        Err(ModelError::DuplicateStateVariable { name }) if name == "value"
    ));
}

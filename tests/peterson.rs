use formal_verification_lab::checker::{
    check, check_with_limits, ExplorationLimits, VerificationStatus,
};
use formal_verification_lab::examples::{
    buggy_peterson_mutex, peterson_mutex, PetersonPc, PetersonState,
};

#[test]
fn peterson_exhaustively_proves_mutual_exclusion() {
    let model = peterson_mutex().unwrap();
    let result = check(&model).unwrap();

    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(result.discovered_states, 20);
    assert_eq!(result.checked_states, 20);
    assert_eq!(result.explored_transitions, 34);
    assert!(result.counterexample.is_none());
    assert!(result.inconclusive_reason.is_none());
}

#[test]
fn peterson_is_safe_at_exact_exploration_bounds() {
    let model = peterson_mutex().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_states: Some(20),
            max_transitions: Some(34),
            max_depth: Some(6),
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Safe);
    assert_eq!(result.discovered_states, 20);
    assert_eq!(result.explored_transitions, 34);
}

#[test]
fn lost_intent_peterson_variant_has_shortest_mutex_counterexample() {
    let model = buggy_peterson_mutex().unwrap();
    let result = check(&model).unwrap();

    assert_eq!(result.status, VerificationStatus::Violated);
    let counterexample = result.counterexample.expect("bug must violate mutex");
    assert_eq!(counterexample.invariant, "mutual-exclusion");
    assert_eq!(counterexample.trace.len(), 7);

    let actions: Vec<_> = counterexample
        .trace
        .iter()
        .filter_map(|step| step.action.as_deref())
        .collect();
    assert_eq!(
        actions,
        vec![
            "p0:set-flag",
            "p0:set-turn",
            "p0:enter",
            "p1:set-flag",
            "p1:set-turn",
            "p1:enter",
        ]
    );

    let final_state = counterexample.trace.last().unwrap().state;
    assert_eq!(
        final_state,
        PetersonState {
            p0: PetersonPc::Critical,
            p1: PetersonPc::Critical,
            ..final_state
        }
    );
}

#[test]
fn insufficient_peterson_state_budget_is_inconclusive() {
    let model = peterson_mutex().unwrap();
    let result = check_with_limits(
        &model,
        ExplorationLimits {
            max_states: Some(19),
            ..ExplorationLimits::default()
        },
    )
    .unwrap();

    assert_eq!(result.status, VerificationStatus::Inconclusive);
    assert!(result.counterexample.is_none());
}

use formal_verification_lab::examples::commuting_counters;
use formal_verification_lab::{
    audit_sleep_set_reduction, IndependenceError, IndependenceRelation, Invariant, ReductionAuditError,
    StateVariable, Transition, TransitionSystem, VerificationStatus,
};

#[test]
fn independence_relation_is_explicit_symmetric_and_validated() {
    let relation = IndependenceRelation::new()
        .with_pair("left", "right")
        .unwrap();

    assert!(relation.independent("left", "right"));
    assert!(relation.independent("right", "left"));
    assert!(!relation.independent("left", "left"));
    assert_eq!(relation.pair_count(), 1);

    assert_eq!(
        IndependenceRelation::new().with_pair("", "right"),
        Err(IndependenceError::EmptyAction)
    );
    assert_eq!(
        IndependenceRelation::new().with_pair("same", "same"),
        Err(IndependenceError::ReflexiveAction {
            action: "same".to_owned()
        })
    );
}

#[test]
fn commuting_product_reduces_edges_but_is_checked_against_exhaustive_baseline() {
    let model = commuting_counters().unwrap();
    let relation = IndependenceRelation::new()
        .with_pair("left:increment", "right:increment")
        .unwrap();
    let audit = audit_sleep_set_reduction(&model, &relation).unwrap();

    assert_eq!(audit.exhaustive.status, VerificationStatus::Safe);
    assert_eq!(audit.reduced.status, VerificationStatus::Safe);
    assert_eq!(audit.exhaustive.discovered_states, 9);
    assert_eq!(audit.exhaustive.explored_transitions, 12);
    assert_eq!(audit.reduced.discovered_states, 9);
    assert_eq!(audit.reduced.explored_transitions, 8);
    assert_eq!(audit.reduced.pruned_transitions, 4);
    assert!(audit.reduced.counterexample.is_none());
}

#[test]
fn empty_independence_relation_degenerates_to_unpruned_exploration() {
    let model = commuting_counters().unwrap();
    let audit = audit_sleep_set_reduction(&model, &IndependenceRelation::new()).unwrap();

    assert_eq!(audit.exhaustive.status, audit.reduced.status);
    assert_eq!(audit.exhaustive.discovered_states, audit.reduced.discovered_states);
    assert_eq!(audit.exhaustive.explored_transitions, audit.reduced.explored_transitions);
    assert_eq!(audit.reduced.pruned_transitions, 0);
}

#[test]
fn incorrect_independence_declaration_fails_closed_on_semantic_mismatch() {
    let model = TransitionSystem::new(
        "non-commuting-diamond",
        vec![StateVariable::new("state", "small control state")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("a", 1), Transition::new("b", 2)],
                1 => vec![Transition::new("b", 3)],
                2 => vec![Transition::new("a", 4)],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("avoid-four", |state: &u8| *state != 4)],
    )
    .unwrap();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();

    assert_eq!(
        audit_sleep_set_reduction(&model, &relation),
        Err(ReductionAuditError::SemanticMismatch {
            exhaustive: VerificationStatus::Violated,
            reduced: VerificationStatus::Safe,
        })
    );
}

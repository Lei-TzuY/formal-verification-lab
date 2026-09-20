use formal_verification_lab::examples::commuting_counters;
use formal_verification_lab::{
    audit_sleep_set_reduction, check, check_validated_sleep_set_reduction, validate_independence,
    IndependenceError, IndependenceRelation, IndependenceValidationError, Invariant,
    ReductionAuditError, StateVariable, Transition, TransitionSystem, VerificationStatus,
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
fn validated_commuting_product_runs_without_second_status_comparison() {
    let model = commuting_counters().unwrap();
    let relation = IndependenceRelation::new()
        .with_pair("left:increment", "right:increment")
        .unwrap();

    let validated = validate_independence(&model, &relation).unwrap();
    assert_eq!(validated.pair_count(), 1);
    assert_eq!(validated.validation_states(), 9);
    assert_eq!(validated.validation_transitions(), 12);

    let reduced = check_validated_sleep_set_reduction(&validated);
    assert_eq!(reduced.status, VerificationStatus::Safe);
    assert_eq!(reduced.discovered_states, 9);
    assert_eq!(reduced.explored_transitions, 8);
    assert_eq!(reduced.pruned_transitions, 4);
    assert!(reduced.counterexample.is_none());
}

#[test]
fn empty_independence_relation_degenerates_to_unpruned_exploration() {
    let model = commuting_counters().unwrap();
    let audit = audit_sleep_set_reduction(&model, &IndependenceRelation::new()).unwrap();

    assert_eq!(audit.exhaustive.status, audit.reduced.status);
    assert_eq!(
        audit.exhaustive.discovered_states,
        audit.reduced.discovered_states
    );
    assert_eq!(
        audit.exhaustive.explored_transitions,
        audit.reduced.explored_transitions
    );
    assert_eq!(audit.reduced.pruned_transitions, 0);
}

#[test]
fn incorrect_independence_declaration_fails_closed_on_semantic_mismatch() {
    let model = non_commuting_model();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();

    assert_eq!(
        audit_sleep_set_reduction(&model, &relation),
        Err(ReductionAuditError::SemanticMismatch {
            exhaustive: VerificationStatus::Violated,
            reduced: VerificationStatus::Safe,
        })
    );
}

#[test]
fn validator_rejects_non_commuting_diamond_before_reduction() {
    let model = non_commuting_model();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();

    let error = validate_independence(&model, &relation).unwrap_err();
    assert!(matches!(
        error,
        IndependenceValidationError::NonCommuting {
            state_index: 0,
            left,
            right,
            ..
        } if left == "a" && right == "b"
    ));
}

#[test]
fn validator_rejects_enabledness_changes() {
    let model = TransitionSystem::new(
        "enabledness-change",
        vec![StateVariable::new("state", "small control state")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("a", 1), Transition::new("b", 2)],
                1 => Vec::new(),
                2 => vec![Transition::new("a", 3)],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();

    let error = validate_independence(&model, &relation).unwrap_err();
    assert!(matches!(
        error,
        IndependenceValidationError::EnablednessChanged {
            state_index: 0,
            action,
            by_action,
            expected_enabled: true,
            observed_enabled: false,
            ..
        } if action == "b" && by_action == "a"
    ));
}

#[test]
fn validator_rejects_ambiguous_configured_action_successors() {
    let model = TransitionSystem::new(
        "ambiguous-action",
        vec![StateVariable::new("state", "small control state")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![
                    Transition::new("a", 1),
                    Transition::new("a", 2),
                    Transition::new("b", 3),
                ],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();

    assert_eq!(
        validate_independence(&model, &relation).unwrap_err(),
        IndependenceValidationError::AmbiguousAction {
            state_index: 0,
            action: "a".to_owned(),
            successors: 2,
        }
    );
}

#[test]
fn validator_rejects_commuting_orders_that_change_invariant_observation() {
    let model = TransitionSystem::new(
        "observation-changing-diamond",
        vec![StateVariable::new("state", "small control state")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("a", 1), Transition::new("b", 2)],
                1 => vec![Transition::new("b", 3)],
                2 => vec![Transition::new("a", 3)],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("avoid-two", |state: &u8| *state != 2)],
    )
    .unwrap();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();

    let error = validate_independence(&model, &relation).unwrap_err();
    assert!(matches!(
        error,
        IndependenceValidationError::InvariantObservationMismatch {
            state_index: 0,
            left,
            right,
            invariant,
            ..
        } if left == "a" && right == "b" && invariant == "avoid-two"
    ));
}

#[test]
fn reducer_preserves_distinct_sleep_contexts_for_the_same_model_state() {
    let model = TransitionSystem::new(
        "sleep-context-self-loops",
        vec![StateVariable::new("state", "single state")],
        vec![0u8],
        |_state| Ok(vec![Transition::new("a", 0), Transition::new("b", 0)]),
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();
    let validated = validate_independence(&model, &relation).unwrap();

    let reduced = check_validated_sleep_set_reduction(&validated);
    assert_eq!(reduced.status, VerificationStatus::Safe);
    assert_eq!(reduced.discovered_states, 1);
    assert_eq!(reduced.checked_states, 2);
    assert_eq!(reduced.explored_transitions, 3);
    assert_eq!(reduced.pruned_transitions, 1);
}

#[test]
fn validated_reduction_counterexample_is_deterministic() {
    let model = TransitionSystem::new(
        "deterministic-reduced-witness",
        vec![StateVariable::new("state", "small control state")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![
                    Transition::new("a", 0),
                    Transition::new("b", 0),
                    Transition::new("fail", 1),
                ],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("avoid-one", |state: &u8| *state != 1)],
    )
    .unwrap();
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();
    let validated = validate_independence(&model, &relation).unwrap();

    let first = check_validated_sleep_set_reduction(&validated);
    let second = check_validated_sleep_set_reduction(&validated);
    assert_eq!(first, second);
    assert_eq!(first.status, VerificationStatus::Violated);
    let counterexample = first.counterexample.unwrap();
    assert_eq!(counterexample.invariant, "avoid-one");
    // The reducer is deterministic depth-first search, not the canonical BFS,
    // so this contract is reproducibility rather than shortest-witness length.
    assert_eq!(counterexample.trace.len(), 3);
    assert_eq!(counterexample.trace[0].action, None);
    assert_eq!(counterexample.trace[0].state, 0);
    assert_eq!(counterexample.trace[1].action.as_deref(), Some("b"));
    assert_eq!(counterexample.trace[1].state, 0);
    assert_eq!(counterexample.trace[2].action.as_deref(), Some("fail"));
    assert_eq!(counterexample.trace[2].state, 1);
}

#[test]
fn generated_validated_reductions_match_exhaustive_safety() {
    let relation = IndependenceRelation::new().with_pair("a", "b").unwrap();
    let mut validated_cases = 0usize;

    for graph_code in 0..4096usize {
        let table = decode_three_state_two_action_graph(graph_code);
        for unsafe_mask in 0..8u8 {
            let model = generated_model(table, unsafe_mask);
            let exhaustive = check(&model).unwrap();

            match validate_independence(&model, &relation) {
                Ok(validated) => {
                    validated_cases += 1;
                    let reduced = check_validated_sleep_set_reduction(&validated);
                    assert_eq!(
                        reduced.status, exhaustive.status,
                        "validated reduction mismatch for graph_code={graph_code}, unsafe_mask={unsafe_mask}, table={table:?}"
                    );
                }
                Err(IndependenceValidationError::Model(error)) => {
                    panic!("generated model unexpectedly failed: {error}")
                }
                Err(
                    IndependenceValidationError::SnapshotInvariant
                    | IndependenceValidationError::AmbiguousAction { .. }
                    | IndependenceValidationError::EnablednessChanged { .. }
                    | IndependenceValidationError::NonCommuting { .. }
                    | IndependenceValidationError::InvariantObservationMismatch { .. },
                ) => {}
            }
        }
    }

    assert!(validated_cases > 0);
}

fn non_commuting_model() -> TransitionSystem<u8> {
    TransitionSystem::new(
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
    .unwrap()
}

fn decode_three_state_two_action_graph(mut code: usize) -> [[Option<u8>; 2]; 3] {
    let mut table = [[None; 2]; 3];
    for state in &mut table {
        for target in state {
            let digit = code % 4;
            code /= 4;
            *target = if digit == 0 {
                None
            } else {
                Some((digit - 1) as u8)
            };
        }
    }
    table
}

fn generated_model(table: [[Option<u8>; 2]; 3], unsafe_mask: u8) -> TransitionSystem<u8> {
    TransitionSystem::new(
        "generated-reduction-graph",
        vec![StateVariable::new("state", "generated state")],
        vec![0u8],
        move |state| {
            let row = table[*state as usize];
            let mut transitions = Vec::new();
            if let Some(target) = row[0] {
                transitions.push(Transition::new("a", target));
            }
            if let Some(target) = row[1] {
                transitions.push(Transition::new("b", target));
            }
            Ok(transitions)
        },
        vec![Invariant::new("generated-safety", move |state: &u8| {
            unsafe_mask & (1 << *state) == 0
        })],
    )
    .unwrap()
}

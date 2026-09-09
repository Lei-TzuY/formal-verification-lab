use formal_verification_lab::{
    check_deadlock_with_limits, check_declarative_deadlock_with_limits, parse_declarative_deadlock_spec,
    parse_declarative_document, BoundedOutcome, DeadlockProperty, DeadlockStatus, ExplorationLimits,
    InconclusiveReason, PropositionExpressionError,
};

fn document() -> formal_verification_lab::DeclarativeDocument {
    parse_declarative_document(
        r#"
model "workflow"
state "run"
state "done"
state "cancelled"
initial "run"
edge "run" "finish" "done"
label "done" "done"
label "cancelled" "cancelled"
"#,
    )
    .unwrap()
}

#[test]
fn boolean_terminal_policy_is_parsed_canonically_and_proves_deadlock_free() {
    let document = document();
    let spec = parse_declarative_deadlock_spec("terminal-policy", "\"done\" or \"cancelled\"")
        .unwrap();
    let result = check_declarative_deadlock_with_limits(
        &document,
        &spec,
        ExplorationLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(spec.canonical_expression(), "(\"done\" or \"cancelled\")");
    assert_eq!(result.expression, "(\"done\" or \"cancelled\")");
    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFree)
    );
    assert_eq!(result.discovered_states, 2);
    assert_eq!(result.checked_states, 2);
    assert_eq!(result.explored_transitions, 1);
    assert!(result.witness.is_none());
}

#[test]
fn declarative_policy_preserves_direct_backend_shortest_witness_and_accounting() {
    let document = document();
    let spec = parse_declarative_deadlock_spec("terminal-policy", "\"cancelled\"").unwrap();
    let frontend = check_declarative_deadlock_with_limits(
        &document,
        &spec,
        ExplorationLimits::unbounded(),
    )
    .unwrap();
    let direct_property =
        DeadlockProperty::new("terminal-policy", |state: &String| state == "cancelled").unwrap();
    let direct = check_deadlock_with_limits(
        document.model(),
        &direct_property,
        ExplorationLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(frontend.outcome, direct.outcome);
    assert_eq!(frontend.discovered_states, direct.discovered_states);
    assert_eq!(frontend.checked_states, direct.checked_states);
    assert_eq!(frontend.explored_transitions, direct.explored_transitions);
    assert_eq!(frontend.max_depth_reached, direct.max_depth_reached);
    assert_eq!(frontend.witness, direct.witness);
    let witness = frontend.witness.unwrap();
    assert_eq!(witness.len(), 2);
    assert_eq!(witness.last().unwrap().state, "done");
}

#[test]
fn unknown_proposition_fails_before_backend_execution() {
    let document = document();
    let spec = parse_declarative_deadlock_spec("terminal-policy", "\"missing\"").unwrap();
    let error = check_declarative_deadlock_with_limits(
        &document,
        &spec,
        ExplorationLimits::unbounded(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        formal_verification_lab::DeclarativeDeadlockError::PropositionExpression(
            PropositionExpressionError::UnknownProposition { proposition }
        ) if proposition == "missing"
    ));
}

#[test]
fn model_cutoff_is_inconclusive_and_never_fabricates_terminal_evidence() {
    let document = document();
    let spec = parse_declarative_deadlock_spec("terminal-policy", "\"done\"").unwrap();
    let result = check_declarative_deadlock_with_limits(
        &document,
        &spec,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
    );
    assert_eq!(result.discovered_states, 1);
    assert_eq!(result.checked_states, 1);
    assert_eq!(result.explored_transitions, 0);
    assert!(result.witness.is_none());
}

#[test]
fn real_deadlock_found_before_a_later_budget_remains_conclusive() {
    let document = document();
    let spec = parse_declarative_deadlock_spec("terminal-policy", "\"cancelled\"").unwrap();
    let result = check_declarative_deadlock_with_limits(
        &document,
        &spec,
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(1),
            max_depth: Some(1),
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFound)
    );
    assert_eq!(result.witness.unwrap().last().unwrap().state, "done");
}

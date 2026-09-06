use formal_verification_lab::multi_response::{
    check_multi_response, check_multi_response_with_product_limits, MultiResponseCounterexample,
    MultiResponseProperty, MultiResponseStatus, ResponseClause,
};
use formal_verification_lab::{
    BoundedOutcome, ExplorationLimits, InconclusiveReason, Invariant, StateVariable, Transition,
    TransitionSystem,
};

fn response_property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "request-eventually-grant",
        vec![ResponseClause::new(
            "request",
            |action| action == "request",
            |action| action == "grant",
        )
        .unwrap()],
    )
    .unwrap()
}

fn model(
    name: &'static str,
    transitions: impl Fn(&usize) -> Vec<Transition<usize>> + Send + Sync + 'static,
) -> TransitionSystem<usize> {
    TransitionSystem::new(
        name,
        vec![StateVariable::new("node", "protocol state")],
        vec![0usize],
        move |state| Ok(transitions(state)),
        vec![Invariant::new("known-node", |state: &usize| *state < 3)],
    )
    .unwrap()
}

#[test]
fn partial_outgoing_cutoff_is_not_fabricated_as_a_pending_terminal() {
    let model = model("partial-nonterminal", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 2)],
        _ => Vec::new(),
    });
    let result = check_multi_response_with_product_limits(
        &model,
        &response_property(),
        ExplorationLimits {
            max_states: None,
            max_transitions: None,
            max_depth: Some(1),
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    assert_eq!(result.product_states, 2);
    assert_eq!(result.checked_product_states, 2);
    assert_eq!(result.explored_product_transitions, 2);
    assert_eq!(result.retained_product_transitions, 1);
    assert_eq!(result.max_product_depth_reached, Some(1));
    assert!(result.counterexample.is_none());
}

#[test]
fn real_pending_terminal_at_an_exact_depth_bound_remains_conclusive() {
    let model = model("real-pending-terminal", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        _ => Vec::new(),
    });
    let result = check_multi_response_with_product_limits(
        &model,
        &response_property(),
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(1),
            max_depth: Some(1),
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated)
    );
    assert_eq!(result.product_states, 2);
    assert_eq!(result.checked_product_states, 2);
    assert_eq!(result.explored_product_transitions, 1);
    assert_eq!(result.retained_product_transitions, 1);
    assert_eq!(result.max_product_depth_reached, Some(1));

    let Some(MultiResponseCounterexample::Finite { clause, trace }) = result.counterexample else {
        panic!("expected a real finite pending-terminal witness");
    };
    assert_eq!(clause, "request");
    assert_eq!(trace.len(), 2);
    assert!(trace.last().unwrap().state.pending[0]);
}

#[test]
fn real_pending_cycle_before_a_later_transition_cutoff_remains_conclusive() {
    let model = model("partial-real-cycle", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("wait", 1), Transition::new("grant", 2)],
        _ => Vec::new(),
    });
    let result = check_multi_response_with_product_limits(
        &model,
        &response_property(),
        ExplorationLimits {
            max_states: None,
            max_transitions: Some(2),
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated)
    );
    assert_eq!(result.product_states, 2);
    assert_eq!(result.checked_product_states, 2);
    assert_eq!(result.explored_product_transitions, 2);
    assert_eq!(result.retained_product_transitions, 2);

    let Some(MultiResponseCounterexample::Infinite {
        clause,
        stem,
        cycle,
    }) = result.counterexample
    else {
        panic!("expected a real pending-cycle witness");
    };
    assert_eq!(clause, "request");
    assert!(stem.last().unwrap().state.pending[0]);
    assert!(cycle.iter().all(|step| step.state.pending[0]));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle.iter().any(|step| step.action.as_deref() == Some("wait")));
}

#[test]
fn exact_product_limits_can_still_prove_response_satisfaction() {
    let model = model("bounded-satisfied-cycle", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 0)],
        _ => Vec::new(),
    });
    let result = check_multi_response_with_product_limits(
        &model,
        &response_property(),
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(2),
            max_depth: Some(1),
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied)
    );
    assert_eq!(result.product_states, 2);
    assert_eq!(result.checked_product_states, 2);
    assert_eq!(result.explored_product_transitions, 2);
    assert_eq!(result.retained_product_transitions, 2);
    assert_eq!(result.max_product_depth_reached, Some(1));
    assert!(result.counterexample.is_none());
}

#[test]
fn zero_product_state_budget_is_honestly_inconclusive_after_model_capture() {
    let model = model("zero-product-budget", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        _ => Vec::new(),
    });
    let result = check_multi_response_with_product_limits(
        &model,
        &response_property(),
        ExplorationLimits {
            max_states: Some(0),
            max_transitions: None,
            max_depth: None,
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 0 })
    );
    assert_eq!(result.model_states, 2);
    assert_eq!(result.model_transitions, 1);
    assert_eq!(result.product_states, 0);
    assert_eq!(result.checked_product_states, 0);
    assert_eq!(result.explored_product_transitions, 0);
    assert_eq!(result.retained_product_transitions, 0);
    assert_eq!(result.max_product_depth_reached, None);
    assert!(result.counterexample.is_none());
}

#[test]
fn unbounded_product_limits_are_exactly_equivalent_to_the_legacy_api() {
    let model = model("unbounded-equivalence", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("wait", 1), Transition::new("grant", 0)],
        _ => Vec::new(),
    });
    let property = response_property();
    let direct = check_multi_response(&model, &property).unwrap();
    let bounded = check_multi_response_with_product_limits(
        &model,
        &property,
        ExplorationLimits::unbounded(),
    )
    .unwrap();

    assert_eq!(
        bounded.outcome,
        BoundedOutcome::Conclusive(direct.status)
    );
    assert_eq!(bounded.property, direct.property);
    assert_eq!(bounded.model_states, direct.model_states);
    assert_eq!(bounded.model_transitions, direct.model_transitions);
    assert_eq!(bounded.product_states, direct.product_states);
    assert_eq!(
        bounded.explored_product_transitions,
        direct.product_transitions
    );
    assert_eq!(
        bounded.retained_product_transitions,
        direct.product_transitions
    );
    assert_eq!(bounded.counterexample, direct.counterexample);
}

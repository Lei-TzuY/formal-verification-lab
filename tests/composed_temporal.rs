use formal_verification_lab::buchi_examples::unfair_second_pulse;
use formal_verification_lab::response_examples::unfair_request_grant_protocol;
use formal_verification_lab::temporal::{
    check_action_temporal, check_action_temporal_with_limits, ActionAtom, ActionTemporalSpec,
    TemporalBackend, TemporalCounterexample, TemporalObligation, TemporalStatus,
};
use formal_verification_lab::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
    ExplorationLimits, InconclusiveReason, Invariant, StateVariable, Transition, TransitionSystem,
};

fn model(
    name: &'static str,
    transitions: impl Fn(&usize) -> Vec<Transition<usize>> + Send + Sync + 'static,
) -> TransitionSystem<usize> {
    TransitionSystem::new(
        name,
        vec![StateVariable::new("node", "temporal protocol state")],
        vec![0usize],
        move |state| Ok(transitions(state)),
        vec![Invariant::new("known-node", |state: &usize| *state < 4)],
    )
    .unwrap()
}

fn limits(
    states: Option<usize>,
    transitions: Option<usize>,
    depth: Option<usize>,
) -> ExplorationLimits {
    ExplorationLimits {
        max_states: states,
        max_transitions: transitions,
        max_depth: depth,
    }
}

fn response_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::response(
        "request-eventually-grant",
        ActionAtom::exact("request").unwrap(),
        ActionAtom::exact("grant").unwrap(),
    )
    .unwrap()
}

fn recurring_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::all_infinitely_often(
        "both-pulses-recur",
        vec![
            ActionAtom::exact("pulse-a").unwrap(),
            ActionAtom::exact("pulse-b").unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn response_model_cutoff_remains_frontend_inconclusive_without_fake_terminal() {
    let model = model("response-prefix", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 2)],
        _ => Vec::new(),
    });

    let result = check_action_temporal_with_limits(
        &model,
        &response_spec(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(result.backend, TemporalBackend::Response);
    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::DepthLimitReached { limit: 1 },
        })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn response_cycle_is_normalized_and_conclusive_before_later_model_cutoff() {
    let model = model("response-cycle", |state| match *state {
        0 => vec![Transition::new("request", 1), Transition::new("branch", 2)],
        1 => vec![Transition::new("wait", 1)],
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });

    let result = check_action_temporal_with_limits(
        &model,
        &response_spec(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(result.backend, TemporalBackend::Response);
    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(TemporalStatus::Violated)
    );
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    let Some(TemporalCounterexample::Infinite {
        obligation,
        cycle,
        ..
    }) = result.counterexample
    else {
        panic!("expected normalized response lasso");
    };
    assert_eq!(obligation, TemporalObligation::Response);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn recurring_cycle_is_normalized_and_identifies_missing_action() {
    let model = model("recurring-cycle", |state| match *state {
        0 => vec![Transition::new("pulse-a", 1), Transition::new("branch", 2)],
        1 => vec![Transition::new("pulse-a", 1)],
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });

    let result = check_action_temporal_with_limits(
        &model,
        &recurring_spec(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(result.backend, TemporalBackend::Buchi);
    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(TemporalStatus::Violated)
    );
    let Some(TemporalCounterexample::Infinite {
        obligation,
        cycle,
        ..
    }) = result.counterexample
    else {
        panic!("expected normalized recurring lasso");
    };
    assert_eq!(
        obligation,
        TemporalObligation::InfinitelyOftenAction("pulse-b".to_owned())
    );
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn frontend_preserves_model_before_product_inconclusive_precedence() {
    let model = model("both-cutoffs", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 2)],
        _ => Vec::new(),
    });

    let result = check_action_temporal_with_limits(
        &model,
        &response_spec(),
        AnalysisLimits::new(limits(None, None, Some(1)), limits(Some(1), None, None)),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::DepthLimitReached { limit: 1 },
        })
    );
    assert_eq!(
        result.product_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
}

#[test]
fn fully_unbounded_staged_response_frontend_matches_legacy_result() {
    let model = unfair_request_grant_protocol().unwrap();
    let spec = response_spec();
    let legacy = check_action_temporal(&model, &spec).unwrap();
    let staged = check_action_temporal_with_limits(&model, &spec, AnalysisLimits::unbounded()).unwrap();

    assert_eq!(staged.backend, legacy.backend);
    assert_eq!(staged.outcome, AnalysisOutcome::Conclusive(legacy.status));
    assert_eq!(staged.model_states, legacy.model_states);
    assert_eq!(staged.explored_model_transitions, legacy.model_transitions);
    assert_eq!(staged.product_states, legacy.product_states);
    assert_eq!(staged.explored_product_transitions, legacy.product_transitions);
    assert_eq!(staged.counterexample, legacy.counterexample);
}

#[test]
fn fully_unbounded_staged_recurring_frontend_matches_legacy_result() {
    let model = unfair_second_pulse().unwrap();
    let spec = recurring_spec();
    let legacy = check_action_temporal(&model, &spec).unwrap();
    let staged = check_action_temporal_with_limits(&model, &spec, AnalysisLimits::unbounded()).unwrap();

    assert_eq!(staged.backend, legacy.backend);
    assert_eq!(staged.outcome, AnalysisOutcome::Conclusive(legacy.status));
    assert_eq!(staged.model_states, legacy.model_states);
    assert_eq!(staged.explored_model_transitions, legacy.model_transitions);
    assert_eq!(staged.product_states, legacy.product_states);
    assert_eq!(staged.explored_product_transitions, legacy.product_transitions);
    assert_eq!(staged.counterexample, legacy.counterexample);
}

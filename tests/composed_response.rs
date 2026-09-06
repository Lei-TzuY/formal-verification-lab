use formal_verification_lab::multi_response::{
    check_multi_response, check_multi_response_with_limits, check_multi_response_with_product_limits,
    MultiResponseCounterexample, MultiResponseProperty, MultiResponseStatus, ResponseClause,
};
use formal_verification_lab::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
    ExplorationLimits, InconclusiveReason, Invariant, StateVariable, Transition, TransitionSystem,
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

#[test]
fn model_depth_cutoff_does_not_fabricate_a_pending_terminal() {
    let model = model("model-prefix-nonterminal", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 2)],
        _ => Vec::new(),
    });

    let result = check_multi_response_with_limits(
        &model,
        &response_property(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
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
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    assert_eq!(result.product_completion, BoundedOutcome::Conclusive(()));
    assert_eq!(result.model_states, 2);
    assert_eq!(result.retained_model_transitions, 1);
    assert_eq!(result.product_states, 2);
    assert!(result.counterexample.is_none());
}

#[test]
fn proven_pending_terminal_survives_a_later_model_cutoff() {
    let model = model("terminal-before-model-cutoff", |state| match *state {
        0 => vec![
            Transition::new("request", 1),
            Transition::new("branch", 2),
        ],
        1 => Vec::new(),
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });

    let result = check_multi_response_with_limits(
        &model,
        &response_property(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(MultiResponseStatus::Violated)
    );
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    let Some(MultiResponseCounterexample::Finite { clause, trace }) = result.counterexample else {
        panic!("expected a justified pending-terminal witness");
    };
    assert_eq!(clause, "request");
    assert_eq!(trace.len(), 2);
    assert_eq!(trace.last().unwrap().state.state, 1);
    assert!(trace.last().unwrap().state.pending[0]);
}

#[test]
fn retained_pending_cycle_survives_a_later_model_cutoff() {
    let model = model("cycle-before-model-cutoff", |state| match *state {
        0 => vec![
            Transition::new("request", 1),
            Transition::new("branch", 2),
        ],
        1 => vec![Transition::new("wait", 1)],
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });

    let result = check_multi_response_with_limits(
        &model,
        &response_property(),
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(MultiResponseStatus::Violated)
    );
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    let Some(MultiResponseCounterexample::Infinite {
        clause,
        stem,
        cycle,
    }) = result.counterexample
    else {
        panic!("expected a justified pending-cycle witness");
    };
    assert_eq!(clause, "request");
    assert_eq!(stem.last().unwrap().state.state, 1);
    assert!(cycle.iter().all(|step| step.state.pending[0]));
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("wait")));
}

#[test]
fn model_stage_has_deterministic_precedence_when_both_stages_cut_off() {
    let model = model("both-stages-cut-off", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 2)],
        _ => Vec::new(),
    });

    let result = check_multi_response_with_limits(
        &model,
        &response_property(),
        AnalysisLimits::new(
            limits(None, None, Some(1)),
            limits(Some(1), None, None),
        ),
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
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    assert_eq!(
        result.product_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
    assert!(result.counterexample.is_none());
}

#[test]
fn unbounded_model_stage_matches_existing_product_only_api() {
    let model = model("product-only-equivalence", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("wait", 1), Transition::new("grant", 0)],
        _ => Vec::new(),
    });
    let property = response_property();
    let product_limits = limits(None, Some(2), None);

    let product_only =
        check_multi_response_with_product_limits(&model, &property, product_limits).unwrap();
    let composed = check_multi_response_with_limits(
        &model,
        &property,
        AnalysisLimits::new(ExplorationLimits::unbounded(), product_limits),
    )
    .unwrap();

    assert_eq!(composed.model_completion, BoundedOutcome::Conclusive(()));
    assert_eq!(
        composed.outcome,
        match product_only.outcome {
            BoundedOutcome::Conclusive(status) => AnalysisOutcome::Conclusive(status),
            BoundedOutcome::Inconclusive(reason) => {
                AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
                    stage: AnalysisStage::Product,
                    reason,
                })
            }
        }
    );
    assert_eq!(composed.model_states, product_only.model_states);
    assert_eq!(
        composed.explored_model_transitions,
        product_only.model_transitions
    );
    assert_eq!(composed.product_states, product_only.product_states);
    assert_eq!(
        composed.checked_product_states,
        product_only.checked_product_states
    );
    assert_eq!(
        composed.explored_product_transitions,
        product_only.explored_product_transitions
    );
    assert_eq!(
        composed.retained_product_transitions,
        product_only.retained_product_transitions
    );
    assert_eq!(composed.counterexample, product_only.counterexample);
}

#[test]
fn exact_model_budget_and_unbounded_product_can_prove_satisfaction() {
    let model = model("exact-model-budget", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("grant", 0)],
        _ => Vec::new(),
    });

    let result = check_multi_response_with_limits(
        &model,
        &response_property(),
        AnalysisLimits::new(
            limits(Some(2), Some(2), Some(1)),
            ExplorationLimits::unbounded(),
        ),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        AnalysisOutcome::Conclusive(MultiResponseStatus::Satisfied)
    );
    assert_eq!(result.model_completion, BoundedOutcome::Conclusive(()));
    assert_eq!(result.product_completion, BoundedOutcome::Conclusive(()));
    assert_eq!(result.model_states, 2);
    assert_eq!(result.explored_model_transitions, 2);
    assert_eq!(result.product_states, 2);
    assert!(result.counterexample.is_none());
}

#[test]
fn fully_unbounded_composed_analysis_is_exactly_legacy_equivalent() {
    let model = model("unbounded-composed-equivalence", |state| match *state {
        0 => vec![Transition::new("request", 1)],
        1 => vec![Transition::new("wait", 1), Transition::new("grant", 0)],
        _ => Vec::new(),
    });
    let property = response_property();
    let direct = check_multi_response(&model, &property).unwrap();
    let composed =
        check_multi_response_with_limits(&model, &property, AnalysisLimits::unbounded()).unwrap();

    assert_eq!(
        composed.outcome,
        AnalysisOutcome::Conclusive(direct.status)
    );
    assert_eq!(composed.property, direct.property);
    assert_eq!(composed.model_states, direct.model_states);
    assert_eq!(composed.explored_model_transitions, direct.model_transitions);
    assert_eq!(composed.product_states, direct.product_states);
    assert_eq!(
        composed.explored_product_transitions,
        direct.product_transitions
    );
    assert_eq!(
        composed.retained_product_transitions,
        direct.product_transitions
    );
    assert_eq!(composed.counterexample, direct.counterexample);
}

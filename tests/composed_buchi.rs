use formal_verification_lab::buchi::{
    check_buchi, check_buchi_with_limits, BuchiCounterexample, BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{pulse_automaton, unfair_second_pulse};
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
        vec![StateVariable::new("node", "pulse protocol state")],
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
fn model_cutoff_does_not_fabricate_strict_terminal_failure() {
    let model = model("strict-prefix-nonterminal", |state| match *state {
        0 => vec![Transition::new("quiet", 1)],
        1 => vec![Transition::new("pulse-both", 2)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();

    let result = check_buchi_with_limits(
        &model,
        &automaton,
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
    assert!(result.counterexample.is_none());
}

#[test]
fn proven_strict_terminal_survives_later_model_cutoff() {
    let model = model("strict-terminal-before-cutoff", |state| match *state {
        0 => vec![Transition::new("quiet", 1), Transition::new("branch", 2)],
        1 => Vec::new(),
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();

    let result = check_buchi_with_limits(
        &model,
        &automaton,
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(result.outcome, AnalysisOutcome::Conclusive(BuchiStatus::Violated));
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    let Some(BuchiCounterexample::FiniteTerminal {
        missing_acceptance,
        trace,
    }) = result.counterexample
    else {
        panic!("expected strict finite-terminal witness");
    };
    assert_eq!(missing_acceptance, "pulse-a-observed");
    assert_eq!(trace.last().unwrap().state.state, 1);
}

#[test]
fn retained_acceptance_avoiding_cycle_survives_later_model_cutoff() {
    let model = model("lasso-before-cutoff", |state| match *state {
        0 => vec![Transition::new("pulse-a", 1), Transition::new("branch", 2)],
        1 => vec![Transition::new("pulse-a", 1)],
        2 => vec![Transition::new("later", 3)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();

    let result = check_buchi_with_limits(
        &model,
        &automaton,
        AnalysisLimits::new(limits(None, None, Some(1)), ExplorationLimits::unbounded()),
    )
    .unwrap();

    assert_eq!(result.outcome, AnalysisOutcome::Conclusive(BuchiStatus::Violated));
    assert_eq!(
        result.model_completion,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 1 })
    );
    let Some(BuchiCounterexample::AcceptanceAvoidingCycle {
        acceptance,
        cycle,
        ..
    }) = result.counterexample
    else {
        panic!("expected acceptance-avoiding lasso");
    };
    assert_eq!(acceptance, "pulse-b-observed");
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn model_stage_precedes_product_stage_when_both_are_incomplete() {
    let model = model("both-cutoffs", |state| match *state {
        0 => vec![Transition::new("pulse-a", 1)],
        1 => vec![Transition::new("pulse-b", 2)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();

    let result = check_buchi_with_limits(
        &model,
        &automaton,
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
fn fully_unbounded_staged_buchi_matches_legacy_result() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let legacy = check_buchi(&model, &automaton).unwrap();
    let staged = check_buchi_with_limits(&model, &automaton, AnalysisLimits::unbounded()).unwrap();

    assert_eq!(staged.outcome, AnalysisOutcome::Conclusive(legacy.status));
    assert_eq!(staged.finite_policy, legacy.finite_policy);
    assert_eq!(staged.acceptance_sets, legacy.acceptance_sets);
    assert_eq!(staged.model_states, legacy.model_states);
    assert_eq!(staged.explored_model_transitions, legacy.model_transitions);
    assert_eq!(staged.product_states, legacy.product_states);
    assert_eq!(
        staged.explored_product_transitions,
        legacy.product_transitions
    );
    assert_eq!(staged.counterexample, legacy.counterexample);
}

use formal_verification_lab::buchi::{
    check_buchi, check_buchi_with_product_limits, BuchiCounterexample, BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{
    alternating_pulses, finite_quiet_run, pulse_automaton, unfair_second_pulse, PulseModelState,
};
use formal_verification_lab::{
    BoundedOutcome, ExplorationLimits, InconclusiveReason, Invariant, StateVariable, Transition,
    TransitionSystem,
};

fn limits(
    max_states: Option<usize>,
    max_transitions: Option<usize>,
    max_depth: Option<usize>,
) -> ExplorationLimits {
    ExplorationLimits {
        max_states,
        max_transitions,
        max_depth,
    }
}

fn model_with_edges(
    name: &'static str,
    successors: impl Fn(&u8) -> Vec<Transition<u8>> + Send + Sync + 'static,
) -> TransitionSystem<u8> {
    TransitionSystem::new(
        name,
        vec![StateVariable::new("node", "bounded Buchi test node")],
        vec![0_u8],
        move |state| Ok(successors(state)),
        vec![Invariant::new("well-formed", |state: &u8| *state < 4)],
    )
    .unwrap()
}

#[test]
fn partial_nonterminal_under_strict_policy_is_not_fabricated_as_terminal() {
    let model = model_with_edges("strict-nonterminal-prefix", |state| match state {
        0 => vec![Transition::new("quiet", 1)],
        1 => vec![Transition::new("quiet", 2)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();

    let result =
        check_buchi_with_product_limits(&model, &automaton, limits(None, Some(0), None)).unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
    );
    assert_eq!(result.model_states, 3);
    assert_eq!(result.model_transitions, 2);
    assert_eq!(result.product_states, 1);
    assert_eq!(result.checked_product_states, 1);
    assert_eq!(result.explored_product_transitions, 0);
    assert_eq!(result.retained_product_transitions, 0);
    assert!(result.counterexample.is_none());
}

#[test]
fn real_strict_terminal_retained_before_later_cutoff_is_conclusive() {
    let model = model_with_edges("strict-terminal-before-cutoff", |state| match state {
        0 => vec![Transition::new("quiet", 1), Transition::new("pulse-a", 2)],
        2 => vec![Transition::new("pulse-b", 2)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();

    let result =
        check_buchi_with_product_limits(&model, &automaton, limits(None, Some(1), None)).unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert_eq!(result.explored_product_transitions, 1);
    assert_eq!(result.retained_product_transitions, 1);
    let BuchiCounterexample::FiniteTerminal {
        missing_acceptance,
        trace,
    } = result.counterexample.unwrap()
    else {
        panic!("expected strict finite-terminal witness");
    };
    assert_eq!(missing_acceptance, "pulse-a-observed");
    assert_eq!(trace.len(), 2);
    assert_eq!(trace.last().unwrap().state.state, 1);
    assert_eq!(trace.last().unwrap().action.as_deref(), Some("quiet"));
}

#[test]
fn real_acceptance_avoiding_cycle_retained_before_cutoff_is_conclusive() {
    let model = model_with_edges("cycle-before-cutoff", |state| match state {
        0 => vec![Transition::new("pulse-a", 0), Transition::new("escape", 1)],
        _ => Vec::new(),
    });
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();

    let result =
        check_buchi_with_product_limits(&model, &automaton, limits(None, Some(3), None)).unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BuchiStatus::Violated)
    );
    assert_eq!(result.explored_product_transitions, 3);
    assert_eq!(result.retained_product_transitions, 3);
    let BuchiCounterexample::AcceptanceAvoidingCycle {
        acceptance,
        stem,
        cycle,
    } = result.counterexample.unwrap()
    else {
        panic!("expected acceptance-avoiding lasso");
    };
    assert_eq!(acceptance, "pulse-b-observed");
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle
        .iter()
        .any(|step| step.action.as_deref() == Some("pulse-a")));
}

#[test]
fn zero_product_state_budget_reports_exact_cutoff_accounting() {
    let result = check_buchi_with_product_limits(
        &alternating_pulses().unwrap(),
        &pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap(),
        limits(Some(0), None, None),
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 0 })
    );
    assert_eq!(result.product_states, 0);
    assert_eq!(result.checked_product_states, 0);
    assert_eq!(result.explored_product_transitions, 0);
    assert_eq!(result.retained_product_transitions, 0);
    assert_eq!(result.max_product_depth_reached, None);
    assert!(result.counterexample.is_none());
}

fn assert_generous_limits_match_unbounded(
    model: TransitionSystem<PulseModelState>,
    policy: FiniteRunPolicy,
) {
    let automaton = pulse_automaton(policy).unwrap();
    let unbounded = check_buchi(&model, &automaton).unwrap();
    let bounded =
        check_buchi_with_product_limits(&model, &automaton, limits(Some(64), Some(128), Some(64)))
            .unwrap();

    assert_eq!(
        bounded.outcome,
        BoundedOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(bounded.finite_policy, unbounded.finite_policy);
    assert_eq!(bounded.acceptance_sets, unbounded.acceptance_sets);
    assert_eq!(bounded.model_states, unbounded.model_states);
    assert_eq!(bounded.model_transitions, unbounded.model_transitions);
    assert_eq!(bounded.product_states, unbounded.product_states);
    assert_eq!(bounded.checked_product_states, unbounded.product_states);
    assert_eq!(
        bounded.explored_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(
        bounded.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(bounded.counterexample, unbounded.counterexample);
}

#[test]
fn generous_limits_match_unbounded_for_both_policies_and_example_shapes() {
    assert_generous_limits_match_unbounded(
        alternating_pulses().unwrap(),
        FiniteRunPolicy::IgnoreTerminals,
    );
    assert_generous_limits_match_unbounded(
        alternating_pulses().unwrap(),
        FiniteRunPolicy::RequireAcceptingTerminal,
    );
    assert_generous_limits_match_unbounded(
        unfair_second_pulse().unwrap(),
        FiniteRunPolicy::IgnoreTerminals,
    );
    assert_generous_limits_match_unbounded(
        unfair_second_pulse().unwrap(),
        FiniteRunPolicy::RequireAcceptingTerminal,
    );
    assert_generous_limits_match_unbounded(
        finite_quiet_run().unwrap(),
        FiniteRunPolicy::IgnoreTerminals,
    );
    assert_generous_limits_match_unbounded(
        finite_quiet_run().unwrap(),
        FiniteRunPolicy::RequireAcceptingTerminal,
    );
}

#[test]
fn repeated_bounded_buchi_runs_are_deterministic() {
    let model = unfair_second_pulse().unwrap();
    let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
    let product_limits = limits(None, Some(2), None);

    let first = check_buchi_with_product_limits(&model, &automaton, product_limits).unwrap();
    let second = check_buchi_with_product_limits(&model, &automaton, product_limits).unwrap();

    assert_eq!(first, second);
}

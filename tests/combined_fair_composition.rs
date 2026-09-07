use formal_verification_lab::{
    check_action_temporal, check_action_temporal_with_fairness_profile,
    check_action_temporal_with_fairness_profile_and_limits,
    check_action_temporal_with_fairness_profile_and_product_limits,
    check_action_temporal_with_strong_fairness, check_action_temporal_with_weak_fairness,
    check_buchi_with_fairness_profile, check_buchi_with_fairness_profile_and_limits,
    check_buchi_with_fairness_profile_and_product_limits, check_response,
    check_response_with_fairness_profile, check_response_with_fairness_profile_and_limits,
    check_response_with_fairness_profile_and_product_limits, check_response_with_strong_fairness,
    check_response_with_weak_fairness, parse_action_temporal, parse_declarative_model,
    AcceptanceSet, ActionAtom, ActionTemporalSpec, AnalysisInconclusiveReason, AnalysisLimits,
    AnalysisOutcome, AnalysisStage, BoundedOutcome, BuchiAutomaton, BuchiCounterexample,
    BuchiProductState, BuchiStatus, ExplorationLimits, FairnessProfile, FiniteRunPolicy,
    InconclusiveReason, Invariant, ResponseCounterexample, ResponseProperty, ResponseStatus,
    StateVariable, StrongFairness, TemporalBackend, TemporalCounterexample, TemporalObligation,
    TemporalStatus, TraceStep, Transition, TransitionSystem, WeakFairness,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Phase {
    Idle,
    Offer,
    Away,
}

fn intermittent_grant_model() -> TransitionSystem<Phase> {
    TransitionSystem::new(
        "m47-intermittent-grant",
        vec![StateVariable::new("phase", "protocol phase")],
        vec![Phase::Idle],
        |state| match state {
            Phase::Idle => Ok(vec![Transition::new("request", Phase::Offer)]),
            Phase::Offer => Ok(vec![
                Transition::new("grant", Phase::Idle),
                Transition::new("defer", Phase::Away),
            ]),
            Phase::Away => Ok(vec![Transition::new("return", Phase::Offer)]),
        },
        vec![Invariant::new("recognized-phase", |_state| true)],
    )
    .unwrap()
}

fn finite_pending_model() -> TransitionSystem<usize> {
    TransitionSystem::new(
        "m47-finite-pending",
        vec![StateVariable::new("node", "protocol point")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("request", 1)]),
            1 => Ok(Vec::new()),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < 2)],
    )
    .unwrap()
}

fn response_property() -> ResponseProperty {
    ResponseProperty::new(
        "request-eventually-grant",
        |action| action == "request",
        |action| action == "grant",
    )
    .unwrap()
}

fn response_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::response(
        "request-eventually-grant",
        ActionAtom::exact("request").unwrap(),
        ActionAtom::exact("grant").unwrap(),
    )
    .unwrap()
}

fn recurring_grant_spec() -> ActionTemporalSpec {
    ActionTemporalSpec::all_infinitely_often(
        "grant-infinitely-often",
        vec![ActionAtom::exact("grant").unwrap()],
    )
    .unwrap()
}

fn response_automaton() -> BuchiAutomaton<bool> {
    BuchiAutomaton::new(
        "request-eventually-grant-response-obligation",
        false,
        |pending, action| {
            if action == "grant" {
                false
            } else if action == "request" {
                true
            } else {
                *pending
            }
        },
        vec![AcceptanceSet::new("response-discharged", |pending: &bool| !*pending).unwrap()],
        FiniteRunPolicy::RequireAcceptingTerminal,
    )
    .unwrap()
}

fn recurring_grant_automaton() -> BuchiAutomaton<bool> {
    BuchiAutomaton::new(
        "grant-infinitely-often",
        false,
        |_seen, action| action == "grant",
        vec![AcceptanceSet::new("grant", |seen: &bool| *seen).unwrap()],
        FiniteRunPolicy::IgnoreTerminals,
    )
    .unwrap()
}

fn mixed_satisfying_profile() -> FairnessProfile {
    FairnessProfile::new(["return"], ["grant"]).unwrap()
}

fn mixed_violating_profile() -> FairnessProfile {
    FairnessProfile::new(["return"], ["defer"]).unwrap()
}

fn state_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: Some(limit),
        max_transitions: None,
        max_depth: None,
    }
}

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

fn temp_model_path() -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fvlab-m47-combined-fair-{}-{id}.fvl",
        std::process::id()
    ))
}

fn buchi_status_as_response(status: BuchiStatus) -> ResponseStatus {
    match status {
        BuchiStatus::Satisfied => ResponseStatus::Satisfied,
        BuchiStatus::Violated => ResponseStatus::Violated,
    }
}

fn buchi_status_as_temporal(status: BuchiStatus) -> TemporalStatus {
    match status {
        BuchiStatus::Satisfied => TemporalStatus::Satisfied,
        BuchiStatus::Violated => TemporalStatus::Violated,
    }
}

fn strip_buchi_trace<S: Clone, A>(
    trace: &[TraceStep<BuchiProductState<S, A>>],
) -> Vec<TraceStep<S>> {
    trace
        .iter()
        .map(|step| TraceStep {
            action: step.action.clone(),
            state: step.state.state.clone(),
        })
        .collect()
}

#[test]
fn response_profile_preserves_none_weak_strong_and_overlap_paths_exactly() {
    let model = intermittent_grant_model();
    let property = response_property();

    assert_eq!(
        check_response_with_fairness_profile(&model, &property, &FairnessProfile::none()).unwrap(),
        check_response(&model, &property).unwrap()
    );

    let weak = WeakFairness::new(["grant"]).unwrap();
    let weak_profile = FairnessProfile::new(["grant"], Vec::<&str>::new()).unwrap();
    assert_eq!(
        check_response_with_fairness_profile(&model, &property, &weak_profile).unwrap(),
        check_response_with_weak_fairness(&model, &property, &weak).unwrap()
    );

    let strong = StrongFairness::new(["grant"]).unwrap();
    let strong_profile = FairnessProfile::new(Vec::<&str>::new(), ["grant"]).unwrap();
    assert_eq!(
        check_response_with_fairness_profile(&model, &property, &strong_profile).unwrap(),
        check_response_with_strong_fairness(&model, &property, &strong).unwrap()
    );

    let overlap = FairnessProfile::new(["grant"], ["grant"]).unwrap();
    assert!(overlap.weak_actions().is_empty());
    assert_eq!(overlap.strong_actions(), &["grant"]);
    assert_eq!(
        check_response_with_fairness_profile(&model, &property, &overlap).unwrap(),
        check_response_with_strong_fairness(&model, &property, &strong).unwrap()
    );
}

#[test]
fn mixed_response_matches_direct_combined_buchi_and_normalizes_the_lasso() {
    let model = intermittent_grant_model();
    let property = response_property();
    let profile = mixed_violating_profile();

    let wrapped = check_response_with_fairness_profile(&model, &property, &profile).unwrap();
    let direct =
        check_buchi_with_fairness_profile(&model, &response_automaton(), &profile).unwrap();

    assert_eq!(wrapped.status, buchi_status_as_response(direct.status));
    assert_eq!(wrapped.model_states, direct.model_states);
    assert_eq!(wrapped.model_transitions, direct.model_transitions);
    assert_eq!(wrapped.product_states, direct.product_states);
    assert_eq!(wrapped.product_transitions, direct.product_transitions);

    let Some(ResponseCounterexample::Infinite { stem, cycle }) = wrapped.counterexample else {
        panic!("expected mixed-fair pending-cycle violation");
    };
    let Some(BuchiCounterexample::AcceptanceAvoidingCycle {
        stem: direct_stem,
        cycle: direct_cycle,
        ..
    }) = direct.counterexample
    else {
        panic!("expected direct combined-fair lasso");
    };
    assert_eq!(
        stem.iter()
            .map(|step| TraceStep {
                action: step.action.clone(),
                state: step.state.state,
            })
            .collect::<Vec<_>>(),
        strip_buchi_trace(&direct_stem)
    );
    assert_eq!(
        cycle
            .iter()
            .map(|step| TraceStep {
                action: step.action.clone(),
                state: step.state.state,
            })
            .collect::<Vec<_>>(),
        strip_buchi_trace(&direct_cycle)
    );
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    let actions = cycle
        .iter()
        .filter_map(|step| step.action.as_deref())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"defer"));
    assert!(actions.contains(&"return"));
}

#[test]
fn finite_pending_terminal_remains_a_violation_under_mixed_fairness() {
    let result = check_response_with_fairness_profile(
        &finite_pending_model(),
        &response_property(),
        &mixed_satisfying_profile(),
    )
    .unwrap();

    assert_eq!(result.status, ResponseStatus::Violated);
    let Some(ResponseCounterexample::Finite { trace }) = result.counterexample else {
        panic!("expected strict pending terminal");
    };
    assert_eq!(trace.len(), 2);
    assert_eq!(trace.last().unwrap().state.state, 1);
    assert!(trace.last().unwrap().state.pending);
}

#[test]
fn product_cutoff_before_pending_terminal_is_inconclusive_but_after_witness_is_conclusive() {
    let model = finite_pending_model();
    let property = response_property();
    let profile = mixed_satisfying_profile();

    let before = check_response_with_fairness_profile_and_product_limits(
        &model,
        &property,
        &profile,
        state_limit(1),
    )
    .unwrap();
    assert_eq!(
        before.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 1 })
    );
    assert!(before.counterexample.is_none());

    let after = check_response_with_fairness_profile_and_product_limits(
        &model,
        &property,
        &profile,
        state_limit(2),
    )
    .unwrap();
    assert_eq!(
        after.outcome,
        BoundedOutcome::Conclusive(ResponseStatus::Violated)
    );
    assert!(matches!(
        after.counterexample,
        Some(ResponseCounterexample::Finite { .. })
    ));
}

#[test]
fn staged_response_keeps_model_cutoff_provenance_and_generous_limits_equal_unbounded() {
    let model = finite_pending_model();
    let property = response_property();
    let profile = mixed_satisfying_profile();

    let cutoff = check_response_with_fairness_profile_and_limits(
        &model,
        &property,
        &profile,
        AnalysisLimits::new(state_limit(1), ExplorationLimits::unbounded()),
    )
    .unwrap();
    assert_eq!(
        cutoff.outcome,
        AnalysisOutcome::Inconclusive(AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::StateLimitReached { limit: 1 },
        })
    );
    assert!(cutoff.counterexample.is_none());

    let unbounded = check_response_with_fairness_profile(&model, &property, &profile).unwrap();
    let staged = check_response_with_fairness_profile_and_limits(
        &model,
        &property,
        &profile,
        AnalysisLimits::unbounded(),
    )
    .unwrap();
    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(staged.model_states, unbounded.model_states);
    assert_eq!(staged.product_states, unbounded.product_states);
    assert_eq!(
        staged.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(staged.counterexample, unbounded.counterexample);
}

#[test]
fn temporal_profile_preserves_existing_backends_for_compatibility_profiles() {
    let model = intermittent_grant_model();
    let weak = WeakFairness::new(["grant"]).unwrap();
    let strong = StrongFairness::new(["grant"]).unwrap();
    let weak_profile = FairnessProfile::new(["grant"], Vec::<&str>::new()).unwrap();
    let strong_profile = FairnessProfile::new(Vec::<&str>::new(), ["grant"]).unwrap();

    for spec in [response_spec(), recurring_grant_spec()] {
        assert_eq!(
            check_action_temporal_with_fairness_profile(&model, &spec, &FairnessProfile::none())
                .unwrap(),
            check_action_temporal(&model, &spec).unwrap()
        );
        assert_eq!(
            check_action_temporal_with_fairness_profile(&model, &spec, &weak_profile).unwrap(),
            check_action_temporal_with_weak_fairness(&model, &spec, &weak).unwrap()
        );
        assert_eq!(
            check_action_temporal_with_fairness_profile(&model, &spec, &strong_profile).unwrap(),
            check_action_temporal_with_strong_fairness(&model, &spec, &strong).unwrap()
        );
    }
}

#[test]
fn mixed_temporal_response_and_recurring_specs_match_direct_combined_buchi_results() {
    let model = intermittent_grant_model();
    let profile = mixed_satisfying_profile();

    let response =
        check_action_temporal_with_fairness_profile(&model, &response_spec(), &profile).unwrap();
    let direct_response =
        check_buchi_with_fairness_profile(&model, &response_automaton(), &profile).unwrap();
    assert_eq!(response.backend, TemporalBackend::Response);
    assert_eq!(
        response.status,
        buchi_status_as_temporal(direct_response.status)
    );
    assert_eq!(response.model_states, direct_response.model_states);
    assert_eq!(response.product_states, direct_response.product_states);
    assert_eq!(
        response.counterexample.is_some(),
        direct_response.counterexample.is_some()
    );

    let recurring =
        check_action_temporal_with_fairness_profile(&model, &recurring_grant_spec(), &profile)
            .unwrap();
    let direct_recurring =
        check_buchi_with_fairness_profile(&model, &recurring_grant_automaton(), &profile).unwrap();
    assert_eq!(recurring.backend, TemporalBackend::Buchi);
    assert_eq!(
        recurring.status,
        buchi_status_as_temporal(direct_recurring.status)
    );
    assert_eq!(recurring.model_states, direct_recurring.model_states);
    assert_eq!(recurring.product_states, direct_recurring.product_states);
    assert_eq!(
        recurring.counterexample.is_some(),
        direct_recurring.counterexample.is_some()
    );
}

#[test]
fn mixed_temporal_violations_keep_frontend_obligation_identity_and_real_closed_cycles() {
    let model = intermittent_grant_model();
    let profile = mixed_violating_profile();

    let response =
        check_action_temporal_with_fairness_profile(&model, &response_spec(), &profile).unwrap();
    assert_eq!(response.backend, TemporalBackend::Response);
    assert_eq!(response.status, TemporalStatus::Violated);
    let Some(TemporalCounterexample::Infinite {
        obligation, cycle, ..
    }) = response.counterexample
    else {
        panic!("expected response lasso");
    };
    assert_eq!(obligation, TemporalObligation::Response);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);

    let recurring =
        check_action_temporal_with_fairness_profile(&model, &recurring_grant_spec(), &profile)
            .unwrap();
    assert_eq!(recurring.backend, TemporalBackend::Buchi);
    assert_eq!(recurring.status, TemporalStatus::Violated);
    let Some(TemporalCounterexample::Infinite {
        obligation, cycle, ..
    }) = recurring.counterexample
    else {
        panic!("expected recurring-action lasso");
    };
    assert_eq!(
        obligation,
        TemporalObligation::InfinitelyOftenAction("grant".to_owned())
    );
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
}

#[test]
fn mixed_temporal_product_and_staged_cutoffs_match_direct_m46_outcomes() {
    let model = intermittent_grant_model();
    let profile = mixed_satisfying_profile();
    let spec = recurring_grant_spec();
    let automaton = recurring_grant_automaton();
    let product_limits = transition_limit(2);

    let wrapped_product = check_action_temporal_with_fairness_profile_and_product_limits(
        &model,
        &spec,
        &profile,
        product_limits,
    )
    .unwrap();
    let direct_product = check_buchi_with_fairness_profile_and_product_limits(
        &model,
        &automaton,
        &profile,
        product_limits,
    )
    .unwrap();
    assert_eq!(
        wrapped_product.outcome.inconclusive_reason(),
        direct_product.outcome.inconclusive_reason()
    );
    assert_eq!(
        wrapped_product.product_states,
        direct_product.product_states
    );
    assert_eq!(
        wrapped_product.explored_product_transitions,
        direct_product.explored_product_transitions
    );

    let limits = AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded());
    let wrapped_staged =
        check_action_temporal_with_fairness_profile_and_limits(&model, &spec, &profile, limits)
            .unwrap();
    let direct_staged =
        check_buchi_with_fairness_profile_and_limits(&model, &automaton, &profile, limits).unwrap();
    assert_eq!(
        wrapped_staged.outcome.inconclusive_reason(),
        direct_staged.outcome.inconclusive_reason()
    );
    assert_eq!(
        wrapped_staged.model_completion,
        direct_staged.model_completion
    );
    assert_eq!(
        wrapped_staged.product_completion,
        direct_staged.product_completion
    );
}

#[test]
fn textual_declarative_model_file_composes_with_mixed_profile_without_new_cli_or_grammar() {
    let path = temp_model_path();
    fs::write(
        &path,
        concat!(
            "model \"m47-file\"\n",
            "state \"idle\"\n",
            "state \"offer\"\n",
            "state \"away\"\n",
            "initial \"idle\"\n",
            "edge \"idle\" \"request\" \"offer\"\n",
            "edge \"offer\" \"grant\" \"idle\"\n",
            "edge \"offer\" \"defer\" \"away\"\n",
            "edge \"away\" \"return\" \"offer\"\n",
        ),
    )
    .unwrap();

    let source = fs::read_to_string(&path).unwrap();
    let model = parse_declarative_model(&source).unwrap();
    let profile = mixed_satisfying_profile();

    let response =
        parse_action_temporal("request-eventually-grant", r#"response("request","grant")"#)
            .unwrap();
    let response_result =
        check_action_temporal_with_fairness_profile(&model, &response, &profile).unwrap();
    assert_eq!(response_result.backend, TemporalBackend::Response);
    assert_eq!(response_result.status, TemporalStatus::Satisfied);

    let recurring =
        parse_action_temporal("grant-infinitely-often", r#"infinitely-often("grant")"#).unwrap();
    let recurring_result =
        check_action_temporal_with_fairness_profile(&model, &recurring, &profile).unwrap();
    assert_eq!(recurring_result.backend, TemporalBackend::Buchi);
    assert_eq!(recurring_result.status, TemporalStatus::Satisfied);

    fs::remove_file(path).unwrap();
}

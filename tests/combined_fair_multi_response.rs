use formal_verification_lab::multi_response::{
    check_multi_response, check_multi_response_with_fairness_profile,
    check_multi_response_with_fairness_profile_and_limits,
    check_multi_response_with_fairness_profile_and_product_limits,
    check_multi_response_with_limits, check_multi_response_with_product_limits,
    check_multi_response_with_strong_fairness,
    check_multi_response_with_strong_fairness_and_limits,
    check_multi_response_with_strong_fairness_and_product_limits,
    check_multi_response_with_weak_fairness, check_multi_response_with_weak_fairness_and_limits,
    check_multi_response_with_weak_fairness_and_product_limits, MultiResponseCounterexample,
    MultiResponseProperty, MultiResponseStatus, ResponseClause,
};
use formal_verification_lab::{
    AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome, ExplorationLimits,
    FairnessProfile, InconclusiveReason, Invariant, StateVariable, Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MixedState {
    Start,
    WeakPending,
    StrongA,
    StrongB,
}

fn dual_property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "dual-response",
        vec![
            ResponseClause::new(
                "class-a",
                |action| action == "request-a",
                |action| action == "grant-a",
            )
            .unwrap(),
            ResponseClause::new(
                "class-b",
                |action| action == "request-b",
                |action| action == "grant-b",
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn mixed_filter_model() -> TransitionSystem<MixedState> {
    TransitionSystem::new(
        "mixed-filter",
        vec![StateVariable::new("phase", "combined-fair response phase")],
        vec![MixedState::Start],
        |state| {
            Ok(match state {
                MixedState::Start => vec![
                    Transition::new("request-a", MixedState::WeakPending),
                    Transition::new("request-b", MixedState::StrongA),
                ],
                MixedState::WeakPending => vec![
                    Transition::new("wait-a", MixedState::WeakPending),
                    Transition::new("grant-a", MixedState::Start),
                ],
                MixedState::StrongA => vec![Transition::new("tick-b", MixedState::StrongB)],
                MixedState::StrongB => vec![
                    Transition::new("tick-b", MixedState::StrongA),
                    Transition::new("grant-b", MixedState::Start),
                ],
            })
        },
        vec![Invariant::new("known-phase", |_state: &MixedState| true)],
    )
    .unwrap()
}

fn mixed_profile() -> FairnessProfile {
    FairnessProfile::new(["grant-a"], ["grant-b"]).unwrap()
}

fn weak_only_profile() -> FairnessProfile {
    FairnessProfile::new(["grant-a"], std::iter::empty::<&str>()).unwrap()
}

fn strong_only_profile() -> FairnessProfile {
    FairnessProfile::new(std::iter::empty::<&str>(), ["grant-b"]).unwrap()
}

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

fn generous_limits() -> AnalysisLimits {
    AnalysisLimits {
        model: ExplorationLimits {
            max_states: Some(64),
            max_transitions: Some(128),
            max_depth: Some(32),
        },
        product: ExplorationLimits {
            max_states: Some(256),
            max_transitions: Some(512),
            max_depth: Some(64),
        },
    }
}

#[test]
fn compatibility_profiles_delegate_exactly_unbounded() {
    let model = mixed_filter_model();
    let property = dual_property();

    let none = FairnessProfile::none();
    assert_eq!(
        check_multi_response_with_fairness_profile(&model, &property, &none).unwrap(),
        check_multi_response(&model, &property).unwrap()
    );

    let weak = weak_only_profile();
    assert_eq!(
        check_multi_response_with_fairness_profile(&model, &property, &weak).unwrap(),
        check_multi_response_with_weak_fairness(&model, &property, weak.weak()).unwrap()
    );

    let strong = strong_only_profile();
    assert_eq!(
        check_multi_response_with_fairness_profile(&model, &property, &strong).unwrap(),
        check_multi_response_with_strong_fairness(&model, &property, strong.strong()).unwrap()
    );
}

#[test]
fn compatibility_profiles_delegate_exactly_product_bounded() {
    let model = mixed_filter_model();
    let property = dual_property();
    let limits = transition_limit(3);

    let none = FairnessProfile::none();
    assert_eq!(
        check_multi_response_with_fairness_profile_and_product_limits(
            &model, &property, &none, limits,
        )
        .unwrap(),
        check_multi_response_with_product_limits(&model, &property, limits).unwrap()
    );

    let weak = weak_only_profile();
    assert_eq!(
        check_multi_response_with_fairness_profile_and_product_limits(
            &model, &property, &weak, limits,
        )
        .unwrap(),
        check_multi_response_with_weak_fairness_and_product_limits(
            &model,
            &property,
            weak.weak(),
            limits,
        )
        .unwrap()
    );

    let strong = strong_only_profile();
    assert_eq!(
        check_multi_response_with_fairness_profile_and_product_limits(
            &model, &property, &strong, limits,
        )
        .unwrap(),
        check_multi_response_with_strong_fairness_and_product_limits(
            &model,
            &property,
            strong.strong(),
            limits,
        )
        .unwrap()
    );
}

#[test]
fn compatibility_profiles_delegate_exactly_staged() {
    let model = mixed_filter_model();
    let property = dual_property();
    let limits = AnalysisLimits {
        model: transition_limit(3),
        product: transition_limit(2),
    };

    let none = FairnessProfile::none();
    assert_eq!(
        check_multi_response_with_fairness_profile_and_limits(&model, &property, &none, limits)
            .unwrap(),
        check_multi_response_with_limits(&model, &property, limits).unwrap()
    );

    let weak = weak_only_profile();
    assert_eq!(
        check_multi_response_with_fairness_profile_and_limits(&model, &property, &weak, limits)
            .unwrap(),
        check_multi_response_with_weak_fairness_and_limits(&model, &property, weak.weak(), limits,)
            .unwrap()
    );

    let strong = strong_only_profile();
    assert_eq!(
        check_multi_response_with_fairness_profile_and_limits(&model, &property, &strong, limits)
            .unwrap(),
        check_multi_response_with_strong_fairness_and_limits(
            &model,
            &property,
            strong.strong(),
            limits,
        )
        .unwrap()
    );
}

#[test]
fn genuinely_mixed_profile_filters_both_independent_unfair_lassos() {
    let model = mixed_filter_model();
    let property = dual_property();

    let weak = check_multi_response_with_fairness_profile(&model, &property, &weak_only_profile())
        .unwrap();
    let strong =
        check_multi_response_with_fairness_profile(&model, &property, &strong_only_profile())
            .unwrap();
    let mixed =
        check_multi_response_with_fairness_profile(&model, &property, &mixed_profile()).unwrap();

    assert_eq!(weak.status, MultiResponseStatus::Violated);
    assert_eq!(strong.status, MultiResponseStatus::Violated);
    assert_eq!(mixed.status, MultiResponseStatus::Satisfied);
    assert!(mixed.counterexample.is_none());
}

#[test]
fn mixed_fairness_never_excuses_a_real_pending_terminal() {
    let model = TransitionSystem::new(
        "finite-pending",
        vec![StateVariable::new("node", "finite response state")],
        vec![0usize],
        |state| {
            Ok(match state {
                0 => vec![Transition::new("request-a", 1)],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("node-domain", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let result =
        check_multi_response_with_fairness_profile(&model, &dual_property(), &mixed_profile())
            .unwrap();

    assert_eq!(result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Finite { clause, trace }) = result.counterexample else {
        panic!("expected a finite pending-terminal counterexample");
    };
    assert_eq!(clause, "class-a");
    assert_eq!(trace.last().unwrap().state.pending, vec![true, false]);
}

#[test]
fn retained_mixed_fair_lasso_preserves_clause_and_pending_vector() {
    let model = TransitionSystem::new(
        "retained-fair-lasso",
        vec![StateVariable::new("node", "fair lasso state")],
        vec![0usize],
        |state| {
            Ok(match state {
                0 => vec![Transition::new("request-a", 1)],
                1 => vec![Transition::new("grant-b", 2)],
                _ => vec![Transition::new("grant-a-shadow", 1)],
            })
        },
        vec![Invariant::new("node-domain", |state: &usize| *state <= 2)],
    )
    .unwrap();
    let profile = FairnessProfile::new(["grant-a-shadow"], ["grant-b"]).unwrap();
    let result =
        check_multi_response_with_fairness_profile(&model, &dual_property(), &profile).unwrap();

    assert_eq!(result.status, MultiResponseStatus::Violated);
    let Some(MultiResponseCounterexample::Infinite {
        clause,
        stem: _,
        cycle,
    }) = result.counterexample
    else {
        panic!("expected a retained mixed-fair lasso");
    };
    assert_eq!(clause, "class-a");
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(cycle.iter().all(|step| step.state.pending[0]));
    let actions = cycle
        .iter()
        .filter_map(|step| step.action.as_deref())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"grant-b"));
    assert!(actions.contains(&"grant-a-shadow"));
}

#[test]
fn product_cutoff_before_witness_is_inconclusive_but_retained_terminal_wins() {
    let property = dual_property();
    let mixed = mixed_filter_model();
    let before = check_multi_response_with_fairness_profile_and_product_limits(
        &mixed,
        &property,
        &mixed_profile(),
        transition_limit(0),
    )
    .unwrap();
    assert_eq!(
        before.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
    );
    assert!(before.counterexample.is_none());

    let retained = TransitionSystem::new(
        "retained-terminal-before-cutoff",
        vec![StateVariable::new("node", "cutoff witness state")],
        vec![0usize],
        |state| {
            Ok(match state {
                0 => vec![Transition::new("request-a", 1), Transition::new("other", 2)],
                1 => Vec::new(),
                _ => vec![Transition::new("other", 2)],
            })
        },
        vec![Invariant::new("node-domain", |state: &usize| *state <= 2)],
    )
    .unwrap();
    let after = check_multi_response_with_fairness_profile_and_product_limits(
        &retained,
        &property,
        &mixed_profile(),
        transition_limit(1),
    )
    .unwrap();
    assert_eq!(
        after.outcome,
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated)
    );
    let Some(MultiResponseCounterexample::Finite { clause, .. }) = after.counterexample else {
        panic!("retained real terminal must remain conclusive");
    };
    assert_eq!(clause, "class-a");
}

#[test]
fn staged_cutoff_preserves_model_provenance_and_generous_limits_match_unbounded() {
    let model = mixed_filter_model();
    let property = dual_property();
    let profile = mixed_profile();
    let cutoff = check_multi_response_with_fairness_profile_and_limits(
        &model,
        &property,
        &profile,
        AnalysisLimits {
            model: transition_limit(0),
            product: ExplorationLimits::unbounded(),
        },
    )
    .unwrap();
    assert_eq!(
        cutoff.outcome,
        AnalysisOutcome::Inconclusive(formal_verification_lab::AnalysisInconclusiveReason {
            stage: AnalysisStage::Model,
            reason: InconclusiveReason::TransitionLimitReached { limit: 0 },
        })
    );

    let unbounded =
        check_multi_response_with_fairness_profile(&model, &property, &profile).unwrap();
    let staged = check_multi_response_with_fairness_profile_and_limits(
        &model,
        &property,
        &profile,
        generous_limits(),
    )
    .unwrap();
    assert_eq!(
        staged.outcome,
        AnalysisOutcome::Conclusive(unbounded.status)
    );
    assert_eq!(staged.model_states, unbounded.model_states);
    assert_eq!(
        staged.explored_model_transitions,
        unbounded.model_transitions
    );
    assert_eq!(staged.product_states, unbounded.product_states);
    assert_eq!(
        staged.retained_product_transitions,
        unbounded.product_transitions
    );
    assert_eq!(staged.counterexample, unbounded.counterexample);
}

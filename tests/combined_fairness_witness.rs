use formal_verification_lab::{
    check_buchi_with_fairness_profile, AcceptanceSet, BuchiAutomaton, BuchiCounterexample,
    BuchiStatus, FairnessProfile, FiniteRunPolicy, Invariant, StateVariable, Transition,
    TransitionSystem,
};

#[test]
fn combined_fair_lasso_uses_shortest_stem_to_exact_cycle_entry() {
    let model = TransitionSystem::new(
        "combined-fair-lasso-alignment",
        vec![StateVariable::new("node", "protocol point")],
        vec![0usize],
        |state| match state {
            0 => Ok(vec![Transition::new("enter", 1)]),
            1 => Ok(vec![Transition::new("w", 1), Transition::new("s", 1)]),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < 2)],
    )
    .unwrap();
    let automaton = BuchiAutomaton::new(
        "reject-every-infinite-run",
        (),
        |_state, _action| (),
        vec![AcceptanceSet::new("never", |_state| false).unwrap()],
        FiniteRunPolicy::IgnoreTerminals,
    )
    .unwrap();
    let profile = FairnessProfile::new(["w"], ["s"]).unwrap();

    let result = check_buchi_with_fairness_profile(&model, &automaton, &profile).unwrap();
    assert_eq!(result.status, BuchiStatus::Violated);

    let BuchiCounterexample::AcceptanceAvoidingCycle { stem, cycle, .. } =
        result.counterexample.expect("combined-fair lasso")
    else {
        panic!("expected acceptance-avoiding lasso");
    };

    assert_eq!(
        stem.len(),
        2,
        "state 1 is exactly one edge from the initial state"
    );
    assert_eq!(stem.last().unwrap().state, cycle.first().unwrap().state);
    assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    assert!(
        cycle.len() >= 3,
        "the mixed witness must execute both obligations"
    );

    let actions = cycle
        .iter()
        .filter_map(|step| step.action.as_deref())
        .collect::<Vec<_>>();
    assert!(actions.contains(&"w"));
    assert!(actions.contains(&"s"));
}

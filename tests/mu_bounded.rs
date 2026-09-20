use formal_verification_lab::{
    evaluate_mu, evaluate_mu_with_limits, BoundedMuError, BoundedMuStatus, BoundedMuTruth,
    BoundedOutcome, ExplorationLimits, InconclusiveReason, Invariant, MuFormula,
    MuValidationError, StateVariable, Transition, TransitionSystem,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Atom {
    P,
    Q,
}

#[test]
fn unbounded_limits_collapse_exactly_to_m75_semantics() {
    let model = three_state_model();
    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state != 2,
        Atom::Q => *state == 2,
    };

    for formula in representative_formulas() {
        let exact = evaluate_mu(&model, &formula, atom).unwrap();
        let bounded =
            evaluate_mu_with_limits(&model, &formula, atom, ExplorationLimits::unbounded())
                .unwrap();

        assert_eq!(
            bounded.outcome,
            BoundedOutcome::Conclusive(if exact.all_initial_states_satisfy() {
                BoundedMuStatus::Satisfied
            } else {
                BoundedMuStatus::Violated
            })
        );
        assert!(bounded.initial_states_complete);
        assert_eq!(bounded.reachable_states, exact.reachable_states);
        assert_eq!(
            bounded.definitely_satisfying_state_indices,
            exact.satisfying_state_indices
        );
        assert_eq!(
            bounded.possibly_satisfying_state_indices,
            exact.satisfying_state_indices
        );
        assert_eq!(bounded.discovered_states, exact.discovered_states);
        assert_eq!(bounded.explored_transitions, exact.explored_transitions);
        assert_eq!(bounded.max_depth_reached, exact.max_depth_reached);
        assert_eq!(bounded.fixpoint_iterations, exact.fixpoint_iterations);

        for (bounded_initial, exact_initial) in bounded.initial.iter().zip(&exact.initial) {
            assert_eq!(bounded_initial.state_index, exact_initial.state_index);
            assert_eq!(bounded_initial.state, exact_initial.state);
            assert_eq!(
                bounded_initial.truth,
                if exact_initial.satisfied {
                    BoundedMuTruth::True
                } else {
                    BoundedMuTruth::False
                }
            );
        }
    }
}

#[test]
fn validation_fails_before_bounded_graph_capture() {
    let model = TransitionSystem::new(
        "validation-before-capture",
        vec![StateVariable::new("state", "single state")],
        vec![0u8],
        |_state| -> Result<Vec<Transition<u8>>, _> {
            panic!("transition relation must not run for invalid mu formulas")
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let unbound = MuFormula::<Atom, u8>::diamond(MuFormula::var(7));
    assert!(matches!(
        evaluate_mu_with_limits(
            &model,
            &unbound,
            |_atom: &Atom, _state: &u8| true,
            ExplorationLimits::unbounded(),
        ),
        Err(BoundedMuError::Validation(
            MuValidationError::UnboundVariable { variable: 7 }
        ))
    ));

    let non_monotone = MuFormula::mu(1u8, MuFormula::<Atom, u8>::negate(MuFormula::var(1)));
    assert!(matches!(
        evaluate_mu_with_limits(
            &model,
            &non_monotone,
            |_atom: &Atom, _state: &u8| true,
            ExplorationLimits::unbounded(),
        ),
        Err(BoundedMuError::Validation(
            MuValidationError::NonMonotoneVariable { variable: 1 }
        ))
    ));
}

#[test]
fn depth_cutoff_keeps_modal_successor_unknown_including_under_negation() {
    let model = TransitionSystem::new(
        "mu-depth-cutoff",
        vec![StateVariable::new("state", "two states")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("next", 1)],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();
    let limits = ExplorationLimits {
        max_depth: Some(0),
        ..ExplorationLimits::unbounded()
    };
    let formulas = [
        MuFormula::<Atom, u8>::diamond(MuFormula::True),
        MuFormula::nu(0u8, MuFormula::<Atom, u8>::diamond(MuFormula::var(0))),
        MuFormula::negate(MuFormula::diamond(MuFormula::atom(Atom::P))),
    ];

    for formula in formulas {
        let result = evaluate_mu_with_limits(
            &model,
            &formula,
            |_atom: &Atom, state: &u8| *state == 1,
            limits,
        )
        .unwrap();

        assert_eq!(
            result.outcome,
            BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 0 })
        );
        assert_eq!(result.initial[0].truth, BoundedMuTruth::Unknown);
    }
}

#[test]
fn retained_existential_fixpoint_can_be_conclusive_before_state_cutoff() {
    let model = fork_model();
    let formula = MuFormula::mu(
        0u8,
        MuFormula::or(
            MuFormula::atom(Atom::Q),
            MuFormula::diamond(MuFormula::var(0)),
        ),
    );
    let result = evaluate_mu_with_limits(
        &model,
        &formula,
        |atom: &Atom, state: &u8| matches!(atom, Atom::Q) && *state == 1,
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied)
    );
    assert_eq!(result.initial[0].truth, BoundedMuTruth::True);
}

#[test]
fn retained_universal_fixpoint_counterexample_is_conclusive_before_state_cutoff() {
    let model = fork_model();
    let formula = MuFormula::nu(
        0u8,
        MuFormula::and(
            MuFormula::atom(Atom::P),
            MuFormula::boxed(MuFormula::var(0)),
        ),
    );
    let result = evaluate_mu_with_limits(
        &model,
        &formula,
        |atom: &Atom, state: &u8| matches!(atom, Atom::P) && *state != 1,
        ExplorationLimits {
            max_states: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BoundedMuStatus::Violated)
    );
    assert_eq!(result.initial[0].truth, BoundedMuTruth::False);
}

#[test]
fn zero_state_budget_never_vacuously_satisfies_all_initials() {
    let model = three_state_model();
    let result = evaluate_mu_with_limits(
        &model,
        &MuFormula::<Atom, u8>::True,
        |_atom: &Atom, _state: &u8| true,
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert!(!result.initial_states_complete);
    assert!(result.initial.is_empty());
    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit: 0 })
    );
}

#[test]
fn generated_two_state_cutoff_oracle_rejects_unsound_bounded_conclusions() {
    let limits = [
        ExplorationLimits::unbounded(),
        ExplorationLimits {
            max_states: Some(0),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_states: Some(1),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_transitions: Some(1),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_depth: Some(0),
            ..ExplorationLimits::unbounded()
        },
    ];
    let formulas = representative_formulas();
    let mut checked = 0usize;

    for graph_code in 0..16usize {
        let edges = decode_two_state_graph(graph_code);
        let model = two_state_model(edges.clone());

        for p_mask in 0..4u8 {
            for q_mask in 0..4u8 {
                for formula in &formulas {
                    let expected_mask = oracle_mu(formula, &edges, p_mask, q_mask);
                    let expected = expected_mask & 1 != 0;

                    for limit in limits {
                        let result = evaluate_mu_with_limits(
                            &model,
                            formula,
                            |atom: &Atom, state: &u8| match atom {
                                Atom::P => p_mask & (1 << *state) != 0,
                                Atom::Q => q_mask & (1 << *state) != 0,
                            },
                            limit,
                        )
                        .unwrap();

                        if let Some(initial) = result.initial.first() {
                            match initial.truth {
                                BoundedMuTruth::True => assert!(
                                    expected,
                                    "false positive graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                                ),
                                BoundedMuTruth::False => assert!(
                                    !expected,
                                    "false negative graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                                ),
                                BoundedMuTruth::Unknown => {}
                            }
                        }

                        match result.outcome {
                            BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied) => assert!(
                                expected,
                                "unsound satisfied graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                            ),
                            BoundedOutcome::Conclusive(BoundedMuStatus::Violated) => assert!(
                                !expected,
                                "unsound violated graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                            ),
                            BoundedOutcome::Inconclusive(_) => {}
                        }
                        checked += 1;
                    }
                }
            }
        }
    }

    assert_eq!(checked, 15_360);
}

fn representative_formulas() -> Vec<MuFormula<Atom, u8>> {
    let p = MuFormula::atom(Atom::P);
    let q = MuFormula::atom(Atom::Q);
    vec![
        p.clone(),
        MuFormula::negate(p.clone()),
        MuFormula::diamond(p.clone()),
        MuFormula::boxed(p.clone()),
        MuFormula::mu(
            0,
            MuFormula::or(q.clone(), MuFormula::diamond(MuFormula::var(0))),
        ),
        MuFormula::nu(
            0,
            MuFormula::and(p.clone(), MuFormula::diamond(MuFormula::var(0))),
        ),
        MuFormula::mu(
            0,
            MuFormula::or(
                q.clone(),
                MuFormula::and(p.clone(), MuFormula::boxed(MuFormula::var(0))),
            ),
        ),
        MuFormula::nu(
            0,
            MuFormula::and(
                p.clone(),
                MuFormula::or(
                    MuFormula::diamond(MuFormula::var(0)),
                    MuFormula::mu(
                        1,
                        MuFormula::or(
                            q.clone(),
                            MuFormula::diamond(MuFormula::var(1)),
                        ),
                    ),
                ),
            ),
        ),
        MuFormula::mu(
            0,
            MuFormula::or(
                q.clone(),
                MuFormula::diamond(MuFormula::nu(
                    0,
                    MuFormula::and(p.clone(), MuFormula::diamond(MuFormula::var(0))),
                )),
            ),
        ),
        MuFormula::negate(MuFormula::mu(
            0,
            MuFormula::or(q, MuFormula::diamond(MuFormula::var(0))),
        )),
    ]
}

fn oracle_mu(
    formula: &MuFormula<Atom, u8>,
    edges: &[Vec<u8>; 2],
    p_mask: u8,
    q_mask: u8,
) -> u8 {
    let mut environment = HashMap::new();
    oracle_eval(formula, edges, p_mask, q_mask, &mut environment)
}

fn oracle_eval(
    formula: &MuFormula<Atom, u8>,
    edges: &[Vec<u8>; 2],
    p_mask: u8,
    q_mask: u8,
    environment: &mut HashMap<u8, u8>,
) -> u8 {
    const UNIVERSE: u8 = 0b11;

    match formula {
        MuFormula::True => UNIVERSE,
        MuFormula::False => 0,
        MuFormula::Atom(Atom::P) => p_mask,
        MuFormula::Atom(Atom::Q) => q_mask,
        MuFormula::Var(variable) => *environment
            .get(variable)
            .expect("generated oracle formulas are closed"),
        MuFormula::Not(inner) => UNIVERSE ^ oracle_eval(inner, edges, p_mask, q_mask, environment),
        MuFormula::And(left, right) => {
            oracle_eval(left, edges, p_mask, q_mask, environment)
                & oracle_eval(right, edges, p_mask, q_mask, environment)
        }
        MuFormula::Or(left, right) => {
            oracle_eval(left, edges, p_mask, q_mask, environment)
                | oracle_eval(right, edges, p_mask, q_mask, environment)
        }
        MuFormula::Diamond(inner) => {
            let target = oracle_eval(inner, edges, p_mask, q_mask, environment);
            oracle_pre_exists(edges, target)
        }
        MuFormula::Box(inner) => {
            let target = oracle_eval(inner, edges, p_mask, q_mask, environment);
            oracle_pre_all(edges, target)
        }
        MuFormula::Mu { variable, body } => {
            oracle_fixpoint(*variable, body, edges, p_mask, q_mask, environment, false)
        }
        MuFormula::Nu { variable, body } => {
            oracle_fixpoint(*variable, body, edges, p_mask, q_mask, environment, true)
        }
    }
}

fn oracle_fixpoint(
    variable: u8,
    body: &MuFormula<Atom, u8>,
    edges: &[Vec<u8>; 2],
    p_mask: u8,
    q_mask: u8,
    environment: &mut HashMap<u8, u8>,
    greatest: bool,
) -> u8 {
    let previous = environment.get(&variable).copied();
    let mut current = if greatest { 0b11 } else { 0 };

    loop {
        environment.insert(variable, current);
        let next = oracle_eval(body, edges, p_mask, q_mask, environment);
        if next == current {
            match previous {
                Some(value) => {
                    environment.insert(variable, value);
                }
                None => {
                    environment.remove(&variable);
                }
            }
            return current;
        }
        current = next;
    }
}

fn oracle_pre_exists(edges: &[Vec<u8>; 2], target: u8) -> u8 {
    let mut result = 0u8;
    for state in 0..2u8 {
        if successors(edges, state)
            .into_iter()
            .any(|next| target & (1 << next) != 0)
        {
            result |= 1 << state;
        }
    }
    result
}

fn oracle_pre_all(edges: &[Vec<u8>; 2], target: u8) -> u8 {
    let mut result = 0u8;
    for state in 0..2u8 {
        if successors(edges, state)
            .into_iter()
            .all(|next| target & (1 << next) != 0)
        {
            result |= 1 << state;
        }
    }
    result
}

fn successors(edges: &[Vec<u8>; 2], state: u8) -> Vec<u8> {
    if edges[state as usize].is_empty() {
        vec![state]
    } else {
        edges[state as usize].clone()
    }
}

fn decode_two_state_graph(code: usize) -> [Vec<u8>; 2] {
    let mut edges = [Vec::new(), Vec::new()];
    for (source, source_edges) in edges.iter_mut().enumerate() {
        for target in 0..2 {
            let bit = source * 2 + target;
            if code & (1 << bit) != 0 {
                source_edges.push(target as u8);
            }
        }
    }
    edges
}

fn two_state_model(edges: [Vec<u8>; 2]) -> TransitionSystem<u8> {
    TransitionSystem::new(
        "generated-bounded-mu",
        vec![StateVariable::new("state", "two-state generated graph")],
        vec![0u8],
        move |state| {
            Ok(edges[*state as usize]
                .iter()
                .map(|target| Transition::new(format!("e{state}-{target}"), *target))
                .collect())
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap()
}

fn three_state_model() -> TransitionSystem<u8> {
    TransitionSystem::new(
        "bounded-mu-three-state",
        vec![StateVariable::new("state", "three-state graph")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("to-one", 1), Transition::new("to-two", 2)],
                1 => vec![Transition::new("stay-one", 1)],
                2 => Vec::new(),
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap()
}

fn fork_model() -> TransitionSystem<u8> {
    TransitionSystem::new(
        "bounded-mu-fork",
        vec![StateVariable::new("state", "fork graph")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("first", 1), Transition::new("second", 2)],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap()
}

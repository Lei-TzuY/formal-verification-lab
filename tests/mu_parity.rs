use formal_verification_lab::{
    evaluate_mu, evaluate_mu_via_parity, validate_mu_formula, Invariant, MuFormula, MuParityError,
    MuTerminalPolicy, MuValidationError, StateVariable, Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Atom {
    P,
    Q,
}

#[test]
fn validation_fails_before_parity_graph_capture() {
    let model = TransitionSystem::new(
        "mu-parity-validation",
        vec![StateVariable::new("state", "single state")],
        vec![0u8],
        |_state| panic!("invalid formula must fail before transition capture"),
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let unbound = MuFormula::<Atom, u8>::var(0);
    assert!(matches!(
        evaluate_mu_via_parity(&model, &unbound, |_atom, _state| false),
        Err(MuParityError::Validation(
            MuValidationError::UnboundVariable { variable: 0 }
        ))
    ));

    let non_monotone = MuFormula::<Atom, u8>::mu(0, MuFormula::negate(MuFormula::var(0)));
    assert!(matches!(
        evaluate_mu_via_parity(&model, &non_monotone, |_atom, _state| false),
        Err(MuParityError::Validation(
            MuValidationError::NonMonotoneVariable { variable: 0 }
        ))
    ));
}

#[test]
fn parity_backend_preserves_terminal_totalization_and_fixpoint_polarity() {
    let model = TransitionSystem::new(
        "mu-parity-terminal",
        vec![StateVariable::new("state", "terminal state")],
        vec![0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let formulas = [
        (MuFormula::<Atom, u8>::diamond(MuFormula::True), true),
        (MuFormula::<Atom, u8>::boxed(MuFormula::False), false),
        (
            MuFormula::<Atom, u8>::nu(0, MuFormula::diamond(MuFormula::var(0))),
            true,
        ),
        (
            MuFormula::<Atom, u8>::mu(0, MuFormula::diamond(MuFormula::var(0))),
            false,
        ),
        (
            MuFormula::<Atom, u8>::negate(MuFormula::mu(0, MuFormula::diamond(MuFormula::var(0)))),
            true,
        ),
    ];

    for (formula, expected) in formulas {
        let parity = evaluate_mu_via_parity(&model, &formula, |_atom, _state| false).unwrap();
        let direct = evaluate_mu(&model, &formula, |_atom, _state| false).unwrap();
        assert_eq!(
            parity.terminal_policy,
            MuTerminalPolicy::TotalizeWithSelfLoop
        );
        assert_eq!(parity.all_initial_states_satisfy(), expected);
        assert_eq!(
            parity.satisfying_state_indices,
            direct.satisfying_state_indices
        );
    }
}

#[test]
fn outer_fixpoint_priority_dominates_dependent_inner_alternation() {
    let model = TransitionSystem::new(
        "mu-parity-priority-regression",
        vec![StateVariable::new("state", "single terminal state")],
        vec![0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let formula = MuFormula::mu(
        0,
        MuFormula::or(
            MuFormula::atom(Atom::P),
            MuFormula::diamond(MuFormula::nu(
                1,
                MuFormula::or(
                    MuFormula::and(
                        MuFormula::atom(Atom::Q),
                        MuFormula::diamond(MuFormula::var(1)),
                    ),
                    MuFormula::diamond(MuFormula::var(0)),
                ),
            )),
        ),
    );

    let direct = evaluate_mu(&model, &formula, |_atom, _state| false).unwrap();
    let parity = evaluate_mu_via_parity(&model, &formula, |_atom, _state| false).unwrap();
    assert!(direct.satisfying_state_indices.is_empty());
    assert_eq!(
        parity.satisfying_state_indices,
        direct.satisfying_state_indices
    );
    assert!(!parity.all_initial_states_satisfy());
}

#[test]
fn generated_parity_backend_matches_m75_on_all_two_state_graphs() {
    let formulas = representative_formulas();
    let mut comparisons = 0usize;

    for graph_code in 0..16usize {
        let model = model_from_edges(decode_two_state_graph(graph_code));

        for p_mask in 0..4u8 {
            for q_mask in 0..4u8 {
                let atom = |atom: &Atom, state: &u8| match atom {
                    Atom::P => p_mask & (1 << *state) != 0,
                    Atom::Q => q_mask & (1 << *state) != 0,
                };

                for formula in &formulas {
                    validate_mu_formula(formula).unwrap();
                    let direct = evaluate_mu(&model, formula, atom).unwrap();
                    let parity = evaluate_mu_via_parity(&model, formula, atom).unwrap();

                    assert_eq!(
                        parity.satisfying_state_indices,
                        direct.satisfying_state_indices,
                        "mu parity mismatch graph={graph_code} p={p_mask} q={q_mask} formula={formula:?}"
                    );
                    assert_eq!(
                        parity.initial, direct.initial,
                        "mu parity initial mismatch graph={graph_code} p={p_mask} q={q_mask} formula={formula:?}"
                    );
                    assert_eq!(parity.discovered_states, direct.discovered_states);
                    assert_eq!(parity.explored_transitions, direct.explored_transitions);
                    assert_eq!(parity.max_depth_reached, direct.max_depth_reached);
                    assert!(parity.parity_game_vertices >= parity.reachable_states.len());
                    comparisons += 1;
                }
            }
        }
    }

    assert_eq!(comparisons, 16 * 4 * 4 * formulas.len());
    assert_eq!(comparisons, 3_584);
}

fn representative_formulas() -> Vec<MuFormula<Atom, u8>> {
    let p = MuFormula::atom(Atom::P);
    let q = MuFormula::atom(Atom::Q);

    vec![
        p.clone(),
        MuFormula::negate(p.clone()),
        MuFormula::diamond(p.clone()),
        MuFormula::boxed(p.clone()),
        MuFormula::and(p.clone(), q.clone()),
        MuFormula::or(p.clone(), MuFormula::negate(q.clone())),
        MuFormula::mu(
            0,
            MuFormula::or(p.clone(), MuFormula::diamond(MuFormula::var(0))),
        ),
        MuFormula::nu(
            0,
            MuFormula::and(p.clone(), MuFormula::diamond(MuFormula::var(0))),
        ),
        MuFormula::mu(
            0,
            MuFormula::or(p.clone(), MuFormula::boxed(MuFormula::var(0))),
        ),
        MuFormula::nu(
            0,
            MuFormula::and(p.clone(), MuFormula::boxed(MuFormula::var(0))),
        ),
        MuFormula::mu(
            0,
            MuFormula::or(
                p.clone(),
                MuFormula::diamond(MuFormula::nu(
                    1,
                    MuFormula::or(
                        MuFormula::and(q.clone(), MuFormula::diamond(MuFormula::var(1))),
                        MuFormula::diamond(MuFormula::var(0)),
                    ),
                )),
            ),
        ),
        MuFormula::nu(
            0,
            MuFormula::and(
                p.clone(),
                MuFormula::boxed(MuFormula::mu(
                    1,
                    MuFormula::and(
                        MuFormula::or(q.clone(), MuFormula::diamond(MuFormula::var(1))),
                        MuFormula::boxed(MuFormula::var(0)),
                    ),
                )),
            ),
        ),
        MuFormula::mu(
            0,
            MuFormula::or(
                p.clone(),
                MuFormula::diamond(MuFormula::nu(
                    0,
                    MuFormula::and(q.clone(), MuFormula::diamond(MuFormula::var(0))),
                )),
            ),
        ),
        MuFormula::negate(MuFormula::mu(
            0,
            MuFormula::or(q.clone(), MuFormula::diamond(MuFormula::var(0))),
        )),
    ]
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

fn model_from_edges(edges: [Vec<u8>; 2]) -> TransitionSystem<u8> {
    TransitionSystem::new(
        "mu-parity-generated",
        vec![StateVariable::new("state", "generated two-state graph")],
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

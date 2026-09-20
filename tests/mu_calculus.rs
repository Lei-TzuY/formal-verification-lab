use formal_verification_lab::{
    compile_ctl_to_mu, evaluate_ctl, evaluate_mu, validate_mu_formula, CtlFormula, Invariant,
    MuFormula, MuTerminalPolicy, MuValidationError, StateVariable, Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Atom {
    P,
    Q,
}

#[test]
fn validator_rejects_unbound_and_negative_fixpoint_variables() {
    let unbound = MuFormula::<Atom, &str>::var("X");
    assert_eq!(
        validate_mu_formula(&unbound),
        Err(MuValidationError::UnboundVariable { variable: "X" })
    );

    let non_monotone = MuFormula::<Atom, &str>::mu(
        "X",
        MuFormula::negate(MuFormula::var("X")),
    );
    assert_eq!(
        validate_mu_formula(&non_monotone),
        Err(MuValidationError::NonMonotoneVariable { variable: "X" })
    );

    let double_negation = MuFormula::<Atom, &str>::mu(
        "X",
        MuFormula::negate(MuFormula::negate(MuFormula::var("X"))),
    );
    validate_mu_formula(&double_negation).unwrap();
}

#[test]
fn lexical_shadowing_uses_the_nearest_fixpoint_binder() {
    let formula = MuFormula::<Atom, &str>::mu(
        "X",
        MuFormula::or(
            MuFormula::atom(Atom::P),
            MuFormula::diamond(MuFormula::nu(
                "X",
                MuFormula::and(
                    MuFormula::atom(Atom::Q),
                    MuFormula::diamond(MuFormula::var("X")),
                ),
            )),
        ),
    );
    validate_mu_formula(&formula).unwrap();

    let model = small_cycle_model();
    let result = evaluate_mu(&model, &formula, |atom, state| match atom {
        Atom::P => *state == 2,
        Atom::Q => *state <= 1,
    })
    .unwrap();

    assert_eq!(result.terminal_policy, MuTerminalPolicy::TotalizeWithSelfLoop);
    assert!(result.fixpoint_iterations > 0);
}

#[test]
fn least_fixpoint_reachability_and_greatest_fixpoint_recurrence_are_executable() {
    let model = small_cycle_model();

    let reach_q = MuFormula::mu(
        "X",
        MuFormula::or(
            MuFormula::atom(Atom::Q),
            MuFormula::diamond(MuFormula::var("X")),
        ),
    );
    let reach = evaluate_mu(&model, &reach_q, |atom, state| match atom {
        Atom::P => *state <= 1,
        Atom::Q => *state == 2,
    })
    .unwrap();
    assert!(reach.all_initial_states_satisfy());

    let forever_p = MuFormula::nu(
        "X",
        MuFormula::and(
            MuFormula::atom(Atom::P),
            MuFormula::diamond(MuFormula::var("X")),
        ),
    );
    let recurrent = evaluate_mu(&model, &forever_p, |atom, state| match atom {
        Atom::P => *state <= 1,
        Atom::Q => *state == 2,
    })
    .unwrap();
    assert!(recurrent.all_initial_states_satisfy());
}

#[test]
fn modal_operators_totalize_reachable_terminals_with_self_loops() {
    let model = TransitionSystem::new(
        "mu-terminal",
        vec![StateVariable::new("state", "terminal state")],
        vec![0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let diamond_true = MuFormula::<Atom, &str>::diamond(MuFormula::True);
    let box_false = MuFormula::<Atom, &str>::boxed(MuFormula::False);
    let nu_loop = MuFormula::<Atom, &str>::nu(
        "X",
        MuFormula::diamond(MuFormula::var("X")),
    );

    assert!(
        evaluate_mu(&model, &diamond_true, |_atom, _state| false)
            .unwrap()
            .all_initial_states_satisfy()
    );
    assert!(
        !evaluate_mu(&model, &box_false, |_atom, _state| false)
            .unwrap()
            .all_initial_states_satisfy()
    );
    assert!(
        evaluate_mu(&model, &nu_loop, |_atom, _state| false)
            .unwrap()
            .all_initial_states_satisfy()
    );
}

#[test]
fn general_negation_around_a_fixpoint_is_supported_when_the_binder_remains_monotone() {
    let model = small_cycle_model();
    let formula = MuFormula::<Atom, &str>::negate(MuFormula::mu(
        "X",
        MuFormula::or(
            MuFormula::atom(Atom::Q),
            MuFormula::diamond(MuFormula::var("X")),
        ),
    ));

    validate_mu_formula(&formula).unwrap();
    let result = evaluate_mu(&model, &formula, |atom, state| match atom {
        Atom::P => *state <= 1,
        Atom::Q => *state == 2,
    })
    .unwrap();
    assert!(!result.all_initial_states_satisfy());
}

#[test]
fn compiled_nested_ctl_matches_the_m69_authority() {
    let model = branching_model();
    let ctl = CtlFormula::and(
        CtlFormula::ag(CtlFormula::or(
            CtlFormula::atom(Atom::P),
            CtlFormula::ef(CtlFormula::atom(Atom::Q)),
        )),
        CtlFormula::negate(CtlFormula::af(CtlFormula::atom(Atom::Q))),
    );
    let mu = compile_ctl_to_mu(&ctl);
    validate_mu_formula(&mu).unwrap();

    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state <= 1,
        Atom::Q => *state == 2,
    };
    let ctl_result = evaluate_ctl(&model, &ctl, atom).unwrap();
    let mu_result = evaluate_mu(&model, &mu, atom).unwrap();

    assert_eq!(
        mu_result.satisfying_state_indices,
        ctl_result.satisfying_state_indices
    );
    assert_eq!(
        mu_result.all_initial_states_satisfy(),
        ctl_result.all_initial_states_satisfy()
    );
}

#[test]
fn generated_ctl_to_mu_unary_differential_matches_all_three_state_graphs() {
    let mut comparisons = 0usize;

    for graph_code in 0..512usize {
        let edges = decode_three_state_graph(graph_code);
        let model = model_from_edges(edges);

        for p_mask in 0..8u8 {
            let atom = move |_atom: &Atom, state: &u8| p_mask & (1 << *state) != 0;
            let p = CtlFormula::atom(Atom::P);
            let formulas = [
                CtlFormula::ex(p.clone()),
                CtlFormula::ax(p.clone()),
                CtlFormula::ef(p.clone()),
                CtlFormula::af(p.clone()),
                CtlFormula::eg(p.clone()),
                CtlFormula::ag(p.clone()),
            ];

            for ctl in formulas {
                let mu = compile_ctl_to_mu(&ctl);
                validate_mu_formula(&mu).unwrap();

                let ctl_result = evaluate_ctl(&model, &ctl, atom).unwrap();
                let mu_result = evaluate_mu(&model, &mu, atom).unwrap();
                assert_eq!(
                    mu_result.satisfying_state_indices,
                    ctl_result.satisfying_state_indices,
                    "CTL->mu unary mismatch graph={graph_code} p_mask={p_mask} formula={ctl:?}"
                );
                comparisons += 1;
            }
        }
    }

    assert_eq!(comparisons, 24_576);
}

#[test]
fn generated_ctl_to_mu_until_differential_matches_all_three_state_graphs() {
    let mut comparisons = 0usize;

    for graph_code in 0..512usize {
        let edges = decode_three_state_graph(graph_code);
        let model = model_from_edges(edges);

        for p_mask in 0..8u8 {
            for q_mask in 0..8u8 {
                let atom = move |atom: &Atom, state: &u8| match atom {
                    Atom::P => p_mask & (1 << *state) != 0,
                    Atom::Q => q_mask & (1 << *state) != 0,
                };
                let p = CtlFormula::atom(Atom::P);
                let q = CtlFormula::atom(Atom::Q);
                let formulas = [
                    CtlFormula::eu(p.clone(), q.clone()),
                    CtlFormula::au(p.clone(), q.clone()),
                ];

                for ctl in formulas {
                    let mu = compile_ctl_to_mu(&ctl);
                    validate_mu_formula(&mu).unwrap();

                    let ctl_result = evaluate_ctl(&model, &ctl, atom).unwrap();
                    let mu_result = evaluate_mu(&model, &mu, atom).unwrap();
                    assert_eq!(
                        mu_result.satisfying_state_indices,
                        ctl_result.satisfying_state_indices,
                        "CTL->mu until mismatch graph={graph_code} p_mask={p_mask} q_mask={q_mask} formula={ctl:?}"
                    );
                    comparisons += 1;
                }
            }
        }
    }

    assert_eq!(comparisons, 65_536);
}

fn decode_three_state_graph(code: usize) -> [Vec<u8>; 3] {
    let mut edges = [Vec::new(), Vec::new(), Vec::new()];
    for (source, source_edges) in edges.iter_mut().enumerate() {
        for target in 0..3 {
            let bit = source * 3 + target;
            if code & (1 << bit) != 0 {
                source_edges.push(target as u8);
            }
        }
    }
    edges
}

fn model_from_edges(edges: [Vec<u8>; 3]) -> TransitionSystem<u8> {
    TransitionSystem::new(
        "mu-generated",
        vec![StateVariable::new("state", "generated three-state graph")],
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

fn small_cycle_model() -> TransitionSystem<u8> {
    TransitionSystem::new(
        "mu-cycle",
        vec![StateVariable::new("state", "cycle with terminal branch")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("next", 1), Transition::new("finish", 2)],
                1 => vec![Transition::new("back", 0)],
                2 => Vec::new(),
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap()
}

fn branching_model() -> TransitionSystem<u8> {
    TransitionSystem::new(
        "mu-branching",
        vec![StateVariable::new("state", "branching graph")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("cycle", 1), Transition::new("done", 2)],
                1 => vec![Transition::new("back", 0)],
                2 => Vec::new(),
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap()
}

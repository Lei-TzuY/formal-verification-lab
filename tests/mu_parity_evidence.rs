use formal_verification_lab::{
    evaluate_mu, evaluate_mu_via_parity, verify_mu_parity_evidence, Invariant, MuFormula,
    MuParityEvidenceError, MuParityMove, MuParityPositionKind, ParityPlayer, StateVariable,
    Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Atom {
    P,
    Q,
}

#[test]
fn typed_strategy_evidence_maps_semantic_moves_and_initial_winners() {
    let terminal = TransitionSystem::new(
        "mu-evidence-terminal",
        vec![StateVariable::new("state", "terminal state")],
        vec![0u8],
        |_state| Ok(Vec::new()),
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let true_result =
        evaluate_mu_via_parity(&terminal, &MuFormula::<Atom, u8>::True, |_atom, _state| false)
            .unwrap();
    assert!(true_result
        .even_strategy
        .choices
        .iter()
        .any(|choice| choice.semantic_move == MuParityMove::OutcomeSelfLoop));
    assert_eq!(true_result.initial_evidence[0].winner, ParityPlayer::Even);
    assert!(true_result.initial_evidence[0].satisfied);
    assert!(matches!(
        true_result.initial_evidence[0].root_position.kind,
        MuParityPositionKind::True
    ));
    assert!(verify_mu_parity_evidence(
        &terminal,
        &MuFormula::<Atom, u8>::True,
        |_atom, _state| false,
        &true_result,
    )
    .is_ok());

    let and_formula = MuFormula::<Atom, u8>::and(MuFormula::True, MuFormula::False);
    let and_result =
        evaluate_mu_via_parity(&terminal, &and_formula, |_atom, _state| false).unwrap();
    assert!(!and_result.initial_evidence[0].satisfied);
    assert_eq!(and_result.initial_evidence[0].winner, ParityPlayer::Odd);
    assert!(and_result
        .odd_strategy
        .choices
        .iter()
        .any(|choice| choice.semantic_move == MuParityMove::BooleanRight));

    let terminal_modal = MuFormula::<Atom, u8>::diamond(MuFormula::True);
    let terminal_modal_result =
        evaluate_mu_via_parity(&terminal, &terminal_modal, |_atom, _state| false).unwrap();
    assert!(terminal_modal_result
        .even_strategy
        .choices
        .iter()
        .any(|choice| choice.semantic_move == MuParityMove::ModalTerminalSelfLoop));

    let nu_formula = MuFormula::<Atom, u8>::nu(
        0,
        MuFormula::diamond(MuFormula::var(0)),
    );
    let nu_result =
        evaluate_mu_via_parity(&terminal, &nu_formula, |_atom, _state| false).unwrap();
    assert!(nu_result
        .even_strategy
        .choices
        .iter()
        .any(|choice| choice.semantic_move == MuParityMove::FixpointBody));
    assert!(nu_result.even_strategy.choices.iter().any(|choice| matches!(
        choice.semantic_move,
        MuParityMove::VariableReturn { .. }
    )));

    let edge_model = TransitionSystem::new(
        "mu-evidence-edge",
        vec![StateVariable::new("state", "two states")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("advance", 1)],
                _ => vec![Transition::new("stay", 1)],
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();
    let modal = MuFormula::<Atom, u8>::diamond(MuFormula::True);
    let modal_result =
        evaluate_mu_via_parity(&edge_model, &modal, |_atom, _state| false).unwrap();
    assert!(modal_result.even_strategy.choices.iter().any(|choice| matches!(
        choice.semantic_move,
        MuParityMove::ModalSuccessor {
            target_state_index: 1
        }
    )));
    assert!(verify_mu_parity_evidence(
        &edge_model,
        &modal,
        |_atom, _state| false,
        &modal_result,
    )
    .is_ok());
}

#[test]
fn evidence_verifier_rejects_position_move_strategy_partition_and_initial_tampering() {
    let model = TransitionSystem::new(
        "mu-evidence-tamper",
        vec![StateVariable::new("state", "two states")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![Transition::new("to-one", 1)],
                _ => vec![Transition::new("stay", 1)],
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();

    let truth = MuFormula::<Atom, u8>::True;
    let original = evaluate_mu_via_parity(&model, &truth, |_atom, _state| false).unwrap();

    let mut bad_position = original.clone();
    bad_position.positions[0].kind = MuParityPositionKind::False;
    assert!(matches!(
        verify_mu_parity_evidence(
            &model,
            &truth,
            |_atom, _state| false,
            &bad_position
        ),
        Err(MuParityEvidenceError::PositionMismatch { vertex: 0 })
    ));

    let mut bad_move = original.clone();
    let first = bad_move
        .even_strategy
        .choices
        .first_mut()
        .expect("true game has an Even-owned winning sink");
    first.semantic_move = MuParityMove::BooleanLeft;
    assert!(matches!(
        verify_mu_parity_evidence(&model, &truth, |_atom, _state| false, &bad_move),
        Err(MuParityEvidenceError::SemanticMoveMismatch {
            player: ParityPlayer::Even,
            ..
        })
    ));

    let mut missing_move = original.clone();
    missing_move.even_strategy.choices.clear();
    assert!(matches!(
        verify_mu_parity_evidence(
            &model,
            &truth,
            |_atom, _state| false,
            &missing_move
        ),
        Err(MuParityEvidenceError::Strategy {
            player: ParityPlayer::Even,
            ..
        })
    ));

    let mut bad_initial = original.clone();
    bad_initial.initial_evidence[0].winner = ParityPlayer::Odd;
    assert!(matches!(
        verify_mu_parity_evidence(
            &model,
            &truth,
            |_atom, _state| false,
            &bad_initial
        ),
        Err(MuParityEvidenceError::InitialEvidenceMismatch { index: 0 })
    ));

    let atom_formula = MuFormula::<Atom, u8>::atom(Atom::P);
    let atom_result =
        evaluate_mu_via_parity(&model, &atom_formula, |atom, state| {
            matches!(atom, Atom::P) && *state == 0
        })
        .unwrap();
    let mut bad_partition = atom_result.clone();
    bad_partition.odd_strategy.winning_positions.clear();
    assert!(matches!(
        verify_mu_parity_evidence(
            &model,
            &atom_formula,
            |atom, state| matches!(atom, Atom::P) && *state == 0,
            &bad_partition
        ),
        Err(MuParityEvidenceError::WinningPartitionMismatch)
    ));
}

#[test]
fn nested_alternation_and_shadowing_evidence_is_canonical_and_verifiable() {
    let model = model_from_edges([vec![0, 1], vec![1]]);
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
                    MuFormula::diamond(MuFormula::mu(
                        0,
                        MuFormula::or(
                            MuFormula::atom(Atom::Q),
                            MuFormula::diamond(MuFormula::var(0)),
                        ),
                    )),
                ),
            )),
        ),
    );

    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state == 1,
        Atom::Q => *state == 0,
    };
    let direct = evaluate_mu(&model, &formula, atom).unwrap();
    let parity = evaluate_mu_via_parity(&model, &formula, atom).unwrap();

    assert_eq!(parity.satisfying_state_indices, direct.satisfying_state_indices);
    assert!(parity.positions.iter().any(|position| matches!(
        position.kind,
        MuParityPositionKind::Fixpoint { .. }
    )));
    assert!(parity.positions.iter().any(|position| matches!(
        position.kind,
        MuParityPositionKind::Variable { .. }
    )));
    assert!(verify_mu_parity_evidence(&model, &formula, atom, &parity).is_ok());
}

#[test]
fn generated_typed_evidence_matches_m75_and_m81_on_all_two_state_graphs() {
    let formulas = representative_formulas();
    let mut comparisons = 0usize;

    for graph_code in 0..16usize {
        let model = model_from_edges(decode_two_state_graph(graph_code));

        for p_mask in 0..4u8 {
            for q_mask in 0..4u8 {
                for formula in &formulas {
                    let direct = evaluate_mu(&model, formula, |atom, state| match atom {
                        Atom::P => p_mask & (1 << *state) != 0,
                        Atom::Q => q_mask & (1 << *state) != 0,
                    })
                    .unwrap();
                    let parity = evaluate_mu_via_parity(&model, formula, |atom, state| match atom {
                        Atom::P => p_mask & (1 << *state) != 0,
                        Atom::Q => q_mask & (1 << *state) != 0,
                    })
                    .unwrap();

                    assert_eq!(
                        parity.satisfying_state_indices,
                        direct.satisfying_state_indices,
                        "truth mismatch graph={graph_code} p={p_mask} q={q_mask} formula={formula:?}"
                    );
                    assert_eq!(
                        parity.initial,
                        direct.initial,
                        "initial mismatch graph={graph_code} p={p_mask} q={q_mask} formula={formula:?}"
                    );
                    assert!(
                        verify_mu_parity_evidence(
                            &model,
                            formula,
                            |atom, state| match atom {
                                Atom::P => p_mask & (1 << *state) != 0,
                                Atom::Q => q_mask & (1 << *state) != 0,
                            },
                            &parity,
                        )
                        .is_ok(),
                        "evidence verification failed graph={graph_code} p={p_mask} q={q_mask} formula={formula:?}"
                    );

                    for initial in &parity.initial_evidence {
                        assert_eq!(
                            initial.winner,
                            if initial.satisfied {
                                ParityPlayer::Even
                            } else {
                                ParityPlayer::Odd
                            }
                        );
                    }
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
        "mu-evidence-generated",
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

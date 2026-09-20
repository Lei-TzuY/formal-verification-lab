use formal_verification_lab::{
    evaluate_ctl, CtlEvidence, CtlEvidenceAction, CtlFormula, CtlTerminalPolicy, Invariant,
    StateVariable, Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Atom {
    P,
    Q,
}

#[test]
fn terminal_states_are_totalized_with_one_self_loop() {
    let model = model_from_edges([Vec::new(), Vec::new(), Vec::new()]);
    let p = CtlFormula::atom(Atom::P);
    let atom = |_atom: &Atom, state: &u8| *state == 0;

    for formula in [
        CtlFormula::ex(p.clone()),
        CtlFormula::ax(p.clone()),
        CtlFormula::ef(p.clone()),
        CtlFormula::af(p.clone()),
        CtlFormula::eg(p.clone()),
        CtlFormula::ag(p.clone()),
        CtlFormula::eu(CtlFormula::True, p.clone()),
        CtlFormula::au(CtlFormula::True, p.clone()),
    ] {
        let result = evaluate_ctl(&model, &formula, atom).unwrap();
        assert_eq!(
            result.terminal_policy,
            CtlTerminalPolicy::TotalizeWithSelfLoop
        );
        assert!(result.initial[0].satisfied);
    }

    let ex = evaluate_ctl(&model, &CtlFormula::ex(p), atom).unwrap();
    assert_eq!(
        ex.initial[0].evidence,
        Some(CtlEvidence::Finite {
            trace: vec![
                formal_verification_lab::CtlEvidenceStep {
                    action: None,
                    state: 0,
                },
                formal_verification_lab::CtlEvidenceStep {
                    action: Some(CtlEvidenceAction::TerminalSelfLoop),
                    state: 0,
                },
            ],
        })
    );
}

#[test]
fn branching_next_distinguishes_existential_and_universal_paths() {
    let model = model_from_edges([vec![(0, 1), (1, 2)], Vec::new(), Vec::new()]);
    let good = CtlFormula::atom(Atom::P);
    let atom = |_atom: &Atom, state: &u8| *state == 1;

    let ex = evaluate_ctl(&model, &CtlFormula::ex(good.clone()), atom).unwrap();
    let ax = evaluate_ctl(&model, &CtlFormula::ax(good), atom).unwrap();

    assert!(ex.initial[0].satisfied);
    assert!(!ax.initial[0].satisfied);

    match ax.initial[0].evidence.as_ref().unwrap() {
        CtlEvidence::Finite { trace } => {
            assert_eq!(trace.len(), 2);
            assert_eq!(trace[1].state, 2);
            assert_eq!(
                trace[1].action,
                Some(CtlEvidenceAction::Model("e0-2".to_owned()))
            );
        }
        other => panic!("expected finite AX counterevidence, got {other:?}"),
    }
}

#[test]
fn existential_eventually_and_until_emit_deterministic_finite_evidence() {
    let model = model_from_edges([vec![(0, 1)], vec![(0, 2)], Vec::new()]);
    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state != 2,
        Atom::Q => *state == 2,
    };

    let ef = evaluate_ctl(&model, &CtlFormula::ef(CtlFormula::atom(Atom::Q)), atom).unwrap();
    let eu = evaluate_ctl(
        &model,
        &CtlFormula::eu(CtlFormula::atom(Atom::P), CtlFormula::atom(Atom::Q)),
        atom,
    )
    .unwrap();

    assert!(ef.initial[0].satisfied);
    assert!(eu.initial[0].satisfied);
    assert_eq!(ef.initial[0].evidence, eu.initial[0].evidence);

    match ef.initial[0].evidence.as_ref().unwrap() {
        CtlEvidence::Finite { trace } => {
            assert_eq!(
                trace.iter().map(|step| step.state).collect::<Vec<_>>(),
                vec![0, 1, 2]
            );
        }
        other => panic!("expected finite EF evidence, got {other:?}"),
    }
}

#[test]
fn existential_globally_and_failed_af_emit_closed_lassos() {
    let model = model_from_edges([vec![(0, 1)], vec![(0, 0)], Vec::new()]);
    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state <= 1,
        Atom::Q => *state == 2,
    };

    let eg = evaluate_ctl(&model, &CtlFormula::eg(CtlFormula::atom(Atom::P)), atom).unwrap();
    let af = evaluate_ctl(&model, &CtlFormula::af(CtlFormula::atom(Atom::Q)), atom).unwrap();

    assert!(eg.initial[0].satisfied);
    assert!(!af.initial[0].satisfied);

    for evidence in [
        eg.initial[0].evidence.as_ref().unwrap(),
        af.initial[0].evidence.as_ref().unwrap(),
    ] {
        match evidence {
            CtlEvidence::Lasso { trace, cycle_start } => {
                assert!(trace.len() >= 2);
                assert_eq!(trace[*cycle_start].state, trace.last().unwrap().state);
            }
            other => panic!("expected lasso evidence, got {other:?}"),
        }
    }
}

#[test]
fn universal_until_failure_reports_bad_prefix_or_nonterminating_avoidance() {
    let finite_bad = model_from_edges([vec![(0, 1)], Vec::new(), Vec::new()]);
    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state == 0,
        Atom::Q => false,
    };
    let formula = CtlFormula::au(CtlFormula::atom(Atom::P), CtlFormula::atom(Atom::Q));
    let result = evaluate_ctl(&finite_bad, &formula, atom).unwrap();
    assert!(!result.initial[0].satisfied);
    assert!(matches!(
        result.initial[0].evidence,
        Some(CtlEvidence::Finite { .. })
    ));

    let infinite_avoidance = model_from_edges([vec![(0, 1)], vec![(0, 0)], Vec::new()]);
    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state <= 1,
        Atom::Q => false,
    };
    let result = evaluate_ctl(&infinite_avoidance, &formula, atom).unwrap();
    assert!(!result.initial[0].satisfied);
    assert!(matches!(
        result.initial[0].evidence,
        Some(CtlEvidence::Lasso { .. })
    ));
}

#[test]
fn nested_formula_reuses_structurally_identical_subformula_sets() {
    let model = model_from_edges([
        vec![(0, 1)],
        vec![(0, 2)],
        Vec::new(),
    ]);
    let eventually_p = CtlFormula::ef(CtlFormula::atom(Atom::P));
    let formula = CtlFormula::and(eventually_p.clone(), eventually_p);
    let atom = |_atom: &Atom, state: &u8| *state == 2;

    let result = evaluate_ctl(&model, &formula, atom).unwrap();
    assert!(result.all_initial_states_satisfy());
    assert_eq!(result.memoized_subformulas, 3);
}

#[test]
fn nested_branching_formula_and_classic_dualities_hold() {
    let model = model_from_edges([vec![(0, 1), (1, 2)], vec![(0, 1)], vec![(0, 2)]]);
    let atom = |_atom: &Atom, state: &u8| *state == 1;
    let p = CtlFormula::atom(Atom::P);

    let nested = CtlFormula::ag(CtlFormula::ef(p.clone()));
    let nested_result = evaluate_ctl(&model, &nested, atom).unwrap();
    assert!(!nested_result.initial[0].satisfied);

    for (left, right) in [
        (
            CtlFormula::ax(p.clone()),
            CtlFormula::negate(CtlFormula::ex(CtlFormula::negate(p.clone()))),
        ),
        (
            CtlFormula::af(p.clone()),
            CtlFormula::negate(CtlFormula::eg(CtlFormula::negate(p.clone()))),
        ),
        (
            CtlFormula::ag(p.clone()),
            CtlFormula::negate(CtlFormula::ef(CtlFormula::negate(p.clone()))),
        ),
    ] {
        let left = evaluate_ctl(&model, &left, atom).unwrap();
        let right = evaluate_ctl(&model, &right, atom).unwrap();
        assert_eq!(
            left.satisfying_state_indices,
            right.satisfying_state_indices
        );
    }
}

#[test]
fn generated_unary_ctl_matches_independent_path_oracle() {
    for graph_code in 0..512usize {
        let edges = decode_graph(graph_code);
        let model = model_from_edges(edges.clone());
        for p_mask in 0..8u8 {
            let atom = move |_atom: &Atom, state: &u8| p_mask & (1 << *state) != 0;
            let p = CtlFormula::atom(Atom::P);

            let formulas = [
                (CtlFormula::ex(p.clone()), UnaryKind::Ex),
                (CtlFormula::ax(p.clone()), UnaryKind::Ax),
                (CtlFormula::ef(p.clone()), UnaryKind::Ef),
                (CtlFormula::af(p.clone()), UnaryKind::Af),
                (CtlFormula::eg(p.clone()), UnaryKind::Eg),
                (CtlFormula::ag(p.clone()), UnaryKind::Ag),
            ];

            for (formula, kind) in formulas {
                let actual = evaluate_ctl(&model, &formula, atom).unwrap();
                for (index, state) in actual.reachable_states.iter().enumerate() {
                    let expected = oracle_unary(&edges, p_mask, *state, kind);
                    assert_eq!(
                        actual.satisfying_state_indices.contains(&index),
                        expected,
                        "unary CTL mismatch graph={graph_code} p_mask={p_mask} state={state} kind={kind:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn generated_until_ctl_matches_independent_path_oracle() {
    for graph_code in 0..512usize {
        let edges = decode_graph(graph_code);
        let model = model_from_edges(edges.clone());
        for p_mask in 0..8u8 {
            for q_mask in 0..8u8 {
                let atom = move |atom: &Atom, state: &u8| match atom {
                    Atom::P => p_mask & (1 << *state) != 0,
                    Atom::Q => q_mask & (1 << *state) != 0,
                };
                let p = CtlFormula::atom(Atom::P);
                let q = CtlFormula::atom(Atom::Q);

                let eu = evaluate_ctl(&model, &CtlFormula::eu(p.clone(), q.clone()), atom).unwrap();
                let au = evaluate_ctl(&model, &CtlFormula::au(p, q), atom).unwrap();

                for (index, state) in eu.reachable_states.iter().enumerate() {
                    assert_eq!(
                        eu.satisfying_state_indices.contains(&index),
                        oracle_eu(&edges, p_mask, q_mask, *state),
                        "EU mismatch graph={graph_code} p_mask={p_mask} q_mask={q_mask} state={state}"
                    );
                    assert_eq!(
                        au.satisfying_state_indices.contains(&index),
                        oracle_au(&edges, p_mask, q_mask, *state),
                        "AU mismatch graph={graph_code} p_mask={p_mask} q_mask={q_mask} state={state}"
                    );
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UnaryKind {
    Ex,
    Ax,
    Ef,
    Af,
    Eg,
    Ag,
}

fn oracle_unary(edges: &[Vec<(usize, u8)>; 3], mask: u8, state: u8, kind: UnaryKind) -> bool {
    let holds = |state: u8| mask & (1 << state) != 0;
    match kind {
        UnaryKind::Ex => successors(edges, state).into_iter().any(holds),
        UnaryKind::Ax => successors(edges, state).into_iter().all(holds),
        UnaryKind::Ef => oracle_ef(edges, mask, state),
        UnaryKind::Af => !oracle_eg(edges, !mask & 0b111, state),
        UnaryKind::Eg => oracle_eg(edges, mask, state),
        UnaryKind::Ag => !oracle_ef(edges, !mask & 0b111, state),
    }
}

fn oracle_ef(edges: &[Vec<(usize, u8)>; 3], mask: u8, source: u8) -> bool {
    let mut queue = std::collections::VecDeque::from([source]);
    let mut seen = [false; 3];
    seen[source as usize] = true;

    while let Some(state) = queue.pop_front() {
        if mask & (1 << state) != 0 {
            return true;
        }
        for next in successors(edges, state) {
            if !seen[next as usize] {
                seen[next as usize] = true;
                queue.push_back(next);
            }
        }
    }
    false
}

fn oracle_eg(edges: &[Vec<(usize, u8)>; 3], mask: u8, source: u8) -> bool {
    fn prefix(edges: &[Vec<(usize, u8)>; 3], mask: u8, state: u8, transitions_left: usize) -> bool {
        if mask & (1 << state) == 0 {
            return false;
        }
        if transitions_left == 0 {
            return true;
        }
        successors(edges, state)
            .into_iter()
            .any(|next| prefix(edges, mask, next, transitions_left - 1))
    }

    prefix(edges, mask, source, 3)
}

fn oracle_eu(edges: &[Vec<(usize, u8)>; 3], p_mask: u8, q_mask: u8, source: u8) -> bool {
    fn search(
        edges: &[Vec<(usize, u8)>; 3],
        p_mask: u8,
        q_mask: u8,
        state: u8,
        visited: u8,
    ) -> bool {
        if q_mask & (1 << state) != 0 {
            return true;
        }
        if p_mask & (1 << state) == 0 {
            return false;
        }

        for next in successors(edges, state) {
            let bit = 1 << next;
            if visited & bit == 0 && search(edges, p_mask, q_mask, next, visited | bit) {
                return true;
            }
        }
        false
    }

    search(edges, p_mask, q_mask, source, 1 << source)
}

fn oracle_au(edges: &[Vec<(usize, u8)>; 3], p_mask: u8, q_mask: u8, source: u8) -> bool {
    let not_q = !q_mask & 0b111;
    let bad = (!p_mask & 0b111) & not_q;
    !oracle_eu(edges, not_q, bad, source) && !oracle_eg(edges, not_q, source)
}

fn successors(edges: &[Vec<(usize, u8)>; 3], state: u8) -> Vec<u8> {
    if edges[state as usize].is_empty() {
        vec![state]
    } else {
        edges[state as usize]
            .iter()
            .map(|(_, target)| *target)
            .collect()
    }
}

fn decode_graph(code: usize) -> [Vec<(usize, u8)>; 3] {
    let mut edges = [Vec::new(), Vec::new(), Vec::new()];
    for source in 0..3 {
        for target in 0..3 {
            let bit = source * 3 + target;
            if code & (1 << bit) != 0 {
                edges[source].push((target, target as u8));
            }
        }
    }
    edges
}

fn model_from_edges(edges: [Vec<(usize, u8)>; 3]) -> TransitionSystem<u8> {
    TransitionSystem::new(
        "ctl-test-model",
        vec![StateVariable::new("state", "small CTL control state")],
        vec![0u8],
        move |state| {
            Ok(edges[*state as usize]
                .iter()
                .map(|(_ordinal, target)| Transition::new(format!("e{state}-{target}"), *target))
                .collect())
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap()
}

use formal_verification_lab::{
    evaluate_ctl, evaluate_ctl_with_limits, BoundedCtlStatus, BoundedCtlTruth, BoundedOutcome,
    CtlEvidence, CtlFormula, ExplorationLimits, InconclusiveReason, Invariant, StateVariable,
    Transition, TransitionSystem,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Atom {
    P,
    Q,
}

#[test]
fn unbounded_limits_collapse_exactly_to_m69_semantics_and_evidence() {
    let model = three_state_model();
    let atom = |atom: &Atom, state: &u8| match atom {
        Atom::P => *state != 2,
        Atom::Q => *state == 2,
    };
    let p = CtlFormula::atom(Atom::P);
    let q = CtlFormula::atom(Atom::Q);
    let formulas = vec![
        CtlFormula::ex(q.clone()),
        CtlFormula::ax(p.clone()),
        CtlFormula::ef(q.clone()),
        CtlFormula::af(q.clone()),
        CtlFormula::eg(p.clone()),
        CtlFormula::ag(p.clone()),
        CtlFormula::eu(p.clone(), q.clone()),
        CtlFormula::au(p.clone(), q.clone()),
        CtlFormula::ag(CtlFormula::or(p, CtlFormula::ef(q))),
    ];

    for formula in formulas {
        let exact = evaluate_ctl(&model, &formula, atom).unwrap();
        let bounded =
            evaluate_ctl_with_limits(&model, &formula, atom, ExplorationLimits::unbounded())
                .unwrap();

        assert_eq!(
            bounded.outcome,
            BoundedOutcome::Conclusive(if exact.all_initial_states_satisfy() {
                BoundedCtlStatus::Satisfied
            } else {
                BoundedCtlStatus::Violated
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
        assert_eq!(bounded.memoized_subformulas, exact.memoized_subformulas);
        assert_eq!(bounded.initial.len(), exact.initial.len());

        for (bounded_initial, exact_initial) in bounded.initial.iter().zip(&exact.initial) {
            assert_eq!(bounded_initial.state_index, exact_initial.state_index);
            assert_eq!(bounded_initial.state, exact_initial.state);
            assert_eq!(
                bounded_initial.truth,
                if exact_initial.satisfied {
                    BoundedCtlTruth::True
                } else {
                    BoundedCtlTruth::False
                }
            );
            assert_eq!(bounded_initial.evidence, exact_initial.evidence);
        }
    }
}

#[test]
fn depth_cutoff_does_not_fabricate_terminal_self_loop() {
    let model = TransitionSystem::new(
        "depth-cutoff",
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

    for formula in [
        CtlFormula::<Atom>::ex(CtlFormula::True),
        CtlFormula::<Atom>::eg(CtlFormula::True),
        CtlFormula::<Atom>::negate(CtlFormula::ex(CtlFormula::atom(Atom::P))),
    ] {
        let result = evaluate_ctl_with_limits(
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
        assert_eq!(result.initial[0].truth, BoundedCtlTruth::Unknown);
        assert!(result.initial[0].evidence.is_none());
    }
}

#[test]
fn retained_existential_witness_is_conclusive_before_later_state_cutoff() {
    let model = fork_model();
    let limits = ExplorationLimits {
        max_states: Some(2),
        ..ExplorationLimits::unbounded()
    };
    let formula = CtlFormula::ef(CtlFormula::atom(Atom::P));
    let result = evaluate_ctl_with_limits(
        &model,
        &formula,
        |_atom: &Atom, state: &u8| *state == 1,
        limits,
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    );
    assert_eq!(result.initial[0].truth, BoundedCtlTruth::True);
    match result.initial[0].evidence.as_ref().unwrap() {
        CtlEvidence::Finite { trace } => {
            assert_eq!(
                trace.iter().map(|step| step.state).collect::<Vec<_>>(),
                vec![0, 1]
            );
        }
        other => panic!("expected finite retained witness, got {other:?}"),
    }
}

#[test]
fn retained_universal_counterexample_is_conclusive_before_later_state_cutoff() {
    let model = fork_model();
    let limits = ExplorationLimits {
        max_states: Some(2),
        ..ExplorationLimits::unbounded()
    };
    let formula = CtlFormula::ag(CtlFormula::atom(Atom::P));
    let result = evaluate_ctl_with_limits(
        &model,
        &formula,
        |_atom: &Atom, state: &u8| *state != 1,
        limits,
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Violated)
    );
    assert_eq!(result.initial[0].truth, BoundedCtlTruth::False);
    assert!(matches!(
        result.initial[0].evidence,
        Some(CtlEvidence::Finite { .. })
    ));
}

#[test]
fn retained_cycle_can_conclusively_prove_eg_and_refute_af() {
    let model = TransitionSystem::new(
        "retained-cycle",
        vec![StateVariable::new("state", "cycle plus unseen branch")],
        vec![0u8],
        |state| {
            Ok(match *state {
                0 => vec![
                    Transition::new("stay", 0),
                    Transition::new("branch", 1),
                ],
                _ => Vec::new(),
            })
        },
        vec![Invariant::new("always", |_state: &u8| true)],
    )
    .unwrap();
    let limits = ExplorationLimits {
        max_transitions: Some(1),
        ..ExplorationLimits::unbounded()
    };

    let eg = evaluate_ctl_with_limits(
        &model,
        &CtlFormula::eg(CtlFormula::atom(Atom::P)),
        |_atom: &Atom, state: &u8| *state == 0,
        limits,
    )
    .unwrap();
    assert_eq!(
        eg.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied)
    );
    assert!(matches!(
        eg.initial[0].evidence,
        Some(CtlEvidence::Lasso { .. })
    ));

    let af = evaluate_ctl_with_limits(
        &model,
        &CtlFormula::af(CtlFormula::atom(Atom::Q)),
        |_atom: &Atom, _state: &u8| false,
        limits,
    )
    .unwrap();
    assert_eq!(
        af.outcome,
        BoundedOutcome::Conclusive(BoundedCtlStatus::Violated)
    );
    assert!(matches!(
        af.initial[0].evidence,
        Some(CtlEvidence::Lasso { .. })
    ));
}

#[test]
fn zero_state_budget_never_vacuously_satisfies_all_initials() {
    let model = three_state_model();
    let limits = ExplorationLimits {
        max_states: Some(0),
        ..ExplorationLimits::unbounded()
    };
    let result = evaluate_ctl_with_limits(
        &model,
        &CtlFormula::<Atom>::True,
        |_atom: &Atom, _state: &u8| true,
        limits,
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
fn generated_two_state_graph_cutoff_oracle_never_allows_false_conclusions() {
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

    let mut checked = 0usize;
    for graph_code in 0..16usize {
        let edges = decode_two_state_graph(graph_code);
        let model = two_state_model(edges.clone());

        for p_mask in 0..4u8 {
            for q_mask in 0..4u8 {
                let formulas = generated_formulas();
                for (formula, oracle) in formulas {
                    let expected = oracle(&edges, p_mask, q_mask, 0);
                    for limit in limits {
                        let result = evaluate_ctl_with_limits(
                            &model,
                            &formula,
                            |atom: &Atom, state: &u8| match atom {
                                Atom::P => p_mask & (1 << *state) != 0,
                                Atom::Q => q_mask & (1 << *state) != 0,
                            },
                            limit,
                        )
                        .unwrap();

                        if let Some(initial) = result.initial.first() {
                            match initial.truth {
                                BoundedCtlTruth::True => assert!(
                                    expected,
                                    "false positive graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                                ),
                                BoundedCtlTruth::False => assert!(
                                    !expected,
                                    "false negative graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                                ),
                                BoundedCtlTruth::Unknown => {}
                            }
                        }

                        match result.outcome {
                            BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied) => assert!(
                                expected,
                                "unsound satisfied graph={graph_code} p={p_mask} q={q_mask} formula={formula:?} limit={limit:?}"
                            ),
                            BoundedOutcome::Conclusive(BoundedCtlStatus::Violated) => assert!(
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

    assert_eq!(checked, 12_288);
}

type Oracle = fn(&[Vec<u8>; 2], u8, u8, u8) -> bool;

fn generated_formulas() -> Vec<(CtlFormula<Atom>, Oracle)> {
    let p = CtlFormula::atom(Atom::P);
    let q = CtlFormula::atom(Atom::Q);
    vec![
        (CtlFormula::ex(p.clone()), oracle_ex_p),
        (CtlFormula::ax(p.clone()), oracle_ax_p),
        (CtlFormula::ef(p.clone()), oracle_ef_p),
        (CtlFormula::af(p.clone()), oracle_af_p),
        (CtlFormula::eg(p.clone()), oracle_eg_p),
        (CtlFormula::ag(p.clone()), oracle_ag_p),
        (CtlFormula::eu(p.clone(), q.clone()), oracle_eu_p_q),
        (CtlFormula::au(p, q), oracle_au_p_q),
    ]
}

fn oracle_ex_p(edges: &[Vec<u8>; 2], p: u8, _q: u8, state: u8) -> bool {
    successors(edges, state)
        .into_iter()
        .any(|next| holds(p, next))
}

fn oracle_ax_p(edges: &[Vec<u8>; 2], p: u8, _q: u8, state: u8) -> bool {
    successors(edges, state)
        .into_iter()
        .all(|next| holds(p, next))
}

fn oracle_ef_p(edges: &[Vec<u8>; 2], p: u8, _q: u8, state: u8) -> bool {
    oracle_ef(edges, p, state)
}

fn oracle_af_p(edges: &[Vec<u8>; 2], p: u8, _q: u8, state: u8) -> bool {
    !oracle_eg(edges, !p & 0b11, state)
}

fn oracle_eg_p(edges: &[Vec<u8>; 2], p: u8, _q: u8, state: u8) -> bool {
    oracle_eg(edges, p, state)
}

fn oracle_ag_p(edges: &[Vec<u8>; 2], p: u8, _q: u8, state: u8) -> bool {
    !oracle_ef(edges, !p & 0b11, state)
}

fn oracle_eu_p_q(edges: &[Vec<u8>; 2], p: u8, q: u8, state: u8) -> bool {
    oracle_eu(edges, p, q, state)
}

fn oracle_au_p_q(edges: &[Vec<u8>; 2], p: u8, q: u8, state: u8) -> bool {
    let not_q = !q & 0b11;
    let bad = (!p & 0b11) & not_q;
    !oracle_eu(edges, not_q, bad, state) && !oracle_eg(edges, not_q, state)
}

fn oracle_ef(edges: &[Vec<u8>; 2], mask: u8, source: u8) -> bool {
    let mut queue = std::collections::VecDeque::from([source]);
    let mut seen = [false; 2];
    seen[source as usize] = true;

    while let Some(state) = queue.pop_front() {
        if holds(mask, state) {
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

fn oracle_eg(edges: &[Vec<u8>; 2], mask: u8, source: u8) -> bool {
    fn prefix(edges: &[Vec<u8>; 2], mask: u8, state: u8, left: usize) -> bool {
        if !holds(mask, state) {
            return false;
        }
        if left == 0 {
            return true;
        }
        successors(edges, state)
            .into_iter()
            .any(|next| prefix(edges, mask, next, left - 1))
    }

    prefix(edges, mask, source, 2)
}

fn oracle_eu(edges: &[Vec<u8>; 2], p: u8, q: u8, source: u8) -> bool {
    fn search(edges: &[Vec<u8>; 2], p: u8, q: u8, state: u8, visited: u8) -> bool {
        if holds(q, state) {
            return true;
        }
        if !holds(p, state) {
            return false;
        }

        for next in successors(edges, state) {
            let bit = 1 << next;
            if visited & bit == 0 && search(edges, p, q, next, visited | bit) {
                return true;
            }
        }
        false
    }

    search(edges, p, q, source, 1 << source)
}

fn holds(mask: u8, state: u8) -> bool {
    mask & (1 << state) != 0
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
        "generated-bounded-ctl",
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
        "bounded-ctl-three-state",
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
        "bounded-ctl-fork",
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

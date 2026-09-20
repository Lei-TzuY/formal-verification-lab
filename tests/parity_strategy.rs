use formal_verification_lab::{
    solve_parity_game, verify_parity_strategy, ParityGame, ParityPlayer, ParityStrategy,
    ParityStrategyError,
};

#[test]
fn solver_emits_certified_deterministic_strategies_without_changing_regions() {
    let game = ParityGame::new(
        vec![ParityPlayer::Even, ParityPlayer::Odd, ParityPlayer::Even],
        vec![0, 1, 2],
        vec![vec![1, 2], vec![1], vec![2]],
    )
    .unwrap();

    let first = solve_parity_game(&game);
    let second = solve_parity_game(&game);

    assert_eq!(first.even_winning_vertices(), vec![0, 2]);
    assert_eq!(first.odd_winning_vertices(), vec![1]);
    assert_eq!(
        first.even_strategy().winning_vertices(),
        first.even_winning_vertices()
    );
    assert_eq!(
        first.odd_strategy().winning_vertices(),
        first.odd_winning_vertices()
    );
    assert_eq!(first.even_strategy(), second.even_strategy());
    assert_eq!(first.odd_strategy(), second.odd_strategy());
    assert_eq!(first.even_strategy().choice(0), Some(2));
    assert_eq!(first.even_strategy().choice(2), Some(2));
    assert_eq!(first.odd_strategy().choice(1), Some(1));
    assert_eq!(verify_parity_strategy(&game, first.even_strategy()), Ok(()));
    assert_eq!(verify_parity_strategy(&game, first.odd_strategy()), Ok(()));
}

#[test]
fn verifier_fails_closed_on_missing_unexpected_invalid_and_out_of_region_choices() {
    let single = ParityGame::new(vec![ParityPlayer::Even], vec![0], vec![vec![0]]).unwrap();
    let missing = ParityStrategy::new(ParityPlayer::Even, vec![0], vec![None]);
    assert_eq!(
        verify_parity_strategy(&single, &missing),
        Err(ParityStrategyError::MissingChoice {
            vertex: 0,
            player: ParityPlayer::Even,
        })
    );

    let opponent_owned = ParityGame::new(vec![ParityPlayer::Odd], vec![0], vec![vec![0]]).unwrap();
    let unexpected = ParityStrategy::new(ParityPlayer::Even, vec![0], vec![Some(0)]);
    assert_eq!(
        verify_parity_strategy(&opponent_owned, &unexpected),
        Err(ParityStrategyError::UnexpectedChoice {
            vertex: 0,
            player: ParityPlayer::Even,
        })
    );

    let invalid_edge_game = ParityGame::new(
        vec![ParityPlayer::Even, ParityPlayer::Odd],
        vec![0, 1],
        vec![vec![0], vec![1]],
    )
    .unwrap();
    let invalid_edge = ParityStrategy::new(ParityPlayer::Even, vec![0, 1], vec![Some(1), None]);
    assert_eq!(
        verify_parity_strategy(&invalid_edge_game, &invalid_edge),
        Err(ParityStrategyError::InvalidChoiceEdge {
            vertex: 0,
            target: 1,
        })
    );

    let leaving_game = ParityGame::new(
        vec![ParityPlayer::Even, ParityPlayer::Odd],
        vec![0, 1],
        vec![vec![0, 1], vec![1]],
    )
    .unwrap();
    let leaves = ParityStrategy::new(ParityPlayer::Even, vec![0], vec![Some(1), None]);
    assert_eq!(
        verify_parity_strategy(&leaving_game, &leaves),
        Err(ParityStrategyError::ChoiceLeavesWinningRegion {
            vertex: 0,
            target: 1,
        })
    );

    let outside_choice = ParityStrategy::new(ParityPlayer::Odd, vec![1], vec![Some(0), Some(1)]);
    assert_eq!(
        verify_parity_strategy(&leaving_game, &outside_choice),
        Err(ParityStrategyError::ChoiceOutsideWinningRegion {
            vertex: 0,
            target: 0,
        })
    );
}

#[test]
fn verifier_rejects_opponent_escape_and_losing_recurrent_cycles_symmetrically() {
    let escape = ParityGame::new(
        vec![ParityPlayer::Odd, ParityPlayer::Even],
        vec![0, 1],
        vec![vec![0, 1], vec![1]],
    )
    .unwrap();
    let not_closed = ParityStrategy::new(ParityPlayer::Even, vec![0], vec![None, None]);
    assert_eq!(
        verify_parity_strategy(&escape, &not_closed),
        Err(ParityStrategyError::OpponentEdgeLeavesWinningRegion {
            vertex: 0,
            target: 1,
        })
    );

    let odd_cycle = ParityGame::new(vec![ParityPlayer::Even], vec![1], vec![vec![0]]).unwrap();
    let fake_even = ParityStrategy::new(ParityPlayer::Even, vec![0], vec![Some(0)]);
    assert_eq!(
        verify_parity_strategy(&odd_cycle, &fake_even),
        Err(ParityStrategyError::LosingCycle {
            player: ParityPlayer::Even,
            vertex: 0,
            priority: 1,
        })
    );

    let even_cycle = ParityGame::new(vec![ParityPlayer::Odd], vec![0], vec![vec![0]]).unwrap();
    let fake_odd = ParityStrategy::new(ParityPlayer::Odd, vec![0], vec![Some(0)]);
    assert_eq!(
        verify_parity_strategy(&even_cycle, &fake_odd),
        Err(ParityStrategyError::LosingCycle {
            player: ParityPlayer::Odd,
            vertex: 0,
            priority: 0,
        })
    );
}

#[test]
fn generated_two_vertex_games_certify_both_m81_winning_regions() {
    let successor_sets = [vec![0], vec![1], vec![0, 1]];
    let mut games = 0usize;

    for owner_mask in 0..4usize {
        let owners = vec![
            if owner_mask & 1 == 0 {
                ParityPlayer::Even
            } else {
                ParityPlayer::Odd
            },
            if owner_mask & 2 == 0 {
                ParityPlayer::Even
            } else {
                ParityPlayer::Odd
            },
        ];

        for first_priority in 0..=2usize {
            for second_priority in 0..=2usize {
                for first_edges in &successor_sets {
                    for second_edges in &successor_sets {
                        let game = ParityGame::new(
                            owners.clone(),
                            vec![first_priority, second_priority],
                            vec![first_edges.clone(), second_edges.clone()],
                        )
                        .unwrap();
                        let solved = solve_parity_game(&game);

                        assert_eq!(
                            solved.even_strategy().winning_vertices(),
                            solved.even_winning_vertices(),
                            "even strategy region mismatch owners={owners:?} priorities=({first_priority},{second_priority}) edges=({first_edges:?},{second_edges:?})"
                        );
                        assert_eq!(
                            solved.odd_strategy().winning_vertices(),
                            solved.odd_winning_vertices(),
                            "odd strategy region mismatch owners={owners:?} priorities=({first_priority},{second_priority}) edges=({first_edges:?},{second_edges:?})"
                        );
                        assert_eq!(
                            verify_parity_strategy(&game, solved.even_strategy()),
                            Ok(()),
                            "even certificate failed owners={owners:?} priorities=({first_priority},{second_priority}) edges=({first_edges:?},{second_edges:?})"
                        );
                        assert_eq!(
                            verify_parity_strategy(&game, solved.odd_strategy()),
                            Ok(()),
                            "odd certificate failed owners={owners:?} priorities=({first_priority},{second_priority}) edges=({first_edges:?},{second_edges:?})"
                        );

                        for vertex in 0..2 {
                            let even_choice = solved.even_strategy().choice(vertex);
                            let odd_choice = solved.odd_strategy().choice(vertex);
                            if solved.even_wins(vertex) && game.owner(vertex) == ParityPlayer::Even
                            {
                                assert!(even_choice.is_some());
                            } else {
                                assert!(even_choice.is_none());
                            }
                            if solved.odd_wins(vertex) && game.owner(vertex) == ParityPlayer::Odd {
                                assert!(odd_choice.is_some());
                            } else {
                                assert!(odd_choice.is_none());
                            }
                        }

                        games += 1;
                    }
                }
            }
        }
    }

    assert_eq!(games, 4 * 3 * 3 * 3 * 3);
    assert_eq!(games, 324);
}

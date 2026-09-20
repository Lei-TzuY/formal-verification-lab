use formal_verification_lab::{
    solve_parity_game, ParityGame, ParityGameError, ParityPlayer,
};

#[test]
fn self_loops_follow_priority_parity() {
    let even = ParityGame::new(
        vec![ParityPlayer::Even],
        vec![0],
        vec![vec![0]],
    )
    .unwrap();
    let even_solution = solve_parity_game(&even);
    assert!(even_solution.even_wins(0));
    assert!(!even_solution.odd_wins(0));

    let odd = ParityGame::new(
        vec![ParityPlayer::Even],
        vec![1],
        vec![vec![0]],
    )
    .unwrap();
    let odd_solution = solve_parity_game(&odd);
    assert!(!odd_solution.even_wins(0));
    assert!(odd_solution.odd_wins(0));
}

#[test]
fn owner_choice_and_attractor_are_respected() {
    let even_choice = ParityGame::new(
        vec![ParityPlayer::Even, ParityPlayer::Odd, ParityPlayer::Even],
        vec![0, 1, 2],
        vec![vec![1, 2], vec![1], vec![2]],
    )
    .unwrap();
    let solution = solve_parity_game(&even_choice);
    assert_eq!(solution.even_winning_vertices(), vec![0, 2]);
    assert_eq!(solution.odd_winning_vertices(), vec![1]);

    let odd_choice = ParityGame::new(
        vec![ParityPlayer::Odd, ParityPlayer::Odd, ParityPlayer::Even],
        vec![0, 1, 2],
        vec![vec![1, 2], vec![1], vec![2]],
    )
    .unwrap();
    let solution = solve_parity_game(&odd_choice);
    assert_eq!(solution.even_winning_vertices(), vec![2]);
    assert_eq!(solution.odd_winning_vertices(), vec![0, 1]);
}

#[test]
fn construction_fails_closed_on_non_total_or_invalid_games() {
    assert_eq!(
        ParityGame::new(Vec::new(), Vec::new(), Vec::new()).unwrap_err(),
        ParityGameError::EmptyGame
    );

    assert!(matches!(
        ParityGame::new(
            vec![ParityPlayer::Even],
            vec![0, 1],
            vec![vec![0]],
        )
        .unwrap_err(),
        ParityGameError::LengthMismatch { .. }
    ));

    assert_eq!(
        ParityGame::new(
            vec![ParityPlayer::Even],
            vec![0],
            vec![Vec::new()],
        )
        .unwrap_err(),
        ParityGameError::DeadEnd { vertex: 0 }
    );

    assert_eq!(
        ParityGame::new(
            vec![ParityPlayer::Even],
            vec![0],
            vec![vec![1]],
        )
        .unwrap_err(),
        ParityGameError::InvalidTarget {
            vertex: 0,
            target: 1,
            vertices: 1,
        }
    );
}

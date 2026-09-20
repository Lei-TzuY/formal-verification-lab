use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParityPlayer {
    Even,
    Odd,
}

impl ParityPlayer {
    fn opponent(self) -> Self {
        match self {
            Self::Even => Self::Odd,
            Self::Odd => Self::Even,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityGameError {
    EmptyGame,
    LengthMismatch {
        owners: usize,
        priorities: usize,
        edges: usize,
    },
    DeadEnd {
        vertex: usize,
    },
    InvalidTarget {
        vertex: usize,
        target: usize,
        vertices: usize,
    },
}

impl fmt::Display for ParityGameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGame => write!(f, "parity game must contain at least one vertex"),
            Self::LengthMismatch {
                owners,
                priorities,
                edges,
            } => write!(
                f,
                "parity game vectors disagree: owners={owners}, priorities={priorities}, edges={edges}"
            ),
            Self::DeadEnd { vertex } => {
                write!(f, "parity game vertex {vertex} has no successor")
            }
            Self::InvalidTarget {
                vertex,
                target,
                vertices,
            } => write!(
                f,
                "parity game edge {vertex}->{target} targets outside {vertices} vertices"
            ),
        }
    }
}

impl std::error::Error for ParityGameError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityGame {
    owners: Vec<ParityPlayer>,
    priorities: Vec<u32>,
    edges: Vec<Vec<usize>>,
}

impl ParityGame {
    pub fn new(
        owners: Vec<ParityPlayer>,
        priorities: Vec<u32>,
        edges: Vec<Vec<usize>>,
    ) -> Result<Self, ParityGameError> {
        let vertices = owners.len();
        if vertices == 0 {
            return Err(ParityGameError::EmptyGame);
        }
        if priorities.len() != vertices || edges.len() != vertices {
            return Err(ParityGameError::LengthMismatch {
                owners: vertices,
                priorities: priorities.len(),
                edges: edges.len(),
            });
        }

        for (vertex, successors) in edges.iter().enumerate() {
            if successors.is_empty() {
                return Err(ParityGameError::DeadEnd { vertex });
            }
            if let Some(&target) = successors.iter().find(|&&target| target >= vertices) {
                return Err(ParityGameError::InvalidTarget {
                    vertex,
                    target,
                    vertices,
                });
            }
        }

        Ok(Self {
            owners,
            priorities,
            edges,
        })
    }

    pub fn vertex_count(&self) -> usize {
        self.owners.len()
    }

    pub fn owner(&self, vertex: usize) -> ParityPlayer {
        self.owners[vertex]
    }

    pub fn priority(&self, vertex: usize) -> u32 {
        self.priorities[vertex]
    }

    pub fn successors(&self, vertex: usize) -> &[usize] {
        &self.edges[vertex]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParitySolution {
    even_winning: Vec<bool>,
    odd_winning: Vec<bool>,
}

impl ParitySolution {
    pub fn even_wins(&self, vertex: usize) -> bool {
        self.even_winning[vertex]
    }

    pub fn odd_wins(&self, vertex: usize) -> bool {
        self.odd_winning[vertex]
    }

    pub fn even_winning_vertices(&self) -> Vec<usize> {
        indices(&self.even_winning)
    }

    pub fn odd_winning_vertices(&self) -> Vec<usize> {
        indices(&self.odd_winning)
    }
}

pub fn solve_parity_game(game: &ParityGame) -> ParitySolution {
    let active = vec![true; game.vertex_count()];
    let (even_winning, odd_winning) = zielonka(game, &active);
    debug_assert!(
        even_winning
            .iter()
            .zip(&odd_winning)
            .all(|(even, odd)| *even ^ *odd),
        "validated total parity games partition into two winning regions"
    );
    ParitySolution {
        even_winning,
        odd_winning,
    }
}

fn zielonka(game: &ParityGame, active: &[bool]) -> (Vec<bool>, Vec<bool>) {
    if !active.iter().any(|present| *present) {
        return (
            vec![false; game.vertex_count()],
            vec![false; game.vertex_count()],
        );
    }

    let highest_priority = active
        .iter()
        .enumerate()
        .filter_map(|(vertex, present)| present.then_some(game.priority(vertex)))
        .max()
        .expect("non-empty active subgame has a priority");
    let player = if highest_priority % 2 == 0 {
        ParityPlayer::Even
    } else {
        ParityPlayer::Odd
    };

    let mut target = vec![false; game.vertex_count()];
    for vertex in 0..game.vertex_count() {
        target[vertex] = active[vertex] && game.priority(vertex) == highest_priority;
    }

    let player_attractor = attractor(game, active, &target, player);
    let remainder = subtract(active, &player_attractor);
    let (even_remainder, odd_remainder) = zielonka(game, &remainder);
    let opponent_has_region = match player.opponent() {
        ParityPlayer::Even => even_remainder.iter().any(|value| *value),
        ParityPlayer::Odd => odd_remainder.iter().any(|value| *value),
    };

    if !opponent_has_region {
        return match player {
            ParityPlayer::Even => (
                union(&even_remainder, &player_attractor),
                odd_remainder,
            ),
            ParityPlayer::Odd => (
                even_remainder,
                union(&odd_remainder, &player_attractor),
            ),
        };
    }

    let opponent_target = match player.opponent() {
        ParityPlayer::Even => &even_remainder,
        ParityPlayer::Odd => &odd_remainder,
    };
    let opponent_attractor = attractor(game, active, opponent_target, player.opponent());
    let second_remainder = subtract(active, &opponent_attractor);
    let (even_second, odd_second) = zielonka(game, &second_remainder);

    match player {
        ParityPlayer::Even => (
            even_second,
            union(&odd_second, &opponent_attractor),
        ),
        ParityPlayer::Odd => (
            union(&even_second, &opponent_attractor),
            odd_second,
        ),
    }
}

fn attractor(
    game: &ParityGame,
    active: &[bool],
    target: &[bool],
    player: ParityPlayer,
) -> Vec<bool> {
    let mut attracted = target.to_vec();

    loop {
        let mut changed = false;
        for vertex in 0..game.vertex_count() {
            if !active[vertex] || attracted[vertex] {
                continue;
            }

            let mut active_successors = game
                .successors(vertex)
                .iter()
                .copied()
                .filter(|&successor| active[successor]);
            let qualifies = if game.owner(vertex) == player {
                active_successors.any(|successor| attracted[successor])
            } else {
                active_successors.all(|successor| attracted[successor])
            };

            if qualifies {
                attracted[vertex] = true;
                changed = true;
            }
        }

        if !changed {
            return attracted;
        }
    }
}

fn subtract(left: &[bool], right: &[bool]) -> Vec<bool> {
    left.iter()
        .zip(right)
        .map(|(left, right)| *left && !*right)
        .collect()
}

fn union(left: &[bool], right: &[bool]) -> Vec<bool> {
    left.iter()
        .zip(right)
        .map(|(left, right)| *left || *right)
        .collect()
}

fn indices(values: &[bool]) -> Vec<usize> {
    values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect()
}

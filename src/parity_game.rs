use std::collections::VecDeque;
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

    fn owns_priority(self, priority: usize) -> bool {
        match self {
            Self::Even => priority.is_multiple_of(2),
            Self::Odd => !priority.is_multiple_of(2),
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
    priorities: Vec<usize>,
    edges: Vec<Vec<usize>>,
}

impl ParityGame {
    pub fn new(
        owners: Vec<ParityPlayer>,
        priorities: Vec<usize>,
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

    pub fn priority(&self, vertex: usize) -> usize {
        self.priorities[vertex]
    }

    pub fn successors(&self, vertex: usize) -> &[usize] {
        &self.edges[vertex]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityStrategy {
    player: ParityPlayer,
    winning_vertices: Vec<usize>,
    choices: Vec<Option<usize>>,
}

impl ParityStrategy {
    pub fn new(
        player: ParityPlayer,
        winning_vertices: Vec<usize>,
        choices: Vec<Option<usize>>,
    ) -> Self {
        Self {
            player,
            winning_vertices,
            choices,
        }
    }

    pub fn player(&self) -> ParityPlayer {
        self.player
    }

    pub fn winning_vertices(&self) -> &[usize] {
        &self.winning_vertices
    }

    pub fn choice(&self, vertex: usize) -> Option<usize> {
        self.choices.get(vertex).copied().flatten()
    }

    pub fn choices(&self) -> &[Option<usize>] {
        &self.choices
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityStrategyError {
    ChoiceLengthMismatch {
        choices: usize,
        vertices: usize,
    },
    InvalidWinningVertex {
        vertex: usize,
        vertices: usize,
    },
    DuplicateWinningVertex {
        vertex: usize,
    },
    ChoiceOutsideWinningRegion {
        vertex: usize,
        target: usize,
    },
    UnexpectedChoice {
        vertex: usize,
        player: ParityPlayer,
    },
    MissingChoice {
        vertex: usize,
        player: ParityPlayer,
    },
    InvalidChoiceEdge {
        vertex: usize,
        target: usize,
    },
    ChoiceLeavesWinningRegion {
        vertex: usize,
        target: usize,
    },
    OpponentEdgeLeavesWinningRegion {
        vertex: usize,
        target: usize,
    },
    LosingCycle {
        player: ParityPlayer,
        vertex: usize,
        priority: usize,
    },
}

impl fmt::Display for ParityStrategyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChoiceLengthMismatch { choices, vertices } => write!(
                f,
                "parity strategy has {choices} choice slots for {vertices} game vertices"
            ),
            Self::InvalidWinningVertex { vertex, vertices } => write!(
                f,
                "parity strategy winning vertex {vertex} is outside {vertices} game vertices"
            ),
            Self::DuplicateWinningVertex { vertex } => {
                write!(f, "parity strategy repeats winning vertex {vertex}")
            }
            Self::ChoiceOutsideWinningRegion { vertex, target } => write!(
                f,
                "parity strategy assigns {vertex}->{target} outside its declared winning region"
            ),
            Self::UnexpectedChoice { vertex, player } => write!(
                f,
                "parity strategy for {player:?} assigns a move at opponent-owned vertex {vertex}"
            ),
            Self::MissingChoice { vertex, player } => write!(
                f,
                "parity strategy for {player:?} is missing a move at owned winning vertex {vertex}"
            ),
            Self::InvalidChoiceEdge { vertex, target } => {
                write!(f, "parity strategy choice {vertex}->{target} is not a game edge")
            }
            Self::ChoiceLeavesWinningRegion { vertex, target } => write!(
                f,
                "parity strategy choice {vertex}->{target} leaves its declared winning region"
            ),
            Self::OpponentEdgeLeavesWinningRegion { vertex, target } => write!(
                f,
                "opponent edge {vertex}->{target} leaves the declared winning region"
            ),
            Self::LosingCycle {
                player,
                vertex,
                priority,
            } => write!(
                f,
                "strategy for {player:?} admits a recurrent cycle through vertex {vertex} with losing maximum priority {priority}"
            ),
        }
    }
}

impl std::error::Error for ParityStrategyError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParitySolution {
    even_winning: Vec<bool>,
    odd_winning: Vec<bool>,
    even_strategy: ParityStrategy,
    odd_strategy: ParityStrategy,
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

    pub fn even_strategy(&self) -> &ParityStrategy {
        &self.even_strategy
    }

    pub fn odd_strategy(&self) -> &ParityStrategy {
        &self.odd_strategy
    }
}

struct RecursiveSolution {
    even_winning: Vec<bool>,
    odd_winning: Vec<bool>,
    even_strategy: Vec<Option<usize>>,
    odd_strategy: Vec<Option<usize>>,
}

pub fn solve_parity_game(game: &ParityGame) -> ParitySolution {
    let active = vec![true; game.vertex_count()];
    let solved = zielonka(game, &active);
    debug_assert!(
        solved
            .even_winning
            .iter()
            .zip(&solved.odd_winning)
            .all(|(even, odd)| *even ^ *odd),
        "validated total parity games partition into two winning regions"
    );

    let even_strategy = ParityStrategy::new(
        ParityPlayer::Even,
        indices(&solved.even_winning),
        solved.even_strategy,
    );
    let odd_strategy = ParityStrategy::new(
        ParityPlayer::Odd,
        indices(&solved.odd_winning),
        solved.odd_strategy,
    );

    ParitySolution {
        even_winning: solved.even_winning,
        odd_winning: solved.odd_winning,
        even_strategy,
        odd_strategy,
    }
}

pub fn verify_parity_strategy(
    game: &ParityGame,
    strategy: &ParityStrategy,
) -> Result<(), ParityStrategyError> {
    let vertices = game.vertex_count();
    if strategy.choices.len() != vertices {
        return Err(ParityStrategyError::ChoiceLengthMismatch {
            choices: strategy.choices.len(),
            vertices,
        });
    }

    let mut region = vec![false; vertices];
    for &vertex in &strategy.winning_vertices {
        if vertex >= vertices {
            return Err(ParityStrategyError::InvalidWinningVertex { vertex, vertices });
        }
        if region[vertex] {
            return Err(ParityStrategyError::DuplicateWinningVertex { vertex });
        }
        region[vertex] = true;
    }

    for vertex in 0..vertices {
        let choice = strategy.choices[vertex];
        if !region[vertex] {
            if let Some(target) = choice {
                return Err(ParityStrategyError::ChoiceOutsideWinningRegion { vertex, target });
            }
            continue;
        }

        if game.owner(vertex) == strategy.player {
            let target = choice.ok_or(ParityStrategyError::MissingChoice {
                vertex,
                player: strategy.player,
            })?;
            if !game.successors(vertex).contains(&target) {
                return Err(ParityStrategyError::InvalidChoiceEdge { vertex, target });
            }
            if !region[target] {
                return Err(ParityStrategyError::ChoiceLeavesWinningRegion { vertex, target });
            }
        } else {
            if choice.is_some() {
                return Err(ParityStrategyError::UnexpectedChoice {
                    vertex,
                    player: strategy.player,
                });
            }
            for &target in game.successors(vertex) {
                if !region[target] {
                    return Err(ParityStrategyError::OpponentEdgeLeavesWinningRegion {
                        vertex,
                        target,
                    });
                }
            }
        }
    }

    verify_strategy_parity(game, strategy, &region)
}

fn zielonka(game: &ParityGame, active: &[bool]) -> RecursiveSolution {
    if !active.iter().any(|present| *present) {
        return RecursiveSolution {
            even_winning: vec![false; game.vertex_count()],
            odd_winning: vec![false; game.vertex_count()],
            even_strategy: vec![None; game.vertex_count()],
            odd_strategy: vec![None; game.vertex_count()],
        };
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

    let (player_attractor, attractor_strategy) = attractor(game, active, &target, player);
    let remainder = subtract(active, &player_attractor);
    let first = zielonka(game, &remainder);
    let opponent = player.opponent();
    let opponent_region = match opponent {
        ParityPlayer::Even => first.even_winning.clone(),
        ParityPlayer::Odd => first.odd_winning.clone(),
    };

    if !opponent_region.iter().any(|value| *value) {
        return finish_player_dominates(
            game,
            active,
            &target,
            player,
            player_attractor,
            attractor_strategy,
            first,
        );
    }

    let (opponent_attractor, opponent_attractor_strategy) =
        attractor(game, active, &opponent_region, opponent);
    let second_remainder = subtract(active, &opponent_attractor);
    let second = zielonka(game, &second_remainder);

    finish_opponent_recovers(
        opponent,
        &opponent_region,
        opponent_attractor,
        opponent_attractor_strategy,
        first,
        second,
    )
}

fn finish_player_dominates(
    game: &ParityGame,
    active: &[bool],
    target: &[bool],
    player: ParityPlayer,
    player_attractor: Vec<bool>,
    attractor_strategy: Vec<Option<usize>>,
    first: RecursiveSolution,
) -> RecursiveSolution {
    let RecursiveSolution {
        even_winning,
        odd_winning,
        even_strategy,
        odd_strategy,
    } = first;

    match player {
        ParityPlayer::Even => {
            let winning = union(&even_winning, &player_attractor);
            let mut strategy = even_strategy;
            overlay_strategy(&mut strategy, &attractor_strategy);
            fill_highest_priority_choices(
                game,
                active,
                target,
                &player_attractor,
                &winning,
                player,
                &mut strategy,
            );
            RecursiveSolution {
                even_winning: winning,
                odd_winning,
                even_strategy: strategy,
                odd_strategy,
            }
        }
        ParityPlayer::Odd => {
            let winning = union(&odd_winning, &player_attractor);
            let mut strategy = odd_strategy;
            overlay_strategy(&mut strategy, &attractor_strategy);
            fill_highest_priority_choices(
                game,
                active,
                target,
                &player_attractor,
                &winning,
                player,
                &mut strategy,
            );
            RecursiveSolution {
                even_winning,
                odd_winning: winning,
                even_strategy,
                odd_strategy: strategy,
            }
        }
    }
}

fn finish_opponent_recovers(
    opponent: ParityPlayer,
    opponent_region: &[bool],
    opponent_attractor: Vec<bool>,
    opponent_attractor_strategy: Vec<Option<usize>>,
    first: RecursiveSolution,
    second: RecursiveSolution,
) -> RecursiveSolution {
    match opponent {
        ParityPlayer::Even => {
            let mut strategy = second.even_strategy;
            overlay_strategy(&mut strategy, &opponent_attractor_strategy);
            overlay_region_strategy(&mut strategy, &first.even_strategy, opponent_region);
            RecursiveSolution {
                even_winning: union(&second.even_winning, &opponent_attractor),
                odd_winning: second.odd_winning,
                even_strategy: strategy,
                odd_strategy: second.odd_strategy,
            }
        }
        ParityPlayer::Odd => {
            let mut strategy = second.odd_strategy;
            overlay_strategy(&mut strategy, &opponent_attractor_strategy);
            overlay_region_strategy(&mut strategy, &first.odd_strategy, opponent_region);
            RecursiveSolution {
                even_winning: second.even_winning,
                odd_winning: union(&second.odd_winning, &opponent_attractor),
                even_strategy: second.even_strategy,
                odd_strategy: strategy,
            }
        }
    }
}

fn attractor(
    game: &ParityGame,
    active: &[bool],
    target: &[bool],
    player: ParityPlayer,
) -> (Vec<bool>, Vec<Option<usize>>) {
    let mut attracted = target.to_vec();
    let mut strategy = vec![None; game.vertex_count()];

    loop {
        let mut changed = false;
        for vertex in 0..game.vertex_count() {
            if !active[vertex] || attracted[vertex] {
                continue;
            }

            if game.owner(vertex) == player {
                if let Some(target) = game
                    .successors(vertex)
                    .iter()
                    .copied()
                    .filter(|&successor| active[successor])
                    .find(|&successor| attracted[successor])
                {
                    attracted[vertex] = true;
                    strategy[vertex] = Some(target);
                    changed = true;
                }
            } else {
                let mut saw_active_successor = false;
                let all_attracted = game
                    .successors(vertex)
                    .iter()
                    .copied()
                    .filter(|&successor| active[successor])
                    .inspect(|_| saw_active_successor = true)
                    .all(|successor| attracted[successor]);
                debug_assert!(
                    saw_active_successor,
                    "Zielonka attractor complements remain total subgames"
                );
                if saw_active_successor && all_attracted {
                    attracted[vertex] = true;
                    changed = true;
                }
            }
        }

        if !changed {
            return (attracted, strategy);
        }
    }
}

fn fill_highest_priority_choices(
    game: &ParityGame,
    active: &[bool],
    target: &[bool],
    attractor: &[bool],
    winning: &[bool],
    player: ParityPlayer,
    strategy: &mut [Option<usize>],
) {
    for vertex in 0..game.vertex_count() {
        if !target[vertex] || game.owner(vertex) != player || strategy[vertex].is_some() {
            continue;
        }

        let active_successors = game
            .successors(vertex)
            .iter()
            .copied()
            .filter(|&successor| active[successor])
            .collect::<Vec<_>>();
        let choice = active_successors
            .iter()
            .copied()
            .find(|&successor| attractor[successor])
            .or_else(|| {
                active_successors
                    .iter()
                    .copied()
                    .find(|&successor| winning[successor])
            })
            .expect("winning highest-priority owned vertex has a winning active successor");
        strategy[vertex] = Some(choice);
    }
}

fn overlay_strategy(target: &mut [Option<usize>], source: &[Option<usize>]) {
    for (target, source) in target.iter_mut().zip(source) {
        if source.is_some() {
            *target = *source;
        }
    }
}

fn overlay_region_strategy(
    target: &mut [Option<usize>],
    source: &[Option<usize>],
    region: &[bool],
) {
    for vertex in 0..target.len() {
        if region[vertex] && source[vertex].is_some() {
            target[vertex] = source[vertex];
        }
    }
}

fn verify_strategy_parity(
    game: &ParityGame,
    strategy: &ParityStrategy,
    region: &[bool],
) -> Result<(), ParityStrategyError> {
    for vertex in 0..game.vertex_count() {
        if !region[vertex] || strategy.player.owns_priority(game.priority(vertex)) {
            continue;
        }

        let priority = game.priority(vertex);
        let allowed = (0..game.vertex_count())
            .map(|candidate| region[candidate] && game.priority(candidate) <= priority)
            .collect::<Vec<_>>();

        for successor in restricted_successors(game, strategy, vertex) {
            if allowed[successor] && can_reach_within(game, strategy, successor, vertex, &allowed) {
                return Err(ParityStrategyError::LosingCycle {
                    player: strategy.player,
                    vertex,
                    priority,
                });
            }
        }
    }

    Ok(())
}

fn can_reach_within(
    game: &ParityGame,
    strategy: &ParityStrategy,
    start: usize,
    goal: usize,
    allowed: &[bool],
) -> bool {
    let mut queue = VecDeque::from([start]);
    let mut seen = vec![false; game.vertex_count()];

    while let Some(vertex) = queue.pop_front() {
        if vertex == goal {
            return true;
        }
        if seen[vertex] {
            continue;
        }
        seen[vertex] = true;

        for successor in restricted_successors(game, strategy, vertex) {
            if allowed[successor] && !seen[successor] {
                queue.push_back(successor);
            }
        }
    }

    false
}

fn restricted_successors(
    game: &ParityGame,
    strategy: &ParityStrategy,
    vertex: usize,
) -> Vec<usize> {
    if game.owner(vertex) == strategy.player {
        vec![strategy.choices[vertex]
            .expect("verified owned winning vertices have a strategy choice")]
    } else {
        game.successors(vertex).to_vec()
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

use crate::graph::{capture_reachable_graph, GraphCaptureError, ReachableGraph};
use crate::model::{ModelError, TransitionSystem};
use crate::mu_calculus::{
    validate_mu_formula, MuFormula, MuInitialEvaluation, MuTerminalPolicy, MuValidationError,
};
use crate::parity_game::{
    solve_parity_game, verify_parity_strategy, ParityGame, ParityGameError, ParityPlayer,
    ParityStrategy, ParityStrategyError,
};
use std::fmt;
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuParityFixpointKind {
    Mu,
    Nu,
}

impl MuParityFixpointKind {
    fn dual(self) -> Self {
        match self {
            Self::Mu => Self::Nu,
            Self::Nu => Self::Mu,
        }
    }

    fn priority(self, alternation_level: usize, max_alternation_level: usize) -> usize {
        let outer_rank = max_alternation_level - alternation_level;
        match self {
            Self::Mu => outer_rank * 2 + 1,
            Self::Nu => outer_rank * 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuParityPositionKind {
    True,
    False,
    Literal {
        positive: bool,
    },
    Variable {
        binder_formula_node: usize,
    },
    And,
    Or,
    Diamond,
    Box,
    Fixpoint {
        kind: MuParityFixpointKind,
        alternation_level: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityPosition {
    pub state_index: usize,
    pub formula_node: usize,
    pub kind: MuParityPositionKind,
    pub owner: ParityPlayer,
    pub priority: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuParityMove {
    OutcomeSelfLoop,
    BooleanLeft,
    BooleanRight,
    ModalSuccessor { target_state_index: usize },
    ModalTerminalSelfLoop,
    FixpointBody,
    VariableReturn { binder_formula_node: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityStrategyChoice {
    pub from: MuParityPosition,
    pub to: MuParityPosition,
    pub semantic_move: MuParityMove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityStrategyEvidence {
    pub player: ParityPlayer,
    pub winning_positions: Vec<MuParityPosition>,
    pub choices: Vec<MuParityStrategyChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityInitialEvidence {
    pub state_index: usize,
    pub satisfied: bool,
    pub winner: ParityPlayer,
    pub root_position: MuParityPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuParityEvaluation<S> {
    pub terminal_policy: MuTerminalPolicy,
    pub reachable_states: Vec<S>,
    pub satisfying_state_indices: Vec<usize>,
    pub initial: Vec<MuInitialEvaluation<S>>,
    pub discovered_states: usize,
    pub explored_transitions: usize,
    pub max_depth_reached: Option<usize>,
    pub parity_game_vertices: usize,
    pub max_priority: usize,
    pub positions: Vec<MuParityPosition>,
    pub even_strategy: MuParityStrategyEvidence,
    pub odd_strategy: MuParityStrategyEvidence,
    pub initial_evidence: Vec<MuParityInitialEvidence>,
}

impl<S> MuParityEvaluation<S> {
    pub fn all_initial_states_satisfy(&self) -> bool {
        self.initial.iter().all(|entry| entry.satisfied)
    }

    pub fn any_initial_state_satisfies(&self) -> bool {
        self.initial.iter().any(|entry| entry.satisfied)
    }
}

#[derive(Debug)]
pub enum MuParityError<V> {
    Validation(MuValidationError<V>),
    Model(ModelError),
    SnapshotInvariant,
    GameSizeOverflow,
    Game(ParityGameError),
    Strategy {
        player: ParityPlayer,
        error: ParityStrategyError,
    },
}

impl<V: fmt::Debug> fmt::Display for MuParityError<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(f, "{error}"),
            Self::Model(error) => write!(f, "mu-calculus parity model capture failed: {error}"),
            Self::SnapshotInvariant => {
                write!(
                    f,
                    "mu-calculus parity reachable-graph snapshot invariant failed"
                )
            }
            Self::GameSizeOverflow => write!(f, "mu-calculus parity evaluation game is too large"),
            Self::Game(error) => write!(f, "mu-calculus parity game construction failed: {error}"),
            Self::Strategy { player, error } => {
                write!(
                    f,
                    "mu-calculus parity {player:?} strategy verification failed: {error}"
                )
            }
        }
    }
}

impl<V: fmt::Debug> std::error::Error for MuParityError<V> {}

impl<V> From<MuValidationError<V>> for MuParityError<V> {
    fn from(value: MuValidationError<V>) -> Self {
        Self::Validation(value)
    }
}

impl<V> From<ParityGameError> for MuParityError<V> {
    fn from(value: ParityGameError) -> Self {
        Self::Game(value)
    }
}

#[derive(Debug)]
pub enum MuParityEvidenceError<V> {
    Validation(MuValidationError<V>),
    Model(ModelError),
    SnapshotInvariant,
    GameSizeOverflow,
    Game(ParityGameError),
    Strategy {
        player: ParityPlayer,
        error: ParityStrategyError,
    },
    EvaluationMismatch,
    PositionCountMismatch {
        expected: usize,
        actual: usize,
    },
    PositionMismatch {
        vertex: usize,
    },
    EvidencePlayerMismatch {
        expected: ParityPlayer,
        actual: ParityPlayer,
    },
    DuplicateChoice {
        player: ParityPlayer,
        vertex: usize,
    },
    SemanticMoveMismatch {
        player: ParityPlayer,
        vertex: usize,
    },
    WinningPartitionMismatch,
    InitialEvidenceMismatch {
        index: usize,
    },
}

impl<V: fmt::Debug> fmt::Display for MuParityEvidenceError<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(f, "{error}"),
            Self::Model(error) => write!(f, "mu-calculus parity evidence model capture failed: {error}"),
            Self::SnapshotInvariant => {
                write!(f, "mu-calculus parity evidence reachable-graph snapshot invariant failed")
            }
            Self::GameSizeOverflow => write!(f, "mu-calculus parity evidence game is too large"),
            Self::Game(error) => write!(f, "mu-calculus parity evidence game construction failed: {error}"),
            Self::Strategy { player, error } => {
                write!(f, "mu-calculus parity {player:?} strategy evidence failed: {error}")
            }
            Self::EvaluationMismatch => {
                write!(f, "mu-calculus parity evidence does not match the canonical evaluation")
            }
            Self::PositionCountMismatch { expected, actual } => write!(
                f,
                "mu-calculus parity evidence has {actual} positions; canonical game has {expected}"
            ),
            Self::PositionMismatch { vertex } => {
                write!(f, "mu-calculus parity evidence position {vertex} is not canonical")
            }
            Self::EvidencePlayerMismatch { expected, actual } => write!(
                f,
                "mu-calculus parity evidence claims player {actual:?}; expected {expected:?}"
            ),
            Self::DuplicateChoice { player, vertex } => write!(
                f,
                "mu-calculus parity {player:?} evidence repeats a choice for vertex {vertex}"
            ),
            Self::SemanticMoveMismatch { player, vertex } => write!(
                f,
                "mu-calculus parity {player:?} evidence has a non-canonical semantic move at vertex {vertex}"
            ),
            Self::WinningPartitionMismatch => {
                write!(f, "mu-calculus parity evidence winning regions do not partition the game")
            }
            Self::InitialEvidenceMismatch { index } => write!(
                f,
                "mu-calculus parity initial evidence entry {index} does not match the canonical root position"
            ),
        }
    }
}

impl<V: fmt::Debug> std::error::Error for MuParityEvidenceError<V> {}

impl<V> From<MuValidationError<V>> for MuParityEvidenceError<V> {
    fn from(value: MuValidationError<V>) -> Self {
        Self::Validation(value)
    }
}

#[derive(Debug, Clone)]
enum EvalNode<A> {
    True,
    False,
    Literal {
        atom: A,
        positive: bool,
    },
    Var {
        binder: usize,
    },
    And {
        left: usize,
        right: usize,
    },
    Or {
        left: usize,
        right: usize,
    },
    Diamond {
        inner: usize,
    },
    Box {
        inner: usize,
    },
    Fix {
        body: usize,
        kind: MuParityFixpointKind,
        alternation_level: usize,
    },
}

pub fn evaluate_mu_via_parity<S, A, V, F>(
    model: &TransitionSystem<S>,
    formula: &MuFormula<A, V>,
    atom_holds: F,
) -> Result<MuParityEvaluation<S>, MuParityError<V>>
where
    S: Clone + Eq + Hash,
    A: Clone,
    V: Clone + Eq,
    F: Fn(&A, &S) -> bool,
{
    validate_mu_formula(formula)?;
    let captured = capture_reachable_graph(model).map_err(map_capture_error)?;

    let mut nodes = Vec::new();
    let mut bindings = Vec::new();
    let root = lower_formula(formula, true, &mut bindings, None, &mut nodes);
    debug_assert!(bindings.is_empty());

    let max_alternation_level = max_alternation_level(&nodes);
    let game = build_evaluation_game(&captured.graph, &nodes, max_alternation_level, &atom_holds)?;
    let solution = solve_parity_game(&game);
    verify_parity_strategy(&game, solution.even_strategy()).map_err(|error| {
        MuParityError::Strategy {
            player: ParityPlayer::Even,
            error,
        }
    })?;
    verify_parity_strategy(&game, solution.odd_strategy()).map_err(|error| {
        MuParityError::Strategy {
            player: ParityPlayer::Odd,
            error,
        }
    })?;
    let node_count = nodes.len();
    let positions = describe_positions(&game, &nodes, captured.graph.states.len());
    let even_strategy = describe_strategy(
        &captured.graph,
        &nodes,
        node_count,
        solution.even_strategy(),
        &positions,
    );
    let odd_strategy = describe_strategy(
        &captured.graph,
        &nodes,
        node_count,
        solution.odd_strategy(),
        &positions,
    );

    let satisfying_state_indices = (0..captured.graph.states.len())
        .filter(|&state| solution.even_wins(position(state, root, node_count)))
        .collect::<Vec<_>>();
    let initial = captured
        .graph
        .initial_ids
        .iter()
        .map(|&state_index| MuInitialEvaluation {
            state_index,
            state: captured.graph.states[state_index].clone(),
            satisfied: solution.even_wins(position(state_index, root, node_count)),
        })
        .collect::<Vec<_>>();
    let max_priority = max_fixpoint_priority(&nodes, max_alternation_level);
    let initial_evidence = captured
        .graph
        .initial_ids
        .iter()
        .map(|&state_index| {
            let root_vertex = position(state_index, root, node_count);
            let satisfied = solution.even_wins(root_vertex);
            MuParityInitialEvidence {
                state_index,
                satisfied,
                winner: if satisfied {
                    ParityPlayer::Even
                } else {
                    ParityPlayer::Odd
                },
                root_position: positions[root_vertex].clone(),
            }
        })
        .collect();

    Ok(MuParityEvaluation {
        terminal_policy: MuTerminalPolicy::TotalizeWithSelfLoop,
        reachable_states: captured.graph.states,
        satisfying_state_indices,
        initial,
        discovered_states: captured.discovered_states,
        explored_transitions: captured.explored_transitions,
        max_depth_reached: captured.max_depth_reached,
        parity_game_vertices: game.vertex_count(),
        max_priority,
        positions,
        even_strategy,
        odd_strategy,
        initial_evidence,
    })
}

pub fn verify_mu_parity_evidence<S, A, V, F>(
    model: &TransitionSystem<S>,
    formula: &MuFormula<A, V>,
    atom_holds: F,
    evaluation: &MuParityEvaluation<S>,
) -> Result<(), MuParityEvidenceError<V>>
where
    S: Clone + Eq + Hash,
    A: Clone,
    V: Clone + Eq,
    F: Fn(&A, &S) -> bool,
{
    validate_mu_formula(formula)?;
    let captured = capture_reachable_graph(model).map_err(map_evidence_capture_error)?;

    let mut nodes = Vec::new();
    let mut bindings = Vec::new();
    let root = lower_formula(formula, true, &mut bindings, None, &mut nodes);
    debug_assert!(bindings.is_empty());
    let max_alternation_level = max_alternation_level(&nodes);
    let game = build_evaluation_game(&captured.graph, &nodes, max_alternation_level, &atom_holds)
        .map_err(map_evidence_game_error)?;
    let node_count = nodes.len();
    let canonical_positions = describe_positions(&game, &nodes, captured.graph.states.len());

    if evaluation.terminal_policy != MuTerminalPolicy::TotalizeWithSelfLoop
        || evaluation.reachable_states != captured.graph.states
        || evaluation.discovered_states != captured.discovered_states
        || evaluation.explored_transitions != captured.explored_transitions
        || evaluation.max_depth_reached != captured.max_depth_reached
        || evaluation.parity_game_vertices != game.vertex_count()
        || evaluation.max_priority != max_fixpoint_priority(&nodes, max_alternation_level)
    {
        return Err(MuParityEvidenceError::EvaluationMismatch);
    }
    if evaluation.positions.len() != canonical_positions.len() {
        return Err(MuParityEvidenceError::PositionCountMismatch {
            expected: canonical_positions.len(),
            actual: evaluation.positions.len(),
        });
    }
    for (vertex, (actual, expected)) in evaluation
        .positions
        .iter()
        .zip(&canonical_positions)
        .enumerate()
    {
        if actual != expected {
            return Err(MuParityEvidenceError::PositionMismatch { vertex });
        }
    }

    let even = reconstruct_strategy(
        ParityPlayer::Even,
        &evaluation.even_strategy,
        &canonical_positions,
        &game,
        &captured.graph,
        &nodes,
        node_count,
    )?;
    let odd = reconstruct_strategy(
        ParityPlayer::Odd,
        &evaluation.odd_strategy,
        &canonical_positions,
        &game,
        &captured.graph,
        &nodes,
        node_count,
    )?;

    verify_parity_strategy(&game, &even).map_err(|error| MuParityEvidenceError::Strategy {
        player: ParityPlayer::Even,
        error,
    })?;
    verify_parity_strategy(&game, &odd).map_err(|error| MuParityEvidenceError::Strategy {
        player: ParityPlayer::Odd,
        error,
    })?;

    let mut even_mask = vec![false; game.vertex_count()];
    for &vertex in even.winning_vertices() {
        even_mask[vertex] = true;
    }
    let mut odd_mask = vec![false; game.vertex_count()];
    for &vertex in odd.winning_vertices() {
        odd_mask[vertex] = true;
    }
    if even_mask
        .iter()
        .zip(&odd_mask)
        .any(|(even, odd)| *even == *odd)
    {
        return Err(MuParityEvidenceError::WinningPartitionMismatch);
    }

    let expected_satisfying = (0..captured.graph.states.len())
        .filter(|&state| even_mask[position(state, root, node_count)])
        .collect::<Vec<_>>();
    if evaluation.satisfying_state_indices != expected_satisfying
        || evaluation.initial.len() != captured.graph.initial_ids.len()
        || evaluation.initial_evidence.len() != captured.graph.initial_ids.len()
    {
        return Err(MuParityEvidenceError::EvaluationMismatch);
    }

    for (index, &state_index) in captured.graph.initial_ids.iter().enumerate() {
        let root_vertex = position(state_index, root, node_count);
        let satisfied = even_mask[root_vertex];
        let initial = &evaluation.initial[index];
        let evidence = &evaluation.initial_evidence[index];
        if initial.state_index != state_index
            || initial.state != captured.graph.states[state_index]
            || initial.satisfied != satisfied
            || evidence.state_index != state_index
            || evidence.satisfied != satisfied
            || evidence.winner
                != if satisfied {
                    ParityPlayer::Even
                } else {
                    ParityPlayer::Odd
                }
            || evidence.root_position != canonical_positions[root_vertex]
        {
            return Err(MuParityEvidenceError::InitialEvidenceMismatch { index });
        }
    }

    Ok(())
}

fn map_evidence_capture_error<V>(error: GraphCaptureError) -> MuParityEvidenceError<V> {
    match error {
        GraphCaptureError::Model(error) => MuParityEvidenceError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            MuParityEvidenceError::SnapshotInvariant
        }
    }
}

fn map_evidence_game_error<V>(error: MuParityError<V>) -> MuParityEvidenceError<V> {
    match error {
        MuParityError::Validation(error) => MuParityEvidenceError::Validation(error),
        MuParityError::Model(error) => MuParityEvidenceError::Model(error),
        MuParityError::SnapshotInvariant => MuParityEvidenceError::SnapshotInvariant,
        MuParityError::GameSizeOverflow => MuParityEvidenceError::GameSizeOverflow,
        MuParityError::Game(error) => MuParityEvidenceError::Game(error),
        MuParityError::Strategy { player, error } => {
            MuParityEvidenceError::Strategy { player, error }
        }
    }
}

fn map_capture_error<V>(error: GraphCaptureError) -> MuParityError<V> {
    match error {
        GraphCaptureError::Model(error) => MuParityError::Model(error),
        GraphCaptureError::UnexpectedInconclusive | GraphCaptureError::SnapshotTargetMissing => {
            MuParityError::SnapshotInvariant
        }
    }
}

fn lower_formula<A, V>(
    formula: &MuFormula<A, V>,
    positive: bool,
    bindings: &mut Vec<(V, usize)>,
    parent_fixpoint: Option<(MuParityFixpointKind, usize)>,
    nodes: &mut Vec<EvalNode<A>>,
) -> usize
where
    A: Clone,
    V: Clone + Eq,
{
    match formula {
        MuFormula::True => push_node(
            nodes,
            if positive {
                EvalNode::True
            } else {
                EvalNode::False
            },
        ),
        MuFormula::False => push_node(
            nodes,
            if positive {
                EvalNode::False
            } else {
                EvalNode::True
            },
        ),
        MuFormula::Atom(atom) => push_node(
            nodes,
            EvalNode::Literal {
                atom: atom.clone(),
                positive,
            },
        ),
        MuFormula::Var(variable) => {
            let binder = bindings
                .iter()
                .rev()
                .find(|(bound, _)| bound == variable)
                .map(|(_, binder)| *binder)
                .expect("validated mu-calculus variables are lexically bound");
            push_node(nodes, EvalNode::Var { binder })
        }
        MuFormula::Not(inner) => lower_formula(inner, !positive, bindings, parent_fixpoint, nodes),
        MuFormula::And(left, right) => {
            let left = lower_formula(left, positive, bindings, parent_fixpoint, nodes);
            let right = lower_formula(right, positive, bindings, parent_fixpoint, nodes);
            if positive {
                push_node(nodes, EvalNode::And { left, right })
            } else {
                push_node(nodes, EvalNode::Or { left, right })
            }
        }
        MuFormula::Or(left, right) => {
            let left = lower_formula(left, positive, bindings, parent_fixpoint, nodes);
            let right = lower_formula(right, positive, bindings, parent_fixpoint, nodes);
            if positive {
                push_node(nodes, EvalNode::Or { left, right })
            } else {
                push_node(nodes, EvalNode::And { left, right })
            }
        }
        MuFormula::Diamond(inner) => {
            let inner = lower_formula(inner, positive, bindings, parent_fixpoint, nodes);
            if positive {
                push_node(nodes, EvalNode::Diamond { inner })
            } else {
                push_node(nodes, EvalNode::Box { inner })
            }
        }
        MuFormula::Box(inner) => {
            let inner = lower_formula(inner, positive, bindings, parent_fixpoint, nodes);
            if positive {
                push_node(nodes, EvalNode::Box { inner })
            } else {
                push_node(nodes, EvalNode::Diamond { inner })
            }
        }
        MuFormula::Mu { variable, body } => lower_fixpoint(
            MuParityFixpointKind::Mu,
            variable,
            body,
            positive,
            bindings,
            parent_fixpoint,
            nodes,
        ),
        MuFormula::Nu { variable, body } => lower_fixpoint(
            MuParityFixpointKind::Nu,
            variable,
            body,
            positive,
            bindings,
            parent_fixpoint,
            nodes,
        ),
    }
}

fn lower_fixpoint<A, V>(
    original_kind: MuParityFixpointKind,
    variable: &V,
    body: &MuFormula<A, V>,
    positive: bool,
    bindings: &mut Vec<(V, usize)>,
    parent_fixpoint: Option<(MuParityFixpointKind, usize)>,
    nodes: &mut Vec<EvalNode<A>>,
) -> usize
where
    A: Clone,
    V: Clone + Eq,
{
    let kind = if positive {
        original_kind
    } else {
        original_kind.dual()
    };
    let alternation_level = match parent_fixpoint {
        Some((parent_kind, level)) if parent_kind != kind => level + 1,
        Some((_, level)) => level,
        None => 0,
    };
    let binder = nodes.len();
    nodes.push(EvalNode::True);
    bindings.push((variable.clone(), binder));
    let body = lower_formula(
        body,
        positive,
        bindings,
        Some((kind, alternation_level)),
        nodes,
    );
    bindings.pop();
    nodes[binder] = EvalNode::Fix {
        body,
        kind,
        alternation_level,
    };
    binder
}

fn push_node<A>(nodes: &mut Vec<EvalNode<A>>, node: EvalNode<A>) -> usize {
    let index = nodes.len();
    nodes.push(node);
    index
}

fn max_alternation_level<A>(nodes: &[EvalNode<A>]) -> usize {
    nodes
        .iter()
        .filter_map(|node| match node {
            EvalNode::Fix {
                alternation_level, ..
            } => Some(*alternation_level),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn max_fixpoint_priority<A>(nodes: &[EvalNode<A>], max_alternation_level: usize) -> usize {
    nodes
        .iter()
        .filter_map(|node| match node {
            EvalNode::Fix {
                kind,
                alternation_level,
                ..
            } => Some(kind.priority(*alternation_level, max_alternation_level)),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn describe_positions<A>(
    game: &ParityGame,
    nodes: &[EvalNode<A>],
    state_count: usize,
) -> Vec<MuParityPosition> {
    let node_count = nodes.len();
    let mut positions = Vec::with_capacity(game.vertex_count());
    for state_index in 0..state_count {
        for (formula_node, node) in nodes.iter().enumerate() {
            let vertex = position(state_index, formula_node, node_count);
            positions.push(MuParityPosition {
                state_index,
                formula_node,
                kind: position_kind(node),
                owner: game.owner(vertex),
                priority: game.priority(vertex),
            });
        }
    }
    positions
}

fn position_kind<A>(node: &EvalNode<A>) -> MuParityPositionKind {
    match node {
        EvalNode::True => MuParityPositionKind::True,
        EvalNode::False => MuParityPositionKind::False,
        EvalNode::Literal { positive, .. } => MuParityPositionKind::Literal {
            positive: *positive,
        },
        EvalNode::Var { binder } => MuParityPositionKind::Variable {
            binder_formula_node: *binder,
        },
        EvalNode::And { .. } => MuParityPositionKind::And,
        EvalNode::Or { .. } => MuParityPositionKind::Or,
        EvalNode::Diamond { .. } => MuParityPositionKind::Diamond,
        EvalNode::Box { .. } => MuParityPositionKind::Box,
        EvalNode::Fix {
            kind,
            alternation_level,
            ..
        } => MuParityPositionKind::Fixpoint {
            kind: *kind,
            alternation_level: *alternation_level,
        },
    }
}

fn describe_strategy<S, A>(
    graph: &ReachableGraph<S>,
    nodes: &[EvalNode<A>],
    node_count: usize,
    strategy: &ParityStrategy,
    positions: &[MuParityPosition],
) -> MuParityStrategyEvidence {
    let winning_positions = strategy
        .winning_vertices()
        .iter()
        .map(|&vertex| positions[vertex].clone())
        .collect();
    let choices = strategy
        .choices()
        .iter()
        .enumerate()
        .filter_map(|(vertex, target)| {
            target.map(|target| MuParityStrategyChoice {
                from: positions[vertex].clone(),
                to: positions[target].clone(),
                semantic_move: classify_semantic_move(graph, nodes, node_count, vertex, target)
                    .expect("solver strategies only select canonical evaluation-game edges"),
            })
        })
        .collect();

    MuParityStrategyEvidence {
        player: strategy.player(),
        winning_positions,
        choices,
    }
}

fn reconstruct_strategy<S, A, V>(
    expected_player: ParityPlayer,
    evidence: &MuParityStrategyEvidence,
    positions: &[MuParityPosition],
    game: &ParityGame,
    graph: &ReachableGraph<S>,
    nodes: &[EvalNode<A>],
    node_count: usize,
) -> Result<ParityStrategy, MuParityEvidenceError<V>> {
    if evidence.player != expected_player {
        return Err(MuParityEvidenceError::EvidencePlayerMismatch {
            expected: expected_player,
            actual: evidence.player,
        });
    }

    let mut winning_vertices = Vec::with_capacity(evidence.winning_positions.len());
    for position in &evidence.winning_positions {
        winning_vertices.push(canonical_vertex(position, positions, node_count)?);
    }

    let mut choices = vec![None; game.vertex_count()];
    for choice in &evidence.choices {
        let vertex = canonical_vertex(&choice.from, positions, node_count)?;
        let target = canonical_vertex(&choice.to, positions, node_count)?;
        if choices[vertex].is_some() {
            return Err(MuParityEvidenceError::DuplicateChoice {
                player: expected_player,
                vertex,
            });
        }
        let semantic_move = classify_semantic_move(graph, nodes, node_count, vertex, target)
            .ok_or(MuParityEvidenceError::SemanticMoveMismatch {
                player: expected_player,
                vertex,
            })?;
        if semantic_move != choice.semantic_move {
            return Err(MuParityEvidenceError::SemanticMoveMismatch {
                player: expected_player,
                vertex,
            });
        }
        choices[vertex] = Some(target);
    }

    Ok(ParityStrategy::new(
        expected_player,
        winning_vertices,
        choices,
    ))
}

fn canonical_vertex<V>(
    evidence: &MuParityPosition,
    positions: &[MuParityPosition],
    node_count: usize,
) -> Result<usize, MuParityEvidenceError<V>> {
    let vertex = evidence
        .state_index
        .checked_mul(node_count)
        .and_then(|base| base.checked_add(evidence.formula_node))
        .ok_or(MuParityEvidenceError::PositionMismatch { vertex: usize::MAX })?;
    if vertex >= positions.len() || positions[vertex] != *evidence {
        return Err(MuParityEvidenceError::PositionMismatch { vertex });
    }
    Ok(vertex)
}

fn classify_semantic_move<S, A>(
    graph: &ReachableGraph<S>,
    nodes: &[EvalNode<A>],
    node_count: usize,
    vertex: usize,
    target: usize,
) -> Option<MuParityMove> {
    let state_index = vertex / node_count;
    let formula_node = vertex % node_count;
    let target_state = target / node_count;
    let target_node = target % node_count;

    match &nodes[formula_node] {
        EvalNode::True | EvalNode::False | EvalNode::Literal { .. } => {
            (vertex == target).then_some(MuParityMove::OutcomeSelfLoop)
        }
        EvalNode::Var { binder } => (state_index == target_state && *binder == target_node)
            .then_some(MuParityMove::VariableReturn {
                binder_formula_node: *binder,
            }),
        EvalNode::And { left, right } | EvalNode::Or { left, right } => {
            if state_index != target_state {
                None
            } else if *left == target_node {
                Some(MuParityMove::BooleanLeft)
            } else if *right == target_node {
                Some(MuParityMove::BooleanRight)
            } else {
                None
            }
        }
        EvalNode::Diamond { inner } | EvalNode::Box { inner } => {
            if *inner != target_node {
                return None;
            }
            if graph.outgoing[state_index].is_empty() {
                (state_index == target_state).then_some(MuParityMove::ModalTerminalSelfLoop)
            } else if graph.outgoing[state_index]
                .iter()
                .any(|edge| edge.target == target_state)
            {
                Some(MuParityMove::ModalSuccessor {
                    target_state_index: target_state,
                })
            } else {
                None
            }
        }
        EvalNode::Fix { body, .. } => (state_index == target_state && *body == target_node)
            .then_some(MuParityMove::FixpointBody),
    }
}

fn build_evaluation_game<S, A, F, V>(
    graph: &ReachableGraph<S>,
    nodes: &[EvalNode<A>],
    max_alternation_level: usize,
    atom_holds: &F,
) -> Result<ParityGame, MuParityError<V>>
where
    F: Fn(&A, &S) -> bool,
{
    let node_count = nodes.len();
    let vertex_count = graph
        .states
        .len()
        .checked_mul(node_count)
        .ok_or(MuParityError::GameSizeOverflow)?;
    let mut owners = Vec::with_capacity(vertex_count);
    let mut priorities = Vec::with_capacity(vertex_count);
    let mut edges = Vec::with_capacity(vertex_count);

    for state in 0..graph.states.len() {
        for (node_index, node) in nodes.iter().enumerate() {
            let current = position(state, node_index, node_count);
            match node {
                EvalNode::True => {
                    owners.push(ParityPlayer::Even);
                    priorities.push(0);
                    edges.push(vec![current]);
                }
                EvalNode::False => {
                    owners.push(ParityPlayer::Even);
                    priorities.push(1);
                    edges.push(vec![current]);
                }
                EvalNode::Literal { atom, positive } => {
                    let holds = (atom_holds)(atom, &graph.states[state]);
                    let truth = if *positive { holds } else { !holds };
                    owners.push(ParityPlayer::Even);
                    priorities.push(if truth { 0 } else { 1 });
                    edges.push(vec![current]);
                }
                EvalNode::Var { binder } => {
                    owners.push(ParityPlayer::Even);
                    priorities.push(0);
                    edges.push(vec![position(state, *binder, node_count)]);
                }
                EvalNode::And { left, right } => {
                    owners.push(ParityPlayer::Odd);
                    priorities.push(0);
                    edges.push(vec![
                        position(state, *left, node_count),
                        position(state, *right, node_count),
                    ]);
                }
                EvalNode::Or { left, right } => {
                    owners.push(ParityPlayer::Even);
                    priorities.push(0);
                    edges.push(vec![
                        position(state, *left, node_count),
                        position(state, *right, node_count),
                    ]);
                }
                EvalNode::Diamond { inner } => {
                    owners.push(ParityPlayer::Even);
                    priorities.push(0);
                    edges.push(modal_successors(graph, state, *inner, node_count));
                }
                EvalNode::Box { inner } => {
                    owners.push(ParityPlayer::Odd);
                    priorities.push(0);
                    edges.push(modal_successors(graph, state, *inner, node_count));
                }
                EvalNode::Fix {
                    body,
                    kind,
                    alternation_level,
                } => {
                    owners.push(ParityPlayer::Even);
                    priorities.push(kind.priority(*alternation_level, max_alternation_level));
                    edges.push(vec![position(state, *body, node_count)]);
                }
            }
        }
    }

    ParityGame::new(owners, priorities, edges).map_err(MuParityError::Game)
}

fn modal_successors<S>(
    graph: &ReachableGraph<S>,
    state: usize,
    inner: usize,
    node_count: usize,
) -> Vec<usize> {
    if graph.outgoing[state].is_empty() {
        vec![position(state, inner, node_count)]
    } else {
        graph.outgoing[state]
            .iter()
            .map(|edge| position(edge.target, inner, node_count))
            .collect()
    }
}

fn position(state: usize, node: usize, node_count: usize) -> usize {
    state * node_count + node
}

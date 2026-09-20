use crate::graph::{capture_reachable_graph, GraphCaptureError, ReachableGraph};
use crate::model::{ModelError, TransitionSystem};
use crate::mu_calculus::{
    validate_mu_formula, MuFormula, MuInitialEvaluation, MuTerminalPolicy, MuValidationError,
};
use crate::parity_game::{solve_parity_game, ParityGame, ParityGameError, ParityPlayer};
use std::fmt;
use std::hash::Hash;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixpointKind {
    Mu,
    Nu,
}

impl FixpointKind {
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

#[derive(Debug, Clone)]
enum EvalNode<A> {
    True,
    False,
    Literal { atom: A, positive: bool },
    Var { binder: usize },
    And { left: usize, right: usize },
    Or { left: usize, right: usize },
    Diamond { inner: usize },
    Box { inner: usize },
    Fix {
        body: usize,
        kind: FixpointKind,
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

    let max_alternation_level = nodes
        .iter()
        .filter_map(|node| match node {
            EvalNode::Fix {
                alternation_level, ..
            } => Some(*alternation_level),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let game = build_evaluation_game(
        &captured.graph,
        &nodes,
        max_alternation_level,
        &atom_holds,
    )?;
    let solution = solve_parity_game(&game);
    let node_count = nodes.len();

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
    let max_priority = nodes
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
        .unwrap_or(0);

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
    })
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
    parent_fixpoint: Option<(FixpointKind, usize)>,
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
            FixpointKind::Mu,
            variable,
            body,
            positive,
            bindings,
            parent_fixpoint,
            nodes,
        ),
        MuFormula::Nu { variable, body } => lower_fixpoint(
            FixpointKind::Nu,
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
    original_kind: FixpointKind,
    variable: &V,
    body: &MuFormula<A, V>,
    positive: bool,
    bindings: &mut Vec<(V, usize)>,
    parent_fixpoint: Option<(FixpointKind, usize)>,
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

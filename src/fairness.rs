use crate::buchi::{
    BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState, BuchiResult, BuchiStatus,
    FiniteRunPolicy,
};
use crate::checker::TraceStep;
use crate::graph::{
    capture_reachable_graph, induced_graph, shortest_path, ReachableGraph,
};
use crate::model::TransitionSystem;
use crate::product::build_action_product;
use crate::recurrence::{
    component_is_cyclic, strongly_connected_components, RecurrenceError,
};
use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;

/// Exact-action weak-fairness assumptions for infinite executions.
///
/// For every configured action `a`, an infinite execution is weakly fair when
/// it does not postpone `a` forever while `a` remains continuously enabled.
/// Equivalently on a repeated finite recurrent walk, each configured action is
/// either disabled at some recurrent state or is taken on the walk.
///
/// The empty set preserves the repository's historical no-fairness semantics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeakFairness {
    actions: Vec<String>,
}

impl WeakFairness {
    pub fn new<I, T>(actions: I) -> Result<Self, FairnessError>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let mut ordered = Vec::new();
        let mut seen = HashSet::new();
        for action in actions {
            let action = action.into();
            if action.trim().is_empty() {
                return Err(FairnessError::EmptyActionName);
            }
            if !seen.insert(action.clone()) {
                return Err(FairnessError::DuplicateAction { action });
            }
            ordered.push(action);
        }
        Ok(Self { actions: ordered })
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn actions(&self) -> &[String] {
        &self.actions
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FairnessError {
    EmptyActionName,
    DuplicateAction { action: String },
}

impl fmt::Display for FairnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActionName => write!(f, "weak-fair action name must not be empty"),
            Self::DuplicateAction { action } => {
                write!(f, "duplicate weak-fair action '{action}'")
            }
        }
    }
}

impl std::error::Error for FairnessError {}

/// Universally verify one generalized Büchi automaton while quantifying only
/// over infinite executions that satisfy the configured exact-action weak
/// fairness assumptions. Finite terminal policy remains unchanged by fairness.
///
/// The empty fairness set is an exact semantic compatibility path for
/// `check_buchi`: it explores the same product and selects the same deterministic
/// acceptance-avoiding lasso when one exists.
pub fn check_buchi_with_weak_fairness<S, A>(
    model: &TransitionSystem<S>,
    automaton: &BuchiAutomaton<A>,
    fairness: &WeakFairness,
) -> Result<BuchiResult<S, A>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    let captured = capture_reachable_graph(model).map_err(RecurrenceError::from)?;
    let product = build_action_product(
        &captured.graph,
        automaton.initial(),
        |state, action| automaton.advance(state, action),
        |state, automaton| BuchiProductState { state, automaton },
    );
    let product_transitions = product.outgoing.iter().map(Vec::len).sum();
    let known_terminal = product
        .outgoing
        .iter()
        .map(Vec::is_empty)
        .collect::<Vec<_>>();
    let counterexample = find_fair_buchi_counterexample(
        &product,
        &known_terminal,
        automaton,
        fairness,
    )?;

    Ok(BuchiResult {
        automaton: automaton.name().to_owned(),
        status: if counterexample.is_some() {
            BuchiStatus::Violated
        } else {
            BuchiStatus::Satisfied
        },
        finite_policy: automaton.finite_policy(),
        acceptance_sets: automaton.acceptance_sets().len(),
        model_states: captured.discovered_states,
        model_transitions: captured.explored_transitions,
        product_states: product.states.len(),
        product_transitions,
        counterexample,
    })
}

fn find_fair_buchi_counterexample<S, A>(
    product: &ReachableGraph<BuchiProductState<S, A>>,
    known_terminal: &[bool],
    automaton: &BuchiAutomaton<A>,
    fairness: &WeakFairness,
) -> Result<Option<BuchiCounterexample<S, A>>, BuchiError>
where
    S: Clone + Eq + Hash,
    A: Clone + Eq + Hash,
{
    if automaton.finite_policy() == FiniteRunPolicy::RequireAcceptingTerminal {
        for (product_id, state) in product.states.iter().enumerate() {
            if !known_terminal[product_id] {
                continue;
            }
            let Some(set) = automaton
                .acceptance_sets()
                .iter()
                .find(|set| !set.contains(&state.automaton))
            else {
                continue;
            };
            let trace = shortest_path(product, &product.initial_ids, product_id, None)
                .ok_or(BuchiError::MissingWitness)?;
            return Ok(Some(BuchiCounterexample::FiniteTerminal {
                missing_acceptance: set.name().to_owned(),
                trace,
            }));
        }
    }

    let mut best: Option<FairBuchiCandidate<S, A>> = None;
    for (acceptance_index, set) in automaton.acceptance_sets().iter().enumerate() {
        let included = product
            .states
            .iter()
            .map(|state| !set.contains(&state.automaton))
            .collect::<Vec<_>>();
        let old_ids = included
            .iter()
            .enumerate()
            .filter_map(|(id, included)| included.then_some(id))
            .collect::<Vec<_>>();
        if old_ids.is_empty() {
            continue;
        }

        let residual = induced_graph(product, &included);
        for component in strongly_connected_components(&residual) {
            if !component_is_cyclic(&residual, &component) {
                continue;
            }
            let Some(cycle) = weakly_fair_cycle(
                product,
                &residual,
                &old_ids,
                &component,
                fairness,
            )? else {
                continue;
            };
            let entry = *component.first().ok_or(BuchiError::MissingWitness)?;
            let product_entry = old_ids[entry];
            let stem = shortest_path(product, &product.initial_ids, product_entry, None)
                .ok_or(BuchiError::MissingWitness)?;
            let candidate = FairBuchiCandidate {
                acceptance_index,
                product_entry,
                stem,
                cycle,
            };
            if best
                .as_ref()
                .is_none_or(|current| fair_candidate_key(&candidate) < fair_candidate_key(current))
            {
                best = Some(candidate);
            }
        }
    }

    Ok(best.map(|candidate| BuchiCounterexample::AcceptanceAvoidingCycle {
        acceptance: automaton.acceptance_sets()[candidate.acceptance_index]
            .name()
            .to_owned(),
        stem: candidate.stem,
        cycle: candidate.cycle,
    }))
}

struct FairBuchiCandidate<S, A> {
    acceptance_index: usize,
    product_entry: usize,
    stem: Vec<TraceStep<BuchiProductState<S, A>>>,
    cycle: Vec<TraceStep<BuchiProductState<S, A>>>,
}

fn fair_candidate_key<S, A>(candidate: &FairBuchiCandidate<S, A>) -> (usize, usize, usize) {
    (
        candidate.stem.len().saturating_sub(1),
        candidate.acceptance_index,
        candidate.product_entry,
    )
}

/// Build a deterministic closed recurrent walk through one cyclic residual SCC
/// that is weakly fair under every configured exact action.
///
/// `full_graph` supplies true action enablement. `residual` may have removed
/// states/edges for a property (for example, one Büchi acceptance-avoiding
/// region), so a fair action that is enabled only through an edge leaving the
/// residual must still count as continuously enabled. `residual_to_full` maps
/// residual state ids back to those full-graph ids.
///
/// A cyclic SCC admits a weakly fair infinite execution iff, for every fair
/// action, either some SCC state disables that action in the full graph or an
/// edge carrying that action remains inside the SCC. Strong connectivity then
/// lets one deterministic closed walk visit all such witnesses each period.
pub(crate) fn weakly_fair_cycle<S>(
    full_graph: &ReachableGraph<S>,
    residual: &ReachableGraph<S>,
    residual_to_full: &[usize],
    component: &[usize],
    fairness: &WeakFairness,
) -> Result<Option<Vec<TraceStep<S>>>, RecurrenceError>
where
    S: Clone + Eq,
{
    let Some(&entry) = component.first() else {
        return Ok(None);
    };
    if residual_to_full.len() != residual.states.len() {
        return Err(RecurrenceError::CycleWitnessMissing);
    }

    let members = component.iter().copied().collect::<HashSet<_>>();
    let mut cycle = vec![TraceStep {
        action: None,
        state: residual.states[entry].clone(),
    }];
    let mut current = entry;

    for action in fairness.actions() {
        if let Some(disabled) = component.iter().copied().find(|node| {
            let full_id = residual_to_full[*node];
            !full_graph.outgoing[full_id]
                .iter()
                .any(|edge| edge.action == *action)
        }) {
            append_path(residual, &members, current, disabled, &mut cycle)?;
            current = disabled;
            continue;
        }

        let internal = component.iter().copied().find_map(|source| {
            residual.outgoing[source]
                .iter()
                .find(|edge| edge.action == *action && members.contains(&edge.target))
                .map(|edge| (source, edge))
        });
        let Some((source, edge)) = internal else {
            return Ok(None);
        };

        append_path(residual, &members, current, source, &mut cycle)?;
        cycle.push(TraceStep {
            action: Some(edge.action.clone()),
            state: residual.states[edge.target].clone(),
        });
        current = edge.target;
    }

    // With no fairness obligations, or when every obligation is witnessed by
    // the entry being disabled, ensure the recurrent witness still executes a
    // real edge before closing.
    if cycle.len() == 1 {
        let edge = residual.outgoing[entry]
            .iter()
            .find(|edge| members.contains(&edge.target))
            .ok_or(RecurrenceError::CycleWitnessMissing)?;
        cycle.push(TraceStep {
            action: Some(edge.action.clone()),
            state: residual.states[edge.target].clone(),
        });
        current = edge.target;
    }

    append_path(residual, &members, current, entry, &mut cycle)?;
    if cycle.len() < 2
        || cycle.first().map(|step| &step.state) != cycle.last().map(|step| &step.state)
    {
        return Err(RecurrenceError::CycleWitnessMissing);
    }
    Ok(Some(cycle))
}

fn append_path<S>(
    graph: &ReachableGraph<S>,
    members: &HashSet<usize>,
    from: usize,
    to: usize,
    trace: &mut Vec<TraceStep<S>>,
) -> Result<(), RecurrenceError>
where
    S: Clone + Eq,
{
    if from == to {
        return Ok(());
    }
    let path = shortest_path(graph, &[from], to, Some(members))
        .ok_or(RecurrenceError::CycleWitnessMissing)?;
    trace.extend(path.into_iter().skip(1));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{check_buchi_with_weak_fairness, weakly_fair_cycle, FairnessError, WeakFairness};
    use crate::buchi::{check_buchi, BuchiCounterexample, BuchiStatus, FiniteRunPolicy};
    use crate::buchi_examples::{finite_quiet_run, pulse_automaton, unfair_second_pulse};
    use crate::graph::{induced_graph, ReachableGraph, SnapshotEdge};
    use crate::recurrence::{component_is_cyclic, strongly_connected_components};
    use std::collections::HashSet;

    const N: usize = 2;
    const EDGE_COUNT: usize = N * N;
    const CODE_COUNT: usize = 4; // absent, fair-a, fair-b, other

    fn decode(mut assignment: usize) -> [u8; EDGE_COUNT] {
        let mut codes = [0; EDGE_COUNT];
        for code in &mut codes {
            *code = (assignment % CODE_COUNT) as u8;
            assignment /= CODE_COUNT;
        }
        codes
    }

    fn action(code: u8) -> &'static str {
        match code {
            1 => "a",
            2 => "b",
            3 => "other",
            _ => unreachable!("absent edges have no action"),
        }
    }

    fn graph(codes: [u8; EDGE_COUNT]) -> ReachableGraph<usize> {
        let mut outgoing = vec![Vec::new(); N];
        for (from, edges) in outgoing.iter_mut().enumerate() {
            for to in 0..N {
                let code = codes[from * N + to];
                if code != 0 {
                    edges.push(SnapshotEdge {
                        action: action(code).to_owned(),
                        target: to,
                    });
                }
            }
        }
        ReachableGraph {
            states: (0..N).collect(),
            outgoing,
            initial_ids: vec![0],
        }
    }

    fn oracle_component_is_fair(
        full: &ReachableGraph<usize>,
        residual: &ReachableGraph<usize>,
        old_ids: &[usize],
        component: &[usize],
        fair_actions: &[&str],
    ) -> bool {
        let members = component.iter().copied().collect::<HashSet<_>>();
        fair_actions.iter().all(|action| {
            let has_disabled_state = component.iter().copied().any(|residual_id| {
                !full.outgoing[old_ids[residual_id]]
                    .iter()
                    .any(|edge| edge.action == *action)
            });
            let has_internal_taken_edge = component.iter().copied().any(|source| {
                residual.outgoing[source]
                    .iter()
                    .any(|edge| edge.action == *action && members.contains(&edge.target))
            });
            has_disabled_state || has_internal_taken_edge
        })
    }

    fn assert_real_closed_fair_cycle(
        full: &ReachableGraph<usize>,
        cycle: &[TraceStep<usize>],
        fair_actions: &[&str],
    ) {
        assert!(cycle.len() >= 2);
        assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
        for pair in cycle.windows(2) {
            let action = pair[1].action.as_deref().expect("cycle edge action");
            assert!(full.outgoing[pair[0].state]
                .iter()
                .any(|edge| edge.action == action && edge.target == pair[1].state));
        }
        for action in fair_actions {
            let taken = cycle
                .iter()
                .skip(1)
                .any(|step| step.action.as_deref() == Some(*action));
            let disabled = cycle.iter().take(cycle.len() - 1).any(|step| {
                !full.outgoing[step.state]
                    .iter()
                    .any(|edge| edge.action == *action)
            });
            assert!(taken || disabled, "fair action {action} lacks a witness");
        }
    }

    #[test]
    fn validates_exact_action_declarations_deterministically() {
        assert_eq!(
            WeakFairness::new([" "]).unwrap_err(),
            FairnessError::EmptyActionName
        );
        assert_eq!(
            WeakFairness::new(["a", "b", "a"]).unwrap_err(),
            FairnessError::DuplicateAction {
                action: "a".to_owned()
            }
        );
        assert!(WeakFairness::none().is_empty());
        assert_eq!(
            WeakFairness::new(["b", "a"]).unwrap().actions(),
            &["b".to_owned(), "a".to_owned()]
        );
    }

    #[test]
    fn enabled_edge_leaving_residual_makes_stay_cycle_unfair() {
        let full = ReachableGraph {
            states: vec![0usize, 1],
            outgoing: vec![
                vec![
                    SnapshotEdge {
                        action: "stay".to_owned(),
                        target: 0,
                    },
                    SnapshotEdge {
                        action: "fair".to_owned(),
                        target: 1,
                    },
                ],
                vec![],
            ],
            initial_ids: vec![0],
        };
        let included = vec![true, false];
        let residual = induced_graph(&full, &included);
        let fairness = WeakFairness::new(["fair"]).unwrap();
        assert!(weakly_fair_cycle(&full, &residual, &[0], &[0], &fairness)
            .unwrap()
            .is_none());
    }

    #[test]
    fn disabled_recurrent_state_and_internal_fair_edge_both_admit_witnesses() {
        let disabled = ReachableGraph {
            states: vec![0usize, 1],
            outgoing: vec![
                vec![SnapshotEdge {
                    action: "other".to_owned(),
                    target: 1,
                }],
                vec![SnapshotEdge {
                    action: "back".to_owned(),
                    target: 0,
                }],
            ],
            initial_ids: vec![0],
        };
        let fairness = WeakFairness::new(["fair"]).unwrap();
        let cycle = weakly_fair_cycle(&disabled, &disabled, &[0, 1], &[0, 1], &fairness)
            .unwrap()
            .unwrap();
        assert_real_closed_fair_cycle(&disabled, &cycle, &["fair"]);

        let taken = ReachableGraph {
            states: vec![0usize],
            outgoing: vec![vec![SnapshotEdge {
                action: "fair".to_owned(),
                target: 0,
            }]],
            initial_ids: vec![0],
        };
        let cycle = weakly_fair_cycle(&taken, &taken, &[0], &[0], &fairness)
            .unwrap()
            .unwrap();
        assert_real_closed_fair_cycle(&taken, &cycle, &["fair"]);
    }

    #[test]
    fn generated_two_node_residuals_match_independent_weak_fairness_oracle() {
        let fair_configs: &[&[&str]] = &[&[], &["a"], &["b"], &["a", "b"]];
        for assignment in 0..CODE_COUNT.pow(EDGE_COUNT as u32) {
            let full = graph(decode(assignment));
            for subset in 1usize..(1usize << N) {
                let included = (0..N)
                    .map(|node| subset & (1usize << node) != 0)
                    .collect::<Vec<_>>();
                let old_ids = included
                    .iter()
                    .enumerate()
                    .filter_map(|(id, included)| included.then_some(id))
                    .collect::<Vec<_>>();
                let residual = induced_graph(&full, &included);
                for component in strongly_connected_components(&residual) {
                    if !component_is_cyclic(&residual, &component) {
                        continue;
                    }
                    for fair_actions in fair_configs {
                        let fairness = WeakFairness::new(fair_actions.iter().copied()).unwrap();
                        let witness = weakly_fair_cycle(
                            &full,
                            &residual,
                            &old_ids,
                            &component,
                            &fairness,
                        )
                        .unwrap();
                        let expected = oracle_component_is_fair(
                            &full,
                            &residual,
                            &old_ids,
                            &component,
                            fair_actions,
                        );
                        assert_eq!(
                            witness.is_some(),
                            expected,
                            "assignment={assignment} subset={subset} component={component:?} fairness={fair_actions:?}"
                        );
                        if let Some(cycle) = witness {
                            assert_real_closed_fair_cycle(&full, &cycle, fair_actions);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn empty_fairness_exactly_preserves_existing_buchi_results() {
        let fairness = WeakFairness::none();
        for model in [unfair_second_pulse().unwrap(), finite_quiet_run().unwrap()] {
            for policy in [
                FiniteRunPolicy::IgnoreTerminals,
                FiniteRunPolicy::RequireAcceptingTerminal,
            ] {
                let automaton = pulse_automaton(policy).unwrap();
                assert_eq!(
                    check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap(),
                    check_buchi(&model, &automaton).unwrap()
                );
            }
        }
    }

    #[test]
    fn weak_fair_pulse_b_excludes_the_unfair_acceptance_avoiding_loop() {
        let model = unfair_second_pulse().unwrap();
        let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
        let ordinary = check_buchi(&model, &automaton).unwrap();
        assert_eq!(ordinary.status, BuchiStatus::Violated);

        let fairness = WeakFairness::new(["pulse-b"]).unwrap();
        let fair = check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap();
        assert_eq!(fair.status, BuchiStatus::Satisfied);
        assert!(fair.counterexample.is_none());
    }

    #[test]
    fn fair_counterexample_itself_carries_the_required_taken_action() {
        let model = unfair_second_pulse().unwrap();
        let automaton = pulse_automaton(FiniteRunPolicy::IgnoreTerminals).unwrap();
        let fairness = WeakFairness::new(["pulse-a"]).unwrap();
        let result = check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap();
        assert_eq!(result.status, BuchiStatus::Violated);
        let Some(BuchiCounterexample::AcceptanceAvoidingCycle { cycle, .. }) = result.counterexample
        else {
            panic!("expected fair acceptance-avoiding cycle");
        };
        assert!(cycle
            .iter()
            .skip(1)
            .any(|step| step.action.as_deref() == Some("pulse-a")));
        assert_eq!(cycle.first().unwrap().state, cycle.last().unwrap().state);
    }

    #[test]
    fn weak_fairness_does_not_change_strict_finite_terminal_policy() {
        let model = finite_quiet_run().unwrap();
        let automaton = pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal).unwrap();
        let fairness = WeakFairness::new(["pulse-a", "pulse-b"]).unwrap();
        let result = check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap();
        assert_eq!(result.status, BuchiStatus::Violated);
        assert!(matches!(
            result.counterexample,
            Some(BuchiCounterexample::FiniteTerminal { .. })
        ));
    }
}

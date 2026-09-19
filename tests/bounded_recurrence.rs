use formal_verification_lab::{
    analyze_recurrence, analyze_recurrence_with_limits, BoundedOutcome, ExplorationLimits,
    InconclusiveReason, Invariant, RecurrenceStatus, StateVariable, Transition, TransitionSystem,
};
use std::collections::{HashMap, VecDeque};

const N: usize = 3;
const EDGE_COUNT: usize = N * N;
const INF: usize = usize::MAX / 4;

fn edge_index(from: usize, to: usize) -> usize {
    from * N + to
}

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << edge_index(from, to)) != 0
}

fn graph_model(mask: usize) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("bounded-recurrence-{mask}"),
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        move |state| {
            let mut successors = Vec::new();
            for to in 0..N {
                if has_edge(mask, *state, to) {
                    successors.push(Transition::new(format!("{}->{to}", *state), to));
                }
            }
            Ok(successors)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

#[derive(Debug, Clone)]
struct OracleNode {
    state: usize,
    depth: usize,
}

#[derive(Debug, Clone)]
struct OracleCapture {
    states: Vec<usize>,
    outgoing: Vec<Vec<usize>>,
    completion: Option<InconclusiveReason>,
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleComponent {
    states: Vec<usize>,
    cyclic: bool,
}

#[derive(Debug, Clone)]
struct OracleResult {
    outcome: BoundedOutcome<RecurrenceStatus>,
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
    cutoff_reason: Option<InconclusiveReason>,
    components: Option<Vec<OracleComponent>>,
    first_cycle_component: Option<usize>,
    first_cycle_entry: Option<usize>,
    first_cycle_distance: Option<usize>,
}

fn capture_oracle(mask: usize, limits: ExplorationLimits) -> OracleCapture {
    let mut nodes = Vec::<OracleNode>::new();
    let mut by_state = HashMap::<usize, usize>::new();
    let mut outgoing = Vec::<Vec<usize>>::new();
    let mut queue = VecDeque::new();
    let mut max_depth_reached = None;

    if let Some(limit) = limits.max_states.filter(|limit| nodes.len() >= *limit) {
        return OracleCapture {
            states: Vec::new(),
            outgoing,
            completion: Some(InconclusiveReason::StateLimitReached { limit }),
            discovered_states: 0,
            checked_states: 0,
            explored_transitions: 0,
            max_depth_reached,
        };
    }

    nodes.push(OracleNode { state: 0, depth: 0 });
    by_state.insert(0, 0);
    outgoing.push(Vec::new());
    queue.push_back(0);
    max_depth_reached = Some(0);

    let mut checked_states = 0usize;
    let mut explored_transitions = 0usize;

    while let Some(source) = queue.pop_front() {
        checked_states += 1;
        let state = nodes[source].state;
        let depth = nodes[source].depth;

        for target_state in 0..N {
            if !has_edge(mask, state, target_state) {
                continue;
            }

            if let Some(limit) = limits
                .max_transitions
                .filter(|limit| explored_transitions >= *limit)
            {
                return finish_capture(
                    nodes,
                    outgoing,
                    Some(InconclusiveReason::TransitionLimitReached { limit }),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                );
            }
            explored_transitions += 1;

            if let Some(target) = by_state.get(&target_state).copied() {
                outgoing[source].push(target);
                continue;
            }

            if let Some(limit) = limits.max_depth.filter(|limit| depth >= *limit) {
                return finish_capture(
                    nodes,
                    outgoing,
                    Some(InconclusiveReason::DepthLimitReached { limit }),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                );
            }

            if let Some(limit) = limits.max_states.filter(|limit| nodes.len() >= *limit) {
                return finish_capture(
                    nodes,
                    outgoing,
                    Some(InconclusiveReason::StateLimitReached { limit }),
                    checked_states,
                    explored_transitions,
                    max_depth_reached,
                );
            }

            let target = nodes.len();
            nodes.push(OracleNode {
                state: target_state,
                depth: depth + 1,
            });
            by_state.insert(target_state, target);
            outgoing.push(Vec::new());
            outgoing[source].push(target);
            queue.push_back(target);
            max_depth_reached = Some(max_depth_reached.unwrap_or(0).max(depth + 1));
        }
    }

    finish_capture(
        nodes,
        outgoing,
        None,
        checked_states,
        explored_transitions,
        max_depth_reached,
    )
}

fn finish_capture(
    nodes: Vec<OracleNode>,
    outgoing: Vec<Vec<usize>>,
    completion: Option<InconclusiveReason>,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
) -> OracleCapture {
    let states = nodes.iter().map(|node| node.state).collect::<Vec<_>>();
    OracleCapture {
        discovered_states: states.len(),
        states,
        outgoing,
        completion,
        checked_states,
        explored_transitions,
        max_depth_reached,
    }
}

fn oracle(mask: usize, limits: ExplorationLimits) -> OracleResult {
    let capture = capture_oracle(mask, limits);
    let count = capture.states.len();
    let mut distance = vec![vec![INF; count]; count];
    for (node, row) in distance.iter_mut().enumerate() {
        row[node] = 0;
    }
    for (from, edges) in capture.outgoing.iter().enumerate() {
        for &to in edges {
            distance[from][to] = distance[from][to].min(1);
        }
    }
    for via in 0..count {
        for from in 0..count {
            for to in 0..count {
                let through = distance[from][via].saturating_add(distance[via][to]);
                distance[from][to] = distance[from][to].min(through);
            }
        }
    }

    let mut assigned = vec![false; count];
    let mut components = Vec::new();
    for node in 0..count {
        if assigned[node] {
            continue;
        }
        let members = (0..count)
            .filter(|other| distance[node][*other] < INF && distance[*other][node] < INF)
            .collect::<Vec<_>>();
        for member in &members {
            assigned[*member] = true;
        }
        let cyclic = members.len() > 1 || capture.outgoing[node].contains(&node);
        components.push((members, cyclic));
    }

    let first_cycle_component = components.iter().position(|(_, cyclic)| *cyclic);
    let first_cycle_entry =
        first_cycle_component.and_then(|index| components[index].0.first().copied());
    let first_cycle_distance = first_cycle_entry.map(|entry| distance[0][entry]);

    let outcome = if first_cycle_component.is_some() {
        BoundedOutcome::Conclusive(RecurrenceStatus::CycleFound)
    } else if let Some(reason) = capture.completion {
        BoundedOutcome::Inconclusive(reason)
    } else {
        BoundedOutcome::Conclusive(RecurrenceStatus::Acyclic)
    };

    let exposed_components = capture.completion.is_none().then(|| {
        components
            .iter()
            .map(|(members, cyclic)| OracleComponent {
                states: members.iter().map(|id| capture.states[*id]).collect(),
                cyclic: *cyclic,
            })
            .collect()
    });

    OracleResult {
        outcome,
        discovered_states: capture.discovered_states,
        checked_states: capture.checked_states,
        explored_transitions: capture.explored_transitions,
        max_depth_reached: capture.max_depth_reached,
        cutoff_reason: capture.completion,
        components: exposed_components,
        first_cycle_component,
        first_cycle_entry: first_cycle_entry.map(|id| capture.states[id]),
        first_cycle_distance,
    }
}

fn profiles() -> [ExplorationLimits; 9] {
    [
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
            max_states: Some(2),
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
        ExplorationLimits {
            max_depth: Some(1),
            ..ExplorationLimits::unbounded()
        },
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(2),
            max_depth: Some(1),
        },
    ]
}

#[test]
fn retained_self_loop_is_conclusive_before_later_transition_cutoff() {
    let mask = (1usize << edge_index(0, 0)) | (1usize << edge_index(0, 1));
    let result = analyze_recurrence_with_limits(
        &graph_model(mask),
        ExplorationLimits {
            max_transitions: Some(1),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(RecurrenceStatus::CycleFound)
    );
    assert!(result.components.is_none());
    assert_eq!(
        result.cutoff_reason,
        Some(InconclusiveReason::TransitionLimitReached { limit: 1 })
    );
    let witness = result.first_cycle.unwrap();
    assert_eq!(witness.stem.len(), 1);
    assert_eq!(witness.cycle.first().unwrap().state, 0);
    assert_eq!(witness.cycle.last().unwrap().state, 0);
}

#[test]
fn retained_two_state_cycle_is_conclusive_before_later_cutoff() {
    let mask =
        (1usize << edge_index(0, 1)) | (1usize << edge_index(1, 0)) | (1usize << edge_index(1, 2));
    let result = analyze_recurrence_with_limits(
        &graph_model(mask),
        ExplorationLimits {
            max_transitions: Some(2),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(RecurrenceStatus::CycleFound)
    );
    assert!(result.components.is_none());
    let witness = result.first_cycle.unwrap();
    assert_eq!(witness.cycle.first().unwrap().state, 0);
    assert_eq!(witness.cycle.last().unwrap().state, 0);
    validate_trace_edges(mask, &witness.stem);
    validate_trace_edges(mask, &witness.cycle);
}

#[test]
fn acyclic_prefix_with_cutoff_is_inconclusive_not_acyclic() {
    let mask = 1usize << edge_index(0, 1);
    let result = analyze_recurrence_with_limits(
        &graph_model(mask),
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached { limit: 0 })
    );
    assert!(result.components.is_none());
    assert!(result.first_cycle.is_none());
}

#[test]
fn exact_sufficient_limits_can_prove_acyclicity_and_expose_full_partition() {
    let mask = 1usize << edge_index(0, 1);
    let result = analyze_recurrence_with_limits(
        &graph_model(mask),
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(1),
            max_depth: Some(1),
        },
    )
    .unwrap();

    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(RecurrenceStatus::Acyclic)
    );
    assert_eq!(result.components.as_ref().unwrap().len(), 2);
    assert!(result.first_cycle.is_none());
}

#[test]
fn unbounded_limits_preserve_sealed_recurrence_structure() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let model = graph_model(mask);
        let sealed = analyze_recurrence(&model).unwrap();
        let bounded =
            analyze_recurrence_with_limits(&model, ExplorationLimits::unbounded()).unwrap();

        let expected_status = if sealed.first_cycle.is_some() {
            RecurrenceStatus::CycleFound
        } else {
            RecurrenceStatus::Acyclic
        };
        assert_eq!(
            bounded.outcome,
            BoundedOutcome::Conclusive(expected_status),
            "mask={mask}"
        );
        assert_eq!(
            bounded.components.as_ref(),
            Some(&sealed.components),
            "mask={mask}"
        );
        assert_eq!(bounded.first_cycle, sealed.first_cycle, "mask={mask}");
        assert_eq!(
            bounded.discovered_states, sealed.discovered_states,
            "mask={mask}"
        );
        assert_eq!(
            bounded.explored_transitions, sealed.explored_transitions,
            "mask={mask}"
        );
        assert_eq!(
            bounded.max_depth_reached, sealed.max_depth_reached,
            "mask={mask}"
        );
    }
}

#[test]
fn all_three_node_graphs_and_limits_match_independent_prefix_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let model = graph_model(mask);

        for limits in profiles() {
            let expected = oracle(mask, limits);
            let first = analyze_recurrence_with_limits(&model, limits).unwrap();
            let second = analyze_recurrence_with_limits(&model, limits).unwrap();
            let context = format!("mask={mask} limits={limits:?}");

            assert_eq!(first, second, "determinism {context}");
            assert_eq!(first.outcome, expected.outcome, "outcome {context}");
            assert_eq!(
                first.discovered_states, expected.discovered_states,
                "discovered {context}"
            );
            assert_eq!(
                first.checked_states, expected.checked_states,
                "checked {context}"
            );
            assert_eq!(
                first.explored_transitions, expected.explored_transitions,
                "transitions {context}"
            );
            assert_eq!(
                first.max_depth_reached, expected.max_depth_reached,
                "depth {context}"
            );
            assert_eq!(
                first.cutoff_reason, expected.cutoff_reason,
                "cutoff reason {context}"
            );

            match (&first.components, &expected.components) {
                (None, None) => {}
                (Some(actual), Some(expected_components)) => {
                    assert_eq!(
                        actual.len(),
                        expected_components.len(),
                        "components {context}"
                    );
                    for (actual, expected_component) in
                        actual.iter().zip(expected_components.iter())
                    {
                        assert_eq!(
                            actual.states, expected_component.states,
                            "component states {context}"
                        );
                        assert_eq!(
                            actual.cyclic, expected_component.cyclic,
                            "component cyclic {context}"
                        );
                    }
                }
                (actual, expected_components) => panic!(
                    "component exposure mismatch {context}: actual={actual:?} expected={expected_components:?}"
                ),
            }

            match (
                expected.first_cycle_component,
                expected.first_cycle_entry,
                expected.first_cycle_distance,
                first.first_cycle.as_ref(),
            ) {
                (None, None, None, None) => {}
                (Some(component), Some(entry), Some(distance), Some(witness)) => {
                    assert_eq!(witness.component_index, component, "component index {context}");
                    assert_eq!(
                        witness.stem.last().unwrap().state,
                        entry,
                        "stem entry {context}"
                    );
                    assert_eq!(
                        witness.stem.len().saturating_sub(1),
                        distance,
                        "stem distance {context}"
                    );
                    assert!(witness.cycle.len() >= 2, "cycle length {context}");
                    assert_eq!(
                        witness.cycle.first().unwrap().state,
                        entry,
                        "cycle entry {context}"
                    );
                    assert_eq!(
                        witness.cycle.last().unwrap().state,
                        entry,
                        "cycle closure {context}"
                    );
                    validate_trace_edges(mask, &witness.stem);
                    validate_trace_edges(mask, &witness.cycle);
                }
                (component, entry, distance, actual) => panic!(
                    "cycle mismatch {context}: component={component:?} entry={entry:?} distance={distance:?} actual={actual:?}"
                ),
            }
        }
    }
}

fn validate_trace_edges(mask: usize, trace: &[formal_verification_lab::TraceStep<usize>]) {
    for pair in trace.windows(2) {
        let from = pair[0].state;
        let to = pair[1].state;
        assert!(
            has_edge(mask, from, to),
            "missing edge {from}->{to} mask={mask}"
        );
        assert_eq!(
            pair[1].action.as_deref(),
            Some(format!("{from}->{to}").as_str())
        );
    }
}

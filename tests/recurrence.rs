use formal_verification_lab::examples::{bounded_counter, traffic_light, Light};
use formal_verification_lab::{
    analyze_recurrence, Invariant, StateVariable, Transition, TransitionSystem,
};

const N: usize = 3;
const EDGE_COUNT: usize = N * N;
const INF: usize = usize::MAX / 4;

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << (from * N + to)) != 0
}

fn all_pairs_shortest_paths(mask: usize) -> [[usize; N]; N] {
    let mut distance = [[INF; N]; N];
    for (node, row) in distance.iter_mut().enumerate() {
        row[node] = 0;
    }
    for (from, row) in distance.iter_mut().enumerate() {
        for (to, value) in row.iter_mut().enumerate() {
            if has_edge(mask, from, to) {
                *value = (*value).min(1);
            }
        }
    }
    for via in 0..N {
        for from in 0..N {
            for to in 0..N {
                let through = distance[from][via].saturating_add(distance[via][to]);
                distance[from][to] = distance[from][to].min(through);
            }
        }
    }
    distance
}

/// For these three-node generated fixtures, canonical BFS starts at node 0 and
/// enumerates numeric successor ids in ascending order. Floyd-Warshall already
/// gives each node's shortest discovery depth; with only two non-initial nodes,
/// sorting by (depth, node id) therefore gives the exact independent discovery
/// order without re-running the checker traversal.
fn expected_discovery_order(distance: &[[usize; N]; N]) -> Vec<usize> {
    let mut order = (0..N)
        .filter(|node| distance[0][*node] < INF)
        .collect::<Vec<_>>();
    order.sort_by_key(|node| (distance[0][*node], *node));
    order
}

fn expected_components(mask: usize, distance: &[[usize; N]; N]) -> Vec<(Vec<usize>, bool)> {
    let discovery_order = expected_discovery_order(distance);
    let mut assigned = [false; N];
    let mut components = Vec::new();

    for &node in &discovery_order {
        if assigned[node] {
            continue;
        }
        let members = discovery_order
            .iter()
            .copied()
            .filter(|other| distance[node][*other] < INF && distance[*other][node] < INF)
            .collect::<Vec<_>>();
        for member in &members {
            assigned[*member] = true;
        }
        let cyclic = members.len() > 1 || has_edge(mask, node, node);
        components.push((members, cyclic));
    }

    components
}

fn graph_model(mask: usize) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("recurrence-graph-{mask}"),
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

#[test]
fn bounded_counter_is_acyclic_with_four_singleton_components() {
    let model = bounded_counter().unwrap();
    let analysis = analyze_recurrence(&model).unwrap();

    assert_eq!(analysis.discovered_states, 4);
    assert_eq!(analysis.explored_transitions, 3);
    assert_eq!(analysis.max_depth_reached, Some(3));
    assert_eq!(analysis.components.len(), 4);
    assert!(analysis
        .components
        .iter()
        .all(|component| !component.cyclic));
    assert!(analysis.first_cycle.is_none());
}

#[test]
fn traffic_light_has_one_recurrent_component_and_closed_witness() {
    let model = traffic_light().unwrap();
    let analysis = analyze_recurrence(&model).unwrap();

    assert_eq!(analysis.discovered_states, 3);
    assert_eq!(analysis.explored_transitions, 3);
    assert_eq!(analysis.components.len(), 1);
    assert!(analysis.components[0].cyclic);
    assert_eq!(analysis.components[0].states.len(), 3);

    let witness = analysis.first_cycle.unwrap();
    assert_eq!(witness.component_index, 0);
    assert_eq!(witness.stem.len(), 1);
    assert_eq!(witness.stem[0].state.light, Light::Red);
    assert_eq!(witness.cycle.len(), 4);
    assert_eq!(witness.cycle.first().unwrap().state.light, Light::Red);
    assert_eq!(witness.cycle.last().unwrap().state.light, Light::Red);
    assert!(witness
        .cycle
        .iter()
        .skip(1)
        .all(|step| step.action.as_deref() == Some("advance")));
}

#[test]
fn all_three_node_graphs_match_independent_mutual_reachability_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        let distance = all_pairs_shortest_paths(mask);
        let expected = expected_components(mask, &distance);
        let model = graph_model(mask);
        let first = analyze_recurrence(&model).unwrap();
        let second = analyze_recurrence(&model).unwrap();

        assert_eq!(first, second, "determinism failed for mask={mask}");
        assert_eq!(first.components.len(), expected.len(), "mask={mask}");

        for (actual, (expected_states, expected_cyclic)) in
            first.components.iter().zip(expected.iter())
        {
            assert_eq!(&actual.states, expected_states, "mask={mask}");
            assert_eq!(actual.cyclic, *expected_cyclic, "mask={mask}");
        }

        let reachable_count = (0..N).filter(|node| distance[0][*node] < INF).count();
        let reachable_edges = (0..N)
            .filter(|node| distance[0][*node] < INF)
            .map(|from| (0..N).filter(|to| has_edge(mask, from, *to)).count())
            .sum::<usize>();
        assert_eq!(first.discovered_states, reachable_count, "mask={mask}");
        assert_eq!(first.explored_transitions, reachable_edges, "mask={mask}");

        let expected_cycle_component = expected.iter().position(|(_, cyclic)| *cyclic);
        match (expected_cycle_component, first.first_cycle) {
            (None, None) => {}
            (Some(component_index), Some(witness)) => {
                assert_eq!(witness.component_index, component_index, "mask={mask}");
                let component = &expected[component_index].0;
                let entry = component[0];
                assert_eq!(witness.stem.last().unwrap().state, entry, "mask={mask}");
                assert_eq!(witness.stem.len() - 1, distance[0][entry], "mask={mask}");
                validate_trace_edges(mask, &witness.stem);

                assert!(witness.cycle.len() >= 2, "mask={mask}");
                assert_eq!(witness.cycle.first().unwrap().state, entry, "mask={mask}");
                assert_eq!(witness.cycle.last().unwrap().state, entry, "mask={mask}");
                assert!(
                    witness.cycle.iter().all(|step| component.contains(&step.state)),
                    "cycle escaped component mask={mask}"
                );
                validate_trace_edges(mask, &witness.cycle);
            }
            (expected_cycle, actual_cycle) => panic!(
                "cycle witness mismatch mask={mask}: expected={expected_cycle:?} actual={actual_cycle:?}"
            ),
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

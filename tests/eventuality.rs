use formal_verification_lab::examples::{bounded_counter, CounterState};
use formal_verification_lab::{
    check_eventuality, check_reachability, EventualityCounterexample, EventualityProperty,
    EventualityStatus, Invariant, ReachabilityProperty, ReachabilityStatus, StateVariable,
    Transition, TransitionSystem,
};

const N: usize = 3;
const EDGE_COUNT: usize = N * N;
const INF: usize = usize::MAX / 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ChoicePhase {
    Start,
    Goal,
    Loop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ChoiceState {
    phase: ChoicePhase,
}

fn choice_model() -> TransitionSystem<ChoiceState> {
    TransitionSystem::new(
        "eventuality-choice",
        vec![StateVariable::new("phase", "branching progress state")],
        vec![ChoiceState {
            phase: ChoicePhase::Start,
        }],
        |state| match state.phase {
            ChoicePhase::Start => Ok(vec![
                Transition::new(
                    "choose-goal",
                    ChoiceState {
                        phase: ChoicePhase::Goal,
                    },
                ),
                Transition::new(
                    "choose-loop",
                    ChoiceState {
                        phase: ChoicePhase::Loop,
                    },
                ),
            ]),
            ChoicePhase::Goal => Ok(Vec::new()),
            ChoicePhase::Loop => Ok(vec![Transition::new(
                "loop",
                ChoiceState {
                    phase: ChoicePhase::Loop,
                },
            )]),
        },
        vec![Invariant::new("well-formed", |_state: &ChoiceState| true)],
    )
    .unwrap()
}

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << (from * N + to)) != 0
}

fn target_contains(target_mask: usize, node: usize) -> bool {
    target_mask & (1usize << node) != 0
}

fn residual_distances(mask: usize, target_mask: usize) -> [[usize; N]; N] {
    let mut distance = [[INF; N]; N];
    for node in 0..N {
        if !target_contains(target_mask, node) {
            distance[node][node] = 0;
        }
    }
    for from in 0..N {
        for to in 0..N {
            if !target_contains(target_mask, from)
                && !target_contains(target_mask, to)
                && has_edge(mask, from, to)
            {
                distance[from][to] = distance[from][to].min(1);
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

fn graph_model(mask: usize) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("eventuality-graph-{mask}"),
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
fn bounded_counter_eventually_reaches_three() {
    let model = bounded_counter().unwrap();
    let property = EventualityProperty::new("eventually-three", |state: &CounterState| {
        state.value == 3
    })
    .unwrap();
    let result = check_eventuality(&model, &property).unwrap();

    assert_eq!(result.status, EventualityStatus::Satisfied);
    assert!(result.counterexample.is_none());
    assert_eq!(result.discovered_states, 4);
    assert_eq!(result.explored_transitions, 3);
}

#[test]
fn bounded_counter_missing_target_has_finite_counterexample() {
    let model = bounded_counter().unwrap();
    let property = EventualityProperty::new("eventually-four", |state: &CounterState| {
        state.value == 4
    })
    .unwrap();
    let result = check_eventuality(&model, &property).unwrap();

    assert_eq!(result.status, EventualityStatus::Violated);
    let EventualityCounterexample::Finite { trace } = result.counterexample.unwrap() else {
        panic!("expected finite counterexample");
    };
    assert_eq!(trace.len(), 4);
    assert_eq!(trace.last().unwrap().state.value, 3);
}

#[test]
fn existential_reachability_does_not_imply_universal_eventuality() {
    let model = choice_model();
    let reachable = ReachabilityProperty::new("goal-reachable", |state: &ChoiceState| {
        state.phase == ChoicePhase::Goal
    })
    .unwrap();
    let eventual = EventualityProperty::new("goal-inevitable", |state: &ChoiceState| {
        state.phase == ChoicePhase::Goal
    })
    .unwrap();

    assert_eq!(
        check_reachability(&model, &reachable).unwrap().status,
        ReachabilityStatus::Reachable
    );
    let result = check_eventuality(&model, &eventual).unwrap();
    assert_eq!(result.status, EventualityStatus::Violated);
    let EventualityCounterexample::Infinite { stem, cycle } = result.counterexample.unwrap() else {
        panic!("expected infinite counterexample");
    };
    assert_eq!(stem.last().unwrap().state.phase, ChoicePhase::Loop);
    assert_eq!(cycle.first().unwrap().state.phase, ChoicePhase::Loop);
    assert_eq!(cycle.last().unwrap().state.phase, ChoicePhase::Loop);
}

#[test]
fn behavior_after_target_does_not_create_a_false_counterexample() {
    let model = TransitionSystem::new(
        "target-then-loop",
        vec![StateVariable::new("node", "state")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("hit-target", 1)]),
            1 => Ok(vec![Transition::new("leave-target", 2)]),
            2 => Ok(vec![Transition::new("loop", 2)]),
            _ => unreachable!(),
        },
        vec![Invariant::new("well-formed", |state: &usize| *state <= 2)],
    )
    .unwrap();
    let property = EventualityProperty::new("eventually-one", |state: &usize| *state == 1).unwrap();

    let result = check_eventuality(&model, &property).unwrap();
    assert_eq!(result.status, EventualityStatus::Satisfied);
    assert!(result.counterexample.is_none());
}

#[test]
fn eventuality_property_names_are_validated() {
    assert!(EventualityProperty::<usize>::new("   ", |_state: &usize| true).is_err());
}

#[test]
fn all_three_node_graphs_and_target_sets_match_independent_oracle() {
    for mask in 0..(1usize << EDGE_COUNT) {
        for target_mask in 0..(1usize << N) {
            let model = graph_model(mask);
            let property = EventualityProperty::new(
                format!("target-{target_mask}"),
                move |state: &usize| target_contains(target_mask, *state),
            )
            .unwrap();
            let first = check_eventuality(&model, &property).unwrap();
            let second = check_eventuality(&model, &property).unwrap();
            assert_eq!(
                first, second,
                "determinism mask={mask} target={target_mask}"
            );

            let distance = residual_distances(mask, target_mask);
            let initial_is_target = target_contains(target_mask, 0);
            let residual_reachable = |node: usize| !initial_is_target && distance[0][node] < INF;
            let has_terminal = (0..N)
                .any(|node| residual_reachable(node) && (0..N).all(|to| !has_edge(mask, node, to)));
            let has_cycle = (0..N).any(|node| {
                residual_reachable(node)
                    && (has_edge(mask, node, node)
                        || (0..N).any(|other| {
                            other != node
                                && residual_reachable(other)
                                && distance[node][other] < INF
                                && distance[other][node] < INF
                        }))
            });
            let expected_violated = has_terminal || has_cycle;
            assert_eq!(
                first.status == EventualityStatus::Violated,
                expected_violated,
                "status mask={mask} target={target_mask}"
            );

            match first.counterexample {
                None => assert!(!expected_violated),
                Some(EventualityCounterexample::Finite { trace }) => {
                    assert!(has_terminal, "finite witness without terminal oracle");
                    validate_trace(mask, target_mask, &trace);
                    let end = trace.last().unwrap().state;
                    assert!(residual_reachable(end));
                    assert!((0..N).all(|to| !has_edge(mask, end, to)));
                    assert_eq!(trace.len() - 1, distance[0][end]);
                }
                Some(EventualityCounterexample::Infinite { stem, cycle }) => {
                    assert!(!has_terminal, "finite terminal must take precedence");
                    assert!(has_cycle, "cycle witness without cycle oracle");
                    validate_trace(mask, target_mask, &stem);
                    validate_trace(mask, target_mask, &cycle);
                    let entry = cycle.first().unwrap().state;
                    assert_eq!(cycle.last().unwrap().state, entry);
                    assert!(cycle.len() >= 2);
                    assert_eq!(stem.last().unwrap().state, entry);
                    assert_eq!(stem.len() - 1, distance[0][entry]);
                }
            }
        }
    }
}

fn validate_trace(
    mask: usize,
    target_mask: usize,
    trace: &[formal_verification_lab::TraceStep<usize>],
) {
    assert!(!trace.is_empty());
    for step in trace {
        assert!(!target_contains(target_mask, step.state));
    }
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

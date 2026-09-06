use formal_verification_lab::buchi::{check_buchi, FiniteRunPolicy};
use formal_verification_lab::buchi_examples::pulse_automaton;
use formal_verification_lab::fairness::{check_buchi_with_weak_fairness, WeakFairness};
use formal_verification_lab::{Invariant, StateVariable, Transition, TransitionSystem};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const ACTION_COUNT: usize = 4;

fn edge_index(from: usize, to: usize) -> usize {
    from * N + to
}

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << edge_index(from, to)) != 0
}

fn decode_actions(mut assignment: usize) -> [u8; EDGE_COUNT] {
    let mut actions = [0u8; EDGE_COUNT];
    for action in &mut actions {
        *action = (assignment % ACTION_COUNT) as u8;
        assignment /= ACTION_COUNT;
    }
    actions
}

fn action_label(code: u8) -> &'static str {
    match code {
        0 => "quiet",
        1 => "pulse-a",
        2 => "pulse-b",
        3 => "pulse-both",
        _ => unreachable!("generated action code is in range"),
    }
}

fn graph_model(mask: usize, actions: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("empty-fairness-compat-{mask}"),
        vec![StateVariable::new("node", "current graph node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                if has_edge(mask, *state, to) {
                    let edge = edge_index(*state, to);
                    next.push(Transition::new(action_label(actions[edge]), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("well-formed", |state: &usize| *state < N)],
    )
    .unwrap()
}

#[test]
fn empty_weak_fairness_matches_existing_buchi_result_for_all_two_node_products() {
    let fairness = WeakFairness::none();
    for graph_mask in 0usize..(1usize << EDGE_COUNT) {
        for assignment in 0usize..ACTION_COUNT.pow(EDGE_COUNT as u32) {
            let actions = decode_actions(assignment);
            for policy in [
                FiniteRunPolicy::IgnoreTerminals,
                FiniteRunPolicy::RequireAcceptingTerminal,
            ] {
                let model = graph_model(graph_mask, actions);
                let automaton = pulse_automaton(policy).unwrap();
                let ordinary = check_buchi(&model, &automaton).unwrap();
                let fair =
                    check_buchi_with_weak_fairness(&model, &automaton, &fairness).unwrap();
                assert_eq!(
                    fair, ordinary,
                    "graph={graph_mask} assignment={assignment} policy={policy:?}"
                );
            }
        }
    }
}

use formal_verification_lab::multi_response::{
    check_multi_response_with_fairness_profile,
    check_multi_response_with_fairness_profile_and_limits,
    check_multi_response_with_fairness_profile_and_product_limits, MultiObligationState,
    MultiResponseCounterexample, MultiResponseProperty, MultiResponseStatus, ResponseClause,
};
use formal_verification_lab::{
    check_buchi_with_fairness_profile, check_buchi_with_fairness_profile_and_limits,
    check_buchi_with_fairness_profile_and_product_limits, AcceptanceSet, AnalysisLimits,
    AnalysisOutcome, BoundedOutcome, BuchiAutomaton, BuchiCounterexample, BuchiProductState,
    BuchiStatus, ExplorationLimits, FairnessProfile, FiniteRunPolicy, Invariant, StateVariable,
    TraceStep, Transition, TransitionSystem,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const ACTION_COUNT: usize = 4;
const ASSIGNMENT_COUNT: usize = ACTION_COUNT.pow(EDGE_COUNT as u32);
const PRODUCT_LIMITS: [usize; 3] = [0, 2, 8];

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvidenceSignature {
    None,
    Finite {
        clause: String,
        trace: Vec<StepSignature>,
    },
    Infinite {
        clause: String,
        stem: Vec<StepSignature>,
        cycle: Vec<StepSignature>,
    },
}

type StepSignature = (Option<String>, usize, Vec<bool>);

fn edge_index(from: usize, to: usize) -> usize {
    from * N + to
}

fn has_edge(mask: usize, from: usize, to: usize) -> bool {
    mask & (1usize << edge_index(from, to)) != 0
}

fn decode_assignment(mut assignment: usize) -> [u8; EDGE_COUNT] {
    let mut codes = [0u8; EDGE_COUNT];
    for code in &mut codes {
        *code = (assignment % ACTION_COUNT) as u8;
        assignment /= ACTION_COUNT;
    }
    codes
}

fn action_for(code: u8) -> &'static str {
    match code {
        0 => "request-a",
        1 => "grant-a",
        2 => "request-b",
        3 => "grant-b",
        _ => unreachable!("generated action code is in range"),
    }
}

fn generated_model(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("combined-fair-multi-response-{graph_mask}"),
        vec![StateVariable::new("node", "generated graph node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                if has_edge(graph_mask, *state, to) {
                    let edge = edge_index(*state, to);
                    next.push(Transition::new(action_for(codes[edge]), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("node-domain", |state: &usize| *state < N)],
    )
    .unwrap()
}

fn property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "generated-dual-response",
        vec![
            ResponseClause::new(
                "class-a",
                |action| action == "request-a",
                |action| action == "grant-a",
            )
            .unwrap(),
            ResponseClause::new(
                "class-b",
                |action| action == "request-b",
                |action| action == "grant-b",
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn automaton() -> BuchiAutomaton<Vec<bool>> {
    BuchiAutomaton::new(
        "generated-dual-response-buchi",
        vec![false, false],
        |pending, action| {
            let mut next = pending.clone();
            match action {
                "grant-a" => next[0] = false,
                "request-a" => next[0] = true,
                "grant-b" => next[1] = false,
                "request-b" => next[1] = true,
                _ => unreachable!("generated action belongs to the four-code alphabet"),
            }
            next
        },
        vec![
            AcceptanceSet::new("class-a", |pending: &Vec<bool>| !pending[0]).unwrap(),
            AcceptanceSet::new("class-b", |pending: &Vec<bool>| !pending[1]).unwrap(),
        ],
        FiniteRunPolicy::RequireAcceptingTerminal,
    )
    .unwrap()
}

fn profile() -> FairnessProfile {
    FairnessProfile::new(["grant-a"], ["grant-b"]).unwrap()
}

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

fn staged_limits(index: usize) -> AnalysisLimits {
    match index {
        0 => AnalysisLimits {
            model: transition_limit(0),
            product: ExplorationLimits::unbounded(),
        },
        1 => AnalysisLimits {
            model: ExplorationLimits::unbounded(),
            product: transition_limit(2),
        },
        2 => AnalysisLimits {
            model: transition_limit(8),
            product: transition_limit(8),
        },
        _ => unreachable!("three staged limit profiles are defined"),
    }
}

fn multi_steps(trace: &[TraceStep<MultiObligationState<usize>>]) -> Vec<StepSignature> {
    trace
        .iter()
        .map(|step| {
            (
                step.action.clone(),
                step.state.state,
                step.state.pending.clone(),
            )
        })
        .collect()
}

fn buchi_steps(trace: &[TraceStep<BuchiProductState<usize, Vec<bool>>>]) -> Vec<StepSignature> {
    trace
        .iter()
        .map(|step| {
            (
                step.action.clone(),
                step.state.state,
                step.state.automaton.clone(),
            )
        })
        .collect()
}

fn multi_evidence(
    counterexample: &Option<MultiResponseCounterexample<usize>>,
) -> EvidenceSignature {
    match counterexample {
        None => EvidenceSignature::None,
        Some(MultiResponseCounterexample::Finite { clause, trace }) => EvidenceSignature::Finite {
            clause: clause.clone(),
            trace: multi_steps(trace),
        },
        Some(MultiResponseCounterexample::Infinite {
            clause,
            stem,
            cycle,
        }) => EvidenceSignature::Infinite {
            clause: clause.clone(),
            stem: multi_steps(stem),
            cycle: multi_steps(cycle),
        },
    }
}

fn buchi_evidence(counterexample: &Option<BuchiCounterexample<usize, Vec<bool>>>) -> EvidenceSignature {
    match counterexample {
        None => EvidenceSignature::None,
        Some(BuchiCounterexample::FiniteTerminal {
            missing_acceptance,
            trace,
        }) => EvidenceSignature::Finite {
            clause: missing_acceptance.clone(),
            trace: buchi_steps(trace),
        },
        Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        }) => EvidenceSignature::Infinite {
            clause: acceptance.clone(),
            stem: buchi_steps(stem),
            cycle: buchi_steps(cycle),
        },
    }
}

fn multi_status(status: BuchiStatus) -> MultiResponseStatus {
    match status {
        BuchiStatus::Satisfied => MultiResponseStatus::Satisfied,
        BuchiStatus::Violated => MultiResponseStatus::Violated,
    }
}

fn bounded_outcome(outcome: &BoundedOutcome<BuchiStatus>) -> BoundedOutcome<MultiResponseStatus> {
    match outcome {
        BoundedOutcome::Conclusive(status) => BoundedOutcome::Conclusive(multi_status(*status)),
        BoundedOutcome::Inconclusive(reason) => BoundedOutcome::Inconclusive(*reason),
    }
}

fn analysis_outcome(outcome: &AnalysisOutcome<BuchiStatus>) -> AnalysisOutcome<MultiResponseStatus> {
    match outcome {
        AnalysisOutcome::Conclusive(status) => AnalysisOutcome::Conclusive(multi_status(*status)),
        AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(*reason),
    }
}

#[test]
fn all_two_node_products_match_direct_combined_fair_buchi() {
    let property = property();
    let automaton = automaton();
    let profile = profile();

    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            let model = generated_model(graph_mask, codes);
            let multi =
                check_multi_response_with_fairness_profile(&model, &property, &profile).unwrap();
            let buchi = check_buchi_with_fairness_profile(&model, &automaton, &profile).unwrap();
            let context = format!("graph={graph_mask} assignment={assignment} codes={codes:?}");

            assert_eq!(multi.status, multi_status(buchi.status), "{context}");
            assert_eq!(multi.model_states, buchi.model_states, "{context}");
            assert_eq!(multi.model_transitions, buchi.model_transitions, "{context}");
            assert_eq!(multi.product_states, buchi.product_states, "{context}");
            assert_eq!(multi.product_transitions, buchi.product_transitions, "{context}");
            assert_eq!(
                multi_evidence(&multi.counterexample),
                buchi_evidence(&buchi.counterexample),
                "{context}"
            );
        }
    }
}

#[test]
fn all_two_node_product_budgets_match_direct_combined_fair_buchi() {
    let property = property();
    let automaton = automaton();
    let profile = profile();

    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            for limit in PRODUCT_LIMITS {
                let model = generated_model(graph_mask, codes);
                let limits = transition_limit(limit);
                let multi = check_multi_response_with_fairness_profile_and_product_limits(
                    &model, &property, &profile, limits,
                )
                .unwrap();
                let buchi = check_buchi_with_fairness_profile_and_product_limits(
                    &model, &automaton, &profile, limits,
                )
                .unwrap();
                let context = format!(
                    "graph={graph_mask} assignment={assignment} codes={codes:?} limit={limit}"
                );

                assert_eq!(multi.outcome, bounded_outcome(&buchi.outcome), "{context}");
                assert_eq!(multi.model_states, buchi.model_states, "{context}");
                assert_eq!(multi.model_transitions, buchi.model_transitions, "{context}");
                assert_eq!(multi.product_states, buchi.product_states, "{context}");
                assert_eq!(
                    multi.checked_product_states, buchi.checked_product_states,
                    "{context}"
                );
                assert_eq!(
                    multi.explored_product_transitions,
                    buchi.explored_product_transitions,
                    "{context}"
                );
                assert_eq!(
                    multi.retained_product_transitions,
                    buchi.retained_product_transitions,
                    "{context}"
                );
                assert_eq!(
                    multi.max_product_depth_reached,
                    buchi.max_product_depth_reached,
                    "{context}"
                );
                assert_eq!(
                    multi_evidence(&multi.counterexample),
                    buchi_evidence(&buchi.counterexample),
                    "{context}"
                );
            }
        }
    }
}

#[test]
fn all_two_node_staged_budgets_match_direct_combined_fair_buchi() {
    let property = property();
    let automaton = automaton();
    let profile = profile();

    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            for limit_index in 0..3 {
                let model = generated_model(graph_mask, codes);
                let limits = staged_limits(limit_index);
                let multi = check_multi_response_with_fairness_profile_and_limits(
                    &model, &property, &profile, limits,
                )
                .unwrap();
                let buchi = check_buchi_with_fairness_profile_and_limits(
                    &model, &automaton, &profile, limits,
                )
                .unwrap();
                let context = format!(
                    "graph={graph_mask} assignment={assignment} codes={codes:?} profile={limit_index}"
                );

                assert_eq!(multi.outcome, analysis_outcome(&buchi.outcome), "{context}");
                assert_eq!(multi.model_completion, buchi.model_completion, "{context}");
                assert_eq!(multi.product_completion, buchi.product_completion, "{context}");
                assert_eq!(multi.model_states, buchi.model_states, "{context}");
                assert_eq!(multi.checked_model_states, buchi.checked_model_states, "{context}");
                assert_eq!(
                    multi.explored_model_transitions,
                    buchi.explored_model_transitions,
                    "{context}"
                );
                assert_eq!(
                    multi.retained_model_transitions,
                    buchi.retained_model_transitions,
                    "{context}"
                );
                assert_eq!(
                    multi.max_model_depth_reached,
                    buchi.max_model_depth_reached,
                    "{context}"
                );
                assert_eq!(multi.product_states, buchi.product_states, "{context}");
                assert_eq!(
                    multi.checked_product_states, buchi.checked_product_states,
                    "{context}"
                );
                assert_eq!(
                    multi.explored_product_transitions,
                    buchi.explored_product_transitions,
                    "{context}"
                );
                assert_eq!(
                    multi.retained_product_transitions,
                    buchi.retained_product_transitions,
                    "{context}"
                );
                assert_eq!(
                    multi.max_product_depth_reached,
                    buchi.max_product_depth_reached,
                    "{context}"
                );
                assert_eq!(
                    multi_evidence(&multi.counterexample),
                    buchi_evidence(&buchi.counterexample),
                    "{context}"
                );
            }
        }
    }
}

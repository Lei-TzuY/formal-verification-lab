use formal_verification_lab::{
    check_buchi_with_fairness_profile_and_limits, check_monitor_with_fairness_profile_and_limits,
    AcceptanceSet, AnalysisLimits, AnalysisOutcome, BuchiAutomaton, BuchiCounterexample,
    BuchiProductState, BuchiStatus, ExplorationLimits, FairnessProfile, FiniteMonitor,
    FiniteRunPolicy, Invariant, MonitorCounterexample, MonitorProductState, MonitorStatus,
    ProgressCondition, RejectCondition, StateVariable, TraceStep, Transition, TransitionSystem,
};

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const ACTION_COUNT: usize = 4;
const ASSIGNMENT_COUNT: usize = ACTION_COUNT.pow(EDGE_COUNT as u32);

type StepSignature = (Option<String>, usize, bool);

#[derive(Debug, Clone, PartialEq, Eq)]
enum EvidenceSignature {
    None,
    Finite {
        condition: String,
        trace: Vec<StepSignature>,
    },
    Cycle {
        condition: String,
        stem: Vec<StepSignature>,
        cycle: Vec<StepSignature>,
    },
}

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
        0 => "other",
        1 => "clear",
        2 => "weak",
        3 => "strong",
        _ => unreachable!("generated action code is in range"),
    }
}

fn generated_model(graph_mask: usize, codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("combined-fair-monitor-staged-differential-{graph_mask}"),
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
        vec![Invariant::new("generated-node-domain", |state: &usize| {
            *state < N
        })],
    )
    .unwrap()
}

fn generated_monitor() -> FiniteMonitor<bool> {
    FiniteMonitor::new(
        "generated-combined-fair-staged-monitor",
        true,
        |state, action| if action == "clear" { false } else { *state },
        Vec::<RejectCondition<bool>>::new(),
        vec![ProgressCondition::new("active-eventually-clears", |state| *state).unwrap()],
    )
    .unwrap()
}

fn generated_buchi() -> BuchiAutomaton<bool> {
    BuchiAutomaton::new(
        "generated-combined-fair-staged-buchi",
        true,
        |state: &bool, action| if action == "clear" { false } else { *state },
        vec![AcceptanceSet::new("active-eventually-clears", |state: &bool| !*state).unwrap()],
        FiniteRunPolicy::RequireAcceptingTerminal,
    )
    .unwrap()
}

fn fairness_profile() -> FairnessProfile {
    FairnessProfile::new(["weak"], ["strong"]).unwrap()
}

fn transition_limit(limit: usize) -> ExplorationLimits {
    ExplorationLimits {
        max_states: None,
        max_transitions: Some(limit),
        max_depth: None,
    }
}

fn staged_limits() -> [AnalysisLimits; 5] {
    [
        AnalysisLimits::new(transition_limit(0), ExplorationLimits::unbounded()),
        AnalysisLimits::new(transition_limit(2), ExplorationLimits::unbounded()),
        AnalysisLimits::new(ExplorationLimits::unbounded(), transition_limit(0)),
        AnalysisLimits::new(transition_limit(2), transition_limit(2)),
        AnalysisLimits::new(transition_limit(8), transition_limit(8)),
    ]
}

fn monitor_status(status: BuchiStatus) -> MonitorStatus {
    match status {
        BuchiStatus::Satisfied => MonitorStatus::Satisfied,
        BuchiStatus::Violated => MonitorStatus::Violated,
    }
}

fn monitor_outcome(outcome: &AnalysisOutcome<BuchiStatus>) -> AnalysisOutcome<MonitorStatus> {
    match outcome {
        AnalysisOutcome::Conclusive(status) => AnalysisOutcome::Conclusive(monitor_status(*status)),
        AnalysisOutcome::Inconclusive(reason) => AnalysisOutcome::Inconclusive(*reason),
    }
}

fn monitor_steps(trace: &[TraceStep<MonitorProductState<usize, bool>>]) -> Vec<StepSignature> {
    trace
        .iter()
        .map(|step| (step.action.clone(), step.state.state, step.state.monitor))
        .collect()
}

fn buchi_steps(trace: &[TraceStep<BuchiProductState<usize, bool>>]) -> Vec<StepSignature> {
    trace
        .iter()
        .map(|step| (step.action.clone(), step.state.state, step.state.automaton))
        .collect()
}

fn monitor_evidence(
    counterexample: &Option<MonitorCounterexample<usize, bool>>,
) -> EvidenceSignature {
    match counterexample {
        None => EvidenceSignature::None,
        Some(MonitorCounterexample::Rejecting { .. }) => {
            panic!("generated monitor has no rejecting conditions")
        }
        Some(MonitorCounterexample::ProgressTerminal { condition, trace }) => {
            EvidenceSignature::Finite {
                condition: condition.clone(),
                trace: monitor_steps(trace),
            }
        }
        Some(MonitorCounterexample::ProgressCycle {
            condition,
            stem,
            cycle,
        }) => EvidenceSignature::Cycle {
            condition: condition.clone(),
            stem: monitor_steps(stem),
            cycle: monitor_steps(cycle),
        },
    }
}

fn buchi_evidence(counterexample: &Option<BuchiCounterexample<usize, bool>>) -> EvidenceSignature {
    match counterexample {
        None => EvidenceSignature::None,
        Some(BuchiCounterexample::FiniteTerminal {
            missing_acceptance,
            trace,
        }) => EvidenceSignature::Finite {
            condition: missing_acceptance.clone(),
            trace: buchi_steps(trace),
        },
        Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        }) => EvidenceSignature::Cycle {
            condition: acceptance.clone(),
            stem: buchi_steps(stem),
            cycle: buchi_steps(cycle),
        },
    }
}

#[test]
fn all_two_node_staged_budgets_match_direct_combined_fair_buchi() {
    let monitor = generated_monitor();
    let automaton = generated_buchi();
    let profile = fairness_profile();

    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            for (limits_index, limits) in staged_limits().into_iter().enumerate() {
                let model = generated_model(graph_mask, codes);
                let monitor_result = check_monitor_with_fairness_profile_and_limits(
                    &model, &monitor, &profile, limits,
                )
                .unwrap();
                let buchi_result = check_buchi_with_fairness_profile_and_limits(
                    &model, &automaton, &profile, limits,
                )
                .unwrap();
                let context = format!(
                    "graph={graph_mask} assignment={assignment} codes={codes:?} limits={limits_index}"
                );

                assert_eq!(
                    monitor_result.outcome,
                    monitor_outcome(&buchi_result.outcome),
                    "{context}"
                );
                assert_eq!(
                    monitor_result.model_completion, buchi_result.model_completion,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.product_completion, buchi_result.product_completion,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.model_states, buchi_result.model_states,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.checked_model_states, buchi_result.checked_model_states,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.explored_model_transitions,
                    buchi_result.explored_model_transitions,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.retained_model_transitions,
                    buchi_result.retained_model_transitions,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.max_model_depth_reached, buchi_result.max_model_depth_reached,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.product_states, buchi_result.product_states,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.checked_product_states, buchi_result.checked_product_states,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.explored_product_transitions,
                    buchi_result.explored_product_transitions,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.retained_product_transitions,
                    buchi_result.retained_product_transitions,
                    "{context}"
                );
                assert_eq!(
                    monitor_result.max_product_depth_reached,
                    buchi_result.max_product_depth_reached,
                    "{context}"
                );
                assert_eq!(
                    monitor_evidence(&monitor_result.counterexample),
                    buchi_evidence(&buchi_result.counterexample),
                    "{context}"
                );
            }
        }
    }
}

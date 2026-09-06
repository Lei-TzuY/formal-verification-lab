use formal_verification_lab::{
    check_eventuality_with_limits, check_exact_state_property_with_limits,
    check_proposition_expression_property_with_limits, check_proposition_property_with_limits,
    check_reachability_with_limits, check_safety_assertion_with_limits, parse_declarative_document,
    parse_declarative_model, parse_proposition_expression, BoundedOutcome,
    EventualityCounterexample, EventualityProperty, EventualityStatus, ExactStatePropertySpec,
    ExplorationLimits, InconclusiveReason, PropositionExpressionPropertySpec,
    PropositionPropertySpec, PropositionSafetySpec, ReachabilityProperty, ReachabilityStatus,
    SafetyStatus,
};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleSearch {
    complete: bool,
    reason: Option<InconclusiveReason>,
    discovered: [bool; 2],
    depths: [Option<usize>; 2],
    checked: usize,
    explored_transitions: usize,
    observed: [[bool; 2]; 2],
    known_terminal: [bool; 2],
}

impl OracleSearch {
    fn discovered_count(&self) -> usize {
        self.discovered.iter().filter(|value| **value).count()
    }
}

fn cutoff_reason(
    limits: ExplorationLimits,
    state_count: usize,
    explored_transitions: usize,
    depth: usize,
    next_is_new: bool,
) -> Option<InconclusiveReason> {
    if let Some(limit) = limits
        .max_transitions
        .filter(|limit| explored_transitions >= *limit)
    {
        return Some(InconclusiveReason::TransitionLimitReached { limit });
    }
    if next_is_new {
        if let Some(limit) = limits.max_depth.filter(|limit| depth >= *limit) {
            return Some(InconclusiveReason::DepthLimitReached { limit });
        }
        if let Some(limit) = limits.max_states.filter(|limit| state_count >= *limit) {
            return Some(InconclusiveReason::StateLimitReached { limit });
        }
    }
    None
}

fn graph_edges(mask: u8, from: usize) -> Vec<usize> {
    (0..2)
        .filter(|to| mask & (1 << (from * 2 + to)) != 0)
        .collect()
}

fn oracle_capture(mask: u8, limits: ExplorationLimits) -> OracleSearch {
    let mut result = OracleSearch {
        complete: false,
        reason: None,
        discovered: [false; 2],
        depths: [None; 2],
        checked: 0,
        explored_transitions: 0,
        observed: [[false; 2]; 2],
        known_terminal: [false; 2],
    };

    if let Some(limit) = limits.max_states.filter(|limit| *limit == 0) {
        result.reason = Some(InconclusiveReason::StateLimitReached { limit });
        return result;
    }

    result.discovered[0] = true;
    result.depths[0] = Some(0);
    let mut queue = VecDeque::from([0_usize]);

    while let Some(from) = queue.pop_front() {
        result.checked += 1;
        let depth = result.depths[from].unwrap();
        let edges = graph_edges(mask, from);
        result.known_terminal[from] = edges.is_empty();

        for to in edges {
            let next_is_new = !result.discovered[to];
            if let Some(reason) = cutoff_reason(
                limits,
                result.discovered_count(),
                result.explored_transitions,
                depth,
                next_is_new,
            ) {
                if matches!(reason, InconclusiveReason::TransitionLimitReached { .. }) {
                    result.reason = Some(reason);
                    return result;
                }
                result.explored_transitions += 1;
                result.reason = Some(reason);
                return result;
            }

            result.explored_transitions += 1;
            result.observed[from][to] = true;
            if next_is_new {
                result.discovered[to] = true;
                result.depths[to] = Some(depth + 1);
                queue.push_back(to);
            }
        }
    }

    result.complete = true;
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleReachability {
    outcome: BoundedOutcome<ReachabilityStatus>,
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    max_depth: Option<usize>,
    witness_distance: Option<usize>,
}

fn oracle_reachability(mask: u8, target_mask: u8, limits: ExplorationLimits) -> OracleReachability {
    let mut discovered = [false; 2];
    let mut depths = [None; 2];
    let mut checked = 0_usize;
    let mut explored = 0_usize;

    if let Some(limit) = limits.max_states.filter(|limit| *limit == 0) {
        return OracleReachability {
            outcome: BoundedOutcome::Inconclusive(InconclusiveReason::StateLimitReached { limit }),
            discovered_states: 0,
            checked_states: 0,
            explored_transitions: 0,
            max_depth: None,
            witness_distance: None,
        };
    }

    discovered[0] = true;
    depths[0] = Some(0);
    let mut queue = VecDeque::from([0_usize]);

    while let Some(from) = queue.pop_front() {
        checked += 1;
        let depth = depths[from].unwrap();
        if target_mask & (1 << from) != 0 {
            return OracleReachability {
                outcome: BoundedOutcome::Conclusive(ReachabilityStatus::Reachable),
                discovered_states: discovered.iter().filter(|value| **value).count(),
                checked_states: checked,
                explored_transitions: explored,
                max_depth: depths.iter().flatten().copied().max(),
                witness_distance: Some(depth),
            };
        }

        for to in graph_edges(mask, from) {
            let next_is_new = !discovered[to];
            if let Some(reason) = cutoff_reason(
                limits,
                discovered.iter().filter(|value| **value).count(),
                explored,
                depth,
                next_is_new,
            ) {
                if !matches!(reason, InconclusiveReason::TransitionLimitReached { .. }) {
                    explored += 1;
                }
                return OracleReachability {
                    outcome: BoundedOutcome::Inconclusive(reason),
                    discovered_states: discovered.iter().filter(|value| **value).count(),
                    checked_states: checked,
                    explored_transitions: explored,
                    max_depth: depths.iter().flatten().copied().max(),
                    witness_distance: None,
                };
            }

            explored += 1;
            if next_is_new {
                discovered[to] = true;
                depths[to] = Some(depth + 1);
                queue.push_back(to);
            }
        }
    }

    OracleReachability {
        outcome: BoundedOutcome::Conclusive(ReachabilityStatus::Unreachable),
        discovered_states: discovered.iter().filter(|value| **value).count(),
        checked_states: checked,
        explored_transitions: explored,
        max_depth: depths.iter().flatten().copied().max(),
        witness_distance: None,
    }
}

fn oracle_eventuality(
    mask: u8,
    target_mask: u8,
    limits: ExplorationLimits,
) -> BoundedOutcome<EventualityStatus> {
    let captured = oracle_capture(mask, limits);
    let mut residual = [false; 2];
    let mut queue = VecDeque::new();
    if captured.discovered[0] && target_mask & 1 == 0 {
        residual[0] = true;
        queue.push_back(0_usize);
    }
    while let Some(from) = queue.pop_front() {
        for (to, reachable) in residual.iter_mut().enumerate() {
            if captured.observed[from][to] && target_mask & (1 << to) == 0 && !*reachable {
                *reachable = true;
                queue.push_back(to);
            }
        }
    }

    if (0..2).any(|state| residual[state] && captured.known_terminal[state]) {
        return BoundedOutcome::Conclusive(EventualityStatus::Violated);
    }

    let mut path = captured.observed;
    for mid in 0..2 {
        for from in 0..2 {
            for to in 0..2 {
                if residual[from] && residual[mid] && residual[to] {
                    path[from][to] |= path[from][mid] && path[mid][to];
                }
            }
        }
    }
    if (0..2).any(|state| residual[state] && path[state][state]) {
        return BoundedOutcome::Conclusive(EventualityStatus::Violated);
    }

    if captured.complete {
        BoundedOutcome::Conclusive(EventualityStatus::Satisfied)
    } else {
        BoundedOutcome::Inconclusive(captured.reason.unwrap())
    }
}

fn limit_profiles() -> Vec<ExplorationLimits> {
    vec![
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
            max_transitions: Some(2),
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
            max_states: Some(1),
            max_transitions: Some(1),
            max_depth: Some(0),
        },
    ]
}

fn generated_model(mask: u8) -> formal_verification_lab::TransitionSystem<String> {
    let mut input =
        String::from("model \"bounded-oracle\"\nstate \"s0\"\nstate \"s1\"\ninitial \"s0\"\n");
    for from in 0..2 {
        for to in 0..2 {
            if mask & (1 << (from * 2 + to)) != 0 {
                input.push_str(&format!("edge \"s{from}\" \"e{from}{to}\" \"s{to}\"\n"));
            }
        }
    }
    parse_declarative_model(&input).unwrap()
}

#[test]
fn generated_bounded_backend_oracle_matches_reachability_and_eventuality() {
    let limits = limit_profiles();
    let mut reachability_cases = 0_usize;
    let mut eventuality_cases = 0_usize;

    for mask in 0_u8..16 {
        for target_mask in 0_u8..4 {
            for &profile in &limits {
                let model = generated_model(mask);
                let expected = oracle_reachability(mask, target_mask, profile);
                let property = ReachabilityProperty::new("bounded-reach", move |state: &String| {
                    let index = state.strip_prefix('s').unwrap().parse::<u8>().unwrap();
                    target_mask & (1 << index) != 0
                })
                .unwrap();
                let actual = check_reachability_with_limits(&model, &property, profile).unwrap();
                assert_eq!(
                    actual.outcome, expected.outcome,
                    "reach mask={mask} target={target_mask} limits={profile:?}"
                );
                assert_eq!(actual.discovered_states, expected.discovered_states);
                assert_eq!(actual.checked_states, expected.checked_states);
                assert_eq!(actual.explored_transitions, expected.explored_transitions);
                assert_eq!(actual.max_depth_reached, expected.max_depth);
                match (actual.witness.as_ref(), expected.witness_distance) {
                    (Some(trace), Some(distance)) => assert_eq!(trace.len() - 1, distance),
                    (None, None) => {}
                    other => panic!("witness mismatch: {other:?}"),
                }
                reachability_cases += 1;

                let model = generated_model(mask);
                let property =
                    EventualityProperty::new("bounded-eventually", move |state: &String| {
                        let index = state.strip_prefix('s').unwrap().parse::<u8>().unwrap();
                        target_mask & (1 << index) != 0
                    })
                    .unwrap();
                let actual = check_eventuality_with_limits(&model, &property, profile).unwrap();
                let expected = oracle_eventuality(mask, target_mask, profile);
                assert_eq!(
                    actual.outcome, expected,
                    "eventuality mask={mask} target={target_mask} limits={profile:?}"
                );
                if let BoundedOutcome::Conclusive(EventualityStatus::Violated) = actual.outcome {
                    assert!(actual.counterexample.is_some());
                }
                if matches!(actual.outcome, BoundedOutcome::Inconclusive(_)) {
                    assert!(actual.counterexample.is_none());
                }
                eventuality_cases += 1;
            }
        }
    }

    assert_eq!(reachability_cases, 640);
    assert_eq!(eventuality_cases, 640);
}

#[test]
fn bounded_eventuality_can_prove_real_counterexamples_before_global_completion() {
    let cycle = parse_declarative_model(
        r#"
model "partial-cycle"
state "start"
state "escape"
initial "start"
edge "start" "spin" "start"
edge "start" "escape" "escape"
"#,
    )
    .unwrap();
    let property = EventualityProperty::new("eventually-never", |_state: &String| false).unwrap();
    let result = check_eventuality_with_limits(
        &cycle,
        &property,
        ExplorationLimits {
            max_transitions: Some(1),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();
    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(EventualityStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(EventualityCounterexample::Infinite { .. })
    ));

    let terminal = parse_declarative_model(
        r#"
model "partial-terminal"
state "terminal"
initial "terminal"
"#,
    )
    .unwrap();
    let result = check_eventuality_with_limits(
        &terminal,
        &property,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();
    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(EventualityStatus::Violated)
    );
    assert!(matches!(
        result.counterexample,
        Some(EventualityCounterexample::Finite { .. })
    ));
}

const FRONTEND_MODEL: &str = r#"
model "bounded-frontends"
state "start"
state "good"
state "done"
initial "start"
edge "start" "advance" "good"
edge "good" "finish" "done"
label "start" "ok"
label "good" "ok"
label "done" "complete"
"#;

#[test]
fn bounded_frontends_propagate_inconclusive_without_fabricating_evidence() {
    let document = parse_declarative_document(FRONTEND_MODEL).unwrap();
    let limits = ExplorationLimits {
        max_depth: Some(0),
        ..ExplorationLimits::unbounded()
    };

    let exact = ExactStatePropertySpec::reachable("reach-done", "done").unwrap();
    let result = check_exact_state_property_with_limits(document.model(), &exact, limits).unwrap();
    assert!(matches!(
        result.outcome,
        BoundedOutcome::Inconclusive(InconclusiveReason::DepthLimitReached { limit: 0 })
    ));
    assert!(result.evidence.is_none());

    let proposition = PropositionPropertySpec::reachable("reach-complete", "complete").unwrap();
    let result = check_proposition_property_with_limits(&document, &proposition, limits).unwrap();
    assert!(matches!(result.outcome, BoundedOutcome::Inconclusive(_)));
    assert!(result.evidence.is_none());

    let expression = parse_proposition_expression(r#""complete" or not "ok""#).unwrap();
    let expression_spec =
        PropositionExpressionPropertySpec::reachable("reach-expression", expression).unwrap();
    let result =
        check_proposition_expression_property_with_limits(&document, &expression_spec, limits)
            .unwrap();
    assert!(matches!(result.outcome, BoundedOutcome::Inconclusive(_)));
    assert!(result.evidence.is_none());

    let expression = parse_proposition_expression(r#""ok" or "complete""#).unwrap();
    let safety = PropositionSafetySpec::always("always-classified", expression).unwrap();
    let result = check_safety_assertion_with_limits(&document, &safety, limits).unwrap();
    assert!(matches!(result.outcome, BoundedOutcome::Inconclusive(_)));
    assert!(result.counterexample.is_none());
}

#[test]
fn bounded_safety_remains_conclusive_when_a_falsifier_is_found_before_cutoff() {
    let document = parse_declarative_document(
        r#"
model "bounded-safety-hit"
state "bad"
state "later"
initial "bad"
edge "bad" "later" "later"
label "later" "ok"
"#,
    )
    .unwrap();
    let expression = parse_proposition_expression(r#""ok""#).unwrap();
    let safety = PropositionSafetySpec::always("always-ok", expression).unwrap();
    let result = check_safety_assertion_with_limits(
        &document,
        &safety,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    )
    .unwrap();
    assert_eq!(
        result.outcome,
        BoundedOutcome::Conclusive(SafetyStatus::Violated)
    );
    let trace = result.counterexample.unwrap();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].state, "bad");
}

fn temp_model_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fvlab-bounded-properties-{}-{nonce}.model",
        std::process::id()
    ))
}

#[test]
fn state_and_proposition_cli_expose_honest_inconclusive_exit_three() {
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let path = temp_model_path();
    fs::write(&path, FRONTEND_MODEL).unwrap();

    let state = Command::new(binary)
        .args([
            "state",
            "file",
            path.to_str().unwrap(),
            r#"reachable("done")"#,
            "--max-depth",
            "0",
        ])
        .output()
        .unwrap();
    assert_eq!(state.status.code(), Some(3));
    let stdout = String::from_utf8(state.stdout).unwrap();
    assert!(stdout.contains("state property: INCONCLUSIVE"));
    assert!(stdout.contains("depth limit reached"));

    let safety = Command::new(binary)
        .args([
            "proposition",
            "always",
            path.to_str().unwrap(),
            r#""ok" or "complete""#,
            "--max-depth",
            "0",
        ])
        .output()
        .unwrap();
    assert_eq!(safety.status.code(), Some(3));
    let stdout = String::from_utf8(safety.stdout).unwrap();
    assert!(stdout.contains("safety: INCONCLUSIVE"));
    assert!(stdout.contains("depth limit reached"));

    fs::remove_file(path).unwrap();
}

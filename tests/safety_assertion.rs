use formal_verification_lab::{
    check_reachability, check_safety_assertion, parse_declarative_document,
    parse_proposition_expression, PropositionExpressionError, PropositionSafetySpec,
    ReachabilityProperty, ReachabilityStatus, SafetyError, SafetyStatus,
};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const SAFE_MODEL: &str = r#"
model "safe-model"
state "start"
state "working"
state "done"
initial "start"
edge "start" "begin" "working"
edge "working" "finish" "done"
label "start" "ok"
label "working" "ok"
label "done" "ok"
"#;

const VIOLATING_MODEL: &str = r#"
model "violating-model"
state "start"
state "working"
state "bad"
initial "start"
edge "start" "work" "working"
edge "start" "fail" "bad"
label "start" "ok"
label "working" "ok"
label "bad" "error"
"#;

#[test]
fn safety_assertion_matches_complement_reachability_and_shortest_counterexample() {
    let safe = parse_declarative_document(SAFE_MODEL).unwrap();
    let expression = parse_proposition_expression(r#""ok""#).unwrap();
    let result = check_safety_assertion(
        &safe,
        &PropositionSafetySpec::always("all-states-ok", expression).unwrap(),
    )
    .unwrap();
    let complement = ReachabilityProperty::new("not-ok", |state: &String| false).unwrap();
    let direct = check_reachability(safe.model(), &complement).unwrap();

    assert_eq!(direct.status, ReachabilityStatus::Unreachable);
    assert_eq!(result.status, SafetyStatus::Safe);
    assert_eq!(result.discovered_states, direct.discovered_states);
    assert_eq!(result.explored_transitions, direct.explored_transitions);
    assert_eq!(result.max_depth_reached, direct.max_depth_reached);
    assert!(result.counterexample.is_none());

    let violating = parse_declarative_document(VIOLATING_MODEL).unwrap();
    let expression = parse_proposition_expression(r#""ok""#).unwrap();
    let result = check_safety_assertion(
        &violating,
        &PropositionSafetySpec::always("all-states-ok", expression).unwrap(),
    )
    .unwrap();
    let direct = ReachabilityProperty::new("not-ok", |state: &String| state == "bad").unwrap();
    let direct = check_reachability(violating.model(), &direct).unwrap();

    assert_eq!(direct.status, ReachabilityStatus::Reachable);
    assert_eq!(result.status, SafetyStatus::Violated);
    assert_eq!(result.discovered_states, direct.discovered_states);
    assert_eq!(result.explored_transitions, direct.explored_transitions);
    assert_eq!(result.max_depth_reached, direct.max_depth_reached);
    assert_eq!(result.counterexample, direct.witness);
    let trace = result.counterexample.unwrap();
    assert_eq!(trace.len(), 2);
    assert_eq!(trace[0].state, "start");
    assert_eq!(trace[1].action.as_deref(), Some("fail"));
    assert_eq!(trace[1].state, "bad");
}

#[test]
fn initial_state_violation_is_zero_transition_shortest_counterexample() {
    let document = parse_declarative_document(
        r#"
model "initial-violation"
state "start"
state "good"
initial "start"
label "good" "ok"
"#,
    )
    .unwrap();
    let expression = parse_proposition_expression(r#""ok""#).unwrap();
    let result = check_safety_assertion(
        &document,
        &PropositionSafetySpec::always("initial-must-be-ok", expression).unwrap(),
    )
    .unwrap();

    assert_eq!(result.status, SafetyStatus::Violated);
    let trace = result.counterexample.unwrap();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].state, "start");
    assert!(trace[0].action.is_none());
}

#[test]
fn unknown_proposition_fails_closed_before_boolean_short_circuiting() {
    let document = parse_declarative_document(SAFE_MODEL).unwrap();
    let expression = parse_proposition_expression(r#""ok" or "missing""#).unwrap();
    let error = check_safety_assertion(
        &document,
        &PropositionSafetySpec::always("unknown-ref", expression).unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error,
        SafetyError::PropositionExpression(PropositionExpressionError::UnknownProposition {
            proposition: "missing".to_owned(),
        })
    );
}

fn generated_document(graph_mask: u16, proposition_mask: u8) -> formal_verification_lab::DeclarativeDocument {
    let mut input = String::from("model \"generated-safety\"\n");
    for state in 0..3 {
        input.push_str(&format!("state \"s{state}\"\n"));
    }
    input.push_str("initial \"s0\"\n");
    for from in 0..3 {
        for to in 0..3 {
            let edge = from * 3 + to;
            if graph_mask & (1 << edge) != 0 {
                input.push_str(&format!(
                    "edge \"s{from}\" \"e{from}{to}\" \"s{to}\"\n"
                ));
            }
        }
    }
    for state in 0..3 {
        if proposition_mask & (1 << state) != 0 {
            input.push_str(&format!("label \"s{state}\" \"p\"\n"));
        }
    }
    parse_declarative_document(&input).unwrap()
}

fn independent_shortest_violation(graph_mask: u16, proposition_mask: u8) -> Option<usize> {
    let mut distance = [None; 3];
    let mut queue = VecDeque::new();
    distance[0] = Some(0_usize);
    queue.push_back(0_usize);

    while let Some(from) = queue.pop_front() {
        let from_distance = distance[from].unwrap();
        for to in 0..3 {
            let edge = from * 3 + to;
            if graph_mask & (1 << edge) != 0 && distance[to].is_none() {
                distance[to] = Some(from_distance + 1);
                queue.push_back(to);
            }
        }
    }

    (0..3)
        .filter(|state| proposition_mask & (1 << state) == 0)
        .filter_map(|state| distance[state])
        .min()
}

fn state_index(state: &str) -> usize {
    state
        .strip_prefix('s')
        .expect("generated state has s prefix")
        .parse()
        .expect("generated state suffix is numeric")
}

#[test]
fn exhaustive_three_state_graph_oracle_matches_safety_semantics() {
    let expression = parse_proposition_expression(r#""p""#).unwrap();
    let mut checked = 0_usize;

    for graph_mask in 0_u16..512 {
        for proposition_mask in 1_u8..8 {
            let document = generated_document(graph_mask, proposition_mask);
            let spec = PropositionSafetySpec::always("always-p", expression.clone()).unwrap();
            let result = check_safety_assertion(&document, &spec).unwrap();
            let repeated = check_safety_assertion(&document, &spec).unwrap();
            assert_eq!(result, repeated);

            match independent_shortest_violation(graph_mask, proposition_mask) {
                None => {
                    assert_eq!(result.status, SafetyStatus::Safe);
                    assert!(result.counterexample.is_none());
                }
                Some(distance) => {
                    assert_eq!(result.status, SafetyStatus::Violated);
                    let trace = result.counterexample.as_ref().unwrap();
                    assert_eq!(trace.len() - 1, distance);
                    let final_state = state_index(&trace.last().unwrap().state);
                    assert_eq!(proposition_mask & (1 << final_state), 0);

                    for pair in trace.windows(2) {
                        let from = state_index(&pair[0].state);
                        let to = state_index(&pair[1].state);
                        let edge = from * 3 + to;
                        assert_ne!(graph_mask & (1 << edge), 0);
                        assert_eq!(pair[1].action.as_deref(), Some(format!("e{from}{to}").as_str()));
                    }
                }
            }
            checked += 1;
        }
    }

    assert_eq!(checked, 3_584);
}

fn temp_model_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fvlab-safety-{label}-{}-{nonce}.model",
        std::process::id()
    ))
}

#[test]
fn safety_cli_is_end_to_end_and_preserves_boolean_frontend_compatibility() {
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let safe_path = temp_model_path("safe");
    let violating_path = temp_model_path("violating");
    fs::write(&safe_path, SAFE_MODEL).unwrap();
    fs::write(&violating_path, VIOLATING_MODEL).unwrap();

    let safe = Command::new(binary)
        .args([
            "proposition",
            "always",
            safe_path.to_str().unwrap(),
            r#""ok""#,
        ])
        .output()
        .unwrap();
    assert!(safe.status.success());
    let stdout = String::from_utf8(safe.stdout).unwrap();
    assert!(stdout.contains("safety: SAFE"));
    assert!(stdout.contains("evidence: none"));

    let violated = Command::new(binary)
        .args([
            "proposition",
            "always",
            violating_path.to_str().unwrap(),
            r#""ok""#,
        ])
        .output()
        .unwrap();
    assert_eq!(violated.status.code(), Some(12));
    let stdout = String::from_utf8(violated.stdout).unwrap();
    assert!(stdout.contains("safety: VIOLATED"));
    assert!(stdout.contains("evidence: SAFETY_COUNTEREXAMPLE"));
    assert!(stdout.contains("--fail--> \"bad\""));

    let malformed = Command::new(binary)
        .args([
            "proposition",
            "always",
            safe_path.to_str().unwrap(),
            r#""ok" and"#,
        ])
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8(malformed.stderr).unwrap();
    assert!(stderr.contains("proposition expression parse error"));

    let previous_frontend = Command::new(binary)
        .args([
            "proposition",
            "expr",
            safe_path.to_str().unwrap(),
            "reachable",
            r#""ok""#,
        ])
        .output()
        .unwrap();
    assert!(previous_frontend.status.success());

    fs::remove_file(safe_path).unwrap();
    fs::remove_file(violating_path).unwrap();
}

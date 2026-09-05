use formal_verification_lab::{
    check_eventuality, check_proposition_expression_property, check_reachability,
    parse_declarative_document, parse_proposition_expression, EventualityCounterexample,
    EventualityProperty, EventualityStatus, ExactStateBackend, ExactStateEvidence, ExactStateStatus,
    PropositionExpression, PropositionExpressionError, PropositionExpressionParseErrorKind,
    PropositionExpressionPropertySpec, ReachabilityProperty, ReachabilityStatus,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const LABELED_GRAPH: &str = r#"
model "boolean-propositions"
state "start"
state "critical-a"
state "critical-error"
state "done"
initial "start"
edge "start" "choose-a" "critical-a"
edge "start" "choose-error" "critical-error"
edge "critical-a" "finish-a" "done"
edge "critical-error" "recover" "done"
label "critical-a" "critical"
label "critical-error" "critical"
label "critical-error" "error"
label "done" "complete"
"#;

const AVOIDING_GRAPH: &str = r#"
model "boolean-eventuality"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "enter" "loop"
edge "loop" "spin" "loop"
edge "loop" "finish" "done"
label "loop" "blocked"
label "done" "complete"
"#;

#[test]
fn parser_respects_precedence_grouping_and_canonical_round_trip() {
    let parsed = parse_proposition_expression(r#"not "a" or "b" and ("c" or not "a")"#).unwrap();
    let expected = PropositionExpression::or(
        PropositionExpression::not(PropositionExpression::atom("a").unwrap()),
        PropositionExpression::and(
            PropositionExpression::atom("b").unwrap(),
            PropositionExpression::or(
                PropositionExpression::atom("c").unwrap(),
                PropositionExpression::not(PropositionExpression::atom("a").unwrap()),
            ),
        ),
    );
    assert_eq!(parsed, expected);

    let canonical = parsed.canonical_expression();
    assert_eq!(parse_proposition_expression(&canonical).unwrap(), parsed);

    let grouped = parse_proposition_expression(r#"("a" or "b") and "c""#).unwrap();
    let ungrouped = parse_proposition_expression(r#""a" or "b" and "c""#).unwrap();
    assert_ne!(grouped, ungrouped);
}

#[test]
fn parser_reports_deterministic_position_aware_errors() {
    let missing_rhs = parse_proposition_expression(r#""a" and"#).unwrap_err();
    assert_eq!(missing_rhs.position(), 7);
    assert!(matches!(
        missing_rhs.kind(),
        PropositionExpressionParseErrorKind::ExpectedExpression
    ));

    let invalid_escape = parse_proposition_expression(r#""a\q""#).unwrap_err();
    assert_eq!(invalid_escape.position(), 2);
    assert!(matches!(
        invalid_escape.kind(),
        PropositionExpressionParseErrorKind::InvalidEscape { escape } if escape == "q"
    ));

    let trailing = parse_proposition_expression(r#""a" "b""#).unwrap_err();
    assert_eq!(trailing.position(), 4);
    assert!(matches!(
        trailing.kind(),
        PropositionExpressionParseErrorKind::TrailingInput
    ));

    let empty = parse_proposition_expression(r#"""#).unwrap_err();
    assert!(matches!(
        empty.kind(),
        PropositionExpressionParseErrorKind::Semantic(PropositionExpressionError::EmptyProposition)
    ));
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OracleExpr {
    Atom(u8),
    Not(Box<OracleExpr>),
    And(Box<OracleExpr>, Box<OracleExpr>),
    Or(Box<OracleExpr>, Box<OracleExpr>),
}

impl OracleExpr {
    fn render(&self) -> String {
        match self {
            Self::Atom(index) => format!("\"{}\"", char::from(b'a' + index)),
            Self::Not(inner) => format!("not ({})", inner.render()),
            Self::And(left, right) => format!("({} and {})", left.render(), right.render()),
            Self::Or(left, right) => format!("({} or {})", left.render(), right.render()),
        }
    }

    fn evaluate(&self, assignment: u8) -> bool {
        match self {
            Self::Atom(index) => assignment & (1 << index) != 0,
            Self::Not(inner) => !inner.evaluate(assignment),
            Self::And(left, right) => left.evaluate(assignment) && right.evaluate(assignment),
            Self::Or(left, right) => left.evaluate(assignment) || right.evaluate(assignment),
        }
    }
}

fn generated_oracle_expressions() -> Vec<OracleExpr> {
    let atoms = vec![OracleExpr::Atom(0), OracleExpr::Atom(1), OracleExpr::Atom(2)];
    let mut level_one = atoms.clone();
    for expression in &atoms {
        level_one.push(OracleExpr::Not(Box::new(expression.clone())));
    }
    for left in &atoms {
        for right in &atoms {
            level_one.push(OracleExpr::And(
                Box::new(left.clone()),
                Box::new(right.clone()),
            ));
            level_one.push(OracleExpr::Or(
                Box::new(left.clone()),
                Box::new(right.clone()),
            ));
        }
    }

    let mut expressions = level_one.clone();
    for expression in &level_one {
        expressions.push(OracleExpr::Not(Box::new(expression.clone())));
    }
    for left in &level_one {
        for right in &level_one {
            expressions.push(OracleExpr::And(
                Box::new(left.clone()),
                Box::new(right.clone()),
            ));
            expressions.push(OracleExpr::Or(
                Box::new(left.clone()),
                Box::new(right.clone()),
            ));
        }
    }
    expressions
}

fn truth_table_document() -> formal_verification_lab::DeclarativeDocument {
    let mut input = String::from("model \"truth-table\"\n");
    for assignment in 0_u8..8 {
        input.push_str(&format!("state \"s{assignment}\"\n"));
    }
    input.push_str("initial \"s0\"\n");
    for assignment in 0_u8..8 {
        for index in 0_u8..3 {
            if assignment & (1 << index) != 0 {
                input.push_str(&format!(
                    "label \"s{assignment}\" \"{}\"\n",
                    char::from(b'a' + index)
                ));
            }
        }
    }
    parse_declarative_document(&input).unwrap()
}

#[test]
fn generated_boolean_truth_table_oracle_matches_parser_and_evaluator() {
    let document = truth_table_document();
    let expressions = generated_oracle_expressions();
    assert_eq!(expressions.len(), 1200);

    let mut checked = 0_usize;
    for oracle in expressions {
        let parsed = parse_proposition_expression(&oracle.render()).unwrap();
        let canonical = parsed.canonical_expression();
        assert_eq!(parse_proposition_expression(&canonical).unwrap(), parsed);

        for assignment in 0_u8..8 {
            let state = format!("s{assignment}");
            assert_eq!(
                parsed.evaluate(&document, &state).unwrap(),
                oracle.evaluate(assignment),
                "expression={} assignment={assignment}",
                oracle.render()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 9_600);
}

#[test]
fn expression_reachability_matches_direct_backend_and_shortest_witness() {
    let document = parse_declarative_document(LABELED_GRAPH).unwrap();
    let direct = ReachabilityProperty::new("reach-clean-critical", |state: &String| {
        state == "critical-a"
    })
    .unwrap();
    let direct_result = check_reachability(document.model(), &direct).unwrap();
    let expression = parse_proposition_expression(r#""critical" and not "error""#).unwrap();
    let frontend = check_proposition_expression_property(
        &document,
        &PropositionExpressionPropertySpec::reachable("reach-clean-critical", expression).unwrap(),
    )
    .unwrap();

    assert_eq!(direct_result.status, ReachabilityStatus::Reachable);
    assert_eq!(frontend.backend, ExactStateBackend::Reachability);
    assert_eq!(frontend.status, ExactStateStatus::Satisfied);
    assert_eq!(frontend.discovered_states, direct_result.discovered_states);
    assert_eq!(frontend.explored_transitions, direct_result.explored_transitions);
    assert_eq!(frontend.max_depth_reached, direct_result.max_depth_reached);
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::ReachabilityWitness {
            trace: direct_result.witness.unwrap(),
        })
    );
    let Some(ExactStateEvidence::ReachabilityWitness { trace }) = frontend.evidence else {
        panic!("expected reachability witness");
    };
    assert_eq!(trace.last().unwrap().state, "critical-a");
}

#[test]
fn expression_eventuality_matches_direct_backend_and_lasso_evidence() {
    let document = parse_declarative_document(AVOIDING_GRAPH).unwrap();
    let direct = EventualityProperty::new("eventually-clear-complete", |state: &String| {
        state == "done"
    })
    .unwrap();
    let direct_result = check_eventuality(document.model(), &direct).unwrap();
    let expression = parse_proposition_expression(r#""complete" and not "blocked""#).unwrap();
    let frontend = check_proposition_expression_property(
        &document,
        &PropositionExpressionPropertySpec::all_eventually(
            "eventually-clear-complete",
            expression,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(direct_result.status, EventualityStatus::Violated);
    assert_eq!(frontend.backend, ExactStateBackend::Eventuality);
    assert_eq!(frontend.status, ExactStateStatus::Violated);
    assert_eq!(frontend.discovered_states, direct_result.discovered_states);
    assert_eq!(frontend.explored_transitions, direct_result.explored_transitions);
    assert_eq!(frontend.max_depth_reached, direct_result.max_depth_reached);
    let Some(EventualityCounterexample::Infinite { stem, cycle }) = direct_result.counterexample
    else {
        panic!("expected direct lasso counterexample");
    };
    assert_eq!(
        frontend.evidence,
        Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle })
    );
}

#[test]
fn unknown_proposition_fails_closed_before_backend_short_circuiting() {
    let document = parse_declarative_document(LABELED_GRAPH).unwrap();
    let expression = parse_proposition_expression(r#""critical" or "missing""#).unwrap();
    let error = check_proposition_expression_property(
        &document,
        &PropositionExpressionPropertySpec::reachable("unknown-ref", expression).unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        error,
        PropositionExpressionError::UnknownProposition {
            proposition: "missing".to_owned(),
        }
    );
}

fn temp_model_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fvlab-proposition-expr-{label}-{}-{nonce}.model",
        std::process::id()
    ))
}

#[test]
fn proposition_expression_file_cli_is_end_to_end_and_m21_compatible() {
    let binary = env!("CARGO_BIN_EXE_fvlab");
    let labeled_path = temp_model_path("labeled");
    let avoiding_path = temp_model_path("avoiding");
    fs::write(&labeled_path, LABELED_GRAPH).unwrap();
    fs::write(&avoiding_path, AVOIDING_GRAPH).unwrap();

    let reach = Command::new(binary)
        .args([
            "proposition",
            "expr",
            labeled_path.to_str().unwrap(),
            "reachable",
            r#""critical" and not "error""#,
        ])
        .output()
        .unwrap();
    assert!(reach.status.success());
    let stdout = String::from_utf8(reach.stdout).unwrap();
    assert!(stdout.contains("backend: REACHABILITY"));
    assert!(stdout.contains("state property: SATISFIED"));
    assert!(stdout.contains("expression:"));
    assert!(stdout.contains("--choose-a--> \"critical-a\""));

    let eventuality = Command::new(binary)
        .args([
            "proposition",
            "expr",
            avoiding_path.to_str().unwrap(),
            "all-eventually",
            r#""complete" and not "blocked""#,
        ])
        .output()
        .unwrap();
    assert_eq!(eventuality.status.code(), Some(11));
    let stdout = String::from_utf8(eventuality.stdout).unwrap();
    assert!(stdout.contains("backend: EVENTUALITY"));
    assert!(stdout.contains("state property: VIOLATED"));
    assert!(stdout.contains("evidence: EVENTUALITY_INFINITE_COUNTEREXAMPLE"));
    assert!(stdout.contains("--spin-->"));

    let malformed = Command::new(binary)
        .args([
            "proposition",
            "expr",
            labeled_path.to_str().unwrap(),
            "reachable",
            r#""critical" and"#,
        ])
        .output()
        .unwrap();
    assert_eq!(malformed.status.code(), Some(2));
    let stderr = String::from_utf8(malformed.stderr).unwrap();
    assert!(stderr.contains("proposition expression parse error"));

    let legacy = Command::new(binary)
        .args([
            "proposition",
            "file",
            labeled_path.to_str().unwrap(),
            "reachable",
            "critical",
        ])
        .output()
        .unwrap();
    assert!(legacy.status.success());

    fs::remove_file(labeled_path).unwrap();
    fs::remove_file(avoiding_path).unwrap();
}

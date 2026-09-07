use formal_verification_lab::multi_response_examples::{
    dual_response_protocol, unfair_dual_response_protocol,
};
use formal_verification_lab::{
    check_multi_response, check_multi_response_temporal,
    check_multi_response_temporal_with_fairness_profile,
    check_multi_response_temporal_with_fairness_profile_and_limits,
    check_multi_response_temporal_with_fairness_profile_and_product_limits,
    check_multi_response_with_fairness_profile,
    check_multi_response_with_fairness_profile_and_limits,
    check_multi_response_with_fairness_profile_and_product_limits, parse_declarative_model,
    parse_multi_response_temporal, AnalysisLimits, AnalysisOutcome, BoundedOutcome,
    ExplorationLimits, FairnessProfile, Invariant, MultiResponseCounterexample,
    MultiResponseProperty, MultiResponseStatus, MultiResponseTemporalParseErrorKind,
    MultiResponseTemporalSpecError, ResponseActionRole, ResponseClause, StateVariable,
    Transition, TransitionSystem,
};

fn source() -> &'static str {
    r#"
# two independently tracked exact-action obligations
response("class-a","request-a","grant-a")
response("class-b","request-b","grant-b")
"#
}

fn parsed_spec() -> formal_verification_lab::MultiResponseTemporalSpec {
    parse_multi_response_temporal("dual-request-response", source()).unwrap()
}

fn direct_property() -> MultiResponseProperty {
    MultiResponseProperty::new(
        "dual-request-response",
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

#[test]
fn parser_preserves_order_and_round_trips_escaped_strings() {
    let input = r#"
response("alpha\\name","request\nA","grant\"A")
response("beta","request\tB","grant\rB")
"#;
    let spec = parse_multi_response_temporal("escaped", input).unwrap();

    assert_eq!(spec.clauses().len(), 2);
    assert_eq!(spec.clauses()[0].name(), "alpha\\name");
    assert_eq!(spec.clauses()[0].trigger(), "request\nA");
    assert_eq!(spec.clauses()[0].response(), "grant\"A");
    assert_eq!(spec.clauses()[1].trigger(), "request\tB");
    assert_eq!(spec.clauses()[1].response(), "grant\rB");

    let canonical = spec.canonical_document();
    let reparsed = parse_multi_response_temporal("escaped", &canonical).unwrap();
    assert_eq!(reparsed, spec);
}

#[test]
fn parser_fails_closed_on_structure_and_semantics() {
    let unknown = parse_multi_response_temporal(
        "bad",
        r#"eventually("class-a","request-a","grant-a")"#,
    )
    .unwrap_err();
    assert!(matches!(
        unknown.kind(),
        MultiResponseTemporalParseErrorKind::UnknownDirective { directive }
            if directive == "eventually"
    ));

    let arity = parse_multi_response_temporal("bad", r#"response("class-a","request-a")"#)
        .unwrap_err();
    assert!(matches!(
        arity.kind(),
        MultiResponseTemporalParseErrorKind::WrongArity {
            expected: 3,
            actual: 2
        }
    ));

    let empty_action = parse_multi_response_temporal(
        "bad",
        r#"response("class-a","   ","grant-a")"#,
    )
    .unwrap_err();
    assert!(matches!(
        empty_action.kind(),
        MultiResponseTemporalParseErrorKind::Semantic(
            MultiResponseTemporalSpecError::EmptyActionName {
                role: ResponseActionRole::Trigger,
                ..
            }
        )
    ));

    let duplicate = parse_multi_response_temporal(
        "bad",
        r#"
response("same","request-a","grant-a")
response("same","request-b","grant-b")
"#,
    )
    .unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        MultiResponseTemporalParseErrorKind::Semantic(
            MultiResponseTemporalSpecError::DuplicateClauseName { name }
        ) if name == "same"
    ));

    let empty = parse_multi_response_temporal("bad", "# only a comment\n").unwrap_err();
    assert!(matches!(
        empty.kind(),
        MultiResponseTemporalParseErrorKind::Semantic(
            MultiResponseTemporalSpecError::NoClauses
        )
    ));
}

#[test]
fn no_fair_frontend_is_exactly_equal_to_direct_multi_response_backend() {
    let property = direct_property();
    let spec = parsed_spec();

    for model in [
        dual_response_protocol().unwrap(),
        unfair_dual_response_protocol().unwrap(),
    ] {
        assert_eq!(
            check_multi_response_temporal(&model, &spec).unwrap(),
            check_multi_response(&model, &property).unwrap()
        );
    }
}

#[test]
fn combined_fair_frontend_is_exactly_equal_to_m49_backend() {
    let model = unfair_dual_response_protocol().unwrap();
    let property = direct_property();
    let spec = parsed_spec();
    let profile = FairnessProfile::new(["grant-a"], ["grant-b"]).unwrap();

    let frontend =
        check_multi_response_temporal_with_fairness_profile(&model, &spec, &profile).unwrap();
    let direct = check_multi_response_with_fairness_profile(&model, &property, &profile).unwrap();
    assert_eq!(frontend, direct);
    assert_eq!(frontend.status, MultiResponseStatus::Satisfied);
}

#[test]
fn product_and_staged_cutoffs_match_the_sealed_backend_exactly() {
    let model = unfair_dual_response_protocol().unwrap();
    let property = direct_property();
    let spec = parsed_spec();
    let profile = FairnessProfile::new(["grant-a"], ["grant-b"]).unwrap();
    let product_limits = ExplorationLimits {
        max_states: Some(2),
        max_transitions: Some(2),
        max_depth: Some(1),
    };

    let frontend = check_multi_response_temporal_with_fairness_profile_and_product_limits(
        &model,
        &spec,
        &profile,
        product_limits,
    )
    .unwrap();
    let direct = check_multi_response_with_fairness_profile_and_product_limits(
        &model,
        &property,
        &profile,
        product_limits,
    )
    .unwrap();
    assert_eq!(frontend, direct);
    assert!(matches!(frontend.outcome, BoundedOutcome::Inconclusive(_)));

    let limits = AnalysisLimits::new(
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: None,
            max_depth: None,
        },
        ExplorationLimits::unbounded(),
    );
    let frontend = check_multi_response_temporal_with_fairness_profile_and_limits(
        &model, &spec, &profile, limits,
    )
    .unwrap();
    let direct = check_multi_response_with_fairness_profile_and_limits(
        &model, &property, &profile, limits,
    )
    .unwrap();
    assert_eq!(frontend, direct);
    assert!(matches!(frontend.outcome, AnalysisOutcome::Inconclusive(_)));
}

#[test]
fn external_declarative_model_reaches_real_multi_response_semantics() {
    let model = parse_declarative_model(
        r#"
model "external-dual"
state "idle"
state "await-a"
state "ready-b"
state "await-b"
initial "idle"
edge "idle" "request-a" "await-a"
edge "await-a" "grant-a" "ready-b"
edge "ready-b" "request-b" "await-b"
edge "await-b" "wait-b" "await-b"
edge "await-b" "grant-b" "idle"
"#,
    )
    .unwrap();
    let spec = parsed_spec();

    let no_fair = check_multi_response_temporal(&model, &spec).unwrap();
    assert_eq!(no_fair.status, MultiResponseStatus::Violated);
    let MultiResponseCounterexample::Infinite { clause, cycle, .. } =
        no_fair.counterexample.unwrap()
    else {
        panic!("expected pending cycle");
    };
    assert_eq!(clause, "class-b");
    assert!(cycle.iter().any(|step| step.action.as_deref() == Some("wait-b")));

    let profile = FairnessProfile::new(["grant-a"], ["grant-b"]).unwrap();
    let fair =
        check_multi_response_temporal_with_fairness_profile(&model, &spec, &profile).unwrap();
    assert_eq!(fair.status, MultiResponseStatus::Satisfied);
}

#[test]
fn finite_pending_terminal_remains_a_violation_under_fairness() {
    let model = TransitionSystem::new(
        "finite-pending",
        vec![StateVariable::new("node", "protocol point")],
        vec![0usize],
        |state| match *state {
            0 => Ok(vec![Transition::new("request-b", 1usize)]),
            1 => Ok(Vec::new()),
            _ => Ok(Vec::new()),
        },
        vec![Invariant::new("domain", |state: &usize| *state <= 1)],
    )
    .unwrap();
    let spec = parsed_spec();
    let profile = FairnessProfile::new(["grant-a"], ["grant-b"]).unwrap();

    let result =
        check_multi_response_temporal_with_fairness_profile(&model, &spec, &profile).unwrap();
    assert_eq!(result.status, MultiResponseStatus::Violated);
    let MultiResponseCounterexample::Finite { clause, trace } = result.counterexample.unwrap()
    else {
        panic!("expected finite pending terminal");
    };
    assert_eq!(clause, "class-b");
    assert_eq!(trace.last().unwrap().state.state, 1);
}

const N: usize = 2;
const EDGE_COUNT: usize = N * N;
const ACTION_COUNT: usize = 4;
const ASSIGNMENT_COUNT: usize = ACTION_COUNT.pow(EDGE_COUNT as u32);

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
        _ => unreachable!("generated code is in range"),
    }
}

fn graph_model(mask: usize, codes: [u8; EDGE_COUNT]) -> TransitionSystem<usize> {
    TransitionSystem::new(
        format!("frontend-graph-{mask}"),
        vec![StateVariable::new("node", "current node")],
        vec![0usize],
        move |state| {
            let mut next = Vec::new();
            for to in 0..N {
                if has_edge(mask, *state, to) {
                    let edge = edge_index(*state, to);
                    next.push(Transition::new(action_for(codes[edge]), to));
                }
            }
            Ok(next)
        },
        vec![Invariant::new("domain", |state: &usize| *state < N)],
    )
    .unwrap()
}

#[test]
fn all_two_node_graphs_and_action_assignments_match_direct_backend() {
    let spec = parsed_spec();
    let property = direct_property();

    for graph_mask in 0..(1usize << EDGE_COUNT) {
        for assignment in 0..ASSIGNMENT_COUNT {
            let codes = decode_assignment(assignment);
            let model = graph_model(graph_mask, codes);
            let frontend = check_multi_response_temporal(&model, &spec).unwrap();
            let direct = check_multi_response(&model, &property).unwrap();
            assert_eq!(
                frontend, direct,
                "graph={graph_mask} assignment={assignment} codes={codes:?}"
            );
        }
    }
}

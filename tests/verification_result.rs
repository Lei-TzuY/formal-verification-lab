use formal_verification_lab::{
    AnalysisInconclusiveReason, AnalysisMultiResponseResult, AnalysisOutcome, AnalysisStage,
    BoundedMultiResponseResult, BoundedOutcome, ExplorationLimits, InconclusiveReason,
    MultiObligationState, MultiResponseCounterexample, MultiResponseResult, MultiResponseStatus,
    TraceStep, VerificationJobCutoffKind, VerificationJobCutoffStage, VerificationJobEvidence,
    VerificationJobOutcome, VerificationJobResultEnvelope, VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
};

fn step(
    action: Option<&str>,
    state: &str,
    pending: &[bool],
) -> TraceStep<MultiObligationState<String>> {
    TraceStep {
        action: action.map(str::to_owned),
        state: MultiObligationState {
            state: state.to_owned(),
            pending: pending.to_vec(),
        },
    }
}

#[test]
fn unbounded_lasso_preserves_clause_and_exact_trace_order() {
    let result = MultiResponseResult {
        property: "two-clause".to_owned(),
        status: MultiResponseStatus::Violated,
        model_states: 2,
        model_transitions: 3,
        product_states: 3,
        product_transitions: 4,
        clause_count: 2,
        counterexample: Some(MultiResponseCounterexample::Infinite {
            clause: "class-b".to_owned(),
            stem: vec![
                step(None, "idle", &[false, false]),
                step(Some("request-b"), "waiting", &[false, true]),
            ],
            cycle: vec![
                step(None, "waiting", &[false, true]),
                step(Some("wait-b"), "waiting", &[false, true]),
            ],
        }),
    };
    let envelope = VerificationJobResultEnvelope::from_unbounded(
        "model-1",
        &["grant-b".to_owned()],
        &["tick".to_owned()],
        ExplorationLimits::unbounded(),
        ExplorationLimits::unbounded(),
        &result,
    );

    assert_eq!(
        envelope.schema_version,
        VERIFICATION_JOB_RESULT_SCHEMA_VERSION
    );
    assert_eq!(envelope.outcome, VerificationJobOutcome::Violated);
    let Some(VerificationJobEvidence::Infinite {
        clause,
        stem,
        cycle,
    }) = &envelope.evidence
    else {
        panic!("expected lasso evidence");
    };
    assert_eq!(clause, "class-b");
    assert_eq!(stem[1].action.as_deref(), Some("request-b"));
    assert_eq!(stem[1].state, "waiting");
    assert_eq!(stem[1].pending, [false, true]);
    assert_eq!(cycle[1].action.as_deref(), Some("wait-b"));

    let first = envelope.to_json();
    let second = envelope.to_json();
    assert_eq!(first, second);
    assert!(
        first.starts_with("{\"schema_version\":1,\"outcome\":\"violated\",\"status\":\"VIOLATED\"")
    );
    assert!(first.contains("\"kind\":\"lasso\",\"clause\":\"class-b\""));
    assert!(
        first.contains("\"action\":\"request-b\",\"state\":\"waiting\",\"pending\":[false,true]")
    );
}

#[test]
fn product_cutoff_is_stage_qualified_and_accounted() {
    let result = BoundedMultiResponseResult::<String> {
        property: "bounded".to_owned(),
        outcome: BoundedOutcome::Inconclusive(InconclusiveReason::TransitionLimitReached {
            limit: 7,
        }),
        model_states: 4,
        model_transitions: 6,
        product_states: 5,
        checked_product_states: 3,
        explored_product_transitions: 7,
        retained_product_transitions: 6,
        max_product_depth_reached: Some(2),
        clause_count: 1,
        counterexample: None,
    };
    let limits = ExplorationLimits {
        max_states: None,
        max_transitions: Some(7),
        max_depth: None,
    };
    let envelope = VerificationJobResultEnvelope::from_product_bounded(
        "bounded-model",
        &[],
        &[],
        ExplorationLimits::unbounded(),
        limits,
        &result,
    );

    assert_eq!(envelope.outcome, VerificationJobOutcome::Inconclusive);
    assert!(envelope
        .to_json()
        .contains("\"outcome\":\"inconclusive\",\"status\":\"INCONCLUSIVE\""));
    let cutoff = envelope.cutoff.expect("cutoff should be present");
    assert_eq!(cutoff.stage, VerificationJobCutoffStage::Product);
    assert_eq!(cutoff.kind, VerificationJobCutoffKind::TransitionLimit);
    assert_eq!(cutoff.limit, 7);
    assert_eq!(envelope.accounting.checked_product_states, Some(3));
    assert_eq!(envelope.accounting.retained_product_transitions, Some(6));
    assert!(envelope
        .to_json()
        .contains("\"cutoff\":{\"stage\":\"product\",\"kind\":\"transition_limit\",\"limit\":7}"));
}

#[test]
fn staged_model_cutoff_preserves_model_before_product_provenance() {
    let reason = AnalysisInconclusiveReason {
        stage: AnalysisStage::Model,
        reason: InconclusiveReason::DepthLimitReached { limit: 2 },
    };
    let result = AnalysisMultiResponseResult::<String> {
        property: "staged".to_owned(),
        outcome: AnalysisOutcome::Inconclusive(reason),
        model_completion: BoundedOutcome::Inconclusive(reason.reason),
        product_completion: BoundedOutcome::Conclusive(()),
        model_states: 3,
        checked_model_states: 2,
        explored_model_transitions: 2,
        retained_model_transitions: 2,
        max_model_depth_reached: Some(2),
        product_states: 0,
        checked_product_states: 0,
        explored_product_transitions: 0,
        retained_product_transitions: 0,
        max_product_depth_reached: None,
        clause_count: 2,
        counterexample: None,
    };
    let model_limits = ExplorationLimits {
        max_states: None,
        max_transitions: None,
        max_depth: Some(2),
    };
    let envelope = VerificationJobResultEnvelope::from_staged(
        "staged-model",
        &[],
        &[],
        model_limits,
        ExplorationLimits::unbounded(),
        &result,
    );

    assert!(envelope
        .to_json()
        .contains("\"outcome\":\"inconclusive\",\"status\":\"INCONCLUSIVE\""));
    let cutoff = envelope.cutoff.expect("model cutoff should be present");
    assert_eq!(cutoff.stage, VerificationJobCutoffStage::Model);
    assert_eq!(cutoff.kind, VerificationJobCutoffKind::DepthLimit);
    assert_eq!(cutoff.limit, 2);
    assert_eq!(envelope.accounting.checked_model_states, Some(2));
    assert!(envelope
        .to_json()
        .contains("\"stage\":\"model\",\"kind\":\"depth_limit\",\"limit\":2"));
}

#[test]
fn json_writer_escapes_quotes_backslashes_controls_and_preserves_utf8() {
    let envelope =
        VerificationJobResultEnvelope::error("bad \"quote\" \\ path\nline\t模型\u{0001}");
    let json = envelope.to_json();

    assert_eq!(
        json,
        "{\"schema_version\":1,\"outcome\":\"error\",\"status\":null,\"model\":null,\"property\":null,\"weak_fair_actions\":[],\"strong_fair_actions\":[],\"model_limits\":{\"max_states\":null,\"max_transitions\":null,\"max_depth\":null},\"product_limits\":{\"max_states\":null,\"max_transitions\":null,\"max_depth\":null},\"accounting\":{\"model_states\":null,\"checked_model_states\":null,\"explored_model_transitions\":null,\"retained_model_transitions\":null,\"max_model_depth_reached\":null,\"product_states\":null,\"checked_product_states\":null,\"explored_product_transitions\":null,\"retained_product_transitions\":null,\"max_product_depth_reached\":null},\"cutoff\":null,\"evidence\":null,\"error\":\"bad \\\"quote\\\" \\\\ path\\nline\\t模型\\u0001\"}"
    );
}

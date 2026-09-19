use formal_verification_lab::{
    analyze_recurrence_with_limits, parse_declarative_model, ExplorationLimits,
    StructuralJobOutcome, StructuralJobResultEnvelope,
};

const ACYCLIC: &str = r#"model "acyclic"
state "a"
state "b"
initial "a"
edge "a" "step" "b"
"#;

const CYCLE_BEFORE_CUTOFF: &str = r#"model "cycle-before-cutoff"
state "a"
state "b"
initial "a"
edge "a" "loop" "a"
edge "a" "later" "b"
"#;

#[test]
fn complete_acyclic_envelope_is_structural_not_pass_fail() {
    let model = parse_declarative_model(ACYCLIC).unwrap();
    let result = analyze_recurrence_with_limits(&model, ExplorationLimits::unbounded()).unwrap();
    let envelope = StructuralJobResultEnvelope::from_recurrence(
        model.name().to_owned(),
        ExplorationLimits::unbounded(),
        &result,
    );

    assert_eq!(envelope.outcome, StructuralJobOutcome::Acyclic);
    assert!(envelope.cutoff.is_none());
    assert_eq!(envelope.components.as_ref().unwrap().len(), 2);
    assert!(envelope.evidence.is_none());

    let json = envelope.to_json();
    assert!(json.contains("\"analysis\":\"recurrence\""));
    assert!(json.contains("\"outcome\":\"acyclic\""));
    assert!(json.contains("\"components\":["));
    assert!(json.contains("\"evidence\":null"));
    assert!(!json.contains("satisfied"));
    assert!(!json.contains("violated"));
    assert!(!json.contains("\"property\""));
}

#[test]
fn conclusive_cycle_can_preserve_cutoff_and_withhold_full_partition() {
    let model = parse_declarative_model(CYCLE_BEFORE_CUTOFF).unwrap();
    let limits = ExplorationLimits {
        max_transitions: Some(1),
        ..ExplorationLimits::unbounded()
    };
    let result = analyze_recurrence_with_limits(&model, limits).unwrap();
    let envelope =
        StructuralJobResultEnvelope::from_recurrence(model.name().to_owned(), limits, &result);

    assert_eq!(envelope.outcome, StructuralJobOutcome::CycleFound);
    assert!(envelope.cutoff.is_some());
    assert!(envelope.components.is_none());
    assert!(envelope.evidence.is_some());

    let json = envelope.to_json();
    assert!(json.contains("\"outcome\":\"cycle_found\""));
    assert!(json.contains("\"cutoff\":{\"kind\":\"transition_limit\",\"limit\":1}"));
    assert!(json.contains("\"components\":null"));
    assert!(json.contains("\"component_index\":0"));
    assert!(json.contains("\"action\":\"loop\""));
}

#[test]
fn error_envelope_is_versioned_neutral_json_and_escapes_messages() {
    let envelope = StructuralJobResultEnvelope::error("bad \"model\"\nline");
    assert_eq!(envelope.outcome, StructuralJobOutcome::Error);

    let json = envelope.to_json();
    assert!(
        json.starts_with("{\"schema_version\":1,\"analysis\":\"recurrence\",\"outcome\":\"error\"")
    );
    assert!(json.contains("\"model\":null"));
    assert!(json.contains("\"components\":null"));
    assert!(json.contains("\"evidence\":null"));
    assert!(json.contains("\"error\":\"bad \\\"model\\\"\\nline\""));
}

use formal_verification_lab::{
    create_declarative_mu_parity_certificate, parse_declarative_document,
    parse_declarative_mu_parity_certificate, render_declarative_mu_parity_certificate,
    verify_declarative_mu_parity_certificate, DeclarativeMuParityCertificateError,
    MuParityCertificateParseError, MuParityMove,
};

const MODEL: &str = r#"
model "certificate-model"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "cycle" "loop"
edge "start" "finish" "done"
edge "loop" "back" "start"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#;

const FORMULA: &str =
    r#"mu X. "complete" or diamond (nu Y. ("ready" and diamond $Y) or diamond $X)"#;

#[test]
fn canonical_model_identity_ignores_comments_and_spacing_but_preserves_semantics_and_order() {
    let compact = parse_declarative_document(MODEL).unwrap();
    let reformatted = parse_declarative_document(
        r#"
# ignored
model    "certificate-model"
state "start"
state "loop"
state "done"
initial      "start"

edge "start" "cycle" "loop"
edge "start" "finish" "done"
edge "loop" "back" "start"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#,
    )
    .unwrap();
    assert_eq!(compact.canonical_identity(), reformatted.canonical_identity());

    let reordered = parse_declarative_document(
        r#"
model "certificate-model"
state "loop"
state "start"
state "done"
initial "start"
edge "start" "cycle" "loop"
edge "start" "finish" "done"
edge "loop" "back" "start"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#,
    )
    .unwrap();
    assert_ne!(compact.canonical_identity(), reordered.canonical_identity());
}

#[test]
fn certificate_render_parse_round_trip_is_exact_and_deterministic() {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();

    let first = render_declarative_mu_parity_certificate(&certificate);
    let parsed = parse_declarative_mu_parity_certificate(&first).unwrap();
    let second = render_declarative_mu_parity_certificate(&parsed);

    assert_eq!(parsed, certificate);
    assert_eq!(second, first);
    assert!(first.starts_with("fvlab-mu-parity-certificate 1\n"));
    assert!(first.contains("model-binding \"model \\"certificate-model\\\""));
    assert!(first.contains("formula \""));
    assert!(first.contains("winning even "));
    assert!(first.contains("choices odd "));
    assert!(first.ends_with("end\n"));
}

#[test]
fn certificate_verification_reconstructs_m85_evidence_without_resolving() {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();
    let text = render_declarative_mu_parity_certificate(&certificate);
    let parsed = parse_declarative_mu_parity_certificate(&text).unwrap();

    assert!(verify_declarative_mu_parity_certificate(&document, FORMULA, &parsed).is_ok());
}

#[test]
fn certificate_binding_rejects_different_model_and_formula() {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();

    let different_model = parse_declarative_document(
        &MODEL.replace(
            r#"edge "start" "finish" "done""#,
            r#"edge "start" "finish-renamed" "done""#,
        ),
    )
    .unwrap();
    assert!(matches!(
        verify_declarative_mu_parity_certificate(&different_model, FORMULA, &certificate),
        Err(DeclarativeMuParityCertificateError::ModelBindingMismatch)
    ));

    assert!(matches!(
        verify_declarative_mu_parity_certificate(
            &document,
            r#"mu X. "complete" or box $X"#,
            &certificate,
        ),
        Err(DeclarativeMuParityCertificateError::FormulaBindingMismatch { .. })
    ));
}

#[test]
fn certificate_parser_fails_closed_on_truncation_duplicates_and_bad_references() {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();
    let rendered = render_declarative_mu_parity_certificate(&certificate);

    let truncated = rendered
        .lines()
        .filter(|line| *line != "end")
        .collect::<Vec<_>>()
        .join("\n");
    assert!(matches!(
        parse_declarative_mu_parity_certificate(&truncated),
        Err(MuParityCertificateParseError::Missing { directive: "end" })
    ));

    let duplicate = rendered.replacen(
        "formula \"",
        &format!("formula \"{}\"\nformula \"", certificate.formula.replace('"', "\\\"")),
        1,
    );
    let duplicate_error = parse_declarative_mu_parity_certificate(&duplicate).unwrap_err();
    assert!(duplicate_error.to_string().contains("duplicate directive 'formula'"));

    let mut out_of_range = certificate.clone();
    out_of_range.even_winning_vertices = vec![certificate.positions.len() + 7];
    assert!(matches!(
        verify_declarative_mu_parity_certificate(&document, FORMULA, &out_of_range),
        Err(DeclarativeMuParityCertificateError::MalformedEvidence { .. })
    ));
}

#[test]
fn certificate_verifier_rejects_semantic_and_initial_tampering() {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();

    let mut bad_move = certificate.clone();
    if let Some(choice) = bad_move
        .even_choices
        .first_mut()
        .or_else(|| bad_move.odd_choices.first_mut())
    {
        choice.semantic_move = MuParityMove::BooleanLeft;
    } else {
        panic!("representative certificate must contain at least one strategy choice");
    }
    assert!(verify_declarative_mu_parity_certificate(&document, FORMULA, &bad_move).is_err());

    let mut bad_initial = certificate.clone();
    bad_initial.initial[0].satisfied = !bad_initial.initial[0].satisfied;
    assert!(verify_declarative_mu_parity_certificate(&document, FORMULA, &bad_initial).is_err());

    let mut bad_accounting = certificate.clone();
    bad_accounting.explored_transitions += 1;
    assert!(verify_declarative_mu_parity_certificate(&document, FORMULA, &bad_accounting).is_err());
}

#[test]
fn parser_rejects_unsupported_version_and_position_count_mismatch() {
    let unsupported = "fvlab-mu-parity-certificate 2\nend\n";
    assert!(matches!(
        parse_declarative_mu_parity_certificate(unsupported),
        Err(MuParityCertificateParseError::UnsupportedVersion { version: 2 })
    ));

    let malformed = r#"fvlab-mu-parity-certificate 1
model-binding "model \"x\"\nstate \"s\"\ninitial \"s\"\n"
formula "true"
accounting 1 0 0 1 0
satisfying 1 0
positions 1
winning even 1 0
winning odd 0
choices even 0
choices odd 0
initials 1
initial 0 true even 0
end
"#;
    assert!(matches!(
        parse_declarative_mu_parity_certificate(malformed),
        Err(MuParityCertificateParseError::CountMismatch {
            section: "positions",
            expected: 1,
            actual: 0,
        })
    ));
}

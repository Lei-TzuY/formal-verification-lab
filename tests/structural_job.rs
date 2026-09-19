use formal_verification_lab::{
    parse_structural_job, ExplorationLimits, StructuralJobAnalysis, StructuralJobParseErrorKind,
};

#[test]
fn structural_job_round_trips_canonically_with_model_limits() {
    let input = r#"
# recurrence is structural, so there is deliberately no property directive
analysis "recurrence"
model "graphs/request\ncycle.fvl"
max-states 12
max-transitions 34
max-depth 5
"#;
    let job = parse_structural_job(input).unwrap();

    assert_eq!(job.analysis(), StructuralJobAnalysis::Recurrence);
    assert_eq!(job.model_path(), "graphs/request\ncycle.fvl");
    assert_eq!(
        job.model_limits(),
        ExplorationLimits {
            max_states: Some(12),
            max_transitions: Some(34),
            max_depth: Some(5),
        }
    );

    let canonical = job.canonical_document();
    assert_eq!(
        canonical,
        "analysis \"recurrence\"\nmodel \"graphs/request\\ncycle.fvl\"\nmax-states 12\nmax-transitions 34\nmax-depth 5"
    );
    assert_eq!(
        parse_structural_job(&canonical)
            .unwrap()
            .canonical_document(),
        canonical
    );
}

#[test]
fn quoted_model_paths_support_the_same_stable_escape_surface() {
    let input = "analysis \"recurrence\"\nmodel \"dir\\\\name\\\"tab\\tmodel.fvl\"";
    let job = parse_structural_job(input).unwrap();
    assert_eq!(job.model_path(), "dir\\name\"tab\tmodel.fvl");
    assert_eq!(
        parse_structural_job(&job.canonical_document())
            .unwrap()
            .model_path(),
        job.model_path()
    );
}

#[test]
fn structural_job_requires_analysis_and_model() {
    let missing_analysis = parse_structural_job("model \"graph.fvl\"").unwrap_err();
    assert!(matches!(
        missing_analysis.kind(),
        StructuralJobParseErrorKind::MissingDirective { directive }
            if directive == "analysis"
    ));

    let missing_model = parse_structural_job("analysis \"recurrence\"").unwrap_err();
    assert!(matches!(
        missing_model.kind(),
        StructuralJobParseErrorKind::MissingDirective { directive }
            if directive == "model"
    ));
}

#[test]
fn unsupported_analysis_and_property_or_temporal_directives_fail_closed() {
    let invalid = parse_structural_job("analysis \"safety\"\nmodel \"graph.fvl\"").unwrap_err();
    assert!(matches!(
        invalid.kind(),
        StructuralJobParseErrorKind::InvalidAnalysis { analysis }
            if analysis == "safety"
    ));

    for directive in [
        "property \"property.txt\"",
        "weak-fair-action \"tick\"",
        "strong-fair-action \"tick\"",
        "max-product-states 4",
    ] {
        let input = format!("analysis \"recurrence\"\nmodel \"graph.fvl\"\n{directive}");
        let error = parse_structural_job(&input).unwrap_err();
        assert!(
            matches!(
                error.kind(),
                StructuralJobParseErrorKind::UnknownDirective { .. }
            ),
            "{directive}: {error}"
        );
    }
}

#[test]
fn duplicate_singletons_empty_path_bad_numbers_and_trailing_input_are_rejected() {
    let duplicate =
        parse_structural_job("analysis \"recurrence\"\nmodel \"a.fvl\"\nmax-depth 1\nmax-depth 2")
            .unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        StructuralJobParseErrorKind::DuplicateDirective { directive }
            if directive == "max-depth"
    ));

    let empty = parse_structural_job("analysis \"recurrence\"\nmodel \"\"").unwrap_err();
    assert!(matches!(
        empty.kind(),
        StructuralJobParseErrorKind::EmptyModelPath
    ));

    let number = parse_structural_job("analysis \"recurrence\"\nmodel \"a.fvl\"\nmax-states nope")
        .unwrap_err();
    assert!(matches!(
        number.kind(),
        StructuralJobParseErrorKind::ExpectedNumber
    ));

    let trailing =
        parse_structural_job("analysis \"recurrence\"\nmodel \"a.fvl\" trailing").unwrap_err();
    assert!(matches!(
        trailing.kind(),
        StructuralJobParseErrorKind::TrailingInput
    ));
}

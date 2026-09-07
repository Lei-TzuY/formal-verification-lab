use formal_verification_lab::{parse_verification_job, VerificationJobParseErrorKind};

#[test]
fn minimal_manifest_parses_and_renders_canonically() {
    let job = parse_verification_job(
        "# portable verification job\nmodel \"models/protocol.fvl\"\nproperty \"properties/response.fvt\"\n",
    )
    .unwrap();

    assert_eq!(job.model_path(), "models/protocol.fvl");
    assert_eq!(job.property_path(), "properties/response.fvt");
    assert!(job.weak_fair_actions().is_empty());
    assert!(job.strong_fair_actions().is_empty());
    assert_eq!(
        job.canonical_document(),
        "model \"models/protocol.fvl\"\nproperty \"properties/response.fvt\""
    );
    assert_eq!(
        parse_verification_job(&job.canonical_document()).unwrap(),
        job
    );
}

#[test]
fn manifest_compiles_fairness_and_all_budget_dimensions_to_m54_options() {
    let job = parse_verification_job(
        "model \"m.fvl\"\n\
         property \"p.fvt\"\n\
         weak-fair-action \"grant-a\"\n\
         weak-fair-action \"grant-b\"\n\
         strong-fair-action \"grant-b\"\n\
         max-model-states 10\n\
         max-model-transitions 20\n\
         max-model-depth 3\n\
         max-product-states 40\n\
         max-product-transitions 50\n\
         max-product-depth 6\n",
    )
    .unwrap();

    assert_eq!(job.weak_fair_actions(), ["grant-a", "grant-b"]);
    assert_eq!(job.strong_fair_actions(), ["grant-b"]);
    assert_eq!(job.model_limits().max_states, Some(10));
    assert_eq!(job.model_limits().max_transitions, Some(20));
    assert_eq!(job.model_limits().max_depth, Some(3));
    assert_eq!(job.product_limits().max_states, Some(40));
    assert_eq!(job.product_limits().max_transitions, Some(50));
    assert_eq!(job.product_limits().max_depth, Some(6));
    assert_eq!(
        job.option_args(),
        [
            "--weak-fair-action",
            "grant-a",
            "--weak-fair-action",
            "grant-b",
            "--strong-fair-action",
            "grant-b",
            "--max-model-states",
            "10",
            "--max-model-transitions",
            "20",
            "--max-model-depth",
            "3",
            "--max-product-states",
            "40",
            "--max-product-transitions",
            "50",
            "--max-product-depth",
            "6",
        ]
    );
}

#[test]
fn quoted_paths_and_actions_round_trip_with_utf8_and_escapes() {
    let job = parse_verification_job(
        "model \"模型\\\\a\\\"b.fvl\"\n\
         property \"規格\\nline.fvt\"\n\
         weak-fair-action \"允許\\tgrant\"\n",
    )
    .unwrap();

    assert_eq!(job.model_path(), "模型\\a\"b.fvl");
    assert_eq!(job.property_path(), "規格\nline.fvt");
    assert_eq!(job.weak_fair_actions(), ["允許\tgrant"]);
    assert_eq!(
        parse_verification_job(&job.canonical_document()).unwrap(),
        job
    );
}

#[test]
fn missing_required_and_duplicate_singleton_directives_fail_closed() {
    let missing = parse_verification_job("model \"m.fvl\"\n").unwrap_err();
    assert_eq!(
        missing.kind(),
        &VerificationJobParseErrorKind::MissingDirective {
            directive: "property".to_owned(),
        }
    );

    let duplicate = parse_verification_job(
        "model \"m.fvl\"\nproperty \"p.fvt\"\nmax-product-states 1\nmax-product-states 2\n",
    )
    .unwrap_err();
    assert_eq!(duplicate.line(), 4);
    assert_eq!(
        duplicate.kind(),
        &VerificationJobParseErrorKind::DuplicateDirective {
            directive: "max-product-states".to_owned(),
        }
    );
}

#[test]
fn unsupported_directives_and_trailing_input_are_rejected() {
    let unsupported =
        parse_verification_job("model \"m.fvl\"\nproperty \"p.fvt\"\ntimeout 100\n").unwrap_err();
    assert_eq!(unsupported.line(), 3);
    assert_eq!(unsupported.column(), 1);
    assert_eq!(
        unsupported.kind(),
        &VerificationJobParseErrorKind::UnknownDirective {
            directive: "timeout".to_owned(),
        }
    );

    let trailing =
        parse_verification_job("model \"m.fvl\" extra\nproperty \"p.fvt\"\n").unwrap_err();
    assert_eq!(trailing.line(), 1);
    assert_eq!(
        trailing.kind(),
        &VerificationJobParseErrorKind::TrailingInput
    );
}

#[test]
fn malformed_numbers_fail_at_the_value_without_saturating_or_guessing() {
    let missing =
        parse_verification_job("model \"m.fvl\"\nproperty \"p.fvt\"\nmax-model-depth nope\n")
            .unwrap_err();
    assert_eq!(missing.line(), 3);
    assert_eq!(
        missing.kind(),
        &VerificationJobParseErrorKind::ExpectedNumber
    );

    let overflow = parse_verification_job(&format!(
        "model \"m.fvl\"\nproperty \"p.fvt\"\nmax-product-states {}0\n",
        usize::MAX
    ))
    .unwrap_err();
    assert_eq!(overflow.line(), 3);
    assert!(matches!(
        overflow.kind(),
        VerificationJobParseErrorKind::InvalidNumber { .. }
    ));
}

#[test]
fn malformed_string_reports_utf8_byte_column_and_escape() {
    let error = parse_verification_job(
        "model \"模型\"\nproperty \"p.fvt\"\nweak-fair-action \"允許\\q\"\n",
    )
    .unwrap_err();

    assert_eq!(error.line(), 3);
    assert!(error.column() > "weak-fair-action \"".len());
    assert_eq!(
        error.kind(),
        &VerificationJobParseErrorKind::InvalidEscape {
            escape: "q".to_owned(),
        }
    );
}

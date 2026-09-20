use formal_verification_lab::{
    create_declarative_mu_parity_certificate, parse_certificate_verification_job,
    parse_declarative_document, render_declarative_mu_parity_certificate,
    run_certificate_verification_job_json, CertificateVerificationJobOutcome,
    CertificateVerificationJobParseErrorKind, CERTIFICATE_VERIFICATION_ERROR_EXIT_CODE,
    CERTIFICATE_VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
    CERTIFICATE_VERIFICATION_REJECTED_EXIT_CODE,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "job-certificate"
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
fn manifest_parser_round_trips_canonically_and_fails_closed() {
    let input = r#"
# comment
model "models/system.fvl"
property "properties/reach.mu"
certificate "certificates/reach.mupc"
"#;
    let job = parse_certificate_verification_job(input).unwrap();
    assert_eq!(job.model_path(), "models/system.fvl");
    assert_eq!(job.property_path(), "properties/reach.mu");
    assert_eq!(job.certificate_path(), "certificates/reach.mupc");

    let canonical = job.canonical_document();
    assert_eq!(
        canonical,
        "model \"models/system.fvl\"\nproperty \"properties/reach.mu\"\ncertificate \"certificates/reach.mupc\""
    );
    assert_eq!(parse_certificate_verification_job(&canonical).unwrap(), job);

    let duplicate = parse_certificate_verification_job(
        "model \"a\"\nmodel \"b\"\nproperty \"p\"\ncertificate \"c\"",
    )
    .unwrap_err();
    assert!(matches!(
        duplicate.kind(),
        CertificateVerificationJobParseErrorKind::DuplicateDirective { directive }
            if directive == "model"
    ));

    let missing =
        parse_certificate_verification_job("model \"a\"\nproperty \"p\"").unwrap_err();
    assert!(matches!(
        missing.kind(),
        CertificateVerificationJobParseErrorKind::MissingDirective { directive }
            if directive == "certificate"
    ));

    let unknown = parse_certificate_verification_job(
        "model \"a\"\nproperty \"p\"\ncertificate \"c\"\nbackend \"parity\"",
    )
    .unwrap_err();
    assert!(matches!(
        unknown.kind(),
        CertificateVerificationJobParseErrorKind::UnknownDirective { directive }
            if directive == "backend"
    ));
}

#[test]
fn runner_verifies_manifest_relative_certificate_and_json_is_deterministic() {
    let root = fixture_dir("verified");
    let manifest = write_valid_fixture(&root);

    let first = run_certificate_verification_job_json(&manifest);
    let second = run_certificate_verification_job_json(&manifest);

    assert_eq!(first, second);
    assert_eq!(first.exit_code, 0);
    assert_eq!(
        first.envelope.schema_version,
        CERTIFICATE_VERIFICATION_JOB_RESULT_SCHEMA_VERSION
    );
    assert_eq!(
        first.envelope.outcome,
        CertificateVerificationJobOutcome::Verified
    );
    assert_eq!(first.envelope.model.as_deref(), Some("models/system.fvl"));
    assert_eq!(
        first.envelope.property.as_deref(),
        Some("properties/reach.mu")
    );
    assert_eq!(
        first.envelope.certificate.as_deref(),
        Some("certificates/reach.mupc")
    );
    assert_eq!(
        first.envelope.canonical_formula.as_deref(),
        Some(
            create_declarative_mu_parity_certificate(
                &parse_declarative_document(MODEL).unwrap(),
                FORMULA,
            )
            .unwrap()
            .formula
            .as_str()
        )
    );
    assert_eq!(first.envelope.certificate_schema_version, Some(1));
    assert_eq!(first.envelope.message, None);
    assert_eq!(first.to_json(), second.to_json());
    assert!(!first.to_json().contains(root.to_str().unwrap()));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn loaded_bad_certificate_is_rejected_not_error() {
    let root = fixture_dir("rejected");
    let manifest = write_valid_fixture(&root);
    let certificate_path = root.join("certificates/reach.mupc");

    let original = fs::read_to_string(&certificate_path).unwrap();
    let truncated = original
        .lines()
        .filter(|line| *line != "end")
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&certificate_path, truncated).unwrap();

    let rejected = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        rejected.exit_code,
        CERTIFICATE_VERIFICATION_REJECTED_EXIT_CODE
    );
    assert_eq!(
        rejected.envelope.outcome,
        CertificateVerificationJobOutcome::Rejected
    );
    assert_eq!(rejected.envelope.certificate_schema_version, Some(1));
    assert!(rejected
        .envelope
        .message
        .as_deref()
        .is_some_and(|message| message.contains("missing 'end'")));

    let unsupported = original.replacen(
        "fvlab-mu-parity-certificate 1",
        "fvlab-mu-parity-certificate 2",
        1,
    );
    fs::write(&certificate_path, unsupported).unwrap();
    let version = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        version.envelope.outcome,
        CertificateVerificationJobOutcome::Rejected
    );
    assert_eq!(version.envelope.certificate_schema_version, Some(2));

    let tampered_accounting = original.replacen("accounting 3 3 ", "accounting 3 4 ", 1);
    assert_ne!(tampered_accounting, original);
    fs::write(&certificate_path, tampered_accounting).unwrap();
    let accounting = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        accounting.envelope.outcome,
        CertificateVerificationJobOutcome::Rejected
    );
    assert!(accounting
        .envelope
        .message
        .as_deref()
        .is_some_and(|message| message.contains("canonical evaluation")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn setup_failures_are_errors_before_certificate_validation() {
    let root = fixture_dir("errors");
    let manifest = write_valid_fixture(&root);

    fs::remove_file(root.join("certificates/reach.mupc")).unwrap();
    let missing_certificate = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        missing_certificate.exit_code,
        CERTIFICATE_VERIFICATION_ERROR_EXIT_CODE
    );
    assert_eq!(
        missing_certificate.envelope.outcome,
        CertificateVerificationJobOutcome::Error
    );
    assert!(missing_certificate
        .envelope
        .message
        .as_deref()
        .is_some_and(|message| message.contains("failed to read mu parity certificate")));

    let malformed_manifest = root.join("bad.job");
    fs::write(
        &malformed_manifest,
        "model \"models/system.fvl\"\nproperty \"properties/reach.mu\"",
    )
    .unwrap();
    let malformed = run_certificate_verification_job_json(&malformed_manifest);
    assert_eq!(malformed.exit_code, CERTIFICATE_VERIFICATION_ERROR_EXIT_CODE);
    assert_eq!(malformed.envelope.model, None);
    assert_eq!(malformed.envelope.canonical_formula, None);

    let manifest = write_valid_fixture(&root);
    fs::write(root.join("properties/reach.mu"), "mu X. not $X").unwrap();
    let invalid_formula = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        invalid_formula.envelope.outcome,
        CertificateVerificationJobOutcome::Error
    );
    assert_eq!(invalid_formula.envelope.certificate_schema_version, None);
    assert!(invalid_formula
        .envelope
        .message
        .as_deref()
        .is_some_and(|message| message.contains("occurs negatively")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn wrong_model_and_formula_bindings_are_rejected_after_valid_setup() {
    let root = fixture_dir("bindings");
    let manifest = write_valid_fixture(&root);

    fs::write(
        root.join("properties/reach.mu"),
        r#"mu X. "complete" or box $X"#,
    )
    .unwrap();
    let formula = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        formula.envelope.outcome,
        CertificateVerificationJobOutcome::Rejected
    );
    assert!(formula
        .envelope
        .message
        .as_deref()
        .is_some_and(|message| message.contains("formula binding mismatch")));

    fs::write(root.join("properties/reach.mu"), FORMULA).unwrap();
    fs::write(
        root.join("models/system.fvl"),
        MODEL.replace(
            r#"edge "start" "finish" "done""#,
            r#"edge "start" "finish-renamed" "done""#,
        ),
    )
    .unwrap();
    let model = run_certificate_verification_job_json(&manifest);
    assert_eq!(
        model.envelope.outcome,
        CertificateVerificationJobOutcome::Rejected
    );
    assert!(model
        .envelope
        .message
        .as_deref()
        .is_some_and(|message| message.contains("different declarative model")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_binary_matches_direct_runner_for_verified_rejected_and_error() {
    let root = fixture_dir("cli");
    let manifest = write_valid_fixture(&root);
    assert_cli_matches(&manifest);

    let certificate_path = root.join("certificates/reach.mupc");
    let original = fs::read_to_string(&certificate_path).unwrap();
    fs::write(
        &certificate_path,
        original
            .lines()
            .filter(|line| *line != "end")
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap();
    assert_cli_matches(&manifest);

    fs::remove_file(&certificate_path).unwrap();
    assert_cli_matches(&manifest);

    fs::remove_dir_all(root).unwrap();
}

fn assert_cli_matches(manifest: &Path) {
    let direct = run_certificate_verification_job_json(manifest);
    let output = Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args([
            "mu",
            "certificate",
            "job",
            manifest.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(direct.exit_code as i32));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", direct.to_json())
    );
    assert!(output.stderr.is_empty());
}

fn write_valid_fixture(root: &Path) -> PathBuf {
    fs::create_dir_all(root.join("models")).unwrap();
    fs::create_dir_all(root.join("properties")).unwrap();
    fs::create_dir_all(root.join("certificates")).unwrap();

    fs::write(root.join("models/system.fvl"), MODEL).unwrap();
    fs::write(root.join("properties/reach.mu"), FORMULA).unwrap();

    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, FORMULA).unwrap();
    fs::write(
        root.join("certificates/reach.mupc"),
        render_declarative_mu_parity_certificate(&certificate),
    )
    .unwrap();

    let manifest = root.join("verify.job");
    fs::write(
        &manifest,
        "model \"models/system.fvl\"\nproperty \"properties/reach.mu\"\ncertificate \"certificates/reach.mupc\"\n",
    )
    .unwrap();
    manifest
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m87-certificate-job-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

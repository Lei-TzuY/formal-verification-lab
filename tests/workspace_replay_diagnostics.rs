use formal_verification_lab::{
    create_workspace_replay_lock, create_workspace_snapshot, diagnose_replay_json_drift,
    parse_workspace_replay_lock, render_workspace_replay_lock, verify_workspace_replay_lock,
    verify_workspace_replay_lock_text, MapTextSourceProvider, WorkspaceReplayJsonDifferenceKind,
    WorkspaceReplayLockVerificationStatus, WorkspaceReplayMode,
    WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE, WORKSPACE_REPLAY_LOCK_VERIFICATION_SCHEMA_VERSION,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "diagnostic-model"
state "s"
initial "s"
"#;

#[test]
fn bounded_json_diagnostics_report_stable_structural_paths_and_kinds() {
    let nested =
        diagnose_replay_json_drift(r#"{"a":{"items":[1,2,3]}}"#, r#"{"a":{"items":[1,9,3]}}"#)
            .unwrap();
    assert_eq!(nested.kind, WorkspaceReplayJsonDifferenceKind::ScalarValue);
    assert_eq!(nested.path.as_deref(), Some("/a/items/1"));
    assert_eq!(nested.expected_preview.as_deref(), Some("2"));
    assert_eq!(nested.actual_preview.as_deref(), Some("9"));

    let missing = diagnose_replay_json_drift(r#"{"a":1,"b":2}"#, r#"{"a":1}"#).unwrap();
    assert_eq!(
        missing.kind,
        WorkspaceReplayJsonDifferenceKind::MissingMember
    );
    assert_eq!(missing.path.as_deref(), Some("/b"));
    assert_eq!(missing.expected_preview.as_deref(), Some("2"));
    assert_eq!(missing.actual_preview, None);

    let unexpected = diagnose_replay_json_drift(r#"{"a":1}"#, r#"{"a":1,"b":2}"#).unwrap();
    assert_eq!(
        unexpected.kind,
        WorkspaceReplayJsonDifferenceKind::UnexpectedMember
    );
    assert_eq!(unexpected.path.as_deref(), Some("/b"));

    let type_change = diagnose_replay_json_drift(r#"{"a":1}"#, r#"{"a":"1"}"#).unwrap();
    assert_eq!(
        type_change.kind,
        WorkspaceReplayJsonDifferenceKind::TypeChange
    );
    assert_eq!(type_change.path.as_deref(), Some("/a"));
    assert_eq!(type_change.expected_preview.as_deref(), Some("number"));
    assert_eq!(type_change.actual_preview.as_deref(), Some("string"));

    let length = diagnose_replay_json_drift(r#"{"a":[1,2]}"#, r#"{"a":[1,2,3]}"#).unwrap();
    assert_eq!(length.kind, WorkspaceReplayJsonDifferenceKind::ArrayLength);
    assert_eq!(length.path.as_deref(), Some("/a"));
    assert_eq!(length.expected_preview.as_deref(), Some("2"));
    assert_eq!(length.actual_preview.as_deref(), Some("3"));

    let escaped_key = diagnose_replay_json_drift(r#"{"a/b~c":1}"#, r#"{"a/b~c":2}"#).unwrap();
    assert_eq!(escaped_key.path.as_deref(), Some("/a~1b~0c"));
}

#[test]
fn structurally_equal_byte_drift_and_invalid_json_use_non_authoritative_fallbacks() {
    let byte_only = diagnose_replay_json_drift(
        r#"{"a":1,"b":[true,null]}"#,
        "{ \"b\" : [ true , null ], \"a\" : 1 }",
    )
    .unwrap();
    assert_eq!(byte_only.kind, WorkspaceReplayJsonDifferenceKind::ByteOnly);
    assert_eq!(byte_only.path, None);
    assert!(byte_only.first_byte_offset < 8);

    let invalid = diagnose_replay_json_drift(r#"{"a":1}"#, r#"{"a":]"#).unwrap();
    assert_eq!(
        invalid.kind,
        WorkspaceReplayJsonDifferenceKind::ByteFallback
    );
    assert_eq!(invalid.path, None);
    assert_eq!(invalid.first_byte_offset, 5);
}

#[test]
fn replay_lock_reports_precise_accounting_path_without_changing_match_authority() {
    let lock = fixture_lock();
    let rendered = render_workspace_replay_lock(&lock);
    assert!(lock.expected_json().contains("\"outcome\":\"satisfied\""));
    assert!(lock.expected_json().contains("\"model_states\":1"));

    let mutated_json = lock
        .expected_json()
        .replacen("\"model_states\":1", "\"model_states\":2", 1);
    assert_ne!(mutated_json, lock.expected_json());
    assert!(mutated_json.contains("\"outcome\":\"satisfied\""));

    let mutated_text = replace_result_frame(&rendered, lock.expected_json(), &mutated_json);
    let mutated = parse_workspace_replay_lock(&mutated_text).unwrap();
    let run = verify_workspace_replay_lock(&mutated);

    assert_eq!(run.exit_code, WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE);
    assert_eq!(
        run.envelope.status,
        WorkspaceReplayLockVerificationStatus::Mismatched
    );
    assert_eq!(
        run.envelope.schema_version,
        WORKSPACE_REPLAY_LOCK_VERIFICATION_SCHEMA_VERSION
    );
    assert_eq!(run.envelope.exit_code_matches, Some(true));
    assert_eq!(run.envelope.json_matches, Some(false));

    let difference = run.envelope.json_difference.as_ref().unwrap();
    assert_eq!(
        difference.kind,
        WorkspaceReplayJsonDifferenceKind::ScalarValue
    );
    assert_eq!(
        difference.path.as_deref(),
        Some("/jobs/0/result/accounting/model_states")
    );
    assert_eq!(difference.expected_preview.as_deref(), Some("2"));
    assert_eq!(difference.actual_preview.as_deref(), Some("1"));

    let json = run.to_json();
    assert!(json.contains("\"json_difference\":{"));
    assert!(json.contains("\"path\":\"/jobs/0/result/accounting/model_states\""));
}

#[test]
fn matched_and_exit_only_drift_do_not_invent_json_differences() {
    let lock = fixture_lock();

    let matched = verify_workspace_replay_lock(&lock);
    assert_eq!(matched.exit_code, 0);
    assert_eq!(
        matched.envelope.status,
        WorkspaceReplayLockVerificationStatus::Matched
    );
    assert_eq!(matched.envelope.json_matches, Some(true));
    assert_eq!(matched.envelope.json_difference, None);

    let rendered = render_workspace_replay_lock(&lock);
    let exit_drift = rendered.replacen("exit 0\n", "exit 1\n", 1);
    assert_ne!(exit_drift, rendered);
    let exit_run = verify_workspace_replay_lock(&parse_workspace_replay_lock(&exit_drift).unwrap());
    assert_eq!(exit_run.exit_code, WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE);
    assert_eq!(exit_run.envelope.exit_code_matches, Some(false));
    assert_eq!(exit_run.envelope.json_matches, Some(true));
    assert_eq!(exit_run.envelope.json_difference, None);
}

#[test]
fn replay_lock_keeps_mismatch_for_structurally_equal_and_invalid_expected_json() {
    let lock = fixture_lock();
    let rendered = render_workspace_replay_lock(&lock);

    let structural_equal_json = format!(" {} ", lock.expected_json());
    let byte_only_text =
        replace_result_frame(&rendered, lock.expected_json(), &structural_equal_json);
    let byte_only =
        verify_workspace_replay_lock(&parse_workspace_replay_lock(&byte_only_text).unwrap());
    assert_eq!(
        byte_only.envelope.status,
        WorkspaceReplayLockVerificationStatus::Mismatched
    );
    assert_eq!(byte_only.envelope.json_matches, Some(false));
    assert_eq!(
        byte_only.envelope.json_difference.as_ref().unwrap().kind,
        WorkspaceReplayJsonDifferenceKind::ByteOnly
    );

    let mut invalid_json = lock.expected_json().to_owned();
    invalid_json.replace_range(0..1, "[");
    assert_eq!(invalid_json.len(), lock.expected_json().len());
    let invalid_text = replace_result_frame(&rendered, lock.expected_json(), &invalid_json);
    let invalid =
        verify_workspace_replay_lock(&parse_workspace_replay_lock(&invalid_text).unwrap());
    assert_eq!(
        invalid.envelope.status,
        WorkspaceReplayLockVerificationStatus::Mismatched
    );
    assert_eq!(invalid.exit_code, WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE);
    assert_eq!(
        invalid.envelope.json_difference.as_ref().unwrap().kind,
        WorkspaceReplayJsonDifferenceKind::ByteFallback
    );
}

#[test]
fn built_lock_verify_json_matches_library_structured_diagnostic() {
    let lock = fixture_lock();
    let rendered = render_workspace_replay_lock(&lock);
    let mutated_json = lock
        .expected_json()
        .replacen("\"model_states\":1", "\"model_states\":2", 1);
    assert_ne!(mutated_json, lock.expected_json());
    let mutated_text = replace_result_frame(&rendered, lock.expected_json(), &mutated_json);

    let direct = verify_workspace_replay_lock_text(&mutated_text);
    let path = temp_file("cli-diagnostic");
    fs::write(&path, mutated_text).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_fvlab-workspace"))
        .args(["lock-verify", path.to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(direct.exit_code as i32));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\n", direct.to_json())
    );
    assert!(output.stderr.is_empty());
    assert!(direct
        .to_json()
        .contains("\"path\":\"/jobs/0/result/accounting/model_states\""));

    fs::remove_file(path).unwrap();
}

fn fixture_lock() -> formal_verification_lab::WorkspaceReplayLock {
    let provider = MapTextSourceProvider::from_sources(vec![
        ("models/system.fvl".to_owned(), MODEL.to_owned()),
        ("properties/true.mu".to_owned(), "true".to_owned()),
        (
            "jobs/verify.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/true.mu\"\n"
                .to_owned(),
        ),
        (
            "diagnostic.suite".to_owned(),
            "suite \"diagnostic\"\njob \"verification\" \"jobs/verify.job\" expect \"satisfied\"\n"
                .to_owned(),
        ),
    ])
    .unwrap();

    let snapshot = create_workspace_snapshot(&provider, "diagnostic.suite").unwrap();
    create_workspace_replay_lock(&snapshot, WorkspaceReplayMode::Raw)
}

fn replace_result_frame(rendered: &str, original_json: &str, replacement_json: &str) -> String {
    let header = format!("result {}\n", original_json.len());
    let header_start = rendered.rfind(&header).expect("result header must exist");
    let frame_start = header_start + header.len();
    let frame_end = frame_start + original_json.len();
    assert_eq!(&rendered[frame_start..frame_end], original_json);

    let mut output = String::new();
    output.push_str(&rendered[..header_start]);
    output.push_str(&format!("result {}\n", replacement_json.len()));
    output.push_str(replacement_json);
    output.push_str(&rendered[frame_end..]);
    output
}

fn temp_file(kind: &str) -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fvlab-m92-replay-diagnostic-{kind}-{}-{id}.lock",
        std::process::id()
    ))
}

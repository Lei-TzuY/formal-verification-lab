use formal_verification_lab::{
    audit_workspace_replay_determinism, audit_workspace_replay_determinism_text,
    create_workspace_snapshot, render_workspace_snapshot, MapTextSourceProvider,
    WorkspaceReplayDeterminismStatus, WorkspaceReplayMode,
    MAX_WORKSPACE_REPLAY_DETERMINISM_ADDITIONAL_ATTEMPTS,
};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "determinism"
state "s"
initial "s"
label "s" "ok"
"#;

fn fixture_snapshot() -> formal_verification_lab::WorkspaceSnapshot {
    let provider = MapTextSourceProvider::from_sources(vec![
        ("models/system.fvl".to_owned(), MODEL.to_owned()),
        ("properties/true.mu".to_owned(), "true".to_owned()),
        (
            "jobs/verify.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/true.mu\"\n"
                .to_owned(),
        ),
        (
            "determinism.suite".to_owned(),
            "suite \"determinism\"\njob \"verification\" \"jobs/verify.job\" expect \"satisfied\"\n"
                .to_owned(),
        ),
    ])
    .unwrap();

    create_workspace_snapshot(&provider, "determinism.suite").unwrap()
}

#[test]
fn raw_and_expectation_replay_are_stable_for_all_requested_attempts() {
    let snapshot = fixture_snapshot();

    let raw = audit_workspace_replay_determinism(&snapshot, WorkspaceReplayMode::Raw, 3);
    assert_eq!(raw.exit_code, 0);
    assert_eq!(
        raw.envelope.status,
        WorkspaceReplayDeterminismStatus::Deterministic
    );
    assert_eq!(raw.envelope.requested_additional_attempts, 3);
    assert_eq!(raw.envelope.completed_attempts, 4);
    assert_eq!(raw.envelope.baseline_exit_code, Some(0));
    assert_eq!(raw.envelope.divergent_attempt_index, None);
    assert_eq!(raw.envelope.json_difference, None);

    let expectations =
        audit_workspace_replay_determinism(&snapshot, WorkspaceReplayMode::Expectations, 4);
    assert_eq!(expectations.exit_code, 0);
    assert_eq!(
        expectations.envelope.status,
        WorkspaceReplayDeterminismStatus::Deterministic
    );
    assert_eq!(expectations.envelope.completed_attempts, 5);
    assert_eq!(expectations.envelope.baseline_exit_code, Some(0));
    assert_eq!(expectations.envelope.json_difference, None);
}

#[test]
fn invalid_attempt_counts_fail_before_any_baseline_result_is_recorded() {
    let snapshot = fixture_snapshot();

    let zero = audit_workspace_replay_determinism(&snapshot, WorkspaceReplayMode::Raw, 0);
    assert_eq!(zero.exit_code, 2);
    assert_eq!(
        zero.envelope.status,
        WorkspaceReplayDeterminismStatus::Error
    );
    assert_eq!(zero.envelope.completed_attempts, 0);
    assert_eq!(zero.envelope.baseline_exit_code, None);
    assert!(zero
        .envelope
        .error
        .as_deref()
        .is_some_and(|message| message.contains("at least one additional attempt")));

    let excessive = audit_workspace_replay_determinism(
        &snapshot,
        WorkspaceReplayMode::Raw,
        MAX_WORKSPACE_REPLAY_DETERMINISM_ADDITIONAL_ATTEMPTS + 1,
    );
    assert_eq!(excessive.exit_code, 2);
    assert_eq!(
        excessive.envelope.status,
        WorkspaceReplayDeterminismStatus::Error
    );
    assert_eq!(excessive.envelope.completed_attempts, 0);
    assert_eq!(excessive.envelope.baseline_exit_code, None);
    assert!(excessive
        .envelope
        .error
        .as_deref()
        .is_some_and(|message| message.contains("exceeds maximum")));
}

#[test]
fn text_api_reports_snapshot_parse_failure_without_replay() {
    let run =
        audit_workspace_replay_determinism_text("not a snapshot", WorkspaceReplayMode::Raw, 2);

    assert_eq!(run.exit_code, 2);
    assert_eq!(run.envelope.status, WorkspaceReplayDeterminismStatus::Error);
    assert_eq!(run.envelope.completed_attempts, 0);
    assert_eq!(run.envelope.baseline_exit_code, None);
    assert!(run
        .envelope
        .error
        .as_deref()
        .is_some_and(|message| message.contains("workspace snapshot parse error")));
}

#[test]
fn deterministic_json_is_repeatable_for_identical_audits() {
    let snapshot = fixture_snapshot();

    let first = audit_workspace_replay_determinism(&snapshot, WorkspaceReplayMode::Raw, 2);
    let second = audit_workspace_replay_determinism(&snapshot, WorkspaceReplayMode::Raw, 2);

    assert_eq!(first, second);
    assert_eq!(first.to_json(), second.to_json());
    assert!(first.to_json().contains("\"status\":\"deterministic\""));
    assert!(first
        .to_json()
        .contains("\"requested_additional_attempts\":2"));
    assert!(first.to_json().contains("\"completed_attempts\":3"));
}

#[test]
fn built_workspace_cli_matches_library_for_raw_expectations_and_count_errors() {
    let snapshot = fixture_snapshot();
    let rendered = render_workspace_snapshot(&snapshot);
    let path = temp_file("audit");
    fs::write(&path, &rendered).unwrap();

    for (mode_name, mode) in [
        ("raw", WorkspaceReplayMode::Raw),
        ("expectations", WorkspaceReplayMode::Expectations),
    ] {
        let direct = audit_workspace_replay_determinism_text(&rendered, mode, 3);
        let output = Command::new(env!("CARGO_BIN_EXE_fvlab-workspace"))
            .args([
                "determinism-check",
                path.to_str().unwrap(),
                mode_name,
                "3",
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

    let direct_zero =
        audit_workspace_replay_determinism_text(&rendered, WorkspaceReplayMode::Raw, 0);
    let zero = Command::new(env!("CARGO_BIN_EXE_fvlab-workspace"))
        .args([
            "determinism-check",
            path.to_str().unwrap(),
            "raw",
            "0",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    assert_eq!(zero.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(zero.stdout).unwrap(),
        format!("{}\n", direct_zero.to_json())
    );
    assert!(zero.stderr.is_empty());

    fs::remove_file(path).unwrap();
}

fn temp_file(kind: &str) -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fvlab-m93-replay-determinism-{kind}-{}-{id}.snapshot",
        std::process::id()
    ))
}

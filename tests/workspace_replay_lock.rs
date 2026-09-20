use formal_verification_lab::{
    create_workspace_replay_lock, create_workspace_snapshot, parse_workspace_replay_lock,
    render_workspace_replay_lock, render_workspace_snapshot, verify_workspace_replay_lock,
    verify_workspace_replay_lock_text, MapTextSourceProvider, RootedFileSystemTextSourceProvider,
    WorkspaceReplayLockParseErrorKind, WorkspaceReplayLockVerificationStatus, WorkspaceReplayMode,
    MAX_WORKSPACE_REPLAY_LOCK_RESULT_BYTES, MAX_WORKSPACE_REPLAY_LOCK_SNAPSHOT_BYTES,
    WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "lock-model"
state "s"
initial "s"
"#;

#[test]
fn exact_replay_lock_is_deterministic_across_providers_and_round_trips() {
    let sources = fixture_sources();
    let root_a = fixture_dir("deterministic-a");
    let root_b = fixture_dir("deterministic-b");
    write_sources(&root_a, &sources);
    write_sources(&root_b, &sources);

    let rooted_a = RootedFileSystemTextSourceProvider::new(&root_a);
    let rooted_b = RootedFileSystemTextSourceProvider::new(&root_b);
    let map = MapTextSourceProvider::from_sources(sources).unwrap();

    let snapshot_a = create_workspace_snapshot(&rooted_a, "lock.suite").unwrap();
    let snapshot_b = create_workspace_snapshot(&rooted_b, "lock.suite").unwrap();
    let snapshot_map = create_workspace_snapshot(&map, "lock.suite").unwrap();

    let raw_a = create_workspace_replay_lock(&snapshot_a, WorkspaceReplayMode::Raw);
    let raw_b = create_workspace_replay_lock(&snapshot_b, WorkspaceReplayMode::Raw);
    let raw_map = create_workspace_replay_lock(&snapshot_map, WorkspaceReplayMode::Raw);
    let rendered = render_workspace_replay_lock(&raw_a);

    assert_eq!(rendered, render_workspace_replay_lock(&raw_b));
    assert_eq!(rendered, render_workspace_replay_lock(&raw_map));
    assert!(!rendered.contains(root_a.to_str().unwrap()));
    assert!(!rendered.contains(root_b.to_str().unwrap()));
    assert!(raw_a.expected_json().contains("\"outcome\":\"satisfied\""));
    assert!(raw_a.expected_json().contains("\"discovered_states\":1"));

    let parsed = parse_workspace_replay_lock(&rendered).unwrap();
    assert_eq!(parsed, raw_a);
    assert_eq!(render_workspace_replay_lock(&parsed), rendered);

    let expectations = create_workspace_replay_lock(&snapshot_a, WorkspaceReplayMode::Expectations);
    assert!(expectations
        .expected_json()
        .contains("\"outcome\":\"matched\""));
    assert_eq!(
        verify_workspace_replay_lock(&expectations).envelope.status,
        WorkspaceReplayLockVerificationStatus::Matched
    );

    fs::remove_dir_all(root_a).unwrap();
    fs::remove_dir_all(root_b).unwrap();
}

#[test]
fn lock_verification_detects_outcome_preserving_full_json_and_exit_drift() {
    let map = MapTextSourceProvider::from_sources(fixture_sources()).unwrap();
    let snapshot = create_workspace_snapshot(&map, "lock.suite").unwrap();
    let lock = create_workspace_replay_lock(&snapshot, WorkspaceReplayMode::Raw);
    let rendered = render_workspace_replay_lock(&lock);

    assert!(rendered.contains("\"discovered_states\":1"));
    let json_drift = rendered.replacen("\"discovered_states\":1", "\"discovered_states\":2", 1);
    assert_ne!(json_drift, rendered);
    let json_lock = parse_workspace_replay_lock(&json_drift).unwrap();
    assert!(json_lock
        .expected_json()
        .contains("\"outcome\":\"satisfied\""));
    let json_check = verify_workspace_replay_lock(&json_lock);
    assert_eq!(
        json_check.exit_code,
        WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE
    );
    assert_eq!(
        json_check.envelope.status,
        WorkspaceReplayLockVerificationStatus::Mismatched
    );
    assert_eq!(json_check.envelope.exit_code_matches, Some(true));
    assert_eq!(json_check.envelope.json_matches, Some(false));

    let exit_drift = rendered.replacen("exit 0\n", "exit 1\n", 1);
    assert_ne!(exit_drift, rendered);
    let exit_check =
        verify_workspace_replay_lock(&parse_workspace_replay_lock(&exit_drift).unwrap());
    assert_eq!(exit_check.envelope.exit_code_matches, Some(false));
    assert_eq!(exit_check.envelope.json_matches, Some(true));

    let both_drift = json_drift.replacen("exit 0\n", "exit 1\n", 1);
    let both_check =
        verify_workspace_replay_lock(&parse_workspace_replay_lock(&both_drift).unwrap());
    assert_eq!(both_check.envelope.exit_code_matches, Some(false));
    assert_eq!(both_check.envelope.json_matches, Some(false));
}

#[test]
fn lock_parser_fails_closed_on_version_mode_truncation_trailing_and_limits() {
    let map = MapTextSourceProvider::from_sources(fixture_sources()).unwrap();
    let snapshot = create_workspace_snapshot(&map, "lock.suite").unwrap();
    let rendered = render_workspace_replay_lock(&create_workspace_replay_lock(
        &snapshot,
        WorkspaceReplayMode::Raw,
    ));

    let unsupported = rendered.replacen(
        "fvlab-workspace-replay-lock 1",
        "fvlab-workspace-replay-lock 2",
        1,
    );
    assert!(matches!(
        parse_workspace_replay_lock(&unsupported),
        Err(error) if matches!(
            error.kind(),
            WorkspaceReplayLockParseErrorKind::UnsupportedVersion { version: 2 }
        )
    ));

    let invalid_mode = rendered.replacen("mode raw\n", "mode bad\n", 1);
    assert!(matches!(
        parse_workspace_replay_lock(&invalid_mode),
        Err(error) if matches!(
            error.kind(),
            WorkspaceReplayLockParseErrorKind::InvalidMode { mode } if mode == "bad"
        )
    ));

    let truncated = &rendered[..rendered.len() - 4];
    assert!(parse_workspace_replay_lock(truncated).is_err());

    let trailing = format!("{rendered}junk");
    assert!(matches!(
        parse_workspace_replay_lock(&trailing),
        Err(error) if matches!(
            error.kind(),
            WorkspaceReplayLockParseErrorKind::TrailingPayload
        )
    ));

    let snapshot_header = format!("snapshot {}\n", render_workspace_snapshot(&snapshot).len());
    let too_large_snapshot = rendered.replacen(
        &snapshot_header,
        &format!(
            "snapshot {}\n",
            MAX_WORKSPACE_REPLAY_LOCK_SNAPSHOT_BYTES + 1
        ),
        1,
    );
    assert!(matches!(
        parse_workspace_replay_lock(&too_large_snapshot),
        Err(error) if matches!(
            error.kind(),
            WorkspaceReplayLockParseErrorKind::SnapshotTooLarge { .. }
        )
    ));

    let result_header = format!("result {}\n", lock_result_len(&rendered));
    let too_large_result = rendered.replacen(
        &result_header,
        &format!("result {}\n", MAX_WORKSPACE_REPLAY_LOCK_RESULT_BYTES + 1),
        1,
    );
    assert!(matches!(
        parse_workspace_replay_lock(&too_large_result),
        Err(error) if matches!(
            error.kind(),
            WorkspaceReplayLockParseErrorKind::ResultTooLarge { .. }
        )
    ));
}

#[test]
fn malformed_embedded_snapshot_is_an_error_not_a_mismatch() {
    let map = MapTextSourceProvider::from_sources(fixture_sources()).unwrap();
    let snapshot = create_workspace_snapshot(&map, "lock.suite").unwrap();
    let rendered = render_workspace_replay_lock(&create_workspace_replay_lock(
        &snapshot,
        WorkspaceReplayMode::Raw,
    ));
    let invalid = rendered.replacen(
        "fvlab-workspace-snapshot 1",
        "fvlab-workspace-snapshot 2",
        1,
    );

    assert!(matches!(
        parse_workspace_replay_lock(&invalid),
        Err(error) if matches!(
            error.kind(),
            WorkspaceReplayLockParseErrorKind::InvalidSnapshot { .. }
        )
    ));

    let run = verify_workspace_replay_lock_text(&invalid);
    assert_eq!(run.exit_code, 2);
    assert_eq!(
        run.envelope.status,
        WorkspaceReplayLockVerificationStatus::Error
    );
    assert_eq!(run.envelope.exit_code_matches, None);
    assert_eq!(run.envelope.json_matches, None);
}

#[test]
fn built_cli_creates_identical_lock_and_verifies_offline_after_sources_are_removed() {
    let root = fixture_dir("cli");
    let sources = fixture_sources();
    write_sources(&root, &sources);
    let rooted = RootedFileSystemTextSourceProvider::new(&root);
    let snapshot = create_workspace_snapshot(&rooted, "lock.suite").unwrap();

    let snapshot_path = root.with_extension("snapshot");
    let lock_path = root.with_extension("lock");
    fs::write(&snapshot_path, render_workspace_snapshot(&snapshot)).unwrap();

    let expected_lock = create_workspace_replay_lock(&snapshot, WorkspaceReplayMode::Raw);
    let create = run_binary(&[
        "lock-create",
        snapshot_path.to_str().unwrap(),
        "raw",
        lock_path.to_str().unwrap(),
    ]);
    assert_eq!(create.status.code(), Some(0));
    assert!(create.stdout.is_empty());
    assert!(create.stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&lock_path).unwrap(),
        render_workspace_replay_lock(&expected_lock)
    );

    fs::remove_dir_all(&root).unwrap();
    fs::remove_file(&snapshot_path).unwrap();

    let direct = verify_workspace_replay_lock(&expected_lock);
    let verify = run_binary(&[
        "lock-verify",
        lock_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(verify.status.code(), Some(direct.exit_code as i32));
    assert_eq!(
        String::from_utf8(verify.stdout).unwrap(),
        format!("{}\n", direct.to_json())
    );
    assert!(verify.stderr.is_empty());

    let mut mismatch_text = fs::read_to_string(&lock_path).unwrap();
    mismatch_text = mismatch_text.replacen("\"discovered_states\":1", "\"discovered_states\":2", 1);
    fs::write(&lock_path, &mismatch_text).unwrap();
    let mismatch_direct = verify_workspace_replay_lock_text(&mismatch_text);
    let mismatch = run_binary(&[
        "lock-verify",
        lock_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(
        mismatch.status.code(),
        Some(WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE as i32)
    );
    assert_eq!(
        String::from_utf8(mismatch.stdout).unwrap(),
        format!("{}\n", mismatch_direct.to_json())
    );
    assert!(mismatch.stderr.is_empty());

    fs::remove_file(lock_path).unwrap();
}

#[test]
fn built_cli_expectation_lock_matches_library() {
    let root = fixture_dir("cli-expectations");
    write_sources(&root, &fixture_sources());
    let rooted = RootedFileSystemTextSourceProvider::new(&root);
    let snapshot = create_workspace_snapshot(&rooted, "lock.suite").unwrap();
    let snapshot_path = root.with_extension("snapshot-expectations");
    let lock_path = root.with_extension("lock-expectations");
    fs::write(&snapshot_path, render_workspace_snapshot(&snapshot)).unwrap();

    let create = run_binary(&[
        "lock-create",
        snapshot_path.to_str().unwrap(),
        "expectations",
        lock_path.to_str().unwrap(),
    ]);
    assert_eq!(create.status.code(), Some(0));
    assert!(create.stderr.is_empty());

    let lock_text = fs::read_to_string(&lock_path).unwrap();
    let parsed = parse_workspace_replay_lock(&lock_text).unwrap();
    assert_eq!(parsed.mode(), WorkspaceReplayMode::Expectations);
    let direct = verify_workspace_replay_lock(&parsed);
    assert_eq!(
        direct.envelope.status,
        WorkspaceReplayLockVerificationStatus::Matched
    );

    let verify = run_binary(&[
        "lock-verify",
        lock_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(verify.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(verify.stdout).unwrap(),
        format!("{}\n", direct.to_json())
    );

    fs::remove_dir_all(root).unwrap();
    fs::remove_file(snapshot_path).unwrap();
    fs::remove_file(lock_path).unwrap();
}

fn lock_result_len(rendered: &str) -> usize {
    rendered
        .lines()
        .find_map(|line| line.strip_prefix("result "))
        .unwrap()
        .parse()
        .unwrap()
}

fn fixture_sources() -> Vec<(String, String)> {
    vec![
        ("models/system.fvl".to_owned(), MODEL.to_owned()),
        ("properties/true.mu".to_owned(), "true".to_owned()),
        (
            "jobs/verify.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/true.mu\"\n"
                .to_owned(),
        ),
        (
            "lock.suite".to_owned(),
            "suite \"lock-suite\"\njob \"verification\" \"jobs/verify.job\" expect \"satisfied\"\n"
                .to_owned(),
        ),
    ]
}

fn write_sources(root: &Path, sources: &[(String, String)]) {
    for (source_id, text) in sources {
        let path = root.join(source_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }
}

fn run_binary(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-workspace"))
        .args(args)
        .output()
        .expect("fvlab-workspace binary should execute")
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m91-replay-lock-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

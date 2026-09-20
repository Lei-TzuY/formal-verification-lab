use formal_verification_lab::{
    create_declarative_mu_parity_certificate, create_workspace_snapshot,
    parse_declarative_document, parse_workspace_snapshot, render_declarative_mu_parity_certificate,
    render_workspace_snapshot, replay_workspace_snapshot_expectations_json,
    replay_workspace_snapshot_json, run_orchestration_suite_expectations_json_with_provider,
    run_orchestration_suite_json_with_provider, MapTextSourceProvider,
    OrchestrationRegressionOutcome, OrchestrationSuiteStatus, RootedFileSystemTextSourceProvider,
    WorkspaceSnapshotParseErrorKind, MAX_WORKSPACE_SNAPSHOT_ENTRIES,
    MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
# portable Ω snapshot with "quotes" and \ backslash
model "snapshot"
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

const REACH_FORMULA: &str = r#"mu X. "complete" or diamond $X"#;

#[test]
fn snapshot_creation_is_byte_identical_across_two_roots_and_map_provider() {
    let sources = fixture_sources();
    let root_a = fixture_dir("create-a");
    let root_b = fixture_dir("create-b");
    write_sources(&root_a, &sources);
    write_sources(&root_b, &sources);

    let provider_a = RootedFileSystemTextSourceProvider::new(&root_a);
    let provider_b = RootedFileSystemTextSourceProvider::new(&root_b);
    let map = MapTextSourceProvider::from_sources(sources.clone()).unwrap();

    let snapshot_a = create_workspace_snapshot(&provider_a, "mixed.suite").unwrap();
    let snapshot_b = create_workspace_snapshot(&provider_b, "mixed.suite").unwrap();
    let snapshot_map = create_workspace_snapshot(&map, "mixed.suite").unwrap();

    let rendered_a = render_workspace_snapshot(&snapshot_a);
    assert_eq!(rendered_a, render_workspace_snapshot(&snapshot_b));
    assert_eq!(rendered_a, render_workspace_snapshot(&snapshot_map));
    assert_eq!(snapshot_a.root_source_id(), "mixed.suite");
    assert_eq!(snapshot_a.source_count(), 9);
    assert!(snapshot_a.source("expected.suite").is_none());
    assert!(snapshot_a.source("models/system.fvl").unwrap().contains('Ω'));
    assert!(!rendered_a.contains(root_a.to_str().unwrap()));
    assert!(!rendered_a.contains(root_b.to_str().unwrap()));

    let parsed = parse_workspace_snapshot(&rendered_a).unwrap();
    assert_eq!(parsed, snapshot_a);
    assert_eq!(render_workspace_snapshot(&parsed), rendered_a);

    fs::remove_dir_all(root_a).unwrap();
    fs::remove_dir_all(root_b).unwrap();
}

#[test]
fn snapshot_replay_survives_host_workspace_removal_and_matches_direct_provider() {
    let sources = fixture_sources();
    let root = fixture_dir("offline");
    write_sources(&root, &sources);
    let rooted = RootedFileSystemTextSourceProvider::new(&root);
    let snapshot = create_workspace_snapshot(&rooted, "mixed.suite").unwrap();

    let map = MapTextSourceProvider::from_sources(sources).unwrap();
    let direct = run_orchestration_suite_json_with_provider(&map, "mixed.suite");
    fs::remove_dir_all(&root).unwrap();

    let replay = replay_workspace_snapshot_json(&snapshot);
    assert_eq!(replay.exit_code, direct.exit_code);
    assert_eq!(replay.to_json(), direct.to_json());
    assert_eq!(replay.envelope.status, OrchestrationSuiteStatus::Complete);
    assert_eq!(replay.envelope.jobs[0].result.outcome_str(), "satisfied");
    assert_eq!(replay.envelope.jobs[1].result.outcome_str(), "violated");
    assert_eq!(replay.envelope.jobs[2].result.outcome_str(), "cycle_found");
    assert_eq!(replay.envelope.jobs[3].result.outcome_str(), "verified");
}

#[test]
fn expectation_snapshot_replay_matches_direct_provider() {
    let sources = fixture_sources();
    let map = MapTextSourceProvider::from_sources(sources).unwrap();
    let snapshot = create_workspace_snapshot(&map, "expected.suite").unwrap();

    let direct =
        run_orchestration_suite_expectations_json_with_provider(&map, "expected.suite");
    let replay = replay_workspace_snapshot_expectations_json(&snapshot);

    assert_eq!(replay.exit_code, 0);
    assert_eq!(replay.to_json(), direct.to_json());
    assert_eq!(replay.envelope.outcome, OrchestrationRegressionOutcome::Matched);
    assert_eq!(replay.envelope.execution_status, OrchestrationSuiteStatus::Complete);
}

#[test]
fn snapshot_replays_rejected_certificate_and_setup_error_without_host_files() {
    let mut rejected_sources = fixture_sources();
    let certificate = rejected_sources
        .iter_mut()
        .find(|(source_id, _)| source_id == "certificates/reach.mupc")
        .unwrap();
    certificate.1 = certificate
        .1
        .lines()
        .filter(|line| *line != "end")
        .collect::<Vec<_>>()
        .join("\n");
    let rejected_provider = MapTextSourceProvider::from_sources(rejected_sources).unwrap();
    let rejected_snapshot =
        create_workspace_snapshot(&rejected_provider, "mixed.suite").unwrap();
    let rejected = replay_workspace_snapshot_json(&rejected_snapshot);
    assert_eq!(rejected.exit_code, 16);
    assert_eq!(rejected.envelope.status, OrchestrationSuiteStatus::Attention);
    assert_eq!(rejected.envelope.jobs[3].result.outcome_str(), "rejected");

    let mut error_sources = fixture_sources();
    let property = error_sources
        .iter_mut()
        .find(|(source_id, _)| source_id == "properties/reach.mu")
        .unwrap();
    property.1 = "mu X. not $X".to_owned();
    let error_provider = MapTextSourceProvider::from_sources(error_sources).unwrap();
    let error_snapshot = create_workspace_snapshot(&error_provider, "mixed.suite").unwrap();
    let error = replay_workspace_snapshot_json(&error_snapshot);
    assert_eq!(error.exit_code, 2);
    assert_eq!(error.envelope.status, OrchestrationSuiteStatus::Error);
    assert!(error
        .envelope
        .jobs
        .iter()
        .any(|job| job.result.outcome_str() == "error"));
}

#[test]
fn parser_fails_closed_on_truncation_trailing_payload_and_resource_caps() {
    let map = MapTextSourceProvider::from_sources(fixture_sources()).unwrap();
    let snapshot = create_workspace_snapshot(&map, "mixed.suite").unwrap();
    let rendered = render_workspace_snapshot(&snapshot);

    let truncated = &rendered[..rendered.len() - 4];
    assert!(parse_workspace_snapshot(truncated).is_err());

    let trailing = format!("{rendered}junk");
    assert!(matches!(
        parse_workspace_snapshot(&trailing),
        Err(error) if matches!(error.kind(), WorkspaceSnapshotParseErrorKind::TrailingPayload)
    ));

    let too_many = format!(
        "fvlab-workspace-snapshot 1\nroot 11\nmixed.suite\nentries {} 0\n",
        MAX_WORKSPACE_SNAPSHOT_ENTRIES + 1
    );
    assert!(matches!(
        parse_workspace_snapshot(&too_many),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::TooManyEntries { .. }
        )
    ));

    let too_large = format!(
        "fvlab-workspace-snapshot 1\nroot 11\nmixed.suite\nentries 0 {}\n",
        MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES + 1
    );
    assert!(matches!(
        parse_workspace_snapshot(&too_large),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::TooManySourceBytes { .. }
        )
    ));
}

#[test]
fn parser_rejects_noncanonical_duplicate_missing_and_invalid_utf8_frames() {
    let noncanonical = concat!(
        "fvlab-workspace-snapshot 1\n",
        "root 16\n",
        "a/../mixed.suite\n",
        "entries 0 0\n",
        "end\n"
    );
    assert!(matches!(
        parse_workspace_snapshot(noncanonical),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::NonCanonicalSourceId { .. }
        )
    ));

    let duplicate = concat!(
        "fvlab-workspace-snapshot 1\n",
        "root 11\n",
        "mixed.suite\n",
        "entries 2 2\n",
        "source 11 1\n",
        "mixed.suite\n",
        "x\n",
        "source 11 1\n",
        "mixed.suite\n",
        "y\n",
        "end\n"
    );
    assert!(matches!(
        parse_workspace_snapshot(duplicate),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::DuplicateSourceId { .. }
        )
    ));

    let root_missing = concat!(
        "fvlab-workspace-snapshot 1\n",
        "root 11\n",
        "mixed.suite\n",
        "entries 0 0\n",
        "end\n"
    );
    assert!(matches!(
        parse_workspace_snapshot(root_missing),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::RootMissing { .. }
        )
    ));

    let missing_dependency_text =
        "suite \"missing\"\njob \"verification\" \"jobs/missing.job\"";
    let missing_dependency = format!(
        "fvlab-workspace-snapshot 1\nroot 11\nmixed.suite\nentries 1 {}\nsource 11 {}\nmixed.suite\n{}\nend\n",
        missing_dependency_text.len(),
        missing_dependency_text.len(),
        missing_dependency_text
    );
    assert!(matches!(
        parse_workspace_snapshot(&missing_dependency),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::MissingDependency { .. }
        )
    ));

    let split_utf8 = concat!(
        "fvlab-workspace-snapshot 1\n",
        "root 1\n",
        "é\n"
    );
    assert!(matches!(
        parse_workspace_snapshot(split_utf8),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::InvalidUtf8Frame { .. }
        )
    ));
}

#[test]
fn parser_rejects_unrelated_embedded_source() {
    let map = MapTextSourceProvider::from_sources(fixture_sources()).unwrap();
    let snapshot = create_workspace_snapshot(&map, "mixed.suite").unwrap();
    let rendered = render_workspace_snapshot(&snapshot);
    let marker = "end\n";
    let prefix = rendered.strip_suffix(marker).unwrap();
    let old_header = format!(
        "entries {} {}",
        snapshot.source_count(),
        snapshot.total_source_bytes()
    );
    let new_header = format!(
        "entries {} {}",
        snapshot.source_count() + 1,
        snapshot.total_source_bytes() + 5
    );
    let prefix = prefix.replacen(&old_header, &new_header, 1);
    let with_extra = format!("{prefix}source 9 5\nextra.txt\nhello\nend\n");

    assert!(matches!(
        parse_workspace_snapshot(&with_extra),
        Err(error) if matches!(
            error.kind(),
            WorkspaceSnapshotParseErrorKind::UnexpectedSource { source_id }
                if source_id == "extra.txt"
        )
    ));
}

#[test]
fn built_workspace_cli_create_and_replay_match_library_after_source_removal() {
    let root = fixture_dir("cli");
    let sources = fixture_sources();
    write_sources(&root, &sources);
    let rooted = RootedFileSystemTextSourceProvider::new(&root);
    let library_snapshot = create_workspace_snapshot(&rooted, "expected.suite").unwrap();
    let expected_artifact = render_workspace_snapshot(&library_snapshot);
    let snapshot_path = root.with_extension("snapshot");

    let create = run_binary(&[
        "create",
        root.to_str().unwrap(),
        "expected.suite",
        snapshot_path.to_str().unwrap(),
    ]);
    assert_eq!(create.status.code(), Some(0));
    assert!(create.stdout.is_empty());
    assert!(create.stderr.is_empty());
    assert_eq!(fs::read_to_string(&snapshot_path).unwrap(), expected_artifact);

    fs::remove_dir_all(&root).unwrap();

    let expected = replay_workspace_snapshot_json(&library_snapshot);
    let replay = run_binary(&[
        "replay",
        snapshot_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(replay.status.code(), Some(expected.exit_code as i32));
    assert_eq!(
        String::from_utf8(replay.stdout).unwrap(),
        format!("{}\n", expected.to_json())
    );
    assert!(replay.stderr.is_empty());

    let expected_regression = replay_workspace_snapshot_expectations_json(&library_snapshot);
    let regression = run_binary(&[
        "replay",
        snapshot_path.to_str().unwrap(),
        "--check-expectations",
        "--format",
        "json",
    ]);
    assert_eq!(
        regression.status.code(),
        Some(expected_regression.exit_code as i32)
    );
    assert_eq!(
        String::from_utf8(regression.stdout).unwrap(),
        format!("{}\n", expected_regression.to_json())
    );
    assert!(regression.stderr.is_empty());

    fs::remove_file(snapshot_path).unwrap();
}

fn fixture_sources() -> Vec<(String, String)> {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate = create_declarative_mu_parity_certificate(&document, REACH_FORMULA).unwrap();
    let certificate_text = render_declarative_mu_parity_certificate(&certificate);

    vec![
        ("models/system.fvl".to_owned(), MODEL.to_owned()),
        (
            "properties/reach.mu".to_owned(),
            REACH_FORMULA.to_owned(),
        ),
        ("properties/false.mu".to_owned(), "false".to_owned()),
        ("certificates/reach.mupc".to_owned(), certificate_text),
        (
            "jobs/verify.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/reach.mu\"\n"
                .to_owned(),
        ),
        (
            "jobs/violate.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/false.mu\"\n"
                .to_owned(),
        ),
        (
            "jobs/structure.job".to_owned(),
            "analysis \"recurrence\"\nmodel \"../models/system.fvl\"\n".to_owned(),
        ),
        (
            "jobs/certificate.job".to_owned(),
            "model \"../models/system.fvl\"\nproperty \"../properties/reach.mu\"\ncertificate \"../certificates/reach.mupc\"\n"
                .to_owned(),
        ),
        (
            "mixed.suite".to_owned(),
            concat!(
                "suite \"mixed\"\n",
                "job \"verification\" \"jobs/verify.job\"\n",
                "job \"verification\" \"jobs/violate.job\"\n",
                "job \"structural\" \"jobs/structure.job\"\n",
                "job \"certificate-verification\" \"jobs/certificate.job\"\n"
            )
            .to_owned(),
        ),
        (
            "expected.suite".to_owned(),
            concat!(
                "suite \"expected\"\n",
                "job \"verification\" \"jobs/verify.job\" expect \"satisfied\"\n",
                "job \"verification\" \"jobs/violate.job\" expect \"violated\"\n",
                "job \"structural\" \"jobs/structure.job\" expect \"cycle_found\"\n",
                "job \"certificate-verification\" \"jobs/certificate.job\" expect \"verified\"\n"
            )
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
        "fvlab-m90-workspace-snapshot-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

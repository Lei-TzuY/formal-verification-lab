use formal_verification_lab::report::render_bounded_recurrence_report;
use formal_verification_lab::{
    analyze_recurrence_with_limits, parse_declarative_model, ExplorationLimits,
};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

const ACYCLIC: &str = r#"model "acyclic-file"
state "a"
state "b"
initial "a"
edge "a" "step" "b"
"#;

const CYCLIC: &str = r#"model "cyclic-file"
state "a"
initial "a"
edge "a" "loop" "a"
"#;

const CYCLE_BEFORE_CUTOFF: &str = r#"model "cycle-before-cutoff"
state "a"
state "b"
initial "a"
edge "a" "loop" "a"
edge "a" "later" "b"
"#;

fn temp_model(kind: &str, input: &str) -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("fvlab-m65-{kind}-{}-{id}.fvl", std::process::id()));
    fs::write(&path, input).expect("temporary model should be writable");
    path
}

fn run(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn direct_report(input: &str, limits: ExplorationLimits) -> String {
    let model = parse_declarative_model(input).expect("fixture model should parse");
    let result = analyze_recurrence_with_limits(&model, limits)
        .expect("direct bounded recurrence should succeed");
    render_bounded_recurrence_report(model.name(), &result)
}

#[test]
fn file_cli_complete_acyclic_matches_direct_backend_and_exposes_full_partition() {
    let path = temp_model("acyclic", ACYCLIC);
    let output = run(&[
        "scc".into(),
        "file".into(),
        path.display().to_string(),
        "--max-states".into(),
        "2".into(),
        "--max-transitions".into(),
        "1".into(),
        "--max-depth".into(),
        "1".into(),
    ]);
    let expected = direct_report(
        ACYCLIC,
        ExplorationLimits {
            max_states: Some(2),
            max_transitions: Some(1),
            max_depth: Some(1),
        },
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), expected);
    assert!(expected.contains("recurrence: ACYCLIC"));
    assert!(expected.contains("scc count: 2"));
    assert!(expected.contains("cutoff reason: none"));
    assert!(!expected.contains("scc partition: unavailable"));
    fs::remove_file(path).ok();
}

#[test]
fn file_cli_complete_cycle_matches_direct_backend_and_reports_closed_witness() {
    let path = temp_model("cycle", CYCLIC);
    let output = run(&["scc".into(), "file".into(), path.display().to_string()]);
    let expected = direct_report(CYCLIC, ExplorationLimits::unbounded());

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), expected);
    assert!(expected.contains("recurrence: CYCLE_FOUND"));
    assert!(expected.contains("scc count: 1"));
    assert!(expected.contains("cycle component: 0"));
    assert!(expected.contains("--loop-->"));
    fs::remove_file(path).ok();
}

#[test]
fn real_cycle_before_later_cutoff_is_conclusive_but_does_not_print_partial_partition() {
    let path = temp_model("cycle-cutoff", CYCLE_BEFORE_CUTOFF);
    let output = run(&[
        "scc".into(),
        "file".into(),
        path.display().to_string(),
        "--max-transitions".into(),
        "1".into(),
    ]);
    let expected = direct_report(
        CYCLE_BEFORE_CUTOFF,
        ExplorationLimits {
            max_transitions: Some(1),
            ..ExplorationLimits::unbounded()
        },
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output), expected);
    assert!(expected.contains("recurrence: CYCLE_FOUND"));
    assert!(expected.contains("cutoff reason: transition limit reached (max 1)"));
    assert!(expected.contains("scc partition: unavailable (incomplete exploration)"));
    assert!(expected.contains("--loop-->"));
    fs::remove_file(path).ok();
}

#[test]
fn cutoff_without_cycle_is_inconclusive_exit_three_and_never_claims_partition() {
    let path = temp_model("inconclusive", ACYCLIC);
    let output = run(&[
        "scc".into(),
        "file".into(),
        path.display().to_string(),
        "--max-transitions".into(),
        "0".into(),
    ]);
    let expected = direct_report(
        ACYCLIC,
        ExplorationLimits {
            max_transitions: Some(0),
            ..ExplorationLimits::unbounded()
        },
    );

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stdout(&output), expected);
    assert!(expected.contains("recurrence: INCONCLUSIVE"));
    assert!(expected.contains("cutoff reason: transition limit reached (max 0)"));
    assert!(expected.contains("scc partition: unavailable (incomplete exploration)"));
    assert!(expected.contains("cycle witness: none"));
    fs::remove_file(path).ok();
}

#[test]
fn file_cli_rejects_malformed_models_and_bad_or_duplicate_limit_options() {
    let malformed = temp_model(
        "malformed",
        "model \"broken\"\nstate \"a\"\ninitial \"missing\"\n",
    );
    let malformed_output = run(&["scc".into(), "file".into(), malformed.display().to_string()]);
    assert_eq!(malformed_output.status.code(), Some(2));
    assert!(stderr(&malformed_output).contains("error:"));
    assert!(stderr(&malformed_output).contains("missing"));

    let valid = temp_model("options", ACYCLIC);
    let duplicate = run(&[
        "scc".into(),
        "file".into(),
        valid.display().to_string(),
        "--max-states".into(),
        "1".into(),
        "--max-states".into(),
        "2".into(),
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("--max-states"));

    let malformed_limit = run(&[
        "scc".into(),
        "file".into(),
        valid.display().to_string(),
        "--max-depth".into(),
        "not-a-number".into(),
    ]);
    assert_eq!(malformed_limit.status.code(), Some(2));
    assert!(stderr(&malformed_limit).contains("--max-depth"));

    fs::remove_file(malformed).ok();
    fs::remove_file(valid).ok();
}

#[test]
fn historical_hard_coded_scc_examples_keep_their_existing_surface() {
    let acyclic = run(&["scc".into(), "counter".into()]);
    assert_eq!(acyclic.status.code(), Some(0));
    let acyclic_stdout = stdout(&acyclic);
    assert!(acyclic_stdout.contains("recurrence: ACYCLIC"));
    assert!(acyclic_stdout.contains("cyclic scc count: 0"));
    assert!(!acyclic_stdout.contains("cutoff reason:"));

    let cyclic = run(&["scc".into(), "traffic-light".into()]);
    assert_eq!(cyclic.status.code(), Some(0));
    let cyclic_stdout = stdout(&cyclic);
    assert!(cyclic_stdout.contains("recurrence: CYCLIC"));
    assert!(cyclic_stdout.contains("cycle component:"));
    assert!(!cyclic_stdout.contains("cutoff reason:"));
}

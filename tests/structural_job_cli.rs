use formal_verification_lab::run_structural_job_json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const ACYCLIC: &str = r#"model "acyclic-job"
state "a"
state "b"
initial "a"
edge "a" "step" "b"
"#;

const CYCLIC: &str = r#"model "cyclic-job"
state "a"
initial "a"
edge "a" "loop" "a"
"#;

const CYCLE_BEFORE_CUTOFF: &str = r#"model "cycle-cutoff-job"
state "a"
state "b"
initial "a"
edge "a" "loop" "a"
edge "a" "later" "b"
"#;

fn temp_root(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("fvlab-m66-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(root.join("models")).unwrap();
    root
}

fn write_job(root: &Path, model: &str, extra: &str) -> PathBuf {
    fs::write(root.join("models/graph.fvl"), model).unwrap();
    let manifest = root.join("job.fvstruct");
    fs::write(
        &manifest,
        format!(
            "analysis \"recurrence\"\nmodel \"models/graph.fvl\"{}",
            if extra.is_empty() {
                String::new()
            } else {
                format!("\n{extra}")
            }
        ),
    )
    .unwrap();
    manifest
}

fn run_cli(args: &[String]) -> Output {
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

#[test]
fn relative_model_job_is_reproducible_and_cli_json_matches_library_runner() {
    let root = temp_root("relative");
    let manifest = write_job(
        &root,
        ACYCLIC,
        "max-states 2\nmax-transitions 1\nmax-depth 1",
    );

    let direct = run_structural_job_json(&manifest);
    let output = run_cli(&[
        "scc".into(),
        "job".into(),
        manifest.display().to_string(),
        "--format".into(),
        "json".into(),
    ]);

    assert_eq!(direct.exit_code, 0);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout(&output).trim_end(), direct.to_json());
    assert!(direct.to_json().contains("\"outcome\":\"acyclic\""));
    assert!(direct.to_json().contains("\"model\":\"acyclic-job\""));
    assert!(direct.to_json().contains("\"components\":["));

    fs::remove_dir_all(root).ok();
}

#[test]
fn complete_cycle_job_is_neutral_success_with_cycle_evidence() {
    let root = temp_root("cycle");
    let manifest = write_job(&root, CYCLIC, "");
    let output = run_cli(&["scc".into(), "job".into(), manifest.display().to_string()]);
    let json = stdout(&output);

    assert_eq!(output.status.code(), Some(0));
    assert!(json.contains("\"outcome\":\"cycle_found\""));
    assert!(json.contains("\"cutoff\":null"));
    assert!(json.contains("\"components\":["));
    assert!(json.contains("\"action\":\"loop\""));
    assert!(!json.contains("violated"));

    fs::remove_dir_all(root).ok();
}

#[test]
fn conclusive_cycle_before_cutoff_keeps_cutoff_but_not_partial_partition() {
    let root = temp_root("cycle-cutoff");
    let manifest = write_job(&root, CYCLE_BEFORE_CUTOFF, "max-transitions 1");
    let output = run_cli(&["scc".into(), "job".into(), manifest.display().to_string()]);
    let json = stdout(&output);

    assert_eq!(output.status.code(), Some(0));
    assert!(json.contains("\"outcome\":\"cycle_found\""));
    assert!(json.contains("\"cutoff\":{\"kind\":\"transition_limit\",\"limit\":1}"));
    assert!(json.contains("\"components\":null"));
    assert!(json.contains("\"evidence\":{\"component_index\":0"));
    assert!(json.contains("\"action\":\"loop\""));

    fs::remove_dir_all(root).ok();
}

#[test]
fn incomplete_acyclic_prefix_is_inconclusive_exit_three() {
    let root = temp_root("inconclusive");
    let manifest = write_job(&root, ACYCLIC, "max-transitions 0");
    let output = run_cli(&["scc".into(), "job".into(), manifest.display().to_string()]);
    let json = stdout(&output);

    assert_eq!(output.status.code(), Some(3));
    assert!(json.contains("\"outcome\":\"inconclusive\""));
    assert!(json.contains("\"cutoff\":{\"kind\":\"transition_limit\",\"limit\":0}"));
    assert!(json.contains("\"components\":null"));
    assert!(json.contains("\"evidence\":null"));

    fs::remove_dir_all(root).ok();
}

#[test]
fn verification_only_directives_fail_closed_as_structural_job_errors() {
    for (index, directive) in [
        "property \"fake.property\"",
        "weak-fair-action \"tick\"",
        "strong-fair-action \"tick\"",
        "max-product-states 1",
    ]
    .into_iter()
    .enumerate()
    {
        let root = temp_root(&format!("invalid-{index}"));
        let manifest = write_job(&root, ACYCLIC, directive);
        let output = run_cli(&[
            "scc".into(),
            "job".into(),
            manifest.display().to_string(),
        ]);
        let json = stdout(&output);

        assert_eq!(output.status.code(), Some(2), "{directive}: {json}");
        assert!(
            json.contains("\"outcome\":\"error\""),
            "{directive}: {json}"
        );
        assert!(
            json.contains("unsupported directive"),
            "{directive}: {json}"
        );

        fs::remove_dir_all(root).ok();
    }
}

#[test]
fn malformed_model_missing_model_file_and_unsupported_format_fail_closed() {
    let root = temp_root("errors");
    let manifest = write_job(
        &root,
        "model \"broken\"\nstate \"a\"\ninitial \"missing\"\n",
        "",
    );

    let malformed = run_cli(&["scc".into(), "job".into(), manifest.display().to_string()]);
    assert_eq!(malformed.status.code(), Some(2));
    assert!(stdout(&malformed).contains("\"outcome\":\"error\""));

    let missing_manifest = root.join("missing-job.fvstruct");
    fs::write(
        &missing_manifest,
        "analysis \"recurrence\"\nmodel \"models/missing.fvl\"",
    )
    .unwrap();
    let missing = run_cli(&[
        "scc".into(),
        "job".into(),
        missing_manifest.display().to_string(),
    ]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stdout(&missing).contains("failed to read declarative model"));

    let format = run_cli(&[
        "scc".into(),
        "job".into(),
        manifest.display().to_string(),
        "--format".into(),
        "yaml".into(),
    ]);
    assert_eq!(format.status.code(), Some(2));
    assert!(stdout(&format).is_empty());
    assert!(stderr(&format).contains("expected json"));

    fs::remove_dir_all(root).ok();
}

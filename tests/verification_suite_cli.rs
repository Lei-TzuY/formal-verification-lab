use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-suite"))
        .args(args)
        .output()
        .expect("fvlab-suite binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("fvlab-m57-cli-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn property_source() -> &'static str {
    "response(\"request\",\"request\",\"grant\")\n"
}

fn satisfied_model_source() -> &'static str {
    "model \"satisfied\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn unfair_model_source() -> &'static str {
    "model \"unfair\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\nedge \"waiting\" \"wait\" \"waiting\"\nedge \"waiting\" \"grant\" \"idle\"\n"
}

fn write_job(root: &Path, name: &str, model_source: &str, tail: &str) {
    fs::write(root.join(format!("{name}.fvl")), model_source).unwrap();
    fs::write(root.join(format!("{name}.fvt")), property_source()).unwrap();
    fs::write(
        root.join(format!("{name}.fvj")),
        format!("model \"{name}.fvl\"\nproperty \"{name}.fvt\"\n{tail}"),
    )
    .unwrap();
}

fn args(suite: &Path) -> Vec<String> {
    vec![
        suite.to_string_lossy().into_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ]
}

#[test]
fn suite_binary_preserves_order_and_aggregate_violation_precedence() {
    let root = fixture_dir("aggregate");
    write_job(&root, "satisfied", satisfied_model_source(), "");
    write_job(
        &root,
        "inconclusive",
        unfair_model_source(),
        "max-product-transitions 1\n",
    );
    write_job(&root, "violated", unfair_model_source(), "");
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"nightly\"\njob \"satisfied.fvj\"\njob \"inconclusive.fvj\"\njob \"violated.fvj\"\n",
    )
    .unwrap();

    let first = run(&args(&suite));
    let second = run(&args(&suite));
    assert_eq!(first.status.code(), Some(7));
    assert!(stderr(&first).is_empty());
    assert_eq!(stdout(&first), stdout(&second));
    let json = stdout(&first);
    assert!(json.starts_with(
        "{\"schema_version\":1,\"outcome\":\"violated\",\"status\":\"VIOLATED\",\"suite\":\"nightly\""
    ));
    let satisfied = json.find("\"manifest\":\"satisfied.fvj\"").unwrap();
    let inconclusive = json
        .find("\"manifest\":\"inconclusive.fvj\"")
        .unwrap();
    let violated = json.find("\"manifest\":\"violated.fvj\"").unwrap();
    assert!(satisfied < inconclusive && inconclusive < violated);
    assert!(json.contains("\"outcome\":\"inconclusive\",\"status\":\"INCONCLUSIVE\""));
    assert!(json.contains("\"kind\":\"lasso\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn suite_binary_keeps_later_results_after_job_error() {
    let root = fixture_dir("error");
    fs::write(root.join("bad.fvj"), "model \"missing.fvl\"\n").unwrap();
    write_job(&root, "satisfied", satisfied_model_source(), "");
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"errors\"\njob \"bad.fvj\"\njob \"satisfied.fvj\"\n",
    )
    .unwrap();

    let output = run(&args(&suite));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).is_empty());
    let json = stdout(&output);
    assert!(json.contains("\"outcome\":\"error\",\"status\":null"));
    assert!(json.contains("\"manifest\":\"bad.fvj\""));
    assert!(json.contains("\"manifest\":\"satisfied.fvj\""));
    assert!(json.contains("\"model\":\"satisfied\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn malformed_suite_is_one_structured_error_envelope() {
    let root = fixture_dir("malformed-suite");
    let suite = root.join("suite.fvs");
    fs::write(&suite, "suite \"missing-jobs\"\n").unwrap();

    let output = run(&args(&suite));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).is_empty());
    let json = stdout(&output);
    assert!(json.starts_with(
        "{\"schema_version\":1,\"outcome\":\"error\",\"status\":null,\"suite\":null,\"jobs\":[]"
    ));
    assert!(json.contains("verification suite requires at least one job"));

    let _ = fs::remove_dir_all(root);
}

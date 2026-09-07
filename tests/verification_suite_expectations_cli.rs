use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run_suite(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab-suite"))
        .args(args)
        .output()
        .expect("fvlab-suite binary should execute")
}

fn run_job_json(manifest: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args([
            "temporal",
            "job",
            manifest.to_str().expect("fixture path should be UTF-8"),
            "--format",
            "json",
        ])
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m58-cli-{kind}-{}-{id}",
        std::process::id()
    ));
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

fn terminal_model_source() -> &'static str {
    "model \"terminal\"\nstate \"idle\"\nstate \"waiting\"\ninitial \"idle\"\nedge \"idle\" \"request\" \"waiting\"\n"
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

fn expectation_args(suite: &Path) -> Vec<String> {
    vec![
        suite.to_string_lossy().into_owned(),
        "--check-expectations".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ]
}

fn raw_args(suite: &Path) -> Vec<String> {
    vec![
        suite.to_string_lossy().into_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ]
}

fn write_all_job_classes(root: &Path) -> [&'static str; 7] {
    fs::write(root.join("bad.fvj"), "model \"missing.fvl\"\n").unwrap();
    write_job(root, "satisfied", satisfied_model_source(), "");
    write_job(root, "finite", terminal_model_source(), "");
    write_job(root, "lasso", unfair_model_source(), "");
    write_job(
        root,
        "product-cutoff",
        unfair_model_source(),
        "max-product-transitions 1\n",
    );
    write_job(
        root,
        "model-cutoff",
        unfair_model_source(),
        "max-model-transitions 1\n",
    );
    write_job(
        root,
        "mixed-fair",
        unfair_model_source(),
        "weak-fair-action \"grant\"\nstrong-fair-action \"unrelated\"\n",
    );
    [
        "bad",
        "satisfied",
        "finite",
        "lasso",
        "product-cutoff",
        "model-cutoff",
        "mixed-fair",
    ]
}

#[test]
fn expectation_binary_matches_all_m57_job_classes_and_preserves_direct_envelopes() {
    let root = fixture_dir("matched-all-classes");
    let names = write_all_job_classes(&root);
    let expected = [
        ("error", "error", Some(2)),
        ("satisfied", "satisfied", Some(0)),
        ("violated", "violated", Some(7)),
        ("violated", "violated", Some(7)),
        ("inconclusive", "inconclusive", Some(3)),
        ("inconclusive", "inconclusive", Some(3)),
        ("satisfied", "satisfied", Some(0)),
    ];

    let suite = root.join("suite.fvs");
    let mut source = String::from("suite \"expected-all-classes\"\n");
    for (name, (expected_outcome, _, _)) in names.iter().zip(expected.iter()) {
        source.push_str(&format!(
            "job \"{name}.fvj\" expect \"{expected_outcome}\"\n"
        ));
    }
    fs::write(&suite, source).unwrap();

    let first = run_suite(&expectation_args(&suite));
    let second = run_suite(&expectation_args(&suite));
    assert_eq!(first.status.code(), Some(0));
    assert!(stderr(&first).is_empty());
    assert_eq!(stdout(&first), stdout(&second));
    let suite_json = stdout(&first);
    assert!(suite_json.starts_with(
        "{\"schema_version\":1,\"outcome\":\"matched\",\"suite\":\"expected-all-classes\""
    ));

    let mut previous_position = None;
    for (name, (expected_outcome, observed, direct_exit)) in names.iter().zip(expected.iter()) {
        let manifest = root.join(format!("{name}.fvj"));
        let direct = run_job_json(&manifest);
        assert_eq!(direct.status.code(), *direct_exit, "job {name}");
        assert!(stderr(&direct).is_empty(), "job {name}: {}", stderr(&direct));
        let direct_json = stdout(&direct).trim_end().to_owned();
        let entry = format!(
            "{{\"manifest\":\"{name}.fvj\",\"expected\":\"{expected_outcome}\",\"observed\":\"{observed}\",\"matched\":true,\"result\":{direct_json}}}"
        );
        assert!(
            suite_json.contains(&entry),
            "expectation entry for {name} must preserve the complete direct M56 envelope"
        );

        let position = suite_json
            .find(&format!("\"manifest\":\"{name}.fvj\""))
            .expect("regression suite should retain every manifest");
        if let Some(previous) = previous_position {
            assert!(previous < position, "regression suite order must be stable");
        }
        previous_position = Some(position);
    }

    assert!(suite_json.contains("\"kind\":\"finite\""));
    assert!(suite_json.contains("\"kind\":\"lasso\""));
    assert!(suite_json.contains("\"stage\":\"product\""));
    assert!(suite_json.contains("\"stage\":\"model\""));
    assert!(suite_json.contains("\"weak_fair_actions\":[\"grant\"]"));
    assert!(suite_json.contains("\"strong_fair_actions\":[\"unrelated\"]"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn expectation_binary_retains_every_mismatch_in_order_and_exits_thirteen() {
    let root = fixture_dir("mismatch-all-classes");
    let names = write_all_job_classes(&root);
    let wrong_expected = [
        ("satisfied", "error"),
        ("violated", "satisfied"),
        ("satisfied", "violated"),
        ("satisfied", "violated"),
        ("violated", "inconclusive"),
        ("error", "inconclusive"),
        ("inconclusive", "satisfied"),
    ];

    let suite = root.join("suite.fvs");
    let mut source = String::from("suite \"mismatch-all-classes\"\n");
    for (name, (expected_outcome, _)) in names.iter().zip(wrong_expected.iter()) {
        source.push_str(&format!(
            "job \"{name}.fvj\" expect \"{expected_outcome}\"\n"
        ));
    }
    fs::write(&suite, source).unwrap();

    let output = run_suite(&expectation_args(&suite));
    assert_eq!(output.status.code(), Some(13));
    assert!(stderr(&output).is_empty());
    let json = stdout(&output);
    assert!(json.starts_with(
        "{\"schema_version\":1,\"outcome\":\"mismatched\",\"suite\":\"mismatch-all-classes\""
    ));
    assert_eq!(json.matches("\"matched\":false").count(), names.len());

    let mut previous_position = None;
    for (name, (expected_outcome, observed)) in names.iter().zip(wrong_expected.iter()) {
        let marker = format!(
            "\"manifest\":\"{name}.fvj\",\"expected\":\"{expected_outcome}\",\"observed\":\"{observed}\",\"matched\":false"
        );
        let position = json
            .find(&marker)
            .unwrap_or_else(|| panic!("missing mismatch entry for {name}"));
        if let Some(previous) = previous_position {
            assert!(previous < position, "mismatch order must follow suite input");
        }
        previous_position = Some(position);
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn expectation_binary_fails_closed_before_jobs_when_an_expectation_is_missing() {
    let root = fixture_dir("missing-expectation");
    write_job(&root, "satisfied", satisfied_model_source(), "");
    let suite = root.join("suite.fvs");
    fs::write(&suite, "suite \"missing\"\njob \"satisfied.fvj\"\n").unwrap();

    let output = run_suite(&expectation_args(&suite));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).is_empty());
    let json = stdout(&output);
    assert!(json.starts_with(
        "{\"schema_version\":1,\"outcome\":\"error\",\"suite\":null,\"jobs\":[]"
    ));
    assert!(json.contains("expectation check requires every verification suite job"));
    assert!(!json.contains("\"model\":\"satisfied\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn expectation_flag_is_explicit_and_raw_m57_behavior_remains_unchanged() {
    let root = fixture_dir("raw-compat");
    write_job(&root, "violated", unfair_model_source(), "");
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"raw-compat\"\njob \"violated.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let raw = run_suite(&raw_args(&suite));
    assert_eq!(raw.status.code(), Some(7));
    assert!(stderr(&raw).is_empty());
    let raw_json = stdout(&raw);
    assert!(raw_json.starts_with(
        "{\"schema_version\":1,\"outcome\":\"violated\",\"status\":\"VIOLATED\",\"suite\":\"raw-compat\""
    ));
    assert!(!raw_json.contains("\"expected\""));
    assert!(!raw_json.contains("\"matched\""));

    let checked = run_suite(&expectation_args(&suite));
    assert_eq!(checked.status.code(), Some(13));
    assert!(stderr(&checked).is_empty());
    assert!(stdout(&checked).contains("\"expected\":\"satisfied\",\"observed\":\"violated\",\"matched\":false"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn expectation_binary_rejects_malformed_cli_usage_without_running_a_suite() {
    let root = fixture_dir("usage");
    write_job(&root, "satisfied", satisfied_model_source(), "");
    let suite = root.join("suite.fvs");
    fs::write(
        &suite,
        "suite \"usage\"\njob \"satisfied.fvj\" expect \"satisfied\"\n",
    )
    .unwrap();

    let missing_format = run_suite(&[
        suite.to_string_lossy().into_owned(),
        "--check-expectations".to_owned(),
    ]);
    assert_eq!(missing_format.status.code(), Some(2));
    assert!(stdout(&missing_format).is_empty());
    assert!(stderr(&missing_format).contains(
        "usage: fvlab-suite <suite-manifest> [--check-expectations] --format json"
    ));

    let wrong_order = run_suite(&[
        suite.to_string_lossy().into_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        "--check-expectations".to_owned(),
    ]);
    assert_eq!(wrong_order.status.code(), Some(2));
    assert!(stdout(&wrong_order).is_empty());
    assert!(stderr(&wrong_order).contains("usage: fvlab-suite"));

    let wrong_format = run_suite(&[
        suite.to_string_lossy().into_owned(),
        "--check-expectations".to_owned(),
        "--format".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(wrong_format.status.code(), Some(2));
    assert!(stdout(&wrong_format).is_empty());
    assert!(stderr(&wrong_format).contains("usage: fvlab-suite"));

    let _ = fs::remove_dir_all(root);
}

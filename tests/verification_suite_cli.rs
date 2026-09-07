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
    let root =
        std::env::temp_dir().join(format!("fvlab-m57-cli-{kind}-{}-{id}", std::process::id()));
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
    let inconclusive = json.find("\"manifest\":\"inconclusive.fvj\"").unwrap();
    let violated = json.find("\"manifest\":\"violated.fvj\"").unwrap();
    assert!(satisfied < inconclusive && inconclusive < violated);
    assert!(json.contains("\"outcome\":\"inconclusive\",\"status\":\"INCONCLUSIVE\""));
    assert!(json.contains("\"kind\":\"lasso\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn suite_binary_differentially_preserves_every_m57_job_class() {
    let root = fixture_dir("differential");
    fs::write(root.join("bad.fvj"), "model \"missing.fvl\"\n").unwrap();
    write_job(&root, "satisfied", satisfied_model_source(), "");
    write_job(&root, "finite", terminal_model_source(), "");
    write_job(&root, "lasso", unfair_model_source(), "");
    write_job(
        &root,
        "product-cutoff",
        unfair_model_source(),
        "max-product-transitions 1\n",
    );
    write_job(
        &root,
        "model-cutoff",
        unfair_model_source(),
        "max-model-transitions 1\n",
    );
    write_job(
        &root,
        "mixed-fair",
        unfair_model_source(),
        "weak-fair-action \"grant\"\nstrong-fair-action \"unrelated\"\n",
    );

    let names = [
        "bad",
        "satisfied",
        "finite",
        "lasso",
        "product-cutoff",
        "model-cutoff",
        "mixed-fair",
    ];
    let suite = root.join("suite.fvs");
    let mut suite_source = String::from("suite \"differential\"\n");
    for name in names {
        suite_source.push_str(&format!("job \"{name}.fvj\"\n"));
    }
    fs::write(&suite, suite_source).unwrap();

    let suite_output = run(&args(&suite));
    assert_eq!(suite_output.status.code(), Some(2));
    assert!(stderr(&suite_output).is_empty());
    let suite_json = stdout(&suite_output);

    let expected_exit_codes = [
        Some(2),
        Some(0),
        Some(7),
        Some(7),
        Some(3),
        Some(3),
        Some(0),
    ];
    let mut previous_position = None;
    for (name, expected_exit) in names.into_iter().zip(expected_exit_codes) {
        let manifest = root.join(format!("{name}.fvj"));
        let direct = run_job_json(&manifest);
        assert_eq!(direct.status.code(), expected_exit, "job {name}");
        assert!(
            stderr(&direct).is_empty(),
            "job {name}: {}",
            stderr(&direct)
        );
        let direct_json = stdout(&direct).trim_end().to_owned();
        let expected_entry = format!("{{\"manifest\":\"{name}.fvj\",\"result\":{direct_json}}}");
        assert!(
            suite_json.contains(&expected_entry),
            "suite entry for {name} must exactly preserve the direct M56 envelope"
        );

        let position = suite_json
            .find(&format!("\"manifest\":\"{name}.fvj\""))
            .expect("suite result should retain every manifest");
        if let Some(previous) = previous_position {
            assert!(
                previous < position,
                "suite entry order must be deterministic"
            );
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

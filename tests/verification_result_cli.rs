use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
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
    let root = std::env::temp_dir().join(format!("fvlab-m56-{kind}-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn unfair_model_source() -> &'static str {
    "model \"job-dual-unfair\"\nstate \"idle\"\nstate \"await-a\"\nstate \"ready-b\"\nstate \"await-b\"\ninitial \"idle\"\nedge \"idle\" \"request-a\" \"await-a\"\nedge \"await-a\" \"grant-a\" \"ready-b\"\nedge \"ready-b\" \"request-b\" \"await-b\"\nedge \"await-b\" \"wait-b\" \"await-b\"\nedge \"await-b\" \"grant-b\" \"idle\"\n"
}

fn terminal_model_source() -> &'static str {
    "model \"job-dual-terminal\"\nstate \"idle\"\nstate \"await-a\"\nstate \"ready-b\"\nstate \"await-b\"\ninitial \"idle\"\nedge \"idle\" \"request-a\" \"await-a\"\nedge \"await-a\" \"grant-a\" \"ready-b\"\nedge \"ready-b\" \"request-b\" \"await-b\"\n"
}

fn property_source() -> &'static str {
    "response(\"class-a\",\"request-a\",\"grant-a\")\nresponse(\"class-b\",\"request-b\",\"grant-b\")\n"
}

fn write_fixture(root: &Path, model_source: &str, manifest_tail: &str) -> PathBuf {
    let model = root.join("model.fvl");
    let property = root.join("property.fvt");
    let manifest = root.join("job.fvj");
    fs::write(&model, model_source).unwrap();
    fs::write(&property, property_source()).unwrap();
    fs::write(
        &manifest,
        format!("model \"model.fvl\"\nproperty \"property.fvt\"\n{manifest_tail}"),
    )
    .unwrap();
    manifest
}

fn json_args(manifest: &Path) -> Vec<String> {
    vec![
        "temporal".to_owned(),
        "job".to_owned(),
        manifest.to_string_lossy().into_owned(),
        "--format".to_owned(),
        "json".to_owned(),
    ]
}

#[test]
fn json_job_reports_lasso_violation_with_canonical_clause_evidence() {
    let root = fixture_dir("lasso");
    let manifest = write_fixture(&root, unfair_model_source(), "");

    let first = run(&json_args(&manifest));
    let second = run(&json_args(&manifest));
    assert_eq!(first.status.code(), Some(7));
    assert_eq!(stdout(&first), stdout(&second));
    assert!(stderr(&first).is_empty());
    let json = stdout(&first);
    assert!(
        json.starts_with("{\"schema_version\":1,\"outcome\":\"violated\",\"status\":\"VIOLATED\"")
    );
    assert!(json.contains("\"model\":\"job-dual-unfair\""));
    assert!(json.contains("\"kind\":\"lasso\",\"clause\":\"class-b\""));
    assert!(json.contains("\"action\":\"wait-b\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn json_job_reports_applied_mixed_fairness_and_satisfaction() {
    let root = fixture_dir("fair");
    let manifest = write_fixture(
        &root,
        unfair_model_source(),
        "weak-fair-action \"grant-b\"\nstrong-fair-action \"unrelated\"\n",
    );

    let output = run(&json_args(&manifest));
    assert!(output.status.success(), "{}", stderr(&output));
    let json = stdout(&output);
    assert!(json.contains("\"outcome\":\"satisfied\",\"status\":\"SATISFIED\""));
    assert!(json.contains("\"weak_fair_actions\":[\"grant-b\"]"));
    assert!(json.contains("\"strong_fair_actions\":[\"unrelated\"]"));
    assert!(json.contains("\"evidence\":null"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn json_job_preserves_finite_terminal_precedence() {
    let root = fixture_dir("finite");
    let manifest = write_fixture(
        &root,
        terminal_model_source(),
        "weak-fair-action \"grant-b\"\nstrong-fair-action \"unrelated\"\n",
    );

    let output = run(&json_args(&manifest));
    assert_eq!(output.status.code(), Some(7));
    let json = stdout(&output);
    assert!(json.contains("\"kind\":\"finite\",\"clause\":\"class-b\""));
    assert!(!json.contains("\"kind\":\"lasso\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn json_job_preserves_product_and_model_cutoff_stage() {
    let product_root = fixture_dir("product-cutoff");
    let product_manifest = write_fixture(
        &product_root,
        unfair_model_source(),
        "max-product-transitions 3\n",
    );
    let product = run(&json_args(&product_manifest));
    assert_eq!(product.status.code(), Some(3));
    let product_json = stdout(&product);
    assert!(product_json.contains("\"outcome\":\"inconclusive\",\"status\":\"INCONCLUSIVE\""));
    assert!(product_json.contains("\"cutoff\":{\"stage\":\"product\""));

    let model_root = fixture_dir("model-cutoff");
    let model_manifest = write_fixture(
        &model_root,
        unfair_model_source(),
        "max-model-transitions 2\n",
    );
    let model = run(&json_args(&model_manifest));
    assert_eq!(model.status.code(), Some(3));
    let model_json = stdout(&model);
    assert!(model_json.contains("\"outcome\":\"inconclusive\",\"status\":\"INCONCLUSIVE\""));
    assert!(model_json.contains("\"cutoff\":{\"stage\":\"model\""));

    let _ = fs::remove_dir_all(product_root);
    let _ = fs::remove_dir_all(model_root);
}

#[test]
fn json_job_malformed_input_is_structured_error_with_exit_two() {
    let root = fixture_dir("error");
    let manifest = root.join("job.fvj");
    fs::write(
        &manifest,
        "model \"m\\\"odel.fvl\"\nproperty \"p.fvt\"\nmax-product-states 1\nmax-product-states 2\n",
    )
    .unwrap();

    let output = run(&json_args(&manifest));
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).is_empty());
    let json = stdout(&output);
    assert!(json.starts_with("{\"schema_version\":1,\"outcome\":\"error\",\"status\":null"));
    assert!(json.contains("duplicate singleton directive 'max-product-states'"));
    assert!(json.ends_with("}\n"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn historical_human_job_surface_is_not_reformatted_as_json() {
    let root = fixture_dir("human-compatibility");
    let manifest = write_fixture(&root, unfair_model_source(), "");
    let args = vec![
        "temporal".to_owned(),
        "job".to_owned(),
        manifest.to_string_lossy().into_owned(),
    ];
    let output = run(&args);

    assert_eq!(output.status.code(), Some(7));
    let human = stdout(&output);
    assert!(human.contains("multi-response: VIOLATED"));
    assert!(human.contains("violated clause: class-b"));
    assert!(!human.starts_with('{'));

    let _ = fs::remove_dir_all(root);
}

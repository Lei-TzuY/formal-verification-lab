use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn run(args: &[String], current_dir: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fvlab"));
    command.args(args);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    command.output().expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("CLI stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("CLI stderr should be UTF-8")
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m55-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("fixture directory should be creatable");
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

fn write_fixture(root: &Path, model_source: &str, manifest_tail: &str) -> (PathBuf, PathBuf, PathBuf) {
    let assets = root.join("portable");
    fs::create_dir_all(&assets).expect("portable directory should be creatable");
    let model = assets.join("model.fvl");
    let property = assets.join("property.fvt");
    let manifest = assets.join("job.fvj");
    fs::write(&model, model_source).expect("model fixture should be writable");
    fs::write(&property, property_source()).expect("property fixture should be writable");
    fs::write(
        &manifest,
        format!(
            "model \"model.fvl\"\nproperty \"property.fvt\"\n{manifest_tail}"
        ),
    )
    .expect("manifest fixture should be writable");
    (model, property, manifest)
}

fn job_args(manifest: &Path) -> Vec<String> {
    vec![
        "temporal".to_owned(),
        "job".to_owned(),
        manifest.to_string_lossy().into_owned(),
    ]
}

fn explicit_args(model: &Path, property: &Path, options: &[&str]) -> Vec<String> {
    let mut args = vec![
        "temporal".to_owned(),
        "multi-file".to_owned(),
        model.to_string_lossy().into_owned(),
        property.to_string_lossy().into_owned(),
    ];
    args.extend(options.iter().map(|value| (*value).to_owned()));
    args
}

fn assert_equivalent(job: &Output, explicit: &Output) {
    assert_eq!(job.status.code(), explicit.status.code());
    assert_eq!(stdout(job), stdout(explicit));
    assert_eq!(stderr(job), stderr(explicit));
}

#[test]
fn job_resolves_relative_paths_against_manifest_not_process_cwd() {
    let root = fixture_dir("relocation");
    let (model, property, manifest) = write_fixture(&root, unfair_model_source(), "");
    let unrelated = root.join("unrelated-cwd");
    fs::create_dir_all(&unrelated).unwrap();

    let job = run(&job_args(&manifest), Some(&unrelated));
    let explicit = run(&explicit_args(&model, &property, &[]), Some(&unrelated));

    assert_eq!(job.status.code(), Some(7));
    assert_equivalent(&job, &explicit);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn job_mixed_fairness_is_exactly_equivalent_to_m54_options() {
    let root = fixture_dir("mixed-fairness");
    let (model, property, manifest) = write_fixture(
        &root,
        unfair_model_source(),
        "weak-fair-action \"grant-b\"\nstrong-fair-action \"unrelated\"\n",
    );

    let job = run(&job_args(&manifest), None);
    let explicit = run(
        &explicit_args(
            &model,
            &property,
            &[
                "--weak-fair-action",
                "grant-b",
                "--strong-fair-action",
                "unrelated",
            ],
        ),
        None,
    );

    assert!(job.status.success(), "{}", stderr(&job));
    assert_equivalent(&job, &explicit);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn job_product_and_model_cutoffs_preserve_exact_provenance() {
    let product_root = fixture_dir("product-cutoff");
    let (product_model, product_property, product_manifest) = write_fixture(
        &product_root,
        unfair_model_source(),
        "max-product-transitions 3\n",
    );
    let product_job = run(&job_args(&product_manifest), None);
    let product_explicit = run(
        &explicit_args(
            &product_model,
            &product_property,
            &["--max-product-transitions", "3"],
        ),
        None,
    );
    assert_eq!(product_job.status.code(), Some(3));
    assert_equivalent(&product_job, &product_explicit);

    let model_root = fixture_dir("model-cutoff");
    let (model, property, manifest) = write_fixture(
        &model_root,
        unfair_model_source(),
        "max-model-transitions 2\n",
    );
    let model_job = run(&job_args(&manifest), None);
    let model_explicit = run(
        &explicit_args(
            &model,
            &property,
            &["--max-model-transitions", "2"],
        ),
        None,
    );
    assert_eq!(model_job.status.code(), Some(3));
    assert_equivalent(&model_job, &model_explicit);

    let _ = fs::remove_dir_all(product_root);
    let _ = fs::remove_dir_all(model_root);
}

#[test]
fn job_fairness_never_excuses_finite_pending_terminal() {
    let root = fixture_dir("finite-terminal");
    let (model, property, manifest) = write_fixture(
        &root,
        terminal_model_source(),
        "weak-fair-action \"grant-b\"\nstrong-fair-action \"unrelated\"\n",
    );

    let job = run(&job_args(&manifest), None);
    let explicit = run(
        &explicit_args(
            &model,
            &property,
            &[
                "--weak-fair-action",
                "grant-b",
                "--strong-fair-action",
                "unrelated",
            ],
        ),
        None,
    );

    assert_eq!(job.status.code(), Some(7));
    assert!(stdout(&job).contains("counterexample: PENDING_TERMINAL"));
    assert_equivalent(&job, &explicit);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn job_manifest_and_referenced_file_failures_are_exit_two() {
    let root = fixture_dir("fail-closed");
    let malformed = root.join("malformed.fvj");
    fs::write(
        &malformed,
        "model \"model.fvl\"\nproperty \"property.fvt\"\nmax-product-states 1\nmax-product-states 2\n",
    )
    .unwrap();
    let malformed_output = run(&job_args(&malformed), None);
    assert_eq!(malformed_output.status.code(), Some(2));
    assert!(stderr(&malformed_output).contains("duplicate singleton directive 'max-product-states'"));

    let missing = root.join("missing.fvj");
    fs::write(
        &missing,
        "model \"missing/model.fvl\"\nproperty \"missing/property.fvt\"\n",
    )
    .unwrap();
    let missing_output = run(&job_args(&missing), None);
    assert_eq!(missing_output.status.code(), Some(2));
    assert!(stderr(&missing_output).contains("failed to read declarative model"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn adding_job_route_preserves_m54_multi_file_behavior() {
    let root = fixture_dir("compatibility");
    let (model, property, _manifest) = write_fixture(&root, unfair_model_source(), "");
    let output = run(&explicit_args(&model, &property, &[]), None);

    assert_eq!(output.status.code(), Some(7));
    assert!(stdout(&output).contains("violated clause: class-b"));
    assert!(stderr(&output).is_empty());
    let _ = fs::remove_dir_all(root);
}

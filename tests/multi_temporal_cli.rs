use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("CLI stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("CLI stderr should be UTF-8")
}

fn temp_path(kind: &str, extension: &str) -> PathBuf {
    let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "fvlab-m54-{kind}-{}-{id}.{extension}",
        std::process::id()
    ))
}

fn write_temp(kind: &str, extension: &str, content: &str) -> PathBuf {
    let path = temp_path(kind, extension);
    fs::write(&path, content).expect("temporary fixture should be writable");
    path
}

fn property_source() -> &'static str {
    "response(\"class-a\",\"request-a\",\"grant-a\")\nresponse(\"class-b\",\"request-b\",\"grant-b\")\n"
}

fn unfair_model_source() -> &'static str {
    "model \"external-dual-unfair\"\nstate \"idle\"\nstate \"await-a\"\nstate \"ready-b\"\nstate \"await-b\"\ninitial \"idle\"\nedge \"idle\" \"request-a\" \"await-a\"\nedge \"await-a\" \"grant-a\" \"ready-b\"\nedge \"ready-b\" \"request-b\" \"await-b\"\nedge \"await-b\" \"wait-b\" \"await-b\"\nedge \"await-b\" \"grant-b\" \"idle\"\n"
}

fn finite_terminal_model_source() -> &'static str {
    "model \"external-dual-terminal\"\nstate \"idle\"\nstate \"await-a\"\nstate \"ready-b\"\nstate \"await-b\"\ninitial \"idle\"\nedge \"idle\" \"request-a\" \"await-a\"\nedge \"await-a\" \"grant-a\" \"ready-b\"\nedge \"ready-b\" \"request-b\" \"await-b\"\n"
}

fn multi_file_args(model: &PathBuf, property: &PathBuf) -> Vec<String> {
    vec![
        "temporal".to_owned(),
        "multi-file".to_owned(),
        model.to_string_lossy().into_owned(),
        property.to_string_lossy().into_owned(),
    ]
}

fn run_owned(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

#[test]
fn multi_file_reports_exact_clause_for_real_no_fair_violation() {
    let model = write_temp("unfair", "fvl", unfair_model_source());
    let property = write_temp("property", "fvt", property_source());
    let output = run_owned(&multi_file_args(&model, &property));

    assert_eq!(output.status.code(), Some(7));
    let text = stdout(&output);
    assert!(text.contains("multi-response: VIOLATED"));
    assert!(text.contains("violated clause: class-b"));
    assert!(text.contains("counterexample: PENDING_CYCLE"));

    let _ = fs::remove_file(model);
    let _ = fs::remove_file(property);
}

#[test]
fn multi_file_reuses_combined_fairness_and_canonical_reporting() {
    let model = write_temp("fairness", "fvl", unfair_model_source());
    let property = write_temp("property", "fvt", property_source());
    let mut args = multi_file_args(&model, &property);
    args.extend([
        "--weak-fair-action".to_owned(),
        "grant-b".to_owned(),
        "--strong-fair-action".to_owned(),
        "unrelated".to_owned(),
    ]);
    let output = run_owned(&args);

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("multi-response: SATISFIED"));
    assert!(text.contains("weak-fair action: \"grant-b\""));
    assert!(text.contains("strong-fair action: \"unrelated\""));

    let _ = fs::remove_file(model);
    let _ = fs::remove_file(property);
}

#[test]
fn multi_file_fairness_never_excuses_finite_pending_terminal() {
    let model = write_temp("terminal", "fvl", finite_terminal_model_source());
    let property = write_temp("property", "fvt", property_source());
    let mut args = multi_file_args(&model, &property);
    args.extend([
        "--weak-fair-action".to_owned(),
        "grant-b".to_owned(),
        "--strong-fair-action".to_owned(),
        "unrelated".to_owned(),
    ]);
    let output = run_owned(&args);

    assert_eq!(output.status.code(), Some(7));
    let text = stdout(&output);
    assert!(text.contains("violated clause: class-b"));
    assert!(text.contains("counterexample: PENDING_TERMINAL"));

    let _ = fs::remove_file(model);
    let _ = fs::remove_file(property);
}

#[test]
fn multi_file_preserves_product_and_model_cutoff_provenance() {
    let model = write_temp("cutoff", "fvl", unfair_model_source());
    let property = write_temp("property", "fvt", property_source());

    let mut product_args = multi_file_args(&model, &property);
    product_args.extend([
        "--max-product-transitions".to_owned(),
        "3".to_owned(),
    ]);
    let product = run_owned(&product_args);
    assert_eq!(product.status.code(), Some(3));
    assert!(stdout(&product)
        .contains("product inconclusive reason: transition limit reached (max 3)"));

    let mut model_args = multi_file_args(&model, &property);
    model_args.extend([
        "--max-model-transitions".to_owned(),
        "2".to_owned(),
    ]);
    let model_cutoff = run_owned(&model_args);
    assert_eq!(model_cutoff.status.code(), Some(3));
    let model_text = stdout(&model_cutoff);
    assert!(model_text.contains("analysis inconclusive stage: model"));
    assert!(model_text.contains("analysis inconclusive reason: transition limit reached (max 2)"));

    let _ = fs::remove_file(model);
    let _ = fs::remove_file(property);
}

#[test]
fn multi_file_property_parse_and_file_errors_fail_closed() {
    let model = write_temp("malformed", "fvl", unfair_model_source());
    let property = write_temp(
        "malformed-property",
        "fvt",
        "eventually(\"class-b\",\"grant-b\")\n",
    );
    let malformed = run_owned(&multi_file_args(&model, &property));
    assert_eq!(malformed.status.code(), Some(2));
    let malformed_text = stderr(&malformed);
    assert!(malformed_text.contains("multi-response temporal parse error at line 1"));
    assert!(malformed_text.contains("unsupported directive 'eventually'"));

    let missing = run(&[
        "temporal",
        "multi-file",
        "/definitely/missing/model.fvl",
        "/definitely/missing/property.fvt",
    ]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("failed to read declarative model"));

    let _ = fs::remove_file(model);
    let _ = fs::remove_file(property);
}

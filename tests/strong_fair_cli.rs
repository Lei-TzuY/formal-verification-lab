use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fvlab(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be UTF-8")
}

fn declarative_request_grant_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after Unix epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("fvlab-strong-fair-cli-{nonce}.fvl"));
    fs::write(
        &path,
        "model \"request-grant-file\"\n\
         state \"idle\"\n\
         state \"waiting\"\n\
         initial \"idle\"\n\
         edge \"idle\" \"request\" \"waiting\"\n\
         edge \"waiting\" \"wait\" \"waiting\"\n\
         edge \"waiting\" \"grant\" \"idle\"\n",
    )
    .expect("temporary declarative model should be writable");
    path
}

#[test]
fn fixed_temporal_route_reports_explicit_strong_fairness() {
    let baseline = fvlab(&["temporal", "request-grant-unfair"]);
    assert_eq!(baseline.status.code(), Some(10));

    let fair = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(fair.status.success(), "{}", stderr(&fair));
    let report = stdout(&fair);
    assert!(report.contains("temporal: SATISFIED"));
    assert!(report.contains("strong fairness actions: 1"));
    assert!(report.contains("strong-fair action: \"grant\""));
    assert!(!report.contains("weak fairness actions:"));
}

#[test]
fn unrelated_strong_fairness_preserves_real_violation() {
    let output = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(output.status.code(), Some(10));
    let report = stdout(&output);
    assert!(report.contains("temporal: VIOLATED"));
    assert!(report.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn textual_and_declarative_routes_share_strong_fair_backend() {
    let textual = fvlab(&[
        "temporal",
        "check",
        "request-grant-unfair",
        "response(\"request\",\"grant\")",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(textual.status.success(), "{}", stderr(&textual));
    assert!(stdout(&textual).contains("strong-fair action: \"grant\""));

    let path = declarative_request_grant_path();
    let path_string = path.to_string_lossy().into_owned();
    let file = fvlab(&[
        "temporal",
        "file",
        &path_string,
        "response(\"request\",\"grant\")",
        "--strong-fair-action",
        "grant",
    ]);
    let _ = fs::remove_file(path);
    assert!(file.status.success(), "{}", stderr(&file));
    let report = stdout(&file);
    assert!(report.contains("model: request-grant-file"));
    assert!(report.contains("strong-fair action: \"grant\""));
}

#[test]
fn product_and_model_cutoffs_remain_inconclusive_with_provenance() {
    let product = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--strong-fair-action",
        "grant",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_report = stdout(&product);
    assert!(product_report.contains("temporal: INCONCLUSIVE"));
    assert!(product_report.contains("strong-fair action: \"grant\""));
    assert!(
        product_report.contains("product inconclusive reason: transition limit reached (max 2)")
    );

    let model = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--strong-fair-action",
        "grant",
        "--max-model-transitions",
        "1",
    ]);
    assert_eq!(model.status.code(), Some(3));
    let model_report = stdout(&model);
    assert!(model_report.contains("temporal: INCONCLUSIVE"));
    assert!(model_report.contains("analysis inconclusive stage: model"));
    assert!(model_report.contains("analysis inconclusive reason: transition limit reached (max 1)"));
    assert!(model_report.contains("strong-fair action: \"grant\""));
}

#[test]
fn malformed_and_mixed_fairness_fail_closed() {
    let duplicate = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--strong-fair-action",
        "grant",
        "--strong-fair-action",
        "grant",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate strong-fair action 'grant'"));

    let missing = fvlab(&["temporal", "request-grant-unfair", "--strong-fair-action"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("option '--strong-fair-action' requires an action value"));

    let mixed = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "grant",
    ]);
    assert_eq!(mixed.status.code(), Some(2));
    assert!(stderr(&mixed).contains("cannot combine weak and strong fairness assumptions"));

    let monitor = fvlab(&[
        "monitor",
        "session-unfair-close",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(monitor.status.code(), Some(2));
    assert!(stderr(&monitor).contains("strong fairness is not supported by monitor"));
}

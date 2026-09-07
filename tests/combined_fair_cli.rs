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
    let path = std::env::temp_dir().join(format!("fvlab-combined-fair-cli-{nonce}.fvl"));
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
fn fixed_temporal_route_accepts_distinct_mixed_profile() {
    let output = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "unrelated",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let report = stdout(&output);
    assert!(report.contains("temporal: SATISFIED"));
    assert!(report.contains("weak fairness actions: 1"));
    assert!(report.contains("weak-fair action: \"unrelated\""));
    assert!(report.contains("strong fairness actions: 1"));
    assert!(report.contains("strong-fair action: \"grant\""));
}

#[test]
fn overlap_is_canonicalized_to_the_strong_class() {
    let output = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let report = stdout(&output);
    assert!(report.contains("temporal: SATISFIED"));
    assert!(report.contains("weak fairness actions: 0"));
    assert!(!report.contains("weak-fair action: \"grant\""));
    assert!(report.contains("strong fairness actions: 1"));
    assert!(report.contains("strong-fair action: \"grant\""));
}

#[test]
fn unrelated_mixed_profile_preserves_a_real_violation() {
    let output = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "weak-unrelated",
        "--strong-fair-action",
        "strong-unrelated",
    ]);
    assert_eq!(output.status.code(), Some(10));
    let report = stdout(&output);
    assert!(report.contains("temporal: VIOLATED"));
    assert!(report.contains("weak-fair action: \"weak-unrelated\""));
    assert!(report.contains("strong-fair action: \"strong-unrelated\""));
}

#[test]
fn textual_and_declarative_routes_share_mixed_profile_dispatch() {
    let textual = fvlab(&[
        "temporal",
        "check",
        "request-grant-unfair",
        "response(\"request\",\"grant\")",
        "--weak-fair-action",
        "unrelated",
        "--strong-fair-action",
        "grant",
    ]);
    assert!(textual.status.success(), "{}", stderr(&textual));
    let textual_report = stdout(&textual);
    assert!(textual_report.contains("weak-fair action: \"unrelated\""));
    assert!(textual_report.contains("strong-fair action: \"grant\""));

    let path = declarative_request_grant_path();
    let path_string = path.to_string_lossy().into_owned();
    let file = fvlab(&[
        "temporal",
        "file",
        &path_string,
        "response(\"request\",\"grant\")",
        "--weak-fair-action",
        "unrelated",
        "--strong-fair-action",
        "grant",
    ]);
    let _ = fs::remove_file(path);
    assert!(file.status.success(), "{}", stderr(&file));
    let file_report = stdout(&file);
    assert!(file_report.contains("model: request-grant-file"));
    assert!(file_report.contains("weak-fair action: \"unrelated\""));
    assert!(file_report.contains("strong-fair action: \"grant\""));
}

#[test]
fn mixed_profile_preserves_product_and_model_cutoff_provenance() {
    let product = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "unrelated",
        "--strong-fair-action",
        "grant",
        "--max-product-transitions",
        "2",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_report = stdout(&product);
    assert!(product_report.contains("temporal: INCONCLUSIVE"));
    assert!(product_report.contains("product inconclusive reason: transition limit reached (max 2)"));
    assert!(product_report.contains("weak-fair action: \"unrelated\""));
    assert!(product_report.contains("strong-fair action: \"grant\""));

    let model = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "unrelated",
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
    assert!(model_report.contains("weak-fair action: \"unrelated\""));
    assert!(model_report.contains("strong-fair action: \"grant\""));
}

#[test]
fn malformed_fairness_still_fails_closed() {
    let duplicate = fvlab(&[
        "temporal",
        "request-grant-unfair",
        "--weak-fair-action",
        "grant",
        "--weak-fair-action",
        "grant",
        "--strong-fair-action",
        "other",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate weak-fair action 'grant'"));

    let missing = fvlab(&["temporal", "request-grant-unfair", "--strong-fair-action"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("option '--strong-fair-action' requires an action value"));
}

#[test]
fn direct_monitor_mixed_fairness_remains_fail_closed() {
    let output = fvlab(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("cannot combine weak and strong fairness assumptions"));
}

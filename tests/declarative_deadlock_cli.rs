use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m63-deadlock-cli-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn write_model(root: &Path) -> PathBuf {
    let path = root.join("model.fvl");
    fs::write(
        &path,
        "model \"workflow\"\nstate \"run\"\nstate \"done\"\nstate \"declared-cancelled\"\ninitial \"run\"\nedge \"run\" \"finish\" \"done\"\nlabel \"done\" \"done\"\nlabel \"declared-cancelled\" \"cancelled\"\n",
    )
    .unwrap();
    path
}

fn run_cli(path: &Path, expression: &str, options: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fvlab-deadlock"));
    command
        .arg(path)
        .arg(expression)
        .args(options)
        .output()
        .expect("fvlab-deadlock binary should execute")
}

#[test]
fn external_file_cli_reports_deadlock_free_and_native_success_exit() {
    let root = fixture_dir("free");
    let model = write_model(&root);
    let output = run_cli(&model, "\"done\"", &[]);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("model: workflow"));
    assert!(stdout.contains("deadlock policy: \"done\""));
    assert!(stdout.contains("deadlock verification: DEADLOCK_FREE"));
    assert!(stdout.contains("witness: none"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn external_file_cli_preserves_shortest_deadlock_witness_and_exit_five() {
    let root = fixture_dir("found");
    let model = write_model(&root);
    let output = run_cli(&model, "\"cancelled\"", &[]);

    assert_eq!(output.status.code(), Some(5));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("deadlock verification: DEADLOCK_FOUND"));
    assert!(stdout.contains("0: run [initial]"));
    assert!(stdout.contains("1: --finish--> done"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn external_file_cli_reports_model_cutoff_as_inconclusive_without_fake_witness() {
    let root = fixture_dir("cutoff");
    let model = write_model(&root);
    let output = run_cli(&model, "\"done\"", &["--max-transitions", "0"]);

    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("deadlock verification: INCONCLUSIVE"));
    assert!(stdout.contains("inconclusive reason: TransitionLimitReached { limit: 0 }"));
    assert!(stdout.contains("explored transitions: 0"));
    assert!(stdout.contains("witness: none"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn external_file_cli_rejects_unknown_proposition_before_verification() {
    let root = fixture_dir("unknown-proposition");
    let model = write_model(&root);
    let output = run_cli(&model, "\"missing\"", &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("missing"));

    let _ = fs::remove_dir_all(root);
}

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("CLI output should be UTF-8")
}

#[test]
fn finite_monitor_cli_exercises_satisfied_rejecting_and_progress_cycle_paths() {
    let satisfied = run(&["monitor", "session-ok"]);
    assert!(satisfied.status.success());
    let satisfied_text = stdout(&satisfied);
    assert!(satisfied_text.contains("model: session-protocol"));
    assert!(satisfied_text.contains("monitor: ordered-session-lifecycle"));
    assert!(satisfied_text.contains("monitor verification: SATISFIED"));
    assert!(satisfied_text.contains("counterexample: none (monitor conditions hold)"));

    let rejecting = run(&["monitor", "session-double-open"]);
    assert_eq!(rejecting.status.code(), Some(8));
    let rejecting_text = stdout(&rejecting);
    assert!(rejecting_text.contains("model: session-double-open"));
    assert!(rejecting_text.contains("monitor verification: VIOLATED"));
    assert!(rejecting_text.contains("violated condition: legal-action-order"));
    assert!(rejecting_text.contains("counterexample: REJECTING_STATE"));
    assert!(rejecting_text.contains("--open-->"));
    assert!(rejecting_text.contains("monitor: Rejected"));

    let progress = run(&["monitor", "session-stuck"]);
    assert_eq!(progress.status.code(), Some(8));
    let progress_text = stdout(&progress);
    assert!(progress_text.contains("model: session-stuck-committed"));
    assert!(progress_text.contains("monitor verification: VIOLATED"));
    assert!(progress_text.contains("violated condition: opened-session-eventually-closes"));
    assert!(progress_text.contains("counterexample: PROGRESS_CYCLE"));
    assert!(progress_text.contains("--commit-->"));
    assert!(progress_text.contains("--tick-->"));
    assert!(progress_text.contains("monitor: Committed"));
}

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
fn buchi_cli_exercises_acceptance_lasso_and_finite_policy_paths() {
    let satisfied = run(&["buchi", "pulses"]);
    assert!(satisfied.status.success());
    let satisfied_text = stdout(&satisfied);
    assert!(satisfied_text.contains("model: alternating-pulses"));
    assert!(satisfied_text.contains("Buchi automaton: infinitely-often-a-and-b"));
    assert!(satisfied_text.contains("Buchi verification: SATISFIED"));
    assert!(satisfied_text.contains("acceptance sets: 2"));

    let unfair = run(&["buchi", "pulses-unfair"]);
    assert_eq!(unfair.status.code(), Some(9));
    let unfair_text = stdout(&unfair);
    assert!(unfair_text.contains("model: unfair-second-pulse"));
    assert!(unfair_text.contains("Buchi verification: VIOLATED"));
    assert!(unfair_text.contains("avoided acceptance set: pulse-b-observed"));
    assert!(unfair_text.contains("counterexample: ACCEPTANCE_AVOIDING_CYCLE"));
    assert!(unfair_text.contains("--pulse-a-->"));

    let ignored = run(&["buchi", "finite-ignore"]);
    assert!(ignored.status.success());
    let ignored_text = stdout(&ignored);
    assert!(ignored_text.contains("model: finite-quiet-run"));
    assert!(ignored_text.contains("finite policy: IGNORE_TERMINALS"));
    assert!(ignored_text.contains("Buchi verification: SATISFIED"));

    let strict = run(&["buchi", "finite-strict"]);
    assert_eq!(strict.status.code(), Some(9));
    let strict_text = stdout(&strict);
    assert!(strict_text.contains("finite policy: REQUIRE_ACCEPTING_TERMINAL"));
    assert!(strict_text.contains("Buchi verification: VIOLATED"));
    assert!(strict_text.contains("missing acceptance set: pulse-a-observed"));
    assert!(strict_text.contains("counterexample: FINITE_TERMINAL"));
    assert!(strict_text.contains("--quiet-->"));
}

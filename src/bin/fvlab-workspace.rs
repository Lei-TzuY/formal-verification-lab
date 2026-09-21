use formal_verification_lab::{
    audit_workspace_replay_determinism_text, create_workspace_replay_lock_from_text,
    create_workspace_snapshot, parse_workspace_snapshot, render_workspace_replay_lock,
    render_workspace_snapshot, replay_workspace_snapshot_expectations_json,
    replay_workspace_snapshot_json, verify_workspace_replay_lock_text,
    RootedFileSystemTextSourceProvider, WorkspaceReplayMode,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, root, source_id, snapshot_path] if command == "create" => {
            create_snapshot(root, source_id, snapshot_path)
        }
        [command, snapshot_path, flag, format]
            if command == "replay" && flag == "--format" && format == "json" =>
        {
            replay_snapshot(snapshot_path, false)
        }
        [command, snapshot_path, check, flag, format]
            if command == "replay"
                && check == "--check-expectations"
                && flag == "--format"
                && format == "json" =>
        {
            replay_snapshot(snapshot_path, true)
        }
        [command, snapshot_path, mode, lock_path] if command == "lock-create" => {
            create_lock(snapshot_path, mode, lock_path)
        }
        [command, lock_path, flag, format]
            if command == "lock-verify" && flag == "--format" && format == "json" =>
        {
            verify_lock(lock_path)
        }
        [command, snapshot_path, mode, additional_attempts, flag, format]
            if command == "determinism-check" && flag == "--format" && format == "json" =>
        {
            audit_determinism(snapshot_path, mode, additional_attempts)
        }
        [command, _, flag, format]
            if (command == "replay" || command == "lock-verify") && flag == "--format" =>
        {
            eprintln!("error: unsupported workspace format '{format}'; expected json");
            ExitCode::from(2)
        }
        [command, _, _, _, flag, format]
            if command == "determinism-check" && flag == "--format" =>
        {
            eprintln!("error: unsupported workspace format '{format}'; expected json");
            ExitCode::from(2)
        }
        _ => {
            eprintln!(
                "usage: fvlab-workspace create <workspace-root> <orchestration-source-id> <snapshot-path>\n       fvlab-workspace replay <snapshot-path> [--check-expectations] --format json\n       fvlab-workspace lock-create <snapshot-path> <raw|expectations> <lock-path>\n       fvlab-workspace lock-verify <lock-path> --format json\n       fvlab-workspace determinism-check <snapshot-path> <raw|expectations> <additional-attempts> --format json"
            );
            ExitCode::from(2)
        }
    }
}

fn create_snapshot(root: &str, source_id: &str, snapshot_path: &str) -> ExitCode {
    let provider = RootedFileSystemTextSourceProvider::new(root);
    let snapshot = match create_workspace_snapshot(&provider, source_id) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let rendered = render_workspace_snapshot(&snapshot);
    if let Err(error) = fs::write(snapshot_path, rendered) {
        eprintln!("error: failed to write workspace snapshot '{snapshot_path}': {error}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn replay_snapshot(snapshot_path: &str, check_expectations: bool) -> ExitCode {
    let input = match fs::read_to_string(snapshot_path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: failed to read workspace snapshot '{snapshot_path}': {error}");
            return ExitCode::from(2);
        }
    };
    let snapshot = match parse_workspace_snapshot(&input) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };

    if check_expectations {
        let run = replay_workspace_snapshot_expectations_json(&snapshot);
        println!("{}", run.to_json());
        ExitCode::from(run.exit_code)
    } else {
        let run = replay_workspace_snapshot_json(&snapshot);
        println!("{}", run.to_json());
        ExitCode::from(run.exit_code)
    }
}

fn create_lock(snapshot_path: &str, mode: &str, lock_path: &str) -> ExitCode {
    let snapshot_text = match fs::read_to_string(snapshot_path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: failed to read workspace snapshot '{snapshot_path}': {error}");
            return ExitCode::from(2);
        }
    };
    let mode = match mode {
        "raw" => WorkspaceReplayMode::Raw,
        "expectations" => WorkspaceReplayMode::Expectations,
        other => {
            eprintln!("error: unsupported workspace replay lock mode '{other}'; expected raw or expectations");
            return ExitCode::from(2);
        }
    };
    let lock = match create_workspace_replay_lock_from_text(&snapshot_text, mode) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = fs::write(lock_path, render_workspace_replay_lock(&lock)) {
        eprintln!("error: failed to write workspace replay lock '{lock_path}': {error}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn verify_lock(lock_path: &str) -> ExitCode {
    let input = match fs::read_to_string(lock_path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: failed to read workspace replay lock '{lock_path}': {error}");
            return ExitCode::from(2);
        }
    };
    let run = verify_workspace_replay_lock_text(&input);
    println!("{}", run.to_json());
    ExitCode::from(run.exit_code)
}

fn audit_determinism(snapshot_path: &str, mode: &str, additional_attempts: &str) -> ExitCode {
    let input = match fs::read_to_string(snapshot_path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: failed to read workspace snapshot '{snapshot_path}': {error}");
            return ExitCode::from(2);
        }
    };
    let mode = match mode {
        "raw" => WorkspaceReplayMode::Raw,
        "expectations" => WorkspaceReplayMode::Expectations,
        other => {
            eprintln!(
                "error: unsupported workspace replay determinism mode '{other}'; expected raw or expectations"
            );
            return ExitCode::from(2);
        }
    };
    let additional_attempts = match additional_attempts.parse::<usize>() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("error: additional attempts must be a non-negative decimal integer");
            return ExitCode::from(2);
        }
    };

    let run = audit_workspace_replay_determinism_text(&input, mode, additional_attempts);
    println!("{}", run.to_json());
    ExitCode::from(run.exit_code)
}

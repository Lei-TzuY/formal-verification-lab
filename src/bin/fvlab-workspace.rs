use formal_verification_lab::{
    create_workspace_snapshot, parse_workspace_snapshot, render_workspace_snapshot,
    replay_workspace_snapshot_expectations_json, replay_workspace_snapshot_json,
    RootedFileSystemTextSourceProvider,
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
        [command, _, flag, format] if command == "replay" && flag == "--format" => {
            eprintln!("error: unsupported workspace replay format '{format}'; expected json");
            ExitCode::from(2)
        }
        _ => {
            eprintln!(
                "usage: fvlab-workspace create <workspace-root> <orchestration-source-id> <snapshot-path>\n       fvlab-workspace replay <snapshot-path> [--check-expectations] --format json"
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

use formal_verification_lab::structural_suite_run::{
    run_structural_suite_expectations_json, run_structural_suite_json,
};
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [manifest_path, flag, format] if flag == "--format" && format == "json" => {
            let run = run_structural_suite_json(manifest_path);
            println!("{}", run.to_json());
            ExitCode::from(run.exit_code)
        }
        [manifest_path, check, flag, format]
            if check == "--check-expectations" && flag == "--format" && format == "json" =>
        {
            let run = run_structural_suite_expectations_json(manifest_path);
            println!("{}", run.to_json());
            ExitCode::from(run.exit_code)
        }
        [_, flag, format] if flag == "--format" => {
            eprintln!("error: unsupported structural suite format '{format}'; expected json");
            ExitCode::from(2)
        }
        _ => {
            eprintln!(
                "usage: fvlab-structural-suite <suite-manifest> [--check-expectations] --format json"
            );
            ExitCode::from(2)
        }
    }
}

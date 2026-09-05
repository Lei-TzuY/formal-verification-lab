use formal_verification_lab::checker::{check_with_limits, ExplorationLimits, VerificationStatus};
use formal_verification_lab::examples::{bounded_counter, buggy_mutex, traffic_light};
use formal_verification_lab::report::render_report;
use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    match args.as_slice() {
        [] => {
            print_examples();
            Ok(ExitCode::SUCCESS)
        }
        [command] if command == "list" => {
            print_examples();
            Ok(ExitCode::SUCCESS)
        }
        [command, rest @ ..] if command == "run" => run_command(rest),
        _ => Err(usage()),
    }
}

fn run_command(args: &[String]) -> Result<ExitCode, String> {
    let (example, option_args) = args.split_first().ok_or_else(usage)?;
    let limits = parse_limits(option_args)?;

    match example.as_str() {
        "counter" => run_model(
            bounded_counter().map_err(|error| error.to_string())?,
            limits,
        ),
        "mutex-bug" => run_model(buggy_mutex().map_err(|error| error.to_string())?, limits),
        "traffic-light" => run_model(traffic_light().map_err(|error| error.to_string())?, limits),
        _ => Err(format!(
            "unknown example '{example}'; expected counter, mutex-bug, or traffic-light"
        )),
    }
}

fn parse_limits(args: &[String]) -> Result<ExplorationLimits, String> {
    let mut limits = ExplorationLimits::unbounded();
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("option '{flag}' requires an integer value"))?;
        let parsed = value
            .parse::<usize>()
            .map_err(|_| format!("option '{flag}' requires a non-negative integer"))?;

        match flag {
            "--max-states" => set_limit(&mut limits.max_states, parsed, flag)?,
            "--max-transitions" => set_limit(&mut limits.max_transitions, parsed, flag)?,
            "--max-depth" => set_limit(&mut limits.max_depth, parsed, flag)?,
            _ => return Err(format!("unknown option '{flag}'\n{}", usage())),
        }

        index += 2;
    }

    Ok(limits)
}

fn set_limit(slot: &mut Option<usize>, value: usize, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate option '{flag}'"))
    } else {
        Ok(())
    }
}

fn run_model<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    limits: ExplorationLimits,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_with_limits(&model, limits).map_err(|error| error.to_string())?;
    print!("{}", render_report(model.name(), &result));

    Ok(match result.status {
        VerificationStatus::Safe => ExitCode::SUCCESS,
        VerificationStatus::Violated => ExitCode::from(1),
        VerificationStatus::Inconclusive => ExitCode::from(3),
    })
}

fn usage() -> String {
    "usage: fvlab [list | run <counter|mutex-bug|traffic-light> [--max-states N] [--max-transitions N] [--max-depth N]]"
        .to_owned()
}

fn print_examples() {
    println!("counter\nmutex-bug\ntraffic-light");
}

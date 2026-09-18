use formal_verification_lab::{
    check_declarative_deadlock_with_limits, parse_declarative_deadlock_spec,
    parse_declarative_document, render_declarative_deadlock_report, BoundedOutcome, DeadlockStatus,
    ExplorationLimits,
};
use std::env;
use std::fs;
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
    let [path, expression, option_args @ ..] = args.as_slice() else {
        return Err(usage());
    };

    let input = fs::read_to_string(path)
        .map_err(|error| format!("failed to read declarative model '{path}': {error}"))?;
    let document = parse_declarative_document(&input).map_err(|error| error.to_string())?;
    let spec = parse_declarative_deadlock_spec("cli-deadlock", expression)
        .map_err(|error| error.to_string())?;
    let limits = parse_limits(option_args)?;
    let result = check_declarative_deadlock_with_limits(&document, &spec, limits)
        .map_err(|error| error.to_string())?;

    print!(
        "{}",
        render_declarative_deadlock_report(document.model().name(), &result)
    );

    Ok(match result.outcome {
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFree) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFound) => ExitCode::from(5),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
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
        return Err(format!("duplicate option '{flag}'"));
    }
    Ok(())
}

fn usage() -> String {
    "usage: fvlab-deadlock <model-path> <legitimate-terminal-expression> [--max-states N] [--max-transitions N] [--max-depth N]".to_owned()
}

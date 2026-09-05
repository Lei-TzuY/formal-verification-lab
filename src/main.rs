use formal_verification_lab::checker::check;
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
        [command, example] if command == "run" => match example.as_str() {
            "counter" => run_model(bounded_counter().map_err(|error| error.to_string())?),
            "mutex-bug" => run_model(buggy_mutex().map_err(|error| error.to_string())?),
            "traffic-light" => run_model(traffic_light().map_err(|error| error.to_string())?),
            _ => Err(format!(
                "unknown example '{example}'; expected counter, mutex-bug, or traffic-light"
            )),
        },
        _ => Err("usage: fvlab [list | run <counter|mutex-bug|traffic-light>]".to_owned()),
    }
}

fn run_model<S>(model: formal_verification_lab::TransitionSystem<S>) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check(&model).map_err(|error| error.to_string())?;
    print!("{}", render_report(model.name(), &result));

    if result.counterexample.is_some() {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn print_examples() {
    println!("counter\nmutex-bug\ntraffic-light");
}

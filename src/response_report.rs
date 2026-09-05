use crate::response::{ResponseCounterexample, ResponseResult, ResponseStatus};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for action response analysis.
pub fn render_response_report<S: Debug>(model_name: &str, result: &ResponseResult<S>) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "response: {}",
        match result.status {
            ResponseStatus::Satisfied => "SATISFIED",
            ResponseStatus::Violated => "VIOLATED",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "model states: {}", result.model_states)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "model transitions: {}",
        result.model_transitions
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "product states: {}", result.product_states)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "product transitions: {}",
        result.product_transitions
    )
    .expect("writing to String cannot fail");

    match &result.counterexample {
        None => writeln!(
            &mut output,
            "counterexample: none (every trigger is eventually answered)"
        )
        .expect("writing to String cannot fail"),
        Some(ResponseCounterexample::Finite { trace }) => {
            writeln!(&mut output, "counterexample: PENDING_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(ResponseCounterexample::Infinite { stem, cycle }) => {
            writeln!(&mut output, "counterexample: PENDING_CYCLE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "stem:").expect("writing to String cannot fail");
            render_trace(&mut output, stem, "initial");
            writeln!(&mut output, "cycle:").expect("writing to String cannot fail");
            render_trace(&mut output, cycle, "cycle-entry");
        }
    }

    output
}

fn render_trace<S: Debug>(output: &mut String, trace: &[crate::checker::TraceStep<S>], root: &str) {
    for (index, step) in trace.iter().enumerate() {
        match &step.action {
            None => writeln!(output, "  {index}: {:?} [{root}]", step.state),
            Some(action) => writeln!(output, "  {index}: --{action}--> {:?}", step.state),
        }
        .expect("writing to String cannot fail");
    }
}

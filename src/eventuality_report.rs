use crate::eventuality::{EventualityCounterexample, EventualityResult, EventualityStatus};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for universal eventuality analysis.
pub fn render_eventuality_report<S: Debug>(
    model_name: &str,
    result: &EventualityResult<S>,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "eventuality: {}",
        match result.status {
            EventualityStatus::Satisfied => "SATISFIED",
            EventualityStatus::Violated => "VIOLATED",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "discovered states: {}",
        result.discovered_states
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "explored transitions: {}",
        result.explored_transitions
    )
    .expect("writing to String cannot fail");
    match result.max_depth_reached {
        Some(depth) => writeln!(&mut output, "max depth reached: {depth}"),
        None => writeln!(&mut output, "max depth reached: none"),
    }
    .expect("writing to String cannot fail");

    match &result.counterexample {
        None => writeln!(
            &mut output,
            "counterexample: none (all maximal executions reach target)"
        )
        .expect("writing to String cannot fail"),
        Some(EventualityCounterexample::Finite { trace }) => {
            writeln!(&mut output, "counterexample: FINITE_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(EventualityCounterexample::Infinite { stem, cycle }) => {
            writeln!(&mut output, "counterexample: INFINITE_CYCLE")
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

use crate::checker::TraceStep;
use crate::safety::{SafetyResult, SafetyStatus};
use std::fmt::Write;

pub fn render_safety_report(model_name: &str, result: &SafetyResult) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(&mut output, "assertion: always {}", result.expression)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "safety: {}",
        match result.status {
            SafetyStatus::Safe => "SAFE",
            SafetyStatus::Violated => "VIOLATED",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "discovered states: {}", result.discovered_states)
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
        None => writeln!(&mut output, "evidence: none").expect("writing to String cannot fail"),
        Some(trace) => {
            writeln!(&mut output, "evidence: SAFETY_COUNTEREXAMPLE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace);
        }
    }

    output
}

fn render_trace(output: &mut String, trace: &[TraceStep<String>]) {
    for (index, step) in trace.iter().enumerate() {
        match &step.action {
            None => writeln!(output, "  {index}: {:?} [initial]", step.state),
            Some(action) => writeln!(output, "  {index}: --{action}--> {:?}", step.state),
        }
        .expect("writing to String cannot fail");
    }
}

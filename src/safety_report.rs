use crate::bounded::BoundedOutcome;
use crate::bounded_report::format_inconclusive_reason;
use crate::checker::TraceStep;
use crate::safety::{BoundedSafetyResult, SafetyResult, SafetyStatus};
use std::fmt::Write;

pub fn render_safety_report(model_name: &str, result: &SafetyResult) -> String {
    let mut output = String::new();
    render_header(
        &mut output,
        model_name,
        &result.property,
        &result.expression,
    );
    writeln!(&mut output, "safety: {}", status_label(result.status))
        .expect("writing to String cannot fail");
    render_accounting(
        &mut output,
        result.discovered_states,
        None,
        result.explored_transitions,
        result.max_depth_reached,
    );
    render_counterexample(&mut output, result.counterexample.as_deref());
    output
}

pub fn render_bounded_safety_report(model_name: &str, result: &BoundedSafetyResult) -> String {
    let mut output = String::new();
    render_header(
        &mut output,
        model_name,
        &result.property,
        &result.expression,
    );
    match result.outcome {
        BoundedOutcome::Conclusive(status) => {
            writeln!(&mut output, "safety: {}", status_label(status))
                .expect("writing to String cannot fail");
        }
        BoundedOutcome::Inconclusive(reason) => {
            writeln!(&mut output, "safety: INCONCLUSIVE").expect("writing to String cannot fail");
            writeln!(
                &mut output,
                "inconclusive reason: {}",
                format_inconclusive_reason(reason)
            )
            .expect("writing to String cannot fail");
        }
    }
    render_accounting(
        &mut output,
        result.discovered_states,
        Some(result.checked_states),
        result.explored_transitions,
        result.max_depth_reached,
    );
    render_counterexample(&mut output, result.counterexample.as_deref());
    output
}

fn render_header(output: &mut String, model_name: &str, property: &str, expression: &str) {
    writeln!(output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(output, "property: {property}").expect("writing to String cannot fail");
    writeln!(output, "assertion: always {expression}").expect("writing to String cannot fail");
}

fn status_label(status: SafetyStatus) -> &'static str {
    match status {
        SafetyStatus::Safe => "SAFE",
        SafetyStatus::Violated => "VIOLATED",
    }
}

fn render_accounting(
    output: &mut String,
    discovered_states: usize,
    checked_states: Option<usize>,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
) {
    writeln!(output, "discovered states: {discovered_states}")
        .expect("writing to String cannot fail");
    if let Some(checked_states) = checked_states {
        writeln!(output, "checked states: {checked_states}")
            .expect("writing to String cannot fail");
    }
    writeln!(output, "explored transitions: {explored_transitions}")
        .expect("writing to String cannot fail");
    match max_depth_reached {
        Some(depth) => writeln!(output, "max depth reached: {depth}"),
        None => writeln!(output, "max depth reached: none"),
    }
    .expect("writing to String cannot fail");
}

fn render_counterexample(output: &mut String, trace: Option<&[TraceStep<String>]>) {
    match trace {
        None => writeln!(output, "evidence: none").expect("writing to String cannot fail"),
        Some(trace) => {
            writeln!(output, "evidence: SAFETY_COUNTEREXAMPLE")
                .expect("writing to String cannot fail");
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace);
        }
    }
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

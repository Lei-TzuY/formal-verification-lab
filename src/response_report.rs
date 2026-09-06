use crate::bounded::BoundedOutcome;
use crate::bounded_report::format_inconclusive_reason;
use crate::response::{
    BoundedResponseResult, ResponseCounterexample, ResponseResult, ResponseStatus,
};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for action response analysis.
pub fn render_response_report<S: Debug>(model_name: &str, result: &ResponseResult<S>) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.property);
    writeln!(&mut output, "response: {}", status_label(result.status))
        .expect("writing to String cannot fail");
    render_unbounded_accounting(&mut output, result);
    render_counterexample(&mut output, result.counterexample.as_ref(), false);
    output
}

/// Render a response analysis whose resource limits apply only to the shared
/// action-product construction after complete model capture.
pub fn render_bounded_response_report<S: Debug>(
    model_name: &str,
    result: &BoundedResponseResult<S>,
) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.property);
    match result.outcome {
        BoundedOutcome::Conclusive(status) => {
            writeln!(&mut output, "response: {}", status_label(status))
                .expect("writing to String cannot fail");
        }
        BoundedOutcome::Inconclusive(reason) => {
            writeln!(&mut output, "response: INCONCLUSIVE")
                .expect("writing to String cannot fail");
            writeln!(
                &mut output,
                "product inconclusive reason: {}",
                format_inconclusive_reason(reason)
            )
            .expect("writing to String cannot fail");
        }
    }
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
        "checked product states: {}",
        result.checked_product_states
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "explored product transitions: {}",
        result.explored_product_transitions
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "retained product transitions: {}",
        result.retained_product_transitions
    )
    .expect("writing to String cannot fail");
    match result.max_product_depth_reached {
        Some(depth) => writeln!(&mut output, "max product depth reached: {depth}"),
        None => writeln!(&mut output, "max product depth reached: none"),
    }
    .expect("writing to String cannot fail");

    let incomplete = matches!(result.outcome, BoundedOutcome::Inconclusive(_));
    render_counterexample(&mut output, result.counterexample.as_ref(), incomplete);
    output
}

fn render_header(output: &mut String, model_name: &str, property: &str) {
    writeln!(output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(output, "property: {property}").expect("writing to String cannot fail");
}

fn status_label(status: ResponseStatus) -> &'static str {
    match status {
        ResponseStatus::Satisfied => "SATISFIED",
        ResponseStatus::Violated => "VIOLATED",
    }
}

fn render_unbounded_accounting<S: Debug>(output: &mut String, result: &ResponseResult<S>) {
    writeln!(output, "model states: {}", result.model_states)
        .expect("writing to String cannot fail");
    writeln!(output, "model transitions: {}", result.model_transitions)
        .expect("writing to String cannot fail");
    writeln!(output, "product states: {}", result.product_states)
        .expect("writing to String cannot fail");
    writeln!(output, "product transitions: {}", result.product_transitions)
        .expect("writing to String cannot fail");
}

fn render_counterexample<S: Debug>(
    output: &mut String,
    counterexample: Option<&ResponseCounterexample<S>>,
    incomplete: bool,
) {
    match counterexample {
        None if incomplete => writeln!(
            output,
            "counterexample: none (product exploration incomplete)"
        )
        .expect("writing to String cannot fail"),
        None => writeln!(
            output,
            "counterexample: none (every trigger is eventually answered)"
        )
        .expect("writing to String cannot fail"),
        Some(ResponseCounterexample::Finite { trace }) => {
            writeln!(output, "counterexample: PENDING_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace, "initial");
        }
        Some(ResponseCounterexample::Infinite { stem, cycle }) => {
            writeln!(output, "counterexample: PENDING_CYCLE")
                .expect("writing to String cannot fail");
            writeln!(output, "stem:").expect("writing to String cannot fail");
            render_trace(output, stem, "initial");
            writeln!(output, "cycle:").expect("writing to String cannot fail");
            render_trace(output, cycle, "cycle-entry");
        }
    }
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

use crate::bounded::BoundedOutcome;
use crate::bounded_report::format_inconclusive_reason;
use crate::checker::TraceStep;
use crate::monitor::{
    BoundedMonitorResult, MonitorCounterexample, MonitorProductState, MonitorResult, MonitorStatus,
};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for deterministic finite-monitor analysis.
pub fn render_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &MonitorResult<S, M>,
) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.monitor);
    writeln!(
        &mut output,
        "monitor verification: {}",
        status_label(result.status)
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
    render_counterexample(&mut output, result.counterexample.as_ref(), false);
    output
}

/// Render finite-monitor verification whose limits apply only to shared action-
/// product construction after complete model capture.
pub fn render_bounded_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &BoundedMonitorResult<S, M>,
) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.monitor);
    match result.outcome {
        BoundedOutcome::Conclusive(status) => {
            writeln!(
                &mut output,
                "monitor verification: {}",
                status_label(status)
            )
            .expect("writing to String cannot fail");
        }
        BoundedOutcome::Inconclusive(reason) => {
            writeln!(&mut output, "monitor verification: INCONCLUSIVE")
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

fn render_header(output: &mut String, model_name: &str, monitor: &str) {
    writeln!(output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(output, "monitor: {monitor}").expect("writing to String cannot fail");
}

fn status_label(status: MonitorStatus) -> &'static str {
    match status {
        MonitorStatus::Satisfied => "SATISFIED",
        MonitorStatus::Violated => "VIOLATED",
    }
}

fn render_counterexample<S: Debug, M: Debug>(
    output: &mut String,
    counterexample: Option<&MonitorCounterexample<S, M>>,
    incomplete: bool,
) {
    match counterexample {
        None if incomplete => writeln!(
            output,
            "counterexample: none (product exploration incomplete)"
        )
        .expect("writing to String cannot fail"),
        None => writeln!(output, "counterexample: none (monitor conditions hold)")
            .expect("writing to String cannot fail"),
        Some(MonitorCounterexample::Rejecting { condition, trace }) => {
            writeln!(output, "violated condition: {condition}")
                .expect("writing to String cannot fail");
            writeln!(output, "counterexample: REJECTING_STATE")
                .expect("writing to String cannot fail");
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace, "initial");
        }
        Some(MonitorCounterexample::ProgressTerminal { condition, trace }) => {
            writeln!(output, "violated condition: {condition}")
                .expect("writing to String cannot fail");
            writeln!(output, "counterexample: PROGRESS_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace, "initial");
        }
        Some(MonitorCounterexample::ProgressCycle {
            condition,
            stem,
            cycle,
        }) => {
            writeln!(output, "violated condition: {condition}")
                .expect("writing to String cannot fail");
            writeln!(output, "counterexample: PROGRESS_CYCLE")
                .expect("writing to String cannot fail");
            writeln!(output, "stem:").expect("writing to String cannot fail");
            render_trace(output, stem, "initial");
            writeln!(output, "cycle:").expect("writing to String cannot fail");
            render_trace(output, cycle, "cycle-entry");
        }
    }
}

fn render_trace<S: Debug, M: Debug>(
    output: &mut String,
    trace: &[TraceStep<MonitorProductState<S, M>>],
    root: &str,
) {
    for (index, step) in trace.iter().enumerate() {
        match &step.action {
            None => writeln!(output, "  {index}: {:?} [{root}]", step.state),
            Some(action) => writeln!(output, "  {index}: --{action}--> {:?}", step.state),
        }
        .expect("writing to String cannot fail");
    }
}

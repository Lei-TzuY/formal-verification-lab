use crate::checker::TraceStep;
use crate::monitor::{MonitorCounterexample, MonitorProductState, MonitorResult, MonitorStatus};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for deterministic finite-monitor analysis.
pub fn render_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &MonitorResult<S, M>,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "monitor: {}", result.monitor).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "monitor verification: {}",
        match result.status {
            MonitorStatus::Satisfied => "SATISFIED",
            MonitorStatus::Violated => "VIOLATED",
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
        None => writeln!(&mut output, "counterexample: none (monitor conditions hold)")
            .expect("writing to String cannot fail"),
        Some(MonitorCounterexample::Rejecting { condition, trace }) => {
            writeln!(&mut output, "violated condition: {condition}")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "counterexample: REJECTING_STATE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(MonitorCounterexample::ProgressTerminal { condition, trace }) => {
            writeln!(&mut output, "violated condition: {condition}")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "counterexample: PROGRESS_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(MonitorCounterexample::ProgressCycle {
            condition,
            stem,
            cycle,
        }) => {
            writeln!(&mut output, "violated condition: {condition}")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "counterexample: PROGRESS_CYCLE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "stem:").expect("writing to String cannot fail");
            render_trace(&mut output, stem, "initial");
            writeln!(&mut output, "cycle:").expect("writing to String cannot fail");
            render_trace(&mut output, cycle, "cycle-entry");
        }
    }

    output
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

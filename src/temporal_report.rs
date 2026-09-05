use crate::temporal::{
    TemporalBackend, TemporalCounterexample, TemporalObligation, TemporalResult, TemporalStatus,
};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for the typed action-temporal frontend.
pub fn render_temporal_report<S: Debug>(model_name: &str, result: &TemporalResult<S>) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "backend: {}",
        match result.backend {
            TemporalBackend::Response => "RESPONSE",
            TemporalBackend::Buchi => "BUCHI",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "temporal: {}",
        match result.status {
            TemporalStatus::Satisfied => "SATISFIED",
            TemporalStatus::Violated => "VIOLATED",
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
        None => {
            writeln!(&mut output, "counterexample: none").expect("writing to String cannot fail")
        }
        Some(TemporalCounterexample::Finite { obligation, trace }) => {
            writeln!(&mut output, "counterexample: FINITE").expect("writing to String cannot fail");
            render_obligation(&mut output, obligation);
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(TemporalCounterexample::Infinite {
            obligation,
            stem,
            cycle,
        }) => {
            writeln!(&mut output, "counterexample: INFINITE")
                .expect("writing to String cannot fail");
            render_obligation(&mut output, obligation);
            writeln!(&mut output, "stem:").expect("writing to String cannot fail");
            render_trace(&mut output, stem, "initial");
            writeln!(&mut output, "cycle:").expect("writing to String cannot fail");
            render_trace(&mut output, cycle, "cycle-entry");
        }
    }

    output
}

fn render_obligation(output: &mut String, obligation: &TemporalObligation) {
    match obligation {
        TemporalObligation::Response => writeln!(output, "obligation: response"),
        TemporalObligation::InfinitelyOftenAction(action) => {
            writeln!(output, "obligation: infinitely-often action '{action}'")
        }
    }
    .expect("writing to String cannot fail");
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

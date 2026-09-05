use crate::checker::TraceStep;
use crate::multi_response::{
    MultiObligationState, MultiResponseCounterexample, MultiResponseResult, MultiResponseStatus,
};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for multi-class response analysis.
pub fn render_multi_response_report<S: Debug>(
    model_name: &str,
    result: &MultiResponseResult<S>,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "multi-response: {}",
        match result.status {
            MultiResponseStatus::Satisfied => "SATISFIED",
            MultiResponseStatus::Violated => "VIOLATED",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "clauses: {}", result.clause_count)
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
            "counterexample: none (every clause is eventually answered)"
        )
        .expect("writing to String cannot fail"),
        Some(MultiResponseCounterexample::Finite { clause, trace }) => {
            writeln!(&mut output, "violated clause: {clause}")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "counterexample: PENDING_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(MultiResponseCounterexample::Infinite {
            clause,
            stem,
            cycle,
        }) => {
            writeln!(&mut output, "violated clause: {clause}")
                .expect("writing to String cannot fail");
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

fn render_trace<S: Debug>(
    output: &mut String,
    trace: &[TraceStep<MultiObligationState<S>>],
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

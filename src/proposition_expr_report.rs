use crate::checker::TraceStep;
use crate::exact_state::{ExactStateBackend, ExactStateEvidence, ExactStateStatus};
use crate::proposition_expr::PropositionExpressionResult;
use std::fmt::Write;

pub fn render_proposition_expression_report(
    model_name: &str,
    result: &PropositionExpressionResult,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(&mut output, "expression: {}", result.expression)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "backend: {}",
        match result.backend {
            ExactStateBackend::Reachability => "REACHABILITY",
            ExactStateBackend::Eventuality => "EVENTUALITY",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "state property: {}",
        match result.status {
            ExactStateStatus::Satisfied => "SATISFIED",
            ExactStateStatus::Violated => "VIOLATED",
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

    match &result.evidence {
        None => writeln!(&mut output, "evidence: none").expect("writing to String cannot fail"),
        Some(ExactStateEvidence::ReachabilityWitness { trace }) => {
            writeln!(&mut output, "evidence: REACHABILITY_WITNESS")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(ExactStateEvidence::EventualityFiniteCounterexample { trace }) => {
            writeln!(&mut output, "evidence: EVENTUALITY_FINITE_COUNTEREXAMPLE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle }) => {
            writeln!(&mut output, "evidence: EVENTUALITY_INFINITE_COUNTEREXAMPLE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "stem:").expect("writing to String cannot fail");
            render_trace(&mut output, stem, "initial");
            writeln!(&mut output, "cycle:").expect("writing to String cannot fail");
            render_trace(&mut output, cycle, "cycle-entry");
        }
    }

    output
}

fn render_trace(output: &mut String, trace: &[TraceStep<String>], root: &str) {
    for (index, step) in trace.iter().enumerate() {
        match &step.action {
            None => writeln!(output, "  {index}: {:?} [{root}]", step.state),
            Some(action) => writeln!(output, "  {index}: --{action}--> {:?}", step.state),
        }
        .expect("writing to String cannot fail");
    }
}

use crate::bounded::BoundedOutcome;
use crate::bounded_report::format_inconclusive_reason;
use crate::checker::TraceStep;
use crate::exact_state::{ExactStateBackend, ExactStateEvidence, ExactStateStatus};
use crate::proposition::{BoundedPropositionResult, PropositionResult};
use std::fmt::Write;

pub fn render_proposition_report(model_name: &str, result: &PropositionResult) -> String {
    let mut output = String::new();
    render_header(
        &mut output,
        model_name,
        &result.property,
        &result.proposition,
        result.backend,
    );
    writeln!(
        &mut output,
        "state property: {}",
        status_label(result.status)
    )
    .expect("writing to String cannot fail");
    render_accounting(
        &mut output,
        result.discovered_states,
        None,
        result.explored_transitions,
        result.max_depth_reached,
    );
    render_evidence(&mut output, result.evidence.as_ref());
    output
}

pub fn render_bounded_proposition_report(
    model_name: &str,
    result: &BoundedPropositionResult,
) -> String {
    let mut output = String::new();
    render_header(
        &mut output,
        model_name,
        &result.property,
        &result.proposition,
        result.backend,
    );
    match result.outcome {
        BoundedOutcome::Conclusive(status) => {
            writeln!(&mut output, "state property: {}", status_label(status))
                .expect("writing to String cannot fail");
        }
        BoundedOutcome::Inconclusive(reason) => {
            writeln!(&mut output, "state property: INCONCLUSIVE")
                .expect("writing to String cannot fail");
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
    render_evidence(&mut output, result.evidence.as_ref());
    output
}

fn render_header(
    output: &mut String,
    model_name: &str,
    property: &str,
    proposition: &str,
    backend: ExactStateBackend,
) {
    writeln!(output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(output, "property: {property}").expect("writing to String cannot fail");
    writeln!(output, "proposition: {proposition:?}").expect("writing to String cannot fail");
    writeln!(
        output,
        "backend: {}",
        match backend {
            ExactStateBackend::Reachability => "REACHABILITY",
            ExactStateBackend::Eventuality => "EVENTUALITY",
        }
    )
    .expect("writing to String cannot fail");
}

fn status_label(status: ExactStateStatus) -> &'static str {
    match status {
        ExactStateStatus::Satisfied => "SATISFIED",
        ExactStateStatus::Violated => "VIOLATED",
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

fn render_evidence(output: &mut String, evidence: Option<&ExactStateEvidence>) {
    match evidence {
        None => writeln!(output, "evidence: none").expect("writing to String cannot fail"),
        Some(ExactStateEvidence::ReachabilityWitness { trace }) => {
            writeln!(output, "evidence: REACHABILITY_WITNESS")
                .expect("writing to String cannot fail");
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace, "initial");
        }
        Some(ExactStateEvidence::EventualityFiniteCounterexample { trace }) => {
            writeln!(output, "evidence: EVENTUALITY_FINITE_COUNTEREXAMPLE")
                .expect("writing to String cannot fail");
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace, "initial");
        }
        Some(ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle }) => {
            writeln!(output, "evidence: EVENTUALITY_INFINITE_COUNTEREXAMPLE")
                .expect("writing to String cannot fail");
            writeln!(output, "stem:").expect("writing to String cannot fail");
            render_trace(output, stem, "initial");
            writeln!(output, "cycle:").expect("writing to String cannot fail");
            render_trace(output, cycle, "cycle-entry");
        }
    }
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

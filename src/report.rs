use crate::checker::{CheckResult, InconclusiveReason, VerificationStatus};
use crate::property::{DeadlockResult, DeadlockStatus, ReachabilityResult, ReachabilityStatus};
use crate::recurrence::RecurrenceAnalysis;
use std::fmt::{Debug, Write};

/// Render a stable, line-oriented report suitable for the CLI and snapshots.
/// The checker itself does not depend on this presentation layer.
pub fn render_report<S: Debug>(model_name: &str, result: &CheckResult<S>) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "status: {}",
        match result.status {
            VerificationStatus::Safe => "SAFE",
            VerificationStatus::Violated => "VIOLATION",
            VerificationStatus::Inconclusive => "INCONCLUSIVE",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "discovered states: {}",
        result.discovered_states
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "checked states: {}", result.checked_states)
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

    writeln!(&mut output, "transitions by action:").expect("writing to String cannot fail");
    if result.transitions_by_action.is_empty() {
        writeln!(&mut output, "  (none)").expect("writing to String cannot fail");
    } else {
        for (action, count) in &result.transitions_by_action {
            writeln!(&mut output, "  {action}: {count}").expect("writing to String cannot fail");
        }
    }

    if let Some(reason) = result.inconclusive_reason {
        match reason {
            InconclusiveReason::StateLimitReached { limit } => {
                writeln!(
                    &mut output,
                    "inconclusive reason: state limit reached (max {limit})"
                )
            }
            InconclusiveReason::TransitionLimitReached { limit } => {
                writeln!(
                    &mut output,
                    "inconclusive reason: transition limit reached (max {limit})"
                )
            }
            InconclusiveReason::DepthLimitReached { limit } => {
                writeln!(
                    &mut output,
                    "inconclusive reason: depth limit reached (max {limit})"
                )
            }
        }
        .expect("writing to String cannot fail");
    }

    if let Some(counterexample) = &result.counterexample {
        writeln!(
            &mut output,
            "violated invariant: {}",
            counterexample.invariant
        )
        .expect("writing to String cannot fail");
        writeln!(&mut output, "counterexample:").expect("writing to String cannot fail");
        render_trace(&mut output, &counterexample.trace, "initial");
    }

    output
}

/// Render a stable report for an existential reachability query.
pub fn render_reachability_report<S: Debug>(
    model_name: &str,
    result: &ReachabilityResult<S>,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "reachability: {}",
        match result.status {
            ReachabilityStatus::Reachable => "REACHABLE",
            ReachabilityStatus::Unreachable => "UNREACHABLE",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "discovered states: {}",
        result.discovered_states
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "checked states: {}", result.checked_states)
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

    if let Some(witness) = &result.witness {
        writeln!(&mut output, "witness:").expect("writing to String cannot fail");
        render_trace(&mut output, witness, "initial");
    } else {
        writeln!(&mut output, "witness: none (reachable graph exhausted)")
            .expect("writing to String cannot fail");
    }

    output
}

/// Render a stable report for reachable deadlock/terminal-state analysis.
pub fn render_deadlock_report<S: Debug>(model_name: &str, result: &DeadlockResult<S>) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "property: {}", result.property).expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "deadlock: {}",
        match result.status {
            DeadlockStatus::DeadlockFound => "DEADLOCK_FOUND",
            DeadlockStatus::DeadlockFree => "DEADLOCK_FREE",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "discovered states: {}",
        result.discovered_states
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "checked states: {}", result.checked_states)
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

    if let Some(witness) = &result.witness {
        writeln!(&mut output, "deadlock witness:").expect("writing to String cannot fail");
        render_trace(&mut output, witness, "initial");
    } else {
        writeln!(
            &mut output,
            "deadlock witness: none (reachable graph exhausted)"
        )
        .expect("writing to String cannot fail");
    }

    output
}

/// Render deterministic SCC structure and, when present, a stem-plus-cycle
/// witness for the first cyclic component in canonical ordering.
pub fn render_recurrence_report<S: Debug>(
    model_name: &str,
    analysis: &RecurrenceAnalysis<S>,
) -> String {
    let mut output = String::new();
    let cyclic_count = analysis
        .components
        .iter()
        .filter(|component| component.cyclic)
        .count();

    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "recurrence: {}",
        if cyclic_count == 0 { "ACYCLIC" } else { "CYCLIC" }
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "discovered states: {}", analysis.discovered_states)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "explored transitions: {}",
        analysis.explored_transitions
    )
    .expect("writing to String cannot fail");
    match analysis.max_depth_reached {
        Some(depth) => writeln!(&mut output, "max depth reached: {depth}"),
        None => writeln!(&mut output, "max depth reached: none"),
    }
    .expect("writing to String cannot fail");
    writeln!(&mut output, "scc count: {}", analysis.components.len())
        .expect("writing to String cannot fail");
    writeln!(&mut output, "cyclic scc count: {cyclic_count}")
        .expect("writing to String cannot fail");

    for (index, component) in analysis.components.iter().enumerate() {
        writeln!(
            &mut output,
            "scc {index}: cyclic={} states={:?}",
            component.cyclic, component.states
        )
        .expect("writing to String cannot fail");
    }

    if let Some(witness) = &analysis.first_cycle {
        writeln!(&mut output, "cycle component: {}", witness.component_index)
            .expect("writing to String cannot fail");
        writeln!(&mut output, "stem:").expect("writing to String cannot fail");
        render_trace(&mut output, &witness.stem, "initial");
        writeln!(&mut output, "cycle:").expect("writing to String cannot fail");
        render_trace(&mut output, &witness.cycle, "cycle-entry");
    } else {
        writeln!(&mut output, "cycle witness: none")
            .expect("writing to String cannot fail");
    }

    output
}

fn render_trace<S: Debug>(
    output: &mut String,
    trace: &[crate::checker::TraceStep<S>],
    root_label: &str,
) {
    for (index, step) in trace.iter().enumerate() {
        match &step.action {
            None => writeln!(output, "  {index}: {:?} [{root_label}]", step.state),
            Some(action) => writeln!(output, "  {index}: --{action}--> {:?}", step.state),
        }
        .expect("writing to String cannot fail");
    }
}

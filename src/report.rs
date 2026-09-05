use crate::checker::{CheckResult, InconclusiveReason, VerificationStatus};
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

        for (index, step) in counterexample.trace.iter().enumerate() {
            match &step.action {
                None => writeln!(&mut output, "  {index}: {:?} [initial]", step.state),
                Some(action) => writeln!(&mut output, "  {index}: --{action}--> {:?}", step.state),
            }
            .expect("writing to String cannot fail");
        }
    }

    output
}

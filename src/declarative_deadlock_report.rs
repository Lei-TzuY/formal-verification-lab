use crate::bounded::BoundedOutcome;
use crate::declarative_deadlock::BoundedDeclarativeDeadlockResult;
use crate::property::DeadlockStatus;
use std::fmt::Write;

pub fn render_declarative_deadlock_report(
    model_name: &str,
    result: &BoundedDeclarativeDeadlockResult,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "deadlock policy: {}", result.expression)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "deadlock verification: {}",
        match &result.outcome {
            BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFree) => "DEADLOCK_FREE",
            BoundedOutcome::Conclusive(DeadlockStatus::DeadlockFound) => "DEADLOCK_FOUND",
            BoundedOutcome::Inconclusive(_) => "INCONCLUSIVE",
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
    writeln!(
        &mut output,
        "max depth reached: {}",
        result
            .max_depth_reached
            .map_or_else(|| "none".to_owned(), |depth| depth.to_string())
    )
    .expect("writing to String cannot fail");

    if let BoundedOutcome::Inconclusive(reason) = result.outcome {
        writeln!(&mut output, "inconclusive reason: {reason:?}")
            .expect("writing to String cannot fail");
    }

    match &result.witness {
        None => writeln!(&mut output, "witness: none").expect("writing to String cannot fail"),
        Some(trace) => {
            writeln!(&mut output, "witness:").expect("writing to String cannot fail");
            for (index, step) in trace.iter().enumerate() {
                match &step.action {
                    None => writeln!(&mut output, "  {index}: {} [initial]", step.state),
                    Some(action) => {
                        writeln!(&mut output, "  {index}: --{action}--> {}", step.state)
                    }
                }
                .expect("writing to String cannot fail");
            }
        }
    }

    output
}

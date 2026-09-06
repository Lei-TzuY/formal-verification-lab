use crate::fairness::WeakFairness;
use crate::temporal::TemporalResult;
use crate::temporal_report::render_temporal_report;
use std::fmt::{Debug, Write};

/// Render one unbounded temporal result together with the exact-action weak
/// fairness assumptions used to obtain it.
///
/// Historical no-fairness reports remain unchanged because callers use this
/// renderer only when the user explicitly supplied at least one assumption.
pub fn render_weak_fair_temporal_report<S: Debug>(
    model_name: &str,
    result: &TemporalResult<S>,
    fairness: &WeakFairness,
) -> String {
    let mut output = render_temporal_report(model_name, result);
    writeln!(&mut output, "weak fairness actions: {}", fairness.actions().len())
        .expect("writing to String cannot fail");
    for action in fairness.actions() {
        writeln!(&mut output, "weak-fair action: {}", quote_action(action))
            .expect("writing to String cannot fail");
    }
    output
}

fn quote_action(action: &str) -> String {
    let mut output = String::from("\"");
    for ch in action.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

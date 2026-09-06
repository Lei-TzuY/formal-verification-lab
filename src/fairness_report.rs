use crate::fairness::WeakFairness;
use crate::temporal::{AnalysisTemporalResult, BoundedTemporalResult, TemporalResult};
use crate::temporal_report::{
    render_analysis_temporal_report, render_bounded_temporal_report, render_temporal_report,
};
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
    append_weak_fairness(&mut output, fairness);
    output
}

/// Render a product-bounded temporal result together with the exact-action
/// weak-fairness assumptions used for recurrent counterexample filtering.
pub fn render_bounded_weak_fair_temporal_report<S: Debug>(
    model_name: &str,
    result: &BoundedTemporalResult<S>,
    fairness: &WeakFairness,
) -> String {
    let mut output = render_bounded_temporal_report(model_name, result);
    append_weak_fairness(&mut output, fairness);
    output
}

/// Render a staged model/product temporal result together with its exact-action
/// weak-fairness assumptions. Stage-qualified cutoff reporting remains owned by
/// the canonical temporal renderer.
pub fn render_analysis_weak_fair_temporal_report<S: Debug>(
    model_name: &str,
    result: &AnalysisTemporalResult<S>,
    fairness: &WeakFairness,
) -> String {
    let mut output = render_analysis_temporal_report(model_name, result);
    append_weak_fairness(&mut output, fairness);
    output
}

fn append_weak_fairness(output: &mut String, fairness: &WeakFairness) {
    writeln!(output, "weak fairness actions: {}", fairness.actions().len())
        .expect("writing to String cannot fail");
    for action in fairness.actions() {
        writeln!(output, "weak-fair action: {}", quote_action(action))
            .expect("writing to String cannot fail");
    }
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

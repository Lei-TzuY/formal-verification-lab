use crate::fairness::WeakFairness;
use crate::monitor::{AnalysisMonitorResult, BoundedMonitorResult, MonitorResult};
use crate::monitor_report::{
    render_analysis_monitor_report, render_bounded_monitor_report, render_monitor_report,
};
use crate::strong_fairness::StrongFairness;
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
    append_fairness(&mut output, "weak", fairness.actions());
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
    append_fairness(&mut output, "weak", fairness.actions());
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
    append_fairness(&mut output, "weak", fairness.actions());
    output
}

/// Render one unbounded temporal result together with the exact-action strong
/// fairness assumptions used to filter infinite executions.
pub fn render_strong_fair_temporal_report<S: Debug>(
    model_name: &str,
    result: &TemporalResult<S>,
    fairness: &StrongFairness,
) -> String {
    let mut output = render_temporal_report(model_name, result);
    append_fairness(&mut output, "strong", fairness.actions());
    output
}

/// Render a product-bounded temporal result with explicit strong-fairness
/// assumptions while preserving canonical product cutoff accounting.
pub fn render_bounded_strong_fair_temporal_report<S: Debug>(
    model_name: &str,
    result: &BoundedTemporalResult<S>,
    fairness: &StrongFairness,
) -> String {
    let mut output = render_bounded_temporal_report(model_name, result);
    append_fairness(&mut output, "strong", fairness.actions());
    output
}

/// Render staged model/product strong-fair temporal analysis. Stage-qualified
/// cutoff provenance remains owned by the canonical temporal renderer.
pub fn render_analysis_strong_fair_temporal_report<S: Debug>(
    model_name: &str,
    result: &AnalysisTemporalResult<S>,
    fairness: &StrongFairness,
) -> String {
    let mut output = render_analysis_temporal_report(model_name, result);
    append_fairness(&mut output, "strong", fairness.actions());
    output
}

/// Render one unbounded finite-monitor result with the exact-action weak
/// fairness assumptions that filtered only its infinite progress cycles.
pub fn render_weak_fair_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &MonitorResult<S, M>,
    fairness: &WeakFairness,
) -> String {
    let mut output = render_monitor_report(model_name, result);
    append_fairness(&mut output, "weak", fairness.actions());
    output
}

/// Render a product-bounded weak-fair finite-monitor result. Product cutoff
/// accounting remains owned by the canonical monitor renderer.
pub fn render_bounded_weak_fair_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &BoundedMonitorResult<S, M>,
    fairness: &WeakFairness,
) -> String {
    let mut output = render_bounded_monitor_report(model_name, result);
    append_fairness(&mut output, "weak", fairness.actions());
    output
}

/// Render staged model/product weak-fair monitor analysis while preserving the
/// canonical stage-qualified cutoff report and appending assumptions separately.
pub fn render_analysis_weak_fair_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &AnalysisMonitorResult<S, M>,
    fairness: &WeakFairness,
) -> String {
    let mut output = render_analysis_monitor_report(model_name, result);
    append_fairness(&mut output, "weak", fairness.actions());
    output
}

/// Render one unbounded finite-monitor result with the exact-action strong
/// fairness assumptions used only to filter its infinite progress cycles.
pub fn render_strong_fair_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &MonitorResult<S, M>,
    fairness: &StrongFairness,
) -> String {
    let mut output = render_monitor_report(model_name, result);
    append_fairness(&mut output, "strong", fairness.actions());
    output
}

/// Render a product-bounded strong-fair finite-monitor result while preserving
/// the canonical product cutoff accounting and reason text.
pub fn render_bounded_strong_fair_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &BoundedMonitorResult<S, M>,
    fairness: &StrongFairness,
) -> String {
    let mut output = render_bounded_monitor_report(model_name, result);
    append_fairness(&mut output, "strong", fairness.actions());
    output
}

/// Render staged model/product strong-fair monitor analysis while preserving
/// canonical stage-qualified cutoff provenance and appending assumptions separately.
pub fn render_analysis_strong_fair_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &AnalysisMonitorResult<S, M>,
    fairness: &StrongFairness,
) -> String {
    let mut output = render_analysis_monitor_report(model_name, result);
    append_fairness(&mut output, "strong", fairness.actions());
    output
}

fn append_fairness(output: &mut String, strength: &str, actions: &[String]) {
    writeln!(output, "{strength} fairness actions: {}", actions.len())
        .expect("writing to String cannot fail");
    for action in actions {
        writeln!(output, "{strength}-fair action: {}", quote_action(action))
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

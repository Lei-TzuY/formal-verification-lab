use crate::bounded::{AnalysisOutcome, AnalysisStage, BoundedOutcome};
use crate::bounded_report::format_inconclusive_reason;
use crate::temporal::{
    AnalysisTemporalResult, BoundedTemporalResult, TemporalBackend, TemporalCounterexample,
    TemporalObligation, TemporalResult, TemporalStatus,
};
use std::fmt::{Debug, Write};

/// Render a stable line-oriented report for the typed action-temporal frontend.
pub fn render_temporal_report<S: Debug>(model_name: &str, result: &TemporalResult<S>) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.property, result.backend);
    writeln!(&mut output, "temporal: {}", status_label(result.status))
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
    render_counterexample(&mut output, result.counterexample.as_ref(), false);
    output
}

/// Render a typed temporal analysis whose resource limits apply only to the
/// selected backend's action-product construction after complete model capture.
pub fn render_bounded_temporal_report<S: Debug>(
    model_name: &str,
    result: &BoundedTemporalResult<S>,
) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.property, result.backend);
    match result.outcome {
        BoundedOutcome::Conclusive(status) => {
            writeln!(&mut output, "temporal: {}", status_label(status))
                .expect("writing to String cannot fail");
        }
        BoundedOutcome::Inconclusive(reason) => {
            writeln!(&mut output, "temporal: INCONCLUSIVE").expect("writing to String cannot fail");
            writeln!(
                &mut output,
                "product inconclusive reason: {}",
                format_inconclusive_reason(reason)
            )
            .expect("writing to String cannot fail");
        }
    }
    writeln!(&mut output, "model states: {}", result.model_states)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "model transitions: {}",
        result.model_transitions
    )
    .expect("writing to String cannot fail");
    render_product_accounting(
        &mut output,
        result.product_states,
        result.checked_product_states,
        result.explored_product_transitions,
        result.retained_product_transitions,
        result.max_product_depth_reached,
    );

    let incomplete = matches!(result.outcome, BoundedOutcome::Inconclusive(_));
    if incomplete && result.counterexample.is_none() {
        writeln!(
            &mut output,
            "counterexample: none (product exploration incomplete)"
        )
        .expect("writing to String cannot fail");
    } else {
        render_counterexample(&mut output, result.counterexample.as_ref(), false);
    }
    output
}

/// Render a typed temporal analysis under independent model-capture and
/// action-product resource budgets.
pub fn render_analysis_temporal_report<S: Debug>(
    model_name: &str,
    result: &AnalysisTemporalResult<S>,
) -> String {
    let mut output = String::new();
    render_header(&mut output, model_name, &result.property, result.backend);
    match result.outcome {
        AnalysisOutcome::Conclusive(status) => {
            writeln!(&mut output, "temporal: {}", status_label(status))
                .expect("writing to String cannot fail");
        }
        AnalysisOutcome::Inconclusive(reason) => {
            writeln!(&mut output, "temporal: INCONCLUSIVE").expect("writing to String cannot fail");
            writeln!(
                &mut output,
                "analysis inconclusive stage: {}",
                stage_label(reason.stage)
            )
            .expect("writing to String cannot fail");
            writeln!(
                &mut output,
                "analysis inconclusive reason: {}",
                format_inconclusive_reason(reason.reason)
            )
            .expect("writing to String cannot fail");
        }
    }

    render_stage_completion(&mut output, "model", &result.model_completion);
    render_stage_completion(&mut output, "product", &result.product_completion);
    writeln!(&mut output, "model states: {}", result.model_states)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "checked model states: {}",
        result.checked_model_states
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "explored model transitions: {}",
        result.explored_model_transitions
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "retained model transitions: {}",
        result.retained_model_transitions
    )
    .expect("writing to String cannot fail");
    match result.max_model_depth_reached {
        Some(depth) => writeln!(&mut output, "max model depth reached: {depth}"),
        None => writeln!(&mut output, "max model depth reached: none"),
    }
    .expect("writing to String cannot fail");
    render_product_accounting(
        &mut output,
        result.product_states,
        result.checked_product_states,
        result.explored_product_transitions,
        result.retained_product_transitions,
        result.max_product_depth_reached,
    );

    let incomplete = matches!(result.outcome, AnalysisOutcome::Inconclusive(_));
    render_counterexample(&mut output, result.counterexample.as_ref(), incomplete);
    output
}

fn render_header(output: &mut String, model_name: &str, property: &str, backend: TemporalBackend) {
    writeln!(output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(output, "property: {property}").expect("writing to String cannot fail");
    writeln!(
        output,
        "backend: {}",
        match backend {
            TemporalBackend::Response => "RESPONSE",
            TemporalBackend::Buchi => "BUCHI",
        }
    )
    .expect("writing to String cannot fail");
}

fn status_label(status: TemporalStatus) -> &'static str {
    match status {
        TemporalStatus::Satisfied => "SATISFIED",
        TemporalStatus::Violated => "VIOLATED",
    }
}

fn stage_label(stage: AnalysisStage) -> &'static str {
    match stage {
        AnalysisStage::Model => "model",
        AnalysisStage::Product => "product",
    }
}

fn render_stage_completion(output: &mut String, stage: &str, completion: &BoundedOutcome<()>) {
    match completion {
        BoundedOutcome::Conclusive(()) => {
            writeln!(output, "{stage} completion: COMPLETE")
                .expect("writing to String cannot fail");
        }
        BoundedOutcome::Inconclusive(reason) => {
            writeln!(output, "{stage} completion: INCONCLUSIVE")
                .expect("writing to String cannot fail");
            writeln!(
                output,
                "{stage} inconclusive reason: {}",
                format_inconclusive_reason(*reason)
            )
            .expect("writing to String cannot fail");
        }
    }
}

fn render_product_accounting(
    output: &mut String,
    product_states: usize,
    checked_product_states: usize,
    explored_product_transitions: usize,
    retained_product_transitions: usize,
    max_product_depth_reached: Option<usize>,
) {
    writeln!(output, "product states: {product_states}").expect("writing to String cannot fail");
    writeln!(output, "checked product states: {checked_product_states}")
        .expect("writing to String cannot fail");
    writeln!(
        output,
        "explored product transitions: {explored_product_transitions}"
    )
    .expect("writing to String cannot fail");
    writeln!(
        output,
        "retained product transitions: {retained_product_transitions}"
    )
    .expect("writing to String cannot fail");
    match max_product_depth_reached {
        Some(depth) => writeln!(output, "max product depth reached: {depth}"),
        None => writeln!(output, "max product depth reached: none"),
    }
    .expect("writing to String cannot fail");
}

fn render_counterexample<S: Debug>(
    output: &mut String,
    counterexample: Option<&TemporalCounterexample<S>>,
    incomplete: bool,
) {
    match counterexample {
        None if incomplete => writeln!(output, "counterexample: none (analysis incomplete)")
            .expect("writing to String cannot fail"),
        None => writeln!(output, "counterexample: none").expect("writing to String cannot fail"),
        Some(TemporalCounterexample::Finite { obligation, trace }) => {
            writeln!(output, "counterexample: FINITE").expect("writing to String cannot fail");
            render_obligation(output, obligation);
            writeln!(output, "trace:").expect("writing to String cannot fail");
            render_trace(output, trace, "initial");
        }
        Some(TemporalCounterexample::Infinite {
            obligation,
            stem,
            cycle,
        }) => {
            writeln!(output, "counterexample: INFINITE").expect("writing to String cannot fail");
            render_obligation(output, obligation);
            writeln!(output, "stem:").expect("writing to String cannot fail");
            render_trace(output, stem, "initial");
            writeln!(output, "cycle:").expect("writing to String cannot fail");
            render_trace(output, cycle, "cycle-entry");
        }
    }
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

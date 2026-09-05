use crate::buchi::{
    BuchiCounterexample, BuchiProductState, BuchiResult, BuchiStatus, FiniteRunPolicy,
};
use crate::checker::TraceStep;
use std::fmt::{Debug, Write};

pub fn render_buchi_report<S: Debug, A: Debug>(
    model_name: &str,
    result: &BuchiResult<S, A>,
) -> String {
    let mut output = String::new();
    writeln!(&mut output, "model: {model_name}").expect("writing to String cannot fail");
    writeln!(&mut output, "Buchi automaton: {}", result.automaton)
        .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "Buchi verification: {}",
        match result.status {
            BuchiStatus::Satisfied => "SATISFIED",
            BuchiStatus::Violated => "VIOLATED",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(
        &mut output,
        "finite policy: {}",
        match result.finite_policy {
            FiniteRunPolicy::IgnoreTerminals => "IGNORE_TERMINALS",
            FiniteRunPolicy::RequireAcceptingTerminal => "REQUIRE_ACCEPTING_TERMINAL",
        }
    )
    .expect("writing to String cannot fail");
    writeln!(&mut output, "acceptance sets: {}", result.acceptance_sets)
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
            "counterexample: none (all configured acceptance obligations hold)"
        )
        .expect("writing to String cannot fail"),
        Some(BuchiCounterexample::FiniteTerminal {
            missing_acceptance,
            trace,
        }) => {
            writeln!(&mut output, "missing acceptance set: {missing_acceptance}")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "counterexample: FINITE_TERMINAL")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "trace:").expect("writing to String cannot fail");
            render_trace(&mut output, trace, "initial");
        }
        Some(BuchiCounterexample::AcceptanceAvoidingCycle {
            acceptance,
            stem,
            cycle,
        }) => {
            writeln!(&mut output, "avoided acceptance set: {acceptance}")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "counterexample: ACCEPTANCE_AVOIDING_CYCLE")
                .expect("writing to String cannot fail");
            writeln!(&mut output, "stem:").expect("writing to String cannot fail");
            render_trace(&mut output, stem, "initial");
            writeln!(&mut output, "cycle:").expect("writing to String cannot fail");
            render_trace(&mut output, cycle, "cycle-entry");
        }
    }

    output
}

fn render_trace<S: Debug, A: Debug>(
    output: &mut String,
    trace: &[TraceStep<BuchiProductState<S, A>>],
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

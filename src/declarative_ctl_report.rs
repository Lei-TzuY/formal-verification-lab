use crate::bounded::BoundedOutcome;
use crate::bounded_report::format_inconclusive_reason;
use crate::ctl::{CtlEvidence, CtlEvidenceAction, CtlEvidenceStep};
use crate::ctl_bounded::{BoundedCtlStatus, BoundedCtlTruth};
use crate::declarative_ctl::{
    BoundedDeclarativeCtlResult, DeclarativeCtlResult, DeclarativeCtlStatus,
};

pub fn render_bounded_declarative_ctl_report(
    model_name: &str,
    result: &BoundedDeclarativeCtlResult,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("model: {model_name}\n"));
    output.push_str(&format!("formula: {}\n", result.formula));
    output.push_str(&format!(
        "CTL: {}\n",
        match &result.evaluation.outcome {
            BoundedOutcome::Conclusive(BoundedCtlStatus::Satisfied) => "SATISFIED",
            BoundedOutcome::Conclusive(BoundedCtlStatus::Violated) => "VIOLATED",
            BoundedOutcome::Inconclusive(_) => "INCONCLUSIVE",
        }
    ));
    if let BoundedOutcome::Inconclusive(reason) = result.evaluation.outcome {
        output.push_str(&format!(
            "inconclusive reason: {}\n",
            format_inconclusive_reason(reason)
        ));
    }
    output.push_str("initial semantics: all initial states must satisfy the formula\n");
    output.push_str("terminal policy: totalize proven terminals with synthetic self-loops\n");
    output.push_str(&format!(
        "initial states complete: {}\n",
        result.evaluation.initial_states_complete
    ));
    output.push_str(&format!(
        "discovered states: {}\n",
        result.evaluation.discovered_states
    ));
    output.push_str(&format!(
        "checked states: {}\n",
        result.evaluation.checked_states
    ));
    output.push_str(&format!(
        "explored transitions: {}\n",
        result.evaluation.explored_transitions
    ));
    output.push_str(&format!(
        "max depth reached: {}\n",
        result
            .evaluation
            .max_depth_reached
            .map_or_else(|| "none".to_owned(), |depth| depth.to_string())
    ));
    output.push_str(&format!(
        "memoized subformulas: {}\n",
        result.evaluation.memoized_subformulas
    ));
    output.push_str(&format!(
        "definitely satisfying states: {}/{}\n",
        result.evaluation.definitely_satisfying_state_indices.len(),
        result.evaluation.reachable_states.len()
    ));
    output.push_str(&format!(
        "possibly satisfying states: {}/{}\n",
        result.evaluation.possibly_satisfying_state_indices.len(),
        result.evaluation.reachable_states.len()
    ));

    for initial in &result.evaluation.initial {
        output.push_str(&format!(
            "initial state {}: {} {:?}\n",
            initial.state_index,
            match initial.truth {
                BoundedCtlTruth::True => "TRUE",
                BoundedCtlTruth::False => "FALSE",
                BoundedCtlTruth::Unknown => "UNKNOWN",
            },
            initial.state
        ));
        match &initial.evidence {
            Some(evidence) => render_evidence(&mut output, evidence),
            None => output.push_str("  evidence: none\n"),
        }
    }

    output
}

pub fn render_declarative_ctl_report(model_name: &str, result: &DeclarativeCtlResult) -> String {
    let mut output = String::new();
    output.push_str(&format!("model: {model_name}\n"));
    output.push_str(&format!("formula: {}\n", result.formula));
    output.push_str(&format!(
        "CTL: {}\n",
        match result.status {
            DeclarativeCtlStatus::Satisfied => "SATISFIED",
            DeclarativeCtlStatus::Violated => "VIOLATED",
        }
    ));
    output.push_str("initial semantics: all initial states must satisfy the formula\n");
    output.push_str("terminal policy: totalize reachable terminals with synthetic self-loops\n");
    output.push_str(&format!(
        "discovered states: {}\n",
        result.evaluation.discovered_states
    ));
    output.push_str(&format!(
        "explored transitions: {}\n",
        result.evaluation.explored_transitions
    ));
    output.push_str(&format!(
        "max depth reached: {}\n",
        result
            .evaluation
            .max_depth_reached
            .map_or_else(|| "none".to_owned(), |depth| depth.to_string())
    ));
    output.push_str(&format!(
        "memoized subformulas: {}\n",
        result.evaluation.memoized_subformulas
    ));
    output.push_str(&format!(
        "satisfying states: {}/{}\n",
        result.evaluation.satisfying_state_indices.len(),
        result.evaluation.reachable_states.len()
    ));

    for initial in &result.evaluation.initial {
        output.push_str(&format!(
            "initial state {}: {} {:?}\n",
            initial.state_index,
            if initial.satisfied {
                "SATISFIED"
            } else {
                "VIOLATED"
            },
            initial.state
        ));
        match &initial.evidence {
            Some(evidence) => render_evidence(&mut output, evidence),
            None => output.push_str("  evidence: none\n"),
        }
    }

    output
}

fn render_evidence(output: &mut String, evidence: &CtlEvidence<String>) {
    match evidence {
        CtlEvidence::Finite { trace } => {
            output.push_str("  evidence: FINITE\n");
            render_trace(output, trace);
        }
        CtlEvidence::Lasso { trace, cycle_start } => {
            output.push_str("  evidence: LASSO\n");
            output.push_str(&format!("  cycle start trace index: {cycle_start}\n"));
            render_trace(output, trace);
        }
    }
}

fn render_trace(output: &mut String, trace: &[CtlEvidenceStep<String>]) {
    for (index, step) in trace.iter().enumerate() {
        match &step.action {
            None => output.push_str(&format!("    [{index}] {:?} [initial]\n", step.state)),
            Some(CtlEvidenceAction::Model(action)) => output.push_str(&format!(
                "    [{index}] --{:?}--> {:?}\n",
                action, step.state
            )),
            Some(CtlEvidenceAction::TerminalSelfLoop) => output.push_str(&format!(
                "    [{index}] --<terminal-self-loop>--> {:?}\n",
                step.state
            )),
        }
    }
}

use crate::bounded::BoundedOutcome;
use crate::bounded_report::format_inconclusive_reason;
use crate::declarative_mu::{
    BoundedDeclarativeMuResult, DeclarativeMuParityResult, DeclarativeMuResult,
    DeclarativeMuStatus,
};
use crate::mu_bounded::{BoundedMuStatus, BoundedMuTruth};

pub fn render_bounded_declarative_mu_report(
    model_name: &str,
    result: &BoundedDeclarativeMuResult,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("model: {model_name}\n"));
    output.push_str(&format!("formula: {}\n", result.formula));
    output.push_str(&format!(
        "MU: {}\n",
        match &result.evaluation.outcome {
            BoundedOutcome::Conclusive(BoundedMuStatus::Satisfied) => "SATISFIED",
            BoundedOutcome::Conclusive(BoundedMuStatus::Violated) => "VIOLATED",
            BoundedOutcome::Inconclusive(_) => "INCONCLUSIVE",
        }
    ));
    if let BoundedOutcome::Inconclusive(reason) = &result.evaluation.outcome {
        output.push_str(&format!(
            "inconclusive reason: {}\n",
            format_inconclusive_reason(*reason)
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
        "fixpoint iterations: {}\n",
        result.evaluation.fixpoint_iterations
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
                BoundedMuTruth::True => "TRUE",
                BoundedMuTruth::False => "FALSE",
                BoundedMuTruth::Unknown => "UNKNOWN",
            },
            initial.state
        ));
    }

    output
}

pub fn render_declarative_mu_parity_report(
    model_name: &str,
    result: &DeclarativeMuParityResult,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("model: {model_name}\n"));
    output.push_str(&format!("formula: {}\n", result.formula));
    output.push_str("backend: parity-game\n");
    output.push_str(&format!(
        "MU: {}\n",
        match result.status {
            DeclarativeMuStatus::Satisfied => "SATISFIED",
            DeclarativeMuStatus::Violated => "VIOLATED",
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
        "parity game vertices: {}\n",
        result.evaluation.parity_game_vertices
    ));
    output.push_str(&format!(
        "max parity priority: {}\n",
        result.evaluation.max_priority
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
    }
    output
}

pub fn render_declarative_mu_report(model_name: &str, result: &DeclarativeMuResult) -> String {
    let mut output = String::new();
    output.push_str(&format!("model: {model_name}\n"));
    output.push_str(&format!("formula: {}\n", result.formula));
    output.push_str(&format!(
        "MU: {}\n",
        match result.status {
            DeclarativeMuStatus::Satisfied => "SATISFIED",
            DeclarativeMuStatus::Violated => "VIOLATED",
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
        "fixpoint iterations: {}\n",
        result.evaluation.fixpoint_iterations
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
    }
    output
}

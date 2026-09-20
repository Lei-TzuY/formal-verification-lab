use crate::declarative_mu::{DeclarativeMuResult, DeclarativeMuStatus};

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

use crate::bounded::BoundedOutcome;
use crate::checker::{ExplorationLimits, InconclusiveReason, TraceStep};
use crate::recurrence::{
    BoundedRecurrenceResult, CycleWitness, RecurrenceStatus, StronglyConnectedComponent,
};
use std::fmt::Write;

pub const STRUCTURAL_JOB_RESULT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralJobOutcome {
    CycleFound,
    Acyclic,
    Inconclusive,
    Error,
}

impl StructuralJobOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CycleFound => "cycle_found",
            Self::Acyclic => "acyclic",
            Self::Inconclusive => "inconclusive",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobLimits {
    pub max_states: Option<usize>,
    pub max_transitions: Option<usize>,
    pub max_depth: Option<usize>,
}

impl From<ExplorationLimits> for StructuralJobLimits {
    fn from(value: ExplorationLimits) -> Self {
        Self {
            max_states: value.max_states,
            max_transitions: value.max_transitions,
            max_depth: value.max_depth,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobAccounting {
    pub discovered_states: Option<usize>,
    pub checked_states: Option<usize>,
    pub explored_transitions: Option<usize>,
    pub max_depth_reached: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralJobCutoffKind {
    StateLimit,
    TransitionLimit,
    DepthLimit,
}

impl StructuralJobCutoffKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::StateLimit => "state_limit",
            Self::TransitionLimit => "transition_limit",
            Self::DepthLimit => "depth_limit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralJobCutoff {
    pub kind: StructuralJobCutoffKind,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobComponent {
    pub states: Vec<String>,
    pub cyclic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobTraceStep {
    pub action: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobCycleEvidence {
    pub component_index: usize,
    pub stem: Vec<StructuralJobTraceStep>,
    pub cycle: Vec<StructuralJobTraceStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralJobResultEnvelope {
    pub schema_version: u32,
    pub analysis: String,
    pub outcome: StructuralJobOutcome,
    pub model: Option<String>,
    pub model_limits: StructuralJobLimits,
    pub accounting: StructuralJobAccounting,
    /// Present whenever model exploration hit a configured resource cutoff,
    /// including when a retained real cycle already makes the structural result
    /// conclusive.
    pub cutoff: Option<StructuralJobCutoff>,
    /// Complete reachable-graph SCC partition. None means exploration did not
    /// complete; it is never a partial partition advertised as exhaustive.
    pub components: Option<Vec<StructuralJobComponent>>,
    pub evidence: Option<StructuralJobCycleEvidence>,
    pub error: Option<String>,
}

impl StructuralJobResultEnvelope {
    pub fn from_recurrence(
        model: impl Into<String>,
        limits: ExplorationLimits,
        result: &BoundedRecurrenceResult<String>,
    ) -> Self {
        Self {
            schema_version: STRUCTURAL_JOB_RESULT_SCHEMA_VERSION,
            analysis: "recurrence".to_owned(),
            outcome: match result.outcome {
                BoundedOutcome::Conclusive(RecurrenceStatus::CycleFound) => {
                    StructuralJobOutcome::CycleFound
                }
                BoundedOutcome::Conclusive(RecurrenceStatus::Acyclic) => {
                    StructuralJobOutcome::Acyclic
                }
                BoundedOutcome::Inconclusive(_) => StructuralJobOutcome::Inconclusive,
            },
            model: Some(model.into()),
            model_limits: limits.into(),
            accounting: StructuralJobAccounting {
                discovered_states: Some(result.discovered_states),
                checked_states: Some(result.checked_states),
                explored_transitions: Some(result.explored_transitions),
                max_depth_reached: result.max_depth_reached,
            },
            cutoff: result.cutoff_reason.map(cutoff),
            components: result
                .components
                .as_ref()
                .map(|components| components.iter().map(convert_component).collect()),
            evidence: result.first_cycle.as_ref().map(convert_cycle),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: STRUCTURAL_JOB_RESULT_SCHEMA_VERSION,
            analysis: "recurrence".to_owned(),
            outcome: StructuralJobOutcome::Error,
            model: None,
            model_limits: ExplorationLimits::unbounded().into(),
            accounting: StructuralJobAccounting {
                discovered_states: None,
                checked_states: None,
                explored_transitions: None,
                max_depth_reached: None,
            },
            cutoff: None,
            components: None,
            evidence: None,
            error: Some(message.into()),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u64(&mut out, "schema_version", self.schema_version as u64, true);
        field_string(&mut out, "analysis", &self.analysis, false);
        field_string(&mut out, "outcome", self.outcome.as_str(), false);
        field_optional_string(&mut out, "model", self.model.as_deref(), false);
        field_limits(&mut out, &self.model_limits);
        field_accounting(&mut out, &self.accounting);

        out.push_str(",\"cutoff\":");
        match self.cutoff {
            Some(value) => write_cutoff(&mut out, value),
            None => out.push_str("null"),
        }

        out.push_str(",\"components\":");
        match &self.components {
            Some(components) => write_components(&mut out, components),
            None => out.push_str("null"),
        }

        out.push_str(",\"evidence\":");
        match &self.evidence {
            Some(evidence) => write_evidence(&mut out, evidence),
            None => out.push_str("null"),
        }

        out.push_str(",\"error\":");
        write_optional_string(&mut out, self.error.as_deref());
        out.push('}');
        out
    }
}

fn cutoff(reason: InconclusiveReason) -> StructuralJobCutoff {
    match reason {
        InconclusiveReason::StateLimitReached { limit } => StructuralJobCutoff {
            kind: StructuralJobCutoffKind::StateLimit,
            limit,
        },
        InconclusiveReason::TransitionLimitReached { limit } => StructuralJobCutoff {
            kind: StructuralJobCutoffKind::TransitionLimit,
            limit,
        },
        InconclusiveReason::DepthLimitReached { limit } => StructuralJobCutoff {
            kind: StructuralJobCutoffKind::DepthLimit,
            limit,
        },
    }
}

fn convert_component(component: &StronglyConnectedComponent<String>) -> StructuralJobComponent {
    StructuralJobComponent {
        states: component.states.clone(),
        cyclic: component.cyclic,
    }
}

fn convert_cycle(witness: &CycleWitness<String>) -> StructuralJobCycleEvidence {
    StructuralJobCycleEvidence {
        component_index: witness.component_index,
        stem: witness.stem.iter().map(convert_step).collect(),
        cycle: witness.cycle.iter().map(convert_step).collect(),
    }
}

fn convert_step(step: &TraceStep<String>) -> StructuralJobTraceStep {
    StructuralJobTraceStep {
        action: step.action.clone(),
        state: step.state.clone(),
    }
}

fn field_name(out: &mut String, name: &str, first: bool) {
    if !first {
        out.push(',');
    }
    write_json_string(out, name);
    out.push(':');
}

fn field_u64(out: &mut String, name: &str, value: u64, first: bool) {
    field_name(out, name, first);
    out.push_str(&value.to_string());
}

fn field_string(out: &mut String, name: &str, value: &str, first: bool) {
    field_name(out, name, first);
    write_json_string(out, value);
}

fn field_optional_string(out: &mut String, name: &str, value: Option<&str>, first: bool) {
    field_name(out, name, first);
    write_optional_string(out, value);
}

fn field_limits(out: &mut String, limits: &StructuralJobLimits) {
    out.push_str(",\"model_limits\":{");
    field_optional_usize(out, "max_states", limits.max_states, true);
    field_optional_usize(out, "max_transitions", limits.max_transitions, false);
    field_optional_usize(out, "max_depth", limits.max_depth, false);
    out.push('}');
}

fn field_accounting(out: &mut String, accounting: &StructuralJobAccounting) {
    out.push_str(",\"accounting\":{");
    field_optional_usize(out, "discovered_states", accounting.discovered_states, true);
    field_optional_usize(out, "checked_states", accounting.checked_states, false);
    field_optional_usize(
        out,
        "explored_transitions",
        accounting.explored_transitions,
        false,
    );
    field_optional_usize(
        out,
        "max_depth_reached",
        accounting.max_depth_reached,
        false,
    );
    out.push('}');
}

fn field_optional_usize(out: &mut String, name: &str, value: Option<usize>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

fn write_cutoff(out: &mut String, cutoff: StructuralJobCutoff) {
    out.push('{');
    field_string(out, "kind", cutoff.kind.as_str(), true);
    field_u64(out, "limit", cutoff.limit as u64, false);
    out.push('}');
}

fn write_components(out: &mut String, components: &[StructuralJobComponent]) {
    out.push('[');
    for (index, component) in components.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"states\":[");
        for (state_index, state) in component.states.iter().enumerate() {
            if state_index != 0 {
                out.push(',');
            }
            write_json_string(out, state);
        }
        out.push(']');
        out.push_str(",\"cyclic\":");
        out.push_str(if component.cyclic { "true" } else { "false" });
        out.push('}');
    }
    out.push(']');
}

fn write_evidence(out: &mut String, evidence: &StructuralJobCycleEvidence) {
    out.push('{');
    field_u64(
        out,
        "component_index",
        evidence.component_index as u64,
        true,
    );
    out.push_str(",\"stem\":");
    write_trace(out, &evidence.stem);
    out.push_str(",\"cycle\":");
    write_trace(out, &evidence.cycle);
    out.push('}');
}

fn write_trace(out: &mut String, trace: &[StructuralJobTraceStep]) {
    out.push('[');
    for (index, step) in trace.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str("\"action\":");
        write_optional_string(out, step.action.as_deref());
        out.push_str(",\"state\":");
        write_json_string(out, &step.state);
        out.push('}');
    }
    out.push(']');
}

fn write_optional_string(out: &mut String, value: Option<&str>) {
    match value {
        Some(value) => write_json_string(out, value),
        None => out.push_str("null"),
    }
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch < '\u{20}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
}

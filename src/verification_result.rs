use crate::bounded::{AnalysisOutcome, AnalysisStage, BoundedOutcome};
use crate::checker::{ExplorationLimits, InconclusiveReason, TraceStep};
use crate::exact_state::{BoundedExactStateResult, ExactStateEvidence, ExactStateStatus};
use crate::multi_response::{
    AnalysisMultiResponseResult, BoundedMultiResponseResult, MultiObligationState,
    MultiResponseCounterexample, MultiResponseResult, MultiResponseStatus,
};
use crate::safety::{BoundedSafetyResult, SafetyStatus};

pub const VERIFICATION_JOB_RESULT_SCHEMA_VERSION: u32 = 1;
pub const VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION: u32 = 2;
pub const VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION: u32 =
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION;
pub const VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION: u32 =
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationJobOutcome {
    Satisfied,
    Violated,
    Inconclusive,
    Error,
}

impl VerificationJobOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Violated => "violated",
            Self::Inconclusive => "inconclusive",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobLimits {
    pub max_states: Option<usize>,
    pub max_transitions: Option<usize>,
    pub max_depth: Option<usize>,
}

impl From<ExplorationLimits> for VerificationJobLimits {
    fn from(value: ExplorationLimits) -> Self {
        Self {
            max_states: value.max_states,
            max_transitions: value.max_transitions,
            max_depth: value.max_depth,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobAccounting {
    pub model_states: Option<usize>,
    pub checked_model_states: Option<usize>,
    pub explored_model_transitions: Option<usize>,
    pub retained_model_transitions: Option<usize>,
    pub max_model_depth_reached: Option<usize>,
    pub product_states: Option<usize>,
    pub checked_product_states: Option<usize>,
    pub explored_product_transitions: Option<usize>,
    pub retained_product_transitions: Option<usize>,
    pub max_product_depth_reached: Option<usize>,
}

impl VerificationJobAccounting {
    fn empty() -> Self {
        Self {
            model_states: None,
            checked_model_states: None,
            explored_model_transitions: None,
            retained_model_transitions: None,
            max_model_depth_reached: None,
            product_states: None,
            checked_product_states: None,
            explored_product_transitions: None,
            retained_product_transitions: None,
            max_product_depth_reached: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationJobCutoffStage {
    Model,
    Product,
}

impl VerificationJobCutoffStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Product => "product",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationJobCutoffKind {
    StateLimit,
    TransitionLimit,
    DepthLimit,
}

impl VerificationJobCutoffKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::StateLimit => "state_limit",
            Self::TransitionLimit => "transition_limit",
            Self::DepthLimit => "depth_limit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationJobCutoff {
    pub stage: VerificationJobCutoffStage,
    pub kind: VerificationJobCutoffKind,
    pub limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobTraceStep {
    pub action: Option<String>,
    pub state: String,
    pub pending: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobStateTraceStep {
    pub action: Option<String>,
    pub state: String,
}

pub type VerificationJobSafetyTraceStep = VerificationJobStateTraceStep;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationJobEvidence {
    Finite {
        clause: String,
        trace: Vec<VerificationJobTraceStep>,
    },
    Infinite {
        clause: String,
        stem: Vec<VerificationJobTraceStep>,
        cycle: Vec<VerificationJobTraceStep>,
    },
    Safety {
        trace: Vec<VerificationJobStateTraceStep>,
    },
    ExactStateReachability {
        trace: Vec<VerificationJobStateTraceStep>,
    },
    ExactStateEventualityFinite {
        trace: Vec<VerificationJobStateTraceStep>,
    },
    ExactStateEventualityInfinite {
        stem: Vec<VerificationJobStateTraceStep>,
        cycle: Vec<VerificationJobStateTraceStep>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationJobResultEnvelope {
    pub schema_version: u32,
    /// Present only for heterogeneous schema revisions. Historical M56
    /// multi-response envelopes intentionally omit this field from JSON.
    pub analysis: Option<String>,
    pub outcome: VerificationJobOutcome,
    pub model: Option<String>,
    pub property: Option<String>,
    pub weak_fair_actions: Vec<String>,
    pub strong_fair_actions: Vec<String>,
    pub model_limits: VerificationJobLimits,
    pub product_limits: VerificationJobLimits,
    pub accounting: VerificationJobAccounting,
    pub cutoff: Option<VerificationJobCutoff>,
    pub evidence: Option<VerificationJobEvidence>,
    pub error: Option<String>,
}

impl VerificationJobResultEnvelope {
    pub fn from_unbounded(
        model: impl Into<String>,
        weak_fair_actions: &[String],
        strong_fair_actions: &[String],
        model_limits: ExplorationLimits,
        product_limits: ExplorationLimits,
        result: &MultiResponseResult<String>,
    ) -> Self {
        Self {
            schema_version: VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            analysis: None,
            outcome: status_outcome(result.status),
            model: Some(model.into()),
            property: Some(result.property.clone()),
            weak_fair_actions: weak_fair_actions.to_vec(),
            strong_fair_actions: strong_fair_actions.to_vec(),
            model_limits: model_limits.into(),
            product_limits: product_limits.into(),
            accounting: VerificationJobAccounting {
                model_states: Some(result.model_states),
                checked_model_states: None,
                explored_model_transitions: Some(result.model_transitions),
                retained_model_transitions: Some(result.model_transitions),
                max_model_depth_reached: None,
                product_states: Some(result.product_states),
                checked_product_states: None,
                explored_product_transitions: Some(result.product_transitions),
                retained_product_transitions: Some(result.product_transitions),
                max_product_depth_reached: None,
            },
            cutoff: None,
            evidence: result.counterexample.as_ref().map(convert_evidence),
            error: None,
        }
    }

    pub fn from_product_bounded(
        model: impl Into<String>,
        weak_fair_actions: &[String],
        strong_fair_actions: &[String],
        model_limits: ExplorationLimits,
        product_limits: ExplorationLimits,
        result: &BoundedMultiResponseResult<String>,
    ) -> Self {
        Self {
            schema_version: VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            analysis: None,
            outcome: bounded_outcome(&result.outcome),
            model: Some(model.into()),
            property: Some(result.property.clone()),
            weak_fair_actions: weak_fair_actions.to_vec(),
            strong_fair_actions: strong_fair_actions.to_vec(),
            model_limits: model_limits.into(),
            product_limits: product_limits.into(),
            accounting: VerificationJobAccounting {
                model_states: Some(result.model_states),
                checked_model_states: None,
                explored_model_transitions: Some(result.model_transitions),
                retained_model_transitions: Some(result.model_transitions),
                max_model_depth_reached: None,
                product_states: Some(result.product_states),
                checked_product_states: Some(result.checked_product_states),
                explored_product_transitions: Some(result.explored_product_transitions),
                retained_product_transitions: Some(result.retained_product_transitions),
                max_product_depth_reached: result.max_product_depth_reached,
            },
            cutoff: result
                .outcome
                .inconclusive_reason()
                .map(|reason| cutoff(VerificationJobCutoffStage::Product, reason)),
            evidence: result.counterexample.as_ref().map(convert_evidence),
            error: None,
        }
    }

    pub fn from_staged(
        model: impl Into<String>,
        weak_fair_actions: &[String],
        strong_fair_actions: &[String],
        model_limits: ExplorationLimits,
        product_limits: ExplorationLimits,
        result: &AnalysisMultiResponseResult<String>,
    ) -> Self {
        let result_cutoff = result.outcome.inconclusive_reason().map(|reason| {
            cutoff(
                match reason.stage {
                    AnalysisStage::Model => VerificationJobCutoffStage::Model,
                    AnalysisStage::Product => VerificationJobCutoffStage::Product,
                },
                reason.reason,
            )
        });
        Self {
            schema_version: VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            analysis: None,
            outcome: analysis_outcome(&result.outcome),
            model: Some(model.into()),
            property: Some(result.property.clone()),
            weak_fair_actions: weak_fair_actions.to_vec(),
            strong_fair_actions: strong_fair_actions.to_vec(),
            model_limits: model_limits.into(),
            product_limits: product_limits.into(),
            accounting: VerificationJobAccounting {
                model_states: Some(result.model_states),
                checked_model_states: Some(result.checked_model_states),
                explored_model_transitions: Some(result.explored_model_transitions),
                retained_model_transitions: Some(result.retained_model_transitions),
                max_model_depth_reached: result.max_model_depth_reached,
                product_states: Some(result.product_states),
                checked_product_states: Some(result.checked_product_states),
                explored_product_transitions: Some(result.explored_product_transitions),
                retained_product_transitions: Some(result.retained_product_transitions),
                max_product_depth_reached: result.max_product_depth_reached,
            },
            cutoff: result_cutoff,
            evidence: result.counterexample.as_ref().map(convert_evidence),
            error: None,
        }
    }

    /// Schema-v2 envelope for declarative Boolean safety jobs. Safety uses only
    /// model-space budgets and never manufactures product accounting.
    pub fn from_safety(
        model: impl Into<String>,
        model_limits: ExplorationLimits,
        result: &BoundedSafetyResult,
    ) -> Self {
        Self {
            schema_version: VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION,
            analysis: Some("safety".to_owned()),
            outcome: safety_outcome(&result.outcome),
            model: Some(model.into()),
            property: Some(result.expression.clone()),
            weak_fair_actions: Vec::new(),
            strong_fair_actions: Vec::new(),
            model_limits: model_limits.into(),
            product_limits: ExplorationLimits::unbounded().into(),
            accounting: model_only_accounting(
                result.discovered_states,
                result.checked_states,
                result.explored_transitions,
                result.max_depth_reached,
            ),
            cutoff: result
                .outcome
                .inconclusive_reason()
                .map(|reason| cutoff(VerificationJobCutoffStage::Model, reason)),
            evidence: result
                .counterexample
                .as_ref()
                .map(|trace| VerificationJobEvidence::Safety {
                    trace: trace.iter().map(convert_state_step).collect(),
                }),
            error: None,
        }
    }

    /// Schema-v2 envelope for the existing bounded exact-state frontend.
    /// Exact-state jobs use model-space budgets only and preserve the backend's
    /// positive reachability witness or finite/lasso eventuality counterexample.
    pub fn from_exact_state(
        model: impl Into<String>,
        property: impl Into<String>,
        model_limits: ExplorationLimits,
        result: &BoundedExactStateResult,
    ) -> Self {
        Self {
            schema_version: VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION,
            analysis: Some("exact-state".to_owned()),
            outcome: exact_state_outcome(&result.outcome),
            model: Some(model.into()),
            property: Some(property.into()),
            weak_fair_actions: Vec::new(),
            strong_fair_actions: Vec::new(),
            model_limits: model_limits.into(),
            product_limits: ExplorationLimits::unbounded().into(),
            accounting: model_only_accounting(
                result.discovered_states,
                result.checked_states,
                result.explored_transitions,
                result.max_depth_reached,
            ),
            cutoff: result
                .outcome
                .inconclusive_reason()
                .map(|reason| cutoff(VerificationJobCutoffStage::Model, reason)),
            evidence: result.evidence.as_ref().map(convert_exact_state_evidence),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            analysis: None,
            outcome: VerificationJobOutcome::Error,
            model: None,
            property: None,
            weak_fair_actions: Vec::new(),
            strong_fair_actions: Vec::new(),
            model_limits: ExplorationLimits::unbounded().into(),
            product_limits: ExplorationLimits::unbounded().into(),
            accounting: VerificationJobAccounting::empty(),
            cutoff: None,
            evidence: None,
            error: Some(message.into()),
        }
    }

    pub fn safety_error(message: impl Into<String>) -> Self {
        let mut envelope = Self::error(message);
        envelope.schema_version = VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION;
        envelope.analysis = Some("safety".to_owned());
        envelope
    }

    pub fn exact_state_error(message: impl Into<String>) -> Self {
        let mut envelope = Self::error(message);
        envelope.schema_version = VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION;
        envelope.analysis = Some("exact-state".to_owned());
        envelope
    }

    /// Deterministic compact JSON for machine consumers.
    ///
    /// M56 schema-v1 field order is preserved exactly when `analysis` is absent.
    /// Heterogeneous schema-v2 envelopes insert `analysis` immediately after the
    /// schema version and use family-specific evidence below.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u64(&mut out, "schema_version", self.schema_version as u64, true);
        if let Some(analysis) = self.analysis.as_deref() {
            field_string(&mut out, "analysis", analysis, false);
        }
        field_string(&mut out, "outcome", self.outcome.as_str(), false);
        field_optional_string(
            &mut out,
            "status",
            canonical_status(self.outcome, self.analysis.as_deref()),
            false,
        );
        field_optional_string(&mut out, "model", self.model.as_deref(), false);
        field_optional_string(&mut out, "property", self.property.as_deref(), false);
        field_string_array(
            &mut out,
            "weak_fair_actions",
            &self.weak_fair_actions,
            false,
        );
        field_string_array(
            &mut out,
            "strong_fair_actions",
            &self.strong_fair_actions,
            false,
        );
        field_limits(&mut out, "model_limits", &self.model_limits, false);
        field_limits(&mut out, "product_limits", &self.product_limits, false);
        field_accounting(&mut out, &self.accounting);
        out.push_str(",\"cutoff\":");
        match self.cutoff {
            Some(value) => write_cutoff(&mut out, value),
            None => out.push_str("null"),
        }
        out.push_str(",\"evidence\":");
        match &self.evidence {
            Some(value) => write_evidence(&mut out, value),
            None => out.push_str("null"),
        }
        out.push_str(",\"error\":");
        write_optional_string(&mut out, self.error.as_deref());
        out.push('}');
        out
    }
}

fn model_only_accounting(
    discovered_states: usize,
    checked_states: usize,
    explored_transitions: usize,
    max_depth_reached: Option<usize>,
) -> VerificationJobAccounting {
    VerificationJobAccounting {
        model_states: Some(discovered_states),
        checked_model_states: Some(checked_states),
        explored_model_transitions: Some(explored_transitions),
        retained_model_transitions: Some(explored_transitions),
        max_model_depth_reached: max_depth_reached,
        product_states: None,
        checked_product_states: None,
        explored_product_transitions: None,
        retained_product_transitions: None,
        max_product_depth_reached: None,
    }
}

fn canonical_status(
    outcome: VerificationJobOutcome,
    analysis: Option<&str>,
) -> Option<&'static str> {
    if analysis == Some("safety") {
        return match outcome {
            VerificationJobOutcome::Satisfied => Some("SAFE"),
            VerificationJobOutcome::Violated => Some("VIOLATED"),
            VerificationJobOutcome::Inconclusive => Some("INCONCLUSIVE"),
            VerificationJobOutcome::Error => None,
        };
    }
    match outcome {
        VerificationJobOutcome::Satisfied => Some("SATISFIED"),
        VerificationJobOutcome::Violated => Some("VIOLATED"),
        VerificationJobOutcome::Inconclusive => Some("INCONCLUSIVE"),
        VerificationJobOutcome::Error => None,
    }
}

fn status_outcome(status: MultiResponseStatus) -> VerificationJobOutcome {
    match status {
        MultiResponseStatus::Satisfied => VerificationJobOutcome::Satisfied,
        MultiResponseStatus::Violated => VerificationJobOutcome::Violated,
    }
}

fn bounded_outcome(outcome: &BoundedOutcome<MultiResponseStatus>) -> VerificationJobOutcome {
    match outcome {
        BoundedOutcome::Conclusive(status) => status_outcome(*status),
        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn analysis_outcome(outcome: &AnalysisOutcome<MultiResponseStatus>) -> VerificationJobOutcome {
    match outcome {
        AnalysisOutcome::Conclusive(status) => status_outcome(*status),
        AnalysisOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn safety_outcome(outcome: &BoundedOutcome<SafetyStatus>) -> VerificationJobOutcome {
    match outcome {
        BoundedOutcome::Conclusive(SafetyStatus::Safe) => VerificationJobOutcome::Satisfied,
        BoundedOutcome::Conclusive(SafetyStatus::Violated) => VerificationJobOutcome::Violated,
        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn exact_state_outcome(outcome: &BoundedOutcome<ExactStateStatus>) -> VerificationJobOutcome {
    match outcome {
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied) => {
            VerificationJobOutcome::Satisfied
        }
        BoundedOutcome::Conclusive(ExactStateStatus::Violated) => VerificationJobOutcome::Violated,
        BoundedOutcome::Inconclusive(_) => VerificationJobOutcome::Inconclusive,
    }
}

fn cutoff(stage: VerificationJobCutoffStage, reason: InconclusiveReason) -> VerificationJobCutoff {
    match reason {
        InconclusiveReason::StateLimitReached { limit } => VerificationJobCutoff {
            stage,
            kind: VerificationJobCutoffKind::StateLimit,
            limit,
        },
        InconclusiveReason::TransitionLimitReached { limit } => VerificationJobCutoff {
            stage,
            kind: VerificationJobCutoffKind::TransitionLimit,
            limit,
        },
        InconclusiveReason::DepthLimitReached { limit } => VerificationJobCutoff {
            stage,
            kind: VerificationJobCutoffKind::DepthLimit,
            limit,
        },
    }
}

fn convert_evidence(
    counterexample: &MultiResponseCounterexample<String>,
) -> VerificationJobEvidence {
    match counterexample {
        MultiResponseCounterexample::Finite { clause, trace } => VerificationJobEvidence::Finite {
            clause: clause.clone(),
            trace: trace.iter().map(convert_step).collect(),
        },
        MultiResponseCounterexample::Infinite {
            clause,
            stem,
            cycle,
        } => VerificationJobEvidence::Infinite {
            clause: clause.clone(),
            stem: stem.iter().map(convert_step).collect(),
            cycle: cycle.iter().map(convert_step).collect(),
        },
    }
}

fn convert_step(step: &TraceStep<MultiObligationState<String>>) -> VerificationJobTraceStep {
    VerificationJobTraceStep {
        action: step.action.clone(),
        state: step.state.state.clone(),
        pending: step.state.pending.clone(),
    }
}

fn convert_state_step(step: &TraceStep<String>) -> VerificationJobStateTraceStep {
    VerificationJobStateTraceStep {
        action: step.action.clone(),
        state: step.state.clone(),
    }
}

fn convert_exact_state_evidence(evidence: &ExactStateEvidence) -> VerificationJobEvidence {
    match evidence {
        ExactStateEvidence::ReachabilityWitness { trace } => {
            VerificationJobEvidence::ExactStateReachability {
                trace: trace.iter().map(convert_state_step).collect(),
            }
        }
        ExactStateEvidence::EventualityFiniteCounterexample { trace } => {
            VerificationJobEvidence::ExactStateEventualityFinite {
                trace: trace.iter().map(convert_state_step).collect(),
            }
        }
        ExactStateEvidence::EventualityInfiniteCounterexample { stem, cycle } => {
            VerificationJobEvidence::ExactStateEventualityInfinite {
                stem: stem.iter().map(convert_state_step).collect(),
                cycle: cycle.iter().map(convert_state_step).collect(),
            }
        }
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

fn field_string_array(out: &mut String, name: &str, values: &[String], first: bool) {
    field_name(out, name, first);
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        write_json_string(out, value);
    }
    out.push(']');
}

fn field_limits(out: &mut String, name: &str, limits: &VerificationJobLimits, first: bool) {
    field_name(out, name, first);
    out.push('{');
    optional_usize_field(out, "max_states", limits.max_states, true);
    optional_usize_field(out, "max_transitions", limits.max_transitions, false);
    optional_usize_field(out, "max_depth", limits.max_depth, false);
    out.push('}');
}

fn field_accounting(out: &mut String, value: &VerificationJobAccounting) {
    out.push_str(",\"accounting\":{");
    optional_usize_field(out, "model_states", value.model_states, true);
    optional_usize_field(
        out,
        "checked_model_states",
        value.checked_model_states,
        false,
    );
    optional_usize_field(
        out,
        "explored_model_transitions",
        value.explored_model_transitions,
        false,
    );
    optional_usize_field(
        out,
        "retained_model_transitions",
        value.retained_model_transitions,
        false,
    );
    optional_usize_field(
        out,
        "max_model_depth_reached",
        value.max_model_depth_reached,
        false,
    );
    optional_usize_field(out, "product_states", value.product_states, false);
    optional_usize_field(
        out,
        "checked_product_states",
        value.checked_product_states,
        false,
    );
    optional_usize_field(
        out,
        "explored_product_transitions",
        value.explored_product_transitions,
        false,
    );
    optional_usize_field(
        out,
        "retained_product_transitions",
        value.retained_product_transitions,
        false,
    );
    optional_usize_field(
        out,
        "max_product_depth_reached",
        value.max_product_depth_reached,
        false,
    );
    out.push('}');
}

fn optional_usize_field(out: &mut String, name: &str, value: Option<usize>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

fn write_optional_string(out: &mut String, value: Option<&str>) {
    match value {
        Some(value) => write_json_string(out, value),
        None => out.push_str("null"),
    }
}

fn write_cutoff(out: &mut String, value: VerificationJobCutoff) {
    out.push('{');
    field_string(out, "stage", value.stage.as_str(), true);
    field_string(out, "kind", value.kind.as_str(), false);
    field_u64(out, "limit", value.limit as u64, false);
    out.push('}');
}

fn write_evidence(out: &mut String, value: &VerificationJobEvidence) {
    match value {
        VerificationJobEvidence::Finite { clause, trace } => {
            out.push('{');
            field_string(out, "kind", "finite", true);
            field_string(out, "clause", clause, false);
            out.push_str(",\"trace\":");
            write_trace(out, trace);
            out.push('}');
        }
        VerificationJobEvidence::Infinite {
            clause,
            stem,
            cycle,
        } => {
            out.push('{');
            field_string(out, "kind", "lasso", true);
            field_string(out, "clause", clause, false);
            out.push_str(",\"stem\":");
            write_trace(out, stem);
            out.push_str(",\"cycle\":");
            write_trace(out, cycle);
            out.push('}');
        }
        VerificationJobEvidence::Safety { trace } => {
            out.push('{');
            field_string(out, "kind", "safety", true);
            out.push_str(",\"trace\":");
            write_state_trace(out, trace);
            out.push('}');
        }
        VerificationJobEvidence::ExactStateReachability { trace } => {
            out.push('{');
            field_string(out, "kind", "reachability_witness", true);
            out.push_str(",\"trace\":");
            write_state_trace(out, trace);
            out.push('}');
        }
        VerificationJobEvidence::ExactStateEventualityFinite { trace } => {
            out.push('{');
            field_string(out, "kind", "eventuality_finite", true);
            out.push_str(",\"trace\":");
            write_state_trace(out, trace);
            out.push('}');
        }
        VerificationJobEvidence::ExactStateEventualityInfinite { stem, cycle } => {
            out.push('{');
            field_string(out, "kind", "eventuality_lasso", true);
            out.push_str(",\"stem\":");
            write_state_trace(out, stem);
            out.push_str(",\"cycle\":");
            write_state_trace(out, cycle);
            out.push('}');
        }
    }
}

fn write_trace(out: &mut String, trace: &[VerificationJobTraceStep]) {
    out.push('[');
    for (index, step) in trace.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push('{');
        field_name(out, "action", true);
        write_optional_string(out, step.action.as_deref());
        field_string(out, "state", &step.state, false);
        out.push_str(",\"pending\":[");
        for (pending_index, pending) in step.pending.iter().enumerate() {
            if pending_index != 0 {
                out.push(',');
            }
            out.push_str(if *pending { "true" } else { "false" });
        }
        out.push_str("]}");
    }
    out.push(']');
}

fn write_state_trace(out: &mut String, trace: &[VerificationJobStateTraceStep]) {
    out.push('[');
    for (index, step) in trace.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        out.push('{');
        field_name(out, "action", true);
        write_optional_string(out, step.action.as_deref());
        field_string(out, "state", &step.state, false);
        out.push('}');
    }
    out.push(']');
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
            ch if ch <= '\u{1f}' => {
                let code = ch as u32;
                out.push_str("\\u00");
                out.push(hex_digit((code >> 4) as u8));
                out.push(hex_digit((code & 0x0f) as u8));
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("hex nibble is always in range"),
    }
}

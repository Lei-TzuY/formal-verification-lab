//! Educational formal-methods laboratory.
//!
//! The crate provides a small explicit-state formal-verification core built
//! from first principles. The semantic core is intentionally independent from
//! the CLI so it can be reused by tests and future front ends.

pub mod bounded;
pub mod bounded_fairness;
mod bounded_report;
pub mod buchi;
pub mod buchi_examples;
pub mod buchi_report;
pub mod builder;
pub mod checker;
pub mod declarative;
pub mod eventuality;
pub mod eventuality_report;
pub mod exact_state;
pub mod exact_state_report;
pub mod examples;
pub mod fairness;
pub mod fairness_report;
mod graph;
pub mod model;
pub mod monitor;
pub mod monitor_examples;
pub mod monitor_report;
pub mod multi_response;
pub mod multi_response_examples;
pub mod multi_response_report;
mod product;
pub mod property;
pub mod proposition;
pub mod proposition_expr;
pub mod proposition_expr_report;
pub mod proposition_report;
pub mod recurrence;
pub mod reduction;
pub mod report;
pub mod response;
pub mod response_examples;
pub mod response_report;
pub mod safety;
pub mod safety_report;
pub mod temporal;
pub mod temporal_parse;
pub mod temporal_report;

pub use bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
pub use bounded_fairness::{
    check_buchi_with_weak_fairness_and_limits, check_buchi_with_weak_fairness_and_product_limits,
};
pub use buchi::{
    check_buchi, check_buchi_with_limits, AcceptanceSet, AnalysisBuchiResult, BuchiAutomaton,
    BuchiCounterexample, BuchiError, BuchiProductState, BuchiResult, BuchiStatus, FiniteRunPolicy,
};
pub use buchi_report::render_buchi_report;
pub use builder::TransitionSystemBuilder;
pub use checker::{
    check, check_with_limits, CheckResult, Counterexample, ExplorationLimits, InconclusiveReason,
    TraceStep, VerificationStatus,
};
pub use declarative::{
    parse_declarative_document, parse_declarative_model, DeclarativeDocument, DeclarativeModelError,
};
pub use eventuality::{
    check_eventuality, check_eventuality_with_limits, BoundedEventualityResult,
    EventualityCounterexample, EventualityError, EventualityProperty, EventualityResult,
    EventualityStatus,
};
pub use eventuality_report::render_eventuality_report;
pub use exact_state::{
    check_exact_state_property, check_exact_state_property_with_limits, parse_exact_state_property,
    BoundedExactStateResult, ExactStateBackend, ExactStateError, ExactStateEvidence,
    ExactStateParseError, ExactStateParseErrorKind, ExactStatePropertySpec, ExactStateResult,
    ExactStateStatus,
};
pub use exact_state_report::render_exact_state_report;
pub use fairness::{check_buchi_with_weak_fairness, FairnessError, WeakFairness};
pub use fairness_report::render_weak_fair_temporal_report;
pub use model::{Invariant, ModelError, StateVariable, Transition, TransitionSystem};
pub use monitor::{
    check_monitor, check_monitor_with_limits, check_monitor_with_product_limits,
    AnalysisMonitorResult, BoundedMonitorResult, FiniteMonitor, MonitorCounterexample,
    MonitorError, MonitorProductState, MonitorResult, MonitorStatus, ProgressCondition,
    RejectCondition,
};
pub use monitor_report::{render_bounded_monitor_report, render_monitor_report};
pub use multi_response::{
    check_multi_response, check_multi_response_with_limits,
    check_multi_response_with_product_limits, check_multi_response_with_weak_fairness,
    check_multi_response_with_weak_fairness_and_limits,
    check_multi_response_with_weak_fairness_and_product_limits, AnalysisMultiResponseResult,
    BoundedMultiResponseResult, MultiObligationState, MultiResponseCounterexample,
    MultiResponseError, MultiResponseProperty, MultiResponseResult, MultiResponseStatus,
    ResponseClause,
};
pub use multi_response_report::render_multi_response_report;
pub use property::{
    check_deadlock, check_reachability, check_reachability_with_limits, BoundedReachabilityResult,
    DeadlockError, DeadlockProperty, DeadlockResult, DeadlockStatus, ReachabilityError,
    ReachabilityProperty, ReachabilityResult, ReachabilityStatus,
};
pub use proposition::{
    check_proposition_property, check_proposition_property_with_limits, BoundedPropositionResult,
    PropositionError, PropositionPropertySpec, PropositionResult,
};
pub use proposition_expr::{
    check_proposition_expression_property, check_proposition_expression_property_with_limits,
    parse_proposition_expression, BoundedPropositionExpressionResult, PropositionExpression,
    PropositionExpressionError, PropositionExpressionParseError,
    PropositionExpressionParseErrorKind, PropositionExpressionPropertySpec,
    PropositionExpressionResult,
};
pub use proposition_expr_report::render_proposition_expression_report;
pub use proposition_report::render_proposition_report;
pub use recurrence::{
    analyze_recurrence, CycleWitness, RecurrenceAnalysis, RecurrenceError,
    StronglyConnectedComponent,
};
pub use reduction::{
    audit_sleep_set_reduction, IndependenceError, IndependenceRelation, ReducedExploration,
    ReductionAudit, ReductionAuditError,
};
pub use response::{
    check_response, check_response_with_limits, check_response_with_product_limits,
    check_response_with_weak_fairness, check_response_with_weak_fairness_and_limits,
    check_response_with_weak_fairness_and_product_limits, AnalysisResponseResult,
    BoundedResponseResult, ObligationState, ResponseCounterexample, ResponseError,
    ResponseProperty, ResponseResult, ResponseStatus,
};
pub use response_report::render_response_report;
pub use safety::{
    check_safety_assertion, check_safety_assertion_with_limits, BoundedSafetyResult,
    PropositionSafetySpec, SafetyError, SafetyResult, SafetyStatus,
};
pub use safety_report::render_safety_report;
pub use temporal::{
    check_action_temporal, check_action_temporal_with_limits,
    check_action_temporal_with_product_limits, check_action_temporal_with_weak_fairness,
    check_action_temporal_with_weak_fairness_and_limits,
    check_action_temporal_with_weak_fairness_and_product_limits, ActionAtom, ActionTemporalSpec,
    AnalysisTemporalResult, BoundedTemporalResult, TemporalBackend, TemporalCounterexample,
    TemporalError, TemporalObligation, TemporalResult, TemporalStatus,
};
pub use temporal_parse::{parse_action_temporal, TemporalParseError, TemporalParseErrorKind};
pub use temporal_report::{render_bounded_temporal_report, render_temporal_report};

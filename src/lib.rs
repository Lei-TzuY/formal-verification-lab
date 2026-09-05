//! Educational formal-methods laboratory.
//!
//! The crate provides a small explicit-state formal-verification core built
//! from first principles. The semantic core is intentionally independent from
//! the CLI so it can be reused by tests and future front ends.

pub mod buchi;
pub mod buchi_examples;
pub mod buchi_report;
pub mod builder;
pub mod checker;
pub mod declarative;
pub mod eventuality;
pub mod eventuality_report;
pub mod examples;
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
pub mod recurrence;
pub mod reduction;
pub mod report;
pub mod response;
pub mod response_examples;
pub mod response_report;
pub mod temporal;
pub mod temporal_parse;
pub mod temporal_report;

pub use buchi::{
    check_buchi, AcceptanceSet, BuchiAutomaton, BuchiCounterexample, BuchiError, BuchiProductState,
    BuchiResult, BuchiStatus, FiniteRunPolicy,
};
pub use buchi_report::render_buchi_report;
pub use builder::TransitionSystemBuilder;
pub use checker::{
    check, check_with_limits, CheckResult, Counterexample, ExplorationLimits, InconclusiveReason,
    TraceStep, VerificationStatus,
};
pub use declarative::{parse_declarative_model, DeclarativeModelError};
pub use eventuality::{
    check_eventuality, EventualityCounterexample, EventualityError, EventualityProperty,
    EventualityResult, EventualityStatus,
};
pub use eventuality_report::render_eventuality_report;
pub use model::{Invariant, ModelError, StateVariable, Transition, TransitionSystem};
pub use monitor::{
    check_monitor, FiniteMonitor, MonitorCounterexample, MonitorError, MonitorProductState,
    MonitorResult, MonitorStatus, ProgressCondition, RejectCondition,
};
pub use monitor_report::render_monitor_report;
pub use multi_response::{
    check_multi_response, MultiObligationState, MultiResponseCounterexample, MultiResponseError,
    MultiResponseProperty, MultiResponseResult, MultiResponseStatus, ResponseClause,
};
pub use multi_response_report::render_multi_response_report;
pub use property::{
    check_deadlock, check_reachability, DeadlockError, DeadlockProperty, DeadlockResult,
    DeadlockStatus, ReachabilityError, ReachabilityProperty, ReachabilityResult,
    ReachabilityStatus,
};
pub use recurrence::{
    analyze_recurrence, CycleWitness, RecurrenceAnalysis, RecurrenceError,
    StronglyConnectedComponent,
};
pub use reduction::{
    audit_sleep_set_reduction, IndependenceError, IndependenceRelation, ReducedExploration,
    ReductionAudit, ReductionAuditError,
};
pub use response::{
    check_response, ObligationState, ResponseCounterexample, ResponseError, ResponseProperty,
    ResponseResult, ResponseStatus,
};
pub use response_report::render_response_report;
pub use temporal::{
    check_action_temporal, ActionAtom, ActionTemporalSpec, TemporalBackend, TemporalCounterexample,
    TemporalError, TemporalObligation, TemporalResult, TemporalStatus,
};
pub use temporal_parse::{parse_action_temporal, TemporalParseError, TemporalParseErrorKind};
pub use temporal_report::render_temporal_report;

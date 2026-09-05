//! Educational formal-methods laboratory.
//!
//! The crate provides a small explicit-state safety model checker built from
//! first principles. The semantic core is intentionally independent from the
//! CLI so it can be reused by tests and future front ends.

pub mod builder;
pub mod checker;
pub mod examples;
pub mod model;
pub mod property;
pub mod reduction;
pub mod report;

pub use builder::TransitionSystemBuilder;
pub use checker::{
    check, check_with_limits, CheckResult, Counterexample, ExplorationLimits, InconclusiveReason,
    TraceStep, VerificationStatus,
};
pub use model::{Invariant, ModelError, StateVariable, Transition, TransitionSystem};
pub use property::{
    check_reachability, ReachabilityError, ReachabilityProperty, ReachabilityResult,
    ReachabilityStatus,
};
pub use reduction::{
    audit_sleep_set_reduction, IndependenceError, IndependenceRelation, ReducedExploration,
    ReductionAudit, ReductionAuditError,
};

//! Educational formal-methods laboratory.
//!
//! The crate provides a small explicit-state formal-verification core built
//! from first principles. The semantic core is intentionally independent from
//! the CLI so it can be reused by tests and future front ends.

pub mod bounded;
pub mod bounded_combined_fairness;
pub mod bounded_fairness;
mod bounded_report;
pub mod bounded_strong_fairness;
pub mod buchi;
pub mod buchi_examples;
pub mod buchi_report;
pub mod builder;
pub mod checker;
pub mod combined_fairness;
pub mod ctl;
pub mod ctl_bounded;
pub mod ctl_parse;
pub mod declarative;
pub mod declarative_ctl;
pub mod declarative_ctl_report;
pub mod declarative_deadlock;
pub mod declarative_deadlock_report;
pub mod declarative_mu;
pub mod declarative_mu_report;
pub mod eventuality;
pub mod eventuality_report;
pub mod exact_state;
pub mod exact_state_report;
pub mod examples;
mod fair_enablement;
pub mod fairness;
pub mod fairness_report;
mod graph;
pub mod model;
pub mod monitor;
pub mod monitor_combined_fairness;
pub mod monitor_examples;
pub mod monitor_fairness;
pub mod monitor_report;
pub mod monitor_strong_fairness;
pub mod mu_bounded;
pub mod mu_calculus;
pub mod mu_parity;
pub mod mu_parse;
pub mod multi_response;
pub mod multi_response_examples;
pub mod multi_response_report;
pub mod multi_temporal;
pub mod parity_game;
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
pub mod strong_fairness;
pub mod structural_job;
pub mod structural_job_run;
pub mod structural_result;
pub mod structural_suite;
pub mod structural_suite_run;
pub mod temporal;
pub mod temporal_parse;
pub mod temporal_report;
mod verification_action_temporal;
mod verification_ctl;
pub mod verification_execution;
pub mod verification_job;
pub mod verification_job_run;
mod verification_mu;
pub mod verification_result;
pub mod verification_suite;
pub mod verification_suite_run;

pub use bounded::{
    AnalysisInconclusiveReason, AnalysisLimits, AnalysisOutcome, AnalysisStage, BoundedOutcome,
};
pub use bounded_combined_fairness::{
    check_buchi_with_fairness_profile_and_limits,
    check_buchi_with_fairness_profile_and_product_limits,
};
pub use bounded_fairness::{
    check_buchi_with_weak_fairness_and_limits, check_buchi_with_weak_fairness_and_product_limits,
};
pub use bounded_strong_fairness::{
    check_buchi_with_strong_fairness_and_limits,
    check_buchi_with_strong_fairness_and_product_limits,
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
pub use combined_fairness::{
    check_buchi_with_fairness_profile, FairnessProfile, FairnessProfileError,
};
pub use ctl::{
    evaluate_ctl, CtlError, CtlEvaluation, CtlEvidence, CtlEvidenceAction, CtlEvidenceStep,
    CtlFormula, CtlInitialEvaluation, CtlTerminalPolicy,
};
pub use ctl_bounded::{
    evaluate_ctl_with_limits, BoundedCtlError, BoundedCtlEvaluation, BoundedCtlInitialEvaluation,
    BoundedCtlStatus, BoundedCtlTruth,
};
pub use ctl_parse::{parse_ctl_formula, render_ctl_formula, CtlParseError, CtlParseErrorKind};
pub use declarative::{
    parse_declarative_document, parse_declarative_model, DeclarativeDocument, DeclarativeModelError,
};
pub use declarative_ctl::{
    check_declarative_ctl, check_declarative_ctl_text, check_declarative_ctl_text_with_limits,
    check_declarative_ctl_with_limits, BoundedDeclarativeCtlResult, DeclarativeCtlError,
    DeclarativeCtlResult, DeclarativeCtlStatus,
};
pub use declarative_ctl_report::{
    render_bounded_declarative_ctl_report, render_declarative_ctl_report,
};
pub use declarative_deadlock::{
    check_declarative_deadlock_with_limits, parse_declarative_deadlock_spec,
    BoundedDeclarativeDeadlockResult, DeclarativeDeadlockError, DeclarativeDeadlockSpec,
};
pub use declarative_deadlock_report::render_declarative_deadlock_report;
pub use declarative_mu::{
    check_declarative_mu, check_declarative_mu_text, check_declarative_mu_text_with_limits,
    check_declarative_mu_with_limits, BoundedDeclarativeMuResult, DeclarativeMuError,
    DeclarativeMuResult, DeclarativeMuStatus,
};
pub use declarative_mu_report::{
    render_bounded_declarative_mu_report, render_declarative_mu_report,
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
pub use monitor_combined_fairness::{
    check_monitor_with_fairness_profile, check_monitor_with_fairness_profile_and_limits,
    check_monitor_with_fairness_profile_and_product_limits,
};
pub use monitor_fairness::{
    check_monitor_with_weak_fairness, check_monitor_with_weak_fairness_and_limits,
    check_monitor_with_weak_fairness_and_product_limits,
};
pub use monitor_report::{render_bounded_monitor_report, render_monitor_report};
pub use monitor_strong_fairness::{
    check_monitor_with_strong_fairness, check_monitor_with_strong_fairness_and_limits,
    check_monitor_with_strong_fairness_and_product_limits,
};
pub use mu_bounded::{
    evaluate_mu_with_limits, BoundedMuError, BoundedMuEvaluation, BoundedMuInitialEvaluation,
    BoundedMuStatus, BoundedMuTruth,
};
pub use mu_calculus::{
    compile_ctl_to_mu, evaluate_mu, validate_mu_formula, MuError, MuEvaluation, MuFormula,
    MuInitialEvaluation, MuTerminalPolicy, MuValidationError,
};
pub use mu_parity::{evaluate_mu_via_parity, MuParityError, MuParityEvaluation};
pub use mu_parse::{parse_mu_formula, render_mu_formula, MuParseError, MuParseErrorKind};
pub use multi_response::{
    check_multi_response, check_multi_response_with_limits,
    check_multi_response_with_product_limits, check_multi_response_with_strong_fairness,
    check_multi_response_with_strong_fairness_and_limits,
    check_multi_response_with_strong_fairness_and_product_limits,
    check_multi_response_with_weak_fairness, check_multi_response_with_weak_fairness_and_limits,
    check_multi_response_with_weak_fairness_and_product_limits, AnalysisMultiResponseResult,
    BoundedMultiResponseResult, MultiObligationState, MultiResponseCounterexample,
    MultiResponseError, MultiResponseProperty, MultiResponseResult, MultiResponseStatus,
    ResponseClause,
};
pub use multi_response_report::render_multi_response_report;
pub use multi_temporal::{
    check_multi_response_temporal, check_multi_response_temporal_with_fairness_profile,
    check_multi_response_temporal_with_fairness_profile_and_limits,
    check_multi_response_temporal_with_fairness_profile_and_product_limits,
    check_multi_response_temporal_with_limits, check_multi_response_temporal_with_product_limits,
    parse_multi_response_temporal, ExactResponseClause, MultiResponseTemporalParseError,
    MultiResponseTemporalParseErrorKind, MultiResponseTemporalSpec, MultiResponseTemporalSpecError,
    ResponseActionRole,
};
pub use parity_game::{
    solve_parity_game, ParityGame, ParityGameError, ParityPlayer, ParitySolution,
};
pub use property::{
    check_deadlock, check_deadlock_with_limits, check_reachability, check_reachability_with_limits,
    BoundedDeadlockResult, BoundedReachabilityResult, DeadlockError, DeadlockProperty,
    DeadlockResult, DeadlockStatus, ReachabilityError, ReachabilityProperty, ReachabilityResult,
    ReachabilityStatus,
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
    analyze_recurrence, analyze_recurrence_with_limits, BoundedRecurrenceResult, CycleWitness,
    RecurrenceAnalysis, RecurrenceError, RecurrenceStatus, StronglyConnectedComponent,
};
pub use reduction::{
    audit_sleep_set_reduction, check_validated_sleep_set_reduction, validate_independence,
    IndependenceError, IndependenceRelation, IndependenceValidationError, ReducedExploration,
    ReductionAudit, ReductionAuditError, ValidatedIndependenceRelation,
};
pub use response::{
    check_response, check_response_with_fairness_profile,
    check_response_with_fairness_profile_and_limits,
    check_response_with_fairness_profile_and_product_limits, check_response_with_limits,
    check_response_with_product_limits, check_response_with_strong_fairness,
    check_response_with_strong_fairness_and_limits,
    check_response_with_strong_fairness_and_product_limits, check_response_with_weak_fairness,
    check_response_with_weak_fairness_and_limits,
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
pub use strong_fairness::{check_buchi_with_strong_fairness, StrongFairness, StrongFairnessError};
pub use structural_job::{
    parse_structural_job, StructuralJob, StructuralJobAnalysis, StructuralJobParseError,
    StructuralJobParseErrorKind,
};
pub use structural_job_run::{run_structural_job_json, StructuralJobJsonRun};
pub use structural_result::{
    StructuralJobAccounting, StructuralJobComponent, StructuralJobCutoff, StructuralJobCutoffKind,
    StructuralJobCycleEvidence, StructuralJobLimits, StructuralJobOutcome,
    StructuralJobResultEnvelope, StructuralJobTraceStep, STRUCTURAL_JOB_RESULT_SCHEMA_VERSION,
};
pub use structural_suite::{
    parse_structural_suite, StructuralExpectedOutcome, StructuralSuite, StructuralSuiteParseError,
    StructuralSuiteParseErrorKind, MAX_STRUCTURAL_SUITE_JOBS,
};
pub use structural_suite_run::{
    load_structural_suite, run_structural_suite_expectations_json, run_structural_suite_json,
    StructuralRegressionSuiteEntryResult, StructuralRegressionSuiteJsonRun,
    StructuralRegressionSuiteOutcome, StructuralRegressionSuiteResultEnvelope,
    StructuralSuiteEntryResult, StructuralSuiteJsonRun, StructuralSuiteLoadError,
    StructuralSuiteOutcome, StructuralSuiteResultEnvelope,
    STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE, STRUCTURAL_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
    STRUCTURAL_SUITE_RESULT_SCHEMA_VERSION,
};
pub use temporal::{
    check_action_temporal, check_action_temporal_with_fairness_profile,
    check_action_temporal_with_fairness_profile_and_limits,
    check_action_temporal_with_fairness_profile_and_product_limits,
    check_action_temporal_with_limits, check_action_temporal_with_product_limits,
    check_action_temporal_with_strong_fairness,
    check_action_temporal_with_strong_fairness_and_limits,
    check_action_temporal_with_strong_fairness_and_product_limits,
    check_action_temporal_with_weak_fairness, check_action_temporal_with_weak_fairness_and_limits,
    check_action_temporal_with_weak_fairness_and_product_limits, ActionAtom, ActionTemporalSpec,
    AnalysisTemporalResult, BoundedTemporalResult, TemporalBackend, TemporalCounterexample,
    TemporalError, TemporalObligation, TemporalResult, TemporalStatus,
};
pub use temporal_parse::{parse_action_temporal, TemporalParseError, TemporalParseErrorKind};
pub use temporal_report::{render_bounded_temporal_report, render_temporal_report};
pub use verification_execution::{
    execute_multi_response, MultiResponseExecutionConfig, MultiResponseExecutionResult,
};
pub use verification_job::{
    parse_verification_job, VerificationJob, VerificationJobAnalysis, VerificationJobParseError,
    VerificationJobParseErrorKind,
};
pub use verification_job_run::{
    load_verification_job, run_verification_job_json, LoadedVerificationJob,
    VerificationJobJsonRun, VerificationJobLoadError,
};
pub use verification_result::{
    VerificationJobAccounting, VerificationJobCtlAction, VerificationJobCtlDetails,
    VerificationJobCtlEvidence, VerificationJobCtlInitial, VerificationJobCtlTraceStep,
    VerificationJobCtlTruth, VerificationJobCutoff, VerificationJobCutoffKind,
    VerificationJobCutoffStage, VerificationJobEvidence, VerificationJobLimits,
    VerificationJobMuDetails, VerificationJobMuInitial, VerificationJobMuTruth,
    VerificationJobOutcome, VerificationJobResultEnvelope, VerificationJobSafetyTraceStep,
    VerificationJobStateTraceStep, VerificationJobTraceStep,
    VERIFICATION_JOB_CTL_RESULT_SCHEMA_VERSION, VERIFICATION_JOB_EXACT_STATE_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_HETEROGENEOUS_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_MU_RESULT_SCHEMA_VERSION, VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
    VERIFICATION_JOB_SAFETY_RESULT_SCHEMA_VERSION,
};
pub use verification_suite::{
    parse_verification_suite, VerificationExpectedOutcome, VerificationSuite,
    VerificationSuiteParseError, VerificationSuiteParseErrorKind, MAX_VERIFICATION_SUITE_JOBS,
};
pub use verification_suite_run::{
    load_verification_suite, run_verification_suite_expectations_json, run_verification_suite_json,
    VerificationRegressionSuiteEntryResult, VerificationRegressionSuiteJsonRun,
    VerificationRegressionSuiteOutcome, VerificationRegressionSuiteResultEnvelope,
    VerificationSuiteEntryResult, VerificationSuiteJsonRun, VerificationSuiteLoadError,
    VerificationSuiteOutcome, VerificationSuiteResultEnvelope,
    VERIFICATION_REGRESSION_MISMATCH_EXIT_CODE,
    VERIFICATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION, VERIFICATION_SUITE_RESULT_SCHEMA_VERSION,
};

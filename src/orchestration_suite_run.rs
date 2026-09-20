use crate::certificate_verification_job_run::{
    run_certificate_verification_job_json_with_provider, CertificateVerificationJobJsonRun,
};
use crate::certificate_verification_result::{
    CertificateVerificationJobOutcome, CertificateVerificationJobResultEnvelope,
};
use crate::orchestration_suite::{
    parse_orchestration_suite, OrchestrationExpectedOutcome, OrchestrationJobFamily,
    OrchestrationSuite,
};
use crate::structural_job_run::{run_structural_job_json_with_provider, StructuralJobJsonRun};
use crate::structural_result::{StructuralJobOutcome, StructuralJobResultEnvelope};
use crate::text_source::{
    path_source_id, resolve_source_id, FileSystemTextSourceProvider, TextSourceProvider,
};
use crate::verification_job_run::{
    run_verification_job_json_with_provider, VerificationJobJsonRun,
};
use crate::verification_result::{VerificationJobOutcome, VerificationJobResultEnvelope};
use std::fmt;
use std::path::Path;

pub const ORCHESTRATION_SUITE_RESULT_SCHEMA_VERSION: u32 = 1;
pub const ORCHESTRATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION: u32 = 1;
pub const ORCHESTRATION_REGRESSION_MISMATCH_EXIT_CODE: u8 = 13;
pub const ORCHESTRATION_ATTENTION_EXIT_CODE: u8 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestrationSuiteStatus {
    Complete,
    Attention,
    Error,
}

impl OrchestrationSuiteStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Attention => "attention",
            Self::Error => "error",
        }
    }

    fn exit_code(self) -> u8 {
        match self {
            Self::Complete => 0,
            Self::Attention => ORCHESTRATION_ATTENTION_EXIT_CODE,
            Self::Error => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestrationNestedResult {
    Verification(Box<VerificationJobResultEnvelope>),
    Structural(Box<StructuralJobResultEnvelope>),
    CertificateVerification(Box<CertificateVerificationJobResultEnvelope>),
}

impl OrchestrationNestedResult {
    pub fn family(&self) -> OrchestrationJobFamily {
        match self {
            Self::Verification(_) => OrchestrationJobFamily::Verification,
            Self::Structural(_) => OrchestrationJobFamily::Structural,
            Self::CertificateVerification(_) => OrchestrationJobFamily::CertificateVerification,
        }
    }

    pub fn outcome_str(&self) -> &'static str {
        match self {
            Self::Verification(result) => verification_outcome_str(result.outcome),
            Self::Structural(result) => result.outcome.as_str(),
            Self::CertificateVerification(result) => result.outcome.as_str(),
        }
    }

    fn is_error(&self) -> bool {
        match self {
            Self::Verification(result) => result.outcome == VerificationJobOutcome::Error,
            Self::Structural(result) => result.outcome == StructuralJobOutcome::Error,
            Self::CertificateVerification(result) => {
                result.outcome == CertificateVerificationJobOutcome::Error
            }
        }
    }

    fn is_rejected(&self) -> bool {
        matches!(
            self,
            Self::CertificateVerification(result)
                if result.outcome == CertificateVerificationJobOutcome::Rejected
        )
    }

    fn to_json(&self) -> String {
        match self {
            Self::Verification(result) => result.to_json(),
            Self::Structural(result) => result.to_json(),
            Self::CertificateVerification(result) => result.to_json(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuiteEntryResult {
    pub family: OrchestrationJobFamily,
    pub manifest: String,
    pub job_exit_code: u8,
    pub result: OrchestrationNestedResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuiteResultEnvelope {
    pub schema_version: u32,
    pub status: OrchestrationSuiteStatus,
    pub suite: Option<String>,
    pub jobs: Vec<OrchestrationSuiteEntryResult>,
    pub error: Option<String>,
}

impl OrchestrationSuiteResultEnvelope {
    fn from_suite(suite: &OrchestrationSuite, jobs: Vec<OrchestrationSuiteEntryResult>) -> Self {
        Self {
            schema_version: ORCHESTRATION_SUITE_RESULT_SCHEMA_VERSION,
            status: aggregate_status(&jobs),
            suite: Some(suite.name().to_owned()),
            jobs,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: ORCHESTRATION_SUITE_RESULT_SCHEMA_VERSION,
            status: OrchestrationSuiteStatus::Error,
            suite: None,
            jobs: Vec::new(),
            error: Some(message.into()),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u64(&mut out, "schema_version", self.schema_version as u64, true);
        field_string(&mut out, "status", self.status.as_str(), false);
        field_optional_string(&mut out, "suite", self.suite.as_deref(), false);
        out.push_str(",\"jobs\":[");
        for (index, job) in self.jobs.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push('{');
            field_string(&mut out, "family", job.family.as_str(), true);
            field_string(&mut out, "manifest", &job.manifest, false);
            field_u64(&mut out, "job_exit_code", job.job_exit_code as u64, false);
            out.push_str(",\"result\":");
            out.push_str(&job.result.to_json());
            out.push('}');
        }
        out.push(']');
        out.push_str(",\"error\":");
        write_optional_string(&mut out, self.error.as_deref());
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuiteJsonRun {
    pub envelope: OrchestrationSuiteResultEnvelope,
    pub exit_code: u8,
}

impl OrchestrationSuiteJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestrationRegressionOutcome {
    Matched,
    Mismatched,
    Error,
}

impl OrchestrationRegressionOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationRegressionEntryResult {
    pub family: OrchestrationJobFamily,
    pub manifest: String,
    pub expected: OrchestrationExpectedOutcome,
    pub observed: String,
    pub matched: bool,
    pub job_exit_code: u8,
    pub result: OrchestrationNestedResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationRegressionSuiteResultEnvelope {
    pub schema_version: u32,
    pub outcome: OrchestrationRegressionOutcome,
    pub execution_status: OrchestrationSuiteStatus,
    pub suite: Option<String>,
    pub jobs: Vec<OrchestrationRegressionEntryResult>,
    pub error: Option<String>,
}

impl OrchestrationRegressionSuiteResultEnvelope {
    fn from_suite(
        suite: &OrchestrationSuite,
        jobs: Vec<OrchestrationRegressionEntryResult>,
    ) -> Self {
        let execution_status = aggregate_regression_status(&jobs);
        let outcome = if jobs.iter().all(|job| job.matched) {
            OrchestrationRegressionOutcome::Matched
        } else {
            OrchestrationRegressionOutcome::Mismatched
        };
        Self {
            schema_version: ORCHESTRATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
            outcome,
            execution_status,
            suite: Some(suite.name().to_owned()),
            jobs,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: ORCHESTRATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
            outcome: OrchestrationRegressionOutcome::Error,
            execution_status: OrchestrationSuiteStatus::Error,
            suite: None,
            jobs: Vec::new(),
            error: Some(message.into()),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u64(&mut out, "schema_version", self.schema_version as u64, true);
        field_string(&mut out, "outcome", self.outcome.as_str(), false);
        field_string(
            &mut out,
            "execution_status",
            self.execution_status.as_str(),
            false,
        );
        field_optional_string(&mut out, "suite", self.suite.as_deref(), false);
        out.push_str(",\"jobs\":[");
        for (index, job) in self.jobs.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push('{');
            field_string(&mut out, "family", job.family.as_str(), true);
            field_string(&mut out, "manifest", &job.manifest, false);
            field_string(&mut out, "expected", job.expected.as_str(), false);
            field_string(&mut out, "observed", &job.observed, false);
            field_bool(&mut out, "matched", job.matched, false);
            field_u64(&mut out, "job_exit_code", job.job_exit_code as u64, false);
            out.push_str(",\"result\":");
            out.push_str(&job.result.to_json());
            out.push('}');
        }
        out.push(']');
        out.push_str(",\"error\":");
        write_optional_string(&mut out, self.error.as_deref());
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationRegressionSuiteJsonRun {
    pub envelope: OrchestrationRegressionSuiteResultEnvelope,
    pub exit_code: u8,
}

impl OrchestrationRegressionSuiteJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrchestrationSuiteLoadError {
    message: String,
}

impl OrchestrationSuiteLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for OrchestrationSuiteLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for OrchestrationSuiteLoadError {}

pub fn load_orchestration_suite(
    manifest_path: impl AsRef<Path>,
) -> Result<OrchestrationSuite, OrchestrationSuiteLoadError> {
    let manifest_source_id = path_source_id(manifest_path.as_ref())
        .map_err(|error| OrchestrationSuiteLoadError::new(error.to_string()))?;
    load_orchestration_suite_with_provider(&FileSystemTextSourceProvider, &manifest_source_id)
}

pub fn load_orchestration_suite_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> Result<OrchestrationSuite, OrchestrationSuiteLoadError> {
    let input = provider.read_text(manifest_source_id).map_err(|error| {
        OrchestrationSuiteLoadError::new(format!(
            "failed to read orchestration suite '{}': {}",
            manifest_source_id,
            error.kind().as_str()
        ))
    })?;
    parse_orchestration_suite(&input)
        .map_err(|error| OrchestrationSuiteLoadError::new(error.to_string()))
}

pub fn run_orchestration_suite_json(manifest_path: impl AsRef<Path>) -> OrchestrationSuiteJsonRun {
    let manifest_source_id = match path_source_id(manifest_path.as_ref()) {
        Ok(source_id) => source_id,
        Err(error) => {
            return OrchestrationSuiteJsonRun {
                envelope: OrchestrationSuiteResultEnvelope::error(error.to_string()),
                exit_code: 2,
            };
        }
    };
    run_orchestration_suite_json_with_provider(&FileSystemTextSourceProvider, &manifest_source_id)
}

pub fn run_orchestration_suite_json_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> OrchestrationSuiteJsonRun {
    match run_orchestration_suite_json_inner(provider, manifest_source_id) {
        Ok(run) => run,
        Err(error) => OrchestrationSuiteJsonRun {
            envelope: OrchestrationSuiteResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

pub fn run_orchestration_suite_expectations_json(
    manifest_path: impl AsRef<Path>,
) -> OrchestrationRegressionSuiteJsonRun {
    let manifest_source_id = match path_source_id(manifest_path.as_ref()) {
        Ok(source_id) => source_id,
        Err(error) => {
            return OrchestrationRegressionSuiteJsonRun {
                envelope: OrchestrationRegressionSuiteResultEnvelope::error(error.to_string()),
                exit_code: 2,
            };
        }
    };
    run_orchestration_suite_expectations_json_with_provider(
        &FileSystemTextSourceProvider,
        &manifest_source_id,
    )
}

pub fn run_orchestration_suite_expectations_json_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> OrchestrationRegressionSuiteJsonRun {
    match run_orchestration_suite_expectations_json_inner(provider, manifest_source_id) {
        Ok(run) => run,
        Err(error) => OrchestrationRegressionSuiteJsonRun {
            envelope: OrchestrationRegressionSuiteResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

fn run_orchestration_suite_json_inner(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> Result<OrchestrationSuiteJsonRun, String> {
    let suite = load_orchestration_suite_with_provider(provider, manifest_source_id)
        .map_err(|error| error.to_string())?;
    let mut jobs = Vec::with_capacity(suite.entries().len());

    for entry in suite.entries() {
        let resolved = resolve_source_id(manifest_source_id, entry.job_path())
            .map_err(|error| error.to_string())?;
        let (job_exit_code, result) = run_entry(provider, entry.family(), &resolved);
        jobs.push(OrchestrationSuiteEntryResult {
            family: entry.family(),
            manifest: entry.job_path().to_owned(),
            job_exit_code,
            result,
        });
    }

    let envelope = OrchestrationSuiteResultEnvelope::from_suite(&suite, jobs);
    let exit_code = envelope.status.exit_code();
    Ok(OrchestrationSuiteJsonRun {
        envelope,
        exit_code,
    })
}

fn run_orchestration_suite_expectations_json_inner(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> Result<OrchestrationRegressionSuiteJsonRun, String> {
    let suite = load_orchestration_suite_with_provider(provider, manifest_source_id)
        .map_err(|error| error.to_string())?;
    if suite
        .entries()
        .iter()
        .any(|entry| entry.expected().is_none())
    {
        return Err(
            "expectation check requires every orchestration suite job to declare an expected outcome"
                .to_owned(),
        );
    }

    let mut jobs = Vec::with_capacity(suite.entries().len());
    for entry in suite.entries() {
        let expected = entry
            .expected()
            .expect("complete expectation validation ran before execution");
        let resolved = resolve_source_id(manifest_source_id, entry.job_path())
            .map_err(|error| error.to_string())?;
        let (job_exit_code, result) = run_entry(provider, entry.family(), &resolved);
        let observed = result.outcome_str().to_owned();
        let matched = expectation_matches(expected, &result);
        jobs.push(OrchestrationRegressionEntryResult {
            family: entry.family(),
            manifest: entry.job_path().to_owned(),
            expected,
            observed,
            matched,
            job_exit_code,
            result,
        });
    }

    let envelope = OrchestrationRegressionSuiteResultEnvelope::from_suite(&suite, jobs);
    let exit_code = match envelope.outcome {
        OrchestrationRegressionOutcome::Matched => 0,
        OrchestrationRegressionOutcome::Mismatched => ORCHESTRATION_REGRESSION_MISMATCH_EXIT_CODE,
        OrchestrationRegressionOutcome::Error => 2,
    };
    Ok(OrchestrationRegressionSuiteJsonRun {
        envelope,
        exit_code,
    })
}

fn run_entry(
    provider: &dyn TextSourceProvider,
    family: OrchestrationJobFamily,
    manifest_source_id: &str,
) -> (u8, OrchestrationNestedResult) {
    match family {
        OrchestrationJobFamily::Verification => {
            let run: VerificationJobJsonRun =
                run_verification_job_json_with_provider(provider, manifest_source_id);
            (
                run.exit_code,
                OrchestrationNestedResult::Verification(Box::new(run.envelope)),
            )
        }
        OrchestrationJobFamily::Structural => {
            let run: StructuralJobJsonRun =
                run_structural_job_json_with_provider(provider, manifest_source_id);
            (
                run.exit_code,
                OrchestrationNestedResult::Structural(Box::new(run.envelope)),
            )
        }
        OrchestrationJobFamily::CertificateVerification => {
            let run: CertificateVerificationJobJsonRun =
                run_certificate_verification_job_json_with_provider(provider, manifest_source_id);
            (
                run.exit_code,
                OrchestrationNestedResult::CertificateVerification(Box::new(run.envelope)),
            )
        }
    }
}

fn aggregate_status(jobs: &[OrchestrationSuiteEntryResult]) -> OrchestrationSuiteStatus {
    if jobs.iter().any(|job| job.result.is_error()) {
        OrchestrationSuiteStatus::Error
    } else if jobs.iter().any(|job| job.result.is_rejected()) {
        OrchestrationSuiteStatus::Attention
    } else {
        OrchestrationSuiteStatus::Complete
    }
}

fn aggregate_regression_status(
    jobs: &[OrchestrationRegressionEntryResult],
) -> OrchestrationSuiteStatus {
    if jobs.iter().any(|job| job.result.is_error()) {
        OrchestrationSuiteStatus::Error
    } else if jobs.iter().any(|job| job.result.is_rejected()) {
        OrchestrationSuiteStatus::Attention
    } else {
        OrchestrationSuiteStatus::Complete
    }
}

fn expectation_matches(
    expected: OrchestrationExpectedOutcome,
    observed: &OrchestrationNestedResult,
) -> bool {
    match (expected, observed) {
        (
            OrchestrationExpectedOutcome::Verification(expected),
            OrchestrationNestedResult::Verification(result),
        ) => expected == result.outcome,
        (
            OrchestrationExpectedOutcome::Structural(expected),
            OrchestrationNestedResult::Structural(result),
        ) => expected == result.outcome,
        (
            OrchestrationExpectedOutcome::CertificateVerification(expected),
            OrchestrationNestedResult::CertificateVerification(result),
        ) => expected == result.outcome,
        _ => false,
    }
}

fn verification_outcome_str(outcome: VerificationJobOutcome) -> &'static str {
    match outcome {
        VerificationJobOutcome::Satisfied => "satisfied",
        VerificationJobOutcome::Violated => "violated",
        VerificationJobOutcome::Inconclusive => "inconclusive",
        VerificationJobOutcome::Error => "error",
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

fn field_bool(out: &mut String, name: &str, value: bool, first: bool) {
    field_name(out, name, first);
    out.push_str(if value { "true" } else { "false" });
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

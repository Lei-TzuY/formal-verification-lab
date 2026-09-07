use crate::verification_job_run::run_verification_job_json;
use crate::verification_result::{VerificationJobOutcome, VerificationJobResultEnvelope};
use crate::verification_suite::{
    parse_verification_suite, VerificationExpectedOutcome, VerificationSuite,
};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const VERIFICATION_SUITE_RESULT_SCHEMA_VERSION: u32 = 1;
pub const VERIFICATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION: u32 = 1;
pub const VERIFICATION_REGRESSION_MISMATCH_EXIT_CODE: u8 = 13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationSuiteOutcome {
    Satisfied,
    Violated,
    Inconclusive,
    Error,
}

impl VerificationSuiteOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Violated => "violated",
            Self::Inconclusive => "inconclusive",
            Self::Error => "error",
        }
    }

    fn status(self) -> Option<&'static str> {
        match self {
            Self::Satisfied => Some("SATISFIED"),
            Self::Violated => Some("VIOLATED"),
            Self::Inconclusive => Some("INCONCLUSIVE"),
            Self::Error => None,
        }
    }

    fn exit_code(self) -> u8 {
        match self {
            Self::Satisfied => 0,
            Self::Violated => 7,
            Self::Inconclusive => 3,
            Self::Error => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationSuiteEntryResult {
    pub manifest: String,
    pub result: VerificationJobResultEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationSuiteResultEnvelope {
    pub schema_version: u32,
    pub outcome: VerificationSuiteOutcome,
    pub suite: Option<String>,
    pub jobs: Vec<VerificationSuiteEntryResult>,
    pub error: Option<String>,
}

impl VerificationSuiteResultEnvelope {
    fn from_suite(suite: &VerificationSuite, jobs: Vec<VerificationSuiteEntryResult>) -> Self {
        let outcome = aggregate_outcome(&jobs);
        Self {
            schema_version: VERIFICATION_SUITE_RESULT_SCHEMA_VERSION,
            outcome,
            suite: Some(suite.name().to_owned()),
            jobs,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: VERIFICATION_SUITE_RESULT_SCHEMA_VERSION,
            outcome: VerificationSuiteOutcome::Error,
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
        field_optional_string(&mut out, "status", self.outcome.status(), false);
        field_optional_string(&mut out, "suite", self.suite.as_deref(), false);
        out.push_str(",\"jobs\":[");
        for (index, job) in self.jobs.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push('{');
            field_string(&mut out, "manifest", &job.manifest, true);
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
pub struct VerificationSuiteJsonRun {
    pub envelope: VerificationSuiteResultEnvelope,
    pub exit_code: u8,
}

impl VerificationSuiteJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationRegressionSuiteOutcome {
    Matched,
    Mismatched,
    Error,
}

impl VerificationRegressionSuiteOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRegressionSuiteEntryResult {
    pub manifest: String,
    pub expected: VerificationExpectedOutcome,
    pub observed: VerificationJobOutcome,
    pub matched: bool,
    pub result: VerificationJobResultEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRegressionSuiteResultEnvelope {
    pub schema_version: u32,
    pub outcome: VerificationRegressionSuiteOutcome,
    pub suite: Option<String>,
    pub jobs: Vec<VerificationRegressionSuiteEntryResult>,
    pub error: Option<String>,
}

impl VerificationRegressionSuiteResultEnvelope {
    fn from_suite(
        suite: &VerificationSuite,
        jobs: Vec<VerificationRegressionSuiteEntryResult>,
    ) -> Self {
        let outcome = if jobs.iter().all(|job| job.matched) {
            VerificationRegressionSuiteOutcome::Matched
        } else {
            VerificationRegressionSuiteOutcome::Mismatched
        };
        Self {
            schema_version: VERIFICATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
            outcome,
            suite: Some(suite.name().to_owned()),
            jobs,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: VERIFICATION_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
            outcome: VerificationRegressionSuiteOutcome::Error,
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
        field_optional_string(&mut out, "suite", self.suite.as_deref(), false);
        out.push_str(",\"jobs\":[");
        for (index, job) in self.jobs.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push('{');
            field_string(&mut out, "manifest", &job.manifest, true);
            field_string(&mut out, "expected", job.expected.as_str(), false);
            field_string(&mut out, "observed", job_outcome_str(job.observed), false);
            field_bool(&mut out, "matched", job.matched, false);
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
pub struct VerificationRegressionSuiteJsonRun {
    pub envelope: VerificationRegressionSuiteResultEnvelope,
    pub exit_code: u8,
}

impl VerificationRegressionSuiteJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationSuiteLoadError {
    message: String,
}

impl VerificationSuiteLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for VerificationSuiteLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for VerificationSuiteLoadError {}

pub fn load_verification_suite(
    manifest_path: impl AsRef<Path>,
) -> Result<VerificationSuite, VerificationSuiteLoadError> {
    let manifest_path = manifest_path.as_ref();
    let input = fs::read_to_string(manifest_path).map_err(|error| {
        VerificationSuiteLoadError::new(format!(
            "failed to read verification suite '{}': {error}",
            manifest_path.display()
        ))
    })?;
    parse_verification_suite(&input)
        .map_err(|error| VerificationSuiteLoadError::new(error.to_string()))
}

pub fn run_verification_suite_json(manifest_path: impl AsRef<Path>) -> VerificationSuiteJsonRun {
    let manifest_path = manifest_path.as_ref();
    match run_verification_suite_json_inner(manifest_path) {
        Ok(run) => run,
        Err(error) => VerificationSuiteJsonRun {
            envelope: VerificationSuiteResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

pub fn run_verification_suite_expectations_json(
    manifest_path: impl AsRef<Path>,
) -> VerificationRegressionSuiteJsonRun {
    let manifest_path = manifest_path.as_ref();
    match run_verification_suite_expectations_json_inner(manifest_path) {
        Ok(run) => run,
        Err(error) => VerificationRegressionSuiteJsonRun {
            envelope: VerificationRegressionSuiteResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

fn run_verification_suite_json_inner(
    manifest_path: &Path,
) -> Result<VerificationSuiteJsonRun, String> {
    let suite = load_verification_suite(manifest_path).map_err(|error| error.to_string())?;
    let base = manifest_path.parent().unwrap_or_else(|| Path::new(""));
    let mut jobs = Vec::with_capacity(suite.job_paths().len());

    for job_path in suite.job_paths() {
        let resolved = resolve_path(base, Path::new(job_path));
        let run = run_verification_job_json(&resolved);
        jobs.push(VerificationSuiteEntryResult {
            manifest: job_path.clone(),
            result: run.envelope,
        });
    }

    let envelope = VerificationSuiteResultEnvelope::from_suite(&suite, jobs);
    let exit_code = envelope.outcome.exit_code();
    Ok(VerificationSuiteJsonRun {
        envelope,
        exit_code,
    })
}

fn run_verification_suite_expectations_json_inner(
    manifest_path: &Path,
) -> Result<VerificationRegressionSuiteJsonRun, String> {
    let suite = load_verification_suite(manifest_path).map_err(|error| error.to_string())?;
    if suite.expected_outcomes().iter().any(Option::is_none) {
        return Err(
            "expectation check requires every verification suite job to declare an expected outcome"
                .to_owned(),
        );
    }

    let base = manifest_path.parent().unwrap_or_else(|| Path::new(""));
    let mut jobs = Vec::with_capacity(suite.job_paths().len());
    for (job_path, expected) in suite
        .job_paths()
        .iter()
        .zip(suite.expected_outcomes().iter().copied())
    {
        let expected = expected.expect("complete expectation validation ran before execution");
        let resolved = resolve_path(base, Path::new(job_path));
        let run = run_verification_job_json(&resolved);
        let observed = run.envelope.outcome;
        jobs.push(VerificationRegressionSuiteEntryResult {
            manifest: job_path.clone(),
            expected,
            observed,
            matched: expectation_matches(expected, observed),
            result: run.envelope,
        });
    }

    let envelope = VerificationRegressionSuiteResultEnvelope::from_suite(&suite, jobs);
    let exit_code = match envelope.outcome {
        VerificationRegressionSuiteOutcome::Matched => 0,
        VerificationRegressionSuiteOutcome::Mismatched => {
            VERIFICATION_REGRESSION_MISMATCH_EXIT_CODE
        }
        VerificationRegressionSuiteOutcome::Error => 2,
    };
    Ok(VerificationRegressionSuiteJsonRun {
        envelope,
        exit_code,
    })
}

fn aggregate_outcome(jobs: &[VerificationSuiteEntryResult]) -> VerificationSuiteOutcome {
    if jobs
        .iter()
        .any(|job| job.result.outcome == VerificationJobOutcome::Error)
    {
        return VerificationSuiteOutcome::Error;
    }
    if jobs
        .iter()
        .any(|job| job.result.outcome == VerificationJobOutcome::Violated)
    {
        return VerificationSuiteOutcome::Violated;
    }
    if jobs
        .iter()
        .any(|job| job.result.outcome == VerificationJobOutcome::Inconclusive)
    {
        return VerificationSuiteOutcome::Inconclusive;
    }
    VerificationSuiteOutcome::Satisfied
}

fn expectation_matches(
    expected: VerificationExpectedOutcome,
    observed: VerificationJobOutcome,
) -> bool {
    matches!(
        (expected, observed),
        (
            VerificationExpectedOutcome::Satisfied,
            VerificationJobOutcome::Satisfied
        ) | (
            VerificationExpectedOutcome::Violated,
            VerificationJobOutcome::Violated
        ) | (
            VerificationExpectedOutcome::Inconclusive,
            VerificationJobOutcome::Inconclusive
        ) | (
            VerificationExpectedOutcome::Error,
            VerificationJobOutcome::Error
        )
    )
}

fn job_outcome_str(outcome: VerificationJobOutcome) -> &'static str {
    match outcome {
        VerificationJobOutcome::Satisfied => "satisfied",
        VerificationJobOutcome::Violated => "violated",
        VerificationJobOutcome::Inconclusive => "inconclusive",
        VerificationJobOutcome::Error => "error",
    }
}

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
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

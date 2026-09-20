use crate::structural_job_run::run_structural_job_json;
use crate::structural_result::{StructuralJobOutcome, StructuralJobResultEnvelope};
use crate::structural_suite::{parse_structural_suite, StructuralExpectedOutcome, StructuralSuite};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const STRUCTURAL_SUITE_RESULT_SCHEMA_VERSION: u32 = 1;
pub const STRUCTURAL_REGRESSION_SUITE_RESULT_SCHEMA_VERSION: u32 = 1;
pub const STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE: u8 = 13;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralSuiteOutcome {
    Complete,
    Inconclusive,
    Error,
}

impl StructuralSuiteOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Inconclusive => "inconclusive",
            Self::Error => "error",
        }
    }

    fn exit_code(self) -> u8 {
        match self {
            Self::Complete => 0,
            Self::Inconclusive => 3,
            Self::Error => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralSuiteEntryResult {
    pub manifest: String,
    pub result: StructuralJobResultEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralSuiteResultEnvelope {
    pub schema_version: u32,
    pub outcome: StructuralSuiteOutcome,
    pub suite: Option<String>,
    pub jobs: Vec<StructuralSuiteEntryResult>,
    pub error: Option<String>,
}

impl StructuralSuiteResultEnvelope {
    fn from_suite(suite: &StructuralSuite, jobs: Vec<StructuralSuiteEntryResult>) -> Self {
        Self {
            schema_version: STRUCTURAL_SUITE_RESULT_SCHEMA_VERSION,
            outcome: aggregate_outcome(&jobs),
            suite: Some(suite.name().to_owned()),
            jobs,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: STRUCTURAL_SUITE_RESULT_SCHEMA_VERSION,
            outcome: StructuralSuiteOutcome::Error,
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
pub struct StructuralSuiteJsonRun {
    pub envelope: StructuralSuiteResultEnvelope,
    pub exit_code: u8,
}

impl StructuralSuiteJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralRegressionSuiteOutcome {
    Matched,
    Mismatched,
    Error,
}

impl StructuralRegressionSuiteOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralRegressionSuiteEntryResult {
    pub manifest: String,
    pub expected: StructuralExpectedOutcome,
    pub observed: StructuralJobOutcome,
    pub matched: bool,
    pub result: StructuralJobResultEnvelope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralRegressionSuiteResultEnvelope {
    pub schema_version: u32,
    pub outcome: StructuralRegressionSuiteOutcome,
    pub suite: Option<String>,
    pub jobs: Vec<StructuralRegressionSuiteEntryResult>,
    pub error: Option<String>,
}

impl StructuralRegressionSuiteResultEnvelope {
    fn from_suite(
        suite: &StructuralSuite,
        jobs: Vec<StructuralRegressionSuiteEntryResult>,
    ) -> Self {
        let outcome = if jobs.iter().all(|job| job.matched) {
            StructuralRegressionSuiteOutcome::Matched
        } else {
            StructuralRegressionSuiteOutcome::Mismatched
        };
        Self {
            schema_version: STRUCTURAL_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
            outcome,
            suite: Some(suite.name().to_owned()),
            jobs,
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: STRUCTURAL_REGRESSION_SUITE_RESULT_SCHEMA_VERSION,
            outcome: StructuralRegressionSuiteOutcome::Error,
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
            field_string(&mut out, "observed", job.observed.as_str(), false);
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
pub struct StructuralRegressionSuiteJsonRun {
    pub envelope: StructuralRegressionSuiteResultEnvelope,
    pub exit_code: u8,
}

impl StructuralRegressionSuiteJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralSuiteLoadError {
    message: String,
}

impl StructuralSuiteLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for StructuralSuiteLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StructuralSuiteLoadError {}

pub fn load_structural_suite(
    manifest_path: impl AsRef<Path>,
) -> Result<StructuralSuite, StructuralSuiteLoadError> {
    let manifest_path = manifest_path.as_ref();
    let input = fs::read_to_string(manifest_path).map_err(|error| {
        StructuralSuiteLoadError::new(format!(
            "failed to read structural suite '{}': {error}",
            manifest_path.display()
        ))
    })?;
    parse_structural_suite(&input).map_err(|error| StructuralSuiteLoadError::new(error.to_string()))
}

pub fn run_structural_suite_json(manifest_path: impl AsRef<Path>) -> StructuralSuiteJsonRun {
    let manifest_path = manifest_path.as_ref();
    match run_structural_suite_json_inner(manifest_path) {
        Ok(run) => run,
        Err(error) => StructuralSuiteJsonRun {
            envelope: StructuralSuiteResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

pub fn run_structural_suite_expectations_json(
    manifest_path: impl AsRef<Path>,
) -> StructuralRegressionSuiteJsonRun {
    let manifest_path = manifest_path.as_ref();
    match run_structural_suite_expectations_json_inner(manifest_path) {
        Ok(run) => run,
        Err(error) => StructuralRegressionSuiteJsonRun {
            envelope: StructuralRegressionSuiteResultEnvelope::error(error),
            exit_code: 2,
        },
    }
}

fn run_structural_suite_json_inner(manifest_path: &Path) -> Result<StructuralSuiteJsonRun, String> {
    let suite = load_structural_suite(manifest_path).map_err(|error| error.to_string())?;
    let base = manifest_path.parent().unwrap_or_else(|| Path::new(""));
    let mut jobs = Vec::with_capacity(suite.job_paths().len());

    for job_path in suite.job_paths() {
        let resolved = resolve_path(base, Path::new(job_path));
        let run = run_structural_job_json(&resolved);
        jobs.push(StructuralSuiteEntryResult {
            manifest: job_path.clone(),
            result: run.envelope,
        });
    }

    let envelope = StructuralSuiteResultEnvelope::from_suite(&suite, jobs);
    let exit_code = envelope.outcome.exit_code();
    Ok(StructuralSuiteJsonRun {
        envelope,
        exit_code,
    })
}

fn run_structural_suite_expectations_json_inner(
    manifest_path: &Path,
) -> Result<StructuralRegressionSuiteJsonRun, String> {
    let suite = load_structural_suite(manifest_path).map_err(|error| error.to_string())?;
    if suite.expected_outcomes().iter().any(Option::is_none) {
        return Err(
            "expectation check requires every structural suite job to declare an expected outcome"
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
        let run = run_structural_job_json(&resolved);
        let observed = run.envelope.outcome;
        jobs.push(StructuralRegressionSuiteEntryResult {
            manifest: job_path.clone(),
            expected,
            observed,
            matched: expectation_matches(expected, observed),
            result: run.envelope,
        });
    }

    let envelope = StructuralRegressionSuiteResultEnvelope::from_suite(&suite, jobs);
    let exit_code = match envelope.outcome {
        StructuralRegressionSuiteOutcome::Matched => 0,
        StructuralRegressionSuiteOutcome::Mismatched => STRUCTURAL_REGRESSION_MISMATCH_EXIT_CODE,
        StructuralRegressionSuiteOutcome::Error => 2,
    };
    Ok(StructuralRegressionSuiteJsonRun {
        envelope,
        exit_code,
    })
}

fn aggregate_outcome(jobs: &[StructuralSuiteEntryResult]) -> StructuralSuiteOutcome {
    if jobs
        .iter()
        .any(|job| job.result.outcome == StructuralJobOutcome::Error)
    {
        return StructuralSuiteOutcome::Error;
    }
    if jobs
        .iter()
        .any(|job| job.result.outcome == StructuralJobOutcome::Inconclusive)
    {
        return StructuralSuiteOutcome::Inconclusive;
    }
    StructuralSuiteOutcome::Complete
}

fn expectation_matches(
    expected: StructuralExpectedOutcome,
    observed: StructuralJobOutcome,
) -> bool {
    matches!(
        (expected, observed),
        (
            StructuralExpectedOutcome::CycleFound,
            StructuralJobOutcome::CycleFound
        ) | (
            StructuralExpectedOutcome::Acyclic,
            StructuralJobOutcome::Acyclic
        ) | (
            StructuralExpectedOutcome::Inconclusive,
            StructuralJobOutcome::Inconclusive
        ) | (
            StructuralExpectedOutcome::Error,
            StructuralJobOutcome::Error
        )
    )
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

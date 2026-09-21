use crate::replay_json_diagnostic::{
    diagnose_replay_json_drift, WorkspaceReplayJsonDifference,
};
use crate::workspace_replay_lock::WorkspaceReplayMode;
use crate::workspace_snapshot::{
    parse_workspace_snapshot, replay_workspace_snapshot_expectations_json,
    replay_workspace_snapshot_json, WorkspaceSnapshot,
};
use std::fmt::Write as _;

pub const WORKSPACE_REPLAY_DETERMINISM_SCHEMA_VERSION: u32 = 1;
pub const MAX_WORKSPACE_REPLAY_DETERMINISM_ADDITIONAL_ATTEMPTS: usize = 64;
pub const WORKSPACE_REPLAY_NONDETERMINISTIC_EXIT_CODE: u8 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceReplayDeterminismStatus {
    Deterministic,
    Nondeterministic,
    Error,
}

impl WorkspaceReplayDeterminismStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::Nondeterministic => "nondeterministic",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplayDeterminismEnvelope {
    pub schema_version: u32,
    pub status: WorkspaceReplayDeterminismStatus,
    pub mode: WorkspaceReplayMode,
    pub requested_additional_attempts: usize,
    pub completed_attempts: usize,
    pub baseline_exit_code: Option<u8>,
    pub divergent_attempt_index: Option<usize>,
    pub actual_exit_code: Option<u8>,
    pub exit_code_matches: Option<bool>,
    pub json_matches: Option<bool>,
    pub json_difference: Option<WorkspaceReplayJsonDifference>,
    pub error: Option<String>,
}

impl WorkspaceReplayDeterminismEnvelope {
    fn deterministic(
        mode: WorkspaceReplayMode,
        requested_additional_attempts: usize,
        completed_attempts: usize,
        baseline_exit_code: u8,
    ) -> Self {
        Self {
            schema_version: WORKSPACE_REPLAY_DETERMINISM_SCHEMA_VERSION,
            status: WorkspaceReplayDeterminismStatus::Deterministic,
            mode,
            requested_additional_attempts,
            completed_attempts,
            baseline_exit_code: Some(baseline_exit_code),
            divergent_attempt_index: None,
            actual_exit_code: None,
            exit_code_matches: None,
            json_matches: None,
            json_difference: None,
            error: None,
        }
    }

    fn nondeterministic(
        mode: WorkspaceReplayMode,
        requested_additional_attempts: usize,
        completed_attempts: usize,
        baseline_exit_code: u8,
        actual_exit_code: u8,
        exit_code_matches: bool,
        json_matches: bool,
        baseline_json: &str,
        actual_json: &str,
    ) -> Self {
        Self {
            schema_version: WORKSPACE_REPLAY_DETERMINISM_SCHEMA_VERSION,
            status: WorkspaceReplayDeterminismStatus::Nondeterministic,
            mode,
            requested_additional_attempts,
            completed_attempts,
            baseline_exit_code: Some(baseline_exit_code),
            divergent_attempt_index: Some(completed_attempts),
            actual_exit_code: Some(actual_exit_code),
            exit_code_matches: Some(exit_code_matches),
            json_matches: Some(json_matches),
            json_difference: if json_matches {
                None
            } else {
                diagnose_replay_json_drift(baseline_json, actual_json)
            },
            error: None,
        }
    }

    fn error(
        mode: WorkspaceReplayMode,
        requested_additional_attempts: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: WORKSPACE_REPLAY_DETERMINISM_SCHEMA_VERSION,
            status: WorkspaceReplayDeterminismStatus::Error,
            mode,
            requested_additional_attempts,
            completed_attempts: 0,
            baseline_exit_code: None,
            divergent_attempt_index: None,
            actual_exit_code: None,
            exit_code_matches: None,
            json_matches: None,
            json_difference: None,
            error: Some(message.into()),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u64(&mut out, "schema_version", self.schema_version as u64, true);
        field_string(&mut out, "status", self.status.as_str(), false);
        field_string(&mut out, "mode", self.mode.as_str(), false);
        field_u64(
            &mut out,
            "requested_additional_attempts",
            self.requested_additional_attempts as u64,
            false,
        );
        field_u64(
            &mut out,
            "completed_attempts",
            self.completed_attempts as u64,
            false,
        );
        field_optional_u64(
            &mut out,
            "baseline_exit_code",
            self.baseline_exit_code.map(u64::from),
            false,
        );
        field_optional_u64(
            &mut out,
            "divergent_attempt_index",
            self.divergent_attempt_index.map(|value| value as u64),
            false,
        );
        field_optional_u64(
            &mut out,
            "actual_exit_code",
            self.actual_exit_code.map(u64::from),
            false,
        );
        field_optional_bool(&mut out, "exit_code_matches", self.exit_code_matches, false);
        field_optional_bool(&mut out, "json_matches", self.json_matches, false);
        field_json_difference(
            &mut out,
            "json_difference",
            self.json_difference.as_ref(),
            false,
        );
        field_optional_string(&mut out, "error", self.error.as_deref(), false);
        out.push('}');
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplayDeterminismJsonRun {
    pub envelope: WorkspaceReplayDeterminismEnvelope,
    pub exit_code: u8,
}

impl WorkspaceReplayDeterminismJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

pub fn audit_workspace_replay_determinism(
    snapshot: &WorkspaceSnapshot,
    mode: WorkspaceReplayMode,
    additional_attempts: usize,
) -> WorkspaceReplayDeterminismJsonRun {
    audit_replay_runs(mode, additional_attempts, || replay(snapshot, mode))
}

pub fn audit_workspace_replay_determinism_text(
    snapshot_text: &str,
    mode: WorkspaceReplayMode,
    additional_attempts: usize,
) -> WorkspaceReplayDeterminismJsonRun {
    if let Err(message) = validate_additional_attempts(additional_attempts) {
        return error_run(mode, additional_attempts, message);
    }

    let snapshot = match parse_workspace_snapshot(snapshot_text) {
        Ok(snapshot) => snapshot,
        Err(error) => return error_run(mode, additional_attempts, error.to_string()),
    };
    audit_workspace_replay_determinism(&snapshot, mode, additional_attempts)
}

fn audit_replay_runs<F>(
    mode: WorkspaceReplayMode,
    additional_attempts: usize,
    mut replay_once: F,
) -> WorkspaceReplayDeterminismJsonRun
where
    F: FnMut() -> (u8, String),
{
    if let Err(message) = validate_additional_attempts(additional_attempts) {
        return error_run(mode, additional_attempts, message);
    }

    let (baseline_exit_code, baseline_json) = replay_once();
    let total_attempts = additional_attempts + 1;

    for attempt_index in 2..=total_attempts {
        let (actual_exit_code, actual_json) = replay_once();
        let exit_code_matches = actual_exit_code == baseline_exit_code;
        let json_matches = actual_json == baseline_json;

        if !exit_code_matches || !json_matches {
            return WorkspaceReplayDeterminismJsonRun {
                envelope: WorkspaceReplayDeterminismEnvelope::nondeterministic(
                    mode,
                    additional_attempts,
                    attempt_index,
                    baseline_exit_code,
                    actual_exit_code,
                    exit_code_matches,
                    json_matches,
                    &baseline_json,
                    &actual_json,
                ),
                exit_code: WORKSPACE_REPLAY_NONDETERMINISTIC_EXIT_CODE,
            };
        }
    }

    WorkspaceReplayDeterminismJsonRun {
        envelope: WorkspaceReplayDeterminismEnvelope::deterministic(
            mode,
            additional_attempts,
            total_attempts,
            baseline_exit_code,
        ),
        exit_code: 0,
    }
}

fn validate_additional_attempts(additional_attempts: usize) -> Result<(), String> {
    if additional_attempts == 0 {
        return Err("workspace replay determinism audit requires at least one additional attempt"
            .to_owned());
    }
    if additional_attempts > MAX_WORKSPACE_REPLAY_DETERMINISM_ADDITIONAL_ATTEMPTS {
        return Err(format!(
            "workspace replay determinism audit exceeds maximum of {} additional attempts",
            MAX_WORKSPACE_REPLAY_DETERMINISM_ADDITIONAL_ATTEMPTS
        ));
    }
    Ok(())
}

fn error_run(
    mode: WorkspaceReplayMode,
    additional_attempts: usize,
    message: impl Into<String>,
) -> WorkspaceReplayDeterminismJsonRun {
    WorkspaceReplayDeterminismJsonRun {
        envelope: WorkspaceReplayDeterminismEnvelope::error(
            mode,
            additional_attempts,
            message,
        ),
        exit_code: 2,
    }
}

fn replay(snapshot: &WorkspaceSnapshot, mode: WorkspaceReplayMode) -> (u8, String) {
    match mode {
        WorkspaceReplayMode::Raw => {
            let run = replay_workspace_snapshot_json(snapshot);
            (run.exit_code, run.to_json())
        }
        WorkspaceReplayMode::Expectations => {
            let run = replay_workspace_snapshot_expectations_json(snapshot);
            (run.exit_code, run.to_json())
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

fn field_optional_u64(out: &mut String, name: &str, value: Option<u64>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

fn field_optional_bool(out: &mut String, name: &str, value: Option<bool>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(true) => out.push_str("true"),
        Some(false) => out.push_str("false"),
        None => out.push_str("null"),
    }
}

fn field_optional_string(out: &mut String, name: &str, value: Option<&str>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => write_json_string(out, value),
        None => out.push_str("null"),
    }
}

fn field_json_difference(
    out: &mut String,
    name: &str,
    value: Option<&WorkspaceReplayJsonDifference>,
    first: bool,
) {
    field_name(out, name, first);
    let Some(value) = value else {
        out.push_str("null");
        return;
    };

    out.push('{');
    field_string(out, "kind", value.kind.as_str(), true);
    field_optional_string(out, "path", value.path.as_deref(), false);
    field_u64(
        out,
        "first_byte_offset",
        value.first_byte_offset as u64,
        false,
    );
    field_optional_string(
        out,
        "expected_preview",
        value.expected_preview.as_deref(),
        false,
    );
    field_optional_string(
        out,
        "actual_preview",
        value.actual_preview.as_deref(),
        false,
    );
    out.push('}');
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
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_second_attempt_json_drift_reuses_structured_diagnostic() {
        let mut attempt = 0usize;
        let run = audit_replay_runs(WorkspaceReplayMode::Raw, 5, || {
            attempt += 1;
            if attempt == 1 {
                (0, r#"{"jobs":[{"result":{"accounting":{"model_states":1}}}]}"#.to_owned())
            } else {
                (0, r#"{"jobs":[{"result":{"accounting":{"model_states":2}}}]}"#.to_owned())
            }
        });

        assert_eq!(attempt, 2);
        assert_eq!(
            run.envelope.status,
            WorkspaceReplayDeterminismStatus::Nondeterministic
        );
        assert_eq!(run.exit_code, WORKSPACE_REPLAY_NONDETERMINISTIC_EXIT_CODE);
        assert_eq!(run.envelope.completed_attempts, 2);
        assert_eq!(run.envelope.divergent_attempt_index, Some(2));
        assert_eq!(run.envelope.exit_code_matches, Some(true));
        assert_eq!(run.envelope.json_matches, Some(false));
        let difference = run.envelope.json_difference.unwrap();
        assert_eq!(
            difference.path.as_deref(),
            Some("/jobs/0/result/accounting/model_states")
        );
    }

    #[test]
    fn injected_exit_drift_stops_at_first_mismatch_without_json_difference() {
        let mut attempt = 0usize;
        let run = audit_replay_runs(WorkspaceReplayMode::Expectations, 4, || {
            attempt += 1;
            if attempt == 1 {
                (0, "{\"status\":\"matched\"}".to_owned())
            } else {
                (13, "{\"status\":\"matched\"}".to_owned())
            }
        });

        assert_eq!(attempt, 2);
        assert_eq!(
            run.envelope.status,
            WorkspaceReplayDeterminismStatus::Nondeterministic
        );
        assert_eq!(run.envelope.exit_code_matches, Some(false));
        assert_eq!(run.envelope.json_matches, Some(true));
        assert_eq!(run.envelope.json_difference, None);
    }

    #[test]
    fn invalid_attempt_counts_fail_before_replay() {
        let mut calls = 0usize;
        let zero = audit_replay_runs(WorkspaceReplayMode::Raw, 0, || {
            calls += 1;
            (0, "{}".to_owned())
        });
        assert_eq!(calls, 0);
        assert_eq!(zero.envelope.status, WorkspaceReplayDeterminismStatus::Error);

        let excessive = audit_replay_runs(
            WorkspaceReplayMode::Raw,
            MAX_WORKSPACE_REPLAY_DETERMINISM_ADDITIONAL_ATTEMPTS + 1,
            || {
                calls += 1;
                (0, "{}".to_owned())
            },
        );
        assert_eq!(calls, 0);
        assert_eq!(
            excessive.envelope.status,
            WorkspaceReplayDeterminismStatus::Error
        );
    }
}

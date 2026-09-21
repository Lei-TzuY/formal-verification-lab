use crate::replay_json_diagnostic::{diagnose_replay_json_drift, WorkspaceReplayJsonDifference};
use crate::workspace_snapshot::{
    parse_workspace_snapshot, render_workspace_snapshot,
    replay_workspace_snapshot_expectations_json, replay_workspace_snapshot_json, WorkspaceSnapshot,
    WorkspaceSnapshotParseError, MAX_WORKSPACE_SNAPSHOT_ENTRIES,
    MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES, MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES,
};
use std::fmt;
use std::str;

pub const WORKSPACE_REPLAY_LOCK_SCHEMA_VERSION: u32 = 1;
pub const WORKSPACE_REPLAY_LOCK_VERIFICATION_SCHEMA_VERSION: u32 = 2;
pub const WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE: u8 = 13;
pub const MAX_WORKSPACE_REPLAY_LOCK_SNAPSHOT_BYTES: usize = MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES
    + MAX_WORKSPACE_SNAPSHOT_ENTRIES * MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES
    + 4 * 1024 * 1024;
pub const MAX_WORKSPACE_REPLAY_LOCK_RESULT_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceReplayMode {
    Raw,
    Expectations,
}

impl WorkspaceReplayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Expectations => "expectations",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "raw" => Some(Self::Raw),
            "expectations" => Some(Self::Expectations),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplayLock {
    schema_version: u32,
    mode: WorkspaceReplayMode,
    expected_exit_code: u8,
    snapshot_text: String,
    expected_json: String,
}

impl WorkspaceReplayLock {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn mode(&self) -> WorkspaceReplayMode {
        self.mode
    }

    pub fn expected_exit_code(&self) -> u8 {
        self.expected_exit_code
    }

    pub fn snapshot_text(&self) -> &str {
        &self.snapshot_text
    }

    pub fn expected_json(&self) -> &str {
        &self.expected_json
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceReplayLockParseErrorKind {
    MissingHeader,
    UnsupportedVersion { version: u32 },
    InvalidHeader { message: String },
    InvalidMode { mode: String },
    InvalidNumber { field: &'static str },
    Truncated { section: &'static str },
    MissingFrameTerminator { section: &'static str },
    InvalidUtf8Frame { section: &'static str },
    SnapshotTooLarge { limit: usize },
    ResultTooLarge { limit: usize },
    InvalidSnapshot { message: String },
    TrailingPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplayLockParseError {
    offset: usize,
    kind: WorkspaceReplayLockParseErrorKind,
}

impl WorkspaceReplayLockParseError {
    fn new(offset: usize, kind: WorkspaceReplayLockParseErrorKind) -> Self {
        Self { offset, kind }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn kind(&self) -> &WorkspaceReplayLockParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for WorkspaceReplayLockParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "workspace replay lock parse error at byte offset {}: ",
            self.offset
        )?;
        match &self.kind {
            WorkspaceReplayLockParseErrorKind::MissingHeader => {
                write!(f, "missing replay lock header")
            }
            WorkspaceReplayLockParseErrorKind::UnsupportedVersion { version } => write!(
                f,
                "unsupported workspace replay lock schema version {version}; expected {WORKSPACE_REPLAY_LOCK_SCHEMA_VERSION}"
            ),
            WorkspaceReplayLockParseErrorKind::InvalidHeader { message } => f.write_str(message),
            WorkspaceReplayLockParseErrorKind::InvalidMode { mode } => {
                write!(f, "unsupported replay mode '{mode}'; expected raw or expectations")
            }
            WorkspaceReplayLockParseErrorKind::InvalidNumber { field } => {
                write!(f, "{field} must be a non-negative decimal integer")
            }
            WorkspaceReplayLockParseErrorKind::Truncated { section } => {
                write!(f, "truncated {section}")
            }
            WorkspaceReplayLockParseErrorKind::MissingFrameTerminator { section } => {
                write!(f, "{section} is not followed by the required newline terminator")
            }
            WorkspaceReplayLockParseErrorKind::InvalidUtf8Frame { section } => {
                write!(f, "{section} byte length splits invalid UTF-8")
            }
            WorkspaceReplayLockParseErrorKind::SnapshotTooLarge { limit } => write!(
                f,
                "embedded workspace snapshot exceeds maximum of {limit} bytes"
            ),
            WorkspaceReplayLockParseErrorKind::ResultTooLarge { limit } => write!(
                f,
                "embedded expected result exceeds maximum of {limit} bytes"
            ),
            WorkspaceReplayLockParseErrorKind::InvalidSnapshot { message } => {
                write!(f, "embedded workspace snapshot is invalid: {message}")
            }
            WorkspaceReplayLockParseErrorKind::TrailingPayload => {
                write!(f, "unexpected trailing payload after replay lock end marker")
            }
        }
    }
}

impl std::error::Error for WorkspaceReplayLockParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceReplayLockVerificationStatus {
    Matched,
    Mismatched,
    Error,
}

impl WorkspaceReplayLockVerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Mismatched => "mismatched",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplayLockVerificationEnvelope {
    pub schema_version: u32,
    pub status: WorkspaceReplayLockVerificationStatus,
    pub mode: Option<WorkspaceReplayMode>,
    pub exit_code_matches: Option<bool>,
    pub json_matches: Option<bool>,
    pub expected_exit_code: Option<u8>,
    pub actual_exit_code: Option<u8>,
    pub json_difference: Option<WorkspaceReplayJsonDifference>,
    pub error: Option<String>,
}

impl WorkspaceReplayLockVerificationEnvelope {
    fn comparison(
        mode: WorkspaceReplayMode,
        expected_exit_code: u8,
        actual_exit_code: u8,
        exit_code_matches: bool,
        json_matches: bool,
        expected_json: &str,
        actual_json: &str,
    ) -> Self {
        Self {
            schema_version: WORKSPACE_REPLAY_LOCK_VERIFICATION_SCHEMA_VERSION,
            status: if exit_code_matches && json_matches {
                WorkspaceReplayLockVerificationStatus::Matched
            } else {
                WorkspaceReplayLockVerificationStatus::Mismatched
            },
            mode: Some(mode),
            exit_code_matches: Some(exit_code_matches),
            json_matches: Some(json_matches),
            expected_exit_code: Some(expected_exit_code),
            actual_exit_code: Some(actual_exit_code),
            json_difference: if json_matches {
                None
            } else {
                diagnose_replay_json_drift(expected_json, actual_json)
            },
            error: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            schema_version: WORKSPACE_REPLAY_LOCK_VERIFICATION_SCHEMA_VERSION,
            status: WorkspaceReplayLockVerificationStatus::Error,
            mode: None,
            exit_code_matches: None,
            json_matches: None,
            expected_exit_code: None,
            actual_exit_code: None,
            json_difference: None,
            error: Some(message.into()),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u64(&mut out, "schema_version", self.schema_version as u64, true);
        field_string(&mut out, "status", self.status.as_str(), false);
        field_optional_string(
            &mut out,
            "mode",
            self.mode.map(WorkspaceReplayMode::as_str),
            false,
        );
        field_optional_bool(&mut out, "exit_code_matches", self.exit_code_matches, false);
        field_optional_bool(&mut out, "json_matches", self.json_matches, false);
        field_optional_u64(
            &mut out,
            "expected_exit_code",
            self.expected_exit_code.map(u64::from),
            false,
        );
        field_optional_u64(
            &mut out,
            "actual_exit_code",
            self.actual_exit_code.map(u64::from),
            false,
        );
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
pub struct WorkspaceReplayLockJsonRun {
    pub envelope: WorkspaceReplayLockVerificationEnvelope,
    pub exit_code: u8,
}

impl WorkspaceReplayLockJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

pub fn create_workspace_replay_lock(
    snapshot: &WorkspaceSnapshot,
    mode: WorkspaceReplayMode,
) -> WorkspaceReplayLock {
    let snapshot_text = render_workspace_snapshot(snapshot);
    let (expected_exit_code, expected_json) = replay(snapshot, mode);
    WorkspaceReplayLock {
        schema_version: WORKSPACE_REPLAY_LOCK_SCHEMA_VERSION,
        mode,
        expected_exit_code,
        snapshot_text,
        expected_json,
    }
}

pub fn create_workspace_replay_lock_from_text(
    snapshot_text: &str,
    mode: WorkspaceReplayMode,
) -> Result<WorkspaceReplayLock, WorkspaceSnapshotParseError> {
    let snapshot = parse_workspace_snapshot(snapshot_text)?;
    Ok(create_workspace_replay_lock(&snapshot, mode))
}

pub fn render_workspace_replay_lock(lock: &WorkspaceReplayLock) -> String {
    let mut output = String::new();
    output.push_str("fvlab-workspace-replay-lock ");
    output.push_str(&lock.schema_version.to_string());
    output.push('\n');
    output.push_str("mode ");
    output.push_str(lock.mode.as_str());
    output.push('\n');
    output.push_str("exit ");
    output.push_str(&lock.expected_exit_code.to_string());
    output.push('\n');
    output.push_str("snapshot ");
    output.push_str(&lock.snapshot_text.len().to_string());
    output.push('\n');
    output.push_str(&lock.snapshot_text);
    output.push('\n');
    output.push_str("result ");
    output.push_str(&lock.expected_json.len().to_string());
    output.push('\n');
    output.push_str(&lock.expected_json);
    output.push('\n');
    output.push_str("end\n");
    output
}

pub fn parse_workspace_replay_lock(
    input: &str,
) -> Result<WorkspaceReplayLock, WorkspaceReplayLockParseError> {
    let mut cursor = ReplayLockCursor::new(input);

    let header_offset = cursor.position();
    let header = cursor.read_line("replay lock header")?;
    let mut header_parts = header.split_ascii_whitespace();
    if header_parts.next() != Some("fvlab-workspace-replay-lock") {
        return Err(WorkspaceReplayLockParseError::new(
            header_offset,
            WorkspaceReplayLockParseErrorKind::MissingHeader,
        ));
    }
    let version = parse_u32_field(header_parts.next(), header_offset, "schema version")?;
    if header_parts.next().is_some() {
        return Err(WorkspaceReplayLockParseError::new(
            header_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "replay lock header has unexpected trailing fields".to_owned(),
            },
        ));
    }
    if version != WORKSPACE_REPLAY_LOCK_SCHEMA_VERSION {
        return Err(WorkspaceReplayLockParseError::new(
            header_offset,
            WorkspaceReplayLockParseErrorKind::UnsupportedVersion { version },
        ));
    }

    let mode_offset = cursor.position();
    let mode_line = cursor.read_line("mode header")?;
    let mut mode_parts = mode_line.split_ascii_whitespace();
    if mode_parts.next() != Some("mode") {
        return Err(WorkspaceReplayLockParseError::new(
            mode_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "expected replay mode header".to_owned(),
            },
        ));
    }
    let mode_value = mode_parts.next().ok_or_else(|| {
        WorkspaceReplayLockParseError::new(
            mode_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "replay mode header is missing a mode".to_owned(),
            },
        )
    })?;
    if mode_parts.next().is_some() {
        return Err(WorkspaceReplayLockParseError::new(
            mode_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "replay mode header has unexpected trailing fields".to_owned(),
            },
        ));
    }
    let mode = WorkspaceReplayMode::parse(mode_value).ok_or_else(|| {
        WorkspaceReplayLockParseError::new(
            mode_offset,
            WorkspaceReplayLockParseErrorKind::InvalidMode {
                mode: mode_value.to_owned(),
            },
        )
    })?;

    let exit_offset = cursor.position();
    let exit_line = cursor.read_line("exit header")?;
    let mut exit_parts = exit_line.split_ascii_whitespace();
    if exit_parts.next() != Some("exit") {
        return Err(WorkspaceReplayLockParseError::new(
            exit_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "expected replay exit header".to_owned(),
            },
        ));
    }
    let expected_exit_code = parse_u8_field(exit_parts.next(), exit_offset, "expected exit code")?;
    if exit_parts.next().is_some() {
        return Err(WorkspaceReplayLockParseError::new(
            exit_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "replay exit header has unexpected trailing fields".to_owned(),
            },
        ));
    }

    let snapshot_header_offset = cursor.position();
    let snapshot_header = cursor.read_line("snapshot header")?;
    let snapshot_len = parse_length_header(
        snapshot_header,
        "snapshot",
        snapshot_header_offset,
        "snapshot byte length",
    )?;
    if snapshot_len > MAX_WORKSPACE_REPLAY_LOCK_SNAPSHOT_BYTES {
        return Err(WorkspaceReplayLockParseError::new(
            snapshot_header_offset,
            WorkspaceReplayLockParseErrorKind::SnapshotTooLarge {
                limit: MAX_WORKSPACE_REPLAY_LOCK_SNAPSHOT_BYTES,
            },
        ));
    }
    let snapshot_text = cursor
        .read_frame(snapshot_len, "workspace snapshot")?
        .to_owned();
    parse_workspace_snapshot(&snapshot_text).map_err(|error| {
        WorkspaceReplayLockParseError::new(
            snapshot_header_offset,
            WorkspaceReplayLockParseErrorKind::InvalidSnapshot {
                message: error.to_string(),
            },
        )
    })?;

    let result_header_offset = cursor.position();
    let result_header = cursor.read_line("result header")?;
    let result_len = parse_length_header(
        result_header,
        "result",
        result_header_offset,
        "result byte length",
    )?;
    if result_len > MAX_WORKSPACE_REPLAY_LOCK_RESULT_BYTES {
        return Err(WorkspaceReplayLockParseError::new(
            result_header_offset,
            WorkspaceReplayLockParseErrorKind::ResultTooLarge {
                limit: MAX_WORKSPACE_REPLAY_LOCK_RESULT_BYTES,
            },
        ));
    }
    let expected_json = cursor
        .read_frame(result_len, "expected result JSON")?
        .to_owned();

    let end_offset = cursor.position();
    if cursor.read_line("end marker")? != "end" {
        return Err(WorkspaceReplayLockParseError::new(
            end_offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: "expected replay lock end marker".to_owned(),
            },
        ));
    }
    if !cursor.is_eof() {
        return Err(WorkspaceReplayLockParseError::new(
            cursor.position(),
            WorkspaceReplayLockParseErrorKind::TrailingPayload,
        ));
    }

    Ok(WorkspaceReplayLock {
        schema_version: version,
        mode,
        expected_exit_code,
        snapshot_text,
        expected_json,
    })
}

pub fn verify_workspace_replay_lock(lock: &WorkspaceReplayLock) -> WorkspaceReplayLockJsonRun {
    let snapshot = match parse_workspace_snapshot(&lock.snapshot_text) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return WorkspaceReplayLockJsonRun {
                envelope: WorkspaceReplayLockVerificationEnvelope::error(error.to_string()),
                exit_code: 2,
            };
        }
    };
    let (actual_exit_code, actual_json) = replay(&snapshot, lock.mode);
    let exit_code_matches = actual_exit_code == lock.expected_exit_code;
    let json_matches = actual_json == lock.expected_json;
    let envelope = WorkspaceReplayLockVerificationEnvelope::comparison(
        lock.mode,
        lock.expected_exit_code,
        actual_exit_code,
        exit_code_matches,
        json_matches,
        &lock.expected_json,
        &actual_json,
    );
    let exit_code = match envelope.status {
        WorkspaceReplayLockVerificationStatus::Matched => 0,
        WorkspaceReplayLockVerificationStatus::Mismatched => {
            WORKSPACE_REPLAY_LOCK_MISMATCH_EXIT_CODE
        }
        WorkspaceReplayLockVerificationStatus::Error => 2,
    };
    WorkspaceReplayLockJsonRun {
        envelope,
        exit_code,
    }
}

pub fn verify_workspace_replay_lock_text(input: &str) -> WorkspaceReplayLockJsonRun {
    match parse_workspace_replay_lock(input) {
        Ok(lock) => verify_workspace_replay_lock(&lock),
        Err(error) => WorkspaceReplayLockJsonRun {
            envelope: WorkspaceReplayLockVerificationEnvelope::error(error.to_string()),
            exit_code: 2,
        },
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

fn parse_length_header(
    line: &str,
    directive: &str,
    offset: usize,
    field: &'static str,
) -> Result<usize, WorkspaceReplayLockParseError> {
    let mut parts = line.split_ascii_whitespace();
    if parts.next() != Some(directive) {
        return Err(WorkspaceReplayLockParseError::new(
            offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: format!("expected {directive} header"),
            },
        ));
    }
    let length = parse_usize_field(parts.next(), offset, field)?;
    if parts.next().is_some() {
        return Err(WorkspaceReplayLockParseError::new(
            offset,
            WorkspaceReplayLockParseErrorKind::InvalidHeader {
                message: format!("{directive} header has unexpected trailing fields"),
            },
        ));
    }
    Ok(length)
}

fn parse_u32_field(
    value: Option<&str>,
    offset: usize,
    field: &'static str,
) -> Result<u32, WorkspaceReplayLockParseError> {
    value
        .ok_or_else(|| {
            WorkspaceReplayLockParseError::new(
                offset,
                WorkspaceReplayLockParseErrorKind::InvalidNumber { field },
            )
        })?
        .parse::<u32>()
        .map_err(|_| {
            WorkspaceReplayLockParseError::new(
                offset,
                WorkspaceReplayLockParseErrorKind::InvalidNumber { field },
            )
        })
}

fn parse_u8_field(
    value: Option<&str>,
    offset: usize,
    field: &'static str,
) -> Result<u8, WorkspaceReplayLockParseError> {
    value
        .ok_or_else(|| {
            WorkspaceReplayLockParseError::new(
                offset,
                WorkspaceReplayLockParseErrorKind::InvalidNumber { field },
            )
        })?
        .parse::<u8>()
        .map_err(|_| {
            WorkspaceReplayLockParseError::new(
                offset,
                WorkspaceReplayLockParseErrorKind::InvalidNumber { field },
            )
        })
}

fn parse_usize_field(
    value: Option<&str>,
    offset: usize,
    field: &'static str,
) -> Result<usize, WorkspaceReplayLockParseError> {
    value
        .ok_or_else(|| {
            WorkspaceReplayLockParseError::new(
                offset,
                WorkspaceReplayLockParseErrorKind::InvalidNumber { field },
            )
        })?
        .parse::<usize>()
        .map_err(|_| {
            WorkspaceReplayLockParseError::new(
                offset,
                WorkspaceReplayLockParseErrorKind::InvalidNumber { field },
            )
        })
}

struct ReplayLockCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ReplayLockCursor<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            position: 0,
        }
    }

    fn position(&self) -> usize {
        self.position
    }

    fn is_eof(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn read_line(
        &mut self,
        section: &'static str,
    ) -> Result<&'a str, WorkspaceReplayLockParseError> {
        let start = self.position;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == b'\n') else {
            return Err(WorkspaceReplayLockParseError::new(
                start,
                WorkspaceReplayLockParseErrorKind::Truncated { section },
            ));
        };
        let end = start + relative_end;
        self.position = end + 1;
        str::from_utf8(&self.bytes[start..end]).map_err(|_| {
            WorkspaceReplayLockParseError::new(
                start,
                WorkspaceReplayLockParseErrorKind::InvalidUtf8Frame { section },
            )
        })
    }

    fn read_frame(
        &mut self,
        length: usize,
        section: &'static str,
    ) -> Result<&'a str, WorkspaceReplayLockParseError> {
        let start = self.position;
        let end = start.checked_add(length).ok_or_else(|| {
            WorkspaceReplayLockParseError::new(
                start,
                WorkspaceReplayLockParseErrorKind::Truncated { section },
            )
        })?;
        if end > self.bytes.len() {
            return Err(WorkspaceReplayLockParseError::new(
                start,
                WorkspaceReplayLockParseErrorKind::Truncated { section },
            ));
        }
        let frame = str::from_utf8(&self.bytes[start..end]).map_err(|_| {
            WorkspaceReplayLockParseError::new(
                start,
                WorkspaceReplayLockParseErrorKind::InvalidUtf8Frame { section },
            )
        })?;
        self.position = end;
        if self.bytes.get(self.position) != Some(&b'\n') {
            return Err(WorkspaceReplayLockParseError::new(
                self.position,
                WorkspaceReplayLockParseErrorKind::MissingFrameTerminator { section },
            ));
        }
        self.position += 1;
        Ok(frame)
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

fn field_optional_u64(out: &mut String, name: &str, value: Option<u64>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

fn field_string(out: &mut String, name: &str, value: &str, first: bool) {
    field_name(out, name, first);
    write_json_string(out, value);
}

fn field_optional_string(out: &mut String, name: &str, value: Option<&str>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => write_json_string(out, value),
        None => out.push_str("null"),
    }
}

fn field_optional_bool(out: &mut String, name: &str, value: Option<bool>, first: bool) {
    field_name(out, name, first);
    match value {
        Some(value) => out.push_str(if value { "true" } else { "false" }),
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
                out.push_str(&format!("\\u{:04x}", ch as u32));
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
}

use crate::certificate_verification_job::parse_certificate_verification_job;
use crate::orchestration_suite::{parse_orchestration_suite, OrchestrationJobFamily};
use crate::orchestration_suite_run::{
    run_orchestration_suite_expectations_json_with_provider,
    run_orchestration_suite_json_with_provider, OrchestrationRegressionSuiteJsonRun,
    OrchestrationSuiteJsonRun,
};
use crate::structural_job::parse_structural_job;
use crate::text_source::{
    normalize_workspace_source_id, resolve_source_id, MapTextSourceProvider, TextSourceErrorKind,
    TextSourceProvider,
};
use crate::verification_job::parse_verification_job;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str;

pub const WORKSPACE_SNAPSHOT_SCHEMA_VERSION: u32 = 1;
pub const MAX_WORKSPACE_SNAPSHOT_ENTRIES: usize = 1024;
pub const MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    schema_version: u32,
    root_source_id: String,
    sources: BTreeMap<String, String>,
}

impl WorkspaceSnapshot {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn root_source_id(&self) -> &str {
        &self.root_source_id
    }

    pub fn sources(&self) -> &BTreeMap<String, String> {
        &self.sources
    }

    pub fn source(&self, source_id: &str) -> Option<&str> {
        self.sources.get(source_id).map(String::as_str)
    }

    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    pub fn total_source_bytes(&self) -> usize {
        self.sources.values().map(String::len).sum()
    }

    pub fn map_provider(&self) -> MapTextSourceProvider {
        MapTextSourceProvider::from_sources(
            self.sources
                .iter()
                .map(|(source_id, text)| (source_id.as_str(), text.clone())),
        )
        .expect("validated workspace snapshot contains canonical logical source ids")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSnapshotBuildError {
    InvalidRootSourceId {
        source_id: String,
        message: String,
    },
    SourceRead {
        source_id: String,
        kind: TextSourceErrorKind,
    },
    InvalidManifest {
        source_id: String,
        message: String,
    },
    DependencyEscapesRoot {
        source_id: String,
        message: String,
    },
    TooManyEntries {
        limit: usize,
    },
    TooManySourceBytes {
        limit: usize,
    },
    SourceIdTooLong {
        source_id: String,
        limit: usize,
    },
}

impl fmt::Display for WorkspaceSnapshotBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRootSourceId { source_id, message } => write!(
                f,
                "invalid workspace root source id '{source_id}': {message}"
            ),
            Self::SourceRead { source_id, kind } => write!(
                f,
                "workspace source '{source_id}' could not be read ({})",
                kind.as_str()
            ),
            Self::InvalidManifest { source_id, message } => {
                write!(f, "workspace manifest '{source_id}' is invalid: {message}")
            }
            Self::DependencyEscapesRoot { source_id, message } => write!(
                f,
                "workspace dependency from '{source_id}' escapes the logical root: {message}"
            ),
            Self::TooManyEntries { limit } => {
                write!(f, "workspace snapshot exceeds maximum of {limit} sources")
            }
            Self::TooManySourceBytes { limit } => write!(
                f,
                "workspace snapshot exceeds maximum of {limit} embedded source bytes"
            ),
            Self::SourceIdTooLong { source_id, limit } => write!(
                f,
                "workspace source id '{source_id}' exceeds maximum of {limit} bytes"
            ),
        }
    }
}

impl std::error::Error for WorkspaceSnapshotBuildError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceSnapshotParseErrorKind {
    MissingHeader,
    UnsupportedVersion {
        version: u32,
    },
    InvalidHeader {
        message: String,
    },
    InvalidNumber {
        field: &'static str,
    },
    Truncated {
        section: &'static str,
    },
    MissingFrameTerminator {
        section: &'static str,
    },
    InvalidUtf8Frame {
        section: &'static str,
    },
    NonCanonicalSourceId {
        source_id: String,
    },
    DuplicateSourceId {
        source_id: String,
    },
    RootMissing {
        source_id: String,
    },
    MissingDependency {
        source_id: String,
    },
    UnexpectedSource {
        source_id: String,
    },
    InvalidEmbeddedManifest {
        source_id: String,
        message: String,
    },
    EntryCountMismatch {
        expected: usize,
        actual: usize,
    },
    SourceByteCountMismatch {
        expected: usize,
        actual: usize,
    },
    TooManyEntries {
        limit: usize,
    },
    TooManySourceBytes {
        limit: usize,
    },
    SourceIdTooLong {
        limit: usize,
    },
    TrailingPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshotParseError {
    offset: usize,
    kind: WorkspaceSnapshotParseErrorKind,
}

impl WorkspaceSnapshotParseError {
    fn new(offset: usize, kind: WorkspaceSnapshotParseErrorKind) -> Self {
        Self { offset, kind }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn kind(&self) -> &WorkspaceSnapshotParseErrorKind {
        &self.kind
    }
}

impl fmt::Display for WorkspaceSnapshotParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "workspace snapshot parse error at byte offset {}: ",
            self.offset
        )?;
        match &self.kind {
            WorkspaceSnapshotParseErrorKind::MissingHeader => {
                write!(f, "missing snapshot header")
            }
            WorkspaceSnapshotParseErrorKind::UnsupportedVersion { version } => write!(
                f,
                "unsupported workspace snapshot schema version {version}; expected {WORKSPACE_SNAPSHOT_SCHEMA_VERSION}"
            ),
            WorkspaceSnapshotParseErrorKind::InvalidHeader { message } => f.write_str(message),
            WorkspaceSnapshotParseErrorKind::InvalidNumber { field } => {
                write!(f, "{field} must be a non-negative decimal integer")
            }
            WorkspaceSnapshotParseErrorKind::Truncated { section } => {
                write!(f, "truncated {section}")
            }
            WorkspaceSnapshotParseErrorKind::MissingFrameTerminator { section } => {
                write!(f, "{section} is not followed by the required newline terminator")
            }
            WorkspaceSnapshotParseErrorKind::InvalidUtf8Frame { section } => {
                write!(f, "{section} byte length splits invalid UTF-8")
            }
            WorkspaceSnapshotParseErrorKind::NonCanonicalSourceId { source_id } => write!(
                f,
                "source id '{source_id}' is not a canonical logical workspace id"
            ),
            WorkspaceSnapshotParseErrorKind::DuplicateSourceId { source_id } => {
                write!(f, "duplicate workspace source id '{source_id}'")
            }
            WorkspaceSnapshotParseErrorKind::RootMissing { source_id } => {
                write!(f, "workspace root source '{source_id}' is not embedded")
            }
            WorkspaceSnapshotParseErrorKind::MissingDependency { source_id } => {
                write!(f, "workspace dependency '{source_id}' is not embedded")
            }
            WorkspaceSnapshotParseErrorKind::UnexpectedSource { source_id } => write!(
                f,
                "workspace snapshot contains unrelated source '{source_id}'"
            ),
            WorkspaceSnapshotParseErrorKind::InvalidEmbeddedManifest {
                source_id,
                message,
            } => write!(
                f,
                "embedded workspace manifest '{source_id}' is invalid: {message}"
            ),
            WorkspaceSnapshotParseErrorKind::EntryCountMismatch { expected, actual } => write!(
                f,
                "workspace entry count mismatch: declared {expected}, parsed {actual}"
            ),
            WorkspaceSnapshotParseErrorKind::SourceByteCountMismatch { expected, actual } => write!(
                f,
                "workspace source byte count mismatch: declared {expected}, parsed {actual}"
            ),
            WorkspaceSnapshotParseErrorKind::TooManyEntries { limit } => {
                write!(f, "workspace snapshot exceeds maximum of {limit} sources")
            }
            WorkspaceSnapshotParseErrorKind::TooManySourceBytes { limit } => write!(
                f,
                "workspace snapshot exceeds maximum of {limit} embedded source bytes"
            ),
            WorkspaceSnapshotParseErrorKind::SourceIdTooLong { limit } => {
                write!(f, "workspace source id exceeds maximum of {limit} bytes")
            }
            WorkspaceSnapshotParseErrorKind::TrailingPayload => {
                write!(f, "unexpected trailing payload after snapshot end marker")
            }
        }
    }
}

impl std::error::Error for WorkspaceSnapshotParseError {}

pub fn create_workspace_snapshot(
    provider: &dyn TextSourceProvider,
    root_source_id: &str,
) -> Result<WorkspaceSnapshot, WorkspaceSnapshotBuildError> {
    let root_source_id = normalize_workspace_source_id(root_source_id).map_err(|error| {
        WorkspaceSnapshotBuildError::InvalidRootSourceId {
            source_id: root_source_id.to_owned(),
            message: error.to_string(),
        }
    })?;
    ensure_build_source_id_len(&root_source_id)?;

    let mut sources = BTreeMap::new();
    let mut total_bytes = 0usize;
    let root_text = read_build_source(
        provider,
        &root_source_id,
        &mut sources,
        &mut total_bytes,
    )?;
    let suite = parse_orchestration_suite(&root_text).map_err(|error| {
        WorkspaceSnapshotBuildError::InvalidManifest {
            source_id: root_source_id.clone(),
            message: error.to_string(),
        }
    })?;

    for entry in suite.entries() {
        let job_source_id =
            resolve_build_dependency(&root_source_id, entry.job_path(), &root_source_id)?;
        let job_text = read_build_source(
            provider,
            &job_source_id,
            &mut sources,
            &mut total_bytes,
        )?;
        for dependency in parse_job_dependencies(entry.family(), &job_source_id, &job_text)
            .map_err(|message| WorkspaceSnapshotBuildError::InvalidManifest {
                source_id: job_source_id.clone(),
                message,
            })?
        {
            let dependency_source_id =
                resolve_build_dependency(&job_source_id, &dependency, &job_source_id)?;
            let _ = read_build_source(
                provider,
                &dependency_source_id,
                &mut sources,
                &mut total_bytes,
            )?;
        }
    }

    let snapshot = WorkspaceSnapshot {
        schema_version: WORKSPACE_SNAPSHOT_SCHEMA_VERSION,
        root_source_id,
        sources,
    };
    debug_assert!(validate_snapshot_closure(&snapshot).is_ok());
    Ok(snapshot)
}

pub fn render_workspace_snapshot(snapshot: &WorkspaceSnapshot) -> String {
    let mut output = String::new();
    output.push_str("fvlab-workspace-snapshot ");
    output.push_str(&snapshot.schema_version.to_string());
    output.push('\n');

    output.push_str("root ");
    output.push_str(&snapshot.root_source_id.len().to_string());
    output.push('\n');
    output.push_str(&snapshot.root_source_id);
    output.push('\n');

    output.push_str("entries ");
    output.push_str(&snapshot.sources.len().to_string());
    output.push(' ');
    output.push_str(&snapshot.total_source_bytes().to_string());
    output.push('\n');

    for (source_id, text) in &snapshot.sources {
        output.push_str("source ");
        output.push_str(&source_id.len().to_string());
        output.push(' ');
        output.push_str(&text.len().to_string());
        output.push('\n');
        output.push_str(source_id);
        output.push('\n');
        output.push_str(text);
        output.push('\n');
    }
    output.push_str("end\n");
    output
}

pub fn parse_workspace_snapshot(
    input: &str,
) -> Result<WorkspaceSnapshot, WorkspaceSnapshotParseError> {
    let mut cursor = SnapshotCursor::new(input);

    let header_offset = cursor.position();
    let header = cursor.read_line("snapshot header")?;
    let mut header_parts = header.split_ascii_whitespace();
    if header_parts.next() != Some("fvlab-workspace-snapshot") {
        return Err(WorkspaceSnapshotParseError::new(
            header_offset,
            WorkspaceSnapshotParseErrorKind::MissingHeader,
        ));
    }
    let version = parse_u32_field(
        header_parts.next(),
        header_offset,
        "schema version",
    )?;
    if header_parts.next().is_some() {
        return Err(WorkspaceSnapshotParseError::new(
            header_offset,
            WorkspaceSnapshotParseErrorKind::InvalidHeader {
                message: "snapshot header has unexpected trailing fields".to_owned(),
            },
        ));
    }
    if version != WORKSPACE_SNAPSHOT_SCHEMA_VERSION {
        return Err(WorkspaceSnapshotParseError::new(
            header_offset,
            WorkspaceSnapshotParseErrorKind::UnsupportedVersion { version },
        ));
    }

    let root_header_offset = cursor.position();
    let root_header = cursor.read_line("root header")?;
    let mut root_parts = root_header.split_ascii_whitespace();
    if root_parts.next() != Some("root") {
        return Err(WorkspaceSnapshotParseError::new(
            root_header_offset,
            WorkspaceSnapshotParseErrorKind::InvalidHeader {
                message: "expected root header".to_owned(),
            },
        ));
    }
    let root_len = parse_usize_field(root_parts.next(), root_header_offset, "root byte length")?;
    if root_parts.next().is_some() {
        return Err(WorkspaceSnapshotParseError::new(
            root_header_offset,
            WorkspaceSnapshotParseErrorKind::InvalidHeader {
                message: "root header has unexpected trailing fields".to_owned(),
            },
        ));
    }
    if root_len > MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES {
        return Err(WorkspaceSnapshotParseError::new(
            root_header_offset,
            WorkspaceSnapshotParseErrorKind::SourceIdTooLong {
                limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES,
            },
        ));
    }
    let root_source_id = cursor.read_frame(root_len, "root source id")?.to_owned();
    validate_parsed_source_id(&root_source_id, root_header_offset)?;

    let entries_header_offset = cursor.position();
    let entries_header = cursor.read_line("entries header")?;
    let mut entries_parts = entries_header.split_ascii_whitespace();
    if entries_parts.next() != Some("entries") {
        return Err(WorkspaceSnapshotParseError::new(
            entries_header_offset,
            WorkspaceSnapshotParseErrorKind::InvalidHeader {
                message: "expected entries header".to_owned(),
            },
        ));
    }
    let expected_entries =
        parse_usize_field(entries_parts.next(), entries_header_offset, "entry count")?;
    let expected_source_bytes =
        parse_usize_field(entries_parts.next(), entries_header_offset, "source byte count")?;
    if entries_parts.next().is_some() {
        return Err(WorkspaceSnapshotParseError::new(
            entries_header_offset,
            WorkspaceSnapshotParseErrorKind::InvalidHeader {
                message: "entries header has unexpected trailing fields".to_owned(),
            },
        ));
    }
    if expected_entries > MAX_WORKSPACE_SNAPSHOT_ENTRIES {
        return Err(WorkspaceSnapshotParseError::new(
            entries_header_offset,
            WorkspaceSnapshotParseErrorKind::TooManyEntries {
                limit: MAX_WORKSPACE_SNAPSHOT_ENTRIES,
            },
        ));
    }
    if expected_source_bytes > MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES {
        return Err(WorkspaceSnapshotParseError::new(
            entries_header_offset,
            WorkspaceSnapshotParseErrorKind::TooManySourceBytes {
                limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES,
            },
        ));
    }

    let mut sources = BTreeMap::new();
    let mut actual_source_bytes = 0usize;
    for _ in 0..expected_entries {
        let source_header_offset = cursor.position();
        let source_header = cursor.read_line("source header")?;
        let mut source_parts = source_header.split_ascii_whitespace();
        if source_parts.next() != Some("source") {
            return Err(WorkspaceSnapshotParseError::new(
                source_header_offset,
                WorkspaceSnapshotParseErrorKind::InvalidHeader {
                    message: "expected source header".to_owned(),
                },
            ));
        }
        let source_id_len =
            parse_usize_field(source_parts.next(), source_header_offset, "source id byte length")?;
        let text_len =
            parse_usize_field(source_parts.next(), source_header_offset, "source text byte length")?;
        if source_parts.next().is_some() {
            return Err(WorkspaceSnapshotParseError::new(
                source_header_offset,
                WorkspaceSnapshotParseErrorKind::InvalidHeader {
                    message: "source header has unexpected trailing fields".to_owned(),
                },
            ));
        }
        if source_id_len > MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES {
            return Err(WorkspaceSnapshotParseError::new(
                source_header_offset,
                WorkspaceSnapshotParseErrorKind::SourceIdTooLong {
                    limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES,
                },
            ));
        }
        actual_source_bytes = actual_source_bytes.checked_add(text_len).ok_or_else(|| {
            WorkspaceSnapshotParseError::new(
                source_header_offset,
                WorkspaceSnapshotParseErrorKind::TooManySourceBytes {
                    limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES,
                },
            )
        })?;
        if actual_source_bytes > MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES {
            return Err(WorkspaceSnapshotParseError::new(
                source_header_offset,
                WorkspaceSnapshotParseErrorKind::TooManySourceBytes {
                    limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES,
                },
            ));
        }

        let source_id = cursor.read_frame(source_id_len, "source id")?.to_owned();
        validate_parsed_source_id(&source_id, source_header_offset)?;
        let text = cursor.read_frame(text_len, "source text")?.to_owned();
        if sources.insert(source_id.clone(), text).is_some() {
            return Err(WorkspaceSnapshotParseError::new(
                source_header_offset,
                WorkspaceSnapshotParseErrorKind::DuplicateSourceId { source_id },
            ));
        }
    }

    if sources.len() != expected_entries {
        return Err(WorkspaceSnapshotParseError::new(
            cursor.position(),
            WorkspaceSnapshotParseErrorKind::EntryCountMismatch {
                expected: expected_entries,
                actual: sources.len(),
            },
        ));
    }
    if actual_source_bytes != expected_source_bytes {
        return Err(WorkspaceSnapshotParseError::new(
            cursor.position(),
            WorkspaceSnapshotParseErrorKind::SourceByteCountMismatch {
                expected: expected_source_bytes,
                actual: actual_source_bytes,
            },
        ));
    }

    let end_offset = cursor.position();
    if cursor.read_line("end marker")? != "end" {
        return Err(WorkspaceSnapshotParseError::new(
            end_offset,
            WorkspaceSnapshotParseErrorKind::InvalidHeader {
                message: "expected snapshot end marker".to_owned(),
            },
        ));
    }
    if !cursor.is_eof() {
        return Err(WorkspaceSnapshotParseError::new(
            cursor.position(),
            WorkspaceSnapshotParseErrorKind::TrailingPayload,
        ));
    }

    let snapshot = WorkspaceSnapshot {
        schema_version: version,
        root_source_id,
        sources,
    };
    if !snapshot.sources.contains_key(&snapshot.root_source_id) {
        return Err(WorkspaceSnapshotParseError::new(
            0,
            WorkspaceSnapshotParseErrorKind::RootMissing {
                source_id: snapshot.root_source_id.clone(),
            },
        ));
    }
    validate_snapshot_closure(&snapshot)?;
    Ok(snapshot)
}

pub fn replay_workspace_snapshot_json(snapshot: &WorkspaceSnapshot) -> OrchestrationSuiteJsonRun {
    let provider = snapshot.map_provider();
    run_orchestration_suite_json_with_provider(&provider, snapshot.root_source_id())
}

pub fn replay_workspace_snapshot_expectations_json(
    snapshot: &WorkspaceSnapshot,
) -> OrchestrationRegressionSuiteJsonRun {
    let provider = snapshot.map_provider();
    run_orchestration_suite_expectations_json_with_provider(&provider, snapshot.root_source_id())
}

fn read_build_source(
    provider: &dyn TextSourceProvider,
    source_id: &str,
    sources: &mut BTreeMap<String, String>,
    total_bytes: &mut usize,
) -> Result<String, WorkspaceSnapshotBuildError> {
    if let Some(text) = sources.get(source_id) {
        return Ok(text.clone());
    }
    if sources.len() >= MAX_WORKSPACE_SNAPSHOT_ENTRIES {
        return Err(WorkspaceSnapshotBuildError::TooManyEntries {
            limit: MAX_WORKSPACE_SNAPSHOT_ENTRIES,
        });
    }
    ensure_build_source_id_len(source_id)?;
    let text = provider.read_text(source_id).map_err(|error| {
        WorkspaceSnapshotBuildError::SourceRead {
            source_id: source_id.to_owned(),
            kind: error.kind(),
        }
    })?;
    *total_bytes = total_bytes.checked_add(text.len()).ok_or(
        WorkspaceSnapshotBuildError::TooManySourceBytes {
            limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES,
        },
    )?;
    if *total_bytes > MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES {
        return Err(WorkspaceSnapshotBuildError::TooManySourceBytes {
            limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_BYTES,
        });
    }
    sources.insert(source_id.to_owned(), text.clone());
    Ok(text)
}

fn ensure_build_source_id_len(source_id: &str) -> Result<(), WorkspaceSnapshotBuildError> {
    if source_id.len() > MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES {
        Err(WorkspaceSnapshotBuildError::SourceIdTooLong {
            source_id: source_id.to_owned(),
            limit: MAX_WORKSPACE_SNAPSHOT_SOURCE_ID_BYTES,
        })
    } else {
        Ok(())
    }
}

fn resolve_build_dependency(
    parent_source_id: &str,
    declared_source_id: &str,
    error_source_id: &str,
) -> Result<String, WorkspaceSnapshotBuildError> {
    let resolved = resolve_source_id(parent_source_id, declared_source_id).map_err(|error| {
        WorkspaceSnapshotBuildError::DependencyEscapesRoot {
            source_id: error_source_id.to_owned(),
            message: error.to_string(),
        }
    })?;
    normalize_workspace_source_id(&resolved).map_err(|error| {
        WorkspaceSnapshotBuildError::DependencyEscapesRoot {
            source_id: error_source_id.to_owned(),
            message: error.to_string(),
        }
    })
}

fn parse_job_dependencies(
    family: OrchestrationJobFamily,
    source_id: &str,
    input: &str,
) -> Result<Vec<String>, String> {
    match family {
        OrchestrationJobFamily::Verification => {
            let job = parse_verification_job(input).map_err(|error| error.to_string())?;
            Ok(vec![
                job.model_path().to_owned(),
                job.property_path().to_owned(),
            ])
        }
        OrchestrationJobFamily::Structural => {
            let job = parse_structural_job(input).map_err(|error| error.to_string())?;
            Ok(vec![job.model_path().to_owned()])
        }
        OrchestrationJobFamily::CertificateVerification => {
            let job =
                parse_certificate_verification_job(input).map_err(|error| error.to_string())?;
            Ok(vec![
                job.model_path().to_owned(),
                job.property_path().to_owned(),
                job.certificate_path().to_owned(),
            ])
        }
    }
    .map_err(|message| format!("{source_id}: {message}"))
}

fn validate_snapshot_closure(
    snapshot: &WorkspaceSnapshot,
) -> Result<(), WorkspaceSnapshotParseError> {
    let root_text = snapshot
        .sources
        .get(&snapshot.root_source_id)
        .ok_or_else(|| {
            WorkspaceSnapshotParseError::new(
                0,
                WorkspaceSnapshotParseErrorKind::RootMissing {
                    source_id: snapshot.root_source_id.clone(),
                },
            )
        })?;
    let suite = parse_orchestration_suite(root_text).map_err(|error| {
        WorkspaceSnapshotParseError::new(
            0,
            WorkspaceSnapshotParseErrorKind::InvalidEmbeddedManifest {
                source_id: snapshot.root_source_id.clone(),
                message: error.to_string(),
            },
        )
    })?;

    let mut expected = BTreeSet::new();
    expected.insert(snapshot.root_source_id.clone());

    for entry in suite.entries() {
        let job_source_id =
            resolve_snapshot_dependency(&snapshot.root_source_id, entry.job_path())?;
        expected.insert(job_source_id.clone());
        let job_text = snapshot.sources.get(&job_source_id).ok_or_else(|| {
            WorkspaceSnapshotParseError::new(
                0,
                WorkspaceSnapshotParseErrorKind::MissingDependency {
                    source_id: job_source_id.clone(),
                },
            )
        })?;
        let dependencies =
            parse_job_dependencies(entry.family(), &job_source_id, job_text).map_err(|message| {
                WorkspaceSnapshotParseError::new(
                    0,
                    WorkspaceSnapshotParseErrorKind::InvalidEmbeddedManifest {
                        source_id: job_source_id.clone(),
                        message,
                    },
                )
            })?;
        for dependency in dependencies {
            let dependency_source_id =
                resolve_snapshot_dependency(&job_source_id, &dependency)?;
            expected.insert(dependency_source_id.clone());
            if !snapshot.sources.contains_key(&dependency_source_id) {
                return Err(WorkspaceSnapshotParseError::new(
                    0,
                    WorkspaceSnapshotParseErrorKind::MissingDependency {
                        source_id: dependency_source_id,
                    },
                ));
            }
        }
    }

    if let Some(unexpected) = snapshot
        .sources
        .keys()
        .find(|source_id| !expected.contains(*source_id))
    {
        return Err(WorkspaceSnapshotParseError::new(
            0,
            WorkspaceSnapshotParseErrorKind::UnexpectedSource {
                source_id: unexpected.clone(),
            },
        ));
    }
    Ok(())
}

fn resolve_snapshot_dependency(
    parent_source_id: &str,
    declared_source_id: &str,
) -> Result<String, WorkspaceSnapshotParseError> {
    let resolved = resolve_source_id(parent_source_id, declared_source_id).map_err(|error| {
        WorkspaceSnapshotParseError::new(
            0,
            WorkspaceSnapshotParseErrorKind::NonCanonicalSourceId {
                source_id: error.source_id().to_owned(),
            },
        )
    })?;
    normalize_workspace_source_id(&resolved).map_err(|_| {
        WorkspaceSnapshotParseError::new(
            0,
            WorkspaceSnapshotParseErrorKind::NonCanonicalSourceId {
                source_id: resolved,
            },
        )
    })
}

fn validate_parsed_source_id(
    source_id: &str,
    offset: usize,
) -> Result<(), WorkspaceSnapshotParseError> {
    let normalized = normalize_workspace_source_id(source_id).map_err(|_| {
        WorkspaceSnapshotParseError::new(
            offset,
            WorkspaceSnapshotParseErrorKind::NonCanonicalSourceId {
                source_id: source_id.to_owned(),
            },
        )
    })?;
    if normalized != source_id {
        return Err(WorkspaceSnapshotParseError::new(
            offset,
            WorkspaceSnapshotParseErrorKind::NonCanonicalSourceId {
                source_id: source_id.to_owned(),
            },
        ));
    }
    Ok(())
}

fn parse_u32_field(
    value: Option<&str>,
    offset: usize,
    field: &'static str,
) -> Result<u32, WorkspaceSnapshotParseError> {
    value
        .ok_or_else(|| {
            WorkspaceSnapshotParseError::new(
                offset,
                WorkspaceSnapshotParseErrorKind::InvalidNumber { field },
            )
        })?
        .parse::<u32>()
        .map_err(|_| {
            WorkspaceSnapshotParseError::new(
                offset,
                WorkspaceSnapshotParseErrorKind::InvalidNumber { field },
            )
        })
}

fn parse_usize_field(
    value: Option<&str>,
    offset: usize,
    field: &'static str,
) -> Result<usize, WorkspaceSnapshotParseError> {
    value
        .ok_or_else(|| {
            WorkspaceSnapshotParseError::new(
                offset,
                WorkspaceSnapshotParseErrorKind::InvalidNumber { field },
            )
        })?
        .parse::<usize>()
        .map_err(|_| {
            WorkspaceSnapshotParseError::new(
                offset,
                WorkspaceSnapshotParseErrorKind::InvalidNumber { field },
            )
        })
}

struct SnapshotCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> SnapshotCursor<'a> {
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
    ) -> Result<&'a str, WorkspaceSnapshotParseError> {
        let start = self.position;
        let Some(relative_end) = self.bytes[start..].iter().position(|byte| *byte == b'\n') else {
            return Err(WorkspaceSnapshotParseError::new(
                start,
                WorkspaceSnapshotParseErrorKind::Truncated { section },
            ));
        };
        let end = start + relative_end;
        self.position = end + 1;
        str::from_utf8(&self.bytes[start..end]).map_err(|_| {
            WorkspaceSnapshotParseError::new(
                start,
                WorkspaceSnapshotParseErrorKind::InvalidUtf8Frame { section },
            )
        })
    }

    fn read_frame(
        &mut self,
        length: usize,
        section: &'static str,
    ) -> Result<&'a str, WorkspaceSnapshotParseError> {
        let start = self.position;
        let end = start.checked_add(length).ok_or_else(|| {
            WorkspaceSnapshotParseError::new(
                start,
                WorkspaceSnapshotParseErrorKind::Truncated { section },
            )
        })?;
        if end > self.bytes.len() {
            return Err(WorkspaceSnapshotParseError::new(
                start,
                WorkspaceSnapshotParseErrorKind::Truncated { section },
            ));
        }
        let frame = str::from_utf8(&self.bytes[start..end]).map_err(|_| {
            WorkspaceSnapshotParseError::new(
                start,
                WorkspaceSnapshotParseErrorKind::InvalidUtf8Frame { section },
            )
        })?;
        self.position = end;
        if self.bytes.get(self.position) != Some(&b'\n') {
            return Err(WorkspaceSnapshotParseError::new(
                self.position,
                WorkspaceSnapshotParseErrorKind::MissingFrameTerminator { section },
            ));
        }
        self.position += 1;
        Ok(frame)
    }
}

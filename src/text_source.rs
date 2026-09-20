use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSourceErrorKind {
    NotFound,
    PermissionDenied,
    InvalidData,
    IsDirectory,
    Other,
}

impl TextSourceErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "not-found",
            Self::PermissionDenied => "permission-denied",
            Self::InvalidData => "invalid-data",
            Self::IsDirectory => "is-directory",
            Self::Other => "io-error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSourceError {
    source_id: String,
    kind: TextSourceErrorKind,
}

impl TextSourceError {
    pub fn new(source_id: impl Into<String>, kind: TextSourceErrorKind) -> Self {
        Self {
            source_id: source_id.into(),
            kind,
        }
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn kind(&self) -> TextSourceErrorKind {
        self.kind
    }
}

impl fmt::Display for TextSourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "source '{}' could not be read ({})",
            self.source_id,
            self.kind.as_str()
        )
    }
}

impl std::error::Error for TextSourceError {}

pub trait TextSourceProvider {
    fn read_text(&self, source_id: &str) -> Result<String, TextSourceError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FileSystemTextSourceProvider;

impl TextSourceProvider for FileSystemTextSourceProvider {
    fn read_text(&self, source_id: &str) -> Result<String, TextSourceError> {
        fs::read_to_string(Path::new(source_id))
            .map_err(|error| TextSourceError::new(source_id, classify_io_error(&error)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootedFileSystemTextSourceProvider {
    root: PathBuf,
}

impl RootedFileSystemTextSourceProvider {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl TextSourceProvider for RootedFileSystemTextSourceProvider {
    fn read_text(&self, source_id: &str) -> Result<String, TextSourceError> {
        let normalized = normalize_workspace_source_id(source_id).map_err(|error| {
            TextSourceError::new(error.source_id().to_owned(), TextSourceErrorKind::InvalidData)
        })?;
        fs::read_to_string(self.root.join(source_id_to_path(&normalized)))
            .map_err(|error| TextSourceError::new(normalized, classify_io_error(&error)))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapTextSourceProvider {
    sources: BTreeMap<String, String>,
}

impl MapTextSourceProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_sources<I, K, V>(sources: I) -> Result<Self, TextSourceIdError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: Into<String>,
    {
        let mut provider = Self::new();
        for (source_id, text) in sources {
            provider.insert(source_id.as_ref(), text)?;
        }
        Ok(provider)
    }

    pub fn insert(
        &mut self,
        source_id: impl AsRef<str>,
        text: impl Into<String>,
    ) -> Result<Option<String>, TextSourceIdError> {
        let source_id = normalize_workspace_source_id(source_id.as_ref())?;
        Ok(self.sources.insert(source_id, text.into()))
    }

    pub fn contains(&self, source_id: &str) -> bool {
        normalize_workspace_source_id(source_id)
            .ok()
            .is_some_and(|source_id| self.sources.contains_key(&source_id))
    }
}

impl TextSourceProvider for MapTextSourceProvider {
    fn read_text(&self, source_id: &str) -> Result<String, TextSourceError> {
        let normalized = normalize_workspace_source_id(source_id).map_err(|error| {
            TextSourceError::new(error.source_id().to_owned(), TextSourceErrorKind::InvalidData)
        })?;
        self.sources
            .get(&normalized)
            .cloned()
            .ok_or_else(|| TextSourceError::new(normalized, TextSourceErrorKind::NotFound))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextSourceIdErrorKind {
    Empty,
    EscapesRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSourceIdError {
    source_id: String,
    kind: TextSourceIdErrorKind,
}

impl TextSourceIdError {
    fn new(source_id: impl Into<String>, kind: TextSourceIdErrorKind) -> Self {
        Self {
            source_id: source_id.into(),
            kind,
        }
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn kind(&self) -> &TextSourceIdErrorKind {
        &self.kind
    }
}

impl fmt::Display for TextSourceIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            TextSourceIdErrorKind::Empty => write!(f, "source identity must not be empty"),
            TextSourceIdErrorKind::EscapesRoot => write!(
                f,
                "source identity '{}' escapes the logical workspace root",
                self.source_id
            ),
        }
    }
}

impl std::error::Error for TextSourceIdError {}

pub fn normalize_source_id(source_id: &str) -> Result<String, TextSourceIdError> {
    if source_id.is_empty() {
        return Err(TextSourceIdError::new(
            source_id,
            TextSourceIdErrorKind::Empty,
        ));
    }

    let source_id = source_id.replace('\\', "/");
    let absolute = source_id.starts_with('/');
    let drive_prefix = source_id
        .as_bytes()
        .get(1)
        .is_some_and(|byte| *byte == b':')
        .then(|| source_id[..2].to_owned());

    let body = if drive_prefix.is_some() {
        source_id[2..].trim_start_matches('/')
    } else {
        source_id.trim_start_matches('/')
    };

    let mut components = Vec::new();
    for component in body.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components
                    .last()
                    .is_some_and(|component| *component != "..")
                {
                    components.pop();
                } else if absolute || drive_prefix.is_some() {
                    return Err(TextSourceIdError::new(
                        source_id,
                        TextSourceIdErrorKind::EscapesRoot,
                    ));
                } else {
                    components.push("..");
                }
            }
            other => components.push(other),
        }
    }

    let joined = components.join("/");
    if let Some(prefix) = drive_prefix {
        if joined.is_empty() {
            Ok(format!("{prefix}/"))
        } else {
            Ok(format!("{prefix}/{joined}"))
        }
    } else if absolute {
        Ok(format!("/{joined}"))
    } else if joined.is_empty() {
        Ok(".".to_owned())
    } else {
        Ok(joined)
    }
}

fn normalize_workspace_source_id(source_id: &str) -> Result<String, TextSourceIdError> {
    let normalized = normalize_source_id(source_id)?;
    if is_absolute_source_id(&normalized) || escapes_logical_root(&normalized) {
        Err(TextSourceIdError::new(
            normalized,
            TextSourceIdErrorKind::EscapesRoot,
        ))
    } else {
        Ok(normalized)
    }
}

pub fn path_source_id(path: &Path) -> Result<String, TextSourceIdError> {
    normalize_source_id(&path.to_string_lossy())
}

pub fn resolve_source_id(
    manifest_source_id: &str,
    declared_source_id: &str,
) -> Result<String, TextSourceIdError> {
    let manifest_source_id = normalize_source_id(manifest_source_id)?;
    let declared_source_id = declared_source_id.replace('\\', "/");
    if is_absolute_source_id(&declared_source_id) {
        return normalize_source_id(&declared_source_id);
    }

    let base = manifest_source_id
        .rsplit_once('/')
        .map_or(".", |(base, _)| if base.is_empty() { "/" } else { base });
    normalize_source_id(&format!("{base}/{declared_source_id}"))
}

fn is_absolute_source_id(source_id: &str) -> bool {
    source_id.starts_with('/')
        || source_id
            .as_bytes()
            .get(1)
            .is_some_and(|byte| *byte == b':')
}

fn escapes_logical_root(source_id: &str) -> bool {
    source_id == ".." || source_id.starts_with("../")
}

fn source_id_to_path(source_id: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for component in source_id.split('/') {
        if !component.is_empty() && component != "." {
            path.push(component);
        }
    }
    path
}

fn classify_io_error(error: &io::Error) -> TextSourceErrorKind {
    match error.kind() {
        io::ErrorKind::NotFound => TextSourceErrorKind::NotFound,
        io::ErrorKind::PermissionDenied => TextSourceErrorKind::PermissionDenied,
        io::ErrorKind::InvalidData => TextSourceErrorKind::InvalidData,
        io::ErrorKind::IsADirectory => TextSourceErrorKind::IsDirectory,
        _ => TextSourceErrorKind::Other,
    }
}

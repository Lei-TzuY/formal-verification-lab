use std::collections::{BTreeMap, BTreeSet};

pub const MAX_REPLAY_JSON_DIAGNOSTIC_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_REPLAY_JSON_DIAGNOSTIC_DEPTH: usize = 128;
pub const MAX_REPLAY_JSON_DIAGNOSTIC_NODES: usize = 262_144;
pub const MAX_REPLAY_JSON_DIAGNOSTIC_PREVIEW_BYTES: usize = 160;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceReplayJsonDifferenceKind {
    ScalarValue,
    TypeChange,
    MissingMember,
    UnexpectedMember,
    ArrayLength,
    ByteOnly,
    ByteFallback,
}

impl WorkspaceReplayJsonDifferenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScalarValue => "scalar_value",
            Self::TypeChange => "type_change",
            Self::MissingMember => "missing_member",
            Self::UnexpectedMember => "unexpected_member",
            Self::ArrayLength => "array_length",
            Self::ByteOnly => "byte_only",
            Self::ByteFallback => "byte_fallback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplayJsonDifference {
    pub kind: WorkspaceReplayJsonDifferenceKind,
    pub path: Option<String>,
    pub first_byte_offset: usize,
    pub expected_preview: Option<String>,
    pub actual_preview: Option<String>,
}

pub fn diagnose_replay_json_drift(
    expected: &str,
    actual: &str,
) -> Option<WorkspaceReplayJsonDifference> {
    if expected == actual {
        return None;
    }

    let first_byte_offset = first_byte_difference(expected.as_bytes(), actual.as_bytes());
    if expected.len() > MAX_REPLAY_JSON_DIAGNOSTIC_BYTES
        || actual.len() > MAX_REPLAY_JSON_DIAGNOSTIC_BYTES
    {
        return Some(byte_fallback(first_byte_offset));
    }

    let expected_value = DiagnosticJsonParser::parse(expected);
    let actual_value = DiagnosticJsonParser::parse(actual);
    match (expected_value, actual_value) {
        (Ok(expected_value), Ok(actual_value)) => {
            if let Some(mut difference) =
                structural_difference(&expected_value, &actual_value, String::new())
            {
                difference.first_byte_offset = first_byte_offset;
                Some(difference)
            } else {
                Some(WorkspaceReplayJsonDifference {
                    kind: WorkspaceReplayJsonDifferenceKind::ByteOnly,
                    path: None,
                    first_byte_offset,
                    expected_preview: None,
                    actual_preview: None,
                })
            }
        }
        _ => Some(byte_fallback(first_byte_offset)),
    }
}

fn byte_fallback(first_byte_offset: usize) -> WorkspaceReplayJsonDifference {
    WorkspaceReplayJsonDifference {
        kind: WorkspaceReplayJsonDifferenceKind::ByteFallback,
        path: None,
        first_byte_offset,
        expected_preview: None,
        actual_preview: None,
    }
}

fn first_byte_difference(expected: &[u8], actual: &[u8]) -> usize {
    expected
        .iter()
        .zip(actual)
        .position(|(expected, actual)| expected != actual)
        .unwrap_or_else(|| expected.len().min(actual.len()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiagnosticJsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<DiagnosticJsonValue>),
    Object(BTreeMap<String, DiagnosticJsonValue>),
}

impl DiagnosticJsonValue {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
        }
    }

    fn preview(&self) -> String {
        let raw = match self {
            Self::Null => "null".to_owned(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.clone(),
            Self::String(value) => quote_preview(value),
            Self::Array(values) => format!("<array len={}>", values.len()),
            Self::Object(values) => format!("<object members={}>", values.len()),
        };
        truncate_preview(&raw)
    }
}

fn quote_preview(value: &str) -> String {
    let mut output = String::from(""");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", ch as u32));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn truncate_preview(value: &str) -> String {
    if value.len() <= MAX_REPLAY_JSON_DIAGNOSTIC_PREVIEW_BYTES {
        return value.to_owned();
    }

    let mut end = MAX_REPLAY_JSON_DIAGNOSTIC_PREVIEW_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    let mut output = value[..end].to_owned();
    output.push_str("...");
    output
}

fn structural_difference(
    expected: &DiagnosticJsonValue,
    actual: &DiagnosticJsonValue,
    path: String,
) -> Option<WorkspaceReplayJsonDifference> {
    match (expected, actual) {
        (DiagnosticJsonValue::Null, DiagnosticJsonValue::Null) => None,
        (DiagnosticJsonValue::Bool(expected), DiagnosticJsonValue::Bool(actual))
            if expected == actual =>
        {
            None
        }
        (DiagnosticJsonValue::Number(expected), DiagnosticJsonValue::Number(actual))
            if expected == actual =>
        {
            None
        }
        (DiagnosticJsonValue::String(expected), DiagnosticJsonValue::String(actual))
            if expected == actual =>
        {
            None
        }
        (DiagnosticJsonValue::Bool(_), DiagnosticJsonValue::Bool(_))
        | (DiagnosticJsonValue::Number(_), DiagnosticJsonValue::Number(_))
        | (DiagnosticJsonValue::String(_), DiagnosticJsonValue::String(_)) => {
            Some(value_difference(
                WorkspaceReplayJsonDifferenceKind::ScalarValue,
                path,
                expected,
                actual,
            ))
        }
        (DiagnosticJsonValue::Array(expected), DiagnosticJsonValue::Array(actual)) => {
            for index in 0..expected.len().min(actual.len()) {
                if let Some(difference) = structural_difference(
                    &expected[index],
                    &actual[index],
                    push_pointer_index(&path, index),
                ) {
                    return Some(difference);
                }
            }
            if expected.len() != actual.len() {
                return Some(WorkspaceReplayJsonDifference {
                    kind: WorkspaceReplayJsonDifferenceKind::ArrayLength,
                    path: Some(path),
                    first_byte_offset: 0,
                    expected_preview: Some(expected.len().to_string()),
                    actual_preview: Some(actual.len().to_string()),
                });
            }
            None
        }
        (DiagnosticJsonValue::Object(expected), DiagnosticJsonValue::Object(actual)) => {
            let keys = expected
                .keys()
                .chain(actual.keys())
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            for key in keys {
                let child_path = push_pointer_key(&path, key);
                match (expected.get(key), actual.get(key)) {
                    (Some(expected), Some(actual)) => {
                        if let Some(difference) =
                            structural_difference(expected, actual, child_path)
                        {
                            return Some(difference);
                        }
                    }
                    (Some(expected), None) => {
                        return Some(WorkspaceReplayJsonDifference {
                            kind: WorkspaceReplayJsonDifferenceKind::MissingMember,
                            path: Some(child_path),
                            first_byte_offset: 0,
                            expected_preview: Some(expected.preview()),
                            actual_preview: None,
                        });
                    }
                    (None, Some(actual)) => {
                        return Some(WorkspaceReplayJsonDifference {
                            kind: WorkspaceReplayJsonDifferenceKind::UnexpectedMember,
                            path: Some(child_path),
                            first_byte_offset: 0,
                            expected_preview: None,
                            actual_preview: Some(actual.preview()),
                        });
                    }
                    (None, None) => unreachable!("key came from at least one object"),
                }
            }
            None
        }
        _ => Some(WorkspaceReplayJsonDifference {
            kind: WorkspaceReplayJsonDifferenceKind::TypeChange,
            path: Some(path),
            first_byte_offset: 0,
            expected_preview: Some(expected.kind_name().to_owned()),
            actual_preview: Some(actual.kind_name().to_owned()),
        }),
    }
}

fn value_difference(
    kind: WorkspaceReplayJsonDifferenceKind,
    path: String,
    expected: &DiagnosticJsonValue,
    actual: &DiagnosticJsonValue,
) -> WorkspaceReplayJsonDifference {
    WorkspaceReplayJsonDifference {
        kind,
        path: Some(path),
        first_byte_offset: 0,
        expected_preview: Some(expected.preview()),
        actual_preview: Some(actual.preview()),
    }
}

fn push_pointer_key(path: &str, key: &str) -> String {
    let escaped = key.replace('~', "~0").replace('/', "~1");
    format!("{path}/{escaped}")
}

fn push_pointer_index(path: &str, index: usize) -> String {
    format!("{path}/{index}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticJsonParseError {
    Syntax,
    ResourceLimit,
}

struct DiagnosticJsonParser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    position: usize,
    nodes: usize,
}

impl<'a> DiagnosticJsonParser<'a> {
    fn parse(input: &'a str) -> Result<DiagnosticJsonValue, DiagnosticJsonParseError> {
        let mut parser = Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
            nodes: 0,
        };
        parser.skip_whitespace();
        let value = parser.parse_value(0)?;
        parser.skip_whitespace();
        if parser.position != parser.bytes.len() {
            return Err(DiagnosticJsonParseError::Syntax);
        }
        Ok(value)
    }

    fn parse_value(
        &mut self,
        depth: usize,
    ) -> Result<DiagnosticJsonValue, DiagnosticJsonParseError> {
        if depth > MAX_REPLAY_JSON_DIAGNOSTIC_DEPTH {
            return Err(DiagnosticJsonParseError::ResourceLimit);
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or(DiagnosticJsonParseError::ResourceLimit)?;
        if self.nodes > MAX_REPLAY_JSON_DIAGNOSTIC_NODES {
            return Err(DiagnosticJsonParseError::ResourceLimit);
        }

        match self.peek_byte() {
            Some(b'n') => {
                self.expect_bytes(b"null")?;
                Ok(DiagnosticJsonValue::Null)
            }
            Some(b't') => {
                self.expect_bytes(b"true")?;
                Ok(DiagnosticJsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_bytes(b"false")?;
                Ok(DiagnosticJsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(DiagnosticJsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => {
                self.parse_number().map(DiagnosticJsonValue::Number)
            }
            _ => Err(DiagnosticJsonParseError::Syntax),
        }
    }

    fn parse_array(
        &mut self,
        depth: usize,
    ) -> Result<DiagnosticJsonValue, DiagnosticJsonParseError> {
        self.expect_byte(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.consume_byte(b']') {
            return Ok(DiagnosticJsonValue::Array(values));
        }

        loop {
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
            self.skip_whitespace();
        }
        Ok(DiagnosticJsonValue::Array(values))
    }

    fn parse_object(
        &mut self,
        depth: usize,
    ) -> Result<DiagnosticJsonValue, DiagnosticJsonParseError> {
        self.expect_byte(b'{')?;
        self.skip_whitespace();
        let mut values = BTreeMap::new();
        if self.consume_byte(b'}') {
            return Ok(DiagnosticJsonValue::Object(values));
        }

        loop {
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            self.skip_whitespace();
            let value = self.parse_value(depth)?;
            if values.insert(key, value).is_some() {
                return Err(DiagnosticJsonParseError::Syntax);
            }
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
            self.skip_whitespace();
        }
        Ok(DiagnosticJsonValue::Object(values))
    }

    fn parse_string(&mut self) -> Result<String, DiagnosticJsonParseError> {
        self.expect_byte(b'"')?;
        let mut output = String::new();

        while self.position < self.bytes.len() {
            match self.bytes[self.position] {
                b'"' => {
                    self.position += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.position += 1;
                    let escape = self
                        .peek_byte()
                        .ok_or(DiagnosticJsonParseError::Syntax)?;
                    self.position += 1;
                    match escape {
                        b'"' => output.push('"'),
                        b'\\' => output.push('\\'),
                        b'/' => output.push('/'),
                        b'b' => output.push('\u{08}'),
                        b'f' => output.push('\u{0c}'),
                        b'n' => output.push('\n'),
                        b'r' => output.push('\r'),
                        b't' => output.push('\t'),
                        b'u' => {
                            let first = self.parse_hex_quad()?;
                            if (0xd800..=0xdbff).contains(&first) {
                                if !self.consume_byte(b'\\') || !self.consume_byte(b'u') {
                                    return Err(DiagnosticJsonParseError::Syntax);
                                }
                                let second = self.parse_hex_quad()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return Err(DiagnosticJsonParseError::Syntax);
                                }
                                let codepoint = 0x10000
                                    + (((first as u32) - 0xd800) << 10)
                                    + ((second as u32) - 0xdc00);
                                output.push(
                                    char::from_u32(codepoint)
                                        .ok_or(DiagnosticJsonParseError::Syntax)?,
                                );
                            } else if (0xdc00..=0xdfff).contains(&first) {
                                return Err(DiagnosticJsonParseError::Syntax);
                            } else {
                                output.push(
                                    char::from_u32(first as u32)
                                        .ok_or(DiagnosticJsonParseError::Syntax)?,
                                );
                            }
                        }
                        _ => return Err(DiagnosticJsonParseError::Syntax),
                    }
                }
                byte if byte <= 0x1f => return Err(DiagnosticJsonParseError::Syntax),
                _ => {
                    let ch = self.input[self.position..]
                        .chars()
                        .next()
                        .ok_or(DiagnosticJsonParseError::Syntax)?;
                    self.position += ch.len_utf8();
                    output.push(ch);
                }
            }
        }

        Err(DiagnosticJsonParseError::Syntax)
    }

    fn parse_hex_quad(&mut self) -> Result<u16, DiagnosticJsonParseError> {
        let end = self
            .position
            .checked_add(4)
            .ok_or(DiagnosticJsonParseError::Syntax)?;
        if end > self.bytes.len() {
            return Err(DiagnosticJsonParseError::Syntax);
        }
        let mut value = 0u16;
        for byte in &self.bytes[self.position..end] {
            value = (value << 4)
                | match byte {
                    b'0'..=b'9' => (byte - b'0') as u16,
                    b'a'..=b'f' => (byte - b'a' + 10) as u16,
                    b'A'..=b'F' => (byte - b'A' + 10) as u16,
                    _ => return Err(DiagnosticJsonParseError::Syntax),
                };
        }
        self.position = end;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, DiagnosticJsonParseError> {
        let start = self.position;
        self.consume_byte(b'-');

        match self.peek_byte() {
            Some(b'0') => {
                self.position += 1;
                if matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    return Err(DiagnosticJsonParseError::Syntax);
                }
            }
            Some(b'1'..=b'9') => {
                self.position += 1;
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.position += 1;
                }
            }
            _ => return Err(DiagnosticJsonParseError::Syntax),
        }

        if self.consume_byte(b'.') {
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(DiagnosticJsonParseError::Syntax);
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        if matches!(self.peek_byte(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                return Err(DiagnosticJsonParseError::Syntax);
            }
            while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                self.position += 1;
            }
        }

        Ok(self.input[start..self.position].to_owned())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.position += 1;
        }
    }

    fn expect_bytes(&mut self, expected: &[u8]) -> Result<(), DiagnosticJsonParseError> {
        let end = self
            .position
            .checked_add(expected.len())
            .ok_or(DiagnosticJsonParseError::Syntax)?;
        if self.bytes.get(self.position..end) != Some(expected) {
            return Err(DiagnosticJsonParseError::Syntax);
        }
        self.position = end;
        Ok(())
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), DiagnosticJsonParseError> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err(DiagnosticJsonParseError::Syntax)
        }
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }
}

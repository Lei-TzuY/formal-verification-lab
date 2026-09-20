use std::fmt::Write as _;

pub const CERTIFICATE_VERIFICATION_JOB_RESULT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateVerificationJobOutcome {
    Verified,
    Rejected,
    Error,
}

impl CertificateVerificationJobOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Rejected => "rejected",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateVerificationJobResultEnvelope {
    pub schema_version: u32,
    pub outcome: CertificateVerificationJobOutcome,
    pub model: Option<String>,
    pub property: Option<String>,
    pub certificate: Option<String>,
    pub canonical_formula: Option<String>,
    pub certificate_schema_version: Option<u32>,
    pub message: Option<String>,
}

impl CertificateVerificationJobResultEnvelope {
    pub fn verified(
        model: String,
        property: String,
        certificate: String,
        canonical_formula: String,
        certificate_schema_version: u32,
    ) -> Self {
        Self {
            schema_version: CERTIFICATE_VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            outcome: CertificateVerificationJobOutcome::Verified,
            model: Some(model),
            property: Some(property),
            certificate: Some(certificate),
            canonical_formula: Some(canonical_formula),
            certificate_schema_version: Some(certificate_schema_version),
            message: None,
        }
    }

    pub fn rejected(
        model: String,
        property: String,
        certificate: String,
        canonical_formula: String,
        certificate_schema_version: Option<u32>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CERTIFICATE_VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            outcome: CertificateVerificationJobOutcome::Rejected,
            model: Some(model),
            property: Some(property),
            certificate: Some(certificate),
            canonical_formula: Some(canonical_formula),
            certificate_schema_version,
            message: Some(message.into()),
        }
    }

    pub fn error(
        model: Option<String>,
        property: Option<String>,
        certificate: Option<String>,
        canonical_formula: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CERTIFICATE_VERIFICATION_JOB_RESULT_SCHEMA_VERSION,
            outcome: CertificateVerificationJobOutcome::Error,
            model,
            property,
            certificate,
            canonical_formula,
            certificate_schema_version: None,
            message: Some(message.into()),
        }
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push('{');
        field_u32(&mut out, "schema_version", self.schema_version, true);
        field_string(&mut out, "outcome", self.outcome.as_str(), false);
        field_optional_string(&mut out, "model", self.model.as_deref(), false);
        field_optional_string(&mut out, "property", self.property.as_deref(), false);
        field_optional_string(
            &mut out,
            "certificate",
            self.certificate.as_deref(),
            false,
        );
        field_optional_string(
            &mut out,
            "canonical_formula",
            self.canonical_formula.as_deref(),
            false,
        );
        field_optional_u32(
            &mut out,
            "certificate_schema_version",
            self.certificate_schema_version,
            false,
        );
        field_optional_string(&mut out, "message", self.message.as_deref(), false);
        out.push('}');
        out
    }
}

fn field_name(out: &mut String, name: &str, first: bool) {
    if !first {
        out.push(',');
    }
    write_json_string(out, name);
    out.push(':');
}

fn field_u32(out: &mut String, name: &str, value: u32, first: bool) {
    field_name(out, name, first);
    out.push_str(&value.to_string());
}

fn field_optional_u32(out: &mut String, name: &str, value: Option<u32>, first: bool) {
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
            ch if ch < '\u{20}' => {
                write!(out, "\\u{:04x}", ch as u32).expect("writing to String cannot fail");
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
}

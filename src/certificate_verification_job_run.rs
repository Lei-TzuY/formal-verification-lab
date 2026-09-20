use crate::certificate_verification_job::{
    parse_certificate_verification_job, CertificateVerificationJob,
};
use crate::certificate_verification_result::CertificateVerificationJobResultEnvelope;
use crate::declarative::parse_declarative_document;
use crate::declarative_mu::validate_declarative_mu_formula;
use crate::mu_parity_certificate::{
    parse_declarative_mu_parity_certificate, verify_declarative_mu_parity_certificate,
};
use crate::mu_parse::{parse_mu_formula, render_mu_formula};
use crate::text_source::{
    path_source_id, resolve_source_id, FileSystemTextSourceProvider, TextSourceProvider,
};
use std::path::Path;

pub const CERTIFICATE_VERIFICATION_REJECTED_EXIT_CODE: u8 = 16;
pub const CERTIFICATE_VERIFICATION_ERROR_EXIT_CODE: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateVerificationJobJsonRun {
    pub exit_code: u8,
    pub envelope: CertificateVerificationJobResultEnvelope,
}

impl CertificateVerificationJobJsonRun {
    pub fn to_json(&self) -> String {
        self.envelope.to_json()
    }
}

pub fn run_certificate_verification_job_json(
    manifest_path: impl AsRef<Path>,
) -> CertificateVerificationJobJsonRun {
    let manifest_source_id = match path_source_id(manifest_path.as_ref()) {
        Ok(source_id) => source_id,
        Err(error) => return error_run(None, error.to_string()),
    };
    run_certificate_verification_job_json_with_provider(
        &FileSystemTextSourceProvider,
        &manifest_source_id,
    )
}

pub fn run_certificate_verification_job_json_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
) -> CertificateVerificationJobJsonRun {
    let manifest_text = match provider.read_text(manifest_source_id) {
        Ok(value) => value,
        Err(error) => {
            return error_run(
                None,
                format!(
                    "failed to read certificate verification job '{}': {}",
                    manifest_source_id,
                    error.kind().as_str()
                ),
            )
        }
    };
    let job = match parse_certificate_verification_job(&manifest_text) {
        Ok(value) => value,
        Err(error) => return error_run(None, error.to_string()),
    };

    run_loaded_job_with_provider(provider, manifest_source_id, &job)
}

fn run_loaded_job_with_provider(
    provider: &dyn TextSourceProvider,
    manifest_source_id: &str,
    job: &CertificateVerificationJob,
) -> CertificateVerificationJobJsonRun {
    let model_source_id = match resolve_source_id(manifest_source_id, job.model_path()) {
        Ok(value) => value,
        Err(error) => return setup_error(job, None, error.to_string()),
    };
    let property_source_id = match resolve_source_id(manifest_source_id, job.property_path()) {
        Ok(value) => value,
        Err(error) => return setup_error(job, None, error.to_string()),
    };
    let certificate_source_id = match resolve_source_id(manifest_source_id, job.certificate_path())
    {
        Ok(value) => value,
        Err(error) => return setup_error(job, None, error.to_string()),
    };

    let model_text = match provider.read_text(&model_source_id) {
        Ok(value) => value,
        Err(error) => {
            return setup_error(
                job,
                None,
                format!(
                    "failed to read declarative model '{}': {}",
                    model_source_id,
                    error.kind().as_str()
                ),
            )
        }
    };
    let document = match parse_declarative_document(&model_text) {
        Ok(value) => value,
        Err(error) => return setup_error(job, None, error.to_string()),
    };

    let property_text = match provider.read_text(&property_source_id) {
        Ok(value) => value,
        Err(error) => {
            return setup_error(
                job,
                None,
                format!(
                    "failed to read mu-calculus property '{}': {}",
                    property_source_id,
                    error.kind().as_str()
                ),
            )
        }
    };
    let formula = match parse_mu_formula(&property_text) {
        Ok(value) => value,
        Err(error) => return setup_error(job, None, error.to_string()),
    };
    if let Err(error) = validate_declarative_mu_formula(&document, &formula) {
        return setup_error(job, None, error.to_string());
    }
    let canonical_formula = render_mu_formula(&formula);

    let certificate_text = match provider.read_text(&certificate_source_id) {
        Ok(value) => value,
        Err(error) => {
            return setup_error(
                job,
                Some(canonical_formula),
                format!(
                    "failed to read mu parity certificate '{}': {}",
                    certificate_source_id,
                    error.kind().as_str()
                ),
            )
        }
    };

    let schema_hint = certificate_schema_hint(&certificate_text);
    let certificate = match parse_declarative_mu_parity_certificate(&certificate_text) {
        Ok(value) => value,
        Err(error) => return rejected_run(job, canonical_formula, schema_hint, error.to_string()),
    };

    if let Err(error) =
        verify_declarative_mu_parity_certificate(&document, &property_text, &certificate)
    {
        return rejected_run(
            job,
            canonical_formula,
            Some(certificate.schema_version),
            error.to_string(),
        );
    }

    CertificateVerificationJobJsonRun {
        exit_code: 0,
        envelope: CertificateVerificationJobResultEnvelope::verified(
            job.model_path().to_owned(),
            job.property_path().to_owned(),
            job.certificate_path().to_owned(),
            canonical_formula,
            certificate.schema_version,
        ),
    }
}

fn rejected_run(
    job: &CertificateVerificationJob,
    canonical_formula: String,
    certificate_schema_version: Option<u32>,
    message: String,
) -> CertificateVerificationJobJsonRun {
    CertificateVerificationJobJsonRun {
        exit_code: CERTIFICATE_VERIFICATION_REJECTED_EXIT_CODE,
        envelope: CertificateVerificationJobResultEnvelope::rejected(
            job.model_path().to_owned(),
            job.property_path().to_owned(),
            job.certificate_path().to_owned(),
            canonical_formula,
            certificate_schema_version,
            message,
        ),
    }
}

fn setup_error(
    job: &CertificateVerificationJob,
    canonical_formula: Option<String>,
    message: String,
) -> CertificateVerificationJobJsonRun {
    error_run(
        Some((
            job.model_path().to_owned(),
            job.property_path().to_owned(),
            job.certificate_path().to_owned(),
            canonical_formula,
        )),
        message,
    )
}

fn error_run(
    paths: Option<(String, String, String, Option<String>)>,
    message: String,
) -> CertificateVerificationJobJsonRun {
    let (model, property, certificate, canonical_formula) = match paths {
        Some((model, property, certificate, canonical_formula)) => (
            Some(model),
            Some(property),
            Some(certificate),
            canonical_formula,
        ),
        None => (None, None, None, None),
    };
    CertificateVerificationJobJsonRun {
        exit_code: CERTIFICATE_VERIFICATION_ERROR_EXIT_CODE,
        envelope: CertificateVerificationJobResultEnvelope::error(
            model,
            property,
            certificate,
            canonical_formula,
            message,
        ),
    }
}

fn certificate_schema_hint(input: &str) -> Option<u32> {
    input.lines().find_map(|raw| {
        let line = raw.trim();
        let rest = line.strip_prefix("fvlab-mu-parity-certificate ")?;
        rest.trim().parse::<u32>().ok()
    })
}

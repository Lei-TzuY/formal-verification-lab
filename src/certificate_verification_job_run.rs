use crate::certificate_verification_job::{
    parse_certificate_verification_job, CertificateVerificationJob,
};
use crate::certificate_verification_result::{
    CertificateVerificationJobOutcome, CertificateVerificationJobResultEnvelope,
};
use crate::declarative::parse_declarative_document;
use crate::declarative_mu::validate_declarative_mu_formula;
use crate::mu_parity_certificate::{
    parse_declarative_mu_parity_certificate, verify_declarative_mu_parity_certificate,
};
use crate::mu_parse::{parse_mu_formula, render_mu_formula};
use std::fs;
use std::path::{Path, PathBuf};

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
    let manifest_path = manifest_path.as_ref();
    let manifest_text = match fs::read_to_string(manifest_path) {
        Ok(value) => value,
        Err(error) => {
            return error_run(
                None,
                format!(
                    "failed to read certificate verification job '{}': {error}",
                    manifest_path.display()
                ),
            )
        }
    };
    let job = match parse_certificate_verification_job(&manifest_text) {
        Ok(value) => value,
        Err(error) => return error_run(None, error.to_string()),
    };

    run_loaded_job(manifest_path, &job)
}

fn run_loaded_job(
    manifest_path: &Path,
    job: &CertificateVerificationJob,
) -> CertificateVerificationJobJsonRun {
    let base = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let model_path = resolve_path(base, Path::new(job.model_path()));
    let property_path = resolve_path(base, Path::new(job.property_path()));
    let certificate_path = resolve_path(base, Path::new(job.certificate_path()));

    let model_text = match fs::read_to_string(&model_path) {
        Ok(value) => value,
        Err(error) => {
            return setup_error(
                job,
                None,
                format!(
                    "failed to read declarative model '{}': {error}",
                    job.model_path()
                ),
            )
        }
    };
    let document = match parse_declarative_document(&model_text) {
        Ok(value) => value,
        Err(error) => return setup_error(job, None, error.to_string()),
    };

    let property_text = match fs::read_to_string(&property_path) {
        Ok(value) => value,
        Err(error) => {
            return setup_error(
                job,
                None,
                format!(
                    "failed to read mu-calculus property '{}': {error}",
                    job.property_path()
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

    let certificate_text = match fs::read_to_string(&certificate_path) {
        Ok(value) => value,
        Err(error) => {
            return setup_error(
                job,
                Some(canonical_formula),
                format!(
                    "failed to read mu parity certificate '{}': {error}",
                    job.certificate_path()
                ),
            )
        }
    };

    let schema_hint = certificate_schema_hint(&certificate_text);
    let certificate = match parse_declarative_mu_parity_certificate(&certificate_text) {
        Ok(value) => value,
        Err(error) => {
            return rejected_run(
                job,
                canonical_formula,
                schema_hint,
                error.to_string(),
            )
        }
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

fn resolve_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    }
}

#[allow(dead_code)]
fn _assert_outcome_exhaustive(outcome: CertificateVerificationJobOutcome) -> &'static str {
    outcome.as_str()
}

use formal_verification_lab::{
    create_declarative_mu_parity_certificate, normalize_source_id, parse_declarative_document,
    render_declarative_mu_parity_certificate, resolve_source_id,
    run_certificate_verification_job_json, run_certificate_verification_job_json_with_provider,
    run_orchestration_suite_expectations_json_with_provider,
    run_orchestration_suite_json, run_orchestration_suite_json_with_provider,
    run_structural_job_json, run_structural_job_json_with_provider, run_verification_job_json,
    run_verification_job_json_with_provider, MapTextSourceProvider,
    RootedFileSystemTextSourceProvider,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

const MODEL: &str = r#"
model "virtual-workspace"
state "start"
state "loop"
state "done"
initial "start"
edge "start" "cycle" "loop"
edge "start" "finish" "done"
edge "loop" "back" "start"
label "start" "ready"
label "loop" "ready"
label "done" "complete"
"#;

const REACH_FORMULA: &str = r#"mu X. "complete" or diamond $X"#;

#[test]
fn logical_source_ids_normalize_and_resolve_without_host_paths() {
    assert_eq!(normalize_source_id("a/./b/../c").unwrap(), "a/c");
    assert_eq!(
        normalize_source_id("../../models/system.fvl").unwrap(),
        "../../models/system.fvl"
    );
    assert_eq!(
        resolve_source_id("workspace/jobs/verify.job", "../models/system.fvl").unwrap(),
        "workspace/models/system.fvl"
    );
    assert_eq!(
        resolve_source_id("workspace/suites/mixed.suite", "../jobs/verify.job").unwrap(),
        "workspace/jobs/verify.job"
    );
}

#[test]
fn path_apis_match_rooted_provider_for_successful_family_and_orchestration_runs() {
    let fixture = Fixture::new("path-parity");
    let rooted = RootedFileSystemTextSourceProvider::new(&fixture.root);

    assert_same_run(
        run_verification_job_json(fixture.root.join("jobs/verify.job")).to_json(),
        run_verification_job_json_with_provider(&rooted, "jobs/verify.job").to_json(),
    );
    assert_same_run(
        run_verification_job_json(fixture.root.join("jobs/violate.job")).to_json(),
        run_verification_job_json_with_provider(&rooted, "jobs/violate.job").to_json(),
    );
    assert_same_run(
        run_structural_job_json(fixture.root.join("jobs/structure.job")).to_json(),
        run_structural_job_json_with_provider(&rooted, "jobs/structure.job").to_json(),
    );
    assert_same_run(
        run_certificate_verification_job_json(fixture.root.join("jobs/certificate.job")).to_json(),
        run_certificate_verification_job_json_with_provider(&rooted, "jobs/certificate.job")
            .to_json(),
    );
    assert_same_run(
        run_orchestration_suite_json(fixture.root.join("mixed.suite")).to_json(),
        run_orchestration_suite_json_with_provider(&rooted, "mixed.suite").to_json(),
    );

    fixture.cleanup();
}

#[test]
fn map_and_rooted_filesystem_providers_match_representative_outcomes() {
    let fixture = Fixture::new("provider-parity");
    let rooted = RootedFileSystemTextSourceProvider::new(&fixture.root);
    let map = fixture.map_provider();

    let verified_fs =
        run_verification_job_json_with_provider(&rooted, "jobs/verify.job");
    let verified_map = run_verification_job_json_with_provider(&map, "jobs/verify.job");
    assert_eq!(verified_fs.to_json(), verified_map.to_json());
    assert_eq!(verified_map.envelope.outcome.as_str(), "satisfied");

    let violated_fs =
        run_verification_job_json_with_provider(&rooted, "jobs/violate.job");
    let violated_map = run_verification_job_json_with_provider(&map, "jobs/violate.job");
    assert_eq!(violated_fs.to_json(), violated_map.to_json());
    assert_eq!(violated_map.envelope.outcome.as_str(), "violated");

    let structural_fs =
        run_structural_job_json_with_provider(&rooted, "jobs/structure.job");
    let structural_map =
        run_structural_job_json_with_provider(&map, "jobs/structure.job");
    assert_eq!(structural_fs.to_json(), structural_map.to_json());
    assert_eq!(structural_map.envelope.outcome.as_str(), "cycle_found");

    let certificate_fs =
        run_certificate_verification_job_json_with_provider(&rooted, "jobs/certificate.job");
    let certificate_map =
        run_certificate_verification_job_json_with_provider(&map, "jobs/certificate.job");
    assert_eq!(certificate_fs.to_json(), certificate_map.to_json());
    assert_eq!(certificate_map.envelope.outcome.as_str(), "verified");

    let orchestration_fs = run_orchestration_suite_json_with_provider(&rooted, "mixed.suite");
    let orchestration_map = run_orchestration_suite_json_with_provider(&map, "mixed.suite");
    assert_eq!(orchestration_fs.to_json(), orchestration_map.to_json());

    let expectations_fs =
        run_orchestration_suite_expectations_json_with_provider(&rooted, "expected.suite");
    let expectations_map =
        run_orchestration_suite_expectations_json_with_provider(&map, "expected.suite");
    assert_eq!(expectations_fs.to_json(), expectations_map.to_json());
    assert_eq!(expectations_map.exit_code, 0);

    fixture.cleanup();
}

#[test]
fn certificate_rejection_is_identical_across_providers() {
    let fixture = Fixture::new("rejected");
    let rooted = RootedFileSystemTextSourceProvider::new(&fixture.root);
    let mut map = fixture.map_provider();

    let certificate_path = fixture.root.join("certificates/reach.mupc");
    let original = fs::read_to_string(&certificate_path).unwrap();
    let truncated = original
        .lines()
        .filter(|line| *line != "end")
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&certificate_path, &truncated).unwrap();
    map.insert("certificates/reach.mupc", truncated).unwrap();

    let fs_run =
        run_certificate_verification_job_json_with_provider(&rooted, "jobs/certificate.job");
    let map_run =
        run_certificate_verification_job_json_with_provider(&map, "jobs/certificate.job");
    assert_eq!(fs_run.to_json(), map_run.to_json());
    assert_eq!(map_run.envelope.outcome.as_str(), "rejected");

    fixture.cleanup();
}

#[test]
fn setup_error_json_is_byte_identical_under_different_host_roots_and_map_provider() {
    let sources = fixture_sources();
    let missing_model_sources = sources
        .into_iter()
        .filter(|(source_id, _)| source_id != "models/system.fvl")
        .collect::<Vec<_>>();

    let root_a = fixture_dir("root-a");
    let root_b = fixture_dir("root-b");
    write_sources(&root_a, &missing_model_sources);
    write_sources(&root_b, &missing_model_sources);

    let fs_a = RootedFileSystemTextSourceProvider::new(&root_a);
    let fs_b = RootedFileSystemTextSourceProvider::new(&root_b);
    let map = MapTextSourceProvider::from_sources(missing_model_sources.clone()).unwrap();

    let run_a = run_orchestration_suite_json_with_provider(&fs_a, "mixed.suite");
    let run_b = run_orchestration_suite_json_with_provider(&fs_b, "mixed.suite");
    let run_map = run_orchestration_suite_json_with_provider(&map, "mixed.suite");

    assert_eq!(run_a.to_json(), run_b.to_json());
    assert_eq!(run_a.to_json(), run_map.to_json());
    assert_eq!(run_a.exit_code, 2);
    assert!(!run_a.to_json().contains(root_a.to_str().unwrap()));
    assert!(!run_a.to_json().contains(root_b.to_str().unwrap()));
    assert!(run_a.to_json().contains("models/system.fvl"));
    assert!(run_a.to_json().contains("not-found"));

    fs::remove_dir_all(root_a).unwrap();
    fs::remove_dir_all(root_b).unwrap();
}

struct Fixture {
    root: PathBuf,
    sources: Vec<(String, String)>,
}

impl Fixture {
    fn new(kind: &str) -> Self {
        let root = fixture_dir(kind);
        let sources = fixture_sources();
        write_sources(&root, &sources);
        Self { root, sources }
    }

    fn map_provider(&self) -> MapTextSourceProvider {
        MapTextSourceProvider::from_sources(self.sources.clone()).unwrap()
    }

    fn cleanup(self) {
        fs::remove_dir_all(self.root).unwrap();
    }
}

fn fixture_sources() -> Vec<(String, String)> {
    let document = parse_declarative_document(MODEL).unwrap();
    let certificate =
        create_declarative_mu_parity_certificate(&document, REACH_FORMULA).unwrap();
    let certificate_text = render_declarative_mu_parity_certificate(&certificate);

    vec![
        ("models/system.fvl".to_owned(), MODEL.to_owned()),
        (
            "properties/reach.mu".to_owned(),
            REACH_FORMULA.to_owned(),
        ),
        ("properties/false.mu".to_owned(), "false".to_owned()),
        ("certificates/reach.mupc".to_owned(), certificate_text),
        (
            "jobs/verify.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/reach.mu\"\n"
                .to_owned(),
        ),
        (
            "jobs/violate.job".to_owned(),
            "analysis \"mu-calculus\"\nmodel \"../models/system.fvl\"\nproperty \"../properties/false.mu\"\n"
                .to_owned(),
        ),
        (
            "jobs/structure.job".to_owned(),
            "analysis \"recurrence\"\nmodel \"../models/system.fvl\"\n".to_owned(),
        ),
        (
            "jobs/certificate.job".to_owned(),
            "model \"../models/system.fvl\"\nproperty \"../properties/reach.mu\"\ncertificate \"../certificates/reach.mupc\"\n"
                .to_owned(),
        ),
        (
            "mixed.suite".to_owned(),
            concat!(
                "suite \"mixed\"\n",
                "job \"verification\" \"jobs/verify.job\"\n",
                "job \"verification\" \"jobs/violate.job\"\n",
                "job \"structural\" \"jobs/structure.job\"\n",
                "job \"certificate-verification\" \"jobs/certificate.job\"\n"
            )
            .to_owned(),
        ),
        (
            "expected.suite".to_owned(),
            concat!(
                "suite \"expected\"\n",
                "job \"verification\" \"jobs/verify.job\" expect \"satisfied\"\n",
                "job \"verification\" \"jobs/violate.job\" expect \"violated\"\n",
                "job \"structural\" \"jobs/structure.job\" expect \"cycle_found\"\n",
                "job \"certificate-verification\" \"jobs/certificate.job\" expect \"verified\"\n"
            )
            .to_owned(),
        ),
    ]
}

fn write_sources(root: &Path, sources: &[(String, String)]) {
    for (source_id, text) in sources {
        let path = root.join(source_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }
}

fn assert_same_run(left: String, right: String) {
    assert_eq!(left, right);
}

fn fixture_dir(kind: &str) -> PathBuf {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "fvlab-m89-virtual-workspace-{kind}-{}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

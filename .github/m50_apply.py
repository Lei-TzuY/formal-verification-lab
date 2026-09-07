from pathlib import Path


def replace_once(text, old, new, label):
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


main = Path("src/main.rs")
text = main.read_text()
text = replace_once(
    text,
    "    render_analysis_fairness_profile_temporal_report, render_analysis_strong_fair_monitor_report,\n    render_analysis_strong_fair_temporal_report, render_analysis_weak_fair_monitor_report,\n    render_analysis_weak_fair_temporal_report, render_bounded_fairness_profile_temporal_report,\n    render_bounded_strong_fair_monitor_report, render_bounded_strong_fair_temporal_report,\n    render_bounded_weak_fair_monitor_report, render_bounded_weak_fair_temporal_report,\n    render_fairness_profile_temporal_report, render_strong_fair_monitor_report,\n",
    "    render_analysis_fairness_profile_monitor_report,\n    render_analysis_fairness_profile_temporal_report, render_analysis_strong_fair_monitor_report,\n    render_analysis_strong_fair_temporal_report, render_analysis_weak_fair_monitor_report,\n    render_analysis_weak_fair_temporal_report, render_bounded_fairness_profile_monitor_report,\n    render_bounded_fairness_profile_temporal_report, render_bounded_strong_fair_monitor_report,\n    render_bounded_strong_fair_temporal_report, render_bounded_weak_fair_monitor_report,\n    render_bounded_weak_fair_temporal_report, render_fairness_profile_monitor_report,\n    render_fairness_profile_temporal_report, render_strong_fair_monitor_report,\n",
    "main fairness-report imports",
)
text = replace_once(
    text,
    "use formal_verification_lab::monitor_examples::{\n",
    "use formal_verification_lab::monitor_combined_fairness::{\n    check_monitor_with_fairness_profile, check_monitor_with_fairness_profile_and_limits,\n    check_monitor_with_fairness_profile_and_product_limits,\n};\nuse formal_verification_lab::monitor_examples::{\n",
    "main combined-monitor imports",
)
text = replace_once(
    text,
    "    let options = parse_temporal_options(option_args)?;\n    if !options.fairness.is_empty() && !options.strong_fairness.is_empty() {\n        return Err(\"cannot combine weak and strong fairness assumptions; choose one fairness strength per analysis\".to_owned());\n    }\n\n    if !options.strong_fairness.is_empty() {\n",
    "    let options = parse_temporal_options(option_args)?;\n    if !options.fairness.is_empty() && !options.strong_fairness.is_empty() {\n        if options.has_model_limits {\n            let limits = AnalysisLimits::new(options.model_limits, options.product_limits);\n            let result = check_monitor_with_fairness_profile_and_limits(\n                &model,\n                &monitor,\n                &options.fairness_profile,\n                limits,\n            )\n            .map_err(|error| error.to_string())?;\n            print!(\n                \"{}\",\n                render_analysis_fairness_profile_monitor_report(\n                    model.name(),\n                    &result,\n                    &options.fairness_profile,\n                )\n            );\n            return Ok(match &result.outcome {\n                AnalysisOutcome::Conclusive(MonitorStatus::Satisfied) => ExitCode::SUCCESS,\n                AnalysisOutcome::Conclusive(MonitorStatus::Violated) => ExitCode::from(8),\n                AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),\n            });\n        }\n\n        if options.has_product_limits {\n            let result = check_monitor_with_fairness_profile_and_product_limits(\n                &model,\n                &monitor,\n                &options.fairness_profile,\n                options.product_limits,\n            )\n            .map_err(|error| error.to_string())?;\n            print!(\n                \"{}\",\n                render_bounded_fairness_profile_monitor_report(\n                    model.name(),\n                    &result,\n                    &options.fairness_profile,\n                )\n            );\n            return Ok(match &result.outcome {\n                BoundedOutcome::Conclusive(MonitorStatus::Satisfied) => ExitCode::SUCCESS,\n                BoundedOutcome::Conclusive(MonitorStatus::Violated) => ExitCode::from(8),\n                BoundedOutcome::Inconclusive(_) => ExitCode::from(3),\n            });\n        }\n\n        let result = check_monitor_with_fairness_profile(&model, &monitor, &options.fairness_profile)\n            .map_err(|error| error.to_string())?;\n        print!(\n            \"{}\",\n            render_fairness_profile_monitor_report(\n                model.name(),\n                &result,\n                &options.fairness_profile,\n            )\n        );\n        return Ok(match result.status {\n            MonitorStatus::Satisfied => ExitCode::SUCCESS,\n            MonitorStatus::Violated => ExitCode::from(8),\n        });\n    }\n\n    if !options.strong_fairness.is_empty() {\n",
    "main monitor mixed dispatch",
)
main.write_text(text)

report = Path("src/fairness_report.rs")
text = report.read_text()
text = replace_once(
    text,
    "//! combined weak/strong fairness profile without changing witness, accounting,\n//! or cutoff semantics.\n",
    "//! combined weak/strong fairness profile without changing witness, accounting,\n//! or cutoff semantics. M50 applies the same reporting contract to finite monitors.\n",
    "fairness report module docs",
)
marker = "fn append_fairness(output: &mut String, strength: &str, actions: &[String]) {"
combined_monitor = '''/// Render one unbounded finite-monitor result with both canonical fairness
/// classes. Overlapping actions appear only in the strong class.
pub fn render_fairness_profile_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &MonitorResult<S, M>,
    profile: &FairnessProfile,
) -> String {
    let mut output = render_monitor_report(model_name, result);
    append_fairness(&mut output, "weak", profile.weak_actions());
    append_fairness(&mut output, "strong", profile.strong_actions());
    output
}

/// Render a product-bounded finite-monitor result with both canonical fairness
/// classes while preserving canonical product-cutoff accounting.
pub fn render_bounded_fairness_profile_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &BoundedMonitorResult<S, M>,
    profile: &FairnessProfile,
) -> String {
    let mut output = render_bounded_monitor_report(model_name, result);
    append_fairness(&mut output, "weak", profile.weak_actions());
    append_fairness(&mut output, "strong", profile.strong_actions());
    output
}

/// Render staged model/product finite-monitor analysis with both canonical
/// fairness classes and unchanged stage-qualified cutoff provenance.
pub fn render_analysis_fairness_profile_monitor_report<S: Debug, M: Debug>(
    model_name: &str,
    result: &AnalysisMonitorResult<S, M>,
    profile: &FairnessProfile,
) -> String {
    let mut output = render_analysis_monitor_report(model_name, result);
    append_fairness(&mut output, "weak", profile.weak_actions());
    append_fairness(&mut output, "strong", profile.strong_actions());
    output
}

'''
text = replace_once(text, marker, combined_monitor + marker, "combined monitor reports")
report.write_text(text)

old_test = Path("tests/monitor_strong_fairness_cli.rs")
text = old_test.read_text()
text = replace_once(
    text,
    "fn monitor_strong_fairness_options_fail_closed_on_duplicate_empty_missing_or_mixed_actions() {",
    "fn monitor_strong_fairness_options_fail_closed_on_duplicate_empty_or_missing_actions() {",
    "obsolete mixed-fail test name",
)
old_mixed = '''    let mixed = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "close",
    ]);
    assert_eq!(mixed.status.code(), Some(2));
    assert!(stderr(&mixed).contains("cannot combine weak and strong fairness assumptions"));

'''
text = replace_once(text, old_mixed, "", "obsolete mixed-fail expectation")
old_test.write_text(text)

cli = Path("tests/monitor_combined_fairness_cli.rs")
if cli.exists():
    raise SystemExit("combined monitor CLI test unexpectedly already exists")
cli.write_text(r'''use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fvlab"))
        .args(args)
        .output()
        .expect("fvlab binary should execute")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("CLI stdout should be UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("CLI stderr should be UTF-8")
}

#[test]
fn mixed_monitor_fairness_is_accepted_reported_and_canonicalizes_overlap() {
    let mixed = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert!(mixed.status.success(), "{}", stderr(&mixed));
    let text = stdout(&mixed);
    assert!(text.contains("monitor verification: SATISFIED"));
    assert!(text.contains("weak fairness actions: 1"));
    assert!(text.contains("weak-fair action: \"close\""));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"unrelated\""));

    let overlap = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "close",
    ]);
    assert!(overlap.status.success(), "{}", stderr(&overlap));
    let overlap_text = stdout(&overlap);
    assert!(overlap_text.contains("weak fairness actions: 0"));
    assert!(overlap_text.contains("strong fairness actions: 1"));
    assert!(overlap_text.contains("strong-fair action: \"close\""));
}

#[test]
fn mixed_monitor_fairness_preserves_finite_and_rejecting_precedence() {
    let rejecting = run(&[
        "monitor",
        "session-double-open",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "commit",
    ]);
    assert_eq!(rejecting.status.code(), Some(8));
    let rejecting_text = stdout(&rejecting);
    assert!(rejecting_text.contains("counterexample: REJECTING_STATE"));
    assert!(rejecting_text.contains("weak-fair action: \"close\""));
    assert!(rejecting_text.contains("strong-fair action: \"commit\""));

    let terminal = run(&[
        "monitor",
        "session-open-terminal",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(terminal.status.code(), Some(8));
    let terminal_text = stdout(&terminal);
    assert!(terminal_text.contains("counterexample: PROGRESS_TERMINAL"));
    assert!(terminal_text.contains("weak-fair action: \"close\""));
    assert!(terminal_text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn mixed_monitor_fairness_preserves_product_and_model_cutoff_provenance() {
    let product = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
        "--max-product-transitions",
        "3",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_text = stdout(&product);
    assert!(product_text.contains("monitor verification: INCONCLUSIVE"));
    assert!(product_text.contains("product inconclusive reason: transition limit reached (max 3)"));
    assert!(product_text.contains("weak-fair action: \"close\""));
    assert!(product_text.contains("strong-fair action: \"unrelated\""));

    let staged = run(&[
        "monitor",
        "session-unfair-close",
        "--weak-fair-action",
        "close",
        "--strong-fair-action",
        "unrelated",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(staged.status.code(), Some(3));
    let staged_text = stdout(&staged);
    assert!(staged_text.contains("analysis inconclusive stage: model"));
    assert!(staged_text.contains("analysis inconclusive reason: transition limit reached (max 2)"));
    assert!(staged_text.contains("weak-fair action: \"close\""));
    assert!(staged_text.contains("strong-fair action: \"unrelated\""));
}
''')

readme = Path("README.md")
text = readme.read_text()
text = replace_once(
    text,
    "M48 exposes that combined profile on the external temporal CLI, and M49 composes the same canonical profile with multi-response verification while preserving exact no-fair, weak-only, and strong-only compatibility paths.",
    "M48 exposes that combined profile on the external temporal CLI, M49 composes the same canonical profile with multi-response verification, and M50 composes it with finite-monitor verification while preserving exact no-fair, weak-only, and strong-only compatibility paths.",
    "README current capability",
)
text = replace_once(
    text,
    "M49 adds typed/backend mixed-fairness multi-response composition across unbounded, product-bounded, and staged APIs. Mixed finite-monitor composition remains the next deliberate frontier.",
    "M49 adds typed/backend mixed-fairness multi-response composition across unbounded, product-bounded, and staged APIs. M50 adds the same combined profile to finite-monitor verification and exposes mixed weak/strong assumptions on the direct `monitor` CLI with canonical overlap handling.",
    "README current mixed monitor status",
)
text = replace_once(
    text,
    "| M49 | Combined-fair multi-response composition with per-clause acceptance, finite-terminal preservation, and bounded/staged provenance. |\n",
    "| M49 | Combined-fair multi-response composition with per-clause acceptance, finite-terminal preservation, and bounded/staged provenance. |\n| M50 | Combined-fair finite-monitor composition plus direct mixed-fair monitor CLI/reporting with rejecting/finite-terminal precedence and bounded/staged provenance. |\n",
    "README M50 milestone row",
)
section = '''### M50 — combined-fair finite-monitor composition and CLI

M50 propagates the canonical M45 `FairnessProfile` through finite-monitor verification across unbounded, product-bounded, and staged APIs. Empty, weak-only, and strong-only profiles delegate exactly to the sealed no-fair, M36 weak-fair, and M43 strong-fair monitor authorities; only genuinely mixed profiles enter the M45/M46 combined-fair generalized Büchi recurrent engine.

Monitor precedence is unchanged: reachable rejecting states remain globally first, real finite terminals with an active progress condition remain violations because fairness constrains only infinite executions, and only infinite active progress cycles are filtered by fairness. Independent progress-condition identity, monitor-state evidence, complete-model enablement, conservative staged provenance, deterministic model-before-product cutoff precedence, and `INCONCLUSIVE` honesty are preserved.

The direct `monitor` command now accepts simultaneous repeated `--weak-fair-action` and `--strong-fair-action` declarations. `FairnessProfile` canonicalizes overlap into the strong class, reports both canonical classes explicitly, and routes mixed unbounded/product/staged invocations into the M50 backend while retaining historical no/weak/strong paths. Monitor violations retain exit `8`; unresolved resource cutoffs retain exit `3`; malformed or duplicate assumptions retain exit `2`.

Executable evidence includes focused mixed-filtering, rejecting/finite-terminal precedence, exact compatibility delegation, product/model cutoff provenance, and generous-limit regressions; generated adapter differentials cover **4,096 unbounded**, **12,288 product-bounded**, and **20,480 staged** cases (**36,864 total**) against the sealed combined-fair Büchi authority. Built-binary regressions additionally verify mixed assumption routing, overlap canonicalization, explicit reporting, finite/rejecting precedence, and product/model cutoff behavior.

M50 adds no fairness-by-default behavior, textual monitor language, second traversal engine, wall-clock proof bound, or performance claim.

'''
text = replace_once(text, "## Architecture\n", section + "## Architecture\n", "README M50 section")
text = replace_once(
    text,
    "src/monitor_strong_fairness.rs strong-fair monitor progress composition over sealed Büchi core\n",
    "src/monitor_strong_fairness.rs strong-fair monitor progress composition over sealed Büchi core\nsrc/monitor_combined_fairness.rs combined weak/strong monitor composition over sealed Büchi core\n",
    "README architecture monitor combined module",
)
text = replace_once(
    text,
    "- M49: **28,672** combined-fair multi-response adapter differential cases: 4,096 unbounded + 12,288 product-bounded + 12,288 staged.\n",
    "- M49: **28,672** combined-fair multi-response adapter differential cases: 4,096 unbounded + 12,288 product-bounded + 12,288 staged;\n- M50: **36,864** combined-fair finite-monitor adapter differential cases: 4,096 unbounded + 12,288 product-bounded + 20,480 staged.\n",
    "README generated evidence M50",
)
text = replace_once(
    text,
    "M49 adds exact no/weak/strong delegation across unbounded/product/staged paths, focused genuinely mixed filtering/finite/lasso/cutoff regressions, and the 28,672-case adapter differential against the sealed combined-fair Büchi authority.\n",
    "M49 adds exact no/weak/strong delegation across unbounded/product/staged paths, focused genuinely mixed filtering/finite/lasso/cutoff regressions, and the 28,672-case adapter differential against the sealed combined-fair Büchi authority. M50 adds exact no/weak/strong monitor delegation, mixed progress filtering, rejecting/finite-terminal precedence, product/staged provenance, the 36,864-case monitor adapter differential, and built-binary mixed monitor routing/reporting regressions.\n",
    "README regression narrative M50",
)
text = text.replace(
    "M25–M29/M33/M39–M49 staged temporal bounds",
    "M25–M29/M33/M39–M50 staged temporal bounds",
)
text = replace_once(
    text,
    "M45–M49 provide explicit combined weak/strong fairness semantics through generalized Büchi, single-response, typed/textual/declarative action-temporal, and typed/backend multi-response APIs across unbounded, product-bounded, and staged analysis; M48 exposes mixed temporal assumptions on the CLI. Mixed finite-monitor composition remains unavailable until M50 and is not silently approximated by dispatch precedence.",
    "M45–M50 provide explicit combined weak/strong fairness semantics through generalized Büchi, single-response, typed/textual/declarative action-temporal, typed/backend multi-response, and finite-monitor APIs across unbounded, product-bounded, and staged analysis; M48 exposes mixed temporal assumptions on the CLI and M50 exposes mixed finite-monitor assumptions on the direct `monitor` CLI.",
    "README combined fairness limitation",
)
text = replace_once(
    text,
    "M37 exposes finite-monitor weak fairness on the direct `monitor` CLI, and M44 exposes the sealed M43 strong-fair finite-monitor backend through the same direct command for unbounded, product-bounded, and staged analysis. Mixed monitor assumptions remain fail-closed until combined-fair monitor composition is sealed.",
    "M37 exposes finite-monitor weak fairness on the direct `monitor` CLI, M44 exposes strong-fair monitor verification, and M50 exposes canonical mixed weak/strong monitor assumptions through the same direct command for unbounded, product-bounded, and staged analysis.",
    "README direct monitor limitation",
)
prefix, marker, _ = text.partition("## Roadmap\n")
if not marker:
    raise SystemExit("README roadmap marker missing")
roadmap = '''## Roadmap

Milestones 1–50 now form a coherent explicit-state stack: safety and bounded honesty -> typed models and independent graph validation -> reachability/deadlock/recurrence -> eventuality and response obligations -> finite monitors and generalized Büchi acceptance -> shared graph/product substrates -> typed/textual/declarative specification frontends -> bounded state properties -> product/staged temporal budgets -> iterative deep-graph SCC traversal -> opt-in exact-action weak fairness -> bounded/staged enablement provenance -> weak-fair response and monitor composition -> opt-in exact-action strong fairness -> proof-honest bounded/staged strong-fair verification -> strong-fair response/temporal/multi-response/monitor composition -> explicit combined weak/strong fairness -> proof-honest bounded/staged combined-fair Büchi -> combined-fair response/action-temporal -> external combined-fair temporal CLI -> combined-fair multi-response -> combined-fair finite-monitor backend and direct CLI/reporting.

M50 seals combined fairness across the finite-monitor backend and direct monitor CLI. The next highest-value integration gap is **Milestone 51: external combined-fair multi-response CLI/reporting**. M49 already provides the typed/backend semantics, but the direct `respond dual-grant` surface still exposes only the historical no-fairness path.

Acceptance criteria for M51:

- extend direct multi-response CLI routing to explicit weak, strong, and genuinely mixed `FairnessProfile` assumptions without changing the textual temporal grammar;
- preserve exact no-fair/weak-only/strong-only compatibility, per-clause identity, pending-vector finite/lasso evidence, and fairness-only-on-infinite-executions semantics;
- expose unbounded, product-bounded, and staged routes with canonical weak/strong reporting, overlap canonicalization, model-before-product cutoff provenance, and exit `3` for unresolved work;
- add built-binary regressions for mixed filtering, overlap, finite pending terminals, retained real lassos, product/model cutoffs, malformed assumptions, and historical no-option compatibility;
- reuse the sealed M49 multi-response adapters and existing fairness report/limit parsing surfaces rather than introducing another traversal or fairness representation;
- add no multi-clause textual temporal grammar, fairness-by-default behavior, wall-clock proof bound, or performance claim.
'''
text = prefix + roadmap
readme.write_text(text)

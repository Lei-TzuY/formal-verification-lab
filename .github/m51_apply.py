from pathlib import Path


def replace_once(text, old, new, label):
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


# Add a finite pending-terminal teaching model for external response semantics.
examples = Path("src/multi_response_examples.rs")
text = examples.read_text()
marker = '''/// Class B may stutter forever while its response is enabled. Without a
/// fairness assumption, the class-B response obligation is violated.
pub fn unfair_dual_response_protocol() -> Result<TransitionSystem<DualResponseState>, ModelError> {
    dual_response_model("dual-response-unfair-b", true)
}

'''
addition = marker + '''/// A finite maximal execution terminates after requesting class B without a
/// response. Fairness constrains only infinite executions, so this remains a
/// real pending-terminal violation under weak, strong, or combined fairness.
pub fn finite_pending_dual_response_protocol(
) -> Result<TransitionSystem<DualResponseState>, ModelError> {
    TransitionSystemBuilder::new("dual-response-finite-pending-b", |state: &DualResponseState| {
        Ok(match state.phase {
            DualResponsePhase::Idle => vec![Transition::new(
                "request-b",
                DualResponseState {
                    phase: DualResponsePhase::AwaitB,
                },
            )],
            DualResponsePhase::AwaitA | DualResponsePhase::ReadyB | DualResponsePhase::AwaitB => {
                Vec::new()
            }
        })
    })
    .state_variable("phase", "dual response protocol phase")
    .initial_state(DualResponseState {
        phase: DualResponsePhase::Idle,
    })
    .safety_invariant("recognized-phase", |_state: &DualResponseState| true)
    .build()
}

'''
text = replace_once(text, marker, addition, "finite pending dual-response model")
examples.write_text(text)

# Add canonical fairness-profile report adapters for multi-response results.
report = Path("src/fairness_report.rs")
text = report.read_text()
text = replace_once(
    text,
    "//! or cutoff semantics. M50 applies the same reporting contract to finite monitors.\n",
    "//! or cutoff semantics. M50 applies the same reporting contract to finite monitors,\n//! and M51 applies it to the external multi-response command surface.\n",
    "fairness report module docs",
)
text = replace_once(
    text,
    "use crate::monitor_report::{\n    render_analysis_monitor_report, render_bounded_monitor_report, render_monitor_report,\n};\n",
    "use crate::monitor_report::{\n    render_analysis_monitor_report, render_bounded_monitor_report, render_monitor_report,\n};\nuse crate::multi_response::{\n    AnalysisMultiResponseResult, BoundedMultiResponseResult, MultiResponseResult,\n};\nuse crate::multi_response_report::{\n    render_analysis_multi_response_report, render_bounded_multi_response_report,\n    render_multi_response_report,\n};\n",
    "fairness report multi-response imports",
)
insert_marker = '''/// Render one unbounded finite-monitor result with the exact-action weak
/// fairness assumptions that filtered only its infinite progress cycles.
'''
functions = '''/// Render one unbounded multi-response result with both canonical fairness
/// classes. Overlapping actions appear only in the strong class.
pub fn render_fairness_profile_multi_response_report<S: Debug>(
    model_name: &str,
    result: &MultiResponseResult<S>,
    profile: &FairnessProfile,
) -> String {
    let mut output = render_multi_response_report(model_name, result);
    append_fairness(&mut output, "weak", profile.weak_actions());
    append_fairness(&mut output, "strong", profile.strong_actions());
    output
}

/// Render product-bounded multi-response verification with both canonical
/// fairness classes while preserving canonical product cutoff accounting.
pub fn render_bounded_fairness_profile_multi_response_report<S: Debug>(
    model_name: &str,
    result: &BoundedMultiResponseResult<S>,
    profile: &FairnessProfile,
) -> String {
    let mut output = render_bounded_multi_response_report(model_name, result);
    append_fairness(&mut output, "weak", profile.weak_actions());
    append_fairness(&mut output, "strong", profile.strong_actions());
    output
}

/// Render staged model/product multi-response verification with both canonical
/// fairness classes and unchanged stage-qualified cutoff provenance.
pub fn render_analysis_fairness_profile_multi_response_report<S: Debug>(
    model_name: &str,
    result: &AnalysisMultiResponseResult<S>,
    profile: &FairnessProfile,
) -> String {
    let mut output = render_analysis_multi_response_report(model_name, result);
    append_fairness(&mut output, "weak", profile.weak_actions());
    append_fairness(&mut output, "strong", profile.strong_actions());
    output
}

'''
text = replace_once(text, insert_marker, functions + insert_marker, "multi-response fairness reports")
report.write_text(text)

# Wire direct multi-response CLI routing to the sealed M49 adapters.
main = Path("src/main.rs")
text = main.read_text()
text = replace_once(
    text,
    "    render_analysis_fairness_profile_monitor_report,\n    render_analysis_fairness_profile_temporal_report, render_analysis_strong_fair_monitor_report,\n",
    "    render_analysis_fairness_profile_monitor_report,\n    render_analysis_fairness_profile_multi_response_report,\n    render_analysis_fairness_profile_temporal_report, render_analysis_strong_fair_monitor_report,\n",
    "main fairness report analysis import",
)
text = replace_once(
    text,
    "    render_bounded_weak_fair_temporal_report, render_fairness_profile_monitor_report,\n    render_fairness_profile_temporal_report, render_strong_fair_monitor_report,\n",
    "    render_bounded_weak_fair_temporal_report,\n    render_bounded_fairness_profile_multi_response_report, render_fairness_profile_monitor_report,\n    render_fairness_profile_multi_response_report, render_fairness_profile_temporal_report,\n    render_strong_fair_monitor_report,\n",
    "main fairness report bounded import",
)
text = replace_once(
    text,
    "use formal_verification_lab::multi_response::{\n    check_multi_response, check_multi_response_with_limits,\n    check_multi_response_with_product_limits, MultiResponseProperty, MultiResponseStatus,\n    ResponseClause,\n};\n",
    "use formal_verification_lab::multi_response::{\n    check_multi_response, check_multi_response_with_fairness_profile,\n    check_multi_response_with_fairness_profile_and_limits,\n    check_multi_response_with_fairness_profile_and_product_limits, check_multi_response_with_limits,\n    check_multi_response_with_product_limits, MultiResponseProperty, MultiResponseStatus,\n    ResponseClause,\n};\n",
    "main combined multi-response imports",
)
text = replace_once(
    text,
    "use formal_verification_lab::multi_response_examples::{\n    dual_response_protocol, unfair_dual_response_protocol,\n};\n",
    "use formal_verification_lab::multi_response_examples::{\n    dual_response_protocol, finite_pending_dual_response_protocol, unfair_dual_response_protocol,\n};\n",
    "main finite pending example import",
)
text = replace_once(
    text,
    '''        "dual-grant-unfair-b" => run_multi_response(
            unfair_dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
            option_args,
        ),
        _ => Err(format!(
            "unknown response query '{query}'; expected request-grant, request-grant-unfair, dual-grant, or dual-grant-unfair-b"
        )),
''',
    '''        "dual-grant-unfair-b" => run_multi_response(
            unfair_dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
            option_args,
        ),
        "dual-grant-terminal-b" => run_multi_response(
            finite_pending_dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
            option_args,
        ),
        _ => Err(format!(
            "unknown response query '{query}'; expected request-grant, request-grant-unfair, dual-grant, dual-grant-unfair-b, or dual-grant-terminal-b"
        )),
''',
    "main finite pending response route",
)
old_run_multi = '''fn run_multi_response<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    property: MultiResponseProperty,
    option_args: &[String],
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    if option_args.is_empty() {
        let result = check_multi_response(&model, &property).map_err(|error| error.to_string())?;
        print!("{}", render_multi_response_report(model.name(), &result));
        return Ok(match result.status {
            MultiResponseStatus::Satisfied => ExitCode::SUCCESS,
            MultiResponseStatus::Violated => ExitCode::from(7),
        });
    }

    if contains_model_limit_flag(option_args) {
        let limits = parse_analysis_limits(option_args)?;
        let result = check_multi_response_with_limits(&model, &property, limits)
            .map_err(|error| error.to_string())?;
        print!(
            "{}",
            render_analysis_multi_response_report(model.name(), &result)
        );
        return Ok(match &result.outcome {
            AnalysisOutcome::Conclusive(MultiResponseStatus::Satisfied) => ExitCode::SUCCESS,
            AnalysisOutcome::Conclusive(MultiResponseStatus::Violated) => ExitCode::from(7),
            AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
        });
    }

    let limits = parse_product_limits(option_args)?;
    let result = check_multi_response_with_product_limits(&model, &property, limits)
        .map_err(|error| error.to_string())?;
    print!(
        "{}",
        render_bounded_multi_response_report(model.name(), &result)
    );
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated) => ExitCode::from(7),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}
'''
new_run_multi = '''fn run_multi_response<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    property: MultiResponseProperty,
    option_args: &[String],
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let has_fairness = option_args.iter().any(|arg| {
        arg == "--weak-fair-action" || arg == "--strong-fair-action"
    });
    if has_fairness {
        let options = parse_temporal_options(option_args)?;
        if options.has_model_limits {
            let limits = AnalysisLimits::new(options.model_limits, options.product_limits);
            let result = check_multi_response_with_fairness_profile_and_limits(
                &model,
                &property,
                &options.fairness_profile,
                limits,
            )
            .map_err(|error| error.to_string())?;
            print!(
                "{}",
                render_analysis_fairness_profile_multi_response_report(
                    model.name(),
                    &result,
                    &options.fairness_profile,
                )
            );
            return Ok(match &result.outcome {
                AnalysisOutcome::Conclusive(MultiResponseStatus::Satisfied) => ExitCode::SUCCESS,
                AnalysisOutcome::Conclusive(MultiResponseStatus::Violated) => ExitCode::from(7),
                AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
            });
        }

        if options.has_product_limits {
            let result = check_multi_response_with_fairness_profile_and_product_limits(
                &model,
                &property,
                &options.fairness_profile,
                options.product_limits,
            )
            .map_err(|error| error.to_string())?;
            print!(
                "{}",
                render_bounded_fairness_profile_multi_response_report(
                    model.name(),
                    &result,
                    &options.fairness_profile,
                )
            );
            return Ok(match &result.outcome {
                BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied) => ExitCode::SUCCESS,
                BoundedOutcome::Conclusive(MultiResponseStatus::Violated) => ExitCode::from(7),
                BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
            });
        }

        let result = check_multi_response_with_fairness_profile(
            &model,
            &property,
            &options.fairness_profile,
        )
        .map_err(|error| error.to_string())?;
        print!(
            "{}",
            render_fairness_profile_multi_response_report(
                model.name(),
                &result,
                &options.fairness_profile,
            )
        );
        return Ok(match result.status {
            MultiResponseStatus::Satisfied => ExitCode::SUCCESS,
            MultiResponseStatus::Violated => ExitCode::from(7),
        });
    }

    if option_args.is_empty() {
        let result = check_multi_response(&model, &property).map_err(|error| error.to_string())?;
        print!("{}", render_multi_response_report(model.name(), &result));
        return Ok(match result.status {
            MultiResponseStatus::Satisfied => ExitCode::SUCCESS,
            MultiResponseStatus::Violated => ExitCode::from(7),
        });
    }

    if contains_model_limit_flag(option_args) {
        let limits = parse_analysis_limits(option_args)?;
        let result = check_multi_response_with_limits(&model, &property, limits)
            .map_err(|error| error.to_string())?;
        print!(
            "{}",
            render_analysis_multi_response_report(model.name(), &result)
        );
        return Ok(match &result.outcome {
            AnalysisOutcome::Conclusive(MultiResponseStatus::Satisfied) => ExitCode::SUCCESS,
            AnalysisOutcome::Conclusive(MultiResponseStatus::Violated) => ExitCode::from(7),
            AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
        });
    }

    let limits = parse_product_limits(option_args)?;
    let result = check_multi_response_with_product_limits(&model, &property, limits)
        .map_err(|error| error.to_string())?;
    print!(
        "{}",
        render_bounded_multi_response_report(model.name(), &result)
    );
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(MultiResponseStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(MultiResponseStatus::Violated) => ExitCode::from(7),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}
'''
text = replace_once(text, old_run_multi, new_run_multi, "main multi-response fairness routing")
old_usage = '''respond <request-grant|request-grant-unfair|dual-grant|dual-grant-unfair-b> [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | monitor'''
new_usage = '''respond <request-grant|request-grant-unfair> [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | respond <dual-grant|dual-grant-unfair-b|dual-grant-terminal-b> [--weak-fair-action ACTION]... [--strong-fair-action ACTION]... [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | monitor'''
text = replace_once(text, old_usage, new_usage, "main response usage")
main.write_text(text)

# Built-binary coverage for external multi-response fairness routing.
cli = Path("tests/multi_response_combined_fairness_cli.rs")
if cli.exists():
    raise SystemExit("M51 CLI test unexpectedly already exists")
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
fn mixed_multi_response_fairness_routes_and_reports_both_classes() {
    let output = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("multi-response: SATISFIED"));
    assert!(text.contains("weak fairness actions: 1"));
    assert!(text.contains("weak-fair action: \"grant-b\""));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn overlapping_multi_response_fairness_is_canonicalized_to_strong() {
    let output = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "grant-b",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("weak fairness actions: 0"));
    assert!(text.contains("strong fairness actions: 1"));
    assert!(text.contains("strong-fair action: \"grant-b\""));
}

#[test]
fn fairness_never_excuses_a_finite_pending_multi_response_terminal() {
    let output = run(&[
        "respond",
        "dual-grant-terminal-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
    ]);
    assert_eq!(output.status.code(), Some(7));
    let text = stdout(&output);
    assert!(text.contains("multi-response: VIOLATED"));
    assert!(text.contains("violated clause: class-b"));
    assert!(text.contains("counterexample: PENDING_TERMINAL"));
    assert!(text.contains("weak-fair action: \"grant-b\""));
    assert!(text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn mixed_multi_response_product_and_model_cutoffs_remain_inconclusive() {
    let product = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
        "--max-product-transitions",
        "3",
    ]);
    assert_eq!(product.status.code(), Some(3));
    let product_text = stdout(&product);
    assert!(product_text.contains("multi-response: INCONCLUSIVE"));
    assert!(product_text.contains("product inconclusive reason: transition limit reached (max 3)"));
    assert!(product_text.contains("weak-fair action: \"grant-b\""));
    assert!(product_text.contains("strong-fair action: \"unrelated\""));

    let model = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--strong-fair-action",
        "unrelated",
        "--max-model-transitions",
        "2",
    ]);
    assert_eq!(model.status.code(), Some(3));
    let model_text = stdout(&model);
    assert!(model_text.contains("analysis inconclusive stage: model"));
    assert!(model_text.contains("analysis inconclusive reason: transition limit reached (max 2)"));
    assert!(model_text.contains("weak-fair action: \"grant-b\""));
    assert!(model_text.contains("strong-fair action: \"unrelated\""));
}

#[test]
fn multi_response_fairness_validation_fails_closed() {
    let duplicate = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--weak-fair-action",
        "grant-b",
        "--weak-fair-action",
        "grant-b",
    ]);
    assert_eq!(duplicate.status.code(), Some(2));
    assert!(stderr(&duplicate).contains("duplicate weak-fair action 'grant-b'"));

    let missing = run(&[
        "respond",
        "dual-grant-unfair-b",
        "--strong-fair-action",
    ]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(stderr(&missing).contains("option '--strong-fair-action' requires an action value"));
}

#[test]
fn historical_no_option_multi_response_path_and_usage_remain_explicit() {
    let output = run(&["respond", "dual-grant"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("multi-response: SATISFIED"));
    assert!(!text.contains("weak fairness actions:"));
    assert!(!text.contains("strong fairness actions:"));

    let usage = run(&["respond", "dual-grant", "--unknown-option", "1"]);
    assert_eq!(usage.status.code(), Some(2));
    let usage_text = stderr(&usage);
    assert!(usage_text.contains("respond <dual-grant|dual-grant-unfair-b|dual-grant-terminal-b>"));
    assert!(usage_text.contains("[--weak-fair-action ACTION]..."));
    assert!(usage_text.contains("[--strong-fair-action ACTION]..."));
}
''')

# Seal M51 and promote the next external response gap.
readme = Path("README.md")
text = readme.read_text()
text = replace_once(
    text,
    "M49 composes the same canonical profile with multi-response verification, and M50 composes it with finite-monitor verification while preserving exact no-fair, weak-only, and strong-only compatibility paths.",
    "M49 composes the same canonical profile with multi-response verification, M50 composes it with finite-monitor verification, and M51 exposes combined-fair multi-response verification on the direct `respond dual-grant*` CLI while preserving exact no-fair, weak-only, and strong-only compatibility paths.",
    "README current capability M51",
)
text = replace_once(
    text,
    "M49 adds typed/backend mixed-fairness multi-response composition across unbounded, product-bounded, and staged APIs. M50 adds the same combined profile to finite-monitor verification and exposes mixed weak/strong assumptions on the direct `monitor` CLI with canonical overlap handling.",
    "M49 adds typed/backend mixed-fairness multi-response composition across unbounded, product-bounded, and staged APIs. M50 adds the same combined profile to finite-monitor verification and exposes mixed weak/strong assumptions on the direct `monitor` CLI with canonical overlap handling. M51 exposes the M49 multi-response authority through direct `respond dual-grant*` routes with canonical weak/strong reporting and bounded/staged cutoff provenance.",
    "README current surface M51",
)
text = replace_once(
    text,
    "| M50 | Combined-fair finite-monitor composition plus direct mixed-fair monitor CLI/reporting with rejecting/finite-terminal precedence and bounded/staged provenance. |\n",
    "| M50 | Combined-fair finite-monitor composition plus direct mixed-fair monitor CLI/reporting with rejecting/finite-terminal precedence and bounded/staged provenance. |\n| M51 | External combined-fair multi-response CLI/reporting with canonical overlap, finite pending-terminal evidence, and bounded/staged provenance. |\n",
    "README M51 milestone row",
)
section = '''### M51 — external combined-fair multi-response CLI/reporting

M51 exposes the sealed M49 multi-response fairness-profile adapters on the direct `respond dual-grant*` command surface without adding another verification engine. The direct multi-response routes accept repeated `--weak-fair-action` and `--strong-fair-action` declarations, construct the canonical M45 `FairnessProfile`, and route unbounded, product-bounded, or staged requests into the corresponding M49 authority.

No-option multi-response invocations preserve the historical M11/M25/M28 paths exactly. Fairness constrains only infinite executions: a real finite terminal with a pending clause remains a violation. `dual-grant-terminal-b` is an executable teaching model for that boundary. Overlapping weak/strong declarations are canonicalized into the strong class, and reports append both canonical assumption classes separately from unchanged multi-response evidence.

Product-only and staged routes retain existing accounting and proof-honest cutoff semantics. Unresolved work returns exit `3`; conclusive multi-response violations retain exit `7`; malformed or duplicate fairness input returns exit `2`. Built-binary regressions cover mixed routing, overlap canonicalization, finite pending terminals, product/model cutoffs, malformed assumptions, explicit usage, and historical no-option compatibility. The M49 **28,672-case** differential remains the semantic authority underneath this external wiring.

M51 adds no multi-clause textual temporal grammar, fairness-by-default behavior, second traversal/fairness representation, wall-clock proof bound, or performance claim.

'''
text = replace_once(text, "## Architecture\n", section + "## Architecture\n", "README M51 section")
text = text.replace(
    "M25–M29/M33/M39–M50 staged temporal bounds",
    "M25–M29/M33/M39–M51 staged temporal bounds",
)
text = replace_once(
    text,
    "- M45–M50 provide explicit combined weak/strong fairness semantics through generalized Büchi, single-response, typed/textual/declarative action-temporal, typed/backend multi-response, and finite-monitor APIs across unbounded, product-bounded, and staged analysis; M48 exposes mixed temporal assumptions on the CLI and M50 exposes mixed finite-monitor assumptions on the direct `monitor` CLI.\n- M35/M42/M49 expose multi-response weak/strong/combined fairness through typed/backend APIs. The direct `respond dual-grant` CLI remains the historical no-fairness surface, and no multi-clause textual temporal syntax is introduced.\n",
    "- M45–M51 provide explicit combined weak/strong fairness semantics through generalized Büchi, single-response, typed/textual/declarative action-temporal, multi-response, and finite-monitor APIs across unbounded, product-bounded, and staged analysis; M48 exposes mixed temporal assumptions on the CLI, M50 exposes mixed finite-monitor assumptions on `monitor`, and M51 exposes mixed multi-response assumptions on direct `respond dual-grant*` routes.\n- M35/M42/M49 provide the multi-response weak/strong/combined fairness backend authority, and M51 exposes it externally for the direct multi-response teaching models. No multi-clause textual temporal syntax is introduced.\n",
    "README combined multi-response limitations",
)
prefix, marker, _ = text.partition("## Roadmap\n")
if not marker:
    raise SystemExit("README roadmap marker missing")
roadmap = '''## Roadmap

Milestones 1–51 now form a coherent explicit-state stack: safety and bounded honesty -> typed models and independent graph validation -> reachability/deadlock/recurrence -> eventuality and response obligations -> finite monitors and generalized Büchi acceptance -> shared graph/product substrates -> typed/textual/declarative specification frontends -> bounded state properties -> product/staged temporal budgets -> iterative deep-graph SCC traversal -> opt-in exact-action weak fairness -> bounded/staged enablement provenance -> weak-fair response and monitor composition -> opt-in exact-action strong fairness -> proof-honest bounded/staged strong-fair verification -> strong-fair response/temporal/multi-response/monitor composition -> explicit combined weak/strong fairness -> proof-honest bounded/staged combined-fair Büchi -> combined-fair response/action-temporal -> external combined-fair temporal CLI -> combined-fair multi-response -> combined-fair finite-monitor backend/direct CLI -> external combined-fair multi-response CLI/reporting.

M51 seals the direct multi-response fairness surface. The next highest-value integration gap is **Milestone 52: external combined-fair single-response CLI/reporting**. The sealed response adapters already exist through M34/M40/M47, but direct `respond request-grant*` invocations still expose only historical no-fairness and resource-limit routing.

Acceptance criteria for M52:

- expose explicit weak, strong, and genuinely mixed `FairnessProfile` assumptions on direct `respond request-grant*` routes while preserving the no-option compatibility path;
- reuse the sealed single-response fairness adapters and the same fairness/limit parser and report composition pattern, with no new response semantics or traversal;
- preserve finite pending-terminal violations, deterministic lasso evidence, overlap canonicalization, model-before-product cutoff provenance, exit `3` for unresolved work, and exit `7` for direct response violations;
- add built-binary regressions for weak/strong/mixed routing, overlap, finite terminals, retained real lassos, product/model cutoffs, malformed assumptions, usage, and historical no-option behavior;
- keep fairness external to the textual temporal grammar and add no fairness-by-default behavior, wall-clock proof bound, or performance claim.
'''
readme.write_text(prefix + roadmap)

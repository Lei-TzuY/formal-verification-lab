use formal_verification_lab::bounded::{AnalysisLimits, AnalysisOutcome, BoundedOutcome};
use formal_verification_lab::buchi::{
    check_buchi, check_buchi_with_limits, check_buchi_with_product_limits, BuchiAutomaton,
    BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{
    alternating_pulses, finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::buchi_report::{
    render_analysis_buchi_report, render_bounded_buchi_report, render_buchi_report,
};
use formal_verification_lab::checker::{check_with_limits, ExplorationLimits, VerificationStatus};
use formal_verification_lab::eventuality::{
    check_eventuality, EventualityProperty, EventualityStatus,
};
use formal_verification_lab::eventuality_report::render_eventuality_report;
use formal_verification_lab::exact_state::{
    check_exact_state_property, check_exact_state_property_with_limits, parse_exact_state_property,
    ExactStateStatus,
};
use formal_verification_lab::exact_state_report::{
    render_bounded_exact_state_report, render_exact_state_report,
};
use formal_verification_lab::examples::{
    bounded_counter, buggy_mutex, buggy_peterson_mutex, commuting_counters, peterson_mutex,
    traffic_light, CounterState, TrafficLightState,
};
use formal_verification_lab::fairness::WeakFairness;
use formal_verification_lab::fairness_report::render_weak_fair_temporal_report;
use formal_verification_lab::monitor::{
    check_monitor, check_monitor_with_limits, check_monitor_with_product_limits, FiniteMonitor,
    MonitorStatus,
};
use formal_verification_lab::monitor_examples::{
    invalid_double_open_protocol, session_monitor, session_protocol, stuck_committed_protocol,
};
use formal_verification_lab::monitor_report::{
    render_analysis_monitor_report, render_bounded_monitor_report, render_monitor_report,
};
use formal_verification_lab::multi_response::{
    check_multi_response, check_multi_response_with_limits,
    check_multi_response_with_product_limits, MultiResponseProperty, MultiResponseStatus,
    ResponseClause,
};
use formal_verification_lab::multi_response_examples::{
    dual_response_protocol, unfair_dual_response_protocol,
};
use formal_verification_lab::multi_response_report::{
    render_analysis_multi_response_report, render_bounded_multi_response_report,
    render_multi_response_report,
};
use formal_verification_lab::property::{
    check_deadlock, check_reachability, DeadlockProperty, DeadlockStatus, ReachabilityProperty,
    ReachabilityStatus,
};
use formal_verification_lab::proposition::{
    check_proposition_property, check_proposition_property_with_limits, PropositionPropertySpec,
};
use formal_verification_lab::proposition_expr::{
    check_proposition_expression_property, check_proposition_expression_property_with_limits,
    parse_proposition_expression, PropositionExpressionPropertySpec,
};
use formal_verification_lab::proposition_expr_report::{
    render_bounded_proposition_expression_report, render_proposition_expression_report,
};
use formal_verification_lab::proposition_report::{
    render_bounded_proposition_report, render_proposition_report,
};
use formal_verification_lab::recurrence::analyze_recurrence;
use formal_verification_lab::reduction::{audit_sleep_set_reduction, IndependenceRelation};
use formal_verification_lab::report::{
    render_deadlock_report, render_reachability_report, render_recurrence_report, render_report,
};
use formal_verification_lab::response::{
    check_response, check_response_with_limits, check_response_with_product_limits,
    ResponseProperty, ResponseStatus,
};
use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol,
};
use formal_verification_lab::response_report::{
    render_analysis_response_report, render_bounded_response_report, render_response_report,
};
use formal_verification_lab::safety::{
    check_safety_assertion, check_safety_assertion_with_limits, PropositionSafetySpec, SafetyStatus,
};
use formal_verification_lab::safety_report::{render_bounded_safety_report, render_safety_report};
use formal_verification_lab::temporal::{
    check_action_temporal, check_action_temporal_with_limits,
    check_action_temporal_with_product_limits, check_action_temporal_with_weak_fairness,
    ActionAtom, ActionTemporalSpec, TemporalStatus,
};
use formal_verification_lab::temporal_parse::parse_action_temporal;
use formal_verification_lab::temporal_report::{
    render_analysis_temporal_report, render_bounded_temporal_report, render_temporal_report,
};
use formal_verification_lab::{parse_declarative_document, parse_declarative_model};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    match args.as_slice() {
        [] => {
            print_examples();
            Ok(ExitCode::SUCCESS)
        }
        [command] if command == "list" => {
            print_examples();
            Ok(ExitCode::SUCCESS)
        }
        [command, rest @ ..] if command == "run" => run_command(rest),
        [command, rest @ ..] if command == "reduce" => reduce_command(rest),
        [command, rest @ ..] if command == "reach" => reach_command(rest),
        [command, rest @ ..] if command == "deadlock" => deadlock_command(rest),
        [command, rest @ ..] if command == "scc" => scc_command(rest),
        [command, rest @ ..] if command == "eventually" => eventuality_command(rest),
        [command, rest @ ..] if command == "respond" => response_command(rest),
        [command, rest @ ..] if command == "monitor" => monitor_command(rest),
        [command, rest @ ..] if command == "buchi" => buchi_command(rest),
        [command, rest @ ..] if command == "temporal" => temporal_command(rest),
        [command, rest @ ..] if command == "state" => state_command(rest),
        [command, rest @ ..] if command == "proposition" => proposition_command(rest),
        _ => Err(usage()),
    }
}

fn run_command(args: &[String]) -> Result<ExitCode, String> {
    let (example, option_args) = args.split_first().ok_or_else(usage)?;
    let limits = parse_limits(option_args)?;

    match example.as_str() {
        "counter" => run_model(
            bounded_counter().map_err(|error| error.to_string())?,
            limits,
        ),
        "mutex-bug" => run_model(buggy_mutex().map_err(|error| error.to_string())?, limits),
        "traffic-light" => run_model(traffic_light().map_err(|error| error.to_string())?, limits),
        "peterson" => run_model(peterson_mutex().map_err(|error| error.to_string())?, limits),
        "peterson-bug" => run_model(
            buggy_peterson_mutex().map_err(|error| error.to_string())?,
            limits,
        ),
        "commuting-counters" => run_model(
            commuting_counters().map_err(|error| error.to_string())?,
            limits,
        ),
        _ => Err(format!(
            "unknown example '{example}'; expected counter, mutex-bug, traffic-light, peterson, peterson-bug, or commuting-counters"
        )),
    }
}

fn reduce_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [example] if example == "commuting-counters" => {
            let model = commuting_counters().map_err(|error| error.to_string())?;
            let relation = IndependenceRelation::new()
                .with_pair("left:increment", "right:increment")
                .map_err(|error| error.to_string())?;
            let audit =
                audit_sleep_set_reduction(&model, &relation).map_err(|error| error.to_string())?;

            println!("model: {}", model.name());
            println!("reduction audit: MATCH");
            println!("independence pairs: {}", relation.pair_count());
            println!(
                "exhaustive status: {}",
                status_label(audit.exhaustive.status)
            );
            println!("exhaustive states: {}", audit.exhaustive.discovered_states);
            println!(
                "exhaustive transitions: {}",
                audit.exhaustive.explored_transitions
            );
            println!("reduced status: {}", status_label(audit.reduced.status));
            println!("reduced states: {}", audit.reduced.discovered_states);
            println!(
                "reduced transitions: {}",
                audit.reduced.explored_transitions
            );
            println!("pruned transitions: {}", audit.reduced.pruned_transitions);

            Ok(status_exit_code(audit.exhaustive.status))
        }
        [example] => Err(format!(
            "unknown reduction example '{example}'; expected commuting-counters"
        )),
        _ => Err(usage()),
    }
}

fn reach_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [query] if query == "counter-three" => {
            run_counter_reachability("reaches-three", |state| state.value == 3)
        }
        [query] if query == "counter-four" => {
            run_counter_reachability("reaches-four", |state| state.value == 4)
        }
        [query] => Err(format!(
            "unknown reachability query '{query}'; expected counter-three or counter-four"
        )),
        _ => Err(usage()),
    }
}

fn run_counter_reachability(
    property_name: &str,
    target: impl Fn(&CounterState) -> bool + Send + Sync + 'static,
) -> Result<ExitCode, String> {
    let model = bounded_counter().map_err(|error| error.to_string())?;
    let property =
        ReachabilityProperty::new(property_name, target).map_err(|error| error.to_string())?;
    let result = check_reachability(&model, &property).map_err(|error| error.to_string())?;
    print!("{}", render_reachability_report(model.name(), &result));

    Ok(match result.status {
        ReachabilityStatus::Reachable => ExitCode::SUCCESS,
        ReachabilityStatus::Unreachable => ExitCode::from(4),
    })
}

fn deadlock_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [query] if query == "counter-terminal-ok" => run_counter_deadlock(
            "counter-completion-is-terminal",
            |state: &CounterState| state.value == 3,
        ),
        [query] if query == "counter-terminal-forbidden" => run_counter_deadlock(
            "no-terminal-state-is-allowed",
            |_state: &CounterState| false,
        ),
        [query] => Err(format!(
            "unknown deadlock query '{query}'; expected counter-terminal-ok or counter-terminal-forbidden"
        )),
        _ => Err(usage()),
    }
}

fn run_counter_deadlock(
    property_name: &str,
    allowed_terminal: impl Fn(&CounterState) -> bool + Send + Sync + 'static,
) -> Result<ExitCode, String> {
    let model = bounded_counter().map_err(|error| error.to_string())?;
    let property = DeadlockProperty::new(property_name, allowed_terminal)
        .map_err(|error| error.to_string())?;
    let result = check_deadlock(&model, &property).map_err(|error| error.to_string())?;
    print!("{}", render_deadlock_report(model.name(), &result));

    Ok(match result.status {
        DeadlockStatus::DeadlockFree => ExitCode::SUCCESS,
        DeadlockStatus::DeadlockFound => ExitCode::from(5),
    })
}

fn scc_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [example] if example == "counter" => {
            run_recurrence(bounded_counter().map_err(|error| error.to_string())?)
        }
        [example] if example == "traffic-light" => {
            run_recurrence(traffic_light().map_err(|error| error.to_string())?)
        }
        [example] => Err(format!(
            "unknown SCC example '{example}'; expected counter or traffic-light"
        )),
        _ => Err(usage()),
    }
}

fn run_recurrence<S>(
    model: formal_verification_lab::TransitionSystem<S>,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let analysis = analyze_recurrence(&model).map_err(|error| error.to_string())?;
    print!("{}", render_recurrence_report(model.name(), &analysis));
    Ok(ExitCode::SUCCESS)
}

fn eventuality_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [query] if query == "counter-three" => {
            let model = bounded_counter().map_err(|error| error.to_string())?;
            let property = EventualityProperty::new(
                "all-paths-eventually-three",
                |state: &CounterState| state.value == 3,
            )
            .map_err(|error| error.to_string())?;
            run_eventuality(model, property)
        }
        [query] if query == "counter-four" => {
            let model = bounded_counter().map_err(|error| error.to_string())?;
            let property = EventualityProperty::new(
                "all-paths-eventually-four",
                |state: &CounterState| state.value == 4,
            )
            .map_err(|error| error.to_string())?;
            run_eventuality(model, property)
        }
        [query] if query == "traffic-never" => {
            let model = traffic_light().map_err(|error| error.to_string())?;
            let property = EventualityProperty::new(
                "impossible-traffic-target",
                |_state: &TrafficLightState| false,
            )
            .map_err(|error| error.to_string())?;
            run_eventuality(model, property)
        }
        [query] => Err(format!(
            "unknown eventuality query '{query}'; expected counter-three, counter-four, or traffic-never"
        )),
        _ => Err(usage()),
    }
}

fn run_eventuality<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    property: EventualityProperty<S>,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_eventuality(&model, &property).map_err(|error| error.to_string())?;
    print!("{}", render_eventuality_report(model.name(), &result));
    Ok(match result.status {
        EventualityStatus::Satisfied => ExitCode::SUCCESS,
        EventualityStatus::Violated => ExitCode::from(6),
    })
}

fn response_command(args: &[String]) -> Result<ExitCode, String> {
    let (query, option_args) = args.split_first().ok_or_else(usage)?;
    match query.as_str() {
        "request-grant" => run_response(
            request_grant_protocol().map_err(|error| error.to_string())?,
            single_response_property()?,
            option_args,
        ),
        "request-grant-unfair" => run_response(
            unfair_request_grant_protocol().map_err(|error| error.to_string())?,
            single_response_property()?,
            option_args,
        ),
        "dual-grant" => run_multi_response(
            dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
            option_args,
        ),
        "dual-grant-unfair-b" => run_multi_response(
            unfair_dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
            option_args,
        ),
        _ => Err(format!(
            "unknown response query '{query}'; expected request-grant, request-grant-unfair, dual-grant, or dual-grant-unfair-b"
        )),
    }
}

fn single_response_property() -> Result<ResponseProperty, String> {
    ResponseProperty::new(
        "request-eventually-grant",
        |action| action == "request",
        |action| action == "grant",
    )
    .map_err(|error| error.to_string())
}

fn dual_response_property() -> Result<MultiResponseProperty, String> {
    MultiResponseProperty::new(
        "dual-request-response",
        vec![
            ResponseClause::new(
                "class-a",
                |action| action == "request-a",
                |action| action == "grant-a",
            )
            .map_err(|error| error.to_string())?,
            ResponseClause::new(
                "class-b",
                |action| action == "request-b",
                |action| action == "grant-b",
            )
            .map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())
}

fn run_response<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    property: ResponseProperty,
    option_args: &[String],
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    if option_args.is_empty() {
        let result = check_response(&model, &property).map_err(|error| error.to_string())?;
        print!("{}", render_response_report(model.name(), &result));
        return Ok(match result.status {
            ResponseStatus::Satisfied => ExitCode::SUCCESS,
            ResponseStatus::Violated => ExitCode::from(7),
        });
    }

    if contains_model_limit_flag(option_args) {
        let limits = parse_analysis_limits(option_args)?;
        let result = check_response_with_limits(&model, &property, limits)
            .map_err(|error| error.to_string())?;
        print!("{}", render_analysis_response_report(model.name(), &result));
        return Ok(match &result.outcome {
            AnalysisOutcome::Conclusive(ResponseStatus::Satisfied) => ExitCode::SUCCESS,
            AnalysisOutcome::Conclusive(ResponseStatus::Violated) => ExitCode::from(7),
            AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
        });
    }

    let limits = parse_product_limits(option_args)?;
    let result = check_response_with_product_limits(&model, &property, limits)
        .map_err(|error| error.to_string())?;
    print!("{}", render_bounded_response_report(model.name(), &result));
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(ResponseStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(ResponseStatus::Violated) => ExitCode::from(7),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}

fn run_multi_response<S>(
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

fn monitor_command(args: &[String]) -> Result<ExitCode, String> {
    let (query, option_args) = args.split_first().ok_or_else(usage)?;
    match query.as_str() {
        "session-ok" => run_monitor(
            session_protocol().map_err(|error| error.to_string())?,
            session_monitor().map_err(|error| error.to_string())?,
            option_args,
        ),
        "session-double-open" => run_monitor(
            invalid_double_open_protocol().map_err(|error| error.to_string())?,
            session_monitor().map_err(|error| error.to_string())?,
            option_args,
        ),
        "session-stuck" => run_monitor(
            stuck_committed_protocol().map_err(|error| error.to_string())?,
            session_monitor().map_err(|error| error.to_string())?,
            option_args,
        ),
        _ => Err(format!(
            "unknown monitor query '{query}'; expected session-ok, session-double-open, or session-stuck"
        )),
    }
}

fn run_monitor<S, M>(
    model: formal_verification_lab::TransitionSystem<S>,
    monitor: FiniteMonitor<M>,
    option_args: &[String],
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    M: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    if option_args.is_empty() {
        let result = check_monitor(&model, &monitor).map_err(|error| error.to_string())?;
        print!("{}", render_monitor_report(model.name(), &result));
        return Ok(match result.status {
            MonitorStatus::Satisfied => ExitCode::SUCCESS,
            MonitorStatus::Violated => ExitCode::from(8),
        });
    }

    if contains_model_limit_flag(option_args) {
        let limits = parse_analysis_limits(option_args)?;
        let result = check_monitor_with_limits(&model, &monitor, limits)
            .map_err(|error| error.to_string())?;
        print!("{}", render_analysis_monitor_report(model.name(), &result));
        return Ok(match &result.outcome {
            AnalysisOutcome::Conclusive(MonitorStatus::Satisfied) => ExitCode::SUCCESS,
            AnalysisOutcome::Conclusive(MonitorStatus::Violated) => ExitCode::from(8),
            AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
        });
    }

    let limits = parse_product_limits(option_args)?;
    let result = check_monitor_with_product_limits(&model, &monitor, limits)
        .map_err(|error| error.to_string())?;
    print!("{}", render_bounded_monitor_report(model.name(), &result));
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(MonitorStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(MonitorStatus::Violated) => ExitCode::from(8),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}

fn buchi_command(args: &[String]) -> Result<ExitCode, String> {
    let (query, option_args) = args.split_first().ok_or_else(usage)?;
    match query.as_str() {
        "pulses" => run_buchi(
            alternating_pulses().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::IgnoreTerminals).map_err(|error| error.to_string())?,
            option_args,
        ),
        "pulses-unfair" => run_buchi(
            unfair_second_pulse().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::IgnoreTerminals).map_err(|error| error.to_string())?,
            option_args,
        ),
        "finite-ignore" => run_buchi(
            finite_quiet_run().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::IgnoreTerminals).map_err(|error| error.to_string())?,
            option_args,
        ),
        "finite-strict" => run_buchi(
            finite_quiet_run().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal)
                .map_err(|error| error.to_string())?,
            option_args,
        ),
        _ => Err(format!(
            "unknown Buchi query '{query}'; expected pulses, pulses-unfair, finite-ignore, or finite-strict"
        )),
    }
}

fn run_buchi<S, A>(
    model: formal_verification_lab::TransitionSystem<S>,
    automaton: BuchiAutomaton<A>,
    option_args: &[String],
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    A: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    if option_args.is_empty() {
        let result = check_buchi(&model, &automaton).map_err(|error| error.to_string())?;
        print!("{}", render_buchi_report(model.name(), &result));
        return Ok(match result.status {
            BuchiStatus::Satisfied => ExitCode::SUCCESS,
            BuchiStatus::Violated => ExitCode::from(9),
        });
    }

    if contains_model_limit_flag(option_args) {
        let limits = parse_analysis_limits(option_args)?;
        let result = check_buchi_with_limits(&model, &automaton, limits)
            .map_err(|error| error.to_string())?;
        print!("{}", render_analysis_buchi_report(model.name(), &result));
        return Ok(match &result.outcome {
            AnalysisOutcome::Conclusive(BuchiStatus::Satisfied) => ExitCode::SUCCESS,
            AnalysisOutcome::Conclusive(BuchiStatus::Violated) => ExitCode::from(9),
            AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
        });
    }

    let limits = parse_product_limits(option_args)?;
    let result = check_buchi_with_product_limits(&model, &automaton, limits)
        .map_err(|error| error.to_string())?;
    print!("{}", render_bounded_buchi_report(model.name(), &result));
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(BuchiStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(BuchiStatus::Violated) => ExitCode::from(9),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}

fn temporal_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [command, path, expression, option_args @ ..] if command == "file" => {
            run_temporal_file(path, expression, option_args)
        }
        [command, model, expression, option_args @ ..] if command == "check" => {
            run_temporal_expression(model, expression, option_args)
        }
        [query, option_args @ ..] if query == "request-grant" => run_temporal(
            request_grant_protocol().map_err(|error| error.to_string())?,
            typed_response_spec()?,
            option_args,
        ),
        [query, option_args @ ..] if query == "request-grant-unfair" => run_temporal(
            unfair_request_grant_protocol().map_err(|error| error.to_string())?,
            typed_response_spec()?,
            option_args,
        ),
        [query, option_args @ ..] if query == "pulses" => run_temporal(
            alternating_pulses().map_err(|error| error.to_string())?,
            typed_pulse_spec()?,
            option_args,
        ),
        [query, option_args @ ..] if query == "pulses-unfair" => run_temporal(
            unfair_second_pulse().map_err(|error| error.to_string())?,
            typed_pulse_spec()?,
            option_args,
        ),
        [query, ..] => Err(format!(
            "unknown temporal query '{query}'; expected request-grant, request-grant-unfair, pulses, pulses-unfair, 'check <model> <expression>', or 'file <path> <expression>'"
        )),
        _ => Err(usage()),
    }
}

fn run_temporal_file(
    path: &str,
    expression: &str,
    option_args: &[String],
) -> Result<ExitCode, String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("failed to read declarative model '{path}': {error}"))?;
    let model = parse_declarative_model(&input).map_err(|error| error.to_string())?;
    let spec =
        parse_action_temporal("cli-temporal", expression).map_err(|error| error.to_string())?;
    run_temporal(model, spec, option_args)
}

fn run_temporal_expression(
    model_name: &str,
    expression: &str,
    option_args: &[String],
) -> Result<ExitCode, String> {
    let spec =
        parse_action_temporal("cli-temporal", expression).map_err(|error| error.to_string())?;
    match model_name {
        "request-grant" => run_temporal(
            request_grant_protocol().map_err(|error| error.to_string())?,
            spec,
            option_args,
        ),
        "request-grant-unfair" => run_temporal(
            unfair_request_grant_protocol().map_err(|error| error.to_string())?,
            spec,
            option_args,
        ),
        "pulses" => run_temporal(
            alternating_pulses().map_err(|error| error.to_string())?,
            spec,
            option_args,
        ),
        "pulses-unfair" => run_temporal(
            unfair_second_pulse().map_err(|error| error.to_string())?,
            spec,
            option_args,
        ),
        _ => Err(format!(
            "unknown temporal model '{model_name}'; expected request-grant, request-grant-unfair, pulses, or pulses-unfair"
        )),
    }
}

fn typed_response_spec() -> Result<ActionTemporalSpec, String> {
    ActionTemporalSpec::response(
        "request-eventually-grant",
        ActionAtom::exact("request").map_err(|error| error.to_string())?,
        ActionAtom::exact("grant").map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn typed_pulse_spec() -> Result<ActionTemporalSpec, String> {
    ActionTemporalSpec::all_infinitely_often(
        "infinitely-often-a-and-b",
        vec![
            ActionAtom::exact("pulse-a").map_err(|error| error.to_string())?,
            ActionAtom::exact("pulse-b").map_err(|error| error.to_string())?,
        ],
    )
    .map_err(|error| error.to_string())
}

fn run_temporal<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    spec: ActionTemporalSpec,
    option_args: &[String],
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    if contains_weak_fairness_flag(option_args) {
        if contains_temporal_limit_flag(option_args) {
            return Err(
                "weak fairness cannot be combined with bounded or staged temporal limits"
                    .to_owned(),
            );
        }
        let fairness = parse_weak_fairness(option_args)?;
        let result = check_action_temporal_with_weak_fairness(&model, &spec, &fairness)
            .map_err(|error| error.to_string())?;
        print!(
            "{}",
            render_weak_fair_temporal_report(model.name(), &result, &fairness)
        );
        return Ok(match result.status {
            TemporalStatus::Satisfied => ExitCode::SUCCESS,
            TemporalStatus::Violated => ExitCode::from(10),
        });
    }

    if option_args.is_empty() {
        let result = check_action_temporal(&model, &spec).map_err(|error| error.to_string())?;
        print!("{}", render_temporal_report(model.name(), &result));
        return Ok(match result.status {
            TemporalStatus::Satisfied => ExitCode::SUCCESS,
            TemporalStatus::Violated => ExitCode::from(10),
        });
    }

    if contains_model_limit_flag(option_args) {
        let limits = parse_analysis_limits(option_args)?;
        let result = check_action_temporal_with_limits(&model, &spec, limits)
            .map_err(|error| error.to_string())?;
        print!("{}", render_analysis_temporal_report(model.name(), &result));
        return Ok(match &result.outcome {
            AnalysisOutcome::Conclusive(TemporalStatus::Satisfied) => ExitCode::SUCCESS,
            AnalysisOutcome::Conclusive(TemporalStatus::Violated) => ExitCode::from(10),
            AnalysisOutcome::Inconclusive(_) => ExitCode::from(3),
        });
    }

    let limits = parse_product_limits(option_args)?;
    let result = check_action_temporal_with_product_limits(&model, &spec, limits)
        .map_err(|error| error.to_string())?;
    print!("{}", render_bounded_temporal_report(model.name(), &result));
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(TemporalStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(TemporalStatus::Violated) => ExitCode::from(10),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}

fn state_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [command, path, expression, option_args @ ..] if command == "file" => {
            run_state_file(path, expression, option_args)
        }
        [query] => Err(format!(
            "unknown state query '{query}'; expected 'file <path> <expression> [--max-states N] [--max-transitions N] [--max-depth N]'"
        )),
        _ => Err(usage()),
    }
}

fn run_state_file(
    path: &str,
    expression: &str,
    option_args: &[String],
) -> Result<ExitCode, String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("failed to read declarative model '{path}': {error}"))?;
    let model = parse_declarative_model(&input).map_err(|error| error.to_string())?;
    let spec =
        parse_exact_state_property("cli-state", expression).map_err(|error| error.to_string())?;

    if option_args.is_empty() {
        let result =
            check_exact_state_property(&model, &spec).map_err(|error| error.to_string())?;
        print!("{}", render_exact_state_report(model.name(), &result));
        return Ok(match result.status {
            ExactStateStatus::Satisfied => ExitCode::SUCCESS,
            ExactStateStatus::Violated => ExitCode::from(11),
        });
    }

    let limits = parse_limits(option_args)?;
    let result = check_exact_state_property_with_limits(&model, &spec, limits)
        .map_err(|error| error.to_string())?;
    print!(
        "{}",
        render_bounded_exact_state_report(model.name(), &result)
    );
    Ok(bounded_state_exit_code(&result.outcome))
}

fn proposition_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [command, path, mode, proposition, option_args @ ..] if command == "file" => {
            run_proposition_file(path, mode, proposition, option_args)
        }
        [command, path, mode, expression, option_args @ ..] if command == "expr" => {
            run_proposition_expression_file(path, mode, expression, option_args)
        }
        [command, path, expression, option_args @ ..] if command == "always" => {
            run_safety_file(path, expression, option_args)
        }
        [query] => Err(format!(
            "unknown proposition query '{query}'; expected 'file <path> <reachable|all-eventually> <proposition> [limits]', 'expr <path> <reachable|all-eventually> <expression> [limits]', or 'always <path> <expression> [limits]'"
        )),
        _ => Err(usage()),
    }
}

fn run_proposition_file(
    path: &str,
    mode: &str,
    proposition: &str,
    option_args: &[String],
) -> Result<ExitCode, String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("failed to read declarative model '{path}': {error}"))?;
    let document = parse_declarative_document(&input).map_err(|error| error.to_string())?;
    let spec = match mode {
        "reachable" => PropositionPropertySpec::reachable("cli-proposition", proposition),
        "all-eventually" => PropositionPropertySpec::all_eventually("cli-proposition", proposition),
        _ => {
            return Err(format!(
                "unknown proposition property mode '{mode}'; expected reachable or all-eventually"
            ));
        }
    }
    .map_err(|error| error.to_string())?;

    if option_args.is_empty() {
        let result =
            check_proposition_property(&document, &spec).map_err(|error| error.to_string())?;
        print!(
            "{}",
            render_proposition_report(document.model().name(), &result)
        );
        return Ok(match result.status {
            ExactStateStatus::Satisfied => ExitCode::SUCCESS,
            ExactStateStatus::Violated => ExitCode::from(11),
        });
    }

    let limits = parse_limits(option_args)?;
    let result = check_proposition_property_with_limits(&document, &spec, limits)
        .map_err(|error| error.to_string())?;
    print!(
        "{}",
        render_bounded_proposition_report(document.model().name(), &result)
    );
    Ok(bounded_state_exit_code(&result.outcome))
}

fn run_proposition_expression_file(
    path: &str,
    mode: &str,
    expression: &str,
    option_args: &[String],
) -> Result<ExitCode, String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("failed to read declarative model '{path}': {error}"))?;
    let document = parse_declarative_document(&input).map_err(|error| error.to_string())?;
    let expression = parse_proposition_expression(expression).map_err(|error| error.to_string())?;
    let spec = match mode {
        "reachable" => {
            PropositionExpressionPropertySpec::reachable("cli-proposition-expression", expression)
        }
        "all-eventually" => PropositionExpressionPropertySpec::all_eventually(
            "cli-proposition-expression",
            expression,
        ),
        _ => {
            return Err(format!(
                "unknown proposition property mode '{mode}'; expected reachable or all-eventually"
            ));
        }
    }
    .map_err(|error| error.to_string())?;

    if option_args.is_empty() {
        let result = check_proposition_expression_property(&document, &spec)
            .map_err(|error| error.to_string())?;
        print!(
            "{}",
            render_proposition_expression_report(document.model().name(), &result)
        );
        return Ok(match result.status {
            ExactStateStatus::Satisfied => ExitCode::SUCCESS,
            ExactStateStatus::Violated => ExitCode::from(11),
        });
    }

    let limits = parse_limits(option_args)?;
    let result = check_proposition_expression_property_with_limits(&document, &spec, limits)
        .map_err(|error| error.to_string())?;
    print!(
        "{}",
        render_bounded_proposition_expression_report(document.model().name(), &result)
    );
    Ok(bounded_state_exit_code(&result.outcome))
}

fn run_safety_file(
    path: &str,
    expression: &str,
    option_args: &[String],
) -> Result<ExitCode, String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("failed to read declarative model '{path}': {error}"))?;
    let document = parse_declarative_document(&input).map_err(|error| error.to_string())?;
    let expression = parse_proposition_expression(expression).map_err(|error| error.to_string())?;
    let spec = PropositionSafetySpec::always("cli-safety", expression)
        .map_err(|error| error.to_string())?;

    if option_args.is_empty() {
        let result = check_safety_assertion(&document, &spec).map_err(|error| error.to_string())?;
        print!("{}", render_safety_report(document.model().name(), &result));
        return Ok(match result.status {
            SafetyStatus::Safe => ExitCode::SUCCESS,
            SafetyStatus::Violated => ExitCode::from(12),
        });
    }

    let limits = parse_limits(option_args)?;
    let result = check_safety_assertion_with_limits(&document, &spec, limits)
        .map_err(|error| error.to_string())?;
    print!(
        "{}",
        render_bounded_safety_report(document.model().name(), &result)
    );
    Ok(match &result.outcome {
        BoundedOutcome::Conclusive(SafetyStatus::Safe) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(SafetyStatus::Violated) => ExitCode::from(12),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    })
}

fn bounded_state_exit_code(outcome: &BoundedOutcome<ExactStateStatus>) -> ExitCode {
    match outcome {
        BoundedOutcome::Conclusive(ExactStateStatus::Satisfied) => ExitCode::SUCCESS,
        BoundedOutcome::Conclusive(ExactStateStatus::Violated) => ExitCode::from(11),
        BoundedOutcome::Inconclusive(_) => ExitCode::from(3),
    }
}

fn parse_limits(args: &[String]) -> Result<ExplorationLimits, String> {
    parse_named_limits(args, "--max-states", "--max-transitions", "--max-depth")
}

fn parse_product_limits(args: &[String]) -> Result<ExplorationLimits, String> {
    parse_named_limits(
        args,
        "--max-product-states",
        "--max-product-transitions",
        "--max-product-depth",
    )
}

fn contains_model_limit_flag(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--max-model-states" | "--max-model-transitions" | "--max-model-depth"
        )
    })
}

fn contains_weak_fairness_flag(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--weak-fair-action")
}

fn contains_temporal_limit_flag(args: &[String]) -> bool {
    args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "--max-model-states"
                | "--max-model-transitions"
                | "--max-model-depth"
                | "--max-product-states"
                | "--max-product-transitions"
                | "--max-product-depth"
        )
    })
}

fn parse_weak_fairness(args: &[String]) -> Result<WeakFairness, String> {
    let mut actions = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        if flag != "--weak-fair-action" {
            return Err(format!("unknown option '{flag}'\n{}", usage()));
        }
        let action = args
            .get(index + 1)
            .ok_or_else(|| "option '--weak-fair-action' requires an action value".to_owned())?;
        actions.push(action.clone());
        index += 2;
    }
    WeakFairness::new(actions).map_err(|error| error.to_string())
}

fn parse_analysis_limits(args: &[String]) -> Result<AnalysisLimits, String> {
    let mut model = ExplorationLimits::unbounded();
    let mut product = ExplorationLimits::unbounded();
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("option '{flag}' requires an integer value"))?;
        let parsed = value
            .parse::<usize>()
            .map_err(|_| format!("option '{flag}' requires a non-negative integer"))?;

        match flag {
            "--max-model-states" => set_limit(&mut model.max_states, parsed, flag)?,
            "--max-model-transitions" => set_limit(&mut model.max_transitions, parsed, flag)?,
            "--max-model-depth" => set_limit(&mut model.max_depth, parsed, flag)?,
            "--max-product-states" => set_limit(&mut product.max_states, parsed, flag)?,
            "--max-product-transitions" => set_limit(&mut product.max_transitions, parsed, flag)?,
            "--max-product-depth" => set_limit(&mut product.max_depth, parsed, flag)?,
            _ => return Err(format!("unknown option '{flag}'\n{}", usage())),
        }

        index += 2;
    }

    Ok(AnalysisLimits::new(model, product))
}

fn parse_named_limits(
    args: &[String],
    state_flag: &str,
    transition_flag: &str,
    depth_flag: &str,
) -> Result<ExplorationLimits, String> {
    let mut limits = ExplorationLimits::unbounded();
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("option '{flag}' requires an integer value"))?;
        let parsed = value
            .parse::<usize>()
            .map_err(|_| format!("option '{flag}' requires a non-negative integer"))?;

        if flag == state_flag {
            set_limit(&mut limits.max_states, parsed, flag)?;
        } else if flag == transition_flag {
            set_limit(&mut limits.max_transitions, parsed, flag)?;
        } else if flag == depth_flag {
            set_limit(&mut limits.max_depth, parsed, flag)?;
        } else {
            return Err(format!("unknown option '{flag}'\n{}", usage()));
        }

        index += 2;
    }

    Ok(limits)
}

fn set_limit(slot: &mut Option<usize>, value: usize, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate option '{flag}'"))
    } else {
        Ok(())
    }
}

fn run_model<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    limits: ExplorationLimits,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_with_limits(&model, limits).map_err(|error| error.to_string())?;
    print!("{}", render_report(model.name(), &result));
    Ok(status_exit_code(result.status))
}

fn status_label(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Safe => "SAFE",
        VerificationStatus::Violated => "VIOLATION",
        VerificationStatus::Inconclusive => "INCONCLUSIVE",
    }
}

fn status_exit_code(status: VerificationStatus) -> ExitCode {
    match status {
        VerificationStatus::Safe => ExitCode::SUCCESS,
        VerificationStatus::Violated => ExitCode::from(1),
        VerificationStatus::Inconclusive => ExitCode::from(3),
    }
}

fn usage() -> String {
    "usage: fvlab [list | run <counter|mutex-bug|traffic-light|peterson|peterson-bug|commuting-counters> [--max-states N] [--max-transitions N] [--max-depth N] | reduce commuting-counters | reach <counter-three|counter-four> | deadlock <counter-terminal-ok|counter-terminal-forbidden> | scc <counter|traffic-light> | eventually <counter-three|counter-four|traffic-never> | respond <request-grant|request-grant-unfair|dual-grant|dual-grant-unfair-b> [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | monitor <session-ok|session-double-open|session-stuck> [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | buchi <pulses|pulses-unfair|finite-ignore|finite-strict> [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | temporal <request-grant|request-grant-unfair|pulses|pulses-unfair> [--weak-fair-action ACTION]... [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | temporal check <request-grant|request-grant-unfair|pulses|pulses-unfair> <expression> [--weak-fair-action ACTION]... [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | temporal file <path> <expression> [--weak-fair-action ACTION]... [--max-model-states N] [--max-model-transitions N] [--max-model-depth N] [--max-product-states N] [--max-product-transitions N] [--max-product-depth N] | state file <path> <expression> [--max-states N] [--max-transitions N] [--max-depth N] | proposition file <path> <reachable|all-eventually> <proposition> [--max-states N] [--max-transitions N] [--max-depth N] | proposition expr <path> <reachable|all-eventually> <expression> [--max-states N] [--max-transitions N] [--max-depth N] | proposition always <path> <expression> [--max-states N] [--max-transitions N] [--max-depth N]]"
        .to_owned()
}

fn print_examples() {
    println!("counter\nmutex-bug\ntraffic-light\npeterson\npeterson-bug\ncommuting-counters");
}

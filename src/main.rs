use formal_verification_lab::buchi::{
    check_buchi, BuchiAutomaton, BuchiStatus, FiniteRunPolicy,
};
use formal_verification_lab::buchi_examples::{
    alternating_pulses, finite_quiet_run, pulse_automaton, unfair_second_pulse,
};
use formal_verification_lab::buchi_report::render_buchi_report;
use formal_verification_lab::checker::{check_with_limits, ExplorationLimits, VerificationStatus};
use formal_verification_lab::eventuality::{
    check_eventuality, EventualityProperty, EventualityStatus,
};
use formal_verification_lab::eventuality_report::render_eventuality_report;
use formal_verification_lab::examples::{
    bounded_counter, buggy_mutex, buggy_peterson_mutex, commuting_counters, peterson_mutex,
    traffic_light, CounterState, TrafficLightState,
};
use formal_verification_lab::monitor::{check_monitor, FiniteMonitor, MonitorStatus};
use formal_verification_lab::monitor_examples::{
    invalid_double_open_protocol, session_monitor, session_protocol, stuck_committed_protocol,
};
use formal_verification_lab::monitor_report::render_monitor_report;
use formal_verification_lab::multi_response::{
    check_multi_response, MultiResponseProperty, MultiResponseStatus, ResponseClause,
};
use formal_verification_lab::multi_response_examples::{
    dual_response_protocol, unfair_dual_response_protocol,
};
use formal_verification_lab::multi_response_report::render_multi_response_report;
use formal_verification_lab::property::{
    check_deadlock, check_reachability, DeadlockProperty, DeadlockStatus, ReachabilityProperty,
    ReachabilityStatus,
};
use formal_verification_lab::recurrence::analyze_recurrence;
use formal_verification_lab::reduction::{audit_sleep_set_reduction, IndependenceRelation};
use formal_verification_lab::report::{
    render_deadlock_report, render_reachability_report, render_recurrence_report, render_report,
};
use formal_verification_lab::response::{check_response, ResponseProperty, ResponseStatus};
use formal_verification_lab::response_examples::{
    request_grant_protocol, unfair_request_grant_protocol,
};
use formal_verification_lab::response_report::render_response_report;
use std::env;
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
    match args {
        [query] if query == "request-grant" => run_response(
            request_grant_protocol().map_err(|error| error.to_string())?,
            single_response_property()?,
        ),
        [query] if query == "request-grant-unfair" => run_response(
            unfair_request_grant_protocol().map_err(|error| error.to_string())?,
            single_response_property()?,
        ),
        [query] if query == "dual-grant" => run_multi_response(
            dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
        ),
        [query] if query == "dual-grant-unfair-b" => run_multi_response(
            unfair_dual_response_protocol().map_err(|error| error.to_string())?,
            dual_response_property()?,
        ),
        [query] => Err(format!(
            "unknown response query '{query}'; expected request-grant, request-grant-unfair, dual-grant, or dual-grant-unfair-b"
        )),
        _ => Err(usage()),
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
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_response(&model, &property).map_err(|error| error.to_string())?;
    print!("{}", render_response_report(model.name(), &result));
    Ok(match result.status {
        ResponseStatus::Satisfied => ExitCode::SUCCESS,
        ResponseStatus::Violated => ExitCode::from(7),
    })
}

fn run_multi_response<S>(
    model: formal_verification_lab::TransitionSystem<S>,
    property: MultiResponseProperty,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_multi_response(&model, &property).map_err(|error| error.to_string())?;
    print!("{}", render_multi_response_report(model.name(), &result));
    Ok(match result.status {
        MultiResponseStatus::Satisfied => ExitCode::SUCCESS,
        MultiResponseStatus::Violated => ExitCode::from(7),
    })
}

fn monitor_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [query] if query == "session-ok" => run_monitor(
            session_protocol().map_err(|error| error.to_string())?,
            session_monitor().map_err(|error| error.to_string())?,
        ),
        [query] if query == "session-double-open" => run_monitor(
            invalid_double_open_protocol().map_err(|error| error.to_string())?,
            session_monitor().map_err(|error| error.to_string())?,
        ),
        [query] if query == "session-stuck" => run_monitor(
            stuck_committed_protocol().map_err(|error| error.to_string())?,
            session_monitor().map_err(|error| error.to_string())?,
        ),
        [query] => Err(format!(
            "unknown monitor query '{query}'; expected session-ok, session-double-open, or session-stuck"
        )),
        _ => Err(usage()),
    }
}

fn run_monitor<S, M>(
    model: formal_verification_lab::TransitionSystem<S>,
    monitor: FiniteMonitor<M>,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    M: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_monitor(&model, &monitor).map_err(|error| error.to_string())?;
    print!("{}", render_monitor_report(model.name(), &result));
    Ok(match result.status {
        MonitorStatus::Satisfied => ExitCode::SUCCESS,
        MonitorStatus::Violated => ExitCode::from(8),
    })
}

fn buchi_command(args: &[String]) -> Result<ExitCode, String> {
    match args {
        [query] if query == "pulses" => run_buchi(
            alternating_pulses().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::IgnoreTerminals).map_err(|error| error.to_string())?,
        ),
        [query] if query == "pulses-unfair" => run_buchi(
            unfair_second_pulse().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::IgnoreTerminals).map_err(|error| error.to_string())?,
        ),
        [query] if query == "finite-ignore" => run_buchi(
            finite_quiet_run().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::IgnoreTerminals).map_err(|error| error.to_string())?,
        ),
        [query] if query == "finite-strict" => run_buchi(
            finite_quiet_run().map_err(|error| error.to_string())?,
            pulse_automaton(FiniteRunPolicy::RequireAcceptingTerminal)
                .map_err(|error| error.to_string())?,
        ),
        [query] => Err(format!(
            "unknown Buchi query '{query}'; expected pulses, pulses-unfair, finite-ignore, or finite-strict"
        )),
        _ => Err(usage()),
    }
}

fn run_buchi<S, A>(
    model: formal_verification_lab::TransitionSystem<S>,
    automaton: BuchiAutomaton<A>,
) -> Result<ExitCode, String>
where
    S: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    A: Clone + Eq + std::hash::Hash + std::fmt::Debug,
{
    let result = check_buchi(&model, &automaton).map_err(|error| error.to_string())?;
    print!("{}", render_buchi_report(model.name(), &result));
    Ok(match result.status {
        BuchiStatus::Satisfied => ExitCode::SUCCESS,
        BuchiStatus::Violated => ExitCode::from(9),
    })
}

fn parse_limits(args: &[String]) -> Result<ExplorationLimits, String> {
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

        match flag {
            "--max-states" => set_limit(&mut limits.max_states, parsed, flag)?,
            "--max-transitions" => set_limit(&mut limits.max_transitions, parsed, flag)?,
            "--max-depth" => set_limit(&mut limits.max_depth, parsed, flag)?,
            _ => return Err(format!("unknown option '{flag}'\n{}", usage())),
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
    "usage: fvlab [list | run <counter|mutex-bug|traffic-light|peterson|peterson-bug|commuting-counters> [--max-states N] [--max-transitions N] [--max-depth N] | reduce commuting-counters | reach <counter-three|counter-four> | deadlock <counter-terminal-ok|counter-terminal-forbidden> | scc <counter|traffic-light> | eventually <counter-three|counter-four|traffic-never> | respond <request-grant|request-grant-unfair|dual-grant|dual-grant-unfair-b> | monitor <session-ok|session-double-open|session-stuck> | buchi <pulses|pulses-unfair|finite-ignore|finite-strict>]"
        .to_owned()
}

fn print_examples() {
    println!("counter\nmutex-bug\ntraffic-light\npeterson\npeterson-bug\ncommuting-counters");
}

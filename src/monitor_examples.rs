use crate::builder::TransitionSystemBuilder;
use crate::model::{ModelError, Transition, TransitionSystem};
use crate::monitor::{FiniteMonitor, MonitorError, ProgressCondition, RejectCondition};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    Closed,
    Open,
    Committed,
}

pub fn session_protocol() -> Result<TransitionSystem<SessionState>, ModelError> {
    TransitionSystemBuilder::new("session-protocol", |state: &SessionState| match state {
        SessionState::Closed => Ok(vec![Transition::new("open", SessionState::Open)]),
        SessionState::Open => Ok(vec![Transition::new("commit", SessionState::Committed)]),
        SessionState::Committed => Ok(vec![Transition::new("close", SessionState::Closed)]),
    })
    .state_variable("session", "session lifecycle state")
    .initial_state(SessionState::Closed)
    .safety_invariant("recognized-session-state", |_state: &SessionState| true)
    .build()
}

pub fn invalid_double_open_protocol() -> Result<TransitionSystem<SessionState>, ModelError> {
    TransitionSystemBuilder::new("session-double-open", |state: &SessionState| match state {
        SessionState::Closed => Ok(vec![Transition::new("open", SessionState::Open)]),
        SessionState::Open => Ok(vec![Transition::new("open", SessionState::Committed)]),
        SessionState::Committed => Ok(Vec::new()),
    })
    .state_variable("session", "deliberately invalid session lifecycle")
    .initial_state(SessionState::Closed)
    .safety_invariant("recognized-session-state", |_state: &SessionState| true)
    .build()
}

pub fn stuck_committed_protocol() -> Result<TransitionSystem<SessionState>, ModelError> {
    TransitionSystemBuilder::new(
        "session-stuck-committed",
        |state: &SessionState| match state {
            SessionState::Closed => Ok(vec![Transition::new("open", SessionState::Open)]),
            SessionState::Open => Ok(vec![Transition::new("commit", SessionState::Committed)]),
            SessionState::Committed => Ok(vec![Transition::new("tick", SessionState::Committed)]),
        },
    )
    .state_variable("session", "session lifecycle with a stuck committed state")
    .initial_state(SessionState::Closed)
    .safety_invariant("recognized-session-state", |_state: &SessionState| true)
    .build()
}

/// The committed session may stutter forever even though `close` remains
/// continuously enabled. Historical monitor semantics therefore report a
/// progress-cycle violation, while weak fairness on `close` excludes only that
/// unfair infinite execution.
pub fn unfair_close_enabled_protocol() -> Result<TransitionSystem<SessionState>, ModelError> {
    TransitionSystemBuilder::new(
        "session-unfair-close-enabled",
        |state: &SessionState| match state {
            SessionState::Closed => Ok(vec![Transition::new("open", SessionState::Open)]),
            SessionState::Open => Ok(vec![Transition::new("commit", SessionState::Committed)]),
            SessionState::Committed => Ok(vec![
                Transition::new("tick", SessionState::Committed),
                Transition::new("close", SessionState::Closed),
            ]),
        },
    )
    .state_variable(
        "session",
        "session lifecycle where close can be postponed forever",
    )
    .initial_state(SessionState::Closed)
    .safety_invariant("recognized-session-state", |_state: &SessionState| true)
    .build()
}

/// A finite maximal execution stops immediately after opening. The monitor's
/// progress condition is still active at the true terminal, so weak fairness
/// must not excuse this violation.
pub fn open_terminal_protocol() -> Result<TransitionSystem<SessionState>, ModelError> {
    TransitionSystemBuilder::new("session-open-terminal", |state: &SessionState| match state {
        SessionState::Closed => Ok(vec![Transition::new("open", SessionState::Open)]),
        SessionState::Open | SessionState::Committed => Ok(Vec::new()),
    })
    .state_variable("session", "session lifecycle with an open terminal")
    .initial_state(SessionState::Closed)
    .safety_invariant("recognized-session-state", |_state: &SessionState| true)
    .build()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionMonitorState {
    Idle,
    Open,
    Committed,
    Rejected,
}

pub fn session_monitor() -> Result<FiniteMonitor<SessionMonitorState>, MonitorError> {
    FiniteMonitor::new(
        "ordered-session-lifecycle",
        SessionMonitorState::Idle,
        |state, action| step_session_monitor(*state, action),
        vec![RejectCondition::new("legal-action-order", |state| {
            *state == SessionMonitorState::Rejected
        })?],
        vec![ProgressCondition::new(
            "opened-session-eventually-closes",
            |state| {
                matches!(
                    state,
                    SessionMonitorState::Open | SessionMonitorState::Committed
                )
            },
        )?],
    )
}

fn step_session_monitor(state: SessionMonitorState, action: &str) -> SessionMonitorState {
    use SessionMonitorState::{Committed, Idle, Open, Rejected};

    if state == Rejected {
        return Rejected;
    }

    match (state, action) {
        (Idle, "open") => Open,
        (Idle, "tick") => Idle,
        (Open, "commit") => Committed,
        (Open, "tick") => Open,
        (Committed, "close") => Idle,
        (Committed, "tick") => Committed,
        _ => Rejected,
    }
}

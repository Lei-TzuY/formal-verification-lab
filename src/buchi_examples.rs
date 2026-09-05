use crate::buchi::{AcceptanceSet, BuchiAutomaton, BuchiError, FiniteRunPolicy};
use crate::builder::TransitionSystemBuilder;
use crate::model::{ModelError, Transition, TransitionSystem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PulseModelState {
    First,
    Second,
    Done,
}

pub fn alternating_pulses() -> Result<TransitionSystem<PulseModelState>, ModelError> {
    TransitionSystemBuilder::new("alternating-pulses", |state: &PulseModelState| match state {
        PulseModelState::First => Ok(vec![Transition::new("pulse-a", PulseModelState::Second)]),
        PulseModelState::Second => Ok(vec![Transition::new("pulse-b", PulseModelState::First)]),
        PulseModelState::Done => Ok(Vec::new()),
    })
    .state_variable("phase", "alternating A/B pulse phase")
    .initial_state(PulseModelState::First)
    .safety_invariant("recognized-phase", |_state: &PulseModelState| true)
    .build()
}

pub fn unfair_second_pulse() -> Result<TransitionSystem<PulseModelState>, ModelError> {
    TransitionSystemBuilder::new("unfair-second-pulse", |state: &PulseModelState| match state {
        PulseModelState::First => Ok(vec![Transition::new("pulse-a", PulseModelState::Second)]),
        PulseModelState::Second => Ok(vec![
            Transition::new("pulse-a", PulseModelState::Second),
            Transition::new("pulse-b", PulseModelState::First),
        ]),
        PulseModelState::Done => Ok(Vec::new()),
    })
    .state_variable("phase", "pulse phase with an optional forever-A execution")
    .initial_state(PulseModelState::First)
    .safety_invariant("recognized-phase", |_state: &PulseModelState| true)
    .build()
}

pub fn finite_quiet_run() -> Result<TransitionSystem<PulseModelState>, ModelError> {
    TransitionSystemBuilder::new("finite-quiet-run", |state: &PulseModelState| match state {
        PulseModelState::First => Ok(vec![Transition::new("quiet", PulseModelState::Done)]),
        PulseModelState::Second | PulseModelState::Done => Ok(Vec::new()),
    })
    .state_variable("phase", "finite pulse example")
    .initial_state(PulseModelState::First)
    .safety_invariant("recognized-phase", |_state: &PulseModelState| true)
    .build()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PulseAutomatonState {
    None,
    A,
    B,
    Both,
}

pub fn pulse_automaton(
    finite_policy: FiniteRunPolicy,
) -> Result<BuchiAutomaton<PulseAutomatonState>, BuchiError> {
    BuchiAutomaton::new(
        "infinitely-often-a-and-b",
        PulseAutomatonState::None,
        |_state, action| match action {
            "pulse-a" => PulseAutomatonState::A,
            "pulse-b" => PulseAutomatonState::B,
            "pulse-both" => PulseAutomatonState::Both,
            _ => PulseAutomatonState::None,
        },
        vec![
            AcceptanceSet::new("pulse-a-observed", |state| {
                matches!(state, PulseAutomatonState::A | PulseAutomatonState::Both)
            })?,
            AcceptanceSet::new("pulse-b-observed", |state| {
                matches!(state, PulseAutomatonState::B | PulseAutomatonState::Both)
            })?,
        ],
        finite_policy,
    )
}

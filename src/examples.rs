use crate::builder::TransitionSystemBuilder;
use crate::model::{Invariant, ModelError, StateVariable, Transition, TransitionSystem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CounterState {
    pub value: u8,
}

pub fn bounded_counter() -> Result<TransitionSystem<CounterState>, ModelError> {
    TransitionSystem::new(
        "bounded-counter",
        vec![StateVariable::new(
            "value",
            "counter value in the range 0..=3",
        )],
        vec![CounterState { value: 0 }],
        |state| {
            if state.value < 3 {
                Ok(vec![Transition::new(
                    "increment",
                    CounterState {
                        value: state.value + 1,
                    },
                )])
            } else {
                Ok(Vec::new())
            }
        },
        vec![Invariant::new("within-bound", |state: &CounterState| {
            state.value <= 3
        })],
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Idle,
    Trying,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MutexState {
    pub p1: Phase,
    pub p2: Phase,
}

/// Deliberately incorrect protocol: each process can enter its critical
/// section after "trying" without checking whether the other process is in
/// its critical section.
pub fn buggy_mutex() -> Result<TransitionSystem<MutexState>, ModelError> {
    TransitionSystem::new(
        "buggy-mutex",
        vec![
            StateVariable::new("p1", "phase of process 1"),
            StateVariable::new("p2", "phase of process 2"),
        ],
        vec![MutexState {
            p1: Phase::Idle,
            p2: Phase::Idle,
        }],
        |state| {
            let mut next = Vec::new();

            match state.p1 {
                Phase::Idle => next.push(Transition::new(
                    "p1:request",
                    MutexState {
                        p1: Phase::Trying,
                        ..*state
                    },
                )),
                Phase::Trying => next.push(Transition::new(
                    "p1:enter",
                    MutexState {
                        p1: Phase::Critical,
                        ..*state
                    },
                )),
                Phase::Critical => next.push(Transition::new(
                    "p1:exit",
                    MutexState {
                        p1: Phase::Idle,
                        ..*state
                    },
                )),
            }

            match state.p2 {
                Phase::Idle => next.push(Transition::new(
                    "p2:request",
                    MutexState {
                        p2: Phase::Trying,
                        ..*state
                    },
                )),
                Phase::Trying => next.push(Transition::new(
                    "p2:enter",
                    MutexState {
                        p2: Phase::Critical,
                        ..*state
                    },
                )),
                Phase::Critical => next.push(Transition::new(
                    "p2:exit",
                    MutexState {
                        p2: Phase::Idle,
                        ..*state
                    },
                )),
            }

            Ok(next)
        },
        vec![Invariant::new("mutual-exclusion", |state: &MutexState| {
            !(state.p1 == Phase::Critical && state.p2 == Phase::Critical)
        })],
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Light {
    Red,
    Green,
    Yellow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrafficLightState {
    pub light: Light,
}

pub fn traffic_light() -> Result<TransitionSystem<TrafficLightState>, ModelError> {
    TransitionSystem::new(
        "traffic-light",
        vec![StateVariable::new("light", "current traffic-light phase")],
        vec![TrafficLightState { light: Light::Red }],
        |state| {
            let light = match state.light {
                Light::Red => Light::Green,
                Light::Green => Light::Yellow,
                Light::Yellow => Light::Red,
            };
            Ok(vec![Transition::new(
                "advance",
                TrafficLightState { light },
            )])
        },
        vec![Invariant::new(
            "recognized-phase",
            |_state: &TrafficLightState| true,
        )],
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessId {
    P0,
    P1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetersonPc {
    Idle,
    SetTurn,
    Wait,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PetersonState {
    pub p0: PetersonPc,
    pub p1: PetersonPc,
    pub flag0: bool,
    pub flag1: bool,
    pub turn: ProcessId,
}

/// Peterson's two-process mutual-exclusion protocol, modeled as atomic program
/// counter steps. Waiting is represented by the absence of an enabled `enter`
/// transition; explicit stuttering is unnecessary for safety reachability.
pub fn peterson_mutex() -> Result<TransitionSystem<PetersonState>, ModelError> {
    peterson_model("peterson-mutex", true)
}

/// Controlled faulty Peterson variant: the request step accidentally clears
/// rather than sets each process's intent flag. With both flags remaining
/// false, each process can pass the wait guard and mutual exclusion fails.
pub fn buggy_peterson_mutex() -> Result<TransitionSystem<PetersonState>, ModelError> {
    peterson_model("peterson-lost-intent", false)
}

fn peterson_model(
    name: &'static str,
    request_sets_intent: bool,
) -> Result<TransitionSystem<PetersonState>, ModelError> {
    TransitionSystemBuilder::new(name, move |state: &PetersonState| {
        let mut next = Vec::new();

        match state.p0 {
            PetersonPc::Idle => next.push(Transition::new(
                "p0:set-flag",
                PetersonState {
                    p0: PetersonPc::SetTurn,
                    flag0: request_sets_intent,
                    ..*state
                },
            )),
            PetersonPc::SetTurn => next.push(Transition::new(
                "p0:set-turn",
                PetersonState {
                    p0: PetersonPc::Wait,
                    turn: ProcessId::P1,
                    ..*state
                },
            )),
            PetersonPc::Wait => {
                if !state.flag1 || state.turn != ProcessId::P1 {
                    next.push(Transition::new(
                        "p0:enter",
                        PetersonState {
                            p0: PetersonPc::Critical,
                            ..*state
                        },
                    ));
                }
            }
            PetersonPc::Critical => next.push(Transition::new(
                "p0:exit",
                PetersonState {
                    p0: PetersonPc::Idle,
                    flag0: false,
                    ..*state
                },
            )),
        }

        match state.p1 {
            PetersonPc::Idle => next.push(Transition::new(
                "p1:set-flag",
                PetersonState {
                    p1: PetersonPc::SetTurn,
                    flag1: request_sets_intent,
                    ..*state
                },
            )),
            PetersonPc::SetTurn => next.push(Transition::new(
                "p1:set-turn",
                PetersonState {
                    p1: PetersonPc::Wait,
                    turn: ProcessId::P0,
                    ..*state
                },
            )),
            PetersonPc::Wait => {
                if !state.flag0 || state.turn != ProcessId::P0 {
                    next.push(Transition::new(
                        "p1:enter",
                        PetersonState {
                            p1: PetersonPc::Critical,
                            ..*state
                        },
                    ));
                }
            }
            PetersonPc::Critical => next.push(Transition::new(
                "p1:exit",
                PetersonState {
                    p1: PetersonPc::Idle,
                    flag1: false,
                    ..*state
                },
            )),
        }

        Ok(next)
    })
    .state_variable("p0.pc", "program counter of process 0")
    .state_variable("p1.pc", "program counter of process 1")
    .state_variable("flag0", "process 0 intent flag")
    .state_variable("flag1", "process 1 intent flag")
    .state_variable("turn", "process favored by the Peterson tie breaker")
    .initial_state(PetersonState {
        p0: PetersonPc::Idle,
        p1: PetersonPc::Idle,
        flag0: false,
        flag1: false,
        turn: ProcessId::P0,
    })
    .safety_invariant("mutual-exclusion", |state: &PetersonState| {
        !(state.p0 == PetersonPc::Critical && state.p1 == PetersonPc::Critical)
    })
    .build()
}

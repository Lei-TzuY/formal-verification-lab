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

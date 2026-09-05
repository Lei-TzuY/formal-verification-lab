use crate::builder::TransitionSystemBuilder;
use crate::model::{ModelError, Transition, TransitionSystem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DualResponsePhase {
    Idle,
    AwaitA,
    ReadyB,
    AwaitB,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DualResponseState {
    pub phase: DualResponsePhase,
}

/// Deterministic protocol that repeatedly discharges class A and class B.
pub fn dual_response_protocol() -> Result<TransitionSystem<DualResponseState>, ModelError> {
    dual_response_model("dual-response", false)
}

/// Class B may stutter forever while its response is enabled. Without a
/// fairness assumption, the class-B response obligation is violated.
pub fn unfair_dual_response_protocol() -> Result<TransitionSystem<DualResponseState>, ModelError> {
    dual_response_model("dual-response-unfair-b", true)
}

fn dual_response_model(
    name: &'static str,
    allow_b_stutter: bool,
) -> Result<TransitionSystem<DualResponseState>, ModelError> {
    TransitionSystemBuilder::new(name, move |state: &DualResponseState| match state.phase {
        DualResponsePhase::Idle => Ok(vec![Transition::new(
            "request-a",
            DualResponseState {
                phase: DualResponsePhase::AwaitA,
            },
        )]),
        DualResponsePhase::AwaitA => Ok(vec![Transition::new(
            "grant-a",
            DualResponseState {
                phase: DualResponsePhase::ReadyB,
            },
        )]),
        DualResponsePhase::ReadyB => Ok(vec![Transition::new(
            "request-b",
            DualResponseState {
                phase: DualResponsePhase::AwaitB,
            },
        )]),
        DualResponsePhase::AwaitB => {
            let mut next = Vec::new();
            if allow_b_stutter {
                next.push(Transition::new(
                    "wait-b",
                    DualResponseState {
                        phase: DualResponsePhase::AwaitB,
                    },
                ));
            }
            next.push(Transition::new(
                "grant-b",
                DualResponseState {
                    phase: DualResponsePhase::Idle,
                },
            ));
            Ok(next)
        }
    })
    .state_variable("phase", "dual response protocol phase")
    .initial_state(DualResponseState {
        phase: DualResponsePhase::Idle,
    })
    .safety_invariant("recognized-phase", |_state: &DualResponseState| true)
    .build()
}

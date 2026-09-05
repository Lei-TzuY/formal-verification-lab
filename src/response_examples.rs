use crate::builder::TransitionSystemBuilder;
use crate::model::{ModelError, Transition, TransitionSystem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestPhase {
    Idle,
    Waiting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestGrantState {
    pub phase: RequestPhase,
}

/// Every request is followed by a grant on the only enabled continuation.
pub fn request_grant_protocol() -> Result<TransitionSystem<RequestGrantState>, ModelError> {
    request_grant_model("request-grant", false)
}

/// The waiting state can stutter forever even though `grant` is also enabled.
/// Without a fairness assumption, that infinite execution violates response.
pub fn unfair_request_grant_protocol() -> Result<TransitionSystem<RequestGrantState>, ModelError> {
    request_grant_model("request-grant-unfair", true)
}

fn request_grant_model(
    name: &'static str,
    allow_wait_stutter: bool,
) -> Result<TransitionSystem<RequestGrantState>, ModelError> {
    TransitionSystemBuilder::new(name, move |state: &RequestGrantState| match state.phase {
        RequestPhase::Idle => Ok(vec![Transition::new(
            "request",
            RequestGrantState {
                phase: RequestPhase::Waiting,
            },
        )]),
        RequestPhase::Waiting => {
            let mut next = Vec::new();
            if allow_wait_stutter {
                next.push(Transition::new(
                    "wait",
                    RequestGrantState {
                        phase: RequestPhase::Waiting,
                    },
                ));
            }
            next.push(Transition::new(
                "grant",
                RequestGrantState {
                    phase: RequestPhase::Idle,
                },
            ));
            Ok(next)
        }
    })
    .state_variable("phase", "request/grant protocol phase")
    .initial_state(RequestGrantState {
        phase: RequestPhase::Idle,
    })
    .safety_invariant("recognized-phase", |_state: &RequestGrantState| true)
    .build()
}

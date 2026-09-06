use crate::checker::InconclusiveReason;

pub(crate) fn format_inconclusive_reason(reason: InconclusiveReason) -> String {
    match reason {
        InconclusiveReason::StateLimitReached { limit } => {
            format!("state limit reached (max {limit})")
        }
        InconclusiveReason::TransitionLimitReached { limit } => {
            format!("transition limit reached (max {limit})")
        }
        InconclusiveReason::DepthLimitReached { limit } => {
            format!("depth limit reached (max {limit})")
        }
    }
}

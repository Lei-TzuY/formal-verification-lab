use crate::checker::{ExplorationLimits, InconclusiveReason};

/// A property result under deterministic exploration limits.
///
/// A conclusive value is returned only when the property-specific proof or
/// counterexample is complete. A resource cutoff is represented explicitly and
/// must never be interpreted as proof of absence, safety, or satisfaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundedOutcome<T> {
    Conclusive(T),
    Inconclusive(InconclusiveReason),
}

impl<T> BoundedOutcome<T> {
    pub fn as_ref(&self) -> BoundedOutcome<&T> {
        match self {
            Self::Conclusive(value) => BoundedOutcome::Conclusive(value),
            Self::Inconclusive(reason) => BoundedOutcome::Inconclusive(*reason),
        }
    }

    pub fn inconclusive_reason(&self) -> Option<InconclusiveReason> {
        match self {
            Self::Conclusive(_) => None,
            Self::Inconclusive(reason) => Some(*reason),
        }
    }
}

/// The deterministic stage whose resource bound prevents a whole-analysis
/// proof from completing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStage {
    Model,
    Product,
}

/// Stage-qualified resource cutoff for composed model/product verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisInconclusiveReason {
    pub stage: AnalysisStage,
    pub reason: InconclusiveReason,
}

/// Whole-analysis outcome for staged model capture followed by product
/// construction. A conclusive counterexample may be returned from a justified
/// prefix, but a positive proof requires every required stage to complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisOutcome<T> {
    Conclusive(T),
    Inconclusive(AnalysisInconclusiveReason),
}

impl<T> AnalysisOutcome<T> {
    pub fn inconclusive_reason(&self) -> Option<AnalysisInconclusiveReason> {
        match self {
            Self::Conclusive(_) => None,
            Self::Inconclusive(reason) => Some(*reason),
        }
    }
}

/// Independent deterministic budgets for the model-capture and action-product
/// stages of one analysis.
///
/// These are exploration counters, not wall-clock, total-memory, or performance
/// guarantees. Model limits are applied first; product limits then apply to the
/// justified captured prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnalysisLimits {
    pub model: ExplorationLimits,
    pub product: ExplorationLimits,
}

impl AnalysisLimits {
    pub const fn unbounded() -> Self {
        Self {
            model: ExplorationLimits::unbounded(),
            product: ExplorationLimits::unbounded(),
        }
    }

    pub const fn new(model: ExplorationLimits, product: ExplorationLimits) -> Self {
        Self { model, product }
    }
}

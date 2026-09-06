use crate::checker::InconclusiveReason;

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

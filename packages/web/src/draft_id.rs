use std::fmt;

/// Stable identifier for a draft item (step, ingredient, meal row, …).
///
/// `Persisted` ids come from the database and are deterministic across SSR and
/// hydration. `New` ids are allocated by the form itself via a per-form counter
/// — never from a process-global source — so server-rendered HTML matches what
/// the client produces on first render and hydration succeeds.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DraftId {
    Persisted(i64),
    New(i64),
}

impl Default for DraftId {
    fn default() -> Self {
        Self::New(0)
    }
}

impl From<i64> for DraftId {
    fn from(value: i64) -> Self {
        Self::Persisted(value)
    }
}

impl fmt::Display for DraftId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DraftId::Persisted(id) => write!(f, "p{id}"),
            DraftId::New(id) => write!(f, "n{id}"),
        }
    }
}

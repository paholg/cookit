use {std::fmt, uuid::Uuid};

/// Stable identifier for a draft item (step, ingredient, meal row, …).
///
/// `Persisted` ids come from the database and are deterministic across SSR and
/// hydration. `New` ids are allocated by the form itself via a per-form counter
/// — never from a process-global source — so server-rendered HTML matches what
/// the client produces on first render and hydration succeeds.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DraftId {
    Persisted(Uuid),
    New(i64),
}

impl Default for DraftId {
    fn default() -> Self {
        Self::New(0)
    }
}

impl<T> From<types::id::Id<T>> for DraftId {
    fn from(id: types::id::Id<T>) -> Self {
        Self::Persisted(*id.as_uuid())
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

//! App-wide running-timer state, persisted to `localStorage` via
//! `use_synced_storage` (set up in [`crate::app`]) so a running bake survives
//! navigation, accidental reloads, and even syncs across tabs.
//!
//! `RunningTimer.started_at_ms` is an absolute wall-clock timestamp; the
//! `remaining` calculation always subtracts `now - started`, so a reload
//! continues counting from where it really should be (or rings if expiry
//! happened while the page was closed).

use {
    crate::client::client,
    dioxus::prelude::*,
    serde::{Deserialize, Serialize},
};

// Persistence is handled by the synced-storage signal created in `App` (see
// `crate::app`); writing to that `Signal` saves to `localStorage` and syncs
// across tabs, so the helpers here just mutate the shared context.

pub const STORAGE_KEY: &str = "cookit.timers";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunningTimer {
    pub id: u64,
    /// Meal the timer was started from, if any. Lets a future click on the
    /// row jump back to the right `MealDetail` (vs. plain `RecipeDetail`).
    pub meal_key: Option<String>,
    pub recipe_key: String,
    /// Cached for display — avoids re-fetching the recipe just to render the
    /// timer bar after a reload.
    pub recipe_name: String,
    /// One-based step number, matching what the recipe view shows.
    pub step_number: i64,
    pub total_seconds: i64,
    pub started_at_ms: i64,
    /// Set once the user dismisses the post-expiry bell. The row stays visible
    /// (so they can see how far overdue they are) but no longer rings.
    pub silenced: bool,
    /// Seconds added by the `+1m` / `+5m` buttons. Kept separate from
    /// `total_seconds` so the original duration is preserved for context.
    pub added_seconds: i64,
}

impl RunningTimer {
    pub fn remaining_seconds(&self, now_ms: i64) -> i64 {
        let elapsed = (now_ms - self.started_at_ms) / 1000;
        self.total_seconds + self.added_seconds - elapsed
    }

    pub fn is_expired(&self, now_ms: i64) -> bool {
        self.remaining_seconds(now_ms) <= 0
    }
}

pub type RunningTimersCtx = Signal<Vec<RunningTimer>>;

pub fn now_ms() -> i64 {
    client().now_ms()
}

/// Allocate a fresh id that won't collide with anything already in `timers`.
pub fn next_timer_id(timers: &[RunningTimer]) -> u64 {
    timers
        .iter()
        .map(|t| t.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

/// Push a new timer onto the shared context. The write persists automatically
/// through the synced-storage signal backing [`RunningTimersCtx`].
pub fn start_timer(
    mut ctx: RunningTimersCtx,
    meal_key: Option<String>,
    recipe_key: String,
    recipe_name: String,
    step_number: i64,
    total_seconds: i64,
) {
    let mut list = ctx.write();
    let id = next_timer_id(&list);
    list.push(RunningTimer {
        id,
        meal_key,
        recipe_key,
        recipe_name,
        step_number,
        total_seconds,
        started_at_ms: now_ms(),
        silenced: false,
        added_seconds: 0,
    });
}

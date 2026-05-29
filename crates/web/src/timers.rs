//! App-wide running-timer state, persisted to `localStorage` so a running bake
//! survives navigation and accidental reloads.
//!
//! `RunningTimer.started_at_ms` is an absolute wall-clock timestamp; the
//! `remaining` calculation always subtracts `now - started`, so a reload
//! continues counting from where it really should be (or rings if expiry
//! happened while the page was closed).

use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use web_time::{SystemTime, UNIX_EPOCH};

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
    /// Set once the user dismisses the post-expiry beep. The row stays visible
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
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

pub fn load_from_storage() -> Vec<RunningTimer> {
    LocalStorage::get(STORAGE_KEY).unwrap_or_default()
}

pub fn save_to_storage(timers: &Vec<RunningTimer>) {
    let _ = LocalStorage::set(STORAGE_KEY, timers);
}

/// Push a new timer onto the shared context and persist.
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
    save_to_storage(&list);
}

/// Attach native listeners (synchronously, in the user-gesture stack) that
/// create or resume the shared `__cookitAudioCtx`. Without this, any
/// AudioContext created later — e.g. when a timer expires several minutes
/// after the most recent click — stays `suspended` under Firefox/Chrome's
/// autoplay policy and the beep produces no sound.
///
/// Listeners are registered in capture phase so they run before any Dioxus
/// handler can cancel propagation. Re-runs of the resume() call are cheap
/// and required: AudioContexts can drop back to suspended (tab backgrounded,
/// OS audio reconfigured, etc.), so every gesture re-primes.
pub const ATTACH_AUDIO_PRIMER_JS: &str = r#"
(function() {
    if (window.__cookitAudioPrimerAttached) return;
    window.__cookitAudioPrimerAttached = true;
    const prime = () => {
        try {
            if (!window.__cookitAudioCtx) {
                const AC = window.AudioContext || window.webkitAudioContext;
                if (!AC) return;
                window.__cookitAudioCtx = new AC();
            }
            if (window.__cookitAudioCtx.state === 'suspended') {
                window.__cookitAudioCtx.resume();
            }
        } catch (e) {}
    };
    ['pointerdown', 'click', 'keydown', 'touchstart'].forEach(ev => {
        document.addEventListener(ev, prime, { capture: true });
    });
})();
"#;

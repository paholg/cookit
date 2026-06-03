//! Global running-timers panel. Shows nothing when no timers are active,
//! otherwise pins a strip at the bottom of the viewport. Each timer is a
//! small pill that the user can click to reveal step info and action buttons
//! for that specific timer; siblings stay compact.

use {
    crate::{
        client::client,
        timers::{self, RunningTimersCtx},
    },
    api::duration::format_countdown,
    dioxus::prelude::*,
    std::collections::HashSet,
};

const BELL_PERIOD_MS: i64 = 30_000;

#[component]
pub fn TimerBar() -> Element {
    let mut timers_ctx = use_context::<RunningTimersCtx>();
    let mut expanded_ids = use_signal(Vec::<u64>::new);
    let mut tick = use_signal(|| 0u64);

    // 1 Hz re-render. The platform `sleep` works on wasm where `tokio::time`
    // doesn't.
    use_future(move || async move {
        loop {
            client().sleep(1000).await;
            tick.with_mut(|t| *t = t.wrapping_add(1));
        }
    });

    // Forces this body to depend on `tick` so the loop above actually causes
    // re-renders. Without a read the optimizer would skip it.
    let _ = tick();

    let now = timers::now_ms();
    let snapshot = timers_ctx.read().clone();

    // All reads go through `peek`, so this future is never torn down and
    // restarted by signal changes; it's spawned once and loops for the
    // component's lifetime, which lets the bell bookkeeping live as plain
    // task-local state rather than signals.
    use_future(move || async move {
        let mut ringing_ids = HashSet::<u64>::new();
        let mut last_bell_ms: Option<i64> = None;

        loop {
            let now = timers::now_ms();
            let current: HashSet<u64> = timers_ctx
                .peek()
                .iter()
                .filter(|t| !t.silenced && t.is_expired(now))
                .map(|t| t.id)
                .collect();

            if current.is_empty() {
                last_bell_ms = None;
            } else {
                let new_expiry = current.iter().any(|id| !ringing_ids.contains(id));
                let nag_due = last_bell_ms.is_none_or(|t| now - t >= BELL_PERIOD_MS);
                if new_expiry || nag_due {
                    client().play_bell();
                    last_bell_ms = Some(now);
                }
            }

            ringing_ids = current;
            client().sleep(1000).await;
        }
    });

    if snapshot.is_empty() {
        return rsx! {};
    }

    rsx! {
        aside { class: "timer-bar", "aria-label": "Running timers",
            ul { class: "timer-list",
                for t in snapshot.iter() {
                    {
                        let remaining = t.remaining_seconds(now);
                        let expired = remaining <= 0;
                        let ringing = expired && !t.silenced;
                        let id = t.id;
                        let step_number = t.step_number;
                        let is_open = expanded_ids.read().contains(&id);
                        let mut class = String::from("timer-row");
                        if expired {
                            class.push_str(" expired");
                        }
                        if ringing {
                            class.push_str(" ringing");
                        }
                        if is_open {
                            class.push_str(" open");
                        }
                        rsx! {
                            li {
                                key: "{t.id}",
                                class: "{class}",
                                onclick: move |_| toggle_expanded(&mut expanded_ids, id),
                                div { class: "timer-row-main",
                                    span { class: "timer-name", "{t.recipe_name}" }
                                    span { class: "timer-remaining", "{format_countdown(remaining)}" }
                                }
                                if is_open {
                                    // Popover floats above the bar rather than
                                    // stretching the row taller, so siblings
                                    // stay put when a single timer is opened.
                                    div {
                                    class: "timer-popover",
                                    onclick: move |e| e.stop_propagation(),
                                        div { class: "timer-popover-step", "{t.recipe_name} - step {step_number}" }
                                        div { class: "timer-actions",
                                            if ringing {
                                                button {
                                                    r#type: "button",
                                                    class: "timer-action silence",
                                                    onclick: move |_| silence(&mut timers_ctx, id),
                                                    "Silence"
                                                }
                                            }
                                            button {
                                                r#type: "button",
                                                class: "timer-action",
                                                onclick: move |_| add_seconds(&mut timers_ctx, id, 60),
                                                "+1m"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "timer-action",
                                                onclick: move |_| add_seconds(&mut timers_ctx, id, 300),
                                                "+5m"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "timer-action",
                                                onclick: move |_| add_seconds(&mut timers_ctx, id, 600),
                                                "+10m"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "timer-action dismiss",
                                                onclick: move |_| {
                                                    dismiss(&mut timers_ctx, id);
                                                    // Don't leave a stale id in the expanded set.
                                                    expanded_ids.write().retain(|x| *x != id);
                                                },
                                                "Dismiss"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn toggle_expanded(ids: &mut Signal<Vec<u64>>, id: u64) {
    let mut list = ids.write();
    if let Some(pos) = list.iter().position(|x| *x == id) {
        list.remove(pos);
    } else {
        list.push(id);
    }
}

fn silence(ctx: &mut RunningTimersCtx, id: u64) {
    let mut list = ctx.write();
    if let Some(t) = list.iter_mut().find(|t| t.id == id) {
        t.silenced = true;
    }
    timers::save_to_storage(&list);
}

fn add_seconds(ctx: &mut RunningTimersCtx, id: u64, seconds: i64) {
    let mut list = ctx.write();
    if let Some(t) = list.iter_mut().find(|t| t.id == id) {
        t.added_seconds += seconds;
        // Adding time un-silences so the user gets re-notified if it expires.
        t.silenced = false;
    }
    timers::save_to_storage(&list);
}

fn dismiss(ctx: &mut RunningTimersCtx, id: u64) {
    let mut list = ctx.write();
    list.retain(|t| t.id != id);
    timers::save_to_storage(&list);
}

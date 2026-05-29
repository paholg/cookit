//! Global running-timers panel. Shows nothing when no timers are active,
//! otherwise pins a strip at the bottom of the viewport. Each timer is a
//! small pill that the user can click to reveal step info and action buttons
//! for that specific timer; siblings stay compact.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use super::duration::format_countdown;
use crate::timers::{self, RunningTimersCtx};

/// Beep keeps running until every expired timer is silenced (or removed).
/// Reuses the shared `__cookitAudioCtx` that the Start-timer click primed
/// (see `timers::ENSURE_AUDIO_JS`) — creating a fresh AudioContext here
/// would be subject to autoplay policy and produce silence in Firefox.
const BEEP_ON_JS: &str = r#"
try {
    const ctx = window.__cookitAudioCtx;
    if (ctx && !window.__cookitBeep) {
        if (ctx.state === 'suspended') { ctx.resume(); }
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.frequency.value = 400;
        gain.gain.value = 0.0;
        osc.connect(gain).connect(ctx.destination);
        osc.start();
        const tick = () => {
            const t = ctx.currentTime;
            gain.gain.cancelScheduledValues(t);
            gain.gain.setValueAtTime(0.25, t + 0.01);
            gain.gain.setValueAtTime(0.0, t + 0.26);
            window.__cookitBeepTimeout = setTimeout(tick, 500);
        };
        tick();
        window.__cookitBeep = { osc, gain };
    }
} catch (e) {}
"#;

const BEEP_OFF_JS: &str = r#"
if (window.__cookitBeepTimeout) {
    clearTimeout(window.__cookitBeepTimeout);
    window.__cookitBeepTimeout = null;
}
if (window.__cookitBeep) {
    try { window.__cookitBeep.osc.stop(); } catch (e) {}
    try { window.__cookitBeep.gain.disconnect(); } catch (e) {}
    window.__cookitBeep = null;
}
"#;

#[component]
pub fn TimerBar() -> Element {
    let mut timers_ctx = use_context::<RunningTimersCtx>();
    let mut expanded_ids = use_signal(Vec::<u64>::new);
    let mut tick = use_signal(|| 0u64);

    // 1 Hz re-render. gloo-timers works on wasm where tokio::time doesn't.
    use_future(move || async move {
        loop {
            TimeoutFuture::new(1000).await;
            tick.with_mut(|t| *t = t.wrapping_add(1));
        }
    });

    // Forces this body to depend on `tick` so the loop above actually causes
    // re-renders. Without a read the optimizer would skip it.
    let _ = tick();

    let now = timers::now_ms();
    let snapshot = timers_ctx.read().clone();

    // Drive the beep from an effect so it runs client-side only (never during
    // SSR). Crucially, the effect must *read tracked signals that change* so
    // Dioxus re-runs it — capturing a plain `bool` would only register a new
    // closure on each render but never re-execute. We therefore recompute
    // `ringing_now` inside the effect from `tick` + `timers_ctx`, both of
    // which are signal reads.
    let mut was_ringing = use_signal(|| false);
    use_effect(move || {
        // Tracked reads: tick (so we re-evaluate on every 1Hz bump) and
        // timers_ctx (so silence/dismiss/add-time take effect immediately).
        let _ = tick();
        let now = timers::now_ms();
        let ringing_now = timers_ctx
            .read()
            .iter()
            .any(|t| !t.silenced && t.is_expired(now));

        if ringing_now != was_ringing() {
            document::eval(if ringing_now { BEEP_ON_JS } else { BEEP_OFF_JS });
            was_ringing.set(ringing_now);
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

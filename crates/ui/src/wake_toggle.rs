use {
    crate::icons::LightbulbIcon,
    dioxus::{core::Task, prelude::*},
};

/// A button that holds a screen wake lock while toggled on.
///
/// The lock is acquired from the registered [`Client`] and lives inside a
/// spawned task for as long as it's held. Cancelling that task drops the
/// guard, which releases the lock — that's how both toggle-off and unmount
/// release it. If the platform drops the lock on its own (e.g. the tab is
/// hidden), [`WakeLock::lost`] wakes us so the button reflects it.
///
/// [`Client`]: crate::Client
/// [`WakeLock::lost`]: crate::WakeLock::lost
#[component]
pub fn WakeLockToggle() -> Element {
    let mut on = use_signal(|| false);

    // The task that owns the wake-lock guard while we're on; cancelling it
    // drops the guard and releases the lock.
    let mut holder = use_signal(|| None::<Task>);

    use_drop(move || {
        if let Some(task) = holder.write().take() {
            task.cancel();
        }
    });

    let label = if on() {
        "Disable wake lock"
    } else {
        "Enable wake lock"
    };

    rsx! {
        button {
            r#type: "button",
            class: if on() { "icon-button wake-toggle on" } else { "icon-button wake-toggle" },
            "aria-label": label,
            "aria-pressed": if on() { "true" } else { "false" },
            title: label,
            onclick: move |_| {
                if on() {
                    on.set(false);

                    if let Some(task) = holder.write().take() {
                        task.cancel();
                    }
                } else {
                    on.set(true);

                    let task = spawn(async move {
                        match crate::client::client().acquire_wake_lock().await {
                            // Hold the guard until the platform drops the lock,
                            // then reflect that in the UI. The guard releases
                            // the lock when this task ends or is cancelled.
                            Some(lock) => {
                                lock.lost().await;
                                on.set(false);
                            }
                            // Couldn't acquire — don't leave the toggle stuck on.
                            None => on.set(false),
                        }
                    });

                    holder.set(Some(task));
                }
            },
            LightbulbIcon {}
        }
    }
}

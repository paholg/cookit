use {dioxus::prelude::*, ui::icons::LightbulbIcon};

/// Acquires a screen wake lock and resolves with `"released"` once the lock
/// is released — either because we asked for it via [`RELEASE_JS`], or because
/// the browser dropped it (tab hidden, navigation, OS power policy, etc.).
///
/// `__cookitWakeWanted` covers the race where the user toggles off before the
/// async `request()` resolves.
const ACQUIRE_JS: &str = r#"
window.__cookitWakeWanted = true;
try {
    if (window.__cookitWakeLock) {
        try { await window.__cookitWakeLock.release(); } catch (e) {}
        window.__cookitWakeLock = null;
    }
    const lock = await navigator.wakeLock.request('screen');
    if (!window.__cookitWakeWanted) {
        try { await lock.release(); } catch (e) {}
        return "released";
    }
    window.__cookitWakeLock = lock;
    await new Promise(resolve => lock.addEventListener('release', resolve));
    if (window.__cookitWakeLock === lock) window.__cookitWakeLock = null;
    return "released";
} catch (e) {
    return "released";
}
"#;

const RELEASE_JS: &str = r#"
window.__cookitWakeWanted = false;
const lock = window.__cookitWakeLock;
if (lock) {
    try { await lock.release(); } catch (e) {}
}
"#;

#[component]
pub fn WakeLockToggle() -> Element {
    let mut on = use_signal(|| false);

    use_drop(move || {
        document::eval(RELEASE_JS);
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
                let now = !on();
                on.set(now);
                if now {
                    spawn(async move {
                        let result = document::eval(ACQUIRE_JS).join::<String>().await;
                        if matches!(result.as_deref(), Ok("released") | Err(_)) {
                            on.set(false);
                        }
                    });
                } else {
                    document::eval(RELEASE_JS);
                }
            },
            LightbulbIcon {}
        }
    }
}

use {db::Timestamp, dioxus::prelude::*, jiff::tz::TimeZone};

/// The user's time zone.
///
/// Start as UTC to agree with the server's initial render.
static TIMEZONE: GlobalSignal<TimeZone> = Signal::global(|| TimeZone::UTC);

pub fn initialize_timezone() {
    use_effect(|| *TIMEZONE.write() = crate::client().timezone());
}

/// Default `strftime` pattern, e.g. `Jun 28, 2026, 3:42 PM`.
const DEFAULT_FORMAT: &str = "%b %-d, %Y, %-I:%M %p %Z";

/// Render a [`Timestamp`] in the user's time zone.
#[component]
pub fn FormatTimestamp(timestamp: Timestamp, format: Option<String>) -> Element {
    let zoned = timestamp.to_zoned(TIMEZONE());
    let pattern = format.as_deref().unwrap_or(DEFAULT_FORMAT);
    let text = zoned.strftime(pattern).to_string();

    rsx! {
        time { datetime: "{timestamp}", "{text}" }
    }
}

use dioxus::prelude::*;

/// Render `children` only after hydration completes.
///
/// Dioxus 0.7 does not synchronize the `value` attribute on `<textarea>` or
/// `<select>` between SSR output and post-hydration DOM state (the `value`
/// attribute is a JS property for those elements, and the SSR renderer emits
/// it as a literal HTML attribute that the browser ignores). Wrapping such
/// elements in `ClientOnly` skips the SSR pass entirely so the first real
/// render happens on the client, where `value:` works correctly.
///
/// See https://github.com/DioxusLabs/dioxus/issues/1841
#[component]
pub fn ClientOnly(children: Element) -> Element {
    let mut hydrated = use_signal(|| false);

    use_effect(move || hydrated.set(true));

    if hydrated() {
        rsx! { {children} }
    } else {
        rsx! {}
    }
}

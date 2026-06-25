//! Form validation: a [`Field`] holds raw text and the [`Memo`] parsed from it;
//! [`Validated`] wraps an input with an inline error/hint. Fields report to a
//! [`FormValidity`] context so the submit button can gate on every field parsing.

use {
    dioxus::prelude::*,
    std::{collections::HashMap, fmt::Display, str::FromStr},
};

/// Validity of every [`Field`] under a form, shared via context. Provide it with
/// [`use_form_validity`]; gate the submit button on [`FormValidity::all_valid`].
#[derive(Clone, Copy)]
pub struct FormValidity {
    fields: Signal<HashMap<usize, bool>>,
    next_id: Signal<usize>,
}

impl FormValidity {
    /// Register a field (initially invalid, so an empty form can't submit).
    fn register(&mut self) -> usize {
        let id = (self.next_id)();
        *self.next_id.write() += 1;
        self.fields.write().insert(id, false);

        id
    }

    fn set(&mut self, id: usize, valid: bool) {
        self.fields.write().insert(id, valid);
    }

    fn remove(&mut self, id: usize) {
        self.fields.write().remove(&id);
    }

    /// True when every registered field parses.
    pub fn all_valid(&self) -> bool {
        self.fields.read().values().all(|&valid| valid)
    }
}

/// Provide a [`FormValidity`] to descendants. Call once in the form component,
/// before any [`use_field`].
pub fn use_form_validity() -> FormValidity {
    let fields = use_signal(HashMap::new);
    let next_id = use_signal(|| 0);

    use_context_provider(|| FormValidity { fields, next_id })
}

/// One input's state: raw text, the value parsed from it, and whether it's been
/// touched. Create with [`use_field`]; read the typed value via [`Field::value`].
pub struct Field<T: 'static> {
    id: usize,
    raw: Signal<String>,
    parsed: Memo<Result<T, String>>,
    touched: Signal<bool>,
}

impl<T: 'static> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Field<T> {}

// Compare by identity. Only here to satisfy the prop bound on `Validated`;
// equality is moot since its `render` callback already prevents memoization.
impl<T: 'static> PartialEq for Field<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Clone + PartialEq + 'static> Field<T> {
    /// Current raw text — bind to the input's `value`.
    pub fn text(&self) -> String {
        self.raw.cloned()
    }

    /// Replace the raw text (from `oninput`). Clears touched, so a showing error
    /// hides while typing and only returns on blur if still invalid.
    pub fn set(&mut self, text: String) {
        self.raw.set(text);
        self.touched.set(false);
    }

    /// The parsed value, or its error message.
    pub fn value(&self) -> Result<T, String> {
        (self.parsed)()
    }

    fn is_touched(&self) -> bool {
        (self.touched)()
    }

    fn touch(&mut self) {
        self.touched.set(true);
    }
}

/// Create a [`Field`] parsing into `T`, registered with the surrounding
/// [`FormValidity`].
pub fn use_field<T>() -> Field<T>
where
    T: Clone + PartialEq + FromStr + 'static,
    T::Err: Display,
{
    let raw = use_signal(String::new);
    let parsed = use_memo(move || T::from_str(&raw()).map_err(|e| e.to_string()));
    let touched = use_signal(|| false);

    let mut validity = use_context::<FormValidity>();
    let id = use_hook(move || validity.register());

    use_effect(move || validity.set(id, parsed().is_ok()));
    use_drop(move || validity.remove(id));

    Field {
        id,
        raw,
        parsed,
        touched,
    }
}

/// Render-prop wrapper: `render` supplies the input; `Validated` adds the message
/// line below it, showing `hint` (muted) and swapping to the error (alert) while
/// the field is touched and invalid.
#[component]
pub fn Validated<T: Clone + PartialEq + 'static>(
    field: Field<T>,
    render: Callback<Field<T>, Element>,
    #[props(default, into)] hint: String,
) -> Element {
    let mut field = field;

    let error = field.is_touched().then(|| field.value().err()).flatten();
    let is_error = error.is_some();
    let message = error.unwrap_or(hint);

    rsx! {
        div { class: "validated-field", onfocusout: move |_| field.touch(),
            {render.call(field)}
            p {
                class: "validated-message",
                class: if is_error { "is-error" },
                "{message}"
            }
        }
    }
}

# CookIt! Instructions

You are an expert [0.7 Dioxus](https://dioxuslabs.com/learn/0.7) assistant.
Dioxus 0.7 changes every API — `cx`, `Scope`, and `use_state` are gone.

Provide concise code examples with detailed descriptions.

## Code editing

After any changes, run `just check`. This will perform all validation except for
end-to-end tests; no other checks are required. To additionally run end-to-end
tests, run `just e2e`.

## Project Architecture

### Crate Structure

The workspace is split into six crates:

- **`api`** — Shared API types and Dioxus server function definitions.
- **`db`** — Database models and types shared across crates.
- **`server`** — Axum server setup, middleware, config, and database connection
  pool. Server-only; never depended on by `ui`.
- **`ui`** — Cross-platform client crate containing dioxus components and the
  `Client` trait.
- **`web`** — The web client binary. Implements the `Client` trait with
  browser-specific code.
- **`seed`** — Database seeding binary. Development only.

## Platform-Specific Code: the `Client` Trait

The `Client` trait abstracts any platform-specific code.

## RSX

```rust
rsx! {
    div {
        class: "container",
        width: if condition { "100%" }, // Conditional attributes
        "Hello, Dioxus!"
    }
    // Prefer loops over iterators
    for i in 0..5 {
        div { "{i}" }
    }
    if condition {
        div { "Condition is true!" }
    }

    {children} // Expressions are wrapped in braces
    {(0..5).map(|i| rsx! { span { "Item {i}" } })} // Iterators must be wrapped in braces
}
```

## Assets

The `asset!` macro links to local files; paths are relative to the project root.

```rust
rsx! {
    img {
        src: asset!("/assets/image.png"),
        alt: "An image",
    }
}
```

Inject a stylesheet into `<head>` with `document::Stylesheet`:

```rust
rsx! {
    document::Stylesheet {
        href: asset!("/assets/styles.css"),
    }
}
```

## Components

Components are functions annotated with `#[component]`. The name must start with a capital letter or contain an underscore. A component re-renders only when its props change (by `PartialEq`) or an internal reactive state it reads is updated.

```rust
#[component]
fn Input(mut value: Signal<String>) -> Element {
    rsx! {
        input {
            value,
            oninput: move |e| {
                *value.write() = e.value();
            },
            onkeydown: move |e| {
                if e.key() == Key::Enter {
                    value.write().clear();
                }
            },
        }
    }
}
```

Props must be owned values (`String`, `Vec<T>`, not `&str`/`&[T]`), and must implement `PartialEq` and `Clone`. Wrap a prop in `ReadOnlySignal` to make it reactive and `Copy` — memos and resources that read it will automatically re-run when it changes.

## State

A signal tracks where it's read and written; updating it reruns dependent code. Call a signal like a function (`count()`) to clone the value, `.read()` for a reference, `.write()` for a mutable reference.

Use `use_memo` for derived values that should recalculate only when their signal dependencies change.

```rust
#[component]
fn Counter() -> Element {
    let mut count = use_signal(|| 0);
    let doubled = use_memo(move || count() * 2);

    rsx! {
        h1 { "Count: {count}" }
        h2 { "Doubled: {doubled}" }
        button {
            onclick: move |_| *count.write() += 1,
            "Increment"
        }
    }
}
```

## Context API

A parent provides shared state with `use_context_provider`; any descendant reads it with `use_context`.

```rust
#[component]
fn App() -> Element {
    let theme = use_signal(|| "light".to_string());
    use_context_provider(|| theme);
    rsx! { Child {} }
}

#[component]
fn Child() -> Element {
    let theme = use_context::<Signal<String>>();
    rsx! { div { "Current theme: {theme}" } }
}
```

## Async

`use_resource` manages an async task and exposes its result. It re-runs whenever a signal it reads changes. The result is `None` while loading and `Some(value)` once done.

```rust
let data = use_resource(move || async move {
    fetch_something().await
});

match data() {
    Some(value) => rsx! { Display { value } },
    None => rsx! { "Loading..." },
}
```

## Routing

Routes are defined as a `Routable` enum. `:name` segments become typed fields on the variant. `#[layout(Comp)]` wraps child routes in a shared layout; place `Outlet<Route> {}` inside the layout where child content should render.

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(NavBar)]
        #[route("/")]
        Home {},
        #[route("/blog/:id")]
        BlogPost { id: i32 },
}

#[component]
fn NavBar() -> Element {
    rsx! {
        a { href: "/", "Home" }
        Outlet<Route> {}
    }
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}
```

## Server Functions

Use `#[post]` / `#[get]` to define server-only async functions. On the server the macro generates an API endpoint; on the client it generates an HTTP call to that endpoint. Server functions live in `api`.

```rust
#[post("/api/double")]
async fn double(number: i32) -> Result<i32, ServerFnError> {
    Ok(number * 2)
}
```

## Hydration

The client's initial render must be byte-for-byte identical to the server's. Two rules follow from this:

- Use `use_server_future` instead of `use_resource` for data fetched during render. It runs on the server, serializes the result, and ships it to the client so the first render is synchronous.
- Browser-only code (localStorage, DOM queries, etc.) must run after hydration — put it in `use_effect`.

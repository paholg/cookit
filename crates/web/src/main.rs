mod client;

fn main() {
    // Register the platform client. Everything else lives in `ui`.
    ui::initialize_client(Box::new(client::WebClient));

    #[cfg(not(feature = "server"))]
    {
        // Route wasm Rust panics to `console.error` with a real stack trace.
        // Without this the browser swallows them silently.
        console_error_panic_hook::set_once();
        dioxus::launch(ui::App);
    }

    #[cfg(feature = "server")]
    server::serve(ui::App)
}

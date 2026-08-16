//! Tracing subscriber setup: stdout logs, plus OTLP export when configured.

mod db;

use {
    crate::config::config,
    opentelemetry::trace::TracerProvider as _,
    opentelemetry_otlp::WithExportConfig as _,
    opentelemetry_sdk::{Resource, trace::SdkTracerProvider},
    std::sync::{Once, OnceLock},
    tracing::Level,
    tracing_subscriber::{
        EnvFilter, Layer, filter::Targets, layer::SubscriberExt, util::SubscriberInitExt,
    },
};

const SERVICE_NAME: &str = "cookit";

/// Set iff we're exporting.
static PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

/// Install the global tracing subscriber.
///
/// Must run before `dioxus::serve`: dioxus installs its own `FmtSubscriber`
/// unless a dispatcher is already set, and it does so before invoking the
/// router callback, so initializing from inside that callback is too late.
pub fn init() {
    let fmt_layer = tracing_subscriber::fmt::layer();
    // Under `dx`, timestamps and targets are noise — the CLI adds its own.
    let fmt_layer = if dioxus_cli_config::is_cli_enabled() {
        fmt_layer.without_time().with_target(false).boxed()
    } else {
        fmt_layer.boxed()
    };

    tracing_subscriber::registry()
        .with(fmt_layer.with_filter(env_filter()))
        .with(otlp_layer())
        .init();

    // Only the OTLP layer consumes query spans — the fmt layer doesn't print
    // spans at all — so don't make diesel render every query without one.
    if PROVIDER.get().is_some() {
        db::install();
    }
}

/// Flush buffered spans when the process is asked to exit.
///
/// Must be called from inside the tokio runtime, so not from [`init`]. Dioxus
/// re-invokes the router builder on every hot-patch, hence the [`Once`].
pub fn install_shutdown_flush() {
    static INSTALLED: Once = Once::new();

    // Nothing to flush without an exporter; leave the default signal handling
    // in place.
    if PROVIDER.get().is_none() {
        return;
    }

    INSTALLED.call_once(|| {
        tokio::spawn(async {
            shutdown_signal().await;

            // `shutdown` blocks until the exporter thread has drained.
            let flushed = tokio::task::spawn_blocking(|| {
                PROVIDER.get().map(SdkTracerProvider::shutdown).transpose()
            })
            .await;

            match flushed {
                Ok(Ok(_)) => (),
                Ok(Err(error)) => eprintln!("Failed to flush traces on shutdown: {error}"),
                Err(error) => eprintln!("Trace flush task failed: {error}"),
            }

            std::process::exit(0);
        });
    });
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = signal(SignalKind::terminate()).expect("failed to hook SIGTERM");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => (),
            _ = terminate.recv() => (),
        }
    }

    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

/// `debug` for our own crates in development, `info` for everything else.
///
/// A blanket `debug` is unusable: tokio-postgres logs every statement along
/// with the raw bytes of its parameters, and axum-session-auth logs a line per
/// session-cache hit. `RUST_LOG`, when set, replaces this wholesale.
fn env_filter() -> EnvFilter {
    const OUR_CRATES: [&str; 4] = ["server", "api", "db", "ui"];

    let default = if cfg!(debug_assertions) {
        let ours = OUR_CRATES.map(|krate| format!("{krate}=debug")).join(",");
        format!("info,{ours}")
    } else {
        "info".to_owned()
    };

    EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .parse_lossy(std::env::var("RUST_LOG").unwrap_or(default))
}

/// Export spans over OTLP when `OTLP_ENDPOINT` is set; `None` otherwise, which
/// leaves the subscriber with just the stdout layer.
fn otlp_layer<S>() -> Option<impl tracing_subscriber::Layer<S>>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let endpoint = config().otlp_endpoint.as_ref()?;

    // The http/protobuf exporter with the blocking client is deliberate: the
    // tonic one needs a tokio runtime at build time, and there is none yet —
    // `dioxus::serve` creates the runtime after this runs.
    let exporter = endpoint
        .join("v1/traces")
        .map_err(anyhow::Error::from)
        .and_then(|url| {
            opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .with_endpoint(url)
                .build()
                .map_err(Into::into)
        });

    let exporter = match exporter {
        Ok(exporter) => exporter,
        Err(error) => {
            // No subscriber yet, so this can't be a `tracing` event.
            eprintln!("Failed to build OTLP exporter; traces are disabled: {error}");
            return None;
        }
    };

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(Resource::builder().with_service_name(SERVICE_NAME).build())
        .build();

    let tracer = provider.tracer(SERVICE_NAME);
    // Both of these keep the provider — and so the exporter thread — alive.
    opentelemetry::global::set_tracer_provider(provider.clone());
    let _ = PROVIDER.set(provider);

    // Only our own spans are worth exporting; dependency spans would bury them.
    Some(
        tracing_opentelemetry::layer()
            .with_tracer(tracer)
            .with_filter(Targets::new().with_target("server", Level::INFO)),
    )
}

use actix_web::rt::task::JoinHandle;
use opentelemetry::runtime::TokioCurrentThread;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use tracing::subscriber::set_global_default;
use tracing::Subscriber;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Compose multiple layers into a `tracing`'s subscriber.
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of the returned subscriber, which is
/// quite complex.
/// We need to explicitly call out that the returned subscriber is
/// `Send` and `Sync` to make it possible to pass it to `init_subscriber`
/// later on.
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let app_name = "newsletter";

    // Start a new Jaeger trace pipeline.
    // Spans are exported in batch - recommended setup for a production application.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name(app_name)
        .install_batch(TokioCurrentThread)
        .expect("Failed to install Open Telemetry tracer");

    // Filter based on level - trace, debug, info, warn, error
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    // Create a `tracing` layer to emit spans as structured logs to stdout
    let formatting_layer = BunyanFormattingLayer::new(
        name, // Output the formatted spans to stdout
        sink,
    );

    // Create a `tracing` layer using the Jaeger tracer
    let opentelemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(opentelemetry_layer)
}

/// Register a subscriber as global default to process span data.
///
/// It should only be called once!
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    // Redirect all 'log' events to our subscriber
    LogTracer::init().expect("Failed to set logger");

    set_global_default(subscriber).expect("Failed to set subscriber");
}

/// Used for when we want to execute a function in a separate thread but still want the tracing logs
/// to work properly.
pub fn spawn_blocking_with_tracing<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let current_span = tracing::Span::current();

    tokio::task::spawn_blocking(move || {
        // We need to execute the function in the scope of the current span
        // because we are executing the function in a different thread.
        // This causes issues with tracing logs.
        current_span.in_scope(f)
    })
}

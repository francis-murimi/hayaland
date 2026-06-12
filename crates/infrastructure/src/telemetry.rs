use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialise a `tracing` subscriber.
///
/// When `json` is true the output is JSON-formatted (useful in production);
/// otherwise a human-readable pretty format is used.
pub fn init_subscriber(log_level: &str, json: bool) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
    let registry = tracing_subscriber::registry().with(env_filter);

    if json {
        registry.with(fmt_layer.json()).init();
    } else {
        registry.with(fmt_layer.pretty()).init();
    }
}

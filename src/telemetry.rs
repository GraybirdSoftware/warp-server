use tracing_subscriber::EnvFilter;

/// Initialise the global tracing subscriber.
///
/// `RUST_LOG` wins when set; otherwise `default_directives` is used.
/// Calling this more than once is harmless (later calls are ignored).
pub fn init(default_directives: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_directives));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}

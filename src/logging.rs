//! Application-wide logging setup.

/// Initialize the process-wide logger.
///
/// `RUST_LOG` controls verbosity. The default keeps normal editor activity
/// quiet while retaining useful diagnostics for Emerald and its dependencies.
pub fn init() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("emerald=info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

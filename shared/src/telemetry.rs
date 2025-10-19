

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};


pub fn init_tracing(
    service_name: &'static str,
    _jaeger_endpoint: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_level(true)
            .compact())
        .init();

    tracing::info!(service = service_name, "✅ Logging initialized");

    Ok(())
}


pub fn init_metrics(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use metrics_exporter_prometheus::PrometheusBuilder;
    use std::net::SocketAddr;

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();

    tracing::info!(
        addr = %addr,
        "Starting Prometheus metrics exporter"
    );

    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()?;

    tracing::info!("✅ Prometheus metrics exporter started at http://{}/metrics", addr);

    Ok(())
}

pub fn record_timing(metric_name: &'static str, duration_secs: f64) {
    metrics::histogram!(metric_name).record(duration_secs);
}

pub fn record_counter(metric_name: &'static str, value: u64) {
    metrics::counter!(metric_name).increment(value);
}


pub fn record_gauge(metric_name: &'static str, value: f64) {
    metrics::gauge!(metric_name).set(value);
}


pub async fn shutdown() {
    tracing::info!("✅ Telemetry shutdown complete");
}

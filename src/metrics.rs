use anyhow::{Context, Result};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;
use tracing::info;

/// Initialize metrics exporter
pub fn init_metrics(addr: SocketAddr) -> Result<()> {
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .context("Failed to install Prometheus exporter")?;

    info!("Metrics server listening on http://{}/metrics", addr);
    Ok(())
}

/// Metrics recorder for tracking scraper operations
pub struct Metrics;

impl Metrics {
    /// Record an HTTP request
    #[allow(dead_code)]
    pub fn record_http_request(_status: u16, _method: &str) {
        counter!("http_requests_total", 1);
    }

    /// Record an HTTP request duration
    #[allow(dead_code)]
    pub fn record_http_duration(duration_ms: f64, _method: &str) {
        histogram!("http_request_duration_ms", duration_ms);
    }

    /// Record a retry attempt
    #[allow(dead_code)]
    pub fn record_retry(_reason: &str) {
        counter!("http_retries_total", 1);
    }

    /// Record a database write
    #[allow(dead_code)]
    pub fn record_db_write(_table: &str, _success: bool) {
        counter!("db_writes_total", 1);
    }

    /// Record number of items processed in a scrape
    pub fn record_items_processed(count: u32) {
        counter!("items_processed_total", count as u64);
    }

    /// Record number of items saved
    pub fn record_items_saved(count: u32) {
        counter!("items_saved_total", count as u64);
    }

    /// Record scrape duration
    pub fn record_scrape_duration(duration_ms: u64) {
        histogram!("scrape_duration_ms", duration_ms as f64);
    }

    /// Record errors by type
    pub fn record_error(_error_type: &str) {
        counter!("errors_total", 1);
    }

    /// Update active scrapes gauge
    pub fn set_active_scrapes(count: i64) {
        gauge!("active_scrapes", count as f64);
    }

    /// Record normalization run
    pub fn record_normalization(stats_count: usize, duration_ms: u64) {
        counter!("normalizations_total", 1);
        gauge!("last_normalization_stats", stats_count as f64);
        histogram!("normalization_duration_ms", duration_ms as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        // Just verify the metrics can be called without panicking
        Metrics::record_http_request(200, "GET");
        Metrics::record_http_duration(123.45, "POST");
        Metrics::record_retry("rate_limit");
        Metrics::record_db_write("listings", true);
        Metrics::record_items_processed(100);
        Metrics::record_items_saved(98);
        Metrics::record_scrape_duration(5000);
        Metrics::record_error("network");
        Metrics::set_active_scrapes(2);
        Metrics::record_normalization(50, 2000);
    }
}

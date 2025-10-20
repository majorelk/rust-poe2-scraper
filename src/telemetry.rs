use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize the tracing subscriber based on configuration
pub fn init(log_format: &str) {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let registry = tracing_subscriber::registry().with(env_filter);

    if log_format == "json" {
        // JSON format for structured logging (production)
        let json_layer = fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true);

        registry.with(json_layer).init();
    } else {
        // Pretty format for human-readable logs (development)
        let pretty_layer = fmt::layer().pretty();

        registry.with(pretty_layer).init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_pretty_format() {
        // Test that init doesn't panic with pretty format
        // Note: We can't actually verify the output without capturing it,
        // but we can ensure the function executes without errors
        init("pretty");
    }

    #[test]
    fn test_init_json_format() {
        // Test that init doesn't panic with json format
        init("json");
    }
}

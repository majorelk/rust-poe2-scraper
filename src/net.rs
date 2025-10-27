use anyhow::{Context, Result};
use governor::{Quota, RateLimiter};
use reqwest::{Client, Response, StatusCode};
use std::num::NonZeroU32;
use std::time::Duration;
use tracing::{debug, warn};

/// HTTP client with rate limiting and retry logic
pub struct HttpClient {
    #[allow(dead_code)]
    client: Client,
    #[allow(dead_code)]
    rate_limiter: RateLimiter<
        governor::state::direct::NotKeyed,
        governor::state::InMemoryState,
        governor::clock::DefaultClock,
    >,
    #[allow(dead_code)]
    max_retries: u32,
    #[allow(dead_code)]
    base_backoff_ms: u64,
}

impl HttpClient {
    /// Create a new HTTP client with rate limiting
    pub fn new(user_agent: &str, requests_per_min: u32) -> Result<Self> {
        let client = Client::builder()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;

        // Convert requests per minute to quota
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_min).unwrap());
        let rate_limiter = RateLimiter::direct(quota);

        Ok(Self {
            client,
            rate_limiter,
            max_retries: 3,
            base_backoff_ms: 1000,
        })
    }

    /// Make a GET request with rate limiting and retries
    #[allow(dead_code)]
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.request_with_retry(|| self.client.get(url)).await
    }

    /// Make a POST request with rate limiting and retries
    #[allow(dead_code)]
    pub async fn post(&self, url: &str) -> reqwest::RequestBuilder {
        self.client.post(url)
    }

    /// Execute a request with rate limiting and retry logic
    #[allow(dead_code)]
    pub async fn execute_with_retry(
        &self,
        request_builder: reqwest::RequestBuilder,
    ) -> Result<Response> {
        self.request_with_retry(|| request_builder.try_clone().unwrap())
            .await
    }

    /// Internal method to handle retries with exponential backoff
    #[allow(dead_code)]
    async fn request_with_retry<F>(&self, request_fn: F) -> Result<Response>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let mut attempt = 0;

        loop {
            // Wait for rate limiter
            self.rate_limiter.until_ready().await;

            let response = request_fn()
                .send()
                .await
                .context("Failed to send HTTP request")?;

            let status = response.status();

            // Check if we should retry
            if self.should_retry(status) && attempt < self.max_retries {
                attempt += 1;

                // Calculate backoff with jitter
                let backoff = self.calculate_backoff(attempt, response.headers());

                warn!(
                    status = status.as_u16(),
                    attempt = attempt,
                    backoff_ms = backoff.as_millis(),
                    "Request failed, retrying after backoff"
                );

                tokio::time::sleep(backoff).await;
                continue;
            }

            // If successful or max retries reached, return the response
            if !status.is_success() && attempt >= self.max_retries {
                warn!(
                    status = status.as_u16(),
                    "Request failed after {} retries", self.max_retries
                );
            }

            return Ok(response);
        }
    }

    /// Check if a status code should trigger a retry
    #[allow(dead_code)]
    fn should_retry(&self, status: StatusCode) -> bool {
        matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS
                | StatusCode::INTERNAL_SERVER_ERROR
                | StatusCode::BAD_GATEWAY
                | StatusCode::SERVICE_UNAVAILABLE
                | StatusCode::GATEWAY_TIMEOUT
        )
    }

    /// Calculate backoff duration with exponential backoff and jitter
    #[allow(dead_code)]
    fn calculate_backoff(&self, attempt: u32, headers: &reqwest::header::HeaderMap) -> Duration {
        // Check for Retry-After header
        if let Some(retry_after) = headers.get("retry-after") {
            if let Ok(retry_str) = retry_after.to_str() {
                // Try to parse as seconds
                if let Ok(seconds) = retry_str.parse::<u64>() {
                    debug!("Using Retry-After header: {} seconds", seconds);
                    return Duration::from_secs(seconds);
                }
            }
        }

        // Exponential backoff: base * 2^(attempt - 1)
        let backoff_ms = self.base_backoff_ms * 2u64.pow(attempt - 1);

        // Add jitter (0-25% of backoff time)
        let jitter_ms = (rand::random::<u64>() % (backoff_ms / 4)).max(1);

        Duration::from_millis(backoff_ms + jitter_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_retry() {
        let client = HttpClient::new("Test/1.0", 60).unwrap();

        assert!(client.should_retry(StatusCode::TOO_MANY_REQUESTS));
        assert!(client.should_retry(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(client.should_retry(StatusCode::BAD_GATEWAY));
        assert!(client.should_retry(StatusCode::SERVICE_UNAVAILABLE));
        assert!(client.should_retry(StatusCode::GATEWAY_TIMEOUT));

        assert!(!client.should_retry(StatusCode::OK));
        assert!(!client.should_retry(StatusCode::NOT_FOUND));
        assert!(!client.should_retry(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn test_backoff_calculation() {
        let client = HttpClient::new("Test/1.0", 60).unwrap();
        let headers = reqwest::header::HeaderMap::new();

        // Test exponential backoff
        let backoff1 = client.calculate_backoff(1, &headers);
        let backoff2 = client.calculate_backoff(2, &headers);
        let backoff3 = client.calculate_backoff(3, &headers);

        // Each should be roughly double the previous (with jitter)
        assert!(backoff1.as_millis() >= 1000); // base * 2^0
        assert!(backoff2.as_millis() >= 2000); // base * 2^1
        assert!(backoff3.as_millis() >= 4000); // base * 2^2
    }

    #[test]
    fn test_retry_after_header() {
        let client = HttpClient::new("Test/1.0", 60).unwrap();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("retry-after", "5".parse().unwrap());

        let backoff = client.calculate_backoff(1, &headers);
        assert_eq!(backoff, Duration::from_secs(5));
    }

    #[tokio::test]
    #[ignore] // This is an integration test that hits a real endpoint
    async fn test_rate_limiting_integration() {
        let client = HttpClient::new("Test/1.0", 120).unwrap(); // 120 requests per minute = 2 per second

        let start = std::time::Instant::now();

        // Make 3 requests - should take at least 1 second due to rate limiting
        for _ in 0..3 {
            let _response = client.get("https://httpbin.org/get").await;
        }

        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(500)); // At least some rate limiting occurred
    }
}

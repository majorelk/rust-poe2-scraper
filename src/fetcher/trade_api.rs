use crate::errors::Result;
use crate::models::ItemResponse;
use crate::ScraperError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Serialize)]
pub struct SearchRequest {
    pub query: TradeQuery,
    pub sort: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SearchResponse {
    pub result: Vec<String>,
    #[serde(default)]
    pub total: Option<u32>,
    pub id: Option<String>,
    #[serde(default)]
    pub complexity: Option<u32>,
}

impl SearchResponse {
    pub fn get_result_ids(&self) -> &[String] {
        &self.result
    }
}

pub struct TradeApiClient {
    client: Client,
    league: String,
    last_request: Instant,
    rate_limit_delay: Duration,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub enum TradeStatus {
    Online,
    OnlineLeague,
    Any,
}

#[derive(Debug, Serialize)]
pub struct TradeQuery {
    pub status: StatusFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub stats: Vec<StatFilter>,
}

#[derive(Debug, Serialize)]
pub struct StatFilter {
    pub r#type: String,
    pub filters: Vec<StatFilterValue>,
    pub disabled: bool,
}

#[derive(Debug, Serialize)]
pub struct StatFilterValue {
    pub id: String,
    pub value: Option<StatValue>,
    pub disabled: bool,
}

#[derive(Debug, Serialize)]
pub struct StatValue {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct StatusFilter {
    pub option: String,
}

#[allow(dead_code)]
impl TradeStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TradeStatus::Online => "online",
            TradeStatus::OnlineLeague => "onlineleague",
            TradeStatus::Any => "any",
        }
    }
}

impl TradeApiClient {
    pub fn new(league: String) -> Self {
        // Get rate limit delay from environment or use default
        let rate_limit_ms = std::env::var("RATE_LIMIT_DELAY_MS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(500);

        info!(
            "Initializing Trade API client with {}ms rate limit",
            rate_limit_ms
        );

        Self {
            client: Client::new(),
            league,
            last_request: Instant::now(),
            rate_limit_delay: Duration::from_millis(rate_limit_ms),
        }
    }

    async fn process_raw_item(&self, raw_item: serde_json::Value) -> Result<ItemResponse> {
        debug!(
            "Processing raw item structure: {}",
            serde_json::to_string_pretty(&raw_item).unwrap_or_default()
        );

        match serde_json::from_value::<ItemResponse>(raw_item.clone()) {
            Ok(response) => {
                debug!(
                    "Successfully processed item: ID={}, Base={}, Type={}, Price={} {}",
                    response.id,
                    response.item.base_type,
                    response.item.type_line,
                    response.listing.price.amount,
                    response.listing.price.currency
                );
                Ok(response)
            }
            Err(e) => {
                warn!("Failed to process item: {}", e);
                if let Some(obj) = raw_item.as_object() {
                    for (key, _) in obj {
                        debug!("Raw item field: {}", key);
                    }
                }
                Err(ScraperError::ParseError(format!(
                    "Failed to parse item: {}",
                    e
                )))
            }
        }
    }

    pub async fn fetch_items(&mut self, ids: &[String]) -> Result<Vec<serde_json::Value>> {
        let mut all_items = Vec::new();

        // Process IDs in batches of 10
        for chunk in ids.chunks(10) {
            // Add randomness to avoid synchronization
            let jitter_ms = rand::random::<u64>() % 100;
            let delay = self.rate_limit_delay + Duration::from_millis(jitter_ms);
            self.respect_rate_limit(delay).await;

            let ids_str = chunk.join(",");
            let url = format!("https://www.pathofexile.com/api/trade2/fetch/{}", ids_str);

            info!("Fetching {} items", chunk.len());
            debug!("Fetch URL: {}", url);

            let response = self
                .client
                .get(&url)
                .header(
                    "User-Agent",
                    "POE2-Scraper/0.1.0 (https://github.com/majorelk/rust-poe2-scraper)",
                )
                .header("Accept", "*/*")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("Content-Type", "application/json")
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Origin", "https://www.pathofexile.com")
                .header(
                    "Referer",
                    format!(
                        "https://www.pathofexile.com/trade2/search/poe2/{}",
                        self.league
                    ),
                )
                .send()
                .await?;

            let status = response.status();
            debug!("Fetch response status: {}", status);

            let response_text = response.text().await?;
            debug!("Fetch response body: {}", response_text);

            // If we hit rate limit, implement exponential backoff
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                warn!("Rate limit hit, implementing backoff");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            if status.is_success() {
                let json_response: serde_json::Value = serde_json::from_str(&response_text)?;
                if let Some(items) = json_response["result"].as_array() {
                    all_items.extend(items.to_vec());
                }
            }

            self.last_request = Instant::now();
        }

        Ok(all_items)
    }

    pub async fn search_items(&mut self, query: SearchRequest) -> Result<SearchResponse> {
        let jitter_ms = rand::random::<u64>() % 100;
        let delay = self.rate_limit_delay + Duration::from_millis(jitter_ms);
        self.respect_rate_limit(delay).await;

        let url = format!(
            "https://www.pathofexile.com/api/trade2/search/poe2/{}",
            self.league
        );

        info!("Sending search request");
        debug!("Search URL: {}", url);
        debug!(
            "Query payload: {}",
            serde_json::to_string_pretty(&query).unwrap_or_default()
        );

        let response = self
            .client
            .post(&url)
            .header(
                "User-Agent",
                "POE2-Scraper/0.1.0 (https://github.com/majorelk/rust-poe2-scraper)",
            )
            .header("Accept", "*/*")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Content-Type", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", "https://www.pathofexile.com")
            .header(
                "Referer",
                format!(
                    "https://www.pathofexile.com/trade2/search/poe2/{}",
                    self.league
                ),
            )
            .json(&query)
            .send()
            .await?;

        debug!("Search response status: {}", response.status());

        let response_text = response.text().await?;
        
        // Log first 500 chars of response for debugging
        let preview = if response_text.len() > 500 {
            &response_text[..500]
        } else {
            &response_text
        };
        debug!("Search response body preview: {}", preview);

        match serde_json::from_str::<SearchResponse>(&response_text) {
            Ok(parsed) => {
                self.last_request = Instant::now();
                Ok(parsed)
            }
            Err(e) => {
                warn!("Failed to parse search response: {}. Response: {}", e, preview);
                Err(crate::errors::ScraperError::ParseError(format!(
                    "Failed to parse search response: {}. Response was: {}",
                    e, preview
                )))
            }
        }
    }

    async fn respect_rate_limit(&self, delay: Duration) {
        let elapsed = self.last_request.elapsed();
        if elapsed < delay {
            tokio::time::sleep(delay - elapsed).await;
        }
    }

    #[allow(dead_code)]
    pub fn build_basic_query(&self, status: TradeStatus) -> SearchRequest {
        SearchRequest {
            query: TradeQuery {
                status: StatusFilter {
                    option: status.as_str().to_string(),
                },
                r#type: None, // No specific item type filter
                stats: vec![StatFilter {
                    r#type: "and".to_string(),
                    filters: vec![],
                    disabled: false,
                }],
            },
            sort: Some(serde_json::json!({
                "price": "asc"
            })),
        }
    }

    #[allow(dead_code)]
    pub fn build_jewel_query(&self, status: TradeStatus) -> SearchRequest {
        SearchRequest {
            query: TradeQuery {
                status: StatusFilter {
                    option: status.as_str().to_string(),
                },
                r#type: Some("jewel".to_string()), // Filter for jewels
                stats: vec![StatFilter {
                    r#type: "and".to_string(),
                    filters: vec![],
                    disabled: false,
                }],
            },
            sort: Some(serde_json::json!({
                "price": "asc"
            })),
        }
    }

    pub async fn fetch_items_with_stats_limited(
        &mut self,
        query: SearchRequest,
        limit: Option<usize>,
    ) -> Result<Vec<ItemResponse>> {
        info!("Starting items with stats fetch");

        let search_response = self.search_items(query).await?;
        info!("Search returned {} results", search_response.result.len());

        // Apply limit to the number of result IDs to fetch
        let result_ids = search_response.get_result_ids();
        let ids_to_fetch: Vec<String> = if let Some(lim) = limit {
            result_ids.iter().take(lim).cloned().collect()
        } else {
            result_ids.to_vec()
        };

        let raw_items = self.fetch_items(&ids_to_fetch).await?;
        let total_items = raw_items.len();
        info!("Fetched {} raw items", total_items);

        let mut processed_items = Vec::new();
        let mut failed_count = 0;

        // Process each raw item using our diagnostic method
        for raw_item in raw_items {
            match self.process_raw_item(raw_item.clone()).await {
                Ok(item) => {
                    debug!(
                        "Processed item: {} - {} {}",
                        item.id, item.item.base_type, item.listing.price.amount
                    );
                    processed_items.push(item);
                }
                Err(e) => {
                    warn!("Failed to process item: {}", e);
                    failed_count += 1;
                }
            }

            // Stop processing if we've reached the limit
            if let Some(lim) = limit {
                if processed_items.len() >= lim {
                    break;
                }
            }
        }

        info!("Processing summary:");
        info!("Total items attempted: {}", total_items);
        info!("Successfully processed: {}", processed_items.len());
        info!("Failed to process: {}", failed_count);

        Ok(processed_items)
    }
}

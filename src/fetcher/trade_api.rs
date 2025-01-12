use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::errors::Result;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize)]
pub struct SearchRequest {
    pub query: serde_json::Value,
    pub sort: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    result: Vec<String>,
    total: u32,
    id: Option<String>,
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

impl TradeApiClient {
    pub fn new(league: String) -> Self {
        Self {
            client: Client::new(),
            league,
            last_request: Instant::now(),
            rate_limit_delay: Duration::from_millis(100),
        }
    }

    pub async fn fetch_items(&mut self, ids: &[String]) -> Result<Vec<serde_json::Value>> {
        self.respect_rate_limit().await;

        let ids_str = ids.join(",");
        let url = format!(
            "https://www.pathofexile.com/api/trade/fetch/{}",
            ids_str
        );

        let response = self.client
            .get(&url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        self.last_request = Instant::now();

        // Extract results array from response
        Ok(response["result"]
            .as_array()
            .unwrap_or(&Vec::new())
            .to_vec())
    }

    pub async fn search_items(&mut self, query: SearchRequest) -> Result<SearchResponse> {
        self.respect_rate_limit().await;
        
        let url = format!(
            "https://www.pathofexile.com/api/trade/search/{}",
            self.league
        );

        let response = self.client
            .post(&url)
            .json(&query)
            .send()
            .await?
            .json::<SearchResponse>()
            .await?;

        self.last_request = Instant::now();
        Ok(response)
    }
    
    async fn respect_rate_limit(&self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.rate_limit_delay {
            tokio::time::sleep(self.rate_limit_delay - elapsed).await;
        }
    }
}
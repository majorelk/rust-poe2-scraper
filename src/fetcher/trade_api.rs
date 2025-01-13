use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::errors::Result;
use std::time::{Duration, Instant};
use crate::models::Item;

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

#[derive(Debug, Serialize)]
pub enum TradeStatus {
    Online,
    OnlineLeague,
    Any,
}

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
            "https://www.pathofexile.com/api/trade2/fetch/{}",
            ids_str
        );

        println!("Fetching items from: {}", url);

        let response = self.client
            .get(&url)
            .header("User-Agent", "OAuth 2.0 Client 1.0")
            .send()
            .await?;

        println!("Fetch response status: {}", response.status());
        
        let response_text = response.text().await?;
        println!("Fetch response body: {}", response_text);

        let json_response: serde_json::Value = serde_json::from_str(&response_text)?;

        self.last_request = Instant::now();

        // Extract results array from response
        Ok(json_response["result"]
            .as_array()
            .unwrap_or(&Vec::new())
            .to_vec())
    }

    pub async fn search_items(&mut self, query: SearchRequest) -> Result<SearchResponse> {
        self.respect_rate_limit().await;
        
        let url = format!(
            "https://www.pathofexile.com/api/trade2/search/poe2/{}",
            self.league
        );

        println!("Sending search request to: {}", url);
        println!("Query payload: {}", serde_json::to_string_pretty(&query).unwrap_or_default());

        let response = self.client
            .post(&url)
            .header("User-Agent", "OAuth 2.0 Client 1.0")
            .header("Content-Type", "application/json")
            .json(&query)
            .send()
            .await?;

        println!("Search response status: {}", response.status());
        
        let response_text = response.text().await?;
        println!("Search response body: {}", response_text);

        // Try to parse the response body into our SearchResponse struct
        match serde_json::from_str::<SearchResponse>(&response_text) {
            Ok(parsed) => {
                self.last_request = Instant::now();
                Ok(parsed)
            },
            Err(e) => {
                eprintln!("Failed to parse search response: {}", e);
                eprintln!("Response body was: {}", response_text);
                Err(crate::errors::ScraperError::ParseError(format!(
                    "Failed to parse search response: {}. Response body: {}", 
                    e, response_text
                )))
            }
        }
    }
    
    async fn respect_rate_limit(&self) {
        let elapsed = self.last_request.elapsed();
        if elapsed < self.rate_limit_delay {
            tokio::time::sleep(self.rate_limit_delay - elapsed).await;
        }
    }

    pub fn build_basic_query(&self, status: TradeStatus) -> SearchRequest {
        SearchRequest {
            query: serde_json::json!({
                "query": {
                    "stats": [
                        {
                            "filters": [],
                            "type": "and"
                        }
                    ],
                    "status": {
                        "option": status.as_str()
                    }
                },
                "sort": {
                    "price": "asc"
                }
            }),
            sort: None,
        }
    }
    
    pub async fn fetch_items_with_stats(&mut self, query: SearchRequest) -> Result<Vec<Item>> {
        println!("Starting items with stats fetch...");
        
        let search_response = self.search_items(query).await?;
        println!("Search returned {} results", search_response.result.len());
        
        let raw_items = self.fetch_items(search_response.get_result_ids()).await?;
        println!("Fetched {} raw items", raw_items.len());
        
        let items: Vec<Item> = raw_items
            .into_iter()
            .filter_map(|raw_item| {
                // Clone raw_item before moving it into from_value
                match serde_json::from_value::<Item>(raw_item.clone()) {
                    Ok(item) => Some(item),
                    Err(e) => {
                        eprintln!("Failed to parse item: {}", e);
                        eprintln!("Raw item data: {}", serde_json::to_string_pretty(&raw_item).unwrap_or_default());
                        None
                    }
                }
            })
            .collect();
    
        println!("Successfully parsed {} items", items.len());
        Ok(items)
    }
}
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::errors::Result;
use std::time::{Duration, Instant};
use crate::models::{Item, ItemResponse};
use rand; // 0.8.4

#[derive(Debug, Serialize)]
pub struct SearchRequest {
    pub query: TradeQuery,
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

#[derive(Debug, Serialize)]
pub struct TradeQuery {
    pub status: StatusFilter,
    pub stats: Vec<StatFilter>,
    pub filters: QueryFilters,
}

#[derive(Debug, Serialize)]
pub struct QueryFilters {
    pub type_filters: TypeFilters,
}

#[derive(Debug, Serialize)]
pub struct TypeFilters {
    pub filters: CategoryFilter,
}

#[derive(Debug, Serialize)]
pub struct CategoryFilter {
    pub category: CategoryOption,
}

#[derive(Debug, Serialize)]
pub struct CategoryOption {
    pub option: String,
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
        let mut all_items = Vec::new();
        
        // Process IDs in batches of 10
        for chunk in ids.chunks(10) {
            // Increase the base delay and add some randomness to avoid synchronization
            let delay = Duration::from_millis(500 + (rand::random::<u64>() % 100));
            self.respect_rate_limit(delay).await;
    
            let ids_str = chunk.join(",");
            let url = format!(
                "https://www.pathofexile.com/api/trade2/fetch/{}",
                ids_str
            );
    
            println!("Fetching items from: {}", url);
    
            let response = self.client
                .get(&url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0")
                .header("Accept", "*/*")
                .header("Accept-Language", "en-US,en;q=0.5")
                .header("Content-Type", "application/json")
                .header("X-Requested-With", "XMLHttpRequest")
                .header("Origin", "https://www.pathofexile.com")
                .header("Referer", format!("https://www.pathofexile.com/trade2/search/poe2/{}", self.league))
                .send()
                .await?;
    
            let status = response.status();
            println!("Fetch response status: {}", status);
            
            let response_text = response.text().await?;
            println!("Fetch response body: {}", response_text);
    
            // If we hit rate limit, wait and retry
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                println!("Rate limit hit, waiting 5 seconds before retry...");
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
        let delay = Duration::from_millis(500 + (rand::random::<u64>() % 100));
        self.respect_rate_limit(delay).await;
        
        let url = format!(
            "https://www.pathofexile.com/api/trade2/search/poe2/{}",
            self.league
        );

        println!("Sending search request to: {}", url);
        println!("Query payload: {}", serde_json::to_string_pretty(&query).unwrap_or_default());

        let response = self.client
            .post(&url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0")
            .header("Accept", "*/*")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Content-Type", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", "https://www.pathofexile.com")
            .header("Referer", format!("https://www.pathofexile.com/trade2/search/poe2/{}", self.league))
            .json(&query)
            .send()
            .await?;

        println!("Search response status: {}", response.status());
        
        let response_text = response.text().await?;
        println!("Search response body: {}", response_text);

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
    
    async fn respect_rate_limit(&self, delay: Duration) {
        let elapsed = self.last_request.elapsed();
        if elapsed < delay {
            tokio::time::sleep(delay - elapsed).await;
        }
    }

    pub fn build_basic_query(&self, status: TradeStatus) -> SearchRequest {
        SearchRequest {
            query: TradeQuery {
                status: StatusFilter {
                    option: status.as_str().to_string(),
                },
                stats: vec![StatFilter {
                    r#type: "and".to_string(),
                    filters: vec![],
                    disabled: false,
                }],
                filters: QueryFilters {
                    type_filters: TypeFilters {
                        filters: CategoryFilter {
                            category: CategoryOption {
                                option: "any".to_string(),
                            },
                        },
                    },
                },
            },
            sort: Some(serde_json::json!({
                "price": "asc"
            })),
        }
    }
    
    pub fn build_jewel_query(&self, status: TradeStatus) -> SearchRequest {
        SearchRequest {
            query: TradeQuery {
                status: StatusFilter {
                    option: status.as_str().to_string(),
                },
                stats: vec![StatFilter {
                    r#type: "and".to_string(),
                    filters: vec![],
                    disabled: false,
                }],
                filters: QueryFilters {
                    type_filters: TypeFilters {
                        filters: CategoryFilter {
                            category: CategoryOption {
                                option: "jewel".to_string(),
                            },
                        },
                    },
                },
            },
            sort: Some(serde_json::json!({
                "price": "asc"
            })),
        }
    }

    pub async fn fetch_items_with_stats(&mut self, query: SearchRequest) -> Result<Vec<ItemResponse>> {
        println!("Starting items with stats fetch...");
        
        let search_response = self.search_items(query).await?;
        println!("Search returned {} results", search_response.result.len());
        
        let raw_items = self.fetch_items(search_response.get_result_ids()).await?;
        println!("Fetched {} raw items", raw_items.len());
        
        let items: Vec<ItemResponse> = raw_items
            .into_iter()
            .filter_map(|raw_item| {
                match serde_json::from_value::<ItemResponse>(raw_item.clone()) {
                    Ok(item) => {
                        // Log useful information about each item
                        println!("Processed item: {} - {} {}", 
                            item.id,
                            item.item.base_type,
                            item.listing.price.amount);
                        Some(item)
                    },
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
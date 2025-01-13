use crate::fetcher::{TradeApiClient, SearchRequest};
use crate::models::{CoreAttribute, Item};
use crate::errors::Result;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

pub struct StatCollector {
    client: TradeApiClient,
    // Store thresholds as ranges to get a better distribution of items
    threshold_ranges: Vec<(u32, u32)>,
    rate_limit_delay: Duration,
}

impl StatCollector {
    pub fn new(client: TradeApiClient) -> Self {
        Self {
            client,
            // Define ranges that will give us a good spread of stat requirements
            threshold_ranges: vec![
                (0, 50),    // Low requirement items
                (51, 100),  // Medium requirement items
                (101, 150), // High requirement items
                (151, 200), // Very high requirement items
            ],
            rate_limit_delay: Duration::from_millis(100),
        }
    }

    pub async fn collect_stat_data(&mut self) -> Result<Vec<Item>> {
        let mut all_items = Vec::new();
        
        // Collect items for each attribute type
        for attr in [CoreAttribute::Strength, CoreAttribute::Dexterity, CoreAttribute::Intelligence] {
            for (min, max) in &self.threshold_ranges {
                // Build query for this attribute range
                let query = self.build_attribute_query(attr.clone(), *min, *max);
                
                // Fetch items and respect rate limiting
                sleep(self.rate_limit_delay).await;
                let items = self.client.fetch_items_with_stats(query).await?;
                
                println!("Collected {} items for {:?} ({}-{})", 
                    items.len(), attr, min, max);
                
                all_items.extend(items);
            }
        }
        
        Ok(all_items)
    }

    fn build_attribute_query(&self, attr: CoreAttribute, min: u32, max: u32) -> SearchRequest {
        // The API expects specific stat IDs for attribute requirements.
        // These IDs are fixed and correspond to the game's internal representation
        // of attribute requirements.
        let stat_id = match attr {
            CoreAttribute::Strength => "explicit.stat_3299347043",
            CoreAttribute::Dexterity => "explicit.stat_1284417561",
            CoreAttribute::Intelligence => "explicit.stat_4220027924",
        };
    
        SearchRequest {
            query: serde_json::json!({
                "query": {
                    // The status filter tells the API we only want items from online sellers
                    "status": {
                        "option": "online"
                    },
                    // Stats need to be structured as filters within a stat group
                    "stats": [{
                        "type": "and",  // This indicates all filters must match
                        "filters": [{
                            "id": stat_id,  // We use the specific stat ID instead of the attribute name
                            "value": {
                                "min": min,
                                "max": max
                            }
                        }]
                    }],
                    // Price filters go in a separate trade_filters section
                    "filters": {
                        "trade_filters": {
                            "filters": {
                                "price": {
                                    "min": 1,
                                    "max": 10
                                }
                            }
                        }
                    }
                }
            }),
            sort: None,  // The sort is now part of the main query json
        }
    }

    // Helper method to save collected data for later analysis
    pub async fn save_collected_data(&self, items: &[Item], path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(items)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    // Helper method to load previously collected data
    pub async fn load_collected_data(path: &str) -> Result<Vec<Item>> {
        let content = tokio::fs::read_to_string(path).await?;
        let items = serde_json::from_str(&content)?;
        Ok(items)
    }
}
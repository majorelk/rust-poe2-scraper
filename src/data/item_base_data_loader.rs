use crate::errors::Result;
use crate::models::{CoreAttribute, ItemBaseType, ItemCategory};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct TradeApiResponse {
    result: Vec<TradeApiCategory>,
}

#[derive(Debug, Deserialize)]
struct TradeApiCategory {
    #[allow(dead_code)]
    id: String,
    label: String,
    entries: Vec<TradeApiEntry>,
}

#[derive(Debug, Deserialize)]
struct TradeApiEntry {
    #[serde(rename = "type")]
    type_name: String,
    #[allow(dead_code)]
    text: String,
}

pub struct BaseDataLoader {
    client: Client,
    base_cache: HashMap<String, ItemBaseType>,
    last_update: std::time::SystemTime,
}

#[allow(dead_code)]
impl BaseDataLoader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_cache: HashMap::new(),
            last_update: std::time::SystemTime::now(),
        }
    }

    pub fn get_all_bases(&self) -> impl Iterator<Item = &ItemBaseType> {
        self.base_cache.values()
    }

    // Load base items from a JSON file (for initial/fallback data)
    pub async fn load_from_file(&mut self, path: &str) -> Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        let bases: HashMap<String, ItemBaseType> = serde_json::from_str(&content)?;
        self.base_cache = bases;
        Ok(())
    }

    // Save current base items to a JSON file
    pub async fn save_to_file(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.base_cache)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    // Update base items from the trade API
    pub async fn update_from_api(&mut self, api_url: &str) -> Result<()> {
        let response = self
            .client
            .get(api_url)
            .header(
                "User-Agent",
                "POE2-Scraper/0.1.0 (https://github.com/majorelk/rust-poe2-scraper)",
            )
            .header("Accept", "*/*")
            .header("Accept-Language", "en-US,en;q=0.5")
            .header("Content-Type", "application/json")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", "https://www.pathofexile.com")
            .header("Referer", "https://www.pathofexile.com/trade2")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await?;
            let snippet = if body.len() > 200 {
                format!("{}...", &body[..200])
            } else {
                body.clone()
            };
            return Err(crate::ScraperError::ApiError(format!(
                "Base data API returned status {}: {}",
                status, snippet
            )));
        }

        let response_text = response.text().await?;
        let api_response: TradeApiResponse = serde_json::from_str(&response_text)
            .map_err(|e| crate::ScraperError::ApiError(format!(
                "Failed to parse base data JSON: {}. Response: {}",
                e,
                if response_text.len() > 200 {
                    format!("{}...", &response_text[..200])
                } else {
                    response_text.clone()
                }
            )))?;

        for category in api_response.result {
            for entry in category.entries {
                if let Some(base_type) = self.convert_api_entry(entry, &category.label) {
                    self.base_cache.insert(base_type.name.clone(), base_type);
                }
            }
        }

        self.last_update = std::time::SystemTime::now();
        Ok(())
    }

    // Convert API entry to our internal ItemBaseType
    fn convert_api_entry(&self, entry: TradeApiEntry, category_label: &str) -> Option<ItemBaseType> {
        let category = self.determine_category(category_label)?;
        // Use the type_name as the base item name
        let base_type = ItemBaseType::new(entry.type_name, category);
        
        // Note: The /api/trade2/data/items endpoint doesn't include requirement data
        // Requirements would need to be fetched from /api/trade2/data/static or determined separately
        
        Some(base_type)
    }

    // Map API category strings to our ItemCategory enum
    fn determine_category(&self, api_category: &str) -> Option<ItemCategory> {
        match api_category.to_lowercase().as_str() {
            "weapons" => Some(ItemCategory::Weapon),
            "armour" | "armor" => Some(ItemCategory::Armour),
            "accessories" => Some(ItemCategory::Accessory),
            "flasks" => Some(ItemCategory::Flask),
            "gems" => Some(ItemCategory::Gem),
            "currency" => Some(ItemCategory::Currency),
            "cards" => Some(ItemCategory::DivinationCard),
            "maps" => Some(ItemCategory::Map),
            _ => Some(ItemCategory::Other),
        }
    }

    // Get a base type by name
    pub fn get_base(&self, name: &str) -> Option<&ItemBaseType> {
        self.base_cache.get(name)
    }

    // Get all bases matching certain criteria
    pub fn get_bases_by_attribute(&self, attr: CoreAttribute) -> Vec<&ItemBaseType> {
        self.base_cache
            .values()
            .filter(|base| base.stat_requirements.primary_attributes.contains(&attr))
            .collect()
    }

    // Check if the cache needs updating (e.g., if it's older than 24 hours)
    pub fn needs_update(&self, update_interval: std::time::Duration) -> bool {
        self.last_update.elapsed().unwrap_or_default() > update_interval
    }

    // Get statistics about the current base cache
    pub fn get_cache_stats(&self) -> serde_json::Value {
        let mut category_counts = HashMap::new();
        let mut attribute_counts = HashMap::new();

        for base in self.base_cache.values() {
            *category_counts
                .entry(format!("{:?}", base.category))
                .or_insert(0) += 1;

            for attr in &base.stat_requirements.primary_attributes {
                *attribute_counts.entry(format!("{:?}", attr)).or_insert(0) += 1;
            }
        }

        serde_json::json!({
            "total_bases": self.base_cache.len(),
            "categories": category_counts,
            "attribute_requirements": attribute_counts,
            "last_update": format!("{:?}", self.last_update),
        })
    }
}

#[allow(dead_code)]
pub async fn initialize_base_loader(_league: &str) -> Result<BaseDataLoader> {
    let mut loader = BaseDataLoader::new();

    // The base item data endpoint is global, not league-specific
    // It returns all base item types without modifiers
    let api_url = "https://www.pathofexile.com/api/trade2/data/items";

    // Try to load initial data from file
    if (loader.load_from_file("data/item_bases.json").await).is_err() {
        // If file doesn't exist or is invalid, update from API
        loader
            .update_from_api(&api_url)
            .await
            .map_err(|e| crate::ScraperError::ApiError(format!("Failed to fetch base item data from API: {}", e)))?;
        // Save the fresh data
        loader.save_to_file("data/item_bases.json").await?;
    }

    // Check if data needs updating
    if loader.needs_update(std::time::Duration::from_secs(86400)) {
        // 24 hours
        loader
            .update_from_api(&api_url)
            .await?;
        loader.save_to_file("data/item_bases.json").await?;
    }

    Ok(loader)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_loader_initialization() {
        let loader = BaseDataLoader::new();
        assert!(loader.base_cache.is_empty());
    }

    #[test]
    fn test_category_determination() {
        let loader = BaseDataLoader::new();
        assert!(matches!(
            loader.determine_category("Weapons"),
            Some(ItemCategory::Weapon)
        ));
        assert!(matches!(
            loader.determine_category("Armour"),
            Some(ItemCategory::Armour)
        ));
        assert!(matches!(
            loader.determine_category("Unknown"),
            Some(ItemCategory::Other)
        ));
    }
}

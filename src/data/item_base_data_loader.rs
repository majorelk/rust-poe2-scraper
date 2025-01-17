use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest::Client;
use crate::models::{
    CoreAttribute,
    StatRequirements,
    ItemBaseType,
    ItemCategory,
};
use crate::errors::Result;

#[derive(Debug, Deserialize)]
struct TradeApiBase {
    name: String,
    category: String,
    requirements: Option<BaseRequirements>,
    // Add other fields as needed based on the API response
}

#[derive(Debug, Deserialize)]
struct BaseRequirements {
    strength: Option<u32>,
    dexterity: Option<u32>,
    intelligence: Option<u32>,
    level: Option<u32>,
}

pub struct BaseDataLoader {
    client: Client,
    base_cache: HashMap<String, ItemBaseType>,
    last_update: std::time::SystemTime,
}

impl BaseDataLoader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_cache: HashMap::new(),
            last_update: std::time::SystemTime::now(),
        }
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
        let response = self.client.get(api_url)
            .send()
            .await?
            .json::<Vec<TradeApiBase>>()
            .await?;

        for base in response {
            if let Some(base_type) = self.convert_api_base(base) {
                self.base_cache.insert(base_type.name.clone(), base_type);
            }
        }

        self.last_update = std::time::SystemTime::now();
        Ok(())
    }

    // Convert API response to our internal ItemBaseType
    fn convert_api_base(&self, api_base: TradeApiBase) -> Option<ItemBaseType> {
        let category = self.determine_category(&api_base.category)?;
        let mut base_type = ItemBaseType::new(api_base.name, category);

        if let Some(reqs) = api_base.requirements {
            // Add strength requirement if present
            if let Some(str_req) = reqs.strength {
                base_type.stat_requirements.add_requirement(
                    CoreAttribute::Strength,
                    str_req
                );
            }

            // Add dexterity requirement if present
            if let Some(dex_req) = reqs.dexterity {
                base_type.stat_requirements.add_requirement(
                    CoreAttribute::Dexterity,
                    dex_req
                );
            }

            // Add intelligence requirement if present
            if let Some(int_req) = reqs.intelligence {
                base_type.stat_requirements.add_requirement(
                    CoreAttribute::Intelligence,
                    int_req
                );
            }

            // Set base level if available
            if let Some(level) = reqs.level {
                base_type.base_level = level;
            }
        }

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
        self.base_cache.values()
            .filter(|base| {
                base.stat_requirements.primary_attributes.contains(&attr)
            })
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
            *category_counts.entry(format!("{:?}", base.category)).or_insert(0) += 1;
            
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

pub async fn initialize_base_loader() -> Result<BaseDataLoader> {
    let mut loader = BaseDataLoader::new();

    // Try to load initial data from file
    if let Err(_) = loader.load_from_file("data/item_bases.json").await {
        // If file doesn't exist or is invalid, update from API
        loader.update_from_api("https://api.pathofexile.com/trade/data/items").await?;
        // Save the fresh data
        loader.save_to_file("data/item_bases.json").await?;
    }

    // Check if data needs updating
    if loader.needs_update(std::time::Duration::from_secs(86400)) {  // 24 hours
        loader.update_from_api("https://api.pathofexile.com/trade/data/items").await?;
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
        assert!(matches!(loader.determine_category("Weapons"), Some(ItemCategory::Weapon)));
        assert!(matches!(loader.determine_category("Armour"), Some(ItemCategory::Armour)));
        assert!(matches!(loader.determine_category("Unknown"), Some(ItemCategory::Other)));
    }
}
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A listing from the trade API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub id: String,
    pub item: Item,
    pub price: Option<Price>,
    pub account: Account,
    pub whisper: Option<String>,
    pub indexed: Option<String>,
    pub stash: Option<StashInfo>,
}

/// An item in a listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub name: Option<String>,
    pub base_type: String,
    pub type_line: String,
    pub item_level: u32,
    pub rarity: String,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub corrupted: Option<bool>,
    pub identified: Option<bool>,
    pub properties: Option<Vec<Property>>,
    pub requirements: Option<Vec<Requirement>>,
    pub modifiers: Vec<Modifier>,
    pub implicit_mods: Option<Vec<String>>,
    pub explicit_mods: Option<Vec<String>>,
}

/// A modifier on an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modifier {
    pub name: String,
    pub tier: Option<String>,
    pub level: Option<u32>,
    pub magnitudes: Option<Vec<Magnitude>>,
}

/// Magnitude of a modifier value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Magnitude {
    pub hash: String,
    #[serde(rename = "min")]
    pub min_value: Option<f64>,
    #[serde(rename = "max")]
    pub max_value: Option<f64>,
}

/// Price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub amount: f64,
    pub currency: String,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub online: Option<OnlineStatus>,
    pub language: Option<String>,
}

/// Online status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineStatus {
    pub league: Option<String>,
    pub status: Option<String>,
}

/// Stash information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashInfo {
    pub name: String,
    pub x: Option<u32>,
    pub y: Option<u32>,
}

/// Property on an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub values: Vec<Vec<serde_json::Value>>,
    #[serde(rename = "displayMode")]
    pub display_mode: Option<u32>,
}

/// Requirement for an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub name: String,
    pub values: Vec<Vec<serde_json::Value>>,
    #[serde(rename = "displayMode")]
    pub display_mode: Option<u32>,
}

/// Metadata about a scrape run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeMeta {
    pub run_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub source_url: String,
    pub shard: Option<String>,
    pub duration_ms: Option<u64>,
    pub page_count: u32,
    pub items_processed: u32,
    pub items_saved: u32,
    pub errors: Vec<String>,
    pub query_params: HashMap<String, String>,
}

impl ScrapeMeta {
    pub fn new(source_url: String, shard: Option<String>) -> Self {
        Self {
            run_id: uuid::Uuid::new_v4().to_string(),
            started_at: chrono::Utc::now(),
            completed_at: None,
            source_url,
            shard,
            duration_ms: None,
            page_count: 0,
            items_processed: 0,
            items_saved: 0,
            errors: Vec::new(),
            query_params: HashMap::new(),
        }
    }

    pub fn complete(&mut self) {
        let now = chrono::Utc::now();
        self.completed_at = Some(now);
        self.duration_ms = Some((now - self.started_at).num_milliseconds() as u64);
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listing_roundtrip() {
        let listing = Listing {
            id: "test123".to_string(),
            item: Item {
                name: Some("Test Item".to_string()),
                base_type: "Iron Sword".to_string(),
                type_line: "Iron Sword".to_string(),
                item_level: 50,
                rarity: "rare".to_string(),
                category: Some("weapon".to_string()),
                icon: None,
                corrupted: Some(false),
                identified: Some(true),
                properties: None,
                requirements: None,
                modifiers: vec![],
                implicit_mods: None,
                explicit_mods: None,
            },
            price: Some(Price {
                amount: 10.0,
                currency: "chaos".to_string(),
            }),
            account: Account {
                name: "TestAccount".to_string(),
                online: None,
                language: None,
            },
            whisper: None,
            indexed: None,
            stash: None,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&listing).unwrap();

        // Deserialize back
        let deserialized: Listing = serde_json::from_str(&json).unwrap();

        assert_eq!(listing.id, deserialized.id);
        assert_eq!(listing.item.name, deserialized.item.name);
        assert_eq!(listing.item.base_type, deserialized.item.base_type);
    }

    #[test]
    fn test_scrape_meta() {
        let mut meta =
            ScrapeMeta::new("https://example.com".to_string(), Some("trade".to_string()));

        assert_eq!(meta.page_count, 0);
        assert_eq!(meta.items_processed, 0);
        assert!(meta.completed_at.is_none());

        meta.page_count = 5;
        meta.items_processed = 100;
        meta.items_saved = 98;
        meta.add_error("Test error".to_string());

        meta.complete();

        assert!(meta.completed_at.is_some());
        assert!(meta.duration_ms.is_some());
        assert_eq!(meta.errors.len(), 1);
    }

    #[test]
    fn test_optional_fields() {
        // Test that we can deserialize with missing optional fields
        let json = r#"{
            "id": "test",
            "item": {
                "base_type": "Test",
                "type_line": "Test",
                "item_level": 1,
                "rarity": "normal",
                "modifiers": []
            },
            "account": {
                "name": "test"
            }
        }"#;

        let listing: Result<Listing, _> = serde_json::from_str(json);
        assert!(listing.is_ok());

        let listing = listing.unwrap();
        assert_eq!(listing.id, "test");
        assert!(listing.price.is_none());
        assert!(listing.item.name.is_none());
    }
}

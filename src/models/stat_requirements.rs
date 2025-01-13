use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// The core attributes that items and modifiers can depend on
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum CoreAttribute {
    Strength,
    Dexterity,
    Intelligence,
}

// Represents requirements for using an item or modifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatRequirements {
    // Primary attributes this item or modifier is based on
    pub primary_attributes: Vec<CoreAttribute>,
    // Minimum values needed for each attribute
    pub attribute_thresholds: HashMap<CoreAttribute, u32>,
}

impl StatRequirements {
    pub fn new() -> Self {
        Self {
            primary_attributes: Vec::new(),
            attribute_thresholds: HashMap::new(),
        }
    }

    // Helper to add a requirement with a threshold
    pub fn add_requirement(&mut self, attr: CoreAttribute, threshold: u32) {
        self.primary_attributes.push(attr.clone());
        self.attribute_thresholds.insert(attr, threshold);
    }

    // Check if this is a pure single-stat requirement
    pub fn is_pure_requirement(&self) -> bool {
        self.primary_attributes.len() == 1
    }

    // Check if this is a hybrid requirement (multiple stats)
    pub fn is_hybrid_requirement(&self) -> bool {
        self.primary_attributes.len() > 1
    }

    // Get the dominant attribute (highest threshold)
    pub fn get_dominant_attribute(&self) -> Option<&CoreAttribute> {
        self.attribute_thresholds
            .iter()
            .max_by_key(|&(_, threshold)| threshold)
            .map(|(attr, _)| attr)
    }
}

// Extend your existing ItemType to include stat requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemBaseType {
    pub name: String,
    pub category: super::item_type::ItemCategory,
    pub stat_requirements: StatRequirements,
    pub implicit_modifiers: Vec<String>,
    pub base_level: u32,
    // Tags help identify special properties of bases
    pub tags: Vec<String>,
}

impl ItemBaseType {
    pub fn new(name: String, category: super::item_type::ItemCategory) -> Self {
        Self {
            name,
            category,
            stat_requirements: StatRequirements::new(),
            implicit_modifiers: Vec::new(),
            base_level: 1,
            tags: Vec::new(),
        }
    }

    // Helper to quickly identify the main attribute requirements
    pub fn get_attribute_profile(&self) -> String {
        let attrs: Vec<_> = self.stat_requirements.primary_attributes
            .iter()
            .map(|attr| match attr {
                CoreAttribute::Strength => "Str",
                CoreAttribute::Dexterity => "Dex",
                CoreAttribute::Intelligence => "Int",
            })
            .collect();
        
        attrs.join("/")
    }
}

// Database to manage item bases
pub struct ItemBaseDatabase {
    bases: HashMap<String, ItemBaseType>,
}

impl ItemBaseDatabase {
    pub fn new() -> Self {
        Self {
            bases: HashMap::new(),
        }
    }

    pub fn add_base(&mut self, base: ItemBaseType) {
        self.bases.insert(base.name.clone(), base);
    }

    pub fn get_base(&self, name: &str) -> Option<&ItemBaseType> {
        self.bases.get(name)
    }

    // Get all bases with specific attribute requirements
    pub fn get_bases_by_attributes(&self, attrs: &[CoreAttribute]) -> Vec<&ItemBaseType> {
        self.bases
            .values()
            .filter(|base| {
                base.stat_requirements
                    .primary_attributes
                    .iter()
                    .all(|attr| attrs.contains(attr))
            })
            .collect()
    }

    // Save the database to a JSON file
    pub async fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self.bases)?;
        tokio::fs::write(path, json).await
    }

    // Load the database from a JSON file
    pub async fn load_from_file(&mut self, path: &str) -> std::io::Result<()> {
        let content = tokio::fs::read_to_string(path).await?;
        self.bases = serde_json::from_str(&content)?;
        Ok(())
    }
}

// Extend your existing ItemModifier to include stat dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierStatRequirements {
    pub requirements: StatRequirements,
    pub scaling_attribute: Option<CoreAttribute>, // Which attribute this modifier scales with
    pub is_hybrid: bool, // Does this modifier benefit from multiple attributes?
}

// Add tests to verify the functionality
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::item_type::ItemCategory;

    #[test]
    fn test_item_base_type_creation() {
        let mut base = ItemBaseType::new(
            "Assassin's Garb".to_string(),
            ItemCategory::Armour,
        );

        base.stat_requirements.add_requirement(CoreAttribute::Dexterity, 100);
        assert_eq!(base.get_attribute_profile(), "Dex");
    }

    #[test]
    fn test_hybrid_requirements() {
        let mut reqs = StatRequirements::new();
        reqs.add_requirement(CoreAttribute::Strength, 50);
        reqs.add_requirement(CoreAttribute::Intelligence, 50);

        assert!(reqs.is_hybrid_requirement());
        assert!(!reqs.is_pure_requirement());
    }
}
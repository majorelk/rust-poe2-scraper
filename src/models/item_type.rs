use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemCategory {
    Weapon,
    Armour,
    Accessory,
    Flask,
    Gem,
    Currency,
    DivinationCard,
    Map,
    Other,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemRarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemType {
    pub category: ItemCategory,
    pub base_type: String,
    pub rarity: ItemRarity,
    pub required_level: Option<u32>,
}

impl ItemType {
    pub fn new(category: ItemCategory, base_type: String, rarity: ItemRarity) -> Self {
        Self {
            category,
            base_type,
            rarity,
            required_level: None,
        }
    }

    pub fn with_level(mut self, level: u32) -> Self {
        self.required_level = Some(level);
        self
    }

    pub fn is_equipment(&self) -> bool {
        matches!(self.category, 
            ItemCategory::Weapon | 
            ItemCategory::Armour | 
            ItemCategory::Accessory
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_type_creation() {
        let item = ItemType::new(
            ItemCategory::Weapon,
            "Siege Axe".to_string(),
            ItemRarity::Unique
        ).with_level(68);

        assert_eq!(item.category, ItemCategory::Weapon);
        assert_eq!(item.base_type, "Siege Axe");
        assert_eq!(item.rarity, ItemRarity::Unique);
        assert_eq!(item.required_level, Some(68));
        assert!(item.is_equipment());
    }
}
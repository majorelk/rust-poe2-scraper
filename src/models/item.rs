use serde::{Deserialize, Serialize};
use super::item_type::{ItemType, ItemRarity};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemModifier {
    pub name: String,
    pub tier: Option<i32>,
    pub values: Vec<f64>,
    pub is_crafted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemPrice {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub item_type: ItemType,
    pub name: Option<String>,
    pub modifiers: Vec<ItemModifier>,
    pub price: Option<ItemPrice>,
    pub stats: HashMap<String, f64>,
    pub corrupted: bool,
}

impl Item {
    pub fn new(id: String, item_type: ItemType) -> Self {
        Self {
            id,
            item_type,
            name: None,
            modifiers: Vec::new(),
            price: None,
            stats: HashMap::new(),
            corrupted: false,
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn add_modifier(&mut self, modifier: ItemModifier) {
        self.modifiers.push(modifier);
    }

    pub fn set_price(&mut self, amount: f64, currency: String) {
        self.price = Some(ItemPrice { amount, currency });
    }

    pub fn is_unique(&self) -> bool {
        self.item_type.rarity == ItemRarity::Unique
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::item_type::ItemCategory;

    #[test]
    fn test_item_creation_and_modification() {
        let item_type = ItemType::new(
            ItemCategory::Weapon,
            "Siege Axe".to_string(),
            ItemRarity::Unique
        );

        let mut item = Item::new("test123".to_string(), item_type)
            .with_name("Soul Taker".to_string());

        assert!(item.is_unique());
        assert_eq!(item.name, Some("Soul Taker".to_string()));

        item.set_price(50.0, "chaos".to_string());
        assert!(item.price.is_some());
    }
}
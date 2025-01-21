use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::item_type::{ItemType, ItemRarity};
use super::stats_requirements::{
    CoreAttribute,
    StatRequirements,
    ModifierStatRequirements,
};
use super::poe_item::ItemResponse;
use crate::ItemCategory;
use crate::errors::{ScraperError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemModifier {
    pub name: String,
    pub tier: Option<i32>,
    pub values: Vec<f64>,
    pub is_crafted: bool,
    pub stat_requirements: Option<ModifierStatRequirements>,
    pub attribute_scaling: Option<HashMap<CoreAttribute, f64>>,
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
    pub stat_requirements: StatRequirements,
    pub attribute_values: HashMap<CoreAttribute, u32>,
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
            stat_requirements: StatRequirements::new(),
            attribute_values: HashMap::new(),
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

    pub fn can_have_modifier(&self, modifier: &ItemModifier) -> bool {
        if let Some(req) = &modifier.stat_requirements {
            for (attr, threshold) in &req.requirements.attribute_thresholds {
                if let Some(value) = self.attribute_values.get(attr) {
                    if value < threshold {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }
    
    pub fn calculate_modifier_value(&self, modifier: &ItemModifier) -> Vec<f64> {
        let mut scaled_values = modifier.values.clone();
        
        if let Some(scaling) = &modifier.attribute_scaling {
            let scaling_factor: f64 = scaling.iter()
                .map(|(attr, factor)| {
                    let attr_value = self.attribute_values.get(attr).unwrap_or(&0);
                    *factor * (*attr_value as f64 / 100.0)
                })
                .sum::<f64>();
                
            scaled_values.iter_mut()
                .for_each(|value| *value *= 1.0 + scaling_factor);
        }
        
        scaled_values
    }
}

impl TryFrom<ItemResponse> for Item {
    type Error = ScraperError;

    fn try_from(response: ItemResponse) -> Result<Self> {
        let item_type = ItemType::new(
            ItemCategory::Other,
            response.item.base_type,
            match response.item.rarity.as_str() {
                "Unique" => ItemRarity::Unique,
                "Rare" => ItemRarity::Rare,
                "Magic" => ItemRarity::Magic,
                _ => ItemRarity::Normal,
            }
        );

        // Convert explicit mods with error handling
        let modifiers = response.item.explicit_mods.iter()
            .zip(response.item.extended.mods.explicit.iter())
            .map(|(text, mod_info)| {
                let values = mod_info.magnitudes.iter()
                    .map(|m| m.min.parse::<f64>())
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(|e| ScraperError::ConversionError(
                        format!("Failed to parse modifier value: {}", e)
                    ))?;

                Ok(ItemModifier {
                    name: text.clone(),
                    tier: mod_info.tier.parse().ok(),
                    values,
                    is_crafted: false,
                    stat_requirements: None,
                    attribute_scaling: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Process requirements with error handling
        let mut attribute_values = HashMap::new();
        let mut stat_requirements = StatRequirements::new();

        for req in &response.item.requirements {
            if let Some(attr) = match req.name.as_str() {
                "Str" | "Strength" => Some(CoreAttribute::Strength),
                "Dex" | "Dexterity" => Some(CoreAttribute::Dexterity),
                "Int" | "Intelligence" => Some(CoreAttribute::Intelligence),
                _ => None
            } {
                if let Some((val_str, _)) = req.values.first() {
                    let value = val_str.parse::<u32>()
                        .map_err(|e| ScraperError::ConversionError(
                            format!("Failed to parse attribute value: {}", e)
                        ))?;
                    
                    attribute_values.insert(attr.clone(), value);
                    stat_requirements.add_requirement(attr, value);
                }
            }
        }

        Ok(Item {
            id: response.id,
            item_type,
            name: Some(response.item.type_line),
            modifiers,
            price: Some(ItemPrice {
                amount: response.listing.price.amount,
                currency: response.listing.price.currency,
            }),
            stats: HashMap::new(),
            corrupted: false,
            stat_requirements,
            attribute_values,
        })
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
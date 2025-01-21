use crate::models::{
    ItemResponse,
    ModifierStats,
    ModInfo
};
use std::collections::HashMap;

pub struct ModifierAnalyzer {
    stats: HashMap<String, ModifierStats>,
    value_ranges: Vec<f64>,
    min_price: Option<f64>,
    max_price: Option<f64>,
}

impl ModifierAnalyzer {
    pub fn new(value_ranges: Vec<f64>) -> Self {
        Self {
            stats: HashMap::new(),
            value_ranges,
            min_price: None,
            max_price: None,
        }
    }

    pub fn process_item(&mut self, item: &ItemResponse) {
        // Price is not an Option in the listing
        let price = &item.listing.price;
        // The explicit mods are directly a Vec, not an Option
        for mod_info in &item.item.extended.mods.explicit {
            self.process_modifier(mod_info, price.amount);
        }
    }

    fn process_modifier(&mut self, mod_info: &ModInfo, price: f64) {
        let stats = self.stats
            .entry(mod_info.name.clone())
            .or_insert_with(|| ModifierStats::new(mod_info.name.clone()));

        // Get the first magnitude value if it exists
        if let Some(magnitude) = mod_info.magnitudes.first() {
            if let Ok(value) = magnitude.min.parse::<f64>() {
                stats.add_data_point(value, price);
            }
        }
    }

    pub fn get_stats(&self, modifier_name: &str) -> Option<&ModifierStats> {
        self.stats.get(modifier_name)
    }

    pub fn set_price_range(&mut self, min: f64, max: f64) {
        self.min_price = Some(min);
        self.max_price = Some(max);
    }
}

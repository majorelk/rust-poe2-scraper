use crate::models::{Item, ItemModifier, ModifierStats, StatisticalMeasures};
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

    pub fn process_item(&mut self, item: &Item) {
        if let Some(price) = item.price.as_ref() {
            for modifier in &item.modifiers {
                self.process_modifier(modifier, price.amount);
            }
        }
    }

    fn process_modifier(&mut self, modifier: &ItemModifier, price: f64) {
        let stats = self.stats
            .entry(modifier.name.clone())
            .or_insert_with(|| ModifierStats::new(modifier.name.clone()));

        for value in &modifier.values {
            stats.add_data_point(*value, price);
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

use std::collections::{HashMap, HashSet};
use crate::data::item_base_data_loader::BaseDataLoader;
use serde::{Serialize, Deserialize};

use crate::models::{
    CoreAttribute,
    StatRequirements,
    Item,
    ItemModifier,
    ItemResponse,
    ModInfo
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCorrelation {
    pub attribute: String,
    pub occurrence_count: u32,
    pub average_threshold: f64,
    pub modifier_correlations: HashMap<String, f64>,
}

#[derive(Debug)]
pub struct StatAnalyzer {
    // Track which modifiers appear on items with specific attribute requirements
    modifier_attribute_occurrences: HashMap<String, HashMap<String, u32>>,
    // Track the average attribute thresholds for each modifier
    modifier_thresholds: HashMap<String, HashMap<String, Vec<u32>>>,
    // Track which modifiers commonly appear together on items with specific attributes
    modifier_correlations: HashMap<String, HashMap<String, u32>>,
    // Keep track of total items processed for calculating percentages
    total_items: u32,
}

impl StatAnalyzer {
    pub fn new() -> Self {
        Self {
            modifier_attribute_occurrences: HashMap::new(),
            modifier_thresholds: HashMap::new(),
            modifier_correlations: HashMap::new(),
            total_items: 0,
        }
    }

    pub fn process_item(&mut self, item: &ItemResponse) {
        self.total_items += 1;

        // Get stat requirements from the ItemResponse
        let stat_requirements = item.get_stat_requirements();
        let item_attributes: HashSet<_> = stat_requirements.keys().collect();

        // The explicit mods are directly a Vec
        for mod_info in &item.item.extended.mods.explicit {
            self.update_modifier_stats(
                mod_info,
                &item_attributes,
                &stat_requirements
            );
        }

        // Update correlations between mods
        self.update_modifier_correlations(&item.item.extended.mods.explicit);
    }

    fn update_modifier_stats(
        &mut self,
        mod_info: &ModInfo,
        item_attributes: &HashSet<&String>,
        stat_requirements: &HashMap<String, u32>
    ) {
        let mod_occurrences = self.modifier_attribute_occurrences
            .entry(mod_info.name.clone())
            .or_default();
        
        let mod_thresholds = self.modifier_thresholds
            .entry(mod_info.name.clone())
            .or_default();

        // Update occurrence counts and thresholds for each attribute
        for attr in item_attributes {
            *mod_occurrences.entry((*attr).clone()).or_default() += 1;
            
            if let Some(&value) = stat_requirements.get(*attr) {
                mod_thresholds
                    .entry((*attr).clone())
                    .or_default()
                    .push(value);
            }
        }
    }

    fn update_modifier_correlations(&mut self, mods: &[ModInfo]) {
        // Update correlations between all pairs of modifiers
        for (i, mod1) in mods.iter().enumerate() {
            for mod2 in mods.iter().skip(i + 1) {
                let correlations = self.modifier_correlations
                    .entry(mod1.name.clone())
                    .or_default();
                
                *correlations.entry(mod2.name.clone()).or_default() += 1;

                // Also update the reverse correlation
                let reverse_correlations = self.modifier_correlations
                    .entry(mod2.name.clone())
                    .or_default();
                
                *reverse_correlations.entry(mod1.name.clone()).or_default() += 1;
            }
        }
    }

    pub fn analyze_attribute_correlations(&self) -> HashMap<String, AttributeCorrelation> {
        let mut correlations = HashMap::new();

        for (modifier_name, attr_occurrences) in &self.modifier_attribute_occurrences {
            for (attr, &count) in attr_occurrences {
                let correlation = correlations
                    .entry(attr.clone())
                    .or_insert_with(|| AttributeCorrelation {
                        attribute: attr.clone(),
                        occurrence_count: 0,
                        average_threshold: 0.0,
                        modifier_correlations: HashMap::new(),
                    });

                correlation.occurrence_count += count;

                // Calculate correlation strength (simplified version)
                let correlation_strength = count as f64 / self.total_items as f64;
                correlation.modifier_correlations.insert(
                    modifier_name.clone(),
                    correlation_strength
                );
            }
        }

        // Calculate average thresholds
        for (attr, correlation) in correlations.iter_mut() {
            let mut total_threshold = 0.0;
            let mut threshold_count = 0;

            for thresholds in self.modifier_thresholds.values() {
                if let Some(values) = thresholds.get(attr) {
                    total_threshold += values.iter().sum::<u32>() as f64;
                    threshold_count += values.len();
                }
            }

            if threshold_count > 0 {
                correlation.average_threshold = total_threshold / threshold_count as f64;
            }
        }

        correlations
    }

    pub fn get_common_modifier_pairs(&self, minimum_correlation: f64) -> Vec<(String, String, f64)> {
        let mut common_pairs = Vec::new();

        for (mod1, correlations) in &self.modifier_correlations {
            for (mod2, &count) in correlations {
                let correlation_strength = count as f64 / self.total_items as f64;
                
                if correlation_strength >= minimum_correlation {
                    common_pairs.push((
                        mod1.clone(),
                        mod2.clone(),
                        correlation_strength
                    ));
                }
            }
        }

        // Sort by correlation strength
        common_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        common_pairs
    }

    pub fn generate_attribute_report(&self) -> serde_json::Value {
        let correlations = self.analyze_attribute_correlations();
        let common_pairs = self.get_common_modifier_pairs(0.1); // 10% correlation threshold

        serde_json::json!({
            "total_items_analyzed": self.total_items,
            "attribute_correlations": correlations,
            "common_modifier_pairs": common_pairs,
            "analysis_summary": {
                "strongest_attribute": correlations.iter()
                    .max_by_key(|(_, c)| c.occurrence_count)
                    .map(|(attr, _)| attr),
                "most_common_threshold": correlations.iter()
                    .map(|(_, c)| c.average_threshold.round() as u32)
                    .max()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::item_type::{ItemType, ItemCategory, ItemRarity};

    #[test]
    fn test_stat_analyzer_basic_functionality() {
        let mut analyzer = StatAnalyzer::new();
        
        // Create a test item with some modifiers
        let mut item = Item::new(
            "test_item".to_string(),
            ItemType::new(
                ItemCategory::Armour,
                "Test Base".to_string(),
                ItemRarity::Rare
            )
        );

        // Add stat requirements
        item.stat_requirements.add_requirement(CoreAttribute::Strength, 100);
        item.attribute_values.insert(CoreAttribute::Strength, 100);

        // Add some modifiers
        let modifier = ItemModifier {
            name: "Test Modifier".to_string(),
            tier: Some(1),
            values: vec![10.0],
            is_crafted: false,
            stat_requirements: None,
            attribute_scaling: None,
        };

        item.modifiers.push(modifier);

        // Process the item
        analyzer.process_item(&item);

        // Verify analysis
        let report = analyzer.generate_attribute_report();
        assert_eq!(report["total_items_analyzed"], 1);
    }
}
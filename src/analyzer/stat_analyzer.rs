use std::collections::{HashMap, HashSet};
use crate::data::item_base_data_loader::BaseDataLoader;
use serde::{Serialize, Deserialize};
use serde_json::json;
use crate::models::{
    CoreAttribute,
    StatRequirements,
    Item,
    ItemModifier,
    ItemResponse,
    ModInfo,
    CleanedItem
};

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum StatRequirementType {
    Single(String),
    Dual(String, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCorrelation {
    pub attribute: String,
    pub occurrence_count: u32,
    pub average_threshold: f64,
    pub modifier_correlations: HashMap<String, f64>,
}

#[derive(Debug)]
pub struct StatAnalyzer {
    modifier_attribute_occurrences: HashMap<String, HashMap<String, u32>>,
    modifier_thresholds: HashMap<String, HashMap<String, Vec<u32>>>,
    modifier_correlations: HashMap<String, HashMap<String, u32>>,
    total_items: u32,
    requirement_distributions: HashMap<StatRequirementType, Vec<(u32, u32)>>,
}

impl StatAnalyzer {
    pub fn new() -> Self {
        Self {
            modifier_attribute_occurrences: HashMap::new(),
            modifier_thresholds: HashMap::new(),
            modifier_correlations: HashMap::new(),
            total_items: 0,
            requirement_distributions: HashMap::new(),
        }
    }

    pub fn process_item(&mut self, item: &ItemResponse) {
        self.total_items += 1;

        self.process_requirements(item);

        // Get stat requirements from the ItemResponse
        let stat_requirements = item.get_stat_requirements();
        let item_attributes: HashSet<_> = stat_requirements.keys().collect();

        for mod_info in &item.item.extended.mods.explicit {
            self.update_modifier_stats(
                mod_info,
                &item_attributes,
                &stat_requirements
            );
        }

        self.update_modifier_correlations(&item.item.extended.mods.explicit);
    }

    pub fn process_cleaned_item(&mut self, item: &CleanedItem) {
        self.total_items += 1;

        // Process requirements using cleaned data
        self.process_cleaned_requirements(item);

        // Get stat requirements from cleaned item
        let stat_requirements = item.get_stat_requirements();
        let item_attributes: HashSet<_> = stat_requirements.keys().collect();

        for mod_info in &item.mod_info.explicit {
            self.update_modifier_stats(
                mod_info,
                &item_attributes,
                &stat_requirements
            );
        }

        self.update_modifier_correlations(&item.mod_info.explicit);
    }

    fn process_requirements(&mut self, item: &ItemResponse) {
        let mut item_reqs = Vec::new();
        
        // Collect all attribute requirements
        for req in &item.item.requirements {
            match req.name.as_str() {
                "[Dexterity|Dex]" | "[Strength|Str]" | "[Intelligence|Int]" => {
                    if let Some(value) = req.values.first() {
                        if let Ok(val) = value.0.parse::<u32>() {
                            item_reqs.push((req.name.clone(), val));
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Sort requirements for consistent ordering
        item_reqs.sort_by(|a, b| a.0.cmp(&b.0));

        // Create requirement type and store values
        match item_reqs.len() {
            1 => {
                let req_type = StatRequirementType::Single(item_reqs[0].0.clone());
                self.requirement_distributions.entry(req_type)
                    .or_insert_with(Vec::new)
                    .push((item_reqs[0].1, 0));
            }
            2 => {
                let req_type = StatRequirementType::Dual(
                    item_reqs[0].0.clone(),
                    item_reqs[1].0.clone()
                );
                self.requirement_distributions.entry(req_type)
                    .or_insert_with(Vec::new)
                    .push((item_reqs[0].1, item_reqs[1].1));
            }
            _ => {}
        }
    }

    fn process_cleaned_requirements(&mut self, item: &CleanedItem) {
        let mut item_reqs = Vec::new();
        
        // Collect all attribute requirements from cleaned item
        for req in &item.requirements {
            match req.name.as_str() {
                "[Dexterity|Dex]" | "[Strength|Str]" | "[Intelligence|Int]" => {
                    if let Some((value, _)) = req.values.first() {
                        if let Ok(val) = value.parse::<u32>() {
                            item_reqs.push((req.name.clone(), val));
                        }
                    }
                }
                _ => {}
            }
        }
        
        // Sort requirements for consistent ordering (same as original)
        item_reqs.sort_by(|a, b| a.0.cmp(&b.0));

        // Create requirement type and store values (same logic as original)
        match item_reqs.len() {
            1 => {
                let req_type = StatRequirementType::Single(item_reqs[0].0.clone());
                self.requirement_distributions.entry(req_type)
                    .or_insert_with(Vec::new)
                    .push((item_reqs[0].1, 0));
            }
            2 => {
                let req_type = StatRequirementType::Dual(
                    item_reqs[0].0.clone(),
                    item_reqs[1].0.clone()
                );
                self.requirement_distributions.entry(req_type)
                    .or_insert_with(Vec::new)
                    .push((item_reqs[0].1, item_reqs[1].1));
            }
            _ => {}
        }
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

    pub fn get_requirement_statistics(&self) -> serde_json::Value {
        let mut stats = serde_json::json!({
            "single_stat_counts": {},
            "dual_stat_counts": {},
            "average_requirements": {},
        });

        for (req_type, values) in &self.requirement_distributions {
            match req_type {
                StatRequirementType::Single(stat) => {
                    let avg = values.iter()
                        .map(|(v, _)| v)
                        .sum::<u32>() as f64 / values.len() as f64;
                    
                    stats["single_stat_counts"][stat.clone()] = json!(values.len());
                    stats["average_requirements"][stat] = json!(avg);
                }
                StatRequirementType::Dual(stat1, stat2) => {
                    let key = format!("{}-{}", stat1, stat2);
                    let avg1 = values.iter().map(|(v1, _)| v1).sum::<u32>() as f64 / values.len() as f64;
                    let avg2 = values.iter().map(|(_, v2)| v2).sum::<u32>() as f64 / values.len() as f64;
                    
                    stats["dual_stat_counts"][key.clone()] = json!(values.len());
                    stats["average_requirements"][format!("{}-1", key)] = json!(avg1);
                    stats["average_requirements"][format!("{}-2", key)] = json!(avg2);
                }
            }
        }

        stats
    }

    pub fn generate_attribute_report(&self) -> serde_json::Value {
        let correlations = self.analyze_attribute_correlations();
        let common_pairs = self.get_common_modifier_pairs(0.1); // 10% correlation threshold

        serde_json::json!({
            "total_items_analyzed": self.total_items,
            "attribute_correlations": correlations,
            "common_modifier_pairs": common_pairs,
            "requirement_statistics": self.get_requirement_statistics(),
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

    // Add new test for cleaned item processing
    #[test]
    fn test_stat_analyzer_cleaned_item() {
        let mut analyzer = StatAnalyzer::new();

        // Create a cleaned item with test data
        let cleaned_item = CleanedItem {
            base_type: "Test Base".to_string(),
            name: "Test Item".to_string(),
            explicit_mods: vec![
                "+100 to maximum Life".to_string(),
                "+50 to Strength".to_string(),
            ],
            item_level: 75,
            properties: vec![],
            requirements: vec![
                ItemRequirement {
                    name: "[Strength|Str]".to_string(),
                    values: vec![("100".to_string(), 0)],
                    display_mode: 1,
                }
            ],
            mod_info: ModInfo {
                explicit: vec![
                    ExplicitMod {
                        level: 1,
                        magnitudes: vec![
                            Magnitude {
                                hash: "test_hash".to_string(),
                                max: "100".to_string(),
                                min: "90".to_string(),
                            }
                        ],
                        name: "Test Modifier".to_string(),
                        tier: "1".to_string(),
                    }
                ],
            },
            mod_hashes: HashMap::new(),
        };

        // Process the cleaned item
        analyzer.process_cleaned_item(&cleaned_item);

        // Verify analysis
        let report = analyzer.generate_attribute_report();
        assert_eq!(report["total_items_analyzed"], 1);

        // Check requirement processing
        let req_stats = analyzer.get_requirement_statistics();
        assert!(req_stats["single_stat_counts"].get("[Strength|Str]").is_some());
    }

    // Add test to compare both implementations
    #[test]
    fn test_compare_implementations() {
        let mut analyzer_original = StatAnalyzer::new();
        let mut analyzer_cleaned = StatAnalyzer::new();

        // Create test data for both types
        let item_response = create_test_item_response();
        let cleaned_item = CleanedItem::from_response(&item_response);

        // Process with both implementations
        analyzer_original.process_item(&item_response);
        analyzer_cleaned.process_cleaned_item(&cleaned_item);

        // Compare results
        let report_original = analyzer_original.generate_attribute_report();
        let report_cleaned = analyzer_cleaned.generate_attribute_report();

        assert_eq!(
            report_original["total_items_analyzed"],
            report_cleaned["total_items_analyzed"]
        );
        
        // Compare requirement statistics
        let stats_original = analyzer_original.get_requirement_statistics();
        let stats_cleaned = analyzer_cleaned.get_requirement_statistics();
        
        assert_eq!(
            stats_original["single_stat_counts"],
            stats_cleaned["single_stat_counts"]
        );
    }

    // Helper function to create test item response
    fn create_test_item_response() -> ItemResponse {
        // Create a minimal ItemResponse for testing
        // You'll need to implement this based on your ItemResponse structure
        todo!("Implement test ItemResponse creation")
    }
}
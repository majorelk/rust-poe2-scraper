use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueRange {
    pub min: f64,
    pub max: f64,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalMeasures {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierStats {
    pub name: String,
    pub total_occurrences: u32,
    pub value_ranges: Vec<ValueRange>,
    pub price_points: Vec<(f64, f64)>, // (value, price) pairs
    pub measures: StatisticalMeasures,
}

impl ModifierStats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            total_occurrences: 0,
            value_ranges: Vec::new(),
            price_points: Vec::new(),
            measures: StatisticalMeasures {
                mean: 0.0,
                median: 0.0,
                std_dev: 0.0,
                min: 0.0,
                max: 0.0,
            },
        }
    }

    pub fn add_data_point(&mut self, value: f64, price: f64) {
        self.total_occurrences += 1;
        self.price_points.push((value, price));
        self.update_measures();
    }

    fn update_measures(&mut self) {
        if self.price_points.is_empty() {
            return;
        }

        let values: Vec<f64> = self.price_points.iter().map(|(v, _)| *v).collect();
        self.measures.min = *values.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        self.measures.max = *values.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap();
        self.measures.mean = values.iter().sum::<f64>() / values.len() as f64;
        
        // Calculate median
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = sorted.len() / 2;
        self.measures.median = if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        };

        // Calculate standard deviation
        let variance = values.iter()
            .map(|v| {
                let diff = v - self.measures.mean;
                diff * diff
            })
            .sum::<f64>() / values.len() as f64;
        self.measures.std_dev = variance.sqrt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modifier_stats_calculations() {
        let mut stats = ModifierStats::new("test_mod".to_string());
        stats.add_data_point(10.0, 100.0);
        stats.add_data_point(20.0, 200.0);
        stats.add_data_point(30.0, 300.0);

        assert_eq!(stats.total_occurrences, 3);
        assert_eq!(stats.measures.mean, 20.0);
        assert_eq!(stats.measures.median, 20.0);
        assert_eq!(stats.measures.min, 10.0);
        assert_eq!(stats.measures.max, 30.0);
    }
}
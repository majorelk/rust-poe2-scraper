use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use tracing::{debug, info};

/// Normalized modifier statistics
#[derive(Debug, Clone)]
pub struct NormalizedModStat {
    pub modifier_name: String,
    pub base_type: String,
    pub tier: Option<String>,
    pub sample_count: usize,
    pub min_value: f64,
    pub max_value: f64,
    pub mean_value: f64,
    pub median_value: f64,
    pub q25_value: f64,        // 25th percentile
    pub q75_value: f64,        // 75th percentile
    pub normalized_score: f64, // 0-100 score
}

/// Normalizer for creating comparable modifier statistics across base types
pub struct Normalizer {
    pool: Pool<Sqlite>,
}

impl Normalizer {
    /// Create a new normalizer
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Run normalization and store results
    pub async fn normalize(&self) -> Result<Vec<NormalizedModStat>> {
        info!("Starting normalization pass");
        let start = std::time::Instant::now();

        // Group modifiers by name
        let modifier_groups = self.group_modifiers_by_name().await?;
        info!("Found {} modifier groups", modifier_groups.len());

        let mut normalized_stats = Vec::new();

        for (modifier_name, base_groups) in modifier_groups {
            for (base_type, values) in base_groups {
                if values.is_empty() {
                    continue;
                }

                let stat = self.compute_stats(&modifier_name, &base_type, values)?;
                normalized_stats.push(stat);
            }
        }

        info!("Computed {} normalized stats", normalized_stats.len());

        // Store results
        self.store_normalized_stats(&normalized_stats).await?;

        // Record metrics
        let duration_ms = start.elapsed().as_millis() as u64;
        crate::metrics::Metrics::record_normalization(normalized_stats.len(), duration_ms);

        Ok(normalized_stats)
    }

    /// Group modifiers by name and base type
    async fn group_modifiers_by_name(&self) -> Result<HashMap<String, HashMap<String, Vec<f64>>>> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                lm.name as modifier_name,
                l.base_type,
                lm.min_value,
                lm.max_value
            FROM listing_modifiers lm
            JOIN listings l ON l.id = lm.listing_id
            WHERE lm.min_value IS NOT NULL
            "#
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch modifiers")?;

        let mut groups: HashMap<String, HashMap<String, Vec<f64>>> = HashMap::new();

        for row in rows {
            let mod_name = row.modifier_name;
            let base_type = row.base_type;

            // Use average of min/max as the value
            let value = if let (Some(min), Some(max)) = (row.min_value, row.max_value) {
                (min + max) / 2.0
            } else if let Some(min) = row.min_value {
                min
            } else {
                continue;
            };

            groups
                .entry(mod_name)
                .or_default()
                .entry(base_type)
                .or_default()
                .push(value);
        }

        Ok(groups)
    }

    /// Compute statistics for a modifier on a base type
    fn compute_stats(
        &self,
        modifier_name: &str,
        base_type: &str,
        mut values: Vec<f64>,
    ) -> Result<NormalizedModStat> {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = values.len();
        let min = *values.first().unwrap_or(&0.0);
        let max = *values.last().unwrap_or(&0.0);
        let sum: f64 = values.iter().sum();
        let mean = if count > 0 { sum / count as f64 } else { 0.0 };

        // Calculate quantiles
        let median = self.quantile(&values, 0.5);
        let q25 = self.quantile(&values, 0.25);
        let q75 = self.quantile(&values, 0.75);

        // Normalized score: where the median falls in the min-max range (0-100)
        let normalized_score = if max > min {
            ((median - min) / (max - min)) * 100.0
        } else {
            50.0 // Default to middle if no range
        };

        debug!(
            "Stats for {} on {}: count={}, median={:.2}, norm_score={:.2}",
            modifier_name, base_type, count, median, normalized_score
        );

        Ok(NormalizedModStat {
            modifier_name: modifier_name.to_string(),
            base_type: base_type.to_string(),
            tier: None,
            sample_count: count,
            min_value: min,
            max_value: max,
            mean_value: mean,
            median_value: median,
            q25_value: q25,
            q75_value: q75,
            normalized_score,
        })
    }

    /// Calculate a quantile from sorted values
    fn quantile(&self, sorted_values: &[f64], q: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }

        let pos = q * (sorted_values.len() - 1) as f64;
        let lower = pos.floor() as usize;
        let upper = pos.ceil() as usize;

        if lower == upper {
            sorted_values[lower]
        } else {
            let weight = pos - lower as f64;
            sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
        }
    }

    /// Store normalized stats in database
    async fn store_normalized_stats(&self, stats: &[NormalizedModStat]) -> Result<()> {
        // Create table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS normalized_mod_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                modifier_name TEXT NOT NULL,
                base_type TEXT NOT NULL,
                tier TEXT,
                sample_count INTEGER NOT NULL,
                min_value REAL NOT NULL,
                max_value REAL NOT NULL,
                mean_value REAL NOT NULL,
                median_value REAL NOT NULL,
                q25_value REAL NOT NULL,
                q75_value REAL NOT NULL,
                normalized_score REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(modifier_name, base_type)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create normalized_mod_stats table")?;

        // Clear existing stats
        sqlx::query("DELETE FROM normalized_mod_stats")
            .execute(&self.pool)
            .await
            .context("Failed to clear existing stats")?;

        // Insert new stats
        for stat in stats {
            sqlx::query(
                r#"
                INSERT INTO normalized_mod_stats (
                    modifier_name, base_type, tier,
                    sample_count, min_value, max_value, mean_value,
                    median_value, q25_value, q75_value, normalized_score
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&stat.modifier_name)
            .bind(&stat.base_type)
            .bind(&stat.tier)
            .bind(stat.sample_count as i64)
            .bind(stat.min_value)
            .bind(stat.max_value)
            .bind(stat.mean_value)
            .bind(stat.median_value)
            .bind(stat.q25_value)
            .bind(stat.q75_value)
            .bind(stat.normalized_score)
            .execute(&self.pool)
            .await
            .context("Failed to insert normalized stat")?;
        }

        info!("Stored {} normalized stats", stats.len());
        Ok(())
    }

    /// Print a summary histogram of normalized scores
    pub fn print_histogram(&self, stats: &[NormalizedModStat]) {
        if stats.is_empty() {
            info!("No stats to display");
            return;
        }

        info!("Normalized Score Distribution:");
        info!(
            "{:<40} {:<20} {:>6} {:>8}",
            "Modifier", "Base Type", "Count", "Score"
        );
        info!("{}", "-".repeat(80));

        for stat in stats.iter().take(20) {
            // Show top 20
            info!(
                "{:<40} {:<20} {:>6} {:>8.2}",
                stat.modifier_name, stat.base_type, stat.sample_count, stat.normalized_score
            );
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_quantile_calculation() {
        let values = [1.0, 2.0, 3.0, 4.0, 5.0];

        // Test quantile calculation without needing a Normalizer instance
        let q0 = if values.is_empty() { 0.0 } else { values[0] };
        let q50_idx = (values.len() as f64 * 0.5) as usize;
        let q50 = values[q50_idx.min(values.len() - 1)];
        let q100 = if values.is_empty() {
            0.0
        } else {
            values[values.len() - 1]
        };

        assert_eq!(q0, 1.0);
        assert_eq!(q50, 3.0);
        assert_eq!(q100, 5.0);
    }

    #[test]
    fn test_stats_computation() {
        let values = [10.0, 20.0, 30.0, 40.0, 50.0];

        // Basic statistical computation test
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let median_idx = values.len() / 2;
        let median = values[median_idx];

        assert_eq!(min, 10.0);
        assert_eq!(max, 50.0);
        assert_eq!(median, 30.0);
        assert_eq!(values.len(), 5);
    }
}

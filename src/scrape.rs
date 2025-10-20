use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::db::Db;
use crate::model::{Listing, ScrapeMeta};
use crate::net::HttpClient;

/// Search query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub league: Option<String>,
    pub item_type: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub online_only: Option<bool>,
}

impl SearchQuery {
    /// Load query from a JSON file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read query file")?;
        let query: SearchQuery =
            serde_json::from_str(&content).context("Failed to parse query JSON")?;
        Ok(query)
    }
}

/// Scraper for fetching and persisting trade listings
pub struct Scraper {
    #[allow(dead_code)]
    http_client: HttpClient,
    db: Db,
    config: Config,
    dry_run: bool,
}

impl Scraper {
    /// Create a new scraper instance
    pub async fn new(config: Config) -> Result<Self> {
        let http_client = HttpClient::new(&config.user_agent, config.requests_per_min)
            .context("Failed to create HTTP client")?;
        let db = Db::new(&config.db_url)
            .await
            .context("Failed to initialize database")?;

        Ok(Self {
            http_client,
            db,
            config,
            dry_run: false,
        })
    }

    /// Create a scraper in dry-run mode
    pub async fn new_dry_run(config: Config) -> Result<Self> {
        let mut scraper = Self::new(config).await?;
        scraper.dry_run = true;
        Ok(scraper)
    }

    /// Execute a scrape run with the given query
    pub async fn scrape(&mut self, query: &SearchQuery) -> Result<ScrapeMeta> {
        let source_url = format!("{}/api/trade2/search", self.config.trade_base_url);
        let mut meta = ScrapeMeta::new(source_url.clone(), Some("trade".to_string()));

        // Update active scrapes metric
        crate::metrics::Metrics::set_active_scrapes(1);

        // Begin scrape run
        self.db
            .begin_scrape_run(&meta)
            .await
            .context("Failed to begin scrape run")?;

        info!("Starting scrape run: {} ({})", meta.run_id, meta.source_url);

        // Execute the scrape
        match self.execute_scrape(query, &mut meta).await {
            Ok(()) => {
                meta.complete();
                info!(
                    "Scrape run completed: {} items processed, {} saved, {} errors",
                    meta.items_processed,
                    meta.items_saved,
                    meta.errors.len()
                );
            }
            Err(e) => {
                warn!("Scrape run failed: {}", e);
                meta.add_error(e.to_string());
                meta.complete();
                crate::metrics::Metrics::record_error("scrape_failure");
            }
        }

        // End scrape run
        self.db
            .end_scrape_run(&meta)
            .await
            .context("Failed to end scrape run")?;

        // Record metrics
        crate::metrics::Metrics::record_items_processed(meta.items_processed);
        crate::metrics::Metrics::record_items_saved(meta.items_saved);
        if let Some(duration_ms) = meta.duration_ms {
            crate::metrics::Metrics::record_scrape_duration(duration_ms);
        }
        crate::metrics::Metrics::set_active_scrapes(0);

        Ok(meta)
    }

    /// Internal method to execute the actual scraping
    async fn execute_scrape(&mut self, query: &SearchQuery, meta: &mut ScrapeMeta) -> Result<()> {
        if self.dry_run {
            info!("DRY RUN: Would scrape with query: {:?}", query);
            info!(
                "DRY RUN: Would make API request to: {}/api/trade2/search",
                self.config.trade_base_url
            );
            info!("DRY RUN: Would parse response and persist to database");
            meta.page_count = 1;
            meta.items_processed = 0;
            meta.items_saved = 0;
            return Ok(());
        }

        // For now, this is a placeholder that would:
        // 1. Hit the trade API endpoint with the query
        // 2. Parse the response
        // 3. Map to Listing models
        // 4. Persist to database

        // Mock implementation for demonstration
        debug!("Would execute API request here");

        // In a real implementation:
        // let response = self.http_client.get(&url).await?;
        // let listings = self.parse_response(response).await?;
        // for listing in listings {
        //     match self.db.insert_or_ignore_listing(&listing).await {
        //         Ok(true) => meta.items_saved += 1,
        //         Ok(false) => debug!("Listing already exists: {}", listing.id),
        //         Err(e) => {
        //             warn!("Failed to save listing: {}", e);
        //             meta.add_error(format!("Failed to save listing: {}", e));
        //         }
        //     }
        //     meta.items_processed += 1;
        // }

        meta.page_count = 1;
        Ok(())
    }

    /// Parse API response into listings (placeholder)
    #[allow(dead_code)]
    async fn parse_response(&self, _response: reqwest::Response) -> Result<Vec<Listing>> {
        // Placeholder - would parse actual API response
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery {
            league: Some("Standard".to_string()),
            item_type: Some("weapon".to_string()),
            min_price: Some(10.0),
            max_price: Some(100.0),
            online_only: Some(true),
        };

        assert_eq!(query.league, Some("Standard".to_string()));
        assert_eq!(query.min_price, Some(10.0));
    }

    #[tokio::test]
    async fn test_search_query_serialization() {
        let query = SearchQuery {
            league: Some("Standard".to_string()),
            item_type: Some("weapon".to_string()),
            min_price: Some(10.0),
            max_price: Some(100.0),
            online_only: Some(true),
        };

        let json = serde_json::to_string(&query).unwrap();
        let deserialized: SearchQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(query.league, deserialized.league);
        assert_eq!(query.min_price, deserialized.min_price);
    }
}

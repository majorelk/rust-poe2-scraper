use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use tracing::{debug, info};

use crate::model::{Listing, ScrapeMeta};

/// Database connection pool and helper functions
pub struct Db {
    pool: Pool<Sqlite>,
}

impl Db {
    /// Initialize database connection pool
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Connecting to database: {}", database_url);

        let pool = SqlitePool::connect(database_url)
            .await
            .context("Failed to connect to database")?;

        info!("Running migrations");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("Failed to run migrations")?;

        Ok(Self { pool })
    }

    /// Begin a new scrape run and return the run_id
    pub async fn begin_scrape_run(&self, meta: &ScrapeMeta) -> Result<String> {
        let query_params = serde_json::to_string(&meta.query_params)?;
        let started_at = meta.started_at.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO scrape_runs (
                run_id, started_at, source_url, shard, query_params
            ) VALUES (?, ?, ?, ?, ?)
            "#,
            meta.run_id,
            started_at,
            meta.source_url,
            meta.shard,
            query_params
        )
        .execute(&self.pool)
        .await
        .context("Failed to begin scrape run")?;

        debug!("Started scrape run: {}", meta.run_id);
        Ok(meta.run_id.clone())
    }

    /// End a scrape run with final statistics
    pub async fn end_scrape_run(&self, meta: &ScrapeMeta) -> Result<()> {
        let completed_at = meta.completed_at.map(|dt| dt.to_rfc3339());
        let errors = serde_json::to_string(&meta.errors)?;
        let duration_ms = meta.duration_ms.map(|d| d as i64);

        sqlx::query!(
            r#"
            UPDATE scrape_runs
            SET completed_at = ?,
                duration_ms = ?,
                page_count = ?,
                items_processed = ?,
                items_saved = ?,
                errors = ?
            WHERE run_id = ?
            "#,
            completed_at,
            duration_ms,
            meta.page_count,
            meta.items_processed,
            meta.items_saved,
            errors,
            meta.run_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to end scrape run")?;

        debug!("Ended scrape run: {}", meta.run_id);
        Ok(())
    }

    /// Insert or ignore a listing (idempotent)
    #[allow(dead_code)]
    pub async fn insert_or_ignore_listing(&self, listing: &Listing) -> Result<bool> {
        let raw_data = serde_json::to_string(listing)?;

        let account_online_league = listing
            .account
            .online
            .as_ref()
            .and_then(|o| o.league.as_ref())
            .map(|s| s.as_str());

        let account_online_status = listing
            .account
            .online
            .as_ref()
            .and_then(|o| o.status.as_ref())
            .map(|s| s.as_str());

        let stash_name = listing.stash.as_ref().map(|s| s.name.as_str());
        let stash_x = listing.stash.as_ref().and_then(|s| s.x);
        let stash_y = listing.stash.as_ref().and_then(|s| s.y);

        let price_amount = listing.price.as_ref().map(|p| p.amount);
        let price_currency = listing.price.as_ref().map(|p| p.currency.as_str());

        let result = sqlx::query!(
            r#"
            INSERT OR IGNORE INTO listings (
                id, item_name, base_type, type_line, item_level, rarity,
                category, icon, corrupted, identified,
                price_amount, price_currency,
                account_name, account_online_league, account_online_status, account_language,
                whisper, indexed,
                stash_name, stash_x, stash_y,
                raw_data
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            listing.id,
            listing.item.name,
            listing.item.base_type,
            listing.item.type_line,
            listing.item.item_level,
            listing.item.rarity,
            listing.item.category,
            listing.item.icon,
            listing.item.corrupted,
            listing.item.identified,
            price_amount,
            price_currency,
            listing.account.name,
            account_online_league,
            account_online_status,
            listing.account.language,
            listing.whisper,
            listing.indexed,
            stash_name,
            stash_x,
            stash_y,
            raw_data
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert listing")?;

        let inserted = result.rows_affected() > 0;

        if inserted {
            // Insert modifiers
            for modifier in &listing.item.modifiers {
                self.insert_modifier(&listing.id, modifier).await?;
            }
        }

        // Record metrics
        crate::metrics::Metrics::record_db_write("listings", true);

        Ok(inserted)
    }

    /// Insert a modifier for a listing
    #[allow(dead_code)]
    async fn insert_modifier(
        &self,
        listing_id: &str,
        modifier: &crate::model::Modifier,
    ) -> Result<()> {
        let magnitude = modifier.magnitudes.as_ref().and_then(|m| m.first());
        let hash = magnitude.map(|m| m.hash.as_str());
        let min_value = magnitude.and_then(|m| m.min_value);
        let max_value = magnitude.and_then(|m| m.max_value);

        sqlx::query!(
            r#"
            INSERT INTO listing_modifiers (
                listing_id, name, tier, level, hash, min_value, max_value
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            listing_id,
            modifier.name,
            modifier.tier,
            modifier.level,
            hash,
            min_value,
            max_value
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert modifier")?;

        Ok(())
    }

    /// Get a listing by ID
    #[allow(dead_code)]
    pub async fn get_listing(&self, id: &str) -> Result<Option<String>> {
        let row = sqlx::query!(
            r#"
            SELECT raw_data FROM listings WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get listing")?;

        Ok(row.map(|r| r.raw_data))
    }

    /// Count total listings
    #[allow(dead_code)]
    pub async fn count_listings(&self) -> Result<i64> {
        let row = sqlx::query!("SELECT COUNT(*) as count FROM listings")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count listings")?;

        Ok(row.count as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Account, Item, Price};

    #[sqlx::test]
    async fn test_insert_listing_idempotent(pool: Pool<Sqlite>) -> Result<()> {
        let db = Db { pool };

        let listing = Listing {
            id: "test-123".to_string(),
            item: Item {
                name: Some("Test Item".to_string()),
                base_type: "Iron Sword".to_string(),
                type_line: "Iron Sword".to_string(),
                item_level: 50,
                rarity: "rare".to_string(),
                category: Some("weapon".to_string()),
                icon: None,
                corrupted: Some(false),
                identified: Some(true),
                properties: None,
                requirements: None,
                modifiers: vec![],
                implicit_mods: None,
                explicit_mods: None,
            },
            price: Some(Price {
                amount: 10.0,
                currency: "chaos".to_string(),
            }),
            account: Account {
                name: "TestAccount".to_string(),
                online: None,
                language: None,
            },
            whisper: None,
            indexed: None,
            stash: None,
        };

        // First insert should succeed
        let inserted1 = db.insert_or_ignore_listing(&listing).await?;
        assert!(inserted1);

        // Second insert should be ignored (idempotent)
        let inserted2 = db.insert_or_ignore_listing(&listing).await?;
        assert!(!inserted2);

        // Verify only one row exists
        let count = db.count_listings().await?;
        assert_eq!(count, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_scrape_run_lifecycle(pool: Pool<Sqlite>) -> Result<()> {
        let db = Db { pool };

        let mut meta =
            ScrapeMeta::new("https://example.com".to_string(), Some("trade".to_string()));

        // Begin scrape run
        let run_id = db.begin_scrape_run(&meta).await?;
        assert_eq!(run_id, meta.run_id);

        // Update meta with some stats
        meta.page_count = 5;
        meta.items_processed = 100;
        meta.items_saved = 98;
        meta.add_error("Test error".to_string());
        meta.complete();

        // End scrape run
        db.end_scrape_run(&meta).await?;

        Ok(())
    }
}

use sqlx::{sqlite::SqlitePool, migrate::MigrateDatabase, Transaction, Sqlite};
use crate::models::{
    Item, 
    ItemModifier, 
    ItemBaseType,
    ItemCategory,
    StatRequirements,
    CoreAttribute
};
use crate::errors::Result;
use std::collections::HashMap;

const DEFAULT_DATABASE_URL: &str = "sqlite:poe_items.db";

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn initialize() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
        
        if !sqlx::Sqlite::database_exists(&database_url).await? {
            println!("Creating new database at {}", database_url);
            sqlx::Sqlite::create_database(&database_url).await?;
        }
        
        let pool = SqlitePool::connect(&database_url).await?;
        
        println!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;
        
        Ok(Self { pool })
    }

    pub async fn store_base_item(&self, base_item: &ItemBaseType) -> Result<i64> {
        let mut tx = self.pool.begin().await?;
        
        // First check if base item exists
        let existing_row = sqlx::query!(
            "SELECT id FROM base_items WHERE name = ?",
            base_item.name
        )
        .fetch_optional(&mut *tx)
        .await?;

        // Prepare all our data before using it in queries
        let stat_requirements_json = serde_json::to_string(&base_item.stat_requirements)?;
        let implicit_mods_json = serde_json::to_string(&base_item.implicit_modifiers)?;
        let tags_json = serde_json::to_string(&base_item.tags)?;
        let category_str = base_item.category.to_string();
        let base_level = base_item.base_level as i64;

        // Handle existing or insert new base item
        let id = if let Some(row) = existing_row {
            // Update existing base item
            sqlx::query!(
                r#"
                UPDATE base_items SET
                    category = ?,
                    stat_requirements = ?,
                    implicit_modifiers = ?,
                    base_level = ?,
                    tags = ?,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
                category_str,
                stat_requirements_json,
                implicit_mods_json,
                base_level,
                tags_json,
                row.id
            )
            .execute(&mut *tx)
            .await?;
            
            row.id.expect("Database returned null ID")
        } else {
            // Insert new base item
            let result = sqlx::query!(
                r#"
                INSERT INTO base_items (
                    name, category, stat_requirements,
                    implicit_modifiers, base_level, tags,
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
                "#,
                base_item.name,
                category_str,
                stat_requirements_json,
                implicit_mods_json,
                base_level,
                tags_json
            )
            .execute(&mut *tx)
            .await?;
            
            result.last_insert_rowid()
        };

        tx.commit().await?;
        Ok(id)
    }

    async fn ensure_modifier(&self, modifier: &ItemModifier, tx: &mut Transaction<'_, Sqlite>) -> Result<i64> {
        let existing_row = sqlx::query!(
            "SELECT id FROM modifiers WHERE name = ?",
            modifier.name
        )
        .fetch_optional(&mut **tx)
        .await?;

        match existing_row {
            Some(row) => Ok(row.id.expect("Database returned null ID")),
            None => {
                // Prepare all data before using in query
                let values_json = serde_json::to_string(&modifier.values)?;
                let stat_requirements_json = modifier.stat_requirements
                    .as_ref()
                    .map(|sr| serde_json::to_string(sr))
                    .transpose()?;
                let attribute_scaling_json = modifier.attribute_scaling
                    .as_ref()
                    .map(|scaling| serde_json::to_string(scaling))
                    .transpose()?;
                let tier = modifier.tier.map(|t| t as i64);

                let result = sqlx::query!(
                    r#"
                    INSERT INTO modifiers (
                        name, tier, modifier_values,
                        is_crafted, stat_requirements,
                        attribute_scaling, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
                    "#,
                    modifier.name,
                    tier,
                    values_json,
                    modifier.is_crafted,
                    stat_requirements_json,
                    attribute_scaling_json
                )
                .execute(&mut **tx)
                .await?;

                Ok(result.last_insert_rowid())
            }
        }
    }

    pub async fn store_collected_item(&self, item: &Item) -> Result<i64> {
        let mut tx = self.pool.begin().await?;

        // Create base item from item type information
        let base_item = ItemBaseType {
            name: item.item_type.base_type.clone(),
            category: item.item_type.category.clone(),
            stat_requirements: StatRequirements::new(),
            implicit_modifiers: vec![],
            base_level: item.item_type.required_level.unwrap_or(1),
            tags: vec![],
        };

        // Store or update base item
        let base_item_id = self.store_base_item(&base_item).await?;

        // Prepare all data before using in query
        let stats_json = serde_json::to_string(&item.stats)?;
        let stat_requirements_json = serde_json::to_string(&item.stat_requirements)?;
        let attribute_values_json = serde_json::to_string(&item.attribute_values)?;
        
        // Extract price information into owned values
        let (price_amount, price_currency) = if let Some(price) = &item.price {
            (Some(price.amount), Some(price.currency.clone()))
        } else {
            (None, None)
        };

        // Insert collected item
        let result = sqlx::query!(
            r#"
            INSERT INTO collected_items (
                trade_id, base_item_id, name,
                price_amount, price_currency,
                stats, corrupted, stat_requirements,
                attribute_values, collected_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
            item.id,
            base_item_id,
            item.name,
            price_amount,
            price_currency,
            stats_json,
            item.corrupted,
            stat_requirements_json,
            attribute_values_json
        )
        .execute(&mut *tx)
        .await?;

        let item_id = result.last_insert_rowid();

        // Store item modifiers with their values
        for modifier in &item.modifiers {
            let modifier_id = self.ensure_modifier(modifier, &mut tx).await?;
            let values_json = serde_json::to_string(&modifier.values)?;
            
            sqlx::query!(
                r#"
                INSERT INTO item_modifiers (
                    item_id, modifier_id, modifier_values
                ) VALUES (?, ?, ?)
                "#,
                item_id,
                modifier_id,
                values_json
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(item_id)
    }

    pub async fn base_item_exists(&self, name: &str) -> Result<bool> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM base_items WHERE name = ?",
            name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count > 0)
    }
}
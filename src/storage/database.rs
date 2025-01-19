use sqlx::{sqlite::SqlitePool, migrate::MigrateDatabase, Transaction, Sqlite};
use crate::models::{
    Item, 
    ItemModifier, 
    ItemBaseType,
    ItemCategory,
    StatRequirements,
    CoreAttribute,
    ItemResponse,
    ItemType,
    ItemRarity
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
        
        // First check if the base item already exists
        let existing_id = sqlx::query!(
            "SELECT id FROM base_items WHERE name = ?",
            base_item.name
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(|row| row.id);

        // Prepare JSON serialized data
        let stat_requirements_json = serde_json::to_string(&base_item.stat_requirements)?;
        let implicit_mods_json = serde_json::to_string(&base_item.implicit_modifiers)?;
        let tags_json = serde_json::to_string(&base_item.tags)?;
        let category_str = base_item.category.to_string();

        let id = if let Some(id) = existing_id {
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
                base_item.base_level as i64,
                tags_json,
                id
            )
            .execute(&mut *tx)
            .await?;
            id
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
                base_item.base_level as i64,
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
        let result = sqlx::query!(
            "SELECT id FROM modifiers WHERE name = ?",
            modifier.name
        )
        .fetch_optional(&mut **tx)
        .await?;

        match result {
            Some(row) => Ok(row.id),
            None => {
                let values_json = serde_json::to_string(&modifier.values)?;
                let stat_requirements_json = modifier.stat_requirements
                    .as_ref()
                    .map(|sr| serde_json::to_string(sr))
                    .transpose()?;
                let attribute_scaling_json = modifier.attribute_scaling
                    .as_ref()
                    .map(|scaling| serde_json::to_string(scaling))
                    .transpose()?;

                let result = sqlx::query!(
                    r#"
                    INSERT INTO modifiers (
                        name, tier, modifier_values,
                        is_crafted, stat_requirements,
                        attribute_scaling, created_at
                    ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
                    "#,
                    modifier.name,
                    modifier.tier.map(|t| t as i64),
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

    pub async fn store_collected_item(&self, item: &ItemResponse) -> Result<i64> {
        let mut tx = self.pool.begin().await?;

        // Convert ItemResponse to our internal Item model
        let base_item = ItemBaseType {
            name: item.item.base_type.clone(),
            category: match item.item.base_type.as_str() {
                // Add your category mapping logic here
                _ => ItemCategory::Other
            },
            stat_requirements: StatRequirements::new(),
            implicit_modifiers: vec![],
            base_level: item.item.ilvl,
            tags: vec![],
        };

        // Store base item first
        let base_item_id = self.store_base_item(&base_item).await?;

        // Store complex data as JSON
        let stats: HashMap<String, f64> = HashMap::new(); // Convert from item.item.properties
        let stats_json = serde_json::to_string(&stats)?;
        let stat_requirements = StatRequirements::new(); // Convert from item.item.requirements
        let stat_requirements_json = serde_json::to_string(&stat_requirements)?;
        let attribute_values = HashMap::new(); // Convert from item.item.requirements
        let attribute_values_json = serde_json::to_string(&attribute_values)?;

        // Cache price data to avoid temporary value issues
        let price_amount = item.listing.price.amount;
        let price_currency = item.listing.price.currency.clone();

        let item_id = sqlx::query!(
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
            item.item.name,
            price_amount,
            price_currency,
            stats_json,
            false, // Set corrupted status based on item data
            stat_requirements_json,
            attribute_values_json
        )
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        // Store modifiers
        for mod_info in &item.item.extended.mods.explicit {
            let modifier = ItemModifier {
                name: mod_info.name.clone(),
                tier: mod_info.tier.parse().ok().map(|t: i32| t),
                values: mod_info.magnitudes.iter()
                    .filter_map(|m| m.min.parse().ok())
                    .collect(),
                is_crafted: false,
                stat_requirements: None,
                attribute_scaling: None,
            };

            let modifier_id = self.ensure_modifier(&modifier, &mut tx).await?;
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
}
use sqlx::{sqlite::SqlitePool, migrate::MigrateDatabase};
use crate::{
    models::{
        Item, 
        ItemModifier, 
        ItemBaseType, 
        StatRequirements
    }, 
    errors::Result
};

const DEFAULT_DATABASE_URL: &str = "sqlite:poe_items.db";

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn initialize() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
        
        // First ensure the database exists
        if !sqlx::Sqlite::database_exists(&database_url).await? {
            println!("Creating new database at {}", database_url);
            sqlx::Sqlite::create_database(&database_url).await?;
        }
        
        // Connect to the database
        let pool = SqlitePool::connect(&database_url).await?;
        
        // Run migrations - this will only apply migrations that haven't been run yet
        println!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await?;
        
        Ok(Self { pool })
    }

    pub async fn store_base_item(&self, base_item: &ItemBaseType) -> Result<i64> {
        // First, serialize the complex structures we need to store as JSON
        let stat_requirements_json = serde_json::to_string(&base_item.stat_requirements)?;
        let implicit_mods_json = serde_json::to_string(&base_item.implicit_modifiers)?;
        let tags_json = serde_json::to_string(&base_item.tags)?;
        let category_str = base_item.category.to_string();

        let result = sqlx::query!(
            r#"
            INSERT INTO base_items (
                name, 
                category,
                stat_requirements,
                implicit_modifiers,
                base_level,
                tags,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(name) DO UPDATE SET
                category = excluded.category,
                stat_requirements = excluded.stat_requirements,
                implicit_modifiers = excluded.implicit_modifiers,
                base_level = excluded.base_level,
                tags = excluded.tags,
                updated_at = CURRENT_TIMESTAMP
            "#,
            base_item.name,
            category_str,
            stat_requirements_json,
            implicit_mods_json,
            base_item.base_level,
            tags_json
        )
        .execute(&self.pool)
        .await?;
    
        Ok(result.last_insert_rowid())
    }

    // Added method to check if a base item exists
    pub async fn base_item_exists(&self, name: &str) -> Result<bool> {
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM base_items WHERE name = ?",
            name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count > 0)
    }

    pub async fn store_modifier(&self, modifier: &ItemModifier) -> Result<i64> {
        // Serialize complex structures to JSON
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
                name, tier, modifier_values, is_crafted,
                stat_requirements, attribute_scaling,
                created_at
            ) VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(name) DO UPDATE SET
                tier = excluded.tier,
                modifier_values = excluded.modifier_values,
                is_crafted = excluded.is_crafted,
                stat_requirements = excluded.stat_requirements,
                attribute_scaling = excluded.attribute_scaling
            "#,
            modifier.name,
            modifier.tier,
            values_json,
            modifier.is_crafted,
            stat_requirements_json,
            attribute_scaling_json
        )
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn store_collected_item(&self, item: &Item) -> Result<i64> {
        let mut tx = self.pool.begin().await?;

        // Serialize complex structures
        let stats_json = serde_json::to_string(&item.stats)?;
        let stat_requirements_json = serde_json::to_string(&item.stat_requirements)?;
        let attribute_values_json = serde_json::to_string(&item.attribute_values)?;

        // First ensure we have the base item
        let base_item_id = self.ensure_base_item(&item.item_type.base_type, &mut tx).await?;

        let item_id = sqlx::query!(
            r#"
            INSERT INTO collected_items (
                trade_id, base_item_id, name, 
                price_amount, price_currency,
                stats, corrupted, stat_requirements,
                attribute_values, collected_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#,
            item.id,
            base_item_id,
            item.name,
            item.price.as_ref().map(|p| p.amount),
            item.price.as_ref().map(|p| &p.currency),
            stats_json,
            item.corrupted,
            stat_requirements_json,
            attribute_values_json
        )
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        // Store each modifier with its values
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

    // Helper method to ensure a base item exists and get its ID
    async fn ensure_base_item(&self, base_item: &ItemBaseType, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<i64> {
        let result = sqlx::query!(
            "SELECT id FROM base_items WHERE name = ?",
            base_item.name
        )
        .fetch_optional(&mut **tx)
        .await?;
    
        if let Some(row) = result {
            Ok(row.id.expect("Database returned null ID"))
        } else {
            self.store_base_item(base_item).await
        }
    }

    // Helper method to ensure a modifier exists and get its ID
    async fn ensure_modifier(&self, modifier: &ItemModifier, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> Result<i64> {
        let result = sqlx::query!(
            "SELECT id FROM modifiers WHERE name = ?",
            modifier.name
        )
        .fetch_optional(&mut **tx)
        .await?;
    
        if let Some(row) = result {
            Ok(row.id.expect("Database returned null ID"))
        } else {
            self.store_modifier(modifier).await
        }
    }
}
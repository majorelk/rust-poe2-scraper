use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};

use crate::fetcher::{
    CategoryFilter, CategoryOption, QueryFilters, SearchRequest, StatFilter, StatusFilter,
    TradeApiClient, TradeQuery, TypeFilters,
};
use crate::{
    analyzer::{ModifierAnalyzer, StatAnalyzer, StatCollector},
    config::Config,
    data::item_base_data_loader::BaseDataLoader,
    errors::ScraperError,
    models::Item,
    storage::Database,
};

// These are the top-level modules
mod analyzer;
mod config;
mod data;
mod db;
mod errors;
mod fetcher;
mod model;
mod models;
mod net;
mod storage;
mod telemetry;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, default_value = "Standard")]
    league: String,

    #[clap(short = 'n', long)]
    min_price: Option<f64>,

    #[clap(short = 'x', long)]
    max_price: Option<f64>,

    #[clap(long)]
    analyze_stats: bool,

    #[clap(long)]
    collect_data: bool,
}

async fn initialize_base_loader() -> Result<BaseDataLoader> {
    let mut loader = BaseDataLoader::new();

    // Try to load initial data from file
    if loader.load_from_file("data/item_bases.json").await.is_err() {
        info!("Item base data file not found or invalid, fetching from API");
        loader
            .update_from_api("https://api.pathofexile.com/trade/data/items")
            .await
            .context("Failed to fetch base item data from API")?;
        loader
            .save_to_file("data/item_bases.json")
            .await
            .context("Failed to save base item data to file")?;
    }

    // Check if data needs updating
    if loader.needs_update(std::time::Duration::from_secs(86400)) {
        info!("Base item data is stale, updating from API");
        loader
            .update_from_api("https://api.pathofexile.com/trade/data/items")
            .await
            .context("Failed to update base item data from API")?;
        loader
            .save_to_file("data/item_bases.json")
            .await
            .context("Failed to save updated base item data")?;
    }

    Ok(loader)
}

fn main() -> Result<()> {
    // Load configuration from environment
    let config = Config::load();

    // Initialize telemetry with config
    telemetry::init(&config.log_format);

    // Log config summary (no secrets)
    config.log_summary();

    tokio::runtime::Runtime::new()
        .context("Failed to create Tokio runtime")?
        .block_on(async {
            let args = Args::parse();

            info!("Starting POE2 scraper");
            info!("League: {}", args.league);

            // Initialize database first
            let db = Database::initialize()
                .await
                .context("Failed to initialize database")?;

            if args.collect_data {
                info!("Starting data collection");
                let client = TradeApiClient::new(args.league.clone());
                let mut collector = StatCollector::new(client);

                info!("Collecting stat data from trade API");
                let items = collector
                    .collect_stat_data()
                    .await
                    .context("Failed to collect stat data")?;
                let total_items = items.len();
                info!("Collected {} items from API", total_items);

                collector
                    .save_collected_data(&items, "collected_data.json")
                    .await
                    .context("Failed to save collected data to file")?;
                info!("Saved items to collected_data.json");

                let mut successful_conversions = 0;
                let mut successful_saves = 0;

                for (index, item_response) in items.into_iter().enumerate() {
                    info!("Processing item {}/{}", index + 1, total_items);

                    match Item::try_from(item_response) {
                        Ok(item) => {
                            successful_conversions += 1;
                            info!(
                                "Successfully converted item: {} ({})",
                                item.name.as_deref().unwrap_or("unnamed"),
                                item.id
                            );

                            match db.store_collected_item(&item).await {
                                Ok(_) => {
                                    successful_saves += 1;
                                    info!("Successfully stored item in database");
                                }
                                Err(e) => {
                                    warn!("Failed to store item in database: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to convert item {}: {}", index + 1, e);
                        }
                    }
                }

                info!("Collection summary:");
                info!("Total items processed: {}", total_items);
                info!("Successful conversions: {}", successful_conversions);
                info!("Successfully saved to DB: {}", successful_saves);
            }

            // Initialize the base loader
            let base_loader = initialize_base_loader()
                .await
                .context("Failed to initialize base loader")?;
            info!("Base item cache statistics:");
            info!(
                "{}",
                serde_json::to_string_pretty(&base_loader.get_cache_stats())?
            );

            // Store base items in database while keeping file-based cache
            for base_item in base_loader.get_all_bases() {
                if let Err(e) = db.store_base_item(base_item).await {
                    warn!("Failed to store base item in database: {}", e);
                }
            }

            let mut client = TradeApiClient::new(args.league);
            let _modifier_analyzer = ModifierAnalyzer::new(vec![0.0, 10.0, 20.0, 30.0, 40.0, 50.0]);
            let _stat_analyzer = StatAnalyzer::new();

            let query = SearchRequest {
                query: TradeQuery {
                    status: StatusFilter {
                        option: "online".to_string(),
                    },
                    stats: vec![StatFilter {
                        r#type: "and".to_string(),
                        filters: vec![],
                        disabled: false,
                    }],
                    filters: QueryFilters {
                        type_filters: TypeFilters {
                            filters: CategoryFilter {
                                category: CategoryOption {
                                    option: "any".to_string(),
                                },
                            },
                        },
                    },
                },
                sort: Some(serde_json::json!({
                    "price": "asc"
                })),
            };

            info!("Searching for items on trade API");
            let search_response = client
                .search_items(query)
                .await
                .context("Failed to search items")?;
            let raw_items = client
                .fetch_items(search_response.get_result_ids())
                .await
                .context("Failed to fetch items")?;

            info!("Processing {} items", raw_items.len());
            for raw_item in raw_items {
                let conversion_result =
                    serde_json::from_value::<crate::models::ItemResponse>(raw_item)
                        .map_err(|e| ScraperError::ParseError(e.to_string()))
                        .and_then(Item::try_from);

                match conversion_result {
                    Ok(mut item) => {
                        if let Some(base_type) = base_loader.get_base(&item.item_type.base_type) {
                            item.stat_requirements = base_type.stat_requirements.clone();

                            if let Err(e) = db.store_collected_item(&item).await {
                                warn!("Failed to store processed item: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to process item: {}", e);
                    }
                }
            }

            // Generate and save analysis reports
            if args.analyze_stats {
                let stat_report = _stat_analyzer.generate_attribute_report();

                info!("Stat Analysis Report:");
                info!("{}", serde_json::to_string_pretty(&stat_report)?);
            }

            info!("Analysis complete!");
            Ok(())
        })
}

use clap::Parser;
use tokio;
use serde_json;

use crate::{
    analyzer::{ModifierAnalyzer, StatAnalyzer, StatCollector},
    models::{Item, ItemModifier},
    errors::{ScraperError, Result},
    data::item_base_data_loader::BaseDataLoader,
};
use crate::fetcher::{
    TradeApiClient,
    SearchRequest,
    TradeQuery,
    StatusFilter,
    StatFilter,
    StatFilterValue,
    StatValue,
    QueryFilters,
    TypeFilters,
    CategoryFilter,
    CategoryOption,
    TradeStatus,
};

// These are your top-level modules
mod analyzer;
mod fetcher;
mod models;
mod errors;
mod data;

// We can define the initialize_base_loader function here for now
async fn initialize_base_loader() -> Result<BaseDataLoader> {
    let mut loader = BaseDataLoader::new();

    // Try to load initial data from file
    if loader.load_from_file("data/item_bases.json").await.is_err() {
        // If file doesn't exist or is invalid, update from API
        loader.update_from_api("https://api.pathofexile.com/trade/data/items").await?;
        // Save the fresh data
        loader.save_to_file("data/item_bases.json").await?;
    }

    // Check if data needs updating
    if loader.needs_update(std::time::Duration::from_secs(86400)) {  // 24 hours
        loader.update_from_api("https://api.pathofexile.com/trade/data/items").await?;
        loader.save_to_file("data/item_bases.json").await?;
    }

    Ok(loader)
}

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

fn main() -> Result<()> {
    tokio::runtime::Runtime::new()?.block_on(async {
        let args = Args::parse();
    
        if args.collect_data {
            println!("Starting data collection...");

            let client = TradeApiClient::new(args.league.clone());
            let mut collector = StatCollector::new(client);
            
            let items = collector.collect_stat_data().await?;
            collector.save_collected_data(&items, "collected_data.json").await?;
            
            println!("Collected {} items", items.len());
        }

        // Initialize the base loader
        let mut base_loader = initialize_base_loader().await?;
        println!("Base item cache statistics:");
        println!("{}", serde_json::to_string_pretty(&base_loader.get_cache_stats())?);
        
        // Now we can use the original args.league since we cloned it earlier
        let mut client = TradeApiClient::new(args.league);
        let mut modifier_analyzer = ModifierAnalyzer::new(vec![
            0.0, 10.0, 20.0, 30.0, 40.0, 50.0
        ]);
        let mut stat_analyzer = StatAnalyzer::new();

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

        let search_response = client.search_items(query).await?;
        let raw_items = client.fetch_items(search_response.get_result_ids()).await?;
        
        for raw_item in raw_items {
            if let Ok(mut item) = serde_json::from_value::<Item>(raw_item) {
                // Look up base type information
                if let Some(base_type) = base_loader.get_base(&item.item_type.base_type) {
                    // Update item with base requirements
                    item.stat_requirements = base_type.stat_requirements.clone();
                }
                
                modifier_analyzer.process_item(&item);
                if args.analyze_stats {
                    stat_analyzer.process_item(&item);
                }
            }
        }

        // Generate and save analysis reports
        if args.analyze_stats {
            let stat_report = stat_analyzer.generate_attribute_report();
            println!("Stat Analysis Report:");
            println!("{}", serde_json::to_string_pretty(&stat_report)?);
        }

        println!("Analysis complete!");
        Ok(())
    })
}
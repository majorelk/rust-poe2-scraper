use clap::Parser;
use tokio;
use serde_json;

use crate::{
    analyzer::ModifierAnalyzer,
    fetcher::{TradeApiClient, SearchRequest},
    models::{Item, ItemModifier},
    errors::{ScraperError, Result},
};

mod analyzer;
mod fetcher;
mod models;
mod errors;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(short, long, default_value = "Ancestor")]
    league: String,

    #[clap(short, long)]
    min_price: Option<f64>,

    #[clap(short, long)]
    max_price: Option<f64>,
    
    #[clap(long)]
    analyze_stats: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let mut client = TradeApiClient::new(args.league);
    let mut analyzer = ModifierAnalyzer::new(vec![
        0.0, 10.0, 20.0, 30.0, 40.0, 50.0
    ]);

    let query = SearchRequest {
        query: serde_json::json!({
            "status": { "option": "online" },
            "price": {
                "min": args.min_price,
                "max": args.max_price
            }
        }),
        sort: None,
    };

    let search_response = client.search_items(query).await?;
    let raw_items = client.fetch_items(search_response.get_result_ids()).await?;
    
    for raw_item in raw_items {
        if let Ok(item) = serde_json::from_value::<Item>(raw_item) {
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
}
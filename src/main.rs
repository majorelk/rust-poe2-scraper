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
            analyzer.process_item(&item);
        }
    }

    println!("Analysis complete!");
    Ok(())
}
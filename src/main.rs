mod item_fetcher;
mod modifier_analyzer;

use crate::item_fetcher::ItemFetcher;
use crate::modifier_analyzer::normalize_modifiers;

#[tokio::main]
async fn main() {
    let fetcher = ItemFetcher::new("Standard"); // Use the league you are interested in
    let query = "modifier";  // Use a valid query to search for items

    // Fetch the item data
    match fetcher.search_items(query).await {
        Ok(items) => {
            println!("Fetched {} items", items.len());
            
            // Normalize the modifiers to calculate their weights
            let modifier_weights = normalize_modifiers(items);
            
            // Output the normalized modifier weights
            println!("Normalized Modifier Weights: {:?}", modifier_weights);
        }
        Err(e) => {
            eprintln!("Error fetching items: {}", e);
        }
    }
}

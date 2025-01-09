use reqwest::{self, Error};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct ItemData {
    pub id: String,
    pub r#type: String,  // Use raw string for "type" as it's a reserved keyword
    pub name: String,
    pub rarity: String,
    pub explicit_mods: Option<Vec<String>>, // Changed to snake case
}

pub struct ItemFetcher {
    league: String,
}

impl ItemFetcher {
    // Constructor to initialize with a league
    pub fn new(league: &str) -> Self {
        ItemFetcher {
            league: league.to_string(),
        }
    }

    // Method to search items by query (you can extend query with more parameters as needed)
    pub async fn search_items(&self, query: &str) -> Result<Vec<ItemData>, reqwest::Error> {
        let search_url = format!("https://www.pathofexile.com/api/trade2/search/{}?query={}", self.league, query);
        let client = reqwest::Client::new();
        let search_response = client.get(&search_url).send().await?;

        // Log the status code to check if the request was successful
        println!("Response Status: {}", search_response.status());

        // Read the response body to inspect it
        let raw_body = search_response.text().await?;
        println!("Raw Response Body: {}", raw_body);

        // Attempt to parse the body and return an error if parsing fails
        let search_results: serde_json::Value = serde_json::from_str(&raw_body).map_err(|e| {
            reqwest::Error::new(reqwest::StatusCode::INTERNAL_SERVER_ERROR, Some(e.to_string()))
        })?;

        // Create a standalone vec for the fallback value
        let empty_vec = vec![];

        // Use the fallback vector correctly
        let item_ids = search_results["result"]
            .as_array()
            .unwrap_or(&empty_vec);  // We use `empty_vec` here instead of a temporary value

        // Convert item IDs into a comma-separated string
        let item_ids_str = item_ids.iter()
            .map(|item| item.as_str().unwrap())
            .collect::<Vec<&str>>()
            .join(",");

        // Fetch detailed data for items
        let fetch_url = format!("https://www.pathofexile.com/api/trade2/fetch/{}", item_ids_str);
        let fetch_response = client.get(&fetch_url).send().await?;
        let fetch_raw_body = fetch_response.text().await?;
        let item_details: Vec<ItemData> = serde_json::from_str(&fetch_raw_body).map_err(|e| {
            reqwest::Error::new(reqwest::StatusCode::INTERNAL_SERVER_ERROR, Some(e.to_string()))
        })?;

        Ok(item_details)
    }
}

// Custom error conversion function
impl From<serde_json::Error> for reqwest::Error {
    fn from(err: serde_json::Error) -> reqwest::Error {
        reqwest::Error::new(reqwest::StatusCode::INTERNAL_SERVER_ERROR, Some(err.to_string()))
    }
}
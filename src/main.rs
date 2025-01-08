use reqwest::Client;
use serde::Deserialize;
use serde_json;
use tokio;

#[derive(Deserialize, Debug)]
struct Item {
    text: Option<String>,
    name: Option<String>, // Make 'name' optional
    flags: Option<Flags>,
}

#[derive(Deserialize, Debug)]
struct Flags {
    unique: bool,
}

#[tokio::main]
async fn main() {
    let api_url = "https://www.pathofexile.com/api/trade2/data/items";
    let client = Client::new();

    // Send a GET request to the API with User-Agent header
    let res = client.get(api_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.36")
        .send()
        .await
        .expect("Failed to send request");

    // Check if the request was successful (status 200)
    if !res.status().is_success() {
        println!("Request failed with status: {}", res.status());
        return;
    }

    // Try to parse the JSON response
    let body = res.json::<serde_json::Value>()
        .await
        .expect("Failed to parse JSON");
    println!("Raw response body: {}", body);
    let json_output = serde_json::to_string_pretty(&body).unwrap();
    println!("Pretty JSON output: {}", json_output);

    // Extract the "result" field
    let items_value = body["result"].clone();
    println!("Items Found: {}", items_value.as_array().unwrap().len());

    let items: Vec<Item> = serde_json::from_value(items_value)
        .expect("Failed to parse items");

    println!("Items count: {}", items.len());

    // Filter unique items
    let unique_items: Vec<&Item> = items.iter()
        .filter(|item| item.flags.as_ref().map_or(false, |flags| flags.unique))
        .collect();

    // Print unique items
    for item in unique_items {
        println!("Unique Item: {} - {}", item.name.as_ref().unwrap_or(&"Unknown".to_string()), item.text.as_ref().unwrap_or(&"".to_string()));
    }
}

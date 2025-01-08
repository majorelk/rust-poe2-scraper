use reqwest::Client;
use scraper::{Html, Selector};
use tokio;
use tokio::time::{sleep, Duration};

async fn get_with_user_agent(url: &str) -> Result<String, reqwest::Error> {
    let client = Client::new();
    let res = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await?;
    let body = res.text().await?;
    Ok(body)
}

async fn scrape_with_delay() {
    let delay = Duration::from_secs(2); // 2 seconds
    sleep(delay).await; // Delay before making a request
}

#[tokio::main]
async fn main() {
    let url = "https://www.pathofexile.com/trade2/search/poe2/Standard";

    // Send the request with a user-agent
    let body = match get_with_user_agent(url).await {
        Ok(body) => body,
        Err(err) => {
            eprintln!("Error fetching data: {}", err);
            return;
        }
    };

    // Print the raw HTML response to debug
    println!("HTML Body: {}", body);

    // Parse the body as HTML
    let document = Html::parse_document(&body);
    let selector = Selector::parse("h1").unwrap();

    // Extract and print all the h1 tags
    for element in document.select(&selector) {
        println!("{}", element.text().collect::<Vec<_>>().join("\n"));
    }

    // Optionally, you can introduce a delay between requests if you're scraping multiple pages
    scrape_with_delay().await;
}

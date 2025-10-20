# Rust Scraper for Path of Exile 2

## Problem Overview

### Modifier Weighting
The exact modifier weightings (i.e., the likelihood of a modifier appearing on items in Path of Exile 2) are not available in the game client and cannot be directly data-mined.

### Goal
The objective is to obtain an overview of modifier distributions (weights) by scraping data from the trade site. By matching item modifiers to actual game modifiers, you can infer the general distribution of those modifiers.

## Solution Approach

### Scraping Data
- The idea is to scrape item listings from the trade site to gather data on item modifiers.
- The data gathered will then be matched to actual game modifiers to get an understanding of their distribution across items.

### Normalization & Breakpoints
- The data will be normalized by applying breakpoints to the modifier weights, which will allow you to generate familiar distribution curves (such as a bell curve).
- The normalization script will also adjust data between base item groups to ensure consistency for similar modifiers across similar base types.

### Bias Consideration
- The trade site is biased towards higher-tier mods or more desirable modifiers (higher-value listings).
- The scraping should account for this bias by including a range of listings, particularly lower-value ones, as higher-value listings may skew the results due to their association with better mods.
- However, in some cases, the bias cannot be fully avoided, as certain rare and highly desirable modifiers may only appear significantly in higher-value listings. This bias is accounted for in part in the normalization process.

### Error Acknowledgement
- There will be some inherent errors or inconsistencies in the data due to the limitations of scraping and normalizing.
- Despite these potential errors, the goal is to achieve a rough estimate of which modifiers are rarer or more common in the game.

## Key Points of the Approach
- The solution involves scraping data, matching item modifiers, normalizing the data, and accounting for biases.
- The success of this solution depends on the balance of scraping items from both low and high-value listings to reduce the bias from higher-tier mods.
- The goal is to generate a broad overview of modifier distributions (and their weights) with an understanding of their relative rarity or commonness, despite potential inconsistencies in the data.

## Getting Started

### Prerequisites
- Rust programming language (1.70+)
- Access to the Path of Exile 2 trade site

### Dev Quickstart

```bash
# Clone and setup
git clone https://github.com/majorelk/rust-poe2-scraper.git
cd rust-poe2-scraper
cp .env.example .env

# Build and run checks
make check          # Run fmt, lint, and test
make build          # Build the project
make run            # Run the application

# Or use cargo directly
cargo build         # Build
cargo test          # Run tests
cargo run -- --help # Show CLI options
```

**Available Make targets:**
- `make fmt` - Format code with rustfmt
- `make lint` - Run clippy linter
- `make test` - Run tests
- `make run` - Run the application
- `make build` - Build the project
- `make check` - Run all checks (fmt, lint, test)
- `make clean` - Clean build artifacts

### Installation
1. Clone the repository:
    ```sh
    git clone https://github.com/majorelk/rust-poe2-scraper.git
    ```
2. Navigate to the project directory:
    ```sh
    cd rust-poe2-scraper
    ```
3. Copy the example environment file and configure as needed:
    ```sh
    cp .env.example .env
    ```
4. Build the project:
    ```sh
    cargo build
    ```

### Configuration

All configuration is done through environment variables. Copy `.env.example` to `.env` and modify as needed:

```bash
# Database Configuration
DATABASE_URL=sqlite:poe_items.db      # SQLite database path
DB_URL=sqlite:poe_items.db            # Alternative database URL

# API Configuration
TRADE_BASE_URL=https://www.pathofexile.com  # POE trade API base URL
USER_AGENT=POE2-Scraper/0.1.0               # HTTP User-Agent string

# Rate Limiting
REQUESTS_PER_MIN=60          # Maximum API requests per minute
RATE_LIMIT_DELAY_MS=500      # Delay between requests in milliseconds

# Logging
RUST_LOG=info                # Log level: trace, debug, info, warn, error
LOG_FORMAT=pretty            # Log format: pretty or json
```

### Usage

The scraper provides several commands for different operations:

#### 1. Scrape Data from Trade API

```bash
# Run a scrape with a query file
cargo run -- --scrape-query @examples/entry-level.json

# With detailed logging
RUST_LOG=debug cargo run -- --scrape-query @examples/entry-level.json

# With JSON logging for production
RUST_LOG=info LOG_FORMAT=json cargo run -- --scrape-query @examples/entry-level.json
```

#### 2. Normalize Scraped Data

```bash
# Run normalization on scraped data
cargo run -- --normalize

# With verbose output
RUST_LOG=debug cargo run -- --normalize
```

#### 3. Enable Metrics (Prometheus)

```bash
# Start scraper with metrics endpoint
cargo run -- --scrape-query @examples/entry-level.json --metrics 0.0.0.0:9464

# Access metrics at http://localhost:9464/metrics
```

#### 4. Dry Run Mode (Safety Rails)

```bash
# Test scraping without making network calls
cargo run -- --scrape-query @examples/entry-level.json --dry-run

# Logs intent without executing
# Useful for testing configuration and query files
```

#### 5. Legacy Commands (for existing workflows)

```bash
# Collect data (legacy)
cargo run -- --collect-data

# Analyze stats (legacy)
cargo run -- --analyze-stats
```

### Complete Example Workflow

```bash
# 1. Setup
cp .env.example .env
make build

# 2. Scrape data from trade API
cargo run -- --scrape-query @examples/entry-level.json

# 3. Normalize the scraped data
cargo run -- --normalize

# 4. Run with metrics enabled
cargo run -- --scrape-query @examples/entry-level.json --metrics 0.0.0.0:9464 &
curl http://localhost:9464/metrics
```

### Output

- **Database**: All scraped data is stored in `poe_items.db` (SQLite)
- **Logs**: Structured logs to stdout (format configurable via `LOG_FORMAT`)
- **Metrics**: Prometheus metrics at `/metrics` endpoint (when enabled)

### Features

#### Production-Ready Infrastructure
- **Rate-limited API requests**: Governor-based rate limiting with configurable RPS and exponential backoff with jitter
- **Polite User-Agent headers**: Properly identifies the scraper to the API
- **Environment-based configuration**: All settings via `.env` file, no hard-coded secrets
- **Structured logging**: JSON or pretty-printed logs with tracing, configurable log levels
- **Idempotent database writes**: Uses `INSERT OR IGNORE` and unique constraints to prevent duplicates
- **Robust error handling**: Uses `anyhow` and `thiserror` for comprehensive error context
- **No `.unwrap()` calls**: All error handling is explicit and graceful
- **Safety rails**: Dry-run mode for testing without making network calls
- **Respectful scraping**: Respects rate limits and provides clear user identification

#### Data Pipeline
- **SQLite persistence**: Comprehensive schema with migrations for listings, modifiers, and audit trails
- **Scrape runs tracking**: Full audit trail with timestamps, duration, items processed, and errors
- **Normalization pipeline**: Bias-aware quantile-based scoring across base types
- **Query-based scraping**: JSON query files for reproducible scraping runs

#### Observability
- **Prometheus metrics**: Expose metrics at `/metrics` endpoint
- **Comprehensive instrumentation**: Tracks HTTP requests, database writes, items processed, scrape duration, normalization runs
- **Active monitoring**: Real-time gauges for active scrapes and error counts

### Available Metrics

When running with `--metrics`, the following metrics are exposed:

- `http_requests_total` - Total HTTP requests made
- `http_retries_total` - Total HTTP retry attempts
- `db_writes_total` - Total database write operations
- `items_processed_total` - Total items processed in scrapes
- `items_saved_total` - Total items successfully saved to database
- `errors_total` - Total errors encountered
- `http_request_duration_ms` - HTTP request duration histogram
- `scrape_duration_ms` - Scrape run duration histogram
- `normalization_duration_ms` - Normalization duration histogram
- `active_scrapes` - Current number of active scrape operations
- `last_normalization_stats` - Number of stats from last normalization
- `normalizations_total` - Total normalization runs

## Contributing
Contributions are welcome! Please open an issue or submit a pull request.

## License
This project is licensed under the MIT License.

## Acknowledgements
- Path of Exile 2 for providing the trade site.
- Rust community for their support and resources.

## Future Enhancements

### Additional Data Sources
- Incorporate data from other Path of Exile 2 community resources to enhance the accuracy of modifier weightings.
- Explore the possibility of using API endpoints provided by the game developers for more direct data access.

### Improved Normalization Techniques
- Develop more sophisticated normalization algorithms to better account for biases and inconsistencies in the scraped data.
- Implement machine learning models to predict modifier distributions based on historical data.

### User Interface
- Create a user-friendly interface to visualize the modifier distributions and allow users to interact with the data.
- Provide filtering options to let users focus on specific item types or modifiers.

### Automation
- Automate the scraping process to run at regular intervals, ensuring that the data remains up-to-date with the latest listings on the trade site.
- Set up notifications or alerts for significant changes in modifier distributions.
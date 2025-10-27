-- Listings table for scraped trade listings
CREATE TABLE IF NOT EXISTS listings (
    id TEXT PRIMARY KEY,
    item_name TEXT,
    base_type TEXT NOT NULL,
    type_line TEXT NOT NULL,
    item_level INTEGER NOT NULL,
    rarity TEXT NOT NULL,
    category TEXT,
    icon TEXT,
    corrupted BOOLEAN,
    identified BOOLEAN,
    price_amount REAL,
    price_currency TEXT,
    account_name TEXT NOT NULL,
    account_online_league TEXT,
    account_online_status TEXT,
    account_language TEXT,
    whisper TEXT,
    indexed TEXT,
    stash_name TEXT,
    stash_x INTEGER,
    stash_y INTEGER,
    raw_data TEXT NOT NULL,  -- Store complete JSON for reference
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Unique constraint on listing ID to ensure idempotency
CREATE UNIQUE INDEX IF NOT EXISTS idx_listings_id ON listings(id);

-- Index for querying by base type
CREATE INDEX IF NOT EXISTS idx_listings_base_type ON listings(base_type);

-- Index for querying by price
CREATE INDEX IF NOT EXISTS idx_listings_price ON listings(price_amount, price_currency);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_listings_created_at ON listings(created_at);

-- Modifiers table for individual modifiers
CREATE TABLE IF NOT EXISTS listing_modifiers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    listing_id TEXT NOT NULL,
    name TEXT NOT NULL,
    tier TEXT,
    level INTEGER,
    hash TEXT,
    min_value REAL,
    max_value REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (listing_id) REFERENCES listings(id) ON DELETE CASCADE
);

-- Index for querying modifiers by listing
CREATE INDEX IF NOT EXISTS idx_listing_modifiers_listing_id ON listing_modifiers(listing_id);

-- Index for querying modifiers by name
CREATE INDEX IF NOT EXISTS idx_listing_modifiers_name ON listing_modifiers(name);

-- Index for querying modifiers by tier
CREATE INDEX IF NOT EXISTS idx_listing_modifiers_tier ON listing_modifiers(tier);

-- Junction table for item properties
CREATE TABLE IF NOT EXISTS listing_properties (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    listing_id TEXT NOT NULL,
    name TEXT NOT NULL,
    property_values TEXT NOT NULL,  -- JSON array (renamed from 'values')
    display_mode INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (listing_id) REFERENCES listings(id) ON DELETE CASCADE
);

-- Index for querying properties by listing
CREATE INDEX IF NOT EXISTS idx_listing_properties_listing_id ON listing_properties(listing_id);

-- Junction table for item requirements
CREATE TABLE IF NOT EXISTS listing_requirements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    listing_id TEXT NOT NULL,
    name TEXT NOT NULL,
    requirement_values TEXT NOT NULL,  -- JSON array (renamed from 'values')
    display_mode INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (listing_id) REFERENCES listings(id) ON DELETE CASCADE
);

-- Index for querying requirements by listing
CREATE INDEX IF NOT EXISTS idx_listing_requirements_listing_id ON listing_requirements(listing_id);

-- Scrape runs table for tracking scrape sessions
CREATE TABLE IF NOT EXISTS scrape_runs (
    run_id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    source_url TEXT NOT NULL,
    shard TEXT,
    duration_ms INTEGER,
    page_count INTEGER NOT NULL DEFAULT 0,
    items_processed INTEGER NOT NULL DEFAULT 0,
    items_saved INTEGER NOT NULL DEFAULT 0,
    errors TEXT,  -- JSON array of error messages
    query_params TEXT,  -- JSON object of query parameters
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for querying scrape runs by time
CREATE INDEX IF NOT EXISTS idx_scrape_runs_started_at ON scrape_runs(started_at);

-- Index for querying scrape runs by completion
CREATE INDEX IF NOT EXISTS idx_scrape_runs_completed ON scrape_runs(completed_at);

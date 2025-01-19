-- Base items table with expanded fields to match ItemBaseType
CREATE TABLE base_items (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category TEXT NOT NULL,                    -- Stores ItemCategory as string
    stat_requirements TEXT NOT NULL,           -- Stores StatRequirements as JSON
    implicit_modifiers TEXT NOT NULL,          -- Stores Vec<String> as JSON
    base_level INTEGER NOT NULL,
    tags TEXT NOT NULL,                        -- Stores Vec<String> as JSON
    created_at TEXT NOT NULL,                  -- SQLite preferred datetime format
    updated_at TEXT NOT NULL                   -- SQLite preferred datetime format
);

-- Modifiers table updated to match ItemModifier structure
CREATE TABLE modifiers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,                 -- Changed from 'text' to match struct
    tier INTEGER,                              -- Made nullable to match Option<i32>
    modifier_values TEXT NOT NULL,             -- Stores Vec<f64> as JSON
    is_crafted BOOLEAN NOT NULL DEFAULT FALSE,
    stat_requirements TEXT,                    -- Stores Option<ModifierStatRequirements> as JSON
    attribute_scaling TEXT,                    -- Stores Option<HashMap<CoreAttribute, f64>> as JSON
    created_at TEXT NOT NULL                   -- SQLite preferred datetime format
);

-- Collected items table expanded to include all Item fields
CREATE TABLE collected_items (
    id INTEGER PRIMARY KEY,
    trade_id TEXT UNIQUE NOT NULL,             -- Maps to Item.id
    base_item_id INTEGER NOT NULL,
    name TEXT,                                 -- Nullable to match Option<String>
    price_amount REAL,                         -- Made nullable for Option<ItemPrice>
    price_currency TEXT,                       -- Made nullable for Option<ItemPrice>
    stats TEXT NOT NULL,                       -- Stores HashMap<String, f64> as JSON
    corrupted BOOLEAN NOT NULL DEFAULT FALSE,
    stat_requirements TEXT NOT NULL,           -- Stores StatRequirements as JSON
    attribute_values TEXT NOT NULL,            -- Stores HashMap<CoreAttribute, u32> as JSON
    collected_at TEXT NOT NULL,                -- SQLite preferred datetime format
    FOREIGN KEY (base_item_id) REFERENCES base_items(id)
);

-- Item modifiers junction table updated to handle Vec<f64> values
CREATE TABLE item_modifiers (
    item_id INTEGER NOT NULL,
    modifier_id INTEGER NOT NULL,
    modifier_values TEXT NOT NULL,             -- Stores Vec<f64> as JSON
    PRIMARY KEY (item_id, modifier_id),
    FOREIGN KEY (item_id) REFERENCES collected_items(id),
    FOREIGN KEY (modifier_id) REFERENCES modifiers(id)
);

-- Updated indexes for new field names
CREATE INDEX idx_collected_items_collected_at ON collected_items(collected_at);
CREATE INDEX idx_base_items_name ON base_items(name);
CREATE INDEX idx_modifiers_name ON modifiers(name);
CREATE INDEX idx_collected_items_trade_id ON collected_items(trade_id);
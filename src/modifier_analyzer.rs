use crate::item_fetcher::ItemData;
use std::collections::HashMap;

pub fn normalize_modifiers(items: Vec<ItemData>) -> HashMap<String, f32> {
    let mut modifier_counts: HashMap<String, f32> = HashMap::new();
    let mut total_items = 0;

    // Count the occurrences of each modifier
    for item in items {
        if let Some(mods) = item.explicit_mods {
            for modifier in mods {
                *modifier_counts.entry(modifier).or_insert(0.0) += 1.0;
            }
            total_items += 1;
        }
    }

    // Normalize by dividing counts by the total number of items
    for (_, count) in modifier_counts.iter_mut() {
        *count /= total_items as f32;
    }

    modifier_counts
}

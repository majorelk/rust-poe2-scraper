use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::ItemResponse;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanedItem {
    pub base_type: String,
    pub name: String,
    pub explicit_mods: Vec<String>,
    pub item_level: u32,
    pub properties: Vec<ItemProperty>,
    pub requirements: Vec<ItemRequirement>,
    pub mod_info: ModInfo,
    pub mod_hashes: HashMap<String, Vec<Vec<i32>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemProperty {
    pub name: String,
    pub values: Vec<(String, i32)>,
    pub display_mode: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemRequirement {
    pub name: String,
    pub values: Vec<(String, i32)>,
    pub display_mode: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModInfo {
    pub explicit: Vec<ExplicitMod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExplicitMod {
    pub level: u32,
    pub magnitudes: Vec<Magnitude>,
    pub name: String,
    pub tier: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Magnitude {
    pub hash: String,
    pub max: String,
    pub min: String,
}

impl CleanedItem {
    pub fn from_response(response: &ItemResponse) -> Self {
        Self {
            base_type: response.item.base_type.clone(),
            name: response.item.type_line.clone(),
            explicit_mods: response.item.explicit_mods.clone(),
            item_level: response.item.ilvl,
            properties: response.item.properties.iter()
                .map(|p| ItemProperty {
                    name: p.name.clone(),
                    values: p.values.clone(),
                    display_mode: p.display_mode,
                })
                .collect(),
            requirements: response.item.requirements.iter()
                .map(|r| ItemRequirement {
                    name: r.name.clone(),
                    values: r.values.clone(),
                    display_mode: r.display_mode,
                })
                .collect(),
            mod_info: ModInfo {
                explicit: response.item.extended.mods.explicit.iter()
                    .map(|m| ExplicitMod {
                        level: m.magnitudes.first().map(|mag| mag.min.parse::<u32>().unwrap_or(0)).unwrap_or(0),
                        magnitudes: m.magnitudes.clone(),
                        name: m.name.clone(),
                        tier: m.tier.clone(),
                    })
                    .collect(),
            },
            mod_hashes: response.item.extended.hashes.explicit.iter()
                .map(|(k, v)| (k.clone(), vec![v.clone()]))
                .collect(),
        }
    }

    pub fn get_stat_requirements(&self) -> HashMap<String, u32> {
        self.requirements.iter()
            .filter(|req| {
                matches!(req.name.as_str(),
                    "[Strength|Str]" | "[Dexterity|Dex]" | "[Intelligence|Int]")
            })
            .filter_map(|req| {
                req.values.first().map(|(value, _)| {
                    (req.name.clone(), value.parse::<u32>().unwrap_or(0))
                })
            })
            .collect()
    }

    pub fn get_explicit_mods(&self) -> Vec<(&str, &str)> {
        self.mod_info.explicit.iter()
            .map(|m| (m.name.as_str(), m.tier.as_str()))
            .collect()
    }
}
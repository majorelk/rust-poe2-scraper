use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::ItemResponse;
use super::poe_item::{Magnitude, ModInfo as PoeModInfo};
use crate::models::poe_item::ModBase;
use std::ops::Deref;
use crate::analyzer::stat_analyzer::ModInfoLike;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CleanedItem {
    // Core item information
    pub base_type: String,      // from baseType
    pub name: String,           // from name
    pub explicit_mods: Vec<String>,  // from explicitMods
    pub item_level: u32,        // from ilvl
    
    // Item attributes
    pub properties: Vec<ItemProperty>,    // from properties
    pub requirements: Vec<ItemRequirement>,  // from requirements
    
    // Mod information
    pub mod_info: ModInfo,      // structured mod data from extended.mods
    pub mod_hashes: HashMap<String, Vec<Vec<i32>>>,  // from extended.hashes
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
    pub explicit: Vec<ExplicitMod>,  // Collection of explicit mods
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExplicitMod {
    #[serde(flatten)]
    base: ModBase,
    pub level: u32,
    // pub magnitudes: Vec<Magnitude>,  // Each mod can have multiple magnitude entries
    // pub name: String,
    // pub tier: String,
}

impl Deref for ExplicitMod {
    type Target = ModBase;
    
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl CleanedItem {
    pub fn from_response(response: &ItemResponse) -> Self {
        Self {
            base_type: response.item.base_type.clone(),
            name: response.item.type_line.clone(),
            explicit_mods: response.item.explicit_mods.clone(),
            item_level: response.item.ilvl,
            
            // Map properties maintaining their structure
            properties: response.item.properties.iter()
                .map(|p| ItemProperty {
                    name: p.name.clone(),
                    values: p.values.clone(),
                    display_mode: p.display_mode,
                })
                .collect(),
            
            // Map requirements maintaining their structure
            requirements: response.item.requirements.iter()
                .map(|r| ItemRequirement {
                    name: r.name.clone(),
                    values: r.values.clone(),
                    display_mode: r.display_mode,
                })
                .collect(),
            
            // Map the explicit mods data
            mod_info: ModInfo {
                explicit: response.item.extended.mods.explicit.iter()
                    .map(|m| ExplicitMod {
                        base: ModBase {
                            name: m.name.clone(),
                            tier: m.tier.clone(),
                            magnitudes: m.magnitudes.clone(),
                        },
                        level: m.magnitudes.first()
                            .map(|mag| mag.min.parse::<u32>().unwrap_or(0))
                            .unwrap_or(0),
                    })
                    .collect(),
            },
            
            // Map the hash data structure
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
            .map(|m| (m.get_name(), m.get_tier()))
            .collect()
    }
}
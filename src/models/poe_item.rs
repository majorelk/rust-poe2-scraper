use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModBase {
    pub name: String,
    pub tier: String,
    pub magnitudes: Vec<Magnitude>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModInfo {
    #[serde(flatten)]
    base: ModBase,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExplicitMod {
    #[serde(flatten)]
    base: ModBase,
    pub level: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ItemResponse {
    pub id: String,
    pub item: ItemData,
    pub listing: ListingData,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ItemData {
    pub base_type: String,
    #[serde(rename = "explicitMods")]
    pub explicit_mods: Vec<String>,
    pub extended: ExtendedData,
    #[serde(rename = "frameType")]
    pub frame_type: i32,
    pub requirements: Vec<Requirement>,
    pub properties: Vec<Property>,
    pub rarity: String,
    #[serde(rename = "typeLine")]
    pub type_line: String,
    pub ilvl: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExtendedData {
    pub mods: ModData,
    pub hashes: HashData,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModData {
    pub explicit: Vec<ModInfo>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Magnitude {
    pub hash: String,
    pub min: String,
    pub max: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HashData {
    pub explicit: Vec<(String, Vec<i32>)>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Requirement {
    pub name: String,
    pub values: Vec<(String, i32)>,
    pub display_mode: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Property {
    pub name: String,
    pub values: Vec<(String, i32)>,
    pub display_mode: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListingData {
    pub price: Price,
    pub account: Account,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Price {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Account {
    pub name: String,
    pub realm: String,
}

impl Deref for ModInfo {
    type Target = ModBase;
    
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl Deref for ExplicitMod {
    type Target = ModBase;
    
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ItemResponse {
    pub fn get_stat_requirements(&self) -> HashMap<String, u32> {
        self.item.requirements
            .iter()
            .filter(|req| req.name == "Strength" || req.name == "Dexterity" || req.name == "Intelligence")
            .filter_map(|req| {
                req.values.first().map(|(value, _)| {
                    (req.name.clone(), value.parse::<u32>().unwrap_or(0))
                })
            })
            .collect()
    }

    pub fn get_explicit_mod_values(&self) -> Vec<(String, f64)> {
        self.item.extended.mods.explicit
            .iter()
            .filter_map(|mod_info| {
                mod_info.magnitudes.first().map(|mag| {
                    (mod_info.name.clone(), mag.min.parse::<f64>().unwrap_or(0.0))
                })
            })
            .collect()
    }
}
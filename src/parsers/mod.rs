use crate::util::Resources;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::collections::HashMap;

pub mod warframestat;
pub mod world_state;

#[derive(Debug, Clone)]
pub struct CetusCycle {
    /// Expiry time for the whole cycle (= night).
    pub expiry: DateTime<Utc>,
}

impl Default for CetusCycle {
    fn default() -> Self {
        CetusCycle {
            expiry: Utc.timestamp(0, 0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Fissure {
    /// Expiry time for the fissure.
    pub expiry: DateTime<Utc>,
    /// Solar node
    pub node: SolarNode,
    /// Missin type in string, e.g: `Capture`.
    pub mission: String,
    /// Fissure tier: Lith, Meso, etc.
    pub tier: FissureTier,
    /// True if this fissure is a void storm.
    pub is_storm: bool,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum FissureTier {
    Unknown,
    Lith,
    Meso,
    Neo,
    Axi,
    Requiem,
}

impl FissureTier {
    pub fn from_str(string: &str) -> Self {
        match string {
            "Lith" => FissureTier::Lith,
            "Meso" => FissureTier::Meso,
            "Neo" => FissureTier::Neo,
            "Axi" => FissureTier::Axi,
            "Requiem" => FissureTier::Requiem,
            &_ => FissureTier::Unknown,
        }
    }
}

impl ToString for FissureTier {
    fn to_string(&self) -> String {
        match self {
            FissureTier::Unknown => "Unknown".to_string(),
            FissureTier::Lith => "Lith".to_string(),
            FissureTier::Meso => "Meso".to_string(),
            FissureTier::Neo => "Neo".to_string(),
            FissureTier::Axi => "Axi".to_string(),
            FissureTier::Requiem => "Requiem".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolarNode {
    pub value: String,
    pub enemy: Option<String>,
    #[serde(alias = "type")]
    pub node_type: Option<String>,
}

impl Default for SolarNode {
    fn default() -> Self {
        SolarNode {
            value: "Unknown".to_string(),
            enemy: None,
            node_type: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SolarNodes(HashMap<String, SolarNode>);

impl SolarNodes {
    fn find_key_for_value<'a>(
        map: &'a HashMap<String, SolarNode>,
        value: &str,
    ) -> Option<&'a String> {
        map.iter()
            .find_map(|(key, val)| if val.value == value { Some(key) } else { None })
    }
}

pub trait TennoParser {
    fn parse_fissures(&self, data: &str) -> Vec<Fissure>;

    fn parse_cetus_cycle(&self, data: &str) -> CetusCycle;
    /// Parses solar node data from the local data file.
    fn solar_nodes(&self) -> SolarNodes {
        let sol_data = Resources::get("data/sol_node.json").unwrap().data;

        let solar_nodes: SolarNodes = serde_json::from_slice(&sol_data).expect("Bad JSON.");
        solar_nodes
    }
    /// Returns the correct solar node based on the key given, e.g: SolNode401.
    fn get_solar_node_by_key(&self, key: &str) -> SolarNode {
        if let Some(node) = self.solar_nodes().0.get(key) {
            return node.clone();
        }

        SolarNode::default()
    }
    /// Returns the correct solar node based on the value given, e.g: Hepit (Void).
    fn get_solar_node_by_value(&self, value: &str) -> SolarNode {
        if let Some(key) = SolarNodes::find_key_for_value(&self.solar_nodes().0, value) {
            return self.get_solar_node_by_key(key);
        }

        SolarNode::default()
    }
}

use crate::parsers::{CetusCycle, Fissure, FissureTier, TennoParser};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

pub struct WarframeStat {}

impl TennoParser for WarframeStat {
    fn parse_fissures(&self, data: &str) -> Vec<Fissure> {
        let parsed: Vec<_Fissure> = serde_json::from_str(data).expect("Deserialize error!");

        let mut fissures = parsed
            .iter()
            .map(|f| Fissure {
                expiry: f.expiry,
                node: self.get_solar_node_by_value(&f.node),
                mission: f.mission_key.clone(),
                tier: FissureTier::from_str(&f.tier),
                is_storm: f.is_storm,
            })
            .collect::<Vec<Fissure>>();
        fissures.sort_by_key(|f| f.tier.clone());

        fissures
    }

    fn parse_cetus_cycle(&self, data: &str) -> CetusCycle {
        let parsed: _CetusCycle = serde_json::from_str(data).expect("Deserialize error!");

        let expiry = if parsed.is_day {
            parsed.expiry + Duration::seconds(3000)
        } else {
            parsed.expiry
        };

        CetusCycle { expiry }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct _Fissure {
    expiry: DateTime<Utc>,
    node: String,
    mission_key: String,
    tier: String,
    is_storm: bool,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct _CetusCycle {
    expiry: DateTime<Utc>,
    is_day: bool,
}

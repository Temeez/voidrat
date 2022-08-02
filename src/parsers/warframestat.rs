use crate::parsers::{
    CetusCycle, Fissure, FissureTier, Invasion, InvasionReward, Reward, TennoParser,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Deserializer};

pub struct WarframeStat {}

impl TennoParser for WarframeStat {
    fn parse_invasions(&self, data: &str) -> Vec<Invasion> {
        let parsed: Vec<_Invasion> = serde_json::from_str(data).expect("Deserialize error!");

        let invasions = parsed
            .iter()
            .filter(|i| !i.completed)
            .map(|i| Invasion {
                activation: i.activation,
                rewards: InvasionReward {
                    attacker: i
                        .attacker_reward
                        .iter()
                        .map(|r| Reward {
                            item: r.item_type.clone(),
                            quantity: r.item_count,
                        })
                        .collect::<Vec<Reward>>(),
                    defender: i
                        .defender_reward
                        .iter()
                        .map(|r| Reward {
                            item: r.item_type.clone(),
                            quantity: r.item_count,
                        })
                        .collect::<Vec<Reward>>(),
                },
                node: self.get_solar_node_by_value(&i.node),
            })
            .collect::<Vec<Invasion>>();

        invasions
    }

    fn parse_fissures(&self, data: &str) -> Vec<Fissure> {
        let parsed: Vec<_Fissure> = serde_json::from_str(data).expect("Deserialize error!");

        let mut fissures = parsed
            .iter()
            .map(|f| Fissure {
                activation: f.activation,
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
    activation: DateTime<Utc>,
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

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct _InvasionReward {
    #[serde(alias = "type")]
    item_type: String,
    #[serde(alias = "count")]
    item_count: u32,
}

#[derive(Debug, Clone)]
struct _Invasion {
    node: String,
    activation: DateTime<Utc>,
    attacker_reward: Vec<_InvasionReward>,
    defender_reward: Vec<_InvasionReward>,
    completed: bool,
}

impl<'de> Deserialize<'de> for _Invasion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        #[serde_with::serde_as]
        struct Outer {
            node: String,
            activation: DateTime<Utc>,
            attacker_reward: RewardInner,
            defender_reward: RewardInner,
            completed: bool,
        }

        #[derive(Deserialize, Debug, Default)]
        #[serde_with::serde_as]
        struct RewardInner {
            #[serde(alias = "countedItems")]
            #[serde_as(deserialize_as = "DefaultOnError")]
            counted_items: Vec<_InvasionReward>,
        }

        let helper = Outer::deserialize(deserializer)?;

        Ok(_Invasion {
            node: helper.node,
            activation: helper.activation,
            attacker_reward: helper.attacker_reward.counted_items,
            defender_reward: helper.defender_reward.counted_items,
            completed: helper.completed,
        })
    }
}

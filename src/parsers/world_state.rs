/// Parsers for the worldState.php
///
use crate::parsers::{CetusCycle, Fissure, FissureTier, TennoParser};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use serde_with::formats::Flexible;
use serde_with::{serde_as, TimestampMilliSeconds};

pub struct WorldState {}

impl TennoParser for WorldState {
    /// Parse active fissures from the world data.
    /// Takes the full world state data.
    fn parse_fissures(&self, data: &str) -> Vec<Fissure> {
        let v: Value = serde_json::from_str(data).expect("Bad world state file!");

        let _fissures: Vec<_Fissure> =
            serde_json::from_str(&v["ActiveMissions"].to_string()).expect("Deserialize error!");
        let _storms: Vec<_Fissure> =
            serde_json::from_str(&v["VoidStorms"].to_string()).expect("Deserialize error!");

        let mut fissures = _fissures
            .iter()
            .map(|f| Fissure {
                expiry: f.expiry,
                node: self.get_solar_node_by_key(&f.node),
                mission: f.mission_type.clone().unwrap().to_string(),
                tier: FissureTier::from_str(&f.modifier.to_string()),
                is_storm: false,
            })
            .collect::<Vec<Fissure>>();

        let mut storms = _storms
            .iter()
            .map(|f| Fissure {
                expiry: f.expiry,
                node: self.get_solar_node_by_key(&f.node),
                mission: self
                    .get_solar_node_by_key(&f.node)
                    .node_type
                    .unwrap_or_else(|| "Unknown".to_string()),
                tier: FissureTier::from_str(&f.modifier.to_string()),
                is_storm: true,
            })
            .collect::<Vec<Fissure>>();

        fissures.append(&mut storms);
        fissures.sort_by_key(|f| f.tier.clone());

        fissures
    }

    /// Parse the cetus data from the world data.
    /// Takes the full world state data.
    fn parse_cetus_cycle(&self, data: &str) -> CetusCycle {
        let v: Value = serde_json::from_str(data).expect("Deserialize error!");

        let syndicates: Vec<_SyndicateMission> =
            serde_json::from_str(&v["SyndicateMissions"].to_string()).expect("Deserialize error!");

        let cetus = syndicates.iter().find(|s| s.tag == "CetusSyndicate");

        CetusCycle {
            expiry: cetus.unwrap().expiry,
        }
    }
}

#[derive(Deserialize, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
#[repr(u8)]
pub enum FissureModifier {
    Unknown,
    VoidT1,
    VoidT2,
    VoidT3,
    VoidT4,
    VoidT5,
}

impl ToString for FissureModifier {
    fn to_string(&self) -> String {
        match self {
            FissureModifier::Unknown => "Unknown".to_string(),
            FissureModifier::VoidT1 => "Lith".to_string(),
            FissureModifier::VoidT2 => "Meso".to_string(),
            FissureModifier::VoidT3 => "Neo".to_string(),
            FissureModifier::VoidT4 => "Axi".to_string(),
            FissureModifier::VoidT5 => "Requiem".to_string(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Debug, Clone)]
pub enum MissionType {
    MT_ARENA,
    MT_ARTIFACT,
    MT_ASSAULT,
    MT_ASSASSINATION,
    MT_CAPTURE,
    MT_DEFENSE,
    MT_DISRUPTION,
    MT_EVACUATION,
    MT_EXCAVATE,
    MT_EXTERMINATION,
    MT_HIVE,
    MT_INTEL,
    MT_LANDSCAPE,
    MT_MOBILE_DEFENSE,
    MT_PVP,
    MT_RESCUE,
    MT_RETRIEVAL,
    MT_SABOTAGE,
    MT_SECTOR,
    MT_SURVIVAL,
    MT_TERRITORY,
    MT_DEFAULT,
}

impl ToString for MissionType {
    fn to_string(&self) -> String {
        match self {
            MissionType::MT_ARENA => "Rathuum".to_string(),
            MissionType::MT_ARTIFACT => "Disruption".to_string(),
            MissionType::MT_ASSAULT => "Assault".to_string(),
            MissionType::MT_ASSASSINATION => "Assassination".to_string(),
            MissionType::MT_CAPTURE => "Capture".to_string(),
            MissionType::MT_DEFENSE => "Defense".to_string(),
            MissionType::MT_DISRUPTION => "Disruption".to_string(),
            MissionType::MT_EVACUATION => "Defection".to_string(),
            MissionType::MT_EXCAVATE => "Excavation".to_string(),
            MissionType::MT_EXTERMINATION => "Extermination".to_string(),
            MissionType::MT_HIVE => "Hive".to_string(),
            MissionType::MT_INTEL => "Spy".to_string(),
            MissionType::MT_LANDSCAPE => "Free Roam".to_string(),
            MissionType::MT_MOBILE_DEFENSE => "Mobile Defense".to_string(),
            MissionType::MT_PVP => "Conclave".to_string(),
            MissionType::MT_RESCUE => "Rescue".to_string(),
            MissionType::MT_RETRIEVAL => "Hijack".to_string(),
            MissionType::MT_SABOTAGE => "Sabotage".to_string(),
            MissionType::MT_SECTOR => "Dark Sector".to_string(),
            MissionType::MT_SURVIVAL => "Survival".to_string(),
            MissionType::MT_TERRITORY => "Interception".to_string(),
            MissionType::MT_DEFAULT => "Unknown".to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SolarNode {
    pub value: String,
    pub enemy: Option<String>,
    #[serde(alias = "type")]
    pub _type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct _Fissure {
    pub region: Option<u8>,
    pub mission_type: Option<MissionType>,
    pub node: String,
    pub modifier: FissureModifier,
    pub expiry: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for _Fissure {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Outer {
            region: Option<u8>,
            mission_type: Option<MissionType>,
            node: String,
            #[serde(alias = "ActiveMissionTier")]
            modifier: FissureModifier,
            expiry: Inner,
        }

        #[derive(Deserialize)]
        struct Inner {
            #[serde(alias = "$date")]
            date: InnerInner,
        }

        #[serde_as]
        #[derive(Deserialize)]
        struct InnerInner {
            #[serde(alias = "$numberLong")]
            #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
            datetime: DateTime<Utc>,
        }

        let helper = Outer::deserialize(deserializer)?;
        Ok(_Fissure {
            region: helper.region,
            node: helper.node,
            mission_type: helper.mission_type,
            expiry: helper.expiry.date.datetime,
            modifier: helper.modifier,
        })
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct _SyndicateJobs {
    job_type: Option<String>,
    pub rewards: String,
    mastery_req: u8,
    min_enemy_level: i16,
    max_enemy_level: i16,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct _SyndicateMission {
    pub tag: String,
    pub jobs: Option<Vec<_SyndicateJobs>>,
    pub activation: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
}

impl<'de> Deserialize<'de> for _SyndicateMission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Outer {
            tag: String,
            jobs: Option<Vec<_SyndicateJobs>>,
            activation: Inner,
            expiry: Inner,
        }

        #[derive(Deserialize)]
        struct Inner {
            #[serde(alias = "$date")]
            date: InnerInner,
        }

        #[serde_as]
        #[derive(Deserialize)]
        struct InnerInner {
            #[serde(alias = "$numberLong")]
            #[serde_as(as = "TimestampMilliSeconds<String, Flexible>")]
            datetime: DateTime<Utc>,
        }

        let helper = Outer::deserialize(deserializer)?;
        Ok(_SyndicateMission {
            tag: helper.tag,
            jobs: helper.jobs,
            activation: helper.activation.date.datetime,
            expiry: helper.expiry.date.datetime,
        })
    }
}

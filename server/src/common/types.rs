use chrono_tz::{
    Europe::{Helsinki, Lisbon, Stockholm},
    Tz,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const CENTRAL: Tz = Stockholm;
const EASTERN: Tz = Helsinki;
const WESTERN: Tz = Lisbon;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum BiddingZone {
    FI,
    SE,
    SP,
    UK,
    AT,
    PT,
    DE,
    NL,
    ES,
    CH,
    DK,
}

impl BiddingZone {
    pub fn to_tz(&self) -> Tz {
        match &self {
            BiddingZone::FI => EASTERN,
            BiddingZone::PT => WESTERN,
            BiddingZone::UK => WESTERN,
            _ => CENTRAL,
        }
    }
}

use std::fmt;

use chrono_tz::{
    Europe::{Helsinki, Lisbon, Stockholm},
    Tz,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use strum_macros::EnumIter;

const CENTRAL: Tz = Stockholm;
const EASTERN: Tz = Helsinki;
const WESTERN: Tz = Lisbon;

#[derive(Debug, Deserialize, Serialize, ToSchema, EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum BiddingZone {
    FI,
    SE1,
    SE2,
    SE3,
    SE4,
    DK1,
    DK2,
    AT,
    PT,
    NL,
    ES,
    CH,
}

impl BiddingZone {
    pub fn to_tz(&self) -> Tz {
        match &self {
            BiddingZone::FI => EASTERN,
            BiddingZone::PT => WESTERN,
            _ => CENTRAL,
        }
    }
}

impl fmt::Display for BiddingZone {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

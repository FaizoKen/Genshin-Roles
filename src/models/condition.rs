use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConditionField {
    Level,
    WorldLevel,
    FinishAchievementNum,
    SpiralAbyss,
    FetterCount,
    Region,
    HasAvatar,
    HasNameCard,
}

impl ConditionField {
    pub fn is_numeric(&self) -> bool {
        !matches!(self, Self::Region | Self::HasAvatar | Self::HasNameCard | Self::SpiralAbyss)
    }

    pub fn json_key(&self) -> &'static str {
        match self {
            Self::Level => "level",
            Self::WorldLevel => "worldLevel",
            Self::FinishAchievementNum => "finishAchievementNum",
            Self::SpiralAbyss => "spiralAbyss",
            Self::FetterCount => "fetterCount",
            Self::Region => "region",
            Self::HasAvatar => "hasAvatar",
            Self::HasNameCard => "hasNameCard",
        }
    }

    /// Returns the PostgreSQL column name for extracted fields,
    /// or None for fields that require JSONB queries (HasAvatar, HasNameCard).
    pub fn sql_column(&self) -> Option<&'static str> {
        match self {
            Self::Level => Some("pc.level"),
            Self::WorldLevel => Some("pc.world_level"),
            Self::FinishAchievementNum => Some("pc.achievements"),
            Self::SpiralAbyss => Some("pc.abyss_progress"),
            Self::FetterCount => Some("pc.fetter_count"),
            Self::Region => Some("pc.region"),
            Self::HasAvatar | Self::HasNameCard => None,
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "level" => Some(Self::Level),
            "worldLevel" => Some(Self::WorldLevel),
            "finishAchievementNum" => Some(Self::FinishAchievementNum),
            "spiralAbyss" => Some(Self::SpiralAbyss),
            "fetterCount" => Some(Self::FetterCount),
            "region" => Some(Self::Region),
            "hasAvatar" => Some(Self::HasAvatar),
            "hasNameCard" => Some(Self::HasNameCard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConditionOperator {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl ConditionOperator {
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "eq" => Some(Self::Eq),
            "gt" => Some(Self::Gt),
            "gte" => Some(Self::Gte),
            "lt" => Some(Self::Lt),
            "lte" => Some(Self::Lte),
            _ => None,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Gt => "gt",
            Self::Gte => "gte",
            Self::Lt => "lt",
            Self::Lte => "lte",
        }
    }

    pub fn sql_operator(&self) -> &'static str {
        match self {
            Self::Eq => "=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::Lt => "<",
            Self::Lte => "<=",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: ConditionField,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

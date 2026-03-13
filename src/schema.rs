use serde_json::{json, Value};
use std::collections::HashMap;

use crate::error::AppError;
use crate::models::condition::{Condition, ConditionField, ConditionOperator};

pub fn build_config_schema(conditions: &[Condition], verify_url: &str, players_url: &str) -> Value {
    let c = conditions.first();

    let mut values = HashMap::new();
    values.insert(
        "field".to_string(),
        json!(c.map(|c| c.field.json_key()).unwrap_or("")),
    );
    values.insert(
        "operator".to_string(),
        json!(c.map(|c| c.operator.key()).unwrap_or("")),
    );

    if let Some(c) = c {
        let value_key = format!("value_{}", c.field.json_key());
        let val = match c.field {
            ConditionField::SpiralAbyss => {
                let n = c.value.as_i64().unwrap_or(0);
                json!(format!("{}-{}", n / 10, n % 10))
            }
            ConditionField::Region => c.value.clone(),
            _ => match &c.value {
                Value::Number(n) => json!(n),
                Value::String(s) => json!(s),
                _ => json!(""),
            },
        };
        values.insert(value_key, val);

        // Populate end value for Between operator
        if c.operator == ConditionOperator::Between {
            if let Some(end) = &c.value_end {
                let end_key = format!("value_end_{}", c.field.json_key());
                let end_val = match c.field {
                    ConditionField::SpiralAbyss => {
                        let n = end.as_i64().unwrap_or(0);
                        json!(format!("{}-{}", n / 10, n % 10))
                    }
                    _ => match end {
                        Value::Number(n) => json!(n),
                        Value::String(s) => json!(s),
                        _ => json!(""),
                    },
                };
                values.insert(end_key, end_val);
            }
        }
    }

    json!({
        "version": 1,
        "name": "Genshin Roles",
        "description": "Assign Discord roles based on Genshin Impact player stats.",
        "sections": [
            {
                "title": "Getting Started",
                "fields": [
                    {
                        "type": "display",
                        "key": "info",
                        "label": "How it works",
                        "value": format!(
                            "This plugin automatically assigns a Discord role to members \
                             based on their Genshin Impact account stats.\n\
                             \n\
                             Step 1 → Members verify by linking their Genshin UID at:\n\
                             {verify_url}\n\
                             \n\
                             Step 2 → You configure a condition below (e.g. Adventure Rank >= 50).\n\
                             \n\
                             Step 3 → Any verified member who meets the condition gets this role automatically. \
                             Player data is refreshed periodically so roles stay up to date.\n\
                             \n\
                             See all verified members for this server:\n\
                             {players_url}"
                        )
                    }
                ]
            },
            {
                "title": "Role Condition",
                "description": "Set the requirement a player must meet to earn this role.",
                "fields": [
                    {
                        "type": "select",
                        "key": "field",
                        "label": "Player stat",
                        "description": "Which player stat to check.",
                        "validation": { "required": true },
                        "options": [
                            {"label": "Adventure Rank", "value": "level"},
                            {"label": "World Level", "value": "worldLevel"},
                            {"label": "Achievements Completed", "value": "finishAchievementNum"},
                            {"label": "Spiral Abyss Progress", "value": "spiralAbyss"},
                            {"label": "Spiral Abyss Stars", "value": "towerStarIndex"},
                            {"label": "Server Region", "value": "region"},
                            {"label": "Showcased Character", "value": "hasAvatar"},
                            {"label": "Showcased Namecard", "value": "hasNameCard"}
                        ]
                    },
                    {
                        "type": "select",
                        "key": "operator",
                        "label": "Comparison",
                        "default_value": "eq",
                        "condition": { "field": "field", "equals_any": ["level", "worldLevel", "finishAchievementNum", "spiralAbyss", "towerStarIndex"] },
                        "options": [
                            {"label": "= equals", "value": "eq"},
                            {"label": "> greater than", "value": "gt"},
                            {"label": ">= at least", "value": "gte"},
                            {"label": "< less than", "value": "lt"},
                            {"label": "<= at most", "value": "lte"},
                            {"label": "↔ between (range)", "value": "between"}
                        ]
                    },
                    {
                        "type": "number",
                        "key": "value_level",
                        "label": "Adventure Rank",
                        "description": "1–60",
                        "validation": { "min": 1, "max": 60 },
                        "condition": { "field": "field", "equals": "level" }
                    },
                    {
                        "type": "number",
                        "key": "value_end_level",
                        "label": "Adventure Rank (end)",
                        "validation": { "min": 1, "max": 60 },
                        "pair_with": "value_level",
                        "conditions": [
                            { "field": "field", "equals": "level" },
                            { "field": "operator", "equals": "between" }
                        ]
                    },
                    {
                        "type": "number",
                        "key": "value_worldLevel",
                        "label": "World Level",
                        "description": "0–8",
                        "validation": { "min": 0, "max": 8 },
                        "condition": { "field": "field", "equals": "worldLevel" }
                    },
                    {
                        "type": "number",
                        "key": "value_end_worldLevel",
                        "label": "World Level (end)",
                        "validation": { "min": 0, "max": 8 },
                        "pair_with": "value_worldLevel",
                        "conditions": [
                            { "field": "field", "equals": "worldLevel" },
                            { "field": "operator", "equals": "between" }
                        ]
                    },
                    {
                        "type": "number",
                        "key": "value_finishAchievementNum",
                        "label": "Achievements count",
                        "validation": { "min": 0 },
                        "condition": { "field": "field", "equals": "finishAchievementNum" }
                    },
                    {
                        "type": "number",
                        "key": "value_end_finishAchievementNum",
                        "label": "Achievements (end)",
                        "validation": { "min": 0 },
                        "pair_with": "value_finishAchievementNum",
                        "conditions": [
                            { "field": "field", "equals": "finishAchievementNum" },
                            { "field": "operator", "equals": "between" }
                        ]
                    },
                    {
                        "type": "text",
                        "key": "value_spiralAbyss",
                        "label": "Floor-Chamber",
                        "description": "e.g. 12-3 (floor 1–12, chamber 1–3)",
                        "validation": { "pattern": "^\\d{1,2}-[1-3]$", "pattern_message": "Use floor-chamber format, e.g. 12-3" },
                        "condition": { "field": "field", "equals": "spiralAbyss" }
                    },
                    {
                        "type": "text",
                        "key": "value_end_spiralAbyss",
                        "label": "Floor-Chamber (end)",
                        "validation": { "pattern": "^\\d{1,2}-[1-3]$", "pattern_message": "Use floor-chamber format, e.g. 12-3" },
                        "pair_with": "value_spiralAbyss",
                        "conditions": [
                            { "field": "field", "equals": "spiralAbyss" },
                            { "field": "operator", "equals": "between" }
                        ]
                    },
                    {
                        "type": "number",
                        "key": "value_towerStarIndex",
                        "label": "Spiral Abyss Stars",
                        "description": "0–36",
                        "validation": { "min": 0, "max": 36 },
                        "condition": { "field": "field", "equals": "towerStarIndex" }
                    },
                    {
                        "type": "number",
                        "key": "value_end_towerStarIndex",
                        "label": "Spiral Abyss Stars (end)",
                        "validation": { "min": 0, "max": 36 },
                        "pair_with": "value_towerStarIndex",
                        "conditions": [
                            { "field": "field", "equals": "towerStarIndex" },
                            { "field": "operator", "equals": "between" }
                        ]
                    },
                    {
                        "type": "select",
                        "key": "value_region",
                        "label": "Server Region",
                        "options": [
                            {"label": "North America (NA)", "value": "NA"},
                            {"label": "Europe (EU)", "value": "EU"},
                            {"label": "Asia", "value": "ASIA"},
                            {"label": "Taiwan / HK / Macao (TW)", "value": "TW"},
                            {"label": "China (CN)", "value": "CN"}
                        ],
                        "condition": { "field": "field", "equals": "region" }
                    },
                    {
                        "type": "number",
                        "key": "value_hasAvatar",
                        "label": "Avatar ID",
                        "description": "Go to https://gi.yatta.moe/en/archive/avatar , pick a character, and copy the ID from avatar/:id in the URL.",
                        "validation": { "min": 1 },
                        "condition": { "field": "field", "equals": "hasAvatar" }
                    },
                    {
                        "type": "number",
                        "key": "value_hasNameCard",
                        "label": "Namecard ID",
                        "description": "Go to https://gi.yatta.moe/en/archive/namecard , pick a namecard, and copy the ID from namecard/:id in the URL.",
                        "validation": { "min": 1 },
                        "condition": { "field": "field", "equals": "hasNameCard" }
                    }
                ]
            },
            {
                "title": "Examples",
                "collapsible": true,
                "default_collapsed": true,
                "fields": [
                    {
                        "type": "display",
                        "key": "examples",
                        "label": "Common setups",
                        "value": "Adventure Rank >= 50  →  AR 50 and above\nWorld Level = 8  →  Max world level only\nAchievements >= 500  →  500+ achievements\nSpiral Abyss >= 12-3  →  Cleared floor 12 chamber 3\nSpiral Abyss Stars >= 33  →  33+ stars in current Abyss cycle\nRegion = Asia  →  Asia server players\nAdventure Rank between 30 to 55  →  AR 30 to 55 (inclusive)"
                    }
                ]
            }
        ],
        "values": values
    })
}

pub fn parse_config(config: &HashMap<String, Value>) -> Result<Vec<Condition>, AppError> {
    let field_key = config
        .get("field")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if field_key.is_empty() {
        return Err(AppError::BadRequest("Field is required".into()));
    }

    let field = ConditionField::from_key(field_key)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid field '{field_key}'")))?;

    let op_key = config
        .get("operator")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let operator = if op_key.is_empty() {
        // Operator is hidden in the UI for fields that only support equals
        match field {
            ConditionField::Region | ConditionField::HasAvatar | ConditionField::HasNameCard => {
                ConditionOperator::Eq
            }
            _ => return Err(AppError::BadRequest("Operator is required".into())),
        }
    } else {
        ConditionOperator::from_key(op_key)
            .ok_or_else(|| AppError::BadRequest(format!("Invalid operator '{op_key}'")))?
    };

    // Prefer field-specific value key (e.g. value_level), fall back to generic "value"
    let specific_key = format!("value_{field_key}");
    let raw_value = config
        .get(&specific_key)
        .or_else(|| config.get("value"));

    let value_str = raw_value
        .and_then(|v| match v {
            Value::String(s) => Some(s.as_str()),
            Value::Number(n) => Some(n.as_i64().map(|_| "").unwrap_or("")),
            _ => None,
        })
        .unwrap_or("");

    // For number fields the platform sends the value as a JSON number
    let value_num = raw_value.and_then(|v| v.as_i64());

    if value_str.is_empty() && value_num.is_none() {
        return Err(AppError::BadRequest("Value is required".into()));
    }

    if matches!(field, ConditionField::Region) && operator != ConditionOperator::Eq {
        return Err(AppError::BadRequest(
            "Region only supports '= (equals)' condition".into(),
        ));
    }

    if operator == ConditionOperator::Between
        && matches!(field, ConditionField::Region | ConditionField::HasAvatar | ConditionField::HasNameCard | ConditionField::FetterCount)
    {
        return Err(AppError::BadRequest(
            "Between is not supported for this field".into(),
        ));
    }

    let value = if matches!(field, ConditionField::SpiralAbyss) {
        let s = value_str;
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(AppError::BadRequest(
                "Spiral Abyss value must be in floor-chamber format (e.g. 12-3)".into(),
            ));
        }
        let floor: i64 = parts[0].trim().parse().map_err(|_| {
            AppError::BadRequest("Floor must be a number (e.g. 12-3)".into())
        })?;
        let chamber: i64 = parts[1].trim().parse().map_err(|_| {
            AppError::BadRequest("Chamber must be a number (e.g. 12-3)".into())
        })?;
        if !(1..=12).contains(&floor) || !(1..=3).contains(&chamber) {
            return Err(AppError::BadRequest(
                "Floor must be 1-12 and chamber must be 1-3".into(),
            ));
        }
        serde_json::Value::Number((floor * 10 + chamber).into())
    } else if matches!(field, ConditionField::Region) {
        let v = value_str.to_uppercase();
        if !["NA", "EU", "ASIA", "TW", "CN"].contains(&v.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid region '{value_str}'. Use NA, EU, ASIA, TW, or CN"
            )));
        }
        serde_json::Value::String(v)
    } else {
        // Numeric fields: accept both JSON number and string
        let n = value_num.or_else(|| value_str.parse::<i64>().ok()).ok_or_else(|| {
            AppError::BadRequest(format!("Value must be a number for '{field_key}'"))
        })?;
        serde_json::Value::Number(n.into())
    };

    // Parse end value for Between operator
    let value_end = if operator == ConditionOperator::Between {
        let end_specific_key = format!("value_end_{field_key}");
        let raw_end = config
            .get(&end_specific_key)
            .or_else(|| config.get("value_end"));

        let end_str = raw_end
            .and_then(|v| match v {
                Value::String(s) => Some(s.as_str()),
                Value::Number(n) => Some(n.as_i64().map(|_| "").unwrap_or("")),
                _ => None,
            })
            .unwrap_or("");
        let end_num = raw_end.and_then(|v| v.as_i64());

        if end_str.is_empty() && end_num.is_none() {
            return Err(AppError::BadRequest("End value is required for between operator".into()));
        }

        let end_val = if matches!(field, ConditionField::SpiralAbyss) {
            let parts: Vec<&str> = end_str.split('-').collect();
            if parts.len() != 2 {
                return Err(AppError::BadRequest(
                    "Spiral Abyss end value must be in floor-chamber format (e.g. 12-3)".into(),
                ));
            }
            let floor: i64 = parts[0].trim().parse().map_err(|_| {
                AppError::BadRequest("End floor must be a number (e.g. 12-3)".into())
            })?;
            let chamber: i64 = parts[1].trim().parse().map_err(|_| {
                AppError::BadRequest("End chamber must be a number (e.g. 12-3)".into())
            })?;
            if !(1..=12).contains(&floor) || !(1..=3).contains(&chamber) {
                return Err(AppError::BadRequest(
                    "End floor must be 1-12 and chamber must be 1-3".into(),
                ));
            }
            serde_json::Value::Number((floor * 10 + chamber).into())
        } else {
            let n = end_num.or_else(|| end_str.parse::<i64>().ok()).ok_or_else(|| {
                AppError::BadRequest(format!("End value must be a number for '{field_key}'"))
            })?;
            serde_json::Value::Number(n.into())
        };

        // Validate start <= end
        if let (Some(start), Some(end)) = (value.as_i64(), end_val.as_i64()) {
            if start > end {
                return Err(AppError::BadRequest(
                    "Start value must be less than or equal to end value".into(),
                ));
            }
        }

        Some(end_val)
    } else {
        None
    };

    Ok(vec![Condition {
        field,
        operator,
        value,
        value_end,
    }])
}
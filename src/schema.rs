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
                        "required": true,
                        "options": [
                            {"label": "Adventure Rank", "value": "level"},
                            {"label": "World Level", "value": "worldLevel"},
                            {"label": "Achievements Completed", "value": "finishAchievementNum"},
                            {"label": "Spiral Abyss Progress", "value": "spiralAbyss"},
                            {"label": "Max Friendship Characters", "value": "fetterCount"},
                            {"label": "Server Region", "value": "region"},
                            {"label": "Showcased Character", "value": "hasAvatar"},
                            {"label": "Showcased Namecard", "value": "hasNameCard"}
                        ]
                    },
                    {
                        "type": "select",
                        "key": "operator",
                        "label": "Comparison",
                        "description": "How to compare the value. Region only supports equals (=).",
                        "required": true,
                        "options": [
                            {"label": "= equals", "value": "eq"},
                            {"label": "> greater than", "value": "gt"},
                            {"label": ">= at least", "value": "gte"},
                            {"label": "< less than", "value": "lt"},
                            {"label": "<= at most", "value": "lte"}
                        ]
                    },
                    {
                        "type": "number",
                        "key": "value_level",
                        "label": "Adventure Rank",
                        "description": "AR threshold (1–60).",
                        "required": true,
                        "min": 1,
                        "max": 60,
                        "condition": { "field": "field", "equals": "level" }
                    },
                    {
                        "type": "number",
                        "key": "value_worldLevel",
                        "label": "World Level",
                        "required": true,
                        "min": 0,
                        "max": 8,
                        "condition": { "field": "field", "equals": "worldLevel" }
                    },
                    {
                        "type": "number",
                        "key": "value_finishAchievementNum",
                        "label": "Achievements count",
                        "description": "Minimum number of achievements completed.",
                        "required": true,
                        "min": 0,
                        "condition": { "field": "field", "equals": "finishAchievementNum" }
                    },
                    {
                        "type": "text",
                        "key": "value_spiralAbyss",
                        "label": "Floor-Chamber",
                        "description": "Floor (1–12) and chamber (1–3) separated by dash, e.g. 12-3.",
                        "required": true,
                        "pattern": "^\\d{1,2}-[1-3]$",
                        "pattern_message": "Use floor-chamber format, e.g. 12-3",
                        "condition": { "field": "field", "equals": "spiralAbyss" }
                    },
                    {
                        "type": "number",
                        "key": "value_fetterCount",
                        "label": "Character count",
                        "description": "Number of characters at max friendship.",
                        "required": true,
                        "min": 0,
                        "condition": { "field": "field", "equals": "fetterCount" }
                    },
                    {
                        "type": "select",
                        "key": "value_region",
                        "label": "Server Region",
                        "required": true,
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
                        "required": true,
                        "min": 1,
                        "condition": { "field": "field", "equals": "hasAvatar" }
                    },
                    {
                        "type": "number",
                        "key": "value_hasNameCard",
                        "label": "Namecard ID",
                        "description": "Go to https://gi.yatta.moe/en/archive/namecard , pick a namecard, and copy the ID from namecard/:id in the URL.",
                        "required": true,
                        "min": 1,
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
                        "value": "Adventure Rank >= 50  →  AR 50 and above\nWorld Level = 8  →  Max world level only\nAchievements >= 500  →  500+ achievements\nSpiral Abyss >= 12-3  →  Cleared floor 12 chamber 3\nRegion = Asia  →  Asia server players"
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

    if op_key.is_empty() {
        return Err(AppError::BadRequest("Operator is required".into()));
    }

    let operator = ConditionOperator::from_key(op_key)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid operator '{op_key}'")))?;

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

    Ok(vec![Condition {
        field,
        operator,
        value,
    }])
}

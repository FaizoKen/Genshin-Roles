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
    values.insert(
        "value".to_string(),
        match c {
            Some(c) if matches!(c.field, ConditionField::SpiralAbyss) => {
                let n = c.value.as_i64().unwrap_or(0);
                json!(format!("{}-{}", n / 10, n % 10))
            }
            Some(Condition { value: Value::Number(n), .. }) => json!(n.to_string()),
            Some(Condition { value: Value::String(s), .. }) => json!(s),
            _ => json!(""),
        },
    );

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
                            "1. Members link their Genshin account at: {verify_url}\n\
                             2. Set a condition below (e.g. Adventure Rank >= 50)\n\
                             3. Members who meet the condition automatically receive this role\n\
                             4. Player data is refreshed periodically to keep roles up to date\n\
                             \n\
                             View all verified players: {players_url}"
                        )
                    }
                ]
            },
            {
                "title": "Role Condition",
                "description": "Choose what a player needs to receive this role. Example: Adventure Rank >= 50 means players at AR 50 or above get the role.",
                "fields": [
                    {
                        "type": "select",
                        "key": "field",
                        "label": "What to check",
                        "description": "Which player stat to evaluate.",
                        "options": [
                            {"label": "Adventure Rank", "value": "level"},
                            {"label": "World Level (0-8)", "value": "worldLevel"},
                            {"label": "Achievements Completed", "value": "finishAchievementNum"},
                            {"label": "Spiral Abyss Progress (e.g. 12-3)", "value": "spiralAbyss"},
                            {"label": "Max Friendship Characters", "value": "fetterCount"},
                            {"label": "Server Region (NA, EU, ASIA, TW, CN)", "value": "region"},
                            {"label": "Showcased Character (by Avatar ID)", "value": "hasAvatar"},
                            {"label": "Showcased Namecard (by Namecard ID)", "value": "hasNameCard"}
                        ],
                        "validation": { "required": true }
                    },
                    {
                        "type": "select",
                        "key": "operator",
                        "label": "Condition",
                        "description": "How to compare the player's value.",
                        "options": [
                            {"label": "= (equals)", "value": "eq"},
                            {"label": "> (greater than)", "value": "gt"},
                            {"label": ">= (at least)", "value": "gte"},
                            {"label": "< (less than)", "value": "lt"},
                            {"label": "<= (at most)", "value": "lte"}
                        ],
                        "validation": { "required": true }
                    },
                    {
                        "type": "text",
                        "key": "value",
                        "label": "Value",
                        "description": "Enter a number (e.g. 50 for AR 50). For Spiral Abyss use floor-chamber (e.g. 12-3). For Region: NA, EU, ASIA, TW, or CN.",
                        "placeholder": "e.g. 50 or 12-3",
                        "validation": { "required": true, "max": 100 }
                    }
                ]
            },
            {
                "title": "Examples",
                "fields": [
                    {
                        "type": "display",
                        "key": "examples",
                        "label": "Common setups",
                        "value": "Adventure Rank >= 50  -  AR 50 and above\nWorld Level = 8  -  Max world level only\nAchievements Completed >= 500  -  500+ achievements\nSpiral Abyss >= 12-3  -  Cleared floor 12 chamber 3\nServer Region = ASIA  -  Asia server players"
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

    let value_str = config
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if value_str.is_empty() {
        return Err(AppError::BadRequest("Value is required".into()));
    }

    if matches!(field, ConditionField::Region) && operator != ConditionOperator::Eq {
        return Err(AppError::BadRequest(
            "Region only supports '= (equals)' condition".into(),
        ));
    }

    let value = if matches!(field, ConditionField::SpiralAbyss) {
        let parts: Vec<&str> = value_str.split('-').collect();
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
    } else if field.is_numeric()
        || matches!(field, ConditionField::HasAvatar | ConditionField::HasNameCard)
    {
        let n: i64 = value_str
            .parse()
            .map_err(|_| AppError::BadRequest(format!("Value must be a number for '{field_key}'")))?;
        serde_json::Value::Number(n.into())
    } else {
        let v = value_str.to_uppercase();
        if !["NA", "EU", "ASIA", "TW", "CN"].contains(&v.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid region '{value_str}'. Use NA, EU, ASIA, TW, or CN"
            )));
        }
        serde_json::Value::String(v)
    };

    Ok(vec![Condition {
        field,
        operator,
        value,
    }])
}

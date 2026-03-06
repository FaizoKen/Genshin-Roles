use crate::models::condition::{Condition, ConditionField, ConditionOperator};

/// Evaluate all conditions against player data. All must pass (AND logic).
pub fn evaluate_conditions(
    conditions: &[Condition],
    player_info: &serde_json::Value,
    region: Option<&str>,
) -> bool {
    conditions
        .iter()
        .all(|c| evaluate_single(c, player_info, region))
}

fn evaluate_single(
    condition: &Condition,
    player_info: &serde_json::Value,
    region: Option<&str>,
) -> bool {
    match &condition.field {
        ConditionField::Region => {
            let actual = region.unwrap_or("");
            let expected = condition.value.as_str().unwrap_or("");
            actual.eq_ignore_ascii_case(expected)
        }
        ConditionField::HasAvatar => {
            let target_id = condition.value.as_i64().unwrap_or(0);
            player_info["showAvatarInfoList"]
                .as_array()
                .is_some_and(|list| {
                    list.iter()
                        .any(|a| a["avatarId"].as_i64() == Some(target_id))
                })
        }
        ConditionField::HasNameCard => {
            let target_id = condition.value.as_i64().unwrap_or(0);
            player_info["showNameCardIdList"]
                .as_array()
                .is_some_and(|list| list.iter().any(|id| id.as_i64() == Some(target_id)))
        }
        numeric_field => {
            let field_name = numeric_field.json_key();
            let actual = player_info[field_name].as_i64().unwrap_or(0);
            let expected = condition.value.as_i64().unwrap_or(0);
            match condition.operator {
                ConditionOperator::Eq => actual == expected,
                ConditionOperator::Gt => actual > expected,
                ConditionOperator::Gte => actual >= expected,
                ConditionOperator::Lt => actual < expected,
                ConditionOperator::Lte => actual <= expected,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_player_info() -> serde_json::Value {
        json!({
            "nickname": "TestPlayer",
            "level": 57,
            "worldLevel": 8,
            "finishAchievementNum": 458,
            "towerFloorIndex": 12,
            "towerLevelIndex": 3,
            "fetterCount": 13,
            "showAvatarInfoList": [
                {"avatarId": 10000021, "level": 90},
                {"avatarId": 10000032, "level": 90}
            ],
            "showNameCardIdList": [210051, 210087, 210018]
        })
    }

    #[test]
    fn test_level_gte() {
        let conditions = vec![Condition {
            field: ConditionField::Level,
            operator: ConditionOperator::Gte,
            value: json!(50),
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA")));
    }

    #[test]
    fn test_level_gte_fail() {
        let conditions = vec![Condition {
            field: ConditionField::Level,
            operator: ConditionOperator::Gte,
            value: json!(60),
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA")));
    }

    #[test]
    fn test_region_eq() {
        let conditions = vec![Condition {
            field: ConditionField::Region,
            operator: ConditionOperator::Eq,
            value: json!("NA"),
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA")));
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("EU")));
    }

    #[test]
    fn test_has_avatar() {
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000021),
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None));
    }

    #[test]
    fn test_has_avatar_missing() {
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(99999999),
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), None));
    }

    #[test]
    fn test_has_namecard() {
        let conditions = vec![Condition {
            field: ConditionField::HasNameCard,
            operator: ConditionOperator::Eq,
            value: json!(210051),
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None));
    }

    #[test]
    fn test_multiple_conditions_and() {
        let conditions = vec![
            Condition {
                field: ConditionField::Level,
                operator: ConditionOperator::Gte,
                value: json!(50),
            },
            Condition {
                field: ConditionField::WorldLevel,
                operator: ConditionOperator::Eq,
                value: json!(8),
            },
            Condition {
                field: ConditionField::Region,
                operator: ConditionOperator::Eq,
                value: json!("NA"),
            },
        ];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA")));
    }

    #[test]
    fn test_multiple_conditions_one_fails() {
        let conditions = vec![
            Condition {
                field: ConditionField::Level,
                operator: ConditionOperator::Gte,
                value: json!(50),
            },
            Condition {
                field: ConditionField::Region,
                operator: ConditionOperator::Eq,
                value: json!("EU"),
            },
        ];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA")));
    }

    #[test]
    fn test_missing_field_defaults_zero() {
        let player = json!({"level": 50});
        let conditions = vec![Condition {
            field: ConditionField::TowerFloorIndex,
            operator: ConditionOperator::Gte,
            value: json!(1),
        }];
        assert!(!evaluate_conditions(&conditions, &player, None));
    }

    #[test]
    fn test_empty_conditions_always_true() {
        let conditions: Vec<Condition> = vec![];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None));
    }
}

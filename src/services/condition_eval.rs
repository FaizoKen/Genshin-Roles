use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Utc};

use crate::models::condition::{Condition, ConditionField, ConditionOperator};

/// Evaluate all conditions against player data. All must pass (AND logic).
pub fn evaluate_conditions(
    conditions: &[Condition],
    player_info: &serde_json::Value,
    region: Option<&str>,
    fetched_at: Option<DateTime<Utc>>,
) -> bool {
    conditions
        .iter()
        .all(|c| evaluate_single(c, player_info, region, fetched_at))
}

fn evaluate_single(
    condition: &Condition,
    player_info: &serde_json::Value,
    region: Option<&str>,
    fetched_at: Option<DateTime<Utc>>,
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
                    list.iter().any(|a| {
                        if a["avatarId"].as_i64() != Some(target_id) {
                            return false;
                        }
                        if let Some(min_level) = condition.avatar_level {
                            if a["level"].as_i64().unwrap_or(0) < min_level {
                                return false;
                            }
                        }
                        if let Some(min_const) = condition.avatar_constellation {
                            if a["talentLevel"].as_i64().unwrap_or(0) < min_const {
                                return false;
                            }
                        }
                        true
                    })
                })
        }
        ConditionField::HasNameCard => {
            let target_id = condition.value.as_i64().unwrap_or(0);
            player_info["showNameCardIdList"]
                .as_array()
                .is_some_and(|list| list.iter().any(|id| id.as_i64() == Some(target_id)))
        }
        ConditionField::SpiralAbyss => {
            // Freshness gate: data must be fetched after the most recent Abyss reset
            if let Some(fetched) = fetched_at {
                let last_reset = last_abyss_reset_utc(region.unwrap_or("NA"));
                if fetched < last_reset {
                    return false;
                }
            }
            let floor = player_info["towerFloorIndex"].as_i64().unwrap_or(0);
            let chamber = player_info["towerLevelIndex"].as_i64().unwrap_or(0);
            let actual = floor * 10 + chamber;
            let expected = condition.value.as_i64().unwrap_or(0);
            compare(actual, expected, &condition.operator, &condition.value_end)
        }
        ConditionField::TowerStarIndex => {
            // Freshness gate: resets on the same cycle as Spiral Abyss
            if let Some(fetched) = fetched_at {
                let last_reset = last_abyss_reset_utc(region.unwrap_or("NA"));
                if fetched < last_reset {
                    return false;
                }
            }
            let actual = player_info["towerStarIndex"].as_i64().unwrap_or(0);
            let expected = condition.value.as_i64().unwrap_or(0);
            compare(actual, expected, &condition.operator, &condition.value_end)
        }
        numeric_field => {
            let field_name = numeric_field.json_key();
            let actual = player_info[field_name].as_i64().unwrap_or(0);
            let expected = condition.value.as_i64().unwrap_or(0);
            compare(actual, expected, &condition.operator, &condition.value_end)
        }
    }
}

fn compare(actual: i64, expected: i64, operator: &ConditionOperator, value_end: &Option<serde_json::Value>) -> bool {
    match operator {
        ConditionOperator::Eq => actual == expected,
        ConditionOperator::Gt => actual > expected,
        ConditionOperator::Gte => actual >= expected,
        ConditionOperator::Lt => actual < expected,
        ConditionOperator::Lte => actual <= expected,
        ConditionOperator::Between => {
            let end = value_end.as_ref().and_then(|v| v.as_i64()).unwrap_or(expected);
            actual >= expected && actual <= end
        }
    }
}

/// Calculate the most recent Spiral Abyss reset datetime in UTC.
/// Abyss resets on the 1st and 16th of every month at 04:00 server time.
///
/// Server time offsets from UTC:
/// - NA: UTC-5 → reset at 09:00 UTC
/// - EU: UTC+1 → reset at 03:00 UTC
/// - ASIA/TW/CN: UTC+8 → reset at 20:00 UTC (previous day)
pub fn last_abyss_reset_utc(region: &str) -> DateTime<Utc> {
    let now = Utc::now();

    // Server reset hour in UTC for each region
    // At 04:00 server time:
    // NA (UTC-5): 04:00 + 5 = 09:00 UTC same day
    // EU (UTC+1): 04:00 - 1 = 03:00 UTC same day
    // ASIA/TW/CN (UTC+8): 04:00 - 8 = 20:00 UTC previous day
    let (reset_hour_utc, prev_day) = match region.to_uppercase().as_str() {
        "NA" => (9, false),
        "EU" => (3, false),
        _ => (20, true), // ASIA, TW, CN
    };

    let today = now.date_naive();
    let year = today.year();
    let month = today.month();

    // Determine the two candidate reset dates this month (1st and 16th)
    // and pick the most recent one that's in the past
    let reset_day_1 = if prev_day {
        // For ASIA/TW/CN, the reset at 04:00 server time = 20:00 UTC on the previous day
        // So "1st at 04:00 ASIA" = Dec 31 20:00 UTC, "16th at 04:00 ASIA" = 15th 20:00 UTC
        naive_date_safe(year, month, 15)
    } else {
        naive_date_safe(year, month, 16)
    };
    let reset_day_0 = if prev_day {
        // Last day of previous month
        let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        first_of_month.pred_opt().unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month, 1).unwrap()
    };

    let reset_time = NaiveTime::from_hms_opt(reset_hour_utc, 0, 0).unwrap();

    let candidate_1 = NaiveDateTime::new(reset_day_1, reset_time)
        .and_utc();
    let candidate_0 = NaiveDateTime::new(reset_day_0, reset_time)
        .and_utc();

    if now >= candidate_1 {
        candidate_1
    } else if now >= candidate_0 {
        candidate_0
    } else {
        // Before the first reset of this month — use the 16th reset of the previous month
        let prev_reset_day = if prev_day {
            let prev_month_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                .pred_opt().unwrap(); // last day of prev month
            NaiveDate::from_ymd_opt(prev_month_date.year(), prev_month_date.month(), 15).unwrap()
        } else {
            let prev_month_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap()
                .pred_opt().unwrap();
            NaiveDate::from_ymd_opt(prev_month_date.year(), prev_month_date.month(), 16).unwrap()
        };
        NaiveDateTime::new(prev_reset_day, reset_time).and_utc()
    }
}

fn naive_date_safe(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
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
            "towerStarIndex": 36,
            "fetterCount": 13,
            "showAvatarInfoList": [
                {"avatarId": 10000021, "level": 90, "talentLevel": 2},
                {"avatarId": 10000032, "level": 80}
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
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_level_gte_fail() {
        let conditions = vec![Condition {
            field: ConditionField::Level,
            operator: ConditionOperator::Gte,
            value: json!(60),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_region_eq() {
        let conditions = vec![Condition {
            field: ConditionField::Region,
            operator: ConditionOperator::Eq,
            value: json!("NA"),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("EU"), None));
    }

    #[test]
    fn test_has_avatar() {
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000021),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_missing() {
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(99999999),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_namecard() {
        let conditions = vec![Condition {
            field: ConditionField::HasNameCard,
            operator: ConditionOperator::Eq,
            value: json!(210051),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_multiple_conditions_and() {
        let conditions = vec![
            Condition {
                field: ConditionField::Level,
                operator: ConditionOperator::Gte,
                value: json!(50),
                value_end: None,
                avatar_level: None,
                avatar_constellation: None,
            },
            Condition {
                field: ConditionField::WorldLevel,
                operator: ConditionOperator::Eq,
                value: json!(8),
                value_end: None,
                avatar_level: None,
                avatar_constellation: None,
            },
            Condition {
                field: ConditionField::Region,
                operator: ConditionOperator::Eq,
                value: json!("NA"),
                value_end: None,
                avatar_level: None,
                avatar_constellation: None,
            },
        ];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_multiple_conditions_one_fails() {
        let conditions = vec![
            Condition {
                field: ConditionField::Level,
                operator: ConditionOperator::Gte,
                value: json!(50),
                value_end: None,
                avatar_level: None,
                avatar_constellation: None,
            },
            Condition {
                field: ConditionField::Region,
                operator: ConditionOperator::Eq,
                value: json!("EU"),
                value_end: None,
                avatar_level: None,
                avatar_constellation: None,
            },
        ];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_spiral_abyss_combined() {
        // Player has floor 12, chamber 3 → progress = 123
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Gte,
            value: json!(121), // 12-1
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        // No fetched_at → freshness check skipped
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_spiral_abyss_exact() {
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Eq,
            value: json!(123), // 12-3
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_spiral_abyss_fail() {
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Gte,
            value: json!(124), // higher than 12-3
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_spiral_abyss_stale_data() {
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Gte,
            value: json!(121),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        // fetched_at far in the past → stale → should fail
        let old_date = DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(!evaluate_conditions(
            &conditions,
            &sample_player_info(),
            Some("NA"),
            Some(old_date),
        ));
    }

    #[test]
    fn test_spiral_abyss_fresh_data() {
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Gte,
            value: json!(121),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        // fetched_at = now → fresh
        let now = Utc::now();
        assert!(evaluate_conditions(
            &conditions,
            &sample_player_info(),
            Some("NA"),
            Some(now),
        ));
    }

    #[test]
    fn test_tower_star_index_gte() {
        let conditions = vec![Condition {
            field: ConditionField::TowerStarIndex,
            operator: ConditionOperator::Gte,
            value: json!(30),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_tower_star_index_fail() {
        let conditions = vec![Condition {
            field: ConditionField::TowerStarIndex,
            operator: ConditionOperator::Gt,
            value: json!(36),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_tower_star_index_stale_data() {
        let conditions = vec![Condition {
            field: ConditionField::TowerStarIndex,
            operator: ConditionOperator::Gte,
            value: json!(30),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        let old_date = DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(!evaluate_conditions(
            &conditions,
            &sample_player_info(),
            Some("NA"),
            Some(old_date),
        ));
    }

    #[test]
    fn test_missing_abyss_fields_defaults_zero() {
        let player = json!({"level": 50});
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Gte,
            value: json!(1),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &player, None, None));
    }

    #[test]
    fn test_between_level_in_range() {
        // Player level is 57, range 50-60 should pass
        let conditions = vec![Condition {
            field: ConditionField::Level,
            operator: ConditionOperator::Between,
            value: json!(50),
            value_end: Some(json!(60)),
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_between_level_out_of_range() {
        // Player level is 57, range 58-60 should fail
        let conditions = vec![Condition {
            field: ConditionField::Level,
            operator: ConditionOperator::Between,
            value: json!(58),
            value_end: Some(json!(60)),
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_between_level_exact_boundary() {
        // Player level is 57, range 57-57 should pass (inclusive)
        let conditions = vec![Condition {
            field: ConditionField::Level,
            operator: ConditionOperator::Between,
            value: json!(57),
            value_end: Some(json!(57)),
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_between_spiral_abyss() {
        // Player has 12-3 (123), range 12-1 (121) to 12-3 (123) should pass
        let conditions = vec![Condition {
            field: ConditionField::SpiralAbyss,
            operator: ConditionOperator::Between,
            value: json!(121),
            value_end: Some(json!(123)),
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), Some("NA"), None));
    }

    #[test]
    fn test_empty_conditions_always_true() {
        let conditions: Vec<Condition> = vec![];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_last_abyss_reset_returns_past() {
        let reset = last_abyss_reset_utc("NA");
        assert!(reset <= Utc::now());
    }

    #[test]
    fn test_has_avatar_with_level() {
        // Avatar 10000021 has level 90 → level >= 90 should pass
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000021),
            value_end: None,
            avatar_level: Some(90),
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_with_level_fail() {
        // Avatar 10000032 has level 80 → level >= 90 should fail
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000032),
            value_end: None,
            avatar_level: Some(90),
            avatar_constellation: None,
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_with_constellation() {
        // Avatar 10000021 has talentLevel 2 → constellation >= 2 should pass
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000021),
            value_end: None,
            avatar_level: None,
            avatar_constellation: Some(2),
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_with_constellation_fail() {
        // Avatar 10000021 has talentLevel 2 → constellation >= 3 should fail
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000021),
            value_end: None,
            avatar_level: None,
            avatar_constellation: Some(3),
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_with_level_and_constellation() {
        // Avatar 10000021: level 90, talentLevel 2 → both pass
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000021),
            value_end: None,
            avatar_level: Some(90),
            avatar_constellation: Some(2),
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_missing_constellation_defaults_zero() {
        // Avatar 10000032 has no talentLevel → defaults to 0 → constellation >= 1 should fail
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000032),
            value_end: None,
            avatar_level: None,
            avatar_constellation: Some(1),
        }];
        assert!(!evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }

    #[test]
    fn test_has_avatar_no_sub_filters() {
        // Just avatar ID, no sub-filters — should work as before
        let conditions = vec![Condition {
            field: ConditionField::HasAvatar,
            operator: ConditionOperator::Eq,
            value: json!(10000032),
            value_end: None,
            avatar_level: None,
            avatar_constellation: None,
        }];
        assert!(evaluate_conditions(&conditions, &sample_player_info(), None, None));
    }
}

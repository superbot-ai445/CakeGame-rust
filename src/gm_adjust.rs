/// GM属性调整系统
/// 使用 editor_attribute_adjustment 表 (uId, aName, aValue)
/// 允许 GM 对玩家属性进行永久调整
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 可调整的属性名映射
fn attr_display_name(attr: &str) -> &str {
    match attr {
        "HP" => "生命",
        "MP" => "魔法",
        "AD" => "物攻",
        "AP" => "魔攻",
        "Defense" => "防御",
        "MagicResistance" | "MDF" => "魔抗",
        "Hit" => "命中",
        "Dodge" => "闪避",
        "Crit" => "暴击",
        "AbsorbHP" => "吸血",
        "ImmuneDamage" => "免伤",
        "ADPTV" => "物穿值",
        "APPTV" => "法穿值",
        _ => attr,
    }
}

fn attr_to_column(attr: &str) -> Option<&str> {
    match attr {
        "HP" | "生命" => Some("HP"),
        "MP" | "魔法" => Some("MP"),
        "AD" | "物攻" => Some("AD"),
        "AP" | "魔攻" => Some("AP"),
        "Defense" | "防御" => Some("Defense"),
        "MagicResistance" | "MDF" | "魔抗" => Some("MagicResistance"),
        "Hit" | "命中" => Some("Hit"),
        "Dodge" | "闪避" => Some("Dodge"),
        "Crit" | "暴击" => Some("Crit"),
        "AbsorbHP" | "吸血" => Some("AbsorbHP"),
        "ImmuneDamage" | "免伤" => Some("ImmuneDamage"),
        "ADPTV" | "物穿值" => Some("ADPTV"),
        "APPTV" | "法穿值" => Some("APPTV"),
        _ => None,
    }
}

/// 获取 GM 对玩家的属性调整列表
pub fn get_adjustments(db: &Database, user_id: &str) -> Vec<(String, i32)> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT aName, aValue FROM editor_attribute_adjustment WHERE uId=?1") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![user_id], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, i32>(1).unwrap_or(0),
        ))
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// 获取指定属性的 GM 调整值
#[allow(dead_code)]
pub fn get_adjustment_value(db: &Database, user_id: &str, attr: &str) -> i32 {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT aValue FROM editor_attribute_adjustment WHERE uId=?1 AND aName=?2") {
        Ok(s) => s,
        Err(_) => return 0,
    };
    stmt.query_row(rusqlite::params![user_id, attr], |row| row.get(0))
        .unwrap_or(0)
}

/// 设置 GM 属性调整
fn set_adjustment(db: &Database, user_id: &str, attr: &str, value: i32) {
    let conn = db.lock_conn();
    let updated = conn
        .execute(
            "UPDATE editor_attribute_adjustment SET aValue=?1 WHERE uId=?2 AND aName=?3",
            rusqlite::params![value, user_id, attr],
        )
        .unwrap_or(0);
    if updated == 0 {
        let _ = conn.execute(
            "INSERT INTO editor_attribute_adjustment (uId, aName, aValue) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, attr, value],
        );
    }
}

/// 删除 GM 属性调整
fn remove_adjustment(db: &Database, user_id: &str, attr: &str) {
    let conn = db.lock_conn();
    let _ = conn.execute(
        "DELETE FROM editor_attribute_adjustment WHERE uId=?1 AND aName=?2",
        rusqlite::params![user_id, attr],
    );
}

/// 属性调整 — GM 命令：属性调整+目标+属性名+数值
/// 支持: 属性调整+玩家ID+生命+100 / 属性调整+玩家ID+生命-100 / 属性调整+玩家ID+生命 (查看)
pub fn cmd_gm_adjust_attr(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power: i32 = db.global_get("Permissions", user_id).parse().unwrap_or(0);
    if power < 100 {
        return format!("{}\n⛔ 你无权操作", prefix);
    }

    if args.is_empty() {
        return format!(
            "{}\n═══ GM属性调整 ═══\n\n\
             格式：\n\
             属性调整+玩家ID+属性名+数值 — 设置调整（正数增加/负数减少）\n\
             属性调整+玩家ID+属性名 — 清除该属性调整\n\
             属性调整+玩家ID — 查看玩家所有调整\n\n\
             可用属性：\n\
             生命/魔法/物攻/魔攻/防御/魔抗\n\
             命中/闪避/暴击/吸血/免伤/物穿值/法穿值",
            prefix
        );
    }

    let parts: Vec<&str> = args.splitn(3, '+').collect();
    let target = parts[0].trim();

    if !db.user_exists(target) {
        return format!("{}\n玩家 [{}] 不存在", prefix, target);
    }

    // 属性调整+玩家ID — 查看所有调整
    if parts.len() == 1 {
        let adjustments = get_adjustments(db, target);
        let target_name = db.read_basic(target, ITEM_NAME);
        if adjustments.is_empty() {
            return format!("{}\n📋 [{}]({}) 暂无属性调整", prefix, target_name, target);
        }
        let mut result = format!("{}\n═══ [{}]({}) 属性调整列表 ═══", prefix, target_name, target);
        for (attr, val) in &adjustments {
            let display = attr_display_name(attr);
            let sign = if *val >= 0 { "+" } else { "" };
            result.push_str(&format!("\n  {}：{}{}", display, sign, val));
        }
        result.push_str(&format!("\n\n共 {} 项调整", adjustments.len()));
        result
    }
    // 属性调整+玩家ID+属性名 — 清除调整
    else if parts.len() == 2 {
        let attr_input = parts[1].trim();
        match attr_to_column(attr_input) {
            Some(attr) => {
                remove_adjustment(db, target, attr);
                let target_name = db.read_basic(target, ITEM_NAME);
                format!(
                    "{}\n✅ 已清除 [{}]({}) 的 {} 调整",
                    prefix,
                    target_name,
                    target,
                    attr_display_name(attr)
                )
            }
            None => format!(
                "{}\n❌ 未知属性：{}\n可选：生命/魔法/物攻/魔攻/防御/魔抗/命中/闪避/暴击/吸血/免伤/物穿值/法穿值",
                prefix, attr_input
            ),
        }
    }
    // 属性调整+玩家ID+属性名+数值 — 设置调整
    else {
        let attr_input = parts[1].trim();
        let value_str = parts[2].trim();

        match attr_to_column(attr_input) {
            Some(attr) => {
                let value: i32 = match value_str.parse() {
                    Ok(v) => v,
                    Err(_) => return format!("{}\n❌ 数值无效：{}", prefix, value_str),
                };

                if value == 0 {
                    remove_adjustment(db, target, attr);
                    let target_name = db.read_basic(target, ITEM_NAME);
                    return format!(
                        "{}\n✅ 已清除 [{}]({}) 的 {} 调整",
                        prefix,
                        target_name,
                        target,
                        attr_display_name(attr)
                    );
                }

                let value = value.clamp(-99999, 99999);
                set_adjustment(db, target, attr, value);
                let target_name = db.read_basic(target, ITEM_NAME);
                let sign = if value >= 0 { "+" } else { "" };
                format!(
                    "{}\n✅ 已设置 [{}]({}) 的 {} 调整为 {}{}",
                    prefix,
                    target_name,
                    target,
                    attr_display_name(attr),
                    sign,
                    value
                )
            }
            None => format!(
                "{}\n❌ 未知属性：{}\n可选：生命/魔法/物攻/魔攻/防御/魔抗/命中/闪避/暴击/吸血/免伤/物穿值/法穿值",
                prefix, attr_input
            ),
        }
    }
}

/// 计算 GM 属性调整的总加成（集成到 calc_total_attrs）
#[allow(clippy::type_complexity)]
pub fn calc_gm_adjustments(
    db: &Database,
    user_id: &str,
) -> (i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32, i32) {
    let adjustments = get_adjustments(db, user_id);
    let mut hp = 0i32;
    let mut mp = 0i32;
    let mut ad = 0i32;
    let mut ap = 0i32;
    let mut def = 0i32;
    let mut mres = 0i32;
    let mut hit = 0i32;
    let mut dodge = 0i32;
    let mut crit = 0i32;
    let mut absorb = 0i32;
    let mut adptv = 0i32;
    let mut apptv = 0i32;
    let mut immune = 0i32;

    for (attr, val) in adjustments {
        match attr.as_str() {
            "HP" => hp += val,
            "MP" => mp += val,
            "AD" => ad += val,
            "AP" => ap += val,
            "Defense" => def += val,
            "MagicResistance" => mres += val,
            "Hit" => hit += val,
            "Dodge" => dodge += val,
            "Crit" => crit += val,
            "AbsorbHP" => absorb += val,
            "ADPTV" => adptv += val,
            "APPTV" => apptv += val,
            "ImmuneDamage" => immune += val,
            _ => {}
        }
    }
    (
        hp, mp, ad, ap, def, mres, hit, dodge, crit, absorb, adptv, apptv, immune,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr_display_name_hp() {
        assert_eq!(attr_display_name("HP"), "生命");
    }

    #[test]
    fn test_attr_display_name_mp() {
        assert_eq!(attr_display_name("MP"), "魔法");
    }

    #[test]
    fn test_attr_display_name_ad() {
        assert_eq!(attr_display_name("AD"), "物攻");
    }

    #[test]
    fn test_attr_display_name_ap() {
        assert_eq!(attr_display_name("AP"), "魔攻");
    }

    #[test]
    fn test_attr_display_name_defense() {
        assert_eq!(attr_display_name("Defense"), "防御");
    }

    #[test]
    fn test_attr_display_name_mdf() {
        assert_eq!(attr_display_name("MDF"), "魔抗");
    }

    #[test]
    fn test_attr_display_name_unknown() {
        assert_eq!(attr_display_name("XYZ"), "XYZ");
    }

    #[test]
    fn test_attr_to_column_english() {
        assert_eq!(attr_to_column("HP"), Some("HP"));
        assert_eq!(attr_to_column("MP"), Some("MP"));
        assert_eq!(attr_to_column("AD"), Some("AD"));
        assert_eq!(attr_to_column("AP"), Some("AP"));
        assert_eq!(attr_to_column("Defense"), Some("Defense"));
        assert_eq!(attr_to_column("Hit"), Some("Hit"));
        assert_eq!(attr_to_column("Dodge"), Some("Dodge"));
        assert_eq!(attr_to_column("Crit"), Some("Crit"));
        assert_eq!(attr_to_column("AbsorbHP"), Some("AbsorbHP"));
        assert_eq!(attr_to_column("ImmuneDamage"), Some("ImmuneDamage"));
        assert_eq!(attr_to_column("ADPTV"), Some("ADPTV"));
        assert_eq!(attr_to_column("APPTV"), Some("APPTV"));
    }

    #[test]
    fn test_attr_to_column_chinese() {
        assert_eq!(attr_to_column("生命"), Some("HP"));
        assert_eq!(attr_to_column("魔法"), Some("MP"));
        assert_eq!(attr_to_column("物攻"), Some("AD"));
        assert_eq!(attr_to_column("魔攻"), Some("AP"));
        assert_eq!(attr_to_column("防御"), Some("Defense"));
        assert_eq!(attr_to_column("魔抗"), Some("MagicResistance"));
        assert_eq!(attr_to_column("命中"), Some("Hit"));
        assert_eq!(attr_to_column("闪避"), Some("Dodge"));
        assert_eq!(attr_to_column("暴击"), Some("Crit"));
        assert_eq!(attr_to_column("吸血"), Some("AbsorbHP"));
        assert_eq!(attr_to_column("免伤"), Some("ImmuneDamage"));
        assert_eq!(attr_to_column("物穿值"), Some("ADPTV"));
        assert_eq!(attr_to_column("法穿值"), Some("APPTV"));
    }

    #[test]
    fn test_attr_to_column_mdf_alias() {
        assert_eq!(attr_to_column("MDF"), Some("MagicResistance"));
    }

    #[test]
    fn test_attr_to_column_unknown() {
        assert_eq!(attr_to_column("Unknown"), None);
        assert_eq!(attr_to_column(""), None);
    }
}

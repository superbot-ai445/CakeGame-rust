/// CakeGame 特技系统
/// 来自 Config_Special 表 — 战斗中可激活的伤害增幅特技
/// 4种特技: 5%伤害/10%伤害/15%伤害/20%伤害，均有30秒冷却
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 特技信息
struct SpecialSkill {
    name: String,
    condition: String,
    #[allow(dead_code)]
    effect_type: String,
    effect_value: i32,
    cooldown: i32,
}

/// 从 Config_Special 读取所有特技
fn get_all_specials(db: &Database) -> Vec<SpecialSkill> {
    db.query_rows(
        "SELECT Name, Condition, Type, Effect, CD FROM Config_Special",
        &[],
        |row| {
            let name: String = row.get(0)?;
            let condition: String = row.get(1).unwrap_or_default();
            let effect_type: String = row.get(2).unwrap_or_default();
            let effect_str: String = row.get(3).unwrap_or_default();
            let cd_str: String = row.get(4).unwrap_or_default();
            Ok(SpecialSkill {
                name,
                condition,
                effect_type,
                effect_value: effect_str.parse().unwrap_or(0),
                cooldown: cd_str.parse().unwrap_or(30),
            })
        },
    )
}

/// 查看特技列表
pub fn cmd_view_specials(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let specials = get_all_specials(db);

    if specials.is_empty() {
        return format!("{}\n═══ 特技系统 ═══\n暂无可用特技。", prefix);
    }

    let args = args.trim();

    // 查看特定特技详情
    if !args.is_empty() {
        for sp in &specials {
            if sp.name.contains(args) || sp.name == args {
                let mut r = format!("{}\n═══ 特技: {} ═══", prefix, sp.name);
                r.push_str(&format!("\n效果: 伤害增幅 {}%", sp.effect_value));
                r.push_str(&format!("\n冷却: {}秒", sp.cooldown));
                let status = if sp.condition == "Active" {
                    "可激活"
                } else {
                    "被动"
                };
                r.push_str(&format!("\n状态: {}", status));
                r.push_str(&format!("\n\n发送 '使用特技+{}' 在战斗中激活此特技", sp.name));
                return r;
            }
        }
        return format!("{}\n找不到特技 [{}]。", prefix, args);
    }

    // 列出所有特技
    let mut r = format!("{}\n═══ 特技系统 ═══", prefix);
    for (i, sp) in specials.iter().enumerate() {
        let status = if sp.condition == "Active" {
            "🟢可激活"
        } else {
            "⚪被动"
        };
        r.push_str(&format!(
            "\n{}. {} | 伤害增幅+{}% | CD:{}秒 | {}",
            i + 1,
            sp.name,
            sp.effect_value,
            sp.cooldown,
            status
        ));
    }
    r.push_str("\n\n发送 '查看特技+特技名' 查看详情");
    r.push_str("\n发送 '使用特技+特技名' 在战斗中激活");
    r
}

/// 使用特技 — 激活临时伤害增幅
pub fn cmd_use_special(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let specials = get_all_specials(db);
    let args = args.trim();

    if args.is_empty() {
        let mut r = format!("{}\n请指定要使用的特技名。可用特技:", prefix);
        for sp in &specials {
            r.push_str(&format!("\n  · {}", sp.name));
        }
        r.push_str("\n\n用法: 使用特技+特技名");
        return r;
    }

    // 查找特技
    let skill = specials.iter().find(|s| s.name.contains(args) || s.name == args);
    if skill.is_none() {
        return format!("{}\n找不到特技 [{}]。发送 '查看特技' 查看可用特技列表。", prefix, args);
    }
    let skill = skill.unwrap();

    if skill.condition != "Active" {
        return format!("{}\n特技 [{}] 是被动特技，无法主动激活。", prefix, skill.name);
    }

    // 检查用户是否存活
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再使用特技。", prefix);
    }

    // 检查冷却 — 通过 write_user_data 存储上次使用时间
    let last_used_key = format!("special_cd_{}", skill.name);
    let last_used = db.read_user_data(user_id, &last_used_key);
    if !last_used.is_empty() {
        if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(&last_used, "%Y-%m-%d %H:%M:%S") {
            let now = Local::now().naive_local();
            let elapsed = (now - last_time).num_seconds();
            if elapsed < skill.cooldown as i64 {
                let remaining = skill.cooldown as i64 - elapsed;
                return format!(
                    "{}\n特技 [{}] 正在冷却中。\n剩余冷却: {}秒",
                    prefix, skill.name, remaining
                );
            }
        }
    }

    // 激活特技 — 添加临时伤害增幅 buff (通过 DynamicAttributes_Register)
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    // 持续时间等于冷却时间（效果持续一个CD周期）
    let expire = (Local::now() + chrono::Duration::seconds(skill.cooldown as i64))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    // 写入增益效果 (AD增幅)
    {
        let conn = db.lock_conn();
        let _ = conn.execute(
            "INSERT INTO DynamicAttributes_Register (User, AttName, AttValue, AttInvalidTime) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![user_id, "AD", skill.effect_value.to_string(), expire],
        );
    }

    // 记录冷却时间
    db.write_user_data(user_id, &last_used_key, &now);

    format!(
        "{}\n✦ 特技激活: {} ✦\n伤害增幅: +{}%\n持续时间: {}秒\n\n         在持续时间内，你的所有攻击伤害提升 {}%！\n冷却时间: {}秒",
        prefix, skill.name, skill.effect_value, skill.cooldown, skill.effect_value, skill.cooldown
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_special_skill_struct() {
        let skill = SpecialSkill {
            name: "TestSkill".to_string(),
            condition: "Active".to_string(),
            effect_type: "Damage".to_string(),
            effect_value: 15,
            cooldown: 30,
        };
        assert_eq!(skill.name, "TestSkill");
        assert_eq!(skill.effect_value, 15);
        assert_eq!(skill.cooldown, 30);
    }

    #[test]
    fn test_cooldown_format() {
        let cooldown = 30i32;
        let msg = format!("冷却: {}秒", cooldown);
        assert!(msg.contains("30"));
    }

    #[test]
    fn test_effect_value_positive() {
        let effect_value = 20i32;
        assert!(effect_value > 0);
        let msg = format!("伤害增幅 +{}%", effect_value);
        assert!(msg.contains("20"));
    }

    #[test]
    fn test_condition_active() {
        let condition = "Active";
        let status = if condition == "Active" { "可激活" } else { "被动" };
        assert_eq!(status, "可激活");
    }

    #[test]
    fn test_condition_passive() {
        let condition = "Passive";
        let status = if condition == "Active" { "可激活" } else { "被动" };
        assert_eq!(status, "被动");
    }

    #[test]
    fn test_last_used_key_format() {
        let skill_name = "暴击强化";
        let key = format!("special_cd_{}", skill_name);
        assert_eq!(key, "special_cd_暴击强化");
    }
}

/// 装备特技系统 — 基于 Config_Equipskills 表
///
/// 特定装备(如【史诗】秋叶刀)穿戴后可解锁专属主动技能，
/// 每个技能有独立冷却时间(CD)和战斗效果。
///
/// 表结构: Name(装备名) | jn(技能名) | Type(效果类型) | Num(效果数值) | Cx(持续/倍率) | CD(冷却秒数)
///
/// 效果类型: 暴击(暴击率加成), 伤害(伤害增幅), 吸血(生命偷取), 护盾(伤害减免)
///
/// 使用流程: 查看装备特技 → 装备对应武器 → 使用装备特技+技能名 → 战斗中触发效果
use crate::db::Database;
use crate::user;

/// 装备特技定义
#[derive(Debug, Clone)]
pub struct EquipUniqueSkill {
    pub equip_name: String,
    pub skill_name: String,
    pub effect_type: String,
    pub effect_value: i32,
    pub multiplier: i32,
    pub cooldown: i32,
}

/// 解析 Num 字段格式 "30&#44TRUE" → (30, true)
fn parse_num_field(raw: &str) -> (i32, bool) {
    let cleaned = raw.replace("&#44;", ",").replace("&#44", ",");
    let parts: Vec<&str> = cleaned.split(',').collect();
    let value = parts.first().and_then(|s| s.trim().parse().ok()).unwrap_or(0);
    let always_active = parts.get(1).map(|s| s.trim().to_uppercase() == "TRUE").unwrap_or(false);
    (value, always_active)
}

/// 从 Config_Equipskills 读取所有装备特技
pub fn load_equip_unique_skills(db: &Database) -> Vec<EquipUniqueSkill> {
    db.query_rows(
        "SELECT Name, jn, Type, Num, Cx, CD FROM Config_Equipskills",
        &[],
        |row| {
            let name: String = row.get(0)?;
            let jn: String = row.get(1)?;
            let effect_type: String = row.get(2).unwrap_or_default();
            let num_raw: String = row.get(3).unwrap_or_default();
            let cx_raw: String = row.get(4).unwrap_or_default();
            let cd_raw: String = row.get(5).unwrap_or_default();

            let (effect_value, _always) = parse_num_field(&num_raw);
            let multiplier = cx_raw.parse().unwrap_or(0);
            let cooldown = cd_raw.parse().unwrap_or(300);

            Ok(EquipUniqueSkill {
                equip_name: name,
                skill_name: jn,
                effect_type,
                effect_value,
                multiplier,
                cooldown,
            })
        },
    )
}

/// 查看当前穿戴装备的特技列表
pub fn cmd_view_equip_unique_skills(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let all_skills = load_equip_unique_skills(db);

    if all_skills.is_empty() {
        return format!("{}\n═══ 装备特技 ═══\n暂无装备特技数据。", prefix);
    }

    let equips = db.equip_all(user_id);
    let equipped_names: Vec<String> = equips.iter().map(|e| e.name.clone()).collect();

    let mut r = format!("{}\n═══ 装备特技系统 ═══", prefix);

    let mut found = 0;
    for skill in &all_skills {
        let is_equipped = equipped_names
            .iter()
            .any(|n| n.contains(&skill.equip_name) || skill.equip_name.contains(n));

        let status = if is_equipped { "✅已激活" } else { "🔒需装备" };
        let effect_desc = format_effect(&skill.effect_type, skill.effect_value, skill.multiplier);

        r.push_str(&format!(
            "\n🗡 {} → [{}]\n   {} | CD:{}秒 | {}",
            skill.equip_name, skill.skill_name, effect_desc, skill.cooldown, status
        ));

        if is_equipped {
            r.push_str(&format!("\n   📌 发送 '使用装备特技+{}' 激活", skill.skill_name));
            found += 1;
        }
    }

    if found == 0 {
        r.push_str("\n\n💡 装备对应武器后即可解锁特技技能");
    }

    r
}

/// 查看装备特技详情
pub fn cmd_equip_unique_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        return format!("{}\n请指定特技名称。例: 特技详情+咸鱼爆发", prefix);
    }

    let all_skills = load_equip_unique_skills(db);

    for skill in &all_skills {
        if skill.skill_name.contains(args) || skill.equip_name.contains(args) {
            let effect_desc = format_effect(&skill.effect_type, skill.effect_value, skill.multiplier);

            let mut r = format!("{}\n═══ 特技: {} ═══", prefix, skill.skill_name);
            r.push_str(&format!("\n所属装备: {}", skill.equip_name));
            r.push_str(&format!("\n效果类型: {}", effect_type_name(&skill.effect_type)));
            r.push_str(&format!("\n效果详情: {}", effect_desc));
            r.push_str(&format!("\n持续倍率: {}", skill.multiplier));
            r.push_str(&format!("\n冷却时间: {}秒", skill.cooldown));

            let equips = db.equip_all(user_id);
            let is_equipped = equips
                .iter()
                .any(|e| e.name.contains(&skill.equip_name) || skill.equip_name.contains(&e.name));

            if is_equipped {
                r.push_str("\n\n✅ 当前已装备对应武器，可使用此特技");
                r.push_str(&format!("\n发送 '使用装备特技+{}' 激活", skill.skill_name));
            } else {
                r.push_str("\n\n🔒 请先装备对应武器才能使用此特技");
            }

            return r;
        }
    }

    format!("{}\n找不到特技 [{}]。发送 '装备特技' 查看所有可用特技。", prefix, args)
}

/// 使用装备特技 — 激活临时战斗增益
pub fn cmd_use_equip_unique_skill(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        return format!("{}\n请指定特技名称。例: 使用装备特技+咸鱼爆发", prefix);
    }

    let all_skills = load_equip_unique_skills(db);

    let skill = match all_skills
        .iter()
        .find(|s| s.skill_name.contains(args) || args.contains(&s.skill_name))
    {
        Some(s) => s.clone(),
        None => return format!("{}\n找不到特技 [{}]。发送 '装备特技' 查看可用列表。", prefix, args),
    };

    let equips = db.equip_all(user_id);
    let is_equipped = equips
        .iter()
        .any(|e| e.name.contains(&skill.equip_name) || skill.equip_name.contains(&e.name));

    if !is_equipped {
        return format!(
            "{}\n🔒 需要装备 [{}] 才能使用特技 [{}]",
            prefix, skill.equip_name, skill.skill_name
        );
    }

    // 检查冷却
    let last_used_key = format!("equip_unique_cd_{}_{}", user_id, skill.skill_name);
    let last_used_str = db.global_get("equip_unique_cd", &last_used_key);
    if !last_used_str.is_empty() {
        if let Ok(last_ts) = last_used_str.parse::<i64>() {
            let now = chrono::Local::now().timestamp();
            let elapsed = now - last_ts;
            if elapsed < skill.cooldown as i64 {
                let remaining = skill.cooldown as i64 - elapsed;
                return format!(
                    "{}\n⏳ 特技 [{}] 冷却中，还需等待 {}秒",
                    prefix, skill.skill_name, remaining
                );
            }
        }
    }

    let now = chrono::Local::now().timestamp();
    db.global_set("equip_unique_cd", &last_used_key, &now.to_string());

    // 激活特技效果 — 添加临时 buff
    let effect_attr = effect_type_to_attr(&skill.effect_type);
    let expiry = (now + skill.multiplier as i64).to_string();

    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO DynamicAttributes_Register \
         (User, AttName, AttValue, AttInvalidTime) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![user_id, effect_attr, skill.effect_value.to_string(), expiry],
    );
    drop(conn);

    let effect_desc = format_effect(&skill.effect_type, skill.effect_value, skill.multiplier);

    format!(
        "{}\n⚡ 特技激活: [{}]\n{}\n持续: {}秒\n冷却: {}秒\n\n💡 效果已生效，下次攻击将享受增益！",
        prefix, skill.skill_name, effect_desc, skill.multiplier, skill.cooldown
    )
}

/// 格式化效果描述
fn format_effect(effect_type: &str, value: i32, multiplier: i32) -> String {
    match effect_type {
        "暴击" => format!("暴击率+{}% | 倍率×{}", value, multiplier),
        "伤害" => format!("伤害+{}% | 持续{}秒", value, multiplier),
        "吸血" => format!("吸血+{}% | 持续{}秒", value, multiplier),
        "护盾" => format!("伤害减免{}% | 持续{}秒", value, multiplier),
        _ => format!("{} +{} | 持续{}", effect_type, value, multiplier),
    }
}

/// 效果类型中文名
fn effect_type_name(effect_type: &str) -> String {
    match effect_type {
        "暴击" => "暴击率增幅".to_string(),
        "伤害" => "伤害增幅".to_string(),
        "吸血" => "生命偷取".to_string(),
        "护盾" => "伤害减免".to_string(),
        other => other.to_string(),
    }
}

/// 效果类型映射到属性名
fn effect_type_to_attr(effect_type: &str) -> String {
    match effect_type {
        "暴击" => "Crit".to_string(),
        "伤害" => "AD".to_string(),
        "吸血" => "AbsorbHP".to_string(),
        "护盾" => "ImmuneDamage".to_string(),
        _ => "AD".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_num_field_standard() {
        let (val, always) = parse_num_field("30&#44TRUE");
        assert_eq!(val, 30);
        assert!(always);
    }

    #[test]
    fn test_parse_num_field_false() {
        let (val, always) = parse_num_field("50&#44FALSE");
        assert_eq!(val, 50);
        assert!(!always);
    }

    #[test]
    fn test_parse_num_field_plain() {
        let (val, _) = parse_num_field("25");
        assert_eq!(val, 25);
    }

    #[test]
    fn test_format_effect_types() {
        assert!(format_effect("暴击", 30, 35).contains("暴击率"));
        assert!(format_effect("伤害", 20, 10).contains("伤害"));
        assert!(format_effect("吸血", 15, 5).contains("吸血"));
        assert!(format_effect("护盾", 10, 8).contains("减免"));
    }

    #[test]
    fn test_effect_type_to_attr() {
        assert_eq!(effect_type_to_attr("暴击"), "Crit");
        assert_eq!(effect_type_to_attr("伤害"), "AD");
        assert_eq!(effect_type_to_attr("吸血"), "AbsorbHP");
        assert_eq!(effect_type_to_attr("护盾"), "ImmuneDamage");
    }

    #[test]
    fn test_effect_type_name() {
        assert_eq!(effect_type_name("暴击"), "暴击率增幅");
        assert_eq!(effect_type_name("伤害"), "伤害增幅");
        assert_eq!(effect_type_name("未知"), "未知");
    }
}

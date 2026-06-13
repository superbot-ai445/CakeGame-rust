/// 装备技能系统 + 持续技能/DOT系统
///
/// 基于 Ext_EquipSkill_Set_hzxyx 表（7条）：
///   eqName(装备名) | eqSkill(被动技能) | atkSkill(攻击附加技能) | rdAtk(触发概率)
///
/// 基于 Ext_Skill_Continued_hzxyx 表（13条）：
///   Name(技能名) | ZZBool(效果类型: 30=增益, 31=减益/控制)
///
/// 装备技能：特定装备穿戴后可获得额外技能效果
/// 持续技能：技能命中后可附带DOT(持续伤害)或增益效果
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 活跃持续效果条目（来自 SkillContinued_Register 表）
#[allow(dead_code)]
pub struct ActiveEffect {
    pub id: String,
    pub skill: String,
    pub round: i32,
    pub effect_type: String,
    pub formula: String,
}

/// 装备技能映射
pub struct EquipSkillEntry {
    pub eq_name: String,
    pub eq_skill: String,  // 被动技能
    pub atk_skill: String, // 攻击附加技能
    pub trigger_rate: i32, // 触发概率 (百分比)
}

/// 持续技能定义
pub struct ContinuousSkillEntry {
    pub name: String,
    pub effect_type: i32, // 30=增益, 31=减益/控制
}

/// 从数据库读取所有装备技能映射
pub fn load_equip_skills(db: &Database) -> Vec<EquipSkillEntry> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT eqName, eqSkill, atkSkill, rdAtk FROM Ext_EquipSkill_Set_hzxyx") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(3).unwrap_or_default(),
        ))
    });

    match rows {
        Ok(mapped) => mapped
            .filter_map(|r| r.ok())
            .map(|(eq, skill, atk, rate)| EquipSkillEntry {
                eq_name: eq,
                eq_skill: skill,
                atk_skill: atk,
                trigger_rate: rate.parse().unwrap_or(100),
            })
            .collect(),
        Err(_) => vec![],
    }
}

/// 从数据库读取所有持续技能定义
pub fn load_continuous_skills(db: &Database) -> Vec<ContinuousSkillEntry> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Name, ZZBool FROM Ext_Skill_Continued_hzxyx") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
        ))
    });

    match rows {
        Ok(mapped) => mapped
            .filter_map(|r| r.ok())
            .map(|(name, zz)| ContinuousSkillEntry {
                name,
                effect_type: zz.parse().unwrap_or(30),
            })
            .collect(),
        Err(_) => vec![],
    }
}

/// 获取玩家已装备的装备名集合
fn get_equipped_names(db: &Database, user_id: &str) -> Vec<String> {
    let equips = db.equip_all(user_id);
    equips.into_iter().map(|e| e.name).collect()
}

/// 检查持续技能是否存在
pub fn is_continuous_skill(db: &Database, skill_name: &str) -> bool {
    let skills = load_continuous_skills(db);
    skills.iter().any(|s| s.name == skill_name)
}

/// 获取持续技能效果类型 (30=增益, 31=减益)
pub fn get_continuous_type(db: &Database, skill_name: &str) -> Option<i32> {
    let skills = load_continuous_skills(db);
    skills.iter().find(|s| s.name == skill_name).map(|s| s.effect_type)
}

// ==================== 指令处理 ====================

/// 查看装备技能 — 显示当前穿戴装备附带的技能效果
pub fn cmd_view_equip_skills(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let equipped = get_equipped_names(db, user_id);

    if equipped.is_empty() {
        return format!("{}\n📦 当前未穿戴任何装备\n💡 使用「查看装备」查看已穿戴装备", prefix);
    }

    let all_skills = load_equip_skills(db);
    let mut found = false;
    let mut out = format!("{}\n⚡ 【装备技能列表】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for eq in &equipped {
        // 精确匹配或包含匹配（如 "【完美】能源驱动器" 匹配 "能源驱动器"）
        for entry in &all_skills {
            if eq.contains(&entry.eq_name) || entry.eq_name.contains(eq) {
                found = true;
                out.push_str(&format!("\n🔹 [{}]\n", eq));
                if !entry.eq_skill.is_empty() {
                    out.push_str(&format!("  ♻️ 被动: {} (常驻)\n", entry.eq_skill));
                }
                if !entry.atk_skill.is_empty() {
                    out.push_str(&format!(
                        "  ⚔️ 攻击: {} (触发率{}%)\n",
                        entry.atk_skill, entry.trigger_rate
                    ));
                }
                break;
            }
        }
    }

    if !found {
        out.push_str("\n📦 当前穿戴的装备没有附带技能\n");
        out.push_str("💡 试试装备「完美」「超界」级别的装备\n");
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 装备技能在战斗中自动触发\n");

    out
}

/// 查看持续技能列表 — 显示所有持续/增强技能
pub fn cmd_view_continuous_skills(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let skills = load_continuous_skills(db);

    if skills.is_empty() {
        return format!("{}\n📋 暂无持续技能数据", prefix);
    }

    let mut out = format!("{}\n🔄 【持续技能列表】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for skill in &skills {
        let type_text = if skill.effect_type == 30 {
            "✨增益"
        } else {
            "💀减益"
        };
        out.push_str(&format!(
            "  {} {} — {}\n",
            type_text,
            skill.name,
            match skill.effect_type {
                30 => "持续强化自身能力",
                31 => "持续削弱目标或造成伤害",
                _ => "特殊效果",
            }
        ));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 持续技能在战斗中命中后自动附带\n");

    out
}

/// 处理装备技能的战斗触发
/// 在攻击后调用，检查是否有装备技能触发
/// 返回 (triggered_skill, extra_damage) 或 None
pub fn try_equip_skill_trigger(db: &Database, user_id: &str) -> Option<(String, i32)> {
    let equipped = get_equipped_names(db, user_id);
    let all_skills = load_equip_skills(db);

    for eq in &equipped {
        for entry in &all_skills {
            if eq.contains(&entry.eq_name) || entry.eq_name.contains(eq) {
                // 检查攻击附加技能
                if !entry.atk_skill.is_empty() {
                    let roll: i32 = rand::thread_rng().gen_range(1..=100);
                    if roll <= entry.trigger_rate {
                        // 获取技能效果值
                        let effect = db.skill_get(&entry.atk_skill).map(|s| s.effect).unwrap_or(0);
                        return Some((entry.atk_skill.clone(), effect));
                    }
                }
                break;
            }
        }
    }

    None
}

/// 处理持续技能/DOT效果
/// 在战斗后调用，检查目标是否受到DOT影响
/// 返回DOT伤害信息
pub fn process_continuous_effect(db: &Database, user_id: &str, skill_name: &str, target: &str) -> Option<String> {
    if !is_continuous_skill(db, skill_name) {
        return None;
    }

    let effect_type = get_continuous_type(db, skill_name).unwrap_or(30);

    // 获取技能效果
    let skill = db.skill_get(skill_name)?;
    let effect = skill.effect;

    if effect_type == 31 {
        // 减益/DOT — 对目标造成持续伤害
        let dot_damage = effect / 3; // DOT伤害为技能效果的1/3
        if dot_damage > 0 {
            // 记录DOT状态
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            db.write_user_data(user_id, &format!("dot.{}.target", skill_name), target);
            db.write_user_data(user_id, &format!("dot.{}.damage", skill_name), &dot_damage.to_string());
            db.write_user_data(user_id, &format!("dot.{}.time", skill_name), &now);
            db.write_user_data(user_id, &format!("dot.{}.turns", skill_name), "3");

            return Some(format!(
                "💀 {}附加了[{}]效果！持续伤害 {}/回合 (3回合)",
                target, skill_name, dot_damage
            ));
        }
    } else {
        // 增益Buff
        let buff_value = effect / 5;
        if buff_value > 0 {
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            db.write_user_data(user_id, &format!("buff.{}.value", skill_name), &buff_value.to_string());
            db.write_user_data(user_id, &format!("buff.{}.time", skill_name), &now);
            db.write_user_data(user_id, &format!("buff.{}.turns", skill_name), "3");

            return Some(format!(
                "✨ {}获得了[{}]增益！+{}/回合 (3回合)",
                user_id, skill_name, buff_value
            ));
        }
    }

    None
}

/// 查看所有活跃持续效果（战斗中生效的DOT/Buff）
/// 基于 SkillContinued_Register 表（32条真实数据）
pub fn cmd_view_active_effects(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    // 查询当前用户的活跃效果
    let user_prefix = format!("user.{}", user_id);
    let mut stmt = match conn.prepare("SELECT Skill, Round, Type, Effect FROM SkillContinued_Register WHERE ID = ?1") {
        Ok(s) => s,
        Err(_) => return format!("{}\n📋 暂无活跃持续效果", prefix),
    };

    let rows = stmt.query_map(rusqlite::params![user_prefix], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(3).unwrap_or_default(),
        ))
    });

    match rows {
        Ok(mapped) => {
            let effects: Vec<_> = mapped.filter_map(|r| r.ok()).collect();
            if effects.is_empty() {
                return format!("{}\n📋 当前无活跃持续效果\\n💡 战斗中释放持续技能后会自动注册", prefix);
            }

            let mut out = format!("{}\n🔄 【活跃持续效果】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
            for (skill, round, etype, effect) in &effects {
                let type_icon = if etype == "#2" {
                    "💀DOT"
                } else if etype == "#3" {
                    "✨增益"
                } else {
                    "🔹效果"
                };
                let effect_preview = if effect.len() > 40 {
                    format!("{}...", &effect[..37])
                } else {
                    effect.clone()
                };
                out.push_str(&format!(
                    "  {} {} — 剩余{}回合\n    公式: {}\n",
                    type_icon, skill, round, effect_preview
                ));
            }
            out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
            out.push_str("💡 持续效果每回合自动结算\n");
            Ok(out)
        }
        Err(_) => Ok(format!("{}\n📋 查询活跃效果失败", prefix)),
    }
    .unwrap_or_else(|_: String| format!("{}\n📋 查询失败", prefix))
}

/// 获取用户所有活跃持续效果（用于战斗结算）
pub fn get_user_active_effects(db: &Database, user_id: &str) -> Vec<ActiveEffect> {
    let conn = db.lock_conn();
    let user_prefix = format!("user.{}", user_id);

    let mut stmt =
        match conn.prepare("SELECT ID, Skill, Round, Type, Effect FROM SkillContinued_Register WHERE ID = ?1") {
            Ok(s) => s,
            Err(_) => return vec![],
        };

    let rows = stmt.query_map(rusqlite::params![user_prefix], |row| {
        Ok(ActiveEffect {
            id: row.get::<_, String>(0).unwrap_or_default(),
            skill: row.get::<_, String>(1).unwrap_or_default(),
            round: row.get::<_, String>(2).unwrap_or_default().parse().unwrap_or(0),
            effect_type: row.get::<_, String>(3).unwrap_or_default(),
            formula: row.get::<_, String>(4).unwrap_or_default(),
        })
    });

    match rows {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    }
}

/// 注册一个持续效果到 SkillContinued_Register
#[allow(dead_code)]
pub fn register_continuous_effect(
    db: &Database,
    user_id: &str,
    skill_name: &str,
    rounds: i32,
    effect_type: &str,
    formula: &str,
) -> bool {
    let conn = db.lock_conn();
    let entity_id = format!("user.{}", user_id);

    // 先删除旧的同名效果（刷新）
    let _ = conn.execute(
        "DELETE FROM SkillContinued_Register WHERE ID = ?1 AND Skill = ?2",
        rusqlite::params![entity_id, skill_name],
    );

    // 插入新效果
    conn.execute(
        "INSERT INTO SkillContinued_Register (ID, Skill, Round, Type, Effect) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![entity_id, skill_name, rounds.to_string(), effect_type, formula],
    )
    .is_ok()
}

/// 移除过期的持续效果（回合 <= 0）
pub fn cleanup_expired_effects(db: &Database) -> i32 {
    let conn = db.lock_conn();
    match conn.execute(
        "DELETE FROM SkillContinued_Register WHERE CAST(Round AS INTEGER) <= 0",
        [],
    ) {
        Ok(n) => n as i32,
        Err(_) => 0,
    }
}

/// 每回合递减所有效果的剩余回合数
pub fn tick_all_effects(db: &Database) -> i32 {
    let conn = db.lock_conn();
    match conn.execute(
        "UPDATE SkillContinued_Register SET Round = CAST(Round AS INTEGER) - 1 WHERE CAST(Round AS INTEGER) > 0",
        [],
    ) {
        Ok(n) => n as i32,
        Err(_) => 0,
    }
}

/// 获取所有活跃效果统计（GM查看）
pub fn count_active_effects(db: &Database) -> (i32, i32, i32) {
    let conn = db.lock_conn();

    let total: i32 = conn
        .query_row("SELECT COUNT(*) FROM SkillContinued_Register", [], |r| r.get(0))
        .unwrap_or(0);

    let dot_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM SkillContinued_Register WHERE Type = '#2'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let buff_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM SkillContinued_Register WHERE Type = '#3'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    (total, dot_count, buff_count)
}

/// 处理所有活跃DOT效果（每回合调用）
/// 返回DOT伤害文本列表
pub fn process_active_dots(db: &Database, user_id: &str) -> Vec<String> {
    let mut results = Vec::new();
    let continuous = load_continuous_skills(db);

    for skill in &continuous {
        let turns_key = format!("dot.{}.turns", skill.name);
        let damage_key = format!("dot.{}.damage", skill.name);
        let target_key = format!("dot.{}.target", skill.name);

        let turns_str = db.read_user_data(user_id, &turns_key);
        if turns_str.is_empty() {
            continue;
        }

        let turns: i32 = turns_str.parse().unwrap_or(0);
        if turns <= 0 {
            // DOT 已过期，清除
            db.write_user_data(user_id, &turns_key, "");
            db.write_user_data(user_id, &damage_key, "");
            db.write_user_data(user_id, &target_key, "");
            continue;
        }

        let damage: i32 = db.read_user_data(user_id, &damage_key).parse().unwrap_or(0);
        let target = db.read_user_data(user_id, &target_key);

        if damage > 0 {
            results.push(format!(
                "💀 [{}]对{}造成 {} 持续伤害 (剩余{}回合)",
                skill.name,
                if target.is_empty() { "目标" } else { &target },
                damage,
                turns - 1
            ));
        }

        // 减少回合数
        db.write_user_data(user_id, &turns_key, &(turns - 1).to_string());
    }

    results
}

/// 处理所有活跃Buff效果（每回合调用）
pub fn process_active_buffs(db: &Database, user_id: &str) -> Vec<String> {
    let mut results = Vec::new();
    let continuous = load_continuous_skills(db);

    for skill in &continuous {
        let turns_key = format!("buff.{}.turns", skill.name);
        let value_key = format!("buff.{}.value", skill.name);

        let turns_str = db.read_user_data(user_id, &turns_key);
        if turns_str.is_empty() {
            continue;
        }

        let turns: i32 = turns_str.parse().unwrap_or(0);
        if turns <= 0 {
            db.write_user_data(user_id, &turns_key, "");
            db.write_user_data(user_id, &value_key, "");
            results.push(format!("✨ [{}]增益效果已消退", skill.name));
            continue;
        }

        let value: i32 = db.read_user_data(user_id, &value_key).parse().unwrap_or(0);
        if value > 0 {
            results.push(format!("✨ [{}]增益中 +{} (剩余{}回合)", skill.name, value, turns - 1));
        }

        db.write_user_data(user_id, &turns_key, &(turns - 1).to_string());
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_effect_struct() {
        let ae = ActiveEffect {
            id: "user.123".to_string(),
            skill: "疾风骤雨".to_string(),
            round: 30,
            effect_type: "#2".to_string(),
            formula: "[我方物攻]*1.2".to_string(),
        };
        assert_eq!(ae.id, "user.123");
        assert_eq!(ae.skill, "疾风骤雨");
        assert_eq!(ae.round, 30);
        assert_eq!(ae.effect_type, "#2");
    }

    #[test]
    fn test_active_effect_types() {
        let dot = ActiveEffect {
            id: "user.1".to_string(),
            skill: "DOT技能".to_string(),
            round: 5,
            effect_type: "#2".to_string(),
            formula: "100".to_string(),
        };
        let buff = ActiveEffect {
            id: "user.1".to_string(),
            skill: "增益技能".to_string(),
            round: 3,
            effect_type: "#3".to_string(),
            formula: "50".to_string(),
        };
        assert_eq!(dot.effect_type, "#2");
        assert_eq!(buff.effect_type, "#3");
        assert!(dot.round > 0);
        assert!(buff.round > 0);
    }

    #[test]
    fn test_entity_id_format() {
        let user_id = "2144321239";
        let entity_id = format!("user.{}", user_id);
        assert_eq!(entity_id, "user.2144321239");
        assert!(entity_id.starts_with("user."));
    }

    #[test]
    fn test_effect_preview_truncation() {
        let long_formula =
            "var Atk=[我方等级]*1.0+[我方法强]*0.33+[生命上限]*0.01;var Def=[对方法防];(Atk*(1-Def/(Atk+Def)));";
        let preview = if long_formula.len() > 40 {
            format!("{}...", &long_formula[..37])
        } else {
            long_formula.to_string()
        };
        assert!(preview.len() <= 41);
        assert!(preview.ends_with("..."));
    }
}

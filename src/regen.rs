/// 被动回复系统 (Passive Regeneration)
/// 基于 Ext_Var_VarSet_hzxyx 表的回血/回蓝/治疗力度配置
/// 每次攻击后自动回复一定量的HP和MP
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 回复配置
struct RegenConfig {
    hp_regen: i32,
    mp_regen: i32,
    heal_power: i32,
}

/// 从 Ext_Var_VarSet_hzxyx 表读取回复配置
fn load_regen_config(db: &Database) -> RegenConfig {
    let conn = db.lock_conn();
    let mut hp_regen = 30i32;
    let mut mp_regen = 30i32;
    let mut heal_power = 30i32;

    if let Ok(mut stmt) = conn.prepare("SELECT Name, Base FROM Ext_Var_VarSet_hzxyx") {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                let val: i32 = row.1.parse().unwrap_or(0);
                match row.0.as_str() {
                    "回血" => hp_regen = val,
                    "回蓝" => mp_regen = val,
                    "治疗力度" => heal_power = val,
                    _ => {}
                }
            }
        }
    }

    RegenConfig {
        hp_regen,
        mp_regen,
        heal_power,
    }
}

/// 应用被动回复效果（战斗后调用）
/// 返回回复日志文本（如果触发了回复）
pub fn apply_regen(db: &Database, user_id: &str) -> String {
    let config = load_regen_config(db);

    // 全服祝福加成
    let blessing_mult: f64 = if crate::world_event::is_server_blessing_active(db) {
        1.5
    } else {
        1.0
    };

    // 饱食度影响回复效率
    let (hunger_mult, _hunger_desc) = crate::food::get_hunger_exp_bonus(db, user_id);
    // 用经验倍率来影响回复: 极饱=1.5x回复, 饥饿=0.5x回复
    let hunger_regen_mult = hunger_mult;

    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    let hp_max: i32 = user::calc_hp_max(db, user_id);
    let mp_current: i32 = db.read_basic(user_id, ITEM_MP_CURRENT).parse().unwrap_or(0);
    let mp_max: i32 = user::calc_mp_max(db, user_id);

    let mut log = Vec::new();

    // HP 回复
    if hp_current < hp_max && config.hp_regen > 0 {
        let boosted_regen = (config.hp_regen as f64 * blessing_mult * hunger_regen_mult) as i32;
        let actual_hp = boosted_regen.min(hp_max - hp_current);
        if actual_hp > 0 {
            db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_current + actual_hp);
            if blessing_mult > 1.0 {
                log.push(format!(
                    "💚 被动回复生命 +{} (祝福加成x{:.1})",
                    actual_hp, blessing_mult
                ));
            } else {
                log.push(format!("💚 被动回复生命 +{}", actual_hp));
            }
        }
    }

    // MP 回复
    if mp_current < mp_max && config.mp_regen > 0 {
        let boosted_regen = (config.mp_regen as f64 * blessing_mult * hunger_regen_mult) as i32;
        let actual_mp = boosted_regen.min(mp_max - mp_current);
        if actual_mp > 0 {
            db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_current + actual_mp);
            if blessing_mult > 1.0 {
                log.push(format!(
                    "💙 被动回复魔法 +{} (祝福加成x{:.1})",
                    actual_mp, blessing_mult
                ));
            } else {
                log.push(format!("💙 被动回复魔法 +{}", actual_mp));
            }
        }
    }

    if log.is_empty() {
        String::new()
    } else {
        log.join("\n")
    }
}

/// 查看被动回复状态
pub fn cmd_view_regen(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let config = load_regen_config(db);

    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    let hp_max: i32 = user::calc_hp_max(db, user_id);
    let mp_current: i32 = db.read_basic(user_id, ITEM_MP_CURRENT).parse().unwrap_or(0);
    let mp_max: i32 = user::calc_mp_max(db, user_id);

    let mut out = String::from("═══ 被动回复系统 ═══\n");
    out.push_str(&format!("💚 每次战斗回复生命: +{}\n", config.hp_regen));
    out.push_str(&format!("💙 每次战斗回复魔法: +{}\n", config.mp_regen));
    out.push_str(&format!("✨ 治疗力度加成: +{}\n", config.heal_power));
    out.push('\n');
    out.push_str(&format!("❤️ 当前生命: {}/{}\n", hp_current, hp_max));
    out.push_str(&format!("💧 当前魔法: {}/{}\n", mp_current, mp_max));

    // 显示满血/满蓝状态
    if hp_current >= hp_max && mp_current >= mp_max {
        out.push_str("\n💪 状态: 全满，无需回复");
    } else {
        let hp_need = (hp_max - hp_current).max(0);
        let mp_need = (mp_max - mp_current).max(0);
        if hp_need > 0 {
            let turns = (hp_need as f64 / config.hp_regen.max(1) as f64).ceil() as i32;
            out.push_str(&format!("\n⏳ 回满生命还需约 {} 次战斗", turns));
        }
        if mp_need > 0 {
            let turns = (mp_need as f64 / config.mp_regen.max(1) as f64).ceil() as i32;
            out.push_str(&format!("⏳ 回满魔法还需约 {} 次战斗", turns));
        }
    }

    out
}

/// 使用治疗物品时，治疗力度加成
#[allow(dead_code)]
pub fn get_heal_power_bonus(db: &Database) -> i32 {
    let config = load_regen_config(db);
    config.heal_power
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        Database {
            conn: std::sync::Mutex::new(conn),
        }
    }

    #[test]
    fn test_regen_config_defaults() {
        let db = make_db();
        let config = load_regen_config(&db);
        assert_eq!(config.hp_regen, 30);
        assert_eq!(config.mp_regen, 30);
        assert_eq!(config.heal_power, 30);
    }

    #[test]
    fn test_regen_config_positive() {
        let db = make_db();
        let config = load_regen_config(&db);
        assert!(config.hp_regen > 0);
        assert!(config.mp_regen > 0);
        assert!(config.heal_power > 0);
    }

    #[test]
    fn test_regen_struct_fields() {
        let config = RegenConfig {
            hp_regen: 50,
            mp_regen: 40,
            heal_power: 60,
        };
        assert_eq!(config.hp_regen, 50);
        assert_eq!(config.mp_regen, 40);
        assert_eq!(config.heal_power, 60);
    }

    #[test]
    fn test_get_heal_power_bonus_default() {
        let db = make_db();
        let bonus = get_heal_power_bonus(&db);
        assert_eq!(bonus, 30);
    }
}

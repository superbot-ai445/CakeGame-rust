/// 装备属性精炼系统
/// 来源: Shared_Data Ext_Attribut_EQ_hzxyx (17条) + Ext_Var_EqValue_hzxyx (15条)
/// 玩家可以通过精炼提升装备的附加属性值
/// 精炼等级越高，装备属性加成越多
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 精炼等级定义
struct RefineLevel {
    level: i32,
    name: &'static str,
    cost_gold: i64,
    cost_diamond: i32,
    success_rate: f64,
    attr_bonus_pct: f64,
}

const REFINE_LEVELS: &[RefineLevel] = &[
    RefineLevel {
        level: 0,
        name: "未精炼",
        cost_gold: 0,
        cost_diamond: 0,
        success_rate: 1.0,
        attr_bonus_pct: 0.0,
    },
    RefineLevel {
        level: 1,
        name: "初窥",
        cost_gold: 500,
        cost_diamond: 0,
        success_rate: 0.95,
        attr_bonus_pct: 0.02,
    },
    RefineLevel {
        level: 2,
        name: "入门",
        cost_gold: 1000,
        cost_diamond: 0,
        success_rate: 0.90,
        attr_bonus_pct: 0.05,
    },
    RefineLevel {
        level: 3,
        name: "熟练",
        cost_gold: 2000,
        cost_diamond: 5,
        success_rate: 0.85,
        attr_bonus_pct: 0.08,
    },
    RefineLevel {
        level: 4,
        name: "精通",
        cost_gold: 5000,
        cost_diamond: 10,
        success_rate: 0.75,
        attr_bonus_pct: 0.12,
    },
    RefineLevel {
        level: 5,
        name: "大师",
        cost_gold: 10000,
        cost_diamond: 20,
        success_rate: 0.65,
        attr_bonus_pct: 0.18,
    },
    RefineLevel {
        level: 6,
        name: "宗师",
        cost_gold: 20000,
        cost_diamond: 50,
        success_rate: 0.50,
        attr_bonus_pct: 0.25,
    },
    RefineLevel {
        level: 7,
        name: "超凡",
        cost_gold: 50000,
        cost_diamond: 100,
        success_rate: 0.35,
        attr_bonus_pct: 0.35,
    },
    RefineLevel {
        level: 8,
        name: "入圣",
        cost_gold: 100000,
        cost_diamond: 200,
        success_rate: 0.20,
        attr_bonus_pct: 0.50,
    },
    RefineLevel {
        level: 9,
        name: "登峰",
        cost_gold: 200000,
        cost_diamond: 500,
        success_rate: 0.10,
        attr_bonus_pct: 0.75,
    },
    RefineLevel {
        level: 10,
        name: "造极",
        cost_gold: 500000,
        cost_diamond: 1000,
        success_rate: 0.05,
        attr_bonus_pct: 1.00,
    },
];

/// 精炼属性类型
const REFINE_ATTRS: &[(&str, &str)] = &[
    ("HP", "生命"),
    ("AD", "物攻"),
    ("AP", "魔攻"),
    ("Defense", "防御"),
    ("MagicResistance", "魔抗"),
    ("Hit", "命中"),
    ("Dodge", "闪避"),
    ("Crit", "暴击"),
    ("AbsorbHP", "吸血"),
    ("ImmuneDamage", "免伤"),
];

/// 从 Shared_Data 读取玩家精炼等级
fn get_refine_level(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    let result = conn.query_row(
        "SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION='Ext_Attribut_EQ_hzxyx'",
        [user_id],
        |row| row.get::<_, String>(0),
    );
    drop(conn);
    match result {
        Ok(val) => val.parse().unwrap_or(0),
        Err(_) => 0,
    }
}

/// 保存精炼等级到 Shared_Data
fn set_refine_level(db: &Database, user_id: &str, level: i32) {
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, 'Ext_Attribut_EQ_hzxyx', ?2)",
        rusqlite::params![user_id, level.to_string()],
    );
    drop(conn);
}

/// 读取精炼属性值
fn get_refine_values(db: &Database, user_id: &str) -> Vec<i32> {
    let conn = db.lock_conn();
    let result = conn.query_row(
        "SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION='Ext_Var_EqValue_hzxyx'",
        [user_id],
        |row| row.get::<_, String>(0),
    );
    drop(conn);
    match result {
        Ok(val) => {
            if val.is_empty() {
                vec![0; REFINE_ATTRS.len()]
            } else {
                let mut vals: Vec<i32> = val.split(',').filter_map(|s| s.parse().ok()).collect();
                vals.resize(REFINE_ATTRS.len(), 0);
                vals
            }
        }
        Err(_) => vec![0; REFINE_ATTRS.len()],
    }
}

/// 保存精炼属性值
fn set_refine_values(db: &Database, user_id: &str, values: &[i32]) {
    let data = values.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, 'Ext_Var_EqValue_hzxyx', ?2)",
        rusqlite::params![user_id, data],
    );
    drop(conn);
}

/// 简单哈希用于确定性随机
fn simple_hash(seed: u64) -> u64 {
    let mut h = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h
}

/// 查看精炼 — 显示玩家精炼状态和可精炼属性
pub fn cmd_view_refine(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level = get_refine_level(db, user_id);
    let values = get_refine_values(db, user_id);

    let level_def = REFINE_LEVELS
        .iter()
        .find(|l| l.level == level)
        .unwrap_or(&REFINE_LEVELS[0]);
    let next_level = REFINE_LEVELS.iter().find(|l| l.level == level + 1);

    let mut out = format!("{}\n🔨 【装备属性精炼系统】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    out.push_str(&format!("📊 当前精炼等级: {} Lv.{}\n", level_def.name, level));
    out.push_str(&format!("📈 属性加成: +{:.0}%\n\n", level_def.attr_bonus_pct * 100.0));

    out.push_str("📋 【当前精炼属性】\n");
    for (_attr_id, attr_name) in REFINE_ATTRS.iter() {
        let val = values
            .get(REFINE_ATTRS.iter().position(|(_, n)| n == attr_name).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        if val > 0 {
            out.push_str(&format!("  {}: +{}\n", attr_name, val));
        }
    }
    if values.iter().all(|&v| v == 0) {
        out.push_str("  暂无精炼属性加成\n");
    }

    if let Some(next) = next_level {
        out.push_str(&format!("\n⬆️ 下一等级: {} Lv.{}\n", next.name, next.level));
        out.push_str(&format!("   消耗: 💰{}金币", next.cost_gold));
        if next.cost_diamond > 0 {
            out.push_str(&format!(" + 💎{}钻石", next.cost_diamond));
        }
        out.push_str(&format!("\n   成功率: {:.0}%\n", next.success_rate * 100.0));
    } else {
        out.push_str("\n🏆 已达到最高等级！\n");
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「精炼升级」提升精炼等级\n");
    out.push_str("💡 使用「精炼属性+属性名」分配精炼点数\n");
    out.push_str("💡 使用「精炼等级」查看所有精炼等级\n");
    out
}

/// 精炼升级 — 提升精炼等级
pub fn cmd_refine_upgrade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level = get_refine_level(db, user_id);

    let next = match REFINE_LEVELS.iter().find(|l| l.level == level + 1) {
        Some(n) => n,
        None => return format!("{}\n🏆 精炼已达到最高等级 Lv.{}，无法继续升级！", prefix, level),
    };

    // 扣除金币 (try subtract, if negative → insufficient)
    if next.cost_gold > 0 {
        let after_gold = db.modify_currency(user_id, CURRENCY_GOLD, "sub", next.cost_gold);
        if after_gold < 0 {
            db.modify_currency(user_id, CURRENCY_GOLD, "add", next.cost_gold);
            return format!(
                "{}\n⚠️ 金币不足！需要💰{}，当前💰{}",
                prefix,
                next.cost_gold,
                after_gold + next.cost_gold
            );
        }
    }

    // 扣除钻石
    if next.cost_diamond > 0 {
        let after_diamond = db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", next.cost_diamond as i64);
        if after_diamond < 0 {
            // 回滚金币
            if next.cost_gold > 0 {
                db.modify_currency(user_id, CURRENCY_GOLD, "add", next.cost_gold);
            }
            return format!(
                "{}\n⚠️ 钻石不足！需要💎{}，当前💎{}",
                prefix,
                next.cost_diamond,
                after_diamond + next.cost_diamond as i64
            );
        }
    }

    // 判定成功/失败 (确定性随机，基于用户ID+等级+日期)
    let date = chrono::Local::now().format("%Y%m%d").to_string();
    let seed_str = format!("{}{}{}", user_id, level, date);
    let hash = simple_hash(seed_str.len() as u64);
    let roll = (hash % 10000) as f64 / 10000.0;
    let success = roll < next.success_rate;

    if success {
        set_refine_level(db, user_id, next.level);
        let prev = REFINE_LEVELS
            .iter()
            .find(|l| l.level == level)
            .unwrap_or(&REFINE_LEVELS[0]);
        format!(
            "{}\n🎉 精炼成功！\n\n🔨 精炼等级: {} Lv.{} → {} Lv.{}\n📈 属性加成: {:.0}% → {:.0}%\n💰 消耗: {}金币{}",
            prefix,
            prev.name,
            level,
            next.name,
            next.level,
            prev.attr_bonus_pct * 100.0,
            next.attr_bonus_pct * 100.0,
            next.cost_gold,
            if next.cost_diamond > 0 {
                format!(" + {}钻石", next.cost_diamond)
            } else {
                String::new()
            }
        )
    } else {
        let prev = REFINE_LEVELS
            .iter()
            .find(|l| l.level == level)
            .unwrap_or(&REFINE_LEVELS[0]);
        format!(
            "{}\n💥 精炼失败！\n\n🔨 当前等级: {} Lv.{}\n🎲 成功率: {:.0}% (本次未命中)\n💰 已消耗: {}金币{}\n💡 再接再厉！",
            prefix,
            prev.name,
            level,
            next.success_rate * 100.0,
            next.cost_gold,
            if next.cost_diamond > 0 {
                format!(" + {}钻石", next.cost_diamond)
            } else {
                String::new()
            }
        )
    }
}

/// 精炼属性 — 分配精炼点数到指定属性
pub fn cmd_refine_attr(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level = get_refine_level(db, user_id);

    if level == 0 {
        return format!("{}\n⚠️ 精炼等级为0，请先使用「精炼升级」提升精炼等级", prefix);
    }

    let attr_input = args.trim();
    if attr_input.is_empty() {
        let mut out = format!("{}\n📋 可分配的精炼属性:\n", prefix);
        for (attr_id, attr_name) in REFINE_ATTRS {
            out.push_str(&format!("  {} (指令: 精炼属性+{})\n", attr_name, attr_id));
        }
        out.push_str("\n💡 使用「精炼属性+属性名」分配1点精炼属性\n");
        return out;
    }

    // 查找匹配的属性
    let attr_idx = REFINE_ATTRS
        .iter()
        .position(|(id, name)| id.eq_ignore_ascii_case(attr_input) || *name == attr_input);

    let idx = match attr_idx {
        Some(i) => i,
        None => {
            let names: Vec<&str> = REFINE_ATTRS.iter().map(|(_, n)| *n).collect();
            return format!(
                "{}\n⚠️ 未找到属性「{}」\n💡 可选属性: {}",
                prefix,
                attr_input,
                names.join("、")
            );
        }
    };

    // 计算可用点数 (精炼等级 * 5 - 已分配总点数)
    let mut values = get_refine_values(db, user_id);
    let total_used: i32 = values.iter().sum();
    let total_available = level * 5;

    if total_used >= total_available {
        return format!(
            "{}\n⚠️ 精炼点数已用完！\n📊 已分配: {}/{} 点\n💡 提升精炼等级可获得更多点数",
            prefix, total_used, total_available
        );
    }

    values[idx] += 1;
    set_refine_values(db, user_id, &values);

    let (_, attr_name) = REFINE_ATTRS[idx];
    format!(
        "{}\n✅ 精炼属性分配成功！\n\n📊 {}: +{}\n📈 已分配: {}/{} 点\n💡 剩余可分配: {} 点",
        prefix,
        attr_name,
        values[idx],
        total_used + 1,
        total_available,
        total_available - total_used - 1
    )
}

/// 精炼等级 — 查看所有精炼等级定义
pub fn cmd_refine_levels(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current = get_refine_level(db, user_id);

    let mut out = format!("{}\n📊 【精炼等级一览】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for level in REFINE_LEVELS {
        let marker = if level.level == current { " ◀ 当前" } else { "" };
        if level.level == 0 {
            out.push_str(&format!("  Lv.0 {} (基础){}\n", level.name, marker));
        } else {
            out.push_str(&format!(
                "  Lv.{} {} | +{:.0}% | 成功{:.0}% | 💰{}{}\n",
                level.level,
                level.name,
                level.attr_bonus_pct * 100.0,
                level.success_rate * 100.0,
                level.cost_gold,
                if level.cost_diamond > 0 {
                    format!("+💎{}", level.cost_diamond)
                } else {
                    String::new()
                },
            ));
            if !marker.is_empty() {
                out.push_str(&format!("                  {}\n", marker));
            }
        }
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 精炼等级越高，属性加成越多，但成功率越低\n");
    out.push_str("💡 每个精炼等级提供5点可分配属性\n");
    out
}

/// 精炼排行 — 全服精炼等级排名 Top10 + 当前用户定位
pub fn cmd_refine_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 查询所有有精炼等级的玩家 (使用 block scope 避免 borrow 问题)
    let mut entries: Vec<(String, String, i32)> = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare(
            "SELECT sd.ID, sd.DATA, u.Name FROM Shared_Data sd \
             LEFT JOIN Basic_User u ON sd.ID = u.User \
             WHERE sd.SECTION = 'Ext_Attribut_EQ_hzxyx' AND sd.DATA != '0' AND sd.DATA != '' \
             ORDER BY CAST(sd.DATA AS INTEGER) DESC",
        ) {
            Ok(s) => s,
            Err(e) => return format!("{}\n❌ 查询失败: {}", prefix, e),
        };

        let mut result = Vec::new();
        if let Ok(rows) = stmt.query_map([], |row| {
            let uid: String = row.get(0).unwrap_or_default();
            let data: String = row.get(1).unwrap_or_default();
            let name: String = row.get(2).unwrap_or_default();
            let level: i32 = data.parse().unwrap_or(0);
            Ok((uid, name, level))
        }) {
            for row in rows.flatten() {
                result.push(row);
            }
        }
        result
    };

    if entries.is_empty() {
        return format!(
            "{}\n🏆 暂无精炼排行数据！\n\n💡 使用「精炼升级」提升精炼等级后即可上榜",
            prefix
        );
    }

    let mut result = format!("{}\n═══ 🔨 精炼排行榜 ═══", prefix);
    result.push_str("\n━━━━━━━━━━━━━━━━━━━━");

    // 按精炼等级降序排列
    entries.sort_by_key(|b| std::cmp::Reverse(b.2));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, name, level)) in entries.iter().take(10).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let display_name = if name.is_empty() { uid.as_str() } else { name.as_str() };
        let level_def = REFINE_LEVELS.iter().find(|l| l.level == *level);
        let level_name = level_def.map(|l| l.name).unwrap_or("未知");
        let bonus = level_def
            .map(|l| format!("+{:.0}%", l.attr_bonus_pct * 100.0))
            .unwrap_or_default();

        result.push_str(&format!(
            "\n{} {}. {} — {} Lv.{} ({})",
            medal,
            i + 1,
            display_name,
            level_name,
            level,
            bonus
        ));
    }

    // 当前用户排名
    if let Some((_, name, level)) = entries.iter().find(|(uid, _, _)| uid == user_id) {
        let rank = entries.iter().position(|(uid, _, _)| uid == user_id).unwrap_or(0) + 1;
        let display_name = if name.is_empty() { "您" } else { name.as_str() };
        let level_def = REFINE_LEVELS.iter().find(|l| l.level == *level);
        let level_name = level_def.map(|l| l.name).unwrap_or("未知");
        result.push_str(&format!(
            "\n\n📍 {}的排名: 第{}名 — {} Lv.{}",
            display_name, rank, level_name, level
        ));
    }

    result.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    result.push_str("\n💡 使用「精炼升级」提升精炼等级\n💡 使用「查看精炼」查看当前精炼状态");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refine_level_count() {
        assert_eq!(REFINE_LEVELS.len(), 11);
        assert_eq!(REFINE_LEVELS[0].level, 0);
        assert_eq!(REFINE_LEVELS[10].level, 10);
    }

    #[test]
    fn test_refine_level_costs_increase() {
        for i in 1..REFINE_LEVELS.len() {
            assert!(REFINE_LEVELS[i].cost_gold >= REFINE_LEVELS[i - 1].cost_gold);
        }
    }

    #[test]
    fn test_refine_success_rate_decrease() {
        for i in 2..REFINE_LEVELS.len() {
            assert!(REFINE_LEVELS[i].success_rate <= REFINE_LEVELS[i - 1].success_rate);
        }
    }

    #[test]
    fn test_refine_attr_bonus_increase() {
        for i in 1..REFINE_LEVELS.len() {
            assert!(REFINE_LEVELS[i].attr_bonus_pct >= REFINE_LEVELS[i - 1].attr_bonus_pct);
        }
    }

    #[test]
    fn test_refine_attrs_count() {
        assert_eq!(REFINE_ATTRS.len(), 10);
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash(42);
        let h2 = simple_hash(42);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_refine_ranking_level_names() {
        // Verify each refine level has a unique name
        let mut names: Vec<&str> = REFINE_LEVELS.iter().map(|l| l.name).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len, "Refine level names should be unique");
    }
}

//! CakeGame 属性克制系统
//! 通过 Ext_Attribut_Avxav_hzxyx / Ext_Attribut_OCC_hzxyx / Ext_Attribut_MON_hzxyx 实现
//! 7种类型: ad(攻击), ap(法术), ae(元素), be(基础), pw(力量), tk(坦克), doom(末日)

use crate::db::Database;

/// 属性克制结果
pub struct TypeEffectiveness {
    /// 克制倍率百分比 (100 = 1x, 120 = 1.2x, 80 = 0.8x)
    pub multiplier: i32,
    /// 攻击者类型
    pub attacker_type: String,
    /// 防御者类型
    pub defender_type: String,
}

/// 获取职业对应属性类型
/// Ext_Attribut_OCC_hzxyx: Name(职业名) → Avname(类型)
pub fn get_occupation_type(db: &Database, occupation: &str) -> String {
    let conn = db.lock_conn();
    let result = conn
        .prepare("SELECT Avname FROM Ext_Attribut_OCC_hzxyx WHERE Name = ?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![occupation], |row| row.get::<_, String>(0))
                .ok()
        });
    result.unwrap_or_else(|| "be".to_string())
}

/// 获取怪物对应属性类型
/// Ext_Attribut_MON_hzxyx: Name(怪物名) → Avname(类型)
/// 大多数怪物是"无"，视为"be"(基础类型)
pub fn get_monster_type(db: &Database, monster_name: &str) -> String {
    let conn = db.lock_conn();
    let result = conn
        .prepare("SELECT Avname FROM Ext_Attribut_MON_hzxyx WHERE Name = ?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![monster_name], |row| row.get::<_, String>(0))
                .ok()
        });
    match result {
        Some(t) if t == "无" || t.is_empty() => "be".to_string(),
        Some(t) => t,
        None => "be".to_string(),
    }
}

/// 获取属性克制倍率
/// Ext_Attribut_Avxav_hzxyx: Name(format: "attacker@defender") → Value(百分比)
pub fn get_type_multiplier(db: &Database, attacker_type: &str, defender_type: &str) -> i32 {
    let key = format!("{}@{}", attacker_type, defender_type);
    let conn = db.lock_conn();
    let result = conn
        .prepare("SELECT Value FROM Ext_Attribut_Avxav_hzxyx WHERE Name = ?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![key], |row| row.get::<_, String>(0))
                .ok()
        });
    result.and_then(|v| v.parse::<i32>().ok()).unwrap_or(100)
}

/// 计算属性克制效果
/// 返回 TypeEffectiveness，包含倍率和类型信息
pub fn calc_type_effectiveness(db: &Database, user_id: &str, monster_name: &str) -> TypeEffectiveness {
    // 获取用户职业类型
    let occupation = db.read_basic(user_id, crate::core::ITEM_OCCUPATION);
    let attacker_type = get_occupation_type(db, &occupation);

    // 获取怪物类型
    let defender_type = get_monster_type(db, monster_name);

    // 获取克制倍率
    let multiplier = get_type_multiplier(db, &attacker_type, &defender_type);

    TypeEffectiveness {
        multiplier,
        attacker_type,
        defender_type,
    }
}

/// 格式化克制提示文本
pub fn format_type_hint(te: &TypeEffectiveness) -> String {
    if te.multiplier > 110 {
        format!(
            "⚔ 属性克制！效果拔群！({} vs {}, {}%)",
            type_name(&te.attacker_type),
            type_name(&te.defender_type),
            te.multiplier
        )
    } else if te.multiplier < 90 {
        format!(
            "⚔ 属性不利...效果不太好...({} vs {}, {}%)",
            type_name(&te.attacker_type),
            type_name(&te.defender_type),
            te.multiplier
        )
    } else if te.multiplier == 100 {
        String::new() // 平等克制不显示
    } else if te.multiplier > 100 {
        format!(
            "⚔ 微弱克制 ({} vs {}, {}%)",
            type_name(&te.attacker_type),
            type_name(&te.defender_type),
            te.multiplier
        )
    } else {
        format!(
            "⚔ 微弱不利 ({} vs {}, {}%)",
            type_name(&te.attacker_type),
            type_name(&te.defender_type),
            te.multiplier
        )
    }
}

/// 类型代码转中文名
pub fn type_name(t: &str) -> &str {
    match t {
        "ad" => "攻击",
        "ap" => "法术",
        "ae" => "元素",
        "be" => "基础",
        "pw" => "力量",
        "tk" => "坦克",
        "doom" => "末日",
        _ => t,
    }
}

/// 所有已知类型代码
const ALL_TYPES: &[&str] = &["ad", "ap", "ae", "be", "pw", "tk", "doom"];

/// 获取所有职业→类型映射
pub fn get_all_occupation_types(db: &Database) -> Vec<(String, String)> {
    let conn = db.lock_conn();
    let mut result = Vec::new();
    if let Ok(mut stmt) = conn.prepare("SELECT Name, Avname FROM Ext_Attribut_OCC_hzxyx") {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                result.push((row.0, row.1));
            }
        }
    }
    result
}

/// 获取完整属性克制表 (所有attacker@defender组合)
#[allow(dead_code)]
pub fn get_all_type_matchups(db: &Database) -> Vec<(String, String, i32)> {
    let conn = db.lock_conn();
    let mut result = Vec::new();
    if let Ok(mut stmt) = conn.prepare("SELECT Name, Value FROM Ext_Attribut_Avxav_hzxyx") {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                let parts: Vec<&str> = row.0.split('@').collect();
                if parts.len() == 2 {
                    let val: i32 = row.1.parse().unwrap_or(100);
                    result.push((parts[0].to_string(), parts[1].to_string(), val));
                }
            }
        }
    }
    result
}

/// 格式化属性克制图鉴总览
pub fn format_type_chart_overview(db: &Database, user_id: &str) -> String {
    let mut out = String::from("══════ 类型克制图鉴 ══════\n\n");

    // 获取用户当前职业类型
    let occupation = db.read_basic(user_id, crate::core::ITEM_OCCUPATION);
    let my_type = get_occupation_type(db, &occupation);
    out.push_str(&format!(
        "📖 你的职业: {} → 类型: {} ({})\n\n",
        occupation,
        type_name(&my_type),
        my_type
    ));

    // 克制关系概述
    out.push_str("⚔ 属性克制关系:\n");
    out.push_str("  攻击(ad) → 克制 法术(ap)\n");
    out.push_str("  法术(ap) → 克制 力量(pw)\n");
    out.push_str("  力量(pw) → 克制 攻击(ad)\n");
    out.push_str("  坦克(tk) → 克制 全面(克制减伤)\n");
    out.push_str("  元素(ae) → 均衡型\n");
    out.push_str("  基础(be) → 无特殊克制\n\n");

    // 用户的克制关系
    out.push_str(&format!("📊 {}({}) 的克制数据:\n", type_name(&my_type), my_type));
    for def_type in ALL_TYPES {
        let mult = get_type_multiplier(db, &my_type, def_type);
        let arrow = if mult > 110 {
            "⬆ 克制"
        } else if mult > 100 {
            "↗ 微优"
        } else if mult == 100 {
            "＝ 平衡"
        } else if mult >= 90 {
            "↘ 微劣"
        } else {
            "⬇ 不利"
        };
        out.push_str(&format!(
            "  {} vs {}({}): {}% {}\n",
            type_name(&my_type),
            type_name(def_type),
            def_type,
            mult,
            arrow
        ));
    }

    out.push_str("\n💡 使用 \"类型图鉴\" 查看完整克制矩阵\n");
    out.push_str("💡 使用 \"职业类型\" 查看各职业对应类型\n");
    out
}

/// 格式化完整类型克制矩阵
#[allow(dead_code)]
pub fn format_full_type_matrix(db: &Database) -> String {
    let mut out = String::from("══════ 属性克制矩阵 ══════\n\n");
    out.push_str("行=攻击方, 列=防御方, 数值=伤害百分比\n");
    out.push_str("⬆>110克制 ↗>100微优 ＝100平衡 ↘<100微劣 ⬇<90不利\n\n");

    // 表头
    out.push_str("攻\\防 ");
    for t in ALL_TYPES {
        out.push_str(&format!("{:>6}", type_name(t)));
    }
    out.push('\n');

    // 每一行
    for atk in ALL_TYPES {
        out.push_str(&format!("{:>4} ", type_name(atk)));
        for def in ALL_TYPES {
            let mult = get_type_multiplier(db, atk, def);
            let marker = if mult > 110 {
                "▲"
            } else if mult > 100 {
                "△"
            } else if mult == 100 {
                "○"
            } else if mult >= 90 {
                "▽"
            } else {
                "▼"
            };
            out.push_str(&format!("{:>5}{}", mult, marker));
        }
        out.push('\n');
    }

    out.push_str("\n图例: ▲强克(>110%) △微克(>100%) ○平衡(100%) ▽微弱(<100%) ▼弱势(<90%)\n");
    out
}

/// 格式化职业类型列表
pub fn format_occupation_types(db: &Database) -> String {
    let mut out = String::from("══════ 职业类型一览 ══════\n\n");

    let occ_types = get_all_occupation_types(db);
    if occ_types.is_empty() {
        out.push_str("暂无职业类型数据\n");
        return out;
    }

    for (name, tcode) in &occ_types {
        let emoji = match tcode.as_str() {
            "ad" => "⚔",
            "ap" => "🔮",
            "ae" => "✨",
            "be" => "🛡",
            "pw" => "💪",
            "tk" => "🏰",
            "doom" => "💀",
            _ => "❓",
        };
        out.push_str(&format!("  {} {} → {} ({})\n", emoji, name, type_name(tcode), tcode));
    }

    out.push_str("\n💡 使用 \"查看属性克制+怪物名\" 查看对特定怪物的克制效果\n");
    out.push_str("💡 使用 \"类型图鉴\" 查看完整克制矩阵\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_name_lookup() {
        assert_eq!(type_name("ad"), "攻击");
        assert_eq!(type_name("ap"), "法术");
        assert_eq!(type_name("tk"), "坦克");
        assert_eq!(type_name("unknown"), "unknown");
    }

    #[test]
    fn test_all_types_constant() {
        assert_eq!(ALL_TYPES.len(), 7);
        assert!(ALL_TYPES.contains(&"ad"));
        assert!(ALL_TYPES.contains(&"tk"));
        assert!(ALL_TYPES.contains(&"doom"));
    }

    #[test]
    fn test_type_name_owned() {
        assert_eq!(type_name("pw"), "力量");
        assert_eq!(type_name("ae"), "元素");
    }
}

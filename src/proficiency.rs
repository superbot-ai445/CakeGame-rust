//! 技能熟练度系统 (Skill Proficiency System)
//!
//! 跟踪玩家每个技能的使用熟练度，使用次数越多熟练度越高。
//! 熟练度越高，技能伤害加成越大。
//!
//! 来源: Skill_Register 表 (1326条记录)
//! 列: User, SkillName, Proficiency, LastUseDate, Occupation
//!
//! 指令: 查看熟练度, 技能熟练度+技能名, 熟练度排行, 熟练度加成
//!
//! 熟练度等级:
//!   新手 (0-99):     无加成
//!   学徒 (100-299):  +2% 技能伤害
//!   熟练 (300-599):  +5% 技能伤害
//!   精通 (600-999):  +8% 技能伤害
//!   大师 (1000+):    +12% 技能伤害

use crate::db::Database;

/// 熟练度等级定义
pub struct ProficiencyTier {
    pub name: &'static str,
    pub min: i32,
    pub max: i32,
    pub damage_bonus_pct: f64,
    pub emoji: &'static str,
}

/// 熟练度等级表
pub const TIERS: &[ProficiencyTier] = &[
    ProficiencyTier {
        name: "新手",
        min: 0,
        max: 99,
        damage_bonus_pct: 0.0,
        emoji: "🌱",
    },
    ProficiencyTier {
        name: "学徒",
        min: 100,
        max: 299,
        damage_bonus_pct: 2.0,
        emoji: "📗",
    },
    ProficiencyTier {
        name: "熟练",
        min: 300,
        max: 599,
        damage_bonus_pct: 5.0,
        emoji: "📘",
    },
    ProficiencyTier {
        name: "精通",
        min: 600,
        max: 999,
        damage_bonus_pct: 8.0,
        emoji: "📙",
    },
    ProficiencyTier {
        name: "大师",
        min: 1000,
        max: i32::MAX,
        damage_bonus_pct: 12.0,
        emoji: "🏆",
    },
];

/// 获取熟练度等级信息
pub fn get_tier(proficiency: i32) -> &'static ProficiencyTier {
    for tier in TIERS.iter() {
        if proficiency >= tier.min && proficiency <= tier.max {
            return tier;
        }
    }
    TIERS.last().unwrap()
}

/// 获取用户某个技能的熟练度
pub fn get_skill_proficiency(db: &Database, user_id: &str, skill_name: &str) -> i32 {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Proficiency FROM Skill_Register WHERE User = ?1 AND SkillName = ?2") {
        Ok(s) => s,
        Err(_) => return 0,
    };
    stmt.query_row(rusqlite::params![user_id, skill_name], |row| {
        let prof_str: String = row.get(0).unwrap_or_default();
        Ok(prof_str.parse().unwrap_or(0))
    })
    .unwrap_or(0)
}

/// 增加技能熟练度（每次使用技能时调用）
pub fn increase_proficiency(db: &Database, user_id: &str, skill_name: &str, occupation: &str) -> i32 {
    let current = get_skill_proficiency(db, user_id, skill_name);
    let new_prof = current + 1;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let conn = db.lock_conn();
    // UPSERT: 更新或插入
    let _ = conn.execute(
        "INSERT INTO Skill_Register (User, SkillName, Proficiency, LastUseDate, Occupation)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(User, SkillName) DO UPDATE SET Proficiency = ?3, LastUseDate = ?4",
        rusqlite::params![user_id, skill_name, new_prof.to_string(), now, occupation],
    );
    new_prof
}

/// 获取技能伤害加成百分比
pub fn get_damage_bonus_pct(db: &Database, user_id: &str, skill_name: &str) -> f64 {
    let prof = get_skill_proficiency(db, user_id, skill_name);
    let tier = get_tier(prof);
    tier.damage_bonus_pct
}

/// 获取用户所有技能熟练度列表
pub fn get_all_proficiencies(db: &Database, user_id: &str) -> Vec<(String, i32, String, String)> {
    // 返回: (技能名, 熟练度, 等级名, 职业)
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT SkillName, Proficiency, Occupation FROM Skill_Register WHERE User = ?1 ORDER BY CAST(Proficiency AS INTEGER) DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([user_id], |row| {
        let name: String = row.get(0).unwrap_or_default();
        let prof_str: String = row.get(1).unwrap_or_default();
        let occ: String = row.get(2).unwrap_or_default();
        let prof: i32 = prof_str.parse().unwrap_or(0);
        let tier_name = get_tier(prof).name.to_string();
        Ok((name, prof, tier_name, occ))
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

// ==================== 指令实现 ====================

/// 查看熟练度 — 显示玩家所有技能的熟练度
pub fn cmd_view_proficiency(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let profs = get_all_proficiencies(db, user_id);
    if profs.is_empty() {
        return "📜 你还没有使用过任何技能！\n💡 使用技能战斗即可提升熟练度。".to_string();
    }

    let nickname = db.read_basic(user_id, "NickName");
    let mut out = format!("═══ {} 的技能熟练度 ═══\n\n", nickname);

    for (i, (name, prof, tier_name, occ)) in profs.iter().take(20).enumerate() {
        let tier = get_tier(*prof);
        let bonus_str = if tier.damage_bonus_pct > 0.0 {
            format!(" (+{:.0}%)", tier.damage_bonus_pct)
        } else {
            String::new()
        };
        let progress = if *prof < 1000 {
            let next_threshold = TIERS.iter().find(|t| t.min > *prof).map(|t| t.min).unwrap_or(1000);
            let pct = (*prof as f64 / next_threshold as f64 * 10.0).min(10.0) as usize;
            let bar: String = "█".repeat(pct) + &"░".repeat(10 - pct);
            format!(" → 下一级: [{}] {}/{}", bar, prof, next_threshold)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{}. {}{} [{}] Lv.{}{}\n   职业: {}{}\n",
            i + 1,
            tier.emoji,
            name,
            tier_name,
            prof,
            bonus_str,
            occ,
            progress
        ));
    }

    if profs.len() > 20 {
        out.push_str(&format!("\n... 共 {} 个技能（显示前20个）\n", profs.len()));
    }

    out.push_str("\n💡 熟练度等级: 🌱新手(0) → 📗学徒(100) → 📘熟练(300) → 📙精通(600) → 🏆大师(1000)");
    out
}

/// 技能熟练度+技能名 — 查看特定技能的详细熟练度
pub fn cmd_proficiency_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let skill_name = args.trim();
    if skill_name.is_empty() {
        return "📜 请指定技能名！\n💡 用法: 技能熟练度+技能名".to_string();
    }

    let prof = get_skill_proficiency(db, user_id, skill_name);
    if prof == 0 {
        // 检查是否存在于 Config_Skills
        let conn = db.lock_conn();
        let exists: bool = conn
            .prepare("SELECT COUNT(*) FROM Config_Skills WHERE Item=?1")
            .ok()
            .and_then(|mut s| s.query_row([skill_name], |row| row.get::<_, i32>(0)).ok())
            .map(|c| c > 0)
            .unwrap_or(false);

        if exists {
            return format!(
                "📜 技能 [{}] 的熟练度: 0\n🌱 等级: 新手 (无加成)\n💡 使用该技能战斗即可提升熟练度！",
                skill_name
            );
        } else {
            return format!("❌ 未找到技能 [{}]！", skill_name);
        }
    }

    let tier = get_tier(prof);
    let nickname = db.read_basic(user_id, "NickName");

    let next_threshold = TIERS.iter().find(|t| t.min > prof).map(|t| t.min);

    let mut out = format!("═══ {} — {} 熟练度详情 ═══\n\n", nickname, skill_name);
    out.push_str(&format!("📊 当前熟练度: {}\n", prof));
    out.push_str(&format!("{} 等级: {}\n", tier.emoji, tier.name));
    if tier.damage_bonus_pct > 0.0 {
        out.push_str(&format!("⚔️ 技能伤害加成: +{:.0}%\n", tier.damage_bonus_pct));
    } else {
        out.push_str("⚔️ 技能伤害加成: 无\n");
    }

    if let Some(next) = next_threshold {
        let remaining = next - prof;
        let pct = (prof as f64 / next as f64 * 100.0).min(100.0);
        let bar_len = 20;
        let filled = (pct / 100.0 * bar_len as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
        out.push_str(&format!("\n📈 升级进度: [{}] {:.1}%\n", bar, pct));
        out.push_str(&format!("⏳ 距离下一级还需: {} 次使用\n", remaining));
    } else {
        out.push_str("\n🏆 已达到最高等级！\n");
    }

    out.push_str("\n🎯 熟练度等级路线:\n");
    for t in TIERS.iter() {
        let marker = if prof >= t.min && prof <= t.max {
            " ← 当前"
        } else {
            ""
        };
        out.push_str(&format!(
            "  {} {}: {}~{} (+{:.0}%){}\n",
            t.emoji,
            t.name,
            t.min,
            if t.max == i32::MAX {
                "∞".to_string()
            } else {
                t.max.to_string()
            },
            t.damage_bonus_pct,
            marker
        ));
    }

    out
}

/// 熟练度排行 — 全服技能熟练度排行
pub fn cmd_proficiency_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let filter_skill = args.trim();

    // Collect raw data first; drop conn before looking up nicknames to avoid deadlock
    let rows: Vec<(String, i32)> = {
        let conn = db.lock_conn();

        if filter_skill.is_empty() {
            let mut stmt = match conn.prepare(
                "SELECT User, SUM(CAST(Proficiency AS INTEGER)) as total
                 FROM Skill_Register WHERE User != ''
                 GROUP BY User ORDER BY total DESC LIMIT 15",
            ) {
                Ok(s) => s,
                Err(e) => return format!("❌ 查询失败: {}", e),
            };
            stmt.query_map([], |row| {
                let uid: String = row.get(0).unwrap_or_default();
                let total: i32 = row.get(1).unwrap_or(0);
                Ok((uid, total))
            })
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        } else {
            let mut stmt = match conn.prepare(
                "SELECT User, CAST(Proficiency AS INTEGER) as prof
                 FROM Skill_Register WHERE SkillName = ?1 AND User != ''
                 ORDER BY prof DESC LIMIT 15",
            ) {
                Ok(s) => s,
                Err(e) => return format!("❌ 查询失败: {}", e),
            };
            stmt.query_map([filter_skill], |row| {
                let uid: String = row.get(0).unwrap_or_default();
                let prof: i32 = row.get(1).unwrap_or(0);
                Ok((uid, prof))
            })
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        }
    }; // conn dropped here — safe to call db.read_basic now

    if rows.is_empty() {
        if filter_skill.is_empty() {
            return "═══ 🏅 全服技能熟练度排行 ═══\n\n📊 暂无熟练度数据！\n💡 使用技能战斗即可积累熟练度。".to_string();
        }
        return format!("📊 技能 [{}] 暂无熟练度数据！", filter_skill);
    }

    let mut out = if filter_skill.is_empty() {
        "═══ 🏅 全服技能熟练度排行 ═══\n\n".to_string()
    } else {
        format!("═══ 🏅 [{}] 熟练度排行 ═══\n\n", filter_skill)
    };

    for (i, (uid, prof)) in rows.iter().enumerate() {
        let nick = db.read_basic(uid, "NickName");
        let tier = get_tier(*prof);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let bonus_str = if tier.damage_bonus_pct > 0.0 {
            format!(" (+{:.0}%)", tier.damage_bonus_pct)
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{} {}. {} — Lv.{} [{}]{}\n",
            medal,
            i + 1,
            nick,
            prof,
            tier.name,
            bonus_str
        ));
    }

    if filter_skill.is_empty() {
        out.push_str("\n💡 用法: 熟练度排行+技能名 查看特定技能排行");
    }

    out.push_str("\n📊 你的总熟练度: ");
    let my_profs = get_all_proficiencies(db, user_id);
    let my_total: i32 = my_profs.iter().map(|p| p.1).sum();
    let my_tier = get_tier(my_total);
    out.push_str(&format!("{} [{}]", my_total, my_tier.name));

    out
}

/// 熟练度加成 — 查看当前技能的伤害加成
pub fn cmd_proficiency_bonus(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let nickname = db.read_basic(user_id, "NickName");
    let profs = get_all_proficiencies(db, user_id);

    if profs.is_empty() {
        return format!(
            "📜 {} 还没有任何技能熟练度！\n💡 使用技能战斗即可积累熟练度。",
            nickname
        );
    }

    let mut out = format!("═══ {} 的技能加成总览 ═══\n\n", nickname);

    let mut total_bonus = 0.0f64;
    let mut bonus_count = 0;

    for (name, prof, tier_name, _) in profs.iter() {
        let tier = get_tier(*prof);
        if tier.damage_bonus_pct > 0.0 {
            bonus_count += 1;
            total_bonus += tier.damage_bonus_pct;
            out.push_str(&format!(
                "  ⚔️ {} [{}] Lv.{} → +{:.0}%伤害\n",
                name, tier_name, prof, tier.damage_bonus_pct
            ));
        }
    }

    if bonus_count == 0 {
        out.push_str("  暂无技能达到学徒等级(100级)获得加成\n");
    } else {
        out.push_str(&format!("\n📊 共 {} 个技能有伤害加成\n", bonus_count));
        out.push_str(&format!("📊 平均加成: +{:.1}%\n", total_bonus / bonus_count as f64));
    }

    out.push_str("\n💡 熟练度等级:\n");
    for t in TIERS.iter() {
        out.push_str(&format!(
            "  {} {}: {}~{} → +{:.0}%伤害\n",
            t.emoji,
            t.name,
            t.min,
            if t.max == i32::MAX {
                "∞".to_string()
            } else {
                t.max.to_string()
            },
            t.damage_bonus_pct
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_lookup() {
        assert_eq!(get_tier(0).name, "新手");
        assert_eq!(get_tier(50).name, "新手");
        assert_eq!(get_tier(99).name, "新手");
        assert_eq!(get_tier(100).name, "学徒");
        assert_eq!(get_tier(299).name, "学徒");
        assert_eq!(get_tier(300).name, "熟练");
        assert_eq!(get_tier(599).name, "熟练");
        assert_eq!(get_tier(600).name, "精通");
        assert_eq!(get_tier(999).name, "精通");
        assert_eq!(get_tier(1000).name, "大师");
        assert_eq!(get_tier(99999).name, "大师");
    }

    #[test]
    fn test_damage_bonus() {
        assert_eq!(get_tier(0).damage_bonus_pct, 0.0);
        assert_eq!(get_tier(100).damage_bonus_pct, 2.0);
        assert_eq!(get_tier(300).damage_bonus_pct, 5.0);
        assert_eq!(get_tier(600).damage_bonus_pct, 8.0);
        assert_eq!(get_tier(1000).damage_bonus_pct, 12.0);
    }
}

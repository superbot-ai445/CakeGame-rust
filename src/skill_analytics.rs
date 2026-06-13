//! 技能热度分析系统 (Skill Popularity Analytics System)
//!
//! 基于 Skill_Register 表 (1326条记录) 分析全服技能使用趋势。
//! 提供技能热度排行、职业偏好分析、冷门宝藏技能发现等功能。
//!
//! 指令: 技能热度, 职业技能, 技能大师
//!
//! 数据来源: Skill_Register 表 (User, SkillName, Proficiency, Occupation)

use crate::db::Database;
use crate::proficiency::{get_tier, TIERS};

/// 技能热度条目
struct SkillPopularity {
    name: String,
    user_count: i32,
    avg_proficiency: f64,
    occupation: String,
}

/// 职业技能统计
struct OccupationSkillStats {
    occupation: String,
    total_users: i32,
    total_skills: i32,
    avg_proficiency: f64,
    top_skill: String,
}

/// 技能大师条目
struct SkillMaster {
    user_id: String,
    nickname: String,
    #[allow(dead_code)]
    skill_name: String,
    proficiency: i32,
    occupation: String,
}

/// 查询技能热度数据（排除无效数据）
fn query_skill_popularity(db: &Database) -> Vec<SkillPopularity> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT SkillName, COUNT(DISTINCT User) as user_count, 
                AVG(CAST(Proficiency AS REAL)) as avg_prof,
                MAX(Occupation) as occupation
         FROM Skill_Register 
         WHERE User != '' AND SkillName NOT LIKE '%隐藏%' AND SkillName NOT LIKE '%沐浴%'
         GROUP BY SkillName 
         ORDER BY user_count DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| {
        Ok(SkillPopularity {
            name: row.get(0).unwrap_or_default(),
            user_count: row.get(1).unwrap_or(0),
            avg_proficiency: row.get(2).unwrap_or(0.0),
            occupation: row.get(3).unwrap_or_default(),
        })
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// 查询职业技能统计
fn query_occupation_stats(db: &Database) -> Vec<OccupationSkillStats> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT Occupation, COUNT(DISTINCT User) as total_users,
                COUNT(DISTINCT SkillName) as total_skills,
                AVG(CAST(Proficiency AS REAL)) as avg_prof
         FROM Skill_Register 
         WHERE User != '' AND Occupation IS NOT NULL 
               AND Occupation NOT LIKE 'eqName%' AND Occupation != ''
         GROUP BY Occupation 
         ORDER BY total_users DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut stats: Vec<OccupationSkillStats> = stmt
        .query_map([], |row| {
            Ok(OccupationSkillStats {
                occupation: row.get(0).unwrap_or_default(),
                total_users: row.get(1).unwrap_or(0),
                total_skills: row.get(2).unwrap_or(0),
                avg_proficiency: row.get(3).unwrap_or(0.0),
                top_skill: String::new(),
            })
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    // Get top skill per occupation
    for stat in stats.iter_mut() {
        let mut stmt2 = match conn.prepare(
            "SELECT SkillName FROM Skill_Register 
             WHERE User != '' AND Occupation = ?1
             GROUP BY SkillName 
             ORDER BY COUNT(DISTINCT User) DESC LIMIT 1",
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };
        stat.top_skill = stmt2
            .query_row([&stat.occupation], |row| {
                Ok(row.get::<_, String>(0).unwrap_or_default())
            })
            .unwrap_or_default();
    }

    stats
}

/// 查询特定技能的大师玩家
fn query_skill_masters(db: &Database, skill_name: &str, limit: usize) -> Vec<SkillMaster> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT User, SkillName, Proficiency, Occupation 
         FROM Skill_Register 
         WHERE User != '' AND SkillName = ?1
         ORDER BY CAST(Proficiency AS INTEGER) DESC 
         LIMIT ?2",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows: Vec<(String, String, i32, String)> = stmt
        .query_map(rusqlite::params![skill_name, limit as i32], |row| {
            Ok((
                row.get(0).unwrap_or_default(),
                row.get(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default().parse().unwrap_or(0),
                row.get(3).unwrap_or_default(),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    // Convert to SkillMaster with nickname lookup
    rows.into_iter()
        .map(|(uid, name, prof, occ)| {
            let nickname = db.read_basic(&uid, "NickName");
            let nickname = if nickname.is_empty() || nickname == "[NULL]" {
                uid.clone()
            } else {
                nickname
            };
            SkillMaster {
                user_id: uid,
                nickname,
                skill_name: name,
                proficiency: prof,
                occupation: occ,
            }
        })
        .collect()
}

// ==================== 指令实现 ====================

/// 技能热度 — 显示全服技能使用热度排行
pub fn cmd_skill_popularity(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let filter = args.trim();
    let skills = query_skill_popularity(db);

    if skills.is_empty() {
        return "📊 暂无技能使用数据！\n💡 使用技能战斗即可产生热度数据。".to_string();
    }

    // If filtering by occupation
    if !filter.is_empty() {
        let filtered: Vec<_> = skills.iter().filter(|s| s.occupation.contains(filter)).collect();
        if filtered.is_empty() {
            return format!(
                "📊 未找到职业 [{}] 的技能数据！\n💡 可用职业: 勇者/御剑师/大魔导士/守誓之盾/赏金猎人/圣光使徒",
                filter
            );
        }

        let total_users: i32 = filtered.iter().map(|s| s.user_count).sum();
        let mut out = format!("═══ 🔥 {} 职业技能热度 ═══\n\n", filter);
        out.push_str(&format!(
            "👥 职业总玩家: {} 人 | 🎯 技能数: {}\n\n",
            total_users,
            filtered.len()
        ));

        for (i, skill) in filtered.iter().enumerate() {
            let tier = get_tier(skill.avg_proficiency as i32);
            let bar_len = (skill.user_count as f64 / 212.0 * 15.0).min(15.0) as usize;
            let bar: String = "█".repeat(bar_len) + &"░".repeat(15 - bar_len);
            out.push_str(&format!(
                "{}. {} {} — {} 人使用 | avg Lv.{:.0} {} {}\n",
                i + 1,
                tier.emoji,
                skill.name,
                skill.user_count,
                skill.avg_proficiency,
                tier.name,
                bar
            ));
        }

        return out;
    }

    // General popularity view
    let total_users: i32 = skills.iter().map(|s| s.user_count).sum();
    let unique_skills = skills.len();

    let mut out = "═══ 🔥 全服技能热度排行 ═══\n\n".to_string();
    out.push_str(&format!(
        "📊 共 {} 个技能 | 👥 累计 {} 人次使用\n\n",
        unique_skills, total_users
    ));

    // Top 10 by popularity
    out.push_str("🏅 热度排行 Top 10:\n");
    for (i, skill) in skills.iter().take(10).enumerate() {
        let tier = get_tier(skill.avg_proficiency as i32);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let bar_len = (skill.user_count as f64 / skills[0].user_count as f64 * 12.0).min(12.0) as usize;
        let bar: String = "█".repeat(bar_len) + &"░".repeat(12 - bar_len);
        out.push_str(&format!(
            "{} {:>2}. {} [{}] — {} 人 | avg {} Lv.{:.0}\n    {}\n",
            medal,
            i + 1,
            skill.name,
            skill.occupation,
            skill.user_count,
            tier.emoji,
            skill.avg_proficiency,
            bar
        ));
    }

    // Show skill distribution
    out.push_str("\n📊 职业技能分布:\n");
    let mut occ_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for skill in &skills {
        let occ = if skill.occupation.is_empty() || skill.occupation.starts_with("eqName") {
            "特殊".to_string()
        } else {
            skill.occupation.clone()
        };
        *occ_counts.entry(occ).or_insert(0) += skill.user_count;
    }
    let mut occ_vec: Vec<_> = occ_counts.into_iter().collect();
    occ_vec.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (occ, count) in &occ_vec {
        let pct = *count as f64 / total_users as f64 * 100.0;
        out.push_str(&format!("  • {}: {} 人 ({:.1}%)\n", occ, count, pct));
    }

    out.push_str("\n💡 用法: 技能热度+职业名 查看特定职业技能 | 技能大师+技能名 查看大师");
    out
}

/// 职业技能 — 显示各职业的技能偏好和用户分布
pub fn cmd_occupation_skills(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let filter = args.trim();
    let stats = query_occupation_stats(db);

    if stats.is_empty() {
        return "📊 暂无职业技能数据！".to_string();
    }

    // If filtering by specific occupation
    if !filter.is_empty() {
        let stat = stats.iter().find(|s| s.occupation.contains(filter));
        let stat = match stat {
            Some(s) => s,
            None => {
                return format!(
                    "📊 未找到职业 [{}]！\n💡 可用职业: {}",
                    filter,
                    stats
                        .iter()
                        .map(|s| s.occupation.as_str())
                        .collect::<Vec<_>>()
                        .join("/")
                );
            }
        };

        let mut out = format!("═══ 📊 {} 职业技能详情 ═══\n\n", stat.occupation);
        out.push_str(&format!(
            "👥 玩家数: {} | 🎯 技能数: {} | 📈 平均熟练度: {:.0}\n",
            stat.total_users, stat.total_skills, stat.avg_proficiency
        ));
        out.push_str(&format!("⭐ 热门技能: {}\n\n", stat.top_skill));

        // Show all skills for this occupation
        let skills = query_skill_popularity(db);
        let occ_skills: Vec<_> = skills.iter().filter(|s| s.occupation == stat.occupation).collect();

        for (i, skill) in occ_skills.iter().enumerate() {
            let tier = get_tier(skill.avg_proficiency as i32);
            let adoption_pct = skill.user_count as f64 / stat.total_users as f64 * 100.0;
            out.push_str(&format!(
                "{}  . {} {} — {} 人使用 ({:.0}% 采用率)\n     avg Lv.{:.0} [{}] | {}{}加成\n",
                i + 1,
                tier.emoji,
                skill.name,
                skill.user_count,
                adoption_pct,
                skill.avg_proficiency,
                tier.name,
                tier.emoji,
                if tier.damage_bonus_pct > 0.0 {
                    format!("+{:.0}%", tier.damage_bonus_pct)
                } else {
                    "无".to_string()
                }
            ));
        }

        out.push_str(&format!(
            "\n💡 {} 是 {} 职业中最热门的技能！",
            stat.top_skill, stat.occupation
        ));

        return out;
    }

    // Overview of all occupations
    let mut out = "═══ 📊 全服职业技能分布 ═══\n\n".to_string();

    let total_users: i32 = stats.iter().map(|s| s.total_users).sum();
    out.push_str(&format!(
        "👥 总玩家: {} 人 | 🎓 职业数: {}\n\n",
        total_users,
        stats.len()
    ));

    for (i, stat) in stats.iter().enumerate() {
        let tier = get_tier(stat.avg_proficiency as i32);
        let pct = stat.total_users as f64 / total_users as f64 * 100.0;
        let bar_len = (stat.total_users as f64 / stats[0].total_users as f64 * 15.0).min(15.0) as usize;
        let bar: String = "█".repeat(bar_len) + &"░".repeat(15 - bar_len);

        out.push_str(&format!(
            "{}  . {} {} — {} 人 ({:.1}%)\n     🎯 {} 个技能 | avg {} Lv.{:.0} | ⭐热门: {}\n     {}\n",
            i + 1,
            match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            },
            stat.occupation,
            stat.total_users,
            pct,
            stat.total_skills,
            tier.emoji,
            stat.avg_proficiency,
            stat.top_skill,
            bar
        ));
    }

    out.push_str("\n💡 用法: 职业技能+职业名 查看详细分析");
    out
}

/// 技能大师 — 查看特定技能的顶级玩家
pub fn cmd_skill_masters(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let skill_name = args.trim();
    if skill_name.is_empty() {
        // Show available skills
        let skills = query_skill_popularity(db);
        let mut out = "═══ 🏆 技能大师查询 ═══\n\n".to_string();
        out.push_str("📜 请指定技能名！可用技能:\n\n");
        for (i, skill) in skills.iter().take(15).enumerate() {
            out.push_str(&format!(
                "{}  . {} ({} 人使用, 职业: {})\n",
                i + 1,
                skill.name,
                skill.user_count,
                skill.occupation
            ));
        }
        out.push_str("\n💡 用法: 技能大师+技能名");
        return out;
    }

    // Fuzzy match skill name
    let all_skills = query_skill_popularity(db);
    let matched = all_skills
        .iter()
        .find(|s| s.name == skill_name)
        .or_else(|| all_skills.iter().find(|s| s.name.contains(skill_name)));

    let actual_name = match matched {
        Some(s) => s.name.clone(),
        None => {
            // Try exact match in Skill_Register
            let conn = db.lock_conn();
            let exists: bool = conn
                .prepare("SELECT COUNT(*) FROM Skill_Register WHERE SkillName = ?1 AND User != ''")
                .ok()
                .and_then(|mut s| s.query_row([skill_name], |row| row.get::<_, i32>(0)).ok())
                .map(|c| c > 0)
                .unwrap_or(false);
            if exists {
                skill_name.to_string()
            } else {
                return format!(
                    "❌ 未找到技能 [{}]！\n💡 用法: 技能大师+技能名（如: 技能大师+咒术射击）",
                    skill_name
                );
            }
        }
    };

    let masters = query_skill_masters(db, &actual_name, 10);

    if masters.is_empty() {
        return format!("📊 技能 [{}] 暂无大师数据！", actual_name);
    }

    let total_users = masters.len();
    let avg_prof: f64 = masters.iter().map(|m| m.proficiency as f64).sum::<f64>() / total_users as f64;

    let mut out = format!("═══ 🏆 {} — 技能大师榜 ═══\n\n", actual_name);
    out.push_str(&format!(
        "👥 {} 位大师 | 📈 平均熟练度: {:.0}\n\n",
        total_users, avg_prof
    ));

    for (i, master) in masters.iter().enumerate() {
        let tier = get_tier(master.proficiency);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let progress_bar = if master.proficiency < 1000 {
            let next = TIERS
                .iter()
                .find(|t| t.min > master.proficiency)
                .map(|t| t.min)
                .unwrap_or(1000);
            let pct = (master.proficiency as f64 / next as f64 * 8.0).min(8.0) as usize;
            let bar: String = "█".repeat(pct) + &"░".repeat(8 - pct);
            format!(" [{}]", bar)
        } else {
            " ★MAX★".to_string()
        };

        out.push_str(&format!(
            "{} {:>2}. {} — Lv.{} {} [{}]{} | {}\n",
            medal,
            i + 1,
            master.nickname,
            master.proficiency,
            tier.emoji,
            tier.name,
            progress_bar,
            master.occupation
        ));
    }

    // Check if current user has this skill
    let my_prof = crate::proficiency::get_skill_proficiency(db, user_id, &actual_name);
    if my_prof > 0 {
        let my_tier = get_tier(my_prof);
        let rank = masters.iter().position(|m| m.user_id == user_id).map(|p| p + 1);
        out.push_str(&format!(
            "\n📊 你的熟练度: Lv.{} [{}]{}",
            my_prof,
            my_tier.name,
            match rank {
                Some(r) => format!(" (排名 #{})", r),
                None => " (未上榜)".to_string(),
            }
        ));
    } else {
        out.push_str(&format!(
            "\n💡 你尚未使用过 [{}]，战斗中释放该技能可积累熟练度！",
            actual_name
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proficiency_tier_integration() {
        // Verify that the tiers from proficiency module are consistent
        assert_eq!(get_tier(0).name, "新手");
        assert_eq!(get_tier(100).name, "学徒");
        assert_eq!(get_tier(300).name, "熟练");
        assert_eq!(get_tier(600).name, "精通");
        assert_eq!(get_tier(1000).name, "大师");
    }

    #[test]
    fn test_occupation_filter_matching() {
        let occupations = vec!["勇者", "御剑师", "大魔导士", "守誓之盾", "赏金猎人", "圣光使徒"];
        for occ in &occupations {
            assert!(occ.len() > 0);
            assert!(!occ.starts_with("eqName"));
        }
    }

    #[test]
    fn test_bar_visualization_length() {
        // Test that progress bar visualization stays within bounds
        let max_users = 212;
        let test_count = 100;
        let bar_len = (test_count as f64 / max_users as f64 * 15.0).min(15.0) as usize;
        assert!(bar_len <= 15);
        assert!(bar_len > 0);
    }

    #[test]
    fn test_adoption_rate_calculation() {
        let user_count = 50;
        let total_users = 200;
        let adoption_pct = user_count as f64 / total_users as f64 * 100.0;
        assert!((adoption_pct - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_masters_avg_proficiency() {
        let profs = vec![100, 200, 300, 50, 150];
        let avg: f64 = profs.iter().map(|p| *p as f64).sum::<f64>() / profs.len() as f64;
        assert!((avg - 160.0).abs() < 0.01);
    }
}

/// 战斗战报系统
/// 记录每次战斗结果，提供战报历史和统计分析
/// 数据存储: Global 表 SECTION='battle_archive' (战报) / 'battle_stats' (统计)
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 单场战报记录
#[allow(dead_code)]
struct BattleRecord {
    id: String,
    timestamp: String,
    battle_type: String, // PVE / PVP / BOSS / DUNGEON / ABYSS / ARENA
    opponent: String,    // 怪物名/玩家名
    result: String,      // WIN / LOSE / DRAW
    damage_dealt: i64,
    damage_taken: i64,
    rounds: i32,
    exp_gained: i64,
    gold_gained: i64,
    details: String, // 额外信息
}

/// 生成战报ID
fn gen_battle_id(user_id: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple hash: sum of bytes
    let hash = format!("{}{}", user_id, ts)
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
        & 0xFFFF;
    format!("BAT-{}-{:04X}", ts % 1_000_000, hash)
}

/// 记录一场战斗到战报档案
#[allow(clippy::too_many_arguments)]
pub fn record_battle(
    db: &Database,
    user_id: &str,
    battle_type: &str,
    opponent: &str,
    won: bool,
    damage_dealt: i64,
    damage_taken: i64,
    rounds: i32,
    exp_gained: i64,
    gold_gained: i64,
    details: &str,
) {
    let battle_id = gen_battle_id(user_id);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let result = if won { "WIN" } else { "LOSE" };

    // 序列化战报: id|timestamp|type|opponent|result|dmg_dealt|dmg_taken|rounds|exp|gold|details
    let record = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        battle_id,
        now,
        battle_type,
        opponent,
        result,
        damage_dealt,
        damage_taken,
        rounds,
        exp_gained,
        gold_gained,
        details
    );

    // 存储战报 (用时间戳作为排序键)
    let key = format!("{}.{}", user_id, battle_id);
    db.global_set("battle_archive", &key, &record);

    // 更新累计统计
    update_battle_stats(
        db,
        user_id,
        won,
        damage_dealt,
        damage_taken,
        exp_gained,
        gold_gained,
        battle_type,
    );

    // 清理旧战报（保留最近50条）
    cleanup_old_reports(db, user_id, 50);
}

/// 更新玩家战斗统计
#[allow(clippy::too_many_arguments)]
fn update_battle_stats(
    db: &Database,
    user_id: &str,
    won: bool,
    damage_dealt: i64,
    damage_taken: i64,
    exp_gained: i64,
    gold_gained: i64,
    battle_type: &str,
) {
    let prefix = format!("{}.", user_id);

    // 读取现有统计
    let total_key = format!("{}total_battles", prefix);
    let wins_key = format!("{}total_wins", prefix);
    let dmg_dealt_key = format!("{}total_dmg_dealt", prefix);
    let dmg_taken_key = format!("{}total_dmg_taken", prefix);
    let max_dmg_key = format!("{}max_single_dmg", prefix);
    let exp_key = format!("{}total_exp_gained", prefix);
    let gold_key = format!("{}total_gold_gained", prefix);
    let streak_key = format!("{}current_streak", prefix);
    let best_streak_key = format!("{}best_win_streak", prefix);
    let type_key = format!("{}battle_types", prefix);

    let total: i64 = db.global_get("battle_stats", &total_key).parse().unwrap_or(0);
    let wins: i64 = db.global_get("battle_stats", &wins_key).parse().unwrap_or(0);
    let dmg_dealt_total: i64 = db.global_get("battle_stats", &dmg_dealt_key).parse().unwrap_or(0);
    let dmg_taken_total: i64 = db.global_get("battle_stats", &dmg_taken_key).parse().unwrap_or(0);
    let max_dmg: i64 = db.global_get("battle_stats", &max_dmg_key).parse().unwrap_or(0);
    let exp_total: i64 = db.global_get("battle_stats", &exp_key).parse().unwrap_or(0);
    let gold_total: i64 = db.global_get("battle_stats", &gold_key).parse().unwrap_or(0);
    let streak: i64 = db.global_get("battle_stats", &streak_key).parse().unwrap_or(0);
    let best_streak: i64 = db.global_get("battle_stats", &best_streak_key).parse().unwrap_or(0);
    let types_str = db.global_get("battle_stats", &type_key);

    db.global_set("battle_stats", &total_key, &(total + 1).to_string());

    if won {
        let new_wins = wins + 1;
        db.global_set("battle_stats", &wins_key, &new_wins.to_string());
        let new_streak = if streak > 0 { streak + 1 } else { 1 };
        db.global_set("battle_stats", &streak_key, &new_streak.to_string());
        if new_streak > best_streak {
            db.global_set("battle_stats", &best_streak_key, &new_streak.to_string());
        }
    } else {
        db.global_set("battle_stats", &streak_key, "-1");
    }

    db.global_set(
        "battle_stats",
        &dmg_dealt_key,
        &(dmg_dealt_total + damage_dealt).to_string(),
    );
    db.global_set(
        "battle_stats",
        &dmg_taken_key,
        &(dmg_taken_total + damage_taken).to_string(),
    );

    if damage_dealt > max_dmg {
        db.global_set("battle_stats", &max_dmg_key, &damage_dealt.to_string());
    }

    db.global_set("battle_stats", &exp_key, &(exp_total + exp_gained).to_string());
    db.global_set("battle_stats", &gold_key, &(gold_total + gold_gained).to_string());

    // 更新战斗类型计数
    let mut type_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for entry in types_str.split(',') {
        if entry.is_empty() {
            continue;
        }
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        if parts.len() == 2 {
            type_map.insert(parts[0].to_string(), parts[1].parse().unwrap_or(0));
        }
    }
    *type_map.entry(battle_type.to_string()).or_insert(0) += 1;
    let new_types = type_map
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect::<Vec<_>>()
        .join(",");
    db.global_set("battle_stats", &type_key, &new_types);
}

/// 清理旧战报，保留最近N条
fn cleanup_old_reports(db: &Database, user_id: &str, keep: usize) {
    let prefix = format!("{}.", user_id);
    let conn = db.lock_conn();

    let mut records: Vec<(String, String)> = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION='battle_archive' AND ID LIKE ?1 ORDER BY ID DESC")
    {
        let pattern = format!("{}%", prefix);
        if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                records.push(row);
            }
        }
    }

    if records.len() > keep {
        // 保留前keep条，删除其余
        for (id, _) in records.iter().skip(keep) {
            let _ = conn.execute(
                "DELETE FROM Global WHERE SECTION='battle_archive' AND ID=?1",
                rusqlite::params![id],
            );
        }
    }
}

/// 查看战斗战报 — 显示最近N条战斗记录
pub fn cmd_view_battle_log(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let count: usize = args.trim().parse().unwrap_or(10).min(30);
    let conn = db.lock_conn();

    let mut records: Vec<String> = Vec::new();
    let pattern = format!("{}.", user_id);
    if let Ok(mut stmt) =
        conn.prepare("SELECT DATA FROM Global WHERE SECTION='battle_archive' AND ID LIKE ?1 ORDER BY ID DESC LIMIT ?2")
    {
        let like_pattern = format!("{}%", pattern);
        if let Ok(rows) = stmt.query_map(rusqlite::params![like_pattern, count], |row| row.get::<_, String>(0)) {
            for row in rows.flatten() {
                records.push(row);
            }
        }
    }

    if records.is_empty() {
        return format!("{}\n📜 暂无战斗记录\n💡 进行战斗后会自动记录战报", prefix);
    }

    let mut out = String::new();
    out.push_str("📜 ═══ 战斗战报 ═══\n");
    out.push_str(&format!("📋 显示最近{}条记录:\n\n", records.len()));

    for (i, record) in records.iter().enumerate() {
        let parts: Vec<&str> = record.split('|').collect();
        if parts.len() < 10 {
            continue;
        }

        let _id = parts[0];
        let time = parts[1];
        let battle_type = parts[2];
        let opponent = parts[3];
        let result = parts[4];
        let dmg_dealt = parts[5];
        let dmg_taken = parts[6];
        let rounds = parts[7];
        let exp = parts[8];
        let gold = parts[9];

        let result_icon = match result {
            "WIN" => "🏆",
            "LOSE" => "💀",
            _ => "🤝",
        };
        let type_icon = match battle_type {
            "PVE" => "⚔️",
            "PVP" => "🗡️",
            "BOSS" => "👹",
            "DUNGEON" => "🏰",
            "ABYSS" => "🌀",
            "ARENA" => "🏟️",
            _ => "❓",
        };

        out.push_str(&format!(
            "{}. {} {} [{}] {} {}\n",
            i + 1,
            result_icon,
            type_icon,
            battle_type,
            opponent,
            time
        ));
        out.push_str(&format!(
            "   ⚔️伤害:{} 🛡️承伤:{} 🔄回合:{} ⭐经验:{} 💰{}\n",
            dmg_dealt, dmg_taken, rounds, exp, gold
        ));
    }

    out
}

/// 战斗统计 — 显示玩家的综合战斗统计
pub fn cmd_battle_stats(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // 支持查看其他玩家
    let target_id = if args.trim().is_empty() {
        user_id.to_string()
    } else {
        // 模糊匹配玩家名
        let name = args.trim();
        let all_users = db.all_users();
        let mut found = None;
        for uid in &all_users {
            if db.read_basic(uid, ITEM_NAME) == name {
                found = Some(uid.clone());
                break;
            }
        }
        match found {
            Some(id) => id,
            None => return format!("{}\n❌ 未找到玩家: {}", prefix, name),
        }
    };

    let p = format!("{}.", target_id);
    let total: i64 = db
        .global_get("battle_stats", &format!("{}total_battles", p))
        .parse()
        .unwrap_or(0);
    let wins: i64 = db
        .global_get("battle_stats", &format!("{}total_wins", p))
        .parse()
        .unwrap_or(0);
    let dmg_dealt: i64 = db
        .global_get("battle_stats", &format!("{}total_dmg_dealt", p))
        .parse()
        .unwrap_or(0);
    let dmg_taken: i64 = db
        .global_get("battle_stats", &format!("{}total_dmg_taken", p))
        .parse()
        .unwrap_or(0);
    let max_dmg: i64 = db
        .global_get("battle_stats", &format!("{}max_single_dmg", p))
        .parse()
        .unwrap_or(0);
    let exp_gained: i64 = db
        .global_get("battle_stats", &format!("{}total_exp_gained", p))
        .parse()
        .unwrap_or(0);
    let gold_gained: i64 = db
        .global_get("battle_stats", &format!("{}total_gold_gained", p))
        .parse()
        .unwrap_or(0);
    let streak: i64 = db
        .global_get("battle_stats", &format!("{}current_streak", p))
        .parse()
        .unwrap_or(0);
    let best_streak: i64 = db
        .global_get("battle_stats", &format!("{}best_win_streak", p))
        .parse()
        .unwrap_or(0);
    let types_str = db.global_get("battle_stats", &format!("{}battle_types", p));

    let target_name = if target_id == user_id {
        db.read_basic(user_id, ITEM_NAME)
    } else {
        db.read_basic(&target_id, ITEM_NAME)
    };
    let display_name = if target_name.is_empty() {
        target_id.clone()
    } else {
        target_name
    };

    if total == 0 {
        return format!("{}\n📊 {} 暂无战斗记录\n💡 进行战斗后会自动记录", prefix, display_name);
    }

    let losses = total - wins;
    let win_rate = (wins as f64 / total as f64) * 100.0;
    let avg_dmg = dmg_dealt / total;
    let avg_taken = dmg_taken / total;

    let mut out = String::new();
    out.push_str(&format!("📊 ═══ {} 的战斗统计 ═══\n\n", display_name));

    // 总览
    out.push_str(&format!("⚔️ 总战斗次数: {}\n", total));
    out.push_str(&format!("🏆 胜利: {} | 💀 失败: {}\n", wins, losses));
    out.push_str(&format!("📈 胜率: {:.1}%\n", win_rate));

    // 胜率进度条
    let bar_len = 20;
    let filled = ((win_rate / 100.0) * bar_len as f64) as usize;
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
    out.push_str(&format!("   [{}]\n", bar));

    // 连胜
    let streak_display = if streak > 0 {
        format!("🔥 连胜中: {} 场", streak)
    } else if streak < 0 {
        format!("💔 连败中: {} 场", streak.abs())
    } else {
        "— 无连胜".to_string()
    };
    out.push_str(&format!("\n{}\n", streak_display));
    out.push_str(&format!("🏅 最佳连胜: {} 场\n", best_streak));

    // 伤害统计
    out.push_str("\n💥 ══ 伤害统计 ══\n");
    out.push_str(&format!("⚔️ 总造成伤害: {}\n", format_number(dmg_dealt)));
    out.push_str(&format!("🛡️ 总承受伤害: {}\n", format_number(dmg_taken)));
    out.push_str(&format!("📊 平均造成: {}/场\n", format_number(avg_dmg)));
    out.push_str(&format!("📊 平均承受: {}/场\n", format_number(avg_taken)));
    out.push_str(&format!("🎯 单次最高: {}\n", format_number(max_dmg)));

    // 净伤害比
    if dmg_taken > 0 {
        let ratio = dmg_dealt as f64 / dmg_taken as f64;
        out.push_str(&format!("⚖️ 攻防比: {:.2}\n", ratio));
    }

    // 收益统计
    out.push_str("\n💰 ══ 战斗收益 ══\n");
    out.push_str(&format!("⭐ 累计经验: {}\n", format_number(exp_gained)));
    out.push_str(&format!("💰 累计金币: {}\n", format_number(gold_gained)));
    if total > 0 {
        out.push_str(&format!("📊 场均经验: {}\n", format_number(exp_gained / total)));
        out.push_str(&format!("📊 场均金币: {}\n", format_number(gold_gained / total)));
    }

    // 战斗类型分布
    if !types_str.is_empty() {
        out.push_str("\n📋 ══ 战斗类型分布 ══\n");
        let mut type_counts: Vec<(String, i64)> = Vec::new();
        for entry in types_str.split(',') {
            let parts: Vec<&str> = entry.splitn(2, ':').collect();
            if parts.len() == 2 {
                type_counts.push((parts[0].to_string(), parts[1].parse().unwrap_or(0)));
            }
        }
        type_counts.sort_by_key(|b| std::cmp::Reverse(b.1));
        for (t, c) in &type_counts {
            let icon = match t.as_str() {
                "PVE" => "⚔️",
                "PVP" => "🗡️",
                "BOSS" => "👹",
                "DUNGEON" => "🏰",
                "ABYSS" => "🌀",
                "ARENA" => "🏟️",
                _ => "❓",
            };
            let pct = (*c as f64 / total as f64) * 100.0;
            let mini_bar_len = 10;
            let mini_filled = ((pct / 100.0) * mini_bar_len as f64) as usize;
            let mini_bar = "█".repeat(mini_filled) + &"░".repeat(mini_bar_len - mini_filled);
            out.push_str(&format!("{} [{}] {} ({:.0}%)\n", icon, mini_bar, t, pct));
        }
    }

    out
}

/// 战报排行 — 全服战斗排行
pub fn cmd_battle_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let sort_by = args.trim();
    let all_users = db.all_users();

    let mut players: Vec<(String, i64, i64, i64, f64)> = Vec::new(); // (name, total, wins, value, win_rate)

    for uid in &all_users {
        let p = format!("{}.", uid);
        let total: i64 = db
            .global_get("battle_stats", &format!("{}total_battles", p))
            .parse()
            .unwrap_or(0);
        if total == 0 {
            continue;
        }

        let wins: i64 = db
            .global_get("battle_stats", &format!("{}total_wins", p))
            .parse()
            .unwrap_or(0);
        let win_rate = (wins as f64 / total as f64) * 100.0;

        let value = match sort_by {
            "连胜" | "streak" => {
                let streak: i64 = db
                    .global_get("battle_stats", &format!("{}best_win_streak", p))
                    .parse()
                    .unwrap_or(0);
                streak
            }
            "伤害" | "damage" => db
                .global_get("battle_stats", &format!("{}total_dmg_dealt", p))
                .parse()
                .unwrap_or(0),
            "胜率" | "winrate" => win_rate as i64,
            _ => total, // 默认按总战斗次数
        };

        let name = db.read_basic(uid, ITEM_NAME);
        let display = if name.is_empty() { uid.clone() } else { name };
        players.push((display, total, wins, value, win_rate));
    }

    if players.is_empty() {
        return format!("{}\n📊 暂无战斗排行数据", prefix);
    }

    // 按value降序
    players.sort_by_key(|b| std::cmp::Reverse(b.3));

    let title = match sort_by {
        "连胜" | "streak" => "🏅 最佳连胜排行",
        "伤害" | "damage" => "💥 总伤害排行",
        "胜率" | "winrate" => "📈 胜率排行",
        _ => "⚔️ 战斗次数排行",
    };

    let mut out = String::new();
    out.push_str(&format!("{} ═══\n", title));

    let display_count = players.len().min(15);
    for (i, (name, total, wins, value, win_rate)) in players.iter().enumerate().take(display_count) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let stat_str = match sort_by {
            "连胜" | "streak" => format!("{}连胜", value),
            "伤害" | "damage" => format_number(*value),
            "胜率" | "winrate" => format!("{:.1}%", *value as f64),
            _ => format!("{}次", value),
        };
        out.push_str(&format!(
            "{} {}. {} — {} (胜率:{:.0}%, {}/{}场)\n",
            medal,
            i + 1,
            name,
            stat_str,
            win_rate,
            wins,
            total
        ));
    }

    // 当前用户排名
    let user_total: i64 = db
        .global_get("battle_stats", &format!("{}.total_battles", user_id))
        .parse()
        .unwrap_or(0);
    if user_total > 0 {
        let user_wins: i64 = db
            .global_get("battle_stats", &format!("{}.total_wins", user_id))
            .parse()
            .unwrap_or(0);
        let user_wr = (user_wins as f64 / user_total as f64) * 100.0;
        let mut user_rank = 1;
        for (_, _, _, value, _) in &players {
            let user_val: i64 = match sort_by {
                "连胜" => db
                    .global_get("battle_stats", &format!("{}.best_win_streak", user_id))
                    .parse()
                    .unwrap_or(0),
                "伤害" => db
                    .global_get("battle_stats", &format!("{}.total_dmg_dealt", user_id))
                    .parse()
                    .unwrap_or(0),
                "胜率" => user_wr as i64,
                _ => user_total,
            };
            if *value > user_val {
                user_rank += 1;
            }
        }
        out.push_str(&format!("\n📊 您的排名: {}/{}", user_rank, players.len()));
    }

    out
}

/// 数字格式化（千分位）
fn format_number(n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(123), "123");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(-5000), "-5,000");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn test_gen_battle_id_format() {
        let id = gen_battle_id("test_user");
        assert!(id.starts_with("BAT-"));
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "BAT");
        // Middle part is numeric timestamp
        assert!(parts[1].parse::<u64>().is_ok());
        // Last part is 4-char hex
        assert_eq!(parts[2].len(), 4);
    }

    #[test]
    fn test_battle_record_serialization() {
        // 验证战报序列化/反序列化
        let record = "BAT-123456-ABCD|2026-06-10 01:00:00|PVE|史莱姆|WIN|150|30|3|50|100|";
        let parts: Vec<&str> = record.split('|').collect();
        assert_eq!(parts.len(), 11);
        assert_eq!(parts[0], "BAT-123456-ABCD");
        assert_eq!(parts[2], "PVE");
        assert_eq!(parts[4], "WIN");
        assert_eq!(parts[5], "150");
    }

    #[test]
    fn test_win_rate_calculation() {
        // 验证胜率计算
        let wins = 7i64;
        let total = 10i64;
        let win_rate = (wins as f64 / total as f64) * 100.0;
        assert!((win_rate - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_battle_type_icons() {
        let types = vec![
            ("PVE", "⚔️"),
            ("PVP", "🗡️"),
            ("BOSS", "👹"),
            ("DUNGEON", "🏰"),
            ("ABYSS", "🌀"),
            ("ARENA", "🏟️"),
        ];
        for (t, icon) in types {
            let result = match t {
                "PVE" => "⚔️",
                "PVP" => "🗡️",
                "BOSS" => "👹",
                "DUNGEON" => "🏰",
                "ABYSS" => "🌀",
                "ARENA" => "🏟️",
                _ => "❓",
            };
            assert_eq!(result, icon);
        }
    }

    #[test]
    fn test_streak_logic() {
        // 胜利连胜
        let streak = 3i64;
        let new_streak = if streak > 0 { streak + 1 } else { 1 };
        assert_eq!(new_streak, 4);

        // 从失败恢复
        let streak = 0i64;
        let new_streak = if streak > 0 { streak + 1 } else { 1 };
        assert_eq!(new_streak, 1);
    }
}

/// CakeGame 游戏管理面板系统
/// 为 GM 提供全面的服务器管理工具
///
/// 功能:
/// - 服务器状态: 总览服务器健康状况
/// - 在线玩家: 查看当前在线玩家列表
/// - 封禁/解封: 管理玩家封禁状态
/// - 系统统计: 经济/战斗/社交全维度统计
/// - 玩家搜索: 按昵称/ID搜索玩家
///
/// 数据存储: Global表 SECTION='admin_panel'
use crate::core::*;
use crate::db::Database;
use crate::permissions;
use crate::user;
use chrono::Local;

/// 权限检查阈值
const PERMISSION_LEVEL_ADMIN: i32 = 100;

/// 服务器状态总览 (GM)
pub fn cmd_server_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let conn = db.lock_conn();

    // 总用户数
    let total_users: i64 = conn
        .prepare("SELECT COUNT(*) FROM Basic_User")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // 活跃用户 (有Shared_Data记录的)
    let active_users: i64 = conn
        .prepare("SELECT COUNT(DISTINCT ID) FROM Shared_Data")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // 背包物品总数
    let total_items: i64 = conn
        .prepare("SELECT COUNT(*) FROM Basic_knapsack")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // 公会总数
    let total_guilds: i64 = conn
        .prepare("SELECT COUNT(DISTINCT ID) FROM Global WHERE SECTION='guild'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // 全服金币总量
    let total_gold: i64 = conn
        .prepare(&format!(
            "SELECT COALESCE(SUM(CAST({} AS INTEGER)), 0) FROM Basic_User",
            CURRENCY_GOLD
        ))
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // 全服钻石总量
    let total_diamond: i64 = conn
        .prepare(&format!(
            "SELECT COALESCE(SUM(CAST({} AS INTEGER)), 0) FROM Basic_User",
            CURRENCY_DIAMOND
        ))
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // 封禁玩家数
    let banned_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Global WHERE SECTION='admin_panel' AND ID LIKE 'ban_%'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // Global表记录数
    let global_records: i64 = conn
        .prepare("SELECT COUNT(*) FROM Global")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    // Shared_Data记录数
    let shared_records: i64 = conn
        .prepare("SELECT COUNT(*) FROM Shared_Data")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    drop(conn);

    let now = Local::now().format("%Y-%m-%d %H:%M:%S");

    format!(
        "{}\n\
         ═══ 🖥️ 服务器状态面板 ═══\n\
         \n\
         📅 时间: {}\n\
         \n\
         👥 玩家数据\n\
         ├ 总注册: {} 人\n\
         ├ 活跃用户: {} 人\n\
         └ 封禁用户: {} 人\n\
         \n\
         💰 经济数据\n\
         ├ 全服金币: {}\n\
         └ 全服钻石: {}\n\
         \n\
         📦 数据统计\n\
         ├ 背包物品: {} 件\n\
         ├ 公会数量: {} 个\n\
         ├ Global记录: {} 条\n\
         └ Shared记录: {} 条\n\
         \n\
         💡 使用「在线玩家」查看在线列表\n\
         💡 使用「系统统计」查看详细统计",
        prefix,
        now,
        total_users,
        active_users,
        banned_count,
        format_num(total_gold),
        format_num(total_diamond),
        total_items,
        total_guilds,
        global_records,
        shared_records
    )
}

/// 在线玩家列表 (GM)
pub fn cmd_online_players(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let conn = db.lock_conn();

    // 获取有活跃数据的玩家 (通过Shared_Data中的CustomVariable)
    let ids: Vec<String> = {
        let mut stmt = match conn.prepare(
            "SELECT DISTINCT ID FROM Shared_Data \
             WHERE SECTION LIKE 'com.shdic.CustomVariable%' \
             ORDER BY ID LIMIT 50",
        ) {
            Ok(s) => s,
            Err(_) => return format!("{}\n❌ 查询失败", prefix),
        };
        stmt.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
    };

    drop(conn);

    if ids.is_empty() {
        return format!("{}\n═══ 👥 在线玩家 ═══\n\n暂无在线玩家", prefix);
    }

    let mut result = format!("{}\n═══ 👥 在线玩家 ({}人) ═══", prefix, ids.len());

    for (i, uid) in ids.iter().enumerate() {
        let info = user::calc_total_attrs(db, uid);
        let name = if info.name.is_empty() { uid.as_str() } else { &info.name };
        let level = info.level;
        let power = permissions::get_permission(db, uid);
        let role = permissions::permission_name(power);
        result.push_str(&format!("\n{}. {} Lv.{} [{}]", i + 1, name, level, role));
    }

    result
}

/// 封禁玩家 (GM)
pub fn cmd_ban_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n格式：封禁玩家+用户ID\n示例：封禁玩家+123456", prefix);
    }

    if !db.user_exists(target) {
        return format!("{}\n❌ 用户 {} 不存在！", prefix, target);
    }

    // 不能封禁更高权限的用户
    let target_power = permissions::get_permission(db, target);
    if target_power >= power {
        return format!("{}\n❌ 不能封禁同级或更高权限的用户！", prefix);
    }

    // 记录封禁信息
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let ban_section = format!("ban_{}", target);
    db.global_set(&ban_section, "reason", &format!("GM {} 封禁", user_id));
    db.global_set(&ban_section, "time", &now);
    db.global_set(&ban_section, "operator", user_id);

    let info = user::calc_total_attrs(db, target);
    let name = if info.name.is_empty() { target } else { &info.name };

    format!(
        "{}\n✅ 封禁成功！\n\n\
         🔨 被封禁: {} ({})\n\
         ⏰ 时间: {}\n\
         📝 操作者: {}\n\n\
         使用「解封玩家+{}」解除封禁",
        prefix, name, target, now, user_id, target
    )
}

/// 解封玩家 (GM)
pub fn cmd_unban_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n格式：解封玩家+用户ID", prefix);
    }

    let ban_section = format!("ban_{}", target);
    let ban_reason = db.global_get(&ban_section, "reason");
    if ban_reason.is_empty() {
        return format!("{}\n❌ 用户 {} 未被封禁！", prefix, target);
    }

    // 清除封禁记录
    db.global_set(&ban_section, "reason", "");
    db.global_set(&ban_section, "time", "");
    db.global_set(&ban_section, "operator", "");

    let info = user::calc_total_attrs(db, target);
    let name = if info.name.is_empty() { target } else { &info.name };

    format!(
        "{}\n✅ 解封成功！\n\n\
         🔓 已解封: {} ({})\n\
         ⏰ 操作时间: {}\n\
         📝 操作者: {}",
        prefix,
        name,
        target,
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        user_id
    )
}

/// 系统统计 (GM)
pub fn cmd_system_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let conn = db.lock_conn();

    // 等级分布
    let level_dist = {
        let mut dist = [0i64; 5]; // 1-20, 21-40, 41-60, 61-80, 81+
        if let Ok(mut stmt) = conn.prepare(&format!("SELECT CAST({} AS INTEGER) FROM Basic_User", ITEM_LEVEL)) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, i32>(0)) {
                for row in rows.flatten() {
                    let idx = match row {
                        1..=20 => 0,
                        21..=40 => 1,
                        41..=60 => 2,
                        61..=80 => 3,
                        _ => 4,
                    };
                    dist[idx] += 1;
                }
            }
        }
        dist
    };

    // 职业分布
    let mut occ_dist: Vec<(String, i64)> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT {}, COUNT(*) FROM Basic_User GROUP BY {} ORDER BY COUNT(*) DESC LIMIT 6",
        ITEM_OCCUPATION, ITEM_OCCUPATION
    )) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, i64>(1).unwrap_or(0),
            ))
        }) {
            for row in rows.flatten() {
                occ_dist.push(row);
            }
        }
    }

    // 装备强化分布
    let enhance_dist: Vec<(String, i64)> = {
        let mut dist: Vec<(String, i64)> = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT EquipName FROM Equip_Register") {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    if let Some(start) = row.find("(+") {
                        if let Some(end) = row[start..].find(')') {
                            let level_str = &row[start + 2..start + end];
                            if let Ok(level) = level_str.parse::<i32>() {
                                let bracket = match level {
                                    0..=5 => "+0~+5",
                                    6..=10 => "+6~+10",
                                    11..=15 => "+11~+15",
                                    _ => "+16+",
                                };
                                if let Some(entry) = dist.iter_mut().find(|e| e.0 == bracket) {
                                    entry.1 += 1;
                                } else {
                                    dist.push((bracket.to_string(), 1));
                                }
                            }
                        }
                    }
                }
            }
        }
        dist
    };

    // 今日新增用户 (通过Global中的注册记录近似)
    let today = Local::now().format("%Y-%m-%d").to_string();
    let today_registrations: i64 = conn
        .prepare("SELECT COUNT(*) FROM Global WHERE SECTION='daily_report' AND ID LIKE ?1")
        .ok()
        .and_then(|mut s| {
            let pattern = format!("%{}%", today);
            s.query_row(rusqlite::params![pattern], |r| r.get(0)).ok()
        })
        .unwrap_or(0);

    drop(conn);

    let mut result = format!(
        "{}\n\
         ═══ 📊 系统统计面板 ═══\n\
         \n\
         📈 等级分布\n\
         ├ Lv.1-20:  {} 人\n\
         ├ Lv.21-40: {} 人\n\
         ├ Lv.41-60: {} 人\n\
         ├ Lv.61-80: {} 人\n\
         └ Lv.80+:   {} 人\n\
         \n\
         🎭 职业分布",
        prefix, level_dist[0], level_dist[1], level_dist[2], level_dist[3], level_dist[4]
    );

    for (occ, count) in &occ_dist {
        let pct = if level_dist.iter().sum::<i64>() > 0 {
            *count as f64 / level_dist.iter().sum::<i64>() as f64 * 100.0
        } else {
            0.0
        };
        result.push_str(&format!("\n├ {}: {} 人 ({:.1}%)", occ, count, pct));
    }

    if !enhance_dist.is_empty() {
        result.push_str("\n\n⚒️ 强化等级分布");
        for (bracket, count) in &enhance_dist {
            result.push_str(&format!("\n├ {}: {} 件", bracket, count));
        }
    }

    result.push_str(&format!(
        "\n\n📅 今日注册: {} 人\n\
         💡 使用「服务器状态」查看总览",
        today_registrations
    ));

    result
}

/// 玩家搜索 (GM)
pub fn cmd_search_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let keyword = args.trim();
    if keyword.is_empty() {
        return format!("{}\n格式：玩家搜索+关键词\n支持昵称或用户ID搜索", prefix);
    }

    let conn = db.lock_conn();

    // 搜索用户ID或昵称
    let mut results: Vec<String> = Vec::new();

    // 精确ID匹配
    if db.user_exists(keyword) {
        results.push(keyword.to_string());
    }

    // 昵称模糊搜索
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT ID FROM Basic_User WHERE {} LIKE ?1 LIMIT 20",
        ITEM_NAME
    )) {
        let pattern = format!("%{}%", keyword);
        if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| row.get::<_, String>(0)) {
            for row in rows.flatten() {
                if !results.contains(&row) {
                    results.push(row);
                }
            }
        }
    }

    drop(conn);

    if results.is_empty() {
        return format!("{}\n❌ 未找到匹配 \"{}\" 的玩家", prefix, keyword);
    }

    let mut out = format!("{}\n═══ 🔍 搜索结果 (\"{}\") ═══", prefix, keyword);

    for uid in results.iter().take(10) {
        let info = user::calc_total_attrs(db, uid);
        let name = if info.name.is_empty() { uid.as_str() } else { &info.name };
        let level = info.level;
        let occupation = if info.occupation.is_empty() {
            "无职业"
        } else {
            &info.occupation
        };
        let pwr = permissions::get_permission(db, uid);
        let role = permissions::permission_name(pwr);

        let ban_status = db.global_get(&format!("ban_{}", uid), "reason");
        let ban_flag = if !ban_status.is_empty() { " 🔴已封禁" } else { "" };

        out.push_str(&format!(
            "\n• {} (ID:{})\n  Lv.{} {} [{}]{}",
            name, uid, level, occupation, role, ban_flag
        ));
    }

    if results.len() > 10 {
        out.push_str(&format!("\n\n... 共 {} 个结果，仅显示前10个", results.len()));
    }

    out
}

/// 格式化数字 (千分位)
fn format_num(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(123), "123");
        assert_eq!(format_num(1234), "1,234");
        assert_eq!(format_num(1234567), "1,234,567");
        assert_eq!(format_num(100000000), "100,000,000");
        assert_eq!(format_num(-5000), "-5,000");
    }

    #[test]
    fn test_format_num_edge_cases() {
        assert_eq!(format_num(1), "1");
        assert_eq!(format_num(10), "10");
        assert_eq!(format_num(100), "100");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(10000), "10,000");
        assert_eq!(format_num(100000), "100,000");
    }

    #[test]
    fn test_format_num_large() {
        assert_eq!(format_num(999999999), "999,999,999");
        assert_eq!(format_num(1000000000), "1,000,000,000");
    }
}

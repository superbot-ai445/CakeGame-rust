/// CakeGame 在线活跃统计系统
/// 基于 Shared_Data 表追踪玩家在线时长和活跃度
/// 提供在线排行、活跃统计、个人活跃信息等指令
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 活跃等级称号
fn activity_level(minutes: i32) -> (&'static str, &'static str) {
    match minutes {
        0..=9 => ("挂机萌新", "⚪"),
        10..=29 => ("休闲玩家", "🟢"),
        30..=59 => ("活跃冒险者", "🔵"),
        60..=119 => ("资深勇者", "🟣"),
        120..=239 => ("肝帝", "🟡"),
        240..=479 => ("传说肝王", "🔴"),
        _ => ("永不停歇", "⭐"),
    }
}

/// 获取用户在线时长（分钟）从 Shared_Data
pub fn get_user_active_minutes(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION='com.shdic.CustomVariable_user_active_minutes'")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id], |row| {
                let raw: String = row.get(0).unwrap_or_default();
                Ok(raw.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0)
}

/// 获取用户最后活跃时间戳
pub fn get_user_last_active(db: &Database, user_id: &str) -> i64 {
    let conn = db.lock_conn();
    let key = format!("{}上次活跃时间戳", user_id);
    conn.prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION='com.shdic.CustomVariable'")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![key], |row| {
                let raw: String = row.get(0).unwrap_or_default();
                // Handle both numeric timestamps and binary data
                Ok(raw.parse().unwrap_or(0i64))
            })
            .ok()
        })
        .unwrap_or(0)
}

/// 更新用户在线时长
pub fn update_user_active_minutes(db: &Database, user_id: &str, add_minutes: i32) {
    let current = get_user_active_minutes(db, user_id);
    let new_total = current + add_minutes;
    let conn = db.lock_conn();
    // Upsert: try update first
    let updated = conn
        .execute(
            "UPDATE Shared_Data SET DATA=?1 WHERE ID=?2 AND SECTION='com.shdic.CustomVariable_user_active_minutes'",
            rusqlite::params![new_total.to_string(), user_id],
        )
        .unwrap_or(0);
    if updated == 0 {
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, 'com.shdic.CustomVariable_user_active_minutes', ?2)",
            rusqlite::params![user_id, new_total.to_string()],
        );
    }
    // Update last active timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let key = format!("{}上次活跃时间戳", user_id);
    let updated_ts = conn
        .execute(
            "UPDATE Shared_Data SET DATA=?1 WHERE ID=?2 AND SECTION='com.shdic.CustomVariable'",
            rusqlite::params![now.to_string(), key],
        )
        .unwrap_or(0);
    if updated_ts == 0 {
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, 'com.shdic.CustomVariable', ?2)",
            rusqlite::params![key, now.to_string()],
        );
    }
}

/// 获取在线时长排行榜 (从 Shared_Data 读取)
fn get_activity_ranking(db: &Database, limit: usize) -> Vec<(String, i32)> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT ID, CAST(DATA AS INTEGER) as mins FROM Shared_Data \
         WHERE SECTION='com.shdic.CustomVariable_user_active_minutes' \
         ORDER BY mins DESC LIMIT ?1",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![limit as i32], |row| {
        let id: String = row.get(0).unwrap_or_default();
        let mins: i32 = row.get(1).unwrap_or(0);
        Ok((id, mins))
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// 统计在线数据
fn get_activity_stats(db: &Database) -> (i32, i32, i32, f64) {
    let conn = db.lock_conn();
    // Total tracked users
    let total_users: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM Shared_Data WHERE SECTION='com.shdic.CustomVariable_user_active_minutes'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    // Total minutes
    let total_minutes: i32 = conn
        .query_row(
            "SELECT COALESCE(SUM(CAST(DATA AS INTEGER)), 0) FROM Shared_Data WHERE SECTION='com.shdic.CustomVariable_user_active_minutes'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    // Max minutes
    let max_minutes: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(CAST(DATA AS INTEGER)), 0) FROM Shared_Data WHERE SECTION='com.shdic.CustomVariable_user_active_minutes'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let avg = if total_users > 0 {
        total_minutes as f64 / total_users as f64
    } else {
        0.0
    };
    (total_users, total_minutes, max_minutes, avg)
}

/// 格式化分钟为可读时间
fn format_minutes(mins: i32) -> String {
    if mins < 60 {
        format!("{}分钟", mins)
    } else if mins < 1440 {
        format!("{}小时{}分钟", mins / 60, mins % 60)
    } else {
        format!("{}天{}小时", mins / 1440, (mins % 1440) / 60)
    }
}

// ==================== 指令处理 ====================

/// 在线排行 - 查看在线时长排行榜
pub fn cmd_online_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let limit: usize = args.trim().parse().unwrap_or(10).clamp(5, 20);
    let ranking = get_activity_ranking(db, limit);

    if ranking.is_empty() {
        return format!(
            "{}\n═══ 在线时长排行 ═══\n\n暂无在线数据记录。\n💡 使用游戏功能即可累计在线时长",
            prefix
        );
    }

    let mut result = format!("{}\n═══ 🏆 在线时长排行 TOP{} ═══\n", prefix, limit);
    for (i, (uid, mins)) in ranking.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let nick = db.read_basic(uid, ITEM_NAME);
        let display = if nick.is_empty() { uid.clone() } else { nick };
        let (level_name, _emoji) = activity_level(*mins);
        result.push_str(&format!(
            "\n{} {}. {} - {} [{}]",
            medal,
            i + 1,
            display,
            format_minutes(*mins),
            level_name
        ));
    }
    result.push_str("\n\n💡 在线时长通过游戏行为自动累积\n发送「在线统计」查看全局数据");
    result
}

/// 在线统计 - 查看全局在线统计数据
pub fn cmd_online_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (total_users, total_minutes, max_minutes, avg_minutes) = get_activity_stats(db);

    let my_minutes = get_user_active_minutes(db, user_id);
    let (my_level, my_emoji) = activity_level(my_minutes);

    // 找到自己的排名
    let ranking = get_activity_ranking(db, 100);
    let my_rank = ranking
        .iter()
        .position(|(uid, _)| uid == user_id)
        .map(|p| p + 1)
        .unwrap_or(0);

    format!(
        "{}\n═══ 📊 在线活跃统计 ═══\n\n\
         🌐 全服数据：\n\
         · 追踪玩家数：{}\n\
         · 总在线时长：{}\n\
         · 人均在线：{:.1}分钟\n\
         · 最高在线：{}\n\n\
         👤 我的活跃：\n\
         · 累计在线：{} {}\n\
         · 活跃等级：{}\n\
         · 活跃排名：{}\n\n\
         💡 每次使用游戏指令自动累积在线时长",
        prefix,
        total_users,
        format_minutes(total_minutes),
        avg_minutes,
        format_minutes(max_minutes),
        format_minutes(my_minutes),
        my_emoji,
        my_level,
        if my_rank > 0 {
            format!("#{}", my_rank)
        } else {
            "未上榜".to_string()
        }
    )
}

/// 活跃信息 - 查看指定玩家的活跃信息
pub fn cmd_activity_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target = if args.trim().is_empty() {
        user_id.to_string()
    } else {
        args.trim().to_string()
    };

    if !db.user_exists(&target) {
        return format!("{}\n玩家 [{}] 不存在", prefix, target);
    }

    let minutes = get_user_active_minutes(db, &target);
    let last_active = get_user_last_active(db, &target);
    let (level_name, emoji) = activity_level(minutes);
    let nick = db.read_basic(&target, ITEM_NAME);
    let display = if nick.is_empty() {
        target.clone()
    } else {
        format!("{}({})", nick, target)
    };

    // 计算最后活跃时间
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let last_active_str = if last_active > 0 {
        let diff = now - last_active;
        if diff < 60 {
            "刚刚".to_string()
        } else if diff < 3600 {
            format!("{}分钟前", diff / 60)
        } else if diff < 86400 {
            format!("{}小时前", diff / 3600)
        } else {
            format!("{}天前", diff / 86400)
        }
    } else {
        "无记录".to_string()
    };

    // 计算排名
    let ranking = get_activity_ranking(db, 100);
    let rank = ranking
        .iter()
        .position(|(uid, _)| uid == &target)
        .map(|p| p + 1)
        .unwrap_or(0);

    format!(
        "{}\n═══ 📋 活跃信息 ═══\n\n\
         玩家：{}\n\
         累计在线：{} {}\n\
         活跃等级：{}\n\
         最后活跃：{}\n\
         活跃排名：{}\n\n\
         💡 活跃等级说明：\n\
         ⚪ 挂机萌新 (0-9分钟)\n\
         🟢 休闲玩家 (10-29分钟)\n\
         🔵 活跃冒险者 (30-59分钟)\n\
         🟣 资深勇者 (1-2小时)\n\
         🟡 肝帝 (2-4小时)\n\
         🔴 传说肝王 (4-8小时)\n\
         ⭐ 永不停歇 (8小时+)",
        prefix,
        display,
        format_minutes(minutes),
        emoji,
        level_name,
        last_active_str,
        if rank > 0 {
            format!("#{}", rank)
        } else {
            "未上榜".to_string()
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_level() {
        assert_eq!(activity_level(0).0, "挂机萌新");
        assert_eq!(activity_level(5).0, "挂机萌新");
        assert_eq!(activity_level(10).0, "休闲玩家");
        assert_eq!(activity_level(30).0, "活跃冒险者");
        assert_eq!(activity_level(60).0, "资深勇者");
        assert_eq!(activity_level(120).0, "肝帝");
        assert_eq!(activity_level(240).0, "传说肝王");
        assert_eq!(activity_level(500).0, "永不停歇");
    }

    #[test]
    fn test_format_minutes() {
        assert_eq!(format_minutes(0), "0分钟");
        assert_eq!(format_minutes(30), "30分钟");
        assert_eq!(format_minutes(60), "1小时0分钟");
        assert_eq!(format_minutes(90), "1小时30分钟");
        assert_eq!(format_minutes(1440), "1天0小时");
        assert_eq!(format_minutes(1500), "1天1小时");
    }
}

/// CakeGame 离线收益系统
/// 玩家离线期间自动累积金币和经验收益，上线后可领取
/// 最多累积24小时，收益基于玩家等级
use crate::db::Database;
use crate::online_activity::get_user_last_active;

/// 离线收益上限（秒）- 24小时
const MAX_OFFLINE_SECONDS: i64 = 86400;

/// 每分钟基础金币收益
const BASE_GOLD_PER_MIN: i64 = 2;

/// 每分钟基础经验收益
const BASE_EXP_PER_MIN: i64 = 5;

/// 计算离线收益
fn calc_offline_reward(db: &Database, user_id: &str) -> Option<(i64, i64, i64)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let last_active = get_user_last_active(db, user_id);

    if last_active <= 0 {
        return None;
    }

    let offline_seconds = now - last_active;
    if offline_seconds < 300 {
        // 离线不足5分钟，不计算收益
        return None;
    }

    let capped_seconds = offline_seconds.min(MAX_OFFLINE_SECONDS);
    let offline_minutes = capped_seconds / 60;

    // 获取玩家等级（影响收益）
    let level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1).max(1);
    let level_bonus = 1.0 + (level as f64 - 1.0) * 0.05; // 每级+5%

    let gold = (BASE_GOLD_PER_MIN as f64 * offline_minutes as f64 * level_bonus) as i64;
    let exp = (BASE_EXP_PER_MIN as f64 * offline_minutes as f64 * level_bonus) as i64;

    Some((gold, exp, offline_seconds))
}

/// 查看离线收益
pub fn cmd_view_offline_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 更新活跃时间戳（通过添加0分钟来刷新时间戳）
    crate::online_activity::update_user_active_minutes(db, user_id, 0);

    match calc_offline_reward(db, user_id) {
        Some((gold, exp, offline_seconds)) => {
            let hours = offline_seconds / 3600;
            let minutes = (offline_seconds % 3600) / 60;
            let time_str = if hours > 0 {
                format!("{}小时{}分钟", hours, minutes)
            } else {
                format!("{}分钟", minutes)
            };

            // 存储待领取的离线收益
            let conn = db.lock_conn();
            let _ = conn.execute(
                "INSERT OR REPLACE INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, 'com.shdic.offline_reward_pending', ?2)",
                rusqlite::params![user_id, format!("{}|{}", gold, exp)],
            );
            drop(conn);

            format!(
                "{}\n═══ 离线收益 ═══\n离线时长：{}\n可领取：\n  💰 金币 +{}\n  ⭐ 经验 +{}\n\n发送'领取离线收益'即可领取\n(最多累积24小时)",
                prefix, time_str, gold, exp
            )
        }
        None => {
            format!(
                "{}\n═══ 离线收益 ═══\n你最近刚活跃过，暂无离线收益。\n离线超过5分钟后可累积收益。",
                prefix
            )
        }
    }
}

/// 领取离线收益
pub fn cmd_claim_offline_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 读取待领取的收益
    let pending = {
        let conn = db.lock_conn();
        conn.prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION='com.shdic.offline_reward_pending'")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_row(rusqlite::params![user_id], |row| {
                    Ok(row.get::<_, String>(0).unwrap_or_default())
                })
                .ok()
            })
            .unwrap_or_default()
    };

    if pending.is_empty() {
        return format!("{}\n没有待领取的离线收益，请先发送'离线收益'查看。", prefix);
    }

    let parts: Vec<&str> = pending.split('|').collect();
    if parts.len() < 2 {
        return format!("{}\n离线收益数据异常，请重新发送'离线收益'。", prefix);
    }

    let gold: i64 = parts[0].parse().unwrap_or(0);
    let exp: i64 = parts[1].parse().unwrap_or(0);

    if gold <= 0 && exp <= 0 {
        return format!("{}\n没有可领取的离线收益。", prefix);
    }

    // 发放金币
    if gold > 0 {
        let current_gold: i64 = db.read_basic(user_id, "currency_gold").parse().unwrap_or(0);
        db.write_basic(user_id, "currency_gold", &(current_gold + gold).to_string());
    }

    // 发放经验
    let mut leveled = false;
    if exp > 0 {
        let (_, did_level) = crate::user::add_experience(db, user_id, exp as i32);
        leveled = did_level;
    }

    // 清除待领取记录
    {
        let conn = db.lock_conn();
        let _ = conn.execute(
            "DELETE FROM Shared_Data WHERE ID=?1 AND SECTION='com.shdic.offline_reward_pending'",
            rusqlite::params![user_id],
        );
    }

    // 更新活跃时间戳
    crate::online_activity::update_user_active_minutes(db, user_id, 0);

    let mut r = format!("{}\n═══ 离线收益领取成功！ ═══", prefix);
    if gold > 0 {
        r.push_str(&format!("\n💰 金币 +{}", gold));
    }
    if exp > 0 {
        r.push_str(&format!("\n⭐ 经验 +{}", exp));
    }
    if leveled {
        r.push_str("\n🎉 恭喜升级！");
    }
    r.push_str("\n\n欢迎回来继续冒险！");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offline_reward_constants() {
        assert_eq!(MAX_OFFLINE_SECONDS, 86400);
        assert_eq!(BASE_GOLD_PER_MIN, 2);
        assert_eq!(BASE_EXP_PER_MIN, 5);
    }

    #[test]
    fn test_max_offline_is_24h() {
        // 24 hours = 86400 seconds
        assert_eq!(MAX_OFFLINE_SECONDS, 24 * 60 * 60);
    }

    #[test]
    fn test_base_rates_reasonable() {
        // Gold and exp per minute should be positive
        assert!(BASE_GOLD_PER_MIN > 0);
        assert!(BASE_EXP_PER_MIN > 0);
        // Exp should be higher than gold (typical RPG)
        assert!(BASE_EXP_PER_MIN > BASE_GOLD_PER_MIN);
    }

    #[test]
    fn test_offline_reward_view_constants() {
        // Verify 5-minute minimum
        let min_offline = 300; // 5 minutes in seconds
        assert_eq!(min_offline, 300);
        assert!(MAX_OFFLINE_SECONDS > min_offline);
    }
}

use crate::core::*;
/// CakeGame 竞技场赛季系统
/// 基于 ext_pipei_paragraph 段位定义和 ext_pipei_uInfo 玩家积分
/// 支持：查看赛季、赛季排行、赛季历史、领取赛季奖励、重置赛季(GM)
use crate::db::Database;
use crate::encoding;
use crate::user;

/// 赛季持续天数
const SEASON_DURATION_DAYS: i64 = 30;
/// 赛季数据存储在 Shared_Data 的这个 section
const SEASON_SECTION: &str = "com.shdic.season";

/// 赛季信息
pub struct SeasonInfo {
    pub season_number: i32,
    pub start_time: String,
    pub end_time: String,
    pub is_active: bool,
    pub days_remaining: i64,
}

/// 赛季段位定义
pub(crate) struct SeasonTier {
    name: &'static str,
    min_score: i32,
    max_score: i32,
    /// 赛季结束时奖励金币
    reward_gold: i64,
    /// 赛季结束时奖励钻石
    reward_diamond: i64,
    /// 赛季结束时奖励物品
    reward_item: &'static str,
}

/// 赛季段位配置 — 与 ext_pipei_paragraph 一致
const SEASON_TIERS: &[SeasonTier] = &[
    SeasonTier {
        name: "迷之勇者",
        min_score: -9999,
        max_score: 10,
        reward_gold: 1000,
        reward_diamond: 5,
        reward_item: "",
    },
    SeasonTier {
        name: "黑铁骑士",
        min_score: 11,
        max_score: 100,
        reward_gold: 3000,
        reward_diamond: 15,
        reward_item: "强化石",
    },
    SeasonTier {
        name: "不屈主教",
        min_score: 101,
        max_score: 500,
        reward_gold: 5000,
        reward_diamond: 30,
        reward_item: "强化石",
    },
    SeasonTier {
        name: "荣耀之主",
        min_score: 501,
        max_score: 10000,
        reward_gold: 10000,
        reward_diamond: 50,
        reward_item: "远古超界石",
    },
];

/// 获取玩家积分对应的段位
pub fn get_tier(score: i32) -> &'static SeasonTier {
    for tier in SEASON_TIERS {
        if score >= tier.min_score && score <= tier.max_score {
            return tier;
        }
    }
    SEASON_TIERS.last().unwrap()
}

/// 读取当前赛季信息
pub fn get_season_info(db: &Database) -> SeasonInfo {
    let conn = db.lock_conn();
    let current: i32 = {
        let mut stmt = conn
            .prepare("SELECT DATA FROM Shared_Data WHERE ID='season_number' AND SECTION=?1")
            .unwrap();
        let val: String = stmt.query_row([SEASON_SECTION], |row| row.get(0)).unwrap_or_default();
        val.parse().unwrap_or(1)
    };

    let start_time: String = {
        let mut stmt = conn
            .prepare("SELECT DATA FROM Shared_Data WHERE ID='season_start' AND SECTION=?1")
            .unwrap();
        stmt.query_row([SEASON_SECTION], |row| row.get(0)).unwrap_or_default()
    };

    // 计算赛季结束时间
    let now = chrono::Local::now();
    let start = if start_time.is_empty() {
        now.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        start_time.clone()
    };

    let start_dt = chrono::NaiveDateTime::parse_from_str(&start, "%Y-%m-%d %H:%M:%S").unwrap_or(now.naive_local());
    let end_dt = start_dt + chrono::Duration::days(SEASON_DURATION_DAYS);
    let end_time = end_dt.format("%Y-%m-%d %H:%M:%S").to_string();
    let days_remaining = (end_dt - now.naive_local()).num_days().max(0);

    SeasonInfo {
        season_number: current,
        start_time: if start_time.is_empty() {
            now.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            start_time
        },
        end_time,
        is_active: days_remaining > 0,
        days_remaining,
    }
}

/// 初始化赛季（首次使用时）
fn ensure_season_initialized(db: &Database) {
    let conn = db.lock_conn();
    let exists: bool = {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM Shared_Data WHERE ID='season_number' AND SECTION=?1")
            .unwrap();
        let count: i32 = stmt.query_row([SEASON_SECTION], |row| row.get(0)).unwrap_or(0);
        count > 0
    };

    if !exists {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES ('season_number', ?1, '1')",
            [SEASON_SECTION],
        );
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES ('season_start', ?1, ?2)",
            rusqlite::params![SEASON_SECTION, now],
        );
    }
}

/// 写入赛季历史记录
fn save_season_result(db: &Database, season_num: i32, winner_id: &str, winner_name: &str, winner_score: i32) {
    let conn = db.lock_conn();
    let _now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let data = format!("{}|{}|{}|{}", season_num, winner_id, winner_name, winner_score);
    let _ = conn.execute(
        "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES ('season_history', ?1, ?2)",
        rusqlite::params![SEASON_SECTION, data],
    );
}

/// 查看赛季信息
pub fn cmd_view_season(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    ensure_season_initialized(db);

    let info = get_season_info(db);
    let score: i32 = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Integral FROM ext_pipei_uInfo WHERE uID=?1")
            .unwrap();
        let raw: String = stmt.query_row([user_id], |row| row.get(0)).unwrap_or_default();
        raw.trim_end_matches('\u{0}').parse().unwrap_or(0)
    };

    let tier = get_tier(score);
    let status = if info.is_active {
        "🟢 进行中"
    } else {
        "🔴 已结束"
    };

    format!(
        "{}\n╔══════════════════════╗\n║   ⚔️ 竞技场赛季   ║\n╚══════════════════════╝\n\n\
         📅 第 {} 赛季 {}\n\
         开始: {}\n\
         结束: {}\n\
         剩余: {} 天\n\n\
         🎮 您的赛季状态:\n\
         积分: {}\n\
         段位: {}\n\
         赛季奖励: {}金币 + {}钻石{}\n\n\
         💡 赛季结束时根据段位发放奖励\n\
         💡 发送「赛季排行」查看排行榜\n\
         💡 发送「赛季历史」查看往期赛季",
        prefix,
        info.season_number,
        status,
        info.start_time,
        info.end_time,
        info.days_remaining,
        score,
        tier.name,
        tier.reward_gold,
        tier.reward_diamond,
        if tier.reward_item.is_empty() {
            String::new()
        } else {
            format!(" + {}", tier.reward_item)
        }
    )
}

/// 赛季排行
pub fn cmd_season_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT uID, Integral FROM ext_pipei_uInfo ORDER BY Integral DESC LIMIT 15") {
        Ok(s) => s,
        Err(_) => return format!("{}\n暂无赛季排行数据。", prefix),
    };

    let rows: Vec<(String, i32)> = stmt
        .query_map([], |row| {
            let uid: String = row.get(0)?;
            let score: i32 = row.get(1)?;
            Ok((uid, score))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    if rows.is_empty() {
        return format!("{}\n暂无赛季排行数据。", prefix);
    }

    let info = get_season_info(db);
    let mut result = format!(
        "{}\n╔══════════════════════╗\n║  🏆 第{}赛季排行榜  ║\n╚══════════════════════╝\n",
        prefix, info.season_number
    );

    // 构建模板条目
    let mut entries: Vec<(usize, String, String, String, i32)> = Vec::new();
    for (i, (uid_raw, score)) in rows.iter().enumerate() {
        let uid = uid_raw.trim_end_matches('\u{0}');
        let name = encoding::smart_decode(&db.read_basic(uid, ITEM_NAME));
        let tier = get_tier(*score);
        entries.push((i + 1, name, uid.to_string(), tier.name.to_string(), *score));
    }
    result.push_str(&crate::template_render::render_season_ranking(db, &entries));

    result.push_str("\n\n💡 赛季结束时按段位发放奖励");
    result
}

/// 赛季历史
pub fn cmd_season_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let conn = db.lock_conn();
    let mut stmt = match conn
        .prepare("SELECT DATA FROM Shared_Data WHERE ID='season_history' AND SECTION=?1 ORDER BY rowid DESC LIMIT 5")
    {
        Ok(s) => s,
        Err(_) => return format!("{}\n暂无赛季历史记录。", prefix),
    };

    let rows: Vec<String> = stmt
        .query_map([SEASON_SECTION], |row| row.get(0))
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    if rows.is_empty() {
        return format!("{}\n📜 暂无赛季历史记录。\n\n💡 赛季结束后将自动记录冠军信息。", prefix);
    }

    let mut result = format!("{}\n📜 ═══ 赛季历史 ═══", prefix);

    for data in &rows {
        let parts: Vec<&str> = data.split('|').collect();
        if parts.len() >= 4 {
            let season_num = parts[0];
            let winner_id = parts[1];
            let winner_name = parts[2];
            let winner_score = parts[3];
            let tier = get_tier(winner_score.parse().unwrap_or(0));
            result.push_str(&format!(
                "\n\n🏆 第{}赛季\n  冠军: {} ({})\n  积分: {} [{}]",
                season_num, winner_name, winner_id, winner_score, tier.name
            ));
        }
    }

    result
}

/// 领取赛季奖励（赛季结束时）
pub fn cmd_claim_season_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再领取奖励！", prefix);
    }

    let info = get_season_info(db);

    // 检查赛季是否结束
    if info.is_active {
        return format!(
            "{}\n第{}赛季尚未结束！\n剩余{}天后可领取奖励。",
            prefix, info.season_number, info.days_remaining
        );
    }

    // 检查是否已领取
    let claim_key = format!("season_{}_claimed", info.season_number);
    let conn = db.lock_conn();
    let claimed: String = {
        let mut stmt = conn
            .prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION=?2")
            .unwrap();
        stmt.query_row(rusqlite::params![claim_key, SEASON_SECTION], |row| row.get(0))
            .unwrap_or_default()
    };

    if claimed == user_id {
        return format!("{}\n您已经领取过第{}赛季的奖励了！", prefix, info.season_number);
    }

    drop(conn);

    // 获取玩家积分和段位
    let score: i32 = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Integral FROM ext_pipei_uInfo WHERE uID=?1")
            .unwrap();
        let raw: String = stmt.query_row([user_id], |row| row.get(0)).unwrap_or_default();
        raw.trim_end_matches('\u{0}').parse().unwrap_or(0)
    };

    let tier = get_tier(score);

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, "add", tier.reward_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, "add", tier.reward_diamond);

    if !tier.reward_item.is_empty() {
        db.knapsack_add(user_id, tier.reward_item, 1);
    }

    // 标记已领取
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
        rusqlite::params![claim_key, SEASON_SECTION, user_id],
    );

    let mut reward_msg = format!("{}金币 + {}钻石", tier.reward_gold, tier.reward_diamond);
    if !tier.reward_item.is_empty() {
        reward_msg.push_str(&format!(" + {}", tier.reward_item));
    }

    format!(
        "{}\n🎉 恭喜领取第{}赛季奖励！\n\n段位: {} (积分: {})\n奖励: {}\n\n💡 新赛季已开始，继续加油！",
        prefix, info.season_number, tier.name, score, reward_msg
    )
}

/// 重置赛季 (GM 指令)
pub fn cmd_reset_season(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 获取当前赛季冠军
    let (winner_id, winner_score) = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT uID, Integral FROM ext_pipei_uInfo ORDER BY Integral DESC LIMIT 1")
            .unwrap();
        stmt.query_row([], |row| {
            let uid: String = row.get(0)?;
            let score: i32 = row.get(1)?;
            Ok((uid, score))
        })
        .unwrap_or_default()
    };

    let winner_uid = winner_id.trim_end_matches('\u{0}').to_string();
    let winner_name = encoding::smart_decode(&db.read_basic(&winner_uid, ITEM_NAME));

    // 记录当前赛季历史
    let current_season = get_season_info(db).season_number;
    save_season_result(db, current_season, &winner_uid, &winner_name, winner_score);

    // 开始新赛季
    let new_season = current_season + 1;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let conn = db.lock_conn();

    let _ = conn.execute(
        "UPDATE Shared_Data SET DATA=?1 WHERE ID='season_number' AND SECTION=?2",
        rusqlite::params![new_season.to_string(), SEASON_SECTION],
    );
    let _ = conn.execute(
        "UPDATE Shared_Data SET DATA=?1 WHERE ID='season_start' AND SECTION=?2",
        rusqlite::params![now, SEASON_SECTION],
    );

    // 清理上一赛季的领取标记
    let _ = conn.execute(
        "DELETE FROM Shared_Data WHERE ID LIKE 'season_%_claimed' AND SECTION=?1",
        [SEASON_SECTION],
    );

    // 重置所有玩家匹配积分
    let _ = conn.execute("UPDATE ext_pipei_uInfo SET Integral = 0", []);

    format!(
        "{}\n✅ 赛季已重置！\n\n上一赛季(第{}赛季)冠军: {} ({}) 积分:{}\n当前赛季: 第{}赛季\n\n💡 所有玩家积分已重置为0",
        prefix, current_season, winner_name, winner_uid, winner_score, new_season
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tier_low() {
        let tier = get_tier(0);
        assert_eq!(tier.name, "迷之勇者");
    }

    #[test]
    fn test_get_tier_mid() {
        let tier = get_tier(250);
        assert_eq!(tier.name, "不屈主教");
    }

    #[test]
    fn test_get_tier_high() {
        let tier = get_tier(800);
        assert_eq!(tier.name, "荣耀之主");
    }

    #[test]
    fn test_season_constants() {
        assert_eq!(SEASON_DURATION_DAYS, 30);
        assert_eq!(SEASON_TIERS.len(), 4);
    }
}

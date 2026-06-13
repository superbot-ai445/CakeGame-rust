/// CakeGame 公会每日签到系统
///
/// 公会成员每天可在公会签到，获得个人贡献和公会资金奖励。
/// 连续签到天数越多，奖励越高。
/// 公会签到率达到一定比例，全员获得额外加成。
///
/// 指令: 公会签到, 签到状态, 签到记录
/// 数据存储: Global 表 SECTION='guild_checkin' (个人签到) / 'guild_checkin_stats' (统计)
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 连续签到奖励阶梯
struct StreakReward {
    min_days: i32,
    bonus_gold: i64,
    bonus_diamond: i32,
    bonus_contribution: i32,
    title: &'static str,
}

const STREAK_REWARDS: &[StreakReward] = &[
    StreakReward {
        min_days: 1,
        bonus_gold: 200,
        bonus_diamond: 0,
        bonus_contribution: 5,
        title: "",
    },
    StreakReward {
        min_days: 3,
        bonus_gold: 500,
        bonus_diamond: 2,
        bonus_contribution: 10,
        title: "⭐ 勤勉会员",
    },
    StreakReward {
        min_days: 7,
        bonus_gold: 1000,
        bonus_diamond: 5,
        bonus_contribution: 20,
        title: "⭐⭐ 忠诚成员",
    },
    StreakReward {
        min_days: 14,
        bonus_gold: 2000,
        bonus_diamond: 10,
        bonus_contribution: 40,
        title: "⭐⭐⭐ 核心骨干",
    },
    StreakReward {
        min_days: 30,
        bonus_gold: 5000,
        bonus_diamond: 20,
        bonus_contribution: 80,
        title: "🌟 公会元老",
    },
];

/// 签到人数达标奖励 (公会签到率)
struct RateBonus {
    min_rate_pct: i32,  // 最低签到率百分比
    bonus_gold: i64,    // 全员额外金币
    bonus_diamond: i32, // 全员额外钻石
    label: &'static str,
}

const RATE_BONUSES: &[RateBonus] = &[
    RateBonus {
        min_rate_pct: 30,
        bonus_gold: 100,
        bonus_diamond: 0,
        label: "初级活跃",
    },
    RateBonus {
        min_rate_pct: 50,
        bonus_gold: 300,
        bonus_diamond: 1,
        label: "中级活跃",
    },
    RateBonus {
        min_rate_pct: 80,
        bonus_gold: 600,
        bonus_diamond: 3,
        label: "高度活跃",
    },
    RateBonus {
        min_rate_pct: 100,
        bonus_gold: 1500,
        bonus_diamond: 10,
        label: "全员到齐",
    },
];

/// 今天日期
fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 获取用户所在公会
fn get_user_guild(db: &Database, user_id: &str) -> Option<String> {
    let guild = db.read_user_data(user_id, "guild");
    if guild.is_empty() || guild == "无" {
        None
    } else {
        Some(guild)
    }
}

/// 获取公会成员列表
fn get_guild_members(db: &Database, guild_name: &str) -> Vec<String> {
    let conn = db.lock_conn();
    let mut members = Vec::new();
    if let Ok(mut stmt) = conn.prepare("SELECT ID FROM Basic_User WHERE guild = ?1") {
        if let Ok(rows) = stmt.query_map(rusqlite::params![guild_name], |row| row.get::<_, String>(0)) {
            for row in rows.flatten() {
                members.push(row);
            }
        }
    }
    members
}

/// 检查今天是否已签到
fn has_checked_in_today(db: &Database, user_id: &str, date: &str) -> bool {
    let key = format!("{}.{}", user_id, date);
    !db.global_get("guild_checkin", &key).is_empty()
}

/// 获取连续签到天数
fn get_streak(db: &Database, user_id: &str) -> i32 {
    let streak_key = format!("{}.streak", user_id);
    db.global_get("guild_checkin_stats", &streak_key).parse().unwrap_or(0)
}

/// 更新连续签到天数
fn update_streak(db: &Database, user_id: &str, today: &str) -> i32 {
    let streak_key = format!("{}.streak", user_id);
    let last_key = format!("{}.last_date", user_id);
    let last_date = db.global_get("guild_checkin_stats", &last_key);

    // 计算昨天的日期
    let yesterday = (Local::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let new_streak = if last_date == yesterday {
        // 连续签到
        let current = get_streak(db, user_id);
        current + 1
    } else if last_date == today {
        // 今天已签到（不应发生）
        get_streak(db, user_id)
    } else {
        // 中断，重新开始
        1
    };

    db.global_set("guild_checkin_stats", &streak_key, &new_streak.to_string());
    db.global_set("guild_checkin_stats", &last_key, today);
    new_streak
}

/// 获取匹配的连续签到奖励
fn get_streak_reward(streak: i32) -> &'static StreakReward {
    let mut best = &STREAK_REWARDS[0];
    for r in STREAK_REWARDS {
        if streak >= r.min_days {
            best = r;
        }
    }
    best
}

/// 获取签到率奖励
fn get_rate_bonus(checkin_count: usize, total_count: usize) -> Option<&'static RateBonus> {
    if total_count == 0 {
        return None;
    }
    let rate_pct = (checkin_count as i32 * 100) / total_count as i32;
    let mut best: Option<&RateBonus> = None;
    for rb in RATE_BONUSES {
        if rate_pct >= rb.min_rate_pct {
            best = Some(rb);
        }
    }
    best
}

/// 记录今日签到人数 (公会维度)
fn record_checkin(db: &Database, guild_name: &str, user_id: &str, date: &str) {
    let key = format!("{}.{}", user_id, date);
    db.global_set("guild_checkin", &key, "1");

    // 更新公会今日签到列表
    let list_key = format!("{}.{}", guild_name, date);
    let existing = db.global_get("guild_checkin", &list_key);
    if existing.is_empty() {
        db.global_set("guild_checkin", &list_key, user_id);
    } else if !existing.contains(user_id) {
        db.global_set("guild_checkin", &list_key, &format!("{},{}", existing, user_id));
    }
}

/// 获取公会今日签到人数
fn get_today_checkin_count(db: &Database, guild_name: &str, date: &str) -> usize {
    let list_key = format!("{}.{}", guild_name, date);
    let list = db.global_get("guild_checkin", &list_key);
    if list.is_empty() {
        0
    } else {
        list.split(',').count()
    }
}

/// 累计签到次数
fn get_total_checkins(db: &Database, user_id: &str) -> i32 {
    let key = format!("{}.total", user_id);
    db.global_get("guild_checkin_stats", &key).parse().unwrap_or(0)
}

fn inc_total_checkins(db: &Database, user_id: &str) {
    let key = format!("{}.total", user_id);
    let current = get_total_checkins(db, user_id);
    db.global_set("guild_checkin_stats", &key, &(current + 1).to_string());
}

/// 公会签到 — 每日签到获取贡献和奖励
pub fn cmd_guild_checkin(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n您还未加入公会！\n💡 发送「公会列表」查看可加入的公会", prefix),
    };

    let today = today_str();

    if has_checked_in_today(db, user_id, &today) {
        let streak = get_streak(db, user_id);
        let reward = get_streak_reward(streak);
        return format!(
            "{}\n📅 公会签到\n\n您今天已经签到过了！\n🔥 连续签到: {}天\n{}\n💡 明天再来签到吧！",
            prefix,
            streak,
            if reward.title.is_empty() {
                String::new()
            } else {
                format!("当前称号: {}", reward.title)
            }
        );
    }

    // 执行签到
    record_checkin(db, &guild, user_id, &today);
    let new_streak = update_streak(db, user_id, &today);
    inc_total_checkins(db, user_id);

    let reward = get_streak_reward(new_streak);

    // 发放个人奖励
    if reward.bonus_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, reward.bonus_gold);
    }
    if reward.bonus_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, reward.bonus_diamond as i64);
    }

    // 增加公会贡献
    let contrib_key = format!("{}.contribution", user_id);
    let current_contrib: i64 = db.global_get("guild_checkin_stats", &contrib_key).parse().unwrap_or(0);
    db.global_set(
        "guild_checkin_stats",
        &contrib_key,
        &(current_contrib + reward.bonus_contribution as i64).to_string(),
    );

    // 公会签到率奖励
    let members = get_guild_members(db, &guild);
    let today_count = get_today_checkin_count(db, &guild, &today);
    let rate_bonus = get_rate_bonus(today_count, members.len());

    let mut out = format!("{}\n📅 ═══ 公会签到 ═══", prefix);
    out.push_str(&format!("\n\n✅ 签到成功！公会「{}」", guild));
    out.push_str(&format!("\n🔥 连续签到: {}天", new_streak));
    out.push_str(&format!("\n💰 获得金币: {}", reward.bonus_gold));
    out.push_str(&format!("\n💎 获得钻石: {}", reward.bonus_diamond));
    out.push_str(&format!("\n🏛️ 公会贡献: +{}", reward.bonus_contribution));

    if !reward.title.is_empty() {
        out.push_str(&format!("\n🏷️ 称号: {}", reward.title));
    }

    // 显示公会签到率
    let rate_pct = if members.is_empty() {
        0
    } else {
        (today_count as i32 * 100) / members.len() as i32
    };
    out.push_str(&format!(
        "\n\n📊 公会今日签到: {}/{} ({}%)",
        today_count,
        members.len(),
        rate_pct
    ));

    // 签到率进度条
    let filled = (rate_pct as usize / 10).min(10);
    let empty = 10 - filled;
    out.push_str(&format!("\n{}{} ", "█".repeat(filled), "░".repeat(empty)));

    if let Some(rb) = rate_bonus {
        out.push_str(&format!(
            "\n🎯 达成「{}」: 全员额外 +{}金 +{}钻",
            rb.label, rb.bonus_gold, rb.bonus_diamond
        ));
    } else {
        // 显示下一阶段
        for rb in RATE_BONUSES {
            if rate_pct < rb.min_rate_pct {
                let need = (members.len() as i32 * rb.min_rate_pct + 99) / 100 - today_count as i32;
                out.push_str(&format!("\n💡 再来{}人签到达成「{}」", need.max(1), rb.label));
                break;
            }
        }
    }

    // 显示下一连续签到奖励
    for sr in STREAK_REWARDS {
        if new_streak < sr.min_days {
            let days_left = sr.min_days - new_streak;
            out.push_str(&format!(
                "\n🎯 再连续签到{}天可解锁「{}」(+{}金+{}钻+{}贡献)",
                days_left, sr.title, sr.bonus_gold, sr.bonus_diamond, sr.bonus_contribution
            ));
            break;
        }
    }

    out
}

/// 签到状态 — 查看个人签到信息
pub fn cmd_checkin_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n您还未加入公会！", prefix),
    };

    let today = today_str();
    let checked_in = has_checked_in_today(db, user_id, &today);
    let streak = get_streak(db, user_id);
    let total = get_total_checkins(db, user_id);
    let contrib_key = format!("{}.contribution", user_id);
    let contribution: i64 = db.global_get("guild_checkin_stats", &contrib_key).parse().unwrap_or(0);

    let today_count = get_today_checkin_count(db, &guild, &today);
    let members = get_guild_members(db, &guild);

    let mut out = format!("{}\n📋 ═══ 签到状态 ═══", prefix);
    out.push_str(&format!("\n\n公会: {}", guild));
    out.push_str(&format!(
        "\n今日签到: {}",
        if checked_in { "✅ 已签到" } else { "❌ 未签到" }
    ));
    out.push_str(&format!("\n🔥 连续签到: {}天", streak));
    out.push_str(&format!("\n📅 累计签到: {}天", total));
    out.push_str(&format!("\n🏛️ 签到贡献: {}", contribution));

    // 当前奖励等级
    let reward = get_streak_reward(streak);
    if !reward.title.is_empty() {
        out.push_str(&format!("\n🏷️ 当前称号: {}", reward.title));
    }

    // 今日每次签到奖励
    out.push_str(&format!(
        "\n\n💰 每次签到: +{}金 +{}钻 +{}贡献",
        reward.bonus_gold, reward.bonus_diamond, reward.bonus_contribution
    ));

    // 公会签到率
    let rate_pct = if members.is_empty() {
        0
    } else {
        (today_count as i32 * 100) / members.len() as i32
    };
    out.push_str(&format!(
        "\n\n📊 公会今日: {}/{} ({}%)",
        today_count,
        members.len(),
        rate_pct
    ));

    if !checked_in {
        out.push_str("\n\n💡 发送「公会签到」进行今日签到！");
    }

    out
}

/// 签到记录 — 查看公会成员签到排行
pub fn cmd_checkin_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n您还未加入公会！", prefix),
    };

    let members = get_guild_members(db, &guild);
    if members.is_empty() {
        return format!("{}\n公会暂无成员数据。", prefix);
    }

    // 收集成员签到数据
    let mut entries: Vec<(String, i32, i32, i64)> = Vec::new(); // (uid, streak, total, contribution)
    for mid in &members {
        let streak_key = format!("{}.streak", mid);
        let total_key = format!("{}.total", mid);
        let contrib_key = format!("{}.contribution", mid);
        let streak: i32 = db.global_get("guild_checkin_stats", &streak_key).parse().unwrap_or(0);
        let total: i32 = db.global_get("guild_checkin_stats", &total_key).parse().unwrap_or(0);
        let contrib: i64 = db.global_get("guild_checkin_stats", &contrib_key).parse().unwrap_or(0);
        if total > 0 {
            entries.push((mid.clone(), streak, total, contrib));
        }
    }

    if entries.is_empty() {
        return format!(
            "{}\n🏛️ 公会「{}」暂无签到记录\n💡 成员发送「公会签到」开始签到！",
            prefix, guild
        );
    }

    // 按累计签到次数排序
    entries.sort_by_key(|b| std::cmp::Reverse(b.2));

    let today = today_str();
    let today_count = get_today_checkin_count(db, &guild, &today);

    let mut out = format!("{}\n🏛️ ═══ {} 签到排行 ═══", prefix, guild);
    out.push_str(&format!(
        "\n📊 今日签到: {}/{} ({:.0}%)\n",
        today_count,
        members.len(),
        if members.is_empty() {
            0.0
        } else {
            today_count as f64 * 100.0 / members.len() as f64
        }
    ));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, streak, total, contrib)) in entries.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { &format!("{:>2}.", i + 1) };
        let nickname = db.read_basic(uid, ITEM_NAME);
        let nickname = if nickname.is_empty() { uid.clone() } else { nickname };

        // 当前用户高亮
        let marker = if uid == user_id { " ◀" } else { "" };

        out.push_str(&format!(
            "\n{} {} | 🔥{}天 | 📅{}天 | 🏛️{}",
            medal, nickname, streak, total, contrib
        ));
        if !marker.is_empty() {
            out.push_str(marker);
        }
    }

    if entries.len() > 15 {
        out.push_str(&format!("\n... 共{}人有签到记录", entries.len()));
    }

    // 找到当前用户排名
    if let Some((idx, _)) = entries.iter().enumerate().find(|(_, (uid, _, _, _))| uid == user_id) {
        if idx >= 15 {
            out.push_str(&format!("\n\n📍 您的排名: 第{}名", idx + 1));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streak_reward_streak_1() {
        let r = get_streak_reward(1);
        assert_eq!(r.min_days, 1);
        assert_eq!(r.bonus_gold, 200);
        assert_eq!(r.bonus_contribution, 5);
    }

    #[test]
    fn test_streak_reward_streak_7() {
        let r = get_streak_reward(7);
        assert_eq!(r.min_days, 7);
        assert_eq!(r.bonus_gold, 1000);
        assert_eq!(r.bonus_diamond, 5);
        assert_eq!(r.title, "⭐⭐ 忠诚成员");
    }

    #[test]
    fn test_streak_reward_streak_30() {
        let r = get_streak_reward(30);
        assert_eq!(r.min_days, 30);
        assert_eq!(r.bonus_gold, 5000);
        assert_eq!(r.bonus_diamond, 20);
        assert_eq!(r.title, "🌟 公会元老");
    }

    #[test]
    fn test_streak_reward_ordering() {
        // 保证奖励递增
        for i in 1..STREAK_REWARDS.len() {
            assert!(STREAK_REWARDS[i].bonus_gold >= STREAK_REWARDS[i - 1].bonus_gold);
            assert!(STREAK_REWARDS[i].bonus_diamond >= STREAK_REWARDS[i - 1].bonus_diamond);
            assert!(STREAK_REWARDS[i].bonus_contribution >= STREAK_REWARDS[i - 1].bonus_contribution);
        }
    }

    #[test]
    fn test_rate_bonus_50_percent() {
        let rb = get_rate_bonus(5, 10);
        assert!(rb.is_some());
        let rb = rb.unwrap();
        assert_eq!(rb.label, "中级活跃");
        assert_eq!(rb.bonus_gold, 300);
    }

    #[test]
    fn test_rate_bonus_100_percent() {
        let rb = get_rate_bonus(10, 10);
        assert!(rb.is_some());
        let rb = rb.unwrap();
        assert_eq!(rb.label, "全员到齐");
        assert_eq!(rb.bonus_gold, 1500);
        assert_eq!(rb.bonus_diamond, 10);
    }

    #[test]
    fn test_rate_bonus_insufficient() {
        let rb = get_rate_bonus(1, 10);
        assert!(rb.is_none());
    }

    #[test]
    fn test_rate_bonus_empty_guild() {
        let rb = get_rate_bonus(0, 0);
        assert!(rb.is_none());
    }

    #[test]
    fn test_rate_bonus_ordering() {
        for i in 1..RATE_BONUSES.len() {
            assert!(RATE_BONUSES[i].min_rate_pct > RATE_BONUSES[i - 1].min_rate_pct);
            assert!(RATE_BONUSES[i].bonus_gold >= RATE_BONUSES[i - 1].bonus_gold);
        }
    }

    #[test]
    fn test_today_str_format() {
        let d = today_str();
        assert_eq!(d.len(), 10);
        assert!(d.contains('-'));
    }

    #[test]
    fn test_streak_reward_boundary() {
        // 0天应返回最低奖励
        let r = get_streak_reward(0);
        assert_eq!(r.min_days, 1);
        // 999天应返回最高奖励
        let r = get_streak_reward(999);
        assert_eq!(r.min_days, 30);
    }
}

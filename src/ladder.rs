/// CakeGame 天梯段位系统 (Ladder League System)
///
/// 综合竞技排名系统，玩家通过各种游戏活动获取天梯积分，
/// 攀升段位等级，获得段位专属奖励和属性加成。
///
/// 7大段位:
///   青铜🥉 (0~499) → 白银🥈 (500~1499) → 黄金🏅 (1500~2999)
///   → 铂金💎 (3000~4999) → 钻石💠 (5000~7999) → 大师🌟 (8000~11999)
///   → 宗师👑 (12000+)
///
/// 积分来源 (8种):
///   PvP胜利(+30) / 击败BOSS(+20) / 深渊通关(+15) / 竞技场胜利(+25)
///   / 公会战(+40) / 完成任务(+10) / 每日签到(+5) / 合成成功(+8)
///
/// 数据存储: Global 表 SECTION='ladder'
///   - points:{uid}    → 当前天梯积分
///   - best:{uid}      → 历史最高段位等级
///   - rewards:{uid}   → 已领取奖励位掩码
///   - daily:{uid}     → 今日获得积分(防刷)
///   - history:{uid}   → 段位变更历史 (最近10条)
///   - season:{uid}    → 赛季积分记录
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use crate::user;

const SECTION: &str = "ladder";

/// 每日积分上限
const DAILY_POINT_CAP: i32 = 500;

/// 段位定义
struct LeagueTier {
    level: i32,
    name: &'static str,
    emoji: &'static str,
    min_points: i32,
    /// 属性加成: (HP, 物攻, 魔攻, 防御, 魔抗)
    bonus: (i32, i32, i32, i32, i32),
    /// 晋级奖励: (金币, 钻石)
    promote_reward: (i64, i64),
    /// 奖励道具
    reward_item: &'static str,
}

const LEAGUES: &[LeagueTier] = &[
    LeagueTier {
        level: 1,
        name: "青铜",
        emoji: "🥉",
        min_points: 0,
        bonus: (0, 0, 0, 0, 0),
        promote_reward: (0, 0),
        reward_item: "",
    },
    LeagueTier {
        level: 2,
        name: "白银",
        emoji: "🥈",
        min_points: 500,
        bonus: (100, 20, 20, 10, 10),
        promote_reward: (3000, 30),
        reward_item: "强化石",
    },
    LeagueTier {
        level: 3,
        name: "黄金",
        emoji: "🏅",
        min_points: 1500,
        bonus: (300, 50, 50, 25, 25),
        promote_reward: (8000, 80),
        reward_item: "高级药水",
    },
    LeagueTier {
        level: 4,
        name: "铂金",
        emoji: "💎",
        min_points: 3000,
        bonus: (600, 100, 100, 50, 50),
        promote_reward: (20000, 200),
        reward_item: "凤凰之羽",
    },
    LeagueTier {
        level: 5,
        name: "钻石",
        emoji: "💠",
        min_points: 5000,
        bonus: (1000, 180, 180, 90, 90),
        promote_reward: (50000, 500),
        reward_item: "传说宝箱",
    },
    LeagueTier {
        level: 6,
        name: "大师",
        emoji: "🌟",
        min_points: 8000,
        bonus: (2000, 350, 350, 180, 180),
        promote_reward: (100000, 1000),
        reward_item: "至尊宝箱",
    },
    LeagueTier {
        level: 7,
        name: "宗师",
        emoji: "👑",
        min_points: 12000,
        bonus: (3500, 600, 600, 300, 300),
        promote_reward: (200000, 2000),
        reward_item: "天梯至尊宝箱",
    },
];

/// 积分来源定义
#[allow(dead_code)]
struct PointSource {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    points: i32,
    daily_limit: i32,
}

const POINT_SOURCES: &[PointSource] = &[
    PointSource {
        id: "pvp_win",
        name: "PvP胜利",
        emoji: "⚔️",
        points: 30,
        daily_limit: 150,
    },
    PointSource {
        id: "boss_kill",
        name: "击败BOSS",
        emoji: "🐉",
        points: 20,
        daily_limit: 100,
    },
    PointSource {
        id: "abyss_clear",
        name: "深渊通关",
        emoji: "🕳️",
        points: 15,
        daily_limit: 75,
    },
    PointSource {
        id: "arena_win",
        name: "竞技场胜利",
        emoji: "🏟️",
        points: 25,
        daily_limit: 125,
    },
    PointSource {
        id: "guild_war",
        name: "公会战",
        emoji: "🏰",
        points: 40,
        daily_limit: 120,
    },
    PointSource {
        id: "quest_done",
        name: "完成任务",
        emoji: "📋",
        points: 10,
        daily_limit: 100,
    },
    PointSource {
        id: "sign_in",
        name: "每日签到",
        emoji: "📅",
        points: 5,
        daily_limit: 5,
    },
    PointSource {
        id: "compose",
        name: "合成成功",
        emoji: "🔧",
        points: 8,
        daily_limit: 80,
    },
];

/// 获取玩家当前段位信息
fn get_league(points: i32) -> &'static LeagueTier {
    let mut result = &LEAGUES[0];
    for tier in LEAGUES {
        if points >= tier.min_points {
            result = tier;
        }
    }
    result
}

/// 获取今日已获积分
fn get_daily_points(db: &Database, user_id: &str) -> i32 {
    let raw = db.global_get(SECTION, &format!("daily:{}", user_id));
    if raw.is_empty() || raw == "[NULL]" {
        return 0;
    }
    // 检查是否是今天的数据 (格式: "日期|积分")
    let parts: Vec<&str> = raw.splitn(2, '|').collect();
    if parts.len() == 2 {
        let today = chrono_today();
        if parts[0] == today {
            return parts[1].parse::<i32>().unwrap_or(0);
        }
    }
    0
}

/// 设置今日已获积分
#[allow(dead_code)]
fn set_daily_points(db: &Database, user_id: &str, points: i32) {
    let today = chrono_today();
    db.global_set(SECTION, &format!("daily:{}", user_id), &format!("{}|{}", today, points));
}

/// 简单的日期获取 (年月日)
fn chrono_today() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // UTC+8
    let secs = secs + 8 * 3600;
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days as i64 + 719468);
    format!("{:04}{:02}{:02}", y, m, d)
}

fn days_to_ymd(g: i64) -> (i64, i64, i64) {
    let y = (10000 * g + 14780) / 3652425;
    let mut doy = g - (365 * y + y / 4 - y / 100 + y / 400);
    if doy < 0 {
        let y2 = y - 1;
        doy = g - (365 * y2 + y2 / 4 - y2 / 100 + y2 / 400);
    }
    let mi = (100 * doy + 52) / 3060;
    let month = (mi + 2) % 12 + 1;
    let year = y + (mi + 2) / 12;
    let day = doy - (mi * 306 + 5) / 10 + 1;
    (year, month, day)
}

/// 记录天梯积分 (公共API，供其他模块调用)
///
/// source: 积分来源ID (pvp_win, boss_kill, abyss_clear, arena_win, guild_war, quest_done, sign_in, compose)
/// 返回实际获得的积分
#[allow(dead_code)]
pub fn record_ladder_points(db: &Database, user_id: &str, source: &str) -> i32 {
    // 查找积分来源
    let src = match POINT_SOURCES.iter().find(|s| s.id == source) {
        Some(s) => s,
        None => return 0,
    };

    // 检查每日上限
    let daily = get_daily_points(db, user_id);
    if daily >= DAILY_POINT_CAP {
        return 0;
    }

    // 检查来源每日上限
    let source_daily_raw = db.global_get(SECTION, &format!("sd:{}:{}", user_id, source));
    let today = chrono_today();
    let source_daily = if !source_daily_raw.is_empty() && source_daily_raw != "[NULL]" {
        let parts: Vec<&str> = source_daily_raw.splitn(2, '|').collect();
        if parts.len() == 2 && parts[0] == today {
            parts[1].parse::<i32>().unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    if source_daily >= src.daily_limit {
        return 0;
    }

    // 计算实际积分 (不超过各上限)
    let remaining_daily = DAILY_POINT_CAP - daily;
    let remaining_source = src.daily_limit - source_daily;
    let actual = src.points.min(remaining_daily).min(remaining_source);
    if actual <= 0 {
        return 0;
    }

    // 更新积分
    let old_points: i32 = {
        let raw = db.global_get(SECTION, &format!("points:{}", user_id));
        raw.parse::<i32>().unwrap_or(0)
    };
    let new_points = old_points + actual;
    db.global_set(SECTION, &format!("points:{}", user_id), &new_points.to_string());

    // 更新每日积分
    set_daily_points(db, user_id, daily + actual);

    // 更新来源每日积分
    let new_source_daily = source_daily + actual;
    db.global_set(
        SECTION,
        &format!("sd:{}:{}", user_id, source),
        &format!("{}|{}", today, new_source_daily),
    );

    // 检查是否晋级
    let old_league = get_league(old_points);
    let new_league = get_league(new_points);
    if new_league.level > old_league.level {
        // 记录晋级历史
        let history_raw = db.global_get(SECTION, &format!("history:{}", user_id));
        let mut entries: Vec<String> = if !history_raw.is_empty() && history_raw != "[NULL]" {
            history_raw.split('|').map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };
        entries.push(format!("{}→{}({})", old_league.name, new_league.name, chrono_today()));
        if entries.len() > 10 {
            entries.drain(0..entries.len() - 10);
        }
        db.global_set(SECTION, &format!("history:{}", user_id), &entries.join("|"));

        // 更新历史最高段位
        db.global_set(SECTION, &format!("best:{}", user_id), &new_league.level.to_string());
    }

    actual
}

/// 获取天梯段位属性加成 (公共API)
#[allow(dead_code)]
pub fn get_ladder_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let raw = db.global_get(SECTION, &format!("points:{}", user_id));
    let points = raw.parse::<i32>().unwrap_or(0);
    let league = get_league(points);
    league.bonus
}

/// 查看天梯段位
pub fn cmd_ladder_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get(SECTION, &format!("points:{}", user_id));
    let points = raw.parse::<i32>().unwrap_or(0);
    let league = get_league(points);
    let daily = get_daily_points(db, user_id);

    // 计算到下一段位的距离
    let next_info = if league.level < 7 {
        let next = LEAGUES.iter().find(|t| t.level == league.level + 1);
        if let Some(n) = next {
            format!("距离{}{}还需 {} 积分", n.emoji, n.name, n.min_points - points)
        } else {
            "已达最高段位！".to_string()
        }
    } else {
        "已达最高段位！🏆".to_string()
    };

    // 属性加成
    let bonus_str = if league.bonus.0 > 0 {
        format!(
            "HP+{} 物攻+{} 魔攻+{} 防御+{} 魔抗+{}",
            league.bonus.0, league.bonus.1, league.bonus.2, league.bonus.3, league.bonus.4
        )
    } else {
        "无".to_string()
    };

    format!(
        "{}\n\
         ═══ 天梯段位 ═══\n\
         当前段位: {}{}\n\
         天梯积分: {}\n\
         今日积分: {}/{}\n\
         属性加成: {}\n\
         {}\n\
         ─────────────\n\
         输入「天梯奖励」领取段位奖励\n\
         输入「天梯排行」查看全服排名\n\
         输入「天梯来源」查看积分获取方式",
        prefix, league.emoji, league.name, points, daily, DAILY_POINT_CAP, bonus_str, next_info,
    )
}

/// 领取段位晋级奖励
pub fn cmd_ladder_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get(SECTION, &format!("points:{}", user_id));
    let points = raw.parse::<i32>().unwrap_or(0);
    let league = get_league(points);

    // 读取已领取奖励位掩码
    let reward_raw = db.global_get(SECTION, &format!("rewards:{}", user_id));
    let claimed: i64 = reward_raw.parse::<i64>().unwrap_or(0);

    let bit = 1i64 << (league.level - 1);
    if claimed & bit != 0 {
        return format!(
            "{}\n\
             ❌ 您当前段位 {}{} 的奖励已领取！\n\
             继续提升段位可领取更高级奖励。",
            prefix, league.emoji, league.name,
        );
    }

    if league.level <= 1 {
        return format!(
            "{}\n\
             ❌ 青铜段位暂无奖励，请先提升到白银段位！",
            prefix,
        );
    }

    // 发放奖励
    if league.promote_reward.0 > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, league.promote_reward.0);
    }
    if league.promote_reward.1 > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, league.promote_reward.1);
    }
    if !league.reward_item.is_empty() {
        db.knapsack_add(user_id, league.reward_item, 1);
    }

    // 标记已领取
    let new_claimed = claimed | bit;
    db.global_set(SECTION, &format!("rewards:{}", user_id), &new_claimed.to_string());

    let mut rewards = Vec::new();
    if league.promote_reward.0 > 0 {
        rewards.push(format!("{}金币", league.promote_reward.0));
    }
    if league.promote_reward.1 > 0 {
        rewards.push(format!("{}钻石", league.promote_reward.1));
    }
    if !league.reward_item.is_empty() {
        rewards.push(league.reward_item.to_string());
    }

    format!(
        "{}\n\
         🎉 恭喜领取 {}{} 段位奖励！\n\
         获得: {}\n\
         继续战斗，冲击更高段位！",
        prefix,
        league.emoji,
        league.name,
        rewards.join(" + "),
    )
}

/// 天梯排行 (全服前15名)
pub fn cmd_ladder_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let all_users = db.all_users();

    let mut rankings: Vec<(String, String, i32)> = Vec::new();
    for uid in &all_users {
        let raw = db.global_get(SECTION, &format!("points:{}", uid));
        let pts = raw.parse::<i32>().unwrap_or(0);
        if pts > 0 {
            let nickname = db.read_basic(uid, "Nickname");
            let name = if nickname.is_empty() || nickname == "[NULL]" {
                uid.clone()
            } else {
                nickname
            };
            rankings.push((uid.clone(), name, pts));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut result = format!("{}\n═══ 天梯排行榜 ═══\n", prefix);

    for (i, (uid, name, pts)) in rankings.iter().take(15).enumerate() {
        let league = get_league(*pts);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let is_self = uid == user_id;
        let mark = if is_self { " ← 您" } else { "" };
        result.push_str(&format!(
            "\n{}. {} {}{} {}{} ({}分){}",
            i + 1,
            medal,
            league.emoji,
            name,
            league.name,
            "",
            pts,
            mark,
        ));
    }

    if rankings.is_empty() {
        result.push_str("\n暂无排名数据");
    }

    // 显示自己的排名
    if let Some((i, _)) = rankings.iter().enumerate().find(|(_, (uid, _, _))| uid == user_id) {
        result.push_str(&format!("\n\n📍 您的排名: 第{}名", i + 1));
    } else {
        result.push_str("\n\n📍 您尚未获得天梯积分");
    }

    result
}

/// 查看积分获取方式
pub fn cmd_ladder_sources(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, _user_id);
    let mut result = format!("{}\n═══ 天梯积分来源 ═══\n", prefix);

    for src in POINT_SOURCES {
        result.push_str(&format!(
            "\n{} {} +{}分/次 (每日上限:{})",
            src.emoji, src.name, src.points, src.daily_limit,
        ));
    }

    result.push_str(&format!(
        "\n\n📊 每日积分总上限: {} 分\n\
         💡 积分来源涵盖PvP/BOSS/深渊/竞技/公会/任务/签到/合成八大系统",
        DAILY_POINT_CAP,
    ));

    result
}

/// 查看天梯段位详情
pub fn cmd_ladder_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get(SECTION, &format!("points:{}", user_id));
    let points = raw.parse::<i32>().unwrap_or(0);
    let league = get_league(points);

    // 历史最高段位
    let best_raw = db.global_get(SECTION, &format!("best:{}", user_id));
    let best_level = best_raw.parse::<i32>().unwrap_or(1);
    let best_league = LEAGUES.iter().find(|t| t.level == best_level).unwrap_or(&LEAGUES[0]);

    // 段位变更历史
    let history_raw = db.global_get(SECTION, &format!("history:{}", user_id));
    let history = if !history_raw.is_empty() && history_raw != "[NULL]" {
        history_raw.split('|').rev().take(5).collect::<Vec<_>>().join("\n  ")
    } else {
        "暂无记录".to_string()
    };

    // 显示所有段位
    let mut tiers = String::new();
    for tier in LEAGUES {
        let current = if tier.level == league.level { " ← 当前" } else { "" };
        tiers.push_str(&format!(
            "\n  {}{} ({}+分) HP+{} 物攻+{}{}",
            tier.emoji, tier.name, tier.min_points, tier.bonus.0, tier.bonus.1, current,
        ));
    }

    format!(
        "{}\n\
         ═══ 天梯详情 ═══\n\
         当前段位: {}{}\n\
         当前积分: {}\n\
         历史最高: {}{}\n\
         \n\
         📊 段位体系:{}\n\
         \n\
         📜 段位变更记录:\n  {}",
        prefix, league.emoji, league.name, points, best_league.emoji, best_league.name, tiers, history,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_league_tiers_count() {
        assert_eq!(LEAGUES.len(), 7);
    }

    #[test]
    fn test_league_tiers_sorted() {
        for i in 1..LEAGUES.len() {
            assert!(LEAGUES[i].min_points > LEAGUES[i - 1].min_points);
        }
    }

    #[test]
    fn test_league_level_unique() {
        let mut levels: Vec<i32> = LEAGUES.iter().map(|t| t.level).collect();
        levels.sort();
        levels.dedup();
        assert_eq!(levels.len(), LEAGUES.len());
    }

    #[test]
    fn test_point_sources_count() {
        assert_eq!(POINT_SOURCES.len(), 8);
    }

    #[test]
    fn test_point_sources_positive() {
        for src in POINT_SOURCES {
            assert!(src.points > 0);
            assert!(src.daily_limit >= src.points);
        }
    }

    #[test]
    fn test_get_league_bronze() {
        let league = get_league(0);
        assert_eq!(league.level, 1);
        assert_eq!(league.name, "青铜");
    }

    #[test]
    fn test_get_league_silver() {
        let league = get_league(500);
        assert_eq!(league.level, 2);
        assert_eq!(league.name, "白银");
    }

    #[test]
    fn test_get_league_grandmaster() {
        let league = get_league(15000);
        assert_eq!(league.level, 7);
        assert_eq!(league.name, "宗师");
    }

    #[test]
    fn test_get_league_boundary() {
        // 边界值测试
        assert_eq!(get_league(499).name, "青铜");
        assert_eq!(get_league(500).name, "白银");
        assert_eq!(get_league(1499).name, "白银");
        assert_eq!(get_league(1500).name, "黄金");
        assert_eq!(get_league(2999).name, "黄金");
        assert_eq!(get_league(3000).name, "铂金");
        assert_eq!(get_league(4999).name, "铂金");
        assert_eq!(get_league(5000).name, "钻石");
        assert_eq!(get_league(7999).name, "钻石");
        assert_eq!(get_league(8000).name, "大师");
        assert_eq!(get_league(11999).name, "大师");
        assert_eq!(get_league(12000).name, "宗师");
    }

    #[test]
    fn test_bonus_escalation() {
        for i in 1..LEAGUES.len() {
            assert!(LEAGUES[i].bonus.0 >= LEAGUES[i - 1].bonus.0);
        }
    }

    #[test]
    fn test_promote_reward_escalation() {
        for i in 2..LEAGUES.len() {
            assert!(LEAGUES[i].promote_reward.0 >= LEAGUES[i - 1].promote_reward.0);
        }
    }

    #[test]
    fn test_daily_point_cap() {
        assert!(DAILY_POINT_CAP > 0);
        // 所有来源的每日上限之和应大于总上限
        let total_source_limits: i32 = POINT_SOURCES.iter().map(|s| s.daily_limit).sum();
        assert!(total_source_limits > DAILY_POINT_CAP);
    }

    #[test]
    fn test_chrono_today_format() {
        let today = chrono_today();
        assert_eq!(today.len(), 8);
        // 应该是有效数字
        assert!(today.parse::<u32>().is_ok());
    }

    #[test]
    fn test_league_emoji_unique() {
        let mut emojis: Vec<&str> = LEAGUES.iter().map(|t| t.emoji).collect();
        emojis.sort();
        emojis.dedup();
        assert_eq!(emojis.len(), LEAGUES.len());
    }

    #[test]
    fn test_point_source_ids_unique() {
        let mut ids: Vec<&str> = POINT_SOURCES.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), POINT_SOURCES.len());
    }

    #[test]
    fn test_bit_mask_no_overlap() {
        for i in 0..LEAGUES.len() {
            for j in 0..LEAGUES.len() {
                if i != j {
                    let bi = 1i64 << (LEAGUES[i].level - 1);
                    let bj = 1i64 << (LEAGUES[j].level - 1);
                    assert_ne!(bi, bj);
                }
            }
        }
    }
}

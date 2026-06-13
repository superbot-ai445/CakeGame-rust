/// CakeGame 游戏日历 & 活动预告系统
///
/// 功能:
/// - 查看日历: 显示每日/每周/每月重置时间表
/// - 今日任务: 今天需要做的事项清单
/// - 活动预告: 即将到来的重置和活动
/// - 重置倒计时: 各系统的重置倒计时
/// - 赛季日历: 当前赛季时间和进度
///
/// 数据存储: Global 表 SECTION='game_calendar' / 'season'
/// 指令: 查看日历, 今日任务, 活动预告, 重置时间, 赛季日历
use crate::db::Database;

/// 日历事件定义
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub name: &'static str,
    pub emoji: &'static str,
    pub frequency: EventFrequency,
    #[allow(dead_code)]
    pub reset_hour: u8, // 重置时间 (24h, UTC+8)
    #[allow(dead_code)]
    pub reset_day: u8, // 每周几重置 (1=周一, 7=周日) 或每月几号
    pub description: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventFrequency {
    Daily,
    Weekly,
    #[allow(dead_code)]
    Monthly,
    Seasonal,
    Permanent,
}

/// 所有日历事件
fn calendar_events() -> Vec<CalendarEvent> {
    vec![
        CalendarEvent {
            name: "每日签到",
            emoji: "📅",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天0点重置签到，连续签到可获得里程碑奖励",
        },
        CalendarEvent {
            name: "每日任务",
            emoji: "📋",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天0点刷新3个日常任务，完成后领取奖励",
        },
        CalendarEvent {
            name: "每日挑战",
            emoji: "⚔️",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天0点刷新挑战目标，击败指定怪物获取奖励",
        },
        CalendarEvent {
            name: "活跃积分",
            emoji: "🎯",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天完成活跃任务积累积分，兑换稀有道具",
        },
        CalendarEvent {
            name: "在线奖励",
            emoji: "🎁",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天在线满一定时间可领取阶段性奖励",
        },
        CalendarEvent {
            name: "VIP签到",
            emoji: "👑",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "VIP用户每日额外签到奖励",
        },
        CalendarEvent {
            name: "每日运势",
            emoji: "🔮",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天抽签获取运势加成，影响当日收益",
        },
        CalendarEvent {
            name: "限时折扣",
            emoji: "🏷️",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天随机3件商品打折出售",
        },
        CalendarEvent {
            name: "钓鱼次数",
            emoji: "🎣",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天钓鱼次数有限，0点重置",
        },
        CalendarEvent {
            name: "幸运转盘",
            emoji: "🎰",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天免费转动1次幸运转盘（VIP额外1次）",
        },
        CalendarEvent {
            name: "NPC对话次数",
            emoji: "💬",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天与NPC对话可提升好感度（每日5次）",
        },
        CalendarEvent {
            name: "公会每日签到",
            emoji: "📝",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "公会成员每日签到贡献公会经验",
        },
        CalendarEvent {
            name: "公会每日委托",
            emoji: "📜",
            frequency: EventFrequency::Daily,
            reset_hour: 0,
            reset_day: 0,
            description: "每天刷新公会委托任务，完成后获取公会贡献",
        },
        CalendarEvent {
            name: "深渊重置",
            emoji: "🌀",
            frequency: EventFrequency::Weekly,
            reset_hour: 0,
            reset_day: 1,
            description: "每周一0点重置无尽深渊进度",
        },
        CalendarEvent {
            name: "公会试炼",
            emoji: "🏛️",
            frequency: EventFrequency::Weekly,
            reset_hour: 0,
            reset_day: 1,
            description: "每周一重置公会试炼BOSS",
        },
        CalendarEvent {
            name: "竞技赛季",
            emoji: "🏆",
            frequency: EventFrequency::Seasonal,
            reset_hour: 0,
            reset_day: 1,
            description: "每30天为一个赛季，赛季结束发放排名奖励",
        },
        CalendarEvent {
            name: "修炼周期",
            emoji: "🧘",
            frequency: EventFrequency::Permanent,
            reset_hour: 0,
            reset_day: 0,
            description: "挂机修炼持续进行，每周期判定突破",
        },
    ]
}

/// 一天中的时间段描述
fn time_of_day() -> (&'static str, &'static str) {
    let hour = chrono_hour();
    match hour {
        0..=5 => ("🌙 深夜", "夜深了，注意休息"),
        6..=8 => ("🌅 清晨", "新的一天开始了"),
        9..=11 => ("☀️ 上午", "精力充沛的时段"),
        12..=13 => ("🌞 中午", "午间时光"),
        14..=17 => ("🌤️ 下午", "下午茶时光"),
        18..=20 => ("🌇 傍晚", "傍晚时分"),
        21..=23 => ("🌙 夜晚", "夜间游戏时光"),
        _ => ("🕐 未知", ""),
    }
}

/// 获取当前小时 (从 Global 表读取或使用系统时间)
fn chrono_hour() -> u8 {
    // 尝试读取系统时钟
    use std::time::{SystemTime, UNIX_EPOCH};
    let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 12;
    };
    // UTC+8 (北京时间)
    let secs = dur.as_secs() + 8 * 3600;
    let hour = (secs % 86400) / 3600;
    hour as u8
}

/// 获取当前星期几 (1=周一, 7=周日)
fn current_weekday() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 1;
    };
    // 1970-01-01 是周四 (4)
    let days = dur.as_secs() / 86400;
    let weekday = ((days + 3) % 7) + 1; // 1=周一 .. 7=周日
    weekday as u8
}

/// 检查今天的活跃任务完成状态
fn check_daily_tasks(db: &Database, user_id: &str) -> Vec<(&'static str, bool)> {
    let mut tasks = Vec::new();

    let sign = db.read_user_data(user_id, "sign_in_date");
    tasks.push(("📅 每日签到", !sign.is_empty()));

    let training = db.read_user_data(user_id, "training_active");
    tasks.push(("🧘 修炼状态", training == "1"));

    let gcheck = db.read_user_data(user_id, "guild_checkin_date");
    tasks.push(("📝 公会签到", !gcheck.is_empty()));

    let dailies = db.read_user_data(user_id, "daily_quests_claimed");
    tasks.push(("📋 每日任务", dailies == "1"));

    let challenge = db.read_user_data(user_id, "daily_challenge_claimed");
    tasks.push(("⚔️ 每日挑战", challenge == "1"));

    let fortune = db.read_user_data(user_id, "daily_fortune_date");
    tasks.push(("🔮 今日运势", !fortune.is_empty()));

    let is_vip = !db.read_user_data(user_id, "vip_expiry").is_empty();
    if is_vip {
        let vip_sign = db.read_user_data(user_id, "vip_checkin_date");
        tasks.push(("👑 VIP签到", !vip_sign.is_empty()));
    }

    let wheel = db.read_user_data(user_id, "lucky_wheel_date");
    tasks.push(("🎰 幸运转盘", !wheel.is_empty()));

    tasks
}

/// 计算距下次每日重置的时间描述
fn daily_reset_countdown() -> String {
    let current_hour = chrono_hour();
    if current_hour == 0 {
        "刚刚重置".to_string()
    } else {
        format!("{}小时后重置", 24 - current_hour)
    }
}

/// 获取赛季剩余天数
fn get_season_remaining_days(db: &Database) -> i32 {
    let elapsed: i32 = db.global_get("season", "season_day").parse().unwrap_or(0);
    30_i32.saturating_sub(elapsed)
}

// ==================== 指令实现 ====================

/// 查看日历 — 显示完整游戏日历
pub fn cmd_view_calendar(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let (tod_emoji, tod_desc) = time_of_day();

    let mut out = format!("{}\n═══ 📅 游戏日历 ═══\n\n{} {}\n", prefix, tod_emoji, tod_desc);

    let events = calendar_events();

    // 每日重置
    out.push_str("\n━━ 🔄 每日重置 (0:00) ━━\n");
    for e in &events {
        if e.frequency == EventFrequency::Daily {
            out.push_str(&format!("  {} {}\n", e.emoji, e.name));
        }
    }
    out.push_str(&format!("  ⏰ {}\n", daily_reset_countdown()));

    // 每周重置
    out.push_str("\n━━ 📆 每周重置 (周一 0:00) ━━\n");
    for e in &events {
        if e.frequency == EventFrequency::Weekly {
            out.push_str(&format!("  {} {} — {}\n", e.emoji, e.name, e.description));
        }
    }

    // 赛季
    out.push_str("\n━━ 🏆 赛季系统 ━━\n");
    for e in &events {
        if e.frequency == EventFrequency::Seasonal {
            out.push_str(&format!("  {} {} — {}\n", e.emoji, e.name, e.description));
        }
    }

    // 永久系统
    out.push_str("\n━━ ⚡ 永久系统 ━━\n");
    for e in &events {
        if e.frequency == EventFrequency::Permanent {
            out.push_str(&format!("  {} {} — {}\n", e.emoji, e.name, e.description));
        }
    }

    // 全服活动检查
    let exp_mul = crate::world_event::get_exp_multiplier(db);
    let gold_mul = crate::world_event::get_gold_multiplier(db);
    if exp_mul > 1.0 || gold_mul > 1.0 {
        out.push_str("\n━━ ⚡ 当前全服活动 ━━\n");
        if exp_mul > 1.0 {
            out.push_str(&format!("  📈 经验加成: x{:.1}\n", exp_mul));
        }
        if gold_mul > 1.0 {
            out.push_str(&format!("  💰 金币加成: x{:.1}\n", gold_mul));
        }
    }

    out.push_str("\n💡 发送「今日任务」查看今日待办事项\n");
    out.push_str("💡 发送「活动预告」查看即将到来的活动\n");
    out.push_str("💡 发送「重置时间」查看各系统重置倒计时\n");
    out
}

/// 今日任务 — 检查今天的任务完成情况
pub fn cmd_today_tasks(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看今日任务！", prefix);
    }

    let tasks = check_daily_tasks(db, user_id);
    let completed = tasks.iter().filter(|(_, done)| *done).count();
    let total = tasks.len();

    let mut out = format!("{}\n═══ 📋 今日任务 ({}/{}) ═══\n", prefix, completed, total);

    // 进度条
    let pct = (completed * 100).checked_div(total).unwrap_or(0);
    let filled = pct / 5;
    let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
    out.push_str(&format!("  [{}] {}%\n\n", bar, pct));

    for (name, done) in &tasks {
        let check = if *done { "✅" } else { "⬜" };
        out.push_str(&format!("  {} {}\n", check, name));
    }

    if completed == total {
        out.push_str("\n🎉 今日任务全部完成！明天记得继续哦~\n");
    } else {
        out.push_str(&format!("\n📌 还有 {} 项任务待完成\n", total - completed));
    }

    out
}

/// 活动预告 — 显示即将到来的活动和重置
pub fn cmd_event_preview(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ 🔮 活动预告 ═══\n\n", prefix);

    // 当前全服活动
    let exp_mul = crate::world_event::get_exp_multiplier(db);
    let gold_mul = crate::world_event::get_gold_multiplier(db);
    let drop_mul = crate::world_event::get_drop_multiplier(db);
    let invasion = crate::world_event::is_monster_invasion_active(db);
    let blessing = crate::world_event::is_server_blessing_active(db);

    if exp_mul > 1.0 || gold_mul > 1.0 || drop_mul > 1.0 || invasion || blessing {
        out.push_str("━━ ⚡ 正在进行的全服活动 ━━\n");
        if exp_mul > 1.0 {
            out.push_str(&format!("  📈 经验加成: x{:.1}\n", exp_mul));
        }
        if gold_mul > 1.0 {
            out.push_str(&format!("  💰 金币加成: x{:.1}\n", gold_mul));
        }
        if drop_mul > 1.0 {
            out.push_str(&format!("  🎁 掉落加成: x{:.1}\n", drop_mul));
        }
        if invasion {
            out.push_str("  👹 怪物入侵: 进行中\n");
        }
        if blessing {
            out.push_str("  🌟 全服福利: 进行中\n");
        }
        out.push('\n');
    }

    // 即将到来的重置
    out.push_str("━━ ⏰ 即将到来的重置 ━━\n");
    let current_hour = chrono_hour();
    let daily_hours = if current_hour == 0 { 24 } else { 24 - current_hour };
    out.push_str(&format!("  📅 每日重置: {}小时后\n", daily_hours));

    let weekday = current_weekday();
    let days_to_monday = if weekday == 1 { 7 } else { 8 - weekday };
    out.push_str(&format!("  📆 每周重置: {}天后 (周一)\n", days_to_monday));

    let season_days = get_season_remaining_days(db);
    if season_days > 0 {
        out.push_str(&format!("  🏆 赛季结束: {}天后\n", season_days));
    }

    // 今日建议
    out.push_str("\n━━ 💡 今日建议 ━━\n");

    let tasks = check_daily_tasks(db, user_id);
    let uncompleted: Vec<&str> = tasks.iter().filter(|(_, done)| !done).map(|(name, _)| *name).collect();

    if uncompleted.is_empty() {
        out.push_str("  ✅ 今日任务已全部完成！\n");
        out.push_str("  💡 可以挑战副本、竞技场或探索新地图\n");
    } else {
        out.push_str(&format!("  📌 还有 {} 项任务未完成:\n", uncompleted.len()));
        for t in uncompleted.iter().take(3) {
            out.push_str(&format!("     → {}\n", t));
        }
    }

    out
}

/// 重置时间 — 显示各系统重置倒计时
pub fn cmd_reset_timers(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ ⏰ 重置倒计时 ═══\n\n", prefix);

    let events = calendar_events();

    // 每日重置
    let daily_hours = {
        let h = chrono_hour();
        if h == 0 {
            24
        } else {
            24 - h
        }
    };
    out.push_str("━━ 📅 每日重置 ━━\n");
    out.push_str(&format!("  ⏰ {} 小时后重置\n", daily_hours));
    for e in &events {
        if e.frequency == EventFrequency::Daily {
            out.push_str(&format!("    {} {}\n", e.emoji, e.name));
        }
    }

    // 每周重置
    let weekday = current_weekday();
    let days_to_monday = if weekday == 1 { 7 } else { 8 - weekday };
    out.push_str("\n━━ 📆 每周重置 ━━\n");
    out.push_str(&format!("  ⏰ {} 天后重置 (周一)\n", days_to_monday));
    for e in &events {
        if e.frequency == EventFrequency::Weekly {
            out.push_str(&format!("    {} {} — {}\n", e.emoji, e.name, e.description));
        }
    }

    // 赛季重置
    let season_days = get_season_remaining_days(db);
    out.push_str("\n━━ 🏆 赛季重置 ━━\n");
    if season_days > 0 {
        out.push_str(&format!("  ⏰ {} 天后赛季结束\n", season_days));
    } else {
        out.push_str("  ⏰ 赛季已结束或信息加载中\n");
    }

    out
}

/// 赛季日历 — 显示当前赛季信息和进度
pub fn cmd_season_calendar(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ 🏆 赛季日历 ═══\n\n", prefix);

    let season_num: i32 = db.global_get("season", "current_season").parse().unwrap_or(1);
    let season_days = get_season_remaining_days(db);
    let season_elapsed = 30_i32.saturating_sub(season_days);

    out.push_str(&format!("  📅 第 {} 赛季\n", season_num));
    out.push_str(&format!("  ⏰ 赛季天数: {}/30 天\n", season_elapsed));

    let pct = if season_elapsed > 0 {
        season_elapsed * 100 / 30
    } else {
        0
    };
    let filled = pct as usize / 5;
    let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
    out.push_str(&format!("  [{}] {}%\n\n", bar, pct));

    // 玩家赛季信息
    if db.user_exists(user_id) {
        let integral: i32 = db.read_user_data(user_id, "match_integral").parse().unwrap_or(0);
        let wins: i32 = db.read_user_data(user_id, "match_wins").parse().unwrap_or(0);
        let losses: i32 = db.read_user_data(user_id, "match_losses").parse().unwrap_or(0);

        out.push_str("━━ 你的赛季数据 ━━\n");
        out.push_str(&format!("  📊 匹配积分: {}\n", integral));
        out.push_str(&format!("  ⚔️ 战绩: {}胜 {}负\n", wins, losses));
        if let Some(win_rate) = (wins * 100_i32).checked_div(wins + losses) {
            out.push_str(&format!("  📈 胜率: {}%\n", win_rate));
        }
    }

    out.push_str("\n━━ 赛季奖励 ━━\n");
    out.push_str("  🥉 青铜以上: 赛季结束时发放金币奖励\n");
    out.push_str("  🥈 白银以上: 额外钻石奖励\n");
    out.push_str("  🥇 黄金以上: 稀有称号 + 材料\n");
    out.push_str("  💎 钻石以上: 传说装备 + 专属时装\n");

    out.push_str("\n💡 发送「匹配」开始竞技提升段位\n");
    out.push_str("💡 发送「赛季排行」查看全服排名\n");
    out
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_events_count() {
        let events = calendar_events();
        assert!(events.len() >= 15, "应至少有15个日历事件");
    }

    #[test]
    fn test_daily_events_exist() {
        let events = calendar_events();
        let daily: Vec<_> = events.iter().filter(|e| e.frequency == EventFrequency::Daily).collect();
        assert!(daily.len() >= 8, "应至少有8个每日事件, got {}", daily.len());
    }

    #[test]
    fn test_weekly_events_exist() {
        let events = calendar_events();
        let weekly: Vec<_> = events
            .iter()
            .filter(|e| e.frequency == EventFrequency::Weekly)
            .collect();
        assert!(weekly.len() >= 1, "应至少有1个每周事件");
    }

    #[test]
    fn test_seasonal_events_exist() {
        let events = calendar_events();
        let seasonal: Vec<_> = events
            .iter()
            .filter(|e| e.frequency == EventFrequency::Seasonal)
            .collect();
        assert!(seasonal.len() >= 1, "应至少有1个赛季事件");
    }

    #[test]
    fn test_daily_reset_countdown_format() {
        let cd = daily_reset_countdown();
        assert!(cd.contains("重置") || cd.contains("刚刚"), "倒计时应包含'重置'或'刚刚'");
    }

    #[test]
    fn test_time_of_day() {
        let (emoji, desc) = time_of_day();
        assert!(!emoji.is_empty(), "时间段应有emoji");
        assert!(!desc.is_empty(), "时间段应有描述");
    }

    #[test]
    fn test_event_frequencies() {
        let events = calendar_events();
        for e in &events {
            if e.frequency == EventFrequency::Daily {
                assert_eq!(e.reset_hour, 0, "每日事件{}应在0点重置", e.name);
            }
            if e.frequency == EventFrequency::Weekly {
                assert!(e.reset_day >= 1 && e.reset_day <= 7, "每周事件{}应指定星期几", e.name);
            }
        }
    }

    #[test]
    fn test_event_names_unique() {
        let events = calendar_events();
        let mut names: Vec<&str> = events.iter().map(|e| e.name).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len, "事件名称应唯一");
    }

    #[test]
    fn test_event_descriptions_not_empty() {
        let events = calendar_events();
        for e in &events {
            assert!(!e.description.is_empty(), "事件{}应有描述", e.name);
        }
    }

    #[test]
    fn test_event_emojis_not_empty() {
        let events = calendar_events();
        for e in &events {
            assert!(!e.emoji.is_empty(), "事件{}应有emoji", e.name);
        }
    }

    #[test]
    fn test_chrono_hour_in_range() {
        let h = chrono_hour();
        assert!(h <= 23, "小时应在0-23之间, got {}", h);
    }

    #[test]
    fn test_current_weekday_in_range() {
        let d = current_weekday();
        assert!((1..=7).contains(&d), "星期应在1-7之间, got {}", d);
    }
}

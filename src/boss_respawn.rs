/// CakeGame BOSS刷新计时器系统
/// 追踪所有BOSS击杀时间，计算重生倒计时，提供全服BOSS状态总览
///
/// 功能:
/// - BOSS状态: 查看所有BOSS当前状态(存活/冷却中/即将刷新)
/// - BOSS刷新: 查看指定BOSS的详细刷新倒计时
/// - BOSS击杀记录: 查看最近BOSS击杀记录
///
/// 数据存储: Global表 SECTION='boss_respawn'
///   key: boss_{name}_killed  = 上次击杀时间戳
///   key: boss_{name}_killer  = 击杀者
///   key: boss_{name}_count   = 累计击杀次数
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::{DateTime, Datelike, Local};

/// 世界BOSS刷新间隔(小时)
const WORLD_BOSS_RESPAWN_HOURS: i64 = 24;

/// 野外BOSS刷新间隔(小时)
const FIELD_BOSS_RESPAWN_HOURS: i64 = 4;

/// 公会试炼BOSS刷新间隔(小时)
const GUILD_TRIAL_RESPAWN_HOURS: i64 = 168; // 7天

/// 深渊BOSS刷新间隔(小时)
const ABYSS_RESPAWN_HOURS: i64 = 2;

/// BOSS类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum BossCategory {
    /// 世界BOSS (每日刷新)
    World,
    /// 野外BOSS (4小时刷新)
    Field,
    /// 公会试炼 (每周刷新)
    GuildTrial,
    /// 深渊BOSS (2小时冷却)
    Abyss,
}

impl BossCategory {
    fn name(&self) -> &'static str {
        match self {
            BossCategory::World => "世界BOSS",
            BossCategory::Field => "野外BOSS",
            BossCategory::GuildTrial => "公会试炼",
            BossCategory::Abyss => "深渊BOSS",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            BossCategory::World => "🌍",
            BossCategory::Field => "⚔️",
            BossCategory::GuildTrial => "🏛️",
            BossCategory::Abyss => "🌀",
        }
    }

    pub fn respawn_hours(&self) -> i64 {
        match self {
            BossCategory::World => WORLD_BOSS_RESPAWN_HOURS,
            BossCategory::Field => FIELD_BOSS_RESPAWN_HOURS,
            BossCategory::GuildTrial => GUILD_TRIAL_RESPAWN_HOURS,
            BossCategory::Abyss => ABYSS_RESPAWN_HOURS,
        }
    }
}

/// BOSS状态信息
#[allow(dead_code)]
pub struct BossStatus {
    pub name: String,
    pub category: BossCategory,
    pub last_killed: Option<DateTime<Local>>,
    pub killer: String,
    pub kill_count: i64,
}

/// 从数据库获取所有野外BOSS名
fn get_field_boss_names(db: &Database) -> Vec<String> {
    db.query_rows("SELECT Name FROM ext_sgmonster_info", &[], |row| {
        Ok(row.get::<_, String>(0).unwrap_or_default())
    })
}

/// 格式化剩余时间
pub fn format_time_remaining(secs: i64) -> String {
    if secs <= 0 {
        return "✅ 已刷新".to_string();
    }
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 24 {
        let days = hours / 24;
        let rem_hours = hours % 24;
        format!("{}天{}小时", days, rem_hours)
    } else if hours > 0 {
        format!("{}小时{}分", hours, minutes)
    } else {
        format!("{}分钟", minutes)
    }
}

/// 状态图标
fn status_icon(remaining_secs: i64) -> &'static str {
    if remaining_secs <= 0 {
        "🟢" // 已刷新
    } else if remaining_secs <= 600 {
        "🟡" // 即将刷新(10分钟内)
    } else {
        "🔴" // 冷却中
    }
}

/// 记录BOSS被击杀
#[allow(dead_code)]
pub fn record_boss_kill(db: &Database, boss_name: &str, killer_id: &str) {
    let now = Local::now().timestamp().to_string();
    let killer_name = db.read_basic(killer_id, ITEM_NAME);
    let key_prefix = format!("boss_{}", boss_name);

    db.global_set("boss_respawn", &format!("{}_killed", key_prefix), &now);
    db.global_set("boss_respawn", &format!("{}_killer", key_prefix), &killer_name);

    // 累计击杀次数
    let count_key = format!("{}_count", key_prefix);
    let current: i64 = db.global_get("boss_respawn", &count_key).parse().unwrap_or(0);
    db.global_set("boss_respawn", &count_key, &(current + 1).to_string());
}

/// 获取BOSS状态
pub fn get_boss_status(db: &Database, name: &str, category: BossCategory) -> BossStatus {
    let key_prefix = format!("boss_{}", name);
    let killed_ts: i64 = db
        .global_get("boss_respawn", &format!("{}_killed", key_prefix))
        .parse()
        .unwrap_or(0);
    let killer = db.global_get("boss_respawn", &format!("{}_killer", key_prefix));
    let kill_count: i64 = db
        .global_get("boss_respawn", &format!("{}_count", key_prefix))
        .parse()
        .unwrap_or(0);

    let last_killed = if killed_ts > 0 {
        DateTime::from_timestamp(killed_ts, 0).map(|dt| dt.with_timezone(&Local))
    } else {
        None
    };

    BossStatus {
        name: name.to_string(),
        category,
        last_killed,
        killer,
        kill_count,
    }
}

/// 计算BOSS剩余冷却时间(秒)
pub fn calc_remaining_secs(status: &BossStatus) -> i64 {
    match status.last_killed {
        Some(killed_time) => {
            let respawn_secs = status.category.respawn_hours() * 3600;
            let elapsed = (Local::now() - killed_time).num_seconds();
            respawn_secs - elapsed
        }
        None => 0, // 从未击杀，BOSS可用
    }
}

/// 世界BOSS固定名称
const WORLD_BOSS_NAMES: &[&str] = &["暗影巨龙", "冰霜女王", "熔岩魔神", "虚空领主", "死亡骑士"];

/// 公会试炼BOSS名称
const TRIAL_BOSS_NAMES: &[&str] = &["哥布林将军", "暗影刺客", "深渊巨蟒", "魔化圣骑士", "终焉审判者"];

/// ═══ BOSS状态 总览 ═══
pub fn cmd_boss_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let now = Local::now();

    let mut out = format!("{}\n═══ 🏰 BOSS刷新状态 ═══", prefix);
    out.push_str(&format!("\n📅 {}", now.format("%Y-%m-%d %H:%M")));

    // 世界BOSS
    out.push_str(&format!(
        "\n\n{} ══ {} ══",
        BossCategory::World.emoji(),
        BossCategory::World.name()
    ));
    for name in WORLD_BOSS_NAMES {
        let status = get_boss_status(db, name, BossCategory::World);
        let remaining = calc_remaining_secs(&status);
        let icon = status_icon(remaining);
        let time_str = format_time_remaining(remaining);
        if remaining <= 0 {
            out.push_str(&format!("\n  {} {} — 🟢 可挑战", icon, name));
        } else {
            let killer_info = if status.killer.is_empty() {
                "上次: 无记录".to_string()
            } else {
                format!("击杀者: {}", status.killer)
            };
            out.push_str(&format!("  \n  {} {} — {} ({})", icon, name, time_str, killer_info));
        }
    }

    // 野外BOSS
    let field_bosses = get_field_boss_names(db);
    if !field_bosses.is_empty() {
        out.push_str(&format!(
            "\n\n{} ══ {} ══",
            BossCategory::Field.emoji(),
            BossCategory::Field.name()
        ));
        for name in &field_bosses {
            let status = get_boss_status(db, name, BossCategory::Field);
            let remaining = calc_remaining_secs(&status);
            let icon = status_icon(remaining);
            let time_str = format_time_remaining(remaining);
            if remaining <= 0 {
                out.push_str(&format!("\n  {} {} — 🟢 可挑战", icon, name));
            } else {
                out.push_str(&format!("  \n  {} {} — {}", icon, name, time_str));
            }
        }
    }

    // 公会试炼
    out.push_str(&format!(
        "\n\n{} ══ {} ══",
        BossCategory::GuildTrial.emoji(),
        BossCategory::GuildTrial.name()
    ));
    for name in TRIAL_BOSS_NAMES {
        let status = get_boss_status(db, name, BossCategory::GuildTrial);
        let remaining = calc_remaining_secs(&status);
        let icon = status_icon(remaining);
        let time_str = format_time_remaining(remaining);
        if remaining <= 0 {
            out.push_str(&format!("\n  {} {} — 🟢 可挑战", icon, name));
        } else {
            out.push_str(&format!("  \n  {} {} — {}", icon, name, time_str));
        }
    }

    // 统计
    let total_kills: i64 = WORLD_BOSS_NAMES
        .iter()
        .map(|name| {
            let key = format!("boss_{}_count", name);
            db.global_get("boss_respawn", &key).parse::<i64>().unwrap_or(0)
        })
        .sum();

    out.push_str(&format!("\n\n📊 世界BOSS累计击杀: {}次", total_kills));
    out.push_str("\n💡 状态: 🟢可挑战 🟡即将刷新 🔴冷却中");
    out.push_str(&format!(
        "\n⏱️ 世界BOSS{}h 野外{}h 试炼{}天",
        WORLD_BOSS_RESPAWN_HOURS,
        FIELD_BOSS_RESPAWN_HOURS,
        GUILD_TRIAL_RESPAWN_HOURS / 24
    ));

    out
}

/// ═══ BOSS刷新 详细 ═══
pub fn cmd_boss_respawn(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        return cmd_boss_status(db, user_id, "", _msg_type, _group);
    }

    // 搜索指定BOSS
    let mut all_bosses: Vec<(String, BossCategory)> = Vec::new();
    for name in WORLD_BOSS_NAMES {
        all_bosses.push((name.to_string(), BossCategory::World));
    }
    let field = get_field_boss_names(db);
    for name in field {
        all_bosses.push((name, BossCategory::Field));
    }
    for name in TRIAL_BOSS_NAMES {
        all_bosses.push((name.to_string(), BossCategory::GuildTrial));
    }

    // 精确+模糊匹配
    let matched: Vec<&(String, BossCategory)> = all_bosses
        .iter()
        .filter(|(name, _)| name == args || name.contains(args))
        .collect();

    if matched.is_empty() {
        return format!("{}\n❌ 未找到BOSS「{}」\n💡 使用「BOSS状态」查看所有BOSS", prefix, args);
    }

    let (name, category) = matched[0];
    let status = get_boss_status(db, name, *category);
    let remaining = calc_remaining_secs(&status);

    let mut out = format!("{}\n═══ {} {} ═══", prefix, category.emoji(), name);
    out.push_str(&format!("\n📋 类型: {} ({})", category.name(), category.emoji()));

    if remaining <= 0 {
        out.push_str("\n🟢 状态: 已刷新 — 可以挑战！");
    } else {
        out.push_str("\n🔴 状态: 冷却中");
        out.push_str(&format!("\n⏱️ 剩余: {}", format_time_remaining(remaining)));
        if let Some(killed_time) = status.last_killed {
            out.push_str(&format!("\n📅 上次击杀: {}", killed_time.format("%m-%d %H:%M")));
        }
        if !status.killer.is_empty() {
            out.push_str(&format!("\n🗡️ 击杀者: {}", status.killer));
        }
    }

    out.push_str(&format!("\n📊 累计击杀: {}次", status.kill_count));
    out.push_str(&format!("\n⏱️ 刷新间隔: {}小时", category.respawn_hours()));

    // 预测下次刷新时间
    if remaining > 0 {
        let respawn_time = Local::now() + chrono::Duration::seconds(remaining);
        out.push_str(&format!(
            "\n📅 预计刷新: {} ({})",
            respawn_time.format("%m-%d %H:%M"),
            format_weekday(respawn_time.weekday().num_days_from_monday()),
        ));
    }

    out
}

/// 星期几
fn format_weekday(day: u32) -> &'static str {
    match day {
        0 => "周一",
        1 => "周二",
        2 => "周三",
        3 => "周四",
        4 => "周五",
        5 => "周六",
        6 => "周日",
        _ => "?",
    }
}

/// ═══ BOSS击杀记录 ═══
pub fn cmd_boss_kill_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut all_bosses: Vec<(String, BossCategory)> = Vec::new();
    for name in WORLD_BOSS_NAMES {
        all_bosses.push((name.to_string(), BossCategory::World));
    }
    let field = get_field_boss_names(db);
    for name in field {
        all_bosses.push((name, BossCategory::Field));
    }

    let mut records: Vec<(String, BossCategory, i64, String, i64)> = Vec::new();
    for (name, cat) in &all_bosses {
        let key_prefix = format!("boss_{}", name);
        let killed_ts: i64 = db
            .global_get("boss_respawn", &format!("{}_killed", key_prefix))
            .parse()
            .unwrap_or(0);
        let killer = db.global_get("boss_respawn", &format!("{}_killer", key_prefix));
        let count: i64 = db
            .global_get("boss_respawn", &format!("{}_count", key_prefix))
            .parse()
            .unwrap_or(0);
        if killed_ts > 0 {
            records.push((name.clone(), *cat, killed_ts, killer, count));
        }
    }

    if records.is_empty() {
        return format!("{}\n📜 暂无BOSS击杀记录\n💡 击杀BOSS后会自动记录", prefix);
    }

    // 按时间倒序
    records.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut out = format!("{}\n═══ 📜 BOSS击杀记录 ═══", prefix);
    for (i, (name, cat, ts, killer, count)) in records.iter().take(10).enumerate() {
        let dt = DateTime::from_timestamp(*ts, 0)
            .map(|d| d.with_timezone(&Local))
            .unwrap_or_default();
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!(
            "\n{} {}{} [{}] — 击杀者: {} (累计{}次)",
            medal,
            cat.emoji(),
            name,
            dt.format("%m-%d %H:%M"),
            if killer.is_empty() { "未知" } else { killer },
            count,
        ));
    }

    out.push_str(&format!("\n\n📊 共{}条击杀记录", records.len()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_category_properties() {
        assert_eq!(BossCategory::World.name(), "世界BOSS");
        assert_eq!(BossCategory::Field.name(), "野外BOSS");
        assert_eq!(BossCategory::GuildTrial.name(), "公会试炼");
        assert_eq!(BossCategory::Abyss.name(), "深渊BOSS");

        assert_eq!(BossCategory::World.respawn_hours(), 24);
        assert_eq!(BossCategory::Field.respawn_hours(), 4);
        assert_eq!(BossCategory::GuildTrial.respawn_hours(), 168);
        assert_eq!(BossCategory::Abyss.respawn_hours(), 2);
    }

    #[test]
    fn test_format_time_remaining_zero() {
        assert_eq!(format_time_remaining(0), "✅ 已刷新");
        assert_eq!(format_time_remaining(-100), "✅ 已刷新");
    }

    #[test]
    fn test_format_time_remaining_minutes() {
        assert_eq!(format_time_remaining(300), "5分钟");
    }

    #[test]
    fn test_format_time_remaining_hours() {
        assert_eq!(format_time_remaining(7200), "2小时0分");
    }

    #[test]
    fn test_format_time_remaining_days() {
        assert_eq!(format_time_remaining(172800), "2天0小时");
    }

    #[test]
    fn test_format_time_remaining_mixed() {
        assert_eq!(format_time_remaining(3661), "1小时1分");
    }

    #[test]
    fn test_status_icon() {
        assert_eq!(status_icon(0), "🟢");
        assert_eq!(status_icon(-1), "🟢");
        assert_eq!(status_icon(300), "🟡");
        assert_eq!(status_icon(600), "🟡");
        assert_eq!(status_icon(601), "🔴");
        assert_eq!(status_icon(3600), "🔴");
    }

    #[test]
    fn test_format_weekday() {
        assert_eq!(format_weekday(0), "周一");
        assert_eq!(format_weekday(6), "周日");
    }

    #[test]
    fn test_boss_category_emoji() {
        assert_eq!(BossCategory::World.emoji(), "🌍");
        assert_eq!(BossCategory::Field.emoji(), "⚔️");
        assert_eq!(BossCategory::GuildTrial.emoji(), "🏛️");
        assert_eq!(BossCategory::Abyss.emoji(), "🌀");
    }

    #[test]
    fn test_boss_respawn_constants() {
        assert!(WORLD_BOSS_RESPAWN_HOURS >= 12);
        assert!(WORLD_BOSS_RESPAWN_HOURS <= 48);
        assert!(FIELD_BOSS_RESPAWN_HOURS < WORLD_BOSS_RESPAWN_HOURS);
        assert!(GUILD_TRIAL_RESPAWN_HOURS > WORLD_BOSS_RESPAWN_HOURS);
        assert!(ABYSS_RESPAWN_HOURS < FIELD_BOSS_RESPAWN_HOURS);
    }
}

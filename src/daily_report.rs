/// CakeGame 玩家日报系统
/// 提供每日活动汇总，追踪玩家每日成长轨迹
///
/// 功能:
/// - 玩家日报: 当日战斗/经济/成长全方位统计
/// - 周报总结: 最近7天汇总趋势
/// - 成长里程碑: 达到特定成就自动记录
///
/// 数据存储: Global表 SECTION='daily_report'
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 获取今日日期字符串
fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 获取N天前日期字符串
fn days_ago_str(n: i64) -> String {
    (Local::now() - chrono::Duration::days(n))
        .format("%Y-%m-%d")
        .to_string()
}

/// 日报数据结构
#[derive(Default)]
struct DailyReport {
    battles: i64,
    wins: i64,
    losses: i64,
    gold_earned: i64,
    gold_spent: i64,
    exp_gained: i64,
    items_collected: i64,
    items_used: i64,
    monsters_killed: i64,
    players_defeated: i64,
    gathering_count: i64,
    crafting_count: i64,
    login_count: i64,
}

/// 从Global表读取日报数据
fn read_report(db: &Database, user_id: &str, date: &str) -> DailyReport {
    let section = format!("daily_report_{}_{}", user_id, date);
    let get = |key: &str| -> i64 { db.global_get(&section, key).parse::<i64>().unwrap_or(0) };
    DailyReport {
        battles: get("battles"),
        wins: get("wins"),
        losses: get("losses"),
        gold_earned: get("gold_earned"),
        gold_spent: get("gold_spent"),
        exp_gained: get("exp_gained"),
        items_collected: get("items_collected"),
        items_used: get("items_used"),
        monsters_killed: get("monsters_killed"),
        players_defeated: get("players_defeated"),
        gathering_count: get("gathering_count"),
        crafting_count: get("crafting_count"),
        login_count: get("login_count"),
    }
}

/// 记录日报事件 (供其他模块调用)
#[allow(dead_code)]
pub fn record_event(db: &Database, user_id: &str, event: &str, value: i64) {
    let date = today_str();
    let section = format!("daily_report_{}_{}", user_id, date);
    let key = event;
    let current: i64 = db.global_get(&section, key).parse().unwrap_or(0);
    db.global_set(&section, key, &(current + value).to_string());
}

/// 统计活跃天数
fn count_active_days(db: &Database, user_id: &str, days: i64) -> i64 {
    let mut count = 0i64;
    for i in 0..days {
        let date = days_ago_str(i);
        let section = format!("daily_report_{}_{}", user_id, date);
        if db.global_get(&section, "login_count").parse::<i64>().unwrap_or(0) > 0 {
            count += 1;
        }
    }
    count
}

/// 生成进度条 (10格)
fn progress_bar(current: i64, target: i64) -> String {
    let pct = if target > 0 {
        (current * 100 / target).min(100)
    } else {
        0
    };
    let filled = (pct / 10) as usize;
    let empty = 10 - filled;
    format!("{}{} {}%", "█".repeat(filled), "░".repeat(empty), pct)
}

/// 战斗评级
fn battle_rating(win_rate: f64) -> &'static str {
    if win_rate >= 80.0 {
        "S"
    } else if win_rate >= 60.0 {
        "A"
    } else if win_rate >= 40.0 {
        "B"
    } else if win_rate >= 20.0 {
        "C"
    } else {
        "D"
    }
}

/// 经济评级
fn economy_rating(earned: i64, spent: i64) -> &'static str {
    if earned == 0 && spent == 0 {
        return "—";
    }
    let ratio = if spent > 0 { earned as f64 / spent as f64 } else { 10.0 };
    if ratio >= 2.0 {
        "S"
    } else if ratio >= 1.5 {
        "A"
    } else if ratio >= 1.0 {
        "B"
    } else if ratio >= 0.5 {
        "C"
    } else {
        "D"
    }
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

/// cmd_daily_report: 玩家日报 — 查看今日活动统计
pub fn cmd_daily_report(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let _ = prefix;
    let date = if args.trim().is_empty() {
        today_str()
    } else {
        args.trim().to_string()
    };
    let report = read_report(db, user_id, &date);

    let win_rate = if report.battles > 0 {
        report.wins as f64 * 100.0 / report.battles as f64
    } else {
        0.0
    };

    let net_gold = report.gold_earned - report.gold_spent;
    let battle_level = battle_rating(win_rate);
    let economy_level = economy_rating(report.gold_earned, report.gold_spent);

    // 活跃度评估
    let total_activity = report.battles + report.gathering_count + report.crafting_count + report.login_count;
    let activity_level = if total_activity >= 50 {
        "🔥 极度活跃"
    } else if total_activity >= 20 {
        "⭐ 活跃"
    } else if total_activity >= 5 {
        "📝 普通"
    } else {
        "💤 休闲"
    };

    let mut out = String::new();
    out.push_str(&format!("📊 === {} 玩家日报 === 📊\n", date));
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    // 战斗概览
    out.push_str(&format!("⚔️ 【战斗概览】评级: {}\n", battle_level));
    out.push_str(&format!(
        "  战斗场次: {} (胜{} 负{})\n",
        report.battles, report.wins, report.losses
    ));
    out.push_str(&format!("  胜率: {:.1}%\n", win_rate));
    out.push_str(&format!("  击杀怪物: {}\n", report.monsters_killed));
    out.push_str(&format!("  击败玩家: {}\n", report.players_defeated));
    out.push_str(&format!("  胜率进度: {}\n\n", progress_bar(report.wins, 10)));

    // 经济概览
    out.push_str(&format!("💰 【经济概览】评级: {}\n", economy_level));
    out.push_str(&format!("  获得金币: {}\n", format_num(report.gold_earned)));
    out.push_str(&format!("  消费金币: {}\n", format_num(report.gold_spent)));
    out.push_str(&format!(
        "  净收入: {}{}\n",
        if net_gold >= 0 { "+" } else { "" },
        format_num(net_gold)
    ));
    out.push_str(&format!("  获得经验: {}\n\n", format_num(report.exp_gained)));

    // 物品活动
    out.push_str("🎒 【物品活动】\n");
    out.push_str(&format!("  获取物品: {}\n", report.items_collected));
    out.push_str(&format!("  使用物品: {}\n\n", report.items_used));

    // 生活技能
    out.push_str("🌿 【生活技能】\n");
    out.push_str(&format!("  采集次数: {}\n", report.gathering_count));
    out.push_str(&format!("  制作次数: {}\n\n", report.crafting_count));

    // 活跃总结
    out.push_str(&format!("📈 【活跃总结】{}\n", activity_level));
    out.push_str(&format!("  总活跃度: {} 次操作\n", total_activity));
    out.push_str(&format!("  登录次数: {}\n\n", report.login_count));

    // 每日目标
    let daily_goals = [
        ("战斗10场", report.battles, 10),
        ("击杀20怪", report.monsters_killed, 20),
        ("采集5次", report.gathering_count, 5),
        ("赚取5000金", report.gold_earned, 5000),
    ];
    out.push_str("🎯 【每日目标】\n");
    let mut completed = 0;
    for (name, current, target) in &daily_goals {
        let status = if *current >= *target {
            completed += 1;
            "✅"
        } else {
            "⬜"
        };
        out.push_str(&format!("  {} {}: {}/{}\n", status, name, current, target));
    }
    out.push_str(&format!("  完成: {}/{}\n", completed, daily_goals.len()));

    if completed == daily_goals.len() {
        out.push_str("\n🎉 恭喜！今日所有目标已完成！\n");
    }

    out
}

/// cmd_weekly_report: 周报总结 — 查看最近7天活动趋势
pub fn cmd_weekly_report(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let _ = prefix;

    let mut total = DailyReport::default();
    let mut active_days = 0i64;

    // 收集7天数据
    let mut daily_battles = Vec::new();
    let mut daily_gold = Vec::new();

    for i in 0..7 {
        let date = days_ago_str(i);
        let report = read_report(db, user_id, &date);
        total.battles += report.battles;
        total.wins += report.wins;
        total.losses += report.losses;
        total.gold_earned += report.gold_earned;
        total.gold_spent += report.gold_spent;
        total.exp_gained += report.exp_gained;
        total.items_collected += report.items_collected;
        total.monsters_killed += report.monsters_killed;
        total.gathering_count += report.gathering_count;
        total.crafting_count += report.crafting_count;
        if report.login_count > 0 {
            active_days += 1;
        }
        daily_battles.push(report.battles);
        daily_gold.push(report.gold_earned);
    }

    let win_rate = if total.battles > 0 {
        total.wins as f64 * 100.0 / total.battles as f64
    } else {
        0.0
    };

    let net_gold = total.gold_earned - total.gold_spent;
    let avg_battles = if active_days > 0 {
        total.battles / active_days
    } else {
        0
    };
    let avg_gold = if active_days > 0 {
        total.gold_earned / active_days
    } else {
        0
    };

    // 趋势分析
    let trend = if daily_battles.len() >= 2 {
        let recent: i64 = daily_battles[..3].iter().sum();
        let older: i64 = daily_battles[3..].iter().sum();
        if recent > older {
            "📈 上升趋势"
        } else if recent < older {
            "📉 下降趋势"
        } else {
            "➡️ 持平"
        }
    } else {
        "📊 数据不足"
    };

    let mut out = String::new();
    out.push_str("📋 === 最近7天周报总结 === 📋\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    out.push_str(&format!("📅 活跃天数: {}/7 天\n", active_days));
    out.push_str(&format!("📊 战斗趋势: {}\n\n", trend));

    out.push_str("⚔️ 【战斗汇总】\n");
    out.push_str(&format!("  总战斗: {} 场 (日均{}场)\n", total.battles, avg_battles));
    out.push_str(&format!(
        "  胜率: {:.1}% ({}胜 {}负)\n",
        win_rate, total.wins, total.losses
    ));
    out.push_str(&format!("  击杀怪物: {}\n\n", total.monsters_killed));

    out.push_str("💰 【经济汇总】\n");
    out.push_str(&format!(
        "  总收入: {} (日均{})\n",
        format_num(total.gold_earned),
        format_num(avg_gold)
    ));
    out.push_str(&format!("  总支出: {}\n", format_num(total.gold_spent)));
    out.push_str(&format!(
        "  净收入: {}{}\n",
        if net_gold >= 0 { "+" } else { "" },
        format_num(net_gold)
    ));
    out.push_str(&format!("  总经验: {}\n\n", format_num(total.exp_gained)));

    out.push_str("🎒 【活动汇总】\n");
    out.push_str(&format!("  物品获取: {}\n", total.items_collected));
    out.push_str(&format!("  采集次数: {}\n", total.gathering_count));
    out.push_str(&format!("  制作次数: {}\n\n", total.crafting_count));

    // 每日明细 (7天柱状图)
    out.push_str("📊 【每日活跃度】\n");
    for i in (0..7).rev() {
        let date = days_ago_str(i);
        let report = read_report(db, user_id, &date);
        let activity = report.battles + report.gathering_count + report.crafting_count;
        let bars = "▓".repeat((activity as usize).min(20));
        let day_label = if i == 0 {
            "今天"
        } else if i == 1 {
            "昨天"
        } else {
            &date[5..]
        }; // MM-DD
        out.push_str(&format!("  {:>4} | {:>3} 次 | {}\n", day_label, activity, bars));
    }

    // 总评
    let overall = if active_days >= 5 && total.battles >= 20 {
        "🌟 优秀玩家"
    } else if active_days >= 3 && total.battles >= 10 {
        "⭐ 活跃玩家"
    } else if active_days >= 1 {
        "📝 普通玩家"
    } else {
        "💤 休息中"
    };
    out.push_str(&format!("\n🏆 周总评: {}\n", overall));

    out
}

/// cmd_growth_milestone: 成长里程碑 — 查看已达成的成长目标
pub fn cmd_growth_milestone(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let _ = prefix;

    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);

    // 累计统计 (从所有日报聚合)
    let mut total_battles = 0i64;
    let mut total_kills = 0i64;
    let mut total_gold = 0i64;
    let mut total_gather = 0i64;
    let active_days = count_active_days(db, user_id, 365);

    for i in 0..365 {
        let date = days_ago_str(i);
        let r = read_report(db, user_id, &date);
        total_battles += r.battles;
        total_kills += r.monsters_killed;
        total_gold += r.gold_earned;
        total_gather += r.gathering_count;
    }

    // 里程碑定义
    let milestones = [
        ("初出茅庐", "达到5级", level >= 5, 0),
        ("小有成就", "达到10级", level >= 10, 0),
        ("身经百战", "达到20级", level >= 20, 0),
        ("一方霸主", "达到30级", level >= 30, 0),
        ("武林传说", "达到50级", level >= 50, 0),
        ("战斗新手", "完成10场战斗", total_battles >= 10, 0),
        ("战斗老手", "完成100场战斗", total_battles >= 100, 0),
        ("战斗狂人", "完成500场战斗", total_battles >= 500, 0),
        ("怪物克星", "击杀50只怪物", total_kills >= 50, 0),
        ("怪物猎人", "击杀500只怪物", total_kills >= 500, 0),
        ("初入宝山", "累计获得10000金币", total_gold >= 10000, 0),
        ("小富即安", "累计获得100000金币", total_gold >= 100000, 0),
        ("富甲一方", "累计获得1000000金币", total_gold >= 1000000, 0),
        ("勤劳蜜蜂", "采集50次", total_gather >= 50, 0),
        ("采集达人", "采集500次", total_gather >= 500, 0),
        ("常驻玩家", "累计活跃7天", active_days >= 7, 0),
        ("铁杆粉丝", "累计活跃30天", active_days >= 30, 0),
        ("忠实玩家", "累计活跃100天", active_days >= 100, 0),
    ];

    let achieved = milestones.iter().filter(|m| m.2).count();
    let total = milestones.len();

    let mut out = String::new();
    out.push_str("🏆 === 成长里程碑 === 🏆\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    out.push_str(&format!(
        "📊 完成进度: {}/{} ({:.0}%)\n",
        achieved,
        total,
        if total > 0 {
            achieved as f64 * 100.0 / total as f64
        } else {
            0.0
        }
    ));
    out.push_str(&format!("{}\n\n", progress_bar(achieved as i64, total as i64)));

    out.push_str("📋 【里程碑列表】\n");
    for (name, desc, done, _) in &milestones {
        let icon = if *done { "✅" } else { "⬜" };
        out.push_str(&format!("  {} {} — {}\n", icon, name, desc));
    }

    // 下一个目标
    if let Some(next) = milestones.iter().find(|m| !m.2) {
        out.push_str(&format!("\n🎯 下一个目标: {} — {}\n", next.0, next.1));
    }

    if achieved == total {
        out.push_str("\n🎊 恭喜！所有里程碑已达成！\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0, 100), "░░░░░░░░░░ 0%");
        assert_eq!(progress_bar(50, 100), "█████░░░░░ 50%");
        assert_eq!(progress_bar(100, 100), "██████████ 100%");
        assert_eq!(progress_bar(150, 100), "██████████ 100%");
        assert_eq!(progress_bar(0, 0), "░░░░░░░░░░ 0%");
    }

    #[test]
    fn test_battle_rating() {
        assert_eq!(battle_rating(90.0), "S");
        assert_eq!(battle_rating(70.0), "A");
        assert_eq!(battle_rating(50.0), "B");
        assert_eq!(battle_rating(30.0), "C");
        assert_eq!(battle_rating(10.0), "D");
    }

    #[test]
    fn test_economy_rating() {
        assert_eq!(economy_rating(0, 0), "—");
        assert_eq!(economy_rating(200, 100), "S");
        assert_eq!(economy_rating(150, 100), "A");
        assert_eq!(economy_rating(100, 100), "B");
        assert_eq!(economy_rating(50, 100), "C");
        assert_eq!(economy_rating(10, 100), "D");
        assert_eq!(economy_rating(100, 0), "S");
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234567), "1,234,567");
        assert_eq!(format_num(-5000), "-5,000");
    }

    #[test]
    fn test_today_str_format() {
        let d = today_str();
        assert_eq!(d.len(), 10);
        assert_eq!(d.chars().nth(4).unwrap(), '-');
        assert_eq!(d.chars().nth(7).unwrap(), '-');
    }

    #[test]
    fn test_days_ago_str() {
        let today = today_str();
        let yesterday = days_ago_str(1);
        assert_ne!(today, yesterday);
        assert_eq!(yesterday.len(), 10);
    }
}

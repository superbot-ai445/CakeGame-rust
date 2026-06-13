/// CakeGame 全服经济面板系统
///
/// 提供服务器经济全貌：
/// - 全服金币/钻石总览
/// - 财富排行 TOP15
/// - 平均财富 & 中位数估算
/// - 经济健康指标 (基尼系数估算)
/// - 拍卖行活跃度
/// - 财富等级分布
///
/// 数据来源: Basic_User 表 (金币/钻石), Global 表 (拍卖行)
/// 指令: 经济面板 / 经济排行 / 经济统计
use crate::db::Database;
use crate::user;

/// 格式化数字 (千分位)
fn format_num(n: i64) -> String {
    if n < 0 {
        return format!("-{}", format_num(-n));
    }
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

/// 财富等级名称
const TIER_NAMES: &[&str] = &[
    "一贫如洗",
    "勉强度日",
    "小有积蓄",
    "小康之家",
    "殷实富裕",
    "富甲一方",
    "腰缠万贯",
    "富可敌国",
    "富甲天下",
];

/// 根据总财富 (金+钻×10) 返回等级索引 0~8
fn wealth_tier_index(total_wealth: i64) -> usize {
    if total_wealth >= 5_000_000 {
        8
    } else if total_wealth >= 2_000_000 {
        7
    } else if total_wealth >= 800_000 {
        6
    } else if total_wealth >= 300_000 {
        5
    } else if total_wealth >= 100_000 {
        4
    } else if total_wealth >= 30_000 {
        3
    } else if total_wealth >= 10_000 {
        2
    } else if total_wealth >= 1_000 {
        1
    } else {
        0
    }
}

/// 经济健康等级
fn economy_health(gini: f64) -> &'static str {
    if gini < 0.2 {
        "🟢 非常健康 (贫富差距极小)"
    } else if gini < 0.4 {
        "🟡 健康 (贫富差距适中)"
    } else if gini < 0.6 {
        "🟠 警告 (贫富差距较大)"
    } else {
        "🔴 危险 (贫富差距悬殊)"
    }
}

/// 从 Basic_User 读取所有用户金币和钻石
fn read_all_wealth(db: &Database) -> Vec<(String, i64, i64)> {
    let user_ids = db.all_users();
    let mut result = Vec::new();
    for uid in user_ids {
        let gold: i64 = db.read_basic(&uid, "金币").parse().unwrap_or(0);
        let diamond: i64 = db.read_basic(&uid, "钻石").parse().unwrap_or(0);
        result.push((uid, gold, diamond));
    }
    result
}

/// 计算基尼系数 (衡量贫富差距, 0=完全平等, 1=完全不平等)
/// 基于简化公式: Gini = Σ|xi - xj| / (2 * n² * μ)
fn calc_gini(values: &[i64]) -> f64 {
    let vals: Vec<f64> = values.iter().map(|v| *v as f64).collect();
    if vals.is_empty() {
        return 0.0;
    }
    let n = vals.len() as f64;
    let mean: f64 = vals.iter().sum::<f64>() / n;
    if mean == 0.0 {
        return 0.0;
    }
    let mut sum_diff = 0.0;
    for i in 0..vals.len() {
        for j in 0..vals.len() {
            sum_diff += (vals[i] - vals[j]).abs();
        }
    }
    sum_diff / (2.0 * n * n * mean)
}

/// cmd_economy_panel: 经济面板 — 全服经济概览
pub fn cmd_economy_panel(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let all = read_all_wealth(db);
    let total_users = all.len() as i64;
    if total_users == 0 {
        return format!("{}\n暂无玩家数据。", prefix);
    }

    let total_gold: i64 = all.iter().map(|(_, g, _)| g).sum();
    let total_diamond: i64 = all.iter().map(|(_, _, d)| d).sum();
    let max_gold = all.iter().map(|(_, g, _)| *g).max().unwrap_or(0);
    let max_diamond = all.iter().map(|(_, _, d)| *d).max().unwrap_or(0);
    let avg_gold = total_gold / total_users;
    let avg_diamond = total_diamond / total_users;

    // 财富等级分布
    let mut tier_counts = [0i64; 9];
    for (_, g, d) in &all {
        let tier = wealth_tier_index(g + *d * 10);
        tier_counts[tier] += 1;
    }

    // 基尼系数
    let wealth_values: Vec<i64> = all.iter().map(|(_, g, d)| g + *d * 10).collect();
    let gini = calc_gini(&wealth_values);
    let health = economy_health(gini);

    // 活跃玩家数 (有金币或钻石的)
    let active = all.iter().filter(|(_, g, d)| *g > 0 || *d > 0).count();

    let mut out = String::new();
    out.push_str("🏦 === 全服经济面板 === 🏦\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    // 总览
    out.push_str("📊 【经济总览】\n");
    out.push_str(&format!("  注册玩家: {}\n", format_num(total_users)));
    out.push_str(&format!("  活跃玩家: {}\n", format_num(active as i64)));
    out.push_str(&format!("  金币总量: {}\n", format_num(total_gold)));
    out.push_str(&format!("  钻石总量: {}\n", format_num(total_diamond)));
    out.push_str(&format!(
        "  等价总量: {} (金+钻×10)\n\n",
        format_num(total_gold + total_diamond * 10)
    ));

    // 平均值
    out.push_str("📈 【人均数据】\n");
    out.push_str(&format!("  人均金币: {}\n", format_num(avg_gold)));
    out.push_str(&format!("  人均钻石: {}\n", format_num(avg_diamond)));
    out.push_str(&format!("  最高金币: {}\n", format_num(max_gold)));
    out.push_str(&format!("  最高钻石: {}\n\n", format_num(max_diamond)));

    // 经济健康
    out.push_str("🏥 【经济健康】\n");
    out.push_str(&format!("  基尼系数: {:.3}\n", gini));
    out.push_str(&format!("  健康等级: {}\n\n", health));

    // 财富分布
    out.push_str("💰 【财富等级分布】\n");
    for (i, count) in tier_counts.iter().enumerate() {
        if *count > 0 {
            let pct = *count as f64 * 100.0 / total_users as f64;
            let bar_len = (pct / 5.0).min(10.0) as usize;
            let bar = "█".repeat(bar_len);
            out.push_str(&format!("  {}: {} ({:.1}%)\n", TIER_NAMES[i], bar, pct));
        }
    }
    out.push('\n');

    out.push_str("💡 输入「经济排行」查看财富排行\n");
    out.push_str("💡 输入「经济统计」查看详细统计\n");

    out
}

/// cmd_economy_ranking: 经济排行 — 全服财富排行
pub fn cmd_economy_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let limit = if args.trim().is_empty() {
        15
    } else {
        args.trim().parse::<usize>().unwrap_or(15).min(50)
    };

    let mut all = read_all_wealth(db);
    // 按总财富降序排序 (金 + 钻×10)
    all.sort_by(|a, b| {
        let wa = a.1 + a.2 * 10;
        let wb = b.1 + b.2 * 10;
        wb.cmp(&wa)
    });

    let mut out = String::new();
    out.push_str("🏆 === 全服财富排行 === 🏆\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    if all.is_empty() {
        out.push_str("暂无玩家数据。\n");
        return out;
    }

    let show_count = limit.min(all.len());
    for (i, (uid, gold, diamond)) in all.iter().take(show_count).enumerate() {
        let rank = i + 1;
        let medal = match rank {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "  ",
        };
        let total_wealth = gold + diamond * 10;
        let nickname = db.read_basic(uid, "昵称");
        let level: i32 = db.read_basic(uid, "等级").parse().unwrap_or(1);
        let is_self = uid == user_id;

        out.push_str(&format!(
            "{} {:>2}. {} (Lv.{}) {}金 {}钻 总计{}",
            medal,
            rank,
            nickname,
            level,
            format_num(*gold),
            format_num(*diamond),
            format_num(total_wealth),
        ));
        if is_self {
            out.push_str(" ← 你");
        }
        out.push('\n');
    }

    // 计算自己的排名
    let my_gold: i64 = db.read_basic(user_id, "金币").parse().unwrap_or(0);
    let my_diamond: i64 = db.read_basic(user_id, "钻石").parse().unwrap_or(0);
    let my_wealth = my_gold + my_diamond * 10;
    let my_rank = all.iter().filter(|(_, g, d)| g + *d * 10 > my_wealth).count() + 1;

    out.push_str(&format!("\n📍 你的排名: 第{}名 / 共{}人\n", my_rank, all.len()));

    out
}

/// cmd_economy_stats: 经济统计 — 详细经济数据分析
pub fn cmd_economy_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let all = read_all_wealth(db);
    let total = all.len().max(1) as i64;

    let mut gold_values: Vec<i64> = all.iter().map(|(_, g, _)| *g).collect();
    let mut diamond_values: Vec<i64> = all.iter().map(|(_, _, d)| *d).collect();

    gold_values.sort_by(|a, b| b.cmp(a));
    diamond_values.sort_by(|a, b| b.cmp(a));

    let total_gold: i64 = gold_values.iter().sum();
    let total_diamond: i64 = diamond_values.iter().sum();

    // 中位数
    let gold_median = if gold_values.is_empty() {
        0
    } else {
        gold_values[gold_values.len() / 2]
    };
    let diamond_median = if diamond_values.is_empty() {
        0
    } else {
        diamond_values[diamond_values.len() / 2]
    };

    // TOP1% 持有比例
    let top1_count = ((total as f64 * 0.01).max(1.0)) as usize;
    let top1_gold: i64 = gold_values.iter().take(top1_count).sum();
    let top1_diamond: i64 = diamond_values.iter().take(top1_count).sum();
    let top1_gold_pct = if total_gold > 0 {
        top1_gold as f64 * 100.0 / total_gold as f64
    } else {
        0.0
    };
    let top1_diamond_pct = if total_diamond > 0 {
        top1_diamond as f64 * 100.0 / total_diamond as f64
    } else {
        0.0
    };

    // TOP10% 持有比例
    let top10_count = ((total as f64 * 0.1).max(1.0)) as usize;
    let top10_gold: i64 = gold_values.iter().take(top10_count).sum();
    let top10_diamond: i64 = diamond_values.iter().take(top10_count).sum();
    let top10_gold_pct = if total_gold > 0 {
        top10_gold as f64 * 100.0 / total_gold as f64
    } else {
        0.0
    };
    let top10_diamond_pct = if total_diamond > 0 {
        top10_diamond as f64 * 100.0 / total_diamond as f64
    } else {
        0.0
    };

    // 穷人比例 (金币<100)
    let poor_count = gold_values.iter().filter(|g| **g < 100).count();
    let poor_pct = poor_count as f64 * 100.0 / total as f64;

    // 拍卖行数据 (通过 Global 表计数)
    let conn = db.lock_conn();
    let auction_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Global WHERE Node = 'auction' AND ID != ''")
        .ok()
        .and_then(|mut stmt| stmt.query_row([], |row| row.get(0)).ok())
        .unwrap_or(0);
    drop(conn);

    let mut out = String::new();
    out.push_str("📊 === 全服经济统计 === 📊\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    // 货币分布
    out.push_str("💰 【货币分布】\n");
    out.push_str(&format!("  金币中位数: {}\n", format_num(gold_median)));
    out.push_str(&format!("  钻石中位数: {}\n\n", format_num(diamond_median)));

    // 财富集中度
    out.push_str("🔍 【财富集中度】\n");
    out.push_str(&format!(
        "  TOP 1% ({}) 持有: {:.1}% 金币 / {:.1}% 钻石\n",
        top1_count, top1_gold_pct, top1_diamond_pct
    ));
    out.push_str(&format!(
        "  TOP 10% ({}) 持有: {:.1}% 金币 / {:.1}% 钻石\n",
        top10_count, top10_gold_pct, top10_diamond_pct
    ));
    out.push_str(&format!(
        "  贫困玩家 (金币<100): {}人 ({:.1}%)\n\n",
        poor_count, poor_pct
    ));

    // 市场活跃度
    out.push_str("🏪 【市场活跃度】\n");
    out.push_str(&format!("  活跃拍卖: {} 件\n", auction_count));
    out.push_str(&format!(
        "  人均持有: {} 金 / {} 钻\n\n",
        format_num(total_gold / total),
        format_num(total_diamond / total)
    ));

    // 经济建议
    out.push_str("💡 【经济建议】\n");
    if poor_pct > 30.0 {
        out.push_str("  ⚠️ 贫困玩家比例过高，建议增加金币产出途径\n");
    }
    if top1_gold_pct > 50.0 {
        out.push_str("  ⚠️ 财富过于集中，建议增加消耗型活动\n");
    }
    if auction_count < 10 {
        out.push_str("  📢 拍卖行不够活跃，建议鼓励玩家交易\n");
    }

    out
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
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(-1234), "-1,234");
    }

    #[test]
    fn test_economy_health() {
        assert!(economy_health(0.1).contains("非常健康"));
        assert!(economy_health(0.3).contains("健康"));
        assert!(economy_health(0.5).contains("警告"));
        assert!(economy_health(0.7).contains("危险"));
    }

    #[test]
    fn test_tier_names_count() {
        assert_eq!(TIER_NAMES.len(), 9);
    }

    #[test]
    fn test_wealth_tier_index() {
        assert_eq!(wealth_tier_index(0), 0);
        assert_eq!(wealth_tier_index(500), 0);
        assert_eq!(wealth_tier_index(1000), 1);
        assert_eq!(wealth_tier_index(10000), 2);
        assert_eq!(wealth_tier_index(30000), 3);
        assert_eq!(wealth_tier_index(100000), 4);
        assert_eq!(wealth_tier_index(300000), 5);
        assert_eq!(wealth_tier_index(800000), 6);
        assert_eq!(wealth_tier_index(2000000), 7);
        assert_eq!(wealth_tier_index(5000000), 8);
    }

    #[test]
    fn test_format_num_large() {
        assert_eq!(format_num(1_000_000_000), "1,000,000,000");
        assert_eq!(format_num(999_999), "999,999");
    }

    #[test]
    fn test_gini_empty() {
        let empty: Vec<i64> = vec![];
        assert_eq!(calc_gini(&empty), 0.0);
    }

    #[test]
    fn test_gini_equal() {
        let values = vec![100, 100, 100, 100];
        assert!(calc_gini(&values) < 0.001);
    }

    #[test]
    fn test_gini_unequal() {
        let values = vec![0, 0, 0, 1000];
        let gini = calc_gini(&values);
        assert!(gini > 0.5);
    }
}

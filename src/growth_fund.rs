/// CakeGame 成长基金系统
///
/// 玩家投资钻石购买成长基金，随着等级提升领取递增奖励。
/// 基金分3档：白银基金(100💎) / 黄金基金(300💎) / 钻石基金(500💎)
/// 达到指定等级后可领取对应阶段奖励（金币+钻石+稀有物品）
/// 每档基金只能购买一次，奖励未领取不影响后续阶段
///
/// 指令: 查看基金/购买基金/领取基金/基金进度/基金排行
/// 数据存储: Global 表 SECTION='growth_fund'
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD};
use crate::db::Database;
use crate::user;

const SECTION: &str = "growth_fund";

/// 基金档位定义
#[allow(dead_code)]
struct FundTier {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    cost_diamond: i64,
    /// 各阶段 (等级要求, 金币奖励, 钻石奖励)
    stages: &'static [(i32, i64, i32)],
    desc: &'static str,
}

const FUND_TIERS: &[FundTier] = &[
    FundTier {
        id: "silver",
        name: "白银基金",
        emoji: "🥈",
        cost_diamond: 100,
        stages: &[
            (10, 2000, 10),
            (20, 5000, 20),
            (30, 10000, 30),
            (40, 20000, 50),
            (50, 50000, 80),
        ],
        desc: "100💎 入门基金，5阶段返还",
    },
    FundTier {
        id: "gold",
        name: "黄金基金",
        emoji: "🥇",
        cost_diamond: 300,
        stages: &[
            (10, 5000, 30),
            (20, 12000, 50),
            (30, 25000, 80),
            (40, 50000, 120),
            (50, 100000, 200),
            (60, 200000, 300),
        ],
        desc: "300💎 进阶基金，6阶段返还，总价值超2倍",
    },
    FundTier {
        id: "diamond",
        name: "钻石基金",
        emoji: "💎",
        cost_diamond: 500,
        stages: &[
            (10, 10000, 50),
            (20, 25000, 100),
            (30, 50000, 150),
            (40, 100000, 250),
            (50, 200000, 400),
            (60, 400000, 600),
            (70, 800000, 1000),
        ],
        desc: "500💎 至尊基金，7阶段返还，总价值超5倍",
    },
];

/// 获取用户基金购买状态 (逗号分隔的已购买基金ID)
fn get_purchased(db: &Database, user_id: &str) -> Vec<String> {
    let key = format!("fund_purchased_{}", user_id);
    let raw = db.global_get(SECTION, &key);
    if raw.is_empty() {
        Vec::new()
    } else {
        raw.split(',').map(|s| s.to_string()).collect()
    }
}

/// 设置已购买基金
fn set_purchased(db: &Database, user_id: &str, purchased: &[String]) {
    let key = format!("fund_purchased_{}", user_id);
    db.global_set(SECTION, &key, &purchased.join(","));
}

/// 获取已领取的阶段 (形如 "silver:1,silver:2,gold:1")
fn get_claimed(db: &Database, user_id: &str) -> Vec<String> {
    let key = format!("fund_claimed_{}", user_id);
    let raw = db.global_get(SECTION, &key);
    if raw.is_empty() {
        Vec::new()
    } else {
        raw.split(',').map(|s| s.to_string()).collect()
    }
}

/// 添加已领取记录
fn add_claimed(db: &Database, user_id: &str, claim_key: &str) {
    let key = format!("fund_claimed_{}", user_id);
    let mut claimed = get_claimed(db, user_id);
    claimed.push(claim_key.to_string());
    db.global_set(SECTION, &key, &claimed.join(","));
}

/// 计算基金总收益
fn calc_total_return(tier: &FundTier) -> (i64, i32) {
    let mut total_gold = 0i64;
    let mut total_diamond = 0i32;
    for &(_, gold, diamond) in tier.stages {
        total_gold += gold;
        total_diamond += diamond;
    }
    (total_gold, total_diamond)
}

/// 查看基金
pub fn cmd_view_fund(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }

    let purchased = get_purchased(db, user_id);
    let claimed = get_claimed(db, user_id);
    let level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);

    let mut out = format!("{}\n═══ 💰 成长基金 ═══\n", prefix);
    out.push_str(&format!("📊 当前等级: {}\n\n", level));

    for tier in FUND_TIERS {
        let is_purchased = purchased.contains(&tier.id.to_string());
        let status = if is_purchased { "✅ 已购买" } else { "❌ 未购买" };
        let (total_gold, total_diamond) = calc_total_return(tier);

        out.push_str(&format!(
            "{} {} — {} {}\n   💰 投资: {}💎 | 总回报: {}金+{}💎\n",
            tier.emoji, tier.name, tier.desc, status, tier.cost_diamond, total_gold, total_diamond
        ));

        if is_purchased {
            out.push_str("   📋 阶段进度:\n");
            for (i, &(req_level, gold, diamond)) in tier.stages.iter().enumerate() {
                let ck = format!("{}:{}", tier.id, i + 1);
                let is_claimed = claimed.contains(&ck);
                let can_claim = level >= req_level && !is_claimed;
                let icon = if is_claimed {
                    "✅"
                } else if can_claim {
                    "🎁"
                } else {
                    "🔒"
                };
                out.push_str(&format!(
                    "     {} Lv.{}: {}金+{}💎{}\n",
                    icon,
                    req_level,
                    gold,
                    diamond,
                    if is_claimed {
                        " [已领取]"
                    } else if can_claim {
                        " [可领取]"
                    } else {
                        ""
                    }
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("💡 发送 购买基金+档位 购买 | 领取基金+档位 领取奖励");
    out
}

/// 购买基金
pub fn cmd_buy_fund(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }

    let tier_name = args.trim();
    if tier_name.is_empty() {
        return format!("{}\n请指定基金档位：白银/黄金/钻石\n💡 示例: 购买基金+白银", prefix);
    }

    let tier = FUND_TIERS
        .iter()
        .find(|t| t.name.contains(tier_name) || t.id == tier_name);

    let tier = match tier {
        Some(t) => t,
        None => return format!("{}\n❌ 未找到基金档位: {}\n可选: 白银/黄金/钻石", prefix, tier_name),
    };

    let mut purchased = get_purchased(db, user_id);
    if purchased.contains(&tier.id.to_string()) {
        return format!("{}\n❌ 您已购买过{}！", prefix, tier.name);
    }

    // 扣除钻石
    let after = db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", tier.cost_diamond);
    if after < 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", tier.cost_diamond);
        return format!(
            "{}\n❌ 钻石不足！需要{}💎，当前{}💎",
            prefix,
            tier.cost_diamond,
            after + tier.cost_diamond
        );
    }

    purchased.push(tier.id.to_string());
    set_purchased(db, user_id, &purchased);

    let (total_gold, total_diamond) = calc_total_return(tier);
    format!(
        "{}\n🎉 成功购买{}{}！\n💰 投资: {}💎\n📈 预计总回报: {}金+{}💎\n💡 达到对应等级后发送 领取基金+{} 领取奖励",
        prefix, tier.emoji, tier.name, tier.cost_diamond, total_gold, total_diamond, tier.name
    )
}

/// 领取基金奖励
pub fn cmd_claim_fund(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }

    let tier_name = args.trim();
    let purchased = get_purchased(db, user_id);
    let claimed = get_claimed(db, user_id);
    let level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);

    // 如果没有指定档位，尝试全部领取
    let tiers_to_check: Vec<&FundTier> = if tier_name.is_empty() {
        FUND_TIERS
            .iter()
            .filter(|t| purchased.contains(&t.id.to_string()))
            .collect()
    } else {
        let tier = FUND_TIERS
            .iter()
            .find(|t| t.name.contains(tier_name) || t.id == tier_name);
        match tier {
            Some(t) => {
                if !purchased.contains(&t.id.to_string()) {
                    return format!("{}\n❌ 您尚未购买{}！", prefix, t.name);
                }
                vec![t]
            }
            None => return format!("{}\n❌ 未找到基金档位: {}", prefix, tier_name),
        }
    };

    if tiers_to_check.is_empty() {
        return format!("{}\n❌ 您尚未购买任何基金！\n💡 发送 查看基金 了解详情", prefix);
    }

    let mut total_gold_claimed = 0i64;
    let mut total_diamond_claimed = 0i32;
    let mut claims_made = Vec::new();

    for tier in &tiers_to_check {
        for (i, &(req_level, gold, diamond)) in tier.stages.iter().enumerate() {
            let ck = format!("{}:{}", tier.id, i + 1);
            if claimed.contains(&ck) {
                continue;
            }
            if level < req_level {
                continue;
            }
            // 可以领取
            add_claimed(db, user_id, &ck);
            db.modify_currency(user_id, CURRENCY_GOLD, "add", gold);
            db.modify_currency(user_id, CURRENCY_DIAMOND, "add", diamond as i64);
            total_gold_claimed += gold;
            total_diamond_claimed += diamond;
            claims_made.push(format!("{}阶段{}", tier.name, i + 1));
        }
    }

    if claims_made.is_empty() {
        return format!("{}\n📭 暂无可领取的基金奖励\n💡 等级提升后才能领取对应阶段奖励", prefix);
    }

    format!(
        "{}\n🎉 基金奖励领取成功！\n📋 领取: {}\n💰 获得: {}金+{}💎\n💡 继续升级解锁更多奖励！",
        prefix,
        claims_made.join("、"),
        total_gold_claimed,
        total_diamond_claimed
    )
}

/// 基金进度
pub fn cmd_fund_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }

    let purchased = get_purchased(db, user_id);
    let claimed = get_claimed(db, user_id);
    let level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);

    if purchased.is_empty() {
        return format!("{}\n📭 您尚未购买任何基金\n💡 发送 查看基金 了解详情", prefix);
    }

    let mut out = format!("{}\n═══ 📊 基金进度 ═══\n\n📊 当前等级: {}\n", prefix, level);

    let mut total_invested = 0i64;
    let mut total_claimed_gold = 0i64;
    let mut total_claimed_diamond = 0i32;
    let mut total_pending_gold = 0i64;
    let mut total_pending_diamond = 0i32;

    for tier in FUND_TIERS {
        if !purchased.contains(&tier.id.to_string()) {
            continue;
        }
        total_invested += tier.cost_diamond;

        let mut claimed_count: usize = 0;
        let mut claimable_count = 0;
        let mut locked_count = 0;

        for (i, &(req_level, gold, diamond)) in tier.stages.iter().enumerate() {
            let ck = format!("{}:{}", tier.id, i + 1);
            if claimed.contains(&ck) {
                claimed_count += 1;
                total_claimed_gold += gold;
                total_claimed_diamond += diamond;
            } else if level >= req_level {
                claimable_count += 1;
                total_pending_gold += gold;
                total_pending_diamond += diamond;
            } else {
                locked_count += 1;
                total_pending_gold += gold;
                total_pending_diamond += diamond;
            }
        }

        let total_stages = tier.stages.len();
        let progress_pct = claimed_count.saturating_mul(100).checked_div(total_stages).unwrap_or(0);
        let bar_len = 10;
        let filled = claimed_count * bar_len / total_stages.max(1);
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);

        out.push_str(&format!(
            "{} {} [{}] {}%\n   ✅已领:{} 🎁可领:{} 🔒未达:{}\n",
            tier.emoji, tier.name, bar, progress_pct, claimed_count, claimable_count, locked_count
        ));
    }

    let (_invest_gold, _) = calc_total_return(FUND_TIERS.iter().find(|t| t.id == "silver").unwrap());
    out.push_str(&format!(
        "\n💰 投资总额: {}💎\n📈 已领取: {}金+{}💎\n⏳ 待领取: {}金+{}💎",
        total_invested, total_claimed_gold, total_claimed_diamond, total_pending_gold, total_pending_diamond
    ));

    if total_pending_gold > 0 || total_pending_diamond > 0 {
        out.push_str("\n\n💡 发送 领取基金 一键领取所有可领奖励");
    }

    out
}

/// 基金排行
pub fn cmd_fund_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 收集所有基金用户
    let sections: Vec<String> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) =
            conn.prepare("SELECT DISTINCT ID FROM Global WHERE SECTION = 'growth_fund' AND ID LIKE 'fund_claimed_%'")
        {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    // fund_claimed_{user_id} → user_id
                    if let Some(uid) = row.strip_prefix("fund_claimed_") {
                        result.push(uid.to_string());
                    }
                }
            }
        }
        result
    };

    if sections.is_empty() {
        return format!("{}\n📭 暂无基金数据", prefix);
    }

    let mut players: Vec<(String, i32, i64)> = Vec::new(); // (name, tier_count, total_gold)

    for uid in &sections {
        let purchased = get_purchased(db, uid);
        let claimed = get_claimed(db, uid);
        let tier_count = purchased.len() as i32;

        let mut total_gold = 0i64;
        for tier in FUND_TIERS {
            if !purchased.contains(&tier.id.to_string()) {
                continue;
            }
            for (i, &(_, gold, _)) in tier.stages.iter().enumerate() {
                let ck = format!("{}:{}", tier.id, i + 1);
                if claimed.contains(&ck) {
                    total_gold += gold;
                }
            }
        }

        if tier_count > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, tier_count, total_gold));
        }
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("{}\n═══ 🏆 基金排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];

    for (i, (name, tier_count, total_gold)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        out.push_str(&format!(
            "\n{} {} — {}档基金 — 已领{}金",
            medal, name, tier_count, total_gold
        ));
    }

    // 用户排名
    let my_name = user::get_msg_prefix(db, user_id);
    if let Some(rank) = players.iter().position(|(name, _, _)| name == &my_name) {
        out.push_str(&format!("\n\n📍 你的排名：第{}名", rank + 1));
    }

    out
}

/// 获取基金总收益加成（供其他模块调用）
#[allow(dead_code)]
pub fn get_fund_bonus(db: &Database, user_id: &str) -> (i64, i32) {
    let purchased = get_purchased(db, user_id);
    let mut bonus_gold_pct = 0i64;
    let mut bonus_diamond_pct = 0i32;

    // 每购买一档基金，增加5%金币加成和3%钻石加成
    for tier in FUND_TIERS {
        if purchased.contains(&tier.id.to_string()) {
            bonus_gold_pct += 5;
            bonus_diamond_pct += 3;
        }
    }

    (bonus_gold_pct, bonus_diamond_pct)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fund_tiers_count() {
        assert_eq!(FUND_TIERS.len(), 3);
    }

    #[test]
    fn test_fund_tier_ids_unique() {
        let ids: Vec<&str> = FUND_TIERS.iter().map(|t| t.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }

    #[test]
    fn test_fund_tier_names() {
        assert_eq!(FUND_TIERS[0].name, "白银基金");
        assert_eq!(FUND_TIERS[1].name, "黄金基金");
        assert_eq!(FUND_TIERS[2].name, "钻石基金");
    }

    #[test]
    fn test_cost_diamond_positive() {
        for tier in FUND_TIERS {
            assert!(tier.cost_diamond > 0, "{} cost must be positive", tier.name);
        }
    }

    #[test]
    fn test_stages_not_empty() {
        for tier in FUND_TIERS {
            assert!(!tier.stages.is_empty(), "{} must have stages", tier.name);
        }
    }

    #[test]
    fn test_stages_requirements_ascending() {
        for tier in FUND_TIERS {
            for i in 1..tier.stages.len() {
                assert!(
                    tier.stages[i].0 > tier.stages[i - 1].0,
                    "{} stage {} level should be > stage {}",
                    tier.name,
                    i,
                    i - 1
                );
            }
        }
    }

    #[test]
    fn test_stages_rewards_positive() {
        for tier in FUND_TIERS {
            for (i, &(level, gold, diamond)) in tier.stages.iter().enumerate() {
                assert!(level > 0, "{} stage {} level must be positive", tier.name, i);
                assert!(gold > 0, "{} stage {} gold must be positive", tier.name, i);
                assert!(diamond > 0, "{} stage {} diamond must be positive", tier.name, i);
            }
        }
    }

    #[test]
    fn test_calc_total_return() {
        let (gold, diamond) = calc_total_return(&FUND_TIERS[0]);
        assert_eq!(gold, 2000 + 5000 + 10000 + 20000 + 50000);
        assert_eq!(diamond, 10 + 20 + 30 + 50 + 80);
    }

    #[test]
    fn test_diamond_tier_highest_return() {
        let (_, d_silver) = calc_total_return(&FUND_TIERS[0]);
        let (_, d_gold) = calc_total_return(&FUND_TIERS[1]);
        let (_, d_diamond) = calc_total_return(&FUND_TIERS[2]);
        assert!(d_diamond > d_gold);
        assert!(d_gold > d_silver);
    }

    #[test]
    fn test_return_exceeds_cost() {
        // 每档基金的总回报钻石应超过投资
        for tier in FUND_TIERS {
            let (_, total_diamond) = calc_total_return(tier);
            assert!(
                total_diamond as i64 > tier.cost_diamond,
                "{} return ({}💎) should exceed cost ({}💎)",
                tier.name,
                total_diamond,
                tier.cost_diamond
            );
        }
    }

    #[test]
    fn test_stages_count() {
        assert_eq!(FUND_TIERS[0].stages.len(), 5);
        assert_eq!(FUND_TIERS[1].stages.len(), 6);
        assert_eq!(FUND_TIERS[2].stages.len(), 7);
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "growth_fund");
    }

    #[test]
    fn test_fund_tier_emojis() {
        assert_eq!(FUND_TIERS[0].emoji, "🥈");
        assert_eq!(FUND_TIERS[1].emoji, "🥇");
        assert_eq!(FUND_TIERS[2].emoji, "💎");
    }

    #[test]
    fn test_each_stage_has_3_values() {
        for tier in FUND_TIERS {
            for &(level, gold, diamond) in tier.stages {
                assert!(level > 0);
                assert!(gold > 0);
                assert!(diamond > 0);
            }
        }
    }

    #[test]
    fn test_gold_rewards_escalate() {
        for tier in FUND_TIERS {
            for i in 1..tier.stages.len() {
                assert!(
                    tier.stages[i].1 > tier.stages[i - 1].1,
                    "{} gold should increase: stage {} ({}) > stage {} ({})",
                    tier.name,
                    i,
                    tier.stages[i].1,
                    i - 1,
                    tier.stages[i - 1].1
                );
            }
        }
    }

    #[test]
    fn test_diamond_rewards_escalate() {
        for tier in FUND_TIERS {
            for i in 1..tier.stages.len() {
                assert!(
                    tier.stages[i].2 > tier.stages[i - 1].2,
                    "{} diamond should increase: stage {} ({}) > stage {} ({})",
                    tier.name,
                    i,
                    tier.stages[i].2,
                    i - 1,
                    tier.stages[i - 1].2
                );
            }
        }
    }

    #[test]
    fn test_higher_tier_more_stages() {
        assert!(FUND_TIERS[2].stages.len() > FUND_TIERS[1].stages.len());
        assert!(FUND_TIERS[1].stages.len() > FUND_TIERS[0].stages.len());
    }

    #[test]
    fn test_desc_not_empty() {
        for tier in FUND_TIERS {
            assert!(!tier.desc.is_empty());
        }
    }
}

/// CakeGame 全服许愿池系统
///
/// 全服玩家共同捐献金币/钻石，达成阶段目标后所有贡献者可领取奖励
/// 数据存储: Global 表 SECTION='wish_pool'
/// Key格式:
///   pool_total  → 当前周期总捐献金币
///   pool_diamond → 当前周期总捐献钻石
///   pool_week   → 当前周期周号（用于检测重置）
///   contrib:{user_id} → 玩家累计捐献金额
///   claimed:{user_id} → 玩家已领取奖励的阶段位掩码
use crate::core::*;
use crate::db::Database;
use crate::user;

const SECTION: &str = "wish_pool";
const PROGRESS_WIDTH: usize = 10;

/// 许愿池阶段定义
struct WishTier {
    tier: u32,
    threshold: i64,
    reward_gold: i64,
    reward_diamond: i64,
    reward_exp: i32,
    reward_item: &'static str,
    desc: &'static str,
}

const WISH_TIERS: &[WishTier] = &[
    WishTier {
        tier: 1,
        threshold: 100_000,
        reward_gold: 500,
        reward_diamond: 0,
        reward_exp: 100,
        reward_item: "",
        desc: "⭐ 初级许愿 — 全服捐献达到10万金币",
    },
    WishTier {
        tier: 2,
        threshold: 500_000,
        reward_gold: 1000,
        reward_diamond: 50,
        reward_exp: 300,
        reward_item: "",
        desc: "⭐⭐ 中级许愿 — 全服捐献达到50万金币",
    },
    WishTier {
        tier: 3,
        threshold: 2_000_000,
        reward_gold: 2000,
        reward_diamond: 100,
        reward_exp: 500,
        reward_item: "幸运宝箱",
        desc: "⭐⭐⭐ 高级许愿 — 全服捐献达到200万金币",
    },
    WishTier {
        tier: 4,
        threshold: 5_000_000,
        reward_gold: 5000,
        reward_diamond: 200,
        reward_exp: 1000,
        reward_item: "",
        desc: "🌟 稀有许愿 — 全服捐献达到500万金币",
    },
    WishTier {
        tier: 5,
        threshold: 10_000_000,
        reward_gold: 10000,
        reward_diamond: 500,
        reward_exp: 2000,
        reward_item: "传说宝箱",
        desc: "🌟🌟 传说许愿 — 全服捐献达到1000万金币",
    },
];

/// djb2哈希
#[allow(dead_code)]
fn djb2_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// 格式化数字（千分位）
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

/// 获取当前周号（基于 Unix 时间戳 / 604800）
fn current_week() -> i64 {
    let ts = chrono::Local::now().timestamp();
    ts / 604800
}

/// 构建进度条 (10格 █░)
fn progress_bar(ratio: f64, width: usize) -> String {
    let r = ratio.clamp(0.0, 1.0);
    let filled = (r * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// 从 Global 表读取许愿池总金币
fn load_pool_total(db: &Database) -> i64 {
    db.global_get(SECTION, "pool_total").parse().unwrap_or(0)
}

/// 保存许愿池总金币
fn save_pool_total(db: &Database, total: i64) {
    db.global_set(SECTION, "pool_total", &total.to_string());
}

/// 从 Global 表读取许愿池总钻石
fn load_pool_diamond(db: &Database) -> i64 {
    db.global_get(SECTION, "pool_diamond").parse().unwrap_or(0)
}

/// 保存许愿池总钻石
fn save_pool_diamond(db: &Database, total: i64) {
    db.global_set(SECTION, "pool_diamond", &total.to_string());
}

/// 读取当前存储的周号
fn load_week(db: &Database) -> i64 {
    db.global_get(SECTION, "pool_week").parse().unwrap_or(0)
}

/// 检查并执行周重置（如果周号变化，清空所有数据）
fn check_weekly_reset(db: &Database) -> bool {
    let stored_week = load_week(db);
    let now_week = current_week();
    if stored_week != now_week {
        // 新的一周，重置许愿池
        save_pool_total(db, 0);
        save_pool_diamond(db, 0);
        db.global_set(SECTION, "pool_week", &now_week.to_string());
        // 清空所有贡献记录和领取记录
        clear_all_contributions(db);
        true
    } else {
        false
    }
}

/// 清空所有玩家的贡献和领取记录
fn clear_all_contributions(db: &Database) {
    let conn = db.lock_conn();
    let _ = conn.execute(
        &format!(
            "DELETE FROM Global WHERE SECTION = '{}' AND (ID LIKE 'contrib:%' OR ID LIKE 'claimed:%')",
            SECTION
        ),
        [],
    );
}

/// 读取玩家贡献金额
fn load_contribution(db: &Database, user_id: &str) -> i64 {
    let key = format!("contrib:{}", user_id);
    db.global_get(SECTION, &key).parse().unwrap_or(0)
}

/// 保存玩家贡献金额
fn save_contribution(db: &Database, user_id: &str, amount: i64) {
    let key = format!("contrib:{}", user_id);
    db.global_set(SECTION, &key, &amount.to_string());
}

/// 读取玩家已领取奖励的阶段位掩码
fn load_claimed_mask(db: &Database, user_id: &str) -> u32 {
    let key = format!("claimed:{}", user_id);
    db.global_get(SECTION, &key).parse().unwrap_or(0)
}

/// 保存玩家已领取奖励的阶段位掩码
fn save_claimed_mask(db: &Database, user_id: &str, mask: u32) {
    let key = format!("claimed:{}", user_id);
    db.global_set(SECTION, &key, &mask.to_string());
}

/// 生成贡献排名列表（从 Global 表读取所有 contrib: 记录）
fn load_all_contributions(db: &Database) -> Vec<(String, i64)> {
    let mut results = Vec::new();
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT ID, DATA FROM Global WHERE SECTION = '{}' AND ID LIKE 'contrib:%'",
        SECTION
    )) {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                let (id, data) = row;
                let uid = id.strip_prefix("contrib:").unwrap_or(&id).to_string();
                let amount: i64 = data.parse().unwrap_or(0);
                if amount > 0 {
                    results.push((uid, amount));
                }
            }
        }
    }
    results.sort_by_key(|b| std::cmp::Reverse(b.1));
    results
}

/// 构建奖励描述文本
fn reward_desc(tier: &WishTier) -> String {
    let mut parts = Vec::new();
    if tier.reward_gold > 0 {
        parts.push(format!("💰{}金币", format_num(tier.reward_gold)));
    }
    if tier.reward_diamond > 0 {
        parts.push(format!("💎{}钻石", format_num(tier.reward_diamond)));
    }
    if tier.reward_exp > 0 {
        parts.push(format!("✨{}经验", format_num(tier.reward_exp as i64)));
    }
    if !tier.reward_item.is_empty() {
        parts.push(format!("🎁{}", tier.reward_item));
    }
    parts.join(" + ")
}

/// 找到当前最高已达成阶段
#[allow(dead_code)]
fn highest_reached_tier(pool_total: i64) -> u32 {
    let mut highest = 0u32;
    for tier in WISH_TIERS {
        if pool_total >= tier.threshold {
            highest = tier.tier;
        }
    }
    highest
}

/// 找到下一个未达成阶段的阈值
fn next_unreached_threshold(pool_total: i64) -> Option<i64> {
    for tier in WISH_TIERS {
        if pool_total < tier.threshold {
            return Some(tier.threshold);
        }
    }
    None
}

/// 指令: 查看许愿池 — 显示当前全服捐献进度、各阶段状态
pub fn cmd_view_wish_pool(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    check_weekly_reset(db);

    let pool_total = load_pool_total(db);
    let pool_diamond = load_pool_diamond(db);
    let my_contrib = load_contribution(db, user_id);
    let claimed = load_claimed_mask(db, user_id);

    let mut out = String::new();
    out.push_str(&format!("{}\n", prefix));
    out.push_str("🌟 ═══ 全服许愿池 ═══ 🌟\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!(
        "💰 当前捐献总额: {}金币 + {}钻石\n",
        format_num(pool_total),
        format_num(pool_diamond)
    ));
    out.push_str(&format!("📊 我的贡献: {}金币\n", format_num(my_contrib)));
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    // 各阶段进度
    for tier in WISH_TIERS {
        let reached = pool_total >= tier.threshold;
        let ratio = if tier.threshold > 0 {
            (pool_total as f64 / tier.threshold as f64).min(1.0)
        } else {
            0.0
        };
        let bar = progress_bar(ratio, PROGRESS_WIDTH);
        let status_icon = if reached { "✅" } else { "⬜" };
        let claimed_bit = 1u32 << (tier.tier - 1);
        let claimed_str = if reached && (claimed & claimed_bit) != 0 {
            " [已领取]"
        } else {
            ""
        };

        out.push_str(&format!("\n{} 阶段{}: {}\n", status_icon, tier.tier, tier.desc));
        out.push_str(&format!(
            "  {} {}/{}\n",
            bar,
            format_num(pool_total.min(tier.threshold)),
            format_num(tier.threshold)
        ));
        out.push_str(&format!("  🎁 奖励: {}{}\n", reward_desc(tier), claimed_str));
    }

    // 下一阶段提示
    if let Some(next_threshold) = next_unreached_threshold(pool_total) {
        let remaining = next_threshold - pool_total;
        out.push_str(&format!("\n💡 距离下一阶段还需 {} 金币\n", format_num(remaining)));
    } else {
        out.push_str("\n🎉 所有阶段已达成！快去领取奖励吧！\n");
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 发送「许愿池捐献+金额」捐献金币\n");
    out.push_str("💡 发送「许愿池捐献钻石+金额」捐献钻石\n");
    out.push_str("💡 发送「许愿池奖励」领取奖励\n");
    out
}

/// 指令: 捐献许愿池 — 捐献金币或钻石
pub fn cmd_contribute_wish(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    check_weekly_reset(db);

    let args_trim = args.trim();

    // 判断是金币还是钻石捐献
    let (is_diamond, amount) = if args_trim.starts_with("钻石") {
        let num_str = args_trim.strip_prefix("钻石").unwrap_or("").trim();
        let amt: i64 = match num_str.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return format!("{}\n⚠️ 请输入正确的捐献金额！\n💡 示例: 许愿池捐献钻石+100", prefix);
            }
        };
        (true, amt)
    } else {
        let amt: i64 = match args_trim.parse() {
            Ok(n) if n > 0 => n,
            _ => {
                return format!("{}\n⚠️ 请输入正确的捐献金额！\n💡 示例: 许愿池捐献+1000", prefix);
            }
        };
        (false, amt)
    };

    // 检查玩家余额
    if is_diamond {
        let diamond: i64 = db.read_basic(user_id, CURRENCY_DIAMOND).parse().unwrap_or(0);
        if diamond < amount {
            return format!(
                "{}\n⚠️ 钻石不足！当前 {} 钻石，需要 {} 钻石",
                prefix,
                format_num(diamond),
                format_num(amount)
            );
        }
        // 扣除钻石
        db.write_basic(user_id, CURRENCY_DIAMOND, &(diamond - amount).to_string());
        // 更新全服钻石池
        let pool_diamond = load_pool_diamond(db);
        save_pool_diamond(db, pool_diamond + amount);
        // 更新个人贡献（钻石按1:10折算为金币计入贡献排名）
        let contrib = load_contribution(db, user_id);
        save_contribution(db, user_id, contrib + amount * 10);
    } else {
        let gold: i64 = db.read_basic(user_id, CURRENCY_GOLD).parse().unwrap_or(0);
        if gold < amount {
            return format!(
                "{}\n⚠️ 金币不足！当前 {} 金币，需要 {} 金币",
                prefix,
                format_num(gold),
                format_num(amount)
            );
        }
        // 扣除金币
        db.write_basic(user_id, CURRENCY_GOLD, &(gold - amount).to_string());
        // 更新全服金币池
        let pool_total = load_pool_total(db);
        save_pool_total(db, pool_total + amount);
        // 更新个人贡献
        let contrib = load_contribution(db, user_id);
        save_contribution(db, user_id, contrib + amount);
    }

    let pool_total = load_pool_total(db);
    let my_contrib = load_contribution(db, user_id);
    let currency_name = if is_diamond { "钻石" } else { "金币" };

    let mut out = String::new();
    out.push_str(&format!("{}\n", prefix));
    out.push_str(&format!(
        "🌟 许愿池捐献成功！\n捐献 {} {} 至全服许愿池\n",
        format_num(amount),
        currency_name
    ));
    out.push_str(&format!(
        "📊 我的总贡献: {} | 全服总额: {}金币\n",
        format_num(my_contrib),
        format_num(pool_total)
    ));

    // 检查是否刚达成新阶段
    for tier in WISH_TIERS {
        if pool_total >= tier.threshold && (pool_total - amount) < tier.threshold {
            out.push_str(&format!("\n🎉🎉🎉 全服达成阶段{}！所有贡献者可领取奖励！\n", tier.tier));
        }
    }

    out
}

/// 指令: 许愿池排行 — 查看捐献排行榜 Top15 + 当前用户位置
pub fn cmd_wish_pool_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    check_weekly_reset(db);

    let all_contribs = load_all_contributions(db);
    if all_contribs.is_empty() {
        return format!(
            "{}\n🌟 许愿池暂无捐献记录\n💡 发送「许愿池捐献+金额」开始捐献！",
            prefix
        );
    }

    let pool_total = load_pool_total(db);

    let mut out = String::new();
    out.push_str(&format!("{}\n", prefix));
    out.push_str(&format!(
        "🏆 ═══ 许愿池捐献排行 ═══ 🏆\n全服总额: {}金币\n",
        format_num(pool_total)
    ));
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    // Top 15
    let top_n = all_contribs.len().min(15);
    for (i, (uid, amount)) in all_contribs.iter().take(top_n).enumerate() {
        let rank_icon = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let name = user::get_msg_prefix(db, uid);
        let is_me = if uid == user_id { " ← 我" } else { "" };
        out.push_str(&format!(
            "{} {}. {} — {}金币{}\n",
            rank_icon,
            i + 1,
            name,
            format_num(*amount),
            is_me
        ));
    }

    // 当前用户排名（如果不在Top15中）
    let my_rank = all_contribs.iter().position(|(uid, _)| uid == user_id);
    match my_rank {
        Some(pos) if pos >= 15 => {
            let my_amount = all_contribs[pos].1;
            out.push_str("...\n");
            out.push_str(&format!(
                "  {}. {} — {}金币 ← 我\n",
                pos + 1,
                prefix,
                format_num(my_amount)
            ));
        }
        None => {
            let my_contrib = load_contribution(db, user_id);
            let rank = all_contribs.len() + 1;
            out.push_str(&format!(
                "  {}. {} — {}金币 ← 我\n",
                rank,
                prefix,
                format_num(my_contrib)
            ));
        }
        _ => {}
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 每周一重置，奖励需手动领取\n");
    out
}

/// 指令: 许愿池奖励 — 查看可领取的奖励并领取
pub fn cmd_wish_pool_rewards(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    check_weekly_reset(db);

    let pool_total = load_pool_total(db);
    let my_contrib = load_contribution(db, user_id);

    if my_contrib <= 0 {
        return format!(
            "{}\n🌟 您本周期尚未向许愿池捐献！\n💡 发送「许愿池捐献+金额」参与捐献后才能领取奖励",
            prefix
        );
    }

    let mut claimed = load_claimed_mask(db, user_id);
    let mut out = String::new();
    out.push_str(&format!("{}\n", prefix));
    out.push_str("🎁 ═══ 许愿池奖励 ═══ 🎁\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    let mut claimed_any = false;
    let mut any_available = false;

    for tier in WISH_TIERS {
        let bit = 1u32 << (tier.tier - 1);
        let already_claimed = (claimed & bit) != 0;
        let reached = pool_total >= tier.threshold;

        if reached && !already_claimed {
            // 可领取
            any_available = true;
            // 发放奖励
            let gold: i64 = db.read_basic(user_id, CURRENCY_GOLD).parse().unwrap_or(0);
            let diamond: i64 = db.read_basic(user_id, CURRENCY_DIAMOND).parse().unwrap_or(0);

            if tier.reward_gold > 0 {
                db.write_basic(user_id, CURRENCY_GOLD, &(gold + tier.reward_gold).to_string());
            }
            if tier.reward_diamond > 0 {
                db.write_basic(user_id, CURRENCY_DIAMOND, &(diamond + tier.reward_diamond).to_string());
            }
            if tier.reward_exp > 0 {
                user::add_experience(db, user_id, tier.reward_exp);
            }
            if !tier.reward_item.is_empty() {
                db.add_item(user_id, tier.reward_item, 1);
            }

            // 标记已领取
            claimed |= bit;
            save_claimed_mask(db, user_id, claimed);
            claimed_any = true;

            out.push_str(&format!("✅ 阶段{} 奖励已领取: {}\n", tier.tier, reward_desc(tier)));
        } else if already_claimed {
            out.push_str(&format!("☑️ 阶段{} 已领取: {}\n", tier.tier, reward_desc(tier)));
        } else if reached {
            out.push_str(&format!("⬜ 阶段{} 已达成（已领取）\n", tier.tier));
        } else {
            let ratio = pool_total as f64 / tier.threshold as f64;
            let percent = (ratio * 100.0) as i32;
            out.push_str(&format!("🔒 阶段{} 未达成 ({}%)\n", tier.tier, percent));
        }
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    if claimed_any {
        out.push_str("🎉 奖励已发放至背包/账户！\n");
    } else if !any_available {
        out.push_str("💡 当前无可领取的奖励\n");
        out.push_str("   继续捐献帮助全服达成更高阶段！\n");
    }

    out.push_str("💡 许愿池每周一重置，请及时领取奖励\n");
    out
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wish_tier_count() {
        assert_eq!(WISH_TIERS.len(), 5, "应有5个许愿池阶段");
    }

    #[test]
    fn test_wish_tier_thresholds_ascending() {
        for i in 1..WISH_TIERS.len() {
            assert!(
                WISH_TIERS[i].threshold > WISH_TIERS[i - 1].threshold,
                "阶段{}阈值({})应大于阶段{}阈值({})",
                WISH_TIERS[i].tier,
                WISH_TIERS[i].threshold,
                WISH_TIERS[i - 1].tier,
                WISH_TIERS[i - 1].threshold
            );
        }
    }

    #[test]
    fn test_wish_tier_values() {
        assert_eq!(WISH_TIERS[0].threshold, 100_000);
        assert_eq!(WISH_TIERS[0].reward_gold, 500);
        assert_eq!(WISH_TIERS[0].reward_exp, 100);

        assert_eq!(WISH_TIERS[1].threshold, 500_000);
        assert_eq!(WISH_TIERS[1].reward_gold, 1000);
        assert_eq!(WISH_TIERS[1].reward_diamond, 50);
        assert_eq!(WISH_TIERS[1].reward_exp, 300);

        assert_eq!(WISH_TIERS[2].threshold, 2_000_000);
        assert_eq!(WISH_TIERS[2].reward_gold, 2000);
        assert_eq!(WISH_TIERS[2].reward_diamond, 100);
        assert_eq!(WISH_TIERS[2].reward_exp, 500);
        assert_eq!(WISH_TIERS[2].reward_item, "幸运宝箱");

        assert_eq!(WISH_TIERS[3].threshold, 5_000_000);
        assert_eq!(WISH_TIERS[3].reward_gold, 5000);
        assert_eq!(WISH_TIERS[3].reward_diamond, 200);
        assert_eq!(WISH_TIERS[3].reward_exp, 1000);

        assert_eq!(WISH_TIERS[4].threshold, 10_000_000);
        assert_eq!(WISH_TIERS[4].reward_gold, 10000);
        assert_eq!(WISH_TIERS[4].reward_diamond, 500);
        assert_eq!(WISH_TIERS[4].reward_exp, 2000);
        assert_eq!(WISH_TIERS[4].reward_item, "传说宝箱");
    }

    #[test]
    fn test_djb2_hash_deterministic() {
        let h1 = djb2_hash("user123");
        let h2 = djb2_hash("user123");
        assert_eq!(h1, h2, "djb2应为确定性哈希");
    }

    #[test]
    fn test_djb2_hash_different_inputs() {
        let h1 = djb2_hash("user1");
        let h2 = djb2_hash("user2");
        assert_ne!(h1, h2, "不同输入应产生不同哈希");
    }

    #[test]
    fn test_djb2_hash_nonzero() {
        let h = djb2_hash("test");
        assert!(h > 0, "哈希值应大于0");
    }

    #[test]
    fn test_format_num_basic() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(123), "123");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1000000), "1,000,000");
        assert_eq!(format_num(10_000_000), "10,000,000");
    }

    #[test]
    fn test_format_num_negative() {
        assert_eq!(format_num(-5000), "-5,000");
        assert_eq!(format_num(-100), "-100");
    }

    #[test]
    fn test_format_num_large() {
        assert_eq!(format_num(1_234_567_890), "1,234,567,890");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(1.0, PROGRESS_WIDTH);
        assert_eq!(bar.chars().filter(|c| *c == '█').count(), PROGRESS_WIDTH);
        assert_eq!(bar.chars().filter(|c| *c == '░').count(), 0);
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0.0, PROGRESS_WIDTH);
        assert_eq!(bar.chars().filter(|c| *c == '█').count(), 0);
        assert_eq!(bar.chars().filter(|c| *c == '░').count(), PROGRESS_WIDTH);
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(0.5, PROGRESS_WIDTH);
        assert_eq!(bar.chars().filter(|c| *c == '█').count(), 5);
        assert_eq!(bar.chars().filter(|c| *c == '░').count(), 5);
    }

    #[test]
    fn test_progress_bar_clamping() {
        let bar_over = progress_bar(1.5, PROGRESS_WIDTH);
        assert_eq!(bar_over.chars().filter(|c| *c == '█').count(), PROGRESS_WIDTH);
        let bar_under = progress_bar(-0.5, PROGRESS_WIDTH);
        assert_eq!(bar_under.chars().filter(|c| *c == '█').count(), 0);
    }

    #[test]
    fn test_progress_bar_total_length() {
        let bar = progress_bar(0.7, PROGRESS_WIDTH);
        assert_eq!(bar.chars().count(), PROGRESS_WIDTH);
    }

    #[test]
    fn test_highest_reached_tier() {
        assert_eq!(highest_reached_tier(0), 0);
        assert_eq!(highest_reached_tier(50_000), 0);
        assert_eq!(highest_reached_tier(100_000), 1);
        assert_eq!(highest_reached_tier(250_000), 1);
        assert_eq!(highest_reached_tier(500_000), 2);
        assert_eq!(highest_reached_tier(1_000_000), 2);
        assert_eq!(highest_reached_tier(2_000_000), 3);
        assert_eq!(highest_reached_tier(5_000_000), 4);
        assert_eq!(highest_reached_tier(10_000_000), 5);
        assert_eq!(highest_reached_tier(20_000_000), 5);
    }

    #[test]
    fn test_next_unreached_threshold() {
        assert_eq!(next_unreached_threshold(0), Some(100_000));
        assert_eq!(next_unreached_threshold(99_999), Some(100_000));
        assert_eq!(next_unreached_threshold(100_000), Some(500_000));
        assert_eq!(next_unreached_threshold(500_000), Some(2_000_000));
        assert_eq!(next_unreached_threshold(2_000_000), Some(5_000_000));
        assert_eq!(next_unreached_threshold(5_000_000), Some(10_000_000));
        assert_eq!(next_unreached_threshold(10_000_000), None);
        assert_eq!(next_unreached_threshold(50_000_000), None);
    }

    #[test]
    fn test_reward_desc_contains_gold() {
        let desc = reward_desc(&WISH_TIERS[0]);
        assert!(desc.contains("500"), "阶段1奖励应包含500");
        assert!(desc.contains("金币"), "阶段1奖励应包含金币");
        assert!(desc.contains("100"), "阶段1奖励应包含100经验");
    }

    #[test]
    fn test_reward_desc_contains_diamond() {
        let desc = reward_desc(&WISH_TIERS[1]);
        assert!(desc.contains("钻石"), "阶段2奖励应包含钻石");
        assert!(desc.contains("50"), "阶段2奖励应包含50钻石");
    }

    #[test]
    fn test_reward_desc_contains_item() {
        let desc = reward_desc(&WISH_TIERS[2]);
        assert!(desc.contains("幸运宝箱"), "阶段3奖励应包含幸运宝箱");
    }

    #[test]
    fn test_reward_desc_rare_item() {
        let desc = reward_desc(&WISH_TIERS[4]);
        assert!(desc.contains("传说宝箱"), "阶段5奖励应包含传说宝箱");
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "wish_pool");
    }

    #[test]
    fn test_progress_width() {
        assert_eq!(PROGRESS_WIDTH, 10);
    }

    #[test]
    fn test_current_week_positive() {
        let week = current_week();
        assert!(week > 0, "当前周号应为正数");
        assert!(week > 2800, "周号应大于2800 (2023年约为2800+)");
    }

    #[test]
    fn test_claimed_mask_bit_operations() {
        // 模拟位掩码操作
        let mut mask: u32 = 0;
        assert_eq!(mask & (1u32 << 0), 0, "阶段1未领取");

        mask |= 1u32 << 0;
        assert_ne!(mask & (1u32 << 0), 0, "阶段1已领取");
        assert_eq!(mask & (1u32 << 1), 0, "阶段2未领取");

        mask |= 1u32 << 2;
        assert_ne!(mask & (1u32 << 2), 0, "阶段3已领取");
        assert_eq!(mask & (1u32 << 1), 0, "阶段2仍未领取");
        assert_eq!(mask, 0b101, "掩码应为101");
    }
}

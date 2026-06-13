/// CakeGame 玩家回归奖励系统
/// 检测玩家离线天数，发放回归奖励，鼓励玩家回归
/// 基于 Shared_Data 上次活跃时间戳检测离线时长
use crate::core::*;
use crate::db::Database;
use crate::online_activity;
use crate::user;

/// 回归等级定义
struct ReturnTier {
    /// 最低离线天数
    min_days: i32,
    /// 等级名称
    name: &'static str,
    /// emoji
    emoji: &'static str,
    /// 金币奖励
    gold: i64,
    /// 钻石奖励
    diamond: i64,
    /// 经验奖励
    exp: i32,
    /// 道具奖励（名称×数量，逗号分隔）
    items: &'static str,
}

const RETURN_TIERS: &[ReturnTier] = &[
    ReturnTier {
        min_days: 3,
        name: "短别归来",
        emoji: "🌅",
        gold: 2000,
        diamond: 20,
        exp: 500,
        items: "中生命药水×3,中魔力药水×3",
    },
    ReturnTier {
        min_days: 7,
        name: "思念成疾",
        emoji: "💫",
        gold: 8000,
        diamond: 80,
        exp: 2000,
        items: "大生命药水×5,大魔力药水×5,强化石×2",
    },
    ReturnTier {
        min_days: 14,
        name: "久别重逢",
        emoji: "🌟",
        gold: 20000,
        diamond: 200,
        exp: 5000,
        items: "超生命药水×5,超魔力药水×5,强化石×5,高级精炼水晶×1",
    },
    ReturnTier {
        min_days: 30,
        name: "王者归来",
        emoji: "👑",
        gold: 50000,
        diamond: 500,
        exp: 15000,
        items: "超生命药水×10,超魔力药水×10,强化石×10,凤凰之羽×1,时空精华×1",
    },
    ReturnTier {
        min_days: 60,
        name: "传说再现",
        emoji: "✨",
        gold: 100000,
        diamond: 1000,
        exp: 30000,
        items: "超生命药水×20,超魔力药水×20,强化石×20,凤凰之羽×3,时空精华×3,复活卷轴×1",
    },
];

const SECTION: &str = "return_reward";

/// 获取玩家离线天数
fn get_offline_days(db: &Database, user_id: &str) -> i64 {
    let last_active = online_activity::get_user_last_active(db, user_id);
    if last_active <= 0 {
        return 0;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let offline_seconds = now - last_active;
    offline_seconds / 86400
}

/// 获取匹配的回归等级
fn get_return_tier(offline_days: i64) -> Option<&'static ReturnTier> {
    let mut best: Option<&ReturnTier> = None;
    for tier in RETURN_TIERS {
        if offline_days >= tier.min_days as i64 {
            best = Some(tier);
        }
    }
    best
}

/// 查看回归奖励
pub fn cmd_view_return_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let offline_days = get_offline_days(db, user_id);

    // 检查是否已领取
    let claimed_key = format!("claimed_{}", user_id);
    let last_claimed = db.global_get(SECTION, &claimed_key);

    let mut r = format!("{}\n═══ 🎁 回归奖励系统 ═══\n", prefix);
    r.push_str(&format!("\n📅 离线天数: {}天", offline_days));

    if !last_claimed.is_empty() {
        r.push_str(&format!("\n✅ 上次领取: {}", last_claimed));
    }

    if offline_days < 3 {
        r.push_str("\n\n💡 离线满3天即可领取回归奖励！");
        r.push_str("\n\n📊 回归奖励预览:");
        for tier in RETURN_TIERS {
            r.push_str(&format!(
                "\n  {} {} (≥{}天): {}金+{}💎+{}经验",
                tier.emoji,
                tier.name,
                tier.min_days,
                format_num(tier.gold),
                tier.diamond,
                tier.exp
            ));
        }
        return r;
    }

    match get_return_tier(offline_days) {
        Some(tier) => {
            // 检查是否已领取同等级
            let tier_key = format!("tier_{}", user_id);
            let last_tier: i32 = db.global_get(SECTION, &tier_key).parse().unwrap_or(0);

            if last_tier >= tier.min_days && !last_claimed.is_empty() {
                r.push_str(&format!("\n\n{} {} — 该等级奖励已领取", tier.emoji, tier.name));
                r.push_str("\n💡 再次离线后回归可领取更高等级奖励");
            } else {
                r.push_str(&format!("\n\n{} 可领取: {}", tier.emoji, tier.name));
                r.push_str(&format!("\n  💰 金币: {}", format_num(tier.gold)));
                r.push_str(&format!("\n  💎 钻石: {}", tier.diamond));
                r.push_str(&format!("\n  ⭐ 经验: {}", tier.exp));
                r.push_str(&format!("\n  🎁 道具: {}", tier.items));
                r.push_str("\n\n📌 发送「领取回归奖励」即可领取！");
            }
        }
        None => {
            r.push_str("\n\n⚠️ 未找到匹配的回归等级");
        }
    }

    r
}

/// 领取回归奖励
pub fn cmd_claim_return_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let offline_days = get_offline_days(db, user_id);
    if offline_days < 3 {
        return format!(
            "{}\n❌ 离线不足3天，无法领取回归奖励！（当前: {}天）",
            prefix, offline_days
        );
    }

    let tier = match get_return_tier(offline_days) {
        Some(t) => t,
        None => return format!("{}\n❌ 未找到匹配的回归等级", prefix),
    };

    // 检查是否已领取同等级
    let tier_key = format!("tier_{}", user_id);
    let claimed_key = format!("claimed_{}", user_id);
    let last_tier: i32 = db.global_get(SECTION, &tier_key).parse().unwrap_or(0);

    if last_tier >= tier.min_days && !db.global_get(SECTION, &claimed_key).is_empty() {
        return format!(
            "{}\n❌ {} 奖励已领取！再次离线后可领取更高等级奖励。",
            prefix, tier.name
        );
    }

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, "add", tier.gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, "add", tier.diamond);
    let (_, leveled) = user::add_experience(db, user_id, tier.exp);

    // 发放道具
    let mut items_received = Vec::new();
    for item_entry in tier.items.split(',') {
        let item_entry = item_entry.trim();
        if let Some(pos) = item_entry.rfind('×') {
            let name = &item_entry[..pos];
            let qty: i32 = item_entry[pos + '×'.len_utf8()..].parse().unwrap_or(1);
            if db.knapsack_add(user_id, name, qty) {
                items_received.push(format!("{}×{}", name, qty));
            }
        }
    }

    // 记录领取
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    db.global_set(SECTION, &claimed_key, &now);
    db.global_set(SECTION, &tier_key, &tier.min_days.to_string());

    // 记录全服回归统计
    let total_key = "total_returns";
    let total: i64 = db.global_get(SECTION, total_key).parse().unwrap_or(0);
    db.global_set(SECTION, total_key, &(total + 1).to_string());

    // 记录贡献到全服里程碑
    crate::server_milestone::record_contribution(db, user_id, "return_claim", 1);

    let mut r = format!("{}\n═══ 🎉 回归奖励领取成功！═══\n", prefix);
    r.push_str(&format!("\n{} {} — 离线{}天", tier.emoji, tier.name, offline_days));
    r.push_str(&format!("\n\n💰 金币: +{}", format_num(tier.gold)));
    r.push_str(&format!("\n💎 钻石: +{}", tier.diamond));
    r.push_str(&format!("\n⭐ 经验: +{}", tier.exp));
    if leveled {
        r.push_str("\n🎊 恭喜升级！");
    }
    if !items_received.is_empty() {
        r.push_str(&format!("\n🎁 道具: {}", items_received.join(", ")));
    }
    r.push_str("\n\n💡 欢迎回来！继续冒险吧！");

    r
}

/// 回归排行榜
pub fn cmd_return_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut players: Vec<(String, i32, String)> = Vec::new(); // (name, tier, date)

    let all_users = db.all_users();
    for uid in &all_users {
        let tier: i32 = db.global_get(SECTION, &format!("tier_{}", uid)).parse().unwrap_or(0);
        let date = db.global_get(SECTION, &format!("claimed_{}", uid));
        if tier > 0 && !date.is_empty() {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, tier, date));
        }
    }

    if players.is_empty() {
        return format!("{}\n暂无回归奖励领取记录。", prefix);
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n═══ 🏆 回归勇士排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, tier, date)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let tier_name = RETURN_TIERS
            .iter()
            .rev()
            .find(|t| t.min_days == *tier)
            .map(|t| t.name)
            .unwrap_or("未知");
        r.push_str(&format!("\n{} {} — {}({}天) [{}]", medal, name, tier_name, tier, date));
    }

    // 用户位置
    let user_tier: i32 = db
        .global_get(SECTION, &format!("tier_{}", user_id))
        .parse()
        .unwrap_or(0);
    if user_tier > 0 {
        if let Some(rank) = players.iter().position(|(_, t, _)| *t == user_tier) {
            r.push_str(&format!("\n\n📍 你的排名：第{}名", rank + 1));
        }
    }

    // 全服统计
    let total: i64 = db.global_get(SECTION, "total_returns").parse().unwrap_or(0);
    r.push_str(&format!("\n\n📊 全服回归总次数: {}", total));

    r
}

/// 回归统计（GM用）
pub fn cmd_return_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let total: i64 = db.global_get(SECTION, "total_returns").parse().unwrap_or(0);

    let mut tier_counts = [0i32; 5];
    let all_users = db.all_users();
    for uid in &all_users {
        let tier: i32 = db.global_get(SECTION, &format!("tier_{}", uid)).parse().unwrap_or(0);
        for (i, def) in RETURN_TIERS.iter().enumerate() {
            if tier >= def.min_days {
                tier_counts[i] += 1;
            }
        }
    }

    let mut r = format!("{}\n═══ 📊 回归系统统计 ═══\n", prefix);
    r.push_str(&format!("\n📈 全服回归总次数: {}", total));
    r.push_str("\n\n📊 各等级分布:");
    for (i, tier) in RETURN_TIERS.iter().enumerate() {
        r.push_str(&format!("\n  {} {}: {}人", tier.emoji, tier.name, tier_counts[i]));
    }

    r
}

/// 格式化数字（千分位）
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_return_tiers_count() {
        assert_eq!(RETURN_TIERS.len(), 5);
    }

    #[test]
    fn test_return_tiers_sorted() {
        for i in 1..RETURN_TIERS.len() {
            assert!(RETURN_TIERS[i].min_days > RETURN_TIERS[i - 1].min_days);
        }
    }

    #[test]
    fn test_return_tiers_rewards_escalate() {
        for i in 1..RETURN_TIERS.len() {
            assert!(RETURN_TIERS[i].gold > RETURN_TIERS[i - 1].gold);
            assert!(RETURN_TIERS[i].diamond > RETURN_TIERS[i - 1].diamond);
            assert!(RETURN_TIERS[i].exp > RETURN_TIERS[i - 1].exp);
        }
    }

    #[test]
    fn test_return_tier_names() {
        let names: Vec<&str> = RETURN_TIERS.iter().map(|t| t.name).collect();
        assert!(names.contains(&"短别归来"));
        assert!(names.contains(&"王者归来"));
        assert!(names.contains(&"传说再现"));
    }

    #[test]
    fn test_return_tier_emojis() {
        for tier in RETURN_TIERS {
            assert!(!tier.emoji.is_empty());
        }
    }

    #[test]
    fn test_get_return_tier_2_days() {
        assert!(get_return_tier(2).is_none());
    }

    #[test]
    fn test_get_return_tier_3_days() {
        let tier = get_return_tier(3).unwrap();
        assert_eq!(tier.name, "短别归来");
    }

    #[test]
    fn test_get_return_tier_7_days() {
        let tier = get_return_tier(7).unwrap();
        assert_eq!(tier.name, "思念成疾");
    }

    #[test]
    fn test_get_return_tier_14_days() {
        let tier = get_return_tier(14).unwrap();
        assert_eq!(tier.name, "久别重逢");
    }

    #[test]
    fn test_get_return_tier_30_days() {
        let tier = get_return_tier(30).unwrap();
        assert_eq!(tier.name, "王者归来");
    }

    #[test]
    fn test_get_return_tier_60_days() {
        let tier = get_return_tier(60).unwrap();
        assert_eq!(tier.name, "传说再现");
    }

    #[test]
    fn test_get_return_tier_100_days() {
        let tier = get_return_tier(100).unwrap();
        assert_eq!(tier.name, "传说再现");
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234567), "1,234,567");
        assert_eq!(format_num(50000), "50,000");
        assert_eq!(format_num(100000), "100,000");
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "return_reward");
    }

    #[test]
    fn test_tier_items_not_empty() {
        for tier in RETURN_TIERS {
            assert!(!tier.items.is_empty());
            // Each item entry should have × separator
            for item in tier.items.split(',') {
                assert!(item.contains('×'), "Item '{}' missing × separator", item);
            }
        }
    }

    #[test]
    fn test_tier_min_days_positive() {
        for tier in RETURN_TIERS {
            assert!(tier.min_days > 0);
        }
    }

    #[test]
    fn test_tier_gold_positive() {
        for tier in RETURN_TIERS {
            assert!(tier.gold > 0);
        }
    }

    #[test]
    fn test_tier_diamond_positive() {
        for tier in RETURN_TIERS {
            assert!(tier.diamond > 0);
        }
    }

    #[test]
    fn test_tier_exp_positive() {
        for tier in RETURN_TIERS {
            assert!(tier.exp > 0);
        }
    }
}

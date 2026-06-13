use crate::core::*;
use crate::db::Database;
use crate::user;
use std::time::{SystemTime, UNIX_EPOCH};

/// 祈福运气等级
#[derive(Debug, Clone)]
struct LuckTier {
    name: &'static str,
    emoji: &'static str,
    gold: i64,
    diamond: i64,
    exp: i32,
    weight: u32, // 抽取权重
}

const LUCK_TIERS: &[LuckTier] = &[
    LuckTier {
        name: "大吉",
        emoji: "🎊",
        gold: 5000,
        diamond: 100,
        exp: 2000,
        weight: 5,
    },
    LuckTier {
        name: "中吉",
        emoji: "🎉",
        gold: 2000,
        diamond: 50,
        exp: 1000,
        weight: 15,
    },
    LuckTier {
        name: "小吉",
        emoji: "🌸",
        gold: 1000,
        diamond: 20,
        exp: 500,
        weight: 30,
    },
    LuckTier {
        name: "末吉",
        emoji: "🍀",
        gold: 500,
        diamond: 10,
        exp: 200,
        weight: 35,
    },
    LuckTier {
        name: "凶",
        emoji: "💀",
        gold: 200,
        diamond: 5,
        exp: 100,
        weight: 15,
    },
];

/// 获取今天日期字符串 (YYYY-MM-DD)
fn today_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 简化：用秒数计算天数偏移，加上 Unix epoch 1970-01-01
    let days = secs / 86400;
    // 简单日期计算 (足够用于游戏)
    let mut y = 1970i32;
    let mut rem = days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if rem < days_in_year {
            break;
        }
        rem -= days_in_year;
        y += 1;
    }
    let is_leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days: [u32; 12] = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0usize;
    while m < 12 && rem >= month_days[m] as u64 {
        rem -= month_days[m] as u64;
        m += 1;
    }
    format!("{:04}-{:02}-{:02}", y, m + 1, rem + 1)
}

/// 根据日期和用户ID生成确定性的运气值 (0-100)
fn daily_luck(user_id: &str, date: &str) -> u32 {
    let seed_str = format!("{}:pray:{}", user_id, date);
    let mut hash: u32 = 5381;
    for b in seed_str.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    hash % 101 // 0-100
}

/// 根据运气值选择运气等级（带VIP权重加成）
fn select_tier(luck: u32, vip_level: i32) -> &'static LuckTier {
    // VIP加成：提高大吉和中吉的概率
    let vip_bonus = vip_level as u32 * 2; // VIP1=+2%, VIP5=+10%

    let total_weight: u32 = LUCK_TIERS
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if i == 0 {
                t.weight + vip_bonus
            }
            // 大吉加成
            else if i == 1 {
                t.weight + vip_bonus / 2
            }
            // 中吉加成
            else {
                t.weight
            }
        })
        .sum();

    let roll = luck * total_weight / 100;
    let mut cumulative = 0u32;
    for (i, tier) in LUCK_TIERS.iter().enumerate() {
        let w = if i == 0 {
            tier.weight + vip_bonus
        } else if i == 1 {
            tier.weight + vip_bonus / 2
        } else {
            tier.weight
        };
        cumulative += w;
        if roll < cumulative {
            return tier;
        }
    }
    LUCK_TIERS.last().unwrap()
}

/// 查看祈福
pub fn cmd_view_pray(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);
    let date = today_string();
    let luck = daily_luck(user_id, &date);

    // 检查今天是否已祈福
    let last_pray = db.read_user_data(user_id, "pray_last_date");
    let pray_count: i32 = db.read_user_data(user_id, "pray_total_count").parse().unwrap_or(0);
    let pray_history = db.read_user_data(user_id, "pray_last_result");

    let vip_level = crate::vip::get_vip_level(db, user_id);

    let mut result = "═══ 每日祈福 ═══\n".to_string();
    result.push_str(&format!("玩家：{}\n", info.name));
    result.push_str(&format!("等级：{}\n", info.level));
    if vip_level > 0 {
        result.push_str(&format!("VIP：VIP{} (祈福加成+{}%)\n", vip_level, vip_level * 2));
    }
    result.push_str(&format!("今日运气值：{}/100\n", luck));
    result.push_str(&format!("累计祈福：{}次\n", pray_count));
    result.push_str(&format!("今日日期：{}\n", date));

    // 显示运气等级预览
    result.push_str("\n═══ 运气等级预览 ═══\n");
    for tier in LUCK_TIERS {
        result.push_str(&format!(
            "{} {} — 金币+{}, 钻石+{}, 经验+{}\n",
            tier.emoji, tier.name, tier.gold, tier.diamond, tier.exp
        ));
    }

    if last_pray == date {
        result.push_str("\n✅ 今日已祈福！\n");
        if !pray_history.is_empty() {
            result.push_str(&format!("上次结果：{}\n", pray_history));
        }
        result.push_str("明日再来吧~ 🌙");
    } else {
        result.push_str("\n🔮 发送「祈福」即可进行今日祈福！");
    }

    format!("{}\n{}", prefix, result)
}

/// 祈福
pub fn cmd_pray(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);
    let date = today_string();

    // 检查今天是否已祈福
    let last_pray = db.read_user_data(user_id, "pray_last_date");
    if last_pray == date {
        return format!("{}\n❌ 今日已经祈福过了！\n🔮 明日再来，每日只能祈福一次。", prefix);
    }

    // 检查是否阵亡
    if info.hp <= 0 {
        return format!("{}\n❌ 你已阵亡，无法祈福！请先恢复生命。", prefix);
    }

    let luck = daily_luck(user_id, &date);
    let vip_level = crate::vip::get_vip_level(db, user_id);
    let tier = select_tier(luck, vip_level);

    // VIP额外加成奖励
    let vip_gold_bonus = (tier.gold as f64 * vip_level as f64 * 0.05) as i64;
    let vip_diamond_bonus = (tier.diamond as f64 * vip_level as f64 * 0.05) as i64;

    let total_gold = tier.gold + vip_gold_bonus;
    let total_diamond = tier.diamond + vip_diamond_bonus;
    let total_exp = tier.exp;

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, total_diamond);

    // 增加经验
    let current_exp: i32 = db.read_basic(user_id, ITEM_EXP).parse().unwrap_or(0);
    db.write_basic_int(user_id, ITEM_EXP, current_exp + total_exp);

    // 记录祈福
    db.write_user_data(user_id, "pray_last_date", &date);
    db.write_user_data(user_id, "pray_last_result", &format!("{}{}", tier.emoji, tier.name));
    let count: i32 = db.read_user_data(user_id, "pray_total_count").parse().unwrap_or(0);
    db.write_user_data(user_id, "pray_total_count", &(count + 1).to_string());

    // 检查升级
    let exp_need: i32 = db.read_basic(user_id, ITEM_EXP_NEED).parse().unwrap_or(100);
    let mut level_up_msg = String::new();
    let mut new_exp = current_exp + total_exp;
    let mut new_level = info.level;
    while new_exp >= exp_need && new_level < 999 {
        new_exp -= exp_need;
        new_level += 1;
        db.write_basic_int(user_id, ITEM_LEVEL, new_level);
        db.write_basic_int(user_id, ITEM_EXP, new_exp);
        level_up_msg = format!("\n🎊 恭喜升级到 {} 级！", new_level);
    }

    let mut result = "═══ 祈福结果 ═══\n".to_string();
    result.push_str(&format!("🔮 今日运气值：{}/100\n", luck));
    result.push_str(&format!("\n{} {} 大吉大利！\n", tier.emoji, tier.name));
    result.push_str("━━━━━━━━━━━━\n");
    result.push_str(&format!("💰 获得金币：+{}\n", total_gold));
    result.push_str(&format!("💎 获得钻石：+{}\n", total_diamond));
    result.push_str(&format!("⭐ 获得经验：+{}\n", total_exp));

    if vip_gold_bonus > 0 || vip_diamond_bonus > 0 {
        result.push_str(&format!("\n🏷️ VIP{} 额外加成：\n", vip_level));
        if vip_gold_bonus > 0 {
            result.push_str(&format!("   💰 金币 +{}\n", vip_gold_bonus));
        }
        if vip_diamond_bonus > 0 {
            result.push_str(&format!("   💎 钻石 +{}\n", vip_diamond_bonus));
        }
    }

    if !level_up_msg.is_empty() {
        result.push_str(&level_up_msg);
    }

    result.push_str("\n━━━━━━━━━━━━\n");
    result.push_str(&format!("✅ 累计祈福：{}次\n", count + 1));
    result.push_str("🌅 明日再来继续祈福吧~");

    format!("{}\n{}", prefix, result)
}

/// 祈福排行
pub fn cmd_pray_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 获取所有玩家的祈福次数
    let users = db.all_users();
    let mut pray_list: Vec<(String, i32)> = Vec::new();

    for uid in &users {
        let count: i32 = db.read_user_data(uid, "pray_total_count").parse().unwrap_or(0);
        if count > 0 {
            let name = db.read_basic(uid, ITEM_NAME);
            pray_list.push((name, count));
        }
    }

    pray_list.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut result = "═══ 祈福排行 ═══\n".to_string();
    result.push_str("虔诚祈福，福运绵长 🙏\n\n");

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, count)) in pray_list.iter().take(10).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        result.push_str(&format!("{} {}. {} — {}次\n", medal, i + 1, name, count));
    }

    if pray_list.is_empty() {
        result.push_str("暂无祈福记录\n");
    }

    // 显示当前用户排名
    let my_name = db.read_basic(user_id, ITEM_NAME);
    if let Some(pos) = pray_list.iter().position(|(n, _)| *n == my_name) {
        result.push_str(&format!("\n📍 你的排名：第{}名 ({}次)", pos + 1, pray_list[pos].1));
    }

    format!("{}\n{}", prefix, result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luck_tier_definitions() {
        assert_eq!(LUCK_TIERS.len(), 5);
        assert_eq!(LUCK_TIERS[0].name, "大吉");
        assert_eq!(LUCK_TIERS[1].name, "中吉");
        assert_eq!(LUCK_TIERS[2].name, "小吉");
        assert_eq!(LUCK_TIERS[3].name, "末吉");
        assert_eq!(LUCK_TIERS[4].name, "凶");
    }

    #[test]
    fn test_luck_tier_rewards_decrease() {
        assert!(LUCK_TIERS[0].gold > LUCK_TIERS[1].gold);
        assert!(LUCK_TIERS[1].gold > LUCK_TIERS[2].gold);
        assert!(LUCK_TIERS[2].gold > LUCK_TIERS[3].gold);
        assert!(LUCK_TIERS[3].gold > LUCK_TIERS[4].gold);
    }

    #[test]
    fn test_luck_tier_weights_sum() {
        let total: u32 = LUCK_TIERS.iter().map(|t| t.weight).sum();
        assert_eq!(total, 100, "Luck tier weights should sum to 100");
    }

    #[test]
    fn test_daily_luck_deterministic() {
        let luck1 = daily_luck("user1", "2026-01-01");
        let luck2 = daily_luck("user1", "2026-01-01");
        assert_eq!(luck1, luck2);
    }

    #[test]
    fn test_daily_luck_range() {
        for i in 0..50 {
            let uid = format!("user_{}", i);
            let luck = daily_luck(&uid, "2026-06-10");
            assert!(luck <= 100);
        }
    }

    #[test]
    fn test_select_tier_no_vip() {
        for luck in 0..=100 {
            let tier = select_tier(luck, 0);
            assert!(!tier.name.is_empty());
        }
    }

    #[test]
    fn test_select_tier_vip_bonus() {
        for vip in 0..=10 {
            let tier = select_tier(50, vip);
            assert!(!tier.name.is_empty());
        }
    }

    #[test]
    fn test_today_string_format() {
        let date = today_string();
        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().nth(4).unwrap(), '-');
        assert_eq!(date.chars().nth(7).unwrap(), '-');
    }
}

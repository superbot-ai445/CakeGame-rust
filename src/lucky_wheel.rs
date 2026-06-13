/// 每日幸运转盘系统
/// 玩家每天可免费转动一次幸运转盘，VIP可额外转动
/// 转盘8个格子，奖品包括金币/钻石/经验/物品
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use crate::user;

/// 转盘格子定义
struct WheelSlot {
    name: &'static str,
    emoji: &'static str,
    reward_type: &'static str,
    amount: i64,
    weight: u32,
}

/// 8格转盘定义（权重总和=100，方便百分比计算）
const WHEEL_SLOTS: [WheelSlot; 8] = [
    WheelSlot {
        name: "金币袋",
        emoji: "💰",
        reward_type: "gold",
        amount: 500,
        weight: 25,
    },
    WheelSlot {
        name: "小经验丹",
        emoji: "💊",
        reward_type: "exp",
        amount: 100,
        weight: 25,
    },
    WheelSlot {
        name: "钻石碎片",
        emoji: "💎",
        reward_type: "diamond",
        amount: 5,
        weight: 15,
    },
    WheelSlot {
        name: "强化石",
        emoji: "🪨",
        reward_type: "item",
        amount: 1,
        weight: 10,
    },
    WheelSlot {
        name: "金币箱",
        emoji: "🎁",
        reward_type: "gold",
        amount: 2000,
        weight: 8,
    },
    WheelSlot {
        name: "大经验丹",
        emoji: "✨",
        reward_type: "exp",
        amount: 500,
        weight: 7,
    },
    WheelSlot {
        name: "钻石宝箱",
        emoji: "👑",
        reward_type: "diamond",
        amount: 20,
        weight: 5,
    },
    WheelSlot {
        name: "超级大奖",
        emoji: "🌟",
        reward_type: "gold",
        amount: 10000,
        weight: 5,
    },
];

/// 计算总权重
fn total_weight() -> u32 {
    WHEEL_SLOTS.iter().map(|s| s.weight).sum()
}

/// 根据随机值选择格子
fn pick_slot(seed: u64) -> usize {
    let total = total_weight();
    let mut roll = (seed % total as u64) as u32;
    for (i, slot) in WHEEL_SLOTS.iter().enumerate() {
        if roll < slot.weight {
            return i;
        }
        roll -= slot.weight;
    }
    0
}

/// 基于日期+用户ID生成每日种子
fn daily_seed(user_id: &str) -> u64 {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let input = format!("{}:{}", today, user_id);
    let mut hash: u64 = 5381;
    for b in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// 查看幸运转盘
pub fn cmd_view_lucky_wheel(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let spins_key = format!("lucky_wheel_spins_{}", user_id);
    let spins_today: i32 = db.global_get(&spins_key, &today).parse().unwrap_or(0);

    let is_vip = db.read_basic(user_id, "vip_level").parse::<i32>().unwrap_or(0) > 0;
    let max_spins: i32 = if is_vip { 2 } else { 1 };
    let remaining = (max_spins - spins_today).max(0);

    let mut out = format!("{}\n🎡 【每日幸运转盘】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    // 显示转盘格子
    out.push_str("┌──────────────────────┐\n");
    for (i, slot) in WHEEL_SLOTS.iter().enumerate() {
        let marker = if i == 0 { " ◀" } else { "" };
        let reward_str = match slot.reward_type {
            "gold" => format!("+{}金币", slot.amount),
            "diamond" => format!("+{}钻石", slot.amount),
            "exp" => format!("+{}经验", slot.amount),
            "item" => "×1".to_string(),
            _ => String::new(),
        };
        let pct = slot.weight * 100 / total_weight();
        out.push_str(&format!(
            "│ {} {} {} [{}%]{}\n",
            slot.emoji, slot.name, reward_str, pct, marker
        ));
    }
    out.push_str("└──────────────────────┘\n\n");

    out.push_str(&format!("📊 今日转动: {}/{} 次\n", spins_today, max_spins));

    if remaining > 0 {
        out.push_str(&format!("💡 使用「转动转盘」来试试运气吧！还剩 {} 次机会\n", remaining));
    } else {
        out.push_str("⏰ 今日转动次数已用完，明天再来吧！\n");
    }

    if !is_vip {
        out.push_str("🌟 VIP用户每天可额外转动1次！使用「VIP充值」开通\n");
    }

    out.push_str("\n📌 指令: 转动转盘 | 转盘记录\n");
    out
}

/// 转动幸运转盘
pub fn cmd_spin_lucky_wheel(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let spins_key = format!("lucky_wheel_spins_{}", user_id);
    let spins_today: i32 = db.global_get(&spins_key, &today).parse().unwrap_or(0);

    let is_vip = db.read_basic(user_id, "vip_level").parse::<i32>().unwrap_or(0) > 0;
    let max_spins: i32 = if is_vip { 2 } else { 1 };

    if spins_today >= max_spins {
        return format!(
            "{}\n🎡 今日转动次数已用完 ({}/{})\n💡 明天再来试试运气吧！",
            prefix, spins_today, max_spins
        );
    }

    // 生成随机种子（每天前N次不同）
    let base_seed = daily_seed(user_id);
    let spin_seed = base_seed.wrapping_add((spins_today as u64).wrapping_mul(7919));

    // 选择格子
    let slot_idx = pick_slot(spin_seed);
    let slot = &WHEEL_SLOTS[slot_idx];

    // 更新转动次数
    db.global_set(&spins_key, &today, &(spins_today + 1).to_string());

    // 记录历史（保留最近10条）
    let history_key = format!("lucky_wheel_history_{}", user_id);
    let record = format!("{}|{}|{}|{}", today, slot.name, slot.reward_type, slot.amount);
    let existing = db.global_get(&history_key, "records");
    let new_history = if existing.is_empty() {
        record.clone()
    } else {
        let mut records: Vec<String> = existing.split('\n').map(|s| s.to_string()).collect();
        records.push(record.clone());
        if records.len() > 10 {
            records = records[records.len() - 10..].to_vec();
        }
        records.join("\n")
    };
    db.global_set(&history_key, "records", &new_history);

    // 发放奖励
    let reward_msg = match slot.reward_type {
        "gold" => {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, slot.amount);
            format!("💰 获得 {} 金币！", slot.amount)
        }
        "diamond" => {
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, slot.amount);
            format!("💎 获得 {} 钻石！", slot.amount)
        }
        "exp" => {
            user::add_experience(db, user_id, slot.amount as i32);
            format!("✨ 获得 {} 经验！", slot.amount)
        }
        "item" => {
            db.knapsack_add(user_id, "强化石", 1);
            "🪨 获得 1 个强化石！".to_string()
        }
        _ => "🎉 什么都没中...".to_string(),
    };

    let mut out = format!("{}\n🎡 【转动幸运转盘】\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("🔄 转盘转动中...\n");
    out.push_str("·  · ·  · · ·  · · ·\n");
    out.push_str(&format!("\n🎯 指针停在了: {} {}\n\n", slot.emoji, slot.name));
    out.push_str(&format!("{}\n\n", reward_msg));

    let remaining = max_spins - spins_today - 1;
    if remaining > 0 {
        out.push_str(&format!("📊 今日剩余 {} 次转动机会\n", remaining));
        out.push_str("💡 使用「转动转盘」再来一次！\n");
    } else {
        out.push_str("📊 今日转动次数已用完\n");
        out.push_str("⏰ 明天再来吧！\n");
    }

    out
}

/// 转盘记录
pub fn cmd_wheel_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let history_key = format!("lucky_wheel_history_{}", user_id);
    let records = db.global_get(&history_key, "records");

    if records.is_empty() {
        return format!(
            "{}\n🎡 【转盘记录】\n━━━━━━━━━━━━━━━━━━━━\n📭 暂无转动记录\n💡 使用「转动转盘」试试运气！",
            prefix
        );
    }

    let mut out = format!("{}\n🎡 【转盘记录】(最近10次)\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    let mut total_gold: i64 = 0;
    let mut total_diamond: i64 = 0;
    let mut total_exp: i64 = 0;

    let lines: Vec<&str> = records.split('\n').collect();
    for (i, line) in lines.iter().enumerate().rev() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let date = parts[0];
            let name = parts[1];
            let rtype = parts[2];
            let amount: i64 = parts[3].parse().unwrap_or(0);

            let emoji = match rtype {
                "gold" => {
                    total_gold += amount;
                    "💰"
                }
                "diamond" => {
                    total_diamond += amount;
                    "💎"
                }
                "exp" => {
                    total_exp += amount;
                    "✨"
                }
                "item" => "🪨",
                _ => "❓",
            };

            let reward_str = match rtype {
                "gold" => format!("+{}金", amount),
                "diamond" => format!("+{}钻", amount),
                "exp" => format!("+{}经验", amount),
                "item" => "+1个".to_string(),
                _ => String::new(),
            };

            out.push_str(&format!("{}. [{}] {} {} ({})\n", i + 1, date, emoji, name, reward_str));
        }
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!(
        "📊 累计收益: 💰{}金 💰{}钻 ✨{}经验\n",
        total_gold, total_diamond, total_exp
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wheel_slots_count() {
        assert_eq!(WHEEL_SLOTS.len(), 8);
    }

    #[test]
    fn test_total_weight() {
        let total = total_weight();
        assert_eq!(total, 25 + 25 + 15 + 10 + 8 + 7 + 5 + 5);
        assert_eq!(total, 100);
    }

    #[test]
    fn test_slot_weights_nonzero() {
        for slot in WHEEL_SLOTS.iter() {
            assert!(slot.weight > 0, "Slot '{}' has zero weight", slot.name);
        }
    }

    #[test]
    fn test_daily_seed_deterministic() {
        let s1 = daily_seed("user1");
        let s2 = daily_seed("user1");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_daily_seed_different_users() {
        let s1 = daily_seed("user1");
        let s2 = daily_seed("user2");
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_pick_slot_range() {
        for seed in 0..200u64 {
            let idx = pick_slot(seed);
            assert!(idx < WHEEL_SLOTS.len(), "pick_slot({}) = {} out of range", seed, idx);
        }
    }

    #[test]
    fn test_slot_reward_types() {
        let valid_types = ["gold", "diamond", "exp", "item", "nothing"];
        for slot in WHEEL_SLOTS.iter() {
            assert!(
                valid_types.contains(&slot.reward_type),
                "Slot '{}' has invalid type '{}'",
                slot.name,
                slot.reward_type
            );
        }
    }

    #[test]
    fn test_slot_amounts_positive() {
        for slot in WHEEL_SLOTS.iter() {
            if slot.reward_type != "nothing" {
                assert!(slot.amount > 0, "Slot '{}' has non-positive amount", slot.name);
            }
        }
    }

    #[test]
    fn test_wheel_slot_names_unique() {
        let mut names: Vec<&str> = WHEEL_SLOTS.iter().map(|s| s.name).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len, "Duplicate slot names found");
    }

    #[test]
    fn test_pick_slot_distribution() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..500u64 {
            seen.insert(pick_slot(seed));
        }
        assert!(seen.len() >= 6, "Only saw {} unique slots in 500 rolls", seen.len());
    }
}

use crate::core::*;
use crate::db::Database;
use crate::user;
use std::time::{SystemTime, UNIX_EPOCH};

/// 保底计数器 key 前缀
const PITY_KEY_PREFIX: &str = "lottery_pity_";

/// 保底阈值：连续N次未获得装备，第N+1次必出装备
const PITY_THRESHOLDS: [u32; 4] = [
    20, // 普通池：20次保底
    15, // 高级池：15次保底
    10, // 至尊池：10次保底
    8,  // 暗黑池：8次保底
];

/// 抽奖池类型
#[derive(Debug, Clone)]
struct LotteryPool {
    name: &'static str,
    emoji: &'static str,
    cost_gold: i64,
    cost_diamond: i64,
    desc: &'static str,
}

const POOLS: &[LotteryPool] = &[
    LotteryPool {
        name: "普通池",
        emoji: "🎰",
        cost_gold: 500,
        cost_diamond: 0,
        desc: "500金币/次，包含各类装备、药剂、材料",
    },
    LotteryPool {
        name: "高级池",
        emoji: "💎",
        cost_gold: 0,
        cost_diamond: 30,
        desc: "30钻石/次，高品质装备概率更高",
    },
    LotteryPool {
        name: "至尊池",
        emoji: "👑",
        cost_gold: 0,
        cost_diamond: 100,
        desc: "100钻石/次，保底精良品质以上装备",
    },
    LotteryPool {
        name: "暗黑池",
        emoji: "🌑",
        cost_gold: 0,
        cost_diamond: 200,
        desc: "200钻石/次，传说级装备概率最高，8次保底",
    },
];

/// 根据种子生成确定性随机数
fn pseudo_random(seed: &str, max: u32) -> u32 {
    let mut hash: u32 = 5381;
    for b in seed.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    hash % max
}

/// 获取当前毫秒时间戳
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 物品权重分类
struct ItemCategory {
    name: &'static str,
    db_type: &'static str,
    weight: u32,         // 普通池权重
    high_weight: u32,    // 高级池权重
    supreme_weight: u32, // 至尊池权重
    dark_weight: u32,    // 暗黑池权重
}

const CATEGORIES: &[ItemCategory] = &[
    ItemCategory {
        name: "装备",
        db_type: "Equip",
        weight: 15,
        high_weight: 25,
        supreme_weight: 40,
        dark_weight: 50,
    },
    ItemCategory {
        name: "药剂",
        db_type: "potion",
        weight: 35,
        high_weight: 30,
        supreme_weight: 20,
        dark_weight: 10,
    },
    ItemCategory {
        name: "材料",
        db_type: "material",
        weight: 30,
        high_weight: 25,
        supreme_weight: 20,
        dark_weight: 10,
    },
    ItemCategory {
        name: "礼包",
        db_type: "GiftBag",
        weight: 10,
        high_weight: 10,
        supreme_weight: 10,
        dark_weight: 15,
    },
    ItemCategory {
        name: "技能书",
        db_type: "技能书",
        weight: 10,
        high_weight: 10,
        supreme_weight: 10,
        dark_weight: 15,
    },
];

/// 获取保底计数
fn get_pity_count(db: &Database, user_id: &str, pool_idx: usize) -> u32 {
    let key = format!("{}{}", PITY_KEY_PREFIX, pool_idx);
    db.read_user_data(user_id, &key).parse().unwrap_or(0)
}

/// 增加保底计数
fn increment_pity(db: &Database, user_id: &str, pool_idx: usize) -> u32 {
    let count = get_pity_count(db, user_id, pool_idx) + 1;
    let key = format!("{}{}", PITY_KEY_PREFIX, pool_idx);
    db.write_user_data(user_id, &key, &count.to_string());
    count
}

/// 重置保底计数
fn reset_pity(db: &Database, user_id: &str, pool_idx: usize) {
    let key = format!("{}{}", PITY_KEY_PREFIX, pool_idx);
    db.write_user_data(user_id, &key, "0");
}

/// 从奖池中抽取一个物品（带保底支持）
fn draw_item_with_pity(db: &Database, user_id: &str, pool_idx: usize, roll: u32) -> (Option<(String, String)>, bool) {
    let pity_count = get_pity_count(db, user_id, pool_idx);
    let threshold = PITY_THRESHOLDS.get(pool_idx).copied().unwrap_or(20);
    let pity_triggered = pity_count >= threshold;

    if pity_triggered {
        // 保底触发：强制选装备类别
        let items: Vec<(String, String)> = db
            .lock_conn()
            .prepare("SELECT ID, Name FROM Config_Goods WHERE Type = 'Equip'")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                    .ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
            })
            .unwrap_or_default();

        if !items.is_empty() {
            // 使用保底计数器作为种子的一部分确保不同结果
            let idx = (pity_count as usize + roll as usize) % items.len();
            let (id, name) = items[idx].clone();
            return (Some((id, format!("[装备]{} ★保底★", name))), true);
        }
    }

    // 正常抽奖
    let result = draw_item(db, pool_idx, roll);

    // 检查是否抽到装备，决定是否增加/重置保底计数
    let is_equip = result
        .as_ref()
        .map(|(_, display)| display.starts_with("[装备]"))
        .unwrap_or(false);
    if is_equip {
        reset_pity(db, user_id, pool_idx);
    } else {
        increment_pity(db, user_id, pool_idx);
    }

    (result, false)
}

/// 从奖池中抽取一个物品
fn draw_item(db: &Database, pool_idx: usize, roll: u32) -> Option<(String, String)> {
    // 按权重选择类别
    let total_weight: u32 = CATEGORIES
        .iter()
        .map(|c| match pool_idx {
            0 => c.weight,
            1 => c.high_weight,
            2 => c.supreme_weight,
            3 => c.dark_weight,
            _ => c.weight,
        })
        .sum();

    let scaled_roll = roll * total_weight / 100;
    let mut cumulative = 0u32;
    let mut selected_type = CATEGORIES[0].db_type;
    let mut selected_name = CATEGORIES[0].name;

    for cat in CATEGORIES {
        let w = match pool_idx {
            0 => cat.weight,
            1 => cat.high_weight,
            2 => cat.supreme_weight,
            3 => cat.dark_weight,
            _ => cat.weight,
        };
        cumulative += w;
        if scaled_roll < cumulative {
            selected_type = cat.db_type;
            selected_name = cat.name;
            break;
        }
    }

    // 从该类别中随机选择一个物品
    let items: Vec<(String, String)> = db
        .lock_conn()
        .prepare("SELECT ID, Name FROM Config_Goods WHERE Type = ?1")
        .ok()?
        .query_map([selected_type], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();

    if items.is_empty() {
        return None;
    }

    let idx = roll as usize % items.len();
    let (id, name) = items[idx].clone();
    Some((id, format!("[{}]{}", selected_name, name)))
}

/// 记录抽奖历史
fn record_draw(db: &Database, user_id: &str, pool_name: &str, item_name: &str) {
    let ts = now_millis();
    let key = format!("lottery_history_{}", ts);
    let value = format!("{}|{}|{}", pool_name, item_name, ts);
    db.write_user_data(user_id, &key, &value);

    // 清理旧记录（保留最近20条）
    let prefix = "lottery_history_";
    let all_keys: Vec<String> = db
        .lock_conn()
        .prepare("SELECT Key FROM Shared_Data WHERE User = ?1 AND Key LIKE ?2")
        .ok()
        .and_then(|mut stmt| {
            let pattern = format!("{}%", prefix);
            stmt.query_map([user_id, &pattern], |row| row.get::<_, String>(0))
                .ok()
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    if all_keys.len() > 20 {
        let mut sorted = all_keys;
        sorted.sort();
        for key in sorted.iter().take(sorted.len() - 20) {
            db.write_user_data(user_id, key, "");
        }
    }
}

/// 查看抽奖
pub fn cmd_view_lottery(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);
    let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);

    let mut result = "═══ 抽奖系统 ═══\n".to_string();
    result.push_str(&format!("玩家：{} (等级{})\n", info.name, info.level));
    result.push_str(&format!("💰 金币：{}  💎 钻石：{}\n\n", gold, diamond));

    for (i, pool) in POOLS.iter().enumerate() {
        let cost_str = if pool.cost_gold > 0 {
            format!("{}金币", pool.cost_gold)
        } else {
            format!("{}钻石", pool.cost_diamond)
        };
        let can_afford = if pool.cost_gold > 0 {
            gold >= pool.cost_gold
        } else {
            diamond >= pool.cost_diamond
        };
        let afford_icon = if can_afford { "✅" } else { "❌" };
        result.push_str(&format!(
            "{} {}. {}{} — {} {}\n",
            pool.emoji,
            i + 1,
            pool.name,
            afford_icon,
            cost_str,
            pool.desc
        ));
    }

    result.push_str("\n═══ 使用说明 ═══\n");
    result.push_str("• 抽奖+普通池 — 花费500金币抽奖\n");
    result.push_str("• 抽奖+高级池 — 花费30钻石抽奖\n");
    result.push_str("• 抽奖+至尊池 — 花费100钻石抽奖\n");
    result.push_str("• 抽奖+暗黑池 — 花费200钻石抽奖（装备概率50%，8次保底）\n");
    result.push_str("• 十连抽+池名 — 10连抽(普通池5000金/高级池300钻/至尊池1000钻/暗黑池2000钻)\n");
    result.push_str("• 抽奖记录 — 查看最近抽奖历史\n");
    result.push_str("\n📊 物品池概率：\n");
    result.push_str("  装备 | 药剂 | 材料 | 礼包 | 技能书\n");

    format!("{}\n{}", prefix, result)
}

/// 抽奖
pub fn cmd_lottery(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);

    if info.hp <= 0 {
        return format!("{}\n❌ 你已阵亡，无法抽奖！请先恢复生命。", prefix);
    }

    let pool_name = args.trim();
    if pool_name.is_empty() {
        return format!(
            "{}\n❌ 请指定奖池名称！\n💡 可选：普通池 / 高级池 / 至尊池 / 暗黑池",
            prefix
        );
    }

    // 模糊匹配奖池
    let pool_idx = if pool_name.contains("普通") || pool_name == "1" {
        0
    } else if pool_name.contains("高级") || pool_name == "2" {
        1
    } else if pool_name.contains("至尊") || pool_name == "3" {
        2
    } else if pool_name.contains("暗黑") || pool_name.contains("dark") || pool_name == "4" {
        3
    } else {
        return format!(
            "{}\n❌ 未找到奖池「{}」！\n💡 可选：普通池 / 高级池 / 至尊池 / 暗黑池",
            prefix, pool_name
        );
    };

    let pool = &POOLS[pool_idx];

    // 检查货币
    if pool.cost_gold > 0 {
        let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        if gold < pool.cost_gold {
            return format!("{}\n💰 金币不足！需要{}，当前{}", prefix, pool.cost_gold, gold);
        }
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, pool.cost_gold);
    } else {
        let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
        if diamond < pool.cost_diamond {
            return format!("{}\n💎 钻石不足！需要{}，当前{}", prefix, pool.cost_diamond, diamond);
        }
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, pool.cost_diamond);
    }

    // 抽奖（带保底）
    let seed = format!("{}:lottery:{}:{}", user_id, pool_idx, now_millis());
    let roll = pseudo_random(&seed, 100);

    let (result, pity_triggered) = draw_item_with_pity(db, user_id, pool_idx, roll);
    let item_display = result.map(|(_, name)| name).unwrap_or_else(|| "神秘物品".to_string());

    // 记录
    record_draw(db, user_id, pool.name, &item_display);

    // 给予物品（直接放入背包）
    let item_id = item_display.split(']').next_back().unwrap_or(&item_display);
    // 尝试找到实际物品ID
    let actual_id: Option<String> = db
        .lock_conn()
        .prepare("SELECT ID FROM Config_Goods WHERE Name = ?1 LIMIT 1")
        .ok()
        .and_then(|mut stmt| stmt.query_row([item_id], |row| row.get::<_, String>(0)).ok());

    if let Some(ref iid) = actual_id {
        // 添加到背包
        db.lock_conn().execute(
            "INSERT INTO Basic_knapsack (ID, Node, Item, Count, Lock, ItemData) VALUES (?1, ?2, ?3, 1, 'FALSE', '{}')",
            rusqlite::params![user_id, "backpack", iid],
        ).ok();
    }

    let mut result = "═══ 抽奖结果 ═══\n".to_string();
    result.push_str(&format!("🎰 奖池：{}{}\n", pool.emoji, pool.name));
    result.push_str(&format!("🎲 运势值：{}/100\n\n", roll));

    // 保底触发提示
    if pity_triggered {
        result.push_str("🛡️ 保底触发！连续多次未获得装备，本次保底出装备！\n\n");
    }

    // 根据运势显示不同特效
    let effect = if roll >= 90 {
        "✨✨✨ 金光闪闪！✨✨✨"
    } else if roll >= 70 {
        "🌟 运气不错！🌟"
    } else if roll >= 50 {
        "🍀 中规中矩"
    } else {
        "💫 略显遗憾..."
    };
    result.push_str(&format!("{}\n\n", effect));

    result.push_str(&format!("🎁 获得：{}\n", item_display));

    if let Some(ref iid) = actual_id {
        result.push_str(&format!("📦 已放入背包 (ID: {})\n", iid));
    }

    // 显示剩余货币
    let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
    result.push_str(&format!("\n💰 剩余金币：{}  💎 剩余钻石：{}", gold, diamond));

    format!("{}\n{}", prefix, result)
}

/// 十连抽
pub fn cmd_lottery_ten(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);

    if info.hp <= 0 {
        return format!("{}\n❌ 你已阵亡，无法抽奖！请先恢复生命。", prefix);
    }

    let pool_name = args.trim();
    if pool_name.is_empty() {
        return format!(
            "{}\n❌ 请指定奖池名称！\n💡 可选：十连抽+普通池 / 十连抽+高级池 / 十连抽+至尊池 / 十连抽+暗黑池",
            prefix
        );
    }

    let pool_idx = if pool_name.contains("普通") || pool_name == "1" {
        0
    } else if pool_name.contains("高级") || pool_name == "2" {
        1
    } else if pool_name.contains("至尊") || pool_name == "3" {
        2
    } else if pool_name.contains("暗黑") || pool_name.contains("dark") || pool_name == "4" {
        3
    } else {
        return format!(
            "{}\n❌ 未找到奖池「{}」！\n💡 可选：十连抽+普通池 / 十连抽+高级池 / 十连抽+至尊池 / 十连抽+暗黑池",
            prefix, pool_name
        );
    };

    let pool = &POOLS[pool_idx];
    let total_cost_gold = pool.cost_gold * 10;
    let total_cost_diamond = pool.cost_diamond * 10;

    // 检查货币
    if total_cost_gold > 0 {
        let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        if gold < total_cost_gold {
            return format!("{}\n💰 金币不足！十连抽需要{}，当前{}", prefix, total_cost_gold, gold);
        }
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, total_cost_gold);
    } else {
        let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
        if diamond < total_cost_diamond {
            return format!(
                "{}\n💎 钻石不足！十连抽需要{}，当前{}",
                prefix, total_cost_diamond, diamond
            );
        }
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, total_cost_diamond);
    }

    // 十连抽（带保底）
    let mut results: Vec<(String, u32)> = Vec::new();
    let mut best_roll = 0u32;
    let mut pity_count = 0u32;
    for i in 0..10u32 {
        let seed = format!("{}:lottery10:{}:{}:{}", user_id, pool_idx, now_millis(), i);
        let roll = pseudo_random(&seed, 100);
        let (result, pity_triggered) = draw_item_with_pity(db, user_id, pool_idx, roll);
        let item_display = result.map(|(_, name)| name).unwrap_or_else(|| "神秘物品".to_string());
        if pity_triggered {
            pity_count += 1;
        }
        if roll > best_roll {
            best_roll = roll;
        }
        results.push((item_display, roll));

        // 给予物品
        let item_id = results.last().unwrap().0.split(']').next_back().unwrap_or("unknown");
        let actual_id: Option<String> = db
            .lock_conn()
            .prepare("SELECT ID FROM Config_Goods WHERE Name = ?1 LIMIT 1")
            .ok()
            .and_then(|mut stmt| stmt.query_row([item_id], |row| row.get::<_, String>(0)).ok());

        if let Some(ref iid) = actual_id {
            db.lock_conn().execute(
                "INSERT INTO Basic_knapsack (ID, Node, Item, Count, Lock, ItemData) VALUES (?1, ?2, ?3, 1, 'FALSE', '{}')",
                rusqlite::params![user_id, "backpack", iid],
            ).ok();
        }

        record_draw(db, user_id, pool.name, &results.last().unwrap().0);
    }

    let mut result = format!("═══ 十连抽结果 — {}{} ═══\n", pool.emoji, pool.name);
    result.push_str(&format!("🎲 最高运势值：{}/100\n\n", best_roll));

    for (i, (item, roll)) in results.iter().enumerate() {
        let star = if *roll >= 90 {
            "⭐"
        } else if *roll >= 70 {
            "✨"
        } else {
            "  "
        };
        result.push_str(&format!("{} {}. {} (运势{})\n", star, i + 1, item, roll));
    }

    let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
    result.push_str(&format!("\n💰 剩余：{}金币  {}钻石", gold, diamond));

    // 保底触发提示
    if pity_count > 0 {
        result.push_str(&format!("\n\n🛡️ 本次十连抽触发{}次保底！", pity_count));
    }

    format!("{}\n{}", prefix, result)
}

/// 抽奖记录
pub fn cmd_lottery_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let prefix_key = "lottery_history_";
    let records: Vec<(String, String)> = db
        .lock_conn()
        .prepare("SELECT Key, Value FROM Shared_Data WHERE User = ?1 AND Key LIKE ?2 ORDER BY Key DESC LIMIT 10")
        .ok()
        .and_then(|mut stmt| {
            let pattern = format!("{}%", prefix_key);
            stmt.query_map(rusqlite::params![user_id, pattern], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .unwrap_or_default();

    let mut result = "═══ 抽奖记录 ═══\n".to_string();

    if records.is_empty() {
        result.push_str("暂无抽奖记录\n");
        result.push_str("💡 发送「抽奖+池名」开始抽奖！");
    } else {
        result.push_str("最近10次抽奖：\n\n");
        for (i, (_key, value)) in records.iter().enumerate() {
            let parts: Vec<&str> = value.split('|').collect();
            if parts.len() >= 2 {
                result.push_str(&format!("{}. [{}] {}\n", i + 1, parts[0], parts[1]));
            }
        }
    }

    format!("{}\n{}", prefix, result)
}

/// 保底信息
pub fn cmd_pity_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut result = "═══ 保底系统 ═══\n".to_string();
    result.push_str("🛡️ 连续抽奖未获得装备时，达到保底次数必出装备！\n\n");

    for (i, pool) in POOLS.iter().enumerate() {
        let count = get_pity_count(db, user_id, i);
        let threshold = PITY_THRESHOLDS.get(i).copied().unwrap_or(20);
        let progress = (count as f64 / threshold as f64 * 10.0) as u32;
        let bar: String = (0..10).map(|j| if j < progress { "█" } else { "░" }).collect();
        let remaining = threshold.saturating_sub(count);

        result.push_str(&format!(
            "{} {} — {}/{}次 {} {}\n",
            pool.emoji,
            pool.name,
            count,
            threshold,
            bar,
            if remaining == 0 {
                "✅ 下次必出装备！".to_string()
            } else {
                format!("还差{}次", remaining)
            }
        ));
    }

    result.push_str("\n💡 每次抽到装备自动重置保底计数");
    result.push_str("\n💡 保底信息 — 查看保底进度");

    format!("{}\n{}", prefix, result)
}

/// 暗黑抽奖（快捷入口，默认使用暗黑池）
pub fn cmd_dark_lottery(db: &Database, user_id: &str, args: &str, msg_type: &str, group: &str) -> String {
    let effective_args = if args.trim().is_empty() { "暗黑池" } else { args };
    cmd_lottery(db, user_id, effective_args, msg_type, group)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_pool_exists() {
        assert_eq!(POOLS.len(), 4);
        assert_eq!(POOLS[3].name, "暗黑池");
        assert_eq!(POOLS[3].cost_diamond, 200);
        assert_eq!(POOLS[3].cost_gold, 0);
    }

    #[test]
    fn test_pity_thresholds() {
        assert_eq!(PITY_THRESHOLDS.len(), 4);
        assert_eq!(PITY_THRESHOLDS[3], 8); // 暗黑池8次保底
    }

    #[test]
    fn test_dark_weights() {
        // 暗黑池装备权重最高(50)
        let equip = CATEGORIES.iter().find(|c| c.name == "装备").unwrap();
        assert_eq!(equip.dark_weight, 50);
        assert!(equip.dark_weight > equip.supreme_weight);
    }

    #[test]
    fn test_dark_lottery_emoji() {
        assert_eq!(POOLS[3].emoji, "🌑");
    }

    #[test]
    fn test_pool_idx_dark() {
        // 验证暗黑池索引
        assert_eq!(POOLS[3].name, "暗黑池");
    }
}

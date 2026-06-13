/// CakeGame 宝箱系统
/// 来源: MessageTemplate `扩展_查询宝箱列表`
/// 开启宝箱获取随机奖励：装备、药剂、材料、金币、钻石
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 宝箱等级定义
struct ChestDef {
    name: &'static str,
    emoji: &'static str,
    cost_gold: i64,
    cost_diamond: i64,
    key_item: &'static str,
    min_level: i32,
    desc: &'static str,
    // 奖励池: (物品名, 权重)
    gold_min: i64,
    gold_max: i64,
    diamond_min: i32,
    diamond_max: i32,
}

const CHESTS: &[ChestDef] = &[
    ChestDef {
        name: "铜宝箱",
        emoji: "🟤",
        cost_gold: 500,
        cost_diamond: 0,
        key_item: "铜钥匙",
        min_level: 1,
        desc: "基础宝箱，可开出药剂和基础材料",
        gold_min: 100,
        gold_max: 800,
        diamond_min: 0,
        diamond_max: 5,
    },
    ChestDef {
        name: "银宝箱",
        emoji: "⬜",
        cost_gold: 2000,
        cost_diamond: 0,
        key_item: "银钥匙",
        min_level: 10,
        desc: "中级宝箱，可开出装备和稀有材料",
        gold_min: 500,
        gold_max: 3000,
        diamond_min: 2,
        diamond_max: 15,
    },
    ChestDef {
        name: "金宝箱",
        emoji: "🟨",
        cost_gold: 0,
        cost_diamond: 50,
        key_item: "金钥匙",
        min_level: 20,
        desc: "高级宝箱，可开出稀有装备和高级材料",
        gold_min: 2000,
        gold_max: 10000,
        diamond_min: 10,
        diamond_max: 50,
    },
    ChestDef {
        name: "至尊宝箱",
        emoji: "🟪",
        cost_gold: 0,
        cost_diamond: 200,
        key_item: "至尊钥匙",
        min_level: 30,
        desc: "传说宝箱，最高概率获得史诗级物品",
        gold_min: 5000,
        gold_max: 30000,
        diamond_min: 30,
        diamond_max: 150,
    },
];

/// 奖励物品池
struct RewardItem {
    name: &'static str,
    weight_copper: u32,
    weight_silver: u32,
    weight_gold: u32,
    weight_supreme: u32,
    max_count: i32,
}

const REWARD_POOL: &[RewardItem] = &[
    // 药剂类
    RewardItem {
        name: "【普通】生命药水",
        weight_copper: 30,
        weight_silver: 20,
        weight_gold: 10,
        weight_supreme: 5,
        max_count: 3,
    },
    RewardItem {
        name: "【普通】魔力药水",
        weight_copper: 25,
        weight_silver: 18,
        weight_gold: 10,
        weight_supreme: 5,
        max_count: 3,
    },
    RewardItem {
        name: "【精良】大生命药水",
        weight_copper: 5,
        weight_silver: 20,
        weight_gold: 15,
        weight_supreme: 10,
        max_count: 2,
    },
    RewardItem {
        name: "【精良】大魔力药水",
        weight_copper: 5,
        weight_silver: 18,
        weight_gold: 15,
        weight_supreme: 10,
        max_count: 2,
    },
    RewardItem {
        name: "【稀有】超生命药水",
        weight_copper: 0,
        weight_silver: 5,
        weight_gold: 15,
        weight_supreme: 12,
        max_count: 2,
    },
    RewardItem {
        name: "【稀有】护肝药剂",
        weight_copper: 0,
        weight_silver: 3,
        weight_gold: 10,
        weight_supreme: 10,
        max_count: 1,
    },
    // 材料类
    RewardItem {
        name: "强化石",
        weight_copper: 15,
        weight_silver: 20,
        weight_gold: 15,
        weight_supreme: 10,
        max_count: 3,
    },
    RewardItem {
        name: "白色精粹",
        weight_copper: 10,
        weight_silver: 10,
        weight_gold: 5,
        weight_supreme: 3,
        max_count: 2,
    },
    RewardItem {
        name: "红色精粹",
        weight_copper: 3,
        weight_silver: 10,
        weight_gold: 10,
        weight_supreme: 8,
        max_count: 2,
    },
    RewardItem {
        name: "蓝色精粹",
        weight_copper: 3,
        weight_silver: 10,
        weight_gold: 10,
        weight_supreme: 8,
        max_count: 2,
    },
    RewardItem {
        name: "紫色精粹",
        weight_copper: 0,
        weight_silver: 3,
        weight_gold: 10,
        weight_supreme: 10,
        max_count: 1,
    },
    RewardItem {
        name: "虚空宝石",
        weight_copper: 0,
        weight_silver: 1,
        weight_gold: 5,
        weight_supreme: 8,
        max_count: 1,
    },
    RewardItem {
        name: "史诗碎片",
        weight_copper: 0,
        weight_silver: 0,
        weight_gold: 3,
        weight_supreme: 8,
        max_count: 1,
    },
    RewardItem {
        name: "远古超界石",
        weight_copper: 0,
        weight_silver: 0,
        weight_gold: 1,
        weight_supreme: 5,
        max_count: 1,
    },
    // 礼包类
    RewardItem {
        name: "初级礼包",
        weight_copper: 5,
        weight_silver: 5,
        weight_gold: 3,
        weight_supreme: 2,
        max_count: 1,
    },
    RewardItem {
        name: "中级礼包",
        weight_copper: 0,
        weight_silver: 3,
        weight_gold: 5,
        weight_supreme: 5,
        max_count: 1,
    },
    RewardItem {
        name: "高级礼包",
        weight_copper: 0,
        weight_silver: 0,
        weight_gold: 2,
        weight_supreme: 5,
        max_count: 1,
    },
];

/// 基于时间戳+用户ID的伪随机
fn chest_random(seed: &str, max: u32) -> u32 {
    let mut hash: u32 = 5381;
    for b in seed.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    hash % max.max(1)
}

/// 计算总权重
fn total_weight(chest_idx: usize) -> u32 {
    REWARD_POOL
        .iter()
        .map(|r| match chest_idx {
            0 => r.weight_copper,
            1 => r.weight_silver,
            2 => r.weight_gold,
            3 => r.weight_supreme,
            _ => 0,
        })
        .sum()
}

/// 从奖励池中按权重选取
fn pick_reward(seed: &str, chest_idx: usize) -> Option<(&'static RewardItem, i32)> {
    let total = total_weight(chest_idx);
    if total == 0 {
        return None;
    }
    let roll = chest_random(seed, total);
    let mut acc: u32 = 0;
    for item in REWARD_POOL.iter() {
        let w = match chest_idx {
            0 => item.weight_copper,
            1 => item.weight_silver,
            2 => item.weight_gold,
            3 => item.weight_supreme,
            _ => 0,
        };
        if w == 0 {
            continue;
        }
        acc += w;
        if roll < acc {
            // 数量随机
            let count_seed = format!("{}count", seed);
            let count = if item.max_count > 1 {
                (chest_random(&count_seed, item.max_count as u32) as i32) + 1
            } else {
                1
            };
            return Some((item, count));
        }
    }
    None
}

/// 查看宝箱列表
pub fn cmd_view_chests(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let user_lv = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let gold = db.read_currency(user_id, "gold");
    let diamond = db.read_currency(user_id, "diamond");

    let mut out = format!("{}\n", prefix);
    out += "╔══════════════════════╗\n";
    out += "║    🎁 宝箱商店 🎁    ║\n";
    out += "╚══════════════════════╝\n\n";
    out += &format!("💰 金币: {} | 💎 钻石: {}\n", gold, diamond);
    out += &format!("📊 等级: {}\n\n", user_lv);

    for (i, chest) in CHESTS.iter().enumerate() {
        let lock = if user_lv < chest.min_level { "🔒" } else { "🔓" };
        let cost_str = if chest.cost_gold > 0 {
            format!("{}金币", chest.cost_gold)
        } else {
            format!("{}钻石", chest.cost_diamond)
        };
        let affordable = if chest.cost_gold > 0 {
            gold >= chest.cost_gold
        } else {
            diamond >= chest.cost_diamond
        };
        let status = if user_lv < chest.min_level {
            "🔒等级不足"
        } else if affordable {
            "✅可开启"
        } else {
            "⏳货币不足"
        };

        // 统计拥有的钥匙
        let key_count = db.get_item_count(user_id, chest.key_item);

        out += &format!(
            "{} {}. [{}] - {} ({})\n",
            chest.emoji,
            i + 1,
            chest.name,
            cost_str,
            status
        );
        out += &format!(
            "   {} 等级要求: Lv.{} | 🔑{}: {}个\n",
            lock, chest.min_level, chest.key_item, key_count
        );
        out += &format!("   📝 {}\n", chest.desc);

        // 显示可能的奖励预览
        out += "   🎯 可能获得: ";
        let mut previews: Vec<&str> = Vec::new();
        for r in REWARD_POOL.iter() {
            let w = match i {
                0 => r.weight_copper,
                1 => r.weight_silver,
                2 => r.weight_gold,
                3 => r.weight_supreme,
                _ => 0,
            };
            if w > 0 && previews.len() < 4 {
                previews.push(r.name);
            }
        }
        out += &previews.join("/");
        out += "...\n\n";
    }

    out += "━━━━━━━━━━━━━━━━━━━━\n";
    out += "📌 发送「开启宝箱+宝箱名」打开宝箱\n";
    out += "🔑 钥匙可通过击杀怪物或商店购买获得\n";
    out += "💡 高级宝箱有概率获得稀有装备和材料\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chest_random_deterministic() {
        // Same seed gives same result
        let a = chest_random("test_seed_1", 100);
        let b = chest_random("test_seed_1", 100);
        assert_eq!(a, b);
    }

    #[test]
    fn test_chest_random_range() {
        for i in 0..100 {
            let seed = format!("seed_{}", i);
            let val = chest_random(&seed, 50);
            assert!(val < 50, "chest_random({}) = {} should be < 50", seed, val);
        }
    }

    #[test]
    fn test_chest_random_different_seeds() {
        // Different seeds should produce different values (usually)
        let mut results = std::collections::HashSet::new();
        for i in 0..20 {
            let seed = format!("unique_{}", i);
            results.insert(chest_random(&seed, 1000));
        }
        // At least 15 out of 20 should be unique
        assert!(results.len() >= 15, "Only {} unique values out of 20", results.len());
    }

    #[test]
    fn test_chest_definitions() {
        assert_eq!(CHESTS.len(), 4);
        assert_eq!(CHESTS[0].name, "铜宝箱");
        assert_eq!(CHESTS[1].name, "银宝箱");
        assert_eq!(CHESTS[2].name, "金宝箱");
        assert_eq!(CHESTS[3].name, "至尊宝箱");
        // Level requirements increase
        assert!(CHESTS[0].min_level <= CHESTS[1].min_level);
        assert!(CHESTS[1].min_level <= CHESTS[2].min_level);
        assert!(CHESTS[2].min_level <= CHESTS[3].min_level);
    }

    #[test]
    fn test_total_weight_nonzero() {
        for i in 0..4 {
            let w = total_weight(i);
            assert!(w > 0, "total_weight({}) should be > 0", i);
        }
    }

    #[test]
    fn test_total_weight_increases_with_tier() {
        // Higher-tier chests generally have more reward options
        let copper = total_weight(0);
        let supreme = total_weight(3);
        assert!(supreme >= copper, "Supreme ({}) >= Copper ({})", supreme, copper);
    }

    #[test]
    fn test_pick_reward_returns_item() {
        // Should always return Some for valid chest indices with nonzero weights
        let mut got_item = false;
        for i in 0..100 {
            let seed = format!("pick_test_{}", i);
            if let Some((item, count)) = pick_reward(&seed, 0) {
                assert!(count >= 1);
                assert!(count <= item.max_count);
                got_item = true;
                break;
            }
        }
        assert!(got_item, "pick_reward should return at least one item in 100 tries");
    }

    #[test]
    fn test_reward_pool_not_empty() {
        assert!(REWARD_POOL.len() >= 10, "REWARD_POOL should have at least 10 items");
    }

    #[test]
    fn test_reward_pool_weights() {
        // Copper items should have copper weight > 0 for at least some items
        let copper_items: Vec<_> = REWARD_POOL.iter().filter(|r| r.weight_copper > 0).collect();
        assert!(copper_items.len() >= 3, "At least 3 items available in copper chests");
        // Supreme should have more items
        let supreme_items: Vec<_> = REWARD_POOL.iter().filter(|r| r.weight_supreme > 0).collect();
        assert!(
            supreme_items.len() >= 10,
            "At least 10 items available in supreme chests"
        );
    }
}

/// 开启宝箱
pub fn cmd_open_chest(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let chest_name = args.trim();

    if chest_name.is_empty() {
        return format!(
            "{}\n❌ 请指定要开启的宝箱名\n例: 开启宝箱+铜宝箱\n可选: 铜宝箱/银宝箱/金宝箱/至尊宝箱",
            prefix
        );
    }

    // 查找宝箱
    let chest_idx = CHESTS.iter().position(|c| c.name == chest_name);
    let (idx, chest) = match chest_idx {
        Some(i) => (i, &CHESTS[i]),
        None => {
            // 模糊匹配
            let found = CHESTS
                .iter()
                .enumerate()
                .find(|(_, c)| c.name.contains(chest_name) || chest_name.contains(&c.name[0..3]));
            match found {
                Some((i, c)) => (i, c),
                None => {
                    return format!(
                        "{}\n❌ 未找到宝箱 [{}]\n可选: 铜宝箱/银宝箱/金宝箱/至尊宝箱",
                        prefix, chest_name
                    );
                }
            }
        }
    };

    // 等级检查
    let user_lv = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if user_lv < chest.min_level {
        return format!(
            "{}\n🔒 等级不足！\n需要等级: Lv.{}\n当前等级: Lv.{}",
            prefix, chest.min_level, user_lv
        );
    }

    // 钥匙检查
    let key_count = db.get_item_count(user_id, chest.key_item);
    if key_count < 1 {
        return format!(
            "{}\n🔑 缺少钥匙！\n开启[{}]需要[{}]\n当前拥有: {}个\n💡 钥匙可通过击杀怪物或商店购买获得",
            prefix, chest.name, chest.key_item, key_count
        );
    }

    // 货币检查
    let gold = db.read_currency(user_id, "gold");
    let diamond = db.read_currency(user_id, "diamond");
    if chest.cost_gold > 0 && gold < chest.cost_gold {
        return format!(
            "{}\n💰 金币不足！\n需要: {}金币\n当前: {}金币\n还差: {}金币",
            prefix,
            chest.cost_gold,
            gold,
            chest.cost_gold - gold
        );
    }
    if chest.cost_diamond > 0 && diamond < chest.cost_diamond {
        return format!(
            "{}\n💎 钻石不足！\n需要: {}钻石\n当前: {}钻石\n还差: {}钻石",
            prefix,
            chest.cost_diamond,
            diamond,
            chest.cost_diamond - diamond
        );
    }

    // 扣除钥匙和货币
    db.remove_item(user_id, chest.key_item, 1);
    if chest.cost_gold > 0 {
        db.write_currency(user_id, "gold", gold - chest.cost_gold);
    }
    if chest.cost_diamond > 0 {
        db.write_currency(user_id, "diamond", diamond - chest.cost_diamond);
    }

    // 生成随机种子
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let seed = format!("{}{}{}", user_id, idx, ts);

    // 抽取奖励：50%概率获得物品，50%概率获得金币/钻石
    let roll = chest_random(&seed, 100);

    let mut out = format!("{}\n", prefix);
    out += &format!("{} 🎁 开启 [{}] 成功！\n", chest.emoji, chest.name);
    out += "━━━ 开箱结果 ━━━\n";

    if roll < 50 {
        // 物品奖励
        if let Some((item, count)) = pick_reward(&seed, idx) {
            db.add_item(user_id, item.name, count);
            out += &format!("🎉 获得: [{}] ×{}\n", item.name, count);
        } else {
            // fallback 金币
            let bonus_gold =
                chest_random(&format!("{}fb", seed), (chest.gold_max - chest.gold_min) as u32) as i64 + chest.gold_min;
            let cur_gold = db.read_currency(user_id, "gold");
            db.write_currency(user_id, "gold", cur_gold + bonus_gold);
            out += &format!("💰 获得: {}金币\n", bonus_gold);
        }
    } else {
        // 金币/钻石奖励
        let gold_range = (chest.gold_max - chest.gold_min) as u32;
        if gold_range > 0 {
            let bonus_gold = chest_random(&format!("{}gold", seed), gold_range) as i64 + chest.gold_min;
            let cur_gold = db.read_currency(user_id, "gold");
            db.write_currency(user_id, "gold", cur_gold + bonus_gold);
            out += &format!("💰 获得: {}金币\n", bonus_gold);
        }
        if chest.diamond_max > 0 {
            let d_range = (chest.diamond_max - chest.diamond_min) as u32;
            if d_range > 0 {
                let bonus_diamond = chest_random(&format!("{}dia", seed), d_range) as i32 + chest.diamond_min;
                let cur_diamond = db.read_currency(user_id, "diamond");
                db.write_currency(user_id, "diamond", cur_diamond + bonus_diamond as i64);
                out += &format!("💎 获得: {}钻石\n", bonus_diamond);
            }
        }
    }

    // 消耗显示
    out += "━━━ 消耗明细 ━━━\n";
    out += &format!("🔑 消耗: [{}] ×1\n", chest.key_item);
    if chest.cost_gold > 0 {
        out += &format!("💰 消耗: {}金币\n", chest.cost_gold);
    }
    if chest.cost_diamond > 0 {
        out += &format!("💎 消耗: {}钻石\n", chest.cost_diamond);
    }

    let remaining_keys = db.get_item_count(user_id, chest.key_item);
    out += &format!("\n🔑 剩余[{}]: {}个\n", chest.key_item, remaining_keys);
    out += "继续开箱获取更多奖励吧！\n";
    out
}

/// 钥匙商店 - 购买钥匙
pub fn cmd_key_shop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let gold = db.read_currency(user_id, "gold");

    // 钥匙定价
    struct KeyDef {
        name: &'static str,
        emoji: &'static str,
        price_gold: i64,
    }
    const KEYS: &[KeyDef] = &[
        KeyDef {
            name: "铜钥匙",
            emoji: "🟤",
            price_gold: 200,
        },
        KeyDef {
            name: "银钥匙",
            emoji: "⬜",
            price_gold: 800,
        },
        KeyDef {
            name: "金钥匙",
            emoji: "🟨",
            price_gold: 3000,
        },
        KeyDef {
            name: "至尊钥匙",
            emoji: "🟪",
            price_gold: 10000,
        },
    ];

    let key_name = args.trim();

    if key_name.is_empty() {
        // 列出钥匙商店
        let mut out = format!("{}\n", prefix);
        out += "🔑 ═══ 钥匙商店 ═══\n\n";
        out += &format!("💰 当前金币: {}\n\n", gold);

        for (i, key) in KEYS.iter().enumerate() {
            let owned = db.get_item_count(user_id, key.name);
            let affordable = gold >= key.price_gold;
            let status = if affordable { "✅可购买" } else { "⏳金币不足" };
            out += &format!(
                "{} {}. [{}] - {}金币 ({}) | 已拥有: {}个\n",
                key.emoji,
                i + 1,
                key.name,
                key.price_gold,
                status,
                owned
            );
        }

        out += "\n📌 发送「宝箱钥匙+钥匙名」购买钥匙\n";
        out += "💡 击杀怪物也有概率掉落钥匙\n";
        return out;
    }

    // 查找钥匙
    let target = match KEYS.iter().find(|k| k.name == key_name) {
        Some(k) => k,
        None => {
            // 模糊匹配
            match KEYS
                .iter()
                .find(|k| k.name.contains(key_name) || key_name.contains(&k.name[0..2]))
            {
                Some(k) => k,
                None => {
                    return format!(
                        "{}\n❌ 未找到钥匙 [{}]\n可选: 铜钥匙/银钥匙/金钥匙/至尊钥匙",
                        prefix, key_name
                    )
                }
            }
        }
    };

    // 金币检查
    if gold < target.price_gold {
        return format!(
            "{}\n💰 金币不足！\n需要: {}金币\n当前: {}金币\n还差: {}金币",
            prefix,
            target.price_gold,
            gold,
            target.price_gold - gold
        );
    }

    // 购买
    db.write_currency(user_id, "gold", gold - target.price_gold);
    db.add_item(user_id, target.name, 1);

    let remaining = db.read_currency(user_id, "gold");
    let owned = db.get_item_count(user_id, target.name);

    let mut out = format!("{}\n", prefix);
    out += &format!("{} 🔑 购买成功！\n", target.emoji);
    out += &format!("🔑 获得: [{}] ×1\n", target.name);
    out += &format!("💰 消耗: {}金币\n", target.price_gold);
    out += &format!("💰 剩余金币: {}\n", remaining);
    out += &format!("🔑 当前[{}]: {}个\n", target.name, owned);
    out += "\n💡 发送「开启宝箱+宝箱名」使用钥匙开箱\n";
    out
}

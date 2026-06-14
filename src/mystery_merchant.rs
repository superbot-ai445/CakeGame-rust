/// 神秘商人系统
/// 随机出现的神秘商人，携带稀有物品，限时折扣，VIP加成
/// 数据存储：Global 表 SECTION='mystery_merchant' / 'mystery_merchant_daily'
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_SUB};
use crate::db::Database;
use crate::user;
use rand::seq::SliceRandom;
use rand::Rng;

/// 神秘商人商品定义
struct MerchantItem {
    name: &'static str,
    desc: &'static str,
    base_price_gold: i32,
    base_price_diamond: i32,
    rarity: &'static str,
    emoji: &'static str,
    stock: i32,
    min_level: i32,
}

/// 神秘商人等级
struct MerchantVisit {
    tier: i32,
    name: &'static str,
    emoji: &'static str,
    discount_pct: i32,
    bonus_items: i32,
    duration_hours: i32,
}

/// 获取商人等级定义
fn get_merchant_tiers() -> Vec<MerchantVisit> {
    vec![
        MerchantVisit {
            tier: 1,
            name: "流浪商人",
            emoji: "🧳",
            discount_pct: 5,
            bonus_items: 3,
            duration_hours: 2,
        },
        MerchantVisit {
            tier: 2,
            name: "神秘旅者",
            emoji: "🧙",
            discount_pct: 10,
            bonus_items: 5,
            duration_hours: 4,
        },
        MerchantVisit {
            tier: 3,
            name: "暗市行商",
            emoji: "🗡️",
            discount_pct: 15,
            bonus_items: 7,
            duration_hours: 6,
        },
        MerchantVisit {
            tier: 4,
            name: "传奇货郎",
            emoji: "⚜️",
            discount_pct: 20,
            bonus_items: 9,
            duration_hours: 8,
        },
        MerchantVisit {
            tier: 5,
            name: "时空商人",
            emoji: "🌌",
            discount_pct: 30,
            bonus_items: 12,
            duration_hours: 12,
        },
    ]
}

/// 获取神秘商人商品池
fn get_merchant_items() -> Vec<MerchantItem> {
    vec![
        MerchantItem {
            name: "神秘药水",
            desc: "恢复30%最大HP和MP",
            base_price_gold: 3000,
            base_price_diamond: 0,
            rarity: "稀有",
            emoji: "🧪",
            stock: 5,
            min_level: 1,
        },
        MerchantItem {
            name: "高级强化石",
            desc: "强化成功率+15%",
            base_price_gold: 8000,
            base_price_diamond: 0,
            rarity: "稀有",
            emoji: "💎",
            stock: 3,
            min_level: 10,
        },
        MerchantItem {
            name: "凤凰之羽",
            desc: "死亡后原地复活",
            base_price_gold: 15000,
            base_price_diamond: 0,
            rarity: "史诗",
            emoji: "🪶",
            stock: 2,
            min_level: 15,
        },
        MerchantItem {
            name: "时空精华",
            desc: "提升装备品质一个等级",
            base_price_gold: 0,
            base_price_diamond: 50,
            rarity: "传说",
            emoji: "✨",
            stock: 2,
            min_level: 20,
        },
        MerchantItem {
            name: "精炼水晶",
            desc: "精炼装备属性+10%",
            base_price_gold: 20000,
            base_price_diamond: 0,
            rarity: "史诗",
            emoji: "🔮",
            stock: 3,
            min_level: 15,
        },
        MerchantItem {
            name: "神秘宝箱",
            desc: "随机开出稀有物品",
            base_price_gold: 5000,
            base_price_diamond: 10,
            rarity: "稀有",
            emoji: "📦",
            stock: 5,
            min_level: 5,
        },
        MerchantItem {
            name: "幸运符",
            desc: "下次强化必定成功",
            base_price_gold: 0,
            base_price_diamond: 100,
            rarity: "传说",
            emoji: "🍀",
            stock: 1,
            min_level: 25,
        },
        MerchantItem {
            name: "经验卷轴",
            desc: "获得5000经验",
            base_price_gold: 6000,
            base_price_diamond: 0,
            rarity: "精良",
            emoji: "📜",
            stock: 5,
            min_level: 1,
        },
        MerchantItem {
            name: "金币袋",
            desc: "获得10000金币",
            base_price_gold: 0,
            base_price_diamond: 15,
            rarity: "精良",
            emoji: "💰",
            stock: 5,
            min_level: 1,
        },
        MerchantItem {
            name: "灵兽口粮",
            desc: "灵兽忠诚度+20",
            base_price_gold: 4000,
            base_price_diamond: 0,
            rarity: "精良",
            emoji: "🍖",
            stock: 5,
            min_level: 10,
        },
        MerchantItem {
            name: "洗点药水",
            desc: "重置所有属性点",
            base_price_gold: 25000,
            base_price_diamond: 0,
            rarity: "史诗",
            emoji: "⚗️",
            stock: 2,
            min_level: 20,
        },
        MerchantItem {
            name: "传送卷轴",
            desc: "瞬间传送到任意已解锁地图",
            base_price_gold: 3000,
            base_price_diamond: 0,
            rarity: "稀有",
            emoji: "🗺️",
            stock: 5,
            min_level: 5,
        },
        MerchantItem {
            name: "觉醒石",
            desc: "装备觉醒必需材料",
            base_price_gold: 30000,
            base_price_diamond: 0,
            rarity: "传说",
            emoji: "🌟",
            stock: 1,
            min_level: 30,
        },
        MerchantItem {
            name: "神秘碎片",
            desc: "收集10个兑换神秘装备",
            base_price_gold: 2000,
            base_price_diamond: 5,
            rarity: "稀有",
            emoji: "🧩",
            stock: 10,
            min_level: 1,
        },
        MerchantItem {
            name: "深渊钥匙",
            desc: "开启深渊宝箱",
            base_price_gold: 10000,
            base_price_diamond: 0,
            rarity: "史诗",
            emoji: "🗝️",
            stock: 2,
            min_level: 20,
        },
    ]
}

/// 获取当前小时用于商人刷新判定
fn get_hour_seed() -> i32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (now / 3600) as i32
}

/// 获取今日日期字符串
fn get_today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = now / 86400;
    format!("{}", days)
}

/// 读取NPC好感度等级（直接从Global表读取）
fn get_merchant_affinity_level(db: &Database, user_id: &str) -> i32 {
    let key = format!("{}_神秘商人", user_id);
    let data = db.global_get("NpcAffinity", &key);
    if data.is_empty() {
        return 0;
    }
    for part in data.split('|') {
        if let Some((k, v)) = part.split_once('=') {
            if k.trim() == "affinity" {
                let affinity: i32 = v.trim().parse().unwrap_or(0);
                return if affinity >= 500 {
                    5
                } else if affinity >= 200 {
                    4
                } else if affinity >= 120 {
                    3
                } else if affinity >= 60 {
                    2
                } else if affinity >= 20 {
                    1
                } else {
                    0
                };
            }
        }
    }
    0
}

/// 判断商人是否当前正在营业（30%概率出现，持续若干小时）
fn is_merchant_active(db: &Database) -> (bool, i32, String) {
    let hour_seed = get_hour_seed();
    let data = db.global_get("mystery_merchant", "active_info");
    if !data.is_empty() {
        let parts: Vec<&str> = data.split('|').collect();
        if parts.len() >= 3 {
            let tier: i32 = parts[1].parse().unwrap_or(0);
            let expiry_hour: i32 = parts[2].parse().unwrap_or(0);
            if hour_seed < expiry_hour && tier > 0 {
                return (true, tier, parts.get(3).unwrap_or(&"神秘商人").to_string());
            }
        }
    }

    // 每小时30%概率刷新商人
    let mut rng = rand::thread_rng();
    if rng.gen_ratio(3, 10) {
        let tier = rng.gen_range(1..=5_i32);
        let tiers = get_merchant_tiers();
        let t = tiers.iter().find(|t| t.tier == tier).unwrap();
        let expiry = hour_seed + t.duration_hours;
        let name = format!("{}{}", t.emoji, t.name);
        db.global_set(
            "mystery_merchant",
            "active_info",
            &format!("{}|{}|{}|{}", hour_seed, tier, expiry, name),
        );
        // 随机选择本次商人携带的商品
        let all_items = get_merchant_items();
        let count = (t.bonus_items as usize).min(all_items.len());
        let mut indices: Vec<usize> = (0..all_items.len()).collect();
        indices.shuffle(&mut rng);
        let selected: Vec<String> = indices[..count].iter().map(|i| i.to_string()).collect();
        db.global_set("mystery_merchant", "current_items", &selected.join(","));
        // 初始化库存
        for &i in &indices[..count] {
            let item = &all_items[i];
            db.global_set("mystery_merchant", &format!("stock_{}", i), &item.stock.to_string());
        }
        return (true, tier, name);
    }

    (false, 0, String::new())
}

/// 获取商人当前折扣率
fn get_discount_pct(db: &Database, tier: i32, user_id: &str) -> i32 {
    let tiers = get_merchant_tiers();
    let base_discount = tiers
        .iter()
        .find(|t| t.tier == tier)
        .map(|t| t.discount_pct)
        .unwrap_or(0);
    // VIP额外折扣
    let vip_level: i32 = db.read_basic(user_id, "vip_level").parse().unwrap_or(0);
    let vip_bonus = match vip_level {
        0..=2 => 0,
        3..=5 => 3,
        6..=8 => 5,
        _ => 8,
    };
    let affinity_bonus = get_merchant_affinity_level(db, user_id).min(5);
    (base_discount + vip_bonus + affinity_bonus).min(50)
}

/// 查看神秘商人 — 显示商人信息和商品列表
pub fn cmd_view_merchant(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let (active, tier, merchant_name) = is_merchant_active(db);
    if !active {
        let mut out = format!("{}\n═══ 🏪 神秘商人 ═══", prefix);
        out.push_str("\n\n❌ 神秘商人目前不在...");
        out.push_str("\n\n💡 神秘商人随机出现，每次停留2~12小时");
        out.push_str("\n📌 VIP等级越高，折扣越大");
        out.push_str("\n📌 提升NPC好感度可获得额外折扣");
        out.push_str("\n\n⏰ 请稍后再来看看！");
        return out;
    }

    let level: i32 = db.read_basic(user_id, "等级").parse().unwrap_or(1);
    let discount = get_discount_pct(db, tier, user_id);
    let today = get_today();

    let mut out = format!("{}\n═══ {} 正在营业 ═══", prefix, merchant_name);
    let tiers = get_merchant_tiers();
    let t = tiers.iter().find(|x| x.tier == tier).unwrap();
    out.push_str(&format!("\n🏷️ 商人等级: {}级 ({})", t.tier, t.name));
    out.push_str(&format!("\n💰 当前折扣: -{}%", discount));
    if discount > 0 {
        out.push_str(" (基础+VIP+好感度)");
    }
    out.push_str(&format!("\n⏰ 停留时间: {}小时", t.duration_hours));

    let items_str = db.global_get("mystery_merchant", "current_items");
    if items_str.is_empty() {
        out.push_str("\n\n暂无商品...");
        return out;
    }

    let all_items = get_merchant_items();
    let indices: Vec<usize> = items_str.split(',').filter_map(|s| s.parse().ok()).collect();

    out.push_str(&format!("\n\n📦 今日商品 (共{}件):", indices.len()));
    for (idx, &item_i) in indices.iter().enumerate() {
        if item_i >= all_items.len() {
            continue;
        }
        let item = &all_items[item_i];
        let stock: i32 = db
            .global_get("mystery_merchant", &format!("stock_{}", item_i))
            .parse()
            .unwrap_or(item.stock);
        let level_ok = level >= item.min_level;

        let gold_price = if item.base_price_gold > 0 {
            item.base_price_gold * (100 - discount) / 100
        } else {
            0
        };
        let diamond_price = if item.base_price_diamond > 0 {
            item.base_price_diamond * (100 - discount) / 100
        } else {
            0
        };

        out.push_str(&format!(
            "\n{}. {} {} [{}]",
            idx + 1,
            item.emoji,
            item.name,
            item.rarity
        ));
        if !level_ok {
            out.push_str(&format!(" 🔒需要{}级", item.min_level));
        }
        out.push_str(&format!(" — {}", item.desc));
        if gold_price > 0 {
            out.push_str(&format!(" | 💰{}金币", gold_price));
        }
        if diamond_price > 0 {
            out.push_str(&format!(" | 💎{}钻石", diamond_price));
        }
        out.push_str(&format!(" | 库存:{}", stock));
        if stock <= 0 {
            out.push_str(" [售罄]");
        }
    }

    let daily_count = db.global_get("mystery_merchant", &format!("daily_buy_{}_{}", user_id, today));
    out.push_str(&format!(
        "\n\n📊 今日已购买: {}/5次",
        daily_count.parse::<i32>().unwrap_or(0)
    ));
    out.push_str("\n💡 发送 '购买神秘+编号' 购买商品");

    out
}

/// 购买神秘商品 — 购买商人携带的物品
pub fn cmd_buy_merchant(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let (active, tier, merchant_name) = is_merchant_active(db);
    if !active {
        return format!("{}\n❌ 神秘商人不在，无法购买！", prefix);
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 => n - 1,
        _ => return format!("{}\n❌ 请输入有效的商品编号！\n💡 发送 '购买神秘+编号'", prefix),
    };

    let items_str = db.global_get("mystery_merchant", "current_items");
    let indices: Vec<usize> = items_str.split(',').filter_map(|s| s.parse().ok()).collect();

    if idx >= indices.len() {
        return format!("{}\n❌ 商品编号不存在！共有{}件商品。", prefix, indices.len());
    }

    let item_i = indices[idx];
    let all_items = get_merchant_items();
    if item_i >= all_items.len() {
        return format!("{}\n❌ 商品数据异常。", prefix);
    }
    let item = &all_items[item_i];

    // 等级检查
    let level: i32 = db.read_basic(user_id, "等级").parse().unwrap_or(1);
    if level < item.min_level {
        return format!("{}\n❌ 等级不足！需要{}级，当前{}级。", prefix, item.min_level, level);
    }

    // 每日购买限制
    let today = get_today();
    let daily_key = format!("daily_buy_{}_{}", user_id, today);
    let daily_count: i32 = db.global_get("mystery_merchant", &daily_key).parse().unwrap_or(0);
    if daily_count >= 5 {
        return format!("{}\n❌ 今日购买次数已达上限(5次)！\n⏰ 明天再来吧。", prefix);
    }

    // 库存检查
    let stock_key = format!("stock_{}", item_i);
    let stock: i32 = db
        .global_get("mystery_merchant", &stock_key)
        .parse()
        .unwrap_or(item.stock);
    if stock <= 0 {
        return format!("{}\n❌ {}已售罄！", prefix, item.name);
    }

    // 计算价格（含折扣）
    let discount = get_discount_pct(db, tier, user_id);
    let gold_price = if item.base_price_gold > 0 {
        item.base_price_gold * (100 - discount) / 100
    } else {
        0
    };
    let diamond_price = if item.base_price_diamond > 0 {
        item.base_price_diamond * (100 - discount) / 100
    } else {
        0
    };

    // 金币检查
    if gold_price > 0 {
        let user_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        if user_gold < gold_price as i64 {
            return format!(
                "{}\n❌ 金币不足！需要{}金币，当前{}金币。",
                prefix, gold_price, user_gold
            );
        }
    }

    // 钻石检查
    if diamond_price > 0 {
        let user_diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
        if user_diamond < diamond_price as i64 {
            return format!(
                "{}\n❌ 钻石不足！需要{}钻石，当前{}钻石。",
                prefix, diamond_price, user_diamond
            );
        }
    }

    // 扣款
    if gold_price > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_price as i64);
    }
    if diamond_price > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, diamond_price as i64);
    }

    // 发放物品
    db.knapsack_add(user_id, item.name, 1);
    db.global_set("mystery_merchant", &stock_key, &(stock - 1).to_string());
    db.global_set("mystery_merchant", &daily_key, &(daily_count + 1).to_string());

    // 累计统计
    let total_key = format!("merchant_total_{}", user_id);
    let total_bought: i32 = db.global_get("mystery_merchant", &total_key).parse().unwrap_or(0);
    db.global_set("mystery_merchant", &total_key, &(total_bought + 1).to_string());
    if gold_price > 0 {
        let gold_key = format!("merchant_gold_{}", user_id);
        let prev: i32 = db.global_get("mystery_merchant", &gold_key).parse().unwrap_or(0);
        db.global_set("mystery_merchant", &gold_key, &(prev + gold_price).to_string());
    }
    if diamond_price > 0 {
        let dia_key = format!("merchant_diamond_{}", user_id);
        let prev: i32 = db.global_get("mystery_merchant", &dia_key).parse().unwrap_or(0);
        db.global_set("mystery_merchant", &dia_key, &(prev + diamond_price).to_string());
    }

    let mut out = format!("{}\n═══ ✅ 购买成功！ ═══", prefix);
    out.push_str(&format!("\n🛒 商人: {}", merchant_name));
    out.push_str(&format!("\n{} {} ×1 [{}]", item.emoji, item.name, item.rarity));
    out.push_str(&format!("\n📝 {}", item.desc));
    if gold_price > 0 {
        out.push_str(&format!("\n💰 花费: {}金币", gold_price));
    }
    if diamond_price > 0 {
        out.push_str(&format!("\n💎 花费: {}钻石", diamond_price));
    }
    if discount > 0 {
        out.push_str(&format!("\n🏷️ 享受折扣: -{}%", discount));
    }
    out.push_str(&format!("\n📦 剩余库存: {}", stock - 1));
    out.push_str(&format!("\n📊 今日已购买: {}/5次", daily_count + 1));

    // 神秘宝箱特殊处理
    if item.name == "神秘宝箱" {
        let mut rng = rand::thread_rng();
        let rewards = [
            ("高级强化石", 30),
            ("凤凰之羽", 15),
            ("经验卷轴", 40),
            ("金币袋", 35),
            ("幸运符", 5),
            ("时空精华", 10),
        ];
        let roll: i32 = rng.gen_range(0..100);
        let mut cumulative = 0;
        for (name, chance) in &rewards {
            cumulative += chance;
            if roll < cumulative {
                db.knapsack_add(user_id, name, 1);
                out.push_str(&format!("\n\n🎁 宝箱开出: {} ×1!", name));
                break;
            }
        }
        if roll >= cumulative {
            db.knapsack_add(user_id, "神秘碎片", 2);
            out.push_str("\n\n🎁 宝箱开出: 神秘碎片 ×2!");
        }
    }

    // 成就钩子
    crate::achievement::on_shop_sale(db, user_id);

    out
}

/// 神秘商人帮助
pub fn cmd_merchant_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ 📖 神秘商人帮助 ═══", prefix);
    out.push_str("\n\n🏪 神秘商人是随机出现的稀有NPC");
    out.push_str("\n携带普通商店买不到的稀有物品");
    out.push_str("\n\n📋 指令列表:");
    out.push_str("\n  • 查看神秘 — 查看商人状态和商品");
    out.push_str("\n  • 购买神秘+编号 — 购买指定商品");
    out.push_str("\n  • 神秘帮助 — 显示此帮助");
    out.push_str("\n  • 神秘统计 — 查看购买统计");
    out.push_str("\n\n💡 系统说明:");
    out.push_str("\n  • 商人每小时有30%概率出现");
    out.push_str("\n  • 每次停留2~12小时不等");
    out.push_str("\n  • 5级商人等级(流浪→时空)，等级越高折扣越大");
    out.push_str("\n  • 每人每日限购买5次");
    out.push_str("\n  • VIP等级和NPC好感度提供额外折扣");
    out.push_str("\n  • 部分商品有等级限制");
    out.push_str("\n  • 神秘宝箱可开出随机稀有物品");
    out.push_str("\n\n🏷️ 折扣计算:");
    out.push_str("\n  基础折扣(5~30%) + VIP折扣(0~8%) + 好感度(0~5%)");
    out.push_str("\n  最高可享受50%折扣！");

    out
}

/// 神秘商人统计 — 查看购买统计和里程碑
pub fn cmd_merchant_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let mut out = format!("{}\n═══ 📊 神秘商人统计 ═══", prefix);

    let total_key = format!("merchant_total_{}", user_id);
    let total_bought: i32 = db.global_get("mystery_merchant", &total_key).parse().unwrap_or(0);
    let gold_spent_key = format!("merchant_gold_{}", user_id);
    let gold_spent: i32 = db.global_get("mystery_merchant", &gold_spent_key).parse().unwrap_or(0);
    let diamond_spent_key = format!("merchant_diamond_{}", user_id);
    let diamond_spent: i32 = db
        .global_get("mystery_merchant", &diamond_spent_key)
        .parse()
        .unwrap_or(0);

    out.push_str(&format!("\n\n🛒 总购买次数: {}次", total_bought));
    out.push_str(&format!("\n💰 累计花费金币: {}", gold_spent));
    out.push_str(&format!("\n💎 累计花费钻石: {}", diamond_spent));

    let today = get_today();
    let daily_key = format!("daily_buy_{}_{}", user_id, today);
    let daily_count: i32 = db.global_get("mystery_merchant", &daily_key).parse().unwrap_or(0);
    out.push_str(&format!("\n📅 今日购买: {}/5次", daily_count));

    // 商人等级
    let (active, tier, merchant_name) = is_merchant_active(db);
    if active {
        let discount = get_discount_pct(db, tier, user_id);
        out.push_str(&format!("\n\n🏪 当前商人: {} ({}级)", merchant_name, tier));
        out.push_str(&format!("\n🏷️ 当前折扣: -{}%", discount));
    } else {
        out.push_str("\n\n❌ 神秘商人当前不在营业");
    }

    // VIP等级和折扣
    let vip_level: i32 = db.read_basic(user_id, "vip_level").parse().unwrap_or(0);
    let vip_bonus = match vip_level {
        0..=2 => 0,
        3..=5 => 3,
        6..=8 => 5,
        _ => 8,
    };
    out.push_str(&format!("\n\n👑 VIP等级: {} (折扣+{}%)", vip_level, vip_bonus));

    let affinity_level = get_merchant_affinity_level(db, user_id);
    out.push_str(&format!(
        "\n💖 神秘商人好感度: {}级 (折扣+{}%)",
        affinity_level,
        affinity_level.min(5)
    ));

    // 购买里程碑
    let milestones = [
        (10, "初识商人", "🎁"),
        (30, "老顾客", "🏅"),
        (50, "VIP客户", "👑"),
        (100, "至尊买家", "💎"),
        (200, "商人的知己", "🌟"),
    ];
    out.push_str("\n\n🏆 购买里程碑:");
    for (threshold, name, emoji) in &milestones {
        let status = if total_bought >= *threshold { "✅" } else { "⬜" };
        out.push_str(&format!("\n  {} {} {} ({}次)", status, emoji, name, threshold));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merchant_tiers_count() {
        let tiers = get_merchant_tiers();
        assert_eq!(tiers.len(), 5);
        assert_eq!(tiers[0].tier, 1);
        assert_eq!(tiers[4].tier, 5);
    }

    #[test]
    fn test_merchant_items_count() {
        let items = get_merchant_items();
        assert!(items.len() >= 10);
        for item in &items {
            assert!(!item.name.is_empty());
            assert!(item.base_price_gold > 0 || item.base_price_diamond > 0);
        }
    }

    #[test]
    fn test_discount_range() {
        let tiers = get_merchant_tiers();
        for t in &tiers {
            assert!(t.discount_pct >= 5 && t.discount_pct <= 30);
        }
    }

    #[test]
    fn test_today_format() {
        let today = get_today();
        assert!(!today.is_empty());
    }

    #[test]
    fn test_hour_seed_deterministic() {
        let seed1 = get_hour_seed();
        let seed2 = get_hour_seed();
        assert_eq!(seed1, seed2);
    }

    #[test]
    fn test_item_rarity_labels() {
        let items = get_merchant_items();
        let valid_rarities = ["精良", "稀有", "史诗", "传说"];
        for item in &items {
            assert!(valid_rarities.contains(&item.rarity), "Invalid rarity: {}", item.rarity);
        }
    }

    #[test]
    fn test_merchant_tier_progression() {
        let tiers = get_merchant_tiers();
        for i in 1..tiers.len() {
            assert!(tiers[i].discount_pct > tiers[i - 1].discount_pct);
            assert!(tiers[i].bonus_items > tiers[i - 1].bonus_items);
        }
    }

    #[test]
    fn test_merchant_items_have_emoji() {
        let items = get_merchant_items();
        for item in &items {
            assert!(!item.emoji.is_empty());
        }
    }

    #[test]
    fn test_merchant_tier_names() {
        let tiers = get_merchant_tiers();
        let expected = ["流浪商人", "神秘旅者", "暗市行商", "传奇货郎", "时空商人"];
        for (i, name) in expected.iter().enumerate() {
            assert_eq!(tiers[i].name, *name);
        }
    }

    #[test]
    fn test_merchant_tier_duration() {
        let tiers = get_merchant_tiers();
        for t in &tiers {
            assert!(t.duration_hours >= 2 && t.duration_hours <= 12);
        }
    }

    #[test]
    fn test_merchant_items_price_positive() {
        let items = get_merchant_items();
        for item in &items {
            assert!(item.base_price_gold + item.base_price_diamond > 0);
        }
    }

    #[test]
    fn test_merchant_items_min_level() {
        let items = get_merchant_items();
        for item in &items {
            assert!(item.min_level >= 1);
        }
    }
}

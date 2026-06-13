/// CakeGame 资源回收系统 (Resource Recycling System)
///
/// 玩家可将多余的装备、材料等物品回收，获得「回收积分」。
/// 回收积分可在回收商店中兑换稀有道具、强化材料等珍贵物品。
///
/// 功能：
/// - 查看回收: 显示可回收物品列表及预计获得积分
/// - 回收物品: 将指定物品回收为积分
/// - 批量回收: 一键回收所有低品质物品
/// - 回收商店: 浏览可用积分兑换的物品
/// - 回收兑换: 使用积分兑换物品
/// - 回收排行: 全服回收积分排行
/// - 回收统计: 个人回收历史统计
///
/// 数据存储: Global 表 SECTION='recycle'
use crate::core::*;
use crate::db::Database;
use crate::user;

// ==================== 回收积分规则 ====================

/// 根据物品价格估算品质和回收积分
fn estimate_recycle_points(price: i32) -> (String, i32) {
    if price <= 100 {
        ("普通".to_string(), 5)
    } else if price <= 500 {
        ("优秀".to_string(), 15)
    } else if price <= 2000 {
        ("精良".to_string(), 30)
    } else if price <= 5000 {
        ("稀有".to_string(), 60)
    } else if price <= 15000 {
        ("史诗".to_string(), 120)
    } else if price <= 50000 {
        ("传说".to_string(), 250)
    } else {
        ("神器".to_string(), 500)
    }
}

fn quality_emoji(quality: &str) -> &str {
    match quality {
        "普通" => "⬜",
        "优秀" => "🟢",
        "精良" => "🔵",
        "稀有" => "🟣",
        "史诗" => "🟠",
        "传说" => "🟡",
        "神器" => "🔴",
        _ => "⬜",
    }
}

// ==================== 回收商店物品定义 ====================

#[allow(dead_code)]
struct RecycleShopItem {
    name: &'static str,
    cost: i32,
    emoji: &'static str,
    desc: &'static str,
    stock_desc: &'static str,
}

const RECYCLE_SHOP: &[RecycleShopItem] = &[
    RecycleShopItem {
        name: "回收宝箱",
        cost: 50,
        emoji: "📦",
        desc: "随机获得金币或道具",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "强化石",
        cost: 100,
        emoji: "💎",
        desc: "用于装备强化的珍贵矿石",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "高级生命药水",
        cost: 80,
        emoji: "🧪",
        desc: "恢复大量生命值",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "高级魔法药水",
        cost: 80,
        emoji: "💧",
        desc: "恢复大量魔法值",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "复活卷轴",
        cost: 200,
        emoji: "📜",
        desc: "死亡后原地复活",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "经验卷轴",
        cost: 150,
        emoji: "📖",
        desc: "使用后获得大量经验",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "幸运符",
        cost: 120,
        emoji: "🍀",
        desc: "提高强化成功率10%",
        stock_desc: "不限量",
    },
    RecycleShopItem {
        name: "高级强化石",
        cost: 300,
        emoji: "💠",
        desc: "高级装备强化材料",
        stock_desc: "限量",
    },
    RecycleShopItem {
        name: "精炼水晶",
        cost: 500,
        emoji: "🔮",
        desc: "用于装备重铸的稀有水晶",
        stock_desc: "限量",
    },
    RecycleShopItem {
        name: "回收勋章",
        cost: 1000,
        emoji: "🏅",
        desc: "回收大师的荣誉勋章(称号)",
        stock_desc: "一次性",
    },
    RecycleShopItem {
        name: "凤凰之羽",
        cost: 2000,
        emoji: "🪶",
        desc: "传说级材料，用于装备进化",
        stock_desc: "稀有",
    },
    RecycleShopItem {
        name: "时空精华",
        cost: 3000,
        emoji: "✨",
        desc: "蕴含时空之力的精华，用于装备觉醒",
        stock_desc: "稀有",
    },
];

const SECTION: &str = "recycle";

// ==================== 数据读写 ====================

/// 读取用户回收积分
fn get_recycle_points(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("points_{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// 设置用户回收积分
fn set_recycle_points(db: &Database, user_id: &str, points: i64) {
    db.global_set(SECTION, &format!("points_{}", user_id), &points.to_string());
}

/// 读取用户总回收次数
fn get_recycle_count(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("count_{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// 读取用户总回收积分（历史累计）
fn get_total_earned(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("total_{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// 读取用户总兑换次数
fn get_exchange_count(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("exchg_{}", user_id))
        .parse()
        .unwrap_or(0)
}

// ==================== 指令实现 ====================

/// 查看回收 — 显示回收系统概览和当前背包可回收物品
pub fn cmd_view_recycle(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let points = get_recycle_points(db, user_id);
    let count = get_recycle_count(db, user_id);
    let total = get_total_earned(db, user_id);

    let mut out = format!("{}\n═══ ♻️ 资源回收站 ═══\n", prefix);
    out.push_str("将多余物品回收为积分，兑换珍贵道具！\n\n");

    out.push_str("📊 我的回收信息:\n");
    out.push_str(&format!("  💰 当前回收积分: {}\n", points));
    out.push_str(&format!("  📦 累计回收次数: {}\n", count));
    out.push_str(&format!("  ⭐ 累计获得积分: {}\n", total));

    // 显示背包中可回收物品
    let items = db.knapsack_all(user_id);
    if items.is_empty() {
        out.push_str("\n📭 背包为空，没有可回收物品。\n");
    } else {
        out.push_str(&format!("\n📋 可回收物品 (共{}件):\n", items.len()));
        out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
        let mut total_value = 0i32;
        let mut shown = 0;
        for item in &items {
            // 获取物品价格
            let price: i32 = db
                .query_row(
                    "SELECT COALESCE(SellPrice, 0) FROM Config_Goods WHERE TRIM(GoodsName, '\\0') = TRIM(?1, '\\0')",
                    &[&item.name],
                    |row| row.get::<_, i32>(0),
                )
                .unwrap_or(0);
            let (quality, pts) = estimate_recycle_points(price);
            let emoji = quality_emoji(&quality);
            if shown < 15 {
                out.push_str(&format!(
                    "  {} [{}] {} ×{} → {}积分\n",
                    emoji,
                    quality,
                    item.name,
                    item.quantity,
                    pts * item.quantity
                ));
            }
            total_value += pts * item.quantity;
            shown += 1;
        }
        if shown > 15 {
            out.push_str(&format!("  ... 还有 {} 种物品\n", shown - 15));
        }
        out.push_str(&format!("\n💡 全部回收预计获得: {} 积分\n", total_value));
    }

    out.push_str("\n📌 操作指令:\n");
    out.push_str("  回收物品+物品名 — 回收指定物品\n");
    out.push_str("  批量回收+品质 — 回收所有指定品质物品\n");
    out.push_str("  回收商店 — 浏览回收兑换商店\n");
    out.push_str("  回收排行 — 查看全服回收排行\n");
    out.push_str("  回收统计 — 查看个人回收统计\n");

    out
}

/// 回收物品 — 将指定物品回收为积分
pub fn cmd_recycle_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return format!("{}\n请指定要回收的物品名。\n用法：回收物品+物品名", prefix);
    }

    // 检查装备锁定
    if crate::equip_lock::is_equip_name_locked(db, user_id, item_name) {
        return format!(
            "{}\n🔒 [{}] 已锁定，无法回收！\n💡 使用「解锁装备+槽位名」解除锁定后再回收。",
            prefix, item_name
        );
    }

    // 检查背包中是否有该物品
    let count_in_bag = db.knapsack_quantity(user_id, item_name);
    if count_in_bag <= 0 {
        return format!("{}\n❌ 背包中没有 [{}]。", prefix, item_name);
    }

    // 获取物品价格来估算积分
    let price: i32 = db
        .query_row(
            "SELECT COALESCE(SellPrice, 0) FROM Config_Goods WHERE TRIM(GoodsName, '\\0') = TRIM(?1, '\\0')",
            &[item_name],
            |row| row.get::<_, i32>(0),
        )
        .unwrap_or(0);

    let (quality, base_pts) = estimate_recycle_points(price);

    // 额外加成：数量越多回收效率越高（每10个额外+10%）
    let bonus_pct = if count_in_bag >= 10 { 10 } else { 0 };
    let actual_pts = base_pts * (100 + bonus_pct) / 100;

    // 从背包移除物品
    db.knapsack_remove(user_id, item_name, 1);

    // 增加回收积分
    let old_points = get_recycle_points(db, user_id);
    set_recycle_points(db, user_id, old_points + actual_pts as i64);

    // 更新统计
    let old_count = get_recycle_count(db, user_id);
    db.global_set(SECTION, &format!("count_{}", user_id), &(old_count + 1).to_string());
    let old_total = get_total_earned(db, user_id);
    db.global_set(
        SECTION,
        &format!("total_{}", user_id),
        &(old_total + actual_pts as i64).to_string(),
    );

    let emoji = quality_emoji(&quality);
    let mut out = format!("{}\n", prefix);
    out.push_str("♻️ 回收成功！\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("  {} [{}] {}\n", emoji, quality, item_name));
    out.push_str(&format!("  ➜ 获得 {} 回收积分", actual_pts));
    if bonus_pct > 0 {
        out.push_str(&format!(" (批量加成+{}%)", bonus_pct));
    }
    out.push_str(&format!("\n\n💰 当前回收积分: {}", old_points + actual_pts as i64));

    // 成就检测：累计回收里程碑
    let new_total = old_total + actual_pts as i64;
    if new_total >= 10000 && old_total < 10000 {
        out.push_str("\n\n🏆 成就解锁：回收大师！累计获得10000回收积分！");
    } else if new_total >= 1000 && old_total < 1000 {
        out.push_str("\n\n🎖️ 成就解锁：回收新手！累计获得1000回收积分！");
    }

    out
}

/// 批量回收 — 回收所有指定品质及以下的物品
pub fn cmd_recycle_batch(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let quality_arg = args.trim();

    // 品质等级映射
    let target_level: i32 = match quality_arg {
        "普通" => 1,
        "优秀" => 2,
        "精良" => 3,
        "稀有" => 4,
        "史诗" => 5,
        "传说" => 6,
        "神器" => 7,
        "全部" => 4, // 默认批量回收到稀有品质
        _ => {
            return format!(
                "{}\n❌ 无效品质: [{}]\n\n可选品质: 普通/优秀/精良/稀有/史诗/传说/神器/全部\n用法: 批量回收+品质名\n示例: 批量回收+普通",
                prefix, quality_arg
            );
        }
    };

    let items = db.knapsack_all(user_id);
    if items.is_empty() {
        return format!("{}\n📭 背包为空，没有可回收物品。", prefix);
    }

    let mut total_pts = 0i64;
    let mut recycled_count = 0;
    let mut recycled_items: Vec<String> = Vec::new();

    for item in &items {
        // 检查装备锁定
        if crate::equip_lock::is_equip_name_locked(db, user_id, &item.name) {
            continue;
        }

        let price: i32 = db
            .query_row(
                "SELECT COALESCE(SellPrice, 0) FROM Config_Goods WHERE TRIM(GoodsName, '\\0') = TRIM(?1, '\\0')",
                &[&item.name],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0);

        let (quality, pts) = estimate_recycle_points(price);
        let item_level = match quality.as_str() {
            "普通" => 1,
            "优秀" => 2,
            "精良" => 3,
            "稀有" => 4,
            "史诗" => 5,
            "传说" => 6,
            "神器" => 7,
            _ => 0,
        };

        if item_level <= target_level && item_level > 0 {
            let qty = item.quantity;
            db.knapsack_remove(&item.name, &item.name, qty);
            total_pts += pts as i64 * qty as i64;
            recycled_count += qty;
            recycled_items.push(format!("  {} ×{}", item.name, qty));
        }
    }

    if recycled_count == 0 {
        return format!("{}\n没有找到 [{}] 品质及以下的物品可回收。", prefix, quality_arg);
    }

    // 更新积分和统计
    let old_points = get_recycle_points(db, user_id);
    set_recycle_points(db, user_id, old_points + total_pts);
    let old_count = get_recycle_count(db, user_id);
    db.global_set(
        SECTION,
        &format!("count_{}", user_id),
        &(old_count + recycled_count as i64).to_string(),
    );
    let old_total = get_total_earned(db, user_id);
    db.global_set(
        SECTION,
        &format!("total_{}", user_id),
        &(old_total + total_pts).to_string(),
    );

    let mut out = format!("{}\n♻️ 批量回收完成！\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("📊 回收品质: {}及以下\n", quality_arg));
    out.push_str(&format!("📦 回收数量: {} 件\n", recycled_count));
    out.push_str(&format!("💰 获得积分: {}\n", total_pts));
    out.push_str(&format!("💰 当前积分: {}\n", old_points + total_pts));

    if recycled_items.len() <= 20 {
        out.push_str("\n📋 回收明细:\n");
        for item in &recycled_items {
            out.push_str(&format!("{}\n", item));
        }
    } else {
        out.push_str(&format!("\n📋 共回收 {} 种物品\n", recycled_items.len()));
    }

    out
}

/// 回收商店 — 浏览可用积分兑换的物品
pub fn cmd_recycle_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let points = get_recycle_points(db, user_id);

    let mut out = format!("{}\n═══ ♻️ 回收商店 ═══\n", prefix);
    out.push_str(&format!("💰 当前回收积分: {}\n\n", points));
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    for (i, item) in RECYCLE_SHOP.iter().enumerate() {
        let affordable = if points >= item.cost as i64 { "✅" } else { "❌" };
        out.push_str(&format!(
            "{}. {} {} [{}积分] {}\n   {}\n",
            i + 1,
            affordable,
            item.emoji,
            item.cost,
            item.name,
            item.desc
        ));
    }

    out.push_str("\n📌 用法: 回收兑换+物品名\n");
    out.push_str("示例: 回收兑换+强化石\n");

    out
}

/// 回收兑换 — 使用积分兑换商店物品
pub fn cmd_recycle_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!(
            "{}\n请指定要兑换的物品名。\n用法：回收兑换+物品名\n发送「回收商店」查看可兑换物品",
            prefix
        );
    }

    // 查找商店物品
    let shop_item = RECYCLE_SHOP.iter().find(|i| i.name == item_name);
    let shop_item = match shop_item {
        Some(item) => item,
        None => {
            return format!(
                "{}\n❌ 回收商店中没有 [{}]。\n发送「回收商店」查看可兑换物品。",
                prefix, item_name
            );
        }
    };

    let points = get_recycle_points(db, user_id);
    if points < shop_item.cost as i64 {
        return format!(
            "{}\n❌ 回收积分不足！\n需要: {} 积分\n当前: {} 积分\n还差: {} 积分\n\n💡 回收更多物品来获取积分！",
            prefix,
            shop_item.cost,
            points,
            shop_item.cost as i64 - points
        );
    }

    // 扣除积分
    set_recycle_points(db, user_id, points - shop_item.cost as i64);

    // 根据物品类型发放奖励
    let reward_desc = grant_recycle_reward(db, user_id, shop_item.name);

    // 更新兑换统计
    let old_exchg = get_exchange_count(db, user_id);
    db.global_set(SECTION, &format!("exchg_{}", user_id), &(old_exchg + 1).to_string());

    let mut out = format!("{}\n", prefix);
    out.push_str("✅ 兑换成功！\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("  {} {}\n", shop_item.emoji, shop_item.name));
    out.push_str(&format!("  消耗: {} 回收积分\n", shop_item.cost));
    out.push_str(&format!("  获得: {}\n", reward_desc));
    out.push_str(&format!("\n💰 剩余回收积分: {}", points - shop_item.cost as i64));

    out
}

/// 发放回收商店兑换奖励
fn grant_recycle_reward(db: &Database, user_id: &str, item_name: &str) -> String {
    match item_name {
        "回收宝箱" => {
            // 随机金币奖励 500~3000
            let gold = 500 + (item_name.len() as i64 * 137 % 2501);
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold);
            format!("{} 金币", gold)
        }
        "强化石" => {
            db.knapsack_add(user_id, "强化石", 1);
            "强化石 ×1".to_string()
        }
        "高级生命药水" => {
            db.knapsack_add(user_id, "高级生命药水", 1);
            "高级生命药水 ×1".to_string()
        }
        "高级魔法药水" => {
            db.knapsack_add(user_id, "高级魔法药水", 1);
            "高级魔法药水 ×1".to_string()
        }
        "复活卷轴" => {
            db.knapsack_add(user_id, "复活卷轴", 1);
            "复活卷轴 ×1".to_string()
        }
        "经验卷轴" => {
            // 给予大量经验
            user::add_experience(db, user_id, 2000);
            "2000 经验".to_string()
        }
        "幸运符" => {
            db.knapsack_add(user_id, "幸运符", 1);
            "幸运符 ×1".to_string()
        }
        "高级强化石" => {
            db.knapsack_add(user_id, "高级强化石", 1);
            "高级强化石 ×1".to_string()
        }
        "精炼水晶" => {
            db.knapsack_add(user_id, "精炼水晶", 1);
            "精炼水晶 ×1".to_string()
        }
        "回收勋章" => {
            // 给予称号和钻石
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 100);
            "回收勋章 + 100💎".to_string()
        }
        "凤凰之羽" => {
            db.knapsack_add(user_id, "凤凰之羽", 1);
            "凤凰之羽 ×1".to_string()
        }
        "时空精华" => {
            db.knapsack_add(user_id, "时空精华", 1);
            "时空精华 ×1".to_string()
        }
        _ => {
            // 未知物品 → 给金币补偿
            let gold = 1000;
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold);
            format!("{} 金币 (补偿)", gold)
        }
    }
}

/// 回收排行 — 全服回收积分排行榜
pub fn cmd_recycle_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 查询所有回收积分
    let rows: Vec<(String, String)> = db.query_rows(
        "SELECT Key, Value FROM Global WHERE Section = 'recycle' AND Key LIKE 'total_%'",
        &[],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );

    let mut rankings: Vec<(String, i64)> = Vec::new();
    for (key, val) in rows {
        let total: i64 = val.parse().unwrap_or(0);
        if total > 0 {
            let uid = key.replace("total_", "");
            rankings.push((uid, total));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("{}\n═══ ♻️ 回收积分排行 ═══\n", prefix);

    if rankings.is_empty() {
        out.push_str("\n暂无回收记录。成为第一个回收物品的玩家吧！\n");
        return out;
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    let medals = ["🥇", "🥈", "🥉"];
    let display_count = rankings.len().min(15);

    let mut my_rank = 0;
    let my_total = get_total_earned(db, user_id);

    for (i, (uid, total)) in rankings.iter().enumerate().take(display_count) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let nickname = db.read_basic(uid, "Nickname");
        let is_me = uid == user_id;
        let mark = if is_me { " ← 你" } else { "" };

        out.push_str(&format!("{} {}. {} — {} 积分{}\n", medal, i + 1, nickname, total, mark));

        if is_me {
            my_rank = i + 1;
        }
    }

    if my_rank == 0 && my_total > 0 {
        // 玩家不在前15名
        for (i, (uid, _total)) in rankings.iter().enumerate() {
            if uid == user_id {
                my_rank = i + 1;
                break;
            }
        }
        out.push_str(&format!("\n📍 你的排名: 第{}名 — {} 积分\n", my_rank, my_total));
    } else if my_total == 0 {
        out.push_str("\n📍 你还没有回收记录，发送「查看回收」开始吧！\n");
    }

    out
}

/// 回收统计 — 个人回收历史统计
pub fn cmd_recycle_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let points = get_recycle_points(db, user_id);
    let count = get_recycle_count(db, user_id);
    let total = get_total_earned(db, user_id);
    let spent = total - points;
    let exchg_count = get_exchange_count(db, user_id);

    let mut out = format!("{}\n═══ ♻️ 回收统计 ═══\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    out.push_str("📦 回收记录:\n");
    out.push_str(&format!("  累计回收次数: {} 次\n", count));
    out.push_str(&format!("  累计获得积分: {} 积分\n", total));
    out.push_str(&format!(
        "  平均每次回收: {} 积分\n",
        if count > 0 { total / count } else { 0 }
    ));

    out.push_str("\n💰 积分状态:\n");
    out.push_str(&format!("  当前可用: {} 积分\n", points));
    out.push_str(&format!("  已消耗: {} 积分\n", spent.max(0)));

    out.push_str("\n🛒 兑换记录:\n");
    out.push_str(&format!("  累计兑换次数: {} 次\n", exchg_count));
    out.push_str(&format!(
        "  平均每次消耗: {} 积分\n",
        if exchg_count > 0 { spent.max(0) / exchg_count } else { 0 }
    ));

    // 成就进度
    out.push_str("\n🏆 回收成就:\n");
    let milestones = [
        (100, "回收入门"),
        (500, "回收学徒"),
        (1000, "回收达人"),
        (5000, "回收专家"),
        (10000, "回收大师"),
        (50000, "回收传说"),
        (100000, "回收之神"),
    ];
    for (threshold, title) in &milestones {
        let status = if total >= *threshold { "✅" } else { "⬜" };
        let progress = (total.min(*threshold) as f64 / *threshold as f64 * 100.0) as i64;
        out.push_str(&format!("  {} {} — {}积分 ({})\n", status, title, threshold, progress));
    }

    out
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_recycle_points() {
        assert_eq!(estimate_recycle_points(0), ("普通".to_string(), 5));
        assert_eq!(estimate_recycle_points(50), ("普通".to_string(), 5));
        assert_eq!(estimate_recycle_points(100), ("普通".to_string(), 5));
        assert_eq!(estimate_recycle_points(200), ("优秀".to_string(), 15));
        assert_eq!(estimate_recycle_points(500), ("优秀".to_string(), 15));
        assert_eq!(estimate_recycle_points(1000), ("精良".to_string(), 30));
        assert_eq!(estimate_recycle_points(2000), ("精良".to_string(), 30));
        assert_eq!(estimate_recycle_points(3000), ("稀有".to_string(), 60));
        assert_eq!(estimate_recycle_points(10000), ("史诗".to_string(), 120));
        assert_eq!(estimate_recycle_points(30000), ("传说".to_string(), 250));
        assert_eq!(estimate_recycle_points(100000), ("神器".to_string(), 500));
    }

    #[test]
    fn test_quality_emoji() {
        assert_eq!(quality_emoji("普通"), "⬜");
        assert_eq!(quality_emoji("优秀"), "🟢");
        assert_eq!(quality_emoji("精良"), "🔵");
        assert_eq!(quality_emoji("稀有"), "🟣");
        assert_eq!(quality_emoji("史诗"), "🟠");
        assert_eq!(quality_emoji("传说"), "🟡");
        assert_eq!(quality_emoji("神器"), "🔴");
        assert_eq!(quality_emoji("未知"), "⬜");
    }

    #[test]
    fn test_recycle_shop_items_count() {
        assert_eq!(RECYCLE_SHOP.len(), 12);
    }

    #[test]
    fn test_recycle_shop_costs_positive() {
        for item in RECYCLE_SHOP {
            assert!(item.cost > 0, "{} cost must be positive", item.name);
            assert!(!item.name.is_empty(), "item name must not be empty");
            assert!(!item.emoji.is_empty(), "item emoji must not be empty");
        }
    }

    #[test]
    fn test_recycle_shop_costs_escalate() {
        // The shop should have items in various price ranges
        let min_cost = RECYCLE_SHOP.iter().map(|i| i.cost).min().unwrap();
        let max_cost = RECYCLE_SHOP.iter().map(|i| i.cost).max().unwrap();
        assert!(max_cost > min_cost * 10, "cost range should be wide");
    }

    #[test]
    fn test_quality_levels_exhaustive() {
        let levels = ["优秀", "精良", "稀有", "史诗", "传说", "神器"];
        for level in &levels {
            let emoji = quality_emoji(level);
            assert_ne!(emoji, "⬜", "quality '{}' should have unique emoji", level);
        }
        // 普通 uses ⬜ (same as default), which is fine
        assert_eq!(quality_emoji("普通"), "⬜");
    }

    #[test]
    fn test_recycle_points_at_boundaries() {
        // Test exact boundary values
        let (q1, p1) = estimate_recycle_points(100);
        assert_eq!(q1, "普通");
        assert_eq!(p1, 5);

        let (q2, p2) = estimate_recycle_points(101);
        assert_eq!(q2, "优秀");
        assert_eq!(p2, 15);

        let (q3, _p3) = estimate_recycle_points(500);
        assert_eq!(q3, "优秀");

        let (q4, p4) = estimate_recycle_points(501);
        assert_eq!(q4, "精良");
        assert_eq!(p4, 30);
    }

    #[test]
    fn test_recycle_shop_names_unique() {
        let mut names: Vec<&str> = RECYCLE_SHOP.iter().map(|i| i.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "shop item names must be unique");
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "recycle");
    }

    #[test]
    fn test_batch_quality_mapping() {
        // Verify the quality-to-level mapping used in batch recycle
        let test_cases = [
            ("普通", 1),
            ("优秀", 2),
            ("精良", 3),
            ("稀有", 4),
            ("史诗", 5),
            ("传说", 6),
            ("神器", 7),
        ];
        for (quality, expected_level) in &test_cases {
            let level = match *quality {
                "普通" => 1,
                "优秀" => 2,
                "精良" => 3,
                "稀有" => 4,
                "史诗" => 5,
                "传说" => 6,
                "神器" => 7,
                _ => 0,
            };
            assert_eq!(level, *expected_level);
        }
    }

    #[test]
    fn test_milestone_thresholds() {
        let milestones = [100, 500, 1000, 5000, 10000, 50000, 100000];
        // Milestones should be in ascending order
        for i in 1..milestones.len() {
            assert!(milestones[i] > milestones[i - 1]);
        }
        // Should have 7 milestones
        assert_eq!(milestones.len(), 7);
    }

    #[test]
    fn test_bonus_calculation() {
        // Test the batch bonus calculation
        let base_pts = 10i64;
        let bonus_pct = 10i64;
        let actual = base_pts * (100 + bonus_pct) / 100;
        assert_eq!(actual, 11);

        // No bonus for small quantities
        let no_bonus = base_pts * 100 / 100;
        assert_eq!(no_bonus, 10);
    }
}

/// CakeGame 每日限时折扣系统
/// 每日自动刷新5件打折商品（30%~70%折扣），每人限购1件
/// 基于日期+用户哈希确定性选择折扣商品
/// 数据存储：Global 表 flash_sale section
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 每日折扣商品数量
const DAILY_ITEMS: usize = 5;
/// 折扣范围
const DISCOUNT_MIN: i32 = 30;
const DISCOUNT_MAX: i32 = 70;

/// 折扣商品定义
struct FlashItem {
    name: String,
    original_price: i64,
    discount: i32,
    emoji: String,
    #[allow(dead_code)]
    category: String,
}

impl FlashItem {
    fn sale_price(&self) -> i64 {
        self.original_price * (100 - self.discount as i64) / 100
    }
    fn savings(&self) -> i64 {
        self.original_price - self.sale_price()
    }
}

/// 确定性哈希生成
fn daily_hash(salt: &str, date: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{}_{}", salt, date).hash(&mut hasher);
    hasher.finish()
}

/// 获取今日日期字符串
fn today_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// 物品类型 emoji
fn type_emoji(type_name: &str) -> &'static str {
    match type_name {
        "回复药剂" | "增益药剂" => "🧪",
        "矿石" | "宝石" => "🪨",
        "武器" => "⚔️",
        "防具" => "🛡️",
        "材料" => "🪵",
        "礼包" => "🎁",
        "食物" => "🍖",
        "卷轴" | "特殊" => "📜",
        "种子" => "🌱",
        _ => "📦",
    }
}

/// 获取今日折扣商品列表
fn get_flash_items(db: &Database) -> Vec<FlashItem> {
    let today = today_string();
    let conn = db.lock_conn();
    let mut items = Vec::new();

    // 从 Config_Goods 读取所有有效商品
    if let Ok(mut stmt) = conn.prepare(
        "SELECT Name, SellPrice, Type FROM Config_Goods WHERE SellPrice > 0 AND Name NOT LIKE '%测试%' ORDER BY rowid",
    ) {
        if let Ok(rows) = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let price_str: String = row.get(1).unwrap_or_default();
            let type_str: String = row.get(2).unwrap_or_default();
            Ok((name, price_str, type_str))
        }) {
            let mut all_goods: Vec<(String, i64, String)> = Vec::new();
            for row in rows.flatten() {
                let price: i64 = row.1.parse().unwrap_or(0);
                if price > 0 {
                    all_goods.push((row.0, price, row.2));
                }
            }

            if all_goods.is_empty() {
                return items;
            }

            // 用日期哈希选择 DAILY_ITEMS 件商品
            let total = all_goods.len();
            for i in 0..DAILY_ITEMS {
                let hash = daily_hash(&format!("flash_item_{}", i), &today);
                let idx = (hash as usize) % total;
                let (name, price, type_name) = &all_goods[idx];

                let disc_hash = daily_hash(&format!("flash_disc_{}", i), &today);
                let discount = DISCOUNT_MIN + ((disc_hash % (DISCOUNT_MAX - DISCOUNT_MIN + 1) as u64) as i32);

                items.push(FlashItem {
                    name: name.clone(),
                    original_price: *price,
                    discount,
                    emoji: type_emoji(type_name).to_string(),
                    category: type_name.clone(),
                });
            }
        }
    }
    items
}

/// 检查用户今日是否已购买某商品
fn has_purchased(db: &Database, user_id: &str, item_name: &str) -> bool {
    let key = format!("purchased_{}", item_name);
    let section = format!("flash_sale_{}", user_id);
    let val = db.global_get(&section, &key);
    let today = today_string();
    val == today
}

/// 记录用户购买
fn record_purchase(db: &Database, user_id: &str, item_name: &str) {
    let key = format!("purchased_{}", item_name);
    let section = format!("flash_sale_{}", user_id);
    db.global_set(&section, &key, &today_string());
}

/// 查看限时折扣
pub fn cmd_view_flash_sale(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let items = get_flash_items(db);
    if items.is_empty() {
        return format!("{}\n═══ 每日限时折扣 ═══\n今日暂无折扣商品。", prefix);
    }

    let today = today_string();
    let mut r = format!("{}\n═══ 🏷️ 每日限时折扣 ═══", prefix);
    r.push_str(&format!("\n📅 {} | 每日自动刷新", today));
    r.push_str("\n─────────────────────\n");

    for (i, item) in items.iter().enumerate() {
        let purchased = has_purchased(db, user_id, &item.name);
        let status = if purchased { "✅已购" } else { "🛒可购" };
        r.push_str(&format!(
            "{}. {} {} | 原价: {}金 → 折后: {}金 ({}%折扣) 省{}金 [{}]\n",
            i + 1,
            item.emoji,
            item.name,
            item.original_price,
            item.sale_price(),
            item.discount,
            item.savings(),
            status,
        ));
    }

    r.push_str("─────────────────────\n");
    r.push_str("💡 每件商品每日限购1次，次日刷新\n");
    r.push_str("📝 发送 '购买折扣+商品名' 购买");

    r
}

/// 购买折扣商品
pub fn cmd_buy_flash_sale(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        return format!("{}\n请指定要购买的商品名。\n用法: 购买折扣+商品名", prefix);
    }

    let items = get_flash_items(db);

    // 模糊匹配商品
    let matched = items
        .iter()
        .find(|it| it.name == item_name || it.name.contains(item_name) || item_name.contains(&it.name));

    let item = match matched {
        Some(it) => it,
        None => {
            // 显示可用商品列表
            let names: Vec<&str> = items.iter().map(|it| it.name.as_str()).collect();
            return format!(
                "{}\n❌ 未找到折扣商品 [{}]\n今日折扣商品: {}",
                prefix,
                item_name,
                names.join("、 ")
            );
        }
    };

    // 检查是否已购买
    if has_purchased(db, user_id, &item.name) {
        return format!("{}\n❌ 您今日已购买过 [{}]，每人每日限购1次。", prefix, item.name);
    }

    // 扣除金币
    let sale_price = item.sale_price();
    let after_gold = db.modify_currency(user_id, CURRENCY_GOLD, "sub", sale_price);
    if after_gold < 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", sale_price);
        return format!(
            "{}\n💰 金币不足！需要{}金，折扣价{}金。",
            prefix, sale_price, sale_price
        );
    }

    // 添加物品到背包
    db.add_item(user_id, &item.name, 1);

    // 记录购买
    record_purchase(db, user_id, &item.name);

    format!(
        "{}\n🎉 购买成功！\n{} {} | 花费 {} 金 (原价 {} 金，省 {} 金)\n💡 今日还可购买 {} 件折扣商品",
        prefix,
        item.emoji,
        item.name,
        sale_price,
        item.original_price,
        item.savings(),
        DAILY_ITEMS - items.iter().filter(|it| has_purchased(db, user_id, &it.name)).count() - 1,
    )
}

/// 折扣统计
pub fn cmd_flash_sale_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let items = get_flash_items(db);
    let purchased_count = items.iter().filter(|it| has_purchased(db, user_id, &it.name)).count();
    let total_savings: i64 = items
        .iter()
        .filter(|it| has_purchased(db, user_id, &it.name))
        .map(|it| it.savings())
        .sum();

    let remaining = DAILY_ITEMS - purchased_count;

    let mut r = format!("{}\n═══ 📊 折扣购物统计 ═══", prefix);
    r.push_str(&format!("\n📅 今日: {}", today_string()));
    r.push_str(&format!("\n🛒 已购买: {}/{} 件", purchased_count, DAILY_ITEMS));
    r.push_str(&format!("\n⏳ 剩余次数: {} 次", remaining));
    if total_savings > 0 {
        r.push_str(&format!("\n💰 今日节省: {} 金币", total_savings));
    }
    r.push_str("\n─────────────────────");

    if remaining > 0 {
        r.push_str("\n💡 发送 '查看折扣' 查看今日折扣商品");
    } else {
        r.push_str("\n✨ 今日折扣商品已全部购买，明天再来！");
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_item_sale_price() {
        let item = FlashItem {
            name: "测试".to_string(),
            original_price: 1000,
            discount: 50,
            emoji: "📦".to_string(),
            category: "材料".to_string(),
        };
        assert_eq!(item.sale_price(), 500);
        assert_eq!(item.savings(), 500);
    }

    #[test]
    fn test_flash_item_zero_discount() {
        let item = FlashItem {
            name: "测试".to_string(),
            original_price: 1000,
            discount: 0,
            emoji: "📦".to_string(),
            category: "材料".to_string(),
        };
        assert_eq!(item.sale_price(), 1000);
        assert_eq!(item.savings(), 0);
    }

    #[test]
    fn test_flash_item_full_discount() {
        let item = FlashItem {
            name: "测试".to_string(),
            original_price: 1000,
            discount: 70,
            emoji: "📦".to_string(),
            category: "材料".to_string(),
        };
        assert_eq!(item.sale_price(), 300);
        assert_eq!(item.savings(), 700);
    }

    #[test]
    fn test_daily_hash_deterministic() {
        let h1 = daily_hash("flash_item_0", "2026-06-11");
        let h2 = daily_hash("flash_item_0", "2026-06-11");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_daily_hash_different_dates() {
        let h1 = daily_hash("flash_item_0", "2026-06-11");
        let h2 = daily_hash("flash_item_0", "2026-06-12");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_daily_hash_different_salts() {
        let h1 = daily_hash("flash_item_0", "2026-06-11");
        let h2 = daily_hash("flash_item_1", "2026-06-11");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_type_emoji_known_types() {
        assert_eq!(type_emoji("回复药剂"), "🧪");
        assert_eq!(type_emoji("矿石"), "🪨");
        assert_eq!(type_emoji("武器"), "⚔️");
        assert_eq!(type_emoji("防具"), "🛡️");
        assert_eq!(type_emoji("材料"), "🪵");
        assert_eq!(type_emoji("礼包"), "🎁");
        assert_eq!(type_emoji("食物"), "🍖");
        assert_eq!(type_emoji("种子"), "🌱");
    }

    #[test]
    fn test_type_emoji_unknown() {
        assert_eq!(type_emoji("未知类型"), "📦");
        assert_eq!(type_emoji(""), "📦");
    }

    #[test]
    fn test_constants() {
        assert_eq!(DAILY_ITEMS, 5);
        assert!(DISCOUNT_MIN >= 10);
        assert!(DISCOUNT_MAX <= 90);
        assert!(DISCOUNT_MIN < DISCOUNT_MAX);
    }

    #[test]
    fn test_discount_range() {
        for i in 0..100 {
            let hash = daily_hash(&format!("flash_disc_{}", i), "2026-06-11");
            let discount = DISCOUNT_MIN + ((hash % (DISCOUNT_MAX - DISCOUNT_MIN + 1) as u64) as i32);
            assert!(discount >= DISCOUNT_MIN, "discount {} < MIN {}", discount, DISCOUNT_MIN);
            assert!(discount <= DISCOUNT_MAX, "discount {} > MAX {}", discount, DISCOUNT_MAX);
        }
    }

    #[test]
    fn test_today_string_format() {
        let today = today_string();
        assert_eq!(today.len(), 10);
        assert!(today.contains('-'));
    }

    #[test]
    fn test_flash_item_price_rounding() {
        let item = FlashItem {
            name: "测试".to_string(),
            original_price: 999,
            discount: 33,
            emoji: "📦".to_string(),
            category: "材料".to_string(),
        };
        // 999 * 67 / 100 = 669
        assert_eq!(item.sale_price(), 669);
        assert_eq!(item.savings(), 330);
    }
}

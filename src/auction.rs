/// CakeGame 拍卖行系统
/// 全服玩家物品交易市场：上架/竞拍/一口价/查看/下架
/// 数据来源: Shared_Data 表 SECTION='auction' 存储拍卖记录
use crate::core::CURRENCY_GOLD;
use crate::db::Database;
use crate::user;

/// 拍卖行 — 查看当前拍卖列表
pub fn cmd_view_auction(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let page: i32 = args.trim().parse().unwrap_or(1).max(1);
    let auctions = get_active_auctions(db);
    let page_size = 8;
    let total = auctions.len();
    let total_pages = ((total as i32) + page_size - 1) / page_size;
    let page = page.min(total_pages.max(1));
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(total);

    let mut out = format!("{}\n═══ 🏪 全服拍卖行 ═══", prefix);
    if auctions.is_empty() {
        out.push_str("\n\n暂无在拍物品\n发送'上架拍卖+物品名+价格'来上架物品！");
    } else {
        out.push_str(&format!("\n📦 共 {} 件在拍物品\n", total));
        for (i, a) in auctions[start..end].iter().enumerate() {
            let idx = start + i + 1;
            out.push_str(&format!(
                "\n{}. [{}] ×{} 💰{}金币\n   卖家: {} | {}",
                idx,
                a.item_name,
                a.quantity,
                a.price,
                mask_name(&a.seller),
                format_remaining_time(a.expire_ts)
            ));
        }
        out.push_str(&format!("\n\n📄 页 {}/{} | 发送'拍卖行+页码'翻页", page, total_pages));
        out.push_str("\n💡 发送'购买+序号'一口价购买");
    }
    out
}

/// 上架拍卖 — 将背包物品上架到拍卖行
pub fn cmd_list_auction(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return format!(
            "{}\n📤 上架拍卖用法：上架拍卖+物品名+价格+数量\n💡 示例：上架拍卖+生命药水+500\n   数量默认1，可指定：上架拍卖+生命药水+500+3",
            prefix
        );
    }

    let item_name = parts[0];
    let price: i64 = match parts[1].parse() {
        Ok(p) if p > 0 => p,
        _ => return format!("{}\n⚠️ 价格必须大于0！", prefix),
    };
    let quantity: i32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);

    if price > 99_999_999 {
        return format!("{}\n⚠️ 价格不能超过9999万金币！", prefix);
    }

    // 检查背包中是否有该物品
    let knapsack = db.knapsack_all(user_id);
    let mut current_count = 0;
    for item in &knapsack {
        if item.name == item_name {
            current_count = item.quantity;
            break;
        }
    }
    if current_count < quantity {
        return format!(
            "{}\n⚠️ 背包中没有足够的[{}]！当前数量: {}",
            prefix, item_name, current_count
        );
    }

    // 每人最多同时上架10件
    let my_auctions = get_user_active_auctions(db, user_id);
    if my_auctions.len() >= 10 {
        return format!("{}\n⚠️ 您同时上架的拍卖已达上限(10件)！请先下架或等待成交。", prefix);
    }

    // 从背包扣除物品
    if !db.knapsack_remove(user_id, item_name, quantity) {
        return format!("{}\n⚠️ 物品扣除失败！", prefix);
    }

    // 生成拍卖ID
    let auction_id = generate_auction_id(user_id);
    let now = chrono_now();
    let expire = now + 24 * 3600; // 24小时后过期

    // 存储到 Shared_Data
    let data = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        auction_id, user_id, item_name, quantity, price, now, expire
    );
    save_auction(db, &auction_id, &data);

    let fee = (price * quantity as i64 * 5 / 100).max(1); // 5%手续费
    format!(
        "{}\n✅ 拍卖上架成功！\n\n📦 物品: [{}] ×{}\n💰 起拍价: {}金币\n📋 拍卖ID: {}\n⏰ 持续时间: 24小时\n💸 预计手续费: {}金币 (5%)\n\n💡 成交后扣除手续费，金币通过邮件发送",
        prefix, item_name, quantity, price, auction_id, fee
    )
}

/// 下架拍卖 — 取消自己的拍卖
pub fn cmd_cancel_auction(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let auction_id = args.trim();
    if auction_id.is_empty() {
        // 显示自己的拍卖列表
        let my_auctions = get_user_active_auctions(db, user_id);
        if my_auctions.is_empty() {
            return format!("{}\n📤 您暂无在拍物品。", prefix);
        }
        let mut out = format!("{}\n═══ 📤 我的拍卖 ═══", prefix);
        for (i, a) in my_auctions.iter().enumerate() {
            out.push_str(&format!(
                "\n{}. [{}] ×{} 💰{}金币 | ID: {} | {}",
                i + 1,
                a.item_name,
                a.quantity,
                a.price,
                a.auction_id,
                format_remaining_time(a.expire_ts)
            ));
        }
        out.push_str("\n\n💡 发送'下架拍卖+拍卖ID'取消拍卖");
        return out;
    }

    // 查找并取消拍卖
    let auctions = get_user_active_auctions(db, user_id);
    let target = auctions.iter().find(|a| a.auction_id == auction_id);
    match target {
        Some(a) => {
            let item_name = a.item_name.clone();
            let qty = a.quantity;
            // 归还物品到背包
            db.knapsack_add(user_id, &item_name, qty);
            // 更新状态
            cancel_auction_record(db, auction_id);
            format!(
                "{}\n✅ 拍卖已取消！\n📦 [{}] ×{} 已归还到背包。",
                prefix, item_name, qty
            )
        }
        None => format!("{}\n⚠️ 未找到拍卖ID: {}（可能已成交或不属于您）", prefix, auction_id),
    }
}

/// 购买拍卖 — 一口价购买
pub fn cmd_buy_auction(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let idx_str = args.trim();
    if idx_str.is_empty() {
        return format!(
            "{}\n💡 发送'购买+序号'一口价购买拍卖物品\n   先发送'拍卖行'查看列表",
            prefix
        );
    }

    let idx: usize = match idx_str.parse::<usize>() {
        Ok(i) if i > 0 => i - 1,
        _ => return format!("{}\n⚠️ 请输入有效序号！", prefix),
    };

    let auctions = get_active_auctions(db);
    if idx >= auctions.len() {
        return format!("{}\n⚠️ 序号超出范围！当前共{}件在拍物品。", prefix, auctions.len());
    }

    let auction = &auctions[idx];

    // 不能购买自己的拍卖
    if auction.seller == user_id {
        return format!("{}\n⚠️ 不能购买自己的拍卖物品！", prefix);
    }

    // 检查金币是否足够
    let buyer_gold = db.read_basic(user_id, CURRENCY_GOLD).parse::<i64>().unwrap_or(0);
    let total_price = auction.price * auction.quantity as i64;
    if buyer_gold < total_price {
        return format!(
            "{}\n⚠️ 金币不足！需要 {}金币，您有 {}金币。",
            prefix, total_price, buyer_gold
        );
    }

    // 扣除买家金币
    db.write_basic(user_id, CURRENCY_GOLD, &(buyer_gold - total_price).to_string());

    // 给卖家金币（扣除5%手续费）
    let fee = (total_price * 5 / 100).max(1);
    let seller_income = total_price - fee;
    let seller_gold = db
        .read_basic(&auction.seller, CURRENCY_GOLD)
        .parse::<i64>()
        .unwrap_or(0);
    db.write_basic(
        &auction.seller,
        CURRENCY_GOLD,
        &(seller_gold + seller_income).to_string(),
    );

    // 给买家物品
    db.knapsack_add(user_id, &auction.item_name, auction.quantity);
    // 记录购买历史
    crate::purchase_history::record_purchase(
        db,
        user_id,
        &auction.seller,
        &auction.item_name,
        total_price,
        "金币",
        auction.quantity,
    );

    // 更新拍卖状态
    mark_auction_sold(db, &auction.auction_id, user_id, total_price);

    let item_name = auction.item_name.clone();
    let qty = auction.quantity;
    format!(
        "{}\n✅ 购买成功！\n\n📦 获得: [{}] ×{}\n💰 花费: {}金币\n\n💡 物品已放入背包",
        prefix, item_name, qty, total_price
    )
}

/// 搜索拍卖 — 按物品名搜索
pub fn cmd_search_auction(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let keyword = args.trim();
    if keyword.is_empty() {
        return format!("{}\n💡 发送'搜索拍卖+物品名'搜索拍卖物品", prefix);
    }

    let auctions = get_active_auctions(db);
    let results: Vec<&AuctionItem> = auctions.iter().filter(|a| a.item_name.contains(keyword)).collect();

    let mut out = format!("{}\n═══ 🔍 拍卖搜索 [{}] ═══", prefix, keyword);
    if results.is_empty() {
        out.push_str(&format!("\n\n未找到包含[{}]的拍卖物品", keyword));
    } else {
        out.push_str(&format!("\n📦 找到 {} 件物品\n", results.len()));
        for (i, a) in results.iter().enumerate().take(20) {
            out.push_str(&format!(
                "\n{}. [{}] ×{} 💰{}金币 | 卖家: {} | {}",
                i + 1,
                a.item_name,
                a.quantity,
                a.price,
                mask_name(&a.seller),
                format_remaining_time(a.expire_ts)
            ));
        }
    }
    out
}

// ==================== 内部数据结构 ====================

struct AuctionItem {
    auction_id: String,
    seller: String,
    item_name: String,
    quantity: i32,
    price: i64,
    #[allow(dead_code)]
    create_ts: i64,
    expire_ts: i64,
}

/// 获取所有活跃拍卖
fn get_active_auctions(db: &Database) -> Vec<AuctionItem> {
    let mut auctions = Vec::new();
    let now = chrono_now();

    if let Ok(conn) = db.conn.lock() {
        if let Ok(mut stmt) = conn.prepare("SELECT ID, DATA FROM Shared_Data WHERE SECTION = 'auction'") {
            if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
                for row in rows.flatten() {
                    if let Some(item) = parse_auction_data(&row.1) {
                        if item.expire_ts > now {
                            auctions.push(item);
                        }
                    }
                }
            }
        }
    }

    // 按价格升序排序
    auctions.sort_by_key(|a| a.price);
    auctions
}

/// 获取用户的活跃拍卖
fn get_user_active_auctions(db: &Database, user_id: &str) -> Vec<AuctionItem> {
    get_active_auctions(db)
        .into_iter()
        .filter(|a| a.seller == user_id)
        .collect()
}

/// 解析拍卖数据
fn parse_auction_data(data: &str) -> Option<AuctionItem> {
    let parts: Vec<&str> = data.split('|').collect();
    if parts.len() < 7 {
        return None;
    }
    Some(AuctionItem {
        auction_id: parts[0].to_string(),
        seller: parts[1].to_string(),
        item_name: parts[2].to_string(),
        quantity: parts[3].parse().unwrap_or(1),
        price: parts[4].parse().unwrap_or(0),
        create_ts: parts[5].parse().unwrap_or(0),
        expire_ts: parts[6].parse().unwrap_or(0),
    })
}

/// 保存拍卖记录到 Shared_Data
fn save_auction(db: &Database, auction_id: &str, data: &str) {
    if let Ok(conn) = db.conn.lock() {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, 'auction', ?2)",
            rusqlite::params![format!("auction.{}", auction_id), data],
        );
    }
}

/// 取消拍卖记录
fn cancel_auction_record(db: &Database, auction_id: &str) {
    if let Ok(conn) = db.conn.lock() {
        let _ = conn.execute(
            "DELETE FROM Shared_Data WHERE ID=?1 AND SECTION='auction'",
            rusqlite::params![format!("auction.{}", auction_id)],
        );
    }
}

/// 标记拍卖已售出
fn mark_auction_sold(db: &Database, auction_id: &str, buyer: &str, price: i64) {
    if let Ok(conn) = db.conn.lock() {
        let data = format!("sold|{}|{}|{}", buyer, price, chrono_now());
        let _ = conn.execute(
            "UPDATE Shared_Data SET SECTION='auction_sold', DATA=?1 WHERE ID=?2 AND SECTION='auction'",
            rusqlite::params![data, format!("auction.{}", auction_id)],
        );
    }
}

/// 生成拍卖ID
fn generate_auction_id(user_id: &str) -> String {
    let now = chrono_now();
    let uid_hash: u64 = user_id
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let hash = (now as u64).wrapping_mul(6364136223846793005).wrapping_add(uid_hash);
    format!("A{:08X}", hash & 0xFFFFFFFF)
}

/// 获取当前时间戳
fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 格式化剩余时间
fn format_remaining_time(expire_ts: i64) -> String {
    let now = chrono_now();
    let remaining = expire_ts - now;
    if remaining <= 0 {
        return "已过期".to_string();
    }
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    if hours > 0 {
        format!("剩余{}h{}m", hours, minutes)
    } else {
        format!("剩余{}m", minutes)
    }
}

/// 遮蔽名字
fn mask_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= 2 {
        return name.to_string();
    }
    format!("{}*{}", chars[0], chars[chars.len() - 1])
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_auction_id() {
        let id1 = generate_auction_id("user1");
        let id2 = generate_auction_id("a_very_long_username");
        assert!(id1.starts_with('A'));
        assert!(id2.starts_with('A'));
        // Different inputs produce different hashes (may be same due to timing, so just check format)
        assert!(id1.len() == 9); // A + 8 hex chars
        assert!(id2.len() == 9);
    }

    #[test]
    fn test_parse_auction_data() {
        let data = "A12345678|seller1|生命药水|5|1000|1700000000|1700086400";
        let item = parse_auction_data(data).unwrap();
        assert_eq!(item.auction_id, "A12345678");
        assert_eq!(item.seller, "seller1");
        assert_eq!(item.item_name, "生命药水");
        assert_eq!(item.quantity, 5);
        assert_eq!(item.price, 1000);
        assert_eq!(item.expire_ts, 1700086400);
    }

    #[test]
    fn test_parse_auction_data_invalid() {
        assert!(parse_auction_data("invalid").is_none());
        assert!(parse_auction_data("a|b|c").is_none());
    }

    #[test]
    fn test_mask_name() {
        assert_eq!(mask_name("测试员"), "测*员");
        assert_eq!(mask_name("ab"), "ab");
        assert_eq!(mask_name("a"), "a");
        assert_eq!(mask_name("玩家名字"), "玩*字");
    }

    #[test]
    fn test_format_remaining_time() {
        let now = chrono_now();
        assert_eq!(format_remaining_time(now + 3661), "剩余1h1m");
        assert_eq!(format_remaining_time(now + 300), "剩余5m");
        assert_eq!(format_remaining_time(now - 1), "已过期");
    }

    #[test]
    fn test_auction_fee_calculation() {
        let total = 10000i64;
        let fee = (total * 5 / 100).max(1);
        assert_eq!(fee, 500);
        // Small amounts still have minimum fee
        let small = 10i64;
        let fee_small = (small * 5 / 100).max(1);
        assert_eq!(fee_small, 1);
    }
}

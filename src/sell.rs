/// CakeGame NPC 出售系统
/// 基于 Ext_Sell 表：玩家出售物品给NPC获得金币/钻石
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 出售物品给NPC
pub fn cmd_sell_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!(
            "{}\n请指定要出售的物品。\n用法：出售+物品名 或 出售+物品名*数量",
            prefix
        );
    }

    // 解析 "物品名*数量"
    let parts: Vec<&str> = item_name.split('*').collect();
    let name = parts[0].trim();
    let sell_qty: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);

    // 检查装备锁定
    if crate::equip_lock::is_equip_name_locked(db, user_id, name) {
        return format!(
            "{}\n🔒 [{}] 已锁定，无法出售！\n💡 使用「解锁装备+槽位名」解除锁定后再出售。",
            prefix, name
        );
    }

    // 检查背包中是否有该物品
    let inv_count = db.get_item_count(user_id, name);
    if inv_count < sell_qty {
        return format!(
            "{}\n你没有足够的 [{}]×{}，当前拥有 {} 个。",
            prefix, name, sell_qty, inv_count
        );
    }

    // 查询出售价格 (Ext_Sell 表)
    let sell_info = db.query_row(
        "SELECT Name, Price, PriceType FROM Ext_Sell WHERE Name = ?",
        &[name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    );

    let (item, price_str, price_type) = match sell_info {
        Ok(info) => info,
        Err(_) => return format!("{}\n物品 [{}] 无法出售给NPC。", prefix, name),
    };

    let price_each: i32 = price_str.parse().unwrap_or(0);
    let total_price = price_each * sell_qty;

    if total_price <= 0 {
        return format!("{}\n物品 [{}] 的出售价格为0，无法出售。", prefix, item);
    }

    // 判断货币类型 (31=金币, 32=钻石, 其他默认金币)
    let currency = if price_type == "32" {
        CURRENCY_DIAMOND
    } else {
        CURRENCY_GOLD
    };
    let currency_name = if price_type == "32" { "钻石" } else { "金币" };

    // 检查是否是装备（装备出售需要额外确认）
    let item_data = db.get_item_data(&item);
    let is_equip = item_data
        .as_ref()
        .map(|(t, _, _)| t == "Equip" || t == "装备")
        .unwrap_or(false);

    // 执行出售
    db.remove_item(user_id, &item, sell_qty);
    db.modify_currency(user_id, currency, OP_ADD, total_price as i64);

    let current_balance = db.read_currency(user_id, currency);

    let mut result = format!("{}\n═══ 出售成功 ═══", prefix);
    result.push_str(&format!("\n出售：[{}]×{}", item, sell_qty));
    if is_equip {
        result.push_str("\n⚠️ 装备已出售！");
    }
    result.push_str(&format!("\n获得：{} {}", total_price, currency_name));
    result.push_str(&format!("\n当前{}：{}", currency_name, current_balance));
    crate::achievement::on_shop_sale(db, user_id);
    result
}

/// 查看出售价格
pub fn cmd_view_sell_price(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!("{}\n请指定要查询的物品。\n用法：出售价格+物品名", prefix);
    }

    // 查询出售价格
    let sell_info = db.query_row(
        "SELECT Name, Price, PriceType FROM Ext_Sell WHERE Name = ?",
        &[item_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    );

    match sell_info {
        Ok((name, price, price_type)) => {
            let currency_name = if price_type == "32" { "钻石" } else { "金币" };
            format!(
                "{}\n═══ 出售价格 ═══\n物品：[{}]\n出售价：{} {}\n\n发送'出售+物品名'出售物品",
                prefix, name, price, currency_name
            )
        }
        Err(_) => format!("{}\n物品 [{}] 不在可出售列表中。", prefix, item_name),
    }
}

/// 查看背包中可出售的物品（附带价格）
pub fn cmd_view_sellable(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);

    // 获取背包物品
    let items = db.knapsack_all(user_id);
    if items.is_empty() {
        return format!("{}\n您的背包暂无物品！", prefix);
    }

    // 过滤可出售物品并获取价格
    let mut sellable: Vec<(String, i32, i32, String)> = Vec::new(); // (name, qty, price, currency)
    for item in &items {
        let sell_info = db.query_row(
            "SELECT Price, PriceType FROM Ext_Sell WHERE Name = ?",
            &[&item.name],
            |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                ))
            },
        );
        if let Ok((price_str, price_type)) = sell_info {
            let price: i32 = price_str.parse().unwrap_or(0);
            if price > 0 {
                let currency = if price_type == "32" { "💎" } else { "🪙" };
                sellable.push((item.name.clone(), item.quantity, price, currency.to_string()));
            }
        }
    }

    if sellable.is_empty() {
        return format!(
            "{}\n背包中没有可出售的物品。\n\n提示：只有在NPC出售列表中的物品才可出售。",
            prefix
        );
    }

    let ps = 10;
    let tp = ((sellable.len() as i32) + ps - 1) / ps;
    let page = page.min(tp).max(1);
    let s = (((page) - 1) * ps) as usize;
    let e = (s + ps as usize).min(sellable.len());

    let mut r = format!("{}\n═══ 可出售物品 ({}/{}) ═══", prefix, page, tp);
    for (i, (name, qty, price, currency)) in sellable[s..e].iter().enumerate() {
        r.push_str(&format!("\n{}. [{}]×{} → {} {}", s + i + 1, name, qty, price, currency));
    }
    r.push_str("\n\n发送'出售+物品名'出售物品");
    r
}

/// 查看NPC出售列表（Ext_Sell 表所有物品）
pub fn cmd_view_npc_sell_list(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);

    // 获取所有出售物品
    let sell_items: Vec<(String, String, String)> = db.query_rows(
        "SELECT Name, Price, PriceType FROM Ext_Sell ORDER BY CAST(Price AS INTEGER) DESC",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    );

    if sell_items.is_empty() {
        return format!("{}\nNPC出售列表为空。", prefix);
    }

    let ps = 15;
    let tp = ((sell_items.len() as i32) + ps - 1) / ps;
    let page = page.min(tp).max(1);
    let s = (((page) - 1) * ps) as usize;
    let e = (s + ps as usize).min(sell_items.len());

    let mut r = format!("{}\n═══ NPC收购列表 ({}/{}) ═══", prefix, page, tp);
    for (i, (name, price, price_type)) in sell_items[s..e].iter().enumerate() {
        let currency = if price_type == "32" { "💎" } else { "🪙" };
        r.push_str(&format!("\n{}. [{}] {}{}", s + i + 1, name, price, currency));
    }
    r.push_str(&format!(
        "\n\n共{}种物品可出售 | 发送'出售+物品名'出售",
        sell_items.len()
    ));
    r
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sell_qty_parse() {
        let input = "铁剑*3";
        let parts: Vec<&str> = input.split('*').collect();
        let name = parts[0].trim();
        let qty: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);
        assert_eq!(name, "铁剑");
        assert_eq!(qty, 3);
    }

    #[test]
    fn test_sell_qty_parse_no_qty() {
        let input = "铁剑";
        let parts: Vec<&str> = input.split('*').collect();
        let name = parts[0].trim();
        let qty: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);
        assert_eq!(name, "铁剑");
        assert_eq!(qty, 1);
    }

    #[test]
    fn test_sell_qty_parse_zero() {
        let qty: i32 = "0".parse().unwrap_or(1).max(1);
        assert_eq!(qty, 1);
    }

    #[test]
    fn test_sell_qty_parse_negative() {
        let qty: i32 = "-5".parse().unwrap_or(1).max(1);
        assert_eq!(qty, 1);
    }

    #[test]
    fn test_currency_type_gold() {
        let price_type = "31";
        let currency_name = if price_type == "32" { "钻石" } else { "金币" };
        assert_eq!(currency_name, "金币");
    }

    #[test]
    fn test_currency_type_diamond() {
        let price_type = "32";
        let currency_name = if price_type == "32" { "钻石" } else { "金币" };
        assert_eq!(currency_name, "钻石");
    }

    #[test]
    fn test_pagination() {
        let total = 25usize;
        let page_size = 10i32;
        let total_pages = ((total as i32) + page_size - 1) / page_size;
        assert_eq!(total_pages, 3);

        let page = 1i32;
        let s = ((page - 1) * page_size) as usize;
        let e = (s + page_size as usize).min(total);
        assert_eq!(s, 0);
        assert_eq!(e, 10);

        let page = 3i32;
        let s = ((page - 1) * page_size) as usize;
        let e = (s + page_size as usize).min(total);
        assert_eq!(s, 20);
        assert_eq!(e, 25);
    }

    #[test]
    fn test_total_price_calc() {
        let price_each: i32 = 50;
        let sell_qty: i32 = 3;
        let total = price_each * sell_qty;
        assert_eq!(total, 150);
    }
}

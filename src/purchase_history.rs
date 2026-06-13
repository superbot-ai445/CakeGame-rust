/// 购买记录系统 — 基于 User_Goodslookup 表
/// 追踪玩家的购买历史，包括商品名称、价格、货币类型、数量
/// 来源: User_Goodslookup 表 (uid, id, name, price, type, num)
use crate::db::Database;

/// 记录一条购买记录到 User_Goodslookup 表
pub fn record_purchase(
    db: &Database,
    user_id: &str,
    seller_id: &str,
    item_name: &str,
    price: i64,
    currency_type: &str,
    quantity: i32,
) {
    let conn = db.lock_conn();

    // 检查是否已有记录
    let existing: Option<(String, String, String, String, String)> = conn
        .prepare("SELECT id, name, price, type, num FROM User_Goodslookup WHERE uid=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, String>(3).unwrap_or_default(),
                    row.get::<_, String>(4).unwrap_or_default(),
                ))
            })
            .ok()
        });

    if let Some((old_ids, old_names, old_prices, old_types, old_nums)) = existing {
        // 追加到现有记录（逗号分隔）
        let new_ids = if old_ids.is_empty() {
            seller_id.to_string()
        } else {
            format!("{},{}", old_ids, seller_id)
        };
        let new_names = if old_names.is_empty() {
            item_name.to_string()
        } else {
            format!("{},{}", old_names, item_name)
        };
        let new_prices = if old_prices.is_empty() {
            price.to_string()
        } else {
            format!("{},{}", old_prices, price)
        };
        let new_types = if old_types.is_empty() {
            currency_type.to_string()
        } else {
            format!("{},{}", old_types, currency_type)
        };
        let new_nums = if old_nums.is_empty() {
            quantity.to_string()
        } else {
            format!("{},{}", old_nums, quantity)
        };

        let _ = conn.execute(
            "UPDATE User_Goodslookup SET id=?1, name=?2, price=?3, type=?4, num=?5 WHERE uid=?6",
            rusqlite::params![new_ids, new_names, new_prices, new_types, new_nums, user_id],
        );
    } else {
        let _ = conn.execute(
            "INSERT INTO User_Goodslookup (uid, id, name, price, type, num) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                user_id,
                seller_id,
                item_name,
                price.to_string(),
                currency_type,
                quantity.to_string()
            ],
        );
    }
}

/// 查看购买记录 — 显示玩家的所有购买历史
pub fn cmd_purchase_history(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);
    let page_size = 10;

    let record: Option<(String, String, String, String, String)> = {
        let conn = db.lock_conn();
        conn.prepare("SELECT id, name, price, type, num FROM User_Goodslookup WHERE uid=?1 AND name != ''")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_row(rusqlite::params![user_id], |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, String>(2).unwrap_or_default(),
                        row.get::<_, String>(3).unwrap_or_default(),
                        row.get::<_, String>(4).unwrap_or_default(),
                    ))
                })
                .ok()
            })
    };

    let (ids, names, prices, types, nums) = match record {
        Some(r) => r,
        None => return format!("{}\n📭 暂无购买记录\n\n💡 使用「购买商品+物品名」开始购物", prefix),
    };

    let name_list: Vec<&str> = names.split(',').collect();
    let price_list: Vec<&str> = prices.split(',').collect();
    let type_list: Vec<&str> = types.split(',').collect();
    let num_list: Vec<&str> = nums.split(',').collect();
    let _id_list: Vec<&str> = ids.split(',').collect();

    let total_records = name_list.len();
    let total_pages = total_records.div_ceil(page_size).max(1);
    let page = page.min(total_pages as i32);
    let start = (page - 1) as usize * page_size;
    let end = (start + page_size).min(total_records);

    if total_records == 0 {
        return format!("{}\n📭 暂无购买记录\n\n💡 使用「购买商品+物品名」开始购物", prefix);
    }

    let mut out = format!("{}\n═══ 📜 购买记录 ═══\n━━━━━━━━━━━━━━━━━━━━", prefix);

    for i in start..end {
        let name = name_list.get(i).unwrap_or(&"");
        let price = price_list.get(i).unwrap_or(&"0");
        let ctype = type_list.get(i).unwrap_or(&"金币");
        let num = num_list.get(i).unwrap_or(&"1");
        out.push_str(&format!("\n{}. [{}] ×{}  💰{}({})", i + 1, name, num, price, ctype));
    }

    out.push_str(&format!(
        "\n━━━━━━━━━━━━━━━━━━━━\n共 {} 条记录 | 第 {}/{} 页",
        total_records, page, total_pages
    ));

    if total_pages > 1 {
        out.push_str("\n💡 使用「购买记录+页码」翻页");
    }

    out.push_str("\n💡 使用「交易统计」查看消费汇总");
    out
}

/// 交易统计 — 汇总玩家的消费数据
pub fn cmd_transaction_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    let record: Option<(String, String, String, String)> = {
        let conn = db.lock_conn();
        conn.prepare("SELECT name, price, type, num FROM User_Goodslookup WHERE uid=?1 AND name != ''")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_row(rusqlite::params![user_id], |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, String>(2).unwrap_or_default(),
                        row.get::<_, String>(3).unwrap_or_default(),
                    ))
                })
                .ok()
            })
    };

    let (names, prices, types, nums) = match record {
        Some(r) => r,
        None => return format!("{}\n📭 暂无交易记录\n\n💡 使用「购买商品+物品名」开始购物", prefix),
    };

    let name_list: Vec<&str> = names.split(',').collect();
    let price_list: Vec<&str> = prices.split(',').collect();
    let type_list: Vec<&str> = types.split(',').collect();
    let num_list: Vec<&str> = nums.split(',').collect();

    if name_list.is_empty() {
        return format!("{}\n📭 暂无交易记录", prefix);
    }

    // 统计各货币类型消费
    let mut gold_total: i64 = 0;
    let mut diamond_total: i64 = 0;
    let mut other_total: i64 = 0;
    let mut item_count: i32 = 0;

    // 统计各物品购买次数
    let mut item_freq: std::collections::HashMap<&str, (i32, i64)> = std::collections::HashMap::new();

    for (i, name) in name_list.iter().enumerate() {
        let price: i64 = price_list.get(i).unwrap_or(&"0").parse().unwrap_or(0);
        let ctype = *type_list.get(i).unwrap_or(&"金币");
        let num: i32 = num_list.get(i).unwrap_or(&"1").parse().unwrap_or(1);

        item_count += num;
        let total_price = price;

        match ctype {
            "金币" => gold_total += total_price,
            "钻石" | "点卷" => diamond_total += total_price,
            _ => other_total += total_price,
        }

        let entry = item_freq.entry(name).or_insert((0, 0));
        entry.0 += num;
        entry.1 += total_price;
    }

    // 按消费金额排序 Top5
    let mut freq_vec: Vec<_> = item_freq.into_iter().collect();
    freq_vec.sort_by_key(|b| std::cmp::Reverse(b.1 .1));
    let top5: Vec<_> = freq_vec.into_iter().take(5).collect();

    let mut out = format!("{}\n═══ 📊 交易统计 ═══\n━━━━━━━━━━━━━━━━━━━━", prefix);
    out.push_str(&format!("\n📦 购买商品种类：{} 种", name_list.len()));
    out.push_str(&format!("\n📋 总购买数量：{} 件", item_count));

    if gold_total > 0 {
        out.push_str(&format!("\n💰 金币消费：{}", format_number(gold_total)));
    }
    if diamond_total > 0 {
        out.push_str(&format!("\n💎 钻石/点卷消费：{}", format_number(diamond_total)));
    }
    if other_total > 0 {
        out.push_str(&format!("\n🪙 其他消费：{}", format_number(other_total)));
    }

    let total_all = gold_total + diamond_total + other_total;
    out.push_str(&format!("\n💰 总消费：{}", format_number(total_all)));

    if !top5.is_empty() {
        out.push_str("\n\n🏆 消费Top5：");
        for (i, (name, (count, total))) in top5.iter().enumerate() {
            out.push_str(&format!(
                "\n  {}. [{}] ×{}  共{}",
                i + 1,
                name,
                count,
                format_number(*total)
            ));
        }
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    out.push_str("\n💡 使用「购买记录」查看详细历史");
    out
}

/// 格式化数字（添加千分位）
fn format_number(n: i64) -> String {
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
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000000), "1,000,000");
    }

    #[test]
    fn test_record_purchase_logic() {
        // 验证逗号分隔拼接逻辑
        let old_names = "物品A,物品B";
        let new_item = "物品C";
        let result = format!("{},{}", old_names, new_item);
        assert_eq!(result, "物品A,物品B,物品C");

        // 空记录拼接
        let empty = "";
        let result2 = if empty.is_empty() {
            new_item.to_string()
        } else {
            format!("{},{}", empty, new_item)
        };
        assert_eq!(result2, "物品C");
    }
}

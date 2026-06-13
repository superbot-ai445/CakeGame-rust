/// CakeGame 物品系统 - 商店购买、合成
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 查看系统商店
pub fn view_shop(db: &Database, user_id: &str, page: i32) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let rows: Vec<(String, String, String, String)> = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Name, Currency, Price, LimitNumber FROM Config_Shop")
            .unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    if rows.is_empty() {
        return format!("{}\n当前商店暂无商品！", prefix);
    }
    let page_size = 10;
    let total_pages = ((rows.len() as i32) + page_size - 1) / page_size;
    let page = page.min(total_pages).max(1);
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(rows.len());
    let mut result = format!("{}\n═══ 系统商店 ({}/{}) ═══", prefix, page, total_pages);
    for (i, (name, currency, price, limit)) in rows[start..end].iter().enumerate() {
        let cn = match currency.as_str() {
            "金币" | "currency_gold" => "金币",
            "钻石" | "currency_diamond" => "钻石",
            _ => currency,
        };
        let lim = if limit != "0" && !limit.is_empty() {
            format!(" (限购:{})", limit)
        } else {
            String::new()
        };
        result.push_str(&format!("\n{}. [{}] - {}{}{}", start + i + 1, name, price, cn, lim));
    }
    result.push_str("\n\n发送'购买+商品名*数量'购买");
    result
}

/// 购买商品
pub fn buy_item(db: &Database, user_id: &str, args: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let parts: Vec<&str> = args.split('*').collect();
    let item_name = parts[0].trim();
    let buy_qty: i32 = parts.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(1).max(1);
    if item_name.is_empty() {
        return format!("{}\n请输入商品名称！", prefix);
    }
    let shop_info: Option<(String, String, String)> = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT Currency, Price, LimitNumber FROM Config_Shop WHERE Name=?1")
            .unwrap();
        stmt.query_row(rusqlite::params![item_name], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .ok()
    };
    let (currency, price_str, limit_str) = match shop_info {
        Some(v) => v,
        None => return format!("{}\n不存在此商品。", prefix),
    };
    let price: i64 = price_str.parse().unwrap_or(0);
    let limit: i32 = limit_str.parse().unwrap_or(0);
    if limit > 0 {
        let bought_key = format!("Commodity.{}.Number", item_name);
        let bought: i32 = db.read_user_data(user_id, &bought_key).parse().unwrap_or(0);
        if bought + buy_qty > limit {
            return format!("{}\n商品库存不足！已购买{}/{}", prefix, bought, limit);
        }
    }
    let total_cost = price * buy_qty as i64;
    // 映射货币类型
    let currency_key = match currency.as_str() {
        "currency_gold" => "金币",
        "currency_diamond" => "钻石",
        _ => &currency,
    };
    let current = db.read_currency(user_id, currency_key);
    let cn = match currency.as_str() {
        "金币" | "currency_gold" => "金币",
        "钻石" | "currency_diamond" => "钻石",
        _ => &currency,
    };
    if current < total_cost {
        return format!("{}\n您的{}余额不足！需要{}，当前{}", prefix, cn, total_cost, current);
    }
    db.modify_currency(user_id, currency_key, OP_SUB, total_cost);
    db.knapsack_add(user_id, item_name, buy_qty);
    crate::collection::record_item_collection(db, user_id, item_name);
    // 记录购买历史
    crate::purchase_history::record_purchase(db, user_id, "NPC商店", item_name, total_cost, cn, buy_qty);
    let bought_key = format!("Commodity.{}.Number", item_name);
    let bought: i32 = db.read_user_data(user_id, &bought_key).parse().unwrap_or(0);
    db.write_user_data(user_id, &bought_key, &(bought + buy_qty).to_string());
    let remaining = db.read_currency(user_id, currency_key);
    format!(
        "{}\n成功购买{}个[{}]\n共消耗{}{}\n剩余{}：{}",
        prefix, buy_qty, item_name, total_cost, cn, cn, remaining
    )
}

/// 查看合成配方
pub fn view_composite(db: &Database, user_id: &str, item_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info: Option<(String, String, String, String, String)> = {
        let conn = db.lock_conn();
        let mut stmt = conn.prepare("SELECT Produce, ConsumeGoods, ConsumeGold, ConsumeDiamond, Success FROM Config_Composite WHERE Produce=?1").unwrap();
        stmt.query_row(rusqlite::params![item_name], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .ok()
    };
    let (produce, goods, gold_str, dia_str, rate_str) = match info {
        Some(v) => v,
        None => return format!("{}\n[{}]无法合成或不存在！", prefix, item_name),
    };
    let mut r = format!("{}\n═══ 合成配方 ═══\n产出：[{}]", prefix, produce);
    let gold: i64 = gold_str.parse().unwrap_or(0);
    if gold > 0 {
        r.push_str(&format!("\n消耗金币：{}", gold));
    }
    let dia: i64 = dia_str.parse().unwrap_or(0);
    if dia > 0 {
        r.push_str(&format!("\n消耗钻石：{}", dia));
    }
    if !goods.is_empty() {
        r.push_str("\n消耗物品：");
        for part in goods.split(',') {
            let p: Vec<&str> = part.trim().split('*').collect();
            if p.len() >= 2 {
                let n = p[0];
                let q: i32 = p[1].parse().unwrap_or(1);
                let owned = db.knapsack_quantity(user_id, n);
                let s = if owned >= q { "✓" } else { "(不足)" };
                r.push_str(&format!("\n  [{}]×{} {}/{}", n, q, owned, s));
            }
        }
    }
    let rate: i32 = rate_str.parse().unwrap_or(100);
    if rate < 100 {
        r.push_str(&format!("\n成功率：{}%", rate));
    }
    r
}

/// 合成物品
pub fn composite_item(db: &Database, user_id: &str, item_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info: Option<(String, String, String, String)> = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT ConsumeGoods, ConsumeGold, ConsumeDiamond, Success FROM Config_Composite WHERE Produce=?1")
            .unwrap();
        stmt.query_row(rusqlite::params![item_name], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .ok()
    };
    let (goods, gold_str, dia_str, rate_str) = match info {
        Some(v) => v,
        None => return format!("{}\n[{}]无法合成！", prefix, item_name),
    };
    let gold: i64 = gold_str.parse().unwrap_or(0);
    if gold > 0 && db.read_currency(user_id, CURRENCY_GOLD) < gold {
        return format!(
            "{}\n金币不足！需要{}，当前{}",
            prefix,
            gold,
            db.read_currency(user_id, CURRENCY_GOLD)
        );
    }
    let dia: i64 = dia_str.parse().unwrap_or(0);
    if dia > 0 && db.read_currency(user_id, CURRENCY_DIAMOND) < dia {
        return format!(
            "{}\n钻石不足！需要{}，当前{}",
            prefix,
            dia,
            db.read_currency(user_id, CURRENCY_DIAMOND)
        );
    }
    if !goods.is_empty() {
        for part in goods.split(',') {
            let p: Vec<&str> = part.trim().split('*').collect();
            if p.len() >= 2 {
                let n = p[0];
                let q: i32 = p[1].parse().unwrap_or(1);
                if db.knapsack_quantity(user_id, n) < q {
                    return format!(
                        "{}\n材料不足！[{}]需要{}，当前{}",
                        prefix,
                        n,
                        q,
                        db.knapsack_quantity(user_id, n)
                    );
                }
            }
        }
    }
    if gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold);
    }
    if dia > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, dia);
    }
    if !goods.is_empty() {
        for part in goods.split(',') {
            let p: Vec<&str> = part.trim().split('*').collect();
            if p.len() >= 2 {
                db.knapsack_remove(user_id, p[0], p[1].parse().unwrap_or(1));
            }
        }
    }
    let rate: i32 = rate_str.parse().unwrap_or(100);
    let roll: i32 = rand::random::<i32>() % 100 + 1;
    if roll <= rate {
        db.knapsack_add(user_id, item_name, 1);
        // 周常任务进度追踪
        crate::weekly_quest::on_composite(db, user_id);
        crate::achievement::on_compose(db, user_id);
        format!("{}\n合成成功！获得 [{}]", prefix, item_name)
    } else {
        format!("{}\n合成失败！材料已消耗。（成功率{}%，本次{}）", prefix, rate, roll)
    }
}

/// 合成列表
pub fn composite_list(db: &Database, user_id: &str, page: i32) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let items: Vec<String> = {
        let conn = db.lock_conn();
        let mut stmt = conn.prepare("SELECT DISTINCT Produce FROM Config_Composite").unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    if items.is_empty() {
        return format!("{}\n当前暂无可合成物品！", prefix);
    }
    let ps = 10;
    let tp = ((items.len() as i32) + ps - 1) / ps;
    let page = page.min(tp).max(1);
    let s = (((page) - 1) * ps) as usize;
    let e = (s + ps as usize).min(items.len());
    let mut r = format!("{}\n═══ 合成列表 ({}/{}) ═══", prefix, page, tp);
    for (i, n) in items[s..e].iter().enumerate() {
        r.push_str(&format!("\n{}. [{}]", s + i + 1, n));
    }
    r.push_str("\n\n发送'合成+物品名'合成，'查看合成+物品名'查看配方");
    r
}

/// 商品筛选 - 按类型筛选系统商店商品
pub fn filter_shop_goods(db: &Database, user_id: &str, args: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let filter_type = args.trim();

    if filter_type.is_empty() {
        return format!(
            "{}\n请指定筛选类型：\n装备 - 武器防具等\n药剂 - 药水草药等\n材料 - 合成材料等\n礼包 - 礼包宝箱等\n技能书 - 技能相关\n货币袋 - 货币类",
            prefix
        );
    }

    // Map Chinese type names to database type values
    let db_type = match filter_type {
        "装备" => "Equip",
        "药剂" => "potion",
        "材料" => "material",
        "礼包" => "GiftBag",
        "技能书" => "skillbook",
        "货币袋" => "MoneyBag",
        _ => {
            return format!(
                "{}\n未知类型 '{}'，可选：装备、药剂、材料、礼包、技能书、货币袋",
                prefix, filter_type
            );
        }
    };

    // Query shop items joined with Config_Goods to filter by type
    let rows: Vec<(String, String, String, String)> = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare(
                "SELECT s.Name, s.Currency, s.Price, s.LimitNumber \
                 FROM Config_Shop s \
                 INNER JOIN Config_Goods g ON s.Name = g.Name \
                 WHERE g.Type = ?1",
            )
            .unwrap();
        stmt.query_map(rusqlite::params![db_type], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    };

    if rows.is_empty() {
        return format!("{}\n商店中没有{}类型的商品！", prefix, filter_type);
    }

    let mut result = format!("{}\n═══ 商店筛选：{} ({}) ═══", prefix, filter_type, rows.len());
    for (i, (name, currency, price, limit)) in rows.iter().enumerate() {
        let cn = match currency.as_str() {
            "金币" | "currency_gold" => "金币",
            "钻石" | "currency_diamond" => "钻石",
            _ => currency.as_str(),
        };
        let lim = if limit != "0" && !limit.is_empty() {
            format!(" (限购:{})", limit)
        } else {
            String::new()
        };
        result.push_str(&format!("\n{}. [{}] - {}{}{}", i + 1, name, price, cn, lim));
    }
    result.push_str("\n\n发送'购买+商品名*数量'购买，'商品筛选+类型'切换筛选");
    result
}

/// 自动查看角色信息（快捷版）
#[allow(dead_code)]
pub fn auto_view_info(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);
    let current_map = db.read_user_data(user_id, "Location");

    let mut result = format!("{}\n═══ 角色概览 ═══", prefix);
    result.push_str(&format!("\n{}", info.name));
    result.push_str(&format!(" Lv.{}", info.level));
    result.push_str(&format!(" 【{}】", info.occupation));
    result.push_str(&format!(
        "\n位置：{}",
        if current_map.is_empty() {
            "新手村"
        } else {
            &current_map
        }
    ));

    result.push_str(&format!("\n\n❤ 生命：{}/{}", info.hp, info.hp_max));
    result.push_str(&format!("\n💧 魔法：{}/{}", info.mp, info.mp_max));
    result.push_str(&format!("\n⚔ 物攻：{}", info.ad));
    result.push_str(&format!("\n🔮 魔攻：{}", info.ap));
    result.push_str(&format!("\n🛡 防御：{}", info.defense));
    result.push_str(&format!("\n✨ 魔抗：{}", info.magic_res));
    result.push_str(&format!("\n💰 金币：{}", info.gold));
    result.push_str(&format!("\n💎 钻石：{}", info.diamond));
    result.push_str(&format!("\n📈 经验：{}/{}", info.exp, info.exp_need));

    result
}

#[allow(dead_code)]
/// 映射中文货币名到数据库键名
pub fn currency_to_db_key(currency: &str) -> &str {
    match currency {
        "金币" | "currency_gold" => "金币",
        "钻石" | "currency_diamond" => "钻石",
        _ => currency,
    }
}

#[allow(dead_code)]
/// 映射中文类型名到数据库类型值
pub fn type_to_db_type(type_name: &str) -> Option<&'static str> {
    match type_name {
        "装备" => Some("Equip"),
        "药剂" => Some("potion"),
        "材料" => Some("material"),
        "礼包" => Some("GiftBag"),
        "技能书" => Some("skillbook"),
        "货币袋" => Some("MoneyBag"),
        _ => None,
    }
}

#[allow(dead_code)]
/// 计算总页数
pub fn calc_total_pages(total_items: i32, page_size: i32) -> i32 {
    if total_items <= 0 || page_size <= 0 {
        return 0;
    }
    (total_items + page_size - 1) / page_size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_to_db_key_gold() {
        assert_eq!(currency_to_db_key("金币"), "金币");
        assert_eq!(currency_to_db_key("currency_gold"), "金币");
    }

    #[test]
    fn test_currency_to_db_key_diamond() {
        assert_eq!(currency_to_db_key("钻石"), "钻石");
        assert_eq!(currency_to_db_key("currency_diamond"), "钻石");
    }

    #[test]
    fn test_currency_to_db_key_unknown() {
        assert_eq!(currency_to_db_key("其他"), "其他");
    }

    #[test]
    fn test_type_to_db_type_all_types() {
        assert_eq!(type_to_db_type("装备"), Some("Equip"));
        assert_eq!(type_to_db_type("药剂"), Some("potion"));
        assert_eq!(type_to_db_type("材料"), Some("material"));
        assert_eq!(type_to_db_type("礼包"), Some("GiftBag"));
        assert_eq!(type_to_db_type("技能书"), Some("skillbook"));
        assert_eq!(type_to_db_type("货币袋"), Some("MoneyBag"));
    }

    #[test]
    fn test_type_to_db_type_unknown() {
        assert_eq!(type_to_db_type("未知"), None);
        assert_eq!(type_to_db_type(""), None);
    }

    #[test]
    fn test_calc_total_pages() {
        assert_eq!(calc_total_pages(0, 10), 0);
        assert_eq!(calc_total_pages(1, 10), 1);
        assert_eq!(calc_total_pages(10, 10), 1);
        assert_eq!(calc_total_pages(11, 10), 2);
        assert_eq!(calc_total_pages(100, 10), 10);
    }

    #[test]
    fn test_calc_total_pages_edge_cases() {
        assert_eq!(calc_total_pages(-1, 10), 0);
        assert_eq!(calc_total_pages(10, 0), 0);
    }
}

/// 获取验证信息（查看系统状态）
#[allow(dead_code)]
pub fn get_verify_info(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut result = format!("{}\n═══ 系统验证信息 ═══", prefix);

    // Count registered users
    let user_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Basic_User", [], |row| row.get(0))
            .unwrap_or(0)
    };
    result.push_str(&format!("\n注册玩家：{}", user_count));

    // Count items
    let item_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Goods", [], |row| row.get(0))
            .unwrap_or(0)
    };
    result.push_str(&format!("\n物品种类：{}", item_count));

    // Count maps
    let map_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Map", [], |row| row.get(0))
            .unwrap_or(0)
    };
    result.push_str(&format!("\n地图数量：{}", map_count));

    // Count monsters
    let monster_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Monster", [], |row| row.get(0))
            .unwrap_or(0)
    };
    result.push_str(&format!("\n怪物模型：{}", monster_count));

    // Count skills
    let skill_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Skills", [], |row| row.get(0))
            .unwrap_or(0)
    };
    result.push_str(&format!("\n技能数量：{}", skill_count));

    // Engine info
    result.push_str("\n\n─── 引擎信息 ───");
    result.push_str("\n引擎：CakeGame-RS v2.0");
    result.push_str("\n语言：Rust");
    result.push_str("\n数据库：gamedata.sdb");
    result.push_str("\n表数量：93+");

    // Shield status
    let shield_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Shield_Register", [], |row| row.get(0))
            .unwrap_or(0)
    };
    if shield_count > 0 {
        result.push_str(&format!("\n🛡 护盾记录：{}", shield_count));
    }

    result
}

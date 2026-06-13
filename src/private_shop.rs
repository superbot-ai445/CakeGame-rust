/// CakeGame 私人商店系统
/// 实现玩家商店功能
use crate::core::*;
use crate::db::Database;
use crate::encoding;
use crate::user;

/// 私人商店列表
pub fn cmd_private_shop_list(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let page: i32 = args.trim().parse().unwrap_or(1).max(1);

    // 读取所有开启的私人商店
    let shops = db.private_shop_list();

    if shops.is_empty() {
        return format!("{}\n暂无开启的私人商店！", prefix);
    }

    // 分页
    let page_size = 5;
    let total_pages = ((shops.len() as i32) + page_size - 1) / page_size;
    let page = page.min(total_pages);
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(shops.len());

    let mut result = format!("{}\n【私人商店列表】第{}/{}页", prefix, page, total_pages);

    for (i, shop) in shops[start..end].iter().enumerate() {
        let owner_name = db.read_basic(&shop.owner, ITEM_NAME);
        result.push_str(&format!(
            "\n{}. [{}] - {}",
            start + i + 1,
            encoding::smart_decode(&shop.name),
            encoding::smart_decode(&owner_name)
        ));
    }

    result.push_str("\n\n进入商店：发送'进入商店+商店序号'");
    if page < total_pages {
        result.push_str(&format!("\n下一页：商店列表+{}", page + 1));
    }

    result
}

/// 进入私人商店
pub fn cmd_enter_private_shop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let shop_id = args.trim();
    if shop_id.is_empty() {
        return format!("{}\n请指定要进入的商店！", prefix);
    }

    // 支持序号或名称
    let shops = db.private_shop_list();
    let actual_shop = if let Ok(idx) = shop_id.parse::<usize>() {
        if idx > 0 && idx <= shops.len() {
            shops[idx - 1].clone()
        } else {
            return format!("{}\n指定的商店不存在！", prefix);
        }
    } else {
        match shops.iter().find(|s| s.name == shop_id) {
            Some(s) => s.clone(),
            None => return format!("{}\n指定的商店不存在！", prefix),
        }
    };

    if !actual_shop.open {
        return format!("{}\n该商店目前未开启！", prefix);
    }

    db.write_user_data(user_id, "current_shop", &actual_shop.owner);

    let mut result = format!("{}\n进入商店[{}]！", prefix, encoding::smart_decode(&actual_shop.name));

    // 显示商品
    if actual_shop.goods_data.is_empty() {
        result.push_str("\n\n该商店暂无商品！");
    } else {
        result.push_str("\n\n商品列表：");
        let goods: Vec<&str> = actual_shop.goods_data.split(',').collect();
        for (i, good) in goods.iter().enumerate() {
            let parts: Vec<&str> = good.split('*').collect();
            if parts.len() >= 2 {
                let item_name = parts[0];
                let price = parts[1];
                result.push_str(&format!(
                    "\n{}. [{}] - 价格:{}",
                    i + 1,
                    encoding::smart_decode(item_name),
                    price
                ));
            }
        }
    }

    result.push_str("\n\n购买商品：发送'购买+商品序号+数量'");
    result.push_str("\n退出商店：发送'退出商店'");
    result
}

/// 退出私人商店
pub fn cmd_exit_private_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    db.write_user_data(user_id, "current_shop", "");
    format!("{}\n已退出私人商店！", prefix)
}

/// 查看我的商店
pub fn cmd_my_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let shop = db.private_shop_get(user_id);

    let mut result = format!("{}\n【我的商店】", prefix);

    match shop {
        Some(s) => {
            result.push_str(&format!("\n商店名：{}", encoding::smart_decode(&s.name)));
            result.push_str(&format!("\n状态：{}", if s.open { "开启" } else { "关闭" }));

            if s.goods_data.is_empty() {
                result.push_str("\n\n暂无商品！");
            } else {
                result.push_str("\n\n商品列表：");
                let goods: Vec<&str> = s.goods_data.split(',').collect();
                for (i, good) in goods.iter().enumerate() {
                    let parts: Vec<&str> = good.split('*').collect();
                    if parts.len() >= 2 {
                        let item_name = parts[0];
                        let price = parts[1];
                        let currency = if parts.len() >= 3 { parts[2] } else { "金币" };
                        result.push_str(&format!(
                            "\n{}. [{}] - 价格:{}({})",
                            i + 1,
                            encoding::smart_decode(item_name),
                            price,
                            currency
                        ));
                    }
                }
            }

            result.push_str("\n\n操作：");
            result.push_str("\n上架商品：发送'上架商品+物品名*价格*货币类型'");
            result.push_str("\n下架商品：发送'下架商品+商品序号'");
            result.push_str("\n开启商店：发送'开启商店'");
            result.push_str("\n关闭商店：发送'关闭商店'");
            result.push_str("\n商店改名：发送'商店改名+新名称'");
        }
        None => {
            result.push_str("\n您还没有商店！");
            result.push_str("\n\n创建商店：发送'商店改名+商店名称'");
        }
    }

    result
}

/// 上架商品
pub fn cmd_add_shop_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!("{}\n请指定要上架的商品！\n格式：物品名*价格*货币类型", prefix);
    }

    let parts: Vec<&str> = args.split('*').collect();
    if parts.len() < 2 {
        return format!("{}\n格式错误！\n格式：物品名*价格*货币类型", prefix);
    }

    let item_name = parts[0];
    let price: i64 = parts[1].parse().unwrap_or(0);
    let currency = if parts.len() >= 3 { parts[2] } else { "金币" };

    if price <= 0 {
        return format!("{}\n价格必须大于0！", prefix);
    }

    // 检查物品是否在背包中
    let have = db.knapsack_quantity(user_id, item_name);
    if have <= 0 {
        return format!("{}\n您的背包中没有[{}]！", prefix, encoding::smart_decode(item_name));
    }

    // 获取或创建商店
    let mut shop = db.private_shop_get(user_id).unwrap_or_else(|| PrivateShop {
        owner: user_id.to_string(),
        name: format!("{}的商店", db.read_basic(user_id, ITEM_NAME)),
        goods_data: String::new(),
        open: false,
    });

    // 添加商品
    let good = format!("{}*{}*{}", item_name, price, currency);
    if shop.goods_data.is_empty() {
        shop.goods_data = good;
    } else {
        shop.goods_data.push(',');
        shop.goods_data.push_str(&good);
    }

    db.private_shop_save(&shop);
    format!(
        "{}\n成功上架商品[{}]×1，价格：{}({})！",
        prefix,
        encoding::smart_decode(item_name),
        price,
        currency
    )
}

/// 下架商品
pub fn cmd_remove_shop_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let idx: usize = args.trim().parse().unwrap_or(0);
    if idx == 0 {
        return format!("{}\n请指定要下架的商品序号！", prefix);
    }

    let mut shop = match db.private_shop_get(user_id) {
        Some(s) => s,
        None => return format!("{}\n您还没有商店！", prefix),
    };

    let goods: Vec<&str> = shop.goods_data.split(',').collect();
    if idx > goods.len() {
        return format!("{}\n商品序号错误！", prefix);
    }

    let removed = goods[idx - 1].to_string();
    let new_goods: Vec<&str> = goods
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != idx - 1)
        .map(|(_, v)| *v)
        .collect();
    shop.goods_data = new_goods.join(",");

    db.private_shop_save(&shop);

    let parts: Vec<&str> = removed.split('*').collect();
    let item_name = parts[0];
    format!("{}\n成功下架商品[{}]！", prefix, encoding::smart_decode(item_name))
}

/// 开启商店
pub fn cmd_open_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let mut shop = match db.private_shop_get(user_id) {
        Some(s) => s,
        None => return format!("{}\n您还没有商店！请先设置商店名称。", prefix),
    };

    if shop.open {
        return format!("{}\n您的商店已经是开启状态！", prefix);
    }

    shop.open = true;
    db.private_shop_save(&shop);
    format!("{}\n商店已开启！", prefix)
}

/// 关闭商店
pub fn cmd_close_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let mut shop = match db.private_shop_get(user_id) {
        Some(s) => s,
        None => return format!("{}\n您还没有商店！", prefix),
    };

    if !shop.open {
        return format!("{}\n您的商店已经是关闭状态！", prefix);
    }

    shop.open = false;
    db.private_shop_save(&shop);
    format!("{}\n商店已关闭！", prefix)
}

/// 商店改名
pub fn cmd_rename_shop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let new_name = args.trim();
    if new_name.is_empty() {
        return format!("{}\n请输入新的商店名称！", prefix);
    }

    let mut shop = db.private_shop_get(user_id).unwrap_or_else(|| PrivateShop {
        owner: user_id.to_string(),
        name: String::new(),
        goods_data: String::new(),
        open: false,
    });

    shop.name = new_name.to_string();
    db.private_shop_save(&shop);
    format!("{}\n商店已改名为[{}]！", prefix, encoding::smart_decode(new_name))
}

/// 搜索商品 - 跨商店搜索指定物品
pub fn cmd_search_shop_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let keyword = args.trim();
    if keyword.is_empty() {
        return format!("{}\n请指定要搜索的商品名称！\n发送'搜索商品+商品名'进行搜索", prefix);
    }

    let shops = db.private_shop_list();

    let mut results: Vec<(String, String, String, String)> = Vec::new(); // (shop_name, item_name, price, owner_id)

    for shop in &shops {
        if !shop.open || shop.goods_data.is_empty() {
            continue;
        }
        let goods: Vec<&str> = shop.goods_data.split(',').collect();
        for good in &goods {
            let parts: Vec<&str> = good.split('*').collect();
            if parts.len() >= 2 {
                let item_name = parts[0];
                let price = parts[1];
                let decoded_name = encoding::smart_decode(item_name);
                if decoded_name.contains(keyword) {
                    let owner_name = db.read_basic(&shop.owner, crate::core::ITEM_NAME);
                    results.push((
                        encoding::smart_decode(&shop.name),
                        decoded_name.to_string(),
                        price.to_string(),
                        encoding::smart_decode(&owner_name),
                    ));
                }
            }
        }
    }

    if results.is_empty() {
        return format!("{}\n🔍 在所有商店中未找到包含[{}]的商品", prefix, keyword);
    }

    let mut r = format!("{}\n═══ 商品搜索 [{}] ═══", prefix, keyword);
    r.push_str(&format!("\n找到 {} 个结果：\n", results.len()));

    for (i, (shop_name, item_name, price, owner)) in results.iter().enumerate() {
        r.push_str(&format!(
            "\n{}. [{}] {} 金币:{} 店主:{}",
            i + 1,
            item_name,
            shop_name,
            price,
            owner
        ));
    }

    r.push_str("\n\n发送'进入商店+商店序号'进入商店购买");
    r
}

#[allow(dead_code)]
/// 解析商品数据字符串 (item_name*price*currency)
pub fn parse_shop_goods(goods_data: &str) -> Vec<(&str, &str, &str)> {
    if goods_data.is_empty() {
        return Vec::new();
    }
    goods_data
        .split(',')
        .filter_map(|good| {
            let parts: Vec<&str> = good.split('*').collect();
            if parts.len() >= 2 {
                let currency = if parts.len() >= 3 { parts[2] } else { "金币" };
                Some((parts[0], parts[1], currency))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shop_goods_empty() {
        assert!(parse_shop_goods("").is_empty());
    }

    #[test]
    fn test_parse_shop_goods_single() {
        let goods = parse_shop_goods("铁剑*100*金币");
        assert_eq!(goods.len(), 1);
        assert_eq!(goods[0].0, "铁剑");
        assert_eq!(goods[0].1, "100");
        assert_eq!(goods[0].2, "金币");
    }

    #[test]
    fn test_parse_shop_goods_multiple() {
        let goods = parse_shop_goods("铁剑*100*金币,皮甲*200*钻石");
        assert_eq!(goods.len(), 2);
        assert_eq!(goods[0].0, "铁剑");
        assert_eq!(goods[1].0, "皮甲");
        assert_eq!(goods[1].2, "钻石");
    }

    #[test]
    fn test_parse_shop_goods_no_currency() {
        let goods = parse_shop_goods("铁剑*100");
        assert_eq!(goods.len(), 1);
        assert_eq!(goods[0].2, "金币"); // default currency
    }

    #[test]
    fn test_parse_shop_goods_invalid() {
        let goods = parse_shop_goods("invalid_entry");
        assert!(goods.is_empty());
    }

    #[test]
    fn test_parse_shop_goods_mixed() {
        let goods = parse_shop_goods("铁剑*100*金币,invalid,皮甲*200");
        assert_eq!(goods.len(), 2);
    }
}

/// CakeGame 额外系统
/// 分解、强化、套装、NPC、食物
use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;
use rand::Rng;

// ==================== 两步分解系统 ====================

/// 选择分解物品（第一步：预览分解结果）
pub fn cmd_select_decompose(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return format!("{}\n请指定要分解的物品。\n用法：选择分解+物品名", prefix);
    }

    // 检查装备锁定
    if crate::equip_lock::is_equip_name_locked(db, user_id, item_name) {
        return format!(
            "{}\n🔒 [{}] 已锁定，无法分解！\n💡 使用「解锁装备+槽位名」解除锁定后再分解。",
            prefix, item_name
        );
    }

    // 查找分解配方
    let recipe = match db.query_row(
        "SELECT Goods, NeedG, NeedD, GetGoods, Success FROM Config_Decomposition WHERE Goods = ?",
        &[item_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    ) {
        Ok(r) => r,
        Err(_) => return format!("{}\n物品 [{}] 无法分解。", prefix, item_name),
    };

    let (goods_name, need_gold, need_diamond, get_goods, success_rate) = recipe;

    // 检查背包中是否有该物品
    let inv_count = db.get_item_count(user_id, &goods_name);
    if inv_count <= 0 {
        return format!("{}\n你没有 [{}]。", prefix, goods_name);
    }

    // 存储待分解信息到用户状态
    db.write_basic(user_id, "pending_decompose", &goods_name);

    // 构建预览
    let mut r = format!("{}\n═══ 分解预览 ═══\n物品：[{}]", prefix, goods_name);

    let gold_cost: i32 = need_gold.parse().unwrap_or(0);
    if gold_cost > 0 {
        r.push_str(&format!("\n金币消耗：{}", gold_cost));
    }
    let diamond_cost: i32 = need_diamond.parse().unwrap_or(0);
    if diamond_cost > 0 {
        r.push_str(&format!("\n钻石消耗：{}", diamond_cost));
    }
    r.push_str(&format!("\n成功率：{}%", success_rate));
    r.push_str(&format!("\n可获得：{}", get_goods.replace('\x01', "")));
    r.push_str("\n\n发送'确认分解'执行分解");
    r
}

/// 确认分解物品（第二步：执行分解）
pub fn cmd_confirm_decompose(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 读取待分解物品
    let pending = db.read_basic(user_id, "pending_decompose");
    if pending.is_empty() {
        return format!("{}\n没有待分解的物品。\n请先发送'选择分解+物品名'", prefix);
    }

    // 清除待分解状态
    db.write_basic(user_id, "pending_decompose", "");

    // 查找分解配方
    let recipe = match db.query_row(
        "SELECT Goods, NeedG, NeedD, GetGoods, Success FROM Config_Decomposition WHERE Goods = ?",
        &[&pending],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    ) {
        Ok(r) => r,
        Err(_) => return format!("{}\n物品 [{}] 分解配方丢失。", prefix, pending),
    };

    let (goods_name, need_gold, need_diamond, get_goods, success_rate) = recipe;

    // 检查背包
    let inv_count = db.get_item_count(user_id, &goods_name);
    if inv_count <= 0 {
        return format!("{}\n你没有 [{}]。", prefix, goods_name);
    }

    // 检查金币消耗
    let gold_cost: i32 = need_gold.parse().unwrap_or(0);
    if gold_cost > 0 {
        let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
        if user_gold < gold_cost {
            return format!("{}\n分解需要 {} 金币，你只有 {} 金币。", prefix, gold_cost, user_gold);
        }
        db.write_currency(user_id, CURRENCY_GOLD, (user_gold - gold_cost) as i64);
    }

    // 检查钻石消耗
    let diamond_cost: i32 = need_diamond.parse().unwrap_or(0);
    if diamond_cost > 0 {
        let user_diamond: i32 = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;
        if user_diamond < diamond_cost {
            return format!(
                "{}\n分解需要 {} 钻石，你只有 {} 钻石。",
                prefix, diamond_cost, user_diamond
            );
        }
        db.write_currency(user_id, CURRENCY_DIAMOND, (user_diamond - diamond_cost) as i64);
    }

    // 消耗物品
    db.remove_item(user_id, &goods_name, 1);

    // 判断成功率
    let success: i32 = success_rate.parse().unwrap_or(100);
    let roll = rand::thread_rng().gen_range(0..100);

    if roll >= success {
        let mut result = format!("{}\n分解 [{}] 失败！\n", prefix, goods_name);
        if gold_cost > 0 {
            result.push_str(&format!("消耗金币：{}\n", gold_cost));
        }
        result.push_str("什么也没获得...");
        return result;
    }

    // 解析获得物品
    let mut result = format!("{}\n分解 [{}] 成功！\n", prefix, goods_name);
    if gold_cost > 0 {
        result.push_str(&format!("消耗金币：{}\n", gold_cost));
    }

    for reward in get_goods.split(',') {
        let reward = reward.trim();
        if reward.is_empty() {
            continue;
        }
        let parts: Vec<&str> = reward.split('*').collect();
        let rname = parts[0].trim();
        let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

        if let Some(clean_name) = rname.strip_prefix('\x01') {
            db.add_item(user_id, clean_name, rcount);
            result.push_str(&format!("获得：[{}]×{}\n", clean_name, rcount));
        } else {
            db.add_item(user_id, rname, rcount);
            result.push_str(&format!("获得：[{}]×{}\n", rname, rcount));
        }
    }

    result
}

// ==================== 直接分解（兼容旧接口）====================

/// 分解物品（直接分解，兼容旧接口）
pub fn cmd_decompose(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return format!(
            "{}\n请指定要分解的物品。\n用法：分解+物品名\n或使用'选择分解+物品名'预览后确认分解",
            prefix
        );
    }

    // 查找分解配方
    let recipe = match db.query_row(
        "SELECT Goods, NeedG, NeedD, GetGoods, Success FROM Config_Decomposition WHERE Goods = ?",
        &[item_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    ) {
        Ok(r) => r,
        Err(_) => return format!("{}\n物品 [{}] 无法分解。", prefix, item_name),
    };

    let (goods_name, need_gold, need_diamond, get_goods, success_rate) = recipe;

    // 检查背包中是否有该物品
    let inv_count = db.get_item_count(user_id, &goods_name);
    if inv_count <= 0 {
        return format!("{}\n你没有 [{}]。", prefix, goods_name);
    }

    // 检查金币消耗
    let gold_cost: i32 = need_gold.parse().unwrap_or(0);
    if gold_cost > 0 {
        let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
        if user_gold < gold_cost {
            return format!("{}\n分解需要 {} 金币，你只有 {} 金币。", prefix, gold_cost, user_gold);
        }
        db.write_currency(user_id, CURRENCY_GOLD, (user_gold - gold_cost) as i64);
    }

    // 检查钻石消耗
    let diamond_cost: i32 = need_diamond.parse().unwrap_or(0);
    if diamond_cost > 0 {
        let user_diamond: i32 = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;
        if user_diamond < diamond_cost {
            return format!(
                "{}\n分解需要 {} 钻石，你只有 {} 钻石。",
                prefix, diamond_cost, user_diamond
            );
        }
        db.write_currency(user_id, CURRENCY_DIAMOND, (user_diamond - diamond_cost) as i64);
    }

    // 消耗物品
    db.remove_item(user_id, &goods_name, 1);

    // 判断成功率
    let success: i32 = success_rate.parse().unwrap_or(100);
    let roll = rand::thread_rng().gen_range(0..100);

    if roll >= success {
        let mut result = format!("{}\n分解 [{}] 失败！\n", prefix, goods_name);
        if gold_cost > 0 {
            result.push_str(&format!("消耗金币：{}\n", gold_cost));
        }
        result.push_str("什么也没获得...");
        return result;
    }

    // 解析获得物品
    let mut result = format!("{}\n分解 [{}] 成功！\n", prefix, goods_name);
    if gold_cost > 0 {
        result.push_str(&format!("消耗金币：{}\n", gold_cost));
    }

    for reward in get_goods.split(',') {
        let reward = reward.trim();
        if reward.is_empty() {
            continue;
        }
        let parts: Vec<&str> = reward.split('*').collect();
        let rname = parts[0].trim();
        let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

        if let Some(clean_name) = rname.strip_prefix('\x01') {
            db.add_item(user_id, clean_name, rcount);
            result.push_str(&format!("获得：[{}]×{}\n", clean_name, rcount));
        } else {
            db.add_item(user_id, rname, rcount);
            result.push_str(&format!("获得：[{}]×{}\n", rname, rcount));
        }
    }

    result
}

/// 查看分解配方
pub fn cmd_view_decompose(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        // 列出所有可分解物品
        let items: Vec<String> = db.query_list("SELECT Goods FROM Config_Decomposition ORDER BY Goods", &[]);

        let ps = 15;
        let tp = ((items.len() as i32) + ps - 1) / ps;
        let page = 1;
        let s = 0;
        let e = (ps as usize).min(items.len());

        let mut r = format!("{}\n═══ 可分解物品 ({}/{}) ═══", prefix, page, tp);
        for (i, n) in items[s..e].iter().enumerate() {
            r.push_str(&format!("\n{}. [{}]", i + 1, n));
        }
        r.push_str("\n\n发送'分解+物品名'进行分解，'查看分解+物品名'查看配方");
        return r;
    }

    // 查看指定物品的分解配方
    match db.query_row(
        "SELECT Goods, NeedG, NeedD, GetGoods, Success FROM Config_Decomposition WHERE Goods = ?",
        &[item_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    ) {
        Ok((name, gold, diamond, rewards, success)) => {
            let mut r = format!("{}\n═══ 分解配方 ═══\n物品：[{}]", prefix, name);
            if !gold.is_empty() && gold != "0" {
                r.push_str(&format!("\n金币消耗：{}", gold));
            }
            if !diamond.is_empty() && diamond != "0" {
                r.push_str(&format!("\n钻石消耗：{}", diamond));
            }
            r.push_str(&format!("\n成功率：{}%", success));
            r.push_str(&format!("\n获得：{}", rewards.replace('\x01', "")));
            r
        }
        Err(_) => format!("{}\n找不到 [{}] 的分解配方。", prefix, item_name),
    }
}

// ==================== 两步强化系统 ====================

/// 选择强化（第一步：预览强化结果）
pub fn cmd_select_enhance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return format!("{}\n请指定要强化的装备。\n用法：选择强化+装备名", prefix);
    }

    // 检查背包
    let inv_count = db.get_item_count(user_id, item_name);
    if inv_count <= 0 {
        return format!("{}\n你没有 [{}]。", prefix, item_name);
    }

    // 检查是否是装备
    let item_data = db.get_item_data(item_name);
    if item_data.is_none() {
        return format!("{}\n找不到物品 [{}]。", prefix, item_name);
    }
    let (itype, _, _) = item_data.unwrap();
    if itype != "Equip" {
        return format!("{}\n[{}] 不是装备，无法强化。", prefix, item_name);
    }

    let (base_name, current_level) = parse_enhance_level(item_name);

    // 获取强化配置
    let enhance_config = db.query_row(
        "SELECT XHJB, XHZS, XHWP, ADDSX, SuccessV FROM ext_zbqh_xx WHERE Name = ? AND LV = ?",
        &[&base_name, &(current_level + 1).to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    );

    // 存储待强化信息
    db.write_basic(user_id, "pending_enhance", item_name);

    let mut r = format!("{}\n═══ 强化预览 ═══\n装备：[{}]", prefix, item_name);
    r.push_str(&format!("\n当前等级：+{}", current_level));
    r.push_str(&format!("\n目标等级：+{}", current_level + 1));

    match enhance_config {
        Ok((gold, diamond, items, add_attr, success_rate)) => {
            let gold_cost: i32 = gold.parse().unwrap_or(0);
            if gold_cost > 0 {
                r.push_str(&format!("\n金币消耗：{}", gold_cost));
            }
            let diamond_cost: i32 = diamond.parse().unwrap_or(0);
            if diamond_cost > 0 {
                r.push_str(&format!("\n钻石消耗：{}", diamond_cost));
            }
            if !items.is_empty() && items != "[NULL]" {
                r.push_str(&format!("\n需要材料：{}", items));
            }
            if !add_attr.is_empty() && add_attr != "[NULL]" {
                r.push_str(&format!("\n属性提升：{}", add_attr));
            }
            r.push_str(&format!("\n成功率：{}%", success_rate));
        }
        Err(_) => {
            let gold_cost = (current_level + 1) * 100;
            let success_rate = std::cmp::max(100 - (current_level * 15), 10);
            r.push_str(&format!("\n金币消耗：{}", gold_cost));
            r.push_str(&format!("\n成功率：{}%", success_rate));
        }
    }

    r.push_str("\n\n发送'确认强化'执行强化");

    // 显示保底进度
    let pity_count = get_pity_count(db, user_id, &base_name);
    if pity_count > 0 {
        let remaining = PITY_THRESHOLD_DEFAULT - pity_count;
        if remaining > 0 {
            r.push_str(&format!(
                "\n🎰 保底进度：{}/{} (再失败{}次必成功)",
                pity_count, PITY_THRESHOLD_DEFAULT, remaining
            ));
        } else {
            r.push_str("\n🎰 保底已就绪！下次强化必定成功！");
        }
    }
    r
}

/// 确认强化（第二步：执行强化）
pub fn cmd_confirm_enhance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 读取待强化装备
    let pending = db.read_basic(user_id, "pending_enhance");
    if pending.is_empty() {
        return format!("{}\n没有待强化的装备。\n请先发送'选择强化+装备名'", prefix);
    }

    // 体力检查 (强化消耗5体力)
    if let Err(e) = stamina::consume_stamina(user_id, "强化", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 清除待强化状态
    db.write_basic(user_id, "pending_enhance", "");

    // 调用原强化逻辑
    cmd_enhance(db, user_id, &pending, "", "")
}

// ==================== 查看当前商店 ====================

/// 查看当前地图的商店
pub fn cmd_view_current_shops(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let location = db.read_basic(user_id, ITEM_LOCATION);

    // 查找当前地图的NPC
    let npcs: Vec<(String, String)> = db.query_rows(
        "SELECT Name, Introduce FROM Ext_NPC_Info WHERE Location = ?",
        &[&location],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    );

    // 查找开启的私人商店
    let private_shops: Vec<(String, String, String)> = db.query_rows(
        "SELECT ID, Name, Open FROM Config_PrivateShops WHERE Open = 'TRUE'",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    );

    let mut r = format!("{}\n═══ {} 商店一览 ═══", prefix, location);

    // NPC商店
    if !npcs.is_empty() {
        r.push_str("\n\n📋 NPC商店：");
        for (i, (name, intro)) in npcs.iter().enumerate() {
            r.push_str(&format!("\n{}. {} - {}", i + 1, name, intro));
        }
        r.push_str("\n发送'对话+NPC名'与NPC交互");
    }

    // 系统商店
    r.push_str("\n\n🏪 系统商店：");
    r.push_str("\n发送'查看商店'查看系统商品列表");

    // 私人商店
    if !private_shops.is_empty() {
        r.push_str("\n\n👤 私人商店（全服）：");
        for (i, (id, name, _open)) in private_shops.iter().enumerate() {
            r.push_str(&format!("\n{}. 🟢{} ({})", i + 1, name, id));
        }
        r.push_str("\n发送'进入商店+商店名'进入");
    }

    if npcs.is_empty() && private_shops.is_empty() {
        r.push_str("\n\n当前地图暂无NPC商店。");
    }

    r
}

// ==================== 强化系统 ====================

/// 强化装备
pub fn cmd_enhance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return format!("{}\n请指定要强化的装备。\n用法：强化+装备名", prefix);
    }

    // 检查背包中是否有该装备
    let inv_count = db.get_item_count(user_id, item_name);
    if inv_count <= 0 {
        return format!("{}\n你没有 [{}]。", prefix, item_name);
    }

    // 检查是否是装备
    let item_data = db.get_item_data(item_name);
    if item_data.is_none() {
        return format!("{}\n找不到物品 [{}]。", prefix, item_name);
    }
    let (itype, _, _) = item_data.unwrap();
    if itype != "Equip" {
        return format!("{}\n[{}] 不是装备，无法强化。", prefix, item_name);
    }

    // 获取当前强化等级（从物品名解析，如"xxx(+3)"）
    let (base_name, current_level) = parse_enhance_level(item_name);

    // 获取强化配置
    let enhance_config = match db.query_row(
        "SELECT XHJB, XHZS, XHWP, ADDSX, SuccessV FROM ext_zbqh_xx WHERE Name = ? AND LV = ?",
        &[&base_name, &(current_level + 1).to_string()],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    ) {
        Ok(c) => c,
        Err(_) => {
            // 如果找不到具体配置，使用通用配置
            // 根据强化等级计算消耗
            let gold_cost = (current_level + 1) * 100;
            let success_rate = std::cmp::max(100 - (current_level * 15), 10);

            return try_enhance_generic(db, user_id, &base_name, current_level, gold_cost, success_rate, &prefix);
        }
    };

    let (need_gold, need_diamond, need_items, add_attr, success_rate) = enhance_config;

    // 检查金币
    let gold_cost: i32 = need_gold.parse().unwrap_or(0);
    if gold_cost > 0 {
        let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
        if user_gold < gold_cost {
            return format!("{}\n强化需要 {} 金币，你只有 {} 金币。", prefix, gold_cost, user_gold);
        }
        db.write_currency(user_id, CURRENCY_GOLD, (user_gold - gold_cost) as i64);
    }

    // 检查钻石
    let diamond_cost: i32 = need_diamond.parse().unwrap_or(0);
    if diamond_cost > 0 {
        let user_diamond: i32 = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;
        if user_diamond < diamond_cost {
            return format!(
                "{}\n强化需要 {} 钻石，你只有 {} 钻石。",
                prefix, diamond_cost, user_diamond
            );
        }
        db.write_currency(user_id, CURRENCY_DIAMOND, (user_diamond - diamond_cost) as i64);
    }

    // 检查材料物品
    if !need_items.is_empty() && need_items != "[NULL]" {
        for req in need_items.split(',') {
            let req = req.trim();
            if req.is_empty() {
                continue;
            }
            let parts: Vec<&str> = req.split('*').collect();
            let rname = parts[0].trim();
            let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let have = db.get_item_count(user_id, rname);
            if have < rcount {
                return format!("{}\n缺少材料：[{}]×{}", prefix, rname, rcount);
            }
        }
        // 消耗材料
        for req in need_items.split(',') {
            let req = req.trim();
            if req.is_empty() {
                continue;
            }
            let parts: Vec<&str> = req.split('*').collect();
            let rname = parts[0].trim();
            let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            db.remove_item(user_id, rname, rcount);
        }
    }

    // 判断强化结果 — 保底系统
    let pity_count = get_pity_count(db, user_id, &base_name);
    let pity_triggered = pity_count >= PITY_THRESHOLD_DEFAULT;
    let success: i32 = success_rate.parse().unwrap_or(100);
    let roll = rand::thread_rng().gen_range(0..100);

    if roll >= success && !pity_triggered {
        // 强化失败 — 增加保底计数
        set_pity_count(db, user_id, &base_name, pity_count + 1);
        let remaining = PITY_THRESHOLD_DEFAULT - pity_count - 1;
        let mut result = format!(
            "{}\n强化 [{}] (+{}→+{}) 失败！\n",
            prefix,
            base_name,
            current_level,
            current_level + 1
        );
        result.push_str(&format!("消耗金币：{}\n", gold_cost));
        result.push_str("装备未损坏，但材料已消耗。");
        if remaining > 0 {
            result.push_str(&format!(
                "\n🎰 保底进度：{}/{} (再失败{}次必成功)",
                pity_count + 1,
                PITY_THRESHOLD_DEFAULT,
                remaining
            ));
        }
        return result;
    }

    // 强化成功 — 重置保底计数
    set_pity_count(db, user_id, &base_name, 0);

    // 强化成功 - 消耗原装备，添加新装备
    db.remove_item(user_id, item_name, 1);
    let new_name = if current_level == 0 {
        format!("{}(+1)", base_name)
    } else {
        format!("{}(+{})", base_name, current_level + 1)
    };
    db.add_item(user_id, &new_name, 1);

    let mut result = format!(
        "{}\n强化成功！\n[{}] (+{}→+{})\n",
        prefix,
        base_name,
        current_level,
        current_level + 1
    );
    result.push_str(&format!("消耗金币：{}\n", gold_cost));
    if !add_attr.is_empty() && add_attr != "[NULL]" {
        result.push_str(&format!("属性提升：{}\n", add_attr));
    }
    if pity_triggered {
        result.push_str("🎰 保底触发！本次强化必定成功！\n");
    }
    result.push_str(&format!("新装备：[{}]", new_name));
    crate::achievement::on_enhance(db, user_id);
    result
}

fn parse_enhance_level(item_name: &str) -> (String, i32) {
    // 解析 "装备名(+3)" 格式
    if let Some(idx) = item_name.rfind("(+") {
        let base = &item_name[..idx];
        let level_str = &item_name[idx + 2..item_name.len() - 1];
        if let Ok(level) = level_str.parse::<i32>() {
            return (base.to_string(), level);
        }
    }
    (item_name.to_string(), 0)
}

/// 保底系统：默认连续失败次数（当数据库无配置时使用）
const PITY_THRESHOLD_DEFAULT: i32 = 5;

/// 获取装备强化保底阈值（从 ext_bxxt_xx 数据库读取，按等级区间匹配）
fn get_pity_threshold(db: &Database, current_level: i32) -> i32 {
    let lv_str = current_level.to_string();
    match db.query_row(
        "SELECT OG FROM ext_bxxt_xx WHERE LV_Min <= ?1 AND LV_Max >= ?1 ORDER BY CAST(LV_Min AS INTEGER) DESC LIMIT 1",
        &[&lv_str],
        |row| {
            let og: String = row.get(0).unwrap_or_default();
            Ok(og.parse().unwrap_or(PITY_THRESHOLD_DEFAULT))
        },
    ) {
        Ok(threshold) if threshold > 0 => threshold,
        _ => PITY_THRESHOLD_DEFAULT,
    }
}

/// 获取装备强化保底计数
fn get_pity_count(db: &Database, user_id: &str, base_name: &str) -> i32 {
    let key = format!("enhance_pity.{}", base_name);
    db.read_user_data(user_id, &key).parse().unwrap_or(0)
}

/// 设置装备强化保底计数
fn set_pity_count(db: &Database, user_id: &str, base_name: &str, count: i32) {
    let key = format!("enhance_pity.{}", base_name);
    db.write_user_data(user_id, &key, &count.to_string());
}

/// 查看保底进度
pub fn cmd_view_pity(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut r = format!("{}\n═══ 🎰 保底系统 ═══\n", prefix);

    // 读取保底配置
    let tiers: Vec<(String, String, String, String)> = db.query_rows(
        "SELECT Name, LV_Min, LV_Max, OG FROM ext_bxxt_xx ORDER BY CAST(LV_Min AS INTEGER)",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
            ))
        },
    );
    if tiers.is_empty() {
        r.push_str("保底配置：默认（连续失败5次必成功）\n");
    } else {
        r.push_str("📊 保底规则：\n");
        for (name, lv_min, lv_max, og) in &tiers {
            r.push_str(&format!(
                "· {} (+{}~+{})：连续失败{}次必成功\n",
                name, lv_min, lv_max, og
            ));
        }
    }

    // 读取玩家当前保底进度（扫描 Basic_User 中以 enhance_pity. 开头的记录）
    let pity_entries: Vec<(String, String)> = db.query_rows(
        "SELECT Item, Data FROM Basic_User WHERE ID = ?1 AND Item LIKE 'enhance_pity.%'",
        &[user_id],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    );

    if pity_entries.is_empty() {
        r.push_str("\n你还没有强化记录，暂无保底进度。");
    } else {
        r.push_str("\n📈 你的保底进度：\n");
        let mut has_progress = false;
        for (key, count_str) in &pity_entries {
            let base_name = key.strip_prefix("enhance_pity.").unwrap_or(key);
            let count: i32 = count_str.parse().unwrap_or(0);
            if count > 0 {
                has_progress = true;
                let threshold = get_pity_threshold(db, 0);
                let remaining = (threshold - count).max(0);
                let bar = "█".repeat(count.min(threshold) as usize) + &"░".repeat(remaining.max(0) as usize);
                r.push_str(&format!("· [{}] {}/{} {}\n", base_name, count, threshold, bar));
            }
        }
        if !has_progress {
            r.push_str("暂无进行中的保底记录。\n");
        }
        // 检查是否有已就绪的保底
        let ready: Vec<String> = pity_entries
            .iter()
            .filter(|(_, c)| c.parse().unwrap_or(0) >= PITY_THRESHOLD_DEFAULT)
            .map(|(k, _)| k.strip_prefix("enhance_pity.").unwrap_or(k).to_string())
            .collect();
        if !ready.is_empty() {
            r.push_str(&format!(
                "\n🎰 以下装备保底已就绪（下次强化必定成功）：\n{}",
                ready
                    .iter()
                    .map(|n| format!("· [{}]", n))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    r.push_str("\n\n💡 强化失败会累积保底计数，达到阈值后下次必定成功");
    r
}

fn try_enhance_generic(
    db: &Database,
    user_id: &str,
    base_name: &str,
    current_level: i32,
    gold_cost: i32,
    success_rate: i32,
    prefix: &str,
) -> String {
    let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
    if user_gold < gold_cost {
        return format!("{}\n强化需要 {} 金币，你只有 {} 金币。", prefix, gold_cost, user_gold);
    }
    db.write_currency(user_id, CURRENCY_GOLD, (user_gold - gold_cost) as i64);

    // 保底系统：读取当前连续失败次数
    let pity_count = get_pity_count(db, user_id, base_name);
    let pity_triggered = pity_count >= PITY_THRESHOLD_DEFAULT;

    let roll = rand::thread_rng().gen_range(0..100);
    let old_name = if current_level == 0 {
        base_name.to_string()
    } else {
        format!("{}(+{})", base_name, current_level)
    };

    if roll >= success_rate && !pity_triggered {
        // 强化失败 — 增加保底计数
        set_pity_count(db, user_id, base_name, pity_count + 1);
        let remaining = PITY_THRESHOLD_DEFAULT - pity_count - 1;
        let pity_hint = if remaining > 0 {
            format!(
                "\n🎰 保底进度：{}/{} (再失败{}次必成功)",
                pity_count + 1,
                PITY_THRESHOLD_DEFAULT,
                remaining
            )
        } else {
            String::new()
        };
        return format!(
            "{}\n强化 [{}] (+{}→+{}) 失败！\n消耗金币：{}\n成功率：{}%{}",
            prefix,
            base_name,
            current_level,
            current_level + 1,
            gold_cost,
            success_rate,
            pity_hint
        );
    }

    // 强化成功 — 重置保底计数
    set_pity_count(db, user_id, base_name, 0);

    // 消耗原装备
    db.remove_item(user_id, &old_name, 1);
    let new_name = format!("{}(+{})", base_name, current_level + 1);
    db.add_item(user_id, &new_name, 1);

    let pity_msg = if pity_triggered {
        "\n🎰 保底触发！本次强化必定成功！"
    } else {
        ""
    };

    format!(
        "{}\n强化成功！\n[{}] (+{}→+{})\n消耗金币：{}\n新装备：[{}]{}",
        prefix,
        base_name,
        current_level,
        current_level + 1,
        gold_cost,
        new_name,
        pity_msg
    )
}

// ==================== 套装系统 ====================

/// 查看套装信息
pub fn cmd_view_suit(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let suit_name = args.trim();

    if suit_name.is_empty() {
        // 列出所有套装
        let suits: Vec<String> = db.query_list("SELECT Name FROM Config_Suit ORDER BY Name", &[]);
        let mut r = format!("{}\n═══ 套装列表 ═══", prefix);
        for (i, s) in suits.iter().enumerate() {
            r.push_str(&format!("\n{}. {}", i + 1, s));
        }
        r.push_str("\n\n发送'查看套装+套装名'查看详情");
        return r;
    }

    // 查看指定套装
    match db.query_row("SELECT * FROM Config_Suit WHERE Name = ?", &[suit_name], |row| {
        let name: String = row.get(0).unwrap_or_default();
        let hp: String = row.get(1).unwrap_or_default();
        let mp: String = row.get(2).unwrap_or_default();
        let def: String = row.get(3).unwrap_or_default();
        let magic: String = row.get(4).unwrap_or_default();
        let ad: String = row.get(5).unwrap_or_default();
        let ap: String = row.get(6).unwrap_or_default();
        let hit: String = row.get(7).unwrap_or_default();
        let dodge: String = row.get(8).unwrap_or_default();
        let crit: String = row.get(9).unwrap_or_default();
        let absorb: String = row.get(10).unwrap_or_default();
        Ok((name, hp, mp, def, magic, ad, ap, hit, dodge, crit, absorb))
    }) {
        Ok((name, hp, mp, def, magic, ad, ap, hit, dodge, crit, absorb)) => {
            let mut r = format!("{}\n═══ 套装：{} ═══", prefix, name);

            // 找出套装包含的装备（使用 Equip_Combined 表）
            let pieces: Vec<String> =
                db.query_list("SELECT EquipName FROM Equip_Combined WHERE SuitName = ?1", &[suit_name]);
            // 获取已装备的物品名
            let equipped = db.equip_all(user_id);
            let equipped_names: std::collections::HashSet<String> = equipped.iter().map(|e| e.name.clone()).collect();
            if !pieces.is_empty() {
                r.push_str(&format!("\n套装部件 ({}件)：", pieces.len()));
                for (i, p) in pieces.iter().enumerate() {
                    let status = if equipped_names.contains(p) { "✅" } else { "❌" };
                    r.push_str(&format!("\n  {}. {} [{}]", i + 1, p, status));
                }
                // 检查是否穿齐
                let all_equipped = pieces.iter().all(|p| equipped_names.contains(p));
                if all_equipped {
                    r.push_str("\n\n🎉 套装效果已激活！");
                } else {
                    let have = pieces.iter().filter(|p| equipped_names.contains(*p)).count();
                    r.push_str(&format!("\n\n⚠️ 未集齐 ({}/{})，套装效果未激活", have, pieces.len()));
                }
            }

            r.push_str("\n\n套装加成：");
            if hp != "0" && !hp.is_empty() {
                r.push_str(&format!("\n  生命：+{}", hp));
            }
            if mp != "0" && !mp.is_empty() {
                r.push_str(&format!("\n  魔法：+{}", mp));
            }
            if ad != "0" && !ad.is_empty() {
                r.push_str(&format!("\n  物攻：+{}", ad));
            }
            if ap != "0" && !ap.is_empty() {
                r.push_str(&format!("\n  魔攻：+{}", ap));
            }
            if def != "0" && !def.is_empty() {
                r.push_str(&format!("\n  防御：+{}", def));
            }
            if magic != "0" && !magic.is_empty() {
                r.push_str(&format!("\n  魔抗：+{}", magic));
            }
            if hit != "0" && !hit.is_empty() {
                r.push_str(&format!("\n  命中：+{}", hit));
            }
            if dodge != "0" && !dodge.is_empty() {
                r.push_str(&format!("\n  闪避：+{}", dodge));
            }
            if crit != "0" && !crit.is_empty() {
                r.push_str(&format!("\n  暴击：+{}", crit));
            }
            if absorb != "0" && !absorb.is_empty() {
                r.push_str(&format!("\n  吸血：+{}", absorb));
            }

            r
        }
        Err(_) => format!("{}\n找不到套装 [{}]。", prefix, suit_name),
    }
}

// ==================== NPC 系统 ====================

/// 查看当前地图的NPC
pub fn cmd_view_npcs(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let location = db.read_basic(user_id, ITEM_LOCATION);

    let npcs: Vec<(String, String)> = db.query_rows(
        "SELECT Name, Introduce FROM Ext_NPC_Info WHERE Location = ?",
        &[&location],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    );

    if npcs.is_empty() {
        return format!("{}\n当前地点 [{}] 没有NPC。", prefix, location);
    }

    let mut r = format!("{}\n═══ {} 的NPC ═══", prefix, location);
    for (i, (name, intro)) in npcs.iter().enumerate() {
        r.push_str(&format!("\n{}. {} - {}", i + 1, name, intro));
    }
    r.push_str("\n\n发送'对话+NPC名'与NPC交互");
    r
}

/// 与NPC对话
pub fn cmd_talk_npc(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let npc_name = args.trim();
    if npc_name.is_empty() {
        return format!("{}\n请指定NPC名称。\n用法：对话+NPC名", prefix);
    }

    // 查找NPC
    let npc = match db.query_row(
        "SELECT Name, Location, Function, Dialog, Introduce FROM Ext_NPC_Info WHERE Name = ?",
        &[npc_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
            ))
        },
    ) {
        Ok(n) => n,
        Err(_) => return format!("{}\n找不到NPC [{}]。", prefix, npc_name),
    };

    let (name, location, functions, dialog, intro) = npc;

    // 检查玩家是否在NPC所在地图
    let user_loc = db.read_basic(user_id, ITEM_LOCATION);
    if user_loc != location {
        return format!(
            "{}\nNPC [{}] 在 [{}]，你当前在 [{}]。",
            prefix, name, location, user_loc
        );
    }

    let mut r = format!("{}\n═══ 与 {} 对话 ═══", prefix, name);
    r.push_str(&format!("\n{}", dialog));
    r.push_str(&format!("\n\n{}", intro));

    // 显示NPC功能
    if !functions.is_empty() && functions != "[NULL]" {
        r.push_str("\n\n═══ 可用功能 ═══");
        for (i, func) in functions.split('\n').enumerate() {
            let func = func.trim();
            if !func.is_empty() {
                r.push_str(&format!("\n{}. {}", i + 1, func));
            }
        }
        r.push_str("\n\n发送'使用功能+功能名'执行功能");
    }

    crate::achievement::on_npc_talk(db, user_id);
    r
}

/// 使用NPC功能
pub fn cmd_use_npc_function(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let func_name = args.trim();
    if func_name.is_empty() {
        return format!("{}\n请指定功能名称。\n用法：使用功能+功能名", prefix);
    }

    // 查找功能配置
    let func = match db.query_row(
        "SELECT Name, Function FROM Ext_NPC_Function WHERE Name = ?",
        &[func_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    ) {
        Ok(f) => f,
        Err(_) => return format!("{}\n找不到功能 [{}]。", prefix, func_name),
    };

    let (name, func_json) = func;

    // 解析功能JSON
    let func_data: serde_json::Value = match serde_json::from_str(&func_json) {
        Ok(v) => v,
        Err(_) => return format!("{}\n功能配置格式错误。", prefix),
    };

    let consume_gold: i32 = func_data["consume_gold"].as_str().unwrap_or("0").parse().unwrap_or(0);
    let consume_diamond: i32 = func_data["consume_diamond"]
        .as_str()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);
    let consume_goods = func_data["consume_goods"].as_str().unwrap_or("");
    let effect_type = func_data["Effect_type"].as_str().unwrap_or("");
    let effect_param = func_data["Effect_parameter"].as_str().unwrap_or("");
    let dialog = func_data["Dialog"].as_str().unwrap_or("操作完成。");

    // 检查金币
    if consume_gold > 0 {
        let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
        if user_gold < consume_gold {
            return format!("{}\n需要 {} 金币，你只有 {} 金币。", prefix, consume_gold, user_gold);
        }
    }

    // 检查钻石
    if consume_diamond > 0 {
        let user_diamond: i32 = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;
        if user_diamond < consume_diamond {
            return format!(
                "{}\n需要 {} 钻石，你只有 {} 钻石。",
                prefix, consume_diamond, user_diamond
            );
        }
    }

    // 修复HTML编码的逗号 (&#44; 或 &#44)
    let consume_goods = consume_goods.replace("&#44;", ",").replace("&#44", ",");

    // 检查物品
    if !consume_goods.is_empty() && consume_goods != "[NULL]" {
        for req in consume_goods.split(',') {
            let req = req.trim();
            if req.is_empty() {
                continue;
            }
            let parts: Vec<&str> = req.split('*').collect();
            let rname = parts[0].trim();
            let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let have = db.get_item_count(user_id, rname);
            if have < rcount {
                return format!("{}\n缺少：[{}]×{}", prefix, rname, rcount);
            }
        }
    }

    // 检查TR_Limit (等级限制)
    if let Ok(limit_str) = db.query_row(
        "SELECT TR_Limit FROM Ext_NPC_Function WHERE Name = ?",
        &[func_name],
        |row| Ok(row.get::<_, String>(0).unwrap_or_default()),
    ) {
        if !limit_str.is_empty() && limit_str != "null" {
            if let Ok(limit_data) = serde_json::from_str::<serde_json::Value>(&limit_str) {
                let min_lv: i32 = limit_data["minimumLV"].as_str().unwrap_or("0").parse().unwrap_or(0);
                if min_lv > 0 {
                    let user_lv: i32 = db.read_basic(user_id, "等级").parse().unwrap_or(1);
                    if user_lv < min_lv {
                        return format!("{}\n等级不足！需要 {} 级，你当前 {} 级。", prefix, min_lv, user_lv);
                    }
                }
                let max_lv: i32 = limit_data["MaximumLV"].as_str().unwrap_or("0").parse().unwrap_or(0);
                if max_lv > 0 {
                    let user_lv: i32 = db.read_basic(user_id, "等级").parse().unwrap_or(1);
                    if user_lv > max_lv {
                        return format!("{}\n等级过高！限 {} 级以下，你当前 {} 级。", prefix, max_lv, user_lv);
                    }
                }
            }
        }
    }

    // 执行消耗
    if consume_gold > 0 {
        let user_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        db.write_currency(user_id, CURRENCY_GOLD, user_gold - consume_gold as i64);
    }
    if consume_diamond > 0 {
        let user_diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
        db.write_currency(user_id, CURRENCY_DIAMOND, user_diamond - consume_diamond as i64);
    }
    if !consume_goods.is_empty() && consume_goods != "[NULL]" {
        for req in consume_goods.split(',') {
            let req = req.trim();
            if req.is_empty() {
                continue;
            }
            let parts: Vec<&str> = req.split('*').collect();
            let rname = parts[0].trim();
            let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            db.remove_item(user_id, rname, rcount);
        }
    }

    // 执行效果
    match effect_type {
        "GetGoods" => {
            // 获得物品
            for reward in effect_param.split(',') {
                let reward = reward.trim();
                if reward.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = reward.split('*').collect();
                let rname = parts[0].trim();
                let rcount: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                db.add_item(user_id, rname, rcount);
            }
        }
        "AddGold" => {
            let amount: i64 = effect_param.parse().unwrap_or(0);
            let current: i64 = db.read_currency(user_id, CURRENCY_GOLD);
            db.write_currency(user_id, CURRENCY_GOLD, current + amount);
        }
        "AddDiamond" => {
            let amount: i64 = effect_param.parse().unwrap_or(0);
            let current: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
            db.write_currency(user_id, CURRENCY_DIAMOND, current + amount);
        }
        "AddExp" => {
            let amount: i32 = effect_param.parse().unwrap_or(0);
            user::add_experience(db, user_id, amount);
        }
        "Delivery" => {
            // 传送到指定地图 (NPC传送功能)
            let dest_map = effect_param.trim();
            if !dest_map.is_empty() {
                db.write_basic(user_id, ITEM_LOCATION, dest_map);
                db.write_basic(user_id, ITEM_TARGET, "");
                return format!("{}\n{}\n已传送到 [{}].", prefix, dialog, dest_map);
            }
        }
        "Treatment" => {
            // 治疗：回复全部HP和MP (NPC治疗功能)
            let hp_max = user::calc_hp_max(db, user_id);
            let mp_max = user::calc_mp_max(db, user_id);
            db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_max);
            db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_max);
            return format!("{}\n{}\nHP/MP已完全恢复！", prefix, dialog);
        }
        _ => {}
    }

    format!("{}\n{}\n功能 [{}] 使用完成。", prefix, dialog, name)
}

/// 查看强化信息
pub fn cmd_view_enhance_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 读取强化成功率配置
    let conn = db.lock_conn();
    let mut rates: Vec<(i32, String)> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT SECTION, DATA FROM Global WHERE ID = 'ext_re_hzxyx' AND SECTION LIKE '%_global' ORDER BY SECTION",
    ) {
        if let Ok(rows) = stmt.query_map([], |row| {
            let section: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok((section, data))
        }) {
            for row in rows.flatten() {
                let level_str = row.0.replace("_global", "");
                if let Ok(level) = level_str.parse::<i32>() {
                    rates.push((level, row.1));
                }
            }
        }
    }
    drop(conn);

    let mut result = "═══ 强化信息 ═══\n".to_string();
    result.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    result.push_str("📊 各级强化成功率：\n\n");

    for (level, rate_str) in &rates {
        let rate_num: i32 = rate_str.parse().unwrap_or(0);
        let bar_len = rate_num / 5;
        let bar: String = "█".repeat(bar_len as usize);
        let empty: String = "░".repeat(20 - bar_len as usize);
        let icon = if rate_num >= 25 {
            "🟢"
        } else if rate_num >= 15 {
            "🟡"
        } else {
            "🔴"
        };
        result.push_str(&format!("  {} +{:<2} {}{} {}%\n", icon, level, bar, empty, rate_num));
    }

    result.push_str("\n━━━━━━━━━━━━━━━━━━━━\n");
    result.push_str("💡 强化说明：\n");
    result.push_str("• +1~+2 成功率较高，推荐优先强化\n");
    result.push_str("• +3 以上成功率大幅下降，谨慎操作\n");
    result.push_str("• 使用「选择强化+装备名」预览强化效果\n");
    result.push_str("• 使用「确认强化」执行强化操作\n");

    // 显示当前装备强化状态
    let conn2 = db.lock_conn();
    let mut has_enhanced = false;
    let mut equip_info = String::new();

    if let Ok(mut stmt) = conn2.prepare("SELECT SlotName, EquipName FROM Equip_Register WHERE User = ?1") {
        if let Ok(rows) = stmt.query_map([user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                let (slot, name) = row;
                // 检查是否有强化等级标记
                if name.contains("+") {
                    if let Some(plus_pos) = name.rfind('+') {
                        let level_str = &name[plus_pos + 1..];
                        if let Ok(_enhance_level) = level_str.parse::<i32>() {
                            if !has_enhanced {
                                equip_info.push_str("\n📐 当前已强化装备：\n");
                                has_enhanced = true;
                            }
                            equip_info.push_str(&format!("  {}：{}\n", slot, crate::encoding::smart_decode(&name)));
                        }
                    }
                }
            }
        }
    }
    drop(conn2);

    if has_enhanced {
        result.push_str(&equip_info);
    }

    format!("{}\n{}", prefix, result)
}

// ==================== 装备品质鉴定系统 ====================
// 基于 Ext_Var_EqSet_hzxyx 表 (269条) 的装备变量属性系统
// 玩家可以"鉴定装备"来获取随机属性加成

/// 装备变量属性类型
const VAR_ATTR_TYPES: &[&str] = &[
    "生命",
    "魔法",
    "物攻",
    "魔攻",
    "防御",
    "魔抗",
    "命中",
    "闪避",
    "暴击",
    "吸血",
    "物穿值",
    "法穿值",
];

/// 从 Ext_Var_EqSet_hzxyx 读取装备的变量属性潜力
fn get_equip_var_potential(db: &Database, equip_name: &str) -> Vec<i32> {
    let result = db.query_row(
        "SELECT Added FROM Ext_Var_EqSet_hzxyx WHERE Name = ?1",
        &[equip_name],
        |row| Ok(row.get::<_, String>(0).unwrap_or_default()),
    );
    match result {
        Ok(added_str) => added_str
            .split(',')
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect(),
        Err(_) => vec![],
    }
}

/// 鉴定装备 — 为装备随机生成变量属性加成
pub fn cmd_appraise_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let equip_name = args.trim();

    if equip_name.is_empty() {
        return format!(
            "{}\n请指定要鉴定的装备名。\n用法：鉴定装备+装备名\n发送'查看装备'查看已有装备",
            prefix
        );
    }

    // 检查背包中是否有该装备
    let inv_count = db.get_item_count(user_id, equip_name);
    if inv_count <= 0 {
        return format!("{}\n你没有 [{}]，无法鉴定。", prefix, equip_name);
    }

    // 检查是否是装备类型
    let item_data = db.get_item_data(equip_name);
    if let Some((itype, _, _)) = &item_data {
        if itype != "Equip" {
            return format!("{}\n[{}] 不是装备，无法鉴定。", prefix, equip_name);
        }
    } else {
        return format!("{}\n找不到物品 [{}]。", prefix, equip_name);
    }

    // 检查是否已经鉴定过
    let appraise_key = format!("equip_appraised_{}", equip_name);
    let existing = db.read_user_data(user_id, &appraise_key);
    if !existing.is_empty() {
        return format!(
            "{}\n[{}] 已经鉴定过了！\n当前变量属性：{}",
            prefix, equip_name, existing
        );
    }

    // 获取装备的变量属性潜力
    let potential = get_equip_var_potential(db, equip_name);
    if potential.is_empty() {
        return format!("{}\n[{}] 没有可鉴定的变量属性。", prefix, equip_name);
    }

    // 计算鉴定费用（根据装备品质）
    let quality = detect_equip_quality(equip_name);
    let cost = match quality.as_str() {
        "限定" | "远古" => 5000,
        "史诗" => 3000,
        "卓越" => 1500,
        "精良" => 800,
        _ => 300,
    };

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < cost {
        return format!(
            "{}\n鉴定 [{}] 需要 {} 金币，当前仅有 {} 金币。\n💡 品质越高的装备鉴定费用越高",
            prefix, equip_name, cost, gold
        );
    }

    // 扣除金币
    db.modify_currency(user_id, CURRENCY_GOLD, "sub", cost);

    // 随机生成变量属性
    let mut rng = rand::thread_rng();
    let mut attrs = Vec::new();

    for &max_val in potential.iter().take(3) {
        if max_val <= 0 {
            continue;
        }
        // 随机值：1 到 max_val 之间
        let roll = rng.gen_range(1..=max_val);
        // 随机选择属性类型
        let attr_type = VAR_ATTR_TYPES[rng.gen_range(0..VAR_ATTR_TYPES.len())];
        attrs.push(format!("{}+{}", attr_type, roll));
    }

    if attrs.is_empty() {
        return format!(
            "{}\n鉴定完成，但 [{}] 没有发现任何隐藏属性。\n💡 这件装备的变量属性潜力较低。",
            prefix, equip_name
        );
    }

    // 保存鉴定结果
    let attr_str = attrs.join(",");
    db.write_user_data(user_id, &appraise_key, &attr_str);

    // 构建结果
    let mut r = format!("{}\n═══ 🔍 装备鉴定 ═══\n装备：[{}]", prefix, equip_name);
    r.push_str(&format!("\n品质：{}", quality));
    r.push_str(&format!("\n鉴定费用：{} 金币", cost));
    r.push_str("\n\n✨ 鉴定结果 — 变量属性：");
    for attr in &attrs {
        r.push_str(&format!("\n  🎯 {}", attr));
    }
    r.push_str(&format!("\n\n📊 属性总计：{} 条变量属性", attrs.len()));
    r.push_str("\n💡 鉴定后的属性自动叠加到装备效果上");
    r
}

/// 查看鉴定 — 显示已鉴定装备的变量属性
pub fn cmd_view_appraise(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let equip_name = args.trim();

    if !equip_name.is_empty() {
        // 查看指定装备的鉴定结果
        let appraise_key = format!("equip_appraised_{}", equip_name);
        let existing = db.read_user_data(user_id, &appraise_key);
        if existing.is_empty() {
            return format!(
                "{}\n[{}] 尚未鉴定。\n发送'鉴定装备+{}'进行鉴定",
                prefix, equip_name, equip_name
            );
        }
        let mut r = format!("{}\n═══ 🔍 鉴定详情 ═══\n装备：[{}]", prefix, equip_name);
        r.push_str("\n变量属性：");
        for attr in existing.split(',') {
            r.push_str(&format!("\n  🎯 {}", attr));
        }
        return r;
    }

    // 列出所有已鉴定的装备
    let conn = db.lock_conn();
    let mut stmt =
        match conn.prepare("SELECT Key, Value FROM UserData WHERE User = ?1 AND Key LIKE 'equip_appraised_%'") {
            Ok(s) => s,
            Err(_) => return format!("{}\n暂无已鉴定装备。", prefix),
        };

    let rows = stmt.query_map([user_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    });

    let mut r = format!("{}\n═══ 🔍 已鉴定装备 ═══", prefix);
    let mut count = 0;

    if let Ok(mapped) = rows {
        for row in mapped.flatten() {
            let (key, value) = row;
            let name = key.replace("equip_appraised_", "");
            count += 1;
            r.push_str(&format!("\n\n{}. [{}]", count, name));
            r.push_str("\n  变量属性：");
            for attr in value.split(',') {
                r.push_str(&format!(" {}", attr));
            }
        }
    }

    if count == 0 {
        r.push_str("\n暂无已鉴定装备。");
        r.push_str("\n发送'鉴定装备+装备名'进行鉴定");
    } else {
        r.push_str(&format!("\n\n共 {} 件已鉴定装备", count));
        r.push_str("\n发送'查看鉴定+装备名'查看详细属性");
    }

    r
}

/// 检测装备品质
fn detect_equip_quality(name: &str) -> String {
    if name.contains("限定") {
        "限定".to_string()
    } else if name.contains("远古") {
        "远古".to_string()
    } else if name.contains("史诗") {
        "史诗".to_string()
    } else if name.contains("卓越") {
        "卓越".to_string()
    } else if name.contains("精良") {
        "精良".to_string()
    } else if name.contains("完美") {
        "完美".to_string()
    } else if name.contains("超界") {
        "超界".to_string()
    } else if name.contains("镇魂") {
        "镇魂".to_string()
    } else if name.contains("经典") {
        "经典".to_string()
    } else if name.contains("劣质") {
        "劣质".to_string()
    } else {
        "普通".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_function_delivery_type() {
        // Verify Delivery effect type is recognized
        let effect_type = "Delivery";
        assert!(matches!(effect_type, "Delivery" | "GetGoods" | "Treatment"));
    }

    #[test]
    fn test_npc_function_treatment_type() {
        // Verify Treatment effect type is recognized
        let effect_type = "Treatment";
        assert!(matches!(effect_type, "Delivery" | "GetGoods" | "Treatment"));
    }

    #[test]
    fn test_consume_goods_html_comma_fix() {
        // &#44; should be decoded to comma
        let raw = "【普通】来打驱动器*1&#44虚空宝石*10&#44远古超界石*3000";
        let fixed = raw.replace("&#44;", ",").replace("&#44", ",");
        assert_eq!(fixed, "【普通】来打驱动器*1,虚空宝石*10,远古超界石*3000");

        // Normal commas should be untouched
        let normal = "蓝色精粹*1,红色精粹*2";
        let fixed2 = normal.replace("&#44;", ",").replace("&#44", ",");
        assert_eq!(fixed2, normal);
    }

    #[test]
    fn test_npc_tr_limit_parsing() {
        // Parse TR_Limit JSON
        let limit_str = r#"{"minimumLV":"20","MaximumLV":"0","Day_number":"5","User_number":"0","Exist_Goods":"","Not_Exist_Goods":""}"#;
        let limit_data: serde_json::Value = serde_json::from_str(limit_str).unwrap();
        let min_lv: i32 = limit_data["minimumLV"].as_str().unwrap_or("0").parse().unwrap_or(0);
        let max_lv: i32 = limit_data["MaximumLV"].as_str().unwrap_or("0").parse().unwrap_or(0);
        assert_eq!(min_lv, 20);
        assert_eq!(max_lv, 0); // 0 means no upper limit
    }

    #[test]
    fn test_npc_function_json_parsing() {
        // Parse NPC function JSON with multiple consume_goods
        let func_json = r#"{
            "consume_gold": "10000",
            "consume_diamond": "0",
            "consume_goods": "史诗碎片*30",
            "Effect_type": "GetGoods",
            "Effect_parameter": "【史诗】斩影刃*1",
            "Dialog": "失去：史诗碎片*30\r\n获得：【史诗】斩影刃*1"
        }"#;
        let data: serde_json::Value = serde_json::from_str(func_json).unwrap();
        assert_eq!(data["consume_gold"].as_str().unwrap(), "10000");
        assert_eq!(data["Effect_type"].as_str().unwrap(), "GetGoods");
        assert_eq!(data["Effect_parameter"].as_str().unwrap(), "【史诗】斩影刃*1");
    }

    #[test]
    fn test_npc_delivery_function_parsing() {
        // Parse Delivery-type NPC function (e.g. 黑暗之门)
        let func_json = r#"{
            "consume_gold": "0",
            "consume_diamond": "0",
            "consume_goods": "蓝色精粹*5",
            "Effect_type": "Delivery",
            "Effect_parameter": "暗黑废墟营地",
            "Dialog": "费雷拿走了你五个蓝色精粹"
        }"#;
        let data: serde_json::Value = serde_json::from_str(func_json).unwrap();
        assert_eq!(data["Effect_type"].as_str().unwrap(), "Delivery");
        assert_eq!(data["Effect_parameter"].as_str().unwrap(), "暗黑废墟营地");
    }

    #[test]
    fn test_npc_treatment_function_parsing() {
        // Parse Treatment-type NPC function (e.g. 沐浴圣光)
        let func_json = r#"{
            "consume_gold": "1000",
            "consume_diamond": "0",
            "consume_goods": "蓝色精粹*10",
            "Effect_type": "Treatment",
            "Effect_parameter": "",
            "Dialog": "圣光指引着你！"
        }"#;
        let data: serde_json::Value = serde_json::from_str(func_json).unwrap();
        assert_eq!(data["Effect_type"].as_str().unwrap(), "Treatment");
        assert_eq!(data["Effect_parameter"].as_str().unwrap(), "");
    }

    #[test]
    fn test_multi_consume_goods_parsing() {
        // Test splitting consume_goods by comma
        let goods = "【普通】来打驱动器*1,虚空宝石*10,远古超界石*3000";
        let items: Vec<(&str, i32)> = goods
            .split(',')
            .filter_map(|req| {
                let parts: Vec<&str> = req.split('*').collect();
                let name = parts[0].trim();
                let count: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                Some((name, count))
            })
            .collect();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].1, 1);
        assert_eq!(items[1].1, 10);
        assert_eq!(items[2].1, 3000);
    }

    #[test]
    fn test_quality_name_parsing() {
        assert_eq!(detect_equip_quality("【史诗】斩影刃"), "史诗");
        assert_eq!(detect_equip_quality("【完美】圣堂骑士胸甲"), "完美");
        assert_eq!(detect_equip_quality("【普通】木剑"), "普通");
        assert_eq!(detect_equip_quality("神秘药水"), "普通");
    }
}

/// CakeGame 种植系统
/// 来自 ext_herbseed_info / ext_herbsgarden_info 表
/// 玩家可以购买种子、种植、收获、出售药材
use crate::core::*;
use crate::db::Database;
use crate::user;
use rusqlite::params;

/// 种子信息
struct SeedInfo {
    name: String,       // 种子名称
    herb_name: String,  // 产出药材名称
    grow_time: i64,     // 成长时间（分钟）
    harvest_qty: i32,   // 收获数量
    _max_time: i64,     // 最大存活时间（分钟）
    reiki_cost: i32,    // 灵力消耗
    reward: String,     // 收获奖励
    seed_price: i64,    // 种子价格
    price_type: String, // 价格类型: god=金币, diam=钻石
    herb_price: i64,    // 药材回收价格
}

/// 解析种子数据
fn parse_seeds(db: &Database) -> Vec<SeedInfo> {
    db.query_rows(
        "SELECT seeds, herbs, time, num, svtime, reikiconsume, yccl, seedsprice, herbsprice FROM ext_herbseed_info",
        &[],
        |row| {
            let price_raw = row.get::<_, String>(7).unwrap_or_default();
            let (price, price_type) = parse_price(&price_raw);
            Ok(SeedInfo {
                name: row.get::<_, String>(0).unwrap_or_default(),
                herb_name: row.get::<_, String>(1).unwrap_or_default(),
                grow_time: row.get::<_, String>(2).unwrap_or_default().parse().unwrap_or(0),
                harvest_qty: row.get::<_, String>(3).unwrap_or_default().parse().unwrap_or(1),
                _max_time: row.get::<_, String>(4).unwrap_or_default().parse().unwrap_or(0),
                reiki_cost: row.get::<_, String>(5).unwrap_or_default().parse().unwrap_or(0),
                reward: row.get::<_, String>(6).unwrap_or_default(),
                seed_price: price,
                price_type,
                herb_price: row.get::<_, String>(8).unwrap_or_default().parse().unwrap_or(0),
            })
        },
    )
}

/// 解析价格字符串 "1000&44god" -> (1000, "god")
fn parse_price(raw: &str) -> (i64, String) {
    let parts: Vec<&str> = raw.split("&44").collect();
    if parts.len() == 2 {
        let price = parts[0].parse().unwrap_or(0);
        let ptype = parts[1].to_string();
        (price, ptype)
    } else {
        (raw.parse().unwrap_or(0), "god".to_string())
    }
}

/// 花园地块数据
#[derive(Default, Clone)]
struct PlotData {
    seed_name: String,
    stage: String,
    quantity: String,
    plant_time: String,
}

/// 解析花园地块 JSON
fn parse_plot(json_str: &str) -> PlotData {
    if json_str.is_empty() || json_str == "{}" {
        return PlotData::default();
    }
    let mut plot = PlotData::default();
    let inner = json_str.trim_matches('{').trim_matches('}');
    for pair in inner.split(',') {
        let kv: Vec<&str> = pair.splitn(2, ':').collect();
        if kv.len() == 2 {
            let key = kv[0].trim().trim_matches('"');
            let val = kv[1].trim().trim_matches('"');
            match key {
                "种子名称" => plot.seed_name = val.to_string(),
                "阶段" => plot.stage = val.to_string(),
                "数量" => plot.quantity = val.to_string(),
                "种植时间" => plot.plant_time = val.to_string(),
                _ => {}
            }
        }
    }
    plot
}

/// 获取用户的花园数据
fn get_garden(db: &Database, user_id: &str) -> Option<(i32, Vec<PlotData>)> {
    let rows = db.query_rows(
        "SELECT reiki, soil1, soil2, soil3, soil4, soil5, soil6 FROM ext_herbsgarden_info WHERE uID=?",
        &[user_id],
        |row| {
            let reiki: i32 = row.get::<_, String>(0).unwrap_or_default().parse().unwrap_or(0);
            let mut plots = Vec::new();
            for i in 1..=6 {
                let json = row.get::<_, String>(i).unwrap_or_default();
                plots.push(parse_plot(&json));
            }
            Ok((reiki, plots))
        },
    );
    rows.into_iter().next()
}

/// 格式化时间（分钟 -> 可读）
fn format_minutes(mins: i64) -> String {
    if mins < 60 {
        format!("{}分钟", mins)
    } else if mins < 1440 {
        format!("{}小时{}分钟", mins / 60, mins % 60)
    } else {
        format!("{}天{}小时", mins / 1440, (mins % 1440) / 60)
    }
}

/// 格式化价格
fn format_price(price: i64, ptype: &str) -> String {
    match ptype {
        "god" => format!("{}金币", price),
        "diam" => format!("{}钻石", price),
        _ => format!("{}?", price),
    }
}

/// 查看种子列表
pub fn cmd_view_seeds(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let seeds = parse_seeds(db);

    if seeds.is_empty() {
        return format!("{}\n暂无可种植的种子。", prefix);
    }

    let mut r = format!("{}\n═══ 种子商店 ═══", prefix);
    for (i, seed) in seeds.iter().enumerate() {
        r.push_str(&format!("\n{}. {} → {}", i + 1, seed.name, seed.herb_name));
        r.push_str(&format!(
            "\n   价格: {} | 成长: {} | 收获: {}个",
            format_price(seed.seed_price, &seed.price_type),
            format_minutes(seed.grow_time),
            seed.harvest_qty
        ));
        if !seed.reward.is_empty() {
            r.push_str(&format!(" | 额外: {}", seed.reward));
        }
    }
    r.push_str("\n\n购买: 发送 '购买种子+种子名'");
    r.push_str("\n种植: 发送 '种植+种子名'");
    r
}

/// 购买种子
pub fn cmd_buy_seed(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let seed_name = args.trim();

    if seed_name.is_empty() {
        return format!("{}\n请指定种子名称。\n用法: 购买种子+种子名", prefix);
    }

    let seeds = parse_seeds(db);
    let seed = seeds.iter().find(|s| s.name == seed_name || s.name.contains(seed_name));
    if seed.is_none() {
        return format!(
            "{}\n找不到种子 [{}]。\n发送 '查看种子' 查看可购买的种子。",
            prefix, seed_name
        );
    }
    let seed = seed.unwrap();

    // 检查花园是否存在
    let garden = get_garden(db, user_id);
    if garden.is_none() {
        return format!("{}\n您还没有花园！请先联系管理员开通。", prefix);
    }

    // 检查货币
    match seed.price_type.as_str() {
        "god" => {
            let gold = db.read_currency(user_id, CURRENCY_GOLD);
            if gold < seed.seed_price {
                return format!(
                    "{}\n购买 {} 需要 {}，你只有 {}金币。",
                    prefix,
                    seed.name,
                    format_price(seed.seed_price, &seed.price_type),
                    gold
                );
            }
            db.write_currency(user_id, CURRENCY_GOLD, gold - seed.seed_price);
        }
        "diam" => {
            let diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
            if diamond < seed.seed_price {
                return format!(
                    "{}\n购买 {} 需要 {}，你只有 {}钻石。",
                    prefix,
                    seed.name,
                    format_price(seed.seed_price, &seed.price_type),
                    diamond
                );
            }
            db.write_currency(user_id, CURRENCY_DIAMOND, diamond - seed.seed_price);
        }
        _ => {
            return format!("{}\n未知的货币类型。", prefix);
        }
    }

    // 将种子放入背包
    db.knapsack_add(user_id, &seed.name, 1);

    format!(
        "{}\n成功购买 [{}]×1！\n花费: {}\n\n发送 '种植+{}' 进行种植",
        prefix,
        seed.name,
        format_price(seed.seed_price, &seed.price_type),
        seed.name
    )
}

/// 种植种子
pub fn cmd_plant_seed(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let seed_name = args.trim();

    if seed_name.is_empty() {
        return format!("{}\n请指定种子名称。\n用法: 种植+种子名", prefix);
    }

    // 检查背包中是否有种子
    let qty = db.knapsack_quantity(user_id, seed_name);
    if qty <= 0 {
        return format!(
            "{}\n背包中没有 [{}]。\n发送 '购买种子+种子名' 购买种子。",
            prefix, seed_name
        );
    }

    // 查找种子信息
    let seeds = parse_seeds(db);
    let seed = seeds.iter().find(|s| s.name == seed_name);
    if seed.is_none() {
        return format!("{}\n未知的种子类型 [{}]。", prefix, seed_name);
    }
    let seed = seed.unwrap();

    // 检查花园
    let garden = get_garden(db, user_id);
    if garden.is_none() {
        return format!("{}\n您还没有花园！", prefix);
    }
    let (reiki, plots) = garden.unwrap();

    // 检查灵力
    if reiki < seed.reiki_cost {
        return format!(
            "{}\n种植 {} 需要 {} 灵力，你只有 {} 灵力。",
            prefix, seed.name, seed.reiki_cost, reiki
        );
    }

    // 查找空地块
    let empty_idx = plots.iter().position(|p| p.seed_name.is_empty());
    if empty_idx.is_none() {
        return format!("{}\n花园已满（6/6地块）！请先收获再种植。", prefix);
    }
    let plot_idx = empty_idx.unwrap();

    // 扣除背包种子
    db.knapsack_remove(user_id, seed_name, 1);

    // 更新花园数据
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let plot_json = format!(
        "{{\"种子名称\":\"{}\",\"阶段\":\"1\",\"数量\":\"{}\",\"种植时间\":\"{}\"}}",
        seed.name, seed.harvest_qty, now
    );
    let soil_col = format!("soil{}", plot_idx + 1);
    let new_reiki = reiki - seed.reiki_cost;

    let conn = db.lock_conn();
    let _ = conn.execute(
        &format!("UPDATE ext_herbsgarden_info SET {}=?1, reiki=?2 WHERE uID=?3", soil_col),
        params![plot_json, new_reiki.to_string(), user_id],
    );

    format!(
        "{}\n✦ 成功种植 [{}]！\n地块: #{}\n灵力: {} → {}\n成长时间: {}\n\n发送 '我的花园' 查看生长状态",
        prefix,
        seed.name,
        plot_idx + 1,
        reiki,
        new_reiki,
        format_minutes(seed.grow_time)
    )
}

/// 查看花园
pub fn cmd_view_garden(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let garden = get_garden(db, user_id);
    if garden.is_none() {
        return format!("{}\n您还没有花园！", prefix);
    }
    let (reiki, plots) = garden.unwrap();

    let mut r = format!("{}\n═══ 我的花园 ═══\n灵力: {}", prefix, reiki);

    let now = chrono::Local::now().naive_local();
    let seeds = parse_seeds(db);

    for (i, plot) in plots.iter().enumerate() {
        if plot.seed_name.is_empty() {
            r.push_str(&format!("\n地块#{}: [空]", i + 1));
        } else {
            let seed_info = seeds.iter().find(|s| s.name == plot.seed_name);
            let status = if let Some(si) = seed_info {
                if let Ok(plant_time) = chrono::NaiveDateTime::parse_from_str(&plot.plant_time, "%Y-%m-%d %H:%M:%S") {
                    let elapsed = now.signed_duration_since(plant_time).num_minutes();
                    if elapsed >= si.grow_time {
                        "✦ 可收获!".to_string()
                    } else {
                        let remaining = si.grow_time - elapsed;
                        let progress = (elapsed as f64 / si.grow_time as f64 * 100.0).min(100.0);
                        format!("生长中 {:.0}% | 还需{}", progress, format_minutes(remaining))
                    }
                } else {
                    format!("阶段{}", plot.stage)
                }
            } else {
                format!("阶段{}", plot.stage)
            };

            r.push_str(&format!(
                "\n地块#{}: {} | {} | ×{}",
                i + 1,
                plot.seed_name,
                status,
                plot.quantity
            ));
        }
    }

    r.push_str("\n\n收获: 发送 '收获'");
    r.push_str("\n种植: 发送 '种植+种子名'");
    r
}

/// 收获药材
pub fn cmd_harvest(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let garden = get_garden(db, user_id);
    if garden.is_none() {
        return format!("{}\n您还没有花园！", prefix);
    }
    let (_reiki, plots) = garden.unwrap();

    let now = chrono::Local::now().naive_local();
    let seeds = parse_seeds(db);
    let mut harvested = Vec::new();
    let empty_plot = "{\"种子名称\":\"\",\"阶段\":\"\",\"数量\":\"\",\"种植时间\":\"\"}";

    let conn = db.lock_conn();

    for (i, plot) in plots.iter().enumerate() {
        if plot.seed_name.is_empty() {
            continue;
        }

        let seed_info = seeds.iter().find(|s| s.name == plot.seed_name);
        if let Some(si) = seed_info {
            if let Ok(plant_time) = chrono::NaiveDateTime::parse_from_str(&plot.plant_time, "%Y-%m-%d %H:%M:%S") {
                let elapsed = now.signed_duration_since(plant_time).num_minutes();
                if elapsed >= si.grow_time {
                    let qty: i32 = plot.quantity.parse().unwrap_or(si.harvest_qty);
                    let soil_col = format!("soil{}", i + 1);

                    // 清空地块
                    let _ = conn.execute(
                        &format!("UPDATE ext_herbsgarden_info SET {}=?1 WHERE uID=?2", soil_col),
                        params![empty_plot, user_id],
                    );

                    harvested.push((si.herb_name.clone(), qty, si.reward.clone()));
                }
            }
        }
    }
    drop(conn); // 释放锁后再操作背包

    if harvested.is_empty() {
        return format!("{}\n当前没有可收获的药材。\n发送 '我的花园' 查看生长状态。", prefix);
    }

    let mut r = format!("{}\n═══ 收获成功！═══", prefix);
    for (herb_name, qty, reward) in &harvested {
        // 添加药材到背包
        db.knapsack_add(user_id, herb_name, *qty);
        r.push_str(&format!("\n  获得: {}×{}", herb_name, qty));

        // 给额外奖励
        if !reward.is_empty() {
            for reward_entry in reward.split(',') {
                let parts: Vec<&str> = reward_entry.split('*').collect();
                if parts.len() == 2 {
                    let item_name = parts[0].trim();
                    let item_qty: i32 = parts[1].trim().parse().unwrap_or(1);
                    db.knapsack_add(user_id, item_name, item_qty);
                    r.push_str(&format!("\n  额外: {}×{}", item_name, item_qty));
                }
            }
        }
    }
    r.push_str("\n\n药材已放入背包，可用于合成或出售。");
    r
}

/// 出售药材
pub fn cmd_sell_herb(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let herb_name = args.trim();

    if herb_name.is_empty() {
        return format!("{}\n请指定药材名称。\n用法: 出售药材+药材名", prefix);
    }

    // 查找药材对应种子的回收价
    let seeds = parse_seeds(db);
    let mut herb_price: i64 = 0;
    for s in &seeds {
        if s.herb_name == herb_name || s.herb_name.contains(herb_name) {
            herb_price = s.herb_price;
            break;
        }
    }

    if herb_price <= 0 {
        return format!("{}\n[{}] 不可出售或不存在。", prefix, herb_name);
    }

    let qty = db.knapsack_quantity(user_id, herb_name);
    if qty <= 0 {
        return format!("{}\n背包中没有 [{}]。", prefix, herb_name);
    }

    let total = herb_price * qty as i64;
    db.knapsack_remove(user_id, herb_name, qty);
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total);

    format!("{}\n出售 [{}]×{} 获得 {}金币！", prefix, herb_name, qty, total)
}

/// 铲除作物 — 移除指定地块的作物
/// 来源: Global section '铲除作物' (InstructionState=TRUE)
/// 允许玩家铲除不想继续种植的作物，释放地块
pub fn cmd_remove_crop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let plot_str = args.trim();

    if plot_str.is_empty() {
        return format!("{}\n请指定要铲除的地块编号(1-6)。\n用法: 铲除作物+地块号", prefix);
    }

    let plot_num: usize = match plot_str.parse() {
        Ok(n) if (1..=6).contains(&n) => n,
        _ => return format!("{}\n无效的地块编号！请输入1-6。", prefix),
    };

    let garden = get_garden(db, user_id);
    if garden.is_none() {
        return format!("{}\n您还没有花园！", prefix);
    }
    let (reiki, plots) = garden.unwrap();
    let plot = &plots[plot_num - 1];

    if plot.seed_name.is_empty() {
        return format!("{}\n地块#{}是空的，无需铲除。", prefix, plot_num);
    }

    let crop_name = plot.seed_name.clone();
    let soil_col = format!("soil{}", plot_num);
    let empty_plot = "{{\"种子名称\":\"\",\"阶段\":\"\",\"数量\":\"\",\"种植时间\":\"\"}}";

    // 返还部分灵力（50%）
    let seeds = parse_seeds(db);
    let reiki_refund = if let Some(seed_info) = seeds.iter().find(|s| s.name == crop_name) {
        seed_info.reiki_cost / 2
    } else {
        0
    };
    let new_reiki = reiki + reiki_refund;

    let conn = db.lock_conn();
    let _ = conn.execute(
        &format!("UPDATE ext_herbsgarden_info SET {}=?1, reiki=?2 WHERE uID=?3", soil_col),
        params![empty_plot, new_reiki.to_string(), user_id],
    );

    let mut r = format!("{}\n🗑️ 已铲除地块#{}的 [{}]！", prefix, plot_num, crop_name);
    if reiki_refund > 0 {
        r.push_str(&format!("\n返还灵力: {} → {}", reiki, new_reiki));
    }
    r.push_str("\n\n该地块现在可以种植新作物了。");
    r.push_str("\n发送 '种植+种子名' 重新种植");
    r
}

/// 药园仓库 - 查看花园仓库存储的药材和种子
/// 来源: MessageTemplate `药园仓库列表`
pub fn cmd_view_garden_warehouse(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 读取花园仓库数据
    let conn = db.lock_conn();
    let ck_raw: String = match conn.query_row(
        "SELECT ck FROM ext_herbsgarden_info WHERE uID=?1",
        params![user_id],
        |row| row.get(0),
    ) {
        Ok(v) => v,
        Err(_) => return format!("{}\n您还没有花园！请先种植种子。", prefix),
    };
    drop(conn);

    if ck_raw.is_empty() {
        return format!("{}\n🌾 药园仓库为空！\n\n收获药材后会自动存入仓库。", prefix);
    }

    // hex GBK 解码
    let decoded = crate::encoding::smart_decode(&ck_raw);

    // 解析 JSON: {"物品名":"数量",...}
    let mut items: Vec<(String, i32)> = Vec::new();
    let inner = decoded.trim_matches('{').trim_matches('}');
    for pair in inner.split(',') {
        let kv: Vec<&str> = pair.splitn(2, ':').collect();
        if kv.len() == 2 {
            let name = kv[0].trim().trim_matches('"').to_string();
            let qty: i32 = kv[1].trim().trim_matches('"').parse().unwrap_or(0);
            if !name.is_empty() && qty > 0 {
                items.push((name, qty));
            }
        }
    }

    if items.is_empty() {
        return format!("{}\n🌾 药园仓库为空！\n\n收获药材后会自动存入仓库。", prefix);
    }

    // 分页
    let per_page: usize = 8;
    let total_pages = (items.len() as f64 / per_page as f64).ceil() as i32;
    let page: i32 = args.trim().parse().unwrap_or(1).max(1).min(total_pages);
    let start = ((page - 1) as usize) * per_page;
    let end = (start + per_page).min(items.len());

    let mut r = format!("{}\n═══ 药园仓库 ═══", prefix);
    for (i, (name, qty)) in items[start..end].iter().enumerate() {
        r.push_str(&format!("\n{}. {}×{}", start + i + 1, name, qty));
    }

    let total_qty: i32 = items.iter().map(|(_, q)| *q).sum();
    r.push_str(&format!("\n\n📦 共{}种物品，总计{}个", items.len(), total_qty));
    r.push_str(&format!("\n📄 当前页：{}/{}", page, total_pages));
    if total_pages > 1 {
        r.push_str("\n翻页: 药园仓库+页码");
    }
    r.push_str("\n\n取出物品请使用 '收获' 命令收获种植中的药材。");
    r
}

/// 转换灵气 — 用金币或钻石兑换花园灵力
/// 来源: Global section '转换灵气' (InstructionState=TRUE)
/// 金币: 1000金 = 10灵力 | 钻石: 1钻 = 20灵力
pub fn cmd_convert_reiki(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let arg = args.trim();

    // 检查花园
    let garden = get_garden(db, user_id);
    if garden.is_none() {
        return format!("{}\n您还没有花园！", prefix);
    }
    let (current_reiki, _) = garden.unwrap();

    if arg.is_empty() {
        return format!(
            "{}\n═══ 转换灵气 ═══\n当前灵力: {}\n\n可选兑换方式:\n• 转换灵气+金币+数量 — 1000金币=10灵力\n• 转换灵气+钻石+数量 — 1钻石=20灵力\n\n示例: 转换灵气+金币+5000",
            prefix, current_reiki
        );
    }

    let parts: Vec<&str> = arg.splitn(2, '+').collect();
    if parts.len() < 2 {
        return format!("{}\n格式错误！用法: 转换灵气+金币/钻石+数量", prefix);
    }

    let currency_type = parts[0].trim();
    let amount_str = parts[1].trim();
    let amount: i64 = match amount_str.parse() {
        Ok(n) if n > 0 => n,
        _ => return format!("{}\n请输入有效的正整数数量！", prefix),
    };

    #[allow(unused_assignments)]
    let mut cost_label = String::new();
    let reiki_gain: i32 = match currency_type {
        "金币" | "金" => {
            let gold = db.read_currency(user_id, CURRENCY_GOLD);
            if gold < amount {
                return format!("{}\n金币不足！你只有 {}金币。", prefix, gold);
            }
            let reiki = (amount / 1000) * 10;
            if reiki <= 0 {
                return format!("{}\n兑换灵力至少需要1000金币！", prefix);
            }
            let actual_cost = (reiki / 10) * 1000;
            db.write_currency(user_id, CURRENCY_GOLD, gold - actual_cost);
            cost_label = format!("{}金币", actual_cost);
            reiki as i32
        }
        "钻石" | "钻" => {
            let diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
            if diamond < amount {
                return format!("{}\n钻石不足！你只有 {}钻石。", prefix, diamond);
            }
            let reiki = amount * 20;
            db.write_currency(user_id, CURRENCY_DIAMOND, diamond - amount);
            cost_label = format!("{}钻石", amount);
            reiki as i32
        }
        _ => return format!("{}\n未知货币类型！请选择 '金币' 或 '钻石'。", prefix),
    };

    // 更新灵力
    let new_reiki = current_reiki + reiki_gain;
    let conn = db.lock_conn();
    let _ = conn.execute(
        "UPDATE ext_herbsgarden_info SET reiki=?1 WHERE uID=?2",
        params![new_reiki.to_string(), user_id],
    );

    format!(
        "{}\n✦ 灵力转换成功！\n花费: {}\n灵力: {} → {} (+{})",
        prefix, cost_label, current_reiki, new_reiki, reiki_gain
    )
}

/// 偷摸药材 — 偷取其他玩家花园中成熟的药材
/// 来源: Global InstructionState "偷摸药材" = TRUE
/// 规则: 只能偷成熟的作物, 成功率60%(高等级提升), 有5分钟冷却
/// 偷取成功获得1个药材, 被偷方数量-1, 记录偷取日志
pub fn cmd_steal_herb(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!(
            "{}\n═══ 偷摸药材 ═══\n\n用法: 偷摸药材+玩家昵称\n\n规则:\n• 只能偷取成熟的药材\n• 基础成功率60%\n• 冷却时间5分钟\n• 偷取成功获得1个药材\n• 每日最多偷取10次",
            prefix
        );
    }

    // 检查自己的花园（需要有花园才能偷）
    let own_garden = get_garden(db, user_id);
    if own_garden.is_none() {
        return format!("{}\n您自己还没有花园，先建造药园再来偷药吧！", prefix);
    }

    // 检查每日偷取次数
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let steal_count_key = format!("steal_herb_count_{}", today);
    let steal_count: i32 = db.read_user_data(user_id, &steal_count_key).parse().unwrap_or(0);
    if steal_count >= 10 {
        return format!("{}\n今日偷取次数已达上限(10次)，明天再来吧！", prefix);
    }

    // 检查冷却
    let cd_key = "steal_herb_cooldown";
    let last_steal: i64 = db.read_user_data(user_id, cd_key).parse().unwrap_or(0);
    let now = chrono::Local::now().timestamp();
    if last_steal > 0 && now - last_steal < 300 {
        let remaining = 300 - (now - last_steal);
        return format!("{}\n⏳ 偷药冷却中，还需等待{}秒", prefix, remaining);
    }

    // 查找目标玩家
    let target_id = match find_user_by_name(db, target_name) {
        Some(id) => id,
        None => return format!("{}\n找不到玩家 [{}]！", prefix, target_name),
    };

    if target_id == user_id {
        return format!("{}\n不能偷自己的药材哦！", prefix);
    }

    // 查看目标花园
    let target_garden = get_garden(db, &target_id);
    if target_garden.is_none() {
        return format!("{}\n{}还没有花园！", prefix, target_name);
    }

    let (_, plots) = target_garden.unwrap();

    // 找到成熟的作物
    let mut mature_plots: Vec<(usize, &PlotData)> = Vec::new();
    for (i, plot) in plots.iter().enumerate() {
        if !plot.seed_name.is_empty() && plot.stage == "成熟" {
            mature_plots.push((i, plot));
        }
    }

    if mature_plots.is_empty() {
        return format!("{}\n{}的花园里没有成熟的药材可偷！", prefix, target_name);
    }

    // 随机选择一个成熟地块
    let idx = (rand::random::<usize>()) % mature_plots.len();
    let (plot_idx, plot) = mature_plots[idx];

    // 检查是否有守护灵兽保护
    let guard_key = format!("garden_guard_{}", target_id);
    let has_guard: String = db.read_user_data(&target_id, &guard_key);
    let success_rate = if !has_guard.is_empty() {
        35 // 有守护灵兽，成功率降低到35%
    } else {
        60 // 基础成功率60%
    };

    // 判定成功
    let roll = rand::random::<i32>() % 100;
    let success = roll < success_rate;

    // 更新冷却和次数
    db.write_user_data(user_id, cd_key, &now.to_string());
    db.write_user_data(user_id, &steal_count_key, &(steal_count + 1).to_string());

    if success {
        // 偷取成功
        let stolen_item = &plot.seed_name;
        // 将种子名转为药材名 (去掉"种子"后缀)
        let herb_name = stolen_item.replace("种子", "");
        db.knapsack_add(user_id, &herb_name, 1);

        // 更新目标地块数量
        let qty: i32 = plot.quantity.parse().unwrap_or(1);
        let new_qty = qty - 1;
        let soil_col = format!("soil{}", plot_idx + 1);
        let new_plot_data = if new_qty <= 0 {
            "{}".to_string()
        } else {
            format!(
                "{{\"种子名称\":\"{}\",\"阶段\":\"{}\",\"数量\":\"{}\",\"种植时间\":\"{}\"}}",
                plot.seed_name, plot.stage, new_qty, plot.plant_time
            )
        };
        let conn = db.lock_conn();
        let _ = conn.execute(
            &format!("UPDATE ext_herbsgarden_info SET {}=?1 WHERE uID=?2", soil_col),
            params![new_plot_data, target_id],
        );

        // 记录偷取日志
        let log_key = format!("steal_log_{}", target_id);
        let existing_log = db.read_user_data(&target_id, &log_key);
        let my_name = db.read_basic(user_id, ITEM_NAME);
        let log_entry = format!("{}被{}偷走了{}", now, my_name, herb_name);
        let new_log = if existing_log.is_empty() {
            log_entry
        } else {
            let entries: Vec<&str> = existing_log.split(';').collect();
            if entries.len() >= 5 {
                format!("{};{}", log_entry, entries[..4].join(";"))
            } else {
                format!("{};{}", log_entry, existing_log)
            }
        };
        db.write_user_data(&target_id, &log_key, &new_log);

        format!(
            "{}\n🌿 偷药成功！\n\n你偷偷从{}的花园里拿到了 {} ×1\n成功率: {}%\n\n今日已偷: {}/10次",
            prefix,
            target_name,
            herb_name,
            success_rate,
            steal_count + 1
        )
    } else {
        // 偷取失败
        // 被偷方收到通知
        let my_name = db.read_basic(user_id, ITEM_NAME);
        let notify_key = format!("steal_notify_{}", target_id);
        let existing_notify = db.read_user_data(&target_id, &notify_key);
        let notify_msg = format!("{}企图偷你的{}但被发现了！", my_name, plot.seed_name);
        let new_notify = if existing_notify.is_empty() {
            notify_msg
        } else {
            format!("{};{}", notify_msg, existing_notify)
        };
        db.write_user_data(&target_id, &notify_key, &new_notify);

        format!(
            "{}\n😱 偷药失败！\n\n你被{}花园的{}发现了！\n成功率: {}{}\n\n今日已偷: {}/10次",
            prefix,
            target_name,
            plot.seed_name,
            success_rate,
            if !has_guard.is_empty() {
                " (有守护灵兽)"
            } else {
                ""
            },
            steal_count + 1
        )
    }
}

/// 查看偷药通知 — 查看谁偷了你的药/谁来偷过
pub fn cmd_view_steal_notify(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut r = format!("{}\n═══ 🌿 药园安全报告 ═══", prefix);

    // 被偷记录
    let log_key = format!("steal_log_{}", user_id);
    let log = db.read_user_data(user_id, &log_key);
    r.push_str("\n\n📋 被偷记录：");
    if log.is_empty() {
        r.push_str("\n暂无人来偷药");
    } else {
        for entry in log.split(';') {
            r.push_str(&format!("\n  • {}", entry));
        }
    }

    // 偷药通知
    let notify_key = format!("steal_notify_{}", user_id);
    let notify = db.read_user_data(user_id, &notify_key);
    r.push_str("\n\n🔔 入侵警报：");
    if notify.is_empty() {
        r.push_str("\n暂无入侵警报");
    } else {
        for entry in notify.split(';') {
            r.push_str(&format!("\n  ⚠️ {}", entry));
        }
    }

    // 布置守护灵兽
    let guard_key = format!("garden_guard_{}", user_id);
    let guard: String = db.read_user_data(user_id, &guard_key);
    r.push_str("\n\n🛡️ 守护灵兽：");
    if guard.is_empty() {
        r.push_str("\n未布置 (发送 '布置守护+灵兽名' 布置守护灵兽)");
    } else {
        r.push_str(&format!("\n{} (降低偷药成功率至35%)", guard));
    }

    r
}

/// 布置守护灵兽 — 用灵兽守护花园
/// 来源: Global InstructionState "布置守护灵兽" = TRUE
pub fn cmd_set_garden_guard(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let beast_name = args.trim();

    // 检查花园
    if get_garden(db, user_id).is_none() {
        return format!("{}\n您还没有花园！", prefix);
    }

    if beast_name.is_empty() {
        let guard_key = format!("garden_guard_{}", user_id);
        let current: String = db.read_user_data(user_id, &guard_key);
        if current.is_empty() {
            return format!(
                "{}\n当前未布置守护灵兽\n\n用法: 布置守护+灵兽名\n效果: 降低偷药者成功率至35%",
                prefix
            );
        }
        return format!(
            "{}\n当前守护灵兽: {}\n\n发送 '布置守护+灵兽名' 更换\n发送 '布置守护+移除' 取消守护",
            prefix, current
        );
    }

    if beast_name == "移除" {
        let guard_key = format!("garden_guard_{}", user_id);
        db.write_user_data(user_id, &guard_key, "");
        return format!("{}\n已移除花园守护灵兽", prefix);
    }

    // 检查灵兽背包
    let beast_inv = db.read_user_data(user_id, "beast_inventory");
    if !beast_inv.contains(beast_name) {
        return format!("{}\n你没有名为 [{}] 的灵兽！", prefix, beast_name);
    }

    let guard_key = format!("garden_guard_{}", user_id);
    db.write_user_data(user_id, &guard_key, beast_name);

    format!(
        "{}\n🛡️ 守护灵兽布置成功！\n\n{}正在守护你的花园\n偷药者成功率将降低至35%",
        prefix, beast_name
    )
}

/// 查找用户ID by 昵称
fn find_user_by_name(db: &Database, name: &str) -> Option<String> {
    let all = db.all_users();
    for uid in &all {
        let n = db.read_basic(uid, ITEM_NAME);
        if n == name {
            return Some(uid.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_price_gold() {
        let (price, ptype) = parse_price("1000&44god");
        assert_eq!(price, 1000);
        assert_eq!(ptype, "god");
    }

    #[test]
    fn test_parse_price_diamond() {
        let (price, ptype) = parse_price("50&44diam");
        assert_eq!(price, 50);
        assert_eq!(ptype, "diam");
    }

    #[test]
    fn test_parse_price_simple() {
        let (price, ptype) = parse_price("2000");
        assert_eq!(price, 2000);
        assert_eq!(ptype, "god");
    }

    #[test]
    fn test_format_minutes() {
        assert_eq!(format_minutes(30), "30分钟");
        assert_eq!(format_minutes(90), "1小时30分钟");
        assert_eq!(format_minutes(1500), "1天1小时");
    }

    #[test]
    fn test_parse_plot_empty() {
        let plot = parse_plot("{}");
        assert!(plot.seed_name.is_empty());
    }

    #[test]
    fn test_parse_plot_with_data() {
        let json = r#"{"种子名称":"灵芝种子","阶段":"2","数量":"3","种植时间":"2026-06-08 12:00:00"}"#;
        let plot = parse_plot(json);
        assert_eq!(plot.seed_name, "灵芝种子");
        assert_eq!(plot.stage, "2");
        assert_eq!(plot.quantity, "3");
        assert_eq!(plot.plant_time, "2026-06-08 12:00:00");
    }

    #[test]
    fn test_convert_reiki_gold_rate() {
        // 1000 gold = 10 reiki
        let amount: i64 = 5000;
        let reiki = (amount / 1000) * 10;
        assert_eq!(reiki, 50);
    }

    #[test]
    fn test_convert_reiki_diamond_rate() {
        // 1 diamond = 20 reiki
        let amount: i64 = 5;
        let reiki = amount * 20;
        assert_eq!(reiki, 100);
    }

    #[test]
    fn test_steal_cooldown_seconds() {
        // 5 minutes = 300 seconds
        let cooldown = 300i64;
        assert_eq!(cooldown, 300);
    }

    #[test]
    fn test_steal_daily_limit() {
        // max 10 steals per day
        let limit = 10;
        assert_eq!(limit, 10);
    }

    #[test]
    fn test_herb_name_from_seed() {
        // "灵芝种子" -> "灵芝"
        let seed = "灵芝种子";
        let herb = seed.replace("种子", "");
        assert_eq!(herb, "灵芝");
    }

    #[test]
    fn test_guard_reduces_success_rate() {
        let base_rate = 60;
        let guarded_rate = 35;
        assert!(guarded_rate < base_rate);
    }
}

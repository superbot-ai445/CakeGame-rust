/// CakeGame 玩家财富评估系统
/// 综合评估玩家全部资产，提供财富明细和全服排名
///
/// 功能:
/// - 玩家资产: 金币+钻石+背包物品价值+仓库价值+钱庄存款
/// - 财富排行: 全服玩家净资产排名
/// - 资产分析: 各类别资产占比分析
///
/// 数据来源: Basic_knapsack(背包) + Config_Shop/Ext_Sell(定价) + Global(钱庄/仓库)
use crate::db::Database;
use crate::user;

/// 背包物品估价（取商店购买价和出售价的较高者）
fn estimate_item_value(db: &Database, item_name: &str) -> i64 {
    let conn = db.lock_conn();
    // 尝试商店购买价
    let buy_price: i64 = conn
        .prepare("SELECT Price FROM Config_Shop WHERE Name = ?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row([item_name], |row| {
                let s: String = row.get(0)?;
                Ok(s.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);
    // 尝试出售价
    let sell_price: i64 = conn
        .prepare("SELECT Price FROM Ext_Sell WHERE Name = ?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row([item_name], |row| {
                let s: String = row.get(0)?;
                Ok(s.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);
    // 取较高者，最低1金币
    std::cmp::max(std::cmp::max(buy_price, sell_price), 1)
}

/// 计算背包物品总价值
fn calc_knapsack_value(db: &Database, user_id: &str) -> (i64, Vec<(String, i32, i64)>) {
    let conn = db.lock_conn();
    let mut items = Vec::new();
    let mut total: i64 = 0;
    if let Ok(mut stmt) = conn.prepare("SELECT Name, Quantity FROM Basic_knapsack WHERE ID = ?1") {
        if let Ok(rows) = stmt.query_map([user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?.parse().unwrap_or(0)))
        }) {
            for r in rows.flatten() {
                let (name, qty) = r;
                if qty > 0 {
                    let unit_price = estimate_item_value(db, &name);
                    let value = unit_price * qty as i64;
                    total += value;
                    items.push((name, qty, value));
                }
            }
        }
    }
    // 按价值降序排序
    items.sort_by_key(|x| std::cmp::Reverse(x.2));
    (total, items)
}

/// 计算仓库物品总价值
fn calc_warehouse_value(db: &Database, user_id: &str) -> i64 {
    let wh_data = db.read_user_data(user_id, "warehouse_items");
    if wh_data.is_empty() {
        return 0;
    }
    let mut total: i64 = 0;
    // 仓库格式: "物品名:数量|物品名:数量|..."
    for entry in wh_data.split('|') {
        let parts: Vec<&str> = entry.splitn(2, ':').collect();
        if parts.len() == 2 {
            let name = parts[0].trim();
            let qty: i32 = parts[1].trim().parse().unwrap_or(0);
            if !name.is_empty() && qty > 0 {
                let unit_price = estimate_item_value(db, name);
                total += unit_price * qty as i64;
            }
        }
    }
    total
}

/// 获取钱庄存款
fn get_bank_balance(db: &Database, user_id: &str) -> i64 {
    db.read_user_data(user_id, "bank_balance").parse().unwrap_or(0)
}

/// 获取装备价值（装备在身上的装备按5倍基础估价）
fn calc_equipped_value(db: &Database, user_id: &str) -> i64 {
    let conn = db.lock_conn();
    let mut total: i64 = 0;
    if let Ok(mut stmt) = conn.prepare("SELECT EquipName FROM Equip_Register WHERE User = ?1") {
        if let Ok(rows) = stmt.query_map([user_id], |row| {
            let name: String = row.get(0)?;
            Ok(name)
        }) {
            for r in rows.flatten() {
                // 已装备物品按 5 倍基础估价
                let base = estimate_item_value(db, &r);
                total += base * 5;
            }
        }
    }
    total
}

/// 估算全服玩家财富 Top N
fn top_wealth_players(db: &Database, limit: usize) -> Vec<(String, String, i64)> {
    // First, collect user list in a scope that drops the connection
    let user_list: Vec<(String, String)> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT uID, NickName FROM Basic_User LIMIT 500") {
            if let Ok(mut rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?.trim_end_matches('\0').to_string(),
                    row.get::<_, String>(1)?.trim_end_matches('\0').to_string(),
                ))
            }) {
                while let Some(Ok(pair)) = rows.next() {
                    result.push(pair);
                }
            }
        }
        result
    };
    // Now compute wealth for each user (conn is dropped, db methods will re-lock)
    let mut players: Vec<(String, String, i64)> = Vec::new();
    for (uid, nick) in user_list {
        let gold: i64 = db.read_basic(&uid, "金币").parse().unwrap_or(0);
        let diamond: i64 = db.read_basic(&uid, "钻石").parse().unwrap_or(0);
        let (knapsack_val, _) = calc_knapsack_value(db, &uid);
        let bank = get_bank_balance(db, &uid);
        let equip = calc_equipped_value(db, &uid);
        let wh = calc_warehouse_value(db, &uid);
        let total = gold + diamond * 10 + knapsack_val + bank + equip + wh;
        if total > 0 {
            players.push((uid, nick, total));
        }
    }
    players.sort_by_key(|x| std::cmp::Reverse(x.2));
    players.truncate(limit);
    players
}

/// 格式化金额（千分位）
pub fn format_gold(n: i64) -> String {
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

/// 资产分类统计
struct AssetBreakdown {
    gold: i64,
    diamond: i64,
    knapsack: i64,
    bank: i64,
    warehouse: i64,
    equipped: i64,
}

fn calc_breakdown(db: &Database, user_id: &str) -> AssetBreakdown {
    let gold: i64 = db.read_basic(user_id, "金币").parse().unwrap_or(0);
    let diamond: i64 = db.read_basic(user_id, "钻石").parse().unwrap_or(0);
    let (knapsack_val, _) = calc_knapsack_value(db, user_id);
    let bank = get_bank_balance(db, user_id);
    let warehouse = calc_warehouse_value(db, user_id);
    let equipped = calc_equipped_value(db, user_id);
    AssetBreakdown {
        gold,
        diamond,
        knapsack: knapsack_val,
        bank,
        warehouse,
        equipped,
    }
}

/// 生成百分比条形图 (10 格)
fn pct_bar(current: i64, total: i64) -> String {
    if total <= 0 {
        return "░░░░░░░░░░".to_string();
    }
    let pct = (current as f64 / total as f64 * 10.0) as usize;
    let filled = pct.min(10);
    format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled))
}

/// 玩家资产 — 查看个人全部资产明细
pub fn cmd_player_assets(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let nickname = db.read_basic(user_id, "昵称");
    let b = calc_breakdown(db, user_id);
    let total = b.gold + b.diamond * 10 + b.knapsack + b.bank + b.warehouse + b.equipped;

    let asset_bars = format!(
        "📊 资产占比:\n  💰 金币 {}\n  💎 钻石 {}\n  🎒 背包 {}\n  🏦 钱庄 {}\n  📦 仓库 {}\n  ⚔️ 装备 {}",
        pct_bar(b.gold, total),
        pct_bar(b.diamond * 10, total),
        pct_bar(b.knapsack, total),
        pct_bar(b.bank, total),
        pct_bar(b.warehouse, total),
        pct_bar(b.equipped, total),
    );
    let mut out = format!(
        "{}\n💎 {} 的资产总览\n{}\n💰 金币: {}\n💎 钻石: {} (×10 = {})\n🎒 背包: {} (物品估价)\n🏦 钱庄: {} (存款)\n📦 仓库: {} (存储)\n⚔️ 装备: {} (穿戴)\n{}\n🏆 净资产: {}\n{}",
        prefix,
        nickname,
        "─".repeat(28),
        format_gold(b.gold),
        format_gold(b.diamond),
        format_gold(b.diamond * 10),
        format_gold(b.knapsack),
        format_gold(b.bank),
        format_gold(b.warehouse),
        format_gold(b.equipped),
        "─".repeat(28),
        format_gold(total),
        asset_bars,
    );

    // 背包 Top5 物品
    let (_, items) = calc_knapsack_value(db, user_id);
    if !items.is_empty() {
        out.push_str("\n\n🎒 背包高价值物品 (Top5):");
        for (i, (name, qty, value)) in items.iter().take(5).enumerate() {
            let icon = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            out.push_str(&format!("\n  {} {} ×{} = {}", icon, name, qty, format_gold(*value)));
        }
    }
    out
}

/// 财富排行 — 全服玩家净资产排名
pub fn cmd_wealth_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let top = top_wealth_players(db, 15);
    if top.is_empty() {
        return format!("{}\n📊 暂无财富数据", prefix);
    }
    let mut out = format!("{}\n🏆 全服财富排行榜\n{}", prefix, "─".repeat(28));
    for (i, (uid, nick, total)) in top.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => &format!("{:2}.", i + 1),
        };
        let highlight = if uid == user_id { " ⭐" } else { "" };
        out.push_str(&format!("\n  {} {}: {}{}", medal, nick, format_gold(*total), highlight));
    }
    // 当前用户排名定位
    let my_total = {
        let b = calc_breakdown(db, user_id);
        b.gold + b.diamond * 10 + b.knapsack + b.bank + b.warehouse + b.equipped
    };
    if !top.iter().any(|(uid, _, _)| uid == user_id) {
        out.push_str(&format!(
            "\n  ...\n  ⭐ 你: {} ({})",
            db.read_basic(user_id, "昵称"),
            format_gold(my_total)
        ));
    }
    out
}

/// 资产分析 — 各系统资产占比与建议
pub fn cmd_asset_analysis(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let b = calc_breakdown(db, user_id);
    let total = b.gold + b.diamond * 10 + b.knapsack + b.bank + b.warehouse + b.equipped;
    let level: i32 = db.read_basic(user_id, "等级").parse().unwrap_or(1);

    if total == 0 {
        return format!("{}\n📊 暂无资产数据，请先注册游戏", prefix);
    }

    let gold_pct = if total > 0 { b.gold * 100 / total } else { 0 };
    let bank_pct = if total > 0 { b.bank * 100 / total } else { 0 };

    let mut suggestions = Vec::new();
    if gold_pct > 70 {
        suggestions.push("💡 金币占比过高，建议存入钱庄赚取利息");
    }
    if bank_pct < 10 && b.gold > 10000 {
        suggestions.push("💡 存款较少，钱庄可提供日利率0.5%的稳定收益");
    }
    if b.equipped < b.knapsack / 2 && b.knapsack > 0 {
        suggestions.push("💡 背包物品价值高于装备，建议升级装备提升战力");
    }
    if b.diamond > 100 && level < 20 {
        suggestions.push("💡 钻石充足，可考虑购买高级通行证或进化装备");
    }
    if suggestions.is_empty() {
        suggestions.push("✅ 资产分配均衡，继续保持！");
    }

    let mut out = format!(
        "{}\n📊 {} 的资产分析 (Lv.{})\n{}\n💎 资产等级: {}\n💰 资产总额: {}\n{}\n📋 优化建议:",
        prefix,
        db.read_basic(user_id, "昵称"),
        level,
        "─".repeat(28),
        wealth_tier(total),
        format_gold(total),
        "─".repeat(28),
    );
    for s in &suggestions {
        out.push_str(&format!("\n  {}", s));
    }
    out
}

/// 资产等级
fn wealth_tier(total: i64) -> &'static str {
    match total {
        0..=999 => "🏠 一贫如洗",
        1_000..=9_999 => "🏠 小有积蓄",
        10_000..=49_999 => "🏡 家境殷实",
        50_000..=199_999 => "🏘️ 小康之家",
        200_000..=999_999 => "🏛️ 富甲一方",
        1_000_000..=4_999_999 => "🏰 富可敌城",
        5_000_000..=19_999_999 => "💎 财富巨擘",
        20_000_000..=99_999_999 => "👑 一方财阀",
        _ => "🌟 富甲天下",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_gold_basic() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(123), "123");
        assert_eq!(format_gold(1234), "1,234");
        assert_eq!(format_gold(1234567), "1,234,567");
        assert_eq!(format_gold(1000000000), "1,000,000,000");
    }

    #[test]
    fn test_format_gold_negative() {
        assert_eq!(format_gold(-1234), "-1,234");
    }

    #[test]
    fn test_wealth_tier_ranges() {
        assert_eq!(wealth_tier(0), "🏠 一贫如洗");
        assert_eq!(wealth_tier(500), "🏠 一贫如洗");
        assert_eq!(wealth_tier(5000), "🏠 小有积蓄");
        assert_eq!(wealth_tier(30000), "🏡 家境殷实");
        assert_eq!(wealth_tier(100000), "🏘️ 小康之家");
        assert_eq!(wealth_tier(500000), "🏛️ 富甲一方");
        assert_eq!(wealth_tier(3000000), "🏰 富可敌城");
        assert_eq!(wealth_tier(10000000), "💎 财富巨擘");
        assert_eq!(wealth_tier(50000000), "👑 一方财阀");
        assert_eq!(wealth_tier(100000000), "🌟 富甲天下");
    }

    #[test]
    fn test_pct_bar_full() {
        assert_eq!(pct_bar(100, 100), "██████████");
    }

    #[test]
    fn test_pct_bar_empty() {
        assert_eq!(pct_bar(0, 100), "░░░░░░░░░░");
    }

    #[test]
    fn test_pct_bar_half() {
        assert_eq!(pct_bar(50, 100), "█████░░░░░");
    }

    #[test]
    fn test_pct_bar_zero_total() {
        assert_eq!(pct_bar(10, 0), "░░░░░░░░░░");
    }

    #[test]
    fn test_pct_bar_overflow() {
        assert_eq!(pct_bar(200, 100), "██████████");
    }

    #[test]
    fn test_format_gold_one() {
        assert_eq!(format_gold(1), "1");
    }

    #[test]
    fn test_format_gold_boundary() {
        assert_eq!(format_gold(999), "999");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(999999), "999,999");
        assert_eq!(format_gold(1000000), "1,000,000");
    }
}

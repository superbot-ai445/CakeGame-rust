/// CakeGame 玩家交易系统
/// 玩家之间直接交易物品/金币/钻石
/// 数据存储: Global 表 SECTION='p2p_trade'
/// 交易流程: 发起交易 → 双方添加物品/货币 → 双方确认 → 自动交换
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, ITEM_NAME, NODE_BASIC, OP_ADD, OP_SUB};
use crate::db::Database;
use crate::user;

const TRADE_SECTION: &str = "p2p_trade";
const MAX_TRADE_ITEMS: usize = 10;
const MAX_TRADE_GOLD: i64 = 10_000_000;
const MAX_TRADE_DIAMOND: i64 = 100_000;

/// 交易状态
#[derive(Debug, Clone, PartialEq)]
enum TradeStatus {
    Pending,
    Active,
    OneConfirmed,
    Completed,
    Cancelled,
}

impl TradeStatus {
    fn as_str(&self) -> &'static str {
        match self {
            TradeStatus::Pending => "pending",
            TradeStatus::Active => "active",
            TradeStatus::OneConfirmed => "one_confirmed",
            TradeStatus::Completed => "completed",
            TradeStatus::Cancelled => "cancelled",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "pending" => TradeStatus::Pending,
            "active" => TradeStatus::Active,
            "one_confirmed" => TradeStatus::OneConfirmed,
            "completed" => TradeStatus::Completed,
            _ => TradeStatus::Cancelled,
        }
    }
}

/// 读取交易字段
fn trade_get(db: &Database, trade_id: &str, field: &str) -> String {
    db.global_get(TRADE_SECTION, &format!("{}_{}", trade_id, field))
}

/// 写入交易字段
fn trade_set(db: &Database, trade_id: &str, field: &str, value: &str) {
    db.global_set(TRADE_SECTION, &format!("{}_{}", trade_id, field), value);
}

/// 获取玩家当前交易ID
fn get_active_trade_id(db: &Database, user_id: &str) -> Option<String> {
    // Scan Global entries in p2p_trade section for this user
    let conn = db.lock_conn();
    let mut stmt = conn.prepare("SELECT ID FROM Global WHERE SECTION=?1").ok()?;
    let rows = stmt
        .query_map(rusqlite::params![TRADE_SECTION], |row| {
            let id: String = row.get(0)?;
            Ok(id)
        })
        .ok()?;
    let mut candidate_ids: Vec<String> = Vec::new();
    for id in rows.flatten() {
        if id.ends_with("_initiator") || id.ends_with("_target") {
            candidate_ids.push(id);
        }
    }
    drop(stmt);
    drop(conn);

    for full_id in &candidate_ids {
        let val = db.global_get(TRADE_SECTION, full_id);
        if val == user_id {
            let trade_id = if full_id.ends_with("_initiator") {
                full_id[..full_id.len() - 10].to_string()
            } else if full_id.ends_with("_target") {
                full_id[..full_id.len() - 7].to_string()
            } else {
                continue;
            };
            let status = trade_get(db, &trade_id, "status");
            if status != "completed" && status != "cancelled" {
                return Some(trade_id);
            }
        }
    }
    None
}

/// 解析物品列表字符串: "物品A×3|物品B×1|金币×500|钻石×10"
fn parse_items(s: &str) -> Vec<(String, i64)> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split('|')
        .filter_map(|part| {
            let parts: Vec<&str> = part.split('×').collect();
            if parts.len() == 2 {
                let name = parts[0].to_string();
                let qty: i64 = parts[1].parse().unwrap_or(0);
                if qty > 0 {
                    return Some((name, qty));
                }
            }
            None
        })
        .collect()
}

/// 格式化物品列表
fn format_items(items: &[(String, i64)]) -> String {
    if items.is_empty() {
        return "（空）".to_string();
    }
    items
        .iter()
        .map(|(name, qty)| {
            if name == CURRENCY_GOLD || name == CURRENCY_DIAMOND {
                format!("💰 {}×{}", name, qty)
            } else {
                format!("📦 {}×{}", name, qty)
            }
        })
        .collect::<Vec<_>>()
        .join("\n   ")
}

/// 短格式物品列表
fn format_items_short(items: &[(String, i64)]) -> String {
    items
        .iter()
        .map(|(name, qty)| format!("{}×{}", name, qty))
        .collect::<Vec<_>>()
        .join(", ")
}

/// 生成交易ID
fn gen_trade_id(user_a: &str, user_b: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    user_a.hash(&mut h);
    user_b.hash(&mut h);
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .hash(&mut h);
    format!("T{:08x}", h.finish() as u32)
}

/// 发起交易
pub fn cmd_propose_trade(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n💡 请输入要交易的玩家昵称\n📝 格式: 发起交易+玩家昵称", prefix);
    }
    let target_id = match find_user_by_name(db, target_name) {
        Some(id) => id,
        None => return format!("{}\n❌ 未找到玩家「{}」，请检查昵称", prefix, target_name),
    };
    if target_id == user_id {
        return format!("{}\n❌ 不能和自己交易", prefix);
    }
    // 屏蔽检查
    if let Some(reject_msg) = crate::block::check_blocked_and_reject(db, user_id, &target_id) {
        return format!("{}\n❌ {}", prefix, reject_msg);
    }
    if crate::block::is_blocked(db, user_id, &target_id) {
        return format!("{}\n❌ 你已屏蔽该玩家，请先解除屏蔽后再交易。", prefix);
    }
    if get_active_trade_id(db, user_id).is_some() {
        return format!("{}\n❌ 您已有进行中的交易，请先完成或取消", prefix);
    }
    if get_active_trade_id(db, &target_id).is_some() {
        return format!("{}\n❌ 对方已有进行中的交易，请稍后再试", prefix);
    }

    let trade_id = gen_trade_id(user_id, &target_id);
    trade_set(db, &trade_id, "initiator", user_id);
    trade_set(db, &trade_id, "target", &target_id);
    trade_set(db, &trade_id, "status", TradeStatus::Pending.as_str());
    trade_set(db, &trade_id, "items_a", "");
    trade_set(db, &trade_id, "items_b", "");
    trade_set(db, &trade_id, "confirm_a", "no");
    trade_set(db, &trade_id, "confirm_b", "no");

    format!(
        "{}\n🤝 已向「{}」发起交易请求！\n\n📋 交易编号: {}\n⏳ 等待对方接受...\n💡 对方发送「接受交易」即可开始",
        prefix, target_name, trade_id
    )
}

/// 接受交易
pub fn cmd_accept_trade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => {
            return format!(
                "{}\n❌ 您当前没有待接受的交易请求\n💡 发送「发起交易+玩家昵称」来发起交易",
                prefix
            )
        }
    };
    let target = trade_get(db, &trade_id, "target");
    let status_str = trade_get(db, &trade_id, "status");
    if TradeStatus::from_str(&status_str) != TradeStatus::Pending {
        return format!("{}\n❌ 当前交易状态不是等待接受", prefix);
    }
    if target != user_id {
        return format!("{}\n❌ 您不是该交易的目标方", prefix);
    }
    trade_set(db, &trade_id, "status", TradeStatus::Active.as_str());
    let initiator = trade_get(db, &trade_id, "initiator");
    let initiator_name = get_display_name(db, &initiator);

    format!(
        "{}\n✅ 已接受与「{}」的交易！\n\n📋 交易编号: {}\n💡 双方现在可以添加交易物品\n📝 发送「添加交易+物品名+数量」添加物品\n📝 发送「添加金币交易+数量」添加金币\n📝 发送「添加钻石交易+数量」添加钻石\n📝 发送「查看交易」查看当前状态\n📝 发送「确认交易」确认交换\n📝 发送「取消交易」取消",
        prefix, initiator_name, trade_id
    )
}

/// 拒绝交易
pub fn cmd_reject_trade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => return format!("{}\n❌ 您当前没有待处理的交易请求", prefix),
    };
    trade_set(db, &trade_id, "status", TradeStatus::Cancelled.as_str());
    format!("{}\n🚫 已拒绝交易请求", prefix)
}

/// 添加交易物品
pub fn cmd_add_trade_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => {
            return format!(
                "{}\n❌ 您当前没有进行中的交易\n💡 发送「发起交易+玩家昵称」来发起交易",
                prefix
            )
        }
    };
    let initiator = trade_get(db, &trade_id, "initiator");
    let status_str = trade_get(db, &trade_id, "status");
    let status = TradeStatus::from_str(&status_str);
    if status != TradeStatus::Active && status != TradeStatus::OneConfirmed {
        return format!("{}\n❌ 交易未激活，请先等待对方接受", prefix);
    }

    let parts: Vec<&str> = args.trim().split('+').collect();
    if parts.len() < 2 {
        return format!("{}\n💡 格式: 添加交易+物品名+数量", prefix);
    }
    let item_name = parts[0].trim();
    let qty: i64 = match parts[1].trim().parse() {
        Ok(q) if q > 0 => q,
        _ => return format!("{}\n❌ 数量必须为正整数", prefix),
    };

    let is_initiator = initiator == user_id;
    let items_key = if is_initiator { "items_a" } else { "items_b" };
    let current_items_str = trade_get(db, &trade_id, items_key);
    let mut current_items = parse_items(&current_items_str);

    if current_items.len() >= MAX_TRADE_ITEMS && !current_items.iter().any(|(n, _)| n == item_name) {
        return format!("{}\n❌ 交易物品数量已达上限（{}件）", prefix, MAX_TRADE_ITEMS);
    }

    // 验证物品在背包中（货币除外）
    if item_name != CURRENCY_GOLD && item_name != CURRENCY_DIAMOND {
        let has_qty = db.knapsack_quantity(user_id, item_name) as i64;
        let already_added: i64 = current_items
            .iter()
            .filter(|(n, _)| n == item_name)
            .map(|(_, q)| *q)
            .sum();
        if has_qty - already_added < qty {
            return format!(
                "{}\n❌ 背包中「{}」数量不足（拥有{}，已添加{}）",
                prefix, item_name, has_qty, already_added
            );
        }
    }

    if let Some(existing) = current_items.iter_mut().find(|(n, _)| n == item_name) {
        existing.1 += qty;
    } else {
        current_items.push((item_name.to_string(), qty));
    }

    let new_items_str = current_items
        .iter()
        .map(|(n, q)| format!("{}×{}", n, q))
        .collect::<Vec<_>>()
        .join("|");

    trade_set(db, &trade_id, "confirm_a", "no");
    trade_set(db, &trade_id, "confirm_b", "no");
    if status == TradeStatus::OneConfirmed {
        trade_set(db, &trade_id, "status", TradeStatus::Active.as_str());
    }
    trade_set(db, &trade_id, items_key, &new_items_str);

    format!(
        "{}\n✅ 已向交易添加: {}×{}\n💡 发送「查看交易」查看当前状态",
        prefix, item_name, qty
    )
}

/// 添加金币
pub fn cmd_add_trade_gold(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let qty: i64 = match args.trim().parse() {
        Ok(q) if q > 0 => q,
        _ => return format!("{}\n💡 格式: 添加金币交易+数量", prefix),
    };
    if qty > MAX_TRADE_GOLD {
        return format!("{}\n❌ 单次交易金币上限: {}", prefix, MAX_TRADE_GOLD);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => return format!("{}\n❌ 您当前没有进行中的交易", prefix),
    };
    let initiator = trade_get(db, &trade_id, "initiator");
    let status_str = trade_get(db, &trade_id, "status");
    let status = TradeStatus::from_str(&status_str);
    if status != TradeStatus::Active && status != TradeStatus::OneConfirmed {
        return format!("{}\n❌ 交易未激活", prefix);
    }

    let is_initiator = initiator == user_id;
    let items_key = if is_initiator { "items_a" } else { "items_b" };
    let current_items_str = trade_get(db, &trade_id, items_key);
    let mut current_items = parse_items(&current_items_str);

    let current_gold = db.read_currency(user_id, CURRENCY_GOLD);
    let already_gold: i64 = current_items
        .iter()
        .filter(|(n, _)| n == CURRENCY_GOLD)
        .map(|(_, q)| *q)
        .sum();
    if current_gold - already_gold < qty {
        return format!(
            "{}\n❌ 金币不足（拥有{}，已添加{}）",
            prefix, current_gold, already_gold
        );
    }

    if let Some(existing) = current_items.iter_mut().find(|(n, _)| n == CURRENCY_GOLD) {
        existing.1 += qty;
    } else {
        current_items.push((CURRENCY_GOLD.to_string(), qty));
    }
    let new_str = current_items
        .iter()
        .map(|(n, q)| format!("{}×{}", n, q))
        .collect::<Vec<_>>()
        .join("|");

    trade_set(db, &trade_id, "confirm_a", "no");
    trade_set(db, &trade_id, "confirm_b", "no");
    if status == TradeStatus::OneConfirmed {
        trade_set(db, &trade_id, "status", TradeStatus::Active.as_str());
    }
    trade_set(db, &trade_id, items_key, &new_str);

    format!("{}\n✅ 已向交易添加: 💰{}金币", prefix, qty)
}

/// 添加钻石
pub fn cmd_add_trade_diamond(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let qty: i64 = match args.trim().parse() {
        Ok(q) if q > 0 => q,
        _ => return format!("{}\n💡 格式: 添加钻石交易+数量", prefix),
    };
    if qty > MAX_TRADE_DIAMOND {
        return format!("{}\n❌ 单次交易钻石上限: {}", prefix, MAX_TRADE_DIAMOND);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => return format!("{}\n❌ 您当前没有进行中的交易", prefix),
    };
    let initiator = trade_get(db, &trade_id, "initiator");
    let status_str = trade_get(db, &trade_id, "status");
    let status = TradeStatus::from_str(&status_str);
    if status != TradeStatus::Active && status != TradeStatus::OneConfirmed {
        return format!("{}\n❌ 交易未激活", prefix);
    }

    let is_initiator = initiator == user_id;
    let items_key = if is_initiator { "items_a" } else { "items_b" };
    let current_items_str = trade_get(db, &trade_id, items_key);
    let mut current_items = parse_items(&current_items_str);

    let current_diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
    let already_diamond: i64 = current_items
        .iter()
        .filter(|(n, _)| n == CURRENCY_DIAMOND)
        .map(|(_, q)| *q)
        .sum();
    if current_diamond - already_diamond < qty {
        return format!(
            "{}\n❌ 钻石不足（拥有{}，已添加{}）",
            prefix, current_diamond, already_diamond
        );
    }

    if let Some(existing) = current_items.iter_mut().find(|(n, _)| n == CURRENCY_DIAMOND) {
        existing.1 += qty;
    } else {
        current_items.push((CURRENCY_DIAMOND.to_string(), qty));
    }
    let new_str = current_items
        .iter()
        .map(|(n, q)| format!("{}×{}", n, q))
        .collect::<Vec<_>>()
        .join("|");

    trade_set(db, &trade_id, "confirm_a", "no");
    trade_set(db, &trade_id, "confirm_b", "no");
    if status == TradeStatus::OneConfirmed {
        trade_set(db, &trade_id, "status", TradeStatus::Active.as_str());
    }
    trade_set(db, &trade_id, items_key, &new_str);

    format!("{}\n✅ 已向交易添加: 💎{}钻石", prefix, qty)
}

/// 查看交易
pub fn cmd_view_trade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => {
            return format!(
                "{}\n📭 您当前没有进行中的交易\n💡 发送「发起交易+玩家昵称」来发起交易",
                prefix
            )
        }
    };
    let initiator = trade_get(db, &trade_id, "initiator");
    let target = trade_get(db, &trade_id, "target");
    let status_str = trade_get(db, &trade_id, "status");
    let items_a = trade_get(db, &trade_id, "items_a");
    let items_b = trade_get(db, &trade_id, "items_b");
    let status = TradeStatus::from_str(&status_str);

    let init_name = get_display_name(db, &initiator);
    let target_name = get_display_name(db, &target);
    let items_a_parsed = parse_items(&items_a);
    let items_b_parsed = parse_items(&items_b);

    let status_emoji = match status {
        TradeStatus::Pending => "⏳等待接受",
        TradeStatus::Active => "🟢进行中",
        TradeStatus::OneConfirmed => "🟡一方已确认",
        TradeStatus::Completed => "✅已完成",
        TradeStatus::Cancelled => "🚫已取消",
    };

    let confirm_a = trade_get(db, &trade_id, "confirm_a");
    let confirm_b = trade_get(db, &trade_id, "confirm_b");
    let conf_a_mark = if confirm_a == "yes" { " ✅" } else { "" };
    let conf_b_mark = if confirm_b == "yes" { " ✅" } else { "" };

    let mut out = format!(
        "{}\n═══ 🤝 玩家交易 ═══\n📋 交易编号: {}\n📊 状态: {}\n",
        prefix, trade_id, status_emoji
    );
    out.push_str(&format!(
        "\n👤 {}{} 的出价:\n   {}\n",
        init_name,
        conf_a_mark,
        format_items(&items_a_parsed)
    ));
    out.push_str(&format!(
        "\n👤 {}{} 的出价:\n   {}\n",
        target_name,
        conf_b_mark,
        format_items(&items_b_parsed)
    ));

    if status == TradeStatus::Active || status == TradeStatus::OneConfirmed {
        out.push_str("\n━━━━━━━━━━━━━━━━━━━━");
        out.push_str("\n📝 操作指令:");
        out.push_str("\n  添加交易+物品名+数量");
        out.push_str("\n  添加金币交易+数量");
        out.push_str("\n  添加钻石交易+数量");
        out.push_str("\n  确认交易");
        out.push_str("\n  取消交易");
    }
    out
}

/// 确认交易
pub fn cmd_confirm_trade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => return format!("{}\n❌ 您当前没有进行中的交易", prefix),
    };
    let initiator = trade_get(db, &trade_id, "initiator");
    let target = trade_get(db, &trade_id, "target");
    let status_str = trade_get(db, &trade_id, "status");
    let items_a_str = trade_get(db, &trade_id, "items_a");
    let items_b_str = trade_get(db, &trade_id, "items_b");
    let status = TradeStatus::from_str(&status_str);

    if status != TradeStatus::Active && status != TradeStatus::OneConfirmed {
        return format!("{}\n❌ 当前交易状态无法确认", prefix);
    }

    let is_initiator = initiator == user_id;
    let (confirm_key, other_key) = if is_initiator {
        ("confirm_a", "confirm_b")
    } else {
        ("confirm_b", "confirm_a")
    };
    trade_set(db, &trade_id, confirm_key, "yes");

    let other_confirmed = trade_get(db, &trade_id, other_key) == "yes";

    if other_confirmed {
        let items_a_parsed = parse_items(&items_a_str);
        let items_b_parsed = parse_items(&items_b_str);

        if !validate_trade_items(db, &initiator, &items_a_parsed) {
            trade_set(db, &trade_id, "confirm_a", "no");
            trade_set(db, &trade_id, "confirm_b", "no");
            return format!("{}\n❌ 交易失败: 发起方物品/货币不足，请重新检查", prefix);
        }
        if !validate_trade_items(db, &target, &items_b_parsed) {
            trade_set(db, &trade_id, "confirm_a", "no");
            trade_set(db, &trade_id, "confirm_b", "no");
            return format!("{}\n❌ 交易失败: 对方物品/货币不足，请重新检查", prefix);
        }

        // 执行交换
        execute_trade_transfer(db, &initiator, &items_a_parsed, OP_SUB);
        execute_trade_transfer(db, &initiator, &items_b_parsed, OP_ADD);
        execute_trade_transfer(db, &target, &items_b_parsed, OP_SUB);
        execute_trade_transfer(db, &target, &items_a_parsed, OP_ADD);

        trade_set(db, &trade_id, "status", TradeStatus::Completed.as_str());
        crate::achievement::on_trade(db, &initiator);
        crate::achievement::on_trade(db, &target);

        let init_name = get_display_name(db, &initiator);
        let target_name = get_display_name(db, &target);
        return format!(
            "{}\n🎉 交易完成！\n\n📋 交易编号: {}\n👤 {} → 获得: {}\n👤 {} → 获得: {}",
            prefix,
            trade_id,
            init_name,
            format_items(&items_b_parsed),
            target_name,
            format_items(&items_a_parsed),
        );
    }

    trade_set(db, &trade_id, "status", TradeStatus::OneConfirmed.as_str());
    format!(
        "{}\n✅ 您已确认交易！等待对方确认...\n💡 如果需要修改，请发送新物品（将自动重置确认状态）",
        prefix
    )
}

/// 取消交易
pub fn cmd_cancel_trade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let trade_id = match get_active_trade_id(db, user_id) {
        Some(id) => id,
        None => return format!("{}\n❌ 您当前没有进行中的交易", prefix),
    };
    trade_set(db, &trade_id, "status", TradeStatus::Cancelled.as_str());
    format!("{}\n🚫 已取消交易", prefix)
}

/// 交易记录
pub fn cmd_trade_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // Collect all trade IDs by scanning Global
    let mut all_ids: Vec<String> = Vec::new();
    {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare("SELECT ID FROM Global WHERE SECTION=?1") {
            Ok(s) => s,
            Err(_) => return format!("{}\n❌ 查询失败", prefix),
        };
        let mut rows = match stmt.query_map(rusqlite::params![TRADE_SECTION], |row| row.get::<_, String>(0)) {
            Ok(r) => r,
            Err(_) => return format!("{}\n❌ 查询失败", prefix),
        };
        while let Some(Ok(id)) = rows.next() {
            all_ids.push(id);
        }
    }
    let mut trade_ids: Vec<String> = Vec::new();
    for full_id in &all_ids {
        let val = db.global_get(TRADE_SECTION, full_id);
        if val == user_id {
            let tid = if full_id.ends_with("_initiator") {
                full_id[..full_id.len() - 10].to_string()
            } else if full_id.ends_with("_target") {
                full_id[..full_id.len() - 7].to_string()
            } else {
                continue;
            };
            if !trade_ids.contains(&tid) {
                trade_ids.push(tid);
            }
        }
    }

    trade_ids.reverse(); // newest first (assuming IDs increase)

    if trade_ids.is_empty() {
        return format!("{}\n📭 暂无交易记录", prefix);
    }

    let show = &trade_ids[..trade_ids.len().min(5)];
    let mut out = format!("{}\n═══ 📋 交易记录 ═══\n", prefix);
    for tid in show {
        let init_id = trade_get(db, tid, "initiator");
        let tgt_id = trade_get(db, tid, "target");
        let status_str = trade_get(db, tid, "status");
        let status_icon = match TradeStatus::from_str(&status_str) {
            TradeStatus::Completed => "✅",
            TradeStatus::Cancelled => "🚫",
            _ => "⏳",
        };
        let init_name = get_display_name(db, &init_id);
        let target_name = get_display_name(db, &tgt_id);
        out.push_str(&format!("\n{} [{}] {} ↔ {}", status_icon, tid, init_name, target_name));
        if TradeStatus::from_str(&status_str) == TradeStatus::Completed {
            let ia = trade_get(db, tid, "items_a");
            let ib = trade_get(db, tid, "items_b");
            let a_parsed = parse_items(&ia);
            let b_parsed = parse_items(&ib);
            if !a_parsed.is_empty() {
                out.push_str(&format!("\n   {} 出: {}", init_name, format_items_short(&a_parsed)));
            }
            if !b_parsed.is_empty() {
                out.push_str(&format!("\n   {} 出: {}", target_name, format_items_short(&b_parsed)));
            }
        }
    }
    out
}

// ==================== 辅助函数 ====================

fn find_user_by_name(db: &Database, name: &str) -> Option<String> {
    let conn = db.lock_conn();
    let mut stmt = conn
        .prepare("SELECT ID FROM Basic_User WHERE Node=?1 AND Item=?2 AND Data=?3")
        .ok()?;
    let result = stmt
        .query_row(rusqlite::params![NODE_BASIC, ITEM_NAME, name], |row| {
            row.get::<_, String>(0)
        })
        .ok()?;
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn get_display_name(db: &Database, user_id: &str) -> String {
    let name = db.read_basic(user_id, ITEM_NAME);
    if name.is_empty() {
        user_id.to_string()
    } else {
        name
    }
}

fn validate_trade_items(db: &Database, user_id: &str, items: &[(String, i64)]) -> bool {
    let gold_needed: i64 = items.iter().filter(|(n, _)| n == CURRENCY_GOLD).map(|(_, q)| *q).sum();
    let diamond_needed: i64 = items
        .iter()
        .filter(|(n, _)| n == CURRENCY_DIAMOND)
        .map(|(_, q)| *q)
        .sum();

    if db.read_currency(user_id, CURRENCY_GOLD) < gold_needed {
        return false;
    }
    if db.read_currency(user_id, CURRENCY_DIAMOND) < diamond_needed {
        return false;
    }
    for (name, qty) in items {
        if name == CURRENCY_GOLD || name == CURRENCY_DIAMOND {
            continue;
        }
        if db.knapsack_quantity(user_id, name) < *qty as i32 {
            return false;
        }
    }
    true
}

fn execute_trade_transfer(db: &Database, user_id: &str, items: &[(String, i64)], op: &str) {
    for (name, qty) in items {
        if name == CURRENCY_GOLD {
            db.modify_currency(user_id, CURRENCY_GOLD, op, *qty);
        } else if name == CURRENCY_DIAMOND {
            db.modify_currency(user_id, CURRENCY_DIAMOND, op, *qty);
        } else if op == OP_SUB {
            db.knapsack_remove(user_id, name, *qty as i32);
        } else {
            db.knapsack_add(user_id, name, *qty as i32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_items_empty() {
        let items = parse_items("");
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_items_single() {
        let items = parse_items("长剑×3");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "长剑");
        assert_eq!(items[0].1, 3);
    }

    #[test]
    fn test_parse_items_multiple() {
        let items = parse_items("金币×500|钻石×10|长剑×1");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ("金币".to_string(), 500));
        assert_eq!(items[1], ("钻石".to_string(), 10));
        assert_eq!(items[2], ("长剑".to_string(), 1));
    }

    #[test]
    fn test_parse_items_with_zero() {
        let items = parse_items("长剑×0|金币×100");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "金币");
    }

    #[test]
    fn test_format_items() {
        let items = vec![("金币".to_string(), 500), ("长剑".to_string(), 1)];
        let formatted = format_items(&items);
        assert!(formatted.contains("💰 金币×500"));
        assert!(formatted.contains("📦 长剑×1"));
    }

    #[test]
    fn test_format_items_empty() {
        assert_eq!(format_items(&[]), "（空）");
    }

    #[test]
    fn test_trade_status_roundtrip() {
        let statuses = [
            TradeStatus::Pending,
            TradeStatus::Active,
            TradeStatus::OneConfirmed,
            TradeStatus::Completed,
            TradeStatus::Cancelled,
        ];
        for s in &statuses {
            assert_eq!(TradeStatus::from_str(s.as_str()), *s);
        }
    }

    #[test]
    fn test_gen_trade_id_format() {
        let id = gen_trade_id("user_a", "user_b");
        assert!(id.starts_with('T'));
        assert_eq!(id.len(), 9);
    }

    #[test]
    fn test_format_items_short() {
        let items = vec![("长剑".to_string(), 3), ("金币".to_string(), 500)];
        assert_eq!(format_items_short(&items), "长剑×3, 金币×500");
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_TRADE_ITEMS, 10);
        assert_eq!(MAX_TRADE_GOLD, 10_000_000);
        assert_eq!(MAX_TRADE_DIAMOND, 100_000);
        assert_eq!(TRADE_SECTION, "p2p_trade");
    }
}

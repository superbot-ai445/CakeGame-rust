/// CakeGame 公会仓库系统
///
/// 公会成员可以将物品存入公会仓库，供其他成员取用
/// 仓库数据存储在 Global 表 SECTION='guild_warehouse'
/// 会长可管理仓库容量，普通成员可存取物品
///
/// 指令: 查看公会仓库, 存入公会仓库, 取出公会仓库, 公会仓库信息
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 公会仓库默认容量（格子数）
const DEFAULT_CAPACITY: i32 = 10;
/// 公会仓库最大容量
const MAX_CAPACITY: i32 = 50;
/// 每格最大堆叠数量
const MAX_STACK: i32 = 999;
/// 存取冷却时间（秒）
const COOLDOWN_SECS: i64 = 10;

/// 解析 "物品名:数量" 格式
fn parse_slot(data: &str) -> (String, i32) {
    if data.is_empty() {
        return (String::new(), 0);
    }
    if let Some((name, qty_str)) = data.rsplit_once(':') {
        (name.to_string(), qty_str.parse().unwrap_or(0))
    } else {
        (data.to_string(), 0)
    }
}

/// 获取公会仓库容量
fn get_capacity(db: &Database, guild: &str) -> i32 {
    let cap_str = db.global_get("guild_warehouse", &format!("{}.capacity", guild));
    if cap_str.is_empty() {
        db.global_set(
            "guild_warehouse",
            &format!("{}.capacity", guild),
            &DEFAULT_CAPACITY.to_string(),
        );
        DEFAULT_CAPACITY
    } else {
        cap_str.parse().unwrap_or(DEFAULT_CAPACITY)
    }
}

/// 获取公会仓库所有物品
fn get_items(db: &Database, guild: &str) -> Vec<(String, i32)> {
    let capacity = get_capacity(db, guild);
    let mut items = Vec::new();
    for i in 0..capacity {
        let key = format!("{}.slot_{}", guild, i);
        let data = db.global_get("guild_warehouse", &key);
        let (name, qty) = parse_slot(&data);
        if !name.is_empty() && qty > 0 {
            items.push((name, qty));
        }
    }
    items
}

/// 查找公会仓库中已有物品的槽位
fn find_item_slot(db: &Database, guild: &str, item_name: &str) -> Option<i32> {
    let capacity = get_capacity(db, guild);
    for i in 0..capacity {
        let key = format!("{}.slot_{}", guild, i);
        let data = db.global_get("guild_warehouse", &key);
        let (name, qty) = parse_slot(&data);
        if name == item_name && qty > 0 && qty < MAX_STACK {
            return Some(i);
        }
    }
    None
}

/// 查找空槽位
fn find_empty_slot(db: &Database, guild: &str) -> Option<i32> {
    let capacity = get_capacity(db, guild);
    for i in 0..capacity {
        let key = format!("{}.slot_{}", guild, i);
        let data = db.global_get("guild_warehouse", &key);
        let (name, qty) = parse_slot(&data);
        if name.is_empty() || qty <= 0 {
            return Some(i);
        }
    }
    None
}

/// 检查冷却
fn check_cooldown(db: &Database, user_id: &str, action: &str) -> bool {
    let key = format!("gw_{}_cd", action);
    let last = db.read_user_data(user_id, &key);
    if last.is_empty() {
        return true;
    }
    if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(&last, "%Y-%m-%d %H:%M:%S") {
        let now = chrono::Local::now().naive_local();
        (now - last_time).num_seconds() >= COOLDOWN_SECS
    } else {
        true
    }
}

/// 记录冷却
fn set_cooldown(db: &Database, user_id: &str, action: &str) {
    let key = format!("gw_{}_cd", action);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.write_user_data(user_id, &key, &now);
}

/// 获取物品品质图标
fn quality_icon(item_name: &str) -> &'static str {
    if item_name.contains("超界") {
        "🌈"
    } else if item_name.contains("史诗") {
        "🟣"
    } else if item_name.contains("稀有") {
        "🔵"
    } else if item_name.contains("精良") {
        "🟢"
    } else if item_name.contains("普通") {
        "⚪"
    } else {
        "📦"
    }
}

/// 查看公会仓库 — 显示仓库物品列表
pub fn cmd_view_guild_warehouse(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看公会仓库！", prefix);
    }

    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您尚未加入公会！请先加入一个公会。", prefix);
    }

    let capacity = get_capacity(db, &guild);
    let items = get_items(db, &guild);
    let used_slots = items.len();

    let mut out = format!(
        "{}\n╔═══════════════════════════╗\n║  🏛️ 公会仓库 [{}]  ║\n╚═══════════════════════════╝\n",
        prefix, guild
    );

    if items.is_empty() {
        out.push_str("\n仓库空空如也…\n快往里面存点好东西吧！");
    } else {
        out.push_str(&format!(
            "\n📦 格子: {}/{}\n━━━━━━━━━━━━━━━━━━━━\n",
            used_slots, capacity
        ));
        for (i, (name, qty)) in items.iter().enumerate() {
            let icon = quality_icon(name);
            out.push_str(&format!("  {} {}. {} ×{}\n", icon, i + 1, name, qty));
        }
        out.push_str("━━━━━━━━━━━━━━━━━━━━");
    }

    out.push_str("\n\n💡 存入: 存入公会仓库+物品名+数量\n💡 取出: 取出公会仓库+物品名+数量\n💡 信息: 公会仓库信息");

    out
}

/// 存入公会仓库 — 从背包存入物品
pub fn cmd_deposit_guild_warehouse(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册！", prefix);
    }

    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您尚未加入公会！", prefix);
    }

    // 解析参数: 物品名+数量
    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n请指定要存入的物品。\n用法: 存入公会仓库+物品名+数量\n例如: 存入公会仓库+生命药水+5",
            prefix
        );
    }

    let parts: Vec<&str> = args.split('+').collect();
    let item_name = parts[0].trim();
    let quantity: i32 = if parts.len() > 1 {
        parts[1].trim().parse().unwrap_or(1)
    } else {
        1
    };

    if quantity <= 0 {
        return format!("{}\n数量必须大于0！", prefix);
    }

    // 检查冷却
    if !check_cooldown(db, user_id, "deposit") {
        return format!("{}\n操作太频繁，请稍后再试。（{}秒冷却）", prefix, COOLDOWN_SECS);
    }

    // 检查背包是否有足够物品
    let owned = db.knapsack_quantity(user_id, item_name);
    if owned < quantity {
        return format!(
            "{}\n背包中 [{}] 数量不足！当前: {}, 需要: {}",
            prefix, item_name, owned, quantity
        );
    }

    // 检查仓库空间
    let existing_slot = find_item_slot(db, &guild, item_name);
    let target_slot = if let Some(slot) = existing_slot {
        // 检查堆叠上限
        let key = format!("{}.slot_{}", guild, slot);
        let data = db.global_get("guild_warehouse", &key);
        let (_, current_qty) = parse_slot(&data);
        if current_qty + quantity > MAX_STACK {
            // 需要新槽位
            match find_empty_slot(db, &guild) {
                Some(s) => s,
                None => {
                    return format!("{}\n公会仓库已满！无法继续存放 [{}]。", prefix, item_name);
                }
            }
        } else {
            slot
        }
    } else {
        match find_empty_slot(db, &guild) {
            Some(s) => s,
            None => {
                return format!("{}\n公会仓库已满！无法继续存放。", prefix);
            }
        }
    };

    // 从背包扣除物品
    if !db.knapsack_remove(user_id, item_name, quantity) {
        return format!("{}\n存入失败，无法从背包扣除物品。", prefix);
    }

    // 写入仓库
    let key = format!("{}.slot_{}", guild, target_slot);
    let data = db.global_get("guild_warehouse", &key);
    let (existing_name, existing_qty) = parse_slot(&data);

    let is_same_item = existing_name == item_name;
    let is_empty = existing_name.is_empty();

    let new_name = if is_empty { item_name.to_string() } else { existing_name };
    let new_qty = if is_same_item || is_empty {
        existing_qty + quantity
    } else {
        // 不应该到这里，但安全起见
        quantity
    };

    db.global_set("guild_warehouse", &key, &format!("{}:{}", new_name, new_qty));

    // 记录存入日志
    let log_key = format!("{}.log", guild);
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let log_entry = format!("[{}] {}存入 {}×{}", now, user_id, item_name, quantity);
    let existing_log = db.global_get("guild_warehouse", &log_key);
    let new_log = if existing_log.is_empty() {
        log_entry
    } else {
        // 保留最近10条日志
        let mut logs: Vec<&str> = existing_log.split('\n').collect();
        logs.insert(0, &log_entry);
        if logs.len() > 10 {
            logs.truncate(10);
        }
        logs.join("\n")
    };
    db.global_set("guild_warehouse", &log_key, &new_log);

    set_cooldown(db, user_id, "deposit");

    format!(
        "{}\n✅ 存入成功！\n📦 {} ×{} → 公会仓库 [{}]\n🎒 背包剩余: {}",
        prefix,
        item_name,
        quantity,
        guild,
        db.knapsack_quantity(user_id, item_name)
    )
}

/// 取出公会仓库 — 从仓库取出物品到背包
pub fn cmd_withdraw_guild_warehouse(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册！", prefix);
    }

    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您尚未加入公会！", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n请指定要取出的物品。\n用法: 取出公会仓库+物品名+数量\n例如: 取出公会仓库+生命药水+3",
            prefix
        );
    }

    let parts: Vec<&str> = args.split('+').collect();
    let item_name = parts[0].trim();
    let quantity: i32 = if parts.len() > 1 {
        parts[1].trim().parse().unwrap_or(1)
    } else {
        1
    };

    if quantity <= 0 {
        return format!("{}\n数量必须大于0！", prefix);
    }

    // 检查冷却
    if !check_cooldown(db, user_id, "withdraw") {
        return format!("{}\n操作太频繁，请稍后再试。（{}秒冷却）", prefix, COOLDOWN_SECS);
    }

    // 查找仓库中的物品
    let capacity = get_capacity(db, &guild);
    let mut found_slot: Option<i32> = None;
    let mut found_qty = 0i32;

    for i in 0..capacity {
        let key = format!("{}.slot_{}", guild, i);
        let data = db.global_get("guild_warehouse", &key);
        let (name, qty) = parse_slot(&data);
        if name == item_name && qty > 0 {
            found_slot = Some(i);
            found_qty = qty;
            break;
        }
    }

    let slot = match found_slot {
        Some(s) => s,
        None => {
            return format!(
                "{}\n公会仓库中没有 [{}]！\n发送 '查看公会仓库' 查看仓库内容。",
                prefix, item_name
            );
        }
    };

    if found_qty < quantity {
        return format!(
            "{}\n公会仓库中 [{}] 数量不足！当前: {}, 需要: {}",
            prefix, item_name, found_qty, quantity
        );
    }

    // 从仓库扣除
    let key = format!("{}.slot_{}", guild, slot);
    let new_qty = found_qty - quantity;
    if new_qty <= 0 {
        db.global_set("guild_warehouse", &key, "");
    } else {
        db.global_set("guild_warehouse", &key, &format!("{}:{}", item_name, new_qty));
    }

    // 添加到背包
    db.knapsack_add(user_id, item_name, quantity);

    // 记录取出日志
    let log_key = format!("{}.log", guild);
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let log_entry = format!("[{}] {}取出 {}×{}", now, user_id, item_name, quantity);
    let existing_log = db.global_get("guild_warehouse", &log_key);
    let new_log = if existing_log.is_empty() {
        log_entry
    } else {
        let mut logs: Vec<&str> = existing_log.split('\n').collect();
        logs.insert(0, &log_entry);
        if logs.len() > 10 {
            logs.truncate(10);
        }
        logs.join("\n")
    };
    db.global_set("guild_warehouse", &log_key, &new_log);

    set_cooldown(db, user_id, "withdraw");

    format!(
        "{}\n✅ 取出成功！\n📦 {} ×{} ← 公会仓库 [{}]\n🎒 背包现有: {}",
        prefix,
        item_name,
        quantity,
        guild,
        db.knapsack_quantity(user_id, item_name)
    )
}

/// 公会仓库信息 — 显示仓库统计和操作日志
pub fn cmd_guild_warehouse_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册！", prefix);
    }

    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您尚未加入公会！", prefix);
    }

    let capacity = get_capacity(db, &guild);
    let items = get_items(db, &guild);
    let used_slots = items.len();

    // 计算物品总价值
    let mut total_items = 0i32;
    for (_, qty) in &items {
        total_items += qty;
    }

    // 统计物品种类
    let mut categories: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for (name, qty) in &items {
        let cat = if name.contains("药水") || name.contains("药剂") {
            "药剂"
        } else if name.contains("矿石") || name.contains("宝石") {
            "矿石/宝石"
        } else if name.contains("武器") || name.contains("剑") || name.contains("刀") {
            "武器"
        } else if name.contains("铠甲") || name.contains("护腿") || name.contains("靴子") || name.contains("头盔")
        {
            "防具"
        } else if name.contains("种子") || name.contains("药材") {
            "种植材料"
        } else {
            "其他"
        };
        *categories.entry(cat.to_string()).or_insert(0) += qty;
    }

    let mut out = format!(
        "{}\n╔═══════════════════════════╗\n║  📊 公会仓库信息 [{}]  ║\n╚═══════════════════════════╝\n",
        prefix, guild
    );

    out.push_str(&format!(
        "\n📦 容量: {}/{} 格\n📋 物品种类: {} 种\n📊 物品总量: {} 件\n",
        used_slots,
        capacity,
        items.len(),
        total_items
    ));

    // 显示分类统计
    if !categories.is_empty() {
        out.push_str("\n━━━ 分类统计 ━━━\n");
        let mut sorted: Vec<_> = categories.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (cat, qty) in sorted {
            out.push_str(&format!("  {} {} ×{}\n", category_icon(cat), cat, qty));
        }
    }

    // 显示最近操作日志
    let log_key = format!("{}.log", guild);
    let log = db.global_get("guild_warehouse", &log_key);
    if !log.is_empty() {
        out.push_str("\n━━━ 最近操作 ━━━\n");
        let entries: Vec<&str> = log.split('\n').take(5).collect();
        for entry in entries {
            out.push_str(&format!("  {}\n", entry));
        }
    }

    // 会长可扩容提示
    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner == user_id && capacity < MAX_CAPACITY {
        out.push_str(&format!(
            "\n💡 会长特权: 发送 '公会仓库扩容' 可增加仓库容量（当前: {}/{})",
            capacity, MAX_CAPACITY
        ));
    }

    out
}

/// 分类图标
fn category_icon(cat: &str) -> &'static str {
    match cat {
        "药剂" => "🧪",
        "矿石/宝石" => "💎",
        "武器" => "⚔️",
        "防具" => "🛡️",
        "种植材料" => "🌱",
        _ => "📦",
    }
}

/// 公会仓库扩容 — 会长可增加仓库容量
pub fn cmd_expand_guild_warehouse(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册！", prefix);
    }

    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您尚未加入公会！", prefix);
    }

    // 检查是否是会长
    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n只有公会会长才能扩容仓库！", prefix);
    }

    let current_cap = get_capacity(db, &guild);
    if current_cap >= MAX_CAPACITY {
        return format!("{}\n公会仓库已达最大容量！({}格)", prefix, MAX_CAPACITY);
    }

    // 扩容费用: 每格500金币
    let expand_amount = 5; // 每次扩容5格
    let new_cap = std::cmp::min(current_cap + expand_amount, MAX_CAPACITY);
    let cost = (expand_amount as i64) * 500;

    // 检查金币
    let gold: i64 = db.read_basic(user_id, CURRENCY_GOLD).parse().unwrap_or(0);
    if gold < cost {
        return format!(
            "{}\n金币不足！扩容需要 {} 金币，当前: {} 金币\n（每格500金币，本次扩容{}格）",
            prefix, cost, gold, expand_amount
        );
    }

    // 扣除金币
    db.write_basic(user_id, CURRENCY_GOLD, &(gold - cost).to_string());

    // 增加容量
    db.global_set("guild_warehouse", &format!("{}.capacity", guild), &new_cap.to_string());

    format!(
        "{}\n✅ 公会仓库扩容成功！\n📦 容量: {}格 → {}格\n💰 花费: {} 金币\n\n感谢会长的慷慨贡献！",
        prefix, current_cap, new_cap, cost
    )
}

/// 查看公会仓库日志 — 单独查看操作记录
pub fn cmd_guild_warehouse_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册！", prefix);
    }

    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您尚未加入公会！", prefix);
    }

    let log_key = format!("{}.log", guild);
    let log = db.global_get("guild_warehouse", &log_key);

    let mut out = format!("{}\n═══ 📜 公会仓库操作日志 [{}] ═══", prefix, guild);

    if log.is_empty() {
        out.push_str("\n\n暂无操作记录。");
    } else {
        out.push('\n');
        for entry in log.split('\n') {
            out.push_str(&format!("  {}\n", entry));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slot() {
        assert_eq!(parse_slot("生命药水:5"), ("生命药水".to_string(), 5));
        assert_eq!(parse_slot("铁剑:1"), ("铁剑".to_string(), 1));
        assert_eq!(parse_slot(""), (String::new(), 0));
        assert_eq!(parse_slot("无数量"), ("无数量".to_string(), 0));
        assert_eq!(parse_slot("高级药水:999"), ("高级药水".to_string(), 999));
        // 测试冒号在物品名中的情况（rsplit_once 取最后一个冒号）
        assert_eq!(parse_slot("物品:特殊:10"), ("物品:特殊".to_string(), 10));
    }

    #[test]
    fn test_quality_icon() {
        assert_eq!(quality_icon("【超界】光寒圣剑"), "🌈");
        assert_eq!(quality_icon("【史诗】秋叶刀"), "🟣");
        assert_eq!(quality_icon("【稀有】银月弓"), "🔵");
        assert_eq!(quality_icon("【精良】铁剑"), "🟢");
        assert_eq!(quality_icon("【普通】木棍"), "⚪");
        assert_eq!(quality_icon("生命药水"), "📦");
    }

    #[test]
    fn test_category_icon() {
        assert_eq!(category_icon("药剂"), "🧪");
        assert_eq!(category_icon("矿石/宝石"), "💎");
        assert_eq!(category_icon("武器"), "⚔️");
        assert_eq!(category_icon("防具"), "🛡️");
        assert_eq!(category_icon("种植材料"), "🌱");
        assert_eq!(category_icon("其他"), "📦");
    }

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_CAPACITY, 10);
        assert_eq!(MAX_CAPACITY, 50);
        assert_eq!(MAX_STACK, 999);
        assert_eq!(COOLDOWN_SECS, 10);
        assert!(DEFAULT_CAPACITY <= MAX_CAPACITY);
    }

    #[test]
    fn test_parse_slot_edge_cases() {
        // 零数量
        assert_eq!(parse_slot("药水:0"), ("药水".to_string(), 0));
        // 负数量
        assert_eq!(parse_slot("药水:-1"), ("药水".to_string(), -1));
        // 只有冒号
        assert_eq!(parse_slot(":"), ("".to_string(), 0));
        // 大数量
        assert_eq!(parse_slot("超级药水:99999"), ("超级药水".to_string(), 99999));
    }
}

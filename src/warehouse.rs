/// CakeGame 仓库系统
/// 玩家可以将背包物品存入仓库，需要时再取出
/// 仓库初始容量5格，可通过扩容石升级（最多50格）
use crate::core::*;
use crate::db::Database;

/// 仓库数据存储节点名
#[allow(dead_code)]
const WAREHOUSE_NODE: &str = "warehouse";
/// 仓库容量Key
const CAPACITY_KEY: &str = "capacity";
/// 仓库格子数前缀 (warehouse_slot_0..N 存储 "物品名:数量")
const SLOT_PREFIX: &str = "warehouse_slot_";
/// 默认仓库容量
const DEFAULT_CAPACITY: i32 = 5;
/// 最大仓库容量
const MAX_CAPACITY: i32 = 50;

/// 获取仓库当前容量
fn get_capacity(db: &Database, user_id: &str) -> i32 {
    let cap_str = db.read_user_data(user_id, CAPACITY_KEY);
    let cap: i32 = if cap_str.is_empty() {
        // 写入默认容量
        db.write_user_data(user_id, CAPACITY_KEY, &DEFAULT_CAPACITY.to_string());
        DEFAULT_CAPACITY
    } else {
        cap_str.parse().unwrap_or(DEFAULT_CAPACITY)
    };
    cap
}

/// 解析仓库槽位数据 "物品名:数量" → (name, qty)
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

/// 获取仓库所有物品
fn get_warehouse_items(db: &Database, user_id: &str) -> Vec<(String, i32)> {
    let capacity = get_capacity(db, user_id);
    let mut items = Vec::new();
    for i in 0..capacity {
        let key = format!("{}{}", SLOT_PREFIX, i);
        let data = db.read_user_data(user_id, &key);
        if !data.is_empty() {
            let (name, qty) = parse_slot(&data);
            if !name.is_empty() && qty > 0 {
                items.push((name, qty));
            }
        }
    }
    items
}

/// 查找仓库中已有该物品的槽位
fn find_item_slot(db: &Database, user_id: &str, item_name: &str) -> Option<i32> {
    let capacity = get_capacity(db, user_id);
    for i in 0..capacity {
        let key = format!("{}{}", SLOT_PREFIX, i);
        let data = db.read_user_data(user_id, &key);
        let (name, _qty) = parse_slot(&data);
        if name == item_name {
            return Some(i);
        }
    }
    None
}

/// 查找仓库中空闲槽位
fn find_empty_slot(db: &Database, user_id: &str) -> Option<i32> {
    let capacity = get_capacity(db, user_id);
    for i in 0..capacity {
        let key = format!("{}{}", SLOT_PREFIX, i);
        let data = db.read_user_data(user_id, &key);
        if data.is_empty() {
            let (_name, qty) = parse_slot(&data);
            if qty <= 0 {
                return Some(i);
            }
        }
    }
    None
}

/// 写入槽位数据
fn write_slot(db: &Database, user_id: &str, slot: i32, name: &str, qty: i32) {
    let key = format!("{}{}", SLOT_PREFIX, slot);
    if qty <= 0 || name.is_empty() {
        db.write_user_data(user_id, &key, "");
    } else {
        db.write_user_data(user_id, &key, &format!("{}:{}", name, qty));
    }
}

/// 获取槽位数据
fn get_slot_qty(db: &Database, user_id: &str, slot: i32) -> (String, i32) {
    let key = format!("{}{}", SLOT_PREFIX, slot);
    let data = db.read_user_data(user_id, &key);
    parse_slot(&data)
}

/// 查看仓库
pub fn cmd_view_warehouse(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let capacity = get_capacity(db, user_id);
    let items = get_warehouse_items(db, user_id);
    let used = items.len();
    let mut r = format!("{}\n═══ 仓库 ═══", prefix);
    r.push_str(&format!("\n📦 容量：{}/{}", used, capacity));

    if items.is_empty() {
        r.push_str("\n\n仓库空空如也~");
        r.push_str("\n发送'存入仓库+物品名'将背包物品存入仓库");
    } else {
        r.push_str("\n\n仓库物品：");
        for (name, qty) in &items {
            r.push_str(&format!("\n  📦 {} ×{}", name, qty));
        }
    }

    r.push_str("\n\n操作指令：");
    r.push_str("\n  存入仓库+物品名 — 存入物品");
    r.push_str("\n  取出仓库+物品名 — 取出物品");
    r.push_str("\n  仓库详情 — 查看容量和升级信息");
    r
}

/// 仓库存入物品
pub fn cmd_warehouse_deposit(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!("{}\n请指定要存入的物品名！\n发送'存入仓库+物品名'", prefix);
    }

    // 检查背包是否有该物品
    let qty = db.knapsack_quantity(user_id, item_name);
    if qty <= 0 {
        return format!("{}\n背包中没有 [{}]！", prefix, item_name);
    }

    // 检查物品是否为装备（已装备的不能存入）
    let equips = crate::db::Database::equip_all(db, user_id);
    if equips.iter().any(|e| e.name == item_name) {
        return format!("{}\n装备中的物品无法存入仓库！请先卸下装备。", prefix);
    }

    let capacity = get_capacity(db, user_id);

    // 查找仓库中已有该物品的槽位
    if let Some(slot) = find_item_slot(db, user_id, item_name) {
        let (_name, existing_qty) = get_slot_qty(db, user_id, slot);
        // 存入全部数量
        write_slot(db, user_id, slot, item_name, existing_qty + qty);
        db.knapsack_remove(user_id, item_name, qty);
        return format!(
            "{}\n✅ 存入成功！\n📦 {} ×{} → 仓库 (槽位{})\n仓库内该物品共: {}",
            prefix,
            item_name,
            qty,
            slot + 1,
            existing_qty + qty
        );
    }

    // 查找空闲槽位
    if let Some(slot) = find_empty_slot(db, user_id) {
        write_slot(db, user_id, slot, item_name, qty);
        db.knapsack_remove(user_id, item_name, qty);
        format!(
            "{}\n✅ 存入成功！\n📦 {} ×{} → 仓库 (槽位{})\n📦 容量：{}/{}",
            prefix,
            item_name,
            qty,
            slot + 1,
            get_warehouse_items(db, user_id).len(),
            capacity
        )
    } else {
        format!(
            "{}\n❌ 仓库已满！容量：{}/{}\n发送'仓库升级'可扩容仓库",
            prefix,
            get_warehouse_items(db, user_id).len(),
            capacity
        )
    }
}

/// 仓库取出物品
pub fn cmd_warehouse_withdraw(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!("{}\n请指定要取出的物品名！\n发送'取出仓库+物品名'", prefix);
    }

    // 查找仓库中该物品
    if let Some(slot) = find_item_slot(db, user_id, item_name) {
        let (name, qty) = get_slot_qty(db, user_id, slot);
        if qty <= 0 {
            // 清除无效槽位
            write_slot(db, user_id, slot, "", 0);
            return format!("{}\n仓库中没有 [{}]！", prefix, item_name);
        }

        // 取出到背包
        db.knapsack_add(user_id, &name, qty);
        write_slot(db, user_id, slot, "", 0);

        format!("{}\n✅ 取出成功！\n📦 {} ×{} ← 仓库", prefix, name, qty)
    } else {
        format!("{}\n仓库中没有 [{}]！", prefix, item_name)
    }
}

/// 仓库详情（容量信息和升级）
pub fn cmd_warehouse_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let capacity = get_capacity(db, user_id);
    let items = get_warehouse_items(db, user_id);
    let used = items.len();

    let mut r = format!("{}\n═══ 仓库详情 ═══", prefix);
    r.push_str(&format!("\n📦 仓库容量：{}/{}", used, capacity));
    r.push_str(&format!("\n🔒 已用槽位：{}", used));
    r.push_str(&format!("\n🔓 空闲槽位：{}", capacity - used as i32));

    if capacity < MAX_CAPACITY {
        let next_cap = std::cmp::min(capacity + 5, MAX_CAPACITY);
        let cost_diamond = (capacity / 5) * 50 + 50; // 每次升级递增50钻石
        r.push_str("\n\n━━━ 仓库升级 ━━━");
        r.push_str(&format!("\n📤 当前容量：{}", capacity));
        r.push_str(&format!("\n📥 升级后容量：{}", next_cap));
        r.push_str(&format!("\n💎 升级费用：{}钻石", cost_diamond));
        r.push_str("\n发送'仓库升级'进行升级");
    } else {
        r.push_str("\n\n⭐ 仓库已达最大容量！");
    }

    // 显示仓库物品列表
    if !items.is_empty() {
        r.push_str("\n\n━━━ 仓库物品清单 ━━━");
        for (idx, (name, qty)) in items.iter().enumerate() {
            r.push_str(&format!("\n  {}. {} ×{}", idx + 1, name, qty));
        }
    }

    r
}

/// 仓库升级
pub fn cmd_warehouse_upgrade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let capacity = get_capacity(db, user_id);

    if capacity >= MAX_CAPACITY {
        return format!("{}\n⭐ 仓库已达最大容量 {} 格！", prefix, MAX_CAPACITY);
    }

    let next_cap = std::cmp::min(capacity + 5, MAX_CAPACITY);
    let cost_diamond = (capacity / 5) * 50 + 50;

    // 检查钻石
    let diamonds = db.read_currency(user_id, CURRENCY_DIAMOND);
    if diamonds < cost_diamond as i64 {
        return format!(
            "{}\n💎 钻石不足！升级需要{}钻石，当前{}钻石",
            prefix, cost_diamond, diamonds
        );
    }

    // 扣除钻石
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, cost_diamond as i64);
    db.write_user_data(user_id, CAPACITY_KEY, &next_cap.to_string());

    format!(
        "{}\n🎉 仓库升级成功！\n📦 容量：{} → {}\n💎 消耗：{}钻石\n💎 剩余：{}钻石",
        prefix,
        capacity,
        next_cap,
        cost_diamond,
        diamonds - cost_diamond as i64
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slot() {
        assert_eq!(parse_slot("铁剑:3"), ("铁剑".to_string(), 3));
        assert_eq!(parse_slot("生命药水:100"), ("生命药水".to_string(), 100));
        assert_eq!(parse_slot(""), (String::new(), 0));
        assert_eq!(parse_slot("无效数据"), ("无效数据".to_string(), 0));
    }

    #[test]
    fn test_parse_slot_with_colon_in_name() {
        assert_eq!(parse_slot("【普通】生命药水:5"), ("【普通】生命药水".to_string(), 5));
    }
}

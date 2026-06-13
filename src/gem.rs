/// CakeGame 宝石镶嵌系统
/// 为装备添加宝石孔位，镶嵌宝石可获得额外属性加成
/// 支持：查看宝石/镶嵌宝石/卸下宝石/合成宝石/宝石背包/宝石属性
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 宝石等级
const GEM_TIERS: &[(&str, i32)] = &[("初级", 1), ("中级", 2), ("高级", 3), ("顶级", 4), ("传说", 5)];

/// 宝石类型定义
#[allow(dead_code)]
struct GemType {
    name: &'static str,
    attr_key: &'static str,
    attr_name: &'static str,
    base_value: i32,
    tier_mult: i32,
}

const GEM_TYPES: &[GemType] = &[
    GemType {
        name: "红宝石",
        attr_key: "gem_hp",
        attr_name: "生命",
        base_value: 50,
        tier_mult: 2,
    },
    GemType {
        name: "蓝宝石",
        attr_key: "gem_mp",
        attr_name: "魔法",
        base_value: 30,
        tier_mult: 2,
    },
    GemType {
        name: "黄宝石",
        attr_key: "gem_ad",
        attr_name: "物攻",
        base_value: 10,
        tier_mult: 2,
    },
    GemType {
        name: "紫宝石",
        attr_key: "gem_ap",
        attr_name: "魔攻",
        base_value: 10,
        tier_mult: 2,
    },
    GemType {
        name: "绿宝石",
        attr_key: "gem_def",
        attr_name: "防御",
        base_value: 8,
        tier_mult: 2,
    },
    GemType {
        name: "白宝石",
        attr_key: "gem_mr",
        attr_name: "魔抗",
        base_value: 8,
        tier_mult: 2,
    },
];

/// 装备槽位最大宝石孔数
const MAX_SLOTS_PER_EQUIP: usize = 3;

/// 宝石合成消耗数量（N个同级 → 1个高级）
const MERGE_COST: i32 = 3;

/// 获取完整宝石名（带等级前缀）
fn gem_full_name(base_name: &str, tier: i32) -> String {
    let tier_name = GEM_TIERS
        .iter()
        .find(|(_, t)| *t == tier)
        .map(|(n, _)| *n)
        .unwrap_or("初级");
    format!("{}{}×{}", tier_name, base_name, tier)
}

/// 解析宝石物品名，返回 (基础名, 等级)
fn parse_gem_name(name: &str) -> Option<(&'static str, i32)> {
    for &(tier_prefix, tier_val) in GEM_TIERS {
        for gt in GEM_TYPES {
            let full = format!("{}{}", tier_prefix, gt.name);
            if name.starts_with(&full) || name == full {
                return Some((gt.name, tier_val));
            }
        }
    }
    None
}

/// 计算宝石属性值
fn gem_attr_value(base_value: i32, tier: i32) -> i32 {
    base_value * tier * tier
}

/// 获取装备已镶嵌宝石（存储格式: "宝石名1|宝石名2|宝石名3"，空位用空串）
fn get_socketed_gems(db: &Database, user_id: &str, slot: &str) -> Vec<String> {
    let key = format!("gem_socket_{}", slot);
    let raw = db.read_user_data(user_id, &key);
    if raw.is_empty() {
        return vec![String::new(); MAX_SLOTS_PER_EQUIP];
    }
    let mut gems: Vec<String> = raw.split('|').map(|s| s.to_string()).collect();
    gems.resize(MAX_SLOTS_PER_EQUIP, String::new());
    gems
}

/// 保存装备镶嵌宝石
fn set_socketed_gems(db: &Database, user_id: &str, slot: &str, gems: &[String]) {
    let key = format!("gem_socket_{}", slot);
    let val = gems.join("|");
    db.write_user_data(user_id, &key, &val);
}

/// 获取玩家宝石背包（存储格式: "宝石名1:数量1|宝石名2:数量2"）
fn get_gem_inventory(db: &Database, user_id: &str) -> Vec<(String, i32)> {
    let raw = db.read_user_data(user_id, "gem_inventory");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].parse::<i32>().unwrap_or(0)))
            } else {
                None
            }
        })
        .filter(|(_, qty)| *qty > 0)
        .collect()
}

/// 保存宝石背包
fn set_gem_inventory(db: &Database, user_id: &str, inv: &[(String, i32)]) {
    let val: Vec<String> = inv
        .iter()
        .filter(|(_, qty)| *qty > 0)
        .map(|(name, qty)| format!("{}:{}", name, qty))
        .collect();
    db.write_user_data(user_id, "gem_inventory", &val.join("|"));
}

/// 添加宝石到背包
fn add_gem_to_inventory(db: &Database, user_id: &str, gem_name: &str, qty: i32) {
    let mut inv = get_gem_inventory(db, user_id);
    if let Some(entry) = inv.iter_mut().find(|(n, _)| n == gem_name) {
        entry.1 += qty;
    } else {
        inv.push((gem_name.to_string(), qty));
    }
    set_gem_inventory(db, user_id, &inv);
}

/// 从背包移除宝石
fn remove_gem_from_inventory(db: &Database, user_id: &str, gem_name: &str, qty: i32) -> bool {
    let mut inv = get_gem_inventory(db, user_id);
    if let Some(entry) = inv.iter_mut().find(|(n, _)| n == gem_name) {
        if entry.1 >= qty {
            entry.1 -= qty;
            set_gem_inventory(db, user_id, &inv);
            return true;
        }
    }
    false
}

/// 查看宝石背包中某个宝石的数量
#[allow(dead_code)]
fn gem_count_in_inventory(db: &Database, user_id: &str, gem_name: &str) -> i32 {
    let inv = get_gem_inventory(db, user_id);
    inv.iter().find(|(n, _)| n == gem_name).map(|(_, q)| *q).unwrap_or(0)
}

// ==================== 公开指令 ====================

/// 查看宝石 — 显示所有宝石类型和属性
pub fn cmd_view_gems(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ 💎 宝石系统 ═══\n", prefix);
    out.push_str("宝石可镶嵌到装备上获得额外属性\n");
    out.push_str("每件装备最多3个宝石孔位\n\n");

    out.push_str("📊 宝石类型：\n");
    for gt in GEM_TYPES {
        let v1 = gem_attr_value(gt.base_value, 1);
        let v3 = gem_attr_value(gt.base_value, 3);
        let v5 = gem_attr_value(gt.base_value, 5);
        out.push_str(&format!(
            "  {} +{}(初级) / +{}(高级) / +{}(传说) {}\n",
            gt.name, v1, v3, v5, gt.attr_name
        ));
    }

    out.push_str("\n📊 宝石等级：\n");
    for &(name, tier) in GEM_TIERS {
        out.push_str(&format!("  {} Lv.{} (×{}属性)\n", name, tier, tier * tier));
    }

    out.push_str("\n📋 指令列表：\n");
    out.push_str("  宝石背包 — 查看拥有的宝石\n");
    out.push_str("  镶嵌宝石+宝石名+槽位 — 镶嵌宝石\n");
    out.push_str("  卸下宝石+槽位 — 卸下宝石\n");
    out.push_str("  合成宝石+宝石名 — 3个同级合成1个高级\n");
    out.push_str("  宝石属性 — 查看镶嵌宝石提供的总属性\n");
    out
}

/// 宝石背包 — 查看拥有的宝石
pub fn cmd_gem_inventory(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let inv = get_gem_inventory(db, user_id);

    let mut out = format!("{}\n═══ 💎 宝石背包 ═══\n", prefix);

    if inv.is_empty() {
        out.push_str("\n背包中没有宝石！\n");
        out.push_str("💡 宝石来源：\n");
        out.push_str("  · 击败BOSS掉落\n");
        out.push_str("  · 副本通关奖励\n");
        out.push_str("  · 炼制系统合成\n");
        out.push_str("  · 商店购买\n");
        return out;
    }

    // 按等级分组
    for &(tier_name, tier_val) in GEM_TIERS {
        let tier_gems: Vec<&(String, i32)> = inv
            .iter()
            .filter(|(name, _)| parse_gem_name(name).map(|(_, t)| t == tier_val).unwrap_or(false))
            .collect();
        if !tier_gems.is_empty() {
            out.push_str(&format!("\n【{}】\n", tier_name));
            for (name, qty) in tier_gems {
                out.push_str(&format!("  {} ×{}\n", name, qty));
            }
        }
    }

    let total: i32 = inv.iter().map(|(_, q)| *q).sum();
    out.push_str(&format!("\n📊 共 {} 颗宝石\n", total));
    out.push_str("💡 使用「镶嵌宝石+宝石名+槽位名」镶嵌\n");
    out
}

/// 镶嵌宝石 — 将宝石镶嵌到指定装备槽位
pub fn cmd_socket_gem(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再镶嵌宝石！", prefix);
    }

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return format!(
            "{}\n用法：镶嵌宝石+宝石名+槽位\n槽位：武器/头盔/胸甲/护肩/腰带/鞋子/项链/戒指/称号\n\n💡 使用「宝石背包」查看拥有的宝石",
            prefix
        );
    }

    let gem_query = parts[0];
    let slot = parts[1];

    let valid_slots = ["武器", "头盔", "胸甲", "护肩", "腰带", "鞋子", "项链", "戒指", "称号"];
    if !valid_slots.contains(&slot) {
        return format!(
            "{}\n无效槽位「{}」！\n有效槽位：{}",
            prefix,
            slot,
            valid_slots.join("、")
        );
    }

    let equip_name = db.read_equip_name(user_id, slot);
    if equip_name == "拳头" || equip_name.is_empty() {
        return format!("{}\n{}槽位没有装备！请先装备后再镶嵌宝石。", prefix, slot);
    }

    let inv = get_gem_inventory(db, user_id);
    let found = inv
        .iter()
        .find(|(name, qty)| *qty > 0 && (name.contains(gem_query) || name == gem_query));

    let (gem_name, _gem_qty) = match found {
        Some((n, q)) => (n.clone(), *q),
        None => {
            return format!(
                "{}\n背包中没有「{}」宝石！\n使用「宝石背包」查看拥有的宝石。",
                prefix, gem_query
            );
        }
    };

    let mut gems = get_socketed_gems(db, user_id, slot);

    let empty_idx = gems.iter().position(|g| g.is_empty());
    let slot_idx = match empty_idx {
        Some(idx) => idx,
        None => {
            return format!(
                "{}\n{}的所有宝石孔位已满（{}/{}）！\n请先卸下已有宝石再镶嵌。",
                prefix,
                slot,
                gems.iter().filter(|g| !g.is_empty()).count(),
                MAX_SLOTS_PER_EQUIP
            );
        }
    };

    let parsed = parse_gem_name(&gem_name);
    if parsed.is_none() {
        return format!("{}\n无法识别宝石「{}」！", prefix, gem_name);
    }
    let (base_name, tier) = parsed.unwrap();

    if !remove_gem_from_inventory(db, user_id, &gem_name, 1) {
        return format!("{}\n{}数量不足！", prefix, gem_name);
    }

    gems[slot_idx] = gem_name.clone();
    set_socketed_gems(db, user_id, slot, &gems);

    let gt = GEM_TYPES.iter().find(|g| g.name == base_name).unwrap();
    let value = gem_attr_value(gt.base_value, tier);

    format!(
        "{}\n═══ 💎 镶嵌成功 ═══\n\n装备：[{}]\n槽位：{} 孔位{}\n宝石：{}\n属性：{} +{}\n\n💡 共 {}/{} 个孔位已使用\n使用「宝石属性」查看全部镶嵌加成",
        prefix,
        equip_name,
        slot,
        slot_idx + 1,
        gem_name,
        gt.attr_name,
        value,
        gems.iter().filter(|g| !g.is_empty()).count(),
        MAX_SLOTS_PER_EQUIP
    )
}

/// 卸下宝石 — 从装备上卸下宝石
pub fn cmd_unsocket_gem(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n用法：卸下宝石+槽位名\n示例：卸下宝石+武器\n\n使用「宝石属性」查看已镶嵌宝石",
            prefix
        );
    }

    let parts: Vec<&str> = args.split([' ', '+']).map(|s| s.trim()).collect();
    let slot = parts[0];
    let slot_num = if parts.len() > 1 {
        parts[1].parse::<usize>().ok()
    } else {
        None
    };

    let equip_name = db.read_equip_name(user_id, slot);
    if equip_name == "拳头" || equip_name.is_empty() {
        return format!("{}\n{}槽位没有装备！", prefix, slot);
    }

    let mut gems = get_socketed_gems(db, user_id, slot);
    let has_gems: Vec<usize> = gems
        .iter()
        .enumerate()
        .filter(|(_, g)| !g.is_empty())
        .map(|(i, _)| i)
        .collect();

    if has_gems.is_empty() {
        return format!("{}\n{}没有镶嵌任何宝石！", prefix, slot);
    }

    let mut removed = Vec::new();

    if let Some(idx) = slot_num {
        let real_idx = idx.saturating_sub(1);
        if real_idx >= MAX_SLOTS_PER_EQUIP || gems[real_idx].is_empty() {
            return format!("{}\n{}的孔位{}没有宝石！", prefix, slot, idx);
        }
        let gem_name = gems[real_idx].clone();
        add_gem_to_inventory(db, user_id, &gem_name, 1);
        removed.push(gem_name);
        gems[real_idx] = String::new();
    } else {
        for gem in gems.iter_mut().take(MAX_SLOTS_PER_EQUIP) {
            if !gem.is_empty() {
                let gem_name = gem.clone();
                add_gem_to_inventory(db, user_id, &gem_name, 1);
                removed.push(gem_name);
                *gem = String::new();
            }
        }
    }

    set_socketed_gems(db, user_id, slot, &gems);

    format!(
        "{}\n═══ 💎 卸下成功 ═══\n\n装备：[{}]\n卸下：{}\n\n💡 宝石已回到宝石背包",
        prefix,
        equip_name,
        removed.join("、")
    )
}

/// 合成宝石 — 3个同级同类型 → 1个高级
pub fn cmd_merge_gem(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再合成宝石！", prefix);
    }

    let gem_query = args.trim();
    if gem_query.is_empty() {
        return format!(
            "{}\n用法：合成宝石+宝石名\n需要 {} 个同级同类型宝石合成 1 个高级宝石\n\n💡 使用「宝石背包」查看拥有的宝石",
            prefix, MERGE_COST
        );
    }

    let inv = get_gem_inventory(db, user_id);
    let found = inv
        .iter()
        .find(|(name, qty)| *qty > 0 && (name.contains(gem_query) || name == gem_query));

    let (gem_name, gem_qty) = match found {
        Some((n, q)) => (n.clone(), *q),
        None => return format!("{}\n背包中没有「{}」宝石！", prefix, gem_query),
    };

    let (base_name, tier) = match parse_gem_name(&gem_name) {
        Some(parsed) => parsed,
        None => return format!("{}\n无法识别宝石「{}」！", prefix, gem_name),
    };

    let max_tier = GEM_TIERS.iter().map(|(_, t)| *t).max().unwrap_or(5);
    if tier >= max_tier {
        return format!("{}\n{}已是最高级宝石，无法继续合成！", prefix, gem_name);
    }

    if gem_qty < MERGE_COST {
        return format!(
            "{}\n{}数量不足！需要 {} 个，当前 {} 个。",
            prefix, gem_name, MERGE_COST, gem_qty
        );
    }

    let gold_cost = tier as i64 * 1000;
    let current_gold: i64 = db.read_basic(user_id, "Currency_gold").parse().unwrap_or(0);
    if current_gold < gold_cost {
        return format!(
            "{}\n金币不足！合成需要 {} 金币，当前 {}。",
            prefix, gold_cost, current_gold
        );
    }

    remove_gem_from_inventory(db, user_id, &gem_name, MERGE_COST);
    db.modify_currency(user_id, "Currency_gold", "sub", gold_cost);

    let next_tier = tier + 1;
    let new_gem_name = gem_full_name(base_name, next_tier);
    add_gem_to_inventory(db, user_id, &new_gem_name, 1);

    let gt = GEM_TYPES.iter().find(|g| g.name == base_name).unwrap();
    let old_value = gem_attr_value(gt.base_value, tier);
    let new_value = gem_attr_value(gt.base_value, next_tier);

    format!(
        "{}\n═══ 💎 合成成功 ═══\n\n消耗：{} ×{} + {}金币\n获得：{}\n属性：{} +{} → +{}\n\n💡 继续合成可获得更高级宝石！",
        prefix, gem_name, MERGE_COST, gold_cost, new_gem_name, gt.attr_name, old_value, new_value
    )
}

/// 宝石属性 — 查看所有装备上镶嵌宝石提供的总属性加成
pub fn cmd_gem_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let slots = ["武器", "头盔", "胸甲", "护肩", "腰带", "鞋子", "项链", "戒指", "称号"];

    let mut total_stats: Vec<(String, i32)> = Vec::new();
    let mut all_gems_detail = Vec::new();

    for slot in &slots {
        let equip_name = db.read_equip_name(user_id, slot);
        let gems = get_socketed_gems(db, user_id, slot);
        let has_gems: Vec<&String> = gems.iter().filter(|g| !g.is_empty()).collect();

        if !has_gems.is_empty() {
            let mut slot_detail = format!("  【{}】{}\n", slot, equip_name);
            for gem_name in &has_gems {
                if let Some((base_name, tier)) = parse_gem_name(gem_name) {
                    if let Some(gt) = GEM_TYPES.iter().find(|g| g.name == base_name) {
                        let value = gem_attr_value(gt.base_value, tier);
                        slot_detail.push_str(&format!("    💎 {} → {} +{}\n", gem_name, gt.attr_name, value));

                        if let Some(entry) = total_stats.iter_mut().find(|(n, _)| n == gt.attr_name) {
                            entry.1 += value;
                        } else {
                            total_stats.push((gt.attr_name.to_string(), value));
                        }
                    }
                }
            }
            all_gems_detail.push(slot_detail);
        }
    }

    let mut out = format!("{}\n═══ 💎 宝石属性总览 ═══\n", prefix);

    if all_gems_detail.is_empty() {
        out.push_str("\n当前没有任何装备镶嵌宝石！\n");
        out.push_str("💡 使用「镶嵌宝石+宝石名+槽位」镶嵌\n");
        out.push_str("💡 使用「查看宝石」了解宝石系统\n");
        return out;
    }

    out.push_str("\n📊 宝石总加成：\n");
    for (attr, value) in &total_stats {
        out.push_str(&format!("  {} +{}\n", attr, value));
    }

    out.push_str("\n📋 详细镶嵌：\n");
    for detail in &all_gems_detail {
        out.push_str(detail);
    }

    out
}

/// 掉落宝石（被战斗/Boss/副本系统调用）
/// 返回掉落的宝石名，如果没有掉落返回 None
#[allow(dead_code)]
pub fn try_gem_drop(db: &Database, user_id: &str, monster_level: i32, is_boss: bool) -> Option<String> {
    let mut rng = rand::thread_rng();

    let drop_rate = if is_boss { 0.15 } else { 0.03 };
    if rng.gen::<f64>() >= drop_rate {
        return None;
    }

    let tier = if is_boss {
        if monster_level >= 40 {
            3
        } else if monster_level >= 20 {
            2
        } else {
            1
        }
    } else if monster_level >= 30 {
        2
    } else {
        1
    };

    let gt = &GEM_TYPES[rng.gen_range(0..GEM_TYPES.len())];
    let gem_name = gem_full_name(gt.name, tier);

    add_gem_to_inventory(db, user_id, &gem_name, 1);
    Some(gem_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gem_full_name() {
        assert_eq!(gem_full_name("红宝石", 1), "初级红宝石×1");
        assert_eq!(gem_full_name("红宝石", 3), "高级红宝石×3");
        assert_eq!(gem_full_name("紫宝石", 5), "传说紫宝石×5");
    }

    #[test]
    fn test_parse_gem_name() {
        assert_eq!(parse_gem_name("初级红宝石×1"), Some(("红宝石", 1)));
        assert_eq!(parse_gem_name("高级蓝宝石×3"), Some(("蓝宝石", 3)));
        assert_eq!(parse_gem_name("传说黄宝石×5"), Some(("黄宝石", 5)));
        assert_eq!(parse_gem_name("普通石头"), None);
    }

    #[test]
    fn test_gem_attr_value() {
        assert_eq!(gem_attr_value(50, 1), 50);
        assert_eq!(gem_attr_value(50, 2), 200);
        assert_eq!(gem_attr_value(50, 3), 450);
        assert_eq!(gem_attr_value(10, 5), 250);
    }
}

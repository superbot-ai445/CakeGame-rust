/// 装备传承系统
/// 允许玩家将一件装备的强化等级传承到另一件装备上
/// 来源: 新增系统 — 扩展装备强化生态
///
/// 传承规则:
/// - 源装备的强化等级会转移到目标装备
/// - 传承后源装备强化等级归零
/// - 传承消耗金币（按强化等级递增）
/// - 高等级传承需要传承石道具
/// - 传承有成功率（等级越高成功率越低）
/// - 失败时源装备等级不降但消耗材料
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 传承石物品名
const INHERIT_STONE: &str = "传承石";

/// 解析装备名中的强化等级 (e.g. "铁剑(+5)" -> ("铁剑", 5))
fn parse_enhance_level(name: &str) -> (String, i32) {
    if let Some(start) = name.find("(+") {
        if let Some(end) = name[start..].find(')') {
            let base = name[..start].to_string();
            let level_str = &name[start + 2..start + end];
            if let Ok(level) = level_str.parse::<i32>() {
                return (base, level);
            }
        }
    }
    (name.to_string(), 0)
}

/// 计算传承消耗金币
fn inherit_cost(from_level: i32) -> i64 {
    match from_level {
        0 => 0,
        1..=3 => 500,
        4..=6 => 2000,
        7..=9 => 8000,
        10..=12 => 25000,
        13..=15 => 80000,
        _ => 200000,
    }
}

/// 计算传承所需传承石数量
fn inherit_stone_count(from_level: i32) -> i32 {
    match from_level {
        0..=3 => 0,
        4..=6 => 1,
        7..=9 => 2,
        10..=12 => 5,
        13..=15 => 10,
        _ => 20,
    }
}

/// 计算传承成功率
fn inherit_success_rate(from_level: i32) -> f64 {
    match from_level {
        0 => 1.0,
        1..=3 => 1.0,
        4..=6 => 0.95,
        7..=9 => 0.85,
        10..=12 => 0.70,
        13..=15 => 0.50,
        _ => 0.30,
    }
}

/// 简单哈希用于确定性随机
fn simple_hash(seed: u64) -> u64 {
    let mut h = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h
}

/// 查看传承 — 显示传承系统说明和当前可用装备
pub fn cmd_view_inherit(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let items = db.knapsack_all(user_id);

    let mut out = format!("{}\n═══ 🔄 装备传承系统 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    out.push_str("📖 传承可将一件装备的强化等级转移到另一件装备上\n\n");

    out.push_str("📋 【传承规则】\n");
    out.push_str("  • 源装备强化等级 → 目标装备\n");
    out.push_str("  • 传承后源装备强化等级归零\n");
    out.push_str("  • 等级≥4需要传承石\n");
    out.push_str("  • 高等级传承有失败风险\n\n");

    out.push_str("💰 【传承消耗】\n");
    out.push_str("  +1~+3: 💰500金币 | 100%成功率\n");
    out.push_str("  +4~+6: 💰2000金币 + 🔮传承石×1 | 95%成功率\n");
    out.push_str("  +7~+9: 💰8000金币 + 🔮传承石×2 | 85%成功率\n");
    out.push_str("  +10~+12: 💰25000金币 + 🔮传承石×5 | 70%成功率\n");
    out.push_str("  +13~+15: 💰80000金币 + 🔮传承石×10 | 50%成功率\n");
    out.push_str("  +16+: 💰200000金币 + 🔮传承石×20 | 30%成功率\n\n");

    // 显示有强化等级的装备
    let enhanced_items: Vec<_> = items.iter().filter(|i| parse_enhance_level(&i.name).1 > 0).collect();

    if enhanced_items.is_empty() {
        out.push_str("📦 背包中没有已强化的装备\n");
    } else {
        out.push_str("📦 【可作为传承源的装备】\n");
        for (i, item) in enhanced_items.iter().enumerate() {
            let (base, level) = parse_enhance_level(&item.name);
            out.push_str(&format!(
                "  {}. [{}] +{} (数量:{})\n",
                i + 1,
                base,
                level,
                item.quantity
            ));
        }
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「传承预览+源装备名+目标装备名」预览传承\n");
    out.push_str("💡 使用「装备传承+源装备名+目标装备名」执行传承\n");
    out
}

/// 传承预览 — 预览传承结果（不执行）
pub fn cmd_inherit_preview(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return format!(
            "{}\n⚠️ 请指定源装备和目标装备\n💡 用法：传承预览+源装备名+目标装备名\n📋 示例：传承预览+铁剑+钢剑",
            prefix
        );
    }

    let source_name = parts[0];
    let target_name = parts[1];

    // 查找源装备
    let items = db.knapsack_all(user_id);
    let source_item = items.iter().find(|i| {
        let (base, level) = parse_enhance_level(&i.name);
        (base == source_name || i.name == source_name) && level > 0
    });

    let source = match source_item {
        Some(item) => item,
        None => {
            return format!(
                "{}\n⚠️ 找不到已强化的装备 [{}]\n💡 请确保装备有强化等级(+N)",
                prefix, source_name
            );
        }
    };

    let (source_base, source_level) = parse_enhance_level(&source.name);

    // 查找目标装备（任何同名或含目标名的装备，包括有强化等级的）
    let target_item = items.iter().find(|i| {
        let (base, _) = parse_enhance_level(&i.name);
        (base == target_name || i.name.contains(target_name)) && i.name != source.name
    });

    let target = match target_item {
        Some(item) => item,
        None => {
            return format!(
                "{}\n⚠️ 找不到目标装备 [{}]\n💡 请确保背包中有该装备",
                prefix, target_name
            );
        }
    };

    let (target_base, target_level) = parse_enhance_level(&target.name);

    let cost = inherit_cost(source_level);
    let stones = inherit_stone_count(source_level);
    let rate = inherit_success_rate(source_level);

    let mut out = format!("{}\n═══ 🔄 传承预览 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    out.push_str(&format!("📤 源装备: [{}] +{}\n", source_base, source_level));
    out.push_str(&format!(
        "📥 目标装备: [{}] {}\n",
        target_base,
        if target_level > 0 {
            format!("+{}", target_level)
        } else {
            "未强化".to_string()
        }
    ));
    out.push_str("\n📊 【传承结果】\n");
    out.push_str(&format!("  传承后目标: [{}] +{}\n", target_base, source_level));
    out.push_str(&format!("  传承后源装备: [{}] +0 (等级归零)\n", source_base));

    if target_level > 0 {
        out.push_str(&format!("  ⚠️ 目标装备已有+{}，传承将覆盖\n", target_level));
    }

    out.push_str("\n💰 【传承消耗】\n");
    out.push_str(&format!("  金币: {}\n", cost));
    if stones > 0 {
        out.push_str(&format!("  传承石: ×{}\n", stones));
    }
    out.push_str(&format!("  成功率: {:.0}%\n", rate * 100.0));

    // 检查玩家资源
    let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
    let user_stones = db.get_item_count(user_id, INHERIT_STONE);
    let can_afford = user_gold >= cost && (stones == 0 || user_stones >= stones);

    if can_afford {
        out.push_str("\n✅ 资源充足，可以传承！\n");
    } else {
        out.push_str("\n❌ 资源不足：\n");
        if user_gold < cost {
            out.push_str(&format!("  金币: {}/{}\n", user_gold, cost));
        }
        if stones > 0 && user_stones < stones {
            out.push_str(&format!("  传承石: {}/{}\n", user_stones, stones));
        }
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 确认后使用「装备传承+源装备名+目标装备名」执行\n");
    out
}

/// 装备传承 — 执行传承操作
pub fn cmd_inherit(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return format!(
            "{}\n⚠️ 请指定源装备和目标装备\n💡 用法：装备传承+源装备名+目标装备名",
            prefix
        );
    }

    let source_name = parts[0];
    let target_name = parts[1];

    // 查找源装备
    let items = db.knapsack_all(user_id);
    let source_item = items.iter().find(|i| {
        let (base, level) = parse_enhance_level(&i.name);
        (base == source_name || i.name == source_name) && level > 0
    });

    let source = match source_item {
        Some(item) => item.clone(),
        None => {
            return format!("{}\n⚠️ 找不到已强化的装备 [{}]", prefix, source_name);
        }
    };

    let (source_base, source_level) = parse_enhance_level(&source.name);

    // 查找目标装备
    let target_item = items.iter().find(|i| {
        let (base, _) = parse_enhance_level(&i.name);
        (base == target_name || i.name.contains(target_name)) && i.name != source.name
    });

    let target = match target_item {
        Some(item) => item.clone(),
        None => {
            return format!("{}\n⚠️ 找不到目标装备 [{}]", prefix, target_name);
        }
    };

    let (target_base, target_level) = parse_enhance_level(&target.name);

    let cost = inherit_cost(source_level);
    let stones = inherit_stone_count(source_level);

    // 检查金币
    let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
    if user_gold < cost {
        return format!("{}\n⚠️ 金币不足！需要💰{}，当前💰{}", prefix, cost, user_gold);
    }

    // 检查传承石
    if stones > 0 {
        let user_stones = db.get_item_count(user_id, INHERIT_STONE);
        if user_stones < stones {
            return format!("{}\n⚠️ 传承石不足！需要🔮×{}，当前🔮×{}", prefix, stones, user_stones);
        }
    }

    // 扣除资源
    db.modify_currency(user_id, CURRENCY_GOLD, "sub", cost);
    if stones > 0 {
        db.remove_item(user_id, INHERIT_STONE, stones);
    }

    // 传承成功率判定
    let rate = inherit_success_rate(source_level);
    let date = chrono::Local::now().format("%Y%m%d%H%M").to_string();
    let seed_str = format!("{}{}{}{}", user_id, source_base, target_base, date);
    let hash = simple_hash(seed_str.len() as u64);
    let roll = (hash % 10000) as f64 / 10000.0;
    let success = roll < rate;

    if success {
        // 移除旧源装备
        db.remove_item(user_id, &source.name, 1);
        // 添加无强化的源装备
        db.add_item(user_id, &source_base, 1);

        // 移除旧目标装备
        db.remove_item(user_id, &target.name, 1);
        // 添加强化后的目标装备
        let new_target_name = format!("{}(+{})", target_base, source_level);
        db.add_item(user_id, &new_target_name, 1);

        format!(
            "{}\n🎉 传承成功！\n\n📤 源装备: [{}] +{} → +0\n📥 目标装备: [{}] {} → +{}\n💰 消耗: {}金币{}\n🎲 成功率: {:.0}%",
            prefix,
            source_base, source_level,
            target_base,
            if target_level > 0 { format!("+{}", target_level) } else { "未强化".to_string() },
            source_level,
            cost,
            if stones > 0 { format!(" + 🔮传承石×{}", stones) } else { String::new() },
            rate * 100.0
        )
    } else {
        format!(
            "{}\n💥 传承失败！\n\n📤 源装备: [{}] +{} (等级保留)\n📥 目标装备: [{}] (无变化)\n💰 已消耗: {}金币{}\n🎲 成功率: {:.0}% (未命中)\n💡 源装备等级未受影响，可再次尝试",
            prefix,
            source_base, source_level,
            target_base,
            cost,
            if stones > 0 { format!(" + 🔮传承石×{}", stones) } else { String::new() },
            rate * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enhance_level() {
        assert_eq!(parse_enhance_level("铁剑(+5)"), ("铁剑".to_string(), 5));
        assert_eq!(parse_enhance_level("钢剑(+12)"), ("钢剑".to_string(), 12));
        assert_eq!(parse_enhance_level("木棍"), ("木棍".to_string(), 0));
        assert_eq!(
            parse_enhance_level("【史诗】屠龙刀(+15)"),
            ("【史诗】屠龙刀".to_string(), 15)
        );
        assert_eq!(parse_enhance_level("铁剑(+)"), ("铁剑(+)".to_string(), 0));
    }

    #[test]
    fn test_inherit_cost_increases() {
        assert_eq!(inherit_cost(0), 0);
        assert!(inherit_cost(1) < inherit_cost(4));
        assert!(inherit_cost(4) < inherit_cost(7));
        assert!(inherit_cost(7) < inherit_cost(10));
        assert!(inherit_cost(10) < inherit_cost(13));
        assert!(inherit_cost(13) < inherit_cost(16));
    }

    #[test]
    fn test_inherit_stone_count() {
        assert_eq!(inherit_stone_count(0), 0);
        assert_eq!(inherit_stone_count(3), 0);
        assert_eq!(inherit_stone_count(4), 1);
        assert_eq!(inherit_stone_count(7), 2);
        assert_eq!(inherit_stone_count(10), 5);
        assert_eq!(inherit_stone_count(15), 10);
    }

    #[test]
    fn test_inherit_success_rate() {
        assert!((inherit_success_rate(1) - 1.0).abs() < 0.01);
        assert!((inherit_success_rate(5) - 0.95).abs() < 0.01);
        assert!((inherit_success_rate(8) - 0.85).abs() < 0.01);
        assert!((inherit_success_rate(11) - 0.70).abs() < 0.01);
        assert!((inherit_success_rate(14) - 0.50).abs() < 0.01);
        assert!((inherit_success_rate(20) - 0.30).abs() < 0.01);
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash(42);
        let h2 = simple_hash(42);
        assert_eq!(h1, h2);
    }
}

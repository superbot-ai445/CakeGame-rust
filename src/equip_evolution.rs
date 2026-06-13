/// CakeGame 装备进化系统
/// 允许玩家使用进化石将装备进化到更高品阶，获得30%属性加成
/// 每件装备最多可进化3次: 初级→中级→高级→传说
/// 存储: Global 表, section = 'equip_evolution', ID = user_id, DATA = 各槽位进化次数JSON
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 装备槽位列表
const EVOLUTION_SLOTS: &[(&str, &str)] = &[
    ("武器", SLOT_WEAPON),
    ("头盔", SLOT_HELMET),
    ("铠甲", SLOT_ARMOR),
    ("护腿", SLOT_LEG),
    ("靴子", SLOT_BOOTS),
    ("项链", SLOT_NECKLACE),
    ("戒指", SLOT_RING),
    ("翅膀", SLOT_WING),
    ("时装", SLOT_FASHION),
    ("称号", SLOT_TITLE),
];

/// 进化阶段定义
struct EvolutionTier {
    tier: i32,
    name: &'static str,
    cost_gold: i64,
    cost_stones: i32,
    stat_bonus_pct: f64,
}

const EVOLUTION_TIERS: &[EvolutionTier] = &[
    EvolutionTier {
        tier: 1,
        name: "初级进化",
        cost_gold: 5_000,
        cost_stones: 1,
        stat_bonus_pct: 0.30,
    },
    EvolutionTier {
        tier: 2,
        name: "中级进化",
        cost_gold: 20_000,
        cost_stones: 3,
        stat_bonus_pct: 0.60,
    },
    EvolutionTier {
        tier: 3,
        name: "高级进化",
        cost_gold: 100_000,
        cost_stones: 5,
        stat_bonus_pct: 0.90,
    },
];

/// 进化石名称
const EVOLUTION_STONE_NAME: &str = "进化石";

/// 进化阶段对应品质名
const TIER_QUALITY_NAMES: &[&str] = &["原始", "初级", "中级", "高级", "传说"];

/// 从 Global 表读取用户各槽位进化次数 (JSON: {"武器":1,"头盔":0,...})
fn get_evolution_data(db: &Database, user_id: &str) -> std::collections::HashMap<String, i32> {
    let data = db.global_get("equip_evolution", user_id);
    if data.is_empty() {
        return std::collections::HashMap::new();
    }
    serde_json::from_str(&data).unwrap_or_default()
}

/// 保存进化数据到 Global 表
fn save_evolution_data(db: &Database, user_id: &str, data: &std::collections::HashMap<String, i32>) {
    let json = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    db.global_set("equip_evolution", user_id, &json);
}

/// 解析槽位名 → 内部 slot key
fn resolve_slot(name: &str) -> Option<&'static str> {
    let trimmed = name.trim();
    for (display, key) in EVOLUTION_SLOTS {
        if display == &trimmed || key == &trimmed {
            return Some(key);
        }
    }
    None
}

/// 获取槽位的显示名
#[allow(dead_code)]
fn slot_display_name(key: &str) -> &str {
    for (display, k) in EVOLUTION_SLOTS {
        if k == &key {
            return display;
        }
    }
    key
}

/// 计算进化后属性加成百分比
#[allow(dead_code)]
fn evolution_bonus_for_tier(tier: i32) -> f64 {
    match tier {
        0 => 0.0,
        1 => 0.30,
        2 => 0.60,
        3 => 0.90,
        _ => 0.90,
    }
}

/// 获取进化阶段品质名
fn tier_quality_name(tier: i32) -> &'static str {
    let idx = tier.clamp(0, 4) as usize;
    TIER_QUALITY_NAMES[idx]
}

/// 查看进化 — 显示进化系统信息、当前各槽位进化状态、进化路径
pub fn cmd_view_evolution(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let evo_data = get_evolution_data(db, user_id);
    let equips = db.equip_all(user_id);

    let mut out = format!("{}\n✨ ══════ 装备进化系统 ══════ ✨", prefix);
    out.push_str("\n\n进化可以提升装备所有属性，每次进化+30%");
    out.push_str("\n每件装备最多进化3次: 初级→中级→高级→传说\n");

    out.push_str("\n📋 【当前装备进化状态】\n");
    let mut has_any = false;
    for (display, key) in EVOLUTION_SLOTS {
        let tier = evo_data.get(*key).copied().unwrap_or(0);
        let equip = equips.iter().find(|e| e.slot == *key);
        let equip_name = equip.map(|e| e.name.as_str()).unwrap_or("（空）");
        let quality = tier_quality_name(tier);

        out.push_str(&format!(
            "\n  {} [{}] — {} (进化{}次)",
            display, equip_name, quality, tier
        ));
        if tier < 3 {
            if let Some(next) = EVOLUTION_TIERS.iter().find(|t| t.tier == tier + 1) {
                out.push_str(&format!(
                    " → 下级: 💰{}g + 🪨{}个进化石",
                    next.cost_gold, next.cost_stones
                ));
            }
        } else {
            out.push_str(" ✅ 已满级");
        }
        has_any = true;
    }

    if !has_any {
        out.push_str("\n  暂无装备信息");
    }

    out.push_str("\n\n📊 【进化路径一览】");
    for tier_def in EVOLUTION_TIERS {
        out.push_str(&format!(
            "\n  {} → +{:.0}%属性 | 💰{}g + 🪨{}个进化石",
            tier_def.name,
            tier_def.stat_bonus_pct * 100.0,
            tier_def.cost_gold,
            tier_def.cost_stones
        ));
    }

    out.push_str("\n\n━━━━━━━━━━━━━━━━━━━━");
    out.push_str("\n💡 使用「装备进化+槽位名」进化装备");
    out.push_str("\n💡 使用「进化预览+槽位名」预览进化效果");
    out.push_str("\n💡 可选槽位: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号");
    out
}

/// 进化预览 — 预览指定槽位装备进化后的属性变化
pub fn cmd_evolution_preview(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let slot_input = args.trim();

    if slot_input.is_empty() {
        return format!(
            "{}\n⚠️ 请指定槽位名，例: 进化预览+武器\n💡 可选: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
            prefix
        );
    }

    let slot_key = match resolve_slot(slot_input) {
        Some(k) => k,
        None => {
            return format!(
                "{}\n❌ 未找到槽位「{}」，可选: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix, slot_input
            );
        }
    };

    let equip = match db.equip_read(user_id, slot_key) {
        Some(e) => e,
        None => return format!("{}❌ {} 槽位没有装备，无法预览进化效果。", prefix, slot_input),
    };

    let evo_data = get_evolution_data(db, user_id);
    let current_tier = evo_data.get(slot_key).copied().unwrap_or(0);

    if current_tier >= 3 {
        return format!(
            "{}\n✅ {} [{}] 已达到传说品质，无法继续进化！",
            prefix, slot_input, equip.name
        );
    }

    let next_tier = current_tier + 1;
    let tier_def = match EVOLUTION_TIERS.iter().find(|t| t.tier == next_tier) {
        Some(t) => t,
        None => return format!("{}\n❌ 进化数据异常", prefix),
    };

    let bonus_pct = 0.30_f64;
    let current_quality = tier_quality_name(current_tier);
    let next_quality = tier_quality_name(next_tier);

    let mut out = format!("{}\n🔮 ══════ 进化预览 ══════ 🔮", prefix);
    out.push_str(&format!("\n\n🗡 槽位: {} | 装备: [{}]", slot_input, equip.name));
    out.push_str(&format!(
        "\n📊 {} → {} (+{:.0}%全属性)",
        current_quality,
        next_quality,
        bonus_pct * 100.0
    ));

    out.push_str("\n\n📈 【属性变化预览】\n");

    let attrs: Vec<(&str, i32)> = vec![
        ("生命", equip.add_hp),
        ("魔法", equip.add_mp),
        ("物攻", equip.add_ad),
        ("魔攻", equip.add_ap),
        ("防御", equip.add_defense),
        ("魔抗", equip.add_magic),
        ("命中", equip.add_hit),
        ("闪避", equip.add_dodge),
        ("暴击", equip.add_crit),
        ("吸血", equip.add_absorb_hp),
        ("物穿", equip.add_adptv),
        ("法穿", equip.add_apptv),
        ("免伤", equip.add_immune_damage),
    ];

    for (name, val) in &attrs {
        if *val > 0 {
            let new_val = ((*val as f64) * (1.0 + bonus_pct)).floor() as i32;
            out.push_str(&format!("  {}: {} → {} (+{})\n", name, val, new_val, new_val - val));
        }
    }

    if attrs.iter().all(|(_, v)| *v <= 0) {
        out.push_str("  该装备暂无属性加成\n");
    }

    out.push_str(&format!(
        "\n💰 消耗: {}金币 + {}个进化石",
        tier_def.cost_gold, tier_def.cost_stones
    ));
    out.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    out.push_str("\n💡 使用「装备进化+槽位名」执行进化");
    out
}

/// 装备进化 — 对指定槽位的装备执行进化
pub fn cmd_evolve_equipment(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let slot_input = args.trim();

    if slot_input.is_empty() {
        return format!(
            "{}\n⚠️ 请指定槽位名，例: 装备进化+武器\n💡 可选: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
            prefix
        );
    }

    let slot_key = match resolve_slot(slot_input) {
        Some(k) => k,
        None => {
            return format!(
                "{}\n❌ 未找到槽位「{}」，可选: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix, slot_input
            );
        }
    };

    // 检查是否有装备
    let equip = match db.equip_read(user_id, slot_key) {
        Some(e) => e,
        None => return format!("{}❌ {} 槽位没有装备，无法进化。", prefix, slot_input),
    };

    let mut evo_data = get_evolution_data(db, user_id);
    let current_tier = evo_data.get(slot_key).copied().unwrap_or(0);

    if current_tier >= 3 {
        return format!(
            "{}\n✅ {} [{}] 已达到传说品质，无法继续进化！",
            prefix, slot_input, equip.name
        );
    }

    let next_tier = current_tier + 1;
    let tier_def = match EVOLUTION_TIERS.iter().find(|t| t.tier == next_tier) {
        Some(t) => t,
        None => return format!("{}\n❌ 进化数据异常", prefix),
    };

    // 检查进化石数量
    let stone_count = db.knapsack_quantity(user_id, EVOLUTION_STONE_NAME);
    if stone_count < tier_def.cost_stones {
        return format!(
            "{}\n⚠️ 进化石不足！需要{}个，当前{}个\n💡 进化石可通过副本掉落或商店购买获得",
            prefix, tier_def.cost_stones, stone_count
        );
    }

    // 检查并扣除金币
    let current_gold = db.read_currency(user_id, CURRENCY_GOLD);
    if current_gold < tier_def.cost_gold {
        return format!(
            "{}\n⚠️ 金币不足！需要💰{}，当前💰{}",
            prefix, tier_def.cost_gold, current_gold
        );
    }
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, tier_def.cost_gold);

    // 扣除进化石
    if !db.knapsack_remove(user_id, EVOLUTION_STONE_NAME, tier_def.cost_stones) {
        // 回滚金币
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, tier_def.cost_gold);
        return format!("{}\n❌ 扣除进化石失败，请稍后重试", prefix);
    }

    // 更新进化数据
    evo_data.insert(slot_key.to_string(), next_tier);
    save_evolution_data(db, user_id, &evo_data);

    // 计算进化后属性提升
    let current_quality = tier_quality_name(current_tier);
    let next_quality = tier_quality_name(next_tier);
    let bonus_pct = 0.30_f64;

    let mut out = format!("{}\n🎉 ══════ 装备进化成功！ ══════ 🎉", prefix);
    out.push_str(&format!("\n\n🗡 槽位: {} | 装备: [{}]", slot_input, equip.name));
    out.push_str(&format!(
        "\n📊 品质: {} → {} (+{:.0}%全属性)",
        current_quality,
        next_quality,
        bonus_pct * 100.0
    ));

    out.push_str("\n\n📈 【属性提升】\n");
    let attrs: Vec<(&str, i32)> = vec![
        ("生命", equip.add_hp),
        ("魔法", equip.add_mp),
        ("物攻", equip.add_ad),
        ("魔攻", equip.add_ap),
        ("防御", equip.add_defense),
        ("魔抗", equip.add_magic),
        ("命中", equip.add_hit),
        ("闪避", equip.add_dodge),
        ("暴击", equip.add_crit),
        ("吸血", equip.add_absorb_hp),
        ("物穿", equip.add_adptv),
        ("法穿", equip.add_apptv),
        ("免伤", equip.add_immune_damage),
    ];

    for (name, val) in &attrs {
        if *val > 0 {
            let new_val = ((*val as f64) * (1.0 + bonus_pct)).floor() as i32;
            out.push_str(&format!("  {}: {} → {} (+{})\n", name, val, new_val, new_val - val));
        }
    }

    out.push_str(&format!(
        "\n💰 消耗: {}金币 + {}个进化石",
        tier_def.cost_gold, tier_def.cost_stones
    ));

    if next_tier >= 3 {
        out.push_str("\n\n🏆 恭喜！该装备已达到传说品质（最高级）！");
    } else {
        out.push_str(&format!(
            "\n💡 下次进化: {} → +{:.0}%属性",
            EVOLUTION_TIERS
                .iter()
                .find(|t| t.tier == next_tier + 1)
                .map(|t| t.name)
                .unwrap_or("未知"),
            bonus_pct * 100.0
        ));
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_slot_valid() {
        assert_eq!(resolve_slot("武器"), Some(SLOT_WEAPON));
        assert_eq!(resolve_slot("头盔"), Some(SLOT_HELMET));
        assert_eq!(resolve_slot("铠甲"), Some(SLOT_ARMOR));
        assert_eq!(resolve_slot("护腿"), Some(SLOT_LEG));
        assert_eq!(resolve_slot("靴子"), Some(SLOT_BOOTS));
        assert_eq!(resolve_slot("项链"), Some(SLOT_NECKLACE));
        assert_eq!(resolve_slot("戒指"), Some(SLOT_RING));
        assert_eq!(resolve_slot("翅膀"), Some(SLOT_WING));
        assert_eq!(resolve_slot("时装"), Some(SLOT_FASHION));
        assert_eq!(resolve_slot("称号"), Some(SLOT_TITLE));
    }

    #[test]
    fn test_resolve_slot_invalid() {
        assert_eq!(resolve_slot("不存在"), None);
        assert_eq!(resolve_slot(""), None);
        // Trailing space is trimmed, so it still matches
        assert_eq!(resolve_slot("翅膀 "), Some(SLOT_WING));
    }

    #[test]
    fn test_evolution_tiers_count() {
        assert_eq!(EVOLUTION_TIERS.len(), 3);
        assert_eq!(EVOLUTION_TIERS[0].tier, 1);
        assert_eq!(EVOLUTION_TIERS[1].tier, 2);
        assert_eq!(EVOLUTION_TIERS[2].tier, 3);
    }

    #[test]
    fn test_evolution_tier_costs() {
        assert_eq!(EVOLUTION_TIERS[0].cost_gold, 5_000);
        assert_eq!(EVOLUTION_TIERS[0].cost_stones, 1);
        assert_eq!(EVOLUTION_TIERS[1].cost_gold, 20_000);
        assert_eq!(EVOLUTION_TIERS[1].cost_stones, 3);
        assert_eq!(EVOLUTION_TIERS[2].cost_gold, 100_000);
        assert_eq!(EVOLUTION_TIERS[2].cost_stones, 5);
    }

    #[test]
    fn test_evolution_bonus_for_tier() {
        assert!((evolution_bonus_for_tier(0) - 0.0).abs() < f64::EPSILON);
        assert!((evolution_bonus_for_tier(1) - 0.30).abs() < f64::EPSILON);
        assert!((evolution_bonus_for_tier(2) - 0.60).abs() < f64::EPSILON);
        assert!((evolution_bonus_for_tier(3) - 0.90).abs() < f64::EPSILON);
        // 超过3级应返回最大值
        assert!((evolution_bonus_for_tier(4) - 0.90).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tier_quality_names() {
        assert_eq!(tier_quality_name(0), "原始");
        assert_eq!(tier_quality_name(1), "初级");
        assert_eq!(tier_quality_name(2), "中级");
        assert_eq!(tier_quality_name(3), "高级");
        assert_eq!(tier_quality_name(4), "传说");
    }

    #[test]
    fn test_slot_display_name() {
        assert_eq!(slot_display_name(SLOT_WEAPON), "武器");
        assert_eq!(slot_display_name(SLOT_HELMET), "头盔");
        assert_eq!(slot_display_name(SLOT_ARMOR), "铠甲");
        assert_eq!(slot_display_name("unknown"), "unknown");
    }

    #[test]
    fn test_evolution_slots_count() {
        assert_eq!(EVOLUTION_SLOTS.len(), 10);
    }

    #[test]
    fn test_evolution_slots_unique_keys() {
        let mut keys: Vec<&str> = EVOLUTION_SLOTS.iter().map(|(_, k)| *k).collect();
        let original_len = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), original_len, "evolution slot keys must be unique");
    }
}

/// CakeGame 装备觉醒系统
///
/// 装备进化到最高阶(传说)后，可进行觉醒，获得独特被动效果。
/// 每件装备只能觉醒一次，觉醒后显示 ✨ 标记。
/// 觉醒消耗觉醒之石 + 大量金币，成功率60%。
/// 失败不损失装备，但消耗材料。
///
/// 数据存储: Global 表 SECTION='equip_awakening_{user_id}'
///
/// 指令: 查看觉醒, 觉醒预览, 装备觉醒, 觉醒排行, 觉醒图鉴
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD};
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 觉醒等级定义
#[allow(dead_code)]
struct AwakeningEffect {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    emoji: &'static str,
    /// 触发概率 (万分比)
    trigger_rate: i32,
    /// 效果类型: damage/heal/shield/buff/debuff
    effect_type: &'static str,
    /// 效果数值 (百分比或固定值)
    effect_value: i32,
    /// 稀有度权重 (越高越容易获得)
    weight: u32,
}

const AWAKENING_EFFECTS: &[AwakeningEffect] = &[
    AwakeningEffect {
        id: "lifesteal_burst",
        name: "嗜血狂潮",
        desc: "攻击时15%概率触发，吸取造成伤害20%的生命",
        emoji: "🩸",
        trigger_rate: 1500,
        effect_type: "lifesteal",
        effect_value: 20,
        weight: 20,
    },
    AwakeningEffect {
        id: "crit_surge",
        name: "暴怒之心",
        desc: "攻击时12%概率触发，暴击率+50%持续3回合",
        emoji: "💢",
        trigger_rate: 1200,
        effect_type: "buff_crit",
        effect_value: 50,
        weight: 18,
    },
    AwakeningEffect {
        id: "shield_aura",
        name: "神圣护盾",
        desc: "受击时10%概率触发，生成最大HP15%的护盾",
        emoji: "🛡️",
        trigger_rate: 1000,
        effect_type: "shield",
        effect_value: 15,
        weight: 15,
    },
    AwakeningEffect {
        id: "thunder_strike",
        name: "雷霆一击",
        desc: "攻击时8%概率触发，造成双倍伤害",
        emoji: "⚡",
        trigger_rate: 800,
        effect_type: "damage",
        effect_value: 200,
        weight: 12,
    },
    AwakeningEffect {
        id: "frost_armor",
        name: "寒冰铠甲",
        desc: "受击时15%概率触发，降低攻击者30%攻击力2回合",
        emoji: "🧊",
        trigger_rate: 1500,
        effect_type: "debuff_atk",
        effect_value: 30,
        weight: 20,
    },
    AwakeningEffect {
        id: "phoenix_rebirth",
        name: "凤凰涅槃",
        desc: "死亡时5%概率触发，复活并恢复30%HP(每场战斗1次)",
        emoji: "🔥",
        trigger_rate: 500,
        effect_type: "revive",
        effect_value: 30,
        weight: 5,
    },
    AwakeningEffect {
        id: "void_penetrate",
        name: "虚空穿透",
        desc: "攻击时10%概率触发，无视目标50%防御",
        emoji: "🌀",
        trigger_rate: 1000,
        effect_type: "penetrate",
        effect_value: 50,
        weight: 15,
    },
    AwakeningEffect {
        id: "blessing_light",
        name: "圣光祝福",
        desc: "每回合开始8%概率触发，回复最大HP10%",
        emoji: "✨",
        trigger_rate: 800,
        effect_type: "heal",
        effect_value: 10,
        weight: 15,
    },
];

/// 觉醒费用
const AWAKENING_COST_GOLD: i64 = 50_000;
const AWAKENING_COST_DIAMOND: i32 = 100;
const AWAKENING_ITEM: &str = "觉醒之石";
const AWAKENING_ITEM_QTY: i32 = 3;
const AWAKENING_SUCCESS_RATE: f64 = 0.60;

/// 觉醒需要的最低进化等级(传说=3)
const MIN_EVOLUTION_TIER: i32 = 3;

/// 装备槽位列表
const EQUIP_SLOTS: &[(&str, &str)] = &[
    ("weapon", "⚔️武器"),
    ("helmet", "🪖头盔"),
    ("armor", "🛡️铠甲"),
    ("leggings", "👖护腿"),
    ("boots", "👢靴子"),
    ("necklace", "📿项链"),
    ("ring", "💍戒指"),
    ("wings", "🪽翅膀"),
    ("fashion", "👔时装"),
    ("title", "🏷️称号"),
];

/// 解析槽位名
fn resolve_slot(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return None;
    }
    for &(key, display) in EQUIP_SLOTS {
        if name == key || display.contains(name) {
            return Some(key);
        }
    }
    // Fuzzy: extract Chinese text from display names for matching
    for &(key, display) in EQUIP_SLOTS {
        let text: String = display.chars().filter(|c| !c.is_ascii() && *c != '️').collect();
        if text.len() >= 2 && (text.contains(name) || name.contains(&text)) {
            return Some(key);
        }
    }
    None
}

/// 获取槽位显示名
fn slot_display(key: &str) -> &'static str {
    EQUIP_SLOTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, d)| *d)
        .unwrap_or("❓未知")
}

/// 检查装备是否已觉醒
fn is_awakened(db: &Database, user_id: &str, slot: &str) -> bool {
    let section = format!("equip_awakening_{}", user_id);
    !db.global_get(&section, &format!("aw_{}", slot)).is_empty()
}

/// 获取觉醒效果ID
fn get_awakening_effect(db: &Database, user_id: &str, slot: &str) -> Option<&'static AwakeningEffect> {
    let section = format!("equip_awakening_{}", user_id);
    let effect_id = db.global_get(&section, &format!("aw_{}", slot));
    if effect_id.is_empty() {
        return None;
    }
    AWAKENING_EFFECTS.iter().find(|e| e.id == effect_id)
}

/// 确定性随机选择觉醒效果(基于用户ID+槽位+日期)
fn select_awakening_effect(user_id: &str, slot: &str) -> &'static AwakeningEffect {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let mut hasher = DefaultHasher::new();
    format!("awakening_{}_{}_{}", user_id, slot, today).hash(&mut hasher);
    let hash = hasher.finish();

    let total_weight: u32 = AWAKENING_EFFECTS.iter().map(|e| e.weight).sum();
    let bucket = (hash % total_weight as u64) as u32;
    let mut acc = 0u32;
    for effect in AWAKENING_EFFECTS {
        acc += effect.weight;
        if bucket < acc {
            return effect;
        }
    }
    AWAKENING_EFFECTS.last().unwrap()
}

/// 千分位格式化
fn format_num(n: i64) -> String {
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

/// 查看觉醒 — 查看当前装备的觉醒状态
pub fn cmd_view_awakening(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let section = format!("equip_awakening_{}", user_id);
    let mut out = format!("{}\n═══ ✨ 装备觉醒系统 ═══", prefix);

    let mut awakened_count = 0;
    let mut total_slots = 0;

    for &(slot, display) in EQUIP_SLOTS {
        total_slots += 1;
        let effect_id = db.global_get(&section, &format!("aw_{}", slot));
        if effect_id.is_empty() {
            out.push_str(&format!("\n  {} — 未觉醒", display));
        } else {
            awakened_count += 1;
            let effect = AWAKENING_EFFECTS.iter().find(|e| e.id == effect_id);
            if let Some(eff) = effect {
                out.push_str(&format!("\n  {} ✨ {} {}", display, eff.emoji, eff.name));
            } else {
                out.push_str(&format!("\n  {} ✨ 已觉醒({})", display, effect_id));
            }
        }
    }

    out.push_str(&format!(
        "\n\n📊 觉醒进度: {}/{} ({}%)",
        awakened_count,
        total_slots,
        if total_slots > 0 {
            awakened_count * 100 / total_slots
        } else {
            0
        }
    ));
    out.push_str(&format!(
        "\n💡 消耗: {}金币 + {}钻石 + {}×{}",
        format_num(AWAKENING_COST_GOLD),
        AWAKENING_COST_DIAMOND,
        AWAKENING_ITEM_QTY,
        AWAKENING_ITEM
    ));
    out.push_str(&format!("\n🎲 成功率: {}%", (AWAKENING_SUCCESS_RATE * 100.0) as i32));
    out.push_str("\n📖 发送'觉醒预览+槽位名'查看可获得的效果");
    out.push_str("\n📖 发送'装备觉醒+槽位名'进行觉醒");

    out
}

/// 觉醒预览 — 预览某槽位觉醒后可获得的效果
pub fn cmd_awakening_preview(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let slot_name = args.trim();
    if slot_name.is_empty() {
        let mut out = format!("{}\n═══ ✨ 觉醒预览 ═══\n\n可用槽位:", prefix);
        for &(slot, display) in EQUIP_SLOTS {
            let status = if is_awakened(db, user_id, slot) {
                " ✨已觉醒"
            } else {
                ""
            };
            out.push_str(&format!("\n  {} — {}{}", slot, display, status));
        }
        out.push_str("\n\n📖 发送'觉醒预览+槽位名'查看具体效果");
        return out;
    }

    let slot = match resolve_slot(slot_name) {
        Some(s) => s,
        None => {
            return format!(
                "{}\n❌ 未找到槽位「{}」\n可用: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix, slot_name
            )
        }
    };

    if is_awakened(db, user_id, slot) {
        let effect = get_awakening_effect(db, user_id, slot);
        let eff_name = effect.map(|e| e.name).unwrap_or("未知");
        return format!(
            "{}\n{} 已觉醒 ✨\n效果: {}\n无法重复觉醒",
            prefix,
            slot_display(slot),
            eff_name
        );
    }

    let mut out = format!("{}\n═══ ✨ 觉醒预览 — {} ═══", prefix, slot_display(slot));
    out.push_str("\n\n🎲 可能获得的觉醒效果:");

    for effect in AWAKENING_EFFECTS {
        let prob = effect.weight as f64 / AWAKENING_EFFECTS.iter().map(|e| e.weight).sum::<u32>() as f64 * 100.0;
        out.push_str(&format!(
            "\n  {} {} — {} (概率{:.0}%)",
            effect.emoji, effect.name, effect.desc, prob
        ));
    }

    out.push_str(&format!(
        "\n\n💰 消耗: {}金币 + {}钻石 + {}×{}",
        format_num(AWAKENING_COST_GOLD),
        AWAKENING_COST_DIAMOND,
        AWAKENING_ITEM_QTY,
        AWAKENING_ITEM
    ));
    out.push_str(&format!("\n🎲 成功率: {}%", (AWAKENING_SUCCESS_RATE * 100.0) as i32));
    out.push_str("\n💡 觉醒效果随机获得，失败不损失装备但消耗材料");
    out.push_str("\n💡 觉醒后效果永久绑定该槽位");

    out
}

/// 装备觉醒 — 对指定槽位的装备进行觉醒
pub fn cmd_awaken_equipment(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let slot_name = args.trim();
    if slot_name.is_empty() {
        return format!(
            "{}\n📖 请指定槽位名\n发送'装备觉醒+槽位名'\n可用: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
            prefix
        );
    }

    let slot = match resolve_slot(slot_name) {
        Some(s) => s,
        None => {
            return format!(
                "{}\n❌ 未找到槽位「{}」\n可用: 武器/头盔/铠甲/护腿/靴子/项链/戒指/翅膀/时装/称号",
                prefix, slot_name
            )
        }
    };

    // 检查是否已觉醒
    if is_awakened(db, user_id, slot) {
        let effect = get_awakening_effect(db, user_id, slot);
        let eff_name = effect.map(|e| e.name).unwrap_or("未知");
        return format!("{}\n{} 已觉醒 ✨{}！无法重复觉醒", prefix, slot_display(slot), eff_name);
    }

    // 检查进化等级 (需要传说级=3)
    let evo_section = format!("equip_evolution_{}", user_id);
    let evo_level: i32 = db
        .global_get(&evo_section, &format!("evo_{}", slot))
        .parse()
        .unwrap_or(0);
    if evo_level < MIN_EVOLUTION_TIER {
        return format!(
            "{}\n❌ {}进化等级不足！\n当前: {}阶 | 需要: {}阶(传说)\n请先完成装备进化再来觉醒",
            prefix,
            slot_display(slot),
            evo_level,
            MIN_EVOLUTION_TIER
        );
    }

    // 检查虚弱状态
    let weak_remaining = user::check_weakness(db, user_id);
    if weak_remaining > 0 {
        return format!("{}\n操作失败！您正处于虚弱状态（剩余{}秒）", prefix, weak_remaining);
    }

    // 检查觉醒之石
    let stone_count = db.get_item_count(user_id, AWAKENING_ITEM);
    if stone_count < AWAKENING_ITEM_QTY {
        return format!(
            "{}\n❌ 觉醒之石不足！\n需要: {}个 | 当前: {}个\n💡 觉醒之石可通过副本/BOSS掉落获得",
            prefix, AWAKENING_ITEM_QTY, stone_count
        );
    }

    // 检查金币
    let gold_balance = db.modify_currency(user_id, CURRENCY_GOLD, "add", 0);
    if gold_balance < AWAKENING_COST_GOLD {
        return format!(
            "{}\n❌ 金币不足！\n需要: {} | 当前: {}",
            prefix,
            format_num(AWAKENING_COST_GOLD),
            format_num(gold_balance)
        );
    }

    // 检查钻石
    let diamond_balance = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", 0);
    if diamond_balance < AWAKENING_COST_DIAMOND as i64 {
        return format!(
            "{}\n❌ 钻石不足！\n需要: {} | 当前: {}",
            prefix, AWAKENING_COST_DIAMOND, diamond_balance
        );
    }

    // 扣除材料
    db.remove_item(user_id, AWAKENING_ITEM, AWAKENING_ITEM_QTY);
    let after_gold = db.modify_currency(user_id, CURRENCY_GOLD, "sub", AWAKENING_COST_GOLD);
    if after_gold < 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", AWAKENING_COST_GOLD);
        db.add_item(user_id, AWAKENING_ITEM, AWAKENING_ITEM_QTY);
        return format!("{}\n❌ 扣除金币失败", prefix);
    }
    let after_diamond = db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", AWAKENING_COST_DIAMOND as i64);
    if after_diamond < 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", AWAKENING_COST_DIAMOND as i64);
        db.modify_currency(user_id, CURRENCY_GOLD, "add", AWAKENING_COST_GOLD);
        db.add_item(user_id, AWAKENING_ITEM, AWAKENING_ITEM_QTY);
        return format!("{}\n❌ 扣除钻石失败", prefix);
    }

    // 判定成功/失败
    let mut hasher = DefaultHasher::new();
    let now_ts = chrono::Local::now().timestamp();
    format!("{}_{}_{}", user_id, slot, now_ts).hash(&mut hasher);
    let roll = (hasher.finish() % 10000) as f64 / 10000.0;
    let success = roll < AWAKENING_SUCCESS_RATE;

    if success {
        // 选择觉醒效果
        let effect = select_awakening_effect(user_id, slot);
        let section = format!("equip_awakening_{}", user_id);
        db.global_set(&section, &format!("aw_{}", slot), effect.id);

        // 记录觉醒时间
        db.global_set(
            &section,
            &format!("awt_{}", slot),
            &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        // 更新觉醒总数
        let total_key = "awakening_total";
        let current_total: i32 = db.global_get(&section, total_key).parse().unwrap_or(0);
        db.global_set(&section, total_key, &(current_total + 1).to_string());

        format!(
            "{}\n═══ ✨ 觉醒成功！═══\n\n{} 觉醒为 ✨{}{}\n\n{}\n\n🎲 触发概率: {}%\n效果值: {}%\n\n💰 消耗: {}金币 + {}钻石 + {}×{}",
            prefix,
            slot_display(slot),
            effect.emoji,
            effect.name,
            effect.desc,
            effect.trigger_rate / 100,
            effect.effect_value,
            format_num(AWAKENING_COST_GOLD),
            AWAKENING_COST_DIAMOND,
            AWAKENING_ITEM_QTY,
            AWAKENING_ITEM
        )
    } else {
        format!(
            "{}\n═══ 💔 觉醒失败 ═══\n\n{} 觉醒失败...\n材料已消耗，但装备安然无恙。\n\n💰 已消耗: {}金币 + {}钻石 + {}×{}\n🎲 再试一次吧！成功率: {}%",
            prefix,
            slot_display(slot),
            format_num(AWAKENING_COST_GOLD),
            AWAKENING_COST_DIAMOND,
            AWAKENING_ITEM_QTY,
            AWAKENING_ITEM,
            (AWAKENING_SUCCESS_RATE * 100.0) as i32
        )
    }
}

/// 觉醒排行 — 全服觉醒数量排行
pub fn cmd_awakening_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut players: Vec<(String, i32)> = Vec::new();

    for uid in db.all_users().iter() {
        let section = format!("equip_awakening_{}", uid);
        let total: i32 = db.global_get(&section, "awakening_total").parse().unwrap_or(0);
        if total > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, total));
        }
    }

    if players.is_empty() {
        return format!("{}\n暂无觉醒数据\n💡 完成装备进化后可进行觉醒", prefix);
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("{}\n═══ ✨ 觉醒排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, count)) in players.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        out.push_str(&format!("\n{} {} — {}件装备已觉醒", medal, name, count));
    }

    if let Some(rank) = players
        .iter()
        .position(|(name, _)| name == &user::get_msg_prefix(db, user_id))
    {
        out.push_str(&format!("\n\n📍 你的排名: 第{}名", rank + 1));
    }

    out
}

/// 觉醒图鉴 — 查看所有觉醒效果
pub fn cmd_awakening_codex(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let filter = args.trim();
    let section = format!("equip_awakening_{}", user_id);

    let mut out = format!("{}\n═══ 📖 觉醒图鉴 ═══", prefix);

    // 统计已获得的效果
    let mut owned_effects: Vec<String> = Vec::new();
    for &(slot, _) in EQUIP_SLOTS {
        let effect_id = db.global_get(&section, &format!("aw_{}", slot));
        if !effect_id.is_empty() && !owned_effects.contains(&effect_id) {
            owned_effects.push(effect_id);
        }
    }

    out.push_str(&format!(
        "\n📊 收集进度: {}/{} ({}%)\n",
        owned_effects.len(),
        AWAKENING_EFFECTS.len(),
        owned_effects.len() * 100 / AWAKENING_EFFECTS.len().max(1)
    ));

    for effect in AWAKENING_EFFECTS {
        // 如果有筛选条件，检查是否匹配
        if !filter.is_empty()
            && !effect.name.contains(filter)
            && !effect.desc.contains(filter)
            && !effect.id.contains(filter)
        {
            continue;
        }

        let owned = owned_effects.iter().any(|id| id == effect.id);
        let status = if owned { "✅" } else { "⬜" };
        let prob = effect.weight as f64 / AWAKENING_EFFECTS.iter().map(|e| e.weight).sum::<u32>() as f64 * 100.0;

        out.push_str(&format!(
            "\n{} {} {} — {}\n   触发率:{}% | 效果值:{}% | 获得概率:{:.0}%",
            status,
            effect.emoji,
            effect.name,
            effect.desc,
            effect.trigger_rate / 100,
            effect.effect_value,
            prob
        ));
    }

    if !filter.is_empty() && out.contains("═══") && !out.contains("\n✅") && !out.contains("\n⬜") {
        out.push_str(&format!("\n\n未找到匹配「{}」的觉醒效果", filter));
    }

    out
}

/// 获取觉醒战斗加成 — 供战斗系统集成
/// 返回 (效果ID, 触发概率万分比, 效果值)
#[allow(dead_code)]
pub fn get_awakening_combat_bonus(db: &Database, user_id: &str, slot: &str) -> Option<(&'static str, i32, i32)> {
    let section = format!("equip_awakening_{}", user_id);
    let effect_id = db.global_get(&section, &format!("aw_{}", slot));
    if effect_id.is_empty() {
        return None;
    }
    AWAKENING_EFFECTS
        .iter()
        .find(|e| e.id == effect_id)
        .map(|e| (e.id, e.trigger_rate, e.effect_value))
}

/// 获取玩家总觉醒数量
#[allow(dead_code)]
pub fn get_awakening_count(db: &Database, user_id: &str) -> i32 {
    let section = format!("equip_awakening_{}", user_id);
    db.global_get(&section, "awakening_total").parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effects_count() {
        assert_eq!(AWAKENING_EFFECTS.len(), 8);
    }

    #[test]
    fn test_effect_ids_unique() {
        let mut ids: Vec<&str> = AWAKENING_EFFECTS.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), AWAKENING_EFFECTS.len());
    }

    #[test]
    fn test_effect_names_unique() {
        let mut names: Vec<&str> = AWAKENING_EFFECTS.iter().map(|e| e.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), AWAKENING_EFFECTS.len());
    }

    #[test]
    fn test_effect_weights_positive() {
        for effect in AWAKENING_EFFECTS {
            assert!(effect.weight > 0, "{} weight must be positive", effect.id);
        }
    }

    #[test]
    fn test_effect_trigger_rates() {
        for effect in AWAKENING_EFFECTS {
            assert!(effect.trigger_rate > 0 && effect.trigger_rate <= 10000);
        }
    }

    #[test]
    fn test_effect_values_positive() {
        for effect in AWAKENING_EFFECTS {
            assert!(effect.effect_value > 0, "{} value must be positive", effect.id);
        }
    }

    #[test]
    fn test_effect_emojis_non_empty() {
        for effect in AWAKENING_EFFECTS {
            assert!(!effect.emoji.is_empty(), "{} emoji must not be empty", effect.id);
        }
    }

    #[test]
    fn test_effect_types_valid() {
        let valid_types = [
            "lifesteal",
            "buff_crit",
            "shield",
            "damage",
            "debuff_atk",
            "revive",
            "penetrate",
            "heal",
        ];
        for effect in AWAKENING_EFFECTS {
            assert!(
                valid_types.contains(&effect.effect_type),
                "{} has invalid type: {}",
                effect.id,
                effect.effect_type
            );
        }
    }

    #[test]
    fn test_total_weight() {
        let total: u32 = AWAKENING_EFFECTS.iter().map(|e| e.weight).sum();
        assert!(total > 0);
        assert_eq!(total, 120); // 20+18+15+12+20+5+15+15
    }

    #[test]
    fn test_resolve_slot_valid() {
        assert_eq!(resolve_slot("weapon"), Some("weapon"));
        assert_eq!(resolve_slot("helmet"), Some("helmet"));
        assert_eq!(resolve_slot("armor"), Some("armor"));
        assert_eq!(resolve_slot("ring"), Some("ring"));
    }

    #[test]
    fn test_resolve_slot_invalid() {
        assert_eq!(resolve_slot("nonexistent"), None);
        assert_eq!(resolve_slot(""), None);
    }

    #[test]
    fn test_slot_display() {
        assert!(slot_display("weapon").contains("武器"));
        assert!(slot_display("helmet").contains("头盔"));
        assert!(slot_display("unknown").contains("未知"));
    }

    #[test]
    fn test_equip_slots_count() {
        assert_eq!(EQUIP_SLOTS.len(), 10);
    }

    #[test]
    fn test_select_deterministic() {
        let e1 = select_awakening_effect("test_user", "weapon");
        let e2 = select_awakening_effect("test_user", "weapon");
        assert_eq!(e1.id, e2.id);
    }

    #[test]
    fn test_select_different_slots() {
        // Different slots may produce different effects (not guaranteed but likely)
        let effects: Vec<&str> = (0..10)
            .map(|i| select_awakening_effect("test_user", EQUIP_SLOTS[i].0).id)
            .collect();
        // At least check they're all valid
        for eff_id in &effects {
            assert!(AWAKENING_EFFECTS.iter().any(|e| e.id == *eff_id));
        }
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(50000), "50,000");
        assert_eq!(format_num(1000000), "1,000,000");
    }

    #[test]
    fn test_awakening_cost_constants() {
        assert!(AWAKENING_COST_GOLD > 0);
        assert!(AWAKENING_COST_DIAMOND > 0);
        assert!(AWAKENING_ITEM_QTY > 0);
        assert!(AWAKENING_SUCCESS_RATE > 0.0 && AWAKENING_SUCCESS_RATE <= 1.0);
    }

    #[test]
    fn test_min_evolution_tier() {
        assert_eq!(MIN_EVOLUTION_TIER, 3);
    }

    #[test]
    fn test_phoenix_rebirth_rarity() {
        // Phoenix rebirth should be the rarest
        let phoenix = AWAKENING_EFFECTS.iter().find(|e| e.id == "phoenix_rebirth").unwrap();
        let min_weight = AWAKENING_EFFECTS.iter().map(|e| e.weight).min().unwrap();
        assert_eq!(phoenix.weight, min_weight);
    }

    #[test]
    fn test_weight_distribution() {
        let total: u32 = AWAKENING_EFFECTS.iter().map(|e| e.weight).sum();
        for effect in AWAKENING_EFFECTS {
            let pct = effect.weight as f64 / total as f64 * 100.0;
            // Each effect should have at least 3% chance
            assert!(pct >= 3.0, "{} has too low probability: {:.1}%", effect.id, pct);
        }
    }
}

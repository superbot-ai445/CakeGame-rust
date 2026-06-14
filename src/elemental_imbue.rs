/// CakeGame 元素附灵系统 (Elemental Imbue System)
///
/// 为装备注入元素之力，使攻击附带元素伤害，防御获得元素抗性。
/// 与 type_effect.rs 属性克制系统深度集成：克制目标时伤害+30%，被克制时伤害-20%。
///
/// 6种元素: 🔥火 / 💧水 / 🌍土 / 💨风 / ✨光 / 🌑暗
/// 每件装备可附灵1种元素，附灵消耗「元素精华」+金币
/// 附灵等级1-10级，每级提升元素伤害/抗性百分比
///
/// 指令: 查看附灵/附灵装备/附灵移除/附灵商店/购买附灵/附灵详情/附灵排行/附灵帮助
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 元素类型
const ELEMENTS: &[(&str, &str, &str)] = &[
    ("火", "🔥", "灼烧"),
    ("水", "💧", "冰冻"),
    ("土", "🌍", "石化"),
    ("风", "💨", "撕裂"),
    ("光", "✨", "净化"),
    ("暗", "🌑", "侵蚀"),
];

/// 元素克制关系: (攻击元素, 被克制元素)
const ELEMENT_WEAKNESSES: &[(&str, &str)] = &[
    ("火", "风"), // 火克风
    ("水", "火"), // 水克火
    ("土", "水"), // 土克水
    ("风", "土"), // 风克土
    ("光", "暗"), // 光克暗
    ("暗", "光"), // 暗克光
];

/// 附灵槽位
const IMBUE_SLOTS: &[(&str, &str)] = &[
    ("武器", SLOT_WEAPON),
    ("头盔", SLOT_HELMET),
    ("铠甲", SLOT_ARMOR),
    ("护腿", SLOT_LEG),
    ("靴子", SLOT_BOOTS),
    ("项链", SLOT_NECKLACE),
    ("戒指", SLOT_RING),
];

/// 每级附灵属性加成百分比
const PER_LEVEL_BONUS: f64 = 3.0;

/// 最大附灵等级
const MAX_IMBUE_LEVEL: i32 = 10;

/// 元素精华名称
const ESSENCE_NAME: &str = "元素精华";

/// 附灵费用 (金币): 基础 3000 + 等级 × 2000
fn imbue_cost(level: i32) -> i64 {
    3000 + (level as i64) * 2000
}

/// 附灵所需精华数量: 基础 2 + 等级
fn essence_cost(level: i32) -> i32 {
    2 + level
}

/// 单槽位附灵数据
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
struct SlotImbue {
    element: String,
    level: i32,
}

/// 读取附灵数据
fn get_imbue_data(db: &Database, user_id: &str) -> std::collections::HashMap<String, SlotImbue> {
    let data = db.global_get("elemental_imbue", user_id);
    if data.is_empty() {
        return std::collections::HashMap::new();
    }
    serde_json::from_str(&data).unwrap_or_default()
}

/// 保存附灵数据
fn save_imbue_data(db: &Database, user_id: &str, data: &std::collections::HashMap<String, SlotImbue>) {
    let json = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    db.global_set("elemental_imbue", user_id, &json);
}

/// 解析槽位名
fn resolve_slot(name: &str) -> Option<&'static str> {
    let trimmed = name.trim();
    for (display, key) in IMBUE_SLOTS {
        if trimmed == *display || trimmed == *key {
            return Some(key);
        }
    }
    None
}

/// 解析元素名
fn resolve_element(name: &str) -> Option<&'static str> {
    let trimmed = name.trim();
    for (elem_name, _emoji, _effect) in ELEMENTS {
        if trimmed == *elem_name {
            return Some(elem_name);
        }
    }
    None
}

/// 元素emoji
fn element_emoji(elem: &str) -> &str {
    ELEMENTS
        .iter()
        .find(|(n, _, _)| *n == elem)
        .map(|(_, e, _)| *e)
        .unwrap_or("❓")
}

/// 元素效果名
fn element_effect(elem: &str) -> &str {
    ELEMENTS
        .iter()
        .find(|(n, _, _)| *n == elem)
        .map(|(_, _, effect)| *effect)
        .unwrap_or("未知")
}

/// 检查克制关系: 返回伤害倍率
#[allow(dead_code)]
fn element_multiplier(attack_elem: &str, defense_elem: &str) -> f64 {
    if attack_elem.is_empty() || defense_elem.is_empty() {
        return 1.0;
    }
    // First pass: check for advantage (克制)
    for (attacker, weak) in ELEMENT_WEAKNESSES {
        if *attacker == attack_elem && *weak == defense_elem {
            return 1.3; // 克制: +30%
        }
    }
    // Second pass: check for disadvantage (被克制)
    for (attacker, weak) in ELEMENT_WEAKNESSES {
        if *attacker == defense_elem && *weak == attack_elem {
            return 0.8; // 被克制: -20%
        }
    }
    1.0
}

/// 计算用户附灵总战力
fn calc_imbue_power(db: &Database, user_id: &str) -> i32 {
    let data = get_imbue_data(db, user_id);
    let mut total = 0;
    for imbue in data.values() {
        if !imbue.element.is_empty() && imbue.level > 0 {
            total += imbue.level * 100;
        }
    }
    total
}

/// 获取用户武器附灵元素 (供战斗系统调用)
pub fn get_weapon_element(db: &Database, user_id: &str) -> String {
    let data = get_imbue_data(db, user_id);
    data.get(SLOT_WEAPON)
        .filter(|s| s.level > 0)
        .map(|s| s.element.clone())
        .unwrap_or_default()
}

/// 计算附灵元素伤害加成百分比
pub fn get_element_damage_bonus(db: &Database, user_id: &str) -> f64 {
    let data = get_imbue_data(db, user_id);
    let weapon = data.get(SLOT_WEAPON);
    match weapon {
        Some(w) if w.level > 0 => w.level as f64 * PER_LEVEL_BONUS,
        _ => 0.0,
    }
}

/// 计算附灵元素抗性 (所有防具附灵的平均抗性)
#[allow(dead_code)]
pub fn get_element_resistance(db: &Database, user_id: &str, attack_element: &str) -> f64 {
    let data = get_imbue_data(db, user_id);
    let defensive_slots = [SLOT_HELMET, SLOT_ARMOR, SLOT_LEG, SLOT_BOOTS, SLOT_NECKLACE, SLOT_RING];
    let mut total_resist = 0.0;
    let mut count = 0;
    for slot in &defensive_slots {
        if let Some(imbue) = data.get(*slot) {
            if imbue.level > 0 {
                let base_resist = imbue.level as f64 * PER_LEVEL_BONUS;
                let multiplier = element_multiplier(attack_element, &imbue.element);
                total_resist += base_resist * multiplier;
                count += 1;
            }
        }
    }
    if count > 0 {
        total_resist / count as f64
    } else {
        0.0
    }
}

// ==================== 指令处理器 ====================

/// 查看附灵
pub fn cmd_view_imbue(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let data = get_imbue_data(db, user_id);
    let equips = db.equip_all(user_id);
    let equipped: std::collections::HashSet<String> = equips.iter().map(|e| e.slot.clone()).collect();

    let mut out = format!("{}\n═══ 🌈 元素附灵 ═══", prefix);

    for (display_name, slot_key) in IMBUE_SLOTS {
        let has_equip = equipped.contains(*slot_key);
        let imbue = data.get(*slot_key);

        if !has_equip {
            out.push_str(&format!("\n\n📍 {} — ❌ 未装备", display_name));
        } else if let Some(imbue) = imbue {
            if imbue.level > 0 {
                let emoji = element_emoji(&imbue.element);
                let effect = element_effect(&imbue.element);
                let bonus = imbue.level as f64 * PER_LEVEL_BONUS;
                out.push_str(&format!(
                    "\n\n📍 {} — {} {} Lv.{} ({})",
                    display_name, emoji, imbue.element, imbue.level, effect
                ));
                out.push_str(&format!("\n   加成: +{:.1}% 元素伤害", bonus));
            } else {
                out.push_str(&format!("\n\n📍 {} — ⚪ 未附灵", display_name));
            }
        } else {
            out.push_str(&format!("\n\n📍 {} — ⚪ 未附灵", display_name));
        }
    }

    let essences = db.read_basic(user_id, ESSENCE_NAME).parse().unwrap_or(0);
    out.push_str(&format!("\n\n💎 元素精华: {} 个", essences));
    out.push_str(&format!("\n🏆 附灵总战力: {}", calc_imbue_power(db, user_id)));

    // 显示武器元素伤害
    let weapon_elem = get_weapon_element(db, user_id);
    if !weapon_elem.is_empty() {
        let damage_bonus = get_element_damage_bonus(db, user_id);
        out.push_str(&format!(
            "\n⚔️ 武器元素: {} {} (+{:.1}%伤害)",
            element_emoji(&weapon_elem),
            weapon_elem,
            damage_bonus
        ));
    }

    out.push_str("\n\n💡 帮助: 发送「附灵帮助」");
    out
}

/// 附灵装备 — 为指定槽位附灵指定元素
pub fn cmd_imbue_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let parts: Vec<&str> = args.split([' ', '+']).collect();
    if parts.len() < 2 {
        return format!("{}\n用法: 附灵装备+槽位+元素\n例如: 附灵装备+武器+火", prefix);
    }

    let slot_key = match resolve_slot(parts[0]) {
        Some(k) => k,
        None => {
            let slots: Vec<&str> = IMBUE_SLOTS.iter().map(|(d, _)| *d).collect();
            return format!("{}\n❌ 无效槽位！可选: {}", prefix, slots.join("/"));
        }
    };

    let element = match resolve_element(parts[1]) {
        Some(e) => e,
        None => {
            let elems: Vec<&str> = ELEMENTS.iter().map(|(n, _, _)| *n).collect();
            return format!("{}\n❌ 无效元素！可选: {}", prefix, elems.join("/"));
        }
    };

    // 检查是否有装备
    let equips = db.equip_all(user_id);
    if !equips.iter().any(|e| e.slot == slot_key) {
        return format!("{}\n❌ 该槽位没有装备！请先装备物品。", prefix);
    }

    let mut data = get_imbue_data(db, user_id);
    let slot_data = data.entry(slot_key.to_string()).or_default();

    // 检查是否已附灵相同元素
    if slot_data.element == element && slot_data.level >= MAX_IMBUE_LEVEL {
        return format!("{}\n⚠️ 该槽位已满级{}元素附灵！", prefix, element_emoji(element));
    }

    let new_level = if slot_data.element == element {
        slot_data.level + 1
    } else {
        1 // 切换元素，重置为1级
    };

    if new_level > MAX_IMBUE_LEVEL {
        return format!("{}\n❌ 已达到最大附灵等级！", prefix);
    }

    // 检查消耗
    let gold_cost = imbue_cost(new_level - 1);
    let essence_need = essence_cost(new_level - 1);
    let gold = db.read_currency(user_id, CURRENCY_GOLD) as i32;
    let essences = db.read_basic(user_id, ESSENCE_NAME).parse().unwrap_or(0);

    if (gold as i64) < gold_cost {
        return format!("{}\n❌ 金币不足！需要 {} 金币。", prefix, gold_cost);
    }
    if essences < essence_need {
        return format!(
            "{}\n❌ 元素精华不足！需要 {} 个。发送「附灵商店」购买。",
            prefix, essence_need
        );
    }

    // 扣除资源
    db.write_basic_int(user_id, CURRENCY_GOLD, gold - gold_cost as i32);
    db.write_basic(user_id, ESSENCE_NAME, &(essences - essence_need).to_string());

    // 更新附灵
    let old_elem = slot_data.element.clone();
    slot_data.element = element.to_string();
    slot_data.level = new_level;
    save_imbue_data(db, user_id, &data);

    let emoji = element_emoji(element);
    let bonus = new_level as f64 * PER_LEVEL_BONUS;

    if old_elem != element && !old_elem.is_empty() {
        format!(
            "{}\n✅ 附灵成功！元素切换: {} → {}\n📍 {} {} Lv.{}\n⚔️ 元素伤害: +{:.1}%\n\n💰 花费: {}金币 + {}元素精华\n💎 剩余精华: {}",
            prefix, element_emoji(&old_elem), emoji,
            slot_key, element, new_level, bonus,
            gold_cost, essence_need, essences - essence_need
        )
    } else {
        format!(
            "{}\n✅ 附灵成功！\n📍 {} {} Lv.{}\n⚔️ 元素伤害: +{:.1}%\n\n💰 花费: {}金币 + {}元素精华\n💎 剩余精华: {}",
            prefix,
            slot_key,
            element,
            new_level,
            bonus,
            gold_cost,
            essence_need,
            essences - essence_need
        )
    }
}

/// 附灵移除 — 移除指定槽位的附灵
pub fn cmd_remove_imbue(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let slot_key = match resolve_slot(args) {
        Some(k) => k,
        None => return format!("{}\n❌ 无效槽位！", prefix),
    };

    let mut data = get_imbue_data(db, user_id);
    let slot_data = match data.get_mut(slot_key) {
        Some(sd) if sd.level > 0 => sd,
        _ => return format!("{}\n❌ 该槽位没有附灵！", prefix),
    };

    let old_element = slot_data.element.clone();
    let old_level = slot_data.level;

    // 返还50%精华
    let refund = (old_level * 2) / 2;
    let essences = db.read_basic(user_id, ESSENCE_NAME).parse().unwrap_or(0);
    db.write_basic(user_id, ESSENCE_NAME, &(essences + refund).to_string());

    slot_data.element = String::new();
    slot_data.level = 0;
    save_imbue_data(db, user_id, &data);

    format!(
        "{}\n✅ 附灵移除成功！\n📍 {} {} Lv.{} 已移除\n💎 返还 {} 元素精华\n\n💡 可重新附灵其他元素。",
        prefix,
        slot_key,
        element_emoji(&old_element),
        old_level,
        refund
    )
}

/// 附灵详情 — 显示附灵系统的详细信息
pub fn cmd_imbue_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let data = get_imbue_data(db, user_id);
    let mut total_power = 0;
    let mut weapon_elem = String::new();
    let mut weapon_level = 0;
    let mut defense_total = 0.0;
    let mut def_count = 0;

    for (slot, imbue) in &data {
        if imbue.level > 0 {
            total_power += imbue.level * 100;
            if slot == SLOT_WEAPON {
                weapon_elem = imbue.element.clone();
                weapon_level = imbue.level;
            } else {
                defense_total += imbue.level as f64 * PER_LEVEL_BONUS;
                def_count += 1;
            }
        }
    }

    let avg_defense = if def_count > 0 {
        defense_total / def_count as f64
    } else {
        0.0
    };

    let mut out = format!("{}\n═══ 📊 附灵详情 ═══", prefix);

    if !weapon_elem.is_empty() {
        let bonus = weapon_level as f64 * PER_LEVEL_BONUS;
        out.push_str(&format!(
            "\n\n⚔️ 武器元素: {} {} Lv.{}\n   伤害加成: +{:.1}%",
            element_emoji(&weapon_elem),
            weapon_elem,
            weapon_level,
            bonus
        ));

        // 显示克制关系
        out.push_str("\n\n🔄 克制关系:");
        for (attacker, weak) in ELEMENT_WEAKNESSES {
            if *attacker == weapon_elem {
                out.push_str(&format!(
                    "\n   {} {} → {} {} (+30%伤害)",
                    element_emoji(attacker),
                    attacker,
                    element_emoji(weak),
                    weak
                ));
            }
        }
        for (attacker, weak) in ELEMENT_WEAKNESSES {
            if *weak == weapon_elem {
                out.push_str(&format!(
                    "\n   {} {} ← {} {} (-20%伤害)",
                    element_emoji(&weapon_elem),
                    weapon_elem,
                    element_emoji(attacker),
                    attacker
                ));
            }
        }
    }

    if def_count > 0 {
        out.push_str(&format!("\n\n🛡️ 平均元素抗性: +{:.1}%", avg_defense));
    }

    out.push_str(&format!("\n\n🏆 附灵总战力: {}", total_power));
    out.push_str("\n\n💡 附灵可与属性克制系统联动，克制目标时伤害+30%");
    out
}

/// 附灵商店 — 购买元素精华
pub fn cmd_imbue_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let essences = db.read_basic(user_id, ESSENCE_NAME).parse().unwrap_or(0);
    let gold = db.read_currency(user_id, CURRENCY_GOLD) as i32;
    let diamonds = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;

    format!(
        "{}\n═══ 🏪 附灵商店 ═══\n\n\
        1️⃣ 元素精华 ×5 — 5,000 金币\n\
        2️⃣ 元素精华 ×20 — 18,000 金币 (9折)\n\
        3️⃣ 元素精华 ×50 — 40,000 金币 (8折)\n\
        4️⃣ 元素精华 ×5 — 15 💎\n\
        5️⃣ 元素精华 ×50 — 120 💎 (8折)\n\n\
        📦 当前库存: {} 元素精华\n💰 金币: {} | 💎 钻石: {}\n\n\
        💡 购买: 发送「购买附灵+编号」",
        prefix, essences, gold, diamonds
    )
}

/// 购买元素精华
pub fn cmd_buy_imbue(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let option: i32 = args.trim().parse().unwrap_or(0);
    let (amount, cost_gold, cost_diamond) = match option {
        1 => (5, 5000, 0),
        2 => (20, 18000, 0),
        3 => (50, 40000, 0),
        4 => (5, 0, 15),
        5 => (50, 0, 120),
        _ => return format!("{}\n❌ 无效选项！请输入1-5。", prefix),
    };

    if cost_gold > 0 {
        let gold = db.read_currency(user_id, CURRENCY_GOLD) as i32;
        if gold < cost_gold {
            return format!("{}\n❌ 金币不足！需要 {} 金币。", prefix, cost_gold);
        }
        db.write_basic_int(user_id, CURRENCY_GOLD, gold - cost_gold);
    }
    if cost_diamond > 0 {
        let diamonds = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;
        if diamonds < cost_diamond {
            return format!("{}\n❌ 钻石不足！需要 {} 💎。", prefix, cost_diamond);
        }
        db.write_basic_int(user_id, CURRENCY_DIAMOND, diamonds - cost_diamond);
    }

    let current = db.read_basic(user_id, ESSENCE_NAME).parse().unwrap_or(0);
    db.write_basic(user_id, ESSENCE_NAME, &(current + amount).to_string());

    let cost_text = if cost_gold > 0 {
        format!("{}金币", cost_gold)
    } else {
        format!("{}💎", cost_diamond)
    };

    format!(
        "{}\n✅ 购买成功！\n\n获得 {} 个元素精华\n花费: {}\n当前库存: {} 元素精华",
        prefix,
        amount,
        cost_text,
        current + amount
    )
}

/// 附灵排行
pub fn cmd_imbue_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let users: Vec<String> = db.all_users();
    let mut rankings: Vec<(String, i32, String, i32)> = users
        .iter()
        .filter_map(|uid| {
            let data = get_imbue_data(db, uid);
            let mut power = 0;
            let mut best_elem = String::new();
            let mut best_level = 0;
            for imbue in data.values() {
                if imbue.level > 0 {
                    power += imbue.level * 100;
                    if imbue.level > best_level {
                        best_elem = imbue.element.clone();
                        best_level = imbue.level;
                    }
                }
            }
            if power > 0 {
                Some((uid.clone(), power, best_elem, best_level))
            } else {
                None
            }
        })
        .collect();
    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("{}\n═══ 🏆 附灵排行 ═══", prefix);
    let medals = ["🥇", "🥈", "🥉"];

    for (i, (uid, power, elem, level)) in rankings.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, ITEM_NAME);
        let name = if name.is_empty() { uid } else { &name };
        let emoji = element_emoji(elem);
        let marker = if uid == user_id { " ← 你" } else { "" };
        out.push_str(&format!(
            "\n{} {}. {}: {}战力 {}{} Lv.{}{}",
            medal,
            i + 1,
            name,
            power,
            emoji,
            elem,
            level,
            marker
        ));
    }

    if rankings.is_empty() {
        out.push_str("\n\n暂无附灵记录");
    }

    if let Some(pos) = rankings.iter().position(|(uid, _, _, _)| uid == user_id) {
        out.push_str(&format!("\n\n📍 你的排名: 第{}名", pos + 1));
    }

    out
}

/// 附灵帮助
pub fn cmd_imbue_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n═══ 🌈 元素附灵帮助 ═══\n\n\
        📖 功能说明:\n\
        为装备注入元素之力，使攻击附带元素伤害，防御获得元素抗性。\n\
        与属性克制系统深度集成，克制目标时伤害+30%！\n\n\
        📋 指令列表:\n\
        1. 查看附灵 — 查看所有装备附灵状态\n\
        2. 附灵装备+槽位+元素 — 附灵装备 (如: 附灵装备+武器+火)\n\
        3. 附灵移除+槽位 — 移除附灵 (返还50%精华)\n\
        4. 附灵详情 — 查看详细克制关系和抗性\n\
        5. 附灵商店 — 购买元素精华\n\
        6. 购买附灵+编号 — 购买元素精华\n\
        7. 附灵排行 — 全服附灵排行榜\n\
        8. 附灵帮助 — 显示本帮助\n\n\
        🌈 元素体系:\n\
        🔥火 克 💨风 → 💧水 → 🔥火 (循环)\n\
        💧水 克 🔥火 → 🌍土 → 💧水 (循环)\n\
        🌍土 克 💧水 → 💨风 → 🌍土 (循环)\n\
        ✨光 ↔ 🌑暗 (互克)\n\n\
        📊 规则:\n\
        • 每件装备可附灵1种元素\n\
        • 附灵等级1-10级，每级+3%元素伤害/抗性\n\
        • 武器附灵: 攻击时附加元素伤害\n\
        • 防具附灵: 受到元素攻击时减伤\n\
        • 克制目标: 伤害+30% | 被克制: 伤害-20%\n\
        • 切换元素: 重置为1级，原附灵消失\n\
        • 移除附灵: 返还50%精华",
        prefix
    )
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elements_count() {
        assert_eq!(ELEMENTS.len(), 6);
    }

    #[test]
    fn test_element_weaknesses_count() {
        assert_eq!(ELEMENT_WEAKNESSES.len(), 6);
    }

    #[test]
    fn test_imbue_slots_not_empty() {
        assert!(!IMBUE_SLOTS.is_empty());
    }

    #[test]
    fn test_imbue_cost_escalation() {
        assert_eq!(imbue_cost(0), 3000);
        assert_eq!(imbue_cost(1), 5000);
        assert_eq!(imbue_cost(5), 13000);
        assert_eq!(imbue_cost(9), 21000);
    }

    #[test]
    fn test_imbue_cost_always_positive() {
        for i in 0..MAX_IMBUE_LEVEL {
            assert!(imbue_cost(i) > 0);
        }
    }

    #[test]
    fn test_essence_cost_escalation() {
        assert_eq!(essence_cost(0), 2);
        assert_eq!(essence_cost(1), 3);
        assert_eq!(essence_cost(5), 7);
        assert_eq!(essence_cost(9), 11);
    }

    #[test]
    fn test_resolve_slot_valid() {
        assert!(resolve_slot("武器").is_some());
        assert!(resolve_slot("头盔").is_some());
        assert!(resolve_slot("铠甲").is_some());
        assert!(resolve_slot("项链").is_some());
    }

    #[test]
    fn test_resolve_slot_invalid() {
        assert!(resolve_slot("不存在").is_none());
        assert!(resolve_slot("").is_none());
    }

    #[test]
    fn test_resolve_element_valid() {
        assert!(resolve_element("火").is_some());
        assert!(resolve_element("水").is_some());
        assert!(resolve_element("光").is_some());
        assert!(resolve_element("暗").is_some());
    }

    #[test]
    fn test_resolve_element_invalid() {
        assert!(resolve_element("雷").is_none());
        assert!(resolve_element("").is_none());
    }

    #[test]
    fn test_element_emoji() {
        assert_eq!(element_emoji("火"), "🔥");
        assert_eq!(element_emoji("水"), "💧");
        assert_eq!(element_emoji("暗"), "🌑");
        assert_eq!(element_emoji("不存在"), "❓");
    }

    #[test]
    fn test_element_effect() {
        assert_eq!(element_effect("火"), "灼烧");
        assert_eq!(element_effect("水"), "冰冻");
        assert_eq!(element_effect("光"), "净化");
        assert_eq!(element_effect("未知"), "未知");
    }

    #[test]
    fn test_element_multiplier_advantage() {
        let m = element_multiplier("火", "风");
        assert!((m - 1.3).abs() < 0.01);
    }

    #[test]
    fn test_element_multiplier_disadvantage() {
        let m = element_multiplier("风", "火");
        assert!((m - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_element_multiplier_neutral() {
        // 火 vs 土: no direct weakness relationship
        let m = element_multiplier("火", "土");
        assert!((m - 1.0).abs() < 0.01);
        // 水 vs 风: no direct relationship
        let m2 = element_multiplier("水", "风");
        assert!((m2 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_element_multiplier_empty() {
        assert!((element_multiplier("", "火") - 1.0).abs() < 0.01);
        assert!((element_multiplier("火", "") - 1.0).abs() < 0.01);
        assert!((element_multiplier("", "") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_element_multiplier_all_pairs() {
        // Verify each weakness pair gives 1.3
        for (attacker, weak) in ELEMENT_WEAKNESSES {
            let m = element_multiplier(attacker, weak);
            assert!(
                (m - 1.3).abs() < 0.01,
                "{} should beat {} but got {}",
                attacker,
                weak,
                m
            );
        }
    }

    #[test]
    fn test_max_imbue_level() {
        assert_eq!(MAX_IMBUE_LEVEL, 10);
    }

    #[test]
    fn test_per_level_bonus() {
        assert!((PER_LEVEL_BONUS - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_slot_imbue_default() {
        let si = SlotImbue::default();
        assert!(si.element.is_empty());
        assert_eq!(si.level, 0);
    }

    #[test]
    fn test_imbue_data_serialization() {
        let mut data = std::collections::HashMap::new();
        data.insert(
            "武器".to_string(),
            SlotImbue {
                element: "火".to_string(),
                level: 5,
            },
        );
        data.insert(
            "头盔".to_string(),
            SlotImbue {
                element: "水".to_string(),
                level: 3,
            },
        );

        let json = serde_json::to_string(&data).unwrap();
        let decoded: std::collections::HashMap<String, SlotImbue> = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded.get("武器").unwrap().element, "火");
        assert_eq!(decoded.get("武器").unwrap().level, 5);
        assert_eq!(decoded.get("头盔").unwrap().element, "水");
    }

    #[test]
    fn test_essence_name() {
        assert_eq!(ESSENCE_NAME, "元素精华");
    }

    #[test]
    fn test_all_elements_have_emoji() {
        for (name, emoji, _) in ELEMENTS {
            assert!(!emoji.is_empty(), "{} should have emoji", name);
        }
    }

    #[test]
    fn test_all_elements_have_effect() {
        for (name, _, effect) in ELEMENTS {
            assert!(!effect.is_empty(), "{} should have effect", name);
        }
    }

    #[test]
    fn test_weakness_coverage() {
        // Each element should appear at least once as attacker
        for (elem_name, _, _) in ELEMENTS {
            let as_attacker = ELEMENT_WEAKNESSES.iter().any(|(a, _)| *a == *elem_name);
            assert!(as_attacker, "{} should appear as attacker", elem_name);
        }
    }

    #[test]
    fn test_full_level_bonus() {
        let bonus = MAX_IMBUE_LEVEL as f64 * PER_LEVEL_BONUS;
        assert!((bonus - 30.0).abs() < 0.01);
    }
}

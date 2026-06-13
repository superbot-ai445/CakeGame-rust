/// CakeGame 装备方案系统 (Equipment Loadout System)
///
/// 允许玩家保存/加载装备配置方案，快速切换不同场景的装备组合
/// 来源: 新增系统 — 扩展装备管理生态
///
/// 功能:
/// - 保存方案: 保存当前装备为命名方案
/// - 查看方案: 列出所有已保存方案
/// - 方案详情: 查看方案的完整装备列表
/// - 加载方案: 一键切换到指定方案
/// - 删除方案: 删除不需要的方案
/// - 方案对比: 对比两个方案的装备差异
///
/// 存储: Global 表 SECTION='equip_loadout_{user_id}'
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 每个玩家最多保存的方案数
const MAX_LOADOUTS: usize = 5;

/// 所有装备槽位
const ALL_SLOTS: &[&str] = &[
    SLOT_WEAPON,
    SLOT_HELMET,
    SLOT_ARMOR,
    SLOT_LEG,
    SLOT_BOOTS,
    SLOT_NECKLACE,
    SLOT_RING,
    SLOT_WING,
    SLOT_FASHION,
    SLOT_TITLE,
];

/// 槽位对应的图标
fn slot_icon(slot: &str) -> &'static str {
    match slot {
        SLOT_WEAPON => "⚔️",
        SLOT_HELMET => "🪖",
        SLOT_ARMOR => "🛡️",
        SLOT_LEG => "👖",
        SLOT_BOOTS => "👢",
        SLOT_NECKLACE => "📿",
        SLOT_RING => "💍",
        SLOT_WING => "🪽",
        SLOT_FASHION => "👗",
        SLOT_TITLE => "🏅",
        _ => "❓",
    }
}

/// 从 Global 表获取方案 section key
fn loadout_section(user_id: &str) -> String {
    format!("equip_loadout_{}", user_id)
}

/// 获取方案名称列表
fn get_loadout_names(db: &Database, user_id: &str) -> Vec<String> {
    let raw = db.global_get(&loadout_section(user_id), "names");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('|')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// 保存方案名称列表
fn save_loadout_names(db: &Database, user_id: &str, names: &[String]) {
    db.global_set(&loadout_section(user_id), "names", &names.join("|"));
}

/// 获取指定方案的装备配置: Vec<(slot, equip_name)>
fn get_loadout_data(db: &Database, user_id: &str, name: &str) -> Vec<(String, String)> {
    let raw = db.global_get(&loadout_section(user_id), &format!("ld_{}", name));
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('\n')
        .filter_map(|line| {
            let mut parts = line.splitn(2, ':');
            let slot = parts.next()?.to_string();
            let item = parts.next().unwrap_or("").to_string();
            Some((slot, item))
        })
        .collect()
}

/// 保存方案装备数据
fn save_loadout_data(db: &Database, user_id: &str, name: &str, items: &[(String, String)]) {
    let raw: Vec<String> = items.iter().map(|(slot, item)| format!("{}:{}", slot, item)).collect();
    db.global_set(&loadout_section(user_id), &format!("ld_{}", name), &raw.join("\n"));
}

/// 获取当前装备: Vec<(slot, equip_name)>
fn get_current_equips(db: &Database, user_id: &str) -> Vec<(String, String)> {
    db.equip_all(user_id)
        .iter()
        .map(|eq| (eq.slot.clone(), eq.name.clone()))
        .collect()
}

/// 格式化装备列表显示
fn format_equip_list(items: &[(String, String)]) -> String {
    let mut out = String::new();
    for (slot, name) in items {
        let icon = slot_icon(slot);
        if name.is_empty() {
            out.push_str(&format!("  {} {}: ❌ 空\n", icon, slot));
        } else {
            out.push_str(&format!("  {} {}: ✅ {}\n", icon, slot, name));
        }
    }
    out
}

/// 计算已装备数量
fn count_equipped(items: &[(String, String)]) -> usize {
    items.iter().filter(|(_, n)| !n.is_empty()).count()
}

/// 当前时间字符串
fn now_str() -> String {
    chrono::Local::now().format("%m-%d %H:%M").to_string()
}

/// 缩短物品名（去掉品质前缀）
fn short_item_name(name: &str) -> &str {
    if name.is_empty() {
        return "—";
    }
    if let Some(pos) = name.find('】') {
        &name[pos + 3..]
    } else {
        name
    }
}

/// 保存方案 — 保存当前装备配置为命名方案
pub fn cmd_save_loadout(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();

    if name.is_empty() {
        let names = get_loadout_names(db, user_id);
        let list = if names.is_empty() {
            "  （暂无方案）\n".to_string()
        } else {
            names
                .iter()
                .enumerate()
                .map(|(i, n)| format!("  {}. {}\n", i + 1, n))
                .collect()
        };
        return format!(
            "{}\n⚠️ 请指定方案名称\n💡 用法：保存方案+方案名\n📋 示例：保存方案+PvE输出\n\n📂 已有方案 ({}/{}):\n{}",
            prefix,
            names.len(),
            MAX_LOADOUTS,
            list
        );
    }

    if name.len() > 20 {
        return format!("{}\n⚠️ 方案名过长（最多20字符，当前{}字符）", prefix, name.len());
    }

    let mut names = get_loadout_names(db, user_id);
    let is_update = names.iter().any(|n| n == name);

    if !is_update && names.len() >= MAX_LOADOUTS {
        return format!(
            "{}\n⚠️ 方案数量已达上限 ({}/{})\n💡 请先删除不需要的方案",
            prefix,
            names.len(),
            MAX_LOADOUTS
        );
    }

    // 获取当前装备
    let current = get_current_equips(db, user_id);
    let equipped_count = count_equipped(&current);

    if equipped_count == 0 {
        return format!("{}\n⚠️ 当前没有穿戴任何装备，无法保存空方案", prefix);
    }

    // 保存方案
    save_loadout_data(db, user_id, name, &current);

    if !is_update {
        names.push(name.to_string());
        save_loadout_names(db, user_id, &names);
    }

    let section = loadout_section(user_id);
    db.global_set(&section, &format!("lt_{}", name), &now_str());

    let action = if is_update { "更新" } else { "保存" };
    format!(
        "{}\n✅ 方案「{}」{}成功！\n\n📋 当前装备已保存 ({}件):\n{}\n📂 已有方案: {}/{}",
        prefix,
        name,
        action,
        equipped_count,
        format_equip_list(&current),
        names.len(),
        MAX_LOADOUTS
    )
}

/// 查看方案 — 列出所有已保存方案
pub fn cmd_view_loadouts(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let names = get_loadout_names(db, user_id);
    let section = loadout_section(user_id);

    let mut out = format!("{}\n═══ 📂 装备方案管理 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    if names.is_empty() {
        out.push_str("📭 暂无保存的方案\n\n");
        out.push_str("💡 使用「保存方案+方案名」保存当前装备\n");
        out.push_str(&format!("💡 最多可保存{}个方案\n", MAX_LOADOUTS));
    } else {
        out.push_str(&format!("📊 方案数量: {}/{}\n\n", names.len(), MAX_LOADOUTS));

        for (i, name) in names.iter().enumerate() {
            let data = get_loadout_data(db, user_id, name);
            let eq_count = count_equipped(&data);
            let time = db.global_get(&section, &format!("lt_{}", name));
            let time_str = if time.is_empty() {
                String::new()
            } else {
                format!(" ({})", time)
            };
            out.push_str(&format!(
                "  {}. 📋 「{}」— {}件装备{}\n",
                i + 1,
                name,
                eq_count,
                time_str
            ));
        }

        out.push('\n');
        out.push_str("💡 操作指令:\n");
        out.push_str("  • 方案详情+方案名 — 查看完整装备\n");
        out.push_str("  • 加载方案+方案名 — 切换到该方案\n");
        out.push_str("  • 删除方案+方案名 — 删除方案\n");
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out
}

/// 方案详情 — 查看指定方案的完整装备列表
pub fn cmd_loadout_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();

    if name.is_empty() {
        let names = get_loadout_names(db, user_id);
        let hint = if names.is_empty() {
            "（暂无）".to_string()
        } else {
            names.join(", ")
        };
        return format!(
            "{}\n⚠️ 请指定方案名\n💡 用法：方案详情+方案名\n📋 已有方案: {}",
            prefix, hint
        );
    }

    let names = get_loadout_names(db, user_id);
    let resolved = resolve_loadout_name(&names, name);
    let match_name = match resolved {
        Some(n) => n,
        None => {
            let hint = if names.is_empty() {
                "（暂无）".to_string()
            } else {
                names.join(", ")
            };
            return format!("{}\n⚠️ 找不到方案「{}」\n📋 已有方案: {}", prefix, name, hint);
        }
    };

    let data = get_loadout_data(db, user_id, match_name);
    let eq_count = count_equipped(&data);
    let section = loadout_section(user_id);
    let time = db.global_get(&section, &format!("lt_{}", match_name));

    let mut out = format!("{}\n═══ 📋 方案「{}」════\n━━━━━━━━━━━━━━━━━━━━\n", prefix, match_name);

    if !time.is_empty() {
        out.push_str(&format!("📅 保存时间: {}\n", time));
    }
    out.push_str(&format!("📊 装备数量: {}/{}\n\n", eq_count, ALL_SLOTS.len()));

    out.push_str(&format_equip_list(&data));

    // 与当前装备对比
    let current = get_current_equips(db, user_id);
    let mut diff_count = 0usize;
    for (slot, loadout_item) in &data {
        let cur_item = current
            .iter()
            .find(|(s, _)| s == slot)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");
        if loadout_item.as_str() != cur_item {
            diff_count += 1;
        }
    }

    if diff_count > 0 {
        out.push_str(&format!("\n📊 与当前装备差异: {}件不同\n", diff_count));
    } else {
        out.push_str("\n✅ 与当前装备完全一致\n");
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("💡 使用「加载方案+{}」切换到此方案\n", match_name));
    out
}

/// 加载方案 — 一键切换到指定方案的装备配置
pub fn cmd_load_loadout(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();

    if name.is_empty() {
        let names = get_loadout_names(db, user_id);
        let hint = if names.is_empty() {
            "（暂无）".to_string()
        } else {
            names.join(", ")
        };
        return format!(
            "{}\n⚠️ 请指定方案名\n💡 用法：加载方案+方案名\n📋 已有方案: {}",
            prefix, hint
        );
    }

    let names = get_loadout_names(db, user_id);
    let resolved = resolve_loadout_name(&names, name);
    let match_name = match resolved {
        Some(n) => n,
        None => {
            return format!("{}\n⚠️ 找不到方案「{}」", prefix, name);
        }
    };

    // 检查虚弱状态
    if user::check_weakness(db, user_id) > 0 {
        return format!("{}\n⚠️ 你处于虚弱状态，无法切换装备方案", prefix);
    }

    let loadout = get_loadout_data(db, user_id, match_name);
    let current = get_current_equips(db, user_id);

    let mut changed = 0usize;
    let mut not_found = 0usize;

    for (slot, target_name) in &loadout {
        if target_name.is_empty() {
            continue;
        }

        // 查找当前槽位的装备
        let cur_name = current
            .iter()
            .find(|(s, _)| s == slot)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");

        // 如果已装备相同物品，跳过
        if cur_name == target_name {
            continue;
        }

        // 检查目标物品是否在背包中
        if db.knapsack_quantity(user_id, target_name) <= 0 {
            not_found += 1;
            continue;
        }

        // 获取目标物品定义
        let item_def = match db.item_get(target_name) {
            Some(def) => def,
            None => {
                not_found += 1;
                continue;
            }
        };

        // 等级检查
        let user_lv: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
        if item_def.data.lv_limit > 0 && user_lv < item_def.data.lv_limit {
            not_found += 1;
            continue;
        }

        // 卸下当前装备（放回背包）
        if !cur_name.is_empty() {
            if let Some(old_name) = db.equip_remove(user_id, slot) {
                db.knapsack_add(user_id, &old_name, 1);
            }
        }

        // 穿上目标装备
        db.equip_set(user_id, slot, &item_def);
        db.knapsack_remove(user_id, target_name, 1);
        changed += 1;
    }

    // 记录加载时间
    let section = loadout_section(user_id);
    db.global_set(&section, &format!("ll_{}", match_name), &now_str());

    let mut out = format!("{}\n═══ 🔄 加载方案「{}」════\n", prefix, match_name);

    if changed > 0 {
        out.push_str(&format!("✅ 成功切换 {} 件装备\n", changed));
    }
    if not_found > 0 {
        out.push_str(&format!("⚠️ {} 件装备不在背包或等级不足\n", not_found));
    }
    if changed == 0 && not_found == 0 {
        out.push_str("✅ 当前装备已与方案一致，无需切换\n");
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out
}

/// 删除方案 — 删除指定方案
pub fn cmd_delete_loadout(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();

    if name.is_empty() {
        let names = get_loadout_names(db, user_id);
        let hint = if names.is_empty() {
            "（暂无）".to_string()
        } else {
            names.join(", ")
        };
        return format!(
            "{}\n⚠️ 请指定要删除的方案名\n💡 用法：删除方案+方案名\n📋 已有方案: {}",
            prefix, hint
        );
    }

    let mut names = get_loadout_names(db, user_id);
    let resolved = resolve_loadout_name(&names, name);
    let match_name = match resolved {
        Some(n) => n.clone(),
        None => {
            return format!("{}\n⚠️ 找不到方案「{}」", prefix, name);
        }
    };

    // 清除数据
    let section = loadout_section(user_id);
    db.global_set(&section, &format!("ld_{}", match_name), "");
    db.global_set(&section, &format!("lt_{}", match_name), "");
    db.global_set(&section, &format!("ll_{}", match_name), "");

    // 更新名称列表
    names.retain(|n| n != &match_name);
    save_loadout_names(db, user_id, &names);

    format!(
        "{}\n🗑️ 方案「{}」已删除\n📂 剩余方案: {}/{}",
        prefix,
        match_name,
        names.len(),
        MAX_LOADOUTS
    )
}

/// 方案对比 — 对比两个方案的差异
pub fn cmd_compare_loadouts(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return format!("{}\n⚠️ 请指定两个方案名\n💡 用法：方案对比+方案A+方案B", prefix);
    }

    let names = get_loadout_names(db, user_id);

    let name_a = match resolve_loadout_name(&names, parts[0]) {
        Some(n) => n,
        None => return format!("{}\n⚠️ 找不到方案「{}」", prefix, parts[0]),
    };
    let name_b = match resolve_loadout_name(&names, parts[1]) {
        Some(n) => n,
        None => return format!("{}\n⚠️ 找不到方案「{}」", prefix, parts[1]),
    };

    let data_a = get_loadout_data(db, user_id, name_a);
    let data_b = get_loadout_data(db, user_id, name_b);

    let mut out = format!("{}\n═══ ⚖️ 方案对比 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    out.push_str(&format!("📋 「{}」 vs 「{}」\n\n", name_a, name_b));
    out.push_str(&format!(
        "📊 装备数量: {} vs {}\n\n",
        count_equipped(&data_a),
        count_equipped(&data_b)
    ));

    let mut same = 0usize;
    let mut diff = 0usize;

    for &slot in ALL_SLOTS {
        let item_a = data_a
            .iter()
            .find(|(s, _)| s == slot)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");
        let item_b = data_b
            .iter()
            .find(|(s, _)| s == slot)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");

        let icon = slot_icon(slot);

        if item_a == item_b {
            same += 1;
            let display = if item_a.is_empty() {
                "空".to_string()
            } else {
                short_item_name(item_a).to_string()
            };
            out.push_str(&format!("  {} {}: {} (相同)\n", icon, slot, display));
        } else {
            diff += 1;
            out.push_str(&format!(
                "  {} {}: {} ↔ {}\n",
                icon,
                slot,
                if item_a.is_empty() {
                    "空".to_string()
                } else {
                    short_item_name(item_a).to_string()
                },
                if item_b.is_empty() {
                    "空".to_string()
                } else {
                    short_item_name(item_b).to_string()
                }
            ));
        }
    }

    out.push_str(&format!("\n📊 相同: {}件 | 不同: {}件\n", same, diff));
    if diff == 0 {
        out.push_str("✅ 两个方案完全一致\n");
    }
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out
}

/// 解析方案名（精确匹配 → 包含匹配）
fn resolve_loadout_name<'a>(names: &'a [String], query: &str) -> Option<&'a String> {
    // 精确匹配
    if let Some(n) = names.iter().find(|n| n.as_str() == query) {
        return Some(n);
    }
    // 包含匹配
    let fuzzy: Vec<&String> = names.iter().filter(|n| n.contains(query)).collect();
    if fuzzy.len() == 1 {
        Some(fuzzy[0])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_slots_count() {
        assert_eq!(ALL_SLOTS.len(), 10);
    }

    #[test]
    fn test_all_slots_match_constants() {
        assert_eq!(ALL_SLOTS[0], SLOT_WEAPON);
        assert_eq!(ALL_SLOTS[1], SLOT_HELMET);
        assert_eq!(ALL_SLOTS[9], SLOT_TITLE);
    }

    #[test]
    fn test_slot_icon_all_slots() {
        for &slot in ALL_SLOTS {
            let icon = slot_icon(slot);
            assert!(!icon.is_empty(), "Empty icon for slot: {}", slot);
            assert_ne!(icon, "❓", "Unknown icon for slot: {}", slot);
        }
    }

    #[test]
    fn test_max_loadouts() {
        assert_eq!(MAX_LOADOUTS, 5);
    }

    #[test]
    fn test_loadout_section_format() {
        assert_eq!(loadout_section("user123"), "equip_loadout_user123");
    }

    #[test]
    fn test_short_item_name_empty() {
        assert_eq!(short_item_name(""), "—");
    }

    #[test]
    fn test_short_item_name_with_quality() {
        assert_eq!(short_item_name("【史诗】屠龙刀(+5)"), "屠龙刀(+5)");
    }

    #[test]
    fn test_short_item_name_plain() {
        assert_eq!(short_item_name("铁剑"), "铁剑");
    }

    #[test]
    fn test_count_equipped_mixed() {
        let items = vec![
            ("武器".to_string(), "铁剑".to_string()),
            ("头盔".to_string(), "".to_string()),
            ("铠甲".to_string(), "铁甲".to_string()),
        ];
        assert_eq!(count_equipped(&items), 2);
    }

    #[test]
    fn test_count_equipped_empty() {
        let items = vec![
            ("武器".to_string(), "".to_string()),
            ("头盔".to_string(), "".to_string()),
        ];
        assert_eq!(count_equipped(&items), 0);
    }

    #[test]
    fn test_format_equip_list() {
        let items = vec![
            ("武器".to_string(), "铁剑".to_string()),
            ("头盔".to_string(), "".to_string()),
        ];
        let out = format_equip_list(&items);
        assert!(out.contains("⚔️"));
        assert!(out.contains("✅ 铁剑"));
        assert!(out.contains("❌ 空"));
    }

    #[test]
    fn test_resolve_exact() {
        let names = vec!["PvE输出".to_string(), "PvP防御".to_string()];
        assert_eq!(
            resolve_loadout_name(&names, "PvE输出").map(|s| s.as_str()),
            Some("PvE输出")
        );
    }

    #[test]
    fn test_resolve_fuzzy() {
        let names = vec!["PvE输出".to_string(), "PvP防御".to_string()];
        assert_eq!(resolve_loadout_name(&names, "PvE").map(|s| s.as_str()), Some("PvE输出"));
    }

    #[test]
    fn test_resolve_fuzzy_ambiguous() {
        let names = vec!["PvE输出".to_string(), "PvE防御".to_string()];
        // 两个匹配，应返回 None
        assert!(resolve_loadout_name(&names, "PvE").is_none());
    }

    #[test]
    fn test_resolve_not_found() {
        let names = vec!["PvE输出".to_string()];
        assert!(resolve_loadout_name(&names, "不存在").is_none());
    }

    #[test]
    fn test_now_str_format() {
        let s = now_str();
        // Should be "MM-DD HH:MM" format
        assert!(s.contains('-'));
        assert!(s.contains(':'));
        assert!(s.len() >= 11);
    }
}

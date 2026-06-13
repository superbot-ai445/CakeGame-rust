/// 超界强化系统 — 基于 Equip_ES_Info 表
/// 超界强化可以进一步提升装备的属性
/// 超界装备图鉴 — 基于 Equip_ES_Register 表
use crate::core::*;
use crate::db::Database;
use std::collections::BTreeMap;

/// 查看超界 — 显示可进行超界强化的装备及材料需求
pub fn cmd_view_super_enhance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare(
        "SELECT Name, LV, XHJB, XHZS, XHWP, ADDSX, SuccessV FROM Equip_ES_Info ORDER BY Name, CAST(LV AS INTEGER)",
    ) {
        Ok(s) => s,
        Err(e) => return format!("{}\n⚠️ 查询超界信息失败: {}", prefix, e),
    };

    let rows: Vec<(String, String, String, String, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
                row.get::<_, String>(6).unwrap_or_default(),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    drop(stmt);
    drop(conn);

    if rows.is_empty() {
        return format!("{}\n📦 暂无可超界强化的装备", prefix);
    }

    let mut out = format!("{}\n⚔️ 【超界强化列表】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for (i, (name, lv, gold, diamond, materials, _attrs_json, success)) in rows.iter().enumerate() {
        let name_decoded = crate::encoding::smart_decode(name);
        let gold_v: i32 = gold.parse().unwrap_or(0);
        let diamond_v: i32 = diamond.parse().unwrap_or(0);
        let materials_decoded = crate::encoding::smart_decode(materials);

        out.push_str(&format!("{}. {} (等级{})\n", i + 1, name_decoded, lv));

        let mut costs = Vec::new();
        if gold_v > 0 {
            costs.push(format!("💰{}金币", gold_v));
        }
        if diamond_v > 0 {
            costs.push(format!("💎{}钻石", diamond_v));
        }
        if !materials_decoded.is_empty() {
            costs.push(format!("📦{}", materials_decoded));
        }
        if !costs.is_empty() {
            out.push_str(&format!("   消耗: {}\n", costs.join(" + ")));
        }
        out.push_str(&format!("   成功率: {}%\n", success));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「超界强化+装备名」进行超界强化\n");
    out
}

/// 超界强化 — 消耗金币/钻石/材料对装备进行超界强化
pub fn cmd_super_enhance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let equip_name = args.trim();

    if equip_name.is_empty() {
        return format!("{}\n⚠️ 请指定装备名，例: 超界强化+【超界】光寒圣剑", prefix);
    }

    let conn = db.lock_conn();

    let es_info = conn.query_row(
        "SELECT LV, XHJB, XHZS, XHWP, ADDSX, SuccessV FROM Equip_ES_Info WHERE Name=?1",
        [equip_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
            ))
        },
    );

    let (_target_lv, cost_gold, cost_diamond, cost_materials, attrs_json, success_rate) = match es_info {
        Ok(info) => info,
        Err(_) => {
            return format!(
                "{}\n⚠️ 未找到装备 [{}] 的超界强化信息\n💡 使用「查看超界」查看可强化的装备",
                prefix, equip_name
            );
        }
    };
    drop(conn);

    let gold_cost: i64 = cost_gold.parse().unwrap_or(0);
    let diamond_cost: i64 = cost_diamond.parse().unwrap_or(0);
    let success_pct: i32 = success_rate.parse().unwrap_or(50);

    // 检查并扣除金币
    if gold_cost > 0 {
        let user_gold = db.modify_currency(user_id, CURRENCY_GOLD, "sub", gold_cost);
        if user_gold < 0 {
            db.modify_currency(user_id, CURRENCY_GOLD, "add", gold_cost);
            return format!("{}\n💰 金币不足！需要 {}金币", prefix, gold_cost);
        }
    }

    // 检查并扣除钻石
    if diamond_cost > 0 {
        let user_diamond = db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", diamond_cost);
        if user_diamond < 0 {
            if gold_cost > 0 {
                db.modify_currency(user_id, CURRENCY_GOLD, "add", gold_cost);
            }
            return format!("{}\n💎 钻石不足！需要 {}钻石", prefix, diamond_cost);
        }
    }

    // 检查并扣除材料
    if !cost_materials.is_empty() {
        let materials_decoded = crate::encoding::smart_decode(&cost_materials);
        let mut material_list = Vec::new();
        for mat_req in materials_decoded.split(',') {
            let mat_req = mat_req.trim();
            if mat_req.is_empty() {
                continue;
            }
            let parts: Vec<&str> = mat_req.splitn(2, '*').collect();
            if parts.len() >= 2 {
                let mat_name = parts[0].trim();
                let mat_qty: i32 = parts[1].trim().parse().unwrap_or(1);
                let user_qty = db.knapsack_quantity(user_id, mat_name);
                if user_qty < mat_qty {
                    if gold_cost > 0 {
                        db.modify_currency(user_id, CURRENCY_GOLD, "add", gold_cost);
                    }
                    if diamond_cost > 0 {
                        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", diamond_cost);
                    }
                    return format!(
                        "{}\n📦 材料不足！需要 [{}]×{}，当前拥有 {}",
                        prefix, mat_name, mat_qty, user_qty
                    );
                }
                material_list.push((mat_name.to_string(), mat_qty));
            }
        }
        for (mat_name, mat_qty) in &material_list {
            db.knapsack_remove(user_id, mat_name, *mat_qty);
        }
    }

    // 判定成功率
    let user_seed: i32 = user_id.bytes().map(|b| b as i32).sum();
    let roll = ((user_seed * 37 + chrono::Local::now().timestamp() as i32) % 100).abs();

    if roll >= success_pct {
        return format!(
            "{}\n💥 超界强化失败！\n📦 材料已消耗，装备未变化\n💡 成功率: {}% (本次骰点: {})",
            prefix, success_pct, roll
        );
    }

    // 成功 — 解析属性加成
    let attrs: serde_json::Value = match serde_json::from_str(&attrs_json) {
        Ok(v) => v,
        Err(_) => return format!("{}\n⚠️ 超界强化数据异常，请联系管理员", prefix),
    };

    let mut bonuses = Vec::new();
    let attr_map = [
        ("Add_HP", "生命"),
        ("Add_MP", "魔法"),
        ("Add_Defense", "防御"),
        ("Add_Magic", "魔抗"),
        ("Add_AD", "物攻"),
        ("Add_AP", "魔攻"),
        ("Add_Hit", "命中"),
        ("Add_Dodge", "闪避"),
        ("Add_Crit", "暴击"),
        ("Add_AbsorbHP", "吸血"),
        ("Add_ImmuneDamage", "免伤"),
        ("Add_ADPTV", "物穿值"),
        ("Add_ADPTR", "物穿比"),
        ("Add_APPTR", "法穿比"),
        ("Add_APPTV", "法穿值"),
    ];

    for (json_key, display_name) in &attr_map {
        let val = attrs.get(*json_key).and_then(|v| v.as_str()).unwrap_or("");
        let v: i32 = val.parse().unwrap_or(0);
        if v > 0 {
            db.write_user_data(user_id, &format!("super_enhance_{}", json_key), &v.to_string());
            bonuses.push(format!("{}+{}", display_name, v));
        }
    }

    let equip_decoded = crate::encoding::smart_decode(equip_name);
    let mut out = format!("{}\n✨ 超界强化成功！\n", prefix);
    out.push_str(&format!("📦 装备: {}\n", equip_decoded));
    if !bonuses.is_empty() {
        out.push_str(&format!("📈 属性加成: {}\n", bonuses.join(" ")));
    }
    out.push_str(&format!("🎯 成功率: {}% (骰点: {})\n", success_pct, roll));
    out.push_str("💡 使用「查看角色」查看当前总属性\n");

    out
}

/// 超界装备图鉴 — 展示所有已登记的超界装备及其隐藏名称
/// 读取 Equip_ES_Register 表 (ID, Name, QName, LV)
pub fn cmd_es_codex(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare("SELECT ID, Name, QName, LV FROM Equip_ES_Register ORDER BY CAST(ID AS INTEGER)")
    {
        Ok(s) => s,
        Err(e) => return format!("{}\n⚠️ 查询超界图鉴失败: {}", prefix, e),
    };

    let rows: Vec<(i32, String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i32>(0).unwrap_or(0),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    drop(stmt);
    drop(conn);

    if rows.is_empty() {
        return format!("{}\n📦 暂无超界装备登记数据", prefix);
    }

    // 如果指定了装备名，显示详情
    let query = args.trim();
    if !query.is_empty() {
        let filtered: Vec<_> = rows
            .iter()
            .filter(|(_, name, qname, _)| {
                let name_decoded = crate::encoding::smart_decode(name);
                let qname_decoded = crate::encoding::smart_decode(qname);
                name_decoded.contains(query) || qname_decoded.contains(query)
            })
            .collect();

        if filtered.is_empty() {
            return format!(
                "{}\n⚠️ 未找到包含 [{}] 的超界装备\n💡 使用「超界图鉴」查看所有装备",
                prefix, query
            );
        }

        let mut out = format!("{}\n🔍 【超界装备详情】{}\n━━━━━━━━━━━━━━━━━━━━\n", prefix, query);

        for (id, name, qname, lv) in &filtered {
            let name_decoded = crate::encoding::smart_decode(name);
            let qname_decoded = crate::encoding::smart_decode(qname);
            // 清理 <HIDDEN:N> 前缀
            let clean_qname = if let Some(pos) = qname_decoded.find('>') {
                &qname_decoded[pos + 1..]
            } else {
                &qname_decoded
            };

            // 检测特殊标记
            let has_stars = clean_qname.contains('★');
            let has_plus = clean_qname.contains("(+");
            let rarity_icon = if has_stars {
                "🌟" // 星级强化版本
            } else if has_plus && !clean_qname.ends_with("(+0)") {
                "✨" // 非零强化版本
            } else {
                "⚔️" // 基础版本
            };

            out.push_str(&format!("{} #{} {}\n", rarity_icon, id, clean_qname));
            out.push_str(&format!("   基础名: {}\n", name_decoded));
            out.push_str(&format!("   等级要求: Lv.{}\n", lv));
            if has_stars {
                out.push_str("   🔖 特殊星级版本 (稀有)\n");
            }
        }

        out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
        return out;
    }

    // 概览模式：按基础装备名分组
    let mut groups: BTreeMap<String, Vec<&(i32, String, String, String)>> = BTreeMap::new();
    for row in &rows {
        let name_decoded = crate::encoding::smart_decode(&row.1);
        groups.entry(name_decoded).or_default().push(row);
    }

    let mut out = format!("{}\n📚 【超界装备图鉴】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    out.push_str(&format!(
        "共 {} 件超界装备登记，{} 种武器类型\n\n",
        rows.len(),
        groups.len()
    ));

    let weapon_icons = ["🗡️", "🔨", "🔱", "⚡", "🗡️"];
    for (idx, (name, entries)) in groups.iter().enumerate() {
        let icon = weapon_icons.get(idx).unwrap_or(&"⚔️");
        let min_lv = entries
            .iter()
            .filter_map(|e| e.3.parse::<i32>().ok())
            .min()
            .unwrap_or(0);
        let max_lv = entries
            .iter()
            .filter_map(|e| e.3.parse::<i32>().ok())
            .max()
            .unwrap_or(0);

        // 统计特殊版本
        let star_count = entries
            .iter()
            .filter(|(_, _, qname, _)| {
                let decoded = crate::encoding::smart_decode(qname);
                decoded.contains('★')
            })
            .count();

        let lv_range = if min_lv == max_lv {
            format!("Lv.{}", min_lv)
        } else {
            format!("Lv.{}-{}", min_lv, max_lv)
        };

        out.push_str(&format!("{} {} ({}件, {})\n", icon, name, entries.len(), lv_range));
        if star_count > 0 {
            out.push_str(&format!("   🌟 含{}个星级强化版本\n", star_count));
        }

        // 显示前3个隐藏名称
        for (i, (_, _, qname, _)) in entries.iter().take(3).enumerate() {
            let qname_decoded = crate::encoding::smart_decode(qname);
            let clean = if let Some(pos) = qname_decoded.find('>') {
                &qname_decoded[pos + 1..]
            } else {
                &qname_decoded
            };
            let marker = if clean.contains('★') { "🌟" } else { "  " };
            out.push_str(&format!("   {} #{}: {}\n", marker, entries[i].0, clean));
        }
        if entries.len() > 3 {
            out.push_str(&format!("   ... 还有{}件\n", entries.len() - 3));
        }
        out.push('\n');
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「超界图鉴+装备名」查看详细信息\n");
    out
}

#[allow(dead_code)]
/// 属性显示名映射 (JSON key → 中文名)
pub const ATTR_DISPLAY_MAP: &[(&str, &str)] = &[
    ("Add_HP", "生命"),
    ("Add_MP", "魔法"),
    ("Add_Defense", "防御"),
    ("Add_Magic", "魔抗"),
    ("Add_AD", "物攻"),
    ("Add_AP", "魔攻"),
    ("Add_Hit", "命中"),
    ("Add_Dodge", "闪避"),
    ("Add_Crit", "暴击"),
    ("Add_AbsorbHP", "吸血"),
    ("Add_ImmuneDamage", "免伤"),
    ("Add_ADPTV", "物穿值"),
    ("Add_ADPTR", "物穿比"),
    ("Add_APPTR", "法穿比"),
    ("Add_APPTV", "法穿值"),
];

#[allow(dead_code)]
/// 清理隐藏名称中的 <HIDDEN:N> 前缀
pub fn clean_hidden_name(qname: &str) -> &str {
    if let Some(pos) = qname.find('>') {
        &qname[pos + 1..]
    } else {
        qname
    }
}

#[allow(dead_code)]
/// 检测装备是否为星级版本
pub fn is_star_version(qname: &str) -> bool {
    qname.contains('★')
}

#[allow(dead_code)]
/// 检测装备是否为非零强化版本
pub fn is_plus_version(qname: &str) -> bool {
    qname.contains("(+") && !qname.ends_with("(+0)")
}

#[allow(dead_code)]
/// 获取装备稀有度图标
pub fn get_rarity_icon(qname: &str) -> &'static str {
    if is_star_version(qname) {
        "🌟"
    } else if is_plus_version(qname) {
        "✨"
    } else {
        "⚔️"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr_display_map_count() {
        assert_eq!(ATTR_DISPLAY_MAP.len(), 15);
    }

    #[test]
    fn test_attr_display_map_all_unique() {
        let keys: Vec<&str> = ATTR_DISPLAY_MAP.iter().map(|(k, _)| *k).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len());
    }

    #[test]
    fn test_attr_display_map_values_non_empty() {
        for (key, val) in ATTR_DISPLAY_MAP {
            assert!(!val.is_empty(), "Display name for {} is empty", key);
        }
    }

    #[test]
    fn test_clean_hidden_name_with_prefix() {
        assert_eq!(clean_hidden_name("<HIDDEN:3>光寒圣剑"), "光寒圣剑");
        assert_eq!(clean_hidden_name("<HIDDEN:1>测试"), "测试");
    }

    #[test]
    fn test_clean_hidden_name_without_prefix() {
        assert_eq!(clean_hidden_name("普通装备"), "普通装备");
        assert_eq!(clean_hidden_name(""), "");
    }

    #[test]
    fn test_is_star_version() {
        assert!(is_star_version("光寒圣剑★"));
        assert!(is_star_version("★强化版本"));
        assert!(!is_star_version("普通版本"));
    }

    #[test]
    fn test_is_plus_version() {
        assert!(is_plus_version("光寒圣剑(+5)"));
        assert!(!is_plus_version("光寒圣剑(+0)"));
        assert!(!is_plus_version("普通版本"));
    }

    #[test]
    fn test_get_rarity_icon() {
        assert_eq!(get_rarity_icon("光寒圣剑★"), "🌟");
        assert_eq!(get_rarity_icon("光寒圣剑(+5)"), "✨");
        assert_eq!(get_rarity_icon("光寒圣剑(+0)"), "⚔️");
        assert_eq!(get_rarity_icon("普通版本"), "⚔️");
    }
}

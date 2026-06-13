/// CakeGame 装备类型加成系统
/// 来自 Global 表中的重甲/皮甲/板甲/布甲数据 (100条)
/// 穿戴同类型装备可获得累积属性加成
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 装备类型定义
const ARMOR_TYPES: &[&str] = &["重甲", "皮甲", "板甲", "布甲"];

/// 属性槽位
const EQUIP_SLOTS: &[&str] = &["肩膀", "上衣", "腰带", "下装", "鞋子"];

/// 解析属性加成字符串 "0.1|0.2|0.1|0.1|0.1|0.1|0.5|0.1|1"
fn parse_attr_bonuses(data: &str) -> Vec<f64> {
    data.split('|').filter_map(|s| s.trim().parse::<f64>().ok()).collect()
}

/// 装备类型加成结果
#[derive(Default, Debug)]
#[allow(dead_code)]
pub struct ArmorTypeBonus {
    pub hp: i32,
    pub mp: i32,
    pub ad: i32,
    pub ap: i32,
    pub defense: i32,
    pub magic_res: i32,
    pub hit: i32,
    pub dodge: i32,
    pub crit: i32,
    /// 匹配的装备类型名称
    pub matched_type: String,
    /// 匹配的装备数量
    pub matched_count: usize,
    /// 总共可匹配的装备数量
    pub total_slots: usize,
}

/// 从 Global 表获取某个装备类型在指定槽位的装备名列表
fn get_type_equip_names(db: &Database, armor_type: &str, slot: &str) -> Vec<String> {
    let key = format!("ext.zblxzb.set{}", slot);
    let data = db.global_get(armor_type, &key);
    if data.is_empty() {
        return Vec::new();
    }
    data.split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 从 Global 表获取某个装备类型在指定槽位的属性加成
fn get_type_attr_bonus(db: &Database, armor_type: &str, slot: &str) -> Vec<f64> {
    let key = format!("ext.zblxts.set{}", slot);
    let data = db.global_get(armor_type, &key);
    parse_attr_bonuses(&data)
}

/// 获取某个装备类型的等级要求
fn get_type_level_req(db: &Database, armor_type: &str, slot: &str) -> i32 {
    let key = format!("ext.zblxlv.set{}", slot);
    db.global_get(armor_type, &key).parse().unwrap_or(0)
}

/// 获取某个装备类型在指定槽位的职业要求
fn get_type_occupation(db: &Database, armor_type: &str, slot: &str) -> String {
    let key = format!("ext.zblxzy.set{}", slot);
    db.global_get(armor_type, &key)
}

/// 找到用户匹配度最高的装备类型，返回 (类型名, 匹配数, 总槽位数)
fn find_best_armor_type(
    db: &Database,
    user_id: &str,
    equipped_by_slot: &std::collections::HashMap<String, String>,
) -> (String, usize, usize) {
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let user_occ = db.read_basic(user_id, ITEM_OCCUPATION);

    let mut best_type = String::new();
    let mut best_count = 0usize;
    let mut best_total = 0usize;

    for &armor_type in ARMOR_TYPES {
        let mut match_count = 0usize;
        let mut total_slots = 0usize;

        for &slot in EQUIP_SLOTS {
            let equip_names = get_type_equip_names(db, armor_type, slot);
            if equip_names.is_empty() {
                continue;
            }
            total_slots += 1;

            // 检查等级要求
            let level_req = get_type_level_req(db, armor_type, slot);
            if level_req > 0 && user_level < level_req {
                continue;
            }

            // 检查职业要求
            let occ_req = get_type_occupation(db, armor_type, slot);
            if !occ_req.is_empty() && !occ_req.contains(&user_occ) {
                continue;
            }

            // 检查该槽位是否穿戴了匹配的装备
            if let Some(equipped_name) = equipped_by_slot.get(slot) {
                if equip_names
                    .iter()
                    .any(|name| equipped_name.contains(name) || name.contains(equipped_name))
                {
                    match_count += 1;
                }
            }
        }

        if match_count > best_count {
            best_count = match_count;
            best_type = armor_type.to_string();
            best_total = total_slots;
        }
    }

    (best_type, best_count, best_total)
}

/// 收集已穿戴装备名称（按槽位）
fn collect_equipped_by_slot(db: &Database, user_id: &str) -> std::collections::HashMap<String, String> {
    let equips = db.equip_all(user_id);
    let mut map = std::collections::HashMap::new();
    for eq in &equips {
        map.insert(eq.slot.clone(), eq.name.clone());
    }
    map
}

/// 计算用户装备类型加成
pub fn calc_armor_type_bonuses(db: &Database, user_id: &str) -> ArmorTypeBonus {
    let equipped_by_slot = collect_equipped_by_slot(db, user_id);
    if equipped_by_slot.is_empty() {
        return ArmorTypeBonus::default();
    }

    let (best_type, best_count, best_total) = find_best_armor_type(db, user_id, &equipped_by_slot);
    if best_count == 0 {
        return ArmorTypeBonus::default();
    }

    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let user_occ = db.read_basic(user_id, ITEM_OCCUPATION);

    // 获取用户基础属性（用于百分比加成计算）
    let base_hp: i32 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(0);
    let base_mp: i32 = db.read_basic(user_id, ITEM_MP).parse().unwrap_or(0);
    let base_ad: i32 = db.read_basic(user_id, ITEM_AD).parse().unwrap_or(0);
    let base_ap: i32 = db.read_basic(user_id, ITEM_AP).parse().unwrap_or(0);
    let base_def: i32 = db.read_basic(user_id, ITEM_DEFENSE).parse().unwrap_or(0);
    let base_mres: i32 = db.read_basic(user_id, ITEM_MAGIC_RES).parse().unwrap_or(0);
    let base_hit: i32 = db.read_basic(user_id, ITEM_HIT).parse().unwrap_or(0);
    let base_dodge: i32 = db.read_basic(user_id, ITEM_DODGE).parse().unwrap_or(0);
    let base_crit: i32 = db.read_basic(user_id, ITEM_CRIT).parse().unwrap_or(0);

    let mut bonus = ArmorTypeBonus {
        matched_type: best_type.clone(),
        matched_count: best_count,
        total_slots: best_total,
        ..Default::default()
    };

    // 遍历匹配类型的所有槽位，计算已匹配装备的属性加成
    for &slot in EQUIP_SLOTS {
        let equip_names = get_type_equip_names(db, &best_type, slot);
        if equip_names.is_empty() {
            continue;
        }

        let level_req = get_type_level_req(db, &best_type, slot);
        if level_req > 0 && user_level < level_req {
            continue;
        }

        let occ_req = get_type_occupation(db, &best_type, slot);
        if !occ_req.is_empty() && !occ_req.contains(&user_occ) {
            continue;
        }

        if let Some(equipped_name) = equipped_by_slot.get(slot) {
            if equip_names
                .iter()
                .any(|name| equipped_name.contains(name) || name.contains(equipped_name))
            {
                let attrs = get_type_attr_bonus(db, &best_type, slot);
                // attrs: [HP%, MP%, AD%, AP%, Def%, MRes%, Hit%, Dodge%, Crit%]
                if attrs.len() >= 9 {
                    bonus.hp += (base_hp as f64 * attrs[0]) as i32;
                    bonus.mp += (base_mp as f64 * attrs[1]) as i32;
                    bonus.ad += (base_ad as f64 * attrs[2]) as i32;
                    bonus.ap += (base_ap as f64 * attrs[3]) as i32;
                    bonus.defense += (base_def as f64 * attrs[4]) as i32;
                    bonus.magic_res += (base_mres as f64 * attrs[5]) as i32;
                    bonus.hit += (base_hit as f64 * attrs[6]) as i32;
                    bonus.dodge += (base_dodge as f64 * attrs[7]) as i32;
                    bonus.crit += (base_crit as f64 * attrs[8]) as i32;
                }
            }
        }
    }

    bonus
}

/// 格式化装备类型加成摘要
fn format_bonus_summary(bonus: &ArmorTypeBonus) -> String {
    let mut parts = Vec::new();
    if bonus.hp > 0 {
        parts.push(format!("生命+{}", bonus.hp));
    }
    if bonus.mp > 0 {
        parts.push(format!("魔法+{}", bonus.mp));
    }
    if bonus.ad > 0 {
        parts.push(format!("物攻+{}", bonus.ad));
    }
    if bonus.ap > 0 {
        parts.push(format!("魔攻+{}", bonus.ap));
    }
    if bonus.defense > 0 {
        parts.push(format!("防御+{}", bonus.defense));
    }
    if bonus.magic_res > 0 {
        parts.push(format!("魔抗+{}", bonus.magic_res));
    }
    if bonus.hit > 0 {
        parts.push(format!("命中+{}", bonus.hit));
    }
    if bonus.dodge > 0 {
        parts.push(format!("闪避+{}", bonus.dodge));
    }
    if bonus.crit > 0 {
        parts.push(format!("暴击+{}", bonus.crit));
    }
    parts.join(" ")
}

/// 查看装备类型系统信息
pub fn cmd_view_armor_types(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if !args.is_empty() {
        return view_armor_type_detail(db, &prefix, user_id, args);
    }

    // 列出所有装备类型
    let mut r = format!("{}\n═══ 装备类型系统 ═══\n穿戴同类型装备可获得属性加成！", prefix);

    for &armor_type in ARMOR_TYPES {
        let mut slot_count = 0;
        let mut occs = std::collections::HashSet::new();
        for &slot in EQUIP_SLOTS {
            let equip_names = get_type_equip_names(db, armor_type, slot);
            if !equip_names.is_empty() {
                slot_count += 1;
            }
            let occ = get_type_occupation(db, armor_type, slot);
            if !occ.is_empty() {
                for o in occ.split('|') {
                    occs.insert(o.trim().to_string());
                }
            }
        }
        let occ_str = if occs.is_empty() {
            "全职业".to_string()
        } else {
            let mut v: Vec<_> = occs.into_iter().collect();
            v.sort();
            v.join("/")
        };
        r.push_str(&format!("\n▸ {} ({}个部位) 适用: {}", armor_type, slot_count, occ_str));
    }

    // 显示当前用户的装备类型匹配情况
    let equipped_by_slot = collect_equipped_by_slot(db, user_id);
    if !equipped_by_slot.is_empty() {
        let (matched_type, matched_count, total_slots) = find_best_armor_type(db, user_id, &equipped_by_slot);
        if matched_count > 0 {
            let bonus = calc_armor_type_bonuses(db, user_id);
            r.push_str(&format!(
                "\n\n═══ 你的装备类型 ═══\n当前穿戴: {} ({}/{})",
                matched_type, matched_count, total_slots
            ));
            let summary = format_bonus_summary(&bonus);
            if !summary.is_empty() {
                r.push_str(&format!("\n属性加成: {}", summary));
            }
        } else {
            r.push_str("\n\n你当前没有穿戴同类型的装备。");
        }
    }

    r.push_str("\n\n发送 '装备类型+类型名' 查看详情");
    r
}

/// 查看特定装备类型详情
fn view_armor_type_detail(db: &Database, prefix: &str, _user_id: &str, args: &str) -> String {
    let armor_type = if args.contains("重甲") {
        "重甲"
    } else if args.contains("皮甲") {
        "皮甲"
    } else if args.contains("板甲") {
        "板甲"
    } else if args.contains("布甲") {
        "布甲"
    } else {
        return format!("{}\n未找到装备类型 [{}]。\n可选: 重甲/皮甲/板甲/布甲", prefix, args);
    };

    let mut r = format!("{}\n═══ 装备类型: {} ═══", prefix, armor_type);

    // 获取职业要求
    let mut occs = std::collections::HashSet::new();
    for &slot in EQUIP_SLOTS {
        let occ = get_type_occupation(db, armor_type, slot);
        if !occ.is_empty() {
            for o in occ.split('|') {
                occs.insert(o.trim().to_string());
            }
        }
    }
    if !occs.is_empty() {
        let mut v: Vec<_> = occs.into_iter().collect();
        v.sort();
        r.push_str(&format!("\n适用职业: {}", v.join("/")));
    }

    r.push_str("\n\n═══ 各部位信息 ═══");
    let labels = ["生命", "魔法", "物攻", "魔攻", "防御", "魔抗", "命中", "闪避", "暴击"];

    for &slot in EQUIP_SLOTS {
        let equip_names = get_type_equip_names(db, armor_type, slot);
        if equip_names.is_empty() {
            continue;
        }
        let level_req = get_type_level_req(db, armor_type, slot);
        let attrs = get_type_attr_bonus(db, armor_type, slot);

        r.push_str(&format!("\n\n▸ {}", slot));
        if level_req > 0 {
            r.push_str(&format!(" (需求等级{})", level_req));
        }
        r.push_str(&format!("\n  装备: {}", equip_names.join(" / ")));
        if attrs.len() >= 9 {
            let mut bonuses = Vec::new();
            for (i, val) in attrs.iter().enumerate() {
                if *val > 0.0 && i < labels.len() {
                    bonuses.push(format!("{}+{}%", labels[i], (val * 100.0) as i32));
                }
            }
            if !bonuses.is_empty() {
                r.push_str(&format!("\n  加成: {}", bonuses.join(" ")));
            }
        }
    }

    r.push_str("\n\n穿戴同类型装备越多，加成越高！");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attr_bonuses_basic() {
        let result = parse_attr_bonuses("0.1|0.2|0.3");
        assert_eq!(result, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_parse_attr_bonuses_full() {
        let result = parse_attr_bonuses("0.1|0.2|0.1|0.1|0.1|0.1|0.5|0.1|1");
        assert_eq!(result.len(), 9);
        assert_eq!(result[0], 0.1);
        assert_eq!(result[8], 1.0);
    }

    #[test]
    fn test_parse_attr_bonuses_empty() {
        let result = parse_attr_bonuses("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_attr_bonuses_invalid() {
        let result = parse_attr_bonuses("abc|def");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_attr_bonuses_mixed() {
        let result = parse_attr_bonuses("0.1|abc|0.3");
        assert_eq!(result, vec![0.1, 0.3]);
    }

    #[test]
    fn test_armor_types_count() {
        assert_eq!(ARMOR_TYPES.len(), 4);
        assert!(ARMOR_TYPES.contains(&"重甲"));
        assert!(ARMOR_TYPES.contains(&"皮甲"));
        assert!(ARMOR_TYPES.contains(&"板甲"));
        assert!(ARMOR_TYPES.contains(&"布甲"));
    }

    #[test]
    fn test_equip_slots_count() {
        assert_eq!(EQUIP_SLOTS.len(), 5);
        assert!(EQUIP_SLOTS.contains(&"肩膀"));
        assert!(EQUIP_SLOTS.contains(&"上衣"));
        assert!(EQUIP_SLOTS.contains(&"腰带"));
        assert!(EQUIP_SLOTS.contains(&"下装"));
        assert!(EQUIP_SLOTS.contains(&"鞋子"));
    }

    #[test]
    fn test_armor_type_bonus_default() {
        let bonus = ArmorTypeBonus::default();
        assert_eq!(bonus.hp, 0);
        assert_eq!(bonus.mp, 0);
        assert_eq!(bonus.ad, 0);
        assert_eq!(bonus.ap, 0);
        assert_eq!(bonus.defense, 0);
        assert_eq!(bonus.magic_res, 0);
        assert_eq!(bonus.hit, 0);
        assert_eq!(bonus.dodge, 0);
        assert_eq!(bonus.crit, 0);
        assert!(bonus.matched_type.is_empty());
        assert_eq!(bonus.matched_count, 0);
        assert_eq!(bonus.total_slots, 0);
    }

    #[test]
    fn test_format_bonus_summary_empty() {
        let bonus = ArmorTypeBonus::default();
        let summary = format_bonus_summary(&bonus);
        assert!(summary.is_empty());
    }

    #[test]
    fn test_format_bonus_summary_with_values() {
        let bonus = ArmorTypeBonus {
            hp: 100,
            ad: 50,
            crit: 10,
            ..Default::default()
        };
        let summary = format_bonus_summary(&bonus);
        assert!(summary.contains("生命+100"));
        assert!(summary.contains("物攻+50"));
        assert!(summary.contains("暴击+10"));
        assert!(!summary.contains("魔法"));
    }
}

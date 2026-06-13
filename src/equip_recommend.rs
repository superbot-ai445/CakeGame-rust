/// CakeGame 装备推荐系统
/// 基于玩家等级和职业，推荐各部位最优装备
/// 使用装备评分公式筛选最佳候选，帮助玩家明确装备升级目标
///
/// 数据来源: Config_Goods 表 (345物品) + Equip_Register (当前装备)
/// 指令: 推荐装备 / 推荐装备+武器 / 推荐装备+头盔 等
use crate::core::{EquipInfo, ItemData};
use crate::db::Database;
use crate::equip_score;
use crate::user;

/// 装备槽位列表
const EQUIP_SLOTS: &[&str] = &[
    "武器", "头盔", "铠甲", "护腿", "靴子", "项链", "戒指", "翅膀", "时装", "称号",
];

/// 槽位emoji
fn slot_emoji(slot: &str) -> &'static str {
    match slot {
        "武器" => "⚔️",
        "头盔" => "🪖",
        "铠甲" => "🛡️",
        "护腿" => "👖",
        "靴子" => "👢",
        "项链" => "📿",
        "戒指" => "💍",
        "翅膀" => "🪽",
        "时装" => "👗",
        "称号" => "🏅",
        _ => "📦",
    }
}

/// 从EquipInfo计算评分
fn calc_equip_info_score(info: &EquipInfo) -> f64 {
    let data = ItemData {
        slot_name: info.slot.clone(),
        occupation: String::new(),
        use_lv: 0,
        add_hp: info.add_hp,
        add_mp: info.add_mp,
        add_defense: info.add_defense,
        add_magic: info.add_magic,
        add_ad: info.add_ad,
        add_ap: info.add_ap,
        add_hit: info.add_hit,
        add_dodge: info.add_dodge,
        add_crit: info.add_crit,
        add_absorb_hp: info.add_absorb_hp,
        add_adptv: info.add_adptv,
        add_adptr: info.add_adptr,
        add_apptr: info.add_apptr,
        add_apptv: info.add_apptv,
        add_immune_damage: info.add_immune_damage,
        special_type: info.special_type.clone(),
        special_value: info.special_value,
        ..ItemData::default()
    };
    equip_score::calc_equip_score(&data)
}

/// 推荐装备候选
struct EquipCandidate {
    name: String,
    slot: String,
    quality_stars: String,
    score: f64,
    level_req: i32,
    occupation: String,
}

/// 从 Config_Goods 获取所有装备候选 (等级过滤)
fn get_all_equipment_candidates(db: &Database, user_level: i32) -> Vec<EquipCandidate> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Name, LtemData FROM Config_Goods WHERE Type='Equip'") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0).unwrap_or_default();
        let data_str: String = row.get(1).unwrap_or_default();
        Ok((name, data_str))
    });

    match rows {
        Ok(mapped) => mapped
            .filter_map(|r| r.ok())
            .filter_map(|(name, data_str)| {
                let data: ItemData = serde_json::from_str(&data_str).ok()?;
                // 等级过滤
                if data.use_lv > user_level {
                    return None;
                }
                // 只推荐有属性加成的装备
                let total = data.add_ad
                    + data.add_ap
                    + data.add_hp
                    + data.add_mp
                    + data.add_defense
                    + data.add_magic
                    + data.add_hit
                    + data.add_dodge
                    + data.add_crit;
                if total <= 0 {
                    return None;
                }
                let slot = if data.slot_name.is_empty() {
                    return None;
                } else {
                    data.slot_name.clone()
                };
                let stars = equip_score::quality_stars(&name);
                let score = equip_score::calc_equip_score(&data);
                Some(EquipCandidate {
                    name,
                    slot,
                    quality_stars: stars.to_string(),
                    score,
                    level_req: data.use_lv,
                    occupation: data.occupation,
                })
            })
            .collect(),
        Err(_) => vec![],
    }
}

/// 检查职业兼容性
fn occupation_compatible(equip_occ: &str, player_occ: &str) -> bool {
    if equip_occ.is_empty() || equip_occ == "通用" || equip_occ == "全职业" || equip_occ == "[NULL]" {
        return true;
    }
    equip_occ.contains(player_occ) || player_occ.contains(equip_occ)
}

/// 推荐装备 — 为玩家推荐各部位最优装备
pub fn cmd_recommend_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let user_level: i32 = db.read_basic(user_id, "P_Lv").parse().unwrap_or(1);
    let player_occ = db.read_basic(user_id, "P_Occupation");

    if player_occ.is_empty() {
        return format!("{}您还未注册！请先发送 注册+昵称 进行注册。", prefix);
    }

    // 获取当前已装备
    let equipped = db.equip_all(user_id);
    let equipped_map: std::collections::HashMap<&str, &EquipInfo> =
        equipped.iter().map(|e| (e.slot.as_str(), e)).collect();
    let equipped_names: Vec<&str> = equipped.iter().map(|e| e.name.as_str()).collect();

    // 获取所有候选装备
    let candidates = get_all_equipment_candidates(db, user_level);

    let target_slot = args.trim();

    if target_slot.is_empty() {
        // 显示所有部位推荐
        let mut out = format!(
            "{}\n═══ 🎯 装备推荐 ═══\n等级: {} | 职业: {}\n",
            prefix, user_level, player_occ
        );

        for &slot in EQUIP_SLOTS {
            let mut slot_candidates: Vec<&EquipCandidate> = candidates
                .iter()
                .filter(|c| c.slot == slot && occupation_compatible(&c.occupation, &player_occ))
                .collect();
            slot_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            let emoji = slot_emoji(slot);

            // 当前装备
            out.push_str(&format!("\n{} {}: ", emoji, slot));
            if let Some(cur) = equipped_map.get(slot) {
                let cur_score = calc_equip_info_score(cur);
                out.push_str(&format!("当前 [{}] ({}分)", cur.name, cur_score as i32));
            } else {
                out.push_str("未装备");
            }

            // 推荐 (排除已装备)
            let top3: Vec<&&EquipCandidate> = slot_candidates
                .iter()
                .filter(|c| !equipped_names.contains(&c.name.as_str()))
                .take(3)
                .collect();

            if top3.is_empty() {
                if slot_candidates.is_empty() {
                    out.push_str("\n  └─ 暂无可推荐装备");
                } else {
                    out.push_str("\n  └─ 已拥有该部位最优装备 ✅");
                }
            } else {
                for (i, cand) in top3.iter().enumerate() {
                    let marker = if i == 0 { " 👑推荐" } else { "" };
                    let connector = if i == 0 {
                        "├─"
                    } else if i == top3.len() - 1 {
                        "└─"
                    } else {
                        "├─"
                    };
                    out.push_str(&format!(
                        "\n  {} {}. [{}] Lv.{} {} ({}分){}",
                        connector,
                        i + 1,
                        cand.name,
                        cand.level_req,
                        cand.quality_stars,
                        cand.score as i32,
                        marker,
                    ));
                }
            }
        }

        out.push_str("\n\n💡 输入'推荐装备+部位名'查看该部位详细推荐");
        out.push_str("\n📌 例如: 推荐装备+武器");
        return out;
    }

    // 指定部位详细推荐
    let slot = EQUIP_SLOTS
        .iter()
        .find(|s| **s == target_slot || target_slot.contains(**s));
    let slot = match slot {
        Some(s) => *s,
        None => {
            return format!(
                "{}\n❌ 未找到部位 '{}'\n可选部位: {}",
                prefix,
                target_slot,
                EQUIP_SLOTS.join("、")
            );
        }
    };

    let mut slot_candidates: Vec<&EquipCandidate> = candidates
        .iter()
        .filter(|c| c.slot == slot && occupation_compatible(&c.occupation, &player_occ))
        .collect();

    if slot_candidates.is_empty() {
        return format!(
            "{}\n{} {} — 暂无适合您等级({})和职业({})的装备",
            prefix,
            slot_emoji(slot),
            slot,
            user_level,
            player_occ
        );
    }

    slot_candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let emoji = slot_emoji(slot);
    let mut out = format!(
        "{}\n═══ {} {} 推荐装备 TOP10 ═══\n等级: {} | 职业: {}\n",
        prefix, emoji, slot, user_level, player_occ
    );

    // 当前装备
    if let Some(cur) = equipped_map.get(slot) {
        let cur_score = calc_equip_info_score(cur);
        out.push_str(&format!("\n📌 当前装备: [{}] ({}分)\n", cur.name, cur_score as i32));
    }

    let max_display = 10.min(slot_candidates.len());
    for (i, cand) in slot_candidates.iter().take(max_display).enumerate() {
        let owned = if equipped_names.contains(&cand.name.as_str()) {
            " ✅已装备"
        } else {
            ""
        };
        let upgrade = if let Some(cur) = equipped_map.get(slot) {
            let cur_score = calc_equip_info_score(cur);
            if cand.score > cur_score {
                let pct = ((cand.score - cur_score) / cur_score * 100.0) as i32;
                format!(" 📈+{}%", pct)
            } else {
                String::new()
            }
        } else {
            " 📈新装备".to_string()
        };

        out.push_str(&format!(
            "\n{}. [{}] Lv.{} {} ({}分){}{}",
            i + 1,
            cand.name,
            cand.level_req,
            cand.quality_stars,
            cand.score as i32,
            upgrade,
            owned,
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_emoji_all_slots() {
        assert_eq!(slot_emoji("武器"), "⚔️");
        assert_eq!(slot_emoji("头盔"), "🪖");
        assert_eq!(slot_emoji("铠甲"), "🛡️");
        assert_eq!(slot_emoji("护腿"), "👖");
        assert_eq!(slot_emoji("靴子"), "👢");
        assert_eq!(slot_emoji("项链"), "📿");
        assert_eq!(slot_emoji("戒指"), "💍");
        assert_eq!(slot_emoji("翅膀"), "🪽");
        assert_eq!(slot_emoji("时装"), "👗");
        assert_eq!(slot_emoji("称号"), "🏅");
        assert_eq!(slot_emoji("未知"), "📦");
    }

    #[test]
    fn test_occupation_compatible() {
        assert!(occupation_compatible("", "勇者"));
        assert!(occupation_compatible("通用", "勇者"));
        assert!(occupation_compatible("全职业", "魔法师"));
        assert!(occupation_compatible("[NULL]", "勇者"));
        assert!(occupation_compatible("勇者", "勇者"));
        assert!(occupation_compatible("勇者|御剑师", "勇者"));
        assert!(!occupation_compatible("魔法师", "勇者"));
    }

    #[test]
    fn test_equip_slots_count() {
        assert_eq!(EQUIP_SLOTS.len(), 10);
    }

    #[test]
    fn test_calc_equip_info_score_zero() {
        let info = EquipInfo {
            slot: "武器".to_string(),
            name: "测试".to_string(),
            add_hp: 0,
            add_mp: 0,
            add_defense: 0,
            add_magic: 0,
            add_ad: 0,
            add_ap: 0,
            add_hit: 0,
            add_dodge: 0,
            add_crit: 0,
            add_absorb_hp: 0,
            add_adptv: 0,
            add_adptr: 0,
            add_apptr: 0,
            add_apptv: 0,
            add_immune_damage: 0,
            special_type: String::new(),
            special_value: 0,
        };
        assert_eq!(calc_equip_info_score(&info), 0.0);
    }

    #[test]
    fn test_calc_equip_info_score_positive() {
        let info = EquipInfo {
            slot: "武器".to_string(),
            name: "测试剑".to_string(),
            add_hp: 100,
            add_mp: 0,
            add_defense: 0,
            add_magic: 0,
            add_ad: 50,
            add_ap: 0,
            add_hit: 0,
            add_dodge: 0,
            add_crit: 0,
            add_absorb_hp: 0,
            add_adptv: 0,
            add_adptr: 0,
            add_apptr: 0,
            add_apptv: 0,
            add_immune_damage: 0,
            special_type: String::new(),
            special_value: 0,
        };
        // score = (50*35 + 100*2.7) / 10 = (1750 + 270) / 10 = 202.0
        let score = calc_equip_info_score(&info);
        assert!(score > 0.0);
        assert!((score - 202.0).abs() < 0.1);
    }

    #[test]
    fn test_occupation_compatible_empty_equipment() {
        // 空职业 = 通用
        assert!(occupation_compatible("", "任何职业"));
    }
}

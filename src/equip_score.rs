/// CakeGame 装备评分系统
/// 为每件装备计算数值评分，帮助玩家快速比较装备优劣
/// 使用战力公式权重: AD×35 + AP×217.5 + HP×2.7 + MP×1.4 + Defense×20 + MagicResistance×18
///                    + Hit×2.25 + Dodge×12 + Crit×60 + ADPTV×20 + APPTV×44.44
use crate::core::ItemData;
use crate::db::Database;
use crate::user;

/// 装备评分详情
pub struct EquipScore {
    pub name: String,
    pub slot: String,
    pub quality: String,
    pub score: f64,
    pub level_req: i32,
    pub occupation: String,
}

/// 计算单件装备的评分
/// 基于战力公式权重，对装备属性加成进行加权求和
pub fn calc_equip_score(data: &ItemData) -> f64 {
    let ad = data.add_ad as f64;
    let ap = data.add_ap as f64;
    let hp = data.add_hp as f64;
    let mp = data.add_mp as f64;
    let def = data.add_defense as f64;
    let mr = data.add_magic as f64;
    let hit = data.add_hit as f64;
    let dodge = data.add_dodge as f64;
    let crit = data.add_crit as f64;
    let adptv = data.add_adptv as f64;
    let apptv = data.add_apptv as f64;
    let absorb = data.add_absorb_hp as f64;
    let immune = data.add_immune_damage as f64;

    // 核心属性权重（来自战力公式）
    let base_score = ad * 35.0
        + ap * 217.5
        + hp * 2.7
        + mp * 1.4
        + def * 20.0
        + mr * 18.0
        + hit * 2.25
        + dodge * 12.0
        + crit * 60.0
        + adptv * 20.0
        + apptv * 44.44;

    // 高级属性加成
    let bonus = absorb * 50.0 + immune * 80.0;

    (base_score + bonus) / 10.0
}

/// 获取装备品质的星级表示
pub fn quality_stars(quality: &str) -> &'static str {
    if quality.contains("超界") {
        "⭐⭐⭐⭐⭐⭐"
    } else if quality.contains("远古") {
        "⭐⭐⭐⭐⭐"
    } else if quality.contains("至臻") || quality.contains("传说") {
        "⭐⭐⭐⭐"
    } else if quality.contains("史诗") {
        "⭐⭐⭐"
    } else if quality.contains("镇魂") || quality.contains("完美") {
        "⭐⭐"
    } else if quality.contains("精良") || quality.contains("稀有") {
        "⭐"
    } else {
        ""
    }
}

/// 获取品质等级数值（用于排序）
#[allow(dead_code)]
pub fn quality_tier(name: &str) -> i32 {
    if name.contains("超界") {
        7
    } else if name.contains("远古") {
        6
    } else if name.contains("至臻") {
        5
    } else if name.contains("史诗") {
        4
    } else if name.contains("镇魂") || name.contains("完美") {
        3
    } else if name.contains("精良") || name.contains("稀有") {
        2
    } else if name.contains("普通") {
        1
    } else {
        0
    }
}

/// 格式化评分为视觉条形图
fn format_score_bar(score: f64, max_score: f64) -> String {
    let ratio = if max_score > 0.0 {
        (score / max_score).min(1.0)
    } else {
        0.0
    };
    let filled = (ratio * 10.0).round() as usize;
    let empty = 10 - filled;
    format!("{}{} {:.0}分", "█".repeat(filled), "░".repeat(empty), score)
}

/// 从数据库读取所有装备并评分
fn score_all_equipment(db: &Database) -> Vec<EquipScore> {
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
                let score = calc_equip_score(&data);
                let quality = quality_stars(&name).to_string();
                Some(EquipScore {
                    name,
                    slot: data.slot_name,
                    quality,
                    score,
                    level_req: data.use_lv,
                    occupation: data.occupation,
                })
            })
            .collect(),
        Err(_) => vec![],
    }
}

/// 查看装备评分 — 列出评分最高的装备
pub fn cmd_equip_score(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    let all_equips = score_all_equipment(db);
    if all_equips.is_empty() {
        return format!("{}\n═══ 装备评分系统 ═══\n暂无装备数据。", prefix);
    }

    // 查看特定装备评分
    if !args.is_empty() {
        let mut found: Vec<&EquipScore> = all_equips.iter().filter(|e| e.name.contains(args)).collect();

        if found.is_empty() {
            return format!("{}\n未找到包含 [{}] 的装备。", prefix, args);
        }

        found.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let max_score = found.first().map(|e| e.score).unwrap_or(1.0);
        let mut r = format!("{}\n═══ 装备评分查询 ═══", prefix);
        for (i, eq) in found.iter().take(10).enumerate() {
            r.push_str(&format!("\n{}. {} {}", i + 1, eq.name, eq.quality));
            r.push_str(&format!("\n   评分: {}", format_score_bar(eq.score, max_score)));
            if !eq.slot.is_empty() {
                r.push_str(&format!(" | 部位: {}", eq.slot));
            }
            if eq.level_req > 0 {
                r.push_str(&format!(" | 需求等级: {}", eq.level_req));
            }
            if !eq.occupation.is_empty() && eq.occupation != "[NULL]" {
                r.push_str(&format!(" | 职业: {}", eq.occupation));
            }
        }
        if found.len() > 10 {
            r.push_str(&format!("\n... 还有 {} 件装备", found.len() - 10));
        }
        return r;
    }

    // 默认: 按部位查看评分排行
    cmd_equip_score_by_slot(db, user_id, "", &all_equips, &prefix)
}

/// 按部位查看装备评分排行
fn cmd_equip_score_by_slot(
    db: &Database,
    user_id: &str,
    slot: &str,
    all_equips: &[EquipScore],
    prefix: &str,
) -> String {
    let user_level: i32 = db.read_basic(user_id, "P_Lv").parse().unwrap_or(1);
    let user_occ = db.read_basic(user_id, "P_Occupation");

    // 收集所有出现的部位
    let mut slots: Vec<String> = all_equips
        .iter()
        .map(|e| e.slot.clone())
        .filter(|s| !s.is_empty())
        .collect::<std::collections::HashSet<String>>()
        .into_iter()
        .collect();
    slots.sort();

    let target_slot = if slot.is_empty() {
        String::new()
    } else {
        slot.to_string()
    };

    let mut r = format!("{}\n═══ 装备评分系统 ═══", prefix);
    r.push_str(&format!("\n当前等级: {} | 职业: {}", user_level, user_occ));
    r.push_str("\n基于战力公式权重计算，评分越高越强。");

    if target_slot.is_empty() {
        // 显示每个部位的最佳装备
        r.push_str("\n\n--- 各部位评分TOP1 ---");
        for slot_name in &slots {
            let mut slot_equips: Vec<&EquipScore> = all_equips
                .iter()
                .filter(|e| e.slot == *slot_name && e.level_req <= user_level)
                .collect();
            slot_equips.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

            if let Some(best) = slot_equips.first() {
                let stars = quality_stars(&best.name);
                r.push_str(&format!(
                    "\n🗡 {} → {} {} ({}分)",
                    slot_name, best.name, stars, best.score as i32
                ));
            }
        }
        r.push_str("\n\n发送 '装备评分+部位名' 查看该部位详细排行");
        r.push_str("\n发送 '装备评分+装备名' 搜索特定装备评分");
        r.push_str(&format!("\n可选部位: {}", slots.join("、")));
    } else {
        // 显示指定部位的排行
        let mut slot_equips: Vec<&EquipScore> = all_equips.iter().filter(|e| e.slot == target_slot).collect();
        slot_equips.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        if slot_equips.is_empty() {
            return format!(
                "{}\n未找到部位 [{}] 的装备。可用部位: {}",
                prefix,
                target_slot,
                slots.join("、")
            );
        }

        let max_score = slot_equips.first().map(|e| e.score).unwrap_or(1.0);
        r.push_str(&format!("\n\n--- {} 评分排行 ---", target_slot));

        for (i, eq) in slot_equips.iter().take(15).enumerate() {
            let can_use = eq.level_req <= user_level;
            let level_tag = if !can_use { " 🔒" } else { "" };
            let occ_tag =
                if !eq.occupation.is_empty() && eq.occupation != "[NULL]" && !eq.occupation.contains(&user_occ) {
                    " ⚠️"
                } else {
                    ""
                };

            r.push_str(&format!(
                "\n{}. {} {} {}{}",
                i + 1,
                eq.name,
                eq.quality,
                level_tag,
                occ_tag
            ));
            r.push_str(&format!("\n   {}", format_score_bar(eq.score, max_score)));
            if eq.level_req > 0 {
                r.push_str(&format!(" Lv.{}", eq.level_req));
            }
        }

        if slot_equips.len() > 15 {
            r.push_str(&format!("\n... 还有 {} 件装备", slot_equips.len() - 15));
        }
        r.push_str("\n🔒=等级不足 ⚠️=职业不符");
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ItemData;

    #[test]
    fn test_calc_equip_score_basic() {
        let data = ItemData {
            add_ad: 10,
            add_hp: 100,
            add_defense: 5,
            ..Default::default()
        };
        let score = calc_equip_score(&data);
        // (10*35 + 100*2.7 + 5*20) / 10 = (350 + 270 + 100) / 10 = 72.0
        assert!((score - 72.0).abs() < 0.1);
    }

    #[test]
    fn test_calc_equip_score_zero() {
        let data = ItemData::default();
        let score = calc_equip_score(&data);
        assert!((score).abs() < 0.1);
    }

    #[test]
    fn test_calc_equip_score_mixed() {
        let data = ItemData {
            add_ad: 50,
            add_ap: 30,
            add_hp: 500,
            add_mp: 200,
            add_defense: 20,
            add_magic: 15,
            add_hit: 100,
            add_dodge: 50,
            add_crit: 10,
            add_adptv: 30,
            add_apptv: 20,
            ..Default::default()
        };
        let score = calc_equip_score(&data);
        assert!(score > 0.0);
        // 应该是高分装备
        assert!(score > 500.0);
    }

    #[test]
    fn test_quality_tier() {
        assert!(quality_tier("【超界】光寒圣剑") > quality_tier("【史诗】秋叶刀"));
        assert!(quality_tier("【史诗】秋叶刀") > quality_tier("【普通】铁剑"));
        assert!(quality_tier("【普通】铁剑") > quality_tier("【劣质】破碎的西洋剑"));
    }

    #[test]
    fn test_quality_stars() {
        assert_eq!(quality_stars("【超界】光寒圣剑"), "⭐⭐⭐⭐⭐⭐");
        assert_eq!(quality_stars("【远古】妖刀村雨"), "⭐⭐⭐⭐⭐");
        assert_eq!(quality_stars("【史诗】秋叶刀"), "⭐⭐⭐");
        assert_eq!(quality_stars("【普通】铁剑"), "");
    }

    #[test]
    fn test_score_bar_format() {
        let bar = format_score_bar(50.0, 100.0);
        assert!(bar.contains("50分"));
        assert!(bar.contains("█"));
    }

    #[test]
    fn test_score_comparison() {
        let weak = ItemData {
            add_ad: 1,
            ..Default::default()
        };
        let strong = ItemData {
            add_ad: 100,
            add_hp: 1000,
            add_defense: 50,
            ..Default::default()
        };
        assert!(calc_equip_score(&strong) > calc_equip_score(&weak));
    }
}

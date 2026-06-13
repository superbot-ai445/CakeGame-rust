use crate::db::Database;
use rusqlite::params;

/// 答题装备数据行类型 (10个String字段)
type ExamGearRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
/// 答题装备完整数据行类型 (19个String字段)
type ExamGearFullRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

/// 查看答题装备 — 显示用户在 ext_examgear_register 中注册的答题装备
pub fn cmd_view_exam_gear(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT SlotName, EquipName, Add_HP, Add_MP, Add_Defense, Add_Magic, Add_AD, Add_AP, \
         Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, Add_ADPTV, Add_ADPTR, \
         Add_APPTR, Add_APPTV, Add_ImmuneDamage, Special_Type, Special_Value \
         FROM ext_examgear_register WHERE User=?1",
    ) {
        Ok(s) => s,
        Err(e) => return format!("⚠️ 查询答题装备失败: {}", e),
    };

    let rows: Vec<ExamGearRow> = stmt
        .query_map(params![user_id], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(), // SlotName
                row.get::<_, String>(1).unwrap_or_default(), // EquipName
                row.get::<_, String>(2).unwrap_or_default(), // Add_HP
                row.get::<_, String>(3).unwrap_or_default(), // Add_MP
                row.get::<_, String>(4).unwrap_or_default(), // Add_Defense
                row.get::<_, String>(5).unwrap_or_default(), // Add_Magic
                row.get::<_, String>(6).unwrap_or_default(), // Add_AD
                row.get::<_, String>(7).unwrap_or_default(), // Add_AP
                row.get::<_, String>(8).unwrap_or_default(), // Add_Hit
                row.get::<_, String>(9).unwrap_or_default(), // Add_Dodge
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    drop(stmt);
    drop(conn);

    if rows.is_empty() {
        return "📋 您没有注册的答题装备。\n💡 答题装备通过答题活动获得，完成答题后可在此查看。".to_string();
    }

    let mut out = String::from("📋 【答题装备列表】\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    for (i, (slot, name, hp, mp, def_, mag, ad, ap, hit, dodge)) in rows.iter().enumerate() {
        let name_decoded = crate::encoding::smart_decode(name);
        let slot_decoded = crate::encoding::smart_decode(slot);
        let hp_v: i32 = hp.parse().unwrap_or(0);
        let mp_v: i32 = mp.parse().unwrap_or(0);
        let def_v: i32 = def_.parse().unwrap_or(0);
        let mag_v: i32 = mag.parse().unwrap_or(0);
        let ad_v: i32 = ad.parse().unwrap_or(0);
        let ap_v: i32 = ap.parse().unwrap_or(0);
        let hit_v: i32 = hit.parse().unwrap_or(0);
        let dodge_v: i32 = dodge.parse().unwrap_or(0);

        out.push_str(&format!(
            "{}. 【{}】{}\n   槽位: {} | 生命+{} 魔法+{} 防御+{} 魔抗+{}\n   物攻+{} 魔攻+{} 命中+{} 闪避+{}\n",
            i + 1,
            slot_decoded,
            name_decoded,
            slot_decoded,
            hp_v,
            mp_v,
            def_v,
            mag_v,
            ad_v,
            ap_v,
            hit_v,
            dodge_v
        ));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「领取答题装备」将答题装备装备到角色上\n");
    out
}

/// 领取答题装备 — 将 ext_examgear_register 中的装备转移到 Equip_Register
pub fn cmd_claim_exam_gear(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 支持指定序号领取，或全部领取
    let target_index: Option<usize> = if args.trim().is_empty() {
        None
    } else {
        args.trim().parse().ok()
    };

    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT SlotName, EquipName, Add_HP, Add_MP, Add_Defense, Add_Magic, Add_AD, Add_AP, \
         Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, Add_ADPTV, Add_ADPTR, \
         Add_APPTR, Add_APPTV, Add_ImmuneDamage, Special_Type, Special_Value \
         FROM ext_examgear_register WHERE User=?1",
    ) {
        Ok(s) => s,
        Err(e) => return format!("⚠️ 查询答题装备失败: {}", e),
    };

    let rows: Vec<ExamGearFullRow> = stmt
        .query_map(params![user_id], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
                row.get::<_, String>(3).unwrap_or_default(),
                row.get::<_, String>(4).unwrap_or_default(),
                row.get::<_, String>(5).unwrap_or_default(),
                row.get::<_, String>(6).unwrap_or_default(),
                row.get::<_, String>(7).unwrap_or_default(),
                row.get::<_, String>(8).unwrap_or_default(),
                row.get::<_, String>(9).unwrap_or_default(),
                row.get::<_, String>(10).unwrap_or_default(),
                row.get::<_, String>(11).unwrap_or_default(),
                row.get::<_, String>(12).unwrap_or_default(),
                row.get::<_, String>(13).unwrap_or_default(),
                row.get::<_, String>(14).unwrap_or_default(),
                row.get::<_, String>(15).unwrap_or_default(),
                row.get::<_, String>(16).unwrap_or_default(),
                row.get::<_, String>(17).unwrap_or_default(),
                row.get::<_, String>(18).unwrap_or_default(),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    drop(stmt);

    if rows.is_empty() {
        return "📋 您没有可领取的答题装备。".to_string();
    }

    let mut claimed = Vec::new();
    let mut skipped = Vec::new();

    let items_to_claim: Vec<(usize, &ExamGearFullRow)> = if let Some(idx) = target_index {
        if idx == 0 || idx > rows.len() {
            return format!("⚠️ 序号无效，可选范围: 1-{}", rows.len());
        }
        vec![(idx, &rows[idx - 1])]
    } else {
        rows.iter().enumerate().map(|(i, r)| (i + 1, r)).collect()
    };

    for (idx, row) in &items_to_claim {
        let slot = &row.0;
        let name = &row.1;
        let name_decoded = crate::encoding::smart_decode(name);

        // Check if the slot is a standard slot
        let standard_slots = [
            "武器", "头盔", "铠甲", "护腿", "靴子", "项链", "戒指", "翅膀", "时装", "称号",
        ];
        let slot_decoded = crate::encoding::smart_decode(slot);

        if !standard_slots.contains(&slot_decoded.as_str()) {
            skipped.push(format!("{}. {} (非标准槽位: {})", idx, name_decoded, slot_decoded));
            continue;
        }

        // Delete old equipment in this slot
        let _ = conn.execute(
            "DELETE FROM Equip_Register WHERE User=?1 AND SlotName=?2",
            params![user_id, slot_decoded],
        );

        // Insert exam gear into Equip_Register
        let result = conn.execute(
            "INSERT INTO Equip_Register (User, SlotName, EquipName, Add_HP, Add_MP, Add_Defense, \
             Add_Magic, Add_AD, Add_AP, Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, \
             Add_ADPTV, Add_ADPTR, Add_APPTR, Add_APPTV, Add_ImmuneDamage, Special_Type, Special_Value) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                user_id,
                slot_decoded,
                name,
                row.2,
                row.3,
                row.4,
                row.5,
                row.6,
                row.7,
                row.8,
                row.9,
                row.10,
                row.11,
                row.12,
                row.13,
                row.14,
                row.15,
                row.16,
                row.17,
                row.18,
            ],
        );

        if result.is_ok() {
            // Remove from exam gear register
            let _ = conn.execute(
                "DELETE FROM ext_examgear_register WHERE User=?1 AND SlotName=?2 AND EquipName=?3",
                params![user_id, slot, name],
            );
            claimed.push(format!("{}. 【{}】{}", idx, slot_decoded, name_decoded));
        } else {
            skipped.push(format!("{}. {} (装备失败)", idx, name_decoded));
        }
    }
    drop(conn);

    let mut out = String::from("📋 【领取答题装备结果】\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    if !claimed.is_empty() {
        out.push_str("✅ 成功领取:\n");
        for c in &claimed {
            out.push_str(&format!("   {}\n", c));
        }
    }

    if !skipped.is_empty() {
        out.push_str("⚠️ 跳过:\n");
        for s in &skipped {
            out.push_str(&format!("   {}\n", s));
        }
    }

    if claimed.is_empty() && !skipped.is_empty() {
        out.push_str("❌ 没有成功领取任何答题装备\n");
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out
}

#[allow(dead_code)]
/// 标准装备槽位列表
pub const STANDARD_SLOTS: &[&str] = &[
    "武器", "头盔", "铠甲", "护腿", "靴子", "项链", "戒指", "翅膀", "时装", "称号",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_slots_count() {
        assert_eq!(STANDARD_SLOTS.len(), 10);
    }

    #[test]
    fn test_standard_slots_all_present() {
        assert!(STANDARD_SLOTS.contains(&"武器"));
        assert!(STANDARD_SLOTS.contains(&"头盔"));
        assert!(STANDARD_SLOTS.contains(&"铠甲"));
        assert!(STANDARD_SLOTS.contains(&"护腿"));
        assert!(STANDARD_SLOTS.contains(&"靴子"));
        assert!(STANDARD_SLOTS.contains(&"项链"));
        assert!(STANDARD_SLOTS.contains(&"戒指"));
        assert!(STANDARD_SLOTS.contains(&"翅膀"));
        assert!(STANDARD_SLOTS.contains(&"时装"));
        assert!(STANDARD_SLOTS.contains(&"称号"));
    }

    #[test]
    fn test_standard_slots_no_duplicates() {
        let mut sorted = STANDARD_SLOTS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), STANDARD_SLOTS.len());
    }

    #[test]
    fn test_exam_gear_row_field_count() {
        // ExamGearRow should have 10 String fields
        let row: ExamGearRow = (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert_eq!(row.0.len(), 0); // just verify it compiles with 10 fields
    }

    #[test]
    fn test_exam_gear_full_row_field_count() {
        // ExamGearFullRow should have 19 String fields
        let row: ExamGearFullRow = (
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        );
        assert_eq!(row.0.len(), 0); // verify 19 fields compile
    }
}

//! 增益/减益系统 (Buff/Debuff System)
//!
//! 管理玩家的临时属性加成和减益效果
//! 来源: DynamicAttributes_Register 表 (128条记录)
//! 支持的属性: AD(物攻), MagicResistance(魔抗), Defense(防御), AbsorbHP(吸血), Hit(命中)
//!
//! 指令: 查看增益, 增益信息, 清除增益

use crate::db::Database;
use crate::user;
use chrono::Local;

/// 增益效果条目
#[derive(Debug, Clone)]
pub struct BuffEntry {
    pub attr_name: String,
    pub value: i32,
    pub expire_time: String,
    pub is_active: bool,
}

/// 属性名称的中文映射
fn attr_display_name(attr: &str) -> &'static str {
    match attr {
        "AD" => "物攻",
        "AP" => "魔攻",
        "Defense" => "防御",
        "MagicResistance" => "魔抗",
        "Hit" => "命中",
        "Dodge" => "闪避",
        "Crit" => "暴击",
        "AbsorbHP" => "吸血",
        "ImmuneDamage" => "免伤",
        "ADPTV" => "物穿",
        "APPTV" => "魔穿",
        _ => "未知",
    }
}

/// 获取用户活跃的增益效果
pub fn get_active_buffs(db: &Database, user_id: &str) -> Vec<BuffEntry> {
    let conn = db.lock_conn();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let mut stmt = match conn
        .prepare("SELECT Id, AttName, AttValue, AttInvalidTime FROM DynamicAttributes_Register WHERE User = ?1")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows: Vec<BuffEntry> = stmt
        .query_map([user_id], |row| {
            let _id: i32 = row.get(0).unwrap_or(0);
            let attr_raw: String = row.get(1).unwrap_or_default();
            // 去除可能的空字符
            let attr_name = attr_raw.trim_matches('\0').to_string();
            let value_str: String = row.get(2).unwrap_or_default();
            let expire: String = row.get(3).unwrap_or_default();
            let value: i32 = value_str.parse().unwrap_or(0);
            let is_active = expire > now;
            Ok(BuffEntry {
                attr_name,
                value,
                expire_time: expire,
                is_active,
            })
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    rows
}

/// 计算用户活跃增益的属性加成总和
pub fn calc_buff_bonuses(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    // 返回: (hp_bonus, mp_bonus, ad_bonus, ap_bonus, def_bonus, mres_bonus, hit_bonus, absorb_bonus)
    let buffs = get_active_buffs(db, user_id);
    let mut ad_bonus = 0;
    let mut ap_bonus = 0;
    let mut def_bonus = 0;
    let mut mres_bonus = 0;
    let mut hit_bonus = 0;
    let mut absorb_bonus = 0;

    for buff in &buffs {
        if !buff.is_active {
            continue;
        }
        match buff.attr_name.trim_matches('\0') {
            "AD" => ad_bonus += buff.value,
            "AP" => ap_bonus += buff.value,
            "Defense" => def_bonus += buff.value,
            "MagicResistance" => mres_bonus += buff.value,
            "Hit" => hit_bonus += buff.value,
            "AbsorbHP" => absorb_bonus += buff.value,
            _ => {}
        }
    }

    (0, 0, ad_bonus, ap_bonus, def_bonus, mres_bonus, hit_bonus, absorb_bonus)
}

/// 清理过期的增益效果
pub fn cleanup_expired_buffs(db: &Database) -> i32 {
    let conn = db.lock_conn();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    match conn.execute(
        "DELETE FROM DynamicAttributes_Register WHERE AttInvalidTime <= ?1",
        [&now],
    ) {
        Ok(count) => count as i32,
        Err(_) => 0,
    }
}

/// 添加增益效果
#[allow(dead_code)]
pub fn add_buff(db: &Database, user_id: &str, attr_name: &str, value: i32, duration_secs: i64) {
    let expire = (Local::now() + chrono::Duration::seconds(duration_secs))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT INTO DynamicAttributes_Register (User, AttName, AttValue, AttInvalidTime) VALUES (?1, ?2, ?3, ?4)",
        [user_id, attr_name, &value.to_string(), &expire],
    );
}

/// 查看增益 — 显示当前所有增益/减益效果
pub fn cmd_view_buffs(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看增益效果！", prefix);
    }

    // 先清理过期
    cleanup_expired_buffs(db);

    let buffs = get_active_buffs(db, user_id);
    let active_buffs: Vec<&BuffEntry> = buffs.iter().filter(|b| b.is_active).collect();

    if active_buffs.is_empty() {
        return format!(
            "{}\n📋 当前没有任何增益/减益效果\n\n💡 增益效果可通过以下方式获得：\n  · 使用特殊物品（如强化药水）\n  · 完成特定任务获得临时加成\n  · 战斗中触发被动技能效果\n  · GM发放增益道具",
            prefix
        );
    }

    let mut result = format!(
        "{}\n╔═══════════════════════╗\n║  📋 增益/减益效果  ║\n╚═══════════════════════╝\n\n",
        prefix
    );

    let mut total_buff = 0i32;
    let mut total_debuff = 0i32;

    for (i, buff) in active_buffs.iter().enumerate() {
        let display_attr = attr_display_name(&buff.attr_name);
        let sign = if buff.value >= 0 { "+" } else { "" };
        let icon = if buff.value >= 0 { "🟢" } else { "🔴" };

        result.push_str(&format!(
            "  {} {}. {} {}{} (到期: {})\n",
            icon,
            i + 1,
            display_attr,
            sign,
            buff.value,
            buff.expire_time
        ));

        if buff.value >= 0 {
            total_buff += buff.value;
        } else {
            total_debuff += buff.value;
        }
    }

    result.push_str(&format!(
        "\n━━━━━━━━━━━━━━━━━━━━\n\
         📊 共 {} 个活跃效果\n\
         🟢 增益总计: +{}\n\
         🔴 减益总计: {}\n\
         \n\
         💡 使用「增益信息」查看属性影响详情",
        active_buffs.len(),
        total_buff,
        total_debuff
    ));

    result
}

/// 增益信息 — 显示增益对各项属性的具体影响
pub fn cmd_buff_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    cleanup_expired_buffs(db);

    let (hp, mp, ad, ap, def, mres, hit, absorb) = calc_buff_bonuses(db, user_id);

    let mut result = format!(
        "{}\n╔═══════════════════════╗\n║  📊 增益属性影响  ║\n╚═══════════════════════╝\n\n",
        prefix
    );

    let attrs = [
        ("⚔ 物攻", ad),
        ("🔮 魔攻", ap),
        ("🛡 防御", def),
        ("🔰 魔抗", mres),
        ("🎯 命中", hit),
        ("💉 吸血", absorb),
        ("❤️ 生命", hp),
        ("💙 魔法", mp),
    ];

    let mut has_effect = false;
    for (name, value) in &attrs {
        if *value != 0 {
            has_effect = true;
            let sign = if *value >= 0 { "+" } else { "" };
            let icon = if *value >= 0 { "↑" } else { "↓" };
            result.push_str(&format!("  {} {} {}{}\n", icon, name, sign, value));
        }
    }

    if !has_effect {
        result.push_str("  暂无活跃的属性加成\n");
    }

    let total = ad + ap + def + mres + hit + absorb + hp + mp;
    result.push_str(&format!(
        "\n━━━━━━━━━━━━━━━━━━━━\n\
         📈 总属性变化: {}\n\
         \n\
         💡 使用「查看增益」查看所有效果列表",
        if total >= 0 {
            format!("+{}", total)
        } else {
            total.to_string()
        }
    ));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr_display_name_ad() {
        assert_eq!(attr_display_name("AD"), "物攻");
    }

    #[test]
    fn test_attr_display_name_all() {
        assert_eq!(attr_display_name("AD"), "物攻");
        assert_eq!(attr_display_name("AP"), "魔攻");
        assert_eq!(attr_display_name("Defense"), "防御");
        assert_eq!(attr_display_name("MagicResistance"), "魔抗");
        assert_eq!(attr_display_name("Hit"), "命中");
        assert_eq!(attr_display_name("Dodge"), "闪避");
        assert_eq!(attr_display_name("Crit"), "暴击");
        assert_eq!(attr_display_name("AbsorbHP"), "吸血");
        assert_eq!(attr_display_name("ImmuneDamage"), "免伤");
        assert_eq!(attr_display_name("ADPTV"), "物穿");
        assert_eq!(attr_display_name("APPTV"), "魔穿");
    }

    #[test]
    fn test_attr_display_name_unknown() {
        assert_eq!(attr_display_name("UnknownAttr"), "未知");
        assert_eq!(attr_display_name(""), "未知");
    }

    #[test]
    fn test_buff_entry_struct() {
        let buff = BuffEntry {
            attr_name: "AD".to_string(),
            value: 10,
            expire_time: "2026-12-31 23:59:59".to_string(),
            is_active: true,
        };
        assert_eq!(buff.attr_name, "AD");
        assert_eq!(buff.value, 10);
        assert!(buff.is_active);
    }
}

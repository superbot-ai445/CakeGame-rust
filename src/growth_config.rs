/// 怪物/职业成长配置系统
///
/// 基于 Ext_Var_MonSet_hzxyx 表（30条）：怪物属性成长配置
///   Name(怪物名) | Av(属性加成: "bonus1,bonus2")
///
/// 基于 Ext_Var_OccSet_hzxyx 表（6条）：职业成长配置
///   Name(职业名) | Grow(成长率: "grow_hp,grow_ad,grow_ap")
///
/// 基于 Ext_Attribut_Set_hzxyx 表（7条）：属性类别定义
///   Name(属性缩写) — tk=体质, pw=力量, ap=法强, ad=物攻, ae=法攻, doom=厄运, be=抗性
///
/// 功能：
/// - 查看怪物成长: 列出所有怪物的属性成长配置
/// - 查看职业成长: 列出所有职业的成长率配置
/// - 修改怪物成长: GM 修改指定怪物的属性加成
/// - 修改职业成长: GM 修改指定职业的成长率
/// - 属性类别列表: 查看所有属性类别定义
use crate::db::Database;
use crate::permissions;
use crate::user;

/// 怪物成长配置条目
pub struct MonsterGrowthEntry {
    pub name: String,
    pub av: String, // "bonus1,bonus2"
}

/// 职业成长配置条目
pub struct OccupationGrowthEntry {
    pub name: String,
    pub grow: String, // "grow_hp,grow_ad,grow_ap"
}

/// 从数据库读取所有怪物成长配置
pub fn load_monster_growths(db: &Database) -> Vec<MonsterGrowthEntry> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Name, Av FROM Ext_Var_MonSet_hzxyx ORDER BY rowid") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
        ))
    });

    match rows {
        Ok(mapped) => mapped
            .filter_map(|r| r.ok())
            .map(|(name, av)| MonsterGrowthEntry { name, av })
            .collect(),
        Err(_) => vec![],
    }
}

/// 从数据库读取所有职业成长配置
pub fn load_occupation_growths(db: &Database) -> Vec<OccupationGrowthEntry> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Name, Grow FROM Ext_Var_OccSet_hzxyx ORDER BY rowid") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
        ))
    });

    match rows {
        Ok(mapped) => mapped
            .filter_map(|r| r.ok())
            .map(|(name, grow)| OccupationGrowthEntry { name, grow })
            .collect(),
        Err(_) => vec![],
    }
}

/// 读取属性类别列表
pub fn load_attribute_sets(db: &Database) -> Vec<String> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Name FROM Ext_Attribut_Set_hzxyx ORDER BY rowid") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows = stmt.query_map([], |row| Ok(row.get::<_, String>(0).unwrap_or_default()));

    match rows {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    }
}

/// 解析 "a,b" 格式的属性值
fn parse_growth_values(grow: &str) -> Vec<&str> {
    grow.split(',').map(|s| s.trim()).collect()
}

/// 获取属性缩写的中文名
fn attr_name_cn(abbr: &str) -> &str {
    match abbr {
        "tk" => "体质",
        "pw" => "力量",
        "ap" => "法强",
        "ad" => "物攻",
        "ae" => "法攻",
        "doom" => "厄运",
        "be" => "抗性",
        _ => abbr,
    }
}

/// 获取属性对应的emoji
fn emoji_for_attr(index: usize) -> &'static str {
    match index {
        0 => "💚",
        1 => "⚔️",
        2 => "🔮",
        3 => "🗡️",
        4 => "✨",
        5 => "💀",
        6 => "🛡️",
        _ => "📊",
    }
}

// ==================== 指令处理 ====================

/// 查看怪物成长 — 列出所有怪物的属性成长配置
pub fn cmd_view_monster_growth(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let entries = load_monster_growths(db);

    if entries.is_empty() {
        return format!("{}\n📋 暂无怪物成长配置数据", prefix);
    }

    // 如果指定了怪物名，显示详情
    if !args.is_empty() {
        if let Some(entry) = entries.iter().find(|e| e.name.contains(args) || args.contains(&e.name)) {
            let values = parse_growth_values(&entry.av);
            let mut out = format!("{}\n📊 【怪物成长详情】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
            out.push_str(&format!("  🐉 怪物: {}\n", entry.name));

            let attr_names = load_attribute_sets(db);
            for (i, val) in values.iter().enumerate() {
                let attr_label = if i < attr_names.len() {
                    format!("{}({})", attr_names[i], attr_name_cn(&attr_names[i]))
                } else {
                    format!("属性{}", i + 1)
                };
                let val_f: f64 = val.parse().unwrap_or(0.0);
                let bar = if val_f > 0.0 {
                    let bars = (val_f * 10.0).min(20.0) as usize;
                    "█".repeat(bars.max(1))
                } else {
                    "░".to_string()
                };
                out.push_str(&format!("  {} {}: {} {}\n", emoji_for_attr(i), attr_label, val, bar));
            }

            out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
            out.push_str("💡 成长值影响怪物战斗时的额外属性加成\n");
            out.push_str("🔧 GM: 修改怪物成长+怪物名+属性序号+数值\n");
            return out;
        }
        return format!("{}\n❌ 未找到包含「{}」的怪物成长配置", prefix, args);
    }

    // 列表模式
    let mut out = format!(
        "{}\n📊 【怪物成长配置】共 {} 种怪物\n━━━━━━━━━━━━━━━━━━━━\n",
        prefix,
        entries.len()
    );

    let attr_names = load_attribute_sets(db);
    // 表头
    out.push_str("  怪物名          |");
    for (i, attr) in attr_names.iter().enumerate() {
        out.push_str(&format!(" {}({})", attr, attr_name_cn(attr)));
        if i < attr_names.len() - 1 {
            out.push('|');
        }
    }
    out.push_str("\n  ─────────────────────────────────────\n");

    for entry in &entries {
        let values = parse_growth_values(&entry.av);
        let mut line = format!("  {:<14}", entry.name);
        for (i, val) in values.iter().enumerate() {
            let attr_label = if i < attr_names.len() {
                attr_names[i].to_string()
            } else {
                format!("A{}", i + 1)
            };
            line.push_str(&format!(" | {}:{}", attr_label, val));
        }
        out.push_str(&format!("{}\n", line));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 查看详情: 查看怪物成长+怪物名\n");
    out.push_str("🔧 GM: 修改怪物成长+怪物名+属性序号+数值\n");

    out
}

/// 查看职业成长 — 列出所有职业的成长率配置
pub fn cmd_view_occupation_growth(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let entries = load_occupation_growths(db);

    if entries.is_empty() {
        return format!("{}\n📋 暂无职业成长配置数据", prefix);
    }

    // 如果指定了职业名，显示详情
    if !args.is_empty() {
        if let Some(entry) = entries.iter().find(|e| e.name.contains(args) || args.contains(&e.name)) {
            let values = parse_growth_values(&entry.grow);
            let labels = ["生命成长", "物攻成长", "法攻成长"];
            let emojis = ["❤️", "⚔️", "🔮"];

            let mut out = format!("{}\n📈 【职业成长详情】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
            out.push_str(&format!("  🎭 职业: {}\n", entry.name));

            for (i, val) in values.iter().enumerate() {
                let label = if i < labels.len() { labels[i] } else { "成长" };
                let emoji = if i < emojis.len() { emojis[i] } else { "📊" };
                let val_f: f64 = val.parse().unwrap_or(0.0);
                let bar = if val_f > 0.0 {
                    let bars = (val_f * 10.0).min(20.0) as usize;
                    "█".repeat(bars.max(1))
                } else {
                    "░".to_string()
                };
                out.push_str(&format!("  {} {}: {} {}\n", emoji, label, val, bar));
            }

            out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
            out.push_str("💡 成长率影响每次升级时自动增加的属性\n");
            out.push_str("🔧 GM: 修改职业成长+职业名+序号+数值 (1=生命 2=物攻 3=法攻)\n");
            return out;
        }
        return format!("{}\n❌ 未找到包含「{}」的职业成长配置", prefix, args);
    }

    // 列表模式
    let mut out = format!(
        "{}\n📈 【职业成长配置】共 {} 个职业\n━━━━━━━━━━━━━━━━━━━━\n",
        prefix,
        entries.len()
    );
    out.push_str("  职业      | 生命成长 | 物攻成长 | 法攻成长\n");
    out.push_str("  ─────────────────────────────────────\n");

    for entry in &entries {
        let values = parse_growth_values(&entry.grow);
        let mut line = format!("  {:<8}", entry.name);
        for val in &values {
            line.push_str(&format!(" | {:>6}", val));
        }
        out.push_str(&format!("{}\n", line));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 查看详情: 查看职业成长+职业名\n");
    out.push_str("🔧 GM: 修改职业成长+职业名+序号+数值 (1=生命 2=物攻 3=法攻)\n");

    out
}

/// 修改怪物成长 — GM 命令: 修改怪物成长+怪物名+属性序号+数值
pub fn cmd_set_monster_growth(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // GM 权限检查 (需要管理员 98+)
    let perm = permissions::get_permission(db, user_id);
    if perm < 98 {
        return format!("{}\n❌ 无权执行此操作，需要管理员权限", prefix);
    }

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return format!(
            "{}\n❌ 格式: 修改怪物成长+怪物名+属性序号+数值\n💡 例: 修改怪物成长+史莱姆+1+5.0",
            prefix
        );
    }

    let monster_name = parts[0];
    let attr_index: usize = match parts[1].parse::<usize>() {
        Ok(n) if n >= 1 => n - 1, // 1-indexed to 0-indexed
        _ => return format!("{}\n❌ 属性序号必须是正整数", prefix),
    };
    let new_value = parts[2];

    // 验证数值格式
    if new_value.parse::<f64>().is_err() {
        return format!("{}\n❌ 数值必须是数字 (如 0, 3.5, 10.0)", prefix);
    }

    // 查找怪物
    let entries = load_monster_growths(db);
    let entry = match entries
        .iter()
        .find(|e| e.name == monster_name || e.name.contains(monster_name))
    {
        Some(e) => e,
        None => return format!("{}\n❌ 未找到怪物「{}」的成长配置", prefix, monster_name),
    };

    let actual_name = entry.name.clone();
    let mut values: Vec<String> = entry.av.split(',').map(|s| s.trim().to_string()).collect();

    if attr_index >= values.len() {
        return format!(
            "{}\n❌ 属性序号超出范围，{} 共有 {} 个属性 (1-{})",
            prefix,
            actual_name,
            values.len(),
            values.len()
        );
    }

    let old_value = values[attr_index].clone();
    values[attr_index] = new_value.to_string();
    let new_av = values.join(",");

    // 更新数据库
    let conn = db.lock_conn();
    match conn.execute(
        "UPDATE Ext_Var_MonSet_hzxyx SET Av=?1 WHERE Name=?2",
        rusqlite::params![new_av, actual_name],
    ) {
        Ok(_) => {
            let attr_names = load_attribute_sets(db);
            let attr_label = if attr_index < attr_names.len() {
                format!("{}({})", attr_names[attr_index], attr_name_cn(&attr_names[attr_index]))
            } else {
                format!("属性{}", attr_index + 1)
            };
            format!(
                "{}\n✅ 怪物成长修改成功！\n━━━━━━━━━━━━━━━━━━━━\n  🐉 怪物: {}\n  📊 {}: {} → {}\n  📋 完整配置: {}\n━━━━━━━━━━━━━━━━━━━━",
                prefix, actual_name, attr_label, old_value, new_value, new_av
            )
        }
        Err(e) => format!("{}\n❌ 修改失败: {}", prefix, e),
    }
}

/// 修改职业成长 — GM 命令: 修改职业成长+职业名+序号+数值
pub fn cmd_set_occupation_growth(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // GM 权限检查 (需要管理员 98+)
    let perm = permissions::get_permission(db, user_id);
    if perm < 98 {
        return format!("{}\n❌ 无权执行此操作，需要管理员权限", prefix);
    }

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return format!("{}\n❌ 格式: 修改职业成长+职业名+序号+数值\n💡 例: 修改职业成长+勇者+1+5.0\n  序号: 1=生命成长 2=物攻成长 3=法攻成长", prefix);
    }

    let occ_name = parts[0];
    let grow_index: usize = match parts[1].parse::<usize>() {
        Ok(n) if n >= 1 => n - 1,
        _ => return format!("{}\n❌ 序号必须是正整数 (1=生命 2=物攻 3=法攻)", prefix),
    };
    let new_value = parts[2];

    if new_value.parse::<f64>().is_err() {
        return format!("{}\n❌ 数值必须是数字 (如 0, 2.5, 10.0)", prefix);
    }

    let entries = load_occupation_growths(db);
    let entry = match entries.iter().find(|e| e.name == occ_name || e.name.contains(occ_name)) {
        Some(e) => e,
        None => return format!("{}\n❌ 未找到职业「{}」的成长配置", prefix, occ_name),
    };

    let actual_name = entry.name.clone();
    let mut values: Vec<String> = entry.grow.split(',').map(|s| s.trim().to_string()).collect();

    if grow_index >= values.len() {
        return format!(
            "{}\n❌ 序号超出范围，{} 共有 {} 个成长属性 (1-{})",
            prefix,
            actual_name,
            values.len(),
            values.len()
        );
    }

    let labels = ["生命成长", "物攻成长", "法攻成长"];
    let old_value = values[grow_index].clone();
    values[grow_index] = new_value.to_string();
    let new_grow = values.join(",");

    let conn = db.lock_conn();
    match conn.execute(
        "UPDATE Ext_Var_OccSet_hzxyx SET Grow=?1 WHERE Name=?2",
        rusqlite::params![new_grow, actual_name],
    ) {
        Ok(_) => {
            let label = if grow_index < labels.len() {
                labels[grow_index]
            } else {
                "成长"
            };
            format!(
                "{}\n✅ 职业成长修改成功！\n━━━━━━━━━━━━━━━━━━━━\n  🎭 职业: {}\n  📊 {}: {} → {}\n  📋 完整配置: {}\n━━━━━━━━━━━━━━━━━━━━",
                prefix, actual_name, label, old_value, new_value, new_grow
            )
        }
        Err(e) => format!("{}\n❌ 修改失败: {}", prefix, e),
    }
}

/// 属性类别列表 — 查看所有属性类别定义
pub fn cmd_view_attribute_sets(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let attrs = load_attribute_sets(db);

    if attrs.is_empty() {
        return format!("{}\n📋 暂无属性类别数据", prefix);
    }

    let mut out = format!(
        "{}\n🏷️ 【属性类别列表】共 {} 种\n━━━━━━━━━━━━━━━━━━━━\n",
        prefix,
        attrs.len()
    );

    for (i, attr) in attrs.iter().enumerate() {
        let cn = attr_name_cn(attr);
        let desc = match attr.as_str() {
            "tk" => "影响生命上限和物理防御",
            "pw" => "影响物理攻击力",
            "ap" => "影响魔法攻击力和技能伤害",
            "ad" => "影响普通攻击和物攻技能",
            "ae" => "影响法术攻击和元素伤害",
            "doom" => "影响暴击率和特殊效果触发",
            "be" => "影响伤害减免和异常状态抗性",
            _ => "自定义属性",
        };
        out.push_str(&format!("  {}. {}({}) — {}\n", i + 1, attr, cn, desc));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 这些属性缩写用于怪物/职业成长配置\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_growth_values() {
        let values = parse_growth_values("3.5,7.0");
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "3.5");
        assert_eq!(values[1], "7.0");
    }

    #[test]
    fn test_parse_growth_values_three() {
        let values = parse_growth_values("1.0,2.5,3.0");
        assert_eq!(values.len(), 3);
        assert_eq!(values[2], "3.0");
    }

    #[test]
    fn test_attr_name_cn() {
        assert_eq!(attr_name_cn("tk"), "体质");
        assert_eq!(attr_name_cn("pw"), "力量");
        assert_eq!(attr_name_cn("ap"), "法强");
        assert_eq!(attr_name_cn("unknown"), "unknown");
    }
}

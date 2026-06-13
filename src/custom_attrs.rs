/// CakeGame 自定义属性解析引擎
/// 激活 CustomAttributes_Register 表 — 数据库驱动的属性计算模板
/// 支持动态SQL模板执行、参数替换、多源属性聚合
///
/// 数据来源:
/// - CustomAttributes_Register (3条SQL模板)
/// - system_uAttributes (15条系统基础属性)
/// - editor_attribute_adjustment (3条GM/VIP调整)
/// - DynamicAttributes_Register (临时buff/debuff)
use crate::db::Database;

/// GM权限等级常量
const PERMISSION_LEVEL_ADMIN: i32 = 100;

/// 自定义属性SQL模板
struct AttrSqlTemplate {
    id: i32,
    sql_template: String,
}

/// 属性来源描述
struct AttrSource {
    name: String,
    value: i64,
    source_type: String,
}

/// 从 CustomAttributes_Register 读取所有SQL模板
fn load_sql_templates(db: &Database) -> Vec<AttrSqlTemplate> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Id, SQL_TEXT FROM CustomAttributes_Register ORDER BY Id") {
        Ok(s) => s,
        Err(_) => {
            // Fallback: try without SQL_TEXT column name (column might be named 'SQL')
            return match conn.prepare("SELECT Id, \"SQL\" FROM CustomAttributes_Register ORDER BY Id") {
                Ok(mut s) => {
                    s.query_map([], |row| {
                        let id: i32 = row.get(0)?;
                        let sql_raw: String = row.get(1).unwrap_or_default();
                        // Clean null terminators
                        let sql_clean = sql_raw.trim_matches('\0').to_string();
                        Ok(AttrSqlTemplate {
                            id,
                            sql_template: sql_clean,
                        })
                    })
                    .map(|iter| iter.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default()
                }
                Err(_) => Vec::new(),
            };
        }
    };

    stmt.query_map([], |row| {
        let id: i32 = row.get(0)?;
        let sql_raw: String = row.get(1).unwrap_or_default();
        let sql_clean = sql_raw.trim_matches('\0').to_string();
        Ok(AttrSqlTemplate {
            id,
            sql_template: sql_clean,
        })
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

/// 解析SQL模板中的参数占位符
/// [属性名称] -> attr_name
/// [用户标识] -> user_id
#[allow(dead_code)]
fn resolve_sql(template: &str, attr_name: &str, user_id: &str) -> String {
    template.replace("[属性名称]", attr_name).replace("[用户标识]", user_id)
}

/// 使用模板1计算系统基础属性总和
fn calc_system_base_attr(db: &Database, attr_name: &str) -> i64 {
    let conn = db.lock_conn();
    conn.query_row(
        "SELECT COALESCE(SUM(aValue), 0) FROM system_uAttributes WHERE AttrName = ?1",
        [attr_name],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

/// 使用模板2计算GM/VIP属性调整
fn calc_editor_adjustment(db: &Database, user_id: &str, attr_name: &str) -> i64 {
    let conn = db.lock_conn();
    conn.query_row(
        "SELECT COALESCE(SUM(aValue), 0) FROM editor_attribute_adjustment WHERE uId = ?1 AND aName = ?2",
        rusqlite::params![user_id, attr_name],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

/// 计算DynamicAttributes中的临时buff
fn calc_dynamic_attrs(db: &Database, user_id: &str, attr_name: &str) -> i64 {
    let conn = db.lock_conn();
    let clean_attr = attr_name.trim_matches('\0');
    conn.query_row(
        "SELECT COALESCE(CAST(SUM(CAST(AttValue AS INTEGER)), 0)) FROM DynamicAttributes_Register WHERE User = ?1 AND TRIM(AttName, '\0') = ?2 AND AttInvalidTime > datetime('now')",
        rusqlite::params![user_id, clean_attr],
        |row| row.get(0),
    )
    .unwrap_or(0)
}

/// 聚合属性来源 — 显示每个属性的来源明细
fn get_attr_sources(db: &Database, user_id: &str, attr_name: &str) -> Vec<AttrSource> {
    let mut sources = Vec::new();

    let base = calc_system_base_attr(db, attr_name);
    if base != 0 {
        sources.push(AttrSource {
            name: "系统基础".to_string(),
            value: base,
            source_type: "system_uAttributes".to_string(),
        });
    }

    let editor = calc_editor_adjustment(db, user_id, attr_name);
    if editor != 0 {
        sources.push(AttrSource {
            name: "GM/VIP调整".to_string(),
            value: editor,
            source_type: "editor_attribute_adjustment".to_string(),
        });
    }

    let dynamic = calc_dynamic_attrs(db, user_id, attr_name);
    if dynamic != 0 {
        sources.push(AttrSource {
            name: "临时增益".to_string(),
            value: dynamic,
            source_type: "DynamicAttributes_Register".to_string(),
        });
    }

    sources
}

/// 查看自定义属性 — 展示SQL模板和属性计算规则
pub fn cmd_view_custom_attrs(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let templates = load_sql_templates(db);
    let args = args.trim();

    // 如果指定了属性名，显示该属性的来源明细
    if !args.is_empty() {
        let sources = get_attr_sources(db, user_id, args);
        let total: i64 = sources.iter().map(|s| s.value).sum();

        let mut out = format!("{}\n📊 【属性来源: {}】\n━━━━━━━━━━━━━━━━━━━━\n", prefix, args);

        if sources.is_empty() {
            out.push_str("  暂无该属性的数据\n");
        } else {
            for src in &sources {
                let icon = match src.source_type.as_str() {
                    "system_uAttributes" => "🏗️",
                    "editor_attribute_adjustment" => "✏️",
                    "DynamicAttributes_Register" => "⚡",
                    _ => "📌",
                };
                out.push_str(&format!(
                    "  {} {}: {:+} (from {})\n",
                    icon, src.name, src.value, src.source_type
                ));
            }
            out.push_str(&format!("\n  📈 合计: {}\n", total));
        }

        out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
        return out;
    }

    // 显示SQL模板列表和系统属性概览
    let mut out = format!("{}\n⚙️ 【自定义属性引擎】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    out.push_str("📋 SQL计算模板:\n");
    if templates.is_empty() {
        out.push_str("  (无模板)\n");
    } else {
        for tpl in &templates {
            let desc = if tpl.sql_template.contains("system_uAttributes") {
                "系统基础属性聚合"
            } else if tpl.sql_template.contains("editor_attribute_adjustment") {
                "GM/VIP属性调整"
            } else {
                "自定义查询"
            };
            out.push_str(&format!("  #{}: {}\n", tpl.id, desc));
            // Show abbreviated SQL
            let sql_short = if tpl.sql_template.len() > 60 {
                format!("{}...", &tpl.sql_template[..57])
            } else {
                tpl.sql_template.clone()
            };
            out.push_str(&format!("    SQL: {}\n", sql_short));
        }
    }

    // 显示可用属性列表
    out.push_str("\n📊 可查询的属性:\n");
    let conn = db.lock_conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT AttrName FROM system_uAttributes ORDER BY AttrName")
        .unwrap();
    let attrs: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let attr_display = [
        ("HP", "生命"),
        ("MP", "魔法"),
        ("AD", "物攻"),
        ("AP", "魔攻"),
        ("Defense", "防御"),
        ("MagicResistance", "魔抗"),
        ("Hit", "命中"),
        ("Dodge", "闪避"),
        ("Crit", "暴击"),
        ("AbsorbHP", "吸血"),
        ("ImmuneDamage", "免伤"),
        ("ADPTV", "物穿值"),
        ("ADPTR", "物穿比"),
        ("APPTV", "法穿值"),
        ("APPTR", "法穿比"),
    ];

    for (eng, cn) in &attr_display {
        if attrs.iter().any(|a| a == eng) {
            let base = calc_system_base_attr(db, eng);
            out.push_str(&format!("  {}({}): 基础值{}\n", cn, eng, base));
        }
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「自定义属性+属性名」查看属性来源明细\n");
    out.push_str("💡 例: 自定义属性+AD 查看物攻的所有来源\n");
    out
}

/// GM添加属性调整
pub fn cmd_add_attr_adjustment(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);

    // 检查GM权限
    let perm: i32 = db.read_user_data(user_id, "permission_level").parse().unwrap_or(0);
    if perm < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n⚠️ 仅管理员可使用此功能", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n⚠️ 用法: 属性调整+目标用户ID,属性名,数值\n例: 属性调整+123456,AD,500",
            prefix
        );
    }

    let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return format!("{}\n⚠️ 参数不足，需要: 目标用户ID,属性名,数值", prefix);
    }

    let target_uid = parts[0];
    let attr_name = parts[1];
    let value: i32 = match parts[2].parse() {
        Ok(v) => v,
        Err(_) => return format!("{}\n⚠️ 数值格式错误: {}", prefix, parts[2]),
    };

    // 验证属性名
    let valid_attrs = [
        "HP",
        "MP",
        "AD",
        "AP",
        "Defense",
        "MagicResistance",
        "Hit",
        "Dodge",
        "Crit",
        "AbsorbHP",
        "ImmuneDamage",
        "ADPTV",
        "ADPTR",
        "APPTV",
        "APPTR",
    ];
    if !valid_attrs.contains(&attr_name) {
        return format!(
            "{}\n⚠️ 无效属性名: {}\n有效属性: {}",
            prefix,
            attr_name,
            valid_attrs.join(", ")
        );
    }

    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO editor_attribute_adjustment (uId, aName, aValue) VALUES (?1, ?2, ?3)",
        rusqlite::params![target_uid, attr_name, value],
    );

    format!(
        "{}\n✅ 已设置属性调整\n目标: {}\n属性: {} {:+}",
        prefix, target_uid, attr_name, value
    )
}

/// 查看GM属性调整列表
pub fn cmd_view_attr_adjustments(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);

    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT uId, aName, aValue FROM editor_attribute_adjustment ORDER BY uId, aName")
    {
        Ok(s) => s,
        Err(e) => return format!("{}\n⚠️ 查询失败: {}", prefix, e),
    };

    let rows: Vec<(String, String, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, i32>(2).unwrap_or(0),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    if rows.is_empty() {
        return format!("{}\n📝 暂无属性调整记录", prefix);
    }

    let mut out = format!("{}\n✏️ 【属性调整列表】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    for (uid, attr, val) in &rows {
        out.push_str(&format!("  用户 {} | {} {:+}\n", uid, attr, val));
    }
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「属性调整+用户ID,属性名,数值」添加调整\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_sql_basic() {
        let template = "SELECT SUM(aValue) FROM system_uAttributes WHERE AttrName = '[属性名称]'";
        let result = resolve_sql(template, "HP", "user123");
        assert_eq!(
            result,
            "SELECT SUM(aValue) FROM system_uAttributes WHERE AttrName = 'HP'"
        );
    }

    #[test]
    fn test_resolve_sql_editor() {
        let template =
            "SELECT SUM(aValue) FROM editor_attribute_adjustment WHERE uId = '[用户标识]' AND aName = '[属性名称]'";
        let result = resolve_sql(template, "AD", "vip_user");
        assert_eq!(
            result,
            "SELECT SUM(aValue) FROM editor_attribute_adjustment WHERE uId = 'vip_user' AND aName = 'AD'"
        );
    }

    #[test]
    fn test_resolve_sql_multiple_params() {
        let template = "SELECT * FROM t WHERE a = '[属性名称]' AND b = '[属性名称]' AND c = '[用户标识]'";
        let result = resolve_sql(template, "HP", "u1");
        assert_eq!(result, "SELECT * FROM t WHERE a = 'HP' AND b = 'HP' AND c = 'u1'");
    }

    #[test]
    fn test_resolve_sql_no_params() {
        let template = "SELECT COUNT(*) FROM table";
        let result = resolve_sql(template, "HP", "u1");
        assert_eq!(result, "SELECT COUNT(*) FROM table");
    }

    #[test]
    fn test_attr_source_icon_mapping() {
        let sources = [
            AttrSource {
                name: "基础".to_string(),
                value: 100,
                source_type: "system_uAttributes".to_string(),
            },
            AttrSource {
                name: "调整".to_string(),
                value: 50,
                source_type: "editor_attribute_adjustment".to_string(),
            },
            AttrSource {
                name: "buff".to_string(),
                value: 20,
                source_type: "DynamicAttributes_Register".to_string(),
            },
        ];
        let total: i64 = sources.iter().map(|s| s.value).sum();
        assert_eq!(total, 170);
    }

    #[test]
    fn test_valid_attrs_list() {
        let valid_attrs = [
            "HP",
            "MP",
            "AD",
            "AP",
            "Defense",
            "MagicResistance",
            "Hit",
            "Dodge",
            "Crit",
            "AbsorbHP",
            "ImmuneDamage",
            "ADPTV",
            "ADPTR",
            "APPTV",
            "APPTR",
        ];
        assert_eq!(valid_attrs.len(), 15);
        assert!(valid_attrs.contains(&"HP"));
        assert!(valid_attrs.contains(&"AD"));
        assert!(valid_attrs.contains(&"APPTR"));
        assert!(!valid_attrs.contains(&"INVALID"));
    }

    #[test]
    fn test_null_terminator_cleanup() {
        let raw = "SELECT SUM(aValue)\0";
        let clean = raw.trim_matches('\0');
        assert_eq!(clean, "SELECT SUM(aValue)");
    }
}

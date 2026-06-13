/// 怪物模板系统 (Monster Model System)
/// 使用 Monster_Model 表 (ID, Node, Item, Data) 实现自定义怪物模板的创建与管理
/// EAV 模型: System→LatestID 跟踪最大ID, <ID>→Basic/Dynamic 节点存储属性
use crate::db::Database;
use crate::permissions;

// ==================== Monster Model EAV Operations ====================

/// 获取下一个可用的怪物模板 ID
fn next_model_id(db: &Database) -> i64 {
    let conn = db.lock_conn();
    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(CAST(Data AS INTEGER), 0) FROM Monster_Model WHERE ID='System' AND Item='LatestID'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
        .parse()
        .unwrap_or(7);
    let new_id = current + 1;
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Monster_Model (ID, Node, Item, Data) VALUES ('System', 'important', 'LatestID', ?1)",
        [&new_id.to_string()],
    );
    new_id
}

/// 列出所有怪物模板 ID（排除 System 行）
fn list_model_ids(db: &Database) -> Vec<String> {
    let conn = db.lock_conn();
    let mut stmt = conn
        .prepare("SELECT DISTINCT ID FROM Monster_Model WHERE ID != 'System' ORDER BY CAST(ID AS INTEGER)")
        .unwrap();
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    ids
}

/// 获取指定怪物模板的属性 (节点→属性→值)
fn get_model_attrs(db: &Database, model_id: &str) -> Vec<(String, String, String)> {
    let conn = db.lock_conn();
    let mut stmt = conn
        .prepare("SELECT Node, Item, Data FROM Monster_Model WHERE ID=?1")
        .unwrap();
    stmt.query_map([model_id], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
        ))
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

/// 获取模板属性值
fn get_model_attr(db: &Database, model_id: &str, node: &str, item: &str) -> Option<String> {
    let conn = db.lock_conn();
    conn.query_row(
        "SELECT Data FROM Monster_Model WHERE ID=?1 AND Node=?2 AND Item=?3",
        [model_id, node, item],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

/// 设置/更新模板属性
fn set_model_attr(db: &Database, model_id: &str, node: &str, item: &str, data: &str) {
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Monster_Model (ID, Node, Item, Data) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![model_id, node, item, data],
    );
}

/// 删除怪物模板的所有属性行
fn delete_model(db: &Database, model_id: &str) -> usize {
    let conn = db.lock_conn();
    conn.execute("DELETE FROM Monster_Model WHERE ID=?1", [model_id])
        .unwrap_or(0)
}

// ==================== Display Helpers ====================

fn monster_type_name(t: &str) -> &str {
    match t {
        "Ordinary" => "普通",
        "Elite" => "精英",
        "Boss" => "BOSS",
        "Rare" => "稀有",
        _ => t,
    }
}

fn format_model_stats(attrs: &[(String, String, String)]) -> String {
    let mut r = String::new();
    let mut basic = Vec::new();
    let mut dynamic = Vec::new();

    for (node, item, data) in attrs {
        if node == "important" {
            continue;
        }
        if node == "Basic" {
            basic.push((item.as_str(), data.as_str()));
        } else if node == "Dynamic" {
            dynamic.push((item.as_str(), data.as_str()));
        }
    }

    // Basic stats
    for (item, data) in &basic {
        let label = match *item {
            "Monster_Name" => "名称",
            "Monster_Type" => "类型",
            "Monster_AD" => "物攻",
            "Monster_AP" => "魔攻",
            "Monster_HP" => "生命",
            "Monster_Defense" => "防御",
            "MagicResistance" => "魔抗",
            "Hit" => "命中",
            "Dodge" => "闪避",
            "Monster_AbsorbHP" => "吸血",
            "Monster_ADPTV" => "物穿值",
            "Monster_ADPTR" => "物穿比",
            "Monster_APPTV" => "法穿值",
            "Monster_APPTR" => "法穿比",
            "Monster_ImmuneDamage" => "免伤",
            _ => item,
        };
        if *item == "Monster_Type" {
            r.push_str(&format!("  {}: {}\n", label, monster_type_name(data)));
        } else {
            r.push_str(&format!("  {}: {}\n", label, data));
        }
    }

    // Dynamic overrides
    if !dynamic.is_empty() {
        r.push_str("  ── 动态修正 ──\n");
        for (item, data) in &dynamic {
            r.push_str(&format!("  {}: {}\n", item, data));
        }
    }

    r
}

// ==================== Public Commands ====================

/// 查看怪物模板列表
pub fn cmd_view_models(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看怪物模板！\n发送【注册+昵称】进行注册".to_string();
    }

    let model_ids = list_model_ids(db);
    if model_ids.is_empty() {
        return "📭 暂无自定义怪物模板。\n\n发送'创建怪物模板+名称'创建新模板".to_string();
    }

    let mut r = String::from("👹 === 怪物模板列表 ===\n━━━━━━━━━━━━━━━━━━━━\n");
    for (i, mid) in model_ids.iter().enumerate() {
        let name = get_model_attr(db, mid, "Basic", "Monster_Name").unwrap_or_else(|| "未命名".to_string());
        let mtype = get_model_attr(db, mid, "Basic", "Monster_Type").unwrap_or_else(|| "Ordinary".to_string());
        let hp = get_model_attr(db, mid, "Basic", "Monster_HP").unwrap_or_else(|| "0".to_string());
        r.push_str(&format!(
            "{}. [{}] {} | HP:{} | 类型:{}\n",
            i + 1,
            mid,
            name,
            hp,
            monster_type_name(&mtype)
        ));
    }
    r.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    r.push_str("\n发送'怪物详情+ID'查看模板详情");
    r.push_str("\n发送'创建怪物模板+名称'创建新模板");
    r.push_str("\n发送'删除怪物模板+ID'删除模板(GM)");
    r
}

/// 查看怪物模板详情
pub fn cmd_view_model_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看怪物模板！".to_string();
    }

    let model_id = args.trim();
    if model_id.is_empty() {
        return "请指定模板ID。\n用法：怪物详情+模板ID".to_string();
    }

    let attrs = get_model_attrs(db, model_id);
    if attrs.is_empty() {
        return format!("找不到模板 ID: {}", model_id);
    }

    let name = get_model_attr(db, model_id, "Basic", "Monster_Name").unwrap_or_else(|| "未命名".to_string());

    let mut r = format!("👹 === 怪物模板 [{}] {} ===\n━━━━━━━━━━━━━━━━━━━━\n", model_id, name);
    r.push_str(&format_model_stats(&attrs));

    // Estimate combat power
    let ad: f64 = get_model_attr(db, model_id, "Basic", "Monster_AD")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let ap: f64 = get_model_attr(db, model_id, "Basic", "Monster_AP")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let hp: f64 = get_model_attr(db, model_id, "Basic", "Monster_HP")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let def: f64 = get_model_attr(db, model_id, "Basic", "Monster_Defense")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let mdf: f64 = get_model_attr(db, model_id, "Basic", "MagicResistance")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    let power = (ad * 35.0 + ap * 217.5 + hp * 2.7 + def * 20.0 + mdf * 18.0) / 10.0;
    r.push_str(&format!("\n⚡ 预估战力: {:.0}\n", power));
    r.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    r.push_str("\n发送'修改怪物模板+ID+属性+数值'修改属性(GM)");
    r
}

/// 创建怪物模板 (GM only)
pub fn cmd_create_model(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册！".to_string();
    }
    if !permissions::get_permission(db, user_id) < 98 {
        return "⚠️ 仅管理员可创建怪物模板！".to_string();
    }

    let name = args.trim();
    if name.is_empty() {
        return "请指定怪物名称。\n用法：创建怪物模板+怪物名称".to_string();
    }

    // Check duplicate names
    let model_ids = list_model_ids(db);
    for mid in &model_ids {
        if let Some(existing_name) = get_model_attr(db, mid, "Basic", "Monster_Name") {
            if existing_name == name {
                return format!("⚠️ 怪物模板 [{}] 已存在（ID: {}）！", name, mid);
            }
        }
    }

    let new_id = next_model_id(db);
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_Name", name);
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_Type", "Ordinary");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_AD", "10");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_AP", "0");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_HP", "100");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_Defense", "5");
    set_model_attr(db, &new_id.to_string(), "Basic", "MagicResistance", "5");
    set_model_attr(db, &new_id.to_string(), "Basic", "Hit", "100");
    set_model_attr(db, &new_id.to_string(), "Basic", "Dodge", "50");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_AbsorbHP", "0");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_ADPTV", "0");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_ADPTR", "0");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_APPTV", "0");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_APPTR", "0");
    set_model_attr(db, &new_id.to_string(), "Basic", "Monster_ImmuneDamage", "0");

    format!(
        "✅ 怪物模板创建成功！\n\n👹 名称: {}\n📌 ID: {}\n类型: 普通\n生命: 100\n物攻: 10\n防御: 5\n\n发送'修改怪物模板+ID+属性+数值'自定义属性",
        name, new_id
    )
}

/// 修改怪物模板属性 (GM only)
pub fn cmd_edit_model(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册！".to_string();
    }
    if !permissions::get_permission(db, user_id) < 98 {
        return "⚠️ 仅管理员可修改怪物模板！".to_string();
    }

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return "用法：修改怪物模板+模板ID+属性名+数值\n\n可修改属性：\n  物攻/魔攻/生命/防御/魔抗/命中/闪避/吸血\n  物穿值/物穿比/法穿值/法穿比/免伤/类型".to_string();
    }

    let model_id = parts[0];
    let attr_name = parts[1];
    let value = parts[2];

    // Verify model exists
    let attrs = get_model_attrs(db, model_id);
    if attrs.is_empty() {
        return format!("找不到模板 ID: {}", model_id);
    }

    let item = match attr_name {
        "物攻" | "AD" => "Monster_AD",
        "魔攻" | "AP" => "Monster_AP",
        "生命" | "HP" => "Monster_HP",
        "防御" | "Defense" => "Monster_Defense",
        "魔抗" | "MDF" => "MagicResistance",
        "命中" | "Hit" => "Hit",
        "闪避" | "Dodge" => "Dodge",
        "吸血" | "AbsorbHP" => "Monster_AbsorbHP",
        "物穿值" | "ADPTV" => "Monster_ADPTV",
        "物穿比" | "ADPTR" => "Monster_ADPTR",
        "法穿值" | "APPTV" => "Monster_APPTV",
        "法穿比" | "APPTR" => "Monster_APPTR",
        "免伤" | "ImmuneDamage" => "Monster_ImmuneDamage",
        "类型" | "Type" => "Monster_Type",
        "名称" | "Name" => "Monster_Name",
        _ => return format!("⚠️ 未知属性: {}\n可修改: 物攻/魔攻/生命/防御/魔抗/命中/闪避/吸血/物穿值/物穿比/法穿值/法穿比/免伤/类型/名称", attr_name),
    };

    // Validate type values
    if item == "Monster_Type" {
        let valid_type = match value {
            "普通" | "Ordinary" => "Ordinary",
            "精英" | "Elite" => "Elite",
            "BOSS" | "Boss" => "Boss",
            "稀有" | "Rare" => "Rare",
            _ => return "⚠️ 无效类型！可选: 普通/精英/BOSS/稀有".to_string(),
        };
        set_model_attr(db, model_id, "Basic", item, valid_type);
    } else {
        // Validate numeric
        if value.parse::<f64>().is_err() {
            return "⚠️ 属性值必须为数字！".to_string();
        }
        set_model_attr(db, model_id, "Basic", item, value);
    }

    let name = get_model_attr(db, model_id, "Basic", "Monster_Name").unwrap_or_else(|| "未命名".to_string());
    format!(
        "✅ 怪物模板 [{}] ({}) 修改成功！\n{} → {}",
        name, model_id, attr_name, value
    )
}

/// 删除怪物模板 (GM only)
pub fn cmd_delete_model(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册！".to_string();
    }
    if !permissions::get_permission(db, user_id) < 98 {
        return "⚠️ 仅管理员可删除怪物模板！".to_string();
    }

    let model_id = args.trim();
    if model_id.is_empty() {
        return "请指定模板ID。\n用法：删除怪物模板+模板ID".to_string();
    }

    let name = get_model_attr(db, model_id, "Basic", "Monster_Name").unwrap_or_else(|| "未知".to_string());

    let deleted = delete_model(db, model_id);
    if deleted == 0 {
        return format!("找不到模板 ID: {}", model_id);
    }

    format!(
        "✅ 怪物模板 [{}] ({}) 已删除！\n共清理 {} 条数据",
        name, model_id, deleted
    )
}

/// 怪物模板类型统计
pub fn cmd_model_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册！".to_string();
    }

    let model_ids = list_model_ids(db);
    if model_ids.is_empty() {
        return "📭 暂无自定义怪物模板。".to_string();
    }

    let mut ordinary = 0;
    let mut elite = 0;
    let mut boss = 0;
    let mut rare = 0;
    let mut total_hp: f64 = 0.0;
    let mut total_ad: f64 = 0.0;
    let mut count = 0;

    for mid in &model_ids {
        let mtype = get_model_attr(db, mid, "Basic", "Monster_Type").unwrap_or_else(|| "Ordinary".to_string());
        match mtype.as_str() {
            "Elite" => elite += 1,
            "Boss" => boss += 1,
            "Rare" => rare += 1,
            _ => ordinary += 1,
        }
        if let Some(hp) = get_model_attr(db, mid, "Basic", "Monster_HP").and_then(|v| v.parse::<f64>().ok()) {
            total_hp += hp;
        }
        if let Some(ad) = get_model_attr(db, mid, "Basic", "Monster_AD").and_then(|v| v.parse::<f64>().ok()) {
            total_ad += ad;
        }
        count += 1;
    }

    let avg_hp = if count > 0 { total_hp / count as f64 } else { 0.0 };
    let avg_ad = if count > 0 { total_ad / count as f64 } else { 0.0 };

    let mut r = String::from("📊 === 怪物模板统计 ===\n━━━━━━━━━━━━━━━━━━━━\n");
    r.push_str(&format!("📦 总模板数: {}\n\n", count));
    r.push_str(&format!("  🟢 普通: {}\n", ordinary));
    r.push_str(&format!("  🔵 精英: {}\n", elite));
    r.push_str(&format!("  🟣 稀有: {}\n", rare));
    r.push_str(&format!("  🔴 BOSS: {}\n\n", boss));
    r.push_str(&format!("  ❤️ 平均生命: {:.0}\n", avg_hp));
    r.push_str(&format!("  ⚔️ 平均物攻: {:.0}\n", avg_ad));
    r.push_str("━━━━━━━━━━━━━━━━━━━━");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monster_type_name() {
        assert_eq!(monster_type_name("Ordinary"), "普通");
        assert_eq!(monster_type_name("Elite"), "精英");
        assert_eq!(monster_type_name("Boss"), "BOSS");
        assert_eq!(monster_type_name("Rare"), "稀有");
        assert_eq!(monster_type_name("Unknown"), "Unknown");
    }

    #[test]
    fn test_format_model_stats_empty() {
        let attrs = vec![];
        let result = format_model_stats(&attrs);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_model_stats_with_data() {
        let attrs = vec![
            ("important".to_string(), "LatestID".to_string(), "7".to_string()),
            ("Basic".to_string(), "Monster_Name".to_string(), "测试怪".to_string()),
            ("Basic".to_string(), "Monster_Type".to_string(), "Elite".to_string()),
            ("Basic".to_string(), "Monster_HP".to_string(), "500".to_string()),
            ("Dynamic".to_string(), "Monster_HP".to_string(), "41".to_string()),
        ];
        let result = format_model_stats(&attrs);
        assert!(result.contains("名称: 测试怪"));
        assert!(result.contains("类型: 精英"));
        assert!(result.contains("生命: 500"));
        assert!(result.contains("动态修正"));
        assert!(result.contains("Monster_HP: 41"));
    }
}

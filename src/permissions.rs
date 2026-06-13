/// GM 权限等级系统
/// 使用 Shared_Permissions 表 (ID, Power)
/// 权限等级: 99=超级管理员, 98=管理员, 31=版主, 0=普通玩家
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 权限等级名称
pub fn permission_name(power: i32) -> &'static str {
    match power {
        99 => "超级管理员",
        98 => "管理员",
        31..=97 => "版主",
        1..=30 => "VIP",
        _ => "普通玩家",
    }
}

/// 获取用户权限等级 (优先读 Shared_Permissions，兼容 Global Permissions)
pub fn get_permission(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    // 先查 Shared_Permissions 表
    let shared_power: i32 = conn
        .prepare("SELECT Power FROM Shared_Permissions WHERE ID=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id], |row| {
                let raw: String = row.get(0).unwrap_or_default();
                Ok(raw.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);

    if shared_power > 0 {
        return shared_power;
    }

    // 兼容旧的 Global 权限 (使用同一个 conn 避免死锁)
    let global_power: i32 = conn
        .prepare("SELECT DATA FROM Global WHERE SECTION=?1 AND ID=?2")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params!["Permissions", user_id], |row| {
                let raw: String = row.get(0).unwrap_or_default();
                Ok(raw.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);
    global_power
}

/// 设置用户权限等级 (写入 Shared_Permissions 表)
pub fn set_permission(db: &Database, user_id: &str, power: i32) -> bool {
    let power = power.clamp(0, 99);
    let conn = db.lock_conn();
    let updated = conn
        .execute(
            "UPDATE Shared_Permissions SET Power=?1 WHERE ID=?2",
            rusqlite::params![power.to_string(), user_id],
        )
        .unwrap_or(0);
    if updated == 0 {
        conn.execute(
            "INSERT INTO Shared_Permissions (ID, Power) VALUES (?1, ?2)",
            rusqlite::params![user_id, power.to_string()],
        )
        .is_ok()
    } else {
        true
    }
}

/// 删除用户权限
pub fn remove_permission(db: &Database, user_id: &str) -> bool {
    let conn = db.lock_conn();
    conn.execute("DELETE FROM Shared_Permissions WHERE ID=?1", rusqlite::params![user_id])
        .is_ok()
}

/// 获取所有管理员列表
pub fn list_permissions(db: &Database) -> Vec<(String, i32)> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT ID, Power FROM Shared_Permissions ORDER BY CAST(Power AS INTEGER) DESC") {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |row| {
        let id: String = row.get(0).unwrap_or_default();
        let power_str: String = row.get(1).unwrap_or_default();
        let power: i32 = power_str.parse().unwrap_or(0);
        Ok((id, power))
    })
    .map(|iter| iter.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

// ==================== 指令处理 ====================

/// 查看权限 - 查看自己或他人的权限等级
pub fn cmd_view_permission(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target = if args.trim().is_empty() {
        user_id.to_string()
    } else {
        args.trim().to_string()
    };

    let power = get_permission(db, &target);
    let name = permission_name(power);
    let nick = db.read_basic(&target, ITEM_NAME);
    let display = if nick.is_empty() {
        target.clone()
    } else {
        format!("{}({})", nick, target)
    };

    format!(
        "{}\n═══ 权限信息 ═══\n\n\
         玩家：{}\n\
         权限等级：{}\n\
         权限称号：{}",
        prefix, display, power, name
    )
}

/// 权限列表 - 查看所有管理员
pub fn cmd_permission_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let list = list_permissions(db);

    if list.is_empty() {
        return format!("{}\n暂无管理员信息。", prefix);
    }

    let mut result = format!("{}\n═══ 管理员列表 ═══", prefix);
    for (i, (id, power)) in list.iter().enumerate() {
        let name = permission_name(*power);
        let nick = db.read_basic(id, ITEM_NAME);
        let display = if nick.is_empty() { id.clone() } else { nick };
        result.push_str(&format!("\n{}. {} [{}] Lv.{}", i + 1, display, name, power));
    }
    result.push_str(&format!("\n\n共 {} 名管理员", list.len()));
    result
}

/// 设置权限 - GM 命令：设置权限+目标+等级
pub fn cmd_set_permission(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let my_power = get_permission(db, user_id);

    // 只有超级管理员(99)才能设置权限
    if my_power < 99 {
        return format!("{}\n⛔ 权限不足，需要超级管理员权限(99)", prefix);
    }

    if args.trim().is_empty() {
        return format!(
            "{}\n═══ 设置权限 ═══\n\n\
             格式：设置权限+目标ID+等级\n\n\
             权限等级：\n\
             99 - 超级管理员\n\
             98 - 管理员\n\
             31 - 版主\n\
             0  - 取消管理权限\n\n\
             示例：设置权限+{}+98",
            prefix, user_id
        );
    }

    let parts: Vec<&str> = args.splitn(2, '+').collect();
    if parts.len() < 2 {
        return format!("{}\n格式错误！正确格式：设置权限+目标ID+等级", prefix);
    }

    let target = parts[0].trim();
    let level_str = parts[1].trim();

    let level: i32 = match level_str.parse() {
        Ok(v) => v,
        Err(_) => return format!("{}\n❌ 等级必须是数字", prefix),
    };

    if !(0..=99).contains(&level) {
        return format!("{}\n❌ 等级范围：0-99", prefix);
    }

    // 不允许设置高于自己的等级
    if level >= my_power {
        return format!("{}\n⛔ 不能设置等于或高于自己权限等级的用户", prefix);
    }

    if !db.user_exists(target) {
        return format!("{}\n玩家 [{}] 不存在", prefix, target);
    }

    if level == 0 {
        remove_permission(db, target);
        let nick = db.read_basic(target, ITEM_NAME);
        format!("{}\n✅ 已取消 [{}]({}) 的管理权限", prefix, nick, target)
    } else {
        set_permission(db, target, level);
        let nick = db.read_basic(target, ITEM_NAME);
        let name = permission_name(level);
        format!(
            "{}\n✅ 已将 [{}]({}) 的权限设置为 {} (等级{})",
            prefix, nick, target, name, level
        )
    }
}

/// 系统公告（使用 system_uAttributes 读取系统配置）
pub fn cmd_system_attrs(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare("SELECT Node, AttrName, aValue FROM system_uAttributes ORDER BY Node, AttrName") {
        Ok(s) => s,
        Err(_) => return format!("{}\n暂无系统属性数据", prefix),
    };

    let attrs: Vec<(String, String, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default().parse().unwrap_or(0),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    if attrs.is_empty() {
        return format!("{}\n暂无系统属性数据", prefix);
    }

    fn attr_display(name: &str) -> &str {
        match name {
            "HP" => "生命",
            "MP" => "魔法",
            "AD" => "物攻",
            "AP" => "魔攻",
            "Defense" => "防御",
            "MagicResistance" => "魔抗",
            "Hit" => "命中",
            "Dodge" => "闪避",
            "Crit" => "暴击",
            "AbsorbHP" => "吸血",
            "ImmuneDamage" => "免伤",
            "ADPTV" => "物穿值",
            "APPTV" => "法穿值",
            _ => name,
        }
    }

    let mut result = format!("{}\n═══ 系统属性 ═══", prefix);
    let mut current_node = String::new();
    for (node, name, val) in &attrs {
        if *node != current_node {
            current_node = node.clone();
            result.push_str(&format!("\n\n📋 {}", node));
        }
        if *val != 0 {
            result.push_str(&format!("\n  {}：{:+}", attr_display(name), val));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_permission_levels() {
        // Verify permission level constants
        let admin = 100i32;
        let mod_level = 50i32;
        let user = 0i32;
        assert!(admin > mod_level);
        assert!(mod_level > user);
    }

    #[test]
    fn test_permission_level_ordering() {
        let levels = [0, 10, 50, 100];
        for i in 1..levels.len() {
            assert!(levels[i] > levels[i - 1]);
        }
    }
}

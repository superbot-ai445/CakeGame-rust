/// CakeGame 数据完整性检查系统
/// GM 管理工具：检测数据库中孤立记录、无效引用、数据异常
///
/// 功能:
/// - 数据检查: 全面扫描数据库完整性
/// - 修复建议: 针对发现的问题给出修复建议
/// - 孤立记录: 检测背包/装备/技能中引用不存在的用户
/// - 经济异常: 检测金币/钻石余额异常
/// - 数据统计: 各表记录数和健康度
use crate::db::Database;
use crate::permissions;
use crate::user;

/// 权限检查阈值
const PERMISSION_LEVEL_ADMIN: i32 = 100;

/// 格式化千分位数字
fn format_num(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 全面数据检查 (GM)
pub fn cmd_data_check(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let conn = db.lock_conn();
    let mut issues: Vec<String> = Vec::new();
    let mut stats: Vec<String> = Vec::new();

    // === 1. 用户数据统计 ===
    let total_users: i64 = conn
        .prepare("SELECT COUNT(DISTINCT ID) FROM Basic_User")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    let users_with_basic: i64 = conn
        .prepare("SELECT COUNT(DISTINCT ID) FROM Basic_User WHERE Node='Basic'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    stats.push(format!("👥 用户总数: {}", format_num(total_users)));
    stats.push(format!("📋 有基础属性的用户: {}", format_num(users_with_basic)));

    // === 2. 孤立背包记录检测 ===
    let orphan_knapsack: i64 = conn
        .prepare("SELECT COUNT(*) FROM Basic_knapsack WHERE ID NOT IN (SELECT DISTINCT ID FROM Basic_User)")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    if orphan_knapsack > 0 {
        issues.push(format!(
            "⚠️ 孤立背包记录: {} 条（用户不存在但有背包数据）",
            format_num(orphan_knapsack)
        ));
    }

    let total_knapsack: i64 = conn
        .prepare("SELECT COUNT(*) FROM Basic_knapsack")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("🎒 背包记录总数: {}", format_num(total_knapsack)));

    // === 3. 孤立装备记录检测 ===
    let orphan_equip: i64 = conn
        .prepare("SELECT COUNT(*) FROM Equip_Register WHERE User NOT IN (SELECT DISTINCT ID FROM Basic_User)")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    if orphan_equip > 0 {
        issues.push(format!("⚠️ 孤立装备记录: {} 条", format_num(orphan_equip)));
    }

    let total_equip: i64 = conn
        .prepare("SELECT COUNT(*) FROM Equip_Register")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("⚔️ 装备记录总数: {}", format_num(total_equip)));

    // === 4. 孤立技能记录检测 ===
    let orphan_skill: i64 = conn
        .prepare("SELECT COUNT(*) FROM Skill_Register WHERE User NOT IN (SELECT DISTINCT ID FROM Basic_User)")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    if orphan_skill > 0 {
        issues.push(format!("⚠️ 孤立技能记录: {} 条", format_num(orphan_skill)));
    }

    let total_skill: i64 = conn
        .prepare("SELECT COUNT(*) FROM Skill_Register")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("✨ 技能记录总数: {}", format_num(total_skill)));

    // === 5. 经济异常检测 ===
    let avg_gold: f64 = conn
        .prepare("SELECT AVG(CAST(Data AS REAL)) FROM Basic_User WHERE Item='金币'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0.0);

    let max_gold: f64 = conn
        .prepare("SELECT MAX(CAST(Data AS REAL)) FROM Basic_User WHERE Item='金币'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0.0);

    let max_diamond: f64 = conn
        .prepare("SELECT MAX(CAST(Data AS REAL)) FROM Basic_User WHERE Item='钻石'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0.0);

    stats.push(format!("💰 平均金币: {}", format_num(avg_gold as i64)));
    stats.push(format!("💰 最高金币: {}", format_num(max_gold as i64)));
    stats.push(format!("💎 最高钻石: {}", format_num(max_diamond as i64)));

    // 金币异常检测 (超过平均值100倍)
    let gold_outliers: i64 = conn
        .prepare("SELECT COUNT(*) FROM Basic_User WHERE Item='金币' AND CAST(Data AS REAL) > ?1")
        .ok()
        .and_then(|mut s| s.query_row([avg_gold * 100.0], |r| r.get(0)).ok())
        .unwrap_or(0);

    if gold_outliers > 0 {
        issues.push(format!(
            "⚠️ 金币异常用户: {} 人（超过均值100倍）",
            format_num(gold_outliers)
        ));
    }

    // === 6. Global 表健康度 ===
    let global_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Global")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    let global_sections: i64 = conn
        .prepare("SELECT COUNT(DISTINCT SECTION) FROM Global")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);

    stats.push(format!(
        "🌍 Global记录: {} 条 ({} 个分区)",
        format_num(global_count),
        format_num(global_sections)
    ));

    // === 7. 公会数据一致性 ===
    let guild_users: i64 = conn
        .prepare("SELECT COUNT(DISTINCT ID) FROM Basic_User WHERE Item='公会' AND Data != '' AND Data IS NOT NULL")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("🏰 有公会的用户: {}", format_num(guild_users)));

    // === 8. Shared_Data 健康度 ===
    let shared_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Shared_Data")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("📊 Shared_Data记录: {}", format_num(shared_count)));

    // === 9. 任务注册数据 ===
    let task_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Task_Register")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("📜 任务注册记录: {}", format_num(task_count)));

    // === 10. 数据库表总数 ===
    let table_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
        .ok()
        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
        .unwrap_or(0);
    stats.push(format!("🗃️ 数据库表总数: {}", table_count));

    // === 汇总报告 ===
    let mut result = format!("{}\n🔍 === 数据完整性检查报告 ===\n", prefix);

    result.push_str("\n📊 数据统计:\n");
    for s in &stats {
        result.push_str(&format!("  {}\n", s));
    }

    if issues.is_empty() {
        result.push_str("\n✅ 恭喜！数据完整性检查通过，未发现异常。\n");
        result.push_str("  所有用户数据、背包记录、装备记录、技能记录均无孤立数据。\n");
    } else {
        result.push_str(&format!("\n⚠️ 发现 {} 个问题:\n", issues.len()));
        for issue in &issues {
            result.push_str(&format!("  {}\n", issue));
        }
        result.push_str("\n💡 修复建议:\n");
        result.push_str("  - 孤立记录: 可安全删除不存在用户的关联数据\n");
        result.push_str("  - 经济异常: 检查是否有刷金/复制漏洞\n");
    }

    result
}

/// 数据表详情 (GM)
pub fn cmd_table_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let conn = db.lock_conn();

    // 获取所有表名和行数
    let mut tables: Vec<(String, i64)> = Vec::new();
    if let Ok(mut stmt) = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name") {
        if let Ok(mut rows) = stmt.query([]) {
            while let Ok(Some(row)) = rows.next() {
                if let Ok(name) = row.get::<_, String>(0) {
                    let count: i64 = conn
                        .prepare(&format!("SELECT COUNT(*) FROM [{}]", name))
                        .ok()
                        .and_then(|mut s| s.query_row([], |r| r.get(0)).ok())
                        .unwrap_or(0);
                    tables.push((name, count));
                }
            }
        }
    }

    // 如果有参数，筛选表名
    let filter = args.trim().to_lowercase();
    let filtered: Vec<&(String, i64)> = if filter.is_empty() {
        tables.iter().collect()
    } else {
        tables
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&filter))
            .collect()
    };

    let mut result = format!("{}\n🗃️ === 数据库表详情 ===\n", prefix);

    if filtered.is_empty() {
        result.push_str(&format!("  未找到匹配 \"{}\" 的表。\n", args));
        return result;
    }

    result.push_str(&format!("  共 {} 张表", filtered.len()));
    if !filter.is_empty() {
        result.push_str(&format!(" (筛选: \"{}\")", args));
    }
    result.push_str(":\n\n");

    for (name, count) in &filtered {
        let icon = if *count == 0 {
            "📭"
        } else if *count < 10 {
            "📦"
        } else if *count < 100 {
            "📚"
        } else {
            "🗄️"
        };
        result.push_str(&format!("  {} {}: {} 行\n", icon, name, format_num(*count)));
    }

    let total_rows: i64 = filtered.iter().map(|(_, c)| c).sum();
    result.push_str(&format!("\n  📈 总记录数: {}", format_num(total_rows)));

    result
}

/// 修复孤立记录 (GM)
pub fn cmd_fix_orphans(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    if args.trim() != "确认" {
        return format!(
            "{}\n⚠️ 修复孤立记录\n\n此操作将删除以下数据:\n  - 不存在用户的背包记录\n  - 不存在用户的装备记录\n  - 不存在用户的技能记录\n\n⚠️ 此操作不可撤销！\n\n发送 \"修复孤立记录+确认\" 执行修复。",
            prefix
        );
    }

    let conn = db.lock_conn();
    let mut fixed = 0i64;

    // 删除孤立背包记录
    if let Ok(mut stmt) =
        conn.prepare("DELETE FROM Basic_knapsack WHERE ID NOT IN (SELECT DISTINCT ID FROM Basic_User)")
    {
        if let Ok(count) = stmt.execute([]) {
            fixed += count as i64;
        }
    }

    // 删除孤立装备记录
    if let Ok(mut stmt) =
        conn.prepare("DELETE FROM Equip_Register WHERE User NOT IN (SELECT DISTINCT ID FROM Basic_User)")
    {
        if let Ok(count) = stmt.execute([]) {
            fixed += count as i64;
        }
    }

    // 删除孤立技能记录
    if let Ok(mut stmt) =
        conn.prepare("DELETE FROM Skill_Register WHERE User NOT IN (SELECT DISTINCT ID FROM Basic_User)")
    {
        if let Ok(count) = stmt.execute([]) {
            fixed += count as i64;
        }
    }

    format!(
        "{}\n✅ 孤立记录修复完成！\n共清理 {} 条孤立记录。",
        prefix,
        format_num(fixed)
    )
}

/// 用户数据详情 (GM)
pub fn cmd_user_data(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = permissions::get_permission(db, user_id);
    if power < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 权限不足！需要管理员权限。", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n用法: 查看用户数据+用户ID\n示例: 查看用户数据+122421005", prefix);
    }

    let conn = db.lock_conn();

    // 检查用户是否存在
    let exists: bool = conn
        .prepare("SELECT COUNT(*) FROM Basic_User WHERE ID=?1")
        .ok()
        .and_then(|mut s| s.query_row([target], |r| r.get::<_, i64>(0)).ok())
        .unwrap_or(0)
        > 0;

    if !exists {
        return format!("{}\n❌ 用户 {} 不存在。", prefix, target);
    }

    let mut result = format!("{}\n👤 === 用户数据详情: {} ===\n", prefix, target);

    // 基础属性
    result.push_str("\n📋 基础属性:\n");
    if let Ok(mut stmt) = conn.prepare("SELECT Item, Data FROM Basic_User WHERE ID=?1 AND Node='Basic' ORDER BY Item") {
        if let Ok(mut rows) = stmt.query([target]) {
            while let Ok(Some(row)) = rows.next() {
                let item: String = row.get(0).unwrap_or_default();
                let data: String = row.get(1).unwrap_or_default();
                result.push_str(&format!("  {}: {}\n", item, data));
            }
        }
    }

    // 背包物品数
    let knapsack_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Basic_knapsack WHERE ID=?1")
        .ok()
        .and_then(|mut s| s.query_row([target], |r| r.get(0)).ok())
        .unwrap_or(0);
    result.push_str(&format!("\n🎒 背包物品: {} 种\n", knapsack_count));

    // 装备数
    let equip_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Equip_Register WHERE User=?1")
        .ok()
        .and_then(|mut s| s.query_row([target], |r| r.get(0)).ok())
        .unwrap_or(0);
    result.push_str(&format!("⚔️ 穿戴装备: {} 件\n", equip_count));

    // 技能数
    let skill_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Skill_Register WHERE User=?1")
        .ok()
        .and_then(|mut s| s.query_row([target], |r| r.get(0)).ok())
        .unwrap_or(0);
    result.push_str(&format!("✨ 已学技能: {} 个\n", skill_count));

    // Global 记录数
    let global_count: i64 = conn
        .prepare("SELECT COUNT(*) FROM Global WHERE ID LIKE ?1")
        .ok()
        .and_then(|mut s| s.query_row([format!("%{}%", target)], |r| r.get(0)).ok())
        .unwrap_or(0);
    result.push_str(&format!("🌍 关联Global记录: {} 条\n", global_count));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_num_zero() {
        assert_eq!(format_num(0), "0");
    }

    #[test]
    fn test_format_num_small() {
        assert_eq!(format_num(999), "999");
    }

    #[test]
    fn test_format_num_thousands() {
        assert_eq!(format_num(1000), "1,000");
    }

    #[test]
    fn test_format_num_large() {
        assert_eq!(format_num(1234567890), "1,234,567,890");
    }

    #[test]
    fn test_format_num_negative() {
        assert_eq!(format_num(-1234), "-1,234");
    }

    #[test]
    fn test_permission_level_admin() {
        assert_eq!(PERMISSION_LEVEL_ADMIN, 100);
    }
}

/// CakeGame 玩家屏蔽系统
/// 允许玩家屏蔽其他玩家，屏蔽后对方无法发送私聊/交易/赠送/公会邀请
/// 数据存储: Global 表 SECTION='player_block' 存储屏蔽列表
use crate::db::Database;
use crate::user;

/// 屏蔽列表最大数量
const MAX_BLOCKS: usize = 50;

/// 屏蔽分隔符
const BLOCK_SEP: &str = ",";

/// 获取玩家的屏蔽列表
pub fn get_blocked_list(db: &Database, user_id: &str) -> Vec<String> {
    let raw = db.global_get("player_block", user_id);
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(BLOCK_SEP)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 保存屏蔽列表
fn save_blocked_list(db: &Database, user_id: &str, blocked: &[String]) {
    db.global_set("player_block", user_id, &blocked.join(BLOCK_SEP));
}

/// 检查是否屏蔽了某个玩家
pub fn is_blocked(db: &Database, user_id: &str, target_id: &str) -> bool {
    let blocked = get_blocked_list(db, user_id);
    blocked.iter().any(|b| b == target_id)
}

/// 根据昵称或ID查找玩家
fn find_player(db: &Database, name_or_id: &str) -> Option<String> {
    // 先按ID查
    if db.user_exists(name_or_id) {
        return Some(name_or_id.to_string());
    }
    // 再按昵称查
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare("SELECT ID FROM Basic_User WHERE Name=?1") {
        if let Ok(mut rows) = stmt.query(rusqlite::params![name_or_id]) {
            if let Ok(Some(row)) = rows.next() {
                if let Ok(id) = row.get::<_, String>(0) {
                    return Some(id);
                }
            }
        }
    }
    None
}

/// 获取玩家昵称
fn get_player_name(db: &Database, user_id: &str) -> String {
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare("SELECT Name FROM Basic_User WHERE ID=?1") {
        if let Ok(mut rows) = stmt.query(rusqlite::params![user_id]) {
            if let Ok(Some(row)) = rows.next() {
                return row.get::<_, String>(0).unwrap_or_else(|_| user_id.to_string());
            }
        }
    }
    user_id.to_string()
}

/// 屏蔽玩家
pub fn cmd_block_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n请指定要屏蔽的玩家！\n用法: 屏蔽玩家+玩家昵称或ID", prefix);
    }

    let target_id = match find_player(db, target_name) {
        Some(id) => id,
        None => return format!("{}\n找不到玩家【{}】，请检查输入！", prefix, target_name),
    };

    // 不能屏蔽自己
    if target_id == user_id {
        return format!("{}\n不能屏蔽自己哦！", prefix);
    }

    let mut blocked = get_blocked_list(db, user_id);

    // 检查是否已屏蔽
    if blocked.contains(&target_id) {
        let name = get_player_name(db, &target_id);
        return format!("{}\n玩家【{}】已在你的屏蔽列表中。", prefix, name);
    }

    // 检查上限
    if blocked.len() >= MAX_BLOCKS {
        return format!("{}\n屏蔽列表已满（最多{}人），请先解除一些屏蔽。", prefix, MAX_BLOCKS);
    }

    blocked.push(target_id.clone());
    save_blocked_list(db, user_id, &blocked);

    let name = get_player_name(db, &target_id);
    format!(
        "{}\n🚫 已屏蔽玩家【{}】\n\n屏蔽后对方无法:\n  · 发送私聊\n  · 发起交易\n  · 赠送物品/金币/钻石\n  · 邀请加入公会\n\n发送【屏蔽列表】查看所有屏蔽\n发送【解除屏蔽+{}】可解除",
        prefix, name, name
    )
}

/// 解除屏蔽
pub fn cmd_unblock_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n请指定要解除屏蔽的玩家！\n用法: 解除屏蔽+玩家昵称或ID", prefix);
    }

    let target_id = match find_player(db, target_name) {
        Some(id) => id,
        None => return format!("{}\n找不到玩家【{}】，请检查输入！", prefix, target_name),
    };

    let mut blocked = get_blocked_list(db, user_id);

    if let Some(pos) = blocked.iter().position(|b| b == &target_id) {
        blocked.remove(pos);
        save_blocked_list(db, user_id, &blocked);
        let name = get_player_name(db, &target_id);
        format!("{}\n✅ 已解除对玩家【{}】的屏蔽。", prefix, name)
    } else {
        let name = get_player_name(db, &target_id);
        format!("{}\n玩家【{}】不在你的屏蔽列表中。", prefix, name)
    }
}

/// 查看屏蔽列表
pub fn cmd_block_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let blocked = get_blocked_list(db, user_id);

    let mut out = format!("{}\n═══ 屏蔽列表 ═══\n", prefix);

    if blocked.is_empty() {
        out.push_str("\n当前没有屏蔽任何玩家。\n");
        out.push_str("\n用法: 屏蔽玩家+玩家昵称或ID");
    } else {
        out.push_str(&format!("已屏蔽 {}/{} 人\n\n", blocked.len(), MAX_BLOCKS));
        for (i, uid) in blocked.iter().enumerate() {
            let name = get_player_name(db, uid);
            out.push_str(&format!("  {}. {} (ID:{})\n", i + 1, name, uid));
        }
        out.push_str("\n解除屏蔽: 解除屏蔽+玩家昵称或ID");
    }

    out
}

/// 屏蔽检查API - 供其他系统调用
/// 返回true表示target被user屏蔽，操作应被拒绝
pub fn check_blocked_and_reject(db: &Database, user_id: &str, target_id: &str) -> Option<String> {
    if is_blocked(db, target_id, user_id) {
        let target_name = get_player_name(db, target_id);
        Some(format!("操作失败：玩家【{}】已将你屏蔽。", target_name))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        // 创建必要的表（匹配实际schema）
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS Basic_User (ID TEXT, Name TEXT, Node TEXT, PRIMARY KEY(ID, Node));
             CREATE TABLE IF NOT EXISTS Global (ID TEXT, SECTION TEXT, DATA TEXT, PRIMARY KEY(ID, SECTION));",
        )
        .unwrap();
        Database {
            conn: std::sync::Mutex::new(conn),
        }
    }

    fn insert_user(conn: &Connection, id: &str, name: &str) {
        conn.execute(
            "INSERT INTO Basic_User (ID, Name, Node) VALUES (?1, ?2, '基础信息')",
            rusqlite::params![id, name],
        )
        .unwrap();
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_BLOCKS, 50);
        assert_eq!(BLOCK_SEP, ",");
    }

    #[test]
    fn test_empty_block_list() {
        let db = make_db();
        let list = get_blocked_list(&db, "user1");
        assert!(list.is_empty());
    }

    #[test]
    fn test_block_and_unblock() {
        let db = make_db();
        // 注册用户
        let conn = db.lock_conn();
        insert_user(&conn, "user1", "玩家A");
        insert_user(&conn, "user2", "玩家B");
        drop(conn);

        // 初始未屏蔽
        assert!(!is_blocked(&db, "user1", "user2"));

        // 屏蔽
        let mut blocked = get_blocked_list(&db, "user1");
        blocked.push("user2".to_string());
        save_blocked_list(&db, "user1", &blocked);

        // 确认已屏蔽
        assert!(is_blocked(&db, "user1", "user2"));
        assert!(get_blocked_list(&db, "user1").len() == 1);

        // 解除屏蔽
        let mut blocked = get_blocked_list(&db, "user1");
        blocked.retain(|b| b != "user2");
        save_blocked_list(&db, "user1", &blocked);

        // 确认已解除
        assert!(!is_blocked(&db, "user1", "user2"));
        assert!(get_blocked_list(&db, "user1").is_empty());
    }

    #[test]
    fn test_multiple_blocks() {
        let db = make_db();
        let conn = db.lock_conn();
        for i in 1..=5 {
            insert_user(&conn, &format!("user{}", i), &format!("玩家{}", i));
        }
        drop(conn);

        // 屏蔽多个玩家
        let blocked = vec!["user2".to_string(), "user3".to_string(), "user4".to_string()];
        save_blocked_list(&db, "user1", &blocked);

        assert!(is_blocked(&db, "user1", "user2"));
        assert!(is_blocked(&db, "user1", "user3"));
        assert!(is_blocked(&db, "user1", "user4"));
        assert!(!is_blocked(&db, "user1", "user5"));
        assert_eq!(get_blocked_list(&db, "user1").len(), 3);
    }

    #[test]
    fn test_block_is_directional() {
        let db = make_db();
        // user1屏蔽user2，但user2没有屏蔽user1
        let blocked = vec!["user2".to_string()];
        save_blocked_list(&db, "user1", &blocked);

        assert!(is_blocked(&db, "user1", "user2"));
        assert!(!is_blocked(&db, "user2", "user1"));
    }

    #[test]
    fn test_check_blocked_and_reject() {
        let db = make_db();
        let conn = db.lock_conn();
        insert_user(&conn, "user1", "玩家A");
        insert_user(&conn, "user2", "玩家B");
        drop(conn);

        // 未屏蔽时返回None
        assert!(check_blocked_and_reject(&db, "user1", "user2").is_none());

        // user2屏蔽user1
        let blocked = vec!["user1".to_string()];
        save_blocked_list(&db, "user2", &blocked);

        // user1发给user2应该被拒绝
        let result = check_blocked_and_reject(&db, "user1", "user2");
        assert!(result.is_some());
        assert!(result.unwrap().contains("屏蔽"));

        // 反向不受影响
        assert!(check_blocked_and_reject(&db, "user2", "user1").is_none());
    }

    #[test]
    fn test_find_player_by_id() {
        let db = make_db();
        let conn = db.lock_conn();
        insert_user(&conn, "user1", "玩家A");
        drop(conn);

        assert_eq!(find_player(&db, "user1"), Some("user1".to_string()));
        assert_eq!(find_player(&db, "不存在"), None);
    }

    #[test]
    fn test_find_player_by_name() {
        let db = make_db();
        let conn = db.lock_conn();
        insert_user(&conn, "user1", "玩家A");
        drop(conn);

        assert_eq!(find_player(&db, "玩家A"), Some("user1".to_string()));
        assert_eq!(find_player(&db, "不存在的"), None);
    }

    #[test]
    fn test_max_blocks_limit() {
        let db = make_db();
        // 创建达到上限的屏蔽列表
        let blocked: Vec<String> = (1..=MAX_BLOCKS).map(|i| format!("user{}", i)).collect();
        save_blocked_list(&db, "user0", &blocked);
        assert_eq!(get_blocked_list(&db, "user0").len(), MAX_BLOCKS);
    }

    #[test]
    fn test_get_player_name_fallback() {
        let db = make_db();
        // 不存在的用户应该返回ID本身
        let name = get_player_name(&db, "nonexistent");
        assert_eq!(name, "nonexistent");
    }
}

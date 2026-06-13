/// CakeGame 玩家留言板系统
/// 社交互动功能：给其他玩家写留言/查看留言/回复/删除
/// 数据来源: Global 表 SECTION='message_board' 存储留言记录
use crate::db::Database;
use crate::user;

const SECTION: &str = "message_board";
const MAX_MESSAGES: usize = 20; // 每人最多保留20条留言
const MAX_MSG_LEN: usize = 100; // 每条留言最多100字

/// 写留言 — 给其他玩家留言
/// 用法: 写留言+玩家ID+留言内容
pub fn cmd_write_message(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let parts: Vec<&str> = args.splitn(2, |c| ['+', ' '].contains(&c)).collect();
    if parts.len() < 2 || parts[0].trim().is_empty() {
        return format!(
            "{}\n📝 用法: 写留言+玩家ID+留言内容\n示例: 写留言+123456+你好呀！",
            prefix
        );
    }

    let target_id = parts[0].trim();
    let content = parts[1].trim();

    if target_id == user_id {
        return format!("{}\n❌ 不能给自己留言哦！", prefix);
    }

    if !db.user_exists(target_id) {
        return format!("{}\n❌ 玩家 {} 不存在！", prefix, target_id);
    }

    if content.is_empty() {
        return format!("{}\n❌ 留言内容不能为空！", prefix);
    }

    if content.len() > MAX_MSG_LEN {
        return format!(
            "{}\n❌ 留言内容过长！最多{}字（当前{}字）",
            prefix,
            MAX_MSG_LEN,
            content.len()
        );
    }

    // 检查留言数量上限
    let count = count_messages(db, target_id);
    if count >= MAX_MESSAGES {
        return format!(
            "{}\n❌ 对方留言箱已满（{}/{}条），请让对方清理后再留言。",
            prefix, count, MAX_MESSAGES
        );
    }

    let sender_name = db.read_basic(user_id, crate::core::ITEM_NAME);
    let target_name = db.read_basic(target_id, crate::core::ITEM_NAME);
    let timestamp = now_str();
    let msg_id = format!("{}.{}", target_id, djb2_hash(&format!("{}{}", user_id, timestamp)));

    // 存储留言: ID="{target_id}.{hash}", SECTION="message_board"
    // DATA="{sender_id}|{sender_name}|{content}|{timestamp}|{read}"
    let data = format!("{}|{}|{}|{}|false", user_id, sender_name, content, timestamp);
    db.global_set(SECTION, &msg_id, &data);

    format!(
        "{}\n📝 留言成功！\n\n💬 给 [{}] 的留言:\n\"{}\"\n\n✅ 对方下次查看留言板时会看到。",
        prefix, target_name, content
    )
}

/// 查看留言 — 查看自己收到的留言
/// 用法: 查看留言 或 查看留言+页码
pub fn cmd_view_messages(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let messages = get_messages_for(db, user_id);
    if messages.is_empty() {
        return format!(
            "{}\n📭 你的留言板为空，暂无留言。\n💡 让朋友发送 写留言+你的ID+内容 给你留言吧！",
            prefix
        );
    }

    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    let page_size = 5;
    let total = messages.len();
    let total_pages = total.div_ceil(page_size);
    let page = page.min(total_pages);
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let unread = messages.iter().filter(|m| !m.read).count();
    let mut out = format!(
        "{}\n📬 === 你的留言板 === ({}/{}页)\n📨 共{}条留言 ({}条未读)\n",
        prefix, page, total_pages, total, unread
    );

    for (i, msg) in messages[start..end].iter().enumerate() {
        let idx = start + i + 1;
        let read_icon = if msg.read { "📭" } else { "📬" };
        out.push_str(&format!(
            "\n{} #{} 来自 [{}]\n   \"{}\"\n   🕐 {}",
            read_icon, idx, msg.sender_name, msg.content, msg.timestamp
        ));
    }

    // 标记为已读
    mark_all_read(db, user_id);

    if total_pages > 1 {
        out.push_str(
            "

📖 发送0027查看留言+页码0027翻页",
        );
    }

    out
}

/// 回复留言 — 回复收到的留言
/// 用法: 回复留言+序号+回复内容
pub fn cmd_reply_message(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let messages = get_messages_for(db, user_id);
    if messages.is_empty() {
        return format!("{}\n📭 你的留言板为空，没有可回复的留言。", prefix);
    }

    let parts: Vec<&str> = args.splitn(2, |c| ['+', ' '].contains(&c)).collect();
    if parts.len() < 2 {
        return format!("{}\n📝 用法: 回复留言+序号+回复内容\n示例: 回复留言+1+谢谢！", prefix);
    }

    let idx: usize = match parts[0].trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= messages.len() => n - 1,
        _ => return format!("{}\n❌ 无效序号！请输入 1-{} 之间的数字。", prefix, messages.len()),
    };

    let reply_content = parts[1].trim();
    if reply_content.is_empty() {
        return format!("{}\n❌ 回复内容不能为空！", prefix);
    }

    if reply_content.len() > MAX_MSG_LEN {
        return format!("{}\n❌ 回复内容过长！最多{}字。", prefix, MAX_MSG_LEN);
    }

    let target_msg = &messages[idx];
    let my_name = db.read_basic(user_id, crate::core::ITEM_NAME);

    // 自动给回复者写一条留言
    let count = count_messages(db, &target_msg.sender_id);
    if count >= MAX_MESSAGES {
        return format!("{}\n❌ 对方留言箱已满，无法发送回复。", prefix);
    }

    let timestamp = now_str();
    let msg_id = format!(
        "{}.{}",
        target_msg.sender_id,
        djb2_hash(&format!("{}{}", user_id, timestamp))
    );
    let data = format!("{}|{}|📨回复: {}|{}|false", user_id, my_name, reply_content, timestamp);
    db.global_set(SECTION, &msg_id, &data);

    format!(
        "{}\n💬 回复成功！\n\n📤 回复 [{}] 的留言:\n\"{}\"\n\n✅ 对方查看留言板时会看到你的回复。",
        prefix, target_msg.sender_name, reply_content
    )
}

/// 删除留言 — 删除自己收到的指定留言
/// 用法: 删除留言+序号
pub fn cmd_delete_message(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let messages = get_messages_for(db, user_id);
    if messages.is_empty() {
        return format!("{}\n📭 你的留言板为空，没有可删除的留言。", prefix);
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= messages.len() => n - 1,
        _ => return format!("{}\n❌ 无效序号！请输入 1-{} 之间的数字。", prefix, messages.len()),
    };

    let msg = &messages[idx];
    // 从 Global 表删除
    let _ = db.conn.lock().unwrap().execute(
        "DELETE FROM Global WHERE SECTION = ?1 AND ID LIKE ?2",
        rusqlite::params![SECTION, format!("%{}%", msg.msg_id)],
    );
    // 精确删除
    let _ = db.conn.lock().unwrap().execute(
        "DELETE FROM Global WHERE SECTION = ?1 AND ID = ?2",
        rusqlite::params![SECTION, msg.msg_id],
    );

    format!(
        "{}\n🗑️ 已删除来自 [{}] 的留言:\n\"{}\"",
        prefix, msg.sender_name, msg.content
    )
}

/// 清空留言 — 清空自己收到的所有留言
/// 用法: 清空留言
pub fn cmd_clear_messages(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let count = count_messages(db, user_id);
    if count == 0 {
        return format!("{}\n📭 你的留言板已经是空的了。", prefix);
    }

    let _ = db.conn.lock().unwrap().execute(
        &format!(
            "DELETE FROM Global WHERE SECTION = '{}' AND ID LIKE '{}.%'",
            SECTION, user_id
        ),
        [],
    );

    format!("{}\n🗑️ 已清空所有{}条留言。", prefix, count)
}

/// 我发出的留言 — 查看自己给别人发的留言记录
/// 用法: 我的留言
pub fn cmd_my_sent_messages(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let sent = get_sent_messages(db, user_id);
    if sent.is_empty() {
        return format!("{}\n📭 你还没有给别人留过言。", prefix);
    }

    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    let page_size = 5;
    let total = sent.len();
    let total_pages = total.div_ceil(page_size);
    let page = page.min(total_pages);
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let mut out = format!(
        "{}\n📤 === 你的发出留言 === ({}/{}页)\n📨 共{}条\n",
        prefix, page, total_pages, total
    );

    for (i, msg) in sent[start..end].iter().enumerate() {
        let idx = start + i + 1;
        let read_icon = if msg.read { "✅已读" } else { "⏳未读" };
        out.push_str(&format!(
            "\n#{} → [{}]\n   \"{}\"\n   🕐 {} | {}",
            idx, msg.recipient_name, msg.content, msg.timestamp, read_icon
        ));
    }

    out
}

// ========== 内部数据结构 ==========

struct Message {
    msg_id: String,
    sender_id: String,
    sender_name: String,
    content: String,
    timestamp: String,
    read: bool,
}

struct SentMessage {
    #[allow(dead_code)]
    msg_id: String,
    recipient_name: String,
    content: String,
    timestamp: String,
    read: bool,
}

// ========== 内部辅助函数 ==========

fn get_messages_for(db: &Database, user_id: &str) -> Vec<Message> {
    let pattern = format!("{}.%", user_id);
    let mut messages = Vec::new();
    if let Ok(conn) = db.conn.lock() {
        if let Ok(mut stmt) = conn.prepare(&format!(
            "SELECT ID, DATA FROM Global WHERE SECTION = '{}' AND ID LIKE ?1 ORDER BY ROWID DESC",
            SECTION
        )) {
            if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                for row in rows.flatten() {
                    let (id, data) = row;
                    let parts: Vec<&str> = data.splitn(5, '|').collect();
                    if parts.len() >= 5 {
                        messages.push(Message {
                            msg_id: id,
                            sender_id: parts[0].to_string(),
                            sender_name: parts[1].to_string(),
                            content: parts[2].to_string(),
                            timestamp: parts[3].to_string(),
                            read: parts[4] == "true",
                        });
                    }
                }
            }
        }
    }
    messages
}

fn get_sent_messages(db: &Database, user_id: &str) -> Vec<SentMessage> {
    let mut sent = Vec::new();
    if let Ok(conn) = db.conn.lock() {
        if let Ok(mut stmt) = conn.prepare(&format!(
            "SELECT ID, DATA FROM Global WHERE SECTION = '{}' ORDER BY ROWID DESC",
            SECTION
        )) {
            if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
                for row in rows.flatten() {
                    let (id, data) = row;
                    let parts: Vec<&str> = data.splitn(5, '|').collect();
                    if parts.len() >= 5 && parts[0] == user_id {
                        let target_id = id.split('.').next().unwrap_or("");
                        let recipient_name = if db.user_exists(target_id) {
                            db.read_basic(target_id, crate::core::ITEM_NAME)
                        } else {
                            target_id.to_string()
                        };
                        sent.push(SentMessage {
                            msg_id: id,
                            recipient_name,
                            content: parts[2].to_string(),
                            timestamp: parts[3].to_string(),
                            read: parts[4] == "true",
                        });
                    }
                }
            }
        }
    }
    sent
}

fn count_messages(db: &Database, user_id: &str) -> usize {
    let pattern = format!("{}.%", user_id);
    let result = db.conn.lock().unwrap().query_row(
        &format!(
            "SELECT COUNT(*) FROM Global WHERE SECTION = '{}' AND ID LIKE ?1",
            SECTION
        ),
        rusqlite::params![pattern],
        |row| row.get::<_, i64>(0),
    );
    result.unwrap_or(0) as usize
}

fn mark_all_read(db: &Database, user_id: &str) {
    let pattern = format!("{}.%", user_id);
    let _ = db.conn.lock().unwrap().execute(
        &format!(
            "UPDATE Global SET DATA = \
             CASE WHEN DATA LIKE '%|false' \
             THEN SUBSTR(DATA, 1, LENGTH(DATA) - 5) || 'true' \
             ELSE DATA END \
             WHERE SECTION = '{}' AND ID LIKE ?1 AND DATA LIKE '%|false'",
            SECTION
        ),
        rusqlite::params![pattern],
    );
}

/// 简单 DJB2 哈希
fn djb2_hash(s: &str) -> String {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    format!("{:08x}", hash & 0xFFFFFFFF)
}

fn now_str() -> String {
    // 简化版时间戳 — 使用秒级时间戳
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_djb2_hash_deterministic() {
        let h1 = djb2_hash("hello");
        let h2 = djb2_hash("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_djb2_hash_different() {
        let h1 = djb2_hash("hello");
        let h2 = djb2_hash("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_djb2_hash_length() {
        let h = djb2_hash("test");
        assert_eq!(h.len(), 8);
    }

    #[test]
    fn test_max_messages_limit() {
        assert_eq!(MAX_MESSAGES, 20);
    }

    #[test]
    fn test_max_msg_len() {
        assert_eq!(MAX_MSG_LEN, 100);
    }

    #[test]
    fn test_now_str_returns_string() {
        let s = now_str();
        assert!(!s.is_empty());
        assert!(s.parse::<u64>().is_ok());
    }

    #[test]
    fn test_section_name() {
        assert_eq!(SECTION, "message_board");
    }
}

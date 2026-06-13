/// CakeGame 私聊系统
/// 玩家间实时私信交流：发送私聊/查看私聊/私聊记录/删除私聊
/// 数据存储: Global 表 SECTION='whisper' 存储私聊消息
use crate::db::Database;
use crate::user;

/// 私聊消息存储 section
const SECTION: &str = "whisper";
/// 私聊冷却（秒）
const COOLDOWN_SECS: u64 = 3;
/// 单条消息最大长度
const MAX_MSG_LEN: usize = 200;

/// 获取当前时间戳
fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// 格式化时间戳
fn format_time(ts: u64) -> String {
    let s = ts % 60;
    let m = (ts / 60) % 60;
    let h = (ts / 3600) % 24;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// 生成消息 key: sender_receiver_timestamp_seq
fn msg_key(a: &str, b: &str, ts: u64, seq: u32) -> String {
    format!("{}_{}_{}_{}", a, b, ts, seq)
}

/// 根据昵称查找用户ID
fn find_user_id_by_name(db: &Database, name: &str) -> String {
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare("SELECT ID FROM Basic_User WHERE Name=?1") {
        if let Ok(mut rows) = stmt.query(rusqlite::params![name]) {
            if let Ok(Some(row)) = rows.next() {
                if let Ok(id) = row.get::<_, String>(0) {
                    return id;
                }
            }
        }
    }
    String::new()
}

/// 私聊发送消息
/// 指令: 私聊+玩家ID+消息内容
pub fn cmd_whisper_send(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先发送 注册+昵称 进行注册。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n═══ 💬 发送私聊 ═══\n\
             用法: 私聊+玩家ID+消息内容\n\n\
             示例: 私聊+张三+你好呀！\n\n\
             💡 发送'查看私聊'查看收件箱\n\
             📝 发送'私聊记录+玩家ID'查看与某人的聊天记录",
            prefix
        );
    }

    // 解析参数: 玩家ID+消息内容
    let parts: Vec<&str> = args.splitn(2, '+').collect();
    if parts.len() < 2 {
        return format!("{}\n❌ 用法错误！正确格式: 私聊+玩家ID+消息内容", prefix);
    }

    let target_name = parts[0].trim();
    let message = parts[1].trim();

    if target_name.is_empty() || message.is_empty() {
        return format!("{}\n❌ 玩家ID和消息内容不能为空！", prefix);
    }

    if message.len() > MAX_MSG_LEN {
        return format!(
            "{}\n❌ 消息过长！最多{}个字符（当前{}个）",
            prefix,
            MAX_MSG_LEN,
            message.len()
        );
    }

    // 不能给自己发消息
    let sender_name = db.read_basic(user_id, crate::core::ITEM_NAME);
    if target_name == sender_name || target_name == user_id {
        return format!("{}\n❌ 不能给自己发私聊消息！", prefix);
    }

    // 查找目标玩家
    let target_id = find_user_id_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n❌ 未找到玩家'{}'！请确认玩家昵称是否正确。", prefix, target_name);
    }

    // 屏蔽检查
    if let Some(reject_msg) = crate::block::check_blocked_and_reject(db, user_id, &target_id) {
        return format!("{}\n❌ {}", prefix, reject_msg);
    }
    if crate::block::is_blocked(db, user_id, &target_id) {
        return format!("{}\n❌ 你已屏蔽该玩家，请先解除屏蔽后再发送私聊。", prefix);
    }

    // 检查冷却
    let cooldown_key = format!("cd_{}", user_id);
    let last_send: u64 = db.global_get(SECTION, &cooldown_key).parse().unwrap_or(0);
    let now = now_ts();
    if now < last_send + COOLDOWN_SECS {
        return format!(
            "{}\n⏳ 发送太频繁！请{}秒后再试。",
            prefix,
            last_send + COOLDOWN_SECS - now
        );
    }

    // 存储消息
    let ts = now;
    let seq_key = format!("seq_{}_{}", user_id, target_id);
    let seq = db.global_get(SECTION, &seq_key).parse::<u32>().unwrap_or(0) + 1;

    // 发送方视角 (sent)
    let sent_key = msg_key(user_id, &target_id, ts, seq);
    let sent_data = format!("S|{}|{}|{}", user_id, message, ts);
    db.global_set(SECTION, &sent_key, &sent_data);

    // 接收方视角 (inbox)
    let inbox_key = msg_key(&target_id, user_id, ts, seq);
    let inbox_data = format!("R|{}|{}|{}|0", user_id, message, ts);
    db.global_set(SECTION, &inbox_key, &inbox_data);

    // 更新序号
    db.global_set(SECTION, &seq_key, &seq.to_string());

    // 更新冷却
    db.global_set(SECTION, &cooldown_key, &now.to_string());

    // 更新未读计数
    let unread_key = format!("unread_{}", target_id);
    let unread: u32 = db.global_get(SECTION, &unread_key).parse().unwrap_or(0);
    db.global_set(SECTION, &unread_key, &(unread + 1).to_string());

    format!("{}\n✅ 私聊已发送给 [{}]\n💬 内容: {}", prefix, target_name, message)
}

/// 查看私聊收件箱
/// 指令: 查看私聊
pub fn cmd_whisper_inbox(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先发送 注册+昵称 进行注册。", prefix);
    }

    let unread_key = format!("unread_{}", user_id);
    let unread: u32 = db.global_get(SECTION, &unread_key).parse().unwrap_or(0);

    // 获取最近消息
    let messages = get_recent_messages(db, user_id, 15);

    let mut out = format!("{}\n═══ 💬 私聊信箱 ═══", prefix);

    if unread > 0 {
        out.push_str(&format!("\n🔔 未读消息: {}条", unread));
    }

    if messages.is_empty() {
        out.push_str("\n\n📭 暂无私聊消息\n");
        out.push_str("💡 发送'私聊+玩家ID+消息'给其他玩家发消息");
    } else {
        out.push_str(&format!("\n📦 最近{}条消息:\n", messages.len()));
        for (i, msg) in messages.iter().enumerate() {
            let direction = if msg.is_sent { "📤→" } else { "📥←" };
            let read_mark = if !msg.is_sent && !msg.read { " 🆕" } else { "" };
            let other = if msg.is_sent {
                &msg.receiver_name
            } else {
                &msg.sender_name
            };
            let content_preview = truncate_str(&msg.content, 30);
            let time_str = format_time(msg.timestamp);
            out.push_str(&format!(
                "\n{}. {} [{}]{}: {} ({})",
                i + 1,
                direction,
                other,
                read_mark,
                content_preview,
                time_str
            ));
        }
        out.push_str("\n\n💡 发送'私聊记录+玩家名'查看完整对话");
        out.push_str("\n🗑️ 发送'删除私聊+玩家名'清空对话");
    }

    // 重置未读
    db.global_set(SECTION, &unread_key, "0");

    out
}

/// 查看与特定玩家的聊天记录
/// 指令: 私聊记录+玩家名
pub fn cmd_whisper_history(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n用法: 私聊记录+玩家名\n💡 先发送'查看私聊'查看所有对话列表", prefix);
    }

    let target_id = find_user_id_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n❌ 未找到玩家'{}'！", prefix, target_name);
    }

    let messages = get_conversation(db, user_id, &target_id, 20);

    let mut out = format!("{}\n═══ 💬 与 [{}] 的对话 ═══", prefix, target_name);

    if messages.is_empty() {
        out.push_str("\n\n📭 暂无对话记录\n");
        out.push_str(&format!("💡 发送'私聊+{}+消息'开始对话", target_name));
    } else {
        out.push_str(&format!("\n📦 最近{}条消息:\n", messages.len()));
        for msg in &messages {
            let (arrow, name) = if msg.sender_id == user_id {
                ("📤", "我")
            } else {
                ("📥", target_name)
            };
            out.push_str(&format!(
                "\n{} {}: {} ({})",
                arrow,
                name,
                msg.content,
                format_time(msg.timestamp)
            ));
        }
        out.push_str(&format!("\n\n💬 发送'私聊+{}+消息'继续对话", target_name));
    }

    out
}

/// 删除与特定玩家的聊天记录
/// 指令: 删除私聊+玩家名
pub fn cmd_whisper_delete(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n用法: 删除私聊+玩家名", prefix);
    }

    let target_id = find_user_id_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n❌ 未找到玩家'{}'！", prefix, target_name);
    }

    let deleted = delete_conversation(db, user_id, &target_id);

    format!("{}\n✅ 已删除与 [{}] 的{}条私聊记录。", prefix, target_name, deleted)
}

// ==================== 内部数据结构 ====================

struct WhisperMsg {
    sender_id: String,
    sender_name: String,
    receiver_id: String,
    receiver_name: String,
    content: String,
    timestamp: u64,
    is_sent: bool,
    read: bool,
}

/// 根据ID获取昵称
fn get_name_by_id(db: &Database, uid: &str) -> String {
    let name = db.read_basic(uid, crate::core::ITEM_NAME);
    if name.is_empty() {
        uid.to_string()
    } else {
        name
    }
}

/// 获取最近消息（混合收发，按时间倒序）
fn get_recent_messages(db: &Database, user_id: &str, limit: usize) -> Vec<WhisperMsg> {
    let mut messages = Vec::new();
    let conn = db.lock_conn();

    // 查找与用户相关的所有消息（通过 key 模式匹配）
    let pattern_sent = format!("{}_%", user_id);
    let pattern_recv = format!("%_{}", user_id);

    if let Ok(mut stmt) = conn.prepare(
        "SELECT ID, DATA FROM Global WHERE Section=?1 \
         AND (ID LIKE ?2 ESCAPE '\\' OR ID LIKE ?3 ESCAPE '\\') \
         ORDER BY ID DESC LIMIT ?4",
    ) {
        let _ = stmt
            .query_map(
                rusqlite::params![SECTION, pattern_sent, pattern_recv, (limit * 3) as i64],
                |row| {
                    let key: String = row.get(0)?;
                    let data: String = row.get(1)?;
                    Ok((key, data))
                },
            )
            .map(|rows| {
                for row in rows.flatten() {
                    if let Some(msg) = parse_whisper_msg(&row.0, &row.1, user_id) {
                        messages.push(msg);
                    }
                }
            });
    }

    // 按时间倒序
    messages.sort_by_key(|a| std::cmp::Reverse(a.timestamp));
    messages.truncate(limit);

    // 补充昵称
    let _ids: Vec<String> = messages
        .iter()
        .flat_map(|m| vec![m.sender_id.clone(), m.receiver_id.clone()])
        .collect();
    drop(conn); // 释放锁再查昵称

    for msg in &mut messages {
        if msg.sender_name.is_empty() {
            msg.sender_name = get_name_by_id(db, &msg.sender_id);
        }
        if msg.receiver_name.is_empty() {
            msg.receiver_name = get_name_by_id(db, &msg.receiver_id);
        }
    }

    messages
}

/// 获取与特定玩家的对话
fn get_conversation(db: &Database, user_id: &str, target_id: &str, limit: usize) -> Vec<WhisperMsg> {
    let mut messages = Vec::new();
    let conn = db.lock_conn();

    let pattern1 = format!("{}_{}%", user_id, target_id);
    let pattern2 = format!("{}_{}%", target_id, user_id);

    if let Ok(mut stmt) = conn.prepare(
        "SELECT ID, DATA FROM Global WHERE Section=?1 \
         AND (ID LIKE ?2 ESCAPE '\\' OR ID LIKE ?3 ESCAPE '\\') \
         ORDER BY ID ASC LIMIT ?4",
    ) {
        let _ = stmt
            .query_map(
                rusqlite::params![SECTION, pattern1, pattern2, (limit * 2) as i64],
                |row| {
                    let key: String = row.get(0)?;
                    let data: String = row.get(1)?;
                    Ok((key, data))
                },
            )
            .map(|rows| {
                for row in rows.flatten() {
                    if let Some(msg) = parse_whisper_msg(&row.0, &row.1, user_id) {
                        messages.push(msg);
                    }
                }
            });
    }

    messages.sort_by_key(|a| a.timestamp);
    messages.truncate(limit);

    let target_name = get_name_by_id(db, target_id);
    let my_name = get_name_by_id(db, user_id);
    for msg in &mut messages {
        if msg.sender_id == user_id {
            msg.sender_name = my_name.clone();
            msg.receiver_name = target_name.clone();
        } else {
            msg.sender_name = target_name.clone();
            msg.receiver_name = my_name.clone();
        }
    }

    messages
}

/// 解析消息
fn parse_whisper_msg(key: &str, data: &str, viewer_id: &str) -> Option<WhisperMsg> {
    // data 格式: S|sender|content|timestamp 或 R|sender|content|timestamp|read
    let parts: Vec<&str> = data.splitn(5, '|').collect();
    if parts.len() < 4 {
        return None;
    }
    let is_sent = parts[0] == "S";
    let sender_id = parts[1].to_string();
    let content = parts[2].to_string();
    let timestamp: u64 = parts[3].parse().unwrap_or(0);
    let read = if parts.len() >= 5 { parts[4] == "1" } else { true };

    // 从 key 解析: sender_receiver_ts_seq
    let key_parts: Vec<&str> = key.splitn(4, '_').collect();
    if key_parts.len() < 4 {
        return None;
    }
    let k_sender = key_parts[0];
    let k_receiver = key_parts[1];

    let (real_sender, real_receiver) = if is_sent {
        (k_sender.to_string(), k_receiver.to_string())
    } else {
        (sender_id.clone(), viewer_id.to_string())
    };

    Some(WhisperMsg {
        sender_id: real_sender,
        sender_name: String::new(),
        receiver_id: real_receiver,
        receiver_name: String::new(),
        content,
        timestamp,
        is_sent,
        read,
    })
}

/// 删除对话
fn delete_conversation(db: &Database, user_id: &str, target_id: &str) -> usize {
    let conn = db.lock_conn();
    let p1 = format!("{}_{}%", user_id, target_id);
    let p2 = format!("{}_{}%", target_id, user_id);
    conn.execute(
        "DELETE FROM Global WHERE Section=?1 AND (ID LIKE ?2 ESCAPE '\\' OR ID LIKE ?3 ESCAPE '\\')",
        rusqlite::params![SECTION, p1, p2],
    )
    .unwrap_or(0)
}

/// 截断字符串
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_key_format() {
        let key = msg_key("user1", "user2", 1000, 1);
        assert_eq!(key, "user1_user2_1000_1");
    }

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(0), "00:00:00");
        assert_eq!(format_time(65), "00:01:05");
        assert_eq!(format_time(3661), "01:01:01");
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world!", 8), "hello...");
        assert_eq!(truncate_str("hi", 5), "hi");
    }

    #[test]
    fn test_truncate_chinese() {
        assert_eq!(truncate_str("你好世界", 3), "...");
        assert_eq!(truncate_str("你好世界", 5), "你好世界");
    }

    #[test]
    fn test_parse_whisper_sent() {
        let msg = parse_whisper_msg("u1_u2_1000_1", "S|u1|你好|1000", "u1");
        assert!(msg.is_some());
        let m = msg.unwrap();
        assert_eq!(m.sender_id, "u1");
        assert_eq!(m.receiver_id, "u2");
        assert_eq!(m.content, "你好");
        assert!(m.is_sent);
    }

    #[test]
    fn test_parse_whisper_received() {
        let msg = parse_whisper_msg("u2_u1_1000_1", "R|u2|你好|1000|0", "u1");
        assert!(msg.is_some());
        let m = msg.unwrap();
        assert_eq!(m.sender_id, "u2");
        assert!(!m.is_sent);
        assert!(!m.read);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_whisper_msg("bad", "bad", "u1").is_none());
        assert!(parse_whisper_msg("a_b_c_d", "X|y", "u1").is_none());
    }

    #[test]
    fn test_constants() {
        assert!(COOLDOWN_SECS > 0);
        assert!(MAX_MSG_LEN >= 50);
        assert_eq!(SECTION, "whisper");
    }
}

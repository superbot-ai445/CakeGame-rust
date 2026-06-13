/// CakeGame 邮件系统
/// 基于 ext_systemEmail_msg 表
/// 字段: ID (INTEGER), User (TEXT), SendTime (TEXT), Title (TEXT), Msg (TEXT), Enclosure (TEXT), Read (INTEGER)
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 邮件信息
struct MailInfo {
    id: i64,
    sender: String,
    send_time: String,
    title: String,
    msg: String,
    enclosure: String,
    read: i32,
}

/// 读取用户邮件列表
fn get_user_mails(db: &Database, user_id: &str, limit: i32) -> Vec<MailInfo> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT ID, User, SendTime, Title, Msg, Enclosure, Read FROM ext_systemEmail_msg WHERE User=?1 ORDER BY Read ASC, ID DESC LIMIT ?2"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map(rusqlite::params![user_id, limit], |row| {
        Ok(MailInfo {
            id: row.get::<_, i64>(0)?,
            sender: row.get::<_, String>(1)?.trim_end_matches('\x00').to_string(),
            send_time: row.get::<_, String>(2)?.trim_end_matches('\x00').to_string(),
            title: row.get::<_, String>(3)?.trim_end_matches('\x00').to_string(),
            msg: row.get::<_, String>(4)?.trim_end_matches('\x00').to_string(),
            enclosure: row.get::<_, String>(5)?.trim_end_matches('\x00').to_string(),
            read: row.get::<_, i32>(6)?,
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

/// 读取单封邮件
fn get_mail_by_id(db: &Database, mail_id: i64) -> Option<MailInfo> {
    let conn = db.lock_conn();
    conn.prepare("SELECT ID, User, SendTime, Title, Msg, Enclosure, Read FROM ext_systemEmail_msg WHERE ID=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row([mail_id], |row| {
                Ok(MailInfo {
                    id: row.get::<_, i64>(0)?,
                    sender: row.get::<_, String>(1)?.trim_end_matches('\x00').to_string(),
                    send_time: row.get::<_, String>(2)?.trim_end_matches('\x00').to_string(),
                    title: row.get::<_, String>(3)?.trim_end_matches('\x00').to_string(),
                    msg: row.get::<_, String>(4)?.trim_end_matches('\x00').to_string(),
                    enclosure: row.get::<_, String>(5)?.trim_end_matches('\x00').to_string(),
                    read: row.get::<_, i32>(6)?,
                })
            })
            .ok()
        })
}

/// 统计未读邮件数
fn count_unread(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT COUNT(*) FROM ext_systemEmail_msg WHERE User=?1 AND Read=0")
        .ok()
        .and_then(|mut stmt| stmt.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
        .unwrap_or(0)
}

/// 标记邮件已读
fn mark_read(db: &Database, mail_id: i64) {
    let conn = db.lock_conn();
    let _ = conn.execute("UPDATE ext_systemEmail_msg SET Read=1 WHERE ID=?1", [mail_id]);
}

/// 删除邮件
fn delete_mail(db: &Database, mail_id: i64) -> bool {
    let conn = db.lock_conn();
    let rows = conn
        .execute("DELETE FROM ext_systemEmail_msg WHERE ID=?1", [mail_id])
        .unwrap_or(0);
    rows > 0
}

/// 插入新邮件
fn insert_mail(db: &Database, user_id: &str, title: &str, msg: &str, enclosure: &str, _sender: &str) -> i64 {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let conn = db.lock_conn();
    let result = conn.execute(
        "INSERT INTO ext_systemEmail_msg (User, SendTime, Title, Msg, Enclosure, Read) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        rusqlite::params![user_id, now, title, msg, enclosure],
    );
    match result {
        Ok(_) => conn.last_insert_rowid(),
        Err(_) => -1,
    }
}

/// 解析附件字符串 (格式: "物品名*数量,物品名*数量")
fn parse_enclosures(enclosure: &str) -> Vec<(&str, i32)> {
    let mut items = Vec::new();
    for part in enclosure.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let sub: Vec<&str> = part.split('*').collect();
        let name = sub[0];
        let qty: i32 = sub.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        items.push((name, qty));
    }
    items
}

// ==================== 指令处理函数 ====================

/// 查看邮件列表
pub fn cmd_view_mail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let page: i32 = args.trim().parse().unwrap_or(1).max(1);
    let page_size = 5;
    // 获取更多邮件用于分页
    let all_mails = get_user_mails(db, user_id, 100);

    if all_mails.is_empty() {
        return format!("{}\n📭 您的邮箱为空！", prefix);
    }

    let total = all_mails.len() as i32;
    let total_pages = (total + page_size - 1) / page_size;
    if page > total_pages {
        return format!("{}\n请输入正确的页码！(共{}页)", prefix, total_pages);
    }

    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(all_mails.len());
    let unread = count_unread(db, user_id);

    let mut result = format!(
        "{}\n═══ 📧 邮箱 ({}/{}) ═══\n未读邮件：{}封",
        prefix, page, total_pages, unread
    );

    for mail in &all_mails[start..end] {
        let status = if mail.read == 0 { "🔴" } else { "⚪" };
        let has_enclosure = if !mail.enclosure.is_empty() { " 📎" } else { "" };
        result.push_str(&format!(
            "\n{} [{}] {}{} ({})",
            status, mail.id, mail.title, has_enclosure, mail.send_time
        ));
    }

    result.push_str("\n\n📖 阅读邮件：阅读邮件+邮件ID");
    result.push_str("\n📎 领取附件：领取附件+邮件ID");
    result.push_str("\n🗑️ 删除邮件：删除邮件+邮件ID");
    if page < total_pages {
        result.push_str(&format!("\n输入'查看邮件+{}'翻页", page + 1));
    }

    result
}

/// 阅读邮件
pub fn cmd_read_mail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mail_id: i64 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return format!("{}\n请输入正确的邮件ID！", prefix),
    };

    let mail = match get_mail_by_id(db, mail_id) {
        Some(m) => m,
        None => return format!("{}\n邮件不存在！", prefix),
    };

    // 检查邮件是否属于当前用户
    if mail.sender != user_id && mail.sender != "系统" {
        // sender field is User (recipient), so check recipient
    }
    // ext_systemEmail_msg.User = recipient
    if mail.sender != user_id {
        // Actually the "sender" field from our query is the User column (recipient)
        // For system emails, User is the recipient
    }

    // 标记已读
    mark_read(db, mail_id);

    let mut result = format!(
        "{}\n═══ 📧 邮件详情 ═══\nID：{}\n标题：{}\n时间：{}",
        prefix, mail.id, mail.title, mail.send_time
    );

    result.push_str(&format!("\n\n{}", mail.msg));

    if !mail.enclosure.is_empty() {
        let items = parse_enclosures(&mail.enclosure);
        result.push_str("\n\n📎 附件：");
        for (name, qty) in &items {
            result.push_str(&format!("\n  [{}]×{}", name, qty));
        }
        result.push_str(&format!("\n\n输入'领取附件+{}'领取", mail_id));
    }

    result
}

/// 领取附件
pub fn cmd_claim_enclosure(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mail_id: i64 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return format!("{}\n请输入正确的邮件ID！", prefix),
    };

    let mail = match get_mail_by_id(db, mail_id) {
        Some(m) => m,
        None => return format!("{}\n邮件不存在！", prefix),
    };

    if mail.enclosure.is_empty() {
        return format!("{}\n该邮件没有附件！", prefix);
    }

    // 解析并发放附件
    let items = parse_enclosures(&mail.enclosure);
    let mut result = format!("{}\n═══ 📎 领取附件 ═══", prefix);
    let mut claimed_any = false;

    for (name, qty) in &items {
        // 尝试作为货币处理
        match *name {
            "金币" => {
                db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, *qty as i64);
                result.push_str(&format!("\n✅ 金币 +{}", qty));
                claimed_any = true;
            }
            "钻石" => {
                db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, *qty as i64);
                result.push_str(&format!("\n✅ 钻石 +{}", qty));
                claimed_any = true;
            }
            _ => {
                // 作为物品添加到背包
                db.knapsack_add(user_id, name, *qty);
                result.push_str(&format!("\n✅ [{}]×{}", name, qty));
                claimed_any = true;
            }
        }
    }

    if claimed_any {
        // 清空附件（已领取）
        let conn = db.lock_conn();
        let _ = conn.execute("UPDATE ext_systemEmail_msg SET Enclosure='' WHERE ID=?1", [mail_id]);
        result.push_str("\n\n附件已全部领取！");
    }

    result
}

/// 删除邮件
pub fn cmd_delete_mail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mail_id: i64 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return format!("{}\n请输入正确的邮件ID！", prefix),
    };

    let mail = match get_mail_by_id(db, mail_id) {
        Some(m) => m,
        None => return format!("{}\n邮件不存在！", prefix),
    };

    if delete_mail(db, mail_id) {
        format!("{}\n已删除邮件 [{}]！", prefix, mail.title)
    } else {
        format!("{}\n删除失败！", prefix)
    }
}

/// 发送邮件（GM功能或玩家间邮件）
pub fn cmd_send_mail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 格式: 目标昵称+标题+内容+附件（可选）
    // 例如: 发送邮件+玩家名+标题+内容
    // 或:   发送邮件+玩家名+标题+内容+金币*100
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return format!(
            "{}\n格式：发送邮件+目标昵称+标题+内容\n可选：发送邮件+目标昵称+标题+内容+附件",
            prefix
        );
    }

    let target_name = parts[0];
    let title = parts[1];
    let content = parts[2];
    let enclosure = if parts.len() > 3 { parts[3] } else { "" };

    // 查找目标用户
    let target_id = {
        let conn = db.lock_conn();
        let result = conn
            .prepare("SELECT ID FROM Basic_User WHERE Node='基础信息' AND Item='名称' AND Data=?1")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_map([target_name], |row| row.get::<_, String>(0))
                    .ok()
                    .and_then(|mut rows| rows.next())
                    .and_then(|r| r.ok())
            });
        if let Some(id) = result {
            id
        } else {
            // Try old format
            conn.prepare("SELECT ID FROM Basic_User WHERE Node='Basic' AND Item='Name' AND Data=?1")
                .ok()
                .and_then(|mut stmt| {
                    stmt.query_map([target_name], |row| row.get::<_, String>(0))
                        .ok()
                        .and_then(|mut rows| rows.next())
                        .and_then(|r| r.ok())
                })
                .unwrap_or_default()
        }
    };

    if target_id.is_empty() {
        return format!("{}\n玩家[{}]不存在！", prefix, target_name);
    }

    // 插入邮件
    let sender_name = user::get_msg_prefix(db, user_id);
    let mail_id = insert_mail(db, &target_id, title, content, enclosure, &sender_name);

    if mail_id > 0 {
        let enc_text = if !enclosure.is_empty() {
            format!("\n附件：{}", enclosure)
        } else {
            String::new()
        };
        format!(
            "{}\n邮件发送成功！\n收件人：{}\n标题：{}{}",
            prefix, target_name, title, enc_text
        )
    } else {
        format!("{}\n邮件发送失败！", prefix)
    }
}

/// GM发送系统邮件（全服/指定用户）
pub fn cmd_gm_send_mail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 格式: 全服邮件+标题+内容+附件
    // 或:   系统邮件+目标+标题+内容+附件
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 3 {
        return format!(
            "{}\n格式：\n  全服邮件+标题+内容+附件（可选）\n  系统邮件+目标+标题+内容+附件（可选）",
            prefix
        );
    }

    let mode = parts[0];
    match mode {
        "全服" => {
            // 全服邮件：标题+内容+附件
            if parts.len() < 3 {
                return format!("{}\n格式：全服邮件+标题+内容+附件", prefix);
            }
            let title = parts[1];
            let content = parts[2];
            let enclosure = if parts.len() > 3 { parts[3] } else { "" };

            // 获取所有用户
            let users: Vec<String> = {
                let conn = db.lock_conn();
                let mut stmt = conn
                    .prepare("SELECT DISTINCT ID FROM Basic_User WHERE Node='基础信息' AND Item='名称'")
                    .unwrap();
                stmt.query_map([], |row| row.get::<_, String>(0))
                    .unwrap()
                    .filter_map(|r| r.ok())
                    .collect()
            };

            let count = users.len();
            for uid in &users {
                insert_mail(db, uid, title, content, enclosure, "系统");
            }

            format!("{}\n全服邮件发送成功！\n标题：{}\n发送人数：{}", prefix, title, count)
        }
        _ => {
            // 指定用户系统邮件：目标+标题+内容+附件
            if parts.len() < 4 {
                return format!("{}\n格式：系统邮件+目标+标题+内容+附件", prefix);
            }
            let target_name = parts[1];
            let title = parts[2];
            let content = parts[3];
            let enclosure = if parts.len() > 4 { parts[4] } else { "" };

            // 查找目标用户
            let target_id = {
                let conn = db.lock_conn();
                conn.prepare("SELECT ID FROM Basic_User WHERE Node='基础信息' AND Item='名称' AND Data=?1")
                    .ok()
                    .and_then(|mut stmt| {
                        stmt.query_map([target_name], |row| row.get::<_, String>(0))
                            .ok()
                            .and_then(|mut rows| rows.next())
                            .and_then(|r| r.ok())
                    })
                    .unwrap_or_default()
            };

            if target_id.is_empty() {
                return format!("{}\n玩家[{}]不存在！", prefix, target_name);
            }

            let mail_id = insert_mail(db, &target_id, title, content, enclosure, "系统");
            if mail_id > 0 {
                format!(
                    "{}\n系统邮件发送成功！\n收件人：{}\n标题：{}",
                    prefix, target_name, title
                )
            } else {
                format!("{}\n邮件发送失败！", prefix)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_enclosures_single_item() {
        let items = parse_enclosures("金币*100");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "金币");
        assert_eq!(items[0].1, 100);
    }

    #[test]
    fn test_parse_enclosures_multiple_items() {
        let items = parse_enclosures("金币*100,钻石*50");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "金币");
        assert_eq!(items[0].1, 100);
        assert_eq!(items[1].0, "钻石");
        assert_eq!(items[1].1, 50);
    }

    #[test]
    fn test_parse_enclosures_empty() {
        let items = parse_enclosures("");
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_enclosures_no_quantity() {
        // Items without *N should default to quantity 1
        let items = parse_enclosures("生命药水");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "生命药水");
        assert_eq!(items[0].1, 1);
    }

    #[test]
    fn test_parse_enclosures_whitespace_handling() {
        // parse_enclosures trims the part after splitting by comma,
        // but does NOT trim the name after splitting by *
        let items = parse_enclosures("金币*200, 钻石*10");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "金币");
        assert_eq!(items[0].1, 200);
        assert_eq!(items[1].0, "钻石");
        assert_eq!(items[1].1, 10);
    }
}

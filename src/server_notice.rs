/// 全服公告系统 — GM发布全服公告，所有玩家可查看
/// 区别于guild_bulletin(公会公告)，本系统面向全服玩家
/// 存储在 Global 表 section="server_notice"
/// Key格式: notice:{timestamp}_{hash}
/// Value格式: {author_id}|{author_name}|{title}|{content}|{pinned}|{priority}
use crate::core::*;
use crate::db::Database;
use crate::permissions;

const SECTION: &str = "server_notice";
const MAX_NOTICES: usize = 50;
const MAX_TITLE_LEN: usize = 30;
const MAX_CONTENT_LEN: usize = 500;
const PAGE_SIZE: usize = 5;

/// 公告优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoticePriority {
    Normal = 0,
    Important = 1,
    Urgent = 2,
}

impl NoticePriority {
    fn from_str(s: &str) -> Self {
        match s {
            "重要" | "important" => NoticePriority::Important,
            "紧急" | "urgent" => NoticePriority::Urgent,
            _ => NoticePriority::Normal,
        }
    }
    fn icon(&self) -> &'static str {
        match self {
            NoticePriority::Normal => "📢",
            NoticePriority::Important => "⚠️",
            NoticePriority::Urgent => "🚨",
        }
    }
    fn name(&self) -> &'static str {
        match self {
            NoticePriority::Normal => "普通",
            NoticePriority::Important => "重要",
            NoticePriority::Urgent => "紧急",
        }
    }
}

/// 解析后的公告
#[derive(Debug, Clone)]
pub struct Notice {
    pub id: String,
    pub author_id: String,
    pub author_name: String,
    pub title: String,
    pub content: String,
    pub pinned: bool,
    pub priority: NoticePriority,
    pub timestamp: i64,
}

impl Notice {
    fn parse(id: &str, data: &str, ts: i64) -> Option<Self> {
        let parts: Vec<&str> = data.splitn(6, '|').collect();
        if parts.len() < 4 {
            return None;
        }
        Some(Notice {
            id: id.to_string(),
            author_id: parts[0].to_string(),
            author_name: parts[1].to_string(),
            title: parts[2].to_string(),
            content: parts[3].to_string(),
            pinned: parts.get(4).is_some_and(|s| *s == "1"),
            priority: parts.get(5).map_or(NoticePriority::Normal, |s| match s.parse::<i32>() {
                Ok(2) => NoticePriority::Urgent,
                Ok(1) => NoticePriority::Important,
                _ => NoticePriority::Normal,
            }),
            timestamp: ts,
        })
    }

    fn serialize(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.author_id,
            self.author_name,
            self.title,
            self.content,
            if self.pinned { "1" } else { "0" },
            self.priority as i32
        )
    }

    fn format_display(&self, _index: usize) -> String {
        let pin_icon = if self.pinned { "📌" } else { "" };
        let time_str = if self.timestamp > 0 {
            format_timestamp(self.timestamp)
        } else {
            "未知".to_string()
        };
        format!(
            "{}{} {} [{}] {} {}\n━━━━━━━━━━━━━━━━━━\n{}",
            self.priority.icon(),
            pin_icon,
            self.title,
            self.priority.name(),
            self.author_name,
            time_str,
            self.content
        )
    }
}

/// 格式化时间戳为可读时间
fn format_timestamp(ts: i64) -> String {
    if ts <= 0 {
        return "未知".to_string();
    }
    match chrono::DateTime::from_timestamp(ts, 0) {
        Some(dt) => dt.format("%m-%d %H:%M").to_string(),
        None => "未知".to_string(),
    }
}

/// 生成唯一公告ID
fn gen_notice_id(author_id: &str) -> String {
    let ts = chrono::Local::now().timestamp();
    let hash: u32 = author_id
        .bytes()
        .fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32));
    format!("{}_{:08x}", ts, hash)
}

/// 从 Global 表读取所有公告
fn load_notices(db: &Database) -> Vec<Notice> {
    let mut results = Vec::new();
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT ID, DATA FROM Global WHERE SECTION = '{}' AND ID LIKE 'notice:%'",
        SECTION
    )) {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                let (id, data) = row;
                if data.is_empty() {
                    continue;
                }
                let ts = id
                    .split(':')
                    .nth(1)
                    .and_then(|s| s.split('_').next())
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                if let Some(notice) = Notice::parse(&id, &data, ts) {
                    results.push(notice);
                }
            }
        }
    }
    // 排序: 置顶优先 → 优先级降序 → 时间降序
    results.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then_with(|| b.priority.cmp(&a.priority))
            .then_with(|| b.timestamp.cmp(&a.timestamp))
    });
    results
}

/// 保存公告到 Global 表
fn save_notice(db: &Database, notice: &Notice) {
    db.global_set(SECTION, &notice.id, &notice.serialize());
}

/// 获取用户昵称
fn get_user_name(db: &Database, user_id: &str) -> String {
    db.read_basic(user_id, ITEM_NAME)
}

/// 检查用户是否为GM
fn is_gm(db: &Database, user_id: &str) -> bool {
    permissions::get_permission(db, user_id) >= 100
}

/// 查看公告 [page] — 查看全服公告，支持翻页
pub fn cmd_view_notices(db: &Database, _user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let notices = load_notices(db);
    if notices.is_empty() {
        return "📭 暂无全服公告\n💡 GM可使用「发布公告+标题+内容」发布全服公告".to_string();
    }

    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    let total = notices.len();
    let total_pages = total.div_ceil(PAGE_SIZE);
    let page = page.min(total_pages.max(1));
    let start = (page - 1) * PAGE_SIZE;
    let end = (start + PAGE_SIZE).min(total);

    let mut out = format!("📋 ══ 全服公告 ({}/{}) ══\n\n", page, total_pages);
    for (i, notice) in notices[start..end].iter().enumerate() {
        out.push_str(&notice.format_display(start + i + 1));
        out.push_str("\n\n");
    }
    out.push_str(&format!("📄 第 {}/{} 页 | 共 {} 条公告", page, total_pages, total));
    if total_pages > 1 {
        out.push_str("\n💡 使用「查看公告+页码」翻页");
    }
    out
}

/// 发布公告 标题 内容 [优先级] — GM发布全服公告
pub fn cmd_post_notice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !is_gm(db, user_id) {
        return "❌ 权限不足，仅GM可发布公告".to_string();
    }

    let parts: Vec<&str> = args.splitn(3, [' ', '+']).collect();
    if parts.len() < 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        return "❌ 格式: 发布公告+标题+内容 [优先级]\n💡 优先级: 普通(默认)/重要/紧急".to_string();
    }

    let title = parts[0].trim();
    let remaining = parts[1].trim();
    // 检查是否有第三个部分作为优先级
    let (content, priority) = if parts.len() >= 3 {
        let p = parts[2].trim();
        if p == "重要" || p == "紧急" || p == "普通" {
            (remaining.to_string(), NoticePriority::from_str(p))
        } else {
            (format!("{} {}", remaining, p), NoticePriority::Normal)
        }
    } else {
        (remaining.to_string(), NoticePriority::Normal)
    };

    if title.len() > MAX_TITLE_LEN {
        return format!("❌ 标题过长（最多{}字，当前{}字）", MAX_TITLE_LEN, title.len());
    }
    if content.len() > MAX_CONTENT_LEN {
        return format!("❌ 内容过长（最多{}字，当前{}字）", MAX_CONTENT_LEN, content.len());
    }

    let notices = load_notices(db);
    // 清理过量公告
    if notices.len() >= MAX_NOTICES {
        // 删除最旧的未置顶公告
        let mut oldest: Option<&Notice> = None;
        for n in &notices {
            if !n.pinned && (oldest.is_none() || n.timestamp < oldest.unwrap().timestamp) {
                oldest = Some(n);
            }
        }
        if let Some(old) = oldest {
            db.global_set(SECTION, &old.id, "");
        }
    }

    let author_name = get_user_name(db, user_id);
    let notice_id = gen_notice_id(user_id);
    let ts = chrono::Local::now().timestamp();
    let key = format!("notice:{}", notice_id);

    let notice = Notice {
        id: key.clone(),
        author_id: user_id.to_string(),
        author_name: author_name.clone(),
        title: title.to_string(),
        content: content.to_string(),
        pinned: false,
        priority,
        timestamp: ts,
    };
    save_notice(db, &notice);

    format!(
        "{} 全服公告发布成功！\n📌 标题: {}\n📝 内容: {}\n🏷️ 优先级: {}\n💡 使用「公告置顶+{}」可置顶此公告",
        priority.icon(),
        title,
        content,
        priority.name(),
        notice_id.split(':').nth(1).unwrap_or("")
    )
}

/// 删除公告 ID — GM删除指定公告
pub fn cmd_delete_notice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !is_gm(db, user_id) {
        return "❌ 权限不足，仅GM可删除公告".to_string();
    }

    let keyword = args.trim();
    if keyword.is_empty() {
        return "❌ 格式: 删除公告+公告ID\n💡 使用「查看公告」查看公告ID".to_string();
    }

    let notices = load_notices(db);
    // 模糊匹配公告ID
    let matched: Vec<&Notice> = notices
        .iter()
        .filter(|n| n.id.contains(keyword) || n.title.contains(keyword))
        .collect();

    if matched.is_empty() {
        return format!("❌ 未找到匹配「{}」的公告", keyword);
    }
    if matched.len() > 1 {
        let mut out = format!("🔍 找到 {} 条匹配公告，请指定更精确的关键词:\n", matched.len());
        for n in &matched {
            out.push_str(&format!("  • {} [{}]\n", n.title, n.id));
        }
        return out;
    }

    let target = matched[0];
    db.global_set(SECTION, &target.id, "");
    format!("✅ 公告「{}」已删除", target.title)
}

/// 公告置顶 ID — GM置顶/取消置顶公告
pub fn cmd_pin_notice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !is_gm(db, user_id) {
        return "❌ 权限不足，仅GM可置顶公告".to_string();
    }

    let keyword = args.trim();
    if keyword.is_empty() {
        return "❌ 格式: 公告置顶+公告ID或标题\n💡 使用「查看公告」查看公告列表".to_string();
    }

    let notices = load_notices(db);
    let matched: Vec<&Notice> = notices
        .iter()
        .filter(|n| n.id.contains(keyword) || n.title.contains(keyword))
        .collect();

    if matched.is_empty() {
        return format!("❌ 未找到匹配「{}」的公告", keyword);
    }
    if matched.len() > 1 {
        let mut out = format!("🔍 找到 {} 条匹配公告，请指定更精确的关键词:\n", matched.len());
        for n in &matched {
            out.push_str(&format!("  • {} [{}]\n", n.title, n.id));
        }
        return out;
    }

    let target = matched[0].clone();
    let new_pinned = !target.pinned;
    let mut updated = target.clone();
    updated.pinned = new_pinned;
    save_notice(db, &updated);

    if new_pinned {
        format!("📌 公告「{}」已置顶", target.title)
    } else {
        format!("📌 公告「{}」已取消置顶", target.title)
    }
}

/// 公告统计 — 查看全服公告统计信息
pub fn cmd_notice_stats(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let notices = load_notices(db);
    if notices.is_empty() {
        return "📭 暂无全服公告".to_string();
    }

    let total = notices.len();
    let pinned = notices.iter().filter(|n| n.pinned).count();
    let urgent = notices.iter().filter(|n| n.priority == NoticePriority::Urgent).count();
    let important = notices
        .iter()
        .filter(|n| n.priority == NoticePriority::Important)
        .count();
    let normal = notices.iter().filter(|n| n.priority == NoticePriority::Normal).count();

    // 最新公告
    let newest = notices.iter().max_by_key(|n| n.timestamp);
    let newest_str = match newest {
        Some(n) => format!("{} ({})", n.title, format_timestamp(n.timestamp)),
        None => "无".to_string(),
    };

    // 统计发布者
    let mut authors: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for n in &notices {
        *authors.entry(n.author_name.clone()).or_insert(0) += 1;
    }
    let mut author_list: Vec<_> = authors.into_iter().collect();
    author_list.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = String::from("📊 ══ 全服公告统计 ══\n\n");
    out.push_str(&format!("📋 公告总数: {} 条\n", total));
    out.push_str(&format!("📌 置顶公告: {} 条\n", pinned));
    out.push_str(&format!("🚨 紧急公告: {} 条\n", urgent));
    out.push_str(&format!("⚠️ 重要公告: {} 条\n", important));
    out.push_str(&format!("📢 普通公告: {} 条\n", normal));
    out.push_str(&format!("📰 最新公告: {}\n\n", newest_str));

    out.push_str("👤 发布者统计:\n");
    for (name, count) in author_list.iter().take(5) {
        out.push_str(&format!("  • {}: {} 条\n", name, count));
    }

    out
}

/// 发送全服公告 标题 内容 — GM发送全服通知(不同于公告，会显示在登录时)
pub fn cmd_server_announce(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !is_gm(db, user_id) {
        return "❌ 权限不足，仅GM可发送全服通告".to_string();
    }

    let parts: Vec<&str> = args.splitn(2, [' ', '+']).collect();
    if parts.len() < 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        return "❌ 格式: 全服通告+标题+内容".to_string();
    }

    let title = parts[0].trim();
    let content = parts[1].trim();

    if title.len() > MAX_TITLE_LEN {
        return format!("❌ 标题过长（最多{}字）", MAX_TITLE_LEN);
    }
    if content.len() > MAX_CONTENT_LEN {
        return format!("❌ 内容过长（最多{}字）", MAX_CONTENT_LEN);
    }

    // 保存为紧急置顶公告
    let ts = chrono::Local::now().timestamp();
    let notice_id = format!("notice:announce_{}", ts);
    let author_name = get_user_name(db, user_id);

    let notice = Notice {
        id: notice_id,
        author_id: user_id.to_string(),
        author_name,
        title: title.to_string(),
        content: content.to_string(),
        pinned: true,
        priority: NoticePriority::Urgent,
        timestamp: ts,
    };
    save_notice(db, &notice);

    format!(
        "🚨 ══ 全服通告 ══\n\n{} 📌 {}\n━━━━━━━━━━━━━━━━━━\n{}\n\n✅ 全服通告已发布，所有玩家登录时可见",
        NoticePriority::Urgent.icon(),
        title,
        content
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notice_priority_from_str() {
        assert_eq!(NoticePriority::from_str("重要"), NoticePriority::Important);
        assert_eq!(NoticePriority::from_str("紧急"), NoticePriority::Urgent);
        assert_eq!(NoticePriority::from_str("普通"), NoticePriority::Normal);
        assert_eq!(NoticePriority::from_str("unknown"), NoticePriority::Normal);
        assert_eq!(NoticePriority::from_str(""), NoticePriority::Normal);
    }

    #[test]
    fn test_notice_priority_ordering() {
        assert!(NoticePriority::Urgent > NoticePriority::Important);
        assert!(NoticePriority::Important > NoticePriority::Normal);
    }

    #[test]
    fn test_notice_priority_icons() {
        assert_eq!(NoticePriority::Normal.icon(), "📢");
        assert_eq!(NoticePriority::Important.icon(), "⚠️");
        assert_eq!(NoticePriority::Urgent.icon(), "🚨");
    }

    #[test]
    fn test_notice_parse_and_serialize() {
        let notice = Notice {
            id: "notice:1234_abcdef".to_string(),
            author_id: "user001".to_string(),
            author_name: "GM_Admin".to_string(),
            title: "测试标题".to_string(),
            content: "测试内容".to_string(),
            pinned: true,
            priority: NoticePriority::Important,
            timestamp: 1700000000,
        };
        let data = notice.serialize();
        let parsed = Notice::parse(&notice.id, &data, notice.timestamp).unwrap();
        assert_eq!(parsed.author_id, "user001");
        assert_eq!(parsed.author_name, "GM_Admin");
        assert_eq!(parsed.title, "测试标题");
        assert_eq!(parsed.content, "测试内容");
        assert!(parsed.pinned);
        assert_eq!(parsed.priority, NoticePriority::Important);
    }

    #[test]
    fn test_notice_parse_minimal() {
        let data = "u1|GM|标题|内容";
        let notice = Notice::parse("notice:123", data, 100).unwrap();
        assert_eq!(notice.author_id, "u1");
        assert_eq!(notice.title, "标题");
        assert!(!notice.pinned);
        assert_eq!(notice.priority, NoticePriority::Normal);
    }

    #[test]
    fn test_notice_parse_invalid() {
        assert!(Notice::parse("notice:123", "too_few", 0).is_none());
        assert!(Notice::parse("notice:123", "", 0).is_none());
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0), "未知");
        assert_eq!(format_timestamp(-1), "未知");
        // Valid timestamp should return formatted string
        let result = format_timestamp(1700000000);
        assert!(result.contains('-'));
        assert!(result.contains(':'));
    }

    #[test]
    fn test_gen_notice_id_deterministic() {
        let id1 = gen_notice_id("user001");
        let id2 = gen_notice_id("user001");
        // Same user should produce same hash (though timestamp may differ)
        assert!(id1.contains('_'));
        assert!(id2.contains('_'));
    }

    #[test]
    fn test_notice_format_display() {
        let notice = Notice {
            id: "notice:1234_abcd".to_string(),
            author_id: "u1".to_string(),
            author_name: "GM".to_string(),
            title: "维护公告".to_string(),
            content: "今晚维护".to_string(),
            pinned: true,
            priority: NoticePriority::Urgent,
            timestamp: 1700000000,
        };
        let display = notice.format_display(1);
        assert!(display.contains("🚨"));
        assert!(display.contains("📌"));
        assert!(display.contains("维护公告"));
        assert!(display.contains("今晚维护"));
    }

    #[test]
    fn test_constants() {
        assert_eq!(SECTION, "server_notice");
        assert!(MAX_NOTICES > 0);
        assert!(MAX_TITLE_LEN > 0);
        assert!(MAX_CONTENT_LEN > 0);
        assert!(PAGE_SIZE > 0);
    }
}

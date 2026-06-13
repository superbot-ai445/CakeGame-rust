/// 公会公告系统 — 公会管理层发布通知，成员查看公告
/// 存储在 Global 表 section="guild_bulletin"
/// Key格式: {guild_name}:bulletin:{timestamp}_{hash}
/// Value格式: {author_id}|{author_name}|{content}|{pinned}
use crate::core::*;
use crate::db::Database;

const MAX_BULLETINS: usize = 20;
const MAX_CONTENT_LEN: usize = 200;
const SECTION: &str = "guild_bulletin";

/// 获取用户所在公会名，失败返回 None
fn get_user_guild(db: &Database, user_id: &str) -> Option<String> {
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        None
    } else {
        Some(guild)
    }
}

/// 检查是否为公会会长
fn is_guild_owner(db: &Database, user_id: &str, guild: &str) -> bool {
    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    owner == user_id
}

/// 生成唯一公告ID (基于时间戳+哈希)
fn gen_bulletin_id(user_id: &str) -> String {
    let ts = chrono::Local::now().timestamp();
    let hash: u32 = user_id
        .bytes()
        .fold(5381u32, |h, b| h.wrapping_mul(33).wrapping_add(b as u32));
    format!("{}_{:08x}", ts, hash)
}

/// 从 Global 表读取公会的所有公告
fn load_bulletins(db: &Database, guild: &str) -> Vec<(String, String, String)> {
    // (key, value, ts_str)
    let prefix = format!("{}:bulletin:", guild);
    let mut results = Vec::new();
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT ID, DATA FROM Global WHERE SECTION = '{}' AND ID LIKE ?1",
        SECTION
    )) {
        let pattern = format!("{}%", prefix);
        if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            for row in rows.flatten() {
                let (id, data) = row;
                if data.is_empty() {
                    continue;
                }
                let ts_str = id
                    .split(':')
                    .nth(2)
                    .and_then(|s| s.split('_').next())
                    .unwrap_or("0")
                    .to_string();
                results.push((id, data, ts_str));
            }
        }
    }
    results
}

/// 发布公会公告 — 会长可发布，内容最长200字
pub fn cmd_post_bulletin(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let content = args.trim();

    if content.is_empty() {
        return format!("{}\n⚠️ 请输入公告内容，例: 发布公告+今日20点公会战集合", prefix);
    }
    if content.len() > MAX_CONTENT_LEN {
        return format!(
            "{}\n⚠️ 公告内容过长！最多{}字，当前{}字",
            prefix,
            MAX_CONTENT_LEN,
            content.len()
        );
    }

    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n⚠️ 您未加入任何公会！", prefix),
    };

    if !is_guild_owner(db, user_id, &guild) {
        return format!("{}\n⚠️ 只有公会会长才能发布公告！", prefix);
    }

    // 检查公告数量上限
    let existing = load_bulletins(db, &guild);
    if existing.len() >= MAX_BULLETINS {
        // 删除最旧的非置顶公告
        let mut oldest: Option<(String, i64)> = None;
        for (key, val, ts_str) in &existing {
            if val.ends_with("|pinned") {
                continue;
            }
            let ts = ts_str.parse::<i64>().unwrap_or(0);
            if oldest.is_none() || ts < oldest.as_ref().unwrap().1 {
                oldest = Some((key.clone(), ts));
            }
        }
        if let Some((old_key, _)) = oldest {
            db.global_set(SECTION, &old_key, "");
        }
    }

    let author_name = crate::user::get_msg_prefix(db, user_id);
    let bulletin_id = gen_bulletin_id(user_id);
    let key = format!("{}:bulletin:{}", guild, bulletin_id);
    let value = format!("{}|{}|{}|unpinned", user_id, author_name, content);
    db.global_set(SECTION, &key, &value);

    format!(
        "{}\n📢 公会公告发布成功！\n━━━━━━━━━━━━━━━━━━━━\n📜 {}\n━━━━━━━━━━━━━━━━━━━━\n💡 成员发送「查看公会公告」即可查看",
        prefix, content
    )
}

/// 查看公会公告 — 按置顶优先+时间倒序显示
pub fn cmd_view_bulletins(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n⚠️ 您未加入任何公会！", prefix),
    };

    let raw = load_bulletins(db, &guild);
    let mut bulletins: Vec<(i64, String, String, bool)> = Vec::new(); // (ts, author, content, pinned)

    for (_key, data, ts_str) in &raw {
        let parts: Vec<&str> = data.splitn(4, '|').collect();
        if parts.len() < 4 {
            continue;
        }
        let ts = ts_str.parse::<i64>().unwrap_or(0);
        let pinned = parts[3] == "pinned";
        bulletins.push((ts, parts[1].to_string(), parts[2].to_string(), pinned));
    }

    if bulletins.is_empty() {
        return format!(
            "{}\n📢 {} 暂无公告\n💡 会长发送「发布公告+内容」发布新公告",
            prefix, guild
        );
    }

    // 置顶优先，同优先级按时间倒序
    bulletins.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| b.0.cmp(&a.0)));

    let mut out = format!("{}\n📢 【{}公会公告】\n━━━━━━━━━━━━━━━━━━━━\n", prefix, guild);

    for (i, (ts, author, content, pinned)) in bulletins.iter().take(10).enumerate() {
        let time_str = if *ts > 0 {
            chrono::DateTime::from_timestamp(*ts, 0)
                .map(|d| d.naive_local())
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "??:??".to_string())
        } else {
            "??:??".to_string()
        };
        let pin_icon = if *pinned { "📌" } else { "  " };
        out.push_str(&format!(
            "{}{}. [{}] {}\n   📝 {}\n",
            pin_icon,
            i + 1,
            time_str,
            author,
            content
        ));
    }

    if bulletins.len() > 10 {
        out.push_str(&format!("... 还有{}条公告\n", bulletins.len() - 10));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 会长: 发布公告+内容 | 删除公告+编号 | 置顶公告+编号\n");
    out
}

/// 删除公会公告 — 会长可删除指定编号的公告
pub fn cmd_delete_bulletin(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n⚠️ 您未加入任何公会！", prefix),
    };

    if !is_guild_owner(db, user_id, &guild) {
        return format!("{}\n⚠️ 只有公会会长才能删除公告！", prefix);
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n > 0 => n - 1,
        _ => return format!("{}\n⚠️ 请输入有效的公告编号，例: 删除公告+1", prefix),
    };

    let raw = load_bulletins(db, &guild);
    let mut entries: Vec<(String, i64)> = Vec::new(); // (key, ts)
    for (key, _data, ts_str) in &raw {
        let ts = ts_str.parse::<i64>().unwrap_or(0);
        entries.push((key.clone(), ts));
    }

    // 同排序逻辑: 按时间倒序
    entries.sort_by_key(|b| std::cmp::Reverse(b.1));

    if idx >= entries.len() {
        return format!("{}\n⚠️ 公告编号无效！当前共{}条公告", prefix, entries.len());
    }

    db.global_set(SECTION, &entries[idx].0, "");
    format!("{}\n✅ 公告 #{} 已删除！", prefix, idx + 1)
}

/// 置顶/取消置顶公会公告
pub fn cmd_pin_bulletin(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n⚠️ 您未加入任何公会！", prefix),
    };

    if !is_guild_owner(db, user_id, &guild) {
        return format!("{}\n⚠️ 只有公会会长才能置顶公告！", prefix);
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n > 0 => n - 1,
        _ => return format!("{}\n⚠️ 请输入有效的公告编号，例: 置顶公告+1", prefix),
    };

    let raw = load_bulletins(db, &guild);
    let mut entries: Vec<(String, String, i64)> = Vec::new(); // (key, value, ts)
    for (key, data, ts_str) in &raw {
        let ts = ts_str.parse::<i64>().unwrap_or(0);
        entries.push((key.clone(), data.clone(), ts));
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.2));

    if idx >= entries.len() {
        return format!("{}\n⚠️ 公告编号无效！当前共{}条公告", prefix, entries.len());
    }

    let (key, val, _) = &entries[idx];
    let new_val = if val.ends_with("|pinned") {
        val.replace("|pinned", "|unpinned")
    } else {
        val.replace("|unpinned", "|pinned")
    };
    let is_pinned = new_val.ends_with("|pinned");
    db.global_set(SECTION, key, &new_val);

    format!(
        "{}\n{} 公告 #{} 已{}！",
        prefix,
        if is_pinned { "📌" } else { "📍" },
        idx + 1,
        if is_pinned { "置顶" } else { "取消置顶" }
    )
}

/// 公会公告统计 — 公会公告总数和最新公告时间
pub fn cmd_bulletin_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n⚠️ 您未加入任何公会！", prefix),
    };

    let raw = load_bulletins(db, &guild);
    let mut total = 0usize;
    let mut pinned_count = 0usize;
    let mut latest_ts: i64 = 0;
    let mut latest_author = String::new();

    for (_key, data, ts_str) in &raw {
        let parts: Vec<&str> = data.splitn(4, '|').collect();
        if parts.len() < 4 {
            continue;
        }
        total += 1;
        if parts[3] == "pinned" {
            pinned_count += 1;
        }
        let ts = ts_str.parse::<i64>().unwrap_or(0);
        if ts > latest_ts {
            latest_ts = ts;
            latest_author = parts[1].to_string();
        }
    }

    let latest_time = if latest_ts > 0 {
        chrono::DateTime::from_timestamp(latest_ts, 0)
            .map(|d| d.naive_local())
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "未知".to_string())
    } else {
        "无".to_string()
    };

    let bar_len = (total * 10 / MAX_BULLETINS).min(10);
    let bar = format!("{}{}", "█".repeat(bar_len), "░".repeat(10 - bar_len));

    format!(
        "{}\n📊 【{}公告统计】\n━━━━━━━━━━━━━━━━━━━━\n\
         📜 总公告数: {}/{}\n\
         📌 置顶公告: {}\n\
         📈 容量: [{}] {}%\n\
         🕐 最新公告: {}\n\
         ✏️ 最新发布者: {}\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         💡 使用「查看公会公告」查看全部公告",
        prefix,
        guild,
        total,
        MAX_BULLETINS,
        pinned_count,
        bar,
        total * 100 / MAX_BULLETINS.max(1),
        latest_time,
        if latest_author.is_empty() {
            "无".to_string()
        } else {
            latest_author
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_bulletins_constant() {
        assert_eq!(MAX_BULLETINS, 20);
    }

    #[test]
    fn test_max_content_len() {
        assert_eq!(MAX_CONTENT_LEN, 200);
    }

    #[test]
    fn test_section_name() {
        assert_eq!(SECTION, "guild_bulletin");
    }

    #[test]
    fn test_gen_bulletin_id_format() {
        let id = gen_bulletin_id("test_user");
        let parts: Vec<&str> = id.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].parse::<i64>().is_ok());
        assert_eq!(parts[1].len(), 8);
        assert!(u32::from_str_radix(parts[1], 16).is_ok());
    }

    #[test]
    fn test_gen_bulletin_id_different_users() {
        let id1 = gen_bulletin_id("user_001");
        let id2 = gen_bulletin_id("user_002");
        let h1 = id1.split('_').nth(1).unwrap();
        let h2 = id2.split('_').nth(1).unwrap();
        assert_eq!(h1.len(), 8);
        assert_eq!(h2.len(), 8);
        assert!(u32::from_str_radix(h1, 16).is_ok());
        assert!(u32::from_str_radix(h2, 16).is_ok());
    }

    #[test]
    fn test_gen_bulletin_id_chinese_chars() {
        let id = gen_bulletin_id("中文用户");
        assert!(id.contains('_'));
        let parts: Vec<&str> = id.split('_').collect();
        assert_eq!(parts.len(), 2);
        assert!(u32::from_str_radix(parts[1], 16).is_ok());
    }

    #[test]
    fn test_content_boundary_at_limit() {
        let content = "a".repeat(MAX_CONTENT_LEN);
        assert_eq!(content.len(), MAX_CONTENT_LEN);
    }

    #[test]
    fn test_content_boundary_over_limit() {
        let over = "a".repeat(MAX_CONTENT_LEN + 1);
        assert!(over.len() > MAX_CONTENT_LEN);
    }

    #[test]
    fn test_gen_bulletin_id_empty_user() {
        let id = gen_bulletin_id("");
        assert!(id.contains('_'));
        let h = id.split('_').nth(1).unwrap();
        // djb2 seed = 5381 → 0x1505
        assert_eq!(h, "00001505");
    }
}

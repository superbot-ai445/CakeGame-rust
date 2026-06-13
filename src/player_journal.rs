/// CakeGame 玩家日志系统
///
/// 追踪玩家重要游戏事件，提供可浏览的活动时间线
///
/// 日志类别:
/// - ⚔️ 战斗: 击杀怪物/击败玩家/阵亡
/// - 📈 成长: 升级/转职/突破
/// - 💰 经济: 购买/出售/交易/强化
/// - 🏰 社交: 加入公会/组队/好友
/// - 🎯 活动: 完成任务/签到/采集
/// - 🎁 获得: 获得物品/获得金币/获得钻石
///
/// 指令: 查看日志 / 日志统计 / 日志搜索 / 记录日志
use crate::db::Database;
use crate::user;

/// 日志条目
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub timestamp: i64,
    pub category: JournalCategory,
    pub message: String,
}

/// 日志类别
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JournalCategory {
    /// 战斗事件
    Combat,
    /// 成长事件
    Growth,
    /// 经济事件
    Economy,
    /// 社交事件
    Social,
    /// 活动事件
    Activity,
    /// 获得事件
    Reward,
}

impl JournalCategory {
    pub fn emoji(&self) -> &str {
        match self {
            Self::Combat => "⚔️",
            Self::Growth => "📈",
            Self::Economy => "💰",
            Self::Social => "🏰",
            Self::Activity => "🎯",
            Self::Reward => "🎁",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Combat => "战斗",
            Self::Growth => "成长",
            Self::Economy => "经济",
            Self::Social => "社交",
            Self::Activity => "活动",
            Self::Reward => "获得",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "战斗" | "combat" | "⚔️" => Some(Self::Combat),
            "成长" | "growth" | "📈" => Some(Self::Growth),
            "经济" | "economy" | "💰" => Some(Self::Economy),
            "社交" | "social" | "🏰" => Some(Self::Social),
            "活动" | "activity" | "🎯" => Some(Self::Activity),
            "获得" | "reward" | "🎁" => Some(Self::Reward),
            _ => None,
        }
    }
}

/// 最大日志条目数 (每用户)
pub const MAX_ENTRIES: usize = 200;

/// 日志存储 Global section 前缀
fn journal_section(uid: &str) -> String {
    format!("player_journal_{}", uid)
}

/// 获取当前时间戳 (秒)
fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 从 Global 表读取用户的日志条目
fn read_entries(db: &Database, uid: &str) -> Vec<JournalEntry> {
    let section = journal_section(uid);
    let data = db.global_get(&section, "entries");
    if data.is_empty() {
        return Vec::new();
    }
    parse_entries(&data)
}

/// 写入日志条目到 Global 表
fn write_entries(db: &Database, uid: &str, entries: &[JournalEntry]) {
    let section = journal_section(uid);
    let serialized = serialize_entries(entries);
    db.global_set(&section, "entries", &serialized);
}

/// 序列化日志条目: timestamp|category|message\n...
pub fn serialize_entries(entries: &[JournalEntry]) -> String {
    entries
        .iter()
        .map(|e| {
            format!(
                "{}|{}|{}",
                e.timestamp,
                category_to_str(e.category),
                e.message.replace('\n', " ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 解析日志条目
pub fn parse_entries(data: &str) -> Vec<JournalEntry> {
    data.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() < 3 {
                return None;
            }
            let timestamp = parts[0].parse::<i64>().ok()?;
            let category = str_to_category(parts[1])?;
            Some(JournalEntry {
                timestamp,
                category,
                message: parts[2].to_string(),
            })
        })
        .collect()
}

fn category_to_str(cat: JournalCategory) -> &'static str {
    match cat {
        JournalCategory::Combat => "combat",
        JournalCategory::Growth => "growth",
        JournalCategory::Economy => "economy",
        JournalCategory::Social => "social",
        JournalCategory::Activity => "activity",
        JournalCategory::Reward => "reward",
    }
}

fn str_to_category(s: &str) -> Option<JournalCategory> {
    match s {
        "combat" => Some(JournalCategory::Combat),
        "growth" => Some(JournalCategory::Growth),
        "economy" => Some(JournalCategory::Economy),
        "social" => Some(JournalCategory::Social),
        "activity" => Some(JournalCategory::Activity),
        "reward" => Some(JournalCategory::Reward),
        _ => None,
    }
}

/// 格式化时间戳为可读格式
fn format_timestamp(ts: i64) -> String {
    let now = now_ts();
    let diff = now - ts;
    if diff < 60 {
        "刚刚".to_string()
    } else if diff < 3600 {
        format!("{}分钟前", diff / 60)
    } else if diff < 86400 {
        format!("{}小时前", diff / 3600)
    } else if diff < 604800 {
        format!("{}天前", diff / 86400)
    } else {
        let days = ts / 86400;
        let y = 1970 + days / 365;
        let d = days % 365;
        format!("{}年第{}天", y, d)
    }
}

/// 添加日志条目 (公共 API，供其他模块调用)
pub fn add_journal_entry(db: &Database, uid: &str, category: JournalCategory, message: &str) {
    let mut entries = read_entries(db, uid);
    entries.push(JournalEntry {
        timestamp: now_ts(),
        category,
        message: message.to_string(),
    });
    // 保留最新 MAX_ENTRIES 条
    if entries.len() > MAX_ENTRIES {
        let drain_count = entries.len() - MAX_ENTRIES;
        entries.drain(..drain_count);
    }
    write_entries(db, uid, &entries);
}

/// cmd_view_journal: 查看日志 — 浏览最近的活动日志
pub fn cmd_view_journal(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let entries = read_entries(db, user_id);
    if entries.is_empty() {
        return format!(
            "{}\n📜 === 玩家日志 === 📜\n━━━━━━━━━━━━━━━━━━━━━━━━\n\n暂无日志记录\n参与游戏活动将自动记录日志！",
            prefix
        );
    }

    // 可选按类别筛选
    let filter_cat = if !args.trim().is_empty() {
        JournalCategory::from_str(args.trim())
    } else {
        None
    };

    let filtered: Vec<&JournalEntry> = if let Some(cat) = filter_cat {
        entries.iter().filter(|e| e.category == cat).collect()
    } else {
        entries.iter().collect()
    };

    let page_size = 15;
    let total_pages = filtered.len().div_ceil(page_size);
    let page = 1; // 显示最新一页 (第一页)

    let start = filtered.len().saturating_sub(page_size);
    let show: Vec<&&JournalEntry> = filtered[start..].iter().rev().collect();

    let mut out = format!("{}\n📜 === 玩家日志 === 📜\n", prefix);
    if let Some(cat) = filter_cat {
        out.push_str(&format!("筛选: {} {}\n", cat.emoji(), cat.name()));
    }
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━\n\n");

    for entry in &show {
        let time_str = format_timestamp(entry.timestamp);
        out.push_str(&format!(
            "  {} {} [{}] {}\n",
            entry.category.emoji(),
            entry.category.name(),
            time_str,
            entry.message,
        ));
    }

    out.push_str(&format!(
        "\n📊 共{}条日志 | 显示最新{}条 | 第{}/{}页\n",
        filtered.len(),
        show.len(),
        page,
        total_pages.max(1)
    ));
    out.push_str("💡 输入「查看日志+类别」按类别筛选 (战斗/成长/经济/社交/活动/获得)\n");

    out
}

/// cmd_journal_stats: 日志统计 — 显示各类型日志数量分布
pub fn cmd_journal_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let entries = read_entries(db, user_id);
    if entries.is_empty() {
        return format!("{}\n📊 暂无日志统计。", prefix);
    }

    let mut counts = [0usize; 6]; // combat, growth, economy, social, activity, reward
    for e in &entries {
        let idx = match e.category {
            JournalCategory::Combat => 0,
            JournalCategory::Growth => 1,
            JournalCategory::Economy => 2,
            JournalCategory::Social => 3,
            JournalCategory::Activity => 4,
            JournalCategory::Reward => 5,
        };
        counts[idx] += 1;
    }

    let categories = [
        (JournalCategory::Combat, counts[0]),
        (JournalCategory::Growth, counts[1]),
        (JournalCategory::Economy, counts[2]),
        (JournalCategory::Social, counts[3]),
        (JournalCategory::Activity, counts[4]),
        (JournalCategory::Reward, counts[5]),
    ];

    let total = entries.len();

    let mut out = format!("{}\n📊 === 日志统计 === 📊\n━━━━━━━━━━━━━━━━━━━━━━━━\n\n", prefix);
    out.push_str(&format!("  📝 总日志: {} 条\n\n", total));

    for (cat, count) in &categories {
        if *count > 0 {
            let pct = *count as f64 * 100.0 / total as f64;
            let bar_len = (pct / 5.0).min(20.0) as usize;
            let bar = "█".repeat(bar_len);
            out.push_str(&format!(
                "  {} {}: {} 条 ({:.1}%) {}\n",
                cat.emoji(),
                cat.name(),
                count,
                pct,
                bar
            ));
        }
    }

    // 最近活动时间
    let latest = entries.iter().max_by_key(|e| e.timestamp);
    if let Some(latest) = latest {
        out.push_str(&format!(
            "\n  🕐 最近活动: {} ({})\n",
            latest.message,
            format_timestamp(latest.timestamp)
        ));
    }

    out.push_str("\n💡 输入「查看日志+类别」按类别查看\n");

    out
}

/// cmd_journal_search: 日志搜索 — 按关键词搜索日志
pub fn cmd_journal_search(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let keyword = args.trim();
    if keyword.is_empty() {
        return format!("{}\n🔍 请输入搜索关键词。\n💡 用法: 日志搜索+关键词", prefix);
    }

    let entries = read_entries(db, user_id);
    let matches: Vec<&JournalEntry> = entries.iter().filter(|e| e.message.contains(keyword)).collect();

    let mut out = format!(
        "{}\n🔍 === 日志搜索: {} === 🔍\n━━━━━━━━━━━━━━━━━━━━━━━━\n\n",
        prefix, keyword
    );

    if matches.is_empty() {
        out.push_str(&format!("未找到包含「{}」的日志。\n", keyword));
    } else {
        let show_count = matches.len().min(15);
        for entry in matches.iter().rev().take(show_count) {
            let time_str = format_timestamp(entry.timestamp);
            out.push_str(&format!(
                "  {} {} [{}] {}\n",
                entry.category.emoji(),
                entry.category.name(),
                time_str,
                entry.message,
            ));
        }
        out.push_str(&format!("\n📊 找到 {} 条匹配日志\n", matches.len()));
    }

    out
}

/// cmd_add_journal: 记录日志 (GM/手动添加)
pub fn cmd_add_journal(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let parts: Vec<&str> = args.splitn(2, '+').collect();
    if parts.len() < 2 {
        return format!(
            "{}\n📝 用法: 记录日志+类别+内容\n类别: 战斗/成长/经济/社交/活动/获得",
            prefix
        );
    }

    let cat_str = parts[0].trim();
    let message = parts[1].trim();

    let category = match JournalCategory::from_str(cat_str) {
        Some(c) => c,
        None => {
            return format!(
                "{}\n❌ 无效类别「{}」\n有效类别: 战斗/成长/经济/社交/活动/获得",
                prefix, cat_str
            );
        }
    };

    if message.is_empty() {
        return format!("{}\n❌ 日志内容不能为空。", prefix);
    }

    if message.len() > 100 {
        return format!("{}\n❌ 日志内容过长（最多100字）。", prefix);
    }

    add_journal_entry(db, user_id, category, message);

    format!("{}\n✅ 日志已记录: {} {}", prefix, category.emoji(), message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_emoji_coverage() {
        let cats = [
            JournalCategory::Combat,
            JournalCategory::Growth,
            JournalCategory::Economy,
            JournalCategory::Social,
            JournalCategory::Activity,
            JournalCategory::Reward,
        ];
        for cat in &cats {
            assert!(!cat.emoji().is_empty());
            assert!(!cat.name().is_empty());
        }
    }

    #[test]
    fn test_category_from_str() {
        assert!(JournalCategory::from_str("战斗").is_some());
        assert!(JournalCategory::from_str("combat").is_some());
        assert!(JournalCategory::from_str("⚔️").is_some());
        assert!(JournalCategory::from_str("成长").is_some());
        assert!(JournalCategory::from_str("经济").is_some());
        assert!(JournalCategory::from_str("社交").is_some());
        assert!(JournalCategory::from_str("活动").is_some());
        assert!(JournalCategory::from_str("获得").is_some());
        assert!(JournalCategory::from_str("invalid").is_none());
    }

    #[test]
    fn test_serialize_parse_roundtrip() {
        let entries = vec![
            JournalEntry {
                timestamp: 1700000000,
                category: JournalCategory::Combat,
                message: "击杀了哥布林".to_string(),
            },
            JournalEntry {
                timestamp: 1700000100,
                category: JournalCategory::Growth,
                message: "升到5级".to_string(),
            },
        ];
        let serialized = serialize_entries(&entries);
        let parsed = parse_entries(&serialized);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].timestamp, 1700000000);
        assert_eq!(parsed[0].message, "击杀了哥布林");
        assert_eq!(parsed[1].category, JournalCategory::Growth);
    }

    #[test]
    fn test_parse_empty() {
        let entries = parse_entries("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_invalid_line() {
        let entries = parse_entries("invalid\nalso|bad\n");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_max_entries_constant() {
        assert_eq!(MAX_ENTRIES, 200);
    }

    #[test]
    fn test_format_timestamp_recent() {
        let now = now_ts();
        let result = format_timestamp(now);
        assert_eq!(result, "刚刚");
    }

    #[test]
    fn test_format_timestamp_minutes() {
        let now = now_ts();
        let result = format_timestamp(now - 300);
        assert!(result.contains("分钟前"));
    }

    #[test]
    fn test_serialize_format() {
        let entries = vec![JournalEntry {
            timestamp: 1000,
            category: JournalCategory::Economy,
            message: "购买了药水".to_string(),
        }];
        let s = serialize_entries(&entries);
        assert_eq!(s, "1000|economy|购买了药水");
    }

    #[test]
    fn test_category_roundtrip() {
        let cats = ["combat", "growth", "economy", "social", "activity", "reward"];
        for cat_str in &cats {
            let cat = str_to_category(cat_str).unwrap();
            assert_eq!(category_to_str(cat), *cat_str);
        }
    }
}

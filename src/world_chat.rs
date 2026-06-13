/// CakeGame 世界聊天频道系统
///
/// 全服玩家实时沟通的聊天系统：
/// - 3种频道: 世界频道🌍 / 交易频道💰 / 组队频道👥
/// - 聊天记录持久化存储 (Global 表)
/// - 消息冷却防刷屏 (世界5秒 / 交易10秒 / 组队5秒)
/// - VIP玩家消息高亮标识
/// - 聊天历史查看 (最近50条)
///
/// 指令: 聊天/查看聊天/切换频道/聊天历史/聊天帮助
/// 数据存储: Global SECTION='WorldChat' + PlayerData 'ChatChannel'
use crate::core::{CURRENCY_GOLD, ITEM_LEVEL, ITEM_NAME, OP_SUB};
use crate::db::Database;

const SECTION: &str = "WorldChat";

/// 频道类型定义
struct Channel {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    cooldown_secs: i64,
    min_level: i32,
    cost_gold: i64,
    desc: &'static str,
}

const CHANNELS: &[Channel] = &[
    Channel {
        id: "world",
        name: "世界频道",
        emoji: "🌍",
        cooldown_secs: 5,
        min_level: 5,
        cost_gold: 0,
        desc: "全服玩家可见，适合交流聊天",
    },
    Channel {
        id: "trade",
        name: "交易频道",
        emoji: "💰",
        cooldown_secs: 10,
        min_level: 10,
        cost_gold: 100,
        desc: "买卖交易专用，发布求购/出售信息",
    },
    Channel {
        id: "team",
        name: "组队频道",
        emoji: "👥",
        cooldown_secs: 5,
        min_level: 3,
        cost_gold: 0,
        desc: "寻找队友组队打怪/副本",
    },
];

/// 获取玩家当前频道
fn get_player_channel(db: &Database, user_id: &str) -> String {
    let ch = db.read_user_data(user_id, "ChatChannel");
    if ch.is_empty() {
        "world".to_string()
    } else {
        ch
    }
}

/// 设置玩家当前频道
fn set_player_channel(db: &Database, user_id: &str, channel: &str) {
    let _ = db.write_user_data(user_id, "ChatChannel", channel);
}

/// 获取当前时间戳（秒）
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 格式化时间戳为可读格式
fn format_timestamp(ts: i64) -> String {
    let now = now_secs();
    let diff = now - ts;
    if diff < 60 {
        "刚刚".to_string()
    } else if diff < 3600 {
        format!("{}分钟前", diff / 60)
    } else if diff < 86400 {
        format!("{}小时前", diff / 3600)
    } else {
        format!("{}天前", diff / 86400)
    }
}

/// 格式化VIP标识
fn vip_badge(vip_level: i32) -> &'static str {
    match vip_level {
        0 => "",
        1 => "⭐",
        2 => "🌟",
        3 => "💎",
        4 => "👑",
        _ => "🔱",
    }
}

/// 存储聊天消息到 Global 表
fn store_message(
    db: &Database,
    channel: &str,
    sender_id: &str,
    sender_name: &str,
    content: &str,
    vip_level: i32,
) -> i64 {
    let counter_raw = db.global_get(SECTION, "counter");
    let msg_id: i64 = counter_raw.parse().unwrap_or(0) + 1;
    db.global_set(SECTION, "counter", &msg_id.to_string());

    let ts = now_secs();
    let value = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        msg_id, channel, sender_id, sender_name, content, ts, vip_level
    );
    let key = format!("msg_{}", msg_id);
    db.global_set(SECTION, &key, &value);

    // 清理旧消息（保留最近200条）
    if msg_id > 200 {
        let old_key = format!("msg_{}", msg_id - 200);
        db.global_set(SECTION, &old_key, "");
    }

    msg_id
}

/// 读取最近N条聊天消息（通过 SQL 查询 Global 表）
fn get_recent_messages(
    db: &Database,
    channel_filter: Option<&str>,
    limit: usize,
) -> Vec<(i64, String, String, String, String, i64, i32)> {
    let conn = db.lock_conn();
    let query = "SELECT ID, Data FROM Global WHERE SECTION = ?1 AND ID LIKE 'msg_%' AND Data != '' ORDER BY CAST(REPLACE(ID, 'msg_', '') AS INTEGER) DESC LIMIT ?2";

    let mut messages = Vec::new();
    let result = conn.prepare(query).ok().and_then(|mut stmt| {
        let rows = stmt.query_map(rusqlite::params![SECTION, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        rows.ok().map(|r| r.collect::<Vec<_>>())
    });

    if let Some(rows) = result {
        for row in rows.into_iter().flatten() {
            let (_key, value) = row;
            let parts: Vec<&str> = value.split('|').collect();
            if parts.len() < 7 {
                continue;
            }
            let msg_channel = parts[1].to_string();
            if let Some(filter) = channel_filter {
                if msg_channel != filter {
                    continue;
                }
            }
            let msg_id = parts[0].parse().unwrap_or(0);
            let sender_id = parts[2].to_string();
            let sender_name = parts[3].to_string();
            let content = parts[4].to_string();
            let ts = parts[5].parse().unwrap_or(0);
            let vip = parts[6].parse().unwrap_or(0);
            messages.push((msg_id, msg_channel, sender_id, sender_name, content, ts, vip));
        }
    }

    messages
}

// ============================================================
// 公开指令
// ============================================================

/// 聊天/发送消息 - 向当前频道发送聊天消息
pub fn cmd_chat_send(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let content = args.trim();

    if content.is_empty() {
        return "❌ 请输入聊天内容！\n📌 发送「聊天+内容」发送消息".to_string();
    }

    if content.len() > 200 {
        return format!("❌ 消息过长！最多200字（当前{}字）", content.len());
    }

    let nickname = db.read_basic(user_id, ITEM_NAME);
    if nickname.is_empty() {
        return "❌ 请先注册账号！".to_string();
    }

    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let vip_level = crate::vip::get_vip_level(db, user_id);

    let channel_id = get_player_channel(db, user_id);
    let ch = match CHANNELS.iter().find(|c| c.id == channel_id) {
        Some(c) => c,
        None => &CHANNELS[0],
    };

    if level < ch.min_level {
        return format!("❌ {}需要等级{}级，当前等级{}级", ch.name, ch.min_level, level);
    }

    let last_ts: i64 = db
        .read_user_data(user_id, &format!("ChatCD_{}", ch.id))
        .parse()
        .unwrap_or(0);
    let now = now_secs();
    let elapsed = now - last_ts;
    if elapsed < ch.cooldown_secs {
        let remaining = ch.cooldown_secs - elapsed;
        return format!(
            "⏳ 聊天冷却中... 还需等待{}秒\n💡 {}冷却时间: {}秒",
            remaining, ch.name, ch.cooldown_secs
        );
    }

    if ch.cost_gold > 0 {
        let gold = db.read_currency(user_id, CURRENCY_GOLD);
        if gold < ch.cost_gold {
            return format!(
                "❌ {}发言需要{}金币，余额不足！（当前: {}金币）",
                ch.name, ch.cost_gold, gold
            );
        }
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, ch.cost_gold);
    }

    let msg_id = store_message(db, ch.id, user_id, &nickname, content, vip_level);
    let _ = db.write_user_data(user_id, &format!("ChatCD_{}", ch.id), &now.to_string());

    let mut out = format!("{} 📤 消息已发送！\n", ch.emoji);
    out += &format!("📢 频道: {} [{}]\n", ch.name, ch.id);
    out += &format!("💬 内容: {}\n", content);
    out += &format!("📝 消息ID: #{}\n", msg_id);
    if ch.cost_gold > 0 {
        out += &format!("💰 消耗: {}金币\n", ch.cost_gold);
    }
    out += &format!("⏳ 冷却: {}秒\n", ch.cooldown_secs);
    out
}

/// 查看聊天 - 查看当前频道最近消息
pub fn cmd_chat_view(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let channel_id = if args.trim().is_empty() {
        get_player_channel(db, user_id)
    } else {
        let input = args.trim();
        match CHANNELS.iter().find(|c| c.id == input || c.name.contains(input)) {
            Some(c) => c.id.to_string(),
            None => input.to_string(),
        }
    };

    let ch = CHANNELS.iter().find(|c| c.id == channel_id);
    let (ch_name, ch_emoji) = match ch {
        Some(c) => (c.name, c.emoji),
        None => return format!("❌ 未知频道: {}\n可用频道: 世界/交易/组队", channel_id),
    };

    let messages = get_recent_messages(db, Some(&channel_id), 20);

    let mut out = format!("{} ═══ {} ═══\n", ch_emoji, ch_name);

    if messages.is_empty() {
        out += "📭 暂无消息，发送第一条消息吧！\n";
    } else {
        out += &format!("📜 最近{}条消息:\n\n", messages.len());
        for msg in messages.iter().rev() {
            let vip = vip_badge(msg.6);
            let time_str = format_timestamp(msg.5);
            out += &format!("[{}] {}{}: {}\n", time_str, msg.3, vip, msg.4);
        }
    }

    out += "\n📌 发送「聊天+内容」发送消息\n";
    out += "💡 发送「切换频道+频道名」切换频道\n";
    out
}

/// 切换频道 - 切换聊天频道
pub fn cmd_chat_switch(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let target = args.trim();
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);

    if target.is_empty() {
        let current = get_player_channel(db, user_id);
        let mut out = "📢 ═══ 聊天频道列表 ═══\n\n".to_string();

        for ch in CHANNELS {
            let marker = if ch.id == current { " ✅当前" } else { "" };
            let lock = if level < ch.min_level { " 🔒" } else { "" };
            out += &format!(
                "{} {} [{}]{}\n   {} 冷却:{}秒",
                ch.emoji, ch.name, ch.id, marker, ch.desc, ch.cooldown_secs
            );
            if ch.cost_gold > 0 {
                out += &format!(" 每次{}金", ch.cost_gold);
            }
            if level < ch.min_level {
                out += &format!(" 需要{}级", ch.min_level);
            }
            out += &format!("{}\n\n", lock);
        }

        out += "📌 发送「切换频道+频道名」切换\n";
        out += "💡 频道名: 世界/交易/组队\n";
        return out;
    }

    let ch = CHANNELS
        .iter()
        .find(|c| c.id == target || c.name.contains(target) || c.id.starts_with(target));

    match ch {
        Some(ch) => {
            if level < ch.min_level {
                return format!("❌ {}需要等级{}级，当前等级{}级", ch.name, ch.min_level, level);
            }
            set_player_channel(db, user_id, ch.id);
            format!(
                "{} 📢 已切换到 {} [{}]\n{}\n💡 现在可以发送消息了",
                ch.emoji, ch.name, ch.id, ch.desc
            )
        }
        None => format!("❌ 未找到频道 [{}]\n可用频道: 世界/交易/组队", target),
    }
}

/// 聊天历史 - 查看所有频道的聊天历史
pub fn cmd_chat_history(db: &Database, _user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let limit: usize = args.trim().parse().unwrap_or(30).min(50);

    let messages = get_recent_messages(db, None, limit);

    let mut out = "📜 ═══ 聊天历史 ═══\n\n".to_string();

    if messages.is_empty() {
        out += "📭 暂无聊天记录\n";
        return out;
    }

    let mut current_channel = String::new();
    for msg in messages.iter().rev() {
        if msg.1 != current_channel {
            current_channel = msg.1.clone();
            let ch = CHANNELS.iter().find(|c| c.id == current_channel);
            let (name, emoji) = match ch {
                Some(c) => (c.name, c.emoji),
                None => ("未知", "❓"),
            };
            out += &format!("━━ {} {} ━━\n", emoji, name);
        }
        let vip = vip_badge(msg.6);
        let time_str = format_timestamp(msg.5);
        out += &format!("[{}] {}{}: {}\n", time_str, msg.3, vip, msg.4);
    }

    out += &format!("\n📊 显示最近{}条消息\n", messages.len());
    out += "💡 发送「历史+数量」查看更多（最多50条）\n";
    out
}

/// 聊天帮助 - 显示聊天系统帮助
pub fn cmd_chat_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let current_channel = get_player_channel(db, user_id);
    let ch = CHANNELS
        .iter()
        .find(|c| c.id == current_channel)
        .unwrap_or(&CHANNELS[0]);

    let mut out = "💬 ═══ 聊天系统帮助 ═══\n\n".to_string();
    out += &format!("📢 当前频道: {} {}\n\n", ch.name, ch.emoji);

    out += "📋 聊天指令:\n";
    out += "  📤 聊天+内容 - 发送消息到当前频道\n";
    out += "  👀 查看聊天 - 查看当前频道消息\n";
    out += "  🔄 切换频道 - 切换/查看频道列表\n";
    out += "  📜 聊天历史+数量 - 查看聊天历史\n";
    out += "  ❓ 聊天帮助 - 显示此帮助\n\n";

    out += "📢 频道详情:\n";
    for ch in CHANNELS {
        out += &format!(
            "  {} {}: 等级{}+ | 冷却{}秒",
            ch.emoji, ch.name, ch.min_level, ch.cooldown_secs
        );
        if ch.cost_gold > 0 {
            out += &format!(" | 费用{}金", ch.cost_gold);
        }
        out += &format!("\n     {}\n", ch.desc);
    }

    out += "\n💡 聊天小贴士:\n";
    out += "  • VIP玩家消息带特殊标识\n";
    out += "  • 交易频道发言需要金币\n";
    out += "  • 消息最多200字\n";
    out += "  • 每个频道有独立冷却时间\n";
    out
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channels_count() {
        assert_eq!(CHANNELS.len(), 3, "Should have 3 channels");
    }

    #[test]
    fn test_channel_ids_unique() {
        let mut ids: Vec<&str> = CHANNELS.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CHANNELS.len(), "Channel IDs must be unique");
    }

    #[test]
    fn test_channel_names_unique() {
        let mut names: Vec<&str> = CHANNELS.iter().map(|c| c.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), CHANNELS.len(), "Channel names must be unique");
    }

    #[test]
    fn test_channel_cooldowns_positive() {
        for ch in CHANNELS {
            assert!(ch.cooldown_secs > 0, "{} cooldown must be positive", ch.name);
        }
    }

    #[test]
    fn test_channel_min_levels() {
        for ch in CHANNELS {
            assert!(ch.min_level >= 1, "{} min_level must be >= 1", ch.name);
            assert!(ch.min_level <= 100, "{} min_level must be <= 100", ch.name);
        }
    }

    #[test]
    fn test_channel_emojis_non_empty() {
        for ch in CHANNELS {
            assert!(!ch.emoji.is_empty(), "{} must have emoji", ch.name);
        }
    }

    #[test]
    fn test_channel_descs_non_empty() {
        for ch in CHANNELS {
            assert!(!ch.desc.is_empty(), "{} must have description", ch.name);
        }
    }

    #[test]
    fn test_channel_cost_non_negative() {
        for ch in CHANNELS {
            assert!(ch.cost_gold >= 0, "{} cost must be non-negative", ch.name);
        }
    }

    #[test]
    fn test_trade_channel_costliest() {
        let trade = CHANNELS.iter().find(|c| c.id == "trade").unwrap();
        for ch in CHANNELS {
            assert!(trade.cost_gold >= ch.cost_gold, "Trade channel should be costliest");
        }
    }

    #[test]
    fn test_world_channel_cooldown_le_trade() {
        let world = CHANNELS.iter().find(|c| c.id == "world").unwrap();
        let trade = CHANNELS.iter().find(|c| c.id == "trade").unwrap();
        assert!(world.cooldown_secs <= trade.cooldown_secs);
    }

    #[test]
    fn test_vip_badge_levels() {
        assert_eq!(vip_badge(0), "");
        assert_eq!(vip_badge(1), "⭐");
        assert_eq!(vip_badge(2), "🌟");
        assert_eq!(vip_badge(3), "💎");
        assert_eq!(vip_badge(4), "👑");
        assert_eq!(vip_badge(5), "🔱");
        assert_eq!(vip_badge(99), "🔱");
    }

    #[test]
    fn test_format_timestamp_just_now() {
        let ts = now_secs();
        assert_eq!(format_timestamp(ts), "刚刚");
    }

    #[test]
    fn test_format_timestamp_minutes_ago() {
        let ts = now_secs() - 300;
        assert_eq!(format_timestamp(ts), "5分钟前");
    }

    #[test]
    fn test_format_timestamp_hours_ago() {
        let ts = now_secs() - 7200;
        assert_eq!(format_timestamp(ts), "2小时前");
    }

    #[test]
    fn test_format_timestamp_days_ago() {
        let ts = now_secs() - 172800;
        assert_eq!(format_timestamp(ts), "2天前");
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "WorldChat");
    }

    #[test]
    fn test_team_channel_lowest_level() {
        let team = CHANNELS.iter().find(|c| c.id == "team").unwrap();
        let world = CHANNELS.iter().find(|c| c.id == "world").unwrap();
        assert!(team.min_level <= world.min_level);
    }

    #[test]
    fn test_trade_channel_highest_level() {
        let trade = CHANNELS.iter().find(|c| c.id == "trade").unwrap();
        for ch in CHANNELS {
            assert!(trade.min_level >= ch.min_level);
        }
    }
}

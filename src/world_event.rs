/// CakeGame 全服活动系统
/// GM 创建限时全服活动（双倍经验、双倍金币、怪物入侵、掉落加成等）
/// 活动期间所有玩家自动享受加成，无需手动参与
/// 数据存储：Global 表 world_event 节点
/// GM权限阈值
const PERMISSION_LEVEL_ADMIN: i32 = 100;
use crate::db::Database;
use crate::user;

/// 活动类型
#[derive(Debug, Clone, PartialEq)]
pub enum WorldEventType {
    /// 双倍经验
    DoubleExp,
    /// 双倍金币
    DoubleGold,
    /// 怪物入侵（额外掉落）
    MonsterInvasion,
    /// 全服福利（定时发奖）
    ServerBlessing,
    /// 掉落加成
    DropBoost,
}

impl WorldEventType {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "双倍经验" | "double_exp" => Some(Self::DoubleExp),
            "双倍金币" | "double_gold" => Some(Self::DoubleGold),
            "怪物入侵" | "monster_invasion" => Some(Self::MonsterInvasion),
            "全服福利" | "server_blessing" => Some(Self::ServerBlessing),
            "掉落加成" | "drop_boost" => Some(Self::DropBoost),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DoubleExp => "双倍经验",
            Self::DoubleGold => "双倍金币",
            Self::MonsterInvasion => "怪物入侵",
            Self::ServerBlessing => "全服福利",
            Self::DropBoost => "掉落加成",
        }
    }

    fn emoji(&self) -> &'static str {
        match self {
            Self::DoubleExp => "📈",
            Self::DoubleGold => "💰",
            Self::MonsterInvasion => "👹",
            Self::ServerBlessing => "🎁",
            Self::DropBoost => "🍀",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::DoubleExp => "战斗经验翻倍",
            Self::DoubleGold => "战斗金币翻倍",
            Self::MonsterInvasion => "野外怪物额外掉落稀有物品",
            Self::ServerBlessing => "在线玩家每10分钟获得奖励",
            Self::DropBoost => "所有掉落概率提升50%",
        }
    }
}

/// 全服活动数据
#[derive(Debug, Clone)]
pub struct WorldEvent {
    pub name: String,
    pub event_type: WorldEventType,
    pub start_time: String,
    pub end_time: String,
    pub multiplier: f64,
    pub description: String,
    pub created_by: String,
}

impl WorldEvent {
    /// 序列化为字符串存储
    fn to_storage(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}",
            self.name,
            self.event_type.as_str(),
            self.start_time,
            self.end_time,
            self.multiplier,
            self.description.replace('|', "/"),
            self.created_by
        )
    }

    /// 从存储字符串反序列化
    fn from_storage(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(7, '|').collect();
        if parts.len() < 7 {
            return None;
        }
        let event_type = WorldEventType::from_str(parts[1])?;
        Some(Self {
            name: parts[0].to_string(),
            event_type,
            start_time: parts[2].to_string(),
            end_time: parts[3].to_string(),
            multiplier: parts[4].parse().unwrap_or(1.0),
            description: parts[5].to_string(),
            created_by: parts[6].to_string(),
        })
    }

    /// 检查活动是否仍在进行中
    pub fn is_active(&self) -> bool {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        now >= self.start_time && now <= self.end_time
    }

    /// 剩余时间描述
    fn remaining(&self) -> String {
        let now = chrono::Local::now().naive_local();
        if let Ok(end) = chrono::NaiveDateTime::parse_from_str(&self.end_time, "%Y-%m-%d %H:%M:%S") {
            let diff = end.signed_duration_since(now);
            if diff.num_seconds() <= 0 {
                return "已结束".to_string();
            }
            let hours = diff.num_hours();
            let mins = diff.num_minutes() % 60;
            if hours > 0 {
                format!("{}小时{}分钟", hours, mins)
            } else {
                format!("{}分钟", mins)
            }
        } else {
            "未知".to_string()
        }
    }
}

/// 获取所有活动（从 Global 表读取）
fn get_all_events(db: &Database) -> Vec<WorldEvent> {
    let raw = db.global_get("world_event", "active_events");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('\n').filter_map(WorldEvent::from_storage).collect()
}

/// 保存所有活动到 Global 表
fn save_all_events(db: &Database, events: &[WorldEvent]) {
    let data: Vec<String> = events.iter().map(|e| e.to_storage()).collect();
    db.global_set("world_event", "active_events", &data.join("\n"));
}

/// 清理已过期的活动
fn cleanup_expired(db: &Database) {
    let events = get_all_events(db);
    let active: Vec<WorldEvent> = events.into_iter().filter(|e| e.is_active()).collect();
    save_all_events(db, &active);
}

/// 查看当前全服活动
pub fn cmd_view_world_events(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    cleanup_expired(db);

    let events = get_all_events(db);
    let active: Vec<&WorldEvent> = events.iter().filter(|e| e.is_active()).collect();

    let mut out = format!("{}\n═══ 🌍 全服活动 ═══", prefix);

    if active.is_empty() {
        out.push_str("\n\n当前没有进行中的全服活动。");
        out.push_str("\n\n💡 全服活动由管理员创建，活动期间所有玩家自动享受加成。");
        out.push_str("\n可选活动类型：");
        out.push_str("\n  📈 双倍经验 — 战斗经验翻倍");
        out.push_str("\n  💰 双倍金币 — 战斗金币翻倍");
        out.push_str("\n  👹 怪物入侵 — 额外掉落稀有物品");
        out.push_str("\n  🎁 全服福利 — 在线玩家定时奖励");
        out.push_str("\n  🍀 掉落加成 — 掉落概率提升50%");
    } else {
        out.push_str(&format!("\n\n当前正在进行 {} 个活动：\n", active.len()));
        for (i, evt) in active.iter().enumerate() {
            out.push_str(&format!(
                "\n{}. {} {} [{}]",
                i + 1,
                evt.event_type.emoji(),
                evt.name,
                evt.event_type.as_str()
            ));
            out.push_str(&format!("\n   {}", evt.description));
            out.push_str(&format!("\n   ⏰ 剩余时间：{}", evt.remaining()));
            if evt.multiplier > 1.0 {
                out.push_str(&format!("\n   ✨ 加成倍率：x{:.1}", evt.multiplier));
            }
        }
    }

    out
}

/// GM 创建全服活动
pub fn cmd_create_world_event(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 权限检查：需要管理员权限
    let perm_level: i32 = db.read_user_data(user_id, "permission_level").parse().unwrap_or(0);
    if perm_level < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 无权操作！只有管理员才能创建全服活动。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n📜 创建全服活动\\n\\n格式：创建活动+类型+持续小时数+活动名称\\n\\n示例：\\n  创建活动+双倍经验+24+周末双倍经验\\n  创建活动+双倍金币+12+午间福利\\n  创建活动+怪物入侵+6+怪物大入侵\\n  创建活动+掉落加成+48+幸运周末\\n  创建活动+全服福利+24+节日福利\\n\\n可选类型：双倍经验/双倍金币/怪物入侵/掉落加成/全服福利",
            prefix
        );
    }

    // 解析参数：类型+小时数+名称
    let parts: Vec<&str> = args.splitn(3, '+').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return format!("{}\n❌ 格式错误！正确格式：创建活动+类型+小时数+活动名称", prefix);
    }

    let event_type = match WorldEventType::from_str(parts[0]) {
        Some(t) => t,
        None => {
            return format!(
                "{}\n❌ 未知活动类型「{}」！\\n可选：双倍经验/双倍金币/怪物入侵/掉落加成/全服福利",
                prefix, parts[0]
            );
        }
    };

    let hours: i32 = match parts[1].parse() {
        Ok(h) if h > 0 && h <= 168 => h,
        _ => {
            return format!("{}\n❌ 持续时间必须是1-168小时（最多7天）！", prefix);
        }
    };

    let event_name = if parts.len() > 2 && !parts[2].is_empty() {
        parts[2].to_string()
    } else {
        format!("{}活动", event_type.as_str())
    };

    let multiplier = match &event_type {
        WorldEventType::DoubleExp | WorldEventType::DoubleGold => 2.0,
        WorldEventType::DropBoost => 1.5,
        WorldEventType::MonsterInvasion | WorldEventType::ServerBlessing => 1.0,
    };

    let now = chrono::Local::now();
    let start_time = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let end_time = (now + chrono::Duration::hours(hours as i64))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let event = WorldEvent {
        name: event_name,
        event_type: event_type.clone(),
        start_time,
        end_time,
        multiplier,
        description: event_type.description().to_string(),
        created_by: user_id.to_string(),
    };

    let mut events = get_all_events(db);
    events.push(event);
    save_all_events(db, &events);

    format!(
        "{}\n✅ 全服活动创建成功！\\n\\n{} {}\\n⏰ 持续时间：{}小时\\n✨ 加成倍率：x{:.1}\\n📅 结束时间：{}\\n\\n所有玩家将自动享受活动加成！",
        prefix,
        event_type.emoji(),
        events.last().unwrap().name,
        hours,
        multiplier,
        (now + chrono::Duration::hours(hours as i64)).format("%Y-%m-%d %H:%M:%S")
    )
}

/// GM 结束全服活动
pub fn cmd_end_world_event(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let perm_level: i32 = db.read_user_data(user_id, "permission_level").parse().unwrap_or(0);
    if perm_level < PERMISSION_LEVEL_ADMIN {
        return format!("{}\n❌ 无权操作！只有管理员才能结束全服活动。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        let events = get_all_events(db);
        let active: Vec<&WorldEvent> = events.iter().filter(|e| e.is_active()).collect();
        if active.is_empty() {
            return format!("{}\n当前没有进行中的活动。", prefix);
        }
        let mut out = format!("{}\n📜 进行中的活动：", prefix);
        for (i, evt) in active.iter().enumerate() {
            out.push_str(&format!(
                "\n  {}. {} — 发送「结束活动+{}」结束",
                i + 1,
                evt.name,
                evt.name
            ));
        }
        return out;
    }

    let mut events = get_all_events(db);
    let before = events.len();
    events.retain(|e| e.name != args || !e.is_active());

    if events.len() == before {
        return format!("{}\n❌ 未找到名为「{}」的活动。", prefix, args);
    }

    save_all_events(db, &events);
    format!("{}\n✅ 已结束活动「{}」！", prefix, args)
}

/// 获取经验加成倍率（供战斗系统调用）
pub fn get_exp_multiplier(db: &Database) -> f64 {
    let events = get_all_events(db);
    let mut multiplier = 1.0;
    for evt in &events {
        if evt.is_active() && evt.event_type == WorldEventType::DoubleExp {
            multiplier *= evt.multiplier;
        }
    }
    multiplier
}

/// 获取金币加成倍率（供战斗系统调用）
pub fn get_gold_multiplier(db: &Database) -> f64 {
    let events = get_all_events(db);
    let mut multiplier = 1.0;
    for evt in &events {
        if evt.is_active() && evt.event_type == WorldEventType::DoubleGold {
            multiplier *= evt.multiplier;
        }
    }
    multiplier
}

/// 获取掉落加成倍率（供战斗/采集系统调用）
pub fn get_drop_multiplier(db: &Database) -> f64 {
    let events = get_all_events(db);
    let mut multiplier = 1.0;
    for evt in &events {
        if evt.is_active() && evt.event_type == WorldEventType::DropBoost {
            multiplier *= evt.multiplier;
        }
    }
    multiplier
}

/// 是否有怪物入侵活动（供战斗系统调用）
pub fn is_monster_invasion_active(db: &Database) -> bool {
    let events = get_all_events(db);
    events
        .iter()
        .any(|e| e.is_active() && e.event_type == WorldEventType::MonsterInvasion)
}

/// 是否有全服福利活动（供在线奖励系统调用）
pub fn is_server_blessing_active(db: &Database) -> bool {
    let events = get_all_events(db);
    events
        .iter()
        .any(|e| e.is_active() && e.event_type == WorldEventType::ServerBlessing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(WorldEventType::from_str("双倍经验"), Some(WorldEventType::DoubleExp));
        assert_eq!(WorldEventType::from_str("double_exp"), Some(WorldEventType::DoubleExp));
        assert_eq!(WorldEventType::from_str("双倍金币"), Some(WorldEventType::DoubleGold));
        assert_eq!(
            WorldEventType::from_str("怪物入侵"),
            Some(WorldEventType::MonsterInvasion)
        );
        assert_eq!(WorldEventType::from_str("掉落加成"), Some(WorldEventType::DropBoost));
        assert_eq!(
            WorldEventType::from_str("全服福利"),
            Some(WorldEventType::ServerBlessing)
        );
        assert_eq!(WorldEventType::from_str("不存在"), None);
    }

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(WorldEventType::DoubleExp.as_str(), "双倍经验");
        assert_eq!(WorldEventType::DoubleGold.as_str(), "双倍金币");
        assert_eq!(WorldEventType::MonsterInvasion.as_str(), "怪物入侵");
        assert_eq!(WorldEventType::ServerBlessing.as_str(), "全服福利");
        assert_eq!(WorldEventType::DropBoost.as_str(), "掉落加成");
    }

    #[test]
    fn test_event_serialization_roundtrip() {
        let event = WorldEvent {
            name: "测试活动".to_string(),
            event_type: WorldEventType::DoubleExp,
            start_time: "2026-06-08 14:00:00".to_string(),
            end_time: "2026-06-09 14:00:00".to_string(),
            multiplier: 2.0,
            description: "战斗经验翻倍".to_string(),
            created_by: "gm001".to_string(),
        };
        let stored = event.to_storage();
        let restored = WorldEvent::from_storage(&stored).unwrap();
        assert_eq!(restored.name, "测试活动");
        assert_eq!(restored.event_type, WorldEventType::DoubleExp);
        assert_eq!(restored.multiplier, 2.0);
        assert_eq!(restored.created_by, "gm001");
    }

    #[test]
    fn test_event_multipliers() {
        assert_eq!(WorldEventType::DoubleExp.emoji(), "📈");
        assert_eq!(WorldEventType::DoubleGold.emoji(), "💰");
        assert_eq!(WorldEventType::MonsterInvasion.emoji(), "👹");
        assert_eq!(WorldEventType::ServerBlessing.emoji(), "🎁");
        assert_eq!(WorldEventType::DropBoost.emoji(), "🍀");
    }

    #[test]
    fn test_event_description() {
        assert_eq!(WorldEventType::DoubleExp.description(), "战斗经验翻倍");
        assert_eq!(WorldEventType::DoubleGold.description(), "战斗金币翻倍");
        assert_eq!(WorldEventType::DropBoost.description(), "所有掉落概率提升50%");
    }

    #[test]
    fn test_event_type_emoji_and_description() {
        for evt_type in &[
            WorldEventType::DoubleExp,
            WorldEventType::DoubleGold,
            WorldEventType::MonsterInvasion,
            WorldEventType::ServerBlessing,
            WorldEventType::DropBoost,
        ] {
            assert!(!evt_type.emoji().is_empty());
            assert!(!evt_type.description().is_empty());
            assert!(!evt_type.as_str().is_empty());
        }
    }

    #[test]
    fn test_event_type_from_str_roundtrip() {
        let types = vec!["双倍经验", "双倍金币", "怪物入侵", "全服福利", "掉落加成"];
        for s in &types {
            let evt = WorldEventType::from_str(s);
            assert!(evt.is_some(), "from_str should parse '{}'", s);
            assert_eq!(evt.unwrap().as_str(), *s);
        }
        assert!(WorldEventType::from_str("unknown").is_none());
        assert!(WorldEventType::from_str("DropBoost").is_none()); // English not supported
    }

    #[test]
    fn test_inactive_event_multipliers() {
        // Without DB, multipliers should return defaults (1.0)
        // This tests the logic path when no active events exist
        // Since we can't easily create a DB in unit tests, we test the WorldEvent.is_active logic
        let event = WorldEvent {
            name: "已过期活动".to_string(),
            event_type: WorldEventType::DoubleExp,
            start_time: "2020-01-01 00:00:00".to_string(),
            end_time: "2020-01-02 00:00:00".to_string(),
            multiplier: 2.0,
            description: "过期".to_string(),
            created_by: "gm".to_string(),
        };
        // Expired event should not be active
        assert!(!event.is_active());
    }
}

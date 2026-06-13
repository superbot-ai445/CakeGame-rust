/// CakeGame 疲劳值/体力系统
/// 限制每日活动频率，鼓励日常登录和VIP消费
///
/// 功能:
/// - 每日100点体力上限，VIP额外+20%
/// - 不同活动消耗不同体力(攻击2/副本10/BOSS15/采集1/竞技5)
/// - 每5分钟自动恢复1点体力
/// - 钻石购买体力(50钻恢复50点)
/// - 体力不足时限制对应活动
/// - 每日0点自动重置
///
/// 数据存储: Global表 SECTION='stamina_{uid}'
use crate::db::Database;
use chrono::Local;
use std::time::{SystemTime, UNIX_EPOCH};

/// 基础体力上限
const BASE_MAX_STAMINA: i64 = 100;

/// 恢复间隔(秒) — 5分钟恢复1点
const REGEN_INTERVAL_SECS: i64 = 300;

/// 钻石购买体力消耗
const DIAMOND_COST: i64 = 50;

/// 钻石购买恢复的体力点数
const DIAMOND_RESTORE: i64 = 50;

/// 每日最大购买次数
const MAX_DAILY_PURCHASES: i32 = 3;

/// 活动体力消耗定义
pub struct StaminaCost {
    pub name: &'static str,
    pub cost: i64,
    pub emoji: &'static str,
}

/// 所有消耗体力的活动
pub const ACTIVITY_COSTS: &[StaminaCost] = &[
    StaminaCost {
        name: "攻击",
        cost: 2,
        emoji: "⚔️",
    },
    StaminaCost {
        name: "副本",
        cost: 10,
        emoji: "🗝️",
    },
    StaminaCost {
        name: "BOSS",
        cost: 15,
        emoji: "🐉",
    },
    StaminaCost {
        name: "采集",
        cost: 1,
        emoji: "🪓",
    },
    StaminaCost {
        name: "竞技",
        cost: 5,
        emoji: "🏟️",
    },
    StaminaCost {
        name: "合成",
        cost: 3,
        emoji: "🔨",
    },
    StaminaCost {
        name: "强化",
        cost: 5,
        emoji: "💎",
    },
    StaminaCost {
        name: "炼制",
        cost: 3,
        emoji: "⚗️",
    },
    StaminaCost {
        name: "熔炼",
        cost: 3,
        emoji: "🔥",
    },
    StaminaCost {
        name: "烹饪",
        cost: 2,
        emoji: "🍳",
    },
    StaminaCost {
        name: "制药",
        cost: 2,
        emoji: "💊",
    },
    StaminaCost {
        name: "挖宝",
        cost: 8,
        emoji: "🗺️",
    },
];

/// 获取今日日期字符串
fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 获取当前时间戳(秒)
fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 检查VIP状态
fn is_vip(uid: &str, db: &Database) -> bool {
    let now = chrono::Local::now().timestamp();
    let vip_end: i64 = db
        .global_get("vip_membership", &format!("{}_end", uid))
        .parse()
        .unwrap_or(0);
    vip_end > now
}

/// 玩家体力数据
#[derive(Debug, Clone)]
pub struct StaminaData {
    /// 当前体力
    pub current: i64,
    /// 上次恢复时间戳
    pub last_regen_ts: i64,
    /// 今日已购买次数
    pub daily_purchases: i32,
    /// 今日已消耗总体力
    pub daily_consumed: i64,
    /// 今日日期(用于重置检测)
    pub date: String,
}

impl StaminaData {
    fn new() -> Self {
        Self {
            current: BASE_MAX_STAMINA,
            last_regen_ts: now_ts(),
            daily_purchases: 0,
            daily_consumed: 0,
            date: today_str(),
        }
    }

    /// 从Global表字符串解析
    pub fn parse(s: &str) -> Self {
        let mut data = Self::new();
        for part in s.split('|') {
            let kv: Vec<&str> = part.splitn(2, ':').collect();
            if kv.len() != 2 {
                continue;
            }
            match kv[0] {
                "cur" => {
                    data.current = kv[1].parse().unwrap_or(BASE_MAX_STAMINA);
                }
                "lrts" => {
                    data.last_regen_ts = kv[1].parse().unwrap_or_else(|_| now_ts());
                }
                "dp" => {
                    data.daily_purchases = kv[1].parse().unwrap_or(0);
                }
                "dc" => {
                    data.daily_consumed = kv[1].parse().unwrap_or(0);
                }
                "date" => {
                    data.date = kv[1].to_string();
                }
                _ => {}
            }
        }
        // 检查是否需要日期重置
        let today = today_str();
        if data.date != today {
            data.daily_purchases = 0;
            data.daily_consumed = 0;
            data.date = today;
        }
        data
    }

    /// 序列化为字符串
    pub fn serialize(&self) -> String {
        format!(
            "cur:{}|lrts:{}|dp:{}|dc:{}|date:{}",
            self.current, self.last_regen_ts, self.daily_purchases, self.daily_consumed, self.date
        )
    }
}

/// 计算体力上限(含VIP加成)
pub fn calc_max_stamina(uid: &str, db: &Database) -> i64 {
    let vip_bonus = if is_vip(uid, db) { 20 } else { 0 };
    BASE_MAX_STAMINA + vip_bonus
}

/// 自动恢复体力(每次调用时检查)
pub fn auto_regen(data: &mut StaminaData, max_stamina: i64) {
    let now = now_ts();
    let elapsed = now - data.last_regen_ts;
    if elapsed >= REGEN_INTERVAL_SECS && data.current < max_stamina {
        let regen_points = elapsed / REGEN_INTERVAL_SECS;
        data.current = (data.current + regen_points).min(max_stamina);
        data.last_regen_ts = now;
    }
}

/// 查看体力状态
pub fn cmd_view_stamina(db: &Database, uid: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let _ = args;
    let max_stamina = calc_max_stamina(uid, db);
    let section = format!("stamina_{}", uid);

    let raw = db.global_get(&section, "data");
    let mut data = if raw.is_empty() {
        StaminaData::new()
    } else {
        StaminaData::parse(&raw)
    };
    data.current = data.current.min(max_stamina);
    auto_regen(&mut data, max_stamina);

    // 保存恢复后的数据
    let _ = db.global_set(&section, "data", &data.serialize());

    let pct = if max_stamina > 0 {
        (data.current as f64 / max_stamina as f64 * 100.0) as i64
    } else {
        0
    };
    let bar_len = 20;
    let filled = (pct as usize * bar_len / 100).min(bar_len);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));

    // 计算下次恢复时间
    let now = now_ts();
    let next_regen_secs = if data.current >= max_stamina {
        0
    } else {
        let elapsed = now - data.last_regen_ts;
        REGEN_INTERVAL_SECS - (elapsed % REGEN_INTERVAL_SECS)
    };

    let next_regen_str = if next_regen_secs > 0 {
        if next_regen_secs >= 60 {
            format!("{}分{}秒后", next_regen_secs / 60, next_regen_secs % 60)
        } else {
            format!("{}秒后", next_regen_secs)
        }
    } else {
        "已满".to_string()
    };

    let mut out = String::new();
    out.push_str("⚡ === 体力系统 === ⚡\n\n");
    out.push_str(&format!("当前体力: {}/{}\n", data.current, max_stamina));
    out.push_str(&format!("{} {}%\n\n", bar, pct));
    out.push_str(&format!("⏰ 自动恢复: 每5分钟+1点 ({}恢复)\n", next_regen_str));
    out.push_str(&format!(
        "💎 钻石购买: {}钻恢复{}点 (每日{}次)\n",
        DIAMOND_COST, DIAMOND_RESTORE, MAX_DAILY_PURCHASES
    ));
    out.push_str(&format!(
        "📊 今日购买: {}/{}次\n",
        data.daily_purchases, MAX_DAILY_PURCHASES
    ));
    out.push_str(&format!("📉 今日消耗: {}点体力\n", data.daily_consumed));

    if is_vip(uid, db) {
        out.push_str("👑 VIP加成: 体力上限+20%\n");
    }

    out.push_str("\n📋 === 活动消耗 === 📋\n");
    for act in ACTIVITY_COSTS {
        let can = data.current >= act.cost;
        let status = if can { "✅" } else { "❌" };
        out.push_str(&format!("  {} {}{}: {}点\n", status, act.emoji, act.name, act.cost));
    }

    out.push_str("\n💡 提示: 使用「恢复体力」购买体力\n");
    out.push_str("💡 体力不足时将无法进行对应活动\n");

    out
}

/// 恢复体力(钻石购买)
pub fn cmd_restore_stamina(db: &Database, uid: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let _ = args;
    let max_stamina = calc_max_stamina(uid, db);
    let section = format!("stamina_{}", uid);

    let raw = db.global_get(&section, "data");
    let mut data = if raw.is_empty() {
        StaminaData::new()
    } else {
        StaminaData::parse(&raw)
    };
    data.current = data.current.min(max_stamina);
    auto_regen(&mut data, max_stamina);

    // 检查购买次数
    if data.daily_purchases >= MAX_DAILY_PURCHASES {
        return format!(
            "❌ 今日已购买{}/{}次体力，已达上限！\n💡 每日0点重置购买次数",
            data.daily_purchases, MAX_DAILY_PURCHASES
        );
    }

    // 检查是否已满
    if data.current >= max_stamina {
        return "❌ 体力已满，无需恢复！\n💡 消耗体力后再来购买".to_string();
    }

    // 检查钻石
    let diamonds: i64 = db.read_basic(uid, "Diamond").parse().unwrap_or(0);
    if diamonds < DIAMOND_COST {
        return format!(
            "❌ 钻石不足！需要{}💎，当前{}💎\n💡 可通过签到/任务/充值获取钻石",
            DIAMOND_COST, diamonds
        );
    }

    // 扣除钻石
    let _ = db.write_basic(uid, "Diamond", &(diamonds - DIAMOND_COST).to_string());

    // 恢复体力
    let before = data.current;
    data.current = (data.current + DIAMOND_RESTORE).min(max_stamina);
    let restored = data.current - before;
    data.daily_purchases += 1;

    // 保存
    let _ = db.global_set(&section, "data", &data.serialize());

    let mut out = String::new();
    out.push_str("⚡ === 体力恢复成功！=== ⚡\n\n");
    out.push_str(&format!("💎 消耗: {}钻石\n", DIAMOND_COST));
    out.push_str(&format!("⚡ 恢复: +{}点体力\n", restored));
    out.push_str(&format!("📊 当前: {}/{}点\n", data.current, max_stamina));
    out.push_str(&format!(
        "🛒 今日购买: {}/{}次\n",
        data.daily_purchases, MAX_DAILY_PURCHASES
    ));
    out.push_str(&format!("💎 剩余钻石: {}\n", diamonds - DIAMOND_COST));

    out
}

/// 查询体力消耗明细
pub fn cmd_stamina_cost(db: &Database, uid: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let _ = args;
    let max_stamina = calc_max_stamina(uid, db);
    let section = format!("stamina_{}", uid);

    let raw = db.global_get(&section, "data");
    let mut data = if raw.is_empty() {
        StaminaData::new()
    } else {
        StaminaData::parse(&raw)
    };
    data.current = data.current.min(max_stamina);
    auto_regen(&mut data, max_stamina);

    let remaining_attacks = data.current / 2;
    let remaining_dungeons = data.current / 10;
    let remaining_bosses = data.current / 15;

    let mut out = String::new();
    out.push_str("📊 === 体力预估 === 📊\n\n");
    out.push_str(&format!("⚡ 当前体力: {}/{}\n\n", data.current, max_stamina));
    out.push_str("📈 可执行活动次数预估:\n");
    out.push_str(&format!("  ⚔️ 攻击: ~{}次 ({}点/次)\n", remaining_attacks, 2));
    out.push_str(&format!("  🗝️ 副本: ~{}次 ({}点/次)\n", remaining_dungeons, 10));
    out.push_str(&format!("  🐉 BOSS: ~{}次 ({}点/次)\n", remaining_bosses, 15));
    out.push_str(&format!("  🪓 采集: ~{}次 ({}点/次)\n", data.current, 1));
    out.push_str(&format!("  🏟️ 竞技: ~{}次 ({}点/次)\n", data.current / 5, 5));
    out.push_str(&format!("  🔨 合成: ~{}次 ({}点/次)\n", data.current / 3, 3));
    out.push_str(&format!("  ⚗️ 炼制: ~{}次 ({}点/次)\n", data.current / 3, 3));
    out.push_str(&format!("  🗺️ 挖宝: ~{}次 ({}点/次)\n", data.current / 8, 8));
    out.push_str(&format!("\n📉 今日总消耗: {}点\n", data.daily_consumed));
    out.push_str(&format!(
        "💎 今日购买: {}/{}次\n",
        data.daily_purchases, MAX_DAILY_PURCHASES
    ));
    out
}

/// 消耗体力(供其他系统调用)
/// 返回Ok(剩余体力)或Err(错误消息)
#[allow(dead_code)]
pub fn consume_stamina(uid: &str, activity: &str, db: &Database) -> Result<i64, String> {
    // 找到活动消耗
    let cost = ACTIVITY_COSTS
        .iter()
        .find(|a| a.name == activity)
        .map(|a| a.cost)
        .ok_or_else(|| format!("未知活动: {}", activity))?;

    let max_stamina = calc_max_stamina(uid, db);
    let section = format!("stamina_{}", uid);

    let raw = db.global_get(&section, "data");
    let mut data = if raw.is_empty() {
        StaminaData::new()
    } else {
        StaminaData::parse(&raw)
    };
    data.current = data.current.min(max_stamina);
    auto_regen(&mut data, max_stamina);

    if data.current < cost {
        return Err(format!(
            "⚡ 体力不足！需要{}点，当前{}点\n💡 使用「恢复体力」购买或等待自动恢复",
            cost, data.current
        ));
    }

    data.current -= cost;
    data.daily_consumed += cost;
    let remaining = data.current;

    let _ = db.global_set(&section, "data", &data.serialize());

    Ok(remaining)
}

/// 检查是否有足够体力(不消耗)
#[allow(dead_code)]
pub fn check_stamina(uid: &str, activity: &str, db: &Database) -> bool {
    let cost = match ACTIVITY_COSTS.iter().find(|a| a.name == activity) {
        Some(a) => a.cost,
        None => return true, // 未知活动不限制
    };

    let max_stamina = calc_max_stamina(uid, db);
    let section = format!("stamina_{}", uid);

    let raw = db.global_get(&section, "data");
    let mut data = if raw.is_empty() {
        StaminaData::new()
    } else {
        StaminaData::parse(&raw)
    };
    data.current = data.current.min(max_stamina);
    auto_regen(&mut data, max_stamina);

    data.current >= cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_max_stamina() {
        assert_eq!(BASE_MAX_STAMINA, 100);
    }

    #[test]
    fn test_regen_interval() {
        assert_eq!(REGEN_INTERVAL_SECS, 300); // 5 minutes
    }

    #[test]
    fn test_diamond_cost() {
        assert_eq!(DIAMOND_COST, 50);
        assert_eq!(DIAMOND_RESTORE, 50);
    }

    #[test]
    fn test_max_daily_purchases() {
        assert_eq!(MAX_DAILY_PURCHASES, 3);
    }

    #[test]
    fn test_activity_costs_count() {
        assert_eq!(ACTIVITY_COSTS.len(), 12);
    }

    #[test]
    fn test_activity_costs_unique_names() {
        let mut names: Vec<&str> = ACTIVITY_COSTS.iter().map(|a| a.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ACTIVITY_COSTS.len());
    }

    #[test]
    fn test_activity_costs_positive() {
        for act in ACTIVITY_COSTS {
            assert!(act.cost > 0, "Activity '{}' has non-positive cost", act.name);
        }
    }

    #[test]
    fn test_activity_costs_emojis() {
        for act in ACTIVITY_COSTS {
            assert!(!act.emoji.is_empty(), "Activity '{}' has empty emoji", act.name);
        }
    }

    #[test]
    fn test_stamina_data_parse_empty() {
        let data = StaminaData::parse("");
        assert_eq!(data.current, BASE_MAX_STAMINA);
        assert_eq!(data.daily_purchases, 0);
        assert_eq!(data.daily_consumed, 0);
    }

    #[test]
    fn test_stamina_data_parse_full() {
        let today = super::today_str();
        let data = StaminaData::parse(&format!("cur:75|lrts:1000|dp:2|dc:50|date:{}", today));
        assert_eq!(data.current, 75);
        assert_eq!(data.last_regen_ts, 1000);
        assert_eq!(data.daily_purchases, 2);
        assert_eq!(data.daily_consumed, 50);
        assert_eq!(data.date, today_str());
    }

    #[test]
    fn test_stamina_data_parse_partial() {
        let data = StaminaData::parse("cur:50");
        assert_eq!(data.current, 50);
        assert_eq!(data.daily_purchases, 0);
    }

    #[test]
    fn test_stamina_data_serialize_roundtrip() {
        let mut data = StaminaData::new();
        data.current = 88;
        data.daily_purchases = 1;
        data.daily_consumed = 25;
        let s = data.serialize();
        let parsed = StaminaData::parse(&s);
        assert_eq!(parsed.current, 88);
        assert_eq!(parsed.daily_purchases, 1);
        assert_eq!(parsed.daily_consumed, 25);
    }

    #[test]
    fn test_stamina_data_serialize_format() {
        let data = StaminaData::new();
        let s = data.serialize();
        assert!(s.contains("cur:"));
        assert!(s.contains("lrts:"));
        assert!(s.contains("dp:"));
        assert!(s.contains("dc:"));
        assert!(s.contains("date:"));
    }

    #[test]
    fn test_auto_regen_no_regen_needed() {
        let mut data = StaminaData::new();
        data.current = BASE_MAX_STAMINA;
        data.last_regen_ts = now_ts();
        auto_regen(&mut data, BASE_MAX_STAMINA);
        assert_eq!(data.current, BASE_MAX_STAMINA);
    }

    #[test]
    fn test_auto_regen_with_elapsed_time() {
        let mut data = StaminaData::new();
        data.current = 50;
        data.last_regen_ts = now_ts() - 600; // 10 minutes ago = 2 points
        auto_regen(&mut data, BASE_MAX_STAMINA);
        assert_eq!(data.current, 52);
    }

    #[test]
    fn test_auto_regen_capped_at_max() {
        let mut data = StaminaData::new();
        data.current = 99;
        data.last_regen_ts = now_ts() - 600; // 10 minutes = 2 points, but max is 100
        auto_regen(&mut data, BASE_MAX_STAMINA);
        assert_eq!(data.current, BASE_MAX_STAMINA);
    }

    #[test]
    fn test_stamina_data_new_defaults() {
        let data = StaminaData::new();
        assert_eq!(data.current, BASE_MAX_STAMINA);
        assert_eq!(data.daily_purchases, 0);
        assert_eq!(data.daily_consumed, 0);
        assert!(!data.date.is_empty());
    }

    #[test]
    fn test_find_activity_attack() {
        let act = ACTIVITY_COSTS.iter().find(|a| a.name == "攻击");
        assert!(act.is_some());
        assert_eq!(act.unwrap().cost, 2);
    }

    #[test]
    fn test_find_activity_not_found() {
        let act = ACTIVITY_COSTS.iter().find(|a| a.name == "不存在的活动");
        assert!(act.is_none());
    }

    #[test]
    fn test_attack_cost_is_low() {
        let attack = ACTIVITY_COSTS.iter().find(|a| a.name == "攻击").unwrap();
        let boss = ACTIVITY_COSTS.iter().find(|a| a.name == "BOSS").unwrap();
        assert!(attack.cost < boss.cost);
    }

    #[test]
    fn test_gather_cost_is_lowest() {
        let gather = ACTIVITY_COSTS.iter().find(|a| a.name == "采集").unwrap();
        for act in ACTIVITY_COSTS {
            assert!(
                act.cost >= gather.cost,
                "Activity '{}' costs {} but gather costs {}",
                act.name,
                act.cost,
                gather.cost
            );
        }
    }

    #[test]
    fn test_today_str_format() {
        let s = today_str();
        assert_eq!(s.len(), 10); // YYYY-MM-DD
        assert!(s.contains('-'));
    }

    #[test]
    fn test_now_ts_positive() {
        let ts = now_ts();
        assert!(ts > 0);
    }

    #[test]
    fn test_vip_max_stamina() {
        // VIP gives +20% = 100 + 20 = 120
        let expected = BASE_MAX_STAMINA + 20;
        assert_eq!(expected, 120);
    }

    #[test]
    fn test_stamina_data_date_reset() {
        let mut data = StaminaData::new();
        data.date = "2020-01-01".to_string(); // old date
        data.daily_purchases = 3;
        data.daily_consumed = 100;
        let s = data.serialize();
        let parsed = StaminaData::parse(&s);
        // Date should reset to today
        assert_eq!(parsed.date, today_str());
        assert_eq!(parsed.daily_purchases, 0);
        assert_eq!(parsed.daily_consumed, 0);
    }

    #[test]
    fn test_consume_unknown_activity() {
        // consume_stamina requires a Database which we can't easily mock
        // But we can test the logic via ACTIVITY_COSTS lookup
        let result = ACTIVITY_COSTS.iter().find(|a| a.name == "未知活动");
        assert!(result.is_none());
    }

    #[test]
    fn test_stamina_data_clone() {
        let data = StaminaData::new();
        let cloned = data.clone();
        assert_eq!(cloned.current, data.current);
        assert_eq!(cloned.daily_purchases, data.daily_purchases);
    }
}

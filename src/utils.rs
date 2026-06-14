use crate::db::Database;
use chrono::Local;

/// 获取当前时间字符串
pub fn chrono_now() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 获取当前日期字符串
pub fn chrono_now_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 获取当前时间戳
pub fn chrono_timestamp() -> i64 {
    Local::now().timestamp()
}

/// DJB2 哈希
pub fn djb2_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// 基于用户ID和日期的随机种子
pub fn daily_seed(user_id: &str) -> u64 {
    let today = chrono_now_date();
    djb2_hash(&format!("{}:{}", user_id, today))
}

/// 格式化数字（带逗号分隔）
pub fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 格式化金币
pub fn format_gold(n: i64) -> String {
    if n >= 100000000 {
        format!("{:.1}亿", n as f64 / 100000000.0)
    } else if n >= 10000 {
        format!("{:.1}万", n as f64 / 10000.0)
    } else {
        format_number(n)
    }
}

/// 格式化分钟为可读时间
pub fn format_minutes(minutes: i64) -> String {
    if minutes < 60 {
        format!("{}分钟", minutes)
    } else if minutes < 1440 {
        format!("{}小时{}分钟", minutes / 60, minutes % 60)
    } else {
        format!("{}天{}小时", minutes / 1440, (minutes % 1440) / 60)
    }
}

/// 属性显示名称
pub fn attr_display_name(attr: &str) -> &'static str {
    match attr {
        "hp" | "HP" => "生命",
        "ad" | "AD" => "攻击",
        "ap" | "AP" => "法术",
        "def" | "DEF" => "防御",
        "spd" | "SPD" => "速度",
        "hit" | "HIT" => "命中",
        "dodge" | "DODGE" => "闪避",
        "crit" | "CRIT" => "暴击",
        "crit_dmg" => "暴击伤害",
        "block" => "格挡",
        "penetrate" => "穿透",
        "lifesteal" => "吸血",
        "resist" => "抗性",
        _ => "未知属性",
    }
}

/// 格式化效果描述
pub fn format_effect(effect_type: &str, value: i32) -> String {
    match effect_type {
        "hp" => format!("生命+{}", value),
        "ad" => format!("攻击+{}", value),
        "ap" => format!("法术+{}", value),
        "def" => format!("防御+{}", value),
        "spd" => format!("速度+{}", value),
        "hit" => format!("命中+{}", value),
        "dodge" => format!("闪避+{}", value),
        "crit" => format!("暴击+{}%", value),
        "crit_dmg" => format!("暴击伤害+{}%", value),
        "block" => format!("格挡+{}", value),
        "penetrate" => format!("穿透+{}", value),
        "lifesteal" => format!("吸血+{}%", value),
        "resist" => format!("抗性+{}", value),
        _ => format!("{}+{}", effect_type, value),
    }
}

/// 怪物名称格式化（截断过长名称）
pub fn floor_monster_name(name: &str, max_len: usize) -> String {
    if name.len() > max_len {
        format!("{}...", &name[..max_len])
    } else {
        name.to_string()
    }
}

/// 计算战斗力
pub fn calc_power(hp: i32, ad: i32, ap: i32, def: i32, spd: i32, hit: i32, dodge: i32, crit: i32) -> i64 {
    (hp as i64 * 1
        + ad as i64 * 10
        + ap as i64 * 10
        + def as i64 * 5
        + spd as i64 * 20
        + hit as i64 * 5
        + dodge as i64 * 5
        + crit as i64 * 8)
}

/// 计算等级所需经验
pub fn level_exp(level: i32) -> i64 {
    (level as i64).pow(2) * 100 + level as i64 * 500
}

/// 安全获取字符串字段
pub fn safe_string(row: &rusqlite::Row, idx: usize) -> String {
    row.get::<_, String>(idx).unwrap_or_default()
}

/// 安全获取整数字段
pub fn safe_i32(row: &rusqlite::Row, idx: usize) -> i32 {
    row.get::<_, i32>(idx).unwrap_or(0)
}

/// 安全获取i64字段
pub fn safe_i64(row: &rusqlite::Row, idx: usize) -> i64 {
    row.get::<_, i64>(idx).unwrap_or(0)
}

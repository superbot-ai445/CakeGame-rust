/// CakeGame 每日活跃度系统
/// 追踪玩家日常行为并发放里程碑奖励
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::{Datelike, Local};

/// 活跃度里程碑定义
struct ActivityMilestone {
    points: i32,
    name: &'static str,
    reward_gold: i32,
    reward_diamond: i32,
    reward_exp: i32,
    reward_item: &'static str,
    emoji: &'static str,
}

const MILESTONES: &[ActivityMilestone] = &[
    ActivityMilestone {
        points: 20,
        name: "活跃新手",
        reward_gold: 200,
        reward_diamond: 5,
        reward_exp: 100,
        reward_item: "",
        emoji: "🌱",
    },
    ActivityMilestone {
        points: 40,
        name: "活跃达人",
        reward_gold: 500,
        reward_diamond: 10,
        reward_exp: 250,
        reward_item: "【普通】生命药水*3",
        emoji: "🌿",
    },
    ActivityMilestone {
        points: 60,
        name: "活跃精英",
        reward_gold: 800,
        reward_diamond: 15,
        reward_exp: 400,
        reward_item: "【普通】魔力药水*3",
        emoji: "🌳",
    },
    ActivityMilestone {
        points: 80,
        name: "活跃大师",
        reward_gold: 1200,
        reward_diamond: 20,
        reward_exp: 600,
        reward_item: "白色精粹*10",
        emoji: "⭐",
    },
    ActivityMilestone {
        points: 100,
        name: "活跃之王",
        reward_gold: 2000,
        reward_diamond: 50,
        reward_exp: 1000,
        reward_item: "【经典】勇士礼盒*1",
        emoji: "👑",
    },
];

/// 活跃度来源定义
struct ActivitySource {
    id: &'static str,
    name: &'static str,
    points: i32,
    max_daily: i32,
    emoji: &'static str,
}

const ACTIVITY_SOURCES: &[ActivitySource] = &[
    ActivitySource {
        id: "sign_in",
        name: "每日签到",
        points: 10,
        max_daily: 1,
        emoji: "📝",
    },
    ActivitySource {
        id: "attack",
        name: "击败怪物",
        points: 2,
        max_daily: 20,
        emoji: "⚔️",
    },
    ActivitySource {
        id: "gather",
        name: "采集资源",
        points: 3,
        max_daily: 10,
        emoji: "🪓",
    },
    ActivitySource {
        id: "quest",
        name: "完成任务",
        points: 10,
        max_daily: 3,
        emoji: "📜",
    },
    ActivitySource {
        id: "shop_buy",
        name: "购买商品",
        points: 2,
        max_daily: 5,
        emoji: "🛒",
    },
    ActivitySource {
        id: "cook",
        name: "烹饪制作",
        points: 3,
        max_daily: 5,
        emoji: "🍳",
    },
    ActivitySource {
        id: "smelt",
        name: "熔炼制作",
        points: 3,
        max_daily: 5,
        emoji: "🔥",
    },
    ActivitySource {
        id: "alchemy",
        name: "炼制制作",
        points: 3,
        max_daily: 5,
        emoji: "⚗️",
    },
    ActivitySource {
        id: "pharmacy",
        name: "制药制作",
        points: 3,
        max_daily: 5,
        emoji: "💊",
    },
    ActivitySource {
        id: "arena",
        name: "匹配竞技",
        points: 5,
        max_daily: 5,
        emoji: "🏟️",
    },
    ActivitySource {
        id: "pray",
        name: "每日祈福",
        points: 5,
        max_daily: 1,
        emoji: "🙏",
    },
    ActivitySource {
        id: "training",
        name: "修炼突破",
        points: 5,
        max_daily: 3,
        emoji: "🧘",
    },
    ActivitySource {
        id: "dungeon",
        name: "副本通关",
        points: 8,
        max_daily: 3,
        emoji: "🏰",
    },
    ActivitySource {
        id: "lottery",
        name: "抽奖",
        points: 2,
        max_daily: 10,
        emoji: "🎰",
    },
    ActivitySource {
        id: "gift",
        name: "赠送他人",
        points: 3,
        max_daily: 3,
        emoji: "🎁",
    },
];

/// 生成今日日期字符串
fn today_str() -> String {
    let now = Local::now();
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

/// 读取活跃度数据 (格式: "日期|总分|id1:count1,id2:count2,...|claimed1,claimed2,...")
fn read_activity_data(db: &Database, user_id: &str) -> (String, i32, Vec<(String, i32)>, Vec<i32>) {
    let raw = db.read_user_data(user_id, "daily_activity");
    let today = today_str();

    if raw.is_empty() {
        return (today, 0, Vec::new(), Vec::new());
    }

    let parts: Vec<&str> = raw.splitn(4, '|').collect();
    let date = parts.first().unwrap_or(&"").to_string();

    if date != today {
        // 新的一天，重置
        return (today, 0, Vec::new(), Vec::new());
    }

    let total: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    let counts: Vec<(String, i32)> = parts
        .get(2)
        .unwrap_or(&"")
        .split(',')
        .filter_map(|s| {
            let kv: Vec<&str> = s.splitn(2, ':').collect();
            if kv.len() == 2 {
                Some((kv[0].to_string(), kv[1].parse().unwrap_or(0)))
            } else {
                None
            }
        })
        .collect();

    let claimed: Vec<i32> = parts
        .get(3)
        .unwrap_or(&"")
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();

    (date, total, counts, claimed)
}

/// 写入活跃度数据
fn write_activity_data(
    db: &Database,
    user_id: &str,
    date: &str,
    total: i32,
    counts: &[(String, i32)],
    claimed: &[i32],
) {
    let counts_str = counts
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect::<Vec<_>>()
        .join(",");
    let claimed_str = claimed.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
    let data = format!("{}|{}|{}|{}", date, total, counts_str, claimed_str);
    let _ = db.write_user_data(user_id, "daily_activity", &data);
}

/// 查看活跃度 - 显示今日活跃度总览和里程碑进度
pub fn cmd_view_activity(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (_, total, counts, claimed) = read_activity_data(db, user_id);
    let today = today_str();

    let mut out = format!("{}\n", prefix);
    out += "📊 ═══ 每日活跃度 ═══\n";
    out += &format!("📅 {}\n", today);
    out += &format!("🎯 今日活跃度: {} 分\n\n", total);

    // 活跃度来源明细
    out += "━━━ 活跃度来源 ━━━\n";
    for source in ACTIVITY_SOURCES {
        let current = counts
            .iter()
            .find(|(k, _)| k == source.id)
            .map(|(_, v)| *v)
            .unwrap_or(0);
        let max_points = source.points * source.max_daily;
        let earned = current * source.points;
        let status = if current >= source.max_daily {
            "✅".to_string()
        } else {
            format!("{}/{}", current, source.max_daily)
        };
        out += &format!(
            "{} {} ({}) +{}/{}分\n",
            source.emoji, source.name, status, earned, max_points
        );
    }

    // 里程碑进度
    out += "\n━━━ 活跃度奖励 ━━━\n";
    for milestone in MILESTONES {
        let is_claimed = claimed.contains(&milestone.points);
        let can_claim = total >= milestone.points && !is_claimed;
        let status = if is_claimed {
            "✅已领取"
        } else if can_claim {
            "🎁可领取"
        } else {
            "🔒未达成"
        };
        let bar_len = 10;
        let fill = ((total.min(milestone.points) as f64 / milestone.points as f64) * bar_len as f64) as i32;
        let empty = bar_len - fill;
        let bar = format!("{}{}", "█".repeat(fill as usize), "░".repeat(empty as usize));

        out += &format!(
            "{} {} {}分 [{}] {}",
            milestone.emoji, milestone.name, milestone.points, bar, status
        );
        if !milestone.reward_item.is_empty() && !is_claimed {
            out += &format!(" → {}", milestone.reward_item);
        }
        out += "\n";
    }

    out += "\n💡 活跃度每日0点重置\n";
    out += "📌 发送「领取活跃+分数」领取奖励\n";
    out
}

/// 领取活跃度奖励
pub fn cmd_claim_activity(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (_, total, counts, mut claimed) = read_activity_data(db, user_id);

    let points: i32 = match args.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            let available: Vec<String> = MILESTONES
                .iter()
                .filter(|m| total >= m.points && !claimed.contains(&m.points))
                .map(|m| format!("{}分({})", m.points, m.name))
                .collect();
            if available.is_empty() {
                return format!("{}\n📭 暂无可领取的活跃度奖励！\n💡 完成日常活动累积活跃度", prefix);
            }
            return format!(
                "{}\n🎁 可领取的奖励: {}\n📌 发送「领取活跃+分数」领取",
                prefix,
                available.join(", ")
            );
        }
    };

    // 查找对应里程碑
    let milestone = match MILESTONES.iter().find(|m| m.points == points) {
        Some(m) => m,
        None => {
            return format!(
                "{}\n❌ 无效的活跃度等级！可选: {}",
                prefix,
                MILESTONES
                    .iter()
                    .map(|m| format!("{}分", m.points))
                    .collect::<Vec<_>>()
                    .join("/")
            );
        }
    };

    // 检查条件
    if total < points {
        return format!("{}\n❌ 活跃度不足！当前 {} 分，需要 {} 分", prefix, total, points);
    }
    if claimed.contains(&points) {
        return format!("{}\n❌ 该奖励今日已领取！明天再来吧~", prefix);
    }

    // 发放奖励
    let gold = db.read_basic(user_id, CURRENCY_GOLD).parse::<i64>().unwrap_or(0);
    let _ = db.write_basic(
        user_id,
        CURRENCY_GOLD,
        &(gold + milestone.reward_gold as i64).to_string(),
    );

    let diamond = db.read_basic(user_id, CURRENCY_DIAMOND).parse::<i64>().unwrap_or(0);
    let _ = db.write_basic(
        user_id,
        CURRENCY_DIAMOND,
        &(diamond + milestone.reward_diamond as i64).to_string(),
    );

    let exp = db.read_basic(user_id, ITEM_EXP).parse::<i32>().unwrap_or(0);
    let _ = db.write_basic_int(user_id, ITEM_EXP, exp + milestone.reward_exp);

    // 发放物品奖励
    if !milestone.reward_item.is_empty() {
        let parts: Vec<&str> = milestone.reward_item.split('*').collect();
        let item_name = parts[0];
        let item_count: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        db.add_item(user_id, item_name, item_count);
    }

    // 标记已领取
    claimed.push(points);
    let today = today_str();
    write_activity_data(db, user_id, &today, total, &counts, &claimed);

    let mut out = format!("{}\n", prefix);
    out += &format!("{} 🎉 领取成功！\n", milestone.emoji);
    out += &format!("━━━ {} ━━━\n", milestone.name);
    out += &format!("💰 金币 +{}\n", milestone.reward_gold);
    out += &format!("💎 钻石 +{}\n", milestone.reward_diamond);
    out += &format!("⭐ 经验 +{}\n", milestone.reward_exp);
    if !milestone.reward_item.is_empty() {
        out += &format!("🎁 物品 +{}\n", milestone.reward_item);
    }
    out += "\n继续完成更多活动，领取更高级奖励！\n";
    out
}

/// 增加活跃度（供其他模块调用）
pub fn add_activity(db: &Database, user_id: &str, source_id: &str) -> i32 {
    let today = today_str();
    let (date, mut total, mut counts, claimed) = read_activity_data(db, user_id);

    // 确保是今天的数据
    let date = if date != today { today.clone() } else { date };

    // 查找来源定义
    let source = match ACTIVITY_SOURCES.iter().find(|s| s.id == source_id) {
        Some(s) => s,
        None => return 0,
    };

    // 查找当前计数
    let mut entry = counts.iter_mut().find(|(k, _)| k == source_id);
    if let Some(ref mut entry) = entry {
        let current = &mut entry.1;
        if *current >= source.max_daily {
            return 0;
        }
        *current += 1;
    } else {
        counts.push((source_id.to_string(), 1));
    }
    total += source.points;

    write_activity_data(db, user_id, &date, total, &counts, &claimed);
    source.points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milestones_count() {
        assert!(
            MILESTONES.len() >= 4,
            "Expected at least 4 milestones, got {}",
            MILESTONES.len()
        );
    }

    #[test]
    fn test_milestones_sorted_by_points() {
        for i in 1..MILESTONES.len() {
            assert!(
                MILESTONES[i].points > MILESTONES[i - 1].points,
                "Milestone {} points ({}) <= milestone {} points ({})",
                i,
                MILESTONES[i].points,
                i - 1,
                MILESTONES[i - 1].points
            );
        }
    }

    #[test]
    fn test_milestones_positive_rewards() {
        for m in MILESTONES.iter() {
            assert!(m.points > 0, "{}: points must be > 0", m.name);
            assert!(m.reward_gold >= 0, "{}: gold must be >= 0", m.name);
            assert!(m.reward_diamond >= 0, "{}: diamond must be >= 0", m.name);
            assert!(m.reward_exp > 0, "{}: exp must be > 0", m.name);
        }
    }

    #[test]
    fn test_milestones_unique_points() {
        let mut points: Vec<i32> = MILESTONES.iter().map(|m| m.points).collect();
        let before = points.len();
        points.sort();
        points.dedup();
        assert_eq!(before, points.len(), "Duplicate milestone points found");
    }

    #[test]
    fn test_milestones_names_not_empty() {
        for m in MILESTONES.iter() {
            assert!(!m.name.is_empty(), "Milestone with {} points has empty name", m.points);
            assert!(!m.emoji.is_empty(), "Milestone {} has empty emoji", m.name);
        }
    }
}

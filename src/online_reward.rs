use crate::user;
/// CakeGame 在线奖励系统
/// 奖励玩家持续在线的时间，达到指定时长可领取奖励
use crate::Database;

/// 在线奖励定义
struct OnlineRewardDef {
    minutes: i32,
    name: &'static str,
    gold: i32,
    diamond: i32,
    item: &'static str,
    item_count: i32,
}

/// 获取在线奖励列表
fn get_online_rewards() -> Vec<OnlineRewardDef> {
    vec![
        OnlineRewardDef {
            minutes: 5,
            name: "5分钟礼包",
            gold: 100,
            diamond: 5,
            item: "【普通】生命药水",
            item_count: 2,
        },
        OnlineRewardDef {
            minutes: 15,
            name: "15分钟礼包",
            gold: 300,
            diamond: 10,
            item: "【普通】魔力药水",
            item_count: 2,
        },
        OnlineRewardDef {
            minutes: 30,
            name: "30分钟礼包",
            gold: 500,
            diamond: 20,
            item: "强化石",
            item_count: 1,
        },
        OnlineRewardDef {
            minutes: 60,
            name: "1小时礼包",
            gold: 1000,
            diamond: 50,
            item: "【普通】大生命药水",
            item_count: 3,
        },
        OnlineRewardDef {
            minutes: 120,
            name: "2小时礼包",
            gold: 2000,
            diamond: 100,
            item: "白色精粹",
            item_count: 2,
        },
        OnlineRewardDef {
            minutes: 180,
            name: "3小时礼包",
            gold: 3000,
            diamond: 150,
            item: "紫色精粹",
            item_count: 1,
        },
        OnlineRewardDef {
            minutes: 300,
            name: "5小时礼包",
            gold: 5000,
            diamond: 300,
            item: "史诗碎片",
            item_count: 1,
        },
    ]
}

/// 读取用户在线时间（分钟）
fn get_online_minutes(db: &Database, user_id: &str) -> i32 {
    db.read_user_data(user_id, "online_reward_minutes").parse().unwrap_or(0)
}

/// 写入用户在线时间
fn set_online_minutes(db: &Database, user_id: &str, mins: i32) {
    db.write_user_data(user_id, "online_reward_minutes", &mins.to_string());
}

/// 读取上次记录时间戳
fn get_last_record_ts(db: &Database, user_id: &str) -> i64 {
    db.read_user_data(user_id, "online_reward_last_ts").parse().unwrap_or(0)
}

/// 写入记录时间戳
fn set_last_record_ts(db: &Database, user_id: &str, ts: i64) {
    db.write_user_data(user_id, "online_reward_last_ts", &ts.to_string());
}

/// 读取已领取的奖励列表
fn get_claimed_rewards(db: &Database, user_id: &str) -> Vec<i32> {
    let data = db.read_user_data(user_id, "online_reward_claimed");
    if data.is_empty() {
        return vec![];
    }
    data.split(',').filter_map(|s| s.parse::<i32>().ok()).collect()
}

/// 标记奖励已领取
fn mark_claimed(db: &Database, user_id: &str, minutes: i32) {
    let mut claimed = get_claimed_rewards(db, user_id);
    if !claimed.contains(&minutes) {
        claimed.push(minutes);
        let data = claimed.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(",");
        db.write_user_data(user_id, "online_reward_claimed", &data);
    }
}

/// 获取当前时间戳（秒）
fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 更新在线时间（每次调用时累加）
fn update_online_time(db: &Database, user_id: &str) -> i32 {
    let last_ts = get_last_record_ts(db, user_id);
    let now = now_ts();

    if last_ts > 0 {
        let elapsed = now - last_ts;
        // 只累加5分钟内的间隔（超过5分钟认为中间下线了）
        if elapsed > 0 && elapsed <= 300 {
            let add_mins = (elapsed / 60) as i32;
            if add_mins > 0 {
                let current = get_online_minutes(db, user_id);
                set_online_minutes(db, user_id, current + add_mins);
            }
        }
    }
    set_last_record_ts(db, user_id, now);
    get_online_minutes(db, user_id)
}

/// 查看在线奖励
pub fn cmd_online_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 更新在线时间
    let total_mins = update_online_time(db, user_id);
    let claimed = get_claimed_rewards(db, user_id);

    let mut result = format!("{}\n═══ 🎁 在线奖励 ═══\n", prefix);
    result.push_str(&format!("⏰ 累计在线: {} 分钟\n\n", total_mins));

    let rewards = get_online_rewards();
    let mut available_count = 0;

    for r in &rewards {
        let is_claimed = claimed.contains(&r.minutes);
        let can_claim = total_mins >= r.minutes && !is_claimed;

        let status = if is_claimed {
            "✅已领取".to_string()
        } else if can_claim {
            available_count += 1;
            "🎁可领取".to_string()
        } else {
            let remaining = r.minutes - total_mins;
            format!("⏳还需{}分钟", remaining)
        };

        result.push_str(&format!(
            "  {} {} — {}金 {}钻 + [{}]×{}\n",
            if can_claim { "👉" } else { "  " },
            r.name,
            r.gold,
            r.diamond,
            r.item,
            r.item_count
        ));
        result.push_str(&format!("      状态: {}\n", status));
    }

    if available_count > 0 {
        result.push_str(&format!(
            "\n💡 发送「领取在线+礼包名」领取奖励 ({}个可领取)\n",
            available_count
        ));
    } else {
        result.push_str("\n💡 继续在线即可解锁更多奖励\n");
    }

    result
}

/// 领取在线奖励
pub fn cmd_claim_online_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();

    if name.is_empty() {
        return format!(
            "{}\n请指定要领取的礼包名！\n💡 发送「领取在线+礼包名」\n可选: 5分钟/15分钟/30分钟/1小时/2小时/3小时/5小时",
            prefix
        );
    }

    // 更新在线时间
    let total_mins = update_online_time(db, user_id);
    let claimed = get_claimed_rewards(db, user_id);

    let rewards = get_online_rewards();

    // 模糊匹配
    let matched = rewards.iter().find(|r| {
        r.name.contains(name)
            || (name.contains("5分") && r.minutes == 5)
            || (name.contains("15分") && r.minutes == 15)
            || (name.contains("30分") && r.minutes == 30)
            || (name.contains("1小时") && r.minutes == 60)
            || (name.contains("2小时") && r.minutes == 120)
            || (name.contains("3小时") && r.minutes == 180)
            || (name.contains("5小时") && r.minutes == 300)
    });

    let reward = match matched {
        Some(r) => r,
        None => {
            return format!(
                "{}\n未找到在线奖励「{}」！\n💡 可选: 5分钟/15分钟/30分钟/1小时/2小时/3小时/5小时",
                prefix, name
            )
        }
    };

    if claimed.contains(&reward.minutes) {
        return format!("{}\n{} 已经领取过了！", prefix, reward.name);
    }

    if total_mins < reward.minutes {
        return format!(
            "{}\n{} 需要在线 {} 分钟，当前仅在线 {} 分钟，还需 {} 分钟。",
            prefix,
            reward.name,
            reward.minutes,
            total_mins,
            reward.minutes - total_mins
        );
    }

    // 发放奖励
    mark_claimed(db, user_id, reward.minutes);

    // 金币
    if reward.gold > 0 {
        let cur_gold: i32 = db.read_basic(user_id, "Currency_gold").parse().unwrap_or(0);
        db.write_basic(user_id, "Currency_gold", &(cur_gold + reward.gold).to_string());
    }
    // 钻石
    if reward.diamond > 0 {
        let cur_diamond: i32 = db.read_basic(user_id, "Currency_diamond").parse().unwrap_or(0);
        db.write_basic(user_id, "Currency_diamond", &(cur_diamond + reward.diamond).to_string());
    }
    // 物品
    if !reward.item.is_empty() && reward.item_count > 0 {
        db.add_item(user_id, reward.item, reward.item_count);
    }

    format!(
        "{}\n🎉 成功领取 {}！\n获得: {}金币 + {}钻石 + [{}]×{}\n\n继续在线可领取更多奖励！",
        prefix, reward.name, reward.gold, reward.diamond, reward.item, reward.item_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_online_rewards_count() {
        let rewards = get_online_rewards();
        assert!(
            rewards.len() >= 4,
            "Expected at least 4 online rewards, got {}",
            rewards.len()
        );
    }

    #[test]
    fn test_online_rewards_sorted_by_minutes() {
        let rewards = get_online_rewards();
        for i in 1..rewards.len() {
            assert!(
                rewards[i].minutes > rewards[i - 1].minutes,
                "Reward {} minutes ({}) <= reward {} minutes ({})",
                i,
                rewards[i].minutes,
                i - 1,
                rewards[i - 1].minutes
            );
        }
    }

    #[test]
    fn test_online_rewards_positive_values() {
        let rewards = get_online_rewards();
        for r in &rewards {
            assert!(r.minutes > 0, "{}: minutes must be > 0", r.name);
            assert!(r.gold > 0, "{}: gold must be > 0", r.name);
            assert!(r.diamond >= 0, "{}: diamond must be >= 0", r.name);
        }
    }

    #[test]
    fn test_online_rewards_gold_increases() {
        let rewards = get_online_rewards();
        for i in 1..rewards.len() {
            assert!(
                rewards[i].gold >= rewards[i - 1].gold,
                "Gold at reward {} ({}) < reward {} ({})",
                i,
                rewards[i].gold,
                i - 1,
                rewards[i - 1].gold
            );
        }
    }

    #[test]
    fn test_online_rewards_unique_minutes() {
        let rewards = get_online_rewards();
        let mut minutes: Vec<i32> = rewards.iter().map(|r| r.minutes).collect();
        let before = minutes.len();
        minutes.sort();
        minutes.dedup();
        assert_eq!(before, minutes.len(), "Duplicate minute values found");
    }

    #[test]
    fn test_online_rewards_names_not_empty() {
        let rewards = get_online_rewards();
        for r in &rewards {
            assert!(!r.name.is_empty(), "Reward at {} minutes has empty name", r.minutes);
        }
    }
}

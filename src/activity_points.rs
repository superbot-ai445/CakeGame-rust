/// CakeGame 每日活跃度积分系统
/// 玩家通过各种日常行为积累活跃度积分，达到里程碑可领取丰厚奖励
/// 每日0点重置，鼓励多样化游戏行为
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 活跃度活动定义
struct ActivityDef {
    key: &'static str,
    name: &'static str,
    emoji: &'static str,
    points: i32,
    daily_max: i32, // 每日上限次数 (0 = 无限)
    description: &'static str,
}

/// 所有活跃度活动
const ACTIVITIES: &[ActivityDef] = &[
    ActivityDef {
        key: "sign",
        name: "每日签到",
        emoji: "📅",
        points: 10,
        daily_max: 1,
        description: "签到获得10活跃度",
    },
    ActivityDef {
        key: "kill",
        name: "击杀怪物",
        emoji: "⚔️",
        points: 2,
        daily_max: 15,
        description: "每次击杀获得2活跃度",
    },
    ActivityDef {
        key: "quest",
        name: "完成任务",
        emoji: "📜",
        points: 15,
        daily_max: 5,
        description: "每完成1个任务获得15活跃度",
    },
    ActivityDef {
        key: "gather",
        name: "采集资源",
        emoji: "🪓",
        points: 3,
        daily_max: 10,
        description: "每次采集获得3活跃度",
    },
    ActivityDef {
        key: "skill",
        name: "释放技能",
        emoji: "✨",
        points: 2,
        daily_max: 15,
        description: "每次释放技能获得2活跃度",
    },
    ActivityDef {
        key: "guild",
        name: "公会操作",
        emoji: "🏰",
        points: 8,
        daily_max: 3,
        description: "公会捐献/签到/委托获得8活跃度",
    },
    ActivityDef {
        key: "shop",
        name: "交易操作",
        emoji: "💰",
        points: 5,
        daily_max: 5,
        description: "购买/出售物品获得5活跃度",
    },
    ActivityDef {
        key: "arena",
        name: "竞技匹配",
        emoji: "🏟️",
        points: 10,
        daily_max: 5,
        description: "每次匹配竞技获得10活跃度",
    },
    ActivityDef {
        key: "boss",
        name: "挑战BOSS",
        emoji: "🐉",
        points: 12,
        daily_max: 5,
        description: "挑战BOSS获得12活跃度",
    },
    ActivityDef {
        key: "dungeon",
        name: "副本通关",
        emoji: "🗝️",
        points: 15,
        daily_max: 3,
        description: "每次副本通关获得15活跃度",
    },
    ActivityDef {
        key: "compose",
        name: "合成物品",
        emoji: "🔨",
        points: 5,
        daily_max: 5,
        description: "合成/锻造/炼制获得5活跃度",
    },
    ActivityDef {
        key: "enhance",
        name: "强化装备",
        emoji: "💎",
        points: 8,
        daily_max: 3,
        description: "强化/超界强化获得8活跃度",
    },
    ActivityDef {
        key: "gift",
        name: "赠送物品",
        emoji: "🎁",
        points: 5,
        daily_max: 5,
        description: "赠送物品/金币/钻石获得5活跃度",
    },
    ActivityDef {
        key: "pray",
        name: "每日祈福",
        emoji: "🙏",
        points: 5,
        daily_max: 1,
        description: "祈福获得5活跃度",
    },
    ActivityDef {
        key: "weather",
        name: "查看天气",
        emoji: "🌤️",
        points: 2,
        daily_max: 1,
        description: "查看天气获得2活跃度",
    },
    ActivityDef {
        key: "train",
        name: "修炼训练",
        emoji: "🧘",
        points: 8,
        daily_max: 3,
        description: "吐纳/冥想/练武/习法获得8活跃度",
    },
];

/// 活跃度里程碑奖励
struct MilestoneDef {
    points: i32,
    name: &'static str,
    emoji: &'static str,
    reward_gold: i32,
    reward_diamond: i32,
    reward_exp: i32,
    reward_item: &'static str, // 物品名 (空=无)
    description: &'static str,
}

const MILESTONES: &[MilestoneDef] = &[
    MilestoneDef {
        points: 30,
        name: "初级活跃",
        emoji: "🌱",
        reward_gold: 500,
        reward_diamond: 5,
        reward_exp: 200,
        reward_item: "",
        description: "达成30活跃度",
    },
    MilestoneDef {
        points: 60,
        name: "日常达人",
        emoji: "🌿",
        reward_gold: 1000,
        reward_diamond: 10,
        reward_exp: 500,
        reward_item: "初级药水",
        description: "达成60活跃度",
    },
    MilestoneDef {
        points: 100,
        name: "活跃先锋",
        emoji: "🌳",
        reward_gold: 2000,
        reward_diamond: 20,
        reward_exp: 1000,
        reward_item: "强化石",
        description: "达成100活跃度",
    },
    MilestoneDef {
        points: 150,
        name: "精力充沛",
        emoji: "🌟",
        reward_gold: 5000,
        reward_diamond: 30,
        reward_exp: 2000,
        reward_item: "高级药水",
        description: "达成150活跃度",
    },
    MilestoneDef {
        points: 200,
        name: "满勤王者",
        emoji: "👑",
        reward_gold: 10000,
        reward_diamond: 50,
        reward_exp: 5000,
        reward_item: "超级礼包",
        description: "达成200活跃度（满分）",
    },
];

const SECTION: &str = "activity_points";

/// 获取今日日期字符串
fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// 获取用户活跃度数据段名
fn user_section(user_id: &str) -> String {
    format!("{}_{}", SECTION, user_id)
}

/// 读取今日活跃度总分
fn get_today_points(db: &Database, user_id: &str) -> i32 {
    let date = db.global_get(&user_section(user_id), "date");
    if date != today_str() {
        return 0;
    }
    db.global_get(&user_section(user_id), "total_points")
        .parse()
        .unwrap_or(0)
}

/// 读取今日某活动完成次数
fn get_activity_count(db: &Database, user_id: &str, key: &str) -> i32 {
    let date = db.global_get(&user_section(user_id), "date");
    if date != today_str() {
        return 0;
    }
    db.global_get(&user_section(user_id), &format!("cnt_{}", key))
        .parse()
        .unwrap_or(0)
}

/// 读取今日已领取的里程碑列表
fn get_claimed_milestones(db: &Database, user_id: &str) -> Vec<i32> {
    let date = db.global_get(&user_section(user_id), "date");
    if date != today_str() {
        return Vec::new();
    }
    let raw = db.global_get(&user_section(user_id), "claimed");
    raw.split('|')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}

/// 保存已领取里程碑
fn save_claimed_milestones(db: &Database, user_id: &str, claimed: &[i32]) {
    let s = claimed.iter().map(|c| c.to_string()).collect::<Vec<_>>().join("|");
    db.global_set(&user_section(user_id), "claimed", &s);
}

/// 记录活跃度（外部API，供其他模块调用）
/// key: 活动类型key，amount: 完成次数（通常为1）
/// 返回实际获得的活跃度积分
#[allow(dead_code)] // 集成到combat/quest/gather等模块后移除
pub fn record_activity(db: &Database, user_id: &str, key: &str, amount: i32) -> i32 {
    let def = match ACTIVITIES.iter().find(|a| a.key == key) {
        Some(d) => d,
        None => return 0,
    };

    // 重置过期数据
    let section = user_section(user_id);
    let stored_date = db.global_get(&section, "date");
    if stored_date != today_str() {
        // 新的一天，重置所有计数
        db.global_set(&section, "date", &today_str());
        db.global_set(&section, "total_points", "0");
        db.global_set(&section, "claimed", "");
        for a in ACTIVITIES {
            db.global_set(&section, &format!("cnt_{}", a.key), "0");
        }
    }

    let current_count = get_activity_count(db, user_id, key);
    let current_points = get_today_points(db, user_id);

    // 计算实际可记录的次数
    let actual = if def.daily_max > 0 {
        let remaining = def.daily_max - current_count;
        if remaining <= 0 {
            return 0;
        }
        amount.min(remaining)
    } else {
        amount
    };

    let earned = actual * def.points;
    db.global_set(&section, &format!("cnt_{}", key), &(current_count + actual).to_string());
    db.global_set(&section, "total_points", &(current_points + earned).to_string());

    earned
}

/// 查看活跃度
pub fn cmd_view_activity(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let total = get_today_points(db, user_id);
    let claimed = get_claimed_milestones(db, user_id);

    let mut out = format!("{}\n═══ 📊 今日活跃度 ═══", prefix);
    out.push_str(&format!("\n📅 日期: {}", today_str()));
    out.push_str(&format!("\n🏆 今日活跃度: {} 点\n", total));

    // 活跃度进度条 (到下一个里程碑)
    let next_ms = MILESTONES
        .iter()
        .find(|m| !claimed.contains(&m.points) && total < m.points);
    if let Some(ms) = next_ms {
        let bar_len = 20;
        let filled = ((total as f64 / ms.points as f64) * bar_len as f64).min(bar_len as f64) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));
        out.push_str(&format!(
            "\n📊 下一里程碑: {} {} ({}点)\n  [{}] {}/{}",
            ms.emoji, ms.name, ms.points, bar, total, ms.points
        ));
    } else {
        out.push_str("\n🎉 所有里程碑已达成！");
    }

    // 里程碑状态
    out.push_str("\n\n═══ 🎁 里程碑奖励 ═══");
    for ms in MILESTONES {
        let status = if claimed.contains(&ms.points) {
            "✅已领取"
        } else if total >= ms.points {
            "🔔可领取"
        } else {
            "🔒未达成"
        };
        out.push_str(&format!(
            "\n{} {} {}点 — {} {}",
            ms.emoji, ms.name, ms.points, status, ms.description
        ));
        let mut rewards = Vec::new();
        if ms.reward_gold > 0 {
            rewards.push(format!("💰{}金", ms.reward_gold));
        }
        if ms.reward_diamond > 0 {
            rewards.push(format!("💎{}钻", ms.reward_diamond));
        }
        if ms.reward_exp > 0 {
            rewards.push(format!("⭐{}经验", ms.reward_exp));
        }
        if !ms.reward_item.is_empty() {
            rewards.push(format!("📦{}", ms.reward_item));
        }
        out.push_str(&format!("\n    奖励: {}", rewards.join(" / ")));
    }

    // 各活动今日完成情况
    out.push_str("\n\n═══ 📋 活动明细 ═══");
    for a in ACTIVITIES {
        let count = get_activity_count(db, user_id, a.key);
        let max_str = if a.daily_max > 0 {
            format!("{}/{}", count, a.daily_max)
        } else {
            format!("{}", count)
        };
        let earned = count * a.points;
        out.push_str(&format!(
            "\n{} {} — {}次 ({}点) {}",
            a.emoji, a.name, max_str, earned, a.description
        ));
    }

    // 计算今日理论最大值
    let max_possible: i32 = ACTIVITIES
        .iter()
        .map(|a| if a.daily_max > 0 { a.daily_max * a.points } else { 0 })
        .sum();
    out.push_str(&format!("\n\n📊 今日理论最大: {} 点 (完成所有活动)", max_possible));

    out
}

/// 领取活跃度奖励
pub fn cmd_claim_activity_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let total = get_today_points(db, user_id);
    let mut claimed = get_claimed_milestones(db, user_id);

    // 解析要领取的里程碑
    let target = if args.is_empty() {
        // 默认领取所有可领取的
        let mut all_rewards: Vec<(String, Vec<String>)> = Vec::new();
        for ms in MILESTONES {
            if total >= ms.points && !claimed.contains(&ms.points) {
                claimed.push(ms.points);
                let rewards = give_milestone_reward(db, user_id, ms);
                all_rewards.push((ms.name.to_string(), rewards));
            }
        }
        if all_rewards.is_empty() {
            return format!(
                "{}\n❌ 没有可领取的里程碑奖励！\n当前活跃度: {}点\n发送【活跃度】查看详情",
                prefix, total
            );
        }
        save_claimed_milestones(db, user_id, &claimed);

        let mut out = format!("{}\n🎉 批量领取活跃度奖励成功！", prefix);
        out.push_str(&format!("\n当前活跃度: {}点", total));
        out.push_str("\n\n═══ 📦 领取记录 ═══");
        for (name, rewards) in &all_rewards {
            out.push_str(&format!("\n✅ {}", name));
            for r in rewards {
                out.push_str(&format!("\n  {}", r));
            }
        }
        return out;
    } else {
        // 指定里程碑
        let target_points: i32 = match args.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                // 按名称查找
                if let Some(ms) = MILESTONES
                    .iter()
                    .find(|m| m.name.contains(args.trim()) || args.trim().contains(m.name))
                {
                    ms.points
                } else {
                    return format!(
                        "{}\n❌ 未找到里程碑 \"{}\"\n可选: {}",
                        prefix,
                        args.trim(),
                        MILESTONES
                            .iter()
                            .map(|m| format!("{}({}点)", m.name, m.points))
                            .collect::<Vec<_>>()
                            .join(" / ")
                    );
                }
            }
        };
        target_points
    };

    let ms = match MILESTONES.iter().find(|m| m.points == target) {
        Some(m) => m,
        None => {
            return format!("{}\n❌ 未找到{}点里程碑", prefix, target);
        }
    };

    if claimed.contains(&ms.points) {
        return format!("{}\n❌ {}({}点) 奖励已领取过！", prefix, ms.name, ms.points);
    }
    if total < ms.points {
        return format!("{}\n❌ 活跃度不足！需要{}点，当前{}点", prefix, ms.points, total);
    }

    claimed.push(ms.points);
    save_claimed_milestones(db, user_id, &claimed);

    let rewards = give_milestone_reward(db, user_id, ms);
    let mut out = format!("{}\n🎉 领取 {}({}) 奖励成功！", prefix, ms.name, ms.emoji);
    out.push_str(&format!("\n活跃度: {}点", total));
    out.push_str("\n\n📦 获得奖励:");
    for r in &rewards {
        out.push_str(&format!("\n  {}", r));
    }
    out
}

/// 发放里程碑奖励
fn give_milestone_reward(db: &Database, user_id: &str, ms: &MilestoneDef) -> Vec<String> {
    let mut rewards = Vec::new();

    if ms.reward_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", ms.reward_gold as i64);
        rewards.push(format!("💰 {} 金币", ms.reward_gold));
    }
    if ms.reward_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", ms.reward_diamond as i64);
        rewards.push(format!("💎 {} 钻石", ms.reward_diamond));
    }
    if ms.reward_exp > 0 {
        user::add_experience(db, user_id, ms.reward_exp);
        rewards.push(format!("⭐ {} 经验", ms.reward_exp));
    }
    if !ms.reward_item.is_empty() {
        db.add_item(user_id, ms.reward_item, 1);
        rewards.push(format!("📦 {}", ms.reward_item));
    }

    rewards
}

/// 活跃度排行
pub fn cmd_activity_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let today = today_str();

    // 第一步：收集所有有活跃度记录的用户 section
    let sections: Vec<String> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare(&format!(
            "SELECT DISTINCT SECTION FROM Global WHERE SECTION LIKE '{SECTION}_%'"
        )) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    result.push(row);
                }
            }
        }
        result
    };

    // 第二步：逐个用户查询总分和日期
    let mut entries: Vec<(String, i32)> = Vec::new();
    for section in &sections {
        let uid = section
            .strip_prefix(&format!("{}_", SECTION))
            .unwrap_or(section)
            .to_string();
        let date = db.global_get(section, "date");
        if date != today {
            continue;
        }
        let points: i32 = db.global_get(section, "total_points").parse().unwrap_or(0);
        if points > 0 {
            entries.push((uid, points));
        }
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.1));
    entries.truncate(15);

    let mut out = format!("{}\n═══ 🏆 今日活跃度排行 ═══", prefix);
    out.push_str(&format!("\n📅 {}", today));

    if entries.is_empty() {
        out.push_str("\n\n暂无活跃度数据");
        return out;
    }

    for (i, (uid, points)) in entries.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let name = db.read_basic(uid, ITEM_NAME);
        let display = if *uid == user_id {
            format!("{} ← 您", name)
        } else {
            name
        };
        out.push_str(&format!("\n{} #{} {} - {}点", medal, i + 1, display, points));
    }

    let my_points = get_today_points(db, user_id);
    if !entries.iter().any(|(uid, _)| uid == user_id) && my_points > 0 {
        out.push_str(&format!("\n\n📊 您的活跃度: {}点 (未进入前15)", my_points));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activities_count() {
        assert_eq!(ACTIVITIES.len(), 16);
    }

    #[test]
    fn test_activity_keys_unique() {
        let mut keys: Vec<&str> = ACTIVITIES.iter().map(|a| a.key).collect();
        let len_before = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), len_before, "Activity keys must be unique");
    }

    #[test]
    fn test_activity_names_unique() {
        let mut names: Vec<&str> = ACTIVITIES.iter().map(|a| a.name).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before, "Activity names must be unique");
    }

    #[test]
    fn test_activity_points_positive() {
        for a in ACTIVITIES {
            assert!(a.points > 0, "Activity '{}' must have positive points", a.key);
        }
    }

    #[test]
    fn test_activity_emojis_non_empty() {
        for a in ACTIVITIES {
            assert!(!a.emoji.is_empty(), "Activity '{}' must have emoji", a.key);
        }
    }

    #[test]
    fn test_milestones_count() {
        assert_eq!(MILESTONES.len(), 5);
    }

    #[test]
    fn test_milestones_sorted_by_points() {
        for i in 1..MILESTONES.len() {
            assert!(
                MILESTONES[i].points > MILESTONES[i - 1].points,
                "Milestones must be sorted by points"
            );
        }
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for ms in MILESTONES {
            assert!(
                ms.reward_gold > 0 || ms.reward_diamond > 0 || ms.reward_exp > 0,
                "Milestone {} must have at least one reward",
                ms.name
            );
        }
    }

    #[test]
    fn test_milestone_rewards_escalate() {
        for i in 1..MILESTONES.len() {
            let prev_total =
                MILESTONES[i - 1].reward_gold + MILESTONES[i - 1].reward_diamond + MILESTONES[i - 1].reward_exp;
            let curr_total = MILESTONES[i].reward_gold + MILESTONES[i].reward_diamond + MILESTONES[i].reward_exp;
            assert!(
                curr_total > prev_total,
                "Milestone {} rewards should be greater than {}",
                MILESTONES[i].name,
                MILESTONES[i - 1].name
            );
        }
    }

    #[test]
    fn test_max_daily_points() {
        let max: i32 = ACTIVITIES
            .iter()
            .map(|a| if a.daily_max > 0 { a.daily_max * a.points } else { 0 })
            .sum();
        // 最大活跃度应 >= 200（满勤王者里程碑）
        assert!(max >= 200, "Max daily points {} should be >= 200", max);
    }

    #[test]
    fn test_milestone_points_within_max() {
        let max: i32 = ACTIVITIES
            .iter()
            .map(|a| if a.daily_max > 0 { a.daily_max * a.points } else { 0 })
            .sum();
        for ms in MILESTONES {
            assert!(
                ms.points <= max,
                "Milestone {} ({}pts) should be achievable within max daily points ({})",
                ms.name,
                ms.points,
                max
            );
        }
    }

    #[test]
    fn test_find_activity_by_key() {
        assert!(ACTIVITIES.iter().any(|a| a.key == "sign"));
        assert!(ACTIVITIES.iter().any(|a| a.key == "kill"));
        assert!(ACTIVITIES.iter().any(|a| a.key == "boss"));
        assert!(!ACTIVITIES.iter().any(|a| a.key == "nonexistent"));
    }

    #[test]
    fn test_milestone_emojis() {
        for ms in MILESTONES {
            assert!(!ms.emoji.is_empty(), "Milestone {} must have emoji", ms.name);
        }
    }

    #[test]
    fn test_today_str_format() {
        let d = today_str();
        assert_eq!(d.len(), 10); // YYYY-MM-DD
        assert!(d.contains('-'));
    }
}

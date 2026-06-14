/// CakeGame PvP 连胜系统
///
/// 追踪玩家 PvP 连胜记录，提供连胜奖励和全服排行。
/// 连胜越高，奖励越丰厚。被击败时连胜中断并记录历史最佳。
///
/// 连胜里程碑:
///   3连胜 → 初露锋芒 (500金+10💎)
///   5连胜 → 势不可挡 (2000金+30💎+强化石)
///  10连胜 → 连胜之王 (5000金+80💎+高级药水)
///  20连胜 → 战神降临 (15000金+200💎+凤凰之羽)
///  50连胜 → 不败传说 (50000金+500💎+传说宝箱)
/// 100连胜 → 至尊战神 (200000金+2000💎+至尊宝箱)
///
/// 数据存储: Global 表 SECTION='pvp_streak'
///   - current:{uid}     → 当前连胜数
///   - best:{uid}        → 历史最佳连胜
///   - reward:{uid}      → 已领取的里程碑位掩码
///   - total_wins:{uid}  → 累计 PvP 胜利次数
///   - total_losses:{uid} → 累计 PvP 失败次数
///   - last_win:{uid}    → 最后胜利时间
///   - streak_start:{uid} → 当前连胜开始时间
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD};
use crate::db::Database;
use crate::user;

const SECTION: &str = "pvp_streak";

/// 连胜里程碑定义
struct StreakMilestone {
    count: i32,
    name: &'static str,
    emoji: &'static str,
    reward_gold: i64,
    reward_diamond: i64,
    reward_item: &'static str,
    /// 位掩码中的位 (1 << index)
    bit: i64,
}

const MILESTONES: &[StreakMilestone] = &[
    StreakMilestone {
        count: 3,
        name: "初露锋芒",
        emoji: "⚡",
        reward_gold: 500,
        reward_diamond: 10,
        reward_item: "",
        bit: 1,
    },
    StreakMilestone {
        count: 5,
        name: "势不可挡",
        emoji: "🔥",
        reward_gold: 2000,
        reward_diamond: 30,
        reward_item: "强化石",
        bit: 2,
    },
    StreakMilestone {
        count: 10,
        name: "连胜之王",
        emoji: "👑",
        reward_gold: 5000,
        reward_diamond: 80,
        reward_item: "高级药水",
        bit: 4,
    },
    StreakMilestone {
        count: 20,
        name: "战神降临",
        emoji: "⚔️",
        reward_gold: 15000,
        reward_diamond: 200,
        reward_item: "凤凰之羽",
        bit: 8,
    },
    StreakMilestone {
        count: 50,
        name: "不败传说",
        emoji: "🌟",
        reward_gold: 50000,
        reward_diamond: 500,
        reward_item: "传说宝箱",
        bit: 16,
    },
    StreakMilestone {
        count: 100,
        name: "至尊战神",
        emoji: "⚜️",
        reward_gold: 200000,
        reward_diamond: 2000,
        reward_item: "至尊宝箱",
        bit: 32,
    },
];

/// 连胜等级名称（用于显示当前连胜等级）
fn streak_tier(current: i32) -> (&'static str, &'static str) {
    if current >= 100 {
        ("至尊战神", "⚜️")
    } else if current >= 50 {
        ("不败传说", "🌟")
    } else if current >= 20 {
        ("战神降临", "⚔️")
    } else if current >= 10 {
        ("连胜之王", "👑")
    } else if current >= 5 {
        ("势不可挡", "🔥")
    } else if current >= 3 {
        ("初露锋芒", "⚡")
    } else {
        ("新秀", "🔰")
    }
}

/// 记录 PvP 胜利（由 pvp.rs / duel.rs / match 系统调用）
#[allow(dead_code)]
pub fn record_win(db: &Database, user_id: &str) {
    let current: i32 = db
        .global_get(SECTION, &format!("current:{}", user_id))
        .parse()
        .unwrap_or(0);
    let new_streak = current + 1;
    db.global_set(SECTION, &format!("current:{}", user_id), &new_streak.to_string());

    // 更新历史最佳
    let best: i32 = db
        .global_get(SECTION, &format!("best:{}", user_id))
        .parse()
        .unwrap_or(0);
    if new_streak > best {
        db.global_set(SECTION, &format!("best:{}", user_id), &new_streak.to_string());
    }

    // 累计胜场
    let total_wins: i32 = db
        .global_get(SECTION, &format!("total_wins:{}", user_id))
        .parse()
        .unwrap_or(0);
    db.global_set(
        SECTION,
        &format!("total_wins:{}", user_id),
        &(total_wins + 1).to_string(),
    );

    // 记录时间
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.global_set(SECTION, &format!("last_win:{}", user_id), &now);

    // 连胜开始时间（首次胜利时记录）
    if current == 0 {
        db.global_set(SECTION, &format!("streak_start:{}", user_id), &now);
    }
}

/// 记录 PvP 失败（连胜中断）
#[allow(dead_code)]
pub fn record_loss(db: &Database, user_id: &str) {
    let current: i32 = db
        .global_get(SECTION, &format!("current:{}", user_id))
        .parse()
        .unwrap_or(0);

    // 更新历史最佳（失败前的连胜也可能创新高）
    let best: i32 = db
        .global_get(SECTION, &format!("best:{}", user_id))
        .parse()
        .unwrap_or(0);
    if current > best {
        db.global_set(SECTION, &format!("best:{}", user_id), &current.to_string());
    }

    // 重置当前连胜
    db.global_set(SECTION, &format!("current:{}", user_id), "0");

    // 累计败场
    let total_losses: i32 = db
        .global_get(SECTION, &format!("total_losses:{}", user_id))
        .parse()
        .unwrap_or(0);
    db.global_set(
        SECTION,
        &format!("total_losses:{}", user_id),
        &(total_losses + 1).to_string(),
    );
}

/// 查看当前连胜状态
pub fn cmd_view_streak(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current: i32 = db
        .global_get(SECTION, &format!("current:{}", user_id))
        .parse()
        .unwrap_or(0);
    let best: i32 = db
        .global_get(SECTION, &format!("best:{}", user_id))
        .parse()
        .unwrap_or(0);
    let total_wins: i32 = db
        .global_get(SECTION, &format!("total_wins:{}", user_id))
        .parse()
        .unwrap_or(0);
    let total_losses: i32 = db
        .global_get(SECTION, &format!("total_losses:{}", user_id))
        .parse()
        .unwrap_or(0);
    let last_win = db.global_get(SECTION, &format!("last_win:{}", user_id));
    let streak_start = db.global_get(SECTION, &format!("streak_start:{}", user_id));

    let (tier_name, tier_emoji) = streak_tier(current);
    let win_rate = if total_wins + total_losses > 0 {
        total_wins * 100 / (total_wins + total_losses)
    } else {
        0
    };

    // 连胜进度条
    let next_milestone = MILESTONES.iter().find(|m| m.count > current);
    let progress = if let Some(next) = next_milestone {
        let pct = (current * 100 / next.count).min(100);
        let filled = ((pct / 5).min(20)) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
        format!(
            "\n🎯 下一里程碑: {} ({}) {} {}/{}",
            next.name, next.count, bar, current, next.count
        )
    } else {
        "\n🏆 已达成所有里程碑！".to_string()
    };

    let mut r = format!(
        "{}\n═══ ⚔️ PvP 连胜 ═══\n\n\
         {} {} 当前连胜: {} 连胜\n\
         🏅 历史最佳: {} 连胜\n\
         📊 胜率: {}% ({}胜{}负)\n\
         {}",
        prefix, tier_emoji, tier_name, current, best, win_rate, total_wins, total_losses, progress
    );

    if !streak_start.is_empty() && current > 0 {
        r.push_str(&format!("\n⏰ 连胜开始: {}", streak_start));
    }
    if !last_win.is_empty() {
        r.push_str(&format!("\n🕐 最近胜利: {}", last_win));
    }

    // 可领取的里程碑
    let claimed: i64 = db
        .global_get(SECTION, &format!("reward:{}", user_id))
        .parse()
        .unwrap_or(0);
    let claimable: Vec<&StreakMilestone> = MILESTONES
        .iter()
        .filter(|m| current >= m.count && (claimed & m.bit) == 0)
        .collect();
    if !claimable.is_empty() {
        r.push_str("\n\n🎁 可领取里程碑奖励:");
        for m in &claimable {
            r.push_str(&format!("\n  {} {} → 领取连胜奖励+{}", m.emoji, m.name, m.count));
        }
    }

    r
}

/// 查看连胜里程碑奖励列表
pub fn cmd_streak_milestones(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current: i32 = db
        .global_get(SECTION, &format!("current:{}", user_id))
        .parse()
        .unwrap_or(0);
    let claimed: i64 = db
        .global_get(SECTION, &format!("reward:{}", user_id))
        .parse()
        .unwrap_or(0);

    let mut r = format!("{}\n═══ 🎯 连胜里程碑 ═══\n", prefix);

    for m in MILESTONES {
        let status = if (claimed & m.bit) != 0 {
            "✅ 已领取"
        } else if current >= m.count {
            "🎁 可领取"
        } else {
            "🔒 未达成"
        };
        r.push_str(&format!(
            "\n{} {} ({}连胜) — {}\n  奖励: {}金+{}💎",
            m.emoji, m.name, m.count, status, m.reward_gold, m.reward_diamond
        ));
        if !m.reward_item.is_empty() {
            r.push_str(&format!("+{}", m.reward_item));
        }
    }

    r
}

/// 领取连胜里程碑奖励
pub fn cmd_claim_streak_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current: i32 = db
        .global_get(SECTION, &format!("current:{}", user_id))
        .parse()
        .unwrap_or(0);
    let claimed: i64 = db
        .global_get(SECTION, &format!("reward:{}", user_id))
        .parse()
        .unwrap_or(0);

    // 解析目标里程碑
    let target_count: i32 = if args.trim().is_empty() {
        // 默认领取所有可领取的
        0
    } else {
        match args.trim().parse::<i32>() {
            Ok(n) => n,
            Err(_) => return format!("{}\n❌ 请输入里程碑连胜数，如：领取连胜奖励+3", prefix),
        }
    };

    let mut total_gold: i64 = 0;
    let mut total_diamond: i64 = 0;
    let mut claimed_items: Vec<String> = Vec::new();
    let mut new_claimed = claimed;

    for m in MILESTONES {
        if current < m.count {
            continue;
        }
        if (claimed & m.bit) != 0 {
            continue;
        }
        if target_count > 0 && m.count != target_count {
            continue;
        }
        // 领取奖励
        total_gold += m.reward_gold;
        total_diamond += m.reward_diamond;
        new_claimed |= m.bit;
        claimed_items.push(format!("{}{}", m.emoji, m.name));
        if !m.reward_item.is_empty() {
            let _ = db.knapsack_add(user_id, m.reward_item, 1);
        }
    }

    if claimed_items.is_empty() {
        if target_count > 0 {
            return format!(
                "{}\n❌ 无法领取连胜{}的奖励\n💡 当前连胜: {}，检查是否已领取或未达成",
                prefix, target_count, current
            );
        }
        return format!("{}\n❌ 没有可领取的连胜里程碑奖励\n💡 当前连胜: {}", prefix, current);
    }

    // 发放金币和钻石
    if total_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", total_gold);
    }
    if total_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", total_diamond);
    }

    // 保存领取记录
    db.global_set(SECTION, &format!("reward:{}", user_id), &new_claimed.to_string());

    let mut r = format!("{}\n🎉 连胜里程碑奖励领取成功！\n", prefix);
    r.push_str(&format!("\n📦 领取的里程碑: {}", claimed_items.join(" / ")));
    r.push_str(&format!("\n💰 获得: {}金 + {}💎", total_gold, total_diamond));

    r
}

/// 连胜排行榜
pub fn cmd_streak_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 收集所有用户ID
    let all_uids = db.all_users();

    let mut players: Vec<(String, i32, i32, i32, i32)> = Vec::new(); // (name, current, best, wins, losses)

    for uid in &all_uids {
        let current: i32 = db.global_get(SECTION, &format!("current:{}", uid)).parse().unwrap_or(0);
        let best: i32 = db.global_get(SECTION, &format!("best:{}", uid)).parse().unwrap_or(0);
        let wins: i32 = db
            .global_get(SECTION, &format!("total_wins:{}", uid))
            .parse()
            .unwrap_or(0);
        let losses: i32 = db
            .global_get(SECTION, &format!("total_losses:{}", uid))
            .parse()
            .unwrap_or(0);
        if wins > 0 || losses > 0 || current > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, current, best, wins, losses));
        }
    }

    if players.is_empty() {
        return format!("{}\n暂无 PvP 连胜数据\n💡 进行 PvP 战斗后自动记录", prefix);
    }

    // 按历史最佳连胜降序排列
    players.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut r = format!("{}\n═══ 🏆 连胜排行榜 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];

    for (i, (name, current, best, wins, losses)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let (tier, emoji) = streak_tier(*best);
        let wr = if wins + losses > 0 {
            wins * 100 / (wins + losses)
        } else {
            0
        };
        r.push_str(&format!(
            "\n{} {} {} — 当前:{}连 最佳:{}连 胜率:{}% ({}W{}L)",
            medal, emoji, name, current, best, wr, wins, losses
        ));
        if i < 3 {
            r.push_str(&format!(" [{}]", tier));
        }
    }

    // 用户排名
    let user_best: i32 = db
        .global_get(SECTION, &format!("best:{}", user_id))
        .parse()
        .unwrap_or(0);
    if let Some(rank) = players
        .iter()
        .position(|(_, _, best, _, _)| *best == user_best && user_best > 0)
    {
        r.push_str(&format!("\n\n📍 你的排名: 第{}名 (最佳连胜: {})", rank + 1, user_best));
    }

    r
}

/// 连胜系统帮助信息
pub fn cmd_streak_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n\
         ═══ ⚔️ PvP 连胜系统 ═══\n\
         \n\
         🎯 系统说明:\n\
         进行 PvP 战斗（攻击玩家/决斗/匹配）胜利后自动累积连胜。\n\
         被击败时连胜中断，历史最佳记录永久保留。\n\
         \n\
         📋 指令:\n\
         1. 查看连胜 — 查看当前连胜状态\n\
         2. 连胜奖励 — 查看里程碑奖励列表\n\
         3. 领取连胜奖励+N — 领取指定里程碑奖励\n\
         4. 连胜排行 — 全服连胜排行榜\n\
         5. 连胜帮助 — 查看帮助信息\n\
         \n\
         🏆 里程碑:\n\
         3连→初露锋芒 | 5连→势不可挡 | 10连→连胜之王\n\
         20连→战神降临 | 50连→不败传说 | 100连→至尊战神\n\
         \n\
         💡 Tips:\n\
         • 每次里程碑只能领取一次奖励\n\
         • 连胜越高奖励越丰厚\n\
         • 即使连胜中断，已领取的奖励不会回收",
        prefix
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milestones_count() {
        assert_eq!(MILESTONES.len(), 6);
    }

    #[test]
    fn test_milestones_sorted_by_count() {
        for i in 1..MILESTONES.len() {
            assert!(
                MILESTONES[i].count > MILESTONES[i - 1].count,
                "Milestones must be sorted by count"
            );
        }
    }

    #[test]
    fn test_milestones_unique_bits() {
        let mut bits: Vec<i64> = MILESTONES.iter().map(|m| m.bit).collect();
        let before = bits.len();
        bits.sort();
        bits.dedup();
        assert_eq!(before, bits.len(), "Milestone bits must be unique");
    }

    #[test]
    fn test_milestones_bits_are_powers_of_two() {
        for m in MILESTONES {
            assert!(
                m.bit > 0 && (m.bit & (m.bit - 1)) == 0,
                "Bit {} for milestone {} is not a power of 2",
                m.bit,
                m.name
            );
        }
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for m in MILESTONES {
            assert!(m.reward_gold > 0, "Gold reward must be positive for {}", m.name);
            assert!(m.reward_diamond > 0, "Diamond reward must be positive for {}", m.name);
        }
    }

    #[test]
    fn test_milestone_rewards_escalate() {
        for i in 1..MILESTONES.len() {
            assert!(
                MILESTONES[i].reward_gold >= MILESTONES[i - 1].reward_gold,
                "Gold rewards must escalate"
            );
            assert!(
                MILESTONES[i].reward_diamond >= MILESTONES[i - 1].reward_diamond,
                "Diamond rewards must escalate"
            );
        }
    }

    #[test]
    fn test_streak_tier_boundaries() {
        assert_eq!(streak_tier(0), ("新秀", "🔰"));
        assert_eq!(streak_tier(2), ("新秀", "🔰"));
        assert_eq!(streak_tier(3), ("初露锋芒", "⚡"));
        assert_eq!(streak_tier(5), ("势不可挡", "🔥"));
        assert_eq!(streak_tier(10), ("连胜之王", "👑"));
        assert_eq!(streak_tier(20), ("战神降临", "⚔️"));
        assert_eq!(streak_tier(50), ("不败传说", "🌟"));
        assert_eq!(streak_tier(100), ("至尊战神", "⚜️"));
        assert_eq!(streak_tier(999), ("至尊战神", "⚜️"));
    }

    #[test]
    fn test_milestone_names_unique() {
        let mut names: Vec<&str> = MILESTONES.iter().map(|m| m.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len(), "Milestone names must be unique");
    }

    #[test]
    fn test_milestone_counts_valid() {
        for m in MILESTONES {
            assert!(m.count >= 3, "Milestone count must be >= 3, got {}", m.count);
        }
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "pvp_streak");
    }
}

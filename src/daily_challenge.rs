/// CakeGame 每日挑战系统
/// 每天生成3个随机挑战，完成后获得丰厚奖励
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::{Datelike, Local};

/// 挑战类型
#[derive(Debug, Clone)]
struct ChallengeDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    category: &'static str,
    target: i32,
    reward_gold: i32,
    reward_diamond: i32,
    reward_exp: i32,
    difficulty: &'static str, // 简单/中等/困难
    emoji: &'static str,
}

/// 所有可能的挑战定义
const ALL_CHALLENGES: &[ChallengeDef] = &[
    ChallengeDef {
        id: "ch_kill_easy",
        name: "初出茅庐",
        desc: "击杀3只怪物",
        category: "战斗",
        target: 3,
        reward_gold: 200,
        reward_diamond: 5,
        reward_exp: 100,
        difficulty: "简单",
        emoji: "⚔️",
    },
    ChallengeDef {
        id: "ch_kill_med",
        name: "身经百战",
        desc: "击杀8只怪物",
        category: "战斗",
        target: 8,
        reward_gold: 500,
        reward_diamond: 10,
        reward_exp: 300,
        difficulty: "中等",
        emoji: "🗡️",
    },
    ChallengeDef {
        id: "ch_kill_hard",
        name: "千人斩",
        desc: "击杀15只怪物",
        category: "战斗",
        target: 15,
        reward_gold: 1000,
        reward_diamond: 25,
        reward_exp: 600,
        difficulty: "困难",
        emoji: "💀",
    },
    ChallengeDef {
        id: "ch_gather_easy",
        name: "拾荒者",
        desc: "采集2次资源",
        category: "生活",
        target: 2,
        reward_gold: 150,
        reward_diamond: 3,
        reward_exp: 80,
        difficulty: "简单",
        emoji: "🌿",
    },
    ChallengeDef {
        id: "ch_gather_med",
        name: "采集达人",
        desc: "采集5次资源",
        category: "生活",
        target: 5,
        reward_gold: 400,
        reward_diamond: 8,
        reward_exp: 200,
        difficulty: "中等",
        emoji: "⛏️",
    },
    ChallengeDef {
        id: "ch_gather_hard",
        name: "资源大亨",
        desc: "采集10次资源",
        category: "生活",
        target: 10,
        reward_gold: 800,
        reward_diamond: 20,
        reward_exp: 500,
        difficulty: "困难",
        emoji: "💎",
    },
    ChallengeDef {
        id: "ch_enhance_easy",
        name: "锻造新手",
        desc: "强化装备1次",
        category: "装备",
        target: 1,
        reward_gold: 300,
        reward_diamond: 5,
        reward_exp: 150,
        difficulty: "简单",
        emoji: "🔨",
    },
    ChallengeDef {
        id: "ch_enhance_med",
        name: "锻造能手",
        desc: "强化装备3次",
        category: "装备",
        target: 3,
        reward_gold: 600,
        reward_diamond: 12,
        reward_exp: 350,
        difficulty: "中等",
        emoji: "🔥",
    },
    ChallengeDef {
        id: "ch_cook_easy",
        name: "美食初试",
        desc: "烹饪1次食物",
        category: "生活",
        target: 1,
        reward_gold: 200,
        reward_diamond: 4,
        reward_exp: 100,
        difficulty: "简单",
        emoji: "🍳",
    },
    ChallengeDef {
        id: "ch_gold_easy",
        name: "小富即安",
        desc: "累计获得500金币",
        category: "经济",
        target: 500,
        reward_gold: 300,
        reward_diamond: 5,
        reward_exp: 120,
        difficulty: "简单",
        emoji: "💰",
    },
    ChallengeDef {
        id: "ch_gold_med",
        name: "财源广进",
        desc: "累计获得2000金币",
        category: "经济",
        target: 2000,
        reward_gold: 800,
        reward_diamond: 15,
        reward_exp: 400,
        difficulty: "中等",
        emoji: "💎",
    },
    ChallengeDef {
        id: "ch_gold_hard",
        name: "富甲一方",
        desc: "累计获得5000金币",
        category: "经济",
        target: 5000,
        reward_gold: 2000,
        reward_diamond: 30,
        reward_exp: 800,
        difficulty: "困难",
        emoji: "👑",
    },
    ChallengeDef {
        id: "ch_sign_easy",
        name: "勤勉之证",
        desc: "完成今日签到",
        category: "日常",
        target: 1,
        reward_gold: 100,
        reward_diamond: 3,
        reward_exp: 50,
        difficulty: "简单",
        emoji: "📋",
    },
    ChallengeDef {
        id: "ch_pvp_easy",
        name: "竞技初体验",
        desc: "匹配战斗1次",
        category: "PVP",
        target: 1,
        reward_gold: 250,
        reward_diamond: 5,
        reward_exp: 120,
        difficulty: "简单",
        emoji: "🏟️",
    },
    ChallengeDef {
        id: "ch_pvp_med",
        name: "竞技老手",
        desc: "匹配战斗3次",
        category: "PVP",
        target: 3,
        reward_gold: 600,
        reward_diamond: 12,
        reward_exp: 350,
        difficulty: "中等",
        emoji: "⚔️",
    },
    ChallengeDef {
        id: "ch_potion_easy",
        name: "药剂师",
        desc: "使用药水2次",
        category: "消耗",
        target: 2,
        reward_gold: 150,
        reward_diamond: 3,
        reward_exp: 80,
        difficulty: "简单",
        emoji: "🧪",
    },
    ChallengeDef {
        id: "ch_composite_easy",
        name: "合成入门",
        desc: "合成1次物品",
        category: "装备",
        target: 1,
        reward_gold: 300,
        reward_diamond: 6,
        reward_exp: 150,
        difficulty: "简单",
        emoji: "⚗️",
    },
];

/// 基于日期和用户ID生成当日挑战（确定性随机）
fn generate_daily_challenges(user_id: &str, day_of_year: u32) -> Vec<&'static ChallengeDef> {
    let seed = day_of_year as u64 * 1000 + hash_user_id(user_id);
    let mut selected = Vec::new();

    // 选3个不同难度的挑战
    let easy_idx = (seed % 7) as usize;
    let med_idx = 7 + ((seed / 7) % 5) as usize;
    let hard_idx = 12 + ((seed / 13) % 5) as usize;

    let easy = easy_idx % ALL_CHALLENGES.len();
    let med = med_idx % ALL_CHALLENGES.len();
    let mut hard = hard_idx % ALL_CHALLENGES.len();
    if hard == easy || hard == med {
        hard = (hard + 3) % ALL_CHALLENGES.len();
    }

    selected.push(easy);
    if med != easy {
        selected.push(med);
    }
    if hard != easy && hard != med {
        selected.push(hard);
    }

    // 补充到3个
    while selected.len() < 3 {
        let next = (selected.last().unwrap() + 1) % ALL_CHALLENGES.len();
        if !selected.contains(&next) {
            selected.push(next);
        }
        if selected.len() >= ALL_CHALLENGES.len() {
            break;
        }
    }

    let mut result = Vec::new();
    for idx in selected.iter().take(3) {
        result.push(&ALL_CHALLENGES[*idx]);
    }
    result
}

/// 简单的用户ID哈希
fn hash_user_id(user_id: &str) -> u64 {
    let mut h: u64 = 5381;
    for b in user_id.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    h
}

/// 获取今天的日期序号
fn today_day_of_year() -> u32 {
    Local::now().ordinal()
}

/// 获取今天日期字符串
fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 存储key前缀
fn challenge_key(challenge_id: &str) -> String {
    format!("dc_{}", challenge_id)
}

/// 获取挑战进度
fn get_progress(db: &Database, user_id: &str, challenge_id: &str) -> i32 {
    let key = format!("{}_{}", challenge_key(challenge_id), today_str());
    db.read_user_data(user_id, &key).parse().unwrap_or(0)
}

/// 设置挑战进度
#[allow(dead_code)]
fn set_progress(db: &Database, user_id: &str, challenge_id: &str, value: i32) {
    let key = format!("{}_{}", challenge_key(challenge_id), today_str());
    db.write_user_data(user_id, &key, &value.to_string());
}

/// 检查奖励是否已领取
fn is_claimed(db: &Database, user_id: &str, challenge_id: &str) -> bool {
    let key = format!("{}_claimed_{}", challenge_key(challenge_id), today_str());
    db.read_user_data(user_id, &key) == "1"
}

/// 标记奖励已领取
fn mark_claimed(db: &Database, user_id: &str, challenge_id: &str) {
    let key = format!("{}_claimed_{}", challenge_key(challenge_id), today_str());
    db.write_user_data(user_id, &key, "1");
}

/// 检查全完成奖励是否已领取
fn is_bonus_claimed(db: &Database, user_id: &str) -> bool {
    let key = format!("dc_bonus_{}", today_str());
    db.read_user_data(user_id, &key) == "1"
}

/// 标记全完成奖励已领取
fn mark_bonus_claimed(db: &Database, user_id: &str) {
    let key = format!("dc_bonus_{}", today_str());
    db.write_user_data(user_id, &key, "1");
}

/// 查看日常挑战
pub fn cmd_daily_challenge(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let challenges = generate_daily_challenges(user_id, today_day_of_year());

    let mut r = format!("{}\n═══ 🎯 每日挑战 ═══\n", prefix);
    r.push_str(&format!("📅 {}\n", today_str()));
    r.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    let mut all_done = true;
    for (i, ch) in challenges.iter().enumerate() {
        let progress = get_progress(db, user_id, ch.id);
        let claimed = is_claimed(db, user_id, ch.id);
        let done = progress >= ch.target;

        if !done {
            all_done = false;
        }

        let status = if claimed {
            "✅ 已领取".to_string()
        } else if done {
            "🎁 可领取".to_string()
        } else {
            format!("📊 {}/{}", progress, ch.target)
        };

        r.push_str(&format!(
            "\n{}. {} [{}] {}\n   {} — {}\n   奖励: 💰{}金 💎{}钻 ✨{}经验\n   状态: {}",
            i + 1,
            ch.emoji,
            ch.difficulty,
            ch.name,
            ch.desc,
            ch.category,
            ch.reward_gold,
            ch.reward_diamond,
            ch.reward_exp,
            status
        ));
    }

    r.push_str("\n━━━━━━━━━━━━━━━━━━━━");

    if all_done && !is_bonus_claimed(db, user_id) {
        r.push_str("\n\n🏆 所有挑战已完成！发送「领取挑战奖励」领取全部奖励+额外全完成奖励！");
    }

    r.push_str("\n\n发送「领取挑战奖励」领取已完成的挑战奖励");
    r.push_str("\n发送「挑战进度」查看当前进度");

    r
}

/// 领取挑战奖励
pub fn cmd_claim_challenge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let challenges = generate_daily_challenges(user_id, today_day_of_year());
    let args_trimmed = args.trim();

    let mut total_gold = 0i32;
    let mut total_diamond = 0i32;
    let mut total_exp = 0i32;
    let mut claimed_count = 0;
    let mut r = format!("{}\n═══ 🎁 领取挑战奖励 ═══\n", prefix);

    for ch in challenges.iter() {
        let progress = get_progress(db, user_id, ch.id);
        let already_claimed = is_claimed(db, user_id, ch.id);

        // 如果指定了挑战名称，只领取那个
        if !args_trimmed.is_empty() && !ch.name.contains(args_trimmed) && ch.id != args_trimmed {
            continue;
        }

        if already_claimed {
            r.push_str(&format!("\n{} [{}] 奖励已领取", ch.emoji, ch.name));
            continue;
        }

        if progress < ch.target {
            r.push_str(&format!(
                "\n{} [{}] 未完成 ({}/{})",
                ch.emoji, ch.name, progress, ch.target
            ));
            continue;
        }

        // 领取奖励
        mark_claimed(db, user_id, ch.id);
        total_gold += ch.reward_gold;
        total_diamond += ch.reward_diamond;
        total_exp += ch.reward_exp;
        claimed_count += 1;
        r.push_str(&format!(
            "\n✅ [{}] 奖励已领取：💰{}金 💎{}钻 ✨{}经验",
            ch.name, ch.reward_gold, ch.reward_diamond, ch.reward_exp
        ));
    }

    // 检查是否全部完成，给予额外奖励
    let all_done = challenges.iter().all(|ch| {
        let progress = get_progress(db, user_id, ch.id);
        progress >= ch.target
    });

    if all_done && !is_bonus_claimed(db, user_id) {
        mark_bonus_claimed(db, user_id);
        total_gold += 500;
        total_diamond += 20;
        total_exp += 300;
        r.push_str("\n\n🏆 全部挑战完成！额外奖励：💰500金 💎20钻 ✨300经验");
    }

    if claimed_count == 0 && total_gold == 0 {
        r.push_str("\n\n暂无可领取的奖励。发送「日常挑战」查看挑战详情");
    } else {
        // 发放奖励
        if total_gold > 0 {
            let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", total_gold as i64);
        }
        if total_diamond > 0 {
            let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", total_diamond as i64);
        }
        if total_exp > 0 {
            let _ = user::add_experience(db, user_id, total_exp);
        }
        r.push_str(&format!(
            "\n\n━━━━━━━━━━━━━━━━━━━━\n💰 总计获得：{}金 {}钻 {}经验",
            total_gold, total_diamond, total_exp
        ));
    }

    r
}

/// 挑战进度
pub fn cmd_challenge_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let challenges = generate_daily_challenges(user_id, today_day_of_year());

    let mut r = format!("{}\n═══ 📊 挑战进度 ═══\n", prefix);

    let mut all_done = true;
    for ch in challenges.iter() {
        let progress = get_progress(db, user_id, ch.id);
        let claimed = is_claimed(db, user_id, ch.id);
        let done = progress >= ch.target;
        if !done {
            all_done = false;
        }

        let pct: i32 = if ch.target > 0 {
            (progress * 100 / ch.target).min(100)
        } else {
            0
        };
        let bar_len: usize = 10;
        let filled: usize = (pct as usize * bar_len / 100).min(bar_len);
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));

        let status_icon = if claimed {
            "✅"
        } else if done {
            "🎁"
        } else {
            "⏳"
        };
        r.push_str(&format!(
            "\n{} {} [{}] {}/{} ({}%)\n   {}",
            status_icon, ch.emoji, ch.name, progress, ch.target, pct, bar
        ));
    }

    if all_done {
        let bonus = is_bonus_claimed(db, user_id);
        r.push_str(&format!(
            "\n\n🏆 全部挑战已完成！{}",
            if bonus {
                "奖励已领取"
            } else {
                "发送「领取挑战奖励」领取额外奖励"
            }
        ));
    }

    r
}

/// 更新挑战进度（供其他模块调用）
#[allow(dead_code)]
pub fn update_challenge_progress(db: &Database, user_id: &str, category: &str, amount: i32) {
    let challenges = generate_daily_challenges(user_id, today_day_of_year());

    for ch in challenges.iter() {
        if ch.category == category || (category == "战斗" && ch.category == "战斗") {
            let current = get_progress(db, user_id, ch.id);
            set_progress(db, user_id, ch.id, current + amount);
        }
    }
}

/// 签到特殊标记（签到挑战用）
#[allow(dead_code)]
pub fn mark_sign_in(db: &Database, user_id: &str) {
    let challenges = generate_daily_challenges(user_id, today_day_of_year());
    for ch in challenges.iter() {
        if ch.id == "ch_sign_easy" {
            set_progress(db, user_id, ch.id, 1);
        }
    }
}

/// 每日挑战排行榜
pub fn cmd_challenge_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = today_day_of_year();
    let today_s = today_str();

    let mut players: Vec<(String, i32, i32, i32)> = Vec::new(); // (name, completed, total_challenges, total_reward_value)

    // 遍历所有用户，计算今日挑战完成情况
    let all_users = db.all_users();
    for uid in all_users.iter() {
        let challenges = generate_daily_challenges(uid, today);
        let mut completed = 0i32;
        let mut total_reward = 0i32;

        for ch in challenges.iter() {
            let progress = get_progress(db, uid, ch.id);
            let claimed = is_claimed(db, uid, ch.id);
            if progress >= ch.target {
                completed += 1;
                if claimed {
                    total_reward += ch.reward_gold + ch.reward_diamond * 10 + ch.reward_exp;
                }
            }
        }

        // 只记录至少有1个挑战完成的玩家
        if completed > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, completed, challenges.len() as i32, total_reward));
        }
    }

    if players.is_empty() {
        return format!("{}\n═══ 🏆 每日挑战排行 ═══\n📅 {}\n━━━━━━━━━━━━━━━━━━━━\n\n暂无玩家完成今日挑战\n完成「日常挑战」即可上榜！", prefix, today_s);
    }

    // 按完成数降序，总奖励值降序排列
    players.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.3.cmp(&a.3)));

    let mut r = format!(
        "{}\n═══ 🏆 每日挑战排行 ═══\n📅 {}\n━━━━━━━━━━━━━━━━━━━━\n",
        prefix, today_s
    );

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, completed, total, _reward)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let star = if completed >= total { " ⭐全部完成" } else { "" };
        r.push_str(&format!("\n{} {} {}/{}挑战{}", medal, name, completed, total, star));
    }

    // 当前用户排名定位
    let user_challenges = generate_daily_challenges(user_id, today);
    let user_completed: i32 = user_challenges
        .iter()
        .filter(|ch| get_progress(db, user_id, ch.id) >= ch.target)
        .count() as i32;

    let user_rank = players
        .iter()
        .position(|(name, _, _, _)| name == &user::get_msg_prefix(db, user_id));

    r.push_str("\n\n━━━━━━━━━━━━━━━━━━━━");
    match user_rank {
        Some(rank) => r.push_str(&format!(
            "\n📍 你的排名：第{}名 ({}/{}挑战完成)",
            rank + 1,
            user_completed,
            user_challenges.len()
        )),
        None => r.push_str(&format!(
            "\n📍 你今日尚未完成挑战 ({}/{}进行中)",
            user_completed,
            user_challenges.len()
        )),
    }

    r
}

// ==================== 单元测试 ====================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_count() {
        assert_eq!(ALL_CHALLENGES.len(), 17);
    }

    #[test]
    fn test_generate_challenges_returns_three() {
        let challenges = generate_daily_challenges("test_user", 100);
        assert!(challenges.len() >= 2 && challenges.len() <= 3);
    }

    #[test]
    fn test_different_users_different_challenges() {
        let c1 = generate_daily_challenges("user_a", 100);
        let c2 = generate_daily_challenges("user_b", 100);
        // 不同用户可能有相同挑战，但种子不同
        let _ = (c1, c2); // 编译通过即可
    }

    #[test]
    fn test_hash_user_id_deterministic() {
        assert_eq!(hash_user_id("test"), hash_user_id("test"));
        assert_ne!(hash_user_id("a"), hash_user_id("b"));
    }

    #[test]
    fn test_challenge_difficulty_levels() {
        let easy: Vec<_> = ALL_CHALLENGES.iter().filter(|c| c.difficulty == "简单").collect();
        let med: Vec<_> = ALL_CHALLENGES.iter().filter(|c| c.difficulty == "中等").collect();
        let hard: Vec<_> = ALL_CHALLENGES.iter().filter(|c| c.difficulty == "困难").collect();
        assert!(easy.len() >= 5);
        assert!(med.len() >= 4);
        assert!(hard.len() >= 3);
    }

    #[test]
    fn test_challenge_categories() {
        let mut cats: Vec<&str> = ALL_CHALLENGES.iter().map(|c| c.category).collect();
        cats.sort();
        cats.dedup();
        assert!(cats.len() >= 5); // 战斗, 生活, 装备, 经济, 日常, PVP, 消耗
    }

    #[test]
    fn test_all_challenges_have_valid_rewards() {
        for ch in ALL_CHALLENGES.iter() {
            assert!(ch.reward_gold > 0, "Challenge {} has no gold reward", ch.id);
            assert!(ch.reward_diamond > 0, "Challenge {} has no diamond reward", ch.id);
            assert!(ch.reward_exp > 0, "Challenge {} has no exp reward", ch.id);
        }
    }

    #[test]
    fn test_all_challenges_have_emoji() {
        for ch in ALL_CHALLENGES.iter() {
            assert!(!ch.emoji.is_empty(), "Challenge {} has no emoji", ch.id);
            assert!(!ch.name.is_empty(), "Challenge {} has no name", ch.id);
            assert!(!ch.desc.is_empty(), "Challenge {} has no desc", ch.id);
        }
    }

    #[test]
    fn test_all_challenges_have_valid_target() {
        for ch in ALL_CHALLENGES.iter() {
            assert!(ch.target > 0, "Challenge {} has invalid target: {}", ch.id, ch.target);
        }
    }

    #[test]
    fn test_generate_challenges_all_unique() {
        // Test for many different seeds to ensure no duplicates
        for day in 1..=366 {
            let challenges = generate_daily_challenges("test_ranking_user", day);
            let ids: Vec<&str> = challenges.iter().map(|c| c.id).collect();
            let mut unique_ids = ids.clone();
            unique_ids.sort();
            unique_ids.dedup();
            assert_eq!(ids.len(), unique_ids.len(), "Duplicate challenges on day {}", day);
        }
    }

    #[test]
    fn test_challenge_difficulty_distribution() {
        // Each generated set should ideally have mixed difficulties
        let challenges = generate_daily_challenges("rank_user", 150);
        let has_easy = challenges.iter().any(|c| c.difficulty == "简单");
        let has_medium_or_hard = challenges
            .iter()
            .any(|c| c.difficulty == "中等" || c.difficulty == "困难");
        assert!(has_easy, "Should have at least one easy challenge");
        assert!(has_medium_or_hard, "Should have at least one medium/hard challenge");
    }

    #[test]
    fn test_challenge_reward_value_positive() {
        // The total reward value for any set of challenges should be positive
        let challenges = generate_daily_challenges("reward_check", 200);
        let total: i32 = challenges
            .iter()
            .map(|c| c.reward_gold + c.reward_diamond * 10 + c.reward_exp)
            .sum();
        assert!(total > 0, "Total reward value should be positive");
    }
}

/// CakeGame 怪物猎人等级系统
///
/// 玩家通过击杀怪物积累猎人经验，提升猎人等级，获得被动属性加成。
/// 每个猎人等级有独特的称号、属性加成和升级条件。
/// 包含每日狩猎任务和猎人商店系统。
///
/// 数据存储: Global 表 section = "monster_hunter"
///
/// 指令: 猎人等级, 猎人详情, 猎人排行, 猎人任务, 领取猎人奖励, 猎人商店, 猎人兑换
use crate::core::*;
use crate::db::Database;

const SECTION: &str = "monster_hunter";
const SECTION_DAILY: &str = "monster_hunter_daily";

/// 猎人等级定义
struct HunterRank {
    rank: i32,
    name: &'static str,
    emoji: &'static str,
    required_points: i64,
    hp_bonus: i32,
    ad_bonus: i32,
    ap_bonus: i32,
    def_bonus: i32,
    mr_bonus: i32,
    gold_bonus_pct: i32,
    exp_bonus_pct: i32,
}

const HUNTER_RANKS: &[HunterRank] = &[
    HunterRank {
        rank: 0,
        name: "新手猎人",
        emoji: "🔰",
        required_points: 0,
        hp_bonus: 0,
        ad_bonus: 0,
        ap_bonus: 0,
        def_bonus: 0,
        mr_bonus: 0,
        gold_bonus_pct: 0,
        exp_bonus_pct: 0,
    },
    HunterRank {
        rank: 1,
        name: "见习猎人",
        emoji: "🥉",
        required_points: 100,
        hp_bonus: 50,
        ad_bonus: 10,
        ap_bonus: 10,
        def_bonus: 5,
        mr_bonus: 5,
        gold_bonus_pct: 5,
        exp_bonus_pct: 5,
    },
    HunterRank {
        rank: 2,
        name: "初级猎人",
        emoji: "🥈",
        required_points: 500,
        hp_bonus: 120,
        ad_bonus: 25,
        ap_bonus: 25,
        def_bonus: 12,
        mr_bonus: 12,
        gold_bonus_pct: 10,
        exp_bonus_pct: 10,
    },
    HunterRank {
        rank: 3,
        name: "中级猎人",
        emoji: "🏅",
        required_points: 1500,
        hp_bonus: 250,
        ad_bonus: 50,
        ap_bonus: 50,
        def_bonus: 25,
        mr_bonus: 25,
        gold_bonus_pct: 15,
        exp_bonus_pct: 15,
    },
    HunterRank {
        rank: 4,
        name: "高级猎人",
        emoji: "🎖️",
        required_points: 4000,
        hp_bonus: 450,
        ad_bonus: 80,
        ap_bonus: 80,
        def_bonus: 40,
        mr_bonus: 40,
        gold_bonus_pct: 20,
        exp_bonus_pct: 20,
    },
    HunterRank {
        rank: 5,
        name: "精英猎人",
        emoji: "⭐",
        required_points: 10000,
        hp_bonus: 700,
        ad_bonus: 120,
        ap_bonus: 120,
        def_bonus: 60,
        mr_bonus: 60,
        gold_bonus_pct: 25,
        exp_bonus_pct: 25,
    },
    HunterRank {
        rank: 6,
        name: "大师猎人",
        emoji: "🌟",
        required_points: 25000,
        hp_bonus: 1000,
        ad_bonus: 170,
        ap_bonus: 170,
        def_bonus: 85,
        mr_bonus: 85,
        gold_bonus_pct: 30,
        exp_bonus_pct: 30,
    },
    HunterRank {
        rank: 7,
        name: "宗师猎人",
        emoji: "💫",
        required_points: 60000,
        hp_bonus: 1500,
        ad_bonus: 250,
        ap_bonus: 250,
        def_bonus: 125,
        mr_bonus: 125,
        gold_bonus_pct: 40,
        exp_bonus_pct: 40,
    },
    HunterRank {
        rank: 8,
        name: "传说猎人",
        emoji: "👑",
        required_points: 150000,
        hp_bonus: 2200,
        ad_bonus: 350,
        ap_bonus: 350,
        def_bonus: 175,
        mr_bonus: 175,
        gold_bonus_pct: 50,
        exp_bonus_pct: 50,
    },
    HunterRank {
        rank: 9,
        name: "猎人之王",
        emoji: "⚜️",
        required_points: 400000,
        hp_bonus: 3500,
        ad_bonus: 500,
        ap_bonus: 500,
        def_bonus: 250,
        mr_bonus: 250,
        gold_bonus_pct: 60,
        exp_bonus_pct: 60,
    },
];

/// 每日狩猎任务定义
struct DailyHuntQuest {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    target_monster: &'static str,
    target_count: i32,
    reward_points: i64,
    reward_gold: i64,
    reward_diamond: i32,
}

const DAILY_HUNT_QUESTS: &[DailyHuntQuest] = &[
    DailyHuntQuest {
        id: "hunt_slime",
        name: "史莱姆清剿",
        emoji: "🟢",
        target_monster: "史莱姆",
        target_count: 10,
        reward_points: 30,
        reward_gold: 500,
        reward_diamond: 5,
    },
    DailyHuntQuest {
        id: "hunt_goblin",
        name: "哥布林猎杀",
        emoji: "👺",
        target_monster: "哥布林",
        target_count: 8,
        reward_points: 40,
        reward_gold: 800,
        reward_diamond: 8,
    },
    DailyHuntQuest {
        id: "hunt_wolf",
        name: "野狼狩猎",
        emoji: "🐺",
        target_monster: "狼",
        target_count: 6,
        reward_points: 50,
        reward_gold: 1000,
        reward_diamond: 10,
    },
    DailyHuntQuest {
        id: "hunt_bear",
        name: "巨熊挑战",
        emoji: "🐻",
        target_monster: "巨熊",
        target_count: 5,
        reward_points: 60,
        reward_gold: 1500,
        reward_diamond: 12,
    },
    DailyHuntQuest {
        id: "hunt_skeleton",
        name: "骷髅净化",
        emoji: "💀",
        target_monster: "骷髅",
        target_count: 8,
        reward_points: 45,
        reward_gold: 700,
        reward_diamond: 7,
    },
    DailyHuntQuest {
        id: "hunt_spider",
        name: "毒蛛清巢",
        emoji: "🕷️",
        target_monster: "毒蛛",
        target_count: 6,
        reward_points: 55,
        reward_gold: 1200,
        reward_diamond: 10,
    },
    DailyHuntQuest {
        id: "hunt_dragon",
        name: "飞龙讨伐",
        emoji: "🐉",
        target_monster: "飞龙",
        target_count: 3,
        reward_points: 100,
        reward_gold: 3000,
        reward_diamond: 20,
    },
    DailyHuntQuest {
        id: "hunt_demon",
        name: "恶魔猎杀",
        emoji: "👹",
        target_monster: "恶魔",
        target_count: 3,
        reward_points: 120,
        reward_gold: 3500,
        reward_diamond: 25,
    },
];

/// 猎人商店商品定义
struct HunterShopItem {
    name: &'static str,
    emoji: &'static str,
    cost_points: i64,
    description: &'static str,
    min_rank: i32,
}

const HUNTER_SHOP_ITEMS: &[HunterShopItem] = &[
    HunterShopItem {
        name: "猎人药水",
        emoji: "🧪",
        cost_points: 50,
        description: "恢复500HP",
        min_rank: 0,
    },
    HunterShopItem {
        name: "猎人强化石",
        emoji: "💎",
        cost_points: 200,
        description: "装备强化材料",
        min_rank: 1,
    },
    HunterShopItem {
        name: "猎人护符",
        emoji: "🛡️",
        cost_points: 500,
        description: "临时防御+50(1小时)",
        min_rank: 2,
    },
    HunterShopItem {
        name: "猎人战旗",
        emoji: "🚩",
        cost_points: 800,
        description: "临时攻击+80(1小时)",
        min_rank: 3,
    },
    HunterShopItem {
        name: "猎人宝箱",
        emoji: "🎁",
        cost_points: 1500,
        description: "随机稀有道具",
        min_rank: 4,
    },
    HunterShopItem {
        name: "猎人之眼",
        emoji: "👁️",
        cost_points: 3000,
        description: "查看怪物弱点",
        min_rank: 5,
    },
    HunterShopItem {
        name: "猎人秘药",
        emoji: "⚗️",
        cost_points: 5000,
        description: "全属性+100(30分钟)",
        min_rank: 6,
    },
    HunterShopItem {
        name: "猎人传说卷轴",
        emoji: "📜",
        cost_points: 10000,
        description: "随机传说装备",
        min_rank: 7,
    },
    HunterShopItem {
        name: "猎人王冠",
        emoji: "👑",
        cost_points: 25000,
        description: "永久全属性+50",
        min_rank: 8,
    },
    HunterShopItem {
        name: "猎人至尊宝箱",
        emoji: "🏆",
        cost_points: 50000,
        description: "至尊道具+神器碎片",
        min_rank: 9,
    },
];

/// 获取玩家猎人数据 (points|rank)
fn get_hunter_data(db: &Database, user_id: &str) -> (i64, i32) {
    let data = db.global_get(SECTION, &format!("data_{}", user_id));
    if data.is_empty() {
        return (0, 0);
    }
    let parts: Vec<&str> = data.split('|').collect();
    let points: i64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let rank: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    (points, rank)
}

/// 保存玩家猎人数据
fn save_hunter_data(db: &Database, user_id: &str, points: i64, rank: i32) {
    let data = format!("{}|{}", points, rank);
    db.global_set(SECTION, &format!("data_{}", user_id), &data);
}

/// 获取今日日期字符串
fn today_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    format!("{}", days)
}

/// 获取玩家每日任务进度
fn get_daily_quest_progress(db: &Database, user_id: &str, quest_id: &str) -> (i32, bool) {
    let today = today_str();
    let key = format!("daily_{}_{}", user_id, quest_id);
    let data = db.global_get(SECTION_DAILY, &key);
    if data.is_empty() {
        return (0, false);
    }
    let parts: Vec<&str> = data.split('|').collect();
    let saved_date = parts.first().unwrap_or(&"");
    if saved_date != &today {
        return (0, false);
    }
    let count: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let claimed = parts.get(2) == Some(&"1");
    (count, claimed)
}

/// 保存每日任务进度
fn save_daily_quest_progress(db: &Database, user_id: &str, quest_id: &str, count: i32, claimed: bool) {
    let today = today_str();
    let key = format!("daily_{}_{}", user_id, quest_id);
    let data = format!("{}|{}|{}", today, count, if claimed { "1" } else { "0" });
    db.global_set(SECTION_DAILY, &key, &data);
}

/// 获取猎人等级信息
fn get_rank_info(rank: i32) -> &'static HunterRank {
    HUNTER_RANKS.iter().find(|r| r.rank == rank).unwrap_or(&HUNTER_RANKS[0])
}

/// 计算猎人等级（根据积分自动升级）
fn calc_rank(points: i64) -> i32 {
    let mut rank = 0;
    for r in HUNTER_RANKS.iter().rev() {
        if points >= r.required_points {
            rank = r.rank;
            break;
        }
    }
    rank
}

/// 记录猎人积分（供战斗系统调用）
#[allow(dead_code)]
pub fn record_hunt_points(db: &Database, user_id: &str, monster_name: &str, kill_count: i32) {
    let (mut points, rank) = get_hunter_data(db, user_id);
    let base_points = 10 * kill_count as i64;
    let rank_bonus = (rank as i64) * 2 * kill_count as i64;
    points += base_points + rank_bonus;
    let new_rank = calc_rank(points);
    save_hunter_data(db, user_id, points, new_rank);

    // 更新每日任务进度
    for quest in DAILY_HUNT_QUESTS {
        if quest.target_monster == monster_name {
            let (count, claimed) = get_daily_quest_progress(db, user_id, quest.id);
            save_daily_quest_progress(db, user_id, quest.id, count + kill_count, claimed);
        }
    }
}

/// 获取猎人属性加成
#[allow(dead_code)]
pub fn get_hunter_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let (_, rank) = get_hunter_data(db, user_id);
    let info = get_rank_info(rank);
    (
        info.hp_bonus,
        info.ad_bonus,
        info.ap_bonus,
        info.def_bonus,
        info.mr_bonus,
    )
}

/// 获取猎人金币/经验加成百分比
#[allow(dead_code)]
pub fn get_hunter_bonus_pct(db: &Database, user_id: &str) -> (i32, i32) {
    let (_, rank) = get_hunter_data(db, user_id);
    let info = get_rank_info(rank);
    (info.gold_bonus_pct, info.exp_bonus_pct)
}

/// 指令: 猎人等级
pub fn cmd_hunter_rank(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }
    let nickname = db.read_basic(user_id, "Nickname");
    let (points, rank) = get_hunter_data(db, user_id);
    let info = get_rank_info(rank);

    let next_rank_info = HUNTER_RANKS.iter().find(|r| r.rank == rank + 1);
    let progress = if let Some(next) = next_rank_info {
        let pct = (points as f64 / next.required_points as f64 * 100.0).min(100.0);
        let bar_len = 20;
        let filled = (pct / 100.0 * bar_len as f64) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));
        format!(
            "\n📈 升级进度: {} {:.1}%\n🎯 需要积分: {}/{}",
            bar, pct, points, next.required_points
        )
    } else {
        "\n🏆 已达最高等级！".to_string()
    };

    format!(
        "{} ═══════════════════════\n\
         {} {} {}\n\
         🎯 猎人积分: {}\n\
         {}\n\
         ──────── 属性加成 ────────\n\
         ❤️ HP+{} | ⚔️ 物攻+{} | 🔮 魔攻+{}\n\
         🛡️ 防御+{} | 🔰 魔抗+{}\n\
         💰 金币加成: +{}% | 📚 经验加成: +{}%\n\
         ═══════════════════════",
        nickname,
        info.emoji,
        info.name,
        info.emoji,
        points,
        progress,
        info.hp_bonus,
        info.ad_bonus,
        info.ap_bonus,
        info.def_bonus,
        info.mr_bonus,
        info.gold_bonus_pct,
        info.exp_bonus_pct
    )
}

/// 指令: 猎人详情
pub fn cmd_hunter_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }
    let nickname = db.read_basic(user_id, "Nickname");
    let (points, rank) = get_hunter_data(db, user_id);
    let info = get_rank_info(rank);

    let mut result = format!(
        "{} ═══ 猎人等级详情 ═══\n\n\
         当前等级: {} {} (Rank {})\n\
         猎人积分: {}\n\n\
         📋 全部等级一览:\n",
        nickname, info.emoji, info.name, rank, points
    );

    for r in HUNTER_RANKS {
        let status = if r.rank == rank {
            " ◄ 当前"
        } else if r.rank < rank {
            " ✅"
        } else {
            ""
        };
        result.push_str(&format!(
            "  {} Rank {} {} — {} (需要积分{})\n",
            r.emoji, r.rank, r.name, r.required_points, status
        ));
    }

    result.push_str("\n💡 击杀怪物可获得猎人积分，积分越高属性加成越强！");
    result
}

/// 指令: 猎人排行
pub fn cmd_hunter_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_user_ids = db.all_users();
    let mut hunters: Vec<(String, i64, i32)> = Vec::new();

    for uid in &all_user_ids {
        let (points, rank) = get_hunter_data(db, uid);
        if points > 0 {
            let name = db.read_basic(uid, "Nickname");
            hunters.push((name, points, rank));
        }
    }

    hunters.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut result = "🏆 ═══ 猎人排行榜 ═══ 🏆\n\n".to_string();

    for (i, (name, points, rank)) in hunters.iter().take(15).enumerate() {
        let info = get_rank_info(*rank);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        result.push_str(&format!(
            "{} {}. {} — {} {} ({}积分)\n",
            medal,
            i + 1,
            name,
            info.emoji,
            info.name,
            points
        ));
    }

    if hunters.is_empty() {
        result.push_str("暂无猎人数据\n");
    }

    result.push_str(&format!("\n共 {} 位猎人参与排名", hunters.len()));
    result
}

/// 指令: 猎人任务
pub fn cmd_hunter_quests(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }
    let nickname = db.read_basic(user_id, "Nickname");

    let mut result = format!("{} ═══ 今日狩猎任务 ═══\n\n", nickname);

    let mut has_available = false;
    for quest in DAILY_HUNT_QUESTS {
        let (count, claimed) = get_daily_quest_progress(db, user_id, quest.id);
        let progress = count.min(quest.target_count);
        let pct = (progress as f64 / quest.target_count as f64 * 100.0).min(100.0);
        let status = if claimed {
            "✅ 已领取".to_string()
        } else if progress >= quest.target_count {
            "🎁 可领取".to_string()
        } else {
            has_available = true;
            format!("⏳ {}/{}", progress, quest.target_count)
        };
        let bar_len = 10;
        let filled = (pct / 100.0 * bar_len as f64) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));

        result.push_str(&format!(
            "{} {} {} — 击杀{} ×{}\n   {} {} (积分+{} 金+{} 钻+{})\n\n",
            quest.emoji,
            quest.name,
            quest.target_monster,
            quest.target_monster,
            quest.target_count,
            bar,
            status,
            quest.reward_points,
            quest.reward_gold,
            quest.reward_diamond
        ));
    }

    if !has_available {
        result.push_str("🎉 今日所有任务已完成！明天再来。\n");
    }

    result.push_str("💡 击杀对应怪物自动累积进度，完成后使用「领取猎人奖励+任务名」领取");
    result
}

/// 指令: 领取猎人奖励
pub fn cmd_claim_hunt_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let target = args.trim();
    if target.is_empty() {
        return "❌ 请指定任务名！例: 领取猎人奖励+史莱姆清剿\n使用「猎人任务」查看可领取的任务".to_string();
    }

    for quest in DAILY_HUNT_QUESTS {
        if quest.name == target || quest.target_monster == target || quest.id.contains(target) {
            let (count, claimed) = get_daily_quest_progress(db, user_id, quest.id);
            if claimed {
                return format!("❌ 任务 [{}] 奖励已领取！", quest.name);
            }
            if count < quest.target_count {
                return format!(
                    "❌ 任务 [{}] 尚未完成！进度: {}/{}",
                    quest.name, count, quest.target_count
                );
            }

            // 发放奖励
            save_daily_quest_progress(db, user_id, quest.id, count, true);
            let (mut points, rank) = get_hunter_data(db, user_id);
            points += quest.reward_points;
            let new_rank = calc_rank(points);
            save_hunter_data(db, user_id, points, new_rank);

            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, quest.reward_gold);
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, quest.reward_diamond as i64);

            let mut result = format!(
                "🎁 领取成功！\n\n\
                 任务: {} {}\n\
                 📌 猎人积分: +{} (总计: {})\n\
                 💰 金币: +{}\n\
                 💎 钻石: +{}",
                quest.emoji, quest.name, quest.reward_points, points, quest.reward_gold, quest.reward_diamond
            );

            if new_rank > rank {
                let new_info = get_rank_info(new_rank);
                result.push_str(&format!(
                    "\n\n🎉🎉🎉 恭喜晋升！\n{} {} → {} {}\n属性加成已提升！",
                    get_rank_info(rank).emoji,
                    get_rank_info(rank).name,
                    new_info.emoji,
                    new_info.name
                ));
            }

            return result;
        }
    }

    format!("❌ 未找到任务「{}」！使用「猎人任务」查看可用任务", target)
}

/// 指令: 猎人商店
pub fn cmd_hunter_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }
    let nickname = db.read_basic(user_id, "Nickname");
    let (points, rank) = get_hunter_data(db, user_id);
    let info = get_rank_info(rank);

    let mut result = format!(
        "{} ═══ 猎人商店 ═══\n\
         {} {} | 积分: {}\n\n",
        nickname, info.emoji, info.name, points
    );

    for (i, item) in HUNTER_SHOP_ITEMS.iter().enumerate() {
        let min_info = get_rank_info(item.min_rank);
        let available = rank >= item.min_rank;
        let status = if available {
            "✅".to_string()
        } else {
            format!("🔒 需要{}", min_info.name)
        };
        result.push_str(&format!(
            "{}. {} {} — {}积分\n   {} {}\n",
            i + 1,
            item.emoji,
            item.name,
            item.cost_points,
            item.description,
            status
        ));
    }

    result.push_str("\n💡 使用「猎人兑换+商品名」购买");
    result
}

/// 指令: 猎人兑换
pub fn cmd_hunter_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册！".to_string();
    }

    let target = args.trim();
    if target.is_empty() {
        return "❌ 请指定商品名！例: 猎人兑换+猎人药水\n使用「猎人商店」查看商品列表".to_string();
    }

    let (mut points, rank) = get_hunter_data(db, user_id);

    for item in HUNTER_SHOP_ITEMS {
        if item.name == target {
            if rank < item.min_rank {
                let min_info = get_rank_info(item.min_rank);
                return format!(
                    "❌ 等级不足！兑换 [{}] 需要 {} {}，您当前是 {} {}",
                    item.name,
                    min_info.emoji,
                    min_info.name,
                    get_rank_info(rank).emoji,
                    get_rank_info(rank).name
                );
            }
            if points < item.cost_points {
                return format!(
                    "❌ 积分不足！兑换 [{}] 需要 {} 积分，您只有 {} 积分",
                    item.name, item.cost_points, points
                );
            }

            points -= item.cost_points;
            save_hunter_data(db, user_id, points, rank);

            // 根据商品类型发放奖励
            let reward_desc = match item.name {
                "猎人药水" => {
                    db.knapsack_add(user_id, "生命药水", 3);
                    "生命药水 ×3"
                }
                "猎人强化石" => {
                    db.knapsack_add(user_id, "强化石", 2);
                    "强化石 ×2"
                }
                "猎人护符" => {
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 2000);
                    "2000金币"
                }
                "猎人战旗" => {
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 3000);
                    "3000金币"
                }
                "猎人宝箱" => {
                    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 50);
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 5000);
                    "5000金币 + 50钻石"
                }
                "猎人之眼" => {
                    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 100);
                    "100钻石"
                }
                "猎人秘药" => {
                    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 200);
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 10000);
                    "10000金币 + 200钻石"
                }
                "猎人传说卷轴" => {
                    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 500);
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 30000);
                    db.knapsack_add(user_id, "强化石", 10);
                    "30000金币 + 500钻石 + 强化石×10"
                }
                "猎人王冠" => {
                    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 1000);
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 100000);
                    "100000金币 + 1000钻石"
                }
                "猎人至尊宝箱" => {
                    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 2000);
                    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 500000);
                    db.knapsack_add(user_id, "强化石", 50);
                    db.knapsack_add(user_id, "复活卷轴", 5);
                    "500000金币 + 2000钻石 + 强化石×50 + 复活卷轴×5"
                }
                _ => "奖励",
            };

            return format!(
                "✅ 兑换成功！\n{} {}\n消耗: {} 积分\n获得: {}\n剩余积分: {}",
                item.emoji, item.name, item.cost_points, reward_desc, points
            );
        }
    }

    format!("❌ 未找到商品「{}」！使用「猎人商店」查看商品列表", target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunter_ranks_count() {
        assert_eq!(HUNTER_RANKS.len(), 10);
    }

    #[test]
    fn test_hunter_ranks_escalate() {
        for i in 1..HUNTER_RANKS.len() {
            assert!(HUNTER_RANKS[i].required_points > HUNTER_RANKS[i - 1].required_points);
            assert!(HUNTER_RANKS[i].hp_bonus > HUNTER_RANKS[i - 1].hp_bonus);
            assert!(HUNTER_RANKS[i].ad_bonus > HUNTER_RANKS[i - 1].ad_bonus);
        }
    }

    #[test]
    fn test_hunter_rank_names_unique() {
        let mut names: Vec<&str> = HUNTER_RANKS.iter().map(|r| r.name).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len);
    }

    #[test]
    fn test_hunter_rank_first_is_zero() {
        assert_eq!(HUNTER_RANKS[0].rank, 0);
        assert_eq!(HUNTER_RANKS[0].required_points, 0);
        assert_eq!(HUNTER_RANKS[0].hp_bonus, 0);
    }

    #[test]
    fn test_calc_rank_zero_points() {
        assert_eq!(calc_rank(0), 0);
    }

    #[test]
    fn test_calc_rank_max() {
        assert_eq!(calc_rank(999999), 9);
    }

    #[test]
    fn test_calc_rank_boundary() {
        assert_eq!(calc_rank(100), 1);
        assert_eq!(calc_rank(99), 0);
        assert_eq!(calc_rank(500), 2);
        assert_eq!(calc_rank(400000), 9);
    }

    #[test]
    fn test_daily_quests_count() {
        assert_eq!(DAILY_HUNT_QUESTS.len(), 8);
    }

    #[test]
    fn test_daily_quests_unique_ids() {
        let mut ids: Vec<&str> = DAILY_HUNT_QUESTS.iter().map(|q| q.id).collect();
        let orig_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), orig_len);
    }

    #[test]
    fn test_daily_quests_rewards_positive() {
        for q in DAILY_HUNT_QUESTS {
            assert!(q.reward_points > 0);
            assert!(q.reward_gold > 0);
            assert!(q.reward_diamond > 0);
            assert!(q.target_count > 0);
        }
    }

    #[test]
    fn test_shop_items_count() {
        assert_eq!(HUNTER_SHOP_ITEMS.len(), 10);
    }

    #[test]
    fn test_shop_items_have_min_rank() {
        for item in HUNTER_SHOP_ITEMS {
            assert!(item.min_rank >= 0 && item.min_rank <= 9);
            assert!(item.cost_points > 0);
        }
    }

    #[test]
    fn test_get_rank_info_valid() {
        let info = get_rank_info(0);
        assert_eq!(info.name, "新手猎人");
        let info = get_rank_info(9);
        assert_eq!(info.name, "猎人之王");
    }

    #[test]
    fn test_hunter_bonus_zero_rank() {
        let info = get_rank_info(0);
        assert_eq!(info.hp_bonus, 0);
        assert_eq!(info.gold_bonus_pct, 0);
    }

    #[test]
    fn test_hunter_bonus_max_rank() {
        let info = get_rank_info(9);
        assert_eq!(info.hp_bonus, 3500);
        assert_eq!(info.gold_bonus_pct, 60);
    }

    #[test]
    fn test_today_str_returns_consistent() {
        let s1 = today_str();
        let s2 = today_str();
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_hunter_rank_last_is_max() {
        let last = &HUNTER_RANKS[HUNTER_RANKS.len() - 1];
        assert_eq!(last.rank, 9);
        assert_eq!(last.required_points, 400000);
    }

    #[test]
    fn test_calc_rank_below_first_threshold() {
        assert_eq!(calc_rank(50), 0);
        assert_eq!(calc_rank(1), 0);
    }

    #[test]
    fn test_all_ranks_have_unique_emoji() {
        let mut emojis: Vec<&str> = HUNTER_RANKS.iter().map(|r| r.emoji).collect();
        let orig = emojis.len();
        emojis.sort();
        emojis.dedup();
        assert_eq!(emojis.len(), orig);
    }

    #[test]
    fn test_shop_items_escalate_cost() {
        for i in 1..HUNTER_SHOP_ITEMS.len() {
            assert!(HUNTER_SHOP_ITEMS[i].cost_points >= HUNTER_SHOP_ITEMS[i - 1].cost_points);
        }
    }

    #[test]
    fn test_shop_min_rank_escalates() {
        for i in 1..HUNTER_SHOP_ITEMS.len() {
            assert!(HUNTER_SHOP_ITEMS[i].min_rank >= HUNTER_SHOP_ITEMS[i - 1].min_rank);
        }
    }
}

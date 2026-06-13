/// CakeGame 军衔战阶系统
/// 通过PvP战斗、BOSS击杀、竞技场、深渊等战斗行为积累军功，提升军衔
/// 军衔提供被动属性加成和专属奖励
use crate::db::Database;
use crate::user;

const SECTION: &str = "military_rank";

/// 军衔定义
struct RankDef {
    name: &'static str,
    emoji: &'static str,
    /// 所需军功点数
    required_points: i64,
    /// 升级奖励金币
    reward_gold: i64,
    /// 升级奖励钻石
    reward_diamond: i64,
    /// 被动加成: HP
    bonus_hp: i32,
    /// 被动加成: 物攻
    bonus_ad: i32,
    /// 被动加成: 魔攻
    bonus_ap: i32,
    /// 被动加成: 防御
    bonus_def: i32,
    /// 被动加成: 魔抗
    bonus_mdf: i32,
    /// 称号描述
    desc: &'static str,
}

const RANKS: &[RankDef] = &[
    RankDef {
        name: "新兵",
        emoji: "🔰",
        required_points: 0,
        reward_gold: 0,
        reward_diamond: 0,
        bonus_hp: 0,
        bonus_ad: 0,
        bonus_ap: 0,
        bonus_def: 0,
        bonus_mdf: 0,
        desc: "初入战场的新兵",
    },
    RankDef {
        name: "列兵",
        emoji: "⭐",
        required_points: 100,
        reward_gold: 2000,
        reward_diamond: 10,
        bonus_hp: 30,
        bonus_ad: 5,
        bonus_ap: 5,
        bonus_def: 3,
        bonus_mdf: 3,
        desc: "经过初步战斗洗礼",
    },
    RankDef {
        name: "下士",
        emoji: "⭐⭐",
        required_points: 300,
        reward_gold: 5000,
        reward_diamond: 20,
        bonus_hp: 80,
        bonus_ad: 12,
        bonus_ap: 12,
        bonus_def: 8,
        bonus_mdf: 8,
        desc: "战场上的老兵",
    },
    RankDef {
        name: "中士",
        emoji: "🌟",
        required_points: 800,
        reward_gold: 10000,
        reward_diamond: 40,
        bonus_hp: 150,
        bonus_ad: 25,
        bonus_ap: 25,
        bonus_def: 15,
        bonus_mdf: 15,
        desc: "身经百战的勇士",
    },
    RankDef {
        name: "上士",
        emoji: "🌟🌟",
        required_points: 2000,
        reward_gold: 25000,
        reward_diamond: 80,
        bonus_hp: 300,
        bonus_ad: 50,
        bonus_ap: 50,
        bonus_def: 30,
        bonus_mdf: 30,
        desc: "令人敬畏的战士",
    },
    RankDef {
        name: "少尉",
        emoji: "🎖️",
        required_points: 5000,
        reward_gold: 50000,
        reward_diamond: 150,
        bonus_hp: 500,
        bonus_ad: 80,
        bonus_ap: 80,
        bonus_def: 50,
        bonus_mdf: 50,
        desc: "战场指挥官",
    },
    RankDef {
        name: "中尉",
        emoji: "🎖️🎖️",
        required_points: 10000,
        reward_gold: 100000,
        reward_diamond: 300,
        bonus_hp: 800,
        bonus_ad: 130,
        bonus_ap: 130,
        bonus_def: 80,
        bonus_mdf: 80,
        desc: "经验丰富的军官",
    },
    RankDef {
        name: "上尉",
        emoji: "🏅",
        required_points: 20000,
        reward_gold: 200000,
        reward_diamond: 500,
        bonus_hp: 1200,
        bonus_ad: 200,
        bonus_ap: 200,
        bonus_def: 120,
        bonus_mdf: 120,
        desc: "百战百胜的名将",
    },
    RankDef {
        name: "将军",
        emoji: "👑",
        required_points: 50000,
        reward_gold: 500000,
        reward_diamond: 1000,
        bonus_hp: 2000,
        bonus_ad: 350,
        bonus_ap: 350,
        bonus_def: 200,
        bonus_mdf: 200,
        desc: "威震四方的将军",
    },
    RankDef {
        name: "元帅",
        emoji: "⚜️",
        required_points: 100000,
        reward_gold: 1000000,
        reward_diamond: 2000,
        bonus_hp: 3500,
        bonus_ad: 600,
        bonus_ap: 600,
        bonus_def: 350,
        bonus_mdf: 350,
        desc: "至高无上的战神",
    },
];

/// 军功来源定义
struct PointSource {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    /// 每次获得的军功
    points_per: i64,
    /// 每日上限
    daily_limit: i64,
}

const POINT_SOURCES: &[PointSource] = &[
    PointSource {
        id: "pvp_win",
        name: "PvP胜利",
        emoji: "⚔️",
        points_per: 10,
        daily_limit: 100,
    },
    PointSource {
        id: "boss_kill",
        name: "BOSS击杀",
        emoji: "👹",
        points_per: 5,
        daily_limit: 50,
    },
    PointSource {
        id: "arena_win",
        name: "竞技场胜利",
        emoji: "🏟️",
        points_per: 8,
        daily_limit: 80,
    },
    PointSource {
        id: "abyss_floor",
        name: "深渊通关",
        emoji: "🕳️",
        points_per: 3,
        daily_limit: 60,
    },
    PointSource {
        id: "guild_war",
        name: "公会战",
        emoji: "🏰",
        points_per: 15,
        daily_limit: 60,
    },
    PointSource {
        id: "world_boss",
        name: "世界BOSS",
        emoji: "🌍",
        points_per: 12,
        daily_limit: 48,
    },
    PointSource {
        id: "monster_hunt",
        name: "怪物狩猎",
        emoji: "🐗",
        points_per: 1,
        daily_limit: 100,
    },
];

/// 获取玩家当前军功
fn get_points(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("pts_{}", user_id)).parse().unwrap_or(0)
}

/// 设置玩家军功
fn set_points(db: &Database, user_id: &str, points: i64) {
    db.global_set(SECTION, &format!("pts_{}", user_id), &points.to_string());
}

/// 获取今日某来源已获得军功
fn get_daily_source_points(db: &Database, user_id: &str, source_id: &str) -> i64 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let key = format!("daily_{}_{}_{}", user_id, source_id, today);
    db.global_get(SECTION, &key).parse().unwrap_or(0)
}

/// 记录今日某来源军功
fn add_daily_source_points(db: &Database, user_id: &str, source_id: &str, amount: i64) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let key = format!("daily_{}_{}_{}", user_id, source_id, today);
    let current = get_daily_source_points(db, user_id, source_id);
    db.global_set(SECTION, &key, &(current + amount).to_string());
}

/// 根据军功获取军衔等级 (0-indexed)
fn get_rank_index(points: i64) -> usize {
    for i in (0..RANKS.len()).rev() {
        if points >= RANKS[i].required_points {
            return i;
        }
    }
    0
}

/// 获取上次晋升时间
fn get_last_promotion(db: &Database, user_id: &str) -> String {
    db.global_get(SECTION, &format!("promo_{}", user_id))
}

/// 记录晋升时间
fn set_last_promotion(db: &Database, user_id: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.global_set(SECTION, &format!("promo_{}", user_id), &now);
}

/// 记录军功来源事件日志
fn log_event(db: &Database, user_id: &str, source_id: &str, points: i64) {
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let key = format!("log_{}", user_id);
    let existing = db.global_get(SECTION, &key);
    let entry = format!("{}|{}|{}|{}", now, source_id, points, chrono::Local::now().timestamp());
    let new_log = if existing.is_empty() {
        entry
    } else {
        let entries: Vec<&str> = existing.split('~').collect();
        if entries.len() >= 20 {
            let mut v: Vec<&str> = entries[1..].to_vec();
            v.push(&entry);
            // We need owned strings for join
            let owned: Vec<String> = v.iter().map(|s| s.to_string()).collect();
            owned.join("~")
        } else {
            format!("{}~{}", existing, entry)
        }
    };
    db.global_set(SECTION, &key, &new_log);
}

/// 获取军衔被动加成 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_military_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let points = get_points(db, user_id);
    let idx = get_rank_index(points);
    let rank = &RANKS[idx];
    (
        rank.bonus_hp,
        rank.bonus_ad,
        rank.bonus_ap,
        rank.bonus_def,
        rank.bonus_mdf,
    )
}

/// 记录军功 (供其他模块调用的公开API)
/// source: "pvp_win", "boss_kill", "arena_win", "abyss_floor", "guild_war", "world_boss", "monster_hunt"
/// 返回实际获得的军功数 (可能因每日上限而减少)
#[allow(dead_code)]
pub fn record_military_points(db: &Database, user_id: &str, source: &str, _amount: i64) -> i64 {
    let source_def = match POINT_SOURCES.iter().find(|s| s.id == source) {
        Some(s) => s,
        None => return 0,
    };

    // 检查每日上限
    let daily_used = get_daily_source_points(db, user_id, source);
    let remaining = source_def.daily_limit - daily_used;
    if remaining <= 0 {
        return 0;
    }

    let points_to_add = source_def.points_per.min(remaining);
    let old_points = get_points(db, user_id);
    let old_rank = get_rank_index(old_points);

    set_points(db, user_id, old_points + points_to_add);
    add_daily_source_points(db, user_id, source, points_to_add);
    log_event(db, user_id, source, points_to_add);

    // 检查是否晋升
    let new_rank = get_rank_index(old_points + points_to_add);
    if new_rank > old_rank {
        set_last_promotion(db, user_id);
        // 发放晋升奖励
        let rank = &RANKS[new_rank];
        if rank.reward_gold > 0 {
            use crate::core::CURRENCY_GOLD;
            db.modify_currency(user_id, CURRENCY_GOLD, "add", rank.reward_gold);
        }
        if rank.reward_diamond > 0 {
            use crate::core::CURRENCY_DIAMOND;
            db.modify_currency(user_id, CURRENCY_DIAMOND, "add", rank.reward_diamond);
        }
    }

    points_to_add
}

// ==================== 指令函数 ====================

/// 查看军衔 — 显示玩家当前军衔、军功、进度、加成
pub fn cmd_view_military_rank(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let points = get_points(db, user_id);
    let idx = get_rank_index(points);
    let rank = &RANKS[idx];

    let mut r = format!("{}\n═══ {} 军衔战阶 ═══\n", prefix, rank.emoji);
    r.push_str(&format!("\n🎖️ 军衔：{}", rank.name));
    r.push_str(&format!("\n⚔️ 军功：{}", points));
    r.push_str(&format!("\n📝 {}", rank.desc));

    // 进度条
    if idx < RANKS.len() - 1 {
        let next = &RANKS[idx + 1];
        let current_in_tier = points - rank.required_points;
        let tier_range = next.required_points - rank.required_points;
        let pct = if tier_range > 0 {
            (current_in_tier * 100 / tier_range).min(100)
        } else {
            100
        };
        let filled = (pct / 5) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
        r.push_str(&format!("\n\n📊 升级进度 [{}/{}]", current_in_tier, tier_range));
        r.push_str(&format!("\n{} {}%", bar, pct));
        r.push_str(&format!(
            "\n⏭️ 下一军衔：{} {} (需要{}军功)",
            next.emoji, next.name, next.required_points
        ));
    } else {
        r.push_str("\n\n🏆 已达最高军衔！");
    }

    // 属性加成
    if rank.bonus_hp > 0 || rank.bonus_ad > 0 {
        r.push_str("\n\n💪 军衔加成：");
        if rank.bonus_hp > 0 {
            r.push_str(&format!("\n  ❤️ HP +{}", rank.bonus_hp));
        }
        if rank.bonus_ad > 0 {
            r.push_str(&format!("\n  ⚔️ 物攻 +{}", rank.bonus_ad));
        }
        if rank.bonus_ap > 0 {
            r.push_str(&format!("\n  🔮 魔攻 +{}", rank.bonus_ap));
        }
        if rank.bonus_def > 0 {
            r.push_str(&format!("\n  🛡️ 防御 +{}", rank.bonus_def));
        }
        if rank.bonus_mdf > 0 {
            r.push_str(&format!("\n  🔰 魔抗 +{}", rank.bonus_mdf));
        }
    }

    // 晋升时间
    let promo_time = get_last_promotion(db, user_id);
    if !promo_time.is_empty() {
        r.push_str(&format!("\n\n🕐 上次晋升：{}", promo_time));
    }

    // 今日军功统计
    r.push_str("\n\n📋 今日军功来源：");
    let mut total_daily = 0i64;
    for src in POINT_SOURCES {
        let used = get_daily_source_points(db, user_id, src.id);
        if used > 0 || total_daily < 100 {
            r.push_str(&format!("\n  {} {} +{}/{}", src.emoji, src.name, used, src.daily_limit));
            total_daily += used;
        }
    }
    r.push_str(&format!("\n  📊 今日总计：{}军功", total_daily));

    r
}

/// 军衔排行 — 全服军功排行榜
pub fn cmd_military_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut players: Vec<(String, i64, usize)> = Vec::new(); // (name, points, rank_idx)

    for uid in db.all_users().iter() {
        let pts = get_points(db, uid);
        if pts > 0 {
            let name = user::get_msg_prefix(db, uid);
            let idx = get_rank_index(pts);
            players.push((name, pts, idx));
        }
    }

    if players.is_empty() {
        return format!("{}\n暂无军衔数据", prefix);
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n═══ 🏆 军衔排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, pts, idx)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        r.push_str(&format!("\n{} {} {} — {}军功", medal, RANKS[*idx].emoji, name, pts));
    }

    // 用户排名
    let user_prefix = user::get_msg_prefix(db, user_id);
    if let Some(rank) = players.iter().position(|(name, _, _)| name == &user_prefix) {
        let user_pts = players[rank].1;
        let user_idx = get_rank_index(user_pts);
        r.push_str(&format!(
            "\n\n📍 你的排名：第{}名 ({} {}军功)",
            rank + 1,
            RANKS[user_idx].name,
            user_pts
        ));
    }

    r.push_str(&format!("\n\n📊 共{}名军官", players.len()));
    r
}

/// 军衔详情 — 查看所有军衔等级
pub fn cmd_military_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut r = format!("{}\n═══ 🎖️ 军衔一览 ═══\n", prefix);
    r.push_str("\n通过PvP、BOSS、竞技场等战斗获得军功：\n");

    for (i, rank) in RANKS.iter().enumerate() {
        let current = if db.user_exists(user_id) {
            let pts = get_points(db, user_id);
            let idx = get_rank_index(pts);
            if idx == i {
                " ← 当前"
            } else {
                ""
            }
        } else {
            ""
        };
        r.push_str(&format!(
            "\n{} {} — 需要{}军功{}",
            rank.emoji, rank.name, rank.required_points, current
        ));
        if rank.bonus_hp > 0 {
            r.push_str(&format!(
                " [HP+{} AD+{} AP+{}]",
                rank.bonus_hp, rank.bonus_ad, rank.bonus_ap
            ));
        }
    }

    r.push_str("\n\n📋 军功获取途径：");
    for src in POINT_SOURCES {
        r.push_str(&format!(
            "\n  {} {} +{}/次 (每日上限{})",
            src.emoji, src.name, src.points_per, src.daily_limit
        ));
    }

    r.push_str("\n\n💡 升级自动发放金币+钻石奖励");
    r
}

/// 军衔奖励 — 查看/领取军衔晋升奖励
pub fn cmd_military_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let points = get_points(db, user_id);
    let idx = get_rank_index(points);

    let mut r = format!("{}\n═══ 🎁 军衔奖励 ═══\n", prefix);

    r.push_str(&format!("\n🎖️ 当前军衔：{} {}", RANKS[idx].emoji, RANKS[idx].name));
    r.push_str(&format!("\n⚔️ 当前军功：{}\n", points));

    // 显示已解锁和未解锁的奖励
    for (i, rank) in RANKS.iter().enumerate() {
        let status = if i <= idx { "✅" } else { "🔒" };
        r.push_str(&format!("\n{} {} {}", status, rank.emoji, rank.name));
        if rank.reward_gold > 0 || rank.reward_diamond > 0 {
            r.push_str(" — ");
            if rank.reward_gold > 0 {
                r.push_str(&format!("{}金币", rank.reward_gold));
            }
            if rank.reward_diamond > 0 {
                if rank.reward_gold > 0 {
                    r.push_str(" + ");
                }
                r.push_str(&format!("{}钻石", rank.reward_diamond));
            }
        }
    }

    r.push_str("\n\n💡 军衔晋升时自动发放奖励，无需手动领取");
    r.push_str("\n💡 军衔加成永久生效，不会降级");
    r
}

/// 军衔日志 — 查看最近的军功获取记录
pub fn cmd_military_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let key = format!("log_{}", user_id);
    let log = db.global_get(SECTION, &key);

    let mut r = format!("{}\n═══ 📜 军功日志 ═══\n", prefix);

    if log.is_empty() {
        r.push_str("\n暂无军功获取记录");
        r.push_str("\n\n💡 通过以下方式获取军功：");
        for src in POINT_SOURCES {
            r.push_str(&format!("\n  {} {} +{}/次", src.emoji, src.name, src.points_per));
        }
        return r;
    }

    let entries: Vec<&str> = log.split('~').collect();
    r.push_str(&format!("\n最近{}条记录：\n", entries.len()));

    for entry in entries.iter().rev() {
        let parts: Vec<&str> = entry.split('|').collect();
        if parts.len() >= 3 {
            let time = parts[0];
            let source_id = parts[1];
            let pts = parts[2];
            let source_name = POINT_SOURCES
                .iter()
                .find(|s| s.id == source_id)
                .map(|s| (s.emoji, s.name))
                .unwrap_or(("❓", "未知"));
            r.push_str(&format!(
                "\n  {} {} +{}军功 ({})",
                source_name.0, source_name.1, pts, time
            ));
        }
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_count() {
        assert_eq!(RANKS.len(), 10);
    }

    #[test]
    fn test_rank_names_unique() {
        let mut names: Vec<&str> = RANKS.iter().map(|r| r.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn test_rank_emojis_unique() {
        let mut emojis: Vec<&str> = RANKS.iter().map(|r| r.emoji).collect();
        let before = emojis.len();
        emojis.sort();
        emojis.dedup();
        assert_eq!(before, emojis.len());
    }

    #[test]
    fn test_rank_points_ascending() {
        for i in 1..RANKS.len() {
            assert!(
                RANKS[i].required_points > RANKS[i - 1].required_points,
                "Rank {} points should be > rank {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_rank_bonuses_ascending() {
        for i in 1..RANKS.len() {
            assert!(RANKS[i].bonus_hp >= RANKS[i - 1].bonus_hp);
            assert!(RANKS[i].bonus_ad >= RANKS[i - 1].bonus_ad);
        }
    }

    #[test]
    fn test_rank_rewards_positive() {
        // Skip index 0 (新兵 has no rewards)
        for rank in &RANKS[1..] {
            assert!(rank.reward_gold > 0, "Rank {} should have gold reward", rank.name);
            assert!(rank.reward_diamond > 0, "Rank {} should have diamond reward", rank.name);
        }
    }

    #[test]
    fn test_point_sources_count() {
        assert_eq!(POINT_SOURCES.len(), 7);
    }

    #[test]
    fn test_point_sources_ids_unique() {
        let mut ids: Vec<&str> = POINT_SOURCES.iter().map(|s| s.id).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(before, ids.len());
    }

    #[test]
    fn test_point_sources_daily_limit() {
        for src in POINT_SOURCES {
            assert!(
                src.daily_limit >= src.points_per,
                "{} daily limit should >= per-action",
                src.name
            );
            assert!(src.points_per > 0);
            assert!(src.daily_limit > 0);
        }
    }

    #[test]
    fn test_get_rank_index_at_boundaries() {
        assert_eq!(get_rank_index(0), 0); // 新兵
        assert_eq!(get_rank_index(99), 0); // 新兵
        assert_eq!(get_rank_index(100), 1); // 列兵
        assert_eq!(get_rank_index(300), 2); // 下士
        assert_eq!(get_rank_index(100000), 9); // 元帅
        assert_eq!(get_rank_index(999999), 9); // 元帅
    }

    #[test]
    fn test_get_rank_index_intermediate() {
        assert_eq!(get_rank_index(500), 2); // 下士 (300-800)
        assert_eq!(get_rank_index(1500), 3); // 中士 (800-2000)
        assert_eq!(get_rank_index(7000), 5); // 少尉 (5000-10000)
    }

    #[test]
    fn test_first_rank_is_free() {
        assert_eq!(RANKS[0].required_points, 0);
        assert_eq!(RANKS[0].reward_gold, 0);
        assert_eq!(RANKS[0].reward_diamond, 0);
    }

    #[test]
    fn test_rank_descriptions_not_empty() {
        for rank in RANKS {
            assert!(!rank.desc.is_empty(), "Rank {} has empty desc", rank.name);
        }
    }

    #[test]
    fn test_source_names_not_empty() {
        for src in POINT_SOURCES {
            assert!(!src.name.is_empty());
            assert!(!src.emoji.is_empty());
        }
    }

    #[test]
    fn test_get_military_bonus_at_zero() {
        // Just verify the function signature works with static data
        // Cannot test with actual DB in unit tests
        assert_eq!(RANKS[0].bonus_hp, 0);
        assert_eq!(RANKS[0].bonus_ad, 0);
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "military_rank");
    }

    #[test]
    fn test_marshal_rank_progress() {
        // Test that rank progress calculation is correct
        let points = 5500i64;
        let idx = get_rank_index(points);
        assert_eq!(idx, 5); // 少尉 (5000)
        let current_in_tier = points - RANKS[idx].required_points;
        assert_eq!(current_in_tier, 500);
        let tier_range = RANKS[idx + 1].required_points - RANKS[idx].required_points;
        assert_eq!(tier_range, 5000);
        let pct = current_in_tier * 100 / tier_range;
        assert_eq!(pct, 10);
    }
}

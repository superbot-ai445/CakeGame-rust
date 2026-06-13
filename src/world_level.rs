/// CakeGame 世界等级系统
///
/// 社区驱动的全局等级系统，随全服玩家进步自动提升:
/// - 世界等级基于全服玩家平均等级自动计算
/// - 等级越高 → 怪物更强但掉落更丰厚
/// - 低等级玩家获得追赶加成（经验/金币倍率提升）
/// - 每日世界等级最多提升1级，防止暴涨
/// - 世界等级里程碑奖励: 每5级发放全服奖励
///
/// 指令: 世界等级, 世界等级详情, 世界等级排行, 世界等级奖励, 领取世界奖励, 世界等级历史
use crate::db::Database;

/// 世界等级阶段定义
struct WorldLevelTier {
    min_level: i32,
    name: &'static str,
    emoji: &'static str,
    desc: &'static str,
    /// (怪物HP加成%, 怪物攻击加成%, 掉率加成%, 经验加成%, 金币加成%)
    modifier: (i32, i32, i32, i32, i32),
    /// 追赶加成: 低于世界等级多少级触发, (经验追赶%, 金币追赶%)
    catchup_threshold: i32,
    catchup_bonus: (i32, i32),
}

const WORLD_LEVEL_TIERS: &[WorldLevelTier] = &[
    WorldLevelTier {
        min_level: 1,
        name: "初生之犊",
        emoji: "🌱",
        desc: "服务器刚刚起步，怪物温和，适合新手探索",
        modifier: (0, 0, 0, 0, 0),
        catchup_threshold: 0,
        catchup_bonus: (0, 0),
    },
    WorldLevelTier {
        min_level: 5,
        name: "崭露头角",
        emoji: "🌿",
        desc: "冒险者们逐渐成长，世界开始展现危险的一面",
        modifier: (10, 10, 5, 10, 10),
        catchup_threshold: 3,
        catchup_bonus: (20, 15),
    },
    WorldLevelTier {
        min_level: 10,
        name: "群雄逐鹿",
        emoji: "⚔️",
        desc: "强者辈出，怪物也在进化，丰厚奖励等待勇者",
        modifier: (25, 20, 10, 20, 20),
        catchup_threshold: 5,
        catchup_bonus: (40, 30),
    },
    WorldLevelTier {
        min_level: 20,
        name: "风云际会",
        emoji: "🌪️",
        desc: "大陆风起云涌，精英怪物出现，稀有掉落概率提升",
        modifier: (50, 40, 20, 35, 35),
        catchup_threshold: 8,
        catchup_bonus: (60, 50),
    },
    WorldLevelTier {
        min_level: 30,
        name: "龙虎争霸",
        emoji: "🐉",
        desc: "传说中的怪物现身，击败它们可获得神器级掉落",
        modifier: (80, 60, 30, 50, 50),
        catchup_threshold: 10,
        catchup_bonus: (80, 65),
    },
    WorldLevelTier {
        min_level: 40,
        name: "霸主时代",
        emoji: "👑",
        desc: "最强者掌控世界，BOSS怪物拥有传奇技能",
        modifier: (120, 90, 45, 70, 70),
        catchup_threshold: 12,
        catchup_bonus: (100, 80),
    },
    WorldLevelTier {
        min_level: 50,
        name: "诸神黄昏",
        emoji: "⚡",
        desc: "神话级威胁降临，只有最强的冒险者才能生存",
        modifier: (180, 130, 65, 100, 100),
        catchup_threshold: 15,
        catchup_bonus: (150, 100),
    },
];

/// 世界等级里程碑
struct WorldMilestone {
    level: i32,
    name: &'static str,
    reward_gold: i32,
    reward_diamond: i32,
    reward_item: &'static str,
}

const WORLD_MILESTONES: &[WorldMilestone] = &[
    WorldMilestone {
        level: 5,
        name: "初露锋芒",
        reward_gold: 5000,
        reward_diamond: 50,
        reward_item: "强化石",
    },
    WorldMilestone {
        level: 10,
        name: "小有成就",
        reward_gold: 15000,
        reward_diamond: 100,
        reward_item: "高级强化石",
    },
    WorldMilestone {
        level: 15,
        name: "渐入佳境",
        reward_gold: 30000,
        reward_diamond: 200,
        reward_item: "凤凰之羽",
    },
    WorldMilestone {
        level: 20,
        name: "势不可挡",
        reward_gold: 50000,
        reward_diamond: 350,
        reward_item: "时空精华",
    },
    WorldMilestone {
        level: 25,
        name: "王者崛起",
        reward_gold: 80000,
        reward_diamond: 500,
        reward_item: "传说宝箱",
    },
    WorldMilestone {
        level: 30,
        name: "大陆霸主",
        reward_gold: 120000,
        reward_diamond: 800,
        reward_item: "神器碎片",
    },
    WorldMilestone {
        level: 40,
        name: "神话时代",
        reward_gold: 200000,
        reward_diamond: 1200,
        reward_item: "神话宝箱",
    },
    WorldMilestone {
        level: 50,
        name: "诸神黄昏",
        reward_gold: 500000,
        reward_diamond: 2000,
        reward_item: "创世之石",
    },
];

/// 获取当前世界等级阶段
fn get_tier_for_level(level: i32) -> &'static WorldLevelTier {
    let mut tier = &WORLD_LEVEL_TIERS[0];
    for t in WORLD_LEVEL_TIERS {
        if level >= t.min_level {
            tier = t;
        }
    }
    tier
}

/// 计算全服平均等级
fn calc_server_avg_level(db: &Database) -> f64 {
    let users = db.all_users();
    if users.is_empty() {
        return 1.0;
    }
    let mut total: i64 = 0;
    let mut count: i64 = 0;
    for uid in &users {
        let lvl: i32 = db.read_basic(uid, "level").parse().unwrap_or(1);
        total += lvl as i64;
        count += 1;
    }
    if count == 0 {
        1.0
    } else {
        total as f64 / count as f64
    }
}

/// 计算世界等级 (基于全服平均等级, 每日最多+1)
pub fn calc_world_level(db: &Database) -> i32 {
    let avg = calc_server_avg_level(db);
    let raw_level = (avg * 0.8).floor() as i32;
    let current = get_current_world_level(db);

    // 每日最多提升1级
    if raw_level > current + 1 {
        current + 1
    } else if raw_level < 1 {
        1
    } else {
        raw_level
    }
}

/// 获取当前存储的世界等级
pub fn get_current_world_level(db: &Database) -> i32 {
    db.global_get("world_level", "level").parse().unwrap_or(1)
}

/// 更新世界等级 (每次计算时调用)
pub fn update_world_level(db: &Database) -> i32 {
    let new_level = calc_world_level(db);
    let old_level = get_current_world_level(db);

    if new_level > old_level {
        db.global_set("world_level", "level", &new_level.to_string());
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        db.global_set("world_level", "last_update", &now);

        // 记录升级历史
        let history = db.global_get("world_level", "history");
        let entry = format!("{}→{}@{}", old_level, new_level, chrono::Local::now().format("%m-%d"));
        let new_history = if history.is_empty() {
            entry
        } else {
            format!("{}|{}", history, entry)
        };
        // 只保留最近20条
        let parts: Vec<&str> = new_history.split('|').collect();
        let trimmed = if parts.len() > 20 {
            parts[parts.len() - 20..].join("|")
        } else {
            new_history
        };
        db.global_set("world_level", "history", &trimmed);
    }

    new_level
}

/// 获取怪物加成 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_monster_bonus(db: &Database) -> (i32, i32) {
    let level = get_current_world_level(db);
    let tier = get_tier_for_level(level);
    (tier.modifier.0, tier.modifier.1)
}

/// 获取掉落加成 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_drop_bonus(db: &Database) -> i32 {
    let level = get_current_world_level(db);
    let tier = get_tier_for_level(level);
    tier.modifier.2
}

/// 获取追赶加成 (供经验/金币系统调用)
#[allow(dead_code)]
pub fn get_catchup_bonus(db: &Database, user_id: &str) -> (i32, i32) {
    let world_level = get_current_world_level(db);
    let tier = get_tier_for_level(world_level);

    if tier.catchup_threshold <= 0 {
        return (0, 0);
    }

    let user_level: i32 = db.read_basic(user_id, "level").parse().unwrap_or(1);
    let diff = world_level - user_level;

    if diff >= tier.catchup_threshold {
        tier.catchup_bonus
    } else if diff > 0 {
        let ratio = diff as f64 / tier.catchup_threshold as f64;
        (
            (tier.catchup_bonus.0 as f64 * ratio) as i32,
            (tier.catchup_bonus.1 as f64 * ratio) as i32,
        )
    } else {
        (0, 0)
    }
}

/// 获取世界等级带来的经验/金币加成
#[allow(dead_code)]
pub fn get_world_bonus(db: &Database) -> (i32, i32) {
    let level = get_current_world_level(db);
    let tier = get_tier_for_level(level);
    (tier.modifier.3, tier.modifier.4)
}

// ==================== 指令实现 ====================

/// 查看世界等级
pub fn cmd_world_level(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let level = update_world_level(db);
    let tier = get_tier_for_level(level);
    let avg = calc_server_avg_level(db);
    let user_level: i32 = db.read_basic(user_id, "level").parse().unwrap_or(1);

    let status = if user_level >= level {
        "✅ 已达标"
    } else {
        "⬆️ 需努力"
    };

    let mut r = format!("{}\n", prefix);
    r.push_str("══════ 世界等级 ══════\n\n");
    r.push_str(&format!("{} {} Lv.{}\n", tier.emoji, tier.name, level));
    r.push_str(&format!("{}\n\n", tier.desc));
    r.push_str(&format!("📊 全服平均等级: {:.1}\n", avg));
    r.push_str(&format!(
        "🎯 你的等级: Lv.{} {}\n\n",
        user_level, status
    ));

    // 怪物变化
    r.push_str("⚔️ 怪物变化:\n");
    r.push_str(&format!(
        "   HP: +{}%  攻击: +{}%\n",
        tier.modifier.0, tier.modifier.1
    ));

    // 全服加成
    r.push_str(&format!(
        "\n🌟 全服加成: 经验+{}% 金币+{}% 掉率+{}%\n",
        tier.modifier.3, tier.modifier.4, tier.modifier.2
    ));

    // 追赶加成
    if tier.catchup_threshold > 0 && user_level < level {
        let diff = level - user_level;
        if diff >= tier.catchup_threshold {
            r.push_str(&format!(
                "🚀 追赶加成已激活: 经验+{}% 金币+{}%\n",
                tier.catchup_bonus.0, tier.catchup_bonus.1
            ));
        } else {
            r.push_str(&format!(
                "💤 追赶加成未激活 (差距需≥{}级, 当前差{}级)\n",
                tier.catchup_threshold, diff
            ));
        }
    }

    // 下一级所需
    if level < 50 {
        let next_avg = (level + 1) as f64 / 0.8;
        r.push_str(&format!("\n📈 下一级需全服平均等级: {:.1}\n", next_avg));
    } else {
        r.push_str("\n🏆 已达世界等级上限！\n");
    }

    r
}

/// 世界等级详情
pub fn cmd_world_level_detail(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let level = get_current_world_level(db);

    let mut r = format!("{}\n", prefix);
    r.push_str("══════ 世界等级详情 ══════\n\n");
    r.push_str("📋 全部等级阶段:\n\n");

    for tier in WORLD_LEVEL_TIERS {
        let marker = if level >= tier.min_level {
            "✅"
        } else {
            "🔒"
        };
        r.push_str(&format!(
            "{} {} Lv.{}+ 「{}」\n",
            marker, tier.emoji, tier.min_level, tier.name
        ));
        r.push_str(&format!(
            "   怪物: HP+{}% 攻击+{}%  掉率+{}%\n",
            tier.modifier.0, tier.modifier.1, tier.modifier.2
        ));
        r.push_str(&format!(
            "   加成: 经验+{}% 金币+{}%\n",
            tier.modifier.3, tier.modifier.4
        ));
        if tier.catchup_threshold > 0 {
            r.push_str(&format!(
                "   追赶: 差距≥{}级 → 经验+{}% 金币+{}%\n",
                tier.catchup_threshold, tier.catchup_bonus.0, tier.catchup_bonus.1
            ));
        }
        r.push('\n');
    }

    r.push_str("━━━━━━━━━━━━━━━━━━\n");
    r.push_str("💡 世界等级 = 全服平均等级 × 0.8\n");
    r.push_str("💡 每日最多提升1级，防止等级暴涨\n");
    r.push_str("💡 低等级玩家自动获得追赶加成\n");

    r
}

/// 世界等级排行
pub fn cmd_world_level_ranking(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let world_level = get_current_world_level(db);

    // 获取所有用户等级并排序
    let all = db.all_users();
    let mut users: Vec<(String, i32)> = all
        .iter()
        .map(|uid| {
            let lvl: i32 = db.read_basic(uid, "level").parse().unwrap_or(1);
            (uid.clone(), lvl)
        })
        .collect();
    users.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n", prefix);
    r.push_str("══════ 世界等级贡献榜 ══════\n\n");
    r.push_str(&format!("🌍 当前世界等级: Lv.{}\n\n", world_level));
    r.push_str("🏆 等级贡献排行:\n");

    let mut user_rank = 0usize;
    for (i, (uid, level)) in users.iter().take(15).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let marker = if uid == user_id { " ← 你" } else { "" };
        r.push_str(&format!(
            "{} #{} Lv.{} {}{}\n",
            medal,
            i + 1,
            level,
            uid,
            marker
        ));
        if uid == user_id {
            user_rank = i + 1;
        }
    }

    if user_rank == 0 {
        for (i, (uid, level)) in users.iter().enumerate() {
            if uid == user_id {
                user_rank = i + 1;
                r.push_str(&format!(
                    "\n📍 你的排名: #{} Lv.{}\n",
                    user_rank, level
                ));
                break;
            }
        }
    }

    let total = users.len();
    let avg = calc_server_avg_level(db);
    r.push_str(&format!(
        "\n📊 全服统计: {}名玩家, 平均等级 {:.1}\n",
        total, avg
    ));

    r
}

/// 世界等级奖励
pub fn cmd_world_level_reward(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let level = get_current_world_level(db);

    let mut r = format!("{}\n", prefix);
    r.push_str("══════ 世界等级奖励 ══════\n\n");

    for ms in WORLD_MILESTONES {
        let status = if level >= ms.level {
            let key = format!("wl_milestone_{}", ms.level);
            let claimed = db.global_get("world_level_rewards", &key);
            if claimed == "1" {
                "✅ 已领取"
            } else {
                "🎁 可领取"
            }
        } else {
            "🔒 未达成"
        };

        let progress = if level < ms.level {
            format!(" ({}/{})", level, ms.level)
        } else {
            String::new()
        };

        r.push_str(&format!(
            "Lv.{} - {}{}\n",
            ms.level, ms.name, progress
        ));
        r.push_str(&format!(
            "   {}  💰{}金 💎{}钻 📦{}\n",
            status, ms.reward_gold, ms.reward_diamond, ms.reward_item
        ));
        r.push('\n');
    }

    r.push_str("━━━━━━━━━━━━━━━━━━\n");
    r.push_str("💡 发送'领取世界奖励+等级'领取对应奖励\n");
    r
}

/// 领取世界等级奖励
pub fn cmd_claim_world_reward(
    db: &Database,
    user_id: &str,
    args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let level = get_current_world_level(db);

    let target: i32 = match args.trim().parse() {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "{}\n请指定要领取的里程碑等级。\n用法: 领取世界奖励+等级\n示例: 领取世界奖励+5",
                prefix
            );
        }
    };

    // 查找对应里程碑
    let milestone = WORLD_MILESTONES.iter().find(|m| m.level == target);
    let ms = match milestone {
        Some(m) => m,
        None => {
            return format!(
                "{}\n不存在等级{}的世界里程碑。\n可选等级: 5, 10, 15, 20, 25, 30, 40, 50",
                prefix, target
            );
        }
    };

    // 检查世界等级是否达标
    if level < ms.level {
        return format!(
            "{}\n🔒 世界等级 Lv.{} 尚未达到 Lv.{}。\n当前世界等级: Lv.{}",
            prefix, level, ms.level, level
        );
    }

    // 检查是否已领取
    let key = format!("wl_milestone_{}", ms.level);
    let claimed = db.global_get("world_level_rewards", &key);
    if claimed == "1" {
        return format!(
            "{}\n✅ Lv.{}「{}」奖励已经领取过了！",
            prefix, ms.level, ms.name
        );
    }

    // 检查玩家等级
    let user_level: i32 = db.read_basic(user_id, "level").parse().unwrap_or(1);
    if user_level < 10 {
        return format!(
            "{}\n❌ 你的等级不足10级，无法领取世界等级奖励。",
            prefix
        );
    }

    // 发放奖励
    let gold: i32 = db.read_basic(user_id, "gold").parse().unwrap_or(0);
    db.write_basic_int(user_id, "gold", gold + ms.reward_gold);

    let diamond: i32 = db.read_basic(user_id, "diamond").parse().unwrap_or(0);
    db.write_basic_int(user_id, "diamond", diamond + ms.reward_diamond);

    // 给道具
    db.add_item(user_id, ms.reward_item, 1);

    // 标记已领取
    db.global_set("world_level_rewards", &key, "1");

    let mut r = format!("{}\n", prefix);
    r.push_str(&format!(
        "🎉 成功领取 Lv.{}「{}」奖励！\n\n",
        ms.level, ms.name
    ));
    r.push_str(&format!("  💰 金币 +{}\n", ms.reward_gold));
    r.push_str(&format!("  💎 钻石 +{}\n", ms.reward_diamond));
    r.push_str(&format!("  📦 道具 +1 [{}]\n", ms.reward_item));
    r.push_str(&format!("\n💰 当前金币: {}\n", gold + ms.reward_gold));
    r.push_str(&format!("💎 当前钻石: {}\n", diamond + ms.reward_diamond));

    r
}

/// 世界等级历史
pub fn cmd_world_level_history(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let level = get_current_world_level(db);
    let last_update = db.global_get("world_level", "last_update");
    let history = db.global_get("world_level", "history");

    let update_str = if last_update.is_empty() {
        "尚未更新".to_string()
    } else {
        last_update
    };

    let mut r = format!("{}\n", prefix);
    r.push_str("══════ 世界等级历史 ══════\n\n");
    r.push_str(&format!("🌍 当前世界等级: Lv.{}\n", level));
    r.push_str(&format!("📅 最后更新: {}\n\n", update_str));

    if history.is_empty() {
        r.push_str("📝 暂无升级记录\n");
    } else {
        r.push_str("📜 升级记录 (最近20条):\n");
        let entries: Vec<&str> = history.split('|').collect();
        for entry in entries.iter().rev() {
            r.push_str(&format!("  📌 {}\n", entry));
        }
    }

    r.push_str("\n━━━━━━━━━━━━━━━━━━\n");
    r.push_str("💡 世界等级每日自动更新，最多+1级\n");

    r
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_lookup() {
        assert_eq!(get_tier_for_level(1).name, "初生之犊");
        assert_eq!(get_tier_for_level(5).name, "崭露头角");
        assert_eq!(get_tier_for_level(10).name, "群雄逐鹿");
        assert_eq!(get_tier_for_level(20).name, "风云际会");
        assert_eq!(get_tier_for_level(30).name, "龙虎争霸");
        assert_eq!(get_tier_for_level(40).name, "霸主时代");
        assert_eq!(get_tier_for_level(50).name, "诸神黄昏");
        assert_eq!(get_tier_for_level(99).name, "诸神黄昏");
    }

    #[test]
    fn test_tier_count() {
        assert_eq!(WORLD_LEVEL_TIERS.len(), 7);
    }

    #[test]
    fn test_tier_escalation() {
        for i in 1..WORLD_LEVEL_TIERS.len() {
            assert!(
                WORLD_LEVEL_TIERS[i].modifier.0 >= WORLD_LEVEL_TIERS[i - 1].modifier.0,
                "HP bonus should escalate at tier {}",
                i
            );
            assert!(
                WORLD_LEVEL_TIERS[i].modifier.3 >= WORLD_LEVEL_TIERS[i - 1].modifier.3,
                "EXP bonus should escalate at tier {}",
                i
            );
        }
    }

    #[test]
    fn test_milestone_count() {
        assert_eq!(WORLD_MILESTONES.len(), 8);
    }

    #[test]
    fn test_milestone_levels_unique() {
        let mut levels: Vec<i32> = WORLD_MILESTONES.iter().map(|m| m.level).collect();
        levels.sort();
        levels.dedup();
        assert_eq!(levels.len(), WORLD_MILESTONES.len());
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for ms in WORLD_MILESTONES {
            assert!(ms.reward_gold > 0, "Gold reward should be positive");
            assert!(
                ms.reward_diamond > 0,
                "Diamond reward should be positive"
            );
        }
    }

    #[test]
    fn test_milestone_rewards_escalate() {
        for i in 1..WORLD_MILESTONES.len() {
            assert!(
                WORLD_MILESTONES[i].reward_gold >= WORLD_MILESTONES[i - 1].reward_gold,
                "Gold reward should escalate"
            );
            assert!(
                WORLD_MILESTONES[i].reward_diamond >= WORLD_MILESTONES[i - 1].reward_diamond,
                "Diamond reward should escalate"
            );
        }
    }

    #[test]
    fn test_catchup_bonus_structure() {
        for tier in WORLD_LEVEL_TIERS {
            if tier.catchup_threshold > 0 {
                assert!(tier.catchup_bonus.0 > 0, "Catchup EXP should be positive");
                assert!(tier.catchup_bonus.1 > 0, "Catchup Gold should be positive");
            }
        }
    }

    #[test]
    fn test_modifier_sum_positive() {
        for tier in WORLD_LEVEL_TIERS {
            if tier.min_level > 1 {
                let reward_sum = tier.modifier.2 + tier.modifier.3 + tier.modifier.4;
                assert!(
                    reward_sum > 0,
                    "Tier {} should have positive reward modifiers",
                    tier.min_level
                );
            }
        }
    }

    #[test]
    fn test_world_level_calc_bounds() {
        let max_tier = &WORLD_LEVEL_TIERS[WORLD_LEVEL_TIERS.len() - 1];
        assert!(max_tier.min_level <= 50, "Max tier should be at most 50");
    }

    #[test]
    fn test_milestone_names_not_empty() {
        for ms in WORLD_MILESTONES {
            assert!(!ms.name.is_empty(), "Milestone name should not be empty");
            assert!(
                !ms.reward_item.is_empty(),
                "Reward item should not be empty"
            );
        }
    }

    #[test]
    fn test_tier_descriptions() {
        for tier in WORLD_LEVEL_TIERS {
            assert!(!tier.name.is_empty());
            assert!(!tier.emoji.is_empty());
            assert!(!tier.desc.is_empty());
        }
    }
}

/// CakeGame 成就系统
/// 追踪玩家里程碑，达成后发放奖励
/// 成就分5大类：战斗/经济/社交/探索/收集
/// 每个成就有铜/银/金/钻 四个等级
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use crate::user;

/// 成就等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AchieveTier {
    Bronze,
    Silver,
    Gold,
    Diamond,
}

impl AchieveTier {
    fn name(&self) -> &'static str {
        match self {
            AchieveTier::Bronze => "铜",
            AchieveTier::Silver => "银",
            AchieveTier::Gold => "金",
            AchieveTier::Diamond => "钻",
        }
    }
    fn icon(&self) -> &'static str {
        match self {
            AchieveTier::Bronze => "🥉",
            AchieveTier::Silver => "🥈",
            AchieveTier::Gold => "🥇",
            AchieveTier::Diamond => "💎",
        }
    }
    fn reward_diamond(&self) -> i32 {
        match self {
            AchieveTier::Bronze => 5,
            AchieveTier::Silver => 15,
            AchieveTier::Gold => 50,
            AchieveTier::Diamond => 150,
        }
    }
    fn reward_gold(&self) -> i32 {
        match self {
            AchieveTier::Bronze => 500,
            AchieveTier::Silver => 2000,
            AchieveTier::Gold => 8000,
            AchieveTier::Diamond => 30000,
        }
    }
}

/// 成就分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AchieveCategory {
    Combat,
    Economy,
    Social,
    Explore,
    Collect,
}

impl AchieveCategory {
    fn name(&self) -> &'static str {
        match self {
            AchieveCategory::Combat => "战斗",
            AchieveCategory::Economy => "经济",
            AchieveCategory::Social => "社交",
            AchieveCategory::Explore => "探索",
            AchieveCategory::Collect => "收集",
        }
    }
    fn icon(&self) -> &'static str {
        match self {
            AchieveCategory::Combat => "⚔️",
            AchieveCategory::Economy => "💰",
            AchieveCategory::Social => "🤝",
            AchieveCategory::Explore => "🗺️",
            AchieveCategory::Collect => "🎒",
        }
    }
}

/// 成就定义
struct AchievementDef {
    id: &'static str,
    name: &'static str,
    category: AchieveCategory,
    description: &'static str,
    thresholds: [i32; 4],
    stat_key: &'static str,
}

const ALL_TIERS: &[AchieveTier] = &[
    AchieveTier::Bronze,
    AchieveTier::Silver,
    AchieveTier::Gold,
    AchieveTier::Diamond,
];

const ALL_CATS: &[AchieveCategory] = &[
    AchieveCategory::Combat,
    AchieveCategory::Economy,
    AchieveCategory::Social,
    AchieveCategory::Explore,
    AchieveCategory::Collect,
];

const ACHIEVEMENTS: &[AchievementDef] = &[
    AchievementDef {
        id: "kill_monster",
        name: "怪物猎人",
        category: AchieveCategory::Combat,
        description: "累计击杀怪物",
        thresholds: [10, 100, 500, 2000],
        stat_key: "total_kills",
    },
    AchievementDef {
        id: "boss_slayer",
        name: "BOSS终结者",
        category: AchieveCategory::Combat,
        description: "累计击败BOSS",
        thresholds: [1, 10, 50, 200],
        stat_key: "boss_kills",
    },
    AchievementDef {
        id: "pvp_winner",
        name: "竞技之王",
        category: AchieveCategory::Combat,
        description: "PvP胜利次数",
        thresholds: [5, 30, 100, 500],
        stat_key: "pvp_wins",
    },
    AchievementDef {
        id: "abyss_floor",
        name: "深渊征服者",
        category: AchieveCategory::Combat,
        description: "深渊到达层数",
        thresholds: [10, 30, 60, 100],
        stat_key: "abyss_max_floor",
    },
    AchievementDef {
        id: "level_reach",
        name: "等级巅峰",
        category: AchieveCategory::Combat,
        description: "达到等级",
        thresholds: [10, 30, 50, 80],
        stat_key: "max_level",
    },
    AchievementDef {
        id: "gold_earned",
        name: "财富之路",
        category: AchieveCategory::Economy,
        description: "累计获得金币",
        thresholds: [10000, 100000, 1000000, 10000000],
        stat_key: "total_gold_earned",
    },
    AchievementDef {
        id: "diamond_earned",
        name: "钻石大亨",
        category: AchieveCategory::Economy,
        description: "累计获得钻石",
        thresholds: [100, 500, 2000, 10000],
        stat_key: "total_diamond_earned",
    },
    AchievementDef {
        id: "trade_count",
        name: "交易达人",
        category: AchieveCategory::Economy,
        description: "完成交易次数",
        thresholds: [5, 30, 100, 500],
        stat_key: "trade_count",
    },
    AchievementDef {
        id: "shop_sales",
        name: "商业巨头",
        category: AchieveCategory::Economy,
        description: "商店出售次数",
        thresholds: [10, 50, 200, 1000],
        stat_key: "shop_sales",
    },
    AchievementDef {
        id: "guild_donate",
        name: "公会贡献者",
        category: AchieveCategory::Social,
        description: "公会捐献次数",
        thresholds: [5, 30, 100, 500],
        stat_key: "guild_donations",
    },
    AchievementDef {
        id: "gift_count",
        name: "慷慨之心",
        category: AchieveCategory::Social,
        description: "赠送物品/金币次数",
        thresholds: [5, 30, 100, 500],
        stat_key: "gift_count",
    },
    AchievementDef {
        id: "shout_count",
        name: "话唠之王",
        category: AchieveCategory::Social,
        description: "世界喊话次数",
        thresholds: [10, 50, 200, 1000],
        stat_key: "shout_count",
    },
    AchievementDef {
        id: "sign_streak",
        name: "坚持不懈",
        category: AchieveCategory::Social,
        description: "累计签到天数",
        thresholds: [7, 30, 100, 365],
        stat_key: "sign_total",
    },
    AchievementDef {
        id: "maps_visited",
        name: "旅行家",
        category: AchieveCategory::Explore,
        description: "访问不同地图数",
        thresholds: [5, 15, 25, 33],
        stat_key: "unique_maps",
    },
    AchievementDef {
        id: "gather_count",
        name: "采集大师",
        category: AchieveCategory::Explore,
        description: "累计采集次数",
        thresholds: [20, 100, 500, 2000],
        stat_key: "total_gathers",
    },
    AchievementDef {
        id: "quest_complete",
        name: "任务达人",
        category: AchieveCategory::Explore,
        description: "完成任务数",
        thresholds: [5, 15, 50, 100],
        stat_key: "quests_done",
    },
    AchievementDef {
        id: "npc_talk",
        name: "社交蝴蝶",
        category: AchieveCategory::Explore,
        description: "与NPC对话次数",
        thresholds: [10, 50, 200, 500],
        stat_key: "npc_talks",
    },
    AchievementDef {
        id: "items_collected",
        name: "收藏家",
        category: AchieveCategory::Collect,
        description: "背包物品种类数",
        thresholds: [10, 50, 150, 300],
        stat_key: "unique_items",
    },
    AchievementDef {
        id: "equip_enhance",
        name: "强化之路",
        category: AchieveCategory::Collect,
        description: "装备强化次数",
        thresholds: [5, 30, 100, 500],
        stat_key: "enhance_count",
    },
    AchievementDef {
        id: "compose_count",
        name: "合成工匠",
        category: AchieveCategory::Collect,
        description: "合成物品次数",
        thresholds: [5, 30, 100, 500],
        stat_key: "compose_count",
    },
    AchievementDef {
        id: "skill_count",
        name: "技能学者",
        category: AchieveCategory::Collect,
        description: "学习技能数",
        thresholds: [3, 10, 30, 60],
        stat_key: "skills_learned",
    },
];

/// 从 Global 表读取成就进度值
fn get_stat(db: &Database, user_id: &str, stat_key: &str) -> i32 {
    let key = format!("ach_{}_{}", user_id, stat_key);
    db.global_get("achievements", &key).parse().unwrap_or(0)
}

/// 写入成就进度值
fn set_stat(db: &Database, user_id: &str, stat_key: &str, value: i32) {
    let key = format!("ach_{}_{}", user_id, stat_key);
    db.global_set("achievements", &key, &value.to_string());
}

/// 增加成就进度
pub fn add_progress(db: &Database, user_id: &str, stat_key: &str, amount: i32) {
    let current = get_stat(db, user_id, stat_key);
    set_stat(db, user_id, stat_key, current + amount);
}

/// 设置最大值成就进度 (取 max)
pub fn set_max_progress(db: &Database, user_id: &str, stat_key: &str, value: i32) {
    let current = get_stat(db, user_id, stat_key);
    if value > current {
        set_stat(db, user_id, stat_key, value);
    }
}

/// 获取已领取奖励的成就ID列表
fn get_claimed(db: &Database, user_id: &str) -> Vec<String> {
    let key = format!("ach_{}_claimed", user_id);
    let raw = db.global_get("achievements", &key);
    if raw.is_empty() {
        Vec::new()
    } else {
        raw.split(',').map(|s| s.to_string()).collect()
    }
}

/// 标记成就已领取
fn mark_claimed(db: &Database, user_id: &str, achieve_id: &str) {
    let mut claimed = get_claimed(db, user_id);
    if !claimed.iter().any(|c| c == achieve_id) {
        claimed.push(achieve_id.to_string());
        let key = format!("ach_{}_claimed", user_id);
        db.global_set("achievements", &key, &claimed.join(","));
    }
}

/// 计算当前达成的最高等级
fn calc_tier(thresholds: &[i32; 4], value: i32) -> Option<AchieveTier> {
    if value >= thresholds[3] {
        Some(AchieveTier::Diamond)
    } else if value >= thresholds[2] {
        Some(AchieveTier::Gold)
    } else if value >= thresholds[1] {
        Some(AchieveTier::Silver)
    } else if value >= thresholds[0] {
        Some(AchieveTier::Bronze)
    } else {
        None
    }
}

/// 进度条 (10格)
fn progress_bar(current: i32, target: i32) -> String {
    let pct = if target > 0 {
        (current.min(target) as f64 / target as f64 * 10.0) as usize
    } else {
        10
    };
    format!("[{}{}]", "█".repeat(pct.min(10)), "░".repeat(10 - pct.min(10)))
}

/// 成就兑换商品定义
struct ExchangeItem {
    name: &'static str,
    cost: u32,
    emoji: &'static str,
    description: &'static str,
    item_type: ExchangeItemType,
    amount: i32,
}

enum ExchangeItemType {
    Gold,
    Diamond,
    Exp,
    Item,
}

const EXCHANGE_ITEMS: &[ExchangeItem] = &[
    ExchangeItem {
        name: "成就宝箱",
        cost: 50,
        emoji: "🎁",
        description: "金币×2000 + 经验×500",
        item_type: ExchangeItemType::Gold,
        amount: 2000,
    },
    ExchangeItem {
        name: "成就钻石袋",
        cost: 80,
        emoji: "💎",
        description: "钻石×30",
        item_type: ExchangeItemType::Diamond,
        amount: 30,
    },
    ExchangeItem {
        name: "成就经验丹",
        cost: 100,
        emoji: "🧪",
        description: "经验×2000",
        item_type: ExchangeItemType::Exp,
        amount: 2000,
    },
    ExchangeItem {
        name: "成就强化石",
        cost: 150,
        emoji: "🔮",
        description: "强化石×3",
        item_type: ExchangeItemType::Item,
        amount: 3,
    },
    ExchangeItem {
        name: "成就高级宝箱",
        cost: 200,
        emoji: "👑",
        description: "金币×10000 + 钻石×80 + 经验×3000",
        item_type: ExchangeItemType::Gold,
        amount: 10000,
    },
    ExchangeItem {
        name: "成就神器碎片",
        cost: 300,
        emoji: "⚔️",
        description: "远古超界石×2",
        item_type: ExchangeItemType::Item,
        amount: 2,
    },
    ExchangeItem {
        name: "成就至尊宝箱",
        cost: 500,
        emoji: "🏆",
        description: "金币×50000 + 钻石×200 + 经验×10000",
        item_type: ExchangeItemType::Gold,
        amount: 50000,
    },
];

/// 成就兑换历史（Global表 section = achievement_exchange_{uid}）
fn get_exchange_count(db: &Database, user_id: &str, item_name: &str) -> i32 {
    let section = format!("achievement_exchange_{}", user_id);
    let key = format!("exchanged_{}", item_name);
    db.global_get(&section, &key).parse().unwrap_or(0)
}

fn set_exchange_count(db: &Database, user_id: &str, item_name: &str, count: i32) {
    let section = format!("achievement_exchange_{}", user_id);
    let key = format!("exchanged_{}", item_name);
    db.global_set(&section, &key, &count.to_string());
}

/// 成就点数
fn calc_points(claimed: &[String]) -> u32 {
    claimed
        .iter()
        .map(|key| {
            if key.contains("钻") {
                40
            } else if key.contains("金") {
                30
            } else if key.contains("银") {
                20
            } else if key.contains("铜") {
                10
            } else {
                0
            }
        })
        .sum()
}

/// 查看成就列表 (成就 或 成就+分类名 或 成就+成就名)
pub fn cmd_achievement_list(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if !args.is_empty() {
        // 按分类查看
        let cat = match args {
            "战斗" | "战斗成就" => Some(AchieveCategory::Combat),
            "经济" | "经济成就" => Some(AchieveCategory::Economy),
            "社交" | "社交成就" => Some(AchieveCategory::Social),
            "探索" | "探索成就" => Some(AchieveCategory::Explore),
            "收集" | "收集成就" => Some(AchieveCategory::Collect),
            _ => None,
        };
        if let Some(category) = cat {
            return view_category(db, user_id, category, &prefix);
        }
        return view_single(db, user_id, args, &prefix);
    }

    // 总览
    let claimed = get_claimed(db, user_id);
    let mut total_done = 0u32;

    let mut r = format!("{}\n═════ 成就系统 ═════\n", prefix);
    r.push_str(&format!("🏆 成就点数: {}\n", calc_points(&claimed)));
    r.push_str(&format!("📊 总等级: {} 个\n\n", ACHIEVEMENTS.len() * 4));

    for cat in ALL_CATS {
        let defs: Vec<_> = ACHIEVEMENTS.iter().filter(|a| a.category == *cat).collect();
        let mut done = 0u32;
        for def in &defs {
            let val = get_stat(db, user_id, def.stat_key);
            for (i, _tier) in ALL_TIERS.iter().enumerate() {
                if val >= def.thresholds[i] {
                    done += 1;
                }
            }
        }
        total_done += done;
        let total = defs.len() as u32 * 4;
        let pct = done.saturating_mul(100).checked_div(total).unwrap_or(0);
        let bar = progress_bar(done as i32, total as i32);
        r.push_str(&format!(
            "{} {}成就: {}/{} {} {}%\n",
            cat.icon(),
            cat.name(),
            done,
            total,
            bar,
            pct
        ));
    }

    r.push_str(&format!(
        "\n📊 总完成: {}/{} 等级\n",
        total_done,
        ACHIEVEMENTS.len() * 4
    ));
    r.push_str("\n💡 发送「成就+分类名」查看分类 (战斗/经济/社交/探索/收集)");
    r.push_str("\n💡 发送「成就+成就名」查看单个成就详情");
    r
}

/// 查看分类成就
fn view_category(db: &Database, user_id: &str, category: AchieveCategory, prefix: &str) -> String {
    let defs: Vec<_> = ACHIEVEMENTS.iter().filter(|a| a.category == category).collect();
    let claimed = get_claimed(db, user_id);

    let mut r = format!("{}\n═══ {}{}成就 ═══\n", prefix, category.icon(), category.name());

    for def in &defs {
        let val = get_stat(db, user_id, def.stat_key);
        let tier_str = match calc_tier(&def.thresholds, val) {
            Some(t) => format!("{}{}", t.icon(), t.name()),
            None => "⚪未达成".to_string(),
        };
        let next_th = def
            .thresholds
            .iter()
            .find(|&&t| val < t)
            .copied()
            .unwrap_or(def.thresholds[3]);
        let bar = progress_bar(val, next_th);

        r.push_str(&format!(
            "\n🎯 {} | {} | {}/{} {}",
            def.name, tier_str, val, next_th, bar
        ));

        // 显示可领取奖励
        for (i, tier) in ALL_TIERS.iter().enumerate() {
            let key = format!("{}_{}", def.id, tier.name());
            if val >= def.thresholds[i] && !claimed.contains(&key) {
                r.push_str(&format!(
                    "\n   🎁 可领取: {}级 ({}金+{}钻) → 「领取成就+{}」",
                    tier.name(),
                    tier.reward_gold(),
                    tier.reward_diamond(),
                    def.name
                ));
            }
        }
    }
    r.push_str("\n\n💡 发送「领取成就+成就名」领取奖励");
    r
}

/// 查看单个成就
fn view_single(db: &Database, user_id: &str, name: &str, prefix: &str) -> String {
    let def = ACHIEVEMENTS.iter().find(|a| a.name.contains(name) || a.id == name);
    let def = match def {
        Some(d) => d,
        None => return format!("{}\n❌ 找不到成就 [{}]", prefix, name),
    };

    let val = get_stat(db, user_id, def.stat_key);
    let claimed = get_claimed(db, user_id);

    let mut r = format!("{}\n═══ 成就: {} ═══\n", prefix, def.name);
    r.push_str(&format!("分类: {}{}\n", def.category.icon(), def.category.name()));
    r.push_str(&format!("说明: {}\n", def.description));
    r.push_str(&format!("当前进度: {}\n\n", val));

    for (i, tier) in ALL_TIERS.iter().enumerate() {
        let key = format!("{}_{}", def.id, tier.name());
        let status = if claimed.contains(&key) {
            "✅已领取"
        } else if val >= def.thresholds[i] {
            "🎁可领取"
        } else {
            "🔒未达成"
        };
        r.push_str(&format!(
            "{} {}级: {} {} | 奖励: {}金+{}钻\n",
            tier.icon(),
            tier.name(),
            def.thresholds[i],
            status,
            tier.reward_gold(),
            tier.reward_diamond()
        ));
    }

    if !claimed.iter().any(|c| c.starts_with(def.id)) {
        r.push_str("\n💡 发送「领取成就+成就名」领取已达成的奖励");
    }
    r
}

/// 领取成就奖励
pub fn cmd_claim_achievement(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        return format!("{}\n请指定要领取的成就名。\n用法: 领取成就+成就名", prefix);
    }

    let def = ACHIEVEMENTS.iter().find(|a| a.name.contains(args) || a.id == args);
    let def = match def {
        Some(d) => d,
        None => return format!("{}\n❌ 找不到成就 [{}]", prefix, args),
    };

    let val = get_stat(db, user_id, def.stat_key);
    let claimed = get_claimed(db, user_id);
    let mut total_gold = 0i32;
    let mut total_diamond = 0i32;
    let mut claimed_any = false;
    let mut claimed_names: Vec<String> = Vec::new();

    for (i, tier) in ALL_TIERS.iter().enumerate() {
        let key = format!("{}_{}", def.id, tier.name());
        if val >= def.thresholds[i] && !claimed.contains(&key) {
            total_gold += tier.reward_gold();
            total_diamond += tier.reward_diamond();
            mark_claimed(db, user_id, &key);
            claimed_any = true;
            claimed_names.push(format!("{}{}", tier.icon(), tier.name()));
        }
    }

    if !claimed_any {
        let next = match calc_tier(&def.thresholds, val) {
            None => format!("需要达到 {}", def.thresholds[0]),
            Some(AchieveTier::Bronze) => format!("需要达到 {}", def.thresholds[1]),
            Some(AchieveTier::Silver) => format!("需要达到 {}", def.thresholds[2]),
            Some(AchieveTier::Gold) => format!("需要达到 {}", def.thresholds[3]),
            Some(AchieveTier::Diamond) => "已全部领取".to_string(),
        };
        return format!(
            "{}\n成就 [{}] 暂无可领取的奖励。\n当前进度: {} / {}",
            prefix, def.name, val, next
        );
    }

    let _ = db.modify_currency(user_id, crate::core::CURRENCY_GOLD, "add", total_gold as i64);
    let _ = db.modify_currency(user_id, crate::core::CURRENCY_DIAMOND, "add", total_diamond as i64);

    format!(
        "{}\n🏆 成就奖励领取成功!\n成就: {}\n领取等级: {}\n获得: {}金币 + {}钻石\n\n继续努力，解锁更多成就等级！",
        prefix,
        def.name,
        claimed_names.join("、"),
        total_gold,
        total_diamond
    )
}

/// 成就排行
pub fn cmd_achievement_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let conn = db.lock_conn();
    let mut stmt =
        match conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION = 'achievements' AND ID LIKE '%_claimed'") {
            Ok(s) => s,
            Err(_) => return format!("{}\n⚠️ 数据查询失败", prefix),
        };

    let mut user_points: Vec<(String, u32)> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let uid = id
                .strip_prefix("ach_")
                .and_then(|s| s.strip_suffix("_claimed"))
                .unwrap_or(&id)
                .to_string();
            let claimed_raw: String = row.get(1).unwrap_or_default();
            let points = calc_points(&claimed_raw.split(',').map(|s| s.to_string()).collect::<Vec<_>>());
            Ok((uid, points))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).filter(|(_, p)| *p > 0).collect())
        .unwrap_or_default();

    drop(stmt);
    drop(conn);

    if user_points.is_empty() {
        return format!(
            "{}\n═══ 成就排行 ═══\n暂无成就记录。\n发送「成就」查看可完成的成就！",
            prefix
        );
    }

    user_points.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n═══ 成就排行 ═══\n\n", prefix);
    for (i, (uid, pts)) in user_points.iter().take(10).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let mark = if uid == user_id { " ← 你" } else { "" };
        r.push_str(&format!("{} {}. {} — {}点{}\n", medal, i + 1, uid, pts, mark));
    }

    if let Some(pos) = user_points.iter().position(|(uid, _)| uid == user_id) {
        r.push_str(&format!("\n📍 你的排名: 第{}名 ({}点)", pos + 1, user_points[pos].1));
    }
    r
}

/// 我的成就统计
pub fn cmd_my_achievements(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let claimed = get_claimed(db, user_id);

    let mut counts = [0u32; 4]; // bronze, silver, gold, diamond
    for key in &claimed {
        if key.contains("铜") {
            counts[0] += 1;
        } else if key.contains("银") {
            counts[1] += 1;
        } else if key.contains("金") {
            counts[2] += 1;
        } else if key.contains("钻") {
            counts[3] += 1;
        }
    }

    let total: u32 = counts.iter().sum();
    let max_possible = ACHIEVEMENTS.len() as u32 * 4;
    let points = calc_points(&claimed);

    let mut r = format!("{}\n═══ 我的成就 ═══\n", prefix);
    r.push_str(&format!("🏆 成就点数: {}\n", points));
    r.push_str(&format!("📊 总进度: {}/{}\n\n", total, max_possible));
    r.push_str(&format!("🥉 铜级: {}\n", counts[0]));
    r.push_str(&format!("🥈 银级: {}\n", counts[1]));
    r.push_str(&format!("🥇 金级: {}\n", counts[2]));
    r.push_str(&format!("💎 钻级: {}\n", counts[3]));

    let pct = total.saturating_mul(100).checked_div(max_possible).unwrap_or(0);
    let bar = progress_bar(total as i32, max_possible as i32);
    r.push_str(&format!("\n完成率: {}% {}", pct, bar));
    r
}

#[allow(dead_code)]
// ===== 成就进度钩子 (供其他模块调用) =====
/// 战斗击杀怪物后调用
pub fn on_monster_killed(db: &Database, user_id: &str) {
    add_progress(db, user_id, "total_kills", 1);
}

/// 击败BOSS后调用
pub fn on_boss_kill(db: &Database, user_id: &str) {
    add_progress(db, user_id, "boss_kills", 1);
}

/// PvP胜利后调用
pub fn on_pvp_win(db: &Database, user_id: &str) {
    add_progress(db, user_id, "pvp_wins", 1);
}

/// 深渊层数更新
pub fn on_abyss_floor(db: &Database, user_id: &str, floor: i32) {
    set_max_progress(db, user_id, "abyss_max_floor", floor);
}

/// 等级提升
pub fn on_level_up(db: &Database, user_id: &str, level: i32) {
    set_max_progress(db, user_id, "max_level", level);
}

/// 获得金币
pub fn on_gold_earned(db: &Database, user_id: &str, amount: i64) {
    add_progress(db, user_id, "total_gold_earned", amount as i32);
}

/// 获得钻石
pub fn on_diamond_earned(db: &Database, user_id: &str, amount: i64) {
    add_progress(db, user_id, "total_diamond_earned", amount as i32);
}

/// 完成交易
pub fn on_trade(db: &Database, user_id: &str) {
    add_progress(db, user_id, "trade_count", 1);
}

/// 商店出售
pub fn on_shop_sale(db: &Database, user_id: &str) {
    add_progress(db, user_id, "shop_sales", 1);
}

/// 公会捐献
pub fn on_guild_donate(db: &Database, user_id: &str) {
    add_progress(db, user_id, "guild_donations", 1);
}

/// 加入公会
pub fn on_guild_joined(_db: &Database, _user_id: &str) {
    // placeholder for future guild-join achievements
}

/// 赠送物品
pub fn on_gift(db: &Database, user_id: &str) {
    add_progress(db, user_id, "gift_count", 1);
}

/// 喊话
pub fn on_shout(db: &Database, user_id: &str) {
    add_progress(db, user_id, "shout_count", 1);
}

/// 签到
pub fn on_sign_in(db: &Database, user_id: &str) {
    add_progress(db, user_id, "sign_total", 1);
}

/// 访问地图
pub fn on_map_visit(db: &Database, user_id: &str) {
    add_progress(db, user_id, "unique_maps", 1);
}

/// 采集
pub fn on_gathered(db: &Database, user_id: &str) {
    add_progress(db, user_id, "total_gathers", 1);
}

/// 完成任务
pub fn on_quest_completed(db: &Database, user_id: &str) {
    add_progress(db, user_id, "quests_done", 1);
}

/// NPC对话
pub fn on_npc_talk(db: &Database, user_id: &str) {
    add_progress(db, user_id, "npc_talks", 1);
}

/// 装备强化
pub fn on_enhance(db: &Database, user_id: &str) {
    add_progress(db, user_id, "enhance_count", 1);
}

/// 合成物品
pub fn on_compose(db: &Database, user_id: &str) {
    add_progress(db, user_id, "compose_count", 1);
}

/// 学习技能
pub fn on_skill_learned(db: &Database, user_id: &str) {
    add_progress(db, user_id, "skills_learned", 1);
}

/// 训练突破
pub fn on_training_breakthrough(_db: &Database, _user_id: &str) {
    // placeholder for future training-breakthrough achievements
}

/// 成就兑换商店 — 查看可兑换商品
pub fn cmd_achievement_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let claimed = get_claimed(db, user_id);
    let points = calc_points(&claimed);

    let mut out = format!("{}\n═══ 🏆 成就兑换商店 ═══", prefix);
    out.push_str(&format!("\n💰 当前成就点数: {}\n", points));
    out.push_str("─────────────────────\n");

    for (i, item) in EXCHANGE_ITEMS.iter().enumerate() {
        let count = get_exchange_count(db, user_id, item.name);
        let affordable = if points >= item.cost { "✅" } else { "❌" };
        out.push_str(&format!(
            "{}. {} {} — {} 点{}",
            i + 1,
            item.emoji,
            item.name,
            item.cost,
            affordable
        ));
        if count > 0 {
            out.push_str(&format!(" (已兑换{}次)", count));
        }
        out.push_str(&format!("\n   📝 {}\n", item.description));
    }

    out.push_str("\n💡 使用「成就兑换+商品名」兑换物品");
    out
}

/// 成就兑换 — 兑换商品
pub fn cmd_achievement_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        let mut out = format!("{}\n❓ 请指定要兑换的商品名！", prefix);
        out.push_str("\n可兑换商品:\n");
        for item in EXCHANGE_ITEMS {
            out.push_str(&format!("  {} {} ({}点)\n", item.emoji, item.name, item.cost));
        }
        out.push_str("\n💡 使用「成就兑换+商品名」兑换");
        return out;
    }

    // 查找商品（精确 → 模糊）
    let item = EXCHANGE_ITEMS.iter().find(|e| e.name == args);
    let item = match item {
        Some(i) => i,
        None => {
            // 模糊匹配
            let matches: Vec<_> = EXCHANGE_ITEMS.iter().filter(|e| e.name.contains(args)).collect();
            match matches.len() {
                0 => {
                    return format!(
                        "{}\n❓ 未找到商品「{}」！使用「成就商店」查看可兑换商品。",
                        prefix, args
                    );
                }
                1 => matches[0],
                _ => {
                    let mut out = format!("{}\n🔍 找到多个匹配商品，请更精确地指定:\n", prefix);
                    for m in &matches {
                        out.push_str(&format!("  {} {} ({}点)\n", m.emoji, m.name, m.cost));
                    }
                    return out;
                }
            }
        }
    };

    // 检查成就点数
    let claimed = get_claimed(db, user_id);
    let points = calc_points(&claimed);

    if points < item.cost {
        return format!(
            "{}\n❌ 成就点数不足！需要 {} 点，当前 {} 点，还差 {} 点。",
            prefix,
            item.cost,
            points,
            item.cost - points
        );
    }

    // 执行兑换
    let mut out = format!("{}\n🎉 兑换成功！", prefix);
    out.push_str(&format!(
        "\n{} {} — 花费 {} 点成就点数",
        item.emoji, item.name, item.cost
    ));
    out.push_str(&format!("\n剩余成就点数: {} → {}", points, points - item.cost));

    // 发放奖励
    match item.item_type {
        ExchangeItemType::Gold => {
            let gold_amount = item.amount as i64;
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_amount);
            out.push_str(&format!("\n💰 获得金币: +{}", gold_amount));
            // 高级宝箱和至尊宝箱有额外奖励
            if item.name.contains("高级") {
                db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 80);
                user::add_experience(db, user_id, 3000);
                out.push_str("\n💎 获得钻石: +80");
                out.push_str("\n📈 获得经验: +3000");
            } else if item.name.contains("至尊") {
                db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, 200);
                user::add_experience(db, user_id, 10000);
                out.push_str("\n💎 获得钻石: +200");
                out.push_str("\n📈 获得经验: +10000");
            } else if item.name.contains("宝箱") {
                user::add_experience(db, user_id, 500);
                out.push_str("\n📈 获得经验: +500");
            }
        }
        ExchangeItemType::Diamond => {
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, item.amount as i64);
            out.push_str(&format!("\n💎 获得钻石: +{}", item.amount));
        }
        ExchangeItemType::Exp => {
            user::add_experience(db, user_id, item.amount);
            out.push_str(&format!("\n📈 获得经验: +{}", item.amount));
        }
        ExchangeItemType::Item => {
            let success = db.add_item(
                user_id,
                item.name.replace("成就", "").replace("碎片", "石").as_str(),
                item.amount,
            );
            if success {
                out.push_str(&format!(
                    "\n📦 获得物品: {} ×{}",
                    item.name.replace("成就", ""),
                    item.amount
                ));
            } else {
                // 回退：发放等价值金币
                let fallback_gold = item.cost as i64 * 100;
                db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, fallback_gold);
                out.push_str(&format!("\n⚠️ 物品添加失败，已退还等价金币: +{}", fallback_gold));
            }
        }
    }

    // 记录兑换次数
    let count = get_exchange_count(db, user_id, item.name);
    set_exchange_count(db, user_id, item.name, count + 1);

    out
}

/// 成就兑换统计 — 查看个人兑换历史
pub fn cmd_achievement_exchange_stats(
    db: &Database,
    user_id: &str,
    _args: &str,
    _msg_type: &str,
    _group: &str,
) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let claimed = get_claimed(db, user_id);
    let points = calc_points(&claimed);
    let mut total_spent = 0u32;
    let mut total_exchanges = 0i32;
    let mut has_history = false;

    let mut out = format!("{}\n═══ 📊 成就兑换统计 ═══", prefix);
    out.push_str(&format!("\n💰 当前成就点数: {}\n", points));
    out.push_str("─────────────────────\n");

    for item in EXCHANGE_ITEMS {
        let count = get_exchange_count(db, user_id, item.name);
        if count > 0 {
            has_history = true;
            let spent = item.cost * count as u32;
            total_spent += spent;
            total_exchanges += count;
            out.push_str(&format!(
                "{} {} ×{} (花费 {} 点)\n",
                item.emoji, item.name, count, spent
            ));
        }
    }

    if !has_history {
        out.push_str("\n📭 暂无兑换记录\n");
    }

    out.push_str("─────────────────────\n");
    out.push_str(&format!("📊 总兑换次数: {} 次\n", total_exchanges));
    out.push_str(&format!("💎 累计花费: {} 点\n", total_spent));
    out.push_str(&format!(
        "🏆 总获得成就点: {} 点 (已花费 {} + 剩余 {})\n",
        total_spent + points,
        total_spent,
        points
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_calc() {
        assert_eq!(calc_tier(&[10, 100, 500, 2000], 5), None);
        assert_eq!(calc_tier(&[10, 100, 500, 2000], 10), Some(AchieveTier::Bronze));
        assert_eq!(calc_tier(&[10, 100, 500, 2000], 100), Some(AchieveTier::Silver));
        assert_eq!(calc_tier(&[10, 100, 500, 2000], 500), Some(AchieveTier::Gold));
        assert_eq!(calc_tier(&[10, 100, 500, 2000], 2000), Some(AchieveTier::Diamond));
        assert_eq!(calc_tier(&[10, 100, 500, 2000], 9999), Some(AchieveTier::Diamond));
    }

    #[test]
    fn test_tier_rewards() {
        assert_eq!(AchieveTier::Bronze.reward_gold(), 500);
        assert_eq!(AchieveTier::Bronze.reward_diamond(), 5);
        assert_eq!(AchieveTier::Silver.reward_gold(), 2000);
        assert_eq!(AchieveTier::Silver.reward_diamond(), 15);
        assert_eq!(AchieveTier::Gold.reward_gold(), 8000);
        assert_eq!(AchieveTier::Gold.reward_diamond(), 50);
        assert_eq!(AchieveTier::Diamond.reward_gold(), 30000);
        assert_eq!(AchieveTier::Diamond.reward_diamond(), 150);
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(AchieveTier::Bronze.name(), "铜");
        assert_eq!(AchieveTier::Silver.name(), "银");
        assert_eq!(AchieveTier::Gold.name(), "金");
        assert_eq!(AchieveTier::Diamond.name(), "钻");
        assert_eq!(AchieveTier::Bronze.icon(), "🥉");
        assert_eq!(AchieveTier::Diamond.icon(), "💎");
    }

    #[test]
    fn test_category_names() {
        assert_eq!(AchieveCategory::Combat.name(), "战斗");
        assert_eq!(AchieveCategory::Economy.name(), "经济");
        assert_eq!(AchieveCategory::Social.name(), "社交");
        assert_eq!(AchieveCategory::Explore.name(), "探索");
        assert_eq!(AchieveCategory::Collect.name(), "收集");
    }

    #[test]
    fn test_achievement_definitions() {
        let mut ids: Vec<&str> = ACHIEVEMENTS.iter().map(|a| a.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), ACHIEVEMENTS.len());
        for def in ACHIEVEMENTS {
            assert!(
                def.thresholds[0] < def.thresholds[1]
                    && def.thresholds[1] < def.thresholds[2]
                    && def.thresholds[2] < def.thresholds[3],
                "Bad thresholds for {}",
                def.id
            );
        }
    }

    #[test]
    fn test_achievement_count() {
        let mut counts = std::collections::HashMap::new();
        for def in ACHIEVEMENTS {
            *counts.entry(def.category).or_insert(0u32) += 1;
        }
        for (cat, count) in &counts {
            assert!(*count >= 3, "Category {:?} has only {} achievements", cat, count);
        }
        assert_eq!(ACHIEVEMENTS.len(), 21);
    }

    #[test]
    fn test_total_tiers() {
        assert_eq!(ACHIEVEMENTS.len() * 4, 84);
    }

    #[test]
    fn test_stat_key_uniqueness() {
        let mut keys: Vec<&str> = ACHIEVEMENTS.iter().map(|a| a.stat_key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), ACHIEVEMENTS.len());
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(0, 100);
        assert_eq!(bar, "[░░░░░░░░░░]");
        let bar = progress_bar(50, 100);
        assert_eq!(bar, "[█████░░░░░]");
        let bar = progress_bar(100, 100);
        assert_eq!(bar, "[██████████]");
    }

    #[test]
    fn test_calc_points() {
        let claimed = vec!["kill_monster_铜".to_string(), "kill_monster_银".to_string()];
        assert_eq!(calc_points(&claimed), 30);
        let empty: Vec<String> = vec![];
        assert_eq!(calc_points(&empty), 0);
    }

    #[test]
    fn test_exchange_items_count() {
        assert_eq!(EXCHANGE_ITEMS.len(), 7);
    }

    #[test]
    fn test_exchange_costs_positive() {
        for item in EXCHANGE_ITEMS {
            assert!(item.cost > 0, "{} has zero cost", item.name);
        }
    }

    #[test]
    fn test_exchange_costs_escalate() {
        for i in 1..EXCHANGE_ITEMS.len() {
            assert!(
                EXCHANGE_ITEMS[i].cost >= EXCHANGE_ITEMS[i - 1].cost,
                "Cost should escalate: {} ({}) < {} ({})",
                EXCHANGE_ITEMS[i].name,
                EXCHANGE_ITEMS[i].cost,
                EXCHANGE_ITEMS[i - 1].name,
                EXCHANGE_ITEMS[i - 1].cost,
            );
        }
    }

    #[test]
    fn test_exchange_names_unique() {
        let mut names: Vec<&str> = EXCHANGE_ITEMS.iter().map(|e| e.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len);
    }

    #[test]
    fn test_exchange_emojis_non_empty() {
        for item in EXCHANGE_ITEMS {
            assert!(!item.emoji.is_empty(), "{} has empty emoji", item.name);
        }
    }

    #[test]
    fn test_exchange_descriptions_non_empty() {
        for item in EXCHANGE_ITEMS {
            assert!(!item.description.is_empty(), "{} has empty description", item.name);
        }
    }

    #[test]
    fn test_exchange_amounts_positive() {
        for item in EXCHANGE_ITEMS {
            assert!(item.amount > 0, "{} has zero amount", item.name);
        }
    }

    #[test]
    fn test_exchange_cost_range() {
        // Costs should be between 50 and 500
        for item in EXCHANGE_ITEMS {
            assert!(item.cost >= 50, "{} cost {} < 50", item.name, item.cost);
            assert!(item.cost <= 500, "{} cost {} > 500", item.name, item.cost);
        }
    }

    #[test]
    fn test_achievement_stat_keys_exist() {
        // Verify all stat keys used by hooks are defined in ACHIEVEMENTS
        let hook_stat_keys = vec![
            "total_kills",
            "boss_kills",
            "pvp_wins",
            "abyss_max_floor",
            "max_level",
            "total_gold_earned",
            "total_diamond_earned",
            "trade_count",
            "shop_sales",
            "guild_donations",
            "gift_count",
            "shout_count",
            "sign_total",
            "unique_maps",
            "total_gathers",
            "quests_done",
            "npc_talks",
            "enhance_count",
            "compose_count",
            "skills_learned",
        ];
        let defined_keys: Vec<&str> = ACHIEVEMENTS.iter().map(|a| a.stat_key).collect();
        for key in &hook_stat_keys {
            assert!(
                defined_keys.contains(key),
                "Hook stat key '{}' not found in ACHIEVEMENTS definitions",
                key
            );
        }
    }
}

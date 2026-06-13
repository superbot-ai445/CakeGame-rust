use crate::db::Database;
use std::collections::HashMap;

// ==================== 声望系统 (Reputation/Faction System) ====================
// 玩家通过完成任务、战斗、捐献等方式获得不同阵营的声望
// 声望等级影响可购买物品、特殊称号、属性加成

/// 声望等级定义
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepLevel {
    Hated = -3,      // 仇恨
    Hostile = -2,    // 敌对
    Unfriendly = -1, // 冷淡
    Neutral = 0,     // 中立
    Friendly = 1,    // 友善
    Honored = 2,     // 尊敬
    Revered = 3,     // 崇敬
    Exalted = 4,     // 崇拜
}

impl RepLevel {
    pub fn from_points(points: i64) -> Self {
        match points {
            ..=-3000 => RepLevel::Hated,
            -2999..=-1000 => RepLevel::Hostile,
            -999..=-1 => RepLevel::Unfriendly,
            0..=999 => RepLevel::Neutral,
            1000..=2999 => RepLevel::Friendly,
            3000..=5999 => RepLevel::Honored,
            6000..=9999 => RepLevel::Revered,
            10000.. => RepLevel::Exalted,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RepLevel::Hated => "仇恨",
            RepLevel::Hostile => "敌对",
            RepLevel::Unfriendly => "冷淡",
            RepLevel::Neutral => "中立",
            RepLevel::Friendly => "友善",
            RepLevel::Honored => "尊敬",
            RepLevel::Revered => "崇敬",
            RepLevel::Exalted => "崇拜",
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            RepLevel::Hated => "💀",
            RepLevel::Hostile => "😡",
            RepLevel::Unfriendly => "😠",
            RepLevel::Neutral => "😐",
            RepLevel::Friendly => "🙂",
            RepLevel::Honored => "😊",
            RepLevel::Revered => "🌟",
            RepLevel::Exalted => "👑",
        }
    }

    pub fn next_threshold(&self) -> i64 {
        match self {
            RepLevel::Hated => -3000,
            RepLevel::Hostile => -1000,
            RepLevel::Unfriendly => 0,
            RepLevel::Neutral => 1000,
            RepLevel::Friendly => 3000,
            RepLevel::Honored => 6000,
            RepLevel::Revered => 10000,
            RepLevel::Exalted => i64::MAX,
        }
    }
}

/// 阵营定义
#[derive(Debug, Clone)]
pub struct Faction {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub emoji: &'static str,
    /// 每级声望提供的属性加成 (attr_name, bonus_per_level)
    pub bonuses: &'static [(&'static str, i64)],
    /// 各声望等级解锁的奖励物品 (level, item_name, description)
    pub rewards: &'static [(i64, &'static str, &'static str)],
}

pub const FACTIONS: &[Faction] = &[
    Faction {
        id: "light",
        name: "光明教廷",
        description: "守护大陆和平的神圣组织，以光明之力驱散黑暗",
        emoji: "✝️",
        bonuses: &[("HP", 50), ("Defense", 5), ("MagicResistance", 3)],
        rewards: &[
            (1000, "光明圣水", "友善: 回复全部HP/MP的神圣药剂"),
            (3000, "圣光护盾卷轴", "尊敬: 获得3回合护盾效果"),
            (6000, "光明裁决之剑", "崇敬: 史诗级光属性武器"),
            (10000, "光明使者称号", "崇拜: 全属性+5%永久加成"),
        ],
    },
    Faction {
        id: "shadow",
        name: "暗影议会",
        description: "隐藏在暗处的秘密组织，掌握禁忌的暗影魔法",
        emoji: "🌑",
        bonuses: &[("AD", 8), ("AP", 8), ("Crit", 3)],
        rewards: &[
            (1000, "暗影药剂", "友善: 临时提升暴击率20%"),
            (3000, "暗影斗篷", "尊敬: 闪避率+15%时装"),
            (6000, "暗影之刃", "崇敬: 史诗级暗属性武器"),
            (10000, "暗影领主称号", "崇拜: 暗属性伤害+20%"),
        ],
    },
    Faction {
        id: "adventurer",
        name: "冒险者公会",
        description: "自由冒险者的家园，遍布大陆各地的据点",
        emoji: "🗺️",
        bonuses: &[("Hit", 5), ("Dodge", 5), ("AbsorbHP", 3)],
        rewards: &[
            (1000, "冒险者补给包", "友善: 随机高级消耗品×5"),
            (3000, "探索者披风", "尊敬: 移动速度+20%"),
            (6000, "冒险王之证", "崇敬: 解锁隐藏地图"),
            (10000, "传奇冒险家称号", "崇拜: 全地图掉落率+15%"),
        ],
    },
    Faction {
        id: "merchant",
        name: "商业联盟",
        description: "掌控大陆经济命脉的强大商人组织",
        emoji: "💰",
        bonuses: &[("gold_bonus", 10), ("diamond_bonus", 2)],
        rewards: &[
            (1000, "折扣卡", "友善: NPC商店9折"),
            (3000, "VIP交易通道", "尊敬: 拍卖行手续费减半"),
            (6000, "金库钥匙", "崇敬: 钱庄利率翻倍"),
            (10000, "商业巨头称号", "崇拜: 所有交易收入+25%"),
        ],
    },
    Faction {
        id: "ancient",
        name: "远古守护者",
        description: "守护远古遗迹的神秘种族，掌握失落的知识",
        emoji: "🏛️",
        bonuses: &[("AP", 10), ("MagicResistance", 8), ("MP", 100)],
        rewards: &[
            (1000, "远古符文", "友善: 装备附魔成功率+10%"),
            (3000, "远古法典", "尊敬: 技能伤害+10%"),
            (6000, "远古神器碎片", "崇敬: 可合成远古神器"),
            (10000, "远古传承者称号", "崇拜: 解锁远古技能树"),
        ],
    },
    Faction {
        id: "monster",
        name: "魔物联盟",
        description: "由开明魔物组成的联盟，寻求与人类和平共处",
        emoji: "👹",
        bonuses: &[("AD", 5), ("HP", 80), ("ImmuneDamage", 3)],
        rewards: &[
            (1000, "魔物精华", "友善: 怪物掉落率+15%"),
            (3000, "魔化之力", "尊敬: 攻击附带吸血效果"),
            (6000, "魔王铠甲", "崇敬: 史诗级魔化防具套装"),
            (10000, "魔物之友称号", "崇拜: 怪物不再主动攻击"),
        ],
    },
];

/// 从Global表读取玩家声望
pub fn get_reputation(db: &Database, user_id: &str, faction_id: &str) -> i64 {
    let key = format!("rep_{}_{}", user_id, faction_id);
    db.global_get("reputation", &key).parse::<i64>().unwrap_or(0)
}

/// 写入声望到Global表
pub fn set_reputation(db: &Database, user_id: &str, faction_id: &str, points: i64) {
    let key = format!("rep_{}_{}", user_id, faction_id);
    db.global_set("reputation", &key, &points.to_string());
}

/// 增加声望
pub fn add_reputation(db: &Database, user_id: &str, faction_id: &str, amount: i64) -> (i64, RepLevel, RepLevel) {
    let old_points = get_reputation(db, user_id, faction_id);
    let old_level = RepLevel::from_points(old_points);
    let new_points = old_points + amount;
    let new_level = RepLevel::from_points(new_points);
    set_reputation(db, user_id, faction_id, new_points);
    (new_points, old_level, new_level)
}

/// 获取玩家所有声望
#[allow(dead_code)]
pub fn get_all_reputations(db: &Database, user_id: &str) -> HashMap<String, i64> {
    let mut reps = HashMap::new();
    for faction in FACTIONS {
        let points = get_reputation(db, user_id, faction.id);
        reps.insert(faction.id.to_string(), points);
    }
    reps
}

/// 声望等级进度条
fn format_rep_bar(points: i64, level: RepLevel) -> String {
    let thresholds = [-3000, -1000, 0, 1000, 3000, 6000, 10000];
    let mut progress = 0.0;

    if level == RepLevel::Exalted {
        progress = 1.0;
    } else {
        let idx = level as i32 + 3; // map -3..4 to 0..7
        if idx < 7 {
            let low = if idx == 0 {
                -10000
            } else {
                thresholds[(idx - 1) as usize]
            };
            let high = thresholds[idx as usize];
            let range = (high - low) as f64;
            if range > 0.0 {
                progress = ((points - low) as f64 / range).clamp(0.0, 0.99);
            }
        }
    }

    let filled = (progress * 10.0) as usize;
    let empty = 10 - filled;
    format!("{}{} {:.0}%", "█".repeat(filled), "░".repeat(empty), progress * 100.0)
}

// ==================== 指令实现 ====================

/// 查看声望 — 显示所有阵营声望
pub fn cmd_view_reputation(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("📜 【声望系统】\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    for faction in FACTIONS {
        let points = get_reputation(db, user_id, faction.id);
        let level = RepLevel::from_points(points);
        let bar = format_rep_bar(points, level);

        out.push_str(&format!("\n{} {} {}\n", faction.emoji, faction.name, level.emoji()));
        out.push_str(&format!("  等级: {} ({:+})\n", level.name(), points));
        out.push_str(&format!("  进度: {}\n", bar));

        // 显示属性加成
        if points > 0 {
            let mut bonuses = Vec::new();
            let level_num = (level as i32).max(0) as i64;
            for &(attr, per_level) in faction.bonuses {
                let total = per_level * level_num;
                if total > 0 {
                    bonuses.push(format!("{}+{}", attr, total));
                }
            }
            if !bonuses.is_empty() {
                out.push_str(&format!("  加成: {}\n", bonuses.join("/")));
            }
        }
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 通过战斗/任务/捐献获得声望\n");
    out.push_str("📖 输入「声望详情+阵营名」查看详情\n");
    out
}

/// 声望详情 — 查看特定阵营详细信息
pub fn cmd_reputation_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let faction_name = args.trim();
    let faction = match find_faction(faction_name) {
        Some(f) => f,
        None => {
            let names: Vec<&str> = FACTIONS.iter().map(|f| f.name).collect();
            return format!("❌ 未找到阵营「{}」\n可用阵营: {}", faction_name, names.join("、"));
        }
    };

    let points = get_reputation(db, user_id, faction.id);
    let level = RepLevel::from_points(points);
    let bar = format_rep_bar(points, level);

    let mut out = format!("{} {} 声望详情\n", faction.emoji, faction.name);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("📖 {}\n\n", faction.description));

    out.push_str(&format!("📊 当前声望: {:+}\n", points));
    out.push_str(&format!("⭐ 声望等级: {} {}\n", level.name(), level.emoji()));
    out.push_str(&format!("📈 等级进度: {}\n\n", bar));

    // 下一级所需
    if level != RepLevel::Exalted {
        let next = level.next_threshold();
        let needed = next - points;
        out.push_str(&format!("⬆️  下一等级需: {} 点声望\n\n", needed.max(0)));
    } else {
        out.push_str("🏆 已达最高等级!\n\n");
    }

    // 属性加成
    out.push_str("💪 属性加成:\n");
    let level_num = (level as i32).max(0) as i64;
    for &(attr, per_level) in faction.bonuses {
        let total = per_level * level_num;
        out.push_str(&format!("  {}: +{} (每级+{})\n", attr, total, per_level));
    }

    // 奖励列表
    out.push_str("\n🎁 声望奖励:\n");
    for &(req_points, item, desc) in faction.rewards {
        let req_level = RepLevel::from_points(req_points);
        let status = if points >= req_points { "✅" } else { "🔒" };
        out.push_str(&format!("  {} [{}] {} — {}\n", status, req_level.name(), item, desc));
    }

    // 声望等级表
    out.push_str("\n📋 声望等级表:\n");
    out.push_str("  💀 仇恨 (<-3000) | 😡 敌对 (-3000~-1000)\n");
    out.push_str("  😠 冷淡 (-1000~0) | 😐 中立 (0~1000)\n");
    out.push_str("  🙂 友善 (1000~3000) | 😊 尊敬 (3000~6000)\n");
    out.push_str("  🌟 崇敬 (6000~10000) | 👑 崇拜 (10000+)\n");

    out
}

/// 领取声望奖励
pub fn cmd_claim_reputation_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let faction_name = args.trim();
    let faction = match find_faction(faction_name) {
        Some(f) => f,
        None => return format!("❌ 未找到阵营「{}」", faction_name),
    };

    let points = get_reputation(db, user_id, faction.id);
    let level = RepLevel::from_points(points);

    // 查找可领取的奖励
    let mut claimable = Vec::new();
    for &(req_points, item, _desc) in faction.rewards {
        if points >= req_points {
            let claim_key = format!("rep_claim_{}_{}_{}", user_id, faction.id, req_points);
            let claimed = db.global_get("reputation", &claim_key);
            if claimed.is_empty() {
                claimable.push((req_points, item));
            }
        }
    }

    if claimable.is_empty() {
        if level == RepLevel::Neutral
            || level == RepLevel::Unfriendly
            || level == RepLevel::Hostile
            || level == RepLevel::Hated
        {
            return format!(
                "{} 当前声望等级「{}」，无奖励可领取\n提升声望到友善(1000+)解锁第一个奖励",
                faction.emoji,
                level.name()
            );
        }
        return format!("{} 所有已解锁奖励已领取", faction.emoji);
    }

    let mut out = format!(
        "{} {} 声望奖励领取\n━━━━━━━━━━━━━━━━━━━━\n",
        faction.emoji, faction.name
    );

    for (req_points, item) in &claimable {
        let claim_key = format!("rep_claim_{}_{}_{}", user_id, faction.id, req_points);
        db.global_set("reputation", &claim_key, "1");

        // 给予奖励物品
        give_reputation_reward(db, user_id, item);

        out.push_str(&format!("✅ 领取成功: {}\n", item));
    }

    out.push_str("\n💡 奖励已发放到背包");
    out
}

/// 声望排行榜
pub fn cmd_reputation_ranking(db: &Database, _user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let faction_name = args.trim();
    let faction = match find_faction(faction_name) {
        Some(f) => f,
        None => {
            // 显示综合排行
            return cmd_reputation_total_ranking(db);
        }
    };

    let mut out = format!("{} {} 声望排行\n━━━━━━━━━━━━━━━━━━━━\n", faction.emoji, faction.name);

    // 从Global表收集声望数据
    let mut rankings: Vec<(String, i64)> = Vec::new();

    // 遍历可能的用户ID (简化: 从Basic_User获取所有用户)
    for uid in db.all_users() {
        let points = get_reputation(db, &uid, faction.id);
        if points != 0 {
            rankings.push((uid, points));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    for (i, (uid, points)) in rankings.iter().take(10).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let level = RepLevel::from_points(*points);
        let nick = db.read_basic(uid, "NickName");
        let display = if nick.is_empty() { uid.clone() } else { nick };
        out.push_str(&format!(
            "{} #{} {} — {} ({})\n",
            medal,
            i + 1,
            display,
            level.name(),
            points
        ));
    }

    if rankings.is_empty() {
        out.push_str("暂无数据\n");
    }

    out
}

/// 综合声望排行（所有阵营总声望）
fn cmd_reputation_total_ranking(db: &Database) -> String {
    let mut out = String::from("🏆 综合声望排行\n━━━━━━━━━━━━━━━━━━━━\n");

    let mut rankings: Vec<(String, i64)> = Vec::new();

    for uid in db.all_users() {
        let mut total = 0i64;
        for faction in FACTIONS {
            total += get_reputation(db, &uid, faction.id);
        }
        if total > 0 {
            rankings.push((uid, total));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    for (i, (uid, total)) in rankings.iter().take(10).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let nick = db.read_basic(uid, "NickName");
        let display = if nick.is_empty() { uid.clone() } else { nick };
        out.push_str(&format!("{} #{} {} — {} 声望\n", medal, i + 1, display, total));
    }

    if rankings.is_empty() {
        out.push_str("暂无数据\n");
    }

    out.push_str("\n💡 输入「声望排行+阵营名」查看特定阵营排行");
    out
}

/// 捐献物品获取声望
pub fn cmd_donate_reputation(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let parts: Vec<&str> = args.splitn(2, |c: char| c.is_whitespace() || c == '+').collect();
    let faction_name = parts.first().unwrap_or(&"").trim();
    let item_name = parts.get(1).unwrap_or(&"").trim();
    let faction = match find_faction(faction_name) {
        Some(f) => f,
        None => return format!("❌ 未找到阵营「{}」", faction_name),
    };

    // 检查背包是否有该物品
    let knapsack = db.knapsack_all(user_id);
    let item = knapsack.iter().find(|k| k.name.contains(item_name));

    let item = match item {
        Some(i) => i,
        None => return format!("❌ 背包中没有「{}」", item_name),
    };

    // 计算声望值 (基于物品品质)
    let rep_gain = calculate_donation_value(&item.name);

    if rep_gain <= 0 {
        return format!("❌ 「{}」无法捐献给{}", item.name, faction.name);
    }

    // 移除物品并增加声望
    db.knapsack_remove(user_id, &item.name, 1);
    let (new_points, old_level, new_level) = add_reputation(db, user_id, faction.id, rep_gain);

    let mut out = format!("{} 捐献成功!\n", faction.emoji);
    out.push_str(&format!("📦 捐献: {} ×1\n", item.name));
    out.push_str(&format!("📈 获得声望: +{}\n", rep_gain));
    out.push_str(&format!("📊 当前声望: {:+}\n", new_points));

    if old_level != new_level {
        out.push_str(&format!(
            "\n🎉 声望等级提升! {} → {} {}",
            old_level.name(),
            new_level.name(),
            new_level.emoji()
        ));
    }

    out
}

/// 声望加成查询 — 计算声望提供的总属性加成
pub fn cmd_reputation_bonus(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("💪 声望属性加成\n━━━━━━━━━━━━━━━━━━━━\n");

    let mut total_bonuses: HashMap<String, i64> = HashMap::new();

    for faction in FACTIONS {
        let points = get_reputation(db, user_id, faction.id);
        let level = RepLevel::from_points(points);
        let level_num = (level as i32).max(0) as i64;

        if level_num > 0 {
            out.push_str(&format!("\n{} {} ({}):\n", faction.emoji, faction.name, level.name()));
            for &(attr, per_level) in faction.bonuses {
                let total = per_level * level_num;
                if total > 0 {
                    out.push_str(&format!("  {}: +{}\n", attr, total));
                    *total_bonuses.entry(attr.to_string()).or_insert(0) += total;
                }
            }
        }
    }

    if total_bonuses.is_empty() {
        out.push_str("\n暂无声望加成，提升声望等级获得属性加成\n");
    } else {
        out.push_str("\n📊 合计加成:\n");
        let mut sorted: Vec<_> = total_bonuses.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (attr, total) in sorted {
            out.push_str(&format!("  {}: +{}\n", attr, total));
        }
    }

    out
}

// ==================== 辅助函数 ====================

fn find_faction(name: &str) -> Option<&'static Faction> {
    // 精确匹配
    if let Some(f) = FACTIONS.iter().find(|f| f.name == name) {
        return Some(f);
    }
    // 模糊匹配
    FACTIONS.iter().find(|f| f.name.contains(name) || name.contains(f.name))
}

fn calculate_donation_value(item_name: &str) -> i64 {
    // 根据物品名称关键词判断捐献价值
    if item_name.contains("传说") || item_name.contains("神器") {
        500
    } else if item_name.contains("史诗") || item_name.contains("超界") {
        200
    } else if item_name.contains("稀有") {
        100
    } else if item_name.contains("精良") {
        50
    } else if item_name.contains("强化石") || item_name.contains("高级") {
        30
    } else if item_name.contains("药水") || item_name.contains("卷轴") {
        10
    } else {
        5 // 基础物品
    }
}

fn give_reputation_reward(db: &Database, user_id: &str, item_name: &str) {
    // 简化: 将奖励记录到Global表，实际应添加到背包
    let reward_key = format!("rep_reward_{}_{}", user_id, item_name);
    db.global_set("reputation", &reward_key, "1");
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faction_definitions() {
        assert_eq!(FACTIONS.len(), 6);
        for faction in FACTIONS {
            assert!(!faction.id.is_empty());
            assert!(!faction.name.is_empty());
            assert!(!faction.bonuses.is_empty());
            assert!(!faction.rewards.is_empty());
        }
    }

    #[test]
    fn test_rep_level_from_points() {
        assert_eq!(RepLevel::from_points(-5000), RepLevel::Hated);
        assert_eq!(RepLevel::from_points(-2000), RepLevel::Hostile);
        assert_eq!(RepLevel::from_points(-500), RepLevel::Unfriendly);
        assert_eq!(RepLevel::from_points(0), RepLevel::Neutral);
        assert_eq!(RepLevel::from_points(500), RepLevel::Neutral);
        assert_eq!(RepLevel::from_points(1500), RepLevel::Friendly);
        assert_eq!(RepLevel::from_points(4000), RepLevel::Honored);
        assert_eq!(RepLevel::from_points(8000), RepLevel::Revered);
        assert_eq!(RepLevel::from_points(15000), RepLevel::Exalted);
    }

    #[test]
    fn test_rep_level_ordering() {
        assert!(RepLevel::Hated < RepLevel::Hostile);
        assert!(RepLevel::Hostile < RepLevel::Unfriendly);
        assert!(RepLevel::Unfriendly < RepLevel::Neutral);
        assert!(RepLevel::Neutral < RepLevel::Friendly);
        assert!(RepLevel::Friendly < RepLevel::Honored);
        assert!(RepLevel::Honored < RepLevel::Revered);
        assert!(RepLevel::Revered < RepLevel::Exalted);
    }

    #[test]
    fn test_rep_level_names() {
        assert_eq!(RepLevel::Hated.name(), "仇恨");
        assert_eq!(RepLevel::Neutral.name(), "中立");
        assert_eq!(RepLevel::Exalted.name(), "崇拜");
    }

    #[test]
    fn test_rep_level_emojis() {
        assert_eq!(RepLevel::Hated.emoji(), "💀");
        assert_eq!(RepLevel::Neutral.emoji(), "😐");
        assert_eq!(RepLevel::Exalted.emoji(), "👑");
    }

    #[test]
    fn test_find_faction_exact() {
        let f = find_faction("光明教廷").unwrap();
        assert_eq!(f.id, "light");
    }

    #[test]
    fn test_find_faction_fuzzy() {
        let f = find_faction("光明");
        assert!(f.is_some());
        assert_eq!(f.unwrap().id, "light");

        let f2 = find_faction("暗影");
        assert!(f2.is_some());
        assert_eq!(f2.unwrap().id, "shadow");
    }

    #[test]
    fn test_find_faction_not_found() {
        assert!(find_faction("不存在的阵营").is_none());
    }

    #[test]
    fn test_donation_value() {
        assert_eq!(calculate_donation_value("传说之剑"), 500);
        assert_eq!(calculate_donation_value("史诗铠甲"), 200);
        assert_eq!(calculate_donation_value("稀有宝石"), 100);
        assert_eq!(calculate_donation_value("精良护符"), 50);
        assert_eq!(calculate_donation_value("强化石"), 30);
        assert_eq!(calculate_donation_value("小回复药水"), 10);
        assert_eq!(calculate_donation_value("破旧的布"), 5);
    }

    #[test]
    fn test_faction_bonus_consistency() {
        // 每个阵营至少有1个属性加成
        for faction in FACTIONS {
            assert!(!faction.bonuses.is_empty(), "阵营 {} 缺少属性加成定义", faction.name);
            // 每个奖励等级应大于0
            for &(req, _, _) in faction.rewards {
                assert!(req > 0, "阵营 {} 奖励等级应>0", faction.name);
            }
        }
    }

    #[test]
    fn test_rep_bar_format() {
        let bar = format_rep_bar(500, RepLevel::Neutral);
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));
        assert!(bar.contains("%"));
    }

    #[test]
    fn test_next_threshold() {
        assert_eq!(RepLevel::Neutral.next_threshold(), 1000);
        assert_eq!(RepLevel::Friendly.next_threshold(), 3000);
        assert_eq!(RepLevel::Exalted.next_threshold(), i64::MAX);
    }
}

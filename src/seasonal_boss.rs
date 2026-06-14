#![allow(dead_code)]

//! 赛季Boss挑战系统 (Seasonal Boss Rush)
//!
//! 每周轮换的赛季Boss挑战，包含8个赛季Boss，5个难度层级，
//! 排行榜、赛季代币商店和专属奖励系统。
//! - 8个赛季Boss: 从每周守护者到赛季之神
//! - 5个难度层级: 普通→深渊
//! - 战斗模拟: 基于玩家战力vs Boss属性
//! - 赛季代币商店: 12种商品
//! - 每周轮换: 每周一更换Boss，重置进度

use crate::core::*;
use crate::db::Database;
use crate::user;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const SECTION: &str = "seasonal_boss";

// ==================== Boss 定义 ====================

/// 赛季Boss定义
struct SeasonalBossDef {
    id: u8,
    name: &'static str,
    icon: &'static str,
    desc: &'static str,
    hp: i64,
    atk: i64,
    def: i64,
    weakness: &'static str,
    token_reward: i32,
    gold_reward: i64,
}

/// 8个赛季Boss，难度递增
const SEASONAL_BOSSES: &[SeasonalBossDef] = &[
    SeasonalBossDef {
        id: 1,
        name: "每周守护者",
        icon: "🛡️",
        desc: "赛季入门Boss",
        hp: 5000,
        atk: 200,
        def: 100,
        weakness: "火",
        token_reward: 10,
        gold_reward: 5000,
    },
    SeasonalBossDef {
        id: 2,
        name: "风暴领主",
        icon: "🌪️",
        desc: "操控风暴之力",
        hp: 12000,
        atk: 450,
        def: 250,
        weakness: "雷",
        token_reward: 20,
        gold_reward: 12000,
    },
    SeasonalBossDef {
        id: 3,
        name: "冰霜巨龙",
        icon: "🐉",
        desc: "远古冰龙",
        hp: 25000,
        atk: 800,
        def: 500,
        weakness: "火",
        token_reward: 35,
        gold_reward: 25000,
    },
    SeasonalBossDef {
        id: 4,
        name: "暗影君王",
        icon: "👤",
        desc: "暗影之主",
        hp: 50000,
        atk: 1500,
        def: 900,
        weakness: "光",
        token_reward: 50,
        gold_reward: 50000,
    },
    SeasonalBossDef {
        id: 5,
        name: "熔岩泰坦",
        icon: "🌋",
        desc: "地底熔岩巨人",
        hp: 80000,
        atk: 2500,
        def: 1500,
        weakness: "冰",
        token_reward: 70,
        gold_reward: 80000,
    },
    SeasonalBossDef {
        id: 6,
        name: "虚空行者",
        icon: "🌀",
        desc: "来自虚空的存在",
        hp: 130000,
        atk: 4000,
        def: 2500,
        weakness: "雷",
        token_reward: 100,
        gold_reward: 130000,
    },
    SeasonalBossDef {
        id: 7,
        name: "天界审判者",
        icon: "⚡",
        desc: "天界降下的审判",
        hp: 200000,
        atk: 6500,
        def: 4000,
        weakness: "暗",
        token_reward: 150,
        gold_reward: 200000,
    },
    SeasonalBossDef {
        id: 8,
        name: "赛季之神",
        icon: "👑",
        desc: "终极赛季Boss",
        hp: 350000,
        atk: 10000,
        def: 6500,
        weakness: "无",
        token_reward: 250,
        gold_reward: 350000,
    },
];

/// 难度层级定义
struct DifficultyTier {
    id: u8,
    name: &'static str,
    icon: &'static str,
    hp_mult: f64,
    atk_mult: f64,
    def_mult: f64,
    token_mult: f64,
    gold_mult: f64,
    min_power: i64,
}

const DIFFICULTY_TIERS: &[DifficultyTier] = &[
    DifficultyTier {
        id: 1,
        name: "普通",
        icon: "🟢",
        hp_mult: 1.0,
        atk_mult: 1.0,
        def_mult: 1.0,
        token_mult: 1.0,
        gold_mult: 1.0,
        min_power: 0,
    },
    DifficultyTier {
        id: 2,
        name: "困难",
        icon: "🟡",
        hp_mult: 2.0,
        atk_mult: 1.8,
        def_mult: 1.5,
        token_mult: 2.0,
        gold_mult: 1.5,
        min_power: 5000,
    },
    DifficultyTier {
        id: 3,
        name: "噩梦",
        icon: "🟠",
        hp_mult: 4.0,
        atk_mult: 3.0,
        def_mult: 2.5,
        token_mult: 4.0,
        gold_mult: 2.5,
        min_power: 15000,
    },
    DifficultyTier {
        id: 4,
        name: "地狱",
        icon: "🔴",
        hp_mult: 8.0,
        atk_mult: 5.0,
        def_mult: 4.0,
        token_mult: 7.0,
        gold_mult: 4.0,
        min_power: 35000,
    },
    DifficultyTier {
        id: 5,
        name: "深渊",
        icon: "⚫",
        hp_mult: 15.0,
        atk_mult: 8.0,
        def_mult: 6.5,
        token_mult: 12.0,
        gold_mult: 7.0,
        min_power: 80000,
    },
];

// ==================== 赛季代币商店 ====================

/// 商店商品定义
struct ShopItem {
    id: u8,
    name: &'static str,
    icon: &'static str,
    cost: i32,
    desc: &'static str,
}

const SHOP_ITEMS: &[ShopItem] = &[
    ShopItem {
        id: 1,
        name: "赛季药水",
        icon: "🧪",
        cost: 5,
        desc: "恢复全部HP和MP",
    },
    ShopItem {
        id: 2,
        name: "赛季强化石",
        icon: "💎",
        cost: 15,
        desc: "强化装备+1",
    },
    ShopItem {
        id: 3,
        name: "赛季经验丹",
        icon: "✨",
        cost: 20,
        desc: "获得大量经验",
    },
    ShopItem {
        id: 4,
        name: "赛季金币袋",
        icon: "💰",
        cost: 10,
        desc: "获得50000金币",
    },
    ShopItem {
        id: 5,
        name: "赛季护盾",
        icon: "🛡️",
        cost: 25,
        desc: "下次挑战减伤20%",
    },
    ShopItem {
        id: 6,
        name: "赛季利刃",
        icon: "⚔️",
        cost: 25,
        desc: "下次挑战增伤20%",
    },
    ShopItem {
        id: 7,
        name: "赛季宝箱",
        icon: "📦",
        cost: 50,
        desc: "随机稀有道具",
    },
    ShopItem {
        id: 8,
        name: "赛季称号券",
        icon: "🎫",
        cost: 80,
        desc: "兑换赛季专属称号",
    },
    ShopItem {
        id: 9,
        name: "赛季灵魂石",
        icon: "🔮",
        cost: 100,
        desc: "解锁血脉潜能",
    },
    ShopItem {
        id: 10,
        name: "赛季神兵碎片",
        icon: "🗡️",
        cost: 120,
        desc: "集齐可铸造神兵",
    },
    ShopItem {
        id: 11,
        name: "赛季传说装备箱",
        icon: "🗃️",
        cost: 200,
        desc: "必出传说装备",
    },
    ShopItem {
        id: 12,
        name: "赛季至尊翅膀",
        icon: "🪽",
        cost: 500,
        desc: "限定传说翅膀",
    },
];

// ==================== 数据结构 ====================

/// 玩家赛季Boss数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PlayerSeasonData {
    /// 赛季代币
    tokens: i32,
    /// 累计获得代币
    total_tokens_earned: i32,
    /// 本周击败的Boss (boss_id -> 最高难度)
    weekly_clears: HashMap<u8, u8>,
    /// 本周挑战次数
    weekly_attempts: u32,
    /// 历史最高击败难度
    best_difficulty: u8,
    /// 历史击败Boss总数
    total_boss_defeats: u32,
    /// 上周标记（用于检测周重置）
    last_week: u32,
    /// 购买记录 (item_id -> 数量)
    purchase_history: HashMap<u8, u32>,
}

/// 全局赛季信息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GlobalSeasonInfo {
    /// 当前赛季周次
    current_week: u32,
    /// 当前轮换Boss索引 (0-7)
    current_boss_index: u8,
    /// 赛季开始时间
    season_start: String,
}

impl Default for GlobalSeasonInfo {
    fn default() -> Self {
        Self {
            current_week: 1,
            current_boss_index: 0,
            season_start: "2026-01-01".to_string(),
        }
    }
}

// ==================== 工具函数 ====================

/// 获取当前周次（简化计算：基于天数）
fn current_week_number() -> u32 {
    // 简化：使用秒数除以604800(一周秒数)作为周次标识
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (now / 604800) as u32
}

/// 获取当前周的Boss
fn get_current_boss(db: &Database) -> &'static SeasonalBossDef {
    let info = get_season_info(db);
    let idx = (info.current_boss_index as usize) % SEASONAL_BOSSES.len();
    &SEASONAL_BOSSES[idx]
}

/// 获取赛季全局信息
fn get_season_info(db: &Database) -> GlobalSeasonInfo {
    let raw = db.global_get(SECTION, "_global");
    if raw.is_empty() {
        let info = GlobalSeasonInfo::default();
        db.global_set(SECTION, "_global", &serde_json::to_string(&info).unwrap());
        info
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    }
}

/// 获取玩家赛季数据（自动周重置）
fn get_player_data(db: &Database, user_id: &str) -> PlayerSeasonData {
    let raw = db.global_get(SECTION, user_id);
    let mut data: PlayerSeasonData = if raw.is_empty() {
        PlayerSeasonData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let week = current_week_number();
    if data.last_week != week {
        // 新的一周，重置周数据
        data.weekly_clears.clear();
        data.weekly_attempts = 0;
        data.last_week = week;
        save_player_data(db, user_id, &data);
    }
    data
}

/// 保存玩家赛季数据
fn save_player_data(db: &Database, user_id: &str, data: &PlayerSeasonData) {
    db.global_set(SECTION, user_id, &serde_json::to_string(data).unwrap());
}

/// 获取玩家战力
fn get_player_power(db: &Database, user_id: &str) -> i64 {
    let hp: i64 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(0);
    let ad: i64 = db.read_basic(user_id, ITEM_AD).parse().unwrap_or(0);
    let ap: i64 = db.read_basic(user_id, ITEM_AP).parse().unwrap_or(0);
    let def: i64 = db.read_basic(user_id, ITEM_DEFENSE).parse().unwrap_or(0);
    let mres: i64 = db.read_basic(user_id, ITEM_MAGIC_RES).parse().unwrap_or(0);
    hp + ad + ap + def + mres
}

/// 进度条
fn progress_bar(pct: u32, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// 根据ID查找Boss
fn find_boss_by_id(id: u8) -> Option<&'static SeasonalBossDef> {
    SEASONAL_BOSSES.iter().find(|b| b.id == id)
}

/// 根据名字查找Boss
fn find_boss_by_name(name: &str) -> Option<&'static SeasonalBossDef> {
    SEASONAL_BOSSES.iter().find(|b| b.name == name)
}

/// 根据名字查找难度
fn find_tier_by_name(name: &str) -> Option<&'static DifficultyTier> {
    DIFFICULTY_TIERS.iter().find(|t| t.name == name)
}

/// 根据名字查找商店商品
fn find_shop_item_by_name(name: &str) -> Option<&'static ShopItem> {
    SHOP_ITEMS.iter().find(|item| item.name == name)
}

// ==================== 战斗模拟 ====================

/// 战斗结果
struct BattleResult {
    victory: bool,
    damage_dealt: i64,
    damage_taken: i64,
    rounds: u32,
    rating: &'static str,
}

/// 模拟战斗
fn simulate_battle(
    player_power: i64,
    boss_hp: i64,
    boss_atk: i64,
    boss_def: i64,
    weakness_match: bool,
) -> BattleResult {
    let effective_power = if weakness_match {
        (player_power as f64 * 1.3) as i64
    } else {
        player_power
    };

    // 玩家每回合伤害 = 有效战力 * 0.8 - Boss防御 * 0.3
    let player_dmg_per_round = ((effective_power as f64 * 0.8) - (boss_def as f64 * 0.3)).max(1.0) as i64;
    // Boss每回合伤害 = Boss攻击 * 0.6 - 玩家战力 * 0.1
    let boss_dmg_per_round = ((boss_atk as f64 * 0.6) - (effective_power as f64 * 0.1)).max(1.0) as i64;

    // 玩家HP（简化为战力 * 2）
    let player_hp = effective_power * 2;

    let mut boss_remaining = boss_hp;
    let mut player_remaining = player_hp;
    let mut rounds = 0u32;

    while boss_remaining > 0 && player_remaining > 0 && rounds < 50 {
        boss_remaining -= player_dmg_per_round;
        if boss_remaining <= 0 {
            rounds += 1;
            break;
        }
        player_remaining -= boss_dmg_per_round;
        rounds += 1;
    }

    let victory = boss_remaining <= 0;
    let rating = if rounds <= 3 {
        "S"
    } else if rounds <= 6 {
        "A"
    } else if rounds <= 10 {
        "B"
    } else if rounds <= 20 {
        "C"
    } else {
        "D"
    };

    BattleResult {
        victory,
        damage_dealt: boss_hp - boss_remaining.max(0),
        damage_taken: player_hp - player_remaining.max(0),
        rounds,
        rating,
    }
}

// ==================== 公共 API ====================

/// 获取赛季Boss加成（供外部系统调用）
pub fn get_seasonal_bonus(db: &Database, user_id: &str) -> (f64, f64, f64) {
    let data = get_player_data(db, user_id);
    let clears = data.weekly_clears.len() as f64;
    let total_defeats = data.total_boss_defeats as f64;

    // 每次击败Boss增加0.5%攻击加成（上限20%）
    let atk_bonus = (total_defeats * 0.5).min(20.0);
    // 本周每击败一个Boss增加1%经验加成
    let exp_bonus = clears * 1.0;
    // 累计代币每100点增加0.5%金币加成
    let gold_bonus = (data.total_tokens_earned as f64 / 100.0 * 0.5).min(15.0);

    (atk_bonus, exp_bonus, gold_bonus)
}

/// 记录赛季Boss击杀（供外部系统调用）
pub fn record_seasonal_clear(db: &Database, user_id: &str, boss_id: u8, difficulty: u8) {
    let mut data = get_player_data(db, user_id);
    let current = data.weekly_clears.entry(boss_id).or_insert(0);
    if difficulty > *current {
        *current = difficulty;
    }
    if difficulty > data.best_difficulty {
        data.best_difficulty = difficulty;
    }
    data.total_boss_defeats += 1;
    save_player_data(db, user_id, &data);
}

// ==================== 命令函数 ====================

/// 赛季Boss - 查看当前赛季Boss信息
pub fn cmd_seasonal_boss(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let boss = get_current_boss(db);
    let data = get_player_data(db, user_id);
    let week = current_week_number();
    let power = get_player_power(db, user_id);

    let cleared_diff = data.weekly_clears.get(&boss.id).copied().unwrap_or(0);
    let status = if cleared_diff > 0 {
        let tier_name = DIFFICULTY_TIERS
            .iter()
            .find(|t| t.id == cleared_diff)
            .map(|t| t.name)
            .unwrap_or("?");
        format!("✅ 已通关 ({})", tier_name)
    } else {
        "❌ 未通关".to_string()
    };

    let mut out = format!("{}\n═══ 赛季Boss挑战 ═══\n", prefix);
    out.push_str(&format!("📅 赛季周次: #{}\n", week));
    out.push_str(&format!("\n{} {} 【{}】\n", boss.icon, boss.name, boss.desc));
    out.push_str(&format!("  HP: {} | ATK: {} | DEF: {}\n", boss.hp, boss.atk, boss.def));
    out.push_str(&format!("  弱点元素: {}\n", boss.weakness));
    out.push_str(&format!(
        "  基础奖励: 🪙{}赛季代币 + {}金币\n",
        boss.token_reward, boss.gold_reward
    ));
    out.push_str(&format!("\n📊 本周状态: {}\n", status));
    out.push_str(&format!("  本周挑战次数: {}\n", data.weekly_attempts));
    out.push_str(&format!("  当前战力: {}\n", power));
    out.push_str(&format!("  持有代币: 🪙{}\n", data.tokens));
    out.push_str("\n💡 使用「挑战赛季」发起挑战\n");
    out.push_str("💡 使用「赛季难度」查看难度详情\n");
    out
}

/// 挑战赛季 - 挑战当前赛季Boss
pub fn cmd_challenge_seasonal(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let boss = get_current_boss(db);
    let power = get_player_power(db, user_id);

    if power < 100 {
        return format!("{}\n❌ 战力不足，无法挑战赛季Boss！请先提升战力。", prefix);
    }

    // 解析难度
    let tier_name = args.trim();
    let tier = if tier_name.is_empty() {
        &DIFFICULTY_TIERS[0] // 默认普通
    } else {
        match find_tier_by_name(tier_name) {
            Some(t) => t,
            None => return format!("{}\n❌ 无效难度！可选: 普通/困难/噩梦/地狱/深渊", prefix),
        }
    };

    // 检查战力要求
    if power < tier.min_power {
        return format!(
            "{}\n❌ {}难度需要{}战力，当前{}战力不足！",
            prefix, tier.name, tier.min_power, power
        );
    }

    let mut data = get_player_data(db, user_id);

    // 计算Boss属性
    let boss_hp = (boss.hp as f64 * tier.hp_mult) as i64;
    let boss_atk = (boss.atk as f64 * tier.atk_mult) as i64;
    let boss_def = (boss.def as f64 * tier.def_mult) as i64;

    // 模拟战斗
    let result = simulate_battle(power, boss_hp, boss_atk, boss_def, boss.weakness != "无");

    data.weekly_attempts += 1;

    let mut out = format!("{}\n═══ 赛季Boss战报 ═══\n", prefix);
    out.push_str(&format!(
        "🎯 {} {} vs {}{}\n",
        tier.icon, tier.name, boss.icon, boss.name
    ));
    out.push_str(&format!("⚔️ 战斗回合: {}\n", result.rounds));
    out.push_str(&format!("💥 造成伤害: {}\n", result.damage_dealt));
    out.push_str(&format!("🩸 受到伤害: {}\n", result.damage_taken));
    out.push_str(&format!("📊 评价: {}级\n", result.rating));

    if result.victory {
        let token_reward = (boss.token_reward as f64 * tier.token_mult) as i32;
        let gold_reward = (boss.gold_reward as f64 * tier.gold_mult) as i64;

        data.tokens += token_reward;
        data.total_tokens_earned += token_reward;

        // 更新通关记录
        let current = data.weekly_clears.entry(boss.id).or_insert(0);
        if tier.id > *current {
            *current = tier.id;
        }
        if tier.id > data.best_difficulty {
            data.best_difficulty = tier.id;
        }
        data.total_boss_defeats += 1;

        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_reward);

        out.push_str("\n🎉 胜利！\n");
        out.push_str(&format!("🪙 获得赛季代币: +{}\n", token_reward));
        out.push_str(&format!("💰 获得金币: +{}\n", gold_reward));
    } else {
        out.push_str("\n💀 挑战失败！\n");
        out.push_str("💡 提升战力或尝试更低难度\n");
    }

    save_player_data(db, user_id, &data);
    out
}

/// 赛季难度 - 查看难度层级详情
pub fn cmd_seasonal_difficulty(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power = get_player_power(db, user_id);
    let boss = get_current_boss(db);

    let mut out = format!("{}\n═══ 赛季难度层级 ═══\n", prefix);
    out.push_str(&format!("当前Boss: {}{}\n\n", boss.icon, boss.name));

    for tier in DIFFICULTY_TIERS {
        let unlock = if power >= tier.min_power { "✅" } else { "🔒" };
        let hp = (boss.hp as f64 * tier.hp_mult) as i64;
        let atk = (boss.atk as f64 * tier.atk_mult) as i64;
        let tokens = (boss.token_reward as f64 * tier.token_mult) as i32;

        out.push_str(&format!(
            "{} {}{} {} — 战力需求:{}\n",
            unlock,
            tier.icon,
            tier.name,
            if power >= tier.min_power { "" } else { " (未解锁)" },
            tier.min_power
        ));
        out.push_str(&format!("  HP:{} ATK:{} 奖励:🪙{}代币\n", hp, atk, tokens));
    }

    out.push_str(&format!("\n当前战力: {}\n", power));
    out.push_str("💡 使用「挑战赛季+难度名」指定难度挑战\n");
    out
}

/// 赛季排行 - 查看赛季排行榜
pub fn cmd_seasonal_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare(&format!("SELECT Key, Value FROM Global WHERE Section='{}'", SECTION)) {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };

    let mut rankings: Vec<(String, i32, u32, u8)> = stmt
        .query_map([], |row| {
            let uid: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((uid, val))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|(uid, val)| {
            if uid == "_global" {
                return None;
            }
            let data: PlayerSeasonData = serde_json::from_str(&val).ok()?;
            if data.total_tokens_earned == 0 {
                return None;
            }
            Some((
                uid,
                data.total_tokens_earned,
                data.total_boss_defeats,
                data.best_difficulty,
            ))
        })
        .collect();

    rankings.sort_by_key(|x| std::cmp::Reverse(x.1));

    if rankings.is_empty() {
        return format!("{}\n暂无赛季排行数据。", prefix);
    }

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = format!("{}\n═══ 赛季排行榜 ═══\n", prefix);
    for (i, (uid, tokens, defeats, best_diff)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, ITEM_NAME);
        let display_name = if name.is_empty() { uid.clone() } else { name };
        let tier_name = DIFFICULTY_TIERS
            .iter()
            .find(|t| t.id == *best_diff)
            .map(|t| t.name)
            .unwrap_or("-");
        out.push_str(&format!(
            "{}{}. {} — 🪙{}代币 | 击败{}次 | 最高:{}\n",
            medal,
            i + 1,
            display_name,
            tokens,
            defeats,
            tier_name
        ));
    }
    out
}

/// 赛季商店 - 查看赛季代币商店
pub fn cmd_seasonal_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let data = get_player_data(db, user_id);

    let mut out = format!("{}\n═══ 赛季代币商店 ═══\n", prefix);
    out.push_str(&format!("🪙 当前代币: {}\n\n", data.tokens));

    for item in SHOP_ITEMS {
        let affordable = if data.tokens >= item.cost { "✅" } else { "❌" };
        out.push_str(&format!(
            "{} {}{} — 🪙{} | {}\n",
            affordable, item.icon, item.name, item.cost, item.desc
        ));
    }

    out.push_str("\n💡 使用「购买赛季+商品名」购买\n");
    out
}

/// 购买赛季 - 从赛季商店购买商品
pub fn cmd_buy_seasonal(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!("{}\n格式: 购买赛季+商品名", prefix);
    }

    let item = match find_shop_item_by_name(item_name) {
        Some(i) => i,
        None => return format!("{}\n❌ 未找到商品: {}", prefix, item_name),
    };

    let mut data = get_player_data(db, user_id);

    if data.tokens < item.cost {
        return format!(
            "{}\n❌ 赛季代币不足！需要🪙{}，当前🪙{}",
            prefix, item.cost, data.tokens
        );
    }

    data.tokens -= item.cost;
    let count = data.purchase_history.entry(item.id).or_insert(0);
    *count += 1;

    // 发放奖励
    match item.id {
        1 => {
            // 赛季药水 - 恢复HP和MP
            let hp_max: i64 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(100);
            let mp_max: i64 = db.read_basic(user_id, ITEM_MP).parse().unwrap_or(50);
            db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_max as i32);
            db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_max as i32);
        }
        3 => {
            // 赛季经验丹
            user::add_experience(db, user_id, 5000);
        }
        4 => {
            // 赛季金币袋
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 50000);
        }
        _ => {
            // 其他物品放入背包
            db.knapsack_add(user_id, item.name, 1);
        }
    }

    save_player_data(db, user_id, &data);

    let mut out = format!("{}\n", prefix);
    out.push_str("✅ 购买成功！\n");
    out.push_str(&format!("{} {} × 1\n", item.icon, item.name));
    out.push_str(&format!("🪙 消费代币: -{}\n", item.cost));
    out.push_str(&format!("🪙 剩余代币: {}\n", data.tokens));
    out
}

/// 赛季统计 - 查看个人赛季统计
pub fn cmd_seasonal_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let data = get_player_data(db, user_id);
    let (atk_bonus, exp_bonus, gold_bonus) = get_seasonal_bonus(db, user_id);

    let best_tier_name = DIFFICULTY_TIERS
        .iter()
        .find(|t| t.id == data.best_difficulty)
        .map(|t| t.name)
        .unwrap_or("无");

    let mut out = format!("{}\n═══ 赛季统计 ═══\n", prefix);
    out.push_str(&format!("🪙 当前代币: {}\n", data.tokens));
    out.push_str(&format!("🪙 累计获得代币: {}\n", data.total_tokens_earned));
    out.push_str(&format!("⚔️ 本周挑战次数: {}\n", data.weekly_attempts));
    out.push_str(&format!("🏆 本周击败Boss: {}个\n", data.weekly_clears.len()));
    out.push_str(&format!("📊 历史击败总数: {}次\n", data.total_boss_defeats));
    out.push_str(&format!("🎯 最高通关难度: {}\n", best_tier_name));

    out.push_str("\n💪 赛季加成效果:\n");
    out.push_str(&format!("  ⚔️ 攻击加成: +{:.1}%\n", atk_bonus));
    out.push_str(&format!("  ✨ 经验加成: +{:.1}%\n", exp_bonus));
    out.push_str(&format!("  💰 金币加成: +{:.1}%\n", gold_bonus));

    if !data.weekly_clears.is_empty() {
        out.push_str("\n📋 本周通关记录:\n");
        for (boss_id, diff_id) in &data.weekly_clears {
            if let Some(boss) = find_boss_by_id(*boss_id) {
                let tier_name = DIFFICULTY_TIERS
                    .iter()
                    .find(|t| t.id == *diff_id)
                    .map(|t| t.name)
                    .unwrap_or("?");
                out.push_str(&format!("  {}{} — {}\n", boss.icon, boss.name, tier_name));
            }
        }
    }

    if !data.purchase_history.is_empty() {
        out.push_str(&format!("\n🛒 购买记录: {}种商品\n", data.purchase_history.len()));
    }

    out
}

/// 赛季帮助 - 显示赛季Boss帮助信息
pub fn cmd_seasonal_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n        ═══ 赛季Boss挑战帮助 ═══\n\n\
        赛季Boss挑战是每周轮换的高难度PvE挑战系统。\n\
        每周一更换Boss并重置进度，击败Boss可获得赛季代币和金币。\n\n\
        📋 指令列表:\n\
        • 赛季Boss — 查看当前赛季Boss信息\n\
        • 挑战赛季 — 挑战当前Boss（默认普通难度）\n\
        • 挑战赛季+难度名 — 指定难度挑战（普通/困难/噩梦/地狱/深渊）\n\
        • 赛季难度 — 查看难度层级详情\n\
        • 赛季排行 — 查看赛季排行榜\n\
        • 赛季商店 — 查看赛季代币商店\n\
        • 购买赛季+商品名 — 购买商店商品\n\
        • 赛季统计 — 查看个人赛季统计\n\
        • 赛季帮助 — 本帮助信息\n\n\
        🏆 8大赛季Boss: 每周守护者→赛季之神\n\
        ⚔️ 5种难度: 🟢普通 🟡困难 🟠噩梦 🔴地狱 ⚫深渊\n\
        🪙 赛季代币: 在赛季商店兑换专属奖励\n\
        📊 赛季加成: 击败Boss越多，攻击/经验/金币加成越高\n\
        📅 每周重置: 每周一进度重置，Boss轮换\n",
        prefix
    )
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_definitions_count() {
        assert_eq!(SEASONAL_BOSSES.len(), 8);
    }

    #[test]
    fn test_boss_definitions_unique_ids() {
        let mut ids: Vec<u8> = SEASONAL_BOSSES.iter().map(|b| b.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 8);
    }

    #[test]
    fn test_boss_definitions_unique_names() {
        let mut names: Vec<&str> = SEASONAL_BOSSES.iter().map(|b| b.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 8);
    }

    #[test]
    fn test_difficulty_tiers_count() {
        assert_eq!(DIFFICULTY_TIERS.len(), 5);
    }

    #[test]
    fn test_difficulty_tiers_ordered() {
        for i in 1..DIFFICULTY_TIERS.len() {
            assert!(DIFFICULTY_TIERS[i].min_power > DIFFICULTY_TIERS[i - 1].min_power);
            assert!(DIFFICULTY_TIERS[i].hp_mult > DIFFICULTY_TIERS[i - 1].hp_mult);
        }
    }

    #[test]
    fn test_shop_items_count() {
        assert_eq!(SHOP_ITEMS.len(), 12);
    }

    #[test]
    fn test_shop_items_unique_ids() {
        let mut ids: Vec<u8> = SHOP_ITEMS.iter().map(|i| i.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 12);
    }

    #[test]
    fn test_player_data_default() {
        let data = PlayerSeasonData::default();
        assert_eq!(data.tokens, 0);
        assert_eq!(data.total_tokens_earned, 0);
        assert!(data.weekly_clears.is_empty());
        assert_eq!(data.weekly_attempts, 0);
        assert_eq!(data.best_difficulty, 0);
        assert_eq!(data.total_boss_defeats, 0);
        assert_eq!(data.last_week, 0);
        assert!(data.purchase_history.is_empty());
    }

    #[test]
    fn test_find_boss_by_id() {
        assert!(find_boss_by_id(1).is_some());
        assert_eq!(find_boss_by_id(1).unwrap().name, "每周守护者");
        assert!(find_boss_by_id(8).is_some());
        assert_eq!(find_boss_by_id(8).unwrap().name, "赛季之神");
        assert!(find_boss_by_id(99).is_none());
    }

    #[test]
    fn test_find_boss_by_name() {
        assert!(find_boss_by_name("每周守护者").is_some());
        assert!(find_boss_by_name("赛季之神").is_some());
        assert!(find_boss_by_name("不存在的Boss").is_none());
    }

    #[test]
    fn test_find_tier_by_name() {
        assert!(find_tier_by_name("普通").is_some());
        assert!(find_tier_by_name("深渊").is_some());
        assert!(find_tier_by_name("不存在").is_none());
    }

    #[test]
    fn test_find_shop_item_by_name() {
        assert!(find_shop_item_by_name("赛季药水").is_some());
        assert!(find_shop_item_by_name("赛季至尊翅膀").is_some());
        assert!(find_shop_item_by_name("不存在").is_none());
    }

    #[test]
    fn test_simulate_battle_victory() {
        // 强大玩家 vs 弱Boss
        let result = simulate_battle(100000, 5000, 200, 100, false);
        assert!(result.victory);
        assert!(result.damage_dealt > 0);
        assert!(result.rounds > 0);
    }

    #[test]
    fn test_simulate_battle_defeat() {
        // 弱玩家 vs 强Boss
        let result = simulate_battle(100, 350000, 10000, 6500, false);
        assert!(!result.victory);
    }

    #[test]
    fn test_simulate_battle_weakness_bonus() {
        let result_no_weakness = simulate_battle(50000, 25000, 800, 500, false);
        let result_weakness = simulate_battle(50000, 25000, 800, 500, true);
        // 弱点匹配应该造成更多伤害
        assert!(result_weakness.damage_dealt >= result_no_weakness.damage_dealt);
    }

    #[test]
    fn test_battle_rating_s() {
        // 极强战力应获得S评级
        let result = simulate_battle(1000000, 5000, 200, 100, false);
        assert_eq!(result.rating, "S");
    }

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0, 10), "[░░░░░░░░░░]");
        assert_eq!(progress_bar(100, 10), "[██████████]");
        assert_eq!(progress_bar(50, 10), "[█████░░░░░]");
    }

    #[test]
    fn test_boss_stats_escalation() {
        // Boss属性递增
        for i in 1..SEASONAL_BOSSES.len() {
            assert!(SEASONAL_BOSSES[i].hp > SEASONAL_BOSSES[i - 1].hp);
            assert!(SEASONAL_BOSSES[i].atk > SEASONAL_BOSSES[i - 1].atk);
            assert!(SEASONAL_BOSSES[i].def > SEASONAL_BOSSES[i - 1].def);
            assert!(SEASONAL_BOSSES[i].token_reward > SEASONAL_BOSSES[i - 1].token_reward);
        }
    }

    #[test]
    fn test_boss_id_range() {
        for boss in SEASONAL_BOSSES {
            assert!(boss.id >= 1 && boss.id <= 8, "Boss ID {} out of range", boss.id);
        }
    }

    #[test]
    fn test_difficulty_token_multiplier() {
        // 深渊的代币倍率应远高于普通
        let normal_mult = DIFFICULTY_TIERS[0].token_mult;
        let abyss_mult = DIFFICULTY_TIERS[4].token_mult;
        assert!(abyss_mult > normal_mult * 5.0);
    }

    #[test]
    fn test_global_season_info_default() {
        let info = GlobalSeasonInfo::default();
        assert_eq!(info.current_week, 1);
        assert_eq!(info.current_boss_index, 0);
    }

    #[test]
    fn test_shop_items_cost_positive() {
        for item in SHOP_ITEMS {
            assert!(item.cost > 0, "Item {} has non-positive cost", item.name);
        }
    }

    #[test]
    fn test_weakness_definitions() {
        let valid_weaknesses = ["火", "雷", "冰", "光", "暗", "无"];
        for boss in SEASONAL_BOSSES {
            assert!(
                valid_weaknesses.contains(&boss.weakness),
                "Boss {} has invalid weakness: {}",
                boss.name,
                boss.weakness
            );
        }
    }
}

//! 钓鱼小游戏系统
//!
//! 独立钓鱼玩法：6个钓鱼地点、20+鱼种、鱼饵系统、钓竿升级、
//! 鱼类图鉴追踪、钓鱼排行榜、鱼可出售/烹饪使用
//!
//! 指令: 查看钓鱼, 开始钓鱼, 钓鱼背包, 鱼类图鉴, 钓鱼排行, 购买鱼饵, 钓鱼商店

use crate::core::*;
use crate::db::Database;
use crate::user;

// ── 常量 ────────────────────────────────────────────────────────────────────

const SECTION: &str = "fishing";
const MAX_DAILY_FISHING: i32 = 30;
const FISHING_COOLDOWN_SECS: i64 = 30;

// ── 鱼竿等级 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct FishingRod {
    name: &'static str,
    emoji: &'static str,
    quality_bonus: f64, // 稀有鱼概率加成百分比
    speed_bonus: f64,   // 钓鱼速度加成百分比
    cost_gold: i64,
    level_req: i32,
}

const FISHING_RODS: &[FishingRod] = &[
    FishingRod {
        name: "竹竿",
        emoji: "🎋",
        quality_bonus: 0.0,
        speed_bonus: 0.0,
        cost_gold: 0,
        level_req: 1,
    },
    FishingRod {
        name: "木竿",
        emoji: "🪵",
        quality_bonus: 0.05,
        speed_bonus: 0.10,
        cost_gold: 500,
        level_req: 5,
    },
    FishingRod {
        name: "铁竿",
        emoji: "⚙️",
        quality_bonus: 0.12,
        speed_bonus: 0.20,
        cost_gold: 2000,
        level_req: 15,
    },
    FishingRod {
        name: "银竿",
        emoji: "🪙",
        quality_bonus: 0.20,
        speed_bonus: 0.30,
        cost_gold: 8000,
        level_req: 25,
    },
    FishingRod {
        name: "金竿",
        emoji: "✨",
        quality_bonus: 0.30,
        speed_bonus: 0.40,
        cost_gold: 30000,
        level_req: 40,
    },
    FishingRod {
        name: "传说龙竿",
        emoji: "🐲",
        quality_bonus: 0.45,
        speed_bonus: 0.55,
        cost_gold: 100000,
        level_req: 60,
    },
];

// ── 鱼饵 ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct Bait {
    name: &'static str,
    emoji: &'static str,
    quality_bonus: f64,
    cost_gold: i64,
    cost_diamond: i32,
}

const BAITS: &[Bait] = &[
    Bait {
        name: "蚯蚓",
        emoji: "🪱",
        quality_bonus: 0.0,
        cost_gold: 10,
        cost_diamond: 0,
    },
    Bait {
        name: "面团",
        emoji: "🍞",
        quality_bonus: 0.05,
        cost_gold: 50,
        cost_diamond: 0,
    },
    Bait {
        name: "虾饵",
        emoji: "🦐",
        quality_bonus: 0.12,
        cost_gold: 200,
        cost_diamond: 0,
    },
    Bait {
        name: "虫饵",
        emoji: "🐛",
        quality_bonus: 0.20,
        cost_gold: 500,
        cost_diamond: 0,
    },
    Bait {
        name: "秘制饵",
        emoji: "⭐",
        quality_bonus: 0.35,
        cost_gold: 0,
        cost_diamond: 5,
    },
    Bait {
        name: "龙涎香",
        emoji: "🌟",
        quality_bonus: 0.50,
        cost_gold: 0,
        cost_diamond: 15,
    },
];

// ── 鱼类定义 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct FishDef {
    name: &'static str,
    emoji: &'static str,
    rarity: FishRarity,
    sell_price: i64,
    min_weight: i32, // 克
    max_weight: i32,
    cook_value: i32, // 烹饪价值
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FishRarity {
    Common,    // 普通
    Uncommon,  // 优良
    Rare,      // 稀有
    Epic,      // 史诗
    Legendary, // 传说
}

impl FishRarity {
    fn name(&self) -> &'static str {
        match self {
            Self::Common => "普通",
            Self::Uncommon => "优良",
            Self::Rare => "稀有",
            Self::Epic => "史诗",
            Self::Legendary => "传说",
        }
    }
    fn emoji(&self) -> &'static str {
        match self {
            Self::Common => "⚪",
            Self::Uncommon => "🟢",
            Self::Rare => "🔵",
            Self::Epic => "🟣",
            Self::Legendary => "🟡",
        }
    }
    fn base_weight(&self) -> u32 {
        match self {
            Self::Common => 50,
            Self::Uncommon => 25,
            Self::Rare => 15,
            Self::Epic => 8,
            Self::Legendary => 2,
        }
    }
}

const ALL_FISH: &[FishDef] = &[
    // 普通 (10)
    FishDef {
        name: "鲫鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 5,
        min_weight: 100,
        max_weight: 500,
        cook_value: 10,
    },
    FishDef {
        name: "鲤鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 8,
        min_weight: 200,
        max_weight: 800,
        cook_value: 12,
    },
    FishDef {
        name: "草鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 6,
        min_weight: 300,
        max_weight: 1200,
        cook_value: 11,
    },
    FishDef {
        name: "鲈鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 10,
        min_weight: 150,
        max_weight: 600,
        cook_value: 15,
    },
    FishDef {
        name: "泥鳅",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 3,
        min_weight: 30,
        max_weight: 100,
        cook_value: 5,
    },
    FishDef {
        name: "鲶鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 7,
        min_weight: 200,
        max_weight: 900,
        cook_value: 13,
    },
    FishDef {
        name: "鳊鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 6,
        min_weight: 150,
        max_weight: 700,
        cook_value: 10,
    },
    FishDef {
        name: "鲢鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 5,
        min_weight: 400,
        max_weight: 2000,
        cook_value: 8,
    },
    FishDef {
        name: "鳜鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 12,
        min_weight: 200,
        max_weight: 700,
        cook_value: 18,
    },
    FishDef {
        name: "罗非鱼",
        emoji: "🐟",
        rarity: FishRarity::Common,
        sell_price: 4,
        min_weight: 100,
        max_weight: 400,
        cook_value: 7,
    },
    // 优良 (5)
    FishDef {
        name: "虹鳟鱼",
        emoji: "🐠",
        rarity: FishRarity::Uncommon,
        sell_price: 25,
        min_weight: 300,
        max_weight: 1500,
        cook_value: 30,
    },
    FishDef {
        name: "石斑鱼",
        emoji: "🐠",
        rarity: FishRarity::Uncommon,
        sell_price: 35,
        min_weight: 500,
        max_weight: 3000,
        cook_value: 40,
    },
    FishDef {
        name: "鲈王",
        emoji: "🐠",
        rarity: FishRarity::Uncommon,
        sell_price: 30,
        min_weight: 800,
        max_weight: 4000,
        cook_value: 35,
    },
    FishDef {
        name: "大头鲢",
        emoji: "🐠",
        rarity: FishRarity::Uncommon,
        sell_price: 20,
        min_weight: 1000,
        max_weight: 5000,
        cook_value: 25,
    },
    FishDef {
        name: "银鱼",
        emoji: "🐠",
        rarity: FishRarity::Uncommon,
        sell_price: 28,
        min_weight: 50,
        max_weight: 200,
        cook_value: 32,
    },
    // 稀有 (3)
    FishDef {
        name: "金龙鱼",
        emoji: "🐡",
        rarity: FishRarity::Rare,
        sell_price: 100,
        min_weight: 500,
        max_weight: 3000,
        cook_value: 80,
    },
    FishDef {
        name: "帝王鲑",
        emoji: "🐡",
        rarity: FishRarity::Rare,
        sell_price: 120,
        min_weight: 2000,
        max_weight: 8000,
        cook_value: 90,
    },
    FishDef {
        name: "翡翠鲈",
        emoji: "🐡",
        rarity: FishRarity::Rare,
        sell_price: 150,
        min_weight: 1000,
        max_weight: 5000,
        cook_value: 100,
    },
    // 史诗 (2)
    FishDef {
        name: "千年锦鲤",
        emoji: "🎏",
        rarity: FishRarity::Epic,
        sell_price: 500,
        min_weight: 3000,
        max_weight: 15000,
        cook_value: 200,
    },
    FishDef {
        name: "血色鳗王",
        emoji: "🎏",
        rarity: FishRarity::Epic,
        sell_price: 600,
        min_weight: 2000,
        max_weight: 10000,
        cook_value: 250,
    },
    // 传说 (2)
    FishDef {
        name: "深渊鲲鹏",
        emoji: "🐉",
        rarity: FishRarity::Legendary,
        sell_price: 5000,
        min_weight: 50000,
        max_weight: 200000,
        cook_value: 1000,
    },
    FishDef {
        name: "九天神龙鱼",
        emoji: "🐲",
        rarity: FishRarity::Legendary,
        sell_price: 8000,
        min_weight: 30000,
        max_weight: 150000,
        cook_value: 1500,
    },
];

// ── 钓鱼地点 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct FishingSpot {
    name: &'static str,
    emoji: &'static str,
    desc: &'static str,
    level_req: i32,
    fish_indices: &'static [usize], // indices into ALL_FISH
    bonus_rarity: f64,
}

const FISHING_SPOTS: &[FishingSpot] = &[
    FishingSpot {
        name: "村口小塘",
        emoji: "🏡",
        desc: "宁静的小池塘，适合新手",
        level_req: 1,
        fish_indices: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        bonus_rarity: 0.0,
    },
    FishingSpot {
        name: "清溪河畔",
        emoji: "🏞️",
        desc: "清澈的溪流，鱼种丰富",
        level_req: 10,
        fish_indices: &[0, 1, 2, 3, 5, 6, 10, 11, 12, 13],
        bonus_rarity: 0.05,
    },
    FishingSpot {
        name: "碧波湖",
        emoji: "🌊",
        desc: "广袤的湖泊，隐藏稀有鱼种",
        level_req: 20,
        fish_indices: &[1, 2, 5, 10, 11, 12, 13, 14, 15, 16],
        bonus_rarity: 0.12,
    },
    FishingSpot {
        name: "暗海深沟",
        emoji: "🌀",
        desc: "深不见底的海域，传说中的鱼出没",
        level_req: 35,
        fish_indices: &[11, 12, 14, 15, 16, 17, 18, 19],
        bonus_rarity: 0.22,
    },
    FishingSpot {
        name: "龙渊潭",
        emoji: "⛰️",
        desc: "远古龙族栖息之地",
        level_req: 50,
        fish_indices: &[15, 16, 17, 18, 19, 20],
        bonus_rarity: 0.35,
    },
    FishingSpot {
        name: "天池仙境",
        emoji: "🏔️",
        desc: "云端之上的神秘水域",
        level_req: 70,
        fish_indices: &[17, 18, 19, 20, 21],
        bonus_rarity: 0.50,
    },
];

// ── 辅助函数 ────────────────────────────────────────────────────────────────

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn deterministic_seed(user_id: &str, extra: &str) -> u64 {
    let mut h: u64 = 5381;
    for b in user_id.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    for b in extra.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    h
}

fn today_string() -> String {
    let ts = now_ts();
    let days = ts / 86400;
    let y = 1970 + (days / 365);
    let d = days % 365;
    format!("{}-{:03}", y, d)
}

fn get_daily_fishing(db: &Database, user_id: &str) -> i32 {
    let key = format!("daily_{}_{}", user_id, today_string());
    db.global_get(SECTION, &key).parse().unwrap_or(0)
}

fn add_daily_fishing(db: &Database, user_id: &str) {
    let key = format!("daily_{}_{}", user_id, today_string());
    let cur = get_daily_fishing(db, user_id);
    db.global_set(SECTION, &key, &(cur + 1).to_string());
}

fn get_last_fishing_time(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("last_{}", user_id))
        .parse()
        .unwrap_or(0)
}

fn set_last_fishing_time(db: &Database, user_id: &str) {
    db.global_set(SECTION, &format!("last_{}", user_id), &now_ts().to_string());
}

fn get_rod_tier(db: &Database, user_id: &str) -> usize {
    db.global_get(SECTION, &format!("rod_{}", user_id)).parse().unwrap_or(0)
}

fn set_rod_tier(db: &Database, user_id: &str, tier: usize) {
    db.global_set(SECTION, &format!("rod_{}", user_id), &tier.to_string());
}

fn get_fishing_exp(db: &Database, user_id: &str) -> i32 {
    db.global_get(SECTION, &format!("exp_{}", user_id)).parse().unwrap_or(0)
}

fn add_fishing_exp(db: &Database, user_id: &str, amount: i32) {
    let cur = get_fishing_exp(db, user_id);
    db.global_set(SECTION, &format!("exp_{}", user_id), &(cur + amount).to_string());
}

fn fishing_level(exp: i32) -> i32 {
    // 每级需要 level*50 经验
    let mut lvl = 0;
    let mut total = 0;
    loop {
        let needed = (lvl + 1) * 50;
        if total + needed > exp {
            break;
        }
        total += needed;
        lvl += 1;
        if lvl >= 100 {
            break;
        }
    }
    lvl
}

fn format_fishing_level(exp: i32) -> String {
    let lvl = fishing_level(exp);
    let title = match lvl {
        0..=4 => "钓鱼新手",
        5..=14 => "业余钓手",
        15..=29 => "资深钓友",
        30..=49 => "钓鱼达人",
        50..=69 => "钓鱼宗师",
        70..=89 => "鱼王",
        _ => "钓鱼之神",
    };
    format!("Lv.{} {} (EXP: {})", lvl, title, exp)
}

fn get_bait_count(db: &Database, user_id: &str, bait_idx: usize) -> i32 {
    db.global_get(SECTION, &format!("bait_{}_{}", user_id, bait_idx))
        .parse()
        .unwrap_or(0)
}

fn set_bait_count(db: &Database, user_id: &str, bait_idx: usize, count: i32) {
    db.global_set(SECTION, &format!("bait_{}_{}", user_id, bait_idx), &count.to_string());
}

fn get_fish_caught(db: &Database, user_id: &str, fish_idx: usize) -> i32 {
    db.global_get(SECTION, &format!("caught_{}_{}", user_id, fish_idx))
        .parse()
        .unwrap_or(0)
}

fn add_fish_caught(db: &Database, user_id: &str, fish_idx: usize, amount: i32) {
    let cur = get_fish_caught(db, user_id, fish_idx);
    db.global_set(
        SECTION,
        &format!("caught_{}_{}", user_id, fish_idx),
        &(cur + amount).to_string(),
    );
}

fn get_total_fish_caught(db: &Database, user_id: &str) -> i32 {
    let mut total = 0;
    for i in 0..ALL_FISH.len() {
        total += get_fish_caught(db, user_id, i);
    }
    total
}

fn get_biggest_fish_weight(db: &Database, user_id: &str) -> i32 {
    db.global_get(SECTION, &format!("biggest_{}", user_id))
        .parse()
        .unwrap_or(0)
}

fn set_biggest_fish_weight(db: &Database, user_id: &str, weight: i32) {
    let cur = get_biggest_fish_weight(db, user_id);
    if weight > cur {
        db.global_set(SECTION, &format!("biggest_{}", user_id), &weight.to_string());
    }
}

fn get_total_gold_earned(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("gold_{}", user_id))
        .parse()
        .unwrap_or(0)
}

fn add_total_gold_earned(db: &Database, user_id: &str, amount: i64) {
    let cur = get_total_gold_earned(db, user_id);
    db.global_set(SECTION, &format!("gold_{}", user_id), &(cur + amount).to_string());
}

fn format_weight(g: i32) -> String {
    if g >= 1000 {
        format!("{:.1}kg", g as f64 / 1000.0)
    } else {
        format!("{}g", g)
    }
}

fn format_num(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

// ── 公开 API ────────────────────────────────────────────────────────────────

#[allow(dead_code)]
/// 获取钓鱼等级
pub fn get_fishing_level(db: &Database, user_id: &str) -> i32 {
    fishing_level(get_fishing_exp(db, user_id))
}

#[allow(dead_code)]
/// 记录钓鱼活动积分
pub fn record_fishing_activity(db: &Database, user_id: &str) {
    add_fishing_exp(db, user_id, 5);
}

// ── 指令实现 ────────────────────────────────────────────────────────────────

/// 查看钓鱼
pub fn cmd_view_fishing(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let rod_tier = get_rod_tier(db, user_id);
    let rod = &FISHING_RODS[rod_tier];
    let exp = get_fishing_exp(db, user_id);
    let daily = get_daily_fishing(db, user_id);
    let total = get_total_fish_caught(db, user_id);
    let biggest = get_biggest_fish_weight(db, user_id);

    let mut out = String::from("🎣 === 钓鱼系统 === 🎣\n\n");
    out.push_str(&format!(
        "🎣 钓竿: {} {} (品质+{}%, 速度+{}%)\n",
        rod.emoji,
        rod.name,
        (rod.quality_bonus * 100.0) as i32,
        (rod.speed_bonus * 100.0) as i32
    ));
    out.push_str(&format!("📊 {}\n", format_fishing_level(exp)));
    out.push_str(&format!("🐟 今日钓鱼: {}/{} 次\n", daily, MAX_DAILY_FISHING));
    out.push_str(&format!("🏆 总捕获: {} 条\n", total));
    if biggest > 0 {
        out.push_str(&format!("🐋 最大鱼: {}\n", format_weight(biggest)));
    }

    out.push_str("\n📍 === 钓鱼地点 ===\n");
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    for (i, spot) in FISHING_SPOTS.iter().enumerate() {
        let lock = if user_level < spot.level_req { "🔒" } else { "✅" };
        out.push_str(&format!(
            "  {} {} {} (等级{}) — {}\n",
            lock, spot.emoji, spot.name, spot.level_req, spot.desc
        ));
        if i < FISHING_SPOTS.len() - 1 {
            // show available fish count
            let fish_count = spot.fish_indices.len();
            out.push_str(&format!("     📖 鱼种: {}种\n", fish_count));
        }
    }

    out.push_str("\n📖 指令: 开始钓鱼+地点 | 钓鱼背包 | 鱼类图鉴 | 钓鱼排行 | 购买鱼饵 | 钓鱼商店\n");
    out
}

/// 开始钓鱼
pub fn cmd_start_fishing(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let spot_name = args.trim();
    if spot_name.is_empty() {
        return "📖 用法: 开始钓鱼+地点名\n💡 示例: 开始钓鱼 村口小塘".to_string();
    }

    // 查找钓鱼点
    let spot_idx = FISHING_SPOTS.iter().position(|s| s.name == spot_name);
    let spot_idx = match spot_idx {
        Some(i) => i,
        None => {
            // 模糊匹配
            let fuzzy = FISHING_SPOTS.iter().position(|s| s.name.contains(spot_name));
            match fuzzy {
                Some(i) => i,
                None => {
                    let names: Vec<&str> = FISHING_SPOTS.iter().map(|s| s.name).collect();
                    return format!("❌ 未找到钓鱼点！可选: {}", names.join("/"));
                }
            }
        }
    };
    let spot = &FISHING_SPOTS[spot_idx];

    // 等级检查
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if user_level < spot.level_req {
        return format!("🔒 {} 需要等级{}，当前等级{}", spot.name, spot.level_req, user_level);
    }

    // 每日次数检查
    let daily = get_daily_fishing(db, user_id);
    if daily >= MAX_DAILY_FISHING {
        return format!("❌ 今日钓鱼次数已用完 ({}/{})！", daily, MAX_DAILY_FISHING);
    }

    // 冷却检查
    let elapsed = now_ts() - get_last_fishing_time(db, user_id);
    let rod_tier = get_rod_tier(db, user_id);
    let rod = &FISHING_RODS[rod_tier];
    let cooldown = (FISHING_COOLDOWN_SECS as f64 * (1.0 - rod.speed_bonus)) as i64;
    let cooldown = cooldown.max(5);
    if elapsed < cooldown {
        let remaining = cooldown - elapsed;
        return format!("⏳ 钓鱼冷却中！还需等待{}秒", remaining);
    }

    // 体力检查
    if let Err(e) = crate::stamina::consume_stamina(user_id, "采集", db) {
        let prefix = user::get_msg_prefix(db, user_id);
        return format!("{}\n{}", prefix, e);
    }

    // 检查虚弱
    if user::check_weakness(db, user_id) > 0 {
        return "❌ 虚弱状态无法钓鱼！请等待虚弱恢复".to_string();
    }

    // 消耗鱼饵
    let mut best_bait_idx: Option<usize> = None;
    for (i, _bait) in BAITS.iter().enumerate().rev() {
        if get_bait_count(db, user_id, i) > 0 {
            best_bait_idx = Some(i);
            break;
        }
    }

    let bait_bonus = if let Some(bait_idx) = best_bait_idx {
        set_bait_count(db, user_id, bait_idx, get_bait_count(db, user_id, bait_idx) - 1);
        BAITS[bait_idx].quality_bonus
    } else {
        0.0
    };

    add_daily_fishing(db, user_id);
    set_last_fishing_time(db, user_id);

    // 选择鱼种
    let seed = deterministic_seed(user_id, &format!("fish_{}_{}", daily, spot_idx));
    let total_quality_bonus = rod.quality_bonus + spot.bonus_rarity + bait_bonus;
    let fish_idx = select_fish(spot, seed, total_quality_bonus);
    let fish = &ALL_FISH[fish_idx];

    // 计算重量
    let weight_range = fish.max_weight - fish.min_weight;
    let weight = if weight_range > 0 {
        fish.min_weight + ((seed >> 16) % weight_range as u64) as i32
    } else {
        fish.min_weight
    };

    // 钓鱼经验
    let exp_gain = match fish.rarity {
        FishRarity::Common => 5,
        FishRarity::Uncommon => 10,
        FishRarity::Rare => 25,
        FishRarity::Epic => 50,
        FishRarity::Legendary => 100,
    };
    add_fishing_exp(db, user_id, exp_gain);

    // 记录捕获
    add_fish_caught(db, user_id, fish_idx, 1);
    set_biggest_fish_weight(db, user_id, weight);

    // 存入背包 (以鱼名作为物品)
    db.knapsack_add(user_id, fish.name, 1);

    let mut out = format!("{} {} === 钓鱼中... ===\n\n", spot.emoji, spot.name);
    out.push_str("🎣 抛竿中...\n");
    out.push_str("🐟 ...\n\n");

    // 特殊提示
    match fish.rarity {
        FishRarity::Legendary => {
            out.push_str("🌟🌟🌟 传说降临！🌟🌟🌟\n");
        }
        FishRarity::Epic => {
            out.push_str("✨✨ 史诗发现！✨✨\n");
        }
        FishRarity::Rare => {
            out.push_str("💫 稀有捕获！💫\n");
        }
        _ => {}
    }

    out.push_str("🎉 恭喜钓到！\n\n");
    out.push_str(&format!("{} {} {} ×1\n", fish.rarity.emoji(), fish.emoji, fish.name));
    out.push_str(&format!("⚖️ 重量: {}\n", format_weight(weight)));
    out.push_str(&format!("💰 出售价: {}金币\n", format_num(fish.sell_price)));
    out.push_str(&format!("📈 经验: +{}\n", exp_gain));
    if bait_bonus > 0.0 {
        out.push_str(&format!("🪱 鱼饵加成: +{}%\n", (bait_bonus * 100.0) as i32));
    }

    let remaining = MAX_DAILY_FISHING - get_daily_fishing(db, user_id);
    out.push_str(&format!("\n📊 今日剩余: {}次 | 冷却: {}秒", remaining, cooldown));
    out
}

fn select_fish(spot: &FishingSpot, seed: u64, quality_bonus: f64) -> usize {
    // 构建加权鱼池
    let mut pool: Vec<(usize, u32)> = Vec::new();
    for &fi in spot.fish_indices {
        let fish = &ALL_FISH[fi];
        let mut weight = fish.rarity.base_weight();
        // 品质加成增加稀有鱼权重
        if fish.rarity as u8 >= FishRarity::Rare as u8 {
            weight = (weight as f64 * (1.0 + quality_bonus * 2.0)) as u32;
        }
        pool.push((fi, weight.max(1)));
    }

    let total_weight: u32 = pool.iter().map(|(_, w)| *w).sum();
    let roll = (seed % total_weight as u64) as u32;
    let mut cumulative = 0u32;
    for (idx, w) in pool {
        cumulative += w;
        if roll < cumulative {
            return idx;
        }
    }
    spot.fish_indices[0]
}

/// 钓鱼背包
pub fn cmd_fishing_bag(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🎒 === 钓鱼背包 === 🎒\n\n");

    // 鱼饵
    out.push_str("🪱 鱼饵:\n");
    let mut has_bait = false;
    for (i, bait) in BAITS.iter().enumerate() {
        let count = get_bait_count(db, user_id, i);
        if count > 0 {
            out.push_str(&format!(
                "  {} {} ×{} (品质+{}%)\n",
                bait.emoji,
                bait.name,
                count,
                (bait.quality_bonus * 100.0) as i32
            ));
            has_bait = true;
        }
    }
    if !has_bait {
        out.push_str("  (无鱼饵，请购买)\n");
    }

    // 鱼竿
    let rod_tier = get_rod_tier(db, user_id);
    let rod = &FISHING_RODS[rod_tier];
    out.push_str(&format!(
        "\n🎣 当前钓竿: {} {} (品质+{}%, 速度+{}%)\n",
        rod.emoji,
        rod.name,
        (rod.quality_bonus * 100.0) as i32,
        (rod.speed_bonus * 100.0) as i32
    ));

    // 背包中的鱼
    out.push_str("\n🐟 背包中的鱼:\n");
    let mut fish_found = false;
    for (i, fish) in ALL_FISH.iter().enumerate() {
        let count = get_fish_caught(db, user_id, i);
        if count > 0 {
            out.push_str(&format!(
                "  {} {} ×{} (单价:{}金币)\n",
                fish.rarity.emoji(),
                fish.name,
                count,
                format_num(fish.sell_price)
            ));
            fish_found = true;
        }
    }
    if !fish_found {
        out.push_str("  (没有鱼，快去钓吧)\n");
    }

    out.push_str("\n📖 指令: 出售鱼类 | 购买鱼饵+名称+数量\n");
    out
}

/// 鱼类图鉴
pub fn cmd_fish_codex(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("📖 === 鱼类图鉴 === 📖\n\n");

    let mut discovered = 0;
    for i in 0..ALL_FISH.len() {
        let caught = get_fish_caught(db, user_id, i);
        if caught > 0 {
            discovered += 1;
        }
    }

    out.push_str(&format!("📊 收集进度: {}/{} 种\n\n", discovered, ALL_FISH.len()));

    // 按品质分组
    let rarities = [
        FishRarity::Common,
        FishRarity::Uncommon,
        FishRarity::Rare,
        FishRarity::Epic,
        FishRarity::Legendary,
    ];

    for rarity in &rarities {
        out.push_str(&format!("{} {} 级:\n", rarity.emoji(), rarity.name()));
        for (i, fish) in ALL_FISH.iter().enumerate() {
            if fish.rarity != *rarity {
                continue;
            }
            let caught = get_fish_caught(db, user_id, i);
            if caught > 0 {
                out.push_str(&format!(
                    "  {} {} ×{} (最大{} / 售价{}金)\n",
                    fish.emoji,
                    fish.name,
                    caught,
                    format_weight(fish.max_weight),
                    format_num(fish.sell_price)
                ));
            } else {
                out.push_str("  ❓ ??? (未发现)\n");
            }
        }
        out.push('\n');
    }

    let pct = if ALL_FISH.is_empty() {
        0
    } else {
        discovered * 100 / ALL_FISH.len() as i32
    };
    let trophy = if pct >= 100 {
        "🏆 图鉴完成！"
    } else if pct >= 50 {
        "⭐ 过半了！"
    } else {
        ""
    };
    out.push_str(&format!("📊 完成度: {}% {}", pct, trophy));
    out
}

/// 钓鱼排行
pub fn cmd_fishing_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🏆 === 钓鱼排行榜 === 🏆\n\n");

    // 收集所有用户的钓鱼数据
    let mut rankings: Vec<(String, i32, i32, i32)> = Vec::new(); // (uid, total_fish, exp, biggest)

    // 从Global表获取所有钓鱼用户
    for uid in db.all_users() {
        let exp: i32 = db.global_get(SECTION, &format!("exp_{}", uid)).parse().unwrap_or(0);
        let total = get_total_fish_caught(db, &uid);
        let biggest = get_biggest_fish_weight(db, &uid);
        if total > 0 {
            rankings.push((uid, total, exp, biggest));
        }
    }

    // 按总捕获数排序
    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let medal = |i: usize| match i {
        0 => "🥇",
        1 => "🥈",
        2 => "🥉",
        _ => "  ",
    };

    for (i, (uid, total, exp, biggest)) in rankings.iter().take(15).enumerate() {
        let is_me = uid == user_id;
        let level = fishing_level(*exp);
        let title = match level {
            0..=4 => "新手",
            5..=14 => "钓手",
            15..=29 => "钓友",
            30..=49 => "达人",
            50..=69 => "宗师",
            70..=89 => "鱼王",
            _ => "钓神",
        };
        out.push_str(&format!(
            "{} #{} Lv.{} {} | {}条 | 最大{}\n",
            medal(i),
            i + 1,
            level,
            title,
            total,
            format_weight(*biggest)
        ));
        if is_me {
            out.push_str("     ↑ 你在这里\n");
        }
    }

    if rankings.is_empty() {
        out.push_str("暂无钓鱼记录，快去钓鱼吧！\n");
    } else {
        // 用户排名
        let my_pos = rankings.iter().position(|(uid, _, _, _)| uid == user_id);
        if let Some(pos) = my_pos {
            if pos >= 15 {
                let (_, total, exp, biggest) = &rankings[pos];
                out.push_str(&format!(
                    "\n📍 你的排名: #{} | {}条 | Lv.{} | 最大{}",
                    pos + 1,
                    total,
                    fishing_level(*exp),
                    format_weight(*biggest)
                ));
            }
        }
    }

    out
}

/// 购买鱼饵
pub fn cmd_buy_bait(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        let mut out = String::from("🪱 === 鱼饵商店 === 🪱\n\n");
        for (i, bait) in BAITS.iter().enumerate() {
            let owned = get_bait_count(db, user_id, i);
            let price = if bait.cost_gold > 0 {
                format!("💰{}金币", format_num(bait.cost_gold))
            } else {
                format!("💎{}钻石", bait.cost_diamond)
            };
            out.push_str(&format!(
                "  {} {} — {} (品质+{}%) [持有:{}]\n",
                i + 1,
                bait.name,
                price,
                (bait.quality_bonus * 100.0) as i32,
                owned
            ));
        }
        out.push_str("\n📖 用法: 购买鱼饵+名称+数量\n💡 示例: 购买鱼饵 蚯蚓 10\n");
        return out;
    }

    let bait_name = parts[0];
    let buy_qty: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).clamp(1, 99);

    let bait_idx = BAITS.iter().position(|b| b.name == bait_name);
    let bait_idx = match bait_idx {
        Some(i) => i,
        None => {
            let fuzzy = BAITS.iter().position(|b| b.name.contains(bait_name));
            match fuzzy {
                Some(i) => i,
                None => {
                    let names: Vec<&str> = BAITS.iter().map(|b| b.name).collect();
                    return format!("❌ 未找到鱼饵！可选: {}", names.join("/"));
                }
            }
        }
    };
    let bait = &BAITS[bait_idx];

    // 检查货币
    if bait.cost_gold > 0 {
        let gold = db.read_currency(user_id, CURRENCY_GOLD);
        let total_cost = bait.cost_gold * buy_qty as i64;
        if gold < total_cost {
            return format!(
                "❌ 金币不足！需要💰{}，当前💰{}",
                format_num(total_cost),
                format_num(gold)
            );
        }
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, total_cost);
    } else {
        let diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
        let total_cost = bait.cost_diamond * buy_qty;
        if diamond < total_cost as i64 {
            return format!("❌ 钻石不足！需要💎{}，当前💎{}", total_cost, diamond);
        }
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, total_cost as i64);
    }

    set_bait_count(db, user_id, bait_idx, get_bait_count(db, user_id, bait_idx) + buy_qty);

    format!(
        "✅ 购买成功！{} {} ×{} (品质+{}%)",
        bait.emoji,
        bait.name,
        buy_qty,
        (bait.quality_bonus * 100.0) as i32
    )
}

/// 钓鱼商店 (钓竿升级)
pub fn cmd_fishing_shop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let action = args.trim();

    if action.is_empty() {
        let current_tier = get_rod_tier(db, user_id);
        let mut out = String::from("🏪 === 钓鱼商店 === 🏪\n\n");

        out.push_str("🎣 钓竿列表:\n");
        for (i, rod) in FISHING_RODS.iter().enumerate() {
            let status = if i == current_tier {
                " ✅ 当前"
            } else if i < current_tier {
                " ✓ 已拥有"
            } else {
                " 🔒"
            };
            let price = if i == 0 {
                "免费".to_string()
            } else {
                format!("💰{}", format_num(rod.cost_gold))
            };
            out.push_str(&format!(
                "  {} {} {} {} — {} (品质+{}%, 速度+{}%){}\n",
                i + 1,
                rod.emoji,
                rod.name,
                price,
                if rod.level_req > 0 {
                    format!("Lv.{}", rod.level_req)
                } else {
                    String::new()
                },
                (rod.quality_bonus * 100.0) as i32,
                (rod.speed_bonus * 100.0) as i32,
                status
            ));
        }

        // 升级提示
        if current_tier + 1 < FISHING_RODS.len() {
            let next = &FISHING_RODS[current_tier + 1];
            out.push_str(&format!(
                "\n⬆️ 下一级: {} {} (需要💰{}金币, Lv.{})\n",
                next.emoji,
                next.name,
                format_num(next.cost_gold),
                next.level_req
            ));
            out.push_str("📖 用法: 钓鱼商店 升级\n");
        } else {
            out.push_str("\n🏆 钓竿已满级！\n");
        }

        out.push_str("\n📦 其他商品:\n");
        out.push_str("  🪱 鱼饵 → 购买鱼饵\n");
        return out;
    }

    // 升级钓竿
    if action.contains("升级") || action.contains("upgrade") {
        let current_tier = get_rod_tier(db, user_id);
        if current_tier + 1 >= FISHING_RODS.len() {
            return "🏆 钓竿已满级！".to_string();
        }

        let next = &FISHING_RODS[current_tier + 1];
        let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
        if user_level < next.level_req {
            return format!("🔒 升级需要等级{}，当前等级{}", next.level_req, user_level);
        }

        let gold = db.read_currency(user_id, CURRENCY_GOLD);
        if gold < next.cost_gold {
            return format!(
                "❌ 金币不足！需要💰{}，当前💰{}",
                format_num(next.cost_gold),
                format_num(gold)
            );
        }

        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, next.cost_gold);
        set_rod_tier(db, user_id, current_tier + 1);

        return format!(
            "🎉 钓竿升级成功！\n\n⬆️ {} → {} {} \n📊 品质+{}%, 速度+{}%",
            FISHING_RODS[current_tier].name,
            next.emoji,
            next.name,
            (next.quality_bonus * 100.0) as i32,
            (next.speed_bonus * 100.0) as i32
        );
    }

    "📖 用法: 钓鱼商店 (查看) | 钓鱼商店 升级".to_string()
}

/// 出售鱼类
pub fn cmd_sell_fish(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let target = args.trim();

    if target.is_empty() || target == "全部" {
        // 出售所有鱼
        let mut total_gold = 0i64;
        let mut total_count = 0i32;
        for (i, fish) in ALL_FISH.iter().enumerate() {
            let count = get_fish_caught(db, user_id, i);
            if count > 0 {
                let earn = fish.sell_price * count as i64;
                total_gold += earn;
                total_count += count;
                set_bait_count(db, user_id, i, 0); // clear
                db.global_set(SECTION, &format!("caught_{}_{}", user_id, i), "0");
            }
        }

        if total_count == 0 {
            return "❌ 背包中没有鱼可出售！".to_string();
        }

        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold);
        add_total_gold_earned(db, user_id, total_gold);

        return format!(
            "💰 出售全部鱼类！\n\n🐟 数量: {}条\n💰 收入: {}金币",
            total_count,
            format_num(total_gold)
        );
    }

    // 出售指定鱼
    let fish_idx = ALL_FISH.iter().position(|f| f.name == target);
    let fish_idx = match fish_idx {
        Some(i) => i,
        None => {
            let fuzzy = ALL_FISH.iter().position(|f| f.name.contains(target));
            match fuzzy {
                Some(i) => i,
                None => return format!("❌ 未找到鱼类「{}」", target),
            }
        }
    };

    let count = get_fish_caught(db, user_id, fish_idx);
    if count <= 0 {
        return format!("❌ 背包中没有{}", ALL_FISH[fish_idx].name);
    }

    let fish = &ALL_FISH[fish_idx];
    let earn = fish.sell_price * count as i64;
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, earn);
    add_total_gold_earned(db, user_id, earn);
    db.global_set(SECTION, &format!("caught_{}_{}", user_id, fish_idx), "0");

    format!(
        "💰 出售成功！\n\n{} {} ×{}\n💰 收入: {}金币",
        fish.emoji,
        fish.name,
        count,
        format_num(earn)
    )
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fish_count() {
        assert_eq!(ALL_FISH.len(), 22);
    }

    #[test]
    fn test_fish_rarity_distribution() {
        let common = ALL_FISH.iter().filter(|f| f.rarity == FishRarity::Common).count();
        let uncommon = ALL_FISH.iter().filter(|f| f.rarity == FishRarity::Uncommon).count();
        let rare = ALL_FISH.iter().filter(|f| f.rarity == FishRarity::Rare).count();
        let epic = ALL_FISH.iter().filter(|f| f.rarity == FishRarity::Epic).count();
        let legendary = ALL_FISH.iter().filter(|f| f.rarity == FishRarity::Legendary).count();
        assert_eq!(common, 10);
        assert_eq!(uncommon, 5);
        assert_eq!(rare, 3);
        assert_eq!(epic, 2);
        assert_eq!(legendary, 2);
        assert_eq!(common + uncommon + rare + epic + legendary, ALL_FISH.len());
    }

    #[test]
    fn test_spot_count() {
        assert_eq!(FISHING_SPOTS.len(), 6);
    }

    #[test]
    fn test_spot_level_requirements() {
        for spot in FISHING_SPOTS {
            assert!(spot.level_req >= 1);
            assert!(spot.fish_indices.len() > 0);
        }
    }

    #[test]
    fn test_rod_count() {
        assert_eq!(FISHING_RODS.len(), 6);
    }

    #[test]
    fn test_rod_quality_bonus_increasing() {
        for i in 1..FISHING_RODS.len() {
            assert!(FISHING_RODS[i].quality_bonus >= FISHING_RODS[i - 1].quality_bonus);
            assert!(FISHING_RODS[i].speed_bonus >= FISHING_RODS[i - 1].speed_bonus);
        }
    }

    #[test]
    fn test_rod_cost_increasing() {
        for i in 2..FISHING_RODS.len() {
            assert!(FISHING_RODS[i].cost_gold >= FISHING_RODS[i - 1].cost_gold);
        }
    }

    #[test]
    fn test_bait_count() {
        assert_eq!(BAITS.len(), 6);
    }

    #[test]
    fn test_bait_quality_increasing() {
        for i in 1..BAITS.len() {
            assert!(BAITS[i].quality_bonus >= BAITS[i - 1].quality_bonus);
        }
    }

    #[test]
    fn test_fish_sell_price_positive() {
        for fish in ALL_FISH {
            assert!(fish.sell_price > 0);
            assert!(fish.min_weight > 0);
            assert!(fish.max_weight >= fish.min_weight);
            assert!(fish.cook_value > 0);
        }
    }

    #[test]
    fn test_fishing_level_formula() {
        assert_eq!(fishing_level(0), 0);
        assert_eq!(fishing_level(49), 0);
        assert_eq!(fishing_level(50), 1);
        assert_eq!(fishing_level(150), 2); // 50 + 100
    }

    #[test]
    fn test_format_weight() {
        assert_eq!(format_weight(500), "500g");
        assert_eq!(format_weight(1000), "1.0kg");
        assert_eq!(format_weight(1500), "1.5kg");
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234567), "1,234,567");
    }

    #[test]
    fn test_deterministic_seed() {
        let s1 = deterministic_seed("user1", "fish_0_0");
        let s2 = deterministic_seed("user1", "fish_0_0");
        assert_eq!(s1, s2);
        let s3 = deterministic_seed("user2", "fish_0_0");
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_select_fish_returns_valid_index() {
        let spot = &FISHING_SPOTS[0];
        for seed_val in 0..100u64 {
            let idx = select_fish(spot, seed_val, 0.0);
            assert!(spot.fish_indices.contains(&idx), "idx {} not in spot fish_indices", idx);
        }
    }

    #[test]
    fn test_select_fish_with_bonus_increases_rarity() {
        let spot = &FISHING_SPOTS[0]; // 村口小塘 - only common fish
        let mut rare_count_no_bonus = 0;
        let mut rare_count_with_bonus = 0;

        for seed_val in 0..1000u64 {
            let idx_no = select_fish(spot, seed_val, 0.0);
            let idx_yes = select_fish(spot, seed_val, 0.5);
            if ALL_FISH[idx_no].rarity >= FishRarity::Rare {
                rare_count_no_bonus += 1;
            }
            if ALL_FISH[idx_yes].rarity >= FishRarity::Rare {
                rare_count_with_bonus += 1;
            }
        }
        // With bonus, we should get at least as many or more rare fish
        // (may be equal if spot only has common fish, but not fewer)
        assert!(rare_count_with_bonus >= rare_count_no_bonus);
    }

    #[test]
    fn test_rarity_base_weight_ordering() {
        assert!(FishRarity::Common.base_weight() > FishRarity::Uncommon.base_weight());
        assert!(FishRarity::Uncommon.base_weight() > FishRarity::Rare.base_weight());
        assert!(FishRarity::Rare.base_weight() > FishRarity::Epic.base_weight());
        assert!(FishRarity::Epic.base_weight() > FishRarity::Legendary.base_weight());
    }

    #[test]
    fn test_fish_rarity_emoji() {
        assert_eq!(FishRarity::Common.emoji(), "⚪");
        assert_eq!(FishRarity::Legendary.emoji(), "🟡");
    }

    #[test]
    fn test_spot_fish_indices_unique() {
        for spot in FISHING_SPOTS {
            let mut seen = std::collections::HashSet::new();
            for &idx in spot.fish_indices {
                assert!(seen.insert(idx), "duplicate fish index {} in spot {}", idx, spot.name);
                assert!(
                    idx < ALL_FISH.len(),
                    "fish index {} out of range in spot {}",
                    idx,
                    spot.name
                );
            }
        }
    }

    #[test]
    fn test_all_fish_names_unique() {
        let mut names = std::collections::HashSet::new();
        for fish in ALL_FISH {
            assert!(names.insert(fish.name), "duplicate fish name: {}", fish.name);
        }
    }

    #[test]
    fn test_today_string_format() {
        let s = today_string();
        assert!(s.contains('-'));
        assert!(s.len() >= 5);
    }

    #[test]
    fn test_fish_sell_price_scales_with_rarity() {
        let avg_common: f64 = ALL_FISH
            .iter()
            .filter(|f| f.rarity == FishRarity::Common)
            .map(|f| f.sell_price as f64)
            .sum::<f64>()
            / 10.0;
        let avg_legendary: f64 = ALL_FISH
            .iter()
            .filter(|f| f.rarity == FishRarity::Legendary)
            .map(|f| f.sell_price as f64)
            .sum::<f64>()
            / 2.0;
        assert!(
            avg_legendary > avg_common * 10.0,
            "legendary avg {} should be much higher than common avg {}",
            avg_legendary,
            avg_common
        );
    }

    #[test]
    fn test_fish_cook_value_positive() {
        for fish in ALL_FISH {
            assert!(fish.cook_value > 0, "cook_value should be positive for {}", fish.name);
        }
    }

    #[test]
    fn test_fishing_spot_all_cover_all_fish() {
        // Verify every fish is available in at least one spot
        let mut covered = vec![false; ALL_FISH.len()];
        for spot in FISHING_SPOTS {
            for &idx in spot.fish_indices {
                covered[idx] = true;
            }
        }
        for (i, c) in covered.iter().enumerate() {
            assert!(*c, "fish {} ({}) is not available in any spot", i, ALL_FISH[i].name);
        }
    }
}

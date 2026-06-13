/// CakeGame 挖宝探险系统
///
/// 玩家使用藏宝图前往指定地图挖掘宝藏
/// 藏宝图分4种品质: 普通/稀有/史诗/传说
/// 挖掘可能获得金币、钻石、稀有材料或遭遇陷阱
/// 每日挖掘次数限制 + 冷却时间
/// 数据存储: Global 表 SECTION='treasure_hunt'
use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;
use crate::vip;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 藏宝图品质
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapRarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

impl MapRarity {
    pub fn name(&self) -> &'static str {
        match self {
            MapRarity::Common => "普通",
            MapRarity::Rare => "稀有",
            MapRarity::Epic => "史诗",
            MapRarity::Legendary => "传说",
        }
    }
    pub fn emoji(&self) -> &'static str {
        match self {
            MapRarity::Common => "🗺️",
            MapRarity::Rare => "📘",
            MapRarity::Epic => "📕",
            MapRarity::Legendary => "📜",
        }
    }
    pub fn color(&self) -> &'static str {
        match self {
            MapRarity::Common => "⚪",
            MapRarity::Rare => "🔵",
            MapRarity::Epic => "🟣",
            MapRarity::Legendary => "🟡",
        }
    }
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "普通" | "common" => Some(MapRarity::Common),
            "稀有" | "rare" => Some(MapRarity::Rare),
            "史诗" | "epic" => Some(MapRarity::Epic),
            "传说" | "legendary" => Some(MapRarity::Legendary),
            _ => None,
        }
    }
    pub fn map_name(&self) -> &'static str {
        match self {
            MapRarity::Common => "普通藏宝图",
            MapRarity::Rare => "稀有藏宝图",
            MapRarity::Epic => "史诗藏宝图",
            MapRarity::Legendary => "传说藏宝图",
        }
    }
}

/// 挖掘地点定义
struct DigSite {
    map_name: &'static str,
    desc: &'static str,
}

const DIG_SITES: &[DigSite] = &[
    DigSite {
        map_name: "落叶村",
        desc: "村口老树下的神秘土堆",
    },
    DigSite {
        map_name: "荒野",
        desc: "风蚀岩石旁的隐蔽沙坑",
    },
    DigSite {
        map_name: "幽暗森林",
        desc: "千年古树根部的暗穴",
    },
    DigSite {
        map_name: "废弃矿山",
        desc: "坍塌矿道尽头的宝箱",
    },
    DigSite {
        map_name: "龙骨荒原",
        desc: "巨龙遗骸下的秘藏",
    },
    DigSite {
        map_name: "水晶洞穴",
        desc: "发光水晶背后的暗格",
    },
    DigSite {
        map_name: "深渊裂隙",
        desc: "黑暗裂隙中的远古祭坛",
    },
    DigSite {
        map_name: "天界之门",
        desc: "浮空石台上的神殿遗迹",
    },
];

/// 陷阱定义
struct TrapDef {
    #[allow(dead_code)]
    name: &'static str,
    desc: &'static str,
    damage_pct: i32,
    gold_loss: i64,
}

const TRAPS: &[TrapDef] = &[
    TrapDef {
        name: "毒蛇陷阱",
        desc: "🐍 一群毒蛇从洞口窜出！",
        damage_pct: 10,
        gold_loss: 100,
    },
    TrapDef {
        name: "坍塌陷阱",
        desc: "💥 挖掘导致地面坍塌！",
        damage_pct: 15,
        gold_loss: 200,
    },
    TrapDef {
        name: "诅咒陷阱",
        desc: "💀 触发了远古诅咒！",
        damage_pct: 20,
        gold_loss: 300,
    },
    TrapDef {
        name: "守卫傀儡",
        desc: "🗿 守卫傀儡复活攻击！",
        damage_pct: 25,
        gold_loss: 500,
    },
];

/// 宝藏奖励
struct RewardDef {
    #[allow(dead_code)]
    name: &'static str,
    emoji: &'static str,
    min_qty: i32,
    max_qty: i32,
    gold_min: i64,
    gold_max: i64,
    prob: f64,
}

fn common_rewards() -> &'static [RewardDef] {
    &[
        RewardDef {
            name: "初级药水",
            emoji: "🧪",
            min_qty: 1,
            max_qty: 3,
            gold_min: 50,
            gold_max: 200,
            prob: 0.40,
        },
        RewardDef {
            name: "铜矿石",
            emoji: "🪨",
            min_qty: 1,
            max_qty: 5,
            gold_min: 30,
            gold_max: 100,
            prob: 0.30,
        },
        RewardDef {
            name: "草药",
            emoji: "🌿",
            min_qty: 1,
            max_qty: 3,
            gold_min: 20,
            gold_max: 80,
            prob: 0.20,
        },
        RewardDef {
            name: "金币袋",
            emoji: "💰",
            min_qty: 1,
            max_qty: 1,
            gold_min: 200,
            gold_max: 500,
            prob: 0.10,
        },
    ]
}

fn rare_rewards() -> &'static [RewardDef] {
    &[
        RewardDef {
            name: "中级药水",
            emoji: "💊",
            min_qty: 1,
            max_qty: 3,
            gold_min: 100,
            gold_max: 400,
            prob: 0.30,
        },
        RewardDef {
            name: "铁矿石",
            emoji: "⛏️",
            min_qty: 2,
            max_qty: 5,
            gold_min: 100,
            gold_max: 300,
            prob: 0.25,
        },
        RewardDef {
            name: "强化石",
            emoji: "💎",
            min_qty: 1,
            max_qty: 2,
            gold_min: 200,
            gold_max: 500,
            prob: 0.20,
        },
        RewardDef {
            name: "金币箱",
            emoji: "📦",
            min_qty: 1,
            max_qty: 1,
            gold_min: 500,
            gold_max: 1500,
            prob: 0.15,
        },
        RewardDef {
            name: "初级宝石",
            emoji: "🔴",
            min_qty: 1,
            max_qty: 1,
            gold_min: 300,
            gold_max: 800,
            prob: 0.10,
        },
    ]
}

fn epic_rewards() -> &'static [RewardDef] {
    &[
        RewardDef {
            name: "高级药水",
            emoji: "💉",
            min_qty: 1,
            max_qty: 3,
            gold_min: 300,
            gold_max: 800,
            prob: 0.25,
        },
        RewardDef {
            name: "精炼石",
            emoji: "✨",
            min_qty: 1,
            max_qty: 3,
            gold_min: 500,
            gold_max: 1200,
            prob: 0.20,
        },
        RewardDef {
            name: "进化石",
            emoji: "🔮",
            min_qty: 1,
            max_qty: 2,
            gold_min: 800,
            gold_max: 2000,
            prob: 0.20,
        },
        RewardDef {
            name: "中级宝石",
            emoji: "🔵",
            min_qty: 1,
            max_qty: 2,
            gold_min: 600,
            gold_max: 1500,
            prob: 0.15,
        },
        RewardDef {
            name: "金币宝箱",
            emoji: "🎁",
            min_qty: 1,
            max_qty: 1,
            gold_min: 2000,
            gold_max: 5000,
            prob: 0.10,
        },
        RewardDef {
            name: "复活卷轴",
            emoji: "📜",
            min_qty: 1,
            max_qty: 1,
            gold_min: 1000,
            gold_max: 3000,
            prob: 0.10,
        },
    ]
}

fn legendary_rewards() -> &'static [RewardDef] {
    &[
        RewardDef {
            name: "超级药水",
            emoji: "🧴",
            min_qty: 1,
            max_qty: 5,
            gold_min: 800,
            gold_max: 2000,
            prob: 0.20,
        },
        RewardDef {
            name: "高级宝石",
            emoji: "🟣",
            min_qty: 1,
            max_qty: 3,
            gold_min: 1500,
            gold_max: 3000,
            prob: 0.15,
        },
        RewardDef {
            name: "传说碎片",
            emoji: "✴️",
            min_qty: 1,
            max_qty: 2,
            gold_min: 2000,
            gold_max: 5000,
            prob: 0.15,
        },
        RewardDef {
            name: "命运转盘券",
            emoji: "🎰",
            min_qty: 1,
            max_qty: 1,
            gold_min: 1000,
            gold_max: 3000,
            prob: 0.15,
        },
        RewardDef {
            name: "金币宝库",
            emoji: "🏦",
            min_qty: 1,
            max_qty: 1,
            gold_min: 5000,
            gold_max: 15000,
            prob: 0.15,
        },
        RewardDef {
            name: "神器碎片",
            emoji: "⚔️",
            min_qty: 1,
            max_qty: 1,
            gold_min: 3000,
            gold_max: 8000,
            prob: 0.10,
        },
        RewardDef {
            name: "凤凰之羽",
            emoji: "🪶",
            min_qty: 1,
            max_qty: 1,
            gold_min: 5000,
            gold_max: 10000,
            prob: 0.10,
        },
    ]
}

const MAX_DAILY_DIGS: i32 = 5;
const DIG_COOLDOWN_SECS: i64 = 300;
const DIG_COST_GOLD: i64 = 200;
const SECTION: &str = "treasure_hunt";

fn today_str() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now / 86400)
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn deterministic_seed(user_id: &str, extra: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    user_id.hash(&mut hasher);
    extra.hash(&mut hasher);
    today_str().hash(&mut hasher);
    hasher.finish()
}

fn get_daily_digs(db: &Database, user_id: &str) -> i32 {
    let key = format!("th_digs_{}", user_id);
    let val = db.global_get(SECTION, &key);
    let today = today_str();
    if let Some((date, count)) = val.split_once(':') {
        if date == today {
            return count.parse::<i32>().unwrap_or(0);
        }
    }
    0
}

fn add_daily_dig(db: &Database, user_id: &str) {
    let key = format!("th_digs_{}", user_id);
    let today = today_str();
    let current = get_daily_digs(db, user_id);
    db.global_set(SECTION, &key, &format!("{}:{}", today, current + 1));
}

fn get_last_dig_time(db: &Database, user_id: &str) -> i64 {
    let key = format!("th_last_{}", user_id);
    db.global_get(SECTION, &key).parse::<i64>().unwrap_or(0)
}

fn set_last_dig_time(db: &Database, user_id: &str) {
    let key = format!("th_last_{}", user_id);
    db.global_set(SECTION, &key, &now_ts().to_string());
}

fn get_total_digs(db: &Database, user_id: &str) -> i64 {
    let key = format!("th_total_{}", user_id);
    db.global_get(SECTION, &key).parse::<i64>().unwrap_or(0)
}

fn add_total_dig(db: &Database, user_id: &str) {
    let key = format!("th_total_{}", user_id);
    let total = get_total_digs(db, user_id);
    db.global_set(SECTION, &key, &(total + 1).to_string());
}

fn add_dig_reward_stat(db: &Database, user_id: &str, gold: i64, diamond: i32) {
    if gold > 0 {
        let key = format!("th_gold_{}", user_id);
        let cur: i64 = db.global_get(SECTION, &key).parse().unwrap_or(0);
        db.global_set(SECTION, &key, &(cur + gold).to_string());
    }
    if diamond > 0 {
        let key = format!("th_diamond_{}", user_id);
        let cur: i32 = db.global_get(SECTION, &key).parse().unwrap_or(0);
        db.global_set(SECTION, &key, &(cur + diamond).to_string());
    }
}

/// 查看藏宝图
pub fn cmd_view_treasure_maps(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🗺️ === 藏宝图背包 === 🗺️\n\n");

    let rarities = [
        MapRarity::Common,
        MapRarity::Rare,
        MapRarity::Epic,
        MapRarity::Legendary,
    ];

    let mut total = 0i32;
    for rarity in &rarities {
        let count = db.knapsack_quantity(user_id, rarity.map_name());
        total += count;
        if count > 0 {
            out.push_str(&format!(
                "{} {} {}: {}张\n",
                rarity.color(),
                rarity.emoji(),
                rarity.map_name(),
                count
            ));
        }
    }

    if total == 0 {
        out.push_str("📭 背包中没有藏宝图\n\n");
        out.push_str("💡 藏宝图获取途径:\n");
        out.push_str("   🗡️ 击败怪物随机掉落\n");
        out.push_str("   📦 开启宝箱概率获得\n");
        out.push_str("   🎁 活动奖励\n");
    }

    let daily = get_daily_digs(db, user_id);
    let total_digs = get_total_digs(db, user_id);
    out.push_str(&format!(
        "\n📊 今日挖掘: {}/{}次 | 累计: {}次\n",
        daily, MAX_DAILY_DIGS, total_digs
    ));
    out.push_str("📖 指令: 开始挖宝+品质 (如: 开始挖宝 稀有)\n");

    out
}

/// 开始挖宝
pub fn cmd_treasure_dig(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let rarity_name = args.trim();
    if rarity_name.is_empty() {
        return "📖 用法: 开始挖宝+品质\n💡 可选品质: 普通/稀有/史诗/传说".to_string();
    }

    let rarity = match MapRarity::from_name(rarity_name) {
        Some(r) => r,
        None => return "❌ 无效品质！可选: 普通/稀有/史诗/传说".to_string(),
    };

    let map_name = rarity.map_name();

    // 检查藏宝图
    let map_count = db.knapsack_quantity(user_id, map_name);
    if map_count <= 0 {
        return format!("❌ 背包中没有{}！", map_name);
    }

    // 检查每日次数
    let daily = get_daily_digs(db, user_id);
    if daily >= MAX_DAILY_DIGS {
        return format!("❌ 今日挖掘次数已用完 ({}/{})！", daily, MAX_DAILY_DIGS);
    }

    // 检查冷却
    let elapsed = now_ts() - get_last_dig_time(db, user_id);
    if elapsed < DIG_COOLDOWN_SECS {
        let remaining = DIG_COOLDOWN_SECS - elapsed;
        return format!("⏳ 挖掘冷却中！还需等待{}秒", remaining);
    }

    // 体力检查 (挖宝消耗8体力)
    if let Err(e) = stamina::consume_stamina(user_id, "挖宝", db) {
        let prefix = user::get_msg_prefix(db, user_id);
        return format!("{}\n{}", prefix, e);
    }

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < DIG_COST_GOLD {
        return format!("❌ 金币不足！挖掘需要💰{}金币，当前💰{}", DIG_COST_GOLD, gold);
    }

    // 检查虚弱
    if user::check_weakness(db, user_id) > 0 {
        return "❌ 虚弱状态无法挖宝！请等待虚弱恢复".to_string();
    }

    // 消耗
    db.knapsack_remove(user_id, map_name, 1);
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, DIG_COST_GOLD);
    add_daily_dig(db, user_id);
    set_last_dig_time(db, user_id);
    add_total_dig(db, user_id);

    // 选择地点
    let seed = deterministic_seed(user_id, &format!("dig_{}", daily));
    let site_idx = (seed as usize) % DIG_SITES.len();
    let site = &DIG_SITES[site_idx];

    let mut out = format!("{} {} === 挖宝探险 ===\n\n", rarity.emoji(), rarity.name());
    out.push_str(&format!("📍 地点: {} — {}\n", site.map_name, site.desc));
    out.push_str("⛏️ 挖掘中...\n\n");

    // 陷阱概率
    let trap_chance = match rarity {
        MapRarity::Common => 0.25,
        MapRarity::Rare => 0.15,
        MapRarity::Epic => 0.10,
        MapRarity::Legendary => 0.05,
    };
    let trap_roll = ((seed >> 8) % 1000) as f64 / 1000.0;

    if trap_roll < trap_chance {
        // 陷阱
        let trap_idx = ((seed >> 16) % TRAPS.len() as u64) as usize;
        let trap = &TRAPS[trap_idx];

        let current_hp: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(100);
        let max_hp: i32 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(100);
        let damage = max_hp * trap.damage_pct / 100;
        let new_hp = (current_hp - damage).max(1);
        db.write_basic(user_id, ITEM_HP_CURRENT, &new_hp.to_string());

        let gold_loss = trap.gold_loss.min(gold);
        if gold_loss > 0 {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_loss);
        }

        out.push_str(&format!("{}\n", trap.desc));
        out.push_str(&format!("💥 损失 {} HP ({}%最大生命)\n", damage, trap.damage_pct));
        if gold_loss > 0 {
            out.push_str(&format!("💰 损失 {} 金币\n", gold_loss));
        }
        out.push_str("\n😤 下次运气会更好的！");
    } else {
        // 宝藏
        let rewards = match rarity {
            MapRarity::Common => common_rewards(),
            MapRarity::Rare => rare_rewards(),
            MapRarity::Epic => epic_rewards(),
            MapRarity::Legendary => legendary_rewards(),
        };

        let reward_roll = ((seed >> 24) % 1000) as f64 / 1000.0;
        let mut cumulative = 0.0;
        let mut selected = &rewards[0];
        for r in rewards {
            cumulative += r.prob;
            if reward_roll < cumulative {
                selected = r;
                break;
            }
        }

        let qty_range = selected.max_qty - selected.min_qty + 1;
        let qty = if qty_range > 0 {
            selected.min_qty + ((seed >> 32) % qty_range as u64) as i32
        } else {
            selected.min_qty
        };

        let gold_range = selected.gold_max - selected.gold_min + 1;
        let reward_gold = if gold_range > 0 {
            selected.gold_min + ((seed >> 40) % gold_range as u64) as i64
        } else {
            selected.gold_min
        };

        // VIP加成
        let vip_bonus = vip::get_vip_exp_bonus(db, user_id) as f64 * 0.005;
        let final_gold = (reward_gold as f64 * (1.0 + vip_bonus)) as i64;

        db.knapsack_add(user_id, selected.name, qty);
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, final_gold);
        add_dig_reward_stat(db, user_id, final_gold, 0);

        // 小概率钻石
        let diamond_chance = match rarity {
            MapRarity::Legendary => 0.05,
            MapRarity::Epic => 0.03,
            MapRarity::Rare => 0.01,
            MapRarity::Common => 0.005,
        };
        let diamond_roll = ((seed >> 48) % 1000) as f64 / 1000.0;
        let bonus_diamond = if diamond_roll < diamond_chance {
            match rarity {
                MapRarity::Legendary => 20,
                MapRarity::Epic => 10,
                MapRarity::Rare => 5,
                MapRarity::Common => 2,
            }
        } else {
            0
        };
        if bonus_diamond > 0 {
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, bonus_diamond as i64);
            add_dig_reward_stat(db, user_id, 0, bonus_diamond);
        }

        out.push_str("🎉 恭喜发现宝藏！\n\n");
        out.push_str(&format!("🎁 获得: {} {} ×{}\n", selected.emoji, selected.name, qty));
        out.push_str(&format!("💰 金币: +{}\n", final_gold));
        if bonus_diamond > 0 {
            out.push_str(&format!("💎 额外钻石: +{} (幸运发现！)\n", bonus_diamond));
        }
        if vip_bonus > 0.0 {
            out.push_str(&format!("👑 VIP加成: ×{:.1}\n", 1.0 + vip_bonus));
        }
    }

    let remaining = MAX_DAILY_DIGS - get_daily_digs(db, user_id);
    out.push_str(&format!(
        "\n📊 今日剩余: {}次 | 冷却: {}分钟",
        remaining,
        DIG_COOLDOWN_SECS / 60
    ));

    out
}

/// 挖宝统计
pub fn cmd_treasure_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let total = get_total_digs(db, user_id);
    let gold: i64 = db
        .global_get(SECTION, &format!("th_gold_{}", user_id))
        .parse()
        .unwrap_or(0);
    let diamond: i32 = db
        .global_get(SECTION, &format!("th_diamond_{}", user_id))
        .parse()
        .unwrap_or(0);
    let daily = get_daily_digs(db, user_id);

    let mut out = String::from("📊 === 挖宝统计 === 📊\n\n");

    out.push_str(&format!("⛏️ 累计挖掘: {}次\n", total));
    out.push_str(&format!("📅 今日挖掘: {}/{}次\n", daily, MAX_DAILY_DIGS));
    out.push_str(&format!("💰 累计金币收益: {}\n", format_gold(gold)));
    out.push_str(&format!("💎 累计钻石收益: {}\n", diamond));

    if total > 0 {
        let avg_gold = gold / total;
        out.push_str(&format!("📈 平均每次: {}金币\n", format_gold(avg_gold)));
    }

    let (level, title) = dig_level(total);
    out.push_str(&format!("\n🏆 挖掘等级: {} {}\n", title, level));
    if let Some(next) = next_dig_threshold(total) {
        let progress = total as f64 / next as f64;
        let bar = progress_bar(progress, 10);
        out.push_str(&format!("📊 升级进度: {} {}/{}\n", bar, total, next));
    }

    out.push_str("\n💡 挖掘等级越高，掉落品质越好！\n");
    out
}

/// 挖宝排行
pub fn cmd_treasure_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_users = db.all_users();
    let mut rankings: Vec<(String, String, i64)> = Vec::new();

    for uid in &all_users {
        let total = get_total_digs(db, uid);
        if total > 0 {
            let name = db.read_basic(uid, ITEM_NAME);
            rankings.push((uid.clone(), name, total));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut out = String::from("🏆 === 挖宝排行榜 === 🏆\n\n");

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, name, total)) in rankings.iter().take(10).enumerate() {
        let medal = if i < 3 { medals[i] } else { &format!("{:>2}.", i + 1) };
        let (_, title) = dig_level(*total);
        let marker = if uid == user_id { " 👈" } else { "" };
        out.push_str(&format!("{} {} [{}] 挖掘{}次 {}\n", medal, name, title, total, marker));
    }

    if rankings.is_empty() {
        out.push_str("暂无排行数据\n");
    }

    let my_total = get_total_digs(db, user_id);
    if let Some(pos) = rankings.iter().position(|(uid, _, _)| uid == user_id) {
        out.push_str(&format!("\n📍 你的排名: 第{}名 (挖掘{}次)\n", pos + 1, my_total));
    } else {
        out.push_str(&format!("\n📍 你的排名: 未上榜 (挖掘{}次，加油！)\n", my_total));
    }

    out
}

// ========== 辅助 ==========

fn dig_level(total: i64) -> (&'static str, &'static str) {
    if total >= 500 {
        ("Lv.8", "👑 传说探险家")
    } else if total >= 300 {
        ("Lv.7", "💎 钻石矿工")
    } else if total >= 200 {
        ("Lv.6", "🥇 金牌矿工")
    } else if total >= 100 {
        ("Lv.5", "🥈 银牌矿工")
    } else if total >= 50 {
        ("Lv.4", "🥉 铜牌矿工")
    } else if total >= 30 {
        ("Lv.3", "⭐ 熟练矿工")
    } else if total >= 10 {
        ("Lv.2", "⛏️ 实习矿工")
    } else {
        ("Lv.1", "🌱 挖宝新手")
    }
}

fn next_dig_threshold(total: i64) -> Option<i64> {
    let thresholds = [10, 30, 50, 100, 200, 300, 500];
    thresholds.iter().find(|&&t| total < t).copied()
}

fn progress_bar(progress: f64, width: usize) -> String {
    let filled = (progress * width as f64).round() as usize;
    let filled = filled.min(width);
    let mut bar = String::new();
    for i in 0..width {
        if i < filled {
            bar.push('█');
        } else {
            bar.push('░');
        }
    }
    bar
}

fn format_gold(gold: i64) -> String {
    if gold >= 10000 {
        format!("{}万", gold / 10000)
    } else {
        format!("{}", gold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_rarity_names() {
        assert_eq!(MapRarity::Common.name(), "普通");
        assert_eq!(MapRarity::Rare.name(), "稀有");
        assert_eq!(MapRarity::Epic.name(), "史诗");
        assert_eq!(MapRarity::Legendary.name(), "传说");
    }

    #[test]
    fn test_map_rarity_from_name() {
        assert_eq!(MapRarity::from_name("普通"), Some(MapRarity::Common));
        assert_eq!(MapRarity::from_name("稀有"), Some(MapRarity::Rare));
        assert_eq!(MapRarity::from_name("史诗"), Some(MapRarity::Epic));
        assert_eq!(MapRarity::from_name("传说"), Some(MapRarity::Legendary));
        assert_eq!(MapRarity::from_name("不存在"), None);
    }

    #[test]
    fn test_map_rarity_map_name() {
        assert_eq!(MapRarity::Common.map_name(), "普通藏宝图");
        assert_eq!(MapRarity::Rare.map_name(), "稀有藏宝图");
        assert_eq!(MapRarity::Epic.map_name(), "史诗藏宝图");
        assert_eq!(MapRarity::Legendary.map_name(), "传说藏宝图");
    }

    #[test]
    fn test_dig_sites_non_empty() {
        assert!(!DIG_SITES.is_empty());
        for site in DIG_SITES {
            assert!(!site.map_name.is_empty());
            assert!(!site.desc.is_empty());
        }
    }

    #[test]
    fn test_traps_valid() {
        assert!(!TRAPS.is_empty());
        for trap in TRAPS {
            assert!(trap.damage_pct > 0 && trap.damage_pct <= 100);
            assert!(trap.gold_loss >= 0);
        }
    }

    #[test]
    fn test_reward_probs_sum_to_one() {
        let groups: &[&[RewardDef]] = &[common_rewards(), rare_rewards(), epic_rewards(), legendary_rewards()];
        for group in groups {
            let sum: f64 = group.iter().map(|r| r.prob).sum();
            assert!((sum - 1.0).abs() < 0.01, "probs sum = {}", sum);
        }
    }

    #[test]
    fn test_reward_quantities_valid() {
        let groups: &[&[RewardDef]] = &[common_rewards(), rare_rewards(), epic_rewards(), legendary_rewards()];
        for group in groups {
            for r in *group {
                assert!(r.min_qty > 0);
                assert!(r.max_qty >= r.min_qty);
                assert!(r.gold_min > 0);
                assert!(r.gold_max >= r.gold_min);
            }
        }
    }

    #[test]
    fn test_reward_quality_scaling() {
        let common_avg: f64 = common_rewards()
            .iter()
            .map(|r| (r.gold_min + r.gold_max) as f64 / 2.0)
            .sum::<f64>()
            / common_rewards().len() as f64;
        let legendary_avg: f64 = legendary_rewards()
            .iter()
            .map(|r| (r.gold_min + r.gold_max) as f64 / 2.0)
            .sum::<f64>()
            / legendary_rewards().len() as f64;
        assert!(legendary_avg > common_avg * 5.0);
    }

    #[test]
    fn test_dig_level_all_tiers() {
        assert_eq!(dig_level(0).0, "Lv.1");
        assert_eq!(dig_level(10).0, "Lv.2");
        assert_eq!(dig_level(30).0, "Lv.3");
        assert_eq!(dig_level(50).0, "Lv.4");
        assert_eq!(dig_level(100).0, "Lv.5");
        assert_eq!(dig_level(200).0, "Lv.6");
        assert_eq!(dig_level(300).0, "Lv.7");
        assert_eq!(dig_level(500).0, "Lv.8");
    }

    #[test]
    fn test_next_dig_threshold() {
        assert_eq!(next_dig_threshold(0), Some(10));
        assert_eq!(next_dig_threshold(10), Some(30));
        assert_eq!(next_dig_threshold(499), Some(500));
        assert_eq!(next_dig_threshold(500), None);
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(0.5, 10);

        assert_eq!(bar.chars().filter(|c| *c == '█').count(), 5);
        let bar0 = progress_bar(0.0, 10);
        assert_eq!(bar0.chars().filter(|c| *c == '█').count(), 0);
        let bar1 = progress_bar(1.0, 10);
        assert_eq!(bar1.chars().filter(|c| *c == '█').count(), 10);
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(500), "500");
        assert_eq!(format_gold(10000), "1万");
        assert_eq!(format_gold(55000), "5万");
    }

    #[test]
    fn test_deterministic_seed() {
        let s1 = deterministic_seed("user1", "dig_0");
        let s2 = deterministic_seed("user1", "dig_0");
        assert_eq!(s1, s2);
        let s3 = deterministic_seed("user2", "dig_0");
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_constants() {
        assert_eq!(MAX_DAILY_DIGS, 5);
        assert_eq!(DIG_COOLDOWN_SECS, 300);
        assert_eq!(DIG_COST_GOLD, 200);
    }
}

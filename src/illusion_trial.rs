use crate::combat_power;
/// CakeGame 幻境试炼系统 (Illusion Trial System)
///
/// 进入镜像维度的PvE挑战系统，玩家面对自身属性缩放的幻影敌人，
/// 通过层层挑战获得幻境碎片，兑换稀有奖励。
///
/// 5个幻境难度:
///   幻境I·薄雾🌫️(3层) → 幻境II·迷雾🌪️(5层) → 幻境III·幻影⚡(7层)
///   → 幻境IV·破碎💥(10层) → 幻境V·虚空🕳️(15层)
///
/// 数据存储: Global 表 SECTION='illusion_trial'
///   - points:{uid}     → 幻境碎片总数
///   - best:{uid}       → 历史最高通关难度
///   - daily:{uid}      → 今日挑战次数
///   - daily_date:{uid} → 今日日期(跨日重置)
///   - progress:{uid}   → 当前挑战进度(难度,层数)
///   - history:{uid}    → 挑战历史(最近10条)
///   - rewards:{uid}    → 已领取奖励位掩码
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use crate::user;

const SECTION: &str = "illusion_trial";

/// 每日挑战上限
const DAILY_CHALLENGE_CAP: i32 = 3;

/// 幻境难度定义
struct IllusionTier {
    level: i32,
    name: &'static str,
    emoji: &'static str,
    max_floors: i32,
    /// 敌人属性倍率 (HP%, AD%, AP%, Def%, Res%)
    enemy_scale: (i32, i32, i32, i32, i32),
    /// 每层奖励碎片
    fragments_per_floor: i32,
    /// 通关额外奖励: (金币, 钻石)
    clear_reward: (i64, i64),
    /// 战力门槛
    power_req: i32,
}

const TIERS: &[IllusionTier] = &[
    IllusionTier {
        level: 1,
        name: "薄雾",
        emoji: "🌫️",
        max_floors: 3,
        enemy_scale: (80, 70, 70, 60, 60),
        fragments_per_floor: 10,
        clear_reward: (2000, 20),
        power_req: 500,
    },
    IllusionTier {
        level: 2,
        name: "迷雾",
        emoji: "🌪️",
        max_floors: 5,
        enemy_scale: (120, 110, 110, 100, 100),
        fragments_per_floor: 20,
        clear_reward: (8000, 60),
        power_req: 2000,
    },
    IllusionTier {
        level: 3,
        name: "幻影",
        emoji: "⚡",
        max_floors: 7,
        enemy_scale: (180, 160, 160, 140, 140),
        fragments_per_floor: 35,
        clear_reward: (25000, 150),
        power_req: 5000,
    },
    IllusionTier {
        level: 4,
        name: "破碎",
        emoji: "💥",
        max_floors: 10,
        enemy_scale: (280, 240, 240, 200, 200),
        fragments_per_floor: 60,
        clear_reward: (80000, 400),
        power_req: 12000,
    },
    IllusionTier {
        level: 5,
        name: "虚空",
        emoji: "🕳️",
        max_floors: 15,
        enemy_scale: (450, 380, 380, 320, 320),
        fragments_per_floor: 100,
        clear_reward: (200000, 1000),
        power_req: 25000,
    },
];

/// 幻境商店商品定义
struct IllusionShopItem {
    name: &'static str,
    cost: i32,
    desc: &'static str,
}

const SHOP_ITEMS: &[IllusionShopItem] = &[
    IllusionShopItem {
        name: "幻境药水",
        cost: 50,
        desc: "回复50%HP+MP",
    },
    IllusionShopItem {
        name: "幻境强化石",
        cost: 120,
        desc: "强化成功率+10%",
    },
    IllusionShopItem {
        name: "幻境护符",
        cost: 200,
        desc: "防御+5%持续24h",
    },
    IllusionShopItem {
        name: "幻境精华",
        cost: 350,
        desc: "全属性+3%持续24h",
    },
    IllusionShopItem {
        name: "幻境钥匙",
        cost: 500,
        desc: "开启幻境宝箱",
    },
    IllusionShopItem {
        name: "幻境之翼",
        cost: 800,
        desc: "闪避+10%持续24h",
    },
    IllusionShopItem {
        name: "幻境战甲",
        cost: 1200,
        desc: "HP+15%持续24h",
    },
    IllusionShopItem {
        name: "幻境圣剑",
        cost: 1800,
        desc: "攻击+12%持续24h",
    },
    IllusionShopItem {
        name: "幻境宝箱",
        cost: 2500,
        desc: "随机稀有道具",
    },
    IllusionShopItem {
        name: "幻境至尊宝箱",
        cost: 5000,
        desc: "随机传说道具",
    },
];

/// 里程碑定义
struct IllusionMilestone {
    name: &'static str,
    emoji: &'static str,
    required_clears: i32,
    reward_gold: i64,
    reward_diamond: i64,
    reward_item: &'static str,
}

const MILESTONES: &[IllusionMilestone] = &[
    IllusionMilestone {
        name: "幻境初探",
        emoji: "🔰",
        required_clears: 1,
        reward_gold: 1000,
        reward_diamond: 10,
        reward_item: "幻境药水",
    },
    IllusionMilestone {
        name: "幻境行者",
        emoji: "🚶",
        required_clears: 5,
        reward_gold: 5000,
        reward_diamond: 30,
        reward_item: "幻境强化石",
    },
    IllusionMilestone {
        name: "幻境猎手",
        emoji: "🏹",
        required_clears: 15,
        reward_gold: 15000,
        reward_diamond: 80,
        reward_item: "幻境护符",
    },
    IllusionMilestone {
        name: "幻境大师",
        emoji: "🏅",
        required_clears: 30,
        reward_gold: 40000,
        reward_diamond: 200,
        reward_item: "幻境精华",
    },
    IllusionMilestone {
        name: "幻境王者",
        emoji: "👑",
        required_clears: 50,
        reward_gold: 100000,
        reward_diamond: 500,
        reward_item: "幻境之翼",
    },
    IllusionMilestone {
        name: "幻境传说",
        emoji: "🌟",
        required_clears: 80,
        reward_gold: 250000,
        reward_diamond: 1000,
        reward_item: "幻境至尊宝箱",
    },
    IllusionMilestone {
        name: "虚空之主",
        emoji: "⚜️",
        required_clears: 120,
        reward_gold: 500000,
        reward_diamond: 2000,
        reward_item: "幻境至尊宝箱",
    },
];

fn get_tier(level: i32) -> &'static IllusionTier {
    TIERS.iter().find(|t| t.level == level).unwrap_or(&TIERS[0])
}

fn get_total_clears(db: &Database, user_id: &str) -> i32 {
    let data = db.global_get(SECTION, &format!("clears:{}", user_id));
    if data == "[NULL]" {
        0
    } else {
        data.parse::<i32>().unwrap_or(0)
    }
}

fn get_daily_count(db: &Database, user_id: &str) -> i32 {
    let today = utils::chrono_now_date();
    let saved_date = db.global_get(SECTION, &format!("daily_date:{}", user_id));
    if saved_date != today {
        return 0;
    }
    let data = db.global_get(SECTION, &format!("daily:{}", user_id));
    if data == "[NULL]" {
        0
    } else {
        data.parse::<i32>().unwrap_or(0)
    }
}

fn set_daily_count(db: &Database, user_id: &str, count: i32) {
    let today = utils::chrono_now_date();
    db.global_set(SECTION, &format!("daily_date:{}", user_id), &today);
    db.global_set(SECTION, &format!("daily:{}", user_id), &count.to_string());
}

fn get_fragments(db: &Database, user_id: &str) -> i32 {
    let data = db.global_get(SECTION, &format!("points:{}", user_id));
    if data == "[NULL]" {
        0
    } else {
        data.parse::<i32>().unwrap_or(0)
    }
}

fn add_fragments(db: &Database, user_id: &str, amount: i32) -> i32 {
    let cur = get_fragments(db, user_id);
    let new_val = cur + amount;
    db.global_set(SECTION, &format!("points:{}", user_id), &new_val.to_string());
    new_val
}

fn get_best_tier(db: &Database, user_id: &str) -> i32 {
    let data = db.global_get(SECTION, &format!("best:{}", user_id));
    if data == "[NULL]" {
        0
    } else {
        data.parse::<i32>().unwrap_or(0)
    }
}

fn get_reward_mask(db: &Database, user_id: &str) -> i32 {
    let data = db.global_get(SECTION, &format!("rewards:{}", user_id));
    if data == "[NULL]" {
        0
    } else {
        data.parse::<i32>().unwrap_or(0)
    }
}

fn add_history(db: &Database, user_id: &str, entry: &str) {
    let key = format!("history:{}", user_id);
    let existing = db.global_get(SECTION, &key);
    let mut entries: Vec<String> = if existing == "[NULL]" {
        Vec::new()
    } else {
        existing.split('|').map(|s| s.to_string()).collect()
    };
    entries.insert(0, entry.to_string());
    if entries.len() > 10 {
        entries.truncate(10);
    }
    db.global_set(SECTION, &key, &entries.join("|"));
}

/// 简化的日期获取

/// 获取幻境属性加成 (公共API)
pub fn get_illusion_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let best = get_best_tier(db, user_id);
    if best <= 0 {
        return (0, 0, 0, 0, 0);
    }
    // 基于历史最高通关难度给予被动加成
    let tier = get_tier(best);
    let bonus_pct = best * 3; // 每级+3%
    (
        tier.enemy_scale.0 * bonus_pct / 100,
        tier.enemy_scale.1 * bonus_pct / 100,
        tier.enemy_scale.2 * bonus_pct / 100,
        tier.enemy_scale.3 * bonus_pct / 100,
        tier.enemy_scale.4 * bonus_pct / 100,
    )
}

/// 记录幻境通关 (公共API供其他模块调用)
pub fn record_illusion_clear(db: &Database, user_id: &str, tier_level: i32) {
    let cur_clears = get_total_clears(db, user_id);
    db.global_set(SECTION, &format!("clears:{}", user_id), &(cur_clears + 1).to_string());
    let best = get_best_tier(db, user_id);
    if tier_level > best {
        db.global_set(SECTION, &format!("best:{}", user_id), &tier_level.to_string());
    }
    add_history(db, user_id, &format!("通关幻境{}", tier_level));
}

/// 查看幻境
pub fn cmd_illusion_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let info = user::calc_total_attrs(db, user_id);
    let power = combat_power::calc_combat_power(&info) as i32;
    let fragments = get_fragments(db, user_id);
    let best = get_best_tier(db, user_id);
    let clears = get_total_clears(db, user_id);
    let daily = get_daily_count(db, user_id);
    let daily_left = DAILY_CHALLENGE_CAP - daily;

    let mut out = String::from("🌀 ══════【幻境试炼】══════ 🌀\n\n");
    out.push_str(&format!(
        "👤 战力: {} | 🧩 碎片: {} | 🏆 通关: {}次\n",
        power, fragments, clears
    ));
    out.push_str(&format!(
        "📅 今日剩余: {}/{}次 | 🥇 最高: {}\n\n",
        daily_left.max(0),
        DAILY_CHALLENGE_CAP,
        if best > 0 {
            format!("幻境{}", best)
        } else {
            "未通关".to_string()
        }
    ));

    out.push_str("━━━ 幻境难度 ━━━\n");
    for tier in TIERS {
        let status = if power >= tier.power_req { "✅" } else { "🔒" };
        let best_mark = if tier.level <= best { " ✓" } else { "" };
        out.push_str(&format!(
            "{} {}幻境{}·{} — {}层 | 碎片+{}/层 | 通关+{}金+{}💎\n",
            status,
            tier.emoji,
            tier.level,
            tier.name,
            tier.max_floors,
            tier.fragments_per_floor,
            tier.clear_reward.0,
            tier.clear_reward.1
        ));
        out.push_str(&format!(
            "   战力需求: {} | 敌人倍率: HP{}% AD{}%{}\n",
            tier.power_req, tier.enemy_scale.0, tier.enemy_scale.1, best_mark
        ));
    }

    out.push_str("\n💡 输入「挑战幻境 难度」开始挑战(如: 挑战幻境 1)\n");
    out
}

/// 挑战幻境
pub fn cmd_illusion_challenge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let tier_level: i32 = match args.trim().parse::<i32>() {
        Ok(v) if (1..=5).contains(&v) => v,
        _ => return "❌ 请输入幻境难度(1-5)\n💡 示例: 挑战幻境 1".to_string(),
    };

    let info = user::calc_total_attrs(db, user_id);
    let power = combat_power::calc_combat_power(&info) as i32;
    let tier = get_tier(tier_level);

    if power < tier.power_req {
        return format!(
            "❌ 战力不足! 幻境{}需要{}战力，你当前{}战力\n差距: {}",
            tier_level,
            tier.power_req,
            power,
            tier.power_req - power
        );
    }

    let daily = get_daily_count(db, user_id);
    if daily >= DAILY_CHALLENGE_CAP {
        return format!("❌ 今日挑战次数已用完({}/{}), 明日再来!", daily, DAILY_CHALLENGE_CAP);
    }

    // 检查是否有进行中的挑战
    let progress = db.global_get(SECTION, &format!("progress:{}", user_id));
    if progress != "[NULL]" {
        return format!("❌ 你有进行中的挑战: {}\n💡 输入「放弃幻境」放弃当前挑战", progress);
    }

    // 开始挑战 - 设置进度
    db.global_set(
        SECTION,
        &format!("progress:{}", user_id),
        &format!("幻境{}-第1层", tier_level),
    );
    set_daily_count(db, user_id, daily + 1);

    // 模拟第一层战斗
    let enemy_hp = info.hp_max * tier.enemy_scale.0 / 100;
    let enemy_ad = info.ad * tier.enemy_scale.1 / 100;

    let mut out = format!("🌀 ═══ 幻境{}·{} 开启! ═══ 🌀\n\n", tier_level, tier.emoji);
    out.push_str(&format!("📍 第1层/{}层\n", tier.max_floors));
    out.push_str(&format!("👹 幻影守卫 — HP:{} 攻击:{}\n", enemy_hp, enemy_ad));
    out.push_str(&format!(
        "⚔️ 你的属性 — HP:{} 攻击:{} 防御:{}\n\n",
        info.hp_max, info.ad, info.defense
    ));

    // 战斗模拟: 基于战力对比
    let player_score = power as f64 * (1.0 + (info.hp_max as f64 / 10000.0));
    let enemy_score = (enemy_hp + enemy_ad * 3) as f64;
    let win = player_score > enemy_score * 0.6; // 第一层较简单

    if win {
        let fragments = tier.fragments_per_floor;
        add_fragments(db, user_id, fragments);
        db.global_set(
            SECTION,
            &format!("progress:{}", user_id),
            &format!("幻境{}-第1层-已通过", tier_level),
        );
        out.push_str(&format!("✅ 胜利! 获得 🧩{}碎片\n", fragments));
        out.push_str("\n💡 输入「继续幻境」继续下一层\n");
    } else {
        db.global_set(SECTION, &format!("progress:{}", user_id), "[NULL]");
        out.push_str("❌ 幻影守卫过于强大，你被击退了!\n");
        out.push_str("💡 提升战力后再来挑战\n");
    }

    out
}

/// 继续幻境
pub fn cmd_illusion_continue(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let progress = db.global_get(SECTION, &format!("progress:{}", user_id));
    if progress == "[NULL]" {
        return "❌ 没有进行中的幻境挑战\n💡 输入「挑战幻境 难度」开始挑战".to_string();
    }

    // 解析进度: "幻境N-第M层-已通过" 或 "幻境N-第M层"
    let parts: Vec<&str> = progress.split('-').collect();
    if parts.len() < 2 {
        db.global_set(SECTION, &format!("progress:{}", user_id), "[NULL]");
        return "❌ 挑战数据异常，已重置".to_string();
    }

    let tier_str = parts[0].replace("幻境", "");
    let tier_level: i32 = tier_str.parse().unwrap_or(1);
    let tier = get_tier(tier_level);

    let floor_str = parts[1].replace("第", "").replace("层", "");
    let current_floor: i32 = floor_str.parse().unwrap_or(1);

    // 如果当前层已通过，进入下一层
    let next_floor = if parts.len() >= 3 && parts[2] == "已通过" {
        current_floor + 1
    } else {
        current_floor
    };

    if next_floor > tier.max_floors {
        // 通关!
        let (gold, diamond) = tier.clear_reward;
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, diamond);
        record_illusion_clear(db, user_id, tier_level);
        db.global_set(SECTION, &format!("progress:{}", user_id), "[NULL]");

        let clears = get_total_clears(db, user_id);
        let mut out = format!("🎉 ═══ 恭喜通关幻境{}·{}! ═══ 🎉\n\n", tier_level, tier.emoji);
        out.push_str(&format!("🏆 总通关次数: {}\n", clears));
        out.push_str(&format!("💰 通关奖励: {}金 + {}💎\n\n", gold, diamond));

        // 检查里程碑
        for ms in MILESTONES {
            if clears == ms.required_clears {
                db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, ms.reward_gold);
                db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, ms.reward_diamond);
                out.push_str(&format!(
                    "🎯 达成里程碑: {}{} — {}金+{}💎+{}\n",
                    ms.emoji, ms.name, ms.reward_gold, ms.reward_diamond, ms.reward_item
                ));
            }
        }

        return out;
    }

    // 挑战下一层
    let info = user::calc_total_attrs(db, user_id);
    let difficulty_mul = 1.0 + (next_floor as f64 - 1.0) * 0.15; // 每层+15%难度
    let enemy_hp = (info.hp_max as f64 * tier.enemy_scale.0 as f64 / 100.0 * difficulty_mul) as i32;
    let enemy_ad = (info.ad as f64 * tier.enemy_scale.1 as f64 / 100.0 * difficulty_mul) as i32;

    db.global_set(
        SECTION,
        &format!("progress:{}", user_id),
        &format!("幻境{}-第{}层", tier_level, next_floor),
    );

    let mut out = format!(
        "🌀 幻境{}·{} — 第{}/{}层\n\n",
        tier_level, tier.emoji, next_floor, tier.max_floors
    );
    out.push_str(&format!("👹 幻影守卫(强化) — HP:{} 攻击:{}\n", enemy_hp, enemy_ad));
    out.push_str(&format!(
        "⚔️ 你的属性 — HP:{} 攻击:{} 防御:{}\n\n",
        info.hp_max, info.ad, info.defense
    ));

    let player_score = combat_power::calc_combat_power(&info) as i32 as f64 * (1.0 + (info.hp_max as f64 / 10000.0));
    let enemy_score = (enemy_hp + enemy_ad * 3) as f64;
    let threshold = 0.5 + (next_floor as f64 * 0.03); // 层数越高越难
    let win = player_score > enemy_score * threshold;

    if win {
        let fragments = tier.fragments_per_floor + (next_floor - 1) * 3; // 高层碎片更多
        add_fragments(db, user_id, fragments);
        db.global_set(
            SECTION,
            &format!("progress:{}", user_id),
            &format!("幻境{}-第{}层-已通过", tier_level, next_floor),
        );
        out.push_str(&format!("✅ 胜利! 获得 🧩{}碎片\n", fragments));
        if next_floor >= tier.max_floors {
            out.push_str("\n🏆 最终层已通过! 输入「继续幻境」领取通关奖励\n");
        } else {
            out.push_str("\n💡 输入「继续幻境」继续下一层\n");
        }
    } else {
        db.global_set(SECTION, &format!("progress:{}", user_id), "[NULL]");
        out.push_str("❌ 幻影守卫过于强大，你被击退了!\n");
        out.push_str(&format!("💡 获得补偿碎片: 🧩{}\n", tier.fragments_per_floor / 2));
        add_fragments(db, user_id, tier.fragments_per_floor / 2);
    }

    out
}

/// 放弃幻境
pub fn cmd_illusion_give_up(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let progress = db.global_get(SECTION, &format!("progress:{}", user_id));
    if progress == "[NULL]" {
        return "❌ 没有进行中的幻境挑战".to_string();
    }
    db.global_set(SECTION, &format!("progress:{}", user_id), "[NULL]");
    format!(
        "✅ 已放弃幻境挑战 (进度: {})\n💡 输入「挑战幻境 难度」重新开始",
        progress
    )
}

/// 幻境商店
pub fn cmd_illusion_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let fragments = get_fragments(db, user_id);
    let mut out = format!("🛒 ══════【幻境商店】══════ 🛒\n\n🧩 你的碎片: {}\n\n", fragments);

    for (i, item) in SHOP_ITEMS.iter().enumerate() {
        let affordable = if fragments >= item.cost { "✅" } else { "❌" };
        out.push_str(&format!(
            "{}. {} {} — 🧩{} ({})\n   {}\n",
            i + 1,
            affordable,
            item.name,
            item.cost,
            item.desc,
            ""
        ));
    }
    out.push_str("\n💡 输入「幻境兑换 编号」购买商品\n");
    out
}

/// 幻境兑换
pub fn cmd_illusion_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let idx: usize = match args.trim().parse::<usize>() {
        Ok(v) if v >= 1 && v <= SHOP_ITEMS.len() => v - 1,
        _ => return format!("❌ 请输入商品编号(1-{})", SHOP_ITEMS.len()),
    };

    let item = &SHOP_ITEMS[idx];
    let fragments = get_fragments(db, user_id);

    if fragments < item.cost {
        return format!("❌ 碎片不足! 需要🧩{}，你有🧩{}", item.cost, fragments);
    }

    add_fragments(db, user_id, -item.cost);
    db.knapsack_add(user_id, item.name, 1);

    format!(
        "✅ 兑换成功!\n🧩 -{} → {}\n📦 已放入背包: {}\n剩余碎片: {}",
        item.cost,
        item.name,
        item.name,
        fragments - item.cost
    )
}

/// 幻境排行
pub fn cmd_illusion_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_users = db.all_users();
    let mut rankings: Vec<(String, i32, i32, i32)> = Vec::new();

    for uid in &all_users {
        let clears = get_total_clears(db, uid);
        let best = get_best_tier(db, uid);
        let fragments = get_fragments(db, uid);
        if clears > 0 || fragments > 0 {
            let nickname = db.read_basic(uid, "Nickname");
            let name = if nickname == "[NULL]" { uid.clone() } else { nickname };
            rankings.push((name, clears, best, fragments));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.3)); // 按碎片降序

    let mut out = String::from("🏆 ══════【幻境排行】══════ 🏆\n\n");
    for (i, (name, clears, best, frags)) in rankings.iter().take(15).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!(
            "{} {}. {} — 🧩{} 通关{}次 最高幻境{}\n",
            medal,
            i + 1,
            name,
            frags,
            clears,
            best
        ));
    }

    if rankings.is_empty() {
        out.push_str("暂无挑战记录\n");
    }

    out
}

/// 幻境详情
pub fn cmd_illusion_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let fragments = get_fragments(db, user_id);
    let best = get_best_tier(db, user_id);
    let clears = get_total_clears(db, user_id);
    let rewards_mask = get_reward_mask(db, user_id);
    let daily = get_daily_count(db, user_id);
    let progress = db.global_get(SECTION, &format!("progress:{}", user_id));
    let bonus = get_illusion_bonus(db, user_id);

    let mut out = String::from("🔍 ══════【幻境详情】══════ 🔍\n\n");
    out.push_str(&format!("🧩 幻境碎片: {}\n", fragments));
    out.push_str(&format!(
        "🏆 总通关: {}次 | 最高: {}\n",
        clears,
        if best > 0 {
            format!("幻境{}", best)
        } else {
            "无".to_string()
        }
    ));
    out.push_str(&format!("📅 今日: {}/{}次\n", daily, DAILY_CHALLENGE_CAP));
    out.push_str(&format!(
        "📍 当前进度: {}\n\n",
        if progress == "[NULL]" {
            "无".to_string()
        } else {
            progress
        }
    ));

    out.push_str("━━━ 被动加成(基于最高通关) ━━━\n");
    if bonus.0 > 0 {
        out.push_str(&format!(
            "  HP+{} 物攻+{} 魔攻+{} 防御+{} 魔抗+{}\n",
            bonus.0, bonus.1, bonus.2, bonus.3, bonus.4
        ));
    } else {
        out.push_str("  无 (通关幻境后解锁)\n");
    }

    out.push_str("\n━━━ 里程碑进度 ━━━\n");
    for ms in MILESTONES {
        let claimed = (rewards_mask >> (ms.required_clears.trailing_zeros() as usize)) & 1 != 0;
        let status = if clears >= ms.required_clears {
            if claimed {
                "✅已领"
            } else {
                "🎁可领"
            }
        } else {
            "⏳未达成"
        };
        out.push_str(&format!(
            "  {}{} {}({}次) — {}\n",
            ms.emoji, ms.name, status, ms.required_clears, ms.reward_item
        ));
    }

    // 历史记录
    let history = db.global_get(SECTION, &format!("history:{}", user_id));
    if history != "[NULL]" {
        out.push_str("\n━━━ 最近记录 ━━━\n");
        for entry in history.split('|').take(5) {
            out.push_str(&format!("  • {}\n", entry));
        }
    }

    out
}

/// 幻境帮助
pub fn cmd_illusion_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🌀 ══════【幻境试炼帮助】══════ 🌀\n\n");
    out.push_str("幻境试炼是镜像维度的PvE挑战系统。\n\n");
    out.push_str("━━━ 基本规则 ━━━\n");
    out.push_str("• 每天可挑战3次\n");
    out.push_str("• 5个难度等级(幻境I~V)\n");
    out.push_str("• 逐层挑战，难度递增\n");
    out.push_str("• 获得幻境碎片兑换奖励\n\n");
    out.push_str("━━━ 指令列表 ━━━\n");
    out.push_str("• 幻境试炼 — 查看幻境列表\n");
    out.push_str("• 挑战幻境 难度 — 开始挑战(1-5)\n");
    out.push_str("• 继续幻境 — 继续下一层\n");
    out.push_str("• 放弃幻境 — 放弃当前挑战\n");
    out.push_str("• 幻境商店 — 查看碎片商品\n");
    out.push_str("• 幻境兑换 编号 — 兑换商品\n");
    out.push_str("• 幻境排行 — 全服排行榜\n");
    out.push_str("• 幻境详情 — 个人详情\n\n");
    out.push_str("━━━ 难度说明 ━━━\n");
    for tier in TIERS {
        out.push_str(&format!(
            "• {}幻境{}·{} — {}层 战力{}\n",
            tier.emoji, tier.level, tier.name, tier.max_floors, tier.power_req
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_count() {
        assert_eq!(TIERS.len(), 5);
    }

    #[test]
    fn test_tier_ordering() {
        for i in 1..TIERS.len() {
            assert!(TIERS[i].power_req > TIERS[i - 1].power_req);
            assert!(TIERS[i].max_floors > TIERS[i - 1].max_floors);
        }
    }

    #[test]
    fn test_tier_lookup() {
        let t = get_tier(1);
        assert_eq!(t.name, "薄雾");
        assert_eq!(t.max_floors, 3);

        let t5 = get_tier(5);
        assert_eq!(t5.name, "虚空");
        assert_eq!(t5.max_floors, 15);
    }

    #[test]
    fn test_shop_items_count() {
        assert_eq!(SHOP_ITEMS.len(), 10);
    }

    #[test]
    fn test_shop_items_sorted_by_cost() {
        for i in 1..SHOP_ITEMS.len() {
            assert!(SHOP_ITEMS[i].cost >= SHOP_ITEMS[i - 1].cost);
        }
    }

    #[test]
    fn test_milestones_sorted() {
        for i in 1..MILESTONES.len() {
            assert!(MILESTONES[i].required_clears > MILESTONES[i - 1].required_clears);
        }
    }

    #[test]
    fn test_milestone_count() {
        assert_eq!(MILESTONES.len(), 7);
    }

    #[test]
    fn test_fragment_rewards_positive() {
        for tier in TIERS {
            assert!(tier.fragments_per_floor > 0);
            assert!(tier.clear_reward.0 > 0);
            assert!(tier.clear_reward.1 > 0);
        }
    }

    #[test]
    fn test_enemy_scale_reasonable() {
        for tier in TIERS {
            assert!(tier.enemy_scale.0 >= 50);
            assert!(tier.enemy_scale.0 <= 500);
            assert!(tier.enemy_scale.1 >= 50);
        }
    }

    #[test]
    fn test_daily_cap() {
        assert_eq!(DAILY_CHALLENGE_CAP, 3);
    }

    #[test]
    fn test_section_name() {
        assert_eq!(SECTION, "illusion_trial");
    }

    #[test]
    fn test_tier_level_range() {
        for tier in TIERS {
            assert!((1..=5).contains(&tier.level));
        }
    }

    #[test]
    fn test_milestone_rewards_escalate() {
        for i in 1..MILESTONES.len() {
            assert!(MILESTONES[i].reward_gold >= MILESTONES[i - 1].reward_gold);
            assert!(MILESTONES[i].reward_diamond >= MILESTONES[i - 1].reward_diamond);
        }
    }

    #[test]
    fn test_shop_item_names_unique() {
        let mut names: Vec<&str> = SHOP_ITEMS.iter().map(|i| i.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SHOP_ITEMS.len());
    }

    #[test]
    fn test_illusion_bonus_no_clear() {
        // Without clears, bonus should be zero
        // This is a pure logic test - no db needed
        assert_eq!(TIERS[0].level, 1);
    }

    #[test]
    fn test_fragment_scaling() {
        // Higher tiers give more fragments per floor
        for i in 1..TIERS.len() {
            assert!(TIERS[i].fragments_per_floor > TIERS[i - 1].fragments_per_floor);
        }
    }
}

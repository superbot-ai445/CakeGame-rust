/// CakeGame 公会秘境系统 (Guild Secret Realm)
///
/// 公会成员共同探索的多层秘境，全公会共享进度。
/// 每层有怪物守卫和宝箱奖励，击败守卫可推进到下一层。
/// 每周重置进度，层数越高奖励越丰厚。
///
/// 数据存储: Global 表 SECTION='guild_realm_{guild_name}'
///
/// 指令: 公会秘境, 秘境探索, 秘境排行, 秘境奖励
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::time::{SystemTime, UNIX_EPOCH};

/// 秘境层数定义
#[allow(dead_code)]
struct RealmFloor {
    floor: i32,
    name: &'static str,
    monster_name: &'static str,
    monster_hp: i64,
    monster_ad: i32,
    gold_reward: i64,
    exp_reward: i32,
    diamond_reward: i32,
    /// 层数越高，掉落稀有物品概率越大
    rare_drop_rate: i32, // 万分比
    rare_drop_item: &'static str,
}

const REALM_FLOORS: &[RealmFloor] = &[
    RealmFloor {
        floor: 1,
        name: "幽暗入口",
        monster_name: "暗影蝙蝠",
        monster_hp: 500,
        monster_ad: 30,
        gold_reward: 200,
        exp_reward: 100,
        diamond_reward: 0,
        rare_drop_rate: 500,
        rare_drop_item: "暗影之翼",
    },
    RealmFloor {
        floor: 2,
        name: "迷雾走廊",
        monster_name: "迷雾幽灵",
        monster_hp: 1000,
        monster_ad: 50,
        gold_reward: 400,
        exp_reward: 200,
        diamond_reward: 5,
        rare_drop_rate: 600,
        rare_drop_item: "幽灵精华",
    },
    RealmFloor {
        floor: 3,
        name: "毒沼大厅",
        monster_name: "沼泽毒蟒",
        monster_hp: 2000,
        monster_ad: 80,
        gold_reward: 600,
        exp_reward: 350,
        diamond_reward: 10,
        rare_drop_rate: 700,
        rare_drop_item: "蟒蛇胆",
    },
    RealmFloor {
        floor: 4,
        name: "烈焰回廊",
        monster_name: "火焰魔像",
        monster_hp: 3500,
        monster_ad: 120,
        gold_reward: 800,
        exp_reward: 500,
        diamond_reward: 15,
        rare_drop_rate: 800,
        rare_drop_item: "魔像核心",
    },
    RealmFloor {
        floor: 5,
        name: "冰封地窖",
        monster_name: "冰霜巨灵",
        monster_hp: 5000,
        monster_ad: 160,
        gold_reward: 1000,
        exp_reward: 700,
        diamond_reward: 20,
        rare_drop_rate: 900,
        rare_drop_item: "冰霜结晶",
    },
    RealmFloor {
        floor: 6,
        name: "雷霆祭坛",
        monster_name: "雷电元素",
        monster_hp: 7000,
        monster_ad: 200,
        gold_reward: 1500,
        exp_reward: 1000,
        diamond_reward: 30,
        rare_drop_rate: 1000,
        rare_drop_item: "雷元素碎片",
    },
    RealmFloor {
        floor: 7,
        name: "暗影深渊",
        monster_name: "暗影领主",
        monster_hp: 10000,
        monster_ad: 260,
        gold_reward: 2000,
        exp_reward: 1500,
        diamond_reward: 40,
        rare_drop_rate: 1200,
        rare_drop_item: "暗影之冠",
    },
    RealmFloor {
        floor: 8,
        name: "炼狱之门",
        monster_name: "炼狱守卫",
        monster_hp: 15000,
        monster_ad: 340,
        gold_reward: 3000,
        exp_reward: 2000,
        diamond_reward: 50,
        rare_drop_rate: 1400,
        rare_drop_item: "炼狱之石",
    },
    RealmFloor {
        floor: 9,
        name: "时空裂缝",
        monster_name: "时空裂隙兽",
        monster_hp: 22000,
        monster_ad: 440,
        gold_reward: 4000,
        exp_reward: 3000,
        diamond_reward: 60,
        rare_drop_rate: 1600,
        rare_drop_item: "时空碎片",
    },
    RealmFloor {
        floor: 10,
        name: "秘境之心",
        monster_name: "秘境守护者",
        monster_hp: 30000,
        monster_ad: 550,
        gold_reward: 5000,
        exp_reward: 5000,
        diamond_reward: 100,
        rare_drop_rate: 2000,
        rare_drop_item: "秘境之心碎片",
    },
    RealmFloor {
        floor: 11,
        name: "混沌领域",
        monster_name: "混沌巨龙",
        monster_hp: 40000,
        monster_ad: 700,
        gold_reward: 6000,
        exp_reward: 6000,
        diamond_reward: 120,
        rare_drop_rate: 2200,
        rare_drop_item: "龙之逆鳞",
    },
    RealmFloor {
        floor: 12,
        name: "神陨之地",
        monster_name: "堕落神使",
        monster_hp: 55000,
        monster_ad: 900,
        gold_reward: 8000,
        exp_reward: 8000,
        diamond_reward: 150,
        rare_drop_rate: 2500,
        rare_drop_item: "神陨结晶",
    },
];

/// 每层每个玩家的探索伤害上限（防止单人碾压）
const MAX_DAMAGE_PER_EXPLORE: i64 = 5000;
/// 每日探索次数限制
const DAILY_EXPLORE_LIMIT: i32 = 5;
/// 探索冷却时间（秒）
const EXPLORE_COOLDOWN: i64 = 60;

/// 获取当前时间戳
fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 获取今日日期字符串 YYYY-MM-DD
fn today_str() -> String {
    // 简单的日期计算：基于秒数
    let ts = now_ts();
    let days = ts / 86400;
    // Unix epoch 1970-01-01 是星期四(第4天)
    // 从 days 计算年月日
    let mut y = 1970i32;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: &[i32] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for (i, &d) in month_days.iter().enumerate() {
        if remaining < d as i64 {
            m = i;
            break;
        }
        remaining -= d as i64;
        m = i + 1;
    }
    format!("{:04}-{:02}-{:02}", y, m + 1, remaining + 1)
}

/// 获取本周标识 (year-week)
fn this_week_str() -> String {
    let ts = now_ts();
    let days = ts / 86400;
    // 计算星期几 (0=周一)
    let weekday = ((days + 3) % 7) as i32; // Unix epoch 是周四，+3 让周一=0
    let week_start = days - weekday as i64;
    format!("w{}", week_start / 7)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// 获取公会当前秘境层数
fn get_guild_floor(db: &Database, guild: &str) -> i32 {
    let key = format!("guild_realm_{}", guild);
    let floor_str = db.global_get(&key, "current_floor");
    floor_str.parse().unwrap_or(1).max(1)
}

/// 获取当前层剩余HP
fn get_floor_hp(db: &Database, guild: &str, floor: i32) -> i64 {
    let key = format!("guild_realm_{}", guild);
    let hp_str = db.global_get(&key, &format!("floor_{}_hp", floor));
    let parsed: i64 = hp_str.parse().unwrap_or(0);
    if parsed <= 0 {
        REALM_FLOORS
            .iter()
            .find(|f| f.floor == floor)
            .map(|f| f.monster_hp)
            .unwrap_or(0)
    } else {
        parsed
    }
}

/// 设置当前层HP
fn set_floor_hp(db: &Database, guild: &str, floor: i32, hp: i64) {
    let key = format!("guild_realm_{}", guild);
    db.global_set(&key, &format!("floor_{}_hp", floor), &hp.to_string());
}

/// 推进到下一层
fn advance_floor(db: &Database, guild: &str) {
    let current = get_guild_floor(db, guild);
    let key = format!("guild_realm_{}", guild);
    db.global_set(&key, "current_floor", &(current + 1).to_string());
}

/// 获取玩家今日探索次数
fn get_today_explores(db: &Database, user_id: &str, guild: &str) -> i32 {
    let today = today_str();
    let key = format!("guild_realm_{}", guild);
    let stored_date = db.global_get(&key, &format!("explore_date_{}", user_id));
    if stored_date != today {
        0
    } else {
        let count_str = db.global_get(&key, &format!("explore_count_{}", user_id));
        count_str.parse().unwrap_or(0)
    }
}

/// 增加探索次数
fn increment_explores(db: &Database, user_id: &str, guild: &str) {
    let today = today_str();
    let key = format!("guild_realm_{}", guild);
    let count = get_today_explores(db, user_id, guild) + 1;
    db.global_set(&key, &format!("explore_date_{}", user_id), &today);
    db.global_set(&key, &format!("explore_count_{}", user_id), &count.to_string());
}

/// 获取玩家上次探索时间戳
fn get_last_explore_time(db: &Database, user_id: &str, guild: &str) -> i64 {
    let key = format!("guild_realm_{}", guild);
    let ts_str = db.global_get(&key, &format!("last_explore_{}", user_id));
    ts_str.parse().unwrap_or(0)
}

/// 设置玩家探索时间戳
fn set_last_explore_time(db: &Database, user_id: &str, guild: &str, ts: i64) {
    let key = format!("guild_realm_{}", guild);
    db.global_set(&key, &format!("last_explore_{}", user_id), &ts.to_string());
}

/// 记录玩家贡献伤害
fn add_contribution(db: &Database, user_id: &str, guild: &str, damage: i64) {
    let key = format!("guild_realm_{}", guild);
    let current: i64 = db
        .global_get(&key, &format!("contrib_{}", user_id))
        .parse()
        .unwrap_or(0);
    db.global_set(&key, &format!("contrib_{}", user_id), &(current + damage).to_string());
}

/// 获取玩家贡献
fn get_contribution(db: &Database, user_id: &str, guild: &str) -> i64 {
    let key = format!("guild_realm_{}", guild);
    db.global_get(&key, &format!("contrib_{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// 获取公会成员列表
fn get_guild_members(db: &Database, guild: &str) -> Vec<String> {
    let raw = db.global_get("UnionData", &format!("{}.Members", guild));
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 检查并重置每周进度
fn check_and_reset_weekly(db: &Database, guild: &str) {
    let key = format!("guild_realm_{}", guild);
    let this_week = this_week_str();
    let stored_week = db.global_get(&key, "reset_week");
    if stored_week != this_week {
        db.global_set(&key, "current_floor", "1");
        db.global_set(&key, "reset_week", &this_week);
        if let Some(f) = REALM_FLOORS.first() {
            set_floor_hp(db, guild, 1, f.monster_hp);
        }
    }
}

/// 进度条
fn progress_bar(pct: i64, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// 简单哈希
fn djb2_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// 格式化金币数值
fn format_gold(gold: i64) -> String {
    if gold >= 10000 {
        format!("{}万", gold / 10000)
    } else {
        format!("{}", gold)
    }
}

/// 公会秘境 — 查看当前公会秘境状态
pub fn cmd_guild_realm(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您未加入公会！请先加入公会再探索秘境。", prefix);
    }

    check_and_reset_weekly(db, &guild);

    let current_floor = get_guild_floor(db, &guild);
    let today_explores = get_today_explores(db, user_id, &guild);
    let my_contrib = get_contribution(db, user_id, &guild);

    let mut r = format!("{}\n═══ 🌀 公会秘境 ═══\n", prefix);
    r.push_str(&format!("🏰 公会: {}\n", guild));
    r.push_str(&format!("📊 当前层数: {}/{}\n", current_floor, REALM_FLOORS.len()));
    r.push_str(&format!("🎯 今日探索: {}/{} 次\n", today_explores, DAILY_EXPLORE_LIMIT));
    r.push_str(&format!("💎 我的贡献: {} 伤害\n", my_contrib));
    r.push_str("\n📜 秘境层数:\n");

    for f in REALM_FLOORS {
        let status = if f.floor < current_floor {
            "✅ 已通关".to_string()
        } else if f.floor == current_floor {
            let hp = get_floor_hp(db, &guild, f.floor);
            let pct = if f.monster_hp > 0 {
                (hp * 100 / f.monster_hp).clamp(0, 100)
            } else {
                0
            };
            let bar = progress_bar(pct, 10);
            format!("⚔️ 进行中 {} {}%", bar, pct)
        } else {
            "🔒 未解锁".to_string()
        };
        r.push_str(&format!(
            "  {}. {} [{}] — {}\n",
            f.floor, f.name, f.monster_name, status
        ));
    }

    r.push_str(&format!(
        "\n💡 指令: 秘境探索 (每日{}次，{}秒冷却)\n",
        DAILY_EXPLORE_LIMIT, EXPLORE_COOLDOWN
    ));
    r
}

/// 秘境探索 — 攻击当前层怪物
pub fn cmd_realm_explore(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您未加入公会！", prefix);
    }

    check_and_reset_weekly(db, &guild);

    let current_floor = get_guild_floor(db, &guild);
    if current_floor as usize > REALM_FLOORS.len() {
        return format!("{}\n🎉 恭喜！公会已通关全部秘境层数！等待下周重置。", prefix);
    }

    // 检查每日次数
    let today_explores = get_today_explores(db, user_id, &guild);
    if today_explores >= DAILY_EXPLORE_LIMIT {
        return format!(
            "{}\n❌ 今日探索次数已用完 ({}/{})！\n💡 明日可继续探索。",
            prefix, today_explores, DAILY_EXPLORE_LIMIT
        );
    }

    // 检查冷却
    let now = now_ts();
    let last = get_last_explore_time(db, user_id, &guild);
    if last > 0 && now - last < EXPLORE_COOLDOWN {
        let remaining = EXPLORE_COOLDOWN - (now - last);
        return format!("{}\n⏳ 探索冷却中！还需等待 {} 秒。", prefix, remaining);
    }

    // 获取当前层信息
    let floor_def = &REALM_FLOORS[current_floor as usize - 1];
    let floor_hp = get_floor_hp(db, &guild, current_floor);

    // 计算伤害（基于玩家战力）
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let ad: i32 = db.read_basic(user_id, ITEM_AD).parse().unwrap_or(10);
    let ap: i32 = db.read_basic(user_id, ITEM_AP).parse().unwrap_or(0);
    let base_damage = (ad + ap).max(10) as i64;
    let level_bonus = 100 + (level as i64 - 1) * 5;
    let mut damage = base_damage * level_bonus / 100;

    // 暴击检查（15%基础暴击率）
    let crit: i32 = db.read_basic(user_id, ITEM_CRIT).parse().unwrap_or(0);
    let crit_rate = (1500 + crit * 10).min(5000);
    let mut is_crit = false;
    {
        let hash = djb2_hash(&format!("{}{}{}", user_id, now, current_floor));
        let roll = (hash % 10000) as i32;
        if roll < crit_rate {
            damage = damage * 150 / 100;
            is_crit = true;
        }
    }

    // 限制单次探索伤害
    damage = damage.min(MAX_DAMAGE_PER_EXPLORE);

    // 应用伤害
    let remaining_hp = (floor_hp - damage).max(0);
    set_floor_hp(db, &guild, current_floor, remaining_hp);

    // 记录贡献
    add_contribution(db, user_id, &guild, damage);

    // 更新探索次数和时间
    increment_explores(db, user_id, &guild);
    set_last_explore_time(db, user_id, &guild, now);

    let mut r = format!("{}\n═══ ⚔️ 秘境探索 ═══\n", prefix);
    r.push_str(&format!("📍 当前: 第{}层 [{}]\n", current_floor, floor_def.name));
    r.push_str(&format!("👹 守卫: {}\n", floor_def.monster_name));

    if is_crit {
        r.push_str(&format!("💥 暴击！造成 {} 伤害！\n", damage));
    } else {
        r.push_str(&format!("⚔️ 造成 {} 伤害\n", damage));
    }

    if remaining_hp <= 0 {
        r.push_str(&format!("\n🎉 击败了 {}！\n", floor_def.monster_name));
        r.push_str(&format!("💰 金币 +{}\n", format_gold(floor_def.gold_reward)));
        r.push_str(&format!("✨ 经验 +{}\n", floor_def.exp_reward));
        if floor_def.diamond_reward > 0 {
            r.push_str(&format!("💎 钻石 +{}\n", floor_def.diamond_reward));
        }

        // 掉落稀有物品
        let drop_hash = djb2_hash(&format!("drop_{}{}{}", user_id, now, current_floor));
        let drop_roll = (drop_hash % 10000) as i32;
        if drop_roll < floor_def.rare_drop_rate {
            r.push_str(&format!("🎁 获得稀有物品: {}！\n", floor_def.rare_drop_item));
            db.knapsack_add(user_id, floor_def.rare_drop_item, 1);
        }

        // 发放奖励
        db.modify_currency(user_id, CURRENCY_GOLD, "add", floor_def.gold_reward);
        let cur_exp: i32 = db.read_basic(user_id, ITEM_EXP).parse().unwrap_or(0);
        db.write_basic_int(user_id, ITEM_EXP, cur_exp + floor_def.exp_reward);
        if floor_def.diamond_reward > 0 {
            db.modify_currency(user_id, CURRENCY_DIAMOND, "add", floor_def.diamond_reward as i64);
        }

        // 推进到下一层
        if current_floor < REALM_FLOORS.len() as i32 {
            advance_floor(db, &guild);
            let next_floor = current_floor + 1;
            let next_def = &REALM_FLOORS[next_floor as usize - 1];
            set_floor_hp(db, &guild, next_floor, next_def.monster_hp);
            r.push_str(&format!("\n🔓 解锁第{}层: [{}]！\n", next_floor, next_def.name));
        } else {
            r.push_str("\n🏆 恭喜！公会已通关全部秘境！\n");
        }
    } else {
        let pct = if floor_def.monster_hp > 0 {
            (remaining_hp * 100 / floor_def.monster_hp).min(100)
        } else {
            0
        };
        let bar = progress_bar(pct, 10);
        r.push_str(&format!("❤️ 剩余: {} HP ({} {}%)\n", remaining_hp, bar, pct));
    }

    let explores_left = DAILY_EXPLORE_LIMIT - get_today_explores(db, user_id, &guild);
    r.push_str(&format!("\n🎯 剩余探索: {} 次\n", explores_left));
    r
}

/// 秘境排行 — 公会成员贡献排行
pub fn cmd_realm_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您未加入公会！", prefix);
    }

    check_and_reset_weekly(db, &guild);

    let key = format!("guild_realm_{}", guild);

    // 获取所有公会成员的贡献
    let members = get_guild_members(db, &guild);
    let mut contributions: Vec<(String, i64)> = Vec::new();
    for member_id in &members {
        let contrib: i64 = db
            .global_get(&key, &format!("contrib_{}", member_id))
            .parse()
            .unwrap_or(0);
        if contrib > 0 {
            contributions.push((member_id.clone(), contrib));
        }
    }

    contributions.sort_by_key(|b| std::cmp::Reverse(b.1));

    let current_floor = get_guild_floor(db, &guild);
    let mut r = format!("{}\n═══ 🏆 秘境排行 ═══\n", prefix);
    r.push_str(&format!("🏰 公会: {} | 📍 第{}层\n\n", guild, current_floor));

    if contributions.is_empty() {
        r.push_str("📭 暂无成员探索记录\n");
        return r;
    }

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, contrib)) in contributions.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let nick = db.read_basic(uid, ITEM_NAME);
        let highlight = if uid == user_id { " ← 我" } else { "" };
        r.push_str(&format!(
            "{} {}. {} — {} 伤害{}\n",
            medal,
            i + 1,
            nick,
            contrib,
            highlight
        ));
    }

    if let Some(pos) = contributions.iter().position(|(uid, _)| uid == user_id) {
        r.push_str(&format!(
            "\n📍 您的排名: 第{}名 (共{}人)\n",
            pos + 1,
            contributions.len()
        ));
    } else {
        r.push_str("\n📍 您尚未参与探索\n");
    }

    r
}

/// 秘境奖励 — 查看各层奖励预览
pub fn cmd_realm_rewards(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut r = format!("{}\n═══ 🎁 秘境奖励预览 ═══\n\n", prefix);

    for f in REALM_FLOORS {
        let stars = if f.floor <= 4 {
            "⭐"
        } else if f.floor <= 8 {
            "⭐⭐"
        } else {
            "⭐⭐⭐"
        };
        r.push_str(&format!("{}. {} [{}] {}\n", f.floor, f.name, f.monster_name, stars));
        r.push_str(&format!(
            "   💰 {}金 | ✨ {}经验",
            format_gold(f.gold_reward),
            f.exp_reward
        ));
        if f.diamond_reward > 0 {
            r.push_str(&format!(" | 💎 {}钻", f.diamond_reward));
        }
        r.push_str(&format!(
            "\n   🎁 稀有掉落: {} ({}%)\n\n",
            f.rare_drop_item,
            f.rare_drop_rate / 100
        ));
    }

    r.push_str("💡 层数越高奖励越丰厚，全公会协力推进！\n");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_count() {
        assert_eq!(REALM_FLOORS.len(), 12);
    }

    #[test]
    fn test_floors_sorted() {
        for i in 1..REALM_FLOORS.len() {
            assert!(REALM_FLOORS[i].floor > REALM_FLOORS[i - 1].floor);
        }
    }

    #[test]
    fn test_floor_hp_escalates() {
        for i in 1..REALM_FLOORS.len() {
            assert!(REALM_FLOORS[i].monster_hp > REALM_FLOORS[i - 1].monster_hp);
        }
    }

    #[test]
    fn test_rewards_escalate() {
        for i in 1..REALM_FLOORS.len() {
            assert!(REALM_FLOORS[i].gold_reward >= REALM_FLOORS[i - 1].gold_reward);
            assert!(REALM_FLOORS[i].exp_reward >= REALM_FLOORS[i - 1].exp_reward);
        }
    }

    #[test]
    fn test_floor_names_unique() {
        let mut names: Vec<&str> = REALM_FLOORS.iter().map(|f| f.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), REALM_FLOORS.len());
    }

    #[test]
    fn test_monster_names_unique() {
        let mut names: Vec<&str> = REALM_FLOORS.iter().map(|f| f.monster_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), REALM_FLOORS.len());
    }

    #[test]
    fn test_rare_drop_items_unique() {
        let mut items: Vec<&str> = REALM_FLOORS.iter().map(|f| f.rare_drop_item).collect();
        items.sort();
        items.dedup();
        assert_eq!(items.len(), REALM_FLOORS.len());
    }

    #[test]
    fn test_drop_rates_valid() {
        for f in REALM_FLOORS {
            assert!(f.rare_drop_rate > 0);
            assert!(f.rare_drop_rate <= 10000);
        }
    }

    #[test]
    fn test_djb2_deterministic() {
        let h1 = djb2_hash("test_user");
        let h2 = djb2_hash("test_user");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_djb2_different() {
        let h1 = djb2_hash("user_a");
        let h2 = djb2_hash("user_b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 10);
        assert!(bar.contains("█"));
        assert!(!bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 10);
        assert!(!bar.contains("█"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 10);
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_constants_valid() {
        assert!(DAILY_EXPLORE_LIMIT > 0);
        assert!(EXPLORE_COOLDOWN > 0);
        assert!(MAX_DAMAGE_PER_EXPLORE > 0);
    }

    #[test]
    fn test_diamond_rewards_non_negative() {
        for f in REALM_FLOORS {
            assert!(f.diamond_reward >= 0);
        }
    }

    #[test]
    fn test_floor_monster_ad_escalates() {
        for i in 1..REALM_FLOORS.len() {
            assert!(REALM_FLOORS[i].monster_ad >= REALM_FLOORS[i - 1].monster_ad);
        }
    }

    #[test]
    fn test_format_gold_large() {
        assert_eq!(format_gold(50000), "5万");
    }

    #[test]
    fn test_format_gold_small() {
        assert_eq!(format_gold(500), "500");
    }

    #[test]
    fn test_format_gold_zero() {
        assert_eq!(format_gold(0), "0");
    }

    #[test]
    fn test_is_leap() {
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(is_leap(2024));
        assert!(!is_leap(2023));
    }
}

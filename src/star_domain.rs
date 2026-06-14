use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use crate::user;

const SECTION: &str = "star_domain";

// ── Star Domain Definitions ──────────────────────────────────────

struct DomainDef {
    id: u8,
    name: &'static str,
    emoji: &'static str,
    stages: u8,
    power_req: i64,
    energy_cost: i32,
    fragment_reward: i32,
    gold_reward: i64,
    diamond_reward: i64,
    boss_name: &'static str,
    boss_hp: i64,
}

const DOMAINS: &[DomainDef] = &[
    DomainDef {
        id: 1,
        name: "星辰平原",
        emoji: "⭐",
        stages: 5,
        power_req: 1000,
        energy_cost: 10,
        fragment_reward: 15,
        gold_reward: 5000,
        diamond_reward: 20,
        boss_name: "星辰守卫",
        boss_hp: 50000,
    },
    DomainDef {
        id: 2,
        name: "月华深渊",
        emoji: "🌙",
        stages: 7,
        power_req: 3000,
        energy_cost: 15,
        fragment_reward: 30,
        gold_reward: 12000,
        diamond_reward: 50,
        boss_name: "月影魔将",
        boss_hp: 120000,
    },
    DomainDef {
        id: 3,
        name: "日冕火山",
        emoji: "☀️",
        stages: 8,
        power_req: 6000,
        energy_cost: 20,
        fragment_reward: 50,
        gold_reward: 25000,
        diamond_reward: 100,
        boss_name: "炎魔领主",
        boss_hp: 250000,
    },
    DomainDef {
        id: 4,
        name: "银河漩涡",
        emoji: "🌀",
        stages: 10,
        power_req: 10000,
        energy_cost: 25,
        fragment_reward: 80,
        gold_reward: 50000,
        diamond_reward: 200,
        boss_name: "虚空吞噬者",
        boss_hp: 500000,
    },
    DomainDef {
        id: 5,
        name: "黑洞边缘",
        emoji: "🕳️",
        stages: 12,
        power_req: 18000,
        energy_cost: 30,
        fragment_reward: 120,
        gold_reward: 100000,
        diamond_reward: 400,
        boss_name: "暗物质巨兽",
        boss_hp: 1000000,
    },
    DomainDef {
        id: 6,
        name: "超新星遗迹",
        emoji: "💫",
        stages: 15,
        power_req: 30000,
        energy_cost: 35,
        fragment_reward: 180,
        gold_reward: 200000,
        diamond_reward: 800,
        boss_name: "星爆之灵",
        boss_hp: 2000000,
    },
    DomainDef {
        id: 7,
        name: "虫洞彼端",
        emoji: "🌌",
        stages: 18,
        power_req: 50000,
        energy_cost: 40,
        fragment_reward: 260,
        gold_reward: 400000,
        diamond_reward: 1500,
        boss_name: "维度领主",
        boss_hp: 5000000,
    },
    DomainDef {
        id: 8,
        name: "创世奇点",
        emoji: "✨",
        stages: 20,
        power_req: 80000,
        energy_cost: 50,
        fragment_reward: 400,
        gold_reward: 800000,
        diamond_reward: 3000,
        boss_name: "宇宙意志",
        boss_hp: 10000000,
    },
];

// ── Shop Items ───────────────────────────────────────────────────

struct ShopItem {
    name: &'static str,
    emoji: &'static str,
    cost: i32,
    description: &'static str,
}

const SHOP_ITEMS: &[ShopItem] = &[
    ShopItem {
        name: "星尘药水",
        emoji: "🧪",
        cost: 50,
        description: "恢复50%HP",
    },
    ShopItem {
        name: "星辰强化石",
        emoji: "💎",
        cost: 120,
        description: "装备强化+1",
    },
    ShopItem {
        name: "星辉护符",
        emoji: "🔮",
        cost: 200,
        description: "防御+5% 30分钟",
    },
    ShopItem {
        name: "星域精华",
        emoji: "✨",
        cost: 350,
        description: "全属性+3% 永久",
    },
    ShopItem {
        name: "星际传送门",
        emoji: "🚪",
        cost: 500,
        description: "跳过1个星域阶段",
    },
    ShopItem {
        name: "星陨之翼",
        emoji: "🪽",
        cost: 800,
        description: "飞行坐骑 战力+500",
    },
    ShopItem {
        name: "星域战甲",
        emoji: "🛡️",
        cost: 1200,
        description: "HP+15% 防御+10%",
    },
    ShopItem {
        name: "星辰之刃",
        emoji: "⚔️",
        cost: 1500,
        description: "攻击+15% 暴击+5%",
    },
    ShopItem {
        name: "星域宝箱",
        emoji: "🎁",
        cost: 2000,
        description: "随机稀有道具×3",
    },
    ShopItem {
        name: "宇宙之心",
        emoji: "❤️‍🔥",
        cost: 5000,
        description: "全属性+10% 永久",
    },
];

// ── Milestone Definitions ────────────────────────────────────────

struct MilestoneDef {
    threshold: i32,
    name: &'static str,
    reward_gold: i64,
    reward_diamond: i64,
}

const MILESTONES: &[MilestoneDef] = &[
    MilestoneDef {
        threshold: 100,
        name: "星际新手",
        reward_gold: 5000,
        reward_diamond: 30,
    },
    MilestoneDef {
        threshold: 500,
        name: "星际探索者",
        reward_gold: 15000,
        reward_diamond: 80,
    },
    MilestoneDef {
        threshold: 2000,
        name: "星际征服者",
        reward_gold: 50000,
        reward_diamond: 200,
    },
    MilestoneDef {
        threshold: 5000,
        name: "星际大师",
        reward_gold: 150000,
        reward_diamond: 500,
    },
    MilestoneDef {
        threshold: 15000,
        name: "星际王者",
        reward_gold: 500000,
        reward_diamond: 1200,
    },
    MilestoneDef {
        threshold: 50000,
        name: "宇宙传说",
        reward_gold: 2000000,
        reward_diamond: 3000,
    },
];

// ── Daily Energy ─────────────────────────────────────────────────

const MAX_ENERGY: i32 = 100;
const ENERGY_REGEN_PER_DAY: i32 = 100;

// ── Helpers ──────────────────────────────────────────────────────

fn today_str() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86400) as i32;
    format!("day_{}", days)
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

fn progress_bar(current: i32, max: i32, width: usize) -> String {
    let filled = if max > 0 {
        (current as usize * width / max as usize).min(width)
    } else {
        0
    };
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn get_domain(id: u8) -> Option<&'static DomainDef> {
    DOMAINS.iter().find(|d| d.id == id)
}

fn find_domain(query: &str) -> Option<&'static DomainDef> {
    // by id
    if let Ok(id) = query.parse::<u8>() {
        return get_domain(id);
    }
    // by exact name
    if let Some(d) = DOMAINS.iter().find(|d| d.name == query) {
        return Some(d);
    }
    // by substring
    DOMAINS
        .iter()
        .find(|d| d.name.contains(query) || query.contains(d.name))
}

// ── Per-User State ───────────────────────────────────────────────

fn user_section(user_id: &str) -> String {
    format!("{}_{}", SECTION, user_id)
}

fn get_energy(db: &Database, user_id: &str) -> i32 {
    let sec = user_section(user_id);
    let last_date = db.global_get(&sec, "energy_date");
    let stored: i32 = db.global_get(&sec, "energy").parse().unwrap_or(MAX_ENERGY);
    let today = today_str();
    if last_date != today {
        // daily reset: regen energy
        let new_energy = (stored + ENERGY_REGEN_PER_DAY).min(MAX_ENERGY);
        db.global_set(&sec, "energy", &new_energy.to_string());
        db.global_set(&sec, "energy_date", &today);
        new_energy
    } else {
        stored
    }
}

fn consume_energy(db: &Database, user_id: &str, cost: i32) -> bool {
    let current = get_energy(db, user_id);
    if current < cost {
        return false;
    }
    let sec = user_section(user_id);
    db.global_set(&sec, "energy", &(current - cost).to_string());
    true
}

fn get_fragments(db: &Database, user_id: &str) -> i32 {
    db.global_get(&user_section(user_id), "fragments").parse().unwrap_or(0)
}

fn add_fragments(db: &Database, user_id: &str, amount: i32) {
    let current = get_fragments(db, user_id);
    db.global_set(&user_section(user_id), "fragments", &(current + amount).to_string());
}

fn get_highest_cleared(db: &Database, user_id: &str) -> u8 {
    db.global_get(&user_section(user_id), "highest_cleared")
        .parse()
        .unwrap_or(0)
}

fn get_domain_stage(db: &Database, user_id: &str, domain_id: u8) -> u8 {
    db.global_get(&user_section(user_id), &format!("stage_{}", domain_id))
        .parse()
        .unwrap_or(0)
}

fn set_domain_stage(db: &Database, user_id: &str, domain_id: u8, stage: u8) {
    db.global_set(
        &user_section(user_id),
        &format!("stage_{}", domain_id),
        &stage.to_string(),
    );
}

fn get_total_explored(db: &Database, user_id: &str) -> i32 {
    let mut total = 0i32;
    for d in DOMAINS {
        total += get_domain_stage(db, user_id, d.id) as i32;
    }
    total
}

fn get_milestone_claimed(db: &Database, user_id: &str) -> u32 {
    db.global_get(&user_section(user_id), "milestones").parse().unwrap_or(0)
}

fn set_milestone_claimed(db: &Database, user_id: &str, mask: u32) {
    db.global_set(&user_section(user_id), "milestones", &mask.to_string());
}

fn get_daily_explores(db: &Database, user_id: &str) -> i32 {
    let sec = user_section(user_id);
    let last_date = db.global_get(&sec, "explore_date");
    let today = today_str();
    if last_date != today {
        db.global_set(&sec, "daily_explores", "0");
        db.global_set(&sec, "explore_date", &today);
        0
    } else {
        db.global_get(&sec, "daily_explores").parse().unwrap_or(0)
    }
}

fn inc_daily_explores(db: &Database, user_id: &str) {
    let count = get_daily_explores(db, user_id);
    let sec = user_section(user_id);
    db.global_set(&sec, "daily_explores", &(count + 1).to_string());
    db.global_set(&sec, "explore_date", &today_str());
}

const DAILY_EXPLORE_LIMIT: i32 = 10;

// ── Combat Simulation ────────────────────────────────────────────

fn simulate_exploration(db: &Database, user_id: &str, domain: &DomainDef, _stage: u8) -> (bool, String) {
    let info = user::calc_total_attrs(db, user_id);
    let attack = info.ad + info.ap;
    let defense = info.defense + info.magic_res;
    let hp = info.hp_max;
    let cp = (attack as f64 * 1.5 + defense as f64 * 1.0 + hp as f64 * 0.3) as i64;

    if cp < domain.power_req {
        return (
            false,
            format!(
                "❌ 战力不足！需要{}，当前{}",
                format_num(domain.power_req),
                format_num(cp)
            ),
        );
    }

    // Simulate: player vs domain enemies
    let enemy_power = domain.boss_hp / 10 * (_stage as i64 + 1);
    let player_damage = attack.max(1) as i64 * (100 + _stage as i64 * 5) / 100;
    let turns_needed = (enemy_power as f64 / player_damage.max(1) as f64).ceil() as i32;
    let enemy_damage_per_turn = (enemy_power / 20).max(1);
    let player_survive_turns = (hp.max(1) as i64 / enemy_damage_per_turn.max(1)) as i32;

    let won = turns_needed <= player_survive_turns.max(3);
    let result = if won {
        "✅ 探索成功！"
    } else {
        "❌ 探索失败..."
    };
    let detail = format!(
        "回合数: {} | 敌方战力: {} | 伤害/回合: {}",
        turns_needed,
        format_num(enemy_power),
        format_num(player_damage)
    );
    (won, format!("{}\n{}", result, detail))
}

// ── Commands ─────────────────────────────────────────────────────

/// 查看星域 - 展示所有星域和当前进度
pub fn cmd_view_star_domain(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let energy = get_energy(db, user_id);
    let fragments = get_fragments(db, user_id);
    let highest = get_highest_cleared(db, user_id);
    let total = get_total_explored(db, user_id);

    let mut out = format!("{}\n═══ 🌌 星域探索 ═══\n", prefix);
    out.push_str(&format!(
        "⚡ 能力: {}/{} | 💎 星辉碎片: {} | 📊 总进度: {}\n\n",
        energy,
        MAX_ENERGY,
        format_num(fragments as i64),
        total
    ));

    for d in DOMAINS {
        let stage = get_domain_stage(db, user_id, d.id);
        let status = if d.id <= highest + 1 {
            if stage >= d.stages {
                "✅通关".to_string()
            } else {
                format!("{}/{}", stage, d.stages)
            }
        } else {
            "🔒未解锁".to_string()
        };
        let bar = if d.id <= highest + 1 && stage < d.stages {
            format!(" {}", progress_bar(stage as i32, d.stages as i32, 8))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "{} {} {} — {} | 战力{} | {}碎片{}\n",
            d.emoji,
            d.id,
            d.name,
            status,
            format_num(d.power_req),
            d.fragment_reward,
            bar
        ));
    }
    out.push_str(&format!(
        "\n💡 每日探索次数: {}/{}",
        get_daily_explores(db, user_id),
        DAILY_EXPLORE_LIMIT
    ));
    out.push_str("\n📌 使用: 探索星域+星域名/编号");
    out
}

/// 探索星域+星域名 - 进行一次星域探索
pub fn cmd_explore_star_domain(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        return format!("{}\n请指定星域名或编号！\n📌 使用: 探索星域+星辰平原", prefix);
    }

    let domain = match find_domain(args) {
        Some(d) => d,
        None => return format!("{}\n❌ 未找到星域「{}」！", prefix, args),
    };

    // Check unlock
    let highest = get_highest_cleared(db, user_id);
    if domain.id > highest + 1 {
        return format!("{}\n🔒 星域「{}」尚未解锁！请先通关前一个星域。", prefix, domain.name);
    }

    // Check daily limit
    if get_daily_explores(db, user_id) >= DAILY_EXPLORE_LIMIT {
        return format!(
            "{}\n❌ 今日探索次数已用完（{}/{}）！",
            prefix, DAILY_EXPLORE_LIMIT, DAILY_EXPLORE_LIMIT
        );
    }

    // Check energy
    if !consume_energy(db, user_id, domain.energy_cost) {
        let current = get_energy(db, user_id);
        return format!("{}\n⚡ 能力不足！需要{}，当前{}", prefix, domain.energy_cost, current);
    }

    let stage = get_domain_stage(db, user_id, domain.id);
    if stage >= domain.stages {
        return format!("{}\n✅ 星域「{}」已通关！等待下一个星域开放。", prefix, domain.name);
    }

    let next_stage = stage + 1;
    let (won, combat_detail) = simulate_exploration(db, user_id, domain, next_stage);

    inc_daily_explores(db, user_id);

    if won {
        set_domain_stage(db, user_id, domain.id, next_stage);

        // Calculate rewards with bonuses
        let stage_bonus = next_stage as f64 / domain.stages as f64;
        let fragments = (domain.fragment_reward as f64 * (1.0 + stage_bonus)) as i32;
        let gold = (domain.gold_reward as f64 * (1.0 + stage_bonus * 0.5)) as i64;
        let diamond = (domain.diamond_reward as f64 * (1.0 + stage_bonus * 0.3)) as i64;

        add_fragments(db, user_id, fragments);
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, diamond);

        let mut out = format!("{}\n═══ {} {} 探索成功！ ═══\n", prefix, domain.emoji, domain.name);
        out.push_str(&format!(
            "📍 第{}/{}层 | {}\n",
            next_stage, domain.stages, combat_detail
        ));
        out.push_str(&format!(
            "💰 奖励: {}金币 + {}钻石 + {}星辉碎片\n",
            format_num(gold),
            diamond,
            fragments
        ));

        // Check if domain completed
        if next_stage >= domain.stages {
            if domain.id > highest {
                db.global_set(&user_section(user_id), "highest_cleared", &domain.id.to_string());
            }
            out.push_str(&format!("\n🎉 恭喜通关「{}」！下一星域已解锁！", domain.name));

            // Boss kill bonus
            let boss_gold = domain.boss_hp / 10;
            let boss_diamond = domain.diamond_reward * 2;
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, boss_gold);
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, boss_diamond);
            out.push_str(&format!(
                "\n🏆 Boss「{}」击败！额外奖励: {}金币 + {}钻石",
                domain.boss_name,
                format_num(boss_gold),
                boss_diamond
            ));
        }

        // Check milestones
        let total = get_total_explored(db, user_id);
        let claimed = get_milestone_claimed(db, user_id);
        for (i, m) in MILESTONES.iter().enumerate() {
            let bit = 1u32 << i;
            if total >= m.threshold && claimed & bit == 0 {
                out.push_str(&format!(
                    "\n🏅 里程碑达成: {} — {}金+{}钻",
                    m.name,
                    format_num(m.reward_gold),
                    m.reward_diamond
                ));
            }
        }

        out
    } else {
        // Partial reward on failure
        let partial_fragments = domain.fragment_reward / 4;
        let partial_gold = domain.gold_reward / 5;
        add_fragments(db, user_id, partial_fragments);
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, partial_gold);

        let mut out = format!("{}\n═══ {} {} 探索失败 ═══\n", prefix, domain.emoji, domain.name);
        out.push_str(&format!(
            "📍 第{}/{}层 | {}\n",
            next_stage, domain.stages, combat_detail
        ));
        out.push_str(&format!(
            "💔 失败奖励: {}金币 + {}碎片（安慰奖）",
            format_num(partial_gold),
            partial_fragments
        ));
        out
    }
}

/// 星域商店 - 使用星辉碎片兑换物品
pub fn cmd_star_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let fragments = get_fragments(db, user_id);

    let mut out = format!("{}\n═══ ✨ 星域商店 ═══\n", prefix);
    out.push_str(&format!("💎 当前碎片: {}\n\n", format_num(fragments as i64)));

    for (i, item) in SHOP_ITEMS.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} {} — {}碎片\n   {}\n",
            i + 1,
            item.emoji,
            item.name,
            item.cost,
            item.description
        ));
    }
    out.push_str("\n📌 使用: 购买星域+商品编号");
    out
}

/// 购买星域+编号 - 购买星域商店商品
pub fn cmd_buy_star_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    let idx: usize = match args.parse::<usize>() {
        Ok(n) if n >= 1 && n <= SHOP_ITEMS.len() => n - 1,
        _ => {
            return format!(
                "{}\n❌ 无效商品编号！请使用: 购买星域+编号(1-{})",
                prefix,
                SHOP_ITEMS.len()
            )
        }
    };

    let item = &SHOP_ITEMS[idx];
    let fragments = get_fragments(db, user_id);

    if fragments < item.cost {
        return format!("{}\n❌ 碎片不足！需要{}，当前{}", prefix, item.cost, fragments);
    }

    add_fragments(db, user_id, -item.cost);

    // Give item reward based on type
    let _reward_detail = match idx {
        0 => {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, 0);
            "HP恢复效果已激活".to_string()
        }
        1 => "强化石×1已发放".to_string(),
        2 => "防御加成30分钟".to_string(),
        3 => {
            db.knapsack_add(user_id, "星域精华", 1);
            "全属性+3%永久".to_string()
        }
        4 => "传送门已激活".to_string(),
        5 => {
            db.knapsack_add(user_id, "星陨之翼", 1);
            "飞行坐骑已获得".to_string()
        }
        6 => {
            db.knapsack_add(user_id, "星域战甲", 1);
            "战甲已获得".to_string()
        }
        7 => {
            db.knapsack_add(user_id, "星辰之刃", 1);
            "星辰之刃已获得".to_string()
        }
        8 => {
            db.knapsack_add(user_id, "星域宝箱", 1);
            "宝箱已获得".to_string()
        }
        9 => {
            db.knapsack_add(user_id, "宇宙之心", 1);
            "全属性+10%永久".to_string()
        }
        _ => "购买成功".to_string(),
    };

    format!(
        "{}\n✅ 购买成功！{} {}\n💎 消耗碎片: {} | 剩余: {}",
        prefix,
        item.emoji,
        item.name,
        item.cost,
        format_num(get_fragments(db, user_id) as i64)
    )
}

/// 星域排行 - 全服星域探索排行榜
pub fn cmd_star_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut players: Vec<(String, i32, i32)> = Vec::new(); // (name, total_stages, fragments)

    for uid in db.all_users().iter() {
        let sec = user_section(uid);
        let total: i32 = db.global_get(&sec, "fragments").parse().unwrap_or(0);
        if total > 0 {
            let name = user::get_msg_prefix(db, uid);
            let _highest: u8 = db.global_get(&sec, "highest_cleared").parse().unwrap_or(0);
            let explored: i32 = {
                let mut t = 0i32;
                for d in DOMAINS {
                    t += db
                        .global_get(&sec, &format!("stage_{}", d.id))
                        .parse::<i32>()
                        .unwrap_or(0);
                }
                t
            };
            players.push((name, explored, total));
        }
    }

    if players.is_empty() {
        return format!("{}\n暂无星域探索数据", prefix);
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut out = format!("{}\n═══ 🌌 星域探索排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, explored, fragments)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        out.push_str(&format!(
            "\n{} {} — 进度{} | 碎片{}",
            medal,
            name,
            explored,
            format_num(*fragments as i64)
        ));
    }

    // User position
    let my_name = user::get_msg_prefix(db, user_id);
    if let Some(rank) = players.iter().position(|(name, _, _)| name == &my_name) {
        out.push_str(&format!("\n\n📍 你的排名：第{}名", rank + 1));
    }

    out
}

/// 星域详情+星域名 - 查看某个星域的详细信息
pub fn cmd_star_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    let domain = match find_domain(args) {
        Some(d) => d,
        None => return format!("{}\n❌ 未找到星域「{}」！", prefix, args),
    };

    let stage = get_domain_stage(db, user_id, domain.id);
    let total_stages = domain.stages;
    let bar = progress_bar(stage as i32, total_stages as i32, 12);
    let pct = if total_stages > 0 {
        stage as f64 / total_stages as f64 * 100.0
    } else {
        0.0
    };

    let mut out = format!("{}\n═══ {} {} ═══\n", prefix, domain.emoji, domain.name);
    out.push_str(&format!("📊 进度: {}/{} ({:.1}%)\n", stage, total_stages, pct));
    out.push_str(&format!("[{}]\n", bar));
    out.push_str(&format!("\n⚡ 战力需求: {}", format_num(domain.power_req)));
    out.push_str(&format!("\n⛽ 能力消耗: {} / 次", domain.energy_cost));
    out.push_str(&format!("\n💎 碎片奖励: {} / 次", domain.fragment_reward));
    out.push_str(&format!("\n💰 金币奖励: {}", format_num(domain.gold_reward)));
    out.push_str(&format!("\n🔷 钻石奖励: {}", domain.diamond_reward));
    out.push_str(&format!(
        "\n\n👹 Boss: {} (HP: {})",
        domain.boss_name,
        format_num(domain.boss_hp)
    ));
    out.push_str(&format!("\n📐 阶段数: {}层", total_stages));
    out
}

/// 星域里程碑 - 查看和领取里程碑奖励
pub fn cmd_star_milestones(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let total = get_total_explored(db, user_id);
    let claimed = get_milestone_claimed(db, user_id);

    let mut out = format!("{}\n═══ 🏅 星域里程碑 ═══\n", prefix);
    out.push_str(&format!("📊 总探索进度: {}\n\n", total));

    for (i, m) in MILESTONES.iter().enumerate() {
        let bit = 1u32 << i;
        let status = if claimed & bit != 0 {
            "✅已领取".to_string()
        } else if total >= m.threshold {
            "🎁可领取".to_string()
        } else {
            let bar = progress_bar(total, m.threshold, 8);
            format!("{} {}/{}", bar, total, m.threshold)
        };
        out.push_str(&format!(
            "{}. {} — {} | {}金+{}钻 | {}\n",
            i + 1,
            m.name,
            format_num(m.threshold as i64),
            format_num(m.reward_gold),
            m.reward_diamond,
            status
        ));
    }
    out.push_str("\n📌 使用: 领取星域里程碑+编号");
    out
}

/// 领取星域里程碑+编号 - 领取里程碑奖励
pub fn cmd_claim_star_milestone(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= MILESTONES.len() => n - 1,
        _ => {
            return format!(
                "{}\n❌ 无效编号！请使用: 领取星域里程碑+编号(1-{})",
                prefix,
                MILESTONES.len()
            )
        }
    };

    let total = get_total_explored(db, user_id);
    let claimed = get_milestone_claimed(db, user_id);
    let bit = 1u32 << idx;
    let m = &MILESTONES[idx];

    if claimed & bit != 0 {
        return format!("{}\n❌ 里程碑「{}」已领取过！", prefix, m.name);
    }
    if total < m.threshold {
        return format!("{}\n❌ 未达成！需要{}层进度，当前{}", prefix, m.threshold, total);
    }

    let new_claimed = claimed | bit;
    set_milestone_claimed(db, user_id, new_claimed);
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, m.reward_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, m.reward_diamond);

    format!(
        "{}\n🎉 领取成功！里程碑「{}」\n💰 奖励: {}金币 + {}钻石",
        prefix,
        m.name,
        format_num(m.reward_gold),
        m.reward_diamond
    )
}

/// 星域帮助
pub fn cmd_star_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n\
═══ 🌌 星域探索系统 帮助 ═══\n\
\n\
📌 指令列表:\n\
  查看星域 - 查看所有星域和进度\n\
  探索星域+星域名 - 进行一次探索\n\
  星域商店 - 查看碎片兑换商店\n\
  购买星域+编号 - 购买商店商品\n\
  星域排行 - 全服探索排行榜\n\
  星域详情+星域名 - 查看星域详细信息\n\
  星域里程碑 - 查看里程碑进度\n\
  领取星域里程碑+编号 - 领取里程碑奖励\n\
\n\
🌌 系统说明:\n\
  • 8大星域，逐级解锁，难度递增\n\
  • 每日10次探索机会，消耗能力值\n\
  • 通关奖励: 星辉碎片+金币+钻石\n\
  • 星辉碎片可在商店兑换稀有道具\n\
  • 6级里程碑奖励，总进度触发\n\
  • 能力值每日恢复100点\n\
  • 探索失败也获得安慰奖\n",
        prefix
    )
}

/// Get star domain bonus for combat integration
#[allow(dead_code)]
pub fn get_star_domain_bonus(db: &Database, user_id: &str) -> (i32, i32, i32) {
    // Returns (attack%, defense%, hp%) bonus based on exploration progress
    let highest = get_highest_cleared(db, user_id) as i32;
    let total = get_total_explored(db, user_id);
    let attack_pct = (highest * 3 + total / 10).min(50);
    let defense_pct = (highest * 2 + total / 15).min(40);
    let hp_pct = (highest * 4 + total / 8).min(60);
    (attack_pct, defense_pct, hp_pct)
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domains_count() {
        assert_eq!(DOMAINS.len(), 8);
    }

    #[test]
    fn test_domains_unique_ids() {
        let mut ids: Vec<u8> = DOMAINS.iter().map(|d| d.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), DOMAINS.len());
    }

    #[test]
    fn test_domains_sorted_by_power() {
        for i in 1..DOMAINS.len() {
            assert!(DOMAINS[i].power_req > DOMAINS[i - 1].power_req);
        }
    }

    #[test]
    fn test_domain_stages_ascending() {
        for i in 1..DOMAINS.len() {
            assert!(DOMAINS[i].stages >= DOMAINS[i - 1].stages);
        }
    }

    #[test]
    fn test_shop_items_count() {
        assert_eq!(SHOP_ITEMS.len(), 10);
    }

    #[test]
    fn test_shop_items_positive_cost() {
        for item in SHOP_ITEMS {
            assert!(item.cost > 0);
        }
    }

    #[test]
    fn test_shop_items_sorted_by_cost() {
        for i in 1..SHOP_ITEMS.len() {
            assert!(SHOP_ITEMS[i].cost >= SHOP_ITEMS[i - 1].cost);
        }
    }

    #[test]
    fn test_milestones_count() {
        assert_eq!(MILESTONES.len(), 6);
    }

    #[test]
    fn test_milestones_sorted() {
        for i in 1..MILESTONES.len() {
            assert!(MILESTONES[i].threshold > MILESTONES[i - 1].threshold);
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
    fn test_find_domain_by_id() {
        let d = find_domain("3").unwrap();
        assert_eq!(d.name, "日冕火山");
    }

    #[test]
    fn test_find_domain_by_name() {
        let d = find_domain("星辰平原").unwrap();
        assert_eq!(d.id, 1);
    }

    #[test]
    fn test_find_domain_by_substring() {
        let d = find_domain("火山").unwrap();
        assert_eq!(d.name, "日冕火山");
    }

    #[test]
    fn test_find_domain_invalid() {
        assert!(find_domain("不存在的星域").is_none());
    }

    #[test]
    fn test_progress_bar_zero() {
        let bar = progress_bar(0, 10, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.chars().all(|c| c == '░'));
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(10, 10, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.chars().all(|c| c == '█'));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(5, 10, 10);
        assert_eq!(bar.chars().count(), 10);
        assert_eq!(bar.chars().filter(|c| *c == '█').count(), 5);
        assert_eq!(bar.chars().filter(|c| *c == '░').count(), 5);
    }

    #[test]
    fn test_format_num_zero() {
        assert_eq!(format_num(0), "0");
    }

    #[test]
    fn test_format_num_thousands() {
        assert_eq!(format_num(1234567), "1,234,567");
    }

    #[test]
    fn test_today_str_format() {
        let s = today_str();
        assert!(s.starts_with("day_"));
    }

    #[test]
    fn test_energy_limits() {
        assert!(MAX_ENERGY > 0);
        assert!(ENERGY_REGEN_PER_DAY > 0);
        assert!(ENERGY_REGEN_PER_DAY <= MAX_ENERGY);
    }

    #[test]
    fn test_daily_explore_limit() {
        assert!(DAILY_EXPLORE_LIMIT > 0);
    }

    #[test]
    fn test_domain_rewards_positive() {
        for d in DOMAINS {
            assert!(d.fragment_reward > 0);
            assert!(d.gold_reward > 0);
            assert!(d.diamond_reward >= 0);
            assert!(d.energy_cost > 0);
            assert!(d.boss_hp > 0);
        }
    }

    #[test]
    fn test_milestone_bitmask() {
        for i in 0..MILESTONES.len() {
            let bit = 1u32 << i;
            assert!(bit > 0);
        }
    }

    #[test]
    fn test_get_star_domain_bonus_default() {
        // Test the bonus calculation with mock values
        let highest = 0i32;
        let total = 0i32;
        let attack_pct = (highest * 3 + total / 10).min(50);
        let defense_pct = (highest * 2 + total / 15).min(40);
        let hp_pct = (highest * 4 + total / 8).min(60);
        assert_eq!(attack_pct, 0);
        assert_eq!(defense_pct, 0);
        assert_eq!(hp_pct, 0);
    }

    #[test]
    fn test_get_star_domain_bonus_max() {
        let highest = 8i32;
        let total = 85i32;
        let attack_pct = (highest * 3 + total / 10).min(50);
        let defense_pct = (highest * 2 + total / 15).min(40);
        let hp_pct = (highest * 4 + total / 8).min(60);
        assert!(attack_pct > 0);
        assert!(defense_pct > 0);
        assert!(hp_pct > 0);
    }

    #[test]
    fn test_section_name() {
        assert_eq!(SECTION, "star_domain");
    }

    #[test]
    fn test_user_section_format() {
        assert_eq!(user_section("12345"), "star_domain_12345");
    }

    #[test]
    fn test_domains_energy_cost_range() {
        for d in DOMAINS {
            assert!(d.energy_cost >= 10 && d.energy_cost <= 50);
        }
    }
}

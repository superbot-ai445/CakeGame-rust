/// CakeGame 怪物攻城系统 (Monster Siege System)
/// 全服协作防守攻城怪物，5波进攻，每波BOSS递增难度
/// 根据贡献排名发放奖励
/// 数据存储: Global 表 SECTION='monster_siege' / 'monster_siege_daily'
use crate::core::*;
use crate::db::Database;
use crate::user;
use crate::vip;
use chrono::{Local, Timelike};
use rand::Rng;

// ==================== 攻城怪物定义 ====================

#[allow(dead_code)]
struct SiegeMonster {
    wave: i32,
    name: &'static str,
    emoji: &'static str,
    max_hp: i64,
    attack: i32,
    defense: i32,
    announcement: &'static str,
}

const SIEGE_MONSTERS: &[SiegeMonster] = &[
    SiegeMonster {
        wave: 1,
        name: "狂暴狼群",
        emoji: "🐺",
        max_hp: 50_000,
        attack: 300,
        defense: 100,
        announcement: "狼嚎四起，城墙告急！",
    },
    SiegeMonster {
        wave: 2,
        name: "巨型蜥蜴人",
        emoji: "🦎",
        max_hp: 100_000,
        attack: 500,
        defense: 200,
        announcement: "蜥蜴大军压境！",
    },
    SiegeMonster {
        wave: 3,
        name: "骷髅军团",
        emoji: "💀",
        max_hp: 200_000,
        attack: 800,
        defense: 350,
        announcement: "亡灵大军来袭！",
    },
    SiegeMonster {
        wave: 4,
        name: "攻城巨龙",
        emoji: "🐉",
        max_hp: 400_000,
        attack: 1200,
        defense: 500,
        announcement: "巨龙俯冲城门！",
    },
    SiegeMonster {
        wave: 5,
        name: "暗黑魔王",
        emoji: "👹",
        max_hp: 800_000,
        attack: 2000,
        defense: 800,
        announcement: "最终BOSS降临！",
    },
];

/// 每日攻击上限
const MAX_ATTACKS_PER_SIEGE: i32 = 5;

/// 攻城触发时间（小时）
const SIEGE_HOURS: &[u32] = &[0, 4, 8, 12, 16, 20];

/// 奖励定义
#[allow(dead_code)]
struct RewardTier {
    min_rank: usize,
    max_rank: usize,
    gold: i64,
    diamond: i32,
    item: &'static str,
}

const REWARD_TIERS: &[RewardTier] = &[
    RewardTier {
        min_rank: 1,
        max_rank: 1,
        gold: 50000,
        diamond: 500,
        item: "暗黑魔王碎片",
    },
    RewardTier {
        min_rank: 2,
        max_rank: 3,
        gold: 30000,
        diamond: 300,
        item: "攻城精魄",
    },
    RewardTier {
        min_rank: 4,
        max_rank: 10,
        gold: 15000,
        diamond: 150,
        item: "攻城宝箱",
    },
    RewardTier {
        min_rank: 11,
        max_rank: 15,
        gold: 8000,
        diamond: 80,
        item: "",
    },
    RewardTier {
        min_rank: 16,
        max_rank: 9999,
        gold: 3000,
        diamond: 30,
        item: "",
    },
];

// ==================== 纯逻辑函数 ====================

/// 获取当前应触发的攻城波次 (基于小时)
#[allow(dead_code)]
pub fn get_siege_trigger_hour() -> Option<u32> {
    let hour = Local::now().hour();
    if SIEGE_HOURS.contains(&hour) {
        Some(hour)
    } else {
        None
    }
}

/// 计算攻城伤害 (纯逻辑)
/// base_attack: 玩家攻击力, crit: 暴击率, vip_level: VIP等级
/// defense: 怪物防御, rng_seed: 随机因子 0.0~1.0, rng_var: 方差因子 -1.0~1.0
#[allow(dead_code)]
pub fn calc_siege_damage(
    base_attack: i32,
    combat_power: i32,
    defense: i32,
    vip_level: i32,
    crit: i32,
    rng_seed: f64,
    rng_var: f64,
) -> i64 {
    let base = (base_attack as f64) * (1.0 + combat_power as f64 / 10000.0);
    let after_def = (base - defense as f64).max(1.0);
    let variance = rng_var.clamp(-1.0, 1.0) * 0.15;
    let varied = after_def * (1.0 + variance);
    let is_crit = (crit as f64 / 100.0) > rng_seed;
    let dmg = if is_crit { varied * 2.0 } else { varied };
    let vip_bonus = 1.0 + vip_level as f64 * 0.05;
    let final_dmg = dmg * vip_bonus;
    final_dmg.max(1.0) as i64
}

/// 判定是否暴击 (纯逻辑)
#[allow(dead_code)]
pub fn is_critical_hit(crit: i32, rng_seed: f64) -> bool {
    (crit as f64 / 100.0) > rng_seed
}

/// 格式化攻城HP进度条
#[allow(dead_code)]
pub fn format_siege_hp_bar(current: i64, max_hp: i64, width: usize) -> String {
    let pct = if max_hp > 0 {
        (current as f64 / max_hp as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    format!("{} {:.1}%", bar, pct * 100.0)
}

/// 格式化大数字
#[allow(dead_code)]
pub fn format_siege_number(n: i64) -> String {
    if n >= 100_000_000 {
        format!("{:.1}亿", n as f64 / 100_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}万", n as f64 / 10_000.0)
    } else {
        format!("{}", n)
    }
}

/// 获取波次怪物定义
#[allow(dead_code, private_interfaces)]
pub fn get_wave_monster(wave: i32) -> Option<&'static SiegeMonster> {
    SIEGE_MONSTERS.iter().find(|m| m.wave == wave)
}

/// 根据贡献排名确定奖励档位 (纯逻辑)
#[allow(dead_code, private_interfaces)]
pub fn determine_reward_tier(rank: usize) -> &'static RewardTier {
    for tier in REWARD_TIERS {
        if rank >= tier.min_rank && rank <= tier.max_rank {
            return tier;
        }
    }
    REWARD_TIERS.last().unwrap()
}

/// 计算奖励 (考虑是否防守成功, doubled if success)
#[allow(dead_code, private_interfaces)]
pub fn calc_reward(tier: &RewardTier, success: bool) -> (i64, i32, bool) {
    let multiplier: i64 = if success { 2 } else { 1 };
    (
        tier.gold * multiplier,
        tier.diamond * multiplier as i32,
        !tier.item.is_empty(),
    )
}

/// 根据波次和状态构建攻城进度显示
#[allow(dead_code)]
pub fn build_siege_progress_text(wave: i32, wave_hp: i64, wave_max_hp: i64, waves_cleared: i32) -> String {
    let mut r = "═══ 攻城进度 ═══".to_string();
    r.push_str(&format!("\n已通过波次: {}/5", waves_cleared));
    if (1..=5).contains(&wave) {
        if let Some(monster) = get_wave_monster(wave) {
            r.push_str(&format!("\n当前第{}波: {} {}", wave, monster.emoji, monster.name));
            r.push_str(&format!("\n{}", monster.announcement));
            r.push_str(&format!("\nHP: {}", format_siege_hp_bar(wave_hp, wave_max_hp, 20)));
            r.push_str(&format!(
                "\n  {}/{}",
                format_siege_number(wave_hp),
                format_siege_number(wave_max_hp)
            ));
        }
    } else if waves_cleared >= 5 {
        r.push_str("\n🎉 全部5波攻城怪物已被击退！");
    }
    r
}

/// 获取奖励物品名
#[allow(dead_code, private_interfaces)]
pub fn get_reward_item(tier: &RewardTier) -> &str {
    tier.item
}

// ==================== 攻城状态管理 ====================

/// 攻城状态
#[derive(Debug, Clone)]
struct SiegeState {
    wave: i32,
    wave_hp: i64,
    wave_max_hp: i64,
    start_time: i64,
    end_time: i64,
    waves_cleared: i32,
    status: String,
    total_damage: i64,
    participants: i32,
}

impl Default for SiegeState {
    fn default() -> Self {
        Self {
            wave: 0,
            wave_hp: 0,
            wave_max_hp: 0,
            start_time: 0,
            end_time: 0,
            waves_cleared: 0,
            status: "waiting".to_string(),
            total_damage: 0,
            participants: 0,
        }
    }
}

/// 解析攻城状态 (简易KV格式)
fn parse_siege_state(data: &str) -> SiegeState {
    let mut state = SiegeState::default();
    for part in data.split(',') {
        let kv: Vec<&str> = part.splitn(2, ':').collect();
        if kv.len() == 2 {
            let key = kv[0].trim();
            let val = kv[1].trim();
            match key {
                "wave" => state.wave = val.parse().unwrap_or(0),
                "wave_hp" => state.wave_hp = val.parse().unwrap_or(0),
                "wave_max_hp" => state.wave_max_hp = val.parse().unwrap_or(0),
                "start_time" => state.start_time = val.parse().unwrap_or(0),
                "end_time" => state.end_time = val.parse().unwrap_or(0),
                "waves_cleared" => state.waves_cleared = val.parse().unwrap_or(0),
                "status" => state.status = val.to_string(),
                "total_damage" => state.total_damage = val.parse().unwrap_or(0),
                "participants" => state.participants = val.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    state
}

/// 序列化攻城状态
fn serialize_siege_state(s: &SiegeState) -> String {
    format!(
        "wave:{},wave_hp:{},wave_max_hp:{},start_time:{},end_time:{},\
         waves_cleared:{},status:{},total_damage:{},participants:{}",
        s.wave,
        s.wave_hp,
        s.wave_max_hp,
        s.start_time,
        s.end_time,
        s.waves_cleared,
        s.status,
        s.total_damage,
        s.participants
    )
}

/// 读取攻城状态
fn read_siege_state(db: &Database) -> SiegeState {
    let data = db.global_get("monster_siege", "state");
    if data.is_empty() {
        return SiegeState::default();
    }
    parse_siege_state(&data)
}

/// 检查是否到了攻城时间并初始化
fn check_and_start_siege(db: &Database) -> SiegeState {
    let state = read_siege_state(db);
    let now = Local::now();
    let hour = now.hour();
    let today = now.format("%Y%m%d").to_string();
    let now_ts = now.timestamp();

    // 如果当前状态是active且未超时，直接返回
    if state.status == "active" && now_ts < state.end_time {
        return state;
    }

    // 检查是否到了新的攻城时间
    if SIEGE_HOURS.contains(&hour) {
        let event_key = format!("{}_{:02}", today, hour);
        let last_event = db.global_get("monster_siege", "last_event");
        if last_event != event_key {
            // 开启新攻城
            let first = &SIEGE_MONSTERS[0];
            let new_state = SiegeState {
                wave: 1,
                wave_hp: first.max_hp,
                wave_max_hp: first.max_hp,
                start_time: now_ts,
                end_time: now_ts + 3600,
                waves_cleared: 0,
                status: "active".to_string(),
                total_damage: 0,
                participants: 0,
            };
            db.global_set("monster_siege", "state", &serialize_siege_state(&new_state));
            db.global_set("monster_siege", "last_event", &event_key);
            return new_state;
        }
    }

    state
}

/// 读取玩家贡献数据
fn read_contribution(db: &Database, user_id: &str) -> PlayerContribution {
    let key = format!("contrib_{}", user_id);
    let data = db.global_get("monster_siege", &key);
    if data.is_empty() {
        return PlayerContribution::default();
    }
    let mut c = PlayerContribution::default();
    for part in data.split(',') {
        let kv: Vec<&str> = part.splitn(2, ':').collect();
        if kv.len() == 2 {
            match kv[0].trim() {
                "total_damage" => c.total_damage = kv[1].trim().parse().unwrap_or(0),
                "attacks_used" => c.attacks_used = kv[1].trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    c
}

fn save_contribution(db: &Database, user_id: &str, c: &PlayerContribution) {
    let key = format!("contrib_{}", user_id);
    let data = format!("total_damage:{},attacks_used:{}", c.total_damage, c.attacks_used);
    db.global_set("monster_siege", &key, &data);
}

#[derive(Debug, Clone, Default)]
struct PlayerContribution {
    total_damage: i64,
    attacks_used: i32,
}

/// 获取今日攻城攻击次数
fn get_daily_attacks(db: &Database, user_id: &str) -> i32 {
    let today = Local::now().format("%Y%m%d").to_string();
    let key = format!("{}_{}", user_id, today);
    let val = db.global_get("monster_siege_daily", &key);
    val.parse().unwrap_or(0)
}

fn increment_daily_attacks(db: &Database, user_id: &str) -> i32 {
    let today = Local::now().format("%Y%m%d").to_string();
    let key = format!("{}_{}", user_id, today);
    let current = get_daily_attacks(db, user_id);
    let new_count = current + 1;
    db.global_set("monster_siege_daily", &key, &new_count.to_string());
    new_count
}

/// 获取所有贡献者排名
fn get_all_contributions(db: &Database) -> Vec<(String, i64)> {
    let participants_str = db.global_get("monster_siege", "participants_list");
    if participants_str.is_empty() {
        return Vec::new();
    }
    let mut results = Vec::new();
    for pid in participants_str.split(',') {
        let pid = pid.trim();
        if pid.is_empty() {
            continue;
        }
        let c = read_contribution(db, pid);
        if c.total_damage > 0 {
            let nickname = db.read_basic(pid, ITEM_NAME);
            let display = if nickname.is_empty() {
                pid.to_string()
            } else {
                format!("{}({})", nickname, pid)
            };
            results.push((display, c.total_damage));
        }
    }
    results.sort_by_key(|b| std::cmp::Reverse(b.1));
    results
}

/// 记录参与者
fn add_participant(db: &Database, user_id: &str) {
    let list = db.global_get("monster_siege", "participants_list");
    let mut participants: Vec<String> = if list.is_empty() {
        Vec::new()
    } else {
        list.split(',').map(|s| s.to_string()).collect()
    };
    if !participants.contains(&user_id.to_string()) {
        participants.push(user_id.to_string());
        db.global_set("monster_siege", "participants_list", &participants.join(","));
        db.global_set("monster_siege", "participants", &participants.len().to_string());
    }
}

// ==================== 命令实现 ====================

/// 查看当前攻城状态
pub fn cmd_siege_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }

    let state = check_and_start_siege(db);

    let mut r = format!("{}\n═══ 🏰 怪物攻城 ═══", prefix);

    if state.status == "active" {
        if let Some(monster) = get_wave_monster(state.wave) {
            r.push_str(&format!(
                "\n\n{} 第{}波: {} {}",
                monster.emoji, state.wave, monster.emoji, monster.name
            ));
            r.push_str(&format!("\n{}", monster.announcement));
            r.push_str("\n\n❤️ 怪物生命:");
            r.push_str(&format!(
                "\n  {}",
                format_siege_hp_bar(state.wave_hp, state.wave_max_hp, 20)
            ));
            r.push_str(&format!(
                "\n  {}/{}",
                format_siege_number(state.wave_hp),
                format_siege_number(state.wave_max_hp)
            ));
            r.push_str("\n\n⚔️ 战况:");
            r.push_str(&format!("\n  已通过波次: {}/5", state.waves_cleared));
            r.push_str(&format!("\n  总伤害: {}", format_siege_number(state.total_damage)));
            r.push_str(&format!("\n  参与人数: {}人", state.participants));

            let remaining = state.end_time - Local::now().timestamp();
            if remaining > 0 {
                let mins = remaining / 60;
                r.push_str(&format!("\n\n⏰ 剩余时间: {}分钟", mins));
            }

            let daily = get_daily_attacks(db, user_id);
            r.push_str("\n\n📋 你的状态:");
            r.push_str(&format!("\n  今日攻击: {}/{}", daily, MAX_ATTACKS_PER_SIEGE));

            let contrib = read_contribution(db, user_id);
            if contrib.total_damage > 0 {
                r.push_str(&format!("\n  累计伤害: {}", format_siege_number(contrib.total_damage)));
            }

            r.push_str("\n\n发送 '参与攻城' 攻击当前波怪物");
        }
    } else if state.status == "ended" || state.waves_cleared >= 5 {
        r.push_str("\n\n✅ 本次攻城已结束！怪物已被击退！");
        r.push_str(&format!("\n通过波次: {}/5", state.waves_cleared));
        r.push_str(&format!("\n总伤害: {}", format_siege_number(state.total_damage)));
        r.push_str(&format!("\n参与人数: {}人", state.participants));
        r.push_str("\n\n发送 '攻城奖励' 领取奖励");
        r.push_str("\n发送 '攻城排行' 查看伤害排名");
    } else {
        r.push_str("\n\n💤 当前无攻城活动");
        r.push_str("\n攻城时间: 0:00, 4:00, 8:00, 12:00, 16:00, 20:00");
        r.push_str("\n每次持续1小时");
    }

    r.push_str("\n\n发送 '攻城帮助' 查看详细说明");
    r
}

/// 参与攻城攻击
pub fn cmd_siege_attack(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }

    let mut state = read_siege_state(db);

    // 自动检查并开启
    if state.status != "active" || Local::now().timestamp() >= state.end_time {
        state = check_and_start_siege(db);
    }

    if state.status != "active" {
        return format!(
            "{}\n当前无攻城活动进行中。\n攻城时间: 0:00, 4:00, 8:00, 12:00, 16:00, 20:00",
            prefix
        );
    }

    // 检查攻击次数
    let daily = get_daily_attacks(db, user_id);
    if daily >= MAX_ATTACKS_PER_SIEGE {
        return format!(
            "{}\n❌ 今日攻城攻击次数已用完（{}/{}）。\n每次攻城活动最多攻击{}次。",
            prefix, daily, MAX_ATTACKS_PER_SIEGE, MAX_ATTACKS_PER_SIEGE
        );
    }

    // 检查怪物是否已死
    if state.wave_hp <= 0 {
        return format!("{}\n当前波怪物已被击退，请等待下一波。", prefix);
    }

    // 计算伤害
    let info = user::calc_total_attrs(db, user_id);
    let vip_level = vip::get_vip_level(db, user_id);
    let mut rng = rand::thread_rng();

    let monster_defense = get_wave_monster(state.wave).map(|m| m.defense).unwrap_or(100);
    let combat_power = info.ad + info.ap;

    let rng_seed: f64 = rng.gen_range(0.0..1.0);
    let rng_var: f64 = rng.gen_range(-1.0..1.0);
    let damage = calc_siege_damage(
        info.ad,
        combat_power,
        monster_defense,
        vip_level,
        info.crit,
        rng_seed,
        rng_var,
    );

    let is_crit = is_critical_hit(info.crit, rng_seed);

    // 扣除怪物HP
    state.wave_hp = (state.wave_hp - damage).max(0);
    state.total_damage += damage;

    // 增加攻击次数
    increment_daily_attacks(db, user_id);

    // 记录贡献
    add_participant(db, user_id);
    let mut contrib = read_contribution(db, user_id);
    contrib.total_damage += damage;
    contrib.attacks_used += 1;
    save_contribution(db, user_id, &contrib);

    // 构建战斗结果
    let monster = get_wave_monster(state.wave).unwrap();
    let mut r = format!(
        "{}\n═══ {} 攻城战斗: {} {} ═══",
        prefix, monster.emoji, monster.emoji, monster.name
    );

    if is_crit {
        r.push_str(&format!(
            "\n💥 暴击！你对 {} 造成 {} 点伤害！",
            monster.name,
            format_siege_number(damage)
        ));
    } else {
        r.push_str(&format!(
            "\n⚔️ 你对 {} 造成 {} 点伤害！",
            monster.name,
            format_siege_number(damage)
        ));
    }
    if vip_level > 0 {
        r.push_str(&format!(" (VIP+{}%)", vip_level * 5));
    }

    r.push_str(&format!(
        "\n\n❤️ 怪物剩余: {}",
        format_siege_hp_bar(state.wave_hp, state.wave_max_hp, 15)
    ));
    r.push_str(&format!(
        "\n  {}/{}",
        format_siege_number(state.wave_hp),
        format_siege_number(state.wave_max_hp)
    ));

    // 检查波次是否击退
    if state.wave_hp <= 0 {
        state.waves_cleared += 1;
        r.push_str(&format!(
            "\n\n🎉🎉🎉 第{}波 {} 已被击退！🎉🎉🎉",
            state.wave, monster.name
        ));

        if state.wave < 5 {
            let next_wave = state.wave + 1;
            let next_monster = get_wave_monster(next_wave).unwrap();
            state.wave = next_wave;
            state.wave_hp = next_monster.max_hp;
            state.wave_max_hp = next_monster.max_hp;

            r.push_str(&format!(
                "\n\n{} 第{}波来袭: {} {}",
                next_monster.emoji, next_wave, next_monster.emoji, next_monster.name
            ));
            r.push_str(&format!("\n{}", next_monster.announcement));
            r.push_str(&format!("\nHP: {}", format_siege_number(next_monster.max_hp)));
        } else {
            state.status = "ended".to_string();
            r.push_str("\n\n🏆🏆🏆 全部5波攻城怪物已被击退！全服勇士胜利！🏆🏆🏆");
            r.push_str("\n\n发送 '攻城奖励' 领取你的奖励！");
        }
    }

    // 保存状态
    db.global_set("monster_siege", "state", &serialize_siege_state(&state));

    let remaining_attacks = MAX_ATTACKS_PER_SIEGE - get_daily_attacks(db, user_id);
    r.push_str(&format!("\n\n📋 剩余攻击次数: {}", remaining_attacks));
    r.push_str("\n发送 '攻城排行' 查看伤害排名");

    r
}

/// 查看攻城进度
pub fn cmd_siege_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }

    let state = read_siege_state(db);

    let mut r = format!("{}\n═══ 📊 攻城进度 ═══", prefix);

    for wave_def in SIEGE_MONSTERS {
        if wave_def.wave < state.wave {
            r.push_str(&format!(
                "\n✅ 第{}波 {} {} — 已击退",
                wave_def.wave, wave_def.emoji, wave_def.name
            ));
        } else if wave_def.wave == state.wave && state.status == "active" {
            r.push_str(&format!(
                "\n⚔️ 第{}波 {} {} — 战斗中",
                wave_def.wave, wave_def.emoji, wave_def.name
            ));
            r.push_str(&format!(
                "\n   HP: {}",
                format_siege_hp_bar(state.wave_hp, state.wave_max_hp, 15)
            ));
        } else {
            r.push_str(&format!(
                "\n🔒 第{}波 {} {} — 未开始",
                wave_def.wave, wave_def.emoji, wave_def.name
            ));
        }
    }

    r.push_str(&format!("\n\n总伤害: {}", format_siege_number(state.total_damage)));
    r.push_str(&format!("\n参与人数: {}人", state.participants));

    let contrib = read_contribution(db, user_id);
    if contrib.total_damage > 0 {
        r.push_str("\n\n📍 你的贡献:");
        r.push_str(&format!("\n  总伤害: {}", format_siege_number(contrib.total_damage)));
        r.push_str(&format!("\n  攻击次数: {}", contrib.attacks_used));
    }

    r
}

/// 查看攻城排行榜
pub fn cmd_siege_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }

    let rankings = get_all_contributions(db);
    let state = read_siege_state(db);

    let mut r = format!("{}\n═══ 🏆 攻城伤害排名 ═══", prefix);

    if state.status == "ended" || state.waves_cleared >= 5 {
        r.push_str("\n✅ 攻城已结束！");
    }
    r.push_str(&format!(
        "\n总伤害: {} | 参与: {}人\n",
        format_siege_number(state.total_damage),
        rankings.len()
    ));

    if rankings.is_empty() {
        r.push_str("\n暂无参与记录。");
        r.push_str("\n发送 '参与攻城' 参与战斗！");
        return r;
    }

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, dmg)) in rankings.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let pct = if state.total_damage > 0 {
            *dmg as f64 / state.total_damage as f64 * 100.0
        } else {
            0.0
        };
        r.push_str(&format!(
            "\n{}{}. {} — {} ({:.1}%)",
            medal,
            i + 1,
            name,
            format_siege_number(*dmg),
            pct
        ));
    }

    let user_contrib = read_contribution(db, user_id);
    if user_contrib.total_damage > 0 {
        let user_rank = rankings
            .iter()
            .position(|(n, _)| n.contains(user_id))
            .map(|p| p + 1)
            .unwrap_or(0);
        r.push_str(&format!(
            "\n\n📍 你的排名: 第{}名 | 伤害: {}",
            user_rank,
            format_siege_number(user_contrib.total_damage)
        ));
    } else {
        r.push_str("\n\n你尚未参与本次攻城。");
        r.push_str("\n发送 '参与攻城' 参与战斗！");
    }

    r
}

/// 领取攻城奖励
pub fn cmd_siege_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }

    let state = read_siege_state(db);
    let contrib = read_contribution(db, user_id);

    if contrib.total_damage <= 0 {
        return format!(
            "{}\n你尚未参与本次攻城，无法领取奖励。\n发送 '参与攻城' 参与战斗！",
            prefix
        );
    }

    let success = state.status == "ended" || state.waves_cleared >= 5;
    if !success && state.status == "active" {
        return format!(
            "{}\n攻城尚未结束，请在攻城结束后领取奖励。\n当前状态: 第{}波进行中",
            prefix, state.wave
        );
    }

    // 检查是否已领取
    let reward_key = format!("rewards_{}", user_id);
    let reward_data = db.global_get("monster_siege", &reward_key);
    if !reward_data.is_empty() && reward_data.contains("claimed:true") {
        return format!("{}\n你已经领取过本次攻城奖励了。", prefix);
    }

    // 计算排名
    let rankings = get_all_contributions(db);
    let user_rank = rankings
        .iter()
        .position(|(n, _)| n.contains(user_id))
        .map(|p| p + 1)
        .unwrap_or(rankings.len());

    let tier = determine_reward_tier(user_rank);
    let (gold, diamond, has_item) = calc_reward(tier, success);

    // 发放奖励
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", gold);
    let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", diamond as i64);

    if has_item {
        db.knapsack_add(user_id, tier.item, 1);
    }

    // 标记已领取
    db.global_set(
        "monster_siege",
        &reward_key,
        &format!("claimed:true,rank:{}", user_rank),
    );

    let mut r = format!("{}\n═══ 🎁 攻城奖励 ═══", prefix);
    r.push_str(&format!("\n你的排名: 第{}名", user_rank));
    if success {
        r.push_str("\n🏆 防守成功！奖励翻倍！");
    }
    r.push_str("\n\n💰 获得奖励:");
    r.push_str(&format!("\n  金币: {}", format_siege_number(gold)));
    r.push_str(&format!("\n  钻石: {}", diamond));
    if has_item {
        r.push_str(&format!("\n  物品: {} ×1", tier.item));
    }
    r.push_str("\n\n📊 你的贡献:");
    r.push_str(&format!("\n  总伤害: {}", format_siege_number(contrib.total_damage)));
    r.push_str(&format!("\n  攻击次数: {}", contrib.attacks_used));

    r
}

/// 攻城帮助
pub fn cmd_siege_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut r = format!("{}\n═══ 📖 怪物攻城帮助 ═══", prefix);

    r.push_str("\n\n🏰 系统说明:");
    r.push_str("\n全服协作防守攻城怪物，共5波进攻！");

    r.push_str("\n\n⏰ 攻城时间:");
    r.push_str("\n  每日 0:00, 4:00, 8:00, 12:00, 16:00, 20:00");
    r.push_str("\n  每次持续1小时");

    r.push_str("\n\n👾 攻城怪物:");
    for m in SIEGE_MONSTERS {
        r.push_str(&format!(
            "\n  第{}波 {} {} (HP:{})",
            m.wave,
            m.emoji,
            m.name,
            format_siege_number(m.max_hp)
        ));
    }

    r.push_str("\n\n⚔️ 战斗规则:");
    r.push_str(&format!("\n  每次攻城最多攻击{}次", MAX_ATTACKS_PER_SIEGE));
    r.push_str("\n  伤害基于玩家攻击力和战力");
    r.push_str("\n  有概率暴击(2倍伤害)");
    r.push_str("\n  VIP等级越高伤害加成越多");

    r.push_str("\n\n🎁 奖励规则:");
    r.push_str("\n  Top 1: 50000金+500钻+暗黑魔王碎片");
    r.push_str("\n  Top 2-3: 30000金+300钻+攻城精魄");
    r.push_str("\n  Top 4-10: 15000金+150钻+攻城宝箱");
    r.push_str("\n  Top 11-15: 8000金+80钻");
    r.push_str("\n  参与奖: 3000金+30钻");
    r.push_str("\n  防守成功(5波全清): 奖励翻倍！");

    r.push_str("\n\n📋 指令列表:");
    r.push_str("\n  怪物攻城 — 查看攻城状态");
    r.push_str("\n  参与攻城 — 攻击当前波怪物");
    r.push_str("\n  攻城进度 — 查看各波进度");
    r.push_str("\n  攻城排行 — 查看伤害排名");
    r.push_str("\n  攻城奖励 — 领取攻城奖励");
    r.push_str("\n  攻城帮助 — 查看本帮助");

    r
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_siege_monsters_count() {
        assert_eq!(SIEGE_MONSTERS.len(), 5);
    }

    #[test]
    fn test_siege_monsters_wave_order() {
        for (i, m) in SIEGE_MONSTERS.iter().enumerate() {
            assert_eq!(m.wave, (i + 1) as i32);
        }
    }

    #[test]
    fn test_siege_monsters_hp_increasing() {
        for i in 1..SIEGE_MONSTERS.len() {
            assert!(SIEGE_MONSTERS[i].max_hp > SIEGE_MONSTERS[i - 1].max_hp);
        }
    }

    #[test]
    fn test_siege_monsters_valid() {
        for m in SIEGE_MONSTERS {
            assert!(m.max_hp > 0);
            assert!(m.attack > 0);
            assert!(m.defense > 0);
            assert!(!m.name.is_empty());
            assert!(!m.emoji.is_empty());
            assert!(!m.announcement.is_empty());
        }
    }

    #[test]
    fn test_calc_siege_damage_basic() {
        // base_attack=1000, combat_power=5000, defense=100, vip=0, crit=0
        let dmg = calc_siege_damage(1000, 5000, 100, 0, 0, 0.5, 0.0);
        assert!(dmg > 0);
        // base = 1000 * (1 + 5000/10000) = 1500
        // after_def = 1500 - 100 = 1400
        assert_eq!(dmg, 1400);
    }

    #[test]
    fn test_calc_siege_damage_with_crit() {
        // crit=50, rng_seed=0.3 -> should crit (0.5 > 0.3)
        let dmg_crit = calc_siege_damage(1000, 0, 0, 0, 50, 0.3, 0.0);
        assert_eq!(dmg_crit, 2000);

        // crit=50, rng_seed=0.7 -> should not crit (0.5 < 0.7)
        let dmg_no_crit = calc_siege_damage(1000, 0, 0, 0, 50, 0.7, 0.0);
        assert_eq!(dmg_no_crit, 1000);
    }

    #[test]
    fn test_calc_siege_damage_vip_bonus() {
        let dmg_vip0 = calc_siege_damage(1000, 0, 0, 0, 0, 0.5, 0.0);
        let dmg_vip5 = calc_siege_damage(1000, 0, 0, 5, 0, 0.5, 0.0);
        assert_eq!(dmg_vip0, 1000);
        assert_eq!(dmg_vip5, 1250);
    }

    #[test]
    fn test_calc_siege_damage_variance() {
        let dmg_high = calc_siege_damage(1000, 0, 0, 0, 0, 0.5, 1.0);
        assert_eq!(dmg_high, 1150);

        let dmg_low = calc_siege_damage(1000, 0, 0, 0, 0, 0.5, -1.0);
        assert_eq!(dmg_low, 850);
    }

    #[test]
    fn test_calc_siege_damage_minimum_one() {
        let dmg = calc_siege_damage(10, 0, 99999, 0, 0, 0.5, 0.0);
        assert!(dmg >= 1);
    }

    #[test]
    fn test_is_critical_hit() {
        assert!(is_critical_hit(50, 0.3));
        assert!(!is_critical_hit(50, 0.7));
        assert!(is_critical_hit(100, 0.99));
        assert!(!is_critical_hit(0, 0.01));
    }

    #[test]
    fn test_format_siege_hp_bar() {
        let bar = format_siege_hp_bar(50, 100, 10);
        assert!(bar.contains("50.0%"));
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));

        let full = format_siege_hp_bar(100, 100, 10);
        assert!(full.contains("100.0%"));

        let empty = format_siege_hp_bar(0, 100, 10);
        assert!(empty.contains("0.0%"));
    }

    #[test]
    fn test_format_siege_hp_bar_edge_cases() {
        let bar = format_siege_hp_bar(-100, 1000, 10);
        assert!(bar.contains("0.0%"));

        let bar = format_siege_hp_bar(2000, 1000, 10);
        assert!(bar.contains("100.0%"));

        let bar = format_siege_hp_bar(0, 0, 10);
        assert!(bar.contains("0.0%"));
    }

    #[test]
    fn test_format_siege_number() {
        assert_eq!(format_siege_number(500), "500");
        assert_eq!(format_siege_number(9999), "9999");
        assert_eq!(format_siege_number(10000), "1.0万");
        assert_eq!(format_siege_number(55000), "5.5万");
        assert_eq!(format_siege_number(100_000_000), "1.0亿");
        assert_eq!(format_siege_number(0), "0");
        assert_eq!(format_siege_number(1_234_567_890), "12.3亿");
    }

    #[test]
    fn test_get_wave_monster() {
        assert!(get_wave_monster(1).is_some());
        assert!(get_wave_monster(5).is_some());
        assert!(get_wave_monster(0).is_none());
        assert!(get_wave_monster(6).is_none());

        let wave1 = get_wave_monster(1).unwrap();
        assert_eq!(wave1.name, "狂暴狼群");
        assert_eq!(wave1.max_hp, 50_000);

        let wave5 = get_wave_monster(5).unwrap();
        assert_eq!(wave5.name, "暗黑魔王");
        assert_eq!(wave5.max_hp, 800_000);
    }

    #[test]
    fn test_determine_reward_tier() {
        let t1 = determine_reward_tier(1);
        assert_eq!(t1.gold, 50000);
        assert_eq!(t1.diamond, 500);

        let t2 = determine_reward_tier(2);
        assert_eq!(t2.gold, 30000);
        assert_eq!(t2.diamond, 300);

        let t3 = determine_reward_tier(3);
        assert_eq!(t3.gold, 30000);

        let t4 = determine_reward_tier(5);
        assert_eq!(t4.gold, 15000);
        assert_eq!(t4.diamond, 150);

        let t5 = determine_reward_tier(11);
        assert_eq!(t5.gold, 8000);
        assert_eq!(t5.diamond, 80);

        let t6 = determine_reward_tier(20);
        assert_eq!(t6.gold, 3000);
        assert_eq!(t6.diamond, 30);
    }

    #[test]
    fn test_calc_reward_normal() {
        let tier = determine_reward_tier(1);
        let (gold, diamond, has_item) = calc_reward(tier, false);
        assert_eq!(gold, 50000);
        assert_eq!(diamond, 500);
        assert!(has_item);
    }

    #[test]
    fn test_calc_reward_success_doubled() {
        let tier = determine_reward_tier(1);
        let (gold, diamond, has_item) = calc_reward(tier, true);
        assert_eq!(gold, 100000);
        assert_eq!(diamond, 1000);
        assert!(has_item);

        let tier_participation = determine_reward_tier(20);
        let (gold, diamond, _) = calc_reward(tier_participation, true);
        assert_eq!(gold, 6000);
        assert_eq!(diamond, 60);
    }

    #[test]
    fn test_build_siege_progress_text() {
        let text = build_siege_progress_text(1, 25000, 50000, 0);
        assert!(text.contains("第1波"));
        assert!(text.contains("狂暴狼群"));
        assert!(text.contains("50.0%"));

        let text_done = build_siege_progress_text(0, 0, 0, 5);
        assert!(text_done.contains("全部5波"));
    }

    #[test]
    fn test_reward_tiers_cover_all_ranks() {
        for rank in 1..=20 {
            let tier = determine_reward_tier(rank);
            assert!(tier.gold > 0);
            assert!(tier.diamond > 0);
        }
    }

    #[test]
    fn test_parse_siege_state() {
        let data = "wave:3,wave_hp:150000,wave_max_hp:200000,\
                     start_time:1000,end_time:2000,waves_cleared:2,\
                     status:active,total_damage:50000,participants:10";
        let state = parse_siege_state(data);
        assert_eq!(state.wave, 3);
        assert_eq!(state.wave_hp, 150000);
        assert_eq!(state.wave_max_hp, 200000);
        assert_eq!(state.waves_cleared, 2);
        assert_eq!(state.status, "active");
        assert_eq!(state.total_damage, 50000);
        assert_eq!(state.participants, 10);
    }

    #[test]
    fn test_serialize_and_parse_roundtrip() {
        let state = SiegeState {
            wave: 4,
            wave_hp: 300000,
            wave_max_hp: 400000,
            start_time: 12345,
            end_time: 23456,
            waves_cleared: 3,
            status: "active".to_string(),
            total_damage: 750000,
            participants: 25,
        };
        let serialized = serialize_siege_state(&state);
        let parsed = parse_siege_state(&serialized);
        assert_eq!(parsed.wave, 4);
        assert_eq!(parsed.wave_hp, 300000);
        assert_eq!(parsed.wave_max_hp, 400000);
        assert_eq!(parsed.waves_cleared, 3);
        assert_eq!(parsed.status, "active");
        assert_eq!(parsed.total_damage, 750000);
        assert_eq!(parsed.participants, 25);
    }

    #[test]
    fn test_siege_hours_valid() {
        for &h in SIEGE_HOURS {
            assert!(h < 24);
        }
        assert_eq!(SIEGE_HOURS.len(), 6);
    }

    #[test]
    fn test_reward_items() {
        let t1 = determine_reward_tier(1);
        assert_eq!(t1.item, "暗黑魔王碎片");

        let t2 = determine_reward_tier(2);
        assert_eq!(t2.item, "攻城精魄");

        let t3 = determine_reward_tier(5);
        assert_eq!(t3.item, "攻城宝箱");

        let t4 = determine_reward_tier(11);
        assert!(t4.item.is_empty());

        let t5 = determine_reward_tier(20);
        assert!(t5.item.is_empty());
    }

    #[test]
    fn test_get_reward_item() {
        let tier = determine_reward_tier(1);
        assert_eq!(get_reward_item(tier), "暗黑魔王碎片");

        let tier2 = determine_reward_tier(20);
        assert_eq!(get_reward_item(tier2), "");
    }
}

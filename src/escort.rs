/// CakeGame 镖局押运系统
/// 经典MMORPG押镖玩法: 接受押运任务→护送货物→抵御劫匪→领取奖励
/// 支持: 查看镖局/接受押运/押运状态/护送货物/完成押运/放弃押运/劫镖/镖局排行/镖局帮助
use crate::core::*;
use crate::db::Database;
use crate::user;
use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

/// 镖车品质
#[derive(Clone, Copy, Debug)]
struct CaravanTier {
    name: &'static str,
    icon: &'static str,
    level_req: i32,
    gold_cost: i64,
    diamond_cost: i64,
    base_reward_gold: i64,
    base_reward_exp: i32,
    risk_pct: u32,
    max_hp: i32,
    escort_time_min: u64,
}

const CARAVAN_TIERS: &[CaravanTier] = &[
    CaravanTier {
        name: "普通镖车",
        icon: "📦",
        level_req: 10,
        gold_cost: 500,
        diamond_cost: 0,
        base_reward_gold: 2000,
        base_reward_exp: 100,
        risk_pct: 15,
        max_hp: 100,
        escort_time_min: 5,
    },
    CaravanTier {
        name: "精良镖车",
        icon: "📦✨",
        level_req: 25,
        gold_cost: 2000,
        diamond_cost: 5,
        base_reward_gold: 8000,
        base_reward_exp: 400,
        risk_pct: 20,
        max_hp: 250,
        escort_time_min: 8,
    },
    CaravanTier {
        name: "稀有镖车",
        icon: "📦💎",
        level_req: 40,
        gold_cost: 8000,
        diamond_cost: 15,
        base_reward_gold: 30000,
        base_reward_exp: 1500,
        risk_pct: 25,
        max_hp: 500,
        escort_time_min: 12,
    },
    CaravanTier {
        name: "史诗镖车",
        icon: "📦🔥",
        level_req: 60,
        gold_cost: 30000,
        diamond_cost: 50,
        base_reward_gold: 120000,
        base_reward_exp: 6000,
        risk_pct: 30,
        max_hp: 1000,
        escort_time_min: 18,
    },
    CaravanTier {
        name: "传说镖车",
        icon: "📦⚜️",
        level_req: 80,
        gold_cost: 100000,
        diamond_cost: 150,
        base_reward_gold: 500000,
        base_reward_exp: 25000,
        risk_pct: 35,
        max_hp: 2000,
        escort_time_min: 25,
    },
];

const SECTION: &str = "escort";
const SECTION_DAILY: &str = "escort_daily";
const SECTION_STATS: &str = "escort_stats";
const SECTION_REP: &str = "escort_rep";
const SECTION_ROB: &str = "escort_rob";
const SECTION_EVIL: &str = "evil_value";
const MAX_DAILY_ESCORT: i32 = 5;
const MAX_DAILY_ROB: i32 = 3;

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn chrono_like_today() -> String {
    format!("day{}", now_secs() / 86400)
}

/// 解析用户的押运状态: tier|start_time|current_hp|rob_attempts|status
fn parse_escort_record(db: &Database, user_id: &str) -> Option<(usize, u64, i32, u32, String)> {
    let data = db.global_get(SECTION, user_id);
    if data.is_empty() {
        return None;
    }
    let parts: Vec<&str> = data.split('|').collect();
    if parts.len() < 4 {
        return None;
    }
    let tier: usize = parts[0].parse().unwrap_or(0);
    let start_time: u64 = parts[1].parse().unwrap_or(0);
    let current_hp: i32 = parts[2].parse().unwrap_or(0);
    let rob_attempts: u32 = parts[3].parse().unwrap_or(0);
    let status = if parts.len() > 4 {
        parts[4].to_string()
    } else {
        "active".to_string()
    };
    Some((tier, start_time, current_hp, rob_attempts, status))
}

fn save_escort_record(
    db: &Database,
    user_id: &str,
    tier: usize,
    start_time: u64,
    current_hp: i32,
    rob_attempts: u32,
    status: &str,
) {
    let data = format!("{}|{}|{}|{}|{}", tier, start_time, current_hp, rob_attempts, status);
    db.global_set(SECTION, user_id, &data);
}

/// 获取今日押运次数
fn get_daily_count(db: &Database, user_id: &str) -> i32 {
    let today = chrono_like_today();
    let key = format!("{}_{}", user_id, today);
    db.global_get(SECTION_DAILY, &key).parse().unwrap_or(0)
}

fn inc_daily_count(db: &Database, user_id: &str) {
    let today = chrono_like_today();
    let key = format!("{}_{}", user_id, today);
    let cur = get_daily_count(db, user_id);
    db.global_set(SECTION_DAILY, &key, &(cur + 1).to_string());
}

/// 获取今日劫镖次数
fn get_rob_count(db: &Database, user_id: &str) -> i32 {
    let today = chrono_like_today();
    let key = format!("{}_{}", user_id, today);
    db.global_get(SECTION_ROB, &key).parse().unwrap_or(0)
}

fn inc_rob_count(db: &Database, user_id: &str) {
    let today = chrono_like_today();
    let key = format!("{}_{}", user_id, today);
    let cur = get_rob_count(db, user_id);
    db.global_set(SECTION_ROB, &key, &(cur + 1).to_string());
}

/// 计算押运加成(基于VIP和等级)
fn calc_bonus_multiplier(db: &Database, user_id: &str) -> f64 {
    let mut mult = 1.0f64;
    // VIP加成
    let vip_data = db.global_get("vip", user_id);
    let vip_level: i32 = vip_data.split('|').next().and_then(|v| v.parse().ok()).unwrap_or(0);
    mult += vip_level as f64 * 0.05;
    // 等级加成
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level >= 80 {
        mult += 0.3;
    } else if level >= 50 {
        mult += 0.15;
    } else if level >= 30 {
        mult += 0.05;
    }
    mult
}

/// 进度百分比
fn calc_progress(start_time: u64, duration_min: u64) -> u32 {
    let elapsed = now_secs().saturating_sub(start_time);
    let total_secs = duration_min * 60;
    (elapsed * 100).checked_div(total_secs).unwrap_or(100).min(100) as u32
}

/// 进度条渲染
fn progress_bar(pct: u32, width: usize) -> String {
    let filled = ((pct as usize * width) / 100).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

// ========== 指令实现 ==========

/// 查看镖局
pub fn cmd_view_escort(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let daily = get_daily_count(db, user_id);

    let mut out = String::from("╔══════════════════════════╗\n");
    out.push_str("║      🏪 镖局押运 🏪      ║\n");
    out.push_str("╚══════════════════════════╝\n\n");
    out.push_str(&format!(
        "📊 等级: {} | 📦 今日押运: {}/{}\n\n",
        level, daily, MAX_DAILY_ESCORT
    ));

    out.push_str("━━━ 可选镖车 ━━━\n");
    for (i, t) in CARAVAN_TIERS.iter().enumerate() {
        let lock = if level < t.level_req { "🔒" } else { "✅" };
        out.push_str(&format!(
            "{} {} Lv.{}+ | 费用: {}金",
            lock, t.icon, t.level_req, t.gold_cost
        ));
        if t.diamond_cost > 0 {
            out.push_str(&format!("+{}💎", t.diamond_cost));
        }
        out.push_str(&format!(
            "\n   奖励: {}金+{}经验 | 风险: {}% | 耐久: {}HP | 时间: {}分钟\n",
            t.base_reward_gold, t.base_reward_exp, t.risk_pct, t.max_hp, t.escort_time_min
        ));
        out.push_str(&format!("   接受押运: 接受押运 {}\n\n", i + 1));
    }

    // 检查进行中的押运
    if let Some((tier, start_time, current_hp, rob_attempts, status)) = parse_escort_record(db, user_id) {
        if status == "active" {
            let t = &CARAVAN_TIERS[tier.min(CARAVAN_TIERS.len() - 1)];
            let pct = calc_progress(start_time, t.escort_time_min);
            out.push_str(&format!(
                "🚛 当前押运: {} {} (HP: {}/{})\n",
                t.icon, t.name, current_hp, t.max_hp
            ));
            out.push_str(&format!("   进度: {} {}%\n", progress_bar(pct, 20), pct));
            out.push_str(&format!("   被劫次数: {}\n", rob_attempts));
            if pct >= 100 {
                out.push_str("   ✅ 押运已完成! 输入「完成押运」领取奖励\n");
            }
        }
    }

    out.push_str("\n💡 帮助: 镖局帮助\n");
    out
}

/// 接受押运
pub fn cmd_accept_escort(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let tier_idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= CARAVAN_TIERS.len() => n - 1,
        _ => return "❌ 请输入镖车等级 (1-5)\n💡 示例: 接受押运 1".to_string(),
    };

    let tier = &CARAVAN_TIERS[tier_idx];
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);

    if level < tier.level_req {
        return format!("❌ 等级不足! {}需要{}级，你当前{}级", tier.name, tier.level_req, level);
    }

    // 检查是否已有进行中的押运
    if let Some((_, _, _, _, status)) = parse_escort_record(db, user_id) {
        if status == "active" {
            return "❌ 你已有进行中的押运，请先完成或放弃当前押运".to_string();
        }
    }

    // 检查每日次数
    let daily = get_daily_count(db, user_id);
    if daily >= MAX_DAILY_ESCORT {
        return format!(
            "❌ 今日押运次数已用完({}/{})，明天再来吧",
            MAX_DAILY_ESCORT, MAX_DAILY_ESCORT
        );
    }

    // 扣除费用
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < tier.gold_cost {
        return format!("❌ 金币不足! 需要{}金，你只有{}金", tier.gold_cost, gold);
    }
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, tier.gold_cost);
    if tier.diamond_cost > 0 {
        let diamonds = db.read_currency(user_id, CURRENCY_DIAMOND);
        if diamonds < tier.diamond_cost {
            // 退还金币
            let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, tier.gold_cost);
            return format!("❌ 钻石不足! 需要{}💎，你只有{}💎", tier.diamond_cost, diamonds);
        }
        let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, tier.diamond_cost);
    }

    save_escort_record(db, user_id, tier_idx, now_secs(), tier.max_hp, 0, "active");
    inc_daily_count(db, user_id);

    format!(
        "✅ 接受押运成功!\n\n{} {} 已出发\n📦 耐久: {}/{}HP\n⏰ 预计{}分钟后到达\n💰 费用: {}金{}\n\n💡 输入「押运状态」查看进度",
        tier.icon,
        tier.name,
        tier.max_hp,
        tier.max_hp,
        tier.escort_time_min,
        tier.gold_cost,
        if tier.diamond_cost > 0 {
            format!("+{}💎", tier.diamond_cost)
        } else {
            String::new()
        }
    )
}

/// 押运状态
pub fn cmd_escort_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier_idx, start_time, current_hp, rob_attempts, _status) = match parse_escort_record(db, user_id) {
        Some(r) if r.4 == "active" => r,
        _ => return "📭 当前没有进行中的押运\n💡 输入「查看镖局」选择镖车".to_string(),
    };

    let tier = &CARAVAN_TIERS[tier_idx.min(CARAVAN_TIERS.len() - 1)];
    let pct = calc_progress(start_time, tier.escort_time_min);
    let elapsed = now_secs().saturating_sub(start_time);
    let remain_secs = (tier.escort_time_min * 60).saturating_sub(elapsed);
    let remain_min = remain_secs / 60;
    let remain_sec = remain_secs % 60;

    let hp_pct = if tier.max_hp > 0 {
        (current_hp as f64 / tier.max_hp as f64 * 100.0) as u32
    } else {
        0
    };
    let hp_bar = progress_bar(hp_pct.min(100), 15);

    let mut out = String::from("╔══════════════════════════╗\n");
    out.push_str("║       🚛 押运状态 🚛      ║\n");
    out.push_str("╚══════════════════════════╝\n\n");
    out.push_str(&format!("{} {}\n\n", tier.icon, tier.name));
    out.push_str(&format!("📦 耐久: {}/{}HP {}\n", current_hp, tier.max_hp, hp_bar));
    out.push_str(&format!("⏳ 进度: {} {}%\n", progress_bar(pct, 20), pct));
    out.push_str(&format!("⏰ 剩余: {}分{}秒\n", remain_min, remain_sec));
    out.push_str(&format!("⚔️ 被劫次数: {}\n\n", rob_attempts));

    if pct >= 100 {
        out.push_str("✅ 押运已完成! 输入「完成押运」领取奖励\n");
    } else if current_hp <= 0 {
        out.push_str("❌ 镖车已被毁! 输入「放弃押运」结束\n");
    } else {
        out.push_str("🚛 押运进行中...\n");
    }

    out.push_str("\n💡 护送货物 | 放弃押运\n");
    out
}

/// 护送货物(加速押运进度)
pub fn cmd_escort_escort(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier_idx, mut start_time, mut current_hp, mut rob_attempts, _status) = match parse_escort_record(db, user_id) {
        Some(r) if r.4 == "active" => r,
        _ => return "📭 当前没有进行中的押运".to_string(),
    };

    let tier = &CARAVAN_TIERS[tier_idx.min(CARAVAN_TIERS.len() - 1)];

    if current_hp <= 0 {
        return "❌ 镖车已被毁，无法继续护送".to_string();
    }

    let pct = calc_progress(start_time, tier.escort_time_min);
    if pct >= 100 {
        return "✅ 押运已完成! 输入「完成押运」领取奖励".to_string();
    }

    // 护送动作: 加速10%进度
    let acceleration = tier.escort_time_min * 6;
    start_time = start_time.saturating_sub(acceleration);

    // 随机遭遇
    let mut rng = rand::thread_rng();
    let event_roll: u32 = rng.gen_range(0..100);
    let mut msg = String::from("🚛 你加速护送货物...\n");

    if event_roll < tier.risk_pct {
        // 遭遇劫匪
        let damage = rng.gen_range(20..=80);
        current_hp = (current_hp - damage).max(0);
        rob_attempts += 1;
        msg.push_str(&format!("⚔️ 遭遇山贼袭击! 镖车受到{}点伤害\n", damage));
        if current_hp <= 0 {
            msg.push_str("❌ 镖车被毁! 押运失败\n");
            save_escort_record(db, user_id, tier_idx, start_time, current_hp, rob_attempts, "failed");
            return msg;
        }
        msg.push_str(&format!("📦 剩余耐久: {}/{}HP\n", current_hp, tier.max_hp));
    } else if event_roll < tier.risk_pct + 20 {
        let heal = rng.gen_range(10..=50);
        current_hp = (current_hp + heal).min(tier.max_hp);
        msg.push_str(&format!("🍀 途经补给站! 镖车恢复{}点耐久\n", heal));
    } else {
        msg.push_str("✅ 路途顺利，没有遇到意外\n");
    }

    save_escort_record(db, user_id, tier_idx, start_time, current_hp, rob_attempts, "active");

    let new_pct = calc_progress(start_time, tier.escort_time_min);
    msg.push_str(&format!(
        "⏳ 进度: {} {}%\n",
        progress_bar(new_pct, 20),
        new_pct.min(100)
    ));

    if new_pct >= 100 {
        msg.push_str("\n✅ 押运已完成! 输入「完成押运」领取奖励\n");
    }

    msg
}

/// 完成押运(领取奖励)
pub fn cmd_complete_escort(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier_idx, start_time, current_hp, rob_attempts, _status) = match parse_escort_record(db, user_id) {
        Some(r) if r.4 == "active" => r,
        _ => return "📭 当前没有进行中的押运".to_string(),
    };

    let tier = &CARAVAN_TIERS[tier_idx.min(CARAVAN_TIERS.len() - 1)];

    if current_hp <= 0 {
        return "❌ 镖车已被毁，无法完成押运\n💡 输入「放弃押运」结束".to_string();
    }

    let pct = calc_progress(start_time, tier.escort_time_min);
    if pct < 100 {
        return format!("❌ 押运尚未完成! 当前进度{}%\n💡 输入「护送货物」加速押运", pct);
    }

    // 计算奖励(耐久越高奖励越多)
    let bonus = calc_bonus_multiplier(db, user_id);
    let hp_ratio = current_hp as f64 / tier.max_hp as f64;
    let gold_reward = (tier.base_reward_gold as f64 * hp_ratio * bonus) as i64;
    let exp_reward = (tier.base_reward_exp as f64 * hp_ratio * bonus) as i32;
    let rep_reward = 5 + tier_idx as i32 * 3;

    // 发放奖励
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_reward);
    user::add_experience(db, user_id, exp_reward);

    // 声望积分
    let rep: i32 = db.global_get(SECTION_REP, user_id).parse().unwrap_or(0);
    db.global_set(SECTION_REP, user_id, &(rep + rep_reward).to_string());

    // 标记完成
    save_escort_record(db, user_id, tier_idx, start_time, current_hp, rob_attempts, "completed");

    // 统计
    let total_key = format!("{}_total", user_id);
    let total_escorts: i32 = db.global_get(SECTION_STATS, &total_key).parse().unwrap_or(0);
    db.global_set(SECTION_STATS, &total_key, &(total_escorts + 1).to_string());

    let gold_key = format!("{}_gold", user_id);
    let total_gold: i64 = db.global_get(SECTION_STATS, &gold_key).parse().unwrap_or(0);
    db.global_set(SECTION_STATS, &gold_key, &(total_gold + gold_reward).to_string());

    let mut out = String::from("🎉 ═══ 押运完成! ═══ 🎉\n\n");
    out.push_str(&format!("{} {} 安全到达!\n\n", tier.icon, tier.name));
    out.push_str(&format!("💰 金币: +{}\n", gold_reward));
    out.push_str(&format!("✨ 经验: +{}\n", exp_reward));
    out.push_str(&format!("📿 声望: +{}\n", rep_reward));
    out.push_str(&format!(
        "📦 最终耐久: {}/{}HP ({}%奖励)\n",
        current_hp,
        tier.max_hp,
        (hp_ratio * 100.0) as i32
    ));
    out.push_str(&format!("⚔️ 被劫次数: {}\n", rob_attempts));

    if hp_ratio >= 1.0 {
        out.push_str("\n🏆 完美押运! 全程无损伤!\n");
    }

    out.push_str("\n💡 输入「查看镖局」继续押运\n");
    out
}

/// 放弃押运
pub fn cmd_abandon_escort(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier_idx, start_time, current_hp, rob_attempts, _status) = match parse_escort_record(db, user_id) {
        Some(r) if r.4 == "active" => r,
        _ => return "📭 当前没有进行中的押运".to_string(),
    };

    let tier = &CARAVAN_TIERS[tier_idx.min(CARAVAN_TIERS.len() - 1)];

    // 退还部分费用
    let refund_gold = tier.gold_cost / 4;
    let refund_diamond = tier.diamond_cost / 4;
    if refund_gold > 0 {
        let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, refund_gold);
    }
    if refund_diamond > 0 {
        let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, refund_diamond);
    }

    save_escort_record(db, user_id, tier_idx, start_time, current_hp, rob_attempts, "failed");

    let mut out = String::from("❌ 已放弃押运\n\n");
    out.push_str(&format!("{} {} 押运已取消\n", tier.icon, tier.name));
    if refund_gold > 0 || refund_diamond > 0 {
        out.push_str("\n💸 退还费用(25%):\n");
        if refund_gold > 0 {
            out.push_str(&format!("   💰 金币: +{}\n", refund_gold));
        }
        if refund_diamond > 0 {
            out.push_str(&format!("   💎 钻石: +{}\n", refund_diamond));
        }
    }
    out.push_str("\n💡 输入「查看镖局」重新接镖\n");
    out
}

/// 劫镖(攻击他人镖车)
pub fn cmd_rob_escort(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let target = args.trim();
    if target.is_empty() {
        return "❌ 请输入要劫镖的目标ID\n💡 示例: 劫镖 10001".to_string();
    }
    if target == user_id {
        return "❌ 不能劫自己的镖".to_string();
    }

    // 检查目标是否有进行中的押运
    let (tier_idx, start_time, target_hp, target_robs, _target_status) = match parse_escort_record(db, target) {
        Some(r) if r.4 == "active" => r,
        _ => return "❌ 目标没有进行中的押运".to_string(),
    };

    let tier = &CARAVAN_TIERS[tier_idx.min(CARAVAN_TIERS.len() - 1)];

    // 每日劫镖限制
    let rob_count = get_rob_count(db, user_id);
    if rob_count >= MAX_DAILY_ROB {
        return format!("❌ 今日劫镖次数已用完({}/{})", MAX_DAILY_ROB, MAX_DAILY_ROB);
    }

    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let target_level: i32 = db.read_basic(target, ITEM_LEVEL).parse().unwrap_or(1);
    if level < target_level - 10 {
        return format!("❌ 等级差距过大! 目标{}级，你{}级", target_level, level);
    }

    // 邪恶值检查
    let evil: i32 = db.global_get(SECTION_EVIL, user_id).parse().unwrap_or(0);
    if evil >= 100 {
        return "❌ 你的邪恶值过高(≥100)，镖局拒绝为你服务".to_string();
    }

    // 劫镖战斗模拟
    let atk: i32 = db.read_basic(user_id, ITEM_AD).parse().unwrap_or(10);
    let def: i32 = db.read_basic(target, ITEM_DEFENSE).parse().unwrap_or(5);
    let mut rng = rand::thread_rng();
    let roll: i32 = rng.gen_range(0..100);
    let success = roll < 50 + (atk - def).clamp(-30, 30);

    // 增加邪恶值
    let new_evil = (evil + 10).min(200);
    db.global_set(SECTION_EVIL, user_id, &new_evil.to_string());

    // 记录劫镖次数
    inc_rob_count(db, user_id);

    let mut out = String::new();

    if success {
        let damage = rng.gen_range(50..=200);
        let new_hp = (target_hp - damage).max(0);
        let new_robs = target_robs + 1;
        save_escort_record(db, target, tier_idx, start_time, new_hp, new_robs, "active");

        let loot = tier.base_reward_gold / 5;
        let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, loot);

        out.push_str("⚔️ ═══ 劫镖成功! ═══ ⚔️\n\n");
        out.push_str(&format!("目标: {}\n", target));
        out.push_str(&format!("{} {} 受到{}点伤害\n", tier.icon, tier.name, damage));
        out.push_str(&format!("📦 目标镖车耐久: {}/{}HP\n", new_hp, tier.max_hp));
        out.push_str(&format!("💰 获得赃物: {}金\n", loot));
        out.push_str(&format!("😈 邪恶值: {}→{}\n", evil, new_evil));
        if new_hp <= 0 {
            out.push_str("\n💥 镖车已被你摧毁!\n");
        }
    } else {
        out.push_str("⚔️ ═══ 劫镖失败! ═══ ⚔️\n\n");
        out.push_str("对方镖师武艺高强，你被击退了\n");
        let penalty = db.read_currency(user_id, CURRENCY_GOLD) / 20;
        if penalty > 0 {
            let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, penalty);
            out.push_str(&format!("💸 损失: {}金\n", penalty));
        }
        out.push_str(&format!("😈 邪恶值: {}→{}\n", evil, new_evil));
    }

    out
}

/// 镖局排行
pub fn cmd_escort_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut entries: Vec<(String, i32, i64)> = Vec::new();

    let users = db.all_users();
    for uid in users {
        let total_key = format!("{}_total", uid);
        let total: i32 = db.global_get(SECTION_STATS, &total_key).parse().unwrap_or(0);
        let gold_key = format!("{}_gold", uid);
        let gold: i64 = db.global_get(SECTION_STATS, &gold_key).parse().unwrap_or(0);
        if total > 0 {
            entries.push((uid, total, gold));
        }
    }

    entries.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    entries.truncate(15);

    let mut out = String::from("╔══════════════════════════╗\n");
    out.push_str("║      📦 镖局押运排行 📦    ║\n");
    out.push_str("╚══════════════════════════╝\n\n");
    out.push_str("排名 | 玩家     | 押运次数 | 累计收益\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for (i, (id, total, gold)) in entries.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!(
            "{} {:2} | {:8} | {:6}次 | {}金\n",
            medal,
            i + 1,
            id,
            total,
            gold
        ));
    }

    if entries.is_empty() {
        out.push_str("暂无押运记录\n");
    }

    out
}

/// 镖局帮助
pub fn cmd_escort_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("╔══════════════════════════╗\n");
    out.push_str("║      🏪 镖局押运帮助 🏪    ║\n");
    out.push_str("╚══════════════════════════╝\n\n");
    out.push_str("📜 系统介绍:\n");
    out.push_str("  镖局押运是一个高风险高回报的玩法。\n");
    out.push_str("  选择镖车接受任务，护送货物到达目的地。\n");
    out.push_str("  押运途中可能遭遇劫匪袭击!\n\n");
    out.push_str("📋 指令列表:\n");
    out.push_str("  查看镖局     - 查看可选镖车和当前状态\n");
    out.push_str("  接受押运 N   - 接受第N级镖车(N=1~5)\n");
    out.push_str("  押运状态     - 查看当前押运进度\n");
    out.push_str("  护送货物     - 加速押运(推进10%进度)\n");
    out.push_str("  完成押运     - 押运完成后领取奖励\n");
    out.push_str("  放弃押运     - 放弃当前押运(退25%费用)\n");
    out.push_str("  劫镖 ID      - 劫掠他人镖车(增加邪恶值)\n");
    out.push_str("  镖局排行     - 查看全服押运排行榜\n\n");
    out.push_str("💡 技巧提示:\n");
    out.push_str("  • 镖车品质越高，奖励越丰厚但风险越大\n");
    out.push_str("  • 多次「护送货物」可加速完成押运\n");
    out.push_str("  • 镖车耐久越高，最终奖励越多\n");
    out.push_str("  • VIP等级提供额外奖励加成(每级+5%)\n");
    out.push_str("  • 劫镖会增加邪恶值，注意控制!\n");
    out.push_str("  • 每日最多押运5次、劫镖3次\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caravan_tiers_count() {
        assert_eq!(CARAVAN_TIERS.len(), 5);
    }

    #[test]
    fn test_caravan_tiers_sorted_by_level() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].level_req > CARAVAN_TIERS[i - 1].level_req);
        }
    }

    #[test]
    fn test_caravan_tiers_sorted_by_cost() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].gold_cost > CARAVAN_TIERS[i - 1].gold_cost);
        }
    }

    #[test]
    fn test_caravan_tiers_sorted_by_reward() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].base_reward_gold > CARAVAN_TIERS[i - 1].base_reward_gold);
        }
    }

    #[test]
    fn test_caravan_tiers_have_names() {
        for t in CARAVAN_TIERS {
            assert!(!t.name.is_empty());
            assert!(!t.icon.is_empty());
        }
    }

    #[test]
    fn test_caravan_tiers_escort_time_escalates() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].escort_time_min > CARAVAN_TIERS[i - 1].escort_time_min);
        }
    }

    #[test]
    fn test_caravan_tiers_hp_escalates() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].max_hp > CARAVAN_TIERS[i - 1].max_hp);
        }
    }

    #[test]
    fn test_caravan_tiers_risk_in_range() {
        for t in CARAVAN_TIERS {
            assert!(t.risk_pct > 0 && t.risk_pct <= 50);
        }
    }

    #[test]
    fn test_progress_bar_empty() {
        assert_eq!(progress_bar(0, 10), "[░░░░░░░░░░]");
    }

    #[test]
    fn test_progress_bar_full() {
        assert_eq!(progress_bar(100, 10), "[██████████]");
    }

    #[test]
    fn test_progress_bar_half() {
        assert_eq!(progress_bar(50, 10), "[█████░░░░░]");
    }

    #[test]
    fn test_progress_bar_overflow() {
        assert_eq!(progress_bar(150, 10), "[██████████]");
    }

    #[test]
    fn test_chrono_like_today_format() {
        let today = chrono_like_today();
        assert!(today.starts_with("day"));
    }

    #[test]
    fn test_section_names() {
        assert_eq!(SECTION, "escort");
        assert_eq!(SECTION_DAILY, "escort_daily");
        assert_eq!(SECTION_STATS, "escort_stats");
        assert_eq!(SECTION_REP, "escort_rep");
    }

    #[test]
    fn test_tier_diamond_costs() {
        assert_eq!(CARAVAN_TIERS[0].diamond_cost, 0);
        assert!(CARAVAN_TIERS[4].diamond_cost > CARAVAN_TIERS[3].diamond_cost);
    }

    #[test]
    fn test_tier_risk_escalates() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].risk_pct >= CARAVAN_TIERS[i - 1].risk_pct);
        }
    }

    #[test]
    fn test_tier_reward_exp_escalates() {
        for i in 1..CARAVAN_TIERS.len() {
            assert!(CARAVAN_TIERS[i].base_reward_exp > CARAVAN_TIERS[i - 1].base_reward_exp);
        }
    }

    #[test]
    fn test_max_daily_limits() {
        assert_eq!(MAX_DAILY_ESCORT, 5);
        assert_eq!(MAX_DAILY_ROB, 3);
    }

    #[test]
    fn test_calc_progress_zero_duration() {
        assert_eq!(calc_progress(0, 0), 100);
    }
}

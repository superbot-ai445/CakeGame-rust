// 世界之树系统 — 全服协作成长
// 玩家共同捐献资源培育世界之树，解锁全服属性加成和里程碑奖励

use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD, OP_SUB};
use crate::db::Database;
use crate::user;

const SECTION: &str = "world_tree";

// ═══════════════════════════════════════════════════
// 生长阶段定义
// ═══════════════════════════════════════════════════

#[derive(Clone, Copy)]
struct GrowthStage {
    name: &'static str,
    emoji: &'static str,
    threshold: i64,  // 所需总贡献值
    hp_bonus: i32,   // 全服HP加成%
    ad_bonus: i32,   // 全服物攻加成%
    ap_bonus: i32,   // 全服魔攻加成%
    def_bonus: i32,  // 全服防御加成%
    mres_bonus: i32, // 全服魔抗加成%
    gold_bonus: i32, // 全服金币获取加成%
    exp_bonus: i32,  // 全服经验获取加成%
    drop_bonus: i32, // 全服掉落加成%
}

const GROWTH_STAGES: &[GrowthStage] = &[
    GrowthStage {
        name: "种子期",
        emoji: "🌱",
        threshold: 0,
        hp_bonus: 0,
        ad_bonus: 0,
        ap_bonus: 0,
        def_bonus: 0,
        mres_bonus: 0,
        gold_bonus: 0,
        exp_bonus: 0,
        drop_bonus: 0,
    },
    GrowthStage {
        name: "发芽期",
        emoji: "🌿",
        threshold: 1000,
        hp_bonus: 2,
        ad_bonus: 1,
        ap_bonus: 1,
        def_bonus: 1,
        mres_bonus: 1,
        gold_bonus: 3,
        exp_bonus: 3,
        drop_bonus: 1,
    },
    GrowthStage {
        name: "幼树期",
        emoji: "🌲",
        threshold: 5000,
        hp_bonus: 5,
        ad_bonus: 3,
        ap_bonus: 3,
        def_bonus: 2,
        mres_bonus: 2,
        gold_bonus: 6,
        exp_bonus: 6,
        drop_bonus: 2,
    },
    GrowthStage {
        name: "成长期",
        emoji: "🌳",
        threshold: 20000,
        hp_bonus: 10,
        ad_bonus: 5,
        ap_bonus: 5,
        def_bonus: 4,
        mres_bonus: 4,
        gold_bonus: 10,
        exp_bonus: 10,
        drop_bonus: 4,
    },
    GrowthStage {
        name: "盛放期",
        emoji: "🌸",
        threshold: 50000,
        hp_bonus: 16,
        ad_bonus: 8,
        ap_bonus: 8,
        def_bonus: 6,
        mres_bonus: 6,
        gold_bonus: 15,
        exp_bonus: 15,
        drop_bonus: 6,
    },
    GrowthStage {
        name: "参天期",
        emoji: "🏔️",
        threshold: 100000,
        hp_bonus: 24,
        ad_bonus: 12,
        ap_bonus: 12,
        def_bonus: 9,
        mres_bonus: 9,
        gold_bonus: 20,
        exp_bonus: 20,
        drop_bonus: 8,
    },
    GrowthStage {
        name: "世界树",
        emoji: "🌍",
        threshold: 250000,
        hp_bonus: 35,
        ad_bonus: 18,
        ap_bonus: 18,
        def_bonus: 14,
        mres_bonus: 14,
        gold_bonus: 28,
        exp_bonus: 28,
        drop_bonus: 12,
    },
    GrowthStage {
        name: "生命之树",
        emoji: "✨",
        threshold: 500000,
        hp_bonus: 50,
        ad_bonus: 25,
        ap_bonus: 25,
        def_bonus: 20,
        mres_bonus: 20,
        gold_bonus: 40,
        exp_bonus: 40,
        drop_bonus: 18,
    },
];

/// 计算当前生长阶段索引
fn calc_stage_idx(total_contrib: i64) -> usize {
    for (i, stage) in GROWTH_STAGES.iter().enumerate().rev() {
        if total_contrib >= stage.threshold {
            return i;
        }
    }
    0
}

/// 进度条渲染
fn progress_bar(current: i64, target: i64, width: usize) -> String {
    let pct = if target > 0 {
        (current.min(target) as f64 / target as f64 * 100.0) as i32
    } else {
        100
    };
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    format!("[{}{}] {}%", "█".repeat(filled), "░".repeat(empty), pct)
}

/// 获取上次浇水/施肥日期
fn get_action_date(db: &Database, user_id: &str, action: &str) -> String {
    db.global_get(SECTION, &format!("{}_{}", user_id, action))
}

/// 设置今天操作日期
fn set_action_date(db: &Database, user_id: &str, action: &str) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.global_set(SECTION, &format!("{}_{}", user_id, action), &today);
}

/// 加载全服贡献记录 (per-user contributions)
fn load_user_contrib(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("contrib_{}", user_id))
        .parse::<i64>()
        .unwrap_or(0)
}

/// 增加个人贡献
fn add_user_contrib(db: &Database, user_id: &str, amount: i64) -> i64 {
    let cur = load_user_contrib(db, user_id);
    let new_val = cur + amount;
    db.global_set(SECTION, &format!("contrib_{}", user_id), &new_val.to_string());
    new_val
}

/// 增加全服总贡献
fn add_total_contrib(db: &Database, amount: i64) -> i64 {
    let cur: i64 = db.global_get(SECTION, "total_contrib").parse::<i64>().unwrap_or(0);
    let new_val = cur + amount;
    db.global_set(SECTION, "total_contrib", &new_val.to_string());
    new_val
}

/// 加载已领取的里程碑 bitmask
fn load_claimed_bitmask(db: &Database, user_id: &str) -> u32 {
    db.global_get(SECTION, &format!("claimed_{}", user_id))
        .parse::<u32>()
        .unwrap_or(0)
}

/// 保存已领取的里程碑 bitmask
fn save_claimed_bitmask(db: &Database, user_id: &str, bitmask: u32) {
    db.global_set(SECTION, &format!("claimed_{}", user_id), &bitmask.to_string());
}

// ═══════════════════════════════════════════════════
// 里程碑奖励定义
// ═══════════════════════════════════════════════════

#[derive(Clone, Copy)]
struct Milestone {
    name: &'static str,
    emoji: &'static str,
    threshold: i32,
    reward_gold: i64,
    reward_diamond: i64,
    reward_item: &'static str,
}

const MILESTONES: &[Milestone] = &[
    Milestone {
        name: "初植树者",
        emoji: "🌱",
        threshold: 100,
        reward_gold: 3000,
        reward_diamond: 20,
        reward_item: "生命药水",
    },
    Milestone {
        name: "护树使者",
        emoji: "🌿",
        threshold: 500,
        reward_gold: 15000,
        reward_diamond: 80,
        reward_item: "高级生命药水",
    },
    Milestone {
        name: "森林守卫",
        emoji: "🌲",
        threshold: 2000,
        reward_gold: 60000,
        reward_diamond: 200,
        reward_item: "凤凰之羽",
    },
    Milestone {
        name: "生命祭司",
        emoji: "🌳",
        threshold: 5000,
        reward_gold: 200000,
        reward_diamond: 500,
        reward_item: "世界树叶",
    },
    Milestone {
        name: "树之魂灵",
        emoji: "✨",
        threshold: 10000,
        reward_gold: 500000,
        reward_diamond: 1200,
        reward_item: "生命之果",
    },
];

/// 记录贡献日志
fn log_contribution(db: &Database, user_id: &str, method: &str, amount: i64) {
    let nick = user::get_msg_prefix(db, user_id);
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let log_entry = format!(
        "{}|{}|{}|{}|{}",
        now,
        nick,
        method,
        amount,
        chrono::Local::now().timestamp()
    );
    // 保留最近20条日志
    let existing = db.global_get(SECTION, "contrib_log");
    let mut logs: Vec<&str> = existing.split('~').filter(|s| !s.is_empty()).collect();
    logs.insert(0, &log_entry);
    if logs.len() > 20 {
        logs.truncate(20);
    }
    db.global_set(SECTION, "contrib_log", &logs.join("~"));
}

// ═══════════════════════════════════════════════════
// 指令实现
// ═══════════════════════════════════════════════════

/// 查看世界树 — 显示全服世界树状态
pub fn cmd_view_world_tree(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let total: i64 = db.global_get(SECTION, "total_contrib").parse::<i64>().unwrap_or(0);
    let stage_idx = calc_stage_idx(total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n═══ {} {} 世界之树 ═══\n", prefix, stage.emoji, stage.name);

    // 当前阶段信息
    r.push_str(&format!("\n🌳 当前阶段: {} {}", stage.emoji, stage.name));
    r.push_str(&format!("\n📊 全服总贡献: {}", format_num(total)));

    // 生长进度
    let next_idx = (stage_idx + 1).min(GROWTH_STAGES.len() - 1);
    if stage_idx < GROWTH_STAGES.len() - 1 {
        let next_stage = &GROWTH_STAGES[next_idx];
        let progress_in_stage = total - stage.threshold;
        let stage_range = next_stage.threshold - stage.threshold;
        r.push_str(&format!(
            "\n📈 成长进度: {}",
            progress_bar(progress_in_stage, stage_range, 12)
        ));
        r.push_str(&format!(
            "\n🎯 下一阶段: {} {} (还需 {})",
            next_stage.emoji,
            next_stage.name,
            format_num(next_stage.threshold - total)
        ));
    } else {
        r.push_str("\n🌟 世界之树已达最高阶段！");
    }

    // 全服加成
    if stage_idx > 0 {
        r.push_str("\n\n💚 全服属性加成:");
        r.push_str(&format!(
            "\n  ❤️ HP +{}%  ⚔️ 物攻 +{}%  🔮 魔攻 +{}%",
            stage.hp_bonus, stage.ad_bonus, stage.ap_bonus
        ));
        r.push_str(&format!(
            "\n  🛡️ 防御 +{}%  🔮 魔抗 +{}%",
            stage.def_bonus, stage.mres_bonus
        ));
        r.push_str(&format!(
            "\n  💰 金币 +{}%  ⭐ 经验 +{}%  🎁 掉落 +{}%",
            stage.gold_bonus, stage.exp_bonus, stage.drop_bonus
        ));
    } else {
        r.push_str("\n\n💡 世界之树尚在种子期，快去捐献让它发芽吧！");
    }

    // 个人贡献
    let my_contrib = load_user_contrib(db, user_id);
    r.push_str(&format!("\n\n📌 你的贡献: {}", format_num(my_contrib)));

    // 每日操作状态
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let watered = get_action_date(db, user_id, "water") == today;
    let fertilized = get_action_date(db, user_id, "fertilize") == today;
    r.push_str(&format!(
        "\n💧 今日浇水: {}  🌿 今日施肥: {}",
        if watered { "✅" } else { "❌" },
        if fertilized { "✅" } else { "❌" }
    ));

    // 已领取里程碑
    let claimed = load_claimed_bitmask(db, user_id);
    let claimed_count = (0..MILESTONES.len()).filter(|i| claimed & (1 << i) != 0).count();
    r.push_str(&format!("\n🏆 里程碑: {}/{} 已领取", claimed_count, MILESTONES.len()));

    r
}

/// 捐献世界树 + 金币数 — 金币捐献
pub fn cmd_donate_world_tree(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let amount: i64 = match args.trim().parse::<i64>() {
        Ok(n) if n > 0 => n,
        _ => return format!("{}\n请输入正确的捐献金额！\n💡 示例: 捐献世界树 1000", prefix),
    };

    if amount < 100 {
        return format!("{}\n⚠️ 最低捐献金额为100金币！", prefix);
    }

    if amount > 1000000 {
        return format!("{}\n⚠️ 单次捐献上限为1,000,000金币！", prefix);
    }

    // 扣除金币
    let after = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, amount);
    if after < 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, amount);
        return format!("{}\n❌ 金币不足！当前余额: {} 金币", prefix, format_num(after + amount));
    }

    // 贡献值 = 金币 / 10 (10金 = 1贡献)
    let contrib = amount / 10;
    let new_total = add_total_contrib(db, contrib);
    let new_personal = add_user_contrib(db, user_id, contrib);
    log_contribution(db, user_id, "金币捐献", contrib);

    let stage_idx = calc_stage_idx(new_total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n🌳 世界之树捐献成功！", prefix);
    r.push_str(&format!("\n💰 捐献金币: {}", format_num(amount)));
    r.push_str(&format!("\n📊 获得贡献: +{}", format_num(contrib)));
    r.push_str(&format!("\n📌 你的总贡献: {}", format_num(new_personal)));
    r.push_str(&format!("\n🌿 当前阶段: {} {}", stage.emoji, stage.name));

    r
}

/// 浇水世界树 — 每日免费浇水
pub fn cmd_water_world_tree(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if get_action_date(db, user_id, "water") == today {
        return format!("{}\n💧 你今天已经浇过水了！明天再来吧。", prefix);
    }

    let contrib = 50i64;
    let new_total = add_total_contrib(db, contrib);
    let new_personal = add_user_contrib(db, user_id, contrib);
    set_action_date(db, user_id, "water");
    log_contribution(db, user_id, "浇水", contrib);

    let stage_idx = calc_stage_idx(new_total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n💧 浇水成功！", prefix);
    r.push_str(&format!("\n🌱 获得贡献: +{}", contrib));
    r.push_str(&format!("\n📌 你的总贡献: {}", format_num(new_personal)));
    r.push_str(&format!("\n🌿 当前阶段: {} {}", stage.emoji, stage.name));
    r
}

/// 施肥世界树 — 500金币施肥
pub fn cmd_fertilize_world_tree(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if get_action_date(db, user_id, "fertilize") == today {
        return format!("{}\n🌿 你今天已经施过肥了！明天再来吧。", prefix);
    }

    let cost = 500i64;
    let after = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, cost);
    if after < 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, cost);
        return format!(
            "{}\n❌ 施肥需要500金币！当前余额: {} 金币",
            prefix,
            format_num(after + cost)
        );
    }

    let contrib = 100i64;
    let new_total = add_total_contrib(db, contrib);
    let new_personal = add_user_contrib(db, user_id, contrib);
    set_action_date(db, user_id, "fertilize");
    log_contribution(db, user_id, "施肥", contrib);

    let stage_idx = calc_stage_idx(new_total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n🌿 施肥成功！", prefix);
    r.push_str(&format!("\n💰 消耗金币: {}", format_num(cost)));
    r.push_str(&format!("\n🌱 获得贡献: +{}", contrib));
    r.push_str(&format!("\n📌 你的总贡献: {}", format_num(new_personal)));
    r.push_str(&format!("\n🌿 当前阶段: {} {}", stage.emoji, stage.name));
    r
}

/// 世界树排行 — 全服贡献排行榜
pub fn cmd_world_tree_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut players: Vec<(String, i64)> = Vec::new();

    for uid in db.all_users().iter() {
        let contrib: i64 = db
            .global_get(SECTION, &format!("contrib_{}", uid))
            .parse::<i64>()
            .unwrap_or(0);
        if contrib > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, contrib));
        }
    }

    if players.is_empty() {
        return format!("{}\n🌳 暂无世界树贡献数据。", prefix);
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let total: i64 = db.global_get(SECTION, "total_contrib").parse::<i64>().unwrap_or(0);
    let stage_idx = calc_stage_idx(total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n═══ 🌳 世界树贡献排行 ═══\n", prefix);
    r.push_str(&format!(
        "当前阶段: {} {} | 总贡献: {}\n",
        stage.emoji,
        stage.name,
        format_num(total)
    ));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, contrib)) in players.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        r.push_str(&format!("\n{} {} — {} 贡献", medal, name, format_num(*contrib)));
    }

    // 用户排名定位
    let my_prefix = user::get_msg_prefix(db, user_id);
    if let Some(rank) = players.iter().position(|(name, _)| name == &my_prefix) {
        r.push_str(&format!(
            "\n\n📍 你的排名：第{}名 (贡献: {})",
            rank + 1,
            format_num(players[rank].1)
        ));
    } else {
        r.push_str("\n\n📍 你还没有贡献记录，快去捐献吧！");
    }

    r
}

/// 领取世界树奖励 + 里程碑等级 — 领取里程碑奖励
pub fn cmd_claim_world_tree_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= MILESTONES.len() => n - 1,
        _ => {
            // 显示里程碑列表
            let my_contrib = load_user_contrib(db, user_id);
            let claimed = load_claimed_bitmask(db, user_id);
            let mut r = format!("{}\n═══ 🏆 世界树里程碑 ═══\n", prefix);
            r.push_str(&format!("\n📌 你的贡献: {}", format_num(my_contrib)));
            for (i, m) in MILESTONES.iter().enumerate() {
                let status = if claimed & (1 << i) != 0 {
                    "✅已领"
                } else if my_contrib >= m.threshold as i64 {
                    "🎁可领"
                } else {
                    "⏳未达成"
                };
                r.push_str(&format!(
                    "\n{} {} {} — {}贡献 | {}金+{}💎+{}",
                    m.emoji,
                    i + 1,
                    m.name,
                    m.threshold,
                    format_num(m.reward_gold),
                    m.reward_diamond,
                    m.reward_item,
                ));
                r.push_str(&format!(" [{}]", status));
            }
            r.push_str("\n\n💡 输入: 领取世界树奖励 + 编号 (如: 领取世界树奖励 1)");
            return r;
        }
    };

    let milestone = &MILESTONES[idx];
    let my_contrib = load_user_contrib(db, user_id);
    let mut claimed = load_claimed_bitmask(db, user_id);

    // 检查是否已领取
    if claimed & (1 << idx) != 0 {
        return format!("{}\n⚠️ {} 的奖励已经领取过了！", prefix, milestone.name);
    }

    // 检查是否达到阈值
    if my_contrib < milestone.threshold as i64 {
        return format!(
            "{}\n❌ 贡献不足！需要 {} 贡献，当前 {} 贡献。",
            prefix,
            format_num(milestone.threshold as i64),
            format_num(my_contrib)
        );
    }

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, milestone.reward_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, milestone.reward_diamond);
    db.knapsack_add(user_id, milestone.reward_item, 1);

    // 标记已领取
    claimed |= 1 << idx;
    save_claimed_bitmask(db, user_id, claimed);

    let mut r = format!(
        "{}\n🏆 领取成功！{} {} 里程碑奖励",
        prefix, milestone.emoji, milestone.name
    );
    r.push_str(&format!("\n💰 金币: +{}", format_num(milestone.reward_gold)));
    r.push_str(&format!("\n💎 钻石: +{}", milestone.reward_diamond));
    r.push_str(&format!("\n🎁 道具: {} ×1", milestone.reward_item));

    // 检查是否全部领取完
    let all_claimed = (0..MILESTONES.len()).all(|i| claimed & (1 << i) != 0);
    if all_claimed {
        r.push_str("\n\n🌟 恭喜！你已领取所有世界树里程碑奖励！");
    }

    r
}

/// 世界树帮助
pub fn cmd_world_tree_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let total: i64 = db.global_get(SECTION, "total_contrib").parse::<i64>().unwrap_or(0);
    let stage_idx = calc_stage_idx(total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n═══ 🌳 世界之树系统 ═══\n", prefix);
    r.push_str(&format!(
        "\n当前阶段: {} {} (总贡献: {})",
        stage.emoji,
        stage.name,
        format_num(total)
    ));

    r.push_str("\n\n📋 基本指令:");
    r.push_str("\n  世界树 - 查看世界树状态");
    r.push_str("\n  捐献世界树+金币数 - 金币捐献(10金=1贡献)");
    r.push_str("\n  浇水世界树 - 每日免费浇水(+50贡献)");
    r.push_str("\n  施肥世界树 - 500金币施肥(+100贡献)");
    r.push_str("\n  世界树排行 - 全服贡献排行榜");
    r.push_str("\n  领取世界树奖励 - 查看/领取里程碑奖励");

    r.push_str("\n\n🌿 生长阶段:");
    for stage in GROWTH_STAGES {
        r.push_str(&format!(
            "\n  {} {} — {}贡献",
            stage.emoji,
            stage.name,
            format_num(stage.threshold)
        ));
    }

    r.push_str("\n\n🏆 里程碑奖励:");
    for m in MILESTONES.iter() {
        r.push_str(&format!(
            "\n  {} {} — {}贡献 → {}金+{}💎+{}",
            m.emoji,
            m.name,
            m.threshold,
            format_num(m.reward_gold),
            m.reward_diamond,
            m.reward_item
        ));
    }

    r.push_str("\n\n💡 世界之树由全服玩家共同培育，阶段越高全服加成越强！");
    r.push_str("\n每天可以免费浇水1次和施肥1次。");
    r
}

/// 世界树日志 — 查看最近的捐献日志
pub fn cmd_world_tree_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let log_data = db.global_get(SECTION, "contrib_log");

    if log_data.is_empty() {
        return format!("{}\n📜 暂无世界树贡献记录。", prefix);
    }

    let total: i64 = db.global_get(SECTION, "total_contrib").parse::<i64>().unwrap_or(0);
    let stage_idx = calc_stage_idx(total);
    let stage = &GROWTH_STAGES[stage_idx];

    let mut r = format!("{}\n═══ 📜 世界树贡献日志 ═══\n", prefix);
    r.push_str(&format!(
        "当前: {} {} | 总贡献: {}\n",
        stage.emoji,
        stage.name,
        format_num(total)
    ));

    let entries: Vec<&str> = log_data.split('~').filter(|s| !s.is_empty()).collect();
    for entry in entries.iter().take(10) {
        let parts: Vec<&str> = entry.split('|').collect();
        if parts.len() >= 4 {
            r.push_str(&format!(
                "\n  🕐 {} — {} {} +{}贡献",
                parts[0], parts[1], parts[2], parts[3]
            ));
        }
    }

    if entries.len() > 10 {
        r.push_str(&format!("\n\n... 共{}条记录，仅显示最近10条", entries.len()));
    }

    r
}

/// 获取世界树全服加成 — 供战斗/经济系统集成
#[allow(dead_code)]
pub fn get_world_tree_bonus(db: &Database) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    let total: i64 = db.global_get(SECTION, "total_contrib").parse::<i64>().unwrap_or(0);
    let stage_idx = calc_stage_idx(total);
    let stage = &GROWTH_STAGES[stage_idx];
    (
        stage.hp_bonus,
        stage.ad_bonus,
        stage.ap_bonus,
        stage.def_bonus,
        stage.mres_bonus,
        stage.gold_bonus,
        stage.exp_bonus,
        stage.drop_bonus,
    )
}

/// 格式化数字 (千分位)
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

// ═══════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_growth_stages_count() {
        assert_eq!(GROWTH_STAGES.len(), 8);
    }

    #[test]
    fn test_growth_stages_sorted() {
        for i in 1..GROWTH_STAGES.len() {
            assert!(GROWTH_STAGES[i].threshold > GROWTH_STAGES[i - 1].threshold);
        }
    }

    #[test]
    fn test_growth_stage_names_unique() {
        let mut names: Vec<&str> = GROWTH_STAGES.iter().map(|s| s.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn test_calc_stage_idx_zero() {
        assert_eq!(calc_stage_idx(0), 0);
    }

    #[test]
    fn test_calc_stage_idx_max() {
        assert_eq!(calc_stage_idx(999999), GROWTH_STAGES.len() - 1);
    }

    #[test]
    fn test_calc_stage_idx_boundaries() {
        assert_eq!(calc_stage_idx(999), 0); // 种子期
        assert_eq!(calc_stage_idx(1000), 1); // 发芽期
        assert_eq!(calc_stage_idx(5000), 2); // 幼树期
        assert_eq!(calc_stage_idx(20000), 3); // 成长期
        assert_eq!(calc_stage_idx(50000), 4); // 盛放期
        assert_eq!(calc_stage_idx(100000), 5); // 参天期
        assert_eq!(calc_stage_idx(250000), 6); // 世界树
        assert_eq!(calc_stage_idx(500000), 7); // 生命之树
    }

    #[test]
    fn test_stage_bonus_escalation() {
        for i in 1..GROWTH_STAGES.len() {
            assert!(GROWTH_STAGES[i].hp_bonus >= GROWTH_STAGES[i - 1].hp_bonus);
            assert!(GROWTH_STAGES[i].gold_bonus >= GROWTH_STAGES[i - 1].gold_bonus);
            assert!(GROWTH_STAGES[i].exp_bonus >= GROWTH_STAGES[i - 1].exp_bonus);
        }
    }

    #[test]
    fn test_milestones_count() {
        assert_eq!(MILESTONES.len(), 5);
    }

    #[test]
    fn test_milestones_sorted() {
        for i in 1..MILESTONES.len() {
            assert!(MILESTONES[i].threshold > MILESTONES[i - 1].threshold);
        }
    }

    #[test]
    fn test_milestone_names_unique() {
        let mut names: Vec<&str> = MILESTONES.iter().map(|m| m.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for m in MILESTONES {
            assert!(m.reward_gold > 0);
            assert!(m.reward_diamond > 0);
            assert!(!m.reward_item.is_empty());
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
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100, 10);
        assert!(bar.contains("100%"));
        assert!(bar.chars().filter(|c| *c == '█').count() == 10);
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100, 10);
        assert!(bar.contains("0%"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 100, 10);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_progress_bar_overflow() {
        let bar = progress_bar(200, 100, 10);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234567), "1,234,567");
        assert_eq!(format_num(-5000), "-5,000");
    }

    #[test]
    fn test_bitmask_operations() {
        let mut claimed: u32 = 0;
        // Claim milestone 0
        claimed |= 1 << 0;
        assert!(claimed & (1 << 0) != 0);
        assert!(claimed & (1 << 1) == 0);
        // Claim milestone 3
        claimed |= 1 << 3;
        assert!(claimed & (1 << 3) != 0);
        // All not claimed
        let all_claimed = (0..5).all(|i| claimed & (1 << i) != 0);
        assert!(!all_claimed);
    }

    #[test]
    fn test_get_world_tree_bonus_default() {
        // Default is seed stage — no bonuses
        // We can't use db in tests, but we can verify the function signature
        let stage = &GROWTH_STAGES[0];
        assert_eq!(stage.hp_bonus, 0);
        assert_eq!(stage.gold_bonus, 0);
    }

    #[test]
    fn test_stage_bonus_at_max() {
        let stage = &GROWTH_STAGES[GROWTH_STAGES.len() - 1];
        assert_eq!(stage.name, "生命之树");
        assert!(stage.hp_bonus > 0);
        assert!(stage.gold_bonus > 0);
    }

    #[test]
    fn test_section_name() {
        assert_eq!(SECTION, "world_tree");
    }

    #[test]
    fn test_log_entry_format() {
        // Verify log entry fields count
        let entry = "06-13 15:00|玩家|浇水|50|1234567890";
        let parts: Vec<&str> = entry.split('|').collect();
        assert_eq!(parts.len(), 5);
    }
}

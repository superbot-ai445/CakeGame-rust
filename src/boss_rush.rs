/// Boss Rush 挑战系统
/// 依次挑战多个BOSS的连续战斗模式，难度递增，奖励累积
///
/// 功能：
/// - 查看Boss Rush: 显示所有难度阶段和奖励预览
/// - 开始Boss Rush: 进入连续BOSS战
/// - Boss Rush进度: 查看当前挑战进度
/// - Boss Rush排行: 全服Boss Rush排行
/// - Boss Rush记录: 个人挑战历史
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use crate::user;

/// Boss Rush 阶段定义
#[allow(dead_code)]
struct RushStage {
    name: &'static str,
    boss_name: &'static str,
    boss_hp: i64,
    boss_ad: i32,
    boss_def: i32,
    boss_mr: i32,
    reward_gold: i32,
    reward_diamond: i32,
    reward_exp: i32,
    emoji: &'static str,
}

/// 5个难度阶段，每阶段3个BOSS
const RUSH_TIERS: &[(&str, &[RushStage])] = &[
    (
        "青铜试炼",
        &[
            RushStage {
                name: "第一关",
                boss_name: "哥布林将军",
                boss_hp: 5000,
                boss_ad: 120,
                boss_def: 30,
                boss_mr: 20,
                reward_gold: 500,
                reward_diamond: 5,
                reward_exp: 200,
                emoji: "🟢",
            },
            RushStage {
                name: "第二关",
                boss_name: "巨型史莱姆",
                boss_hp: 8000,
                boss_ad: 160,
                boss_def: 50,
                boss_mr: 30,
                reward_gold: 800,
                reward_diamond: 8,
                reward_exp: 350,
                emoji: "🟢",
            },
            RushStage {
                name: "第三关",
                boss_name: "骷髅骑士",
                boss_hp: 12000,
                boss_ad: 220,
                boss_def: 70,
                boss_mr: 50,
                reward_gold: 1200,
                reward_diamond: 12,
                reward_exp: 500,
                emoji: "🟢",
            },
        ],
    ),
    (
        "白银试炼",
        &[
            RushStage {
                name: "第一关",
                boss_name: "无头骑士",
                boss_hp: 18000,
                boss_ad: 320,
                boss_def: 100,
                boss_mr: 80,
                reward_gold: 2000,
                reward_diamond: 20,
                reward_exp: 800,
                emoji: "🔵",
            },
            RushStage {
                name: "第二关",
                boss_name: "冰霜巨龙",
                boss_hp: 25000,
                boss_ad: 420,
                boss_def: 130,
                boss_mr: 110,
                reward_gold: 3000,
                reward_diamond: 30,
                reward_exp: 1200,
                emoji: "🔵",
            },
            RushStage {
                name: "第三关",
                boss_name: "暗影狼王",
                boss_hp: 35000,
                boss_ad: 550,
                boss_def: 160,
                boss_mr: 140,
                reward_gold: 4000,
                reward_diamond: 40,
                reward_exp: 1600,
                emoji: "🔵",
            },
        ],
    ),
    (
        "黄金试炼",
        &[
            RushStage {
                name: "第一关",
                boss_name: "火焰魔神",
                boss_hp: 50000,
                boss_ad: 700,
                boss_def: 200,
                boss_mr: 180,
                reward_gold: 5000,
                reward_diamond: 50,
                reward_exp: 2000,
                emoji: "🟡",
            },
            RushStage {
                name: "第二关",
                boss_name: "深渊领主",
                boss_hp: 70000,
                boss_ad: 900,
                boss_def: 260,
                boss_mr: 220,
                reward_gold: 7000,
                reward_diamond: 70,
                reward_exp: 2800,
                emoji: "🟡",
            },
            RushStage {
                name: "第三关",
                boss_name: "远古巨像",
                boss_hp: 100000,
                boss_ad: 1100,
                boss_def: 330,
                boss_mr: 280,
                reward_gold: 10000,
                reward_diamond: 100,
                reward_exp: 4000,
                emoji: "🟡",
            },
        ],
    ),
    (
        "钻石试炼",
        &[
            RushStage {
                name: "第一关",
                boss_name: "死亡骑士",
                boss_hp: 150000,
                boss_ad: 1400,
                boss_def: 400,
                boss_mr: 350,
                reward_gold: 15000,
                reward_diamond: 150,
                reward_exp: 5000,
                emoji: "💎",
            },
            RushStage {
                name: "第二关",
                boss_name: "虚空领主",
                boss_hp: 200000,
                boss_ad: 1800,
                boss_def: 500,
                boss_mr: 430,
                reward_gold: 20000,
                reward_diamond: 200,
                reward_exp: 7000,
                emoji: "💎",
            },
            RushStage {
                name: "第三关",
                boss_name: "混沌魔神",
                boss_hp: 280000,
                boss_ad: 2200,
                boss_def: 600,
                boss_mr: 520,
                reward_gold: 30000,
                reward_diamond: 300,
                reward_exp: 10000,
                emoji: "💎",
            },
        ],
    ),
    (
        "传说试炼",
        &[
            RushStage {
                name: "第一关",
                boss_name: "灭世龙王",
                boss_hp: 400000,
                boss_ad: 2800,
                boss_def: 750,
                boss_mr: 650,
                reward_gold: 40000,
                reward_diamond: 400,
                reward_exp: 15000,
                emoji: "🔴",
            },
            RushStage {
                name: "第二关",
                boss_name: "终焉审判者",
                boss_hp: 550000,
                boss_ad: 3500,
                boss_def: 900,
                boss_mr: 800,
                reward_gold: 60000,
                reward_diamond: 600,
                reward_exp: 20000,
                emoji: "🔴",
            },
            RushStage {
                name: "终极BOSS",
                boss_name: "毁灭之神",
                boss_hp: 800000,
                boss_ad: 4500,
                boss_def: 1100,
                boss_mr: 1000,
                reward_gold: 100000,
                reward_diamond: 1000,
                reward_exp: 30000,
                emoji: "🔴",
            },
        ],
    ),
];

/// 每日Boss Rush最大次数
const MAX_DAILY_ATTEMPTS: i32 = 3;

/// VIP额外次数
const VIP_EXTRA_ATTEMPTS: i32 = 2;

/// 冷却时间（秒）
const RUSH_COOLDOWN_SECS: i64 = 300;

/// 获取玩家等级
fn get_user_level(db: &Database, user_id: &str) -> i32 {
    db.read_basic(user_id, "LV").parse::<i32>().unwrap_or(1)
}

/// 获取玩家战斗属性
fn get_player_stats(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let hp: i32 = db.read_basic(user_id, "HP_Max").parse().unwrap_or(100);
    let ad: i32 = db.read_basic(user_id, "AD").parse().unwrap_or(10);
    let def: i32 = db.read_basic(user_id, "Defense").parse().unwrap_or(5);
    let mr: i32 = db.read_basic(user_id, "MagicResistance").parse().unwrap_or(5);
    let crit: i32 = db.read_basic(user_id, "Crit").parse().unwrap_or(5).min(50);
    (hp, ad, def, mr, crit)
}

/// 检查VIP状态
fn is_vip(db: &Database, user_id: &str) -> bool {
    let vip_end: i64 = db
        .global_get("vip_membership", &format!("{}_end", user_id))
        .parse()
        .unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    vip_end > now
}

/// 获取当前日期字符串（天数）
fn today_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now / 86400)
}

/// 获取当前时间戳
fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 计算Boss Rush总分
fn calc_rush_score(tier_idx: usize, stages_cleared: i32, total_damage: i64, turns_used: i32) -> i64 {
    let tier_bonus = (tier_idx as i64 + 1) * 10000;
    let stage_bonus = stages_cleared as i64 * 3000;
    let damage_score = total_damage / 100;
    let efficiency_bonus = if turns_used > 0 {
        (stages_cleared as i64 * 1000) / turns_used as i64
    } else {
        0
    };
    tier_bonus + stage_bonus + damage_score + efficiency_bonus
}

/// 模拟Boss Rush战斗（回合制）
fn simulate_rush_battle(
    player_hp: i32,
    player_ad: i32,
    player_def: i32,
    player_mr: i32,
    player_crit: i32,
    stage: &RushStage,
) -> (bool, i32, i64, i32) {
    let mut p_hp = player_hp as i64;
    let mut b_hp = stage.boss_hp;
    let mut total_damage: i64 = 0;
    let mut turns = 0i32;
    let mut rng_seed: u64 = 12345u64;

    loop {
        turns += 1;
        if turns > 100 {
            break;
        }

        // 玩家攻击
        rng_seed = rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let crit_roll = (rng_seed % 100) as i32;
        let base_dmg = (player_ad - stage.boss_def).max(1);
        let dmg = if crit_roll < player_crit {
            base_dmg * 150 / 100
        } else {
            base_dmg
        };
        b_hp -= dmg as i64;
        total_damage += dmg as i64;
        if b_hp <= 0 {
            return (true, p_hp as i32, total_damage, turns);
        }

        // Boss攻击
        rng_seed = rng_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let boss_dmg = (stage.boss_ad as i64 - player_def.max(player_mr) as i64).max(1);
        p_hp -= boss_dmg;
        if p_hp <= 0 {
            return (false, 0, total_damage, turns);
        }
    }
    (false, p_hp as i32, total_damage, turns)
}

/// 查看Boss Rush — 显示所有难度阶段和奖励预览
pub fn cmd_view_boss_rush(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level = get_user_level(db, user_id);
    let today = today_string();
    let attempts_used: i32 = db
        .global_get("boss_rush", &format!("{}_rush_attempts_{}", user_id, today))
        .parse()
        .unwrap_or(0);
    let max_attempts = if is_vip(db, user_id) {
        MAX_DAILY_ATTEMPTS + VIP_EXTRA_ATTEMPTS
    } else {
        MAX_DAILY_ATTEMPTS
    };
    let vip_tag = if is_vip(db, user_id) { " (VIP+2)" } else { "" };

    let mut out = format!("{}\n═══ ⚔️ Boss Rush 连续挑战 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    out.push_str("🔥 挑战连续BOSS，难度递增，奖励累积！\n\n");
    out.push_str(&format!(
        "📊 今日挑战次数: {}/{}{}\n\n",
        attempts_used, max_attempts, vip_tag
    ));

    for (i, (tier_name, stages)) in RUSH_TIERS.iter().enumerate() {
        let min_level = match i {
            0 => 1,
            1 => 15,
            2 => 30,
            3 => 50,
            _ => 70,
        };
        let locked = level < min_level;
        let status_icon = if locked { "🔒" } else { "🔓" };

        let total_gold: i32 = stages.iter().map(|s| s.reward_gold).sum();
        let total_diamond: i32 = stages.iter().map(|s| s.reward_diamond).sum();
        let total_exp: i32 = stages.iter().map(|s| s.reward_exp).sum();

        out.push_str(&format!(
            "{} {}{} (Lv.{}) — {}个BOSS | 💰{}  💎{}  ✨{}\n",
            stages[0].emoji,
            status_icon,
            tier_name,
            min_level,
            stages.len(),
            format_num(total_gold),
            format_num(total_diamond),
            format_num(total_exp)
        ));

        if !locked {
            for stage in stages.iter() {
                out.push_str(&format!(
                    "    {} {} — HP:{} AD:{} DEF:{}\n",
                    stage.emoji,
                    stage.boss_name,
                    format_num(stage.boss_hp as i32),
                    stage.boss_ad,
                    stage.boss_def
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("💡 指令：开始Boss Rush+难度名 (如: 开始Boss Rush+青铜)\n");
    out.push_str("💡 指令：Boss Rush进度 / Boss Rush排行 / Boss Rush记录\n");
    out
}

/// 开始Boss Rush — 进入连续BOSS战
pub fn cmd_start_boss_rush(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level = get_user_level(db, user_id);

    if args.trim().is_empty() {
        return format!(
            "{}\n❌ 请指定Boss Rush难度！\n💡 可选: 青铜/白银/黄金/钻石/传说\n",
            prefix
        );
    }

    // 检查冷却
    let last_rush: i64 = db
        .global_get("boss_rush", &format!("{}_rush_cooldown", user_id))
        .parse()
        .unwrap_or(0);
    let now = now_ts();
    if now - last_rush < RUSH_COOLDOWN_SECS {
        let remaining = RUSH_COOLDOWN_SECS - (now - last_rush);
        return format!("{}\n⏳ Boss Rush冷却中！还需{}秒\n", prefix, remaining);
    }

    // 检查每日次数
    let today = today_string();
    let attempts_used: i32 = db
        .global_get("boss_rush", &format!("{}_rush_attempts_{}", user_id, today))
        .parse()
        .unwrap_or(0);
    let max_attempts = if is_vip(db, user_id) {
        MAX_DAILY_ATTEMPTS + VIP_EXTRA_ATTEMPTS
    } else {
        MAX_DAILY_ATTEMPTS
    };
    if attempts_used >= max_attempts {
        return format!(
            "{}\n❌ 今日Boss Rush次数已用完 ({}/{})！\n💡 每日{}次，VIP额外{}次\n",
            prefix, attempts_used, max_attempts, MAX_DAILY_ATTEMPTS, VIP_EXTRA_ATTEMPTS
        );
    }

    // 查找匹配的难度
    let query = args.trim();
    let tier_idx = RUSH_TIERS
        .iter()
        .position(|(name, _)| name.contains(query) || query.contains(&name[..2]));

    let tier_idx = match tier_idx {
        Some(idx) => idx,
        None => {
            return format!(
                "{}\n❌ 未找到难度「{}」！\n💡 可选: 青铜/白银/黄金/钻石/传说\n",
                prefix, query
            )
        }
    };

    let min_level = match tier_idx {
        0 => 1,
        1 => 15,
        2 => 30,
        3 => 50,
        _ => 70,
    };
    if level < min_level {
        return format!(
            "{}\n🔒 等级不足！{}需要Lv.{}，你当前Lv.{}\n",
            prefix, RUSH_TIERS[tier_idx].0, min_level, level
        );
    }

    let (player_hp, player_ad, player_def, player_mr, player_crit) = get_player_stats(db, user_id);
    let (tier_name, stages) = &RUSH_TIERS[tier_idx];

    let mut out = format!(
        "{}\n═══ ⚔️ Boss Rush: {} ═══\n━━━━━━━━━━━━━━━━━━━━\n",
        prefix, tier_name
    );

    let mut total_gold = 0i32;
    let mut total_diamond = 0i32;
    let mut total_exp = 0i32;
    let mut stages_cleared = 0i32;
    let mut total_damage = 0i64;
    let mut total_turns = 0i32;
    let mut current_hp = player_hp;

    for (i, stage) in stages.iter().enumerate() {
        out.push_str(&format!(
            "\n{} 第{}关: {} (HP:{})\n",
            stage.emoji,
            i + 1,
            stage.boss_name,
            format_num(stage.boss_hp as i32)
        ));

        let (won, remaining_hp, damage, turns) =
            simulate_rush_battle(current_hp, player_ad, player_def, player_mr, player_crit, stage);

        total_damage += damage;
        total_turns += turns;

        if won {
            stages_cleared += 1;
            total_gold += stage.reward_gold;
            total_diamond += stage.reward_diamond;
            total_exp += stage.reward_exp;
            current_hp = remaining_hp.max(1);
            out.push_str(&format!(
                "  ✅ 胜利！(剩余HP:{}, {}回合, 伤害:{})\n",
                format_num(remaining_hp),
                turns,
                format_num(damage as i32)
            ));
            out.push_str(&format!(
                "  📦 奖励: 💰{} 💎{} ✨{}\n",
                format_num(stage.reward_gold),
                format_num(stage.reward_diamond),
                format_num(stage.reward_exp)
            ));
        } else {
            out.push_str(&format!(
                "  ❌ 战败！(坚持{}回合, 累计伤害:{})\n",
                turns,
                format_num(damage as i32)
            ));
            out.push_str("  💀 Boss Rush结束！\n");
            break;
        }
    }

    // 发放奖励
    if total_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold as i64);
    }
    if total_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, total_diamond as i64);
    }
    if total_exp > 0 {
        user::add_experience(db, user_id, total_exp);
    }

    let score = calc_rush_score(tier_idx, stages_cleared, total_damage, total_turns);

    out.push_str("\n═══ 📊 挑战总结 ═══\n━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("🏆 通关: {}/{}关\n", stages_cleared, stages.len()));
    out.push_str(&format!("⚔️ 总伤害: {}\n", format_num(total_damage as i32)));
    out.push_str(&format!("🔄 总回合: {}\n", total_turns));
    out.push_str(&format!("⭐ 评分: {}\n", format_num(score as i32)));

    if stages_cleared as usize == stages.len() {
        out.push_str("\n🎉 全部通关！完美表现！\n");
        let bonus_gold = total_gold / 5;
        let bonus_diamond = total_diamond / 5;
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, bonus_gold as i64);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, bonus_diamond as i64);
        out.push_str(&format!(
            "🎁 全通关额外奖励: 💰{} 💎{}\n",
            format_num(bonus_gold),
            format_num(bonus_diamond)
        ));
    }

    // 记录
    let _ = db.global_set(
        "boss_rush",
        &format!("{}_rush_attempts_{}", user_id, today),
        &(attempts_used + 1).to_string(),
    );
    let _ = db.global_set("boss_rush", &format!("{}_rush_cooldown", user_id), &now.to_string());

    let best_key = format!("{}_rush_best_{}", user_id, tier_idx);
    let current_best: i64 = db.global_get("boss_rush", &best_key).parse().unwrap_or(0);
    if score > current_best {
        let _ = db.global_set("boss_rush", &best_key, &score.to_string());
        out.push_str("🏅 新纪录！\n");
    }

    let total_score_key = format!("{}_rush_total", user_id);
    let current_total: i64 = db.global_get("boss_rush", &total_score_key).parse().unwrap_or(0);
    let _ = db.global_set("boss_rush", &total_score_key, &(current_total + score).to_string());

    let clears_key = format!("{}_rush_clears", user_id);
    let current_clears: i32 = db.global_get("boss_rush", &clears_key).parse().unwrap_or(0);
    let _ = db.global_set("boss_rush", &clears_key, &(current_clears + stages_cleared).to_string());

    out
}

/// Boss Rush进度 — 查看当前挑战状态
pub fn cmd_boss_rush_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = today_string();
    let attempts_used: i32 = db
        .global_get("boss_rush", &format!("{}_rush_attempts_{}", user_id, today))
        .parse()
        .unwrap_or(0);
    let max_attempts = if is_vip(db, user_id) {
        MAX_DAILY_ATTEMPTS + VIP_EXTRA_ATTEMPTS
    } else {
        MAX_DAILY_ATTEMPTS
    };

    let mut out = format!("{}\n═══ 📊 Boss Rush 进度 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    out.push_str(&format!("📅 今日挑战: {}/{}次\n\n", attempts_used, max_attempts));

    out.push_str("🏆 最佳记录:\n");
    for (i, (tier_name, stages)) in RUSH_TIERS.iter().enumerate() {
        let best_key = format!("{}_rush_best_{}", user_id, i);
        let best_score: i64 = db.global_get("boss_rush", &best_key).parse().unwrap_or(0);
        if best_score > 0 {
            out.push_str(&format!(
                "  {} {}: ⭐{}\n",
                stages[0].emoji,
                tier_name,
                format_num(best_score as i32)
            ));
        } else {
            out.push_str(&format!("  {} {}: 未挑战\n", stages[0].emoji, tier_name));
        }
    }

    let total_score: i64 = db
        .global_get("boss_rush", &format!("{}_rush_total", user_id))
        .parse()
        .unwrap_or(0);
    let total_clears: i32 = db
        .global_get("boss_rush", &format!("{}_rush_clears", user_id))
        .parse()
        .unwrap_or(0);
    out.push_str(&format!(
        "\n📈 累计数据:\n  ⭐ 总评分: {}\n  🏆 累计通关: {}关\n",
        format_num(total_score as i32),
        total_clears
    ));

    let last_rush: i64 = db
        .global_get("boss_rush", &format!("{}_rush_cooldown", user_id))
        .parse()
        .unwrap_or(0);
    let now = now_ts();
    if now - last_rush < RUSH_COOLDOWN_SECS {
        out.push_str(&format!("\n⏳ 冷却中: {}秒\n", RUSH_COOLDOWN_SECS - (now - last_rush)));
    } else {
        out.push_str("\n✅ 可以挑战！\n");
    }
    out
}

/// Boss Rush排行 — 全服排行
pub fn cmd_boss_rush_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut out = format!("{}\n═══ 🏆 Boss Rush 排行榜 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    let mut entries: Vec<(String, i64)> = Vec::new();
    {
        let conn = db.lock_conn();
        let mut stmt =
            match conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION='boss_rush' AND ID LIKE '%_rush_total'") {
                Ok(s) => s,
                Err(_) => return format!("{}\n⚠️ 数据查询失败", prefix),
            };
        let results: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get(0).unwrap_or_default(), row.get(1).unwrap_or_default()))
            })
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        drop(stmt);
        for (key, value) in results {
            let uid = key.strip_prefix("").unwrap_or(&key);
            // strip suffix "_rush_total"
            let uid = uid.trim_end_matches("_rush_total");
            let score: i64 = value.parse().unwrap_or(0);
            if score > 0 && !uid.is_empty() {
                entries.push((uid.to_string(), score));
            }
        }
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.1));

    if entries.is_empty() {
        out.push_str("\n📊 暂无挑战记录！\n💡 成为第一个挑战Boss Rush的勇者吧！\n");
        return out;
    }

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, score)) in entries.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let nn = db.read_basic(uid, "NickName");
        let nickname = if nn.is_empty() { uid.clone() } else { nn };
        let clears: i32 = db
            .global_get("boss_rush", &format!("{}_rush_clears", uid))
            .parse()
            .unwrap_or(0);
        let marker = if uid == user_id { " ◀" } else { "" };
        out.push_str(&format!(
            "{}{}. {} — ⭐{} (通关:{}关){}\n",
            medal,
            i + 1,
            nickname,
            format_num(*score as i32),
            clears,
            marker
        ));
    }

    let user_rank = entries.iter().position(|(uid, _)| uid == user_id);
    if let Some(rank) = user_rank {
        if rank >= 15 {
            out.push_str(&format!(
                "\n📍 你的排名: 第{}名 — ⭐{}\n",
                rank + 1,
                format_num(entries[rank].1 as i32)
            ));
        }
    }
    out
}

/// Boss Rush记录 — 个人挑战历史
pub fn cmd_boss_rush_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ 📋 Boss Rush 挑战记录 ═══\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for (i, (tier_name, stages)) in RUSH_TIERS.iter().enumerate() {
        let best_key = format!("{}_rush_best_{}", user_id, i);
        let best_score: i64 = db.global_get("boss_rush", &best_key).parse().unwrap_or(0);
        let total_gold: i32 = stages.iter().map(|s| s.reward_gold).sum();
        let total_diamond: i32 = stages.iter().map(|s| s.reward_diamond).sum();

        out.push_str(&format!("\n{} {}:\n", stages[0].emoji, tier_name));
        if best_score > 0 {
            out.push_str(&format!("  ⭐ 最佳评分: {}\n", format_num(best_score as i32)));
            out.push_str(&format!(
                "  💰 全通奖励: 💰{} 💎{} (+20%全通加成)\n",
                format_num(total_gold),
                format_num(total_diamond)
            ));
        } else {
            out.push_str("  ⏳ 未挑战\n");
        }
    }

    let total_score: i64 = db
        .global_get("boss_rush", &format!("{}_rush_total", user_id))
        .parse()
        .unwrap_or(0);
    let total_clears: i32 = db
        .global_get("boss_rush", &format!("{}_rush_clears", user_id))
        .parse()
        .unwrap_or(0);
    if total_score > 0 {
        let avg = if total_clears > 0 {
            format_num((total_score / total_clears as i64) as i32)
        } else {
            "0".to_string()
        };
        out.push_str(&format!(
            "\n═══ 📈 总计 ═══\n  ⭐ 总评分: {}\n  🏆 总通关: {}关\n  📊 平均每关: {}分\n",
            format_num(total_score as i32),
            total_clears,
            avg
        ));
    }
    out
}

/// 千分位格式化
fn format_num(n: i32) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rush_tiers_count() {
        assert_eq!(RUSH_TIERS.len(), 5);
    }

    #[test]
    fn test_tier_stages_count() {
        for (name, stages) in RUSH_TIERS {
            assert_eq!(stages.len(), 3, "Tier '{}' should have 3 stages", name);
        }
    }

    #[test]
    fn test_tier_names() {
        let names: Vec<&str> = RUSH_TIERS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"青铜试炼"));
        assert!(names.contains(&"传说试炼"));
    }

    #[test]
    fn test_stage_rewards_positive() {
        for (_, stages) in RUSH_TIERS {
            for stage in stages.iter() {
                assert!(stage.reward_gold > 0);
                assert!(stage.reward_diamond > 0);
                assert!(stage.reward_exp > 0);
            }
        }
    }

    #[test]
    fn test_difficulty_scaling() {
        for (_, stages) in RUSH_TIERS {
            assert!(stages[0].boss_hp < stages[1].boss_hp);
            assert!(stages[1].boss_hp < stages[2].boss_hp);
            assert!(stages[0].boss_ad < stages[1].boss_ad);
            assert!(stages[1].boss_ad < stages[2].boss_ad);
        }
    }

    #[test]
    fn test_tier_difficulty_ordering() {
        let first_boss_hp = RUSH_TIERS[0].1[0].boss_hp;
        let last_boss_hp = RUSH_TIERS[4].1[2].boss_hp;
        assert!(last_boss_hp > first_boss_hp * 10);
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(123), "123");
        assert_eq!(format_num(1234), "1,234");
        assert_eq!(format_num(1234567), "1,234,567");
    }

    #[test]
    fn test_calc_rush_score() {
        let full = calc_rush_score(0, 3, 30000, 20);
        let partial = calc_rush_score(0, 1, 10000, 20);
        assert!(full > partial);
    }

    #[test]
    fn test_calc_rush_score_tier_bonus() {
        let bronze = calc_rush_score(0, 3, 30000, 20);
        let legendary = calc_rush_score(4, 3, 30000, 20);
        assert!(legendary > bronze);
    }

    #[test]
    fn test_boss_rush_battle_won() {
        let (won, _, _, _) = simulate_rush_battle(100000, 5000, 1000, 1000, 30, &RUSH_TIERS[0].1[0]);
        assert!(won);
    }

    #[test]
    fn test_boss_rush_battle_lost() {
        let (won, _, _, _) = simulate_rush_battle(100, 10, 5, 5, 5, &RUSH_TIERS[4].1[2]);
        assert!(!won);
    }

    #[test]
    fn test_simulate_returns_damage() {
        let (_, _, damage, turns) = simulate_rush_battle(10000, 500, 100, 100, 20, &RUSH_TIERS[0].1[0]);
        assert!(damage > 0);
        assert!(turns > 0);
    }

    #[test]
    fn test_constants_reasonable() {
        assert!(MAX_DAILY_ATTEMPTS > 0);
        assert!(VIP_EXTRA_ATTEMPTS > 0);
        assert!(RUSH_COOLDOWN_SECS > 0);
    }

    #[test]
    fn test_stage_emojis() {
        for (_, stages) in RUSH_TIERS {
            for stage in stages.iter() {
                assert!(!stage.emoji.is_empty());
                assert!(!stage.boss_name.is_empty());
            }
        }
    }
}

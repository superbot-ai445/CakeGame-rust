/// CakeGame 属性试炼系统 (Attribute Trial System)
///
/// 9种属性试炼，每种10级，挑战成功获得永久属性加成。
/// 每日每种试炼3次挑战机会，消耗金币（递增），VIP额外+1次。
/// 战力门槛匹配：试炼等级×玩家等级系数决定怪物难度。
/// 数据存储: Global 表 section='attr_trial'
///
/// 指令: 查看试炼, 挑战试炼, 试炼排行
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 试炼定义
struct TrialDef {
    key: &'static str,
    name: &'static str,
    emoji: &'static str,
    attr_name: &'static str,
    desc: &'static str,
}

const TRIALS: &[TrialDef] = &[
    TrialDef {
        key: "hp",
        name: "生命试炼",
        emoji: "❤️",
        attr_name: "HP",
        desc: "极限生存挑战 — 坚持到底方能领悟生命之力",
    },
    TrialDef {
        key: "mp",
        name: "魔力试炼",
        emoji: "💧",
        attr_name: "MP",
        desc: "魔力源泉挑战 — 掌控魔力的涌动",
    },
    TrialDef {
        key: "ad",
        name: "力量试炼",
        emoji: "⚔️",
        attr_name: "AD",
        desc: "力量突破挑战 — 以力破万法",
    },
    TrialDef {
        key: "ap",
        name: "智慧试炼",
        emoji: "🔮",
        attr_name: "AP",
        desc: "智慧启迪挑战 — 魔法的真谛",
    },
    TrialDef {
        key: "def",
        name: "坚韧试炼",
        emoji: "🛡️",
        attr_name: "Defense",
        desc: "钢铁意志挑战 — 不动如山",
    },
    TrialDef {
        key: "mr",
        name: "抗魔试炼",
        emoji: "🌀",
        attr_name: "MagicResistance",
        desc: "元素抗性挑战 — 魔法无效化",
    },
    TrialDef {
        key: "hit",
        name: "精准试炼",
        emoji: "🎯",
        attr_name: "Hit",
        desc: "百步穿杨挑战 — 例不虚发",
    },
    TrialDef {
        key: "dodge",
        name: "敏捷试炼",
        emoji: "💨",
        attr_name: "Dodge",
        desc: "极限闪避挑战 — 来去如风",
    },
    TrialDef {
        key: "crit",
        name: "暴击试炼",
        emoji: "💥",
        attr_name: "Crit",
        desc: "致命一击挑战 — 一击必杀",
    },
];

const MAX_LEVEL: i32 = 10;
const DAILY_ATTEMPTS: i32 = 3;
/// 每级永久属性加成百分比
const BONUS_PCT_PER_LEVEL: f64 = 0.02;

/// 获取试炼怪物属性（基于试炼等级和玩家等级）
fn get_trial_monster(trial_level: i32, player_level: i32) -> (i32, i32, i32, i32, i32) {
    let level_mult = 1.0 + (player_level as f64 * 0.08);
    let trial_mult = 1.0 + (trial_level as f64 * 0.35);
    let base = 100.0 * level_mult * trial_mult;
    let hp = (base * 3.0) as i32;
    let ad = (base * 0.6) as i32;
    let def = (base * 0.25) as i32;
    let mr = (base * 0.2) as i32;
    let gold_cost = trial_level * 200 + 100;
    (hp, ad, def, mr, gold_cost)
}

/// 获取试炼奖励
fn get_trial_rewards(trial_level: i32) -> (i32, i32, i32) {
    let gold = 200 + trial_level * 150;
    let diamond = 5 + trial_level * 3;
    let exp = 100 + trial_level * 80;
    (gold, diamond, exp)
}

/// 读取玩家试炼数据
fn get_trial_level(db: &Database, user_id: &str, trial_key: &str) -> i32 {
    db.global_get("attr_trial", &format!("{}_{}", user_id, trial_key))
        .parse::<i32>()
        .unwrap_or(0)
}

/// 读取今日已用次数
fn get_trial_attempts(db: &Database, user_id: &str, trial_key: &str, today: &str) -> i32 {
    let date_key = format!("{}_{}_date", user_id, trial_key);
    let last_date = db.global_get("attr_trial", &date_key);
    if last_date != today {
        0
    } else {
        let attempts_key = format!("{}_{}_attempts", user_id, trial_key);
        db.global_get("attr_trial", &attempts_key).parse::<i32>().unwrap_or(0)
    }
}

/// 检查VIP
fn is_vip(db: &Database, user_id: &str) -> bool {
    let now = chrono::Local::now().timestamp();
    let vip_end: i64 = db
        .global_get("vip_membership", &format!("{}_end", user_id))
        .parse()
        .unwrap_or(0);
    vip_end > now
}

/// 数字格式化
pub fn format_num(n: i64) -> String {
    if n >= 10000 {
        format!("{:.1}万", n as f64 / 10000.0)
    } else if n >= 1000 {
        let s = n.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
        }
        result.chars().rev().collect()
    } else {
        n.to_string()
    }
}

/// 查看试炼 — 列出所有属性试炼及当前进度
pub fn cmd_view_attr_trials(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let attrs = user::calc_total_attrs(db, user_id);
    let player_level = attrs.level;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut out = format!("{}\n═══ ⚔️ 属性试炼殿 ═══", prefix);
    out.push_str("\n挑战属性试炼，突破自我极限，获得永久属性加成！");
    out.push_str(&format!(
        "\n\n📊 当前等级: {}  |  每日挑战次数: {}次/试炼(VIP+1)",
        player_level, DAILY_ATTEMPTS
    ));
    out.push_str(&format!(
        "\n💡 每通过一级，该属性永久+{:.0}%",
        BONUS_PCT_PER_LEVEL * 100.0
    ));

    let mut total_completed: i32 = 0;
    let mut total_bonus_pct: f64 = 0.0;

    for t in TRIALS {
        let completed = get_trial_level(db, user_id, t.key);
        let attempts = get_trial_attempts(db, user_id, t.key, &today);
        total_completed += completed;
        total_bonus_pct += completed as f64 * BONUS_PCT_PER_LEVEL;

        let progress = if completed >= MAX_LEVEL {
            "👑 已圆满".to_string()
        } else {
            let bar_len = 5;
            let filled = (completed * bar_len / MAX_LEVEL).min(bar_len);
            let empty = bar_len - filled;
            format!(
                "{filled}{empty} Lv.{completed}/{MAX_LEVEL}",
                filled = "█".repeat(filled as usize),
                empty = "░".repeat(empty as usize),
                completed = completed,
                MAX_LEVEL = MAX_LEVEL
            )
        };

        let max_att = if is_vip(db, user_id) {
            DAILY_ATTEMPTS + 1
        } else {
            DAILY_ATTEMPTS
        };
        let remaining = (max_att - attempts).max(0);
        let next_level = completed + 1;
        let cost_hint = if completed >= MAX_LEVEL {
            String::new()
        } else {
            let (_, _, _, _, gold_cost) = get_trial_monster(next_level, player_level);
            format!(" | 💰{}", format_num(gold_cost as i64))
        };

        out.push_str(&format!(
            "\n  {} {} {} {}/{}次{}",
            t.emoji, t.name, progress, remaining, max_att, cost_hint
        ));
    }

    out.push_str(&format!(
        "\n\n📈 总计完成: {}级  |  总属性加成: +{:.1}%",
        total_completed,
        total_bonus_pct * 100.0
    ));
    out.push_str("\n\n💡 发送「挑战试炼+属性名」开始挑战（如：挑战试炼 力量）");
    out.push_str("\n💡 发送「试炼排行」查看全服排名");
    out
}

/// 挑战试炼 — 进行一次属性试炼战斗
pub fn cmd_challenge_attr_trial(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        return format!("{}\n❌ 请输入试炼名称！如：挑战试炼 力量", prefix);
    }

    // 查找匹配的试炼
    let trial = TRIALS.iter().find(|t| {
        t.name.contains(args)
            || t.key == args
            || t.attr_name.eq_ignore_ascii_case(args)
            || (args.len() >= 2 && t.name.contains(&args[..args.len().min(6)]))
    });

    let trial = match trial {
        Some(t) => t,
        None => {
            let names: Vec<&str> = TRIALS.iter().map(|t| t.name).collect();
            return format!("\n❌ 未找到试炼「{}」！\n可用试炼: {}", args, names.join("、"));
        }
    };

    // 检查虚弱状态
    let weakness = user::check_weakness(db, user_id);
    if weakness > 0 {
        return format!("{}\n❌ 你处于虚弱状态（{}秒后恢复），无法挑战试炼！", prefix, weakness);
    }

    let attrs = user::calc_total_attrs(db, user_id);
    let player_level = attrs.level;

    if player_level < 5 {
        return format!("{}\n❌ 等级不足！属性试炼需要达到5级。", prefix);
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let completed = get_trial_level(db, user_id, trial.key);
    let attempts = get_trial_attempts(db, user_id, trial.key, &today);

    if completed >= MAX_LEVEL {
        return format!(
            "\n{} 你已完成 {}{} 的所有试炼！属性加成已达上限 +{:.0}%",
            prefix,
            trial.emoji,
            trial.name,
            MAX_LEVEL as f64 * BONUS_PCT_PER_LEVEL * 100.0
        );
    }

    // 检查每日次数（VIP额外+1）
    let max_attempts = if is_vip(db, user_id) {
        DAILY_ATTEMPTS + 1
    } else {
        DAILY_ATTEMPTS
    };
    if attempts >= max_attempts {
        return format!(
            "\n❌ 今日 {}{} 挑战次数已用完（{}/{}）！明天再来。",
            trial.emoji, trial.name, attempts, max_attempts
        );
    }

    let next_level = completed + 1;
    let (monster_hp, monster_ad, _monster_def, monster_mr, gold_cost) = get_trial_monster(next_level, player_level);

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < gold_cost as i64 {
        return format!(
            "\n❌ 金币不足！挑战 {} Lv.{} 需要 {}金币，你只有 {}。",
            trial.name,
            next_level,
            format_num(gold_cost as i64),
            format_num(gold)
        );
    }

    // 扣除金币
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_cost as i64);

    // 获取玩家属性
    let player_hp = attrs.hp_max.max(1);
    let player_ad = attrs.ad;
    let player_ap = attrs.ap;
    let player_def = attrs.defense;
    let player_mr = attrs.magic_res;
    let player_hit = attrs.hit;
    let player_dodge = attrs.dodge;
    let player_crit = attrs.crit;

    // 回合制战斗模拟
    let mut p_hp = player_hp;
    let mut m_hp = monster_hp;
    let mut rounds = 0;
    let max_rounds = 20;
    let mut battle_log: Vec<String> = Vec::new();

    while p_hp > 0 && m_hp > 0 && rounds < max_rounds {
        rounds += 1;

        // 玩家攻击
        let hit_roll: i32 = rand::random::<u32>() as i32 % 100;
        if hit_roll < player_hit.min(95) {
            let base_dmg = (player_ad + player_ap / 2).max(1);
            let def_reduction = (monster_mr * 2 / 3).max(0);
            let mut dmg = (base_dmg - def_reduction).max(1);

            // 暴击
            let crit_roll: i32 = rand::random::<u32>() as i32 % 100;
            let is_crit = crit_roll < player_crit.min(50);
            if is_crit {
                dmg = dmg * 150 / 100;
            }

            m_hp -= dmg;
            battle_log.push(format!(
                "第{}回合: 你{}攻击造成{}伤害{}",
                rounds,
                trial.emoji,
                format_num(dmg as i64),
                if is_crit { " 💥暴击！" } else { "" }
            ));
        } else {
            battle_log.push(format!("第{}回合: 你的攻击被闪避了！", rounds));
        }

        if m_hp <= 0 {
            break;
        }

        // 怪物攻击
        let dodge_roll: i32 = rand::random::<u32>() as i32 % 100;
        if dodge_roll < player_dodge.min(40) {
            battle_log.push(format!("第{}回合: 你闪避了怪物的攻击！💨", rounds));
        } else {
            let def_reduction = player_def + player_mr / 2;
            let m_dmg = (monster_ad - def_reduction / 2).max(1);
            p_hp -= m_dmg;
            battle_log.push(format!(
                "第{}回合: 怪物攻击造成{}伤害",
                rounds,
                format_num(m_dmg as i64)
            ));
        }
    }

    // 消耗尝试次数
    let attempts_key = format!("{}_{}_attempts", user_id, trial.key);
    let date_key = format!("{}_{}_date", user_id, trial.key);
    db.global_set("attr_trial", &attempts_key, &(attempts + 1).to_string());
    db.global_set("attr_trial", &date_key, &today);

    let won = m_hp <= 0;

    let mut result = format!("\n═══ {}{} Lv.{} 试炼 ═══\n", trial.emoji, trial.name, next_level);
    result.push_str(&format!("📖 {}\n\n", trial.desc));

    // 战斗日志（只显示最后5回合）
    let log_start = if battle_log.len() > 5 { battle_log.len() - 5 } else { 0 };
    if log_start > 0 {
        result.push_str("...（战斗过程省略）...\n");
    }
    for log in &battle_log[log_start..] {
        result.push_str(&format!("  {}\n", log));
    }

    result.push_str(&format!(
        "\n📊 战斗结果: {}回合 | 你HP:{} | 怪物HP:{}",
        rounds,
        format_num(p_hp.max(0) as i64),
        format_num(m_hp.max(0) as i64)
    ));

    if won {
        // 通关成功！
        let (reward_gold, reward_diamond, reward_exp) = get_trial_rewards(next_level);
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, reward_gold as i64);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, reward_diamond as i64);
        user::add_experience(db, user_id, reward_exp);

        // 更新试炼等级
        db.global_set(
            "attr_trial",
            &format!("{}_{}", user_id, trial.key),
            &next_level.to_string(),
        );

        let bonus_pct = next_level as f64 * BONUS_PCT_PER_LEVEL * 100.0;
        result.push_str(&format!(
            "\n\n🎉 ✅ 试炼通过！{}{} 属性永久加成 +{:.0}%",
            trial.emoji, trial.attr_name, bonus_pct
        ));
        result.push_str(&format!(
            "\n💰 奖励: {}金币 + {}钻石 + {}经验",
            format_num(reward_gold as i64),
            format_num(reward_diamond as i64),
            format_num(reward_exp as i64)
        ));

        if next_level >= MAX_LEVEL {
            result.push_str(&format!(
                "\n\n🏆 恭喜！你已完成 {}{} 的全部试炼！\n属性加成已达上限 +{:.0}% — 你是真正的{}大师！",
                trial.emoji,
                trial.name,
                MAX_LEVEL as f64 * BONUS_PCT_PER_LEVEL * 100.0,
                trial.name.replace("试炼", "")
            ));
        }
    } else {
        result.push_str("\n\n❌ 试炼失败！怪物太强了，回去提升实力再来吧。");
        // 失败返还一半金币
        let refund = gold_cost as i64 / 2;
        if refund > 0 {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, refund);
            result.push_str(&format!("\n💰 返还一半金币: {}", format_num(refund)));
        }
    }

    result
}

/// 试炼排行 — 全服属性试炼排名
pub fn cmd_attr_trial_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut out = format!("{}\n═══ 🏆 属性试炼全服排行 ═══\n", prefix);

    let users = db.all_users();

    let mut rankings: Vec<(String, i32, f64)> = Vec::new();

    for uid in &users {
        let mut total_levels: i32 = 0;
        for t in TRIALS {
            total_levels += get_trial_level(db, uid, t.key);
        }
        if total_levels > 0 {
            let name = user::get_msg_prefix(db, uid);
            let bonus = total_levels as f64 * BONUS_PCT_PER_LEVEL * 100.0;
            rankings.push((name, total_levels, bonus));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    if rankings.is_empty() {
        out.push_str("\n暂无玩家参与属性试炼");
        out.push_str("\n发送「挑战试炼+属性名」成为第一个挑战者！");
        return out;
    }

    let medals = ["🥇", "🥈", "🥉"];
    let display_count = rankings.len().min(15);

    for (i, (name, levels, bonus)) in rankings.iter().take(display_count).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let marker = if name == &prefix { " ← 你" } else { "" };
        out.push_str(&format!(
            "\n{} {}. {} — {}级/{}级 (属性+{:.1}%){}",
            medal,
            i + 1,
            name,
            levels,
            TRIALS.len() as i32 * MAX_LEVEL,
            bonus,
            marker
        ));
    }

    // 当前用户排名定位
    let mut my_total: i32 = 0;
    for t in TRIALS {
        my_total += get_trial_level(db, user_id, t.key);
    }

    if my_total > 0 {
        let my_rank = rankings
            .iter()
            .position(|(n, _, _)| n == &prefix)
            .unwrap_or(rankings.len());
        if my_rank >= display_count {
            out.push_str(&format!(
                "\n\n📍 你的排名: #{} — {}级/{}级",
                my_rank + 1,
                my_total,
                TRIALS.len() as i32 * MAX_LEVEL
            ));
        }
    } else {
        out.push_str("\n\n📍 你尚未参与属性试炼");
    }

    out.push_str(&format!("\n\n📊 共 {} 名玩家参与试炼", rankings.len()));
    out
}

/// 获取试炼加成属性值（供 user.rs 集成）
#[allow(dead_code)]
pub fn get_trial_attr_bonus(db: &Database, user_id: &str, attr_name: &str) -> f64 {
    for t in TRIALS {
        if t.attr_name.eq_ignore_ascii_case(attr_name) {
            let level = get_trial_level(db, user_id, t.key);
            return level as f64 * BONUS_PCT_PER_LEVEL;
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trials_count() {
        assert_eq!(TRIALS.len(), 9);
    }

    #[test]
    fn test_trial_keys_unique() {
        let mut keys: Vec<&str> = TRIALS.iter().map(|t| t.key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), TRIALS.len());
    }

    #[test]
    fn test_trial_attr_names_unique() {
        let mut names: Vec<&str> = TRIALS.iter().map(|t| t.attr_name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), TRIALS.len());
    }

    #[test]
    fn test_trial_emojis_non_empty() {
        for t in TRIALS {
            assert!(!t.emoji.is_empty(), "Trial {} has empty emoji", t.key);
        }
    }

    #[test]
    fn test_max_level_positive() {
        assert!(MAX_LEVEL > 0);
        assert_eq!(MAX_LEVEL, 10);
    }

    #[test]
    fn test_bonus_pct_per_level() {
        assert!((BONUS_PCT_PER_LEVEL - 0.02).abs() < f64::EPSILON);
        let max_bonus = MAX_LEVEL as f64 * BONUS_PCT_PER_LEVEL;
        assert!((max_bonus - 0.20).abs() < f64::EPSILON);
    }

    #[test]
    fn test_trial_monster_scaling() {
        let (hp1, ad1, _, _, cost1) = get_trial_monster(1, 10);
        let (hp10, ad10, _, _, cost10) = get_trial_monster(10, 10);
        assert!(hp10 > hp1);
        assert!(ad10 > ad1);
        assert!(cost10 > cost1);
    }

    #[test]
    fn test_trial_monster_player_level_scaling() {
        let (hp_low, _, _, _, _) = get_trial_monster(5, 1);
        let (hp_high, _, _, _, _) = get_trial_monster(5, 50);
        assert!(hp_high > hp_low);
    }

    #[test]
    fn test_trial_rewards_positive() {
        for level in 1..=MAX_LEVEL {
            let (gold, diamond, exp) = get_trial_rewards(level);
            assert!(gold > 0, "Gold reward at level {} should be > 0", level);
            assert!(diamond > 0, "Diamond reward at level {} should be > 0", level);
            assert!(exp > 0, "Exp reward at level {} should be > 0", level);
        }
    }

    #[test]
    fn test_trial_rewards_escalate() {
        let (g1, d1, e1) = get_trial_rewards(1);
        let (g5, d5, e5) = get_trial_rewards(5);
        let (g10, d10, e10) = get_trial_rewards(10);
        assert!(g5 > g1);
        assert!(g10 > g5);
        assert!(d5 > d1);
        assert!(d10 > d5);
        assert!(e5 > e1);
        assert!(e10 > e5);
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234), "1,234");
        assert_eq!(format_num(10000), "1.0万");
        assert_eq!(format_num(56789), "5.7万");
    }

    #[test]
    fn test_trial_desc_non_empty() {
        for t in TRIALS {
            assert!(!t.desc.is_empty(), "Trial {} has empty desc", t.key);
        }
    }

    #[test]
    fn test_daily_attempts_constant() {
        assert_eq!(DAILY_ATTEMPTS, 3);
    }
}

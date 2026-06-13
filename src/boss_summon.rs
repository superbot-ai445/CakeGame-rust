/// CakeGame Boss召唤系统
///
/// 玩家使用召唤令牌在当前地图召唤特殊Boss进行战斗。
/// 召唤的Boss比普通野外Boss更强，但掉落更丰厚。
/// 支持：召唤Boss / 召唤列表 / 召唤记录 / 召唤排行
///
/// 数据存储: Global 表 SECTION='boss_summon'
use crate::core::*;
use crate::db::Database;
use crate::user;
use crate::vip;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 召唤令牌物品名
const SUMMON_TOKEN: &str = "Boss召唤令";

/// 召唤Boss定义
struct SummonBoss {
    name: &'static str,
    emoji: &'static str,
    min_level: i32,
    hp: i64,
    ad: i32,
    def: i32,
    mr: i32,
    reward_gold: i64,
    reward_diamond: i32,
    reward_exp: i32,
    /// 额外掉落物品名（概率掉落）
    drop_item: &'static str,
    drop_rate: f64,
    /// 需要召唤令数量
    token_cost: i32,
}

const SUMMON_BOSSES: &[SummonBoss] = &[
    SummonBoss {
        name: "暗影召唤者",
        emoji: "🌑",
        min_level: 10,
        hp: 15000,
        ad: 180,
        def: 60,
        mr: 50,
        reward_gold: 3000,
        reward_diamond: 20,
        reward_exp: 2000,
        drop_item: "强化石",
        drop_rate: 0.5,
        token_cost: 1,
    },
    SummonBoss {
        name: "雷霆巨像",
        emoji: "⚡",
        min_level: 25,
        hp: 50000,
        ad: 400,
        def: 120,
        mr: 100,
        reward_gold: 8000,
        reward_diamond: 50,
        reward_exp: 6000,
        drop_item: "高级强化石",
        drop_rate: 0.35,
        token_cost: 2,
    },
    SummonBoss {
        name: "深渊领主",
        emoji: "🔥",
        min_level: 40,
        hp: 120000,
        ad: 800,
        def: 200,
        mr: 180,
        reward_gold: 20000,
        reward_diamond: 100,
        reward_exp: 15000,
        drop_item: "传说进化石",
        drop_rate: 0.25,
        token_cost: 3,
    },
    SummonBoss {
        name: "灭世龙王",
        emoji: "🐲",
        min_level: 55,
        hp: 300000,
        ad: 1800,
        def: 400,
        mr: 350,
        reward_gold: 50000,
        reward_diamond: 200,
        reward_exp: 40000,
        drop_item: "龙王之心",
        drop_rate: 0.15,
        token_cost: 5,
    },
    SummonBoss {
        name: "混沌之主",
        emoji: "👁️",
        min_level: 70,
        hp: 800000,
        ad: 4000,
        def: 800,
        mr: 700,
        reward_gold: 150000,
        reward_diamond: 500,
        reward_exp: 100000,
        drop_item: "混沌结晶",
        drop_rate: 0.10,
        token_cost: 8,
    },
];

/// 查看可召唤Boss列表
pub fn cmd_summon_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}═══ 📜 Boss召唤列表 ═══\n\n", prefix);
    out.push_str("使用「召唤Boss+Boss名」进行召唤\n");
    out.push_str(&format!("需要消耗「{}」道具\n\n", SUMMON_TOKEN));

    for (i, boss) in SUMMON_BOSSES.iter().enumerate() {
        let can_summon = level >= boss.min_level;
        let status = if can_summon { "✅" } else { "🔒" };
        out.push_str(&format!(
            "{} {}. {} {}\n   等级要求: {} | 消耗: {}个令牌\n   HP: {} | 攻击: {} | 防御: {}/魔抗{}\n   奖励: {}金 + {}钻 + {}经验\n   掉落: {}({:.0}%)\n\n",
            status,
            i + 1,
            boss.emoji,
            boss.name,
            boss.min_level,
            boss.token_cost,
            format_num(boss.hp),
            boss.ad,
            boss.def,
            boss.mr,
            format_num(boss.reward_gold),
            boss.reward_diamond,
            format_num(boss.reward_exp as i64),
            boss.drop_item,
            boss.drop_rate * 100.0
        ));
    }
    out
}

/// 召唤并挑战Boss
pub fn cmd_summon_boss(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return "请输入要召唤的Boss名称\n发送「召唤列表」查看可召唤Boss".to_string();
    }

    // 检查虚弱
    if user::check_weakness(db, user_id) > 0 {
        return "你处于虚弱状态，无法召唤Boss！".to_string();
    }

    let level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);

    // 模糊匹配Boss
    let boss_idx = SUMMON_BOSSES
        .iter()
        .position(|b| b.name.contains(args) || args.contains(b.name));
    let boss_idx = match boss_idx {
        Some(i) => i,
        None => return format!("未找到名为「{}」的召唤Boss\n发送「召唤列表」查看可召唤Boss", args),
    };

    let boss = &SUMMON_BOSSES[boss_idx];

    // 等级检查
    if level < boss.min_level {
        return format!("召唤{}需要等级{}，你当前等级{}", boss.name, boss.min_level, level);
    }

    // 冷却检查（10分钟）
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let last_summon: i64 = db
        .global_get("boss_summon", &format!("cooldown_{}", user_id))
        .parse()
        .unwrap_or(0);
    if now - last_summon < 600 {
        let remain = 600 - (now - last_summon);
        return format!("召唤冷却中，还需{}秒", remain);
    }

    // 检查令牌
    let token_count = db.knapsack_quantity(user_id, SUMMON_TOKEN);
    if token_count < boss.token_cost {
        return format!(
            "召唤{}需要{}个{}，你当前有{}个",
            boss.name, boss.token_cost, SUMMON_TOKEN, token_count
        );
    }

    // 消耗令牌
    db.knapsack_remove(user_id, SUMMON_TOKEN, boss.token_cost);

    // 战斗模拟
    let user_hp: i32 = db.read_basic(user_id, "HP_Max").parse().unwrap_or(100);
    let user_ad: i32 = db.read_basic(user_id, "AD").parse().unwrap_or(10);
    let user_def: i32 = db.read_basic(user_id, "Defense").parse().unwrap_or(5);
    let user_mr: i32 = db.read_basic(user_id, "MagicResistance").parse().unwrap_or(5);

    let mut boss_hp = boss.hp;
    let mut player_hp = user_hp as i64;
    let mut total_damage: i64 = 0;
    let mut rounds = 0;

    let seed = {
        let mut h = DefaultHasher::new();
        user_id.hash(&mut h);
        now.hash(&mut h);
        boss.name.hash(&mut h);
        h.finish()
    };

    while boss_hp > 0 && player_hp > 0 && rounds < 50 {
        rounds += 1;

        // 玩家攻击
        let hit_roll = ((seed.wrapping_add(rounds as u64 * 7)) % 100) as i32;
        if hit_roll >= 15 {
            // 85%命中
            let is_crit = hit_roll >= 90; // 10%暴击
            let base_dmg = (user_ad as i64 - boss.def as i64).max(1);
            let dmg = if is_crit { base_dmg * 2 } else { base_dmg };
            boss_hp -= dmg;
            total_damage += dmg;
        }

        if boss_hp <= 0 {
            break;
        }

        // Boss攻击
        let boss_hit = ((seed.wrapping_add(rounds as u64 * 13)) % 100) as i32;
        if boss_hit >= 20 {
            // 80%命中
            let boss_dmg = (boss.ad as i64 - user_def as i64 - user_mr as i64 / 2).max(1);
            player_hp -= boss_dmg;
        }
    }

    let won = boss_hp <= 0;

    // 更新冷却
    db.global_set("boss_summon", &format!("cooldown_{}", user_id), &now.to_string());

    // 记录战斗结果
    let result_str = if won { "WIN" } else { "LOSE" };
    let record_entry = format!("{}|{}|{}|{}", boss.name, result_str, total_damage, now);
    let mut records = db.global_get("boss_summon", &format!("records_{}", user_id));
    if !records.is_empty() {
        records.push('\n');
    }
    records.push_str(&record_entry);
    // 保留最近20条
    let lines: Vec<&str> = records.lines().collect();
    if lines.len() > 20 {
        records = lines[lines.len() - 20..].join("\n");
    }
    db.global_set("boss_summon", &format!("records_{}", user_id), &records);

    // 更新统计
    let kills_key = format!("kills_{}", user_id);
    let kills: i32 = db.global_get("boss_summon", &kills_key).parse().unwrap_or(0);
    if won {
        db.global_set("boss_summon", &kills_key, &(kills + 1).to_string());
    }

    // 输出
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}═══ {} {}挑战 ═══\n\n", prefix, boss.emoji, boss.name);
    out.push_str(&format!("回合数: {}\n", rounds));
    out.push_str(&format!("造成伤害: {}\n", format_num(total_damage)));
    out.push_str(&format!("Boss剩余HP: {}\n\n", format_num(boss_hp.max(0))));

    if won {
        // VIP加成
        let vip_bonus = vip::get_vip_exp_bonus(db, user_id) as f64 / 100.0 + 1.0;
        let final_gold = (boss.reward_gold as f64 * vip_bonus) as i64;
        let final_exp = (boss.reward_exp as f64 * vip_bonus) as i32;

        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, final_gold);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, boss.reward_diamond as i64);
        user::add_experience(db, user_id, final_exp);

        out.push_str("🏆 战斗胜利！\n");
        out.push_str(&format!("💰 金币: +{}\n", format_num(final_gold)));
        out.push_str(&format!("💎 钻石: +{}\n", boss.reward_diamond));
        out.push_str(&format!("⭐ 经验: +{}\n", format_num(final_exp as i64)));

        // 概率掉落
        let drop_seed = ((seed.wrapping_add(total_damage as u64)) % 1000) as f64 / 1000.0;
        if drop_seed < boss.drop_rate {
            db.knapsack_add(user_id, boss.drop_item, 1);
            out.push_str(&format!("🎁 掉落: {} ×1\n", boss.drop_item));
        }
    } else {
        out.push_str("💀 挑战失败！Boss太强了\n");
        out.push_str("💡 提示: 提升装备和等级后再来挑战\n");
    }

    out
}

/// 查看召唤记录
pub fn cmd_summon_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let records = db.global_get("boss_summon", &format!("records_{}", user_id));
    let kills: i32 = db
        .global_get("boss_summon", &format!("kills_{}", user_id))
        .parse()
        .unwrap_or(0);

    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}═══ 📋 召唤记录 ═══\n\n", prefix);
    out.push_str(&format!("累计召唤胜利: {}次\n\n", kills));

    if records.is_empty() {
        out.push_str("暂无召唤记录\n发送「召唤列表」查看可召唤Boss");
    } else {
        let lines: Vec<&str> = records.lines().collect();
        let start = if lines.len() > 10 { lines.len() - 10 } else { 0 };
        for line in &lines[start..] {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 4 {
                let boss_name = parts[0];
                let result = parts[1];
                let damage: i64 = parts[2].parse().unwrap_or(0);
                let ts: i64 = parts[3].parse().unwrap_or(0);
                let icon = if result == "WIN" { "🏆" } else { "💀" };
                let time_str = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|t| t.format("%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "??:??".to_string());
                out.push_str(&format!(
                    "{} {} | 伤害:{} | {}\n",
                    icon,
                    boss_name,
                    format_num(damage),
                    time_str
                ));
            }
        }
    }
    out
}

/// 召唤排行
pub fn cmd_summon_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut entries: Vec<(String, i32)> = Vec::new();

    // 从Basic_User获取所有用户
    let users = db.all_users();
    for uid in users {
        let kills: i32 = db
            .global_get("boss_summon", &format!("kills_{}", uid))
            .parse()
            .unwrap_or(0);
        if kills > 0 {
            entries.push((uid, kills));
        }
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = String::from("═══ 🏅 召唤Boss排行 ═══\n\n");
    if entries.is_empty() {
        out.push_str("暂无召唤记录\n");
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (uid, kills)) in entries.iter().take(10).enumerate() {
            let medal = if i < 3 {
                medals[i]
            } else {
                // safe: format! won't panic
                &format!("{:>2}.", i + 1)
            };
            let name = db.read_basic(uid, "NickName");
            out.push_str(&format!("{} {} - {}次击杀\n", medal, name, kills));
        }
    }
    out
}

/// 数字格式化
fn format_num(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_count() {
        assert_eq!(SUMMON_BOSSES.len(), 5);
    }

    #[test]
    fn test_boss_level_ordering() {
        for i in 1..SUMMON_BOSSES.len() {
            assert!(SUMMON_BOSSES[i].min_level >= SUMMON_BOSSES[i - 1].min_level);
        }
    }

    #[test]
    fn test_boss_hp_scaling() {
        for i in 1..SUMMON_BOSSES.len() {
            assert!(SUMMON_BOSSES[i].hp > SUMMON_BOSSES[i - 1].hp);
        }
    }

    #[test]
    fn test_boss_token_cost_scaling() {
        for i in 1..SUMMON_BOSSES.len() {
            assert!(SUMMON_BOSSES[i].token_cost >= SUMMON_BOSSES[i - 1].token_cost);
        }
    }

    #[test]
    fn test_boss_rewards_positive() {
        for boss in SUMMON_BOSSES {
            assert!(boss.reward_gold > 0);
            assert!(boss.reward_diamond > 0);
            assert!(boss.reward_exp > 0);
        }
    }

    #[test]
    fn test_boss_drop_rate_range() {
        for boss in SUMMON_BOSSES {
            assert!(boss.drop_rate > 0.0 && boss.drop_rate <= 1.0);
        }
    }

    #[test]
    fn test_boss_names_unique() {
        let mut names: Vec<&str> = SUMMON_BOSSES.iter().map(|b| b.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SUMMON_BOSSES.len());
    }

    #[test]
    fn test_boss_emojis_non_empty() {
        for boss in SUMMON_BOSSES {
            assert!(!boss.emoji.is_empty());
        }
    }

    #[test]
    fn test_boss_ad_scaling() {
        for i in 1..SUMMON_BOSSES.len() {
            assert!(SUMMON_BOSSES[i].ad > SUMMON_BOSSES[i - 1].ad);
        }
    }

    #[test]
    fn test_format_num_units() {
        assert_eq!(format_num(500), "500");
        assert_eq!(format_num(1500), "1.5K");
        assert_eq!(format_num(1500000), "1.5M");
    }

    #[test]
    fn test_summon_token_name() {
        assert_eq!(SUMMON_TOKEN, "Boss召唤令");
    }

    #[test]
    fn test_boss_drop_items_unique() {
        let mut items: Vec<&str> = SUMMON_BOSSES.iter().map(|b| b.drop_item).collect();
        items.sort();
        items.dedup();
        assert_eq!(items.len(), SUMMON_BOSSES.len());
    }
}

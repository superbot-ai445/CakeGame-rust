/// CakeGame 世界BOSS突袭系统
/// 全服协作击败巨型世界BOSS，所有玩家贡献伤害
/// 每日刷新世界BOSS，根据伤害排名发放奖励
/// 数据存储: Global 表 SECTION='world_boss'
use crate::core::*;
use crate::db::Database;
use crate::user;
use crate::vip;
use chrono::{Datelike, Local, Timelike};
use rand::Rng;

/// 世界BOSS定义
struct WorldBossDef {
    name: &'static str,
    title: &'static str,
    emoji: &'static str,
    max_hp: i64,
    attack: i32,
    defense: i32,
    magic_res: i32,
    level: i32,
    /// 掉落奖励: (物品名, 概率%)
    drops: &'static [(&'static str, f64)],
    /// 基础经验奖励
    reward_exp: i32,
    /// 基础金币奖励
    reward_gold: i64,
    /// 基础钻石奖励
    reward_diamond: i32,
    /// 出现时段 (小时, 0-23)
    spawn_hour: u32,
    description: &'static str,
}

/// 世界BOSS池 (每天随机选一个)
const WORLD_BOSSES: &[WorldBossDef] = &[
    WorldBossDef {
        name: "暗影巨龙",
        title: "毁灭之翼",
        emoji: "🐉",
        max_hp: 100_000,
        attack: 500,
        defense: 200,
        magic_res: 200,
        level: 50,
        drops: &[("龙鳞碎片", 80.0), ("暗影精华", 40.0), ("巨龙之心", 10.0)],
        reward_exp: 500,
        reward_gold: 5000,
        reward_diamond: 50,
        spawn_hour: 0,
        description: "远古暗影巨龙，毁灭一切的力量",
    },
    WorldBossDef {
        name: "冰霜女王",
        title: "永冬之主",
        emoji: "❄️",
        max_hp: 80_000,
        attack: 400,
        defense: 150,
        magic_res: 300,
        level: 45,
        drops: &[("冰晶石", 80.0), ("寒冰权杖碎片", 30.0), ("永冬之冠", 8.0)],
        reward_exp: 400,
        reward_gold: 4000,
        reward_diamond: 40,
        spawn_hour: 6,
        description: "掌控冰霜之力的古老女王",
    },
    WorldBossDef {
        name: "熔岩魔神",
        title: "烈焰之怒",
        emoji: "🔥",
        max_hp: 120_000,
        attack: 600,
        defense: 250,
        magic_res: 100,
        level: 55,
        drops: &[("熔岩结晶", 80.0), ("魔神之血", 35.0), ("烈焰之核", 12.0)],
        reward_exp: 600,
        reward_gold: 6000,
        reward_diamond: 60,
        spawn_hour: 12,
        description: "沉睡千年的熔岩魔神苏醒了",
    },
    WorldBossDef {
        name: "虚空领主",
        title: "时空裂隙",
        emoji: "🌀",
        max_hp: 150_000,
        attack: 700,
        defense: 300,
        magic_res: 300,
        level: 60,
        drops: &[("虚空碎片", 80.0), ("时空之钥", 25.0), ("次元之心", 5.0)],
        reward_exp: 800,
        reward_gold: 8000,
        reward_diamond: 80,
        spawn_hour: 18,
        description: "来自虚空深处的恐怖存在",
    },
    WorldBossDef {
        name: "死亡骑士",
        title: "永恒亡灵",
        emoji: "💀",
        max_hp: 200_000,
        attack: 800,
        defense: 350,
        magic_res: 250,
        level: 65,
        drops: &[("亡灵骨片", 80.0), ("死亡之刃碎片", 20.0), ("不死者之心", 3.0)],
        reward_exp: 1000,
        reward_gold: 10000,
        reward_diamond: 100,
        spawn_hour: 21,
        description: "统率亡灵军团的死亡骑士",
    },
];

/// 获取今天的世界BOSS索引 (基于日期确定性)
fn get_today_boss_index() -> usize {
    let now = Local::now();
    let day_seed = now.year() as usize * 10000 + now.month() as usize * 100 + now.day() as usize;
    day_seed % WORLD_BOSSES.len()
}

/// 世界BOSS是否已激活（当前时段BOSS可被攻击）
fn is_boss_active(boss: &WorldBossDef) -> bool {
    let now = Local::now();
    let hour = now.hour();
    // BOSS在spawn_hour后激活，持续12小时
    let start = boss.spawn_hour;
    let end = (start + 12) % 24;
    if start < end {
        hour >= start && hour < end
    } else {
        hour >= start || hour < end
    }
}

/// 读取世界BOSS当前状态
fn read_boss_state(db: &Database) -> (i64, i64, String) {
    let hp_str = db.global_get("world_boss", "current_hp");
    let hp: i64 = hp_str.parse().unwrap_or(-1);
    let total_dmg: i64 = db.global_get("world_boss", "total_damage").parse().unwrap_or(0);
    let date = db.global_get("world_boss", "spawn_date");
    (hp, total_dmg, date)
}

/// 初始化/重置世界BOSS
fn spawn_boss(db: &Database, boss: &WorldBossDef) {
    let today = Local::now().format("%Y-%m-%d").to_string();
    db.global_set("world_boss", "current_hp", &boss.max_hp.to_string());
    db.global_set("world_boss", "max_hp", &boss.max_hp.to_string());
    db.global_set("world_boss", "boss_name", boss.name);
    db.global_set("world_boss", "spawn_date", &today);
    db.global_set("world_boss", "total_damage", "0");
    db.global_set("world_boss", "participants", "0");
    db.global_set("world_boss", "defeated", "false");
    // 清除昨日排名
    db.global_set("world_boss", "rankings", "");
}

/// 记录玩家伤害
fn record_damage(db: &Database, user_id: &str, damage: i64) {
    let key = format!("dmg_{}", user_id);
    let prev: i64 = db.global_get("world_boss", &key).parse().unwrap_or(0);
    db.global_set("world_boss", &key, &(prev + damage).to_string());

    // 更新参与者列表
    let participants_str = db.global_get("world_boss", "participants_list");
    let mut participants: Vec<String> = if participants_str.is_empty() {
        Vec::new()
    } else {
        participants_str.split(',').map(|s| s.to_string()).collect()
    };
    if !participants.contains(&user_id.to_string()) {
        participants.push(user_id.to_string());
        db.global_set("world_boss", "participants_list", &participants.join(","));
        db.global_set("world_boss", "participants", &participants.len().to_string());
    }

    // 更新总伤害
    let total: i64 = db.global_get("world_boss", "total_damage").parse().unwrap_or(0);
    db.global_set("world_boss", "total_damage", &(total + damage).to_string());
}

/// 格式化血量条
fn format_hp_bar(current: i64, max: i64, width: usize) -> String {
    let pct = if max > 0 {
        (current as f64 / max as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    format!("{} {:.1}%", bar, pct * 100.0)
}

/// 查看世界BOSS
pub fn cmd_view_world_boss(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let idx = get_today_boss_index();
    let boss = &WORLD_BOSSES[idx];
    let (current_hp, _total_dmg, spawn_date) = read_boss_state(db);
    let today = Local::now().format("%Y-%m-%d").to_string();

    // 自动刷新BOSS
    if spawn_date != today || current_hp < 0 {
        spawn_boss(db, boss);
    }

    let (current_hp, total_dmg, _) = read_boss_state(db);
    let is_active = is_boss_active(boss);
    let participants: usize = db.global_get("world_boss", "participants").parse().unwrap_or(0);
    let defeated = db.global_get("world_boss", "defeated") == "true";

    let mut r = format!(
        "{}\n═══ {} 世界BOSS: {} {} ═══\n{}",
        prefix, boss.emoji, boss.name, boss.title, boss.description
    );

    r.push_str("\n\n📊 BOSS信息:");
    r.push_str(&format!("\n  等级: {}", boss.level));
    r.push_str(&format!(
        "\n  攻击: {} | 防御: {} | 魔抗: {}",
        boss.attack, boss.defense, boss.magic_res
    ));

    if defeated {
        r.push_str("\n\n✅ 今日世界BOSS已被击败！");
        r.push_str(&format!("\n总伤害: {}", format_damage(total_dmg)));
        r.push_str(&format!("\n参与者: {} 人", participants));
        r.push_str("\n\n明日将刷新新的世界BOSS。");
    } else {
        r.push_str("\n\n❤️ 生命值:");
        r.push_str(&format!("\n  {}", format_hp_bar(current_hp, boss.max_hp, 20)));
        r.push_str(&format!(
            "\n  {}/{}",
            format_damage(current_hp),
            format_damage(boss.max_hp)
        ));
        r.push_str("\n\n⚔️ 战况:");
        r.push_str(&format!("\n  总伤害: {}", format_damage(total_dmg)));
        r.push_str(&format!("\n  参与人数: {} 人", participants));

        if is_active {
            r.push_str("\n\n🟢 BOSS已激活！发送 '挑战世界BOSS' 参与战斗");
        } else {
            r.push_str(&format!(
                "\n\n🔴 BOSS尚未活跃 (活跃时段: {}:00 - {}:00)",
                boss.spawn_hour,
                (boss.spawn_hour + 12) % 24
            ));
        }
    }

    // 显示掉落预览
    r.push_str("\n\n🎁 击败掉落:");
    for &(item, prob) in boss.drops {
        r.push_str(&format!("\n  {} — {:.0}%", item, prob));
    }

    r.push_str(&format!(
        "\n\n💰 基础奖励: {}金币 + {}经验 + {}钻石",
        format_damage(boss.reward_gold),
        boss.reward_exp,
        boss.reward_diamond
    ));
    r.push_str("\n\n发送 '挑战世界BOSS' 对世界BOSS造成伤害");
    r.push_str("\n发送 '世界BOSS排名' 查看伤害排名");
    r
}

/// 挑战世界BOSS
pub fn cmd_challenge_world_boss(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let idx = get_today_boss_index();
    let boss = &WORLD_BOSSES[idx];
    let (current_hp, _total_dmg, spawn_date) = read_boss_state(db);
    let today = Local::now().format("%Y-%m-%d").to_string();

    if spawn_date != today || current_hp < 0 {
        spawn_boss(db, boss);
    }

    let (current_hp, _, _) = read_boss_state(db);

    // 检查BOSS是否已被击败
    if current_hp <= 0 || db.global_get("world_boss", "defeated") == "true" {
        return format!(
            "{}\n今日世界BOSS [{}] 已被击败！\n明日将刷新新的世界BOSS。",
            prefix, boss.name
        );
    }

    // 检查BOSS是否活跃
    if !is_boss_active(boss) {
        return format!(
            "{}\n世界BOSS [{}] 尚未活跃！\n活跃时段: {}:00 - {}:00",
            prefix,
            boss.name,
            boss.spawn_hour,
            (boss.spawn_hour + 12) % 24
        );
    }

    // 检查玩家是否存活
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再挑战世界BOSS。", prefix);
    }

    // 检查冷却 (每玩家每10分钟可挑战一次)
    let cd_key = format!("cd_{}", user_id);
    let last_attack: i64 = db.global_get("world_boss", &cd_key).parse().unwrap_or(0);
    let now_ts = Local::now().timestamp();
    let cooldown_secs = 600; // 10 minutes
    if now_ts - last_attack < cooldown_secs {
        let remaining = cooldown_secs - (now_ts - last_attack);
        return format!("{}\n⏳ 世界BOSS挑战冷却中！还需等待 {}秒。", prefix, remaining);
    }

    // 计算伤害
    let info = user::calc_total_attrs(db, user_id);
    let mut rng = rand::thread_rng();

    let base_dmg = (info.ad + info.ap - boss.defense - boss.magic_res).max(10) as i64;
    let variance = rng.gen_range(-base_dmg.abs().max(1)..=base_dmg.abs().max(1));
    let raw_dmg = (base_dmg + variance).max(1);
    let is_crit = rng.gen_range(0..100) < info.crit;
    let damage = if is_crit { raw_dmg * 2 } else { raw_dmg };

    // VIP加成
    let vip_bonus_pct = vip::get_vip_exp_bonus(db, user_id) as i64;
    let final_damage = damage + damage * vip_bonus_pct / 100;

    // 记录冷却
    db.global_set("world_boss", &cd_key, &now_ts.to_string());

    // 记录伤害
    record_damage(db, user_id, final_damage);

    // 扣除BOSS血量
    let new_hp = (current_hp - final_damage).max(0);
    db.global_set("world_boss", "current_hp", &new_hp.to_string());

    // 构建战斗日志
    let mut r = format!("{}\n═══ {} 挑战世界BOSS: {} ═══", prefix, boss.emoji, boss.name);

    if is_crit {
        r.push_str(&format!(
            "\n💥 暴击！你对 {} 造成 {} 点伤害！",
            boss.name,
            format_damage(final_damage)
        ));
    } else {
        r.push_str(&format!(
            "\n⚔️ 你对 {} 造成 {} 点伤害！",
            boss.name,
            format_damage(final_damage)
        ));
    }

    if vip_bonus_pct > 0 {
        r.push_str(&format!(" (VIP+{}%)", vip_bonus_pct));
    }

    r.push_str(&format!(
        "\n\n❤️ BOSS剩余血量: {}",
        format_hp_bar(new_hp, boss.max_hp, 15)
    ));
    r.push_str(&format!("\n  {}/{}", format_damage(new_hp), format_damage(boss.max_hp)));

    // 检查是否击败
    if new_hp <= 0 {
        db.global_set("world_boss", "defeated", "true");
        r.push_str(&format!("\n\n🎉🎉🎉 {} 已被全服勇士击败！🎉🎉🎉", boss.name));

        // 发放个人奖励
        let reward_exp = boss.reward_exp + (info.level * 10);
        let reward_gold = boss.reward_gold + (info.level as i64 * 100);
        let reward_diamond = boss.reward_diamond;

        let (_, leveled_up) = user::add_experience(db, user_id, reward_exp);
        let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", reward_gold);
        let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", reward_diamond as i64);

        r.push_str(&format!(
            "\n\n💰 你的奖励: {}金币 + {}经验 + {}钻石",
            format_damage(reward_gold),
            reward_exp,
            reward_diamond
        ));
        if leveled_up {
            r.push_str(" ★ 升级了！");
        }

        // 掉落物品
        let mut drops_got: Vec<String> = Vec::new();
        for &(item, prob) in boss.drops {
            let roll: f64 = rng.gen_range(0.0..100.0);
            if roll < prob {
                db.knapsack_add(user_id, item, 1);
                drops_got.push(item.to_string());
            }
        }
        if !drops_got.is_empty() {
            r.push_str("\n\n🎁 掉落物品:");
            for item in &drops_got {
                r.push_str(&format!("\n  ✦ {}", item));
            }
        }

        // 记录击败者
        db.global_set("world_boss", "defeated_by", user_id);
    }

    r.push_str("\n\n⏳ 下次可挑战: 10分钟后\n发送 '世界BOSS排名' 查看伤害排名");

    r
}

/// 世界BOSS排名
pub fn cmd_world_boss_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let idx = get_today_boss_index();
    let boss = &WORLD_BOSSES[idx];
    let participants_str = db.global_get("world_boss", "participants_list");

    if participants_str.is_empty() {
        return format!(
            "{}\n今日尚无世界BOSS战斗记录。\n发送 '查看世界BOSS' 查看BOSS信息。",
            prefix
        );
    }

    let participants: Vec<&str> = participants_str.split(',').collect();
    let mut rankings: Vec<(String, i64)> = Vec::new();

    for pid in &participants {
        let key = format!("dmg_{}", pid);
        let dmg: i64 = db.global_get("world_boss", &key).parse().unwrap_or(0);
        if dmg > 0 {
            let nickname = db.read_basic(pid, ITEM_NAME);
            rankings.push((format!("{}({})", nickname, pid), dmg));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n═══ {} {} 伤害排名 ═══", prefix, boss.emoji, boss.name);

    let (_, total_dmg, _) = read_boss_state(db);
    let defeated = db.global_get("world_boss", "defeated") == "true";

    if defeated {
        r.push_str("\n✅ 今日BOSS已被击败！");
    }
    r.push_str(&format!(
        "\n总伤害: {} | 参与: {}人\n",
        format_damage(total_dmg),
        rankings.len()
    ));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, dmg)) in rankings.iter().take(10).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let pct = if total_dmg > 0 {
            *dmg as f64 / total_dmg as f64 * 100.0
        } else {
            0.0
        };
        r.push_str(&format!(
            "\n{}{}. {} — {} ({:.1}%)",
            medal,
            i + 1,
            name,
            format_damage(*dmg),
            pct
        ));
    }

    // 显示当前用户排名
    let user_key = format!("dmg_{}", user_id);
    let user_dmg: i64 = db.global_get("world_boss", &user_key).parse().unwrap_or(0);
    if user_dmg > 0 {
        let user_rank = rankings
            .iter()
            .position(|(n, _)| n.contains(user_id))
            .map(|p| p + 1)
            .unwrap_or(0);
        r.push_str(&format!(
            "\n\n📍 你的排名: 第{}名 | 伤害: {}",
            user_rank,
            format_damage(user_dmg)
        ));
    } else {
        r.push_str("\n\n你尚未参与今日世界BOSS战斗。");
        r.push_str("\n发送 '挑战世界BOSS' 参与战斗！");
    }

    r
}

/// 格式化大数字
fn format_damage(n: i64) -> String {
    if n >= 100_000_000 {
        format!("{:.1}亿", n as f64 / 100_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}万", n as f64 / 10_000.0)
    } else {
        format!("{}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_damage() {
        assert_eq!(format_damage(500), "500");
        assert_eq!(format_damage(9999), "9999");
        assert_eq!(format_damage(10000), "1.0万");
        assert_eq!(format_damage(55000), "5.5万");
        assert_eq!(format_damage(100_000_000), "1.0亿");
        assert_eq!(format_damage(0), "0");
    }

    #[test]
    fn test_format_hp_bar() {
        let bar = format_hp_bar(50, 100, 10);
        assert!(bar.contains("50.0%"));
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));

        let full = format_hp_bar(100, 100, 10);
        assert!(full.contains("100.0%"));

        let empty = format_hp_bar(0, 100, 10);
        assert!(empty.contains("0.0%"));
    }

    #[test]
    fn test_boss_definitions() {
        assert_eq!(WORLD_BOSSES.len(), 5);
        for boss in WORLD_BOSSES {
            assert!(boss.max_hp > 0);
            assert!(boss.attack > 0);
            assert!(boss.defense > 0);
            assert!(boss.reward_exp > 0);
            assert!(boss.reward_gold > 0);
            assert!(boss.spawn_hour < 24);
            assert!(!boss.drops.is_empty());
        }
    }

    #[test]
    fn test_today_boss_index_range() {
        let idx = get_today_boss_index();
        assert!(idx < WORLD_BOSSES.len());
    }

    #[test]
    fn test_boss_active_hours() {
        let boss = &WORLD_BOSSES[0]; // spawn_hour=0, active 0-12
                                     // Just verify it doesn't panic
        let _active = is_boss_active(boss);
    }

    #[test]
    fn test_hp_bar_edge_cases() {
        // negative HP
        let bar = format_hp_bar(-100, 1000, 10);
        assert!(bar.contains("0.0%"));

        // HP > max
        let bar = format_hp_bar(2000, 1000, 10);
        assert!(bar.contains("100.0%"));
    }

    #[test]
    fn test_damage_format_large() {
        assert_eq!(format_damage(1_234_567_890), "12.3亿");
        assert_eq!(format_damage(99_999), "10.0万");
    }
}

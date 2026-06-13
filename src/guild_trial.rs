/// CakeGame 公会试炼系统
///
/// 公会级别的PvE挑战，所有公会成员合作击败试炼BOSS。
/// 每周重置，公会逐步解锁更高难度的试炼关卡。
///
/// 指令: 公会试炼, 挑战试炼, 试炼进度
use crate::core::*;
use crate::db::Database;
use crate::user;
use crate::vip;
use chrono::{Datelike, Local};

/// 试炼BOSS定义
struct TrialBoss {
    level: i32,
    name: &'static str,
    hp: i64,
    reward_gold: i64,
    reward_exp: i32,
    reward_diamond: i32,
    reward_item: &'static str,
    intro: &'static str,
}

const TRIAL_BOSSES: &[TrialBoss] = &[
    TrialBoss {
        level: 1,
        name: "哥布林将军",
        hp: 5000,
        reward_gold: 500,
        reward_exp: 100,
        reward_diamond: 5,
        reward_item: "【普通】生命药水*3",
        intro: "哥布林部落的首领，率领部众入侵公会领地。击败它可获取基础物资！",
    },
    TrialBoss {
        level: 2,
        name: "暗影骑士",
        hp: 15000,
        reward_gold: 1000,
        reward_exp: 300,
        reward_diamond: 10,
        reward_item: "强化石*2",
        intro: "堕落的骑士被黑暗力量侵蚀，已成为暗影军团的先锋。需要公会全力应对！",
    },
    TrialBoss {
        level: 3,
        name: "远古巨龙",
        hp: 50000,
        reward_gold: 3000,
        reward_exp: 800,
        reward_diamond: 20,
        reward_item: "【精良】勇者宝箱*1",
        intro: "沉睡千年的远古巨龙苏醒了！它的龙息可以摧毁一切，公会需要团结一致！",
    },
    TrialBoss {
        level: 4,
        name: "深渊领主",
        hp: 100000,
        reward_gold: 5000,
        reward_exp: 1500,
        reward_diamond: 35,
        reward_item: "【史诗】传奇宝箱*1",
        intro: "来自深渊的恶魔领主，拥有毁灭性的黑暗魔法。这是对公会的终极考验！",
    },
    TrialBoss {
        level: 5,
        name: "终焉审判者",
        hp: 200000,
        reward_gold: 10000,
        reward_exp: 3000,
        reward_diamond: 60,
        reward_item: "复活卷轴*1",
        intro: "传说中的终焉审判者，万物终结的化身。只有最强的公会才能挑战它！",
    },
];

/// 获取本周一的日期字符串（试炼每周重置）
fn get_week_start() -> String {
    let now = Local::now();
    let days_since_monday = now.weekday().num_days_from_monday();
    let monday = now - chrono::Duration::days(days_since_monday as i64);
    monday.format("%Y-%m-%d").to_string()
}

/// 读取公会试炼数据
fn read_trial_data(db: &Database, guild: &str, key: &str) -> String {
    db.global_get("GuildTrial", &format!("{}.{}", guild, key))
}

/// 写入公会试炼数据
fn write_trial_data(db: &Database, guild: &str, key: &str, value: &str) {
    db.global_set("GuildTrial", &format!("{}.{}", guild, key), value);
}

/// 初始化或重置公会试炼（每周一重置）
fn ensure_trial(db: &Database, guild: &str) {
    let saved_week = read_trial_data(db, guild, "Week");
    let current_week = get_week_start();

    if saved_week != current_week {
        // 新的一周，重置试炼
        write_trial_data(db, guild, "Week", &current_week);
        write_trial_data(db, guild, "Level", "1");
        write_trial_data(db, guild, "Damage", "0");
        write_trial_data(db, guild, "Participants", "");
        write_trial_data(db, guild, "Completed", "0");
    }
}

/// 获取当前试炼BOSS
fn get_current_boss(db: &Database, guild: &str) -> &'static TrialBoss {
    let level: i32 = read_trial_data(db, guild, "Level").parse().unwrap_or(1);
    let idx = (level - 1).max(0) as usize;
    TRIAL_BOSSES.get(idx).unwrap_or(&TRIAL_BOSSES[0])
}

/// 计算玩家对试炼BOSS的伤害
fn calc_trial_damage(db: &Database, user_id: &str) -> i64 {
    let info = user::calc_total_attrs(db, user_id);
    let ad = info.ad.max(1) as f64;
    let ap = info.ap.max(0) as f64;
    let level = info.level as f64;

    // 基础伤害 = (物攻 + 魔攻) * 等级加成
    let base = ad + ap;
    let level_bonus = 1.0 + (level * 0.05);
    let damage = (base * level_bonus) as i64;

    // VIP加成
    let vip_mult = if vip::get_vip_level(db, user_id) > 0 { 1.2 } else { 1.0 };
    let final_damage = (damage as f64 * vip_mult) as i64;
    final_damage.max(10) // 最低10点伤害
}

/// 查看公会试炼
pub fn cmd_guild_trial(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！无法参与公会试炼。", prefix);
    }

    ensure_trial(db, &guild);
    let boss = get_current_boss(db, &guild);
    let damage: i64 = read_trial_data(db, &guild, "Damage").parse().unwrap_or(0);
    let completed: i32 = read_trial_data(db, &guild, "Completed").parse().unwrap_or(0);
    let hp_remaining = (boss.hp - damage).max(0);

    let mut result = format!(
        "{}\n╔════════════════════════════╗\n║    ⚔️  公会试炼  ⚔️    ║\n╚════════════════════════════╝",
        prefix
    );

    result.push_str(&format!("\n🏰 公会: [{}]", guild));
    result.push_str(&format!("\n📊 当前关卡: 第{}关", boss.level));

    if completed > 0 {
        result.push_str(&format!("\n✅ 本周已完成{}关试炼！", completed));
    }

    result.push_str(&format!("\n\n👹 BOSS: {} ({}级)", boss.name, boss.level));
    result.push_str(&format!("\n📖 {}", boss.intro));
    result.push_str(&format!("\n❤️ 生命: {}/{}", hp_remaining, boss.hp));

    // 生命条
    let bar_len = 20;
    let filled = if boss.hp > 0 {
        ((hp_remaining as f64 / boss.hp as f64) * bar_len as f64) as usize
    } else {
        0
    };
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
    result.push_str(&format!("\n   [{}]", bar));

    if hp_remaining <= 0 {
        result.push_str("\n\n🎉 本周试炼已通关！下周一会重置。");
        // 显示下一关预览
        if (boss.level as usize) < TRIAL_BOSSES.len() {
            let next = &TRIAL_BOSSES[boss.level as usize];
            result.push_str(&format!(
                "\n\n📋 下一关预告: {} ({}级)\n   {}",
                next.name, next.level, next.intro
            ));
        } else {
            result.push_str("\n\n🏆 恭喜！公会已完成全部试炼关卡！");
        }
    } else {
        result.push_str(&format!(
            "\n\n🎁 通关奖励:\n   金币: +{}\n   经验: +{}\n   钻石: +{}",
            boss.reward_gold, boss.reward_exp, boss.reward_diamond
        ));
        if !boss.reward_item.is_empty() {
            result.push_str(&format!("\n   物品: {}", boss.reward_item));
        }
        result.push_str("\n\n发送 '挑战试炼' 对BOSS造成伤害！");
    }

    // 参与人数
    let participants = read_trial_data(db, &guild, "Participants");
    let count = if participants.is_empty() {
        0
    } else {
        participants.split(',').count()
    };
    result.push_str(&format!("\n\n👥 本周参与人数: {}", count));

    result
}

/// 挑战试炼BOSS
pub fn cmd_challenge_trial(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！无法挑战试炼。", prefix);
    }

    ensure_trial(db, &guild);

    // 检查玩家是否存活
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命值再挑战试炼！", prefix);
    }

    let boss = get_current_boss(db, &guild);
    let damage_done: i64 = read_trial_data(db, &guild, "Damage").parse().unwrap_or(0);
    let hp_remaining = boss.hp - damage_done;

    if hp_remaining <= 0 {
        return format!("{}\n本周试炼已通关！等待下周重置。", prefix);
    }

    // 计算伤害
    let damage = calc_trial_damage(db, user_id);
    let actual_damage = damage.min(hp_remaining);
    let new_damage = damage_done + actual_damage;

    // 更新伤害
    write_trial_data(db, &guild, "Damage", &new_damage.to_string());

    // 记录参与者
    let mut participants = read_trial_data(db, &guild, "Participants");
    if !participants.contains(user_id) {
        if !participants.is_empty() {
            participants.push(',');
        }
        participants.push_str(user_id);
        write_trial_data(db, &guild, "Participants", &participants);
    }

    // 记录个人累计伤害
    let personal_key = format!("PersonalDamage.{}", user_id);
    let personal_damage: i64 = read_trial_data(db, &guild, &personal_key).parse().unwrap_or(0);
    write_trial_data(
        db,
        &guild,
        &personal_key,
        &(personal_damage + actual_damage).to_string(),
    );

    let new_hp_remaining = (boss.hp - new_damage).max(0);
    let mut result = format!("{}\n⚔️ 您对[{}]造成了 {} 点伤害！", prefix, boss.name, actual_damage);

    // VIP加成提示
    if vip::get_vip_level(db, user_id) > 0 {
        result.push_str(" (VIP加成20%)");
    }

    result.push_str(&format!(
        "\n👹 {} 剩余生命: {}/{}",
        boss.name, new_hp_remaining, boss.hp
    ));

    // 生命条
    let bar_len = 20;
    let filled = if boss.hp > 0 {
        ((new_hp_remaining as f64 / boss.hp as f64) * bar_len as f64) as usize
    } else {
        0
    };
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
    result.push_str(&format!("\n   [{}]", bar));

    if new_hp_remaining <= 0 {
        // BOSS被击败！发放奖励
        result.push_str(&format!("\n\n🎉🎉🎉 {} 已被击败！！！", boss.name));

        // 给参与者发放奖励
        let reward_gold = boss.reward_gold;
        let reward_exp = boss.reward_exp;
        let reward_diamond = boss.reward_diamond;

        // 当前挑战者获得奖励
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, reward_gold);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, reward_diamond as i64);
        user::add_experience(db, user_id, reward_exp);

        // 发放物品奖励
        if !boss.reward_item.is_empty() {
            db.add_item(user_id, boss.reward_item, 1);
        }

        result.push_str(&format!(
            "\n\n🎁 您的奖励:\n   金币: +{}\n   经验: +{}\n   钻石: +{}",
            reward_gold, reward_exp, reward_diamond
        ));
        if !boss.reward_item.is_empty() {
            result.push_str(&format!("\n   物品: {}", boss.reward_item));
        }

        // 为其他参与者也发放奖励（离线奖励方式）
        let participants = read_trial_data(db, &guild, "Participants");
        for pid in participants.split(',') {
            if pid.is_empty() || pid == user_id {
                continue;
            }
            // 存储离线奖励
            let reward_key = format!("TrialReward.{}.{}", boss.level, pid);
            let reward_data = format!("{}|{}|{}|{}", reward_gold, reward_exp, reward_diamond, boss.reward_item);
            write_trial_data(db, &guild, &reward_key, &reward_data);
        }

        // 推进到下一关
        let new_level = boss.level + 1;
        let completed: i32 = read_trial_data(db, &guild, "Completed").parse().unwrap_or(0);
        write_trial_data(db, &guild, "Completed", &(completed + 1).to_string());

        if (new_level as usize) <= TRIAL_BOSSES.len() {
            write_trial_data(db, &guild, "Level", &new_level.to_string());
            write_trial_data(db, &guild, "Damage", "0");
            // 保留参与者名单用于下一关发奖
            let next_boss = &TRIAL_BOSSES[(new_level - 1) as usize];
            result.push_str(&format!(
                "\n\n⬆️ 公会试炼已推进到第{}关！\n👹 下一关: {} — {}",
                new_level, next_boss.name, next_boss.intro
            ));
        } else {
            result.push_str("\n\n🏆 公会已完成全部试炼关卡！下周重置后可再次挑战。");
        }
    } else {
        result.push_str(&format!(
            "\n\n💪 继续努力！再造成 {} 点伤害即可通关！",
            new_hp_remaining
        ));
    }

    result.push_str("\n\n发送 '试炼进度' 查看详细排名");
    result
}

/// 试炼进度（伤害排名）
pub fn cmd_trial_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    ensure_trial(db, &guild);

    let boss = get_current_boss(db, &guild);
    let total_damage: i64 = read_trial_data(db, &guild, "Damage").parse().unwrap_or(0);
    let completed: i32 = read_trial_data(db, &guild, "Completed").parse().unwrap_or(0);
    let participants = read_trial_data(db, &guild, "Participants");

    let mut result = format!(
        "{}\n═══ 公会试炼进度 ═══\n公会: [{}]\n本周已完成: {}关",
        prefix, guild, completed
    );

    // 显示所有关卡状态
    result.push_str("\n\n📋 关卡总览:");
    for (i, tb) in TRIAL_BOSSES.iter().enumerate() {
        let status = if (i as i32) < completed {
            "✅ 已通关"
        } else if (i as i32) == completed {
            "⚔️ 当前关卡"
        } else {
            "🔒 未解锁"
        };
        result.push_str(&format!("\n  {}. {} — {}", i + 1, tb.name, status));
    }

    // 当前BOSS状态
    let hp_remaining = (boss.hp - total_damage).max(0);
    result.push_str(&format!(
        "\n\n👹 当前BOSS: {} (第{}关)\n❤️ 生命: {}/{}",
        boss.name, boss.level, hp_remaining, boss.hp
    ));

    // 伤害排名
    if !participants.is_empty() {
        let mut damages: Vec<(String, i64)> = Vec::new();
        for pid in participants.split(',') {
            if pid.is_empty() {
                continue;
            }
            let personal_key = format!("PersonalDamage.{}", pid);
            let d: i64 = read_trial_data(db, &guild, &personal_key).parse().unwrap_or(0);
            damages.push((pid.to_string(), d));
        }
        damages.sort_by_key(|b| std::cmp::Reverse(b.1));

        result.push_str("\n\n🏆 伤害排名:");
        for (i, (pid, dmg)) in damages.iter().enumerate().take(10) {
            let name = user::get_msg_prefix(db, pid);
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            result.push_str(&format!("\n{}{}. {} — {}伤害", medal, i + 1, name, dmg));
        }

        // 显示自己的排名
        if let Some((rank, _)) = damages.iter().enumerate().find(|(_, (p, _))| p == user_id) {
            let my_damage = damages[rank].1;
            result.push_str(&format!("\n\n📍 您的排名: 第{}名 ({}伤害)", rank + 1, my_damage));
        }
    } else {
        result.push_str("\n\n暂无参与者。发送 '挑战试炼' 开始挑战！");
    }

    // 检查是否有未领取的奖励
    let reward_key = format!("TrialReward.{}.{}", boss.level, user_id);
    let reward_data = read_trial_data(db, &guild, &reward_key);
    if !reward_data.is_empty() {
        result.push_str("\n\n⚠️ 您有未领取的试炼奖励！请查看公会试炼领取。");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trial_boss_count() {
        assert_eq!(TRIAL_BOSSES.len(), 5);
        assert_eq!(TRIAL_BOSSES[0].name, "哥布林将军");
        assert_eq!(TRIAL_BOSSES[4].name, "终焉审判者");
    }

    #[test]
    fn test_trial_boss_hp_progression() {
        // HP应递增
        for i in 1..TRIAL_BOSSES.len() {
            assert!(
                TRIAL_BOSSES[i].hp > TRIAL_BOSSES[i - 1].hp,
                "Boss {} HP should be greater than Boss {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_trial_boss_rewards_progression() {
        // 奖励应递增
        for i in 1..TRIAL_BOSSES.len() {
            assert!(TRIAL_BOSSES[i].reward_gold >= TRIAL_BOSSES[i - 1].reward_gold);
            assert!(TRIAL_BOSSES[i].reward_diamond >= TRIAL_BOSSES[i - 1].reward_diamond);
        }
    }
}

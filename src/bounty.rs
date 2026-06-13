/// CakeGame 赏金任务系统
/// 玩家可在赏金板接取狩猎任务，击杀指定怪物获得额外奖励
/// 数据表: Ext_BountyTask_Info, Ext_BountyTask_log
/// 用户数据: bounty_active = 当前接取的任务名
///           bounty_{name}_count = 已击杀数
///           bounty_{name}_date = 接取日期
use crate::core::*;
use crate::db::Database;
use chrono::Local;

/// 赏金任务定义
struct BountyDef {
    name: &'static str,
    target_monster: &'static str,
    kill_required: i32,
    reward_gold: i64,
    reward_exp: i32,
    reward_item: &'static str,
    reward_item_qty: i32,
    min_level: i32,
    description: &'static str,
}

/// 预设赏金任务（基于 Config_Monster 中的怪物）
const BOUNTIES: &[BountyDef] = &[
    BountyDef {
        name: "哥布林猎手",
        target_monster: "哥布林",
        kill_required: 5,
        reward_gold: 200,
        reward_exp: 100,
        reward_item: "【普通】生命药水",
        reward_item_qty: 3,
        min_level: 1,
        description: "格兰森林的哥布林数量泛滥，请冒险者前往清除",
    },
    BountyDef {
        name: "狼群威胁",
        target_monster: "灰狼",
        kill_required: 3,
        reward_gold: 300,
        reward_exp: 150,
        reward_item: "【普通】魔法药水",
        reward_item_qty: 2,
        min_level: 3,
        description: "磐石之地的灰狼袭击了商队，需要勇士前去讨伐",
    },
    BountyDef {
        name: "史莱姆清除令",
        target_monster: "绿色史莱姆",
        kill_required: 8,
        reward_gold: 150,
        reward_exp: 80,
        reward_item: "【普通】生命药水",
        reward_item_qty: 5,
        min_level: 1,
        description: "沼泽之地到处都是史莱姆，村民们不堪其扰",
    },
    BountyDef {
        name: "骷髅猎杀",
        target_monster: "骷髅兵",
        kill_required: 4,
        reward_gold: 500,
        reward_exp: 250,
        reward_item: "【精良】生命药水",
        reward_item_qty: 2,
        min_level: 5,
        description: "寂寞之路出现了骷髅兵，请有经验的冒险者前往消灭",
    },
    BountyDef {
        name: "火焰蜥蜴",
        target_monster: "火焰蜥蜴",
        kill_required: 3,
        reward_gold: 800,
        reward_exp: 400,
        reward_item: "【精良】魔法药水",
        reward_item_qty: 2,
        min_level: 8,
        description: "熔岩地带的火焰蜥蜴越来越狂暴，需要强者前去镇压",
    },
    BountyDef {
        name: "暗影刺客",
        target_monster: "暗影刺客",
        kill_required: 2,
        reward_gold: 1200,
        reward_exp: 600,
        reward_item: "【稀有】生命药水",
        reward_item_qty: 1,
        min_level: 10,
        description: "暗影组织的刺客出没于古堡附近，悬赏击杀",
    },
    BountyDef {
        name: "巨龙之怒",
        target_monster: "远古巨龙",
        kill_required: 1,
        reward_gold: 3000,
        reward_exp: 1500,
        reward_item: "【传说】生命药水",
        reward_item_qty: 1,
        min_level: 15,
        description: "传说中的远古巨龙再次苏醒，唯有最强的勇士能够挑战",
    },
];

/// 查看赏金任务列表
pub fn cmd_view_bounties(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let active = db.read_user_data(user_id, "bounty_active");

    let mut r = format!("{}\n═══ 赏金任务 ═══\n", prefix);

    for (i, b) in BOUNTIES.iter().enumerate() {
        let locked = level < b.min_level;
        let is_active = active == b.name;

        r.push_str(&format!(
            "\n{}. 【{}】{}",
            i + 1,
            b.name,
            if is_active { " ★进行中" } else { "" }
        ));
        if locked {
            r.push_str(&format!("\n   🔒 需要等级{}", b.min_level));
        } else {
            r.push_str(&format!("\n   {}", b.description));
            r.push_str(&format!("\n   目标: 击杀{} ×{}", b.target_monster, b.kill_required));
            r.push_str(&format!("\n   奖励: {}金币 + {}经验", b.reward_gold, b.reward_exp));
            if b.reward_item_qty > 0 {
                r.push_str(&format!(" + {}×{}", b.reward_item, b.reward_item_qty));
            }
        }
        r.push('\n');
    }

    r.push_str("\n发送'接受赏金+任务名'接取任务");
    r.push_str("\n发送'赏金进度'查看当前任务进度");
    r.push_str("\n发送'提交赏金'提交完成的任务");
    r
}

/// 接受赏金任务
pub fn cmd_accept_bounty(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let name = args.trim();

    if name.is_empty() {
        return format!("{}\n请指定任务名！\n发送'赏金任务'查看可接取的任务", prefix);
    }

    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);

    // 检查是否已有进行中的任务
    let active = db.read_user_data(user_id, "bounty_active");
    if !active.is_empty() {
        return format!(
            "{}\n你已有进行中的赏金任务 [{}]！\n请先完成或放弃当前任务",
            prefix, active
        );
    }

    // 查找匹配的赏金任务
    let bounty = match BOUNTIES.iter().find(|b| b.name == name) {
        Some(b) => b,
        None => {
            return format!("{}\n未找到赏金任务 [{}]\n发送'赏金任务'查看可接取的任务", prefix, name);
        }
    };

    // 等级检查
    if level < bounty.min_level {
        return format!(
            "{}\n等级不足！接取 [{}] 需要等级{}，当前等级{}",
            prefix, name, bounty.min_level, level
        );
    }

    // 设置进行中的任务
    db.write_user_data(user_id, "bounty_active", name);
    db.write_user_data(user_id, &format!("bounty_{}_count", name), "0");
    db.write_user_data(
        user_id,
        &format!("bounty_{}_date", name),
        &Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    // 尝试写入 Ext_BountyTask_log（如果表存在）
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR IGNORE INTO Ext_BountyTask_log (taskMode, fromQQ, taskId, TIMESTAMP, ishis) VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params!["accepted", user_id, name, Local::now().format("%Y-%m-%d %H:%M:%S").to_string()],
    );
    drop(conn);

    format!(
        "{}\n✅ 已接取赏金任务 [{}]\n\n{}\n目标: 击杀{} ×{}\n奖励: {}金币 + {}经验",
        prefix,
        name,
        bounty.description,
        bounty.target_monster,
        bounty.kill_required,
        bounty.reward_gold,
        bounty.reward_exp
    )
}

/// 查看赏金进度
pub fn cmd_bounty_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let active = db.read_user_data(user_id, "bounty_active");

    if active.is_empty() {
        return format!("{}\n你没有进行中的赏金任务\n发送'赏金任务'查看可接取的任务", prefix);
    }

    let bounty = match BOUNTIES.iter().find(|b| b.name == active) {
        Some(b) => b,
        None => return format!("{}\n任务数据异常，请联系管理员", prefix),
    };

    let count: i32 = db
        .read_user_data(user_id, &format!("bounty_{}_count", active))
        .parse()
        .unwrap_or(0);
    let date = db.read_user_data(user_id, &format!("bounty_{}_date", active));

    let progress_pct = (count as f64 / bounty.kill_required as f64 * 100.0).min(100.0) as i32;
    let bar_len = 10;
    let filled = (progress_pct as f64 / 100.0 * bar_len as f64) as usize;
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);

    let mut r = format!("{}\n═══ 赏金进度 ═══", prefix);
    r.push_str(&format!("\n\n任务: {}", active));
    r.push_str(&format!("\n{}", bounty.description));
    r.push_str(&format!(
        "\n\n目标: 击杀{} {} / {}",
        bounty.target_monster, count, bounty.kill_required
    ));
    r.push_str(&format!("\n进度: [{}] {}%", bar, progress_pct));
    r.push_str(&format!("\n接取时间: {}", date));

    if count >= bounty.kill_required {
        r.push_str("\n\n✅ 任务完成！发送'提交赏金'领取奖励");
    } else {
        r.push_str(&format!(
            "\n\n还需击杀 {} 只{}",
            bounty.kill_required - count,
            bounty.target_monster
        ));
    }
    r
}

/// 提交赏金任务
pub fn cmd_submit_bounty(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let active = db.read_user_data(user_id, "bounty_active");

    if active.is_empty() {
        return format!("{}\n{}", prefix, crate::template_render::render_no_bounty_tasks(db));
    }

    let bounty = match BOUNTIES.iter().find(|b| b.name == active) {
        Some(b) => b,
        None => return format!("{}\n任务数据异常", prefix),
    };

    let count: i32 = db
        .read_user_data(user_id, &format!("bounty_{}_count", active))
        .parse()
        .unwrap_or(0);

    if count < bounty.kill_required {
        return format!(
            "{}\n任务尚未完成！还需击杀 {} 只{}",
            prefix,
            bounty.kill_required - count,
            bounty.target_monster
        );
    }

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, bounty.reward_gold);
    crate::user::add_experience(db, user_id, bounty.reward_exp);
    if bounty.reward_item_qty > 0 {
        db.knapsack_add(user_id, bounty.reward_item, bounty.reward_item_qty);
    }

    // 清除任务状态
    db.write_user_data(user_id, "bounty_active", "");
    db.write_user_data(user_id, &format!("bounty_{}_count", active), "");
    db.write_user_data(user_id, &format!("bounty_{}_date", active), "");

    // 记录到 log
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR IGNORE INTO Ext_BountyTask_log (taskMode, fromQQ, taskId, TIMESTAMP, ishis) VALUES (?1, ?2, ?3, ?4, 1)",
        rusqlite::params!["completed", user_id, active, Local::now().format("%Y-%m-%d %H:%M:%S").to_string()],
    );
    drop(conn);

    // 更新完成计数
    let done_key = "bounty_total_completed";
    let done: i32 = db.read_user_data(user_id, done_key).parse().unwrap_or(0);
    db.write_user_data(user_id, done_key, &(done + 1).to_string());

    // 构建奖励描述用于模板渲染
    let mut reward_parts = vec![
        format!("{}金币", bounty.reward_gold),
        format!("{}经验", bounty.reward_exp),
    ];
    if bounty.reward_item_qty > 0 {
        reward_parts.push(format!("{}×{}", bounty.reward_item, bounty.reward_item_qty));
    }
    let reward_str = reward_parts.join("、");
    let mut r = format!(
        "{}\n{}",
        prefix,
        crate::template_render::render_bounty_submit_success(db, &reward_str)
    );
    r.push_str(&format!("\n\n任务: {}", active));
    r.push_str(&format!("\n\n累计完成赏金任务: {}次", done + 1));
    r.push_str("\n\n发送'赏金任务'接取新任务");
    r
}

/// 放弃赏金任务
pub fn cmd_abandon_bounty(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let active = db.read_user_data(user_id, "bounty_active");

    if active.is_empty() {
        return format!("{}\n你没有进行中的赏金任务", prefix);
    }

    // 清除任务状态
    db.write_user_data(user_id, "bounty_active", "");
    db.write_user_data(user_id, &format!("bounty_{}_count", active), "");
    db.write_user_data(user_id, &format!("bounty_{}_date", active), "");

    format!("{}\n已放弃赏金任务 [{}]\n发送'赏金任务'接取新任务", prefix, active)
}

/// 战斗中击杀怪物后调用，增加赏金进度
/// 返回 Some(任务完成提示) 如果任务刚好完成
pub fn on_monster_killed(db: &Database, user_id: &str, monster_name: &str) -> Option<String> {
    let active = db.read_user_data(user_id, "bounty_active");
    if active.is_empty() {
        return None;
    }

    let bounty = BOUNTIES.iter().find(|b| b.name == active)?;
    if bounty.target_monster != monster_name {
        return None;
    }

    let key = format!("bounty_{}_count", active);
    let count: i32 = db.read_user_data(user_id, &key).parse().unwrap_or(0);
    let new_count = count + 1;
    db.write_user_data(user_id, &key, &new_count.to_string());

    if new_count >= bounty.kill_required {
        Some(format!("\n📋 赏金任务 [{}] 完成！发送'提交赏金'领取奖励", active))
    } else {
        Some(format!(
            "\n📋 赏金进度: {} {} / {}",
            bounty.target_monster, new_count, bounty.kill_required
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounties_not_empty() {
        assert!(!BOUNTIES.is_empty(), "BOUNTIES should not be empty");
        assert_eq!(BOUNTIES.len(), 7, "Should have 7 predefined bounties");
    }

    #[test]
    fn test_bounty_definitions_valid() {
        for b in BOUNTIES {
            assert!(!b.name.is_empty(), "Bounty name must not be empty");
            assert!(!b.target_monster.is_empty(), "Target monster must not be empty");
            assert!(b.kill_required > 0, "Kill required must be positive");
            assert!(b.reward_gold > 0, "Reward gold must be positive");
            assert!(b.reward_exp > 0, "Reward exp must be positive");
            assert!(b.min_level >= 1, "Min level must be at least 1");
        }
    }

    #[test]
    fn test_bounty_order_by_min_level() {
        // Bounties should be roughly ordered by difficulty
        let first = &BOUNTIES[0];
        let last = &BOUNTIES[BOUNTIES.len() - 1];
        assert!(
            first.min_level <= last.min_level,
            "First bounty should be easier than last"
        );
    }

    #[test]
    fn test_bounty_unique_names() {
        let mut names: Vec<&str> = BOUNTIES.iter().map(|b| b.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "All bounty names must be unique");
    }

    #[test]
    fn test_bounty_reward_scaling() {
        // Higher level bounties should generally give more rewards
        let level_1: Vec<_> = BOUNTIES.iter().filter(|b| b.min_level <= 1).collect();
        let level_10_plus: Vec<_> = BOUNTIES.iter().filter(|b| b.min_level >= 10).collect();

        if !level_1.is_empty() && !level_10_plus.is_empty() {
            let avg_low: i64 = level_1.iter().map(|b| b.reward_gold).sum::<i64>() / level_1.len() as i64;
            let avg_high: i64 = level_10_plus.iter().map(|b| b.reward_gold).sum::<i64>() / level_10_plus.len() as i64;
            assert!(
                avg_high > avg_low,
                "Higher level bounties should reward more gold on average"
            );
        }
    }
}

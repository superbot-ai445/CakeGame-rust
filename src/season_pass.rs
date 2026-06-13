/// CakeGame 赛季通行证系统
/// 30级阶梯式赛季奖励系统，通过日常活跃获取通行证经验
///
/// 功能:
/// - 查看通行证: 当前等级、经验进度、已领取/未领取奖励
/// - 领取通行证奖励: 领取对应等级的免费/高级奖励
/// - 购买高级通行证: 钻石解锁高级奖励通道
/// - 通行证排行: 全服通行证等级排名
///
/// 存储: Global表 SECTION='season_pass'
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;
use std::collections::HashMap;

const SECTION: &str = "season_pass";

/// 通行证等级定义
struct PassTier {
    level: u32,
    exp_required: u64,
    free_reward: &'static str,
    free_amount: u64,
    premium_reward: &'static str,
    premium_amount: u64,
}

/// 30级通行证奖励阶梯
const PASS_TIERS: [PassTier; 30] = [
    PassTier {
        level: 1,
        exp_required: 100,
        free_reward: "金币",
        free_amount: 500,
        premium_reward: "钻石",
        premium_amount: 10,
    },
    PassTier {
        level: 2,
        exp_required: 200,
        free_reward: "金币",
        free_amount: 800,
        premium_reward: "强化石",
        premium_amount: 2,
    },
    PassTier {
        level: 3,
        exp_required: 300,
        free_reward: "生命药水",
        free_amount: 5,
        premium_reward: "钻石",
        premium_amount: 15,
    },
    PassTier {
        level: 4,
        exp_required: 400,
        free_reward: "金币",
        free_amount: 1000,
        premium_reward: "复活卷轴",
        premium_amount: 1,
    },
    PassTier {
        level: 5,
        exp_required: 500,
        free_reward: "钻石",
        free_amount: 5,
        premium_reward: "超生命药水",
        premium_amount: 3,
    },
    PassTier {
        level: 6,
        exp_required: 600,
        free_reward: "金币",
        free_amount: 1200,
        premium_reward: "强化石",
        premium_amount: 3,
    },
    PassTier {
        level: 7,
        exp_required: 700,
        free_reward: "魔力药水",
        free_amount: 5,
        premium_reward: "钻石",
        premium_amount: 20,
    },
    PassTier {
        level: 8,
        exp_required: 800,
        free_reward: "金币",
        free_amount: 1500,
        premium_reward: "高级强化石",
        premium_amount: 1,
    },
    PassTier {
        level: 9,
        exp_required: 900,
        free_reward: "钻石",
        free_amount: 8,
        premium_reward: "复活卷轴",
        premium_amount: 2,
    },
    PassTier {
        level: 10,
        exp_required: 1000,
        free_reward: "强化石",
        free_amount: 2,
        premium_reward: "钻石",
        premium_amount: 30,
    },
    PassTier {
        level: 11,
        exp_required: 1100,
        free_reward: "金币",
        free_amount: 2000,
        premium_reward: "强化石",
        premium_amount: 5,
    },
    PassTier {
        level: 12,
        exp_required: 1200,
        free_reward: "生命药水",
        free_amount: 10,
        premium_reward: "超生命药水",
        premium_amount: 5,
    },
    PassTier {
        level: 13,
        exp_required: 1300,
        free_reward: "金币",
        free_amount: 2500,
        premium_reward: "钻石",
        premium_amount: 25,
    },
    PassTier {
        level: 14,
        exp_required: 1400,
        free_reward: "钻石",
        free_amount: 10,
        premium_reward: "高级强化石",
        premium_amount: 2,
    },
    PassTier {
        level: 15,
        exp_required: 1500,
        free_reward: "强化石",
        free_amount: 3,
        premium_reward: "钻石",
        premium_amount: 50,
    },
    PassTier {
        level: 16,
        exp_required: 1600,
        free_reward: "金币",
        free_amount: 3000,
        premium_reward: "复活卷轴",
        premium_amount: 3,
    },
    PassTier {
        level: 17,
        exp_required: 1700,
        free_reward: "魔力药水",
        free_amount: 10,
        premium_reward: "强化石",
        premium_amount: 5,
    },
    PassTier {
        level: 18,
        exp_required: 1800,
        free_reward: "金币",
        free_amount: 3500,
        premium_reward: "超魔力药水",
        premium_amount: 5,
    },
    PassTier {
        level: 19,
        exp_required: 1900,
        free_reward: "钻石",
        free_amount: 15,
        premium_reward: "钻石",
        premium_amount: 40,
    },
    PassTier {
        level: 20,
        exp_required: 2000,
        free_reward: "强化石",
        free_amount: 5,
        premium_reward: "高级强化石",
        premium_amount: 3,
    },
    PassTier {
        level: 21,
        exp_required: 2100,
        free_reward: "金币",
        free_amount: 4000,
        premium_reward: "钻石",
        premium_amount: 50,
    },
    PassTier {
        level: 22,
        exp_required: 2200,
        free_reward: "生命药水",
        free_amount: 15,
        premium_reward: "强化石",
        premium_amount: 8,
    },
    PassTier {
        level: 23,
        exp_required: 2300,
        free_reward: "金币",
        free_amount: 4500,
        premium_reward: "复活卷轴",
        premium_amount: 5,
    },
    PassTier {
        level: 24,
        exp_required: 2400,
        free_reward: "钻石",
        free_amount: 20,
        premium_reward: "超生命药水",
        premium_amount: 10,
    },
    PassTier {
        level: 25,
        exp_required: 2500,
        free_reward: "强化石",
        free_amount: 5,
        premium_reward: "钻石",
        premium_amount: 80,
    },
    PassTier {
        level: 26,
        exp_required: 2600,
        free_reward: "金币",
        free_amount: 5000,
        premium_reward: "高级强化石",
        premium_amount: 5,
    },
    PassTier {
        level: 27,
        exp_required: 2700,
        free_reward: "钻石",
        free_amount: 25,
        premium_reward: "强化石",
        premium_amount: 10,
    },
    PassTier {
        level: 28,
        exp_required: 2800,
        free_reward: "金币",
        free_amount: 6000,
        premium_reward: "复活卷轴",
        premium_amount: 5,
    },
    PassTier {
        level: 29,
        exp_required: 2900,
        free_reward: "钻石",
        free_amount: 30,
        premium_reward: "钻石",
        premium_amount: 100,
    },
    PassTier {
        level: 30,
        exp_required: 3000,
        free_reward: "强化石",
        free_amount: 10,
        premium_reward: "高级强化石",
        premium_amount: 10,
    },
];

/// 高级通行证购买价格（钻石）
const PREMIUM_PASS_COST: i64 = 300;

/// 每日通行证经验获取上限
const DAILY_EXP_CAP: u64 = 500;

/// 各活动类型的通行证经验
#[allow(dead_code)]
fn exp_for_activity(activity: &str) -> u64 {
    match activity {
        "签到" => 50,
        "攻击" => 10,
        "挑战副本" => 80,
        "挑战地宫" => 60,
        "完成任务" => 40,
        "合成物品" => 20,
        "采集" => 15,
        "祈福" => 30,
        "修炼" => 25,
        "竞技匹配" => 50,
        "挑战BOSS" => 100,
        "公会捐献" => 40,
        "烹饪" => 20,
        "炼制" => 20,
        "熔炼" => 20,
        "种植" => 15,
        _ => 10,
    }
}

/// 辅助: 读取 Global 值并解析
fn get_u32(db: &Database, id: &str) -> u32 {
    db.global_get(SECTION, id).parse::<u32>().unwrap_or(0)
}
fn get_u64(db: &Database, id: &str) -> u64 {
    db.global_get(SECTION, id).parse::<u64>().unwrap_or(0)
}
fn get_str(db: &Database, id: &str) -> String {
    let v = db.global_get(SECTION, id);
    if v.is_empty() {
        String::new()
    } else {
        v
    }
}
fn set_val(db: &Database, id: &str, val: &str) {
    db.global_set(SECTION, id, val);
}

/// 读取玩家通行证数据
fn read_pass_data(db: &Database, uid: &str) -> (u32, u64, bool, String, u64) {
    let level = get_u32(db, &format!("{}.level", uid));
    let exp = get_u64(db, &format!("{}.exp", uid));
    let premium = get_str(db, &format!("{}.premium", uid)) == "1";
    let claimed_free = get_str(db, &format!("{}.claimed_free", uid));
    let today_raw = get_str(db, &format!("{}.today_exp", uid));
    let today_exp = if let Some((date, val)) = today_raw.split_once('|') {
        if date == Local::now().format("%Y-%m-%d").to_string() {
            val.parse::<u64>().unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };
    (level, exp, premium, claimed_free, today_exp)
}

/// 写入通行证数据
fn write_pass_data(db: &Database, uid: &str, level: u32, exp: u64, premium: bool, claimed_free: &str, today_exp: u64) {
    set_val(db, &format!("{}.level", uid), &level.to_string());
    set_val(db, &format!("{}.exp", uid), &exp.to_string());
    set_val(db, &format!("{}.premium", uid), if premium { "1" } else { "0" });
    set_val(db, &format!("{}.claimed_free", uid), claimed_free);
    let today = Local::now().format("%Y-%m-%d").to_string();
    set_val(db, &format!("{}.today_exp", uid), &format!("{}|{}", today, today_exp));
}

/// 进度条可视化
fn progress_bar(current: u64, max: u64, width: usize) -> String {
    if max == 0 {
        return "░".repeat(width);
    }
    let filled = ((current as f64 / max as f64) * width as f64) as usize;
    let filled = filled.min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// 检查奖励是否已领取
fn is_claimed(claimed_str: &str, level: u32) -> bool {
    claimed_str.split(',').any(|s| s.parse::<u32>().ok() == Some(level))
}

/// 添加已领取标记
fn mark_claimed(claimed_str: &str, level: u32) -> String {
    if claimed_str.is_empty() {
        level.to_string()
    } else {
        format!("{},{}", claimed_str, level)
    }
}

/// 奖励图标
fn reward_icon(reward: &str) -> &'static str {
    match reward {
        "金币" => "💰",
        "钻石" => "💎",
        "强化石" => "🔨",
        "高级强化石" => "🔨",
        "生命药水" => "🧪",
        "魔力药水" => "🧪",
        "超生命药水" => "💊",
        "超魔力药水" => "💊",
        "复活卷轴" => "📜",
        _ => "🎁",
    }
}

/// 查看通行证
pub fn cmd_view_season_pass(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (level, exp, premium, claimed_free, today_exp) = read_pass_data(db, user_id);

    let pass_type = if premium {
        "🌟 高级通行证"
    } else {
        "📋 普通通行证"
    };

    // 当前等级进度
    let current_tier = PASS_TIERS.iter().find(|t| t.level == level + 1);
    let progress_str = if let Some(tier) = current_tier {
        let bar = progress_bar(exp, tier.exp_required, 10);
        format!("{}/{} {}", exp, tier.exp_required, bar)
    } else {
        "🏆 已满级".to_string()
    };

    // 已领取统计
    let claimed_count = if claimed_free.is_empty() {
        0
    } else {
        claimed_free.split(',').filter(|s| !s.is_empty()).count()
    };

    // 今日经验
    let daily_cap_pct = format!("{}/{}", today_exp.min(DAILY_EXP_CAP), DAILY_EXP_CAP);

    // 下一级奖励预览
    let next_reward_str = if let Some(tier) = current_tier {
        let free = format!(
            "{} {}×{}",
            reward_icon(tier.free_reward),
            tier.free_reward,
            tier.free_amount
        );
        let prem = if premium {
            format!(
                " | 🌟 {} {}×{}",
                reward_icon(tier.premium_reward),
                tier.premium_reward,
                tier.premium_amount
            )
        } else {
            " | 🔒 高级奖励".to_string()
        };
        format!("Lv.{}: {}{}", tier.level, free, prem)
    } else {
        "🏆 所有奖励已解锁".to_string()
    };

    // 最近可领取奖励
    let mut claimable = Vec::new();
    for tier in PASS_TIERS.iter() {
        if tier.level <= level && !is_claimed(&claimed_free, tier.level) {
            claimable.push(tier.level);
        }
        if claimable.len() >= 5 {
            break;
        }
    }
    let claimable_str = if claimable.is_empty() {
        "✅ 所有可领取奖励已领取".to_string()
    } else {
        format!(
            "🎁 可领取: Lv.{}",
            claimable
                .iter()
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
                .join(", Lv.")
        )
    };

    format!(
        "{prefix}\n\
         ═══ 赛季通行证 ═══\n\
         {pass_type} | Lv.{level}/30\n\
         📊 经验进度: {progress_str}\n\
         📅 今日经验: {daily_cap_pct}\n\
         📦 已领取: {claimed_count}/30 级\n\
         \n\
         ⏭️ 下一级奖励:\n\
         {next_reward_str}\n\
         \n\
         {claimable_str}\n\
         \n\
         💡 指令: 领取通行证奖励 | 购买高级通行证 | 通行证排行"
    )
}

/// 增加通行证经验（供其他系统调用）
#[allow(dead_code)]
pub fn add_pass_exp(db: &Database, user_id: &str, activity: &str) -> String {
    let (mut level, mut exp, premium, claimed_free, mut today_exp) = read_pass_data(db, user_id);

    if level >= 30 {
        return String::new();
    }

    let base_exp = exp_for_activity(activity);
    if today_exp >= DAILY_EXP_CAP {
        return String::new();
    }
    let actual_exp = base_exp.min(DAILY_EXP_CAP - today_exp);
    today_exp += actual_exp;
    exp += actual_exp;

    let mut leveled_up = false;
    while level < 30 {
        if let Some(tier) = PASS_TIERS.iter().find(|t| t.level == level + 1) {
            if exp >= tier.exp_required {
                exp -= tier.exp_required;
                level += 1;
                leveled_up = true;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    write_pass_data(db, user_id, level, exp, premium, &claimed_free, today_exp);

    if leveled_up {
        format!("🎉 通行证升级！→ Lv.{}", level)
    } else {
        String::new()
    }
}

/// 领取通行证奖励
pub fn cmd_claim_pass_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (level, exp, premium, claimed_free, today_exp) = read_pass_data(db, user_id);

    let target_level = if args.is_empty() {
        0u32
    } else {
        match args.trim().parse::<u32>() {
            Ok(l) => l,
            Err(_) => {
                return format!("{prefix}\n❌ 请输入有效的等级数字，例如: 领取通行证奖励 5");
            }
        }
    };

    let mut total_rewards: HashMap<String, u64> = HashMap::new();
    let mut claimed_levels = Vec::new();
    let mut new_claimed = claimed_free.clone();

    for tier in PASS_TIERS.iter() {
        if tier.level > level {
            break;
        }
        if target_level != 0 && tier.level != target_level {
            continue;
        }
        if is_claimed(&new_claimed, tier.level) {
            continue;
        }

        *total_rewards.entry(tier.free_reward.to_string()).or_insert(0) += tier.free_amount;
        claimed_levels.push(tier.level);

        if premium {
            *total_rewards.entry(tier.premium_reward.to_string()).or_insert(0) += tier.premium_amount;
        }

        new_claimed = mark_claimed(&new_claimed, tier.level);
    }

    if claimed_levels.is_empty() {
        if target_level != 0 {
            return format!("{prefix}\n❌ Lv.{target_level} 无法领取（等级不够或已领取）");
        } else {
            return format!("{prefix}\n✅ 所有可领取的奖励已领取完毕！");
        }
    }

    // 发放奖励
    for (item, amount) in &total_rewards {
        match item.as_str() {
            "金币" => {
                db.modify_currency(user_id, CURRENCY_GOLD, "add", *amount as i64);
            }
            "钻石" => {
                db.modify_currency(user_id, CURRENCY_DIAMOND, "add", *amount as i64);
            }
            _ => {
                db.knapsack_add(user_id, item, *amount as i32);
            }
        }
    }

    write_pass_data(db, user_id, level, exp, premium, &new_claimed, today_exp);

    let reward_list: Vec<String> = total_rewards
        .iter()
        .map(|(item, amount)| format!("  {} {}×{}", reward_icon(item), item, amount))
        .collect();

    let levels_str = if claimed_levels.len() == 1 {
        format!("Lv.{}", claimed_levels[0])
    } else {
        format!(
            "Lv.{}~Lv.{}",
            claimed_levels.iter().min().unwrap(),
            claimed_levels.iter().max().unwrap()
        )
    };

    let premium_note = if !premium {
        "\n💡 购买高级通行证可额外领取高级奖励！"
    } else {
        ""
    };

    format!(
        "{prefix}\n\
         🎁 通行证奖励领取成功！\n\
         📦 领取等级: {levels_str} (共{}级)\n\
         \n\
         已获得:\n\
         {}\n\
         {premium_note}",
        claimed_levels.len(),
        reward_list.join("\n"),
    )
}

/// 购买高级通行证
pub fn cmd_buy_premium_pass(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (level, exp, premium, claimed_free, today_exp) = read_pass_data(db, user_id);

    if premium {
        return format!("{prefix}\n✅ 你已经拥有高级通行证了！\n💡 可使用「领取通行证奖励」领取双倍奖励。");
    }

    let diamonds = db.read_currency(user_id, CURRENCY_DIAMOND);
    if diamonds < PREMIUM_PASS_COST {
        return format!(
            "{prefix}\n💎 钻石不足！\n\
             高级通行证需要 {PREMIUM_PASS_COST} 钻石\n\
             当前钻石: {diamonds}\n\
             💡 可通过签到、任务、充值获取钻石"
        );
    }

    db.modify_currency(user_id, CURRENCY_DIAMOND, "add", -PREMIUM_PASS_COST);
    write_pass_data(db, user_id, level, exp, true, &claimed_free, today_exp);

    format!(
        "{prefix}\n\
         🌟 高级通行证购买成功！\n\
         💎 消耗: {PREMIUM_PASS_COST} 钻石\n\
         \n\
         ✨ 解锁权益:\n\
         · 每级额外领取高级奖励\n\
         · 已解锁等级的高级奖励可在领取时一并获取\n\
         · 30级全部奖励等你来拿！\n\
         \n\
         💡 使用「领取通行证奖励」领取你的高级奖励"
    )
}

/// 通行证排行
pub fn cmd_pass_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 使用 query_rows 收集所有通行证等级数据
    let rows = db.query_rows(
        "SELECT ID, DATA FROM Global WHERE SECTION = ?1 AND ID LIKE '%.level'",
        &[SECTION],
        |row| {
            let id: String = row.get(0).unwrap_or_default();
            let data: String = row.get(1).unwrap_or_default();
            Ok((id, data))
        },
    );

    let mut players: Vec<(String, u32, u64, bool)> = Vec::new();
    let mut my_level = 0u32;
    let _my_exp = get_u64(db, &format!("{}.exp", user_id));
    let _my_premium = get_str(db, &format!("{}.premium", user_id)) == "1";

    for (id, data) in &rows {
        // ID format: "uid.level"
        if let Some(uid) = id.strip_suffix(".level") {
            let level = data.parse::<u32>().unwrap_or(0);
            if level > 0 {
                let exp = get_u64(db, &format!("{}.exp", uid));
                let premium = get_str(db, &format!("{}.premium", uid)) == "1";
                players.push((uid.to_string(), level, exp, premium));
                if uid == user_id {
                    my_level = level;
                }
            }
        }
    }

    players.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));

    let mut result = format!(
        "{prefix}\n\
         ═══ 通行证排行榜 ═══\n\
         🏆 全服通行证等级排名\n\
         \n"
    );

    let medals = ["🥇", "🥈", "🥉"];
    let mut my_rank = 0usize;

    for (i, (uid, level, exp, premium)) in players.iter().take(10).enumerate() {
        let medal = if i < 3 { medals[i] } else { &format!("{:>2}.", i + 1) };
        let pass_icon = if *premium { "🌟" } else { "📋" };
        let next_exp = PASS_TIERS
            .iter()
            .find(|t| t.level == level + 1)
            .map(|t| t.exp_required)
            .unwrap_or(0);
        let exp_str = if *level < 30 {
            format!("({}/{})", exp, next_exp)
        } else {
            "(🏆满级)".to_string()
        };

        let display_name = user::get_msg_prefix(db, uid);

        result.push_str(&format!(
            "{} {} Lv.{} {} {} {}\n",
            medal,
            display_name,
            level,
            pass_icon,
            exp_str,
            if uid == user_id { "← 你" } else { "" }
        ));

        if uid == user_id {
            my_rank = i + 1;
        }
    }

    if my_rank == 0 && my_level > 0 {
        for (i, (uid, _, _, _)) in players.iter().enumerate() {
            if uid == user_id {
                my_rank = i + 1;
                break;
            }
        }
        if my_rank > 0 {
            result.push_str(&format!("\n📍 你的排名: 第{}名 (Lv.{})", my_rank, my_level));
        }
    } else if my_level == 0 {
        result.push_str("\n📍 你还没有通行证经验，快去冒险吧！");
    }

    if players.is_empty() {
        result.push_str("暂无通行证数据\n");
    }

    result.push_str(
        "\n💡 参与日常活动获取通行证经验:\n\
         · 签到 +50 | 攻击 +10 | 副本 +80\n\
         · 任务 +40 | 采集 +15 | BOSS +100",
    );

    result
}

/// 查看通行证帮助
pub fn cmd_pass_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    "📋 赛季通行证系统帮助\n\
     \n\
     ═══ 什么是通行证？ ═══\n\
     赛季通行证是30级阶梯式奖励系统。\n\
     通过日常游戏活动获取通行证经验，\n\
     升级后可领取丰厚奖励！\n\
     \n\
     ═══ 如何获取经验？ ═══\n\
     · 每日签到: +50 EXP\n\
     · 攻击怪物: +10 EXP\n\
     · 挑战副本: +80 EXP\n\
     · 完成任务: +40 EXP\n\
     · 挑战BOSS: +100 EXP\n\
     · 采集资源: +15 EXP\n\
     · 竞技匹配: +50 EXP\n\
     · 公会捐献: +40 EXP\n\
     · 每日祈福: +30 EXP\n\
     · 修炼: +25 EXP\n\
     \n\
     ⏰ 每日经验上限: 500 EXP\n\
     \n\
     ═══ 通行证类型 ═══\n\
     📋 普通通行证: 免费，可领取免费奖励\n\
     🌟 高级通行证: 300钻石，额外领取高级奖励\n\
     \n\
     ═══ 指令列表 ═══\n\
     · 查看通行证 - 查看当前进度和奖励\n\
     · 领取通行证奖励 - 领取所有可领取的奖励\n\
     · 领取通行证奖励+等级 - 领取指定等级奖励\n\
     · 购买高级通行证 - 解锁高级奖励通道\n\
     · 通行证排行 - 全服通行证等级排名\n"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_tiers_count() {
        assert_eq!(PASS_TIERS.len(), 30);
    }

    #[test]
    fn test_pass_tiers_level_unique() {
        let mut levels: Vec<u32> = PASS_TIERS.iter().map(|t| t.level).collect();
        levels.sort();
        levels.dedup();
        assert_eq!(levels.len(), 30);
    }

    #[test]
    fn test_pass_tiers_exp_positive() {
        for tier in &PASS_TIERS {
            assert!(tier.exp_required > 0);
            assert!(tier.free_amount > 0);
            assert!(tier.premium_amount > 0);
        }
    }

    #[test]
    fn test_exp_increasing() {
        for i in 1..PASS_TIERS.len() {
            assert!(PASS_TIERS[i].exp_required >= PASS_TIERS[i - 1].exp_required);
        }
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(50, 100, 10);
        assert_eq!(bar.chars().count(), 10);
        assert_eq!(bar, "█████░░░░░");

        let bar_full = progress_bar(100, 100, 10);
        assert_eq!(bar_full.chars().count(), 10);

        let bar_empty = progress_bar(0, 100, 10);
        assert_eq!(bar_empty.chars().count(), 10);
    }

    #[test]
    fn test_claimed_tracking() {
        let mut claimed = String::new();
        assert!(!is_claimed(&claimed, 1));

        claimed = mark_claimed(&claimed, 1);
        assert!(is_claimed(&claimed, 1));
        assert!(!is_claimed(&claimed, 2));

        claimed = mark_claimed(&claimed, 5);
        assert!(is_claimed(&claimed, 1));
        assert!(is_claimed(&claimed, 5));
        assert!(!is_claimed(&claimed, 3));
    }

    #[test]
    fn test_exp_for_activity() {
        assert_eq!(exp_for_activity("签到"), 50);
        assert_eq!(exp_for_activity("挑战BOSS"), 100);
        assert_eq!(exp_for_activity("攻击"), 10);
        assert_eq!(exp_for_activity("未知活动"), 10);
    }

    #[test]
    fn test_reward_icon_coverage() {
        assert_eq!(reward_icon("金币"), "💰");
        assert_eq!(reward_icon("钻石"), "💎");
        assert_eq!(reward_icon("强化石"), "🔨");
        assert_eq!(reward_icon("生命药水"), "🧪");
        assert_eq!(reward_icon("未知物品"), "🎁");
    }

    #[test]
    fn test_total_exp_formula() {
        let total: u64 = PASS_TIERS.iter().map(|t| t.exp_required).sum();
        assert_eq!(total, (1..=30u64).map(|i| i * 100).sum::<u64>());
    }
}

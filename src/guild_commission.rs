/// CakeGame 公会每日委托系统
///
/// 公会级别的每日合作任务系统。每天刷新 5 个随机委托，
/// 公会成员共同完成，获取公会贡献值和个人奖励。
/// 全部完成触发公会繁荣奖励。
///
/// 指令: 公会委托, 接受委托, 提交委托, 委托进度, 委托排行
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::{Datelike, Local};

/// 委托类型定义
struct CommissionDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    category: &'static str,
    target_count: i32,
    reward_gold: i64,
    reward_diamond: i32,
    reward_contribution: i32,
    reward_item: &'static str,
    difficulty: &'static str,
}

const COMMISSION_DEFS: &[CommissionDef] = &[
    CommissionDef {
        id: "kill_monster",
        name: "讨伐魔物",
        desc: "击败 20 只任意怪物",
        category: "战斗",
        target_count: 20,
        reward_gold: 800,
        reward_diamond: 5,
        reward_contribution: 30,
        reward_item: "【普通】生命药水*3",
        difficulty: "⭐",
    },
    CommissionDef {
        id: "kill_boss",
        name: "首领猎人",
        desc: "击败 3 只 BOSS",
        category: "战斗",
        target_count: 3,
        reward_gold: 2000,
        reward_diamond: 15,
        reward_contribution: 80,
        reward_item: "强化石*2",
        difficulty: "⭐⭐⭐",
    },
    CommissionDef {
        id: "gather_herb",
        name: "药材采集",
        desc: "采集 10 株药材",
        category: "生活",
        target_count: 10,
        reward_gold: 500,
        reward_diamond: 3,
        reward_contribution: 20,
        reward_item: "白色精粹*2",
        difficulty: "⭐",
    },
    CommissionDef {
        id: "gather_ore",
        name: "矿石开采",
        desc: "开采 10 块矿石",
        category: "生活",
        target_count: 10,
        reward_gold: 500,
        reward_diamond: 3,
        reward_contribution: 20,
        reward_item: "蓝色精粹*1",
        difficulty: "⭐",
    },
    CommissionDef {
        id: "sign_count",
        name: "签到打卡",
        desc: "公会成员累计签到 10 次",
        category: "日常",
        target_count: 10,
        reward_gold: 300,
        reward_diamond: 2,
        reward_contribution: 15,
        reward_item: "",
        difficulty: "⭐",
    },
    CommissionDef {
        id: "pvp_win",
        name: "竞技胜利",
        desc: "公会成员累计 PVP 胜利 5 次",
        category: "PVP",
        target_count: 5,
        reward_gold: 1200,
        reward_diamond: 10,
        reward_contribution: 50,
        reward_item: "复活卷轴*1",
        difficulty: "⭐⭐",
    },
    CommissionDef {
        id: "enhance_equip",
        name: "锻造大师",
        desc: "公会成员累计强化装备 10 次",
        category: "装备",
        target_count: 10,
        reward_gold: 1000,
        reward_diamond: 8,
        reward_contribution: 40,
        reward_item: "强化石*3",
        difficulty: "⭐⭐",
    },
    CommissionDef {
        id: "use_gold",
        name: "经济贡献",
        desc: "公会成员累计消费 10000 金币",
        category: "经济",
        target_count: 10000,
        reward_gold: 1500,
        reward_diamond: 5,
        reward_contribution: 35,
        reward_item: "【精良】勇者宝箱*1",
        difficulty: "⭐⭐",
    },
    CommissionDef {
        id: "collect_herb_garden",
        name: "药园丰收",
        desc: "公会成员累计收获药材 5 次",
        category: "生活",
        target_count: 5,
        reward_gold: 600,
        reward_diamond: 3,
        reward_contribution: 25,
        reward_item: "种子*3",
        difficulty: "⭐",
    },
    CommissionDef {
        id: "dungeon_clear",
        name: "副本挑战",
        desc: "公会成员累计通关副本 3 次",
        category: "战斗",
        target_count: 3,
        reward_gold: 1800,
        reward_diamond: 12,
        reward_contribution: 60,
        reward_item: "【精良】勇者宝箱*1",
        difficulty: "⭐⭐⭐",
    },
    CommissionDef {
        id: "composite_craft",
        name: "合成达人",
        desc: "公会成员累计合成物品 5 次",
        category: "生活",
        target_count: 5,
        reward_gold: 800,
        reward_diamond: 5,
        reward_contribution: 30,
        reward_item: "白色精粹*3",
        difficulty: "⭐",
    },
    CommissionDef {
        id: "skill_use",
        name: "技能修行",
        desc: "公会成员累计释放技能 30 次",
        category: "战斗",
        target_count: 30,
        reward_gold: 1000,
        reward_diamond: 8,
        reward_contribution: 40,
        reward_item: "【普通】魔法药水*3",
        difficulty: "⭐⭐",
    },
];

/// Global 表 section 名
const SECTION: &str = "guild_commission";

/// 获取今日日期字符串
fn today_string() -> String {
    let now = Local::now();
    format!("{:04}{:02}{:02}", now.year(), now.month(), now.day())
}

/// 简单哈希 (djb2 变体)
fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u32);
    }
    hash
}

/// 根据日期和公会名确定性选择今日 5 个委托
fn get_today_commissions(guild_name: &str) -> Vec<&'static CommissionDef> {
    let seed = format!("{}_{}", guild_name, today_string());
    let hash = simple_hash(&seed);
    let total = COMMISSION_DEFS.len();
    let mut selected = Vec::new();
    let mut used = std::collections::HashSet::new();

    for i in 0..5u32 {
        let idx = ((hash.wrapping_add(i.wrapping_mul(7919))) as usize) % total;
        let mut attempt = idx;
        while used.contains(&attempt) {
            attempt = (attempt + 1) % total;
        }
        used.insert(attempt);
        selected.push(&COMMISSION_DEFS[attempt]);
    }
    selected
}

/// 格式化委托进度条
fn format_progress_bar(current: i32, target: i32) -> String {
    let ratio = (current as f64 / target as f64).min(1.0);
    let filled = (ratio * 10.0).round() as usize;
    let empty = 10 - filled;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    let pct = (ratio * 100.0).round() as i32;
    format!("{} {}/{} ({}%)", bar, current, target, pct)
}

/// 获取公会名
fn get_guild_name(db: &Database, user_id: &str) -> Option<String> {
    let name = db.read_user_data(user_id, "union_name");
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// 查看公会委托列表
pub fn cmd_view_commissions(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild_name = match get_guild_name(db, user_id) {
        Some(n) => n,
        None => return format!("{}\n您还没有加入公会，请先加入公会后再查看委托。", prefix),
    };

    let commissions = get_today_commissions(&guild_name);
    let today = today_string();

    let mut result = format!(
        "{}\n═══ 公会每日委托 ═══\n📅 日期: {}\n🏰 公会: {}\n",
        prefix, today, guild_name
    );

    let mut total_done = 0i32;

    for (i, comm) in commissions.iter().enumerate() {
        let current: i32 = db
            .global_get(SECTION, &format!("{}_{}", guild_name, comm.id))
            .parse()
            .unwrap_or(0);
        let done = current >= comm.target_count;
        if done {
            total_done += 1;
        }

        let status = if done { "✅" } else { "⬜" };
        result.push_str(&format!(
            "\n{} {}. [{}] {} {}",
            status,
            i + 1,
            comm.category,
            comm.name,
            comm.difficulty
        ));
        result.push_str(&format!("\n   {}", comm.desc));
        result.push_str(&format!(
            "\n   进度: {}",
            format_progress_bar(current, comm.target_count)
        ));
        if !done {
            result.push_str(&format!(
                "\n   奖励: {}金 {}钻 贡献+{}",
                comm.reward_gold, comm.reward_diamond, comm.reward_contribution
            ));
            if !comm.reward_item.is_empty() {
                result.push_str(&format!(" {}", comm.reward_item));
            }
        }
        result.push('\n');
    }

    result.push_str(&format!("\n📊 完成进度: {}/{}", total_done, 5));

    if total_done == 5 {
        result.push_str("\n\n🎉 所有委托已完成！公会获得繁荣奖励！");
        result.push_str("\n   5000公会资金 + 2000经验 + 全员50钻");
    }

    result.push_str("\n\n💡 发送 '接受委托+编号' 领取个人任务");
    result.push_str("\n💡 发送 '委托进度' 查看详细进度");
    result.push_str("\n💡 发送 '委托排行' 查看公会成员贡献排名");
    result
}

/// 接受委托 — 玩家个人接取某个委托任务
pub fn cmd_accept_commission(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild_name = match get_guild_name(db, user_id) {
        Some(n) => n,
        None => return format!("{}\n您还没有加入公会。", prefix),
    };

    let args = args.trim();
    if args.is_empty() {
        return format!("{}\n请指定委托编号。\n用法: 接受委托+编号（1~5）", prefix);
    }

    let idx: usize = match args.parse::<usize>() {
        Ok(n) if (1..=5).contains(&n) => n - 1,
        _ => return format!("{}\n无效编号，请输入 1~5。", prefix),
    };

    let commissions = get_today_commissions(&guild_name);
    if idx >= commissions.len() {
        return format!("{}\n委托不存在。", prefix);
    }

    let comm = &commissions[idx];

    // 检查全局进度是否已完成
    let current: i32 = db
        .global_get(SECTION, &format!("{}_{}", guild_name, comm.id))
        .parse()
        .unwrap_or(0);
    if current >= comm.target_count {
        return format!("{}\n委托 [{}] 已经完成了！选择其他委托吧。", prefix, comm.name);
    }

    // 检查玩家是否已接受此委托
    let accepted_key = format!("comm_acc_{}", today_string());
    let accepted_raw = db.read_user_data(user_id, &accepted_key);
    let accepted_list: Vec<&str> = accepted_raw.split(',').filter(|s| !s.is_empty()).collect();
    if accepted_list.contains(&comm.id) {
        return format!("{}\n您已经接受了委托 [{}]。请完成后再接新的。", prefix, comm.name);
    }

    if accepted_list.len() >= 3 {
        return format!("{}\n您同时持有的委托已达上限(3个)。请先完成已有委托。", prefix);
    }

    let new_accepted = if accepted_raw.is_empty() {
        comm.id.to_string()
    } else {
        format!("{},{}", accepted_raw, comm.id)
    };
    db.write_user_data(user_id, &accepted_key, &new_accepted);

    let item_str = if comm.reward_item.is_empty() {
        String::new()
    } else {
        format!(" {}", comm.reward_item)
    };

    format!(
        "{}\n✅ 已接受委托: [{}]\n📋 {}\n🎯 目标: {}\n💰 奖励: {}金 {}钻 贡献+{}{}\n\n\
         完成后发送 '提交委托+{}' 领取奖励。",
        prefix,
        comm.name,
        comm.desc,
        comm.target_count,
        comm.reward_gold,
        comm.reward_diamond,
        comm.reward_contribution,
        item_str,
        idx + 1
    )
}

/// 提交委托 — 完成后领取奖励
pub fn cmd_submit_commission(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild_name = match get_guild_name(db, user_id) {
        Some(n) => n,
        None => return format!("{}\n您还没有加入公会。", prefix),
    };

    let args = args.trim();
    if args.is_empty() {
        return format!("{}\n请指定委托编号。\n用法: 提交委托+编号（1~5）", prefix);
    }

    let idx: usize = match args.parse::<usize>() {
        Ok(n) if (1..=5).contains(&n) => n - 1,
        _ => return format!("{}\n无效编号，请输入 1~5。", prefix),
    };

    let commissions = get_today_commissions(&guild_name);
    if idx >= commissions.len() {
        return format!("{}\n委托不存在。", prefix);
    }

    let comm = &commissions[idx];

    // 检查是否已接受
    let accepted_key = format!("comm_acc_{}", today_string());
    let accepted_raw = db.read_user_data(user_id, &accepted_key);
    let accepted_list: Vec<&str> = accepted_raw.split(',').filter(|s| !s.is_empty()).collect();
    if !accepted_list.contains(&comm.id) {
        return format!(
            "{}\n您还没有接受委托 [{}]。请先发送 '接受委托+{}'。",
            prefix,
            comm.name,
            idx + 1
        );
    }

    // 检查全局进度
    let current: i32 = db
        .global_get(SECTION, &format!("{}_{}", guild_name, comm.id))
        .parse()
        .unwrap_or(0);
    if current < comm.target_count {
        return format!(
            "{}\n委托 [{}] 尚未完成。\n当前进度: {}/{}\n需要全公会成员共同努力！",
            prefix, comm.name, current, comm.target_count
        );
    }

    // 检查是否已领取
    let claimed_key = format!("comm_claim_{}", today_string());
    let claimed_raw = db.read_user_data(user_id, &claimed_key);
    let claimed_list: Vec<&str> = claimed_raw.split(',').filter(|s| !s.is_empty()).collect();
    if claimed_list.contains(&comm.id) {
        return format!("{}\n您已经领取了委托 [{}] 的奖励。", prefix, comm.name);
    }

    // 发放奖励
    if comm.reward_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, comm.reward_gold);
    }
    if comm.reward_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, comm.reward_diamond as i64);
    }

    // 记录贡献值
    let contrib_key = format!("comm_contrib_{}", today_string());
    let old_contrib: i32 = db.read_user_data(user_id, &contrib_key).parse().unwrap_or(0);
    db.write_user_data(
        user_id,
        &contrib_key,
        &(old_contrib + comm.reward_contribution).to_string(),
    );

    // 记录已领取
    let new_claimed = if claimed_raw.is_empty() {
        comm.id.to_string()
    } else {
        format!("{},{}", claimed_raw, comm.id)
    };
    db.write_user_data(user_id, &claimed_key, &new_claimed);

    // 从已接受列表移除
    let new_accepted: Vec<&str> = accepted_list.into_iter().filter(|&s| s != comm.id).collect();
    db.write_user_data(user_id, &accepted_key, &new_accepted.join(","));

    // 增加公会繁荣度
    let flourish_key = format!("flourish_{}", today_string());
    let old_flourish: i32 = db.global_get(SECTION, &flourish_key).parse().unwrap_or(0);
    db.global_set(
        SECTION,
        &flourish_key,
        &(old_flourish + comm.reward_contribution).to_string(),
    );

    let mut result = format!(
        "{}\n🎉 委托完成！[{}]\n\n📦 获得奖励:\n  💰 金币: +{}\n  💎 钻石: +{}\n  🏅 公会贡献: +{}",
        prefix, comm.name, comm.reward_gold, comm.reward_diamond, comm.reward_contribution
    );

    if !comm.reward_item.is_empty() {
        db.knapsack_add(user_id, comm.reward_item, 1);
        result.push_str(&format!("\n  🎁 物品: {}", comm.reward_item));
    }

    // 检查是否全部完成
    let all_done = commissions.iter().all(|c| {
        let v: i32 = db
            .global_get(SECTION, &format!("{}_{}", guild_name, c.id))
            .parse()
            .unwrap_or(0);
        v >= c.target_count
    });

    if all_done {
        result.push_str("\n\n🎊🎊🎊 恭喜！公会今日所有委托已全部完成！");
        result.push_str("\n🏰 公会获得繁荣奖励: 5000资金 + 2000经验 + 全员50钻");
    }

    result
}

/// 委托进度 — 查看今日公会成员贡献统计
pub fn cmd_commission_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild_name = match get_guild_name(db, user_id) {
        Some(n) => n,
        None => return format!("{}\n您还没有加入公会。", prefix),
    };

    let commissions = get_today_commissions(&guild_name);
    let mut result = format!("{}\n═══ 委托进度详情 ═══\n", prefix);

    for (i, comm) in commissions.iter().enumerate() {
        let current: i32 = db
            .global_get(SECTION, &format!("{}_{}", guild_name, comm.id))
            .parse()
            .unwrap_or(0);
        let done = current >= comm.target_count;

        result.push_str(&format!(
            "\n{} {}. {} [{}]\n   进度: {}",
            if done { "✅" } else { "⬜" },
            i + 1,
            comm.name,
            comm.category,
            format_progress_bar(current, comm.target_count)
        ));
    }

    // 个人贡献
    let contrib_key = format!("comm_contrib_{}", today_string());
    let my_contrib: i32 = db.read_user_data(user_id, &contrib_key).parse().unwrap_or(0);
    result.push_str(&format!("\n\n🏅 您今日贡献: {} 点", my_contrib));

    // 公会繁荣度
    let flourish_key = format!("flourish_{}", today_string());
    let flourish: i32 = db.global_get(SECTION, &flourish_key).parse().unwrap_or(0);
    result.push_str(&format!("\n🏰 公会今日繁荣: {} 点", flourish));

    // 个人接受状态
    let accepted_key = format!("comm_acc_{}", today_string());
    let accepted_raw = db.read_user_data(user_id, &accepted_key);
    if !accepted_raw.is_empty() {
        result.push_str(&format!("\n📋 您当前接受的委托: {}", accepted_raw));
    }

    result
}

/// 委托排行 — 公会成员贡献排名
pub fn cmd_commission_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild_name = match get_guild_name(db, user_id) {
        Some(n) => n,
        None => return format!("{}\n您还没有加入公会。", prefix),
    };

    // 获取公会成员
    let members_raw = db.global_get("Guild", &format!("members_{}", guild_name));
    let members: Vec<&str> = members_raw.split(',').filter(|s| !s.is_empty()).collect();

    if members.is_empty() {
        return format!("{}\n公会没有成员数据。", prefix);
    }

    let today = today_string();
    let mut rankings: Vec<(String, i32)> = Vec::new();

    for member in &members {
        let contrib_key = format!("comm_contrib_{}", today);
        let contrib: i32 = db.read_user_data(member, &contrib_key).parse().unwrap_or(0);
        if contrib > 0 {
            let nickname = db.read_user_data(member, "nickname");
            let display = if nickname.is_empty() {
                member.to_string()
            } else {
                nickname
            };
            rankings.push((display, contrib));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut result = format!("{}\n═══ 委托贡献排行 ═══\n📅 {}\n", prefix, today);

    if rankings.is_empty() {
        result.push_str("\n暂无贡献数据。");
    } else {
        for (i, (name, contrib)) in rankings.iter().enumerate().take(10) {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            result.push_str(&format!("\n{} {}. {} — {} 贡献", medal, i + 1, name, contrib));
        }
    }

    // 个人排名
    let my_contrib_key = format!("comm_contrib_{}", today);
    let my_contrib: i32 = db.read_user_data(user_id, &my_contrib_key).parse().unwrap_or(0);
    if my_contrib > 0 {
        let my_nickname = db.read_user_data(user_id, "nickname");
        let my_rank = rankings.iter().position(|(n, _)| *n == my_nickname || *n == user_id);
        if let Some(rank) = my_rank {
            result.push_str(&format!("\n\n📍 您的排名: 第{}名 ({} 贡献)", rank + 1, my_contrib));
        }
    } else {
        result.push_str("\n\n📍 您今日暂无贡献。接受并完成委托即可上榜！");
    }

    result
}

/// 帮助信息
#[allow(dead_code)]
pub fn commission_help() -> &'static str {
    "═══ 公会委托系统 ═══\n\
     每天刷新 5 个随机委托，公会成员共同努力完成！\n\n\
     📋 公会委托 — 查看今日委托列表\n\
     ✅ 接受委托+编号 — 接取指定委托（最多3个）\n\
     📤 提交委托+编号 — 完成后领取奖励\n\
     📊 委托进度 — 查看详细进度\n\
     🏅 委托排行 — 查看公会成员贡献排名\n\n\
     💡 委托进度由全公会成员共同累积\n\
     💡 全部完成可获得公会繁荣奖励"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commission_defs_count() {
        assert!(COMMISSION_DEFS.len() >= 10, "应至少有10个委托定义");
    }

    #[test]
    fn test_commission_ids_unique() {
        let mut ids: Vec<&str> = COMMISSION_DEFS.iter().map(|c| c.id).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), before, "委托 ID 必须唯一");
    }

    #[test]
    fn test_commission_rewards_positive() {
        for comm in COMMISSION_DEFS {
            assert!(comm.reward_gold > 0, "金币奖励必须为正: {}", comm.name);
            assert!(comm.reward_diamond > 0, "钻石奖励必须为正: {}", comm.name);
            assert!(comm.reward_contribution > 0, "贡献值必须为正: {}", comm.name);
            assert!(comm.target_count > 0, "目标数量必须为正: {}", comm.name);
        }
    }

    #[test]
    fn test_today_string_format() {
        let s = today_string();
        assert_eq!(s.len(), 8, "日期格式应为 YYYYMMDD");
        assert!(s.chars().all(|c| c.is_ascii_digit()), "日期应全为数字");
    }

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash("test_guild_20260610");
        let h2 = simple_hash("test_guild_20260610");
        assert_eq!(h1, h2, "哈希应确定性一致");
    }

    #[test]
    fn test_simple_hash_different_inputs() {
        let h1 = simple_hash("guild_a");
        let h2 = simple_hash("guild_b");
        assert_ne!(h1, h2, "不同输入应产生不同哈希");
    }

    #[test]
    fn test_today_commissions_count() {
        let comms = get_today_commissions("test_guild");
        assert_eq!(comms.len(), 5, "每日应刷新5个委托");
    }

    #[test]
    fn test_today_commissions_no_duplicates() {
        let comms = get_today_commissions("test_guild");
        let mut ids: Vec<&str> = comms.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), comms.len(), "每日委托不应重复");
    }

    #[test]
    fn test_today_commissions_deterministic() {
        let c1 = get_today_commissions("my_guild");
        let c2 = get_today_commissions("my_guild");
        let ids1: Vec<&str> = c1.iter().map(|c| c.id).collect();
        let ids2: Vec<&str> = c2.iter().map(|c| c.id).collect();
        assert_eq!(ids1, ids2, "同公会同日期委托应一致");
    }

    #[test]
    fn test_today_commissions_different_guilds() {
        let c1 = get_today_commissions("guild_alpha");
        let c2 = get_today_commissions("guild_beta");
        assert_eq!(c1.len(), 5);
        assert_eq!(c2.len(), 5);
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = format_progress_bar(10, 10);
        assert!(bar.contains("100%"), "满进度应显示100%");
        assert!(bar.contains("██████████"), "满进度应全满");
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = format_progress_bar(0, 10);
        assert!(bar.contains("0%"), "零进度应显示0%");
        assert!(bar.contains("░░░░░░░░░░"), "零进度应全空");
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = format_progress_bar(5, 10);
        assert!(bar.contains("50%"), "半进度应显示50%");
    }

    #[test]
    fn test_commission_categories() {
        let categories: Vec<&str> = COMMISSION_DEFS.iter().map(|c| c.category).collect();
        assert!(categories.contains(&"战斗"), "应有战斗类委托");
        assert!(categories.contains(&"生活"), "应有生活类委托");
    }

    #[test]
    fn test_commission_difficulty_range() {
        for comm in COMMISSION_DEFS {
            assert!(!comm.difficulty.is_empty(), "难度不能为空: {}", comm.name);
        }
    }
}

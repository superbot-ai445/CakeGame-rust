/// CakeGame 地图探索进度系统 (Map Exploration Progress System)
///
/// 追踪玩家在每张地图的探索进度：击杀怪物、采集资源、与NPC对话
/// 完成探索里程碑获得奖励，全地图探索完成获得传说成就
/// 数据存储: Global 表 SECTION='map_explore'
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, ITEM_LEVEL, OP_ADD};
use crate::db::Database;
use crate::user;

/// 探索里程碑定义
struct ExploreMilestone {
    pct: i32,
    name: &'static str,
    reward_gold: i32,
    reward_diamond: i32,
    reward_exp: i32,
    #[allow(dead_code)]
    emoji: &'static str,
}

const EXPLORE_MILESTONES: &[ExploreMilestone] = &[
    ExploreMilestone {
        pct: 25,
        name: "初探者",
        reward_gold: 300,
        reward_diamond: 5,
        reward_exp: 150,
        emoji: "🔍",
    },
    ExploreMilestone {
        pct: 50,
        name: "探路者",
        reward_gold: 800,
        reward_diamond: 15,
        reward_exp: 400,
        emoji: "🧭",
    },
    ExploreMilestone {
        pct: 75,
        name: "探险家",
        reward_gold: 2000,
        reward_diamond: 30,
        reward_exp: 800,
        emoji: "🗺️",
    },
    ExploreMilestone {
        pct: 100,
        name: "探索大师",
        reward_gold: 5000,
        reward_diamond: 80,
        reward_exp: 2000,
        emoji: "👑",
    },
];

/// 探索任务类型
const EXPLORE_TASKS: &[(&str, &str)] = &[
    ("kill", "击杀怪物"),
    ("collect", "采集资源"),
    ("npc", "对话NPC"),
    ("boss", "挑战BOSS"),
    ("gather", "采集药材"),
];

/// 生成探索记录的key
fn explore_key(user_id: &str, map_name: &str, task_type: &str) -> String {
    format!("{}_{}_{}", user_id, map_name, task_type)
}

/// djb2哈希
#[allow(dead_code)]
fn djb2_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// 记录探索事件
#[allow(dead_code)]
pub fn record_explore(db: &Database, user_id: &str, map_name: &str, task_type: &str) {
    let section = "map_explore";
    let key = explore_key(user_id, map_name, task_type);
    let current: i32 = db.global_get(section, &key).parse().unwrap_or(0);
    db.global_set(section, &key, &(current + 1).to_string());
    // 同时记录总探索次数
    let total_key = format!("{}_total", user_id);
    let total: i32 = db.global_get(section, &total_key).parse().unwrap_or(0);
    db.global_set(section, &total_key, &(total + 1).to_string());
    // 记录已探索地图
    let maps_key = format!("{}_maps", user_id);
    let maps_raw = db.global_get(section, &maps_key);
    if !maps_raw.contains(map_name) {
        let new_maps = if maps_raw.is_empty() {
            map_name.to_string()
        } else {
            format!("{},{}", maps_raw, map_name)
        };
        db.global_set(section, &maps_key, &new_maps);
    }
}

/// 获取地图探索进度 (0-100%)
fn calc_map_progress(db: &Database, user_id: &str, map_name: &str) -> i32 {
    let section = "map_explore";
    let mut total_tasks = 0i32;
    let mut completed_tasks = 0i32;

    for (task_type, _desc) in EXPLORE_TASKS {
        total_tasks += 1;
        let key = explore_key(user_id, map_name, task_type);
        let count: i32 = db.global_get(section, &key).parse().unwrap_or(0);
        // 每种任务需要至少完成3次算完成
        if count >= 3 {
            completed_tasks += 1;
        }
    }

    if total_tasks == 0 {
        return 0;
    }
    (completed_tasks * 100) / total_tasks
}

/// 进度条可视化
fn progress_bar(pct: i32, width: usize) -> String {
    let filled = (pct as usize * width) / 100;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// 探索等级称号
fn explore_title(pct: i32) -> (&'static str, &'static str) {
    match pct {
        0..=24 => ("未探索", "⬜"),
        25..=49 => ("初探者", "🔍"),
        50..=74 => ("探路者", "🧭"),
        75..=99 => ("探险家", "🗺️"),
        _ => ("探索大师", "👑"),
    }
}

/// 获取所有地图名
fn get_all_maps() -> Vec<(&'static str, &'static str, i32)> {
    vec![
        ("复活岛", "复活点，也是安全区", 1),
        ("格兰森林", "新手的天堂", 1),
        ("艾尔村庄", "新手村", 1),
        ("莱茵高原", "脱离菜鸟的人才可以进入", 10),
        ("磐石之地", "格兰森林深处", 12),
        ("破旧的小屋", "神秘小屋", 15),
        ("寂寞之路", "通往远方的路", 15),
        ("沼泽之地", "危险的沼泽", 18),
        ("黄沙盆地", "干燥的盆地", 20),
        ("暗影森林", "被黑暗笼罩的森林", 22),
        ("冰封峡谷", "终年冰封", 25),
        ("熔岩洞窟", "炙热的地下", 28),
        ("圣光城", "神圣的城市", 30),
        ("溧寒の城", "古老的城市", 35),
        ("沙斯州市", "繁华的都市", 40),
        ("古代遗迹", "远古文明", 45),
        ("黑暗深渊", "无尽的黑暗", 50),
        ("天空之城", "浮空城市", 55),
        ("龙之巢穴", "龙族领地", 60),
        ("深渊之底", "最深处", 65),
        ("时空裂缝", "时空错乱", 70),
        ("混沌领域", "混沌之地", 75),
        ("创世神殿", "神的殿堂", 80),
        ("灰烬村庄", "被毁灭的村庄", 30),
        ("时王实验室", "时间实验室", 35),
    ]
}

/// 查看探索进度 — 显示当前地图或全部地图的探索情况
pub fn cmd_view_explore(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看探索进度！", prefix);
    }

    let section = "map_explore";
    let all_maps = get_all_maps();

    // 指定地图时显示详情
    if !args.trim().is_empty() {
        let query = args.trim();
        let found = all_maps.iter().find(|(name, _, _)| name.contains(query));
        if let Some((name, _intro, min_lv)) = found {
            let pct = calc_map_progress(db, user_id, name);
            let (title, icon) = explore_title(pct);

            let mut result = format!(
                "{}\n╔═════════════════════════════╗\n║  {} {} 探索详情  ║\n╚═════════════════════════════╝\n\n",
                prefix, icon, name
            );

            result.push_str(&format!("  📝 {} (Lv.{}+)\n\n", _intro, min_lv));
            result.push_str(&format!("  探索进度: {} {}%\n", progress_bar(pct, 10), pct));
            result.push_str(&format!("  探索等级: {} {}\n\n", icon, title));

            result.push_str("  ── 探索任务 ──\n");
            for (task_type, desc) in EXPLORE_TASKS {
                let key = explore_key(user_id, name, task_type);
                let count: i32 = db.global_get(section, &key).parse().unwrap_or(0);
                let status = if count >= 3 { "✅" } else { "⬜" };
                result.push_str(&format!("  {} {}: {}/3 次\n", status, desc, count.min(3)));
            }

            // 显示里程碑
            result.push_str("\n  ── 探索里程碑 ──\n");
            let claimed_key = format!("{}_{}_claimed", user_id, name);
            let claimed_raw = db.global_get(section, &claimed_key);
            for m in EXPLORE_MILESTONES {
                let claimed = claimed_raw.contains(&m.pct.to_string());
                let status = if claimed {
                    "✅"
                } else if pct >= m.pct {
                    "🎁"
                } else {
                    "🔒"
                };
                result.push_str(&format!(
                    "  {} {}% {} — {}金 {}钻 {}exp\n",
                    status, m.pct, m.name, m.reward_gold, m.reward_diamond, m.reward_exp
                ));
            }

            if pct < 100 {
                result.push_str("\n  💡 完成所有探索任务（每种至少3次）提升进度！");
            } else {
                result.push_str("\n  🎉 恭喜！这张地图已完全探索！");
            }

            return result;
        } else {
            return format!(
                "{}\n❌ 未找到包含「{}」的地图\n💡 使用「探索进度」查看所有地图",
                prefix, query
            );
        }
    }

    // 全部地图概览
    let mut result = format!(
        "{}\n╔═════════════════════════════╗\n║  🗺️  地图探索进度  ║\n╚═════════════════════════════╝\n\n",
        prefix
    );

    let mut total_pct = 0i64;
    let mut fully_explored = 0i32;
    let mut maps_with_progress = 0i32;

    for (name, _intro, min_lv) in &all_maps {
        let pct = calc_map_progress(db, user_id, name);
        total_pct += pct as i64;
        if pct >= 100 {
            fully_explored += 1;
        }
        if pct > 0 {
            maps_with_progress += 1;
        }
        let (title, icon) = explore_title(pct);
        result.push_str(&format!(
            "  {} {} Lv.{:>2}+ {:>3}% {} {}\n",
            icon,
            name,
            min_lv,
            pct,
            progress_bar(pct, 5),
            title
        ));
    }

    let overall_pct = if all_maps.is_empty() {
        0
    } else {
        (total_pct / all_maps.len() as i64) as i32
    };
    let (overall_title, overall_icon) = explore_title(overall_pct);

    result.push_str(&format!(
        "\n━━━━━━━━━━━━━━━━━━━━━━━━\n\
         📊 总体探索: {} {}%\n\
         {} 已完全探索: {}/{} 张\n\
         🔍 已探索地图: {}/{} 张\n\
         🏆 探索等级: {} {}\n\
         \n\
         💡 使用「探索详情+地图名」查看单张地图详情\n\
         💡 使用「探索奖励+地图名」领取里程碑奖励",
        progress_bar(overall_pct, 10),
        overall_pct,
        "🗺️",
        fully_explored,
        all_maps.len(),
        maps_with_progress,
        all_maps.len(),
        overall_icon,
        overall_title
    ));

    result
}

/// 领取探索奖励 — 达到里程碑后领取奖励
pub fn cmd_claim_explore_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再领取奖励！", prefix);
    }

    let map_name = args.trim();
    if map_name.is_empty() {
        return format!(
            "{}\n❓ 请指定地图名！\n💡 用法: 探索奖励+地图名\n📋 示例: 探索奖励+格兰森林",
            prefix
        );
    }

    let all_maps = get_all_maps();
    let found = all_maps.iter().find(|(name, _, _)| name.contains(map_name));
    let (name, _, _) = match found {
        Some(m) => m,
        None => return format!("{}\n❌ 未找到包含「{}」的地图", prefix, map_name),
    };

    let section = "map_explore";
    let pct = calc_map_progress(db, user_id, name);

    let mut claimed_any = false;
    let mut total_gold = 0i64;
    let mut total_diamond = 0i64;
    let mut total_exp = 0i32;
    let mut claimed_pcts = Vec::new();
    let mut already_claimed = Vec::new();

    let claimed_key = format!("{}_{}_claimed", user_id, name);
    let claimed_raw = db.global_get(section, &claimed_key);

    for m in EXPLORE_MILESTONES {
        if pct < m.pct {
            continue;
        }
        if claimed_raw.contains(&m.pct.to_string()) {
            already_claimed.push(m.name);
            continue;
        }
        total_gold += m.reward_gold as i64;
        total_diamond += m.reward_diamond as i64;
        total_exp += m.reward_exp;
        claimed_pcts.push(m.pct.to_string());
        claimed_any = true;
    }

    if !claimed_any {
        if !already_claimed.is_empty() {
            return format!(
                "{}\n📋 {}的所有可领取奖励已领取完毕！\n💡 继续探索提升进度解锁更多奖励",
                prefix, name
            );
        }
        return format!(
            "{}\n🔒 {} 的探索进度不足（当前{}%）\n💡 至少达到25%才能领取奖励\n📋 使用「探索详情+{}」查看任务",
            prefix, name, pct, name
        );
    }

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, total_diamond);
    user::add_experience(db, user_id, total_exp);

    // 标记已领取
    let new_claimed = if claimed_raw.is_empty() {
        claimed_pcts.join(",")
    } else {
        let mut pcts: Vec<String> = claimed_raw.split(',').map(|s| s.to_string()).collect();
        for p in &claimed_pcts {
            if !pcts.contains(p) {
                pcts.push(p.clone());
            }
        }
        pcts.join(",")
    };
    db.global_set(section, &claimed_key, &new_claimed);

    let mut result = format!("{}\n🎉 探索奖励领取成功！\n\n📍 地图: {}\n", prefix, name);

    result.push_str("  ── 领取的里程碑 ──\n");
    for m in EXPLORE_MILESTONES {
        if claimed_pcts.contains(&m.pct.to_string()) {
            result.push_str(&format!("  ✅ {} {}%\n", m.name, m.pct));
        }
    }

    result.push_str(&format!(
        "\n  ── 奖励内容 ──\n\
         💰 金币: +{}\n\
         💎 钻石: +{}\n\
         ⭐ 经验: +{}",
        total_gold, total_diamond, total_exp
    ));

    if !already_claimed.is_empty() {
        result.push_str(&format!("\n\n  ℹ️ 已领取过: {}", already_claimed.join(", ")));
    }

    result
}

/// 探索排行 — 全服探索进度排行
pub fn cmd_explore_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看排行！", prefix);
    }

    let section = "map_explore";
    let all_maps = get_all_maps();

    // 从Basic_User获取用户列表
    let user_list: Vec<(String, String)> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT uID, NickName FROM Basic_User LIMIT 200") {
            if let Ok(mut rows) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?.trim_end_matches('\0').to_string(),
                    row.get::<_, String>(1)?.trim_end_matches('\0').to_string(),
                ))
            }) {
                while let Some(Ok(pair)) = rows.next() {
                    result.push(pair);
                }
            }
        }
        result
    };

    let mut user_scores: Vec<(String, String, i32, i32, i32)> = Vec::new();

    for (uid, nick) in &user_list {
        // 检查是否有探索数据
        let maps_raw = db.global_get(section, &format!("{}_maps", uid));
        if maps_raw.is_empty() {
            continue;
        }
        let maps_visited = maps_raw.split(',').count() as i32;

        let mut total_pct = 0i64;
        let mut fully_explored = 0i32;
        for (name, _, _) in &all_maps {
            let pct = calc_map_progress(db, uid, name);
            total_pct += pct as i64;
            if pct >= 100 {
                fully_explored += 1;
            }
        }
        let avg_pct = if all_maps.is_empty() {
            0
        } else {
            (total_pct / all_maps.len() as i64) as i32
        };
        user_scores.push((uid.clone(), nick.clone(), avg_pct, fully_explored, maps_visited));
    }

    // 按平均探索百分比排序
    user_scores.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));

    let medals = ["🥇", "🥈", "🥉"];
    let mut result = format!(
        "{}\n╔═════════════════════════════╗\n║  🏆 地图探索排行  ║\n╚═════════════════════════════╝\n\n",
        prefix
    );

    let display_count = user_scores.len().min(15);
    for (i, (uid, nick, avg_pct, fully_explored, _maps_visited)) in user_scores.iter().take(display_count).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let level: i32 = db.read_basic(uid, ITEM_LEVEL).parse().unwrap_or(1);
        let (title, icon) = explore_title(*avg_pct);
        let is_self = uid == user_id;
        let marker = if is_self { " ⬅️ 你" } else { "" };

        result.push_str(&format!(
            "  {} #{:>2} {} Lv{} {} {}% {}/{}完全探索 {}{}\n",
            medal,
            i + 1,
            nick,
            level,
            icon,
            avg_pct,
            fully_explored,
            all_maps.len(),
            title,
            marker
        ));
    }

    if user_scores.is_empty() {
        result.push_str("  暂无探索数据\n");
    }

    // 显示当前用户排名
    if let Some((pos, _)) = user_scores
        .iter()
        .enumerate()
        .find(|(_, (uid, _, _, _, _))| uid == user_id)
    {
        result.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━\n\
             📍 你的排名: #{} / {}\n",
            pos + 1,
            user_scores.len()
        ));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explore_milestones_count() {
        assert_eq!(EXPLORE_MILESTONES.len(), 4);
    }

    #[test]
    fn test_explore_milestones_ordering() {
        for i in 1..EXPLORE_MILESTONES.len() {
            assert!(EXPLORE_MILESTONES[i].pct > EXPLORE_MILESTONES[i - 1].pct);
        }
    }

    #[test]
    fn test_explore_milestones_rewards_positive() {
        for m in EXPLORE_MILESTONES {
            assert!(m.reward_gold > 0);
            assert!(m.reward_diamond > 0);
            assert!(m.reward_exp > 0);
        }
    }

    #[test]
    fn test_explore_tasks_count() {
        assert_eq!(EXPLORE_TASKS.len(), 5);
    }

    #[test]
    fn test_explore_title_range() {
        let (name, icon) = explore_title(0);
        assert_eq!(name, "未探索");
        assert!(!icon.is_empty());

        let (name, _) = explore_title(25);
        assert_eq!(name, "初探者");

        let (name, _) = explore_title(50);
        assert_eq!(name, "探路者");

        let (name, _) = explore_title(75);
        assert_eq!(name, "探险家");

        let (name, _) = explore_title(100);
        assert_eq!(name, "探索大师");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 10);
        assert_eq!(bar, "█████░░░░░");
    }

    #[test]
    fn test_get_all_maps_count() {
        let maps = get_all_maps();
        assert!(maps.len() >= 20);
    }

    #[test]
    fn test_djb2_hash_deterministic() {
        let h1 = djb2_hash("test_map_kill");
        let h2 = djb2_hash("test_map_kill");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_djb2_hash_different() {
        let h1 = djb2_hash("map_a");
        let h2 = djb2_hash("map_b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_explore_key_format() {
        let key = explore_key("user123", "格兰森林", "kill");
        assert_eq!(key, "user123_格兰森林_kill");
    }

    #[test]
    fn test_explore_milestones_full_range() {
        let pcts: Vec<i32> = EXPLORE_MILESTONES.iter().map(|m| m.pct).collect();
        assert!(pcts.contains(&25));
        assert!(pcts.contains(&50));
        assert!(pcts.contains(&75));
        assert!(pcts.contains(&100));
    }
}

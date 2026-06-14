/// CakeGame 赛季征途系统
/// 章节制赛季目标，5章递进难度，每章含6-8个成就目标
/// 完成章节解锁专属奖励，追踪个人进度
/// 数据存储: Global 表 SECTION='season_journey_{user_id}'
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 章节定义
struct ChapterDef {
    id: i32,
    name: &'static str,
    emoji: &'static str,
    description: &'static str,
    /// 目标定义: (目标ID, 目标名, 目标类型, 目标值, 描述)
    objectives: &'static [(&'static str, &'static str, &'static str, i64, &'static str)],
    /// 章节奖励: (金币, 钻石, 道具名, 道具数量)
    reward_gold: i64,
    reward_diamond: i32,
    reward_item: &'static str,
    reward_item_qty: i32,
}

const CHAPTERS: &[ChapterDef] = &[
    ChapterDef {
        id: 1,
        name: "征途初启",
        emoji: "🌱",
        description: "踏上征途的第一步，完成基础挑战",
        objectives: &[
            ("reach_lv10", "等级达到10级", "level", 10, "提升角色等级至10"),
            ("kill_50_monsters", "击杀50只怪物", "kill", 50, "在野外击败50只怪物"),
            (
                "earn_10000_gold",
                "累计获得10000金币",
                "gold",
                10000,
                "通过战斗和交易赚取金币",
            ),
            ("enhance_once", "强化装备1次", "enhance", 1, "对任意装备进行强化"),
            ("sign_in_3_days", "连续签到3天", "sign_in", 3, "每日签到不可中断"),
            (
                "collect_10_items",
                "收集10种物品",
                "collect",
                10,
                "背包中持有10种不同物品",
            ),
        ],
        reward_gold: 5000,
        reward_diamond: 50,
        reward_item: "征途宝箱·初",
        reward_item_qty: 1,
    },
    ChapterDef {
        id: 2,
        name: "历练成长",
        emoji: "🌿",
        description: "在战斗中磨砺自己，掌握更多技能",
        objectives: &[
            ("reach_lv30", "等级达到30级", "level", 30, "提升角色等级至30"),
            ("kill_200_monsters", "击杀200只怪物", "kill", 200, "累计击败200只怪物"),
            ("earn_100000_gold", "累计获得10万金币", "gold", 100000, "财富积累的证明"),
            ("enhance_5_times", "强化装备5次", "enhance", 5, "持续强化你的装备"),
            ("sign_in_7_days", "连续签到7天", "sign_in", 7, "一周不间断签到"),
            ("join_guild", "加入公会", "guild", 1, "寻找志同道合的伙伴"),
            ("complete_5_quests", "完成5个任务", "quest", 5, "接受并完成各类任务"),
        ],
        reward_gold: 20000,
        reward_diamond: 100,
        reward_item: "征途宝箱·历",
        reward_item_qty: 1,
    },
    ChapterDef {
        id: 3,
        name: "锋芒初露",
        emoji: "⚔️",
        description: "在竞技场和副本中展现实力",
        objectives: &[
            ("reach_lv50", "等级达到50级", "level", 50, "半百之境"),
            ("kill_500_monsters", "击杀500只怪物", "kill", 500, "战斗经验丰富"),
            ("earn_500000_gold", "累计获得50万金币", "gold", 500000, "富甲一方"),
            ("enhance_10_times", "强化装备10次", "enhance", 10, "装备不断精进"),
            ("sign_in_14_days", "连续签到14天", "sign_in", 14, "两周坚持"),
            ("challenge_boss_3", "挑战BOSS 3次", "boss", 3, "直面强敌"),
            ("pvp_win_5", "PvP胜利5次", "pvp", 5, "在玩家对战中取胜"),
            ("craft_3_items", "合成3件物品", "craft", 3, "掌握合成技艺"),
        ],
        reward_gold: 50000,
        reward_diamond: 200,
        reward_item: "征途宝箱·锋",
        reward_item_qty: 1,
    },
    ChapterDef {
        id: 4,
        name: "王者之路",
        emoji: "🏆",
        description: "挑战更高难度，问鼎全服巅峰",
        objectives: &[
            ("reach_lv80", "等级达到80级", "level", 80, "高阶冒险者"),
            ("kill_1000_monsters", "击杀1000只怪物", "kill", 1000, "千斩之名"),
            ("earn_2000000_gold", "累计获得200万金币", "gold", 2000000, "金币大亨"),
            ("enhance_20_times", "强化装备20次", "enhance", 20, "强化达人"),
            ("sign_in_30_days", "连续签到30天", "sign_in", 30, "一月坚持"),
            ("challenge_boss_10", "挑战BOSS 10次", "boss", 10, "BOSS猎人"),
            ("pvp_win_20", "PvP胜利20次", "pvp", 20, "竞技强者"),
            ("guild_donate_10", "公会捐献10次", "guild_donate", 10, "公会栋梁"),
        ],
        reward_gold: 200000,
        reward_diamond: 500,
        reward_item: "征途宝箱·王",
        reward_item_qty: 1,
    },
    ChapterDef {
        id: 5,
        name: "传说永恒",
        emoji: "✨",
        description: "登顶征途之巅，铸就不朽传说",
        objectives: &[
            ("reach_lv100", "等级达到100级", "level", 100, "百级传说"),
            ("kill_5000_monsters", "击杀5000只怪物", "kill", 5000, "万夫莫敌"),
            ("earn_10000000_gold", "累计获得1000万金币", "gold", 10000000, "富可敌国"),
            ("enhance_50_times", "强化装备50次", "enhance", 50, "强化宗师"),
            ("sign_in_60_days", "连续签到60天", "sign_in", 60, "两个月坚持"),
            ("challenge_boss_30", "挑战BOSS 30次", "boss", 30, "BOSS终结者"),
            ("pvp_win_50", "PvP胜利50次", "pvp", 50, "竞技王者"),
            ("guild_donate_30", "公会捐献30次", "guild_donate", 30, "公会功臣"),
        ],
        reward_gold: 1000000,
        reward_diamond: 2000,
        reward_item: "征途宝箱·传",
        reward_item_qty: 1,
    },
];

/// 章节总数
fn total_chapters() -> usize {
    CHAPTERS.len()
}

/// 查找章节
fn find_chapter(id: i32) -> Option<&'static ChapterDef> {
    CHAPTERS.iter().find(|c| c.id == id)
}

/// 用户数据 Section
fn user_section(user_id: &str) -> String {
    format!("season_journey_{}", user_id)
}

/// 获取用户的当前章节
fn get_current_chapter(db: &Database, user_id: &str) -> i32 {
    let val = db.global_get(&user_section(user_id), "current_chapter");
    if val.is_empty() {
        1
    } else {
        val.parse().unwrap_or(1)
    }
}

/// 获取目标进度
fn get_progress(db: &Database, user_id: &str, obj_id: &str) -> i64 {
    let key = format!("prog_{}", obj_id);
    db.global_get(&user_section(user_id), &key).parse().unwrap_or(0)
}

/// 设置目标进度
#[allow(dead_code)]
fn set_progress(db: &Database, user_id: &str, obj_id: &str, value: i64) {
    let key = format!("prog_{}", obj_id);
    db.global_set(&user_section(user_id), &key, &value.to_string());
}

/// 检查章节是否已完成（所有目标达成）
fn is_chapter_complete(db: &Database, user_id: &str, chapter: &ChapterDef) -> bool {
    chapter
        .objectives
        .iter()
        .all(|(obj_id, _, _, target, _)| get_progress(db, user_id, obj_id) >= *target)
}

/// 检查章节奖励是否已领取
fn is_reward_claimed(db: &Database, user_id: &str, chapter_id: i32) -> bool {
    let key = format!("claimed_{}", chapter_id);
    db.global_get(&user_section(user_id), &key) == "1"
}

/// 进度条
fn progress_bar(current: i64, target: i64, width: usize) -> String {
    let pct = if target > 0 {
        (current.min(target) as f64 / target as f64 * 100.0).min(100.0)
    } else {
        100.0
    };
    let filled = (pct / 100.0 * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{} {:.0}%", "█".repeat(filled), "░".repeat(empty), pct)
}

/// 记录征途进度 — 供其他模块调用
/// obj_type: "level", "kill", "gold", "enhance", "sign_in", "collect",
///           "guild", "quest", "boss", "pvp", "craft", "guild_donate"
/// amount: 累积量（level用当前等级，其他用增量）
#[allow(dead_code)]
pub fn record_journey_progress(db: &Database, user_id: &str, obj_type: &str, amount: i64) {
    let current_chapter = get_current_chapter(db, user_id);
    if current_chapter > total_chapters() as i32 {
        return; // 全部完成
    }

    if let Some(chapter) = find_chapter(current_chapter) {
        for (obj_id, _, otype, _target, _) in chapter.objectives {
            if *otype == obj_type {
                let key = format!("prog_{}", obj_id);
                let current: i64 = db.global_get(&user_section(user_id), &key).parse().unwrap_or(0);
                let new_val = if obj_type == "level" {
                    amount // 直接设置为当前等级
                } else {
                    current + amount
                };
                db.global_set(&user_section(user_id), &key, &new_val.to_string());
            }
        }
    }
}

/// 查看征途 — 主指令
pub fn cmd_view_season_journey(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current_chapter = get_current_chapter(db, user_id);
    let mut out = format!("{}\n═══ 🗺️ 赛季征途 ═══\n", prefix);

    for chapter in CHAPTERS {
        let status = if chapter.id < current_chapter {
            "✅ 已完成"
        } else if chapter.id == current_chapter {
            if is_chapter_complete(db, user_id, chapter) {
                "🎁 可领取"
            } else {
                "🔓 进行中"
            }
        } else {
            "🔒 未解锁"
        };

        let completed_count = chapter
            .objectives
            .iter()
            .filter(|(oid, _, _, target, _)| get_progress(db, user_id, oid) >= *target)
            .count();
        let total = chapter.objectives.len();

        out.push_str(&format!(
            "\n{} {}章·{} [{}/{}] {}",
            chapter.emoji, chapter.id, chapter.name, completed_count, total, status
        ));
    }

    out.push_str(
        "

💡 使用「征途详情+章节号」查看具体目标",
    );
    out
}

/// 征途详情 — 查看某章的具体目标
pub fn cmd_journey_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let chapter_id: i32 = args.trim().parse().unwrap_or(0);
    if chapter_id < 1 || chapter_id > total_chapters() as i32 {
        return format!("{}\n❌ 无效的章节号！请输入1~{}的数字。", prefix, total_chapters());
    }

    let chapter = match find_chapter(chapter_id) {
        Some(c) => c,
        None => return format!("{}\n❌ 章节不存在。", prefix),
    };

    let current_chapter = get_current_chapter(db, user_id);
    let locked = chapter_id > current_chapter;

    let mut out = format!(
        "\n{} {} {}章·{} {}\n{}\n",
        chapter.emoji, chapter.emoji, chapter.id, chapter.name, chapter.emoji, chapter.description
    );

    if locked {
        out.push_str("\n🔒 请先完成前一章节的所有目标。");
        return out;
    }

    out.push_str("\n📋 目标列表：\n");
    for (i, (obj_id, name, _, target, desc)) in chapter.objectives.iter().enumerate() {
        let prog = get_progress(db, user_id, obj_id);
        let done = prog >= *target;
        let icon = if done { "✅" } else { "⬜" };
        let bar = progress_bar(prog, *target, 8);
        out.push_str(&format!(
            "\n{}. {} {} {} {}/{}",
            i + 1,
            icon,
            name,
            bar,
            prog.min(*target),
            target
        ));
        if !done {
            out.push_str(&format!("\n   💬 {}", desc));
        }
    }

    // 奖励预览
    out.push_str(&format!(
        "\n\n🎁 章节奖励:\n   💰 {}金币 + 💎 {}钻石 + 🎁 {}×{}",
        chapter.reward_gold, chapter.reward_diamond, chapter.reward_item, chapter.reward_item_qty
    ));

    if is_chapter_complete(db, user_id, chapter) {
        if is_reward_claimed(db, user_id, chapter_id) {
            out.push_str("\n\n✅ 奖励已领取！");
        } else {
            out.push_str("\n\n🎉 所有目标已完成！使用「领取征途奖励+章节号」领取奖励！");
        }
    }

    out
}

/// 领取征途奖励
pub fn cmd_claim_journey_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let chapter_id: i32 = args.trim().parse().unwrap_or(0);
    if chapter_id < 1 || chapter_id > total_chapters() as i32 {
        return format!("{}\n❌ 无效的章节号！请输入1~{}的数字。", prefix, total_chapters());
    }

    let chapter = match find_chapter(chapter_id) {
        Some(c) => c,
        None => return format!("{}\n❌ 章节不存在。", prefix),
    };

    let current_chapter = get_current_chapter(db, user_id);
    if chapter_id > current_chapter {
        return format!("{}\n🔒 该章节尚未解锁！请先完成前一章节。", prefix);
    }

    if is_reward_claimed(db, user_id, chapter_id) {
        return format!("{}\n❌ 该章节奖励已领取！", prefix);
    }

    if !is_chapter_complete(db, user_id, chapter) {
        return format!("{}\n❌ 该章节还有未完成的目标！", prefix);
    }

    // 发放奖励
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, chapter.reward_gold);
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, chapter.reward_diamond as i64);
    if !chapter.reward_item.is_empty() {
        db.knapsack_add(user_id, chapter.reward_item, chapter.reward_item_qty);
    }

    // 标记已领取
    let key = format!("claimed_{}", chapter_id);
    db.global_set(&user_section(user_id), &key, "1");

    // 推进到下一章节
    if chapter_id == current_chapter && chapter_id < total_chapters() as i32 {
        db.global_set(&user_section(user_id), "current_chapter", &(chapter_id + 1).to_string());
    }

    // 记录征途完成时间
    let time_key = format!("completed_{}", chapter_id);
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();
    db.global_set(&user_section(user_id), &time_key, &now);

    format!(
        "\n{}🎉 恭喜完成征途第{}章·{}！\n\n🎁 获得奖励:\n   💰 {}金币\n   💎 {}钻石\n   🎁 {}×{}\n\n🗺️ 下一章节已解锁！使用「征途详情+{}」查看新目标。",
        prefix,
        chapter_id,
        chapter.name,
        chapter.reward_gold,
        chapter.reward_diamond,
        chapter.reward_item,
        chapter.reward_item_qty,
        if chapter_id < total_chapters() as i32 { chapter_id + 1 } else { chapter_id }
    )
}

/// 征途排行 — 全服征途进度排行
pub fn cmd_journey_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut players: Vec<(String, i32, i32)> = Vec::new(); // (name, chapter, completed_objectives)

    for uid in db.all_users().iter() {
        let section = user_section(uid);
        let chapter: i32 = db.global_get(&section, "current_chapter").parse().unwrap_or(1);
        let mut total_completed = 0i32;

        for ch in CHAPTERS {
            for (obj_id, _, _, target, _) in ch.objectives {
                let prog = db.global_get(&section, &format!("prog_{}", obj_id));
                let prog_val: i64 = prog.parse().unwrap_or(0);
                if prog_val >= *target {
                    total_completed += 1;
                }
            }
        }

        if total_completed > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, chapter, total_completed));
        }
    }

    if players.is_empty() {
        return format!("{}\n暂无征途数据。", prefix);
    }

    players.sort_by_key(|b| std::cmp::Reverse((b.1, b.2)));

    let mut out = format!("{}\n═══ 🗺️ 征途排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, chapter, completed)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        out.push_str(&format!(
            "\n{} {} — 第{}章 目标完成{}/{}",
            medal,
            name,
            chapter,
            completed,
            total_objectives()
        ));
    }

    // 用户排名
    let user_name = user::get_msg_prefix(db, user_id);
    if let Some(rank) = players.iter().position(|(name, _, _)| name == &user_name) {
        out.push_str(&format!("\n\n📍 你的排名：第{}名", rank + 1));
    }

    out
}

/// 总目标数
fn total_objectives() -> usize {
    CHAPTERS.iter().map(|c| c.objectives.len()).sum()
}

/// 征途统计 — 个人征途数据概览
pub fn cmd_journey_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current_chapter = get_current_chapter(db, user_id);
    let mut total_completed_obj = 0usize;
    let mut total_obj = 0usize;
    let mut claimed_chapters = 0usize;

    for ch in CHAPTERS {
        for (obj_id, _, _, target, _) in ch.objectives {
            total_obj += 1;
            let prog = get_progress(db, user_id, obj_id);
            if prog >= *target {
                total_completed_obj += 1;
            }
        }
        if is_reward_claimed(db, user_id, ch.id) {
            claimed_chapters += 1;
        }
    }

    let total_gold_earned: i64 = CHAPTERS
        .iter()
        .filter(|c| is_reward_claimed(db, user_id, c.id))
        .map(|c| c.reward_gold)
        .sum();
    let total_diamond_earned: i32 = CHAPTERS
        .iter()
        .filter(|c| is_reward_claimed(db, user_id, c.id))
        .map(|c| c.reward_diamond)
        .sum();

    let pct = if total_obj > 0 {
        total_completed_obj as f64 / total_obj as f64 * 100.0
    } else {
        0.0
    };

    let bar = progress_bar(total_completed_obj as i64, total_obj as i64, 12);

    format!(
        "\n{}═══ 📊 征途统计 ═══\n\n\
         🗺️ 当前章节：第{}章·{}\n\
         📋 目标完成：{}/{} ({:.1}%)\n\
         {}\n\
         🏆 已通关章节：{}/{}\n\
         💰 累计获得金币：{}\n\
         💎 累计获得钻石：{}\n\
         🎁 已领取宝箱：{}个",
        prefix,
        current_chapter,
        CHAPTERS
            .iter()
            .find(|c| c.id == current_chapter)
            .map(|c| c.name)
            .unwrap_or("已完成"),
        total_completed_obj,
        total_obj,
        pct,
        bar,
        claimed_chapters,
        total_chapters(),
        total_gold_earned,
        total_diamond_earned,
        claimed_chapters,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chapter_count() {
        assert_eq!(CHAPTERS.len(), 5);
    }

    #[test]
    fn test_chapter_ids_unique() {
        let mut ids: Vec<i32> = CHAPTERS.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CHAPTERS.len());
    }

    #[test]
    fn test_chapter_ids_sequential() {
        for (i, ch) in CHAPTERS.iter().enumerate() {
            assert_eq!(ch.id, (i + 1) as i32);
        }
    }

    #[test]
    fn test_objective_ids_unique_per_chapter() {
        for ch in CHAPTERS {
            let mut ids: Vec<&str> = ch.objectives.iter().map(|(id, _, _, _, _)| *id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), ch.objectives.len());
        }
    }

    #[test]
    fn test_objective_targets_positive() {
        for ch in CHAPTERS {
            for (_, _, _, target, _) in ch.objectives {
                assert!(*target > 0);
            }
        }
    }

    #[test]
    fn test_objective_escalation() {
        // 等级目标应递增
        let level_targets: Vec<i64> = CHAPTERS
            .iter()
            .filter_map(|c| {
                c.objectives
                    .iter()
                    .find(|(_, _, t, _, _)| *t == "level")
                    .map(|(_, _, _, t, _)| *t)
            })
            .collect();
        for w in level_targets.windows(2) {
            assert!(w[1] > w[0], "Level targets should escalate");
        }
    }

    #[test]
    fn test_kill_targets_escalate() {
        let kill_targets: Vec<i64> = CHAPTERS
            .iter()
            .filter_map(|c| {
                c.objectives
                    .iter()
                    .find(|(_, _, t, _, _)| *t == "kill")
                    .map(|(_, _, _, t, _)| *t)
            })
            .collect();
        for w in kill_targets.windows(2) {
            assert!(w[1] > w[0], "Kill targets should escalate");
        }
    }

    #[test]
    fn test_reward_escalation() {
        for w in CHAPTERS.windows(2) {
            assert!(w[1].reward_gold >= w[0].reward_gold, "Gold reward should escalate");
            assert!(
                w[1].reward_diamond >= w[0].reward_diamond,
                "Diamond reward should escalate"
            );
        }
    }

    #[test]
    fn test_reward_items_defined() {
        for ch in CHAPTERS {
            assert!(!ch.reward_item.is_empty());
            assert!(ch.reward_item_qty > 0);
        }
    }

    #[test]
    fn test_chapter_names_unique() {
        let mut names: Vec<&str> = CHAPTERS.iter().map(|c| c.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), CHAPTERS.len());
    }

    #[test]
    fn test_total_objectives() {
        let total: usize = CHAPTERS.iter().map(|c| c.objectives.len()).sum();
        assert!(total >= 30, "Should have at least 30 total objectives");
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100, 10);
        assert!(bar.contains("0%"));
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100, 10);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_find_chapter_valid() {
        assert!(find_chapter(1).is_some());
        assert!(find_chapter(5).is_some());
        assert!(find_chapter(0).is_none());
        assert!(find_chapter(6).is_none());
    }

    #[test]
    fn test_user_section_format() {
        let section = user_section("test_user");
        assert_eq!(section, "season_journey_test_user");
    }

    #[test]
    fn test_objective_descriptions() {
        for ch in CHAPTERS {
            for (_, name, _, _, desc) in ch.objectives {
                assert!(!name.is_empty());
                assert!(!desc.is_empty());
            }
        }
    }

    #[test]
    fn test_all_objective_types_valid() {
        let valid_types = [
            "level",
            "kill",
            "gold",
            "enhance",
            "sign_in",
            "collect",
            "guild",
            "quest",
            "boss",
            "pvp",
            "craft",
            "guild_donate",
        ];
        for ch in CHAPTERS {
            for (_, _, otype, _, _) in ch.objectives {
                assert!(valid_types.contains(otype), "Unknown objective type: {}", otype);
            }
        }
    }

    #[test]
    fn test_emoji_defined() {
        for ch in CHAPTERS {
            assert!(!ch.emoji.is_empty());
        }
    }
}

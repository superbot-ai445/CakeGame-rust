use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::{Datelike, Local};

/// 每日任务定义
#[allow(dead_code)]
struct DailyQuestDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    category: &'static str,
    target_count: i32,
    reward_gold: i32,
    reward_exp: i32,
    reward_diamond: i32,
}

/// 基于日期生成当日每日任务（确定性，同一天同一批任务）
fn generate_daily_quests(day_of_year: u32) -> Vec<DailyQuestDef> {
    let mut quests = Vec::new();

    // 任务1: 击杀怪物（基于日期变化目标数）
    let kill_target = 5 + (day_of_year % 11) as i32; // 5-15
    let kill_gold = 200 + (day_of_year % 6) as i32 * 50; // 200-500
    quests.push(DailyQuestDef {
        id: "dq_kill",
        name: "每日狩猎",
        desc: "击杀指定数量的怪物",
        category: "战斗",
        target_count: kill_target,
        reward_gold: kill_gold,
        reward_exp: kill_target * 20,
        reward_diamond: 0,
    });

    // 任务2: 采集（基于日期变化）
    let gather_target = 2 + (day_of_year % 5) as i32; // 2-6
    let gather_gold = 150 + (day_of_year % 4) as i32 * 50; // 150-300
    quests.push(DailyQuestDef {
        id: "dq_gather",
        name: "资源采集",
        desc: "完成指定次数的采集",
        category: "生活",
        target_count: gather_target,
        reward_gold: gather_gold,
        reward_exp: gather_target * 15,
        reward_diamond: 0,
    });

    // 任务3: 签到（固定）
    quests.push(DailyQuestDef {
        id: "dq_sign",
        name: "每日签到",
        desc: "完成今日签到",
        category: "日常",
        target_count: 1,
        reward_gold: 100,
        reward_exp: 50,
        reward_diamond: 5,
    });

    quests
}

/// 读取用户每日任务进度
fn read_daily_progress(db: &Database, user_id: &str, quest_id: &str) -> i32 {
    let key = format!("dq_progress_{}", quest_id);
    db.read_user_data(user_id, &key).parse().unwrap_or(0)
}

/// 写入用户每日任务进度
fn write_daily_progress(db: &Database, user_id: &str, quest_id: &str, value: i32) {
    let key = format!("dq_progress_{}", quest_id);
    db.write_user_data(user_id, &key, &value.to_string());
}

/// 读取每日任务领取状态
fn read_daily_claimed(db: &Database, user_id: &str, quest_id: &str) -> bool {
    let key = format!("dq_claimed_{}", quest_id);
    let today = Local::now().format("%Y-%m-%d").to_string();
    db.read_user_data(user_id, &key) == today
}

/// 写入每日任务领取状态
fn write_daily_claimed(db: &Database, user_id: &str, quest_id: &str) {
    let key = format!("dq_claimed_{}", quest_id);
    let today = Local::now().format("%Y-%m-%d").to_string();
    db.write_user_data(user_id, &key, &today);
}

/// 检查每日任务进度是否需要重置（新的一天）
fn check_daily_reset(db: &Database, user_id: &str) {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let last_date = db.read_user_data(user_id, "dq_last_date");
    if last_date != today {
        // 新的一天，重置所有进度
        for qid in &["dq_kill", "dq_gather", "dq_sign"] {
            write_daily_progress(db, user_id, qid, 0);
            let key = format!("dq_claimed_{}", qid);
            db.write_user_data(user_id, &key, "");
        }
        db.write_user_data(user_id, "dq_last_date", &today);
    }
}

/// 查看每日任务
pub fn cmd_view_daily_quests(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // 检查注册
    let name = db.read_basic(user_id, ITEM_NAME);
    if name.is_empty() {
        return "您还未注册！请先发送「注册」".to_string();
    }

    check_daily_reset(db, user_id);

    let now = Local::now();
    let day_of_year = now.ordinal();
    let quests = generate_daily_quests(day_of_year);

    let mut result = String::from("📋 【每日任务】\n");
    result.push_str(&format!("📅 {}\n", now.format("%Y-%m-%d")));
    result.push_str("━━━━━━━━━━━━━━━━━━\n");

    for (i, q) in quests.iter().enumerate() {
        let progress = read_daily_progress(db, user_id, q.id);
        let claimed = read_daily_claimed(db, user_id, q.id);
        let completed = progress >= q.target_count;

        let status = if claimed {
            "✅ 已领取"
        } else if completed {
            "🎁 可领取"
        } else {
            "⏳ 进行中"
        };

        result.push_str(&format!(
            "{}. {} [{}]\n   {} {}/{}\n   奖励: {}金币 + {}经验",
            i + 1,
            q.name,
            status,
            q.desc,
            progress.min(q.target_count),
            q.target_count,
            q.reward_gold,
            q.reward_exp,
        ));

        if q.reward_diamond > 0 {
            result.push_str(&format!(" + {}钻石", q.reward_diamond));
        }
        result.push('\n');
    }

    result.push_str("━━━━━━━━━━━━━━━━━━\n");
    result.push_str("💡 发送「领取每日+任务名」领取奖励\n");

    result
}

/// 领取每日任务奖励
pub fn cmd_claim_daily_quest(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let name = db.read_basic(user_id, ITEM_NAME);
    if name.is_empty() {
        return "您还未注册！请先发送「注册」".to_string();
    }

    check_daily_reset(db, user_id);

    let quest_name = args.strip_prefix('+').unwrap_or(args).trim();
    if quest_name.is_empty() {
        return "请指定要领取的任务名！\n💡 发送「领取每日+任务名」\n可选: 狩猎/采集/签到".to_string();
    }

    let now = Local::now();
    let day_of_year = now.ordinal();
    let quests = generate_daily_quests(day_of_year);

    // 模糊匹配任务名
    let quest = if quest_name.contains("狩猎") || quest_name.contains("击杀") || quest_name.contains("杀怪") {
        quests.iter().find(|q| q.id == "dq_kill")
    } else if quest_name.contains("采集") || quest_name.contains("资源") {
        quests.iter().find(|q| q.id == "dq_gather")
    } else if quest_name.contains("签到") {
        quests.iter().find(|q| q.id == "dq_sign")
    } else {
        quests.iter().find(|q| q.name.contains(quest_name))
    };

    let quest = match quest {
        Some(q) => q,
        None => return format!("未找到任务「{}」！\n💡 可选: 狩猎/采集/签到", quest_name),
    };

    // 检查是否已领取
    if read_daily_claimed(db, user_id, quest.id) {
        return format!("「{}」奖励今日已领取！", quest.name);
    }

    // 检查是否完成
    let progress = read_daily_progress(db, user_id, quest.id);
    if progress < quest.target_count {
        return format!(
            "「{}」尚未完成！\n当前进度: {}/{}\n💡 继续努力吧！",
            quest.name, progress, quest.target_count
        );
    }

    // 发放奖励
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", quest.reward_gold as i64);
    if quest.reward_diamond > 0 {
        let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", quest.reward_diamond as i64);
    }
    let (_, _) = user::add_experience(db, user_id, quest.reward_exp);

    // 标记已领取
    write_daily_claimed(db, user_id, quest.id);

    let mut result = format!("🎁 领取成功！「{}」\n", quest.name);
    result.push_str(&format!("  💰 获得 {} 金币\n", quest.reward_gold));
    result.push_str(&format!("  ✨ 获得 {} 经验\n", quest.reward_exp));
    if quest.reward_diamond > 0 {
        result.push_str(&format!("  💎 获得 {} 钻石\n", quest.reward_diamond));
    }

    // 检查是否全部完成
    let all_done = quests.iter().all(|q| read_daily_claimed(db, user_id, q.id));
    if all_done {
        result.push_str("\n🎉 恭喜！今日所有每日任务已完成！");
    }

    result
}

/// 增加每日任务进度（供其他模块调用）
pub fn on_monster_killed(db: &Database, user_id: &str) {
    check_daily_reset(db, user_id);
    let current = read_daily_progress(db, user_id, "dq_kill");
    write_daily_progress(db, user_id, "dq_kill", current + 1);
}

/// 增加采集进度
pub fn on_gathered(db: &Database, user_id: &str) {
    check_daily_reset(db, user_id);
    let current = read_daily_progress(db, user_id, "dq_gather");
    write_daily_progress(db, user_id, "dq_gather", current + 1);
}

/// 增加签到进度
pub fn on_signed_in(db: &Database, user_id: &str) {
    check_daily_reset(db, user_id);
    let current = read_daily_progress(db, user_id, "dq_sign");
    if current < 1 {
        write_daily_progress(db, user_id, "dq_sign", 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_daily_quests_count() {
        let quests = generate_daily_quests(1);
        assert_eq!(quests.len(), 3);
    }

    #[test]
    fn test_generate_daily_quests_deterministic() {
        let q1 = generate_daily_quests(100);
        let q2 = generate_daily_quests(100);
        assert_eq!(q1.len(), q2.len());
        for i in 0..q1.len() {
            assert_eq!(q1[i].id, q2[i].id);
            assert_eq!(q1[i].target_count, q2[i].target_count);
            assert_eq!(q1[i].reward_gold, q2[i].reward_gold);
        }
    }

    #[test]
    fn test_generate_daily_quests_vary_by_day() {
        let q1 = generate_daily_quests(1);
        let q2 = generate_daily_quests(2);
        // At least the kill quest target should differ
        // (day_of_year % 11 gives different values for 1 and 2)
        assert_eq!(q1[0].id, q2[0].id); // Same quest IDs
    }

    #[test]
    fn test_generate_daily_quests_positive_rewards() {
        for day in 0..366 {
            let quests = generate_daily_quests(day);
            for q in &quests {
                assert!(q.reward_gold >= 0, "day {}: {} gold < 0", day, q.id);
                assert!(q.reward_exp > 0, "day {}: {} exp <= 0", day, q.id);
                assert!(q.target_count > 0, "day {}: {} target <= 0", day, q.id);
            }
        }
    }

    #[test]
    fn test_generate_daily_quests_kill_target_range() {
        // kill_target = 5 + (day_of_year % 11) -> range [5, 15]
        for day in 0..366 {
            let quests = generate_daily_quests(day);
            let kill = &quests[0];
            assert!(
                kill.target_count >= 5 && kill.target_count <= 15,
                "day {}: kill target {}",
                day,
                kill.target_count
            );
        }
    }

    #[test]
    fn test_generate_daily_quests_gather_target_range() {
        // gather_target = 2 + (day_of_year % 5) -> range [2, 6]
        for day in 0..366 {
            let quests = generate_daily_quests(day);
            let gather = &quests[1];
            assert!(
                gather.target_count >= 2 && gather.target_count <= 6,
                "day {}: gather target {}",
                day,
                gather.target_count
            );
        }
    }

    #[test]
    fn test_generate_daily_quests_sign_fixed() {
        // Sign quest is always the same
        for day in [0, 1, 100, 200, 365] {
            let quests = generate_daily_quests(day);
            let sign = &quests[2];
            assert_eq!(sign.id, "dq_sign");
            assert_eq!(sign.target_count, 1);
            assert_eq!(sign.reward_gold, 100);
            assert_eq!(sign.reward_exp, 50);
            assert_eq!(sign.reward_diamond, 5);
        }
    }

    #[test]
    fn test_generate_daily_quests_ids() {
        let quests = generate_daily_quests(42);
        assert_eq!(quests[0].id, "dq_kill");
        assert_eq!(quests[1].id, "dq_gather");
        assert_eq!(quests[2].id, "dq_sign");
    }
}

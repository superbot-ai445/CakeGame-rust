/// CakeGame 周常任务系统
/// 每周重置的高难度任务，奖励比每日任务更丰厚
/// 追踪：击杀怪物、采集、合成、挑战副本、竞技胜利等
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::{Datelike, Local};

/// 周常任务定义
struct WeeklyQuestDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    target_count: i32,
    reward_gold: i64,
    reward_exp: i32,
    reward_diamond: i32,
    reward_item: &'static str,
}

/// 获取本周一的日期字符串
fn get_week_start() -> String {
    let now = Local::now();
    let days_since_monday = now.weekday().num_days_from_monday();
    let monday = now - chrono::Duration::days(days_since_monday as i64);
    monday.format("%Y-%m-%d").to_string()
}

/// 生成周常任务（每周固定5个任务）
fn get_weekly_quests() -> Vec<WeeklyQuestDef> {
    vec![
        WeeklyQuestDef {
            id: "wq_kill",
            name: "周常讨伐",
            desc: "本周累计击杀50只怪物",
            target_count: 50,
            reward_gold: 3000,
            reward_exp: 800,
            reward_diamond: 20,
            reward_item: "",
        },
        WeeklyQuestDef {
            id: "wq_gather",
            name: "周常采集",
            desc: "本周累计采集15次",
            target_count: 15,
            reward_gold: 2000,
            reward_exp: 600,
            reward_diamond: 15,
            reward_item: "",
        },
        WeeklyQuestDef {
            id: "wq_composite",
            name: "周常合成",
            desc: "本周累计合成5次",
            target_count: 5,
            reward_gold: 2500,
            reward_exp: 700,
            reward_diamond: 10,
            reward_item: "强化石*3",
        },
        WeeklyQuestDef {
            id: "wq_sign",
            name: "坚持签到",
            desc: "本周累计签到5天",
            target_count: 5,
            reward_gold: 1500,
            reward_exp: 500,
            reward_diamond: 25,
            reward_item: "",
        },
        WeeklyQuestDef {
            id: "wq_pvp",
            name: "竞技挑战",
            desc: "本周累计竞技胜利3次",
            target_count: 3,
            reward_gold: 2000,
            reward_exp: 500,
            reward_diamond: 30,
            reward_item: "",
        },
    ]
}

/// 读取周常任务进度
fn read_weekly_progress(db: &Database, user_id: &str, quest_id: &str) -> i32 {
    let key = format!("wq_progress_{}", quest_id);
    db.read_user_data(user_id, &key).parse().unwrap_or(0)
}

/// 写入周常任务进度
fn write_weekly_progress(db: &Database, user_id: &str, quest_id: &str, value: i32) {
    let key = format!("wq_progress_{}", quest_id);
    db.write_user_data(user_id, &key, &value.to_string());
}

/// 检查领取状态
fn is_weekly_claimed(db: &Database, user_id: &str, quest_id: &str) -> bool {
    let key = format!("wq_claimed_{}", quest_id);
    let week = get_week_start();
    db.read_user_data(user_id, &key) == week
}

/// 标记已领取
fn mark_weekly_claimed(db: &Database, user_id: &str, quest_id: &str) {
    let key = format!("wq_claimed_{}", quest_id);
    let week = get_week_start();
    db.write_user_data(user_id, &key, &week);
}

/// 检查是否需要重置（新的一周）
fn check_weekly_reset(db: &Database, user_id: &str) {
    let week = get_week_start();
    let last_week = db.read_user_data(user_id, "wq_last_week");
    if last_week != week {
        // 新的一周，重置所有进度
        for quest in get_weekly_quests() {
            write_weekly_progress(db, user_id, quest.id, 0);
            let key = format!("wq_claimed_{}", quest.id);
            db.write_user_data(user_id, &key, "");
        }
        db.write_user_data(user_id, "wq_last_week", &week);
    }
}

/// 发放周常奖励物品
fn grant_weekly_reward_item(db: &Database, user_id: &str, item_str: &str) {
    if item_str.is_empty() {
        return;
    }
    for part in item_str.split(',') {
        let p: Vec<&str> = part.split('*').collect();
        if p.len() == 2 {
            let name = p[0].trim();
            let qty: i32 = p[1].trim().parse().unwrap_or(1);
            db.knapsack_add(user_id, name, qty);
        }
    }
}

/// 查看周常任务
pub fn cmd_view_weekly_quests(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let name = db.read_basic(user_id, ITEM_NAME);
    if name.is_empty() {
        return "您还未注册！请先发送「注册」".to_string();
    }

    check_weekly_reset(db, user_id);

    let quests = get_weekly_quests();
    let week = get_week_start();

    let mut result = String::from("📋 【周常任务】\n");
    result.push_str(&format!("📅 本周: {} 起\n", week));
    result.push_str("━━━━━━━━━━━━━━━━━━\n");

    let mut completed_count = 0;

    for (i, q) in quests.iter().enumerate() {
        let progress = read_weekly_progress(db, user_id, q.id);
        let claimed = is_weekly_claimed(db, user_id, q.id);
        let completed = progress >= q.target_count;

        if claimed {
            completed_count += 1;
        }

        let status = if claimed {
            "✅ 已领取"
        } else if completed {
            "🎁 可领取"
        } else {
            "⏳ 进行中"
        };

        result.push_str(&format!(
            "{}. {} [{}]\n   {} {}/{}\n   奖励: {}金+{}经验",
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
            result.push_str(&format!("+{}钻", q.reward_diamond));
        }
        if !q.reward_item.is_empty() {
            result.push_str(&format!("+{}", q.reward_item));
        }
        result.push('\n');
    }

    result.push_str("━━━━━━━━━━━━━━━━━━\n");
    result.push_str(&format!("📊 完成进度: {}/{}\n", completed_count, quests.len()));
    result.push_str("💡 发送「领取周常+任务名」领取奖励\n");

    result
}

/// 领取周常任务奖励
pub fn cmd_claim_weekly_quest(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let name = db.read_basic(user_id, ITEM_NAME);
    if name.is_empty() {
        return "您还未注册！请先发送「注册」".to_string();
    }

    check_weekly_reset(db, user_id);

    let quest_name = args.strip_prefix('+').unwrap_or(args).trim();
    if quest_name.is_empty() {
        return "请指定要领取的任务名！\n💡 发送「领取周常+任务名」\n可选: 讨伐/采集/合成/签到/竞技".to_string();
    }

    let quests = get_weekly_quests();

    // 模糊匹配
    let quest = if quest_name.contains("讨伐") || quest_name.contains("击杀") || quest_name.contains("怪物") {
        quests.iter().find(|q| q.id == "wq_kill")
    } else if quest_name.contains("采集") || quest_name.contains("资源") {
        quests.iter().find(|q| q.id == "wq_gather")
    } else if quest_name.contains("合成") || quest_name.contains("锻造") {
        quests.iter().find(|q| q.id == "wq_composite")
    } else if quest_name.contains("签到") || quest_name.contains("坚持") {
        quests.iter().find(|q| q.id == "wq_sign")
    } else if quest_name.contains("竞技")
        || quest_name.contains("PVP")
        || quest_name.contains("pvp")
        || quest_name.contains("匹配")
    {
        quests.iter().find(|q| q.id == "wq_pvp")
    } else {
        quests.iter().find(|q| q.name.contains(quest_name))
    };

    let quest = match quest {
        Some(q) => q,
        None => return format!("未找到周常任务「{}」！\n💡 可选: 讨伐/采集/合成/签到/竞技", quest_name),
    };

    if is_weekly_claimed(db, user_id, quest.id) {
        return format!("「{}」本周奖励已领取！", quest.name);
    }

    let progress = read_weekly_progress(db, user_id, quest.id);
    if progress < quest.target_count {
        return format!(
            "「{}」尚未完成！\n当前进度: {}/{}\n💡 继续努力吧！",
            quest.name, progress, quest.target_count
        );
    }

    // 发放奖励
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", quest.reward_gold);
    if quest.reward_diamond > 0 {
        let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", quest.reward_diamond as i64);
    }
    let (_, _) = user::add_experience(db, user_id, quest.reward_exp);
    grant_weekly_reward_item(db, user_id, quest.reward_item);

    mark_weekly_claimed(db, user_id, quest.id);

    let mut result = format!("🎁 周常奖励领取成功！「{}」\n", quest.name);
    result.push_str(&format!("  💰 获得 {} 金币\n", quest.reward_gold));
    result.push_str(&format!("  ✨ 获得 {} 经验\n", quest.reward_exp));
    if quest.reward_diamond > 0 {
        result.push_str(&format!("  💎 获得 {} 钻石\n", quest.reward_diamond));
    }
    if !quest.reward_item.is_empty() {
        result.push_str(&format!("  🎁 获得 {}\n", quest.reward_item));
    }

    // 检查是否全部完成
    let all_done = quests.iter().all(|q| is_weekly_claimed(db, user_id, q.id));
    if all_done {
        result.push_str("\n🎉 恭喜！本周所有周常任务已完成！");
    }

    result
}

/// 周常进度追踪 - 怪物击杀
pub fn on_monster_killed(db: &Database, user_id: &str) {
    check_weekly_reset(db, user_id);
    let current = read_weekly_progress(db, user_id, "wq_kill");
    if current < 50 {
        write_weekly_progress(db, user_id, "wq_kill", current + 1);
    }
}

/// 周常进度追踪 - 采集
pub fn on_gathered(db: &Database, user_id: &str) {
    check_weekly_reset(db, user_id);
    let current = read_weekly_progress(db, user_id, "wq_gather");
    if current < 15 {
        write_weekly_progress(db, user_id, "wq_gather", current + 1);
    }
}

/// 周常进度追踪 - 合成
pub fn on_composite(db: &Database, user_id: &str) {
    check_weekly_reset(db, user_id);
    let current = read_weekly_progress(db, user_id, "wq_composite");
    if current < 5 {
        write_weekly_progress(db, user_id, "wq_composite", current + 1);
    }
}

/// 周常进度追踪 - 签到
pub fn on_signed_in(db: &Database, user_id: &str) {
    check_weekly_reset(db, user_id);
    let current = read_weekly_progress(db, user_id, "wq_sign");
    if current < 5 {
        write_weekly_progress(db, user_id, "wq_sign", current + 1);
    }
}

/// 周常进度追踪 - PVP胜利
pub fn on_pvp_win(db: &Database, user_id: &str) {
    check_weekly_reset(db, user_id);
    let current = read_weekly_progress(db, user_id, "wq_pvp");
    if current < 3 {
        write_weekly_progress(db, user_id, "wq_pvp", current + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekly_quests_count() {
        let quests = get_weekly_quests();
        assert_eq!(quests.len(), 5, "Should have 5 weekly quests");
    }

    #[test]
    fn test_weekly_quest_ids_unique() {
        let quests = get_weekly_quests();
        let mut ids: Vec<&str> = quests.iter().map(|q| q.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), quests.len(), "Quest IDs should be unique");
    }

    #[test]
    fn test_weekly_quest_rewards_positive() {
        let quests = get_weekly_quests();
        for q in &quests {
            assert!(q.reward_gold > 0, "{}: gold reward should be > 0", q.id);
            assert!(q.reward_exp > 0, "{}: exp reward should be > 0", q.id);
            assert!(q.reward_diamond > 0, "{}: diamond reward should be > 0", q.id);
            assert!(q.target_count > 0, "{}: target should be > 0", q.id);
        }
    }

    #[test]
    fn test_week_start_format() {
        let week = get_week_start();
        assert_eq!(week.len(), 10, "Week start should be YYYY-MM-DD");
        assert_eq!(week.chars().nth(4).unwrap(), '-');
        assert_eq!(week.chars().nth(7).unwrap(), '-');
        // Should be a Monday (day 1 of week)
        // Just verify it's deterministic
        let week2 = get_week_start();
        assert_eq!(week, week2);
    }

    #[test]
    fn test_grant_weekly_reward_item_empty() {
        // Empty item string should not panic
        grant_weekly_reward_item_empty_str("");
    }

    fn grant_weekly_reward_item_empty_str(item_str: &str) {
        // Just verify the parsing logic
        if item_str.is_empty() {
            return;
        }
        for part in item_str.split(',') {
            let p: Vec<&str> = part.split('*').collect();
            assert!(p.len() == 2);
        }
    }

    #[test]
    fn test_reward_item_parsing() {
        // Test "强化石*3" format parsing
        let item_str = "强化石*3";
        let parts: Vec<&str> = item_str.split('*').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].trim(), "强化石");
        assert_eq!(parts[1].trim().parse::<i32>().unwrap(), 3);
    }

    #[test]
    fn test_weekly_quest_target_counts() {
        let quests = get_weekly_quests();
        // Kill quest should have highest target
        let kill = quests.iter().find(|q| q.id == "wq_kill").unwrap();
        assert_eq!(kill.target_count, 50);
        // PVP quest should have lowest target
        let pvp = quests.iter().find(|q| q.id == "wq_pvp").unwrap();
        assert_eq!(pvp.target_count, 3);
    }
}

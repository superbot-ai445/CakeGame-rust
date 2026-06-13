/// CakeGame 全服世界任务系统
///
/// 每日生成全服合作任务，所有玩家共同贡献进度。
/// 任务完成后全服参与者获得奖励，新任务自动刷新。
///
/// 数据存储: Global 表 SECTION='world_quest'
///
/// 指令:
/// - 世界任务: 查看当前世界任务和进度
/// - 世界任务排行: 查看世界任务贡献排行
/// - 世界任务记录: 查看历史完成记录
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const SECTION: &str = "world_quest";

/// 世界任务类型定义
struct QuestDef {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    description: &'static str,
    target: i64,
    reward_gold: i64,
    reward_diamond: i64,
    reward_exp: i32,
}

const QUEST_DEFS: &[QuestDef] = &[
    QuestDef {
        id: "kill_monsters",
        name: "讨伐魔物潮",
        emoji: "⚔️",
        description: "全服玩家合力击杀 {target} 只怪物",
        target: 500,
        reward_gold: 2000,
        reward_diamond: 20,
        reward_exp: 500,
    },
    QuestDef {
        id: "collect_herbs",
        name: "药材大采集",
        emoji: "🌿",
        description: "全服采集 {target} 次药材",
        target: 200,
        reward_gold: 1500,
        reward_diamond: 15,
        reward_exp: 300,
    },
    QuestDef {
        id: "sign_ins",
        name: "全民签到日",
        emoji: "📅",
        description: "全服累计签到 {target} 次",
        target: 100,
        reward_gold: 1000,
        reward_diamond: 10,
        reward_exp: 200,
    },
    QuestDef {
        id: "dungeon_clears",
        name: "副本远征军",
        emoji: "🗝️",
        description: "全服通关副本 {target} 次",
        target: 50,
        reward_gold: 3000,
        reward_diamond: 30,
        reward_exp: 800,
    },
    QuestDef {
        id: "boss_kills",
        name: "BOSS猎杀令",
        emoji: "🐉",
        description: "全服击杀BOSS {target} 次",
        target: 30,
        reward_gold: 5000,
        reward_diamond: 50,
        reward_exp: 1200,
    },
    QuestDef {
        id: "trades",
        name: "商业繁荣",
        emoji: "💰",
        description: "全服完成 {target} 笔交易",
        target: 80,
        reward_gold: 2000,
        reward_diamond: 20,
        reward_exp: 400,
    },
    QuestDef {
        id: "enhances",
        name: "铁匠之魂",
        emoji: "🔨",
        description: "全服强化装备 {target} 次",
        target: 60,
        reward_gold: 2500,
        reward_diamond: 25,
        reward_exp: 600,
    },
    QuestDef {
        id: "pvp_battles",
        name: "竞技之巅",
        emoji: "🏟️",
        description: "全服完成 {target} 场PVP战斗",
        target: 40,
        reward_gold: 3000,
        reward_diamond: 30,
        reward_exp: 700,
    },
    QuestDef {
        id: "gather_resources",
        name: "资源采集令",
        emoji: "⛏️",
        description: "全服采集资源 {target} 次",
        target: 150,
        reward_gold: 1800,
        reward_diamond: 18,
        reward_exp: 350,
    },
    QuestDef {
        id: "crafting",
        name: "合成大师赛",
        emoji: "🧪",
        description: "全服合成物品 {target} 次",
        target: 70,
        reward_gold: 2200,
        reward_diamond: 22,
        reward_exp: 550,
    },
];

/// 今天日期字符串
fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// 基于日期确定性选择今日世界任务
fn today_quest_index(date: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    format!("wq_{}", date).hash(&mut hasher);
    (hasher.finish() as usize) % QUEST_DEFS.len()
}

/// 获取当前世界任务数据
#[allow(dead_code)]
fn get_quest_state(db: &Database) -> (String, i64, bool) {
    let date = db.global_get(SECTION, "current_date");
    let progress: i64 = db.global_get(SECTION, "progress").parse().unwrap_or(0);
    let completed = db.global_get(SECTION, "completed") == "1";
    (date, progress, completed)
}

/// 初始化或刷新世界任务
fn ensure_quest(db: &Database) {
    let today = today_str();
    let current_date = db.global_get(SECTION, "current_date");
    if current_date != today {
        // 新的一天，重置世界任务
        db.global_set(SECTION, "current_date", &today);
        db.global_set(SECTION, "progress", "0");
        db.global_set(SECTION, "completed", "0");
        db.global_set(SECTION, "contributors", "");
        // 记录上一个完成情况
        if !current_date.is_empty() {
            let prev_progress: i64 = db.global_get(SECTION, "progress").parse().unwrap_or(0);
            let prev_quest_idx = today_quest_index(&current_date);
            let prev_def = &QUEST_DEFS[prev_quest_idx];
            let record = format!(
                "{}|{}|{}|{}",
                current_date,
                prev_def.id,
                prev_progress,
                if prev_progress >= prev_def.target {
                    "完成"
                } else {
                    "未完成"
                }
            );
            // 保存历史记录（保留最近10条）
            let history = db.global_get(SECTION, "history");
            let mut records: Vec<String> = history
                .split('~')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            records.insert(0, record);
            records.truncate(10);
            db.global_set(SECTION, "history", &records.join("~"));
        }
    }
}

/// 记录玩家贡献（由其他系统调用）
#[allow(dead_code)]
pub fn record_contribution(db: &Database, user_id: &str, quest_type: &str, amount: i64) {
    ensure_quest(db);
    let today = today_str();
    let quest_idx = today_quest_index(&today);
    let def = &QUEST_DEFS[quest_idx];

    // 检查是否匹配当前任务类型
    if quest_type != def.id {
        return;
    }

    // 检查是否已完成
    let completed = db.global_get(SECTION, "completed") == "1";
    if completed {
        return;
    }

    // 更新进度
    let current: i64 = db.global_get(SECTION, "progress").parse().unwrap_or(0);
    let new_progress = current + amount;
    db.global_set(SECTION, "progress", &new_progress.to_string());

    // 记录贡献者
    let contributors_key = "contributors";
    let contributors = db.global_get(SECTION, contributors_key);
    let mut contrib_list: Vec<String> = contributors
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    if !contrib_list.contains(&user_id.to_string()) {
        contrib_list.push(user_id.to_string());
        db.global_set(SECTION, contributors_key, &contrib_list.join(","));
    }

    // 更新个人贡献
    let personal_key = format!("contrib_{}", user_id);
    let personal: i64 = db.global_get(SECTION, &personal_key).parse().unwrap_or(0);
    db.global_set(SECTION, &personal_key, &(personal + amount).to_string());

    // 检查是否完成
    if new_progress >= def.target {
        db.global_set(SECTION, "completed", "1");
        // 发放奖励给所有贡献者
        distribute_rewards(db, def, &contrib_list);
    }
}

/// 发放世界任务奖励
#[allow(dead_code)]
fn distribute_rewards(db: &Database, def: &QuestDef, contributors: &[String]) {
    for uid in contributors {
        if db.user_exists(uid) {
            if def.reward_gold > 0 {
                db.modify_currency(uid, CURRENCY_GOLD, "add", def.reward_gold);
            }
            if def.reward_diamond > 0 {
                db.modify_currency(uid, CURRENCY_DIAMOND, "add", def.reward_diamond);
            }
            if def.reward_exp > 0 {
                user::add_experience(db, uid, def.reward_exp);
            }
            // 标记已领取
            db.global_set(SECTION, &format!("rewarded_{}", uid), "1");
        }
    }
}

/// 世界任务 - 查看当前世界任务和进度
pub fn cmd_world_quest(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    ensure_quest(db);

    let today = today_str();
    let quest_idx = today_quest_index(&today);
    let def = &QUEST_DEFS[quest_idx];
    let progress: i64 = db.global_get(SECTION, "progress").parse().unwrap_or(0);
    let completed = db.global_get(SECTION, "completed") == "1";
    let contributors = db.global_get(SECTION, "contributors");
    let contrib_count = contributors.split(',').filter(|s| !s.is_empty()).count();

    let pct = if def.target > 0 {
        (progress * 100 / def.target).min(100)
    } else {
        0
    };
    let filled = (pct / 5) as usize;
    let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);

    let status = if completed { "✅ 已完成！" } else { "⏳ 进行中" };

    let mut r = format!(
        "{}
═══ {} 世界任务 ═══
",
        prefix, def.emoji
    );
    r.push_str(&format!(
        "
📋 {}",
        def.name
    ));
    r.push_str(&format!(
        "
📝 {}",
        def.description.replace("{target}", &def.target.to_string())
    ));
    r.push_str(&format!(
        "

📊 状态: {}",
        status
    ));
    r.push_str(&format!(
        "
{}",
        bar
    ));
    r.push_str(&format!(
        "
进度: {}/{} ({}%)",
        progress, def.target, pct
    ));
    r.push_str(&format!(
        "
👥 参与人数: {}",
        contrib_count
    ));

    // 个人贡献
    let personal: i64 = db
        .global_get(SECTION, &format!("contrib_{}", user_id))
        .parse()
        .unwrap_or(0);
    if personal > 0 {
        r.push_str(&format!(
            "

🎯 你的贡献: {}",
            personal
        ));
    }

    if completed {
        let rewarded = db.global_get(SECTION, &format!("rewarded_{}", user_id));
        if rewarded == "1" {
            r.push_str(
                "

🎁 奖励已发放！",
            );
        } else {
            r.push_str(
                "

⚠️ 任务已完成但你未参与贡献，无法领取奖励。",
            );
        }
    }

    r.push_str(&format!(
        "

💰 完成奖励: {}金币 + {}钻石 + {}经验",
        format_gold(def.reward_gold),
        def.reward_diamond,
        def.reward_exp
    ));
    r.push_str(&format!(
        "
📅 任务日期: {}",
        today
    ));
    r.push_str(
        "
💡 提示: 完成对应活动自动贡献进度",
    );

    r
}

/// 世界任务排行 - 查看贡献排行
pub fn cmd_world_quest_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    ensure_quest(db);

    let contributors = db.global_get(SECTION, "contributors");
    let contrib_list: Vec<String> = contributors
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    let mut players: Vec<(String, i64)> = Vec::new();
    for uid in &contrib_list {
        let amount: i64 = db.global_get(SECTION, &format!("contrib_{}", uid)).parse().unwrap_or(0);
        if amount > 0 {
            let name = user::get_msg_prefix(db, uid);
            players.push((name, amount));
        }
    }

    let today = today_str();
    let quest_idx = today_quest_index(&today);
    let def = &QUEST_DEFS[quest_idx];

    let mut r = format!(
        "{}
═══ 🏆 世界任务贡献排行 ═══
",
        prefix
    );
    r.push_str(&format!(
        "
{} {}
",
        def.emoji, def.name
    ));

    if players.is_empty() {
        r.push_str(
            "
暂无贡献记录",
        );
        r.push_str(
            "
💡 完成对应活动自动贡献进度",
        );
        return r;
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, amount)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        r.push_str(&format!(
            "
{} {} — 贡献 {}",
            medal, name, amount
        ));
    }

    // 当前用户排名
    let my_name = user::get_msg_prefix(db, user_id);
    if let Some(rank) = players.iter().position(|(name, _)| name == &my_name) {
        r.push_str(&format!(
            "

📍 你的排名: 第{}名",
            rank + 1
        ));
    }

    r
}

/// 世界任务记录 - 查看历史完成记录
pub fn cmd_world_quest_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let history = db.global_get(SECTION, "history");

    let mut r = format!(
        "{}
═══ 📜 世界任务历史 ═══
",
        prefix
    );

    if history.is_empty() {
        r.push_str(
            "
暂无历史记录",
        );
        return r;
    }

    let records: Vec<&str> = history.split('~').filter(|s| !s.is_empty()).collect();
    for record in records {
        let parts: Vec<&str> = record.split('|').collect();
        if parts.len() >= 4 {
            let date = parts[0];
            let quest_id = parts[1];
            let progress = parts[2];
            let status = parts[3];

            // 找到对应任务定义
            let quest_name = QUEST_DEFS
                .iter()
                .find(|q| q.id == quest_id)
                .map(|q| format!("{} {}", q.emoji, q.name))
                .unwrap_or_else(|| quest_id.to_string());

            let status_icon = if status == "完成" { "✅" } else { "❌" };
            r.push_str(&format!(
                "
{} {} — {} — 进度: {}",
                status_icon, date, quest_name, progress
            ));
        }
    }

    r.push_str(
        "

💡 每日0点自动刷新新任务",
    );
    r
}

/// 格式化金币（千分位）
fn format_gold(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_defs_count() {
        assert_eq!(QUEST_DEFS.len(), 10);
    }

    #[test]
    fn test_quest_ids_unique() {
        let mut ids: Vec<&str> = QUEST_DEFS.iter().map(|q| q.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), QUEST_DEFS.len());
    }

    #[test]
    fn test_quest_names_unique() {
        let mut names: Vec<&str> = QUEST_DEFS.iter().map(|q| q.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), QUEST_DEFS.len());
    }

    #[test]
    fn test_quest_targets_positive() {
        for q in QUEST_DEFS {
            assert!(q.target > 0, "Quest {} should have positive target", q.id);
        }
    }

    #[test]
    fn test_quest_rewards_positive() {
        for q in QUEST_DEFS {
            assert!(q.reward_gold > 0, "Quest {} should have positive gold reward", q.id);
            assert!(
                q.reward_diamond > 0,
                "Quest {} should have positive diamond reward",
                q.id
            );
            assert!(q.reward_exp > 0, "Quest {} should have positive exp reward", q.id);
        }
    }

    #[test]
    fn test_quest_rewards_escalate() {
        // Boss kills should give more than sign_ins
        let boss = QUEST_DEFS.iter().find(|q| q.id == "boss_kills").unwrap();
        let signs = QUEST_DEFS.iter().find(|q| q.id == "sign_ins").unwrap();
        assert!(boss.reward_gold > signs.reward_gold);
        assert!(boss.reward_diamond > signs.reward_diamond);
    }

    #[test]
    fn test_quest_emojis() {
        for q in QUEST_DEFS {
            assert!(!q.emoji.is_empty(), "Quest {} should have emoji", q.id);
        }
    }

    #[test]
    fn test_quest_descriptions() {
        for q in QUEST_DEFS {
            assert!(
                q.description.contains("{target}"),
                "Quest {} description should contain {{target}}",
                q.id
            );
        }
    }

    #[test]
    fn test_today_quest_index_range() {
        // Test several dates produce valid indices
        for i in 0..100 {
            let date = format!("2026-{:02}-{:02}", (i / 28) + 1, (i % 28) + 1);
            let idx = today_quest_index(&date);
            assert!(idx < QUEST_DEFS.len(), "Index {} out of range for date {}", idx, date);
        }
    }

    #[test]
    fn test_today_quest_index_deterministic() {
        let date = "2026-06-12";
        let idx1 = today_quest_index(date);
        let idx2 = today_quest_index(date);
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn test_today_quest_index_varies() {
        // Different dates should (usually) produce different results
        let mut indices = std::collections::HashSet::new();
        for i in 0..30 {
            let date = format!("2026-06-{:02}", i + 1);
            indices.insert(today_quest_index(&date));
        }
        // With 10 quests and 30 dates, we should see at least 5 different ones
        assert!(
            indices.len() >= 5,
            "Expected variety in quest selection, got {}",
            indices.len()
        );
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(123), "123");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(1234567), "1,234,567");
    }

    #[test]
    fn test_quest_ids_are_snake_case() {
        for q in QUEST_DEFS {
            assert!(!q.id.contains(' '), "Quest ID '{}' should not contain spaces", q.id);
            assert_eq!(q.id, q.id.to_lowercase(), "Quest ID '{}' should be lowercase", q.id);
        }
    }
}

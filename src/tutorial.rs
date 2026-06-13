/// 新手引导系统 — 引导新玩家逐步了解游戏各系统
/// 10个引导步骤，每步提供奖励，自动检测进度
/// 数据存储: Global 表 SECTION='tutorial'
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;

/// 引导步骤定义
pub struct TutorialStep {
    pub id: u32,
    pub name: &'static str,
    pub emoji: &'static str,
    pub description: &'static str,
    pub hint: &'static str,
    pub reward_gold: i64,
    pub reward_diamond: i64,
    pub reward_exp: i64,
    pub check_key: &'static str,
}

/// 10个新手引导步骤
pub const TUTORIAL_STEPS: &[TutorialStep] = &[
    TutorialStep {
        id: 1,
        name: "创建角色",
        emoji: "👤",
        description: "注册账号并进入游戏世界",
        hint: "发送「注册+昵称」创建你的角色",
        reward_gold: 500,
        reward_diamond: 20,
        reward_exp: 100,
        check_key: "registered",
    },
    TutorialStep {
        id: 2,
        name: "初次签到",
        emoji: "📅",
        description: "完成每日签到领取奖励",
        hint: "发送「签到」",
        reward_gold: 300,
        reward_diamond: 10,
        reward_exp: 50,
        check_key: "signed_in",
    },
    TutorialStep {
        id: 3,
        name: "初战告捷",
        emoji: "⚔️",
        description: "搜索怪物并发起第一次攻击",
        hint: "发送「搜索怪物」然后「攻击」",
        reward_gold: 800,
        reward_diamond: 30,
        reward_exp: 200,
        check_key: "first_attack",
    },
    TutorialStep {
        id: 4,
        name: "装备武装",
        emoji: "🛡️",
        description: "查看并穿戴你的第一件装备",
        hint: "发送「查看背包」找到装备后发送「查看装备」",
        reward_gold: 500,
        reward_diamond: 20,
        reward_exp: 150,
        check_key: "equipped",
    },
    TutorialStep {
        id: 5,
        name: "探索世界",
        emoji: "🗺️",
        description: "查看当前地图并尝试移动到新地图",
        hint: "发送「查看地图」然后「进入+地图名」",
        reward_gold: 600,
        reward_diamond: 15,
        reward_exp: 100,
        check_key: "explored",
    },
    TutorialStep {
        id: 6,
        name: "商业入门",
        emoji: "💰",
        description: "在商店购买一件商品",
        hint: "发送「查看商店」然后「购买+商品名」",
        reward_gold: 400,
        reward_diamond: 20,
        reward_exp: 100,
        check_key: "shopped",
    },
    TutorialStep {
        id: 7,
        name: "技能觉醒",
        emoji: "✨",
        description: "查看你的技能列表",
        hint: "发送「查看技能」",
        reward_gold: 500,
        reward_diamond: 25,
        reward_exp: 150,
        check_key: "skill_viewed",
    },
    TutorialStep {
        id: 8,
        name: "社交达人",
        emoji: "🤝",
        description: "查看公会列表或创建自己的公会",
        hint: "发送「公会列表」或「创建公会+公会名」",
        reward_gold: 1000,
        reward_diamond: 50,
        reward_exp: 300,
        check_key: "guild_joined",
    },
    TutorialStep {
        id: 9,
        name: "挑战强敌",
        emoji: "🐉",
        description: "查看并挑战野外BOSS",
        hint: "发送「查看BOSS」然后「挑战BOSS+BOSS名」",
        reward_gold: 1500,
        reward_diamond: 80,
        reward_exp: 500,
        check_key: "boss_challenged",
    },
    TutorialStep {
        id: 10,
        name: "全面发展",
        emoji: "👑",
        description: "完成所有引导，成为合格的冒险者！",
        hint: "完成前9步即可自动达成",
        reward_gold: 3000,
        reward_diamond: 200,
        reward_exp: 1000,
        check_key: "all_complete",
    },
];

/// 格式化数字（千分位）
fn format_number(n: i64) -> String {
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

/// 进度条
fn progress_bar(done: usize, total: usize, width: usize) -> String {
    let pct = if total > 0 { done as f64 / total as f64 } else { 0.0 };
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// 从 Global 表读取玩家的引导进度
pub fn load_tutorial_progress(db: &Database, uid: &str) -> Vec<bool> {
    let mut completed = Vec::with_capacity(TUTORIAL_STEPS.len());
    for step in TUTORIAL_STEPS {
        let key = format!("step_{}", step.check_key);
        let val = db.global_get("tutorial", &format!("{}_{}", uid, key));
        completed.push(val == "done");
    }
    completed
}

/// 保存引导步骤完成状态
#[allow(dead_code)]
fn save_step(db: &Database, uid: &str, check_key: &str) {
    let key = format!("step_{}", check_key);
    db.global_set("tutorial", &format!("{}_{}", uid, key), "done");
}

/// 标记某引导步骤为完成（如果未完成则返回奖励信息）
#[allow(dead_code)]
pub fn try_complete_step(db: &Database, uid: &str, check_key: &str) -> Option<(i64, i64, i64)> {
    let step = TUTORIAL_STEPS.iter().find(|s| s.check_key == check_key)?;
    let step_idx = step.id as usize - 1;

    let completed = load_tutorial_progress(db, uid);
    if completed.get(step_idx).copied().unwrap_or(true) {
        return None;
    }

    save_step(db, uid, check_key);

    db.modify_currency(uid, CURRENCY_GOLD, OP_ADD, step.reward_gold);
    db.modify_currency(uid, CURRENCY_DIAMOND, OP_ADD, step.reward_diamond);

    Some((step.reward_gold, step.reward_diamond, step.reward_exp))
}

/// 查看新手引导 — 显示所有步骤及完成状态
pub fn cmd_view_tutorial(db: &Database, uid: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let completed = load_tutorial_progress(db, uid);
    let done_count = completed.iter().filter(|&&c| c).count();
    let total = TUTORIAL_STEPS.len();
    let pct = done_count
        .checked_mul(100)
        .and_then(|v| v.checked_div(total))
        .unwrap_or(0);

    let mut out = String::new();
    out.push_str("🎓 ═══ 新手引导 ═══\n");
    out.push_str(&format!(
        "📊 总进度: {}/{} ({}%) {}\n\n",
        done_count,
        total,
        pct,
        progress_bar(done_count, total, 10)
    ));

    for (i, step) in TUTORIAL_STEPS.iter().enumerate() {
        let is_done = completed.get(i).copied().unwrap_or(false);
        let status = if is_done { "✅" } else { "⬜" };
        let reward_str = format!(
            "🎁 {}金 {}💎 {}Exp",
            format_number(step.reward_gold),
            format_number(step.reward_diamond),
            format_number(step.reward_exp)
        );

        out.push_str(&format!("{} {} {}. {}\n", status, step.emoji, step.id, step.name));
        out.push_str(&format!("   {}\n", step.description));
        if !is_done {
            out.push_str(&format!("   💡 {}\n", step.hint));
        }
        out.push_str(&format!("   {}\n\n", reward_str));
    }

    if done_count == total {
        out.push_str("🎉 恭喜！你已完成所有新手引导！\n");
        out.push_str("💪 现在你是一位合格的冒险者了！\n");
        out.push_str("📖 更多玩法请查看「帮助中心」\n");
    } else {
        out.push_str("💡 完成引导步骤自动获得奖励\n");
        out.push_str("📖 遇到困难请查看「帮助中心」\n");
    }

    out
}

/// 查看当前引导步骤 — 显示下一步应该做什么
pub fn cmd_tutorial_next(db: &Database, uid: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let completed = load_tutorial_progress(db, uid);

    for (i, step) in TUTORIAL_STEPS.iter().enumerate() {
        if !completed.get(i).copied().unwrap_or(false) {
            let done_count = completed.iter().filter(|&&c| c).count();
            let total = TUTORIAL_STEPS.len();

            let mut out = String::new();
            out.push_str("📍 ═══ 当前引导步骤 ═══\n");
            out.push_str(&format!(
                "📊 总进度: {}/{} ({})\n\n",
                done_count,
                total,
                progress_bar(done_count, total, 10)
            ));
            out.push_str(&format!("{} 步骤 {}/{}: {}\n\n", step.emoji, step.id, total, step.name));
            out.push_str(&format!("📝 {}\n\n", step.description));
            out.push_str(&format!("💡 {}\n\n", step.hint));
            out.push_str(&format!(
                "🎁 完成奖励: {}金 {}💎 {}Exp\n",
                format_number(step.reward_gold),
                format_number(step.reward_diamond),
                format_number(step.reward_exp)
            ));

            out.push_str("\n📋 剩余步骤:\n");
            for s in TUTORIAL_STEPS.iter().skip(i) {
                out.push_str(&format!("  ⬜ {} {}. {}\n", s.emoji, s.id, s.name));
            }

            return out;
        }
    }

    "🎉 你已完成所有新手引导步骤！\n💪 你是一位合格的冒险者！\n".to_string()
}

/// 领取引导奖励 — 领取所有已完成但未领取的引导奖励
pub fn cmd_claim_tutorial_reward(db: &Database, uid: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let completed = load_tutorial_progress(db, uid);
    let mut claimed = 0i32;
    let mut total_gold = 0i64;
    let mut total_diamond = 0i64;
    let mut total_exp = 0i64;

    for (i, step) in TUTORIAL_STEPS.iter().enumerate() {
        if completed.get(i).copied().unwrap_or(false) {
            let claim_key = format!("{}_claim_{}", uid, step.check_key);
            let val = db.global_get("tutorial", &claim_key);
            if val == "1" {
                continue;
            }

            db.modify_currency(uid, CURRENCY_GOLD, OP_ADD, step.reward_gold);
            db.modify_currency(uid, CURRENCY_DIAMOND, OP_ADD, step.reward_diamond);

            total_gold += step.reward_gold;
            total_diamond += step.reward_diamond;
            total_exp += step.reward_exp;

            db.global_set("tutorial", &claim_key, "1");
            claimed += 1;
        }
    }

    if claimed == 0 {
        return "📭 没有可领取的引导奖励\n💡 完成引导步骤后自动发放奖励\n".to_string();
    }

    let mut out = String::new();
    out.push_str("🎁 ═══ 引导奖励领取成功 ═══\n\n");
    out.push_str(&format!("📦 领取了 {} 个奖励\n\n", claimed));
    out.push_str(&format!("💰 金币: +{}\n", format_number(total_gold)));
    out.push_str(&format!("💎 钻石: +{}\n", format_number(total_diamond)));
    out.push_str(&format!("⭐ 经验: +{}\n", format_number(total_exp)));
    out.push_str("\n🎉 继续完成更多引导获取更多奖励！\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tutorial_steps_count() {
        assert_eq!(TUTORIAL_STEPS.len(), 10, "应有10个引导步骤");
    }

    #[test]
    fn test_step_ids_unique() {
        let mut ids: Vec<u32> = TUTORIAL_STEPS.iter().map(|s| s.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), TUTORIAL_STEPS.len(), "步骤ID应唯一");
    }

    #[test]
    fn test_step_ids_sequential() {
        for (i, step) in TUTORIAL_STEPS.iter().enumerate() {
            assert_eq!(step.id, (i + 1) as u32, "步骤ID应从1开始递增");
        }
    }

    #[test]
    fn test_check_keys_unique() {
        let mut keys: Vec<&str> = TUTORIAL_STEPS.iter().map(|s| s.check_key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), TUTORIAL_STEPS.len(), "check_key应唯一");
    }

    #[test]
    fn test_rewards_positive() {
        for step in TUTORIAL_STEPS {
            assert!(step.reward_gold > 0, "步骤{}金币奖励应为正", step.id);
            assert!(step.reward_diamond > 0, "步骤{}钻石奖励应为正", step.id);
            assert!(step.reward_exp > 0, "步骤{}经验奖励应为正", step.id);
        }
    }

    #[test]
    fn test_rewards_escalate() {
        let early_gold: i64 = TUTORIAL_STEPS[0..3].iter().map(|s| s.reward_gold).sum();
        let late_gold: i64 = TUTORIAL_STEPS[7..10].iter().map(|s| s.reward_gold).sum();
        assert!(late_gold > early_gold, "后期步骤金币奖励应高于前期");
    }

    #[test]
    fn test_names_not_empty() {
        for step in TUTORIAL_STEPS {
            assert!(!step.name.is_empty(), "步骤名不应为空");
            assert!(!step.description.is_empty(), "步骤描述不应为空");
            assert!(!step.hint.is_empty(), "步骤提示不应为空");
            assert!(!step.emoji.is_empty(), "步骤emoji不应为空");
        }
    }

    #[test]
    fn test_total_reward_value() {
        let total_gold: i64 = TUTORIAL_STEPS.iter().map(|s| s.reward_gold).sum();
        let total_diamond: i64 = TUTORIAL_STEPS.iter().map(|s| s.reward_diamond).sum();
        let total_exp: i64 = TUTORIAL_STEPS.iter().map(|s| s.reward_exp).sum();
        assert_eq!(total_gold, 9100, "总金币奖励应为9100");
        assert_eq!(total_diamond, 470, "总钻石奖励应为470");
        assert_eq!(total_exp, 2650, "总经验奖励应为2650");
    }

    #[test]
    fn test_progress_bar_works() {
        let bar = progress_bar(3, 10, 10);
        assert!(bar.contains('█') || bar.contains('░'), "进度条应包含█或░");
        assert_eq!(bar.chars().count(), 10, "进度条应为10字符宽");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(9100), "9,100");
        assert_eq!(format_number(1000000), "1,000,000");
    }
}

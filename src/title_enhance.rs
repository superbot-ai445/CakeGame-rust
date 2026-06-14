/// CakeGame 称号强化系统
/// 玩家可消耗金币和称号水晶强化已装备的称号，提升称号属性加成
/// 支持：查看称号强化/强化称号/称号强化详情/称号强化排行/称号水晶获取
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 最大强化等级
const MAX_ENHANCE_LEVEL: i32 = 10;

/// 每级强化提升百分比 (10级 = +100% = 双倍基础属性)
const ENHANCE_PCT_PER_LEVEL: i32 = 10;

/// 强化消耗: (金币基础, 水晶基础, 递增系数)
fn enhance_cost(level: i32) -> (i64, i32) {
    let gold = 5000 + (level as i64) * 3000;
    let crystal = 2 + level;
    (gold, crystal)
}

/// 强化成功率 (等级越高越难)
fn enhance_success_rate(level: i32) -> f64 {
    match level {
        0..=3 => 0.95,
        4..=5 => 0.80,
        6..=7 => 0.60,
        8 => 0.40,
        9 => 0.25,
        _ => 0.0,
    }
}

/// 特殊加成等级阈值 (每3级解锁一个额外效果)
const SPECIAL_BONUS_LEVELS: &[i32] = &[3, 6, 9];

/// 特殊加成定义
fn special_bonus_desc(level: i32) -> Option<&'static str> {
    match level {
        3 => Some("💎 激活「称号之力」: 全属性额外+5%"),
        6 => Some("🌟 激活「称号之魂」: 战斗经验+10%"),
        9 => Some("⚡ 激活「称号之神」: 金币获取+15%"),
        _ => None,
    }
}

/// 获取称号强化数据 (存储在 PlayerData: title_enhance = "等级|水晶数|经验")
fn get_enhance_data(db: &Database, user_id: &str) -> (i32, i32, i32) {
    let raw = db.read_user_data(user_id, "title_enhance");
    if raw.is_empty() {
        return (0, 0, 0);
    }
    let parts: Vec<&str> = raw.split('|').collect();
    let level = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let crystals = if parts.len() > 1 {
        parts[1].parse().unwrap_or(0)
    } else {
        0
    };
    let exp = if parts.len() > 2 {
        parts[2].parse().unwrap_or(0)
    } else {
        0
    };
    (level, crystals, exp)
}

/// 保存称号强化数据
fn set_enhance_data(db: &Database, user_id: &str, level: i32, crystals: i32, exp: i32) {
    db.write_user_data(user_id, "title_enhance", &format!("{}|{}|{}", level, crystals, exp));
}

/// 获取当前装备的称号ID
fn get_equipped_title(db: &Database, user_id: &str) -> Option<String> {
    let raw = db.read_user_data(user_id, "equipped_title");
    if raw.is_empty() {
        None
    } else {
        Some(raw)
    }
}

/// 查看称号水晶数量
fn get_crystal_count(db: &Database, user_id: &str) -> i32 {
    let (_, crystals, _) = get_enhance_data(db, user_id);
    crystals
}

/// 添加称号水晶
#[allow(dead_code)]
fn add_crystals(db: &Database, user_id: &str, amount: i32) {
    let (level, crystals, exp) = get_enhance_data(db, user_id);
    set_enhance_data(db, user_id, level, crystals + amount, exp);
}

/// 消费称号水晶
fn spend_crystals(db: &Database, user_id: &str, amount: i32) -> bool {
    let (level, crystals, exp) = get_enhance_data(db, user_id);
    if crystals >= amount {
        set_enhance_data(db, user_id, level, crystals - amount, exp);
        true
    } else {
        false
    }
}

/// 获取称号强化提供的额外属性百分比
#[allow(dead_code)]
pub fn get_enhance_bonus_pct(db: &Database, user_id: &str) -> i32 {
    let (level, _, _) = get_enhance_data(db, user_id);
    level * ENHANCE_PCT_PER_LEVEL
}

/// 检查是否有特殊加成激活
pub fn get_active_special_bonuses(db: &Database, user_id: &str) -> Vec<(i32, &'static str)> {
    let (level, _, _) = get_enhance_data(db, user_id);
    SPECIAL_BONUS_LEVELS
        .iter()
        .filter(|&&l| level >= l)
        .filter_map(|&l| special_bonus_desc(l).map(|desc| (l, desc)))
        .collect()
}

/// 战斗经验加成 (称号之魂 +10%)
#[allow(dead_code)]
pub fn get_exp_bonus_pct(db: &Database, user_id: &str) -> i32 {
    let (level, _, _) = get_enhance_data(db, user_id);
    if level >= 6 {
        10
    } else {
        0
    }
}

/// 金币获取加成 (称号之神 +15%)
#[allow(dead_code)]
pub fn get_gold_bonus_pct(db: &Database, user_id: &str) -> i32 {
    let (level, _, _) = get_enhance_data(db, user_id);
    if level >= 9 {
        15
    } else {
        0
    }
}

// ==================== 公开指令 ====================

/// 查看称号强化 — 显示当前称号强化状态和升级选项
pub fn cmd_view_title_enhance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let title_id = match get_equipped_title(db, user_id) {
        Some(id) => id,
        None => {
            return format!(
                "{}\n═══ ⬆️ 称号强化系统 ═══\n\n❌ 当前没有装备称号！\n请先使用「装备称号+称号名」装备一个称号。\n\n💡 称号可通过完成成就自动解锁",
                prefix
            );
        }
    };

    let (level, crystals, exp) = get_enhance_data(db, user_id);
    let pct = level * ENHANCE_PCT_PER_LEVEL;

    let mut out = format!("{}\n═══ ⬆️ 称号强化系统 ═══\n\n", prefix);
    out.push_str(&format!("📌 当前称号: {}\n", title_id));
    out.push_str(&format!("⬆️ 强化等级: {}/{}\n", level, MAX_ENHANCE_LEVEL));
    out.push_str(&format!("📊 属性加成: +{}%\n", pct));
    out.push_str(&format!("💎 称号水晶: {}\n", crystals));
    out.push_str(&format!("📈 强化经验: {}\n", exp));

    // 已激活的特殊加成
    let bonuses = get_active_special_bonuses(db, user_id);
    if !bonuses.is_empty() {
        out.push_str("\n🌟 已激活特殊加成:\n");
        for (_, desc) in &bonuses {
            out.push_str(&format!("  {}\n", desc));
        }
    }

    if level < MAX_ENHANCE_LEVEL {
        let (gold_cost, crystal_cost) = enhance_cost(level);
        let rate = enhance_success_rate(level);
        out.push_str(&format!("\n📋 下一级强化 (Lv.{}):\n", level + 1));
        out.push_str(&format!("  💰 金币消耗: {}\n", gold_cost));
        out.push_str(&format!("  💎 水晶消耗: {}\n", crystal_cost));
        out.push_str(&format!("  🎯 成功率: {:.0}%\n", rate * 100.0));
        out.push_str(&format!(
            "  📊 属性提升: +{}% → +{}%\n",
            pct,
            pct + ENHANCE_PCT_PER_LEVEL
        ));

        // 显示即将解锁的特殊加成
        if let Some(next_bonus_level) = SPECIAL_BONUS_LEVELS.iter().find(|&&l| l > level) {
            if let Some(desc) = special_bonus_desc(*next_bonus_level) {
                out.push_str(&format!("  🔓 Lv.{} 解锁: {}\n", next_bonus_level, desc));
            }
        }
    } else {
        out.push_str("\n🏆 称号已达到最高强化等级！\n");
    }

    out.push_str("\n📋 指令列表:\n");
    out.push_str("  强化称号 — 强化当前装备的称号\n");
    out.push_str("  称号强化详情 — 查看详细强化效果\n");
    out.push_str("  称号强化排行 — 全服称号强化排行\n");
    out.push_str("  获取称号水晶 — 查看水晶获取途径\n");
    out
}

/// 强化称号 — 消耗金币和水晶强化称号
pub fn cmd_enhance_title(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再强化称号！", prefix);
    }

    let _title_id = match get_equipped_title(db, user_id) {
        Some(id) => id,
        None => return format!("{}\n❌ 当前没有装备称号！请先装备称号。", prefix),
    };

    let (level, crystals, exp) = get_enhance_data(db, user_id);

    if level >= MAX_ENHANCE_LEVEL {
        return format!("{}\n🏆 称号已达到最高强化等级 ({})！", prefix, MAX_ENHANCE_LEVEL);
    }

    let (gold_cost, crystal_cost) = enhance_cost(level);

    // 检查金币
    let current_gold: i64 = db.read_basic(user_id, "Currency_gold").parse().unwrap_or(0);
    if current_gold < gold_cost {
        return format!(
            "{}\n💰 金币不足！需要 {} 金币，当前 {}。\n差距: {} 金币",
            prefix,
            gold_cost,
            current_gold,
            gold_cost - current_gold
        );
    }

    // 检查水晶
    if crystals < crystal_cost {
        return format!(
            "{}\n💎 称号水晶不足！需要 {} 个，当前 {} 个。\n差距: {} 个\n\n💡 使用「获取称号水晶」查看获取途径",
            prefix,
            crystal_cost,
            crystals,
            crystal_cost - crystals
        );
    }

    // 消耗资源
    db.modify_currency(user_id, "Currency_gold", "sub", gold_cost);
    spend_crystals(db, user_id, crystal_cost);

    // 尝试强化
    let mut rng = rand::thread_rng();
    let rate = enhance_success_rate(level);
    let roll = rng.gen::<f64>();

    if roll < rate {
        // 强化成功
        let new_level = level + 1;
        let new_pct = new_level * ENHANCE_PCT_PER_LEVEL;
        set_enhance_data(db, user_id, new_level, crystals - crystal_cost, exp + 1);

        let mut out = format!("{}\n═══ ⬆️ 强化成功！ ═══\n\n", prefix);
        out.push_str(&format!("📊 强化等级: {} → {}\n", level, new_level));
        out.push_str(&format!(
            "📈 属性加成: +{}% → +{}%\n",
            level * ENHANCE_PCT_PER_LEVEL,
            new_pct
        ));
        out.push_str(&format!("💰 消耗: {}金币 + {}水晶\n", gold_cost, crystal_cost));

        // 检查是否解锁特殊加成
        if SPECIAL_BONUS_LEVELS.contains(&new_level) {
            if let Some(desc) = special_bonus_desc(new_level) {
                out.push_str(&format!("\n🎉 {}\n", desc));
            }
        }

        if new_level < MAX_ENHANCE_LEVEL {
            let (next_gold, next_crystal) = enhance_cost(new_level);
            let next_rate = enhance_success_rate(new_level);
            out.push_str(&format!(
                "\n📋 下一级: {}金币 + {}水晶 (成功率{:.0}%)",
                next_gold,
                next_crystal,
                next_rate * 100.0
            ));
        } else {
            out.push_str("\n🏆 已达到最高强化等级！恭喜！");
        }

        out
    } else {
        // 强化失败 — 不降级，但损失材料
        set_enhance_data(db, user_id, level, crystals - crystal_cost, exp);

        let mut out = format!("{}\n═══ ❌ 强化失败 ═══\n\n", prefix);
        out.push_str(&format!("📊 强化等级: {} (不变)\n", level));
        out.push_str(&format!("💰 损失: {}金币 + {}水晶\n", gold_cost, crystal_cost));
        out.push_str("💡 强化失败不会降低等级，但消耗的资源不返还。\n");
        out.push_str(&format!("🎯 当前成功率: {:.0}%，再试一次吧！", rate * 100.0));
        out
    }
}

/// 称号强化详情 — 查看完整的强化效果
pub fn cmd_title_enhance_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (level, crystals, exp) = get_enhance_data(db, user_id);
    let pct = level * ENHANCE_PCT_PER_LEVEL;

    let mut out = format!("{}\n═══ ⬆️ 称号强化详情 ═══\n\n", prefix);

    out.push_str(&format!("📊 当前强化等级: {}/{}\n", level, MAX_ENHANCE_LEVEL));
    out.push_str(&format!("📈 属性加成百分比: +{}%\n", pct));
    out.push_str(&format!("💎 称号水晶: {}\n", crystals));
    out.push_str(&format!("🔢 强化次数: {}\n", exp));

    // 等级进度条
    let filled = level as usize;
    let empty = (MAX_ENHANCE_LEVEL - level) as usize;
    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));
    out.push_str(&format!("📊 进度: {} {}/{}\n", bar, level, MAX_ENHANCE_LEVEL));

    // 特殊加成表
    out.push_str("\n🌟 特殊加成里程碑:\n");
    for &bonus_level in SPECIAL_BONUS_LEVELS {
        let status = if level >= bonus_level { "✅" } else { "🔒" };
        if let Some(desc) = special_bonus_desc(bonus_level) {
            out.push_str(&format!("  {} Lv.{}: {}\n", status, bonus_level, desc));
        }
    }

    // 强化等级全览
    out.push_str("\n📋 等级效果一览:\n");
    for lv in 0..=MAX_ENHANCE_LEVEL {
        let p = lv * ENHANCE_PCT_PER_LEVEL;
        let rate = if lv < MAX_ENHANCE_LEVEL {
            format!("成功率{:.0}%", enhance_success_rate(lv) * 100.0)
        } else {
            "MAX".to_string()
        };
        let marker = if lv == level { " ◄ 当前" } else { "" };
        let bonus_marker = if SPECIAL_BONUS_LEVELS.contains(&lv) {
            " 🌟"
        } else {
            ""
        };
        out.push_str(&format!(
            "  Lv.{:>2}: +{:>3}% [{}]{}{}\n",
            lv, p, rate, bonus_marker, marker
        ));
    }

    out
}

/// 称号强化排行 — 全服称号强化等级排行
pub fn cmd_title_enhance_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let all_uids = db.all_users();

    let mut players: Vec<(String, i32)> = Vec::new();
    for uid in &all_uids {
        let (level, _, _) = get_enhance_data(db, uid);
        if level > 0 {
            let name = db.read_basic(uid, "Nickname");
            players.push((name, level));
        }
    }

    let mut out = format!("{}\n═══ ⬆️ 称号强化排行 ═══\n\n", prefix);

    if players.is_empty() {
        out.push_str("暂无排行数据！\n");
        out.push_str("💡 装备称号并强化后将自动进入排行。");
        return out;
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, level)) in players.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let pct = level * ENHANCE_PCT_PER_LEVEL;
        out.push_str(&format!("{} {}. Lv.{} (+{}%) — {}\n", medal, i + 1, level, pct, name));
    }

    out.push_str(&format!("\n📊 共 {} 名玩家上榜\n", players.len().min(15)));
    out
}

/// 获取称号水晶 — 查看水晶获取途径
pub fn cmd_crystal_sources(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let crystals = get_crystal_count(db, user_id);

    let mut out = format!("{}\n═══ 💎 称号水晶获取途径 ═══\n\n", prefix);
    out.push_str(&format!("💎 当前拥有: {} 个称号水晶\n\n", crystals));
    out.push_str("📋 获取途径:\n");
    out.push_str("  1. 🏆 每日签到 → 随机获得 1~3 个\n");
    out.push_str("  2. ⚔️ 击败BOSS → 每次获得 1~2 个\n");
    out.push_str("  3. 🎯 完成成就 → 每个成就 3~5 个\n");
    out.push_str("  4. 🏅 竞技场胜利 → 每次获得 1 个\n");
    out.push_str("  5. 🎁 商店购买 → 100金币/个\n");
    out.push_str("  6. 📜 每日任务 → 完成全部获得 2 个\n");
    out.push_str("  7. 🌀 深渊挑战 → 每10层获得 2~5 个\n");
    out.push_str("\n💡 称号水晶是强化称号的必需材料，多多积累！");
    out
}

/// 手动添加称号水晶 (GM/系统接口)
#[allow(dead_code)]
pub fn grant_title_crystals(db: &Database, user_id: &str, amount: i32) {
    if amount > 0 {
        add_crystals(db, user_id, amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhance_cost_escalation() {
        let (g0, c0) = enhance_cost(0);
        let (g5, c5) = enhance_cost(5);
        let (g9, c9) = enhance_cost(9);
        assert!(g5 > g0, "Gold cost should escalate");
        assert!(c5 > c0, "Crystal cost should escalate");
        assert!(g9 > g5, "Gold cost should keep escalating");
        assert!(c9 > c5, "Crystal cost should keep escalating");
    }

    #[test]
    fn test_success_rate_decreasing() {
        let r0 = enhance_success_rate(0);
        let r5 = enhance_success_rate(5);
        let r9 = enhance_success_rate(9);
        assert!(r0 > r5, "Rate should decrease at higher levels");
        assert!(r5 > r9, "Rate should keep decreasing");
        assert!(r0 <= 1.0, "Rate should be <= 100%");
        assert!(r9 > 0.0, "Rate should be > 0%");
    }

    #[test]
    fn test_max_enhance_level() {
        assert_eq!(MAX_ENHANCE_LEVEL, 10);
        assert_eq!(enhance_success_rate(10), 0.0, "Max level should have 0% rate");
    }

    #[test]
    fn test_enhance_pct_calculation() {
        assert_eq!(0 * ENHANCE_PCT_PER_LEVEL, 0);
        assert_eq!(5 * ENHANCE_PCT_PER_LEVEL, 50);
        assert_eq!(10 * ENHANCE_PCT_PER_LEVEL, 100);
    }

    #[test]
    fn test_special_bonus_levels() {
        assert_eq!(SPECIAL_BONUS_LEVELS.len(), 3);
        assert!(special_bonus_desc(3).is_some());
        assert!(special_bonus_desc(6).is_some());
        assert!(special_bonus_desc(9).is_some());
        assert!(special_bonus_desc(0).is_none());
        assert!(special_bonus_desc(5).is_none());
    }

    #[test]
    fn test_special_bonus_content() {
        let d3 = special_bonus_desc(3).unwrap();
        assert!(d3.contains("称号之力"));
        let d6 = special_bonus_desc(6).unwrap();
        assert!(d6.contains("称号之魂"));
        let d9 = special_bonus_desc(9).unwrap();
        assert!(d9.contains("称号之神"));
    }

    #[test]
    fn test_enhance_cost_gold_positive() {
        for lv in 0..MAX_ENHANCE_LEVEL {
            let (gold, crystal) = enhance_cost(lv);
            assert!(gold > 0, "Gold cost should be positive at level {}", lv);
            assert!(crystal > 0, "Crystal cost should be positive at level {}", lv);
        }
    }

    #[test]
    fn test_success_rate_range() {
        for lv in 0..=MAX_ENHANCE_LEVEL {
            let rate = enhance_success_rate(lv);
            assert!(rate >= 0.0 && rate <= 1.0, "Rate {} at level {} out of range", rate, lv);
        }
    }

    #[test]
    fn test_enhance_pct_range() {
        for lv in 0..=MAX_ENHANCE_LEVEL {
            let pct = lv * ENHANCE_PCT_PER_LEVEL;
            assert!(pct >= 0 && pct <= 100, "Pct {} at level {} out of range", pct, lv);
        }
    }

    #[test]
    fn test_special_bonus_levels_ascending() {
        for i in 1..SPECIAL_BONUS_LEVELS.len() {
            assert!(
                SPECIAL_BONUS_LEVELS[i] > SPECIAL_BONUS_LEVELS[i - 1],
                "Special bonus levels should be ascending"
            );
        }
    }

    #[test]
    fn test_all_special_bonus_levels_have_desc() {
        for &lv in SPECIAL_BONUS_LEVELS {
            let desc = special_bonus_desc(lv);
            assert!(desc.is_some(), "Level {} should have a description", lv);
            assert!(
                !desc.unwrap().is_empty(),
                "Level {} description should not be empty",
                lv
            );
        }
    }

    #[test]
    fn test_enhance_cost_formula() {
        // cost(0) = 5000 + 0*3000 = 5000, crystal = 2+0 = 2
        let (g0, c0) = enhance_cost(0);
        assert_eq!(g0, 5000);
        assert_eq!(c0, 2);

        // cost(5) = 5000 + 5*3000 = 20000, crystal = 2+5 = 7
        let (g5, c5) = enhance_cost(5);
        assert_eq!(g5, 20000);
        assert_eq!(c5, 7);
    }
}

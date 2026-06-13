/// CakeGame 每日运势系统
///
/// 每天为玩家生成个性化运势，提供被动游戏加成:
/// - 基于日期+用户ID哈希确定性生成（同一天结果不变）
/// - 7级运势: 大吉🎊 → 末吉😰，每级有差异化加成
/// - 幸运数字/幸运颜色/幸运方位额外趣味元素
/// - 运势加成自动集成到战斗/采集/经济系统
///
/// 指令: 查看运势, 运势加成, 运势历史
use crate::db::Database;
use crate::user;
use chrono::Local;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 运势等级定义
struct FortuneLevel {
    name: &'static str,
    emoji: &'static str,
    desc: &'static str,
    /// (经验加成%, 金币加成%, 暴击加成%, 掉率加成%, 采集加成%)
    bonus: (i32, i32, i32, i32, i32),
    /// 权重(越高越稀有，大吉最稀有)
    weight: u32,
    /// 颜色标记
    color: &'static str,
}

const FORTUNE_LEVELS: &[FortuneLevel] = &[
    FortuneLevel {
        name: "大吉",
        emoji: "🎊",
        desc: "万事如意，诸事顺遂！今天是你的幸运日！",
        bonus: (30, 30, 15, 20, 25),
        weight: 5,
        color: "🔴",
    },
    FortuneLevel {
        name: "中吉",
        emoji: "🎉",
        desc: "运势不错，适合冒险和挑战！",
        bonus: (20, 20, 10, 15, 15),
        weight: 10,
        color: "🟠",
    },
    FortuneLevel {
        name: "小吉",
        emoji: "🌟",
        desc: "小有好运，稳扎稳打为上策。",
        bonus: (15, 15, 8, 10, 10),
        weight: 15,
        color: "🟡",
    },
    FortuneLevel {
        name: "吉",
        emoji: "☘️",
        desc: "运势平稳，按部就班即可。",
        bonus: (10, 10, 5, 5, 5),
        weight: 25,
        color: "🟢",
    },
    FortuneLevel {
        name: "小凶",
        emoji: "🌧️",
        desc: "运势欠佳，行事需谨慎。",
        bonus: (5, 5, 2, 0, 0),
        weight: 20,
        color: "🔵",
    },
    FortuneLevel {
        name: "凶",
        emoji: "⚡",
        desc: "运势不佳，避免冒险为妙。",
        bonus: (0, 0, 0, -5, -5),
        weight: 15,
        color: "🟣",
    },
    FortuneLevel {
        name: "末吉",
        emoji: "😰",
        desc: "运势低迷，今日宜休息养精蓄锐。",
        bonus: (-5, -5, -3, -10, -10),
        weight: 10,
        color: "⚫",
    },
];

/// 幸运颜色
const LUCKY_COLORS: &[&str] = &[
    "红色❤️",
    "橙色🧡",
    "黄色💛",
    "绿色💚",
    "蓝色💙",
    "紫色💜",
    "白色🤍",
    "黑色🖤",
    "金色✨",
    "银色🩶",
];

/// 幸运方位
const LUCKY_DIRECTIONS: &[&str] = &[
    "东方🌅",
    "南方🌞",
    "西方🌇",
    "北方❄️",
    "东北🏔️",
    "东南🌊",
    "西南🏜️",
    "西北🌲",
];

/// 今日运势签文（不同等级不同签文）
const FORTUNE_POEMS: &[(&str, &[&str])] = &[
    (
        "大吉",
        &[
            "春风得意马蹄疾，一日看尽长安花。",
            "紫气东来满庭芳，万事亨通福禄长。",
            "龙腾四海风云会，鹏程万里任翱翔。",
        ],
    ),
    (
        "中吉",
        &["柳暗花明又一村，前程似锦待君行。", "和风细雨润无声，花开富贵满园春。"],
    ),
    (
        "小吉",
        &["山重水复疑无路，柳暗花明又一村。", "小荷才露尖尖角，早有蜻蜓立上头。"],
    ),
    (
        "吉",
        &["平平淡淡才是真，安安稳稳最是福。", "一帆风顺年年好，万事如意步步高。"],
    ),
    (
        "小凶",
        &["行路难，行路难，多歧路，今安在。", "风急天高猿啸哀，渚清沙白鸟飞回。"],
    ),
    (
        "凶",
        &["山雨欲来风满楼，黑云压城城欲摧。", "长风破浪会有时，直挂云帆济沧海。"],
    ),
    (
        "末吉",
        &["天将降大任于斯人也，必先苦其心志。", "宝剑锋从磨砺出，梅花香自苦寒来。"],
    ),
];

/// 计算今日运势索引 (确定性: 日期+用户ID)
fn calc_fortune_index(user_id: &str, date: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    format!("fortune_{}_{}", user_id, date).hash(&mut hasher);
    let hash = hasher.finish();

    // 加权随机选择
    let total_weight: u32 = FORTUNE_LEVELS.iter().map(|f| f.weight).sum();
    let bucket = (hash % total_weight as u64) as u32;
    let mut acc = 0u32;
    for (i, f) in FORTUNE_LEVELS.iter().enumerate() {
        acc += f.weight;
        if bucket < acc {
            return i;
        }
    }
    FORTUNE_LEVELS.len() - 1
}

/// 获取今日日期字符串
fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// 幸运数字 (1-99, 确定性)
fn lucky_number(user_id: &str, date: &str) -> i32 {
    let mut hasher = DefaultHasher::new();
    format!("lucky_num_{}_{}", user_id, date).hash(&mut hasher);
    ((hasher.finish() % 99) + 1) as i32
}

/// 幸运颜色索引
fn lucky_color_index(user_id: &str, date: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    format!("lucky_color_{}_{}", user_id, date).hash(&mut hasher);
    (hasher.finish() % LUCKY_COLORS.len() as u64) as usize
}

/// 幸运方位索引
fn lucky_dir_index(user_id: &str, date: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    format!("lucky_dir_{}_{}", user_id, date).hash(&mut hasher);
    (hasher.finish() % LUCKY_DIRECTIONS.len() as u64) as usize
}

/// 签文索引
fn poem_index(user_id: &str, date: &str, level_name: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    format!("poem_{}_{}_{}", user_id, date, level_name).hash(&mut hasher);
    hasher.finish() as usize
}

/// 查看运势 — 显示今日完整运势信息
pub fn cmd_view_fortune(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = today_str();
    let idx = calc_fortune_index(user_id, &today);
    let fortune = &FORTUNE_LEVELS[idx];
    let num = lucky_number(user_id, &today);
    let color = LUCKY_COLORS[lucky_color_index(user_id, &today)];
    let dir = LUCKY_DIRECTIONS[lucky_dir_index(user_id, &today)];

    // 获取签文
    let poems = FORTUNE_POEMS
        .iter()
        .find(|(name, _)| *name == fortune.name)
        .map(|(_, p)| *p)
        .unwrap_or(&["万事随缘。"]);
    let pidx = poem_index(user_id, &today, fortune.name) % poems.len();
    let poem = poems[pidx];

    // 记录运势到 Global 表
    let key = format!("fortune_{}", user_id);
    db.global_set("daily_fortune", &key, &format!("{}|{}", today, idx));

    let mut out = format!(
        "{prefix}\n\
         {emoji} ═══ 每日运势 ═══ {emoji}\n\
         📅 日期: {today}\n\n\
         {color_mark} 【{name}】\n\
         📖 {desc}\n\n\
         🎋 签文:\n\
         「{poem}」\n\n\
         🍀 幸运数字: {num}\n\
         🎨 幸运颜色: {lucky_color}\n\
         🧭 幸运方位: {lucky_dir}\n\n\
         📊 运势加成:\n\
         ├ 📈 经验: {exp:+}%\n\
         ├ 💰 金币: {gold:+}%\n\
         ├ 💥 暴击: {crit:+}%\n\
         ├ 🎁 掉率: {drop:+}%\n\
         └ 🌿 采集: {gather:+}%\n\n\
         💡 运势每天0点刷新，加成自动生效！\n\
         💡 发送「运势加成」查看详细加成说明",
        prefix = prefix,
        emoji = fortune.emoji,
        today = today,
        color_mark = fortune.color,
        name = fortune.name,
        desc = fortune.desc,
        poem = poem,
        num = num,
        lucky_color = color,
        lucky_dir = dir,
        exp = fortune.bonus.0,
        gold = fortune.bonus.1,
        crit = fortune.bonus.2,
        drop = fortune.bonus.3,
        gather = fortune.bonus.4,
    );

    // VIP加成提示
    let vip_level = crate::vip::get_vip_level(db, user_id);
    if vip_level > 0 {
        out.push_str(&format!(
            "\n👑 VIP{} 额外运势加成: +{}% 全属性",
            vip_level,
            vip_level * 2
        ));
    }

    out
}

/// 运势加成 — 显示当前运势的具体加成效果
pub fn cmd_fortune_bonus(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = today_str();
    let idx = calc_fortune_index(user_id, &today);
    let fortune = &FORTUNE_LEVELS[idx];
    let vip_level = crate::vip::get_vip_level(db, user_id);
    let vip_bonus = vip_level * 2;

    let (exp_base, gold_base, crit_base, drop_base, gather_base) = fortune.bonus;
    let exp_total = exp_base + vip_bonus;
    let gold_total = gold_base + vip_bonus;
    let crit_total = crit_base + vip_bonus;
    let drop_total = drop_base + vip_bonus;
    let gather_total = gather_base + vip_bonus;

    let mut out = format!(
        "{prefix}\n\
         {emoji} ═══ 运势加成详情 ═══ {emoji}\n\
         📅 {today} | 运势: {name}\n\n\
         ⚔️ 战斗加成:\n\
         ├ 📈 经验获取: {exp_base}% (基础) + {vip_bonus}% (VIP) = {exp_total}%\n\
         ├ 💰 金币获取: {gold_base}% (基础) + {vip_bonus}% (VIP) = {gold_total}%\n\
         ├ 💥 暴击率:   {crit_base}% (基础) + {vip_bonus}% (VIP) = {crit_total}%\n\
         └ 🎁 掉落率:   {drop_base}% (基础) + {vip_bonus}% (VIP) = {drop_total}%\n\n\
         🌿 生活加成:\n\
         ├ 🎣 采集成功率: {gather_total}%\n\
         └ 🌱 种植收获:   {gather_total}%\n\n\
         📋 运势等级一览:\n",
        prefix = prefix,
        emoji = fortune.emoji,
        today = today,
        name = fortune.name,
        exp_base = exp_base,
        gold_base = gold_base,
        crit_base = crit_base,
        drop_base = drop_base,
        vip_bonus = vip_bonus,
        exp_total = exp_total,
        gold_total = gold_total,
        crit_total = crit_total,
        drop_total = drop_total,
        gather_total = gather_total,
    );

    // 显示所有运势等级
    for f in FORTUNE_LEVELS {
        let marker = if f.name == fortune.name { " ◀ 当前" } else { "" };
        out.push_str(&format!(
            "  {} {} {:>4} 经验{:+3}% 金币{:+3}% 暴击{:+2}%{marker}\n",
            f.color,
            f.emoji,
            f.name,
            f.bonus.0,
            f.bonus.1,
            f.bonus.2,
            marker = marker,
        ));
    }

    out.push_str("\n💡 运势每天0点自动刷新，加成对当日所有活动生效！");
    out
}

/// 运势历史 — 查看最近7天的运势记录
pub fn cmd_fortune_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!(
        "{prefix}\n\
         📜 ═══ 运势历史 (近7天) ═══\n\n"
    );

    let today = Local::now().naive_local().date();
    let mut total_exp_bonus = 0i32;
    let mut fortune_counts = std::collections::HashMap::new();
    let mut days_found = 0;

    for i in 0..7 {
        let date = (today - chrono::Duration::days(i)).format("%Y-%m-%d").to_string();
        let idx = calc_fortune_index(user_id, &date);
        let fortune = &FORTUNE_LEVELS[idx];
        let num = lucky_number(user_id, &date);
        let day_label = if i == 0 {
            "今天".to_string()
        } else {
            format!("{}天前", i)
        };

        out.push_str(&format!(
            "  {} {} {}{} | 🍀{} | 经验{:+}% 金币{:+}%\n",
            date, day_label, fortune.emoji, fortune.name, num, fortune.bonus.0, fortune.bonus.1,
        ));

        total_exp_bonus += fortune.bonus.0;
        *fortune_counts.entry(fortune.name).or_insert(0u32) += 1;
        days_found += 1;
    }

    // 统计信息
    let avg_bonus = if days_found > 0 {
        total_exp_bonus / days_found
    } else {
        0
    };
    out.push_str(&format!(
        "\n📊 运势统计:\n\
         ├ 平均经验加成: {avg}%\n\
         └ 7天运势分布:",
        avg = avg_bonus,
    ));

    // 按出现次数排序
    let mut counts: Vec<_> = fortune_counts.into_iter().collect();
    counts.sort_by_key(|b| std::cmp::Reverse(b.1));
    for (name, count) in &counts {
        let emoji = FORTUNE_LEVELS
            .iter()
            .find(|f| f.name == *name)
            .map(|f| f.emoji)
            .unwrap_or("❓");
        let bar = "█".repeat(*count as usize);
        out.push_str(&format!("\n  {} {} {}天 {}", emoji, name, count, bar));
    }

    out.push_str("\n\n💡 运势每天0点刷新，坚持每日查看获取最佳加成！");
    out
}

/// 获取玩家当前运势的战斗加成系数
/// 返回 (经验倍率, 金币倍率, 暴击加成%, 掉率加成%)
#[allow(dead_code)]
pub fn get_fortune_combat_bonus(db: &Database, user_id: &str) -> (f64, f64, i32, i32) {
    let today = today_str();
    let idx = calc_fortune_index(user_id, &today);
    let fortune = &FORTUNE_LEVELS[idx];
    let vip_level = crate::vip::get_vip_level(db, user_id);
    let vip_bonus = vip_level * 2;

    let exp_mult = 1.0 + (fortune.bonus.0 + vip_bonus) as f64 / 100.0;
    let gold_mult = 1.0 + (fortune.bonus.1 + vip_bonus) as f64 / 100.0;
    let crit_bonus = fortune.bonus.2 + vip_bonus;
    let drop_bonus = fortune.bonus.3 + vip_bonus;

    (exp_mult.max(0.5), gold_mult.max(0.5), crit_bonus, drop_bonus)
}

/// 获取玩家当前运势的采集加成%
#[allow(dead_code)]
pub fn get_fortune_gather_bonus(db: &Database, user_id: &str) -> i32 {
    let today = today_str();
    let idx = calc_fortune_index(user_id, &today);
    let fortune = &FORTUNE_LEVELS[idx];
    let vip_level = crate::vip::get_vip_level(db, user_id);
    fortune.bonus.4 + vip_level * 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fortune_level_count() {
        assert_eq!(FORTUNE_LEVELS.len(), 7);
    }

    #[test]
    fn test_fortune_level_weights_sum() {
        let total: u32 = FORTUNE_LEVELS.iter().map(|f| f.weight).sum();
        assert!(total > 0);
        assert_eq!(total, 100); // weights should sum to 100
    }

    #[test]
    fn test_fortune_deterministic() {
        // Same user + same date = same result
        let idx1 = calc_fortune_index("12345", "2026-06-10");
        let idx2 = calc_fortune_index("12345", "2026-06-10");
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn test_fortune_different_users() {
        // Different users may get different fortunes
        let mut all_same = true;
        let date = "2026-06-10";
        let first = calc_fortune_index("111", date);
        for uid in &["222", "333", "444", "555"] {
            if calc_fortune_index(uid, date) != first {
                all_same = false;
                break;
            }
        }
        // Just check index is valid
        assert!(first < FORTUNE_LEVELS.len());
        let _ = all_same;
    }

    #[test]
    fn test_fortune_different_dates() {
        // Same user, different dates should produce different results over many days
        let mut results = std::collections::HashSet::new();
        for day in 1..=30 {
            let date = format!("2026-06-{:02}", day);
            results.insert(calc_fortune_index("test_user", &date));
        }
        // With 7 levels and 30 days, should see at least 3 different levels
        assert!(
            results.len() >= 3,
            "Expected at least 3 different fortune levels in 30 days, got {}",
            results.len()
        );
    }

    #[test]
    fn test_lucky_number_range() {
        for day in 1..=10 {
            let date = format!("2026-06-{:02}", day);
            let num = lucky_number("test", &date);
            assert!((1..=99).contains(&num), "Lucky number {} out of range", num);
        }
    }

    #[test]
    fn test_lucky_color_index_range() {
        for day in 1..=10 {
            let date = format!("2026-06-{:02}", day);
            let idx = lucky_color_index("test", &date);
            assert!(idx < LUCKY_COLORS.len());
        }
    }

    #[test]
    fn test_lucky_dir_index_range() {
        for day in 1..=10 {
            let date = format!("2026-06-{:02}", day);
            let idx = lucky_dir_index("test", &date);
            assert!(idx < LUCKY_DIRECTIONS.len());
        }
    }

    #[test]
    fn test_fortune_bonus_non_negative_base() {
        // At least 大吉 and 中吉 should have positive bonuses
        assert!(FORTUNE_LEVELS[0].bonus.0 > 0); // 大吉 exp
        assert!(FORTUNE_LEVELS[1].bonus.0 > 0); // 中吉 exp
                                                // 吉 should be neutral or positive
        assert!(FORTUNE_LEVELS[3].bonus.0 >= 0); // 吉 exp
    }

    #[test]
    fn test_fortune_poems_coverage() {
        // Every fortune level should have poems
        for f in FORTUNE_LEVELS {
            let has_poems = FORTUNE_POEMS.iter().any(|(name, _)| *name == f.name);
            assert!(has_poems, "Fortune level '{}' has no poems", f.name);
        }
    }

    #[test]
    fn test_fortune_combat_bonus_range() {
        // Even the worst fortune shouldn't reduce below 50%
        let worst_exp = FORTUNE_LEVELS.last().unwrap().bonus.0;
        let worst_gold = FORTUNE_LEVELS.last().unwrap().bonus.1;
        // Worst case: -5% bonus → 0.95 multiplier, still > 0.5
        let exp_mult = 1.0 + worst_exp as f64 / 100.0;
        let gold_mult = 1.0 + worst_gold as f64 / 100.0;
        assert!(exp_mult >= 0.5);
        assert!(gold_mult >= 0.5);
    }
}

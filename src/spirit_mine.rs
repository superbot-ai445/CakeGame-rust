/// CakeGame 灵矿开采系统
///
/// 玩家在不同地图发现并占领灵矿，被动产出矿物资源。
/// 矿脉可升级提升产出，其他玩家可争夺矿脉控制权。
/// 矿物可用于锻造/合成/出售，形成经济循环。
///
/// 数据来源: Global 表 SECTION='spirit_mine' / 'spirit_mine_log'
/// 指令: 查看灵矿/占领灵矿/灵矿升级/灵矿收获/灵矿争夺/灵矿排行/灵矿帮助
use crate::combat_power;
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::time::{SystemTime, UNIX_EPOCH};

const SECTION: &str = "spirit_mine";
const SECTION_LOG: &str = "spirit_mine_log";

/// 矿脉类型
const VEIN_TYPES: &[(&str, &str, &str, i64, i32, i32)] = &[
    // (名称, emoji, 矿物名, 每小时产出, 最大储量, 占领费用金币)
    ("铜矿脉", "🟤", "铜矿石", 10, 500, 1000),
    ("铁矿脉", "⬜", "铁矿石", 20, 1000, 5000),
    ("银矿脉", "🔘", "银矿石", 40, 2000, 15000),
    ("金矿脉", "🟡", "金矿石", 80, 4000, 50000),
    ("水晶矿脉", "💎", "水晶", 150, 8000, 150000),
    ("秘银矿脉", "🟣", "秘银矿", 300, 15000, 500000),
    ("星陨矿脉", "⭐", "星陨石", 600, 30000, 1500000),
];

/// 升级消耗 (等级→金币)
fn upgrade_cost(level: i32) -> i64 {
    (level as i64) * (level as i64) * 10000
}

/// 产出倍率 (等级→百分比)
fn level_multiplier(level: i32) -> f64 {
    1.0 + (level as f64 - 1.0) * 0.25
}

/// 获取当前时间戳
fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 解析矿脉数据
fn parse_mine_data(raw: &str) -> Option<(usize, i32, i64, i64)> {
    // format: "vein_type|level|last_collect_ts|stored"
    let parts: Vec<&str> = raw.split('|').collect();
    if parts.len() < 4 {
        return None;
    }
    let vein_type = parts[0].parse::<usize>().ok()?;
    let level = parts[1].parse::<i32>().ok()?;
    let last_ts = parts[2].parse::<i64>().ok()?;
    let stored = parts[3].parse::<i64>().ok()?;
    Some((vein_type, level, last_ts, stored))
}

/// 计算当前存储量
fn calc_stored(vein_type: usize, level: i32, last_ts: i64, stored: i64) -> i64 {
    if vein_type >= VEIN_TYPES.len() {
        return stored;
    }
    let base_rate = VEIN_TYPES[vein_type].3;
    let rate = (base_rate as f64 * level_multiplier(level)) as i64;
    let elapsed_hours = (now_ts() - last_ts) as f64 / 3600.0;
    let max_storage = VEIN_TYPES[vein_type].4 as i64;
    let new_stored = stored + (rate as f64 * elapsed_hours) as i64;
    new_stored.min(max_storage)
}

/// 进度条辅助
fn progress_bar(pct: i32, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// 查看灵矿 — 显示自己的矿脉状态
pub fn cmd_view_mine(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let key = format!("mine_{}", user_id);
    let raw = db.global_get(SECTION, &key);

    let mut out = format!("{}\n═══ ⛏️ 灵矿开采 ═══", prefix);

    if raw.is_empty() {
        out.push_str("\n\n您尚未拥有矿脉！");
        out.push_str("\n发送 '占领灵矿+矿脉编号' 来占领一座矿脉");
        out.push_str("\n\n📋 可用矿脉:");
        for (i, v) in VEIN_TYPES.iter().enumerate() {
            out.push_str(&format!(
                "\n  {}. {} {} — 产出:{}/时 最大:{} 占领费:{}金",
                i + 1,
                v.1,
                v.0,
                v.3,
                v.4,
                v.5
            ));
        }
    } else if let Some((vein_type, level, last_ts, old_stored)) = parse_mine_data(&raw) {
        if vein_type >= VEIN_TYPES.len() {
            return format!("{}\n⚠️ 矿脉数据异常，请联系管理员。", prefix);
        }
        let v = &VEIN_TYPES[vein_type];
        let stored = calc_stored(vein_type, level, last_ts, old_stored);
        let rate = (v.3 as f64 * level_multiplier(level)) as i64;
        let pct = (stored as f64 / v.4 as f64 * 100.0).min(100.0) as i32;
        let bar = progress_bar(pct, 20);

        out.push_str(&format!(
            "\n\n{} {} (Lv.{})\n矿物: {}\n产出速率: {}/时\n存储: {}/{} ({}%)\n{}\n升级消耗: {}金 (产出+25%)\n占领消耗: {}金",
            v.1, v.0, level, v.2, rate, stored, v.4, pct, bar, upgrade_cost(level + 1), v.5
        ));

        if stored >= v.4 as i64 {
            out.push_str("\n\n⚠️ 矿仓已满！请及时收获！");
        }

        let remaining = v.4 as i64 - stored;
        let hours_to_full = if rate > 0 {
            remaining as f64 / rate as f64
        } else {
            999.0
        };
        out.push_str(&format!("\n预计 {:.1} 小时后满仓", hours_to_full));
    }

    out
}

/// 占领灵矿 — 选择一座矿脉占领
pub fn cmd_claim_mine(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= VEIN_TYPES.len() => n - 1,
        _ => {
            return format!("{}\n请指定矿脉编号(1-{})！\n例: 占领灵矿+1", prefix, VEIN_TYPES.len());
        }
    };

    let key = format!("mine_{}", user_id);
    let existing = db.global_get(SECTION, &key);
    if !existing.is_empty() {
        return format!(
            "{}\n⚠️ 您已拥有矿脉！请先放弃后再占领新的。\n发送 '灵矿收获' 收取当前矿物。",
            prefix
        );
    }

    let v = &VEIN_TYPES[idx];
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < v.5 as i64 {
        return format!(
            "{}\n❌ 金币不足！占领 {} 需要 {}金，您只有 {}金。",
            prefix, v.0, v.5, gold
        );
    }

    // 扣除金币
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, v.5 as i64);

    // 创建矿脉数据
    let data = format!("{}|1|{}|0", idx, now_ts());
    db.global_set(SECTION, &key, &data);

    // 记录日志
    let log_key = format!("log_{}_{}", user_id, now_ts());
    db.global_set(SECTION_LOG, &log_key, &format!("占领{}", v.0));

    format!(
        "{}\n🎉 成功占领 {} {}！\n\n产出速率: {}/时 (Lv.1)\n最大存储: {}\n发送 '灵矿收获' 收取矿物！\n发送 '灵矿升级' 提升产出！",
        prefix, v.1, v.0, v.3, v.4
    )
}

/// 灵矿收获 — 收取存储的矿物
pub fn cmd_harvest_mine(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let key = format!("mine_{}", user_id);
    let raw = db.global_get(SECTION, &key);
    if raw.is_empty() {
        return format!("{}\n您尚未拥有矿脉！发送 '占领灵矿+编号' 来占领。", prefix);
    }

    let (vein_type, level, last_ts, old_stored) = match parse_mine_data(&raw) {
        Some(d) => d,
        None => return format!("{}\n⚠️ 矿脉数据异常。", prefix),
    };

    if vein_type >= VEIN_TYPES.len() {
        return format!("{}\n⚠️ 矿脉数据异常。", prefix);
    }

    let v = &VEIN_TYPES[vein_type];
    let stored = calc_stored(vein_type, level, last_ts, old_stored);

    if stored <= 0 {
        return format!("{}\n矿仓暂无可收取的矿物，稍后再来。", prefix);
    }

    // 更新存储为0，刷新时间戳
    let data = format!("{}|{}|{}|0", vein_type, level, now_ts());
    db.global_set(SECTION, &key, &data);

    // 将矿物存入背包 (作为金币等价物加成)
    // 矿物按品质给予金币: 1矿石 = 品质系数 * 2 金币
    let gold_value = stored * 2;
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_value);

    // 记录日志
    let log_key = format!("log_{}_{}", user_id, now_ts());
    db.global_set(
        SECTION_LOG,
        &log_key,
        &format!("收获{}×{}({}金)", v.2, stored, gold_value),
    );

    format!(
        "{}\n⛏️ 收获成功！\n\n获得: {} ×{} (价值{}金)\n矿仓已清空，继续开采中...",
        prefix, v.2, stored, gold_value
    )
}

/// 灵矿升级 — 提升矿脉等级增加产出
pub fn cmd_upgrade_mine(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let key = format!("mine_{}", user_id);
    let raw = db.global_get(SECTION, &key);
    if raw.is_empty() {
        return format!("{}\n您尚未拥有矿脉！发送 '占领灵矿+编号' 来占领。", prefix);
    }

    let (vein_type, level, last_ts, old_stored) = match parse_mine_data(&raw) {
        Some(d) => d,
        None => return format!("{}\n⚠️ 矿脉数据异常。", prefix),
    };

    let max_level = 20;
    if level >= max_level {
        return format!("{}\n⚠️ 矿脉已达最高等级 Lv.{}！", prefix, max_level);
    }

    let cost = upgrade_cost(level + 1);
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < cost {
        return format!(
            "{}\n❌ 金币不足！升级到 Lv.{} 需要 {}金，您只有 {}金。",
            prefix,
            level + 1,
            cost,
            gold
        );
    }

    // 先结算存储
    let stored = calc_stored(vein_type, level, last_ts, old_stored);

    // 扣除金币
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, cost);

    let new_level = level + 1;
    let data = format!("{}|{}|{}|{}", vein_type, new_level, now_ts(), stored);
    db.global_set(SECTION, &key, &data);

    let v = &VEIN_TYPES[vein_type];
    let new_rate = (v.3 as f64 * level_multiplier(new_level)) as i64;
    let old_rate = (v.3 as f64 * level_multiplier(level)) as i64;

    format!(
        "{}\n⬆️ 灵矿升级成功！\n\n{} {} Lv.{} → Lv.{}\n产出速率: {}/时 → {}/时 (+{})\n消耗: {}金",
        prefix,
        v.1,
        v.0,
        level,
        new_level,
        old_rate,
        new_rate,
        new_rate - old_rate,
        cost
    )
}

/// 灵矿争夺 — PvP争夺其他玩家的矿脉
pub fn cmd_raid_mine(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_id = args.trim();
    if target_id.is_empty() {
        return format!("{}\n请指定争夺目标！例: 灵矿争夺+目标ID", prefix);
    }

    if target_id == user_id {
        return format!("{}\n❌ 不能争夺自己的矿脉！", prefix);
    }

    // 检查自己是否已有矿脉
    let my_key = format!("mine_{}", user_id);
    if !db.global_get(SECTION, &my_key).is_empty() {
        return format!("{}\n❌ 您已拥有矿脉！只能同时持有一座矿脉。", prefix);
    }

    // 检查目标矿脉
    let target_key = format!("mine_{}", target_id);
    let target_raw = db.global_get(SECTION, &target_key);
    if target_raw.is_empty() {
        return format!("{}\n❌ 目标玩家没有矿脉可争夺。", prefix);
    }

    // 争夺费用
    let raid_cost = 10000i64;
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < raid_cost {
        return format!("{}\n❌ 争夺需要{}金，您只有{}金。", prefix, raid_cost, gold);
    }

    // 战力对比
    let my_info = user::calc_total_attrs(db, user_id);
    let target_info = user::calc_total_attrs(db, target_id);
    let my_power = combat_power::calc_combat_power(&my_info);
    let target_power = combat_power::calc_combat_power(&target_info);

    // 攻方有20%战力惩罚，增加争夺难度
    let effective_my = my_power * 0.8;
    let win_rate = (effective_my / (effective_my + target_power)).clamp(0.1, 0.9);

    let roll: f64 = rand::random();
    let _ = db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, raid_cost);

    if roll < win_rate {
        // 争夺成功：转移矿脉
        let (vein_type, level, _last_ts, stored) = parse_mine_data(&target_raw).unwrap_or((0, 1, now_ts(), 0));
        let new_data = format!("{}|{}|{}|{}", vein_type, level, now_ts(), stored);
        db.global_set(SECTION, &my_key, &new_data);
        db.global_set(SECTION, &target_key, ""); // 清空对方矿脉

        let v = &VEIN_TYPES[vein_type.min(VEIN_TYPES.len() - 1)];

        // 记录日志
        let log_key = format!("log_{}_{}", user_id, now_ts());
        db.global_set(SECTION_LOG, &log_key, &format!("争夺{}的{}成功", target_id, v.0));

        format!(
            "{}\n⚔️ 争夺成功！\n\n您击败了 {}，夺取了 {} {} (Lv.{})！\n矿脉内还有 {} 个{}等待收获。",
            prefix, target_id, v.1, v.0, level, stored, v.2
        )
    } else {
        // 争夺失败
        let log_key = format!("log_{}_{}", user_id, now_ts());
        db.global_set(SECTION_LOG, &log_key, &format!("争夺{}失败", target_id));

        format!(
            "{}\n⚔️ 争夺失败！\n\n{} 的守卫力量太强，您被击退了！\n消耗: {}金\n(您的战力{:.0} vs 对方战力{:.0})",
            prefix, target_id, raid_cost, my_power, target_power
        )
    }
}

/// 灵矿排行 — 全服矿脉排行
pub fn cmd_mine_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // 收集所有矿脉数据
    let mut mines: Vec<(String, usize, i32, i64, i64)> = Vec::new();

    for uid in db.all_users().iter() {
        let key = format!("mine_{}", uid);
        let val = db.global_get(SECTION, &key);
        if !val.is_empty() {
            if let Some((vein_type, level, last_ts, old_stored)) = parse_mine_data(&val) {
                let stored = calc_stored(vein_type, level, last_ts, old_stored);
                let score = (level as i64) * 1000 + stored + (vein_type as i64) * 500;
                mines.push((uid.to_string(), vein_type, level, stored, score));
            }
        }
    }

    mines.sort_by_key(|b| std::cmp::Reverse(b.4));
    mines.truncate(15);

    let mut out = format!("{}\n═══ 🏆 灵矿排行 ═══", prefix);

    if mines.is_empty() {
        out.push_str("\n\n暂无玩家占领矿脉");
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (uid, vt, level, stored, _)) in mines.iter().enumerate() {
            let medal = if i < 3 { medals[i] } else { "  " };
            let v = &VEIN_TYPES[*vt];
            let name = db.read_basic(uid, ITEM_NAME);
            let display_name = if name.is_empty() { uid.clone() } else { name };
            out.push_str(&format!(
                "\n{} {}. {} — {} {} Lv.{} (存储:{})",
                medal,
                i + 1,
                display_name,
                v.1,
                v.0,
                level,
                stored
            ));
        }

        // 显示自己排名
        let my_rank = mines.iter().position(|(uid, _, _, _, _)| uid == user_id);
        if let Some(r) = my_rank {
            out.push_str(&format!("\n\n📍 您的排名: 第 {} 名", r + 1));
        } else {
            out.push_str("\n\n📍 您尚未上榜 (未占领矿脉)");
        }
    }

    out
}

/// 灵矿帮助
pub fn cmd_mine_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n═══ ⛏️ 灵矿开采系统 ═══\n\n\
        🏗️ 系统介绍:\n\
        灵矿是散布在各地的矿物资源点，玩家可以占领并被动产出矿物。\n\
        矿物可兑换金币，支持升级和PvP争夺。\n\n\
        📋 指令列表:\n\
        • 查看灵矿 — 查看当前矿脉状态\n\
        • 占领灵矿+编号 — 占领一座矿脉(1-7)\n\
        • 灵矿收获 — 收取存储的矿物\n\
        • 灵矿升级 — 提升矿脉等级(+25%产出/级)\n\
        • 灵矿争夺+目标ID — PvP争夺他人矿脉\n\
        • 灵矿排行 — 全服矿脉排行\n\
        • 灵矿帮助 — 显示本帮助\n\n\
        ⚙️ 规则说明:\n\
        • 每人只能同时持有一座矿脉\n\
        • 矿仓有容量上限，满仓后停止产出\n\
        • 升级最高20级，每级+25%产出\n\
        • 争夺需消耗10000金，攻方战力-20%\n\
        • 矿物收获自动兑换为金币(×2倍率)",
        prefix
    )
}

/// 矿脉统计 API — 供外部系统查询
#[allow(dead_code)]
pub fn get_mine_bonus(db: &Database, user_id: &str) -> (i64, String) {
    let key = format!("mine_{}", user_id);
    let raw = db.global_get(SECTION, &key);
    if raw.is_empty() {
        return (0, String::new());
    }
    if let Some((vein_type, level, last_ts, old_stored)) = parse_mine_data(&raw) {
        let stored = calc_stored(vein_type, level, last_ts, old_stored);
        let v = &VEIN_TYPES[vein_type.min(VEIN_TYPES.len() - 1)];
        return (stored, v.2.to_string());
    }
    (0, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vein_types_count() {
        assert_eq!(VEIN_TYPES.len(), 7);
    }

    #[test]
    fn test_vein_types_sorted_by_output() {
        for i in 1..VEIN_TYPES.len() {
            assert!(VEIN_TYPES[i].3 > VEIN_TYPES[i - 1].3, "产出应递增");
        }
    }

    #[test]
    fn test_vein_types_sorted_by_cost() {
        for i in 1..VEIN_TYPES.len() {
            assert!(VEIN_TYPES[i].5 > VEIN_TYPES[i - 1].5, "费用应递增");
        }
    }

    #[test]
    fn test_upgrade_cost_escalates() {
        assert!(upgrade_cost(2) > upgrade_cost(1));
        assert!(upgrade_cost(10) > upgrade_cost(5));
        assert!(upgrade_cost(20) > upgrade_cost(10));
    }

    #[test]
    fn test_level_multiplier() {
        assert_eq!(level_multiplier(1), 1.0);
        assert!(level_multiplier(5) > level_multiplier(1));
        assert!(level_multiplier(20) > level_multiplier(10));
    }

    #[test]
    fn test_parse_mine_data_valid() {
        let result = parse_mine_data("2|5|1000000|500");
        assert!(result.is_some());
        let (vt, lvl, ts, stored) = result.unwrap();
        assert_eq!(vt, 2);
        assert_eq!(lvl, 5);
        assert_eq!(ts, 1000000);
        assert_eq!(stored, 500);
    }

    #[test]
    fn test_parse_mine_data_invalid() {
        assert!(parse_mine_data("").is_none());
        assert!(parse_mine_data("abc|1|0|0").is_none());
        assert!(parse_mine_data("1|2|3").is_none());
    }

    #[test]
    fn test_calc_stored_no_growth() {
        let stored = calc_stored(0, 1, now_ts(), 100);
        assert_eq!(stored, 100);
    }

    #[test]
    fn test_calc_stored_respects_max() {
        let stored = calc_stored(0, 1, 0, 0);
        assert!(stored <= VEIN_TYPES[0].4 as i64);
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 20);
        assert_eq!(bar, "[░░░░░░░░░░░░░░░░░░░░]");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 20);
        assert_eq!(bar, "[████████████████████]");
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 20);
        assert_eq!(bar, "[██████████░░░░░░░░░░]");
    }

    #[test]
    fn test_vein_names_unique() {
        let mut names: Vec<&str> = VEIN_TYPES.iter().map(|v| v.0).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before, "矿脉名称应唯一");
    }

    #[test]
    fn test_vein_emoji_unique() {
        let mut emojis: Vec<&str> = VEIN_TYPES.iter().map(|v| v.1).collect();
        let len_before = emojis.len();
        emojis.sort();
        emojis.dedup();
        assert_eq!(emojis.len(), len_before, "矿脉emoji应唯一");
    }

    #[test]
    fn test_upgrade_cost_positive() {
        for level in 1..=20 {
            assert!(upgrade_cost(level) > 0);
        }
    }

    #[test]
    fn test_level_multiplier_range() {
        let m1 = level_multiplier(1);
        let m20 = level_multiplier(20);
        assert_eq!(m1, 1.0);
        assert!(m20 > 4.0);
        assert!(m20 < 10.0);
    }

    #[test]
    fn test_vein_max_storage_positive() {
        for v in VEIN_TYPES {
            assert!(v.4 > 0, "最大储量应为正");
            assert!(v.3 > 0, "产出速率应为正");
        }
    }

    #[test]
    fn test_vein_cost_positive() {
        for v in VEIN_TYPES {
            assert!(v.5 > 0, "占领费用应为正");
        }
    }

    #[test]
    fn test_upgrade_cost_order() {
        for level in 1..20 {
            assert!(upgrade_cost(level + 1) > upgrade_cost(level), "升级费用应递增");
        }
    }

    #[test]
    fn test_calc_stored_growth_positive() {
        // With 1 hour elapsed, stored should grow
        let base = VEIN_TYPES[0].3; // 10/h
        let one_hour_ago = now_ts() - 3600;
        let stored = calc_stored(0, 1, one_hour_ago, 0);
        assert!(stored > 0, "经过1小时应有产出");
        assert!(stored <= base + 1, "产出应接近基础速率");
    }
}

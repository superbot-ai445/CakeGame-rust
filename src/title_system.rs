#![allow(dead_code)]

//! 称号系统 (Title System)
//!
//! 玩家通过达成成就解锁称号，装备称号获得被动属性加成。
//! - 15种成就称号: 签到/等级/BOSS/PvP/公会/金币/钻石/深渊
//! - 称号装备系统: 百分比属性加成
//! - 进度自动追踪: record_title_progress() API
//! - 全服称号排行

use crate::core::ITEM_NAME;
use crate::db::Database;
use crate::user;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type TitleDef = (
    u16,
    &'static str,
    &'static str,
    &'static str,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
    f64,
);

/// 称号定义
const TITLES: &[TitleDef] = &[
    // (id, name, icon, desc, hp_pct, ad_pct, ap_pct, def_pct, mres_pct, gold_pct, exp_pct)
    (1, "新人冒险者", "🔰", "完成注册", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    (2, "勤勉签到者", "📅", "累计签到7天", 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    (3, "连续签到王", "🏆", "累计签到30天", 5.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    (4, "等级新星", "⭐", "达到20级", 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    (5, "等级大师", "🌟", "达到50级", 0.0, 5.0, 5.0, 0.0, 0.0, 0.0, 0.0),
    (6, "等级传奇", "💫", "达到80级", 0.0, 8.0, 8.0, 3.0, 0.0, 0.0, 0.0),
    (7, "BOSS猎人", "🐉", "击败10个BOSS", 0.0, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    (
        8,
        "BOSS终结者",
        "🐲",
        "击败50个BOSS",
        5.0,
        10.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ),
    (9, "竞技勇士", "⚔️", "PvP胜利20次", 0.0, 5.0, 0.0, 3.0, 0.0, 0.0, 0.0),
    (10, "不败战神", "👑", "PvP胜利100次", 0.0, 10.0, 0.0, 8.0, 0.0, 0.0, 0.0),
    (11, "公会元老", "🏛️", "公会捐献50次", 5.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0),
    (12, "万金富翁", "💰", "累计100万金币", 0.0, 0.0, 0.0, 0.0, 0.0, 5.0, 0.0),
    (13, "钻石大亨", "💎", "累计5万钻石", 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 5.0),
    (14, "深渊行者", "🕳️", "深渊30层", 0.0, 0.0, 8.0, 5.0, 0.0, 0.0, 0.0),
    (
        15,
        "全能王者",
        "⚜️",
        "解锁所有称号",
        10.0,
        10.0,
        10.0,
        10.0,
        10.0,
        10.0,
        10.0,
    ),
];

/// 称号数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TitleData {
    pub unlocked: Vec<u16>,
    pub equipped: Option<u16>,
    pub progress: HashMap<String, u64>,
}

impl TitleData {
    fn is_unlocked(&self, id: u16) -> bool {
        self.unlocked.contains(&id)
    }

    fn unlock(&mut self, id: u16) {
        if !self.unlocked.contains(&id) {
            self.unlocked.push(id);
        }
    }
}

fn get_title_def(id: u16) -> Option<&'static TitleDef> {
    TITLES.iter().find(|t| t.0 == id)
}

fn find_title_by_name(name: &str) -> Option<&'static TitleDef> {
    TITLES.iter().find(|t| t.1 == name)
}

/// 获取装备称号的属性加成 (百分比)
pub fn get_title_bonus(db: &Database, user_id: &str) -> (f64, f64, f64, f64, f64, f64, f64) {
    let raw = db.global_get("title_system", user_id);
    let data: TitleData = if raw.is_empty() {
        TitleData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    if let Some(equipped_id) = data.equipped {
        if let Some(title) = get_title_def(equipped_id) {
            return (title.4, title.5, title.6, title.7, title.8, title.9, title.10);
        }
    }
    (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
}

/// 记录称号进度 (供外部系统调用)
pub fn record_title_progress(db: &Database, user_id: &str, event_type: &str, amount: u64) {
    let raw = db.global_get("title_system", user_id);
    let mut data: TitleData = if raw.is_empty() {
        TitleData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let key = event_type.to_string();
    let current = data.progress.entry(key).or_insert(0);
    *current += amount;

    // Auto-unlock check
    check_and_unlock(&mut data);

    db.global_set("title_system", user_id, &serde_json::to_string(&data).unwrap());
}

fn check_and_unlock(data: &mut TitleData) {
    let sign_ins = *data.progress.get("sign_in").unwrap_or(&0);
    let level = *data.progress.get("level").unwrap_or(&0);
    let boss_kills = *data.progress.get("boss_kill").unwrap_or(&0);
    let pvp_wins = *data.progress.get("pvp_win").unwrap_or(&0);
    let guild_donates = *data.progress.get("guild_donate").unwrap_or(&0);
    let total_gold = *data.progress.get("total_gold").unwrap_or(&0);
    let total_diamonds = *data.progress.get("total_diamonds").unwrap_or(&0);
    let abyss_floor = *data.progress.get("abyss_floor").unwrap_or(&0);

    // 新人冒险者: always unlocked on registration
    data.unlock(1);
    if sign_ins >= 7 {
        data.unlock(2);
    }
    if sign_ins >= 30 {
        data.unlock(3);
    }
    if level >= 20 {
        data.unlock(4);
    }
    if level >= 50 {
        data.unlock(5);
    }
    if level >= 80 {
        data.unlock(6);
    }
    if boss_kills >= 10 {
        data.unlock(7);
    }
    if boss_kills >= 50 {
        data.unlock(8);
    }
    if pvp_wins >= 20 {
        data.unlock(9);
    }
    if pvp_wins >= 100 {
        data.unlock(10);
    }
    if guild_donates >= 50 {
        data.unlock(11);
    }
    if total_gold >= 1_000_000 {
        data.unlock(12);
    }
    if total_diamonds >= 50_000 {
        data.unlock(13);
    }
    if abyss_floor >= 30 {
        data.unlock(14);
    }

    // 全能王者: all 14 titles unlocked
    if data.unlocked.len() >= 14 {
        data.unlock(15);
    }
}

/// 查看称号
pub fn cmd_title_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get("title_system", user_id);
    let data: TitleData = if raw.is_empty() {
        // First access: grant "新人冒险者" automatically
        let mut d = TitleData::default();
        d.unlock(1);
        db.global_set("title_system", user_id, &serde_json::to_string(&d).unwrap());
        d
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let equipped_name = data.equipped.and_then(get_title_def).map(|t| t.1).unwrap_or("无");

    let mut out = format!("{}\n═══ 称号系统 ═══\n", prefix);
    out.push_str(&format!("当前装备: {}\n\n", equipped_name));

    for title in TITLES {
        let status = if data.is_unlocked(title.0) { "✅" } else { "🔒" };
        let eq = if data.equipped == Some(title.0) { " 👈" } else { "" };
        out.push_str(&format!(
            "{} {}{} {} — {}{}\n",
            status,
            title.2,
            title.1,
            title.3,
            unlocked_bonus_str(title),
            eq
        ));
    }

    out.push_str(&format!("\n已解锁: {}/{}\n", data.unlocked.len(), TITLES.len()));
    out
}

fn unlocked_bonus_str(t: &(u16, &str, &str, &str, f64, f64, f64, f64, f64, f64, f64)) -> String {
    let mut parts = Vec::new();
    if t.4 > 0.0 {
        parts.push(format!("HP+{:.0}%", t.4));
    }
    if t.5 > 0.0 {
        parts.push(format!("物攻+{:.0}%", t.5));
    }
    if t.6 > 0.0 {
        parts.push(format!("魔攻+{:.0}%", t.6));
    }
    if t.7 > 0.0 {
        parts.push(format!("防御+{:.0}%", t.7));
    }
    if t.8 > 0.0 {
        parts.push(format!("魔抗+{:.0}%", t.8));
    }
    if t.9 > 0.0 {
        parts.push(format!("金币+{:.0}%", t.9));
    }
    if t.10 > 0.0 {
        parts.push(format!("经验+{:.0}%", t.10));
    }
    if parts.is_empty() {
        "无加成".to_string()
    } else {
        parts.join(" ")
    }
}

/// 装备称号
pub fn cmd_title_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();
    if name.is_empty() {
        return format!("{}\n格式: 装备称号+称号名", prefix);
    }

    let title = match find_title_by_name(name) {
        Some(t) => t,
        None => return format!("{}\n❌ 未找到称号: {}", prefix, name),
    };

    let raw = db.global_get("title_system", user_id);
    let mut data: TitleData = if raw.is_empty() {
        TitleData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    if !data.is_unlocked(title.0) {
        return format!("{}\n❌ 称号「{}」尚未解锁! 需要: {}", prefix, title.1, title.3);
    }

    data.equipped = Some(title.0);
    db.global_set("title_system", user_id, &serde_json::to_string(&data).unwrap());

    format!(
        "{}\n✅ 装备称号: {}{}\n加成: {}",
        prefix,
        title.2,
        title.1,
        unlocked_bonus_str(title)
    )
}

/// 卸下称号
pub fn cmd_title_unequip(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get("title_system", user_id);
    let mut data: TitleData = if raw.is_empty() {
        TitleData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    if data.equipped.is_none() {
        return format!("{}\n当前没有装备任何称号。", prefix);
    }

    let old_name = data
        .equipped
        .and_then(get_title_def)
        .map(|t| format!("{}{}", t.2, t.1))
        .unwrap_or_default();
    data.equipped = None;
    db.global_set("title_system", user_id, &serde_json::to_string(&data).unwrap());

    format!("{}\n✅ 已卸下称号: {}", prefix, old_name)
}

/// 称号详情
pub fn cmd_title_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let name = args.trim();
    if name.is_empty() {
        return format!("{}\n格式: 称号详情+称号名", prefix);
    }

    let title = match find_title_by_name(name) {
        Some(t) => t,
        None => return format!("{}\n❌ 未找到称号: {}", prefix, name),
    };

    let raw = db.global_get("title_system", user_id);
    let data: TitleData = if raw.is_empty() {
        TitleData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };
    let status = if data.is_unlocked(title.0) {
        "✅ 已解锁"
    } else {
        "🔒 未解锁"
    };
    let equipped = if data.equipped == Some(title.0) {
        " 👈 当前装备"
    } else {
        ""
    };

    let mut out = format!("{}\n═══ {}{} {} ═══\n", prefix, title.2, title.1, status);
    out.push_str(&format!("条件: {}\n", title.3));
    out.push_str(&format!("属性加成: {}\n", unlocked_bonus_str(title)));
    out.push_str(&format!("状态: {}{}\n", status, equipped));
    out
}

/// 称号进度
pub fn cmd_title_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let raw = db.global_get("title_system", user_id);
    let data: TitleData = if raw.is_empty() {
        TitleData::default()
    } else {
        serde_json::from_str(&raw).unwrap_or_default()
    };

    let mut out = format!("{}\n═══ 称号进度 ═══\n", prefix);

    let milestones: Vec<(&str, &str, u64)> = vec![
        ("sign_in", "勤勉签到者(7天)", 7),
        ("sign_in", "连续签到王(30天)", 30),
        ("level", "等级新星(20级)", 20),
        ("level", "等级大师(50级)", 50),
        ("level", "等级传奇(80级)", 80),
        ("boss_kill", "BOSS猎人(10个)", 10),
        ("boss_kill", "BOSS终结者(50个)", 50),
        ("pvp_win", "竞技勇士(20次)", 20),
        ("pvp_win", "不败战神(100次)", 100),
        ("guild_donate", "公会元老(50次)", 50),
        ("total_gold", "万金富翁(100万)", 1_000_000),
        ("total_diamonds", "钻石大亨(5万)", 50_000),
        ("abyss_floor", "深渊行者(30层)", 30),
    ];

    for (key, label, target) in milestones {
        let current = *data.progress.get(key).unwrap_or(&0);
        let pct = (current * 100 / target).min(100);
        let bar = progress_bar(pct as u32, 10);
        let status = if current >= target { "✅" } else { "" };
        out.push_str(&format!(
            "  {} {}/{} {} {}\n",
            label,
            current.min(target),
            target,
            bar,
            status
        ));
    }

    out.push_str(&format!("\n已解锁: {}/{}\n", data.unlocked.len(), TITLES.len()));
    out
}

/// 称号排行
pub fn cmd_title_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare("SELECT Key, Value FROM Global WHERE Section='title_system'") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };

    let mut rankings: Vec<(String, usize, Option<u16>)> = stmt
        .query_map([], |row| {
            let uid: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((uid, val))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|(uid, val)| {
            let data: TitleData = serde_json::from_str(&val).ok()?;
            if data.unlocked.is_empty() {
                return None;
            }
            Some((uid, data.unlocked.len(), data.equipped))
        })
        .collect();

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    if rankings.is_empty() {
        return format!("{}\n暂无称号排行数据。", prefix);
    }

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = format!("{}\n═══ 称号排行 ═══\n", prefix);
    for (i, (uid, count, equipped)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, ITEM_NAME);
        let display_name = if name.is_empty() { uid.clone() } else { name };
        let eq_str = equipped
            .and_then(get_title_def)
            .map(|t| format!(" [{}{}]", t.2, t.1))
            .unwrap_or_default();
        out.push_str(&format!(
            "{}{}. {} — {}个称号{}\n",
            medal,
            i + 1,
            display_name,
            count,
            eq_str
        ));
    }
    out
}

/// 称号列表
pub fn cmd_title_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let mut out = format!("{}\n═══ 称号列表 ═══\n", prefix);
    for title in TITLES {
        out.push_str(&format!(
            "  {} {} — {} | 加成: {}\n",
            title.2,
            title.1,
            title.3,
            unlocked_bonus_str(title)
        ));
    }
    out.push_str("\n使用「装备称号+名称」装备称号获得属性加成。\n");
    out
}

/// 称号帮助
pub fn cmd_title_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    format!(
        "{}\n        ═══ 称号系统帮助 ═══\n\n        通过达成各种成就解锁称号，装备称号获得被动属性百分比加成。\n\n        📋 指令列表:\n        • 查看称号 — 查看所有称号状态\n        • 装备称号+名称 — 装备称号获得加成\n        • 卸下称号 — 卸下当前称号\n        • 称号详情+名称 — 查看称号详细信息\n        • 称号进度 — 查看解锁进度\n        • 称号排行 — 全服称号数量排行\n        • 称号列表 — 列出所有称号和条件\n        • 称号帮助 — 本帮助信息\n\n        🏆 15种称号: 新人冒险者→全能王者\n        📊 8大维度: 签到/等级/BOSS/PvP/公会/金币/钻石/深渊\n        💪 加成类型: HP/物攻/魔攻/防御/魔抗/金币获取/经验获取 (百分比)\n        ✨ 全能王者: 解锁全部称号后获得10%全属性加成\n",
        prefix
    )
}

/// Helper: progress bar
fn progress_bar(pct: u32, width: usize) -> String {
    let filled = (pct as usize * width / 100).min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_definitions_count() {
        assert_eq!(TITLES.len(), 15);
    }

    #[test]
    fn test_title_definitions_unique_ids() {
        let mut ids: Vec<u16> = TITLES.iter().map(|t| t.0).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 15);
    }

    #[test]
    fn test_title_definitions_unique_names() {
        let mut names: Vec<&str> = TITLES.iter().map(|t| t.1).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 15);
    }

    #[test]
    fn test_title_data_default() {
        let data = TitleData::default();
        assert!(data.unlocked.is_empty());
        assert!(data.equipped.is_none());
        assert!(data.progress.is_empty());
    }

    #[test]
    fn test_title_data_unlock() {
        let mut data = TitleData::default();
        data.unlock(1);
        assert!(data.is_unlocked(1));
        assert!(!data.is_unlocked(2));
        // Double unlock should not duplicate
        data.unlock(1);
        assert_eq!(data.unlocked.len(), 1);
    }

    #[test]
    fn test_title_data_equip_unequip() {
        let mut data = TitleData::default();
        data.unlock(1);
        data.equipped = Some(1);
        assert_eq!(data.equipped, Some(1));
        data.equipped = None;
        assert!(data.equipped.is_none());
    }

    #[test]
    fn test_title_bonus_none_equipped() {
        // No bonus when no title equipped
        let data = TitleData::default();
        assert!(data.equipped.is_none());
    }

    #[test]
    fn test_title_bonus_with_title() {
        let title = get_title_def(3).unwrap();
        assert!(title.4 > 0.0); // HP bonus
    }

    #[test]
    fn test_check_and_unlock_basic() {
        let mut data = TitleData::default();
        data.progress.insert("sign_in".to_string(), 7);
        check_and_unlock(&mut data);
        assert!(data.is_unlocked(1)); // 新人冒险者
        assert!(data.is_unlocked(2)); // 勤勉签到者
        assert!(!data.is_unlocked(3)); // not yet 30
    }

    #[test]
    fn test_check_and_unlock_all() {
        let mut data = TitleData::default();
        data.progress.insert("sign_in".to_string(), 100);
        data.progress.insert("level".to_string(), 100);
        data.progress.insert("boss_kill".to_string(), 100);
        data.progress.insert("pvp_win".to_string(), 200);
        data.progress.insert("guild_donate".to_string(), 100);
        data.progress.insert("total_gold".to_string(), 10_000_000);
        data.progress.insert("total_diamonds".to_string(), 100_000);
        data.progress.insert("abyss_floor".to_string(), 100);
        check_and_unlock(&mut data);
        assert_eq!(data.unlocked.len(), 15);
        assert!(data.is_unlocked(15)); // 全能王者
    }

    #[test]
    fn test_find_title_by_name() {
        assert!(find_title_by_name("新人冒险者").is_some());
        assert!(find_title_by_name("不存在的称号").is_none());
    }

    #[test]
    fn test_unlocked_bonus_str() {
        let title = get_title_def(1).unwrap();
        assert_eq!(unlocked_bonus_str(title), "无加成");
        let title = get_title_def(3).unwrap();
        assert!(unlocked_bonus_str(title).contains("HP"));
    }

    #[test]
    fn test_progress_bar() {
        assert_eq!(progress_bar(0, 10), "[░░░░░░░░░░]");
        assert_eq!(progress_bar(100, 10), "[██████████]");
        assert_eq!(progress_bar(50, 10), "[█████░░░░░]");
    }

    #[test]
    fn test_title_id_range() {
        for title in TITLES {
            assert!(title.0 >= 1 && title.0 <= 15, "Title ID {} out of range", title.0);
        }
    }

    #[test]
    fn test_first_title_is_free() {
        let title = get_title_def(1).unwrap();
        assert_eq!(title.1, "新人冒险者");
        assert_eq!(title.3, "完成注册");
    }
}

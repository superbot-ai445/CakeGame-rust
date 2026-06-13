/// CakeGame 称号系统
/// 基于玩家成就自动解锁称号，可装备提供属性加成
use crate::db::Database;
use crate::user;

/// 称号定义
struct TitleDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    /// 解锁条件类型
    unlock_type: &'static str,
    /// 解锁条件值
    unlock_value: i64,
    /// 属性加成: (属性名, 加成值)
    bonus_hp: i32,
    bonus_mp: i32,
    bonus_ad: i32,
    bonus_ap: i32,
    bonus_def: i32,
    bonus_mdf: i32,
}

const TITLES: &[TitleDef] = &[
    // 等级称号
    TitleDef {
        id: "lv10",
        name: "初出茅庐",
        desc: "达到10级",
        unlock_type: "level",
        unlock_value: 10,
        bonus_hp: 50,
        bonus_mp: 30,
        bonus_ad: 5,
        bonus_ap: 5,
        bonus_def: 3,
        bonus_mdf: 3,
    },
    TitleDef {
        id: "lv20",
        name: "小有经验",
        desc: "达到20级",
        unlock_type: "level",
        unlock_value: 20,
        bonus_hp: 100,
        bonus_mp: 60,
        bonus_ad: 10,
        bonus_ap: 10,
        bonus_def: 6,
        bonus_mdf: 6,
    },
    TitleDef {
        id: "lv30",
        name: "身经百战",
        desc: "达到30级",
        unlock_type: "level",
        unlock_value: 30,
        bonus_hp: 200,
        bonus_mp: 120,
        bonus_ad: 20,
        bonus_ap: 20,
        bonus_def: 12,
        bonus_mdf: 12,
    },
    TitleDef {
        id: "lv40",
        name: "久经沙场",
        desc: "达到40级",
        unlock_type: "level",
        unlock_value: 40,
        bonus_hp: 350,
        bonus_mp: 200,
        bonus_ad: 35,
        bonus_ap: 35,
        bonus_def: 20,
        bonus_mdf: 20,
    },
    TitleDef {
        id: "lv50",
        name: "传奇勇者",
        desc: "达到50级",
        unlock_type: "level",
        unlock_value: 50,
        bonus_hp: 500,
        bonus_mp: 300,
        bonus_ad: 50,
        bonus_ap: 50,
        bonus_def: 30,
        bonus_mdf: 30,
    },
    // PVP称号
    TitleDef {
        id: "pvp10",
        name: "竞技新星",
        desc: "匹配竞技胜利10次",
        unlock_type: "pvp_wins",
        unlock_value: 10,
        bonus_hp: 80,
        bonus_mp: 40,
        bonus_ad: 15,
        bonus_ap: 15,
        bonus_def: 8,
        bonus_mdf: 8,
    },
    TitleDef {
        id: "pvp50",
        name: "竞技达人",
        desc: "匹配竞技胜利50次",
        unlock_type: "pvp_wins",
        unlock_value: 50,
        bonus_hp: 200,
        bonus_mp: 100,
        bonus_ad: 30,
        bonus_ap: 30,
        bonus_def: 18,
        bonus_mdf: 18,
    },
    TitleDef {
        id: "pvp200",
        name: "竞技王者",
        desc: "匹配竞技胜利200次",
        unlock_type: "pvp_wins",
        unlock_value: 200,
        bonus_hp: 500,
        bonus_mp: 250,
        bonus_ad: 60,
        bonus_ap: 60,
        bonus_def: 40,
        bonus_mdf: 40,
    },
    // 财富称号
    TitleDef {
        id: "rich1",
        name: "小富即安",
        desc: "累计获得10万金币",
        unlock_type: "total_gold",
        unlock_value: 100_000,
        bonus_hp: 60,
        bonus_mp: 40,
        bonus_ad: 8,
        bonus_ap: 8,
        bonus_def: 5,
        bonus_mdf: 5,
    },
    TitleDef {
        id: "rich2",
        name: "腰缠万贯",
        desc: "累计获得100万金币",
        unlock_type: "total_gold",
        unlock_value: 1_000_000,
        bonus_hp: 200,
        bonus_mp: 120,
        bonus_ad: 25,
        bonus_ap: 25,
        bonus_def: 15,
        bonus_mdf: 15,
    },
    TitleDef {
        id: "rich3",
        name: "富甲一方",
        desc: "累计获得1000万金币",
        unlock_type: "total_gold",
        unlock_value: 10_000_000,
        bonus_hp: 500,
        bonus_mp: 300,
        bonus_ad: 50,
        bonus_ap: 50,
        bonus_def: 35,
        bonus_mdf: 35,
    },
    // 收集称号
    TitleDef {
        id: "gather50",
        name: "勤劳采集者",
        desc: "采集50次",
        unlock_type: "gather_count",
        unlock_value: 50,
        bonus_hp: 100,
        bonus_mp: 80,
        bonus_ad: 10,
        bonus_ap: 10,
        bonus_def: 8,
        bonus_mdf: 8,
    },
    TitleDef {
        id: "gather200",
        name: "资源大师",
        desc: "采集200次",
        unlock_type: "gather_count",
        unlock_value: 200,
        bonus_hp: 300,
        bonus_mp: 200,
        bonus_ad: 25,
        bonus_ap: 25,
        bonus_def: 18,
        bonus_mdf: 18,
    },
    // 签到称号
    TitleDef {
        id: "sign7",
        name: "勤勉之士",
        desc: "连续签到7天",
        unlock_type: "sign_streak",
        unlock_value: 7,
        bonus_hp: 80,
        bonus_mp: 50,
        bonus_ad: 8,
        bonus_ap: 8,
        bonus_def: 5,
        bonus_mdf: 5,
    },
    TitleDef {
        id: "sign30",
        name: "坚定不移",
        desc: "连续签到30天",
        unlock_type: "sign_streak",
        unlock_value: 30,
        bonus_hp: 300,
        bonus_mp: 180,
        bonus_ad: 30,
        bonus_ap: 30,
        bonus_def: 20,
        bonus_mdf: 20,
    },
    TitleDef {
        id: "sign100",
        name: "铁杵磨针",
        desc: "累计签到100天",
        unlock_type: "sign_total",
        unlock_value: 100,
        bonus_hp: 600,
        bonus_mp: 400,
        bonus_ad: 60,
        bonus_ap: 60,
        bonus_def: 40,
        bonus_mdf: 40,
    },
    // 副本称号
    TitleDef {
        id: "dungeon10",
        name: "副本探索者",
        desc: "完成副本10次",
        unlock_type: "dungeon_clears",
        unlock_value: 10,
        bonus_hp: 150,
        bonus_mp: 80,
        bonus_ad: 18,
        bonus_ap: 18,
        bonus_def: 10,
        bonus_mdf: 10,
    },
    TitleDef {
        id: "dungeon50",
        name: "副本征服者",
        desc: "完成副本50次",
        unlock_type: "dungeon_clears",
        unlock_value: 50,
        bonus_hp: 400,
        bonus_mp: 200,
        bonus_ad: 40,
        bonus_ap: 40,
        bonus_def: 25,
        bonus_mdf: 25,
    },
    // BOSS称号
    TitleDef {
        id: "boss5",
        name: "怪物猎人",
        desc: "击败BOSS 5次",
        unlock_type: "boss_kills",
        unlock_value: 5,
        bonus_hp: 120,
        bonus_mp: 60,
        bonus_ad: 15,
        bonus_ap: 15,
        bonus_def: 10,
        bonus_mdf: 10,
    },
    TitleDef {
        id: "boss20",
        name: "BOSS终结者",
        desc: "击败BOSS 20次",
        unlock_type: "boss_kills",
        unlock_value: 20,
        bonus_hp: 350,
        bonus_mp: 180,
        bonus_ad: 35,
        bonus_ap: 35,
        bonus_def: 22,
        bonus_mdf: 22,
    },
];

/// 获取用户已解锁的称号列表
fn get_unlocked_titles(db: &Database, user_id: &str) -> Vec<String> {
    let raw = db.read_user_data(user_id, "unlocked_titles");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 保存已解锁称号
fn save_unlocked_titles(db: &Database, user_id: &str, titles: &[String]) {
    db.write_user_data(user_id, "unlocked_titles", &titles.join(","));
}

/// 检查并自动解锁称号
pub fn check_title_unlock(db: &Database, user_id: &str) -> Vec<String> {
    let mut newly_unlocked = Vec::new();
    let mut unlocked = get_unlocked_titles(db, user_id);

    for title in TITLES {
        if unlocked.contains(&title.id.to_string()) {
            continue;
        }

        let met = match title.unlock_type {
            "level" => {
                let level: i64 = db.read_basic(user_id, "Level").parse().unwrap_or(1);
                level >= title.unlock_value
            }
            "pvp_wins" => {
                let wins: i64 = db.read_user_data(user_id, "pvp_wins").parse().unwrap_or(0);
                wins >= title.unlock_value
            }
            "total_gold" => {
                let total: i64 = db.read_user_data(user_id, "total_gold_earned").parse().unwrap_or(0);
                total >= title.unlock_value
            }
            "gather_count" => {
                let total_gather: i64 = {
                    let fishing: i64 = db.read_user_data(user_id, "gather_fishing_count").parse().unwrap_or(0);
                    let mining: i64 = db.read_user_data(user_id, "gather_mining_count").parse().unwrap_or(0);
                    let herb: i64 = db.read_user_data(user_id, "gather_herb_count").parse().unwrap_or(0);
                    let collect: i64 = db.read_user_data(user_id, "gather_collect_count").parse().unwrap_or(0);
                    fishing + mining + herb + collect
                };
                total_gather >= title.unlock_value
            }
            "sign_streak" => {
                let streak: i64 = db.read_user_data(user_id, "sign_in_streak").parse().unwrap_or(0);
                streak >= title.unlock_value
            }
            "sign_total" => {
                let total: i64 = db.read_user_data(user_id, "sign_in_total").parse().unwrap_or(0);
                total >= title.unlock_value
            }
            "dungeon_clears" => {
                let clears: i64 = db.read_user_data(user_id, "dungeon_clears").parse().unwrap_or(0);
                clears >= title.unlock_value
            }
            "boss_kills" => {
                let kills: i64 = db.read_user_data(user_id, "boss_kills").parse().unwrap_or(0);
                kills >= title.unlock_value
            }
            _ => false,
        };

        if met {
            unlocked.push(title.id.to_string());
            newly_unlocked.push(title.name.to_string());
        }
    }

    if !newly_unlocked.is_empty() {
        save_unlocked_titles(db, user_id, &unlocked);
    }

    newly_unlocked
}

/// 获取当前装备的称号ID
fn get_equipped_title(db: &Database, user_id: &str) -> String {
    db.read_user_data(user_id, "equipped_title")
}

/// 查看称号列表
pub fn cmd_view_titles(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看称号！", prefix);
    }

    // 先检查是否有新称号解锁
    let new_titles = check_title_unlock(db, user_id);
    let unlocked = get_unlocked_titles(db, user_id);
    let equipped_id = get_equipped_title(db, user_id);

    let args_trimmed = args.trim();

    // 查看特定称号详情
    if !args_trimmed.is_empty() {
        if let Some(title) = TITLES.iter().find(|t| t.name == args_trimmed || t.id == args_trimmed) {
            let is_unlocked = unlocked.contains(&title.id.to_string());
            let is_equipped = equipped_id == title.id;
            let mut result = format!("{}\n═══ 称号详情 ═══\n", prefix);
            result.push_str(&format!("📌 {}\n", title.name));
            result.push_str(&format!("📖 {}\n", title.desc));
            result.push_str(&format!(
                "状态：{}\n",
                if is_equipped {
                    "✅ 已装备"
                } else if is_unlocked {
                    "🔓 已解锁"
                } else {
                    "🔒 未解锁"
                }
            ));
            result.push_str("\n属性加成：");
            if title.bonus_hp > 0 {
                result.push_str(&format!("\n  ❤️ 生命+{}", title.bonus_hp));
            }
            if title.bonus_mp > 0 {
                result.push_str(&format!("\n  💧 魔法+{}", title.bonus_mp));
            }
            if title.bonus_ad > 0 {
                result.push_str(&format!("\n  ⚔️ 物攻+{}", title.bonus_ad));
            }
            if title.bonus_ap > 0 {
                result.push_str(&format!("\n  🔮 魔攻+{}", title.bonus_ap));
            }
            if title.bonus_def > 0 {
                result.push_str(&format!("\n  🛡️ 防御+{}", title.bonus_def));
            }
            if title.bonus_mdf > 0 {
                result.push_str(&format!("\n  🔰 魔抗+{}", title.bonus_mdf));
            }
            if is_unlocked && !is_equipped {
                result.push_str(&format!("\n\n💡 使用「装备称号+{}」来装备", title.name));
            }
            return result;
        }
        return format!("{}\n❌ 未找到称号「{}」", prefix, args_trimmed);
    }

    // 称号列表
    let mut result = format!("{}\n═══ 称号系统 ═══\n", prefix);
    result.push_str(&format!("已解锁：{}/{}\n", unlocked.len(), TITLES.len()));

    if let Some(eq_title) = TITLES.iter().find(|t| t.id == equipped_id) {
        result.push_str(&format!("当前装备：🏷️ {}\n", eq_title.name));
    } else {
        result.push_str("当前装备：无\n");
    }

    if !new_titles.is_empty() {
        result.push_str(&format!("\n🎉 新解锁称号：{}\n", new_titles.join("、")));
    }

    // 按类型分组显示
    let categories: &[(&str, &str)] = &[
        ("level", "📊 等级称号"),
        ("pvp_wins", "⚔️ 竞技称号"),
        ("total_gold", "💰 财富称号"),
        ("gather_count", "🌿 采集称号"),
        ("sign_streak", "📅 签到称号"),
        ("sign_total", "📋 累计签到"),
        ("dungeon_clears", "🏰 副本称号"),
        ("boss_kills", "👹 BOSS称号"),
    ];

    for (cat_type, cat_name) in categories {
        let cat_titles: Vec<_> = TITLES.iter().filter(|t| t.unlock_type == *cat_type).collect();
        if cat_titles.is_empty() {
            continue;
        }
        result.push_str(&format!("\n{}\n", cat_name));
        for title in &cat_titles {
            let is_unlocked = unlocked.contains(&title.id.to_string());
            let is_equipped = equipped_id == title.id;
            let icon = if is_equipped {
                "🏷️"
            } else if is_unlocked {
                "✅"
            } else {
                "🔒"
            };
            result.push_str(&format!("  {} {} - {}\n", icon, title.name, title.desc));
        }
    }

    result.push_str("\n💡 使用「装备称号+称号名」装备称号");
    result.push_str("\n💡 使用「查看称号+称号名」查看详情");
    result.push_str("\n💡 使用「卸下称号」取下当前称号");

    result
}

/// 装备称号
pub fn cmd_equip_title(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再装备称号！", prefix);
    }

    let title_name = args.trim();
    if title_name.is_empty() {
        return format!("{}\n请指定要装备的称号名称！\n用法: 装备称号+称号名", prefix);
    }

    // 检查死亡
    let hp: i32 = db.read_basic(user_id, "P_HP").parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再装备称号。", prefix);
    }

    // 先检查解锁
    check_title_unlock(db, user_id);
    let unlocked = get_unlocked_titles(db, user_id);

    // 查找称号
    let title = match TITLES.iter().find(|t| t.name == title_name || t.id == title_name) {
        Some(t) => t,
        None => return format!("{}\n❌ 未找到称号「{}」", prefix, title_name),
    };

    if !unlocked.contains(&title.id.to_string()) {
        return format!(
            "{}\n🔒 称号「{}」尚未解锁！\n解锁条件：{}",
            prefix, title.name, title.desc
        );
    }

    // 装备称号
    db.write_user_data(user_id, "equipped_title", title.id);

    let mut result = format!("{}\n✅ 成功装备称号：🏷️ {}\n", prefix, title.name);
    result.push_str("属性加成：");
    if title.bonus_hp > 0 {
        result.push_str(&format!(" 生命+{}", title.bonus_hp));
    }
    if title.bonus_mp > 0 {
        result.push_str(&format!(" 魔法+{}", title.bonus_mp));
    }
    if title.bonus_ad > 0 {
        result.push_str(&format!(" 物攻+{}", title.bonus_ad));
    }
    if title.bonus_ap > 0 {
        result.push_str(&format!(" 魔攻+{}", title.bonus_ap));
    }
    if title.bonus_def > 0 {
        result.push_str(&format!(" 防御+{}", title.bonus_def));
    }
    if title.bonus_mdf > 0 {
        result.push_str(&format!(" 魔抗+{}", title.bonus_mdf));
    }

    result
}

/// 卸下称号
pub fn cmd_unequip_title(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册！", prefix);
    }

    let equipped_id = get_equipped_title(db, user_id);
    if equipped_id.is_empty() {
        return format!("{}\n当前没有装备任何称号。", prefix);
    }

    let title_name = TITLES
        .iter()
        .find(|t| t.id == equipped_id)
        .map(|t| t.name)
        .unwrap_or("未知称号");

    db.write_user_data(user_id, "equipped_title", "");
    format!("{}\n✅ 已卸下称号：🏷️ {}", prefix, title_name)
}

/// 计算称号属性加成（供 calc_total_attrs 调用）
pub fn get_title_bonuses(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32, i32) {
    let equipped_id = get_equipped_title(db, user_id);
    if equipped_id.is_empty() {
        return (0, 0, 0, 0, 0, 0);
    }

    match TITLES.iter().find(|t| t.id == equipped_id) {
        Some(title) => (
            title.bonus_hp,
            title.bonus_mp,
            title.bonus_ad,
            title.bonus_ap,
            title.bonus_def,
            title.bonus_mdf,
        ),
        None => (0, 0, 0, 0, 0, 0),
    }
}

/// 根据ID查找称号定义
#[allow(dead_code)]
fn find_title_by_id(id: &str) -> Option<&'static TitleDef> {
    TITLES.iter().find(|t| t.id == id)
}

/// 获取所有称号定义（供外部查询）
#[allow(dead_code)]
fn all_titles() -> &'static [TitleDef] {
    TITLES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_titles_not_empty() {
        assert!(!TITLES.is_empty(), "称号列表不应为空");
    }

    #[test]
    fn test_titles_unique_ids() {
        let mut ids: Vec<&str> = TITLES.iter().map(|t| t.id).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), before, "称号ID不应重复");
    }

    #[test]
    fn test_titles_unique_names() {
        let mut names: Vec<&str> = TITLES.iter().map(|t| t.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), before, "称号名称不应重复");
    }

    #[test]
    fn test_titles_have_valid_unlock_types() {
        let valid_types = [
            "level",
            "pvp_wins",
            "total_gold",
            "gather_count",
            "boss_kills",
            "sign_streak",
            "sign_total",
            "dungeon_clears",
        ];
        for title in TITLES {
            assert!(
                valid_types.contains(&title.unlock_type),
                "称号 {} 的解锁类型 {} 不合法",
                title.name,
                title.unlock_type
            );
        }
    }

    #[test]
    fn test_titles_unlock_value_positive() {
        for title in TITLES {
            assert!(title.unlock_value > 0, "称号 {} 的解锁值应为正数", title.name);
        }
    }

    #[test]
    fn test_titles_bonuses_non_negative() {
        for title in TITLES {
            assert!(title.bonus_hp >= 0, "称号 {} 的HP加成应非负", title.name);
            assert!(title.bonus_mp >= 0, "称号 {} 的MP加成应非负", title.name);
            assert!(title.bonus_ad >= 0, "称号 {} 的AD加成应非负", title.name);
            assert!(title.bonus_ap >= 0, "称号 {} 的AP加成应非负", title.name);
            assert!(title.bonus_def >= 0, "称号 {} 的防御加成应非负", title.name);
            assert!(title.bonus_mdf >= 0, "称号 {} 的魔抗加成应非负", title.name);
        }
    }

    #[test]
    fn test_titles_level_unlocks_ordered() {
        let level_titles: Vec<&TitleDef> = TITLES.iter().filter(|t| t.unlock_type == "level").collect();
        for i in 1..level_titles.len() {
            assert!(
                level_titles[i].unlock_value > level_titles[i - 1].unlock_value,
                "等级称号 {} 的解锁值应大于 {}",
                level_titles[i].name,
                level_titles[i - 1].name
            );
        }
    }

    #[test]
    fn test_titles_bonus_scaling() {
        // 同类型称号，解锁值越高，加成应越大
        let level_titles: Vec<&TitleDef> = TITLES.iter().filter(|t| t.unlock_type == "level").collect();
        for i in 1..level_titles.len() {
            assert!(
                level_titles[i].bonus_hp >= level_titles[i - 1].bonus_hp,
                "等级称号 {} 的HP加成应大于等于 {}",
                level_titles[i].name,
                level_titles[i - 1].name
            );
        }
    }

    #[test]
    fn test_find_title_by_id() {
        let title = find_title_by_id("lv10");
        assert!(title.is_some());
        assert_eq!(title.unwrap().name, "初出茅庐");
    }

    #[test]
    fn test_find_title_by_id_not_found() {
        let title = find_title_by_id("nonexistent");
        assert!(title.is_none());
    }

    #[test]
    fn test_all_titles_returns_titles() {
        let titles = all_titles();
        assert_eq!(titles.len(), TITLES.len());
    }
}

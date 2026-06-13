/// CakeGame 竞技荣誉商店系统
/// 玩家通过竞技匹配获取荣誉积分，可在荣誉商店兑换专属奖励
/// 荣誉积分来源: 匹配胜利、赛季排名、竞技场锦标赛
///
/// 指令: 荣誉商店, 荣誉兑换, 荣誉余额, 荣誉记录
use crate::core::*;
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 荣誉积分存储 section
const HONOR_SECTION: &str = "arena_honor";

/// 荣誉商店物品定义
struct HonorItem {
    name: &'static str,
    cost: i32,
    description: &'static str,
    item_type: &'static str,
    quantity: i32,
    min_tier: i32,
}

const HONOR_ITEMS: &[HonorItem] = &[
    // Tier 0: 不限
    HonorItem {
        name: "荣誉生命药水",
        cost: 50,
        description: "恢复5000生命值，竞技专用",
        item_type: "item",
        quantity: 5,
        min_tier: 0,
    },
    HonorItem {
        name: "荣誉魔法药水",
        cost: 50,
        description: "恢复3000魔法值，竞技专用",
        item_type: "item",
        quantity: 5,
        min_tier: 0,
    },
    HonorItem {
        name: "强化石",
        cost: 100,
        description: "用于强化装备的基础材料",
        item_type: "item",
        quantity: 3,
        min_tier: 0,
    },
    HonorItem {
        name: "复活卷轴",
        cost: 200,
        description: "死亡后立即复活，保留全部经验",
        item_type: "item",
        quantity: 1,
        min_tier: 0,
    },
    HonorItem {
        name: "经验加成卡",
        cost: 150,
        description: "使用后30分钟内经验获取+50%",
        item_type: "item",
        quantity: 1,
        min_tier: 0,
    },
    HonorItem {
        name: "金币加成卡",
        cost: 150,
        description: "使用后30分钟内金币获取+50%",
        item_type: "item",
        quantity: 1,
        min_tier: 0,
    },
    // Tier 1: 青铜
    HonorItem {
        name: "重铸石",
        cost: 400,
        description: "重新随机装备附加属性",
        item_type: "item",
        quantity: 2,
        min_tier: 1,
    },
    // Tier 2: 白银
    HonorItem {
        name: "高级强化石",
        cost: 300,
        description: "高等级强化必备材料",
        item_type: "item",
        quantity: 1,
        min_tier: 2,
    },
    HonorItem {
        name: "传承石",
        cost: 500,
        description: "装备传承的必备道具",
        item_type: "item",
        quantity: 1,
        min_tier: 2,
    },
    HonorItem {
        name: "钻石福袋",
        cost: 600,
        description: "随机获得50~200钻石",
        item_type: "currency",
        quantity: 0,
        min_tier: 2,
    },
    // Tier 3: 黄金
    HonorItem {
        name: "竞技战旗",
        cost: 800,
        description: "稀有称号道具，佩戴后PVP伤害+5%",
        item_type: "title",
        quantity: 1,
        min_tier: 3,
    },
    // Tier 4: 铂金
    HonorItem {
        name: "胜利之翼",
        cost: 1500,
        description: "传说称号道具，佩戴后全属性+3%",
        item_type: "title",
        quantity: 1,
        min_tier: 4,
    },
    // Tier 5: 钻石
    HonorItem {
        name: "封印之刃",
        cost: 2000,
        description: "史诗级武器，竞技场专属",
        item_type: "item",
        quantity: 1,
        min_tier: 5,
    },
    HonorItem {
        name: "不朽战铠",
        cost: 2000,
        description: "史诗级铠甲，竞技场专属",
        item_type: "item",
        quantity: 1,
        min_tier: 5,
    },
];

/// 段位等级名
fn tier_name(tier: i32) -> &'static str {
    match tier {
        0 => "不限",
        1 => "青铜",
        2 => "白银",
        3 => "黄金",
        4 => "铂金",
        5 => "钻石",
        _ => "王者",
    }
}

/// 物品类型emoji
fn item_emoji(item_type: &str) -> &'static str {
    match item_type {
        "item" => "📦",
        "title" => "🏅",
        "buff" => "⚡",
        "currency" => "💎",
        _ => "🎫",
    }
}

/// 获取玩家荣誉积分
pub fn get_honor_points(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare(&format!(
        "SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION='{}.points'",
        HONOR_SECTION
    ))
    .ok()
    .and_then(|mut stmt| {
        stmt.query_row(rusqlite::params![user_id], |row| {
            let raw: String = row.get(0).unwrap_or_default();
            Ok(raw.parse().unwrap_or(0i32))
        })
        .ok()
    })
    .unwrap_or(0)
}

/// 设置荣誉积分
fn set_honor_points(db: &Database, user_id: &str, points: i32) {
    let conn = db.lock_conn();
    let key = format!("{}.points", HONOR_SECTION);
    let data = points.to_string();
    let updated = conn
        .execute(
            "UPDATE Shared_Data SET DATA=?1 WHERE ID=?2 AND SECTION=?3",
            rusqlite::params![data, user_id, key],
        )
        .unwrap_or(0);
    if updated == 0 {
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, key, data],
        );
    }
}

/// 增加荣誉积分
#[allow(dead_code)]
pub fn add_honor_points(db: &Database, user_id: &str, amount: i32) {
    let current = get_honor_points(db, user_id);
    set_honor_points(db, user_id, current + amount);
}

/// 获取玩家荣誉兑换记录
fn get_honor_history(db: &Database, user_id: &str) -> Vec<String> {
    let conn = db.lock_conn();
    let key = format!("{}.history", HONOR_SECTION);
    conn.prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION=?2")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id, key], |row| {
                Ok(row.get::<_, String>(0).unwrap_or_default())
            })
            .ok()
        })
        .map(|s| {
            if s.is_empty() {
                vec![]
            } else {
                s.split('|').map(String::from).collect()
            }
        })
        .unwrap_or_default()
}

/// 添加兑换记录
fn add_honor_history(db: &Database, user_id: &str, entry: &str) {
    let mut history = get_honor_history(db, user_id);
    history.push(entry.to_string());
    if history.len() > 20 {
        history = history[history.len() - 20..].to_vec();
    }
    let data = history.join("|");
    let conn = db.lock_conn();
    let key = format!("{}.history", HONOR_SECTION);
    let updated = conn
        .execute(
            "UPDATE Shared_Data SET DATA=?1 WHERE ID=?2 AND SECTION=?3",
            rusqlite::params![data, user_id, key],
        )
        .unwrap_or(0);
    if updated == 0 {
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, key, data],
        );
    }
}

/// 获取玩家竞技段位 (基于匹配积分)
fn get_player_arena_tier(db: &Database, user_id: &str) -> i32 {
    let score = get_match_integral(db, user_id);
    match score {
        0..=99 => 0,
        100..=299 => 1,
        300..=599 => 2,
        600..=999 => 3,
        1000..=1499 => 4,
        1500..=2499 => 5,
        _ => 6,
    }
}

/// 获取匹配积分
fn get_match_integral(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT Integral FROM ext_pipei_uInfo WHERE uID=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id], |row| Ok(row.get::<_, i32>(0).unwrap_or(0)))
                .ok()
        })
        .unwrap_or(0)
}

/// 获取匹配胜场
fn get_match_wins(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT Win FROM ext_pipei_uInfo WHERE uID=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id], |row| Ok(row.get::<_, i32>(0).unwrap_or(0)))
                .ok()
        })
        .unwrap_or(0)
}

/// 获取匹配败场
fn get_match_losses(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT Lose FROM ext_pipei_uInfo WHERE uID=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id], |row| Ok(row.get::<_, i32>(0).unwrap_or(0)))
                .ok()
        })
        .unwrap_or(0)
}

/// 授予荣誉称号
fn grant_honor_title(db: &Database, user_id: &str, title: &str) {
    let key = format!("{}.titles", HONOR_SECTION);
    let existing = db
        .lock_conn()
        .prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION=?2")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id, key], |row| {
                Ok(row.get::<_, String>(0).unwrap_or_default())
            })
            .ok()
        })
        .unwrap_or_default();

    let mut titles: Vec<&str> = if existing.is_empty() {
        vec![]
    } else {
        existing.split(',').collect()
    };
    if !titles.contains(&title) {
        titles.push(title);
    }
    let data = titles.join(",");
    let conn = db.lock_conn();
    let updated = conn
        .execute(
            "UPDATE Shared_Data SET DATA=?1 WHERE ID=?2 AND SECTION=?3",
            rusqlite::params![data, user_id, key],
        )
        .unwrap_or(0);
    if updated == 0 {
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, key, data],
        );
    }
}

/// 授予荣誉buff
fn grant_honor_buff(db: &Database, user_id: &str, buff: &str) {
    let key = format!("{}.buffs", HONOR_SECTION);
    let now = chrono::Local::now().timestamp();
    let entry = format!("{}@{}", buff, now);
    let conn = db.lock_conn();
    let updated = conn
        .execute(
            "UPDATE Shared_Data SET DATA=?1 WHERE ID=?2 AND SECTION=?3",
            rusqlite::params![entry, user_id, key],
        )
        .unwrap_or(0);
    if updated == 0 {
        let _ = conn.execute(
            "INSERT INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, key, entry],
        );
    }
}

/// 获取荣誉称号列表
#[allow(dead_code)]
pub fn get_honor_titles(db: &Database, user_id: &str) -> Vec<String> {
    let key = format!("{}.titles", HONOR_SECTION);
    db.lock_conn()
        .prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION=?2")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_row(rusqlite::params![user_id, key], |row| {
                Ok(row.get::<_, String>(0).unwrap_or_default())
            })
            .ok()
        })
        .map(|s| {
            if s.is_empty() {
                vec![]
            } else {
                s.split(',').map(String::from).collect()
            }
        })
        .unwrap_or_default()
}

// ==================== 指令 ====================

/// 查看荣誉商店
pub fn cmd_honor_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let points = get_honor_points(db, user_id);

    let mut out = format!("{}\n═══ 🏆 竞技荣誉商店 ═══\n\n💰 当前荣誉积分: {}\n\n", prefix, points);

    let mut current_tier = -1i32;
    for item in HONOR_ITEMS {
        if item.min_tier != current_tier {
            current_tier = item.min_tier;
            out.push_str(&format!("\n── {}段位以上 ──\n", tier_name(current_tier)));
        }
        out.push_str(&format!(
            "  {} {} | {}荣誉点 | {}{}\n",
            item_emoji(item.item_type),
            item.name,
            item.cost,
            item.description,
            if item.quantity > 1 {
                format!(" x{}", item.quantity)
            } else {
                String::new()
            }
        ));
    }

    out.push_str("\n\n💡 使用「荣誉兑换+物品名」兑换奖励");
    out.push_str("\n💡 使用「荣誉余额」查看积分详情");
    out.push_str("\n💡 使用「荣誉记录」查看兑换历史");
    out
}

/// 荣誉兑换
pub fn cmd_honor_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        let mut out = format!("{}\n❓ 请指定要兑换的物品名！\n\n可用兑换:\n", prefix);
        for item in HONOR_ITEMS {
            out.push_str(&format!("  • {} ({}荣誉点)\n", item.name, item.cost));
        }
        out.push_str("\n💡 使用「荣誉兑换+物品名」进行兑换");
        return out;
    }

    let matched = HONOR_ITEMS.iter().find(|i| i.name.contains(item_name));
    let item = match matched {
        Some(item) => item,
        None => {
            return format!(
                "{}\n❌ 未找到「{}」的兑换物品！\n💡 使用「荣誉商店」查看可兑换列表。",
                prefix, item_name
            );
        }
    };

    // 检查段位要求
    let player_tier = get_player_arena_tier(db, user_id);
    if player_tier < item.min_tier {
        return format!(
            "{}\n🔒 兑换「{}」需要 {} 段位以上！\n当前段位: {}\n💡 提升竞技匹配排名可提升段位。",
            prefix,
            item.name,
            tier_name(item.min_tier),
            tier_name(player_tier)
        );
    }

    // 检查积分
    let points = get_honor_points(db, user_id);
    if points < item.cost {
        return format!(
            "{}\n💰 荣誉积分不足！\n需要: {} 荣誉点\n当前: {} 荣誉点\n差额: {} 荣誉点\n\n💡 参与竞技匹配获取荣誉积分。",
            prefix,
            item.cost,
            points,
            item.cost - points
        );
    }

    // 扣除积分
    set_honor_points(db, user_id, points - item.cost);

    // 根据物品类型处理
    let result = match item.item_type {
        "item" => {
            db.add_item(user_id, item.name, item.quantity);
            format!("📦 获得: {} x{}", item.name, item.quantity)
        }
        "title" => {
            grant_honor_title(db, user_id, item.name);
            format!("🏅 获得称号: {}", item.name)
        }
        "currency" => {
            let mut rng = rand::thread_rng();
            let diamond: i64 = rng.gen_range(50..=200);
            let cur = db.read_currency(user_id, CURRENCY_DIAMOND);
            db.write_currency(user_id, CURRENCY_DIAMOND, cur + diamond);
            format!("💎 获得: {} 钻石", diamond)
        }
        "buff" => {
            grant_honor_buff(db, user_id, item.name);
            format!("⚡ 获得增益: {}", item.name)
        }
        _ => format!("✅ 兑换成功: {}", item.name),
    };

    // 记录兑换
    let now = chrono::Local::now().format("%m-%d %H:%M");
    add_honor_history(db, user_id, &format!("{}兑换{}(-{}点)", now, item.name, item.cost));

    format!(
        "{}\n═══ 🏆 荣誉兑换成功 ═══\n\n✅ 兑换: {}\n{}\n💰 消耗: {} 荣誉点\n💰 剩余: {} 荣誉点",
        prefix,
        item.name,
        result,
        item.cost,
        points - item.cost
    )
}

/// 荣誉余额
pub fn cmd_honor_balance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let points = get_honor_points(db, user_id);
    let tier = get_player_arena_tier(db, user_id);
    let wins = get_match_wins(db, user_id);
    let losses = get_match_losses(db, user_id);
    let total = wins + losses;
    let winrate = if total > 0 {
        wins as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    format!(
        "{}\n═══ 💰 荣誉积分详情 ═══\n\n\
         🏆 荣誉积分: {}\n\
         🎖 当前段位: {} ({})\n\
         ⚔ 匹配次数: {} (胜{} 负{})\n\
         📊 胜率: {:.1}%\n\n\
         💡 荣誉积分获取方式:\n\
         • 匹配胜利: +20 荣誉\n\
         • 匹配失败: +5 荣誉\n\
         • 赛季结算: 按排名发放\n\
         • 竞技锦标赛: 按名次发放\n\n\
         💡 使用「荣誉商店」查看可兑换奖励",
        prefix,
        points,
        tier_name(tier),
        tier,
        total,
        wins,
        losses,
        winrate
    )
}

/// 荣誉记录
pub fn cmd_honor_history_cmd(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let history = get_honor_history(db, user_id);

    let mut out = format!("{}\n═══ 📜 荣誉兑换记录 ═══\n\n", prefix);

    if history.is_empty() {
        out.push_str("暂无兑换记录\n");
    } else {
        for (i, entry) in history.iter().rev().enumerate() {
            out.push_str(&format!("  {}. {}\n", i + 1, entry));
        }
    }

    out.push_str("\n💡 使用「荣誉商店」查看可兑换奖励");
    out
}

/// 匹配胜利后自动发放荣誉积分
#[allow(dead_code)]
pub fn on_match_victory(db: &Database, user_id: &str) {
    add_honor_points(db, user_id, 20);
}

/// 匹配失败后发放少量荣誉积分
#[allow(dead_code)]
pub fn on_match_defeat(db: &Database, user_id: &str) {
    add_honor_points(db, user_id, 5);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_honor_items_count() {
        assert!(HONOR_ITEMS.len() >= 10);
    }

    #[test]
    fn test_honor_items_unique_names() {
        let mut names: Vec<&str> = HONOR_ITEMS.iter().map(|i| i.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate honor item names");
    }

    #[test]
    fn test_honor_items_costs_positive() {
        for item in HONOR_ITEMS {
            assert!(item.cost > 0, "{} has non-positive cost", item.name);
        }
    }

    #[test]
    fn test_honor_items_costs_escalate() {
        let tier0_max = HONOR_ITEMS
            .iter()
            .filter(|i| i.min_tier == 0)
            .map(|i| i.cost)
            .max()
            .unwrap_or(0);
        let tier5_min = HONOR_ITEMS
            .iter()
            .filter(|i| i.min_tier == 5)
            .map(|i| i.cost)
            .min()
            .unwrap_or(i32::MAX);
        assert!(tier5_min >= tier0_max);
    }

    #[test]
    fn test_tier_names() {
        assert_eq!(tier_name(0), "不限");
        assert_eq!(tier_name(1), "青铜");
        assert_eq!(tier_name(2), "白银");
        assert_eq!(tier_name(3), "黄金");
        assert_eq!(tier_name(4), "铂金");
        assert_eq!(tier_name(5), "钻石");
        assert_eq!(tier_name(6), "王者");
        assert_eq!(tier_name(99), "王者");
    }

    #[test]
    fn test_item_emoji() {
        assert_eq!(item_emoji("item"), "📦");
        assert_eq!(item_emoji("title"), "🏅");
        assert_eq!(item_emoji("buff"), "⚡");
        assert_eq!(item_emoji("currency"), "💎");
    }

    #[test]
    fn test_honor_items_have_descriptions() {
        for item in HONOR_ITEMS {
            assert!(!item.description.is_empty(), "{} missing description", item.name);
        }
    }

    #[test]
    fn test_honor_items_valid_types() {
        let valid = ["item", "title", "buff", "currency"];
        for item in HONOR_ITEMS {
            assert!(
                valid.contains(&item.item_type),
                "{} has invalid type: {}",
                item.name,
                item.item_type
            );
        }
    }

    #[test]
    fn test_honor_section_constant() {
        assert_eq!(HONOR_SECTION, "arena_honor");
    }

    #[test]
    fn test_honor_items_sorted_by_tier() {
        // Items should be grouped by tier (non-decreasing)
        let mut last_tier = -1i32;
        for item in HONOR_ITEMS {
            assert!(
                item.min_tier >= last_tier,
                "Items not sorted by tier: {} has tier {} after tier {}",
                item.name,
                item.min_tier,
                last_tier
            );
            last_tier = item.min_tier;
        }
    }

    #[test]
    fn test_honor_items_quantity_positive_for_items() {
        for item in HONOR_ITEMS {
            if item.item_type == "item" {
                assert!(
                    item.quantity > 0,
                    "{} is item type but quantity is {}",
                    item.name,
                    item.quantity
                );
            }
        }
    }
}

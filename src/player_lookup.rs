/// CakeGame 玩家信息查询系统
/// 允许玩家查看其他玩家的公开资料信息
/// 包括等级、职业、公会、装备评分、段位等公开数据
use crate::core::*;
use crate::db::Database;

/// 查询玩家信息
/// 支持昵称精确匹配或用户ID精确匹配
pub fn cmd_player_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let target_name = args.trim();
    if target_name.is_empty() {
        return "📝 请输入要查询的玩家昵称或ID\n\n💡 格式：查看玩家+昵称\n📝 示例：查看玩家+张三".to_string();
    }

    // 查找目标用户：先尝试ID精确匹配，再尝试昵称模糊匹配
    let target_id = match find_user_by_name_or_id(db, target_name) {
        Some(id) => id,
        None => return format!("❌ 未找到玩家「{}」\n💡 请输入正确的昵称或用户ID", target_name),
    };

    // 自己查看自己
    let is_self = target_id == user_id;

    // 读取基础信息
    let nickname = db.read_basic(&target_id, ITEM_NAME);
    if nickname.is_empty() {
        return "❌ 玩家数据异常".to_string();
    }

    let level: i32 = db.read_basic(&target_id, ITEM_LEVEL).parse().unwrap_or(1);
    let occupation = db.read_basic(&target_id, ITEM_OCCUPATION);
    let location = db.read_basic(&target_id, ITEM_LOCATION);
    let guild = db.read_basic(&target_id, ITEM_GUILD);
    let exp: i64 = db.read_basic(&target_id, ITEM_EXP).parse().unwrap_or(0);
    let exp_need: i64 = db.read_basic(&target_id, ITEM_EXP_NEED).parse().unwrap_or(100);

    // 基础属性
    let hp: i32 = db.read_basic(&target_id, ITEM_HP).parse().unwrap_or(0);
    let ad: i32 = db.read_basic(&target_id, ITEM_AD).parse().unwrap_or(0);
    let ap: i32 = db.read_basic(&target_id, ITEM_AP).parse().unwrap_or(0);
    let defense: i32 = db.read_basic(&target_id, ITEM_DEFENSE).parse().unwrap_or(0);
    let magic_res: i32 = db.read_basic(&target_id, ITEM_MAGIC_RES).parse().unwrap_or(0);

    // 货币
    let gold: i64 = read_currency(db, &target_id, CURRENCY_GOLD);
    let diamond: i64 = read_currency(db, &target_id, CURRENCY_DIAMOND);

    // 职业图标
    let occ_icon = occupation_icon(&occupation);

    // 等级称号
    let level_title = get_level_title(level);

    // 经验进度
    let exp_pct = if exp_need > 0 {
        (exp as f64 / exp_need as f64 * 100.0).min(100.0)
    } else {
        0.0
    };
    let exp_bar = progress_bar(exp_pct, 10);

    // 计算战力评分
    let combat_power = calc_display_combat_power(hp, ad, ap, defense, magic_res);

    let mut result = String::new();
    result.push_str("👤 === 玩家信息查询 ===\n\n");
    result.push_str(&format!("📛 昵称：{}\n", nickname));
    result.push_str(&format!("🎯 等级：Lv.{} {}\n", level, level_title));
    result.push_str(&format!("🎭 职业：{} {}\n", occupation, occ_icon));
    result.push_str(&format!("📍 位置：{}\n", location));

    if !guild.is_empty() && guild != "[NULL]" {
        result.push_str(&format!("🏛️ 公会：{}\n", guild));
    }

    result.push_str("\n📊 === 属性概览 ===\n");
    result.push_str(&format!("❤️ 生命：{}\n", format_number(hp as i64)));
    result.push_str(&format!("⚔️ 物攻：{}\n", format_number(ad as i64)));
    result.push_str(&format!("🔮 魔攻：{}\n", format_number(ap as i64)));
    result.push_str(&format!("🛡️ 防御：{}\n", format_number(defense as i64)));
    result.push_str(&format!("✨ 魔抗：{}\n", format_number(magic_res as i64)));

    result.push_str(&format!("\n💪 战力评估：{}\n", format_number(combat_power)));
    result.push_str(&format!(
        "📈 经验：{}/{} ({}%)\n{}\n",
        format_number(exp),
        format_number(exp_need),
        exp_pct as i32,
        exp_bar
    ));

    // 货币信息（自己显示详细，他人只显示等级段位）
    if is_self {
        result.push_str("\n💰 === 资产 ===\n");
        result.push_str(&format!("🪙 金币：{}\n", format_number(gold)));
        result.push_str(&format!("💎 钻石：{}\n", format_number(diamond)));
    } else {
        let wealth_tier = get_wealth_tier(gold, diamond);
        result.push_str(&format!("\n💰 财富等级：{}\n", wealth_tier));
    }

    // 装备概览
    result.push_str("\n🎒 === 装备概览 ===\n");
    let slots = [
        ("武器", SLOT_WEAPON),
        ("头盔", SLOT_HELMET),
        ("铠甲", SLOT_ARMOR),
        ("护腿", SLOT_LEG),
        ("靴子", SLOT_BOOTS),
        ("项链", SLOT_NECKLACE),
        ("戒指", SLOT_RING),
        ("翅膀", SLOT_WING),
    ];
    let mut equipped_count = 0;
    for (label, slot) in &slots {
        let item = get_equipped_item(db, &target_id, slot);
        if !item.is_empty() && item != "[NULL]" {
            let quality_icon = get_quality_icon(&item);
            result.push_str(&format!("  {} {}: {}\n", quality_icon, label, item));
            equipped_count += 1;
        }
    }
    if equipped_count == 0 {
        result.push_str("  (暂无装备)\n");
    }

    if is_self {
        result.push_str("\n💡 这是你自己的公开资料\n");
    }

    result
}

/// 批量查看同地图玩家简要信息
pub fn cmd_nearby_players(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let location = db.read_basic(user_id, ITEM_LOCATION);

    // 获取同地图玩家
    let conn = db.lock_conn();
    let mut stmt =
        match conn.prepare("SELECT ID, Data FROM Basic_User WHERE Node=?1 AND Item=?2 AND Data=?3 AND ID != ?4") {
            Ok(s) => s,
            Err(_) => return "❌ 查询失败".to_string(),
        };

    let mut players = Vec::new();
    if let Ok(rows) = stmt.query_map(rusqlite::params![NODE_BASIC, ITEM_LOCATION, location, user_id], |row| {
        let uid: String = row.get(0)?;
        Ok(uid)
    }) {
        for row in rows.flatten().take(20) {
            let name = db.read_basic(&row, ITEM_NAME);
            let lvl: i32 = db.read_basic(&row, ITEM_LEVEL).parse().unwrap_or(1);
            let occ = db.read_basic(&row, ITEM_OCCUPATION);
            if !name.is_empty() {
                players.push((name, lvl, occ));
            }
        }
    }

    let mut result = format!("📍 === {} 的周围玩家 ===\n\n", location);

    if players.is_empty() {
        result.push_str("当前地图没有其他玩家\n");
    } else {
        result.push_str(&format!("共 {} 名玩家：\n\n", players.len()));
        for (i, (name, lvl, occ)) in players.iter().enumerate() {
            let occ_icon = occupation_icon(occ);
            result.push_str(&format!("{}. {} Lv.{} {} {}\n", i + 1, name, lvl, occ, occ_icon));
        }
    }

    result.push_str("\n💡 使用「查看玩家+昵称」查看详细资料\n");

    result
}

// ==================== 辅助函数 ====================

/// 通过昵称或ID查找用户
fn find_user_by_name_or_id(db: &Database, query: &str) -> Option<String> {
    let conn = db.lock_conn();

    // 先尝试ID精确匹配
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM Basic_User WHERE ID=?1 AND Node=?2",
            rusqlite::params![query, NODE_BASIC],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let exists = exists > 0;
    if exists {
        return Some(query.to_string());
    }

    // 再尝试昵称精确匹配
    let result: Result<String, _> = conn.query_row(
        "SELECT ID FROM Basic_User WHERE Node=?1 AND Item=?2 AND Data=?3",
        rusqlite::params![NODE_BASIC, ITEM_NAME, query],
        |row| row.get(0),
    );
    if let Ok(id) = result {
        return Some(id);
    }

    // 最后尝试昵称模糊匹配
    let mut stmt = conn
        .prepare("SELECT ID, Data FROM Basic_User WHERE Node=?1 AND Item=?2 AND Data LIKE '%' || ?3 || '%'")
        .ok()?;
    let mut rows = stmt
        .query_map(rusqlite::params![NODE_BASIC, ITEM_NAME, query], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            Ok((id, name))
        })
        .ok()?;

    rows.next().map(|r| r.unwrap().0)
}

/// 读取货币
fn read_currency(db: &Database, user_id: &str, currency: &str) -> i64 {
    let conn = db.lock_conn();
    conn.query_row(
        "SELECT Data FROM Basic_User WHERE ID=?1 AND Node=?2 AND Item=?3",
        rusqlite::params![user_id, NODE_CURRENCY, currency],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_default()
    .parse()
    .unwrap_or(0)
}

/// 获取装备的物品名
fn get_equipped_item(db: &Database, user_id: &str, slot: &str) -> String {
    let conn = db.lock_conn();
    conn.query_row(
        "SELECT Data FROM Basic_User WHERE ID=?1 AND Node=?2 AND Item=?3",
        rusqlite::params![user_id, NODE_EQUIP, slot],
        |row| row.get::<_, String>(0),
    )
    .unwrap_or_default()
}

/// 职业图标
fn occupation_icon(occupation: &str) -> &'static str {
    match occupation {
        "勇者" => "⚔️",
        "御剑师" => "🗡️",
        "魔法师" => "🔮",
        "弓箭手" => "🏹",
        "治愈师" => "💚",
        "暗影" => "🌑",
        _ => "❓",
    }
}

/// 获取品质图标
fn get_quality_icon(item_name: &str) -> &'static str {
    if item_name.contains("圣物") {
        "🔴"
    } else if item_name.contains("传说") {
        "🟡"
    } else if item_name.contains("超界") {
        "🌈"
    } else if item_name.contains("史诗") {
        "🟣"
    } else if item_name.contains("完美") {
        "🔵"
    } else if item_name.contains("稀有") {
        "🔷"
    } else if item_name.contains("精良") {
        "🟢"
    } else {
        "⚪"
    }
}

/// 获取等级称号
fn get_level_title(level: i32) -> &'static str {
    match level {
        1..=10 => "初入江湖",
        11..=20 => "小有名气",
        21..=30 => "崭露头角",
        31..=40 => "名动一方",
        41..=50 => "声名远播",
        51..=60 => "威震四海",
        61..=70 => "一代宗师",
        71..=80 => "绝世强者",
        81..=90 => "传奇霸主",
        91..=100 => "至高神域",
        _ => "凡人",
    }
}

/// 财富等级
fn get_wealth_tier(gold: i64, diamond: i64) -> String {
    let total = gold + diamond * 1000;
    if total >= 10_000_000 {
        "👑 富甲天下".to_string()
    } else if total >= 5_000_000 {
        "💰 腰缠万贯".to_string()
    } else if total >= 1_000_000 {
        "🪙 小有资产".to_string()
    } else if total >= 100_000 {
        "📊 温饱有余".to_string()
    } else if total >= 10_000 {
        "🪙 勉强度日".to_string()
    } else {
        "😅 一贫如洗".to_string()
    }
}

/// 计算战力评分（简化版）
fn calc_display_combat_power(hp: i32, ad: i32, ap: i32, def: i32, mr: i32) -> i64 {
    (hp as i64 * 3) + (ad as i64 * 5) + (ap as i64 * 5) + (def as i64 * 4) + (mr as i64 * 4)
}

/// 格式化数字（千分位）
fn format_number(n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// 进度条
fn progress_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("[{}{}] {:.1}%", "█".repeat(filled), "░".repeat(empty), pct)
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_title_all_ranges() {
        assert_eq!(get_level_title(1), "初入江湖");
        assert_eq!(get_level_title(15), "小有名气");
        assert_eq!(get_level_title(25), "崭露头角");
        assert_eq!(get_level_title(35), "名动一方");
        assert_eq!(get_level_title(45), "声名远播");
        assert_eq!(get_level_title(55), "威震四海");
        assert_eq!(get_level_title(65), "一代宗师");
        assert_eq!(get_level_title(75), "绝世强者");
        assert_eq!(get_level_title(85), "传奇霸主");
        assert_eq!(get_level_title(95), "至高神域");
        assert_eq!(get_level_title(0), "凡人");
    }

    #[test]
    fn test_wealth_tier_ordering() {
        assert!(get_wealth_tier(0, 0).contains("一贫如洗"));
        assert!(get_wealth_tier(15000, 0).contains("勉强度日"));
        assert!(get_wealth_tier(150000, 0).contains("温饱有余"));
        assert!(get_wealth_tier(1500000, 0).contains("小有资产"));
        assert!(get_wealth_tier(6000000, 0).contains("腰缠万贯"));
        assert!(get_wealth_tier(12000000, 0).contains("富甲天下"));
    }

    #[test]
    fn test_quality_icon_all_tiers() {
        assert_eq!(get_quality_icon("圣物武器"), "🔴");
        assert_eq!(get_quality_icon("传说之剑"), "🟡");
        assert_eq!(get_quality_icon("超界装备"), "🌈");
        assert_eq!(get_quality_icon("史诗碎片"), "🟣");
        assert_eq!(get_quality_icon("完美铠甲"), "🔵");
        assert_eq!(get_quality_icon("稀有药水"), "🔷");
        assert_eq!(get_quality_icon("精良靴子"), "🟢");
        assert_eq!(get_quality_icon("普通药水"), "⚪");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(100000000), "100,000,000");
    }

    #[test]
    fn test_combat_power_formula() {
        let cp = calc_display_combat_power(100, 50, 30, 20, 10);
        assert_eq!(cp, 100 * 3 + 50 * 5 + 30 * 5 + 20 * 4 + 10 * 4);
        assert_eq!(calc_display_combat_power(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(50.0, 10);
        assert!(bar.contains("50.0%"));
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));

        let full = progress_bar(100.0, 5);
        assert!(full.contains("100.0%"));

        let empty = progress_bar(0.0, 5);
        assert!(empty.contains("0.0%"));
    }

    #[test]
    fn test_occupation_icon() {
        assert_eq!(occupation_icon("勇者"), "⚔️");
        assert_eq!(occupation_icon("魔法师"), "🔮");
        assert_eq!(occupation_icon("未知"), "❓");
    }
}

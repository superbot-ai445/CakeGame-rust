use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD};
use crate::db::Database;
use crate::user;

const SECTION: &str = "guild_buildings";

/// Building definition
#[allow(dead_code)]
struct BuildingDef {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    description: &'static str,
    /// 5 levels of gold cost (level 1-5)
    gold_costs: &'static [i64],
    /// 5 levels of guild fund cost
    fund_costs: &'static [i64],
    /// 5 levels of diamond cost
    diamond_costs: &'static [i64],
    /// 5 levels of effect description
    effect_descs: &'static [&'static str],
    /// Effect type: "atk_pct", "def_pct", "hp_pct", "exp_pct", "gold_pct", "gather_pct", "crit_pct", "all_pct"
    effect_type: &'static str,
    /// Effect values per level (percentage)
    effect_values: &'static [i32],
}

const BUILDINGS: &[BuildingDef] = &[
    BuildingDef {
        id: "training_ground",
        name: "训练场",
        emoji: "🏋️",
        description: "提升公会成员攻击力",
        gold_costs: &[5000, 15000, 40000, 100000, 250000],
        fund_costs: &[1000, 3000, 8000, 20000, 50000],
        diamond_costs: &[0, 50, 150, 400, 1000],
        effect_descs: &["攻击+2%", "攻击+4%", "攻击+7%", "攻击+10%", "攻击+15%"],
        effect_type: "atk_pct",
        effect_values: &[2, 4, 7, 10, 15],
    },
    BuildingDef {
        id: "smithy",
        name: "铁匠铺",
        emoji: "🔨",
        description: "提升公会成员防御力",
        gold_costs: &[5000, 15000, 40000, 100000, 250000],
        fund_costs: &[1000, 3000, 8000, 20000, 50000],
        diamond_costs: &[0, 50, 150, 400, 1000],
        effect_descs: &["防御+2%", "防御+4%", "防御+7%", "防御+10%", "防御+15%"],
        effect_type: "def_pct",
        effect_values: &[2, 4, 7, 10, 15],
    },
    BuildingDef {
        id: "alchemy_room",
        name: "药剂房",
        emoji: "🧪",
        description: "提升公会成员生命值",
        gold_costs: &[4000, 12000, 30000, 80000, 200000],
        fund_costs: &[800, 2500, 6000, 15000, 40000],
        diamond_costs: &[0, 40, 120, 300, 800],
        effect_descs: &["HP+3%", "HP+6%", "HP+10%", "HP+15%", "HP+20%"],
        effect_type: "hp_pct",
        effect_values: &[3, 6, 10, 15, 20],
    },
    BuildingDef {
        id: "treasury",
        name: "宝库",
        emoji: "🏦",
        description: "提升公会成员金币获取",
        gold_costs: &[8000, 25000, 60000, 150000, 400000],
        fund_costs: &[1500, 4000, 10000, 25000, 60000],
        diamond_costs: &[0, 60, 180, 500, 1200],
        effect_descs: &["金币+3%", "金币+6%", "金币+10%", "金币+15%", "金币+20%"],
        effect_type: "gold_pct",
        effect_values: &[3, 6, 10, 15, 20],
    },
    BuildingDef {
        id: "wall",
        name: "城墙",
        emoji: "🏰",
        description: "提升公会成员魔抗",
        gold_costs: &[6000, 18000, 45000, 110000, 280000],
        fund_costs: &[1200, 3500, 9000, 22000, 55000],
        diamond_costs: &[0, 55, 160, 420, 1050],
        effect_descs: &["魔抗+2%", "魔抗+5%", "魔抗+8%", "魔抗+12%", "魔抗+18%"],
        effect_type: "mr_pct",
        effect_values: &[2, 5, 8, 12, 18],
    },
    BuildingDef {
        id: "altar",
        name: "祭坛",
        emoji: "⛩️",
        description: "提升公会成员经验获取",
        gold_costs: &[6000, 18000, 45000, 120000, 300000],
        fund_costs: &[1200, 3500, 9000, 22000, 55000],
        diamond_costs: &[0, 50, 150, 400, 1000],
        effect_descs: &["经验+5%", "经验+10%", "经验+15%", "经验+22%", "经验+30%"],
        effect_type: "exp_pct",
        effect_values: &[5, 10, 15, 22, 30],
    },
    BuildingDef {
        id: "tavern",
        name: "酒馆",
        emoji: "🍺",
        description: "提升公会成员暴击率",
        gold_costs: &[7000, 20000, 50000, 130000, 320000],
        fund_costs: &[1300, 3800, 9500, 24000, 58000],
        diamond_costs: &[0, 55, 165, 430, 1100],
        effect_descs: &["暴击+1%", "暴击+2%", "暴击+3%", "暴击+5%", "暴击+8%"],
        effect_type: "crit_pct",
        effect_values: &[1, 2, 3, 5, 8],
    },
    BuildingDef {
        id: "watchtower",
        name: "瞭望塔",
        emoji: "🗼",
        description: "提升公会成员采集效率",
        gold_costs: &[4000, 12000, 30000, 80000, 200000],
        fund_costs: &[800, 2500, 6000, 15000, 40000],
        diamond_costs: &[0, 40, 120, 300, 800],
        effect_descs: &["采集+5%", "采集+10%", "采集+15%", "采集+22%", "采集+30%"],
        effect_type: "gather_pct",
        effect_values: &[5, 10, 15, 22, 30],
    },
];

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

fn guild_section(guild_name: &str) -> String {
    format!("{}_{}", SECTION, guild_name)
}

fn get_building_level(db: &Database, guild_name: &str, building_id: &str) -> i32 {
    let section = guild_section(guild_name);
    let val = db.global_get(&section, &format!("lv_{}", building_id));
    val.parse::<i32>().unwrap_or(0)
}

fn set_building_level(db: &Database, guild_name: &str, building_id: &str, level: i32) {
    let section = guild_section(guild_name);
    db.global_set(&section, &format!("lv_{}", building_id), &level.to_string());
}

fn get_guild_fund(db: &Database, guild_name: &str) -> i64 {
    let section = guild_section(guild_name);
    db.global_get(&section, "fund").parse::<i64>().unwrap_or(0)
}

fn set_guild_fund(db: &Database, guild_name: &str, fund: i64) {
    let section = guild_section(guild_name);
    db.global_set(&section, "fund", &fund.to_string());
}

fn get_guild_name(db: &Database, user_id: &str) -> Option<String> {
    let name = db.global_get("user_guild", user_id);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn is_guild_leader(db: &Database, user_id: &str, guild_name: &str) -> bool {
    let leader = db.global_get(&format!("guild_{}", guild_name), "leader");
    leader == user_id
}

fn find_building(name: &str) -> Option<&'static BuildingDef> {
    BUILDINGS.iter().find(|b| b.id == name || b.name == name).or_else(|| {
        BUILDINGS
            .iter()
            .find(|b| b.name.contains(name) || name.contains(b.name))
    })
}

/// Get total building bonus for a guild member's effect_type
/// Returns percentage bonus (0 if no building or level 0)
#[allow(dead_code)]
pub fn get_guild_building_bonus(db: &Database, user_id: &str, effect_type: &str) -> i32 {
    let guild_name = match get_guild_name(db, user_id) {
        Some(g) => g,
        None => return 0,
    };
    let mut total = 0i32;
    for b in BUILDINGS {
        if b.effect_type == effect_type {
            let level = get_building_level(db, &guild_name, b.id);
            if level > 0 && level as usize <= b.effect_values.len() {
                total += b.effect_values[(level - 1) as usize];
            }
        }
    }
    total
}

/// 查看公会建筑 — 列出所有建筑及等级
pub fn cmd_view_buildings(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let guild_name = match get_guild_name(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您还未加入公会！", prefix),
    };
    let fund = get_guild_fund(db, &guild_name);
    let mut r = format!(
        "{}\n═══ 🏗️ 公会建筑 ═══\n公会: {}\n公会资金: {} 金币\n",
        prefix,
        guild_name,
        format_gold(fund)
    );
    let mut total_level = 0i32;
    for b in BUILDINGS {
        let level = get_building_level(db, &guild_name, b.id);
        total_level += level;
        let level_bar = if level == 0 {
            "⬜⬜⬜⬜⬜".to_string()
        } else {
            format!("{}{}", "🟩".repeat(level as usize), "⬜".repeat(5 - level as usize))
        };
        let effect = if level > 0 {
            b.effect_descs[(level - 1) as usize]
        } else {
            "未建造"
        };
        r.push_str(&format!(
            "\n{} {} Lv.{}/5\n   {} | {}\n   {}\n",
            b.emoji, b.name, level, level_bar, effect, b.description
        ));
    }
    r.push_str(&format!("\n📊 建筑总等级: {}/40", total_level));
    r
}

/// 查看建筑详情 — 显示升级消耗
pub fn cmd_building_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let guild_name = match get_guild_name(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您还未加入公会！", prefix),
    };
    let building = match find_building(args.trim()) {
        Some(b) => b,
        None => {
            let mut r = format!("{}\n❌ 未找到建筑「{}」\n\n可用建筑:", prefix, args.trim());
            for b in BUILDINGS {
                r.push_str(&format!("\n  {} {}", b.emoji, b.name));
            }
            return r;
        }
    };
    let level = get_building_level(db, &guild_name, building.id);
    let mut r = format!(
        "{}\n═══ {} {} 详情 ═══\n等级: {}/5\n{}\n",
        prefix, building.emoji, building.name, level, building.description
    );
    // Show all levels
    for i in 0..5usize {
        let lv = (i + 1) as i32;
        let status = if level >= lv {
            "✅"
        } else if level + 1 == lv {
            "➡️"
        } else {
            "🔒"
        };
        r.push_str(&format!(
            "\n{} Lv.{}: {} | 金币:{} 公会资金:{} 钻石:{}",
            status,
            lv,
            building.effect_descs[i],
            format_gold(building.gold_costs[i]),
            format_gold(building.fund_costs[i]),
            building.diamond_costs[i],
        ));
    }
    if level >= 5 {
        r.push_str("\n\n🎉 已达最高等级！");
    }
    r
}

/// 升级建筑 — 消耗金币+公会资金+钻石
pub fn cmd_upgrade_building(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let guild_name = match get_guild_name(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您还未加入公会！", prefix),
    };
    if !is_guild_leader(db, user_id, &guild_name) {
        return format!("{}\n❌ 只有会长才能升级公会建筑！", prefix);
    }
    let building = match find_building(args.trim()) {
        Some(b) => b,
        None => {
            return format!(
                "{}\n❌ 未找到建筑「{}」\n\n请输入建筑名称，如: 升级建筑 训练场",
                prefix,
                args.trim()
            );
        }
    };
    let level = get_building_level(db, &guild_name, building.id);
    if level >= 5 {
        return format!("{}\n{} {} 已达到最高等级5级！", prefix, building.emoji, building.name);
    }
    let idx = level as usize;
    let gold_cost = building.gold_costs[idx];
    let fund_cost = building.fund_costs[idx];
    let diamond_cost = building.diamond_costs[idx];

    // Check guild fund
    let fund = get_guild_fund(db, &guild_name);
    if fund < fund_cost {
        return format!(
            "{}\n❌ 公会资金不足！需要{}，当前{}",
            prefix,
            format_gold(fund_cost),
            format_gold(fund)
        );
    }
    // Check player gold
    let gold_balance = db.modify_currency(user_id, CURRENCY_GOLD, "add", 0);
    if gold_balance < gold_cost {
        return format!(
            "{}\n❌ 金币不足！需要{}，当前{}",
            prefix,
            format_gold(gold_cost),
            format_gold(gold_balance)
        );
    }
    // Check player diamond
    if diamond_cost > 0 {
        let diamond_balance = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", 0);
        if diamond_balance < diamond_cost {
            return format!("{}\n❌ 钻石不足！需要{}，当前{}", prefix, diamond_cost, diamond_balance);
        }
        let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", diamond_cost);
    }
    // Deduct gold
    let after_gold = db.modify_currency(user_id, CURRENCY_GOLD, "sub", gold_cost);
    if after_gold < 0 {
        // Rollback
        let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", gold_cost);
        if diamond_cost > 0 {
            let _ = db.modify_currency(user_id, CURRENCY_DIAMOND, "add", diamond_cost);
        }
        return format!("{}\n❌ 扣除金币失败！", prefix);
    }
    // Deduct guild fund
    set_guild_fund(db, &guild_name, fund - fund_cost);

    // Upgrade
    let new_level = level + 1;
    set_building_level(db, &guild_name, building.id, new_level);

    format!(
        "{}\n🎉 {} {} 升级成功！\n\n等级: {} → {}\n新效果: {}\n消耗: {}金币 + {}公会资金 + {}钻石",
        prefix,
        building.emoji,
        building.name,
        level,
        new_level,
        building.effect_descs[idx],
        format_gold(gold_cost),
        format_gold(fund_cost),
        diamond_cost,
    )
}

/// 公会建筑排行 — 按建筑总等级排名
pub fn cmd_building_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // Collect all guild sections
    let guild_data: Vec<(String, i32)> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare(&format!(
            "SELECT DISTINCT SECTION FROM Global WHERE SECTION LIKE '{}_%'",
            SECTION
        )) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    let guild = row.replace(&format!("{}_", SECTION), "");
                    let mut total = 0i32;
                    for b in BUILDINGS {
                        let section = format!("{}_{}", SECTION, guild);
                        let val: i32 = db.global_get(&section, &format!("lv_{}", b.id)).parse().unwrap_or(0);
                        total += val;
                    }
                    if total > 0 {
                        result.push((guild, total));
                    }
                }
            }
        }
        result
    };

    if guild_data.is_empty() {
        return format!("{}\n📊 暂无公会建筑数据", prefix);
    }

    let mut entries = guild_data;
    entries.sort_by_key(|b| std::cmp::Reverse(b.1));

    let medals = ["🥇", "🥈", "🥉"];
    let mut r = format!("{}\n═══ 🏗️ 公会建筑排行 ═══\n", prefix);
    for (i, (guild, total)) in entries.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let pct = total * 100 / 40;
        r.push_str(&format!("\n{} {} — 总等级 {}/40 ({}%)", medal, guild, total, pct));
    }

    // User position
    if let Some(user_guild) = get_guild_name(db, user_id) {
        if let Some(rank) = entries.iter().position(|(g, _)| *g == user_guild) {
            r.push_str(&format!("\n\n📍 你的公会「{}」: 第{}名", user_guild, rank + 1));
        }
    }

    r
}

/// 捐献公会资金 — 金币转化为公会建筑资金
pub fn cmd_donate_building_fund(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let guild_name = match get_guild_name(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您还未加入公会！", prefix),
    };
    let amount: i64 = args.trim().parse().unwrap_or(0);
    if amount <= 0 {
        return format!(
            "{}\n请指定捐献金额，如: 捐献资金 1000\n\n当前公会资金: {}",
            prefix,
            format_gold(get_guild_fund(db, &guild_name))
        );
    }
    if amount < 100 {
        return format!("{}\n❌ 最低捐献金额为100金币！", prefix);
    }
    // Check gold
    let after = db.modify_currency(user_id, CURRENCY_GOLD, "sub", amount);
    if after < 0 {
        let _ = db.modify_currency(user_id, CURRENCY_GOLD, "add", amount);
        return format!(
            "{}\n❌ 金币不足！需要{}，当前{}",
            prefix,
            format_gold(amount),
            format_gold(after + amount)
        );
    }
    // Add to guild fund (1:1 ratio)
    let fund = get_guild_fund(db, &guild_name);
    set_guild_fund(db, &guild_name, fund + amount);

    format!(
        "{}\n💰 捐献成功！\n\n捐献: {} 金币\n公会资金: {} → {}\n\n感谢你的贡献！",
        prefix,
        format_gold(amount),
        format_gold(fund),
        format_gold(fund + amount),
    )
}

/// 建筑效果总览 — 显示公会所有活跃加成
pub fn cmd_building_effects(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }
    let guild_name = match get_guild_name(db, user_id) {
        Some(g) => g,
        None => return format!("{}\n❌ 您还未加入公会！", prefix),
    };

    let mut r = format!("{}\n═══ ✨ 公会建筑加成 ═══\n公会: {}\n", prefix, guild_name);

    let mut has_effect = false;
    for b in BUILDINGS {
        let level = get_building_level(db, &guild_name, b.id);
        if level > 0 {
            has_effect = true;
            let effect = b.effect_descs[(level - 1) as usize];
            r.push_str(&format!("\n{} {} Lv.{}: {}", b.emoji, b.name, level, effect));
        }
    }

    if !has_effect {
        r.push_str("\n\n暂无建筑加成，请先建造或升级建筑。");
    }

    r.push_str("\n\n💡 建筑加成对所有公会成员生效\n   会长可在「公会建筑」中升级建筑");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buildings_count() {
        assert_eq!(BUILDINGS.len(), 8);
    }

    #[test]
    fn test_building_ids_unique() {
        let mut ids: Vec<&str> = BUILDINGS.iter().map(|b| b.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BUILDINGS.len());
    }

    #[test]
    fn test_building_names_unique() {
        let mut names: Vec<&str> = BUILDINGS.iter().map(|b| b.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), BUILDINGS.len());
    }

    #[test]
    fn test_cost_arrays_length() {
        for b in BUILDINGS {
            assert_eq!(b.gold_costs.len(), 5, "{} gold_costs should have 5 entries", b.id);
            assert_eq!(b.fund_costs.len(), 5, "{} fund_costs should have 5 entries", b.id);
            assert_eq!(b.diamond_costs.len(), 5, "{} diamond_costs should have 5 entries", b.id);
            assert_eq!(b.effect_descs.len(), 5, "{} effect_descs should have 5 entries", b.id);
            assert_eq!(b.effect_values.len(), 5, "{} effect_values should have 5 entries", b.id);
        }
    }

    #[test]
    fn test_costs_escalate() {
        for b in BUILDINGS {
            for i in 1..5 {
                assert!(
                    b.gold_costs[i] > b.gold_costs[i - 1],
                    "{} gold cost should escalate at level {}",
                    b.id,
                    i + 1
                );
                assert!(
                    b.fund_costs[i] > b.fund_costs[i - 1],
                    "{} fund cost should escalate at level {}",
                    b.id,
                    i + 1
                );
            }
        }
    }

    #[test]
    fn test_effects_positive() {
        for b in BUILDINGS {
            for v in b.effect_values {
                assert!(*v > 0, "{} effect values should be positive", b.id);
            }
        }
    }

    #[test]
    fn test_effects_escalate() {
        for b in BUILDINGS {
            for i in 1..5 {
                assert!(
                    b.effect_values[i] > b.effect_values[i - 1],
                    "{} effect should escalate at level {}",
                    b.id,
                    i + 1
                );
            }
        }
    }

    #[test]
    fn test_emojis_non_empty() {
        for b in BUILDINGS {
            assert!(!b.emoji.is_empty(), "{} emoji should not be empty", b.id);
        }
    }

    #[test]
    fn test_descriptions_non_empty() {
        for b in BUILDINGS {
            assert!(!b.description.is_empty(), "{} description should not be empty", b.id);
        }
    }

    #[test]
    fn test_find_building_by_id() {
        assert!(find_building("training_ground").is_some());
        assert!(find_building("smithy").is_some());
        assert!(find_building("nonexistent").is_none());
    }

    #[test]
    fn test_find_building_by_name() {
        assert!(find_building("训练场").is_some());
        assert!(find_building("铁匠铺").is_some());
        assert!(find_building("不存在的").is_none());
    }

    #[test]
    fn test_find_building_fuzzy() {
        assert!(find_building("训练").is_some());
        assert!(find_building("铁匠").is_some());
    }

    #[test]
    fn test_effect_types_unique() {
        let mut types: Vec<&str> = BUILDINGS.iter().map(|b| b.effect_type).collect();
        types.sort();
        types.dedup();
        assert_eq!(types.len(), BUILDINGS.len());
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(100), "100");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(1000000), "1,000,000");
    }

    #[test]
    fn test_guild_section_format() {
        assert_eq!(guild_section("test_guild"), "guild_buildings_test_guild");
    }

    #[test]
    fn test_first_level_costs_positive() {
        for b in BUILDINGS {
            assert!(b.gold_costs[0] > 0, "{} first level gold cost should be positive", b.id);
            assert!(b.fund_costs[0] > 0, "{} first level fund cost should be positive", b.id);
            assert_eq!(b.diamond_costs[0], 0, "{} first level diamond cost should be 0", b.id);
        }
    }

    #[test]
    fn test_max_level_diamond_costs() {
        for b in BUILDINGS {
            assert!(b.diamond_costs[4] > 0, "{} max level should have diamond cost", b.id);
        }
    }

    #[test]
    fn test_all_5_levels_have_effects() {
        for b in BUILDINGS {
            for i in 0..5usize {
                assert!(
                    !b.effect_descs[i].is_empty(),
                    "{} level {} effect desc should not be empty",
                    b.id,
                    i + 1
                );
            }
        }
    }
}

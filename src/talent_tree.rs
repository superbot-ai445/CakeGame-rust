/// CakeGame 天赋树系统 (Talent Tree System)
///
/// 玩家每升一级获得1天赋点，可分配到不同天赋路径
/// 五条天赋路径: 力量/智慧/坚韧/敏捷/不朽，各有10级被动加成
/// 天赋点可免费重置（每日限3次）
///
/// 指令: 查看天赋, 天赋详情, 分配天赋, 重置天赋, 天赋排行
use crate::core::*;
use crate::db::Database;

/// 天赋路径定义
struct TalentPath {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    desc: &'static str,
    /// 每级加成: (属性名, 每点加成值)
    bonuses: &'static [(&'static str, i32)],
}

const TALENT_PATHS: &[TalentPath] = &[
    TalentPath {
        id: "power",
        name: "力量",
        emoji: "⚔️",
        desc: "提升物理攻击和暴击率，适合近战职业",
        bonuses: &[("AD", 8), ("Crit", 1)],
    },
    TalentPath {
        id: "wisdom",
        name: "智慧",
        emoji: "🔮",
        desc: "提升魔法攻击和命中率，适合法师职业",
        bonuses: &[("AP", 8), ("Hit", 2)],
    },
    TalentPath {
        id: "toughness",
        name: "坚韧",
        emoji: "🛡️",
        desc: "提升生命值和防御力，适合坦克职业",
        bonuses: &[("HP", 50), ("Defense", 3), ("MagicResistance", 2)],
    },
    TalentPath {
        id: "agility",
        name: "敏捷",
        emoji: "💨",
        desc: "提升闪避和吸血能力，适合刺客职业",
        bonuses: &[("Dodge", 2), ("AbsorbHP", 1)],
    },
    TalentPath {
        id: "immortal",
        name: "不朽",
        emoji: "💎",
        desc: "提升免伤和穿透，适合高阶玩家",
        bonuses: &[("ImmuneDamage", 1), ("ADPTV", 1), ("APPTV", 1)],
    },
];

const MAX_LEVEL: i32 = 10;
const MAX_RESETS_PER_DAY: i32 = 3;

/// 解析天赋分配 (path_id:level,...)
pub fn parse_talent_alloc(s: &str) -> Vec<(String, i32)> {
    let mut result = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let pieces: Vec<&str> = part.splitn(2, ':').collect();
        if pieces.len() == 2 {
            let path_id = pieces[0].trim().to_string();
            if let Ok(level) = pieces[1].trim().parse::<i32>() {
                if level > 0 {
                    result.push((path_id, level));
                }
            }
        }
    }
    result
}

/// 序列化天赋分配
pub fn serialize_talent_alloc(allocs: &[(String, i32)]) -> String {
    allocs
        .iter()
        .map(|(id, lvl)| format!("{}:{}", id, lvl))
        .collect::<Vec<_>>()
        .join(",")
}

/// 查找天赋路径
fn find_path(query: &str) -> Option<&'static TalentPath> {
    let q = query.trim();
    // 精确匹配ID
    if let Some(p) = TALENT_PATHS.iter().find(|p| p.id == q) {
        return Some(p);
    }
    // 精确匹配名称
    if let Some(p) = TALENT_PATHS.iter().find(|p| p.name == q) {
        return Some(p);
    }
    // 模糊匹配
    TALENT_PATHS.iter().find(|p| p.name.contains(q) || p.id.contains(q))
}

/// 计算天赋属性加成
#[allow(dead_code)]
pub fn calc_talent_bonuses(allocs: &[(String, i32)]) -> Vec<(String, i32)> {
    let mut bonuses: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for (path_id, level) in allocs {
        if let Some(path) = TALENT_PATHS.iter().find(|p| p.id == path_id.as_str()) {
            for &(attr, per_point) in path.bonuses {
                *bonuses.entry(attr.to_string()).or_insert(0) += per_point * level;
            }
        }
    }
    let mut result: Vec<_> = bonuses.into_iter().collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.1));
    result
}

/// 计算已分配的天赋点总数
fn total_allocated(allocs: &[(String, i32)]) -> i32 {
    allocs.iter().map(|(_, lvl)| lvl).sum()
}

/// 计算可用天赋点
fn available_points(db: &Database, user_id: &str) -> i32 {
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    // 每级获得1天赋点，从5级开始
    let total_points = (level - 4).max(0);
    let alloc_key = format!("talent_alloc_{}", user_id);
    let alloc_str = db.global_get("talent_tree", &alloc_key);
    let allocs = parse_talent_alloc(&alloc_str);
    let used = total_allocated(&allocs);
    (total_points - used).max(0)
}

/// 获取今日重置次数
fn get_reset_count(db: &Database, user_id: &str) -> i32 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let key = format!("talent_reset_{}_{}", user_id, today);
    db.global_get("talent_tree", &key).parse::<i32>().unwrap_or(0)
}

/// 查看天赋
pub fn cmd_view_talent(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level < 5 {
        return "⚠️ 天赋系统需要5级才能开启！\n💡 继续升级吧，达到5级后即可分配天赋点。".to_string();
    }

    let alloc_key = format!("talent_alloc_{}", user_id);
    let alloc_str = db.global_get("talent_tree", &alloc_key);
    let allocs = parse_talent_alloc(&alloc_str);
    let total_pts = (level - 4).max(0);
    let used_pts = total_allocated(&allocs);
    let avail_pts = (total_pts - used_pts).max(0);
    let resets_today = get_reset_count(db, user_id);

    let mut out = String::new();
    out.push_str("🌳 天赋树系统\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!(
        "👤 等级: {} | 🎯 总天赋点: {} | ✨ 已分配: {} | 🆓 可用: {}\n",
        level, total_pts, used_pts, avail_pts
    ));
    out.push_str(&format!("🔄 今日重置: {}/{}次\n\n", resets_today, MAX_RESETS_PER_DAY));

    for path in TALENT_PATHS {
        let allocated = allocs
            .iter()
            .find(|(id, _)| id == path.id)
            .map(|(_, lvl)| *lvl)
            .unwrap_or(0);

        let bar = progress_bar(allocated, MAX_LEVEL, 10);
        out.push_str(&format!("{} {} {}\n", path.emoji, path.name, bar));
        out.push_str(&format!("   等级: {}/{}\n", allocated, MAX_LEVEL));
        out.push_str(&format!("   描述: {}\n", path.desc));

        // 显示当前加成
        let current_bonuses: Vec<String> = path
            .bonuses
            .iter()
            .map(|(attr, per)| format!("{} +{}", attr_display_name(attr), per * allocated))
            .collect();
        out.push_str(&format!(
            "   加成: {}\n",
            if current_bonuses.is_empty() {
                "无".to_string()
            } else {
                current_bonuses.join(", ")
            }
        ));

        if allocated < MAX_LEVEL {
            let next_bonuses: Vec<String> = path
                .bonuses
                .iter()
                .map(|(attr, per)| format!("{}+{}", attr_display_name(attr), per))
                .collect();
            out.push_str(&format!("   📈 下一级: {}\n", next_bonuses.join(", ")));
        }
        out.push('\n');
    }

    if avail_pts > 0 {
        out.push_str("💡 使用「分配天赋+路径名:等级」来分配天赋点\n");
        out.push_str("   例如: 分配天赋+力量:3 或 分配天赋+智慧:2,坚韧:1\n");
    }
    out.push_str(&format!(
        "🔄 使用「重置天赋」可重新分配（每日{}次）\n",
        MAX_RESETS_PER_DAY
    ));

    out
}

/// 天赋详情
pub fn cmd_talent_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let path_query = args.trim();
    let path = match find_path(path_query) {
        Some(p) => p,
        None => {
            let names: Vec<&str> = TALENT_PATHS.iter().map(|p| p.name).collect();
            return format!("⚠️ 未找到天赋路径「{}」\n💡 可用路径: {}", path_query, names.join("、"));
        }
    };

    let alloc_key = format!("talent_alloc_{}", user_id);
    let alloc_str = db.global_get("talent_tree", &alloc_key);
    let allocs = parse_talent_alloc(&alloc_str);
    let allocated = allocs
        .iter()
        .find(|(id, _)| id == path.id)
        .map(|(_, lvl)| *lvl)
        .unwrap_or(0);

    let mut out = String::new();
    out.push_str(&format!("{} {} 天赋详情\n", path.emoji, path.name));
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("📝 描述: {}\n", path.desc));
    out.push_str(&format!("📊 当前等级: {}/{}\n\n", allocated, MAX_LEVEL));

    // 每级加成明细
    out.push_str("📊 每级加成明细:\n");
    for &(attr, per_point) in path.bonuses {
        let current = per_point * allocated;
        let max_val = per_point * MAX_LEVEL;
        out.push_str(&format!(
            "   {} : 每级+{} (当前+{}/最大+{})\n",
            attr_display_name(attr),
            per_point,
            current,
            max_val
        ));
    }

    out.push('\n');
    out.push_str("📊 等级加成预览:\n");
    for lvl in 0..=MAX_LEVEL {
        if lvl == 0 || lvl % 2 == 0 || lvl == allocated || lvl == allocated + 1 || lvl == MAX_LEVEL {
            let marker = if lvl == allocated { " ← 当前" } else { "" };
            let bonuses: Vec<String> = path
                .bonuses
                .iter()
                .map(|(attr, per)| format!("{}+{}", attr_display_name(attr), per * lvl))
                .collect();
            out.push_str(&format!("   Lv.{:2}: {}{}\n", lvl, bonuses.join(", "), marker));
        }
    }

    if allocated < MAX_LEVEL {
        let avail = available_points(db, user_id);
        out.push_str(&format!("\n🆓 可用天赋点: {}\n", avail));
        out.push_str(&format!("💡 使用「分配天赋+{}:N」分配N点\n", path.name));
    }

    out
}

/// 分配天赋点
pub fn cmd_allocate_talent(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let input = args.trim();
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level < 5 {
        return "⚠️ 天赋系统需要5级才能开启！".to_string();
    }

    let allocations = parse_talent_alloc(input);
    if allocations.is_empty() {
        return "⚠️ 格式错误！请使用: 分配天赋+力量:3 或 分配天赋+智慧:2,坚韧:1".to_string();
    }

    // 验证所有路径
    for (path_id, _) in &allocations {
        if find_path(path_id).is_none() {
            let names: Vec<&str> = TALENT_PATHS.iter().map(|p| p.name).collect();
            return format!("⚠️ 未找到天赋路径「{}」\n💡 可用路径: {}", path_id, names.join("、"));
        }
    }

    let alloc_key = format!("talent_alloc_{}", user_id);
    let alloc_str = db.global_get("talent_tree", &alloc_key);
    let mut current_allocs = parse_talent_alloc(&alloc_str);
    let total_pts = (level - 4).max(0);
    let used_pts = total_allocated(&current_allocs);

    let mut total_to_add = 0i32;
    for (_, lvl) in &allocations {
        total_to_add += lvl;
    }

    if used_pts + total_to_add > total_pts {
        return format!(
            "⚠️ 天赋点不足！\n📊 总天赋点: {} | 已分配: {} | 本次需要: {}\n🆓 剩余可用: {}",
            total_pts,
            used_pts,
            total_to_add,
            (total_pts - used_pts).max(0)
        );
    }

    // 验证单路径不超过上限
    for (path_id, add_lvl) in &allocations {
        let current = current_allocs
            .iter()
            .find(|(id, _)| id == path_id)
            .map(|(_, lvl)| *lvl)
            .unwrap_or(0);
        if current + add_lvl > MAX_LEVEL {
            let path = find_path(path_id).unwrap();
            return format!(
                "⚠️ {} 天赋已达上限！当前: {}/{}，无法再加{}点",
                path.name, current, MAX_LEVEL, add_lvl
            );
        }
    }

    // 应用分配
    let mut out = String::new();
    out.push_str("✅ 天赋分配成功！\n\n");

    for (path_id, add_lvl) in &allocations {
        let path = find_path(path_id).unwrap();
        let entry = current_allocs.iter_mut().find(|(id, _)| id == path_id);
        match entry {
            Some((_, lvl)) => {
                let old = *lvl;
                *lvl += add_lvl;
                out.push_str(&format!(
                    "{} {}: Lv.{} → Lv.{} (+{})\n",
                    path.emoji, path.name, old, lvl, add_lvl
                ));
                for &(attr, per) in path.bonuses {
                    out.push_str(&format!("   {} +{}\n", attr_display_name(attr), per * add_lvl));
                }
            }
            None => {
                current_allocs.push((path_id.clone(), *add_lvl));
                out.push_str(&format!(
                    "{} {}: Lv.0 → Lv.{} (+{})\n",
                    path.emoji, path.name, add_lvl, add_lvl
                ));
                for &(attr, per) in path.bonuses {
                    out.push_str(&format!("   {} +{}\n", attr_display_name(attr), per * add_lvl));
                }
            }
        }
    }

    let new_used = total_allocated(&current_allocs);
    let remaining = (total_pts - new_used).max(0);
    out.push_str(&format!("\n📊 已分配: {} | 剩余: {}", new_used, remaining));

    // 保存
    let new_alloc_str = serialize_talent_alloc(&current_allocs);
    db.global_set("talent_tree", &alloc_key, &new_alloc_str);

    out
}

/// 重置天赋
pub fn cmd_reset_talent(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level < 5 {
        return "⚠️ 天赋系统需要5级才能开启！".to_string();
    }

    let resets_today = get_reset_count(db, user_id);
    if resets_today >= MAX_RESETS_PER_DAY {
        return format!(
            "⚠️ 今日重置次数已用完！({}/{})\n💡 每日最多重置{}次，明天再来吧。",
            resets_today, MAX_RESETS_PER_DAY, MAX_RESETS_PER_DAY
        );
    }

    let alloc_key = format!("talent_alloc_{}", user_id);
    let alloc_str = db.global_get("talent_tree", &alloc_key);
    if alloc_str.is_empty() {
        return "⚠️ 你还没有分配过天赋点，无需重置。".to_string();
    }

    let allocs = parse_talent_alloc(&alloc_str);
    if allocs.is_empty() {
        return "⚠️ 你还没有分配过天赋点，无需重置。".to_string();
    }

    let freed_points = total_allocated(&allocs);

    // 清空分配
    db.global_set("talent_tree", &alloc_key, "");

    // 更新重置次数
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let reset_key = format!("talent_reset_{}_{}", user_id, today);
    db.global_set("talent_tree", &reset_key, &(resets_today + 1).to_string());

    let mut out = String::new();
    out.push_str("🔄 天赋重置成功！\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━\n");

    for (path_id, lvl) in &allocs {
        if let Some(path) = find_path(path_id) {
            out.push_str(&format!(
                "{} {}: Lv.{} → Lv.0 (释放{}点)\n",
                path.emoji, path.name, lvl, lvl
            ));
        }
    }

    out.push_str(&format!("\n✅ 释放天赋点: {}\n", freed_points));
    out.push_str(&format!("📊 今日重置: {}/{}\n", resets_today + 1, MAX_RESETS_PER_DAY));
    out.push_str("💡 使用「查看天赋」重新分配天赋点。");

    out
}

/// 天赋排行
pub fn cmd_talent_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut rankings: Vec<(String, i32, String)> = Vec::new();

    // 从Global表获取所有天赋分配
    let conn = match db.conn.lock() {
        Ok(c) => c,
        Err(_) => return "⚠️ 数据库错误".to_string(),
    };
    let mut stmt = match conn
        .prepare("SELECT Key, Value FROM Global WHERE Section = 'talent_tree' AND Key LIKE 'talent_alloc_%'")
    {
        Ok(s) => s,
        Err(_) => return "⚠️ 查询失败".to_string(),
    };

    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    for (key, value) in rows {
        if value.is_empty() {
            continue;
        }
        let uid = key.replace("talent_alloc_", "");
        let allocs = parse_talent_alloc(&value);
        let total = total_allocated(&allocs);
        let nickname_raw = db.read_basic(&uid, ITEM_NAME);
        let nickname = if nickname_raw.is_empty() {
            uid.clone()
        } else {
            nickname_raw
        };
        rankings.push((uid, total, nickname));
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = String::new();
    out.push_str("🌳 天赋排行榜\n");
    out.push_str("━━━━━━━━━━━━━━━━━━━━━━\n");

    if rankings.is_empty() {
        out.push_str("暂无玩家分配天赋点。\n");
        return out;
    }

    for (i, (uid, total, nickname)) in rankings.iter().enumerate().take(15) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let bar = progress_bar(*total, MAX_LEVEL * TALENT_PATHS.len() as i32, 8);
        let marker = if uid == user_id { " ← 你" } else { "" };
        out.push_str(&format!(
            "{} #{} {} {} {}点 {}\n",
            medal,
            i + 1,
            nickname,
            bar,
            total,
            marker
        ));
    }

    // 用户自身定位
    if let Some(pos) = rankings.iter().position(|(uid, _, _)| uid == user_id) {
        if pos >= 15 {
            out.push_str(&format!("\n📍 你的排名: #{} ({}点)\n", pos + 1, rankings[pos].1));
        }
    } else {
        let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
        let total_pts = (user_level - 4).max(0);
        out.push_str(&format!("\n📍 你尚未分配天赋点 (可用: {}点)\n", total_pts));
    }

    out
}

/// 属性显示名
fn attr_display_name(attr: &str) -> String {
    match attr {
        "HP" => "生命值".to_string(),
        "MP" => "魔法值".to_string(),
        "AD" => "物攻".to_string(),
        "AP" => "魔攻".to_string(),
        "Defense" => "防御".to_string(),
        "MagicResistance" => "魔抗".to_string(),
        "Hit" => "命中".to_string(),
        "Dodge" => "闪避".to_string(),
        "Crit" => "暴击".to_string(),
        "AbsorbHP" => "吸血".to_string(),
        "ImmuneDamage" => "免伤".to_string(),
        "ADPTV" => "物穿".to_string(),
        "APPTV" => "法穿".to_string(),
        _ => attr.to_string(),
    }
}

/// 进度条
fn progress_bar(current: i32, max: i32, width: usize) -> String {
    let max = max.max(1);
    let filled = ((current as f64 / max as f64) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_count() {
        assert_eq!(TALENT_PATHS.len(), 5);
    }

    #[test]
    fn test_path_ids_unique() {
        let ids: Vec<&str> = TALENT_PATHS.iter().map(|p| p.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }

    #[test]
    fn test_path_names_unique() {
        let names: Vec<&str> = TALENT_PATHS.iter().map(|p| p.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_talent_alloc("").is_empty());
        assert!(parse_talent_alloc("  ").is_empty());
    }

    #[test]
    fn test_parse_single() {
        let allocs = parse_talent_alloc("power:3");
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].0, "power");
        assert_eq!(allocs[0].1, 3);
    }

    #[test]
    fn test_parse_multiple() {
        let allocs = parse_talent_alloc("power:3,wisdom:2,toughness:1");
        assert_eq!(allocs.len(), 3);
        assert_eq!(allocs[0].0, "power");
        assert_eq!(allocs[0].1, 3);
        assert_eq!(allocs[1].0, "wisdom");
        assert_eq!(allocs[1].1, 2);
        assert_eq!(allocs[2].0, "toughness");
        assert_eq!(allocs[2].1, 1);
    }

    #[test]
    fn test_parse_filters_zero() {
        let allocs = parse_talent_alloc("power:0,wisdom:5");
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].0, "wisdom");
    }

    #[test]
    fn test_parse_filters_negative() {
        let allocs = parse_talent_alloc("power:-1,wisdom:5");
        assert_eq!(allocs.len(), 1);
        assert_eq!(allocs[0].0, "wisdom");
    }

    #[test]
    fn test_serialize_empty() {
        assert_eq!(serialize_talent_alloc(&[]), "");
    }

    #[test]
    fn test_serialize_roundtrip() {
        let original = vec![("power".to_string(), 3), ("wisdom".to_string(), 2)];
        let s = serialize_talent_alloc(&original);
        let parsed = parse_talent_alloc(&s);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "power");
        assert_eq!(parsed[0].1, 3);
        assert_eq!(parsed[1].0, "wisdom");
        assert_eq!(parsed[1].1, 2);
    }

    #[test]
    fn test_calc_bonuses_single_path() {
        let allocs = vec![("power".to_string(), 5)];
        let bonuses = calc_talent_bonuses(&allocs);
        let ad = bonuses.iter().find(|(a, _)| a == "AD").map(|(_, v)| *v).unwrap_or(0);
        let crit = bonuses.iter().find(|(a, _)| a == "Crit").map(|(_, v)| *v).unwrap_or(0);
        assert_eq!(ad, 40); // 8 * 5
        assert_eq!(crit, 5); // 1 * 5
    }

    #[test]
    fn test_calc_bonuses_multiple_paths() {
        let allocs = vec![("power".to_string(), 3), ("wisdom".to_string(), 2)];
        let bonuses = calc_talent_bonuses(&allocs);
        let ad = bonuses.iter().find(|(a, _)| a == "AD").map(|(_, v)| *v).unwrap_or(0);
        let ap = bonuses.iter().find(|(a, _)| a == "AP").map(|(_, v)| *v).unwrap_or(0);
        let crit = bonuses.iter().find(|(a, _)| a == "Crit").map(|(_, v)| *v).unwrap_or(0);
        let hit = bonuses.iter().find(|(a, _)| a == "Hit").map(|(_, v)| *v).unwrap_or(0);
        assert_eq!(ad, 24); // 8 * 3
        assert_eq!(ap, 16); // 8 * 2
        assert_eq!(crit, 3); // 1 * 3
        assert_eq!(hit, 4); // 2 * 2
    }

    #[test]
    fn test_calc_bonuses_empty() {
        let bonuses = calc_talent_bonuses(&[]);
        assert!(bonuses.is_empty());
    }

    #[test]
    fn test_find_path_exact() {
        assert!(find_path("power").is_some());
        assert!(find_path("力量").is_some());
        assert!(find_path("wisdom").is_some());
        assert!(find_path("智慧").is_some());
    }

    #[test]
    fn test_find_path_fuzzy() {
        // "力" should match "力量"
        assert!(find_path("力").is_some());
    }

    #[test]
    fn test_find_path_not_found() {
        assert!(find_path("不存在").is_none());
        assert!(find_path("xyz").is_none());
    }

    #[test]
    fn test_total_allocated() {
        let allocs = vec![("power".to_string(), 5), ("wisdom".to_string(), 3)];
        assert_eq!(total_allocated(&allocs), 8);
    }

    #[test]
    fn test_total_allocated_empty() {
        assert_eq!(total_allocated(&[]), 0);
    }

    #[test]
    fn test_max_level() {
        assert_eq!(MAX_LEVEL, 10);
    }

    #[test]
    fn test_max_resets() {
        assert_eq!(MAX_RESETS_PER_DAY, 3);
    }

    #[test]
    fn test_attr_display_name() {
        assert_eq!(attr_display_name("AD"), "物攻");
        assert_eq!(attr_display_name("AP"), "魔攻");
        assert_eq!(attr_display_name("HP"), "生命值");
        assert_eq!(attr_display_name("Defense"), "防御");
        assert_eq!(attr_display_name("Crit"), "暴击");
        assert_eq!(attr_display_name("Dodge"), "闪避");
        assert_eq!(attr_display_name("Hit"), "命中");
        assert_eq!(attr_display_name("unknown"), "unknown");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(10, 10, 10);
        assert_eq!(bar, "[██████████]");
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 10, 10);
        assert_eq!(bar, "[░░░░░░░░░░]");
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(5, 10, 10);
        assert_eq!(bar, "[█████░░░░░]");
    }

    #[test]
    fn test_bonus_scaling() {
        // Each level should add consistent bonuses
        for path in TALENT_PATHS {
            for &(attr, per_point) in path.bonuses {
                assert!(
                    per_point > 0,
                    "path {} attr {} per_point should be > 0",
                    path.name,
                    attr
                );
            }
        }
    }

    #[test]
    fn test_path_emojis() {
        for path in TALENT_PATHS {
            assert!(!path.emoji.is_empty(), "path {} should have emoji", path.name);
            assert!(!path.desc.is_empty(), "path {} should have description", path.name);
        }
    }
}

/// CakeGame 玩家对比系统
/// 提供玩家之间的属性对比功能 + 装备对比功能
use crate::core::*;
use crate::db::Database;
use crate::equip_score;
use crate::user;

/// 根据昵称查找用户ID（与 social::find_user_by_name 逻辑一致）
fn find_user_by_name(db: &Database, name: &str) -> String {
    let conn = db.lock_conn();
    // 新用户：Node='基础信息', Item='名称'
    let result = conn
        .prepare("SELECT ID FROM Basic_User WHERE Node='基础信息' AND Item='名称' AND Data=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([name], |row| row.get::<_, String>(0))
                .ok()
                .and_then(|mut rows| rows.next())
                .and_then(|r| r.ok())
        });
    if let Some(id) = result {
        return id;
    }
    // 老用户：Node='Basic', Item='Name'
    let result = conn
        .prepare("SELECT ID FROM Basic_User WHERE Node='Basic' AND Item='Name' AND Data=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([name], |row| row.get::<_, String>(0))
                .ok()
                .and_then(|mut rows| rows.next())
                .and_then(|r| r.ok())
        });
    result.unwrap_or_default()
}

/// 计算战力评分（与 router::cmd_combat_power 中公式一致）
fn calc_combat_power(info: &UserInfo) -> i64 {
    let power = info.hp_max as f64 * 0.5
        + info.mp_max as f64 * 0.3
        + info.ad as f64 * 2.0
        + info.ap as f64 * 2.0
        + info.defense as f64 * 1.5
        + info.magic_res as f64 * 1.5
        + info.hit as f64 * 1.0
        + info.dodge as f64 * 1.0
        + info.crit as f64 * 1.0
        + info.absorb_hp as f64 * 0.5
        + info.shield as f64 * 0.3;
    power as i64
}

/// 玩家对比
pub fn cmd_player_compare(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let target_input = args.trim();
    if target_input.is_empty() {
        return format!(
            "{}\n\
             ═══ 玩家对比 ═══\n\
             ❌ 请指定对比目标！\n\
             用法: 对比+玩家昵称\n\
             示例: 对比小明",
            prefix
        );
    }

    // 通过昵称查找目标玩家
    let target_id = find_user_by_name(db, target_input);

    // 也尝试直接用ID查找（兼容传入ID的情况）
    let target_id = if target_id.is_empty() && db.user_exists(target_input) {
        target_input.to_string()
    } else {
        target_id
    };

    if target_id.is_empty() {
        return format!(
            "{}\n\
             ═══ 玩家对比 ═══\n\
             ❌ 找不到玩家 [{}]！\n\
             请确认昵称是否正确。",
            prefix, target_input
        );
    }

    // 不能和自己对比
    if target_id == user_id {
        return format!(
            "{}\n\
             ═══ 玩家对比 ═══\n\
             ❌ 不能和自己对比哦~\n\
             请指定其他玩家的昵称。",
            prefix
        );
    }

    // 获取双方总属性
    let my_info = user::calc_total_attrs(db, user_id);
    let target_info = user::calc_total_attrs(db, &target_id);

    let my_power = calc_combat_power(&my_info);
    let target_power = calc_combat_power(&target_info);

    // 对比属性列表
    struct StatItem {
        name: &'static str,
        icon: &'static str,
        my_val: i32,
        target_val: i32,
    }

    let stats = vec![
        StatItem {
            name: "等级",
            icon: "📊",
            my_val: my_info.level,
            target_val: target_info.level,
        },
        StatItem {
            name: "生命",
            icon: "❤️",
            my_val: my_info.hp_max,
            target_val: target_info.hp_max,
        },
        StatItem {
            name: "魔法",
            icon: "💙",
            my_val: my_info.mp_max,
            target_val: target_info.mp_max,
        },
        StatItem {
            name: "物攻",
            icon: "⚔️",
            my_val: my_info.ad,
            target_val: target_info.ad,
        },
        StatItem {
            name: "魔攻",
            icon: "🔮",
            my_val: my_info.ap,
            target_val: target_info.ap,
        },
        StatItem {
            name: "防御",
            icon: "🛡️",
            my_val: my_info.defense,
            target_val: target_info.defense,
        },
        StatItem {
            name: "魔抗",
            icon: "🔰",
            my_val: my_info.magic_res,
            target_val: target_info.magic_res,
        },
        StatItem {
            name: "命中",
            icon: "🎯",
            my_val: my_info.hit,
            target_val: target_info.hit,
        },
        StatItem {
            name: "闪避",
            icon: "💨",
            my_val: my_info.dodge,
            target_val: target_info.dodge,
        },
        StatItem {
            name: "暴击",
            icon: "💥",
            my_val: my_info.crit,
            target_val: target_info.crit,
        },
    ];

    // 统计优势
    let mut my_advantage = 0i32;
    let mut target_advantage = 0i32;
    let mut ties = 0i32;

    for s in &stats {
        if s.my_val > s.target_val {
            my_advantage += 1;
        } else if s.target_val > s.my_val {
            target_advantage += 1;
        } else {
            ties += 1;
        }
    }

    // 战力对比
    if my_power > target_power {
        my_advantage += 1;
    } else if target_power > my_power {
        target_advantage += 1;
    } else {
        ties += 1;
    }

    // 构建输出
    let mut result = format!(
        "{}\n\
         ═══ 玩家对比 ═══\n\
         {} vs {}",
        prefix, my_info.name, target_info.name
    );

    result.push_str("\n─────────────────\n");
    result.push_str("属性      我      对方    差距\n");
    result.push_str("─────────────────\n");

    for s in &stats {
        let diff = s.my_val - s.target_val;
        let diff_str = if diff > 0 {
            format!("+{}", diff)
        } else {
            diff.to_string()
        };
        let indicator = if s.my_val > s.target_val {
            "↑"
        } else if s.target_val > s.my_val {
            "↓"
        } else {
            "="
        };
        result.push_str(&format!(
            "{} {:<4} {:>6}  {:>6}  {:>6}{}\n",
            s.icon, s.name, s.my_val, s.target_val, diff_str, indicator
        ));
    }

    // 战力对比
    let power_diff = my_power - target_power;
    let power_diff_str = if power_diff > 0 {
        format!("+{}", power_diff)
    } else {
        power_diff.to_string()
    };
    let power_indicator = if my_power > target_power {
        "↑"
    } else if target_power > my_power {
        "↓"
    } else {
        "="
    };
    result.push_str(&format!(
        "🏆 战力  {:>6}  {:>6}  {:>6}{}\n",
        my_power, target_power, power_diff_str, power_indicator
    ));

    result.push_str("─────────────────\n");

    // 总结
    result.push_str("\n📊 对比总结：\n");
    if my_advantage > target_advantage {
        result.push_str(&format!(
            "🏆 你占据优势！（{}胜 {}平 {}负）\n",
            my_advantage, ties, target_advantage
        ));
        result.push_str(&format!("💪 你在 {} 项属性上领先于 {}", my_advantage, target_info.name));
    } else if target_advantage > my_advantage {
        result.push_str(&format!(
            "😔 {} 占据优势！（{}胜 {}平 {}负）\n",
            target_info.name, target_advantage, ties, my_advantage
        ));
        result.push_str(&format!(
            "📌 {} 在 {} 项属性上领先于你",
            target_info.name, target_advantage
        ));
    } else {
        result.push_str(&format!(
            "🤝 旗鼓相当！（{}胜 {}平 {}负）\n",
            my_advantage, ties, target_advantage
        ));
        result.push_str("⚖️ 双方实力接近，各有千秋");
    }

    result
}

/// 从数据库读取单件装备数据
fn read_equip_data(db: &Database, item_name: &str) -> Option<(String, ItemData)> {
    let conn = db.lock_conn();
    let mut stmt = conn
        .prepare("SELECT Name, LtemData FROM Config_Goods WHERE Type='Equip' AND Name LIKE ?1")
        .ok()?;
    let pattern = format!("%{}%", item_name);
    let mut rows = stmt
        .query_map([pattern], |row| {
            let name: String = row.get(0).unwrap_or_default();
            let data: String = row.get(1).unwrap_or_default();
            Ok((name, data))
        })
        .ok()?;

    let (name, data_str) = rows.next()?.ok()?;
    let data: ItemData = serde_json::from_str(&data_str).ok()?;
    Some((name, data))
}

/// 格式化属性差异箭头
fn diff_indicator(a: i32, b: i32) -> &'static str {
    if a > b {
        " ↑"
    } else if a < b {
        " ↓"
    } else {
        ""
    }
}

/// 格式化单行属性对比
fn format_stat_line(icon: &str, name: &str, val_a: i32, val_b: i32) -> String {
    let diff = val_a - val_b;
    let diff_str = if diff > 0 {
        format!(" [+{}]", diff)
    } else if diff < 0 {
        format!(" [{}]", diff)
    } else {
        String::new()
    };
    let indicator = diff_indicator(val_a, val_b);
    format!(
        "{}{:<6} {:>6} vs {:>6}{}{}\n",
        icon, name, val_a, val_b, diff_str, indicator
    )
}

/// 装备对比 — 对比两件装备的属性差异
/// 用法: 装备对比+物品名1+物品名2
pub fn cmd_item_compare(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再使用装备对比！", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n\
             ═══ 装备对比 ═══\n\
             ❌ 请指定要对比的两件装备！\n\
             用法: 装备对比+物品名1+物品名2\n\
             示例: 装备对比+屠龙宝刀+拉拉肥",
            prefix
        );
    }

    // 解析参数（用+或空格分隔）
    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        // 尝试用空格分隔
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() < 2 {
            return format!(
                "{}\n\
                 ═══ 装备对比 ═══\n\
                 ❌ 需要指定两件装备！\n\
                 用法: 装备对比+物品名1+物品名2",
                prefix
            );
        }
        return compare_two_items(db, user_id, parts[0], parts[1]);
    }

    compare_two_items(db, user_id, parts[0], parts[1])
}

/// 执行两件装备的对比
fn compare_two_items(db: &Database, user_id: &str, name_a: &str, name_b: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let item_a = read_equip_data(db, name_a);
    let item_b = read_equip_data(db, name_b);

    if item_a.is_none() {
        return format!("{}\n❌ 找不到装备 [{}]！", prefix, name_a);
    }
    if item_b.is_none() {
        return format!("{}\n❌ 找不到装备 [{}]！", prefix, name_b);
    }

    let (real_name_a, data_a) = item_a.unwrap();
    let (real_name_b, data_b) = item_b.unwrap();

    let score_a = equip_score::calc_equip_score(&data_a);
    let score_b = equip_score::calc_equip_score(&data_b);

    let quality_a = equip_score::quality_stars(&real_name_a);
    let quality_b = equip_score::quality_stars(&real_name_b);

    let mut result = format!(
        "{}\n\
         ═══ 装备对比 ═══\n\
         {} {} vs {} {}\n",
        prefix, real_name_a, quality_a, real_name_b, quality_b
    );

    result.push_str("─────────────────────────────\n");
    result.push_str("属性        物品A      物品B\n");
    result.push_str("─────────────────────────────\n");

    // 部位
    result.push_str(&format!(
        "📍 部位    {:<10} {:<10}\n",
        data_a.slot_name, data_b.slot_name
    ));

    // 等级要求
    result.push_str(&format!(
        "📊 等级    {:<10} {:<10}\n",
        if data_a.use_lv > 0 {
            format!("Lv.{}", data_a.use_lv)
        } else {
            "无限制".to_string()
        },
        if data_b.use_lv > 0 {
            format!("Lv.{}", data_b.use_lv)
        } else {
            "无限制".to_string()
        }
    ));

    // 职业要求
    let occ_a = if data_a.occupation.is_empty() || data_a.occupation == "[NULL]" {
        "全职业".to_string()
    } else {
        data_a.occupation.clone()
    };
    let occ_b = if data_b.occupation.is_empty() || data_b.occupation == "[NULL]" {
        "全职业".to_string()
    } else {
        data_b.occupation.clone()
    };
    result.push_str(&format!("👤 职业    {:<10} {:<10}\n", occ_a, occ_b));

    result.push_str("─────────────────────────────\n");
    result.push_str("属性对比:\n");

    // 属性对比列表
    let stats: Vec<(&str, &str, i32, i32)> = vec![
        ("❤️", "生命", data_a.add_hp, data_b.add_hp),
        ("💙", "魔法", data_a.add_mp, data_b.add_mp),
        ("⚔️", "物攻", data_a.add_ad, data_b.add_ad),
        ("🔮", "魔攻", data_a.add_ap, data_b.add_ap),
        ("🛡️", "防御", data_a.add_defense, data_b.add_defense),
        ("🔰", "魔抗", data_a.add_magic, data_b.add_magic),
        ("🎯", "命中", data_a.add_hit, data_b.add_hit),
        ("💨", "闪避", data_a.add_dodge, data_b.add_dodge),
        ("💥", "暴击", data_a.add_crit, data_b.add_crit),
        ("🩸", "吸血", data_a.add_absorb_hp, data_b.add_absorb_hp),
        ("🔰", "免伤", data_a.add_immune_damage, data_b.add_immune_damage),
        ("⚔️", "穿透", data_a.add_adptv, data_b.add_adptv),
        ("🔮", "魔穿", data_a.add_apptv, data_b.add_apptv),
    ];

    let mut a_wins = 0i32;
    let mut b_wins = 0i32;
    let mut ties = 0i32;

    for (icon, name, val_a, val_b) in &stats {
        if *val_a == 0 && *val_b == 0 {
            continue; // 跳过双方都为0的属性
        }
        result.push_str(&format_stat_line(icon, name, *val_a, *val_b));
        if val_a > val_b {
            a_wins += 1;
        } else if val_b > val_a {
            b_wins += 1;
        } else {
            ties += 1;
        }
    }

    result.push_str("─────────────────────────────\n");

    // 评分对比
    result.push_str(&format!("🏆 评分    {:>8.1}   {:>8.1}", score_a, score_b));
    if score_a > score_b {
        result.push_str(&format!(" [+{:.1}]\n", score_a - score_b));
    } else if score_b > score_a {
        result.push_str(&format!(" [{:.1}]\n", score_a - score_b));
    } else {
        result.push('\n');
    }

    // 总结
    result.push_str("\n📊 对比总结：\n");
    if a_wins > b_wins {
        result.push_str(&format!(
            "🏆 {} 更优！（{}项领先 {}项持平 {}项落后）\n",
            real_name_a, a_wins, ties, b_wins
        ));
    } else if b_wins > a_wins {
        result.push_str(&format!(
            "🏆 {} 更优！（{}项领先 {}项持平 {}项落后）\n",
            real_name_b, b_wins, ties, a_wins
        ));
    } else {
        result.push_str(&format!("🤝 旗鼓相当！（{}项持平）\n", ties));
    }

    // 推荐
    if score_a > score_b * 1.2 {
        result.push_str(&format!("💡 推荐使用 {}\n", real_name_a));
    } else if score_b > score_a * 1.2 {
        result.push_str(&format!("💡 推荐使用 {}\n", real_name_b));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_indicator() {
        assert_eq!(diff_indicator(10, 5), " ↑");
        assert_eq!(diff_indicator(5, 10), " ↓");
        assert_eq!(diff_indicator(5, 5), "");
    }

    #[test]
    fn test_format_stat_line() {
        let line = format_stat_line("⚔️", "物攻", 100, 80);
        assert!(line.contains("物攻"));
        assert!(line.contains("100"));
        assert!(line.contains("80"));
        assert!(line.contains("[+20]"));
        assert!(line.contains("↑"));
    }

    #[test]
    fn test_format_stat_line_equal() {
        let line = format_stat_line("🛡️", "防御", 50, 50);
        assert!(line.contains("防御"));
        assert!(!line.contains("[+0]"));
        assert!(!line.contains("↑"));
        assert!(!line.contains("↓"));
    }

    #[test]
    fn test_format_stat_line_lower() {
        let line = format_stat_line("💙", "魔法", 30, 60);
        assert!(line.contains("[-30]"));
        assert!(line.contains("↓"));
    }

    #[test]
    fn test_calc_combat_power() {
        let info = UserInfo {
            hp_max: 100,
            mp_max: 50,
            ad: 20,
            ap: 15,
            defense: 10,
            magic_res: 8,
            hit: 5,
            dodge: 3,
            crit: 2,
            absorb_hp: 1,
            shield: 0,
            ..Default::default()
        };
        let power = calc_combat_power(&info);
        // 100*0.5 + 50*0.3 + 20*2 + 15*2 + 10*1.5 + 8*1.5 + 5*1 + 3*1 + 2*1 + 1*0.5 + 0*0.3
        // = 50 + 15 + 40 + 30 + 15 + 12 + 5 + 3 + 2 + 0.5 + 0 = 172.5 -> 172
        assert_eq!(power, 172);
    }

    #[test]
    fn test_calc_combat_power_zero() {
        let info = UserInfo::default();
        let power = calc_combat_power(&info);
        assert_eq!(power, 0);
    }
}

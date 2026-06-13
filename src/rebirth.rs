/// CakeGame 转生系统 (Rebirth/Prestige System)
///
/// 来源: Global 表 ext_re_hzxyx 配置
/// 高级玩家达到一定等级后可转生，重置等级并获得永久属性加成
/// 每次转生增加转生次数，提高属性加成比例
///
/// 指令: 转生信息, 执行转生, 转生排行
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 转生配置（从 Global 表 ext_re_hzxyx 读取）
#[allow(dead_code)]
struct RebirthConfig {
    min_level: i32,     // 最低转生等级 (Lv_global)
    cost_diamond: i64,  // 转生消耗钻石 (Cz_global)
    reset_level: i32,   // 转生后重置等级 (Csh_global)
    return_map: String, // 转生后传送地图 (15_global)
    /// 各转生阶段等级要求 (1_global ~ 14_global)
    stage_levels: Vec<i32>,
}

/// 从 Global 表读取转生配置
fn load_config(db: &Database) -> RebirthConfig {
    let min_level: i32 = db.global_get("ext_re_hzxyx", "Lv_global").parse().unwrap_or(60);
    let cost_diamond: i64 = db.global_get("ext_re_hzxyx", "Cz_global").parse().unwrap_or(35);
    let reset_level: i32 = db.global_get("ext_re_hzxyx", "Csh_global").parse().unwrap_or(31);
    let return_map_raw = db.global_get("ext_re_hzxyx", "15_global");
    let return_map = if return_map_raw.is_empty() {
        "主城".to_string()
    } else {
        return_map_raw
    };

    let mut stage_levels = Vec::new();
    for i in 1..=14 {
        let key = format!("{}_global", i);
        let val: i32 = db.global_get("ext_re_hzxyx", &key).parse().unwrap_or(10);
        stage_levels.push(val);
    }

    RebirthConfig {
        min_level,
        cost_diamond,
        reset_level,
        return_map,
        stage_levels,
    }
}

/// 获取用户转生次数
fn get_rebirth_count(db: &Database, user_id: &str) -> i32 {
    db.read_user_data(user_id, "rebirth_count").parse().unwrap_or(0)
}

/// 设置用户转生次数
fn set_rebirth_count(db: &Database, user_id: &str, count: i32) {
    db.write_user_data(user_id, "rebirth_count", &count.to_string());
}

/// 获取转生日期
fn get_rebirth_date(db: &Database, user_id: &str) -> String {
    db.read_user_data(user_id, "rebirth_date")
}

/// 设置转生日期
fn set_rebirth_date(db: &Database, user_id: &str, date: &str) {
    db.write_user_data(user_id, "rebirth_date", date);
}

/// 计算转生等级要求（根据转生次数）
fn get_required_level(config: &RebirthConfig, rebirth_count: i32) -> i32 {
    let idx = rebirth_count as usize;
    if idx < config.stage_levels.len() {
        config.stage_levels[idx]
    } else {
        // 超出预定义阶段，使用最后阶段的值
        *config.stage_levels.last().unwrap_or(&10)
    }
}

/// 计算转生加成（每次转生 +5% 全属性）
fn get_rebirth_bonus_pct(rebirth_count: i32) -> i32 {
    rebirth_count * 5
}

/// 转生信息 — 查看转生状态和条件
pub fn cmd_rebirth_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let config = load_config(db);
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
    let rebirth_count = get_rebirth_count(db, user_id);
    let bonus_pct = get_rebirth_bonus_pct(rebirth_count);
    let required_level = get_required_level(&config, rebirth_count);

    let mut r = format!("{}\n═══ 🔄 转生系统 ═══", prefix);
    r.push_str("\n\n📊 当前状态:");
    r.push_str(&format!("\n  等级: Lv.{}", level));
    r.push_str(&format!("\n  转生次数: {}次", rebirth_count));
    r.push_str(&format!("\n  转生加成: 全属性+{}%", bonus_pct));
    r.push_str(&format!("\n  💎 钻石: {}", diamond));

    r.push_str("\n\n📋 下次转生条件:");
    r.push_str(&format!("\n  等级要求: Lv.{}", required_level));
    r.push_str(&format!("\n  钻石消耗: {}", config.cost_diamond));

    let can_rebirth = level >= required_level && diamond >= config.cost_diamond;
    if can_rebirth {
        r.push_str("\n\n✅ 条件满足！发送 '执行转生' 开始转生");
    } else {
        r.push_str("\n\n❌ 条件不满足:");
        if level < required_level {
            r.push_str(&format!("\n  需要等级 Lv.{} (当前 Lv.{})", required_level, level));
        }
        if diamond < config.cost_diamond {
            r.push_str(&format!("\n  需要 {}💎 (当前 {}💎)", config.cost_diamond, diamond));
        }
    }

    let last_date = get_rebirth_date(db, user_id);
    if !last_date.is_empty() {
        r.push_str(&format!("\n\n📅 上次转生: {}", last_date));
    }

    r.push_str(&format!(
        "\n\n💡 转生后等级重置为 Lv.{}，传送到{}",
        config.reset_level, config.return_map
    ));
    r.push_str("\n💡 转生次数越多，全属性加成越高！");
    r.push_str("\n\n📖 发送 '转生排行' 查看全服转生排名");

    r
}

/// 执行转生 — 重置等级，获得永久加成
pub fn cmd_rebirth_execute(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let config = load_config(db);
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);
    let rebirth_count = get_rebirth_count(db, user_id);
    let required_level = get_required_level(&config, rebirth_count);

    // 检查等级
    if level < required_level {
        return format!("{}\n转生失败！需要 Lv.{}，当前 Lv.{}", prefix, required_level, level);
    }

    // 检查钻石
    if diamond < config.cost_diamond {
        return format!(
            "{}\n转生失败！需要 {}💎，当前 {}💎",
            prefix, config.cost_diamond, diamond
        );
    }

    // 检查是否虚弱中
    let weak_remaining = user::check_weakness(db, user_id);
    if weak_remaining > 0 {
        return format!(
            "{}\n转生失败！您正处于虚弱状态（剩余{}秒），请等虚弱恢复后再转生。",
            prefix, weak_remaining
        );
    }

    // 记录转生前的状态
    let old_level = level;

    // 扣除钻石
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, config.cost_diamond);

    // 重置等级为转生后等级
    db.write_basic(user_id, ITEM_LEVEL, &config.reset_level.to_string());

    // 重置当前经验为0
    db.write_basic(user_id, "CurrentExp", "0");

    // 恢复满血满蓝
    let hp_max = user::calc_hp_max(db, user_id);
    let mp_max = user::calc_mp_max(db, user_id);
    db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_max);
    db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_max);

    // 传送到主城
    db.write_basic(user_id, ITEM_LOCATION, &config.return_map);

    // 更新转生次数
    let new_count = rebirth_count + 1;
    set_rebirth_count(db, user_id, new_count);
    set_rebirth_date(
        db,
        user_id,
        &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    let new_bonus = get_rebirth_bonus_pct(new_count);

    let mut r = format!("{}\n═══ 🔄 转生成功！ ═══", prefix);
    r.push_str(&format!("\n\n🎊 恭喜！完成第{}次转生！", new_count));
    r.push_str(&format!("\n📊 Lv.{} → Lv.{}", old_level, config.reset_level));
    r.push_str(&format!("\n💰 消耗: {}💎", config.cost_diamond));
    r.push_str(&format!("\n📍 已传送到: {}", config.return_map));
    r.push_str(&format!("\n\n✨ 转生加成: 全属性+{}%", new_bonus));
    r.push_str(&format!(
        "\n💡 继续升级，下次转生需要 Lv.{}",
        get_required_level(&config, new_count)
    ));
    r.push_str("\n\n🎉 每次转生都会永久提升5%全属性加成！");

    r
}

/// 转生排行 — 全服转生排名
pub fn cmd_rebirth_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 收集所有转生玩家数据
    let users = db.all_users();
    let mut rankings: Vec<(String, String, i32, String)> = Vec::new();

    for uid in &users {
        let count = get_rebirth_count(db, uid);
        if count > 0 {
            let name = db.read_basic(uid, ITEM_NAME);
            let name = if name.is_empty() { uid.clone() } else { name };
            let date = get_rebirth_date(db, uid);
            rankings.push((uid.clone(), name, count, date));
        }
    }

    // 按转生次数降序排序
    rankings.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut r = format!("{}\n═══ 🔄 转生排行榜 ═══", prefix);

    if rankings.is_empty() {
        r.push_str("\n\n暂无转生记录");
        r.push_str("\n💡 达到转生等级后发送 '转生信息' 了解详情！");
        return r;
    }

    r.push_str(&format!("\n🏆 共 {} 位玩家已转生\n", rankings.len()));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (_, name, count, date)) in rankings.iter().take(10).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let bonus = get_rebirth_bonus_pct(*count);
        r.push_str(&format!(
            "\n{} {}. {} - {}次转生 (属性+{}%)",
            medal,
            i + 1,
            name,
            count,
            bonus
        ));
        if !date.is_empty() {
            r.push_str(&format!(" [{}]", date));
        }
    }

    // 显示当前用户排名
    let my_count = get_rebirth_count(db, user_id);
    if my_count > 0 {
        if let Some(pos) = rankings.iter().position(|(uid, _, _, _)| uid == user_id) {
            r.push_str(&format!("\n\n📍 你的排名: 第{}名 ({}次转生)", pos + 1, my_count));
        }
    } else {
        r.push_str("\n\n📍 你还没有转生记录");
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rebirth_bonus_pct() {
        assert_eq!(get_rebirth_bonus_pct(0), 0);
        assert_eq!(get_rebirth_bonus_pct(1), 5);
        assert_eq!(get_rebirth_bonus_pct(5), 25);
        assert_eq!(get_rebirth_bonus_pct(10), 50);
    }

    #[test]
    fn test_required_level_stages() {
        let config = RebirthConfig {
            min_level: 60,
            cost_diamond: 35,
            reset_level: 31,
            return_map: "主城".to_string(),
            stage_levels: vec![31, 30, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10],
        };

        // 第一次转生需要 31 级
        assert_eq!(get_required_level(&config, 0), 31);
        // 第二次转生需要 30 级
        assert_eq!(get_required_level(&config, 1), 30);
        // 第三次转生需要 10 级
        assert_eq!(get_required_level(&config, 2), 10);
        // 超出阶段使用最后值
        assert_eq!(get_required_level(&config, 20), 10);
    }

    #[test]
    fn test_rebirth_count_range() {
        // 测试转生次数边界
        assert_eq!(get_rebirth_bonus_pct(0), 0);
        assert_eq!(get_rebirth_bonus_pct(100), 500);
    }
}

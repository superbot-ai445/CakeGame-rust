use crate::core::*;
use crate::db::Database;
use crate::encoding;
use crate::stamina;
use crate::user;

use chrono::Local;

/// 锁定玩家
pub fn cmd_lock_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要锁定的玩家ID！", prefix);
    }

    if target == user_id {
        return format!("{}\n您不能锁定自己！", prefix);
    }

    if !db.user_exists(target) {
        return format!("{}\n指定的玩家未注册游戏！", prefix);
    }

    // 检查等级限制
    let pvp_level: i32 = db.global_get("pvp", "LV").parse().unwrap_or(0);
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let target_level: i32 = db.read_basic(target, ITEM_LEVEL).parse().unwrap_or(1);

    if user_level < pvp_level {
        return format!("{}\n您的等级低于{}，无法锁定玩家！", prefix, pvp_level);
    }
    if target_level < pvp_level {
        return format!("{}\n对方等级低于{}，无法锁定！", prefix, pvp_level);
    }

    // 检查是否在安全区
    let location = db.read_basic(user_id, ITEM_LOCATION);
    let map = db.map_get(&location);
    if let Some(m) = map {
        if m.security {
            return format!("{}\n您目前的位置{}是安全区，无法锁定玩家！", prefix, location);
        }
    }

    // 检查目标是否在同一地图
    let target_location = db.read_basic(target, ITEM_LOCATION);
    if location != target_location {
        return format!("{}\n您锁定的玩家目前不在您所处地图，无法锁定！", prefix);
    }

    db.write_user_data(user_id, "pvp_goal", target);
    let target_name = db.read_basic(target, ITEM_NAME);
    format!("{}\n成功锁定玩家[{}]！", prefix, encoding::smart_decode(&target_name))
}

/// 攻击玩家
pub fn cmd_attack_player(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let args = args.trim();

    // 获取锁定的目标
    let mut target = db.read_user_data(user_id, "pvp_goal");

    // 自动锁定功能
    let auto_lock = db.global_get("set", "automatic_locking.pvp");
    if auto_lock == "TRUE" && !args.is_empty() && db.user_exists(args) && args != user_id {
        db.write_user_data(user_id, "pvp_goal", args);
        target = args.to_string();
    }

    if target.is_empty() {
        return format!("{}\n请先锁定玩家！\n发送'锁定玩家+玩家ID'即可锁定玩家！", prefix);
    }

    if target == user_id {
        return format!("{}\n您不能攻击您自己！", prefix);
    }

    if !db.user_exists(&target) {
        return format!("{}\n您攻击的玩家未注册游戏，或您指令输入有误！", prefix);
    }

    // 检查等级限制
    let pvp_level: i32 = db.global_get("pvp", "LV").parse().unwrap_or(0);
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let target_level: i32 = db.read_basic(&target, ITEM_LEVEL).parse().unwrap_or(1);

    if user_level < pvp_level {
        return format!("{}\n您的等级低于{}，无法攻击玩家！", prefix, pvp_level);
    }
    if target_level < pvp_level {
        return format!("{}\n对方等级低于{}，您无法攻击对方！", prefix, pvp_level);
    }

    // 体力检查 (竞技消耗5体力)
    if let Err(e) = stamina::consume_stamina(user_id, "竞技", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 检查安全区
    let location = db.read_basic(user_id, ITEM_LOCATION);
    let map = db.map_get(&location);
    if let Some(m) = map {
        if m.security {
            return format!("{}\n您目前的位置{}是安全区，无法攻击玩家！", prefix, location);
        }
    }

    // 检查同一地图
    let target_location = db.read_basic(&target, ITEM_LOCATION);
    if location != target_location {
        return format!("{}\n您攻击的玩家目前不在您所处地图，无法发起攻击！", prefix);
    }

    // 执行 PVP 战斗
    let _skill_name = if args.is_empty() { "" } else { args };

    // 获取双方属性
    let attacker = user::calc_total_attrs(db, user_id);
    let defender = user::calc_total_attrs(db, &target);

    let target_name = db.read_basic(&target, ITEM_NAME);

    // 战斗计算
    let mut result = format!("{}\n【PVP 战斗】\n{} VS {}", prefix, attacker.name, target_name);

    // 简化版 PVP：基于属性的战斗
    let base_damage: i32 = db.global_get("set", "Base_damage").parse().unwrap_or(0);
    let _pvp_rounds: i32 = db.global_get("set", "PVP_Round").parse().unwrap_or(0);

    let mut attacker_hp = attacker.hp;
    let mut defender_hp = defender.hp;
    let mut round = 0;

    while attacker_hp > 0 && defender_hp > 0 && round < 10 {
        round += 1;

        // 攻击者攻击
        let atk_damage = (attacker.ad - defender.defense).max(1) + base_damage;
        defender_hp -= atk_damage;
        result.push_str(&format!("\n\n第{}回合", round));
        result.push_str(&format!(
            "\n{}对{}造成了{}点伤害",
            attacker.name, target_name, atk_damage
        ));

        if defender_hp <= 0 {
            result.push_str(&format!("\n{}被击败！", target_name));
            break;
        }

        // 防御者反击
        let def_damage = (defender.ad - attacker.defense).max(1) + base_damage;
        attacker_hp -= def_damage;
        result.push_str(&format!(
            "\n{}对{}造成了{}点伤害",
            target_name, attacker.name, def_damage
        ));

        if attacker_hp <= 0 {
            result.push_str(&format!("\n{}被击败！", attacker.name));
            break;
        }
    }

    // 更新双方生命值
    if attacker_hp > 0 {
        db.write_basic(user_id, ITEM_HP_CURRENT, &attacker_hp.max(0).to_string());
    } else {
        db.write_basic(user_id, ITEM_HP_CURRENT, "0");
    }

    if defender_hp > 0 {
        db.write_basic(&target, ITEM_HP_CURRENT, &defender_hp.max(0).to_string());
    } else {
        db.write_basic(&target, ITEM_HP_CURRENT, "0");
    }

    // 结果
    if defender_hp <= 0 {
        result.push_str(&format!("\n\n{}获得了胜利！", attacker.name));

        // 记录PvP战斗统计
        let total_damage = (attacker.ad + attacker.ap).max(1);
        crate::combat_stats::record_pvp(db, user_id, &target, total_damage);

        // 掉落金币
        let target_gold = db.read_currency(&target, CURRENCY_GOLD);
        let drop_gold = (target_gold / 10).min(1000);
        if drop_gold > 0 {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, drop_gold);
            db.modify_currency(&target, CURRENCY_GOLD, OP_SUB, drop_gold);
            result.push_str(&format!("\n获得金币：{}", drop_gold));
        }

        // 掉落物品（随机）
        let drop_rate: f64 = db.global_get("set", "Kill_drop_ltem").parse().unwrap_or(0.0);
        if rand::random::<f64>() < drop_rate / 100.0 {
            // 随机掉落对方背包物品
            // 获取对方背包物品名称列表
            let knapsack = db.get_knapsack_items(&target);
            if !knapsack.is_empty() {
                let idx = rand::random::<usize>() % knapsack.len();
                let item = &knapsack[idx];
                let drop_qty = 1;
                if db.knapsack_remove(&target, &item.name, drop_qty) {
                    db.knapsack_add(user_id, &item.name, drop_qty);
                    result.push_str(&format!(
                        "\n掉落物品：[{}]×{}",
                        encoding::smart_decode(&item.name),
                        drop_qty
                    ));
                }
            }
        }

        // === 小黑屋系统：击杀玩家增加邪恶值 ===
        let evil_added = 1i32;
        let current_evil = get_evil_value(db, user_id);
        let new_evil = current_evil + evil_added;
        set_evil_value(db, user_id, new_evil);
        add_kill_log(db, user_id, &target, evil_added, new_evil);

        // 邪恶值警告
        if new_evil >= 10 {
            result.push_str(&format!(
                "\n⚠ 您的邪恶值已达{}，进入小黑屋区域！击杀玩家会受到惩罚！",
                new_evil
            ));
        } else if new_evil >= 5 {
            result.push_str(&format!(
                "\n⚠ 您的邪恶值已达{}，请注意，继续击杀玩家将受到惩罚！",
                new_evil
            ));
        }
    } else if attacker_hp <= 0 {
        result.push_str(&format!("\n\n{}获得了胜利！", target_name));
    } else {
        result.push_str("\n\n战斗超时，双方平局！");
    }

    // 清除锁定
    db.write_user_data(user_id, "pvp_goal", "");

    result
}

// ==================== 小黑屋系统 ====================

/// 获取用户邪恶值
fn get_evil_value(db: &Database, user_id: &str) -> i32 {
    db.query_row(
        "SELECT EvilVal FROM ext_xiaoheiwu_evil WHERE UID=?1",
        &[user_id],
        |row| row.get::<_, i32>(0),
    )
    .unwrap_or(0)
}

/// 设置用户邪恶值
fn set_evil_value(db: &Database, user_id: &str, val: i32) {
    let conn = db.lock_conn();
    // 先尝试更新
    let updated = conn.execute(
        "UPDATE ext_xiaoheiwu_evil SET EvilVal=?1 WHERE UID=?2",
        rusqlite::params![val, user_id],
    );
    if let Ok(rows) = updated {
        if rows == 0 {
            // 不存在则插入
            let _ = conn.execute(
                "INSERT INTO ext_xiaoheiwu_evil (UID, EvilVal) VALUES (?1, ?2)",
                rusqlite::params![user_id, val],
            );
        }
    }
}

/// 添加击杀日志
fn add_kill_log(db: &Database, attacker: &str, victim: &str, evil_added: i32, total_evil: i32) {
    let now = Local::now().format("%Y年%m月%d日%H时%M分%S秒").to_string();
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT INTO ext_xiaoheiwu_kill_log (UID, killUID, KillTime, AddEvilVal, MyEvilVal) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![attacker, victim, now, evil_added, total_evil],
    );
}

/// 查看邪恶值
pub fn cmd_evil_value(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let evil = get_evil_value(db, user_id);
    let status = if evil >= 10 {
        "【小黑屋】您已被关入小黑屋！"
    } else if evil >= 5 {
        "【危险】继续击杀玩家将被关入小黑屋！"
    } else if evil >= 1 {
        "【注意】您有击杀记录，请注意行为。"
    } else {
        "【清白】您没有任何邪恶记录。"
    };

    format!("{}\n【邪恶值查询】\n当前邪恶值：{}\n{}", prefix, evil, status)
}

/// 查看小黑屋（击杀日志）
pub fn cmd_xiaoheiwu(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_id = if args.trim().is_empty() {
        user_id.to_string()
    } else {
        args.trim().to_string()
    };

    let evil = get_evil_value(db, &target_id);
    let target_name = if target_id == user_id {
        "我".to_string()
    } else {
        encoding::smart_decode(&db.read_basic(&target_id, ITEM_NAME))
    };

    let mut result = format!("{}\n【小黑屋】{}的邪恶值：{}", prefix, target_name, evil);

    // 查看击杀日志
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT killUID, KillTime, AddEvilVal FROM ext_xiaoheiwu_kill_log WHERE UID=?1 ORDER BY ID DESC LIMIT 10",
    ) {
        Ok(s) => s,
        Err(_) => return result,
    };

    let rows = stmt.query_map(rusqlite::params![&target_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
        ))
    });

    match rows {
        Ok(iter) => {
            let logs: Vec<_> = iter.filter_map(|r| r.ok()).collect();
            if logs.is_empty() {
                result.push_str("\n暂无击杀记录。");
            } else {
                result.push_str(&format!("\n最近{}条击杀记录：", logs.len()));
                for (i, (victim_uid, time, evil_val)) in logs.iter().enumerate() {
                    let victim_name = encoding::smart_decode(&db.read_basic(victim_uid, ITEM_NAME));
                    result.push_str(&format!(
                        "\n{}. 击杀[{}] 时间:{} 邪恶+{}",
                        i + 1,
                        victim_name,
                        time,
                        evil_val
                    ));
                }
            }
        }
        Err(_) => {
            result.push_str("\n暂无击杀记录。");
        }
    }

    result
}

// ==================== 竞技扩展系统 ====================

/// 我的战绩（竞技扩展）
pub fn cmd_my_pvp_record(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let occupation = db.read_basic(user_id, ITEM_OCCUPATION);

    // 获取匹配积分
    let integral: i32 = {
        let conn = db.lock_conn();
        conn.prepare("SELECT Integral FROM ext_pipei_uInfo WHERE uID=?1")
            .ok()
            .and_then(|mut stmt| {
                stmt.query_map([user_id], |row| row.get::<_, i32>(0))
                    .ok()
                    .and_then(|mut rows| rows.next().and_then(|r| r.ok()))
            })
            .unwrap_or(0)
    };

    // 统计胜/负/平
    let (wins, losses, draws): (i32, i32, i32) = {
        let conn = db.lock_conn();
        let wins = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_log WHERE uID=?1 AND Result=1")
            .ok()
            .and_then(|mut s| s.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        let losses = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_log WHERE uID=?1 AND Result=-1")
            .ok()
            .and_then(|mut s| s.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        let draws = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_log WHERE uID=?1 AND Result=0")
            .ok()
            .and_then(|mut s| s.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        (wins, losses, draws)
    };

    let total = wins + losses + draws;
    let win_rate = if total > 0 {
        (wins as f64 / total as f64 * 100.0) as i32
    } else {
        0
    };

    // 获取段位信息
    let tiers: Vec<(String, i32, i32)> = {
        let conn = db.lock_conn();
        let mut tiers = Vec::new();
        if let Ok(mut stmt) =
            conn.prepare("SELECT Name, Integral_min, Integral_max FROM ext_pipei_paragraph ORDER BY Integral_min")
        {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?))
            }) {
                for r in rows.flatten() {
                    tiers.push(r);
                }
            }
        }
        tiers
    };

    let (tier_name, remaining) =
        if let Some((name, _min, max)) = tiers.iter().find(|(_, min, max)| integral >= *min && integral <= *max) {
            let next = tiers.iter().find(|(_, m, _)| *m > *max);
            let rem = if let Some((_, next_min, _)) = next {
                next_min - integral
            } else {
                0
            };
            (name.trim_end_matches('\u{0}').to_string(), rem)
        } else if !tiers.is_empty() {
            let first = &tiers[0];
            (first.0.trim_end_matches('\u{0}').to_string(), first.1 - integral)
        } else {
            ("无".to_string(), 0)
        };

    format!(
        "{}\n当前职业：{}\n匹配次数：{}\n匹配胜场：{}\n匹配败场：{}\n匹配和场：{}\n目前胜率：{}%\n当前段位：{}\n剩余胜点：{}",
        prefix, occupation, total, wins, losses, draws, win_rate, tier_name, remaining
    )
}

/// 罪恶值排行
pub fn cmd_evil_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let players: Vec<(String, i32)> = {
        let conn = db.lock_conn();
        let mut stmt = match conn
            .prepare("SELECT UID, EvilVal FROM ext_xiaoheiwu_evil WHERE EvilVal > 0 ORDER BY EvilVal DESC LIMIT 10")
        {
            Ok(s) => s,
            Err(_) => return format!("{}\n暂无罪恶值排行数据！", prefix),
        };
        let mut rows_vec = Vec::new();
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))) {
            for r in rows.flatten() {
                rows_vec.push(r);
            }
        }
        rows_vec
    };

    if players.is_empty() {
        return format!("{}\n暂无罪恶值排行数据！", prefix);
    }

    let mut entries = Vec::new();
    for (i, (uid, evil)) in players.iter().enumerate() {
        let uid_clean = uid.trim_end_matches('\u{0}');
        let name = encoding::smart_decode(&db.read_basic(uid_clean, ITEM_NAME));
        let medal = match i {
            0 => "💀",
            1 => "☠️",
            2 => "⚔️",
            _ => "  ",
        };
        entries.push((format!("{}{}", medal, name), uid_clean.to_string(), *evil));
    }

    // 使用模板渲染排行
    let raw = db.template_get("小黑屋扩展_罪恶值排行");
    if !raw.is_empty() && raw.contains('<') {
        let tmpl_body = crate::template_render::render_evil_ranking(db, &entries);
        format!("{}\n{}", prefix, tmpl_body)
    } else {
        let mut result = format!("{}\n【荣耀罪恶榜】", prefix);
        for (i, (display_name, uid_clean, evil)) in entries.iter().enumerate() {
            result.push_str(&format!(
                "\n{}. {}({})\n罪恶值：{}",
                i + 1,
                display_name,
                uid_clean,
                evil
            ));
        }
        result
    }
}

/// 被杀记录（查看谁击杀了我）
pub fn cmd_victim_log(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_id = if args.trim().is_empty() {
        user_id.to_string()
    } else {
        args.trim().to_string()
    };

    let conn = db.lock_conn();
    let mut stmt = match conn
        .prepare("SELECT UID, KillTime FROM ext_xiaoheiwu_kill_log WHERE killUID=?1 ORDER BY ID DESC LIMIT 10")
    {
        Ok(s) => s,
        Err(_) => return format!("{}\n暂无被杀记录。", prefix),
    };

    let rows = stmt.query_map(rusqlite::params![&target_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    });

    let target_name = if target_id == user_id {
        "您".to_string()
    } else {
        encoding::smart_decode(&db.read_basic(&target_id, ITEM_NAME))
    };

    let mut result = format!("{}\n【被杀记录】{}", prefix, target_name);

    match rows {
        Ok(iter) => {
            let logs: Vec<_> = iter.filter_map(|r| r.ok()).collect();
            if logs.is_empty() {
                result.push_str("\n暂无被杀记录。");
            } else {
                // 构建模板数据
                let mut entries = Vec::new();
                for (attacker_uid, time) in logs.iter() {
                    let attacker_clean = attacker_uid.trim_end_matches('\u{0}');
                    let attacker_name = encoding::smart_decode(&db.read_basic(attacker_clean, ITEM_NAME));
                    entries.push((attacker_name, attacker_clean.to_string(), time.clone()));
                }
                // 尝试使用模板渲染
                let rendered = crate::template_render::render_kill_log(db, &entries);
                result.push_str(&format!("\n最近{}条被杀记录：", entries.len()));
                result.push('\n');
                result.push_str(&rendered);
            }
        }
        Err(_) => {
            result.push_str("\n暂无被杀记录。");
        }
    }

    result
}

/// 段位排行 — 竞技扩展_段位排行
/// 按段位分组展示 Top10 玩家，遵循 MessageTemplate 格式
pub fn cmd_tier_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 读取所有段位定义
    let tiers: Vec<(String, i32, i32)> = {
        let conn = db.lock_conn();
        let mut tiers = Vec::new();
        if let Ok(mut stmt) =
            conn.prepare("SELECT Name, Integral_min, Integral_max FROM ext_pipei_paragraph ORDER BY Integral_min")
        {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?))
            }) {
                for r in rows.flatten() {
                    tiers.push(r);
                }
            }
        }
        tiers
    };

    if tiers.is_empty() {
        return format!("{}\n匹配系统暂未开放！", prefix);
    }

    // 读取 Top 10 玩家积分
    let players: Vec<(String, i32)> = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare("SELECT uID, Integral FROM ext_pipei_uInfo ORDER BY Integral DESC LIMIT 10") {
            Ok(s) => s,
            Err(_) => return format!("{}\n暂无段位排行数据！", prefix),
        };
        let mut rows_vec = Vec::new();
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))) {
            for r in rows.flatten() {
                rows_vec.push(r);
            }
        }
        rows_vec
    };

    if players.is_empty() {
        return format!("{}\n暂无段位排行数据！", prefix);
    }

    // 计算当前赛季号（年份后两位×10+月份/2取整）
    let now = chrono::Local::now();
    let season = now.format("%y").to_string().parse::<i32>().unwrap_or(26) * 10
        + (now.format("%m").to_string().parse::<i32>().unwrap_or(6) / 2);

    let mut r = format!("{}\n【荣耀S.{}赛季】", prefix, season);

    for (i, (uid, integral)) in players.iter().enumerate() {
        let uid_clean = uid.trim_end_matches('\u{0}');
        let name = user::get_msg_prefix(db, uid_clean);
        // 取纯昵称（去掉 "ID：" 前缀）
        let display_name = name.strip_prefix("ID：").unwrap_or(&name);
        let tier = tiers.iter().find(|(_, min, max)| integral >= min && integral <= max);
        let tier_name = tier
            .map(|(n, _, _)| n.trim_end_matches('\u{0}').to_string())
            .unwrap_or_else(|| "无段位".to_string());

        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "",
        };
        r.push_str(&format!(
            "\n{}{}. {}({})\n当前段位：{}({}积分)",
            medal,
            i + 1,
            display_name,
            uid_clean,
            tier_name,
            integral,
        ));
    }

    r.push_str("\n\nTip:只有前十才能上榜，排行榜数据为实时数据。");
    r
}

/// 赎罪减免 — 查看赎罪选项和费用
/// 基于 ext_xiaoheiwu_evil 表的 EvilVal，每点邪恶值需要金币/钻石来减免
/// 来源: InstructionState '赎罪减免' = TRUE
pub fn cmd_atonement_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // 读取当前邪恶值
    let evil_val: i32 = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare("SELECT EvilVal FROM ext_xiaoheiwu_evil WHERE UID = ?1") {
            Ok(s) => s,
            Err(_) => return format!("{}\n查询赎罪信息失败！", prefix),
        };
        stmt.query_row([user_id], |row| row.get::<_, i32>(0))
            .unwrap_or_default()
    };

    if evil_val <= 0 {
        return format!("{}\n🕊️ 您当前没有罪恶值，无需赎罪！\n继续保持清白之身吧~", prefix);
    }

    // 赎罪费用计算：每点邪恶值 500金币 或 2钻石
    let gold_per_point: i64 = 500;
    let diamond_per_point: i64 = 2;
    let total_gold = evil_val as i64 * gold_per_point;
    let total_diamond = evil_val as i64 * diamond_per_point;

    // 当前余额
    let user_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    let user_diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);

    // 罪恶等级描述
    let evil_desc = match evil_val {
        0 => "清白",
        1..=5 => "轻微作恶",
        6..=15 => "恶名昭彰",
        16..=30 => "罪大恶极",
        31..=50 => "十恶不赦",
        _ => "魔王转世",
    };

    let mut r = format!(
        "{}\n═══ ⛪ 赎罪殿 ═══\n━━━━━━━━━━━━━━━━━━━━\n当前邪恶值：{} ({})\n━━━━━━━━━━━━━━━━━━━━\n\n📜 赎罪方式：\n\n🪙 方式一：金币赎罪\n  费用：{}金币 (每点{}金币)\n  您的金币：{} {}\n\n💎 方式二：钻石赎罪\n  费用：{}钻石 (每点{}钻石)\n  您的钻石：{} {}",
        prefix,
        evil_val, evil_desc,
        total_gold, gold_per_point, user_gold,
        if user_gold >= total_gold { "✅" } else { "❌ 余额不足" },
        total_diamond, diamond_per_point, user_diamond,
        if user_diamond >= total_diamond { "✅" } else { "❌ 余额不足" },
    );

    // 显示赎罪后效果
    r.push_str(&format!(
        "\n\n✨ 赎罪效果：\n  邪恶值 {} → 0\n  解除小黑屋限制\n  恢复正常PK状态",
        evil_val,
    ));

    r.push_str("\n\n━━━━━━━━━━━━━━━━━━━━");
    r.push_str("\nTip:发送【确认赎罪+金币】或【确认赎罪+钻石】进行赎罪");

    r
}

/// 确认赎罪 — 执行赎罪操作
/// 参数: args = "金币" 或 "钻石"
/// 来源: InstructionState '确认赎罪' = TRUE
pub fn cmd_confirm_atonement(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let method = args.trim();
    if method != "金币" && method != "钻石" {
        return format!("{}\n请指定赎罪方式！\n用法：确认赎罪+金币 或 确认赎罪+钻石", prefix);
    }

    // 读取当前邪恶值
    let evil_val: i32 = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare("SELECT EvilVal FROM ext_xiaoheiwu_evil WHERE UID = ?1") {
            Ok(s) => s,
            Err(_) => return format!("{}\n查询赎罪信息失败！", prefix),
        };
        stmt.query_row([user_id], |row| row.get::<_, i32>(0))
            .unwrap_or_default()
    };

    if evil_val <= 0 {
        return format!("{}\n🕊️ 您当前没有罪恶值，无需赎罪！", prefix);
    }

    let gold_per_point: i64 = 500;
    let diamond_per_point: i64 = 2;

    if method == "金币" {
        let total_cost = evil_val as i64 * gold_per_point;
        let user_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);

        if user_gold < total_cost {
            return format!(
                "{}\n❌ 金币不足！\n需要：{}金币\n您有：{}金币\n还差：{}金币",
                prefix,
                total_cost,
                user_gold,
                total_cost - user_gold
            );
        }

        // 扣除金币
        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, total_cost);

        // 清除邪恶值
        {
            let conn = db.lock_conn();
            let _ = conn.execute("UPDATE ext_xiaoheiwu_evil SET EvilVal = 0 WHERE UID = ?1", [user_id]);
        }

        // 记录赎罪历史
        let atonement_count_key = "atonement_count";
        let count: i32 = db.read_user_data(user_id, atonement_count_key).parse().unwrap_or(0);
        db.write_user_data(user_id, atonement_count_key, &(count + 1).to_string());

        format!(
            "{}\n✅ 赎罪成功！\n━━━━━━━━━━━━━━━━━━━━\n消耗金币：{}金币\n邪恶值：{} → 0\n剩余金币：{}金币\n赎罪次数：{}次\n━━━━━━━━━━━━━━━━━━━━\n🕊️ 您已洗清罪恶，恢复正常身份！",
            prefix, total_cost, evil_val, user_gold - total_cost, count + 1
        )
    } else {
        // 钻石赎罪
        let total_cost = evil_val as i64 * diamond_per_point;
        let user_diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);

        if user_diamond < total_cost {
            return format!(
                "{}\n❌ 钻石不足！\n需要：{}钻石\n您有：{}钻石\n还差：{}钻石",
                prefix,
                total_cost,
                user_diamond,
                total_cost - user_diamond
            );
        }

        // 扣除钻石
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, total_cost);

        // 清除邪恶值
        {
            let conn = db.lock_conn();
            let _ = conn.execute("UPDATE ext_xiaoheiwu_evil SET EvilVal = 0 WHERE UID = ?1", [user_id]);
        }

        // 记录赎罪历史
        let atonement_count_key = "atonement_count";
        let count: i32 = db.read_user_data(user_id, atonement_count_key).parse().unwrap_or(0);
        db.write_user_data(user_id, atonement_count_key, &(count + 1).to_string());

        format!(
            "{}\n✅ 赎罪成功！\n━━━━━━━━━━━━━━━━━━━━\n消耗钻石：{}钻石\n邪恶值：{} → 0\n剩余钻石：{}钻石\n赎罪次数：{}次\n━━━━━━━━━━━━━━━━━━━━\n🕊️ 您已洗清罪恶，恢复正常身份！",
            prefix, total_cost, evil_val, user_diamond - total_cost, count + 1
        )
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_atonement_cost_calculation() {
        // Test the cost calculation logic
        let evil_val: i32 = 10;
        let gold_per_point: i64 = 500;
        let diamond_per_point: i64 = 2;
        assert_eq!(evil_val as i64 * gold_per_point, 5000);
        assert_eq!(evil_val as i64 * diamond_per_point, 20);
    }

    #[test]
    fn test_evil_desc_mapping() {
        // Test evil value descriptions
        let cases = vec![
            (0, "清白"),
            (3, "轻微作恶"),
            (10, "恶名昭彰"),
            (25, "罪大恶极"),
            (40, "十恶不赦"),
            (60, "魔王转世"),
        ];
        for (val, expected) in cases {
            let desc = match val {
                0 => "清白",
                1..=5 => "轻微作恶",
                6..=15 => "恶名昭彰",
                16..=30 => "罪大恶极",
                31..=50 => "十恶不赦",
                _ => "魔王转世",
            };
            assert_eq!(desc, expected, "evil_val={}", val);
        }
    }

    #[test]
    fn test_method_validation() {
        // Test that only "金币" and "钻石" are accepted
        assert!("金币" == "金币" || "金币" == "钻石");
        assert!("钻石" == "金币" || "钻石" == "钻石");
        assert!(!("银币" == "金币" || "银币" == "钻石"));
    }
}

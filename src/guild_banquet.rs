/// CakeGame 公会盛宴系统
/// 公会成员共同参与的宴会，提供临时增益buff
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD};
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const SECTION: &str = "guild_feast";

/// 宴会等级定义
struct FeastDef {
    name: &'static str,
    emoji: &'static str,
    cost_gold: i64,
    cost_diamond: i64,
    /// 持续时间（分钟）
    duration_min: i32,
    /// 增益描述
    bonus_desc: &'static str,
    /// HP加成百分比
    bonus_hp_pct: i32,
    /// AD加成百分比
    bonus_ad_pct: i32,
    /// AP加成百分比
    bonus_ap_pct: i32,
    /// 经验加成百分比
    bonus_exp_pct: i32,
    /// 金币加成百分比
    bonus_gold_pct: i32,
    /// 参与奖励金币
    reward_gold: i64,
    /// 参与奖励钻石
    reward_diamond: i64,
    /// 参与奖励经验
    reward_exp: i32,
    /// 最大参与人数
    max_participants: usize,
}

const FEASTS: &[FeastDef] = &[
    FeastDef {
        name: "家常便饭",
        emoji: "🍲",
        cost_gold: 5000,
        cost_diamond: 0,
        duration_min: 30,
        bonus_desc: "全属性小幅提升",
        bonus_hp_pct: 5,
        bonus_ad_pct: 5,
        bonus_ap_pct: 5,
        bonus_exp_pct: 10,
        bonus_gold_pct: 5,
        reward_gold: 200,
        reward_diamond: 5,
        reward_exp: 100,
        max_participants: 20,
    },
    FeastDef {
        name: "丰盛佳宴",
        emoji: "🍖",
        cost_gold: 20000,
        cost_diamond: 10,
        duration_min: 60,
        bonus_desc: "全属性中等提升",
        bonus_hp_pct: 10,
        bonus_ad_pct: 10,
        bonus_ap_pct: 10,
        bonus_exp_pct: 20,
        bonus_gold_pct: 10,
        reward_gold: 500,
        reward_diamond: 15,
        reward_exp: 300,
        max_participants: 30,
    },
    FeastDef {
        name: "皇家盛宴",
        emoji: "👑",
        cost_gold: 100000,
        cost_diamond: 50,
        duration_min: 120,
        bonus_desc: "全属性大幅提升",
        bonus_hp_pct: 20,
        bonus_ad_pct: 20,
        bonus_ap_pct: 20,
        bonus_exp_pct: 50,
        bonus_gold_pct: 20,
        reward_gold: 2000,
        reward_diamond: 50,
        reward_exp: 800,
        max_participants: 50,
    },
    FeastDef {
        name: "神域仙宴",
        emoji: "🌟",
        cost_gold: 500000,
        cost_diamond: 200,
        duration_min: 180,
        bonus_desc: "全属性极致提升",
        bonus_hp_pct: 30,
        bonus_ad_pct: 30,
        bonus_ap_pct: 30,
        bonus_exp_pct: 100,
        bonus_gold_pct: 30,
        reward_gold: 5000,
        reward_diamond: 100,
        reward_exp: 2000,
        max_participants: 50,
    },
];

fn today_str() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn now_ts() -> i64 {
    chrono::Local::now().timestamp()
}

fn gen_feast_id(guild: &str, ts: i64) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{}_{}", guild, ts).hash(&mut hasher);
    format!("F{:08X}", hasher.finish() as u32)
}

/// 格式化千分位数字
fn format_num(n: i64) -> String {
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

/// 进度条
fn progress_bar(current: i64, max: i64, width: usize) -> String {
    let pct = if max > 0 {
        (current.min(max) as f64 / max as f64 * width as f64) as usize
    } else {
        0
    };
    let filled = pct.min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// 查看公会盛宴：显示当前进行中的盛宴和历史
pub fn cmd_view_feast(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = db.global_get("user_guild", user_id);
    if guild.is_empty() {
        return format!("{}\n您还未加入公会！", prefix);
    }

    let section = format!("{}_{}", SECTION, guild);
    let mut r = format!("{}\n═══ 🍽️ {}公会盛宴 ═══\n", prefix, guild);

    // 当前进行中的盛宴
    let current_id = db.global_get(&section, "current_feast");
    if !current_id.is_empty() {
        let feast_name = db.global_get(&section, &format!("feast_{}_name", current_id));
        let feast_emoji = db.global_get(&section, &format!("feast_{}_emoji", current_id));
        let feast_host = db.global_get(&section, &format!("feast_{}_host", current_id));
        let feast_ts: i64 = db
            .global_get(&section, &format!("feast_{}_ts", current_id))
            .parse()
            .unwrap_or(0);
        let feast_dur: i32 = db
            .global_get(&section, &format!("feast_{}_dur", current_id))
            .parse()
            .unwrap_or(0);
        let participants_raw = db.global_get(&section, &format!("feast_{}_parts", current_id));
        let part_count = if participants_raw.is_empty() {
            0
        } else {
            participants_raw.split(',').filter(|s| !s.is_empty()).count()
        };
        let max_parts: usize = db
            .global_get(&section, &format!("feast_{}_max", current_id))
            .parse()
            .unwrap_or(20);

        let elapsed_min = ((now_ts() - feast_ts) / 60).max(0) as i32;
        let remaining = (feast_dur - elapsed_min).max(0);

        if remaining > 0 {
            let bonus_exp: i32 = db
                .global_get(&section, &format!("feast_{}_exp_pct", current_id))
                .parse()
                .unwrap_or(0);
            let bonus_gold: i32 = db
                .global_get(&section, &format!("feast_{}_gold_pct", current_id))
                .parse()
                .unwrap_or(0);

            r.push_str(&format!("\n🎉 当前盛宴: {} {}\n", feast_emoji, feast_name));
            r.push_str(&format!("📢 发起人: {}\n", feast_host));
            r.push_str(&format!("👥 参与人数: {}/{}\n", part_count, max_parts));
            r.push_str(&format!("⏱️ 剩余时间: {}分钟\n", remaining));
            r.push_str(&format!("✨ 增益: 经验+{}% 金币+{}%\n", bonus_exp, bonus_gold));
            r.push_str(&format!(
                "📊 参与进度: {} {}/{}\n",
                progress_bar(part_count as i64, max_parts as i64, 10),
                part_count,
                max_parts
            ));

            // 检查当前用户是否已参与
            if participants_raw.split(',').any(|s| s == user_id) {
                r.push_str("\n✅ 您已参加本次盛宴，享受增益中！");
            } else {
                r.push_str("\n💡 输入「参加盛宴」加入本次宴会！");
            }
        } else {
            r.push_str("\n🍽️ 当前无进行中的盛宴\n");
            r.push_str("💡 公会成员可输入「发起盛宴」举办宴会\n");
        }
    } else {
        r.push_str("\n🍽️ 当前无进行中的盛宴\n");
        r.push_str("💡 公会成员可输入「发起盛宴」举办宴会\n");
    }

    // 宴会等级预览
    r.push_str("\n\n═══ 📋 宴会等级 ═══\n");
    for (i, def) in FEASTS.iter().enumerate() {
        r.push_str(&format!(
            "\n{} {} {} — {}金{}{}",
            i + 1,
            def.emoji,
            def.name,
            format_num(def.cost_gold),
            if def.cost_diamond > 0 {
                format!("+{}💎", def.cost_diamond)
            } else {
                String::new()
            },
            if i == 0 { " (入门)" } else { "" }
        ));
        r.push_str(&format!(
            "\n   {} | 持续{}分钟 | 最多{}人",
            def.bonus_desc,
            def.duration_min,
            max_participants_str(def.max_participants)
        ));
    }

    // 今日盛宴统计
    let today = today_str();
    let history = db.global_get(&section, "history");
    let today_count = history.split('~').filter(|entry| entry.contains(&today)).count();
    r.push_str(&format!("\n\n📅 今日已举办: {}场盛宴", today_count));

    r
}

fn max_participants_str(n: usize) -> String {
    format!("{}人", n)
}

/// 发起盛宴：公会成员消耗金币/钻石举办宴会
pub fn cmd_host_feast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = db.global_get("user_guild", user_id);
    if guild.is_empty() {
        return format!("{}\n您还未加入公会！", prefix);
    }

    // 检查虚弱状态
    let weak_remaining = user::check_weakness(db, user_id);
    if weak_remaining > 0 {
        return format!("{}\n操作失败！您正处于虚弱状态（剩余{}秒）", prefix, weak_remaining);
    }

    let section = format!("{}_{}", SECTION, guild);

    // 检查是否有进行中的盛宴
    let current_id = db.global_get(&section, "current_feast");
    if !current_id.is_empty() {
        let feast_ts: i64 = db
            .global_get(&section, &format!("feast_{}_ts", current_id))
            .parse()
            .unwrap_or(0);
        let feast_dur: i32 = db
            .global_get(&section, &format!("feast_{}_dur", current_id))
            .parse()
            .unwrap_or(0);
        let elapsed_min = ((now_ts() - feast_ts) / 60).max(0) as i32;
        if feast_dur - elapsed_min > 0 {
            return format!("{}\n❌ 公会正在进行盛宴，请等待结束后再发起！", prefix);
        }
    }

    // 每日限制：每人每天最多发起2场盛宴
    let today = today_str();
    let host_key = format!("host_count_{}_{}", user_id, today);
    let host_count: i32 = db.global_get(&section, &host_key).parse().unwrap_or(0);
    if host_count >= 2 {
        return format!("{}\n❌ 您今日已发起2场盛宴，请明日再来！", prefix);
    }

    // 解析宴会等级
    let args_trimmed = args.trim();
    let feast_idx = if args_trimmed.is_empty() {
        0
    } else if let Ok(n) = args_trimmed.parse::<usize>() {
        if n == 0 || n > FEASTS.len() {
            return format!("{}\n❌ 无效的宴会等级！请输入1-{}的数字。", prefix, FEASTS.len());
        }
        n - 1
    } else {
        // 模糊匹配名称
        match FEASTS
            .iter()
            .position(|f| f.name.contains(args_trimmed) || args_trimmed.contains(f.name))
        {
            Some(idx) => idx,
            None => {
                return format!("{}\n❌ 未找到名为「{}」的宴会等级！", prefix, args_trimmed);
            }
        }
    };

    let def = &FEASTS[feast_idx];

    // 扣除金币
    if def.cost_gold > 0 {
        let after = db.modify_currency(user_id, CURRENCY_GOLD, "sub", def.cost_gold);
        if after < 0 {
            db.modify_currency(user_id, CURRENCY_GOLD, "add", def.cost_gold);
            return format!(
                "{}\n❌ 金币不足！发起{}需要{}金币，当前{}金币。",
                prefix,
                def.name,
                format_num(def.cost_gold),
                format_num(after + def.cost_gold)
            );
        }
    }

    // 扣除钻石
    if def.cost_diamond > 0 {
        let after = db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", def.cost_diamond as i64);
        if after < 0 {
            db.modify_currency(user_id, CURRENCY_DIAMOND, "add", def.cost_diamond as i64);
            if def.cost_gold > 0 {
                db.modify_currency(user_id, CURRENCY_GOLD, "add", def.cost_gold);
            }
            return format!(
                "{}\n❌ 钻石不足！发起{}需要{}钻石，当前{}钻石。",
                prefix,
                def.name,
                def.cost_diamond,
                format_num(after + def.cost_diamond as i64)
            );
        }
    }

    let ts = now_ts();
    let feast_id = gen_feast_id(&guild, ts);

    // 写入盛宴数据
    db.global_set(&section, "current_feast", &feast_id);
    db.global_set(&section, &format!("feast_{}_name", feast_id), def.name);
    db.global_set(&section, &format!("feast_{}_emoji", feast_id), def.emoji);
    db.global_set(
        &section,
        &format!("feast_{}_host", feast_id),
        &user::get_msg_prefix(db, user_id),
    );
    db.global_set(&section, &format!("feast_{}_host_id", feast_id), user_id);
    db.global_set(&section, &format!("feast_{}_ts", feast_id), &ts.to_string());
    db.global_set(
        &section,
        &format!("feast_{}_dur", feast_id),
        &def.duration_min.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_max", feast_id),
        &def.max_participants.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_hp_pct", feast_id),
        &def.bonus_hp_pct.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_ad_pct", feast_id),
        &def.bonus_ad_pct.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_ap_pct", feast_id),
        &def.bonus_ap_pct.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_exp_pct", feast_id),
        &def.bonus_exp_pct.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_gold_pct", feast_id),
        &def.bonus_gold_pct.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_reward_gold", feast_id),
        &def.reward_gold.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_reward_diamond", feast_id),
        &def.reward_diamond.to_string(),
    );
    db.global_set(
        &section,
        &format!("feast_{}_reward_exp", feast_id),
        &def.reward_exp.to_string(),
    );
    // 发起人自动参与
    db.global_set(&section, &format!("feast_{}_parts", feast_id), user_id);

    // 更新发起次数
    db.global_set(&section, &host_key, &(host_count + 1).to_string());

    // 更新历史记录
    let mut history = db.global_get(&section, "history");
    if history.len() > 2000 {
        // 截断旧记录
        let parts: Vec<&str> = history.split('~').collect();
        history = parts
            .iter()
            .skip(parts.len() / 2)
            .cloned()
            .collect::<Vec<_>>()
            .join("~");
    }
    let entry = format!(
        "{}|{}|{}|{}",
        today,
        feast_id,
        def.name,
        user::get_msg_prefix(db, user_id)
    );
    if history.is_empty() {
        history = entry;
    } else {
        history = format!("{}~{}", history, entry);
    }
    db.global_set(&section, "history", &history);

    // 更新总发起统计
    let total_key = "total_feasts_hosted";
    let total: i32 = db.global_get(&section, total_key).parse().unwrap_or(0);
    db.global_set(&section, total_key, &(total + 1).to_string());

    // 更新个人贡献
    let contrib_key = format!("contrib_{}", user_id);
    let contrib: i32 = db.global_get(&section, &contrib_key).parse().unwrap_or(0);
    db.global_set(&section, &contrib_key, &(contrib + 1).to_string());

    let mut r = format!("{}\n", prefix);
    r.push_str(&format!("═══ {} {} 发起了盛宴！ ═══\n", def.emoji, def.name));
    r.push_str(&format!("💰 消耗: {}金币", format_num(def.cost_gold)));
    if def.cost_diamond > 0 {
        r.push_str(&format!(" + {}💎", def.cost_diamond));
    }
    r.push_str(&format!("\n⏰ 持续时间: {}分钟\n", def.duration_min));
    r.push_str(&format!(
        "✨ 增益: HP+{}% AD+{}% AP+{}% 经验+{}% 金币+{}%\n",
        def.bonus_hp_pct, def.bonus_ad_pct, def.bonus_ap_pct, def.bonus_exp_pct, def.bonus_gold_pct
    ));
    r.push_str(&format!(
        "🎁 参与奖励: {}金币 + {}💎 + {}经验\n",
        format_num(def.reward_gold),
        def.reward_diamond,
        def.reward_exp
    ));
    r.push_str(&format!("👥 最多{}人参与\n", def.max_participants));
    r.push_str("\n📢 公会成员请在盛宴结束前输入「参加盛宴」领取奖励！");

    r
}

/// 参加盛宴：领取奖励
pub fn cmd_join_feast(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = db.global_get("user_guild", user_id);
    if guild.is_empty() {
        return format!("{}\n您还未加入公会！", prefix);
    }

    let section = format!("{}_{}", SECTION, guild);
    let current_id = db.global_get(&section, "current_feast");
    if current_id.is_empty() {
        return format!("{}\n❌ 当前没有进行中的盛宴！", prefix);
    }

    // 检查盛宴是否结束
    let feast_ts: i64 = db
        .global_get(&section, &format!("feast_{}_ts", current_id))
        .parse()
        .unwrap_or(0);
    let feast_dur: i32 = db
        .global_get(&section, &format!("feast_{}_dur", current_id))
        .parse()
        .unwrap_or(0);
    let elapsed_min = ((now_ts() - feast_ts) / 60).max(0) as i32;
    if feast_dur - elapsed_min <= 0 {
        return format!("{}\n❌ 盛宴已结束！", prefix);
    }

    // 检查是否已参与
    let participants_raw = db.global_get(&section, &format!("feast_{}_parts", current_id));
    if participants_raw.split(',').any(|s| s == user_id) {
        return format!("{}\n✅ 您已参加本次盛宴，正在享受增益中！", prefix);
    }

    // 检查参与人数限制
    let max_parts: usize = db
        .global_get(&section, &format!("feast_{}_max", current_id))
        .parse()
        .unwrap_or(20);
    let part_count = participants_raw.split(',').filter(|s| !s.is_empty()).count();
    if part_count >= max_parts {
        return format!("{}\n❌ 盛宴参与人数已满({}人)！", prefix, max_parts);
    }

    // 添加参与者
    let new_parts = if participants_raw.is_empty() {
        user_id.to_string()
    } else {
        format!("{},{}", participants_raw, user_id)
    };
    db.global_set(&section, &format!("feast_{}_parts", current_id), &new_parts);

    // 发放奖励
    let reward_gold: i64 = db
        .global_get(&section, &format!("feast_{}_reward_gold", current_id))
        .parse()
        .unwrap_or(0);
    let reward_diamond: i64 = db
        .global_get(&section, &format!("feast_{}_reward_diamond", current_id))
        .parse()
        .unwrap_or(0);
    let reward_exp: i32 = db
        .global_get(&section, &format!("feast_{}_reward_exp", current_id))
        .parse()
        .unwrap_or(0);

    if reward_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", reward_gold);
    }
    if reward_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", reward_diamond);
    }
    if reward_exp > 0 {
        user::add_experience(db, user_id, reward_exp);
    }

    let feast_name = db.global_get(&section, &format!("feast_{}_name", current_id));
    let feast_emoji = db.global_get(&section, &format!("feast_{}_emoji", current_id));

    let mut r = format!("{}\n", prefix);
    r.push_str("═══ 🎉 参加盛宴成功！ ═══\n");
    r.push_str(&format!("{} {}\n", feast_emoji, feast_name));
    r.push_str(&format!(
        "\n🎁 获得奖励:\n  💰 {}金币\n  💎 {}钻石\n  ⭐ {}经验",
        format_num(reward_gold),
        reward_diamond,
        reward_exp
    ));

    let remaining = feast_dur - elapsed_min;
    r.push_str(&format!("\n\n⏱️ 盛宴剩余: {}分钟", remaining));
    r.push_str(&format!("\n👥 参与人数: {}/{}", part_count + 1, max_parts));

    // 活跃积分记录
    crate::activity_points::record_activity(db, user_id, "guild", 1);

    r
}

/// 宴会排行：按举办次数排名
pub fn cmd_feast_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild = db.global_get("user_guild", user_id);
    if guild.is_empty() {
        return format!("{}\n您还未加入公会！", prefix);
    }

    let section = format!("{}_{}", SECTION, guild);

    // 收集所有贡献者数据（两阶段避免死锁）
    let sections_and_keys: Vec<(String, i32)> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare(&format!(
            "SELECT ID, DATA FROM Global WHERE SECTION = '{}' AND ID LIKE 'contrib_%'",
            section
        )) {
            if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
                for row in rows.flatten() {
                    let (key, data) = row;
                    let uid = key.strip_prefix("contrib_").unwrap_or(&key);
                    let count: i32 = data.parse().unwrap_or(0);
                    if count > 0 {
                        result.push((uid.to_string(), count));
                    }
                }
            }
        }
        result
    };

    let mut entries: Vec<(String, i32)> = Vec::new();
    for (uid, count) in &sections_and_keys {
        let name = user::get_msg_prefix(db, uid);
        entries.push((name, *count));
    }

    if entries.is_empty() {
        return format!(
            "{}\n🏰 {} 暂无盛宴举办记录\n\n💡 输入「发起盛宴」举办第一场宴会！",
            prefix, guild
        );
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n═══ 🏆 {} 宴会排行 ═══\n", prefix, guild);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, count)) in entries.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        r.push_str(&format!("\n{} {} — {}场盛宴", medal, name, count));
    }

    // 当前用户排名
    let my_name = user::get_msg_prefix(db, user_id);
    if let Some(rank) = entries.iter().position(|(name, _)| name == &my_name) {
        r.push_str(&format!("\n\n📍 您的排名: 第{}名 ({}场)", rank + 1, entries[rank].1));
    }

    // 公会统计
    let total: i32 = db.global_get(&section, "total_feasts_hosted").parse().unwrap_or(0);
    r.push_str(&format!("\n\n📊 公会累计举办: {}场盛宴", total));

    r
}

/// 宴会信息：查看盛宴增益详情
pub fn cmd_feast_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let args_trimmed = args.trim();

    // 如果没有指定参数，显示当前盛宴的增益信息
    if args_trimmed.is_empty() {
        let guild = db.global_get("user_guild", user_id);
        if guild.is_empty() {
            return format!("{}\n您还未加入公会！", prefix);
        }

        let section = format!("{}_{}", SECTION, guild);
        let current_id = db.global_get(&section, "current_feast");
        if current_id.is_empty() {
            return format!(
                "{}\n❌ 当前没有进行中的盛宴！\n\n💡 输入「宴会信息+盛宴名称」查看详细信息",
                prefix
            );
        }

        let feast_ts: i64 = db
            .global_get(&section, &format!("feast_{}_ts", current_id))
            .parse()
            .unwrap_or(0);
        let feast_dur: i32 = db
            .global_get(&section, &format!("feast_{}_dur", current_id))
            .parse()
            .unwrap_or(0);
        let elapsed_min = ((now_ts() - feast_ts) / 60).max(0) as i32;
        let remaining = (feast_dur - elapsed_min).max(0);

        if remaining <= 0 {
            return format!("{}\n❌ 盛宴已结束！", prefix);
        }

        let feast_name = db.global_get(&section, &format!("feast_{}_name", current_id));
        let feast_emoji = db.global_get(&section, &format!("feast_{}_emoji", current_id));
        let feast_host = db.global_get(&section, &format!("feast_{}_host", current_id));
        let hp_pct: i32 = db
            .global_get(&section, &format!("feast_{}_hp_pct", current_id))
            .parse()
            .unwrap_or(0);
        let ad_pct: i32 = db
            .global_get(&section, &format!("feast_{}_ad_pct", current_id))
            .parse()
            .unwrap_or(0);
        let ap_pct: i32 = db
            .global_get(&section, &format!("feast_{}_ap_pct", current_id))
            .parse()
            .unwrap_or(0);
        let exp_pct: i32 = db
            .global_get(&section, &format!("feast_{}_exp_pct", current_id))
            .parse()
            .unwrap_or(0);
        let gold_pct: i32 = db
            .global_get(&section, &format!("feast_{}_gold_pct", current_id))
            .parse()
            .unwrap_or(0);
        let participants_raw = db.global_get(&section, &format!("feast_{}_parts", current_id));
        let part_count = participants_raw.split(',').filter(|s| !s.is_empty()).count();
        let max_parts: usize = db
            .global_get(&section, &format!("feast_{}_max", current_id))
            .parse()
            .unwrap_or(20);

        let mut r = format!("{}\n", prefix);
        r.push_str(&format!("═══ {} {} 增益详情 ═══\n", feast_emoji, feast_name));
        r.push_str(&format!("📢 发起人: {}\n", feast_host));
        r.push_str(&format!("⏱️ 剩余: {}分钟\n", remaining));
        r.push_str(&format!("👥 {}/{}人\n\n", part_count, max_parts));
        r.push_str("✨ 当前增益效果:\n");
        if hp_pct > 0 {
            r.push_str(&format!("  ❤️ 生命上限 +{}%\n", hp_pct));
        }
        if ad_pct > 0 {
            r.push_str(&format!("  ⚔️ 物理攻击 +{}%\n", ad_pct));
        }
        if ap_pct > 0 {
            r.push_str(&format!("  🔮 魔法攻击 +{}%\n", ap_pct));
        }
        if exp_pct > 0 {
            r.push_str(&format!("  📈 经验获取 +{}%\n", exp_pct));
        }
        if gold_pct > 0 {
            r.push_str(&format!("  💰 金币获取 +{}%\n", gold_pct));
        }

        r.push_str(&format!(
            "\n📊 参与进度: {} {}/{}",
            progress_bar(part_count as i64, max_parts as i64, 10),
            part_count,
            max_parts
        ));

        return r;
    }

    // 指定宴会名称：显示等级详情
    let feast_idx = if let Ok(n) = args_trimmed.parse::<usize>() {
        if n == 0 || n > FEASTS.len() {
            return format!("{}\n❌ 无效的宴会等级！请输入1-{}的数字。", prefix, FEASTS.len());
        }
        n - 1
    } else {
        match FEASTS
            .iter()
            .position(|f| f.name.contains(args_trimmed) || args_trimmed.contains(f.name))
        {
            Some(idx) => idx,
            None => return format!("{}\n❌ 未找到名为「{}」的宴会等级！", prefix, args_trimmed),
        }
    };

    let def = &FEASTS[feast_idx];
    let mut r = format!("{}\n", prefix);
    r.push_str(&format!("═══ {} {} 详情 ═══\n\n", def.emoji, def.name));
    r.push_str(&format!("📝 {}\n\n", def.bonus_desc));
    r.push_str("✨ 增益效果:\n");
    r.push_str(&format!("  ❤️ 生命上限 +{}%\n", def.bonus_hp_pct));
    r.push_str(&format!("  ⚔️ 物理攻击 +{}%\n", def.bonus_ad_pct));
    r.push_str(&format!("  🔮 魔法攻击 +{}%\n", def.bonus_ap_pct));
    r.push_str(&format!("  📈 经验获取 +{}%\n", def.bonus_exp_pct));
    r.push_str(&format!("  💰 金币获取 +{}%\n\n", def.bonus_gold_pct));
    r.push_str("🎁 参与奖励:\n");
    r.push_str(&format!("  💰 {}金币\n", format_num(def.reward_gold)));
    r.push_str(&format!("  💎 {}钻石\n", def.reward_diamond));
    r.push_str(&format!("  ⭐ {}经验\n\n", def.reward_exp));
    r.push_str(&format!("⏰ 持续时间: {}分钟\n", def.duration_min));
    r.push_str(&format!("👥 最多参与: {}人\n", def.max_participants));
    r.push_str(&format!("\n💰 发起费用: {}金币", format_num(def.cost_gold)));
    if def.cost_diamond > 0 {
        r.push_str(&format!(" + {}💎", def.cost_diamond));
    }
    r.push_str(&format!("\n\n💡 输入「发起盛宴+{}」举办此宴会", feast_idx + 1));

    r
}

/// 获取公会盛宴经验加成（供战斗系统集成）
#[allow(dead_code)]
pub fn get_feast_exp_bonus(db: &Database, user_id: &str) -> i32 {
    let guild = db.global_get("user_guild", user_id);
    if guild.is_empty() {
        return 0;
    }

    let section = format!("{}_{}", SECTION, guild);
    let current_id = db.global_get(&section, "current_feast");
    if current_id.is_empty() {
        return 0;
    }

    let feast_ts: i64 = db
        .global_get(&section, &format!("feast_{}_ts", current_id))
        .parse()
        .unwrap_or(0);
    let feast_dur: i32 = db
        .global_get(&section, &format!("feast_{}_dur", current_id))
        .parse()
        .unwrap_or(0);
    let elapsed_min = ((now_ts() - feast_ts) / 60).max(0) as i32;
    if feast_dur - elapsed_min <= 0 {
        return 0;
    }

    // 检查用户是否参与
    let participants_raw = db.global_get(&section, &format!("feast_{}_parts", current_id));
    if !participants_raw.split(',').any(|s| s == user_id) {
        return 0;
    }

    db.global_get(&section, &format!("feast_{}_exp_pct", current_id))
        .parse()
        .unwrap_or(0)
}

/// 获取公会盛宴金币加成（供经济系统集成）
#[allow(dead_code)]
pub fn get_feast_gold_bonus(db: &Database, user_id: &str) -> i32 {
    let guild = db.global_get("user_guild", user_id);
    if guild.is_empty() {
        return 0;
    }

    let section = format!("{}_{}", SECTION, guild);
    let current_id = db.global_get(&section, "current_feast");
    if current_id.is_empty() {
        return 0;
    }

    let feast_ts: i64 = db
        .global_get(&section, &format!("feast_{}_ts", current_id))
        .parse()
        .unwrap_or(0);
    let feast_dur: i32 = db
        .global_get(&section, &format!("feast_{}_dur", current_id))
        .parse()
        .unwrap_or(0);
    let elapsed_min = ((now_ts() - feast_ts) / 60).max(0) as i32;
    if feast_dur - elapsed_min <= 0 {
        return 0;
    }

    let participants_raw = db.global_get(&section, &format!("feast_{}_parts", current_id));
    if !participants_raw.split(',').any(|s| s == user_id) {
        return 0;
    }

    db.global_get(&section, &format!("feast_{}_gold_pct", current_id))
        .parse()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feast_defs_count() {
        assert_eq!(FEASTS.len(), 4);
    }

    #[test]
    fn test_feast_names_unique() {
        let mut names: Vec<&str> = FEASTS.iter().map(|f| f.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), FEASTS.len());
    }

    #[test]
    fn test_feast_costs_positive() {
        for def in FEASTS {
            assert!(def.cost_gold > 0, "feast '{}' must cost gold", def.name);
        }
    }

    #[test]
    fn test_feast_costs_escalate() {
        for i in 1..FEASTS.len() {
            assert!(
                FEASTS[i].cost_gold >= FEASTS[i - 1].cost_gold,
                "feast '{}' should cost more than '{}'",
                FEASTS[i].name,
                FEASTS[i - 1].name
            );
        }
    }

    #[test]
    fn test_feast_rewards_positive() {
        for def in FEASTS {
            assert!(def.reward_gold > 0, "feast '{}' must reward gold", def.name);
            assert!(
                def.reward_diamond >= 0,
                "feast '{}' diamond reward must be non-negative",
                def.name
            );
            assert!(def.reward_exp > 0, "feast '{}' must reward exp", def.name);
        }
    }

    #[test]
    fn test_feast_rewards_escalate() {
        for i in 1..FEASTS.len() {
            assert!(
                FEASTS[i].reward_gold >= FEASTS[i - 1].reward_gold,
                "feast '{}' rewards should escalate",
                FEASTS[i].name
            );
        }
    }

    #[test]
    fn test_feast_duration_positive() {
        for def in FEASTS {
            assert!(def.duration_min > 0, "feast '{}' must have positive duration", def.name);
        }
    }

    #[test]
    fn test_feast_duration_escalate() {
        for i in 1..FEASTS.len() {
            assert!(
                FEASTS[i].duration_min >= FEASTS[i - 1].duration_min,
                "feast '{}' duration should escalate",
                FEASTS[i].name
            );
        }
    }

    #[test]
    fn test_feast_bonus_pct_range() {
        for def in FEASTS {
            assert!(def.bonus_hp_pct > 0 && def.bonus_hp_pct <= 100);
            assert!(def.bonus_ad_pct > 0 && def.bonus_ad_pct <= 100);
            assert!(def.bonus_ap_pct > 0 && def.bonus_ap_pct <= 100);
            assert!(def.bonus_exp_pct > 0 && def.bonus_exp_pct <= 200);
            assert!(def.bonus_gold_pct > 0 && def.bonus_gold_pct <= 100);
        }
    }

    #[test]
    fn test_feast_emojis_non_empty() {
        for def in FEASTS {
            assert!(!def.emoji.is_empty(), "feast '{}' must have emoji", def.name);
        }
    }

    #[test]
    fn test_feast_max_participants() {
        for def in FEASTS {
            assert!(
                def.max_participants >= 10,
                "feast '{}' must allow at least 10 participants",
                def.name
            );
        }
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(123), "123");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1000000), "1,000,000");
        assert_eq!(format_num(999999), "999,999");
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(5, 10, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.contains('█'));
        assert!(bar.contains('░'));

        let full = progress_bar(10, 10, 10);
        assert_eq!(full.chars().count(), 10);
        assert!(full.chars().all(|c| c == '█'));

        let empty = progress_bar(0, 10, 10);
        assert_eq!(empty.chars().count(), 10);
        assert!(empty.chars().all(|c| c == '░'));
    }

    #[test]
    fn test_gen_feast_id_deterministic() {
        let id1 = gen_feast_id("测试公会", 1000000);
        let id2 = gen_feast_id("测试公会", 1000000);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_gen_feast_id_different_guilds() {
        let id1 = gen_feast_id("公会A", 1000000);
        let id2 = gen_feast_id("公会B", 1000000);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_gen_feast_id_format() {
        let id = gen_feast_id("test", 12345);
        assert!(id.starts_with('F'), "feast ID should start with F");
        assert!(id.len() >= 5, "feast ID should be at least 5 chars");
    }

    #[test]
    fn test_today_str_format() {
        let today = today_str();
        assert_eq!(today.len(), 10);
        assert!(today.contains('-'));
    }

    #[test]
    fn test_feast_bonus_escalate() {
        for i in 1..FEASTS.len() {
            assert!(
                FEASTS[i].bonus_exp_pct >= FEASTS[i - 1].bonus_exp_pct,
                "feast '{}' exp bonus should escalate",
                FEASTS[i].name
            );
        }
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "guild_feast");
    }

    #[test]
    fn test_feast_diamond_costs() {
        // First feast should be free diamonds
        assert_eq!(FEASTS[0].cost_diamond, 0);
        // Higher tiers should cost diamonds
        assert!(FEASTS[FEASTS.len() - 1].cost_diamond > 0);
    }

    #[test]
    fn test_max_participants_str() {
        assert_eq!(max_participants_str(20), "20人");
        assert_eq!(max_participants_str(50), "50人");
    }
}

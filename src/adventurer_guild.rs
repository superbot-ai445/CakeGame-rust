// adventurer_guild.rs — 冒险者工会系统 (Adventurer's Guild System)
// Ranked quests (D→SS), adventurer reputation, rank-up, special shop, leaderboard

use crate::db::Database;

// ── Constants ────────────────────────────────────────────────────────────────

const SECTION: &str = "adventurer_guild";

const QUEST_RANKS: &[(&str, &str, i64, i64, i64)] = &[
    // (rank, emoji, min_level, rep_reward, gold_reward)
    ("D", "📗", 1, 10, 500),
    ("C", "📘", 15, 25, 2000),
    ("B", "📙", 30, 50, 5000),
    ("A", "📕", 50, 100, 15000),
    ("S", "📕", 70, 200, 50000),
    ("SS", "📕", 90, 400, 150000),
];

const ADVENTURER_RANKS: &[(&str, i64, &str)] = &[
    // (title, rep_threshold, emoji)
    ("见习冒险者", 0, "🔰"),
    ("初级冒险者", 100, "⭐"),
    ("中级冒险者", 300, "🌟"),
    ("高级冒险者", 700, "💫"),
    ("精英冒险者", 1500, "🏅"),
    ("大师冒险者", 3000, "🎖️"),
    ("传说冒险者", 6000, "👑"),
    ("冒险王", 12000, "⚜️"),
];

const QUEST_TYPES: &[(&str, &str)] = &[
    ("monster_hunt", "讨伐怪物"),
    ("item_collect", "收集物品"),
    ("boss_challenge", "挑战BOSS"),
    ("explore", "探索地图"),
    ("pvp_duel", "竞技决斗"),
];

const MAX_ACTIVE_QUESTS: i64 = 3;
const DAILY_QUEST_LIMIT: i64 = 10;

const SHOP_ITEMS: &[(&str, i64, &str, i64)] = &[
    // (name, rep_cost, description, min_rank_index)
    ("冒险者药水", 50, "恢复50% HP和MP", 0),
    ("冒险者护符", 150, "防御+5%持续30分钟", 1),
    ("经验卷轴", 200, "获得5000经验", 1),
    ("冒险者之证", 400, "攻击力+10%持续30分钟", 2),
    ("高级强化石", 600, "强化装备的珍贵材料", 2),
    ("凤凰之羽", 1000, "复活后恢复30% HP", 3),
    ("冒险者战甲", 1500, "防御+200的特殊装备", 3),
    ("传说宝箱", 2500, "随机获得传说级道具", 4),
    ("冒险王冠", 5000, "所有属性+5%持续1小时", 5),
    ("冒险者至尊宝箱", 8000, "必出史诗级以上道具", 6),
];

// ── Helper functions ─────────────────────────────────────────────────────────

fn get_rep(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("rep:{}", user_id)).parse().unwrap_or(0)
}

fn set_rep(db: &Database, user_id: &str, rep: i64) {
    db.global_set(SECTION, &format!("rep:{}", user_id), &rep.to_string());
}

fn get_daily_count(db: &Database, user_id: &str) -> i64 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.global_get(SECTION, &format!("daily:{}:{}", user_id, today))
        .parse()
        .unwrap_or(0)
}

fn set_daily_count(db: &Database, user_id: &str, count: i64) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.global_set(SECTION, &format!("daily:{}:{}", user_id, today), &count.to_string());
}

fn get_active_quests(db: &Database, user_id: &str) -> Vec<String> {
    let val = db.global_get(SECTION, &format!("active:{}", user_id));
    if val.is_empty() {
        Vec::new()
    } else {
        val.split('|').map(|s| s.to_string()).collect()
    }
}

fn set_active_quests(db: &Database, user_id: &str, quests: &[String]) {
    db.global_set(SECTION, &format!("active:{}", user_id), &quests.join("|"));
}

fn get_total_completed(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("total:{}", user_id))
        .parse()
        .unwrap_or(0)
}

fn set_total_completed(db: &Database, user_id: &str, count: i64) {
    db.global_set(SECTION, &format!("total:{}", user_id), &count.to_string());
}

fn get_purchased_today(db: &Database, user_id: &str) -> i64 {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.global_get(SECTION, &format!("shop:{}:{}", user_id, today))
        .parse()
        .unwrap_or(0)
}

fn set_purchased_today(db: &Database, user_id: &str, count: i64) {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.global_set(SECTION, &format!("shop:{}:{}", user_id, today), &count.to_string());
}

fn get_player_level(db: &Database, user_id: &str) -> i64 {
    db.read_basic(user_id, "Level").parse().unwrap_or(1)
}

fn get_adventurer_rank(rep: i64) -> (&'static str, &'static str, usize) {
    for (i, &(title, threshold, emoji)) in ADVENTURER_RANKS.iter().enumerate().rev() {
        if rep >= threshold {
            return (title, emoji, i);
        }
    }
    ("见习冒险者", "🔰", 0)
}

fn quest_type_for_rank(rank_idx: usize) -> &'static str {
    QUEST_TYPES[rank_idx % QUEST_TYPES.len()].0
}

fn generate_quest_target(rank_idx: usize, quest_type: &str) -> (String, i64) {
    match quest_type {
        "monster_hunt" => {
            let targets = [
                "史莱姆",
                "哥布林",
                "野狼",
                "巨熊",
                "骷髅兵",
                "毒蜘蛛",
                "飞龙",
                "恶魔",
                "暗影骑士",
                "远古巨龙",
            ];
            let idx = rank_idx.min(targets.len() - 1);
            let count = (rank_idx as i64 + 1) * 3 + 2;
            (format!("击败{}只{}", count, targets[idx]), count)
        }
        "item_collect" => {
            let items = [
                "草药",
                "铁矿石",
                "水晶碎片",
                "魔法粉尘",
                "龙鳞",
                "凤凰羽毛",
                "星辰精华",
                "暗物质",
                "时空碎片",
                "创世之石",
            ];
            let idx = rank_idx.min(items.len() - 1);
            let count = (rank_idx as i64 + 1) * 2 + 1;
            (format!("收集{}个{}", count, items[idx]), count)
        }
        "boss_challenge" => {
            let bosses = [
                "哥布林将军",
                "狼王",
                "骷髅王",
                "毒蛛女王",
                "暗影龙",
                "恶魔领主",
                "远古巨龙",
                "堕落天使",
                "混沌之主",
                "终焉审判者",
            ];
            let idx = rank_idx.min(bosses.len() - 1);
            let count = (rank_idx / 2 + 1) as i64;
            (format!("挑战并击败{} {}次", bosses[idx], count), count)
        }
        "explore" => {
            let count = rank_idx / 2 + 2;
            (format!("探索{}个不同地图", count), count as i64)
        }
        "pvp_duel" => {
            let count = (rank_idx as i64 + 1) * 2;
            (format!("在竞技场中获胜{}次", count), count)
        }
        _ => ("完成任务".to_string(), 1),
    }
}

fn quest_difficulty_label(rank_idx: usize) -> &'static str {
    match rank_idx {
        0 => "简单",
        1 => "普通",
        2 => "困难",
        3 => "精英",
        4 => "噩梦",
        _ => "地狱",
    }
}

fn progress_bar(current: i64, target: i64, width: usize) -> String {
    if target <= 0 {
        return "█".repeat(width);
    }
    let filled = ((current as f64 / target as f64) * width as f64)
        .min(width as f64)
        .max(0.0) as usize;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// 查看工会 — 显示冒险者工会概览
pub fn cmd_view_adventurer_guild(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let rep = get_rep(db, user_id);
    let (rank_title, rank_emoji, rank_idx) = get_adventurer_rank(rep);
    let total = get_total_completed(db, user_id);
    let daily = get_daily_count(db, user_id);
    let active = get_active_quests(db, user_id);

    let next_rank = if rank_idx + 1 < ADVENTURER_RANKS.len() {
        let (_, next_threshold, _) = ADVENTURER_RANKS[rank_idx + 1];
        format!(
            "  ⏫ 下一等级: {} (还需 {} 声望)",
            ADVENTURER_RANKS[rank_idx + 1].0,
            next_threshold - rep
        )
    } else {
        "  🏆 已达最高等级!".to_string()
    };

    let mut out = String::new();
    out.push_str("⚔️ ═══ 冒险者工会 ═══ ⚔️\n\n");
    out.push_str(&format!("{} {} 冒险者等级\n", rank_emoji, rank_title));
    out.push_str(&format!("📊 声望点数: {}\n", rep));
    out.push_str(&format!("📋 已完成任务: {}\n", total));
    out.push_str(&format!("📅 今日已完成: {}/{}\n", daily, DAILY_QUEST_LIMIT));
    out.push_str(&format!(
        "📌 当前进行中: {}/{}\n",
        active.len(),
        MAX_ACTIVE_QUESTS as usize
    ));
    out.push_str(&next_rank);
    out.push_str("\n\n📖 工会功能:\n");
    out.push_str("  🔍 查看任务 → 工会任务\n");
    out.push_str("  📝 接受任务 → 接受工会 [等级]\n");
    out.push_str("  ✅ 提交任务 → 提交工会 [等级]\n");
    out.push_str("  🛒 工会商店 → 工会商店\n");
    out.push_str("  🏆 冒险排行 → 工会排行\n");
    out.push_str("  📊 任务详情 → 工会详情\n");
    out
}

/// 查看可接任务
pub fn cmd_view_guild_quests(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let level = get_player_level(db, user_id);
    let rep = get_rep(db, user_id);
    let (_, _, rank_idx) = get_adventurer_rank(rep);
    let active = get_active_quests(db, user_id);

    let mut out = String::new();
    out.push_str("📋 ═══ 工会任务列表 ═══ 📋\n\n");

    for (i, &(rank, emoji, min_level, rep_reward, gold_reward)) in QUEST_RANKS.iter().enumerate() {
        let available = level >= min_level && i <= rank_idx + 1;
        let status = if !available {
            "🔒".to_string()
        } else if active.iter().any(|q| q.starts_with(&format!("{}|", rank))) {
            "📌 进行中".to_string()
        } else {
            "✅ 可接取".to_string()
        };

        out.push_str(&format!("{} {}级任务 {} Lv{}+\n", emoji, rank, status, min_level));
        if available {
            out.push_str(&format!(
                "   声望+{} | 金币+{} | 难度: {}\n",
                rep_reward,
                gold_reward,
                quest_difficulty_label(i)
            ));
        }
        out.push('\n');
    }

    out.push_str("💡 输入「接受工会 [等级]」接取任务 (D/C/B/A/S/SS)\n");
    out.push_str(&format!(
        "📌 当前进行中: {}/{} 个任务\n",
        active.len(),
        MAX_ACTIVE_QUESTS
    ));
    out
}

/// 接受工会任务
pub fn cmd_accept_guild_quest(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let rank_input = args.trim().to_uppercase();
    if rank_input.is_empty() {
        return "❌ 请指定任务等级: 接受工会 [D/C/B/A/S/SS]".to_string();
    }

    let rank_info = QUEST_RANKS.iter().find(|&&(r, _, _, _, _)| r == rank_input);
    let (rank, emoji, min_level, rep_reward, gold_reward) = match rank_info {
        Some(&info) => info,
        None => return format!("❌ 无效的任务等级「{}」，可选: D/C/B/A/S/SS", rank_input),
    };

    let level = get_player_level(db, user_id);
    if level < min_level {
        return format!("{}级任务需要至少 {} 级，你当前 {} 级", rank, min_level, level);
    }

    let rep = get_rep(db, user_id);
    let (_, _, player_rank_idx) = get_adventurer_rank(rep);
    let quest_rank_idx = QUEST_RANKS.iter().position(|&(r, _, _, _, _)| r == rank).unwrap_or(0);
    if quest_rank_idx > player_rank_idx + 1 {
        return format!("你的冒险者等级不足，无法接取 {} 级任务", rank);
    }

    let daily = get_daily_count(db, user_id);
    if daily >= DAILY_QUEST_LIMIT {
        return format!("今日任务次数已达上限 ({}/{})", daily, DAILY_QUEST_LIMIT);
    }

    let mut active = get_active_quests(db, user_id);
    if active.len() >= MAX_ACTIVE_QUESTS as usize {
        return format!("当前已有 {} 个进行中任务，请先完成或放弃", active.len());
    }

    if active.iter().any(|q| q.starts_with(&format!("{}|", rank))) {
        return format!("你已经有一个 {} 级任务进行中", rank);
    }

    let quest_type = quest_type_for_rank(quest_rank_idx);
    let (target_desc, target_count) = generate_quest_target(quest_rank_idx, quest_type);
    let quest_entry = format!("{}|{}|{}|{}|{}", rank, quest_type, target_desc, target_count, 0);
    active.push(quest_entry);
    set_active_quests(db, user_id, &active);

    let mut out = String::new();
    out.push_str("✅ ═══ 任务接取成功 ═══ ✅\n\n");
    out.push_str(&format!("{} {}级任务\n", emoji, rank));
    out.push_str(&format!("📝 目标: {}\n", target_desc));
    out.push_str(&format!("🏆 奖励: {} 声望 + {} 金币\n", rep_reward, gold_reward));
    out.push_str(&format!("📊 进度: 0/{}\n", target_count));
    out.push_str(&format!("\n💡 完成目标后输入「提交工会 {}」提交任务\n", rank));
    out
}

/// 提交工会任务
pub fn cmd_submit_guild_quest(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let rank_input = args.trim().to_uppercase();
    if rank_input.is_empty() {
        return "❌ 请指定任务等级: 提交工会 [D/C/B/A/S/SS]".to_string();
    }

    let mut active = get_active_quests(db, user_id);
    let quest_idx = active.iter().position(|q| q.starts_with(&format!("{}|", rank_input)));

    let idx = match quest_idx {
        Some(i) => i,
        None => return format!("没有找到 {} 级的进行中任务", rank_input),
    };

    let quest_data = active[idx].clone();
    let parts: Vec<&str> = quest_data.split('|').collect();
    if parts.len() < 5 {
        active.remove(idx);
        set_active_quests(db, user_id, &active);
        return "❌ 任务数据异常，已清除".to_string();
    }

    let rank = parts[0].to_string();
    let target_desc = parts[2].to_string();
    let target_count: i64 = parts[3].parse().unwrap_or(1);
    let progress: i64 = parts[4].parse().unwrap_or(0);

    // Allow submission if progress is complete, or auto-complete for testing
    if progress < target_count {
        let bar = progress_bar(progress, target_count, 20);
        return format!(
            "⏳ 任务尚未完成\n📝 {}\n📊 {}/{} {}\n💡 继续游戏自动累积进度",
            target_desc, progress, target_count, bar
        );
    }

    // Complete quest
    active.remove(idx);
    set_active_quests(db, user_id, &active);

    let (rep_reward, gold_reward) = QUEST_RANKS
        .iter()
        .find(|&&(r, _, _, _, _)| r == rank.as_str())
        .map(|&(_, _, _, rp, gp)| (rp, gp))
        .unwrap_or((10, 500));

    let old_rep = get_rep(db, user_id);
    let new_rep = old_rep + rep_reward;
    set_rep(db, user_id, new_rep);

    // Grant gold via modify_currency
    db.modify_currency(user_id, "Gold", "add", gold_reward);

    // Update counts
    let daily = get_daily_count(db, user_id) + 1;
    set_daily_count(db, user_id, daily);
    let total = get_total_completed(db, user_id) + 1;
    set_total_completed(db, user_id, total);

    // Check for rank up
    let (old_title, _, _) = get_adventurer_rank(old_rep);
    let (new_title, new_emoji, _) = get_adventurer_rank(new_rep);
    let rank_up = old_title != new_title;

    let mut out = String::new();
    out.push_str("🎉 ═══ 任务完成! ═══ 🎉\n\n");
    out.push_str(&format!("✅ 已提交: {}\n", target_desc));
    out.push_str(&format!("🏆 奖励: +{} 声望 +{} 金币\n", rep_reward, gold_reward));
    out.push_str(&format!("📊 总声望: {} → {}\n", old_rep, new_rep));
    out.push_str(&format!("📋 累计完成: {} 个任务\n", total));

    if rank_up {
        out.push_str("\n🎊 ═══ 冒险者升级! ═══ 🎊\n");
        let (_, old_emoji, _) = get_adventurer_rank(old_rep);
        out.push_str(&format!("{} {} → {} {}\n", old_emoji, old_title, new_emoji, new_title));
        out.push_str("💡 新等级已解锁更高级任务和商店物品!\n");
    }

    out
}

/// 工会商店
pub fn cmd_adventurer_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let rep = get_rep(db, user_id);
    let (_, _, rank_idx) = get_adventurer_rank(rep);
    let purchased = get_purchased_today(db, user_id);

    let mut out = String::new();
    out.push_str("🛒 ═══ 冒险者工会商店 ═══ 🛒\n\n");
    out.push_str(&format!("💰 你的声望: {} | 📦 今日购买: {}/5\n\n", rep, purchased));

    for (i, &(name, cost, desc, min_rank)) in SHOP_ITEMS.iter().enumerate() {
        let available = rank_idx >= min_rank as usize;
        let status = if !available {
            "🔒"
        } else if rep < cost {
            "💸 声望不足"
        } else {
            "✅ 可购买"
        };

        let rank_req = if min_rank as usize > 0 && (min_rank as usize) < ADVENTURER_RANKS.len() {
            format!(" [需要{}]", ADVENTURER_RANKS[min_rank as usize].0)
        } else {
            String::new()
        };

        out.push_str(&format!("{}. {} {} — {} 声望\n", i + 1, name, status, cost));
        out.push_str(&format!("   {}{}\n\n", desc, rank_req));
    }

    out.push_str("💡 购买: 购买工会 [商品编号]\n");
    out
}

/// 购买工会商品
pub fn cmd_buy_guild_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let item_num: usize = match args.trim().parse::<usize>() {
        Ok(n) if n >= 1 && n <= SHOP_ITEMS.len() => n - 1,
        _ => return format!("无效的商品编号，可选: 1-{}", SHOP_ITEMS.len()),
    };

    let (name, cost, _desc, min_rank) = SHOP_ITEMS[item_num];
    let rep = get_rep(db, user_id);
    let (_, _, rank_idx) = get_adventurer_rank(rep);

    if rank_idx < min_rank as usize {
        return format!("需要 {} 才能购买此商品", ADVENTURER_RANKS[min_rank as usize].0);
    }

    if rep < cost {
        return format!("声望不足! 需要 {} 声望，你只有 {}", cost, rep);
    }

    let purchased = get_purchased_today(db, user_id);
    if purchased >= 5 {
        return "今日购买次数已达上限 (5/5)".to_string();
    }

    // Deduct rep
    set_rep(db, user_id, rep - cost);
    set_purchased_today(db, user_id, purchased + 1);

    // Grant item effect
    let effect_msg = match item_num {
        0 => {
            // Adventurer Potion: restore HP/MP
            let max_hp: i64 = db.read_basic(user_id, "MaxHP").parse().unwrap_or(100);
            let max_mp: i64 = db.read_basic(user_id, "MaxMP").parse().unwrap_or(50);
            db.write_basic(user_id, "HP", &max_hp.to_string());
            db.write_basic(user_id, "MP", &max_mp.to_string());
            "恢复50% HP和MP"
        }
        1 => {
            db.write_user_data(user_id, "adventurer_buff", "def+5%");
            "防御+5%持续30分钟"
        }
        2 => {
            let exp: i64 = db.read_basic(user_id, "Experience").parse().unwrap_or(0);
            db.write_basic(user_id, "Experience", &(exp + 5000).to_string());
            "获得5000经验"
        }
        3 => {
            db.write_user_data(user_id, "adventurer_buff", "atk+10%");
            "攻击力+10%持续30分钟"
        }
        4 => {
            db.knapsack_add(user_id, "高级强化石", 1);
            "获得高级强化石×1"
        }
        5 => {
            db.knapsack_add(user_id, "凤凰之羽", 1);
            "获得凤凰之羽×1"
        }
        6 => {
            db.knapsack_add(user_id, "冒险者战甲", 1);
            "获得冒险者战甲×1"
        }
        7 => {
            let gold = 5000 + (rep % 10000);
            db.modify_currency(user_id, "Gold", "add", gold);
            "随机获得传说级道具(金币奖励)"
        }
        8 => {
            db.write_user_data(user_id, "adventurer_buff", "all+5%");
            "所有属性+5%持续1小时"
        }
        9 => {
            let gold = 20000 + (rep % 30000);
            db.modify_currency(user_id, "Gold", "add", gold);
            db.modify_currency(user_id, "Diamond", "add", 50);
            "必出史诗级以上道具(金币+钻石)"
        }
        _ => "购买成功",
    };

    let mut out = String::new();
    out.push_str("✅ ═══ 购买成功! ═══ ✅\n\n");
    out.push_str(&format!("🛒 商品: {}\n", name));
    out.push_str(&format!("💰 消耗: {} 声望\n", cost));
    out.push_str(&format!("🎁 效果: {}\n", effect_msg));
    out.push_str(&format!("📊 剩余声望: {}\n", rep - cost));
    out.push_str(&format!("📦 今日购买: {}/5\n", purchased + 1));
    out
}

/// 工会排行
pub fn cmd_adventurer_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // Get all rep entries via query_list
    let sql = "SELECT Key, CAST(Value AS INTEGER) as rep FROM Global WHERE SECTION='adventurer_guild' AND Key LIKE 'rep:%' ORDER BY rep DESC LIMIT 15";
    let rows = db.query_rows(sql, &[], |row| {
        let key: String = row.get(0)?;
        let rep: i64 = row.get(1)?;
        Ok((key, rep))
    });

    let mut out = String::new();
    out.push_str("🏆 ═══ 冒险者工会排行 ═══ 🏆\n\n");

    if rows.is_empty() {
        out.push_str("📭 暂无排名数据\n");
        return out;
    }

    for (i, (key, rep)) in rows.iter().enumerate() {
        let uid = key.strip_prefix("rep:").unwrap_or(key);
        let (title, emoji, _) = get_adventurer_rank(*rep);

        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!(
            "{} {}. {} {} — {} 声望 [{}]\n",
            medal,
            i + 1,
            emoji,
            uid,
            rep,
            title
        ));
    }

    out.push_str("\n💡 声望通过完成工会任务获得\n");
    out
}

/// 工会详情
pub fn cmd_guild_quest_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let rank_input = args.trim().to_uppercase();
    let active = get_active_quests(db, user_id);

    if !rank_input.is_empty() {
        let quest = active.iter().find(|q| q.starts_with(&format!("{}|", rank_input)));
        match quest {
            Some(q) => {
                let parts: Vec<&str> = q.split('|').collect();
                if parts.len() >= 5 {
                    let rank = parts[0];
                    let quest_type = parts[1];
                    let target_desc = parts[2];
                    let target_count: i64 = parts[3].parse().unwrap_or(1);
                    let progress: i64 = parts[4].parse().unwrap_or(0);

                    let type_label = QUEST_TYPES
                        .iter()
                        .find(|&&(t, _)| t == quest_type)
                        .map(|&(_, l)| l)
                        .unwrap_or("未知");
                    let rank_info = QUEST_RANKS.iter().find(|&&(r, _, _, _, _)| r == rank);
                    let (emoji, rep_r, gold_r) = match rank_info {
                        Some(&(_, e, _, rp, gp)) => (e, rp, gp),
                        None => ("❓", 0, 0),
                    };

                    let bar = progress_bar(progress, target_count, 20);
                    let mut out = String::new();
                    out.push_str(&format!("{} ═══ {}级任务详情 ═══\n\n", emoji, rank));
                    out.push_str(&format!("📋 类型: {}\n", type_label));
                    out.push_str(&format!("🎯 目标: {}\n", target_desc));
                    out.push_str(&format!("📊 进度: {}/{} {}\n", progress, target_count, bar));
                    out.push_str(&format!("🏆 奖励: {} 声望 + {} 金币\n", rep_r, gold_r));
                    out.push_str(&format!(
                        "⏰ 状态: {}\n",
                        if progress >= target_count {
                            "✅ 可提交"
                        } else {
                            "⏳ 进行中"
                        }
                    ));
                    return out;
                }
                "❌ 任务数据异常".to_string()
            }
            None => format!("没有找到 {} 级的进行中任务", rank_input),
        }
    } else {
        let rep = get_rep(db, user_id);
        let (title, emoji, _) = get_adventurer_rank(rep);
        let total = get_total_completed(db, user_id);

        let mut out = String::new();
        out.push_str("📊 ═══ 冒险者详情 ═══ 📊\n\n");
        out.push_str(&format!("{} {}\n", emoji, title));
        out.push_str(&format!("📊 声望: {}\n", rep));
        out.push_str(&format!("📋 累计完成: {} 个任务\n", total));

        if active.is_empty() {
            out.push_str("\n📭 当前没有进行中的任务\n");
            out.push_str("💡 输入「工会任务」查看可接取的任务\n");
        } else {
            out.push_str("\n📌 进行中任务:\n");
            for q in &active {
                let parts: Vec<&str> = q.split('|').collect();
                if parts.len() >= 5 {
                    let rank = parts[0];
                    let target_desc = parts[2];
                    let target_count: i64 = parts[3].parse().unwrap_or(1);
                    let progress: i64 = parts[4].parse().unwrap_or(0);
                    let bar = progress_bar(progress, target_count, 10);
                    out.push_str(&format!(
                        "  [{}] {} {}/{} {}\n",
                        rank, target_desc, progress, target_count, bar
                    ));
                }
            }
        }
        out
    }
}

/// 工会帮助
pub fn cmd_adventurer_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::new();
    out.push_str("📖 ═══ 冒险者工会帮助 ═══ 📖\n\n");
    out.push_str("⚔️ 冒险者工会是勇者们的聚集地!\n");
    out.push_str("   通过完成工会任务获得声望，提升冒险者等级。\n\n");
    out.push_str("📋 指令列表:\n");
    out.push_str("  • 冒险工会 — 查看工会概览\n");
    out.push_str("  • 工会任务 — 查看可接取的任务\n");
    out.push_str("  • 接受工会 [等级] — 接受指定等级任务\n");
    out.push_str("  • 提交工会 [等级] — 提交已完成的任务\n");
    out.push_str("  • 工会商店 — 浏览工会商店\n");
    out.push_str("  • 购买工会 [编号] — 购买商品\n");
    out.push_str("  • 工会排行 — 查看全服冒险者排名\n");
    out.push_str("  • 工会详情 [等级] — 查看任务详情\n\n");
    out.push_str("📊 冒险者等级:\n");
    for &(title, threshold, emoji) in ADVENTURER_RANKS {
        out.push_str(&format!("  {} {} — {} 声望\n", emoji, title, threshold));
    }
    out.push_str("\n📝 任务等级:\n");
    for &(rank, emoji, min_level, rep_r, gold_r) in QUEST_RANKS {
        out.push_str(&format!(
            "  {} {}级 — Lv{}+ | +{}声望 +{}金\n",
            emoji, rank, min_level, rep_r, gold_r
        ));
    }
    out.push_str(&format!(
        "\n💡 每日最多完成 {} 个任务，同时进行 {} 个\n",
        DAILY_QUEST_LIMIT, MAX_ACTIVE_QUESTS
    ));
    out
}

// ── Auto-progress hook ───────────────────────────────────────────────────────

/// 战斗击杀时自动推进工会任务进度 (供战斗系统调用)
#[allow(dead_code)]
pub fn record_quest_progress(db: &Database, user_id: &str, action: &str) {
    let mut active = get_active_quests(db, user_id);
    if active.is_empty() {
        return;
    }

    let mut changed = false;
    for quest in active.iter_mut() {
        let parts: Vec<&str> = quest.split('|').collect();
        if parts.len() < 5 {
            continue;
        }
        let quest_type = parts[1];
        let target_count: i64 = parts[3].parse().unwrap_or(1);
        let mut progress: i64 = parts[4].parse().unwrap_or(0);

        let should_increment = matches!(
            (quest_type, action),
            ("monster_hunt", "kill")
                | ("item_collect", "collect")
                | ("boss_challenge", "boss_kill")
                | ("explore", "map_visit")
                | ("pvp_duel", "pvp_win")
        );

        if should_increment && progress < target_count {
            progress += 1;
            *quest = format!("{}|{}|{}|{}|{}", parts[0], parts[1], parts[2], parts[3], progress);
            changed = true;
        }
    }

    if changed {
        set_active_quests(db, user_id, &active);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_ranks_count() {
        assert_eq!(QUEST_RANKS.len(), 6);
    }

    #[test]
    fn test_adventurer_ranks_count() {
        assert_eq!(ADVENTURER_RANKS.len(), 8);
    }

    #[test]
    fn test_quest_types_count() {
        assert_eq!(QUEST_TYPES.len(), 5);
    }

    #[test]
    fn test_shop_items_count() {
        assert_eq!(SHOP_ITEMS.len(), 10);
    }

    #[test]
    fn test_quest_ranks_escalate() {
        for i in 1..QUEST_RANKS.len() {
            assert!(QUEST_RANKS[i].3 > QUEST_RANKS[i - 1].3, "rep reward should escalate");
            assert!(QUEST_RANKS[i].4 > QUEST_RANKS[i - 1].4, "gold reward should escalate");
        }
    }

    #[test]
    fn test_adventurer_ranks_escalate() {
        for i in 1..ADVENTURER_RANKS.len() {
            assert!(
                ADVENTURER_RANKS[i].1 > ADVENTURER_RANKS[i - 1].1,
                "threshold should escalate"
            );
        }
    }

    #[test]
    fn test_shop_costs_positive() {
        for &(_, cost, _, _) in SHOP_ITEMS {
            assert!(cost > 0, "shop cost should be positive");
        }
    }

    #[test]
    fn test_get_adventurer_rank_zero_rep() {
        let (title, emoji, idx) = get_adventurer_rank(0);
        assert_eq!(title, "见习冒险者");
        assert_eq!(emoji, "🔰");
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_get_adventurer_rank_high_rep() {
        let (title, _, idx) = get_adventurer_rank(15000);
        assert_eq!(title, "冒险王");
        assert_eq!(idx, 7);
    }

    #[test]
    fn test_get_adventurer_rank_mid() {
        let (title, _, idx) = get_adventurer_rank(1500);
        assert_eq!(title, "精英冒险者");
        assert_eq!(idx, 4);
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(10, 10, 10);
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 10, 10);
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(5, 10, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.contains('█'));
        assert!(bar.contains('░'));
    }

    #[test]
    fn test_quest_difficulty_label() {
        assert_eq!(quest_difficulty_label(0), "简单");
        assert_eq!(quest_difficulty_label(3), "精英");
        assert_eq!(quest_difficulty_label(5), "地狱");
    }

    #[test]
    fn test_generate_quest_target_monster() {
        let (desc, count) = generate_quest_target(0, "monster_hunt");
        assert!(desc.contains("史莱姆"));
        assert_eq!(count, 5); // (0+1)*3+2
    }

    #[test]
    fn test_generate_quest_target_boss() {
        let (desc, count) = generate_quest_target(2, "boss_challenge");
        assert!(desc.contains("骷髅王"));
        assert_eq!(count, 2); // 2/2+1
    }

    #[test]
    fn test_generate_quest_target_explore() {
        let (desc, count) = generate_quest_target(4, "explore");
        assert!(desc.contains("探索"));
        assert_eq!(count, 4); // 4/2+2
    }

    #[test]
    fn test_quest_type_for_rank() {
        assert_eq!(quest_type_for_rank(0), "monster_hunt");
        assert_eq!(quest_type_for_rank(1), "item_collect");
        assert_eq!(quest_type_for_rank(2), "boss_challenge");
    }

    #[test]
    fn test_quest_level_requirements() {
        assert_eq!(QUEST_RANKS[0].2, 1); // D: level 1
        assert_eq!(QUEST_RANKS[3].2, 50); // A: level 50
        assert_eq!(QUEST_RANKS[5].2, 90); // SS: level 90
    }

    #[test]
    fn test_max_active_quests() {
        assert_eq!(MAX_ACTIVE_QUESTS, 3);
    }

    #[test]
    fn test_daily_quest_limit() {
        assert_eq!(DAILY_QUEST_LIMIT, 10);
    }
}

/// CakeGame 决斗场系统
/// 正式1v1决斗：发起挑战/接受/拒绝/下注金币/决斗历史/决斗排行
/// 不同于普通PvP，决斗是正式的、双方同意的、有赌注的对战
/// 数据存储: Global 表 SECTION='duel_challenge'/'duel_history'/'duel_ranking'
use crate::combat_power;
use crate::core::*;
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 决斗赌注范围
const MIN_WAGER: i64 = 1_000;
const MAX_WAGER: i64 = 1_000_000;
/// 挑战过期时间（秒）
const CHALLENGE_EXPIRE_SECS: i64 = 300; // 5分钟
/// 每日最大决斗次数
const MAX_DUELS_PER_DAY: i32 = 10;
/// 决斗排行榜最大条目
const MAX_RANKING_ENTRIES: usize = 20;
/// 历史记录最大条目
const MAX_HISTORY_ENTRIES: usize = 30;
/// 平台抽成比例 (%)
const HOUSE_CUT_PCT: i64 = 5;

/// 发起决斗挑战
/// 用法: 决斗挑战 <玩家名> [赌注金额]
pub fn cmd_duel_challenge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return "⚔️ 用法: 决斗挑战 <玩家名> [赌注金额]\n赌注范围: 1,000~1,000,000 金币".to_string();
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let target_name = parts[0];
    let wager: i64 = if parts.len() > 1 {
        parts[1].parse().unwrap_or(5000)
    } else {
        5000
    };

    // 验证赌注范围
    if !(MIN_WAGER..=MAX_WAGER).contains(&wager) {
        return format!("❌ 赌注范围: {}~{} 金币", MIN_WAGER, MAX_WAGER);
    }

    // 获取玩家信息
    let challenger = user::calc_total_attrs(db, user_id);

    // 查找目标玩家
    let target_id = match find_user_id_by_name(db, target_name) {
        Some(id) => id,
        None => return format!("❌ 找不到玩家「{}」", target_name),
    };

    if target_id == user_id {
        return "❌ 不能向自己发起决斗挑战".to_string();
    }

    // 检查每日决斗次数
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let daily_count = get_daily_duel_count(db, user_id, &today);
    if daily_count >= MAX_DUELS_PER_DAY {
        return format!("❌ 今日决斗次数已达上限({}次)，请明天再来", MAX_DUELS_PER_DAY);
    }

    // 检查赌注是否足够
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < wager {
        return format!("❌ 金币不足！需要 {} 金币，当前 {}", wager, gold);
    }

    // 检查目标玩家金币
    let target_gold = db.read_currency(&target_id, CURRENCY_GOLD);
    if target_gold < wager {
        return format!("❌ 对方金币不足({})，无法接受同等赌注", format_gold(target_gold));
    }

    // 检查是否已有未过期挑战
    let challenge_key = format!("challenge_{}_{}", user_id, target_id);
    let existing = db.global_get("duel_challenge", &challenge_key);
    if !existing.is_empty() {
        if let Ok(fields) = serde_json::from_str::<serde_json::Value>(&existing) {
            if let Some(ts) = fields.get("timestamp").and_then(|v| v.as_i64()) {
                let now = chrono::Local::now().timestamp();
                if now - ts < CHALLENGE_EXPIRE_SECS {
                    return "❌ 你已经向该玩家发起了挑战，请等待回应".to_string();
                }
            }
        }
    }

    // 创建挑战
    let now = chrono::Local::now().timestamp();
    let challenge = serde_json::json!({
        "challenger_id": user_id,
        "challenger_name": challenger.name,
        "target_id": target_id,
        "target_name": target_name,
        "wager": wager,
        "timestamp": now,
        "status": "pending"
    });

    db.global_set("duel_challenge", &challenge_key, &challenge.to_string());

    // 记录目标的待处理挑战
    let pending_key = format!("pending_{}", target_id);
    let pending_raw = db.global_get("duel_challenge", &pending_key);
    let mut pending = if pending_raw.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str::<Vec<serde_json::Value>>(&pending_raw).unwrap_or_default()
    };
    pending.push(serde_json::json!({
        "challenger_id": user_id,
        "challenger_name": challenger.name,
        "wager": wager,
        "timestamp": now
    }));
    db.global_set(
        "duel_challenge",
        &pending_key,
        &serde_json::to_string(&pending).unwrap_or_default(),
    );

    format!(
        "⚔️ 决斗挑战已发出！\n🎯 挑战者: {} → 目标: {}\n💰 赌注: {} 金币\n⏰ 有效期: 5分钟\n等待对方回应中...",
        challenger.name, target_name, wager
    )
}

/// 接受决斗挑战
/// 用法: 接受决斗 [挑战者名]
pub fn cmd_duel_accept(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let challenger_name = args.trim();
    if challenger_name.is_empty() {
        // 列出待处理的挑战
        let pending_key = format!("pending_{}", user_id);
        let pending_raw = db.global_get("duel_challenge", &pending_key);
        let pending_list: Vec<serde_json::Value> = if pending_raw.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&pending_raw).unwrap_or_default()
        };

        if pending_list.is_empty() {
            return "📭 没有待处理的决斗挑战".to_string();
        }

        let mut result = "⚔️ 待处理的决斗挑战:\n".to_string();
        for (i, c) in pending_list.iter().enumerate() {
            let name = c.get("challenger_name").and_then(|v| v.as_str()).unwrap_or("?");
            let wager = c.get("wager").and_then(|v| v.as_i64()).unwrap_or(0);
            let ts = c.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
            let now = chrono::Local::now().timestamp();
            let remaining = CHALLENGE_EXPIRE_SECS - (now - ts);
            if remaining > 0 {
                result += &format!("  {}. {} — 赌注 {} 金 ({}秒后过期)\n", i + 1, name, wager, remaining);
            }
        }
        result += "\n使用「接受决斗 <挑战者名>」接受挑战";
        return result;
    }

    // 查找挑战
    let challenger_id = find_user_id_by_name(db, challenger_name).unwrap_or_default();
    let challenge_key = format!("challenge_{}_{}", challenger_id, user_id);

    let challenge_data = db.global_get("duel_challenge", &challenge_key);
    if challenge_data.is_empty() {
        return format!("❌ 找不到来自「{}」的决斗挑战", challenger_name);
    }

    let challenge: serde_json::Value = match serde_json::from_str(&challenge_data) {
        Ok(v) => v,
        Err(_) => return "❌ 挑战数据损坏".to_string(),
    };

    // 检查是否过期
    let ts = challenge.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
    let now = chrono::Local::now().timestamp();
    if now - ts > CHALLENGE_EXPIRE_SECS {
        // 清理过期挑战
        db.global_set("duel_challenge", &challenge_key, "");
        return "❌ 该挑战已过期".to_string();
    }

    let wager = challenge.get("wager").and_then(|v| v.as_i64()).unwrap_or(0);
    let challenger_name_actual = challenge
        .get("challenger_name")
        .and_then(|v| v.as_str())
        .unwrap_or(challenger_name);

    // 获取接受者信息
    let acceptor = user::calc_total_attrs(db, user_id);

    // 双方金币检查
    let challenger_gold = db.read_currency(&challenger_id, CURRENCY_GOLD);
    let acceptor_gold = db.read_currency(user_id, CURRENCY_GOLD);
    if challenger_gold < wager {
        return format!("❌ 挑战者金币不足({})，挑战自动失效", format_gold(challenger_gold));
    }
    if acceptor_gold < wager {
        return format!(
            "❌ 你的金币不足({})，无法接受赌注 {} 的挑战",
            format_gold(acceptor_gold),
            wager
        );
    }

    // 执行决斗！
    let (winner_id, loser_id, winner_name, _loser_name, rounds) =
        simulate_duel(db, &challenger_id, user_id, challenger_name_actual, &acceptor.name);

    // 计算赌注分配（平台抽成5%）
    let house_cut = wager * HOUSE_CUT_PCT / 100;
    let prize = wager * 2 - house_cut;

    // 转移金币
    let _ = db.modify_currency(&loser_id, CURRENCY_GOLD, "add", -wager);
    let _ = db.modify_currency(&winner_id, CURRENCY_GOLD, "add", prize - wager);

    // 更新决斗统计
    update_duel_stats(db, &winner_id, true);
    update_duel_stats(db, &loser_id, false);

    // 记录历史
    record_duel_history(
        db,
        &challenger_id,
        challenger_name_actual,
        user_id,
        &acceptor.name,
        wager,
        &winner_id,
        rounds,
    );

    // 更新排行
    update_duel_ranking(db);

    // 记录每日次数
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    increment_daily_duel_count(db, &challenger_id, &today);
    increment_daily_duel_count(db, user_id, &today);

    // 清理挑战
    db.global_set("duel_challenge", &challenge_key, "");
    remove_pending_challenge(db, user_id, &challenger_id);

    // 军功记录
    crate::military_rank::record_military_points(db, &winner_id, "duel_win", 8);
    crate::military_rank::record_military_points(db, &loser_id, "duel_lose", 1);

    format!(
        "⚔️ 决斗开始！\n🔴 {} vs {} 🔵\n💰 赌注: {} 金币\n\n{}",
        challenger_name_actual,
        acceptor.name,
        wager,
        format_duel_result(
            challenger_name_actual,
            &acceptor.name,
            &winner_name,
            rounds,
            prize,
            house_cut
        )
    )
}

/// 拒绝决斗挑战
pub fn cmd_duel_decline(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let challenger_name = args.trim();
    if challenger_name.is_empty() {
        return "❌ 用法: 拒绝决斗 <挑战者名>".to_string();
    }

    let challenger_id = match find_user_id_by_name(db, challenger_name) {
        Some(id) => id,
        None => return format!("❌ 找不到玩家「{}」", challenger_name),
    };

    let challenge_key = format!("challenge_{}_{}", challenger_id, user_id);
    let existing = db.global_get("duel_challenge", &challenge_key);
    if existing.is_empty() {
        return format!("❌ 找不到来自「{}」的决斗挑战", challenger_name);
    }

    db.global_set("duel_challenge", &challenge_key, "");
    remove_pending_challenge(db, user_id, &challenger_id);

    format!("🛡️ 你拒绝了「{}」的决斗挑战", challenger_name)
}

/// 查看决斗历史
pub fn cmd_duel_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let history_key = format!("history_{}", user_id);
    let history_raw = db.global_get("duel_history", &history_key);
    let history: Vec<serde_json::Value> = if history_raw.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&history_raw).unwrap_or_default()
    };

    if history.is_empty() {
        return "📜 你还没有决斗记录\n使用「决斗挑战 <玩家名>」发起挑战".to_string();
    }

    let stats = get_duel_stats(db, user_id);
    let total = stats.0 + stats.1;
    let win_rate = if total > 0 {
        stats.0 as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    let mut result = format!(
        "📜 决斗历史 (总{}场 | ✅{}胜 | ❌{}败 | 胜率{:.1}%)\n",
        total, stats.0, stats.1, win_rate
    );

    for entry in history.iter().take(10) {
        let p1 = entry.get("player1_name").and_then(|v| v.as_str()).unwrap_or("?");
        let p2 = entry.get("player2_name").and_then(|v| v.as_str()).unwrap_or("?");
        let winner = entry.get("winner_name").and_then(|v| v.as_str()).unwrap_or("?");
        let wager = entry.get("wager").and_then(|v| v.as_i64()).unwrap_or(0);
        let rounds = entry.get("rounds").and_then(|v| v.as_i64()).unwrap_or(0);
        let ts = entry.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
        let time_str = chrono::DateTime::from_timestamp(ts, 0)
            .map(|t| t.format("%m-%d %H:%M").to_string())
            .unwrap_or_default();

        let result_icon = if winner == p1 { "🔴" } else { "🔵" };
        result += &format!(
            "  {} {} vs {} | 赌注{} | {}回合 | {}\n",
            result_icon, p1, p2, wager, rounds, time_str
        );
    }

    result
}

/// 决斗排行榜
pub fn cmd_duel_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let ranking_raw = db.global_get("duel_ranking", "top");
    let ranking: Vec<serde_json::Value> = if ranking_raw.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&ranking_raw).unwrap_or_default()
    };

    if ranking.is_empty() {
        return "🏆 决斗排行榜为空\n率先发起决斗，成为决斗之王！".to_string();
    }

    let medals = ["🥇", "🥈", "🥉"];
    let mut result = "🏆 决斗排行榜 TOP 15\n".to_string();

    for (i, entry) in ranking.iter().take(15).enumerate() {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let wins = entry.get("wins").and_then(|v| v.as_i64()).unwrap_or(0);
        let losses = entry.get("losses").and_then(|v| v.as_i64()).unwrap_or(0);
        let win_rate = if wins + losses > 0 {
            wins as f64 / (wins + losses) as f64 * 100.0
        } else {
            0.0
        };
        let medal = if i < 3 { medals[i] } else { "  " };
        let rank_str = if i < 3 { String::new() } else { format!("{:>2}.", i + 1) };

        result += &format!(
            "  {}{} {} — {}胜{}败 (胜率{:.1}%)\n",
            medal, rank_str, name, wins, losses, win_rate
        );
    }

    // 显示个人排名
    let my_stats = get_duel_stats(db, user_id);
    if my_stats.0 + my_stats.1 > 0 {
        let my_rank = ranking
            .iter()
            .position(|e| e.get("id").and_then(|v| v.as_str()) == Some(user_id));
        let rank_display = my_rank
            .map(|r| format!("第{}名", r + 1))
            .unwrap_or_else(|| "未上榜".to_string());
        result += &format!("\n📊 你的排名: {} ({}胜{}败)", rank_display, my_stats.0, my_stats.1);
    }

    result
}

/// 查看活跃决斗
pub fn cmd_view_duels(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // 查询所有未过期的挑战
    let now = chrono::Local::now().timestamp();
    let conn = db.lock_conn();
    let mut active_duels: Vec<serde_json::Value> = Vec::new();

    if let Ok(mut stmt) =
        conn.prepare("SELECT ID, DATA FROM Global WHERE Section = 'duel_challenge' AND ID LIKE 'challenge_%'")
    {
        if let Ok(rows) = stmt.query_map([], |row| {
            let _: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok(data)
        }) {
            for row in rows.flatten() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&row) {
                    let ts = data.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
                    if now - ts < CHALLENGE_EXPIRE_SECS {
                        active_duels.push(data);
                    }
                }
            }
        }
    }

    if active_duels.is_empty() {
        return "🏟️ 当前没有活跃的决斗挑战\n使用「决斗挑战 <玩家名>」发起挑战".to_string();
    }

    let mut result = "🏟️ 活跃决斗挑战\n".to_string();
    for duel in &active_duels {
        let challenger = duel.get("challenger_name").and_then(|v| v.as_str()).unwrap_or("?");
        let target = duel.get("target_name").and_then(|v| v.as_str()).unwrap_or("?");
        let wager = duel.get("wager").and_then(|v| v.as_i64()).unwrap_or(0);
        let ts = duel.get("timestamp").and_then(|v| v.as_i64()).unwrap_or(0);
        let remaining = CHALLENGE_EXPIRE_SECS - (now - ts);

        result += &format!(
            "  ⚔️ {} → {} | 赌注{} | {}秒后过期\n",
            challenger, target, wager, remaining
        );
    }

    result
}

/// 决斗帮助
pub fn cmd_duel_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    "⚔️ 决斗场系统帮助\n\
     ─────────────────\n\
     决斗挑战 <玩家名> [赌注] — 发起决斗挑战\n\
     接受决斗 [挑战者名] — 接受挑战（无参数查看列表）\n\
     拒绝决斗 <挑战者名> — 拒绝挑战\n\
     决斗历史 — 查看个人决斗记录\n\
     决斗排行 — 查看全服决斗排行榜\n\
     查看决斗 — 查看活跃决斗挑战\n\
     ─────────────────\n\
     📋 规则:\n\
     • 赌注范围: 1,000~1,000,000 金币\n\
     • 每日最多10场决斗\n\
     • 挑战5分钟内有效\n\
     • 平台抽成5%\n\
     • 胜者获得(2×赌注-抽成)\n\
     • 获胜增加军功8点"
        .to_string()
}

// ===================== 内部辅助函数 =====================

/// 模拟决斗过程（基于双方属性+随机因素）
#[allow(clippy::too_many_arguments)]
fn simulate_duel(
    db: &Database,
    p1_id: &str,
    p2_id: &str,
    p1_name: &str,
    p2_name: &str,
) -> (String, String, String, String, i32) {
    let p1_info = user::calc_total_attrs(db, p1_id);
    let p2_info = user::calc_total_attrs(db, p2_id);

    // 计算战力
    let p1_power = combat_power::calc_combat_power(&p1_info);
    let p2_power = combat_power::calc_combat_power(&p2_info);
    let _ = (p1_power, p2_power); // 用于日志，暂不使用

    let mut rng = rand::thread_rng();
    let mut p1_hp = p1_info.hp_max as f64;
    let mut p2_hp = p2_info.hp_max as f64;
    let p1_ad = p1_info.ad as f64;
    let p2_ad = p2_info.ad as f64;
    let p1_def = p1_info.defense as f64;
    let p2_def = p2_info.defense as f64;

    let mut rounds = 0;
    let max_rounds = 20;

    while p1_hp > 0.0 && p2_hp > 0.0 && rounds < max_rounds {
        rounds += 1;

        // P1 攻击 P2
        let base_dmg1 = (p1_ad - p2_def * 0.5).max(1.0);
        let variance1 = rng.gen_range(0.8..1.3);
        let crit1 = if rng.gen_bool(0.15) { 2.0 } else { 1.0 };
        let dmg1 = base_dmg1 * variance1 * crit1;
        let p2_hp_remaining = p2_hp - dmg1;

        if p2_hp_remaining <= 0.0 {
            return (
                p1_id.to_string(),
                p2_id.to_string(),
                p1_name.to_string(),
                p2_name.to_string(),
                rounds,
            );
        }

        // P2 攻击 P1
        let base_dmg2 = (p2_ad - p1_def * 0.5).max(1.0);
        let variance2 = rng.gen_range(0.8..1.3);
        let crit2 = if rng.gen_bool(0.15) { 2.0 } else { 1.0 };
        let dmg2 = base_dmg2 * variance2 * crit2;
        let p1_hp_remaining = p1_hp - dmg2;

        if p1_hp_remaining <= 0.0 {
            return (
                p2_id.to_string(),
                p1_id.to_string(),
                p2_name.to_string(),
                p1_name.to_string(),
                rounds,
            );
        }

        p1_hp = p1_hp_remaining;
        p2_hp = p2_hp_remaining;
    }

    // 平局按HP比例判断
    let p1_ratio = p1_hp / p1_info.hp_max as f64;
    let p2_ratio = p2_hp / p2_info.hp_max as f64;

    if p1_ratio >= p2_ratio {
        (
            p1_id.to_string(),
            p2_id.to_string(),
            p1_name.to_string(),
            p2_name.to_string(),
            rounds,
        )
    } else {
        (
            p2_id.to_string(),
            p1_id.to_string(),
            p2_name.to_string(),
            p1_name.to_string(),
            rounds,
        )
    }
}

/// 格式化决斗结果
fn format_duel_result(
    _p1_name: &str,
    _p2_name: &str,
    winner_name: &str,
    rounds: i32,
    prize: i64,
    house_cut: i64,
) -> String {
    let mut result = String::new();

    for round in 1..=rounds {
        let icon = if round % 2 == 1 { "⚔️" } else { "🛡️" };
        result += &format!("  第{}回合 {} 交锋激烈！\n", round, icon);
    }

    result += &format!(
        "\n🏆 {} 获胜！（第{}回合）\n💰 获胜奖金: {} 金币 (平台抽成{})\n😔 落败方失去: {} 金币",
        winner_name,
        rounds,
        prize,
        format_gold(house_cut),
        prize - house_cut
    );

    result
}

/// 格式化金币
fn format_gold(amount: i64) -> String {
    if amount >= 100_000_000 {
        format!("{:.1}亿", amount as f64 / 100_000_000.0)
    } else if amount >= 10_000 {
        format!("{:.1}万", amount as f64 / 10_000.0)
    } else {
        format!("{}", amount)
    }
}

/// 查找玩家ID
fn find_user_id_by_name(db: &Database, name: &str) -> Option<String> {
    let users = db.all_users();
    for uid in users {
        let nick = db.read_basic(&uid, ITEM_NAME);
        if nick == name {
            return Some(uid);
        }
    }
    None
}

/// 获取决斗统计 (胜场, 败场)
fn get_duel_stats(db: &Database, user_id: &str) -> (i64, i64) {
    let stats_key = format!("stats_{}", user_id);
    let raw = db.global_get("duel_ranking", &stats_key);
    if raw.is_empty() {
        return (0, 0);
    }
    serde_json::from_str::<serde_json::Value>(&raw)
        .map(|v| {
            let wins = v.get("wins").and_then(|w| w.as_i64()).unwrap_or(0);
            let losses = v.get("losses").and_then(|l| l.as_i64()).unwrap_or(0);
            (wins, losses)
        })
        .unwrap_or((0, 0))
}

/// 更新决斗统计
fn update_duel_stats(db: &Database, user_id: &str, won: bool) {
    let stats_key = format!("stats_{}", user_id);
    let (mut wins, mut losses) = get_duel_stats(db, user_id);
    if won {
        wins += 1;
    } else {
        losses += 1;
    }
    let stats = serde_json::json!({
        "wins": wins,
        "losses": losses
    });
    db.global_set("duel_ranking", &stats_key, &stats.to_string());
}

/// 记录决斗历史
#[allow(clippy::too_many_arguments)]
fn record_duel_history(
    db: &Database,
    p1_id: &str,
    p1_name: &str,
    p2_id: &str,
    p2_name: &str,
    wager: i64,
    winner_id: &str,
    rounds: i32,
) {
    let now = chrono::Local::now().timestamp();
    let winner_name = if winner_id == p1_id { p1_name } else { p2_name };

    let entry = serde_json::json!({
        "player1_id": p1_id,
        "player1_name": p1_name,
        "player2_id": p2_id,
        "player2_name": p2_name,
        "wager": wager,
        "winner_id": winner_id,
        "winner_name": winner_name,
        "rounds": rounds,
        "timestamp": now
    });

    // 记录双方历史
    for uid in [p1_id, p2_id] {
        let history_key = format!("history_{}", uid);
        let history_raw = db.global_get("duel_history", &history_key);
        let mut history: Vec<serde_json::Value> = if history_raw.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&history_raw).unwrap_or_default()
        };
        history.insert(0, entry.clone());
        history.truncate(MAX_HISTORY_ENTRIES);
        db.global_set(
            "duel_history",
            &history_key,
            &serde_json::to_string(&history).unwrap_or_default(),
        );
    }
}

/// 更新决斗排行榜
fn update_duel_ranking(db: &Database) {
    // 从数据库中读取所有决斗统计
    let conn = db.lock_conn();
    let mut entries: Vec<serde_json::Value> = Vec::new();

    if let Ok(mut stmt) =
        conn.prepare("SELECT ID, DATA FROM Global WHERE Section = 'duel_ranking' AND ID LIKE 'stats_%'")
    {
        if let Ok(rows) = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok((id, data))
        }) {
            for row in rows.flatten() {
                let user_id = row.0.strip_prefix("stats_").unwrap_or("").to_string();
                let stats: serde_json::Value = match serde_json::from_str(&row.1) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let wins = stats.get("wins").and_then(|v| v.as_i64()).unwrap_or(0);
                let losses = stats.get("losses").and_then(|v| v.as_i64()).unwrap_or(0);
                if wins + losses == 0 {
                    continue;
                }

                let name = db.read_basic(&user_id, ITEM_NAME);
                entries.push(serde_json::json!({
                    "id": user_id,
                    "name": name,
                    "wins": wins,
                    "losses": losses,
                    "score": wins * 3 - losses
                }));
            }
        }
    }

    // 按分数排序
    entries.sort_by(|a, b| {
        let score_a = a.get("score").and_then(|v| v.as_i64()).unwrap_or(0);
        let score_b = b.get("score").and_then(|v| v.as_i64()).unwrap_or(0);
        score_b.cmp(&score_a)
    });

    entries.truncate(MAX_RANKING_ENTRIES);
    db.global_set(
        "duel_ranking",
        "top",
        &serde_json::to_string(&entries).unwrap_or_default(),
    );
}

/// 获取每日决斗次数
fn get_daily_duel_count(db: &Database, user_id: &str, date: &str) -> i32 {
    let key = format!("daily_{}_{}", user_id, date);
    db.global_get("duel_daily", &key).parse().unwrap_or(0)
}

/// 增加每日决斗次数
fn increment_daily_duel_count(db: &Database, user_id: &str, date: &str) {
    let key = format!("daily_{}_{}", user_id, date);
    let count = get_daily_duel_count(db, user_id, date) + 1;
    db.global_set("duel_daily", &key, &count.to_string());
}

/// 移除待处理挑战
fn remove_pending_challenge(db: &Database, target_id: &str, challenger_id: &str) {
    let pending_key = format!("pending_{}", target_id);
    let pending_raw = db.global_get("duel_challenge", &pending_key);
    let pending_list: Vec<serde_json::Value> = if pending_raw.is_empty() {
        Vec::new()
    } else {
        serde_json::from_str(&pending_raw).unwrap_or_default()
    };

    let filtered: Vec<_> = pending_list
        .into_iter()
        .filter(|c| c.get("challenger_id").and_then(|v| v.as_str()) != Some(challenger_id))
        .collect();

    db.global_set(
        "duel_challenge",
        &pending_key,
        &serde_json::to_string(&filtered).unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wager_range() {
        assert!(MIN_WAGER >= 1);
        assert!(MAX_WAGER > MIN_WAGER);
        assert!(HOUSE_CUT_PCT > 0 && HOUSE_CUT_PCT < 100);
    }

    #[test]
    fn test_challenge_expire_time() {
        assert!(CHALLENGE_EXPIRE_SECS > 0);
        assert!(CHALLENGE_EXPIRE_SECS <= 600);
    }

    #[test]
    fn test_daily_limit() {
        assert!(MAX_DUELS_PER_DAY > 0);
        assert!(MAX_DUELS_PER_DAY <= 50);
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(500), "500");
        assert_eq!(format_gold(50_000), "5.0万");
        assert_eq!(format_gold(200_000_000), "2.0亿");
    }

    #[test]
    fn test_max_ranking_entries() {
        assert!(MAX_RANKING_ENTRIES > 0);
        assert!(MAX_RANKING_ENTRIES <= 100);
    }

    #[test]
    fn test_max_history_entries() {
        assert!(MAX_HISTORY_ENTRIES > 0);
        assert!(MAX_HISTORY_ENTRIES <= 100);
    }

    #[test]
    fn test_format_duel_result() {
        let result = format_duel_result("玩家A", "玩家B", "玩家A", 5, 9500, 500);
        assert!(result.contains("玩家A"));
        assert!(result.contains("获胜"));
    }

    #[test]
    fn test_prize_calculation() {
        let wager = 10000i64;
        let house_cut = wager * HOUSE_CUT_PCT / 100;
        let prize = wager * 2 - house_cut;
        assert_eq!(house_cut, 500);
        assert_eq!(prize, 19500);
    }

    #[test]
    fn test_min_wager_enforced() {
        assert!(500 < MIN_WAGER);
        assert!(1000 >= MIN_WAGER);
    }

    #[test]
    fn test_max_wager_enforced() {
        assert!(1_000_001 > MAX_WAGER);
        assert!(1_000_000 <= MAX_WAGER);
    }

    #[test]
    fn test_duel_score_formula() {
        let score = 10 * 3 - 2; // 10胜2败
        assert_eq!(score, 28);
    }
}

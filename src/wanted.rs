/// 全服悬赏通缉系统
///
/// 玩家可以发布悬赏通缉其他玩家（需要一定邪恶值或花费金币），
/// 其他玩家可以接受通缉任务并获得赏金。
/// 数据存储在 Global 表:
///   SECTION='wanted_bounty', ID='bounty_{timestamp}_{poster}' → JSON悬赏数据
///   SECTION='wanted_hunter', ID='{user_id}' → 猎人统计JSON
///   SECTION='wanted_stats', ID='global' → 全服通缉统计
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 通缉赏金状态
#[derive(Debug, Clone, PartialEq)]
enum BountyStatus {
    Active,    // 活跃 — 可被接受
    Accepted,  // 已接受 — 猎人正在追踪
    Completed, // 已完成 — 猎人成功击杀
    Expired,   // 已过期
    Cancelled, // 已取消
}

impl BountyStatus {
    fn as_str(&self) -> &'static str {
        match self {
            BountyStatus::Active => "活跃",
            BountyStatus::Accepted => "追踪中",
            BountyStatus::Completed => "已完成",
            BountyStatus::Expired => "已过期",
            BountyStatus::Cancelled => "已取消",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "活跃" => BountyStatus::Active,
            "追踪中" => BountyStatus::Accepted,
            "已完成" => BountyStatus::Completed,
            "已过期" => BountyStatus::Expired,
            "已取消" => BountyStatus::Cancelled,
            _ => BountyStatus::Active,
        }
    }
}

/// 悬赏数据
#[derive(Debug, Clone)]
struct Bounty {
    id: String,
    poster: String,      // 发布者 user_id
    target: String,      // 目标 user_id
    target_name: String, // 目标昵称
    reward_gold: i64,    // 金币赏金
    reward_diamond: i64, // 钻石赏金
    reason: String,      // 通缉原因
    status: BountyStatus,
    hunter: String,     // 接受的猎人 user_id
    created_at: String, // 创建时间
    expire_at: String,  // 过期时间
    evil_value: i32,    // 目标邪恶值
}

impl Bounty {
    fn to_json(&self) -> String {
        format!(
            "{{\"poster\":\"{}\",\"target\":\"{}\",\"target_name\":\"{}\",\"reward_gold\":{},\"reward_diamond\":{},\"reason\":\"{}\",\"status\":\"{}\",\"hunter\":\"{}\",\"created_at\":\"{}\",\"expire_at\":\"{}\",\"evil_value\":{}}}",
            self.poster, self.target, self.target_name, self.reward_gold, self.reward_diamond,
            self.reason, self.status.as_str(), self.hunter, self.created_at, self.expire_at, self.evil_value
        )
    }

    fn from_json(id: &str, json: &str) -> Option<Self> {
        let get = |key: &str| -> String {
            let pattern = format!("\"{}\":", key);
            if let Some(start) = json.find(&pattern) {
                let after = &json[start + pattern.len()..];
                // Handle quoted strings vs numbers
                if let Some(rest) = after.strip_prefix('"') {
                    if let Some(end) = rest.find('"') {
                        return rest[..end].to_string();
                    }
                } else {
                    let end = after.find([',', '}']).unwrap_or(after.len());
                    return after[..end].trim().to_string();
                }
            }
            String::new()
        };

        Some(Bounty {
            id: id.to_string(),
            poster: get("poster"),
            target: get("target"),
            target_name: get("target_name"),
            reward_gold: get("reward_gold").parse().unwrap_or(0),
            reward_diamond: get("reward_diamond").parse().unwrap_or(0),
            reason: get("reason"),
            status: BountyStatus::from_str(&get("status")),
            hunter: get("hunter"),
            created_at: get("created_at"),
            expire_at: get("expire_at"),
            evil_value: get("evil_value").parse().unwrap_or(0),
        })
    }
}

/// 发布悬赏最低金币
const MIN_BOUNTY_GOLD: i64 = 5000;
/// 发布悬赏最低钻石
#[allow(dead_code)]
const MIN_BOUNTY_DIAMOND: i64 = 5;
/// 悬赏持续时间（小时）
const BOUNTY_DURATION_HOURS: i32 = 24;
/// 自动通缉邪恶值阈值
const AUTO_WANTED_EVIL_THRESHOLD: i32 = 50;
/// 猎人佣金比例 (赏金的 80%)
const HUNTER_COMMISSION_RATIO: f64 = 0.8;
/// 最大同时活跃悬赏数
const MAX_ACTIVE_BOUNTIES: usize = 20;
/// 最大单人悬赏数
const MAX_PER_USER_BOUNTIES: usize = 3;

/// 生成悬赏ID
fn bounty_id(poster: &str) -> String {
    let now = chrono::Local::now();
    let ts = now.timestamp();
    let mut hasher = DefaultHasher::new();
    format!("{}_{}", poster, ts).hash(&mut hasher);
    format!("bounty_{}_{:08x}", ts, hasher.finish() as u32)
}

/// 获取当前时间字符串
fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 计算过期时间
fn expire_str() -> String {
    let expire = chrono::Local::now() + chrono::Duration::hours(BOUNTY_DURATION_HOURS as i64);
    expire.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 加载所有活跃悬赏
fn load_active_bounties(db: &Database) -> Vec<Bounty> {
    let mut bounties = Vec::new();
    let conn = db.lock_conn();
    if let Ok(mut stmt) =
        conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION = 'wanted_bounty' AND ID LIKE 'bounty_%'")
    {
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            for row in rows.flatten() {
                let (id, data) = row;
                if data.is_empty() {
                    continue;
                }
                if let Some(b) = Bounty::from_json(&id, &data) {
                    // 自动过期检查
                    if b.status == BountyStatus::Active || b.status == BountyStatus::Accepted {
                        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        if now > b.expire_at {
                            // 过期，不加载
                            continue;
                        }
                    }
                    bounties.push(b);
                }
            }
        }
    }
    bounties
}

/// 保存悬赏数据
fn save_bounty(db: &Database, bounty: &Bounty) {
    db.global_set("wanted_bounty", &bounty.id, &bounty.to_json());
}

/// 查看通缉榜
pub fn cmd_view_wanted(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let bounties = load_active_bounties(db);
    let active: Vec<&Bounty> = bounties
        .iter()
        .filter(|b| b.status == BountyStatus::Active || b.status == BountyStatus::Accepted)
        .collect();

    let mut r = format!("{}\n═══ 🔴 全服通缉榜 ═══\n", prefix);

    if active.is_empty() {
        r.push_str("\n当前没有活跃的通缉令！\n");
        r.push_str("使用「发布通缉」来发布悬赏。");
        return r;
    }

    // 按赏金降序排列
    let mut sorted: Vec<&Bounty> = active;
    sorted.sort_by_key(|b| std::cmp::Reverse(b.reward_gold + b.reward_diamond * 1000));

    for (i, b) in sorted.iter().take(10).enumerate() {
        let status_icon = match b.status {
            BountyStatus::Active => "🔴",
            BountyStatus::Accepted => "🟡",
            _ => "⚪",
        };
        let evil_tag = if b.evil_value >= AUTO_WANTED_EVIL_THRESHOLD {
            " ⚠️极度危险"
        } else {
            ""
        };
        r.push_str(&format!(
            "\n{} #{} 「{}」{}\n  💰 {}金币 {}💎 | 原因: {}",
            status_icon,
            i + 1,
            b.target_name,
            evil_tag,
            format_gold(b.reward_gold),
            b.reward_diamond,
            b.reason
        ));
        if b.status == BountyStatus::Accepted {
            let hunter_name = user::get_msg_prefix(db, &b.hunter);
            r.push_str(&format!("\n  🗡️ 猎人: {} | 追踪中...", hunter_name));
        }
    }

    r.push_str(&format!(
        "\n\n📋 活跃通缉: {}条\n💡 使用「接受通缉+目标名」接受任务\n💡 使用「发布通缉+目标+金币+原因」发布悬赏",
        sorted.len()
    ));

    r
}

/// 发布通缉
pub fn cmd_post_bounty(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // 格式: 目标 金币 原因
    let parts: Vec<&str> = args.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return format!(
            "{}\n用法: 发布通缉 目标名 赏金金币 [原因]\n示例: 发布通缉 坏蛋 10000 恶意PK新手",
            prefix
        );
    }

    let target_name = parts[0].trim();
    let reward_gold: i64 = match parts[1].trim().parse() {
        Ok(n) => n,
        Err(_) => return format!("{}\n❌ 赏金必须是数字！", prefix),
    };
    let reason = if parts.len() >= 3 {
        parts[2].trim()
    } else {
        "违反江湖规矩"
    };

    if reward_gold < MIN_BOUNTY_GOLD {
        return format!("{}\n❌ 最低悬赏金额为 {} 金币！", prefix, MIN_BOUNTY_GOLD);
    }

    // 查找目标玩家
    let target_id = find_user_by_name(db, target_name);
    if target_id.is_none() {
        return format!("{}\n❌ 未找到玩家「{}」！", prefix, target_name);
    }
    let target_id = target_id.unwrap();

    if target_id == user_id {
        return format!("{}\n❌ 不能通缉自己！", prefix);
    }

    // 检查个人悬赏数量
    let bounties = load_active_bounties(db);
    let my_count = bounties
        .iter()
        .filter(|b| b.poster == user_id && (b.status == BountyStatus::Active || b.status == BountyStatus::Accepted))
        .count();
    if my_count >= MAX_PER_USER_BOUNTIES {
        return format!("{}\n❌ 您同时发布的通缉不能超过 {} 条！", prefix, MAX_PER_USER_BOUNTIES);
    }

    if bounties.len() >= MAX_ACTIVE_BOUNTIES {
        return format!(
            "{}\n❌ 全服通缉数量已达上限 {} 条，请等待过期后重试！",
            prefix, MAX_ACTIVE_BOUNTIES
        );
    }

    // 检查是否已对该目标有通缉
    let existing = bounties
        .iter()
        .find(|b| b.target == target_id && b.status == BountyStatus::Active);
    if existing.is_some() {
        return format!("{}\n❌ 「{}」已有活跃通缉！请追加赏金或等待过期。", prefix, target_name);
    }

    // 扣除金币
    let after_gold = db.modify_currency(user_id, CURRENCY_GOLD, "sub", reward_gold);
    if after_gold < 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", reward_gold);
        return format!(
            "{}\n❌ 金币不足！需要 {} 金币，当前 {}",
            prefix,
            reward_gold,
            after_gold + reward_gold
        );
    }

    // 获取目标邪恶值
    let evil_key = format!("evil_{}", target_id);
    let evil_value: i32 = db.global_get("xiaoheiwu_evil", &evil_key).parse().unwrap_or(0);

    // 创建通缉
    let bounty = Bounty {
        id: bounty_id(user_id),
        poster: user_id.to_string(),
        target: target_id.clone(),
        target_name: target_name.to_string(),
        reward_gold,
        reward_diamond: 0,
        reason: reason.to_string(),
        status: BountyStatus::Active,
        hunter: String::new(),
        created_at: now_str(),
        expire_at: expire_str(),
        evil_value,
    };

    save_bounty(db, &bounty);

    // 更新统计
    let total_posted: i32 = db.global_get("wanted_stats", "total_posted").parse().unwrap_or(0);
    db.global_set("wanted_stats", "total_posted", &(total_posted + 1).to_string());
    let total_gold: i64 = db.global_get("wanted_stats", "total_gold").parse().unwrap_or(0);
    db.global_set("wanted_stats", "total_gold", &(total_gold + reward_gold).to_string());

    let mut r = format!("{}\n✅ 通缉令发布成功！\n", prefix);
    r.push_str("━━━━━━━━━━━━━━\n");
    r.push_str(&format!("🎯 目标: {}\n", target_name));
    r.push_str(&format!("💰 赏金: {} 金币\n", format_gold(reward_gold)));
    r.push_str(&format!("📝 原因: {}\n", reason));
    r.push_str(&format!("⏰ 持续: {} 小时\n", BOUNTY_DURATION_HOURS));
    r.push_str("━━━━━━━━━━━━━━\n");
    r.push_str("其他玩家可使用「接受通缉」接取任务！");

    r
}

/// 接受通缉
pub fn cmd_accept_bounty(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n用法: 接受通缉 目标名\n示例: 接受通缉 坏蛋", prefix);
    }

    let bounties = load_active_bounties(db);

    // 检查是否有该目标的通缉
    let bounty = bounties
        .iter()
        .find(|b| b.target_name == target_name && b.status == BountyStatus::Active);

    if bounty.is_none() {
        return format!("{}\n❌ 未找到「{}」的活跃通缉令！", prefix, target_name);
    }
    let bounty = bounty.unwrap().clone();

    if bounty.poster == user_id {
        return format!("{}\n❌ 不能接受自己发布的通缉！", prefix);
    }

    if bounty.target == user_id {
        return format!("{}\n❌ 通缉目标不能自己接取！", prefix);
    }

    // 检查是否已有接受的通缉
    let my_accepted = bounties
        .iter()
        .find(|b| b.hunter == user_id && b.status == BountyStatus::Accepted);
    if my_accepted.is_some() {
        return format!("{}\n❌ 您已有正在追踪的通缉任务！请先完成或放弃。", prefix);
    }

    // 更新状态
    let mut updated = bounty;
    updated.status = BountyStatus::Accepted;
    updated.hunter = user_id.to_string();
    save_bounty(db, &updated);

    let mut r = format!("{}\n🗡️ 通缉任务接受成功！\n", prefix);
    r.push_str("━━━━━━━━━━━━━━\n");
    r.push_str(&format!("🎯 目标: {}\n", updated.target_name));
    r.push_str(&format!("💰 赏金: {} 金币\n", format_gold(updated.reward_gold)));
    r.push_str(&format!("📝 原因: {}\n", updated.reason));
    r.push_str("━━━━━━━━━━━━━━\n");
    r.push_str("在PvP中击败目标即可领取赏金！\n");
    r.push_str("击败后使用「领取赏金」领取奖励。");

    r
}

/// 领取赏金（击败目标后）
pub fn cmd_claim_bounty(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n用法: 领取赏金 目标名\n示例: 领取赏金 坏蛋", prefix);
    }

    let bounties = load_active_bounties(db);

    // 查找自己接受的对应通缉
    let bounty = bounties
        .iter()
        .find(|b| b.hunter == user_id && b.target_name == target_name && b.status == BountyStatus::Accepted);

    if bounty.is_none() {
        return format!("{}\n❌ 您没有关于「{}」的进行中通缉任务！", prefix, target_name);
    }
    let bounty = bounty.unwrap().clone();

    // 检查目标是否真的被击败过（通过检查近期PvP记录）
    let pvp_key = format!("pvp_win_{}_{}", user_id, bounty.target);
    let has_win: bool = !db.global_get("wanted_pvp", &pvp_key).is_empty();

    if !has_win {
        return format!(
            "{}\n❌ 您尚未在PvP中击败「{}」！\n💡 提示: 先在战斗中击败目标，再来领取赏金。",
            prefix, target_name
        );
    }

    // 发放赏金（猎人佣金）
    let gold_reward = (bounty.reward_gold as f64 * HUNTER_COMMISSION_RATIO) as i64;
    let diamond_reward = (bounty.reward_diamond as f64 * HUNTER_COMMISSION_RATIO) as i64;

    if gold_reward > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", gold_reward);
    }
    if diamond_reward > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", diamond_reward);
    }

    // 更新通缉状态
    let mut completed = bounty.clone();
    completed.status = BountyStatus::Completed;
    save_bounty(db, &completed);

    // 更新猎人统计
    let hunter_key = format!("hunter_{}", user_id);
    let kills: i32 = db.global_get("wanted_hunter", &hunter_key).parse().unwrap_or(0);
    db.global_set("wanted_hunter", &hunter_key, &(kills + 1).to_string());
    let gold_earned_key = format!("hunter_gold_{}", user_id);
    let total_earned: i64 = db.global_get("wanted_hunter", &gold_earned_key).parse().unwrap_or(0);
    db.global_set(
        "wanted_hunter",
        &gold_earned_key,
        &(total_earned + gold_reward).to_string(),
    );

    // 更新全服统计
    let total_completed: i32 = db.global_get("wanted_stats", "total_completed").parse().unwrap_or(0);
    db.global_set("wanted_stats", "total_completed", &(total_completed + 1).to_string());

    // 清除PvP记录
    db.global_set("wanted_pvp", &pvp_key, "");

    let mut r = format!("{}\n💰 赏金领取成功！\n", prefix);
    r.push_str("━━━━━━━━━━━━━━\n");
    if gold_reward > 0 {
        r.push_str(&format!(
            "🪙 获得: {} 金币 (佣金{}%)\n",
            format_gold(gold_reward),
            (HUNTER_COMMISSION_RATIO * 100.0) as i32
        ));
    }
    if diamond_reward > 0 {
        r.push_str(&format!("💎 获得: {} 钻石\n", diamond_reward));
    }
    r.push_str(&format!("🎯 目标: {}\n", bounty.target_name));
    r.push_str("━━━━━━━━━━━━━━\n");
    r.push_str("优秀的猎人！继续保持！");

    r
}

/// 通缉详情
pub fn cmd_wanted_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_name = args.trim();
    if target_name.is_empty() {
        return format!("{}\n用法: 通缉详情 目标名", prefix);
    }

    let bounties = load_active_bounties(db);
    let bounty = bounties.iter().find(|b| {
        b.target_name == target_name && (b.status == BountyStatus::Active || b.status == BountyStatus::Accepted)
    });

    if bounty.is_none() {
        // 也检查已完成的
        let completed = bounties.iter().find(|b| b.target_name == target_name);
        if completed.is_none() {
            return format!("{}\n❌ 未找到关于「{}」的通缉记录。", prefix, target_name);
        }
        let b = completed.unwrap();
        let mut r = format!("{}\n═══ 📋 通缉详情 ═══\n", prefix);
        r.push_str(&format!("🎯 目标: {}\n", b.target_name));
        r.push_str(&format!("📊 状态: {} ✅\n", b.status.as_str()));
        r.push_str(&format!("💰 赏金: {} 金币\n", format_gold(b.reward_gold)));
        if !b.hunter.is_empty() {
            let hunter_name = user::get_msg_prefix(db, &b.hunter);
            r.push_str(&format!("🗡️ 猎人: {}\n", hunter_name));
        }
        return r;
    }

    let b = bounty.unwrap();
    let mut r = format!("{}\n═══ 📋 通缉详情 ═══\n", prefix);
    r.push_str(&format!("🎯 目标: {}\n", b.target_name));
    r.push_str(&format!("🔴 状态: {}\n", b.status.as_str()));
    r.push_str(&format!("💰 赏金: {} 金币", format_gold(b.reward_gold)));
    if b.reward_diamond > 0 {
        r.push_str(&format!(" + {}💎", b.reward_diamond));
    }
    r.push('\n');
    r.push_str(&format!("📝 原因: {}\n", b.reason));
    r.push_str(&format!("😈 邪恶值: {}\n", b.evil_value));
    r.push_str(&format!("⏰ 发布: {}\n", b.created_at));
    r.push_str(&format!("⏳ 过期: {}\n", b.expire_at));

    let poster_name = user::get_msg_prefix(db, &b.poster);
    r.push_str(&format!("📢 发布者: {}\n", poster_name));

    if !b.hunter.is_empty() {
        let hunter_name = user::get_msg_prefix(db, &b.hunter);
        r.push_str(&format!("🗡️ 猎人: {} (追踪中)\n", hunter_name));
    }

    r
}

/// 猎人排行
pub fn cmd_wanted_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut hunters: Vec<(String, i32, i64)> = Vec::new(); // (name, kills, gold_earned)

    // 从Global表获取所有猎人数据
    let sections: Vec<String> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT DISTINCT ID FROM Global WHERE SECTION = 'wanted_hunter' AND ID LIKE 'hunter_%' AND ID NOT LIKE 'hunter\\_gold\\_%' ESCAPE '\\'"
        ) {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    result.push(row);
                }
            }
        }
        result
    };

    for section_id in &sections {
        // section_id is like "hunter_123456"
        if let Some(uid) = section_id.strip_prefix("hunter_") {
            if uid.is_empty() {
                continue;
            }
            let kills: i32 = db.global_get("wanted_hunter", section_id).parse().unwrap_or(0);
            if kills > 0 {
                let gold_key = format!("hunter_gold_{}", uid);
                let gold: i64 = db.global_get("wanted_hunter", &gold_key).parse().unwrap_or(0);
                let name = user::get_msg_prefix(db, uid);
                hunters.push((name, kills, gold));
            }
        }
    }

    if hunters.is_empty() {
        return format!("{}\n📋 暂无猎人数据！\n成为第一个通缉猎人吧！", prefix);
    }

    hunters.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut r = format!("{}\n═══ 🗡️ 通缉猎人排行 ═══\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, kills, gold)) in hunters.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        r.push_str(&format!(
            "\n{} {} — 击杀:{} 累计赏金:{}",
            medal,
            name,
            kills,
            format_gold(*gold)
        ));
    }

    // 用户位置
    let my_name = user::get_msg_prefix(db, user_id);
    if let Some(rank) = hunters.iter().position(|(name, _, _)| name == &my_name) {
        r.push_str(&format!("\n\n📍 你的排名: 第{}名", rank + 1));
    }

    let total_completed: i32 = db.global_get("wanted_stats", "total_completed").parse().unwrap_or(0);
    let total_posted: i32 = db.global_get("wanted_stats", "total_posted").parse().unwrap_or(0);
    r.push_str(&format!(
        "\n\n📊 全服统计: 已发布{}次 已完成{}次",
        total_posted, total_completed
    ));

    r
}

/// 我的通缉信息
pub fn cmd_my_wanted(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let bounties = load_active_bounties(db);

    // 我发布的通缉
    let my_posted: Vec<&Bounty> = bounties.iter().filter(|b| b.poster == user_id).collect();

    // 我接受的通缉
    let my_hunting: Vec<&Bounty> = bounties.iter().filter(|b| b.hunter == user_id).collect();

    // 我被通缉的情况
    let targeted_me: Vec<&Bounty> = bounties
        .iter()
        .filter(|b| b.target == user_id && (b.status == BountyStatus::Active || b.status == BountyStatus::Accepted))
        .collect();

    let mut r = format!("{}\n═══ 📋 我的通缉信息 ═══\n", prefix);

    // 被通缉
    if !targeted_me.is_empty() {
        r.push_str(&format!("\n⚠️ 你正被通缉！({}条)", targeted_me.len()));
        for b in &targeted_me {
            r.push_str(&format!(
                "\n  💰 {}金币 | 原因: {}",
                format_gold(b.reward_gold),
                b.reason
            ));
            if !b.hunter.is_empty() {
                let hunter_name = user::get_msg_prefix(db, &b.hunter);
                r.push_str(&format!(" | 🗡️猎人: {}", hunter_name));
            }
        }
    }

    // 我接受的
    if !my_hunting.is_empty() {
        r.push_str(&format!("\n\n🗡️ 正在追踪: {}条", my_hunting.len()));
        for b in &my_hunting {
            r.push_str(&format!(
                "\n  🎯 {} — 💰{} | 使用「领取赏金+{}」领取",
                b.target_name,
                format_gold(b.reward_gold),
                b.target_name
            ));
        }
    }

    // 我发布的
    if !my_posted.is_empty() {
        r.push_str(&format!("\n\n📢 我发布的通缉: {}条", my_posted.len()));
        for b in &my_posted {
            r.push_str(&format!(
                "\n  🎯 {} — 💰{} [{}]",
                b.target_name,
                format_gold(b.reward_gold),
                b.status.as_str()
            ));
        }
    }

    if targeted_me.is_empty() && my_hunting.is_empty() && my_posted.is_empty() {
        r.push_str("\n暂无通缉相关信息。");
    }

    r
}

/// 记录PvP胜利（从pvp模块调用）
#[allow(dead_code)]
pub fn record_pvp_win(db: &Database, winner_id: &str, loser_id: &str) {
    let pvp_key = format!("pvp_win_{}_{}", winner_id, loser_id);
    db.global_set("wanted_pvp", &pvp_key, &now_str());
}

/// 自动通缉高邪恶值玩家
#[allow(dead_code)]
pub fn auto_wanted_check(db: &Database) {
    let bounties = load_active_bounties(db);
    let active_count = bounties
        .iter()
        .filter(|b| b.status == BountyStatus::Active || b.status == BountyStatus::Accepted)
        .count();

    if active_count >= MAX_ACTIVE_BOUNTIES {
        return;
    }

    // 查找高邪恶值玩家
    let evil_players: Vec<(String, i32)> = {
        let conn = db.lock_conn();
        let mut result = Vec::new();
        if let Ok(mut stmt) =
            conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION = 'xiaoheiwu_evil' AND DATA != ''")
        {
            if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
                for row in rows.flatten() {
                    let (id, evil_str) = row;
                    // ID 格式: "evil_{user_id}"
                    if let Some(uid) = id.strip_prefix("evil_") {
                        let evil: i32 = evil_str.parse().unwrap_or(0);
                        if evil >= AUTO_WANTED_EVIL_THRESHOLD {
                            result.push((uid.to_string(), evil));
                        }
                    }
                }
            }
        }
        result
    };

    for (uid, evil) in &evil_players {
        // 检查是否已有活跃通缉
        let already_wanted = bounties
            .iter()
            .any(|b| b.target == *uid && (b.status == BountyStatus::Active || b.status == BountyStatus::Accepted));
        if already_wanted {
            continue;
        }

        // 自动发布系统通缉
        let reward = 5000 + (*evil as i64 - AUTO_WANTED_EVIL_THRESHOLD as i64) * 100;
        let target_name = user::get_msg_prefix(db, uid);

        let bounty = Bounty {
            id: bounty_id("system"),
            poster: "system".to_string(),
            target: uid.clone(),
            target_name,
            reward_gold: reward,
            reward_diamond: 0,
            reason: format!("邪恶值过高({})", evil),
            status: BountyStatus::Active,
            hunter: String::new(),
            created_at: now_str(),
            expire_at: expire_str(),
            evil_value: *evil,
        };
        save_bounty(db, &bounty);
    }
}

/// 查找用户ID（按昵称）
fn find_user_by_name(db: &Database, name: &str) -> Option<String> {
    let all_users = db.all_users();
    for uid in &all_users {
        let nickname = db.read_basic(uid, ITEM_NAME);
        if nickname == name {
            return Some(uid.clone());
        }
    }
    None
}

/// 格式化金币显示
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounty_status_roundtrip() {
        for status in &[
            BountyStatus::Active,
            BountyStatus::Accepted,
            BountyStatus::Completed,
            BountyStatus::Expired,
            BountyStatus::Cancelled,
        ] {
            let s = status.as_str();
            let back = BountyStatus::from_str(s);
            assert_eq!(*status, back);
        }
    }

    #[test]
    fn test_bounty_json_roundtrip() {
        let b = Bounty {
            id: "bounty_test".to_string(),
            poster: "123".to_string(),
            target: "456".to_string(),
            target_name: "测试玩家".to_string(),
            reward_gold: 10000,
            reward_diamond: 10,
            reason: "测试原因".to_string(),
            status: BountyStatus::Active,
            hunter: String::new(),
            created_at: "2026-06-12 22:00:00".to_string(),
            expire_at: "2026-06-13 22:00:00".to_string(),
            evil_value: 75,
        };
        let json = b.to_json();
        let parsed = Bounty::from_json("bounty_test", &json).unwrap();
        assert_eq!(parsed.poster, "123");
        assert_eq!(parsed.target, "456");
        assert_eq!(parsed.reward_gold, 10000);
        assert_eq!(parsed.reward_diamond, 10);
        assert_eq!(parsed.reason, "测试原因");
        assert_eq!(parsed.status, BountyStatus::Active);
        assert_eq!(parsed.evil_value, 75);
    }

    #[test]
    fn test_bounty_json_completed() {
        let b = Bounty {
            id: "bounty_done".to_string(),
            poster: "system".to_string(),
            target: "789".to_string(),
            target_name: "通缉犯".to_string(),
            reward_gold: 50000,
            reward_diamond: 0,
            reason: "邪恶值过高".to_string(),
            status: BountyStatus::Completed,
            hunter: "100".to_string(),
            created_at: "2026-06-12 10:00:00".to_string(),
            expire_at: "2026-06-13 10:00:00".to_string(),
            evil_value: 120,
        };
        let json = b.to_json();
        let parsed = Bounty::from_json("bounty_done", &json).unwrap();
        assert_eq!(parsed.status, BountyStatus::Completed);
        assert_eq!(parsed.hunter, "100");
        assert_eq!(parsed.target_name, "通缉犯");
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(999), "999");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(1000000), "1,000,000");
        assert_eq!(format_gold(50000), "50,000");
    }

    #[test]
    fn test_constants() {
        assert!(MIN_BOUNTY_GOLD >= 1000);
        assert!(MIN_BOUNTY_DIAMOND >= 1);
        assert!(BOUNTY_DURATION_HOURS >= 1);
        assert!(AUTO_WANTED_EVIL_THRESHOLD >= 10);
        assert!(HUNTER_COMMISSION_RATIO > 0.0 && HUNTER_COMMISSION_RATIO <= 1.0);
        assert!(MAX_ACTIVE_BOUNTIES >= 5);
        assert!(MAX_PER_USER_BOUNTIES >= 1);
    }

    #[test]
    fn test_bounty_id_unique() {
        // Different users produce different IDs even in same second
        let id1 = bounty_id("user1");
        let id2 = bounty_id("user2");
        assert_ne!(id1, id2);
        assert!(id1.starts_with("bounty_"));
        assert!(id2.starts_with("bounty_"));
    }

    #[test]
    fn test_wanted_status_display() {
        assert_eq!(BountyStatus::Active.as_str(), "活跃");
        assert_eq!(BountyStatus::Accepted.as_str(), "追踪中");
        assert_eq!(BountyStatus::Completed.as_str(), "已完成");
        assert_eq!(BountyStatus::Expired.as_str(), "已过期");
        assert_eq!(BountyStatus::Cancelled.as_str(), "已取消");
    }

    #[test]
    fn test_wanted_status_unknown() {
        // Unknown status defaults to Active
        assert_eq!(BountyStatus::from_str("unknown"), BountyStatus::Active);
        assert_eq!(BountyStatus::from_str(""), BountyStatus::Active);
    }

    #[test]
    fn test_hunter_commission() {
        let gold: i64 = 10000;
        let reward = (gold as f64 * HUNTER_COMMISSION_RATIO) as i64;
        assert_eq!(reward, 8000); // 80%
    }

    #[test]
    fn test_evil_threshold_sensible() {
        // Evil threshold should be > 0 and reasonable
        assert!(AUTO_WANTED_EVIL_THRESHOLD > 0);
        assert!(AUTO_WANTED_EVIL_THRESHOLD <= 100);
    }
}

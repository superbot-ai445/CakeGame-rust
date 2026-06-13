/// CakeGame NPC 好感度系统
/// 与NPC互动提升好感度，解锁特殊对话和奖励
/// 数据存储: Shared_Data 表, SECTION='NpcAffinity'
///
/// 功能:
/// - 查看NPC好感度列表
/// - 与NPC对话提升好感度（每日上限）
/// - 赠送礼物提升好感度
/// - 好感度等级: 陌生→点头之交→熟悉→友好→亲密→挚友→灵魂伴侣
/// - 好感度里程碑奖励
/// - 好感度排行
use crate::db::Database;
use rusqlite::params;

const AFFINITY_SECTION: &str = "NpcAffinity";
const DAILY_TALK_LIMIT: i32 = 5;
const TALK_AFFINITY_GAIN: i32 = 2;
const GIFT_AFFINITY_GAIN: i32 = 5;
const DAILY_MAX_AFFINITY: i32 = 20;

/// 好感度等级定义
pub struct AffinityLevel {
    pub name: &'static str,
    pub threshold: i32,
    pub emoji: &'static str,
    pub bonus_desc: &'static str,
}

pub const AFFINITY_LEVELS: &[AffinityLevel] = &[
    AffinityLevel {
        name: "陌生",
        threshold: 0,
        emoji: "😶",
        bonus_desc: "无加成",
    },
    AffinityLevel {
        name: "点头之交",
        threshold: 20,
        emoji: "🙂",
        bonus_desc: "NPC对话额外+1金币",
    },
    AffinityLevel {
        name: "熟悉",
        threshold: 60,
        emoji: "😊",
        bonus_desc: "商店折扣5%",
    },
    AffinityLevel {
        name: "友好",
        threshold: 120,
        emoji: "😄",
        bonus_desc: "商店折扣10%+特殊任务",
    },
    AffinityLevel {
        name: "亲密",
        threshold: 200,
        emoji: "🥰",
        bonus_desc: "商店折扣15%+专属物品",
    },
    AffinityLevel {
        name: "挚友",
        threshold: 350,
        emoji: "💛",
        bonus_desc: "商店折扣20%+隐藏功能",
    },
    AffinityLevel {
        name: "灵魂伴侣",
        threshold: 500,
        emoji: "🌟",
        bonus_desc: "商店折扣30%+终极奖励",
    },
];

/// 里程碑奖励定义
struct MilestoneReward {
    threshold: i32,
    reward_desc: &'static str,
    gold_reward: i64,
    diamond_reward: i32,
}

const MILESTONES: &[MilestoneReward] = &[
    MilestoneReward {
        threshold: 20,
        reward_desc: "新手见面礼",
        gold_reward: 500,
        diamond_reward: 5,
    },
    MilestoneReward {
        threshold: 60,
        reward_desc: "友谊见证",
        gold_reward: 2000,
        diamond_reward: 15,
    },
    MilestoneReward {
        threshold: 120,
        reward_desc: "信赖之礼",
        gold_reward: 5000,
        diamond_reward: 30,
    },
    MilestoneReward {
        threshold: 200,
        reward_desc: "亲密馈赠",
        gold_reward: 10000,
        diamond_reward: 50,
    },
    MilestoneReward {
        threshold: 350,
        reward_desc: "挚友之心",
        gold_reward: 25000,
        diamond_reward: 100,
    },
    MilestoneReward {
        threshold: 500,
        reward_desc: "灵魂共鸣",
        gold_reward: 50000,
        diamond_reward: 200,
    },
];

/// 好感度数据
#[derive(Debug, Clone)]
pub struct NpcAffinityData {
    pub npc_name: String,
    pub affinity: i32,
    pub daily_talks: i32,
    pub daily_gifts: i32,
    pub daily_affinity_gained: i32,
    pub total_gifts: i32,
    pub total_talks: i32,
    pub milestones_claimed: String, // comma-separated thresholds
    pub last_interact_date: String,
}

impl NpcAffinityData {
    fn new(npc_name: &str) -> Self {
        Self {
            npc_name: npc_name.to_string(),
            affinity: 0,
            daily_talks: 0,
            daily_gifts: 0,
            daily_affinity_gained: 0,
            total_gifts: 0,
            total_talks: 0,
            milestones_claimed: String::new(),
            last_interact_date: String::new(),
        }
    }

    fn serialize(&self) -> String {
        format!(
            "affinity={}|daily_talks={}|daily_gifts={}|daily_gain={}|total_gifts={}|total_talks={}|milestones={}|last_date={}",
            self.affinity, self.daily_talks, self.daily_gifts, self.daily_affinity_gained,
            self.total_gifts, self.total_talks, self.milestones_claimed, self.last_interact_date
        )
    }

    fn deserialize(data: &str, npc_name: &str) -> Self {
        let mut result = Self::new(npc_name);
        for part in data.split('|') {
            if let Some((k, v)) = part.split_once('=') {
                match k.trim() {
                    "affinity" => result.affinity = v.trim().parse().unwrap_or(0),
                    "daily_talks" => result.daily_talks = v.trim().parse().unwrap_or(0),
                    "daily_gifts" => result.daily_gifts = v.trim().parse().unwrap_or(0),
                    "daily_gain" => result.daily_affinity_gained = v.trim().parse().unwrap_or(0),
                    "total_gifts" => result.total_gifts = v.trim().parse().unwrap_or(0),
                    "total_talks" => result.total_talks = v.trim().parse().unwrap_or(0),
                    "milestones" => result.milestones_claimed = v.trim().to_string(),
                    "last_date" => result.last_interact_date = v.trim().to_string(),
                    _ => {}
                }
            }
        }
        result
    }
}

/// 获取好感度等级信息
pub fn get_affinity_level(affinity: i32) -> &'static AffinityLevel {
    let mut result = &AFFINITY_LEVELS[0];
    for level in AFFINITY_LEVELS {
        if affinity >= level.threshold {
            result = level;
        }
    }
    result
}

/// 获取下一级所需好感度
pub fn get_next_level_info(affinity: i32) -> Option<(&'static str, i32)> {
    for level in AFFINITY_LEVELS {
        if affinity < level.threshold {
            return Some((level.name, level.threshold));
        }
    }
    None
}

/// 从数据库读取好感度数据
fn load_affinity(db: &Database, user_id: &str, npc_name: &str) -> NpcAffinityData {
    let key = format!("{}.{}", user_id, npc_name);
    let conn = db.lock_conn();
    if let Ok(mut stmt) = conn.prepare("SELECT DATA FROM Shared_Data WHERE ID=?1 AND SECTION=?2") {
        if let Ok(data) = stmt.query_row(params![key, AFFINITY_SECTION], |row| {
            Ok(row.get::<_, String>(0).unwrap_or_default())
        }) {
            return NpcAffinityData::deserialize(&data, npc_name);
        }
    }
    NpcAffinityData::new(npc_name)
}

/// 保存好感度数据
fn save_affinity(db: &Database, user_id: &str, data: &NpcAffinityData) {
    let key = format!("{}.{}", user_id, data.npc_name);
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Shared_Data (ID, SECTION, DATA) VALUES (?1, ?2, ?3)",
        params![key, AFFINITY_SECTION, data.serialize()],
    );
}

/// 获取今日日期字符串
fn today_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// 重置每日计数（如果日期变了）
fn reset_daily_if_needed(data: &mut NpcAffinityData) {
    let today = today_str();
    if data.last_interact_date != today {
        data.daily_talks = 0;
        data.daily_gifts = 0;
        data.daily_affinity_gained = 0;
        data.last_interact_date = today;
    }
}

/// 查看所有NPC好感度
pub fn cmd_view_npc_affinity(db: &Database, user_id: &str) -> String {
    let conn = db.lock_conn();
    let npcs: Vec<String> = if let Ok(mut stmt) = conn.prepare("SELECT Name FROM Ext_NPC_Info") {
        if let Ok(rows) = stmt.query_map([], |row| Ok(row.get::<_, String>(0).unwrap_or_default())) {
            rows.filter_map(|r| r.ok()).collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    if npcs.is_empty() {
        return "📜 当前没有可互动的NPC。".to_string();
    }

    drop(conn); // release lock before loading affinities

    let mut result = String::from("📜 === NPC好感度 ===\n\n");
    for npc_name in &npcs {
        let data = load_affinity(db, user_id, npc_name);
        let level = get_affinity_level(data.affinity);
        let progress_bar = make_progress_bar(data.affinity, 500);
        result.push_str(&format!(
            "{} {} {} Lv.{}\n   好感度: {} {} | 对话{}次 | 赠礼{}次\n",
            level.emoji,
            npc_name,
            level.name,
            get_level_index(data.affinity),
            data.affinity,
            progress_bar,
            data.total_talks,
            data.total_gifts
        ));
        if let Some((next_name, next_threshold)) = get_next_level_info(data.affinity) {
            result.push_str(&format!(
                "   ⏳ 升级到「{}」还需 {} 点好感度\n",
                next_name,
                next_threshold - data.affinity
            ));
        }
        result.push('\n');
    }
    result.push_str("💡 说明: 与NPC对话(+2/次)或赠送礼物(+5/次)提升好感度\n");
    result.push_str(&format!(
        "📊 每日好感度上限: {}点 | 对话上限: {}次\n",
        DAILY_MAX_AFFINITY, DAILY_TALK_LIMIT
    ));
    result.push_str("📌 指令: NPC对话 [NPC名] | NPC赠礼 [NPC名]\n");
    result.push_str("📌 指令: NPC好感奖励 [NPC名] | NPC好感排行");
    result
}

/// 与NPC对话提升好感度
pub fn cmd_npc_talk(db: &Database, user_id: &str, npc_name: &str) -> String {
    // 验证NPC存在
    let conn = db.lock_conn();
    let npc_exists = conn
        .prepare("SELECT Name FROM Ext_NPC_Info WHERE Name=?1")
        .ok()
        .and_then(|mut s| s.query_row(params![npc_name], |_| Ok(())).ok())
        .is_some();
    let dialog: String = if npc_exists {
        conn.prepare("SELECT Dialog FROM Ext_NPC_Info WHERE Name=?1")
            .ok()
            .and_then(|mut s| s.query_row(params![npc_name], |row| row.get(0)).ok())
            .unwrap_or_default()
    } else {
        String::new()
    };
    drop(conn);

    if !npc_exists {
        return format!("❌ 找不到NPC「{}」。请使用「查看NPC」确认NPC名称。", npc_name);
    }

    let mut data = load_affinity(db, user_id, npc_name);
    reset_daily_if_needed(&mut data);

    // 检查每日对话上限
    if data.daily_talks >= DAILY_TALK_LIMIT {
        return format!(
            "😶 你今天已经和「{}」聊了很多了！\n每日对话上限: {}次\n明天再来吧~",
            npc_name, DAILY_TALK_LIMIT
        );
    }

    // 检查每日好感度上限
    if data.daily_affinity_gained >= DAILY_MAX_AFFINITY {
        return format!(
            "📊 你今天的好感度已达上限！\n每日好感度上限: {}点\n{} {} ({})",
            DAILY_MAX_AFFINITY,
            get_affinity_level(data.affinity).emoji,
            npc_name,
            get_affinity_level(data.affinity).name
        );
    }

    // 计算实际好感度增益
    let actual_gain = TALK_AFFINITY_GAIN.min(DAILY_MAX_AFFINITY - data.daily_affinity_gained);
    let old_level = get_affinity_level(data.affinity);
    data.affinity += actual_gain;
    data.daily_talks += 1;
    data.daily_affinity_gained += actual_gain;
    data.total_talks += 1;
    data.last_interact_date = today_str();
    let new_level = get_affinity_level(data.affinity);

    save_affinity(db, user_id, &data);

    let mut result = format!(
        "💬 你和「{}」聊了会天\n📜 「{}」\n❤️ 好感度 +{} (当前: {})\n",
        npc_name,
        dialog.lines().next().unwrap_or("..."),
        actual_gain,
        data.affinity
    );

    // 升级提示
    if new_level.name != old_level.name {
        result.push_str(&format!(
            "\n🎉 好感度升级！{} {} → {} {}\n{}\n",
            old_level.emoji, old_level.name, new_level.emoji, new_level.name, new_level.bonus_desc
        ));
    }

    result.push_str(&format!(
        "\n📊 今日: 对话 {}/{}次 | 好感度 {}/{}点",
        data.daily_talks, DAILY_TALK_LIMIT, data.daily_affinity_gained, DAILY_MAX_AFFINITY
    ));

    result
}

/// 赠送礼物给NPC提升好感度
pub fn cmd_npc_gift(db: &Database, user_id: &str, npc_name: &str) -> String {
    // 验证NPC存在
    let conn = db.lock_conn();
    let npc_exists = conn
        .prepare("SELECT Name FROM Ext_NPC_Info WHERE Name=?1")
        .ok()
        .and_then(|mut s| s.query_row(params![npc_name], |_| Ok(())).ok())
        .is_some();
    drop(conn);

    if !npc_exists {
        return format!("❌ 找不到NPC「{}」。请使用「查看NPC」确认NPC名称。", npc_name);
    }

    let mut data = load_affinity(db, user_id, npc_name);
    reset_daily_if_needed(&mut data);

    // 检查每日好感度上限
    if data.daily_affinity_gained >= DAILY_MAX_AFFINITY {
        return format!(
            "📊 你今天对「{}」的好感度已达上限！\n每日好感度上限: {}点",
            npc_name, DAILY_MAX_AFFINITY
        );
    }

    // 赠送礼物需要金币
    let conn = db.lock_conn();
    let gold: i64 = conn
        .prepare("SELECT Gold FROM Basic_User WHERE ID=?1")
        .ok()
        .and_then(|mut s| s.query_row(params![user_id], |row| row.get(0)).ok())
        .unwrap_or(0);
    drop(conn);

    let gift_cost: i64 = 500;
    if gold < gift_cost {
        return format!("💰 赠送礼物需要{}金币，你只有{}金币！", gift_cost, gold);
    }

    // 扣除金币
    let conn = db.lock_conn();
    let _ = conn.execute(
        "UPDATE Basic_User SET Gold=Gold-?1 WHERE ID=?2",
        params![gift_cost, user_id],
    );
    drop(conn);

    // 增加好感度
    let actual_gain = GIFT_AFFINITY_GAIN.min(DAILY_MAX_AFFINITY - data.daily_affinity_gained);
    let old_level = get_affinity_level(data.affinity);
    data.affinity += actual_gain;
    data.daily_gifts += 1;
    data.daily_affinity_gained += actual_gain;
    data.total_gifts += 1;
    data.last_interact_date = today_str();
    let new_level = get_affinity_level(data.affinity);

    save_affinity(db, user_id, &data);

    let mut result = format!(
        "🎁 你赠送了礼物给「{}」！\n💰 消耗: {}金币\n❤️ 好感度 +{} (当前: {})\n",
        npc_name, gift_cost, actual_gain, data.affinity
    );

    // 升级提示
    if new_level.name != old_level.name {
        result.push_str(&format!(
            "\n🎉 好感度升级！{} {} → {} {}\n{}\n",
            old_level.emoji, old_level.name, new_level.emoji, new_level.name, new_level.bonus_desc
        ));
    }

    result.push_str(&format!(
        "\n📊 今日: 赠礼 {}次 | 好感度 {}/{}点",
        data.daily_gifts, data.daily_affinity_gained, DAILY_MAX_AFFINITY
    ));

    result
}

/// 查看NPC好感度详情
pub fn cmd_npc_affinity_detail(db: &Database, user_id: &str, npc_name: &str) -> String {
    let conn = db.lock_conn();
    let npc_info = conn
        .prepare("SELECT Name, Location, Introduce, Dialog FROM Ext_NPC_Info WHERE Name=?1")
        .ok()
        .and_then(|mut s| {
            s.query_row(params![npc_name], |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, String>(2).unwrap_or_default(),
                    row.get::<_, String>(3).unwrap_or_default(),
                ))
            })
            .ok()
        });
    drop(conn);

    let (name, location, introduce, dialog) = match npc_info {
        Some(info) => info,
        None => return format!("❌ 找不到NPC「{}」。", npc_name),
    };

    let mut data = load_affinity(db, user_id, npc_name);
    reset_daily_if_needed(&mut data);

    let level = get_affinity_level(data.affinity);
    let progress_bar = make_progress_bar(data.affinity, 500);
    let level_idx = get_level_index(data.affinity);
    let discount = get_shop_discount(data.affinity);

    let mut result = format!(
        "{} === {} === {}\n📍 位置: {}\n📖 简介: {}\n💬 台词: 「{}」\n\n",
        level.emoji,
        name,
        level.name,
        location,
        introduce,
        dialog.lines().next().unwrap_or("...")
    );

    result.push_str(&format!(
        "❤️ 好感度: {} {} (等级 {})\n{}\n",
        data.affinity, progress_bar, level_idx, progress_bar
    ));
    result.push_str(&format!("🏷️ 当前加成: {}\n", level.bonus_desc));
    result.push_str(&format!("🛒 商店折扣: {}%\n", discount));
    result.push_str(&format!(
        "📊 统计: 对话{}次 | 赠礼{}次\n",
        data.total_talks, data.total_gifts
    ));

    if let Some((next_name, next_threshold)) = get_next_level_info(data.affinity) {
        result.push_str(&format!(
            "\n⏳ 下一级: 「{}」 (还需{}点好感度)\n",
            next_name,
            next_threshold - data.affinity
        ));
    } else {
        result.push_str("\n🌟 已达到最高好感度等级！\n");
    }

    // 里程碑奖励
    result.push_str("\n🏆 里程碑奖励:\n");
    for milestone in MILESTONES {
        let claimed = data
            .milestones_claimed
            .split(',')
            .any(|s| s.trim() == milestone.threshold.to_string());
        let status = if claimed {
            "✅ 已领取".to_string()
        } else if data.affinity >= milestone.threshold {
            "🎁 可领取！(NPC好感奖励)".to_string()
        } else {
            format!("🔒 需要{}好感度", milestone.threshold)
        };
        result.push_str(&format!(
            "  {}点: {} ({}金+{}💎) {}\n",
            milestone.threshold, milestone.reward_desc, milestone.gold_reward, milestone.diamond_reward, status
        ));
    }

    result.push_str("\n📌 指令: NPC赠礼 [NPC名] | NPC对话 [NPC名]");
    result
}

/// 领取NPC好感度里程碑奖励
pub fn cmd_claim_npc_affinity_reward(db: &Database, user_id: &str, npc_name: &str) -> String {
    let conn = db.lock_conn();
    let npc_exists = conn
        .prepare("SELECT Name FROM Ext_NPC_Info WHERE Name=?1")
        .ok()
        .and_then(|mut s| s.query_row(params![npc_name], |_| Ok(())).ok())
        .is_some();
    drop(conn);

    if !npc_exists {
        return format!("❌ 找不到NPC「{}」。", npc_name);
    }

    let mut data = load_affinity(db, user_id, npc_name);
    reset_daily_if_needed(&mut data);

    let mut claimed_any = false;
    let mut total_gold: i64 = 0;
    let mut total_diamond: i32 = 0;
    let mut rewards_text = String::new();

    for milestone in MILESTONES {
        let already_claimed = data
            .milestones_claimed
            .split(',')
            .any(|s| s.trim() == milestone.threshold.to_string());
        if !already_claimed && data.affinity >= milestone.threshold {
            total_gold += milestone.gold_reward;
            total_diamond += milestone.diamond_reward;
            if !data.milestones_claimed.is_empty() {
                data.milestones_claimed.push(',');
            }
            data.milestones_claimed.push_str(&milestone.threshold.to_string());
            rewards_text.push_str(&format!(
                "  🏆 {}点里程碑: {} (+{}金 +{}💎)\n",
                milestone.threshold, milestone.reward_desc, milestone.gold_reward, milestone.diamond_reward
            ));
            claimed_any = true;
        }
    }

    if !claimed_any {
        return format!(
            "😅 「{}」没有可领取的里程碑奖励。\n当前好感度: {} ({})\n需要继续提升好感度！",
            npc_name,
            data.affinity,
            get_affinity_level(data.affinity).name
        );
    }

    save_affinity(db, user_id, &data);

    // 发放奖励
    let conn = db.lock_conn();
    let _ = conn.execute(
        "UPDATE Basic_User SET Gold=Gold+?1, Diamond=Diamond+?2 WHERE ID=?3",
        params![total_gold, total_diamond, user_id],
    );
    drop(conn);

    format!(
        "🎉 成功领取「{}」好感度里程碑奖励！\n\n{}\n💰 总计获得: {}金币 + {}💎\n❤️ 当前好感度: {} ({})",
        npc_name,
        rewards_text,
        total_gold,
        total_diamond,
        data.affinity,
        get_affinity_level(data.affinity).name
    )
}

/// NPC好感度排行
pub fn cmd_npc_affinity_ranking(db: &Database) -> String {
    let conn = db.lock_conn();
    let mut entries: Vec<(String, String, i32)> = Vec::new();

    if let Ok(mut stmt) =
        conn.prepare("SELECT ID, DATA FROM Shared_Data WHERE SECTION=?1 ORDER BY CAST(DATA AS INTEGER) DESC LIMIT 20")
    {
        if let Ok(rows) = stmt.query_map(params![AFFINITY_SECTION], |row| {
            let id: String = row.get(0)?;
            let data: String = row.get(1)?;
            Ok((id, data))
        }) {
            for row in rows.flatten() {
                let parts: Vec<&str> = row.0.split('.').collect();
                if parts.len() == 2 {
                    let npc_name = parts[1].to_string();
                    let parsed = NpcAffinityData::deserialize(&row.1, &npc_name);
                    entries.push((parts[0].to_string(), npc_name, parsed.affinity));
                }
            }
        }
    }

    if entries.is_empty() {
        return "📊 NPC好感度排行暂无数据。\n💡 快去和NPC互动提升好感度吧！".to_string();
    }

    // Sort by affinity descending
    entries.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut result = String::from("📊 === NPC好感度排行 ===\n\n");
    for (i, (user_id, npc_name, affinity)) in entries.iter().take(15).enumerate() {
        let level = get_affinity_level(*affinity);
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        result.push_str(&format!(
            "{} {}. {} × {} - {} {} ({})\n",
            medal,
            i + 1,
            user_id,
            npc_name,
            level.emoji,
            affinity,
            level.name
        ));
    }

    result
}

/// 获取好感度等级索引
fn get_level_index(affinity: i32) -> i32 {
    let mut idx = 0;
    for (i, level) in AFFINITY_LEVELS.iter().enumerate() {
        if affinity >= level.threshold {
            idx = i as i32;
        }
    }
    idx
}

/// 获取商店折扣百分比
pub fn get_shop_discount(affinity: i32) -> i32 {
    match affinity {
        0..=19 => 0,
        20..=59 => 5,
        60..=119 => 10,
        120..=199 => 15,
        200..=349 => 20,
        350..=499 => 25,
        _ => 30,
    }
}

/// 生成进度条
fn make_progress_bar(value: i32, max: i32) -> String {
    let pct = (value as f64 / max as f64).min(1.0);
    let filled = (pct * 10.0) as usize;
    let empty = 10 - filled;
    format!(
        "[{}{}] {}%",
        "█".repeat(filled),
        "░".repeat(empty),
        (pct * 100.0) as i32
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affinity_levels_sorted() {
        for i in 1..AFFINITY_LEVELS.len() {
            assert!(
                AFFINITY_LEVELS[i].threshold > AFFINITY_LEVELS[i - 1].threshold,
                "Affinity levels must be sorted ascending"
            );
        }
    }

    #[test]
    fn test_get_affinity_level_basic() {
        assert_eq!(get_affinity_level(0).name, "陌生");
        assert_eq!(get_affinity_level(19).name, "陌生");
        assert_eq!(get_affinity_level(20).name, "点头之交");
        assert_eq!(get_affinity_level(59).name, "点头之交");
        assert_eq!(get_affinity_level(60).name, "熟悉");
        assert_eq!(get_affinity_level(500).name, "灵魂伴侣");
        assert_eq!(get_affinity_level(999).name, "灵魂伴侣");
    }

    #[test]
    fn test_get_next_level_info() {
        assert_eq!(get_next_level_info(0), Some(("点头之交", 20)));
        assert_eq!(get_next_level_info(19), Some(("点头之交", 20)));
        assert_eq!(get_next_level_info(20), Some(("熟悉", 60)));
        assert_eq!(get_next_level_info(500), None);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let data = NpcAffinityData {
            npc_name: "测试NPC".to_string(),
            affinity: 150,
            daily_talks: 3,
            daily_gifts: 1,
            daily_affinity_gained: 11,
            total_gifts: 20,
            total_talks: 50,
            milestones_claimed: "20,60,120".to_string(),
            last_interact_date: "2026-06-12".to_string(),
        };
        let serialized = data.serialize();
        let deserialized = NpcAffinityData::deserialize(&serialized, "测试NPC");
        assert_eq!(deserialized.affinity, 150);
        assert_eq!(deserialized.daily_talks, 3);
        assert_eq!(deserialized.total_talks, 50);
        assert_eq!(deserialized.milestones_claimed, "20,60,120");
    }

    #[test]
    fn test_serialize_empty() {
        let data = NpcAffinityData::new("空NPC");
        let serialized = data.serialize();
        let deserialized = NpcAffinityData::deserialize(&serialized, "空NPC");
        assert_eq!(deserialized.affinity, 0);
        assert_eq!(deserialized.daily_talks, 0);
    }

    #[test]
    fn test_shop_discount() {
        assert_eq!(get_shop_discount(0), 0);
        assert_eq!(get_shop_discount(20), 5);
        assert_eq!(get_shop_discount(60), 10);
        assert_eq!(get_shop_discount(120), 15);
        assert_eq!(get_shop_discount(200), 20);
        assert_eq!(get_shop_discount(350), 25);
        assert_eq!(get_shop_discount(500), 30);
    }

    #[test]
    fn test_progress_bar() {
        let bar = make_progress_bar(0, 100);
        assert!(bar.contains("0%"));
        let bar = make_progress_bar(50, 100);
        assert!(bar.contains("50%"));
        let bar = make_progress_bar(100, 100);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_progress_bar_overflow() {
        let bar = make_progress_bar(200, 100);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_get_level_index() {
        assert_eq!(get_level_index(0), 0);
        assert_eq!(get_level_index(20), 1);
        assert_eq!(get_level_index(500), 6);
    }

    #[test]
    fn test_affinity_level_emojis_unique() {
        let emojis: Vec<&str> = AFFINITY_LEVELS.iter().map(|l| l.emoji).collect();
        let mut unique = emojis.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(emojis.len(), unique.len(), "Affinity level emojis should be unique");
    }

    #[test]
    fn test_milestone_thresholds_within_range() {
        for m in MILESTONES {
            assert!(m.threshold <= 500, "Milestone threshold {} exceeds max", m.threshold);
            assert!(m.gold_reward > 0, "Milestone gold reward must be positive");
            assert!(m.diamond_reward > 0, "Milestone diamond reward must be positive");
        }
    }

    #[test]
    fn test_milestone_rewards_escalate() {
        for i in 1..MILESTONES.len() {
            assert!(
                MILESTONES[i].gold_reward > MILESTONES[i - 1].gold_reward,
                "Gold rewards should escalate"
            );
            assert!(
                MILESTONES[i].diamond_reward > MILESTONES[i - 1].diamond_reward,
                "Diamond rewards should escalate"
            );
        }
    }

    #[test]
    fn test_daily_constants() {
        assert!(DAILY_TALK_LIMIT > 0);
        assert!(TALK_AFFINITY_GAIN > 0);
        assert!(GIFT_AFFINITY_GAIN > 0);
        assert!(DAILY_MAX_AFFINITY > 0);
        assert!(
            GIFT_AFFINITY_GAIN > TALK_AFFINITY_GAIN,
            "Gifts should give more than talking"
        );
    }

    #[test]
    fn test_reset_daily_new_date() {
        let mut data = NpcAffinityData {
            npc_name: "测试".to_string(),
            affinity: 50,
            daily_talks: 5,
            daily_gifts: 3,
            daily_affinity_gained: 20,
            total_gifts: 10,
            total_talks: 20,
            milestones_claimed: String::new(),
            last_interact_date: "2020-01-01".to_string(),
        };
        reset_daily_if_needed(&mut data);
        assert_eq!(data.daily_talks, 0);
        assert_eq!(data.daily_gifts, 0);
        assert_eq!(data.daily_affinity_gained, 0);
        assert_eq!(data.last_interact_date, today_str());
        // Total counts should not reset
        assert_eq!(data.total_talks, 20);
    }
}

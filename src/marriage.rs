#![allow(dead_code)]
/// CakeGame 婚礼伴侣系统
///
/// 玩家可以求婚、结婚、享受伴侣加成。
/// 10级婚姻等级，每级递增属性加成。
/// 伴侣亲密度系统：一起战斗/签到增加亲密度。
///
/// 指令: 求婚/回应求婚/查看伴侣/离婚/伴侣加成/伴侣排行
/// 数据存储: read_user_data/write_user_data
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, ITEM_NAME};
use crate::db::Database;

const SECTION: &str = "marriage"; // used as key prefix convention
const DATA_KEY: &str = "marriage_data";
const PROPOSAL_KEY: &str = "marriage_proposal";
const INCOMING_KEY: &str = "marriage_incoming";
const NOTIFY_KEY: &str = "marriage_notify";

/// 求婚戒指定义
struct RingDef {
    name: &'static str,
    emoji: &'static str,
    cost_gold: i64,
    cost_diamond: i64,
    intimacy_bonus: i32,
    desc: &'static str,
}

const RINGS: &[RingDef] = &[
    RingDef {
        name: "铜戒指",
        emoji: "💍",
        cost_gold: 5000,
        cost_diamond: 0,
        intimacy_bonus: 10,
        desc: "朴实无华的铜戒指，代表真心",
    },
    RingDef {
        name: "银戒指",
        emoji: "💎",
        cost_gold: 20000,
        cost_diamond: 10,
        intimacy_bonus: 30,
        desc: "闪亮的银戒指，代表承诺",
    },
    RingDef {
        name: "金戒指",
        emoji: "👑",
        cost_gold: 50000,
        cost_diamond: 50,
        intimacy_bonus: 60,
        desc: "华贵的金戒指，代表永恒",
    },
    RingDef {
        name: "钻石戒指",
        emoji: "🌟",
        cost_gold: 100000,
        cost_diamond: 200,
        intimacy_bonus: 100,
        desc: "璀璨的钻石戒指，代表不朽的爱",
    },
];

/// 婚姻等级定义
struct MarriageLevel {
    level: u32,
    name: &'static str,
    emoji: &'static str,
    intimacy_required: i32,
    bonus_hp_pct: i32,
    bonus_ad_pct: i32,
    bonus_ap_pct: i32,
    bonus_exp_pct: i32,
    bonus_gold_pct: i32,
    bonus_def_pct: i32,
}

const MARRIAGE_LEVELS: &[MarriageLevel] = &[
    MarriageLevel {
        level: 1,
        name: "新婚燕尔",
        emoji: "💒",
        intimacy_required: 0,
        bonus_hp_pct: 2,
        bonus_ad_pct: 1,
        bonus_ap_pct: 1,
        bonus_exp_pct: 2,
        bonus_gold_pct: 1,
        bonus_def_pct: 1,
    },
    MarriageLevel {
        level: 2,
        name: "琴瑟和鸣",
        emoji: "🎵",
        intimacy_required: 50,
        bonus_hp_pct: 4,
        bonus_ad_pct: 2,
        bonus_ap_pct: 2,
        bonus_exp_pct: 4,
        bonus_gold_pct: 2,
        bonus_def_pct: 2,
    },
    MarriageLevel {
        level: 3,
        name: "如胶似漆",
        emoji: "💕",
        intimacy_required: 150,
        bonus_hp_pct: 6,
        bonus_ad_pct: 3,
        bonus_ap_pct: 3,
        bonus_exp_pct: 6,
        bonus_gold_pct: 3,
        bonus_def_pct: 3,
    },
    MarriageLevel {
        level: 4,
        name: "举案齐眉",
        emoji: "🌺",
        intimacy_required: 300,
        bonus_hp_pct: 8,
        bonus_ad_pct: 5,
        bonus_ap_pct: 5,
        bonus_exp_pct: 8,
        bonus_gold_pct: 5,
        bonus_def_pct: 5,
    },
    MarriageLevel {
        level: 5,
        name: "相濡以沫",
        emoji: "🦢",
        intimacy_required: 500,
        bonus_hp_pct: 10,
        bonus_ad_pct: 7,
        bonus_ap_pct: 7,
        bonus_exp_pct: 10,
        bonus_gold_pct: 7,
        bonus_def_pct: 7,
    },
    MarriageLevel {
        level: 6,
        name: "心有灵犀",
        emoji: "💫",
        intimacy_required: 800,
        bonus_hp_pct: 13,
        bonus_ad_pct: 9,
        bonus_ap_pct: 9,
        bonus_exp_pct: 13,
        bonus_gold_pct: 9,
        bonus_def_pct: 9,
    },
    MarriageLevel {
        level: 7,
        name: "比翼双飞",
        emoji: "🕊️",
        intimacy_required: 1200,
        bonus_hp_pct: 16,
        bonus_ad_pct: 11,
        bonus_ap_pct: 11,
        bonus_exp_pct: 16,
        bonus_gold_pct: 11,
        bonus_def_pct: 11,
    },
    MarriageLevel {
        level: 8,
        name: "天作之合",
        emoji: "🌈",
        intimacy_required: 1800,
        bonus_hp_pct: 20,
        bonus_ad_pct: 14,
        bonus_ap_pct: 14,
        bonus_exp_pct: 20,
        bonus_gold_pct: 14,
        bonus_def_pct: 14,
    },
    MarriageLevel {
        level: 9,
        name: "海誓山盟",
        emoji: "🏔️",
        intimacy_required: 2500,
        bonus_hp_pct: 25,
        bonus_ad_pct: 17,
        bonus_ap_pct: 17,
        bonus_exp_pct: 25,
        bonus_gold_pct: 17,
        bonus_def_pct: 17,
    },
    MarriageLevel {
        level: 10,
        name: "神仙眷侣",
        emoji: "✨",
        intimacy_required: 3500,
        bonus_hp_pct: 30,
        bonus_ad_pct: 20,
        bonus_ap_pct: 20,
        bonus_exp_pct: 30,
        bonus_gold_pct: 20,
        bonus_def_pct: 20,
    },
];

/// 求婚冷却(秒): 24小时
const PROPOSE_COOLDOWN_SECS: u64 = 86400;
/// 每日亲密度上限
const DAILY_INTIMACY_CAP: i32 = 100;
/// 签到亲密度
const SIGN_IN_INTIMACY: i32 = 5;
/// 攻击亲密度(每次)
const ATTACK_INTIMACY: i32 = 1;
/// 赠送亲密度(每次)
const GIFT_INTIMACY: i32 = 3;

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn today_str() -> String {
    let secs = now_ts();
    let days = secs / 86400;
    let y = 1970 + days / 365;
    let d = days % 365;
    format!("{:04}-{:03}", y, d)
}

/// 婚姻数据
struct MarriageData {
    partner_id: String,
    partner_name: String,
    intimacy: i32,
    marry_time: u64,
    ring_name: String,
    ring_emoji: String,
    today_intimacy: i32,
    today_date: String,
    total_sign_in: i32,
    total_attacks: i32,
}

impl MarriageData {
    fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(10, '|').collect();
        if parts.len() < 6 {
            return None;
        }
        Some(Self {
            partner_id: parts[0].to_string(),
            partner_name: parts[1].to_string(),
            intimacy: parts[2].parse().unwrap_or(0),
            marry_time: parts[3].parse().unwrap_or(0),
            ring_name: parts[4].to_string(),
            ring_emoji: parts[5].to_string(),
            today_intimacy: parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
            today_date: parts.get(7).map(|s| s.to_string()).unwrap_or_default(),
            total_sign_in: parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0),
            total_attacks: parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(0),
        })
    }

    fn serialize(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.partner_id,
            self.partner_name,
            self.intimacy,
            self.marry_time,
            self.ring_name,
            self.ring_emoji,
            self.today_intimacy,
            self.today_date,
            self.total_sign_in,
            self.total_attacks,
        )
    }
}

/// 获取婚姻等级
fn get_marriage_level(intimacy: i32) -> &'static MarriageLevel {
    let mut result = &MARRIAGE_LEVELS[0];
    for lvl in MARRIAGE_LEVELS {
        if intimacy >= lvl.intimacy_required {
            result = lvl;
        }
    }
    result
}

/// 获取下一级婚姻等级
fn get_next_marriage_level(intimacy: i32) -> Option<&'static MarriageLevel> {
    MARRIAGE_LEVELS.iter().find(|lvl| intimacy < lvl.intimacy_required)
}

/// 进度条
fn progress_bar(current: i32, max: i32, width: usize) -> String {
    let pct = if max > 0 {
        (current as f64 / max as f64).min(1.0)
    } else {
        1.0
    };
    let filled = (pct * width as f64) as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{} {:.0}%", "█".repeat(filled), "░".repeat(empty), pct * 100.0,)
}

/// 格式化数字(千分位)
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

/// 根据名称查找用户ID
fn find_user_by_name(db: &Database, name: &str) -> Option<String> {
    let all = db.all_users();
    for uid in &all {
        let n = db.read_basic(uid, ITEM_NAME);
        if n == name {
            return Some(uid.clone());
        }
    }
    None
}

/// 解析玩家ID
fn resolve_player(db: &Database, input: &str) -> Option<String> {
    if db.user_exists(input) {
        return Some(input.to_string());
    }
    find_user_by_name(db, input)
}

/// 读取婚姻数据
fn read_marriage(db: &Database, user_id: &str) -> Option<MarriageData> {
    let raw = db.read_user_data(user_id, DATA_KEY);
    if raw.is_empty() {
        return None;
    }
    MarriageData::parse(&raw)
}

/// 写入婚姻数据
fn write_marriage(db: &Database, user_id: &str, data: &MarriageData) {
    db.write_user_data(user_id, DATA_KEY, &data.serialize());
}

// ============================================================
// 指令实现
// ============================================================

/// 求婚
pub fn cmd_propose(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let target = args.trim();
    if target.is_empty() {
        let mut lines: Vec<String> = vec![
            "💍 求婚系统".to_string(),
            String::new(),
            "用法: 求婚 +玩家ID/昵称 +戒指序号".to_string(),
            "示例: 求婚 123456 2".to_string(),
            String::new(),
            "可用戒指:".to_string(),
        ];
        for (i, ring) in RINGS.iter().enumerate() {
            lines.push(format!(
                "  {}. {} {} — {}金币{} ({})",
                i + 1,
                ring.emoji,
                ring.name,
                format_num(ring.cost_gold),
                if ring.cost_diamond > 0 {
                    format!("+{}💎", ring.cost_diamond)
                } else {
                    String::new()
                },
                ring.desc,
            ));
        }
        lines.push(String::new());
        lines.push("⚠️ 求婚后需等待对方回应，24小时内未回应自动取消".to_string());
        return lines.join("\n");
    }

    let parts: Vec<&str> = args.split_whitespace().collect();
    let target_str = parts[0];
    let ring_idx = if parts.len() > 1 {
        parts[1].parse::<usize>().unwrap_or(1).saturating_sub(1)
    } else {
        0
    };

    if ring_idx >= RINGS.len() {
        return format!("❌ 无效的戒指序号，请输入1-{}", RINGS.len());
    }
    let ring = &RINGS[ring_idx];

    // Check: not already married
    if read_marriage(db, user_id).is_some() {
        return "❌ 你已经有伴侣了，需先离婚才能再次求婚".to_string();
    }

    // Check: target exists
    let target_id = match resolve_player(db, target_str) {
        Some(id) => id,
        None => return format!("❌ 找不到玩家「{}」", target_str),
    };
    if target_id == user_id {
        return "❌ 不能向自己求婚".to_string();
    }

    // Check: target not already married
    if read_marriage(db, &target_id).is_some() {
        return "❌ 对方已经有伴侣了".to_string();
    }

    // Check: no pending proposal
    let pending = db.read_user_data(user_id, PROPOSAL_KEY);
    if !pending.is_empty() {
        let pp: Vec<&str> = pending.splitn(2, '|').collect();
        if pp.len() == 2 {
            let ts: u64 = pp[1].parse().unwrap_or(0);
            if now_ts().saturating_sub(ts) < PROPOSE_COOLDOWN_SECS {
                let remaining = PROPOSE_COOLDOWN_SECS - (now_ts().saturating_sub(ts));
                let hours = remaining / 3600;
                return format!("❌ 你有一个待回应的求婚，请等待{}小时后重试", hours);
            }
        }
    }

    // Check: balance
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
    if gold < ring.cost_gold {
        return format!(
            "❌ 金币不足！需要{}金，当前{}金",
            format_num(ring.cost_gold),
            format_num(gold)
        );
    }
    if diamond < ring.cost_diamond {
        return format!("❌ 钻石不足！需要{}💎，当前{}💎", ring.cost_diamond, diamond);
    }

    // Deduct cost
    db.write_currency(user_id, CURRENCY_GOLD, gold - ring.cost_gold);
    if ring.cost_diamond > 0 {
        db.write_currency(user_id, CURRENCY_DIAMOND, diamond - ring.cost_diamond);
    }

    // Store pending proposal
    let target_name = db.read_basic(&target_id, ITEM_NAME);
    let proposal_data = format!(
        "{}|{}|{}|{}|{}|{}",
        target_id,
        target_name,
        ring_idx,
        ring.name,
        ring.emoji,
        now_ts()
    );
    db.write_user_data(user_id, PROPOSAL_KEY, &proposal_data);

    // Store incoming proposal for target
    let sender_name = db.read_basic(user_id, ITEM_NAME);
    let incoming_data = format!("{}|{}|{}|{}|{}", user_id, sender_name, ring_idx, ring.name, ring.emoji);
    db.write_user_data(&target_id, INCOMING_KEY, &incoming_data);

    format!(
        "💍 求婚成功！\n\n你向 {} 送出了 {} {}\n等待对方回应...\n\n💡 对方可用「回应求婚 接受/拒绝」来回应",
        target_name, ring.emoji, ring.name,
    )
}

/// 回应求婚
pub fn cmd_respond_proposal(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let response = args.trim();
    if response.is_empty() {
        return "用法: 回应求婚 接受/拒绝".to_string();
    }

    let accept = match response {
        "接受" | "同意" | "是" | "accept" | "yes" => true,
        "拒绝" | "不同意" | "否" | "reject" | "no" => false,
        _ => return "❌ 请回复「接受」或「拒绝」".to_string(),
    };

    // Check: not already married
    if read_marriage(db, user_id).is_some() {
        return "❌ 你已经有伴侣了".to_string();
    }

    // Find incoming proposal
    let incoming = db.read_user_data(user_id, INCOMING_KEY);
    if incoming.is_empty() {
        return "❌ 你没有待回应的求婚".to_string();
    }

    let parts: Vec<&str> = incoming.splitn(5, '|').collect();
    if parts.len() < 5 {
        return "❌ 求婚数据异常".to_string();
    }

    let sender_id = parts[0].to_string();
    let sender_name = parts[1].to_string();
    let ring_idx: usize = parts[2].parse().unwrap_or(0);
    let ring_name = parts[3].to_string();
    let ring_emoji = parts[4].to_string();

    // Clear proposals
    db.delete_user_data(user_id, INCOMING_KEY);
    db.delete_user_data(&sender_id, PROPOSAL_KEY);

    if !accept {
        // Refund
        if ring_idx < RINGS.len() {
            let ring = &RINGS[ring_idx];
            let gold = db.read_currency(&sender_id, CURRENCY_GOLD);
            let diamond = db.read_currency(&sender_id, CURRENCY_DIAMOND);
            db.write_currency(&sender_id, CURRENCY_GOLD, gold + ring.cost_gold);
            if ring.cost_diamond > 0 {
                db.write_currency(&sender_id, CURRENCY_DIAMOND, diamond + ring.cost_diamond);
            }
        }
        let target_name = db.read_basic(user_id, ITEM_NAME);
        db.write_user_data(&sender_id, NOTIFY_KEY, &format!("💔 {} 拒绝了你的求婚", target_name));
        return format!("💔 你拒绝了 {} 的求婚", sender_name);
    }

    // Accept — create marriage for both parties
    let my_name = db.read_basic(user_id, ITEM_NAME);
    let ts = now_ts();
    let today = today_str();
    let initial_intimacy = RINGS.get(ring_idx).map(|r| r.intimacy_bonus).unwrap_or(10);

    let my_data = MarriageData {
        partner_id: sender_id.clone(),
        partner_name: sender_name.clone(),
        intimacy: initial_intimacy,
        marry_time: ts,
        ring_name: ring_name.clone(),
        ring_emoji: ring_emoji.clone(),
        today_intimacy: 0,
        today_date: today.clone(),
        total_sign_in: 0,
        total_attacks: 0,
    };
    write_marriage(db, user_id, &my_data);

    let partner_data = MarriageData {
        partner_id: user_id.to_string(),
        partner_name: my_name.clone(),
        intimacy: initial_intimacy,
        marry_time: ts,
        ring_name: ring_name.clone(),
        ring_emoji: ring_emoji.clone(),
        today_intimacy: 0,
        today_date: today,
        total_sign_in: 0,
        total_attacks: 0,
    };
    write_marriage(db, &sender_id, &partner_data);

    // Give initial marriage gift to both
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
    let partner_gold = db.read_currency(&sender_id, CURRENCY_GOLD);
    let partner_diamond = db.read_currency(&sender_id, CURRENCY_DIAMOND);
    db.write_currency(user_id, CURRENCY_GOLD, gold + 10000);
    db.write_currency(user_id, CURRENCY_DIAMOND, diamond + 50);
    db.write_currency(&sender_id, CURRENCY_GOLD, partner_gold + 10000);
    db.write_currency(&sender_id, CURRENCY_DIAMOND, partner_diamond + 50);

    let level = get_marriage_level(initial_intimacy);
    format!(
        "💒 恭喜！你接受了 {} 的求婚！\n\n{} {} ✨\n💕 初始亲密度: +{}\n🏷️ 婚姻等级: {} {}\n\n🎁 双方各获得: 10,000金币 + 50钻石\n\n💡 每日签到+{}亲密度，一起战斗+{}亲密度",
        sender_name, ring_emoji, my_data.ring_name, initial_intimacy, level.emoji, level.name,
        SIGN_IN_INTIMACY, ATTACK_INTIMACY,
    )
}

/// 查看伴侣信息
pub fn cmd_view_marriage(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Check for pending incoming proposals
    let incoming = db.read_user_data(user_id, INCOMING_KEY);
    if !incoming.is_empty() {
        let parts: Vec<&str> = incoming.splitn(5, '|').collect();
        if parts.len() >= 5 {
            lines.push("💌 你有一条待回应的求婚:".to_string());
            lines.push(format!("  来自: {}", parts[1]));
            lines.push(format!("  戒指: {} {}", parts[4], parts[3]));
            lines.push("  💡 输入「回应求婚 接受」或「回应求婚 拒绝」".to_string());
            lines.push(String::new());
        }
    }

    // Check notification
    let notify = db.read_user_data(user_id, NOTIFY_KEY);
    if !notify.is_empty() {
        lines.push(format!("📢 通知: {}", notify));
        db.delete_user_data(user_id, NOTIFY_KEY);
        lines.push(String::new());
    }

    let data = match read_marriage(db, user_id) {
        Some(d) => d,
        None => {
            if lines.is_empty() {
                return "💍 你目前单身\n\n💡 输入「求婚」查看求婚系统说明".to_string();
            }
            lines.push("💍 你目前单身".to_string());
            lines.push("💡 输入「求婚」查看求婚系统说明".to_string());
            return lines.join("\n");
        }
    };

    let level = get_marriage_level(data.intimacy);
    let next = get_next_marriage_level(data.intimacy);
    let married_days = now_ts().saturating_sub(data.marry_time) / 86400;

    lines.push(format!("{} 💒 我的伴侣信息", level.emoji));
    lines.push("─".repeat(30));
    lines.push(format!("💍 伴侣: {}", data.partner_name));
    lines.push(format!("💍 结婚戒指: {} {}", data.ring_emoji, data.ring_name));
    lines.push(format!("📅 结婚天数: {}天", married_days));
    lines.push(String::new());

    lines.push(format!("💕 亲密度: {}", data.intimacy));
    if let Some(next_lvl) = next {
        let remaining = next_lvl.intimacy_required - data.intimacy;
        lines.push(format!(
            "📊 {} {} → {} {} (还需{}点)",
            level.emoji, level.name, next_lvl.emoji, next_lvl.name, remaining
        ));
        lines.push(format!(
            "   {}",
            progress_bar(
                data.intimacy - level.intimacy_required,
                next_lvl.intimacy_required - level.intimacy_required,
                15
            )
        ));
    } else {
        lines.push(format!("📊 {} {} (最高等级)", level.emoji, level.name));
        lines.push(format!("   {}", progress_bar(100, 100, 15)));
    }
    lines.push(String::new());

    let today = today_str();
    let today_intimacy = if data.today_date == today {
        data.today_intimacy
    } else {
        0
    };
    lines.push(format!("📈 今日亲密度: {}/{}", today_intimacy, DAILY_INTIMACY_CAP));
    lines.push(format!("   {}", progress_bar(today_intimacy, DAILY_INTIMACY_CAP, 15)));
    lines.push(format!(
        "📊 累计签到: {}次 | 战斗: {}次",
        data.total_sign_in, data.total_attacks
    ));
    lines.push(String::new());

    lines.push(format!("⚔️ 当前伴侣加成 ({} {}):", level.emoji, level.name));
    lines.push(format!(
        "  ❤️ 生命+{}% | ⚔️ 物攻+{}% | 🔮 魔攻+{}%",
        level.bonus_hp_pct, level.bonus_ad_pct, level.bonus_ap_pct
    ));
    lines.push(format!(
        "  🛡️ 防御+{}% | 📈 经验+{}% | 💰 金币+{}%",
        level.bonus_def_pct, level.bonus_exp_pct, level.bonus_gold_pct
    ));
    lines.push(String::new());

    if let Some(next_lvl) = next {
        lines.push(format!("🔮 升级到 {} {} 后:", next_lvl.emoji, next_lvl.name));
        lines.push(format!(
            "  ❤️ 生命+{}% | ⚔️ 物攻+{}% | 🔮 魔攻+{}%",
            next_lvl.bonus_hp_pct, next_lvl.bonus_ad_pct, next_lvl.bonus_ap_pct
        ));
        lines.push(format!(
            "  🛡️ 防御+{}% | 📈 经验+{}% | 💰 金币+{}%",
            next_lvl.bonus_def_pct, next_lvl.bonus_exp_pct, next_lvl.bonus_gold_pct
        ));
    }

    lines.join("\n")
}

/// 离婚
pub fn cmd_divorce(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let confirm = args.trim();
    if confirm != "确认" {
        return "⚠️ 离婚将清除所有婚姻数据，且7天内不能再次求婚\n\n输入「离婚 确认」执行离婚".to_string();
    }

    let data = match read_marriage(db, user_id) {
        Some(d) => d,
        None => return "❌ 你目前单身".to_string(),
    };

    let partner_id = data.partner_id.clone();
    let partner_name = data.partner_name.clone();
    let intimacy = data.intimacy;

    // Clear both sides
    db.delete_user_data(user_id, DATA_KEY);
    db.delete_user_data(&partner_id, DATA_KEY);

    // Notify partner
    let my_name = db.read_basic(user_id, ITEM_NAME);
    db.write_user_data(
        &partner_id,
        NOTIFY_KEY,
        &format!("💔 {} 已与你离婚 (亲密度{})", my_name, intimacy),
    );

    format!(
        "💔 你已与 {} 离婚\n💕 亲密度 {} 已清除\n\n⏳ 7天内不能再次求婚",
        partner_name, intimacy
    )
}

/// 伴侣加成详情
pub fn cmd_marriage_bonus(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let data = match read_marriage(db, user_id) {
        Some(d) => d,
        None => return "❌ 你目前单身，结婚后可享受伴侣加成\n\n💡 输入「求婚」查看求婚系统说明".to_string(),
    };

    let level = get_marriage_level(data.intimacy);
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("⚔️ {} 伴侣加成详情", level.emoji));
    lines.push("─".repeat(30));
    lines.push(format!("💍 伴侣: {} | 💕 亲密度: {}", data.partner_name, data.intimacy));
    lines.push(format!("🏷️ 婚姻等级: {} {}", level.emoji, level.name));
    lines.push(String::new());

    lines.push("📊 当前加成属性:".to_string());
    lines.push(format!("  ❤️  生命:    +{}%", level.bonus_hp_pct));
    lines.push(format!("  ⚔️  物攻:    +{}%", level.bonus_ad_pct));
    lines.push(format!("  🔮  魔攻:    +{}%", level.bonus_ap_pct));
    lines.push(format!("  🛡️  防御:    +{}%", level.bonus_def_pct));
    lines.push(format!("  📈  经验:    +{}%", level.bonus_exp_pct));
    lines.push(format!("  💰  金币:    +{}%", level.bonus_gold_pct));
    lines.push(String::new());

    lines.push("📋 所有婚姻等级加成:".to_string());
    for lvl in MARRIAGE_LEVELS {
        let marker = if lvl.level == level.level { " ← 当前" } else { "" };
        lines.push(format!(
            "  Lv.{} {} {} 亲密度≥{}  ❤️+{}% ⚔️+{}% 🔮+{}% 🛡️+{}% 📈+{}% 💰+{}%{}",
            lvl.level,
            lvl.emoji,
            lvl.name,
            lvl.intimacy_required,
            lvl.bonus_hp_pct,
            lvl.bonus_ad_pct,
            lvl.bonus_ap_pct,
            lvl.bonus_def_pct,
            lvl.bonus_exp_pct,
            lvl.bonus_gold_pct,
            marker,
        ));
    }

    lines.join("\n")
}

/// 伴侣排行
pub fn cmd_marriage_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_users = db.all_users();
    let mut entries: Vec<(String, String, i32)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for uid in &all_users {
        if seen.contains(uid) {
            continue;
        }
        if let Some(data) = read_marriage(db, uid) {
            if !seen.contains(&data.partner_id) {
                seen.insert(uid.clone());
                seen.insert(data.partner_id.clone());
                let name1 = db.read_basic(uid, ITEM_NAME);
                let name2 = data.partner_name.clone();
                entries.push((
                    format!("{}❤️{}", name1, name2),
                    get_marriage_level(data.intimacy).emoji.to_string(),
                    data.intimacy,
                ));
            }
        }
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut lines: Vec<String> = Vec::new();
    lines.push("💒 伴侣排行榜 — 亲密度".to_string());
    lines.push("─".repeat(35));

    if entries.is_empty() {
        lines.push("暂无伴侣数据".to_string());
        return lines.join("\n");
    }

    for (i, (names, emoji, intimacy)) in entries.iter().take(15).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let level = get_marriage_level(*intimacy);
        lines.push(format!(
            "{} {} {} {} 💕{}",
            medal,
            emoji,
            names,
            level.name,
            format_num(*intimacy as i64)
        ));
    }

    // Show current user position
    if let Some(my_data) = read_marriage(db, user_id) {
        let my_total = my_data.intimacy;
        let my_rank = entries.iter().position(|e| e.2 <= my_total).unwrap_or(entries.len());
        lines.push(String::new());
        lines.push(format!(
            "📍 你的排名: 第{}名 (亲密度{})",
            my_rank + 1,
            format_num(my_total as i64)
        ));
    }

    lines.join("\n")
}

// ============================================================
// 公共API - 供其他系统调用
// ============================================================

/// 记录签到亲密度
pub fn record_marriage_sign_in(db: &Database, user_id: &str) {
    if let Some(mut data) = read_marriage(db, user_id) {
        let today = today_str();
        if data.today_date != today {
            data.today_intimacy = 0;
            data.today_date = today;
        }
        if data.today_intimacy < DAILY_INTIMACY_CAP {
            data.today_intimacy += SIGN_IN_INTIMACY;
            data.intimacy += SIGN_IN_INTIMACY;
            data.total_sign_in += 1;
            write_marriage(db, user_id, &data);
        }
    }
}

/// 记录战斗亲密度
pub fn record_marriage_attack(db: &Database, user_id: &str) {
    if let Some(mut data) = read_marriage(db, user_id) {
        let today = today_str();
        if data.today_date != today {
            data.today_intimacy = 0;
            data.today_date = today;
        }
        if data.today_intimacy < DAILY_INTIMACY_CAP {
            data.today_intimacy += ATTACK_INTIMACY;
            data.intimacy += ATTACK_INTIMACY;
            data.total_attacks += 1;
            write_marriage(db, user_id, &data);
        }
    }
}

/// 获取婚姻战斗加成(百分比)
pub fn get_marriage_combat_bonus(db: &Database, user_id: &str) -> MarriageBonus {
    if let Some(data) = read_marriage(db, user_id) {
        let level = get_marriage_level(data.intimacy);
        return MarriageBonus {
            hp_pct: level.bonus_hp_pct,
            ad_pct: level.bonus_ad_pct,
            ap_pct: level.bonus_ap_pct,
            def_pct: level.bonus_def_pct,
            exp_pct: level.bonus_exp_pct,
            gold_pct: level.bonus_gold_pct,
        };
    }
    MarriageBonus::default()
}

/// 婚姻加成结构体
#[derive(Debug, Default)]
pub struct MarriageBonus {
    pub hp_pct: i32,
    pub ad_pct: i32,
    pub ap_pct: i32,
    pub def_pct: i32,
    pub exp_pct: i32,
    pub gold_pct: i32,
}

// ============================================================
/// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levels_count() {
        assert_eq!(MARRIAGE_LEVELS.len(), 10);
    }

    #[test]
    fn test_levels_unique() {
        let mut names: Vec<&str> = MARRIAGE_LEVELS.iter().map(|l| l.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), MARRIAGE_LEVELS.len());
    }

    #[test]
    fn test_levels_ordering() {
        for i in 1..MARRIAGE_LEVELS.len() {
            assert!(MARRIAGE_LEVELS[i].intimacy_required > MARRIAGE_LEVELS[i - 1].intimacy_required);
            assert!(MARRIAGE_LEVELS[i].bonus_hp_pct >= MARRIAGE_LEVELS[i - 1].bonus_hp_pct);
        }
    }

    #[test]
    fn test_rings_count() {
        assert_eq!(RINGS.len(), 4);
    }

    #[test]
    fn test_rings_costs_positive() {
        for ring in RINGS {
            assert!(ring.cost_gold >= 0);
            assert!(ring.cost_diamond >= 0);
            assert!(ring.cost_gold + ring.cost_diamond as i64 > 0);
        }
    }

    #[test]
    fn test_rings_costs_escalate() {
        for i in 1..RINGS.len() {
            let total_prev = RINGS[i - 1].cost_gold + RINGS[i - 1].cost_diamond as i64;
            let total_curr = RINGS[i].cost_gold + RINGS[i].cost_diamond as i64;
            assert!(total_curr >= total_prev);
        }
    }

    #[test]
    fn test_rings_emojis_non_empty() {
        for ring in RINGS {
            assert!(!ring.emoji.is_empty());
            assert!(!ring.name.is_empty());
        }
    }

    #[test]
    fn test_marriage_level_from_zero() {
        let level = get_marriage_level(0);
        assert_eq!(level.level, 1);
        assert_eq!(level.name, "新婚燕尔");
    }

    #[test]
    fn test_marriage_level_max() {
        let level = get_marriage_level(99999);
        assert_eq!(level.level, 10);
        assert_eq!(level.name, "神仙眷侣");
    }

    #[test]
    fn test_marriage_level_boundary() {
        let level = get_marriage_level(50);
        assert_eq!(level.level, 2);
        let level = get_marriage_level(49);
        assert_eq!(level.level, 1);
    }

    #[test]
    fn test_next_level_first() {
        let next = get_next_marriage_level(0);
        assert!(next.is_some());
        assert_eq!(next.unwrap().level, 2);
    }

    #[test]
    fn test_next_level_max() {
        let next = get_next_marriage_level(99999);
        assert!(next.is_none());
    }

    #[test]
    fn test_marriage_data_parse_roundtrip() {
        let data = MarriageData {
            partner_id: "42".to_string(),
            partner_name: "TestPartner".to_string(),
            intimacy: 150,
            marry_time: 1700000000,
            ring_name: "金戒指".to_string(),
            ring_emoji: "👑".to_string(),
            today_intimacy: 15,
            today_date: "2026-163".to_string(),
            total_sign_in: 30,
            total_attacks: 120,
        };
        let serialized = data.serialize();
        let parsed = MarriageData::parse(&serialized).unwrap();
        assert_eq!(parsed.partner_id, "42");
        assert_eq!(parsed.partner_name, "TestPartner");
        assert_eq!(parsed.intimacy, 150);
        assert_eq!(parsed.ring_name, "金戒指");
        assert_eq!(parsed.today_intimacy, 15);
        assert_eq!(parsed.total_sign_in, 30);
        assert_eq!(parsed.total_attacks, 120);
    }

    #[test]
    fn test_marriage_data_parse_empty() {
        assert!(MarriageData::parse("").is_none());
    }

    #[test]
    fn test_marriage_data_parse_minimal() {
        let data = MarriageData::parse("id1|name1|0|0|ring|💍");
        assert!(data.is_some());
        let d = data.unwrap();
        assert_eq!(d.partner_id, "id1");
        assert_eq!(d.today_intimacy, 0);
        assert_eq!(d.total_sign_in, 0);
        assert_eq!(d.total_attacks, 0);
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100, 10);
        assert!(bar.contains("100%"));
        assert!(bar.contains("██████████"));
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100, 10);
        assert!(bar.contains("0%"));
        assert!(bar.contains("░░░░░░░░░░"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 100, 10);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1000000), "1,000,000");
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
    }

    #[test]
    fn test_bonus_constants() {
        assert!(SIGN_IN_INTIMACY > 0);
        assert!(ATTACK_INTIMACY > 0);
        assert!(GIFT_INTIMACY > 0);
        assert!(DAILY_INTIMACY_CAP > 0);
        assert!(SIGN_IN_INTIMACY <= DAILY_INTIMACY_CAP);
    }

    #[test]
    fn test_marriage_bonus_default() {
        let bonus = MarriageBonus::default();
        assert_eq!(bonus.hp_pct, 0);
        assert_eq!(bonus.ad_pct, 0);
    }

    #[test]
    fn test_intimacy_bonus_escalates() {
        for i in 1..RINGS.len() {
            assert!(RINGS[i].intimacy_bonus >= RINGS[i - 1].intimacy_bonus);
        }
    }

    #[test]
    fn test_level_bonus_total_positive() {
        for lvl in MARRIAGE_LEVELS {
            let total = lvl.bonus_hp_pct
                + lvl.bonus_ad_pct
                + lvl.bonus_ap_pct
                + lvl.bonus_def_pct
                + lvl.bonus_exp_pct
                + lvl.bonus_gold_pct;
            assert!(total > 0);
        }
    }

    #[test]
    fn test_level_emojis_non_empty() {
        for lvl in MARRIAGE_LEVELS {
            assert!(!lvl.emoji.is_empty());
        }
    }

    #[test]
    fn test_today_str_format() {
        let s = today_str();
        assert!(s.len() >= 8);
        assert!(s.contains('-'));
    }

    #[test]
    fn test_level_bonus_pct_range() {
        for lvl in MARRIAGE_LEVELS {
            assert!(lvl.bonus_hp_pct > 0 && lvl.bonus_hp_pct <= 50);
            assert!(lvl.bonus_ad_pct > 0 && lvl.bonus_ad_pct <= 50);
            assert!(lvl.bonus_gold_pct > 0 && lvl.bonus_gold_pct <= 50);
        }
    }

    #[test]
    fn test_section_constant() {
        assert_eq!(SECTION, "marriage");
    }

    #[test]
    fn test_ring_names_unique() {
        let mut names: Vec<&str> = RINGS.iter().map(|r| r.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), RINGS.len());
    }
}

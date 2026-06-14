/// CakeGame 守护灵系统 (Guardian Spirit System)
/// 玩家可收集守护灵，培养亲密度，进化守护灵，获得被动属性加成。
/// 守护灵通过击败BOSS、深渊探索、签到等方式获取灵魂碎片唤醒。
/// 数据存储：Global 表 SECTION='guardian_spirit_{user_id}'
use crate::core::*;
use crate::db::Database;

/// 守护灵类型: (名称, 元素, 描述, (HP%, 物攻%, 魔攻%, 防御%, 魔抗%))
#[allow(clippy::type_complexity)]
const SPIRIT_TYPES: &[(&str, &str, &str, (f64, f64, f64, f64, f64))] = &[
    ("🔥炎灵·祝融", "火", "火焰之灵，灼热之力", (0.0, 8.0, 3.0, 0.0, 0.0)),
    ("❄️冰灵·玄冥", "水", "寒冰之灵，冰封之力", (3.0, 0.0, 0.0, 5.0, 8.0)),
    ("⚡雷灵·雷泽", "雷", "雷电之灵，雷霆之力", (0.0, 5.0, 8.0, 0.0, 0.0)),
    ("🌿木灵·句芒", "木", "生命之灵，治愈之力", (10.0, 0.0, 3.0, 3.0, 3.0)),
    ("🪨土灵·后土", "土", "大地之灵，磐石之力", (5.0, 0.0, 0.0, 10.0, 5.0)),
    ("🌪️风灵·飞廉", "风", "狂风之灵，疾速之力", (0.0, 6.0, 6.0, 0.0, 0.0)),
    ("✨光灵·羲和", "光", "圣光之灵，净化之力", (6.0, 3.0, 3.0, 3.0, 6.0)),
    ("🌑暗灵·烛龙", "暗", "暗影之灵，侵蚀之力", (0.0, 7.0, 7.0, 0.0, 0.0)),
    ("💫星灵·太白", "星", "星辰之灵，命运之力", (4.0, 4.0, 4.0, 4.0, 4.0)),
    ("🌀混沌·帝江", "混沌", "混沌之灵，创世之力", (6.0, 6.0, 6.0, 6.0, 6.0)),
];

/// 进化阶段: (名称, 亲密度阈值, 属性倍率)
const EVOLUTION_STAGES: &[(&str, u32, f64)] = &[
    ("幼灵🌱", 0, 1.0),
    ("觉醒灵✨", 20, 1.5),
    ("成熟灵🌟", 60, 2.0),
    ("精英灵💫", 120, 3.0),
    ("传说灵👑", 200, 4.5),
    ("神话灵⚜️", 350, 6.5),
    ("太古灵🌌", 500, 10.0),
];

/// 灵魂碎片来源: (名称, 数量, 描述)
const FRAGMENT_SOURCES: &[(&str, u32, &str)] = &[
    ("击败BOSS", 8, "击败各类BOSS获得灵魂碎片"),
    ("深渊探索", 5, "通关深渊层获得灵魂碎片"),
    ("每日签到", 2, "每日签到获取灵魂碎片"),
    ("公会活动", 3, "参与公会战/试炼获得灵魂碎片"),
    ("PvP战斗", 2, "PvP胜利获得灵魂碎片"),
    ("采集收获", 1, "采集/种植收获获得灵魂碎片"),
];

/// 亲密度里程碑: (名称, 亲密度阈值, 金币奖励, 钻石奖励)
const AFFINITY_MILESTONES: &[(&str, u32, u64, u64)] = &[
    ("初次共鸣", 10, 2000, 15),
    ("灵魂契约", 30, 8000, 50),
    ("心意相通", 80, 25000, 150),
    ("灵魂合一", 150, 80000, 400),
    ("永恒羁绊", 300, 200000, 1000),
    ("太古传承", 500, 500000, 2500),
];

/// 守护灵数据
#[derive(Debug, Clone)]
struct SpiritData {
    spirit_type: usize,       // 守护灵类型索引
    affinity: u32,            // 亲密度
    evolution_stage: usize,   // 进化阶段索引
    total_fragments: u32,     // 累计碎片
    daily_interactions: u32,  // 今日互动次数
    milestones_claimed: u32,  // 已领取里程碑(bitmask)
    last_interaction: String, // 上次互动日期
}

impl SpiritData {
    fn new(spirit_type: usize) -> Self {
        Self {
            spirit_type,
            affinity: 0,
            evolution_stage: 0,
            total_fragments: 0,
            daily_interactions: 0,
            milestones_claimed: 0,
            last_interaction: String::new(),
        }
    }

    fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() < 7 {
            return None;
        }
        Some(Self {
            spirit_type: parts[0].parse().unwrap_or(0),
            affinity: parts[1].parse().unwrap_or(0),
            evolution_stage: parts[2].parse().unwrap_or(0),
            total_fragments: parts[3].parse().unwrap_or(0),
            daily_interactions: parts[4].parse().unwrap_or(0),
            milestones_claimed: parts[5].parse().unwrap_or(0),
            last_interaction: parts[6].to_string(),
        })
    }

    #[allow(clippy::inherent_to_string)]
    fn to_string(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            self.spirit_type,
            self.affinity,
            self.evolution_stage,
            self.total_fragments,
            self.daily_interactions,
            self.milestones_claimed,
            self.last_interaction
        )
    }
}

/// 获取用户数据
fn get_user_value(db: &Database, user_id: &str, key: &str) -> String {
    let section = format!("guardian_spirit_{}", user_id);
    db.global_get(&section, key)
}

/// 设置用户数据
fn set_user_value(db: &Database, user_id: &str, key: &str, value: &str) {
    let section = format!("guardian_spirit_{}", user_id);
    db.global_set(&section, key, value);
}

/// 获取用户守护灵数据
fn get_spirit_data(db: &Database, user_id: &str) -> Option<SpiritData> {
    let raw = get_user_value(db, user_id, "spirit");
    if raw.is_empty() {
        return None;
    }
    SpiritData::from_string(&raw)
}

/// 保存用户守护灵数据
fn save_spirit_data(db: &Database, user_id: &str, data: &SpiritData) {
    set_user_value(db, user_id, "spirit", &data.to_string());
}

/// 获取今天的日期字符串
fn today_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    format!("{}", days)
}

/// 获取进化阶段信息
fn get_evolution_stage(affinity: u32) -> (usize, &'static str, f64) {
    let mut idx = 0;
    for (i, &(_, threshold, _)) in EVOLUTION_STAGES.iter().enumerate() {
        if affinity >= threshold {
            idx = i;
        }
    }
    let (name, _, multiplier) = EVOLUTION_STAGES[idx];
    (idx, name, multiplier)
}

/// 生成进度条
fn progress_bar(current: u32, target: u32, width: usize) -> String {
    let pct = if target == 0 {
        1.0
    } else {
        (current as f64 / target as f64).min(1.0)
    };
    let filled = (pct * width as f64) as usize;
    let empty = width - filled;
    format!("{}{} {:.0}%", "█".repeat(filled), "░".repeat(empty), pct * 100.0)
}

/// 计算守护灵属性加成
pub fn get_spirit_bonus(db: &Database, user_id: &str) -> (f64, f64, f64, f64, f64) {
    let data = match get_spirit_data(db, user_id) {
        Some(d) => d,
        None => return (0.0, 0.0, 0.0, 0.0, 0.0),
    };
    let (_, _, multiplier) = get_evolution_stage(data.affinity);
    let (_, _, _, base_bonus) = SPIRIT_TYPES[data.spirit_type.min(SPIRIT_TYPES.len() - 1)];
    (
        base_bonus.0 * multiplier,
        base_bonus.1 * multiplier,
        base_bonus.2 * multiplier,
        base_bonus.3 * multiplier,
        base_bonus.4 * multiplier,
    )
}

/// 查看守护灵
pub fn cmd_view_spirit(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let data = match get_spirit_data(db, user_id) {
        Some(d) => d,
        None => {
            let mut out = String::from("🔮 ═══════【守护灵殿堂】═══════ 🔮\n\n");
            out.push_str("你还没有唤醒任何守护灵。\n\n");
            out.push_str("📜 如何获取守护灵：\n");
            out.push_str("  收集10个灵魂碎片即可唤醒守护灵\n\n");
            out.push_str("💎 碎片来源：\n");
            for (name, amount, desc) in FRAGMENT_SOURCES {
                out.push_str(&format!("  • {} (+{}/次) — {}\n", name, amount, desc));
            }
            out.push_str("\n💡 输入 \"收集碎片\" 查看你的碎片收集进度\n");
            out.push_str("💡 输入 \"唤醒守护灵\" 消耗碎片唤醒守护灵\n");
            return out;
        }
    };

    let (spirit_name, spirit_element, spirit_desc, _) = SPIRIT_TYPES[data.spirit_type.min(SPIRIT_TYPES.len() - 1)];
    let (stage_idx, stage_name, multiplier) = get_evolution_stage(data.affinity);
    let bonus = get_spirit_bonus(db, user_id);

    let mut out = String::from("🔮 ═══════【守护灵殿堂】═══════ 🔮\n\n");
    out.push_str(&format!("守护灵: {}\n", spirit_name));
    out.push_str(&format!("元素: {} | {}\n", spirit_element, spirit_desc));
    out.push_str(&format!("进化阶段: {} (×{:.1}属性倍率)\n", stage_name, multiplier));
    out.push_str(&format!("亲密度: {} 💕\n", data.affinity));
    out.push_str(&format!("累计碎片: {} 🔮\n\n", data.total_fragments));

    out.push_str("📊 当前属性加成：\n");
    out.push_str(&format!(
        "  ❤️ HP: +{:.1}%  ⚔️ 物攻: +{:.1}%  🔮 魔攻: +{:.1}%\n",
        bonus.0, bonus.1, bonus.2
    ));
    out.push_str(&format!("  🛡️ 防御: +{:.1}%  🔮 魔抗: +{:.1}%\n\n", bonus.3, bonus.4));

    // 下一阶段进度
    if stage_idx + 1 < EVOLUTION_STAGES.len() {
        let (next_name, next_threshold, _) = EVOLUTION_STAGES[stage_idx + 1];
        out.push_str(&format!(
            "📈 下一阶段: {} (亲密度 {}/{})\n",
            next_name, data.affinity, next_threshold
        ));
        out.push_str(&format!(
            "  进度: {}\n\n",
            progress_bar(data.affinity, next_threshold, 15)
        ));
    } else {
        out.push_str("🌟 已达到最高进化阶段！\n\n");
    }

    // 每日互动状态
    let today = today_str();
    if data.last_interaction == today {
        out.push_str(&format!("💬 今日互动: {}/5次\n", data.daily_interactions));
    } else {
        out.push_str("💬 今日互动: 0/5次 (新的一天)\n");
    }

    out.push_str("\n📋 可用指令：\n");
    out.push_str("  守护灵 — 查看守护灵状态\n");
    out.push_str("  唤醒守护灵 — 消耗碎片唤醒守护灵\n");
    out.push_str("  互动守护灵 — 与守护灵互动增加亲密度\n");
    out.push_str("  守护灵详情 — 查看进化路线和碎片来源\n");
    out.push_str("  守护灵排行 — 全服守护灵亲密度排行\n");
    out.push_str("  守护灵帮助 — 查看帮助信息\n");

    out
}

/// 唤醒守护灵
pub fn cmd_awake_spirit(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 如果已有守护灵
    if let Some(data) = get_spirit_data(db, user_id) {
        let (name, _, _, _) = SPIRIT_TYPES[data.spirit_type.min(SPIRIT_TYPES.len() - 1)];
        return format!(
            "❌ 你已经拥有守护灵 {}，无法再次唤醒。\n💡 输入 \"守护灵\" 查看详情。",
            name
        );
    }

    // 检查碎片
    let fragments: u32 = get_user_value(db, user_id, "fragments").parse().unwrap_or(0);
    if fragments < 10 {
        return format!(
            "❌ 灵魂碎片不足！需要10个碎片，当前仅有{}个。\n💡 通过击败BOSS、深渊探索、签到等方式获取碎片。",
            fragments
        );
    }

    // 确定守护灵类型
    let spirit_type = if args.is_empty() {
        // 随机选择
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        (nanos as usize) % SPIRIT_TYPES.len()
    } else {
        // 尝试按名称匹配
        let trimmed = args.trim();
        let mut found = None;
        for (i, (name, _, _, _)) in SPIRIT_TYPES.iter().enumerate() {
            if name.contains(trimmed) || trimmed.contains(&name[2..]) {
                found = Some(i);
                break;
            }
        }
        match found {
            Some(idx) => idx,
            None => {
                let mut out = String::from("❌ 未找到该守护灵！可选守护灵：\n");
                for (i, (name, element, _, _)) in SPIRIT_TYPES.iter().enumerate() {
                    out.push_str(&format!("  {}. {} ({})\n", i + 1, name, element));
                }
                out.push_str("\n💡 输入 \"唤醒守护灵+名称\" 指定守护灵，或直接 \"唤醒守护灵\" 随机唤醒。");
                return out;
            }
        }
    };

    // 扣除碎片并创建守护灵
    set_user_value(db, user_id, "fragments", &(fragments - 10).to_string());

    let mut data = SpiritData::new(spirit_type);
    data.total_fragments = 10;
    save_spirit_data(db, user_id, &data);

    let (name, element, desc, bonus) = SPIRIT_TYPES[spirit_type];

    let mut out = String::from("🎉 ═══ 守护灵唤醒成功！═══ 🎉\n\n");
    out.push_str(&format!("✨ 你唤醒了 {} ({})！\n", name, element));
    out.push_str(&format!("📖 {}\n\n", desc));
    out.push_str("📊 初始属性加成：\n");
    out.push_str(&format!(
        "  ❤️ HP: +{:.1}%  ⚔️ 物攻: +{:.1}%  🔮 魔攻: +{:.1}%\n",
        bonus.0, bonus.1, bonus.2
    ));
    out.push_str(&format!("  🛡️ 防御: +{:.1}%  🔮 魔抗: +{:.1}%\n\n", bonus.3, bonus.4));
    out.push_str("💡 通过 \"互动守护灵\" 增加亲密度，解锁更高进化阶段！\n");
    out.push_str(&format!("💡 剩余碎片: {} 🔮\n", fragments - 10));

    out
}

/// 互动守护灵 — 增加亲密度
pub fn cmd_interact_spirit(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut data = match get_spirit_data(db, user_id) {
        Some(d) => d,
        None => return "❌ 你还没有守护灵！输入 \"唤醒守护灵\" 获取守护灵。".to_string(),
    };

    let today = today_str();

    // 重置每日互动次数
    if data.last_interaction != today {
        data.daily_interactions = 0;
        data.last_interaction = today.clone();
    }

    if data.daily_interactions >= 5 {
        return "❌ 今日互动次数已达上限(5次/天)。明天再来吧！".to_string();
    }

    // 增加亲密度
    let affinity_gain = match data.daily_interactions {
        0 => 5, // 第一次互动
        1 => 4,
        2 => 3,
        3 => 2,
        _ => 1,
    };

    data.affinity += affinity_gain;
    data.daily_interactions += 1;

    // 检查是否进化
    let (old_stage, _, _) = get_evolution_stage(data.affinity - affinity_gain);
    let (new_stage, new_stage_name, new_multiplier) = get_evolution_stage(data.affinity);
    data.evolution_stage = new_stage;

    save_spirit_data(db, user_id, &data);

    let (spirit_name, _, _, _) = SPIRIT_TYPES[data.spirit_type.min(SPIRIT_TYPES.len() - 1)];

    let mut out = String::from("💕 ═══ 守护灵互动 ═══ 💕\n\n");
    out.push_str(&format!("你与 {} 进行了亲密互动。\n", spirit_name));
    out.push_str(&format!("亲密度 +{} → 当前: {} 💕\n", affinity_gain, data.affinity));
    out.push_str(&format!("今日互动: {}/5次\n\n", data.daily_interactions));

    if new_stage > old_stage {
        out.push_str("🎉🎉🎉 守护灵进化了！🎉🎉🎉\n");
        out.push_str(&format!(
            "新阶段: {} (×{:.1}属性倍率)\n\n",
            new_stage_name, new_multiplier
        ));
    }

    // 检查里程碑
    for (i, &(name, threshold, gold, diamond)) in AFFINITY_MILESTONES.iter().enumerate() {
        let bit = 1u32 << i;
        if data.affinity >= threshold && (data.milestones_claimed & bit) == 0 {
            out.push_str(&format!("🏆 里程碑达成: {} (亲密度{})\n", name, threshold));
            out.push_str(&format!("  💰 奖励: {}金币 + {}钻石\n", gold, diamond));
            out.push_str(&format!("  💡 输入 \"领取灵缘奖励+{}\" 领取\n", name));
        }
    }

    out
}

/// 领取灵缘里程碑奖励
pub fn cmd_claim_spirit_milestone(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let mut data = match get_spirit_data(db, user_id) {
        Some(d) => d,
        None => return "❌ 你还没有守护灵！".to_string(),
    };

    let target_name = args.trim();
    if target_name.is_empty() {
        let mut out = String::from("🏆 ═══ 灵缘里程碑 ═══ 🏆\n\n");
        for (i, &(name, threshold, gold, diamond)) in AFFINITY_MILESTONES.iter().enumerate() {
            let bit = 1u32 << i;
            let claimed = (data.milestones_claimed & bit) != 0;
            let reached = data.affinity >= threshold;
            let status = if claimed {
                "✅ 已领取"
            } else if reached {
                "🔓 可领取"
            } else {
                "🔒 未达成"
            };
            out.push_str(&format!(
                "  {} {} (亲密度{}) — {}金+{}💎 → {}\n",
                status, name, threshold, gold, diamond, status
            ));
        }
        return out;
    }

    // 查找匹配的里程碑
    let mut found_idx = None;
    for (i, &(name, threshold, _, _)) in AFFINITY_MILESTONES.iter().enumerate() {
        if name.contains(target_name) || target_name.contains(name) {
            found_idx = Some(i);
            break;
        }
        // 也检查亲密度阈值
        if format!("{}", threshold) == target_name {
            found_idx = Some(i);
            break;
        }
    }

    let idx = match found_idx {
        Some(i) => i,
        None => {
            return format!(
                "❌ 未找到里程碑 \"{}\"。输入 \"领取灵缘奖励\" 查看所有里程碑。",
                target_name
            )
        }
    };

    let bit = 1u32 << idx;
    if (data.milestones_claimed & bit) != 0 {
        return "❌ 该里程碑奖励已领取过了。".to_string();
    }
    let (_, threshold, gold, diamond) = AFFINITY_MILESTONES[idx];
    if data.affinity < threshold {
        return format!("❌ 亲密度不足！需要{}，当前{}。", threshold, data.affinity);
    }

    // 发放奖励
    data.milestones_claimed |= bit;
    save_spirit_data(db, user_id, &data);

    let current_gold: u64 = db.read_basic(user_id, CURRENCY_GOLD).parse().unwrap_or(0);
    db.write_basic(user_id, CURRENCY_GOLD, &(current_gold + gold).to_string());
    let current_diamond: u64 = db.read_basic(user_id, CURRENCY_DIAMOND).parse().unwrap_or(0);
    db.write_basic(user_id, CURRENCY_DIAMOND, &(current_diamond + diamond).to_string());

    let (name, _, _, _) = SPIRIT_TYPES[data.spirit_type.min(SPIRIT_TYPES.len() - 1)];

    let mut out = String::from("🎉 ═══ 里程碑奖励领取成功！═══ 🎉\n\n");
    out.push_str(&format!("🏆 {}\n", AFFINITY_MILESTONES[idx].0));
    out.push_str(&format!("💰 金币: +{}\n", gold));
    out.push_str(&format!("💎 钻石: +{}\n\n", diamond));
    out.push_str(&format!("守护灵 {} 的亲密度: {} 💕\n", name, data.affinity));
    out
}

/// 守护灵详情 — 进化路线和碎片来源
pub fn cmd_spirit_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🔮 ═══════【守护灵详情】═══════ 🔮\n\n");

    // 进化路线
    out.push_str("📈 进化路线：\n");
    for &(name, threshold, multiplier) in EVOLUTION_STAGES.iter() {
        out.push_str(&format!("  → {} (亲密度{}, ×{:.1}倍率)\n", name, threshold, multiplier));
    }
    out.push('\n');

    // 守护灵图鉴
    out.push_str("📖 守护灵图鉴：\n");
    for (i, &(name, element, desc, bonus)) in SPIRIT_TYPES.iter().enumerate() {
        out.push_str(&format!("  {}. {} [{}] — {}\n", i + 1, name, element, desc));
        out.push_str(&format!(
            "     加成: HP+{:.0}% 物攻+{:.0}% 魔攻+{:.0}% 防御+{:.0}% 魔抗+{:.0}%\n",
            bonus.0, bonus.1, bonus.2, bonus.3, bonus.4
        ));
    }
    out.push('\n');

    // 碎片来源
    out.push_str("💎 碎片来源：\n");
    for (name, amount, desc) in FRAGMENT_SOURCES {
        out.push_str(&format!("  • {} (+{}/次) — {}\n", name, amount, desc));
    }

    // 用户当前状态
    if let Some(data) = get_spirit_data(db, user_id) {
        let fragments: u32 = get_user_value(db, user_id, "fragments").parse().unwrap_or(0);
        let (name, _, _, _) = SPIRIT_TYPES[data.spirit_type.min(SPIRIT_TYPES.len() - 1)];
        out.push_str(&format!(
            "\n📊 你的状态: {} | 亲密度:{} | 碎片:{}\n",
            name, data.affinity, fragments
        ));
    } else {
        let fragments: u32 = get_user_value(db, user_id, "fragments").parse().unwrap_or(0);
        out.push_str(&format!("\n📊 你的碎片: {} (需要10个唤醒守护灵)\n", fragments));
    }

    out
}

/// 守护灵排行
pub fn cmd_spirit_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // 扫描所有用户数据
    let mut rankings: Vec<(String, u32, usize, usize)> = Vec::new();

    // 从 Basic_User 表获取所有用户
    let users = db.all_users();
    for uid in &users {
        let section = format!("guardian_spirit_{}", uid);
        let raw = db.global_get(&section, "spirit");
        if raw.is_empty() {
            continue;
        }
        if let Some(data) = SpiritData::from_string(&raw) {
            rankings.push((uid.clone(), data.affinity, data.spirit_type, data.evolution_stage));
        }
    }

    if rankings.is_empty() {
        return "📊 守护灵排行榜\n\n暂无玩家拥有守护灵。".to_string();
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = String::from("🔮 ═══════【守护灵排行榜】═══════ 🔮\n\n");

    for (i, (uid, affinity, spirit_type, _stage)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, ITEM_NAME);
        let display_name = if name.is_empty() { uid.clone() } else { name };
        let st_idx = (*spirit_type).min(SPIRIT_TYPES.len() - 1);
        let (spirit_name, _, _, _) = SPIRIT_TYPES[st_idx];
        let (_, stage_name, multiplier) = get_evolution_stage(*affinity);
        out.push_str(&format!(
            "{}{}. {} — {} {} (亲密度:{} ×{:.1})\n",
            medal,
            i + 1,
            display_name,
            spirit_name,
            stage_name,
            affinity,
            multiplier
        ));
    }

    // 用户排名
    let user_pos = rankings.iter().position(|(uid, _, _, _)| uid == user_id);
    if let Some(pos) = user_pos {
        out.push_str(&format!("\n📍 你的排名: 第{}名", pos + 1));
    }

    out
}

/// 守护灵帮助
pub fn cmd_spirit_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🔮 ═══════【守护灵帮助】═══════ 🔮\n\n");
    out.push_str("守护灵系统让你收集并培养守护灵，获得强大的被动属性加成。\n\n");
    out.push_str("📋 指令列表：\n");
    out.push_str("  守护灵 — 查看你的守护灵状态\n");
    out.push_str("  唤醒守护灵 — 消耗10碎片唤醒守护灵\n");
    out.push_str("  唤醒守护灵+名称 — 指定唤醒特定守护灵\n");
    out.push_str("  互动守护灵 — 与守护灵互动增加亲密度\n");
    out.push_str("  领取灵缘奖励 — 查看/领取灵缘里程碑奖励\n");
    out.push_str("  守护灵详情 — 查看进化路线和守护灵图鉴\n");
    out.push_str("  守护灵排行 — 全服守护灵亲密度排行\n");
    out.push_str("  守护灵帮助 — 查看帮助信息\n\n");
    out.push_str("🎯 玩法说明：\n");
    out.push_str("  1. 收集灵魂碎片（击败BOSS/深渊/签到/公会/PvP/采集）\n");
    out.push_str("  2. 集齐10个碎片唤醒守护灵\n");
    out.push_str("  3. 每日互动增加亲密度（5次/天）\n");
    out.push_str("  4. 亲密度达标自动进化，属性倍率递增\n");
    out.push_str("  5. 达成灵缘里程碑领取额外奖励\n\n");
    out.push_str("📊 进化阶段：幼灵→觉醒灵→成熟灵→精英灵→传说灵→神话灵→太古灵\n");
    out.push_str("💡 10种守护灵各有不同元素属性加成，选择最适合你的！\n");

    out
}

/// 记录碎片获取 (供其他系统调用)
#[allow(dead_code)]
pub fn record_spirit_fragment(db: &Database, user_id: &str, source: &str) {
    let amount = FRAGMENT_SOURCES
        .iter()
        .find(|(name, _, _)| name == &source)
        .map(|(_, amount, _)| *amount)
        .unwrap_or(1);

    let current: u32 = get_user_value(db, user_id, "fragments").parse().unwrap_or(0);
    set_user_value(db, user_id, "fragments", &(current + amount).to_string());

    // 同时增加守护灵的累计碎片
    if let Some(mut data) = get_spirit_data(db, user_id) {
        data.total_fragments += amount;
        save_spirit_data(db, user_id, &data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open("/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb").unwrap()
    }

    #[test]
    fn test_spirit_types_count() {
        assert_eq!(SPIRIT_TYPES.len(), 10);
    }

    #[test]
    fn test_evolution_stages_count() {
        assert_eq!(EVOLUTION_STAGES.len(), 7);
    }

    #[test]
    fn test_evolution_stages_sorted() {
        for i in 1..EVOLUTION_STAGES.len() {
            assert!(EVOLUTION_STAGES[i].1 > EVOLUTION_STAGES[i - 1].1);
        }
    }

    #[test]
    fn test_fragment_sources_count() {
        assert_eq!(FRAGMENT_SOURCES.len(), 6);
    }

    #[test]
    fn test_affinity_milestones_count() {
        assert_eq!(AFFINITY_MILESTONES.len(), 6);
    }

    #[test]
    fn test_affinity_milestones_sorted() {
        for i in 1..AFFINITY_MILESTONES.len() {
            assert!(AFFINITY_MILESTONES[i].1 > AFFINITY_MILESTONES[i - 1].1);
        }
    }

    #[test]
    fn test_get_evolution_stage_at_zero() {
        let (idx, name, multiplier) = get_evolution_stage(0);
        assert_eq!(idx, 0);
        assert_eq!(name, "幼灵🌱");
        assert_eq!(multiplier, 1.0);
    }

    #[test]
    fn test_get_evolution_stage_at_max() {
        let (idx, name, multiplier) = get_evolution_stage(1000);
        assert_eq!(idx, 6);
        assert_eq!(name, "太古灵🌌");
        assert_eq!(multiplier, 10.0);
    }

    #[test]
    fn test_get_evolution_stage_boundary() {
        let (idx, _, _) = get_evolution_stage(20);
        assert_eq!(idx, 1);
        let (idx, _, _) = get_evolution_stage(19);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100, 10);
        assert!(bar.contains("0%"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100, 10);
        assert!(bar.contains("100%"));
        assert!(bar.contains("█"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 100, 10);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_progress_bar_overflow() {
        let bar = progress_bar(150, 100, 10);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_spirit_data_roundtrip() {
        let data = SpiritData {
            spirit_type: 3,
            affinity: 42,
            evolution_stage: 2,
            total_fragments: 50,
            daily_interactions: 3,
            milestones_claimed: 0b101,
            last_interaction: "12345".to_string(),
        };
        let s = data.to_string();
        let restored = SpiritData::from_string(&s).unwrap();
        assert_eq!(restored.spirit_type, 3);
        assert_eq!(restored.affinity, 42);
        assert_eq!(restored.evolution_stage, 2);
        assert_eq!(restored.total_fragments, 50);
        assert_eq!(restored.daily_interactions, 3);
        assert_eq!(restored.milestones_claimed, 0b101);
    }

    #[test]
    fn test_spirit_data_invalid() {
        assert!(SpiritData::from_string("").is_none());
        assert!(SpiritData::from_string("1,2").is_none());
    }

    #[test]
    fn test_spirit_bonus_no_spirit() {
        let db = test_db();
        let (hp, ad, ap, df, mr) = get_spirit_bonus(&db, "nonexistent_guardian_user");
        assert_eq!(hp, 0.0);
        assert_eq!(ad, 0.0);
        assert_eq!(ap, 0.0);
        assert_eq!(df, 0.0);
        assert_eq!(mr, 0.0);
    }

    #[test]
    fn test_spirit_bonus_with_data() {
        let db = test_db();
        let uid = "test_guardian_spirit_bonus";
        let data = SpiritData::new(0); // 炎灵
        save_spirit_data(&db, uid, &data);
        let (hp, ad, ap, _df, _mr) = get_spirit_bonus(&db, uid);
        assert_eq!(hp, 0.0); // 炎灵HP=0
        assert_eq!(ad, 8.0); // 炎灵物攻=8
        assert_eq!(ap, 3.0); // 炎灵魔攻=3
    }

    #[test]
    fn test_today_str_not_empty() {
        let s = today_str();
        assert!(!s.is_empty());
    }

    #[test]
    fn test_all_spirit_types_unique() {
        let mut names: Vec<&str> = SPIRIT_TYPES.iter().map(|(n, _, _, _)| *n).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), SPIRIT_TYPES.len());
    }

    #[test]
    fn test_evolution_multiplier_always_increasing() {
        for i in 1..EVOLUTION_STAGES.len() {
            assert!(
                EVOLUTION_STAGES[i].2 > EVOLUTION_STAGES[i - 1].2,
                "Stage {} multiplier should be > stage {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_milestone_bitmask() {
        let mut claimed: u32 = 0;
        for i in 0..6 {
            let bit = 1u32 << i;
            assert_eq!(claimed & bit, 0);
            claimed |= bit;
            assert_ne!(claimed & bit, 0);
        }
        assert_eq!(claimed, 0b111111);
    }
}

/// CakeGame 地图挖掘系统
/// 在各地图中挖掘埋藏的宝物，探索隐藏资源
/// 支持：查看挖掘点/开始挖掘/挖掘背包/挖掘排行/挖掘图鉴
use crate::db::Database;
use crate::user;

/// 挖掘点定义
struct DigSite {
    name: &'static str,
    map_name: &'static str,
    min_level: i32,
    emoji: &'static str,
    /// 可能的掉落物品 (物品名, 权重)
    drops: &'static [(&'static str, i32)],
    /// 稀有掉落 (物品名, 掉率百分比)
    rare_drops: &'static [(&'static str, i32)],
    /// 金币奖励范围
    gold_min: i64,
    gold_max: i64,
    /// 经验奖励
    exp: i64,
}

const DIG_SITES: &[DigSite] = &[
    DigSite {
        name: "格兰森林浅层",
        map_name: "格兰森林",
        min_level: 1,
        emoji: "🌿",
        drops: &[
            ("铜矿石", 30),
            ("铁矿石", 20),
            ("木材", 25),
            ("生命药水", 15),
            ("魔力药水", 10),
        ],
        rare_drops: &[("强化石", 5), ("绿色精粹", 3)],
        gold_min: 50,
        gold_max: 200,
        exp: 30,
    },
    DigSite {
        name: "磐石矿脉",
        map_name: "磐石之地",
        min_level: 12,
        emoji: "🪨",
        drops: &[
            ("铁矿石", 25),
            ("银矿石", 20),
            ("铜矿石", 15),
            ("大生命药水", 10),
            ("大魔力药水", 10),
        ],
        rare_drops: &[("强化石", 8), ("紫色精粹", 3), ("宝石碎片", 2)],
        gold_min: 200,
        gold_max: 500,
        exp: 80,
    },
    DigSite {
        name: "莱茵古战场",
        map_name: "莱茵高原",
        min_level: 10,
        emoji: "⚔️",
        drops: &[
            ("铁矿石", 20),
            ("银矿石", 15),
            ("战魂碎片", 10),
            ("生命药水", 20),
            ("魔力药水", 15),
        ],
        rare_drops: &[("强化石", 6), ("灵魂结晶", 2)],
        gold_min: 150,
        gold_max: 400,
        exp: 60,
    },
    DigSite {
        name: "幽暗沼泽秘藏",
        map_name: "幽暗沼泽",
        min_level: 18,
        emoji: "🌿",
        drops: &[
            ("银矿石", 20),
            ("金矿石", 10),
            ("毒蘑菇", 15),
            ("大生命药水", 15),
            ("解毒药", 20),
        ],
        rare_drops: &[("强化石", 10), ("蓝色精粹", 4), ("虚空宝石", 1)],
        gold_min: 300,
        gold_max: 700,
        exp: 120,
    },
    DigSite {
        name: "荒芜之地遗迹",
        map_name: "荒芜之地",
        min_level: 22,
        emoji: "🏚️",
        drops: &[
            ("金矿石", 15),
            ("秘银矿石", 10),
            ("古代残片", 12),
            ("超生命药水", 10),
            ("超魔力药水", 10),
        ],
        rare_drops: &[("强化石", 12), ("红色精粹", 3), ("初级红宝石", 2)],
        gold_min: 500,
        gold_max: 1200,
        exp: 180,
    },
    DigSite {
        name: "冰霜洞穴深处",
        map_name: "冰霜洞穴",
        min_level: 28,
        emoji: "❄️",
        drops: &[
            ("秘银矿石", 15),
            ("寒冰碎片", 12),
            ("精金矿石", 8),
            ("超生命药水", 12),
            ("超魔力药水", 12),
        ],
        rare_drops: &[("强化石", 15), ("绿色精粹", 5), ("初级蓝宝石", 2)],
        gold_min: 800,
        gold_max: 2000,
        exp: 250,
    },
    DigSite {
        name: "火山熔岩矿脉",
        map_name: "烈焰火山",
        min_level: 35,
        emoji: "🔥",
        drops: &[
            ("精金矿石", 15),
            ("熔岩结晶", 12),
            ("火灵石", 10),
            ("超生命药水", 15),
            ("超魔力药水", 12),
        ],
        rare_drops: &[("强化石", 18), ("橙色精粹", 3), ("初级黄宝石", 2)],
        gold_min: 1200,
        gold_max: 3000,
        exp: 350,
    },
    DigSite {
        name: "龙眠神殿废墟",
        map_name: "龙眠神殿",
        min_level: 45,
        emoji: "🐉",
        drops: &[
            ("龙鳞碎片", 15),
            ("精金矿石", 12),
            ("龙血结晶", 10),
            ("复活卷轴", 8),
            ("超生命药水", 15),
        ],
        rare_drops: &[("强化石", 20), ("传说精粹", 2), ("初级紫宝石", 2)],
        gold_min: 2000,
        gold_max: 5000,
        exp: 500,
    },
    DigSite {
        name: "暗影深渊裂隙",
        map_name: "暗影深渊",
        min_level: 55,
        emoji: "🌀",
        drops: &[
            ("暗影结晶", 15),
            ("深渊矿石", 12),
            ("虚空碎片", 10),
            ("复活卷轴", 10),
            ("超生命药水", 15),
        ],
        rare_drops: &[("强化石", 22), ("传说精粹", 4), ("初级绿宝石", 2)],
        gold_min: 3000,
        gold_max: 8000,
        exp: 700,
    },
    DigSite {
        name: "天境废土禁区",
        map_name: "天境废土",
        min_level: 65,
        emoji: "⭐",
        drops: &[
            ("天境矿石", 15),
            ("星辰碎片", 12),
            ("神铁矿", 10),
            ("复活卷轴", 12),
            ("超生命药水", 15),
        ],
        rare_drops: &[("强化石", 25), ("传说精粹", 5), ("初级白宝石", 2)],
        gold_min: 5000,
        gold_max: 15000,
        exp: 1000,
    },
];

/// 每日挖掘次数上限
const DAILY_DIG_LIMIT: i32 = 20;

/// 挖掘冷却时间（秒）
const DIG_COOLDOWN_SECS: i64 = 60;

/// 挖掘体力消耗
const DIG_STAMINA_COST: i32 = 3;

/// 工具品质加成定义
struct DigTool {
    name: &'static str,
    quality_bonus: i64, // 百分比加成
}

const DIG_TOOLS: &[DigTool] = &[
    DigTool {
        name: "铁锹",
        quality_bonus: 0,
    },
    DigTool {
        name: "钢锹",
        quality_bonus: 10,
    },
    DigTool {
        name: "秘银锹",
        quality_bonus: 25,
    },
    DigTool {
        name: "精金锹",
        quality_bonus: 50,
    },
    DigTool {
        name: "龙骨锹",
        quality_bonus: 80,
    },
    DigTool {
        name: "传说神锹",
        quality_bonus: 120,
    },
];

/// 解析 Global 表中的挖掘数据
fn parse_excavation_data(raw: &str) -> (i32, i64, Vec<String>) {
    let mut today_digs = 0i32;
    let mut last_dig_ts = 0i64;
    let mut found_items: Vec<String> = Vec::new();

    for part in raw.split(';') {
        let kv: Vec<&str> = part.splitn(2, '=').collect();
        if kv.len() == 2 {
            match kv[0] {
                "digs" => today_digs = kv[1].parse().unwrap_or(0),
                "ts" => last_dig_ts = kv[1].parse().unwrap_or(0),
                "items" => {
                    found_items = kv[1]
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .collect();
                }
                _ => {}
            }
        }
    }
    (today_digs, last_dig_ts, found_items)
}

/// 序列化挖掘数据
fn serialize_excavation_data(digs: i32, ts: i64, items: &[String]) -> String {
    format!("digs={};ts={};items={}", digs, ts, items.join(","))
}

/// 确定性随机：基于日期+用户ID
fn deterministic_rand(seed: u64, index: u32) -> u32 {
    let mut h: u64 = seed.wrapping_add(index as u64);
    h = h.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    h = h ^ (h >> 33);
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h = h ^ (h >> 33);
    (h & 0x7FFFFFFF) as u32
}

/// 今日日期字符串
fn today_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple date from epoch days
    let days = (now / 86400) as u32;
    // Approximate year/month/day
    let y = 1970 + days / 365;
    let d = days % 365;
    let m = d / 30 + 1;
    let dd = d % 30 + 1;
    format!("{:04}{:02}{:02}", y, m, dd)
}

/// 获取挖掘点详情
fn get_dig_site_for_map(map_name: &str) -> Option<&'static DigSite> {
    DIG_SITES.iter().find(|ds| ds.map_name == map_name)
}

/// 选择掉落物品（基于权重）
fn select_drop(drops: &[(&'static str, i32)], rng_val: u32) -> &'static str {
    let total: i32 = drops.iter().map(|(_, w)| *w).sum();
    if total <= 0 {
        return drops[0].0;
    }
    let pick = (rng_val as i32).rem_euclid(total);
    let mut acc = 0i32;
    for (name, weight) in drops {
        acc += weight;
        if pick < acc {
            return name;
        }
    }
    drops[drops.len() - 1].0
}

// ==================== 公开指令 ====================

/// 查看挖掘点 — 显示所有可挖掘的地图和详情
pub fn cmd_view_dig_sites(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let user_level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);
    let current_map = db.read_basic(user_id, "Map");

    let mut out = format!("{}\n═══ ⛏️ 挖掘系统 ═══\n", prefix);
    out.push_str("在各地图中挖掘埋藏的宝物和矿石！\n");
    out.push_str(&format!(
        "每日上限: {} 次 | 体力消耗: {} 点/次\n\n",
        DAILY_DIG_LIMIT, DIG_STAMINA_COST
    ));

    out.push_str("📍 挖掘点列表：\n");
    for (i, site) in DIG_SITES.iter().enumerate() {
        let level_ok = user_level >= site.min_level;
        let is_here = current_map == site.map_name;
        let status = if !level_ok {
            "🔒"
        } else if is_here {
            "📍"
        } else {
            "✅"
        };
        let level_icon = if level_ok {
            ""
        } else {
            &format!(" (需要Lv.{})", site.min_level)
        };
        out.push_str(&format!(
            "  {} {}{}. {}{}\n",
            status,
            site.emoji,
            i + 1,
            site.name,
            level_icon
        ));
        if level_ok {
            let gold_range = format!("{}~{}", site.gold_min, site.gold_max);
            let rare_names: Vec<&str> = site.rare_drops.iter().map(|(n, _)| *n).collect();
            out.push_str(&format!(
                "     金币:{} 经验:{} 稀有:{}\n",
                gold_range,
                site.exp,
                rare_names.join("/")
            ));
        }
    }

    out.push_str("\n🔧 挖掘工具（背包中的锹可提升奖励）：\n");
    for tool in DIG_TOOLS {
        let bonus_str = if tool.quality_bonus > 0 {
            format!("+{}%", tool.quality_bonus)
        } else {
            "基础".to_string()
        };
        out.push_str(&format!("  {} ({})\n", tool.name, bonus_str));
    }

    out.push_str("\n📋 指令列表：\n");
    out.push_str("  开始挖掘 — 在当前地图挖掘\n");
    out.push_str("  挖掘背包 — 查看挖掘获得的物品\n");
    out.push_str("  挖掘排行 — 全服挖掘达人排行\n");
    out.push_str("  挖掘图鉴 — 已发现的挖掘物品图鉴\n");
    out
}

/// 开始挖掘 — 在当前地图挖掘宝物
pub fn cmd_excavate(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再挖掘！", prefix);
    }

    let user_level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);
    let current_map = db.read_basic(user_id, "Map");

    // 查找当前地图的挖掘点
    let site = match get_dig_site_for_map(&current_map) {
        Some(s) => s,
        None => {
            let available: Vec<&str> = DIG_SITES
                .iter()
                .filter(|ds| user_level >= ds.min_level)
                .map(|ds| ds.map_name)
                .collect();
            return format!(
                "{}\n❌ 「{}」没有可挖掘的资源！\n\n可挖掘的地图：{}",
                prefix,
                current_map,
                if available.is_empty() {
                    "暂无（等级不足）".to_string()
                } else {
                    available.join("、")
                }
            );
        }
    };

    // 等级检查
    if user_level < site.min_level {
        return format!(
            "{}\n🔒 等级不足！{}需要 Lv.{}，当前 Lv.{}",
            prefix, site.name, site.min_level, user_level
        );
    }

    // 读取挖掘状态
    let section = format!("excavation_{}", user_id);
    let raw = db.global_get("excavation", &section);
    let (mut today_digs, last_dig_ts, found_items) = parse_excavation_data(&raw);

    // 日期重置
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let today = today_string();
    let last_date = if last_dig_ts > 0 {
        let last_days = (last_dig_ts / 86400) as u32;
        let ly = 1970 + last_days / 365;
        let ld = last_days % 365;
        let lm = ld / 30 + 1;
        let ldd = ld % 30 + 1;
        format!("{:04}{:02}{:02}", ly, lm, ldd)
    } else {
        String::new()
    };

    if today != last_date {
        today_digs = 0;
    }

    // 每日次数限制
    if today_digs >= DAILY_DIG_LIMIT {
        return format!(
            "{}\n⏰ 今日挖掘次数已用完！({}/{})\n每天零点重置。",
            prefix, today_digs, DAILY_DIG_LIMIT
        );
    }

    // 冷却检查
    if last_dig_ts > 0 && (now - last_dig_ts) < DIG_COOLDOWN_SECS {
        let remaining = DIG_COOLDOWN_SECS - (now - last_dig_ts);
        return format!("{}\n⏳ 挖掘冷却中！还需等待 {} 秒。", prefix, remaining);
    }

    // 体力检查 (通过 stamina 模块间接检查 - 简化处理)
    // 这里简化：直接扣除金币作为体力消耗的替代
    let dig_cost = 100 * (today_digs as i64 + 1); // 递增金币消耗
    let current_gold: i64 = db.read_basic(user_id, "Currency_gold").parse().unwrap_or(0);
    if current_gold < dig_cost {
        return format!("{}\n💰 金币不足！本次挖掘需要 {} 金币（递增消耗）。", prefix, dig_cost);
    }

    // 扣除金币
    db.modify_currency(user_id, "Currency_gold", "sub", dig_cost);

    // 检查工具加成
    let mut tool_bonus: i64 = 0;
    let knapsack = db.query_rows(
        "SELECT ItemName FROM Basic_knapsack WHERE User=? AND ItemName LIKE '%锹%'",
        &[user_id],
        |row| Ok(row.get::<_, String>(0).unwrap_or_default()),
    );
    if let Some(tool_name) = knapsack.first() {
        for tool in DIG_TOOLS {
            if tool_name.contains(tool.name) {
                tool_bonus = tool.quality_bonus;
                break;
            }
        }
    }

    // 确定性随机
    let today_seed = today
        .chars()
        .fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64));
    let user_seed = user_id
        .chars()
        .fold(0u64, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u64));
    let seed = today_seed ^ user_seed ^ (today_digs as u64);

    let rng1 = deterministic_rand(seed, 1);
    let rng2 = deterministic_rand(seed, 2);
    let rng3 = deterministic_rand(seed, 3);

    // 计算金币奖励
    let gold_range = site.gold_max - site.gold_min;
    let base_gold = site.gold_min + (rng1 as i64 % gold_range.max(1));
    let bonus_gold = base_gold * tool_bonus / 100;
    let total_gold = base_gold + bonus_gold;

    // 计算经验奖励
    let bonus_exp = site.exp * tool_bonus / 100;
    let total_exp = site.exp + bonus_exp;

    // 掉落物品
    let item = select_drop(site.drops, rng2);
    let item_with_qty = format!("{}×1", item);

    // 稀有掉落检查
    let mut rare_item = String::new();
    for (rare_name, rare_pct) in site.rare_drops {
        let rare_roll = deterministic_rand(seed, 100 + rng3) % 100;
        if rare_roll < (*rare_pct as u32 + tool_bonus as u32 / 10).min(50) {
            rare_item = rare_name.to_string();
            break;
        }
    }

    // 发放奖励
    db.modify_currency(user_id, "Currency_gold", "add", total_gold);
    user::add_experience(db, user_id, total_exp as i32);
    db.add_item(user_id, item, 1);
    if !rare_item.is_empty() {
        db.add_item(user_id, &rare_item, 1);
    }

    // 更新挖掘统计
    today_digs += 1;
    let mut new_items = found_items.clone();
    new_items.push(item.to_string());
    if !rare_item.is_empty() {
        new_items.push(format!("★{}", rare_item));
    }
    // 限制记录长度
    if new_items.len() > 100 {
        new_items = new_items[new_items.len() - 100..].to_vec();
    }

    let data = serialize_excavation_data(today_digs, now, &new_items);
    db.global_set("excavation", &section, &data);

    // 更新全局挖掘统计
    let total_digs_str = db.global_get("excavation", "total_digs");
    let total_digs: i64 = total_digs_str.parse().unwrap_or(0) + 1;
    db.global_set("excavation", "total_digs", &total_digs.to_string());

    // 构建输出
    let mut out = format!("{}\n═══ ⛏️ {} 挖掘结果 ═══\n", prefix, site.emoji);
    out.push_str(&format!("📍 地点：{}\n", site.name));
    out.push_str(&format!("💰 消耗：{} 金币\n", dig_cost));
    if tool_bonus > 0 {
        out.push_str(&format!("🔧 工具加成：+{}%\n", tool_bonus));
    }
    out.push('\n');

    out.push_str("🎁 获得：\n");
    out.push_str(&format!("  💰 金币 +{}\n", total_gold));
    out.push_str(&format!("  ⭐ 经验 +{}\n", total_exp));
    out.push_str(&format!("  📦 {}\n", item_with_qty));
    if !rare_item.is_empty() {
        out.push_str(&format!("  💎 ★稀有★ {}！\n", rare_item));
    }

    out.push_str(&format!("\n📊 今日挖掘: {}/{} 次", today_digs, DAILY_DIG_LIMIT));

    out
}

/// 挖掘背包 — 查看挖掘获得的物品记录
pub fn cmd_excavation_bag(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let section = format!("excavation_{}", user_id);
    let raw = db.global_get("excavation", &section);
    let (today_digs, _, found_items) = parse_excavation_data(&raw);

    let mut out = format!("{}\n═══ ⛏️ 挖掘背包 ═══\n", prefix);

    if found_items.is_empty() {
        out.push_str("\n还没有挖掘到任何物品！\n");
        out.push_str("💡 使用「查看挖掘」了解可挖掘的地图\n");
        out.push_str("💡 使用「开始挖掘」在当前地图挖掘\n");
        return out;
    }

    // 统计物品数量
    let mut item_counts: Vec<(String, i32)> = Vec::new();
    for item in &found_items {
        let clean_name = item.trim_start_matches("★");
        let is_rare = item.starts_with("★");
        let display = if is_rare {
            format!("★{}★", clean_name)
        } else {
            clean_name.to_string()
        };
        if let Some(entry) = item_counts.iter_mut().find(|(n, _)| *n == display) {
            entry.1 += 1;
        } else {
            item_counts.push((display, 1));
        }
    }

    // 分类显示
    let rare_items: Vec<&(String, i32)> = item_counts.iter().filter(|(n, _)| n.starts_with("★")).collect();
    let normal_items: Vec<&(String, i32)> = item_counts.iter().filter(|(n, _)| !n.starts_with("★")).collect();

    if !rare_items.is_empty() {
        out.push_str("\n💎 稀有发现：\n");
        for (name, qty) in &rare_items {
            out.push_str(&format!("  {} ×{}\n", name, qty));
        }
    }

    if !normal_items.is_empty() {
        out.push_str("\n📦 普通物品：\n");
        for (name, qty) in &normal_items {
            out.push_str(&format!("  {} ×{}\n", name, qty));
        }
    }

    out.push_str(&format!(
        "\n📊 今日挖掘: {} 次 | 物品种类: {} | 总数: {}\n",
        today_digs,
        item_counts.len(),
        found_items.len()
    ));

    out
}

/// 挖掘排行 — 全服挖掘达人排行
pub fn cmd_excavation_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 遍历 Global 表中的 excavation section
    let all_entries = db.query_rows(
        "SELECT Name, Data FROM Global WHERE SECTION='excavation' AND Name LIKE 'excavation_%'",
        &[],
        |row| {
            let name = row.get::<_, String>(0).unwrap_or_default();
            let data = row.get::<_, String>(1).unwrap_or_default();
            Ok((name, data))
        },
    );

    let mut rankings: Vec<(String, i32, i32)> = Vec::new(); // (user_id, total_digs, rare_count)

    for (name, data) in &all_entries {
        let uid = name.strip_prefix("excavation_").unwrap_or(name).to_string();
        let (digs, _, items) = parse_excavation_data(data);
        let rare_count = items.iter().filter(|i| i.starts_with("★")).count() as i32;
        if digs > 0 {
            rankings.push((uid, digs, rare_count));
        }
    }

    rankings.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));

    let mut out = format!("{}\n═══ ⛏️ 挖掘排行榜 ═══\n", prefix);

    if rankings.is_empty() {
        out.push_str("\n暂无挖掘记录！\n");
        out.push_str("💡 使用「开始挖掘」成为第一个挖掘者！\n");
        return out;
    }

    out.push_str("\n🏆 全服挖掘达人 Top15：\n");
    for (i, (uid, digs, rare)) in rankings.iter().take(15).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let is_me = uid == user_id;
        let mark = if is_me { " ← 你" } else { "" };

        // 尝试获取昵称
        let nickname = db.read_basic(uid, "Nickname");
        let display_name = if nickname.is_empty() { uid.clone() } else { nickname };

        out.push_str(&format!(
            "  {} #{} {} | 挖掘:{}次 | 稀有:{}件{}\n",
            medal,
            i + 1,
            display_name,
            digs,
            rare,
            mark
        ));
    }

    // 当前用户排名
    if let Some(pos) = rankings.iter().position(|(uid, _, _)| uid == user_id) {
        out.push_str(&format!("\n📍 你的排名: #{} / {}\n", pos + 1, rankings.len()));
    } else {
        out.push_str("\n📍 你还没有挖掘记录！\n");
    }

    // 全服统计
    let total_digs_str = db.global_get("excavation", "total_digs");
    let total_digs: i64 = total_digs_str.parse().unwrap_or(0);
    out.push_str(&format!("📊 全服总挖掘次数: {}\n", total_digs));

    out
}

/// 挖掘图鉴 — 已发现的物品图鉴
pub fn cmd_excavation_codex(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 收集所有可能的挖掘物品
    let mut all_possible_items: Vec<(&str, &str)> = Vec::new(); // (item_name, site_name)
    for site in DIG_SITES {
        for (item, _) in site.drops {
            if !all_possible_items.iter().any(|(n, _)| *n == *item) {
                all_possible_items.push((item, site.name));
            }
        }
        for (item, _) in site.rare_drops {
            if !all_possible_items.iter().any(|(n, _)| *n == *item) {
                all_possible_items.push((item, site.name));
            }
        }
    }

    // 获取玩家已发现的物品
    let section = format!("excavation_{}", user_id);
    let raw = db.global_get("excavation", &section);
    let (_, _, found_items) = parse_excavation_data(&raw);
    let mut unique_found: Vec<&str> = found_items.iter().map(|s| s.trim_start_matches("★")).collect();
    unique_found.sort();
    unique_found.dedup();

    let mut out = format!("{}\n═══ ⛏️ 挖掘图鉴 ═══\n", prefix);

    let discovered = unique_found.len();
    let total = all_possible_items.len();
    let pct = discovered.saturating_mul(100).checked_div(total).unwrap_or(0);

    out.push_str(&format!("\n📊 收集进度: {}/{} ({}%)\n", discovered, total, pct));

    // 进度条
    let bar_len = 20;
    let filled = (pct * bar_len / 100).min(bar_len);
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
    out.push_str(&format!("  [{}]\n\n", bar));

    // 按来源显示
    for site in DIG_SITES {
        let mut site_items: Vec<(&str, bool)> = Vec::new(); // (name, is_discovered)
        for (item, _) in site.drops {
            let found = unique_found.contains(item);
            site_items.push((item, found));
        }
        for (item, _) in site.rare_drops {
            let found = unique_found.contains(item);
            site_items.push((item, found));
        }

        let found_count = site_items.iter().filter(|(_, f)| *f).count();
        let total_count = site_items.len();
        let icon = if found_count == total_count { "✅" } else { "📝" };

        out.push_str(&format!("{} {} ({}/{})\n", icon, site.name, found_count, total_count));
        for (item, found) in &site_items {
            let mark = if *found { "✅" } else { "❓" };
            out.push_str(&format!("    {} {}\n", mark, item));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dig_sites_count() {
        assert_eq!(DIG_SITES.len(), 10);
    }

    #[test]
    fn test_dig_sites_unique_names() {
        let mut names: Vec<&str> = DIG_SITES.iter().map(|s| s.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), DIG_SITES.len());
    }

    #[test]
    fn test_dig_sites_unique_maps() {
        let mut maps: Vec<&str> = DIG_SITES.iter().map(|s| s.map_name).collect();
        maps.sort();
        maps.dedup();
        assert_eq!(maps.len(), DIG_SITES.len());
    }

    #[test]
    fn test_dig_sites_level_valid() {
        for site in DIG_SITES {
            assert!(site.min_level > 0, "{}: level should be > 0", site.name);
            assert!(site.min_level <= 100, "{}: level should be <= 100", site.name);
        }
    }

    #[test]
    fn test_dig_sites_gold_range_valid() {
        for site in DIG_SITES {
            assert!(site.gold_min > 0, "{}: gold_min should be > 0", site.name);
            assert!(site.gold_max >= site.gold_min, "{}: gold_max < gold_min", site.name);
            assert!(site.exp > 0, "{}: exp should be > 0", site.name);
        }
    }

    #[test]
    fn test_dig_sites_drops_not_empty() {
        for site in DIG_SITES {
            assert!(!site.drops.is_empty(), "{}: drops should not be empty", site.name);
            assert!(
                !site.rare_drops.is_empty(),
                "{}: rare_drops should not be empty",
                site.name
            );
        }
    }

    #[test]
    fn test_dig_tools_count() {
        assert_eq!(DIG_TOOLS.len(), 6);
    }

    #[test]
    fn test_dig_tools_bonus_ascending() {
        for i in 1..DIG_TOOLS.len() {
            assert!(
                DIG_TOOLS[i].quality_bonus > DIG_TOOLS[i - 1].quality_bonus,
                "Tool bonus should be ascending"
            );
        }
    }

    #[test]
    fn test_parse_excavation_data_empty() {
        let (digs, ts, items) = parse_excavation_data("");
        assert_eq!(digs, 0);
        assert_eq!(ts, 0);
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_excavation_data_full() {
        let data = "digs=5;ts=1234567890;items=铜矿石,铁矿石,★强化石";
        let (digs, ts, items) = parse_excavation_data(data);
        assert_eq!(digs, 5);
        assert_eq!(ts, 1234567890);
        assert_eq!(items, vec!["铜矿石", "铁矿石", "★强化石"]);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let items = vec!["铜矿石".to_string(), "★强化石".to_string()];
        let data = serialize_excavation_data(3, 1000, &items);
        let (digs, ts, parsed_items) = parse_excavation_data(&data);
        assert_eq!(digs, 3);
        assert_eq!(ts, 1000);
        assert_eq!(parsed_items, items);
    }

    #[test]
    fn test_select_drop_basic() {
        let drops = &[("物品A", 50), ("物品B", 30), ("物品C", 20)];
        let result = select_drop(drops, 10);
        assert!(["物品A", "物品B", "物品C"].contains(&result));
    }

    #[test]
    fn test_select_drop_zero_weight() {
        let drops = &[("唯一物品", 1)];
        let result = select_drop(drops, 0);
        assert_eq!(result, "唯一物品");
    }

    #[test]
    fn test_deterministic_rand() {
        let r1 = deterministic_rand(12345, 0);
        let r2 = deterministic_rand(12345, 0);
        assert_eq!(r1, r2); // Same seed = same result

        let r3 = deterministic_rand(12345, 1);
        assert_ne!(r1, r3); // Different index = different result
    }

    #[test]
    fn test_daily_limit_constant() {
        assert_eq!(DAILY_DIG_LIMIT, 20);
    }

    #[test]
    fn test_cooldown_constant() {
        assert_eq!(DIG_COOLDOWN_SECS, 60);
    }
}

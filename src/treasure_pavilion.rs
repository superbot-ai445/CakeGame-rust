/// CakeGame 藏宝阁系统
///
/// 收藏品展示与交换中心。玩家可展示稀有收藏品，获得声望积分，在藏宝阁商店兑换珍贵道具。
///
/// 数据存储: Global 表 SECTION='treasure_pavilion'
use crate::db::Database;
use crate::user::get_msg_prefix;
use std::collections::HashMap;

/// 收藏品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectibleType {
    Equip,       // 装备
    Mount,       // 坐骑
    Divine,      // 神兵
    Rune,        // 符文
    Inscription, // 铭文
    Bloodline,   // 血脉
    Title,       // 称号
    Beast,       // 灵兽
}

impl CollectibleType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "装备" | "equip" => Some(Self::Equip),
            "坐骑" | "mount" => Some(Self::Mount),
            "神兵" | "divine" => Some(Self::Divine),
            "符文" | "rune" => Some(Self::Rune),
            "铭文" | "inscription" => Some(Self::Inscription),
            "血脉" | "bloodline" => Some(Self::Bloodline),
            "称号" | "title" => Some(Self::Title),
            "灵兽" | "beast" => Some(Self::Beast),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Equip => "装备",
            Self::Mount => "坐骑",
            Self::Divine => "神兵",
            Self::Rune => "符文",
            Self::Inscription => "铭文",
            Self::Bloodline => "血脉",
            Self::Title => "称号",
            Self::Beast => "灵兽",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Equip => "⚔️",
            Self::Mount => "🐎",
            Self::Divine => "🗡️",
            Self::Rune => "🔮",
            Self::Inscription => "📜",
            Self::Bloodline => "🩸",
            Self::Title => "👑",
            Self::Beast => "🐾",
        }
    }
}

/// 收藏品条目
#[derive(Debug, Clone)]
pub struct CollectibleItem {
    pub item_type: CollectibleType,
    pub name: String,
    pub rarity: u8,     // 1-5星稀有度
    pub prestige: u32,  // 声望积分
    pub source: String, // 获取来源
}

/// 藏宝阁数据
#[derive(Debug, Clone)]
pub struct TreasurePavilionData {
    pub showcases: Vec<CollectibleItem>, // 展示的收藏品
    pub prestige_points: u32,            // 累计声望积分
    pub daily_exchanges: u8,             // 每日交换次数
    pub exchange_history: Vec<String>,   // 交换历史
    pub total_rarity_score: u32,         // 总稀有度评分
}

impl TreasurePavilionData {
    pub fn new() -> Self {
        Self {
            showcases: Vec::new(),
            prestige_points: 0,
            daily_exchanges: 0,
            exchange_history: Vec::new(),
            total_rarity_score: 0,
        }
    }

    /// 计算展示柜评分
    pub fn showcase_score(&self) -> u32 {
        self.showcases.iter().map(|c| c.rarity as u32 * 100 + c.prestige).sum()
    }

    /// 展示柜是否已满（最多20个展示位）
    pub fn is_full(&self) -> bool {
        self.showcases.len() >= 20
    }

    /// 按类型统计
    pub fn count_by_type(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for item in &self.showcases {
            *counts.entry(item.item_type.display_name().to_string()).or_insert(0) += 1;
        }
        counts
    }
}

/// 藏宝阁等级
pub fn get_pavilion_level(score: u32) -> (u8, &'static str, &'static str) {
    match score {
        0..=499 => (1, "陋室藏珍", "🏠"),
        500..=1999 => (2, "雅阁纳宝", "🏛️"),
        2000..=4999 => (3, "宝殿聚珍", "🏰"),
        5000..=9999 => (4, "天宫宝库", "🌟"),
        10000..=19999 => (5, "神域珍藏", "✨"),
        20000..=49999 => (6, "混沌宝阁", "🌀"),
        _ => (7, "创世珍殿", "💎"),
    }
}

/// 藏宝阁商店商品
pub fn get_shop_items() -> Vec<(u32, &'static str, &'static str)> {
    vec![
        (50, "藏宝图碎片", "寻宝系统的必备材料"),
        (150, "鉴定水晶", "鉴定装备品质的稀有道具"),
        (300, "传承之书", "保留强化等级转移装备属性"),
        (500, "星辉精华", "星图系统的高级材料"),
        (800, "时空结晶", "时空裂隙的核心材料"),
        (1200, "凤凰之羽", "复活并恢复50%HP"),
        (2000, "命运转盘券", "免费转动一次命运转盘"),
        (3500, "传说宝箱", "随机开出传说级道具"),
        (5000, "神匠锤", "强化必定成功(1-10级)"),
        (8000, "创世之尘", "神兵铸造的顶级材料"),
        (12000, "至尊宝藏钥匙", "开启至尊宝藏获得极品奖励"),
        (20000, "藏宝阁至尊宝箱", "保底开出史诗级以上道具"),
    ]
}

/// 稀有度名称
pub fn rarity_name(rarity: u8) -> (&'static str, &'static str) {
    match rarity {
        1 => ("普通", "⬜"),
        2 => ("稀有", "🟢"),
        3 => ("珍贵", "🔵"),
        4 => ("极品", "🟣"),
        5 => ("传说", "🟠"),
        _ => ("未知", "❓"),
    }
}

// ==================== 指令处理器 ====================

/// 查看藏宝阁
pub fn cmd_view_treasure_pavilion(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }
    let data = load_pavilion_data(db, user_id);
    let score = data.showcase_score();
    let (level, level_name, icon) = get_pavilion_level(score);

    let mut resp = format!("{}\n{} === 藏宝阁 ===\n", prefix, icon);
    resp.push_str(&format!("等级: Lv.{} {}\n", level, level_name));
    resp.push_str(&format!("展示评分: {}\n", score));
    resp.push_str(&format!("声望积分: {}\n", data.prestige_points));
    resp.push_str(&format!("展示柜: {}/20\n\n", data.showcases.len()));

    if data.showcases.is_empty() {
        resp.push_str("📭 展示柜空空如也，快去收藏珍品吧！\n");
        resp.push_str("使用「藏宝阁展示 <类型> <名称>」展示收藏品\n");
    } else {
        resp.push_str("📦 展示柜:\n");
        for (i, item) in data.showcases.iter().enumerate() {
            let (rname, _ricon) = rarity_name(item.rarity);
            resp.push_str(&format!(
                "  {}. {} {} [{}] +{}声望\n",
                i + 1,
                item.item_type.icon(),
                item.name,
                rname,
                item.prestige
            ));
        }
        resp.push_str(&format!("\n总稀有度: ⭐{}\n", data.total_rarity_score));
    }

    let counts = data.count_by_type();
    if !counts.is_empty() {
        resp.push_str("\n📊 收藏统计: ");
        for (t, c) in &counts {
            resp.push_str(&format!("{}:{} ", t, c));
        }
    }

    resp.push_str("\n\n💡 指令: 藏宝阁展示/藏宝阁交换/藏宝阁排行/藏宝阁商店/购买藏宝/藏宝阁详情");
    resp
}

/// 展示收藏品
pub fn cmd_showcase_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return format!(
            "{}\n用法: 藏宝阁展示 <类型> <名称>\n类型: 装备/坐骑/神兵/符文/铭文/血脉/称号/灵兽",
            prefix
        );
    }
    let item_type_str = parts[0];
    let name = parts[1];
    let item_type = match CollectibleType::from_str(item_type_str) {
        Some(t) => t,
        None => {
            return format!(
                "{}\n未知收藏品类型: {}\n可选: 装备/坐骑/神兵/符文/铭文/血脉/称号/灵兽",
                prefix, item_type_str
            )
        }
    };

    let mut data = load_pavilion_data(db, user_id);

    if data.is_full() {
        return format!("{}\n展示柜已满(最多20个)，请先撤下一些收藏品", prefix);
    }

    if data.showcases.iter().any(|c| c.name == name) {
        return format!("{}\n「{}」已经在展示柜中了", prefix, name);
    }

    let (rarity, prestige) = match item_type {
        CollectibleType::Divine => (4, 200),
        CollectibleType::Bloodline => (4, 180),
        CollectibleType::Mount => (3, 120),
        CollectibleType::Rune => (3, 100),
        CollectibleType::Inscription => (3, 100),
        CollectibleType::Equip => (2, 60),
        CollectibleType::Title => (3, 150),
        CollectibleType::Beast => (3, 130),
    };

    let item = CollectibleItem {
        item_type,
        name: name.to_string(),
        rarity,
        prestige,
        source: "手动展示".to_string(),
    };

    data.showcases.push(item);
    data.total_rarity_score = data.showcases.iter().map(|c| c.rarity as u32).sum();
    data.prestige_points += prestige;

    save_pavilion_data(db, user_id, &data);

    let (rname, _ricon) = rarity_name(rarity);
    let mut resp = format!("{}\n✅ 展示成功！\n", prefix);
    resp.push_str(&format!("{} {} [{}] 已放入展示柜\n", item_type.icon(), name, rname));
    resp.push_str(&format!("获得 {} 声望积分\n", prestige));
    resp.push_str(&format!("当前总声望: {}", data.prestige_points));
    resp
}

/// 藏宝阁交换
pub fn cmd_exchange_treasure(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }
    let item_name = args.trim();
    if item_name.is_empty() {
        return format!(
            "{}\n用法: 藏宝阁交换 <商品名>\n使用「藏宝阁商店」查看可兑换商品",
            prefix
        );
    }

    let mut data = load_pavilion_data(db, user_id);

    if data.daily_exchanges >= 5 {
        return format!("{}\n今日交换次数已达上限(5次/天)", prefix);
    }

    let shop_items = get_shop_items();
    let target = match shop_items.iter().find(|(_, name, _)| *name == item_name) {
        Some(t) => t,
        None => {
            return format!(
                "{}\n未找到商品「{}」\n使用「藏宝阁商店」查看可兑换商品",
                prefix, item_name
            )
        }
    };

    let (cost, name, _desc) = target;
    if data.prestige_points < *cost {
        return format!("{}\n声望不足! 需要{}点，当前{}点", prefix, cost, data.prestige_points);
    }

    data.prestige_points -= cost;
    data.daily_exchanges += 1;
    data.exchange_history.push(format!("兑换{}", name));
    if data.exchange_history.len() > 20 {
        data.exchange_history.remove(0);
    }

    save_pavilion_data(db, user_id, &data);

    let mut resp = format!("{}\n✅ 兑换成功！\n", prefix);
    resp.push_str(&format!("获得: {}\n", name));
    resp.push_str(&format!("消耗: {} 声望积分\n", cost));
    resp.push_str(&format!("剩余声望: {}\n", data.prestige_points));
    resp.push_str(&format!("今日交换: {}/5", data.daily_exchanges));
    resp
}

/// 藏宝阁商店
pub fn cmd_treasure_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    let items = get_shop_items();
    let mut resp = format!("{}\n🏪 === 藏宝阁商店 ===\n\n", prefix);
    resp.push_str("用声望积分兑换珍贵道具:\n\n");

    for (i, (cost, name, desc)) in items.iter().enumerate() {
        resp.push_str(&format!("{}. {} - {}声望\n   {}\n", i + 1, name, cost, desc));
    }

    resp.push_str("\n💡 指令: 购买藏宝 <商品名>");
    resp.push_str("\n📌 每日限兑换5次");
    resp
}

/// 藏宝阁排行
pub fn cmd_treasure_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    let rankings = get_pavilion_rankings(db);

    let mut resp = format!("{}\n🏆 === 藏宝阁排行 ===\n\n", prefix);

    if rankings.is_empty() {
        resp.push_str("暂无排行数据\n");
    } else {
        for (i, (name, score, prestige)) in rankings.iter().enumerate() {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            let (level, level_name, _) = get_pavilion_level(*score);
            resp.push_str(&format!(
                "{} {}. {} - Lv.{} {} 评分:{} 声望:{}\n",
                medal,
                i + 1,
                name,
                level,
                level_name,
                score,
                prestige
            ));
        }
    }

    resp
}

/// 藏宝阁详情
pub fn cmd_treasure_detail(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作。", prefix);
    }
    let data = load_pavilion_data(db, user_id);
    let score = data.showcase_score();
    let (level, level_name, icon) = get_pavilion_level(score);

    let next_threshold = match level {
        1 => 500,
        2 => 2000,
        3 => 5000,
        4 => 10000,
        5 => 20000,
        6 => 50000,
        _ => 0,
    };

    let mut resp = format!("{}\n{} === 藏宝阁详情 ===\n\n", prefix, icon);
    resp.push_str(&format!("等级: Lv.{} {}\n", level, level_name));
    resp.push_str(&format!("展示评分: {}\n", score));
    resp.push_str(&format!("声望积分: {}\n", data.prestige_points));
    resp.push_str(&format!("展示柜: {}/20\n\n", data.showcases.len()));

    if next_threshold > 0 {
        let progress = (score as f64 / next_threshold as f64 * 100.0).min(100.0);
        let filled = (progress / 5.0) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
        resp.push_str(&format!("升级进度: [{}] {:.0}%\n", bar, progress));
        resp.push_str(&format!("下一级需要: {} 评分\n\n", next_threshold));
    } else {
        resp.push_str("已达最高等级! 🎉\n\n");
    }

    let counts = data.count_by_type();
    resp.push_str("📊 收藏分类:\n");
    for ct in &[
        CollectibleType::Equip,
        CollectibleType::Mount,
        CollectibleType::Divine,
        CollectibleType::Rune,
        CollectibleType::Inscription,
        CollectibleType::Bloodline,
        CollectibleType::Title,
        CollectibleType::Beast,
    ] {
        let count = counts.get(ct.display_name()).copied().unwrap_or(0);
        resp.push_str(&format!("  {} {}: {}件\n", ct.icon(), ct.display_name(), count));
    }

    resp.push_str(&format!("\n📈 每日交换: {}/5次\n", data.daily_exchanges));

    if !data.exchange_history.is_empty() {
        resp.push_str("\n📋 最近交换:\n");
        for h in data.exchange_history.iter().rev().take(5) {
            resp.push_str(&format!("  · {}\n", h));
        }
    }

    resp
}

// ==================== 数据持久化 ====================

const SECTION: &str = "treasure_pavilion";

fn load_pavilion_data(db: &Database, user_id: &str) -> TreasurePavilionData {
    let json_str = db.global_get(SECTION, &format!("u{}", user_id));

    if json_str.is_empty() {
        return TreasurePavilionData::new();
    }

    let mut data = TreasurePavilionData::new();

    // 解析prestige_points
    if let Some(val) = extract_json_value(&json_str, "p") {
        data.prestige_points = val.parse().unwrap_or(0);
    }
    if let Some(val) = extract_json_value(&json_str, "de") {
        data.daily_exchanges = val.parse().unwrap_or(0);
    }
    if let Some(val) = extract_json_value(&json_str, "tr") {
        data.total_rarity_score = val.parse().unwrap_or(0);
    }

    // 解析展示柜
    if let Some(showcases_str) = extract_json_value(&json_str, "s") {
        for entry in showcases_str.split('|') {
            if entry.is_empty() {
                continue;
            }
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() >= 4 {
                let item_type = match parts[0] {
                    "eq" => CollectibleType::Equip,
                    "mo" => CollectibleType::Mount,
                    "di" => CollectibleType::Divine,
                    "ru" => CollectibleType::Rune,
                    "in" => CollectibleType::Inscription,
                    "bl" => CollectibleType::Bloodline,
                    "ti" => CollectibleType::Title,
                    "be" => CollectibleType::Beast,
                    _ => continue,
                };
                data.showcases.push(CollectibleItem {
                    item_type,
                    name: parts[1].to_string(),
                    rarity: parts[2].parse().unwrap_or(1),
                    prestige: parts[3].parse().unwrap_or(0),
                    source: if parts.len() > 4 {
                        parts[4].to_string()
                    } else {
                        String::new()
                    },
                });
            }
        }
    }

    // 解析交换历史
    if let Some(history_str) = extract_json_value(&json_str, "h") {
        data.exchange_history = history_str
            .split('|')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
    }

    data
}

fn save_pavilion_data(db: &Database, user_id: &str, data: &TreasurePavilionData) {
    let showcases_str: String = data
        .showcases
        .iter()
        .map(|c| {
            let type_str = match c.item_type {
                CollectibleType::Equip => "eq",
                CollectibleType::Mount => "mo",
                CollectibleType::Divine => "di",
                CollectibleType::Rune => "ru",
                CollectibleType::Inscription => "in",
                CollectibleType::Bloodline => "bl",
                CollectibleType::Title => "ti",
                CollectibleType::Beast => "be",
            };
            format!("{}:{}:{}:{}:{}", type_str, c.name, c.rarity, c.prestige, c.source)
        })
        .collect::<Vec<_>>()
        .join("|");

    let history_str = data.exchange_history.join("|");

    let json = format!(
        r#"{{"p":{},"de":{},"tr":{},"s":"{}","h":"{}"}}"#,
        data.prestige_points, data.daily_exchanges, data.total_rarity_score, showcases_str, history_str
    );

    db.global_set(SECTION, &format!("u{}", user_id), &json);
}

fn get_pavilion_rankings(db: &Database) -> Vec<(String, u32, u32)> {
    // Scan known user sections for rankings
    let mut rankings: Vec<(String, u32, u32)> = Vec::new();

    // We iterate over recent user IDs from Basic_User
    // Use a simple approach: scan global entries
    for i in 0..10000 {
        let key = format!("u{}", i);
        let value = db.global_get(SECTION, &key);
        if value.is_empty() {
            continue;
        }

        let score: u32 = extract_json_value(&value, "tr")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let prestige: u32 = extract_json_value(&value, "p")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        if score > 0 || prestige > 0 {
            let username = db.read_basic(&i.to_string(), "昵称").replace('"', "");
            let display_name = if username.is_empty() {
                format!("玩家{}", i)
            } else {
                username
            };
            rankings.push((display_name, score + prestige, prestige));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));
    rankings.truncate(15);
    rankings
}

fn extract_json_value(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];

    // Handle quoted string values
    if rest.starts_with('"') {
        let end = rest.strip_prefix('"')?.find('"')?;
        return Some(rest[1..end + 1].to_string());
    }

    // Handle numeric values
    let end = rest.find([',', '}', ' ']).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pavilion_levels() {
        assert_eq!(get_pavilion_level(0).0, 1);
        assert_eq!(get_pavilion_level(500).0, 2);
        assert_eq!(get_pavilion_level(2000).0, 3);
        assert_eq!(get_pavilion_level(5000).0, 4);
        assert_eq!(get_pavilion_level(10000).0, 5);
        assert_eq!(get_pavilion_level(20000).0, 6);
        assert_eq!(get_pavilion_level(50000).0, 7);
    }

    #[test]
    fn test_collectible_types() {
        assert_eq!(CollectibleType::from_str("装备"), Some(CollectibleType::Equip));
        assert_eq!(CollectibleType::from_str("坐骑"), Some(CollectibleType::Mount));
        assert_eq!(CollectibleType::from_str("神兵"), Some(CollectibleType::Divine));
        assert_eq!(CollectibleType::from_str("invalid"), None);
    }

    #[test]
    fn test_rarity_names() {
        assert_eq!(rarity_name(1).0, "普通");
        assert_eq!(rarity_name(5).0, "传说");
        assert_eq!(rarity_name(3).1, "🔵");
    }

    #[test]
    fn test_treasure_data() {
        let mut data = TreasurePavilionData::new();
        assert!(!data.is_full());
        assert_eq!(data.showcase_score(), 0);

        data.showcases.push(CollectibleItem {
            item_type: CollectibleType::Divine,
            name: "炎龙之怒".to_string(),
            rarity: 4,
            prestige: 200,
            source: "测试".to_string(),
        });

        assert_eq!(data.showcase_score(), 600); // 4*100 + 200
        assert_eq!(data.count_by_type().get("神兵"), Some(&1));
    }

    #[test]
    fn test_pavilion_full() {
        let mut data = TreasurePavilionData::new();
        for i in 0..20 {
            data.showcases.push(CollectibleItem {
                item_type: CollectibleType::Equip,
                name: format!("装备{}", i),
                rarity: 1,
                prestige: 10,
                source: "test".to_string(),
            });
        }
        assert!(data.is_full());
    }

    #[test]
    fn test_shop_items() {
        let items = get_shop_items();
        assert!(items.len() >= 10);
        assert!(items.iter().all(|(cost, _, _)| *cost > 0));
    }

    #[test]
    fn test_collectible_type_display() {
        assert_eq!(CollectibleType::Equip.display_name(), "装备");
        assert_eq!(CollectibleType::Mount.icon(), "🐎");
        assert_eq!(CollectibleType::Divine.icon(), "🗡️");
        assert_eq!(CollectibleType::Bloodline.icon(), "🩸");
    }

    #[test]
    fn test_extract_json_value() {
        let json = r#"{"p":100,"name":"test","s":"abc|def"}"#;
        assert_eq!(extract_json_value(json, "p"), Some("100".to_string()));
        assert_eq!(extract_json_value(json, "s"), Some("abc|def".to_string()));
        assert_eq!(extract_json_value(json, "missing"), None);
    }

    #[test]
    fn test_level_progress() {
        let (level, name, icon) = get_pavilion_level(2500);
        assert_eq!(level, 3);
        assert_eq!(name, "宝殿聚珍");
        assert_eq!(icon, "🏰");
    }
}

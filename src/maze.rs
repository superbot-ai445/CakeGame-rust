/// CakeGame 幻境迷宫系统
/// 程序化生成的迷宫副本，每天基于日期哈希生成独特布局
/// 玩家在岔路口选择路径，遭遇战斗/宝箱/陷阱/商店/治疗/Boss
/// 5层递进难度，到达中心获得终极奖励
///
/// 存储: Global 表, section = 'illusion_maze'
///   - {uid}:current = 玩家当前迷宫状态 JSON
///   - {uid}:history = 完成记录 JSON
///   - ranking = 全服排行数据
use crate::core::*;
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 每日进入上限
const DAILY_LIMIT: i32 = 3;
/// 最大层数
const MAX_FLOORS: i32 = 5;
/// 每层节点数
const NODES_PER_FLOOR: i32 = 4;
/// 迷宫section
const SECTION: &str = "illusion_maze";

/// 节点事件类型
#[derive(Debug, Clone, PartialEq)]
enum MazeEvent {
    Combat,   // 战斗
    Treasure, // 宝箱
    Trap,     // 陷阱
    Shop,     // 商店
    Healing,  // 治疗
    Boss,     // 层主
    Empty,    // 空房间
    Start,    // 起点
    Exit,     // 出口
}

impl MazeEvent {
    fn emoji(&self) -> &str {
        match self {
            MazeEvent::Combat => "⚔️",
            MazeEvent::Treasure => "🎁",
            MazeEvent::Trap => "💀",
            MazeEvent::Shop => "🏪",
            MazeEvent::Healing => "💊",
            MazeEvent::Boss => "🐉",
            MazeEvent::Empty => "🔲",
            MazeEvent::Start => "🏁",
            MazeEvent::Exit => "🏆",
        }
    }
    #[allow(dead_code)]
    fn name(&self) -> &str {
        match self {
            MazeEvent::Combat => "怪物伏击",
            MazeEvent::Treasure => "神秘宝箱",
            MazeEvent::Trap => "致命陷阱",
            MazeEvent::Shop => "迷宫商人",
            MazeEvent::Healing => "治疗泉",
            MazeEvent::Boss => "层主守卫",
            MazeEvent::Empty => "空房间",
            MazeEvent::Start => "入口",
            MazeEvent::Exit => "出口",
        }
    }
}

/// 迷宫节点
#[allow(dead_code)]
struct MazeNode {
    event: MazeEvent,
    floor: i32,
    index: i32,
}

/// 迷宫状态
#[derive(Debug, Clone)]
struct MazeState {
    floor: i32,       // 当前层 (1-5)
    node: i32,        // 当前节点 (0-3)
    hp_pct: i32,      // 当前HP百分比
    gold_earned: i64, // 累计获得金币
    exp_earned: i32,  // 累计获得经验
    items_found: i32, // 找到物品数
    #[allow(dead_code)]
    events_log: String, // 事件日志
    completed: bool,  // 是否通关
}

impl MazeState {
    fn new() -> Self {
        MazeState {
            floor: 1,
            node: 0,
            hp_pct: 100,
            gold_earned: 0,
            exp_earned: 0,
            items_found: 0,
            events_log: String::new(),
            completed: false,
        }
    }

    fn to_json(&self) -> String {
        format!(
            r#"{{"floor":{},"node":{},"hp":{},"gold":{},"exp":{},"items":{},"completed":{}}}"#,
            self.floor, self.node, self.hp_pct, self.gold_earned, self.exp_earned, self.items_found, self.completed
        )
    }

    fn from_json(json: &str) -> Option<Self> {
        // Simple JSON parsing without serde
        let get_val = |key: &str| -> String {
            let pattern = format!("\"{}\":", key);
            if let Some(start) = json.find(&pattern) {
                let after = &json[start + pattern.len()..];
                let end = after.find([',', '}']).unwrap_or(after.len());
                after[..end].trim().to_string()
            } else {
                String::new()
            }
        };
        let floor: i32 = get_val("floor").parse().unwrap_or(1);
        let node: i32 = get_val("node").parse().unwrap_or(0);
        let hp: i32 = get_val("hp").parse().unwrap_or(100);
        let gold: i64 = get_val("gold").parse().unwrap_or(0);
        let exp: i32 = get_val("exp").parse().unwrap_or(0);
        let items: i32 = get_val("items").parse().unwrap_or(0);
        let completed = get_val("completed") == "true";
        Some(MazeState {
            floor,
            node,
            hp_pct: hp,
            gold_earned: gold,
            exp_earned: exp,
            items_found: items,
            events_log: String::new(),
            completed,
        })
    }
}

/// 今日日期字符串
fn today_str() -> String {
    let now = chrono::Local::now();
    now.format("%Y-%m-%d").to_string()
}

/// 确定性哈希
fn maze_hash(user_id: &str, floor: i32, node: i32, date: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("maze_{}_{}_{}_{}", user_id, floor, node, date).hash(&mut hasher);
    hasher.finish()
}

/// 根据哈希选择事件类型
fn select_event(hash: u64, floor: i32, node: i32, total_nodes: i32) -> MazeEvent {
    // 最后一个节点是出口或Boss
    if floor == MAX_FLOORS && node == total_nodes - 1 {
        return MazeEvent::Exit;
    }
    if node == total_nodes - 1 && floor < MAX_FLOORS {
        return MazeEvent::Boss;
    }
    if node == 0 {
        return MazeEvent::Start;
    }

    // 权重选择
    let weights: [(MazeEvent, u32); 6] = [
        (MazeEvent::Combat, 30),
        (MazeEvent::Treasure, 15),
        (MazeEvent::Trap, 15),
        (MazeEvent::Shop, 10),
        (MazeEvent::Healing, 15),
        (MazeEvent::Empty, 15),
    ];
    let total: u32 = weights.iter().map(|(_, w)| w).sum();
    let bucket = (hash % total as u64) as u32;
    let mut acc = 0u32;
    for (event, weight) in &weights {
        acc += weight;
        if bucket < acc {
            return event.clone();
        }
    }
    MazeEvent::Empty
}

/// 层数怪物等级
fn floor_monster_level(floor: i32) -> i32 {
    5 + (floor - 1) * 10
}

/// 层数怪物名
fn floor_monster_name(floor: i32, hash: u64) -> &'static str {
    let monsters: &[&[&str]] = &[
        &["迷宫蝙蝠", "暗影鼠", "石像小鬼", "毒蘑菇"],
        &["迷宫守卫", "暗影骑士", "石魔像", "毒蛇"],
        &["深渊恶魔", "暗影领主", "远古石像", "剧毒花"],
        &["虚空行者", "幽灵将军", "泰坦碎片", "瘟疫使者"],
        &["迷宫之王", "暗影帝王", "远古守护者", "混沌化身"],
    ];
    let idx = (hash as usize) % monsters[(floor - 1) as usize].len();
    monsters[(floor - 1) as usize][idx]
}

/// 生成迷宫地图显示
fn render_maze_map(state: &MazeState) -> String {
    let mut out = String::new();
    out.push_str("\n🗺️ ━━━━━━ 幻境迷宫地图 ━━━━━━\n");
    for f in 1..=MAX_FLOORS {
        let marker = if f == state.floor {
            "▶"
        } else if f < state.floor {
            "✅"
        } else {
            " "
        };
        out.push_str(&format!("{} 第{}层 {}\n", marker, f, floor_label(f)));
    }
    out.push_str(&format!("\n📍 当前: 第{}层 第{}个房间\n", state.floor, state.node));
    out
}

fn floor_label(floor: i32) -> &'static str {
    match floor {
        1 => "幽暗入口",
        2 => "回响走廊",
        3 => "深渊大厅",
        4 => "虚空回廊",
        5 => "混沌核心",
        _ => "未知",
    }
}

/// HP进度条
fn hp_bar(pct: i32) -> String {
    let filled = (pct / 10).clamp(0, 10) as usize;
    let empty = 10 - filled;
    format!("{}{} {}%", "█".repeat(filled), "░".repeat(empty), pct)
}

/// 事件结果
struct EventResult {
    message: String,
    hp_change: i32,
    gold: i64,
    exp: i32,
    item: Option<String>,
}

/// 处理战斗事件
fn process_combat(db: &Database, floor: i32, user_id: &str, hash: u64) -> EventResult {
    let monster = floor_monster_name(floor, hash);
    let level = floor_monster_level(floor);
    let info = user::calc_total_attrs(db, user_id);
    let player_power = info.ad + info.ap + info.defense;
    let monster_power = level * 20 + floor * 30;
    let win = player_power > monster_power || (hash % 100) < 60;

    if win {
        let gold: i64 = 50 + floor as i64 * 30 + (hash % 50) as i64;
        let exp: i32 = 20 + floor * 15 + (hash % 20) as i32;
        EventResult {
            message: format!("⚔️ 遭遇 {} (Lv.{})！\n你奋勇战斗，击败了敌人！", monster, level),
            hp_change: -(5 + floor * 3 + (hash % 10) as i32),
            gold,
            exp,
            item: None,
        }
    } else {
        EventResult {
            message: format!("⚔️ 遭遇 {} (Lv.{})！\n战斗失败，你受到重创...", monster, level),
            hp_change: -(15 + floor * 5 + (hash % 15) as i32),
            gold: 0,
            exp: 5,
            item: None,
        }
    }
}

/// 处理宝箱事件
fn process_treasure(floor: i32, hash: u64) -> EventResult {
    let gold: i64 = 100 + floor as i64 * 50 + (hash % 100) as i64;
    let items = ["强化石", "小瓶药水", "传送卷轴", "复活卷轴", "高级药水"];
    let item_idx = (hash as usize) % items.len();
    let has_item = (hash % 100) < 50;

    EventResult {
        message: if has_item {
            format!("🎁 发现神秘宝箱！获得 {} 金币和 {}", gold, items[item_idx])
        } else {
            format!("🎁 发现神秘宝箱！获得 {} 金币", gold)
        },
        hp_change: 0,
        gold,
        exp: 10 + floor * 5,
        item: if has_item {
            Some(items[item_idx].to_string())
        } else {
            None
        },
    }
}

/// 处理陷阱事件
fn process_trap(floor: i32, hash: u64) -> EventResult {
    let dmg: i32 = 10 + floor * 5 + (hash % 10) as i32;
    EventResult {
        message: format!("💀 触发陷阱！受到 {} 点伤害", dmg),
        hp_change: -dmg,
        gold: 0,
        exp: 5,
        item: None,
    }
}

/// 处理治疗事件
fn process_healing(hash: u64) -> EventResult {
    let heal: i32 = 30 + (hash % 30) as i32;
    EventResult {
        message: format!("💊 发现治疗泉！恢复 {}% 生命", heal),
        hp_change: heal,
        gold: 0,
        exp: 5,
        item: None,
    }
}

/// 处理Boss事件
fn process_boss(floor: i32, hash: u64) -> EventResult {
    let boss_names = ["幽暗领主", "回响守卫", "深渊之眼", "虚空行者", "混沌之王"];
    let boss = boss_names[(floor - 1) as usize];
    let gold: i64 = 200 + floor as i64 * 100;
    let exp = 50 + floor * 30;

    // Boss战有一定难度
    let win = (hash % 100) < 70;
    if win {
        EventResult {
            message: format!(
                "🐉 层主 {} 现身！\n激烈战斗后，你击败了层主！\n💰 获得 {} 金币 + {} 经验",
                boss, gold, exp
            ),
            hp_change: -(20 + floor * 8),
            gold,
            exp,
            item: Some("层主宝箱".to_string()),
        }
    } else {
        EventResult {
            message: format!("🐉 层主 {} 现身！\n你被层主击退... 勇气可嘉！", boss),
            hp_change: -(30 + floor * 10),
            gold: 50,
            exp: 15,
            item: None,
        }
    }
}

/// 保存迷宫状态
fn save_state(db: &Database, user_id: &str, state: &MazeState) {
    let section = format!("{}_{}", SECTION, user_id);
    db.global_set(&section, "current", &state.to_json());
}

/// 加载迷宫状态
fn load_state(db: &Database, user_id: &str) -> Option<MazeState> {
    let section = format!("{}_{}", SECTION, user_id);
    let json = db.global_get(&section, "current");
    if json.is_empty() {
        None
    } else {
        MazeState::from_json(&json)
    }
}

/// 今日进入次数
fn daily_entries(db: &Database, user_id: &str) -> i32 {
    let section = format!("{}_{}", SECTION, user_id);
    let date = db.global_get(&section, "entry_date");
    if date == today_str() {
        db.global_get(&section, "entry_count").parse().unwrap_or(0)
    } else {
        0
    }
}

/// 记录进入次数
fn record_entry(db: &Database, user_id: &str) {
    let section = format!("{}_{}", SECTION, user_id);
    let today = today_str();
    let date = db.global_get(&section, "entry_date");
    let count = if date == today {
        db.global_get(&section, "entry_count").parse::<i32>().unwrap_or(0) + 1
    } else {
        1
    };
    db.global_set(&section, "entry_date", &today);
    db.global_set(&section, "entry_count", &count.to_string());
}

// ==================== 公共API ====================

/// 查看迷宫
pub fn cmd_view_maze(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let entries = daily_entries(db, user_id);
    let mut out = String::new();
    out.push_str(&format!("{}\n", prefix));
    out.push_str("🏰 ━━━━━━ 幻境迷宫 ━━━━━━\n\n");
    out.push_str("幻境迷宫每日自动重置，探索5层迷宫\n");
    out.push_str("在岔路口选择路径，遭遇各种事件\n");
    out.push_str("击败最终层主即可通关！\n\n");
    out.push_str(&format!("📊 每日进入: {}/{} 次\n", entries, DAILY_LIMIT));
    out.push_str(&format!("🏔️ 最大层数: {} 层\n", MAX_FLOORS));
    out.push_str(&format!("📍 每层节点: {} 个房间\n", NODES_PER_FLOOR));
    out.push_str("\n🎮 事件类型:\n");
    out.push_str("  ⚔️ 怪物伏击 - 战斗击败获得奖励\n");
    out.push_str("  🎁 神秘宝箱 - 金币+随机道具\n");
    out.push_str("  💀 致命陷阱 - 受到伤害\n");
    out.push_str("  🏪 迷宫商人 - 可购买道具\n");
    out.push_str("  💊 治疗泉 - 恢复生命\n");
    out.push_str("  🐉 层主守卫 - 击败后进入下一层\n");
    out.push_str("  🏆 最终出口 - 通关迷宫\n\n");

    // 检查是否有进行中的迷宫
    if let Some(state) = load_state(db, user_id) {
        if !state.completed {
            out.push_str("📍 进行中的迷宫:\n");
            out.push_str(&render_maze_map(&state));
            out.push_str(&format!("❤️ 生命: {}\n", hp_bar(state.hp_pct)));
            out.push_str(&format!(
                "💰 累计: {}金 / {}经验\n",
                state.gold_earned, state.exp_earned
            ));
            out.push_str("\n💡 使用「选择路径+左/右」继续探索\n");
        }
    } else {
        out.push_str("💡 使用「进入迷宫」开始探索\n");
    }

    out
}

/// 进入迷宫
pub fn cmd_enter_maze(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // 虚弱检查
    let weak = user::check_weakness(db, user_id);
    if weak > 0 {
        return format!("{}\n❌ 您正处于虚弱状态（剩余{}秒），无法进入迷宫", prefix, weak);
    }

    // 每日限制
    let entries = daily_entries(db, user_id);
    if entries >= DAILY_LIMIT {
        return format!("{}\n❌ 今日进入迷宫已达上限 ({}/{})", prefix, entries, DAILY_LIMIT);
    }

    // 检查进行中的迷宫
    if let Some(state) = load_state(db, user_id) {
        if !state.completed && state.hp_pct > 0 {
            return format!(
                "{}\n❌ 您已有进行中的迷宫，请先完成或放弃\n💡 使用「选择路径+左/右」继续探索",
                prefix
            );
        }
    }

    // 等级门槛
    let info = user::calc_total_attrs(db, user_id);
    if info.level < 10 {
        return format!("{}\n❌ 进入迷宫需要至少10级（当前{}级）", prefix, info.level);
    }

    // 创建新迷宫
    let state = MazeState::new();
    save_state(db, user_id, &state);
    record_entry(db, user_id);

    let date = today_str();
    let hash = maze_hash(user_id, 1, 0, &date);

    let mut out = String::new();
    out.push_str(&format!("{}\n", prefix));
    out.push_str("🏰 ━━━━━━ 进入幻境迷宫 ━━━━━━\n\n");
    out.push_str(&format!("📅 迷宫编号: {:016X}\n", hash));
    out.push_str(&format!("🏔️ 第1层: {}\n\n", floor_label(1)));
    out.push_str("🏁 你站在迷宫入口\n");
    out.push_str("前方有两条岔路...\n\n");
    out.push_str("💡 使用「选择路径+左」或「选择路径+右」前进\n");
    out.push_str(&format!("❤️ 生命: {}\n", hp_bar(100)));
    out.push_str(&format!("📊 今日进入: {}/{} 次\n", entries + 1, DAILY_LIMIT));
    out
}

/// 选择路径
pub fn cmd_choose_path(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let mut state = match load_state(db, user_id) {
        Some(s) if !s.completed && s.hp_pct > 0 => s,
        Some(s) if s.completed => {
            return format!("{}\n✅ 迷宫已通关！使用「进入迷宫」开始新的挑战", prefix);
        }
        _ => {
            return format!("{}\n❌ 没有进行中的迷宫\n💡 使用「进入迷宫」开始探索", prefix);
        }
    };

    // 解析方向
    let direction = args.trim();
    let _dir_num = match direction {
        "左" | "left" | "1" => 0,
        "右" | "right" | "2" => 1,
        "直" | "straight" | "3" => 2,
        _ => {
            return format!(
                "{}\n❌ 请选择方向: 左/右/直\n💡 使用「选择路径+左」或「选择路径+右」",
                prefix
            );
        }
    };

    // 移动到下一个节点
    state.node += 1;
    if state.node >= NODES_PER_FLOOR {
        // 进入下一层
        state.floor += 1;
        state.node = 1; // 跳过起始节点
        if state.floor > MAX_FLOORS {
            // 通关！
            state.completed = true;
            state.floor = MAX_FLOORS;
            save_state(db, user_id, &state);

            // 发放通关奖励
            let bonus_gold = state.gold_earned / 5;
            let bonus_exp = state.exp_earned / 5;
            db.modify_currency(user_id, CURRENCY_GOLD, "add", bonus_gold + 500);
            crate::user::add_experience(db, user_id, bonus_exp + 200);

            // 记录排行
            let section = format!("{}_ranking", SECTION);
            let today = today_str();
            let rank_key = format!("{}_{}", user_id, today);
            db.global_set(
                &section,
                &rank_key,
                &format!("{}|{}|{}", user_id, state.hp_pct, state.gold_earned),
            );

            let mut out = format!("{}\n", prefix);
            out.push_str("🏆 ━━━━━━ 迷宫通关！━━━━━━\n\n");
            out.push_str("恭喜你击败最终层主，征服了幻境迷宫！\n\n");
            out.push_str("📊 探险统计:\n");
            out.push_str(&format!("  ❤️ 剩余生命: {}%\n", state.hp_pct));
            out.push_str(&format!("  💰 累计获得: {} 金币\n", state.gold_earned));
            out.push_str(&format!("  ⭐ 累计经验: {}\n", state.exp_earned));
            out.push_str(&format!("  🎁 找到物品: {} 件\n\n", state.items_found));
            out.push_str("🎁 通关额外奖励:\n");
            out.push_str(&format!("  💰 {} 金币\n", bonus_gold + 500));
            out.push_str(&format!("  ⭐ {} 经验\n", bonus_exp + 200));
            out.push_str("\n💡 使用「迷宫排行」查看全服排名");
            return out;
        }
    }

    let date = today_str();
    let hash = maze_hash(user_id, state.floor, state.node, &date);
    let event = select_event(hash, state.floor, state.node, NODES_PER_FLOOR);

    let result = match &event {
        MazeEvent::Combat => process_combat(db, state.floor, user_id, hash),
        MazeEvent::Treasure => process_treasure(state.floor, hash),
        MazeEvent::Trap => process_trap(state.floor, hash),
        MazeEvent::Healing => process_healing(hash),
        MazeEvent::Boss => process_boss(state.floor, hash),
        MazeEvent::Shop => EventResult {
            message: "🏪 迷宫商人出现了！\n你浏览了他的商品，但似乎没有合适的...".to_string(),
            hp_change: 0,
            gold: 0,
            exp: 5,
            item: None,
        },
        MazeEvent::Start => EventResult {
            message: format!("🏁 进入第{}层: {}", state.floor, floor_label(state.floor)),
            hp_change: 0,
            gold: 0,
            exp: 0,
            item: None,
        },
        MazeEvent::Empty | MazeEvent::Exit => EventResult {
            message: "🔲 空荡荡的房间，继续前进吧".to_string(),
            hp_change: 0,
            gold: 0,
            exp: 3,
            item: None,
        },
    };

    // 应用结果
    state.hp_pct = (state.hp_pct + result.hp_change).clamp(0, 100);
    state.gold_earned += result.gold;
    state.exp_earned += result.exp;
    if result.item.is_some() {
        state.items_found += 1;
    }

    // 实际发放奖励
    if result.gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", result.gold);
    }
    if result.exp > 0 {
        crate::user::add_experience(db, user_id, result.exp);
    }
    if let Some(ref item) = result.item {
        let _ = db.add_item(user_id, item, 1);
    }

    // 保存状态
    save_state(db, user_id, &state);

    // 检查死亡
    if state.hp_pct <= 0 {
        // 死亡 — 给予部分奖励并结束
        let mut out = format!("{}\n", prefix);
        out.push_str("💀 ━━━━━━ 迷宫失败 ━━━━━━\n\n");
        out.push_str(&format!("{}\n\n", result.message));
        out.push_str("你的生命耗尽，被传送回城...\n\n");
        out.push_str("📊 探险统计:\n");
        out.push_str(&format!("  🏔️ 到达: 第{}层 第{}房间\n", state.floor, state.node));
        out.push_str(&format!("  💰 获得: {} 金币\n", state.gold_earned));
        out.push_str(&format!("  ⭐ 获得: {} 经验\n", state.exp_earned));
        out.push_str("\n💡 使用「进入迷宫」重新挑战");
        return out;
    }

    // 正常显示
    let mut out = format!("{}\n", prefix);
    out.push_str(&format!(
        "🏰 第{}层 {} - {}\n\n",
        state.floor,
        floor_label(state.floor),
        event.emoji()
    ));
    out.push_str(&format!("{}\n\n", result.message));
    out.push_str(&format!("❤️ 生命: {}\n", hp_bar(state.hp_pct)));
    out.push_str(&format!(
        "💰 累计: {}金 / {}经验\n",
        state.gold_earned, state.exp_earned
    ));

    // 显示下一步提示
    if event == MazeEvent::Boss || event == MazeEvent::Exit {
        out.push_str(&format!("\n⬆️ 进入第{}层！\n", state.floor));
    }
    out.push_str("\n💡 使用「选择路径+左/右」继续探索\n");

    out
}

/// 迷宫进度
pub fn cmd_maze_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let entries = daily_entries(db, user_id);
    let mut out = format!("{}\n", prefix);
    out.push_str("📊 ━━━━━━ 迷宫进度 ━━━━━━\n\n");
    out.push_str(&format!("📅 今日进入: {}/{} 次\n\n", entries, DAILY_LIMIT));

    match load_state(db, user_id) {
        Some(state) if !state.completed => {
            out.push_str(&render_maze_map(&state));
            out.push_str(&format!("❤️ 生命: {}\n", hp_bar(state.hp_pct)));
            out.push_str(&format!("💰 累计金币: {}\n", state.gold_earned));
            out.push_str(&format!("⭐ 累计经验: {}\n", state.exp_earned));
            out.push_str(&format!("🎁 找到物品: {} 件\n", state.items_found));
            if state.hp_pct <= 0 {
                out.push_str("\n💀 迷宫失败，使用「进入迷宫」重新挑战\n");
            }
        }
        Some(state) if state.completed => {
            out.push_str("✅ 今日迷宫已通关！\n\n");
            out.push_str(&format!("  ❤️ 剩余生命: {}%\n", state.hp_pct));
            out.push_str(&format!("  💰 总获得: {} 金币\n", state.gold_earned));
            out.push_str(&format!("  ⭐ 总经验: {}\n", state.exp_earned));
            out.push_str(&format!("  🎁 物品: {} 件\n", state.items_found));
            out.push_str("\n💡 使用「进入迷宫」开始新挑战\n");
        }
        _ => {
            out.push_str("❌ 没有进行中的迷宫\n");
            out.push_str("💡 使用「进入迷宫」开始探索\n");
        }
    }

    out
}

/// 迷宫排行
pub fn cmd_maze_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let section = format!("{}_ranking", SECTION);
    let today = today_str();

    // 收集今日排行数据
    let mut entries: Vec<(String, i32, i64)> = Vec::new();
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(&format!("SELECT ID, DATA FROM Global WHERE SECTION = '{}'", section)) {
        Ok(s) => s,
        Err(_) => {
            return format!("{}\n❌ 查询失败", prefix);
        }
    };
    if let Ok(mut rows) = stmt.query([]) {
        while let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0).unwrap_or_default();
            let data: String = row.get(1).unwrap_or_default();
            if let Some(date_part) = id.rsplit('_').next() {
                if date_part == today && !data.is_empty() {
                    let parts: Vec<&str> = data.split('|').collect();
                    if parts.len() >= 3 {
                        let uid = parts[0].to_string();
                        let hp: i32 = parts[1].parse().unwrap_or(0);
                        let gold: i64 = parts[2].parse().unwrap_or(0);
                        entries.push((uid, hp, gold));
                    }
                }
            }
        }
    }
    drop(stmt);
    drop(conn);

    // 按金币排序（通关且生命高的优先）
    entries.sort_by_key(|&(_, hp, gold)| std::cmp::Reverse(gold + hp as i64 * 10));

    let mut out = format!("{}\n", prefix);
    out.push_str("🏆 ━━━━━━ 迷宫排行 ━━━━━━\n\n");
    out.push_str(&format!("📅 {}\n\n", today));

    if entries.is_empty() {
        out.push_str("暂无今日通关记录\n");
        out.push_str("💡 成为第一个征服迷宫的人吧！");
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (uid, hp, gold)) in entries.iter().enumerate().take(15) {
            let medal = if i < 3 { medals[i] } else { &format!("{:2}.", i + 1) };
            let marker = if uid == user_id { " ⬅️ 你" } else { "" };
            out.push_str(&format!("{} {} - {}金 HP:{}%{}\n", medal, uid, gold, hp, marker));
        }
    }

    // 用户排名
    if let Some(pos) = entries.iter().position(|(uid, _, _)| uid == user_id) {
        out.push_str(&format!("\n📍 你的排名: 第{}名", pos + 1));
    }

    out
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maze_event_emoji() {
        assert_eq!(MazeEvent::Combat.emoji(), "⚔️");
        assert_eq!(MazeEvent::Treasure.emoji(), "🎁");
        assert_eq!(MazeEvent::Trap.emoji(), "💀");
        assert_eq!(MazeEvent::Shop.emoji(), "🏪");
        assert_eq!(MazeEvent::Healing.emoji(), "💊");
        assert_eq!(MazeEvent::Boss.emoji(), "🐉");
        assert_eq!(MazeEvent::Exit.emoji(), "🏆");
    }

    #[test]
    fn test_maze_event_name() {
        assert_eq!(MazeEvent::Combat.name(), "怪物伏击");
        assert_eq!(MazeEvent::Boss.name(), "层主守卫");
        assert_eq!(MazeEvent::Healing.name(), "治疗泉");
    }

    #[test]
    fn test_floor_label() {
        assert_eq!(floor_label(1), "幽暗入口");
        assert_eq!(floor_label(3), "深渊大厅");
        assert_eq!(floor_label(5), "混沌核心");
        assert_eq!(floor_label(99), "未知");
    }

    #[test]
    fn test_floor_monster_level() {
        assert_eq!(floor_monster_level(1), 5);
        assert_eq!(floor_monster_level(3), 25);
        assert_eq!(floor_monster_level(5), 45);
    }

    #[test]
    fn test_hp_bar() {
        assert_eq!(hp_bar(100), "██████████ 100%");
        assert_eq!(hp_bar(50), "█████░░░░░ 50%");
        assert_eq!(hp_bar(0), "░░░░░░░░░░ 0%");
        assert_eq!(hp_bar(110), "██████████ 110%");
    }

    #[test]
    fn test_maze_hash_deterministic() {
        let h1 = maze_hash("player1", 1, 0, "2026-01-01");
        let h2 = maze_hash("player1", 1, 0, "2026-01-01");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_maze_hash_different_inputs() {
        let h1 = maze_hash("player1", 1, 0, "2026-01-01");
        let h2 = maze_hash("player1", 1, 1, "2026-01-01");
        let h3 = maze_hash("player2", 1, 0, "2026-01-01");
        assert_ne!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_select_event_last_node_boss() {
        let event = select_event(0, 2, NODES_PER_FLOOR - 1, NODES_PER_FLOOR as i32);
        assert_eq!(event, MazeEvent::Boss);
    }

    #[test]
    fn test_select_event_last_floor_exit() {
        let event = select_event(0, MAX_FLOORS, NODES_PER_FLOOR - 1, NODES_PER_FLOOR as i32);
        assert_eq!(event, MazeEvent::Exit);
    }

    #[test]
    fn test_select_event_first_node_start() {
        let event = select_event(0, 1, 0, NODES_PER_FLOOR as i32);
        assert_eq!(event, MazeEvent::Start);
    }

    #[test]
    fn test_maze_state_roundtrip() {
        let state = MazeState {
            floor: 3,
            node: 2,
            hp_pct: 75,
            gold_earned: 500,
            exp_earned: 200,
            items_found: 2,
            events_log: String::new(),
            completed: false,
        };
        let json = state.to_json();
        let loaded = MazeState::from_json(&json).unwrap();
        assert_eq!(loaded.floor, 3);
        assert_eq!(loaded.node, 2);
        assert_eq!(loaded.hp_pct, 75);
        assert_eq!(loaded.gold_earned, 500);
        assert_eq!(loaded.exp_earned, 200);
        assert_eq!(loaded.items_found, 2);
        assert!(!loaded.completed);
    }

    #[test]
    fn test_maze_state_completed() {
        let state = MazeState {
            floor: 5,
            node: 3,
            hp_pct: 50,
            gold_earned: 1000,
            exp_earned: 500,
            items_found: 5,
            events_log: String::new(),
            completed: true,
        };
        let json = state.to_json();
        let loaded = MazeState::from_json(&json).unwrap();
        assert!(loaded.completed);
    }

    #[test]
    fn test_maze_state_new() {
        let state = MazeState::new();
        assert_eq!(state.floor, 1);
        assert_eq!(state.node, 0);
        assert_eq!(state.hp_pct, 100);
        assert_eq!(state.gold_earned, 0);
        assert!(!state.completed);
    }

    #[test]
    fn test_floor_monster_name() {
        let name = floor_monster_name(1, 0);
        assert!(!name.is_empty());
        let name2 = floor_monster_name(5, 0);
        assert!(!name2.is_empty());
    }

    #[test]
    fn test_daily_limit_constant() {
        assert_eq!(DAILY_LIMIT, 3);
        assert_eq!(MAX_FLOORS, 5);
        assert_eq!(NODES_PER_FLOOR, 4);
    }
}

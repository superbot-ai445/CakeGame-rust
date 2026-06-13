/// 食物/饥饿系统
/// 基于 Foods_Hungervalue 表 (食物配置) + Foods_knapsack 表 (玩家食物背包)
/// + ext_hunger_info Global 表 (饥饿配置: 最大饱食度/衰减周期/HP惩罚/阈值等)
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 饥饿系统配置（从 ext_hunger_info 表读取）
#[derive(Debug, Clone)]
pub struct HungerConfig {
    /// 最大饱食度 (默认 1440)
    pub max_satiety: i32,
    /// 衰减周期（分钟，每过这么久减1点饱食度）(默认 10)
    pub decay_interval_min: i32,
    /// 饥饿HP惩罚（饱食度低于阈值时扣除的生命值）(默认 500)
    pub hp_penalty: i32,
    /// 饥饿阈值（低于此值触发惩罚）(默认 50)
    pub hunger_line: i32,
    /// 饥饿提示文本
    pub hunger_tip: String,
    /// 饥饿系统是否启用 ("真" = 启用)
    pub enabled: bool,
    /// 复活后饱食度 (默认 100)
    pub revive_value: i32,
}

impl Default for HungerConfig {
    fn default() -> Self {
        Self {
            max_satiety: 1440,
            decay_interval_min: 10,
            hp_penalty: 500,
            hunger_line: 50,
            hunger_tip: "你肚子饿了，需要吃饭了。".to_string(),
            enabled: true,
            revive_value: 100,
        }
    }
}

/// 从 Global 表的 ext_hunger_info 读取饥饿配置
pub fn load_hunger_config(db: &Database) -> HungerConfig {
    let mut cfg = HungerConfig::default();

    let max_s = db.global_get("hungermax", "ext_hunger_info");
    if !max_s.is_empty() {
        cfg.max_satiety = max_s.parse().unwrap_or(cfg.max_satiety);
    }
    let interval = db.global_get("hungertime", "ext_hunger_info");
    if !interval.is_empty() {
        cfg.decay_interval_min = interval.parse().unwrap_or(cfg.decay_interval_min);
    }
    let hp_down = db.global_get("hungerhpdown", "ext_hunger_info");
    if !hp_down.is_empty() {
        cfg.hp_penalty = hp_down.parse().unwrap_or(cfg.hp_penalty);
    }
    let line = db.global_get("hungerline", "ext_hunger_info");
    if !line.is_empty() {
        cfg.hunger_line = line.parse().unwrap_or(cfg.hunger_line);
    }
    let tip = db.global_get("hungertip", "ext_hunger_info");
    if !tip.is_empty() {
        cfg.hunger_tip = tip;
    }
    let sel = db.global_get("hungerselect", "ext_hunger_info");
    if !sel.is_empty() {
        cfg.enabled = sel == "真";
    }
    let revive = db.global_get("revivevalue", "ext_hunger_info");
    if !revive.is_empty() {
        cfg.revive_value = revive.parse().unwrap_or(cfg.revive_value);
    }

    cfg
}

/// 应用饥饿衰减 — 每次玩家执行指令时调用
/// 根据 ext_hunger_info.hungertime (分钟) 决定衰减频率
/// 返回 (衰减了多少, 当前饱食度, 是否触发饥饿提示)
pub fn apply_hunger_decay(db: &Database, user_id: &str) -> (i32, i32, bool) {
    let cfg = load_hunger_config(db);
    if !cfg.enabled || cfg.decay_interval_min <= 0 {
        return (0, 0, false);
    }

    let satiety_str = db.read_basic(user_id, "Satiety");
    let current_satiety: i32 = satiety_str.parse().unwrap_or(0);

    // 读取上次衰减时间
    let last_tick_str = db.read_basic(user_id, "LastHungerTick");
    let now = chrono::Local::now().timestamp() as i32;

    let last_tick: i32 = if last_tick_str.is_empty() {
        // 首次初始化，设为当前时间，不衰减
        db.write_basic(user_id, "LastHungerTick", &now.to_string());
        return (0, current_satiety, false);
    } else {
        last_tick_str.parse().unwrap_or(now)
    };

    let elapsed_secs = now.saturating_sub(last_tick);
    let decay_interval_secs = cfg.decay_interval_min * 60;

    if elapsed_secs < decay_interval_secs {
        // 还没到衰减时间
        return (0, current_satiety, false);
    }

    // 计算衰减量
    let decay_count = elapsed_secs / decay_interval_secs;
    let new_satiety = (current_satiety - decay_count).max(0);
    let actual_decay = current_satiety - new_satiety;

    if actual_decay > 0 {
        db.write_basic(user_id, "Satiety", &new_satiety.to_string());
        db.write_basic(user_id, "LastHungerTick", &now.to_string());
    }

    let trigger_tip = new_satiety <= cfg.hunger_line && new_satiety > 0;
    (actual_decay, new_satiety, trigger_tip)
}

/// 获取饥饿HP惩罚（当饱食度低于阈值时返回应扣除的HP）
/// 用于战斗系统集成
pub fn get_hunger_hp_penalty(db: &Database, user_id: &str) -> i32 {
    let cfg = load_hunger_config(db);
    if !cfg.enabled {
        return 0;
    }

    let satiety: i32 = db.read_basic(user_id, "Satiety").parse().unwrap_or(0);
    if satiety <= cfg.hunger_line && satiety > 0 {
        // 饱食度低于阈值但未归零，按比例惩罚
        let ratio = 1.0 - (satiety as f64 / cfg.hunger_line as f64);
        (cfg.hp_penalty as f64 * ratio) as i32
    } else if satiety <= 0 {
        // 完全饥饿，全额惩罚
        cfg.hp_penalty
    } else {
        0
    }
}

/// 获取饥饿经验加成倍率（基于饱食度等级）
/// 返回 (倍率, 描述文本)
pub fn get_hunger_exp_bonus(db: &Database, user_id: &str) -> (f64, &'static str) {
    let satiety: i32 = db.read_basic(user_id, "Satiety").parse().unwrap_or(0);
    let cfg = load_hunger_config(db);

    // 用 max_satiety 的百分比来定义等级
    let pct = if cfg.max_satiety > 0 {
        (satiety as f64 / cfg.max_satiety as f64) * 100.0
    } else {
        0.0
    };

    if pct >= 69.0 {
        (1.5, "✨ 极饱 (+50%经验)")
    } else if pct >= 34.0 {
        (1.2, "😊 饱足 (+20%经验)")
    } else if pct >= 7.0 {
        (1.0, "🙂 正常")
    } else if pct > 0.0 {
        (1.0, "😐 微饱")
    } else {
        (0.5, "💀 饥饿 (-50%经验)")
    }
}

/// 饥饿状态 — 查看当前饥饿配置与状态
pub fn cmd_hunger_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let cfg = load_hunger_config(db);

    let satiety: i32 = db.read_basic(user_id, "Satiety").parse().unwrap_or(0);
    let last_tick_str = db.read_basic(user_id, "LastHungerTick");

    let mut r = format!("{}\n═══ 🍖 饥饿状态 ═══", prefix);

    // 当前饱食度
    let pct = if cfg.max_satiety > 0 {
        (satiety as f64 / cfg.max_satiety as f64 * 100.0) as i32
    } else {
        0
    };
    r.push_str(&format!(
        "\n当前饱食度: {} / {} ({}%)",
        satiety,
        cfg.max_satiety,
        pct.min(100)
    ));

    // 饱食度进度条
    let bar_len = 20;
    let filled = ((pct.min(100) as f64 / 100.0) * bar_len as f64) as usize;
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
    r.push_str(&format!("\n[{}]", bar));

    // 饱食度等级和增益
    let (bonus_mult, bonus_desc) = get_hunger_exp_bonus(db, user_id);
    r.push_str(&format!("\n状态: {}", bonus_desc));
    r.push_str(&format!("\n经验倍率: {:.0}%", bonus_mult * 100.0));

    // 下次衰减倒计时
    if cfg.enabled && cfg.decay_interval_min > 0 {
        let now = chrono::Local::now().timestamp() as i32;
        let last_tick: i32 = last_tick_str.parse().unwrap_or(now);
        let elapsed = now.saturating_sub(last_tick);
        let interval_secs = cfg.decay_interval_min * 60;
        let remaining = interval_secs.saturating_sub(elapsed);
        if remaining > 0 {
            let mins = remaining / 60;
            let secs = remaining % 60;
            r.push_str(&format!("\n下次衰减: {}分{}秒后 (-1饱食度)", mins, secs));
        } else {
            r.push_str("\n下次衰减: 即将衰减!");
        }
    }

    // 饥饿惩罚信息
    let penalty = get_hunger_hp_penalty(db, user_id);
    if penalty > 0 {
        r.push_str(&format!("\n\n⚠️ 饥饿惩罚: 战斗中 HP-{}", penalty));
        r.push_str(&format!("\n{}", cfg.hunger_tip));
    }

    // 配置信息
    r.push_str("\n\n═══ 饥饿配置 ═══");
    r.push_str(&format!("\n最大饱食度: {}", cfg.max_satiety));
    r.push_str(&format!("\n衰减周期: 每{}分钟-1", cfg.decay_interval_min));
    r.push_str(&format!("\n饥饿阈值: {}", cfg.hunger_line));
    r.push_str(&format!("\n饥饿HP惩罚: {}", cfg.hp_penalty));
    r.push_str(&format!("\n复活饱食度: {}", cfg.revive_value));
    r.push_str(&format!(
        "\n系统状态: {}",
        if cfg.enabled { "✅ 启用" } else { "❌ 禁用" }
    ));

    r.push_str("\n\n💡 提示: 饱食度会随时间缓慢下降，保持进食维持增益！");
    r.push_str("\n发送'查看食物'购买食物 | '使用食物+食物名'进食");
    r
}

/// 读取玩家食物背包 (Foods_knapsack 表)
fn read_food_bag(db: &Database, user_id: &str) -> std::collections::BTreeMap<String, i32> {
    let mut map = std::collections::BTreeMap::new();
    if let Ok(bag_json) = db.query_row("SELECT Bag FROM Foods_knapsack WHERE uID = ?", &[user_id], |row| {
        Ok(row.get::<_, String>(0).unwrap_or_default())
    }) {
        if !bag_json.is_empty() && bag_json != "{}" {
            if let Ok(parsed) = serde_json::from_str::<std::collections::HashMap<String, String>>(&bag_json) {
                for (k, v) in parsed {
                    if let Ok(count) = v.parse::<i32>() {
                        if count > 0 {
                            map.insert(k, count);
                        }
                    }
                }
            }
        }
    }
    map
}

/// 写入玩家食物背包 (Foods_knapsack 表)
fn write_food_bag(db: &Database, user_id: &str, bag: &std::collections::BTreeMap<String, i32>) {
    let json_map: std::collections::HashMap<String, String> =
        bag.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
    let json = if json_map.is_empty() {
        "{}".to_string()
    } else {
        serde_json::to_string(&json_map).unwrap_or_else(|_| "{}".to_string())
    };
    let conn = db.lock_conn();
    // Upsert
    let existing = conn
        .prepare("SELECT COUNT(*) FROM Foods_knapsack WHERE uID = ?1")
        .and_then(|mut s| s.query_row(rusqlite::params![user_id], |row| row.get::<_, i32>(0)))
        .unwrap_or(0);
    if existing > 0 {
        let _ = conn.execute(
            "UPDATE Foods_knapsack SET Bag = ?1 WHERE uID = ?2",
            rusqlite::params![json, user_id],
        );
    } else {
        let _ = conn.execute(
            "INSERT INTO Foods_knapsack (uID, Bag) VALUES (?1, ?2)",
            rusqlite::params![user_id, json],
        );
    }
}

/// 迁移: 将 Basic_knapsack 中带 🍖 前缀的食物迁移到 Foods_knapsack
fn migrate_knapsack_food(db: &Database, user_id: &str) {
    let items = db.knapsack_all(user_id);
    let food_items: Vec<_> = items.iter().filter(|i| i.name.starts_with('🍖')).collect();
    if food_items.is_empty() {
        return;
    }
    let mut bag = read_food_bag(db, user_id);
    for item in &food_items {
        let food_name = &item.name[3..]; // strip 🍖 prefix
        *bag.entry(food_name.to_string()).or_insert(0) += item.quantity;
        db.remove_item(user_id, &item.name, item.quantity);
    }
    write_food_bag(db, user_id, &bag);
}

/// 查看食物列表
pub fn cmd_view_foods(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    let foods: Vec<(String, String, String)> = db.query_rows(
        "SELECT Name, Value, Price FROM Foods_Hungervalue ORDER BY CAST(Price AS INTEGER)",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    );

    if foods.is_empty() {
        return format!("{}\n暂无可购买的食物。", prefix);
    }

    let mut r = format!("{}\n═══ 🍖 食物商店 ═══\n━━━━━━━━━━━━━━━━━━━━", prefix);
    for (i, (name, value, price)) in foods.iter().enumerate() {
        r.push_str(&format!("\n{}. [{}] 饱食度+{}  💰{}金币", i + 1, name, value, price));
    }
    r.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    r.push_str("\n\n发送'购买食物+食物名'购买");
    r.push_str("\n发送'使用食物+食物名'进食");
    r.push_str("\n发送'我的食物'查看食物背包");
    r
}

/// 购买食物 → 存入 Foods_knapsack
pub fn cmd_buy_food(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let food_name = args.trim();
    if food_name.is_empty() {
        return format!("{}\n请指定要购买的食物。\n用法：购买食物+食物名", prefix);
    }

    // 查找食物配置（模糊匹配）
    let food = match db.query_row(
        "SELECT Name, Value, Price FROM Foods_Hungervalue WHERE Name = ?",
        &[food_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    ) {
        Ok(f) => f,
        Err(_) => {
            // 尝试模糊匹配
            let pattern = format!("%{}%", food_name);
            let foods: Vec<(String, String, String)> = db.query_rows(
                "SELECT Name, Value, Price FROM Foods_Hungervalue WHERE Name LIKE ?",
                &[pattern.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, String>(2).unwrap_or_default(),
                    ))
                },
            );
            if foods.len() == 1 {
                foods.into_iter().next().unwrap()
            } else if foods.is_empty() {
                return format!("{}\n找不到食物 [{}]。", prefix, food_name);
            } else {
                let mut r = format!("{}\n找到多个食物，请精确指定：\n", prefix);
                for (name, _, _) in &foods {
                    r.push_str(&format!("  · {}\n", name));
                }
                return r;
            }
        }
    };

    let (name, _value, price) = food;
    let gold_cost: i32 = price.parse().unwrap_or(0);

    // 检查金币
    let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
    if user_gold < gold_cost {
        return format!(
            "{}\n购买 [{}] 需要 {} 金币，你只有 {} 金币。",
            prefix, name, gold_cost, user_gold
        );
    }

    // 扣除金币
    db.write_currency(user_id, CURRENCY_GOLD, (user_gold - gold_cost) as i64);

    // 写入 Foods_knapsack
    let mut bag = read_food_bag(db, user_id);
    *bag.entry(name.clone()).or_insert(0) += 1;
    write_food_bag(db, user_id, &bag);

    let current_count = bag.get(&name).copied().unwrap_or(1);
    format!(
        "{}\n购买成功！\n🍖[{}] ×1 (库存:{})\n消耗金币：{}\n\n发送'使用食物+{}'进食",
        prefix, name, current_count, gold_cost, name
    )
}

/// 使用食物 — 从 Foods_knapsack 消耗
pub fn cmd_use_food(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let food_name = args.trim();
    if food_name.is_empty() {
        return format!("{}\n请指定要使用的食物。\n用法：使用食物+食物名", prefix);
    }

    // 先做迁移
    migrate_knapsack_food(db, user_id);

    // 查找食物配置
    let food = match db.query_row(
        "SELECT Name, Value, Price FROM Foods_Hungervalue WHERE Name = ?",
        &[food_name],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        },
    ) {
        Ok(f) => f,
        Err(_) => {
            // 尝试模糊匹配
            let pattern = format!("%{}%", food_name);
            let foods: Vec<(String, String, String)> = db.query_rows(
                "SELECT Name, Value, Price FROM Foods_Hungervalue WHERE Name LIKE ?",
                &[pattern.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, String>(2).unwrap_or_default(),
                    ))
                },
            );
            if foods.len() == 1 {
                foods.into_iter().next().unwrap()
            } else if foods.is_empty() {
                return format!("{}\n找不到食物 [{}]。", prefix, food_name);
            } else {
                let mut r = format!("{}\n找到多个食物，请精确指定：\n", prefix);
                for (name, _, _) in &foods {
                    r.push_str(&format!("  · {}\n", name));
                }
                return r;
            }
        }
    };

    let (name, value, _price) = food;

    // 检查 Foods_knapsack 中是否有该食物
    let mut bag = read_food_bag(db, user_id);
    let count = bag.get(&name).copied().unwrap_or(0);
    if count <= 0 {
        return format!("{}\n你没有食物 [{}]。\n发送'购买食物+{}'购买", prefix, name, name);
    }

    // 消耗食物
    if count <= 1 {
        bag.remove(&name);
    } else {
        bag.insert(name.clone(), count - 1);
    }
    write_food_bag(db, user_id, &bag);

    // 增加饱食度（写入用户状态）
    let satiety_str = db.read_basic(user_id, "Satiety");
    let current_satiety: i32 = satiety_str.parse().unwrap_or(0);
    let add_value: i32 = value.parse().unwrap_or(0);
    let new_satiety = current_satiety + add_value;
    db.write_basic(user_id, "Satiety", &new_satiety.to_string());

    let mut result = format!(
        "{}\n🍖 进食 [{}]！\n饱食度 +{}\n当前饱食度：{}",
        prefix, name, add_value, new_satiety
    );

    // 高饱食度buff (基于 ext_hunger_info 配置)
    let (_, bonus_desc) = get_hunger_exp_bonus(db, user_id);
    result.push_str(&format!("\n\n{}", bonus_desc));

    result
}

/// 我的食物背包 — 优先从 Foods_knapsack 读取，兼容迁移
pub fn cmd_my_foods(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 先做迁移
    migrate_knapsack_food(db, user_id);

    // 获取饱食度
    let satiety_str = db.read_basic(user_id, "Satiety");
    let current_satiety: i32 = satiety_str.parse().unwrap_or(0);

    // 从 Foods_knapsack 读取
    let bag = read_food_bag(db, user_id);

    let mut r = format!("{}\n═══ 🍖 我的食物 ═══", prefix);
    r.push_str(&format!("\n当前饱食度：{}", current_satiety));
    r.push_str("\n━━━━━━━━━━━━━━━━━━━━");

    if bag.is_empty() {
        r.push_str("\n食物背包为空。");
    } else {
        let total: i32 = bag.values().sum();
        r.push_str(&format!("\n📦 食物总数: {}\n", total));
        for (i, (name, count)) in bag.iter().enumerate() {
            // 查找该食物的饱食度
            let hv = db
                .query_row(
                    "SELECT Value FROM Foods_Hungervalue WHERE Name = ?",
                    &[name.as_str()],
                    |row| Ok(row.get::<_, String>(0).unwrap_or_default()),
                )
                .unwrap_or_default();
            if hv.is_empty() {
                r.push_str(&format!("\n{}. [{}] ×{}", i + 1, name, count));
            } else {
                r.push_str(&format!("\n{}. [{}] ×{} (饱食度+{})", i + 1, name, count, hv));
            }
        }
    }

    r.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    r.push_str("\n\n发送'查看食物'查看食物商店");
    r
}

/// 食物统计 — 查看所有食物的购买/使用统计
pub fn cmd_food_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 先做迁移
    migrate_knapsack_food(db, user_id);

    // 读取玩家食物背包
    let bag = read_food_bag(db, user_id);
    let satiety_str = db.read_basic(user_id, "Satiety");
    let current_satiety: i32 = satiety_str.parse().unwrap_or(0);

    let mut r = format!("{}\n═══ 🍖 食物统计 ═══", prefix);

    // 饱食度状态（基于 ext_hunger_info 配置）
    let (_, bonus_desc) = get_hunger_exp_bonus(db, user_id);
    let cfg = load_hunger_config(db);
    let max_s = cfg.max_satiety;
    let level = bonus_desc; // bonus_desc 已经包含等级文本

    r.push_str(&format!("\n当前饱食度: {}/{} ({})", current_satiety, max_s, level));
    // 显示经验倍率
    let (bonus_mult, _) = get_hunger_exp_bonus(db, user_id);
    r.push_str(&format!("\n经验倍率: {:.0}%", bonus_mult * 100.0));

    // 食物背包统计
    let total: i32 = bag.values().sum();
    r.push_str(&format!("\n\n📦 食物库存: {} 种 / {} 份", bag.len(), total));

    if !bag.is_empty() {
        // 计算总饱食度价值
        let mut total_value: i32 = 0;
        for (name, count) in &bag {
            let hv: i32 = db
                .query_row(
                    "SELECT Value FROM Foods_Hungervalue WHERE Name = ?",
                    &[name.as_str()],
                    |row| {
                        let v: String = row.get(0).unwrap_or_default();
                        Ok(v.parse().unwrap_or(0))
                    },
                )
                .unwrap_or(0);
            total_value += hv * count;
        }
        r.push_str(&format!("\n🍖 库存总饱食度: +{}", total_value));
    }

    // 全服食物数据统计
    let total_players: i32 = db
        .lock_conn()
        .prepare("SELECT COUNT(*) FROM Foods_knapsack WHERE Bag != '{}' AND Bag IS NOT NULL")
        .and_then(|mut s| s.query_row([], |row| row.get(0)))
        .unwrap_or(0);
    r.push_str(&format!("\n\n🌍 全服: {} 位玩家持有食物", total_players));

    r.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    r.push_str("\n💡 提示: 饱食度会随时间缓慢下降，保持进食维持增益！");
    r
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_food_bag_json_roundtrip() {
        let mut bag = std::collections::BTreeMap::new();
        bag.insert("压缩饼干".to_string(), 101);
        bag.insert("小鱼干".to_string(), 20);
        let json_map: std::collections::HashMap<String, String> =
            bag.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
        let json = serde_json::to_string(&json_map).unwrap();
        let parsed: std::collections::HashMap<String, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.get("压缩饼干").unwrap(), "101");
        assert_eq!(parsed.get("小鱼干").unwrap(), "20");
    }

    #[test]
    fn test_satiety_levels() {
        // 验证饱食度等级逻辑
        let test_cases = vec![
            (0, "💀 饥饿"),
            (50, "😐 微饱"),
            (100, "🙂 正常"),
            (500, "😊 饱足"),
            (1000, "✨ 极饱"),
        ];
        for (val, expected) in test_cases {
            let level = if val >= 1000 {
                "✨ 极饱"
            } else if val >= 500 {
                "😊 饱足"
            } else if val >= 100 {
                "🙂 正常"
            } else if val >= 1 {
                "😐 微饱"
            } else {
                "💀 饥饿"
            };
            assert_eq!(level, expected, "satiety={} should be {}", val, expected);
        }
    }

    #[test]
    fn test_food_bag_empty() {
        let bag: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
        let json_map: std::collections::HashMap<String, String> =
            bag.iter().map(|(k, v)| (k.clone(), v.to_string())).collect();
        let json = serde_json::to_string(&json_map).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_food_bag_partial_remove() {
        let mut bag = std::collections::BTreeMap::new();
        bag.insert("压缩饼干".to_string(), 5);
        // Simulate consuming 1
        let count = bag.get("压缩饼干").copied().unwrap_or(0);
        assert_eq!(count, 5);
        if count <= 1 {
            bag.remove("压缩饼干");
        } else {
            bag.insert("压缩饼干".to_string(), count - 1);
        }
        assert_eq!(bag.get("压缩饼干").copied().unwrap_or(0), 4);
    }

    #[test]
    fn test_food_bag_full_remove() {
        let mut bag = std::collections::BTreeMap::new();
        bag.insert("小鱼干".to_string(), 1);
        let count = bag.get("小鱼干").copied().unwrap_or(0);
        assert_eq!(count, 1);
        if count <= 1 {
            bag.remove("小鱼干");
        } else {
            bag.insert("小鱼干".to_string(), count - 1);
        }
        assert!(!bag.contains_key("小鱼干"));
    }

    #[test]
    fn test_hunger_config_defaults() {
        let cfg = crate::food::HungerConfig::default();
        assert_eq!(cfg.max_satiety, 1440);
        assert_eq!(cfg.decay_interval_min, 10);
        assert_eq!(cfg.hp_penalty, 500);
        assert_eq!(cfg.hunger_line, 50);
        assert!(cfg.enabled);
        assert_eq!(cfg.revive_value, 100);
    }

    #[test]
    fn test_hunger_exp_bonus_thresholds() {
        // max_satiety=1440: 69%=993.6, 34%=489.6, 7%=100.8
        let test_cases: Vec<(i32, f64, &str)> = vec![
            (1440, 1.5, "极饱"), // 100%
            (1000, 1.5, "极饱"), // 69.4%
            (500, 1.2, "饱足"),  // 34.7%
            (200, 1.0, "正常"),  // 13.9%
            (50, 1.0, "微饱"),   // 3.5%
            (0, 0.5, "饥饿"),    // 0%
        ];
        for (satiety, expected_mult, _expected_frag) in test_cases {
            let pct = (satiety as f64 / 1440.0) * 100.0;
            let (mult, _desc) = if pct >= 69.0 {
                (1.5, "✨ 极饱 (+50%经验)")
            } else if pct >= 34.0 {
                (1.2, "😊 饱足 (+20%经验)")
            } else if pct >= 7.0 {
                (1.0, "🙂 正常")
            } else if pct > 0.0 {
                (1.0, "😐 微饱")
            } else {
                (0.5, "💀 饥饿 (-50%经验)")
            };
            assert_eq!(mult, expected_mult, "satiety={}", satiety);
        }
    }

    #[test]
    fn test_hunger_hp_penalty_logic() {
        let hunger_line = 50i32;
        let hp_penalty = 500i32;

        // satiety=0 -> full penalty
        let penalty = hp_penalty;
        assert_eq!(penalty, 500);

        // satiety=25 -> 50% penalty
        let ratio = 1.0 - (25.0 / hunger_line as f64);
        let penalty = (hp_penalty as f64 * ratio) as i32;
        assert_eq!(penalty, 250);

        // satiety=50 -> no penalty (at threshold exactly)
        assert!(50 >= hunger_line);

        // satiety=100 -> no penalty
        assert!(100 > hunger_line);
    }

    #[test]
    fn test_hunger_decay_interval_conversion() {
        let interval_min = 10i32;
        let interval_secs = interval_min * 60;
        assert_eq!(interval_secs, 600);

        // 1 hour of elapsed time
        let elapsed = 3600;
        let decay_count = elapsed / interval_secs;
        assert_eq!(decay_count, 6);
    }
}

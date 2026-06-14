/// 时空裂隙系统
/// 随机出现的时空裂隙，玩家进入挑战波次怪物，收集时空碎片兑换奖励
/// 数据存储：Global 表 SECTION='temporal_rift' / 'temporal_rift_daily'
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, OP_ADD};
use crate::db::Database;
use rand::Rng;

/// 裂隙等级定义
struct RiftLevel {
    level: i32,
    name: &'static str,
    emoji: &'static str,
    waves: i32,
    base_hp: i32,
    base_ad: i32,
    shard_reward: i32,
    gold_reward: i32,
    diamond_reward: i32,
    min_power: i32,
}

/// 获取裂隙等级定义
fn get_rift_levels() -> Vec<RiftLevel> {
    vec![
        RiftLevel {
            level: 1,
            name: "微型裂隙",
            emoji: "🌀",
            waves: 3,
            base_hp: 500,
            base_ad: 80,
            shard_reward: 10,
            gold_reward: 2000,
            diamond_reward: 5,
            min_power: 500,
        },
        RiftLevel {
            level: 2,
            name: "小型裂隙",
            emoji: "🌪️",
            waves: 5,
            base_hp: 1200,
            base_ad: 180,
            shard_reward: 25,
            gold_reward: 5000,
            diamond_reward: 15,
            min_power: 2000,
        },
        RiftLevel {
            level: 3,
            name: "中型裂隙",
            emoji: "⚡",
            waves: 7,
            base_hp: 2500,
            base_ad: 350,
            shard_reward: 50,
            gold_reward: 12000,
            diamond_reward: 30,
            min_power: 5000,
        },
        RiftLevel {
            level: 4,
            name: "大型裂隙",
            emoji: "💥",
            waves: 10,
            base_hp: 5000,
            base_ad: 600,
            shard_reward: 100,
            gold_reward: 30000,
            diamond_reward: 60,
            min_power: 10000,
        },
        RiftLevel {
            level: 5,
            name: "深渊裂隙",
            emoji: "🕳️",
            waves: 15,
            base_hp: 10000,
            base_ad: 1200,
            shard_reward: 200,
            gold_reward: 80000,
            diamond_reward: 150,
            min_power: 25000,
        },
        RiftLevel {
            level: 6,
            name: "灭世裂隙",
            emoji: "☄️",
            waves: 20,
            base_hp: 20000,
            base_ad: 2500,
            shard_reward: 400,
            gold_reward: 200000,
            diamond_reward: 300,
            min_power: 50000,
        },
    ]
}

/// 裂隙商店物品
struct RiftShopItem {
    name: &'static str,
    cost: i32,
    description: &'static str,
    emoji: &'static str,
}

fn get_rift_shop_items() -> Vec<RiftShopItem> {
    vec![
        RiftShopItem {
            name: "时空精华",
            cost: 50,
            description: "用于装备附魔的稀有材料",
            emoji: "✨",
        },
        RiftShopItem {
            name: "裂隙宝箱",
            cost: 100,
            description: "随机开出珍贵道具",
            emoji: "🎁",
        },
        RiftShopItem {
            name: "时空护符",
            cost: 200,
            description: "增加30%经验获取24小时",
            emoji: "📿",
        },
        RiftShopItem {
            name: "裂隙之翼",
            cost: 500,
            description: "永久增加5%移动速度",
            emoji: "🪽",
        },
        RiftShopItem {
            name: "时空结晶",
            cost: 800,
            description: "强化装备时必定成功",
            emoji: "💎",
        },
        RiftShopItem {
            name: "裂隙战甲",
            cost: 1500,
            description: "专属装备:防御+500,魔抗+300",
            emoji: "🛡️",
        },
        RiftShopItem {
            name: "时空之刃",
            cost: 2000,
            description: "专属装备:物攻+800,暴击+15%",
            emoji: "⚔️",
        },
        RiftShopItem {
            name: "裂隙之心",
            cost: 3000,
            description: "永久增加10%全属性",
            emoji: "❤️‍🔥",
        },
        RiftShopItem {
            name: "时空钥匙",
            cost: 5000,
            description: "开启时空宝藏(随机传说装备)",
            emoji: "🗝️",
        },
        RiftShopItem {
            name: "裂隙至尊宝箱",
            cost: 10000,
            description: "必出神器级道具",
            emoji: "👑",
        },
    ]
}

/// 裂隙怪物名称
fn get_rift_monsters(level: i32) -> Vec<&'static str> {
    match level {
        1 => vec!["裂隙虫", "时空蚁", "虚空气泡"],
        2 => vec!["裂隙猎犬", "时空蝠", "虚空游荡者"],
        3 => vec!["裂隙守卫", "时空蜥蜴", "虚空骑士"],
        4 => vec!["裂隙巨兽", "时空龙蜥", "虚空领主"],
        5 => vec!["裂隙恶魔", "时空古龙", "虚空帝王"],
        6 => vec!["裂隙毁灭者", "时空之王", "虚空终焉"],
        _ => vec!["未知生物"],
    }
}

/// 从Global表读取玩家裂隙数据
fn get_rift_data(db: &Database, user_id: &str) -> (i32, i32, i32, i32) {
    // (total_shards, best_level, best_waves, total_clears)
    let data = db.global_get("temporal_rift", user_id);
    if data.is_empty() || data == "[NULL]" {
        return (0, 0, 0, 0);
    }
    let parts: Vec<&str> = data.split(',').collect();
    if parts.len() >= 4 {
        (
            parts[0].parse().unwrap_or(0),
            parts[1].parse().unwrap_or(0),
            parts[2].parse().unwrap_or(0),
            parts[3].parse().unwrap_or(0),
        )
    } else {
        (0, 0, 0, 0)
    }
}

/// 保存玩家裂隙数据
fn save_rift_data(db: &Database, user_id: &str, shards: i32, best_level: i32, best_waves: i32, clears: i32) {
    db.global_set(
        "temporal_rift",
        user_id,
        &format!("{},{},{},{}", shards, best_level, best_waves, clears),
    );
}

/// 读取每日进入次数
fn get_daily_entries(db: &Database, user_id: &str) -> i32 {
    let key = format!("{}_entries", user_id);
    let data = db.global_get("temporal_rift_daily", &key);
    if data.is_empty() || data == "[NULL]" {
        return 0;
    }
    let parts: Vec<&str> = data.split(',').collect();
    if parts.len() >= 2 {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        if parts[0] == today {
            return parts[1].parse().unwrap_or(0);
        }
    }
    0
}

/// 增加每日进入次数
fn add_daily_entry(db: &Database, user_id: &str) {
    let key = format!("{}_entries", user_id);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let count = get_daily_entries(db, user_id) + 1;
    db.global_set("temporal_rift_daily", &key, &format!("{},{}", today, count));
}

/// 计算玩家战力
fn get_player_power(db: &Database, user_id: &str) -> i32 {
    let info = crate::user::calc_total_attrs(db, user_id);
    info.hp_max / 5 + info.ad + info.ap + info.defense + info.magic_res
}

// ============ 指令实现 ============

/// 查看裂隙 — 显示当前可用裂隙和玩家进度
pub fn cmd_view_rifts(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let levels = get_rift_levels();
    let (shards, best_level, best_waves, clears) = get_rift_data(db, user_id);
    let daily = get_daily_entries(db, user_id);
    let power = get_player_power(db, user_id);
    let max_daily = 5;

    let mut out = String::from("╔══════════════════════════════╗\n");
    out.push_str("║      ⏳ 时空裂隙 ⏳        ║\n");
    out.push_str("╚══════════════════════════════╝\n\n");
    out.push_str(&format!(
        "💰 时空碎片: {} | 🏆 最佳: 第{}层-{}波\n",
        shards, best_level, best_waves
    ));
    out.push_str(&format!(
        "⚔️ 战力: {} | 📅 今日: {}/{}次 | 🔄 总通关: {}次\n\n",
        power, daily, max_daily, clears
    ));

    out.push_str("【可挑战裂隙】\n");
    for lv in &levels {
        let status = if power >= lv.min_power {
            "✅"
        } else {
            "❌战力不足"
        };
        out.push_str(&format!(
            "  {} Lv.{} {} — {}波 | 碎片+{} | 金+{} | 💎+{} | {}\n",
            lv.emoji, lv.level, lv.name, lv.waves, lv.shard_reward, lv.gold_reward, lv.diamond_reward, status
        ));
    }
    out.push_str(&format!(
        "\n💡 每日最多挑战{}次 | 输入「进入裂隙+等级」开始挑战\n",
        max_daily
    ));
    out
}

/// 进入裂隙 — 选择难度开始挑战
pub fn cmd_enter_rift(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let level: i32 = args.trim().parse().unwrap_or(0);
    if !(1..=6).contains(&level) {
        return "❌ 请输入裂隙等级(1-6)\n💡 例: 进入裂隙+3".to_string();
    }

    let levels = get_rift_levels();
    let rift = levels.iter().find(|l| l.level == level).unwrap();
    let power = get_player_power(db, user_id);
    if power < rift.min_power {
        return format!("❌ 战力不足! 需要{}战力，当前{}", rift.min_power, power);
    }

    let daily = get_daily_entries(db, user_id);
    if daily >= 5 {
        return "❌ 今日挑战次数已用完(5/5)，明天再来!".to_string();
    }

    add_daily_entry(db, user_id);

    // 模拟裂隙战斗
    let mut rng = rand::thread_rng();
    let monsters = get_rift_monsters(level);
    let (shards, best_level, best_waves, clears) = get_rift_data(db, user_id);
    let mut waves_cleared = 0i32;
    let mut total_damage = 0i64;
    let mut total_gold = 0i32;
    let mut battle_log = Vec::new();

    for wave in 1..=rift.waves {
        let monster_idx = rng.gen_range(0..monsters.len());
        let monster = monsters[monster_idx];
        let monster_hp = rift.base_hp + (wave - 1) * rift.base_hp / rift.waves;
        let monster_ad = rift.base_ad + (wave - 1) * rift.base_ad / rift.waves;

        // 战斗模拟
        let player_hp = power * 3;
        let player_dmg = power / 2 + rng.gen_range(0..power / 4);
        let monster_dmg = monster_ad - (power / 10).min(monster_ad / 2);
        let turns = (monster_hp + player_dmg - 1) / player_dmg.max(1);
        let damage_taken = turns * monster_dmg.max(1);

        if damage_taken < player_hp {
            // 胜利
            waves_cleared = wave;
            total_damage += monster_hp as i64;
            let wave_gold = rift.gold_reward / rift.waves;
            total_gold += wave_gold;
            let crit = if rng.gen_bool(0.2) { " 💥暴击!" } else { "" };
            battle_log.push(format!(
                "  ⚔️ 第{}波 {} — 击败! 伤害:{}{}",
                wave,
                monster,
                player_dmg * turns,
                crit
            ));
        } else {
            battle_log.push(format!("  💀 第{}波 {} — 阵亡! (HP不足)", wave, monster));
            break;
        }
    }

    // 计算奖励
    let shards_earned = rift.shard_reward * waves_cleared / rift.waves;
    let diamond_earned = if waves_cleared == rift.waves {
        rift.diamond_reward
    } else {
        rift.diamond_reward * waves_cleared / rift.waves / 2
    };
    let clear_bonus = if waves_cleared == rift.waves {
        " 🎉完美通关!"
    } else {
        ""
    };

    // 发放奖励
    if shards_earned > 0 {
        save_rift_data(
            db,
            user_id,
            shards + shards_earned,
            best_level.max(level),
            best_waves.max(waves_cleared),
            if waves_cleared == rift.waves {
                clears + 1
            } else {
                clears
            },
        );
    }
    if total_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold as i64);
    }
    if diamond_earned > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, diamond_earned as i64);
    }

    let mut out = String::from("╔══════════════════════════════╗\n");
    out.push_str(&format!("║   {} {} Lv.{} 战斗报告   ║\n", rift.emoji, rift.name, level));
    out.push_str("╚══════════════════════════════╝\n\n");
    for log in &battle_log {
        out.push_str(log);
        out.push('\n');
    }
    out.push_str(&format!(
        "\n📊 通关: {}/{}波{}\n",
        waves_cleared, rift.waves, clear_bonus
    ));
    out.push_str(&format!(
        "💰 金币: +{} | 💎 钻石: +{} | ⏳ 碎片: +{}\n",
        total_gold, diamond_earned, shards_earned
    ));
    out.push_str(&format!("⚔️ 总伤害: {}\n", total_damage));
    out
}

/// 裂隙排行 — 全服裂隙排名
pub fn cmd_rift_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut rankings: Vec<(String, i32, i32, i32)> = Vec::new();

    // 从Global表获取所有裂隙数据
    let rows = db.query_rows(
        "SELECT id, data FROM Global WHERE section = 'temporal_rift'",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    );

    for (uid, data) in &rows {
        if uid.is_empty() || data.is_empty() || data == "[NULL]" {
            continue;
        }
        let parts: Vec<&str> = data.split(',').collect();
        if parts.len() >= 4 {
            let shards: i32 = parts[0].parse().unwrap_or(0);
            let best_level: i32 = parts[1].parse().unwrap_or(0);
            let clears: i32 = parts[3].parse().unwrap_or(0);
            rankings.push((uid.clone(), shards, best_level, clears));
        }
    }

    rankings.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    rankings.truncate(15);

    let mut out = String::from("╔══════════════════════════════╗\n");
    out.push_str("║     ⏳ 时空裂隙排行榜 ⏳     ║\n");
    out.push_str("╚══════════════════════════════╝\n\n");

    if rankings.is_empty() {
        out.push_str("暂无数据，成为第一个挑战裂隙的勇者吧!\n");
    } else {
        for (i, (uid, shards, level, clears)) in rankings.iter().enumerate() {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            out.push_str(&format!(
                "{} #{} {} — 碎片:{} | 最高:Lv.{} | 通关:{}次\n",
                medal,
                i + 1,
                uid,
                shards,
                level,
                clears
            ));
        }
    }
    out
}

/// 裂隙商店 — 查看可兑换物品
pub fn cmd_rift_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let items = get_rift_shop_items();
    let (shards, _, _, _) = get_rift_data(db, user_id);

    let mut out = String::from("╔══════════════════════════════╗\n");
    out.push_str("║     ⏳ 裂隙商店 ⏳          ║\n");
    out.push_str("╚══════════════════════════════╝\n\n");
    out.push_str(&format!("💰 当前时空碎片: {}\n\n", shards));

    for (i, item) in items.iter().enumerate() {
        let affordable = if shards >= item.cost { "✅" } else { "❌" };
        out.push_str(&format!(
            "  {}. {} {} — {}碎片 | {} {}\n",
            i + 1,
            item.emoji,
            item.name,
            item.cost,
            item.description,
            affordable
        ));
    }
    out.push_str("\n💡 输入「裂隙兑换+序号」兑换物品\n");
    out
}

/// 裂隙兑换 — 兑换商店物品
pub fn cmd_rift_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let idx: usize = args.trim().parse().unwrap_or(0);
    if idx == 0 || idx > 10 {
        return "❌ 请输入正确的商品序号(1-10)".to_string();
    }

    let items = get_rift_shop_items();
    let item = &items[idx - 1];
    let (mut shards, best_level, best_waves, clears) = get_rift_data(db, user_id);

    if shards < item.cost {
        return format!("❌ 时空碎片不足! 需要{}，当前{}", item.cost, shards);
    }

    shards -= item.cost;
    save_rift_data(db, user_id, shards, best_level, best_waves, clears);

    // 根据物品给予奖励
    match idx {
        1 => {
            db.knapsack_add(user_id, "时空精华", 1);
        }
        2 => {
            db.knapsack_add(user_id, "裂隙宝箱", 1);
        }
        3 => {
            db.knapsack_add(user_id, "时空护符", 1);
        }
        4 => {
            db.knapsack_add(user_id, "裂隙之翼", 1);
        }
        5 => {
            db.knapsack_add(user_id, "时空结晶", 1);
        }
        6 => {
            db.knapsack_add(user_id, "裂隙战甲", 1);
        }
        7 => {
            db.knapsack_add(user_id, "时空之刃", 1);
        }
        8 => {
            db.knapsack_add(user_id, "裂隙之心", 1);
        }
        9 => {
            db.knapsack_add(user_id, "时空钥匙", 1);
        }
        10 => {
            db.knapsack_add(user_id, "裂隙至尊宝箱", 1);
        }
        _ => {}
    }

    format!(
        "✅ 兑换成功!\n{} {} 已放入背包\n💰 剩余时空碎片: {}",
        item.emoji, item.name, shards
    )
}

/// 裂隙统计 — 详细裂隙挑战统计
pub fn cmd_rift_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (shards, best_level, best_waves, clears) = get_rift_data(db, user_id);
    let daily = get_daily_entries(db, user_id);
    let power = get_player_power(db, user_id);
    let levels = get_rift_levels();

    let mut out = String::from("╔══════════════════════════════╗\n");
    out.push_str("║     ⏳ 裂隙统计 ⏳          ║\n");
    out.push_str("╚══════════════════════════════╝\n\n");
    out.push_str(&format!("⚔️ 当前战力: {}\n", power));
    out.push_str(&format!("⏳ 时空碎片: {}\n", shards));
    out.push_str(&format!("🏆 最高挑战: Lv.{} — {}波\n", best_level, best_waves));
    out.push_str(&format!("🔄 完美通关: {}次\n", clears));
    out.push_str(&format!("📅 今日挑战: {}/5次\n\n", daily));

    // 下一目标
    if best_level < 6 {
        let next = levels.iter().find(|l| l.level == best_level + 1).unwrap();
        out.push_str(&format!(
            "🎯 下一目标: {} Lv.{} (需{}战力)\n",
            next.emoji, next.name, next.min_power
        ));
    } else {
        out.push_str("🎊 已征服所有裂隙!\n");
    }

    // 成就等级
    let achievement = match clears {
        0..=0 => "裂隙新手🔰",
        1..=5 => "裂隙探索者🥉",
        6..=15 => "裂隙征服者🥈",
        16..=30 => "裂隙大师🏅",
        31..=50 => "裂隙王者🎖️",
        51..=100 => "裂隙传说⭐",
        _ => "时空之主👑",
    };
    out.push_str(&format!("🏅 成就等级: {}\n", achievement));
    out
}

/// 裂隙帮助 — 显示裂隙系统帮助
pub fn cmd_rift_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("╔══════════════════════════════╗\n");
    out.push_str("║     ⏳ 时空裂隙帮助 ⏳      ║\n");
    out.push_str("╚══════════════════════════════╝\n\n");
    out.push_str("【时空裂隙系统】\n");
    out.push_str("时空裂隙是随机出现的异次元空间，内含强大怪物和稀有宝藏。\n");
    out.push_str("挑战裂隙可获得时空碎片，用于兑换专属装备和道具。\n\n");
    out.push_str("【指令列表】\n");
    out.push_str("  📋 查看裂隙 — 查看可用裂隙和进度\n");
    out.push_str("  ⚔️ 进入裂隙+等级 — 进入指定等级裂隙(1-6)\n");
    out.push_str("  📊 裂隙统计 — 查看详细挑战统计\n");
    out.push_str("  🏆 裂隙排行 — 全服裂隙排名\n");
    out.push_str("  🛒 裂隙商店 — 查看可兑换物品\n");
    out.push_str("  🔄 裂隙兑换+序号 — 兑换商店物品\n");
    out.push_str("  ❓ 裂隙帮助 — 显示此帮助\n\n");
    out.push_str("【规则说明】\n");
    out.push_str("  • 每日最多挑战5次\n");
    out.push_str("  • 6个难度等级，战力需达标\n");
    out.push_str("  • 全部波次通关=完美通关，奖励翻倍\n");
    out.push_str("  • 时空碎片可兑换专属装备\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rift_levels_count() {
        let levels = get_rift_levels();
        assert_eq!(levels.len(), 6);
    }

    #[test]
    fn test_rift_levels_escalate() {
        let levels = get_rift_levels();
        for i in 1..levels.len() {
            assert!(levels[i].base_hp > levels[i - 1].base_hp);
            assert!(levels[i].shard_reward > levels[i - 1].shard_reward);
            assert!(levels[i].min_power > levels[i - 1].min_power);
        }
    }

    #[test]
    fn test_rift_levels_unique() {
        let levels = get_rift_levels();
        let mut names: Vec<&str> = levels.iter().map(|l| l.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 6);
    }

    #[test]
    fn test_rift_shop_items_count() {
        let items = get_rift_shop_items();
        assert_eq!(items.len(), 10);
    }

    #[test]
    fn test_rift_shop_costs_escalate() {
        let items = get_rift_shop_items();
        for i in 1..items.len() {
            assert!(items[i].cost >= items[i - 1].cost);
        }
    }

    #[test]
    fn test_rift_monsters_exist() {
        for level in 1..=6 {
            let monsters = get_rift_monsters(level);
            assert!(!monsters.is_empty());
        }
    }

    #[test]
    fn test_rift_monsters_unique_per_level() {
        for level in 1..=6 {
            let monsters = get_rift_monsters(level);
            let mut sorted = monsters.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), monsters.len());
        }
    }

    #[test]
    fn test_rift_help_content() {
        let db = Database::open(":memory:").unwrap();
        let help = cmd_rift_help(&db, "", "", "", "");
        assert!(help.contains("时空裂隙"));
        assert!(help.contains("查看裂隙"));
        assert!(help.contains("进入裂隙"));
        assert!(help.contains("裂隙商店"));
    }

    #[test]
    fn test_rift_data_format() {
        // 测试数据格式
        let data = "100,3,7,5";
        let parts: Vec<&str> = data.split(',').collect();
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0].parse::<i32>().unwrap(), 100);
        assert_eq!(parts[1].parse::<i32>().unwrap(), 3);
        assert_eq!(parts[2].parse::<i32>().unwrap(), 7);
        assert_eq!(parts[3].parse::<i32>().unwrap(), 5);
    }

    #[test]
    fn test_achievement_levels() {
        assert_eq!(
            match 0i32 {
                0..=0 => "新手",
                _ => "其他",
            },
            "新手"
        );
        assert_eq!(
            match 10i32 {
                1..=5 => "低",
                6..=15 => "中",
                _ => "高",
            },
            "中"
        );
    }
}

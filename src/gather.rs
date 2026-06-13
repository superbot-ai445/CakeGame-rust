/// CakeGame 采集系统 + 副职系统
/// 支持4种采集方式：钓鱼、挖矿、采药、收集
/// 每种采集有冷却时间，采集获得随机材料物品
/// 副职系统：每种采集类型有独立的技能等级和熟练度
use crate::db::Database;
use crate::stamina;

/// 技能等级称号
const SKILL_TITLES: &[&str] = &[
    "无称号",
    "见习学徒",
    "初级工匠",
    "中级工匠",
    "高级工匠",
    "资深工匠",
    "专家级工匠",
    "大师级工匠",
    "宗师级工匠",
    "传说工匠",
    "神级工匠",
];

/// 升级所需熟练度 (索引=等级, 值=该等级升级所需累计次数)
/// 等级1需要5次, 等级2需要15次, ...
const LEVEL_THRESHOLDS: &[i32] = &[0, 5, 15, 30, 50, 80, 120, 180, 250, 350, 500];

/// 获取等级对应的称号
pub fn get_title(level: i32) -> &'static str {
    if !(0..=10).contains(&level) {
        return SKILL_TITLES[0];
    }
    SKILL_TITLES[level as usize]
}

/// 根据总采集次数计算等级
pub fn calc_level(total_count: i32) -> i32 {
    for i in (1..=10).rev() {
        if total_count >= LEVEL_THRESHOLDS[i] {
            return i as i32;
        }
    }
    0
}

/// 计算升级到下一级还需要的次数
fn count_to_next_level(total_count: i32) -> i32 {
    let current_level = calc_level(total_count);
    if current_level >= 10 {
        return 0; // 已满级
    }
    LEVEL_THRESHOLDS[(current_level + 1) as usize] - total_count
}

/// 采集类型配置
struct GatherType {
    name: &'static str,
    trigger: &'static str,
    /// 可能采集到的物品 (物品名, 基础概率百分比)
    items: &'static [(&'static str, i32)],
    /// 每次采集消耗的金币（基础值，高等级减少）
    cost_gold: i32,
    /// 冷却时间秒数（基础值，高等级减少）
    cooldown_secs: i64,
}

const GATHER_TYPES: &[GatherType] = &[
    GatherType {
        name: "钓鱼",
        trigger: "钓鱼",
        items: &[("小鱼", 50), ("鲤鱼", 30), ("金鱼", 15), ("神秘鱼", 5)],
        cost_gold: 10,
        cooldown_secs: 60,
    },
    GatherType {
        name: "挖矿",
        trigger: "挖矿",
        items: &[("铜矿石", 50), ("铁矿石", 30), ("金矿石", 15), ("钻石矿石", 5)],
        cost_gold: 20,
        cooldown_secs: 120,
    },
    GatherType {
        name: "采药",
        trigger: "采药",
        items: &[("草药", 50), ("灵芝", 30), ("仙草", 15), ("神农草", 5)],
        cost_gold: 15,
        cooldown_secs: 90,
    },
    GatherType {
        name: "收集",
        trigger: "收集",
        items: &[("木材", 50), ("石头", 30), ("水晶", 15), ("神秘碎片", 5)],
        cost_gold: 5,
        cooldown_secs: 30,
    },
];

/// 根据等级计算实际冷却时间（每级减少3%，最多30%）
fn calc_cooldown(base: i64, level: i32) -> i64 {
    let reduction = std::cmp::min(level * 3, 30);
    base * (100 - reduction as i64) / 100
}

/// 根据等级计算实际金币消耗（每级减少2%，最多20%）
fn calc_cost(base: i32, level: i32) -> i32 {
    let reduction = std::cmp::min(level * 2, 20);
    let result = base * (100 - reduction) / 100;
    std::cmp::max(result, 1) // 至少1金币
}

/// 根据等级提升稀有物品概率（每级+1%到稀有项）
fn adjust_rare_prob(base_prob: i32, item_index: usize, level: i32) -> i32 {
    // 只提升最后两项（稀有物品，index >= 2）
    if item_index >= 2 {
        base_prob + level
    } else {
        base_prob
    }
}

/// 查看采集信息
pub fn cmd_view_gather(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let mut r = format!("{}\n═══ 采集系统 ═══", prefix);
    r.push_str("\n\n可进行的采集活动：");

    for gt in GATHER_TYPES {
        let count_key = format!("gather_{}_count", gt.trigger);
        let count: i32 = db.read_user_data(user_id, &count_key).parse().unwrap_or(0);
        let level = calc_level(count);
        let actual_cooldown = calc_cooldown(gt.cooldown_secs, level);
        let actual_cost = calc_cost(gt.cost_gold, level);

        // 检查冷却
        let key = format!("gather_{}_cd", gt.trigger);
        let last_time: i64 = db.read_user_data(user_id, &key).parse().unwrap_or(0);
        let now = chrono::Local::now().timestamp();
        let remaining = if last_time > 0 {
            let elapsed = now - last_time;
            if elapsed < actual_cooldown {
                actual_cooldown - elapsed
            } else {
                0
            }
        } else {
            0
        };

        r.push_str(&format!("\n\n【{} Lv.{}】{}", level, gt.name, get_title(level)));
        r.push_str(&format!("\n  消耗：{}金币", actual_cost));
        if level > 0 {
            r.push_str(&format!(" (原{})", gt.cost_gold));
        }
        r.push_str(&format!("\n  冷却：{}秒", actual_cooldown));
        if level > 0 {
            r.push_str(&format!(" (原{})", gt.cooldown_secs));
        }
        if remaining > 0 {
            r.push_str(&format!("\n  ⏳ 冷却中 (剩余{}秒)", remaining));
        } else {
            r.push_str("\n  ✅ 可采集");
        }
    }

    r.push_str("\n\n发送'采集+类型'进行采集（如：采集+钓鱼）");
    r.push_str("\n发送'副职'查看采集技能详情");
    r
}

/// 执行采集
pub fn cmd_gather(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let gather_type = args.trim();

    if gather_type.is_empty() {
        return format!("{}\n请指定采集类型！\n可选：钓鱼、挖矿、采药、收集", prefix);
    }

    // 查找匹配的采集类型
    let gt = match GATHER_TYPES
        .iter()
        .find(|g| g.trigger == gather_type || g.name == gather_type)
    {
        Some(g) => g,
        None => {
            return format!(
                "{}\n未知的采集类型 [{}]\n可选：钓鱼、挖矿、采药、收集",
                prefix, gather_type
            );
        }
    };

    // 读取当前等级
    let count_key = format!("gather_{}_count", gt.trigger);
    let count: i32 = db.read_user_data(user_id, &count_key).parse().unwrap_or(0);
    let level = calc_level(count);
    let actual_cooldown = calc_cooldown(gt.cooldown_secs, level);
    let actual_cost = calc_cost(gt.cost_gold, level);

    // 检查冷却
    let cd_key = format!("gather_{}_cd", gt.trigger);
    let last_time: i64 = db.read_user_data(user_id, &cd_key).parse().unwrap_or(0);
    let now = chrono::Local::now().timestamp();
    if last_time > 0 && now - last_time < actual_cooldown {
        let remaining = actual_cooldown - (now - last_time);
        return format!("{}\n⏳ {}冷却中，还需等待{}秒", prefix, gt.name, remaining);
    }

    // 体力检查 (采集消耗1体力)
    if let Err(e) = stamina::consume_stamina(user_id, "采集", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 检查金币
    let gold = db.read_currency(user_id, crate::core::CURRENCY_GOLD);
    if gold < actual_cost as i64 {
        return format!(
            "{}\n金币不足！{}需要{}金币，当前{}金币",
            prefix, gt.name, actual_cost, gold
        );
    }

    // 扣除金币
    db.modify_currency(user_id, crate::core::CURRENCY_GOLD, "sub", actual_cost as i64);

    // 随机选择采集物品（等级影响稀有物品概率）
    let roll: i32 = rand::random::<i32>() % 100;
    let mut cumulative = 0;
    let mut gathered_item = gt.items[0].0;
    for (idx, &(item, prob)) in gt.items.iter().enumerate() {
        cumulative += adjust_rare_prob(prob, idx, level);
        if roll < cumulative {
            gathered_item = item;
            break;
        }
    }

    // 添加物品到背包
    db.knapsack_add(user_id, gathered_item, 1);

    // 更新冷却时间
    db.write_user_data(user_id, &cd_key, &now.to_string());

    // 成就追踪
    crate::achievement::on_gathered(db, user_id);

    // 更新采集次数统计
    let new_count = count + 1;
    db.write_user_data(user_id, &count_key, &new_count.to_string());

    // 检查是否升级
    let old_level = level;
    let new_level = calc_level(new_count);
    let level_up_msg = if new_level > old_level {
        format!(
            "\n\n🎉 技能升级！{} Lv.{} → Lv.{}！\n新称号：{}",
            gt.name,
            old_level,
            new_level,
            get_title(new_level)
        )
    } else {
        String::new()
    };

    let to_next = count_to_next_level(new_count);
    let progress_msg = if new_level >= 10 {
        "\n⭐ 已达最高等级！".to_string()
    } else {
        format!("\n距下一级还需{}次采集", to_next)
    };

    // 每日任务进度追踪
    crate::daily_quest::on_gathered(db, user_id);

    // 周常任务进度追踪
    crate::weekly_quest::on_gathered(db, user_id);

    format!(
        "{}\n🎣 {}成功！\n\n获得：{} ×1\n消耗：{}金币\n\n{}等级：Lv.{} [{}]{}{}",
        prefix,
        gt.name,
        gathered_item,
        actual_cost,
        gt.name,
        new_level,
        get_title(new_level),
        level_up_msg,
        progress_msg
    )
}

/// 查看采集统计
pub fn cmd_gather_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let mut r = format!("{}\n═══ 采集统计 ═══", prefix);

    for gt in GATHER_TYPES {
        let count_key = format!("gather_{}_count", gt.trigger);
        let count: i32 = db.read_user_data(user_id, &count_key).parse().unwrap_or(0);
        let level = calc_level(count);
        r.push_str(&format!(
            "\n{} Lv.{} [{}]：{}次",
            gt.name,
            level,
            get_title(level),
            count
        ));
    }

    r
}

/// 查看副职详情
pub fn cmd_view_subprofession(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let mut r = format!("{}\n═══ 副职技能 ═══", prefix);
    r.push_str("\n采集技能等级影响：");
    r.push_str("\n  • 稀有物品概率提升");
    r.push_str("\n  • 采集冷却时间缩短");
    r.push_str("\n  • 金币消耗降低");

    for gt in GATHER_TYPES {
        let count_key = format!("gather_{}_count", gt.trigger);
        let count: i32 = db.read_user_data(user_id, &count_key).parse().unwrap_or(0);
        let level = calc_level(count);
        let title = get_title(level);
        let actual_cooldown = calc_cooldown(gt.cooldown_secs, level);
        let actual_cost = calc_cost(gt.cost_gold, level);
        let to_next = count_to_next_level(count);

        r.push_str(&format!("\n\n━━━ {} ━━━", gt.name));
        r.push_str(&format!("\n🏷️ 称号：{}", title));
        r.push_str(&format!("\n📊 等级：Lv.{}/10", level));
        r.push_str(&format!("\n📈 累计采集：{}次", count));

        if level >= 10 {
            r.push_str("\n⭐ 已达最高等级");
        } else {
            r.push_str(&format!(
                "\n⏳ 升级进度：{}/{} (还需{}次)",
                count,
                LEVEL_THRESHOLDS[level as usize + 1],
                to_next
            ));
        }

        // 显示等级加成
        if level > 0 {
            let cd_reduction = std::cmp::min(level * 3, 30);
            let cost_reduction = std::cmp::min(level * 2, 20);
            r.push_str("\n✨ 加成效果：");
            r.push_str(&format!(
                "\n  冷却 -{}% ({}秒→{}秒)",
                cd_reduction, gt.cooldown_secs, actual_cooldown
            ));
            r.push_str(&format!(
                "\n  消耗 -{}% ({}金→{}金)",
                cost_reduction, gt.cost_gold, actual_cost
            ));
            r.push_str(&format!("\n  稀有率 +{}%", level));
        }

        // 显示可获得物品及概率
        r.push_str("\n🎁 可获得物品：");
        for (idx, &(item, base_prob)) in gt.items.iter().enumerate() {
            let actual_prob = adjust_rare_prob(base_prob, idx, level);
            let prob_display = std::cmp::min(actual_prob, 95); // cap at 95%
            r.push_str(&format!("\n  {} ×1 ({}%)", item, prob_display));
        }
    }

    // 显示等级称号一览
    r.push_str("\n\n━━━ 等级称号一览 ━━━");
    for i in 1..=10 {
        r.push_str(&format!(
            "\nLv.{:2} [{}] - 需{}次",
            i, SKILL_TITLES[i], LEVEL_THRESHOLDS[i]
        ));
    }

    r
}

/// 采集排行榜 — 全服采集达人排名
pub fn cmd_gather_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let filter = args.trim();

    // 确定筛选类型
    let target_type = if filter.is_empty() {
        None
    } else {
        GATHER_TYPES.iter().find(|g| g.trigger == filter || g.name == filter)
    };

    if !filter.is_empty() && target_type.is_none() {
        return format!(
            "{}\n未知的采集类型 [{}]\n可选：钓鱼、挖矿、采药、收集\n或不带参数查看综合排名",
            prefix, filter
        );
    }

    // 读取所有用户数据
    let user_ids = db.all_users();

    // 收集每个用户的采集数据
    let mut rankings: Vec<(String, String, i32, i32)> = Vec::new(); // (uid, name, total, max_level)

    for uid in &user_ids {
        let name = db.read_basic(uid, crate::core::ITEM_NAME);
        let name = if name.is_empty() { uid.clone() } else { name };

        if let Some(gt) = target_type {
            // 按类型排名
            let count_key = format!("gather_{}_count", gt.trigger);
            let count: i32 = db.read_user_data(uid, &count_key).parse().unwrap_or(0);
            if count > 0 {
                let level = calc_level(count);
                rankings.push((uid.clone(), name, count, level));
            }
        } else {
            // 综合排名 — 总采集次数
            let mut total = 0i32;
            let mut max_lv = 0i32;
            for gt in GATHER_TYPES {
                let count_key = format!("gather_{}_count", gt.trigger);
                let count: i32 = db.read_user_data(uid, &count_key).parse().unwrap_or(0);
                total += count;
                let lv = calc_level(count);
                if lv > max_lv {
                    max_lv = lv;
                }
            }
            if total > 0 {
                rankings.push((uid.clone(), name, total, max_lv));
            }
        }
    }

    // 按总次数降序排序
    rankings.sort_by_key(|x| std::cmp::Reverse(x.2));

    let type_name = if let Some(gt) = target_type { gt.name } else { "综合" };

    let mut r = format!("{}\n═══ 🏆 采集排行榜 [{}] ═══", prefix, type_name);

    if rankings.is_empty() {
        r.push_str("\n\n暂无采集记录\n发送'采集+类型'开始采集吧！");
    } else {
        let display_count = std::cmp::min(rankings.len(), 10);
        let mut entries: Vec<(usize, String, i32, String)> = Vec::new();
        for (i, (ref _uid, ref name, _count, level)) in rankings.iter().enumerate().take(display_count) {
            entries.push((i + 1, name.clone(), *level, get_title(*level).to_string()));
        }
        r.push_str(&format!(
            "\n{}",
            crate::template_render::render_gather_ranking(db, &entries)
        ));

        // 显示当前用户排名
        let my_rank = rankings.iter().position(|(uid, _, _, _)| uid == user_id);
        if let Some(pos) = my_rank {
            if pos >= 10 {
                let (_, _name, count, level) = &rankings[pos];
                r.push_str(&format!(
                    "\n\n📍 您的排名：第{}名 | Lv.{} [{}] | {}次",
                    pos + 1,
                    level,
                    get_title(*level),
                    count
                ));
            }
        } else {
            r.push_str("\n\n📍 您暂无采集记录");
        }

        r.push_str(&format!("\n\n📊 共{}位玩家参与采集", rankings.len()));
    }

    r.push_str("\n\n💡 发送'采集排行+类型'查看分类排名\n可选：钓鱼、挖矿、采药、收集");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_level_zero() {
        assert_eq!(calc_level(0), 0);
        assert_eq!(calc_level(4), 0);
    }

    #[test]
    fn test_calc_level_progression() {
        assert_eq!(calc_level(5), 1);
        assert_eq!(calc_level(15), 2);
        assert_eq!(calc_level(30), 3);
        assert_eq!(calc_level(50), 4);
        assert_eq!(calc_level(80), 5);
        assert_eq!(calc_level(120), 6);
        assert_eq!(calc_level(180), 7);
        assert_eq!(calc_level(250), 8);
        assert_eq!(calc_level(350), 9);
        assert_eq!(calc_level(500), 10);
    }

    #[test]
    fn test_calc_level_boundary() {
        // Just below threshold stays at previous level
        assert_eq!(calc_level(4), 0);
        assert_eq!(calc_level(14), 1);
        assert_eq!(calc_level(29), 2);
        assert_eq!(calc_level(499), 9);
        // Above max threshold stays at 10
        assert_eq!(calc_level(999), 10);
        assert_eq!(calc_level(10000), 10);
    }

    #[test]
    fn test_get_title_all_levels() {
        assert_eq!(get_title(0), "无称号");
        assert_eq!(get_title(1), "见习学徒");
        assert_eq!(get_title(5), "资深工匠");
        assert_eq!(get_title(10), "神级工匠");
        // Out of bounds returns default
        assert_eq!(get_title(-1), "无称号");
        assert_eq!(get_title(11), "无称号");
    }

    #[test]
    fn test_calc_cooldown_reduction() {
        // Level 0: no reduction
        assert_eq!(calc_cooldown(60, 0), 60);
        // Level 1: -3%
        assert_eq!(calc_cooldown(100, 1), 97);
        // Level 5: -15%
        assert_eq!(calc_cooldown(100, 5), 85);
        // Level 10: -30% (max)
        assert_eq!(calc_cooldown(100, 10), 70);
        // Level 15: still -30% (capped)
        assert_eq!(calc_cooldown(100, 15), 70);
    }

    #[test]
    fn test_calc_cost_reduction() {
        // Level 0: no reduction
        assert_eq!(calc_cost(20, 0), 20);
        // Level 1: -2%
        assert_eq!(calc_cost(100, 1), 98);
        // Level 5: -10%
        assert_eq!(calc_cost(100, 5), 90);
        // Level 10: -20% (max)
        assert_eq!(calc_cost(100, 10), 80);
        // Minimum 1 gold
        assert_eq!(calc_cost(1, 10), 1);
    }

    #[test]
    fn test_adjust_rare_prob() {
        // Common items (index < 2) don't get boosted
        assert_eq!(adjust_rare_prob(50, 0, 5), 50);
        assert_eq!(adjust_rare_prob(30, 1, 5), 30);
        // Rare items (index >= 2) get +level%
        assert_eq!(adjust_rare_prob(15, 2, 0), 15);
        assert_eq!(adjust_rare_prob(15, 2, 5), 20);
        assert_eq!(adjust_rare_prob(5, 3, 10), 15);
    }

    #[test]
    fn test_count_to_next_level() {
        // From 0, need 5 to reach level 1
        assert_eq!(count_to_next_level(0), 5);
        // From 3, need 2 more to reach level 1
        assert_eq!(count_to_next_level(3), 2);
        // From 5 (level 1), need 10 more to reach level 2
        assert_eq!(count_to_next_level(5), 10);
        // Max level returns 0
        assert_eq!(count_to_next_level(500), 0);
        assert_eq!(count_to_next_level(999), 0);
    }
}

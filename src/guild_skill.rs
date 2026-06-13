/// CakeGame 公会科技系统
///
/// 公会级别的被动加成研究系统。公会成员共同投入资源解锁科技，
/// 所有公会成员共享科技加成效果。科技有多级，逐级解锁更强效果。
///
/// 指令: 公会科技, 科技详情, 研究科技, 科技贡献
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 科技定义
struct TechDef {
    id: &'static str,
    name: &'static str,
    desc: &'static str,
    category: &'static str,
    max_level: i32,
    /// 每级研究所需贡献值（基数）
    base_cost: i64,
    /// 每级成本递增系数 (%)
    cost_scale: i32,
    /// 每级加成效果（描述）
    effect_per_level: &'static str,
    /// 加成属性键（用于计算实际加成）
    attr_key: &'static str,
    /// 每级加成数值
    value_per_level: f64,
    /// 解锁所需公会等级
    unlock_guild_level: i32,
}

const TECHS: &[TechDef] = &[
    // === 基础科技 (公会1级解锁) ===
    TechDef {
        id: "hp_boost",
        name: "生命强化",
        desc: "提升所有公会成员的最大生命值",
        category: "基础",
        max_level: 10,
        base_cost: 500,
        cost_scale: 150,
        effect_per_level: "最大生命+%d",
        attr_key: "hp",
        value_per_level: 50.0,
        unlock_guild_level: 1,
    },
    TechDef {
        id: "mp_boost",
        name: "魔力强化",
        desc: "提升所有公会成员的最大魔法值",
        category: "基础",
        max_level: 10,
        base_cost: 500,
        cost_scale: 150,
        effect_per_level: "最大魔法+%d",
        attr_key: "mp",
        value_per_level: 30.0,
        unlock_guild_level: 1,
    },
    TechDef {
        id: "exp_boost",
        name: "经验加成",
        desc: "提升公会成员战斗获得的经验值",
        category: "基础",
        max_level: 10,
        base_cost: 800,
        cost_scale: 200,
        effect_per_level: "经验加成+%d%",
        attr_key: "exp_bonus",
        value_per_level: 3.0,
        unlock_guild_level: 1,
    },
    // === 战斗科技 (公会2级解锁) ===
    TechDef {
        id: "ad_boost",
        name: "物攻强化",
        desc: "提升所有公会成员的物理攻击力",
        category: "战斗",
        max_level: 10,
        base_cost: 1000,
        cost_scale: 200,
        effect_per_level: "物攻+%d",
        attr_key: "ad",
        value_per_level: 10.0,
        unlock_guild_level: 2,
    },
    TechDef {
        id: "ap_boost",
        name: "魔攻强化",
        desc: "提升所有公会成员的魔法攻击力",
        category: "战斗",
        max_level: 10,
        base_cost: 1000,
        cost_scale: 200,
        effect_per_level: "魔攻+%d",
        attr_key: "ap",
        value_per_level: 10.0,
        unlock_guild_level: 2,
    },
    TechDef {
        id: "crit_boost",
        name: "暴击强化",
        desc: "提升所有公会成员的暴击率",
        category: "战斗",
        max_level: 8,
        base_cost: 1200,
        cost_scale: 250,
        effect_per_level: "暴击+%d",
        attr_key: "crit",
        value_per_level: 2.0,
        unlock_guild_level: 2,
    },
    // === 防御科技 (公会3级解锁) ===
    TechDef {
        id: "def_boost",
        name: "防御强化",
        desc: "提升所有公会成员的物理防御力",
        category: "防御",
        max_level: 10,
        base_cost: 1000,
        cost_scale: 200,
        effect_per_level: "防御+%d",
        attr_key: "defense",
        value_per_level: 8.0,
        unlock_guild_level: 3,
    },
    TechDef {
        id: "mdf_boost",
        name: "魔抗强化",
        desc: "提升所有公会成员的魔法抗性",
        category: "防御",
        max_level: 10,
        base_cost: 1000,
        cost_scale: 200,
        effect_per_level: "魔抗+%d",
        attr_key: "magic_res",
        value_per_level: 8.0,
        unlock_guild_level: 3,
    },
    TechDef {
        id: "absorb_boost",
        name: "吸血强化",
        desc: "提升所有公会成员的吸血比例",
        category: "防御",
        max_level: 5,
        base_cost: 2000,
        cost_scale: 300,
        effect_per_level: "吸血+%d",
        attr_key: "absorb_hp",
        value_per_level: 2.0,
        unlock_guild_level: 3,
    },
    // === 经济科技 (公会2级解锁) ===
    TechDef {
        id: "gold_boost",
        name: "金币加成",
        desc: "提升公会成员战斗获得的金币",
        category: "经济",
        max_level: 10,
        base_cost: 800,
        cost_scale: 200,
        effect_per_level: "金币加成+%d%",
        attr_key: "gold_bonus",
        value_per_level: 3.0,
        unlock_guild_level: 2,
    },
    // === 高级科技 (公会4级解锁) ===
    TechDef {
        id: "hit_boost",
        name: "命中强化",
        desc: "提升所有公会成员的命中值",
        category: "高级",
        max_level: 8,
        base_cost: 1500,
        cost_scale: 300,
        effect_per_level: "命中+%d",
        attr_key: "hit",
        value_per_level: 3.0,
        unlock_guild_level: 4,
    },
    TechDef {
        id: "dodge_boost",
        name: "闪避强化",
        desc: "提升所有公会成员的闪避值",
        category: "高级",
        max_level: 8,
        base_cost: 1500,
        cost_scale: 300,
        effect_per_level: "闪避+%d",
        attr_key: "dodge",
        value_per_level: 3.0,
        unlock_guild_level: 4,
    },
];

/// 获取公会等级
fn get_guild_level(db: &Database, guild_name: &str) -> i32 {
    db.global_get("UnionData", &format!("{}.Level", guild_name))
        .parse()
        .unwrap_or(1)
}

/// 获取科技当前等级
fn get_tech_level(db: &Database, guild_name: &str, tech_id: &str) -> i32 {
    db.global_get("guild_tech", &format!("{}.{}", guild_name, tech_id))
        .parse()
        .unwrap_or(0)
}

/// 设置科技等级
fn set_tech_level(db: &Database, guild_name: &str, tech_id: &str, level: i32) {
    db.global_set("guild_tech", &format!("{}.{}", guild_name, tech_id), &level.to_string());
}

/// 获取公会当前总贡献值
fn get_guild_contribution(db: &Database, guild_name: &str) -> i64 {
    db.global_get("guild_tech", &format!("{}.contrib", guild_name))
        .parse()
        .unwrap_or(0)
}

/// 增加公会贡献值
fn add_guild_contribution(db: &Database, guild_name: &str, amount: i64) {
    let current = get_guild_contribution(db, guild_name);
    db.global_set(
        "guild_tech",
        &format!("{}.contrib", guild_name),
        &(current + amount).to_string(),
    );
}

/// 计算研究某级科技所需贡献值
fn calc_research_cost(tech: &TechDef, target_level: i32) -> i64 {
    let base = tech.base_cost as f64;
    let scale = tech.cost_scale as f64 / 100.0;
    (base * scale.powi(target_level - 1)) as i64
}

/// 获取公会已研究的科技加成汇总（供其他模块调用）
pub fn get_guild_tech_bonus(db: &Database, user_id: &str) -> std::collections::HashMap<String, f64> {
    let mut bonuses = std::collections::HashMap::new();
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return bonuses;
    }

    for tech in TECHS {
        let level = get_tech_level(db, &guild, tech.id);
        if level > 0 {
            let total = tech.value_per_level * level as f64;
            *bonuses.entry(tech.attr_key.to_string()).or_insert(0.0) += total;
        }
    }
    bonuses
}

/// 查看公会科技
pub fn cmd_view_guild_tech(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您还未加入任何公会，无法查看公会科技。", prefix);
    }

    let guild_level = get_guild_level(db, &guild);
    let contrib = get_guild_contribution(db, &guild);

    let mut out = format!(
        "{}\n═══ 🔬 公会科技 — {} ═══\n\n🏛 公会等级: Lv.{}\n💎 总贡献值: {}\n",
        prefix, guild, guild_level, contrib
    );

    // 按分类整理
    let categories = ["基础", "战斗", "防御", "经济", "高级"];
    for cat in &categories {
        let cat_techs: Vec<_> = TECHS.iter().filter(|t| t.category == *cat).collect();
        if cat_techs.is_empty() {
            continue;
        }
        out.push_str(&format!("\n┌── {}科技 ──\n", cat));
        for tech in &cat_techs {
            let level = get_tech_level(db, &guild, tech.id);
            let locked = guild_level < tech.unlock_guild_level;
            if locked {
                out.push_str(&format!("│ 🔒 {} (需公会等级{})\n", tech.name, tech.unlock_guild_level));
            } else if level >= tech.max_level {
                out.push_str(&format!("│ ✅ {} Lv.MAX — {}\n", tech.name, tech.desc));
            } else {
                let cost = calc_research_cost(tech, level + 1);
                out.push_str(&format!(
                    "│ ⚙ {} Lv.{}/{} — 下一级: {}贡献\n│   {} 效果: {}\n",
                    tech.name,
                    level,
                    tech.max_level,
                    cost,
                    if level > 0 { "当前" } else { "未研究" },
                    if level > 0 {
                        format_effect(tech, level)
                    } else {
                        tech.desc.to_string()
                    },
                ));
            }
        }
        out.push_str("└──────────────\n");
    }

    out.push_str("\n💡 使用「科技详情+科技名」查看详细信息\n");
    out.push_str("💡 使用「研究科技+科技名」研究科技\n");
    out.push_str("💡 使用「科技贡献+金币/钻石+数量」贡献资源\n");
    out
}

/// 格式化效果描述
fn format_effect(tech: &TechDef, level: i32) -> String {
    let value = tech.value_per_level * level as f64;
    tech.effect_per_level.replace("%d", &format!("{}", value as i32))
}

/// 科技详情
pub fn cmd_tech_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您还未加入任何公会。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!("{}\n❓ 请指定科技名！用法：科技详情+科技名", prefix);
    }

    // 模糊匹配
    let tech = TECHS.iter().find(|t| t.name.contains(args) || args.contains(t.name));
    let tech = match tech {
        Some(t) => t,
        None => {
            return format!(
                "{}\n❌ 未找到名为「{}」的科技。使用「公会科技」查看所有科技。",
                prefix, args
            )
        }
    };

    let guild_level = get_guild_level(db, &guild);
    let level = get_tech_level(db, &guild, tech.id);
    let locked = guild_level < tech.unlock_guild_level;

    let mut out = format!("{}\n═══ 🔬 科技详情 — {} ═══\n\n", prefix, tech.name);
    out.push_str(&format!("📋 描述: {}\n", tech.desc));
    out.push_str(&format!("📂 分类: {}科技\n", tech.category));
    out.push_str(&format!("📊 最高等级: {}\n", tech.max_level));

    if locked {
        out.push_str(&format!(
            "🔒 需要公会等级: {} (当前: {})\n",
            tech.unlock_guild_level, guild_level
        ));
    } else {
        out.push_str(&format!("📈 当前等级: {}/{}\n", level, tech.max_level));
        if level > 0 {
            out.push_str(&format!("✨ 当前效果: {}\n", format_effect(tech, level)));
        }
        if level < tech.max_level {
            let cost = calc_research_cost(tech, level + 1);
            out.push_str(&format!("💰 研究下一级需要: {}贡献\n", cost));
            out.push_str(&format!("🔮 下一级效果: {}\n", format_effect(tech, level + 1)));
        } else {
            out.push_str("✅ 已达到最高等级！\n");
        }
    }

    // 显示全级消耗表
    out.push_str("\n📊 等级消耗一览:\n");
    for lv in 1..=tech.max_level {
        let cost = calc_research_cost(tech, lv);
        let effect = format_effect(tech, lv);
        let marker = if lv == level { " ◀ 当前" } else { "" };
        out.push_str(&format!("  Lv.{}: {}贡献 → {}{}\n", lv, cost, effect, marker));
    }

    out
}

/// 研究科技（消耗公会贡献）
pub fn cmd_research_tech(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您还未加入任何公会。", prefix);
    }

    // 检查是否是会长
    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n❌ 只有公会会长可以研究科技！\n💡 请联系会长操作。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!("{}\n❓ 请指定科技名！用法：研究科技+科技名", prefix);
    }

    // 模糊匹配
    let tech = TECHS.iter().find(|t| t.name.contains(args) || args.contains(t.name));
    let tech = match tech {
        Some(t) => t,
        None => return format!("{}\n❌ 未找到名为「{}」的科技。", prefix, args),
    };

    let guild_level = get_guild_level(db, &guild);
    if guild_level < tech.unlock_guild_level {
        return format!(
            "{}\n🔒 科技「{}」需要公会等级{}，当前公会等级{}。",
            prefix, tech.name, tech.unlock_guild_level, guild_level
        );
    }

    let current_level = get_tech_level(db, &guild, tech.id);
    if current_level >= tech.max_level {
        return format!("{}\n✅ 科技「{}」已达到最高等级{}！", prefix, tech.name, tech.max_level);
    }

    let next_level = current_level + 1;
    let cost = calc_research_cost(tech, next_level);
    let contrib = get_guild_contribution(db, &guild);

    if contrib < cost {
        return format!(
            "{}\n❌ 贡献值不足！需要 {} 当前 {}（还差 {}）\n💡 使用「科技贡献+金币/钻石+数量」增加公会贡献。",
            prefix,
            cost,
            contrib,
            cost - contrib
        );
    }

    // 扣除贡献，提升科技等级
    add_guild_contribution(db, &guild, -cost);
    set_tech_level(db, &guild, tech.id, next_level);

    format!(
        "{}\n✅ 🔬 公会科技「{}」研究成功！\n\n📊 等级: {} → {}\n✨ 效果: {}\n💎 消耗贡献: {}\n🏛 剩余贡献: {}\n\n🎉 所有公会成员已获得加成！",
        prefix,
        tech.name,
        current_level,
        next_level,
        format_effect(tech, next_level),
        cost,
        get_guild_contribution(db, &guild)
    )
}

/// 科技贡献（投入资源增加公会贡献值）
pub fn cmd_tech_contribute(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n❌ 您还未加入任何公会。", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n❓ 请指定贡献方式！\n\n💡 用法：\n  科技贡献+金币+数量\n  科技贡献+钻石+数量\n\n📊 贡献比例：\n  1000金币 = 1贡献\n  1钻石 = 5贡献",
            prefix
        );
    }

    let parts: Vec<&str> = args.splitn(2, ['+', ' ']).collect();
    if parts.len() < 2 {
        return format!("{}\n❓ 格式：科技贡献+金币/钻石+数量", prefix);
    }

    let currency = parts[0].trim();
    let amount_str = parts[1].trim();

    let amount: i64 = match amount_str.parse() {
        Ok(n) if n > 0 => n,
        _ => return format!("{}\n❌ 请输入有效的正整数数量！", prefix),
    };

    let (currency_key, contrib_per_unit, display_name) = match currency {
        "金币" | "金" | "gold" => (CURRENCY_GOLD, 1.0_f64 / 1000.0, "金币"),
        "钻石" | "钻" | "diamond" => (CURRENCY_DIAMOND, 5.0, "钻石"),
        _ => return format!("{}\n❌ 不支持的货币类型「{}」。支持：金币、钻石", prefix, currency),
    };

    let balance = db.read_currency(user_id, currency_key);
    if balance < amount {
        return format!("{}\n❌ {}不足！需要 {} 当前 {}", prefix, display_name, amount, balance);
    }

    let contrib_gained = (amount as f64 * contrib_per_unit) as i64;
    if contrib_gained < 1 {
        return format!("{}\n❌ 贡献数量太少了！至少需要1000金币或1钻石。", prefix);
    }

    // 扣除货币
    db.modify_currency(user_id, currency_key, OP_SUB, amount);
    add_guild_contribution(db, &guild, contrib_gained);

    format!(
        "{}\n✅ 💰 科技贡献成功！\n\n📤 消耗: {} {}\n💎 获得贡献: {}\n🏛 公会总贡献: {}\n🏆 个人剩余: {} {}\n\n💡 使用「研究科技+科技名」研究科技",
        prefix,
        amount,
        display_name,
        contrib_gained,
        get_guild_contribution(db, &guild),
        db.read_currency(user_id, currency_key),
        display_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_research_cost() {
        let tech = &TECHS[0]; // 生命强化: base=500, scale=150%
        assert_eq!(calc_research_cost(tech, 1), 500); // 500 * 1.5^0 = 500
        assert_eq!(calc_research_cost(tech, 2), 750); // 500 * 1.5^1 = 750
        assert_eq!(calc_research_cost(tech, 3), 1125); // 500 * 1.5^2 = 1125
                                                       // Level 10 should be expensive
        let cost_10 = calc_research_cost(tech, 10);
        assert!(cost_10 > 5000, "Level 10 cost {} should be > 5000", cost_10);
    }

    #[test]
    fn test_tech_definitions() {
        // All techs should have valid data
        for tech in TECHS {
            assert!(!tech.id.is_empty(), "Tech ID empty");
            assert!(!tech.name.is_empty(), "Tech name empty");
            assert!(tech.max_level > 0, "Tech {} max_level invalid", tech.name);
            assert!(tech.base_cost > 0, "Tech {} base_cost invalid", tech.name);
            assert!(tech.cost_scale > 0, "Tech {} cost_scale invalid", tech.name);
            assert!(tech.value_per_level > 0.0, "Tech {} value invalid", tech.name);
            assert!(tech.unlock_guild_level > 0, "Tech {} unlock level invalid", tech.name);
        }
        assert_eq!(TECHS.len(), 12, "Should have 12 techs");
    }

    #[test]
    fn test_tech_categories() {
        let cats: Vec<&str> = TECHS.iter().map(|t| t.category).collect();
        assert!(cats.contains(&"基础"));
        assert!(cats.contains(&"战斗"));
        assert!(cats.contains(&"防御"));
        assert!(cats.contains(&"经济"));
        assert!(cats.contains(&"高级"));
    }

    #[test]
    fn test_tech_unlock_levels() {
        // 基础科技应1级解锁
        for tech in TECHS.iter().filter(|t| t.category == "基础") {
            assert_eq!(tech.unlock_guild_level, 1, "{} should unlock at level 1", tech.name);
        }
        // 战斗/经济科技应2级解锁
        for tech in TECHS.iter().filter(|t| t.category == "战斗" || t.category == "经济") {
            assert_eq!(tech.unlock_guild_level, 2, "{} should unlock at level 2", tech.name);
        }
        // 防御科技应3级解锁
        for tech in TECHS.iter().filter(|t| t.category == "防御") {
            assert_eq!(tech.unlock_guild_level, 3, "{} should unlock at level 3", tech.name);
        }
        // 高级科技应4级解锁
        for tech in TECHS.iter().filter(|t| t.category == "高级") {
            assert_eq!(tech.unlock_guild_level, 4, "{} should unlock at level 4", tech.name);
        }
    }

    #[test]
    fn test_format_effect() {
        let tech = &TECHS[0]; // 生命强化: value_per_level=50.0
        assert_eq!(format_effect(tech, 1), "最大生命+50");
        assert_eq!(format_effect(tech, 5), "最大生命+250");
        assert_eq!(format_effect(tech, 10), "最大生命+500");

        let exp_tech = &TECHS[2]; // 经验加成: value_per_level=3.0
        assert_eq!(format_effect(exp_tech, 1), "经验加成+3%");
        assert_eq!(format_effect(exp_tech, 5), "经验加成+15%");
    }

    #[test]
    fn test_cost_increases_with_level() {
        let tech = &TECHS[0];
        for lv in 1..tech.max_level {
            let cost_now = calc_research_cost(tech, lv);
            let cost_next = calc_research_cost(tech, lv + 1);
            assert!(
                cost_next > cost_now,
                "Cost should increase: Lv{}={} < Lv{}={}",
                lv,
                cost_now,
                lv + 1,
                cost_next
            );
        }
    }

    #[test]
    fn test_tech_ids_unique() {
        let mut ids: Vec<&str> = TECHS.iter().map(|t| t.id).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Tech IDs should be unique");
    }
}

/// CakeGame 战斗风格系统
/// 允许玩家选择不同的战斗风格，影响战斗中的属性加成
/// 4种风格: 进攻型/防御型/均衡型/暴击型
/// 数据存储: Global表 SECTION='combat_style'
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 战斗风格定义
struct CombatStyle {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    description: &'static str,
    /// (属性名, 加成百分比) 列表
    bonuses: &'static [(&'static str, i32)],
}

/// 所有可用的战斗风格
const STYLES: &[CombatStyle] = &[
    CombatStyle {
        id: "aggressive",
        name: "进攻型",
        emoji: "⚔️",
        description: "牺牲防御换取强大攻击力，适合速战速决",
        bonuses: &[("AD", 15), ("AP", 15), ("Defense", -10), ("MagicResistance", -10)],
    },
    CombatStyle {
        id: "defensive",
        name: "防御型",
        emoji: "🛡️",
        description: "强化防御能力，降低攻击换取生存力",
        bonuses: &[
            ("Defense", 20),
            ("MagicResistance", 20),
            ("AbsorbHP", 5),
            ("AD", -8),
            ("AP", -8),
        ],
    },
    CombatStyle {
        id: "balanced",
        name: "均衡型",
        emoji: "⚖️",
        description: "攻守兼备，全面提升各项属性",
        bonuses: &[
            ("AD", 5),
            ("AP", 5),
            ("Defense", 5),
            ("MagicResistance", 5),
            ("Hit", 3),
            ("Dodge", 3),
        ],
    },
    CombatStyle {
        id: "critical",
        name: "暴击型",
        emoji: "💥",
        description: "专注暴击和命中，追求一击必杀",
        bonuses: &[
            ("Crit", 20),
            ("Hit", 15),
            ("AD", 5),
            ("Defense", -5),
            ("MagicResistance", -5),
        ],
    },
];

/// 属性名称中文映射
fn attr_cn(attr: &str) -> &'static str {
    match attr {
        "HP" => "生命",
        "MP" => "魔法",
        "AD" => "物攻",
        "AP" => "魔攻",
        "Defense" => "防御",
        "MagicResistance" => "魔抗",
        "Hit" => "命中",
        "Dodge" => "闪避",
        "Crit" => "暴击",
        "AbsorbHP" => "吸血",
        "ImmuneDamage" => "免伤",
        "ADPTV" => "物穿",
        "APPTV" => "魔穿",
        _ => "未知",
    }
}

/// 获取玩家当前战斗风格
pub fn get_combat_style(db: &Database, user_id: &str) -> String {
    db.global_get("combat_style", user_id)
}

/// 获取风格的属性加成列表（供combat.rs调用）
pub fn get_style_bonuses(db: &Database, user_id: &str) -> Vec<(&'static str, i32)> {
    let style_id = get_combat_style(db, user_id);
    if style_id.is_empty() {
        return Vec::new(); // 未设置风格，无加成
    }
    for style in STYLES {
        if style.id == style_id {
            return style.bonuses.to_vec();
        }
    }
    Vec::new()
}

/// 查看战斗风格
pub fn cmd_view_combat_style(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current = get_combat_style(db, user_id);
    let args = args.trim();

    // 查看特定风格详情
    if !args.is_empty() {
        for style in STYLES {
            if style.name.contains(args) || style.id == args {
                let mut r = format!("{}\n═══ {} {} ═══", prefix, style.emoji, style.name);
                r.push_str(&format!("\n{}", style.description));
                r.push_str("\n\n📊 属性加成:");
                for &(attr, pct) in style.bonuses {
                    let sign = if pct > 0 { "+" } else { "" };
                    r.push_str(&format!("\n  {} {}{}%", attr_cn(attr), sign, pct));
                }
                let is_current = current == style.id;
                if is_current {
                    r.push_str("\n\n✅ 当前使用的战斗风格");
                } else {
                    r.push_str(&format!("\n\n发送 '切换战斗风格+{}' 切换到此风格", style.name));
                }
                return r;
            }
        }
        return format!("{}\n❌ 未找到战斗风格 [{}]", prefix, args);
    }

    // 列出所有风格
    let mut r = format!("{}\n═══ ⚔️ 战斗风格系统 ═══", prefix);
    r.push_str("\n选择适合你的战斗风格，获得不同的属性加成\n");

    for style in STYLES {
        let marker = if current == style.id { " ✅当前" } else { "" };
        r.push_str(&format!(
            "\n{} {}{} — {}",
            style.emoji, style.name, marker, style.description
        ));
        // 显示主要加成
        let main_bonuses: Vec<String> = style
            .bonuses
            .iter()
            .filter(|&&(_, pct)| pct > 0)
            .map(|&(attr, pct)| format!("{}+{}%", attr_cn(attr), pct))
            .collect();
        r.push_str(&format!("\n   主要加成: {}", main_bonuses.join(" / ")));
    }

    if current.is_empty() {
        r.push_str("\n\n⚠️ 你尚未选择战斗风格！发送 '切换战斗风格+风格名' 选择");
    }
    r.push_str("\n\n💡 发送 '查看战斗风格+风格名' 查看详情");
    r.push_str("\n💡 发送 '切换战斗风格+风格名' 切换风格（免费）");
    r
}

/// 切换战斗风格
pub fn cmd_switch_combat_style(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    if args.is_empty() {
        let mut r = format!("{}\n❓ 请指定要切换的战斗风格:", prefix);
        for style in STYLES {
            r.push_str(&format!("\n  {} {}", style.emoji, style.name));
        }
        r.push_str("\n\n用法: 切换战斗风格+风格名");
        return r;
    }

    // 查找目标风格
    let target = STYLES.iter().find(|s| s.name.contains(args) || s.id == args);

    let target = match target {
        Some(s) => s,
        None => {
            let mut r = format!("{}\n❌ 未找到战斗风格 [{}]", prefix, args);
            r.push_str("\n可用风格: ");
            for style in STYLES {
                r.push_str(&format!("{} ", style.name));
            }
            return r;
        }
    };

    let current = get_combat_style(db, user_id);
    if current == target.id {
        return format!(
            "{}\n{} 你已经是 {} 了，无需重复切换。",
            prefix, target.emoji, target.name
        );
    }

    // 检查玩家存活
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n💀 阵亡状态无法切换战斗风格，请先恢复生命。", prefix);
    }

    // 切换风格
    db.global_set("combat_style", user_id, target.id);

    let mut r = format!("{}\n{} 战斗风格已切换为: {}", prefix, target.emoji, target.name);
    r.push_str(&format!("\n{}", target.description));
    r.push_str("\n\n📊 新属性加成:");
    for &(attr, pct) in target.bonuses {
        let sign = if pct > 0 { "+" } else { "" };
        r.push_str(&format!("\n  {} {}{}%", attr_cn(attr), sign, pct));
    }
    r.push_str("\n\n💡 风格加成将在下次战斗时生效");
    r
}

/// 战斗风格详情（当前风格的详细信息）
pub fn cmd_combat_style_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current = get_combat_style(db, user_id);

    if current.is_empty() {
        let mut r = format!("{}\n═══ ⚔️ 战斗风格详情 ═══", prefix);
        r.push_str("\n\n⚠️ 你尚未选择战斗风格！");
        r.push_str("\n\n📋 可选风格:");
        for style in STYLES {
            r.push_str(&format!("\n  {} {} — {}", style.emoji, style.name, style.description));
        }
        r.push_str("\n\n💡 发送 '切换战斗风格+风格名' 选择你的战斗风格");
        return r;
    }

    // 找到当前风格
    let style = STYLES.iter().find(|s| s.id == current).unwrap();

    let mut r = format!("{}\n═══ {} {} 详情 ═══", prefix, style.emoji, style.name);
    r.push_str(&format!("\n📝 {}", style.description));

    r.push_str("\n\n📊 属性加成明细:");
    for &(attr, pct) in style.bonuses {
        let sign = if pct > 0 { "+" } else { "" };
        let indicator = if pct > 0 { "📈" } else { "📉" };
        r.push_str(&format!("\n  {} {}: {}%{}", indicator, attr_cn(attr), sign, pct));
    }

    // 显示其他可选风格
    r.push_str("\n\n🔄 其他可选风格:");
    for s in STYLES {
        if s.id != current {
            r.push_str(&format!("\n  {} {} — {}", s.emoji, s.name, s.description));
        }
    }
    r.push_str("\n\n💡 发送 '切换战斗风格+风格名' 更换风格（免费）");
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_definitions() {
        assert_eq!(STYLES.len(), 4);
        let ids: Vec<&str> = STYLES.iter().map(|s| s.id).collect();
        assert!(ids.contains(&"aggressive"));
        assert!(ids.contains(&"defensive"));
        assert!(ids.contains(&"balanced"));
        assert!(ids.contains(&"critical"));
    }

    #[test]
    fn test_style_bonuses_sum() {
        // Each style should have at least one positive and one negative or all positive
        for style in STYLES {
            let has_positive = style.bonuses.iter().any(|&(_, v)| v > 0);
            assert!(
                has_positive,
                "Style {} should have at least one positive bonus",
                style.name
            );
        }
    }

    #[test]
    fn test_attr_cn_mapping() {
        assert_eq!(attr_cn("AD"), "物攻");
        assert_eq!(attr_cn("Defense"), "防御");
        assert_eq!(attr_cn("Crit"), "暴击");
        assert_eq!(attr_cn("Unknown"), "未知");
    }

    #[test]
    fn test_aggressive_tradeoff() {
        // Aggressive should boost AD/AP but reduce Defense/MR
        let aggressive = &STYLES[0];
        assert_eq!(aggressive.id, "aggressive");
        let ad_bonus = aggressive.bonuses.iter().find(|&&(a, _)| a == "AD").unwrap().1;
        let def_bonus = aggressive.bonuses.iter().find(|&&(a, _)| a == "Defense").unwrap().1;
        assert!(ad_bonus > 0, "Aggressive should boost AD");
        assert!(def_bonus < 0, "Aggressive should reduce Defense");
    }

    #[test]
    fn test_balanced_all_positive() {
        // Balanced should have all positive bonuses
        let balanced = STYLES.iter().find(|s| s.id == "balanced").unwrap();
        for &(_, pct) in balanced.bonuses {
            assert!(pct > 0, "Balanced style should have all positive bonuses, got {}", pct);
        }
    }

    #[test]
    fn test_critical_focus() {
        // Critical should heavily boost Crit
        let critical = STYLES.iter().find(|s| s.id == "critical").unwrap();
        let crit_bonus = critical.bonuses.iter().find(|&&(a, _)| a == "Crit").unwrap().1;
        assert!(crit_bonus >= 15, "Critical style should have significant Crit bonus");
    }

    #[test]
    fn test_defensive_focus() {
        // Defensive should heavily boost Defense and MagicResistance
        let defensive = STYLES.iter().find(|s| s.id == "defensive").unwrap();
        let def_bonus = defensive.bonuses.iter().find(|&&(a, _)| a == "Defense").unwrap().1;
        let mr_bonus = defensive
            .bonuses
            .iter()
            .find(|&&(a, _)| a == "MagicResistance")
            .unwrap()
            .1;
        assert!(def_bonus >= 15, "Defensive style should have significant Defense bonus");
        assert!(mr_bonus >= 15, "Defensive style should have significant MR bonus");
    }
}

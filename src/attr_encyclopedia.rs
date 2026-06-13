/// 属性百科系统
/// 提供游戏内所有属性的详细说明、计算公式和获取方式
/// 数据来源: system_uAttributes 表 + 各系统属性定义
///
/// 指令: 属性百科, 属性详情+属性名
use crate::db::Database;
use crate::template::{render_cgd_section, render_template, TemplateContext};
use crate::user;

/// 属性百科条目
#[derive(Debug, Clone)]
pub struct AttrEntry {
    pub name: &'static str,        // 英文名
    pub cn_name: &'static str,     // 中文名
    pub category: &'static str,    // 分类
    pub description: &'static str, // 描述
    pub formula: &'static str,     // 计算公式
    pub sources: &'static str,     // 获取方式
    pub icon: &'static str,        // 图标
}

/// 所有属性百科数据
fn attr_entries() -> Vec<AttrEntry> {
    vec![
        AttrEntry {
            name: "HP",
            cn_name: "生命值",
            category: "基础属性",
            description: "角色的生命值上限，归零时角色阵亡。阵亡后需要等待复活或使用复活卷轴。",
            formula: "基础值 + 等级加成 + 装备加成 + 宝石加成 + 灵兽加成 + 增益效果",
            sources: "升级、装备穿戴、宝石镶嵌、灵兽出战、GM调整、VIP加成",
            icon: "❤️",
        },
        AttrEntry {
            name: "MP",
            cn_name: "魔法值",
            category: "基础属性",
            description: "角色的魔法值上限，释放技能时消耗。魔法值不足时无法使用技能。",
            formula: "基础值 + 等级加成 + 装备加成 + 职业加成",
            sources: "升级、装备穿戴、职业转职",
            icon: "💎",
        },
        AttrEntry {
            name: "AD",
            cn_name: "物理攻击",
            category: "攻击属性",
            description: "物理攻击伤害的基础值，影响普通攻击和物理技能伤害。",
            formula: "基础值 + 等级加成 + 装备加成 + 宝石加成 + 锻炼加成 + 增益效果",
            sources: "升级、装备穿戴、宝石镶嵌、吐纳修炼、GM调整",
            icon: "⚔️",
        },
        AttrEntry {
            name: "AP",
            cn_name: "魔法攻击",
            category: "攻击属性",
            description: "魔法攻击伤害的基础值，影响魔法技能伤害。法系职业的核心属性。",
            formula: "基础值 + 等级加成 + 装备加成 + 冥想加成 + 增益效果",
            sources: "升级、装备穿戴、冥想修炼、GM调整",
            icon: "🔮",
        },
        AttrEntry {
            name: "Defense",
            cn_name: "物理防御",
            category: "防御属性",
            description: "减少受到的物理伤害。防御越高，受到的物理伤害越低。",
            formula: "基础值 + 等级加成 + 装备加成 + 宝石加成 + 练武加成 + 增益效果",
            sources: "升级、装备穿戴、宝石镶嵌、练武修炼、GM调整",
            icon: "🛡️",
        },
        AttrEntry {
            name: "MagicResistance",
            cn_name: "魔法抗性",
            category: "防御属性",
            description: "减少受到的魔法伤害。魔抗越高，受到的魔法伤害越低。",
            formula: "基础值 + 等级加成 + 装备加成 + 习法加成 + 增益效果",
            sources: "升级、装备穿戴、习法修炼、GM调整",
            icon: "🔰",
        },
        AttrEntry {
            name: "Hit",
            cn_name: "命中",
            category: "战斗属性",
            description: "影响攻击命中率。命中越高，攻击越不容易被闪避。攻速也由此值影响。",
            formula: "基础值 + 装备加成 + 增益效果",
            sources: "装备穿戴、增益效果",
            icon: "🎯",
        },
        AttrEntry {
            name: "Dodge",
            cn_name: "闪避",
            category: "战斗属性",
            description: "影响闪避攻击的概率。闪避越高，越容易避开敌人的攻击。",
            formula: "基础值 + 装备加成",
            sources: "装备穿戴",
            icon: "💨",
        },
        AttrEntry {
            name: "Crit",
            cn_name: "暴击率",
            category: "战斗属性",
            description: "触发暴击的概率。暴击时造成额外伤害。暴击率以百分比显示。",
            formula: "基础值 + 装备加成 + 增益效果",
            sources: "装备穿戴、增益效果",
            icon: "💥",
        },
        AttrEntry {
            name: "AbsorbHP",
            cn_name: "生命偷取",
            category: "战斗属性",
            description: "攻击时按比例回复自身生命值。以百分比计算。",
            formula: "装备加成 + 增益效果",
            sources: "装备穿戴、增益效果、灵兽加成",
            icon: "🧛",
        },
        AttrEntry {
            name: "ImmuneDamage",
            cn_name: "伤害免疫",
            category: "防御属性",
            description: "按比例减少受到的所有伤害。以百分比计算，非常稀有的属性。",
            formula: "装备加成 + 增益效果",
            sources: "高级装备穿戴、增益效果",
            icon: "✨",
        },
        AttrEntry {
            name: "ADPTV",
            cn_name: "物理穿透值",
            category: "攻击属性",
            description: "无视对方物理防御的固定值。直接减少对方的防御力。",
            formula: "装备加成 + 增益效果",
            sources: "装备穿戴、GM调整",
            icon: "🗡️",
        },
        AttrEntry {
            name: "APPTV",
            cn_name: "魔法穿透值",
            category: "攻击属性",
            description: "无视对方法术抗性的固定值。直接减少对方的魔抗。",
            formula: "装备加成 + 增益效果",
            sources: "装备穿戴、GM调整",
            icon: "🌊",
        },
        AttrEntry {
            name: "CombatPower",
            cn_name: "战力评分",
            category: "综合属性",
            description: "综合评估角色战斗力的数值。基于所有属性加权计算得出。",
            formula: "(物攻×35 + 魔攻×217.5 + 生命×2.7 + 魔法×1.4 + 防御×20 + 魔抗×18 + 命中×2.25 + 闪避×12 + 暴击×60 + 物穿×20 + 法穿×44.44) ÷ 10",
            sources: "所有属性提升均影响战力",
            icon: "⚡",
        },
    ]
}

/// 属性分类列表
fn categories() -> Vec<(&'static str, &'static str)> {
    vec![
        ("基础属性", "🏗️"),
        ("攻击属性", "⚔️"),
        ("防御属性", "🛡️"),
        ("战斗属性", "🎯"),
        ("综合属性", "⚡"),
    ]
}

/// 属性百科 — 显示所有属性概览
pub fn cmd_attr_encyclopedia(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let entries = attr_entries();

    // 如果指定了属性名，显示详情
    let arg = args.trim();
    if !arg.is_empty() {
        return cmd_attr_detail_inner(db, user_id, arg, &prefix);
    }

    let mut lines: Vec<String> = Vec::new();
    lines.push("📖 ══════ 属性百科 ══════".to_string());
    lines.push(String::new());

    for (cat_name, icon) in categories() {
        let cat_entries: Vec<&AttrEntry> = entries.iter().filter(|e| e.category == cat_name).collect();
        if cat_entries.is_empty() {
            continue;
        }
        lines.push(format!("{}【{}】", icon, cat_name));
        for entry in &cat_entries {
            lines.push(format!(
                "  {} {} ({}) — {}",
                entry.icon,
                entry.cn_name,
                entry.name,
                &entry.description[..20.min(entry.description.len())]
            ));
        }
        lines.push(String::new());
    }

    // 显示玩家当前属性值（如果有注册）
    if db.user_exists(user_id) {
        let info = user::calc_total_attrs(db, user_id);
        lines.push("📊 你的当前属性:".to_string());
        lines.push(format!(
            "  ❤️ 生命: {}/{}  💎 魔法: {}/{}",
            info.hp, info.hp_max, info.mp, info.mp_max
        ));
        lines.push(format!(
            "  ⚔️ 物攻: {}  🔮 魔攻: {}  🛡️ 防御: {}  🔰 魔抗: {}",
            info.ad, info.ap, info.defense, info.magic_res
        ));
        lines.push(format!(
            "  🎯 命中: {}  💨 闪避: {}  💥 暴击: {}  🧛 吸血: {}",
            info.hit, info.dodge, info.crit, info.absorb_hp
        ));
        lines.push(String::new());
    }

    lines.push("💡 发送「属性详情+属性名」查看详细说明".to_string());
    lines.push("  例: 属性详情+物攻  属性详情+HP  属性详情+暴击".to_string());

    // 尝试使用模板渲染
    let mut ctx = TemplateContext::new();
    ctx.set_many(&[
        ("属性总数", &entries.len().to_string()),
        ("分类总数", &categories().len().to_string()),
    ]);
    if let Some(rendered) = render_template(db, "属性百科", "概览", &ctx) {
        return rendered;
    }

    format!("{}\n{}", prefix, lines.join("\n"))
}

/// 属性详情 — 显示单个属性的详细信息
fn cmd_attr_detail_inner(db: &Database, user_id: &str, attr_name: &str, prefix: &str) -> String {
    let entries = attr_entries();

    // 查找匹配的属性（支持中文名或英文名）
    let entry = entries.iter().find(|e| {
        e.cn_name == attr_name
            || e.name.eq_ignore_ascii_case(attr_name)
            || e.name.to_lowercase() == attr_name.to_lowercase()
    });

    let entry = match entry {
        Some(e) => e,
        None => {
            let available: Vec<&str> = entries.iter().map(|e| e.cn_name).collect();
            return format!(
                "{}\n❌ 未找到属性「{}」\n\n可用属性: {}",
                prefix,
                attr_name,
                available.join("、")
            );
        }
    };

    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("{} ════ {} ({}) ════", entry.icon, entry.cn_name, entry.name));
    lines.push(String::new());
    lines.push(format!("📂 分类: {}", entry.category));
    lines.push(format!("📝 说明: {}", entry.description));
    lines.push(format!("📐 公式: {}", entry.formula));
    lines.push(format!("📍 来源: {}", entry.sources));

    // 显示玩家当前值（如果有注册）
    if db.user_exists(user_id) {
        let info = user::calc_total_attrs(db, user_id);
        let current_val = match entry.name {
            "HP" => format!("{}/{}", info.hp, info.hp_max),
            "MP" => format!("{}/{}", info.mp, info.mp_max),
            "AD" => info.ad.to_string(),
            "AP" => info.ap.to_string(),
            "Defense" => info.defense.to_string(),
            "MagicResistance" => info.magic_res.to_string(),
            "Hit" => info.hit.to_string(),
            "Dodge" => info.dodge.to_string(),
            "Crit" => format!("{}%", info.crit),
            "AbsorbHP" => format!("{}%", info.absorb_hp),
            "ImmuneDamage" => format!("{}%", info.immune),
            "ADPTV" => format!("{}", info.ad_ptv),
            "APPTV" => format!("{}", info.ap_ptv),
            "CombatPower" => {
                let cp = crate::combat_power::calc_combat_power(&info);
                format!("{:.1}", cp)
            }
            _ => "—".to_string(),
        };
        lines.push(String::new());
        lines.push(format!("📊 你的当前{}: {}", entry.cn_name, current_val));

        // 显示相关增益效果
        let buffs = crate::buff::get_active_buffs(db, user_id);
        let related_buffs: Vec<&crate::buff::BuffEntry> = buffs
            .iter()
            .filter(|b| {
                b.is_active && (b.attr_name == entry.name || b.attr_name.to_lowercase() == entry.name.to_lowercase())
            })
            .collect();
        if !related_buffs.is_empty() {
            lines.push(format!("🔮 活跃增益: {}个效果", related_buffs.len()));
            for buff in &related_buffs {
                let sign = if buff.value >= 0 { "+" } else { "" };
                lines.push(format!("  · {}{} (到期: {})", sign, buff.value, buff.expire_time));
            }
        }
    }

    lines.push(String::new());
    lines.push("💡 发送「属性百科」查看所有属性列表".to_string());

    // 尝试使用模板渲染
    let mut ctx = TemplateContext::new();
    ctx.set("属性名", entry.cn_name);
    ctx.set("属性英文名", entry.name);
    ctx.set("属性描述", entry.description);
    ctx.set("计算公式", entry.formula);
    ctx.set("获取方式", entry.sources);
    ctx.set("分类", entry.category);
    if let Some(rendered) = render_template(db, "属性百科", "详情", &ctx) {
        return rendered;
    }
    // 备用: 尝试 CGD 原始模板渲染
    let raw_tmpl = db.template_get("属性百科");
    if !raw_tmpl.is_empty() {
        if let Some(rendered) = render_cgd_section(&raw_tmpl, "详情", &ctx) {
            return rendered;
        }
    }

    format!("{}\n{}", prefix, lines.join("\n"))
}

/// 搜索属性 — 按关键词搜索
#[allow(dead_code)]
pub fn search_attrs(keyword: &str) -> Vec<AttrEntry> {
    let entries = attr_entries();
    let kw = keyword.to_lowercase();
    entries
        .into_iter()
        .filter(|e| {
            e.cn_name.contains(keyword)
                || e.name.to_lowercase().contains(&kw)
                || e.description.contains(keyword)
                || e.category.contains(keyword)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attr_entries_count() {
        let entries = attr_entries();
        // 应该有14个属性条目
        assert!(
            entries.len() >= 13,
            "Expected at least 13 attr entries, got {}",
            entries.len()
        );
    }

    #[test]
    fn test_all_attrs_have_required_fields() {
        let entries = attr_entries();
        for entry in &entries {
            assert!(!entry.name.is_empty(), "Empty name");
            assert!(!entry.cn_name.is_empty(), "Empty cn_name for {}", entry.name);
            assert!(!entry.description.is_empty(), "Empty description for {}", entry.name);
            assert!(!entry.formula.is_empty(), "Empty formula for {}", entry.name);
            assert!(!entry.sources.is_empty(), "Empty sources for {}", entry.name);
        }
    }

    #[test]
    fn test_search_by_cn_name() {
        let results = search_attrs("生命");
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.cn_name.contains("生命")));
    }

    #[test]
    fn test_search_by_en_name() {
        let results = search_attrs("HP");
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.name == "HP"));
    }

    #[test]
    fn test_search_by_category() {
        let results = search_attrs("攻击");
        assert!(!results.is_empty());
        assert!(results.iter().any(|e| e.category.contains("攻击")));
    }

    #[test]
    fn test_search_no_match() {
        let results = search_attrs("不存在的属性");
        assert!(results.is_empty());
    }

    #[test]
    fn test_categories_complete() {
        let cats = categories();
        let entries = attr_entries();
        for entry in &entries {
            assert!(
                cats.iter().any(|(c, _)| *c == entry.category),
                "Category '{}' not in categories list (from attr {})",
                entry.category,
                entry.name
            );
        }
    }

    #[test]
    fn test_combat_power_formula_present() {
        let entries = attr_entries();
        let cp = entries.iter().find(|e| e.name == "CombatPower");
        assert!(cp.is_some());
        assert!(cp.unwrap().formula.contains("物攻"));
    }
}

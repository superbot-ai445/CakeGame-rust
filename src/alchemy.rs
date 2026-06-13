//! 炼制系统
//!
//! 四类炼制: 材料炼制 / 合成炼制 / 元素炼制 / 装备炼制
//! 来源: ext_lianzhicl_info, ext_lianzhihc_info, ext_lianzhiys_info, ext_lianzhizb_info (均空表, 使用内置配方)
//! 指令: 查看炼制, 炼制, 可炼制

use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;

/// 炼制配方
#[allow(dead_code)]
struct AlchemyRecipe {
    /// 配方名称 (用于显示和匹配)
    name: &'static str,
    /// 炼制类别
    category: &'static str,
    /// 输入材料列表: (物品名, 数量)
    inputs: &'static [(&'static str, i32)],
    /// 输出物品名
    output: &'static str,
    /// 输出数量
    output_qty: i32,
    /// 消耗金币
    cost_gold: i64,
    /// 最低等级要求
    min_level: i32,
    /// 配方描述
    desc: &'static str,
}

/// 内置炼制配方 — 材料精炼 + 高级合成 + 元素转化 + 装备锻造
const ALCHEMY_RECIPES: &[AlchemyRecipe] = &[
    // ==================== 材料炼制 ====================
    // 将基础材料精炼为高级材料
    AlchemyRecipe {
        name: "精炼强化石",
        category: "材料炼制",
        inputs: &[("强化石", 3)],
        output: "强化石",
        output_qty: 5,
        cost_gold: 200,
        min_level: 5,
        desc: "3个强化石精炼为5个，提升纯度",
    },
    AlchemyRecipe {
        name: "精炼白色精粹",
        category: "材料炼制",
        inputs: &[("白色精粹", 2)],
        output: "白色精粹",
        output_qty: 4,
        cost_gold: 150,
        min_level: 3,
        desc: "白色精粹结晶提纯",
    },
    AlchemyRecipe {
        name: "精炼红色精粹",
        category: "材料炼制",
        inputs: &[("红色精粹", 2)],
        output: "红色精粹",
        output_qty: 4,
        cost_gold: 150,
        min_level: 3,
        desc: "红色精粹结晶提纯",
    },
    AlchemyRecipe {
        name: "精炼蓝色精粹",
        category: "材料炼制",
        inputs: &[("蓝色精粹", 2)],
        output: "蓝色精粹",
        output_qty: 4,
        cost_gold: 150,
        min_level: 3,
        desc: "蓝色精粹结晶提纯",
    },
    AlchemyRecipe {
        name: "精炼绿色精粹",
        category: "材料炼制",
        inputs: &[("绿色精粹", 2)],
        output: "绿色精粹",
        output_qty: 4,
        cost_gold: 150,
        min_level: 3,
        desc: "绿色精粹结晶提纯",
    },
    AlchemyRecipe {
        name: "精炼紫色精粹",
        category: "材料炼制",
        inputs: &[("紫色精粹", 2)],
        output: "紫色精粹",
        output_qty: 4,
        cost_gold: 150,
        min_level: 3,
        desc: "紫色精粹结晶提纯",
    },
    // ==================== 合成炼制 ====================
    // 多种材料合成高级材料
    AlchemyRecipe {
        name: "合成虚空宝石",
        category: "合成炼制",
        inputs: &[("强化石", 5), ("白色精粹", 3), ("红色精粹", 3)],
        output: "虚空宝石",
        output_qty: 1,
        cost_gold: 800,
        min_level: 10,
        desc: "多色精粹融合为虚空宝石",
    },
    AlchemyRecipe {
        name: "合成远古超界石",
        category: "合成炼制",
        inputs: &[("虚空宝石", 2), ("紫色精粹", 5), ("强化石", 10)],
        output: "远古超界石",
        output_qty: 1,
        cost_gold: 2000,
        min_level: 15,
        desc: "虚空宝石与精粹共鸣产生远古之力",
    },
    AlchemyRecipe {
        name: "合成怨念之息",
        category: "合成炼制",
        inputs: &[("紫色精粹", 3), ("红色精粹", 3), ("蓝色精粹", 3)],
        output: "怨念之息",
        output_qty: 1,
        cost_gold: 500,
        min_level: 8,
        desc: "三色精粹凝聚的神秘气息",
    },
    AlchemyRecipe {
        name: "合成史诗碎片",
        category: "合成炼制",
        inputs: &[("怨念之息", 2), ("强化石", 8), ("绿色精粹", 4)],
        output: "史诗碎片",
        output_qty: 1,
        cost_gold: 1500,
        min_level: 12,
        desc: "史诗装备的核心材料",
    },
    // ==================== 元素炼制 ====================
    // 药剂 + 精粹转化
    AlchemyRecipe {
        name: "炼制大生命药水",
        category: "元素炼制",
        inputs: &[("【普通】生命药水", 3), ("红色精粹", 1)],
        output: "【普通】大生命药水",
        output_qty: 1,
        cost_gold: 100,
        min_level: 5,
        desc: "生命药水与红色精粹融合",
    },
    AlchemyRecipe {
        name: "炼制大魔力药水",
        category: "元素炼制",
        inputs: &[("【普通】魔力药水", 3), ("蓝色精粹", 1)],
        output: "【普通】大魔力药水",
        output_qty: 1,
        cost_gold: 100,
        min_level: 5,
        desc: "魔力药水与蓝色精粹融合",
    },
    AlchemyRecipe {
        name: "炼制超生命药水",
        category: "元素炼制",
        inputs: &[("【普通】大生命药水", 3), ("紫色精粹", 2)],
        output: "【稀有】超生命药水",
        output_qty: 1,
        cost_gold: 300,
        min_level: 10,
        desc: "大生命药水与紫色精粹的高级融合",
    },
    AlchemyRecipe {
        name: "炼制超魔力药水",
        category: "元素炼制",
        inputs: &[("【普通】大魔力药水", 3), ("紫色精粹", 2)],
        output: "【稀有】超魔力药水",
        output_qty: 1,
        cost_gold: 300,
        min_level: 10,
        desc: "大魔力药水与紫色精粹的高级融合",
    },
    AlchemyRecipe {
        name: "炼制护肝药剂",
        category: "元素炼制",
        inputs: &[("【普通】红草药", 2), ("【普通】蓝草药", 2), ("绿色精粹", 1)],
        output: "护肝药剂",
        output_qty: 1,
        cost_gold: 200,
        min_level: 7,
        desc: "双色草药与绿色精粹调和",
    },
    AlchemyRecipe {
        name: "炼制兴奋剂",
        category: "元素炼制",
        inputs: &[("【普通】紫草药", 3), ("红色精粹", 2), ("绿色精粹", 1)],
        output: "兴奋剂",
        output_qty: 1,
        cost_gold: 250,
        min_level: 8,
        desc: "紫草药精华为战斗注入爆发力",
    },
    // ==================== 装备炼制 ====================
    // 材料合成装备
    AlchemyRecipe {
        name: "锻造古树之杖",
        category: "装备炼制",
        inputs: &[("强化石", 10), ("绿色精粹", 5), ("虚空宝石", 1)],
        output: "【卓越】古树之杖",
        output_qty: 1,
        cost_gold: 3000,
        min_level: 15,
        desc: "古树之力凝聚的传说法杖",
    },
    AlchemyRecipe {
        name: "锻造密制虎骨胸甲",
        category: "装备炼制",
        inputs: &[("强化石", 12), ("红色精粹", 5), ("虚空宝石", 1)],
        output: "【卓越】密制虎骨胸甲",
        output_qty: 1,
        cost_gold: 3500,
        min_level: 15,
        desc: "虎骨精华为勇者打造的胸甲",
    },
    AlchemyRecipe {
        name: "锻造密制虎骨战靴",
        category: "装备炼制",
        inputs: &[("强化石", 8), ("蓝色精粹", 4), ("怨念之息", 1)],
        output: "【卓越】密制虎骨战靴",
        output_qty: 1,
        cost_gold: 2500,
        min_level: 12,
        desc: "虎骨精华打造的轻便战靴",
    },
    AlchemyRecipe {
        name: "锻造密制虎骨护肩",
        category: "装备炼制",
        inputs: &[("强化石", 8), ("白色精粹", 4), ("怨念之息", 1)],
        output: "【卓越】密制虎骨护肩",
        output_qty: 1,
        cost_gold: 2500,
        min_level: 12,
        desc: "虎骨精华打造的护肩",
    },
    AlchemyRecipe {
        name: "锻造密制虎骨绑腿",
        category: "装备炼制",
        inputs: &[("强化石", 8), ("紫色精粹", 4), ("怨念之息", 1)],
        output: "【卓越】密制虎骨绑腿",
        output_qty: 1,
        cost_gold: 2500,
        min_level: 12,
        desc: "虎骨精华打造的绑腿",
    },
    AlchemyRecipe {
        name: "锻造密制虎骨腰带",
        category: "装备炼制",
        inputs: &[("强化石", 8), ("绿色精粹", 4), ("怨念之息", 1)],
        output: "【卓越】密制虎骨腰带",
        output_qty: 1,
        cost_gold: 2500,
        min_level: 12,
        desc: "虎骨精华打造的腰带",
    },
];

/// 查找配方 (精确名称 或 模糊匹配)
fn find_recipe(name: &str) -> Option<&'static AlchemyRecipe> {
    // 精确匹配配方名
    if let Some(r) = ALCHEMY_RECIPES.iter().find(|r| r.name == name) {
        return Some(r);
    }
    // 精确匹配输出物品
    if let Some(r) = ALCHEMY_RECIPES.iter().find(|r| r.output == name) {
        return Some(r);
    }
    // 模糊匹配配方名
    ALCHEMY_RECIPES
        .iter()
        .find(|r| r.name.contains(name) || name.contains(r.name))
        .or_else(|| {
            ALCHEMY_RECIPES
                .iter()
                .find(|r| r.output.contains(name) || name.contains(r.output))
        })
}

/// 查看炼制 — 显示所有炼制配方
pub fn cmd_view_alchemy(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看炼制配方！", prefix);
    }

    let category_filter = args.trim();
    let mut result = String::from(
        "╔══════════════════════╗\n\
         ║    ⚗️ 炼制系统 ⚗️    ║\n\
         ╚══════════════════════╝\n\n\
         将材料转化为更强大的物品！\n\n",
    );

    let categories = ["材料炼制", "合成炼制", "元素炼制", "装备炼制"];
    let icons = ["🔨", "⚗️", "✨", "⚔️"];

    for (cat, icon) in categories.iter().zip(icons.iter()) {
        // 如果有分类筛选且不匹配则跳过
        if !category_filter.is_empty() && !cat.contains(category_filter) {
            continue;
        }

        let recipes: Vec<&AlchemyRecipe> = ALCHEMY_RECIPES.iter().filter(|r| r.category == *cat).collect();

        if recipes.is_empty() {
            continue;
        }

        result.push_str(&format!("━━━ {} {} ━━━\n", icon, cat));
        for r in &recipes {
            let inputs_str: Vec<String> = r.inputs.iter().map(|(name, qty)| format!("{}×{}", name, qty)).collect();
            result.push_str(&format!(
                "  「{}」 {} → {}×{}\n\
                 \t需要: {} + {}金币 | Lv{}\n",
                r.name,
                inputs_str.join(" + "),
                r.output,
                r.output_qty,
                inputs_str.join(" + "),
                r.cost_gold,
                r.min_level
            ));
        }
        result.push('\n');
    }

    if category_filter.is_empty() {
        result.push_str(&format!(
            "💡 共 {} 个配方\n\
             📌 使用「炼制+配方名」进行炼制\n\
             📌 使用「可炼制」查看背包中可炼制的配方\n\
             📌 使用「查看炼制+类别名」筛选分类\n\
             \t类别: 材料炼制 / 合成炼制 / 元素炼制 / 装备炼制",
            ALCHEMY_RECIPES.len()
        ));
    }

    result
}

/// 可炼制 — 查看背包中可炼制的配方
pub fn cmd_alchemiable(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let backpack = db.knapsack_all(user_id);
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let gold = db.read_currency(user_id, CURRENCY_GOLD);

    let mut available: Vec<(&AlchemyRecipe, bool)> = Vec::new();
    for recipe in ALCHEMY_RECIPES {
        // 检查所有材料是否满足
        let mut can_craft = true;
        for &(input_name, need_qty) in recipe.inputs {
            let has_qty = backpack
                .iter()
                .filter(|item| item.name == input_name)
                .map(|item| item.quantity)
                .sum::<i32>();
            if has_qty < need_qty {
                can_craft = false;
                break;
            }
        }
        let level_ok = user_level >= recipe.min_level;
        let gold_ok = gold >= recipe.cost_gold;
        let fully_ready = can_craft && level_ok && gold_ok;
        available.push((recipe, fully_ready));
    }

    if available.is_empty() {
        return format!(
            "{}\n当前没有可匹配的炼制配方。\n💡 使用「查看炼制」查看所有配方。",
            prefix
        );
    }

    let mut result = String::from(
        "╔══════════════════════╗\n\
         ║  ⚗️ 可炼制配方 ⚗️   ║\n\
         ╚══════════════════════╝\n\n",
    );

    let ready: Vec<&(&AlchemyRecipe, bool)> = available.iter().filter(|(_, r)| *r).collect();
    let partial: Vec<&(&AlchemyRecipe, bool)> = available.iter().filter(|(_, r)| !*r).collect();

    if !ready.is_empty() {
        result.push_str("✅ 可立即炼制:\n");
        for (i, (recipe, _)) in ready.iter().enumerate() {
            let inputs_str: Vec<String> = recipe
                .inputs
                .iter()
                .map(|(name, qty)| {
                    let has = backpack
                        .iter()
                        .filter(|item| item.name == *name)
                        .map(|item| item.quantity)
                        .sum::<i32>();
                    format!("{}({}/{})", name, has, qty)
                })
                .collect();
            result.push_str(&format!(
                "  {}. 「{}」 {} → {}×{}\n",
                i + 1,
                recipe.name,
                inputs_str.join(" + "),
                recipe.output,
                recipe.output_qty
            ));
        }
        result.push('\n');
    }

    if !partial.is_empty() {
        result.push_str("⏳ 材料不足但可匹配:\n");
        for (i, (recipe, _)) in partial.iter().enumerate() {
            let inputs_detail: Vec<String> = recipe
                .inputs
                .iter()
                .map(|(name, qty)| {
                    let has = backpack
                        .iter()
                        .filter(|item| item.name == *name)
                        .map(|item| item.quantity)
                        .sum::<i32>();
                    if has >= *qty {
                        format!("{}({}/{})", name, has, qty)
                    } else {
                        format!("{}({}/{})❌", name, has, qty)
                    }
                })
                .collect();
            result.push_str(&format!(
                "  {}. 「{}」 {} → {}×{} | Lv{} | {}金\n",
                i + 1,
                recipe.name,
                inputs_detail.join(" + "),
                recipe.output,
                recipe.output_qty,
                recipe.min_level,
                recipe.cost_gold
            ));
        }
        result.push('\n');
    }

    result.push_str(&format!("💰 当前金币：{} | Lv{}", gold, user_level));

    result
}

/// 炼制 — 执行炼制
pub fn cmd_alchemy(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再进行炼制！", prefix);
    }

    let recipe_name = args.trim();
    if recipe_name.is_empty() {
        return format!(
            "{}\n请指定要炼制的配方。\n用法：炼制+配方名\n💡 使用「查看炼制」查看所有配方\n💡 使用「可炼制」查看可用配方",
            prefix
        );
    }

    // 查找配方
    let recipe = match find_recipe(recipe_name) {
        Some(r) => r,
        None => {
            // 尝试在配方名中模糊搜索
            let similar: Vec<&str> = ALCHEMY_RECIPES
                .iter()
                .filter(|r| r.name.contains(recipe_name) || recipe_name.contains(r.name))
                .map(|r| r.name)
                .collect();

            if similar.is_empty() {
                return format!(
                    "{}\n未找到「{}」的炼制配方。\n💡 使用「查看炼制」查看所有配方。",
                    prefix, recipe_name
                );
            } else {
                return format!(
                    "{}\n未找到精确匹配，你是否要找：\n{}",
                    prefix,
                    similar
                        .iter()
                        .map(|s| format!("  · {}", s))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        }
    };

    // 检查等级
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if user_level < recipe.min_level {
        return format!(
            "{}\n等级不足！炼制「{}」需要 Lv{}，当前 Lv{}。",
            prefix, recipe.name, recipe.min_level, user_level
        );
    }

    // 体力检查 (炼制消耗3体力)
    if let Err(e) = stamina::consume_stamina(user_id, "炼制", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < recipe.cost_gold {
        return format!(
            "{}\n金币不足！炼制「{}」需要{}金币，当前仅有{}金币。",
            prefix, recipe.name, recipe.cost_gold, gold
        );
    }

    // 检查所有材料
    let backpack = db.knapsack_all(user_id);
    let mut missing: Vec<String> = Vec::new();
    for &(input_name, need_qty) in recipe.inputs {
        let has_qty = backpack
            .iter()
            .filter(|item| item.name == input_name)
            .map(|item| item.quantity)
            .sum::<i32>();
        if has_qty < need_qty {
            missing.push(format!("  · {} ({}/{})", input_name, has_qty, need_qty));
        }
    }

    if !missing.is_empty() {
        return format!(
            "{}\n材料不足！缺少：\n{}\n💡 使用「可炼制」查看当前可炼制的配方。",
            prefix,
            missing.join("\n")
        );
    }

    // 执行炼制
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, recipe.cost_gold);
    for &(input_name, need_qty) in recipe.inputs {
        db.remove_item(user_id, input_name, need_qty);
    }
    db.add_item(user_id, recipe.output, recipe.output_qty);

    let new_gold = db.read_currency(user_id, CURRENCY_GOLD);
    let output_count = db.knapsack_quantity(user_id, recipe.output);

    let inputs_str: Vec<String> = recipe
        .inputs
        .iter()
        .map(|(name, qty)| format!("{}×{}", name, qty))
        .collect();

    format!(
        "{}\n⚗️ 炼制成功！\n\n\
         配方：「{}」\n\
         消耗：{} + {}金币\n\
         获得：[{}]×{}\n\n\
         💰 剩余金币：{}\n\
         📦 {}库存：{}",
        prefix,
        recipe.name,
        inputs_str.join(" + "),
        recipe.cost_gold,
        recipe.output,
        recipe.output_qty,
        new_gold,
        recipe.output,
        output_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alchemy_recipes_count() {
        assert_eq!(ALCHEMY_RECIPES.len(), 22);
    }

    #[test]
    fn test_recipe_names_unique() {
        let mut names: Vec<&str> = ALCHEMY_RECIPES.iter().map(|r| r.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len(), "Recipe names should be unique");
    }

    #[test]
    fn test_recipe_output_names_unique() {
        let mut outputs: Vec<&str> = ALCHEMY_RECIPES.iter().map(|r| r.output).collect();
        let before = outputs.len();
        outputs.sort();
        outputs.dedup();
        assert_eq!(before, outputs.len(), "Recipe outputs should be unique");
    }

    #[test]
    fn test_recipe_costs_positive() {
        for r in ALCHEMY_RECIPES {
            assert!(r.cost_gold > 0, "Recipe '{}' should have positive cost", r.name);
        }
    }

    #[test]
    fn test_recipe_output_qty_positive() {
        for r in ALCHEMY_RECIPES {
            assert!(r.output_qty > 0, "Recipe '{}' should have positive output qty", r.name);
        }
    }

    #[test]
    fn test_recipe_inputs_non_empty() {
        for r in ALCHEMY_RECIPES {
            assert!(!r.inputs.is_empty(), "Recipe '{}' should have at least 1 input", r.name);
            for &(name, qty) in r.inputs {
                assert!(!name.is_empty(), "Input name should not be empty in '{}'", r.name);
                assert!(qty > 0, "Input qty should be positive in '{}'", r.name);
            }
        }
    }

    #[test]
    fn test_recipe_min_level_positive() {
        for r in ALCHEMY_RECIPES {
            assert!(r.min_level > 0, "Recipe '{}' should have positive min_level", r.name);
        }
    }

    #[test]
    fn test_recipe_categories() {
        let valid = ["材料炼制", "合成炼制", "元素炼制", "装备炼制"];
        for r in ALCHEMY_RECIPES {
            assert!(
                valid.contains(&r.category),
                "Recipe '{}' has invalid category: {}",
                r.name,
                r.category
            );
        }
    }

    #[test]
    fn test_find_recipe_exact_name() {
        let r = find_recipe("精炼强化石");
        assert!(r.is_some());
        assert_eq!(r.unwrap().name, "精炼强化石");
    }

    #[test]
    fn test_find_recipe_exact_output() {
        let r = find_recipe("虚空宝石");
        assert!(r.is_some());
        assert_eq!(r.unwrap().output, "虚空宝石");
    }

    #[test]
    fn test_find_recipe_fuzzy() {
        let r = find_recipe("强化石");
        assert!(r.is_some());
    }

    #[test]
    fn test_find_recipe_not_found() {
        let r = find_recipe("不存在的配方");
        assert!(r.is_none());
    }

    #[test]
    fn test_find_recipe_empty() {
        // Note: empty string matches via fuzzy matching since "".contains("") is true
        let r = find_recipe("");
        // This is expected behavior - empty string matches first recipe
        assert!(r.is_some());
    }

    #[test]
    fn test_category_counts() {
        let material = ALCHEMY_RECIPES.iter().filter(|r| r.category == "材料炼制").count();
        let synth = ALCHEMY_RECIPES.iter().filter(|r| r.category == "合成炼制").count();
        let element = ALCHEMY_RECIPES.iter().filter(|r| r.category == "元素炼制").count();
        let equip = ALCHEMY_RECIPES.iter().filter(|r| r.category == "装备炼制").count();
        assert_eq!(material, 6);
        assert_eq!(synth, 4);
        assert_eq!(element, 6);
        assert_eq!(equip, 6);
        assert_eq!(material + synth + element + equip, ALCHEMY_RECIPES.len());
    }

    #[test]
    fn test_cost_increases_with_level() {
        // Within each category, higher level recipes should generally cost more
        let equip_recipes: Vec<&AlchemyRecipe> = ALCHEMY_RECIPES.iter().filter(|r| r.category == "装备炼制").collect();
        // At least verify level 15 recipes cost more than level 12
        let l15_min = equip_recipes
            .iter()
            .filter(|r| r.min_level == 15)
            .map(|r| r.cost_gold)
            .min()
            .unwrap_or(0);
        let l12_max = equip_recipes
            .iter()
            .filter(|r| r.min_level == 12)
            .map(|r| r.cost_gold)
            .max()
            .unwrap_or(i64::MAX);
        assert!(l15_min >= l12_max, "Level 15 equip recipes should cost >= level 12");
    }

    #[test]
    fn test_desc_not_empty() {
        for r in ALCHEMY_RECIPES {
            assert!(!r.desc.is_empty(), "Recipe '{}' should have a description", r.name);
        }
    }
}

//! 熔炼系统
//!
//! 将装备和物品熔炼为强化材料、精华等稀有资源
//! 来源: ext_rongzhi_info (空表, 使用内置配方)
//! 指令: 查看熔炼, 熔炼, 可熔炼

use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;

/// 熔炼配方
struct SmeltRecipe {
    /// 输入物品名
    input: &'static str,
    /// 输出物品名
    output: &'static str,
    /// 输出数量
    output_qty: i32,
    /// 消耗金币
    cost_gold: i64,
}

/// 内置熔炼配方
/// 设计思路: 装备→强化石/精华, 药剂→药剂精华, 材料→元素结晶
const SMELT_RECIPES: &[SmeltRecipe] = &[
    // === 装备熔炼 ===
    SmeltRecipe {
        input: "【普通】铁剑",
        output: "初级强化石",
        output_qty: 2,
        cost_gold: 50,
    },
    SmeltRecipe {
        input: "【普通】木杖",
        output: "初级强化石",
        output_qty: 2,
        cost_gold: 50,
    },
    SmeltRecipe {
        input: "【优秀】铁剑",
        output: "中级强化石",
        output_qty: 2,
        cost_gold: 200,
    },
    SmeltRecipe {
        input: "【优秀】木杖",
        output: "中级强化石",
        output_qty: 2,
        cost_gold: 200,
    },
    // === 药剂熔炼 ===
    SmeltRecipe {
        input: "【普通】生命药水",
        output: "药剂精华",
        output_qty: 1,
        cost_gold: 30,
    },
    SmeltRecipe {
        input: "【普通】魔力药水",
        output: "药剂精华",
        output_qty: 1,
        cost_gold: 30,
    },
    SmeltRecipe {
        input: "大生命药水",
        output: "药剂精华",
        output_qty: 2,
        cost_gold: 80,
    },
    SmeltRecipe {
        input: "大魔力药水",
        output: "药剂精华",
        output_qty: 2,
        cost_gold: 80,
    },
    SmeltRecipe {
        input: "超生命药水",
        output: "高级药剂精华",
        output_qty: 1,
        cost_gold: 200,
    },
    SmeltRecipe {
        input: "超魔力药水",
        output: "高级药剂精华",
        output_qty: 1,
        cost_gold: 200,
    },
    // === 材料熔炼 (升级) ===
    SmeltRecipe {
        input: "初级强化石",
        output: "中级强化石",
        output_qty: 1,
        cost_gold: 100,
    },
    SmeltRecipe {
        input: "中级强化石",
        output: "高级强化石",
        output_qty: 1,
        cost_gold: 500,
    },
    SmeltRecipe {
        input: "药剂精华",
        output: "高级药剂精华",
        output_qty: 1,
        cost_gold: 300,
    },
    // === 特殊熔炼 ===
    SmeltRecipe {
        input: "初级礼包",
        output: "元素结晶",
        output_qty: 1,
        cost_gold: 100,
    },
    SmeltRecipe {
        input: "中级礼包",
        output: "元素结晶",
        output_qty: 3,
        cost_gold: 300,
    },
    SmeltRecipe {
        input: "高级礼包",
        output: "元素结晶",
        output_qty: 5,
        cost_gold: 500,
    },
];

/// 查找配方 (精确或模糊匹配)
fn find_recipe(name: &str) -> Option<&'static SmeltRecipe> {
    // 精确匹配
    if let Some(r) = SMELT_RECIPES.iter().find(|r| r.input == name) {
        return Some(r);
    }
    // 模糊匹配
    SMELT_RECIPES
        .iter()
        .find(|r| r.input.contains(name) || name.contains(r.input))
}

/// 查看熔炼 — 显示所有熔炼配方
pub fn cmd_view_smelt(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看熔炼配方！", prefix);
    }

    let mut result = String::from(
        "╔══════════════════════╗\n\
         ║    🔥 熔炼系统 🔥    ║\n\
         ╚══════════════════════╝\n\n\
         将装备和物品熔炼为珍贵材料！\n\n",
    );

    // 按类别分组显示
    result.push_str("━━━ 装备熔炼 ━━━\n");
    for r in SMELT_RECIPES
        .iter()
        .filter(|r| r.input.contains('铁') || r.input.contains('木') || r.input.contains('杖'))
    {
        result.push_str(&format!(
            "  {} → {}×{} ({}金币)\n",
            r.input, r.output, r.output_qty, r.cost_gold
        ));
    }

    result.push_str("\n━━━ 药剂熔炼 ━━━\n");
    for r in SMELT_RECIPES
        .iter()
        .filter(|r| r.input.contains("药水") || r.input.contains("药剂"))
    {
        result.push_str(&format!(
            "  {} → {}×{} ({}金币)\n",
            r.input, r.output, r.output_qty, r.cost_gold
        ));
    }

    result.push_str("\n━━━ 材料升级 ━━━\n");
    for r in SMELT_RECIPES
        .iter()
        .filter(|r| r.input.contains("强化石") || r.input.contains("精华"))
    {
        result.push_str(&format!(
            "  {} → {}×{} ({}金币)\n",
            r.input, r.output, r.output_qty, r.cost_gold
        ));
    }

    result.push_str("\n━━━ 特殊熔炼 ━━━\n");
    for r in SMELT_RECIPES.iter().filter(|r| r.input.contains("礼包")) {
        result.push_str(&format!(
            "  {} → {}×{} ({}金币)\n",
            r.input, r.output, r.output_qty, r.cost_gold
        ));
    }

    result.push_str(&format!(
        "\n💡 共 {} 个配方\n\
         📌 使用「可熔炼」查看背包中可熔炼的物品\n\
         📌 使用「熔炼+物品名」进行熔炼",
        SMELT_RECIPES.len()
    ));

    result
}

/// 可熔炼 — 查看背包中可熔炼的物品
pub fn cmd_smeltable(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let backpack = db.knapsack_all(user_id);
    if backpack.is_empty() {
        return format!("{}\n背包为空，没有可熔炼的物品。", prefix);
    }

    let mut smeltable_items: Vec<(String, i32, &str, i32, i64)> = Vec::new();
    for item in &backpack {
        if let Some(recipe) = find_recipe(&item.name) {
            smeltable_items.push((
                item.name.clone(),
                item.quantity,
                recipe.output,
                recipe.output_qty,
                recipe.cost_gold,
            ));
        }
    }

    if smeltable_items.is_empty() {
        return format!(
            "{}\n背包中没有可熔炼的物品。\n💡 使用「查看熔炼」查看所有熔炼配方。",
            prefix
        );
    }

    let mut result = String::from(
        "╔═══════════════════╗\n\
         ║  🔥 可熔炼物品 🔥  ║\n\
         ╚═══════════════════╝\n\n",
    );

    for (i, (name, qty, output, out_qty, cost)) in smeltable_items.iter().enumerate() {
        result.push_str(&format!(
            "  {}. {} ×{} → {}×{} ({}金币/次)\n",
            i + 1,
            name,
            qty,
            output,
            out_qty,
            cost
        ));
    }

    result.push_str(&format!(
        "\n📌 使用「熔炼+物品名」进行熔炼\n\
         💰 当前金币：{}",
        db.read_currency(user_id, CURRENCY_GOLD)
    ));

    result
}

/// 熔炼 — 将物品熔炼为材料
pub fn cmd_smelt(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再进行熔炼！", prefix);
    }

    let item_name = args.trim();
    if item_name.is_empty() {
        return format!(
            "{}\n请指定要熔炼的物品。\n用法：熔炼+物品名\n💡 使用「可熔炼」查看背包中可熔炼的物品",
            prefix
        );
    }

    // 查找配方
    let recipe = match find_recipe(item_name) {
        Some(r) => r,
        None => {
            return format!(
                "{}\n未找到「{}」的熔炼配方。\n💡 使用「查看熔炼」查看所有配方。",
                prefix, item_name
            );
        }
    };

    // 检查背包中是否有该物品
    let backpack = db.knapsack_all(user_id);
    let has_item = backpack.iter().any(|item| item.name == recipe.input);
    if !has_item {
        // 尝试模糊匹配
        let similar: Vec<&str> = backpack
            .iter()
            .filter(|item| item.name.contains(item_name) || item_name.contains(item.name.as_str()))
            .map(|item| item.name.as_str())
            .collect();

        if similar.is_empty() {
            return format!(
                "{}\n背包中没有「{}」。\n💡 使用「可熔炼」查看背包中可熔炼的物品。",
                prefix, recipe.input
            );
        } else {
            return format!(
                "{}\n背包中没有「{}」，但你有：\n{}\n💡 试试「熔炼+{}」",
                prefix,
                recipe.input,
                similar
                    .iter()
                    .map(|s| format!("  · {}", s))
                    .collect::<Vec<_>>()
                    .join("\n"),
                similar[0]
            );
        }
    }

    // 体力检查 (熔炼消耗3体力)
    if let Err(e) = stamina::consume_stamina(user_id, "熔炼", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < recipe.cost_gold {
        return format!(
            "{}\n金币不足！熔炼「{}」需要{}金币，当前仅有{}金币。",
            prefix, recipe.input, recipe.cost_gold, gold
        );
    }

    // 执行熔炼
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, recipe.cost_gold);
    db.remove_item(user_id, recipe.input, 1);
    db.add_item(user_id, recipe.output, recipe.output_qty);

    let new_gold = db.read_currency(user_id, CURRENCY_GOLD);
    let output_count = db.knapsack_quantity(user_id, recipe.output);

    format!(
        "{}\n🔥 熔炼成功！\n\n\
         消耗：[{}]×1 + {}金币\n\
         获得：[{}]×{}\n\n\
         💰 剩余金币：{}\n\
         📦 {}库存：{}",
        prefix, recipe.input, recipe.cost_gold, recipe.output, recipe.output_qty, new_gold, recipe.output, output_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smelt_recipes_count() {
        assert_eq!(SMELT_RECIPES.len(), 16);
    }

    #[test]
    fn test_smelt_recipes_positive_outputs() {
        for r in SMELT_RECIPES {
            assert!(r.output_qty > 0, "{} output_qty must be positive", r.input);
            assert!(r.cost_gold > 0, "{} cost_gold must be positive", r.input);
        }
    }

    #[test]
    fn test_smelt_recipes_unique_inputs() {
        let mut inputs: Vec<&str> = SMELT_RECIPES.iter().map(|r| r.input).collect();
        let before = inputs.len();
        inputs.sort();
        inputs.dedup();
        assert_eq!(inputs.len(), before, "Smelt recipe inputs must be unique");
    }

    #[test]
    fn test_find_recipe_exact() {
        let r = find_recipe("【普通】铁剑");
        assert!(r.is_some());
        assert_eq!(r.unwrap().output, "初级强化石");
    }

    #[test]
    fn test_find_recipe_fuzzy() {
        let r = find_recipe("铁剑");
        assert!(r.is_some());
        assert_eq!(r.unwrap().output, "初级强化石");
    }

    #[test]
    fn test_find_recipe_not_found() {
        let r = find_recipe("不存在的物品");
        assert!(r.is_none());
    }

    #[test]
    fn test_smelt_recipe_categories() {
        let equip_count = SMELT_RECIPES
            .iter()
            .filter(|r| r.input.contains('铁') || r.input.contains('木') || r.input.contains('杖'))
            .count();
        let potion_count = SMELT_RECIPES
            .iter()
            .filter(|r| r.input.contains("药水") || r.input.contains("药剂"))
            .count();
        let material_count = SMELT_RECIPES
            .iter()
            .filter(|r| r.input.contains("强化石") || r.input.contains("精华"))
            .count();
        let special_count = SMELT_RECIPES.iter().filter(|r| r.input.contains("礼包")).count();
        assert!(equip_count > 0, "Must have equipment recipes");
        assert!(potion_count > 0, "Must have potion recipes");
        assert!(material_count > 0, "Must have material upgrade recipes");
        assert!(special_count > 0, "Must have special recipes");
        // Categories may overlap (e.g. "药剂精华" matches both potion and material)
        let categorized = equip_count + potion_count + material_count + special_count;
        assert!(
            categorized >= SMELT_RECIPES.len(),
            "Categorized {} should cover all {} recipes",
            categorized,
            SMELT_RECIPES.len()
        );
    }

    #[test]
    fn test_smelt_cost_increases_with_tier() {
        let basic = find_recipe("初级强化石").unwrap();
        let mid = find_recipe("中级强化石").unwrap();
        assert!(mid.cost_gold > basic.cost_gold, "Mid-tier upgrade should cost more");
    }
}

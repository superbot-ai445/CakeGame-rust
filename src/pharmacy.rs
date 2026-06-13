/// CakeGame 制药系统
/// 使用 ext_zhiyao_info 表（制药配方），支持查看配方、制药、查看可制药配方
/// 与种植系统联动：采集材料 → 制作药剂 → 战斗使用
use crate::db::Database;
use crate::stamina;
use crate::user;

/// 制药配方
struct PharmacyRecipe {
    name: &'static str,
    time: i32,
    ingredients: &'static [(&'static str, i32)],
    result_qty: i32,
}

/// 内置制药配方（模拟 ext_zhiyao_info 表数据）
/// 配方设计理念：从基础材料逐步升级到高级药剂
const PHARMACY_RECIPES: &[PharmacyRecipe] = &[
    PharmacyRecipe {
        name: "【普通】生命药水",
        time: 30,
        ingredients: &[("白色精粹", 10), ("【普通】红草药", 3)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "【普通】魔力药水",
        time: 30,
        ingredients: &[("蓝色精粹", 10), ("【普通】蓝草药", 3)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "【普通】大生命药水",
        time: 60,
        ingredients: &[("红色精粹", 5), ("白色精粹", 20)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "【普通】大魔力药水",
        time: 60,
        ingredients: &[("红色精粹", 5), ("蓝色精粹", 20)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "【稀有】超生命药水",
        time: 120,
        ingredients: &[("红色精粹", 20), ("紫色精粹", 5), ("强化石", 1)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "【稀有】超魔力药水",
        time: 120,
        ingredients: &[("蓝色精粹", 30), ("紫色精粹", 5), ("强化石", 1)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "【稀有】北姬的红药水",
        time: 180,
        ingredients: &[("小北姬花", 10), ("红色精粹", 30), ("紫色精粹", 10)],
        result_qty: 1,
    },
    PharmacyRecipe {
        name: "护肝药剂",
        time: 300,
        ingredients: &[("紫色精粹", 20), ("虚空宝石", 1)],
        result_qty: 1,
    },
];

/// 查看所有制药配方
pub fn cmd_view_pharmacy(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut out = format!("{}\n═══ 🧪 制药配方 ═══", prefix);
    out.push_str("\n使用「制药+药剂名」制作药剂\n");

    for (i, recipe) in PHARMACY_RECIPES.iter().enumerate() {
        let ingredients_str: Vec<String> = recipe
            .ingredients
            .iter()
            .map(|(name, qty)| format!("{}x{}", name, qty))
            .collect();
        out.push_str(&format!(
            "\n{}. [{}] ⏱{}秒 | 材料：{} | 产出：x{}",
            i + 1,
            recipe.name,
            recipe.time,
            ingredients_str.join(" + "),
            recipe.result_qty
        ));
    }

    out.push_str("\n\n💡 材料来源：种植系统收获 + 采集系统");
    out
}

/// 制药 — 制作药剂
pub fn cmd_craft_medicine(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let recipe_name = args.trim();

    if recipe_name.is_empty() {
        return format!(
            "{}\n❓ 请指定药剂名！用法：制药+药剂名\n💡 使用「查看制药」查看可用配方。",
            prefix
        );
    }

    // 查找配方（支持模糊匹配）
    let recipe = PHARMACY_RECIPES
        .iter()
        .find(|r| r.name == recipe_name)
        .or_else(|| PHARMACY_RECIPES.iter().find(|r| r.name.contains(recipe_name)));

    let recipe = match recipe {
        Some(r) => r,
        None => {
            let mut out = format!("{}\n❌ 未找到药剂「{}」！\n可用配方：", prefix, recipe_name);
            for r in PHARMACY_RECIPES {
                out.push_str(&format!("\n  · {}", r.name));
            }
            return out;
        }
    };

    // 体力检查 (制药消耗2体力)
    if let Err(e) = stamina::consume_stamina(user_id, "制药", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 检查生命值
    let hp_current: i32 = db
        .read_basic(user_id, crate::core::ITEM_HP_CURRENT)
        .parse()
        .unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再制药。", prefix);
    }

    // 检查材料
    let mut all_have = true;
    let mut missing = Vec::new();

    for &(item_name, need_qty) in recipe.ingredients {
        let have_qty = db.knapsack_quantity(user_id, item_name);
        if have_qty < need_qty {
            all_have = false;
            missing.push(format!("{}（需要{}个，拥有{}个）", item_name, need_qty, have_qty));
        }
    }

    if !all_have {
        let mut out = format!("{}\n❌ 材料不足，无法制作「{}」！", prefix, recipe.name);
        for m in &missing {
            out.push_str(&format!("\n  缺少：{}", m));
        }
        out.push_str("\n\n💡 材料来源：");
        out.push_str("\n  · 精粹类：种植系统收获药材后获得");
        out.push_str("\n  · 强化石/虚空宝石：种植高级种子获得");
        out.push_str("\n  · 草药类：背包使用或商店购买");
        return out;
    }

    // 消耗材料
    for &(item_name, need_qty) in recipe.ingredients {
        db.knapsack_remove(user_id, item_name, need_qty);
    }

    // 添加制作的药剂到背包
    db.knapsack_add(user_id, recipe.name, recipe.result_qty);

    let ingredients_str: Vec<String> = recipe
        .ingredients
        .iter()
        .map(|(name, qty)| format!("{}x{}", name, qty))
        .collect();

    format!(
        "{}\n═══ 🧪 制药成功 ═══\n\n📜 药剂：{}\n📦 获得：{} x{}\n🔧 消耗：{}\n⏱ 制作用时：{}秒\n\n💡 药剂已放入背包，战斗中可使用",
        prefix,
        recipe.name,
        recipe.name,
        recipe.result_qty,
        ingredients_str.join(" + "),
        recipe.time
    )
}

/// 查看可制药配方（材料齐全的）
pub fn cmd_available_pharmacy(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut available = Vec::new();
    let mut unavailable = Vec::new();

    for recipe in PHARMACY_RECIPES {
        let mut can_craft = true;
        let mut missing_items = Vec::new();

        for &(item_name, need_qty) in recipe.ingredients {
            let have_qty = db.knapsack_quantity(user_id, item_name);
            if have_qty < need_qty {
                can_craft = false;
                missing_items.push(format!("{}({}/{})", item_name, have_qty, need_qty));
            }
        }

        if can_craft {
            available.push(recipe.name);
        } else {
            unavailable.push(format!("{}: 缺{}", recipe.name, missing_items.join(",")));
        }
    }

    let mut out = format!("{}\n═══ 🧪 可制药配方 ═══", prefix);

    if available.is_empty() {
        out.push_str("\n暂无可制药配方（材料不足）");
    } else {
        for name in &available {
            out.push_str(&format!("\n✅ {}", name));
        }
    }

    if !unavailable.is_empty() {
        out.push_str("\n\n═══ ❌ 材料不足 ═══");
        for item in &unavailable {
            out.push_str(&format!("\n❌ {}", item));
        }
    }

    out.push_str("\n\n💡 使用「制药+药剂名」制作药剂");
    out.push_str("\n获取材料：种植系统 → 收获 → 获得精粹");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_count() {
        assert_eq!(PHARMACY_RECIPES.len(), 8, "Should have 8 pharmacy recipes");
    }

    #[test]
    fn test_recipe_names_unique() {
        let mut names: Vec<&str> = PHARMACY_RECIPES.iter().map(|r| r.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), PHARMACY_RECIPES.len(), "Recipe names should be unique");
    }

    #[test]
    fn test_recipe_positive_values() {
        for (i, recipe) in PHARMACY_RECIPES.iter().enumerate() {
            assert!(recipe.time > 0, "Recipe {} time should be > 0", i);
            assert!(recipe.result_qty > 0, "Recipe {} result_qty should be > 0", i);
            assert!(!recipe.ingredients.is_empty(), "Recipe {} should have ingredients", i);
            for &(name, qty) in recipe.ingredients {
                assert!(!name.is_empty(), "Ingredient name should not be empty");
                assert!(qty > 0, "Ingredient qty should be > 0");
            }
        }
    }

    #[test]
    fn test_recipe_escalating_time() {
        // Higher-tier recipes should generally take longer
        let basic_time = PHARMACY_RECIPES[0].time; // 30s
        let rare_time = PHARMACY_RECIPES[4].time; // 120s
        assert!(
            rare_time > basic_time,
            "Rare recipe ({}) > basic ({})",
            rare_time,
            basic_time
        );
    }

    #[test]
    fn test_recipe_ingredient_format() {
        // All ingredients should be (name, quantity) pairs
        for recipe in PHARMACY_RECIPES {
            for &(name, qty) in recipe.ingredients {
                assert!(name.len() >= 2, "Ingredient name '{}' too short", name);
                assert!(qty >= 1 && qty <= 100, "Ingredient qty {} out of range", qty);
            }
        }
    }
}

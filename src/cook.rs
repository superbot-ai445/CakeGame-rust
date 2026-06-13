/// CakeGame 烹饪系统
/// 读取 ext_cook_info 表，支持查看烹饪配方和制作食物
use crate::db::Database;
use crate::stamina;
use crate::user;

/// 查看所有烹饪配方
pub fn cmd_view_cooking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let recipes = db.get_all_cooking_recipes();

    if recipes.is_empty() {
        return format!("{}\n🍳 暂无烹饪配方数据！\n💡 后续更新将添加烹饪配方。", prefix);
    }

    let mut out = format!("{}\n═══ 🍳 烹饪配方 ═══", prefix);
    for (i, recipe) in recipes.iter().enumerate() {
        out.push_str(&format!(
            "\n{}. {} | ⏱{}秒 | 材料：{}",
            i + 1,
            recipe.name,
            recipe.time,
            recipe.foodstuff
        ));
    }
    out.push_str("\n\n💡 使用「烹饪+配方名」制作食物");
    out
}

/// 烹饪制作食物
pub fn cmd_cook(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let recipe_name = args.trim();

    if recipe_name.is_empty() {
        return format!(
            "{}\n❓ 请指定配方名！用法：烹饪+配方名\n💡 使用「查看烹饪」查看可用配方。",
            prefix
        );
    }

    // 查找配方
    let recipe = match db.get_cooking_recipe(recipe_name) {
        Some(r) => r,
        None => {
            return format!(
                "{}\n❌ 未找到配方「{}」！\n💡 使用「查看烹饪」查看可用配方。",
                prefix, recipe_name
            );
        }
    };

    // 体力检查 (烹饪消耗2体力)
    if let Err(e) = stamina::consume_stamina(user_id, "烹饪", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 解析所需材料 (格式: "物品名x数量|物品名x数量")
    let ingredients: Vec<&str> = recipe
        .foodstuff
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let mut all_have = true;
    let mut missing = Vec::new();

    for ingredient in &ingredients {
        // 解析 "物品名x数量" 格式
        let parts: Vec<&str> = ingredient.split('x').collect();
        let (item_name, need_qty) = if parts.len() >= 2 {
            let name = parts[0].trim();
            let qty: i32 = parts[1].trim().parse().unwrap_or(1);
            (name, qty)
        } else {
            (ingredient.trim(), 1)
        };

        if item_name.is_empty() {
            continue;
        }

        let have_qty = db.knapsack_quantity(user_id, item_name);
        if have_qty < need_qty {
            all_have = false;
            missing.push(format!("{}（需要{}个，拥有{}个）", item_name, need_qty, have_qty));
        }
    }

    if !all_have {
        let mut out = format!("{}\n❌ 材料不足，无法烹饪「{}」！", prefix, recipe_name);
        for m in &missing {
            out.push_str(&format!("\n  缺少：{}", m));
        }
        out.push_str("\n💡 去采集或商店获取材料。");
        return out;
    }

    // 消耗材料
    for ingredient in &ingredients {
        let parts: Vec<&str> = ingredient.split('x').collect();
        let (item_name, need_qty) = if parts.len() >= 2 {
            let name = parts[0].trim();
            let qty: i32 = parts[1].trim().parse().unwrap_or(1);
            (name, qty)
        } else {
            (ingredient.trim(), 1)
        };

        if item_name.is_empty() {
            continue;
        }
        db.knapsack_remove(user_id, item_name, need_qty);
    }

    // 添加制作的食物到背包（配方名作为物品名）
    db.knapsack_add(user_id, &recipe.name, 1);

    format!(
        "{}\n🍳 烹饪成功！\n📜 配方：{}\n🎒 获得：{} x1\n⏱ 烹饪用时：{}秒",
        prefix, recipe_name, recipe.name, recipe.time
    )
}

/// 查看可烹饪配方（材料齐全的）
pub fn cmd_available_cooking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let recipes = db.get_all_cooking_recipes();

    if recipes.is_empty() {
        return format!("{}\n🍳 暂无烹饪配方数据！", prefix);
    }

    let mut available = Vec::new();
    let mut unavailable = Vec::new();

    for recipe in &recipes {
        let ingredients: Vec<&str> = recipe
            .foodstuff
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let mut can_cook = true;
        let mut missing_items = Vec::new();

        for ingredient in &ingredients {
            let parts: Vec<&str> = ingredient.split('x').collect();
            let (item_name, need_qty) = if parts.len() >= 2 {
                let name = parts[0].trim();
                let qty: i32 = parts[1].trim().parse().unwrap_or(1);
                (name, qty)
            } else {
                (ingredient.trim(), 1)
            };

            if item_name.is_empty() {
                continue;
            }

            let have_qty = db.knapsack_quantity(user_id, item_name);
            if have_qty < need_qty {
                can_cook = false;
                missing_items.push(format!("{}({}/{})", item_name, have_qty, need_qty));
            }
        }

        if can_cook {
            available.push(recipe.name.clone());
        } else {
            unavailable.push(format!("{}: 缺{}", recipe.name, missing_items.join(",")));
        }
    }

    let mut out = format!("{}\n═══ 🍳 可烹饪配方 ═══", prefix);
    if available.is_empty() {
        out.push_str("\n暂无可烹饪配方（材料不足）");
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
    out.push_str("\n\n💡 使用「烹饪+配方名」制作食物");
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ingredient_parse_with_qty() {
        let ingredient = "铁矿石x3";
        let parts: Vec<&str> = ingredient.split('x').collect();
        assert_eq!(parts.len(), 2);
        let name = parts[0].trim();
        let qty: i32 = parts[1].trim().parse().unwrap_or(1);
        assert_eq!(name, "铁矿石");
        assert_eq!(qty, 3);
    }

    #[test]
    fn test_ingredient_parse_no_qty() {
        let ingredient = "铁矿石";
        let parts: Vec<&str> = ingredient.split('x').collect();
        assert_eq!(parts.len(), 1);
        let name = parts[0].trim();
        let qty: i32 = if parts.len() >= 2 {
            parts[1].trim().parse().unwrap_or(1)
        } else {
            1
        };
        assert_eq!(name, "铁矿石");
        assert_eq!(qty, 1);
    }

    #[test]
    fn test_ingredient_parse_multiple() {
        let foodstuff = "铁矿石x3|木材x2|药草x1";
        let ingredients: Vec<&str> = foodstuff
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(ingredients.len(), 3);
    }

    #[test]
    fn test_ingredient_parse_empty() {
        let foodstuff = "";
        let ingredients: Vec<&str> = foodstuff
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(ingredients.is_empty());
    }

    #[test]
    fn test_ingredient_parse_with_pipe_separator() {
        let foodstuff = "铁矿石x5|木材x3";
        let mut parsed = Vec::new();
        for ingredient in foodstuff.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            let parts: Vec<&str> = ingredient.split('x').collect();
            let (item_name, need_qty) = if parts.len() >= 2 {
                (parts[0].trim(), parts[1].trim().parse().unwrap_or(1))
            } else {
                (ingredient.trim(), 1)
            };
            parsed.push((item_name.to_string(), need_qty));
        }
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "铁矿石");
        assert_eq!(parsed[0].1, 5);
        assert_eq!(parsed[1].0, "木材");
        assert_eq!(parsed[1].1, 3);
    }
}

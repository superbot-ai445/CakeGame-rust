/// 装备附魔系统 — 为装备添加元素属性加成
/// 使用附魔材料为装备附加额外属性（物攻/魔攻/防御/魔抗/生命/魔法）
use crate::core::*;
use crate::db::Database;

/// 附魔配方定义
struct EnchantRecipe {
    name: &'static str,
    enchant_type: &'static str,
    bonus: i32,
    gold_cost: i32,
    material: &'static str,
    material_count: i32,
    level_req: i32,
}

fn get_enchant_recipes() -> Vec<EnchantRecipe> {
    vec![
        EnchantRecipe {
            name: "火焰附魔",
            enchant_type: "AD",
            bonus: 15,
            gold_cost: 500,
            material: "强化石",
            material_count: 3,
            level_req: 5,
        },
        EnchantRecipe {
            name: "寒冰附魔",
            enchant_type: "AP",
            bonus: 15,
            gold_cost: 500,
            material: "白色精粹",
            material_count: 3,
            level_req: 5,
        },
        EnchantRecipe {
            name: "大地附魔",
            enchant_type: "Defense",
            bonus: 12,
            gold_cost: 400,
            material: "强化石",
            material_count: 2,
            level_req: 3,
        },
        EnchantRecipe {
            name: "暗影附魔",
            enchant_type: "MagicResistance",
            bonus: 12,
            gold_cost: 400,
            material: "白色精粹",
            material_count: 2,
            level_req: 3,
        },
        EnchantRecipe {
            name: "生命附魔",
            enchant_type: "HP",
            bonus: 200,
            gold_cost: 600,
            material: "红色精粹",
            material_count: 2,
            level_req: 8,
        },
        EnchantRecipe {
            name: "魔力附魔",
            enchant_type: "MP",
            bonus: 150,
            gold_cost: 600,
            material: "蓝色精粹",
            material_count: 2,
            level_req: 8,
        },
        EnchantRecipe {
            name: "烈焰附魔",
            enchant_type: "AD",
            bonus: 30,
            gold_cost: 1500,
            material: "红色精粹",
            material_count: 5,
            level_req: 15,
        },
        EnchantRecipe {
            name: "雷霆附魔",
            enchant_type: "AP",
            bonus: 30,
            gold_cost: 1500,
            material: "紫色精粹",
            material_count: 5,
            level_req: 15,
        },
        EnchantRecipe {
            name: "圣光附魔",
            enchant_type: "Defense",
            bonus: 25,
            gold_cost: 1200,
            material: "绿色精粹",
            material_count: 4,
            level_req: 12,
        },
        EnchantRecipe {
            name: "虚空附魔",
            enchant_type: "MagicResistance",
            bonus: 25,
            gold_cost: 1200,
            material: "虚空宝石",
            material_count: 2,
            level_req: 12,
        },
        EnchantRecipe {
            name: "龙血附魔",
            enchant_type: "HP",
            bonus: 500,
            gold_cost: 3000,
            material: "远古超界石",
            material_count: 1,
            level_req: 20,
        },
        EnchantRecipe {
            name: "奥术附魔",
            enchant_type: "MP",
            bonus: 400,
            gold_cost: 3000,
            material: "元素结晶",
            material_count: 2,
            level_req: 20,
        },
    ]
}

fn enchant_type_name(t: &str) -> &str {
    match t {
        "AD" => "物攻",
        "AP" => "魔攻",
        "Defense" => "防御",
        "MagicResistance" => "魔抗",
        "HP" => "生命",
        "MP" => "魔法",
        _ => t,
    }
}

fn get_level(db: &Database, user_id: &str) -> i32 {
    db.read_user_data(user_id, "LV").parse::<i32>().unwrap_or(1)
}

/// 查看附魔 — 列出所有附魔配方或查看装备当前附魔
pub fn cmd_view_enchant(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let user_level = get_level(db, user_id);
    let args = args.trim();

    // 查看特定装备的附魔状态
    if !args.is_empty() {
        return view_equip_enchant(db, user_id, args);
    }

    let recipes = get_enchant_recipes();
    let mut out = format!("{}\n🔮 【附魔系统】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for (i, r) in recipes.iter().enumerate() {
        let locked = user_level < r.level_req;
        let lock_icon = if locked { "🔒" } else { "🔮" };
        out.push_str(&format!(
            "{} {}. {} → {}+{} (💰{} 📦{}×{})\n",
            lock_icon,
            i + 1,
            r.name,
            enchant_type_name(r.enchant_type),
            r.bonus,
            r.gold_cost,
            r.material,
            r.material_count,
        ));
        if locked {
            out.push_str(&format!("   ⚠️ 需要等级{}\n", r.level_req));
        }
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「附魔+附魔名+装备名」进行附魔\n");
    out.push_str("💡 使用「查看附魔+装备名」查看装备附魔状态\n");
    out.push_str("💡 使用「可附魔」查看可附魔的装备\n");
    out
}

/// 查看装备附魔状态
fn view_equip_enchant(db: &Database, user_id: &str, equip_name: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    // 检查装备是否存在（背包或装备栏）
    let in_knapsack: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM Basic_knapsack WHERE ID=?1 AND Name=?2",
            rusqlite::params![user_id, equip_name],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let in_equips: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM Equip_Register WHERE User=?1 AND EquipName=?2",
            rusqlite::params![user_id, equip_name],
            |row| row.get(0),
        )
        .unwrap_or(0);

    drop(conn);

    if in_knapsack == 0 && in_equips == 0 {
        return format!("{}\n⚠️ 背包和装备栏中未找到 [{}]", prefix, equip_name);
    }

    // 读取附魔数据
    let enchant_data = db.read_user_data(user_id, &format!("enchant_{}", equip_name));

    if enchant_data.is_empty() {
        return format!(
            "{}\n🔮 [{}] 尚未附魔\n💡 使用「附魔+附魔名+装备名」进行附魔",
            prefix, equip_name
        );
    }

    // 解析附魔数据: "类型|值"
    let parts: Vec<&str> = enchant_data.split('|').collect();
    if parts.len() < 2 {
        return format!("{}\n🔮 [{}] 附魔数据异常", prefix, equip_name);
    }

    let etype = parts[0];
    let val: i32 = parts[1].parse().unwrap_or(0);

    let mut out = format!("{}\n🔮 【{}附魔信息】\n━━━━━━━━━━━━━━━━━━━━\n", prefix, equip_name);
    out.push_str(&format!("附魔属性: {}+{}\n", enchant_type_name(etype), val));
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「附魔」可重新附魔覆盖当前附魔\n");
    out
}

/// 可附魔 — 显示背包中可附魔的装备
pub fn cmd_enchantable(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare(
        "SELECT Name, COUNT(*) as cnt FROM Basic_knapsack WHERE ID=?1 \
         AND (Name LIKE '%剑%' OR Name LIKE '%杖%' OR Name LIKE '%锤%' OR Name LIKE '%刀%' \
         OR Name LIKE '%甲%' OR Name LIKE '%衣%' OR Name LIKE '%盾%' OR Name LIKE '%盔%' \
         OR Name LIKE '%靴%' OR Name LIKE '%肩%' OR Name LIKE '%护%' OR Name LIKE '%戒%' \
         OR Name LIKE '%项链%' OR Name LIKE '%披风%' OR Name LIKE '%腰带%') \
         GROUP BY Name ORDER BY cnt DESC LIMIT 20",
    ) {
        Ok(s) => s,
        Err(e) => return format!("{}\n⚠️ 查询失败: {}", prefix, e),
    };

    let items: Vec<(String, i32)> = stmt
        .query_map([user_id], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, i32>(1).unwrap_or(1),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    drop(stmt);
    drop(conn);

    if items.is_empty() {
        return format!("{}\n📦 背包中没有可附魔的装备", prefix);
    }

    let mut out = format!("{}\n🔮 【可附魔装备】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);

    for (item_name, cnt) in &items {
        let ench_key = format!("enchant_{}", item_name);
        let ench_data = db.read_user_data(user_id, &ench_key);
        let ench_status = if ench_data.is_empty() {
            "⬜ 未附魔".to_string()
        } else {
            let parts: Vec<&str> = ench_data.split('|').collect();
            if parts.len() >= 2 {
                format!("🔮 {}+{}", enchant_type_name(parts[0]), parts[1])
            } else {
                "🔮 已附魔".to_string()
            }
        };
        let cnt_str = if *cnt > 1 { format!("×{}", cnt) } else { String::new() };
        out.push_str(&format!("  {} {}{}\n", item_name, cnt_str, ench_status));
    }

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「附魔+附魔名+装备名」进行附魔\n");
    out
}

/// 附魔 — 为装备添加附魔效果
pub fn cmd_enchant(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let user_level = get_level(db, user_id);

    // 检查死亡状态
    let hp: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n⚠️ 阵亡状态无法进行附魔操作", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n⚠️ 请指定附魔名称和装备名\n💡 格式: 附魔+附魔名+装备名\n💡 例: 附魔+火焰附魔+铁剑",
            prefix
        );
    }

    // 解析参数: 先尝试匹配已知附魔名
    let recipes = get_enchant_recipes();
    let mut matched_recipe: Option<&EnchantRecipe> = None;
    let mut equip_name = "";

    for r in &recipes {
        if let Some(rest) = args.strip_prefix(r.name) {
            let rest = rest.trim_start_matches('+').trim();
            if !rest.is_empty() {
                matched_recipe = Some(r);
                equip_name = rest;
                break;
            }
        }
    }

    // 尝试模糊匹配附魔名
    if matched_recipe.is_none() {
        for r in &recipes {
            if args.contains(r.name) {
                let idx = args.find(r.name).unwrap();
                let rest = args[idx + r.name.len()..].trim_start_matches('+').trim();
                if !rest.is_empty() {
                    matched_recipe = Some(r);
                    equip_name = rest;
                    break;
                }
            }
        }
    }

    let recipe = match matched_recipe {
        Some(r) => r,
        None => {
            return format!(
                "{}\n⚠️ 未找到匹配的附魔名称\n💡 可用附魔: {}\n💡 格式: 附魔+附魔名+装备名",
                prefix,
                recipes.iter().map(|r| r.name).collect::<Vec<_>>().join("、")
            );
        }
    };

    if equip_name.is_empty() {
        return format!("{}\n⚠️ 请指定要附魔的装备名", prefix);
    }

    // 等级检查
    if user_level < recipe.level_req {
        return format!(
            "{}\n⚠️ 等级不足，{}需要等级{}，当前等级{}",
            prefix, recipe.name, recipe.level_req, user_level
        );
    }

    let conn = db.lock_conn();

    // 检查装备是否在背包中
    let in_knapsack: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM Basic_knapsack WHERE ID=?1 AND Name=?2",
            rusqlite::params![user_id, equip_name],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // 检查装备是否在装备栏中
    let in_equips: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM Equip_Register WHERE User=?1 AND EquipName=?2",
            rusqlite::params![user_id, equip_name],
            |row| row.get(0),
        )
        .unwrap_or(0);

    drop(conn);

    if in_knapsack == 0 && in_equips == 0 {
        return format!("{}\n⚠️ 背包和装备栏中未找到 [{}]", prefix, equip_name);
    }

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < recipe.gold_cost as i64 {
        return format!("{}\n⚠️ 金币不足，需要💰{}，当前💰{}", prefix, recipe.gold_cost, gold);
    }

    // 检查材料
    let mat_count = db.knapsack_quantity(user_id, recipe.material);
    if mat_count < recipe.material_count {
        return format!(
            "{}\n⚠️ 材料不足，需要📦{}×{}，当前拥有{}个\n💡 材料可通过采集、分解、炼制获得",
            prefix, recipe.material, recipe.material_count, mat_count
        );
    }

    // 扣除金币
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, recipe.gold_cost as i64);
    // 扣除材料
    db.knapsack_remove(user_id, recipe.material, recipe.material_count);

    // 保存附魔数据
    let ench_value = format!("{}|{}", recipe.enchant_type, recipe.bonus);
    db.write_user_data(user_id, &format!("enchant_{}", equip_name), &ench_value);

    let mut out = format!("{}\n🎉 附魔成功！\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(&format!("装备: {}\n", equip_name));
    out.push_str(&format!(
        "附魔: {} → {}+{}\n",
        recipe.name,
        enchant_type_name(recipe.enchant_type),
        recipe.bonus
    ));
    out.push_str(&format!(
        "消耗: 💰{} + 📦{}×{}\n",
        recipe.gold_cost, recipe.material, recipe.material_count
    ));
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    // 更新装备属性（在装备栏中的装备直接加成）
    if in_equips > 0 {
        let col = match recipe.enchant_type {
            "AD" => "Add_AD",
            "AP" => "Add_AP",
            "Defense" => "Add_Defense",
            "MagicResistance" => "Add_MagicResistance",
            "HP" => "Add_HP",
            "MP" => "Add_MP",
            _ => "",
        };
        if !col.is_empty() {
            let conn2 = db.lock_conn();
            let _ = conn2.execute(
                &format!(
                    "UPDATE Equip_Register SET {} = {} + ?1 WHERE User=?2 AND EquipName=?3",
                    col, col
                ),
                rusqlite::params![recipe.bonus, user_id, equip_name],
            );
            drop(conn2);
            out.push_str("✅ 附魔加成已自动应用到装备属性\n");
        }
    } else {
        out.push_str("💡 装备在背包中，穿上后附魔加成自动生效\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enchant_recipes_count() {
        let recipes = get_enchant_recipes();
        assert!(recipes.len() >= 8, "Expected at least 8 recipes, got {}", recipes.len());
    }

    #[test]
    fn test_enchant_recipes_unique_names() {
        let recipes = get_enchant_recipes();
        let mut names: Vec<&str> = recipes.iter().map(|r| r.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len(), "Duplicate recipe names");
    }

    #[test]
    fn test_enchant_recipes_positive_values() {
        for r in &get_enchant_recipes() {
            assert!(r.bonus > 0, "{}: bonus must be > 0", r.name);
            assert!(r.gold_cost > 0, "{}: gold_cost must be > 0", r.name);
            assert!(r.material_count > 0, "{}: material_count must be > 0", r.name);
            assert!(r.level_req > 0, "{}: level_req must be > 0", r.name);
        }
    }

    #[test]
    fn test_enchant_recipes_valid_types() {
        let valid = ["AD", "AP", "Defense", "MagicResistance", "HP", "MP"];
        for r in &get_enchant_recipes() {
            assert!(valid.contains(&r.enchant_type), "Invalid type: {}", r.enchant_type);
        }
    }

    #[test]
    fn test_enchant_type_name_all() {
        assert_eq!(enchant_type_name("AD"), "物攻");
        assert_eq!(enchant_type_name("AP"), "魔攻");
        assert_eq!(enchant_type_name("Defense"), "防御");
        assert_eq!(enchant_type_name("MagicResistance"), "魔抗");
        assert_eq!(enchant_type_name("HP"), "生命");
        assert_eq!(enchant_type_name("MP"), "魔法");
    }

    #[test]
    fn test_enchant_type_name_passthrough() {
        assert_eq!(enchant_type_name("Unknown"), "Unknown");
        assert_eq!(enchant_type_name(""), "");
    }
}

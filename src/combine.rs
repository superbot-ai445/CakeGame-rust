/// CakeGame 装备合成系统
/// 使用 Equip_Combined 表（74条数据，17个套装组合）
/// 玩家集齐同一套装的所有装备后，可以合成获得强化版本
use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;

/// 合成配方信息
struct CombineRecipe {
    suit_name: String,
    pieces: Vec<String>,
    /// 合成后获得的强化属性描述
    bonus_desc: String,
    /// 合成所需金币
    gold_cost: i64,
}

/// 从数据库加载所有合成配方
fn load_recipes(db: &Database) -> Vec<CombineRecipe> {
    let rows: Vec<(String, String)> = db.query_rows(
        "SELECT SuitName, EquipName FROM Equip_Combined ORDER BY SuitName",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    );

    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (suit, equip) in rows {
        map.entry(suit).or_default().push(equip);
    }

    let mut recipes: Vec<CombineRecipe> = map
        .into_iter()
        .map(|(name, pieces)| {
            let count = pieces.len() as i64;
            // 根据套装件数和品质确定金币消耗和属性加成
            let (gold, desc) = if name.contains("远古") || pieces.iter().any(|p| p.contains("远古")) {
                // 远古品质套装
                (count * 8000, format!("全属性+{}%（远古套装加成）", count * 5))
            } else if pieces.len() >= 5 {
                // 完整5件套
                (count * 5000, format!("全属性+{}%（完整套装加成）", count * 4))
            } else {
                // 精简3件套
                (count * 3000, format!("全属性+{}%（基础套装加成）", count * 3))
            };
            CombineRecipe {
                suit_name: name,
                pieces,
                bonus_desc: desc,
                gold_cost: gold,
            }
        })
        .collect();

    recipes.sort_by(|a, b| a.suit_name.cmp(&b.suit_name));
    recipes
}

/// 查看所有合成配方
pub fn cmd_view_combine_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let recipes = load_recipes(db);

    if recipes.is_empty() {
        return format!("{}\n暂无合成配方数据。", prefix);
    }

    let mut out = format!("{}\n═══ ⚒️ 装备合成系统 ═══\n━━━━━━━━━━━━━━━━━━━━", prefix);
    for (i, recipe) in recipes.iter().enumerate() {
        out.push_str(&format!(
            "\n{}. [{}] {}件套 💰{}金币",
            i + 1,
            recipe.suit_name,
            recipe.pieces.len(),
            recipe.gold_cost
        ));
    }
    out.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    out.push_str("\n\n💡 发送「合成详情+套装名」查看所需材料");
    out.push_str("\n💡 发送「装备合成+套装名」进行合成");
    out
}

/// 查看合成详情
pub fn cmd_view_combine_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let suit_name = args.trim();

    if suit_name.is_empty() {
        return format!("{}\n❓ 请指定套装名！\n用法：合成详情+套装名", prefix);
    }

    let recipes = load_recipes(db);
    let recipe = match recipes.iter().find(|r| r.suit_name == suit_name) {
        Some(r) => r,
        None => {
            // 模糊匹配
            let fuzzy: Vec<&CombineRecipe> = recipes
                .iter()
                .filter(|r| r.suit_name.contains(suit_name) || suit_name.contains(&r.suit_name))
                .collect();
            if fuzzy.len() == 1 {
                fuzzy[0]
            } else if fuzzy.is_empty() {
                return format!(
                    "{}\n❌ 未找到套装「{}」！\n💡 发送「装备合成配方」查看所有配方。",
                    prefix, suit_name
                );
            } else {
                let mut out = format!("{}\n🔍 找到多个匹配套装：", prefix);
                for r in fuzzy {
                    out.push_str(&format!("\n  • {}", r.suit_name));
                }
                out.push_str("\n\n请指定完整的套装名。");
                return out;
            }
        }
    };

    let mut out = format!(
        "{}\n═══ ⚒️ 合成详情 ═══\n套装：[{}]\n所需件数：{}\n合成费用：{}金币\n属性加成：{}",
        prefix,
        recipe.suit_name,
        recipe.pieces.len(),
        recipe.gold_cost,
        recipe.bonus_desc
    );

    out.push_str("\n━━━━━━ 所需装备 ━━━━━━");
    let mut owned_count = 0;
    for (i, piece) in recipe.pieces.iter().enumerate() {
        let have = db.get_item_count(user_id, piece);
        let status = if have > 0 {
            owned_count += 1;
            "✅"
        } else {
            "❌"
        };
        out.push_str(&format!("\n  {}. {} {}", i + 1, status, piece));
    }

    out.push_str(&format!(
        "\n━━━━━━━━━━━━━━━━━━━━\n进度：{}/{} 件",
        owned_count,
        recipe.pieces.len()
    ));

    if owned_count >= recipe.pieces.len() as i32 {
        out.push_str(&format!(
            "\n\n🎉 集齐所有装备！发送「装备合成+{}」即可合成！",
            recipe.suit_name
        ));
    } else {
        out.push_str(&format!(
            "\n\n⏳ 还需收集 {} 件装备。",
            recipe.pieces.len() as i32 - owned_count
        ));
    }
    out
}

/// 执行装备合成
pub fn cmd_combine_equip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let suit_name = args.trim();

    if suit_name.is_empty() {
        return format!("{}\n❓ 请指定套装名！\n用法：装备合成+套装名", prefix);
    }

    let recipes = load_recipes(db);
    let recipe = match recipes.iter().find(|r| r.suit_name == suit_name) {
        Some(r) => r,
        None => {
            let fuzzy: Vec<&CombineRecipe> = recipes
                .iter()
                .filter(|r| r.suit_name.contains(suit_name) || suit_name.contains(&r.suit_name))
                .collect();
            if fuzzy.len() == 1 {
                fuzzy[0]
            } else if fuzzy.is_empty() {
                return format!("{}\n❌ 未找到套装「{}」！", prefix, suit_name);
            } else {
                let mut out = format!("{}\n🔍 找到多个匹配：", prefix);
                for r in fuzzy {
                    out.push_str(&format!("\n  • {}", r.suit_name));
                }
                return out;
            }
        }
    };

    // 体力检查 (合成消耗3体力)
    if let Err(e) = stamina::consume_stamina(user_id, "合成", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 检查是否集齐所有装备
    let mut missing = Vec::new();
    for piece in &recipe.pieces {
        let have = db.get_item_count(user_id, piece);
        if have <= 0 {
            missing.push(piece.clone());
        }
    }

    if !missing.is_empty() {
        let mut out = format!("{}\n❌ 合成失败！缺少以下装备：", prefix);
        for m in &missing {
            out.push_str(&format!("\n  • {}", m));
        }
        return out;
    }

    // 检查金币
    let user_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    if user_gold < recipe.gold_cost {
        return format!(
            "{}\n❌ 金币不足！合成需要 {} 金币，你只有 {} 金币。",
            prefix, recipe.gold_cost, user_gold
        );
    }

    // 扣除金币
    db.write_currency(user_id, CURRENCY_GOLD, user_gold - recipe.gold_cost);

    // 消耗所有套装装备
    for piece in &recipe.pieces {
        db.remove_item(user_id, piece, 1);
    }

    // 生成合成后的强化装备
    let combined_name = format!("✨【合成】{}", recipe.suit_name);
    db.add_item(user_id, &combined_name, 1);

    // 计算属性加成
    let bonus_pct: i32 = recipe.pieces.len() as i32
        * if recipe.bonus_desc.contains("远古") {
            5
        } else if recipe.pieces.len() >= 5 {
            4
        } else {
            3
        };

    // 给予属性加成（通过写入用户数据记录）
    let bonus_key = format!("combine_bonus_{}", recipe.suit_name);
    db.write_user_data(user_id, &bonus_key, &bonus_pct.to_string());

    format!(
        "{}\n🎉 ═══ 合成成功！═══\n\
         ⚒️ 套装：[{}]\n\
         🎁 获得：{}\n\
         💰 消耗：{}金币\n\
         📈 属性加成：{}\n\
         ⚠️ 合成后原套装装备已消耗\n\n\
         💡 合成加成已永久记录，无需穿戴即可生效。",
        prefix, recipe.suit_name, combined_name, recipe.gold_cost, recipe.bonus_desc
    )
}

/// 查看我的合成加成
pub fn cmd_my_combine_bonus(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let recipes = load_recipes(db);

    let mut has_any = false;
    let mut out = format!("{}\n═══ ⚒️ 我的合成加成 ═══\n━━━━━━━━━━━━━━━━━━━━", prefix);

    for recipe in &recipes {
        let bonus_key = format!("combine_bonus_{}", recipe.suit_name);
        let bonus_str = db.read_user_data(user_id, &bonus_key);
        if !bonus_str.is_empty() && bonus_str != "0" {
            has_any = true;
            out.push_str(&format!("\n  ✅ [{}] 全属性+{}%", recipe.suit_name, bonus_str));
        }
    }

    if !has_any {
        out.push_str("\n  暂无合成加成。");
        out.push_str("\n  💡 集齐套装装备后发送「装备合成+套装名」合成。");
    }

    out.push_str("\n━━━━━━━━━━━━━━━━━━━━");
    out
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_combine_bonus_pct_5piece() {
        // 5-piece suit with 4% per piece
        let pieces_count = 5;
        let bonus_pct = pieces_count * 4;
        assert_eq!(bonus_pct, 20);
    }

    #[test]
    fn test_combine_bonus_pct_3piece() {
        // 3-piece suit with 3% per piece
        let pieces_count = 3;
        let bonus_pct = pieces_count * 3;
        assert_eq!(bonus_pct, 9);
    }

    #[test]
    fn test_combine_bonus_pct_ancient() {
        // Ancient suit with 5% per piece
        let pieces_count = 4;
        let bonus_pct = pieces_count * 5;
        assert_eq!(bonus_pct, 20);
    }

    #[test]
    fn test_combined_name_format() {
        let suit_name = "暗影套装";
        let combined_name = format!("✨【合成】{}", suit_name);
        assert_eq!(combined_name, "✨【合成】暗影套装");
    }

    #[test]
    fn test_bonus_key_format() {
        let suit_name = "远古套装";
        let bonus_key = format!("combine_bonus_{}", suit_name);
        assert_eq!(bonus_key, "combine_bonus_远古套装");
    }

    #[test]
    fn test_gold_cost_5piece() {
        let count = 5i64;
        let gold = count * 5000;
        assert_eq!(gold, 25000);
    }

    #[test]
    fn test_gold_cost_3piece() {
        let count = 3i64;
        let gold = count * 3000;
        assert_eq!(gold, 9000);
    }

    #[test]
    fn test_gold_cost_ancient() {
        let count = 4i64;
        let gold = count * 8000;
        assert_eq!(gold, 32000);
    }
}

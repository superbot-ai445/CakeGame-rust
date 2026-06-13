use crate::db::Database;

/// 收集图鉴系统 - 追踪玩家发现的物品/装备/怪物，达成里程碑获得奖励
/// 数据来源: Config_Goods (345物品), Config_Monster (32怪物), Equip_Register (装备)
/// 存储: user_data keys: collected_items, collected_monsters, collection_milestones
///
/// 收集里程碑配置
struct Milestone {
    count: u32,
    name: &'static str,
    reward_gold: i64,
    reward_diamond: i64,
    reward_item: &'static str,
}

fn milestones() -> Vec<Milestone> {
    vec![
        Milestone {
            count: 10,
            name: "初窥门径",
            reward_gold: 500,
            reward_diamond: 20,
            reward_item: "",
        },
        Milestone {
            count: 30,
            name: "见多识广",
            reward_gold: 1500,
            reward_diamond: 50,
            reward_item: "生命药水*3",
        },
        Milestone {
            count: 60,
            name: "博闻强识",
            reward_gold: 3000,
            reward_diamond: 100,
            reward_item: "强化石*2",
        },
        Milestone {
            count: 100,
            name: "学富五车",
            reward_gold: 5000,
            reward_diamond: 200,
            reward_item: "初级礼包*1",
        },
        Milestone {
            count: 150,
            name: "百科全书",
            reward_gold: 10000,
            reward_diamond: 400,
            reward_item: "中级礼包*1",
        },
        Milestone {
            count: 200,
            name: "万物皆明",
            reward_gold: 20000,
            reward_diamond: 800,
            reward_item: "高级礼包*1",
        },
        Milestone {
            count: 300,
            name: "图鉴大师",
            reward_gold: 50000,
            reward_diamond: 1500,
            reward_item: "史诗碎片*5",
        },
    ]
}

/// 记录玩家收集到的物品 (由其他系统调用)
pub fn record_item_collection(db: &Database, user_id: &str, item_name: &str) {
    let collected = db.read_user_data(user_id, "collected_items");
    if collected.is_empty() {
        db.write_user_data(user_id, "collected_items", item_name);
    } else if !collected.split(',').any(|s| s == item_name) {
        let new_val = format!("{},{}", collected, item_name);
        db.write_user_data(user_id, "collected_items", &new_val);
    }
}

/// 记录玩家发现的怪物 (由战斗系统调用)
pub fn record_monster_discovery(db: &Database, user_id: &str, monster_name: &str) {
    let collected = db.read_user_data(user_id, "collected_monsters");
    if collected.is_empty() {
        db.write_user_data(user_id, "collected_monsters", monster_name);
    } else if !collected.split(',').any(|s| s == monster_name) {
        let new_val = format!("{},{}", collected, monster_name);
        db.write_user_data(user_id, "collected_monsters", &new_val);
    }
}

/// 查看收集图鉴 - 显示收集进度和里程碑
pub fn cmd_view_collection(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看收集图鉴！\n发送【注册+昵称】进行注册".to_string();
    }

    let collected_items_str = db.read_user_data(user_id, "collected_items");
    let collected_monsters_str = db.read_user_data(user_id, "collected_monsters");

    let item_list: Vec<&str> = collected_items_str.split(',').filter(|s| !s.is_empty()).collect();
    let monster_list: Vec<&str> = collected_monsters_str.split(',').filter(|s| !s.is_empty()).collect();

    let item_count = item_list.len() as u32;
    let monster_count = monster_list.len() as u32;
    let total_count = item_count + monster_count;

    let args_trimmed = args.trim();

    // 子命令: 查看已收集物品列表
    if args_trimmed.contains("物品") || args_trimmed.contains("道具") {
        if item_list.is_empty() {
            return "📦 物品图鉴为空！\n去冒险收集物品吧~".to_string();
        }
        let mut out = format!("📦 物品收集图鉴 ({}/345)\n{}\n", item_count, "─".repeat(20));
        for (i, name) in item_list.iter().enumerate() {
            out.push_str(&format!("  ✅ {}. {}\n", i + 1, name));
        }
        out.push_str("\n💡 收集更多物品获得里程碑奖励！");
        return out;
    }

    // 子命令: 查看已发现怪物列表
    if args_trimmed.contains("怪物") || args_trimmed.contains("魔物") {
        if monster_list.is_empty() {
            return "👹 怪物图鉴为空！\n去战斗发现怪物吧~".to_string();
        }
        let mut out = format!("👹 怪物发现图鉴 ({}/32)\n{}\n", monster_count, "─".repeat(20));
        for (i, name) in monster_list.iter().enumerate() {
            out.push_str(&format!("  ✅ {}. {}\n", i + 1, name));
        }
        out.push_str("\n💡 发现更多怪物获得里程碑奖励！");
        return out;
    }

    // 子命令: 领取里程碑奖励
    if args_trimmed.contains("领取") || args_trimmed.contains("奖励") {
        return claim_collection_milestone(db, user_id, total_count);
    }

    // 默认: 显示总览
    let mut out = String::from("📚 收集图鉴总览\n");
    out.push_str(&format!("{}\n", "═".repeat(24)));
    out.push_str(&format!("  📦 物品收集: {}/345\n", item_count));
    out.push_str(&format!("  👹 怪物发现: {}/32\n", monster_count));
    out.push_str(&format!("  📊 总收集度: {} / 377\n\n", total_count));

    // 进度条
    let pct = (total_count as f64 / 377.0 * 100.0).min(100.0);
    let filled = (pct / 5.0) as usize;
    let bar: String = "█".repeat(filled) + &"░".repeat(20 - filled);
    out.push_str(&format!("  进度: [{}] {:.1}%\n\n", bar, pct));

    // 里程碑状态
    let claimed_str = db.read_user_data(user_id, "collection_milestones");
    let claimed_set: std::collections::HashSet<u32> =
        claimed_str.split(',').filter_map(|s| s.parse::<u32>().ok()).collect();

    out.push_str("🏆 收集里程碑\n");
    for ms in milestones() {
        let status = if claimed_set.contains(&ms.count) {
            "✅ 已领取"
        } else if total_count >= ms.count {
            "🎁 可领取"
        } else {
            "⏳ 未达成"
        };
        out.push_str(&format!(
            "  {} {}件: {} (+{}金/+{}钻",
            status, ms.count, ms.name, ms.reward_gold, ms.reward_diamond
        ));
        if !ms.reward_item.is_empty() {
            out.push_str(&format!("/{}", ms.reward_item));
        }
        out.push_str(")\n");
    }

    out.push_str("\n💡 收集提示:");
    out.push_str("\n  📦 物品图鉴 — 查看收集图鉴+物品");
    out.push_str("\n  👹 怪物图鉴 — 查看收集图鉴+怪物");
    out.push_str("\n  🎁 领取奖励 — 查看收集图鉴+领取");

    out
}

/// 领取收集里程碑奖励
fn claim_collection_milestone(db: &Database, user_id: &str, total_count: u32) -> String {
    let claimed_str = db.read_user_data(user_id, "collection_milestones");
    let mut claimed_set: std::collections::HashSet<u32> =
        claimed_str.split(',').filter_map(|s| s.parse::<u32>().ok()).collect();

    let mut rewards = Vec::new();

    for ms in milestones() {
        if total_count >= ms.count && !claimed_set.contains(&ms.count) {
            claimed_set.insert(ms.count);

            // 发放金币奖励
            if ms.reward_gold > 0 {
                let cur_gold = db.read_user_data(user_id, "Currency_gold").parse::<i64>().unwrap_or(0);
                db.write_user_data(user_id, "Currency_gold", &(cur_gold + ms.reward_gold).to_string());
            }

            // 发放钻石奖励
            if ms.reward_diamond > 0 {
                let cur_diamond = db
                    .read_user_data(user_id, "Currency_diamond")
                    .parse::<i64>()
                    .unwrap_or(0);
                db.write_user_data(
                    user_id,
                    "Currency_diamond",
                    &(cur_diamond + ms.reward_diamond).to_string(),
                );
            }

            // 发放物品奖励
            if !ms.reward_item.is_empty() {
                for part in ms.reward_item.split('+') {
                    let part = part.trim();
                    if let Some((name, count_str)) = parse_item_count(part) {
                        if let Ok(count) = count_str.parse::<i32>() {
                            db.add_item(user_id, name, count);
                        }
                    }
                }
            }

            let reward_desc = if ms.reward_item.is_empty() {
                String::new()
            } else {
                format!("/{}", ms.reward_item)
            };
            rewards.push(format!(
                "  🏆 {} ({}件) — {}金/{}钻{}",
                ms.name, ms.count, ms.reward_gold, ms.reward_diamond, reward_desc
            ));
        }
    }

    // 保存已领取状态
    let new_claimed: Vec<String> = claimed_set.iter().map(|c| c.to_string()).collect();
    db.write_user_data(user_id, "collection_milestones", &new_claimed.join(","));

    if rewards.is_empty() {
        return "暂无可领取的里程碑奖励！\n继续收集更多物品和怪物吧~".to_string();
    }

    let mut out = String::from("🎁 收集里程碑奖励领取成功！\n\n");
    for r in &rewards {
        out.push_str(r);
        out.push('\n');
    }
    out
}

/// 解析 "物品名*数量" 格式
fn parse_item_count(s: &str) -> Option<(&str, &str)> {
    if let Some(pos) = s.rfind('*') {
        let name = &s[..pos];
        let count = &s[pos + 1..];
        if !name.is_empty() && !count.is_empty() {
            return Some((name, count));
        }
    }
    None
}

/// 收集统计 - 快速查看收集进度
pub fn cmd_collection_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看收集统计！".to_string();
    }

    let items_str = db.read_user_data(user_id, "collected_items");
    let items: Vec<&str> = items_str.split(',').filter(|s| !s.is_empty()).collect();
    let monsters_str = db.read_user_data(user_id, "collected_monsters");
    let monsters: Vec<&str> = monsters_str.split(',').filter(|s| !s.is_empty()).collect();

    let total = items.len() + monsters.len();
    let claimed_str = db.read_user_data(user_id, "collection_milestones");
    let claimed_count = claimed_str.split(',').filter(|s| !s.is_empty()).count();

    let next_milestone = milestones()
        .iter()
        .find(|ms| (total as u32) < ms.count)
        .map(|ms| format!("下个里程碑: {}件 ({})", ms.count, ms.name));

    format!(
        "📊 收集统计\n{}\n  📦 物品: {}/345\n  👹 怪物: {}/32\n  📈 总计: {}/377\n  🏆 已领里程碑: {}个\n  {}",
        "─".repeat(20),
        items.len(),
        monsters.len(),
        total,
        claimed_count,
        next_milestone.unwrap_or_else(|| "🎉 全部里程碑已达成！".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milestones_count() {
        assert_eq!(milestones().len(), 7);
    }

    #[test]
    fn test_milestones_sorted_by_count() {
        let ms = milestones();
        for i in 1..ms.len() {
            assert!(ms[i].count > ms[i - 1].count, "Milestones should be sorted by count");
        }
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for ms in milestones() {
            assert!(
                ms.reward_gold > 0 || ms.reward_diamond > 0,
                "Milestone '{}' should have rewards",
                ms.name
            );
        }
    }

    #[test]
    fn test_milestone_names_unique() {
        let mut names: Vec<&str> = milestones().iter().map(|m| m.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(before, names.len());
    }

    #[test]
    fn test_parse_item_count_basic() {
        let r = parse_item_count("强化石*3");
        assert_eq!(r, Some(("强化石", "3")));
    }

    #[test]
    fn test_parse_item_count_no_star() {
        assert!(parse_item_count("强化石").is_none());
    }

    #[test]
    fn test_parse_item_count_empty() {
        assert!(parse_item_count("").is_none());
    }

    #[test]
    fn test_parse_item_count_empty_name() {
        assert!(parse_item_count("*3").is_none());
    }

    #[test]
    fn test_parse_item_count_empty_count() {
        assert!(parse_item_count("强化石*").is_none());
    }

    #[test]
    fn test_parse_item_count_chinese_item() {
        let r = parse_item_count("初级礼包*1");
        assert_eq!(r, Some(("初级礼包", "1")));
    }

    #[test]
    fn test_parse_item_count_multiple_stars() {
        // rsplitn(2, '*') means last star is the separator
        let r = parse_item_count("特殊*物品*5");
        assert_eq!(r, Some(("特殊*物品", "5")));
    }

    #[test]
    fn test_milestone_count_range() {
        let ms = milestones();
        assert_eq!(ms[0].count, 10);
        assert_eq!(ms.last().unwrap().count, 300);
    }

    #[test]
    fn test_milestone_gold_increases() {
        let ms = milestones();
        for i in 1..ms.len() {
            assert!(
                ms[i].reward_gold >= ms[i - 1].reward_gold,
                "Gold reward should increase"
            );
        }
    }
}

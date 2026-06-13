/// CakeGame 活跃积分兑换系统
/// 使用活跃积分兑换各种道具奖励
use crate::db::Database;
use crate::user;

/// 兑换物品定义
struct ExchangeItem {
    name: &'static str,
    cost: i32,
    emoji: &'static str,
    desc: &'static str,
}

const EXCHANGE_ITEMS: &[ExchangeItem] = &[
    ExchangeItem {
        name: "生命药水",
        cost: 50,
        emoji: "🧪",
        desc: "恢复生命值的药水",
    },
    ExchangeItem {
        name: "强化石",
        cost: 100,
        emoji: "💎",
        desc: "用于装备强化",
    },
    ExchangeItem {
        name: "初级礼包",
        cost: 200,
        emoji: "🎁",
        desc: "包含多种基础道具",
    },
    ExchangeItem {
        name: "紫色精粹",
        cost: 300,
        emoji: "🔮",
        desc: "高级精粹材料",
    },
    ExchangeItem {
        name: "史诗碎片",
        cost: 500,
        emoji: "⭐",
        desc: "稀有史诗级材料",
    },
];

/// 读取活跃积分
fn read_activity_points(db: &Database, user_id: &str) -> i32 {
    db.read_user_data(user_id, "ActivityPoints").parse().unwrap_or(0)
}

/// 写入活跃积分
fn write_activity_points(db: &Database, user_id: &str, points: i32) {
    let _ = db.write_user_data(user_id, "ActivityPoints", &points.to_string());
}

/// 活跃积分兑换 - 列出兑换物品或执行兑换
pub fn cmd_activity_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current_points = read_activity_points(db, user_id);

    let item_name = args.trim();

    if item_name.is_empty() {
        // 列出所有兑换物品
        let mut out = format!("{}\n", prefix);
        out += "🏪 ═══ 活跃积分兑换 ═══\n";
        out += &format!("🎯 当前活跃积分: {} 分\n\n", current_points);
        out += "━━━ 可兑换物品 ━━━\n";

        for (i, item) in EXCHANGE_ITEMS.iter().enumerate() {
            let status = if current_points >= item.cost {
                "✅可兑换"
            } else {
                "🔒积分不足"
            };
            out += &format!(
                "{} {}. [{}] - {}积分 ({})\n   {}\n",
                item.emoji,
                i + 1,
                item.name,
                item.cost,
                status,
                item.desc
            );
        }

        out += "\n📌 发送「活跃兑换+物品名」进行兑换\n";
        out += "💡 活跃积分通过完成日常活动获得\n";
        return out;
    }

    // 查找兑换物品
    let target = match EXCHANGE_ITEMS.iter().find(|item| item.name == item_name) {
        Some(item) => item,
        None => {
            let names: Vec<&str> = EXCHANGE_ITEMS.iter().map(|i| i.name).collect();
            return format!(
                "{}\n❌ 未找到兑换物品 [{}]\n可兑换: {}",
                prefix,
                item_name,
                names.join("/")
            );
        }
    };

    // 检查积分是否足够
    if current_points < target.cost {
        return format!(
            "{}\n❌ 活跃积分不足！\n当前积分: {}\n需要积分: {}\n还差: {} 分",
            prefix,
            current_points,
            target.cost,
            target.cost - current_points
        );
    }

    // 扣除积分并发放物品
    let new_points = current_points - target.cost;
    write_activity_points(db, user_id, new_points);
    db.add_item(user_id, target.name, 1);

    let mut out = format!("{}\n", prefix);
    out += &format!("{} 🎉 兑换成功！\n", target.emoji);
    out += "━━━ 兑换详情 ━━━\n";
    out += &format!("🎁 获得: [{}]\n", target.name);
    out += &format!("💰 消耗: {} 活跃积分\n", target.cost);
    out += &format!("📊 剩余积分: {} 分\n", new_points);
    out += "\n继续完成活动获取更多积分吧！\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exchange_items_count() {
        assert_eq!(EXCHANGE_ITEMS.len(), 5);
    }

    #[test]
    fn test_exchange_items_unique_names() {
        let mut names: Vec<&str> = EXCHANGE_ITEMS.iter().map(|i| i.name).collect();
        let before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), before, "Exchange item names must be unique");
    }

    #[test]
    fn test_exchange_items_positive_costs() {
        for item in EXCHANGE_ITEMS {
            assert!(item.cost > 0, "{} cost must be positive", item.name);
        }
    }

    #[test]
    fn test_exchange_items_cost_ordering() {
        // Costs should generally increase (higher tier = more expensive)
        for i in 1..EXCHANGE_ITEMS.len() {
            assert!(
                EXCHANGE_ITEMS[i].cost >= EXCHANGE_ITEMS[i - 1].cost,
                "Costs should be non-decreasing: {} ({}) > {} ({})",
                EXCHANGE_ITEMS[i - 1].name,
                EXCHANGE_ITEMS[i - 1].cost,
                EXCHANGE_ITEMS[i].name,
                EXCHANGE_ITEMS[i].cost
            );
        }
    }

    #[test]
    fn test_exchange_items_emoji_non_empty() {
        for item in EXCHANGE_ITEMS {
            assert!(!item.emoji.is_empty(), "{} must have an emoji", item.name);
        }
    }

    #[test]
    fn test_exchange_items_desc_non_empty() {
        for item in EXCHANGE_ITEMS {
            assert!(!item.desc.is_empty(), "{} must have a description", item.name);
        }
    }
}

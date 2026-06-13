/// CakeGame 属性重置系统 (Attribute Respec System)
///
/// 允许玩家重置通过训练（吐纳/冥想/练武/习法）和自动修炼获得的永久属性加成
/// 重置需要消耗钻石，消耗量根据重置的属性总量递增
///
/// 指令: 属性重置, 确认重置, 重置记录
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 可重置的属性配置
struct RespecTarget {
    name: &'static str,
    attr_name: &'static str,
    bonus_key: &'static str,
}

const RESPEC_TARGETS: &[RespecTarget] = &[
    RespecTarget {
        name: "吐纳",
        attr_name: "生命",
        bonus_key: "training_hp_bonus",
    },
    RespecTarget {
        name: "冥想",
        attr_name: "魔法",
        bonus_key: "training_mp_bonus",
    },
    RespecTarget {
        name: "练武",
        attr_name: "物攻",
        bonus_key: "training_ad_bonus",
    },
    RespecTarget {
        name: "习法",
        attr_name: "魔攻",
        bonus_key: "training_ap_bonus",
    },
];

/// 计算重置钻石消耗
/// 每点属性加成消耗 2 钻石，最低 50 钻石
fn calc_respec_cost(total_bonus: i32) -> i64 {
    if total_bonus <= 0 {
        return 0;
    }
    let cost = (total_bonus as i64) * 2;
    cost.max(50)
}

/// 查看属性重置信息
pub fn cmd_view_respec(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再使用属性重置功能！", prefix);
    }

    let mut result = format!(
        "{}\n╔═══════════════════════╗\n║  🔄 属性重置系统  ║\n╚═══════════════════════╝\n",
        prefix
    );

    let mut total_bonus = 0i32;
    let mut has_any = false;

    result.push_str("\n📊 当前训练加成:\n");

    for target in RESPEC_TARGETS {
        let bonus: i32 = db.read_user_data(user_id, target.bonus_key).parse().unwrap_or(0);

        if bonus > 0 {
            has_any = true;
            result.push_str(&format!("  · {}({}) +{}\n", target.name, target.attr_name, bonus));
            total_bonus += bonus;
        }
    }

    if !has_any {
        result.push_str("  暂无训练加成，无需重置。\n");
        result.push_str("\n💡 通过「吐纳」「冥想」「练武」「习法」或「开始修炼」获得属性加成");
        return result;
    }

    let cost = calc_respec_cost(total_bonus);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);

    result.push_str(&format!(
        "\n━━━━━━━━━━━━━━━━━━━━\n\
         📈 属性加成总计: +{}\n\
         💎 重置费用: {} 钻石\n\
         💎 当前钻石: {}\n\
         {}\n\
         \n\
         ⚠️ 重置后所有训练加成归零，此操作不可逆！\n\
         \n\
         💡 使用「确认重置」执行重置操作",
        total_bonus,
        cost,
        diamond,
        if diamond >= cost {
            "✅ 钻石充足"
        } else {
            "❌ 钻石不足"
        }
    ));

    result
}

/// 确认属性重置
pub fn cmd_confirm_respec(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再使用！", prefix);
    }

    // 计算总加成
    let mut total_bonus = 0i32;
    let mut bonuses = Vec::new();

    for target in RESPEC_TARGETS {
        let bonus: i32 = db.read_user_data(user_id, target.bonus_key).parse().unwrap_or(0);
        if bonus > 0 {
            total_bonus += bonus;
            bonuses.push((target.bonus_key, target.attr_name, bonus));
        }
    }

    if total_bonus <= 0 {
        return format!("{}\n暂无训练加成，无需重置。", prefix);
    }

    let cost = calc_respec_cost(total_bonus);
    let diamond: i64 = db.read_currency(user_id, CURRENCY_DIAMOND);

    if diamond < cost {
        return format!("{}\n💎 钻石不足！需要 {}，当前 {}。", prefix, cost, diamond);
    }

    // 检查是否有修炼进行中
    let auto_evo_target = db.read_user_data(user_id, "auto_evo_target");
    if !auto_evo_target.is_empty() {
        return format!("{}\n❌ 你正在修炼中，请先发送「停止修炼」再进行属性重置。", prefix);
    }

    // 扣除钻石
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, cost);

    // 重置所有训练加成
    let mut reset_details = String::new();
    for (key, name, bonus) in &bonuses {
        db.write_user_data(user_id, key, "0");
        reset_details.push_str(&format!("  {} -{}\n", name, bonus));
    }

    // 记录重置次数
    let respec_count: i32 = db.read_user_data(user_id, "respec_count").parse().unwrap_or(0);
    db.write_user_data(user_id, "respec_count", &(respec_count + 1).to_string());

    // 记录最后重置时间
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.write_user_data(user_id, "respec_last_time", &now);

    format!(
        "{}\n╔═══════════════════════╗\n║  ✅ 属性重置成功！  ║\n╚═══════════════════════╝\n\
         \n\
         已重置属性:\n\
         {}\
         \n\
         💎 消耗钻石: {}\n\
         📊 累计重置: {} 次\n\
         \n\
         💡 你可以重新通过「吐纳」「冥想」「练武」「习法」来分配属性加成",
        prefix,
        reset_details,
        cost,
        respec_count + 1
    )
}

/// 查看重置记录
pub fn cmd_respec_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let respec_count: i32 = db.read_user_data(user_id, "respec_count").parse().unwrap_or(0);
    let last_time = db.read_user_data(user_id, "respec_last_time");

    let mut result = format!(
        "{}\n═══ 重置记录 ═══\n\
         累计重置次数: {} 次\n",
        prefix, respec_count
    );

    if respec_count > 0 && !last_time.is_empty() {
        result.push_str(&format!("最后重置时间: {}\n", last_time));
    }

    // 显示当前训练状态
    result.push_str("\n📊 当前训练加成:\n");
    let mut total = 0i32;
    for target in RESPEC_TARGETS {
        let bonus: i32 = db.read_user_data(user_id, target.bonus_key).parse().unwrap_or(0);
        result.push_str(&format!("  {}({}): +{}\n", target.name, target.attr_name, bonus));
        total += bonus;
    }
    result.push_str(&format!("  ━━━━━━━\n  总计: +{}\n", total));

    let cost = if total > 0 { calc_respec_cost(total) } else { 0 };
    result.push_str(&format!("\n💡 下次重置费用: {} 钻石", cost));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_respec_cost_calculation() {
        // 0 bonus -> 0 cost
        assert_eq!(calc_respec_cost(0), 0);
        // 10 bonus -> 20 diamonds, but min is 50
        assert_eq!(calc_respec_cost(10), 50);
        // 30 bonus -> 60 diamonds
        assert_eq!(calc_respec_cost(30), 60);
        // 100 bonus -> 200 diamonds
        assert_eq!(calc_respec_cost(100), 200);
        // Negative bonus -> 0 cost
        assert_eq!(calc_respec_cost(-5), 0);
    }
}

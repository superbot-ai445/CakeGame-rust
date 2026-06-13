/// CakeGame 签到补签系统
/// 允许玩家使用钻石补签本月错过的签到日期
///
/// 指令: 补签列表, 补签+日期
/// 数据存储: sign_cal_YYYY-MM (已有签到日历字段)
///          sign_makeup_count_YYYY-MM (本月补签次数)
///
/// 补签规则:
///   - 仅可补签当月已过去的日期
///   - 每次补签消耗钻石，费用随本月补签次数递增
///   - 补签计入连续签到天数和累计签到天数
///   - 补签不触发里程碑奖励（仅普通签到奖励）
///   - 每月最多补签5次
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 补签基础费用（钻石）
const MAKEUP_BASE_COST: i32 = 10;
/// 补签费用递增（每次+5钻石）
const MAKEUP_COST_INCREMENT: i32 = 5;
/// 每月最大补签次数
const MAKEUP_MAX_PER_MONTH: i32 = 5;

/// 计算补签费用
fn calc_makeup_cost(used_count: i32) -> i32 {
    MAKEUP_BASE_COST + used_count * MAKEUP_COST_INCREMENT
}

/// 获取当月已签到的日期集合
fn get_signed_days(db: &Database, user_id: &str, month: &str) -> std::collections::HashSet<i32> {
    let cal_key = format!("sign_cal_{}", month);
    let cal_data = db.read_user_data(user_id, &cal_key);
    if cal_data.is_empty() {
        std::collections::HashSet::new()
    } else {
        cal_data
            .split(',')
            .filter_map(|d| d.trim().parse::<i32>().ok())
            .collect()
    }
}

/// 获取本月补签次数
fn get_makeup_count(db: &Database, user_id: &str, month: &str) -> i32 {
    let key = format!("sign_makeup_count_{}", month);
    db.read_user_data(user_id, &key).parse().unwrap_or(0)
}

/// 增加本月补签次数
fn increment_makeup_count(db: &Database, user_id: &str, month: &str) {
    let count = get_makeup_count(db, user_id, month);
    let key = format!("sign_makeup_count_{}", month);
    db.write_user_data(user_id, &key, &(count + 1).to_string());
}

/// 获取本月天数
fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// 补签列表 — 显示可补签的日期和费用
pub fn cmd_makeup_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let now = Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_month = now.format("%Y-%m").to_string();
    let today_day: i32 = today[8..10].parse().unwrap_or(1);
    let year: i32 = current_month[..4].parse().unwrap_or(2026);
    let month: i32 = current_month[5..7].parse().unwrap_or(1);

    let signed_days = get_signed_days(db, user_id, &current_month);
    let makeup_count = get_makeup_count(db, user_id, &current_month);

    // 找出可补签的日期（今天之前且未签到的）
    let mut missed: Vec<i32> = (1..today_day).filter(|d| !signed_days.contains(d)).collect();

    let mut result = format!("{}\n═══ 📅 补签系统 ═══", prefix);
    result.push_str(&format!("\n📅 当前月份: {}年{}月", year, month));
    result.push_str(&format!("\n📊 本月补签: {}/{}次", makeup_count, MAKEUP_MAX_PER_MONTH));
    result.push_str(&format!(
        "\n💰 当前钻石: 💎{}",
        db.read_currency(user_id, CURRENCY_DIAMOND)
    ));

    if makeup_count >= MAKEUP_MAX_PER_MONTH {
        result.push_str("\n\n❌ 本月补签次数已用完（5/5）");
        result.push_str("\n💡 每月最多补签5次，下月重置");
        return result;
    }

    if missed.is_empty() {
        result.push_str("\n\n✅ 本月没有漏签日期！继续保持！");
        return result;
    }

    let next_cost = calc_makeup_cost(makeup_count);
    result.push_str(&format!("\n\n📋 可补签日期 (共{}天):", missed.len()));
    result.push_str(&format!("\n💡 下次补签费用: 💎{}钻石", next_cost));
    result.push('\n');

    // 每行显示5天
    missed.sort();
    for (i, &day) in missed.iter().enumerate() {
        if i % 5 == 0 {
            result.push_str("\n  ");
        }
        result.push_str(&format!(" {}日", day));
    }

    result.push_str("\n\n📝 使用方法: 发送 '补签+日期' (如: 补签+5)");
    result.push_str("\n💡 补签计入连续签到天数，但不触发里程碑奖励");
    result
}

/// 补签 — 补签指定日期
pub fn cmd_makeup_sign(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let now = Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let current_month = now.format("%Y-%m").to_string();
    let today_day: i32 = today[8..10].parse().unwrap_or(1);
    let year: i32 = current_month[..4].parse().unwrap_or(2026);
    let month: i32 = current_month[5..7].parse().unwrap_or(1);

    // 解析日期参数
    let day: i32 = match args.trim().parse() {
        Ok(d) => d,
        Err(_) => {
            return format!("{}\n❌ 请输入有效的日期数字！\n📝 例如: 补签+5", prefix);
        }
    };

    // 验证日期范围
    if day < 1 || day > days_in_month(year, month) {
        return format!(
            "\n❌ 无效日期！{}年{}月的有效日期为 1~{}日",
            year,
            month,
            days_in_month(year, month)
        );
    }

    // 不能补签今天或未来的日期
    if day >= today_day {
        return format!("{}\n❌ 只能补签已过去的日期！今天是{}日", prefix, today_day);
    }

    // 检查是否已经签到
    let signed_days = get_signed_days(db, user_id, &current_month);
    if signed_days.contains(&day) {
        return format!("{}\n❌ {}月{}日已经签到了，无需补签！", prefix, month, day);
    }

    // 检查补签次数
    let makeup_count = get_makeup_count(db, user_id, &current_month);
    if makeup_count >= MAKEUP_MAX_PER_MONTH {
        return format!(
            "{}\n❌ 本月补签次数已用完 ({}/{})！",
            prefix, makeup_count, MAKEUP_MAX_PER_MONTH
        );
    }

    // 计算费用
    let cost = calc_makeup_cost(makeup_count);
    let diamonds = db.read_currency(user_id, CURRENCY_DIAMOND);

    if diamonds < cost as i64 {
        return format!(
            "{}\n❌ 钻石不足！补签需要💎{}钻石，当前仅有💎{}",
            prefix, cost, diamonds
        );
    }

    // 扣除钻石
    db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, cost as i64);

    // 记录签到到日历
    let cal_key = format!("sign_cal_{}", current_month);
    let mut cal = db.read_user_data(user_id, &cal_key);
    let day_str = format!("{}", day);
    if !cal.contains(&day_str) {
        if cal.is_empty() {
            cal = day_str;
        } else {
            cal = format!("{},{}", cal, day_str);
        }
        db.write_user_data(user_id, &cal_key, &cal);
    }

    // 增加补签次数
    increment_makeup_count(db, user_id, &current_month);

    // 累计签到天数
    let total: i32 = db.read_user_data(user_id, "sign_in_total").parse().unwrap_or(0);
    db.write_user_data(user_id, "sign_in_total", &(total + 1).to_string());

    // 重新计算连续签到天数（补签后可能修复断裂的连续签到）
    let updated_signed = get_signed_days(db, user_id, &current_month);
    let mut streak = 0;
    // 从今天往前数连续签到天数（不包括今天如果今天没签到）
    let today_signed = updated_signed.contains(&today_day);
    if today_signed {
        for d in (1..=today_day).rev() {
            if updated_signed.contains(&d) {
                streak += 1;
            } else {
                break;
            }
        }
    } else {
        for d in (1..today_day).rev() {
            if updated_signed.contains(&d) {
                streak += 1;
            } else {
                break;
            }
        }
    }
    // 更新连续签到天数（只增不减）
    let old_streak: i32 = db.read_user_data(user_id, "sign_in_sustain").parse().unwrap_or(0);
    if streak > old_streak {
        db.write_user_data(user_id, "sign_in_sustain", &streak.to_string());
    }

    // 补签奖励（比正常签到少，不给里程碑）
    let gold_reward: i64 = 50;
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_reward);

    // 活跃度追踪
    let _ = crate::activity::add_activity(db, user_id, "sign_in");

    let new_count = makeup_count + 1;
    let mut result = format!("{}\n✅ 补签成功！{}月{}日已补签", prefix, month, day);
    result.push_str(&format!("\n💎 消耗钻石: {}", cost));
    result.push_str(&format!("\n💰 获得金币: {}", gold_reward));
    result.push_str(&format!("\n📊 累计签到: {}天", total + 1));
    result.push_str(&format!("\n📅 连续签到: {}天", streak.max(old_streak)));
    result.push_str(&format!("\n🔄 本月补签: {}/{}次", new_count, MAKEUP_MAX_PER_MONTH));

    if new_count >= MAKEUP_MAX_PER_MONTH {
        result.push_str("\n⚠️ 本月补签次数已用完！");
    } else {
        let next_cost = calc_makeup_cost(new_count);
        result.push_str(&format!("\n💡 下次补签费用: 💎{}钻石", next_cost));
    }

    result.push_str("\n\n💡 发送 '签到' 进行今日签到");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_makeup_cost() {
        assert_eq!(calc_makeup_cost(0), 10); // 第1次: 10钻石
        assert_eq!(calc_makeup_cost(1), 15); // 第2次: 15钻石
        assert_eq!(calc_makeup_cost(2), 20); // 第3次: 20钻石
        assert_eq!(calc_makeup_cost(3), 25); // 第4次: 25钻石
        assert_eq!(calc_makeup_cost(4), 30); // 第5次: 30钻石
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29); // leap year
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn test_days_in_month_all_months() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 3), 31);
        assert_eq!(days_in_month(2026, 5), 31);
        assert_eq!(days_in_month(2026, 7), 31);
        assert_eq!(days_in_month(2026, 8), 31);
        assert_eq!(days_in_month(2026, 10), 31);
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 6), 30);
        assert_eq!(days_in_month(2026, 9), 30);
        assert_eq!(days_in_month(2026, 11), 30);
    }

    #[test]
    fn test_leap_year_rules() {
        assert_eq!(days_in_month(2000, 2), 29); // divisible by 400 → leap
        assert_eq!(days_in_month(1900, 2), 28); // divisible by 100 but not 400 → not leap
        assert_eq!(days_in_month(2024, 2), 29); // divisible by 4 but not 100 → leap
        assert_eq!(days_in_month(2023, 2), 28); // not divisible by 4 → not leap
    }

    #[test]
    fn test_makeup_cost_escalation() {
        // 确保费用递增
        for i in 0..5 {
            assert!(calc_makeup_cost(i + 1) > calc_makeup_cost(i));
        }
        // 总补签成本 (10+15+20+25+30 = 100钻石)
        let total: i32 = (0..5).map(calc_makeup_cost).sum();
        assert_eq!(total, 100);
    }

    #[test]
    fn test_makeup_constants() {
        assert_eq!(MAKEUP_BASE_COST, 10);
        assert_eq!(MAKEUP_COST_INCREMENT, 5);
        assert_eq!(MAKEUP_MAX_PER_MONTH, 5);
    }
}

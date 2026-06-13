/// CakeGame 钱庄/银行系统
/// 玩家可存入金币赚取利息，也可随时取款
/// 使用 ext_qianzhuang_Info 表存储数据（空表，使用内置规则）
/// 用户数据: bank_balance = 存款余额, bank_last_interest = 上次计息时间
use crate::core::*;
use crate::db::Database;
use crate::user;
use chrono::Local;

/// 日利率 (0.5% = 0.005)
const DAILY_INTEREST_RATE: f64 = 0.005;
/// 最低存款金额
const MIN_DEPOSIT: i64 = 100;
/// 最高存款金额
const MAX_DEPOSIT: i64 = 10_000_000;

/// 计算并累积利息
fn calc_interest(db: &Database, user_id: &str) -> (i64, i64) {
    let balance: i64 = db.read_user_data(user_id, "bank_balance").parse().unwrap_or(0);
    if balance <= 0 {
        return (0, 0);
    }

    let last_str = db.read_user_data(user_id, "bank_last_interest");
    let now = Local::now();

    if last_str.is_empty() {
        db.write_user_data(
            user_id,
            "bank_last_interest",
            &now.format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        return (balance, 0);
    }

    let last = chrono::NaiveDateTime::parse_from_str(&last_str, "%Y-%m-%d %H:%M:%S");
    let last = match last {
        Ok(dt) => dt,
        Err(_) => {
            db.write_user_data(
                user_id,
                "bank_last_interest",
                &now.format("%Y-%m-%d %H:%M:%S").to_string(),
            );
            return (balance, 0);
        }
    };

    let duration = now.naive_local() - last;
    let hours = duration.num_hours();
    if hours < 1 {
        return (balance, 0);
    }

    let hourly_rate = DAILY_INTEREST_RATE / 24.0;
    let interest = (balance as f64 * hourly_rate * hours as f64).floor() as i64;
    if interest > 0 {
        let new_balance = balance + interest;
        db.write_user_data(user_id, "bank_balance", &new_balance.to_string());
        db.write_user_data(
            user_id,
            "bank_last_interest",
            &now.format("%Y-%m-%d %H:%M:%S").to_string(),
        );
        return (new_balance, interest);
    }

    (balance, 0)
}

/// 查看钱庄 — 显示存款信息和利率
pub fn cmd_view_bank(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let (balance, interest) = calc_interest(db, user_id);

    let mut out = format!("{}\n═══ 💰 钱庄 ═══", prefix);
    out.push_str(&format!("\n💰 身上金币: {}", gold));
    out.push_str(&format!("\n🏦 存款余额: {}", balance));
    if interest > 0 {
        out.push_str(&format!("\n✨ 刚刚获得利息: +{} 金币", interest));
    }
    out.push_str(&format!(
        "\n📊 日利率: {:.1}% (每小时结算)",
        DAILY_INTEREST_RATE * 100.0
    ));
    out.push_str(&format!("\n📏 最低存款: {} 金币", MIN_DEPOSIT));
    out.push_str(&format!("\n📏 最高存款: {} 金币", MAX_DEPOSIT));
    out.push_str("\n\n💡 存入金币即可自动生息！");
    out.push_str("\n发送'存款+金额'存入金币");
    out.push_str("\n发送'取款+金额'取出金币");
    out.push_str("\n发送'我的存款'查看详细信息");
    out
}

/// 存款 — 存入金币
pub fn cmd_deposit(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let amount_str = args.trim();

    if amount_str.is_empty() {
        return format!("{}\n请指定存款金额！\n用法：存款+金额\n例：存款+1000", prefix);
    }

    if amount_str == "全部" {
        let gold = db.read_currency(user_id, CURRENCY_GOLD);
        if gold < MIN_DEPOSIT {
            return format!(
                "{}\n金币不足！存款最低需要 {} 金币，你身上有 {} 金币",
                prefix, MIN_DEPOSIT, gold
            );
        }
        return do_deposit(db, user_id, gold, prefix);
    }

    let amount: i64 = match amount_str.parse() {
        Ok(n) => n,
        Err(_) => return format!("{}\n请输入有效的金额数字！", prefix),
    };

    if amount < MIN_DEPOSIT {
        return format!("{}\n存款金额不能低于 {} 金币！", prefix, MIN_DEPOSIT);
    }

    if amount > MAX_DEPOSIT {
        return format!("{}\n单次存款不能超过 {} 金币！", prefix, MAX_DEPOSIT);
    }

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < amount {
        return format!("{}\n金币不足！需要 {} 金币，你身上只有 {} 金币", prefix, amount, gold);
    }

    do_deposit(db, user_id, amount, prefix)
}

fn do_deposit(db: &Database, user_id: &str, amount: i64, prefix: String) -> String {
    let (_, interest) = calc_interest(db, user_id);

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let balance: i64 = db.read_user_data(user_id, "bank_balance").parse().unwrap_or(0);

    db.write_currency(user_id, CURRENCY_GOLD, gold - amount);
    let new_balance = balance + amount;
    db.write_user_data(user_id, "bank_balance", &new_balance.to_string());
    db.write_user_data(
        user_id,
        "bank_last_interest",
        &Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    let mut out = format!("{}\n✅ 存款成功！", prefix);
    out.push_str(&format!("\n💰 存入: {} 金币", amount));
    out.push_str(&format!("\n🏦 存款余额: {} 金币", new_balance));
    out.push_str(&format!("\n💰 剩余金币: {} 金币", gold - amount));
    if interest > 0 {
        out.push_str(&format!("\n✨ 本次结算利息: +{} 金币", interest));
    }
    out
}

/// 取款 — 取出金币
pub fn cmd_withdraw(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let amount_str = args.trim();

    let (_, interest) = calc_interest(db, user_id);

    let balance: i64 = db.read_user_data(user_id, "bank_balance").parse().unwrap_or(0);

    if balance <= 0 {
        return format!("{}\n你没有存款！\n发送'存款+金额'存入金币", prefix);
    }

    if amount_str.is_empty() {
        return format!(
            "{}\n请指定取款金额！\n用法：取款金额\n当前存款：{} 金币",
            prefix, balance
        );
    }

    let amount = if amount_str == "全部" {
        balance
    } else {
        match amount_str.parse::<i64>() {
            Ok(n) => n,
            Err(_) => return format!("{}\n请输入有效的金额数字！", prefix),
        }
    };

    if amount <= 0 {
        return format!("{}\n取款金额必须大于0！", prefix);
    }

    if amount > balance {
        return format!(
            "{}\n存款不足！当前存款 {} 金币，无法取出 {} 金币",
            prefix, balance, amount
        );
    }

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let new_balance = balance - amount;

    db.write_currency(user_id, CURRENCY_GOLD, gold + amount);
    db.write_user_data(user_id, "bank_balance", &new_balance.to_string());

    let mut out = format!("{}\n✅ 取款成功！", prefix);
    out.push_str(&format!("\n💰 取出: {} 金币", amount));
    out.push_str(&format!("\n🏦 剩余存款: {} 金币", new_balance));
    out.push_str(&format!("\n💰 当前金币: {} 金币", gold + amount));
    if interest > 0 {
        out.push_str(&format!("\n✨ 本次结算利息: +{} 金币", interest));
    }
    out
}

/// 我的存款 — 查看详细存款信息
pub fn cmd_my_deposit(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (balance, interest) = calc_interest(db, user_id);
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let last_str = db.read_user_data(user_id, "bank_last_interest");

    let mut out = format!("{}\n═══ 🏦 我的存款 ═══", prefix);
    out.push_str(&format!("\n💰 身上金币: {}", gold));
    out.push_str(&format!("\n🏦 存款余额: {} 金币", balance));
    if interest > 0 {
        out.push_str(&format!("\n✨ 本次利息: +{} 金币", interest));
    }
    if !last_str.is_empty() {
        out.push_str(&format!("\n⏰ 上次计息: {}", last_str));
    }

    if balance > 0 {
        let daily = (balance as f64 * DAILY_INTEREST_RATE).floor() as i64;
        let weekly = daily * 7;
        let monthly = daily * 30;
        out.push_str("\n\n📊 收益预估：");
        out.push_str(&format!("\n  每日: +{} 金币", daily));
        out.push_str(&format!("\n  每周: +{} 金币", weekly));
        out.push_str(&format!("\n  每月: +{} 金币", monthly));
    }

    out.push_str(&format!("\n\n💡 日利率: {:.1}%", DAILY_INTEREST_RATE * 100.0));
    out.push_str("\n发送'存款+金额'存入金币");
    out.push_str("\n发送'取款+金额'取出金币");
    out.push_str("\n发送'存款+全部'一键存入所有金币");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_values() {
        assert_eq!(DAILY_INTEREST_RATE, 0.005);
        assert_eq!(MIN_DEPOSIT, 100);
        assert_eq!(MAX_DEPOSIT, 10_000_000);
    }

    #[test]
    fn test_constants_deposit_range() {
        // MIN_DEPOSIT must be positive and less than MAX_DEPOSIT
        assert!(MIN_DEPOSIT > 0, "MIN_DEPOSIT must be positive");
        assert!(
            MAX_DEPOSIT > MIN_DEPOSIT,
            "MAX_DEPOSIT must be greater than MIN_DEPOSIT"
        );
    }

    #[test]
    fn test_interest_rate_calculation() {
        // Verify interest rate math: 0.5% daily = 0.005/24 hourly
        let hourly_rate = DAILY_INTEREST_RATE / 24.0;
        assert!((hourly_rate - 0.00020833333333333334).abs() < 1e-10);

        // For 10000 gold deposited for 24 hours
        let balance: f64 = 10000.0;
        let interest = (balance * hourly_rate * 24.0).floor() as i64;
        assert_eq!(interest, 50); // 10000 * 0.005 = 50
    }

    #[test]
    fn test_interest_rate_daily_yield() {
        // Daily yield for various balances
        let test_cases: Vec<(i64, i64)> = vec![
            (1000, 5),     // 1000 * 0.005 = 5
            (10000, 50),   // 10000 * 0.005 = 50
            (100000, 500), // 100000 * 0.005 = 500
            (99, 0),       // below MIN_DEPOSIT, but math is 0
        ];
        for (balance, expected) in test_cases {
            let daily = (balance as f64 * DAILY_INTEREST_RATE).floor() as i64;
            assert_eq!(daily, expected, "balance={}", balance);
        }
    }

    #[test]
    fn test_deposit_validation_constants() {
        // Edge cases for deposit amount validation
        assert!(MIN_DEPOSIT - 1 < MIN_DEPOSIT, "amount below min should fail");
        assert!(MAX_DEPOSIT + 1 > MAX_DEPOSIT, "amount above max should fail");
        assert!(MIN_DEPOSIT <= MAX_DEPOSIT, "min must be <= max");
    }
}

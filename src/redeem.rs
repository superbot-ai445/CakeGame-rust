/// CakeGame 兑换码系统
/// GM 生成兑换码，玩家兑换奖励
use crate::db::Database;
use crate::user;

const CURRENCY_GOLD: &str = "Currency_gold";
const CURRENCY_DIAMOND: &str = "Currency_diamond";
const OP_ADD: &str = "+";

/// 生成简单伪随机码 (8位大写字母数字)
fn generate_code(seed: u64) -> String {
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut code = String::with_capacity(8);
    let mut val = seed;
    for _ in 0..8 {
        val = val.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let idx = (val >> 33) as usize % CHARS.len();
        code.push(CHARS[idx] as char);
    }
    code
}

/// 从 Global 表读取兑换码数据
/// 格式: section="RedeemCode", id=CODE, data="类型|数量|已用次数|最大次数|创建者"
fn read_code_data(db: &Database, code: &str) -> Option<(String, i64, i32, i32, String)> {
    let raw = db.global_get("RedeemCode", code);
    if raw.is_empty() {
        return None;
    }
    let parts: Vec<&str> = raw.splitn(5, '|').collect();
    if parts.len() < 5 {
        return None;
    }
    let reward_type = parts[0].to_string();
    let amount: i64 = parts[1].parse().unwrap_or(0);
    let used: i32 = parts[2].parse().unwrap_or(0);
    let max_uses: i32 = parts[3].parse().unwrap_or(0);
    let creator = parts[4].to_string();
    Some((reward_type, amount, used, max_uses, creator))
}

fn write_code_data(db: &Database, code: &str, reward_type: &str, amount: i64, used: i32, max_uses: i32, creator: &str) {
    let data = format!("{}|{}|{}|{}|{}", reward_type, amount, used, max_uses, creator);
    db.global_set("RedeemCode", code, &data);
}

/// 判断用户是否有 GM 权限
fn is_gm(db: &Database, user_id: &str) -> bool {
    let power: i32 = db.global_get("Permissions", user_id).parse().unwrap_or(0);
    power >= 100
}

/// 生成兑换码 (GM)
pub fn cmd_create_redeem(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !is_gm(db, user_id) {
        return format!("{}\n❌ 你无权操作，需要GM权限", prefix);
    }

    // 格式: 生成兑换码+类型+数量+使用次数
    let parts: Vec<&str> = args.splitn(3, '+').collect();
    if parts.len() < 3 {
        return format!(
            "{}\n📌 格式: 生成兑换码+类型+数量+使用次数\n类型: 金币/钻石/物品名\n示例: 生成兑换码+金币+10000+50",
            prefix
        );
    }

    let reward_type = parts[0].trim();
    let amount: i64 = match parts[1].trim().parse() {
        Ok(v) if v > 0 => v,
        _ => return format!("{}\n❌ 数量必须为正整数", prefix),
    };
    let max_uses: i32 = match parts[2].trim().parse() {
        Ok(v) if v > 0 => v,
        _ => return format!("{}\n❌ 使用次数必须为正整数", prefix),
    };

    // 验证物品是否存在（如果不是金币/钻石）
    if reward_type != "金币" && reward_type != "钻石" && db.item_get(reward_type).is_none() {
        return format!("{}\n❌ 物品 [{}] 不存在", prefix, reward_type);
    }

    // 生成唯一码
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let seed = timestamp.wrapping_mul(amount as u64).wrapping_add(max_uses as u64);
    let code = generate_code(seed);

    write_code_data(db, &code, reward_type, amount, 0, max_uses, user_id);

    let reward_desc = match reward_type {
        "金币" => format!("💰 {} 金币", amount),
        "钻石" => format!("💎 {} 钻石", amount),
        _ => format!("🎁 {} ×{}", reward_type, amount),
    };

    let mut out = format!("{}\n", prefix);
    out += "✅ 兑换码生成成功！\n";
    out += "━━━ 兑换码信息 ━━━\n";
    out += &format!("🔑 兑换码: {}\n", code);
    out += &format!("🎁 奖励: {}\n", reward_desc);
    out += &format!("📊 可使用次数: {}\n", max_uses);
    out += "\n📌 玩家发送「兑换码+码」即可兑换\n";
    out
}

/// 兑换码 (玩家)
pub fn cmd_redeem_code(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let code = args.trim().to_uppercase();

    if code.is_empty() {
        return format!("{}\n📌 格式: 兑换码+兑换码\n示例: 兑换码+ABCD1234", prefix);
    }

    // 读取兑换码数据
    let (reward_type, amount, used, max_uses, _creator) = match read_code_data(db, &code) {
        Some(d) => d,
        None => return format!("{}\n❌ 兑换码 [{}] 无效或不存在", prefix, code),
    };

    // 检查是否已用完
    if used >= max_uses {
        return format!("{}\n❌ 兑换码 [{}] 已被使用完毕", prefix, code);
    }

    // 检查用户是否已使用过此码
    let used_key = format!("redeem_used_{}", code);
    let already_used = db.read_user_data(user_id, &used_key);
    if !already_used.is_empty() {
        return format!("{}\n❌ 你已经使用过此兑换码了", prefix);
    }

    // 发放奖励
    let reward_desc = match reward_type.as_str() {
        "金币" => {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, amount);
            format!("💰 {} 金币", amount)
        }
        "钻石" => {
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, amount);
            format!("💎 {} 钻石", amount)
        }
        item_name => {
            db.add_item(user_id, item_name, amount as i32);
            format!("🎁 {} ×{}", item_name, amount)
        }
    };

    // 更新使用次数
    write_code_data(db, &code, &reward_type, amount, used + 1, max_uses, &_creator);

    // 标记用户已使用
    db.write_user_data(user_id, &used_key, "1");

    let mut out = format!("{}\n", prefix);
    out += "🎉 兑换成功！\n";
    out += "━━━ 兑换详情 ━━━\n";
    out += &format!("🔑 兑换码: {}\n", code);
    out += &format!("🎁 获得: {}\n", reward_desc);
    out += &format!("📊 剩余次数: {}/{}\n", max_uses - used - 1, max_uses);
    out
}

/// 兑换码列表 (GM)
pub fn cmd_redeem_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !is_gm(db, user_id) {
        return format!("{}\n❌ 你无权操作，需要GM权限", prefix);
    }

    // 查询所有兑换码
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT ID, DATA FROM Global WHERE SECTION='RedeemCode'") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };

    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    drop(stmt);
    drop(conn);

    if rows.is_empty() {
        return format!(
            "{}\n📭 暂无兑换码\n\n📌 发送「生成兑换码+类型+数量+使用次数」创建兑换码",
            prefix
        );
    }

    let mut out = format!("{}\n", prefix);
    out += "📋 ═══ 兑换码列表 ═══\n";
    out += "━━━━━━━━━━━━━━━━\n";

    for (i, (code, data)) in rows.iter().enumerate() {
        let parts: Vec<&str> = data.splitn(5, '|').collect();
        if parts.len() < 5 {
            continue;
        }
        let reward_type = parts[0];
        let amount = parts[1];
        let used = parts[2];
        let max_uses = parts[3];
        let creator = parts[4];

        let status = if used >= max_uses {
            "🔴已用完"
        } else {
            "🟢有效"
        };
        let reward_desc = match reward_type {
            "金币" => format!("{}金币", amount),
            "钻石" => format!("{}钻石", amount),
            _ => format!("{}×{}", reward_type, amount),
        };

        out += &format!(
            "{}. [{}] {} | {}/{}次 | {}\n",
            i + 1,
            code,
            reward_desc,
            used,
            max_uses,
            status
        );
        out += &format!("   创建者: {}\n", creator);
    }

    out += "\n📌 发送「生成兑换码+类型+数量+使用次数」创建新码\n";
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_length() {
        let code = generate_code(12345);
        assert_eq!(code.len(), 8);
    }

    #[test]
    fn test_generate_code_deterministic() {
        let code1 = generate_code(42);
        let code2 = generate_code(42);
        assert_eq!(code1, code2);
    }

    #[test]
    fn test_generate_code_different_seeds() {
        let code1 = generate_code(1);
        let code2 = generate_code(2);
        assert_ne!(code1, code2);
    }

    #[test]
    fn test_generate_code_chars_valid() {
        const VALID: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let code = generate_code(999);
        for ch in code.chars() {
            assert!(VALID.contains(&(ch as u8)), "Invalid char: {}", ch);
        }
    }

    #[test]
    fn test_generate_code_no_confusing_chars() {
        // Should not contain I, O, 0, 1 to avoid confusion
        let code = generate_code(123456);
        assert!(!code.contains('I'));
        assert!(!code.contains('O'));
        assert!(!code.contains('0'));
        assert!(!code.contains('1'));
    }
}

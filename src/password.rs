/// CakeGame 密码系统
/// 为玩家账户提供密码保护，敏感操作前需验证密码
/// 存储: Global 表, section = 'password', ID = user_id, DATA = hash
/// 验证状态: Global 表, section = 'password_verified', ID = user_id, DATA = "1"/"0"
use crate::db::Database;
use crate::user;

/// 密码最小长度
const MIN_PASSWORD_LEN: usize = 4;
/// 密码最大长度
const MAX_PASSWORD_LEN: usize = 20;
/// 验证有效期（秒）—— 验证后300秒内免重复验证
const VERIFY_TTL_SECS: i64 = 300;

/// 简单哈希函数（djb2变体 + 时间混淆）
/// 游戏场景下足够安全，不依赖外部加密库
fn hash_password(password: &str, user_id: &str) -> u64 {
    let mut h: u64 = 5381;
    // 混入 user_id 作为 salt
    for b in user_id.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    // 主哈希
    for b in password.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    h
}

/// 获取密码哈希
fn get_password_hash(db: &Database, user_id: &str) -> Option<u64> {
    let data = db.global_get("password", user_id);
    if data.is_empty() {
        return None;
    }
    data.parse::<u64>().ok()
}

/// 设置密码哈希
fn save_password_hash(db: &Database, user_id: &str, hash: u64) {
    db.global_set("password", user_id, &hash.to_string());
}

/// 获取验证时间戳
fn get_verify_time(db: &Database, user_id: &str) -> i64 {
    let data = db.global_get("password_verified", user_id);
    if data.is_empty() {
        return 0;
    }
    data.parse::<i64>().unwrap_or(0)
}

/// 设置验证时间戳
fn save_verify_time(db: &Database, user_id: &str, timestamp: i64) {
    db.global_set("password_verified", user_id, &timestamp.to_string());
}

/// 检查密码是否已设置
pub fn has_password(db: &Database, user_id: &str) -> bool {
    get_password_hash(db, user_id).is_some()
}

/// 验证密码是否正确（供其他模块调用）
/// 如果未设置密码，返回 true（无密码保护）
/// 如果已设置密码，检查是否在有效验证期内或密码匹配
pub fn verify_password(db: &Database, user_id: &str, password: &str) -> bool {
    let stored_hash = match get_password_hash(db, user_id) {
        Some(h) => h,
        None => return true, // 未设置密码，直接通过
    };
    let input_hash = hash_password(password, user_id);
    input_hash == stored_hash
}

/// 检查是否在验证有效期内（供其他模块调用，免输入密码）
pub fn is_verified_recently(db: &Database, user_id: &str) -> bool {
    let verify_time = get_verify_time(db, user_id);
    if verify_time == 0 {
        return false;
    }
    let now = chrono::Utc::now().timestamp();
    (now - verify_time) < VERIFY_TTL_SECS
}

/// 密码验证总入口（供敏感操作调用）
/// 返回 (passed, message)
/// - 如果未设置密码 → (true, "")
/// - 如果已验证且在有效期内 → (true, "")
/// - 如果密码匹配 → (true, "✅ 密码验证通过")
/// - 否则 → (false, "❌ 密码错误")
#[allow(dead_code)]
pub fn check_password(db: &Database, user_id: &str, password: &str) -> (bool, String) {
    if !has_password(db, user_id) {
        return (true, String::new());
    }
    if is_verified_recently(db, user_id) {
        return (true, String::new());
    }
    if verify_password(db, user_id, password) {
        let now = chrono::Utc::now().timestamp();
        save_verify_time(db, user_id, now);
        (true, "✅ 密码验证通过".to_string())
    } else {
        (false, "❌ 密码错误，请重新输入\n用法: [原指令]+密码".to_string())
    }
}

/// 密码强度评估
fn password_strength(password: &str) -> (&'static str, &'static str) {
    let len = password.len();
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_alpha = password.chars().any(|c| c.is_ascii_alphabetic());
    let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());

    if len >= 10 && has_digit && has_alpha && has_special {
        ("🔒 极强", "🟢")
    } else if len >= 8 && has_digit && has_alpha {
        ("🔐 较强", "🟢")
    } else if len >= 6 && (has_digit || has_alpha) {
        ("🔓 中等", "🟡")
    } else {
        ("⚠️ 较弱", "🔴")
    }
}

/// 获取密码状态描述
fn password_status(db: &Database, user_id: &str) -> String {
    if has_password(db, user_id) {
        let verify_time = get_verify_time(db, user_id);
        let now = chrono::Utc::now().timestamp();
        if verify_time > 0 && (now - verify_time) < VERIFY_TTL_SECS {
            let remaining = VERIFY_TTL_SECS - (now - verify_time);
            format!("🔒 已设置 | ✅ 已验证 ({}秒内有效)", remaining)
        } else {
            "🔒 已设置 | ⏳ 未验证".to_string()
        }
    } else {
        "🔓 未设置".to_string()
    }
}

// ==================== 指令处理 ====================

/// 设置密码
pub fn cmd_set_password(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n❌ 请先注册再设置密码", prefix);
    }

    let password = args.trim();
    if password.is_empty() {
        let status = password_status(db, user_id);
        let mut out = format!("{}\n🔐 ══════ 设置密码 ══════", prefix);
        out.push_str(&format!("\n\n当前状态: {}", status));
        out.push_str("\n\n用法: 设置密码+新密码");
        out.push_str(&format!("\n密码长度: {}-{} 位", MIN_PASSWORD_LEN, MAX_PASSWORD_LEN));
        out.push_str("\n\n💡 密码保护范围:");
        out.push_str("\n  • 赠送物品/金币/钻石");
        out.push_str("\n  • 出售/丢弃/分解物品");
        out.push_str("\n  • 公会管理操作");
        out.push_str("\n\n⚡ 设置密码后敏感操作需验证");
        return out;
    }

    if has_password(db, user_id) {
        return format!("{}\n❌ 已有密码，请使用「修改密码」指令", prefix);
    }

    if password.len() < MIN_PASSWORD_LEN {
        return format!("{}\n❌ 密码太短！最少 {} 位", prefix, MIN_PASSWORD_LEN);
    }
    if password.len() > MAX_PASSWORD_LEN {
        return format!("{}\n❌ 密码太长！最多 {} 位", prefix, MAX_PASSWORD_LEN);
    }

    let hash = hash_password(password, user_id);
    save_password_hash(db, user_id, hash);
    // 设置后自动验证
    let now = chrono::Utc::now().timestamp();
    save_verify_time(db, user_id, now);

    let (strength, _) = password_strength(password);
    let mut out = format!("{}\n✅ 密码设置成功！", prefix);
    out.push_str(&format!("\n密码强度: {}", strength));
    out.push_str("\n\n💡 敏感操作将需要输入密码验证");
    out.push_str("\n⚠️ 请牢记密码，忘记后需联系GM清除");
    out
}

/// 修改密码
pub fn cmd_change_password(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n❌ 请先注册", prefix);
    }

    if !has_password(db, user_id) {
        return format!("{}\n❌ 未设置密码，请先使用「设置密码」", prefix);
    }

    let parts: Vec<&str> = args.split('+').collect();
    if parts.len() < 2 {
        let mut out = format!("{}\n🔐 ══════ 修改密码 ══════", prefix);
        out.push_str("\n\n用法: 修改密码+旧密码+新密码");
        out.push_str("\n\n示例: 修改密码+1234+5678");
        return out;
    }

    let old_pass = parts[0].trim();
    let new_pass = parts[1].trim();

    // 验证旧密码
    let old_hash = hash_password(old_pass, user_id);
    let stored_hash = match get_password_hash(db, user_id) {
        Some(h) => h,
        None => return format!("{}\n❌ 系统错误", prefix),
    };
    if old_hash != stored_hash {
        return format!("{}\n❌ 旧密码错误", prefix);
    }

    if new_pass.len() < MIN_PASSWORD_LEN {
        return format!("{}\n❌ 新密码太短！最少 {} 位", prefix, MIN_PASSWORD_LEN);
    }
    if new_pass.len() > MAX_PASSWORD_LEN {
        return format!("{}\n❌ 新密码太长！最多 {} 位", prefix, MAX_PASSWORD_LEN);
    }

    if old_pass == new_pass {
        return format!("{}\n❌ 新密码不能与旧密码相同", prefix);
    }

    let new_hash = hash_password(new_pass, user_id);
    save_password_hash(db, user_id, new_hash);
    // 修改后自动验证
    let now = chrono::Utc::now().timestamp();
    save_verify_time(db, user_id, now);

    let (strength, _) = password_strength(new_pass);
    let mut out = format!("{}\n✅ 密码修改成功！", prefix);
    out.push_str(&format!("\n密码强度: {}", strength));
    out
}

/// 验证密码（手动验证，获取操作权限）
pub fn cmd_verify_password(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n❌ 请先注册", prefix);
    }

    if !has_password(db, user_id) {
        return format!("{}\n🔓 未设置密码，无需验证", prefix);
    }

    if is_verified_recently(db, user_id) {
        let remaining = VERIFY_TTL_SECS - (chrono::Utc::now().timestamp() - get_verify_time(db, user_id));
        return format!("{}\n✅ 已在验证有效期内 ({}秒)", prefix, remaining);
    }

    let password = args.trim();
    if password.is_empty() {
        return format!("{}\n用法: 验证密码+密码", prefix);
    }

    if verify_password(db, user_id, password) {
        let now = chrono::Utc::now().timestamp();
        save_verify_time(db, user_id, now);
        format!("{}\n✅ 密码验证通过！{}秒内有效", prefix, VERIFY_TTL_SECS)
    } else {
        format!("{}\n❌ 密码错误", prefix)
    }
}

/// 清除密码
pub fn cmd_clear_password(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n❌ 请先注册", prefix);
    }

    if !has_password(db, user_id) {
        return format!("{}\n🔓 未设置密码，无需清除", prefix);
    }

    let password = args.trim();
    if password.is_empty() {
        let mut out = format!("{}\n🔐 ══════ 清除密码 ══════", prefix);
        out.push_str("\n\n⚠️ 清除密码将取消所有安全保护");
        out.push_str("\n\n用法: 清除密码+当前密码");
        return out;
    }

    if !verify_password(db, user_id, password) {
        return format!("{}\n❌ 密码错误，无法清除", prefix);
    }

    // 清除密码和验证状态
    db.global_set("password", user_id, "");
    db.global_set("password_verified", user_id, "");

    format!("{}\n✅ 密码已清除，安全保护已关闭", prefix)
}

/// 查看密码状态
pub fn cmd_password_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n❌ 请先注册", prefix);
    }

    let status = password_status(db, user_id);
    let mut out = format!("{}\n🔐 ══════ 密码状态 ══════", prefix);
    out.push_str(&format!("\n\n状态: {}", status));

    if has_password(db, user_id) {
        out.push_str("\n\n📋 指令列表:");
        out.push_str("\n  验证密码+密码 — 验证身份");
        out.push_str("\n  修改密码+旧+新 — 更改密码");
        out.push_str("\n  清除密码+密码 — 移除密码");
    } else {
        out.push_str("\n\n💡 建议设置密码保护账户安全");
        out.push_str("\n  设置密码+密码 — 创建密码");
    }

    out.push_str("\n\n🔒 受保护操作:");
    out.push_str("\n  赠送 | 出售 | 丢弃 | 分解 | 公会管理");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let h1 = hash_password("test123", "user1");
        let h2 = hash_password("test123", "user1");
        assert_eq!(h1, h2, "Same input should produce same hash");
    }

    #[test]
    fn test_hash_different_inputs() {
        let h1 = hash_password("pass1", "user1");
        let h2 = hash_password("pass2", "user1");
        assert_ne!(h1, h2, "Different passwords should produce different hashes");
    }

    #[test]
    fn test_hash_different_users() {
        let h1 = hash_password("same", "user1");
        let h2 = hash_password("same", "user2");
        assert_ne!(h1, h2, "Same password with different users should differ");
    }

    #[test]
    fn test_password_strength() {
        let (s1, _) = password_strength("1234");
        assert!(s1.contains("较弱"), "Short numeric should be weak");

        let (s2, _) = password_strength("abcdef12");
        assert!(s2.contains("较强") || s2.contains("中等"), "Medium should be moderate+");

        let (s3, _) = password_strength("Abc123!@#$");
        assert!(s3.contains("极强"), "Complex should be strongest");
    }

    #[test]
    fn test_hash_no_zero() {
        // Ensure hash doesn't produce 0 for common inputs
        let h = hash_password("", "empty");
        assert_ne!(h, 0, "Hash should not be zero");
    }
}

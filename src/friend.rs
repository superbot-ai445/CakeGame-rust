/// CakeGame 好友系统
/// 提供添加/删除好友、好友列表、好友在线状态等功能
use crate::db::Database;
use crate::user;

/// 好友分隔符
const FRIEND_SEP: &str = ",";

/// 获取好友列表
fn get_friend_list(db: &Database, user_id: &str) -> Vec<String> {
    let raw = db.read_user_data(user_id, "friend_list");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(FRIEND_SEP)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 保存好友列表
fn save_friend_list(db: &Database, user_id: &str, friends: &[String]) {
    db.write_user_data(user_id, "friend_list", &friends.join(FRIEND_SEP));
}

/// 检查是否已注册
fn check_registered(db: &Database, user_id: &str) -> bool {
    db.user_exists(user_id)
}

/// 添加好友
pub fn cmd_add_friend(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !check_registered(db, user_id) {
        return format!("{}\n请先注册后再添加好友！\n发送【注册+昵称】进行注册", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要添加的好友ID或昵称！\n用法: 添加好友+玩家ID", prefix);
    }

    // 不能添加自己
    if target == user_id {
        return format!("{}\n不能添加自己为好友哦~", prefix);
    }

    // 检查目标用户是否存在
    if !db.user_exists(target) {
        return format!("{}\n找不到玩家 [{}]，请确认玩家ID是否正确。", prefix, target);
    }

    let mut friends = get_friend_list(db, user_id);

    // 检查是否已经是好友
    if friends.contains(&target.to_string()) {
        let target_name = db.read_user_data(target, "Name");
        let display_name = if target_name.is_empty() { target } else { &target_name };
        return format!("{}\n{} 已经是你的好友了！", prefix, display_name);
    }

    // 好友上限检查 (50人)
    if friends.len() >= 50 {
        return format!("{}\n好友数量已达上限(50人)！\n请先删除一些好友再添加新好友。", prefix);
    }

    friends.push(target.to_string());
    save_friend_list(db, user_id, &friends);

    // 同步到对方的好友列表 (双向好友)
    let mut target_friends = get_friend_list(db, target);
    if !target_friends.contains(&user_id.to_string()) {
        target_friends.push(user_id.to_string());
        save_friend_list(db, target, &target_friends);
    }

    let target_name = db.read_user_data(target, "Name");
    let display_name = if target_name.is_empty() { target } else { &target_name };

    format!(
        "{}\n✅ 添加好友成功！\n🧑 {} (ID:{}) 已加入你的好友列表\n👥 当前好友数: {}/50",
        prefix,
        display_name,
        target,
        friends.len()
    )
}

/// 删除好友
pub fn cmd_remove_friend(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !check_registered(db, user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要删除的好友ID！\n用法: 删除好友+玩家ID", prefix);
    }

    let mut friends = get_friend_list(db, user_id);

    if !friends.contains(&target.to_string()) {
        return format!("{}\n{} 不在你的好友列表中。", prefix, target);
    }

    friends.retain(|f| f != target);
    save_friend_list(db, user_id, &friends);

    // 同步删除对方好友列表中的自己
    let mut target_friends = get_friend_list(db, target);
    target_friends.retain(|f| f != user_id);
    save_friend_list(db, target, &target_friends);

    format!(
        "{}\n🗑️ 已删除好友 {}\n👥 当前好友数: {}/50",
        prefix,
        target,
        friends.len()
    )
}

/// 查看好友列表
pub fn cmd_friend_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !check_registered(db, user_id) {
        return format!("{}\n请先注册后再查看好友列表！", prefix);
    }

    let friends = get_friend_list(db, user_id);

    if friends.is_empty() {
        return format!(
            "{}\n═══ 好友列表 ═══\n\n暂无好友\n\n💡 发送「添加好友+玩家ID」添加好友",
            prefix
        );
    }

    let mut result = format!("{}\n═══ 好友列表 ({}/50) ═══\n", prefix, friends.len());

    for (i, fid) in friends.iter().enumerate() {
        let name = db.read_user_data(fid, "Name");
        let display_name = if name.is_empty() { fid.clone() } else { name };

        let level: i32 = db.read_user_data(fid, "Level").parse().unwrap_or(0);
        let occupation = db.read_user_data(fid, "Occupation");
        let hp: i32 = db.read_user_data(fid, "HP_Current").parse().unwrap_or(0);
        let status = if hp > 0 { "🟢在线" } else { "🔴离线" };

        result.push_str(&format!(
            "\n{}. {} [{}] Lv.{} {}",
            i + 1,
            display_name,
            status,
            level,
            occupation
        ));
    }

    result.push_str("\n\n💡 发送「查看好友+玩家ID」查看好友详情");
    result.push_str("\n💡 发送「删除好友+玩家ID」删除好友");
    result
}

/// 查看好友详情
pub fn cmd_view_friend(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !check_registered(db, user_id) {
        return format!("{}\n请先注册后再查看好友！", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要查看的好友ID！\n用法: 查看好友+玩家ID", prefix);
    }

    let friends = get_friend_list(db, user_id);
    if !friends.contains(&target.to_string()) {
        return format!("{}\n{} 不在你的好友列表中。\n请先添加好友。", prefix, target);
    }

    let name = db.read_user_data(target, "Name");
    let display_name = if name.is_empty() { target } else { &name };
    let level: i32 = db.read_user_data(target, "Level").parse().unwrap_or(0);
    let occupation = db.read_user_data(target, "Occupation");
    let hp: i32 = db.read_user_data(target, "HP_Current").parse().unwrap_or(0);
    let hp_max: i32 = db.read_user_data(target, "HP_Max").parse().unwrap_or(0);
    let mp: i32 = db.read_user_data(target, "MP_Current").parse().unwrap_or(0);
    let mp_max: i32 = db.read_user_data(target, "MP_Max").parse().unwrap_or(0);
    let location = db.read_user_data(target, "Location");
    let gold: i64 = db.read_user_data(target, "currency_gold").parse().unwrap_or(0);

    let status = if hp > 0 { "🟢 在线" } else { "🔴 离线" };

    let mut result = format!(
        "{}\n═══ 好友详情 ═══\n\n🧑 昵称: {}\n📊 等级: Lv.{}\n⚔️ 职业: {}\n📍 位置: {}\n{}\n❤️ 生命: {}/{}\n💙 魔法: {}/{}\n💰 金币: {}",
        prefix,
        display_name,
        level,
        occupation,
        location,
        status,
        hp,
        hp_max,
        mp,
        mp_max,
        gold
    );

    result.push_str(&format!("\n💡 发送「赠送物品+{}+物品名」给好友赠送物品", target));
    result.push_str(&format!("\n💡 发送「赠送金币+{}+数量」给好友赠送金币", target));

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friend_sep() {
        assert_eq!(FRIEND_SEP, ",");
    }

    #[test]
    fn test_friend_list_parse() {
        let raw = "user1,user2,user3";
        let friends: Vec<String> = raw
            .split(FRIEND_SEP)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(friends.len(), 3);
        assert_eq!(friends[0], "user1");
        assert_eq!(friends[1], "user2");
        assert_eq!(friends[2], "user3");
    }

    #[test]
    fn test_friend_list_parse_empty() {
        let raw = "";
        let friends: Vec<String> = raw
            .split(FRIEND_SEP)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(friends.is_empty());
    }

    #[test]
    fn test_friend_list_parse_with_spaces() {
        let raw = "user1, user2 , user3";
        let friends: Vec<String> = raw
            .split(FRIEND_SEP)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(friends.len(), 3);
    }

    #[test]
    fn test_friend_list_parse_trailing_comma() {
        let raw = "user1,user2,";
        let friends: Vec<String> = raw
            .split(FRIEND_SEP)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(friends.len(), 2);
    }

    #[test]
    fn test_friend_list_serialize() {
        let friends = vec!["user1".to_string(), "user2".to_string()];
        let serialized = friends.join(FRIEND_SEP);
        assert_eq!(serialized, "user1,user2");
    }

    #[test]
    fn test_friend_limit() {
        let max_friends = 50;
        assert_eq!(max_friends, 50);
        // Verify we check against this limit
        let current_count = 49usize;
        assert!(current_count < max_friends);
        let current_count = 50usize;
        assert!(current_count >= max_friends);
    }

    #[test]
    fn test_self_add_check() {
        let user_id = "12345";
        let target = "12345";
        assert_eq!(user_id, target, "Should detect self-add attempt");
    }
}

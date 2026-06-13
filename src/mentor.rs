/// CakeGame 师徒系统
/// 提供拜师/收徒/出师等师徒关系管理功能
use crate::db::Database;
use crate::user;

const MAX_APPRENTICES: usize = 3; // 每个师傅最多收3个徒弟
const GRADUATE_LEVEL: i32 = 20; // 出师等级要求

/// 获取师傅ID
fn get_master(db: &Database, user_id: &str) -> String {
    db.read_user_data(user_id, "master_id")
}

/// 设置师傅
fn set_master(db: &Database, user_id: &str, master_id: &str) {
    db.write_user_data(user_id, "master_id", master_id);
}

/// 清除师傅关系
fn clear_master(db: &Database, user_id: &str) {
    db.delete_user_data(user_id, "master_id");
}

/// 获取徒弟列表
fn get_apprentices(db: &Database, master_id: &str) -> Vec<String> {
    let raw = db.read_user_data(master_id, "apprentice_list");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 保存徒弟列表
fn save_apprentices(db: &Database, master_id: &str, list: &[String]) {
    db.write_user_data(master_id, "apprentice_list", &list.join(","));
}

/// 获取拜师申请列表
fn get_apprentice_requests(db: &Database, master_id: &str) -> Vec<String> {
    let raw = db.read_user_data(master_id, "apprentice_requests");
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 保存拜师申请列表
fn save_apprentice_requests(db: &Database, master_id: &str, list: &[String]) {
    db.write_user_data(master_id, "apprentice_requests", &list.join(","));
}

/// 获取用户等级
fn get_level(db: &Database, user_id: &str) -> i32 {
    db.read_user_data(user_id, "LV").parse::<i32>().unwrap_or(1)
}

/// 显示名（优先昵称，否则ID）
fn display_name(db: &Database, uid: &str) -> String {
    let name = db.read_user_data(uid, "Name");
    if name.is_empty() {
        uid.to_string()
    } else {
        name
    }
}

/// 拜师 - 向目标玩家发送拜师申请
pub fn cmd_apprentice_request(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再拜师！\n发送【注册+昵称】进行注册", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要拜的师傅ID！\n用法: 拜师+师傅ID", prefix);
    }

    // 不能拜自己为师
    if target == user_id {
        return format!("{}\n不能拜自己为师哦~", prefix);
    }

    // 检查目标用户是否存在
    if !db.user_exists(target) {
        return format!("{}\n找不到玩家 [{}]，请确认玩家ID是否正确。", prefix, target);
    }

    // 检查是否已经有师傅
    let current_master = get_master(db, user_id);
    if !current_master.is_empty() {
        let display = display_name(db, &current_master);
        return format!(
            "{}\n你已经有师傅了！\n👨‍🏫 当前师傅: {}\n💡 如需更换师傅，请先【逐师】解除关系。",
            prefix, display
        );
    }

    // 检查目标是否已有师傅（不能拜有师傅的人为师）
    let target_master = get_master(db, target);
    if !target_master.is_empty() {
        return format!("{}\n该玩家自己也有师傅，不能拜他为师。", prefix);
    }

    // 检查对方的徒弟数量
    let apprentices = get_apprentices(db, target);
    if apprentices.len() >= MAX_APPRENTICES {
        return format!("{}\n该师傅的徒弟数量已满({}人)，无法拜师。", prefix, MAX_APPRENTICES);
    }

    // 检查是否已发送申请
    let mut requests = get_apprentice_requests(db, target);
    if requests.contains(&user_id.to_string()) {
        let display = display_name(db, target);
        return format!("{}\n你已经向 {} 发送过拜师申请了，请等待对方处理。", prefix, display);
    }

    // 添加申请
    requests.push(user_id.to_string());
    save_apprentice_requests(db, target, &requests);

    let display = display_name(db, target);

    format!(
        "{}\n✅ 拜师申请已发送！\n📨 已向 [{}] 发送拜师请求\n⏳ 请等待对方同意收徒\n💡 对方可使用【收徒列表】查看申请",
        prefix, display
    )
}

/// 收徒列表 - 查看拜师申请
pub fn cmd_apprentice_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let requests = get_apprentice_requests(db, user_id);
    let apprentices = get_apprentices(db, user_id);

    let mut result = format!("{}\n🎓 === 师徒面板 ===\n\n", prefix);

    // 当前徒弟
    result.push_str(&format!("👥 当前徒弟: {}/{}\n", apprentices.len(), MAX_APPRENTICES));
    if apprentices.is_empty() {
        result.push_str("  (暂无徒弟)\n");
    } else {
        for (i, aid) in apprentices.iter().enumerate() {
            let name = display_name(db, aid);
            let level = get_level(db, aid);
            result.push_str(&format!("  {}. {} (Lv.{}) [ID:{}]\n", i + 1, name, level, aid));
        }
    }

    // 拜师申请
    result.push_str(&format!("\n📩 拜师申请: {}条\n", requests.len()));
    if requests.is_empty() {
        result.push_str("  (暂无申请)\n");
    } else {
        for (i, rid) in requests.iter().enumerate() {
            let name = display_name(db, rid);
            let level = get_level(db, rid);
            result.push_str(&format!("  {}. {} (Lv.{}) [ID:{}]\n", i + 1, name, level, rid));
        }
        result.push_str("\n💡 发送【同意收徒+玩家ID】收为徒弟\n💡 发送【拒绝收徒+玩家ID】拒绝申请");
    }

    result
}

/// 同意收徒 - 接受拜师申请
pub fn cmd_accept_apprentice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要接受的申请者ID！\n用法: 同意收徒+玩家ID", prefix);
    }

    // 检查申请列表
    let mut requests = get_apprentice_requests(db, user_id);
    if !requests.contains(&target.to_string()) {
        return format!("{}\n没有收到来自 [{}] 的拜师申请。", prefix, target);
    }

    // 检查徒弟数量
    let mut apprentices = get_apprentices(db, user_id);
    if apprentices.len() >= MAX_APPRENTICES {
        return format!("{}\n徒弟数量已达上限({}人)！", prefix, MAX_APPRENTICES);
    }

    // 检查目标是否已有师傅
    let current_master = get_master(db, target);
    if !current_master.is_empty() {
        // 从申请列表移除
        requests.retain(|r| r != target);
        save_apprentice_requests(db, user_id, &requests);
        return format!("{}\n该玩家已经有师傅了，申请已自动移除。", prefix);
    }

    // 从申请列表移除，添加到徒弟列表
    requests.retain(|r| r != target);
    save_apprentice_requests(db, user_id, &requests);

    apprentices.push(target.to_string());
    save_apprentices(db, user_id, &apprentices);

    // 设置对方的师傅
    set_master(db, target, user_id);

    let display = display_name(db, target);

    format!(
        "{}\n🎉 收徒成功！\n🧑 {} (ID:{}) 已成为你的徒弟\n👥 当前徒弟数: {}/{}\n💡 徒弟达到Lv.{}后可使用【出师】",
        prefix,
        display,
        target,
        apprentices.len(),
        MAX_APPRENTICES,
        GRADUATE_LEVEL
    )
}

/// 拒绝收徒 - 拒绝拜师申请
pub fn cmd_reject_apprentice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要拒绝的申请者ID！\n用法: 拒绝收徒+玩家ID", prefix);
    }

    let mut requests = get_apprentice_requests(db, user_id);
    if !requests.contains(&target.to_string()) {
        return format!("{}\n没有收到来自 [{}] 的拜师申请。", prefix, target);
    }

    requests.retain(|r| r != target);
    save_apprentice_requests(db, user_id, &requests);

    let display = display_name(db, target);

    format!("{}\n❌ 已拒绝 {} 的拜师申请。", prefix, display)
}

/// 逐师 - 解除师徒关系（师傅逐出徒弟 / 徒弟离开师傅）
pub fn cmd_dismiss_apprentice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        return format!("{}\n请指定要解除关系的玩家ID！\n用法: 逐师+玩家ID", prefix);
    }

    // 检查是否是自己的徒弟
    let mut apprentices = get_apprentices(db, user_id);
    if apprentices.contains(&target.to_string()) {
        // 师傅逐出徒弟
        apprentices.retain(|a| a != target);
        save_apprentices(db, user_id, &apprentices);
        clear_master(db, target);

        let display = display_name(db, target);

        return format!(
            "{}\n👋 已逐出徒弟 [{}]\n👥 当前徒弟数: {}/{}",
            prefix,
            display,
            apprentices.len(),
            MAX_APPRENTICES
        );
    }

    // 检查是否是自己的师傅
    let master = get_master(db, user_id);
    if master == target {
        // 徒弟离开师傅
        let mut master_apprentices = get_apprentices(db, &master);
        master_apprentices.retain(|a| a != user_id);
        save_apprentices(db, &master, &master_apprentices);
        clear_master(db, user_id);

        let display = display_name(db, target);

        return format!(
            "{}\n👋 你已离开师傅 [{}] 的门下\n💡 你现在可以重新拜师了。",
            prefix, display
        );
    }

    format!("{}\n[{}] 不是你的徒弟，也不是你的师傅。", prefix, target)
}

/// 出师 - 徒弟毕业（需要达到等级要求）
pub fn cmd_graduate(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再操作！", prefix);
    }

    let master = get_master(db, user_id);
    if master.is_empty() {
        return format!("{}\n你还没有师傅，无法出师。", prefix);
    }

    let level = get_level(db, user_id);
    if level < GRADUATE_LEVEL {
        return format!(
            "{}\n🎓 出师需要达到 Lv.{}，你当前 Lv.{}\n⏳ 继续努力升级吧！",
            prefix, GRADUATE_LEVEL, level
        );
    }

    // 从师傅的徒弟列表移除
    let mut master_apprentices = get_apprentices(db, &master);
    master_apprentices.retain(|a| a != user_id);
    save_apprentices(db, &master, &master_apprentices);
    clear_master(db, user_id);

    let master_display = display_name(db, &master);
    let my_display = display_name(db, user_id);

    // 出师奖励
    let gold_reward = 2000;
    let diamond_reward = 100;
    db.modify_currency(user_id, "Currency_gold", "add", gold_reward);
    db.modify_currency(user_id, "Currency_diamond", "add", diamond_reward);
    db.knapsack_add(user_id, "初级礼包", 1);

    // 记录出师次数
    let graduate_count: i32 = db.read_user_data(user_id, "graduate_count").parse::<i32>().unwrap_or(0) + 1;
    db.write_user_data(user_id, "graduate_count", &graduate_count.to_string());

    format!(
        "{}\n🎓 === 恭喜出师！===\n\n👨‍🏫 师傅: {}\n🧑 徒弟: {}\n📊 出师等级: Lv.{}\n\n🎁 出师奖励:\n  💰 金币 +{}\n  💎 钻石 +{}\n  📦 初级礼包 ×1\n\n💡 你已出师，可以收徒弟了！",
        prefix, master_display, my_display, level, gold_reward, diamond_reward
    )
}

/// 查看师徒关系 - 查看自己的师傅和徒弟信息
pub fn cmd_view_mentor(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let mut result = format!("{}\n👨‍🏫 === 师徒关系 ===\n\n", prefix);

    // 师傅信息
    let master = get_master(db, user_id);
    if master.is_empty() {
        result.push_str("🎓 师傅: 无（可发送【拜师+ID】拜师）\n");
    } else {
        let display = display_name(db, &master);
        let master_level = get_level(db, &master);
        let master_occ = db.read_user_data(&master, "Occupation");
        result.push_str(&format!(
            "🎓 师傅: {} (Lv.{}) [{}]\n   ID: {}\n",
            display, master_level, master_occ, master
        ));
    }

    // 徒弟信息
    let apprentices = get_apprentices(db, user_id);
    result.push_str(&format!("\n👥 徒弟: {}/{}\n", apprentices.len(), MAX_APPRENTICES));
    if apprentices.is_empty() {
        result.push_str("  (暂无徒弟，可等待他人拜师)\n");
    } else {
        for (i, aid) in apprentices.iter().enumerate() {
            let name = display_name(db, aid);
            let level = get_level(db, aid);
            let occ = db.read_user_data(aid, "Occupation");
            result.push_str(&format!("  {}. {} (Lv.{}) [{}]\n", i + 1, name, level, occ));
        }
    }

    // 待处理申请数
    let requests = get_apprentice_requests(db, user_id);
    if !requests.is_empty() {
        result.push_str(&format!(
            "\n📩 待处理申请: {}条（发送【收徒列表】查看）\n",
            requests.len()
        ));
    }

    // 出师统计
    let graduate_count: i32 = db.read_user_data(user_id, "graduate_count").parse::<i32>().unwrap_or(0);
    if graduate_count > 0 {
        result.push_str(&format!("\n🏆 历史出师: {}人\n", graduate_count));
    }

    result.push_str("\n💡 指令:\n  拜师+ID | 收徒列表 | 同意收徒+ID\n  拒绝收徒+ID | 逐师+ID | 出师 | 师徒");

    result
}

#[allow(dead_code)]
/// 解析逗号分隔的ID列表
pub fn parse_id_list(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[allow(dead_code)]
/// 序列化ID列表为逗号分隔字符串
pub fn serialize_id_list(list: &[String]) -> String {
    list.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_apprentices() {
        assert_eq!(MAX_APPRENTICES, 3);
    }

    #[test]
    fn test_graduate_level() {
        assert_eq!(GRADUATE_LEVEL, 20);
    }

    #[test]
    fn test_parse_id_list_empty() {
        assert!(parse_id_list("").is_empty());
    }

    #[test]
    fn test_parse_id_list_single() {
        let list = parse_id_list("user123");
        assert_eq!(list, vec!["user123"]);
    }

    #[test]
    fn test_parse_id_list_multiple() {
        let list = parse_id_list("user1,user2,user3");
        assert_eq!(list, vec!["user1", "user2", "user3"]);
    }

    #[test]
    fn test_parse_id_list_with_spaces() {
        let list = parse_id_list("user1, user2 , user3");
        assert_eq!(list, vec!["user1", "user2", "user3"]);
    }

    #[test]
    fn test_parse_id_list_trailing_comma() {
        let list = parse_id_list("user1,user2,");
        assert_eq!(list, vec!["user1", "user2"]);
    }

    #[test]
    fn test_serialize_id_list_empty() {
        assert_eq!(serialize_id_list(&[]), "");
    }

    #[test]
    fn test_serialize_id_list_roundtrip() {
        let original = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let serialized = serialize_id_list(&original);
        let parsed = parse_id_list(&serialized);
        assert_eq!(original, parsed);
    }
}

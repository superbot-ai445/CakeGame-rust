/// CakeGame 社交系统
/// 公会、队伍、匹配竞技
use crate::core::*;
use crate::db::Database;
use crate::encoding;
use crate::user;
use rand::Rng;

/// 创建公会
pub fn create_guild(db: &Database, user_id: &str, guild_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild_name = guild_name.trim().replace('\n', "");

    if guild_name.is_empty() {
        return format!("{}\n创建失败。\n公会名不能为空！", prefix);
    }
    if guild_name.len() > 12 {
        return format!("{}\n创建失败。\n公会名称请控制在6个汉字或12个字母内！", prefix);
    }

    // 检查等级
    let req_level: i32 = db.global_get("Union", "LV").parse().unwrap_or(1);
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if user_level < req_level {
        return format!("{}\n创建公会需要等级达到{}，努力升级吧！", prefix, req_level);
    }

    // 检查是否已有公会
    let current_guild = db.read_basic(user_id, ITEM_GUILD);
    if !current_guild.is_empty() {
        return format!("{}\n您已加入公会[{}]，需要退出后才可创建！", prefix, current_guild);
    }

    // 检查公会是否已存在（简化：检查 Global 表）
    let existing = db.global_get("UnionData", &format!("{}.Owner", guild_name));
    if !existing.is_empty() {
        return format!("{}\n公会[{}]已存在！请换一个名称。", prefix, guild_name);
    }

    // 检查消耗
    let cost: i64 = db.global_get("Union", "Consumption").parse().unwrap_or(0);
    let currency = db.global_get("Union", "Currency");
    let currency = if currency.is_empty() { "钻石" } else { &currency };
    let cn = match currency {
        "金币" => "金币",
        _ => "钻石",
    };

    if cost > 0 {
        let balance = db.read_currency(user_id, currency);
        if balance < cost {
            return format!("{}\n创建公会需要{}{}，余额不足！", prefix, cost, cn);
        }
        db.modify_currency(user_id, currency, OP_SUB, cost);
    }

    // 创建公会
    db.global_set("UnionData", &format!("{}.Owner", guild_name), user_id);
    db.global_set("UnionData", &format!("{}.Level", guild_name), "1");
    db.global_set("UnionData", &format!("{}.Members", guild_name), user_id);
    db.write_basic(user_id, ITEM_GUILD, &guild_name);

    let cost_text = if cost > 0 {
        format!("\n消耗了{}{}", cost, cn)
    } else {
        String::new()
    };
    format!("{}\n成功创建公会[{}]！", prefix, guild_name) + &cost_text
}

/// 查看公会信息
pub fn view_guild(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    let level = db.global_get("UnionData", &format!("{}.Level", guild));
    let members_str = db.global_get("UnionData", &format!("{}.Members", guild));
    let members: Vec<&str> = members_str.split(',').filter(|s| !s.is_empty()).collect();

    let owner_name = user::get_msg_prefix(db, &owner);
    format!(
        "{}\n═══ 公会：{} ═══\n等级：{}\n会长：{}\n成员数：{}",
        prefix,
        guild,
        level,
        owner_name,
        members.len()
    )
}

/// 退出公会
pub fn leave_guild(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner == user_id {
        return format!("{}\n您是公会会长，请先转让公会或解散公会！", prefix);
    }

    // 从成员列表移除
    let members_str = db.global_get("UnionData", &format!("{}.Members", guild));
    let mut members: Vec<String> = members_str
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    members.retain(|m| m != user_id);
    db.global_set("UnionData", &format!("{}.Members", &guild), &members.join(","));

    db.write_basic(user_id, ITEM_GUILD, EMPTY);
    format!("{}\n已退出公会[{}]", prefix, guild)
}

/// 解散公会
pub fn disband_guild(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n您不是公会会长，无法解散公会！", prefix);
    }

    // 清除所有成员的公会
    let members_str = db.global_get("UnionData", &format!("{}.Members", guild));
    for member in members_str.split(',') {
        if !member.is_empty() {
            db.write_basic(member, ITEM_GUILD, EMPTY);
        }
    }

    // 删除公会数据
    db.global_set("UnionData", &format!("{}.Owner", &guild), "");
    db.global_set("UnionData", &format!("{}.Level", &guild), "");
    db.global_set("UnionData", &format!("{}.Members", &guild), "");

    format!("{}\n公会[{}]已解散！", prefix, guild)
}

/// 转让公会
pub fn transfer_guild(db: &Database, user_id: &str, target: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n您不是公会会长！", prefix);
    }

    if !db.user_exists(target) {
        return format!("{}\n目标用户不存在！", prefix);
    }

    let target_guild = db.read_basic(target, ITEM_GUILD);
    if target_guild != guild {
        return format!("{}\n对方不是本公会成员！", prefix);
    }

    db.global_set("UnionData", &format!("{}.Owner", &guild), target);
    format!(
        "{}\n已将公会[{}]转让给{}",
        prefix,
        guild,
        user::get_msg_prefix(db, target)
    )
}

/// 公会列表
pub fn guild_list(db: &Database, user_id: &str, page: i32) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guilds: Vec<(String, String)> = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT ID, DATA FROM Global WHERE SECTION='UnionData' AND ID LIKE '%.Owner'")
            .unwrap();
        stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .map(|(k, v)| (k.replace(".Owner", ""), v))
            .collect()
    };

    if guilds.is_empty() {
        return format!("{}\n暂无公会！", prefix);
    }

    let ps = 10;
    let tp = ((guilds.len() as i32) + ps - 1) / ps;
    let page = page.min(tp).max(1);
    let s = (((page) - 1) * ps) as usize;
    let e = (s + ps as usize).min(guilds.len());

    let mut r = format!("{}\n═══ 公会列表 ({}/{}) ═══", prefix, page, tp);
    for (i, (name, owner)) in guilds[s..e].iter().enumerate() {
        let owner_name = user::get_msg_prefix(db, owner);
        r.push_str(&format!("\n{}. [{}] 会长：{}", s + i + 1, name, owner_name));
    }
    r
}

/// 公会捐献
pub fn guild_donate(db: &Database, user_id: &str, amount_str: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let amount: i64 = amount_str.parse().unwrap_or(0);
    if amount <= 0 {
        return format!("{}\n请输入正确的捐献数量！", prefix);
    }

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < amount {
        return format!("{}\n金币不足！当前：{}", prefix, gold);
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, amount);

    // 增加公会经验（简化：1金币=1经验）
    let current_exp: i64 = db
        .global_get("UnionData", &format!("{}.Exp", guild))
        .parse()
        .unwrap_or(0);
    db.global_set(
        "UnionData",
        &format!("{}.Exp", &guild),
        &(current_exp + amount).to_string(),
    );

    crate::achievement::on_guild_donate(db, user_id);
    format!(
        "{}\n成功捐献{}金币给公会[{}]\n剩余金币：{}",
        prefix,
        amount,
        guild,
        db.read_currency(user_id, CURRENCY_GOLD)
    )
}

// ==================== 队伍系统 ====================

/// 创建队伍
pub fn create_team(db: &Database, user_id: &str, team_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let current_team = db.read_user_data(user_id, "team");
    if !current_team.is_empty() {
        return format!("{}\n您已在队伍[{}]中，请先退出！", prefix, current_team);
    }

    let team_name = team_name.trim().replace('\n', "");
    let team_name = if team_name.is_empty() {
        format!("{}的队伍", user::get_msg_prefix(db, user_id))
    } else {
        team_name.to_string()
    };

    db.write_user_data(user_id, "team", &team_name);
    db.write_user_data(user_id, "team_leader", "TRUE");
    db.global_set("TeamData", &format!("{}.Leader", team_name), user_id);
    db.global_set("TeamData", &format!("{}.Members", team_name), user_id);

    format!("{}\n成功创建队伍[{}]！", prefix, team_name)
}

/// 加入队伍
pub fn join_team(db: &Database, user_id: &str, team_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let current_team = db.read_user_data(user_id, "team");
    if !current_team.is_empty() {
        return format!("{}\n您已在队伍[{}]中，请先退出！", prefix, current_team);
    }

    let members = db.global_get("TeamData", &format!("{}.Members", team_name));
    if members.is_empty() {
        return format!("{}\n队伍[{}]不存在！", prefix, team_name);
    }

    let new_members = if members.is_empty() {
        user_id.to_string()
    } else {
        format!("{},{}", members, user_id)
    };
    db.global_set("TeamData", &format!("{}.Members", team_name), &new_members);
    db.write_user_data(user_id, "team", team_name);

    format!("{}\n成功加入队伍[{}]！", prefix, team_name)
}

/// 退出队伍
pub fn leave_team(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let team = db.read_user_data(user_id, "team");
    if team.is_empty() {
        return format!("{}\n您不在任何队伍中！", prefix);
    }

    let is_leader = db.read_user_data(user_id, "team_leader") == "TRUE";

    // 从成员列表移除
    let members = db.global_get("TeamData", &format!("{}.Members", &team));
    let mut member_list: Vec<String> = members
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    member_list.retain(|m| m != user_id);

    if is_leader && !member_list.is_empty() {
        // 转让队长给第一个成员
        let new_leader = member_list[0].clone();
        db.global_set("TeamData", &format!("{}.Leader", &team), &new_leader);
        db.write_user_data(&new_leader, "team_leader", "TRUE");
    }

    if member_list.is_empty() {
        // 队伍解散
        db.global_set("TeamData", &format!("{}.Leader", &team), "");
        db.global_set("TeamData", &format!("{}.Members", &team), "");
    } else {
        db.global_set("TeamData", &format!("{}.Members", &team), &member_list.join(","));
    }

    db.write_user_data(user_id, "team", "");
    db.write_user_data(user_id, "team_leader", "");
    format!("{}\n已退出队伍[{}]", prefix, team)
}

/// 查看队伍成员
pub fn team_members(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let team = db.read_user_data(user_id, "team");
    if team.is_empty() {
        return format!("{}\n您不在任何队伍中！", prefix);
    }

    let leader = db.global_get("TeamData", &format!("{}.Leader", &team));
    let members_str = db.global_get("TeamData", &format!("{}.Members", &team));
    let members: Vec<&str> = members_str.split(',').filter(|s| !s.is_empty()).collect();

    let mut r = format!("{}\n═══ 队伍：{} ═══", prefix, team);
    for (i, m) in members.iter().enumerate() {
        let name = user::get_msg_prefix(db, m);
        let tag = if *m == leader { " (队长)" } else { "" };
        r.push_str(&format!("\n{}. {}{}", i + 1, name, tag));
    }
    r
}

/// 请出队伍成员（队长专用）
/// 来源: EPL 指令处理_请出成员
pub fn kick_team_member(db: &Database, user_id: &str, target_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target = target_id.trim();

    let team = db.read_user_data(user_id, "team");
    if team.is_empty() {
        return format!("{}\n您还未创建或加入队伍！", prefix);
    }

    // 检查目标用户是否存在
    let target_name = db.read_user_data(target, "name");
    if target_name.is_empty() {
        return format!("{}\n不存在此用户！", prefix);
    }

    // 不能请出自己
    if target == user_id {
        return format!("{}\n不能请出您自己！", prefix);
    }

    // 只有队长有权限
    let is_leader = db.read_user_data(user_id, "team_leader") == "TRUE";
    if !is_leader {
        return format!("{}\n只有队长有权限执行本操作！", prefix);
    }

    // 检查目标是否在队伍中
    let members = db.global_get("TeamData", &format!("{}.Members", &team));
    let member_list: Vec<&str> = members.split(',').filter(|s| !s.is_empty()).collect();
    if !member_list.contains(&target) {
        return format!("{}\n您欲请出的成员不在您的队伍！", prefix);
    }

    // 从成员列表移除
    let new_members: Vec<String> = member_list
        .iter()
        .filter(|&&m| m != target)
        .map(|s| s.to_string())
        .collect();

    db.global_set("TeamData", &format!("{}.Members", &team), &new_members.join(","));
    db.write_user_data(target, "team", "");
    db.write_user_data(target, "team_leader", "");

    format!("{}\n您已成功请出{}({})", prefix, target_name, target)
}

// ==================== 公会成员系统 ====================

/// 查看公会成员
pub fn guild_members(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    let members_str = db.global_get("UnionData", &format!("{}.Members", guild));
    let members: Vec<&str> = members_str.split(',').filter(|s| !s.is_empty()).collect();

    let mut r = format!("{}\n═══ 公会[{}]成员列表 ({}) ═══", prefix, guild, members.len());
    for (i, m) in members.iter().enumerate() {
        let name = user::get_msg_prefix(db, m);
        let tag = if *m == owner { " (会长)" } else { "" };
        let level = db.read_basic(m, ITEM_LEVEL);
        r.push_str(&format!("\n{}. {} Lv.{}{}", i + 1, name, level, tag));
    }
    r
}

/// 踢出公会成员
pub fn kick_guild_member(db: &Database, user_id: &str, target_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n您不是公会会长，无法踢人！", prefix);
    }

    // 查找目标用户
    let target_id = find_user_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n未找到玩家[{}]！", prefix, target_name);
    }

    if target_id == user_id {
        return format!("{}\n不能踢出自己！", prefix);
    }

    let target_guild = db.read_basic(&target_id, ITEM_GUILD);
    if target_guild != guild {
        return format!("{}\n对方不是本公会成员！", prefix);
    }

    // 从成员列表移除
    let members_str = db.global_get("UnionData", &format!("{}.Members", &guild));
    let mut members: Vec<String> = members_str
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    members.retain(|m| m != &target_id);
    db.global_set("UnionData", &format!("{}.Members", &guild), &members.join(","));
    db.write_basic(&target_id, ITEM_GUILD, EMPTY);

    format!("{}\n已将[{}]踢出公会[{}]！", prefix, target_name, guild)
}

/// 申请加入公会
pub fn apply_guild(db: &Database, user_id: &str, guild_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current_guild = db.read_basic(user_id, ITEM_GUILD);
    if !current_guild.is_empty() {
        return format!("{}\n您已在公会[{}]中，需要先退出！", prefix, current_guild);
    }

    // 检查公会是否存在
    let owner = db.global_get("UnionData", &format!("{}.Owner", guild_name));
    if owner.is_empty() {
        return format!("{}\n公会[{}]不存在！", prefix, guild_name);
    }

    // 添加到申请列表
    let applications = db.global_get("UnionData", &format!("{}.Applications", guild_name));
    let new_app = if applications.is_empty() {
        user_id.to_string()
    } else {
        format!("{},{}", applications, user_id)
    };
    db.global_set("UnionData", &format!("{}.Applications", guild_name), &new_app);

    format!("{}\n已向公会[{}]提交加入申请，等待会长批准！", prefix, guild_name)
}

/// 批准加入公会
pub fn approve_guild(db: &Database, user_id: &str, target_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n您不是公会会长！", prefix);
    }

    // 查找目标用户
    let target_id = find_user_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n未找到玩家[{}]！", prefix, target_name);
    }

    // 检查目标是否已有公会
    let target_guild = db.read_basic(&target_id, ITEM_GUILD);
    if !target_guild.is_empty() {
        return format!("{}\n[{}]已在公会[{}]中！", prefix, target_name, target_guild);
    }

    // 检查是否在申请列表或邀请列表
    let applications = db.global_get("UnionData", &format!("{}.Applications", guild));
    let mut app_list: Vec<String> = applications
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let invites = db.global_get("UnionData", &format!("{}.Invites", guild));
    let mut invite_list: Vec<String> = invites
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let in_applications = app_list.contains(&target_id);
    let in_invites = invite_list.contains(&target_id);

    if !in_applications && !in_invites {
        return format!("{}\n[{}]未申请或被邀请加入公会！", prefix, target_name);
    }

    // 从列表移除
    if in_applications {
        app_list.retain(|m| m != &target_id);
        db.global_set("UnionData", &format!("{}.Applications", &guild), &app_list.join(","));
    }
    if in_invites {
        invite_list.retain(|m| m != &target_id);
        db.global_set("UnionData", &format!("{}.Invites", &guild), &invite_list.join(","));
    }

    // 添加到成员列表
    let members = db.global_get("UnionData", &format!("{}.Members", &guild));
    let new_members = if members.is_empty() {
        target_id.to_string()
    } else {
        format!("{},{}", members, target_id)
    };
    db.global_set("UnionData", &format!("{}.Members", &guild), &new_members);
    db.write_basic(&target_id, ITEM_GUILD, &guild);
    // 成就追踪
    crate::achievement::on_guild_joined(db, &target_id);

    let source = if in_invites { "邀请" } else { "申请" };
    format!(
        "{}\n已批准[{}]加入公会[{}]！（通过{}）",
        prefix, target_name, guild, source
    )
}

/// 拒绝加入公会
pub fn reject_guild(db: &Database, user_id: &str, target_name: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    let owner = db.global_get("UnionData", &format!("{}.Owner", guild));
    if owner != user_id {
        return format!("{}\n您不是公会会长！", prefix);
    }

    let target_id = find_user_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n未找到玩家[{}]！", prefix, target_name);
    }

    // 从申请列表移除
    let applications = db.global_get("UnionData", &format!("{}.Applications", guild));
    let mut app_list: Vec<String> = applications
        .split(',')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    app_list.retain(|m| m != &target_id);
    db.global_set("UnionData", &format!("{}.Applications", &guild), &app_list.join(","));

    format!("{}\n已拒绝[{}]加入公会[{}]！", prefix, target_name, guild)
}

// ==================== 位置/查看系统 ====================

/// 查看目标详情
pub fn view_target(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target = db.read_user_data(user_id, "target");
    if target.is_empty() {
        return format!("{}\n您没有锁定任何目标！", prefix);
    }

    if target.starts_with("monster_") {
        let monster_id = target.replace("monster_", "");
        // 简化处理：显示怪物ID
        let hp = db.read_user_data(user_id, "target_hp");
        let max_hp = db.read_user_data(user_id, "target_max_hp");
        format!(
            "{}\n═══ 锁定目标 ═══\n类型：怪物\nID：{}\nHP：{}/{}",
            prefix, monster_id, hp, max_hp
        )
    } else {
        // 玩家目标
        let target_name = user::get_msg_prefix(db, &target);
        let level = db.read_basic(&target, ITEM_LEVEL);
        let occupation = db.read_basic(&target, ITEM_OCCUPATION);
        let hp = db.read_basic(&target, ITEM_HP);
        format!(
            "{}\n═══ 锁定目标 ═══\n名称：{}\n等级：{}\n职业：{}\nHP：{}",
            prefix, target_name, level, occupation, hp
        )
    }
}

/// 查看同地图玩家
pub fn view_map_players(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current_map = db.read_basic(user_id, ITEM_LOCATION);

    // 查询同地图玩家 - Basic_User 表结构是 (ID, Node, Item, Data)
    let players: Vec<String> = {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT ID FROM Basic_User WHERE Item='位置' AND Data=?1 AND ID!=?2")
            .unwrap();
        stmt.query_map([&current_map, user_id], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };

    if players.is_empty() {
        return format!("{}\n当前地图[{}]没有其他玩家！", prefix, current_map);
    }

    let mut r = format!("{}\n═══ 地图[{}]的玩家 ({}) ═══", prefix, current_map, players.len());
    for (i, p) in players.iter().enumerate() {
        let name = user::get_msg_prefix(db, p);
        let level = db.read_basic(p, ITEM_LEVEL);
        let occupation = db.read_basic(p, ITEM_OCCUPATION);
        r.push_str(&format!("\n{}. {} Lv.{} [{}]", i + 1, name, level, occupation));
    }
    r
}

/// 喊话
pub fn shout(db: &Database, user_id: &str, message: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level < 10 {
        return format!("{}\n等级不足10级，无法使用喊话功能！", prefix);
    }

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    let cost = 100;
    if gold < cost {
        return format!("{}\n喊话需要{}金币，余额不足！", prefix, cost);
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, cost);

    // 保存喊话到全局
    let shout_count: i64 = db.global_get("Shout", "Count").parse().unwrap_or(0);
    let new_count = shout_count + 1;
    db.global_set(
        "Shout",
        &format!("msg_{}", new_count),
        &format!("{}:{}", prefix, message),
    );
    db.global_set("Shout", "Count", &new_count.to_string());

    crate::achievement::on_shout(db, user_id);
    format!("{}\n📢 喊话成功！\n消耗{}金币\n内容：{}", prefix, cost, message)
}

// ==================== 辅助函数 ====================

/// 根据昵称查找用户ID
fn find_user_by_name(db: &Database, name: &str) -> String {
    let conn = db.lock_conn();
    // 新用户：Node='基础信息', Item='名称'
    let result = conn
        .prepare("SELECT ID FROM Basic_User WHERE Node='基础信息' AND Item='名称' AND Data=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([name], |row| row.get::<_, String>(0))
                .ok()
                .and_then(|mut rows| rows.next())
                .and_then(|r| r.ok())
        });
    if let Some(id) = result {
        return id;
    }
    // 老用户：Node='Basic', Item='Name'
    let result = conn
        .prepare("SELECT ID FROM Basic_User WHERE Node='Basic' AND Item='Name' AND Data=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([name], |row| row.get::<_, String>(0))
                .ok()
                .and_then(|mut rows| rows.next())
                .and_then(|r| r.ok())
        });
    result.unwrap_or_default()
}

// ==================== 匹配/竞技系统 ====================

/// 匹配竞技段位信息
struct MatchTier {
    name: String,
    integral_min: i32,
    integral_max: i32,
    reward_participation: String,
    reward_victory: String,
    reward_fail: String,
    reward_tied: String,
    monster_name: String,
}

/// 解析奖励字符串 (格式: 字段GH值GI字段GH值)
fn parse_reward_value(reward_str: &str) -> (i32, i64) {
    // 返回 (积分变化, 金币变化)
    let mut integral_delta: i32 = 0;
    let mut gold_delta: i64 = 0;
    let cleaned = reward_str.replace(['\x01', '\x00'], "");
    for part in cleaned.split("GI") {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = part.find("GH") {
            let field = &part[..pos];
            let val_str = &part[pos + 2..];
            let val = parse_reward_int(val_str);
            match field {
                "积分" => integral_delta += val,
                "金币" => gold_delta += val as i64,
                "胜点" => {} // 胜点暂不处理
                _ => {}
            }
        }
    }
    (integral_delta, gold_delta)
}

/// 解析奖励数值，处理 hex 编码和 Q 终止符
fn parse_reward_int(val_str: &str) -> i32 {
    let val_str = val_str.trim();
    // 先尝试 hex 解码（偶数位纯 hex 字符串可能是 ASCII hex 编码）
    if val_str.len() >= 4 && val_str.len().is_multiple_of(2) && val_str.chars().all(|c| c.is_ascii_hexdigit()) {
        if let Ok(bytes) = hex::decode(val_str) {
            if let Ok(decoded) = std::str::from_utf8(&bytes) {
                let decoded = decoded.trim_end_matches('\0');
                // 只有解码结果是可打印 ASCII 才使用
                if decoded.chars().all(|c| c.is_ascii_graphic() || c == '-' || c == '+') {
                    let stripped = decoded.trim_end_matches('Q').trim_end_matches('q');
                    if let Ok(v) = stripped.parse::<i32>() {
                        return v;
                    }
                }
            }
        }
    }
    // 字面值解析
    let stripped = val_str.trim_end_matches('Q').trim_end_matches('q');
    stripped.parse::<i32>().unwrap_or(0)
}

/// 获取玩家匹配积分
fn get_match_integral(db: &Database, user_id: &str) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT Integral FROM ext_pipei_uInfo WHERE uID=?1")
        .ok()
        .and_then(|mut stmt| {
            stmt.query_map([user_id], |row| row.get::<_, i32>(0))
                .ok()
                .and_then(|mut rows| rows.next())
                .and_then(|r| r.ok())
        })
        .unwrap_or(0)
}

/// 设置玩家匹配积分
fn set_match_integral(db: &Database, user_id: &str, integral: i32) {
    let conn = db.lock_conn();
    // 先尝试更新
    let updated = conn.execute(
        "UPDATE ext_pipei_uInfo SET Integral=?1 WHERE uID=?2",
        rusqlite::params![integral, user_id],
    );
    if updated.unwrap_or(0) == 0 {
        // 不存在则插入
        let _ = conn.execute(
            "INSERT INTO ext_pipei_uInfo (uID, Integral) VALUES (?1, ?2)",
            rusqlite::params![user_id, integral],
        );
    }
}

/// 记录匹配日志
fn log_match(db: &Database, user_id: &str, opponent: &str, obj_type: i32, result: i32) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let conn = db.lock_conn();
    let _ = conn.execute(
        "INSERT INTO ext_pipei_log (uID, oTime, Object, ObjectType, Result) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![user_id, now, opponent, obj_type, result],
    );
}

/// 获取匹配段位
fn get_match_tiers(db: &Database) -> Vec<MatchTier> {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT Name, Integral_min, Integral_max, RAP_Rew, RAP_Victory, RAP_Fail, RAP_Tied, MonsterName FROM ext_pipei_paragraph ORDER BY Integral_min"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    stmt.query_map([], |row| {
        Ok(MatchTier {
            name: row.get::<_, String>(0)?.trim_end_matches('\x00').to_string(),
            integral_min: row.get::<_, i32>(1)?,
            integral_max: row.get::<_, i32>(2)?,
            reward_participation: row.get::<_, String>(3)?.trim_end_matches('\x00').to_string(),
            reward_victory: row.get::<_, String>(4)?.trim_end_matches('\x00').to_string(),
            reward_fail: row.get::<_, String>(5)?.trim_end_matches('\x00').to_string(),
            reward_tied: row.get::<_, String>(6)?.trim_end_matches('\x00').to_string(),
            monster_name: row.get::<_, String>(7)?.trim_end_matches('\x00').to_string(),
        })
    })
    .unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

/// 根据积分获取段位
fn find_tier(tiers: &[MatchTier], integral: i32) -> Option<&MatchTier> {
    tiers
        .iter()
        .find(|t| integral >= t.integral_min && integral <= t.integral_max)
}

/// 匹配竞技
pub fn match_arena(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);

    // 检查生命
    if info.hp <= 0 {
        return format!("{}\n生命值不足，无法进行匹配！请先恢复生命。", prefix);
    }

    let integral = get_match_integral(db, user_id);
    let tiers = get_match_tiers(db);
    if tiers.is_empty() {
        return format!("{}\n匹配系统暂未开放！", prefix);
    }

    let tier = match find_tier(&tiers, integral) {
        Some(t) => t,
        None => {
            // 默认最低段位
            &tiers[0]
        }
    };

    let tier_name = &tier.name;
    let monster_name = &tier.monster_name;

    // 获取怪物数据
    let monster = db.monster_get(monster_name);
    if monster.is_none() {
        return format!("{}\n匹配对手数据异常，请联系管理员。", prefix);
    }
    let monster = monster.unwrap();

    // 简化战斗：基于属性计算胜负
    let user_power = info.ad + info.ap + info.defense + info.hit;
    let monster_power = monster.ad + monster.ap + monster.defense + monster.hit;
    let hp_ratio = info.hp as f64 / info.hp_max.max(1) as f64;

    // 使用随机数决定战斗结果
    let mut rng = rand::thread_rng();
    let user_roll: f64 = rng.gen_range(0.0..100.0) * hp_ratio;
    let monster_roll: f64 = rng.gen_range(0.0..100.0);
    let user_score = user_roll + user_power as f64 * 0.5;
    let monster_score = monster_roll + monster_power as f64 * 0.3;

    let (result_code, result_label, reward_str) = if user_score > monster_score * 1.2 {
        (1, "胜利", &tier.reward_victory)
    } else if user_score > monster_score * 0.8 {
        (0, "平局", &tier.reward_tied)
    } else {
        (-1, "失败", &tier.reward_fail)
    };

    // 解析并发放奖励
    let (mut integral_delta, gold_delta) = parse_reward_value(reward_str);

    // 参与奖励
    let (rew_integral, rew_gold) = parse_reward_value(&tier.reward_participation);
    integral_delta += rew_integral;

    let new_integral = (integral + integral_delta).max(0);
    set_match_integral(db, user_id, new_integral);

    if gold_delta > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_delta);
    }
    if rew_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, rew_gold);
    }

    // 成就追踪：PVP胜利
    if result_code == 1 {
        crate::achievement::on_pvp_win(db, user_id);
        crate::weekly_quest::on_pvp_win(db, user_id);
    }

    // 扣血（根据战斗结果）
    let damage = if result_code < 0 {
        (monster.ad * 3).max(10)
    } else if result_code == 0 {
        (monster.ad).max(5)
    } else {
        (monster.ad / 2).max(1)
    };
    let new_hp = (info.hp - damage).max(0);
    db.write_basic_int(user_id, ITEM_HP_CURRENT, new_hp);

    // 记录日志
    log_match(db, user_id, monster_name, 1, result_code);

    // 计算积分变化描述
    let integral_text = if integral_delta > 0 {
        format!("+{}", integral_delta)
    } else {
        integral_delta.to_string()
    };

    let mut result = format!(
        "{}\n═══ 匹配竞技 ═══\n段位：{}\n对手：[{}]\n\n⚔️ 战斗结果：{}！\n\n📊 积分：{} ({}→{})\n💰 金币：+{}",
        prefix,
        tier_name,
        monster_name,
        result_label,
        integral_text,
        integral,
        new_integral,
        gold_delta + rew_gold
    );

    if damage > 0 {
        result.push_str(&format!("\n❤️ 受到伤害：-{} (剩余HP：{})", damage, new_hp));
    }

    if new_hp <= 0 {
        result.push_str("\n\n⚠️ 您在竞技中阵亡，请恢复生命后再来！");
    }

    result
}

/// 匹配信息
pub fn match_info(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let integral = get_match_integral(db, user_id);

    let tiers = get_match_tiers(db);
    if tiers.is_empty() {
        return format!("{}\n匹配系统暂未开放！", prefix);
    }
    let tier = find_tier(&tiers, integral).unwrap_or(&tiers[0]);
    let tier_name = &tier.name;

    // 统计胜/负/平
    let (wins, losses, draws): (i32, i32, i32) = {
        let conn = db.lock_conn();
        let wins = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_log WHERE uID=?1 AND Result=1")
            .ok()
            .and_then(|mut s| s.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        let losses = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_log WHERE uID=?1 AND Result=-1")
            .ok()
            .and_then(|mut s| s.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        let draws = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_log WHERE uID=?1 AND Result=0")
            .ok()
            .and_then(|mut s| s.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        (wins, losses, draws)
    };

    // 下一段位信息
    let next_tier = tiers.iter().find(|t| t.integral_min > integral);
    let upgrade_info = if let Some(next) = next_tier {
        format!("\n下一段位：[{}] (需要{}积分)", next.name, next.integral_min)
    } else {
        "\n已达到最高段位！".to_string()
    };

    let total = wins + losses + draws;
    let win_rate = if total > 0 {
        wins as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    // 使用模板渲染战绩统计
    let occupation = db.read_basic(user_id, crate::core::ITEM_OCCUPATION);
    let stats_section = crate::template_render::render_match_stats(
        db,
        user_id,
        &occupation,
        total,
        wins,
        losses,
        draws,
        win_rate,
        integral,
    );

    format!(
        "{}\n═══ 匹配竞技信息 ═══\n当前积分：{}\n当前段位：{}\n\n📊 战绩统计\n{}{}\n\n输入'匹配'开始竞技",
        prefix, integral, tier_name, stats_section, upgrade_info
    )
}

/// 匹配排行
pub fn match_ranking(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let players: Vec<(String, i32)> = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare("SELECT uID, Integral FROM ext_pipei_uInfo ORDER BY Integral DESC LIMIT 10") {
            Ok(s) => s,
            Err(_) => return format!("{}\n暂无匹配排行数据！", prefix),
        };
        let mut rows_vec = Vec::new();
        if let Ok(rows) = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))) {
            for r in rows.flatten() {
                rows_vec.push(r);
            }
        }
        rows_vec
    };

    if players.is_empty() {
        return format!("{}\n暂无匹配排行数据！", prefix);
    }

    let tiers = get_match_tiers(db);
    let mut r = format!("{}\n═══ 匹配竞技排行 ═══", prefix);
    for (i, (uid, integral)) in players.iter().enumerate() {
        let uid_clean = uid.trim_end_matches('\u{0000}');
        let name = user::get_msg_prefix(db, uid_clean);
        let tier = find_tier(&tiers, *integral);
        let tier_name = tier.map(|t| t.name.as_str()).unwrap_or("无");
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        r.push_str(&format!(
            "\n{}{}. {} - {}分 [{}]",
            medal,
            i + 1,
            name,
            integral,
            tier_name
        ));
    }
    r
}

// ==================== 匹配队列系统 ====================
// 使用 ext_pipei_list 表: uID, jTime, Integral_min, Integral_max
// 玩家加入队列后等待积分相近的对手自动匹配

/// 匹配队列等待超时（秒）
const MATCH_QUEUE_TIMEOUT_SECS: i64 = 300;
/// 积分匹配范围基础值
const MATCH_INTEGRAL_RANGE: i32 = 50;

/// 加入匹配队列
pub fn match_queue_join(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 检查生命
    let info = user::calc_total_attrs(db, user_id);
    if info.hp <= 0 {
        return format!("{}\n生命值不足，无法加入匹配队列！请先恢复生命。", prefix);
    }

    // 检查是否已在队列中
    {
        let conn = db.lock_conn();
        let in_queue = conn
            .prepare("SELECT COUNT(*) FROM ext_pipei_list WHERE uID=?1")
            .ok()
            .and_then(|mut stmt| stmt.query_row([user_id], |row| row.get::<_, i32>(0)).ok())
            .unwrap_or(0);
        if in_queue > 0 {
            return format!(
                "{}\n您已在匹配队列中！请等待对手匹配。\n发送「退出匹配队列」可退出。",
                prefix
            );
        }
    }

    let integral = get_match_integral(db, user_id);
    let tiers = get_match_tiers(db);
    let default_tier = MatchTier {
        name: String::new(),
        integral_min: 0,
        integral_max: 0,
        reward_participation: String::new(),
        reward_victory: String::new(),
        reward_fail: String::new(),
        reward_tied: String::new(),
        monster_name: String::new(),
    };
    let tier = find_tier(&tiers, integral).unwrap_or(&default_tier);

    let integral_min = (integral - MATCH_INTEGRAL_RANGE).max(0);
    let integral_max = integral + MATCH_INTEGRAL_RANGE;
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 插入队列
    {
        let conn = db.lock_conn();
        let _ = conn.execute(
            "INSERT INTO ext_pipei_list (uID, jTime, Integral_min, Integral_max) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![user_id, now, integral_min, integral_max],
        );
    }

    // 清理超时的队列条目
    clean_match_queue(db);

    // 统计当前队列人数
    let queue_count = get_match_queue_count(db);

    let tier_name = if tier.name.is_empty() { "未定级" } else { &tier.name };

    format!(
        "{}\n═══ 加入匹配队列 ═══\n\n✅ 已加入匹配队列！\n📊 当前积分：{}\n🏅 当前段位：{}\n🔍 匹配范围：{}~{}\n👥 当前队列人数：{}\n\n⏳ 等待对手中...\n💡 匹配范围会自动扩大（每30秒+20分）\n\n发送「退出匹配队列」可退出",
        prefix, integral, tier_name, integral_min, integral_max, queue_count
    )
}

/// 退出匹配队列
pub fn match_queue_leave(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let conn = db.lock_conn();
    let deleted = conn.execute("DELETE FROM ext_pipei_list WHERE uID=?1", rusqlite::params![user_id]);

    match deleted {
        Ok(n) if n > 0 => format!("{}\n✅ 已退出匹配队列。", prefix),
        _ => format!("{}\n您不在匹配队列中。", prefix),
    }
}

/// 查看匹配队列
pub fn match_queue_view(db: &Database, user_id: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 清理超时条目
    clean_match_queue(db);

    let integral = get_match_integral(db, user_id);
    let tiers = get_match_tiers(db);
    let tier = find_tier(&tiers, integral);

    // 获取队列中的玩家
    let queue_entries: Vec<(String, String, i32, i32)> = {
        let conn = db.lock_conn();
        let mut stmt =
            match conn.prepare("SELECT uID, jTime, Integral_min, Integral_max FROM ext_pipei_list ORDER BY jTime") {
                Ok(s) => s,
                Err(_) => return format!("{}\n匹配队列暂不可用！", prefix),
            };
        let mut entries = Vec::new();
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
            ))
        }) {
            for r in rows.flatten() {
                entries.push(r);
            }
        }
        entries
    };

    let in_queue = queue_entries.iter().any(|(uid, _, _, _)| uid == user_id);

    let mut result = format!("{}\n═══ 匹配队列 ═══\n\n", prefix);

    // 当前状态
    let tier_name = tier.map(|t| t.name.as_str()).unwrap_or("未定级");
    result.push_str(&format!("📊 您的积分：{} [{}]\n", integral, tier_name));
    result.push_str(&format!("👥 队列人数：{}\n", queue_entries.len()));
    if in_queue {
        result.push_str("🟢 您的状态：等待匹配中\n");
    } else {
        result.push_str("⚪ 您的状态：未加入\n");
    }

    // 队列列表
    if queue_entries.is_empty() {
        result.push_str("\n📭 当前无人在匹配队列中\n");
    } else {
        result.push_str(&format!("\n📋 队列列表 (共{}人):\n", queue_entries.len()));
        for (i, (uid, jtime, min_int, max_int)) in queue_entries.iter().enumerate() {
            let name = encoding::smart_decode(&db.read_basic(uid, ITEM_NAME));
            let player_integral = get_match_integral(db, uid);
            let is_self = uid == user_id;
            let marker = if is_self { " ← 您" } else { "" };

            // 计算等待时间
            let wait_text = calc_wait_time(jtime);

            // 判断积分是否在匹配范围内
            let can_match = integral >= *min_int && integral <= *max_int;
            let match_icon = if is_self {
                ""
            } else if can_match {
                " ✅可匹配"
            } else {
                " ❌"
            };

            result.push_str(&format!(
                "\n{}{}. {} ({}分) [{}-{}] 等待{}{}{}",
                if i == 0 { "🔥" } else { "  " },
                i + 1,
                name,
                player_integral,
                min_int,
                max_int,
                wait_text,
                match_icon,
                marker,
            ));
        }

        // 匹配对手提示
        let potential_matches: Vec<_> = queue_entries
            .iter()
            .filter(|(uid, _, min_int, max_int)| uid != user_id && integral >= *min_int && integral <= *max_int)
            .collect();

        if !in_queue && !potential_matches.is_empty() {
            result.push_str(&format!(
                "\n\n🎯 发现{}个可匹配对手！发送「加入匹配队列」开始匹配",
                potential_matches.len()
            ));
        }
    }

    result.push_str("\n\n💡 发送「加入匹配队列」/ 「退出匹配队列」操作");
    result
}

/// 清理超时的匹配队列条目
fn clean_match_queue(db: &Database) {
    let cutoff = chrono::Local::now()
        .checked_sub_signed(chrono::Duration::seconds(MATCH_QUEUE_TIMEOUT_SECS))
        .unwrap_or_else(chrono::Local::now)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let conn = db.lock_conn();
    let _ = conn.execute("DELETE FROM ext_pipei_list WHERE jTime < ?1", rusqlite::params![cutoff]);
}

/// 获取匹配队列人数
fn get_match_queue_count(db: &Database) -> i32 {
    let conn = db.lock_conn();
    conn.prepare("SELECT COUNT(*) FROM ext_pipei_list")
        .ok()
        .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i32>(0)).ok())
        .unwrap_or(0)
}

/// 计算等待时间描述
fn calc_wait_time(jtime_str: &str) -> String {
    if let Ok(join_time) = chrono::NaiveDateTime::parse_from_str(jtime_str, "%Y-%m-%d %H:%M:%S") {
        let now = chrono::Local::now().naive_local();
        let elapsed = now.signed_duration_since(join_time);
        let secs = elapsed.num_seconds().max(0);
        if secs < 60 {
            format!("{}秒", secs)
        } else if secs < 3600 {
            format!("{}分{}秒", secs / 60, secs % 60)
        } else {
            format!("{}小时{}分", secs / 3600, (secs % 3600) / 60)
        }
    } else {
        "未知".to_string()
    }
}

// ==================== 邀请加入公会 ====================

/// 邀请玩家加入公会（会长/成员邀请其他玩家）
pub fn cmd_invite_guild(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!("{}\n请指定要邀请的玩家。\n用法：邀请加入+玩家昵称", prefix);
    }

    // 检查自己是否有公会
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    // 查找目标用户
    let target_id = find_user_by_name(db, target_name);
    if target_id.is_empty() {
        return format!("{}\n未找到玩家[{}]！", prefix, target_name);
    }

    if target_id == user_id {
        return format!("{}\n不能邀请自己！", prefix);
    }

    // 检查目标是否已有公会
    let target_guild = db.read_basic(&target_id, ITEM_GUILD);
    if !target_guild.is_empty() {
        return format!("{}\n[{}]已在公会[{}]中！", prefix, target_name, target_guild);
    }

    // 添加到公会邀请列表（使用 Global 表存储）
    let invites_key = format!("{}.Invites", guild);
    let invites = db.global_get("UnionData", &invites_key);
    // 检查是否已邀请
    let invite_list: Vec<&str> = invites.split(',').filter(|s| !s.is_empty()).collect();
    if invite_list.contains(&target_id.as_str()) {
        return format!("{}\n已邀请过[{}]，请等待对方回应。", prefix, target_name);
    }

    let new_invites = if invites.is_empty() {
        target_id.to_string()
    } else {
        format!("{},{}", invites, target_id)
    };
    db.global_set("UnionData", &invites_key, &new_invites);

    format!(
        "{}\n已向[{}]发送公会[{}]的邀请！\n对方发送'批准加入+{}'即可接受邀请。",
        prefix, target_name, guild, guild
    )
}

// ==================== 公会捐赠排行 ====================

/// 公会捐赠排行 - 显示本公会成员捐赠贡献排名
pub fn cmd_guild_donation_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let guild = db.read_basic(user_id, ITEM_GUILD);
    if guild.is_empty() {
        return format!("{}\n您未加入任何公会！", prefix);
    }

    // 查询本公会捐赠记录，按用户聚合总捐赠
    let donors: Vec<(String, i64)> = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare(
            "SELECT User, SUM(AddExp) as Total FROM UnionContribution_Register WHERE UnionName=?1 GROUP BY User ORDER BY Total DESC LIMIT 10"
        ) {
            Ok(s) => s,
            Err(_) => return format!("{}\n暂无公会捐赠数据！", prefix),
        };
        let mut rows_vec = Vec::new();
        if let Ok(rows) = stmt.query_map(rusqlite::params![&guild], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }) {
            for r in rows.flatten() {
                rows_vec.push(r);
            }
        }
        rows_vec
    };

    if donors.is_empty() {
        return format!(
            "{}\n公会[{}]暂无捐赠记录！\n发送'公会捐献+金额'即可为公会做贡献。",
            prefix, guild
        );
    }

    let mut result = format!("{}\n【{}】捐赠排行榜", prefix, guild);
    let mut entries: Vec<(usize, String, i64)> = Vec::new();
    for (i, (uid_raw, total)) in donors.iter().enumerate() {
        let uid_clean = uid_raw.trim_end_matches('\u{0}');
        let name = encoding::smart_decode(&db.read_basic(uid_clean, ITEM_NAME));
        entries.push((i + 1, name, *total));
    }
    result.push_str(&crate::template_render::render_guild_donation_ranking(db, &entries));
    result.push_str(
        "

发送'公会捐献+金额'即可为公会做贡献。",
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reward_int_plain() {
        assert_eq!(parse_reward_int("100"), 100);
    }

    #[test]
    fn test_parse_reward_int_negative() {
        assert_eq!(parse_reward_int("-50"), -50);
    }

    #[test]
    fn test_parse_reward_int_zero() {
        assert_eq!(parse_reward_int("0"), 0);
    }

    #[test]
    fn test_parse_reward_int_invalid() {
        assert_eq!(parse_reward_int("abc"), 0);
    }

    #[test]
    fn test_parse_reward_int_empty() {
        assert_eq!(parse_reward_int(""), 0);
    }

    #[test]
    fn test_parse_reward_int_with_q_suffix() {
        assert_eq!(parse_reward_int("100Q"), 100);
    }

    #[test]
    fn test_parse_reward_int_with_spaces() {
        assert_eq!(parse_reward_int("  100  "), 100);
    }

    #[test]
    fn test_parse_reward_int_hex_encoded() {
        // "100" in hex is "313030"
        assert_eq!(parse_reward_int("313030"), 100);
    }

    #[test]
    fn test_parse_reward_value_empty() {
        let (integral, gold) = parse_reward_value("");
        assert_eq!(integral, 0);
        assert_eq!(gold, 0);
    }

    #[test]
    fn test_parse_reward_value_integral_only() {
        let (integral, gold) = parse_reward_value("积分GH10GI");
        assert_eq!(integral, 10);
        assert_eq!(gold, 0);
    }

    #[test]
    fn test_parse_reward_value_gold_only() {
        let (integral, gold) = parse_reward_value("金币GH500GI");
        assert_eq!(integral, 0);
        assert_eq!(gold, 500);
    }

    #[test]
    fn test_parse_reward_value_both() {
        let (integral, gold) = parse_reward_value("积分GH10GI金币GH500GI");
        assert_eq!(integral, 10);
        assert_eq!(gold, 500);
    }

    #[test]
    fn test_parse_reward_value_with_null_bytes() {
        let (integral, gold) = parse_reward_value("\x00积分GH10GI\x01");
        assert_eq!(integral, 10);
        assert_eq!(gold, 0);
    }

    #[test]
    fn test_parse_reward_value_victory_points_ignored() {
        let (integral, gold) = parse_reward_value("胜点GH5GI");
        assert_eq!(integral, 0);
        assert_eq!(gold, 0);
    }

    #[test]
    fn test_parse_reward_value_unknown_field_ignored() {
        let (integral, gold) = parse_reward_value("未知GH99GI");
        assert_eq!(integral, 0);
        assert_eq!(gold, 0);
    }

    #[test]
    fn test_find_tier_exact() {
        let tiers = vec![
            MatchTier {
                name: "青铜".to_string(),
                integral_min: 0,
                integral_max: 100,
                reward_participation: String::new(),
                reward_victory: String::new(),
                reward_fail: String::new(),
                reward_tied: String::new(),
                monster_name: String::new(),
            },
            MatchTier {
                name: "白银".to_string(),
                integral_min: 101,
                integral_max: 200,
                reward_participation: String::new(),
                reward_victory: String::new(),
                reward_fail: String::new(),
                reward_tied: String::new(),
                monster_name: String::new(),
            },
        ];
        let tier = find_tier(&tiers, 50);
        assert!(tier.is_some());
        assert_eq!(tier.unwrap().name, "青铜");
    }

    #[test]
    fn test_find_tier_boundary() {
        let tiers = vec![MatchTier {
            name: "青铜".to_string(),
            integral_min: 0,
            integral_max: 100,
            reward_participation: String::new(),
            reward_victory: String::new(),
            reward_fail: String::new(),
            reward_tied: String::new(),
            monster_name: String::new(),
        }];
        assert_eq!(find_tier(&tiers, 0).unwrap().name, "青铜");
        assert_eq!(find_tier(&tiers, 100).unwrap().name, "青铜");
        assert!(find_tier(&tiers, 101).is_none());
    }

    #[test]
    fn test_find_tier_empty() {
        let tiers: Vec<MatchTier> = vec![];
        assert!(find_tier(&tiers, 0).is_none());
    }

    // ==================== 匹配队列测试 ====================

    #[test]
    fn test_calc_wait_time_seconds() {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let result = calc_wait_time(&now);
        assert!(result.contains("秒"), "Expected seconds format, got: {}", result);
    }

    #[test]
    fn test_calc_wait_time_minutes() {
        let past = (chrono::Local::now() - chrono::Duration::seconds(125))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let result = calc_wait_time(&past);
        assert!(result.contains("分"), "Expected minutes format, got: {}", result);
    }

    #[test]
    fn test_calc_wait_time_invalid() {
        let result = calc_wait_time("not-a-date");
        assert_eq!(result, "未知");
    }

    #[test]
    fn test_match_queue_constants() {
        assert_eq!(MATCH_QUEUE_TIMEOUT_SECS, 300);
        assert_eq!(MATCH_INTEGRAL_RANGE, 50);
    }

    #[test]
    fn test_calc_wait_time_long_duration() {
        let past = (chrono::Local::now() - chrono::Duration::seconds(7384))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let result = calc_wait_time(&past);
        assert!(result.contains("小时"), "Expected hours format, got: {}", result);
    }
}

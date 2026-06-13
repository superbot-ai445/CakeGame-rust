/// CakeGame 公会联盟系统
///
/// 允许公会之间组建联盟，共享联盟增益、联盟频道、联盟战等。
///
/// 功能:
/// - 创建联盟 (会长操作，消耗金币)
/// - 邀请公会加入联盟
/// - 接受/拒绝联盟邀请
/// - 退出联盟
/// - 解散联盟 (盟主操作)
/// - 联盟列表
/// - 联盟信息 (成员公会、联盟等级、联盟资金)
/// - 联盟捐献 (增加联盟资金)
/// - 联盟排行 (按联盟等级/资金排名)
///
/// 数据存储: Global 表 SECTION='guild_alliance'
///   key: alliance_{id}            = 联盟基本信息 JSON
///   key: alliance_invite_{guild}  = 待处理邀请
///   key: guild_alliance_{guild}   = 公会所属联盟 ID
///
/// 联盟等级:
///   1级 (0资金)     - 基础联盟，+2% 全属性
///   2级 (5000资金)  - 铜牌联盟，+4% 全属性
///   3级 (20000资金) - 银牌联盟，+6% 全属性
///   4级 (50000资金) - 金牌联盟，+8% 全属性
///   5级 (150000资金)- 钻石联盟，+10% 全属性
use crate::db::Database;
use crate::user;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 联盟最大成员公会数
const MAX_ALLIANCE_MEMBERS: usize = 5;

/// 联盟创建费用 (金币)
const CREATE_COST_GOLD: i64 = 100_000;

/// 联盟等级定义: (等级, 所需资金, 属性加成百分比, 等级名, emoji)
const ALLIANCE_LEVELS: &[(i32, i64, f64, &str, &str)] = &[
    (1, 0, 0.02, "初级联盟", "🥉"),
    (2, 5_000, 0.04, "铜牌联盟", "🥈"),
    (3, 20_000, 0.06, "银牌联盟", "🏅"),
    (4, 50_000, 0.08, "金牌联盟", "🥇"),
    (5, 150_000, 0.10, "钻石联盟", "💎"),
];

/// 联盟数据结构
#[derive(Debug, Clone)]
struct Alliance {
    id: String,
    name: String,
    /// 盟主公会名
    leader_guild: String,
    /// 成员公会列表 (逗号分隔)
    member_guilds: String,
    /// 联盟资金
    fund: i64,
    /// 联盟等级 (1-5)
    level: i32,
    /// 创建时间
    created_at: String,
    /// 联盟公告
    notice: String,
}

impl Alliance {
    fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","name":"{}","leader":"{}","members":"{}","fund":{},"level":{},"created":"{}","notice":"{}"}}"#,
            self.id,
            self.name,
            self.leader_guild,
            self.member_guilds,
            self.fund,
            self.level,
            self.created_at,
            self.notice
        )
    }

    fn from_json(json: &str) -> Option<Self> {
        // Simple JSON parsing (no serde dependency for this module)
        let id = extract_json_str(json, "id")?;
        let name = extract_json_str(json, "name")?;
        let leader_guild = extract_json_str(json, "leader")?;
        let member_guilds = extract_json_str(json, "members").unwrap_or_default();
        let fund = extract_json_i64(json, "fund").unwrap_or(0);
        let level = extract_json_i32(json, "level").unwrap_or(1);
        let created_at = extract_json_str(json, "created").unwrap_or_default();
        let notice = extract_json_str(json, "notice").unwrap_or_default();
        Some(Alliance {
            id,
            name,
            leader_guild,
            member_guilds,
            fund,
            level,
            created_at,
            notice,
        })
    }

    fn member_list(&self) -> Vec<&str> {
        self.member_guilds.split(',').filter(|s| !s.is_empty()).collect()
    }
}

/// 简易 JSON 字段提取
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_json_i64(json: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];
    let end = rest.find([',', '}'])?;
    rest[..end].trim().parse().ok()
}

fn extract_json_i32(json: &str, key: &str) -> Option<i32> {
    extract_json_i64(json, key).map(|v| v as i32)
}

/// 生成联盟 ID
fn gen_alliance_id(leader_guild: &str) -> String {
    let mut hasher = DefaultHasher::new();
    leader_guild.hash(&mut hasher);
    let hash = hasher.finish();
    format!("ALN-{:08X}", hash & 0xFFFFFFFF)
}

/// 获取当前联盟等级信息
fn get_level_info(level: i32) -> &'static (i32, i64, f64, &'static str, &'static str) {
    ALLIANCE_LEVELS
        .iter()
        .find(|(l, _, _, _, _)| *l == level)
        .unwrap_or(&ALLIANCE_LEVELS[0])
}

/// 根据资金计算联盟等级
fn calc_level(fund: i64) -> i32 {
    let mut lv = 1;
    for &(level, required, _, _, _) in ALLIANCE_LEVELS {
        if fund >= required {
            lv = level;
        }
    }
    lv
}

/// 格式化金币数字
fn format_gold(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 获取公会的联盟
fn get_guild_alliance(db: &Database, guild_name: &str) -> Option<Alliance> {
    let alliance_id = db.global_get("guild_alliance", &format!("guild_alliance_{}", guild_name));
    if alliance_id.is_empty() {
        return None;
    }
    let json = db.global_get("guild_alliance", &format!("alliance_{}", alliance_id));
    if json.is_empty() {
        return None;
    }
    Alliance::from_json(&json)
}

/// 保存联盟数据
fn save_alliance(db: &Database, alliance: &Alliance) {
    db.global_set(
        "guild_alliance",
        &format!("alliance_{}", alliance.id),
        &alliance.to_json(),
    );
    // 更新成员公会 → 联盟 ID 映射
    for guild in alliance.member_list() {
        db.global_set("guild_alliance", &format!("guild_alliance_{}", guild), &alliance.id);
    }
}

/// 创建联盟 (公会会长操作)
pub fn cmd_create_alliance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let nickname = db.read_basic(user_id, "NickName");
    let user_guild = db.read_basic(user_id, "unionName");

    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会，无法创建联盟。", prefix);
    }

    // 检查是否是公会会长
    let guild_info = db.global_get("set", &format!("guild_{}", user_guild));
    if guild_info.is_empty() {
        return format!("{}\n❌ 公会「{}」不存在。", prefix, user_guild);
    }

    // 简单检查: 创建者应该是公会会长
    let leader = extract_json_str(&guild_info, "leader").unwrap_or_default();
    if leader != nickname && leader != user_id {
        return format!("{}\n❌ 只有公会会长才能创建联盟。", prefix);
    }

    // 检查公会是否已在联盟中
    if get_guild_alliance(db, &user_guild).is_some() {
        return format!("{}\n❌ 公会「{}」已经在联盟中，不能重复创建。", prefix, user_guild);
    }

    let alliance_name = args.trim();
    if alliance_name.is_empty() {
        return format!(
            "{}\n📋 创建联盟\n\n用法: 创建联盟+联盟名\n创建费用: {} 金币\n\n💡 联盟可以容纳最多 {} 个公会，共享联盟增益。",
            prefix,
            format_gold(CREATE_COST_GOLD),
            MAX_ALLIANCE_MEMBERS
        );
    }

    if alliance_name.len() > 16 {
        return format!("{}\n❌ 联盟名不能超过16个字符。", prefix);
    }

    // 检查联盟名是否已存在 (遍历所有联盟)
    // 简化处理: 检查 Global 表中是否有同名联盟
    let existing = db.global_get("guild_alliance", "alliance_list");
    if existing.contains(alliance_name) {
        return format!("{}\n❌ 联盟名「{}」已被使用，请换一个名字。", prefix, alliance_name);
    }

    // 检查金币
    let gold: i64 = db.read_basic(user_id, "Gold").parse().unwrap_or(0);
    if gold < CREATE_COST_GOLD {
        return format!(
            "{}\n❌ 金币不足！创建联盟需要 {} 金币，您当前有 {} 金币。",
            prefix,
            format_gold(CREATE_COST_GOLD),
            format_gold(gold)
        );
    }

    // 扣除金币
    db.write_basic(user_id, "Gold", &(gold - CREATE_COST_GOLD).to_string());

    // 创建联盟
    let alliance_id = gen_alliance_id(&user_guild);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();

    let alliance = Alliance {
        id: alliance_id.clone(),
        name: alliance_name.to_string(),
        leader_guild: user_guild.clone(),
        member_guilds: user_guild.clone(),
        fund: 0,
        level: 1,
        created_at: now,
        notice: String::new(),
    };

    save_alliance(db, &alliance);

    // 更新联盟列表索引
    let list = db.global_get("guild_alliance", "alliance_list");
    let new_list = if list.is_empty() {
        alliance_id.clone()
    } else {
        format!("{},{}", list, alliance_id)
    };
    db.global_set("guild_alliance", "alliance_list", &new_list);

    let (_, _, bonus_pct, level_name, level_emoji) = get_level_info(1);

    format!(
        "{}\n🎉 联盟创建成功！\n\n\
         🏷️ 联盟名: {}\n\
         🔑 联盟ID: {}\n\
         👑 盟主公会: {}\n\
         {} 当前等级: {} ({:.0}% 全属性加成)\n\
         👥 成员上限: {} 个公会\n\
         💰 已消耗: {} 金币\n\n\
         💡 使用「邀请联盟+公会名」邀请其他公会加入联盟。",
        prefix,
        alliance_name,
        alliance_id,
        user_guild,
        level_emoji,
        level_name,
        bonus_pct * 100.0,
        MAX_ALLIANCE_MEMBERS,
        format_gold(CREATE_COST_GOLD)
    )
}

/// 邀请公会加入联盟 (盟主公会操作)
pub fn cmd_invite_alliance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let target_guild = args.trim();
    if target_guild.is_empty() {
        return format!(
            "{}\n📋 邀请公会加入联盟\n\n用法: 邀请联盟+公会名\n\n💡 只有盟主公会的会长可以邀请其他公会。",
            prefix
        );
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    // 检查用户的公会是否在联盟中
    let alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    // 检查是否是盟主公会
    if alliance.leader_guild != user_guild {
        return format!(
            "{}\n❌ 只有盟主公会「{}」的会长才能邀请其他公会。",
            prefix, alliance.leader_guild
        );
    }

    // 检查目标公会是否存在
    let target_info = db.global_get("set", &format!("guild_{}", target_guild));
    if target_info.is_empty() {
        return format!("{}\n❌ 公会「{}」不存在。", prefix, target_guild);
    }

    // 检查目标公会是否已在联盟中
    if alliance.member_list().contains(&target_guild) {
        return format!("{}\n❌ 公会「{}」已经在您的联盟中了。", prefix, target_guild);
    }

    // 检查联盟人数
    if alliance.member_list().len() >= MAX_ALLIANCE_MEMBERS {
        return format!("{}\n❌ 联盟已满（最多 {} 个公会）。", prefix, MAX_ALLIANCE_MEMBERS);
    }

    // 检查目标公会是否在其他联盟中
    if get_guild_alliance(db, target_guild).is_some() {
        return format!("{}\n❌ 公会「{}」已经在其他联盟中。", prefix, target_guild);
    }

    // 发送邀请
    db.global_set(
        "guild_alliance",
        &format!("alliance_invite_{}", target_guild),
        &format!(
            r#"{{"alliance_id":"{}","alliance_name":"{}","from":"{}","ts":"{}"}}"#,
            alliance.id,
            alliance.name,
            user_guild,
            chrono::Local::now().timestamp()
        ),
    );

    format!(
        "{}\n✅ 已向公会「{}」发送联盟邀请！\n\n\
         🏷️ 联盟: {}\n\
         📝 等待对方公会会长接受邀请。\n\
         💡 邀请有效期: 24小时",
        prefix, target_guild, alliance.name
    )
}

/// 接受联盟邀请
pub fn cmd_accept_alliance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    // 检查是否有邀请
    let invite_json = db.global_get("guild_alliance", &format!("alliance_invite_{}", user_guild));
    if invite_json.is_empty() {
        return format!("{}\n❌ 您的公会没有待处理的联盟邀请。", prefix);
    }

    let alliance_id = extract_json_str(&invite_json, "alliance_id").unwrap_or_default();
    let alliance_name = extract_json_str(&invite_json, "alliance_name").unwrap_or_default();

    // 检查邀请是否过期 (24小时)
    let ts = extract_json_i64(&invite_json, "ts").unwrap_or(0);
    let now = chrono::Local::now().timestamp();
    if now - ts > 86400 {
        db.global_set("guild_alliance", &format!("alliance_invite_{}", user_guild), "");
        return format!("{}\n❌ 邀请已过期（超过24小时）。", prefix);
    }

    // 获取联盟
    let alliance_json = db.global_get("guild_alliance", &format!("alliance_{}", alliance_id));
    let mut alliance = match Alliance::from_json(&alliance_json) {
        Some(a) => a,
        None => {
            db.global_set("guild_alliance", &format!("alliance_invite_{}", user_guild), "");
            return format!("{}\n❌ 联盟不存在或已解散。", prefix);
        }
    };

    // 再次检查人数
    if alliance.member_list().len() >= MAX_ALLIANCE_MEMBERS {
        db.global_set("guild_alliance", &format!("alliance_invite_{}", user_guild), "");
        return format!("{}\n❌ 联盟已满，无法加入。", prefix);
    }

    // 检查是否已在联盟中
    if get_guild_alliance(db, &user_guild).is_some() {
        db.global_set("guild_alliance", &format!("alliance_invite_{}", user_guild), "");
        return format!("{}\n❌ 您的公会已经在其他联盟中。", prefix);
    }

    // 加入联盟
    if alliance.member_guilds.is_empty() {
        alliance.member_guilds = user_guild.clone();
    } else {
        alliance.member_guilds = format!("{},{}", alliance.member_guilds, user_guild);
    }

    save_alliance(db, &alliance);

    // 清除邀请
    db.global_set("guild_alliance", &format!("alliance_invite_{}", user_guild), "");

    let members = alliance.member_list();

    format!(
        "{}\n🎉 公会「{}」已加入联盟「{}」！\n\n\
         👥 当前联盟成员: {} 个公会\n\
         📋 成员公会: {}\n\
         💡 使用「联盟信息」查看联盟详情。",
        prefix,
        user_guild,
        alliance_name,
        members.len(),
        members.join("、")
    )
}

/// 拒绝联盟邀请
pub fn cmd_reject_alliance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let invite_json = db.global_get("guild_alliance", &format!("alliance_invite_{}", user_guild));
    if invite_json.is_empty() {
        return format!("{}\n❌ 您的公会没有待处理的联盟邀请。", prefix);
    }

    let alliance_name = extract_json_str(&invite_json, "alliance_name").unwrap_or_else(|| "未知".to_string());

    // 清除邀请
    db.global_set("guild_alliance", &format!("alliance_invite_{}", user_guild), "");

    format!("{}\n✅ 已拒绝联盟「{}」的邀请。", prefix, alliance_name)
}

/// 退出联盟
pub fn cmd_leave_alliance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let mut alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    // 盟主公会不能退出，只能解散
    if alliance.leader_guild == user_guild {
        return format!(
            "{}\n❌ 盟主公会不能退出联盟，请使用「解散联盟」或将盟主转让给其他公会后再退出。",
            prefix
        );
    }

    // 从成员列表中移除
    let members: Vec<String> = alliance
        .member_list()
        .iter()
        .filter(|&&g| g != user_guild)
        .map(|&s| s.to_string())
        .collect();
    alliance.member_guilds = members.join(",");

    save_alliance(db, &alliance);

    // 清除公会→联盟映射
    db.global_set("guild_alliance", &format!("guild_alliance_{}", user_guild), "");

    format!("{}\n✅ 公会「{}」已退出联盟「{}」。", prefix, user_guild, alliance.name)
}

/// 解散联盟 (盟主操作)
pub fn cmd_disband_alliance(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    if alliance.leader_guild != user_guild {
        return format!(
            "{}\n❌ 只有盟主公会「{}」的会长才能解散联盟。",
            prefix, alliance.leader_guild
        );
    }

    let name = alliance.name.clone();

    // 清除所有成员公会的映射
    for guild in alliance.member_list() {
        db.global_set("guild_alliance", &format!("guild_alliance_{}", guild), "");
    }

    // 删除联盟数据
    db.global_set("guild_alliance", &format!("alliance_{}", alliance.id), "");

    // 从联盟列表中移除
    let list = db.global_get("guild_alliance", "alliance_list");
    let new_list: Vec<&str> = list.split(',').filter(|s| *s != alliance.id).collect();
    db.global_set("guild_alliance", "alliance_list", &new_list.join(","));

    format!("{}\n✅ 联盟「{}」已解散。\n所有成员公会已自动退出。", prefix, name)
}

/// 联盟信息
pub fn cmd_alliance_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    let (_, _, bonus_pct, level_name, level_emoji) = get_level_info(alliance.level);
    let members = alliance.member_list();

    // 计算升级进度
    let next_level = ALLIANCE_LEVELS.iter().find(|(l, _, _, _, _)| *l > alliance.level);
    let upgrade_info = if let Some((_, required, _, _, _)) = next_level {
        let pct = ((alliance.fund as f64 / *required as f64) * 100.0).min(100.0);
        let filled = (pct / 10.0) as usize;
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));
        format!(
            "📊 升级进度: {} {:.0}% (需要 {} 资金)",
            bar,
            pct,
            format_gold(*required)
        )
    } else {
        "🏆 已达最高等级！".to_string()
    };

    let notice = if alliance.notice.is_empty() {
        "暂无公告".to_string()
    } else {
        alliance.notice.clone()
    };

    format!(
        "{}\n🏛️ 联盟信息 — {}\n\n\
         🔑 联盟ID: {}\n\
         {} {} (等级 {})\n\
         👑 盟主公会: {}\n\
         💰 联盟资金: {}\n\
         ✨ 联盟加成: +{:.0}% 全属性\n\
         📅 创建时间: {}\n\n\
         👥 成员公会 ({}/{}):\n{}\n\n\
         {}\n\n\
         📢 联盟公告: {}",
        prefix,
        alliance.name,
        alliance.id,
        level_emoji,
        level_name,
        alliance.level,
        alliance.leader_guild,
        format_gold(alliance.fund),
        bonus_pct * 100.0,
        alliance.created_at,
        members.len(),
        MAX_ALLIANCE_MEMBERS,
        members
            .iter()
            .enumerate()
            .map(|(i, g)| {
                let tag = if *g == alliance.leader_guild { " 👑" } else { "" };
                format!("  {}. {}{}", i + 1, g, tag)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        upgrade_info,
        notice
    )
}

/// 联盟列表 (全服)
pub fn cmd_alliance_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let list_str = db.global_get("guild_alliance", "alliance_list");
    if list_str.is_empty() {
        return format!(
            "{}\n📋 全服联盟列表\n\n暂无联盟。使用「创建联盟」创建第一个联盟！",
            prefix
        );
    }

    let mut alliances: Vec<Alliance> = list_str
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|id| {
            let json = db.global_get("guild_alliance", &format!("alliance_{}", id));
            Alliance::from_json(&json)
        })
        .collect();

    // 按等级和资金排序
    alliances.sort_by(|a, b| b.level.cmp(&a.level).then(b.fund.cmp(&a.fund)));

    let mut lines: Vec<String> = vec![format!("📋 全服联盟列表 (共 {} 个联盟)\n", alliances.len())];

    for (i, a) in alliances.iter().enumerate().take(20) {
        let (_, _, _, _, emoji) = get_level_info(a.level);
        let member_count = a.member_list().len();
        lines.push(format!(
            "  {}. {} {} | {}级 | 资金:{} | 成员:{}个公会 | {}",
            i + 1,
            emoji,
            a.name,
            a.level,
            format_gold(a.fund),
            member_count,
            a.leader_guild
        ));
    }

    lines.push("\n💡 使用「联盟信息」查看您所在联盟的详情。".to_string());

    format!("{}\n{}", prefix, lines.join("\n"))
}

/// 联盟捐献 (增加联盟资金)
pub fn cmd_donate_alliance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let mut alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    let amount_str = args.trim();
    if amount_str.is_empty() {
        let _nickname = db.read_basic(user_id, "NickName");
        let gold: i64 = db.read_basic(user_id, "Gold").parse().unwrap_or(0);
        return format!(
            "{}\n📋 联盟捐献\n\n用法: 联盟捐献+金额\n\n\
             💰 您的金币: {}\n\
             🏛️ 当前联盟资金: {}\n\
             ✨ 捐献 1000 金币 = 1 联盟资金\n\n\
             💡 联盟资金可用于提升联盟等级，增强全联盟属性加成。",
            prefix,
            format_gold(gold),
            format_gold(alliance.fund)
        );
    }

    let amount: i64 = match amount_str.replace(",", "").parse() {
        Ok(a) if a > 0 => a,
        _ => return format!("{}\n❌ 请输入有效的捐献金额（正整数）。", prefix),
    };

    let gold: i64 = db.read_basic(user_id, "Gold").parse().unwrap_or(0);
    if gold < amount {
        return format!(
            "{}\n❌ 金币不足！您需要 {} 金币，当前只有 {} 金币。",
            prefix,
            format_gold(amount),
            format_gold(gold)
        );
    }

    // 扣除金币，增加联盟资金
    db.write_basic(user_id, "Gold", &(gold - amount).to_string());
    let fund_gain = amount / 1000;
    alliance.fund += fund_gain;

    // 检查是否升级
    let old_level = alliance.level;
    let new_level = calc_level(alliance.fund);
    alliance.level = new_level;

    save_alliance(db, &alliance);

    let (_, _, bonus_pct, level_name, level_emoji) = get_level_info(new_level);

    let level_up_msg = if new_level > old_level {
        format!(
            "\n\n🎊🎉 联盟等级提升！\n{} {} → {} {}\n✨ 全联盟属性加成提升至 {:.0}%！",
            get_level_info(old_level).4,
            get_level_info(old_level).3,
            level_emoji,
            level_name,
            bonus_pct * 100.0
        )
    } else {
        String::new()
    };

    // 记录捐献日志
    let nickname = db.read_basic(user_id, "NickName");
    let log_key = format!("donate_log_{}", alliance.id);
    let old_log = db.global_get("guild_alliance", &log_key);
    let entry = format!(
        "{}|{}|{}|{}",
        nickname,
        amount,
        fund_gain,
        chrono::Local::now().format("%m-%d %H:%M")
    );
    let new_log = if old_log.is_empty() {
        entry
    } else {
        // 保留最近20条
        let mut entries: Vec<&str> = old_log.split(';').collect();
        entries.insert(0, &entry);
        entries.truncate(20);
        // Need to own the strings
        let owned: Vec<String> = entries.iter().map(|s| s.to_string()).collect();
        owned.join(";")
    };
    db.global_set("guild_alliance", &log_key, &new_log);

    format!(
        "{}\n✅ 捐献成功！\n\n\
         💰 捐献金额: {} 金币\n\
         📈 增加联盟资金: {}\n\
         🏛️ 当前联盟资金: {}\n\
         {} {}{}",
        prefix,
        format_gold(amount),
        format_gold(fund_gain),
        format_gold(alliance.fund),
        level_emoji,
        level_name,
        level_up_msg
    )
}

/// 联盟排行
pub fn cmd_alliance_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let list_str = db.global_get("guild_alliance", "alliance_list");
    if list_str.is_empty() {
        return format!("{}\n🏆 联盟排行\n\n暂无联盟数据。", prefix);
    }

    let mut alliances: Vec<Alliance> = list_str
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|id| {
            let json = db.global_get("guild_alliance", &format!("alliance_{}", id));
            Alliance::from_json(&json)
        })
        .collect();

    // 按等级和资金排序
    alliances.sort_by(|a, b| b.level.cmp(&a.level).then(b.fund.cmp(&a.fund)));

    let user_guild = db.read_basic(user_id, "unionName");
    let user_alliance_id = if !user_guild.is_empty() && user_guild != "无" {
        db.global_get("guild_alliance", &format!("guild_alliance_{}", user_guild))
    } else {
        String::new()
    };

    let mut lines: Vec<String> = vec!["🏆 联盟排行榜\n".to_string()];

    for (i, a) in alliances.iter().enumerate().take(15) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let (_, _, _, _, emoji) = get_level_info(a.level);
        let members = a.member_list().len();
        let highlight = if a.id == user_alliance_id { " ⭐" } else { "" };
        lines.push(format!(
            "  {} {}. {} {} | {}级 | 资金:{} | {}个公会{}",
            medal,
            i + 1,
            emoji,
            a.name,
            a.level,
            format_gold(a.fund),
            members,
            highlight
        ));
    }

    // 显示用户位置
    if !user_alliance_id.is_empty() {
        if let Some(pos) = alliances.iter().position(|a| a.id == user_alliance_id) {
            if pos >= 15 {
                lines.push(format!("\n  ⭐ 您的联盟: 第 {} 名 — {}", pos + 1, alliances[pos].name));
            }
        }
    }

    format!("{}\n{}", prefix, lines.join("\n"))
}

/// 联盟捐献日志
pub fn cmd_alliance_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    let log_key = format!("donate_log_{}", alliance.id);
    let log = db.global_get("guild_alliance", &log_key);

    if log.is_empty() {
        return format!("{}\n📜 联盟捐献记录\n\n暂无捐献记录。", prefix);
    }

    let mut lines: Vec<String> = vec!["📜 联盟捐献记录\n".to_string()];

    for entry in log.split(';').take(10) {
        let parts: Vec<&str> = entry.split('|').collect();
        if parts.len() >= 4 {
            lines.push(format!(
                "  💰 {} 捐献 {} 金币 (+{} 资金) [{}]",
                parts[0], parts[1], parts[2], parts[3]
            ));
        }
    }

    format!("{}\n{}", prefix, lines.join("\n"))
}

/// 设置联盟公告 (盟主操作)
pub fn cmd_alliance_notice(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let user_guild = db.read_basic(user_id, "unionName");
    if user_guild.is_empty() || user_guild == "无" {
        return format!("{}\n❌ 您未加入任何公会。", prefix);
    }

    let mut alliance = match get_guild_alliance(db, &user_guild) {
        Some(a) => a,
        None => return format!("{}\n❌ 您的公会不在任何联盟中。", prefix),
    };

    if alliance.leader_guild != user_guild {
        return format!("{}\n❌ 只有盟主公会可以设置联盟公告。", prefix);
    }

    let notice = args.trim();
    if notice.is_empty() {
        return format!(
            "{}\n📋 设置联盟公告\n\n用法: 联盟公告+公告内容\n当前公告: {}",
            prefix,
            if alliance.notice.is_empty() {
                "暂无"
            } else {
                &alliance.notice
            }
        );
    }

    if notice.len() > 100 {
        return format!("{}\n❌ 公告内容不能超过100个字符。", prefix);
    }

    alliance.notice = notice.to_string();
    save_alliance(db, &alliance);

    format!("{}\n✅ 联盟公告已更新。\n📢 {}", prefix, notice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alliance_levels_count() {
        assert_eq!(ALLIANCE_LEVELS.len(), 5);
    }

    #[test]
    fn test_alliance_levels_unique() {
        let mut levels: Vec<i32> = ALLIANCE_LEVELS.iter().map(|(l, _, _, _, _)| *l).collect();
        levels.sort();
        levels.dedup();
        assert_eq!(levels.len(), ALLIANCE_LEVELS.len());
    }

    #[test]
    fn test_alliance_levels_ordering() {
        for i in 1..ALLIANCE_LEVELS.len() {
            assert!(ALLIANCE_LEVELS[i].1 > ALLIANCE_LEVELS[i - 1].1);
        }
    }

    #[test]
    fn test_bonus_pct_range() {
        for &(_, _, pct, _, _) in ALLIANCE_LEVELS {
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_max_members() {
        assert!(MAX_ALLIANCE_MEMBERS >= 3);
        assert!(MAX_ALLIANCE_MEMBERS <= 10);
    }

    #[test]
    fn test_create_cost_positive() {
        assert!(CREATE_COST_GOLD > 0);
    }

    #[test]
    fn test_gen_alliance_id_deterministic() {
        let id1 = gen_alliance_id("TestGuild");
        let id2 = gen_alliance_id("TestGuild");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_gen_alliance_id_different() {
        let id1 = gen_alliance_id("GuildA");
        let id2 = gen_alliance_id("GuildB");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_gen_alliance_id_format() {
        let id = gen_alliance_id("Test");
        assert!(id.starts_with("ALN-"));
        assert_eq!(id.len(), 12); // ALN- + 8 hex chars
    }

    #[test]
    fn test_calc_level_zero_fund() {
        assert_eq!(calc_level(0), 1);
    }

    #[test]
    fn test_calc_level_max() {
        assert_eq!(calc_level(200_000), 5);
    }

    #[test]
    fn test_calc_level_boundary() {
        assert_eq!(calc_level(5_000), 2);
        assert_eq!(calc_level(4_999), 1);
        assert_eq!(calc_level(20_000), 3);
        assert_eq!(calc_level(50_000), 4);
        assert_eq!(calc_level(150_000), 5);
    }

    #[test]
    fn test_get_level_info_valid() {
        let (_, _, pct, name, emoji) = get_level_info(1);
        assert_eq!(*name, "初级联盟");
        assert!(!emoji.is_empty());
        assert_eq!(*pct, 0.02);
    }

    #[test]
    fn test_get_level_info_invalid_defaults() {
        let (level, _, _, _, _) = get_level_info(99);
        assert_eq!(*level, 1); // defaults to first
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(1234), "1,234");
        assert_eq!(format_gold(1_000_000), "1,000,000");
        assert_eq!(format_gold(999), "999");
    }

    #[test]
    fn test_alliance_json_roundtrip() {
        let a = Alliance {
            id: "ALN-12345678".to_string(),
            name: "测试联盟".to_string(),
            leader_guild: "领袖公会".to_string(),
            member_guilds: "领袖公会,成员A,成员B".to_string(),
            fund: 5000,
            level: 2,
            created_at: "2026-01-01".to_string(),
            notice: "欢迎".to_string(),
        };
        let json = a.to_json();
        let parsed = Alliance::from_json(&json).unwrap();
        assert_eq!(parsed.id, "ALN-12345678");
        assert_eq!(parsed.name, "测试联盟");
        assert_eq!(parsed.leader_guild, "领袖公会");
        assert_eq!(parsed.fund, 5000);
        assert_eq!(parsed.level, 2);
        assert_eq!(parsed.notice, "欢迎");
    }

    #[test]
    fn test_alliance_member_list() {
        let a = Alliance {
            id: "T".to_string(),
            name: "T".to_string(),
            leader_guild: "A".to_string(),
            member_guilds: "A,B,C".to_string(),
            fund: 0,
            level: 1,
            created_at: String::new(),
            notice: String::new(),
        };
        let members = a.member_list();
        assert_eq!(members.len(), 3);
        assert_eq!(members[0], "A");
        assert_eq!(members[2], "C");
    }

    #[test]
    fn test_alliance_member_list_empty() {
        let a = Alliance {
            id: "T".to_string(),
            name: "T".to_string(),
            leader_guild: "A".to_string(),
            member_guilds: String::new(),
            fund: 0,
            level: 1,
            created_at: String::new(),
            notice: String::new(),
        };
        let members = a.member_list();
        assert_eq!(members.len(), 0);
    }

    #[test]
    fn test_extract_json_str() {
        let json = r#"{"id":"ALN-123","name":"联盟A"}"#;
        assert_eq!(extract_json_str(json, "id"), Some("ALN-123".to_string()));
        assert_eq!(extract_json_str(json, "name"), Some("联盟A".to_string()));
        assert_eq!(extract_json_str(json, "missing"), None);
    }

    #[test]
    fn test_extract_json_i64() {
        let json = r#"{"fund":12345,"level":3}"#;
        assert_eq!(extract_json_i64(json, "fund"), Some(12345));
        assert_eq!(extract_json_i32(json, "level"), Some(3));
    }
}

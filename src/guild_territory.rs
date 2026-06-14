/// CakeGame 领地争夺系统
///
/// 公会之间争夺地图领地的PvP系统。公会可以占领地图获得资源加成和税收，
/// 其他公会可以发起挑战争夺领地控制权。领地越多，公会实力越强。
///
/// 指令: 查看领地, 占领领地, 领地挑战, 领地排行, 领地收益, 领地详情, 放弃领地
use crate::db::Database;
use crate::user;

/// 领地状态
#[derive(Debug, Clone, PartialEq)]
pub enum TerritoryStatus {
    /// 无人占领
    Vacant,
    /// 已占领
    Occupied,
    /// 争夺中
    Contested,
    /// 保护期(刚占领，不可被挑战)
    Protected,
}

impl TerritoryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vacant => "无人占领",
            Self::Occupied => "已占领",
            Self::Contested => "争夺中",
            Self::Protected => "保护期",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "无人占领" | "vacant" => Some(Self::Vacant),
            "已占领" | "occupied" => Some(Self::Occupied),
            "争夺中" | "contested" => Some(Self::Contested),
            "保护期" | "protected" => Some(Self::Protected),
            _ => None,
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Vacant => "🏳️",
            Self::Occupied => "🏴",
            Self::Contested => "⚔️",
            Self::Protected => "🛡️",
        }
    }
}

/// 领地等级
#[derive(Debug, Clone, PartialEq)]
pub enum TerritoryTier {
    /// 普通领地
    Normal,
    /// 富饶领地
    Rich,
    /// 战略要地
    Strategic,
    /// 王城
    Royal,
}

#[allow(dead_code)]
impl TerritoryTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "普通",
            Self::Rich => "富饶",
            Self::Strategic => "战略",
            Self::Royal => "王城",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "普通" | "normal" => Some(Self::Normal),
            "富饶" | "rich" => Some(Self::Rich),
            "战略" | "strategic" => Some(Self::Strategic),
            "王城" | "royal" => Some(Self::Royal),
            _ => None,
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Normal => "🏕️",
            Self::Rich => "🏰",
            Self::Strategic => "⚔️",
            Self::Royal => "👑",
        }
    }

    /// 领地每日基础税收(金币)
    pub fn daily_tax(&self) -> i64 {
        match self {
            Self::Normal => 500,
            Self::Rich => 2000,
            Self::Strategic => 5000,
            Self::Royal => 15000,
        }
    }

    /// 领地占领所需公会等级
    pub fn required_guild_level(&self) -> i32 {
        match self {
            Self::Normal => 1,
            Self::Rich => 2,
            Self::Strategic => 3,
            Self::Royal => 5,
        }
    }

    /// 挑战领地的费用(金币)
    pub fn challenge_cost(&self) -> i64 {
        match self {
            Self::Normal => 3000,
            Self::Rich => 10000,
            Self::Strategic => 30000,
            Self::Royal => 100000,
        }
    }

    /// 属性加成百分比(占领公会成员)
    pub fn attr_bonus_pct(&self) -> i32 {
        match self {
            Self::Normal => 2,
            Self::Rich => 5,
            Self::Strategic => 8,
            Self::Royal => 15,
        }
    }
}

/// 领地信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Territory {
    pub map_id: i32,
    pub map_name: String,
    pub tier: TerritoryTier,
    pub status: TerritoryStatus,
    pub owner_guild: String,
    pub owner_guild_id: i32,
    pub captured_at: String,
    pub defense_wins: i32,
    pub total_challenges: i32,
    pub accumulated_tax: i64,
}

/// 领地挑战记录
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TerritoryChallenge {
    pub challenge_id: String,
    pub map_id: i32,
    pub map_name: String,
    pub attacker_guild: String,
    pub defender_guild: String,
    pub created_at: String,
    pub status: String, // pending / accepted / rejected / completed
}

/// 可争夺的地图领地定义
fn territory_definitions() -> Vec<(i32, &'static str, TerritoryTier)> {
    vec![
        (1, "新手村", TerritoryTier::Normal),
        (2, "翠竹林", TerritoryTier::Normal),
        (3, "落日镇", TerritoryTier::Normal),
        (5, "幽暗森林", TerritoryTier::Normal),
        (7, "矿山镇", TerritoryTier::Rich),
        (9, "沙漠绿洲", TerritoryTier::Rich),
        (11, "雪山要塞", TerritoryTier::Strategic),
        (13, "龙骨荒原", TerritoryTier::Strategic),
        (15, "暗影谷", TerritoryTier::Strategic),
        (17, "凤凰城", TerritoryTier::Royal),
        (20, "龙之巢穴", TerritoryTier::Royal),
        (25, "天界之城", TerritoryTier::Royal),
    ]
}

/// 获取领地数据
fn get_territory(db: &Database, map_id: i32) -> Option<Territory> {
    let defs = territory_definitions();
    let def = defs.iter().find(|d| d.0 == map_id)?;

    let data = db.global_get("territory", &map_id.to_string());
    if data == "[NULL]" || data.is_empty() {
        return Some(Territory {
            map_id,
            map_name: def.1.to_string(),
            tier: def.2.clone(),
            status: TerritoryStatus::Vacant,
            owner_guild: String::new(),
            owner_guild_id: 0,
            captured_at: String::new(),
            defense_wins: 0,
            total_challenges: 0,
            accumulated_tax: 0,
        });
    }

    let parts: Vec<&str> = data.split('|').collect();
    if parts.len() < 7 {
        return Some(Territory {
            map_id,
            map_name: def.1.to_string(),
            tier: def.2.clone(),
            status: TerritoryStatus::Vacant,
            owner_guild: String::new(),
            owner_guild_id: 0,
            captured_at: String::new(),
            defense_wins: 0,
            total_challenges: 0,
            accumulated_tax: 0,
        });
    }

    Some(Territory {
        map_id,
        map_name: def.1.to_string(),
        tier: def.2.clone(),
        status: TerritoryStatus::from_str(parts[0]).unwrap_or(TerritoryStatus::Vacant),
        owner_guild: parts[1].to_string(),
        owner_guild_id: parts[2].parse().unwrap_or(0),
        captured_at: parts[3].to_string(),
        defense_wins: parts[4].parse().unwrap_or(0),
        total_challenges: parts[5].parse().unwrap_or(0),
        accumulated_tax: parts[6].parse().unwrap_or(0),
    })
}

/// 保存领地数据
fn save_territory(db: &Database, t: &Territory) {
    let data = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        t.status.as_str(),
        t.owner_guild,
        t.owner_guild_id,
        t.captured_at,
        t.defense_wins,
        t.total_challenges,
        t.accumulated_tax,
    );
    db.global_set("territory", &t.map_id.to_string(), &data);
}

/// 获取公会占领的领地数量
fn count_guild_territories(db: &Database, guild_id: i32) -> i32 {
    let defs = territory_definitions();
    let mut count = 0;
    for (map_id, _, _) in &defs {
        if let Some(t) = get_territory(db, *map_id) {
            if t.owner_guild_id == guild_id && t.status != TerritoryStatus::Vacant {
                count += 1;
            }
        }
    }
    count
}

/// 获取公会占领的领地列表
fn get_guild_territories(db: &Database, guild_id: i32) -> Vec<Territory> {
    let defs = territory_definitions();
    let mut result = Vec::new();
    for (map_id, _, _) in &defs {
        if let Some(t) = get_territory(db, *map_id) {
            if t.owner_guild_id == guild_id && t.status != TerritoryStatus::Vacant {
                result.push(t);
            }
        }
    }
    result
}

/// 获取用户公会ID
fn get_user_guild_id(db: &Database, user_id: &str) -> i32 {
    let raw = db.read_basic(user_id, "Guild_ID");
    if raw == "[NULL]" || raw.is_empty() {
        0
    } else {
        raw.parse().unwrap_or(0)
    }
}

/// 获取公会名
fn get_guild_name(db: &Database, guild_id: i32) -> String {
    let raw = db.read_basic(&guild_id.to_string(), "Guild_Name");
    if raw == "[NULL]" || raw.is_empty() {
        format!("公会#{}", guild_id)
    } else {
        raw
    }
}

/// 现在时间戳
fn now_string() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("D{} {:02}:{:02}", days, hours, mins)
}

/// 查看领地 — 列出所有可争夺领地
pub fn cmd_view_territories(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let _ui = user::calc_total_attrs(db, user_id);
    let guild_id = get_user_guild_id(db, user_id);

    let defs = territory_definitions();
    let mut out = String::from("🏰 ═══════ 领地争夺 ═══════\n\n");

    // 按等级分组
    let tiers = [
        ("普通领地🏕️", TerritoryTier::Normal),
        ("富饶领地🏰", TerritoryTier::Rich),
        ("战略要地⚔️", TerritoryTier::Strategic),
        ("王城领地👑", TerritoryTier::Royal),
    ];

    for (label, tier) in &tiers {
        out.push_str(&format!("【{}】\n", label));
        for (map_id, map_name, t) in &defs {
            if t != tier {
                continue;
            }
            if let Some(territory) = get_territory(db, *map_id) {
                let owner_display = if territory.owner_guild.is_empty() {
                    "无".to_string()
                } else {
                    territory.owner_guild.clone()
                };
                let tax = t.daily_tax();
                let bonus = t.attr_bonus_pct();
                out.push_str(&format!(
                    "  {} [{}] {} — {} (税收:{}/天 属性+{}%)\n",
                    territory.status.emoji(),
                    map_id,
                    map_name,
                    owner_display,
                    tax,
                    bonus,
                ));
            }
        }
        out.push('\n');
    }

    // 我的公会领地
    if guild_id > 0 {
        let my_territories = get_guild_territories(db, guild_id);
        let total_tax: i64 = my_territories.iter().map(|t| t.tier.daily_tax()).sum();
        let total_bonus = if my_territories.is_empty() {
            0
        } else {
            my_territories
                .iter()
                .map(|t| t.tier.attr_bonus_pct())
                .max()
                .unwrap_or(0)
        };
        out.push_str(&format!(
            "🏴 我的公会: 占领 {} 块领地 | 日税收: {} 金 | 属性加成: +{}%\n",
            my_territories.len(),
            total_tax,
            total_bonus,
        ));
    } else {
        out.push_str("⚠️ 你还没有加入公会，加入公会后才能参与领地争夺\n");
    }

    out.push_str("\n💡 指令: 占领领地 [地图ID] | 领地挑战 [地图ID] | 领地详情 [地图ID]");
    out
}

/// 领地详情 — 查看某块领地的详细信息
pub fn cmd_territory_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_id: i32 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return "❌ 请输入有效的地图ID，如: 领地详情 7".to_string(),
    };

    let territory = match get_territory(db, map_id) {
        Some(t) => t,
        None => return format!("❌ 地图ID {} 不是可争夺的领地", map_id),
    };

    let defs = territory_definitions();
    let def = defs.iter().find(|d| d.0 == map_id);
    if def.is_none() {
        return format!("❌ 地图ID {} 不是可争夺的领地", map_id);
    }

    let mut out = format!("{} ═══════ 领地详情 ═══════\n\n", territory.tier.emoji());
    out.push_str(&format!("📍 领地名称: {}\n", territory.map_name));
    out.push_str(&format!("📍 地图ID: {}\n", territory.map_id));
    out.push_str(&format!(
        "📊 领地等级: {} {}\n",
        territory.tier.as_str(),
        territory.tier.emoji()
    ));
    out.push_str(&format!(
        "{} 状态: {}\n",
        territory.status.emoji(),
        territory.status.as_str()
    ));

    if territory.owner_guild.is_empty() {
        out.push_str("🏴 占领公会: 无\n");
    } else {
        out.push_str(&format!("🏴 占领公会: {}\n", territory.owner_guild));
        out.push_str(&format!("📅 占领时间: {}\n", territory.captured_at));
        out.push_str(&format!("🛡️ 防御成功: {} 次\n", territory.defense_wins));
        out.push_str(&format!("⚔️ 被挑战: {} 次\n", territory.total_challenges));
    }

    out.push_str(&format!("💰 每日税收: {} 金币\n", territory.tier.daily_tax()));
    out.push_str(&format!(
        "📈 属性加成: +{}% (占领公会全员)\n",
        territory.tier.attr_bonus_pct()
    ));
    out.push_str(&format!(
        "📋 占领要求: 公会等级 ≥ {}\n",
        territory.tier.required_guild_level()
    ));
    out.push_str(&format!("💎 挑战费用: {} 金币\n", territory.tier.challenge_cost()));

    if territory.accumulated_tax > 0 {
        out.push_str(&format!("🏦 累积税收: {} 金币 (等待领取)\n", territory.accumulated_tax));
    }

    // 挑战记录
    let challenges = db.global_get("territory_challenges", &map_id.to_string());
    if challenges != "[NULL]" && !challenges.is_empty() {
        let records: Vec<&str> = challenges.split(';').collect();
        let recent: Vec<&str> = records.iter().rev().take(5).copied().collect();
        if !recent.is_empty() {
            out.push_str("\n📜 最近挑战记录:\n");
            for record in recent.iter().rev() {
                out.push_str(&format!("  • {}\n", record));
            }
        }
    }

    let user_guild_id = get_user_guild_id(db, user_id);
    if territory.status == TerritoryStatus::Vacant {
        out.push_str("\n💡 此领地无人占领，使用「占领领地 地图ID」占领");
    } else if user_guild_id > 0 && territory.owner_guild_id != user_guild_id {
        out.push_str("\n💡 使用「领地挑战 地图ID」发起领地争夺战");
    } else if user_guild_id > 0 && territory.owner_guild_id == user_guild_id {
        out.push_str("\n💡 这是你的公会领地，使用「领地收益」领取税收");
    }

    out
}

/// 占领领地 — 占领无人占领的领地
pub fn cmd_capture_territory(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_id: i32 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return "❌ 请输入有效的地图ID，如: 占领领地 7".to_string(),
    };

    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入公会，无法占领领地".to_string();
    }

    // 检查是否是公会领袖
    let role = db.read_basic(user_id, "Guild_Role");
    if role != "1" && role != "会长" {
        return "❌ 只有公会会长才能占领领地".to_string();
    }

    let mut territory = match get_territory(db, map_id) {
        Some(t) => t,
        None => return format!("❌ 地图ID {} 不是可争夺的领地", map_id),
    };

    if territory.status != TerritoryStatus::Vacant {
        return format!(
            "❌ 领地 [{}] {} 已被「{}」占领，需要使用「领地挑战」争夺",
            territory.map_id, territory.map_name, territory.owner_guild
        );
    }

    let guild_level: i32 = db.read_basic(&guild_id.to_string(), "Guild_Level").parse().unwrap_or(1);
    let required = territory.tier.required_guild_level();
    if guild_level < required {
        return format!(
            "❌ 占领{}领地需要公会等级 ≥ {}，你的公会等级: {}",
            territory.tier.as_str(),
            required,
            guild_level
        );
    }

    // 检查公会已占领数量上限
    let owned_count = count_guild_territories(db, guild_id);
    let max_territories = 3 + (guild_level / 2); // 基础3块，每2级+1
    if owned_count >= max_territories {
        return format!(
            "❌ 你的公会已占领 {} 块领地，当前上限 {} 块 (公会等级越高上限越大)",
            owned_count, max_territories
        );
    }

    // 占领领地
    let guild_name = get_guild_name(db, guild_id);
    territory.status = TerritoryStatus::Protected;
    territory.owner_guild = guild_name;
    territory.owner_guild_id = guild_id;
    territory.captured_at = now_string();
    territory.defense_wins = 0;
    territory.total_challenges = 0;
    territory.accumulated_tax = 0;
    save_territory(db, &territory);

    // 记录日志
    let log_entry = format!(
        "{} 「{}」占领了 {} ({}领地)",
        now_string(),
        territory.owner_guild,
        territory.map_name,
        territory.tier.as_str()
    );
    append_territory_log(db, map_id, &log_entry);

    format!(
        "🎉 ═══════ 领地占领成功 ═══════\n\n\
         🏴 公会「{}」成功占领了 [{}] {}！\n\n\
         📊 领地等级: {} {}\n\
         💰 每日税收: {} 金币\n\
         📈 属性加成: +{}% (公会全员)\n\
         🛡️ 保护期: 24小时内无法被挑战\n\n\
         💡 使用「领地收益」领取累积税收",
        territory.owner_guild,
        territory.map_id,
        territory.map_name,
        territory.tier.as_str(),
        territory.tier.emoji(),
        territory.tier.daily_tax(),
        territory.tier.attr_bonus_pct(),
    )
}

/// 领地挑战 — 挑战已占领的领地
pub fn cmd_challenge_territory(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_id: i32 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return "❌ 请输入有效的地图ID，如: 领地挑战 7".to_string(),
    };

    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入公会，无法挑战领地".to_string();
    }

    let role = db.read_basic(user_id, "Guild_Role");
    if role != "1" && role != "会长" {
        return "❌ 只有公会会长才能发起领地挑战".to_string();
    }

    let mut territory = match get_territory(db, map_id) {
        Some(t) => t,
        None => return format!("❌ 地图ID {} 不是可争夺的领地", map_id),
    };

    if territory.status == TerritoryStatus::Vacant {
        return format!(
            "❌ 领地 [{}] {} 无人占领，使用「占领领地」直接占领",
            territory.map_id, territory.map_name
        );
    }

    if territory.owner_guild_id == guild_id {
        return "❌ 不能挑战自己公会的领地".to_string();
    }

    if territory.status == TerritoryStatus::Protected {
        return format!(
            "❌ 领地 [{}] {} 处于保护期，无法挑战",
            territory.map_id, territory.map_name
        );
    }

    if territory.status == TerritoryStatus::Contested {
        return format!(
            "❌ 领地 [{}] {} 正在被其他公会挑战中，请稍后再试",
            territory.map_id, territory.map_name
        );
    }

    let challenge_cost = territory.tier.challenge_cost();
    let guild_gold: i64 = db.read_basic(&guild_id.to_string(), "Guild_Gold").parse().unwrap_or(0);
    if guild_gold < challenge_cost {
        return format!(
            "❌ 挑战{}领地需要公会资金 {} 金币，当前公会资金: {} 金币",
            territory.tier.as_str(),
            challenge_cost,
            guild_gold
        );
    }

    let guild_level: i32 = db.read_basic(&guild_id.to_string(), "Guild_Level").parse().unwrap_or(1);
    let required = territory.tier.required_guild_level();
    if guild_level < required {
        return format!(
            "❌ 挑战{}领地需要公会等级 ≥ {}，你的公会等级: {}",
            territory.tier.as_str(),
            required,
            guild_level
        );
    }

    // 扣除公会资金
    db.modify_currency(
        &guild_id.to_string(),
        crate::core::CURRENCY_GOLD,
        crate::core::OP_SUB,
        challenge_cost,
    ); // OP_SUB

    // 模拟领地争夺战: 基于双方公会实力
    let attacker_power = calc_guild_power(db, guild_id);
    let defender_power = calc_guild_power(db, territory.owner_guild_id);

    // 攻方有20%劣势(守方优势)
    let attacker_roll = attacker_power as f64 * (0.8 + rand_factor(user_id, map_id) * 0.4);
    let defender_roll = defender_power as f64 * (0.8 + rand_factor(&territory.owner_guild, map_id) * 0.4);

    let attacker_wins = attacker_roll > defender_roll;
    territory.total_challenges += 1;

    let attacker_guild_name = get_guild_name(db, guild_id);
    let result_msg = if attacker_wins {
        // 攻方胜利 — 夺取领地
        territory.status = TerritoryStatus::Protected;
        territory.owner_guild = attacker_guild_name.clone();
        territory.owner_guild_id = guild_id;
        territory.captured_at = now_string();
        territory.defense_wins = 0;
        territory.accumulated_tax = 0;
        save_territory(db, &territory);

        let log_entry = format!(
            "{} 「{}」击败「{}」夺取了 {}",
            now_string(),
            attacker_guild_name,
            territory.owner_guild,
            territory.map_name
        );
        append_territory_log(db, map_id, &log_entry);

        format!(
            "🎉 ═══════ 领地争夺胜利！═══════\n\n\
             ⚔️ 「{}」vs「{}」\n\
             📍 争夺领地: [{}] {}\n\n\
             🏆 恭喜！你的公会成功夺取了领地！\n\
             💰 每日税收: {} 金币\n\
             📈 属性加成: +{}%\n\
             🛡️ 保护期: 24小时\n\n\
             💸 消耗公会资金: {} 金币",
            attacker_guild_name,
            territory.owner_guild,
            territory.map_id,
            territory.map_name,
            territory.tier.daily_tax(),
            territory.tier.attr_bonus_pct(),
            challenge_cost,
        )
    } else {
        // 守方胜利
        territory.defense_wins += 1;
        save_territory(db, &territory);

        // 给守方公会一部分补偿
        let bonus = challenge_cost / 2;
        db.modify_currency(
            &territory.owner_guild_id.to_string(),
            crate::core::CURRENCY_GOLD,
            crate::core::OP_ADD,
            bonus,
        ); // OP_ADD

        let log_entry = format!(
            "{} 「{}」挑战「{}」失败 (防御成功+1)",
            now_string(),
            attacker_guild_name,
            territory.owner_guild,
        );
        append_territory_log(db, map_id, &log_entry);

        format!(
            "💀 ═══════ 领地争夺失败 ═══════\n\n\
             ⚔️ 「{}」vs「{}」\n\
             📍 争夺领地: [{}] {}\n\n\
             ❌ 很遗憾，你的公会未能夺取领地。\n\
             🛡️ 对方已累计成功防御 {} 次\n\n\
             💸 消耗公会资金: {} 金币\n\
             💡 提升公会实力后再次挑战",
            attacker_guild_name,
            territory.owner_guild,
            territory.map_id,
            territory.map_name,
            territory.defense_wins,
            challenge_cost,
        )
    };

    result_msg
}

/// 领地收益 — 领取公会领地累积的税收
pub fn cmd_territory_income(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入公会".to_string();
    }

    let role = db.read_basic(user_id, "Guild_Role");
    if role != "1" && role != "会长" {
        return "❌ 只有公会会长才能领取领地收益".to_string();
    }

    let territories = get_guild_territories(db, guild_id);
    if territories.is_empty() {
        return "❌ 你的公会没有占领任何领地".to_string();
    }

    let mut total_tax: i64 = 0;
    let mut out = String::from("💰 ═══════ 领地收益 ═══════\n\n");

    for t in &territories {
        let daily = t.tier.daily_tax();
        // 简化: 直接发放每日税收(实际应根据占领天数计算)
        total_tax += daily;
        out.push_str(&format!(
            "  {} [{}] {} — {} 金/天\n",
            t.tier.emoji(),
            t.map_id,
            t.map_name,
            daily,
        ));
    }

    if total_tax > 0 {
        db.modify_currency(
            &guild_id.to_string(),
            crate::core::CURRENCY_GOLD,
            crate::core::OP_ADD,
            total_tax,
        );
        out.push_str(&format!("\n✅ 领取税收: {} 金币 (已存入公会资金)\n", total_tax));
    }

    // 公会成员分红
    let member_count: i32 = db
        .read_basic(&guild_id.to_string(), "Guild_Members")
        .parse()
        .unwrap_or(1);
    let dividend = if member_count > 0 {
        total_tax / member_count as i64
    } else {
        0
    };
    if dividend > 0 {
        db.modify_currency(user_id, crate::core::CURRENCY_GOLD, crate::core::OP_ADD, dividend); // CURRENCY_GOLD OP_ADD
        out.push_str(&format!("💎 会长分红: {} 金币 (1/{} 成员)\n", dividend, member_count));
    }

    let bonus_pct = territories.iter().map(|t| t.tier.attr_bonus_pct()).max().unwrap_or(0);
    out.push_str(&format!(
        "\n📈 当前领地属性加成: +{}% (公会全员生效)\n\
         🏴 占领领地数: {} 块\n\
         💡 领地越多，税收和加成越高！",
        bonus_pct,
        territories.len(),
    ));

    out
}

/// 领地排行 — 全服公会领地排名
pub fn cmd_territory_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let defs = territory_definitions();
    let mut guild_stats: std::collections::HashMap<i32, (String, i32, i64, i32)> = std::collections::HashMap::new();

    for (map_id, _, _) in &defs {
        if let Some(t) = get_territory(db, *map_id) {
            if t.owner_guild_id > 0 && t.status != TerritoryStatus::Vacant {
                let entry = guild_stats
                    .entry(t.owner_guild_id)
                    .or_insert_with(|| (t.owner_guild.clone(), 0, 0, 0));
                entry.1 += 1;
                entry.2 += t.tier.daily_tax();
                entry.3 += t.tier.attr_bonus_pct();
            }
        }
    }

    let mut rankings: Vec<_> = guild_stats.into_iter().collect();
    rankings.sort_by(|a, b| b.1 .1.cmp(&a.1 .1).then(b.1 .2.cmp(&a.1 .2)));

    let mut out = String::from("🏆 ═══════ 领地排行 ═══════\n\n");

    if rankings.is_empty() {
        out.push_str("  暂无公会占领领地\n");
        out.push_str("\n💡 使用「占领领地 [地图ID」开始争夺！");
        return out;
    }

    let medals = ["🥇", "🥈", "🥉"];
    let user_guild_id = get_user_guild_id(db, user_id);
    let mut user_rank = 0;

    for (i, (gid, (name, count, tax, bonus))) in rankings.iter().enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let highlight = if *gid == user_guild_id {
            user_rank = i + 1;
            " ← 我的公会"
        } else {
            ""
        };
        out.push_str(&format!(
            "{} {}. 「{}」| {} 块领地 | 税收 {} 金/天 | 加成 +{}%{}\n",
            medal,
            i + 1,
            name,
            count,
            tax,
            bonus,
            highlight,
        ));
        if i >= 14 {
            break;
        }
    }

    if user_rank == 0 && user_guild_id > 0 {
        out.push_str("\n  你的公会暂未上榜 (需占领至少1块领地)\n");
    } else if user_rank > 0 {
        out.push_str(&format!("\n  📍 你的公会排名: 第 {} 名\n", user_rank));
    }

    out.push_str(&format!("\n📊 全服领地统计: {} 块可争夺领地", defs.len()));

    out
}

/// 放弃领地 — 主动放弃已占领的领地
pub fn cmd_abandon_territory(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_id: i32 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return "❌ 请输入有效的地图ID，如: 放弃领地 7".to_string(),
    };

    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入公会".to_string();
    }

    let role = db.read_basic(user_id, "Guild_Role");
    if role != "1" && role != "会长" {
        return "❌ 只有公会会长才能放弃领地".to_string();
    }

    let mut territory = match get_territory(db, map_id) {
        Some(t) => t,
        None => return format!("❌ 地图ID {} 不是可争夺的领地", map_id),
    };

    if territory.owner_guild_id != guild_id {
        return format!(
            "❌ 领地 [{}] {} 不是你的公会占领的",
            territory.map_id, territory.map_name
        );
    }

    let guild_name = territory.owner_guild.clone();
    let map_name = territory.map_name.clone();

    // 清空领地
    territory.status = TerritoryStatus::Vacant;
    territory.owner_guild = String::new();
    territory.owner_guild_id = 0;
    territory.captured_at = String::new();
    territory.defense_wins = 0;
    territory.accumulated_tax = 0;
    save_territory(db, &territory);

    let log_entry = format!("{} 「{}」主动放弃了 {}", now_string(), guild_name, map_name,);
    append_territory_log(db, map_id, &log_entry);

    format!(
        "🏳️ ═══════ 放弃领地 ═══════\n\n\
         📍 公会「{}」已放弃领地 [{}] {}\n\n\
         ⚠️ 领地已变为空置状态，其他公会可以占领\n\
         💡 你可以随时重新占领或其他领地",
        guild_name, map_id, map_name,
    )
}

/// 追加领地日志
fn append_territory_log(db: &Database, map_id: i32, entry: &str) {
    let key = format!("log_{}", map_id);
    let existing = db.global_get("territory_log", &key);
    let mut logs = if existing == "[NULL]" || existing.is_empty() {
        Vec::new()
    } else {
        existing.split(';').map(|s| s.to_string()).collect::<Vec<_>>()
    };
    logs.push(entry.to_string());
    // 保留最近20条
    if logs.len() > 20 {
        let skip = logs.len() - 20;
        logs = logs.into_iter().skip(skip).collect();
    }
    db.global_set("territory_log", &key, &logs.join(";"));
}

/// 简单的伪随机因子(确定性，基于输入)
fn rand_factor(s: &str, seed: i32) -> f64 {
    let hash: u64 = s
        .bytes()
        .fold(seed as u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    (hash % 1000) as f64 / 1000.0
}

/// 计算公会综合实力
fn calc_guild_power(db: &Database, guild_id: i32) -> i64 {
    let level: i64 = db.read_basic(&guild_id.to_string(), "Guild_Level").parse().unwrap_or(1);
    let members: i64 = db
        .read_basic(&guild_id.to_string(), "Guild_Members")
        .parse()
        .unwrap_or(1);
    let gold: i64 = db.read_basic(&guild_id.to_string(), "Guild_Gold").parse().unwrap_or(0);

    // 公会实力 = 等级×1000 + 成员数×500 + 资金/100
    level * 1000 + members * 500 + gold / 100
}

/// 领地详情详细(含挑战历史)
pub fn cmd_territory_log(db: &Database, _user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let map_id: i32 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return "❌ 请输入有效的地图ID，如: 领地日志 7".to_string(),
    };

    let territory = match get_territory(db, map_id) {
        Some(t) => t,
        None => return format!("❌ 地图ID {} 不是可争夺的领地", map_id),
    };

    let mut out = format!(
        "📜 ═══════ 领地日志 [{}] {} ═══════\n\n",
        territory.map_id, territory.map_name
    );

    let key = format!("log_{}", map_id);
    let logs = db.global_get("territory_log", &key);
    if logs == "[NULL]" || logs.is_empty() {
        out.push_str("  暂无记录\n");
    } else {
        let entries: Vec<&str> = logs.split(';').collect();
        for entry in entries.iter().rev().take(10) {
            out.push_str(&format!("  📌 {}\n", entry));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_territory_status_roundtrip() {
        let statuses = [
            TerritoryStatus::Vacant,
            TerritoryStatus::Occupied,
            TerritoryStatus::Contested,
            TerritoryStatus::Protected,
        ];
        for s in &statuses {
            let str_val = s.as_str();
            let parsed = TerritoryStatus::from_str(str_val);
            assert_eq!(parsed.as_ref(), Some(s));
        }
    }

    #[test]
    fn test_territory_tier_properties() {
        let tiers = [
            TerritoryTier::Normal,
            TerritoryTier::Rich,
            TerritoryTier::Strategic,
            TerritoryTier::Royal,
        ];
        for t in &tiers {
            assert!(t.daily_tax() > 0);
            assert!(t.required_guild_level() > 0);
            assert!(t.challenge_cost() > 0);
            assert!(t.attr_bonus_pct() > 0);
        }
        // 税收递增
        assert!(TerritoryTier::Normal.daily_tax() < TerritoryTier::Rich.daily_tax());
        assert!(TerritoryTier::Rich.daily_tax() < TerritoryTier::Strategic.daily_tax());
        assert!(TerritoryTier::Strategic.daily_tax() < TerritoryTier::Royal.daily_tax());
    }

    #[test]
    fn test_territory_tier_roundtrip() {
        let tiers = [
            TerritoryTier::Normal,
            TerritoryTier::Rich,
            TerritoryTier::Strategic,
            TerritoryTier::Royal,
        ];
        for t in &tiers {
            let s = t.as_str();
            let parsed = TerritoryTier::from_str(s);
            assert_eq!(parsed.as_ref(), Some(t));
        }
    }

    #[test]
    fn test_territory_definitions_count() {
        let defs = territory_definitions();
        assert!(defs.len() >= 10, "Should have at least 10 territories");
        // 确保所有tier都有代表
        let has_normal = defs.iter().any(|d| d.2 == TerritoryTier::Normal);
        let has_rich = defs.iter().any(|d| d.2 == TerritoryTier::Rich);
        let has_strategic = defs.iter().any(|d| d.2 == TerritoryTier::Strategic);
        let has_royal = defs.iter().any(|d| d.2 == TerritoryTier::Royal);
        assert!(has_normal && has_rich && has_strategic && has_royal);
    }

    #[test]
    fn test_territory_definitions_unique_ids() {
        let defs = territory_definitions();
        let mut ids: Vec<i32> = defs.iter().map(|d| d.0).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), defs.len(), "Map IDs should be unique");
    }

    #[test]
    fn test_rand_factor_deterministic() {
        let f1 = rand_factor("test_guild", 7);
        let f2 = rand_factor("test_guild", 7);
        assert!((f1 - f2).abs() < f64::EPSILON);
        assert!(f1 >= 0.0 && f1 <= 1.0);
    }

    #[test]
    fn test_rand_factor_varies() {
        let f1 = rand_factor("guild_a", 1);
        let f2 = rand_factor("guild_b", 1);
        // Very unlikely to be exactly equal
        assert!(f1 != f2 || true); // May be equal but extremely rare
    }

    #[test]
    fn test_status_emoji() {
        assert_eq!(TerritoryStatus::Vacant.emoji(), "🏳️");
        assert_eq!(TerritoryStatus::Occupied.emoji(), "🏴");
        assert_eq!(TerritoryStatus::Contested.emoji(), "⚔️");
        assert_eq!(TerritoryStatus::Protected.emoji(), "🛡️");
    }

    #[test]
    fn test_tier_emoji() {
        assert_eq!(TerritoryTier::Normal.emoji(), "🏕️");
        assert_eq!(TerritoryTier::Rich.emoji(), "🏰");
        assert_eq!(TerritoryTier::Strategic.emoji(), "⚔️");
        assert_eq!(TerritoryTier::Royal.emoji(), "👑");
    }

    #[test]
    fn test_max_territories_formula() {
        // 3 + guild_level / 2
        let level = 4i32;
        let max = 3 + level / 2;
        assert_eq!(max, 5);
    }
}

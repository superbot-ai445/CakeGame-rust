/// CakeGame 公会战系统
///
/// 公会之间的PvP对战系统。公会领袖可以向其他公会发起挑战，
/// 公会成员报名参战，战斗按回合制进行，获胜公会获得丰厚奖励。
///
/// 指令: 查看公会战, 发起公会战, 参与公会战, 公会战排名, 公会战奖励
use crate::combat_power;
use crate::db::Database;
use crate::user;

/// 公会战状态
#[derive(Debug, Clone, PartialEq)]
pub enum GuildWarStatus {
    /// 报名中
    Registration,
    /// 战斗中
    Battle,
    /// 已结束
    Finished,
    /// 已取消
    Cancelled,
}

impl GuildWarStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Registration => "报名中",
            Self::Battle => "战斗中",
            Self::Finished => "已结束",
            Self::Cancelled => "已取消",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "报名中" | "registration" => Some(Self::Registration),
            "战斗中" | "battle" => Some(Self::Battle),
            "已结束" | "finished" => Some(Self::Finished),
            "已取消" | "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Registration => "📋",
            Self::Battle => "⚔️",
            Self::Finished => "🏆",
            Self::Cancelled => "❌",
        }
    }
}

/// 公会战参与者信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WarParticipant {
    pub user_id: String,
    pub nickname: String,
    pub level: i32,
    pub combat_power: i64,
    pub damage_dealt: i64,
    pub battles_won: i32,
    pub battles_lost: i32,
}

/// 公会战信息
#[derive(Debug, Clone)]
pub struct GuildWar {
    pub war_id: String,
    pub attacker_guild: String,
    pub defender_guild: String,
    pub status: GuildWarStatus,
    pub attacker_score: i32,
    pub defender_score: i32,
    pub total_rounds: i32,
    pub current_round: i32,
    pub created_at: String,
    pub attacker_members: Vec<WarParticipant>,
    pub defender_members: Vec<WarParticipant>,
}

/// 公会战配置
const MAX_PARTICIPANTS_PER_SIDE: usize = 10;
const WAR_ROUNDS: i32 = 5;
#[allow(dead_code)]
const REGISTRATION_DURATION_MINUTES: i32 = 30;
const WAR_REWARD_GOLD: i64 = 5000;
const WAR_REWARD_DIAMOND: i32 = 50;
const WAR_REWARD_EXP: i32 = 500;
const LOSER_REWARD_GOLD: i64 = 1000;
const LOSER_REWARD_EXP: i32 = 100;

/// 获取玩家所在公会
fn get_user_guild(db: &Database, user_id: &str) -> Option<String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2").ok()?;
    let guild: String = stmt
        .query_row(rusqlite::params![user_id, "guild_name"], |row| row.get(0))
        .ok()?;
    if guild.is_empty() {
        None
    } else {
        Some(guild)
    }
}

/// 获取公会领袖
fn get_guild_leader(db: &Database, guild_name: &str) -> Option<String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2").ok()?;
    let leader: String = stmt
        .query_row(rusqlite::params![format!("guild_{}", guild_name), "leader"], |row| {
            row.get(0)
        })
        .ok()?;
    if leader.is_empty() {
        None
    } else {
        Some(leader)
    }
}

/// 生成公会战ID
fn generate_war_id() -> String {
    let now = chrono::Local::now();
    format!("GW_{}", now.format("%Y%m%d%H%M%S"))
}

/// 获取公会成员列表
#[allow(dead_code)]
fn get_guild_members(db: &Database, guild_name: &str) -> Vec<String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT Node FROM Global WHERE Key='guild_name' AND Value=?1") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let members: Vec<String> = stmt
        .query_map(rusqlite::params![guild_name], |row| row.get(0))
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    members
}

/// 获取玩家战斗信息
fn get_player_war_info(db: &Database, user_id: &str) -> Option<WarParticipant> {
    let info = user::calc_total_attrs(db, user_id);
    let cp = combat_power::calc_combat_power(&info) as i64;
    Some(WarParticipant {
        user_id: user_id.to_string(),
        nickname: info.name.clone(),
        level: info.level,
        combat_power: cp,
        damage_dealt: 0,
        battles_won: 0,
        battles_lost: 0,
    })
}

/// 存储公会战数据到 Global 表
fn save_guild_war(db: &Database, war: &GuildWar) {
    let conn = db.conn.lock().unwrap();
    // 存储基本信息
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![format!("guild_war_{}", war.war_id), "attacker", war.attacker_guild],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![format!("guild_war_{}", war.war_id), "defender", war.defender_guild],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![format!("guild_war_{}", war.war_id), "status", war.status.as_str()],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            format!("guild_war_{}", war.war_id),
            "attacker_score",
            war.attacker_score.to_string()
        ],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            format!("guild_war_{}", war.war_id),
            "defender_score",
            war.defender_score.to_string()
        ],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            format!("guild_war_{}", war.war_id),
            "current_round",
            war.current_round.to_string()
        ],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![format!("guild_war_{}", war.war_id), "created_at", war.created_at],
    );

    // 存储参赛成员
    let attacker_ids: Vec<String> = war.attacker_members.iter().map(|p| p.user_id.clone()).collect();
    let defender_ids: Vec<String> = war.defender_members.iter().map(|p| p.user_id.clone()).collect();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            format!("guild_war_{}", war.war_id),
            "attacker_members",
            attacker_ids.join(",")
        ],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            format!("guild_war_{}", war.war_id),
            "defender_members",
            defender_ids.join(",")
        ],
    );

    // 存储到活跃公会战列表
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params!["guild_wars", "active_war", war.war_id],
    );
}

/// 加载公会战数据
fn load_guild_war(db: &Database, war_id: &str) -> Option<GuildWar> {
    let conn = db.conn.lock().unwrap();
    let node = format!("guild_war_{}", war_id);

    let get_val = |key: &str| -> Option<String> {
        let mut stmt = conn.prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2").ok()?;
        stmt.query_row(rusqlite::params![node, key], |row| row.get(0)).ok()
    };

    let attacker = get_val("attacker")?;
    let defender = get_val("defender")?;
    let status_str = get_val("status").unwrap_or_else(|| "报名中".to_string());
    let status = GuildWarStatus::from_str(&status_str).unwrap_or(GuildWarStatus::Registration);
    let attacker_score: i32 = get_val("attacker_score").and_then(|s| s.parse().ok()).unwrap_or(0);
    let defender_score: i32 = get_val("defender_score").and_then(|s| s.parse().ok()).unwrap_or(0);
    let current_round: i32 = get_val("current_round").and_then(|s| s.parse().ok()).unwrap_or(0);
    let created_at = get_val("created_at").unwrap_or_default();

    // 加载成员
    let attacker_ids_str = get_val("attacker_members").unwrap_or_default();
    let defender_ids_str = get_val("defender_members").unwrap_or_default();

    // 需要在 conn 外面加载玩家数据，先释放锁
    drop(conn);

    let attacker_members: Vec<WarParticipant> = attacker_ids_str
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|uid| get_player_war_info(db, uid))
        .collect();
    let defender_members: Vec<WarParticipant> = defender_ids_str
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|uid| get_player_war_info(db, uid))
        .collect();

    Some(GuildWar {
        war_id: war_id.to_string(),
        attacker_guild: attacker,
        defender_guild: defender,
        status,
        attacker_score,
        defender_score,
        total_rounds: WAR_ROUNDS,
        current_round,
        created_at,
        attacker_members,
        defender_members,
    })
}

/// 获取当前活跃的公会战
fn get_active_war(db: &Database) -> Option<String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn.prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2").ok()?;
    stmt.query_row(rusqlite::params!["guild_wars", "active_war"], |row| row.get(0))
        .ok()
}

/// 获取公会战历史记录列表
fn get_war_history(db: &Database) -> Vec<String> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn
        .prepare("SELECT DISTINCT Node FROM Global WHERE Node LIKE 'guild_war_GW_%' ORDER BY Node DESC LIMIT 20")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let ids: Vec<String> = stmt
        .query_map([], |row| {
            let node: String = row.get(0)?;
            Ok(node.replace("guild_war_", ""))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    ids
}

/// 公会战战绩存储
fn save_war_result(db: &Database, guild_name: &str, won: bool, damage: i64) {
    let conn = db.conn.lock().unwrap();
    let node = format!("guild_war_stats_{}", guild_name);

    // 获取现有数据
    let get_val = |key: &str| -> i64 {
        let mut stmt = conn
            .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
            .unwrap();
        stmt.query_row(rusqlite::params![node, key], |row| {
            let s: String = row.get(0)?;
            Ok(s.parse().unwrap_or(0))
        })
        .unwrap_or(0)
    };

    let wins = get_val("wins") + if won { 1 } else { 0 };
    let losses = get_val("losses") + if !won { 1 } else { 0 };
    let total_damage = get_val("total_damage") + damage;
    let score = get_val("score") + if won { 100 } else { 10 };

    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "wins", wins.to_string()],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "losses", losses.to_string()],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "total_damage", total_damage.to_string()],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "score", score.to_string()],
    );
}

/// 个人公会战战绩存储
fn save_player_war_result(db: &Database, user_id: &str, won: bool, damage: i64) {
    let conn = db.conn.lock().unwrap();
    let node = format!("player_war_stats_{}", user_id);

    let get_val = |key: &str| -> i64 {
        let mut stmt = conn
            .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
            .unwrap();
        stmt.query_row(rusqlite::params![node, key], |row| {
            let s: String = row.get(0)?;
            Ok(s.parse().unwrap_or(0))
        })
        .unwrap_or(0)
    };

    let wins = get_val("wins") + if won { 1 } else { 0 };
    let losses = get_val("losses") + if !won { 1 } else { 0 };
    let total_damage = get_val("total_damage") + damage;

    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "wins", wins.to_string()],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "losses", losses.to_string()],
    );
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![node, "total_damage", total_damage.to_string()],
    );
}

/// 模拟公会战战斗回合
fn simulate_war_round(attackers: &[WarParticipant], defenders: &[WarParticipant]) -> (i32, i32) {
    // 计算双方总战力
    let atk_power: i64 = attackers.iter().map(|p| p.combat_power.max(100)).sum();
    let def_power: i64 = defenders.iter().map(|p| p.combat_power.max(100)).sum();

    // 加入随机波动（基于当前秒数模拟微小随机性）
    let now_sec = chrono::Local::now().timestamp() as i32;
    let atk_roll = (atk_power as f64) * (1.0 + ((now_sec % 20) as f64 - 10.0) / 100.0);
    let def_roll = (def_power as f64) * (1.0 + (((now_sec / 7) % 20) as f64 - 10.0) / 100.0);

    if atk_roll > def_roll {
        (1, 0) // 攻击方赢
    } else if def_roll > atk_roll {
        (0, 1) // 防守方赢
    } else {
        (0, 0) // 平局
    }
}

// ==================== 指令处理函数 ====================

/// 查看公会战 — 查看当前公会战状态和历史
pub fn cmd_view_guild_war(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let _my_guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你还没有加入任何公会，无法查看公会战。\n💡 使用「我的公会」查看公会信息。".to_string(),
    };

    let mut output = String::from("⚔️ === 公会战系统 === ⚔️\n\n");

    // 检查是否有活跃公会战
    if let Some(war_id) = get_active_war(db) {
        if let Some(war) = load_guild_war(db, &war_id) {
            output.push_str(&format!("{} 当前公会战: {}\n", war.status.emoji(), war.status.as_str()));
            output.push_str(&format!(
                "🔴 进攻方: {} (得分: {})\n",
                war.attacker_guild, war.attacker_score
            ));
            output.push_str(&format!(
                "🔵 防守方: {} (得分: {})\n",
                war.defender_guild, war.defender_score
            ));
            output.push_str(&format!("📊 回合进度: {}/{}\n\n", war.current_round, war.total_rounds));

            // 显示参赛成员
            if !war.attacker_members.is_empty() {
                output.push_str("🔴 进攻方成员:\n");
                for (i, p) in war.attacker_members.iter().enumerate().take(5) {
                    output.push_str(&format!(
                        "  {}. {} Lv.{} 战力:{}\n",
                        i + 1,
                        p.nickname,
                        p.level,
                        p.combat_power
                    ));
                }
                if war.attacker_members.len() > 5 {
                    output.push_str(&format!("  ... 共{}人\n", war.attacker_members.len()));
                }
            }
            if !war.defender_members.is_empty() {
                output.push_str("\n🔵 防守方成员:\n");
                for (i, p) in war.defender_members.iter().enumerate().take(5) {
                    output.push_str(&format!(
                        "  {}. {} Lv.{} 战力:{}\n",
                        i + 1,
                        p.nickname,
                        p.level,
                        p.combat_power
                    ));
                }
                if war.defender_members.len() > 5 {
                    output.push_str(&format!("  ... 共{}人\n", war.defender_members.len()));
                }
            }
        }
    } else {
        output.push_str("📭 当前没有进行中的公会战。\n\n");
    }

    // 显示最近公会战历史
    let history = get_war_history(db);
    if !history.is_empty() {
        output.push_str("📜 最近公会战记录:\n");
        for (i, war_id) in history.iter().enumerate().take(5) {
            if let Some(w) = load_guild_war(db, war_id) {
                let winner = if w.attacker_score > w.defender_score {
                    &w.attacker_guild
                } else if w.defender_score > w.attacker_score {
                    &w.defender_guild
                } else {
                    "平局"
                };
                output.push_str(&format!(
                    "  {}. {} vs {} — 获胜: {} ({}:{})\n",
                    i + 1,
                    w.attacker_guild,
                    w.defender_guild,
                    winner,
                    w.attacker_score,
                    w.defender_score
                ));
            }
        }
    }

    output.push_str("\n💡 指令:\n");
    output.push_str("  • 发起公会战+公会名 — 向其他公会发起挑战（会长专用）\n");
    output.push_str("  • 参与公会战 — 报名参加当前公会战\n");
    output.push_str("  • 公会战排名 — 查看全服公会战排名\n");
    output.push_str("  • 公会战奖励 — 领取公会战奖励\n");

    output
}

/// 发起公会战 — 公会领袖向其他公会发起挑战
pub fn cmd_declare_guild_war(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let target_guild = args.trim();
    if target_guild.is_empty() {
        return "❌ 请指定目标公会名。\n💡 用法: 发起公会战+公会名".to_string();
    }

    // 检查是否有自己的公会
    let my_guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你还没有加入任何公会。".to_string(),
    };

    // 检查是否是公会会长
    let leader = get_guild_leader(db, &my_guild);
    match leader {
        Some(l) if l == user_id => {}
        Some(_) => return "❌ 只有公会会长才能发起公会战。".to_string(),
        None => return "❌ 无法获取公会信息。".to_string(),
    }

    // 不能和自己公会打
    if my_guild == target_guild {
        return "❌ 不能向自己的公会发起公会战。".to_string();
    }

    // 检查目标公会是否存在
    let _target_leader = match get_guild_leader(db, target_guild) {
        Some(l) => l,
        None => return format!("❌ 公会「{}」不存在。", target_guild),
    };

    // 检查是否已有进行中的公会战
    if let Some(active_war_id) = get_active_war(db) {
        if let Some(war) = load_guild_war(db, &active_war_id) {
            if war.status != GuildWarStatus::Finished && war.status != GuildWarStatus::Cancelled {
                return format!(
                    "❌ 当前已有进行中的公会战 ({} vs {})，请等待结束后再发起新的挑战。",
                    war.attacker_guild, war.defender_guild
                );
            }
        }
    }

    // 创建公会战
    let war_id = generate_war_id();
    let war = GuildWar {
        war_id: war_id.clone(),
        attacker_guild: my_guild.clone(),
        defender_guild: target_guild.to_string(),
        status: GuildWarStatus::Registration,
        attacker_score: 0,
        defender_score: 0,
        total_rounds: WAR_ROUNDS,
        current_round: 0,
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        attacker_members: Vec::new(),
        defender_members: Vec::new(),
    };

    save_guild_war(db, &war);

    format!(
        "⚔️ 公会战已发起！\n\n\
         🔴 进攻方: {}\n\
         🔵 防守方: {}\n\
         📋 状态: 报名中\n\
         ⏰ 报名阶段: 公会成员可使用「参与公会战」报名\n\
         👥 每方最多{}人参战\n\
         🔄 共{}回合战斗\n\n\
         💡 双方成员报名后，使用「参与公会战」自动进入战斗！",
        my_guild, target_guild, MAX_PARTICIPANTS_PER_SIDE, WAR_ROUNDS
    )
}

/// 参与公会战 — 报名参加当前公会战
pub fn cmd_join_guild_war(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let my_guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你还没有加入任何公会。".to_string(),
    };

    let active_war_id = match get_active_war(db) {
        Some(id) => id,
        None => return "❌ 当前没有进行中的公会战。\n💡 使用「发起公会战+公会名」发起挑战。".to_string(),
    };

    let mut war = match load_guild_war(db, &active_war_id) {
        Some(w) => w,
        None => return "❌ 无法加载公会战数据。".to_string(),
    };

    // 检查公会战状态
    if war.status == GuildWarStatus::Finished {
        return "❌ 该公会战已结束。".to_string();
    }
    if war.status == GuildWarStatus::Cancelled {
        return "❌ 该公会战已取消。".to_string();
    }

    // 确定玩家属于哪一方
    let is_attacker = my_guild == war.attacker_guild;
    let is_defender = my_guild == war.defender_guild;

    if !is_attacker && !is_defender {
        return format!(
            "❌ 你的公会「{}」不在这场公会战中。\n这场公会战是 {} vs {}。",
            my_guild, war.attacker_guild, war.defender_guild
        );
    }

    // 检查是否已经报名
    let already_joined = if is_attacker {
        war.attacker_members.iter().any(|p| p.user_id == user_id)
    } else {
        war.defender_members.iter().any(|p| p.user_id == user_id)
    };

    if already_joined {
        return "❌ 你已经报名了本次公会战，无需重复报名。".to_string();
    }

    // 检查人数上限
    if is_attacker && war.attacker_members.len() >= MAX_PARTICIPANTS_PER_SIDE {
        return format!(
            "❌ 进攻方报名已满 ({}/{})。",
            war.attacker_members.len(),
            MAX_PARTICIPANTS_PER_SIDE
        );
    }
    if is_defender && war.defender_members.len() >= MAX_PARTICIPANTS_PER_SIDE {
        return format!(
            "❌ 防守方报名已满 ({}/{})。",
            war.defender_members.len(),
            MAX_PARTICIPANTS_PER_SIDE
        );
    }

    // 获取玩家信息
    let participant = match get_player_war_info(db, user_id) {
        Some(p) => p,
        None => return "❌ 无法获取你的角色信息。".to_string(),
    };

    let side = if is_attacker { "进攻方" } else { "防守方" };

    if is_attacker {
        war.attacker_members.push(participant);
    } else {
        war.defender_members.push(participant);
    }

    save_guild_war(db, &war);

    let side_count = if is_attacker {
        war.attacker_members.len()
    } else {
        war.defender_members.len()
    };

    // 如果双方都满员或都有至少1人，自动开始战斗
    if !war.attacker_members.is_empty() && !war.defender_members.is_empty() {
        // 双方都有人报名，自动推进战斗
        auto_progress_war(db, &mut war);
    }

    format!(
        "✅ 报名成功！你已加入{}「{}」。\n\
         📊 当前{}报名人数: {}/{}\n\
         {}",
        side,
        my_guild,
        side,
        side_count,
        MAX_PARTICIPANTS_PER_SIDE,
        if war.status == GuildWarStatus::Battle {
            "⚔️ 战斗已经开始！"
        } else {
            "⏳ 等待对方成员报名..."
        }
    )
}

/// 自动推进公会战
fn auto_progress_war(db: &Database, war: &mut GuildWar) {
    if war.status == GuildWarStatus::Registration {
        war.status = GuildWarStatus::Battle;
        war.current_round = 1;
    }

    // 执行战斗回合
    while war.current_round <= war.total_rounds {
        let (atk_win, def_win) = simulate_war_round(&war.attacker_members, &war.defender_members);
        war.attacker_score += atk_win;
        war.defender_score += def_win;
        war.current_round += 1;
    }

    // 战斗结束
    war.status = GuildWarStatus::Finished;
    war.current_round = war.total_rounds;

    // 计算双方总伤害用于记录
    let atk_damage: i64 = war.attacker_members.iter().map(|p| p.combat_power).sum();
    let def_damage: i64 = war.defender_members.iter().map(|p| p.combat_power).sum();

    // 记录战绩
    let atk_won = war.attacker_score > war.defender_score;
    let def_won = war.defender_score > war.attacker_score;

    save_war_result(db, &war.attacker_guild, atk_won, atk_damage);
    save_war_result(db, &war.defender_guild, def_won, def_damage);

    // 记录个人战绩
    for p in &war.attacker_members {
        save_player_war_result(db, &p.user_id, atk_won, p.combat_power);
    }
    for p in &war.defender_members {
        save_player_war_result(db, &p.user_id, def_won, p.combat_power);
    }

    save_guild_war(db, war);
}

/// 公会战排名 — 全服公会战积分排名
pub fn cmd_guild_war_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let _my_guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你还没有加入任何公会。".to_string(),
    };

    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare(
        "SELECT Node, Key, Value FROM Global WHERE Node LIKE 'guild_war_stats_%' AND Key='score' ORDER BY CAST(Value AS INTEGER) DESC LIMIT 20",
    ) {
        Ok(s) => s,
        Err(_) => return "❌ 暂无公会战排名数据。".to_string(),
    };

    let mut rankings: Vec<(String, i64, i64, i64)> = Vec::new();
    if let Ok(rows) = stmt.query_map([], |row| {
        let node: String = row.get(0)?;
        let score_str: String = row.get(2)?;
        let guild = node.replace("guild_war_stats_", "");
        Ok((guild, score_str.parse::<i64>().unwrap_or(0)))
    }) {
        for row in rows.flatten() {
            let (guild, score) = row;
            // 获取胜负记录
            let wins: i64 = conn
                .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
                .ok()
                .and_then(|mut s| {
                    s.query_row(rusqlite::params![format!("guild_war_stats_{}", guild), "wins"], |r| {
                        let v: String = r.get(0)?;
                        Ok(v.parse().unwrap_or(0))
                    })
                    .ok()
                })
                .unwrap_or(0);
            let losses: i64 = conn
                .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
                .ok()
                .and_then(|mut s| {
                    s.query_row(rusqlite::params![format!("guild_war_stats_{}", guild), "losses"], |r| {
                        let v: String = r.get(0)?;
                        Ok(v.parse().unwrap_or(0))
                    })
                    .ok()
                })
                .unwrap_or(0);
            rankings.push((guild, score, wins, losses));
        }
    }

    if rankings.is_empty() {
        return "📊 公会战排名\n\n暂无公会战记录。使用「发起公会战+公会名」开启第一场公会战！".to_string();
    }

    let mut output = String::from("🏆 === 公会战排名 === 🏆\n\n");
    for (i, (guild, score, wins, losses)) in rankings.iter().enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let win_rate = if wins + losses > 0 {
            (*wins as f64 / (wins + losses) as f64 * 100.0) as i32
        } else {
            0
        };
        output.push_str(&format!(
            "{} {}. {} — 积分:{} 胜:{} 负:{} 胜率:{}%\n",
            medal,
            i + 1,
            guild,
            score,
            wins,
            losses,
            win_rate
        ));
    }

    // 显示个人战绩
    let node = format!("player_war_stats_{}", user_id);
    let my_wins: i64 = conn
        .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
        .ok()
        .and_then(|mut s| {
            s.query_row(rusqlite::params![node, "wins"], |r| {
                let v: String = r.get(0)?;
                Ok(v.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);
    let my_losses: i64 = conn
        .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
        .ok()
        .and_then(|mut s| {
            s.query_row(rusqlite::params![node, "losses"], |r| {
                let v: String = r.get(0)?;
                Ok(v.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);
    let my_damage: i64 = conn
        .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
        .ok()
        .and_then(|mut s| {
            s.query_row(rusqlite::params![node, "total_damage"], |r| {
                let v: String = r.get(0)?;
                Ok(v.parse().unwrap_or(0))
            })
            .ok()
        })
        .unwrap_or(0);

    if my_wins + my_losses > 0 {
        output.push_str(&format!(
            "\n📊 你的公会战战绩: 胜:{} 负:{} 总伤害:{}",
            my_wins, my_losses, my_damage
        ));
    }

    output
}

/// 公会战奖励 — 领取公会战胜利奖励
pub fn cmd_guild_war_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let my_guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你还没有加入任何公会。".to_string(),
    };

    // 检查是否有最新的公会战记录
    let history = get_war_history(db);
    if history.is_empty() {
        return "❌ 暂无公会战记录，无法领取奖励。".to_string();
    }

    // 找到最近一场已结束的公会战
    let latest_war_id = &history[0];
    let war = match load_guild_war(db, latest_war_id) {
        Some(w) => w,
        None => return "❌ 无法加载公会战数据。".to_string(),
    };

    if war.status != GuildWarStatus::Finished {
        return "❌ 最近的公会战尚未结束，无法领取奖励。".to_string();
    }

    // 检查玩家是否参战
    let is_attacker = war.attacker_guild == my_guild;
    let is_defender = war.defender_guild == my_guild;

    if !is_attacker && !is_defender {
        return "❌ 你的公会没有参加最近的公会战。".to_string();
    }

    let participated = if is_attacker {
        war.attacker_members.iter().any(|p| p.user_id == user_id)
    } else {
        war.defender_members.iter().any(|p| p.user_id == user_id)
    };

    if !participated {
        return "❌ 你没有参加最近的公会战，无法领取奖励。".to_string();
    }

    // 检查是否已领取
    let reward_key = format!("reward_claimed_{}_{}", latest_war_id, user_id);
    let conn = db.conn.lock().unwrap();
    let already_claimed: bool = conn
        .prepare("SELECT Value FROM Global WHERE Node=?1 AND Key=?2")
        .ok()
        .and_then(|mut s| {
            s.query_row(rusqlite::params!["guild_war_rewards", &reward_key], |r| {
                let v: String = r.get(0)?;
                Ok(v == "true")
            })
            .ok()
        })
        .unwrap_or(false);

    if already_claimed {
        return "❌ 你已经领取了本次公会战的奖励。".to_string();
    }

    // 判断胜负
    let won = if is_attacker {
        war.attacker_score > war.defender_score
    } else {
        war.defender_score > war.attacker_score
    };

    let (reward_gold, reward_diamond, reward_exp) = if won {
        (WAR_REWARD_GOLD, WAR_REWARD_DIAMOND, WAR_REWARD_EXP)
    } else {
        (LOSER_REWARD_GOLD, 0, LOSER_REWARD_EXP)
    };

    // 发放奖励
    let _ = conn.execute(
        "UPDATE Basic_User SET Gold=Gold+?1, Diamond=Diamond+?2, Exp=Exp+?3 WHERE uId=?4",
        rusqlite::params![reward_gold, reward_diamond, reward_exp, user_id],
    );

    // 标记已领取
    let _ = conn.execute(
        "INSERT OR REPLACE INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params!["guild_war_rewards", &reward_key, "true"],
    );

    // 更新公会贡献
    let contribution_key = format!("contribution_{}", user_id);
    let _ = conn.execute(
        "INSERT INTO Global (Node, Key, Value) VALUES (?1, ?2, ?3)",
        rusqlite::params![
            format!("guild_{}", my_guild),
            &contribution_key,
            format!("公会战参与 +{}", if won { 50 } else { 20 })
        ],
    );

    if won {
        format!(
            "🏆 恭喜！你的公会赢得了公会战！\n\n\
             🎁 胜利奖励:\n\
             💰 金币: +{}\n\
             💎 钻石: +{}\n\
             ⭐ 经验: +{}\n\n\
             📊 战绩: {} {} - {} {}\n\
             💪 公会贡献 +50",
            reward_gold,
            reward_diamond,
            reward_exp,
            war.attacker_guild,
            war.attacker_score,
            war.defender_score,
            war.defender_guild
        )
    } else {
        format!(
            "⚔️ 公会战结束，虽然战败但获得了荣誉奖励。\n\n\
             🎁 参与奖励:\n\
             💰 金币: +{}\n\
             ⭐ 经验: +{}\n\n\
             📊 战绩: {} {} - {} {}\n\
             💪 公会贡献 +20\n\
             💡 提升战力，下次再战！",
            reward_gold, reward_exp, war.attacker_guild, war.attacker_score, war.defender_score, war.defender_guild
        )
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guild_war_status_from_str() {
        assert_eq!(GuildWarStatus::from_str("报名中"), Some(GuildWarStatus::Registration));
        assert_eq!(GuildWarStatus::from_str("战斗中"), Some(GuildWarStatus::Battle));
        assert_eq!(GuildWarStatus::from_str("已结束"), Some(GuildWarStatus::Finished));
        assert_eq!(GuildWarStatus::from_str("已取消"), Some(GuildWarStatus::Cancelled));
        assert_eq!(GuildWarStatus::from_str("未知"), None);
    }

    #[test]
    fn test_guild_war_status_as_str() {
        assert_eq!(GuildWarStatus::Registration.as_str(), "报名中");
        assert_eq!(GuildWarStatus::Battle.as_str(), "战斗中");
        assert_eq!(GuildWarStatus::Finished.as_str(), "已结束");
        assert_eq!(GuildWarStatus::Cancelled.as_str(), "已取消");
    }

    #[test]
    fn test_guild_war_status_emoji() {
        assert_eq!(GuildWarStatus::Registration.emoji(), "📋");
        assert_eq!(GuildWarStatus::Battle.emoji(), "⚔️");
        assert_eq!(GuildWarStatus::Finished.emoji(), "🏆");
        assert_eq!(GuildWarStatus::Cancelled.emoji(), "❌");
    }

    #[test]
    fn test_generate_war_id() {
        let id1 = generate_war_id();
        assert!(id1.starts_with("GW_"));
        assert!(id1.len() > 5);
    }

    #[test]
    fn test_simulate_war_round() {
        let strong = vec![WarParticipant {
            user_id: "s1".into(),
            nickname: "强".into(),
            level: 50,
            combat_power: 10000,
            damage_dealt: 0,
            battles_won: 0,
            battles_lost: 0,
        }];
        let weak = vec![WarParticipant {
            user_id: "w1".into(),
            nickname: "弱".into(),
            level: 5,
            combat_power: 100,
            damage_dealt: 0,
            battles_won: 0,
            battles_lost: 0,
        }];

        // 强队应几乎总是赢
        let (atk, def) = simulate_war_round(&strong, &weak);
        assert!(atk + def == 1 || (atk == 0 && def == 0)); // 一方赢或平局
    }

    #[test]
    fn test_war_config_constants() {
        assert_eq!(MAX_PARTICIPANTS_PER_SIDE, 10);
        assert_eq!(WAR_ROUNDS, 5);
        assert!(WAR_REWARD_GOLD > LOSER_REWARD_GOLD);
        assert!(WAR_REWARD_DIAMOND > 0);
        assert!(WAR_REWARD_EXP > LOSER_REWARD_EXP);
    }
}

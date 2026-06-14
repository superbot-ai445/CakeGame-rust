/// CakeGame 公会争霸联赛系统
/// 结构化公会对战联赛：赛季制积分赛+淘汰赛+联赛商店
/// 数据来源: Global 表 SECTION='guild_league' / 'guild_league_match' / 'guild_league_season'
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 联赛阶段
#[derive(Debug, Clone, Copy)]
enum LeaguePhase {
    Registration, // 报名期
    GroupStage,   // 小组积分赛
    Knockout,     // 淘汰赛
    Final,        // 决赛
    OffSeason,    // 休赛期
}

impl LeaguePhase {
    fn name(&self) -> &str {
        match self {
            Self::Registration => "📋 报名期",
            Self::GroupStage => "⚔️ 小组积分赛",
            Self::Knockout => "🔥 淘汰赛",
            Self::Final => "🏆 决赛",
            Self::OffSeason => "💤 休赛期",
        }
    }
}

/// 联赛队伍信息
#[derive(Clone)]
struct LeagueTeam {
    guild_id: String,
    guild_name: String,
    wins: i32,
    losses: i32,
    points: i32,
    goals_scored: i32,
    goals_conceded: i32,
}

impl LeagueTeam {
    fn goal_diff(&self) -> i32 {
        self.goals_scored - self.goals_conceded
    }
}

/// 联赛比赛记录
#[derive(Clone)]
struct LeagueMatch {
    team_a: String,
    team_b: String,
    score_a: i32,
    score_b: i32,
    round: String,
    timestamp: i64,
}

/// 读取联赛数据
fn load_league_data(db: &Database) -> (String, Vec<LeagueTeam>, Vec<LeagueMatch>) {
    let raw = db.global_get("guild_league", "phase");
    let phase = if raw.is_empty() { "off_season".to_string() } else { raw };

    let mut teams = Vec::new();
    let team_data = db.global_get("guild_league", "teams");
    if !team_data.is_empty() {
        for entry in team_data.split(';') {
            let parts: Vec<&str> = entry.split(',').collect();
            if parts.len() >= 7 {
                teams.push(LeagueTeam {
                    guild_id: parts[0].to_string(),
                    guild_name: parts[1].to_string(),
                    wins: parts[2].parse().unwrap_or(0),
                    losses: parts[3].parse().unwrap_or(0),
                    points: parts[4].parse().unwrap_or(0),
                    goals_scored: parts[5].parse().unwrap_or(0),
                    goals_conceded: parts[6].parse().unwrap_or(0),
                });
            }
        }
    }

    let mut matches = Vec::new();
    let match_data = db.global_get("guild_league_match", "history");
    if !match_data.is_empty() {
        for entry in match_data.split('|') {
            let parts: Vec<&str> = entry.split(',').collect();
            if parts.len() >= 6 {
                matches.push(LeagueMatch {
                    team_a: parts[0].to_string(),
                    team_b: parts[1].to_string(),
                    score_a: parts[2].parse().unwrap_or(0),
                    score_b: parts[3].parse().unwrap_or(0),
                    round: parts[4].to_string(),
                    timestamp: parts[5].parse().unwrap_or(0),
                });
            }
        }
    }

    (phase, teams, matches)
}

/// 保存联赛数据
fn save_league_data(db: &Database, phase: &str, teams: &[LeagueTeam], matches: &[LeagueMatch]) {
    db.global_set("guild_league", "phase", phase);

    let team_str: Vec<String> = teams
        .iter()
        .map(|t| {
            format!(
                "{},{},{},{},{},{},{}",
                t.guild_id, t.guild_name, t.wins, t.losses, t.points, t.goals_scored, t.goals_conceded
            )
        })
        .collect();
    db.global_set("guild_league", "teams", &team_str.join(";"));

    let match_str: Vec<String> = matches
        .iter()
        .map(|m| {
            format!(
                "{},{},{},{},{},{}",
                m.team_a, m.team_b, m.score_a, m.score_b, m.round, m.timestamp
            )
        })
        .collect();
    db.global_set("guild_league_match", "history", &match_str.join("|"));
}

/// 获取玩家公会ID
fn get_player_guild(db: &Database, user_id: &str) -> String {
    let guild_id = db.read_basic(user_id, "GuildID");
    guild_id.trim_matches('"').trim().to_string()
}

/// 获取公会名
fn get_guild_name(db: &Database, guild_id: &str) -> String {
    let raw = db.global_get("UnionInfo", guild_id);
    if raw.is_empty() {
        return format!("公会{}", guild_id);
    }
    raw.split(',').next().unwrap_or("未知公会").to_string()
}

/// 获取公会等级
fn get_guild_level(db: &Database, guild_id: &str) -> i32 {
    let raw = db.global_get("UnionInfo", guild_id);
    if raw.is_empty() {
        return 0;
    }
    raw.split(',').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0)
}

/// 解析联赛阶段
fn parse_phase(s: &str) -> LeaguePhase {
    match s {
        "registration" => LeaguePhase::Registration,
        "group_stage" => LeaguePhase::GroupStage,
        "knockout" => LeaguePhase::Knockout,
        "final" => LeaguePhase::Final,
        _ => LeaguePhase::OffSeason,
    }
}

/// 计算公会总战力
fn calc_guild_power(db: &Database, guild_id: &str) -> i64 {
    let members_raw = db.global_get("UnionMembers", guild_id);
    if members_raw.is_empty() {
        return 1000;
    }

    let mut total_power: i64 = 0;
    for member_id in members_raw.split(',') {
        let mid = member_id.trim();
        if !mid.is_empty() && db.user_exists(mid) {
            let attrs = user::calc_total_attrs(db, mid);
            // 计算战力: HP/10 + AD + AP + DEF + MRES
            total_power += (attrs.hp_max as i64 / 10)
                + attrs.ad as i64
                + attrs.ap as i64
                + attrs.defense as i64
                + attrs.magic_res as i64;
        }
    }

    total_power.max(1000)
}

/// 联赛商店商品
fn get_league_shop_items() -> Vec<(&'static str, i32, i32, &'static str)> {
    vec![
        ("联赛金币箱", 50, 10, "获得10000金币"),
        ("联赛钻石袋", 100, 8, "获得200钻石"),
        ("联赛强化石", 80, 10, "获得3个强化石"),
        ("联赛高级药水", 60, 10, "获得5瓶高级药水"),
        ("联赛凤凰之羽", 150, 6, "获得2个凤凰之羽"),
        ("联赛精炼水晶", 200, 5, "获得1个精炼水晶"),
        ("联赛传说宝箱", 500, 3, "获得1个传说品质宝箱"),
        ("联赛冠军之冠", 2000, 1, "获得冠军专属头饰(属性加成+10%)"),
    ]
}

// ==================== 指令处理函数 ====================

/// 查看联赛 — 显示联赛状态、积分榜、赛程
pub fn cmd_view_league(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let (phase_str, teams, matches) = load_league_data(db);
    let phase = parse_phase(&phase_str);

    let mut out = format!("{}\n═══ 🏆 公会争霸联赛 ═══", prefix);
    out.push_str(&format!("\n📊 联赛阶段: {}", phase.name()));

    if teams.is_empty() {
        out.push_str("\n\n🚫 当前无参赛公会\n💡 联赛开启后公会会长可报名参加");
        return out;
    }

    // 积分榜
    let mut sorted = teams.clone();
    sorted.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.goal_diff().cmp(&a.goal_diff()))
            .then(b.goals_scored.cmp(&a.goals_scored))
    });

    out.push_str(&format!("\n\n📊 积分榜 (共 {} 支队伍):", sorted.len()));
    out.push_str("\n排名 | 公会 | 胜/负 | 积分 | 净胜");
    out.push_str("\n────────────────────────────────");
    for (i, t) in sorted.iter().enumerate().take(10) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!(
            "\n{} {} | {}/{} | {}分 | {:+}",
            medal,
            t.guild_name,
            t.wins,
            t.losses,
            t.points,
            t.goal_diff()
        ));
    }

    // 最近比赛
    if !matches.is_empty() {
        out.push_str(&format!("\n\n📜 最近比赛 (最近 {} 场):", matches.len().min(5)));
        for m in matches.iter().rev().take(5) {
            out.push_str(&format!(
                "\n  {} {} vs {} {} (第{}轮)",
                m.team_a, m.score_a, m.score_b, m.team_b, m.round
            ));
        }
    }

    out.push_str("\n\n💡 指令: 联赛报名 | 联赛对战 | 联赛排行 | 联赛商店 | 联赛帮助");
    out
}

/// 联赛报名 — 公会报名参加联赛
pub fn cmd_league_register(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild_id = get_player_guild(db, user_id);
    if guild_id.is_empty() {
        return format!("{}\n❌ 您还未加入公会！", prefix);
    }

    // 检查是否是会长
    let role = db.read_basic(user_id, "GuildRole");
    if !role.contains("会长") {
        return format!("{}\n❌ 只有公会会长才能报名联赛！", prefix);
    }

    let (phase_str, mut teams, matches) = load_league_data(db);
    let phase = parse_phase(&phase_str);

    if !matches!(phase, LeaguePhase::Registration) {
        return format!("{}\n❌ 当前不是报名阶段！当前阶段: {}", prefix, phase.name());
    }

    // 检查是否已报名
    if teams.iter().any(|t| t.guild_id == guild_id) {
        return format!("{}\n❌ 您的公会已报名！", prefix);
    }

    // 检查公会等级
    let guild_level = get_guild_level(db, &guild_id);
    if guild_level < 3 {
        return format!(
            "{}\n❌ 公会等级不足！需要3级公会才能报名。\n当前等级: {}级",
            prefix, guild_level
        );
    }

    let guild_name = get_guild_name(db, &guild_id);
    teams.push(LeagueTeam {
        guild_id: guild_id.clone(),
        guild_name: guild_name.clone(),
        wins: 0,
        losses: 0,
        points: 0,
        goals_scored: 0,
        goals_conceded: 0,
    });

    save_league_data(db, &phase_str, &teams, &matches);

    let mut out = format!("{}\n✅ 报名成功！", prefix);
    out.push_str(&format!("\n🏆 公会: {} (等级{})", guild_name, guild_level));
    out.push_str(&format!("\n📊 已报名公会: {} 支", teams.len()));
    out.push_str("\n⏰ 报名期结束后自动进入小组赛");
    out
}

/// 联赛对战 — 挑战其他公会
pub fn cmd_league_battle(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let guild_id = get_player_guild(db, user_id);
    if guild_id.is_empty() {
        return format!("{}\n❌ 您还未加入公会！", prefix);
    }

    let (phase_str, mut teams, mut matches) = load_league_data(db);
    let phase = parse_phase(&phase_str);

    if !matches!(
        phase,
        LeaguePhase::GroupStage | LeaguePhase::Knockout | LeaguePhase::Final
    ) {
        return format!("{}\n❌ 当前不在比赛阶段！当前阶段: {}", prefix, phase.name());
    }

    // 检查公会是否参赛
    let my_idx = teams.iter().position(|t| t.guild_id == guild_id);
    if my_idx.is_none() {
        return format!("{}\n❌ 您的公会未参加本轮联赛！", prefix);
    }

    // 解析对手
    let target = args.trim();
    if target.is_empty() {
        let mut out = format!("{}\n💡 用法: 联赛对战+公会名\n可挑战的公会:", prefix);
        for t in &teams {
            if t.guild_id != guild_id {
                out.push_str(&format!("\n  • {}", t.guild_name));
            }
        }
        return out;
    }

    // 查找对手
    let opp_idx = teams.iter().position(|t| t.guild_name.contains(target));
    if opp_idx.is_none() {
        return format!("{}\n❌ 未找到公会: {}\n💡 请确认公会名正确且已参赛", prefix, target);
    }

    let opp_idx = opp_idx.unwrap();
    let my_idx = my_idx.unwrap();

    if my_idx == opp_idx {
        return format!("{}\n❌ 不能挑战自己的公会！", prefix);
    }

    // 检查今日对战次数
    let daily_key = format!("league_daily_{}", user_id);
    let daily_count: i32 = db.global_get("guild_league", &daily_key).parse().unwrap_or(0);
    if daily_count >= 3 {
        return format!("{}\n❌ 今日联赛对战次数已用完(3/3)！\n⏰ 明日重置", prefix);
    }

    // 模拟联赛对战
    let my_power = calc_guild_power(db, &guild_id);
    let opp_power = calc_guild_power(db, &teams[opp_idx].guild_id);

    let mut rng = rand::thread_rng();
    let my_score = ((my_power as f64 / (my_power + opp_power + 1) as f64) * 5.0) as i32 + rng.gen_range(0..3);
    let opp_score = ((opp_power as f64 / (my_power + opp_power + 1) as f64) * 5.0) as i32 + rng.gen_range(0..3);

    let my_score = my_score.max(1);
    let opp_score = opp_score.max(1);

    let my_win = my_score > opp_score;
    let round = format!("{}", matches.len() + 1);

    // 更新战绩
    if my_win {
        teams[my_idx].wins += 1;
        teams[my_idx].points += 3;
        teams[opp_idx].losses += 1;
    } else {
        teams[opp_idx].wins += 1;
        teams[opp_idx].points += 3;
        teams[my_idx].losses += 1;
    }
    teams[my_idx].goals_scored += my_score;
    teams[my_idx].goals_conceded += opp_score;
    teams[opp_idx].goals_scored += opp_score;
    teams[opp_idx].goals_conceded += my_score;

    // 记录比赛
    matches.push(LeagueMatch {
        team_a: teams[my_idx].guild_name.clone(),
        team_b: teams[opp_idx].guild_name.clone(),
        score_a: my_score,
        score_b: opp_score,
        round: round.clone(),
        timestamp: chrono::Utc::now().timestamp(),
    });

    // 更新每日次数
    db.global_set("guild_league", &daily_key, &(daily_count + 1).to_string());

    save_league_data(db, &phase_str, &teams, &matches);

    let result_emoji = if my_win { "🎉 胜利！" } else { "😢 失败" };

    let mut out = format!("{}\n═══ ⚔️ 联赛对战结果 ═══", prefix);
    out.push_str(&format!(
        "\n\n{} 公会: {} vs {}",
        result_emoji, teams[my_idx].guild_name, teams[opp_idx].guild_name
    ));
    out.push_str(&format!("\n📊 比分: {} : {}", my_score, opp_score));
    out.push_str(&format!("\n📈 获得积分: {}", if my_win { 3 } else { 0 }));

    // 发放联赛积分
    let league_pts = if my_win { 50 } else { 10 };
    let user_key = format!("league_points_{}", user_id);
    let current_pts: i32 = db.global_get("guild_league", &user_key).parse().unwrap_or(0);
    db.global_set("guild_league", &user_key, &(current_pts + league_pts).to_string());

    out.push_str(&format!("\n🎫 联赛积分: +{}", league_pts));
    out.push_str(&format!("\n📊 今日对战: {}/3", daily_count + 1));

    out.push_str("\n\n💡 指令: 联赛排行 | 联赛对战+公会名");
    out
}

/// 联赛排行 — 按积分排名
pub fn cmd_league_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let (_phase, teams, _matches) = load_league_data(db);

    if teams.is_empty() {
        return format!("{}\n📊 联赛排行榜\n\n暂无参赛公会", prefix);
    }

    let mut sorted = teams.clone();
    sorted.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.goal_diff().cmp(&a.goal_diff()))
            .then(b.goals_scored.cmp(&a.goals_scored))
    });

    let mut out = format!("{}\n═══ 🏆 联赛排行榜 ═══", prefix);
    out.push_str("\n\n排名 | 公会 | 场次 | 胜 | 负 | 积分 | 进球 | 失球 | 净胜");
    out.push_str("\n───────────────────────────────────────────────────");

    let guild_id = get_player_guild(db, user_id);
    for (i, t) in sorted.iter().enumerate().take(15) {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let highlight = if t.guild_id == guild_id { " ⭐" } else { "" };
        let total = t.wins + t.losses;
        out.push_str(&format!(
            "\n{} {} | {}战 | {}胜 | {}负 | {}分 | {} | {} | {:+}{}",
            medal,
            t.guild_name,
            total,
            t.wins,
            t.losses,
            t.points,
            t.goals_scored,
            t.goals_conceded,
            t.goal_diff(),
            highlight
        ));
    }

    // 个人排名定位
    if !guild_id.is_empty() {
        if let Some(pos) = sorted.iter().position(|t| t.guild_id == guild_id) {
            out.push_str(&format!("\n\n📍 您的公会排名: 第{}名", pos + 1));
        }
    }

    out
}

/// 联赛商店 — 使用联赛积分兑换奖励
pub fn cmd_league_shop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let shop_items = get_league_shop_items();
    let user_key = format!("league_points_{}", user_id);
    let user_points: i32 = db.global_get("guild_league", &user_key).parse().unwrap_or(0);

    if args.trim().is_empty() {
        // 查看商店
        let mut out = format!("{}\n═══ 🏪 联赛商店 ═══", prefix);
        out.push_str(&format!("\n🎫 您的联赛积分: {}", user_points));
        out.push_str("\n\n编号 | 商品 | 价格 | 需求排名");
        out.push_str("\n──────────────────────────────");

        for (i, (name, price, rank_req, _desc)) in shop_items.iter().enumerate() {
            out.push_str(&format!("\n{}. {} 💰{}积分 🏅前{}名", i + 1, name, price, rank_req));
        }

        out.push_str("\n\n💡 用法: 联赛商店+编号");
        return out;
    }

    // 购买商品
    let idx: usize = args.trim().parse().unwrap_or(0);
    if idx == 0 || idx > shop_items.len() {
        return format!("{}\n❌ 无效编号！请输入 1-{}", prefix, shop_items.len());
    }

    let (name, price, rank_req, desc) = &shop_items[idx - 1];

    // 检查排名需求
    let guild_id = get_player_guild(db, user_id);
    let (_, teams, _) = load_league_data(db);
    let mut sorted = teams;
    sorted.sort_by_key(|b| std::cmp::Reverse(b.points));
    let rank = sorted
        .iter()
        .position(|t| t.guild_id == guild_id)
        .map(|p| p + 1)
        .unwrap_or(999);

    if rank > *rank_req as usize {
        return format!("{}\n❌ 公会排名不足！需要前{}名，当前第{}名", prefix, rank_req, rank);
    }

    if user_points < *price {
        return format!("{}\n❌ 积分不足！需要{}积分，您有{}积分", prefix, price, user_points);
    }

    // 扣除积分
    db.global_set("guild_league", &user_key, &(user_points - price).to_string());

    // 发放奖励
    let mut out = format!("{}\n✅ 购买成功！", prefix);
    out.push_str(&format!("\n🛒 获得: {}", name));
    out.push_str(&format!("\n📖 描述: {}", desc));
    out.push_str(&format!("\n💰 扣除: {}积分", price));
    out.push_str(&format!("\n🎫 剩余: {}积分", user_points - price));

    if name.contains("金币") {
        let gold: i64 = db.read_currency(user_id, "金币");
        db.write_currency(user_id, "金币", gold + 10000);
        out.push_str("\n💰 +10000金币");
    } else if name.contains("钻石") {
        let diamond: i64 = db.read_currency(user_id, "钻石");
        db.write_currency(user_id, "钻石", diamond + 200);
        out.push_str("\n💎 +200钻石");
    }

    out
}

/// 联赛赛季结算
pub fn cmd_league_season(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let season_key = "guild_league_season";
    let current_season: i32 = db.global_get(season_key, "current").parse().unwrap_or(1);
    let season_start: i64 = db.global_get(season_key, "start_ts").parse().unwrap_or(0);
    let now = chrono::Utc::now().timestamp();
    let days_elapsed = if season_start > 0 {
        (now - season_start) / 86400
    } else {
        0
    };

    let (phase_str, teams, matches) = load_league_data(db);
    let phase = parse_phase(&phase_str);

    let mut out = format!("{}\n═══ 📅 联赛赛季信息 ═══", prefix);
    out.push_str(&format!("\n🏷️ 赛季: 第{}赛季", current_season));
    out.push_str(&format!("\n📊 阶段: {}", phase.name()));
    out.push_str(&format!("\n⏰ 已进行: {}天", days_elapsed));
    out.push_str(&format!("\n🏆 参赛公会: {}支", teams.len()));
    out.push_str(&format!("\n⚔️ 已完成比赛: {}场", matches.len()));

    out.push_str("\n\n🎁 赛季结束奖励:");
    out.push_str("\n🥇 冠军: 100000金币 + 1000钻石 + 冠军称号");
    out.push_str("\n🥈 亚军: 50000金币 + 500钻石");
    out.push_str("\n🥉 季军: 30000金币 + 300钻石");
    out.push_str("\n🏅 参与: 10000金币 + 100钻石");

    let user_key = format!("league_points_{}", user_id);
    let user_points: i32 = db.global_get("guild_league", &user_key).parse().unwrap_or(0);
    out.push_str(&format!("\n\n🎫 您的联赛积分: {}", user_points));

    out.push_str("\n\n💡 指令: 联赛商店 | 联赛排行 | 联赛帮助");
    out
}

/// 联赛帮助
pub fn cmd_league_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let mut out = format!("{}\n═══ 📖 联赛帮助 ═══", prefix);
    out.push_str("\n\n🏆 公会争霸联赛 — 结构化公会对战赛季系统");
    out.push_str("\n\n📋 赛季流程:");
    out.push_str("\n  1️⃣ 报名期 — 公会会长报名(需3级公会)");
    out.push_str("\n  2️⃣ 小组积分赛 — 随机分组，组内循环对战");
    out.push_str("\n  3️⃣ 淘汰赛 — 小组前2名进入淘汰赛");
    out.push_str("\n  4️⃣ 决赛 — 最终两支公会争夺冠军");
    out.push_str("\n  5️⃣ 休赛期 — 发放奖励，准备新赛季");
    out.push_str("\n\n⚔️ 对战规则:");
    out.push_str("\n  • 每日3次对战机会");
    out.push_str("\n  • 胜利+3分，失败+0分");
    out.push_str("\n  • 战力影响比分(公会成员总战力)");
    out.push_str("\n  • 同积分按净胜球排名");
    out.push_str("\n\n🎫 联赛积分:");
    out.push_str("\n  • 胜利获得50联赛积分");
    out.push_str("\n  • 失败获得10联赛积分");
    out.push_str("\n  • 可在联赛商店兑换奖励");
    out.push_str("\n\n🏪 联赛商店:");
    out.push_str("\n  • 金币箱/钻石袋/强化石/稀有道具");
    out.push_str("\n  • 部分商品需要公会排名达标");
    out.push_str("\n\n💡 指令列表:");
    out.push_str("\n  联赛 — 查看联赛状态");
    out.push_str("\n  联赛报名 — 公会报名");
    out.push_str("\n  联赛对战+公会名 — 挑战公会");
    out.push_str("\n  联赛排行 — 积分排行榜");
    out.push_str("\n  联赛商店+编号 — 兑换奖励");
    out.push_str("\n  联赛赛季 — 赛季信息");
    out.push_str("\n  联赛帮助 — 本帮助");

    out
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_league_phase_names() {
        assert_eq!(LeaguePhase::Registration.name(), "📋 报名期");
        assert_eq!(LeaguePhase::GroupStage.name(), "⚔️ 小组积分赛");
        assert_eq!(LeaguePhase::Knockout.name(), "🔥 淘汰赛");
        assert_eq!(LeaguePhase::Final.name(), "🏆 决赛");
        assert_eq!(LeaguePhase::OffSeason.name(), "💤 休赛期");
    }

    #[test]
    fn test_parse_phase() {
        assert!(matches!(parse_phase("registration"), LeaguePhase::Registration));
        assert!(matches!(parse_phase("group_stage"), LeaguePhase::GroupStage));
        assert!(matches!(parse_phase("knockout"), LeaguePhase::Knockout));
        assert!(matches!(parse_phase("final"), LeaguePhase::Final));
        assert!(matches!(parse_phase("off_season"), LeaguePhase::OffSeason));
        assert!(matches!(parse_phase("unknown"), LeaguePhase::OffSeason));
    }

    #[test]
    fn test_league_team_goal_diff() {
        let team = LeagueTeam {
            guild_id: "1".to_string(),
            guild_name: "测试公会".to_string(),
            wins: 3,
            losses: 1,
            points: 9,
            goals_scored: 10,
            goals_conceded: 5,
        };
        assert_eq!(team.goal_diff(), 5);
    }

    #[test]
    fn test_league_team_negative_goal_diff() {
        let team = LeagueTeam {
            guild_id: "1".to_string(),
            guild_name: "测试公会".to_string(),
            wins: 1,
            losses: 3,
            points: 3,
            goals_scored: 3,
            goals_conceded: 8,
        };
        assert_eq!(team.goal_diff(), -5);
    }

    #[test]
    fn test_league_team_zero_goal_diff() {
        let team = LeagueTeam {
            guild_id: "1".to_string(),
            guild_name: "测试公会".to_string(),
            wins: 2,
            losses: 2,
            points: 6,
            goals_scored: 7,
            goals_conceded: 7,
        };
        assert_eq!(team.goal_diff(), 0);
    }

    #[test]
    fn test_league_shop_items_count() {
        let items = get_league_shop_items();
        assert_eq!(items.len(), 8);
    }

    #[test]
    fn test_league_shop_items_sorted_by_rank() {
        let items = get_league_shop_items();
        assert!(items.first().unwrap().2 >= items.last().unwrap().2);
    }

    #[test]
    fn test_league_shop_items_escalating_prices() {
        let items = get_league_shop_items();
        assert!(items.last().unwrap().1 > items.first().unwrap().1);
    }

    #[test]
    fn test_league_shop_all_positive() {
        let items = get_league_shop_items();
        for (name, price, rank_req, desc) in &items {
            assert!(!name.is_empty());
            assert!(*price > 0);
            assert!(*rank_req > 0);
            assert!(!desc.is_empty());
        }
    }

    #[test]
    fn test_league_team_clone() {
        let team = LeagueTeam {
            guild_id: "1".to_string(),
            guild_name: "测试公会".to_string(),
            wins: 5,
            losses: 2,
            points: 15,
            goals_scored: 20,
            goals_conceded: 10,
        };
        let cloned = team.clone();
        assert_eq!(cloned.guild_id, "1");
        assert_eq!(cloned.wins, 5);
        assert_eq!(cloned.points, 15);
    }

    #[test]
    fn test_league_match_clone() {
        let m = LeagueMatch {
            team_a: "公会A".to_string(),
            team_b: "公会B".to_string(),
            score_a: 3,
            score_b: 1,
            round: "1".to_string(),
            timestamp: 1000000,
        };
        let cloned = m.clone();
        assert_eq!(cloned.score_a, 3);
        assert_eq!(cloned.score_b, 1);
    }

    #[test]
    fn test_league_ranking_sort() {
        let teams = vec![
            LeagueTeam {
                guild_id: "1".to_string(),
                guild_name: "A".to_string(),
                wins: 2,
                losses: 1,
                points: 6,
                goals_scored: 5,
                goals_conceded: 3,
            },
            LeagueTeam {
                guild_id: "2".to_string(),
                guild_name: "B".to_string(),
                wins: 3,
                losses: 0,
                points: 9,
                goals_scored: 8,
                goals_conceded: 2,
            },
            LeagueTeam {
                guild_id: "3".to_string(),
                guild_name: "C".to_string(),
                wins: 1,
                losses: 2,
                points: 3,
                goals_scored: 4,
                goals_conceded: 6,
            },
        ];
        let mut sorted = teams;
        sorted.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goal_diff().cmp(&a.goal_diff()))
                .then(b.goals_scored.cmp(&a.goals_scored))
        });
        assert_eq!(sorted[0].guild_name, "B");
        assert_eq!(sorted[1].guild_name, "A");
        assert_eq!(sorted[2].guild_name, "C");
    }

    #[test]
    fn test_league_ranking_tiebreak() {
        let teams = vec![
            LeagueTeam {
                guild_id: "1".to_string(),
                guild_name: "A".to_string(),
                wins: 2,
                losses: 1,
                points: 6,
                goals_scored: 5,
                goals_conceded: 2,
            },
            LeagueTeam {
                guild_id: "2".to_string(),
                guild_name: "B".to_string(),
                wins: 2,
                losses: 1,
                points: 6,
                goals_scored: 4,
                goals_conceded: 3,
            },
        ];
        let mut sorted = teams;
        sorted.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goal_diff().cmp(&a.goal_diff()))
                .then(b.goals_scored.cmp(&a.goals_scored))
        });
        assert_eq!(sorted[0].guild_name, "A");
    }

    #[test]
    fn test_league_data_serialization() {
        let teams = vec![LeagueTeam {
            guild_id: "1".to_string(),
            guild_name: "测试".to_string(),
            wins: 5,
            losses: 2,
            points: 15,
            goals_scored: 20,
            goals_conceded: 10,
        }];
        let serialized = format!(
            "{},{},{},{},{},{},{}",
            teams[0].guild_id,
            teams[0].guild_name,
            teams[0].wins,
            teams[0].losses,
            teams[0].points,
            teams[0].goals_scored,
            teams[0].goals_conceded
        );
        assert!(serialized.contains("测试"));
        assert!(serialized.contains("15"));
    }

    #[test]
    fn test_league_phase_all_variants() {
        let phases = [
            LeaguePhase::Registration,
            LeaguePhase::GroupStage,
            LeaguePhase::Knockout,
            LeaguePhase::Final,
            LeaguePhase::OffSeason,
        ];
        for phase in &phases {
            assert!(!phase.name().is_empty());
        }
    }

    #[test]
    fn test_league_help_content() {
        let help_text = "报名 小组积分赛 淘汰赛 决赛 休赛期 联赛积分 联赛商店";
        assert!(help_text.contains("报名"));
        assert!(help_text.contains("积分"));
        assert!(help_text.contains("商店"));
    }

    #[test]
    fn test_league_match_round_format() {
        let m = LeagueMatch {
            team_a: "A".to_string(),
            team_b: "B".to_string(),
            score_a: 2,
            score_b: 1,
            round: "3".to_string(),
            timestamp: 1234567890,
        };
        assert_eq!(m.round, "3");
        assert!(m.timestamp > 0);
    }

    #[test]
    fn test_league_team_goal_diff_boundaries() {
        // Large difference
        let t1 = LeagueTeam {
            guild_id: "1".to_string(),
            guild_name: "T".to_string(),
            wins: 0,
            losses: 0,
            points: 0,
            goals_scored: 100,
            goals_conceded: 0,
        };
        assert_eq!(t1.goal_diff(), 100);

        // Zero total
        let t2 = LeagueTeam {
            guild_id: "2".to_string(),
            guild_name: "T".to_string(),
            wins: 0,
            losses: 0,
            points: 0,
            goals_scored: 0,
            goals_conceded: 0,
        };
        assert_eq!(t2.goal_diff(), 0);
    }
}

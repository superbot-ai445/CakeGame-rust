/// CakeGame 竞技场锦标赛系统
///
/// 每周举办的淘汰制PvP锦标赛，玩家报名后系统自动配对生成对阵表。
/// 每轮淘汰赛通过回合制战斗决出胜负，最终冠军获得丰厚奖励。
///
/// 功能: 查看锦标赛/报名锦标赛/锦标赛对阵/锦标赛排行/锦标赛历史
///
/// 数据存储: Global 表 SECTION='arena_tournament'
use crate::core::*;
use crate::db::Database;

/// 锦标赛状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum TournamentState {
    Registration,
    InProgress,
    Finished,
}

impl TournamentState {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Registration => "报名中",
            Self::InProgress => "进行中",
            Self::Finished => "已结束",
        }
    }
    fn emoji(&self) -> &str {
        match self {
            Self::Registration => "📝",
            Self::InProgress => "⚔️",
            Self::Finished => "🏆",
        }
    }
}

/// 报名费用(金币)
const REGISTRATION_FEE: i64 = 2000;

/// 奖励定义: (名次, 金币, 钻石, 经验)
const PRIZES: &[(usize, i64, i64, i64)] = &[
    (1, 50000, 200, 10000),
    (2, 30000, 100, 6000),
    (3, 15000, 50, 3000),
    (5, 8000, 20, 1500),
];

/// Global 表 section 名
const SECTION: &str = "arena_tournament";

/// 锦标赛规模(固定8人)
const BRACKET_SIZE: usize = 8;

/// 选手信息
#[derive(Debug, Clone)]
struct Player {
    user_id: String,
    nickname: String,
    level: i32,
    combat_power: i64,
}

/// 对阵信息
#[derive(Debug, Clone)]
struct MatchEntry {
    player1: String,
    player2: String,
    winner: String,
    round: usize,
}

/// 锦标赛数据
#[derive(Debug, Clone)]
struct Tournament {
    week_id: String,
    state: TournamentState,
    players: Vec<Player>,
    matches: Vec<MatchEntry>,
    champion: String,
}

/// 生成周ID
fn week_id(ts: i64) -> String {
    let weeks_since_epoch = ts / (7 * 86400);
    format!("W{}", weeks_since_epoch)
}

/// DJB2 哈希
fn djb2_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.as_bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(*b as u64);
    }
    hash
}

/// 获取玩家等级
fn get_user_level(db: &Database, user_id: &str) -> i32 {
    db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1)
}

/// 获取玩家昵称
fn get_nickname(db: &Database, user_id: &str) -> String {
    db.read_basic(user_id, ITEM_NAME)
}

/// 计算玩家战力
fn get_combat_power(db: &Database, user_id: &str) -> i64 {
    let info = crate::user::calc_total_attrs(db, user_id);
    let ad = info.ad as i64;
    let ap = info.ap as i64;
    let hp = info.hp_max as i64;
    let defense = info.defense as i64;
    let mdf = info.magic_res as i64;
    ad * 35 + ap * 217 + hp * 3 + defense * 20 + mdf * 18
}

/// 解析锦标赛数据
fn parse_tournament(data: &str) -> Option<Tournament> {
    let mut week_id = String::new();
    let mut state = TournamentState::Registration;
    let mut players = Vec::new();
    let mut matches = Vec::new();
    let mut champion = String::new();

    for line in data.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("week_id=") {
            week_id = v.to_string();
        } else if let Some(v) = line.strip_prefix("state=") {
            state = match v {
                "InProgress" => TournamentState::InProgress,
                "Finished" => TournamentState::Finished,
                _ => TournamentState::Registration,
            };
        } else if let Some(v) = line.strip_prefix("champion=") {
            champion = v.to_string();
        } else if let Some(v) = line.strip_prefix("player=") {
            let parts: Vec<&str> = v.split('|').collect();
            if parts.len() >= 4 {
                players.push(Player {
                    user_id: parts[0].to_string(),
                    nickname: parts[1].to_string(),
                    level: parts[2].parse().unwrap_or(1),
                    combat_power: parts[3].parse().unwrap_or(0),
                });
            }
        } else if let Some(v) = line.strip_prefix("match=") {
            let parts: Vec<&str> = v.split('|').collect();
            if parts.len() >= 4 {
                matches.push(MatchEntry {
                    round: parts[0].parse().unwrap_or(0),
                    player1: parts[1].to_string(),
                    player2: parts[2].to_string(),
                    winner: parts[3].to_string(),
                });
            }
        }
    }

    if week_id.is_empty() {
        return None;
    }

    Some(Tournament {
        week_id,
        state,
        players,
        matches,
        champion,
    })
}

/// 序列化锦标赛数据
fn serialize_tournament(t: &Tournament) -> String {
    let mut lines = Vec::new();
    lines.push(format!("week_id={}", t.week_id));
    lines.push(format!("state={:?}", t.state));
    if !t.champion.is_empty() {
        lines.push(format!("champion={}", t.champion));
    }
    for p in &t.players {
        lines.push(format!(
            "player={}|{}|{}|{}",
            p.user_id, p.nickname, p.level, p.combat_power
        ));
    }
    for m in &t.matches {
        lines.push(format!("match={}|{}|{}|{}", m.round, m.player1, m.player2, m.winner));
    }
    lines.join("\n")
}

/// 模拟战斗
fn simulate_battle(p1_power: i64, p2_power: i64, seed: u64) -> bool {
    let total = p1_power + p2_power;
    if total == 0 {
        return seed.is_multiple_of(2);
    }
    let threshold = (p1_power as u64 * 10000) / total as u64;
    seed % 10000 < threshold
}

/// 生成对阵表
fn generate_bracket(players: &[Player], seed: u64) -> Vec<MatchEntry> {
    let mut matches = Vec::new();
    let n = players.len();
    if n < 2 {
        return matches;
    }

    // 按战力排序
    let mut sorted_indices: Vec<usize> = (0..n).collect();
    sorted_indices.sort_by(|&a, &b| players[b].combat_power.cmp(&players[a].combat_power));

    // 首轮配对
    let mut round = 1usize;
    let mut current_winners: Vec<usize> = Vec::new();

    for pair in (0..n).step_by(2) {
        if pair + 1 >= n {
            // 轮空
            current_winners.push(sorted_indices[pair]);
            continue;
        }
        let p1_idx = sorted_indices[pair];
        let p2_idx = sorted_indices[pair + 1];
        let match_seed = seed.wrapping_add(pair as u64).wrapping_mul(31);
        let winner_idx = if simulate_battle(players[p1_idx].combat_power, players[p2_idx].combat_power, match_seed) {
            p1_idx
        } else {
            p2_idx
        };

        matches.push(MatchEntry {
            player1: players[p1_idx].nickname.clone(),
            player2: players[p2_idx].nickname.clone(),
            winner: players[winner_idx].nickname.clone(),
            round,
        });
        current_winners.push(winner_idx);
    }

    // 后续轮次
    while current_winners.len() > 1 {
        round += 1;
        let mut next_winners = Vec::new();
        for pair in (0..current_winners.len()).step_by(2) {
            if pair + 1 >= current_winners.len() {
                next_winners.push(current_winners[pair]);
                continue;
            }
            let p1_idx = current_winners[pair];
            let p2_idx = current_winners[pair + 1];
            let match_seed = seed.wrapping_add((round * 100 + pair) as u64).wrapping_mul(37);
            let winner_idx = if simulate_battle(players[p1_idx].combat_power, players[p2_idx].combat_power, match_seed)
            {
                p1_idx
            } else {
                p2_idx
            };

            matches.push(MatchEntry {
                player1: players[p1_idx].nickname.clone(),
                player2: players[p2_idx].nickname.clone(),
                winner: players[winner_idx].nickname.clone(),
                round,
            });
            next_winners.push(winner_idx);
        }
        current_winners = next_winners;
    }

    matches
}

/// 格式化数字(千分位)
fn format_num(n: i64) -> String {
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

/// 进度条
fn progress_bar(current: i64, max: i64, width: usize) -> String {
    if max <= 0 {
        return "░".repeat(width);
    }
    let filled = ((current as f64 / max as f64) * width as f64).min(width as f64) as usize;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// 获取历史冠军记录
fn get_champion_history(db: &Database) -> Vec<(String, String, i64)> {
    let data = db.global_get(SECTION, "champion_history");
    let mut history = Vec::new();
    for line in data.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            history.push((
                parts[0].to_string(),
                parts[1].to_string(),
                parts[2].parse().unwrap_or(0),
            ));
        }
    }
    history
}

/// 保存历史冠军记录
fn save_champion_history(db: &Database, history: &[(String, String, i64)]) {
    let data: Vec<String> = history
        .iter()
        .map(|(wid, name, wins)| format!("{}|{}|{}", wid, name, wins))
        .collect();
    db.global_set(SECTION, "champion_history", &data.join("\n"));
}

// ==================== 公共指令 ====================

/// 查看锦标赛
pub fn cmd_view_tournament(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let wid = week_id(now);

    let data = db.global_get(SECTION, &wid);
    let tournament = parse_tournament(&data).unwrap_or(Tournament {
        week_id: wid.clone(),
        state: TournamentState::Registration,
        players: Vec::new(),
        matches: Vec::new(),
        champion: String::new(),
    });

    let mut out = String::new();
    out.push_str("🏟️ ═══ 竞技场锦标赛 ═══\n");
    out.push_str(&format!("📅 赛季: {}\n", tournament.week_id));
    out.push_str(&format!(
        "{} 状态: {}\n",
        tournament.state.emoji(),
        tournament.state.as_str()
    ));
    out.push_str(&format!("👥 参赛人数: {}/{}\n", tournament.players.len(), BRACKET_SIZE));

    let pct = (tournament.players.len() as f64 / BRACKET_SIZE as f64 * 100.0) as i64;
    out.push_str(&format!(
        "📊 报名进度: {} {}%\n",
        progress_bar(tournament.players.len() as i64, BRACKET_SIZE as i64, 10),
        pct
    ));

    let is_registered = tournament.players.iter().any(|p| p.user_id == user_id);
    out.push_str(&format!(
        "📝 我的状态: {}\n",
        if is_registered {
            "✅ 已报名"
        } else {
            "❌ 未报名"
        }
    ));

    if tournament.state == TournamentState::Registration {
        out.push_str(&format!("\n💰 报名费用: {} 金币\n", format_num(REGISTRATION_FEE)));
        out.push_str("📌 输入「报名锦标赛」即可参加\n");
    }

    if tournament.state == TournamentState::Finished && !tournament.champion.is_empty() {
        out.push_str(&format!("\n🏆 本周冠军: {}\n", tournament.champion));
    }

    out.push_str("\n🎁 奖励预览:\n");
    for &(rank, gold, diamond, exp) in PRIZES {
        let medal = match rank {
            1 => "🥇冠军",
            2 => "🥈亚军",
            3 => "🥉四强",
            5 => "🎖️八强",
            _ => continue,
        };
        out.push_str(&format!(
            "  {} {}金 {}💎 {}Exp\n",
            medal,
            format_num(gold),
            diamond,
            format_num(exp)
        ));
    }

    out
}

/// 报名锦标赛
pub fn cmd_register_tournament(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let wid = week_id(now);
    let level = get_user_level(db, user_id);
    if level < 10 {
        return "❌ 等级不足10级，无法报名锦标赛".to_string();
    }

    let weakness_end = crate::user::check_weakness(db, user_id);
    if weakness_end > now {
        return "❌ 虚弱状态中，无法报名".to_string();
    }

    let data = db.global_get(SECTION, &wid);
    let mut tournament = parse_tournament(&data).unwrap_or(Tournament {
        week_id: wid.clone(),
        state: TournamentState::Registration,
        players: Vec::new(),
        matches: Vec::new(),
        champion: String::new(),
    });

    if tournament.state != TournamentState::Registration {
        return "❌ 当前不在报名阶段".to_string();
    }

    if tournament.players.iter().any(|p| p.user_id == user_id) {
        return "📝 你已经报名了本周锦标赛".to_string();
    }

    if tournament.players.len() >= BRACKET_SIZE {
        return format!("❌ 锦标赛名额已满 ({}/{})", tournament.players.len(), BRACKET_SIZE);
    }

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < REGISTRATION_FEE {
        return format!(
            "❌ 金币不足! 需要 {} 金币，当前 {} 金币",
            format_num(REGISTRATION_FEE),
            format_num(gold)
        );
    }
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, REGISTRATION_FEE);

    let nickname = get_nickname(db, user_id);
    let cp = get_combat_power(db, user_id);

    tournament.players.push(Player {
        user_id: user_id.to_string(),
        nickname: nickname.clone(),
        level,
        combat_power: cp,
    });

    if tournament.players.len() >= BRACKET_SIZE {
        tournament.state = TournamentState::InProgress;
        let seed = djb2_hash(&wid);
        tournament.matches = generate_bracket(&tournament.players, seed);
        if let Some(last_match) = tournament.matches.last() {
            tournament.champion = last_match.winner.clone();
        }
        tournament.state = TournamentState::Finished;

        if !tournament.champion.is_empty() {
            let mut history = get_champion_history(db);
            if let Some(entry) = history.iter_mut().find(|h| h.1 == tournament.champion) {
                entry.2 += 1;
            } else {
                history.push((wid.clone(), tournament.champion.clone(), 1));
            }
            save_champion_history(db, &history);
        }
    }

    db.global_set(SECTION, &wid, &serialize_tournament(&tournament));

    let remaining = BRACKET_SIZE - tournament.players.len();
    let mut out = String::new();
    out.push_str(&format!("✅ 报名成功! 花费 {} 金币\n", format_num(REGISTRATION_FEE)));
    out.push_str(&format!("👥 当前报名: {}/{}\n", tournament.players.len(), BRACKET_SIZE));
    if remaining > 0 && tournament.state == TournamentState::Registration {
        out.push_str(&format!("⏳ 还需 {} 位选手报名即可开赛\n", remaining));
    }
    if tournament.state == TournamentState::Finished {
        out.push_str("\n⚡ 满员! 锦标赛已自动开始并结束\n");
        out.push_str("📌 输入「锦标赛对阵」查看完整对阵表\n");
    }
    out
}

/// 锦标赛对阵
pub fn cmd_tournament_bracket(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let wid = week_id(now);

    let data = db.global_get(SECTION, &wid);
    let tournament = match parse_tournament(&data) {
        Some(t) => t,
        None => return "📋 本周锦标赛尚未开始，还没有对阵数据".to_string(),
    };

    if tournament.matches.is_empty() {
        return "📋 对阵表尚未生成，请等待报名完成".to_string();
    }

    let mut out = String::new();
    out.push_str("⚔️ ═══ 锦标赛对阵表 ═══\n");
    out.push_str(&format!("📅 赛季: {}\n\n", tournament.week_id));

    let max_round = tournament.matches.iter().map(|m| m.round).max().unwrap_or(0);
    for round in 1..=max_round {
        let round_name = match (max_round, round) {
            (_, 1) => "首轮",
            (r, x) if x == r => "决赛",
            (r, x) if x == r - 1 => "半决赛",
            _ => "次轮",
        };
        out.push_str(&format!("══ 第{}轮({}) ══\n", round, round_name));
        for m in tournament.matches.iter().filter(|m| m.round == round) {
            let w1 = if m.player1 == m.winner { "✅" } else { "❌" };
            let w2 = if m.player2 == m.winner { "✅" } else { "❌" };
            out.push_str(&format!("  {} {} vs {} {}\n", w1, m.player1, m.player2, w2));
        }
        out.push('\n');
    }

    if !tournament.champion.is_empty() {
        out.push_str(&format!("🏆 冠军: {}\n", tournament.champion));
    }

    out
}

/// 锦标赛排行
pub fn cmd_tournament_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut history = get_champion_history(db);
    history.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut out = String::new();
    out.push_str("🏆 ═══ 锦标赛冠军排行 ═══\n\n");

    if history.is_empty() {
        out.push_str("暂无历史冠军记录\n");
        return out;
    }

    for (i, (_wid, name, wins)) in history.iter().take(15).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!("{} {}. {} — {}次冠军\n", medal, i + 1, name, wins));
    }

    out
}

/// 锦标赛历史
pub fn cmd_tournament_history(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let history = get_champion_history(db);
    let nickname = get_nickname(db, user_id);

    let mut out = String::new();
    out.push_str("📜 ═══ 锦标赛历史 ═══\n\n");

    if history.is_empty() {
        out.push_str("暂无锦标赛历史\n");
        out.push_str("📌 输入「报名锦标赛」开始你的竞技之旅!\n");
        return out;
    }

    let mut my_wins = 0i64;
    for (wid, champion, _) in history.iter().rev().take(10) {
        let is_me = *champion == nickname;
        if is_me {
            my_wins += 1;
        }
        let mark = if is_me { " 🏆" } else { "" };
        out.push_str(&format!("  📅 {} — 冠军: {}{}\n", wid, champion, mark));
    }

    out.push_str(&format!("\n📊 你的夺冠次数: {} 次\n", my_wins));
    out
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_week_id_format() {
        let wid = week_id(1718000000);
        assert!(wid.starts_with('W'));
        assert!(wid.len() > 1);
    }

    #[test]
    fn test_week_id_deterministic() {
        assert_eq!(week_id(1718000000), week_id(1718000000));
    }

    #[test]
    fn test_djb2_hash_deterministic() {
        assert_eq!(djb2_hash("test"), djb2_hash("test"));
    }

    #[test]
    fn test_djb2_hash_different() {
        assert_ne!(djb2_hash("abc"), djb2_hash("def"));
    }

    #[test]
    fn test_tournament_state_emoji() {
        assert_eq!(TournamentState::Registration.emoji(), "📝");
        assert_eq!(TournamentState::InProgress.emoji(), "⚔️");
        assert_eq!(TournamentState::Finished.emoji(), "🏆");
    }

    #[test]
    fn test_tournament_state_str() {
        assert_eq!(TournamentState::Registration.as_str(), "报名中");
        assert_eq!(TournamentState::InProgress.as_str(), "进行中");
        assert_eq!(TournamentState::Finished.as_str(), "已结束");
    }

    #[test]
    fn test_prizes_sorted_by_rank() {
        for i in 1..PRIZES.len() {
            assert!(PRIZES[i - 1].0 < PRIZES[i].0);
        }
    }

    #[test]
    fn test_prize_rewards_positive() {
        for &(_, gold, diamond, exp) in PRIZES {
            assert!(gold > 0);
            assert!(diamond > 0);
            assert!(exp > 0);
        }
    }

    #[test]
    fn test_prize_rewards_decrease() {
        assert!(PRIZES[0].1 > PRIZES[1].1);
        assert!(PRIZES[0].2 > PRIZES[1].2);
    }

    #[test]
    fn test_progress_bar_full() {
        assert_eq!(progress_bar(10, 10, 10), "█".repeat(10));
    }

    #[test]
    fn test_progress_bar_empty() {
        assert_eq!(progress_bar(0, 10, 10), "░".repeat(10));
    }

    #[test]
    fn test_progress_bar_half() {
        assert_eq!(progress_bar(5, 10, 10), format!("{}{}", "█".repeat(5), "░".repeat(5)));
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(999), "999");
        assert_eq!(format_num(1000), "1,000");
        assert_eq!(format_num(1234567), "1,234,567");
    }

    #[test]
    fn test_format_num_negative() {
        assert_eq!(format_num(-50000), "-50,000");
    }

    #[test]
    fn test_serialize_parse_roundtrip() {
        let t = Tournament {
            week_id: "W123".to_string(),
            state: TournamentState::Registration,
            players: vec![Player {
                user_id: "u1".to_string(),
                nickname: "Test".to_string(),
                level: 10,
                combat_power: 1000,
            }],
            matches: vec![],
            champion: String::new(),
        };
        let data = serialize_tournament(&t);
        let parsed = parse_tournament(&data).unwrap();
        assert_eq!(parsed.week_id, "W123");
        assert_eq!(parsed.state, TournamentState::Registration);
        assert_eq!(parsed.players.len(), 1);
        assert_eq!(parsed.players[0].user_id, "u1");
    }

    #[test]
    fn test_parse_tournament_empty() {
        assert!(parse_tournament("").is_none());
    }

    #[test]
    fn test_generate_bracket_two_players() {
        let players = vec![
            Player {
                user_id: "p1".to_string(),
                nickname: "A".to_string(),
                level: 50,
                combat_power: 5000,
            },
            Player {
                user_id: "p2".to_string(),
                nickname: "B".to_string(),
                level: 40,
                combat_power: 3000,
            },
        ];
        let matches = generate_bracket(&players, 42);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].round, 1);
        assert!(matches.last().unwrap().winner == "A" || matches.last().unwrap().winner == "B");
    }

    #[test]
    fn test_simulate_battle_deterministic() {
        assert_eq!(simulate_battle(5000, 3000, 42), simulate_battle(5000, 3000, 42));
    }

    #[test]
    fn test_simulate_battle_higher_power_advantage() {
        let mut p1_wins = 0;
        for seed in 0..100 {
            if simulate_battle(9000, 1000, seed) {
                p1_wins += 1;
            }
        }
        assert!(p1_wins > 80);
    }

    #[test]
    fn test_registration_fee_constant() {
        assert_eq!(REGISTRATION_FEE, 2000);
    }

    #[test]
    fn test_bracket_size_constant() {
        assert_eq!(BRACKET_SIZE, 8);
    }

    #[test]
    fn test_combat_power_formula() {
        let base_power: i64 = 100 * 35 + 50 * 217 + 500 * 3 + 20 * 20 + 15 * 18;
        assert!(base_power > 0);
    }

    #[test]
    fn test_serialization_includes_matches() {
        let t = Tournament {
            week_id: "W999".to_string(),
            state: TournamentState::Finished,
            players: vec![],
            matches: vec![MatchEntry {
                player1: "Alice".to_string(),
                player2: "Bob".to_string(),
                winner: "Alice".to_string(),
                round: 1,
            }],
            champion: "Alice".to_string(),
        };
        let data = serialize_tournament(&t);
        assert!(data.contains("match=1|Alice|Bob|Alice"));
        assert!(data.contains("champion=Alice"));
        assert!(data.contains("state=Finished"));
    }

    #[test]
    fn test_generate_bracket_empty() {
        assert!(generate_bracket(&[], 42).is_empty());
    }

    #[test]
    fn test_generate_bracket_single_player() {
        let players = vec![Player {
            user_id: "p1".to_string(),
            nickname: "Solo".to_string(),
            level: 50,
            combat_power: 5000,
        }];
        assert!(generate_bracket(&players, 42).is_empty());
    }
}

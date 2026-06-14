/// CakeGame 公会副本系统
///
/// 公会成员组队挑战副本，逐层击败怪物，获取丰厚奖励。
/// 副本按难度分级，需要公会等级和成员战力门槛。
///
/// 功能: 查看副本/进入副本/副本战斗/副本进度/副本奖励/副本排行/副本帮助/放弃副本
///
/// 数据存储: Global 表 SECTION='guild_dungeon' / 'guild_dungeon_daily' / 'guild_dungeon_stats'
use crate::db::Database;

/// 副本难度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DungeonDifficulty {
    Normal,    // 普通
    Hard,      // 困难
    Expert,    // 精英
    Master,    // 大师
    Nightmare, // 噩梦
    Hell,      // 地狱
}

impl DungeonDifficulty {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "普通",
            Self::Hard => "困难",
            Self::Expert => "精英",
            Self::Master => "大师",
            Self::Nightmare => "噩梦",
            Self::Hell => "地狱",
        }
    }
    fn emoji(&self) -> &str {
        match self {
            Self::Normal => "🟢",
            Self::Hard => "🔵",
            Self::Expert => "🟣",
            Self::Master => "🟠",
            Self::Nightmare => "🔴",
            Self::Hell => "⚫",
        }
    }
    fn guild_level_req(&self) -> i64 {
        match self {
            Self::Normal => 1,
            Self::Hard => 2,
            Self::Expert => 3,
            Self::Master => 4,
            Self::Nightmare => 5,
            Self::Hell => 6,
        }
    }
    fn combat_power_req(&self) -> i64 {
        match self {
            Self::Normal => 500,
            Self::Hard => 2000,
            Self::Expert => 5000,
            Self::Master => 12000,
            Self::Nightmare => 30000,
            Self::Hell => 60000,
        }
    }
    fn num_floors(&self) -> usize {
        match self {
            Self::Normal => 3,
            Self::Hard => 5,
            Self::Expert => 7,
            Self::Master => 10,
            Self::Nightmare => 12,
            Self::Hell => 15,
        }
    }
    /// 每层怪物基础HP
    fn base_floor_hp(&self) -> i64 {
        match self {
            Self::Normal => 3000,
            Self::Hard => 8000,
            Self::Expert => 20000,
            Self::Master => 50000,
            Self::Nightmare => 120000,
            Self::Hell => 300000,
        }
    }
    /// 每层怪物基础攻击
    fn base_floor_atk(&self) -> i64 {
        match self {
            Self::Normal => 200,
            Self::Hard => 500,
            Self::Expert => 1200,
            Self::Master => 3000,
            Self::Nightmare => 7000,
            Self::Hell => 15000,
        }
    }
    /// 通关基础金币奖励(每层)
    fn gold_per_floor(&self) -> i64 {
        match self {
            Self::Normal => 500,
            Self::Hard => 1500,
            Self::Expert => 4000,
            Self::Master => 10000,
            Self::Nightmare => 25000,
            Self::Hell => 60000,
        }
    }
    /// 通关基础经验奖励(每层)
    fn exp_per_floor(&self) -> i64 {
        match self {
            Self::Normal => 300,
            Self::Hard => 800,
            Self::Expert => 2000,
            Self::Master => 5000,
            Self::Nightmare => 12000,
            Self::Hell => 30000,
        }
    }
    /// 全通关额外钻石奖励
    fn diamond_full_clear(&self) -> i64 {
        match self {
            Self::Normal => 30,
            Self::Hard => 80,
            Self::Expert => 200,
            Self::Master => 500,
            Self::Nightmare => 1200,
            Self::Hell => 3000,
        }
    }
    fn all() -> &'static [DungeonDifficulty] {
        &[
            Self::Normal,
            Self::Hard,
            Self::Expert,
            Self::Master,
            Self::Nightmare,
            Self::Hell,
        ]
    }
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "普通" => Some(Self::Normal),
            "困难" => Some(Self::Hard),
            "精英" => Some(Self::Expert),
            "大师" => Some(Self::Master),
            "噩梦" => Some(Self::Nightmare),
            "地狱" => Some(Self::Hell),
            _ => None,
        }
    }
}

/// 副本每层怪物名称

/// 副本数据结构
struct DungeonState {
    difficulty: DungeonDifficulty,
    current_floor: usize,
    total_damage: i64,
    floors_cleared: usize,
    started_at: String,
}

impl DungeonState {
    fn new(diff: DungeonDifficulty) -> Self {
        Self {
            difficulty: diff,
            current_floor: 0,
            total_damage: 0,
            floors_cleared: 0,
            started_at: utils::chrono_now(),
        }
    }

    fn from_json(json: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(json).ok()?;
        let diff_num = v.get("difficulty")?.as_i64()? as usize;
        let diff = match diff_num {
            0 => DungeonDifficulty::Normal,
            1 => DungeonDifficulty::Hard,
            2 => DungeonDifficulty::Expert,
            3 => DungeonDifficulty::Master,
            4 => DungeonDifficulty::Nightmare,
            5 => DungeonDifficulty::Hell,
            _ => return None,
        };
        Some(Self {
            difficulty: diff,
            current_floor: v.get("floor")?.as_i64()? as usize,
            total_damage: v.get("damage")?.as_i64().unwrap_or(0),
            floors_cleared: v.get("cleared")?.as_i64()? as usize,
            started_at: v.get("started")?.as_str().unwrap_or("").to_string(),
        })
    }

    fn to_json(&self) -> String {
        format!(
            r#"{{"difficulty":{},"floor":{},"damage":{},"cleared":{},"started":"{}"}}"#,
            self.difficulty as u8, self.current_floor, self.total_damage, self.floors_cleared, self.started_at,
        )
    }

    fn is_complete(&self) -> bool {
        self.floors_cleared >= self.difficulty.num_floors()
    }
}

/// 简化的战力计算
fn estimate_combat_power(db: &Database, user_id: &str) -> i64 {
    let info = crate::user::calc_total_attrs(db, user_id);
    let hp = info.hp as f64;
    let ad = info.ad as f64;
    let ap = info.ap as f64;
    let def = info.defense as f64;
    let mres = info.magic_res as f64;
    (hp * 0.1 + ad * 2.0 + ap * 2.0 + def * 1.5 + mres * 1.5) as i64
}

/// 简单战斗模拟: 返回(伤害输出, 是否胜利)
fn simulate_floor_battle(power: i64, floor_hp: i64, floor_atk: i64) -> (i64, bool) {
    // 玩家每回合伤害: power * (0.8~1.2随机)
    let rand_factor = 80 + ((power * 7) % 41); // 80~120
    let player_damage = (power as f64 * rand_factor as f64 / 100.0) as i64;
    let player_damage = player_damage.max(1);

    // 计算需要多少回合击杀怪物
    let rounds_to_kill = (floor_hp + player_damage - 1) / player_damage;

    // 怪物每回合对玩家的伤害
    let monster_damage_per_round = (floor_atk as f64 * 0.5) as i64;

    // 玩家总HP(简化: power * 3)
    let player_hp = power * 3;

    // 怪物造成的总伤害
    let total_monster_damage = monster_damage_per_round * (rounds_to_kill - 1).max(0);

    let won = player_hp > total_monster_damage;
    let total_damage = player_damage * rounds_to_kill;

    (total_damage, won)
}

/// 获取玩家公会名
fn get_user_guild(db: &Database, user_id: &str) -> Option<String> {
    let guild = db.read_basic(user_id, "公会");
    if guild.is_empty() {
        None
    } else {
        Some(guild)
    }
}

/// 辅助: 获取当前时间

/// 奖励金币
fn reward_gold(db: &Database, user_id: &str, amount: i64) {
    let cur: i64 = db.read_basic(user_id, "金币").parse().unwrap_or(0);
    db.write_basic(user_id, "金币", &(cur + amount).to_string());
}

/// 奖励经验
fn reward_exp(db: &Database, user_id: &str, amount: i64) {
    let cur: i64 = db.read_basic(user_id, "经验").parse().unwrap_or(0);
    db.write_basic(user_id, "经验", &(cur + amount).to_string());
}

/// 奖励钻石
fn reward_diamond(db: &Database, user_id: &str, amount: i64) {
    let cur: i64 = db.read_basic(user_id, "钻石").parse().unwrap_or(0);
    db.write_basic(user_id, "钻石", &(cur + amount).to_string());
}

// ==================== 指令处理器 ====================

/// 查看公会副本
pub fn cmd_view_dungeon(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你未加入任何公会，无法查看公会副本。".to_string(),
    };

    let power = estimate_combat_power(db, user_id);
    let section = format!("guild_dungeon_{}", guild);
    let daily_section = format!("guild_dungeon_daily_{}_{}", guild, user_id);

    // 读取今日挑战次数
    let daily_str = db.global_get(&daily_section, "challenge_count");
    let challenges_today: i64 = daily_str.parse().unwrap_or(0);

    let mut out = String::from("🏰 公会副本一览\n");
    out.push_str(&format!("⚔️ 你的战力: {} | 今日挑战: {}/5\n", power, challenges_today));
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    for (i, diff) in DungeonDifficulty::all().iter().enumerate() {
        let prog_key = format!("progress_{}", i);
        let progress_str = db.global_get(&section, &prog_key);
        let best_floor: i64 = progress_str.parse().unwrap_or(0);
        let total_floors = diff.num_floors() as i64;

        let status = if best_floor >= total_floors {
            "✅ 已通关".to_string()
        } else if best_floor > 0 {
            format!("📊 进度 {}/{}", best_floor, total_floors)
        } else {
            "🔒 未挑战".to_string()
        };

        let can_enter = power >= diff.combat_power_req();
        let lock = if can_enter { "" } else { " 🔒战力不足" };

        out.push_str(&format!(
            "{} {}{} — Lv{}公会 | 战力{}\n",
            diff.emoji(),
            diff.as_str(),
            lock,
            diff.guild_level_req(),
            diff.combat_power_req(),
        ));
        out.push_str(&format!(
            "   {} | {}层 | 奖励: {}金/层 + {}💎全通\n",
            status,
            total_floors,
            diff.gold_per_floor(),
            diff.diamond_full_clear()
        ));
    }

    out.push_str("\n💡 输入「进入副本+难度」开始挑战（如: 进入副本+困难）");
    out
}

/// 进入副本
pub fn cmd_enter_dungeon(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你未加入任何公会，无法进入副本。".to_string(),
    };

    let diff = match DungeonDifficulty::from_str(args.trim()) {
        Some(d) => d,
        None => return "❌ 未知难度。可选: 普通/困难/精英/大师/噩梦/地狱".to_string(),
    };

    let power = estimate_combat_power(db, user_id);
    if power < diff.combat_power_req() {
        return format!(
            "❌ 战力不足！{}副本需要{}战力，你当前{}。",
            diff.as_str(),
            diff.combat_power_req(),
            power
        );
    }

    // 检查今日挑战次数
    let daily_section = format!("guild_dungeon_daily_{}_{}", guild, user_id);
    let daily_str = db.global_get(&daily_section, "challenge_count");
    let challenges_today: i64 = daily_str.parse().unwrap_or(0);
    if challenges_today >= 5 {
        return "❌ 今日挑战次数已用完（每日5次），明天再来吧！".to_string();
    }

    // 检查是否已在副本中
    let active_section = format!("guild_dungeon_active_{}_{}", guild, user_id);
    let existing = db.global_get(&active_section, "state");
    if !existing.is_empty() {
        return "❌ 你已在副本中，请先完成或放弃当前副本。输入「副本战斗」继续挑战。".to_string();
    }

    // 创建新的副本状态
    let state = DungeonState::new(diff);
    db.global_set(&active_section, "state", &state.to_json());
    db.global_set(&active_section, "difficulty", &(diff as u8).to_string());
    db.global_set(&daily_section, "challenge_count", &(challenges_today + 1).to_string());

    let monster = utils::floor_monster_name(diff, 0);
    let monster_hp = diff.base_floor_hp();
    let floors = diff.num_floors();

    format!(
        "🏰 进入{}副本「{}之境」\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         📍 当前: 第1/{}层\n\
         👹 怪物: {} (HP: {})\n\
         ⚔️ 你的战力: {}\n\
         \n\
         💡 输入「副本战斗」挑战当前层\n\
         💡 输入「副本进度」查看详情\n\
         💡 输入「放弃副本」退出",
        diff.emoji(),
        diff.as_str(),
        floors,
        monster,
        monster_hp,
        power,
    )
}

/// 副本战斗
pub fn cmd_dungeon_battle(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你未加入任何公会。".to_string(),
    };

    let active_section = format!("guild_dungeon_active_{}_{}", guild, user_id);
    let state_json = db.global_get(&active_section, "state");
    if state_json.is_empty() {
        return "❌ 你不在任何副本中。输入「进入副本+难度」开始。".to_string();
    }

    let mut state = match DungeonState::from_json(&state_json) {
        Some(s) => s,
        None => return "❌ 副本数据异常，请放弃后重新进入。".to_string(),
    };

    if state.is_complete() {
        return "✅ 该副本已通关！请领取奖励或进入新副本。".to_string();
    }

    let power = estimate_combat_power(db, user_id);
    let floor = state.current_floor;
    let diff = state.difficulty;

    // 每层怪物属性递增 (floor+1) 倍率
    let multiplier = 1.0 + (floor as f64 * 0.25);
    let monster_hp = (diff.base_floor_hp() as f64 * multiplier) as i64;
    let monster_atk = (diff.base_floor_atk() as f64 * multiplier) as i64;
    let monster_name = utils::floor_monster_name(diff, floor);

    let (damage_dealt, won) = simulate_floor_battle(power, monster_hp, monster_atk);
    state.total_damage += damage_dealt;

    let mut out = String::new();

    if won {
        state.floors_cleared += 1;
        state.current_floor += 1;

        let floor_gold = diff.gold_per_floor();
        let floor_exp = diff.exp_per_floor();

        // 发放层奖励
        reward_gold(db, user_id, floor_gold);
        reward_exp(db, user_id, floor_exp);

        out.push_str(&format!(
            "⚔️ 副本战斗 — 第{}层\n\
             ━━━━━━━━━━━━━━━━━━━━\n\
             👹 {}: HP {}\n\
             💥 造成伤害: {}\n\
             ✅ 胜利！击败了{}！\n\
             \n\
             🎁 奖励: +{}金币 +{}经验\n",
            floor + 1,
            monster_name,
            monster_hp,
            damage_dealt,
            monster_name,
            floor_gold,
            floor_exp,
        ));

        if state.is_complete() {
            // 全通关额外奖励
            let full_diamonds = diff.diamond_full_clear();
            reward_diamond(db, user_id, full_diamonds);

            out.push_str(&format!(
                "\n🎉🎉🎉 恭喜通关{}副本！🎉🎉🎉\n\
                 🏆 全通关额外奖励: +{}💎钻石\n\
                 📊 总伤害: {} | 总层: {}\n",
                diff.as_str(),
                full_diamonds,
                state.total_damage,
                state.floors_cleared,
            ));

            // 记录统计数据
            let stats_section = format!("guild_dungeon_stats_{}", guild);
            let clear_key = format!("clear_{}_{}", user_id, diff as u8);
            db.global_set(&stats_section, &clear_key, &state.total_damage.to_string());

            // 更新全局通关计数
            let total_key = format!("total_clears_{}", diff as u8);
            let current_str = db.global_get(&stats_section, &total_key);
            let current_clears: i64 = current_str.parse().unwrap_or(0);
            db.global_set(&stats_section, &total_key, &(current_clears + 1).to_string());

            // 更新最佳进度
            let prog_section = format!("guild_dungeon_{}", guild);
            let prog_key = format!("progress_{}", diff as u8);
            db.global_set(&prog_section, &prog_key, &state.floors_cleared.to_string());
        } else {
            let next_monster = utils::floor_monster_name(diff, state.current_floor);
            let next_hp = (diff.base_floor_hp() as f64 * (1.0 + state.current_floor as f64 * 0.25)) as i64;
            out.push_str(&format!(
                "\n📍 下一层: 第{}层 — {} (HP: {})\n\
                 💡 输入「副本战斗」继续挑战",
                state.current_floor + 1,
                next_monster,
                next_hp,
            ));
        }
    } else {
        // 失败
        out.push_str(&format!(
            "⚔️ 副本战斗 — 第{}层\n\
             ━━━━━━━━━━━━━━━━━━━━\n\
             👹 {}: HP {}\n\
             💥 造成伤害: {}\n\
             ❌ 战斗失败！被{}击败...\n\
             \n\
             💡 提升战力后重试，或输入「放弃副本」退出",
            floor + 1,
            monster_name,
            monster_hp,
            damage_dealt,
            monster_name,
        ));
    }

    // 保存状态
    db.global_set(&active_section, "state", &state.to_json());
    out
}

/// 副本进度
pub fn cmd_dungeon_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你未加入任何公会。".to_string(),
    };

    let active_section = format!("guild_dungeon_active_{}_{}", guild, user_id);
    let state_json = db.global_get(&active_section, "state");
    if state_json.is_empty() {
        return "❌ 你不在任何副本中。".to_string();
    }

    let state = match DungeonState::from_json(&state_json) {
        Some(s) => s,
        None => return "❌ 副本数据异常。".to_string(),
    };

    let total_floors = state.difficulty.num_floors();
    let progress_pct = (state.floors_cleared as f64 / total_floors as f64 * 100.0) as i64;
    let bar_len = 20;
    let filled = (state.floors_cleared * bar_len) / total_floors;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_len - filled));

    let mut out = format!(
        "🏰 副本进度 — {}{}副本\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         📊 进度: [{}] {}%\n\
         📍 当前层: 第{}/{}层\n\
         💥 累计伤害: {}\n\
         🕐 开始时间: {}\n",
        state.difficulty.emoji(),
        state.difficulty.as_str(),
        bar,
        progress_pct,
        state.current_floor + 1,
        total_floors,
        state.total_damage,
        state.started_at,
    );

    if state.is_complete() {
        out.push_str("\n✅ 副本已通关！输入「副本奖励」领取奖励。");
    } else {
        let monster = utils::floor_monster_name(state.difficulty, state.current_floor);
        out.push_str(&format!("\n👹 当前怪物: {}\n💡 输入「副本战斗」继续挑战", monster));
    }

    out
}

/// 副本奖励
pub fn cmd_dungeon_reward(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你未加入任何公会。".to_string(),
    };

    let stats_section = format!("guild_dungeon_stats_{}", guild);
    let mut out = String::from("🎁 公会副本奖励记录\n━━━━━━━━━━━━━━━━━━━━\n");

    for (i, diff) in DungeonDifficulty::all().iter().enumerate() {
        let clear_key = format!("clear_{}_{}", user_id, i);
        let cleared = !db.global_get(&stats_section, &clear_key).is_empty();
        let status = if cleared { "✅ 已通关" } else { "❌ 未通关" };
        out.push_str(&format!(
            "{} {} — {} | 奖励: {}💎\n",
            diff.emoji(),
            diff.as_str(),
            status,
            diff.diamond_full_clear()
        ));
    }

    // 统计全公会通关情况
    let mut total_clears = 0i64;
    for i in 0..6 {
        let total_key = format!("total_clears_{}", i);
        let count: i64 = db.global_get(&stats_section, &total_key).parse().unwrap_or(0);
        total_clears += count;
    }
    out.push_str(&format!("\n📊 公会总通关次数: {}", total_clears));
    out.push_str("\n💡 输入「进入副本+难度」继续挑战！");
    out
}

/// 副本排行
pub fn cmd_dungeon_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // 从所有用户中收集公会列表，再查询各公会的通关统计
    let mut guild_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for uid in db.all_users() {
        let g = db.read_basic(&uid, "公会");
        if !g.is_empty() {
            guild_set.insert(g);
        }
    }

    let mut rankings: Vec<(String, i64)> = Vec::new();
    for guild_name in &guild_set {
        let stats_section = format!("guild_dungeon_stats_{}", guild_name);
        let mut total_clears: i64 = 0;
        for i in 0..6 {
            let total_key = format!("total_clears_{}", i);
            let count: i64 = db.global_get(&stats_section, &total_key).parse().unwrap_or(0);
            total_clears += count;
        }
        if total_clears > 0 {
            rankings.push((guild_name.clone(), total_clears));
        }
    }
    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = String::from("🏆 公会副本排行\n━━━━━━━━━━━━━━━━━━━━\n");
    let medals = ["🥇", "🥈", "🥉"];

    for (i, (guild, clears)) in rankings.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let guild_display = guild.replace("guild_dungeon_stats_", "公会");
        out.push_str(&format!("{} {} — {}次通关\n", medal, guild_display, clears));
    }

    if rankings.is_empty() {
        out.push_str("暂无公会通关记录\n");
    }
    out
}

/// 副本帮助
pub fn cmd_dungeon_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    "🏰 公会副本系统帮助\n\
     ━━━━━━━━━━━━━━━━━━━━\n\
     📖 公会副本是公会成员共同挑战的PvE内容。\n\
     \n\
     🎯 6个难度等级:\n\
     🟢 普通(3层) → 🔵 困难(5层) → 🟣 精英(7层)\n\
     🟠 大师(10层) → 🔴 噩梦(12层) → ⚫ 地狱(15层)\n\
     \n\
     📋 指令列表:\n\
     • 查看副本 — 查看所有副本及进度\n\
     • 进入副本+难度 — 开始挑战(如: 进入副本+困难)\n\
     • 副本战斗 — 挑战当前层怪物\n\
     • 副本进度 — 查看当前副本进度\n\
     • 副本奖励 — 查看奖励记录\n\
     • 副本排行 — 公会副本通关排行\n\
     • 放弃副本 — 放弃当前副本(进度清零)\n\
     \n\
     💡 规则说明:\n\
     • 每日最多挑战5次(进入副本计1次)\n\
     • 每层胜利获得金币+经验\n\
     • 全通关获得额外钻石奖励\n\
     • 战斗失败不扣次数，可重试\n\
     • 每层怪物属性递增(+25%/层)\n\
     \n\
     ⚠️ 需要加入公会才能参与！"
        .to_string()
}

/// 放弃副本
pub fn cmd_dungeon_abandon(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild = match get_user_guild(db, user_id) {
        Some(g) => g,
        None => return "❌ 你未加入任何公会。".to_string(),
    };

    let active_section = format!("guild_dungeon_active_{}_{}", guild, user_id);
    let existing = db.global_get(&active_section, "state");
    if existing.is_empty() {
        return "❌ 你不在任何副本中。".to_string();
    }

    db.global_set(&active_section, "state", "");
    "✅ 已放弃当前副本，进度已清零。\n💡 输入「进入副本+难度」重新开始挑战。".to_string()
}

// ==================== 测试 ====================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_properties() {
        assert_eq!(DungeonDifficulty::Normal.as_str(), "普通");
        assert_eq!(DungeonDifficulty::Hell.emoji(), "⚫");
        assert_eq!(DungeonDifficulty::Expert.num_floors(), 7);
        assert_eq!(DungeonDifficulty::Master.guild_level_req(), 4);
        assert_eq!(DungeonDifficulty::Nightmare.combat_power_req(), 30000);
        assert_eq!(DungeonDifficulty::Hell.diamond_full_clear(), 3000);
    }

    #[test]
    fn test_difficulty_ordering() {
        assert!(DungeonDifficulty::Normal < DungeonDifficulty::Hard);
        assert!(DungeonDifficulty::Expert < DungeonDifficulty::Master);
        assert!(DungeonDifficulty::Nightmare < DungeonDifficulty::Hell);
    }

    #[test]
    fn test_difficulty_all() {
        assert_eq!(DungeonDifficulty::all().len(), 6);
    }

    #[test]
    fn test_difficulty_from_str() {
        assert_eq!(DungeonDifficulty::from_str("普通"), Some(DungeonDifficulty::Normal));
        assert_eq!(DungeonDifficulty::from_str("地狱"), Some(DungeonDifficulty::Hell));
        assert_eq!(DungeonDifficulty::from_str("未知"), None);
    }

    #[test]
    fn test_floor_monster_names() {
        let name = utils::floor_monster_name(DungeonDifficulty::Normal, 0);
        assert_eq!(name, "哥布林哨兵");

        let name = utils::floor_monster_name(DungeonDifficulty::Hell, 14);
        assert_eq!(name, "地狱之主");

        let name = utils::floor_monster_name(DungeonDifficulty::Master, 5);
        assert_eq!(name, "远古巨龙");
    }

    #[test]
    fn test_dungeon_state_new() {
        let state = DungeonState::new(DungeonDifficulty::Hard);
        assert_eq!(state.difficulty, DungeonDifficulty::Hard);
        assert_eq!(state.current_floor, 0);
        assert_eq!(state.total_damage, 0);
        assert_eq!(state.floors_cleared, 0);
        assert!(!state.is_complete());
    }

    #[test]
    fn test_dungeon_state_json_roundtrip() {
        let mut state = DungeonState::new(DungeonDifficulty::Expert);
        state.current_floor = 3;
        state.total_damage = 50000;
        state.floors_cleared = 3;

        let json = state.to_json();
        let restored = DungeonState::from_json(&json).unwrap();
        assert_eq!(restored.difficulty, DungeonDifficulty::Expert);
        assert_eq!(restored.current_floor, 3);
        assert_eq!(restored.total_damage, 50000);
        assert_eq!(restored.floors_cleared, 3);
    }

    #[test]
    fn test_dungeon_state_complete() {
        let mut state = DungeonState::new(DungeonDifficulty::Normal);
        state.floors_cleared = 3;
        assert!(state.is_complete());

        state.floors_cleared = 2;
        assert!(!state.is_complete());
    }

    #[test]
    fn test_combat_simulation_win() {
        // 高战力应该赢
        let (damage, won) = simulate_floor_battle(10000, 3000, 200);
        assert!(won);
        assert!(damage > 0);
    }

    #[test]
    fn test_combat_simulation_loss() {
        // 低战力应该输
        let (_, won) = simulate_floor_battle(10, 300000, 15000);
        assert!(!won);
    }

    #[test]
    fn test_combat_simulation_damage_always_positive() {
        for power in [1, 100, 1000, 50000] {
            let (damage, _) = simulate_floor_battle(power, 1000, 100);
            assert!(damage > 0, "Damage should be positive for power={}", power);
        }
    }

    #[test]
    fn test_difficulty_gold_exp_scaling() {
        let diffs = DungeonDifficulty::all();
        for i in 1..diffs.len() {
            assert!(diffs[i].gold_per_floor() > diffs[i - 1].gold_per_floor());
            assert!(diffs[i].exp_per_floor() > diffs[i - 1].exp_per_floor());
            assert!(diffs[i].diamond_full_clear() > diffs[i - 1].diamond_full_clear());
        }
    }

    #[test]
    fn test_difficulty_requirements_scaling() {
        let diffs = DungeonDifficulty::all();
        for i in 1..diffs.len() {
            assert!(diffs[i].combat_power_req() >= diffs[i - 1].combat_power_req());
            assert!(diffs[i].guild_level_req() >= diffs[i - 1].guild_level_req());
            assert!(diffs[i].base_floor_hp() >= diffs[i - 1].base_floor_hp());
        }
    }

    #[test]
    fn test_dungeon_help_content() {
        let help = cmd_dungeon_help(
            &Database::open("/opt/data/cakegame-rs/CakeGame调试器/data/CakeGameData/gamedata.sdb").unwrap(),
            "test",
            "",
            "",
            "",
        );
        assert!(help.contains("公会副本"));
        assert!(help.contains("查看副本"));
        assert!(help.contains("进入副本"));
        assert!(help.contains("副本战斗"));
        assert!(help.contains("副本进度"));
        assert!(help.contains("副本奖励"));
        assert!(help.contains("副本排行"));
        assert!(help.contains("每日最多挑战5次"));
    }

    #[test]
    fn test_floor_monster_boundary() {
        // 测试边界情况(floor超出范围)
        let name = utils::floor_monster_name(DungeonDifficulty::Normal, 99);
        assert!(!name.is_empty());

        let name = utils::floor_monster_name(DungeonDifficulty::Hell, 0);
        assert_eq!(name, "地狱犬");
    }

    #[test]
    fn test_all_difficulties_have_monsters() {
        for diff in DungeonDifficulty::all() {
            for floor in 0..diff.num_floors() {
                let name = utils::floor_monster_name(*diff, floor);
                assert!(!name.is_empty(), "Empty monster name for {:?} floor {}", diff, floor);
            }
        }
    }

    #[test]
    fn test_state_json_invalid() {
        assert!(DungeonState::from_json("").is_none());
        assert!(DungeonState::from_json("{}").is_none());
        assert!(DungeonState::from_json("invalid").is_none());
    }
}

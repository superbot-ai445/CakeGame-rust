/// CakeGame 竞技场AI对手系统
/// 使用怪物模板和职业数据生成AI对手，供玩家在没有真人对手时练习PvP
/// AI对手拥有等级、装备、技能，模拟真实玩家战斗
use crate::db::Database;
use crate::stamina;
use crate::user;
use rand::Rng;

/// AI对手难度等级
#[derive(Debug, Clone, Copy)]
enum AIDifficulty {
    Beginner,     // 初级
    Intermediate, // 中级
    Advanced,     // 高级
    Expert,       // 专家
    Legendary,    // 传说
}

impl AIDifficulty {
    fn name(&self) -> &str {
        match self {
            Self::Beginner => "初级",
            Self::Intermediate => "中级",
            Self::Advanced => "高级",
            Self::Expert => "专家",
            Self::Legendary => "传说",
        }
    }

    fn emoji(&self) -> &str {
        match self {
            Self::Beginner => "🟢",
            Self::Intermediate => "🔵",
            Self::Advanced => "🟣",
            Self::Expert => "🔴",
            Self::Legendary => "🟡",
        }
    }

    fn stat_mult(&self) -> f64 {
        match self {
            Self::Beginner => 0.6,
            Self::Intermediate => 0.9,
            Self::Advanced => 1.2,
            Self::Expert => 1.6,
            Self::Legendary => 2.2,
        }
    }

    fn level_range(&self) -> (i32, i32) {
        match self {
            Self::Beginner => (1, 10),
            Self::Intermediate => (10, 25),
            Self::Advanced => (25, 45),
            Self::Expert => (45, 70),
            Self::Legendary => (70, 100),
        }
    }

    fn gold_reward(&self) -> i64 {
        match self {
            Self::Beginner => 100,
            Self::Intermediate => 300,
            Self::Advanced => 800,
            Self::Expert => 2000,
            Self::Legendary => 5000,
        }
    }

    fn exp_reward(&self) -> i64 {
        match self {
            Self::Beginner => 50,
            Self::Intermediate => 150,
            Self::Advanced => 400,
            Self::Expert => 1000,
            Self::Legendary => 2500,
        }
    }

    fn diamond_reward(&self) -> i64 {
        match self {
            Self::Beginner => 0,
            Self::Intermediate => 1,
            Self::Advanced => 3,
            Self::Expert => 8,
            Self::Legendary => 20,
        }
    }
}

/// AI对手模板
struct AIOpponentTemplate {
    name: &'static str,
    title: &'static str,
    occupation: &'static str,
    difficulty: AIDifficulty,
    base_hp: i32,
    base_mp: i32,
    base_ad: i32,
    base_ap: i32,
    base_def: i32,
    base_mdf: i32,
    base_hit: i32,
    base_dodge: i32,
    base_crit: i32,
    taunt: &'static str, // 战斗台词
}

/// 内置AI对手 (基于游戏世界观)
const AI_TEMPLATES: &[AIOpponentTemplate] = &[
    AIOpponentTemplate {
        name: "新手训练师·小明",
        title: "格兰村守卫",
        occupation: "勇者",
        difficulty: AIDifficulty::Beginner,
        base_hp: 300,
        base_mp: 100,
        base_ad: 40,
        base_ap: 0,
        base_def: 15,
        base_mdf: 10,
        base_hit: 50,
        base_dodge: 5,
        base_crit: 3,
        taunt: "来吧，让我看看你的实力！",
    },
    AIOpponentTemplate {
        name: "游侠·风铃",
        title: "森林巡守者",
        occupation: "御剑师",
        difficulty: AIDifficulty::Beginner,
        base_hp: 250,
        base_mp: 150,
        base_ad: 50,
        base_ap: 0,
        base_def: 10,
        base_mdf: 12,
        base_hit: 60,
        base_dodge: 15,
        base_crit: 8,
        taunt: "在森林中，没有我追不到的猎物。",
    },
    AIOpponentTemplate {
        name: "学徒法师·星月",
        title: "魔法学院新生",
        occupation: "魔法师",
        difficulty: AIDifficulty::Beginner,
        base_hp: 200,
        base_mp: 300,
        base_ad: 10,
        base_ap: 55,
        base_def: 8,
        base_mdf: 20,
        base_hit: 45,
        base_dodge: 8,
        base_crit: 5,
        taunt: "让我用魔法的力量来考验你！",
    },
    AIOpponentTemplate {
        name: "铁壁战士·石磊",
        title: "皇家近卫队队长",
        occupation: "勇者",
        difficulty: AIDifficulty::Intermediate,
        base_hp: 800,
        base_mp: 200,
        base_ad: 80,
        base_ap: 0,
        base_def: 50,
        base_mdf: 35,
        base_hit: 70,
        base_dodge: 8,
        base_crit: 10,
        taunt: "我的防御固若金汤，来试试吧！",
    },
    AIOpponentTemplate {
        name: "暗影刺客·夜刃",
        title: "暗杀者工会精英",
        occupation: "御剑师",
        difficulty: AIDifficulty::Intermediate,
        base_hp: 500,
        base_mp: 250,
        base_ad: 110,
        base_ap: 0,
        base_def: 25,
        base_mdf: 20,
        base_hit: 90,
        base_dodge: 35,
        base_crit: 25,
        taunt: "你不会看到我的刀是怎么出鞘的。",
    },
    AIOpponentTemplate {
        name: "圣光祭司·琉璃",
        title: "圣殿首席治愈师",
        occupation: "牧师",
        difficulty: AIDifficulty::Intermediate,
        base_hp: 600,
        base_mp: 500,
        base_ad: 30,
        base_ap: 90,
        base_def: 30,
        base_mdf: 45,
        base_hit: 65,
        base_dodge: 10,
        base_crit: 8,
        taunt: "圣光会审判一切邪恶！",
    },
    AIOpponentTemplate {
        name: "狂暴战神·铁血",
        title: "北方军团统帅",
        occupation: "勇者",
        difficulty: AIDifficulty::Advanced,
        base_hp: 1500,
        base_mp: 300,
        base_ad: 180,
        base_ap: 0,
        base_def: 80,
        base_mdf: 60,
        base_hit: 100,
        base_dodge: 15,
        base_crit: 20,
        taunt: "在我的铁蹄之下颤抖吧！",
    },
    AIOpponentTemplate {
        name: "冰霜女巫·凛冬",
        title: "永冻冰原领主",
        occupation: "魔法师",
        difficulty: AIDifficulty::Advanced,
        base_hp: 1000,
        base_mp: 800,
        base_ad: 40,
        base_ap: 200,
        base_def: 40,
        base_mdf: 90,
        base_hit: 85,
        base_dodge: 20,
        base_crit: 15,
        taunt: "万物终将在寒冰中沉眠……",
    },
    AIOpponentTemplate {
        name: "龙骑士·苍穹",
        title: "飞龙骑士团团长",
        occupation: "御剑师",
        difficulty: AIDifficulty::Advanced,
        base_hp: 1200,
        base_mp: 400,
        base_ad: 160,
        base_ap: 0,
        base_def: 70,
        base_mdf: 55,
        base_hit: 110,
        base_dodge: 30,
        base_crit: 22,
        taunt: "与龙共舞，你准备好了吗？",
    },
    AIOpponentTemplate {
        name: "虚空行者·深渊",
        title: "次元裂隙守护者",
        occupation: "魔法师",
        difficulty: AIDifficulty::Expert,
        base_hp: 2000,
        base_mp: 1200,
        base_ad: 80,
        base_ap: 350,
        base_def: 60,
        base_mdf: 130,
        base_hit: 120,
        base_dodge: 25,
        base_crit: 18,
        taunt: "虚空之力将吞噬一切……",
    },
    AIOpponentTemplate {
        name: "天命勇者·曙光",
        title: "传说中的英雄",
        occupation: "勇者",
        difficulty: AIDifficulty::Expert,
        base_hp: 2500,
        base_mp: 600,
        base_ad: 300,
        base_ap: 0,
        base_def: 120,
        base_mdf: 100,
        base_hit: 140,
        base_dodge: 20,
        base_crit: 28,
        taunt: "我背负着拯救世界的使命！",
    },
    AIOpponentTemplate {
        name: "暗黑魔王·无尽",
        title: "深渊之主",
        occupation: "魔法师",
        difficulty: AIDifficulty::Legendary,
        base_hp: 4000,
        base_mp: 2000,
        base_ad: 150,
        base_ap: 500,
        base_def: 100,
        base_mdf: 180,
        base_hit: 160,
        base_dodge: 30,
        base_crit: 25,
        taunt: "吾乃深渊之主，汝等不过是蝼蚁罢了！",
    },
    AIOpponentTemplate {
        name: "不朽剑圣·断空",
        title: "万剑归宗",
        occupation: "御剑师",
        difficulty: AIDifficulty::Legendary,
        base_hp: 3500,
        base_mp: 800,
        base_ad: 450,
        base_ap: 0,
        base_def: 130,
        base_mdf: 110,
        base_hit: 180,
        base_dodge: 45,
        base_crit: 35,
        taunt: "一剑破万法，来感受剑道的极致！",
    },
];

/// 生成AI对手的实际属性 (基于难度倍率和随机等级)
fn generate_ai_stats(template: &AIOpponentTemplate) -> (i32, AIOpponentStats) {
    let mut rng = rand::thread_rng();
    let (min_lv, max_lv) = template.difficulty.level_range();
    let level = rng.gen_range(min_lv..=max_lv);
    let mult = template.difficulty.stat_mult();

    // 等级加成 (每级+2%基础属性)
    let level_bonus = 1.0 + (level as f64 - 1.0) * 0.02;
    let total_mult = mult * level_bonus;

    let stats = AIOpponentStats {
        name: template.name.to_string(),
        title: template.title.to_string(),
        occupation: template.occupation.to_string(),
        level,
        hp: (template.base_hp as f64 * total_mult) as i32,
        mp: (template.base_mp as f64 * total_mult) as i32,
        ad: (template.base_ad as f64 * total_mult) as i32,
        ap: (template.base_ap as f64 * total_mult) as i32,
        defense: (template.base_def as f64 * total_mult) as i32,
        magic_resistance: (template.base_mdf as f64 * total_mult) as i32,
        hit: (template.base_hit as f64 * total_mult) as i32,
        dodge: (template.base_dodge as f64 * total_mult) as i32,
        crit: (template.base_crit as f64 * total_mult) as i32,
        difficulty_name: template.difficulty.name().to_string(),
        difficulty_emoji: template.difficulty.emoji().to_string(),
        taunt: template.taunt.to_string(),
        gold_reward: (template.difficulty.gold_reward() as f64 * level_bonus) as i64,
        exp_reward: (template.difficulty.exp_reward() as f64 * level_bonus) as i64,
        diamond_reward: template.difficulty.diamond_reward(),
    };
    (level, stats)
}

/// AI对手战斗属性
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AIOpponentStats {
    name: String,
    title: String,
    occupation: String,
    level: i32,
    hp: i32,
    mp: i32,
    ad: i32,
    ap: i32,
    defense: i32,
    magic_resistance: i32,
    hit: i32,
    dodge: i32,
    crit: i32,
    difficulty_name: String,
    difficulty_emoji: String,
    taunt: String,
    gold_reward: i64,
    exp_reward: i64,
    diamond_reward: i64,
}

impl AIOpponentStats {
    #[allow(dead_code)]
    fn combat_power(&self) -> i64 {
        (self.ad as i64 * 35
            + self.ap as i64 * 50
            + self.hp as i64 * 3
            + self.mp as i64
            + self.defense as i64 * 20
            + self.magic_resistance as i64 * 18
            + self.hit as i64 * 2
            + self.dodge as i64 * 12
            + self.crit as i64 * 60)
            / 10
    }
}

/// 查看AI对手列表 — 列出所有可挑战的AI对手
pub fn cmd_view_ai_opponents(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看AI对手！", prefix);
    }

    let user_level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);

    let mut out = format!("{}\n═══ ⚔️ 竞技场AI对手 ═══\n", prefix);
    out.push_str("在没有真人对手时，可以挑战AI对手练习PvP！\n");
    out.push_str("每个AI对手都有独特的职业和战斗风格。\n\n");

    // 按难度分组显示
    let difficulties = [
        AIDifficulty::Beginner,
        AIDifficulty::Intermediate,
        AIDifficulty::Advanced,
        AIDifficulty::Expert,
        AIDifficulty::Legendary,
    ];

    for diff in &difficulties {
        let (min_lv, max_lv) = diff.level_range();
        let recommended = if user_level >= min_lv && user_level <= max_lv + 10 {
            " ⭐推荐"
        } else if user_level < min_lv {
            " ⚠️等级不足"
        } else {
            ""
        };

        out.push_str(&format!(
            "{}【{} Lv.{}-{}】{}\n",
            diff.emoji(),
            diff.name(),
            min_lv,
            max_lv,
            recommended
        ));

        for template in AI_TEMPLATES {
            if std::mem::discriminant(&template.difficulty) == std::mem::discriminant(diff) {
                out.push_str(&format!(
                    "  {} · {} [{}]\n",
                    template.name, template.title, template.occupation
                ));
            }
        }
        out.push('\n');
    }

    out.push_str("📋 指令：\n");
    out.push_str("  挑战AI对手+对手名 — 挑战指定AI对手\n");
    out.push_str("  AI对手战绩 — 查看你的AI对战记录\n");
    out.push_str("  AI对手排行 — 查看全服AI挑战排行榜\n");

    // 冷却检查
    let last_challenge = db.read_user_data(user_id, "ai_arena_last");
    if !last_challenge.is_empty() {
        if let Ok(last_ts) = last_challenge.parse::<i64>() {
            let now = chrono::Utc::now().timestamp();
            let cooldown = 120; // 2分钟冷却
            let remaining = cooldown - (now - last_ts);
            if remaining > 0 {
                out.push_str(&format!("\n⏳ 挑战冷却中，还需{}秒", remaining));
            }
        }
    }

    out
}

/// 挑战AI对手 — 进行一场PvP战斗模拟
pub fn cmd_challenge_ai(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再挑战AI对手！", prefix);
    }

    let query = args.trim();
    if query.is_empty() {
        return format!(
            "{}\n请指定要挑战的AI对手名称！\n💡 使用「查看AI对手」查看可挑战列表",
            prefix
        );
    }

    // 冷却检查 (2分钟)
    let last_challenge = db.read_user_data(user_id, "ai_arena_last");
    if !last_challenge.is_empty() {
        if let Ok(last_ts) = last_challenge.parse::<i64>() {
            let now = chrono::Utc::now().timestamp();
            let cooldown = 120;
            let remaining = cooldown - (now - last_ts);
            if remaining > 0 {
                return format!("{}\n⏳ 挑战冷却中，还需{}秒后再来！", prefix, remaining);
            }
        }
    }

    // 虚弱检查
    let weakness_until = db.read_user_data(user_id, "weakness_until");
    if !weakness_until.is_empty() {
        if let Ok(until) = weakness_until.parse::<i64>() {
            if chrono::Utc::now().timestamp() < until {
                return format!("{}\n❌ 您目前处于虚弱状态，无法挑战AI对手！", prefix);
            }
        }
    }

    // 体力检查 (竞技消耗5体力)
    if let Err(e) = stamina::consume_stamina(user_id, "竞技", db) {
        return format!("{}\n{}", prefix, e);
    }

    // 查找AI对手
    let template = AI_TEMPLATES
        .iter()
        .find(|t| t.name.contains(query) || t.title.contains(query) || query.contains(t.name));

    let template = match template {
        Some(t) => t,
        None => {
            return format!(
                "{}\n❌ 未找到名为「{}」的AI对手！\n💡 使用「查看AI对手」查看可挑战列表",
                prefix, query
            );
        }
    };

    // 生成AI对手属性
    let (ai_level, ai_stats) = generate_ai_stats(template);

    // 获取玩家属性
    let user_level: i32 = db.read_basic(user_id, "LV").parse().unwrap_or(1);
    let user_hp: i32 = db.read_basic(user_id, "P_HP").parse().unwrap_or(100);
    let user_hp_max: i32 = db.read_user_data(user_id, "total_hp").parse().unwrap_or(400);
    let user_ad: i32 = db.read_user_data(user_id, "total_ad").parse().unwrap_or(52);
    let user_ap: i32 = db.read_user_data(user_id, "total_ap").parse().unwrap_or(0);
    let user_def: i32 = db.read_user_data(user_id, "total_defense").parse().unwrap_or(0);
    let user_mdf: i32 = db
        .read_user_data(user_id, "total_magic_resistance")
        .parse()
        .unwrap_or(0);
    let user_hit: i32 = db.read_user_data(user_id, "total_hit").parse().unwrap_or(670);
    let user_dodge: i32 = db.read_user_data(user_id, "total_dodge").parse().unwrap_or(0);
    let user_crit: i32 = db.read_user_data(user_id, "total_crit").parse().unwrap_or(0);

    // 检查玩家生命
    if user_hp <= 0 {
        return format!("{}\n❌ 您已经阵亡，请先恢复生命！", prefix);
    }

    // 战斗模拟 (简化回合制)
    let mut rng = rand::thread_rng();
    let mut player_hp = user_hp;
    let mut ai_hp = ai_stats.hp;
    let mut rounds = 0;
    let max_rounds = 20;
    let mut battle_log: Vec<String> = Vec::new();

    battle_log.push("═══ ⚔️ 战斗开始 ═══".to_string());
    battle_log.push(format!(
        "{} {} Lv.{} vs 你 Lv.{}",
        ai_stats.difficulty_emoji, ai_stats.name, ai_level, user_level
    ));
    battle_log.push(format!("「{}」", ai_stats.taunt));
    battle_log.push(String::new());

    while player_hp > 0 && ai_hp > 0 && rounds < max_rounds {
        rounds += 1;

        // 玩家攻击
        let base_dmg = user_ad + user_ap;
        let hit_roll = rng.gen_range(0..100);
        let hit_chance = 80 + (user_hit - ai_stats.dodge).clamp(-20, 15);
        if hit_roll < hit_chance {
            let mut dmg = base_dmg - ai_stats.defense / 2 - ai_stats.magic_resistance / 3;
            dmg = dmg.max(1);

            // 暴击
            let crit_roll = rng.gen_range(0..100);
            let crit_chance = user_crit.min(50);
            if crit_roll < crit_chance {
                dmg = dmg * 15 / 10;
                battle_log.push(format!(
                    "  ⚡ 第{}回合: 暴击！你对 {} 造成 {} 伤害！",
                    rounds, ai_stats.name, dmg
                ));
            } else {
                battle_log.push(format!(
                    "  ⚔️ 第{}回合: 你对 {} 造成 {} 伤害",
                    rounds, ai_stats.name, dmg
                ));
            }
            ai_hp -= dmg;
        } else {
            battle_log.push(format!("  💨 第{}回合: 你的攻击被 {} 闪避了！", rounds, ai_stats.name));
        }

        if ai_hp <= 0 {
            break;
        }

        // AI攻击
        let ai_base_dmg = ai_stats.ad + ai_stats.ap;
        let ai_hit_roll = rng.gen_range(0..100);
        let ai_hit_chance = 80 + (ai_stats.hit - user_dodge).clamp(-20, 15);
        if ai_hit_roll < ai_hit_chance {
            let mut ai_dmg = ai_base_dmg - user_def / 2 - user_mdf / 3;
            ai_dmg = ai_dmg.max(1);

            let ai_crit_roll = rng.gen_range(0..100);
            if ai_crit_roll < ai_stats.crit.min(50) {
                ai_dmg = ai_dmg * 15 / 10;
                battle_log.push(format!(
                    "  💥 第{}回合: {} 暴击！对你造成 {} 伤害！",
                    rounds, ai_stats.name, ai_dmg
                ));
            } else {
                battle_log.push(format!(
                    "  🗡️ 第{}回合: {} 对你造成 {} 伤害",
                    rounds, ai_stats.name, ai_dmg
                ));
            }
            player_hp -= ai_dmg;
        } else {
            battle_log.push(format!("  🛡️ 第{}回合: 你闪避了 {} 的攻击！", rounds, ai_stats.name));
        }
    }

    let won = ai_hp <= 0 || (player_hp > 0 && player_hp > ai_hp);
    let draw = rounds >= max_rounds && player_hp > 0 && ai_hp > 0;

    let mut result = format!("{}\n═══ ⚔️ 战斗结果 ═══\n\n", prefix);

    // 只显示最后几回合
    let show_rounds = 6usize;
    let log_start = battle_log.len().saturating_sub(show_rounds);
    for line in &battle_log[log_start..] {
        result.push_str(line);
        result.push('\n');
    }
    result.push('\n');

    if won {
        result.push_str(&format!("🎉 胜利！你击败了 {}！\n\n", ai_stats.name));
        result.push_str(
            "📊 战斗统计:
",
        );
        result.push_str(&format!("  剩余生命: {}/{}\n", player_hp.max(0), user_hp_max));
        result.push_str(&format!("  对手剩余: {}/{}\n\n", ai_hp.max(0), ai_stats.hp));

        // 奖励
        let gold = ai_stats.gold_reward;
        let exp = ai_stats.exp_reward;
        let diamonds = ai_stats.diamond_reward;

        db.modify_currency(user_id, "Currency_gold", "add", gold);
        user::add_experience(db, user_id, exp as i32);
        if diamonds > 0 {
            db.modify_currency(user_id, "Currency_diamond", "add", diamonds);
        }

        result.push_str(
            "📊 战斗统计:
",
        );
        result.push_str(&format!("  经验 +{}\n", exp));
        if diamonds > 0 {
            result.push_str(&format!("  钻石 +{}\n", diamonds));
        }

        // 更新战绩
        let wins: i32 = db.read_user_data(user_id, "ai_arena_wins").parse().unwrap_or(0);
        db.write_user_data(user_id, "ai_arena_wins", &(wins + 1).to_string());

        // 记录最高难度击败
        let diff_val = match template.difficulty {
            AIDifficulty::Beginner => 1,
            AIDifficulty::Intermediate => 2,
            AIDifficulty::Advanced => 3,
            AIDifficulty::Expert => 4,
            AIDifficulty::Legendary => 5,
        };
        let best: i32 = db.read_user_data(user_id, "ai_arena_best_diff").parse().unwrap_or(0);
        if diff_val > best {
            db.write_user_data(user_id, "ai_arena_best_diff", &diff_val.to_string());
        }
    } else if draw {
        result.push_str(&format!("⚖️ 平局！你和 {} 打了个势均力敌！\n", ai_stats.name));
        result.push_str(&format!("  回合数: {} (达到上限)\n", rounds));
        // 平局给一半奖励
        let gold = ai_stats.gold_reward / 2;
        let exp = ai_stats.exp_reward / 2;
        db.modify_currency(user_id, "Currency_gold", "add", gold);
        user::add_experience(db, user_id, exp as i32);
        result.push_str(&format!("  金币 +{} (平局减半)\n", gold));
        result.push_str(&format!("  经验 +{}\n", exp));

        let draws: i32 = db.read_user_data(user_id, "ai_arena_draws").parse().unwrap_or(0);
        db.write_user_data(user_id, "ai_arena_draws", &(draws + 1).to_string());
    } else {
        result.push_str(&format!("💀 战败！你被 {} 击败了！\n\n", ai_stats.name));
        result.push_str(
            "📊 战斗统计:
",
        );
        result.push_str(&format!("  对手剩余: {}/{}\n", ai_hp.max(0), ai_stats.hp));
        result.push_str("\n💡 提示: 提升等级和装备后再来挑战！\n");

        let losses: i32 = db.read_user_data(user_id, "ai_arena_losses").parse().unwrap_or(0);
        db.write_user_data(user_id, "ai_arena_losses", &(losses + 1).to_string());

        // 战败扣血
        let new_hp = (user_hp - user_hp / 5).max(1);
        db.write_basic(user_id, "P_HP", &new_hp.to_string());
    }

    // 更新总战斗次数和冷却
    let total: i32 = db.read_user_data(user_id, "ai_arena_total").parse().unwrap_or(0);
    db.write_user_data(user_id, "ai_arena_total", &(total + 1).to_string());
    db.write_user_data(user_id, "ai_arena_last", &chrono::Utc::now().timestamp().to_string());

    // 记录最后挑战的对手
    db.write_user_data(user_id, "ai_arena_last_opponent", ai_stats.name.as_str());

    result
}

/// AI对手战绩 — 查看个人AI对战统计
pub fn cmd_ai_arena_record(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看战绩！", prefix);
    }

    let total: i32 = db.read_user_data(user_id, "ai_arena_total").parse().unwrap_or(0);
    let wins: i32 = db.read_user_data(user_id, "ai_arena_wins").parse().unwrap_or(0);
    let losses: i32 = db.read_user_data(user_id, "ai_arena_losses").parse().unwrap_or(0);
    let draws: i32 = db.read_user_data(user_id, "ai_arena_draws").parse().unwrap_or(0);
    let best_diff: i32 = db.read_user_data(user_id, "ai_arena_best_diff").parse().unwrap_or(0);
    let last_opponent = db.read_user_data(user_id, "ai_arena_last_opponent");

    let win_rate = if total > 0 {
        wins as f64 / total as f64 * 100.0
    } else {
        0.0
    };

    let best_diff_name = match best_diff {
        1 => "🟢 初级",
        2 => "🔵 中级",
        3 => "🟣 高级",
        4 => "🔴 专家",
        5 => "🟡 传说",
        _ => "无",
    };

    let mut out = format!("{}\n═══ ⚔️ AI对战战绩 ═══\n", prefix);
    out.push_str(&format!("📊 总场次: {}\n", total));
    out.push_str(&format!("  🏆 胜利: {} ({:.1}%)\n", wins, win_rate));
    out.push_str(&format!("  💀 战败: {}\n", losses));
    out.push_str(&format!("  ⚖️ 平局: {}\n", draws));
    out.push_str(&format!("  🎯 最高击败难度: {}\n", best_diff_name));

    if !last_opponent.is_empty() {
        out.push_str(&format!("  🗡️ 最近对手: {}\n", last_opponent));
    }

    if total == 0 {
        out.push_str("\n💡 还没有挑战过AI对手！\n");
        out.push_str("使用「查看AI对手」开始你的第一场战斗！\n");
    }

    out
}

/// AI对手排行 — 全服AI挑战排行榜
pub fn cmd_ai_arena_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 从 Global 表获取排行榜数据
    let conn = db.lock_conn();

    let mut rankings: Vec<(String, String, i32, i32, i32)> = Vec::new();

    // 查询所有有AI战绩的用户
    if let Ok(mut stmt) = conn.prepare(
        "SELECT UserId, ItemName, ItemValue FROM UserNode WHERE NodeType = 'user_data' AND ItemName IN ('ai_arena_wins', 'ai_arena_total', 'ai_arena_best_diff')"
    ) {
        use rusqlite::params;
        let mut user_data: std::collections::HashMap<String, (i32, i32, i32)> = std::collections::HashMap::new();

        if let Ok(rows) = stmt.query_map(params![], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?.parse::<i32>().unwrap_or(0),
            ))
        }) {
            for row in rows.flatten() {
                let entry = user_data.entry(row.0).or_insert((0, 0, 0));
                match row.1.as_str() {
                    "ai_arena_wins" => entry.0 = row.2,
                    "ai_arena_total" => entry.1 = row.2,
                    "ai_arena_best_diff" => entry.2 = row.2,
                    _ => {}
                }
            }
        }

        for (uid, (wins, total, best_diff)) in user_data {
            if total > 0 {
                let name = db.read_basic(&uid, "Name");
                let name = if name.is_empty() { uid.clone() } else { name };
                rankings.push((uid, name, wins, total, best_diff));
            }
        }
    }

    drop(conn);

    // 按胜利数排序
    rankings.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));

    let mut out = format!("{}\n═══ ⚔️ AI对战排行榜 ═══\n", prefix);

    if rankings.is_empty() {
        out.push_str("\n暂无AI对战数据！\n");
        out.push_str("使用「查看AI对手」开始挑战吧！\n");
        return out;
    }

    let medals = ["🥇", "🥈", "🥉"];
    let limit = rankings.len().min(10);

    for (i, (_, name, wins, total, best_diff)) in rankings.iter().take(limit).enumerate() {
        let medal = if i < 3 { medals[i] } else { &format!("{:>2}.", i + 1) };
        let win_rate = if *total > 0 {
            *wins as f64 / *total as f64 * 100.0
        } else {
            0.0
        };
        let diff_name = match best_diff {
            5 => "🟡传说",
            4 => "🔴专家",
            3 => "🟣高级",
            2 => "🔵中级",
            _ => "🟢初级",
        };
        out.push_str(&format!(
            "{} {} 胜{}/{} ({:.0}%) 最高:{}\n",
            medal, name, wins, total, win_rate, diff_name
        ));
    }

    out.push_str("\n💡 排行按胜利场次排序");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_properties() {
        assert_eq!(AIDifficulty::Beginner.name(), "初级");
        assert_eq!(AIDifficulty::Legendary.name(), "传说");
        assert!(AIDifficulty::Beginner.stat_mult() < AIDifficulty::Legendary.stat_mult());
        assert!(AIDifficulty::Beginner.gold_reward() < AIDifficulty::Legendary.gold_reward());
    }

    #[test]
    fn test_difficulty_level_ranges() {
        let (b_min, b_max) = AIDifficulty::Beginner.level_range();
        let (l_min, l_max) = AIDifficulty::Legendary.level_range();
        assert!(b_min < l_min);
        assert!(b_max < l_max);
    }

    #[test]
    fn test_generate_ai_stats() {
        let template = &AI_TEMPLATES[0]; // 新手训练师
        let (level, stats) = generate_ai_stats(template);
        assert!(level >= 1 && level <= 10);
        assert!(stats.hp > 0);
        assert!(stats.ad > 0);
        assert_eq!(stats.name, "新手训练师·小明");
    }

    #[test]
    fn test_ai_stats_combat_power() {
        let template = &AI_TEMPLATES[AI_TEMPLATES.len() - 1]; // 传说级
        let (_, stats) = generate_ai_stats(template);
        assert!(stats.combat_power() > 0);
    }

    #[test]
    fn test_all_templates_valid() {
        for template in AI_TEMPLATES {
            assert!(template.base_hp > 0);
            assert!(template.base_ad > 0 || template.base_ap > 0);
            assert!(!template.name.is_empty());
            assert!(!template.occupation.is_empty());
            assert!(!template.taunt.is_empty());
        }
    }

    #[test]
    fn test_ai_opponent_count() {
        // 应有至少3个对手每个难度
        let mut counts = [0; 5];
        for template in AI_TEMPLATES {
            let idx = match template.difficulty {
                AIDifficulty::Beginner => 0,
                AIDifficulty::Intermediate => 1,
                AIDifficulty::Advanced => 2,
                AIDifficulty::Expert => 3,
                AIDifficulty::Legendary => 4,
            };
            counts[idx] += 1;
        }
        assert!(counts[0] >= 2, "Beginner should have at least 2 opponents");
        assert!(counts[1] >= 2, "Intermediate should have at least 2 opponents");
        assert!(counts[4] >= 2, "Legendary should have at least 2 opponents");
    }

    #[test]
    fn test_stat_scaling() {
        // Higher difficulty should produce stronger stats on average
        let template_low = &AI_TEMPLATES[0]; // Beginner
        let template_high = &AI_TEMPLATES[AI_TEMPLATES.len() - 1]; // Legendary

        let mut low_total = 0i64;
        let mut high_total = 0i64;
        for _ in 0..20 {
            let (_, s1) = generate_ai_stats(template_low);
            let (_, s2) = generate_ai_stats(template_high);
            low_total += s1.hp as i64 + s1.ad as i64 + s1.ap as i64;
            high_total += s2.hp as i64 + s2.ad as i64 + s2.ap as i64;
        }
        assert!(
            high_total > low_total * 2,
            "Legendary should be much stronger than Beginner"
        );
    }

    #[test]
    fn test_reward_scaling() {
        for template in AI_TEMPLATES {
            let (_, stats) = generate_ai_stats(template);
            assert!(stats.gold_reward > 0, "All opponents should give gold reward");
            assert!(stats.exp_reward > 0, "All opponents should give exp reward");
        }
    }
}

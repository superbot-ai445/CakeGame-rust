/// CakeGame 每日答题挑战系统
///
/// 每天生成 5 道 RPG 知识题目，答对获得金币/钻石奖励。
/// 连续答题天数越多，奖励越高（连续天数加成）。
/// 全服答题排行榜按累计答对题数排名。
///
/// 数据存储: Global 表 section = "quiz"
///
/// 指令: 开始答题, 答题, 答题排行, 我的答题记录
use crate::core::*;
use crate::db::Database;

/// 题库: (题目, 选项A, 选项B, 选项C, 选项D, 正确答案编号1-4, 解析)
const QUESTION_POOL: &[(&str, &str, &str, &str, &str, u8, &str)] = &[
    // 战斗系统类
    (
        "游戏中「暴击」属性的英文缩写是什么？",
        "AD",
        "AP",
        "Crit",
        "Hit",
        3,
        "Crit = Critical，暴击率",
    ),
    (
        "「释放技能」需要消耗什么资源？",
        "金币",
        "魔法值(MP)",
        "钻石",
        "体力",
        2,
        "技能消耗MP",
    ),
    (
        "被怪物击杀后会进入什么状态？",
        "虚弱状态",
        "无敌状态",
        "加速状态",
        "隐身状态",
        1,
        "死亡后获得虚弱debuff",
    ),
    (
        "攻击玩家需要先执行什么操作？",
        "搜索怪物",
        "锁定玩家",
        "释放技能",
        "进入地图",
        2,
        "先锁定玩家才能攻击",
    ),
    (
        "游戏中「Dodge」属性代表什么？",
        "暴击",
        "闪避",
        "穿透",
        "吸血",
        2,
        "Dodge = 闪避",
    ),
    (
        "「AbsorbHP」属性有什么效果？",
        "增加最大生命",
        "生命偷取",
        "减少受到伤害",
        "增加防御",
        2,
        "AbsorbHP = 生命偷取",
    ),
    (
        "「ImmuneDamage」属性有什么效果？",
        "免疫所有伤害",
        "伤害免疫(减伤百分比)",
        "增加魔抗",
        "增加闪避",
        2,
        "减伤百分比",
    ),
    (
        "战斗中可以使用什么恢复HP？",
        "食物",
        "药水",
        "技能",
        "以上都可以",
        4,
        "多种方式恢复HP",
    ),
    // 职业系统类
    (
        "以下哪个是游戏中的可选职业？",
        "战士",
        "勇者",
        "骑士",
        "弓手",
        2,
        "勇者是初始职业之一",
    ),
    (
        "「御剑师」是什么类型的职业？",
        "物理近战",
        "魔法远程",
        "物理远程",
        "辅助治疗",
        1,
        "御剑师以剑术近战为主",
    ),
    (
        "转换职业需要消耗什么？",
        "金币",
        "钻石",
        "等级",
        "以上都需要",
        4,
        "转职需要等级和资源",
    ),
    ("游戏中共有多少个可选职业？", "4", "5", "6", "7", 3, "6个职业"),
    // 地图系统类
    (
        "进入相邻地图使用的指令是什么？",
        "移动+方向",
        "进入+方向",
        "传送+方向",
        "前往+方向",
        2,
        "使用「进入+方向」",
    ),
    (
        "地图传送需要消耗什么？",
        "金币",
        "钻石",
        "传送石",
        "无需消耗",
        1,
        "地图传送消耗金币",
    ),
    (
        "「查看地图」指令显示什么信息？",
        "所有地图",
        "当前位置和相邻地图",
        "怪物分布",
        "玩家分布",
        2,
        "显示当前位置及出口",
    ),
    // 物品系统类
    (
        "「强化」装备的英文是？",
        "Enhance",
        "Enchant",
        "Upgrade",
        "Refine",
        1,
        "强化 = Enhance",
    ),
    (
        "「附魔」和「强化」是同一种操作吗？",
        "是",
        "不是，附魔增加元素属性",
        "不确定",
        "看情况",
        2,
        "附魔增加元素属性，强化增加基础属性",
    ),
    (
        "以下哪个不是装备部位？",
        "武器",
        "饰品",
        "药水",
        "头盔",
        3,
        "药水是消耗品不是装备部位",
    ),
    (
        "「合成」物品需要什么？",
        "金币",
        "合成配方",
        "钻石",
        "等级",
        2,
        "需要对应合成配方",
    ),
    (
        "分解装备可以获得什么？",
        "金币",
        "材料和强化石",
        "钻石",
        "经验",
        2,
        "分解获得材料",
    ),
    // 公会系统类
    (
        "创建公会需要消耗什么？",
        "金币",
        "钻石",
        "等级",
        "声望",
        2,
        "创建公会消耗钻石",
    ),
    ("公会试炼每周重置几次？", "0次", "1次", "2次", "3次", 2, "每周重置1次"),
    (
        "「公会战」中每方最多几人参战？",
        "5人",
        "8人",
        "10人",
        "15人",
        3,
        "每方最多10人",
    ),
    // 社交系统类
    (
        "好友系统最多可以添加多少好友？",
        "50",
        "100",
        "无上限",
        "200",
        3,
        "无上限添加好友",
    ),
    (
        "「对比」指令可以做什么？",
        "交易装备",
        "比较两个玩家属性",
        "组队",
        "PK",
        2,
        "对比两个玩家属性",
    ),
    // 游戏机制类
    (
        "签到连续多少天可以获得里程碑奖励？",
        "3天",
        "5天",
        "7天",
        "10天",
        1,
        "连续3天即可获得里程碑奖励",
    ),
    (
        "VIP签到每天可以额外获得什么？",
        "经验",
        "VIP积分",
        "钻石",
        "装备",
        2,
        "VIP签到获得积分",
    ),
    (
        "离线收益最多可以累积多少小时？",
        "8小时",
        "12小时",
        "24小时",
        "48小时",
        3,
        "最多累积24小时",
    ),
    ("每日祈福系统可以祈福几次？", "1次", "2次", "3次", "5次", 1, "每天1次"),
    ("「无尽深渊」共有多少层？", "50层", "80层", "100层", "200层", 3, "100层"),
    // 种植系统类
    (
        "种植药材需要什么资源？",
        "种子",
        "金币",
        "灵力",
        "以上都需要",
        4,
        "需要种子和灵力",
    ),
    (
        "药园中「转换灵气」可以用什么兑换？",
        "金币或钻石",
        "经验",
        "材料",
        "装备",
        1,
        "金币或钻石兑换灵力",
    ),
    // 采集系统类
    (
        "采集系统有几种采集类型？",
        "2种",
        "3种",
        "4种",
        "5种",
        3,
        "钓鱼/挖矿/采药/收集 4种",
    ),
    (
        "采集需要消耗什么？",
        "体力",
        "金币",
        "时间",
        "金币和冷却时间",
        4,
        "消耗金币且有冷却时间",
    ),
    // BOSS系统类
    (
        "野外BOSS在哪里查看？",
        "查看BOSS",
        "搜索怪物",
        "查看地图",
        "进入地图",
        1,
        "使用「查看BOSS」",
    ),
    (
        "挑战副本失败会怎样？",
        "扣除金币",
        "扣除HP不重置进度",
        "回到起点",
        "无影响",
        2,
        "失败扣血但保留进度",
    ),
    // 经济系统类
    (
        "「钱庄」系统可以做什么？",
        "交易装备",
        "存款和取款",
        "购买商品",
        "拍卖",
        2,
        "钱庄用于存取金币",
    ),
    (
        "「拍卖行」可以做什么？",
        "NPC商店购买",
        "玩家间物品交易",
        "分解装备",
        "强化装备",
        2,
        "拍卖行是玩家交易市场",
    ),
    (
        "出售物品给NPC的价格是原价的多少？",
        "50%",
        "30%",
        "70%",
        "100%",
        2,
        "NPC收购价约30%",
    ),
    // 灵兽系统类
    (
        "灵兽出战后能提供什么加成？",
        "经验值",
        "属性加成(HP/AD/防御等)",
        "金币",
        "技能",
        2,
        "灵兽提供属性加成",
    ),
    (
        "灵兽的忠诚度会影响什么？",
        "外观",
        "加成比例",
        "等级",
        "技能",
        2,
        "忠诚度影响加成比例",
    ),
    // 抽奖系统类
    (
        "抽奖系统有几种奖池？",
        "1种",
        "2种",
        "3种",
        "4种",
        3,
        "普通池/高级池/至尊池",
    ),
    (
        "「十连抽」指的是什么？",
        "抽10次单抽",
        "一次抽10个",
        "分10天各抽1次",
        "以上都不是",
        2,
        "一次抽取10个物品",
    ),
    // 称号系统类
    (
        "称号有什么实际效果？",
        "纯装饰",
        "可增加属性",
        "提升等级",
        "增加经验",
        2,
        "称号可增加属性",
    ),
    // 副本系统类
    (
        "副本排行榜记录什么？",
        "等级排行",
        "通关时间和速度",
        "金币排行",
        "装备评分",
        2,
        "记录副本通关成绩",
    ),
    // 赎罪系统类
    (
        "邪恶值可以通过什么方式降低？",
        "等待自然消退",
        "金币或钻石赎罪",
        "击杀怪物",
        "下线",
        2,
        "通过金币或钻石赎罪",
    ),
    // 烹饪系统类
    (
        "烹饪系统可以制作什么？",
        "装备",
        "食物",
        "药水",
        "材料",
        2,
        "烹饪制作食物",
    ),
    // 修炼系统类
    (
        "以下哪种不是修炼类型？",
        "吐纳",
        "冥想",
        "烹饪",
        "习法",
        3,
        "烹饪是生活技能不是修炼",
    ),
    (
        "修炼可以增加什么？",
        "等级",
        "属性加成",
        "金币",
        "技能等级",
        2,
        "修炼增加属性加成",
    ),
    // 技能系统类
    (
        "「技能连招」最多可以连几段？",
        "2段",
        "3段",
        "4段",
        "5段",
        2,
        "最多3段连招",
    ),
    (
        "连续签到30天可以获得什么特殊奖励？",
        "金币",
        "钻石",
        "稀有道具",
        "所有以上",
        4,
        "里程碑奖励包括金币、钻石和稀有道具",
    ),
];

/// 简单的确定性随机: 基于日期 + 用户ID的哈希
fn daily_seed(uid: &str) -> u64 {
    let now = chrono::Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let combined = format!("{}:{}", date_str, uid);
    // FNV-1a hash
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in combined.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// 题目解析结果类型
type QuizQuestion = (
    usize,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    u8,
    &'static str,
);

/// 从题库中选取今日的5道题（基于确定性随机）
fn pick_daily_questions(uid: &str) -> Vec<QuizQuestion> {
    let seed = daily_seed(uid);
    let pool_len = QUESTION_POOL.len();
    let mut indices: Vec<usize> = (0..pool_len).collect();

    // Fisher-Yates shuffle with deterministic seed
    let mut rng = seed;
    for i in (1..indices.len()).rev() {
        rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (rng >> 33) as usize % (i + 1);
        indices.swap(i, j);
    }

    indices
        .iter()
        .take(5)
        .map(|&i| {
            let q = QUESTION_POOL[i];
            (i, q.0, q.1, q.2, q.3, q.4, q.5, q.6)
        })
        .collect()
}

/// 获取今天的日期字符串
fn today_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// 开始答题 - 生成今日的5道题
pub fn cmd_start_quiz(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}", user_id);
    let today = today_str();

    // 检查是否已答过
    let last_date = db.global_get("quiz", format!("{}.last_date", user_id).as_str());
    let answered_key = format!("{}.{}.answered", user_id, today);
    let answered_str = db.global_get("quiz", &answered_key);
    let answered: i32 = answered_str.parse().unwrap_or(0);

    if last_date == today && answered >= 5 {
        return format!(
            "{}\n📝 今日答题已完成！\n\n💡 明天再来挑战新题目吧~\n📊 使用「我的答题记录」查看历史成绩",
            prefix
        );
    }

    let questions = pick_daily_questions(user_id);

    let mut out = format!("{}\n═══ 📝 每日答题挑战 ═══\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(format!("📅 日期: {}\n", today).as_str());
    out.push_str(format!("📊 已答: {}/5 题\n\n", answered).as_str());

    for (idx, (_qid, q, a, b, c, d, _correct, _explanation)) in questions.iter().enumerate() {
        let num = idx + 1;
        out.push_str(format!("❓ 第{}题: {}\n", num, q).as_str());
        out.push_str(format!("  A. {}\n", a).as_str());
        out.push_str(format!("  B. {}\n", b).as_str());
        out.push_str(format!("  C. {}\n", c).as_str());
        out.push_str(format!("  D. {}\n\n", d).as_str());
    }

    // 存储今日题目信息
    let question_ids: Vec<String> = questions.iter().map(|(i, _, _, _, _, _, _, _)| i.to_string()).collect();
    db.global_set(
        "quiz",
        format!("{}.{}.questions", user_id, today).as_str(),
        question_ids.join(",").as_str(),
    );
    db.global_set("quiz", format!("{}.last_date", user_id).as_str(), today.as_str());

    // 获取连续天数
    let streak: i32 = db
        .global_get("quiz", format!("{}.streak", user_id).as_str())
        .parse()
        .unwrap_or(0);
    if streak > 0 {
        out.push_str(format!("🔥 连续答题: {} 天\n", streak).as_str());
    }

    out.push_str("\n💡 使用「答题+答案」提交答案\n");
    out.push_str("📝 示例: 答题+1A 2B 3C 4D 5A\n");
    out.push_str("📝 或逐题: 答题+A\n");

    out
}

/// 答题 - 提交答案
pub fn cmd_answer_quiz(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}", user_id);
    let today = today_str();

    // 检查是否已开始
    let question_key = format!("{}.{}.questions", user_id, today);
    let question_ids_str = db.global_get("quiz", &question_key);
    if question_ids_str.is_empty() {
        return format!("{}\n❌ 今日还未开始答题！\n💡 使用「开始答题」开始今日挑战", prefix);
    }

    let question_ids: Vec<usize> = question_ids_str.split(',').filter_map(|s| s.parse().ok()).collect();

    if question_ids.len() != 5 {
        return format!("{}\n❌ 题目数据异常，请重新开始答题", prefix);
    }

    // 获取今日已答题数
    let answered_key = format!("{}.{}.answered", user_id, today);
    let answered: i32 = db.global_get("quiz", &answered_key).parse().unwrap_or(0);

    if answered >= 5 {
        return format!("{}\n📝 今日5题已全部作答！\n💡 使用「开始答题」查看今日题目", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n❌ 请提供答案！\n📝 示例: 答题+1A 2B 3C 4D 5A\n📝 或逐题: 答题+A",
            prefix
        );
    }

    // 解析答案
    let answers = parse_answers(args, 5 - answered);

    if answers.is_empty() {
        return format!(
            "{}\n❌ 答案格式错误！\n📝 示例: 答题+1A 2B 3C 4D 5A\n📝 或逐题: 答题+A",
            prefix
        );
    }

    let mut out = format!("{}\n═══ 📝 答题结果 ═══\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    let mut correct_count: i32 = 0;
    let mut new_answered = answered;

    for (_q_num, user_answer) in &answers {
        if new_answered >= 5 {
            break;
        }

        let idx = new_answered as usize;
        if idx >= question_ids.len() {
            break;
        }

        let qid = question_ids[idx];
        let (question, _a, _b, _c, _d, correct, explanation) = QUESTION_POOL[qid];

        let correct_letter = match correct {
            1 => "A",
            2 => "B",
            3 => "C",
            4 => "D",
            _ => "?",
        };

        let user_ans_upper = user_answer.to_uppercase();
        let is_correct = user_ans_upper == correct_letter;

        if is_correct {
            correct_count += 1;
            out.push_str(format!("✅ 第{}题: 正确！\n", idx + 1).as_str());
            out.push_str(format!("   📖 {}\n\n", explanation).as_str());
        } else {
            out.push_str(format!("❌ 第{}题: 错误！\n", idx + 1).as_str());
            out.push_str(format!("   你的答案: {} | 正确答案: {}\n", user_ans_upper, correct_letter).as_str());
            out.push_str(format!("   题目: {}\n", question).as_str());
            out.push_str(format!("   📖 {}\n\n", explanation).as_str());
        }

        new_answered += 1;
    }

    // 保存答题进度
    db.global_set("quiz", &answered_key, new_answered.to_string().as_str());

    // 保存今日正确数
    let today_correct_key = format!("{}.{}.correct", user_id, today);
    let prev_correct: i32 = db.global_get("quiz", &today_correct_key).parse().unwrap_or(0);
    db.global_set(
        "quiz",
        &today_correct_key,
        (prev_correct + correct_count).to_string().as_str(),
    );

    // 更新总答题数和总正确数
    let total_key = format!("{}.total_questions", user_id);
    let total_correct_key = format!("{}.total_correct", user_id);
    let total_q: i32 = db.global_get("quiz", &total_key).parse().unwrap_or(0);
    let total_c: i32 = db.global_get("quiz", &total_correct_key).parse().unwrap_or(0);
    db.global_set(
        "quiz",
        &total_key,
        (total_q + answers.len() as i32).to_string().as_str(),
    );
    db.global_set(
        "quiz",
        &total_correct_key,
        (total_c + correct_count).to_string().as_str(),
    );

    // 计算奖励
    let streak: i32 = db
        .global_get("quiz", format!("{}.streak", user_id).as_str())
        .parse()
        .unwrap_or(0);
    let streak_bonus = if streak >= 7 {
        2
    } else if streak >= 3 {
        1
    } else {
        0
    };
    let gold_per_correct = 200 + streak_bonus * 50;
    let diamond_per_correct = 2 + streak_bonus;
    let gold_reward = correct_count as i64 * gold_per_correct as i64;
    let diamond_reward = correct_count as i64 * diamond_per_correct as i64;

    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(format!("📊 本轮结果: {}/{} 正确\n", correct_count, answers.len()).as_str());
    out.push_str(format!("📊 今日进度: {}/5 题\n", new_answered).as_str());

    if gold_reward > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_reward);
        out.push_str(format!("💰 金币奖励: +{}\n", format_number(gold_reward)).as_str());
    }
    if diamond_reward > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, diamond_reward);
        out.push_str(format!("💎 钻石奖励: +{}\n", format_number(diamond_reward)).as_str());
    }

    if streak > 0 {
        out.push_str(format!("🔥 连续答题: {} 天 (奖励加成+{}%)\n", streak, streak_bonus * 25).as_str());
    }

    // 全部答完
    if new_answered >= 5 {
        let today_correct: i32 = db.global_get("quiz", &today_correct_key).parse().unwrap_or(0);

        // 更新连续天数
        let prev_streak_date = db.global_get("quiz", format!("{}.streak_date", user_id).as_str());
        let yesterday = (chrono::Utc::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();

        let new_streak = if prev_streak_date == yesterday || prev_streak_date == today {
            streak + 1
        } else {
            1 // streak reset or first quiz
        };

        db.global_set(
            "quiz",
            format!("{}.streak", user_id).as_str(),
            new_streak.to_string().as_str(),
        );
        db.global_set("quiz", format!("{}.streak_date", user_id).as_str(), today.as_str());

        out.push_str("\n🎉 今日答题全部完成！\n");
        out.push_str(format!("✅ 今日正确: {}/5\n", today_correct).as_str());
        out.push_str(format!("🔥 连续答题: {} 天\n", new_streak).as_str());

        // 全对奖励
        if today_correct == 5 {
            let bonus_gold = 1000;
            let bonus_diamond = 10;
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, bonus_gold);
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, bonus_diamond);
            out.push_str(
                format!(
                    "🏆 全对额外奖励: +{}金币 +{}钻石\n",
                    format_number(bonus_gold),
                    format_number(bonus_diamond)
                )
                .as_str(),
            );
        }
    } else {
        out.push_str(format!("\n💡 还剩 {} 题，使用「答题+答案」继续\n", 5 - new_answered).as_str());
    }

    out
}

/// 解析答案字符串
/// 支持: "1A 2B 3C 4D 5A" 或 "A" 或 "A B C D A"
fn parse_answers(args: &str, max_count: i32) -> Vec<(i32, String)> {
    let mut results = Vec::new();
    let args = args.trim();

    // 格式1: "1A 2B 3C 4D 5A" — 手动解析数字+字母模式
    let chars: Vec<char> = args.chars().collect();
    let mut ci = 0;
    while ci < chars.len() {
        if chars[ci].is_ascii_digit() {
            let num = chars[ci].to_digit(10).unwrap_or(0) as i32;
            // skip whitespace
            let mut j = ci + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() {
                let c: char = chars[j].to_ascii_uppercase();
                if c == 'A' || c == 'B' || c == 'C' || c == 'D' {
                    results.push((num, c.to_string()));
                    ci = j + 1;
                    continue;
                }
            }
        }
        ci += 1;
    }

    if !results.is_empty() {
        return results;
    }

    // 格式2: "A" 或 "A B C D A" (按顺序)
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() == 1 && parts[0].len() == 1 {
        let c = parts[0].to_uppercase();
        if c == "A" || c == "B" || c == "C" || c == "D" {
            results.push((1, c));
        }
    } else {
        for (i, part) in parts.iter().enumerate() {
            let c = part.to_uppercase();
            if c.len() == 1 && (c == "A" || c == "B" || c == "C" || c == "D") {
                results.push(((i as i32) + 1, c));
                if results.len() >= max_count as usize {
                    break;
                }
            }
        }
    }

    results
}

/// 格式化数字（千分位分隔）
fn format_number(n: i64) -> String {
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

/// 答题排行 - 全服答题排行
pub fn cmd_quiz_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}", user_id);

    let mut players: Vec<(String, i32, i32, i32)> = Vec::new();

    if let Ok(conn) = db.conn.lock() {
        if let Ok(mut stmt) = conn.prepare("SELECT DISTINCT uId FROM Basic_User") {
            if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
                for row in rows.flatten() {
                    let total_q: i32 = db
                        .global_get("quiz", format!("{}.total_questions", row).as_str())
                        .parse()
                        .unwrap_or(0);
                    if total_q > 0 {
                        let total_c: i32 = db
                            .global_get("quiz", format!("{}.total_correct", row).as_str())
                            .parse()
                            .unwrap_or(0);
                        let streak: i32 = db
                            .global_get("quiz", format!("{}.streak", row).as_str())
                            .parse()
                            .unwrap_or(0);
                        let name = format!("ID：{}", row);
                        players.push((name, total_c, total_q, streak));
                    }
                }
            }
        }
    }

    if players.is_empty() {
        return format!(
            "{}\n📊 答题排行榜\n━━━━━━━━━━━━━━━━━━━━\n暂无答题数据\n💡 使用「开始答题」成为第一位答题达人！",
            prefix
        );
    }

    players.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("{}\n═══ 📊 答题排行榜 ═══\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");

    let medals = ["🥇", "🥈", "🥉"];

    for (i, (name, correct, total, streak)) in players.iter().enumerate().take(10) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let accuracy = if *total > 0 {
            (*correct as f64 / *total as f64 * 100.0) as i32
        } else {
            0
        };
        out.push_str(
            format!(
                "{} {}. {} — ✅{}/{}题 ({}%) 🔥{}天\n",
                medal,
                i + 1,
                name,
                correct,
                total,
                accuracy,
                streak
            )
            .as_str(),
        );
    }

    // 当前用户排名
    let user_total_q: i32 = db
        .global_get("quiz", format!("{}.total_questions", user_id).as_str())
        .parse()
        .unwrap_or(0);
    if user_total_q > 0 {
        let user_total_c: i32 = db
            .global_get("quiz", format!("{}.total_correct", user_id).as_str())
            .parse()
            .unwrap_or(0);
        let user_streak: i32 = db
            .global_get("quiz", format!("{}.streak", user_id).as_str())
            .parse()
            .unwrap_or(0);
        let user_name = format!("ID：{}", user_id);
        let rank = players
            .iter()
            .position(|(n, _, _, _)| *n == user_name)
            .map(|p| p + 1)
            .unwrap_or(0);
        let accuracy = (user_total_c as f64 / user_total_q as f64 * 100.0) as i32;
        out.push_str("\n━━━━━━━━━━━━━━━━━━━━\n");
        out.push_str(format!("📍 你的排名: 第{}名\n", rank).as_str());
        out.push_str(
            format!(
                "📊 累计答对: {}/{}题 ({}%) 🔥{}天\n",
                user_total_c, user_total_q, accuracy, user_streak
            )
            .as_str(),
        );
    }

    out.push_str("\n💡 使用「开始答题」开始今日挑战！");
    out
}

/// 我的答题记录 - 查看个人答题统计
pub fn cmd_my_quiz(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}", user_id);
    let today = today_str();

    let total_q: i32 = db
        .global_get("quiz", format!("{}.total_questions", user_id).as_str())
        .parse()
        .unwrap_or(0);
    let total_c: i32 = db
        .global_get("quiz", format!("{}.total_correct", user_id).as_str())
        .parse()
        .unwrap_or(0);
    let streak: i32 = db
        .global_get("quiz", format!("{}.streak", user_id).as_str())
        .parse()
        .unwrap_or(0);
    let answered: i32 = db
        .global_get("quiz", format!("{}.{}.answered", user_id, today).as_str())
        .parse()
        .unwrap_or(0);
    let today_correct: i32 = db
        .global_get("quiz", format!("{}.{}.correct", user_id, today).as_str())
        .parse()
        .unwrap_or(0);

    let accuracy = if total_q > 0 {
        (total_c as f64 / total_q as f64 * 100.0) as i32
    } else {
        0
    };

    let mut out = format!("{}\n═══ 📊 我的答题记录 ═══\n", prefix);
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str(format!("📅 今日进度: {}/5 题", answered).as_str());
    if answered > 0 {
        out.push_str(format!(" (正确 {} 题)", today_correct).as_str());
    }
    out.push('\n');
    out.push_str(format!("📊 累计答题: {} 题\n", total_q).as_str());
    out.push_str(format!("✅ 累计正确: {} 题\n", total_c).as_str());
    out.push_str(format!("📈 正确率: {}%\n", accuracy).as_str());
    out.push_str(format!("🔥 连续答题: {} 天\n", streak).as_str());

    // 等级称号
    let title = match total_c {
        0..=9 => "📖 答题新手",
        10..=49 => "📚 知识学徒",
        50..=99 => "🎓 博学之士",
        100..=199 => "🏅 答题达人",
        200..=499 => "🏆 知识大师",
        _ => "👑 答题之王",
    };
    out.push_str(format!("称号: {}\n", title).as_str());

    // 奖励倍率提示
    if streak >= 7 {
        out.push_str("\n🔥 连续7天以上，金币奖励+100 钻石+2/题\n");
    } else if streak >= 3 {
        out.push_str("\n🔥 连续3天以上，金币奖励+50 钻石+1/题\n");
    }

    if answered < 5 {
        out.push_str("\n💡 使用「开始答题」开始今日挑战！");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_answers_single() {
        let result = parse_answers("A", 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], (1, "A".to_string()));
    }

    #[test]
    fn test_parse_answers_multiple() {
        let result = parse_answers("1A 2B 3C 4D 5A", 5);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], (1, "A".to_string()));
        assert_eq!(result[1], (2, "B".to_string()));
        assert_eq!(result[2], (3, "C".to_string()));
        assert_eq!(result[3], (4, "D".to_string()));
        assert_eq!(result[4], (5, "A".to_string()));
    }

    #[test]
    fn test_parse_answers_lowercase() {
        let result = parse_answers("1a 2b 3c", 5);
        assert_eq!(result.len(), 3);
        // parse_answers normalizes to uppercase
        assert_eq!(result[0], (1, "A".to_string()));
        assert_eq!(result[1], (2, "B".to_string()));
        assert_eq!(result[2], (3, "C".to_string()));
    }

    #[test]
    fn test_parse_answers_invalid() {
        let result = parse_answers("", 5);
        assert!(result.is_empty());

        let result = parse_answers("xyz", 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_daily_seed_deterministic() {
        let s1 = daily_seed("test_user");
        let s2 = daily_seed("test_user");
        assert_eq!(s1, s2);

        let s3 = daily_seed("other_user");
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_pick_daily_questions_count() {
        let questions = pick_daily_questions("test_user");
        assert_eq!(questions.len(), 5);
    }

    #[test]
    fn test_pick_daily_questions_unique() {
        let questions = pick_daily_questions("test_user");
        let mut indices: Vec<usize> = questions.iter().map(|(i, _, _, _, _, _, _, _)| *i).collect();
        indices.sort();
        indices.dedup();
        assert_eq!(indices.len(), 5, "Questions should be unique");
    }

    #[test]
    fn test_question_pool_coverage() {
        for (i, (q, a, b, c, d, correct, explanation)) in QUESTION_POOL.iter().enumerate() {
            assert!(!q.is_empty(), "Question {} should not be empty", i);
            assert!(!a.is_empty(), "Option A for question {} should not be empty", i);
            assert!(!b.is_empty(), "Option B for question {} should not be empty", i);
            assert!(!c.is_empty(), "Option C for question {} should not be empty", i);
            assert!(!d.is_empty(), "Option D for question {} should not be empty", i);
            assert!(
                *correct >= 1 && *correct <= 4,
                "Correct answer for question {} should be 1-4",
                i
            );
            assert!(
                !explanation.is_empty(),
                "Explanation for question {} should not be empty",
                i
            );
        }
    }
}

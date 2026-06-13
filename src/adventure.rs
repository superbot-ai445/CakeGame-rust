/// CakeGame 奇遇历险系统 (Serendipity Adventure System)
///
/// 经典RPG奇遇系统 — 随机触发的冒险事件，玩家做出选择影响结局
/// 12个奇遇事件覆盖5种类型(战斗⚔️/探索🗺️/社交🤝/机缘🍀/秘境🌀)
/// 每个奇遇2-3个选择，不同选择带来不同结局和奖励
/// 奇遇触发基于日期+用户ID哈希的确定性随机
/// 玩家可查看奇遇日志追踪所有已发现的奇遇
///
/// 指令: 查看奇遇, 选择奇遇, 奇遇日志, 奇遇排行, 奇遇信息
use crate::core::*;
use crate::db::Database;

/// 奇遇类型
const ADVENTURE_TYPES: &[(&str, &str)] = &[
    ("combat", "⚔️"),
    ("explore", "🗺️"),
    ("social", "🤝"),
    ("fate", "🍀"),
    ("realm", "🌀"),
];

/// 奇遇选择
struct Choice {
    text: &'static str,
    outcome: &'static str,
    reward_gold: i64,
    reward_diamond: i64,
    reward_exp: i64,
    reward_item: &'static str,
    success: bool,
}

/// 奇遇事件定义
struct Adventure {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    atype: &'static str,
    emoji: &'static str,
    min_level: i32,
    choices: &'static [Choice],
    rarity: i32,
}

/// 12个奇遇事件
const ADVENTURES: &[Adventure] = &[
    Adventure {
        id: "lost_traveler",
        name: "迷途旅人",
        description: "你在荒野中遇到一位受伤的旅人，他身旁散落着一些物品。远处传来狼嚎声……",
        atype: "social",
        emoji: "🧑‍🦯",
        min_level: 5,
        rarity: 1,
        choices: &[
            Choice {
                text: "伸出援手，帮助旅人",
                outcome: "旅人感激涕零，将随身的宝物赠予你：「恩人，这是我家传的宝物，请收下！」",
                reward_gold: 500,
                reward_diamond: 10,
                reward_exp: 200,
                reward_item: "旅人的谢礼",
                success: true,
            },
            Choice {
                text: "拿走散落的物品后离开",
                outcome: "你匆匆捡起物品离开，但总觉得背后有一道阴冷的目光……邪恶值+5",
                reward_gold: 800,
                reward_diamond: 0,
                reward_exp: 100,
                reward_item: "",
                success: false,
            },
            Choice {
                text: "假装没看见，绕路而行",
                outcome: "你选择避开这个麻烦。也许这样最安全，但你错过了一个改变命运的机会。",
                reward_gold: 0,
                reward_diamond: 0,
                reward_exp: 50,
                reward_item: "",
                success: true,
            },
        ],
    },
    Adventure {
        id: "ancient_chest",
        name: "古旧宝箱",
        description: "你在一个废弃的矿洞深处发现了一只被藤蔓缠绕的古旧宝箱，上面刻着奇异的符文。箱子发出微弱的光芒……",
        atype: "explore",
        emoji: "📦",
        min_level: 10,
        rarity: 1,
        choices: &[
            Choice {
                text: "小心地撬开宝箱",
                outcome: "宝箱缓缓打开，里面是一堆金币和一颗闪烁的宝石！符文消散时，你感到一股温暖的力量涌入体内。",
                reward_gold: 1000,
                reward_diamond: 20,
                reward_exp: 300,
                reward_item: "远古宝石",
                success: true,
            },
            Choice {
                text: "用武器砸开宝箱",
                outcome: "砰！宝箱碎裂，但里面的东西也损坏了大半。你只找回了一些残片。",
                reward_gold: 300,
                reward_diamond: 0,
                reward_exp: 150,
                reward_item: "破碎的符文",
                success: true,
            },
        ],
    },
    Adventure {
        id: "shadow_duel",
        name: "暗影决斗",
        description: "夜幕降临，一个黑衣人突然出现在你面前。他递给你一把匕首，嘴角浮现一抹冷笑：「来，证明你的实力。」",
        atype: "combat",
        emoji: "🗡️",
        min_level: 15,
        rarity: 2,
        choices: &[
            Choice {
                text: "接受挑战，全力一战",
                outcome: "一场惊心动魄的决斗后，你以微弱优势获胜！黑衣人露出赞许的目光：「不错，你有资格拥有这个。」",
                reward_gold: 2000,
                reward_diamond: 50,
                reward_exp: 800,
                reward_item: "暗影勋章",
                success: true,
            },
            Choice {
                text: "婉言拒绝，转身离开",
                outcome: "黑衣人冷哼一声消失在黑暗中。你感觉错过了什么重要的东西……",
                reward_gold: 0,
                reward_diamond: 0,
                reward_exp: 100,
                reward_item: "",
                success: true,
            },
        ],
    },
    Adventure {
        id: "fairy_spring",
        name: "精灵之泉",
        description: "在密林深处，你发现了一汪清澈见底的泉水。泉水周围开满了从未见过的花朵，空气中弥漫着甜美的香气。一只精灵正在泉边梳洗……",
        atype: "realm",
        emoji: "🧚",
        min_level: 20,
        rarity: 2,
        choices: &[
            Choice {
                text: "恭敬地向精灵问好",
                outcome: "精灵微笑道：「很久没有客人了。」她用泉水在你额头画了一个符号，你感到体内涌动着新的力量。",
                reward_gold: 1500,
                reward_diamond: 30,
                reward_exp: 600,
                reward_item: "精灵的祝福",
                success: true,
            },
            Choice {
                text: "悄悄装一瓶泉水就走",
                outcome: "精灵察觉了你的行为，泉水瞬间化为普通的溪水。你沮丧地离开了。",
                reward_gold: 200,
                reward_diamond: 0,
                reward_exp: 80,
                reward_item: "普通溪水",
                success: false,
            },
            Choice {
                text: "在泉边修炼一段时间",
                outcome: "你闭目修炼，感受着泉水散发的灵气。数个时辰后，你感觉自己的修为有所精进。",
                reward_gold: 0,
                reward_diamond: 0,
                reward_exp: 1000,
                reward_item: "",
                success: true,
            },
        ],
    },
    Adventure {
        id: "merchant_caravan",
        name: "神秘商队",
        description: "一支来自远方的商队路过此地，领头的是一位白胡子老者。他的车上摆满了各地珍奇物品，但他的眼神中似乎隐藏着什么秘密……",
        atype: "social",
        emoji: "🐪",
        min_level: 8,
        rarity: 1,
        choices: &[
            Choice {
                text: "与老者攀谈，询问旅途见闻",
                outcome: "老者讲述了一个关于失落宝藏的传说，并赠你一张残破的地图碎片作为信物。",
                reward_gold: 300,
                reward_diamond: 5,
                reward_exp: 250,
                reward_item: "藏宝图碎片",
                success: true,
            },
            Choice {
                text: "直接购买商队的特产",
                outcome: "你买到了一些稀有的货物，虽然价格不菲，但物有所值。",
                reward_gold: -500,
                reward_diamond: 0,
                reward_exp: 200,
                reward_item: "异域香料",
                success: true,
            },
        ],
    },
    Adventure {
        id: "thunder_peak",
        name: "雷鸣峰巅",
        description: "你攀上了一座终年雷云密布的山峰。峰顶有一块被雷击了无数次的黑色巨石，石缝中闪烁着电弧。传说中，这里曾是雷神的修炼之地……",
        atype: "realm",
        emoji: "⚡",
        min_level: 30,
        rarity: 3,
        choices: &[
            Choice {
                text: "将手放在巨石上，接受雷电洗礼",
                outcome: "雷电涌入你的身体，你几乎无法承受！但最终，你挺了过来，体内多了一股狂暴的力量。",
                reward_gold: 3000,
                reward_diamond: 80,
                reward_exp: 1500,
                reward_item: "雷神碎片",
                success: true,
            },
            Choice {
                text: "在巨石旁打坐冥想",
                outcome: "你感受着雷电的气息，领悟了一些天地法则。虽然没有直接获得力量，但你的精神力得到了提升。",
                reward_gold: 1000,
                reward_diamond: 20,
                reward_exp: 800,
                reward_item: "雷霆冥想术",
                success: true,
            },
            Choice {
                text: "这太危险了，立刻下山",
                outcome: "明智的选择。你安全地离开了山峰，但心中不免有些遗憾。",
                reward_gold: 0,
                reward_diamond: 0,
                reward_exp: 50,
                reward_item: "",
                success: true,
            },
        ],
    },
    Adventure {
        id: "ghost_ship",
        name: "幽灵船",
        description: "海面上飘来一艘破旧的帆船，船上没有活人的气息。月光下，你隐约看到船舱内闪烁着幽幽的蓝光……",
        atype: "explore",
        emoji: "🚢",
        min_level: 25,
        rarity: 2,
        choices: &[
            Choice {
                text: "登上幽灵船探索",
                outcome: "你在船舱中发现了一个被封印的宝箱！打开后，里面是海盗遗留的财宝和一本古老的航海日志。",
                reward_gold: 2500,
                reward_diamond: 40,
                reward_exp: 700,
                reward_item: "海盗藏宝图",
                success: true,
            },
            Choice {
                text: "向幽灵船射火箭",
                outcome: "火焰吞噬了幽灵船，在燃烧的残骸中你捞到了一些融化的金属和宝石碎片。",
                reward_gold: 800,
                reward_diamond: 15,
                reward_exp: 400,
                reward_item: "幽灵船残片",
                success: true,
            },
        ],
    },
    Adventure {
        id: "fortune_teller",
        name: "命运占卜师",
        description: "一位神秘的占卜师出现在路边，她的眼睛被黑色丝带蒙住，但似乎能看见一切。她向你伸出手：「让我为你看看命运的走向……」",
        atype: "fate",
        emoji: "🔮",
        min_level: 10,
        rarity: 1,
        choices: &[
            Choice {
                text: "将手放在她的掌心",
                outcome: "占卜师微微一笑：「你的命运线异常明亮……」她赠予你一枚护身符，据说能带来好运。",
                reward_gold: 200,
                reward_diamond: 15,
                reward_exp: 150,
                reward_item: "命运护身符",
                success: true,
            },
            Choice {
                text: "摇头拒绝，继续赶路",
                outcome: "占卜师在你身后轻声说：「命运不会等待犹豫的人……」你没有回头。",
                reward_gold: 0,
                reward_diamond: 0,
                reward_exp: 30,
                reward_item: "",
                success: true,
            },
        ],
    },
    Adventure {
        id: "dragon_cave",
        name: "龙穴探秘",
        description: "你在山脉深处发现了一个巨大的洞穴，洞口散落着巨大的鳞片和焦黑的骨骼。一股热浪从洞内涌出……这里曾是龙的巢穴！",
        atype: "combat",
        emoji: "🐉",
        min_level: 40,
        rarity: 3,
        choices: &[
            Choice {
                text: "深入龙穴，寻找传说中的龙之宝藏",
                outcome: "你小心翼翼地深入洞穴，果然在最深处找到了一堆金币和龙牙！虽然龙已不在，但洞穴中残留的龙威仍让人窒息。",
                reward_gold: 5000,
                reward_diamond: 100,
                reward_exp: 2000,
                reward_item: "远古龙牙",
                success: true,
            },
            Choice {
                text: "在洞口附近搜寻有价值的东西",
                outcome: "你在外围找到了几片完整的龙鳞和一些珍贵的矿石。收获不错，而且安全。",
                reward_gold: 2000,
                reward_diamond: 30,
                reward_exp: 800,
                reward_item: "龙鳞碎片",
                success: true,
            },
            Choice {
                text: "太危险了，立刻离开",
                outcome: "你明智地选择了撤退。不是所有冒险都值得用生命去赌。",
                reward_gold: 0,
                reward_diamond: 0,
                reward_exp: 100,
                reward_item: "",
                success: true,
            },
        ],
    },
    Adventure {
        id: "mysterious_garden",
        name: "神秘花园",
        description: "你无意间走进了一片与世隔绝的花园，四季的花朵同时盛开，蝴蝶翩翩起舞。花园中央有一棵巨大的银色古树，树上挂满了祈愿的红绸……",
        atype: "fate",
        emoji: "🌸",
        min_level: 12,
        rarity: 1,
        choices: &[
            Choice {
                text: "在银色古树下祈愿",
                outcome: "古树沙沙作响，一片银色树叶飘落到你手中。你感到一股宁静的力量注入灵魂。",
                reward_gold: 500,
                reward_diamond: 25,
                reward_exp: 300,
                reward_item: "银叶祈愿符",
                success: true,
            },
            Choice {
                text: "采摘一些奇花异草",
                outcome: "你小心翼翼地采集了几株药草。虽然花园的主人似乎并不介意，但你总觉得有人在看着你。",
                reward_gold: 300,
                reward_diamond: 0,
                reward_exp: 200,
                reward_item: "四时花",
                success: true,
            },
        ],
    },
    Adventure {
        id: "arena_ghost",
        name: "斗技场幽魂",
        description: "午夜时分，你路过一座废弃的斗技场。月光下，一个身穿古代铠甲的幽魂站在场地中央，向你发出挑战：「来，与我一战！让我重温战斗的快感！」",
        atype: "combat",
        emoji: "👻",
        min_level: 35,
        rarity: 2,
        choices: &[
            Choice {
                text: "接受幽魂战士的挑战",
                outcome: "你与幽魂展开了一场史诗般的对决！最终，你的攻击穿透了幽魂的身体。他化为光点消散：「谢谢你，我终于可以安息了……」",
                reward_gold: 3000,
                reward_diamond: 60,
                reward_exp: 1200,
                reward_item: "幽魂战甲碎片",
                success: true,
            },
            Choice {
                text: "请求幽魂传授战斗技巧",
                outcome: "幽魂沉思片刻，开始向你展示古老的剑术。虽然只学到了皮毛，但也受益匪浅。",
                reward_gold: 500,
                reward_diamond: 0,
                reward_exp: 900,
                reward_item: "古剑术残卷",
                success: true,
            },
        ],
    },
    Adventure {
        id: "fallen_star",
        name: "坠落之星",
        description: "一颗流星划过夜空，坠落在不远处的山谷中。你赶到现场，发现一块散发蓝光的陨石嵌在地面。陨石的温度出乎意料地温和，表面刻满了未知的文字……",
        atype: "fate",
        emoji: "⭐",
        min_level: 20,
        rarity: 3,
        choices: &[
            Choice {
                text: "尝试解读陨石上的文字",
                outcome: "你凝视文字，脑海中浮现出一幅星空图。一段古老的智慧融入你的意识——你对世界有了更深的理解。",
                reward_gold: 2000,
                reward_diamond: 50,
                reward_exp: 1500,
                reward_item: "星陨结晶",
                success: true,
            },
            Choice {
                text: "将陨石搬回家研究",
                outcome: "陨石比想象中沉重得多，你费了九牛二虎之力才搬回去。虽然有些残损，但仍是有价值的研究材料。",
                reward_gold: 1000,
                reward_diamond: 20,
                reward_exp: 600,
                reward_item: "陨石碎片",
                success: true,
            },
            Choice {
                text: "在陨石旁休息一晚，第二天再处理",
                outcome: "你在星光下安然入睡。第二天醒来时，陨石已经消失了，只留下一个浅坑和淡淡的蓝光印记。",
                reward_gold: 0,
                reward_diamond: 10,
                reward_exp: 400,
                reward_item: "星尘印记",
                success: true,
            },
        ],
    },
];

/// djb2 哈希函数
fn djb2_hash(s: &str) -> usize {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash as usize
}

/// 解析 Global 表中存储的奇遇状态
fn parse_adventure_log(raw: &str) -> Vec<(String, usize)> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let Ok(idx) = parts[1].parse::<usize>() {
                    return Some((parts[0].to_string(), idx));
                }
            }
            None
        })
        .collect()
}

/// 序列化奇遇记录
fn serialize_adventure_log(log: &[(String, usize)]) -> String {
    log.iter()
        .map(|(id, idx)| format!("{}:{}", id, idx))
        .collect::<Vec<_>>()
        .join("|")
}

/// 稀有度名称和图标
fn rarity_info(rarity: i32) -> (&'static str, &'static str) {
    match rarity {
        1 => ("普通", "🟢"),
        2 => ("稀有", "🔵"),
        3 => ("传说", "🟣"),
        _ => ("未知", "⚪"),
    }
}

/// 奇遇类型名
fn type_name(t: &str) -> &'static str {
    match t {
        "combat" => "战斗奇遇",
        "explore" => "探索奇遇",
        "social" => "社交奇遇",
        "fate" => "机缘奇遇",
        "realm" => "秘境奇遇",
        _ => "未知",
    }
}

/// 类型图标
fn type_emoji(t: &str) -> &str {
    ADVENTURE_TYPES
        .iter()
        .find(|(k, _)| *k == t)
        .map(|(_, e)| *e)
        .unwrap_or("❓")
}

fn get_user_level(db: &Database, user_id: &str) -> i32 {
    db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1)
}

fn format_gold(n: i64) -> String {
    if n < 0 {
        return format!("-{}", format_gold(-n));
    }
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

/// 获取今日可用的奇遇列表
fn get_today_adventures(_user_id: &str, level: i32, completed_ids: &[&str]) -> Vec<&'static Adventure> {
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let mut result = Vec::new();

    for slot in 0..3 {
        let seed = djb2_hash(&format!("slot_{}_{}", slot, today));
        let candidates: Vec<&Adventure> = ADVENTURES
            .iter()
            .filter(|a| a.min_level <= level && !completed_ids.contains(&a.id))
            .collect();
        if !candidates.is_empty() {
            result.push(candidates[seed % candidates.len()]);
        }
    }

    result
}

/// 查看今日奇遇
pub fn cmd_view_adventure(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let level = get_user_level(db, user_id);
    let completed_raw = db.global_get(&format!("adventure_log_{}", user_id), "entries");
    let completed = parse_adventure_log(&completed_raw);
    let completed_ids: Vec<&str> = completed.iter().map(|(id, _)| id.as_str()).collect();
    let today_adventures = get_today_adventures(user_id, level, &completed_ids);

    if today_adventures.is_empty() {
        return "📭 今日没有可用的奇遇事件。\n\n💡 提示：提升等级可解锁更多奇遇，完成的奇遇每日重置。".to_string();
    }

    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    let mut out = String::from("✨ === 今日奇遇 === ✨\n\n");
    out.push_str(&format!("📅 {} | 🎯 已完成 {} 个奇遇\n\n", today, completed.len()));

    for (i, adv) in today_adventures.iter().enumerate() {
        let (rarity_name, rarity_icon) = rarity_info(adv.rarity);
        let t_emoji = type_emoji(adv.atype);
        out.push_str(&format!(
            "{}. {} {} [{}{}] Lv.{} | {}\n   {}\n   ┗ 选项: {} 种\n\n",
            i + 1,
            adv.emoji,
            adv.name,
            rarity_icon,
            rarity_name,
            adv.min_level,
            t_emoji,
            adv.description,
            adv.choices.len(),
        ));
    }

    out.push_str("📝 输入「选择奇遇+编号+选项」做出选择\n");
    out.push_str("   例: 选择奇遇+1+1\n\n");
    out.push_str("📖 输入「奇遇日志」查看历史记录\n");
    out.push_str("🏆 输入「奇遇排行」查看全服排名\n");
    out.push_str("ℹ️  输入「奇遇信息」查看所有奇遇事件");

    out
}

/// 选择奇遇
pub fn cmd_choose_adventure(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let level = get_user_level(db, user_id);
    let completed_raw = db.global_get(&format!("adventure_log_{}", user_id), "entries");
    let completed = parse_adventure_log(&completed_raw);
    let completed_ids: Vec<&str> = completed.iter().map(|(id, _)| id.as_str()).collect();

    // 解析参数
    let parts: Vec<&str> = args.split(['+', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return "❌ 格式: 选择奇遇+编号+选项\n   例: 选择奇遇+1+1".to_string();
    }

    let adventure_num: usize = match parts[0].parse::<usize>() {
        Ok(n) if n >= 1 => n - 1,
        _ => return "❌ 无效的奇遇编号，请输入 1-3".to_string(),
    };
    let choice_num: usize = match parts[1].parse::<usize>() {
        Ok(n) if n >= 1 => n - 1,
        _ => return "❌ 无效的选项编号".to_string(),
    };

    let today_adventures = get_today_adventures(user_id, level, &completed_ids);

    if adventure_num >= today_adventures.len() {
        return format!("❌ 无效的奇遇编号，今日只有 {} 个奇遇", today_adventures.len());
    }

    let adv = today_adventures[adventure_num];
    if choice_num >= adv.choices.len() {
        return format!("❌ 无效的选项，「{}」只有 {} 个选项", adv.name, adv.choices.len());
    }

    if completed_ids.contains(&adv.id) {
        return format!("❌ 你已经完成了「{}」奇遇，今日无法重复选择", adv.name);
    }

    let choice = &adv.choices[choice_num];

    // 发放奖励
    let mut out = String::new();
    out.push_str(&format!("✨ === {} 奇遇结局 === ✨\n\n", adv.name));
    out.push_str(&format!("📖 {}\n\n", choice.outcome));

    if choice.reward_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "add", choice.reward_gold);
        out.push_str(&format!("💰 金币 +{}\n", format_gold(choice.reward_gold)));
    } else if choice.reward_gold < 0 {
        let deduct = (-choice.reward_gold).min(10000);
        db.modify_currency(user_id, CURRENCY_GOLD, "sub", deduct);
        out.push_str(&format!("💰 金币 -{}\n", format_gold(deduct)));
    }

    if choice.reward_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "add", choice.reward_diamond);
        out.push_str(&format!("💎 钻石 +{}\n", choice.reward_diamond));
    }
    if choice.reward_exp > 0 {
        let cur_exp: i32 = db.read_basic(user_id, ITEM_EXP).parse().unwrap_or(0);
        db.write_basic_int(user_id, ITEM_EXP, cur_exp + choice.reward_exp as i32);
        out.push_str(&format!("⭐ 经验 +{}\n", choice.reward_exp));
    }
    if !choice.reward_item.is_empty() {
        db.knapsack_add(user_id, choice.reward_item, 1);
        out.push_str(&format!("🎁 获得物品: {}\n", choice.reward_item));
    }

    // 记录用户到玩家列表（用于排行）
    let players_raw = db.global_get("adventure_system", "players");
    if !players_raw.split(',').any(|s| s == user_id) {
        let new_players = if players_raw.is_empty() {
            user_id.to_string()
        } else {
            format!("{},{}", players_raw, user_id)
        };
        db.global_set("adventure_system", "players", &new_players);
    }

    // 记录到 Global 表
    let mut new_completed = completed.clone();
    new_completed.push((adv.id.to_string(), choice_num));
    db.global_set(
        &format!("adventure_log_{}", user_id),
        "entries",
        &serialize_adventure_log(&new_completed),
    );

    let total_raw = db.global_get(&format!("adventure_log_{}", user_id), "total");
    let total: i64 = total_raw.parse().unwrap_or(0);
    db.global_set(&format!("adventure_log_{}", user_id), "total", &(total + 1).to_string());

    if !choice.success {
        out.push_str("\n⚠️ 这个选择似乎并不是最好的……下次三思而后行？\n");
    } else {
        out.push_str("\n✅ 奇遇完成！\n");
    }

    out
}

/// 奇遇日志
pub fn cmd_adventure_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let completed_raw = db.global_get(&format!("adventure_log_{}", user_id), "entries");
    let completed = parse_adventure_log(&completed_raw);
    let total_raw = db.global_get(&format!("adventure_log_{}", user_id), "total");
    let total: i64 = total_raw.parse().unwrap_or(0);

    let mut out = String::from("📖 === 奇遇日志 === 📖\n\n");
    out.push_str(&format!("🎯 总奇遇次数: {}\n", total));
    out.push_str(&format!("📜 已记录: {} 条\n\n", completed.len()));

    if completed.is_empty() {
        out.push_str("📭 还没有奇遇记录。\n");
        out.push_str("💡 输入「查看奇遇」查看今日可用的奇遇事件。");
        return out;
    }

    for (i, (adv_id, choice_idx)) in completed.iter().rev().enumerate().take(15) {
        if let Some(adv) = ADVENTURES.iter().find(|a| a.id == adv_id.as_str()) {
            let (rarity_name, _rarity_icon) = rarity_info(adv.rarity);
            if let Some(choice) = adv.choices.get(*choice_idx) {
                let status = if choice.success { "✅" } else { "❌" };
                let reward_str = if choice.reward_item.is_empty() {
                    format!(
                        "{}金币/{}钻石/{}经验",
                        format_gold(choice.reward_gold.max(0)),
                        choice.reward_diamond,
                        choice.reward_exp
                    )
                } else {
                    choice.reward_item.to_string()
                };
                out.push_str(&format!(
                    "{}. {} {} {}[{}] — {}\n   ┗ {}\n\n",
                    i + 1,
                    adv.emoji,
                    adv.name,
                    status,
                    rarity_name,
                    choice.text,
                    reward_str,
                ));
            }
        }
    }

    // 统计成功率
    let success_count = completed
        .iter()
        .filter(|(id, idx)| {
            ADVENTURES
                .iter()
                .find(|a| a.id == id.as_str())
                .and_then(|a| a.choices.get(*idx))
                .map(|c| c.success)
                .unwrap_or(false)
        })
        .count();

    let success_rate = if completed.is_empty() {
        0
    } else {
        success_count * 100 / completed.len()
    };
    out.push_str(&format!(
        "📊 成功率: {}% ({}/{})",
        success_rate,
        success_count,
        completed.len()
    ));

    out
}

/// 奇遇排行
pub fn cmd_adventure_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // 从 adventure_players 全局键获取所有参与过奇遇的用户ID
    let players_raw = db.global_get("adventure_system", "players");
    let player_ids: Vec<&str> = players_raw.split(',').filter(|s| !s.is_empty()).collect();
    let mut rankings: Vec<(String, i64)> = Vec::new();

    for uid in &player_ids {
        let total_raw = db.global_get(&format!("adventure_log_{}", uid), "total");
        let total: i64 = total_raw.parse().unwrap_or(0);
        if total > 0 {
            let nickname = db.read_basic(uid, ITEM_NAME);
            rankings.push((format!("{}({})", nickname, uid), total));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = String::from("🏆 === 奇遇排行 === 🏆\n\n");

    if rankings.is_empty() {
        out.push_str("📭 暂无奇遇记录。");
        return out;
    }

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, count)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let bar = "█".repeat((*count as usize).min(20));
        out.push_str(&format!("{} {}. {} — {} 次\n    {}\n", medal, i + 1, name, count, bar));
    }

    let my_total: i64 = db
        .global_get(&format!("adventure_log_{}", user_id), "total")
        .parse()
        .unwrap_or(0);
    if my_total > 0 {
        let my_rank = rankings
            .iter()
            .position(|(name, _)| name.contains(user_id))
            .unwrap_or(rankings.len());
        out.push_str(&format!("\n📍 你的排名: 第 {} 名 ({} 次奇遇)", my_rank + 1, my_total));
    } else {
        out.push_str("\n📍 你还没有奇遇记录，输入「查看奇遇」开始冒险吧！");
    }

    out
}

/// 奇遇信息 — 查看所有奇遇或搜索特定奇遇
pub fn cmd_adventure_info(_db: &Database, _user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let search = args.trim();
    if search.is_empty() {
        let total = ADVENTURE_TYPES
            .iter()
            .map(|(k, e)| {
                let count = ADVENTURES.iter().filter(|a| a.atype == *k).count();
                format!("{} {} — {} 个奇遇", e, type_name(k), count)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let mut out = String::from("✨ === 奇遇系统 === ✨\n\n");
        out.push_str("📜 奇遇类型:\n");
        out.push_str(&total);
        out.push_str(&format!("\n\n🎯 总计: {} 个奇遇事件\n", ADVENTURES.len()));
        out.push_str(&format!(
            "📊 普通🟢: {} | 稀有🔵: {} | 传说🟣: {}\n\n",
            ADVENTURES.iter().filter(|a| a.rarity == 1).count(),
            ADVENTURES.iter().filter(|a| a.rarity == 2).count(),
            ADVENTURES.iter().filter(|a| a.rarity == 3).count(),
        ));
        out.push_str("💡 输入「查看奇遇」查看今日可用的奇遇\n");
        out.push_str("📝 输入「选择奇遇+编号+选项」做出选择\n");
        out.push_str("📖 输入「奇遇日志」查看历史记录\n");
        out.push_str("🏆 输入「奇遇排行」查看全服排名");

        return out;
    }

    // 搜索
    let found: Vec<&Adventure> = ADVENTURES
        .iter()
        .filter(|a| a.name.contains(search) || a.id.contains(search) || a.description.contains(search))
        .collect();

    if found.is_empty() {
        return format!("❌ 未找到包含「{}」的奇遇", search);
    }

    let mut out = String::new();
    for adv in found.iter().take(3) {
        let (rarity_name, rarity_icon) = rarity_info(adv.rarity);
        let t_emoji = type_emoji(adv.atype);
        out.push_str(&format!(
            "✨ {} {} [{}{}] | {} {}\n{}\n最低等级: Lv.{}\n\n选择:\n",
            adv.emoji,
            adv.name,
            rarity_icon,
            rarity_name,
            t_emoji,
            type_name(adv.atype),
            adv.description,
            adv.min_level,
        ));
        for (i, choice) in adv.choices.iter().enumerate() {
            let status = if choice.success { "✅" } else { "⚠️" };
            out.push_str(&format!("  {}. {} {} ", i + 1, status, choice.text));
            let mut rewards = Vec::new();
            if choice.reward_gold > 0 {
                rewards.push(format!("💰{}", format_gold(choice.reward_gold)));
            }
            if choice.reward_diamond > 0 {
                rewards.push(format!("💎{}", choice.reward_diamond));
            }
            if choice.reward_exp > 0 {
                rewards.push(format!("⭐{}", choice.reward_exp));
            }
            if !choice.reward_item.is_empty() {
                rewards.push(format!("🎁{}", choice.reward_item));
            }
            if !rewards.is_empty() {
                out.push_str(&format!("→ {}", rewards.join(" ")));
            }
            out.push('\n');
        }
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adventures_count() {
        assert_eq!(ADVENTURES.len(), 12);
    }

    #[test]
    fn test_adventure_ids_unique() {
        let ids: Vec<&str> = ADVENTURES.iter().map(|a| a.id).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len());
    }

    #[test]
    fn test_adventure_names_unique() {
        let names: Vec<&str> = ADVENTURES.iter().map(|a| a.name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names.len(), sorted.len());
    }

    #[test]
    fn test_adventure_types_coverage() {
        let defined_types: Vec<&str> = ADVENTURE_TYPES.iter().map(|(k, _)| *k).collect();
        for adv in ADVENTURES.iter() {
            assert!(
                defined_types.contains(&adv.atype),
                "Adventure '{}' has unknown type '{}'",
                adv.name,
                adv.atype
            );
        }
    }

    #[test]
    fn test_adventure_choices_non_empty() {
        for adv in ADVENTURES.iter() {
            assert!(
                adv.choices.len() >= 2,
                "Adventure '{}' must have at least 2 choices",
                adv.name
            );
        }
    }

    #[test]
    fn test_adventure_emojis_non_empty() {
        for adv in ADVENTURES.iter() {
            assert!(!adv.emoji.is_empty());
        }
    }

    #[test]
    fn test_adventure_min_level_positive() {
        for adv in ADVENTURES.iter() {
            assert!(adv.min_level > 0);
        }
    }

    #[test]
    fn test_adventure_rarity_range() {
        for adv in ADVENTURES.iter() {
            assert!(
                (1..=3).contains(&adv.rarity),
                "Adventure '{}' rarity must be 1-3",
                adv.name
            );
        }
    }

    #[test]
    fn test_adventure_rewards_non_negative() {
        for adv in ADVENTURES.iter() {
            for choice in adv.choices.iter() {
                assert!(
                    choice.reward_diamond >= 0,
                    "Adventure '{}': diamond must be non-negative",
                    adv.name
                );
                assert!(
                    choice.reward_exp >= 0,
                    "Adventure '{}': exp must be non-negative",
                    adv.name
                );
            }
        }
    }

    #[test]
    fn test_adventure_type_emoji_coverage() {
        for (t, emoji) in ADVENTURE_TYPES {
            assert!(!emoji.is_empty());
            let count = ADVENTURES.iter().filter(|a| a.atype == *t).count();
            assert!(count > 0, "Type '{}' has no adventures", t);
        }
    }

    #[test]
    fn test_serialize_parse_roundtrip() {
        let log = vec![
            ("lost_traveler".to_string(), 0),
            ("ancient_chest".to_string(), 1),
            ("shadow_duel".to_string(), 0),
        ];
        let serialized = serialize_adventure_log(&log);
        let parsed = parse_adventure_log(&serialized);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], ("lost_traveler".to_string(), 0));
        assert_eq!(parsed[1], ("ancient_chest".to_string(), 1));
        assert_eq!(parsed[2], ("shadow_duel".to_string(), 0));
    }

    #[test]
    fn test_parse_empty_log() {
        let parsed = parse_adventure_log("");
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_djb2_hash_deterministic() {
        let h1 = djb2_hash("test_user_20260612");
        let h2 = djb2_hash("test_user_20260612");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_djb2_hash_different_inputs() {
        let h1 = djb2_hash("user_a");
        let h2 = djb2_hash("user_b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_rarity_info() {
        assert_eq!(rarity_info(1), ("普通", "🟢"));
        assert_eq!(rarity_info(2), ("稀有", "🔵"));
        assert_eq!(rarity_info(3), ("传说", "🟣"));
    }

    #[test]
    fn test_type_name() {
        assert_eq!(type_name("combat"), "战斗奇遇");
        assert_eq!(type_name("explore"), "探索奇遇");
        assert_eq!(type_name("social"), "社交奇遇");
        assert_eq!(type_name("fate"), "机缘奇遇");
        assert_eq!(type_name("realm"), "秘境奇遇");
        assert_eq!(type_name("unknown"), "未知");
    }

    #[test]
    fn test_format_gold() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(999), "999");
        assert_eq!(format_gold(1000), "1,000");
        assert_eq!(format_gold(1234567), "1,234,567");
        assert_eq!(format_gold(-5000), "-5,000");
    }

    #[test]
    fn test_serialize_empty_log() {
        let log: Vec<(String, usize)> = Vec::new();
        assert_eq!(serialize_adventure_log(&log), "");
    }

    #[test]
    fn test_adventure_descriptions_not_empty() {
        for adv in ADVENTURES.iter() {
            assert!(!adv.description.is_empty());
        }
    }

    #[test]
    fn test_choice_texts_not_empty() {
        for adv in ADVENTURES.iter() {
            for choice in adv.choices.iter() {
                assert!(!choice.text.is_empty());
                assert!(!choice.outcome.is_empty());
            }
        }
    }

    #[test]
    fn test_adventure_type_counts() {
        let combat_count = ADVENTURES.iter().filter(|a| a.atype == "combat").count();
        let fate_count = ADVENTURES.iter().filter(|a| a.atype == "fate").count();
        assert!(combat_count >= 2, "Must have at least 2 combat adventures");
        assert!(fate_count >= 2, "Must have at least 2 fate adventures");
    }
}

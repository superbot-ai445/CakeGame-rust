/// 命格系统 (Destiny System)
///
/// 玩家通过完成特定成就和事件，解锁命格碎片并激活命运特质。
/// 每个命格提供独特的被动加成，高级命格需要前置命格解锁。
///
/// 命格等级: 凡命 → 安命 → 福命 → 贵命 → 天命 → 帝命 → 神命
/// 命格类型: 8大命运方向 (战/法/商/幸/缘/隐/慧/命)
use crate::db::Database;
use rusqlite::params;

/// 命格等级
const DESTINY_TIERS: &[DestinyTier] = &[
    DestinyTier {
        name: "凡命",
        level: 1,
        emoji: "⚪",
        color: "白色",
        required_fragments: 0,
        bonus_pct: 0,
    },
    DestinyTier {
        name: "安命",
        level: 2,
        emoji: "🟢",
        color: "绿色",
        required_fragments: 5,
        bonus_pct: 5,
    },
    DestinyTier {
        name: "福命",
        level: 3,
        emoji: "🔵",
        color: "蓝色",
        required_fragments: 15,
        bonus_pct: 10,
    },
    DestinyTier {
        name: "贵命",
        level: 4,
        emoji: "🟣",
        color: "紫色",
        required_fragments: 30,
        bonus_pct: 18,
    },
    DestinyTier {
        name: "天命",
        level: 5,
        emoji: "🟠",
        color: "橙色",
        required_fragments: 50,
        bonus_pct: 28,
    },
    DestinyTier {
        name: "帝命",
        level: 6,
        emoji: "🔴",
        color: "红色",
        required_fragments: 80,
        bonus_pct: 40,
    },
    DestinyTier {
        name: "神命",
        level: 7,
        emoji: "🟡",
        color: "金色",
        required_fragments: 120,
        bonus_pct: 55,
    },
];

struct DestinyTier {
    name: &'static str,
    level: i32,
    #[allow(dead_code)]
    emoji: &'static str,
    #[allow(dead_code)]
    color: &'static str,
    required_fragments: i32,
    #[allow(dead_code)]
    bonus_pct: i32,
}

/// 命格特质定义
struct DestinyTrait {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    destiny_type: &'static str,
    tier_level: i32,
    hp_bonus: i32,
    ad_bonus: i32,
    ap_bonus: i32,
    def_bonus: i32,
    mdef_bonus: i32,
    special_effect: &'static str,
}

const DESTINY_TRAITS: &[DestinyTrait] = &[
    // 战命方向
    DestinyTrait {
        id: "warrior_spirit",
        name: "战士之魂",
        description: "天生为战而生，物攻永久提升",
        destiny_type: "战命",
        tier_level: 1,
        hp_bonus: 50,
        ad_bonus: 15,
        ap_bonus: 0,
        def_bonus: 5,
        mdef_bonus: 0,
        special_effect: "物攻+15",
    },
    DestinyTrait {
        id: "battle_fury",
        name: "战斗狂怒",
        description: "越战越勇，HP越低攻击越高",
        destiny_type: "战命",
        tier_level: 2,
        hp_bonus: 100,
        ad_bonus: 25,
        ap_bonus: 0,
        def_bonus: 5,
        mdef_bonus: 5,
        special_effect: "HP<50%时攻击+30%",
    },
    DestinyTrait {
        id: "iron_will",
        name: "钢铁意志",
        description: "坚不可摧的意志，防御大幅提升",
        destiny_type: "战命",
        tier_level: 3,
        hp_bonus: 200,
        ad_bonus: 10,
        ap_bonus: 0,
        def_bonus: 25,
        mdef_bonus: 15,
        special_effect: "防御+25",
    },
    DestinyTrait {
        id: "berserker_blood",
        name: "狂战之血",
        description: "狂战士的血脉，暴击率提升",
        destiny_type: "战命",
        tier_level: 4,
        hp_bonus: 150,
        ad_bonus: 40,
        ap_bonus: 0,
        def_bonus: 10,
        mdef_bonus: 5,
        special_effect: "暴击率+15%",
    },
    DestinyTrait {
        id: "war_god",
        name: "战神降临",
        description: "战神之力附体，全属性大幅提升",
        destiny_type: "战命",
        tier_level: 5,
        hp_bonus: 500,
        ad_bonus: 60,
        ap_bonus: 30,
        def_bonus: 30,
        mdef_bonus: 20,
        special_effect: "全属性大幅加成",
    },
    // 法命方向
    DestinyTrait {
        id: "arcane_talent",
        name: "魔法天赋",
        description: "天生的魔法亲和力，魔攻提升",
        destiny_type: "法命",
        tier_level: 1,
        hp_bonus: 30,
        ad_bonus: 0,
        ap_bonus: 15,
        def_bonus: 0,
        mdef_bonus: 5,
        special_effect: "魔攻+15",
    },
    DestinyTrait {
        id: "mana_flow",
        name: "魔力涌流",
        description: "魔力源源不断，魔法上限提升",
        destiny_type: "法命",
        tier_level: 2,
        hp_bonus: 80,
        ad_bonus: 0,
        ap_bonus: 25,
        def_bonus: 5,
        mdef_bonus: 10,
        special_effect: "魔攻+25",
    },
    DestinyTrait {
        id: "arcane_mastery",
        name: "奥术精通",
        description: "掌握奥术之力，魔攻大幅增加",
        destiny_type: "法命",
        tier_level: 3,
        hp_bonus: 120,
        ad_bonus: 0,
        ap_bonus: 40,
        def_bonus: 10,
        mdef_bonus: 20,
        special_effect: "魔攻+40",
    },
    DestinyTrait {
        id: "spell_supreme",
        name: "法神之威",
        description: "法神降世之力，魔法伤害翻倍",
        destiny_type: "法命",
        tier_level: 5,
        hp_bonus: 300,
        ad_bonus: 10,
        ap_bonus: 80,
        def_bonus: 15,
        mdef_bonus: 40,
        special_effect: "魔法伤害+20%",
    },
    // 商命方向
    DestinyTrait {
        id: "merchant_instinct",
        name: "商人本能",
        description: "天生的商业头脑，金币获取增加",
        destiny_type: "商命",
        tier_level: 1,
        hp_bonus: 20,
        ad_bonus: 5,
        ap_bonus: 5,
        def_bonus: 5,
        mdef_bonus: 5,
        special_effect: "金币获取+10%",
    },
    DestinyTrait {
        id: "treasure_sense",
        name: "寻宝直觉",
        description: "第六感指引宝藏位置",
        destiny_type: "商命",
        tier_level: 3,
        hp_bonus: 80,
        ad_bonus: 10,
        ap_bonus: 10,
        def_bonus: 10,
        mdef_bonus: 10,
        special_effect: "掉落率+15%",
    },
    DestinyTrait {
        id: "golden_touch",
        name: "点石成金",
        description: "触碰之物皆变为黄金",
        destiny_type: "商命",
        tier_level: 5,
        hp_bonus: 200,
        ad_bonus: 20,
        ap_bonus: 20,
        def_bonus: 20,
        mdef_bonus: 20,
        special_effect: "金币获取+50%",
    },
    // 幸命方向
    DestinyTrait {
        id: "lucky_star",
        name: "福星高照",
        description: "幸运之神眷顾，暴击率提升",
        destiny_type: "幸命",
        tier_level: 1,
        hp_bonus: 30,
        ad_bonus: 5,
        ap_bonus: 5,
        def_bonus: 3,
        mdef_bonus: 3,
        special_effect: "暴击率+5%",
    },
    DestinyTrait {
        id: "miracle",
        name: "奇迹之力",
        description: "创造奇迹的力量，受到致命伤害时有概率存活",
        destiny_type: "幸命",
        tier_level: 4,
        hp_bonus: 300,
        ad_bonus: 15,
        ap_bonus: 15,
        def_bonus: 15,
        mdef_bonus: 15,
        special_effect: "致死存活率10%",
    },
    // 缘命方向
    DestinyTrait {
        id: "kindred_spirit",
        name: "灵犀相通",
        description: "与他人产生共鸣，组队加成提升",
        destiny_type: "缘命",
        tier_level: 1,
        hp_bonus: 50,
        ad_bonus: 8,
        ap_bonus: 8,
        def_bonus: 8,
        mdef_bonus: 8,
        special_effect: "组队经验+10%",
    },
    DestinyTrait {
        id: "fate_bond",
        name: "命运羁绊",
        description: "与伴侣的命运相连",
        destiny_type: "缘命",
        tier_level: 3,
        hp_bonus: 150,
        ad_bonus: 20,
        ap_bonus: 20,
        def_bonus: 15,
        mdef_bonus: 15,
        special_effect: "伴侣加成+20%",
    },
    // 隐命方向
    DestinyTrait {
        id: "shadow_step",
        name: "暗影步法",
        description: "暗影中的行者，闪避率提升",
        destiny_type: "隐命",
        tier_level: 1,
        hp_bonus: 20,
        ad_bonus: 10,
        ap_bonus: 0,
        def_bonus: 3,
        mdef_bonus: 3,
        special_effect: "闪避率+5%",
    },
    DestinyTrait {
        id: "phantom_dodge",
        name: "幻影闪避",
        description: "化为幻影避开攻击",
        destiny_type: "隐命",
        tier_level: 4,
        hp_bonus: 100,
        ad_bonus: 25,
        ap_bonus: 10,
        def_bonus: 10,
        mdef_bonus: 10,
        special_effect: "闪避率+15%",
    },
    // 慧命方向
    DestinyTrait {
        id: "wisdom_eye",
        name: "慧眼识珠",
        description: "洞察一切的慧眼，经验值获取增加",
        destiny_type: "慧命",
        tier_level: 1,
        hp_bonus: 30,
        ad_bonus: 5,
        ap_bonus: 5,
        def_bonus: 5,
        mdef_bonus: 5,
        special_effect: "经验获取+10%",
    },
    DestinyTrait {
        id: "enlightenment",
        name: "顿悟",
        description: "瞬间顿悟天地之道",
        destiny_type: "慧命",
        tier_level: 4,
        hp_bonus: 200,
        ad_bonus: 20,
        ap_bonus: 20,
        def_bonus: 15,
        mdef_bonus: 15,
        special_effect: "经验获取+40%",
    },
    // 命命方向 (终极)
    DestinyTrait {
        id: "destiny_master",
        name: "命运主宰",
        description: "掌控命运本身，全属性大幅提升",
        destiny_type: "命命",
        tier_level: 6,
        hp_bonus: 600,
        ad_bonus: 50,
        ap_bonus: 50,
        def_bonus: 40,
        mdef_bonus: 40,
        special_effect: "全属性+55%",
    },
    DestinyTrait {
        id: "fate_transcend",
        name: "命运超脱",
        description: "超越命运的束缚，获得至高之力",
        destiny_type: "命命",
        tier_level: 7,
        hp_bonus: 1000,
        ad_bonus: 80,
        ap_bonus: 80,
        def_bonus: 60,
        mdef_bonus: 60,
        special_effect: "全属性大幅提升",
    },
];

/// 获取命格等级信息
fn get_tier(level: i32) -> &'static DestinyTier {
    DESTINY_TIERS
        .iter()
        .find(|t| t.level == level)
        .unwrap_or(&DESTINY_TIERS[0])
}

/// 读取玩家命格数据
fn load_destiny_data(db: &Database, user_id: &str) -> (i32, i32, String) {
    let conn = db.lock_conn();
    let tier = {
        let key = format!("{}_tier", user_id);
        let mut stmt = conn
            .prepare("SELECT Value FROM Global WHERE SECTION='destiny' AND Key=?1")
            .unwrap();
        stmt.query_row(params![key], |row| row.get::<_, String>(0))
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1)
    };
    let fragments = {
        let key = format!("{}_fragments", user_id);
        let mut stmt = conn
            .prepare("SELECT Value FROM Global WHERE SECTION='destiny' AND Key=?1")
            .unwrap();
        stmt.query_row(params![key], |row| row.get::<_, String>(0))
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    };
    let traits = {
        let key = format!("{}_traits", user_id);
        let mut stmt = conn
            .prepare("SELECT Value FROM Global WHERE SECTION='destiny' AND Key=?1")
            .unwrap();
        stmt.query_row(params![key], |row| row.get::<_, String>(0))
            .unwrap_or_default()
    };
    (tier, fragments, traits)
}

/// 保存玩家命格数据
fn save_destiny_data(db: &Database, user_id: &str, tier: i32, fragments: i32, traits: &str) {
    let conn = db.lock_conn();
    for (suffix, val) in &[
        ("_tier", tier.to_string()),
        ("_fragments", fragments.to_string()),
        ("_traits", traits.to_string()),
    ] {
        let key = format!("{}{}", user_id, suffix);
        let _ = conn.execute(
            "INSERT OR REPLACE INTO Global (SECTION, Key, Value) VALUES ('destiny', ?1, ?2)",
            params![key, val],
        );
    }
}

/// 获取命格总属性加成
pub fn get_destiny_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let (_tier, _fragments, traits_str) = load_destiny_data(db, user_id);
    if traits_str.is_empty() {
        return (0, 0, 0, 0, 0);
    }
    let activated: Vec<&str> = traits_str.split(',').collect();
    let (mut hp, mut ad, mut ap, mut def, mut mdef) = (0i32, 0i32, 0i32, 0i32, 0i32);
    for trait_id in &activated {
        if let Some(t) = DESTINY_TRAITS.iter().find(|dt| dt.id == *trait_id) {
            hp += t.hp_bonus;
            ad += t.ad_bonus;
            ap += t.ap_bonus;
            def += t.def_bonus;
            mdef += t.mdef_bonus;
        }
    }
    (hp, ad, ap, def, mdef)
}

/// 指令: 查看命格
pub fn cmd_view_destiny(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier, fragments, traits_str) = load_destiny_data(db, user_id);
    let current = get_tier(tier);
    let next = if tier < 7 { Some(get_tier(tier + 1)) } else { None };

    let mut out = String::from("═══════ 🌟 命格系统 🌟 ═══════\n\n");
    out.push_str(&format!(
        "当前命格: {} {} ({})\n",
        current.emoji, current.name, current.color
    ));
    out.push_str(&format!("命格碎片: {} 个\n", fragments));
    if let Some(n) = next {
        out.push_str(&format!(
            "下一等级: {} {} (需要 {} 碎片)\n",
            n.emoji, n.name, n.required_fragments
        ));
        let progress = (fragments as f64 / n.required_fragments as f64 * 100.0).min(100.0);
        let bar_len = 20;
        let filled = (progress / 100.0 * bar_len as f64) as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
        out.push_str(&format!("升级进度: [{}] {:.0}%\n\n", bar, progress));
    } else {
        out.push_str("\n🎉 已达最高命格！\n\n");
    }

    let (hp, ad, ap, def_val, mdef) = get_destiny_bonus(db, user_id);
    out.push_str("【命格加成】\n");
    out.push_str(&format!("  ❤️ HP+{}  ⚔️ 物攻+{}  🔮 魔攻+{}\n", hp, ad, ap));
    out.push_str(&format!("  🛡️ 防御+{}  💫 魔抗+{}\n", def_val, mdef));

    let activated: Vec<&str> = if traits_str.is_empty() {
        vec![]
    } else {
        traits_str.split(',').collect()
    };
    out.push_str(&format!("\n【已激活特质】({}个)\n", activated.len()));
    if activated.is_empty() {
        out.push_str("  暂无已激活特质\n");
    } else {
        for tid in &activated {
            if let Some(t) = DESTINY_TRAITS.iter().find(|dt| dt.id == *tid) {
                out.push_str(&format!("  {} {} — {}\n", t.destiny_type, t.name, t.special_effect));
            }
        }
    }

    out.push_str("\n💡 使用「命格碎片」+金币兑换/钻石兑换 获取碎片\n");
    out.push_str("💡 使用「激活命格」+特质ID 激活特质\n");
    out.push_str("💡 使用「命格目录」查看所有可激活特质\n");
    out
}

/// 指令: 获取命格碎片
pub fn cmd_gain_fragments(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return "❌ 请指定兑换方式：金币兑换 / 钻石兑换\n💡 金币兑换=10000金币=1碎片\n💡 钻石兑换=10钻石=3碎片"
            .to_string();
    }

    if !db.user_exists(user_id) {
        return "❌ 请先注册".to_string();
    }

    let (tier, fragments, traits_str) = load_destiny_data(db, user_id);
    let next_tier = if tier < 7 { Some(get_tier(tier + 1)) } else { None };

    let (new_fragments, cost_desc) = if args.contains("金币") {
        let gold = db.read_currency(user_id, "金币");
        let cost = 10000i64;
        if gold < cost {
            return format!("❌ 金币不足！需要 {} 金币 (当前: {})", cost, gold);
        }
        db.write_currency(user_id, "金币", gold - cost);
        (fragments + 1, format!("消耗 {} 金币获得 1 碎片", cost))
    } else if args.contains("钻石") {
        let diamond = db.read_currency(user_id, "钻石");
        let cost = 10i64;
        if diamond < cost {
            return format!("❌ 钻石不足！需要 {} 钻石 (当前: {})", cost, diamond);
        }
        db.write_currency(user_id, "钻石", diamond - cost);
        (fragments + 3, format!("消耗 {} 钻石获得 3 碎片", cost))
    } else {
        return "❌ 无效兑换方式，请使用「金币兑换」或「钻石兑换」".to_string();
    };

    save_destiny_data(db, user_id, tier, new_fragments, &traits_str);

    let mut out = format!("{}\n", cost_desc);
    out.push_str(&format!("当前碎片: {} 个\n", new_fragments));
    if let Some(n) = next_tier {
        if new_fragments >= n.required_fragments {
            out.push_str(&format!(
                "🎯 碎片已足够升级到 {} {}！使用「命格升级」\n",
                n.emoji, n.name
            ));
        }
    }
    out
}

/// 指令: 命格升级
pub fn cmd_upgrade_destiny(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier, fragments, traits_str) = load_destiny_data(db, user_id);
    if tier >= 7 {
        return "❌ 已达最高命格等级「神命」！".to_string();
    }
    let next = get_tier(tier + 1);
    if fragments < next.required_fragments {
        return format!(
            "❌ 碎片不足！需要 {} 碎片，当前 {} 碎片",
            next.required_fragments, fragments
        );
    }

    let new_fragments = fragments - next.required_fragments;
    save_destiny_data(db, user_id, tier + 1, new_fragments, &traits_str);

    let mut out = String::from("🎉 命格升级成功！\n\n");
    out.push_str(&format!(
        "{} {} → {} {}\n",
        get_tier(tier).emoji,
        get_tier(tier).name,
        next.emoji,
        next.name
    ));
    out.push_str(&format!("等级加成: +{}% 全属性\n", next.bonus_pct));
    out.push_str(&format!("剩余碎片: {}\n", new_fragments));
    out.push_str("\n💡 新特质已解锁，使用「命格目录」查看\n");
    out
}

/// 指令: 命格目录
pub fn cmd_destiny_codex(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let (tier, _fragments, traits_str) = load_destiny_data(db, user_id);
    let activated: Vec<&str> = if traits_str.is_empty() {
        vec![]
    } else {
        traits_str.split(',').collect()
    };

    let mut out = String::from("═══════ 📖 命格目录 📖 ═══════\n\n");
    let mut current_type = "";
    for t in DESTINY_TRAITS {
        if t.destiny_type != current_type {
            current_type = t.destiny_type;
            out.push_str(&format!("━━ {} ━━\n", current_type));
        }
        let status = if activated.contains(&t.id) {
            "✅ 已激活"
        } else if tier >= t.tier_level {
            "🔓 可激活"
        } else {
            "🔒 未解锁"
        };
        let tier_info = get_tier(t.tier_level);
        out.push_str(&format!(
            "  {} {} [{}] — {} (需{})\n",
            status, t.name, t.id, t.description, tier_info.name
        ));
        out.push_str(&format!(
            "    加成: ❤️+{} ⚔️+{} 🔮+{} 🛡️+{} 💫+{} | {}\n",
            t.hp_bonus, t.ad_bonus, t.ap_bonus, t.def_bonus, t.mdef_bonus, t.special_effect
        ));
    }
    out.push_str("\n💡 使用「激活命格」+特质ID 激活特质\n");
    out
}

/// 指令: 激活命格特质
pub fn cmd_activate_destiny(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let trait_id = args.trim();
    if trait_id.is_empty() {
        return "❌ 请指定特质ID，使用「命格目录」查看可用特质".to_string();
    }

    let trait_def = match DESTINY_TRAITS.iter().find(|t| t.id == trait_id) {
        Some(t) => t,
        None => return format!("❌ 未找到特质「{}」，请检查ID", trait_id),
    };

    let (tier, fragments, traits_str) = load_destiny_data(db, user_id);
    let activated: Vec<&str> = if traits_str.is_empty() {
        vec![]
    } else {
        traits_str.split(',').collect()
    };

    if activated.contains(&trait_id) {
        return format!("❌ 特质「{}」已激活", trait_def.name);
    }
    if tier < trait_def.tier_level {
        return format!(
            "❌ 命格等级不足！需要 {} (当前: {})",
            get_tier(trait_def.tier_level).name,
            get_tier(tier).name
        );
    }

    let new_traits = if traits_str.is_empty() {
        trait_id.to_string()
    } else {
        format!("{},{}", traits_str, trait_id)
    };

    save_destiny_data(db, user_id, tier, fragments, &new_traits);

    let mut out = String::from("🎉 特质激活成功！\n\n");
    out.push_str(&format!(
        "{} {} — {}\n",
        trait_def.destiny_type, trait_def.name, trait_def.description
    ));
    out.push_str(&format!(
        "属性加成: ❤️+{} ⚔️+{} 🔮+{} 🛡️+{} 💫+{}\n",
        trait_def.hp_bonus, trait_def.ad_bonus, trait_def.ap_bonus, trait_def.def_bonus, trait_def.mdef_bonus
    ));
    out.push_str(&format!("特殊效果: {}\n", trait_def.special_effect));
    out
}

/// 指令: 命格排行
pub fn cmd_destiny_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare(
        "SELECT Key, Value FROM Global WHERE SECTION='destiny' AND Key LIKE '%_tier' ORDER BY CAST(Value AS INTEGER) DESC LIMIT 15",
    ) {
        Ok(s) => s,
        Err(_) => return "❌ 暂无排行数据".to_string(),
    };

    let rows: Vec<(String, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1)
                    .ok()
                    .and_then(|v| v.parse::<i32>().ok())
                    .unwrap_or(1),
            ))
        })
        .map(|r| r.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut out = String::from("═══════ 🌟 命格排行 🌟 ═══════\n\n");
    if rows.is_empty() {
        out.push_str("暂无排行数据\n");
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (key, tier_val)) in rows.iter().enumerate() {
            let uid = key.replace("_tier", "");
            let tier_info = get_tier(*tier_val);
            let medal = if i < 3 { medals[i] } else { "  " };
            out.push_str(&format!(
                "{} {}. {} — {} {}\n",
                medal,
                i + 1,
                uid,
                tier_info.emoji,
                tier_info.name
            ));
        }
    }
    out
}

/// 指令: 命格帮助
pub fn cmd_destiny_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    String::from(
        "═══════ ❓ 命格系统帮助 ═══════\n\n\
         命格系统是角色深层成长系统，通过收集碎片升级命格等级，激活特质获得永久属性加成。\n\n\
         【命格等级】\n\
         ⚪ 凡命 → 🟢 安命(5碎片) → 🔵 福命(15) → 🟣 贵命(30)\n\
         → 🟠 天命(50) → 🔴 帝命(80) → 🟡 神命(120)\n\n\
         【碎片获取】\n\
         · 金币兑换: 10000金币 = 1碎片\n\
         · 钻石兑换: 10钻石 = 3碎片\n\
         · 击败BOSS/完成成就 也有概率掉落\n\n\
         【特质系统】\n\
         · 8大命运方向: 战命/法命/商命/幸命/缘命/隐命/慧命/命命\n\
         · 每个方向3-4个特质，越高级加成越强\n\
         · 命格等级达到要求后可激活对应特质\n\n\
         【指令列表】\n\
         · 查看命格 — 查看当前命格状态\n\
         · 命格碎片+金币兑换/钻石兑换 — 获取碎片\n\
         · 命格升级 — 升级命格等级\n\
         · 命格目录 — 查看所有特质\n\
         · 激活命格+特质ID — 激活指定特质\n\
         · 命格排行 — 全服命格排行\n\
         · 命格帮助 — 查看帮助",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_count() {
        assert_eq!(DESTINY_TIERS.len(), 7);
    }

    #[test]
    fn test_tier_escalation() {
        for i in 1..DESTINY_TIERS.len() {
            assert!(DESTINY_TIERS[i].required_fragments > DESTINY_TIERS[i - 1].required_fragments);
            assert!(DESTINY_TIERS[i].bonus_pct > DESTINY_TIERS[i - 1].bonus_pct);
        }
    }

    #[test]
    fn test_tier_lookup() {
        assert_eq!(get_tier(1).name, "凡命");
        assert_eq!(get_tier(7).name, "神命");
        assert_eq!(get_tier(99).name, "凡命"); // fallback
    }

    #[test]
    fn test_trait_count() {
        assert!(
            DESTINY_TRAITS.len() >= 20,
            "Expected at least 20 destiny traits, got {}",
            DESTINY_TRAITS.len()
        );
    }

    #[test]
    fn test_trait_ids_unique() {
        let mut ids: Vec<&str> = DESTINY_TRAITS.iter().map(|t| t.id).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "Trait IDs are not unique");
    }

    #[test]
    fn test_trait_types() {
        let types: Vec<&str> = DESTINY_TRAITS.iter().map(|t| t.destiny_type).collect();
        assert!(types.contains(&"战命"));
        assert!(types.contains(&"法命"));
        assert!(types.contains(&"商命"));
        assert!(types.contains(&"命命"));
    }

    #[test]
    fn test_trait_bonuses_positive() {
        for t in DESTINY_TRAITS {
            assert!(t.hp_bonus >= 0, "Trait {} has negative HP bonus", t.id);
            assert!(
                t.ad_bonus > 0 || t.ap_bonus > 0,
                "Trait {} has no offensive bonus",
                t.id
            );
        }
    }

    #[test]
    fn test_trait_tier_level_range() {
        for t in DESTINY_TRAITS {
            assert!(
                t.tier_level >= 1 && t.tier_level <= 7,
                "Trait {} has invalid tier_level {}",
                t.id,
                t.tier_level
            );
        }
    }

    #[test]
    fn test_higher_tier_traits_have_higher_bonuses() {
        let warrior_traits: Vec<&DestinyTrait> = DESTINY_TRAITS.iter().filter(|t| t.destiny_type == "战命").collect();
        let mut sorted = warrior_traits;
        sorted.sort_by_key(|t| t.tier_level);
        if sorted.len() >= 2 {
            let first_total = sorted[0].hp_bonus + sorted[0].ad_bonus + sorted[0].def_bonus;
            let last_total = sorted[sorted.len() - 1].hp_bonus
                + sorted[sorted.len() - 1].ad_bonus
                + sorted[sorted.len() - 1].def_bonus;
            assert!(last_total > first_total);
        }
    }

    #[test]
    fn test_ultimate_destiny_traits() {
        let master = DESTINY_TRAITS.iter().find(|t| t.id == "destiny_master").unwrap();
        let transcend = DESTINY_TRAITS.iter().find(|t| t.id == "fate_transcend").unwrap();
        assert!(master.hp_bonus >= 500);
        assert!(transcend.hp_bonus >= 800);
        assert!(transcend.tier_level == 7);
    }

    #[test]
    fn test_tier_emoji_non_empty() {
        for t in DESTINY_TIERS {
            assert!(!t.emoji.is_empty());
            assert!(!t.name.is_empty());
        }
    }

    #[test]
    fn test_trait_names_non_empty() {
        for t in DESTINY_TRAITS {
            assert!(!t.id.is_empty());
            assert!(!t.name.is_empty());
            assert!(!t.description.is_empty());
        }
    }

    #[test]
    fn test_all_tiers_have_traits() {
        for tier in 1..=7 {
            let count = DESTINY_TRAITS.iter().filter(|t| t.tier_level == tier).count();
            assert!(count >= 1, "No traits for tier {}", tier);
        }
    }
}

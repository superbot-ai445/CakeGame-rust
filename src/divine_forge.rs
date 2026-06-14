/// CakeGame 神兵铸造系统 (Divine Forge System)
///
/// 终局武器铸造系统：收集神兵材料，铸造传说武器，突破进化
/// 神兵拥有独立技能和属性加成，不同于普通装备
///
/// 神兵品质: 凡铁→精钢→秘银→星陨→龙魂→神话→创世
/// 铸造材料: 从BOSS/副本/世界事件掉落
/// 突破进化: 使用材料+碎片突破品质上限
///
/// 指令: 神兵列表, 铸造神兵, 神兵详情, 神兵突破, 神兵强化, 神兵分解, 神兵排行, 神兵帮助
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 神兵品质等级
const DIVINE_TIERS: &[DivineTier] = &[
    DivineTier {
        name: "凡铁",
        level: 1,
        emoji: "⬜",
        color: "白色",
        bonus_pct: 0,
        upgrade_cost: 100,
        material_cost: 5,
    },
    DivineTier {
        name: "精钢",
        level: 2,
        emoji: "🟢",
        color: "绿色",
        bonus_pct: 15,
        upgrade_cost: 300,
        material_cost: 15,
    },
    DivineTier {
        name: "秘银",
        level: 3,
        emoji: "🔵",
        color: "蓝色",
        bonus_pct: 35,
        upgrade_cost: 800,
        material_cost: 40,
    },
    DivineTier {
        name: "星陨",
        level: 4,
        emoji: "🟣",
        color: "紫色",
        bonus_pct: 60,
        upgrade_cost: 2000,
        material_cost: 100,
    },
    DivineTier {
        name: "龙魂",
        level: 5,
        emoji: "🟠",
        color: "橙色",
        bonus_pct: 90,
        upgrade_cost: 5000,
        material_cost: 250,
    },
    DivineTier {
        name: "神话",
        level: 6,
        emoji: "🔴",
        color: "红色",
        bonus_pct: 130,
        upgrade_cost: 12000,
        material_cost: 600,
    },
    DivineTier {
        name: "创世",
        level: 7,
        emoji: "🟡",
        color: "金色",
        bonus_pct: 180,
        upgrade_cost: 30000,
        material_cost: 1500,
    },
];

/// 神兵武器定义
const DIVINE_WEAPONS: &[DivineWeaponDef] = &[
    DivineWeaponDef {
        name: "炎龙之怒",
        weapon_type: "剑",
        emoji: "🗡️🔥",
        lore: "远古炎龙陨落时遗留的龙鳞铸就，蕴含焚天灭地之力",
        base_hp: 200,
        base_ad: 80,
        base_ap: 0,
        base_def: 20,
        base_mr: 15,
        special_name: "龙焰斩",
        special_desc: "攻击时15%概率触发龙焰，造成150%攻击力的火属性伤害",
        special_chance: 15,
        special_mult: 150,
        source: "击败野外BOSS·炎龙",
    },
    DivineWeaponDef {
        name: "寒冰之矛",
        weapon_type: "矛",
        emoji: "🔱❄️",
        lore: "取自北境千年寒冰之心锻造，刺出时冻结万物",
        base_hp: 150,
        base_ad: 90,
        base_ap: 0,
        base_def: 10,
        base_mr: 25,
        special_name: "冰封突刺",
        special_desc: "攻击时12%概率冰封目标，降低30%速度持续2回合",
        special_chance: 12,
        special_mult: 130,
        source: "击败副本BOSS·冰霜巨灵",
    },
    DivineWeaponDef {
        name: "雷神之锤",
        weapon_type: "锤",
        emoji: "🔨⚡",
        lore: "雷神陨落后遗落人间的神器，挥击间雷霆万钧",
        base_hp: 300,
        base_ad: 70,
        base_ap: 0,
        base_def: 35,
        base_mr: 10,
        special_name: "雷霆一击",
        special_desc: "攻击时10%概率引发雷击，对目标及周围敌人造成120%伤害",
        special_chance: 10,
        special_mult: 120,
        source: "击败世界BOSS·雷霆之主",
    },
    DivineWeaponDef {
        name: "暗影匕首",
        weapon_type: "匕首",
        emoji: "🗡️🌑",
        lore: "暗影位面渗出的物质凝结而成，专为暗杀而生",
        base_hp: 100,
        base_ad: 100,
        base_ap: 0,
        base_def: 5,
        base_mr: 20,
        special_name: "致命暗影",
        special_desc: "攻击时20%概率暴击率翻倍，暴击伤害+80%",
        special_chance: 20,
        special_mult: 180,
        source: "完成深渊第50层挑战",
    },
    DivineWeaponDef {
        name: "圣光法杖",
        weapon_type: "杖",
        emoji: "🪄✨",
        lore: "天使赐福的圣物，凝聚纯净的圣光之力",
        base_hp: 180,
        base_ad: 0,
        base_ap: 95,
        base_def: 15,
        base_mr: 30,
        special_name: "圣光洗礼",
        special_desc: "释放技能时18%概率恢复自身15%最大生命值",
        special_chance: 18,
        special_mult: 115,
        source: "完成公会试炼第五层",
    },
    DivineWeaponDef {
        name: "生命之弓",
        weapon_type: "弓",
        emoji: "🏹🌿",
        lore: "世界树的枝干制成，箭矢命中后汲取敌人的生命力",
        base_hp: 120,
        base_ad: 85,
        base_ap: 0,
        base_def: 10,
        base_mr: 15,
        special_name: "生命汲取",
        special_desc: "攻击时16%概率吸取造成伤害的25%恢复自身HP",
        special_chance: 16,
        special_mult: 125,
        source: "击败野外BOSS·远古树精",
    },
    DivineWeaponDef {
        name: "虚空之书",
        weapon_type: "书",
        emoji: "📖🌀",
        lore: "记载虚空禁忌知识的典籍，翻开即触碰混沌",
        base_hp: 160,
        base_ad: 0,
        base_ap: 110,
        base_def: 8,
        base_mr: 35,
        special_name: "虚空风暴",
        special_desc: "释放技能时14%概率触发虚空风暴，全体敌人受到90%魔攻伤害",
        special_chance: 14,
        special_mult: 90,
        source: "完成时空裂隙·灭世难度",
    },
    DivineWeaponDef {
        name: "混沌之盾",
        weapon_type: "盾",
        emoji: "🛡️🌀",
        lore: "混沌初开时的第一件造物，能吸收一切伤害",
        base_hp: 500,
        base_ad: 30,
        base_ap: 0,
        base_def: 60,
        base_mr: 40,
        special_name: "混沌护盾",
        special_desc: "受到攻击时20%概率生成护盾，吸收200%防御力的伤害",
        special_chance: 20,
        special_mult: 200,
        source: "完成冒险工会SS级任务",
    },
];

// 铸造所需材料
#[allow(dead_code)]
const FORGE_MATERIALS: &[ForgeMaterial] = &[
    ForgeMaterial {
        name: "神铁矿石",
        emoji: "⛏️",
        source: "挖矿/BOSS掉落",
        base_drop: 3,
    },
    ForgeMaterial {
        name: "龙鳞碎片",
        emoji: "🐉",
        source: "龙系BOSS",
        base_drop: 1,
    },
    ForgeMaterial {
        name: "星辰精华",
        emoji: "⭐",
        source: "深渊/裂隙",
        base_drop: 2,
    },
    ForgeMaterial {
        name: "灵魂结晶",
        emoji: "💎",
        source: "竞技场/PvP",
        base_drop: 2,
    },
    ForgeMaterial {
        name: "混沌之尘",
        emoji: "🌀",
        source: "世界事件",
        base_drop: 1,
    },
];

const SECTION: &str = "divine_forge";

struct DivineTier {
    name: &'static str,
    level: i32,
    emoji: &'static str,
    #[allow(dead_code)]
    color: &'static str,
    bonus_pct: i32,
    upgrade_cost: i64,
    material_cost: i64,
}

struct DivineWeaponDef {
    name: &'static str,
    weapon_type: &'static str,
    emoji: &'static str,
    lore: &'static str,
    base_hp: i32,
    base_ad: i32,
    base_ap: i32,
    base_def: i32,
    base_mr: i32,
    special_name: &'static str,
    special_desc: &'static str,
    special_chance: i32,
    special_mult: i32,
    source: &'static str,
}

#[allow(dead_code)]
struct ForgeMaterial {
    name: &'static str,
    emoji: &'static str,
    source: &'static str,
    base_drop: i32,
}

#[derive(Clone)]
struct PlayerDivineWeapon {
    weapon_idx: usize,
    tier_level: i32,
    enhance_level: i32,
    #[allow(dead_code)]
    refine_count: i32,
}

/// 计算神兵属性（含品质加成）
fn calc_weapon_stats(weapon_idx: usize, tier_level: i32) -> (i32, i32, i32, i32, i32) {
    let w = &DIVINE_WEAPONS[weapon_idx];
    let tier = DIVINE_TIERS
        .iter()
        .find(|t| t.level == tier_level)
        .unwrap_or(&DIVINE_TIERS[0]);
    let mult = 100 + tier.bonus_pct;
    (
        w.base_hp * mult / 100,
        w.base_ad * mult / 100,
        w.base_ap * mult / 100,
        w.base_def * mult / 100,
        w.base_mr * mult / 100,
    )
}

/// 战力计算
fn calc_weapon_power(hp: i32, ad: i32, ap: i32, def: i32, mr: i32) -> i32 {
    hp / 5 + ad * 3 + ap * 3 + def * 2 + mr * 2
}

/// 获取神兵存储 key
fn divine_key(user_id: &str) -> String {
    format!("divine_forge_{}", user_id)
}

/// 解析玩家神兵数据
fn parse_divine_data(db: &Database, user_id: &str) -> Vec<PlayerDivineWeapon> {
    let key = divine_key(user_id);
    let data = db.global_get(SECTION, &key);
    if data.is_empty() {
        return Vec::new();
    }
    data.split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(';').collect();
            if parts.len() >= 4 {
                Some(PlayerDivineWeapon {
                    weapon_idx: parts[0].parse().unwrap_or(0),
                    tier_level: parts[1].parse().unwrap_or(1),
                    enhance_level: parts[2].parse().unwrap_or(0),
                    refine_count: parts[3].parse().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

/// 保存玩家神兵数据
fn save_divine_data(db: &Database, user_id: &str, weapons: &[PlayerDivineWeapon]) {
    let key = divine_key(user_id);
    let data = weapons
        .iter()
        .map(|w| {
            format!(
                "{};{};{};{}",
                w.weapon_idx, w.tier_level, w.enhance_level, w.refine_count
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    db.global_set(SECTION, &key, &data);
}

/// 获取玩家锻造材料数
fn get_materials(db: &Database, user_id: &str) -> i64 {
    let key = format!("divine_materials_{}", user_id);
    db.global_get(SECTION, &key).parse().unwrap_or(0)
}

/// 设置材料数
fn set_materials(db: &Database, user_id: &str, amount: i64) {
    let key = format!("divine_materials_{}", user_id);
    db.global_set(SECTION, &key, &amount.to_string());
}

/// 获取碎片数
fn get_fragments(db: &Database, user_id: &str) -> i64 {
    let key = format!("divine_fragments_{}", user_id);
    db.global_get(SECTION, &key).parse().unwrap_or(0)
}

/// 设置碎片数
fn set_fragments(db: &Database, user_id: &str, amount: i64) {
    let key = format!("divine_fragments_{}", user_id);
    db.global_set(SECTION, &key, &amount.to_string());
}

// ==================== 公共API ====================

/// 获取神兵加成 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_divine_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let weapons = parse_divine_data(db, user_id);
    let mut total = (0, 0, 0, 0, 0);
    for w in &weapons {
        let (hp, ad, ap, def, mr) = calc_weapon_stats(w.weapon_idx, w.tier_level);
        let enhance_mult = 100 + w.enhance_level * 2;
        total.0 += hp * enhance_mult / 100;
        total.1 += ad * enhance_mult / 100;
        total.2 += ap * enhance_mult / 100;
        total.3 += def * enhance_mult / 100;
        total.4 += mr * enhance_mult / 100;
    }
    total
}

// ==================== 指令实现 ====================

/// 神兵列表 - 查看所有可铸造神兵
pub fn cmd_divine_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "⚠️ 请先注册游戏\n发送: 注册+昵称".to_string();
    }

    let weapons = parse_divine_data(db, user_id);
    let materials = get_materials(db, user_id);
    let fragments = get_fragments(db, user_id);

    let mut out = String::from("⚔️ ═══ 神兵铸造殿 ═══ ⚔️\n\n");
    out.push_str(&format!(
        "📦 锻造材料: {} 个 | 💎 神兵碎片: {} 个\n\n",
        materials, fragments
    ));

    out.push_str("📜 可铸造神兵:\n");
    for (i, w) in DIVINE_WEAPONS.iter().enumerate() {
        let owned = weapons.iter().find(|pw| pw.weapon_idx == i);
        if let Some(pw) = owned {
            let tier = DIVINE_TIERS
                .iter()
                .find(|t| t.level == pw.tier_level)
                .unwrap_or(&DIVINE_TIERS[0]);
            let (hp, ad, ap, def, mr) = calc_weapon_stats(i, pw.tier_level);
            let power = calc_weapon_power(hp, ad, ap, def, mr);
            out.push_str(&format!(
                "{} {} [{}] {} Lv.{} | 战力:{} | HP:{} 攻:{} 魔:{} 防:{} 抗:{}\n",
                w.emoji, w.name, tier.name, tier.emoji, pw.enhance_level, power, hp, ad, ap, def, mr
            ));
        } else {
            out.push_str(&format!(
                "  ❓ {} {} [未铸造] — 类型:{} | 来源:{}\n",
                w.emoji, w.name, w.weapon_type, w.source
            ));
        }
    }

    out.push_str("\n💡 指令: 铸造神兵+名称 | 神兵详情+名称 | 神兵突破 | 神兵强化+名称");
    out
}

/// 铸造神兵 - 消耗材料铸造指定神兵
pub fn cmd_divine_forge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "⚠️ 请先注册游戏\n发送: 注册+昵称".to_string();
    }

    let weapon_name = args.trim();
    if weapon_name.is_empty() {
        return "⚠️ 请指定神兵名称\n格式: 铸造神兵+神兵名称\n例如: 铸造神兵+炎龙之怒".to_string();
    }

    let weapon_idx = DIVINE_WEAPONS.iter().position(|w| w.name == weapon_name);
    let weapon_idx = match weapon_idx {
        Some(idx) => idx,
        None => {
            let names: Vec<&str> = DIVINE_WEAPONS.iter().map(|w| w.name).collect();
            return format!("⚠️ 未找到神兵「{}」\n可铸造: {}", weapon_name, names.join("、"));
        }
    };

    let mut weapons = parse_divine_data(db, user_id);
    if weapons.iter().any(|w| w.weapon_idx == weapon_idx) {
        return format!("⚠️ 你已拥有「{}」，不可重复铸造", weapon_name);
    }

    // 等级要求
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if user_level < 30 {
        return "⚠️ 铸造神兵需要达到30级".to_string();
    }

    // 材料检查
    let materials = get_materials(db, user_id);
    let cost = DIVINE_TIERS[0].material_cost;
    if materials < cost {
        return format!(
            "⚠️ 材料不足！铸造「{}」需要 {} 个锻造材料，当前仅有 {} 个\n💡 材料可通过击败BOSS/挖矿/副本获得",
            weapon_name, cost, materials
        );
    }

    // 金币检查
    let gold_cost: i64 = 10000;
    let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
    if user_gold < gold_cost {
        return format!("⚠️ 金币不足！铸造需要 {} 金币", gold_cost);
    }

    // 扣除资源
    set_materials(db, user_id, materials - cost);
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_cost);

    // 铸造
    weapons.push(PlayerDivineWeapon {
        weapon_idx,
        tier_level: 1,
        enhance_level: 0,
        refine_count: 0,
    });
    save_divine_data(db, user_id, &weapons);

    let w = &DIVINE_WEAPONS[weapon_idx];
    let (hp, ad, ap, def, mr) = calc_weapon_stats(weapon_idx, 1);
    let power = calc_weapon_power(hp, ad, ap, def, mr);

    let mut out = format!(
        "🎉 ═══ 神兵铸造成功！ ═══ 🎉\n\n\
         {} {} 已铸成！\n\
         📖 {}\n\n\
         ⚔️ 类型: {} | 品质: ⬜ 凡铁\n\
         📊 战力: {} | HP:{} 攻:{} 魔:{} 防:{} 抗:{}\n\n\
         ✨ 特技: {} — {}\n\n\
         💡 消耗: {} 材料 + {} 金币\n\
         🔧 使用「神兵突破」可提升品质",
        w.emoji,
        w.name,
        w.lore,
        w.weapon_type,
        power,
        hp,
        ad,
        ap,
        def,
        mr,
        w.special_name,
        w.special_desc,
        cost,
        gold_cost
    );

    // 经验奖励
    user::add_experience(db, user_id, 500);
    out.push_str("\n🎁 获得 500 经验奖励！");
    out
}

/// 神兵详情 - 查看指定神兵详细信息
pub fn cmd_divine_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "⚠️ 请先注册游戏\n发送: 注册+昵称".to_string();
    }

    let weapon_name = args.trim();
    if weapon_name.is_empty() {
        return "⚠️ 请指定神兵名称\n格式: 神兵详情+神兵名称".to_string();
    }

    let weapons = parse_divine_data(db, user_id);
    let weapon_idx = DIVINE_WEAPONS.iter().position(|w| w.name == weapon_name);
    let weapon_idx = match weapon_idx {
        Some(idx) => idx,
        None => return format!("⚠️ 未找到神兵「{}」", weapon_name),
    };

    let pw = match weapons.iter().find(|w| w.weapon_idx == weapon_idx) {
        Some(pw) => pw,
        None => return format!("⚠️ 你尚未铸造「{}」\n发送: 铸造神兵+{}", weapon_name, weapon_name),
    };

    let w = &DIVINE_WEAPONS[weapon_idx];
    let tier = DIVINE_TIERS
        .iter()
        .find(|t| t.level == pw.tier_level)
        .unwrap_or(&DIVINE_TIERS[0]);
    let (hp, ad, ap, def, mr) = calc_weapon_stats(weapon_idx, pw.tier_level);
    let enhance_mult = 100 + pw.enhance_level * 2;
    let hp_e = hp * enhance_mult / 100;
    let ad_e = ad * enhance_mult / 100;
    let ap_e = ap * enhance_mult / 100;
    let def_e = def * enhance_mult / 100;
    let mr_e = mr * enhance_mult / 100;
    let power = calc_weapon_power(hp_e, ad_e, ap_e, def_e, mr_e);

    let next_tier = DIVINE_TIERS.iter().find(|t| t.level == pw.tier_level + 1);

    let mut out = format!(
        "⚔️ ═══ {} {} ═══ ⚔️\n\n\
         {} {}\n\
         📖 {}\n\n\
         🏷️ 类型: {} | 品质: {} {} Lv.{}\n\
         🔧 强化: +{} (加成: +{}%)\n\
         🔨 精炼次数: {}\n\n\
         📊 ═══ 属性 (含强化) ═══\n\
         ❤️ 生命: {} (+{})\n\
         ⚔️ 物攻: {} (+{})\n\
         🔮 魔攻: {} (+{})\n\
         🛡️ 防御: {} (+{})\n\
         🔰 魔抗: {} (+{})\n\
         💪 战力: {}\n\n\
         ✨ ═══ 特技: {} ═══\n\
         {}\n\
         触发概率: {}% | 伤害倍率: {}%\n",
        w.emoji,
        w.name,
        w.emoji,
        tier.emoji,
        w.lore,
        w.weapon_type,
        tier.name,
        tier.color,
        pw.tier_level,
        pw.enhance_level,
        pw.enhance_level * 2,
        pw.refine_count,
        hp_e,
        hp_e - hp,
        ad_e,
        ad_e - ad,
        ap_e,
        ap_e - ap,
        def_e,
        def_e - def,
        mr_e,
        mr_e - mr,
        power,
        w.special_name,
        w.special_desc,
        w.special_chance,
        w.special_mult
    );

    if let Some(next) = next_tier {
        let materials = get_materials(db, user_id);
        let fragments = get_fragments(db, user_id);
        let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
        out.push_str(&format!(
            "⬆️ ═══ 下次突破 ═══\n\
             目标: {} {} → {} {}\n\
             需要: {} 材料 + {} 碎片 + {} 金币\n\
             当前: {} 材料 / {} 碎片 / {} 金币\n",
            tier.emoji,
            tier.name,
            next.emoji,
            next.name,
            next.material_cost,
            next.material_cost / 2,
            next.upgrade_cost,
            materials,
            fragments,
            user_gold
        ));
    } else {
        out.push_str("👑 已达到最高品质·创世级！\n");
    }

    out
}

/// 神兵突破 - 提升神兵品质等级
pub fn cmd_divine_upgrade(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "⚠️ 请先注册游戏\n发送: 注册+昵称".to_string();
    }

    let weapons = parse_divine_data(db, user_id);
    if weapons.is_empty() {
        return "⚠️ 你还没有铸造任何神兵\n发送: 铸造神兵+名称".to_string();
    }

    // 找到品质最低的可突破神兵
    let mut best_weapon: Option<usize> = None;
    let mut best_tier = i32::MAX;
    for (i, w) in weapons.iter().enumerate() {
        if w.tier_level < best_tier && w.tier_level < DIVINE_TIERS.len() as i32 {
            best_tier = w.tier_level;
            best_weapon = Some(i);
        }
    }

    let wi = match best_weapon {
        Some(i) => i,
        None => return "👑 所有神兵已达最高品质·创世级！".to_string(),
    };

    let pw = &weapons[wi];
    let current_tier = DIVINE_TIERS
        .iter()
        .find(|t| t.level == pw.tier_level)
        .unwrap_or(&DIVINE_TIERS[0]);
    let next_tier = match DIVINE_TIERS.iter().find(|t| t.level == pw.tier_level + 1) {
        Some(t) => t,
        None => return format!("👑 「{}」已达最高品质！", DIVINE_WEAPONS[pw.weapon_idx].name),
    };

    // 资源检查
    let materials = get_materials(db, user_id);
    let fragments = get_fragments(db, user_id);
    let user_gold = db.read_currency(user_id, CURRENCY_GOLD);

    if materials < next_tier.material_cost {
        return format!(
            "⚠️ 材料不足！突破到{}需要{}个材料，当前{}个\n💡 击败BOSS/挖矿/副本可获得材料",
            next_tier.name, next_tier.material_cost, materials
        );
    }
    if fragments < next_tier.material_cost / 2 {
        return format!(
            "⚠️ 碎片不足！突破到{}需要{}个碎片，当前{}个\n💡 分解多余神兵可获得碎片",
            next_tier.name,
            next_tier.material_cost / 2,
            fragments
        );
    }
    if user_gold < next_tier.upgrade_cost {
        return format!("⚠️ 金币不足！突破需要{}金币", next_tier.upgrade_cost);
    }

    // 扣除资源
    set_materials(db, user_id, materials - next_tier.material_cost);
    set_fragments(db, user_id, fragments - next_tier.material_cost / 2);
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, next_tier.upgrade_cost);

    // 突破
    let mut weapons = weapons;
    weapons[wi].tier_level = next_tier.level;
    weapons[wi].enhance_level = 0; // 突破重置强化等级
    save_divine_data(db, user_id, &weapons);

    let w = &DIVINE_WEAPONS[weapons[wi].weapon_idx];
    let (hp, ad, ap, def, mr) = calc_weapon_stats(weapons[wi].weapon_idx, next_tier.level);
    let power = calc_weapon_power(hp, ad, ap, def, mr);

    let mut out = format!(
        "🎉 ═══ 神兵突破成功！ ═══ 🎉\n\n\
         {} {}\n\
         {} {} → {} {}\n\n\
         📊 新属性: HP:{} 攻:{} 魔:{} 防:{} 抗:{} | 战力:{}\n\n\
         💰 消耗: {} 材料 + {} 碎片 + {} 金币\n",
        w.emoji,
        w.name,
        current_tier.emoji,
        current_tier.name,
        next_tier.emoji,
        next_tier.name,
        hp,
        ad,
        ap,
        def,
        mr,
        power,
        next_tier.material_cost,
        next_tier.material_cost / 2,
        next_tier.upgrade_cost
    );

    // 经验奖励
    let exp = next_tier.level as i32 * 200;
    user::add_experience(db, user_id, exp);
    out.push_str(&format!("🎁 获得 {} 经验奖励！", exp));
    out
}

/// 神兵强化 - 消耗材料强化已拥有的神兵
pub fn cmd_divine_enhance(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "⚠️ 请先注册游戏\n发送: 注册+昵称".to_string();
    }

    let weapon_name = args.trim();
    if weapon_name.is_empty() {
        return "⚠️ 请指定神兵名称\n格式: 神兵强化+神兵名称".to_string();
    }

    let weapons = parse_divine_data(db, user_id);
    let weapon_idx = DIVINE_WEAPONS.iter().position(|w| w.name == weapon_name);
    let weapon_idx = match weapon_idx {
        Some(idx) => idx,
        None => return format!("⚠️ 未找到神兵「{}」", weapon_name),
    };

    let wi = match weapons.iter().position(|w| w.weapon_idx == weapon_idx) {
        Some(i) => i,
        None => return format!("⚠️ 你尚未铸造「{}」", weapon_name),
    };

    let pw = weapons[wi].clone();
    let max_enhance = pw.tier_level * 10;
    if pw.enhance_level >= max_enhance {
        return format!(
            "⚠️ 「{}」当前品质({})的强化上限为+{}，需先突破品质",
            weapon_name,
            DIVINE_TIERS
                .iter()
                .find(|t| t.level == pw.tier_level)
                .map(|t| t.name)
                .unwrap_or("?"),
            max_enhance
        );
    }

    // 消耗检查
    let materials = get_materials(db, user_id);
    let cost = 5 + pw.enhance_level as i64;
    if materials < cost {
        return format!(
            "⚠️ 材料不足！强化+{}需要{}材料，当前{}",
            pw.enhance_level + 1,
            cost,
            materials
        );
    }

    let gold_cost = 1000 + pw.enhance_level as i64 * 500;
    let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
    if user_gold < gold_cost {
        return format!("⚠️ 金币不足！强化需要{}金币", gold_cost);
    }

    // 成功率: 100% - 强化等级 * 3，最低30%
    let success_rate = std::cmp::max(30, 100 - pw.enhance_level * 3);

    // 扣除资源
    set_materials(db, user_id, materials - cost);
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_cost);

    // 判定成功/失败
    let roll = rand::random::<u32>() % 100;
    let mut weapons = weapons;
    if roll < success_rate as u32 {
        // 成功
        weapons[wi].enhance_level += 1;
        save_divine_data(db, user_id, &weapons);

        let w = &DIVINE_WEAPONS[weapon_idx];
        let (hp, ad, ap, def, mr) = calc_weapon_stats(weapon_idx, weapons[wi].tier_level);
        let mult = 100 + weapons[wi].enhance_level * 2;
        let power = calc_weapon_power(
            hp * mult / 100,
            ad * mult / 100,
            ap * mult / 100,
            def * mult / 100,
            mr * mult / 100,
        );

        format!(
            "✨ 强化成功！「{}」 → +{}\n\
             📊 战力: {} | 加成: +{}%\n\
             💰 消耗: {} 材料 + {} 金币\n\
             📈 成功率: {}% | 下次需要: {} 材料 + {} 金币",
            w.name,
            weapons[wi].enhance_level,
            power,
            weapons[wi].enhance_level * 2,
            cost,
            gold_cost,
            success_rate,
            5 + weapons[wi].enhance_level as i64,
            1000 + weapons[wi].enhance_level as i64 * 500
        )
    } else {
        // 失败但不降级
        save_divine_data(db, user_id, &weapons);
        format!(
            "💥 强化失败！「{}」+{} 未变化\n\
             📈 成功率: {}% | 再试一次吧！\n\
             💰 已消耗: {} 材料 + {} 金币",
            DIVINE_WEAPONS[weapon_idx].name, pw.enhance_level, success_rate, cost, gold_cost
        )
    }
}

/// 神兵分解 - 分解多余神兵获取碎片
pub fn cmd_divine_dismantle(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "⚠️ 请先注册游戏\n发送: 注册+昵称".to_string();
    }

    let weapon_name = args.trim();
    if weapon_name.is_empty() {
        return "⚠️ 请指定神兵名称\n格式: 神兵分解+神兵名称\n⚠️ 分解后不可恢复！".to_string();
    }

    let weapon_idx = DIVINE_WEAPONS.iter().position(|w| w.name == weapon_name);
    let weapon_idx = match weapon_idx {
        Some(idx) => idx,
        None => return format!("⚠️ 未找到神兵「{}」", weapon_name),
    };

    let mut weapons = parse_divine_data(db, user_id);
    let wi = match weapons.iter().position(|w| w.weapon_idx == weapon_idx) {
        Some(i) => i,
        None => return format!("⚠️ 你尚未拥有「{}」", weapon_name),
    };

    // 至少保留1把
    if weapons.len() <= 1 {
        return "⚠️ 至少保留1把神兵，无法分解最后一把".to_string();
    }

    let pw = weapons.remove(wi);
    save_divine_data(db, user_id, &weapons);

    // 碎片奖励: 基础50 + 品质*30 + 强化*5
    let fragment_gain: i64 = 50 + pw.tier_level as i64 * 30 + pw.enhance_level as i64 * 5;
    let material_gain: i64 = pw.tier_level as i64 * 10;
    let current_fragments = get_fragments(db, user_id);
    let current_materials = get_materials(db, user_id);
    set_fragments(db, user_id, current_fragments + fragment_gain);
    set_materials(db, user_id, current_materials + material_gain);

    let tier = DIVINE_TIERS
        .iter()
        .find(|t| t.level == pw.tier_level)
        .unwrap_or(&DIVINE_TIERS[0]);

    format!(
        "♻️ ═══ 神兵分解完成 ═══ ♻️\n\n\
         分解: {} {} {} Lv.{}\n\n\
         💎 获得碎片: +{} (总计: {})\n\
         📦 获得材料: +{} (总计: {})\n\n\
         💡 碎片和材料可用于突破其他神兵",
        DIVINE_WEAPONS[weapon_idx].emoji,
        DIVINE_WEAPONS[weapon_idx].name,
        tier.emoji,
        pw.enhance_level,
        fragment_gain,
        current_fragments + fragment_gain,
        material_gain,
        current_materials + material_gain
    )
}

/// 神兵排行 - 全服神兵拥有排行
pub fn cmd_divine_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_uids = db.all_users();
    let mut rankings: Vec<(String, i32, i32, String)> = Vec::new();

    for uid in &all_uids {
        let value = db.global_get(SECTION, &divine_key(uid));
        if value.is_empty() {
            continue;
        }
        let weapons: Vec<PlayerDivineWeapon> = value
            .split('|')
            .filter_map(|entry| {
                let parts: Vec<&str> = entry.split(';').collect();
                if parts.len() >= 4 {
                    Some(PlayerDivineWeapon {
                        weapon_idx: parts[0].parse().unwrap_or(0),
                        tier_level: parts[1].parse().unwrap_or(1),
                        enhance_level: parts[2].parse().unwrap_or(0),
                        refine_count: parts[3].parse().unwrap_or(0),
                    })
                } else {
                    None
                }
            })
            .collect();

        if weapons.is_empty() {
            continue;
        }

        let total_power: i32 = weapons
            .iter()
            .map(|w| {
                let (hp, ad, ap, def, mr) = calc_weapon_stats(w.weapon_idx, w.tier_level);
                let mult = 100 + w.enhance_level * 2;
                calc_weapon_power(
                    hp * mult / 100,
                    ad * mult / 100,
                    ap * mult / 100,
                    def * mult / 100,
                    mr * mult / 100,
                )
            })
            .sum();

        let best_tier = weapons.iter().map(|w| w.tier_level).max().unwrap_or(0);
        let count = weapons.len() as i32;
        let tier_name = DIVINE_TIERS
            .iter()
            .find(|t| t.level == best_tier)
            .map(|t| t.name)
            .unwrap_or("无");
        let name = user::get_msg_prefix(db, uid);
        rankings.push((name, total_power, count, tier_name.to_string()));
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = String::from("⚔️ ═══ 神兵排行 ═══ ⚔️\n\n");
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (name, power, count, tier)) in rankings.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        out.push_str(&format!(
            "{} #{} 玩家:{} | 战力:{} | 数量:{} | 最高:{}\n",
            medal,
            i + 1,
            name,
            power,
            count,
            tier
        ));
    }

    if rankings.is_empty() {
        out.push_str("暂无神兵铸造记录\n");
    }

    out.push_str("\n💡 铸造和突破神兵提升排名！");
    out
}

/// 神兵帮助
pub fn cmd_divine_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    String::from(
        "⚔️ ═══ 神兵铸造系统帮助 ═══ ⚔️\n\n\
         📜 指令列表:\n\
         1. 神兵列表 — 查看所有可铸造神兵\n\
         2. 铸造神兵+名称 — 消耗材料铸造神兵(30级)\n\
         3. 神兵详情+名称 — 查看神兵详细属性\n\
         4. 神兵突破 — 提升神兵品质等级\n\
         5. 神兵强化+名称 — 强化神兵提升属性\n\
         6. 神兵分解+名称 — 分解神兵获取碎片\n\
         7. 神兵排行 — 全服神兵排名\n\
         8. 神兵帮助 — 查看帮助\n\n\
         🏆 品质等级:\n\
         ⬜ 凡铁 → 🟢 精钢 → 🔵 秘银 → 🟣 星陨\n\
         → 🟠 龙魂 → 🔴 神话 → 🟡 创世\n\n\
         📦 材料获取:\n\
         • 击败BOSS掉落锻造材料\n\
         • 挖矿/采集获得神铁矿石\n\
         • 深渊/裂隙获得星辰精华\n\
         • 分解多余神兵获取碎片\n\n\
         💡 提示:\n\
         • 品质越高，属性加成越强\n\
         • 强化上限=品质等级×10\n\
         • 突破会重置强化等级\n\
         • 分解返还碎片+材料",
    )
}

/// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_count() {
        assert_eq!(DIVINE_TIERS.len(), 7);
    }

    #[test]
    fn test_weapon_count() {
        assert_eq!(DIVINE_WEAPONS.len(), 8);
    }

    #[test]
    fn test_tier_escalation() {
        for i in 1..DIVINE_TIERS.len() {
            assert!(DIVINE_TIERS[i].bonus_pct > DIVINE_TIERS[i - 1].bonus_pct);
            assert!(DIVINE_TIERS[i].upgrade_cost >= DIVINE_TIERS[i - 1].upgrade_cost);
            assert!(DIVINE_TIERS[i].material_cost >= DIVINE_TIERS[i - 1].material_cost);
        }
    }

    #[test]
    fn test_tier_names_unique() {
        let mut names: Vec<&str> = DIVINE_TIERS.iter().map(|t| t.name).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before);
    }

    #[test]
    fn test_weapon_names_unique() {
        let mut names: Vec<&str> = DIVINE_WEAPONS.iter().map(|w| w.name).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before);
    }

    #[test]
    fn test_weapon_emojis() {
        for w in DIVINE_WEAPONS {
            assert!(!w.emoji.is_empty());
            assert!(!w.special_name.is_empty());
            assert!(!w.special_desc.is_empty());
        }
    }

    #[test]
    fn test_calc_weapon_stats_tier1() {
        let (hp, ad, ap, def, mr) = calc_weapon_stats(0, 1);
        assert_eq!(hp, DIVINE_WEAPONS[0].base_hp);
        assert_eq!(ad, DIVINE_WEAPONS[0].base_ad);
        assert_eq!(ap, DIVINE_WEAPONS[0].base_ap);
        assert_eq!(def, DIVINE_WEAPONS[0].base_def);
        assert_eq!(mr, DIVINE_WEAPONS[0].base_mr);
    }

    #[test]
    fn test_calc_weapon_stats_higher_tier() {
        let (hp1, ad1, _, _, _) = calc_weapon_stats(0, 1);
        let (hp2, ad2, _, _, _) = calc_weapon_stats(0, 3);
        assert!(hp2 > hp1);
        assert!(ad2 > ad1);
    }

    #[test]
    fn test_calc_weapon_power() {
        let power = calc_weapon_power(100, 50, 0, 20, 10);
        assert!(power > 0);
        assert_eq!(power, 100 / 5 + 50 * 3 + 0 * 3 + 20 * 2 + 10 * 2);
    }

    #[test]
    fn test_material_types() {
        assert_eq!(FORGE_MATERIALS.len(), 5);
        for m in FORGE_MATERIALS {
            assert!(!m.name.is_empty());
            assert!(!m.emoji.is_empty());
            assert!(m.base_drop > 0);
        }
    }

    #[test]
    fn test_weapon_sources() {
        for w in DIVINE_WEAPONS {
            assert!(!w.source.is_empty());
        }
    }

    #[test]
    fn test_divine_key_format() {
        let key = divine_key("user123");
        assert_eq!(key, "divine_forge_user123");
    }

    #[test]
    fn test_weapon_types() {
        let types: Vec<&str> = DIVINE_WEAPONS.iter().map(|w| w.weapon_type).collect();
        assert!(types.contains(&"剑"));
        assert!(types.contains(&"矛"));
        assert!(types.contains(&"锤"));
        assert!(types.contains(&"匕首"));
        assert!(types.contains(&"杖"));
        assert!(types.contains(&"弓"));
        assert!(types.contains(&"书"));
        assert!(types.contains(&"盾"));
    }

    #[test]
    fn test_enhance_success_rate_bounds() {
        // At level 0, should be 100%
        let rate_0 = std::cmp::max(30, 100 - 0 * 3);
        assert_eq!(rate_0, 100);
        // At level 20, should be 40%
        let rate_20 = std::cmp::max(30, 100 - 20 * 3);
        assert_eq!(rate_20, 40);
        // At level 30+, should cap at 30%
        let rate_30 = std::cmp::max(30, 100 - 30 * 3);
        assert_eq!(rate_30, 30);
    }

    #[test]
    fn test_dismantle_formula() {
        // tier 1, enhance 0: 50 + 1*30 + 0*5 = 80
        let gain_t1 = 50 + 1 * 30 + 0 * 5;
        assert_eq!(gain_t1, 80);
        // tier 5, enhance 20: 50 + 5*30 + 20*5 = 300
        let gain_t5 = 50 + 5 * 30 + 20 * 5;
        assert_eq!(gain_t5, 300);
    }

    #[test]
    fn test_all_weapon_base_stats_positive() {
        for w in DIVINE_WEAPONS {
            assert!(w.base_hp > 0, "{} should have positive HP", w.name);
            assert!(w.base_ad + w.base_ap > 0, "{} should have some attack", w.name);
        }
    }
}

// 坐骑系统 — 收集、培养、骑乘，全面提升战力
// 各种神兽坐骑，升星进阶，被动属性加成，坐骑技能

use crate::core::{CURRENCY_GOLD, OP_SUB};
use crate::db::Database;
use crate::user;

const SECTION: &str = "mount_system";
const SECTION_DAILY: &str = "mount_daily";

// ═══════════════════════════════════════════════════
// 坐骑品质体系
// ═══════════════════════════════════════════════════

struct MountQuality {
    name: &'static str,
    emoji: &'static str,
    stat_multiplier: f64,
    max_stars: i32,
    unlock_cost_gold: i64,
}

const MOUNT_QUALITIES: &[MountQuality] = &[
    MountQuality {
        name: "普通",
        emoji: "⬜",
        stat_multiplier: 1.0,
        max_stars: 3,
        unlock_cost_gold: 5000,
    },
    MountQuality {
        name: "优秀",
        emoji: "🟢",
        stat_multiplier: 1.5,
        max_stars: 5,
        unlock_cost_gold: 20000,
    },
    MountQuality {
        name: "稀有",
        emoji: "🔵",
        stat_multiplier: 2.2,
        max_stars: 7,
        unlock_cost_gold: 80000,
    },
    MountQuality {
        name: "史诗",
        emoji: "🟣",
        stat_multiplier: 3.5,
        max_stars: 9,
        unlock_cost_gold: 300000,
    },
    MountQuality {
        name: "传说",
        emoji: "🟠",
        stat_multiplier: 5.0,
        max_stars: 10,
        unlock_cost_gold: 1000000,
    },
];

// ═══════════════════════════════════════════════════
// 14种坐骑
// ═══════════════════════════════════════════════════

struct MountDef {
    id: &'static str,
    name: &'static str,
    emoji: &'static str,
    quality: usize, // index into MOUNT_QUALITIES
    base_hp: i32,
    base_ad: i32,
    base_ap: i32,
    base_def: i32,
    base_mres: i32,
    speed_bonus: i32, // 移动速度加成 %
    skill_name: &'static str,
    skill_desc: &'static str,
    skill_chance: i32, // 触发概率 %
    description: &'static str,
}

const MOUNT_DEFS: &[MountDef] = &[
    // 普通品质 (0)
    MountDef {
        id: "brown_horse",
        name: "棕马",
        emoji: "🐴",
        quality: 0,
        base_hp: 50,
        base_ad: 10,
        base_ap: 5,
        base_def: 8,
        base_mres: 5,
        speed_bonus: 10,
        skill_name: "疾奔",
        skill_desc: "战斗开始时15%概率闪避第一击",
        skill_chance: 15,
        description: "忠诚的棕色战马，骑士的首选坐骑",
    },
    MountDef {
        id: "gray_wolf",
        name: "灰狼",
        emoji: "🐺",
        quality: 0,
        base_hp: 40,
        base_ad: 15,
        base_ap: 5,
        base_def: 6,
        base_mres: 8,
        speed_bonus: 15,
        skill_name: "狼嚎",
        skill_desc: "战斗开始时10%概率降低敌方防御5%",
        skill_chance: 10,
        description: "北方荒原的灰狼，敏捷而凶猛",
    },
    // 优秀品质 (1)
    MountDef {
        id: "war_bear",
        name: "战熊",
        emoji: "🐻",
        quality: 1,
        base_hp: 120,
        base_ad: 25,
        base_ap: 10,
        base_def: 20,
        base_mres: 12,
        speed_bonus: 5,
        skill_name: "熊掌重击",
        skill_desc: "12%概率造成150%伤害",
        skill_chance: 12,
        description: "铁甲战熊，力量与耐力的完美结合",
    },
    MountDef {
        id: "white_tiger",
        name: "白虎",
        emoji: "🐯",
        quality: 1,
        base_hp: 80,
        base_ad: 35,
        base_ap: 15,
        base_def: 12,
        base_mres: 15,
        speed_bonus: 20,
        skill_name: "虎啸",
        skill_desc: "15%概率使敌人眩晕1回合",
        skill_chance: 15,
        description: "雪原白虎，威风凛凛的百兽之王",
    },
    MountDef {
        id: "desert_camel",
        name: "沙漠骆驼",
        emoji: "🐫",
        quality: 1,
        base_hp: 100,
        base_ad: 12,
        base_ap: 12,
        base_def: 25,
        base_mres: 20,
        speed_bonus: 8,
        skill_name: "沙漠之魂",
        skill_desc: "沙漠地图战斗时全属性+10%",
        skill_chance: 100,
        description: "穿越沙漠的忠实伙伴，耐力惊人",
    },
    // 稀有品质 (2)
    MountDef {
        id: "flame_steed",
        name: "烈焰驹",
        emoji: "🔥",
        quality: 2,
        base_hp: 180,
        base_ad: 45,
        base_ap: 30,
        base_def: 25,
        base_mres: 20,
        speed_bonus: 25,
        skill_name: "烈焰冲锋",
        skill_desc: "18%概率对敌方造成灼烧(持续3回合)",
        skill_chance: 18,
        description: "浑身燃烧火焰的神驹，所过之处寸草不生",
    },
    MountDef {
        id: "ice_unicorn",
        name: "冰晶独角兽",
        emoji: "🦄",
        quality: 2,
        base_hp: 160,
        base_ad: 30,
        base_ap: 50,
        base_def: 22,
        base_mres: 30,
        speed_bonus: 20,
        skill_name: "冰晶护盾",
        skill_desc: "20%概率生成等同于魔攻50%的护盾",
        skill_chance: 20,
        description: "寒冰国度的独角圣兽，魔力充盈",
    },
    // 史诗品质 (3)
    MountDef {
        id: "thunder_drake",
        name: "雷霆飞龙",
        emoji: "🐉",
        quality: 3,
        base_hp: 300,
        base_ad: 70,
        base_ap: 50,
        base_def: 40,
        base_mres: 35,
        speed_bonus: 35,
        skill_name: "雷霆一击",
        skill_desc: "22%概率释放雷电造成双倍伤害",
        skill_chance: 22,
        description: "翱翔天际的雷龙，电闪雷鸣",
    },
    MountDef {
        id: "shadow_panther",
        name: "暗影猎豹",
        emoji: "🐆",
        quality: 3,
        base_hp: 220,
        base_ad: 90,
        base_ap: 30,
        base_def: 30,
        base_mres: 40,
        speed_bonus: 40,
        skill_name: "暗影突袭",
        skill_desc: "25%概率无视防御进行攻击",
        skill_chance: 25,
        description: "暗夜中的无声杀手，致命而优雅",
    },
    MountDef {
        id: "golden_lion",
        name: "金鬃狮王",
        emoji: "🦁",
        quality: 3,
        base_hp: 350,
        base_ad: 60,
        base_ap: 40,
        base_def: 55,
        base_mres: 45,
        speed_bonus: 20,
        skill_name: "狮王之怒",
        skill_desc: "20%概率震慑敌人降低攻击20%",
        skill_chance: 20,
        description: "草原之王，金色鬃毛彰显王者之气",
    },
    // 传说品质 (4)
    MountDef {
        id: "phoenix",
        name: "凤凰涅槃",
        emoji: "🔥",
        quality: 4,
        base_hp: 500,
        base_ad: 80,
        base_ap: 100,
        base_def: 60,
        base_mres: 60,
        speed_bonus: 30,
        skill_name: "涅槃重生",
        skill_desc: "死亡时30%概率复活并恢复50%HP",
        skill_chance: 30,
        description: "浴火重生的神鸟，永生不灭",
    },
    MountDef {
        id: "void_dragon",
        name: "虚空巨龙",
        emoji: "🐲",
        quality: 4,
        base_hp: 600,
        base_ad: 120,
        base_ap: 80,
        base_def: 70,
        base_mres: 55,
        speed_bonus: 45,
        skill_name: "虚空吞噬",
        skill_desc: "25%概率吞噬敌人10%当前HP",
        skill_chance: 25,
        description: "来自虚空深处的远古巨龙，毁天灭地",
    },
    MountDef {
        id: "celestial_qilin",
        name: "天界麒麟",
        emoji: "✨",
        quality: 4,
        base_hp: 450,
        base_ad: 70,
        base_ap: 90,
        base_def: 80,
        base_mres: 80,
        speed_bonus: 25,
        skill_name: "天降祥瑞",
        skill_desc: "战斗胜利时额外获得20%金币和经验",
        skill_chance: 100,
        description: "天界降临的祥瑞神兽，万邪不侵",
    },
    MountDef {
        id: "chaos_behemoth",
        name: "混沌巨兽",
        emoji: "👹",
        quality: 4,
        base_hp: 800,
        base_ad: 150,
        base_ap: 60,
        base_def: 90,
        base_mres: 70,
        speed_bonus: 10,
        skill_name: "混沌践踏",
        skill_desc: "18%概率对敌方全体造成50%伤害",
        skill_chance: 18,
        description: "混沌初开时诞生的远古巨兽，大地为之震颤",
    },
];

// ═══════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════

fn parse_mount_data(data: &str) -> (Vec<String>, String, i32) {
    // owned_mounts: JSON array of mount IDs, active_mount: mount_id, feed_points: i32
    let mut owned: Vec<String> = Vec::new();
    let mut active = String::new();
    let mut feed_points = 0i32;
    for part in data.split('|') {
        if let Some(v) = part.strip_prefix("owned=") {
            if v != "none" && !v.is_empty() {
                owned = v.split(',').map(|s| s.to_string()).collect();
            }
        } else if let Some(v) = part.strip_prefix("active=") {
            active = v.to_string();
        } else if let Some(v) = part.strip_prefix("feed=") {
            feed_points = v.parse::<i32>().unwrap_or(0);
        }
    }
    (owned, active, feed_points)
}

fn serialize_mount_data(owned: &[String], active: &str, feed_points: i32) -> String {
    let owned_str = if owned.is_empty() {
        "none".to_string()
    } else {
        owned.join(",")
    };
    format!("owned={}|active={}|feed={}", owned_str, active, feed_points)
}

fn parse_mount_stars(data: &str) -> std::collections::HashMap<String, i32> {
    let mut map = std::collections::HashMap::new();
    for part in data.split(',') {
        if part.is_empty() {
            continue;
        }
        let kv: Vec<&str> = part.splitn(2, ':').collect();
        if kv.len() == 2 {
            if let Ok(stars) = kv[1].parse::<i32>() {
                map.insert(kv[0].to_string(), stars);
            }
        }
    }
    map
}

fn serialize_mount_stars(map: &std::collections::HashMap<String, i32>) -> String {
    let mut parts: Vec<String> = map.iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
    parts.sort();
    parts.join(",")
}

fn find_mount(id: &str) -> Option<&'static MountDef> {
    MOUNT_DEFS.iter().find(|m| m.id == id)
}

fn mount_power(m: &MountDef, stars: i32) -> i64 {
    let q = &MOUNT_QUALITIES[m.quality];
    let star_mult = 1.0 + (stars as f64) * 0.1;
    let base = (m.base_hp + m.base_ad + m.base_ap + m.base_def + m.base_mres) as f64;
    (base * q.stat_multiplier * star_mult + m.speed_bonus as f64 * 10.0) as i64
}

fn star_upgrade_cost(stars: i32) -> i64 {
    match stars {
        0..=2 => 5000 + stars as i64 * 3000,
        3..=5 => 20000 + (stars - 3) as i64 * 15000,
        6..=8 => 80000 + (stars - 6) as i64 * 40000,
        _ => 250000 + (stars - 9) as i64 * 100000,
    }
}

fn today_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (secs / 86400) as i64;
    format!("day_{}", days)
}

// ═══════════════════════════════════════════════════
// 公共 API：获取坐骑属性加成
// ═══════════════════════════════════════════════════

#[allow(dead_code)]
pub fn get_mount_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32) {
    let data = db.global_get(SECTION, user_id);
    let (_, active, _) = parse_mount_data(&data);
    if active.is_empty() || active == "none" {
        return (0, 0, 0, 0, 0);
    }
    let stars_data = db.global_get(SECTION, &format!("stars_{}", user_id));
    let stars_map = parse_mount_stars(&stars_data);
    let stars = stars_map.get(&active).copied().unwrap_or(0);

    if let Some(m) = find_mount(&active) {
        let q = &MOUNT_QUALITIES[m.quality];
        let mult = q.stat_multiplier * (1.0 + stars as f64 * 0.1);
        (
            (m.base_hp as f64 * mult) as i32,
            (m.base_ad as f64 * mult) as i32,
            (m.base_ap as f64 * mult) as i32,
            (m.base_def as f64 * mult) as i32,
            (m.base_mres as f64 * mult) as i32,
        )
    } else {
        (0, 0, 0, 0, 0)
    }
}

// ═══════════════════════════════════════════════════
// 命令：查看坐骑
// ═══════════════════════════════════════════════════

pub fn cmd_view_mount(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let info = user::calc_total_attrs(db, user_id);
    let level = info.level;

    let data = db.global_get(SECTION, user_id);
    let (owned, active, feed_points) = parse_mount_data(&data);
    let stars_data = db.global_get(SECTION, &format!("stars_{}", user_id));
    let stars_map = parse_mount_stars(&stars_data);

    let mut out = String::from("══════ 🐴 坐骑系统 🐴 ══════\n");
    out.push_str(&format!(
        "等级: {} | 已收集: {}/{} 坐骑\n",
        level,
        owned.len(),
        MOUNT_DEFS.len()
    ));
    out.push_str(&format!("培养值: {}\n", feed_points));
    out.push('\n');

    if active.is_empty() || active == "none" {
        out.push_str("当前未骑乘坐骑\n");
        out.push_str("使用「坐骑列表」查看所有可获得的坐骑\n");
    } else if let Some(m) = find_mount(&active) {
        let q = &MOUNT_QUALITIES[m.quality];
        let stars = stars_map.get(&active).copied().unwrap_or(0);
        let star_str = "⭐".repeat(stars as usize);
        out.push_str(&format!("当前骑乘: {} {} [{}{}]\n", m.emoji, m.name, q.emoji, q.name));
        out.push_str(&format!("星级: {} ({}/{})\n", star_str, stars, q.max_stars));
        out.push_str(&format!("速度加成: +{}%\n", m.speed_bonus));
        out.push_str(&format!(
            "技能: {} — {} ({}%概率)\n",
            m.skill_name, m.skill_desc, m.skill_chance
        ));
        let (hp, ad, ap, def, mres) = get_mount_bonus(db, user_id);
        out.push_str(&format!(
            "属性加成: HP+{} AD+{} AP+{} DEF+{} MRES+{}\n",
            hp, ad, ap, def, mres
        ));
        let power = mount_power(m, stars);
        out.push_str(&format!("坐骑战力: {}\n", power));
    }
    out.push_str("\n指令: 坐骑列表/骑乘[坐骑]/坐骑升星/坐骑培养/坐骑放生/坐骑排行\n");
    out
}

// ═══════════════════════════════════════════════════
// 命令：坐骑列表
// ═══════════════════════════════════════════════════

pub fn cmd_mount_list(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let data = db.global_get(SECTION, user_id);
    let (owned, active, _) = parse_mount_data(&data);
    let stars_data = db.global_get(SECTION, &format!("stars_{}", user_id));
    let stars_map = parse_mount_stars(&stars_data);
    let owned_set: std::collections::HashSet<String> = owned.iter().cloned().collect();

    let filter = args.trim();
    let filter_quality = if !filter.is_empty() {
        MOUNT_QUALITIES.iter().position(|q| q.name == filter)
    } else {
        None
    };

    let mut out = String::from("══════ 📋 坐骑列表 ══════\n");

    for (qi, q) in MOUNT_QUALITIES.iter().enumerate() {
        let mounts: Vec<&MountDef> = MOUNT_DEFS.iter().filter(|m| m.quality == qi).collect();
        if mounts.is_empty() {
            continue;
        }
        if let Some(fq) = filter_quality {
            if fq != qi {
                continue;
            }
        }
        out.push_str(&format!(
            "\n{} {}品质 (×{:.1}倍率)\n",
            q.emoji, q.name, q.stat_multiplier
        ));
        for m in mounts {
            let owned_mark = if owned_set.contains(m.id) { "✅" } else { "❌" };
            let active_mark = if active == m.id { " 🏇骑乘中" } else { "" };
            let stars = stars_map.get(m.id).copied().unwrap_or(0);
            let star_str = if stars > 0 {
                format!(" ⭐{}", stars)
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  {} {} {}{}{}\n",
                owned_mark, m.emoji, m.name, star_str, active_mark
            ));
            if owned_set.contains(m.id) {
                out.push_str(&format!(
                    "    HP+{} AD+{} AP+{} DEF+{} MRES+{} SPD+{}%\n",
                    m.base_hp, m.base_ad, m.base_ap, m.base_def, m.base_mres, m.speed_bonus
                ));
            } else {
                out.push_str(&format!("    费用: {} 金币 | {}\n", q.unlock_cost_gold, m.description));
            }
        }
    }
    if filter_quality.is_some() && !out.contains("✅") && !out.contains("❌") {
        out.push_str("无该品质坐骑\n");
    }
    out
}

// ═══════════════════════════════════════════════════
// 命令：购买坐骑
// ═══════════════════════════════════════════════════

pub fn cmd_buy_mount(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let mount_id = args.trim();
    if mount_id.is_empty() {
        return "格式: 购买坐骑 [坐骑ID]\n使用「坐骑列表」查看可购买坐骑\nID示例: brown_horse, flame_steed, phoenix"
            .to_string();
    }

    let m = match find_mount(mount_id) {
        Some(m) => m,
        None => return format!("未找到坐骑「{}」，请检查ID", mount_id),
    };

    let q = &MOUNT_QUALITIES[m.quality];

    let data = db.global_get(SECTION, user_id);
    let (mut owned, active, feed) = parse_mount_data(&data);
    let owned_set: std::collections::HashSet<String> = owned.iter().cloned().collect();

    if owned_set.contains(mount_id) {
        return format!("你已拥有坐骑「{} {}」", m.emoji, m.name);
    }

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < q.unlock_cost_gold {
        return format!("金币不足！需要 {} 金币，当前 {}", q.unlock_cost_gold, gold);
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, q.unlock_cost_gold);

    owned.push(mount_id.to_string());
    let new_active = if active.is_empty() || active == "none" {
        mount_id.to_string()
    } else {
        active
    };
    let new_data = serialize_mount_data(&owned, &new_active, feed);
    db.global_set(SECTION, user_id, &new_data);

    // 初始化星级
    let stars_data = db.global_get(SECTION, &format!("stars_{}", user_id));
    let mut stars_map = parse_mount_stars(&stars_data);
    stars_map.entry(mount_id.to_string()).or_insert(0);
    db.global_set(
        SECTION,
        &format!("stars_{}", user_id),
        &serialize_mount_stars(&stars_map),
    );

    let mut out = "══════ 🎉 购买成功！═══════\n".to_string();
    out.push_str(&format!("获得坐骑: {} {} [{}{}]\n", m.emoji, m.name, q.emoji, q.name));
    out.push_str(&format!("消耗: {} 金币\n", q.unlock_cost_gold));
    if new_active == mount_id {
        out.push_str("已自动设为骑乘坐骑！\n");
    }
    out.push_str(&format!("技能: {} — {}\n", m.skill_name, m.skill_desc));
    out
}

// ═══════════════════════════════════════════════════
// 命令：骑乘坐骑
// ═══════════════════════════════════════════════════

#[allow(dead_code)]
pub fn cmd_ride_mount(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let mount_id = args.trim();
    if mount_id.is_empty() {
        return "格式: 骑乘 [坐骑ID]".to_string();
    }

    let m = match find_mount(mount_id) {
        Some(m) => m,
        None => return format!("未找到坐骑「{}」", mount_id),
    };

    let data = db.global_get(SECTION, user_id);
    let (owned, _active, feed) = parse_mount_data(&data);
    let owned_set: std::collections::HashSet<String> = owned.iter().cloned().collect();

    if !owned_set.contains(mount_id) {
        return format!("你未拥有坐骑「{}」，请先购买", m.name);
    }

    let new_data = serialize_mount_data(&owned, mount_id, feed);
    db.global_set(SECTION, user_id, &new_data);

    let q = &MOUNT_QUALITIES[m.quality];
    let mut out = "══════ 🏇 骑乘成功！═══════\n".to_string();
    out.push_str(&format!("当前骑乘: {} {} [{}{}]\n", m.emoji, m.name, q.emoji, q.name));
    out.push_str(&format!("速度加成: +{}%\n", m.speed_bonus));
    out.push_str(&format!("技能: {}\n", m.skill_name));
    let (hp, ad, ap, def, mres) = get_mount_bonus(db, user_id);
    out.push_str(&format!(
        "属性加成: HP+{} AD+{} AP+{} DEF+{} MRES+{}\n",
        hp, ad, ap, def, mres
    ));
    out
}

// ═══════════════════════════════════════════════════
// 命令：坐骑升星
// ═══════════════════════════════════════════════════

pub fn cmd_mount_upgrade(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let mount_id = args.trim();
    if mount_id.is_empty() {
        return "格式: 坐骑升星 [坐骑ID]".to_string();
    }

    let m = match find_mount(mount_id) {
        Some(m) => m,
        None => return format!("未找到坐骑「{}」", mount_id),
    };

    let q = &MOUNT_QUALITIES[m.quality];

    let data = db.global_get(SECTION, user_id);
    let (owned, active, feed) = parse_mount_data(&data);
    let owned_set: std::collections::HashSet<String> = owned.iter().cloned().collect();

    if !owned_set.contains(mount_id) {
        return format!("你未拥有坐骑「{}」", m.name);
    }

    let stars_data = db.global_get(SECTION, &format!("stars_{}", user_id));
    let mut stars_map = parse_mount_stars(&stars_data);
    let current_stars = stars_map.get(mount_id).copied().unwrap_or(0);

    if current_stars >= q.max_stars {
        return format!(
            "{} {} 已达到最高星级 ({}/{})",
            m.emoji, m.name, current_stars, q.max_stars
        );
    }

    let cost = star_upgrade_cost(current_stars);
    let feed_needed = 10 + current_stars * 5;
    let gold = db.read_currency(user_id, CURRENCY_GOLD);

    if gold < cost {
        return format!("金币不足！升星需要 {} 金币，当前 {}", cost, gold);
    }
    if feed < feed_needed {
        return format!(
            "培养值不足！升星需要 {} 培养值，当前 {}。使用「坐骑培养」获取培养值",
            feed_needed, feed
        );
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, cost);
    let new_stars = current_stars + 1;
    stars_map.insert(mount_id.to_string(), new_stars);
    db.global_set(
        SECTION,
        &format!("stars_{}", user_id),
        &serialize_mount_stars(&stars_map),
    );
    db.global_set(
        SECTION,
        user_id,
        &serialize_mount_data(&owned, &active, feed - feed_needed),
    );

    let new_hp = (m.base_hp as f64 * q.stat_multiplier * (1.0 + new_stars as f64 * 0.1)) as i32;
    let new_ad = (m.base_ad as f64 * q.stat_multiplier * (1.0 + new_stars as f64 * 0.1)) as i32;
    let power = mount_power(m, new_stars);

    let mut out = "══════ ⭐ 升星成功！═══════\n".to_string();
    out.push_str(&format!(
        "{} {} {} → {}\n",
        m.emoji,
        m.name,
        "⭐".repeat(current_stars as usize),
        "⭐".repeat(new_stars as usize)
    ));
    out.push_str(&format!("消耗: {} 金币 + {} 培养值\n", cost, feed_needed));
    out.push_str(&format!(
        "HP加成: {} → {}\n",
        (m.base_hp as f64 * q.stat_multiplier * (1.0 + current_stars as f64 * 0.1)) as i32,
        new_hp
    ));
    out.push_str(&format!(
        "AD加成: {} → {}\n",
        (m.base_ad as f64 * q.stat_multiplier * (1.0 + current_stars as f64 * 0.1)) as i32,
        new_ad
    ));
    out.push_str(&format!("坐骑战力: {}\n", power));
    out
}

// ═══════════════════════════════════════════════════
// 命令：坐骑培养
// ═══════════════════════════════════════════════════

pub fn cmd_mount_feed(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let daily_key = format!("{}_{}", user_id, today_str());
    let daily_data = db.global_get(SECTION_DAILY, &daily_key);
    let feeds_today: i32 = daily_data.parse::<i32>().unwrap_or(0);

    if feeds_today >= 10 {
        return "今日培养次数已达上限(10次)，请明天再来".to_string();
    }

    let gold_cost = 2000 + feeds_today as i64 * 1000;
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < gold_cost {
        return format!("金币不足！培养需要 {} 金币，当前 {}", gold_cost, gold);
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_cost);

    let data = db.global_get(SECTION, user_id);
    let (owned, active, feed) = parse_mount_data(&data);

    // 培养值 = 基础20 + 随机10-30
    let base_gain = 20i32;
    let extra = (feeds_today as i32 % 21) + 10; // pseudo-random 10-30
    let gain = base_gain + extra;

    db.global_set(SECTION, user_id, &serialize_mount_data(&owned, &active, feed + gain));
    db.global_set(SECTION_DAILY, &daily_key, &(feeds_today + 1).to_string());

    let mut out = "══════ 🌿 培养成功！═══════\n".to_string();
    out.push_str(&format!("培养值: {} → {}\n", feed, feed + gain));
    out.push_str(&format!("消耗: {} 金币 (第{}/10次)\n", gold_cost, feeds_today + 1));
    out.push_str("培养值可用于「坐骑升星」\n");
    out
}

// ═══════════════════════════════════════════════════
// 命令：坐骑放生
// ═══════════════════════════════════════════════════

pub fn cmd_mount_release(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let mount_id = args.trim();
    if mount_id.is_empty() {
        return "格式: 坐骑放生 [坐骑ID] — 放生坐骑返还50%购买费用".to_string();
    }

    let m = match find_mount(mount_id) {
        Some(m) => m,
        None => return format!("未找到坐骑「{}」", mount_id),
    };

    let data = db.global_get(SECTION, user_id);
    let (owned, active, feed) = parse_mount_data(&data);
    let owned_set: std::collections::HashSet<String> = owned.iter().cloned().collect();

    if !owned_set.contains(mount_id) {
        return format!("你未拥有坐骑「{}」", m.name);
    }

    if active == mount_id {
        return format!("无法放生当前骑乘的坐骑「{}」，请先切换坐骑", m.name);
    }

    let q = &MOUNT_QUALITIES[m.quality];
    let refund = q.unlock_cost_gold / 2;
    db.modify_currency(user_id, CURRENCY_GOLD, crate::core::OP_ADD, refund);

    let new_owned: Vec<String> = owned.into_iter().filter(|id| id != mount_id).collect();
    db.global_set(SECTION, user_id, &serialize_mount_data(&new_owned, &active, feed));

    // 清除星级
    let stars_data = db.global_get(SECTION, &format!("stars_{}", user_id));
    let mut stars_map = parse_mount_stars(&stars_data);
    stars_map.remove(mount_id);
    db.global_set(
        SECTION,
        &format!("stars_{}", user_id),
        &serialize_mount_stars(&stars_map),
    );

    let mut out = "══════ 🍃 放生成功 ══════\n".to_string();
    out.push_str(&format!("放生坐骑: {} {}\n", m.emoji, m.name));
    out.push_str(&format!("返还: {} 金币 (50%购买费用)\n", refund));
    out
}

// ═══════════════════════════════════════════════════
// 命令：坐骑排行
// ═══════════════════════════════════════════════════

#[allow(dead_code)]
pub fn cmd_mount_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_users = db.all_users();
    let mut rankings: Vec<(String, String, i64, String)> = Vec::new();

    for uid in &all_users {
        let data = db.global_get(SECTION, uid);
        let (_, active, _) = parse_mount_data(&data);
        if active.is_empty() || active == "none" {
            continue;
        }

        let stars_data = db.global_get(SECTION, &format!("stars_{}", uid));
        let stars_map = parse_mount_stars(&stars_data);
        let stars = stars_map.get(&active).copied().unwrap_or(0);

        if let Some(m) = find_mount(&active) {
            let power = mount_power(m, stars);
            let nickname = db.read_basic(uid, "Nickname");
            let nick = if nickname == "[NULL]" || nickname.is_empty() {
                uid.clone()
            } else {
                nickname
            };
            rankings.push((uid.clone(), nick, power, format!("{} {}", m.emoji, m.name)));
        }
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.2));

    let mut out = String::from("══════ 🏆 坐骑排行 🏆 ══════\n");
    let limit = rankings.len().min(15);
    for (i, (_, nick, power, mount_name)) in rankings.iter().take(limit).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        out.push_str(&format!(
            "{} {} {} — {} (战力:{})\n",
            medal,
            i + 1,
            nick,
            mount_name,
            power
        ));
    }
    if rankings.is_empty() {
        out.push_str("暂无坐骑排行数据\n");
    }
    out
}

// ═══════════════════════════════════════════════════
// 命令：坐骑帮助
// ═══════════════════════════════════════════════════

pub fn cmd_mount_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("══════ 🐴 坐骑系统帮助 ══════\n");
    out.push_str("▸ 坐骑列表 — 查看所有坐骑(可按品质筛选)\n");
    out.push_str("▸ 购买坐骑 [ID] — 购买坐骑(消耗金币)\n");
    out.push_str("▸ 骑乘 [ID] — 切换骑乘坐骑\n");
    out.push_str("▸ 坐骑升星 [ID] — 消耗金币+培养值提升坐骑星级\n");
    out.push_str("▸ 坐骑培养 — 消耗金币获取培养值(每日10次)\n");
    out.push_str("▸ 坐骑放生 [ID] — 放生坐骑，返还50%金币\n");
    out.push_str("▸ 坐骑排行 — 全服坐骑战力排行\n");
    out.push('\n');
    out.push_str("══════ 品质体系 ══════\n");
    for q in MOUNT_QUALITIES {
        out.push_str(&format!(
            "  {} {} ×{:.1}倍率 最高{}星 购买费:{}金\n",
            q.emoji, q.name, q.stat_multiplier, q.max_stars, q.unlock_cost_gold
        ));
    }
    out.push('\n');
    out.push_str("══════ 系统说明 ══════\n");
    out.push_str("• 购买坐骑自动设为骑乘，骑乘即获得属性加成\n");
    out.push_str("• 升星需消耗金币+培养值，培养通过「坐骑培养」获取\n");
    out.push_str("• 每种坐骑拥有独特战斗技能，骑乘后自动生效\n");
    out.push_str("• 放生坐骑返还50%金币，但星级清零\n");
    out.push_str("• 坐骑属性加成自动集成到战斗系统\n");
    out
}

// ═══════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_count() {
        assert_eq!(MOUNT_DEFS.len(), 14);
    }

    #[test]
    fn test_quality_count() {
        assert_eq!(MOUNT_QUALITIES.len(), 5);
    }

    #[test]
    fn test_find_mount() {
        assert!(find_mount("phoenix").is_some());
        assert!(find_mount("nonexistent").is_none());
    }

    #[test]
    fn test_mount_power() {
        let phoenix = find_mount("phoenix").unwrap();
        let power_0 = mount_power(phoenix, 0);
        let power_5 = mount_power(phoenix, 5);
        assert!(power_5 > power_0);
    }

    #[test]
    fn test_star_upgrade_cost() {
        assert!(star_upgrade_cost(0) < star_upgrade_cost(5));
        assert!(star_upgrade_cost(5) < star_upgrade_cost(9));
    }

    #[test]
    fn test_serialize_deserialize() {
        let owned = vec!["phoenix".to_string(), "brown_horse".to_string()];
        let data = serialize_mount_data(&owned, "phoenix", 100);
        let (o2, a2, f2) = parse_mount_data(&data);
        assert_eq!(o2.len(), 2);
        assert_eq!(a2, "phoenix");
        assert_eq!(f2, 100);
    }

    #[test]
    fn test_serialize_empty() {
        let data = serialize_mount_data(&[], "none", 0);
        let (o, a, f) = parse_mount_data(&data);
        assert!(o.is_empty());
        assert_eq!(a, "none");
        assert_eq!(f, 0);
    }

    #[test]
    fn test_stars_serialize() {
        let mut map = std::collections::HashMap::new();
        map.insert("phoenix".to_string(), 3);
        map.insert("brown_horse".to_string(), 1);
        let s = serialize_mount_stars(&map);
        let m2 = parse_mount_stars(&s);
        assert_eq!(m2.get("phoenix"), Some(&3));
        assert_eq!(m2.get("brown_horse"), Some(&1));
    }

    #[test]
    fn test_mount_quality_order() {
        for (i, q) in MOUNT_QUALITIES.iter().enumerate() {
            if i > 0 {
                assert!(q.stat_multiplier > MOUNT_QUALITIES[i - 1].stat_multiplier);
            }
        }
    }

    #[test]
    fn test_today_str() {
        let d = today_str();
        assert!(d.starts_with("day_"));
    }

    #[test]
    fn test_mount_unique_ids() {
        let mut ids: Vec<&str> = MOUNT_DEFS.iter().map(|m| m.id).collect();
        let len_before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len_before);
    }

    #[test]
    fn test_mount_quality_indices() {
        for m in MOUNT_DEFS {
            assert!(
                m.quality < MOUNT_QUALITIES.len(),
                "mount {} has invalid quality index",
                m.id
            );
        }
    }

    #[test]
    fn test_mount_base_stats_positive() {
        for m in MOUNT_DEFS {
            assert!(m.base_hp > 0, "mount {} has 0 hp", m.id);
            assert!(m.speed_bonus > 0, "mount {} has 0 speed", m.id);
        }
    }
}

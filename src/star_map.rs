//! 星图系统 (Star Map / Constellation System)
//!
//! A celestial progression system where players collect star fragments from
//! various activities to unlock 12 zodiac constellations. Each constellation
//! has 5 stars to light up, and fully completing a constellation grants
//! permanent passive attribute bonuses.
//!
//! Data storage: Global表 SECTION='star_map'

use crate::db::Database;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SECTION: &str = "star_map";

const ZODIAC_COUNT: usize = 12;
const STARS_PER_CONSTELLATION: usize = 5;

const STAR_NAMES: [&str; STARS_PER_CONSTELLATION] = ["α", "β", "γ", "δ", "ε"];

const STAR_FRAGMENT_COST: [i64; STARS_PER_CONSTELLATION] = [10, 25, 50, 100, 200];

const DAILY_FRAGMENT_CAP: i64 = 500;

struct ZodiacDef {
    name: &'static str,
    emoji: &'static str,
    element: &'static str,
    hp_bonus: i64,
    ad_bonus: i64,
    ap_bonus: i64,
    def_bonus: i64,
    mres_bonus: i64,
}

const ZODIAC: [ZodiacDef; ZODIAC_COUNT] = [
    ZodiacDef {
        name: "白羊座",
        emoji: "♈",
        element: "火",
        hp_bonus: 200,
        ad_bonus: 30,
        ap_bonus: 30,
        def_bonus: 15,
        mres_bonus: 15,
    },
    ZodiacDef {
        name: "金牛座",
        emoji: "♉",
        element: "地",
        hp_bonus: 300,
        ad_bonus: 20,
        ap_bonus: 20,
        def_bonus: 25,
        mres_bonus: 25,
    },
    ZodiacDef {
        name: "双子座",
        emoji: "♊",
        element: "风",
        hp_bonus: 150,
        ad_bonus: 25,
        ap_bonus: 40,
        def_bonus: 10,
        mres_bonus: 20,
    },
    ZodiacDef {
        name: "巨蟹座",
        emoji: "♋",
        element: "水",
        hp_bonus: 400,
        ad_bonus: 15,
        ap_bonus: 15,
        def_bonus: 30,
        mres_bonus: 30,
    },
    ZodiacDef {
        name: "狮子座",
        emoji: "♌",
        element: "火",
        hp_bonus: 250,
        ad_bonus: 50,
        ap_bonus: 20,
        def_bonus: 20,
        mres_bonus: 10,
    },
    ZodiacDef {
        name: "处女座",
        emoji: "♍",
        element: "地",
        hp_bonus: 200,
        ad_bonus: 20,
        ap_bonus: 50,
        def_bonus: 15,
        mres_bonus: 25,
    },
    ZodiacDef {
        name: "天秤座",
        emoji: "♎",
        element: "风",
        hp_bonus: 180,
        ad_bonus: 35,
        ap_bonus: 35,
        def_bonus: 20,
        mres_bonus: 20,
    },
    ZodiacDef {
        name: "天蝎座",
        emoji: "♏",
        element: "水",
        hp_bonus: 220,
        ad_bonus: 45,
        ap_bonus: 30,
        def_bonus: 18,
        mres_bonus: 18,
    },
    ZodiacDef {
        name: "射手座",
        emoji: "♐",
        element: "火",
        hp_bonus: 160,
        ad_bonus: 40,
        ap_bonus: 25,
        def_bonus: 12,
        mres_bonus: 15,
    },
    ZodiacDef {
        name: "摩羯座",
        emoji: "♑",
        element: "地",
        hp_bonus: 350,
        ad_bonus: 30,
        ap_bonus: 20,
        def_bonus: 35,
        mres_bonus: 30,
    },
    ZodiacDef {
        name: "水瓶座",
        emoji: "♒",
        element: "风",
        hp_bonus: 180,
        ad_bonus: 20,
        ap_bonus: 45,
        def_bonus: 18,
        mres_bonus: 28,
    },
    ZodiacDef {
        name: "双鱼座",
        emoji: "♓",
        element: "水",
        hp_bonus: 280,
        ad_bonus: 25,
        ap_bonus: 35,
        def_bonus: 22,
        mres_bonus: 35,
    },
];

struct StarShopItem {
    name: &'static str,
    cost: i64,
    description: &'static str,
    emoji: &'static str,
}

const SHOP_ITEMS: [StarShopItem; 8] = [
    StarShopItem {
        name: "星辉药水",
        cost: 50,
        description: "恢复50%HP",
        emoji: "🧪",
    },
    StarShopItem {
        name: "星辰强化石",
        cost: 120,
        description: "强化装备+1",
        emoji: "💎",
    },
    StarShopItem {
        name: "星尘精华",
        cost: 200,
        description: "合成材料",
        emoji: "✨",
    },
    StarShopItem {
        name: "星座护符",
        cost: 350,
        description: "临时+5%全属性(1h)",
        emoji: "🔮",
    },
    StarShopItem {
        name: "流星碎片",
        cost: 500,
        description: "用于星座突破",
        emoji: "☄️",
    },
    StarShopItem {
        name: "银河钥匙",
        cost: 800,
        description: "开启银河宝箱",
        emoji: "🗝️",
    },
    StarShopItem {
        name: "星图卷轴",
        cost: 1200,
        description: "查看隐藏星座",
        emoji: "📜",
    },
    StarShopItem {
        name: "星辰至尊宝箱",
        cost: 2000,
        description: "随机传说道具",
        emoji: "🌟",
    },
];

struct FragmentSource {
    name: &'static str,
    base_amount: i64,
    emoji: &'static str,
}

const FRAGMENT_SOURCES: [FragmentSource; 8] = [
    FragmentSource {
        name: "击败BOSS",
        base_amount: 15,
        emoji: "👹",
    },
    FragmentSource {
        name: "PvP胜利",
        base_amount: 10,
        emoji: "⚔️",
    },
    FragmentSource {
        name: "深渊通关",
        base_amount: 12,
        emoji: "🕳️",
    },
    FragmentSource {
        name: "完成任务",
        base_amount: 8,
        emoji: "📋",
    },
    FragmentSource {
        name: "每日签到",
        base_amount: 5,
        emoji: "📅",
    },
    FragmentSource {
        name: "合成成功",
        base_amount: 3,
        emoji: "🔨",
    },
    FragmentSource {
        name: "采矿采集",
        base_amount: 6,
        emoji: "⛏️",
    },
    FragmentSource {
        name: "竞技场胜利",
        base_amount: 10,
        emoji: "🏟️",
    },
];

// Zodiac all-complete ultimate bonus
const ZODIAC_ALL_COMPLETE_HP: i64 = 1000;
const ZODIAC_ALL_COMPLETE_AD: i64 = 200;
const ZODIAC_ALL_COMPLETE_AP: i64 = 200;
const ZODIAC_ALL_COMPLETE_DEF: i64 = 100;
const ZODIAC_ALL_COMPLETE_MRES: i64 = 100;

// ---------------------------------------------------------------------------
// Data helpers — stored as individual Global keys for simplicity
// ---------------------------------------------------------------------------

/// Get user's current fragment count
fn get_fragments(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("fragments:{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// Set user's fragment count
fn set_fragments(db: &Database, user_id: &str, val: i64) {
    db.global_set(SECTION, &format!("fragments:{}", user_id), &val.to_string());
}

/// Get today's earned fragments
#[allow(dead_code)]
fn get_today_fragments(db: &Database, user_id: &str) -> i64 {
    let today = utils::chrono_now_date();
    let saved = db.global_get(SECTION, &format!("today_date:{}", user_id));
    if saved != today {
        return 0;
    }
    db.global_get(SECTION, &format!("today_frags:{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// Add to today's earned fragments
#[allow(dead_code)]
fn add_today_fragments(db: &Database, user_id: &str, amount: i64) {
    let today = utils::chrono_now_date();
    let cur = get_today_fragments(db, user_id);
    db.global_set(SECTION, &format!("today_date:{}", user_id), &today);
    db.global_set(
        SECTION,
        &format!("today_frags:{}", user_id),
        &(cur + amount).to_string(),
    );
}

/// Get total fragments ever earned (for ranking)
fn get_total_fragments_earned(db: &Database, user_id: &str) -> i64 {
    db.global_get(SECTION, &format!("total_earned:{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// Add to total fragments earned
#[allow(dead_code)]
fn add_total_fragments_earned(db: &Database, user_id: &str, amount: i64) {
    let cur = get_total_fragments_earned(db, user_id);
    db.global_set(
        SECTION,
        &format!("total_earned:{}", user_id),
        &(cur + amount).to_string(),
    );
}

/// Get constellation bitmask (12 constellations, stored as "b0,b1,...,b11")
fn get_constellations(db: &Database, user_id: &str) -> [u8; ZODIAC_COUNT] {
    let data = db.global_get(SECTION, &format!("constellations:{}", user_id));
    if data.is_empty() {
        return [0u8; ZODIAC_COUNT];
    }
    let mut result = [0u8; ZODIAC_COUNT];
    for (i, part) in data.split(',').enumerate() {
        if i < ZODIAC_COUNT {
            result[i] = part.parse().unwrap_or(0);
        }
    }
    result
}

/// Save constellation bitmask
fn set_constellations(db: &Database, user_id: &str, data: &[u8; ZODIAC_COUNT]) {
    let s: Vec<String> = data.iter().map(|b| b.to_string()).collect();
    db.global_set(SECTION, &format!("constellations:{}", user_id), &s.join(","));
}

/// Get shop purchases today
fn get_shop_today(db: &Database, user_id: &str) -> u32 {
    let today = utils::chrono_now_date();
    let saved = db.global_get(SECTION, &format!("shop_date:{}", user_id));
    if saved != today {
        return 0;
    }
    db.global_get(SECTION, &format!("shop_today:{}", user_id))
        .parse()
        .unwrap_or(0)
}

/// Increment shop purchases today
fn add_shop_today(db: &Database, user_id: &str) {
    let today = utils::chrono_now_date();
    let cur = get_shop_today(db, user_id);
    db.global_set(SECTION, &format!("shop_date:{}", user_id), &today);
    db.global_set(SECTION, &format!("shop_today:{}", user_id), &(cur + 1).to_string());
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

fn count_stars(bitmask: u8) -> u8 {
    bitmask.count_ones() as u8
}

fn is_constellation_complete(bitmask: u8) -> bool {
    bitmask & 0x1F == 0x1F
}

fn total_stars(data: &[u8; ZODIAC_COUNT]) -> u32 {
    data.iter().map(|&b| count_stars(b) as u32).sum()
}

fn count_completed(data: &[u8; ZODIAC_COUNT]) -> u8 {
    data.iter().filter(|&&b| is_constellation_complete(b)).count() as u8
}

/// Calculate passive bonuses from star map (for external integration)
pub fn get_star_map_bonus(db: &Database, user_id: &str) -> (i64, i64, i64, i64, i64) {
    let data = get_constellations(db, user_id);
    let mut hp = 0i64;
    let mut ad = 0i64;
    let mut ap = 0i64;
    let mut def = 0i64;
    let mut mres = 0i64;

    for (i, &bitmask) in data.iter().enumerate() {
        let stars = count_stars(bitmask);
        if stars > 0 {
            let z = &ZODIAC[i];
            let ratio = stars as f64 / STARS_PER_CONSTELLATION as f64;
            hp += (z.hp_bonus as f64 * ratio) as i64;
            ad += (z.ad_bonus as f64 * ratio) as i64;
            ap += (z.ap_bonus as f64 * ratio) as i64;
            def += (z.def_bonus as f64 * ratio) as i64;
            mres += (z.mres_bonus as f64 * ratio) as i64;
        }
    }

    if count_completed(&data) as usize >= ZODIAC_COUNT {
        hp += ZODIAC_ALL_COMPLETE_HP;
        ad += ZODIAC_ALL_COMPLETE_AD;
        ap += ZODIAC_ALL_COMPLETE_AP;
        def += ZODIAC_ALL_COMPLETE_DEF;
        mres += ZODIAC_ALL_COMPLETE_MRES;
    }

    (hp, ad, ap, def, mres)
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// 星图 — View star map overview
pub fn cmd_star_map(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let data = get_constellations(db, user_id);
    let fragments = get_fragments(db, user_id);
    let today_frags = get_today_fragments(db, user_id);
    let completed = count_completed(&data);
    let stars_total = total_stars(&data);

    let mut lines = vec![
        "═══════════════════════════════════".to_string(),
        "⭐ 星 图 系 统 ⭐".to_string(),
        "═══════════════════════════════════".to_string(),
        format!(
            "✨ 星辉碎片: {} (今日: {}/{})",
            fragments, today_frags, DAILY_FRAGMENT_CAP
        ),
        format!("🏆 已完成星座: {}/{}", completed, ZODIAC_COUNT),
        format!(
            "⭐ 已点亮星辰: {}/{}",
            stars_total,
            ZODIAC_COUNT * STARS_PER_CONSTELLATION
        ),
        "".to_string(),
        "── 十二星座 ──".to_string(),
    ];

    for (i, z) in ZODIAC.iter().enumerate() {
        let bitmask = data[i];
        let stars = count_stars(bitmask);
        let complete = is_constellation_complete(bitmask);
        let status = if complete { "✅" } else { "" };
        let stars_display: String = (0..STARS_PER_CONSTELLATION)
            .map(|s| if bitmask & (1 << s) != 0 { "⭐" } else { "☆" })
            .collect::<Vec<_>>()
            .join("");
        let bonus_str = if complete {
            format!(
                "HP+{} AD+{} AP+{} DEF+{} MRES+{}",
                z.hp_bonus, z.ad_bonus, z.ap_bonus, z.def_bonus, z.mres_bonus
            )
        } else {
            String::new()
        };
        lines.push(format!(
            "{} {} {} ({}) {} {}/5 {}",
            z.emoji, z.name, stars_display, z.element, status, stars, bonus_str
        ));
    }

    let (hp, ad, ap, def, mres) = get_star_map_bonus(db, user_id);
    lines.push("".to_string());
    lines.push("── 总属性加成 ──".to_string());
    lines.push(format!(
        "❤️ HP+{}  ⚔️ AD+{}  🔮 AP+{}  🛡️ DEF+{}  🔰 MRES+{}",
        hp, ad, ap, def, mres
    ));

    if completed as usize >= ZODIAC_COUNT {
        lines.push("".to_string());
        lines.push("🌟✨ 全星座觉醒 ✨🌟".to_string());
        lines.push(format!(
            "🎊 额外加成: HP+{} AD+{} AP+{} DEF+{} MRES+{}",
            ZODIAC_ALL_COMPLETE_HP,
            ZODIAC_ALL_COMPLETE_AD,
            ZODIAC_ALL_COMPLETE_AP,
            ZODIAC_ALL_COMPLETE_DEF,
            ZODIAC_ALL_COMPLETE_MRES
        ));
    }

    lines.push("".to_string());
    lines.push("指令: 点亮星座 <序号> | 星辉商店 | 星辉兑换 <序号> | 星图排行 | 星图帮助".to_string());

    lines.join("\n")
}

/// 点亮星座 — Light next star in a constellation
pub fn cmd_light_star(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if (1..=ZODIAC_COUNT).contains(&n) => n - 1,
        _ => return format!("❌ 用法: 点亮星座 <序号(1-{})>", ZODIAC_COUNT),
    };

    let mut data = get_constellations(db, user_id);
    let bitmask = data[idx];

    if is_constellation_complete(bitmask) {
        return format!("❌ {}已全部点亮！", ZODIAC[idx].name);
    }

    // Find next star to light
    let next_star = (0..STARS_PER_CONSTELLATION).find(|&s| bitmask & (1 << s) == 0);
    let star_idx = match next_star {
        Some(s) => s,
        None => return "❌ 无法找到未点亮的星辰".to_string(),
    };

    let cost = STAR_FRAGMENT_COST[star_idx];
    let fragments = get_fragments(db, user_id);
    if fragments < cost {
        return format!("❌ 碎片不足！需要{}个星辉碎片，当前仅有{}", cost, fragments);
    }

    // Light the star
    set_fragments(db, user_id, fragments - cost);
    data[idx] |= 1 << star_idx;
    set_constellations(db, user_id, &data);

    let z = &ZODIAC[idx];
    let new_stars = count_stars(data[idx]);
    let complete = is_constellation_complete(data[idx]);
    let completed_count = count_completed(&data);

    let mut lines = vec![
        format!("{} 点亮 {} · {} 星成功！", z.emoji, z.name, STAR_NAMES[star_idx]),
        format!("✨ 消耗碎片: {} (剩余: {})", cost, fragments - cost),
        format!("⭐ 当前进度: {}/5 星", new_stars),
    ];

    if complete {
        lines.push("".to_string());
        lines.push(format!("🎉 {} 全部点亮！星座觉醒！", z.name));
        lines.push(format!(
            "❤️ HP+{} ⚔️ AD+{} 🔮 AP+{} 🛡️ DEF+{} 🔰 MRES+{}",
            z.hp_bonus, z.ad_bonus, z.ap_bonus, z.def_bonus, z.mres_bonus
        ));

        if completed_count as usize >= ZODIAC_COUNT {
            lines.push("".to_string());
            lines.push("🌟✨ 十二星座全部觉醒！获得终极星图之力！✨🌟".to_string());
        }
    } else {
        let next_cost = STAR_FRAGMENT_COST
            .iter()
            .enumerate()
            .find(|(s, _)| data[idx] & (1 << s) == 0)
            .map(|(_, &c)| c)
            .unwrap_or(0);
        lines.push(format!("💡 下一颗星辰需要 {} 碎片", next_cost));
    }

    lines.join("\n")
}

/// 星辉商店 — View star fragment shop
pub fn cmd_star_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let fragments = get_fragments(db, user_id);
    let shop_today = get_shop_today(db, user_id);

    let mut lines = vec![
        "═══════════════════════════════════".to_string(),
        "🛒 星辉商店 🛒".to_string(),
        "═══════════════════════════════════".to_string(),
        format!("✨ 当前碎片: {} (今日购买: {}/5)", fragments, shop_today),
        "".to_string(),
    ];

    for (i, item) in SHOP_ITEMS.iter().enumerate() {
        let affordable = if fragments >= item.cost { "✅" } else { "❌" };
        lines.push(format!(
            "{}. {} {} — {}碎片 ({}) {}",
            i + 1,
            item.emoji,
            item.name,
            item.cost,
            item.description,
            affordable
        ));
    }

    lines.push("".to_string());
    lines.push("指令: 星辉兑换 <序号(1-8)>".to_string());

    lines.join("\n")
}

/// 星辉兑换 — Purchase from star shop
pub fn cmd_star_buy(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if (1..=SHOP_ITEMS.len()).contains(&n) => n - 1,
        _ => return format!("❌ 用法: 星辉兑换 <序号(1-{})>", SHOP_ITEMS.len()),
    };

    let shop_today = get_shop_today(db, user_id);
    if shop_today >= 5 {
        return "❌ 今日购买次数已达上限(5次)".to_string();
    }

    let item = &SHOP_ITEMS[idx];
    let fragments = get_fragments(db, user_id);
    if fragments < item.cost {
        return format!("❌ 碎片不足！需要{}碎片，当前{}", item.cost, fragments);
    }

    set_fragments(db, user_id, fragments - item.cost);
    add_shop_today(db, user_id);

    format!(
        "🛒 兑换成功！\n{} {} — {}\n✨ 消耗碎片: {} (剩余: {})\n📦 今日已购买: {}/5",
        item.emoji,
        item.name,
        item.description,
        item.cost,
        fragments - item.cost,
        shop_today + 1
    )
}

/// 星图排行 — Star map leaderboard
pub fn cmd_star_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let all_users = db.all_users();
    let mut rankings: Vec<(String, i64, u8, u32)> = Vec::new();
    for uid in &all_users {
        let total = get_total_fragments_earned(db, uid);
        if total > 0 {
            let con = get_constellations(db, uid);
            let completed = count_completed(&con);
            let stars = total_stars(&con);
            let nickname = db.read_basic(uid, "Nickname");
            let display = if nickname == "[NULL]" { uid.clone() } else { nickname };
            rankings.push((display, total, completed, stars));
        }
    }
    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut lines = vec![
        "═══════════════════════════════════".to_string(),
        "🏆 星图排行榜 🏆".to_string(),
        "═══════════════════════════════════".to_string(),
        "".to_string(),
    ];

    if rankings.is_empty() {
        lines.push("暂无排行数据".to_string());
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (name, total, completed, stars)) in rankings.iter().take(15).enumerate() {
            let medal = if i < 3 { medals[i] } else { "  " };
            lines.push(format!(
                "{} {}. {} — 星辉:{} 星座:{}/12 星:{}/60",
                medal,
                i + 1,
                name,
                total,
                completed,
                stars
            ));
        }
    }

    // Find player rank
    let my_total = get_total_fragments_earned(db, user_id);
    if my_total > 0 {
        let my_rank = rankings.iter().position(|(_, t, _, _)| *t == my_total);
        if let Some(rank) = my_rank {
            lines.push("".to_string());
            lines.push(format!("📍 你的排名: 第{}名", rank + 1));
        }
    }

    lines.join("\n")
}

/// 星图帮助 — Star map help
pub fn cmd_star_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut lines = vec![
        "═══════════════════════════════════".to_string(),
        "⭐ 星图系统帮助 ⭐".to_string(),
        "═══════════════════════════════════".to_string(),
        "".to_string(),
        "📖 系统介绍:".to_string(),
        "  星图系统让你通过收集星辉碎片，点亮十二星座，获得永久属性加成。".to_string(),
        "".to_string(),
        "🌟 星座系统:".to_string(),
        "  • 12个星座，每个有5颗星辰(α→ε)".to_string(),
        "  • 每颗星辰需要不同数量的碎片点亮".to_string(),
        "  • 点亮越多星辰，属性加成越高".to_string(),
        "  • 完整点亮一个星座=觉醒，获得全部加成".to_string(),
        "".to_string(),
        "✨ 碎片获取:".to_string(),
    ];

    for s in &FRAGMENT_SOURCES {
        lines.push(format!("  {} {} +{}", s.emoji, s.name, s.base_amount));
    }
    lines.push(format!("  📊 每日上限: {} 碎片", DAILY_FRAGMENT_CAP));

    lines.extend([
        "".to_string(),
        "🛒 星辉商店:".to_string(),
        "  • 8种珍贵商品，碎片兑换".to_string(),
        "  • 每日限购5次".to_string(),
        "".to_string(),
        "📋 指令列表:".to_string(),
        "  星图 — 查看星图总览".to_string(),
        "  点亮星座 <序号> — 点亮星座的下一颗星".to_string(),
        "  星辉商店 — 查看商店".to_string(),
        "  星辉兑换 <序号> — 购买商品".to_string(),
        "  星图排行 — 全服排行榜".to_string(),
        "  星图帮助 — 本帮助".to_string(),
    ]);

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// API for integration with other systems
// ---------------------------------------------------------------------------

/// Record fragment gain from an activity (call from combat/quest/etc.)
/// Returns actual fragments gained (0 if at cap).
#[allow(dead_code)]
pub fn record_fragment_gain(db: &Database, user_id: &str, source: &str, multiplier: f64) -> i64 {
    let base = FRAGMENT_SOURCES
        .iter()
        .find(|s| s.name == source)
        .map(|s| s.base_amount)
        .unwrap_or(5);

    let gain = ((base as f64 * multiplier) as i64).max(1);
    let today = get_today_fragments(db, user_id);
    let remaining = DAILY_FRAGMENT_CAP - today;
    let actual = gain.min(remaining).max(0);
    if actual == 0 {
        return 0;
    }

    let cur = get_fragments(db, user_id);
    set_fragments(db, user_id, cur + actual);
    add_today_fragments(db, user_id, actual);
    add_total_fragments_earned(db, user_id, actual);
    actual
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zodiac_count() {
        assert_eq!(ZODIAC_COUNT, 12);
        assert_eq!(ZODIAC.len(), 12);
    }

    #[test]
    fn test_zodiac_unique_names() {
        let mut names: Vec<&str> = ZODIAC.iter().map(|z| z.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 12);
    }

    #[test]
    fn test_star_fragment_costs_escalate() {
        for i in 1..STAR_FRAGMENT_COST.len() {
            assert!(
                STAR_FRAGMENT_COST[i] > STAR_FRAGMENT_COST[i - 1],
                "Star {} cost should be > star {}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_count_stars() {
        assert_eq!(count_stars(0b00000), 0);
        assert_eq!(count_stars(0b00001), 1);
        assert_eq!(count_stars(0b01010), 2);
        assert_eq!(count_stars(0b11111), 5);
        assert_eq!(count_stars(0b11110), 4);
    }

    #[test]
    fn test_is_constellation_complete() {
        assert!(!is_constellation_complete(0b00000));
        assert!(!is_constellation_complete(0b11110));
        assert!(is_constellation_complete(0b11111));
        assert!(is_constellation_complete(0b111111)); // extra bits ignored
    }

    #[test]
    fn test_total_stars() {
        let mut data = [0u8; ZODIAC_COUNT];
        data[0] = 0b00111; // 3 stars
        data[1] = 0b11111; // 5 stars
        data[5] = 0b00001; // 1 star
        assert_eq!(total_stars(&data), 9);
    }

    #[test]
    fn test_count_completed() {
        let mut data = [0u8; ZODIAC_COUNT];
        assert_eq!(count_completed(&data), 0);
        data[0] = 0b11111;
        assert_eq!(count_completed(&data), 1);
        data[3] = 0b11111;
        assert_eq!(count_completed(&data), 2);
    }

    #[test]
    fn test_bonus_full_constellation() {
        let z = &ZODIAC[0];
        let ratio = 5.0 / 5.0;
        let hp = (z.hp_bonus as f64 * ratio) as i64;
        assert_eq!(hp, 200);
    }

    #[test]
    fn test_bonus_partial_constellation() {
        let z = &ZODIAC[0];
        let ratio = 3.0 / 5.0;
        let hp = (z.hp_bonus as f64 * ratio) as i64;
        assert_eq!(hp, 120);
    }

    #[test]
    fn test_zodiac_elements() {
        let fire_count = ZODIAC.iter().filter(|z| z.element == "火").count();
        let earth_count = ZODIAC.iter().filter(|z| z.element == "地").count();
        let water_count = ZODIAC.iter().filter(|z| z.element == "水").count();
        let wind_count = ZODIAC.iter().filter(|z| z.element == "风").count();
        assert_eq!(fire_count + earth_count + water_count + wind_count, 12);
        assert!(fire_count >= 2);
        assert!(earth_count >= 2);
        assert!(water_count >= 2);
        assert!(wind_count >= 2);
    }

    #[test]
    fn test_shop_items_count() {
        assert_eq!(SHOP_ITEMS.len(), 8);
        for item in &SHOP_ITEMS {
            assert!(item.cost > 0);
            assert!(!item.name.is_empty());
        }
    }

    #[test]
    fn test_shop_items_escalate() {
        for i in 1..SHOP_ITEMS.len() {
            assert!(SHOP_ITEMS[i].cost >= SHOP_ITEMS[i - 1].cost);
        }
    }

    #[test]
    fn test_fragment_sources() {
        assert_eq!(FRAGMENT_SOURCES.len(), 8);
        for source in &FRAGMENT_SOURCES {
            assert!(source.base_amount > 0);
            assert!(!source.name.is_empty());
        }
    }

    #[test]
    fn test_daily_cap() {
        assert_eq!(DAILY_FRAGMENT_CAP, 500);
    }

    #[test]
    fn test_zodiac_all_complete_bonus() {
        assert!(ZODIAC_ALL_COMPLETE_HP > 0);
        assert!(ZODIAC_ALL_COMPLETE_AD > 0);
        assert!(ZODIAC_ALL_COMPLETE_AP > 0);
        assert!(ZODIAC_ALL_COMPLETE_DEF > 0);
        assert!(ZODIAC_ALL_COMPLETE_MRES > 0);
    }

    #[test]
    fn test_zodiac_hp_bonuses_positive() {
        for z in &ZODIAC {
            assert!(z.hp_bonus > 0);
            assert!(z.ad_bonus > 0);
            assert!(z.ap_bonus > 0);
            assert!(z.def_bonus > 0);
            assert!(z.mres_bonus > 0);
        }
    }

    #[test]
    fn test_light_star_consumes_fragments() {
        let mut data = [0u8; ZODIAC_COUNT];
        let mut fragments = 100i64;
        let cost = STAR_FRAGMENT_COST[0]; // 10
        fragments -= cost;
        data[0] |= 1 << 0;
        assert_eq!(fragments, 90);
        assert_eq!(count_stars(data[0]), 1);
    }

    #[test]
    fn test_star_names() {
        assert_eq!(STAR_NAMES, ["α", "β", "γ", "δ", "ε"]);
    }

    #[test]
    fn test_zodiac_id_mapping() {
        // Ensure zodiac array index matches logical ordering
        assert_eq!(ZODIAC[0].name, "白羊座");
        assert_eq!(ZODIAC[11].name, "双鱼座");
    }

    #[test]
    fn test_section_name() {
        assert_eq!(SECTION, "star_map");
    }
}

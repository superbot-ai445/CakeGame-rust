/// CakeGame 灵脉系统
///
/// 散布在各地图的灵脉矿点，玩家可探索发现、击败守护兽后占领，
/// 持续产出资源（金币/经验/灵石/稀有材料）。其他玩家可挑战夺取。
///
/// 功能: 探索灵脉/灵脉详情/占领灵脉/放弃灵脉/灵脉收益/灵脉排行/灵脉帮助
///
/// 数据存储: Global 表 SECTION='spirit_vein' / 'spirit_vein_daily' / 'spirit_vein_stats'
use crate::core::*;
use crate::db::Database;

/// 灵脉品质等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VeinQuality {
    Common,    // 普通
    Uncommon,  // 优良
    Rare,      // 稀有
    Epic,      // 史诗
    Legendary, // 传说
}

impl VeinQuality {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Common => "普通",
            Self::Uncommon => "优良",
            Self::Rare => "稀有",
            Self::Epic => "史诗",
            Self::Legendary => "传说",
        }
    }
    fn emoji(&self) -> &str {
        match self {
            Self::Common => "⬜",
            Self::Uncommon => "🟢",
            Self::Rare => "🔵",
            Self::Epic => "🟣",
            Self::Legendary => "🟠",
        }
    }
    /// 每10分钟产出金币
    fn gold_per_tick(&self) -> i64 {
        match self {
            Self::Common => 50,
            Self::Uncommon => 150,
            Self::Rare => 400,
            Self::Epic => 1000,
            Self::Legendary => 3000,
        }
    }
    /// 每10分钟产出经验
    fn exp_per_tick(&self) -> i64 {
        match self {
            Self::Common => 30,
            Self::Uncommon => 100,
            Self::Rare => 250,
            Self::Epic => 600,
            Self::Legendary => 1500,
        }
    }
    /// 每10分钟产出灵石
    fn spirit_per_tick(&self) -> i64 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 3,
            Self::Rare => 8,
            Self::Epic => 20,
            Self::Legendary => 50,
        }
    }
    /// 守护兽战力
    fn guardian_power(&self) -> i64 {
        match self {
            Self::Common => 500,
            Self::Uncommon => 2000,
            Self::Rare => 8000,
            Self::Epic => 25000,
            Self::Legendary => 80000,
        }
    }
    /// 稀有材料掉率 (%)
    fn rare_material_rate(&self) -> i64 {
        match self {
            Self::Common => 0,
            Self::Uncommon => 5,
            Self::Rare => 15,
            Self::Epic => 30,
            Self::Legendary => 50,
        }
    }
    #[allow(dead_code)]
    fn index(&self) -> usize {
        match self {
            Self::Common => 0,
            Self::Uncommon => 1,
            Self::Rare => 2,
            Self::Epic => 3,
            Self::Legendary => 4,
        }
    }
}

#[allow(dead_code)]
const ALL_QUALITIES: [VeinQuality; 5] = [
    VeinQuality::Common,
    VeinQuality::Uncommon,
    VeinQuality::Rare,
    VeinQuality::Epic,
    VeinQuality::Legendary,
];

/// 灵脉定义
struct VeinTemplate {
    id: i64,
    name: &'static str,
    map_name: &'static str,
    quality: VeinQuality,
    guardian: &'static str,
    description: &'static str,
}

const VEIN_TEMPLATES: &[VeinTemplate] = &[
    VeinTemplate {
        id: 1,
        name: "青草灵脉",
        map_name: "新手村",
        quality: VeinQuality::Common,
        guardian: "灵脉石像",
        description: "新手村边陲的微弱灵脉",
    },
    VeinTemplate {
        id: 2,
        name: "晨露灵脉",
        map_name: "晨曦森林",
        quality: VeinQuality::Common,
        guardian: "树精守卫",
        description: "森林边缘的清新灵脉",
    },
    VeinTemplate {
        id: 3,
        name: "溪谷灵脉",
        map_name: "翡翠溪谷",
        quality: VeinQuality::Common,
        guardian: "水元素",
        description: "溪谷深处的水系灵脉",
    },
    VeinTemplate {
        id: 4,
        name: "风石灵脉",
        map_name: "风啸峡谷",
        quality: VeinQuality::Uncommon,
        guardian: "风暴鹰",
        description: "峡谷峭壁上的风系灵脉",
    },
    VeinTemplate {
        id: 5,
        name: "矿洞灵脉",
        map_name: "废弃矿洞",
        quality: VeinQuality::Uncommon,
        guardian: "岩石巨人",
        description: "矿洞深处的土系灵脉",
    },
    VeinTemplate {
        id: 6,
        name: "月华灵脉",
        map_name: "月光湖畔",
        quality: VeinQuality::Uncommon,
        guardian: "月影狐",
        description: "月光照耀下的湖泊灵脉",
    },
    VeinTemplate {
        id: 7,
        name: "熔岩灵脉",
        map_name: "火焰山",
        quality: VeinQuality::Rare,
        guardian: "熔岩领主",
        description: "火焰山口的火系灵脉",
    },
    VeinTemplate {
        id: 8,
        name: "冰晶灵脉",
        map_name: "寒冰宫殿",
        quality: VeinQuality::Rare,
        guardian: "冰霜龙",
        description: "永冻宫殿内的冰系灵脉",
    },
    VeinTemplate {
        id: 9,
        name: "暗影灵脉",
        map_name: "暗影沼泽",
        quality: VeinQuality::Rare,
        guardian: "暗影魔",
        description: "沼泽深处的暗系灵脉",
    },
    VeinTemplate {
        id: 10,
        name: "雷霆灵脉",
        map_name: "雷鸣山巅",
        quality: VeinQuality::Rare,
        guardian: "雷兽",
        description: "山巅雷云中的雷系灵脉",
    },
    VeinTemplate {
        id: 11,
        name: "古树灵脉",
        map_name: "精灵森林",
        quality: VeinQuality::Epic,
        guardian: "远古树人",
        description: "精灵圣地的生命灵脉",
    },
    VeinTemplate {
        id: 12,
        name: "龙骨灵脉",
        map_name: "龙骨荒原",
        quality: VeinQuality::Epic,
        guardian: "骨龙",
        description: "远古巨龙遗骸中的灵脉",
    },
    VeinTemplate {
        id: 13,
        name: "星陨灵脉",
        map_name: "星陨盆地",
        quality: VeinQuality::Epic,
        guardian: "星辰守卫",
        description: "天外陨石落地形成的灵脉",
    },
    VeinTemplate {
        id: 14,
        name: "圣光灵脉",
        map_name: "圣光神殿",
        quality: VeinQuality::Epic,
        guardian: "圣光天使",
        description: "神殿祭坛上的神圣灵脉",
    },
    VeinTemplate {
        id: 15,
        name: "混沌灵脉",
        map_name: "混沌深渊",
        quality: VeinQuality::Legendary,
        guardian: "混沌巨兽",
        description: "深渊裂缝中溢出的混沌灵脉",
    },
    VeinTemplate {
        id: 16,
        name: "时空灵脉",
        map_name: "时空裂隙",
        quality: VeinQuality::Legendary,
        guardian: "时空守护者",
        description: "时间夹缝中的永恒灵脉",
    },
    VeinTemplate {
        id: 17,
        name: "创世灵脉",
        map_name: "世界之巅",
        quality: VeinQuality::Legendary,
        guardian: "创世巨神",
        description: "世界尽头的终极灵脉",
    },
];

const MAX_EXPLORES_PER_DAY: i64 = 10;
const MAX_CLAIMS_PER_DAY: i64 = 3;
const MAX_COLLECTS_PER_DAY: i64 = 20;
const MAX_OWNED_VEINS: usize = 3;
const COLLECT_INTERVAL_SECS: i64 = 600; // 10 minutes

// ── Data Access Helpers ──────────────────────────────────────────

fn now_ts() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn today_key() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("day_{}", secs / 86400)
}

/// Load vein state: "owner_id|owner_name|occupied_at|accum_gold|accum_exp|accum_spirit|total_yield"
fn load_vein(db: &Database, vein_id: i64) -> (i64, String, i64, i64, i64, i64, i64) {
    let raw = db.global_get("spirit_vein", &format!("vein_{}", vein_id));
    if raw.is_empty() {
        return (0, String::new(), 0, 0, 0, 0, 0);
    }
    let p: Vec<&str> = raw.split('|').collect();
    (
        p.first().and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(1).unwrap_or(&"").to_string(),
        p.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
    )
}

#[allow(clippy::too_many_arguments)]
fn save_vein(
    db: &Database,
    vein_id: i64,
    owner_id: i64,
    owner_name: &str,
    occupied_at: i64,
    ag: i64,
    ae: i64,
    as_: i64,
    ty: i64,
) {
    let val = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        owner_id, owner_name, occupied_at, ag, ae, as_, ty
    );
    db.global_set("spirit_vein", &format!("vein_{}", vein_id), &val);
}

fn load_daily(db: &Database, user_id: &str) -> (i64, i64, i64, i64) {
    let key = format!("{}_{}", user_id, today_key());
    let raw = db.global_get("spirit_vein_daily", &key);
    if raw.is_empty() {
        return (0, 0, 0, 0);
    }
    let p: Vec<&str> = raw.split('|').collect();
    (
        p.first().and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        p.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
    )
}

fn save_daily(db: &Database, user_id: &str, explores: i64, claims: i64, defenses: i64, collects: i64) {
    let key = format!("{}_{}", user_id, today_key());
    db.global_set(
        "spirit_vein_daily",
        &key,
        &format!("{}|{}|{}|{}", explores, claims, defenses, collects),
    );
}

fn get_spirit_stones(db: &Database, user_id: &str) -> i64 {
    db.read_user_data(user_id, "spirit_stones").parse().unwrap_or(0)
}

fn set_spirit_stones(db: &Database, user_id: &str, val: i64) {
    db.write_user_data(user_id, "spirit_stones", &val.to_string());
}

/// Get nickname from basic data
fn get_nickname(db: &Database, user_id: &str) -> String {
    db.read_basic(user_id, ITEM_NAME)
}

/// Calculate combat power
fn get_combat_power(db: &Database, user_id: &str) -> i64 {
    let info = crate::user::calc_total_attrs(db, user_id);
    let ad = info.ad as i64;
    let ap = info.ap as i64;
    let hp = info.hp_max as i64;
    let defense = info.defense as i64;
    let mr = info.magic_res as i64;
    ad + ap + hp / 10 + defense + mr
}

fn count_owned_veins(db: &Database, user_id: &str) -> usize {
    let uid: i64 = user_id.parse().unwrap_or(0);
    VEIN_TEMPLATES
        .iter()
        .filter(|v| {
            let (oid, ..) = load_vein(db, v.id);
            oid == uid
        })
        .count()
}

fn calc_pending(tpl: &VeinTemplate, occupied_at: i64, ag: i64, ae: i64, as_: i64) -> (i64, i64, i64) {
    if occupied_at <= 0 {
        return (0, 0, 0);
    }
    let elapsed = now_ts() - occupied_at;
    if elapsed <= 0 {
        return (0, 0, 0);
    }
    let ticks = elapsed / COLLECT_INTERVAL_SECS;
    if ticks <= 0 {
        return (0, 0, 0);
    }
    let g = tpl.quality.gold_per_tick() * ticks;
    let e = tpl.quality.exp_per_tick() * ticks;
    let s = tpl.quality.spirit_per_tick() * ticks;
    ((g - ag).max(0), (e - ae).max(0), (s - as_).max(0))
}

#[allow(dead_code)]
fn progress_bar(current: i64, max: i64) -> String {
    let pct = if max > 0 { (current * 100 / max).min(100) } else { 0 };
    let filled = (pct / 10) as usize;
    let empty = 10 - filled;
    format!("[{}{}{}%]", "█".repeat(filled), "░".repeat(empty), pct)
}

// ── Command Handlers ─────────────────────────────────────────────

/// 探索灵脉
pub fn cmd_explore_vein(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let (mut explores, claims, defenses, collects) = load_daily(db, user_id);
    if explores >= MAX_EXPLORES_PER_DAY {
        return format!("❌ 今日探索次数已用完 ({}/{})", explores, MAX_EXPLORES_PER_DAY);
    }

    let map_name = args.trim();
    let veins: Vec<&VeinTemplate> = VEIN_TEMPLATES
        .iter()
        .filter(|v| map_name == "all" || v.map_name.contains(map_name) || map_name.contains(v.map_name))
        .collect();

    if veins.is_empty() {
        let suggestions: Vec<&str> = VEIN_TEMPLATES.iter().take(5).map(|v| v.map_name).collect();
        return format!(
            "❌ 在「{}」未发现灵脉\n💡 已知灵脉所在: {}\n💡 输入「探索灵脉 all」查看全部",
            map_name,
            suggestions.join("、")
        );
    }

    explores += 1;
    save_daily(db, user_id, explores, claims, defenses, collects);

    let mut result = format!("🔍 探索「{}」灵脉结果:\n\n", map_name);
    for vein in &veins {
        let (oid, ref oname, _oa, ag, ae, as_, _ty) = load_vein(db, vein.id);
        let (pg, pe, ps) = calc_pending(vein, _oa, ag, ae, as_);
        let status = if oid == 0 {
            "🟢 空闲可占领".to_string()
        } else if user_id.parse::<i64>().unwrap_or(0) == oid {
            "⭐ 已被你占领".to_string()
        } else {
            format!("🔴 被{}占领", oname)
        };
        let pending = if pg + pe + ps > 0 {
            format!(" 📦累积:{}金/{}经验/{}灵石", pg, pe, ps)
        } else {
            String::new()
        };
        result += &format!(
            "{} {} [{}] — {}{}\n  守护兽: {} (战力{}) | 产出: {}金/{}经验/{}灵石/10分钟\n",
            vein.quality.emoji(),
            vein.name,
            vein.quality.as_str(),
            status,
            pending,
            vein.guardian,
            vein.quality.guardian_power(),
            vein.quality.gold_per_tick(),
            vein.quality.exp_per_tick(),
            vein.quality.spirit_per_tick(),
        );
    }
    result += &format!("\n📊 今日探索: {}/{}", explores, MAX_EXPLORES_PER_DAY);
    result += "\n💡 使用「占领灵脉 [名称]」占领空闲灵脉";
    result
}

/// 灵脉详情
pub fn cmd_vein_detail(db: &Database, _user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let vein_name = args.trim();
    let tpl = VEIN_TEMPLATES.iter().find(|v| v.name.contains(vein_name));
    let tpl = match tpl {
        Some(t) => t,
        None => return format!("❌ 未找到灵脉「{}」", vein_name),
    };
    let (oid, oname, oa, ag, ae, as_, ty) = load_vein(db, tpl.id);
    let (pg, pe, ps) = calc_pending(tpl, oa, ag, ae, as_);
    let owner = if oid == 0 {
        "无".to_string()
    } else {
        format!("{}(ID:{})", oname, oid)
    };
    let occupied_time = if oa > 0 {
        let mins = (now_ts() - oa) / 60;
        format!("{}小时{}分钟前", mins / 60, mins % 60)
    } else {
        "未占领".to_string()
    };
    let rare = if tpl.quality.rare_material_rate() > 0 {
        "🍀"
    } else {
        "⬜"
    };
    format!(
        "🏔️ 灵脉详情: {}{}\n\n\
         品质: {} {}\n\
         地点: {}\n\
         描述: {}\n\
         守护兽: {} (战力{})\n\
         占领者: {}\n\
         占领时间: {}\n\n\
         📈 每10分钟产出:\n\
         ⚔️ 金币: +{}\n\
         ⭐ 经验: +{}\n\
         💎 灵石: +{}\n\
         {} 稀有材料掉率: {}%\n\n\
         📦 当前累积: {}金 / {}经验 / {}灵石\n\
         📊 历史总产出: {}\n\n\
         💡 挑战守护兽战力要求: {}",
        tpl.quality.emoji(),
        tpl.name,
        tpl.quality.emoji(),
        tpl.quality.as_str(),
        tpl.map_name,
        tpl.description,
        tpl.guardian,
        tpl.quality.guardian_power(),
        owner,
        occupied_time,
        tpl.quality.gold_per_tick(),
        tpl.quality.exp_per_tick(),
        tpl.quality.spirit_per_tick(),
        rare,
        tpl.quality.rare_material_rate(),
        pg,
        pe,
        ps,
        ty,
        tpl.quality.guardian_power(),
    )
}

/// 占领灵脉
pub fn cmd_occupy_vein(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let vein_name = args.trim();
    let tpl = VEIN_TEMPLATES.iter().find(|v| v.name.contains(vein_name));
    let tpl = match tpl {
        Some(t) => t,
        None => return format!("❌ 未找到灵脉「{}」", vein_name),
    };

    let (explores, mut claims, defenses, collects) = load_daily(db, user_id);
    if claims >= MAX_CLAIMS_PER_DAY {
        return format!("❌ 今日占领次数已用完 ({}/{})", claims, MAX_CLAIMS_PER_DAY);
    }

    let owned = count_owned_veins(db, user_id);
    if owned >= MAX_OWNED_VEINS {
        return format!("❌ 你最多同时占领{}个灵脉，当前已占领{}个", MAX_OWNED_VEINS, owned);
    }

    let uid: i64 = user_id.parse().unwrap_or(0);
    let (oid, _oname, oa, ag, ae, as_, ty) = load_vein(db, tpl.id);

    if oid == uid {
        return format!("❌ 你已经占领了「{}」", tpl.name);
    }

    // Power check
    let user_power = get_combat_power(db, user_id);
    if user_power < tpl.quality.guardian_power() {
        return format!(
            "❌ 战力不足！挑战「{}」守护兽需要{}战力，你当前{}战力\n💡 差距: {}",
            tpl.name,
            tpl.quality.guardian_power(),
            user_power,
            tpl.quality.guardian_power() - user_power
        );
    }

    // Pay out pending to old owner
    if oid > 0 {
        let (pg, pe, _ps) = calc_pending(tpl, oa, ag, ae, as_);
        if pg > 0 || pe > 0 {
            let old_uid = &oid.to_string();
            db.modify_currency(old_uid, CURRENCY_GOLD, OP_ADD, pg);
            db.modify_currency(old_uid, CURRENCY_DIAMOND, OP_ADD, pe / 100); // some exp as diamond
        }
    }

    // Occupy
    let nickname = get_nickname(db, user_id);
    save_vein(db, tpl.id, uid, &nickname, now_ts(), 0, 0, 0, ty);

    claims += 1;
    save_daily(db, user_id, explores, claims, defenses, collects);

    // Stats
    let stat_key = format!("stats_{}", user_id);
    let cur: i64 = db.global_get("spirit_vein_stats", &stat_key).parse().unwrap_or(0);
    db.global_set("spirit_vein_stats", &stat_key, &(cur + 1).to_string());

    format!(
        "🎉 成功击败「{}」守护兽，占领了{}{}！\n\n\
         📍 地点: {}\n\
         📈 每10分钟产出: {}金 / {}经验 / {}灵石\n\
         📦 使用「收取灵石」领取累积收益\n\
         ⚔️ 已占领: {}/{}个灵脉\n\n\
         ⚠️ 注意: 其他更高战力的玩家可以挑战夺取你的灵脉",
        tpl.guardian,
        tpl.quality.emoji(),
        tpl.name,
        tpl.map_name,
        tpl.quality.gold_per_tick(),
        tpl.quality.exp_per_tick(),
        tpl.quality.spirit_per_tick(),
        owned + 1,
        MAX_OWNED_VEINS,
    )
}

/// 收取灵脉收益
pub fn cmd_collect_vein(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let (explores, claims, defenses, mut collects) = load_daily(db, user_id);
    if collects >= MAX_COLLECTS_PER_DAY {
        return format!("❌ 今日收取次数已用完 ({}/{})", collects, MAX_COLLECTS_PER_DAY);
    }

    let uid: i64 = user_id.parse().unwrap_or(0);
    let mut total_gold: i64 = 0;
    let mut total_exp: i64 = 0;
    let mut total_spirit: i64 = 0;
    let mut collected = Vec::new();

    for tpl in VEIN_TEMPLATES {
        let (oid, _oname, oa, ag, ae, as_, ty) = load_vein(db, tpl.id);
        if oid != uid {
            continue;
        }
        let (pg, pe, ps) = calc_pending(tpl, oa, ag, ae, as_);
        if pg <= 0 && pe <= 0 && ps <= 0 {
            continue;
        }
        total_gold += pg;
        total_exp += pe;
        total_spirit += ps;
        save_vein(
            db,
            tpl.id,
            uid,
            &_oname,
            now_ts(),
            ag + pg,
            ae + pe,
            as_ + ps,
            ty + pg + pe + ps,
        );
        collected.push(format!(
            "  {}{}: +{}金 +{}经验 +{}灵石",
            tpl.quality.emoji(),
            tpl.name,
            pg,
            pe,
            ps
        ));
    }

    if collected.is_empty() {
        return "❌ 你当前没有灵脉累积收益\n💡 使用「探索灵脉 [地图名]」发现并占领灵脉".to_string();
    }

    // Grant rewards
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold);
    let old_stones = get_spirit_stones(db, user_id);
    set_spirit_stones(db, user_id, old_stones + total_spirit);

    // Rare material drops
    let materials = ["灵脉结晶", "元素精华", "矿脉碎片", "远古矿石", "混沌矿尘"];
    let mut rare_drops = Vec::new();
    for tpl in VEIN_TEMPLATES.iter().filter(|v| {
        let (oid, ..) = load_vein(db, v.id);
        oid == uid
    }) {
        if tpl.quality.rare_material_rate() > 0 {
            let roll = (now_ts() + tpl.id) % 100;
            if roll < tpl.quality.rare_material_rate() {
                let mat_idx = (tpl.id as usize) % materials.len();
                rare_drops.push(materials[mat_idx]);
            }
        }
    }

    collects += 1;
    save_daily(db, user_id, explores, claims, defenses, collects);

    let mut result = format!(
        "💰 灵脉收益收取成功！\n\n📦 收取明细:\n{}\n\n📊 合计: +{}金 / +{}经验 / +{}灵石",
        collected.join("\n"),
        total_gold,
        total_exp,
        total_spirit
    );

    if !rare_drops.is_empty() {
        result += &format!("\n\n🍀 稀有材料掉落: {}", rare_drops.join("、"));
    }

    result += &format!(
        "\n\n📊 今日收取: {}/{}\n💎 当前灵石: {}",
        collects,
        MAX_COLLECTS_PER_DAY,
        get_spirit_stones(db, user_id)
    );
    result
}

/// 放弃灵脉
pub fn cmd_abandon_vein(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let vein_name = args.trim();
    let tpl = VEIN_TEMPLATES.iter().find(|v| v.name.contains(vein_name));
    let tpl = match tpl {
        Some(t) => t,
        None => return format!("❌ 未找到灵脉「{}」", vein_name),
    };
    let uid: i64 = user_id.parse().unwrap_or(0);
    let (oid, _oname, oa, ag, ae, as_, _ty) = load_vein(db, tpl.id);
    if oid != uid {
        return format!("❌ 你没有占领「{}」", tpl.name);
    }
    // Pay out pending
    let (pg, pe, ps) = calc_pending(tpl, oa, ag, ae, as_);
    if pg > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, pg);
    }
    if ps > 0 {
        let old = get_spirit_stones(db, user_id);
        set_spirit_stones(db, user_id, old + ps);
    }
    save_vein(db, tpl.id, 0, "", 0, 0, 0, 0, 0);
    format!(
        "✅ 已放弃「{}{}」灵脉\n📦 最后收益: +{}金 / +{}经验 / +{}灵石",
        tpl.quality.emoji(),
        tpl.name,
        pg,
        pe,
        ps
    )
}

/// 灵脉收益概览
pub fn cmd_vein_income(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let uid: i64 = user_id.parse().unwrap_or(0);
    let mut result = "💰 灵脉收益概览:\n\n".to_string();
    let mut total_hg: i64 = 0;
    let mut total_he: i64 = 0;
    let mut total_hs: i64 = 0;
    let mut has_any = false;

    for tpl in VEIN_TEMPLATES {
        let (oid, _oname, oa, ag, ae, as_, _ty) = load_vein(db, tpl.id);
        if oid != uid {
            continue;
        }
        has_any = true;
        let (pg, pe, ps) = calc_pending(tpl, oa, ag, ae, as_);
        let hg = tpl.quality.gold_per_tick() * 6;
        let he = tpl.quality.exp_per_tick() * 6;
        let hs = tpl.quality.spirit_per_tick() * 6;
        total_hg += hg;
        total_he += he;
        total_hs += hs;
        result += &format!(
            "{}{}: 累积📦{}金/{}经验/{}灵石 | 每小时⏰{}金/{}经验/{}灵石\n",
            tpl.quality.emoji(),
            tpl.name,
            pg,
            pe,
            ps,
            hg,
            he,
            hs
        );
    }

    if !has_any {
        return "❌ 你当前没有占领任何灵脉\n💡 使用「探索灵脉 [地图名]」发现并占领灵脉".to_string();
    }

    let (_, _, _, collects) = load_daily(db, user_id);
    result += &format!(
        "\n📊 每小时总产出: {}金 / {}经验 / {}灵石\n\
         💎 当前灵石: {}\n\
         📈 今日收取: {}/{}\n\
         ⚔️ 已占领: {}/{}",
        total_hg,
        total_he,
        total_hs,
        get_spirit_stones(db, user_id),
        collects,
        MAX_COLLECTS_PER_DAY,
        count_owned_veins(db, user_id),
        MAX_OWNED_VEINS,
    );
    result
}

/// 灵脉排行
pub fn cmd_vein_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut owner_map: std::collections::HashMap<i64, (String, i64, i64)> = std::collections::HashMap::new();
    for tpl in VEIN_TEMPLATES {
        let (oid, oname, _oa, _ag, _ae, _as_, ty) = load_vein(db, tpl.id);
        if oid > 0 {
            let entry = owner_map.entry(oid).or_insert((oname, 0, 0));
            entry.1 += 1;
            entry.2 += ty;
        }
    }
    let mut rankings: Vec<(i64, String, i64, i64)> = owner_map
        .into_iter()
        .map(|(id, (name, count, y))| (id, name, count, y))
        .collect();
    rankings.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));

    let medals = ["🥇", "🥈", "🥉"];
    let mut result = "🏔️ 灵脉占领排行:\n\n".to_string();
    for (i, (_id, name, count, total_yield)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { &format!("{:>2}.", i + 1) };
        result += &format!("{} {} — {}个灵脉 | 总产出:{}\n", medal, name, count, total_yield);
    }

    let uid: i64 = user_id.parse().unwrap_or(0);
    let user_rank = rankings.iter().position(|(id, _, _, _)| *id == uid);
    match user_rank {
        Some(pos) => {
            result += &format!(
                "\n📍 你的排名: 第{}名 ({}个灵脉, 总产出{})",
                pos + 1,
                rankings[pos].2,
                rankings[pos].3
            );
        }
        None => {
            result += "\n📍 你尚未占领任何灵脉";
        }
    }
    result
}

/// 灵脉帮助
pub fn cmd_vein_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    "\
🏔️ 【灵脉系统】帮助

散布在各地图的灵脉矿点，蕴含丰富的灵力资源。
击败守护兽后占领灵脉，持续产出金币、经验和灵石。

📋 指令列表:
  探索灵脉 [地图名] — 探索指定地图的灵脉
  灵脉详情 [名称] — 查看灵脉详细信息
  占领灵脉 [名称] — 击败守护兽占领灵脉
  收取灵石 — 领取所有灵脉的累积收益
  放弃灵脉 [名称] — 放弃占领的灵脉
  灵脉收益 — 查看收益概览
  灵脉排行 — 查看全服灵脉排行
  灵石商店 — 灵石兑换商店
  灵石兑换 [序号] — 兑换商店商品
  灵脉帮助 — 显示本帮助

📊 灵脉品质:
  ⬜ 普通 — 50金/30经验/1灵石 (10分钟)
  🟢 优良 — 150金/100经验/3灵石 (10分钟)
  🔵 稀有 — 400金/250经验/8灵石 (10分钟)
  🟣 史诗 — 1000金/600经验/20灵石 (10分钟)
  🟠 传说 — 3000金/1500经验/50灵石 (10分钟)

⚙️ 规则说明:
  • 每日探索10次，占领3次，收取20次
  • 最多同时占领3个灵脉
  • 战力需超过守护兽才能占领
  • 灵石可用于灵脉商店兑换稀有道具
  • 更高战力的玩家可以挑战夺取你的灵脉
  • 稀有品质以上灵脉有概率产出稀有材料
"
    .to_string()
}

/// 灵石商店
pub fn cmd_spirit_shop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let stones = get_spirit_stones(db, user_id);
    let items = [
        (50_i64, "灵脉药水", "❤️", "恢复100% HP"),
        (100, "灵脉精华", "💎", "强化成功率+10%"),
        (200, "灵脉护符", "🛡️", "防御+5%持续1小时"),
        (500, "灵脉之翼", "🪽", "经验获取+20%持续2小时"),
        (1000, "灵脉结晶", "🔮", "全属性+3%永久"),
        (2000, "灵脉圣石", "✨", "暴击率+5%永久"),
        (5000, "灵脉至尊宝箱", "🎁", "随机传说装备碎片"),
    ];
    let mut result = format!("🏪 灵石商店 (当前灵石: {})\n\n", stones);
    for (i, (cost, name, emoji, desc)) in items.iter().enumerate() {
        let affordable = if stones >= *cost { "✅" } else { "❌" };
        result += &format!(
            "{}. {} {} — {}灵石 {} ({})\n",
            i + 1,
            emoji,
            name,
            cost,
            affordable,
            desc
        );
    }
    result += "\n💡 使用「灵石兑换 [序号]」购买";
    result
}

/// 灵石兑换
pub fn cmd_spirit_exchange(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "❌ 请先注册账号".to_string();
    }
    let slot: usize = args.trim().parse().unwrap_or(0);
    if slot == 0 || slot > 7 {
        return "❌ 无效的商品序号 (1-7)".to_string();
    }
    let items = [
        (50_i64, "灵脉药水"),
        (100, "灵脉精华"),
        (200, "灵脉护符"),
        (500, "灵脉之翼"),
        (1000, "灵脉结晶"),
        (2000, "灵脉圣石"),
        (5000, "灵脉至尊宝箱"),
    ];
    let (cost, name) = items[slot - 1];
    let stones = get_spirit_stones(db, user_id);
    if stones < cost {
        return format!("❌ 灵石不足！需要{}灵石，当前{}", cost, stones);
    }
    set_spirit_stones(db, user_id, stones - cost);
    // Grant gold equivalent
    let gold_bonus = cost * 10;
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_bonus);
    format!(
        "✅ 兑换成功！花费{}灵石购买了「{}」\n💰 获得等价{}金币\n💎 剩余灵石: {}",
        cost,
        name,
        gold_bonus,
        stones - cost
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vein_quality_ordering() {
        assert!(VeinQuality::Common < VeinQuality::Legendary);
        assert!(VeinQuality::Rare < VeinQuality::Epic);
    }

    #[test]
    fn test_vein_quality_gold_escalation() {
        let vals: Vec<i64> = ALL_QUALITIES.iter().map(|q| q.gold_per_tick()).collect();
        for w in vals.windows(2) {
            assert!(w[1] > w[0], "Gold should escalate: {} > {}", w[1], w[0]);
        }
    }

    #[test]
    fn test_vein_quality_exp_escalation() {
        let vals: Vec<i64> = ALL_QUALITIES.iter().map(|q| q.exp_per_tick()).collect();
        for w in vals.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn test_vein_quality_spirit_escalation() {
        let vals: Vec<i64> = ALL_QUALITIES.iter().map(|q| q.spirit_per_tick()).collect();
        for w in vals.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn test_vein_quality_guardian_power_escalation() {
        let vals: Vec<i64> = ALL_QUALITIES.iter().map(|q| q.guardian_power()).collect();
        for w in vals.windows(2) {
            assert!(w[1] > w[0]);
        }
    }

    #[test]
    fn test_vein_templates_count() {
        assert_eq!(VEIN_TEMPLATES.len(), 17);
    }

    #[test]
    fn test_vein_template_ids_unique() {
        let mut ids: Vec<i64> = VEIN_TEMPLATES.iter().map(|v| v.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), VEIN_TEMPLATES.len());
    }

    #[test]
    fn test_vein_template_names_unique() {
        let mut names: Vec<&str> = VEIN_TEMPLATES.iter().map(|v| v.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), VEIN_TEMPLATES.len());
    }

    #[test]
    fn test_quality_display() {
        assert_eq!(VeinQuality::Common.as_str(), "普通");
        assert_eq!(VeinQuality::Legendary.as_str(), "传说");
        assert_eq!(VeinQuality::Common.emoji(), "⬜");
        assert_eq!(VeinQuality::Legendary.emoji(), "🟠");
    }

    #[test]
    fn test_quality_index() {
        for (i, q) in ALL_QUALITIES.iter().enumerate() {
            assert_eq!(q.index(), i);
        }
    }

    #[test]
    fn test_rare_material_rate_only_higher() {
        assert_eq!(VeinQuality::Common.rare_material_rate(), 0);
        assert!(VeinQuality::Legendary.rare_material_rate() > VeinQuality::Rare.rare_material_rate());
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100);
        assert!(bar.contains("0%"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100);
        assert!(bar.contains("100%"));
        assert!(bar.contains("█"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 100);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_all_veins_have_maps() {
        for tpl in VEIN_TEMPLATES {
            assert!(!tpl.map_name.is_empty());
            assert!(!tpl.name.is_empty());
            assert!(!tpl.guardian.is_empty());
            assert!(!tpl.description.is_empty());
        }
    }

    #[test]
    fn test_quality_all_have_unique_display() {
        let mut displays: Vec<&str> = ALL_QUALITIES.iter().map(|q| q.as_str()).collect();
        displays.sort();
        displays.dedup();
        assert_eq!(displays.len(), ALL_QUALITIES.len());
    }

    #[test]
    fn test_constants_sanity() {
        assert!(MAX_EXPLORES_PER_DAY > 0);
        assert!(MAX_CLAIMS_PER_DAY > 0);
        assert!(MAX_OWNED_VEINS > 0);
        assert!(COLLECT_INTERVAL_SECS > 0);
    }

    #[test]
    fn test_vein_quality_rare_material_escalation() {
        let rates: Vec<i64> = ALL_QUALITIES.iter().map(|q| q.rare_material_rate()).collect();
        assert_eq!(rates[0], 0);
        for w in rates[1..].windows(2) {
            assert!(w[1] >= w[0]);
        }
    }

    #[test]
    fn test_calc_pending_zero_time() {
        let tpl = &VEIN_TEMPLATES[0];
        let (g, e, s) = calc_pending(tpl, 0, 0, 0, 0);
        assert_eq!(g, 0);
        assert_eq!(e, 0);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_vein_template_coverage() {
        // All 5 qualities should be represented
        for q in &ALL_QUALITIES {
            assert!(
                VEIN_TEMPLATES.iter().any(|v| v.quality == *q),
                "Missing quality: {:?}",
                q
            );
        }
    }

    #[test]
    fn test_vein_ids_sequential() {
        for (i, tpl) in VEIN_TEMPLATES.iter().enumerate() {
            assert_eq!(tpl.id, (i + 1) as i64);
        }
    }
}

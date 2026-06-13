/// CakeGame 时装收集系统
///
/// 玩家可以收集时装，穿戴后获得称号和属性加成。
/// 时装分为5个品质(普通/稀有/史诗/传说/神话)，集齐同品质有额外套装加成。
/// 时装不覆盖装备属性，是独立的收集系统。
///
/// 指令: 查看时装/购买时装/穿戴时装/卸下时装/时装图鉴/时装排行
/// 数据存储: read_user_data/write_user_data
use crate::core::{CURRENCY_DIAMOND, CURRENCY_GOLD, ITEM_NAME};
use crate::db::Database;
use crate::user;

#[allow(dead_code)]
const SECTION: &str = "costume";
const OWNED_KEY: &str = "costume_owned"; // JSON list of owned costume IDs
const EQUIPPED_KEY: &str = "costume_equipped"; // currently equipped costume ID (or empty)
#[allow(dead_code)]
const DATA_KEY: &str = "costume_data"; // JSON {total_spent, collection_count}

/// 时装品质
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quality {
    Common,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl Quality {
    fn name(&self) -> &'static str {
        match self {
            Quality::Common => "普通",
            Quality::Rare => "稀有",
            Quality::Epic => "史诗",
            Quality::Legendary => "传说",
            Quality::Mythic => "神话",
        }
    }
    fn emoji(&self) -> &'static str {
        match self {
            Quality::Common => "⚪",
            Quality::Rare => "🟢",
            Quality::Epic => "🟣",
            Quality::Legendary => "🟡",
            Quality::Mythic => "🔴",
        }
    }
    fn set_bonus_pct(&self) -> i32 {
        match self {
            Quality::Common => 2,
            Quality::Rare => 5,
            Quality::Epic => 8,
            Quality::Legendary => 12,
            Quality::Mythic => 20,
        }
    }
}

/// 时装定义
struct CostumeDef {
    id: i32,
    name: &'static str,
    title: &'static str,
    quality: Quality,
    cost_gold: i64,
    cost_diamond: i32,
    hp_bonus: i32,
    ad_bonus: i32,
    ap_bonus: i32,
    def_bonus: i32,
    #[allow(dead_code)]
    desc: &'static str,
}

const COSTUMES: &[CostumeDef] = &[
    // === 普通品质 ===
    CostumeDef {
        id: 1,
        name: "布衣素衫",
        title: "布衣侠客",
        quality: Quality::Common,
        cost_gold: 1000,
        cost_diamond: 0,
        hp_bonus: 20,
        ad_bonus: 5,
        ap_bonus: 0,
        def_bonus: 3,
        desc: "朴素的布衣，初入江湖的标准装束",
    },
    CostumeDef {
        id: 2,
        name: "学徒法袍",
        title: "魔法学徒",
        quality: Quality::Common,
        cost_gold: 1000,
        cost_diamond: 0,
        hp_bonus: 15,
        ad_bonus: 0,
        ap_bonus: 8,
        def_bonus: 2,
        desc: "魔法师协会发给新人的法袍",
    },
    CostumeDef {
        id: 3,
        name: "猎人皮甲",
        title: "丛林猎手",
        quality: Quality::Common,
        cost_gold: 1200,
        cost_diamond: 0,
        hp_bonus: 25,
        ad_bonus: 6,
        ap_bonus: 0,
        def_bonus: 4,
        desc: "猎人公会制式装备，轻便耐用",
    },
    CostumeDef {
        id: 4,
        name: "铁匠围裙",
        title: "锻造达人",
        quality: Quality::Common,
        cost_gold: 800,
        cost_diamond: 0,
        hp_bonus: 30,
        ad_bonus: 3,
        ap_bonus: 0,
        def_bonus: 6,
        desc: "铁匠铺的围裙，沾满了铁锈和汗水",
    },
    // === 稀有品质 ===
    CostumeDef {
        id: 5,
        name: "翡翠战甲",
        title: "翡翠骑士",
        quality: Quality::Rare,
        cost_gold: 5000,
        cost_diamond: 5,
        hp_bonus: 60,
        ad_bonus: 15,
        ap_bonus: 0,
        def_bonus: 12,
        desc: "镶嵌翡翠的精良战甲，闪闪发光",
    },
    CostumeDef {
        id: 6,
        name: "星辰法衣",
        title: "星辰法师",
        quality: Quality::Rare,
        cost_gold: 5000,
        cost_diamond: 5,
        hp_bonus: 45,
        ad_bonus: 0,
        ap_bonus: 20,
        def_bonus: 8,
        desc: "绣满星图的法衣，夜晚会发出微光",
    },
    CostumeDef {
        id: 7,
        name: "暗影披风",
        title: "暗影行者",
        quality: Quality::Rare,
        cost_gold: 6000,
        cost_diamond: 8,
        hp_bonus: 50,
        ad_bonus: 18,
        ap_bonus: 5,
        def_bonus: 6,
        desc: "暗影组织的标志性披风，令人畏惧",
    },
    CostumeDef {
        id: 8,
        name: "烈焰战袍",
        title: "烈焰战士",
        quality: Quality::Rare,
        cost_gold: 5500,
        cost_diamond: 6,
        hp_bonus: 55,
        ad_bonus: 12,
        ap_bonus: 12,
        def_bonus: 10,
        desc: "用火焰蜥蜴皮制成的战袍，自带温度",
    },
    // === 史诗品质 ===
    CostumeDef {
        id: 9,
        name: "龙鳞圣甲",
        title: "屠龙勇士",
        quality: Quality::Epic,
        cost_gold: 20000,
        cost_diamond: 30,
        hp_bonus: 120,
        ad_bonus: 30,
        ap_bonus: 0,
        def_bonus: 25,
        desc: "用远古龙鳞打造的圣甲，刀枪不入",
    },
    CostumeDef {
        id: 10,
        name: "天界圣衣",
        title: "天界使者",
        quality: Quality::Epic,
        cost_gold: 22000,
        cost_diamond: 35,
        hp_bonus: 100,
        ad_bonus: 10,
        ap_bonus: 35,
        def_bonus: 20,
        desc: "据说是天界使者遗留在人间的圣衣",
    },
    CostumeDef {
        id: 11,
        name: "冥王黑铠",
        title: "冥界之主",
        quality: Quality::Epic,
        cost_gold: 25000,
        cost_diamond: 40,
        hp_bonus: 150,
        ad_bonus: 25,
        ap_bonus: 25,
        def_bonus: 30,
        desc: "来自冥界的黑色铠甲，散发死亡气息",
    },
    // === 传说品质 ===
    CostumeDef {
        id: 12,
        name: "凤凰涅槃衣",
        title: "涅槃重生者",
        quality: Quality::Legendary,
        cost_gold: 80000,
        cost_diamond: 100,
        hp_bonus: 250,
        ad_bonus: 40,
        ap_bonus: 40,
        def_bonus: 40,
        desc: "凤凰涅槃后留下的羽毛编织而成，拥有重生之力",
    },
    CostumeDef {
        id: 13,
        name: "时空行者装",
        title: "时空旅人",
        quality: Quality::Legendary,
        cost_gold: 100000,
        cost_diamond: 120,
        hp_bonus: 200,
        ad_bonus: 50,
        ap_bonus: 50,
        def_bonus: 35,
        desc: "穿越时空的旅人留下的神秘服装",
    },
    CostumeDef {
        id: 14,
        name: "诸神黄昏铠",
        title: "诸神黄昏",
        quality: Quality::Legendary,
        cost_gold: 120000,
        cost_diamond: 150,
        hp_bonus: 300,
        ad_bonus: 60,
        ap_bonus: 30,
        def_bonus: 50,
        desc: "传说在诸神黄昏之战中打造的神器铠甲",
    },
    // === 神话品质 ===
    CostumeDef {
        id: 15,
        name: "创世神装",
        title: "创世之神",
        quality: Quality::Mythic,
        cost_gold: 300000,
        cost_diamond: 500,
        hp_bonus: 500,
        ad_bonus: 80,
        ap_bonus: 80,
        def_bonus: 70,
        desc: "创世神留下的唯一遗物，蕴含无穷力量",
    },
    CostumeDef {
        id: 16,
        name: "混沌至尊袍",
        title: "混沌之主",
        quality: Quality::Mythic,
        cost_gold: 500000,
        cost_diamond: 800,
        hp_bonus: 600,
        ad_bonus: 100,
        ap_bonus: 100,
        def_bonus: 80,
        desc: "混沌之力凝聚而成的至尊法袍，佩戴者掌控混沌",
    },
];

/// 解析拥有的时装ID列表
fn parse_owned(raw: &str) -> Vec<i32> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(',').filter_map(|s| s.trim().parse::<i32>().ok()).collect()
}

/// 序列化拥有的时装ID列表
fn serialize_owned(ids: &[i32]) -> String {
    ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")
}

/// 查找时装定义
fn find_costume(id: i32) -> Option<&'static CostumeDef> {
    COSTUMES.iter().find(|c| c.id == id)
}

/// 格式化属性加成
fn format_bonus(c: &CostumeDef) -> String {
    let mut parts = Vec::new();
    if c.hp_bonus > 0 {
        parts.push(format!("HP+{}", c.hp_bonus));
    }
    if c.ad_bonus > 0 {
        parts.push(format!("物攻+{}", c.ad_bonus));
    }
    if c.ap_bonus > 0 {
        parts.push(format!("魔攻+{}", c.ap_bonus));
    }
    if c.def_bonus > 0 {
        parts.push(format!("防御+{}", c.def_bonus));
    }
    parts.join(" ")
}

/// 检查是否集齐某品质的时装
fn has_quality_set(owned: &[i32], quality: Quality) -> bool {
    COSTUMES
        .iter()
        .filter(|c| c.quality == quality)
        .all(|c| owned.contains(&c.id))
}

/// 获取集齐品质套装加成百分比
fn get_set_bonus_pct(owned: &[i32]) -> i32 {
    let mut total = 0;
    for q in [
        Quality::Common,
        Quality::Rare,
        Quality::Epic,
        Quality::Legendary,
        Quality::Mythic,
    ] {
        if has_quality_set(owned, q) {
            total += q.set_bonus_pct();
        }
    }
    total
}

/// 获取某品质拥有的数量
fn count_quality(owned: &[i32], quality: Quality) -> usize {
    owned
        .iter()
        .filter(|&&id| find_costume(id).map(|c| c.quality == quality).unwrap_or(false))
        .count()
}

/// 获取某品质的总数
fn total_quality(quality: Quality) -> usize {
    COSTUMES.iter().filter(|c| c.quality == quality).count()
}

// ==================== 指令实现 ====================

/// 查看时装商店 — 列出所有可购买的时装
pub fn cmd_view_costumes(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let owned_raw = db.read_user_data(user_id, OWNED_KEY);
    let owned = parse_owned(&owned_raw);
    let equipped: i32 = db.read_user_data(user_id, EQUIPPED_KEY).parse().unwrap_or(0);

    let mut r = format!("{}\n═══ 👗 时装商店 ═══\n", prefix);
    r.push_str(&format!("📦 已收集: {}/{} 件\n\n", owned.len(), COSTUMES.len()));

    // 按品质分组显示
    for q in [
        Quality::Mythic,
        Quality::Legendary,
        Quality::Epic,
        Quality::Rare,
        Quality::Common,
    ] {
        let q_count = count_quality(&owned, q);
        let q_total = total_quality(q);
        r.push_str(&format!("{} {} ({}/{})\n", q.emoji(), q.name(), q_count, q_total));

        for c in COSTUMES.iter().filter(|c| c.quality == q) {
            let status = if c.id == equipped {
                " 👈穿戴中"
            } else if owned.contains(&c.id) {
                " ✅已拥有"
            } else {
                ""
            };
            r.push_str(&format!("  [{}] {} — {}{}\n", c.id, c.name, format_bonus(c), status));
        }
        r.push('\n');
    }

    r.push_str("💡 指令: 购买时装+编号 / 穿戴时装+编号 / 时装图鉴\n");
    r
}

/// 购买时装 — 花费金币/钻石购买指定时装
pub fn cmd_buy_costume(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let id: i32 = match args.trim().parse() {
        Ok(v) => v,
        Err(_) => {
            return format!(
                "{}\n❌ 请输入正确的时装编号！\n💡 用法: 购买时装+编号\n查看时装可查看所有编号",
                prefix
            );
        }
    };

    let costume = match find_costume(id) {
        Some(c) => c,
        None => {
            return format!(
                "{}\n❌ 时装编号 {} 不存在！\n💡 有效编号: 1~{}",
                prefix,
                id,
                COSTUMES.len()
            );
        }
    };

    let owned_raw = db.read_user_data(user_id, OWNED_KEY);
    let mut owned = parse_owned(&owned_raw);

    if owned.contains(&id) {
        return format!(
            "{}\n❌ 您已拥有「{}」！无需重复购买。\n💡 穿戴时装+{} 可装备此时装",
            prefix, costume.name, id
        );
    }

    // 检查金币
    if costume.cost_gold > 0 {
        let gold = db.read_basic(user_id, CURRENCY_GOLD).parse::<i64>().unwrap_or(0);
        if gold < costume.cost_gold {
            return format!(
                "{}\n❌ 金币不足！需要 {} 金币，当前 {} 金币。\n💡 差额: {} 金币",
                prefix,
                costume.cost_gold,
                gold,
                costume.cost_gold - gold
            );
        }
    }

    // 检查钻石
    if costume.cost_diamond > 0 {
        let diamond = db.read_basic(user_id, CURRENCY_DIAMOND).parse::<i64>().unwrap_or(0);
        if diamond < costume.cost_diamond as i64 {
            return format!(
                "{}\n❌ 钻石不足！需要 {} 钻石，当前 {} 钻石。\n💡 差额: {} 钻石",
                prefix,
                costume.cost_diamond,
                diamond,
                costume.cost_diamond as i64 - diamond
            );
        }
    }

    // 扣费
    if costume.cost_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, "sub", costume.cost_gold);
    }
    if costume.cost_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, "sub", costume.cost_diamond as i64);
    }

    // 添加到拥列表
    owned.push(id);
    owned.sort();
    db.write_user_data(user_id, OWNED_KEY, &serialize_owned(&owned));

    let quality_set = has_quality_set(&owned, costume.quality);
    let mut r = format!(
        "{}\n✅ 购买成功！获得时装「{}」{} {}\n",
        prefix,
        costume.name,
        costume.quality.emoji(),
        costume.quality.name()
    );
    r.push_str(&format!("  📛 称号: {}\n", costume.title));
    r.push_str(&format!("  📊 属性: {}\n", format_bonus(costume)));
    if costume.cost_gold > 0 {
        r.push_str(&format!("  💰 花费: {} 金币", costume.cost_gold));
    }
    if costume.cost_diamond > 0 {
        if costume.cost_gold > 0 {
            r.push_str(" + ");
        } else {
            r.push_str("  💰 花费: ");
        }
        r.push_str(&format!("{} 钻石", costume.cost_diamond));
    }
    r.push('\n');

    if quality_set {
        r.push_str(&format!(
            "\n🎉 集齐全部【{}】品质时装！套装加成: 全属性+{}%\n",
            costume.quality.name(),
            costume.quality.set_bonus_pct()
        ));
    }

    r.push_str(&format!("\n💡 穿戴时装+{} 可装备此时装\n", id));
    r
}

/// 穿戴时装 — 装备指定时装获得属性加成
pub fn cmd_equip_costume(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let id: i32 = match args.trim().parse() {
        Ok(v) => v,
        Err(_) => {
            return format!("{}\n❌ 请输入正确的时装编号！\n💡 用法: 穿戴时装+编号", prefix);
        }
    };

    let costume = match find_costume(id) {
        Some(c) => c,
        None => {
            return format!("{}\n❌ 时装编号 {} 不存在！", prefix, id);
        }
    };

    let owned_raw = db.read_user_data(user_id, OWNED_KEY);
    let owned = parse_owned(&owned_raw);

    if !owned.contains(&id) {
        return format!(
            "{}\n❌ 您尚未拥有「{}」！\n💡 请先购买时装+{} 购买",
            prefix, costume.name, id
        );
    }

    let current_equipped: i32 = db.read_user_data(user_id, EQUIPPED_KEY).parse().unwrap_or(0);

    if current_equipped == id {
        return format!("{}\n❌ 您已穿戴「{}」！无需重复穿戴。", prefix, costume.name);
    }

    db.write_user_data(user_id, EQUIPPED_KEY, &id.to_string());

    let mut r = format!(
        "{}\n✅ 穿戴成功！时装「{}」{} 已装备\n",
        prefix,
        costume.name,
        costume.quality.emoji()
    );
    r.push_str(&format!("  📛 获得称号: {}\n", costume.title));
    r.push_str(&format!("  📊 属性加成: {}\n", format_bonus(costume)));

    let set_pct = get_set_bonus_pct(&owned);
    if set_pct > 0 {
        r.push_str(&format!("  🎖️ 套装加成: 全属性+{}%\n", set_pct));
    }

    r.push_str("\n💡 卸下时装 可取下当前时装\n");
    r
}

/// 卸下时装 — 取下当前穿戴的时装
pub fn cmd_unequip_costume(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let current: i32 = db.read_user_data(user_id, EQUIPPED_KEY).parse().unwrap_or(0);

    if current == 0 {
        return format!("{}\n❌ 您当前没有穿戴任何时装！", prefix);
    }

    let name = find_costume(current).map(|c| c.name).unwrap_or("未知时装");

    db.write_user_data(user_id, EQUIPPED_KEY, "0");

    format!("{}\n✅ 已卸下时装「{}」！\n💡 穿戴时装+编号 可重新装备", prefix, name)
}

/// 时装图鉴 — 查看时装收集进度和套装奖励
pub fn cmd_costume_codex(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let owned_raw = db.read_user_data(user_id, OWNED_KEY);
    let owned = parse_owned(&owned_raw);
    let equipped: i32 = db.read_user_data(user_id, EQUIPPED_KEY).parse().unwrap_or(0);

    let mut r = format!("{}\n═══ 📖 时装图鉴 ═══\n", prefix);

    // 当前穿戴
    if equipped > 0 {
        if let Some(c) = find_costume(equipped) {
            r.push_str(&format!(
                "👗 当前穿戴: {} {} [{}]\n",
                c.quality.emoji(),
                c.name,
                c.title
            ));
            r.push_str(&format!("  📊 加成: {}\n\n", format_bonus(c)));
        }
    } else {
        r.push_str("👗 当前未穿戴时装\n\n");
    }

    // 收集进度
    r.push_str(&format!(
        "📦 收集进度: {}/{} ({:.0}%)\n\n",
        owned.len(),
        COSTUMES.len(),
        if COSTUMES.is_empty() {
            0.0
        } else {
            owned.len() as f64 / COSTUMES.len() as f64 * 100.0
        }
    ));

    // 品质收集进度
    r.push_str("📊 品质收集:\n");
    for q in [
        Quality::Mythic,
        Quality::Legendary,
        Quality::Epic,
        Quality::Rare,
        Quality::Common,
    ] {
        let cnt = count_quality(&owned, q);
        let total = total_quality(q);
        let complete = has_quality_set(&owned, q);
        let status = if complete { " ✅ 集齐!" } else { "" };
        let bar = simple_bar(cnt, total, 10);
        r.push_str(&format!(
            "  {} {} {}/{} {}{}\n",
            q.emoji(),
            q.name(),
            cnt,
            total,
            bar,
            status
        ));
    }

    // 套装加成
    let set_pct = get_set_bonus_pct(&owned);
    r.push_str(&format!("\n🎖️ 当前套装加成: 全属性+{}%\n", set_pct));

    // 各品质套装奖励
    r.push_str("\n📋 套装奖励:\n");
    for q in [
        Quality::Common,
        Quality::Rare,
        Quality::Epic,
        Quality::Legendary,
        Quality::Mythic,
    ] {
        let status = if has_quality_set(&owned, q) { "✅" } else { "⬜" };
        r.push_str(&format!(
            "  {} 集齐{}品质 → 全属性+{}%\n",
            status,
            q.name(),
            q.set_bonus_pct()
        ));
    }

    r.push_str("\n💡 购买时装+编号 / 时装排行\n");
    r
}

/// 时装排行 — 按收集数量排行
pub fn cmd_costume_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let _user_name = db.read_basic(user_id, ITEM_NAME);

    // 获取所有用户
    let users = db.all_users();
    let mut rankings: Vec<(String, String, usize, i32)> = Vec::new(); // (uid, name, count, equipped)

    for uid in &users {
        let owned_raw = db.read_user_data(uid, OWNED_KEY);
        let owned = parse_owned(&owned_raw);
        if !owned.is_empty() {
            let name = db.read_basic(uid, ITEM_NAME);
            let equipped: i32 = db.read_user_data(uid, EQUIPPED_KEY).parse().unwrap_or(0);
            rankings.push((uid.clone(), name, owned.len(), equipped));
        }
    }

    rankings.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));

    let mut r = format!("{}\n═══ 🏆 时装排行 ═══\n", prefix);

    if rankings.is_empty() {
        r.push_str("暂无玩家收集时装。\n💡 购买时装+编号 开始收集！");
        return r;
    }

    for (i, (_, name, count, equipped)) in rankings.iter().take(20).enumerate() {
        let medal = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        let eq_name = if *equipped > 0 {
            find_costume(*equipped)
                .map(|c| format!(" [{}]", c.name))
                .unwrap_or_default()
        } else {
            String::new()
        };
        r.push_str(&format!("{} {}. {} — {} 件{}\n", medal, i + 1, name, count, eq_name));
    }

    // 显示自己的排名
    if let Some(pos) = rankings.iter().position(|(uid, _, _, _)| uid == user_id) {
        r.push_str(&format!("\n📍 您的排名: 第 {} 名 ({} 件)\n", pos + 1, rankings[pos].2));
    }

    r.push_str("\n💡 时装图鉴 查看收集详情\n");
    r
}

/// 简单进度条
fn simple_bar(current: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "░".repeat(width);
    }
    let filled = (current * width + total / 2) / total;
    let filled = filled.min(width);
    let mut s = "█".repeat(filled);
    s.push_str(&"░".repeat(width - filled));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_costume_count() {
        assert_eq!(COSTUMES.len(), 16);
    }

    #[test]
    fn test_costume_ids_unique() {
        let mut ids: Vec<i32> = COSTUMES.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), COSTUMES.len());
    }

    #[test]
    fn test_costume_names_unique() {
        let mut names: Vec<&str> = COSTUMES.iter().map(|c| c.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), COSTUMES.len());
    }

    #[test]
    fn test_quality_distribution() {
        let common = total_quality(Quality::Common);
        let rare = total_quality(Quality::Rare);
        let epic = total_quality(Quality::Epic);
        let legendary = total_quality(Quality::Legendary);
        let mythic = total_quality(Quality::Mythic);
        assert!(common >= 2);
        assert!(rare >= 2);
        assert!(epic >= 2);
        assert!(legendary >= 2);
        assert!(mythic >= 1);
        assert_eq!(common + rare + epic + legendary + mythic, COSTUMES.len());
    }

    #[test]
    fn test_quality_emoji() {
        assert_eq!(Quality::Common.emoji(), "⚪");
        assert_eq!(Quality::Rare.emoji(), "🟢");
        assert_eq!(Quality::Epic.emoji(), "🟣");
        assert_eq!(Quality::Legendary.emoji(), "🟡");
        assert_eq!(Quality::Mythic.emoji(), "🔴");
    }

    #[test]
    fn test_quality_set_bonus() {
        assert!(Quality::Common.set_bonus_pct() < Quality::Rare.set_bonus_pct());
        assert!(Quality::Rare.set_bonus_pct() < Quality::Epic.set_bonus_pct());
        assert!(Quality::Epic.set_bonus_pct() < Quality::Legendary.set_bonus_pct());
        assert!(Quality::Legendary.set_bonus_pct() < Quality::Mythic.set_bonus_pct());
    }

    #[test]
    fn test_parse_owned_empty() {
        let ids = parse_owned("");
        assert!(ids.is_empty());
    }

    #[test]
    fn test_parse_owned_single() {
        let ids = parse_owned("5");
        assert_eq!(ids, vec![5]);
    }

    #[test]
    fn test_parse_owned_multiple() {
        let ids = parse_owned("1,3,5,7");
        assert_eq!(ids, vec![1, 3, 5, 7]);
    }

    #[test]
    fn test_parse_owned_with_spaces() {
        let ids = parse_owned(" 1 , 3 , 5 ");
        assert_eq!(ids, vec![1, 3, 5]);
    }

    #[test]
    fn test_serialize_owned() {
        assert_eq!(serialize_owned(&[1, 3, 5]), "1,3,5");
    }

    #[test]
    fn test_serialize_owned_empty() {
        assert_eq!(serialize_owned(&[]), "");
    }

    #[test]
    fn test_find_costume_valid() {
        let c = find_costume(1).unwrap();
        assert_eq!(c.name, "布衣素衫");
        assert_eq!(c.quality, Quality::Common);
    }

    #[test]
    fn test_find_costume_invalid() {
        assert!(find_costume(999).is_none());
        assert!(find_costume(0).is_none());
        assert!(find_costume(-1).is_none());
    }

    #[test]
    fn test_find_costume_mythic() {
        let c = find_costume(15).unwrap();
        assert_eq!(c.quality, Quality::Mythic);
        assert_eq!(c.name, "创世神装");
    }

    #[test]
    fn test_format_bonus() {
        let c = find_costume(1).unwrap();
        let bonus = format_bonus(c);
        assert!(bonus.contains("HP+20"));
        assert!(bonus.contains("物攻+5"));
        assert!(bonus.contains("防御+3"));
    }

    #[test]
    fn test_format_bonus_mythic() {
        let c = find_costume(16).unwrap();
        let bonus = format_bonus(c);
        assert!(bonus.contains("HP+600"));
        assert!(bonus.contains("物攻+100"));
        assert!(bonus.contains("魔攻+100"));
        assert!(bonus.contains("防御+80"));
    }

    #[test]
    fn test_has_quality_set_empty() {
        assert!(!has_quality_set(&[], Quality::Common));
    }

    #[test]
    fn test_has_quality_set_partial() {
        assert!(!has_quality_set(&[1, 2, 3], Quality::Common)); // missing 4
    }

    #[test]
    fn test_has_quality_set_complete() {
        assert!(has_quality_set(&[1, 2, 3, 4], Quality::Common));
    }

    #[test]
    fn test_has_quality_set_wrong_ids() {
        assert!(!has_quality_set(&[1, 2, 3, 5], Quality::Common)); // 5 is rare
    }

    #[test]
    fn test_get_set_bonus_empty() {
        assert_eq!(get_set_bonus_pct(&[]), 0);
    }

    #[test]
    fn test_get_set_bonus_common_only() {
        let pct = get_set_bonus_pct(&[1, 2, 3, 4]);
        assert_eq!(pct, Quality::Common.set_bonus_pct());
    }

    #[test]
    fn test_get_set_bonus_common_and_rare() {
        let pct = get_set_bonus_pct(&[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(pct, Quality::Common.set_bonus_pct() + Quality::Rare.set_bonus_pct());
    }

    #[test]
    fn test_count_quality() {
        assert_eq!(count_quality(&[1, 2, 5], Quality::Common), 2);
        assert_eq!(count_quality(&[1, 2, 5], Quality::Rare), 1);
        assert_eq!(count_quality(&[], Quality::Common), 0);
    }

    #[test]
    fn test_simple_bar_full() {
        let bar = simple_bar(10, 10, 10);
        assert!(bar.contains("█"));
        assert!(!bar.contains("░"));
    }

    #[test]
    fn test_simple_bar_empty() {
        let bar = simple_bar(0, 10, 10);
        assert!(!bar.contains("█"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_simple_bar_half() {
        let bar = simple_bar(5, 10, 10);
        assert!(bar.contains("█"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_simple_bar_zero_total() {
        let bar = simple_bar(0, 0, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_costume_cost_escalates() {
        let common_costs: Vec<i64> = COSTUMES
            .iter()
            .filter(|c| c.quality == Quality::Common)
            .map(|c| c.cost_gold)
            .collect();
        let mythic_costs: Vec<i64> = COSTUMES
            .iter()
            .filter(|c| c.quality == Quality::Mythic)
            .map(|c| c.cost_gold)
            .collect();
        let avg_common: i64 = common_costs.iter().sum::<i64>() / common_costs.len() as i64;
        let avg_mythic: i64 = mythic_costs.iter().sum::<i64>() / mythic_costs.len() as i64;
        assert!(avg_mythic > avg_common * 10);
    }

    #[test]
    fn test_mythic_hp_bonus_escalates() {
        let mythic_min_hp = COSTUMES
            .iter()
            .filter(|c| c.quality == Quality::Mythic)
            .map(|c| c.hp_bonus)
            .min()
            .unwrap_or(0);
        let common_max_hp = COSTUMES
            .iter()
            .filter(|c| c.quality == Quality::Common)
            .map(|c| c.hp_bonus)
            .max()
            .unwrap_or(0);
        assert!(mythic_min_hp > common_max_hp * 5);
    }

    #[test]
    fn test_all_positive_bonuses() {
        for c in COSTUMES {
            assert!(c.hp_bonus > 0, "Costume {} should have HP bonus", c.id);
            // At least one offensive stat should be positive
            assert!(
                c.ad_bonus > 0 || c.ap_bonus > 0,
                "Costume {} should have at least one offensive bonus",
                c.id
            );
            assert!(c.def_bonus > 0, "Costume {} should have def bonus", c.id);
        }
    }

    #[test]
    fn test_all_costs_positive() {
        for c in COSTUMES {
            assert!(
                c.cost_gold > 0 || c.cost_diamond > 0,
                "Costume {} should have a cost",
                c.id
            );
        }
    }

    #[test]
    fn test_all_have_names_and_titles() {
        for c in COSTUMES {
            assert!(!c.name.is_empty());
            assert!(!c.title.is_empty());
            assert!(!c.desc.is_empty());
        }
    }
}

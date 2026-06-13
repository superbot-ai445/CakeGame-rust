/// CakeGame 灵兽驯养系统
/// 全新系统：在野外地图捕获怪物作为灵兽伙伴，喂养进化，战斗加成
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 灵兽品质
#[derive(Debug, Clone, Copy)]
enum BeastRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl BeastRarity {
    fn name(&self) -> &str {
        match self {
            Self::Common => "普通",
            Self::Uncommon => "优秀",
            Self::Rare => "稀有",
            Self::Epic => "史诗",
            Self::Legendary => "传说",
        }
    }

    fn emoji(&self) -> &str {
        match self {
            Self::Common => "⚪",
            Self::Uncommon => "🟢",
            Self::Rare => "🔵",
            Self::Epic => "🟣",
            Self::Legendary => "🟡",
        }
    }

    fn capture_rate(&self) -> f64 {
        match self {
            Self::Common => 0.80,
            Self::Uncommon => 0.50,
            Self::Rare => 0.30,
            Self::Epic => 0.15,
            Self::Legendary => 0.05,
        }
    }

    #[allow(dead_code)]
    fn from_monster_type(t: &str) -> Self {
        match t {
            "Ordinary" => Self::Common,
            "Boss" => Self::Rare,
            _ => Self::Uncommon,
        }
    }
}

/// 灵兽定义 (从怪物转化)
struct BeastTemplate {
    name: &'static str,
    species: &'static str,
    rarity: BeastRarity,
    base_hp: i32,
    base_ad: i32,
    base_def: i32,
    base_mdf: i32,
    skill: &'static str,
    skill_desc: &'static str,
    #[allow(dead_code)]
    favorite_food: &'static str,
    evolves_to: &'static str,
    evolve_level: i32,
    evolve_loyalty: i32,
}

/// 内置灵兽模板 (基于 Config_Monster 数据)
const BEAST_TEMPLATES: &[BeastTemplate] = &[
    BeastTemplate {
        name: "史莱姆",
        species: "黏液系",
        rarity: BeastRarity::Common,
        base_hp: 30,
        base_ad: 8,
        base_def: 5,
        base_mdf: 3,
        skill: "黏液弹",
        skill_desc: "发射黏液弹，降低目标15%速度",
        favorite_food: "果冻",
        evolves_to: "巨型史莱姆",
        evolve_level: 10,
        evolve_loyalty: 50,
    },
    BeastTemplate {
        name: "哥布林",
        species: "人形系",
        rarity: BeastRarity::Common,
        base_hp: 25,
        base_ad: 12,
        base_def: 4,
        base_mdf: 2,
        skill: "偷袭",
        skill_desc: "突然袭击，25%概率双倍伤害",
        favorite_food: "烤肉",
        evolves_to: "哥布林王",
        evolve_level: 15,
        evolve_loyalty: 60,
    },
    BeastTemplate {
        name: "牛头人",
        species: "兽人系",
        rarity: BeastRarity::Uncommon,
        base_hp: 50,
        base_ad: 15,
        base_def: 10,
        base_mdf: 5,
        skill: "冲锋",
        skill_desc: "强力冲锋，击退目标并眩晕1回合",
        favorite_food: "力量药水",
        evolves_to: "牛头巨兽",
        evolve_level: 12,
        evolve_loyalty: 70,
    },
    BeastTemplate {
        name: "十夫长",
        species: "战士系",
        rarity: BeastRarity::Uncommon,
        base_hp: 40,
        base_ad: 18,
        base_def: 8,
        base_mdf: 6,
        skill: "战吼",
        skill_desc: "鼓舞士气，主人攻击力+10%持续3回合",
        favorite_food: "铁甲药水",
        evolves_to: "百夫长",
        evolve_level: 18,
        evolve_loyalty: 75,
    },
    BeastTemplate {
        name: "火烧拉拉肥",
        species: "元素系",
        rarity: BeastRarity::Rare,
        base_hp: 35,
        base_ad: 25,
        base_def: 6,
        base_mdf: 12,
        skill: "烈焰吐息",
        skill_desc: "喷射烈焰，附带灼烧效果持续扣血",
        favorite_food: "火焰精华",
        evolves_to: "熔岩巨龙",
        evolve_level: 25,
        evolve_loyalty: 85,
    },
    BeastTemplate {
        name: "雷劈拉拉肥",
        species: "元素系",
        rarity: BeastRarity::Rare,
        base_hp: 35,
        base_ad: 22,
        base_def: 5,
        base_mdf: 15,
        skill: "雷电链",
        skill_desc: "释放雷电链，可连锁攻击3个目标",
        favorite_food: "雷电结晶",
        evolves_to: "雷霆巨龙",
        evolve_level: 25,
        evolve_loyalty: 85,
    },
    BeastTemplate {
        name: "活死人",
        species: "亡灵系",
        rarity: BeastRarity::Uncommon,
        base_hp: 60,
        base_ad: 10,
        base_def: 12,
        base_mdf: 8,
        skill: "不死之身",
        skill_desc: "死亡后30%概率复活，恢复20%生命",
        favorite_food: "暗影精华",
        evolves_to: "亡灵领主",
        evolve_level: 20,
        evolve_loyalty: 80,
    },
    BeastTemplate {
        name: "无头骑士",
        species: "亡灵系",
        rarity: BeastRarity::Epic,
        base_hp: 80,
        base_ad: 30,
        base_def: 20,
        base_mdf: 15,
        skill: "死亡冲锋",
        skill_desc: "以生命为代价发动致命冲锋，伤害+50%但自损10%",
        favorite_food: "灵魂之石",
        evolves_to: "死亡领主",
        evolve_level: 30,
        evolve_loyalty: 90,
    },
];

/// 灵兽品质映射 (基于怪物等级)
fn get_beast_rarity(level: i32) -> BeastRarity {
    if level >= 40 {
        BeastRarity::Epic
    } else if level >= 25 {
        BeastRarity::Rare
    } else if level >= 15 {
        BeastRarity::Uncommon
    } else {
        BeastRarity::Common
    }
}

/// 查看灵兽 - 显示当前地图可捕获的灵兽
pub fn cmd_view_beasts(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let location = db.read_basic(user_id, ITEM_LOCATION);

    let mut out = format!("{}\n", prefix);
    out += "🐾 ═══ 灵兽驯养系统 ═══\n\n";

    // 显示当前地图可捕获的灵兽
    out += &format!("📍 当前地图: {}\n\n", location);

    // 从 Config_Monster 获取当前地图的怪物
    let (monster_names, map_level) = match db.map_get(&location) {
        Some(map_def) => {
            let lvl = map_def.level;
            (map_def.monsters, lvl)
        }
        None => (vec![], 1),
    };
    if monster_names.is_empty() {
        out += "🏜️ 当前地图没有可捕获的灵兽\n";
        out += "💡 前往其他地图寻找灵兽吧！\n\n";
    } else {
        out += "🔍 当前地图可捕获的灵兽:\n";
        for (i, monster_name) in monster_names.iter().enumerate() {
            if let Some(m) = db.monster_get(monster_name) {
                let rarity = get_beast_rarity(map_level);
                let capture_rate = rarity.capture_rate() * 100.0;
                out += &format!(
                    "{}. {}[{}] {} Lv.{}\n",
                    i + 1,
                    rarity.emoji(),
                    rarity.name(),
                    m.name,
                    map_level
                );
                out += &format!("   ❤️{} ⚔️{} 🛡️{} 🔮{}\n", m.hp, m.ad, m.defense, m.magic_resistance);
                out += &format!("   捕获率: {:.0}%\n", capture_rate);
            }
        }
    }

    out += "\n📌 指令列表:\n";
    out += "• 捕获灵兽+怪物名 - 捕获当前地图的灵兽\n";
    out += "• 我的灵兽 - 查看已捕获的灵兽\n";
    out += "• 灵兽图鉴 - 查看灵兽图鉴\n";
    out += "• 灵兽出战+灵兽名 - 设置出战灵兽\n";
    out += "• 灵兽喂食+灵兽名 - 喂养灵兽增加忠诚度\n";
    out += "• 灵兽进化+灵兽名 - 进化灵兽\n";
    out
}

/// 捕获灵兽 - 在当前地图捕获怪物作为灵兽
pub fn cmd_capture_beast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!(
            "{}\n❌ 请指定要捕获的灵兽名称！\n💡 使用「查看灵兽」查看当前地图可捕获的灵兽",
            prefix
        );
    }

    // 检查玩家状态
    let hp: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n❌ 您已阵亡，无法捕获灵兽！请先恢复生命。", prefix);
    }

    // 检查当前位置
    let location = db.read_basic(user_id, ITEM_LOCATION);
    let monster_names = match db.map_get(&location) {
        Some(map_def) => map_def.monsters,
        None => vec![],
    };

    // 查找目标怪物并获取其属性
    let target_monster_name = monster_names
        .iter()
        .find(|name| name.contains(target_name) || target_name.contains(name.as_str()));

    let monster = match target_monster_name.and_then(|name| db.monster_get(name)) {
        Some(m) => m,
        None => {
            return format!(
                "{}\n❌ 当前地图没有找到「{}」！\n💡 使用「查看灵兽」查看当前地图可捕获的灵兽",
                prefix, target_name
            );
        }
    };

    let monster_name = monster.name.clone();
    let map_level = db.map_get(&location).map(|mp| mp.level).unwrap_or(1);
    let level = map_level;
    let hp_val = monster.hp;
    let ad = monster.ad;
    let def = monster.defense;
    let mdf = monster.magic_resistance;

    // 计算捕获概率
    let rarity = get_beast_rarity(level);
    let base_rate = rarity.capture_rate();

    // 玩家等级加成 (每级+1%)
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let level_bonus = (user_level as f64 * 0.01).min(0.20); // 最多+20%

    // 生成随机数
    let rand_val: f64 = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        (nanos % 10000) as f64 / 10000.0
    };

    let final_rate = (base_rate + level_bonus).min(0.95);
    let success = rand_val < final_rate;

    let mut out = format!("{}\n", prefix);
    out += &format!("🎯 目标: {}[{}] Lv.{}\n", rarity.emoji(), monster_name, level);
    out += &format!(
        "📊 捕获率: {:.0}% (+等级加成{:.0}%)\n",
        base_rate * 100.0,
        level_bonus * 100.0
    );

    if success {
        // 捕获成功 - 检查灵兽数量限制
        let beast_count = db.read_user_data(user_id, "beast_count").parse::<i32>().unwrap_or(0);
        if beast_count >= 6 {
            return format!(
                "{}\n❌ 灵兽栏已满！最多同时拥有6只灵兽\n💡 使用「放生灵兽+灵兽名」释放灵兽腾出空间",
                prefix
            );
        }

        // 保存灵兽数据
        let beast_id = format!("beast_{}", beast_count + 1);
        let beast_data = format!(
            "{},{},{},{},{},{},{},{},{}",
            monster_name,
            "未分类",
            level,
            hp_val,
            ad,
            def,
            mdf,
            50, // 初始忠诚度
            0   // 初始经验
        );
        db.write_user_data(user_id, &beast_id, &beast_data);
        db.write_user_data(user_id, "beast_count", &(beast_count + 1).to_string());

        // 添加到灵兽图鉴
        let mut codex = db.read_user_data(user_id, "beast_codex");
        if !codex.contains(&monster_name) {
            if codex.is_empty() {
                codex = monster_name.clone();
            } else {
                codex.push(',');
                codex.push_str(&monster_name);
            }
            db.write_user_data(user_id, "beast_codex", &codex);
        }

        out += "✅ 捕获成功！\n\n";
        out += &format!("🎉 获得新灵兽: {}[{}]\n", rarity.emoji(), monster_name);
        out += "   种族: 未分类\n";
        out += &format!("   等级: Lv.{}\n", level);
        out += "   忠诚度: 50/100\n";
        out += &format!("   ❤️{} ⚔️{} 🛡️{} 🔮{}\n", hp_val, ad, def, mdf);
        out += "\n💡 使用「灵兽出战+灵兽名」设置出战灵兽\n";
    } else {
        out += "❌ 捕获失败！灵兽逃跑了...\n";
        out += "💡 提升等级可以增加捕获成功率\n";
    }

    out
}

/// 我的灵兽 - 查看已捕获的灵兽列表
pub fn cmd_my_beasts(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let beast_count = db.read_user_data(user_id, "beast_count").parse::<i32>().unwrap_or(0);
    let active_beast = db.read_user_data(user_id, "active_beast");

    let mut out = format!("{}\n", prefix);
    out += "🐾 ═══ 我的灵兽 ═══\n\n";

    if beast_count == 0 {
        out += "📭 还没有捕获任何灵兽！\n";
        out += "💡 使用「查看灵兽」查看当前地图可捕获的灵兽\n";
        out += "💡 使用「捕获灵兽+怪物名」捕获灵兽\n";
        return out;
    }

    out += &format!("📊 灵兽栏: {}/6\n\n", beast_count);

    for i in 1..=beast_count {
        let beast_id = format!("beast_{}", i);
        let beast_data = db.read_user_data(user_id, &beast_id);

        if beast_data.is_empty() {
            continue;
        }

        let parts: Vec<&str> = beast_data.split(',').collect();
        if parts.len() < 9 {
            continue;
        }

        let name = parts[0];
        let species = parts[1];
        let level: i32 = parts[2].parse().unwrap_or(1);
        let hp: i32 = parts[3].parse().unwrap_or(0);
        let ad: i32 = parts[4].parse().unwrap_or(0);
        let def: i32 = parts[5].parse().unwrap_or(0);
        let mdf: i32 = parts[6].parse().unwrap_or(0);
        let loyalty: i32 = parts[7].parse().unwrap_or(0);
        let exp: i32 = parts[8].parse().unwrap_or(0);

        let rarity = get_beast_rarity(level);
        let is_active = active_beast == beast_id;

        let loyalty_bar = "█".repeat((loyalty / 10) as usize) + &"░".repeat(10 - (loyalty / 10) as usize);

        out += &format!(
            "{}{}. {}[{}] Lv.{} {}\n",
            if is_active { "⚔️ " } else { "" },
            i,
            rarity.emoji(),
            name,
            level,
            if is_active { "[出战中]" } else { "" }
        );
        out += &format!("   种族: {} | 忠诚度: [{}] {}/100\n", species, loyalty_bar, loyalty);
        out += &format!("   ❤️生命:{} ⚔️物攻:{} 🛡️防御:{} 🔮魔抗:{}\n", hp, ad, def, mdf);
        out += &format!("   经验: {}/{}\n", exp, level * 100);
        out += "\n";
    }

    out += "📌 指令:\n";
    out += "• 灵兽出战+灵兽名 - 设置出战灵兽\n";
    out += "• 灵兽喂食+灵兽名 - 喂养灵兽\n";
    out += "• 灵兽进化+灵兽名 - 进化灵兽\n";
    out += "• 放生灵兽+灵兽名 - 放生灵兽\n";
    out
}

/// 灵兽出战 - 设置出战灵兽
pub fn cmd_set_active_beast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!(
            "{}\n❌ 请指定要出战的灵兽名称！\n💡 使用「我的灵兽」查看已拥有的灵兽",
            prefix
        );
    }

    let beast_count = db.read_user_data(user_id, "beast_count").parse::<i32>().unwrap_or(0);
    if beast_count == 0 {
        return format!("{}\n❌ 您还没有任何灵兽！\n💡 使用「捕获灵兽+怪物名」捕获灵兽", prefix);
    }

    // 查找灵兽
    for i in 1..=beast_count {
        let beast_id = format!("beast_{}", i);
        let beast_data = db.read_user_data(user_id, &beast_id);

        if beast_data.is_empty() {
            continue;
        }

        let parts: Vec<&str> = beast_data.split(',').collect();
        if parts.is_empty() {
            continue;
        }

        let name = parts[0];
        if name.contains(target_name) || target_name.contains(name) {
            let level: i32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let loyalty: i32 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);

            // 检查忠诚度要求
            if loyalty < 20 {
                return format!(
                    "{}\n❌ {}的忠诚度太低，不愿出战！\n💡 忠诚度需要达到20以上，使用「灵兽喂食+{}」提升忠诚度",
                    prefix, name, name
                );
            }

            db.write_user_data(user_id, "active_beast", &beast_id);

            let rarity = get_beast_rarity(level);
            let mut out = format!("{}\n", prefix);
            out += "✅ 设置成功！\n\n";
            out += &format!("⚔️ 出战灵兽: {}[{}] Lv.{}\n", rarity.emoji(), name, level);
            out += &format!("   忠诚度: {}/100\n", loyalty);
            out += "\n💡 出战灵兽将在战斗中为您提供属性加成和特殊技能！\n";
            return out;
        }
    }

    format!(
        "{}\n❌ 没有找到名为「{}」的灵兽！\n💡 使用「我的灵兽」查看已拥有的灵兽",
        prefix, target_name
    )
}

/// 灵兽喂食 - 喂养灵兽增加忠诚度
pub fn cmd_feed_beast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!(
            "{}\n❌ 请指定要喂食的灵兽名称！\n💡 使用「我的灵兽」查看已拥有的灵兽",
            prefix
        );
    }

    let beast_count = db.read_user_data(user_id, "beast_count").parse::<i32>().unwrap_or(0);
    if beast_count == 0 {
        return format!("{}\n❌ 您还没有任何灵兽！\n💡 使用「捕获灵兽+怪物名」捕获灵兽", prefix);
    }

    // 查找灵兽
    for i in 1..=beast_count {
        let beast_id = format!("beast_{}", i);
        let beast_data = db.read_user_data(user_id, &beast_id);

        if beast_data.is_empty() {
            continue;
        }

        let parts: Vec<&str> = beast_data.split(',').collect();
        if parts.len() < 9 {
            continue;
        }

        let name = parts[0];
        if name.contains(target_name) || target_name.contains(name) {
            let level: i32 = parts[2].parse().unwrap_or(1);
            let hp: i32 = parts[3].parse().unwrap_or(0);
            let ad: i32 = parts[4].parse().unwrap_or(0);
            let def: i32 = parts[5].parse().unwrap_or(0);
            let mdf: i32 = parts[6].parse().unwrap_or(0);
            let loyalty: i32 = parts[7].parse().unwrap_or(0);
            let exp: i32 = parts[8].parse().unwrap_or(0);

            // 检查忠诚度上限
            if loyalty >= 100 {
                return format!(
                    "{}\n❌ {}的忠诚度已经达到上限100！\n💡 可以尝试「灵兽进化+{}」进化灵兽",
                    prefix, name, name
                );
            }

            // 喂食消耗金币
            let gold_cost = 100 * level as i64;
            let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
            if user_gold < gold_cost {
                return format!(
                    "{}\n❌ 金币不足！喂食{}需要{}金币\n💡 当前金币: {}",
                    prefix, name, gold_cost, user_gold
                );
            }

            // 扣除金币
            db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_cost);

            // 增加忠诚度 (随机+5~15)
            let loyalty_gain = 5 + (level % 11);
            let new_loyalty = (loyalty + loyalty_gain).min(100);

            // 增加经验 (随机+10~30)
            let exp_gain = 10 + (level % 21);
            let new_exp = exp + exp_gain;

            // 检查升级
            let mut new_level = level;
            let mut new_hp = hp;
            let mut new_ad = ad;
            let mut new_def = def;
            let mut new_mdf = mdf;
            let mut remaining_exp = new_exp;

            while remaining_exp >= new_level * 100 {
                remaining_exp -= new_level * 100;
                new_level += 1;
                // 升级属性增长
                new_hp += 5 + (new_level / 5);
                new_ad += 3 + (new_level / 8);
                new_def += 2 + (new_level / 10);
                new_mdf += 2 + (new_level / 10);
            }

            // 保存更新后的灵兽数据
            let new_data = format!(
                "{},{},{},{},{},{},{},{},{}",
                name, parts[1], new_level, new_hp, new_ad, new_def, new_mdf, new_loyalty, remaining_exp
            );
            db.write_user_data(user_id, &beast_id, &new_data);

            let rarity = get_beast_rarity(new_level);
            let mut out = format!("{}\n", prefix);
            out += "🍖 喂食成功！\n\n";
            out += &format!("{} {} Lv.{}\n", rarity.emoji(), name, new_level);
            out += &format!("💰 消耗: {}金币\n", gold_cost);
            out += &format!("💖 忠诚度: {} → {}/100 (+{})\n", loyalty, new_loyalty, loyalty_gain);
            out += &format!(
                "⭐ 经验: {} → {}/{} (+{})\n",
                exp,
                remaining_exp,
                new_level * 100,
                exp_gain
            );

            if new_level > level {
                out += &format!("\n🎉 灵兽升级了！Lv.{} → Lv.{}\n", level, new_level);
                out += &format!(
                    "   ❤️+{} ⚔️+{} 🛡️+{} 🔮+{}\n",
                    new_hp - hp,
                    new_ad - ad,
                    new_def - def,
                    new_mdf - mdf
                );
            }

            return out;
        }
    }

    format!(
        "{}\n❌ 没有找到名为「{}」的灵兽！\n💡 使用「我的灵兽」查看已拥有的灵兽",
        prefix, target_name
    )
}

/// 灵兽进化 - 进化灵兽到更高形态
pub fn cmd_evolve_beast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!(
            "{}\n❌ 请指定要进化的灵兽名称！\n💡 使用「我的灵兽」查看已拥有的灵兽",
            prefix
        );
    }

    let beast_count = db.read_user_data(user_id, "beast_count").parse::<i32>().unwrap_or(0);
    if beast_count == 0 {
        return format!("{}\n❌ 您还没有任何灵兽！\n💡 使用「捕获灵兽+怪物名」捕获灵兽", prefix);
    }

    // 查找灵兽
    for i in 1..=beast_count {
        let beast_id = format!("beast_{}", i);
        let beast_data = db.read_user_data(user_id, &beast_id);

        if beast_data.is_empty() {
            continue;
        }

        let parts: Vec<&str> = beast_data.split(',').collect();
        if parts.len() < 9 {
            continue;
        }

        let name = parts[0];
        if name.contains(target_name) || target_name.contains(name) {
            let level: i32 = parts[2].parse().unwrap_or(1);
            let hp: i32 = parts[3].parse().unwrap_or(0);
            let ad: i32 = parts[4].parse().unwrap_or(0);
            let def: i32 = parts[5].parse().unwrap_or(0);
            let mdf: i32 = parts[6].parse().unwrap_or(0);
            let loyalty: i32 = parts[7].parse().unwrap_or(0);
            let exp: i32 = parts[8].parse().unwrap_or(0);

            // 查找进化模板
            let template = BEAST_TEMPLATES.iter().find(|t| t.name == name);

            let (evolve_name, evolve_level_req, evolve_loyalty_req) = if let Some(t) = template {
                (t.evolves_to.to_string(), t.evolve_level, t.evolve_loyalty)
            } else {
                // 通用进化要求
                ("未知形态".to_string(), 20, 80)
            };

            // 检查进化条件
            let mut errors = Vec::new();
            if level < evolve_level_req {
                errors.push(format!("等级不足 (当前:{}, 需求:{})", level, evolve_level_req));
            }
            if loyalty < evolve_loyalty_req {
                errors.push(format!("忠诚度不足 (当前:{}, 需求:{})", loyalty, evolve_loyalty_req));
            }

            if !errors.is_empty() {
                let mut out = format!("{}\n", prefix);
                out += &format!("❌ {} 无法进化！\n\n", name);
                out += "📊 进化条件:\n";
                out += &format!("   目标形态: {}\n", evolve_name);
                out += &format!(
                    "   需求等级: Lv.{} {}\n",
                    evolve_level_req,
                    if level >= evolve_level_req { "✅" } else { "❌" }
                );
                out += &format!(
                    "   需求忠诚: {}/100 {}\n",
                    evolve_loyalty_req,
                    if loyalty >= evolve_loyalty_req { "✅" } else { "❌" }
                );
                out += "\n💡 继续喂食和战斗来提升灵兽等级和忠诚度！\n";
                return out;
            }

            // 进化消耗金币
            let gold_cost = 5000 * level as i64;
            let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
            if user_gold < gold_cost {
                return format!(
                    "{}\n❌ 金币不足！进化需要{}金币\n💡 当前金币: {}",
                    prefix, gold_cost, user_gold
                );
            }

            // 扣除金币
            db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_cost);

            // 进化属性大幅提升
            let new_hp = hp * 2 + 50;
            let new_ad = ad * 2 + 20;
            let new_def = def * 2 + 15;
            let new_mdf = mdf * 2 + 15;

            // 保存进化后的灵兽数据
            let new_data = format!(
                "{},{},{},{},{},{},{},{},{}",
                evolve_name,
                parts[1], // 保持原种族
                level,
                new_hp,
                new_ad,
                new_def,
                new_mdf,
                loyalty,
                exp
            );
            db.write_user_data(user_id, &beast_id, &new_data);

            // 更新灵兽图鉴
            let mut codex = db.read_user_data(user_id, "beast_codex");
            if !codex.contains(&evolve_name) {
                if codex.is_empty() {
                    codex = evolve_name.clone();
                } else {
                    codex.push(',');
                    codex.push_str(&evolve_name);
                }
                db.write_user_data(user_id, "beast_codex", &codex);
            }

            let _rarity = get_beast_rarity(level);
            let mut out = format!("{}\n", prefix);
            out += "🌟 ═══ 灵兽进化成功！═══ 🌟\n\n";
            out += &format!("{} → {}\n", name, evolve_name);
            out += &format!("💰 消耗: {}金币\n\n", gold_cost);
            out += "📊 属性提升:\n";
            out += &format!("   ❤️生命: {} → {} (+{})\n", hp, new_hp, new_hp - hp);
            out += &format!("   ⚔️物攻: {} → {} (+{})\n", ad, new_ad, new_ad - ad);
            out += &format!("   🛡️防御: {} → {} (+{})\n", def, new_def, new_def - def);
            out += &format!("   🔮魔抗: {} → {} (+{})\n", mdf, new_mdf, new_mdf - mdf);
            out += "\n🎉 进化后的灵兽更加强大，继续培养吧！\n";

            return out;
        }
    }

    format!(
        "{}\n❌ 没有找到名为「{}」的灵兽！\n💡 使用「我的灵兽」查看已拥有的灵兽",
        prefix, target_name
    )
}

/// 灵兽图鉴 - 查看已发现的灵兽种类
pub fn cmd_beast_codex(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let codex = db.read_user_data(user_id, "beast_codex");

    let mut out = format!("{}\n", prefix);
    out += "📖 ═══ 灵兽图鉴 ═══\n\n";

    if codex.is_empty() {
        out += "📭 还没有发现任何灵兽！\n";
        out += "💡 前往不同地图探索，捕获各种灵兽吧！\n";
        return out;
    }

    let discovered: Vec<&str> = codex.split(',').collect();
    let total = BEAST_TEMPLATES.len();
    let found = discovered.len();

    out += &format!(
        "📊 发现进度: {}/{} ({:.0}%)\n\n",
        found,
        total,
        (found as f64 / total as f64) * 100.0
    );

    // 按品质分组显示
    let rarities = [
        BeastRarity::Common,
        BeastRarity::Uncommon,
        BeastRarity::Rare,
        BeastRarity::Epic,
        BeastRarity::Legendary,
    ];

    for rarity in &rarities {
        let templates: Vec<&BeastTemplate> = BEAST_TEMPLATES
            .iter()
            .filter(|t| std::mem::discriminant(&t.rarity) == std::mem::discriminant(rarity))
            .collect();

        if templates.is_empty() {
            continue;
        }

        out += &format!("━━━ {} {} ━━━\n", rarity.emoji(), rarity.name());

        for template in templates {
            let is_discovered = discovered.contains(&template.name);
            let status = if is_discovered { "✅" } else { "❓" };

            if is_discovered {
                out += &format!("{} {}[{}]\n", status, template.name, template.species);
                out += &format!(
                    "   ❤️{} ⚔️{} 🛡️{} 🔮{}\n",
                    template.base_hp, template.base_ad, template.base_def, template.base_mdf
                );
                out += &format!("   技能: {} - {}\n", template.skill, template.skill_desc);
                out += &format!(
                    "   进化: {} (Lv.{}/忠诚{})\n",
                    template.evolves_to, template.evolve_level, template.evolve_loyalty
                );
            } else {
                out += &format!("{} ???[未知]\n", status);
                out += "   尚未发现，前往野外探索吧！\n";
            }
            out += "\n";
        }
    }

    out += "💡 捕获更多灵兽来完善图鉴！\n";
    out
}

/// 放生灵兽 - 释放灵兽腾出空间
pub fn cmd_release_beast(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!(
            "{}\n❌ 请指定要放生的灵兽名称！\n💡 使用「我的灵兽」查看已拥有的灵兽",
            prefix
        );
    }

    let beast_count = db.read_user_data(user_id, "beast_count").parse::<i32>().unwrap_or(0);
    let active_beast = db.read_user_data(user_id, "active_beast");

    // 查找灵兽
    for i in 1..=beast_count {
        let beast_id = format!("beast_{}", i);
        let beast_data = db.read_user_data(user_id, &beast_id);

        if beast_data.is_empty() {
            continue;
        }

        let parts: Vec<&str> = beast_data.split(',').collect();
        if parts.is_empty() {
            continue;
        }

        let name = parts[0];
        if name.contains(target_name) || target_name.contains(name) {
            // 检查是否是出战灵兽
            if active_beast == beast_id {
                db.write_user_data(user_id, "active_beast", "");
            }

            // 清空灵兽数据
            db.write_user_data(user_id, &beast_id, "");

            let rarity = get_beast_rarity(parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1));
            let mut out = format!("{}\n", prefix);
            out += &format!("👋 已放生 {}[{}]！\n\n", rarity.emoji(), name);
            out += "💡 灵兽已回归自然\n";
            return out;
        }
    }

    format!(
        "{}\n❌ 没有找到名为「{}」的灵兽！\n💡 使用「我的灵兽」查看已拥有的灵兽",
        prefix, target_name
    )
}

/// 获取出战灵兽的属性加成 (供战斗系统调用)
pub fn get_active_beast_bonus(db: &Database, user_id: &str) -> Option<(String, i32, i32, i32, i32, String)> {
    let active_beast = db.read_user_data(user_id, "active_beast");
    if active_beast.is_empty() {
        return None;
    }

    let beast_data = db.read_user_data(user_id, &active_beast);
    if beast_data.is_empty() {
        return None;
    }

    let parts: Vec<&str> = beast_data.split(',').collect();
    if parts.len() < 9 {
        return None;
    }

    let name = parts[0].to_string();
    let _level: i32 = parts[2].parse().unwrap_or(1);
    let hp: i32 = parts[3].parse().unwrap_or(0);
    let ad: i32 = parts[4].parse().unwrap_or(0);
    let def: i32 = parts[5].parse().unwrap_or(0);
    let mdf: i32 = parts[6].parse().unwrap_or(0);
    let loyalty: i32 = parts[7].parse().unwrap_or(0);

    // 忠诚度影响加成比例
    let bonus_ratio = loyalty as f64 / 100.0;

    // 查找技能
    let skill = BEAST_TEMPLATES
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.skill.to_string())
        .unwrap_or_else(|| "普通攻击".to_string());

    Some((
        name,
        (hp as f64 * bonus_ratio) as i32,
        (ad as f64 * bonus_ratio) as i32,
        (def as f64 * bonus_ratio) as i32,
        (mdf as f64 * bonus_ratio) as i32,
        skill,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beast_rarity_names() {
        assert_eq!(BeastRarity::Common.name(), "普通");
        assert_eq!(BeastRarity::Uncommon.name(), "优秀");
        assert_eq!(BeastRarity::Rare.name(), "稀有");
        assert_eq!(BeastRarity::Epic.name(), "史诗");
        assert_eq!(BeastRarity::Legendary.name(), "传说");
    }

    #[test]
    fn test_beast_rarity_emojis() {
        assert_eq!(BeastRarity::Common.emoji(), "⚪");
        assert_eq!(BeastRarity::Uncommon.emoji(), "🟢");
        assert_eq!(BeastRarity::Rare.emoji(), "🔵");
        assert_eq!(BeastRarity::Epic.emoji(), "🟣");
        assert_eq!(BeastRarity::Legendary.emoji(), "🟡");
    }

    #[test]
    fn test_beast_rarity_capture_rates() {
        assert_eq!(BeastRarity::Common.capture_rate(), 0.80);
        assert_eq!(BeastRarity::Uncommon.capture_rate(), 0.50);
        assert_eq!(BeastRarity::Rare.capture_rate(), 0.30);
        assert_eq!(BeastRarity::Epic.capture_rate(), 0.15);
        assert_eq!(BeastRarity::Legendary.capture_rate(), 0.05);
    }

    #[test]
    fn test_capture_rate_decreasing() {
        // Higher rarity should have lower capture rate
        assert!(BeastRarity::Common.capture_rate() > BeastRarity::Uncommon.capture_rate());
        assert!(BeastRarity::Uncommon.capture_rate() > BeastRarity::Rare.capture_rate());
        assert!(BeastRarity::Rare.capture_rate() > BeastRarity::Epic.capture_rate());
        assert!(BeastRarity::Epic.capture_rate() > BeastRarity::Legendary.capture_rate());
    }

    #[test]
    fn test_from_monster_type() {
        assert!(matches!(
            BeastRarity::from_monster_type("Ordinary"),
            BeastRarity::Common
        ));
        assert!(matches!(BeastRarity::from_monster_type("Boss"), BeastRarity::Rare));
        assert!(matches!(BeastRarity::from_monster_type("Elite"), BeastRarity::Uncommon));
        assert!(matches!(
            BeastRarity::from_monster_type("Unknown"),
            BeastRarity::Uncommon
        ));
    }

    #[test]
    fn test_get_beast_rarity_by_level() {
        assert!(matches!(get_beast_rarity(1), BeastRarity::Common));
        assert!(matches!(get_beast_rarity(10), BeastRarity::Common));
        assert!(matches!(get_beast_rarity(15), BeastRarity::Uncommon));
        assert!(matches!(get_beast_rarity(25), BeastRarity::Rare));
        assert!(matches!(get_beast_rarity(40), BeastRarity::Epic));
        assert!(matches!(get_beast_rarity(99), BeastRarity::Epic));
    }
}

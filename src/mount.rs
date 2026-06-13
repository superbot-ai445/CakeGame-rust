/// CakeGame 坐骑系统
///
/// 玩家可获取、培养坐骑，骑乘后获得属性加成。
/// 支持：查看坐骑/坐骑列表/骑乘/下骑/喂养坐骑/坐骑排行
///
/// 数据存储: Global 表 SECTION='mount'
use crate::core::*;
use crate::db::Database;
use crate::user;

/// 坐骑品质
#[derive(Debug, Clone, Copy)]
enum MountRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl MountRarity {
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
}

/// 坐骑模板定义
struct MountTemplate {
    name: &'static str,
    species: &'static str,
    rarity: MountRarity,
    emoji: &'static str,
    /// 速度等级 (1-10)
    speed: i32,
    /// 每级HP加成
    hp_per_level: i32,
    /// 每级AD加成
    ad_per_level: i32,
    /// 每级防御加成
    def_per_level: i32,
    /// 每级魔抗加成
    mdf_per_level: i32,
    /// 最大等级
    max_level: i32,
    /// 获得方式描述
    obtain: &'static str,
    /// 可进化为
    evolves_to: &'static str,
    /// 进化所需等级
    evolve_level: i32,
    /// 进化所需喂养次数
    evolve_feed: i32,
}

const MOUNT_TEMPLATES: &[MountTemplate] = &[
    // 普通坐骑
    MountTemplate {
        name: "小毛驴",
        species: "驴",
        rarity: MountRarity::Common,
        emoji: "🫏",
        speed: 1,
        hp_per_level: 10,
        ad_per_level: 2,
        def_per_level: 1,
        mdf_per_level: 1,
        max_level: 20,
        obtain: "新手村任务奖励",
        evolves_to: "快驴",
        evolve_level: 10,
        evolve_feed: 20,
    },
    MountTemplate {
        name: "大棕马",
        species: "马",
        rarity: MountRarity::Common,
        emoji: "🐴",
        speed: 2,
        hp_per_level: 15,
        ad_per_level: 3,
        def_per_level: 2,
        mdf_per_level: 1,
        max_level: 25,
        obtain: "10级商店购买 (500金币)",
        evolves_to: "战马",
        evolve_level: 15,
        evolve_feed: 30,
    },
    MountTemplate {
        name: "快驴",
        species: "驴",
        rarity: MountRarity::Uncommon,
        emoji: "🫏",
        speed: 3,
        hp_per_level: 20,
        ad_per_level: 4,
        def_per_level: 2,
        mdf_per_level: 2,
        max_level: 30,
        obtain: "小毛驴进化",
        evolves_to: "疾风驴",
        evolve_level: 20,
        evolve_feed: 40,
    },
    // 优秀坐骑
    MountTemplate {
        name: "战马",
        species: "马",
        rarity: MountRarity::Uncommon,
        emoji: "🐴",
        speed: 3,
        hp_per_level: 25,
        ad_per_level: 5,
        def_per_level: 3,
        mdf_per_level: 2,
        max_level: 30,
        obtain: "大棕马进化",
        evolves_to: "烈焰战马",
        evolve_level: 20,
        evolve_feed: 40,
    },
    MountTemplate {
        name: "森林狼",
        species: "狼",
        rarity: MountRarity::Uncommon,
        emoji: "🐺",
        speed: 4,
        hp_per_level: 18,
        ad_per_level: 6,
        def_per_level: 2,
        mdf_per_level: 3,
        max_level: 30,
        obtain: "击败狼王BOSS掉落 (15%概率)",
        evolves_to: "银狼王",
        evolve_level: 20,
        evolve_feed: 50,
    },
    // 稀有坐骑
    MountTemplate {
        name: "疾风驴",
        species: "驴",
        rarity: MountRarity::Rare,
        emoji: "💨",
        speed: 5,
        hp_per_level: 30,
        ad_per_level: 6,
        def_per_level: 3,
        mdf_per_level: 3,
        max_level: 40,
        obtain: "快驴进化",
        evolves_to: "",
        evolve_level: 0,
        evolve_feed: 0,
    },
    MountTemplate {
        name: "烈焰战马",
        species: "马",
        rarity: MountRarity::Rare,
        emoji: "🔥",
        speed: 5,
        hp_per_level: 35,
        ad_per_level: 8,
        def_per_level: 4,
        mdf_per_level: 3,
        max_level: 40,
        obtain: "战马进化",
        evolves_to: "天马",
        evolve_level: 30,
        evolve_feed: 60,
    },
    MountTemplate {
        name: "银狼王",
        species: "狼",
        rarity: MountRarity::Rare,
        emoji: "🐺",
        speed: 5,
        hp_per_level: 25,
        ad_per_level: 10,
        def_per_level: 3,
        mdf_per_level: 4,
        max_level: 40,
        obtain: "森林狼进化",
        evolves_to: "暗影魔狼",
        evolve_level: 30,
        evolve_feed: 60,
    },
    MountTemplate {
        name: "铁甲犀牛",
        species: "犀牛",
        rarity: MountRarity::Rare,
        emoji: "🦏",
        speed: 3,
        hp_per_level: 50,
        ad_per_level: 5,
        def_per_level: 8,
        mdf_per_level: 5,
        max_level: 40,
        obtain: "公会试炼第5关奖励",
        evolves_to: "钢甲犀牛王",
        evolve_level: 30,
        evolve_feed: 60,
    },
    // 史诗坐骑
    MountTemplate {
        name: "天马",
        species: "马",
        rarity: MountRarity::Epic,
        emoji: "🦄",
        speed: 7,
        hp_per_level: 45,
        ad_per_level: 10,
        def_per_level: 5,
        mdf_per_level: 5,
        max_level: 50,
        obtain: "烈焰战马进化",
        evolves_to: "圣光天马",
        evolve_level: 40,
        evolve_feed: 80,
    },
    MountTemplate {
        name: "暗影魔狼",
        species: "狼",
        rarity: MountRarity::Epic,
        emoji: "🌑",
        speed: 7,
        hp_per_level: 35,
        ad_per_level: 14,
        def_per_level: 4,
        mdf_per_level: 6,
        max_level: 50,
        obtain: "银狼王进化",
        evolves_to: "幽冥狼神",
        evolve_level: 40,
        evolve_feed: 80,
    },
    MountTemplate {
        name: "钢甲犀牛王",
        species: "犀牛",
        rarity: MountRarity::Epic,
        emoji: "🦏",
        speed: 4,
        hp_per_level: 70,
        ad_per_level: 7,
        def_per_level: 12,
        mdf_per_level: 7,
        max_level: 50,
        obtain: "铁甲犀牛进化",
        evolves_to: "",
        evolve_level: 0,
        evolve_feed: 0,
    },
    MountTemplate {
        name: "冰霜巨鹰",
        species: "鹰",
        rarity: MountRarity::Epic,
        emoji: "🦅",
        speed: 8,
        hp_per_level: 30,
        ad_per_level: 12,
        def_per_level: 4,
        mdf_per_level: 8,
        max_level: 50,
        obtain: "深渊50层通关奖励",
        evolves_to: "风暴巨鹰",
        evolve_level: 40,
        evolve_feed: 80,
    },
    // 传说坐骑
    MountTemplate {
        name: "圣光天马",
        species: "马",
        rarity: MountRarity::Legendary,
        emoji: "✨",
        speed: 9,
        hp_per_level: 60,
        ad_per_level: 14,
        def_per_level: 7,
        mdf_per_level: 7,
        max_level: 60,
        obtain: "天马进化",
        evolves_to: "",
        evolve_level: 0,
        evolve_feed: 0,
    },
    MountTemplate {
        name: "幽冥狼神",
        species: "狼",
        rarity: MountRarity::Legendary,
        emoji: "👻",
        speed: 9,
        hp_per_level: 45,
        ad_per_level: 20,
        def_per_level: 6,
        mdf_per_level: 8,
        max_level: 60,
        obtain: "暗影魔狼进化",
        evolves_to: "",
        evolve_level: 0,
        evolve_feed: 0,
    },
    MountTemplate {
        name: "风暴巨鹰",
        species: "鹰",
        rarity: MountRarity::Legendary,
        emoji: "🌪️",
        speed: 10,
        hp_per_level: 40,
        ad_per_level: 16,
        def_per_level: 5,
        mdf_per_level: 10,
        max_level: 60,
        obtain: "冰霜巨鹰进化",
        evolves_to: "",
        evolve_level: 0,
        evolve_feed: 0,
    },
    MountTemplate {
        name: "远古巨龙",
        species: "龙",
        rarity: MountRarity::Legendary,
        emoji: "🐉",
        speed: 10,
        hp_per_level: 80,
        ad_per_level: 18,
        def_per_level: 10,
        mdf_per_level: 10,
        max_level: 60,
        obtain: "全服BOSS击杀奖励 (0.5%概率)",
        evolves_to: "",
        evolve_level: 0,
        evolve_feed: 0,
    },
];

/// 喂养消耗食物定义
struct FeedItem {
    name: &'static str,
    exp_gain: i32,
    emoji: &'static str,
}

const FEED_ITEMS: &[FeedItem] = &[
    FeedItem {
        name: "胡萝卜",
        exp_gain: 5,
        emoji: "🥕",
    },
    FeedItem {
        name: "苹果",
        exp_gain: 8,
        emoji: "🍎",
    },
    FeedItem {
        name: "精制饲料",
        exp_gain: 15,
        emoji: "🌾",
    },
    FeedItem {
        name: "灵果",
        exp_gain: 30,
        emoji: "🍇",
    },
    FeedItem {
        name: "龙涎草",
        exp_gain: 50,
        emoji: "🌿",
    },
];

const SECTION: &str = "mount";

/// 查找坐骑模板 (模糊匹配)
fn find_template(name: &str) -> Option<&'static MountTemplate> {
    // 精确匹配
    if let Some(t) = MOUNT_TEMPLATES.iter().find(|t| t.name == name) {
        return Some(t);
    }
    // 模糊匹配
    let matches: Vec<&MountTemplate> = MOUNT_TEMPLATES
        .iter()
        .filter(|t| t.name.contains(name) || name.contains(t.name))
        .collect();
    if matches.len() == 1 {
        return Some(matches[0]);
    }
    None
}

/// 获取用户坐骑数据
fn get_user_mount(db: &Database, user_id: &str) -> Option<(String, i32, i32, String)> {
    let raw = db.global_get(SECTION, &format!("mount_{}", user_id));
    if raw.is_empty() {
        return None;
    }
    // 格式: name|level|feed_count|ride_status
    let parts: Vec<&str> = raw.split('|').collect();
    if parts.len() >= 3 {
        let name = parts[0].to_string();
        let level: i32 = parts[1].parse().unwrap_or(1);
        let feed_count: i32 = parts[2].parse().unwrap_or(0);
        let ride = if parts.len() >= 4 {
            parts[3].to_string()
        } else {
            "off".to_string()
        };
        Some((name, level, feed_count, ride))
    } else {
        None
    }
}

/// 保存用户坐骑数据
fn save_user_mount(db: &Database, user_id: &str, name: &str, level: i32, feed_count: i32, ride: &str) {
    db.global_set(
        SECTION,
        &format!("mount_{}", user_id),
        &format!("{}|{}|{}|{}", name, level, feed_count, ride),
    );
}

/// 计算坐骑属性加成
fn calc_mount_bonus(template: &MountTemplate, level: i32) -> (i32, i32, i32, i32) {
    (
        template.hp_per_level * level,
        template.ad_per_level * level,
        template.def_per_level * level,
        template.mdf_per_level * level,
    )
}

/// 喂养经验需求公式：level * 10
fn feed_exp_required(level: i32) -> i32 {
    level * 10
}

/// 进度条
fn progress_bar(current: i32, max: i32, width: usize) -> String {
    if max <= 0 {
        return "░".repeat(width);
    }
    let filled = ((current as f64 / max as f64) * width as f64).min(width as f64) as usize;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// 查看坐骑 — 查看当前坐骑状态
pub fn cmd_view_mount(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    match get_user_mount(db, user_id) {
        None => format!(
            "{}\n═══ 🐴 坐骑系统 ═══\n\n您还没有坐骑！\n\n💡 获取方式：\n  • 新手10级后自动获得「小毛驴」\n  • 击败特定BOSS掉落\n  • 公会试炼奖励\n  • 深渊通关奖励\n\n使用「坐骑列表」查看所有坐骑",
            prefix
        ),
        Some((name, level, feed_count, ride)) => {
            if let Some(template) = find_template(&name) {
                let (hp, ad, def, mdf) = calc_mount_bonus(template, level);
                let ride_status = if ride == "on" { "✅ 骑乘中" } else { "❌ 未骑乘" };
                let feed_needed = feed_exp_required(level);
                let feed_bar = progress_bar(feed_count % feed_needed, feed_needed, 10);
                let mut out = format!(
                    "{}\n═══ {} {} ═══\n\n\
                    📊 品质：{} {}\n\
                    🐾 种族：{}\n\
                    📈 等级：{}/{}\n\
                    🏃 速度：{}\n\
                    🍖 喂养：{} {}/{}\n\
                    🎯 状态：{}\n\n\
                    ═══ 属性加成 ═══\n\
                    ❤️ HP +{}  ⚔️ AD +{}  🛡️ DEF +{}  🔮 MR +{}\n",
                    prefix,
                    template.emoji,
                    name,
                    template.rarity.emoji(),
                    template.rarity.name(),
                    template.species,
                    level,
                    template.max_level,
                    template.speed,
                    feed_bar,
                    feed_count % feed_needed,
                    feed_needed,
                    ride_status,
                    hp, ad, def, mdf,
                );
                // 进化信息
                if !template.evolves_to.is_empty() {
                    out.push_str(&format!(
                        "\n🔄 进化：{} → {} (需等级{}+喂养{}次)\n",
                        name, template.evolves_to, template.evolve_level, template.evolve_feed,
                    ));
                }
                out.push_str("\n💡 指令：骑乘/下骑/喂养坐骑+食物名/坐骑列表\n");
                out
            } else {
                format!("{}\n❌ 坐骑数据异常：{}", prefix, name)
            }
        }
    }
}

/// 坐骑列表 — 查看所有可获得的坐骑
pub fn cmd_mount_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current_mount = get_user_mount(db, user_id).map(|(n, _, _, _)| n).unwrap_or_default();

    let mut out = format!("{}\n═══ 🐴 坐骑图鉴 ═══\n", prefix);

    let by_rarity: Vec<(usize, &str)> = vec![
        (0, "⚪ 普通"),
        (1, "🟢 优秀"),
        (2, "🔵 稀有"),
        (3, "🟣 史诗"),
        (4, "🟡 传说"),
    ];

    for (rarity_idx, rarity_label) in &by_rarity {
        out.push_str(&format!("\n{}\n", rarity_label));
        for t in MOUNT_TEMPLATES.iter() {
            let ri = match t.rarity {
                MountRarity::Common => 0,
                MountRarity::Uncommon => 1,
                MountRarity::Rare => 2,
                MountRarity::Epic => 3,
                MountRarity::Legendary => 4,
            };
            if ri == *rarity_idx {
                let owned = if current_mount == t.name { " ✅" } else { "" };
                let (hp, ad, def, mdf) = calc_mount_bonus(t, 1);
                out.push_str(&format!(
                    "  {} {} (速{}/HP+{}/AD+{}/DEF+{}/MR+{}) — {}{}\n",
                    t.emoji, t.name, t.speed, hp, ad, def, mdf, t.obtain, owned,
                ));
            }
        }
    }

    out.push_str(&format!(
        "\n共 {} 种坐骑\n💡 使用「查看坐骑」查看当前坐骑状态",
        MOUNT_TEMPLATES.len()
    ));
    out
}

/// 骑乘 — 装备当前坐骑
pub fn cmd_mount_ride(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    match get_user_mount(db, user_id) {
        None => format!("{}\n❌ 您还没有坐骑！先通过任务或BOSS掉落获取坐骑吧。", prefix),
        Some((name, level, feed_count, ride)) => {
            if ride == "on" {
                return format!("{}\n❌ 您已经骑乘着「{}」了！", prefix, name);
            }
            save_user_mount(db, user_id, &name, level, feed_count, "on");
            if let Some(template) = find_template(&name) {
                let (hp, ad, def, mdf) = calc_mount_bonus(template, level);
                format!(
                    "{}\n═══ 🐴 骑乘成功 ═══\n\n\
                    {} {} 已装备！\n\n\
                    ⚡ 属性加成已生效：\n\
                    ❤️ HP +{}  ⚔️ AD +{}  🛡️ DEF +{}  🔮 MR +{}\n\n\
                    💡 使用「下骑」取消骑乘状态",
                    prefix, template.emoji, name, hp, ad, def, mdf,
                )
            } else {
                format!("{}\n✅ 骑乘「{}」成功！", prefix, name)
            }
        }
    }
}

/// 下骑 — 取消骑乘状态
pub fn cmd_mount_dismount(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    match get_user_mount(db, user_id) {
        None => format!("{}\n❌ 您还没有坐骑！", prefix),
        Some((name, level, feed_count, ride)) => {
            if ride != "on" {
                return format!("{}\n❌ 您当前没有骑乘坐骑。", prefix);
            }
            save_user_mount(db, user_id, &name, level, feed_count, "off");
            format!(
                "{}\n✅ 已下骑「{}」，属性加成已取消。\n💡 使用「骑乘」重新装备坐骑。",
                prefix, name,
            )
        }
    }
}

/// 喂养坐骑 — 使用食物提升坐骑经验
pub fn cmd_feed_mount(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let food_name = args.trim();
    if food_name.is_empty() {
        let mut out = format!("{}\n═══ 🍖 喂养食物列表 ═══\n\n", prefix);
        for f in FEED_ITEMS {
            out.push_str(&format!("  {} {} (经验+{})\n", f.emoji, f.name, f.exp_gain));
        }
        out.push_str("\n💡 用法：喂养坐骑+食物名");
        return out;
    }

    // 查找食物
    let food = FEED_ITEMS
        .iter()
        .find(|f| f.name == food_name || f.name.contains(food_name));
    let food = match food {
        Some(f) => f,
        None => {
            return format!(
                "{}\n❌ 未找到食物「{}」！\n💡 使用「喂养坐骑」查看可用食物列表。",
                prefix, food_name,
            );
        }
    };

    match get_user_mount(db, user_id) {
        None => format!("{}\n❌ 您还没有坐骑！", prefix),
        Some((name, level, feed_count, ride)) => {
            let template = match find_template(&name) {
                Some(t) => t,
                None => return format!("{}\n❌ 坐骑数据异常", prefix),
            };

            if level >= template.max_level {
                return format!("{}\n❌ 「{}」已满级({})！无法继续喂养。", prefix, name, level);
            }

            let new_feed = feed_count + food.exp_gain;

            // 检查是否升级
            let mut new_level = level;
            let mut remaining = new_feed;
            while remaining >= feed_exp_required(new_level) && new_level < template.max_level {
                remaining -= feed_exp_required(new_level);
                new_level += 1;
            }

            save_user_mount(db, user_id, &name, new_level, remaining, &ride);

            if new_level > level {
                let (hp, ad, def, mdf) = calc_mount_bonus(template, new_level);
                let mut out = format!(
                    "{}\n═══ 🎉 坐骑升级！═══\n\n\
                    {} {} Lv.{} → Lv.{}\n\n\
                    ⚡ 新属性加成：\n\
                    ❤️ HP +{}  ⚔️ AD +{}  🛡️ DEF +{}  🔮 MR +{}\n",
                    prefix, template.emoji, name, level, new_level, hp, ad, def, mdf,
                );
                // 检查是否可以进化
                if !template.evolves_to.is_empty()
                    && new_level >= template.evolve_level
                    && new_feed >= template.evolve_feed
                {
                    out.push_str(&format!(
                        "\n🔄 「{}」已满足进化条件！使用「坐骑进化」进化为「{}」\n",
                        name, template.evolves_to,
                    ));
                }
                out
            } else {
                let feed_bar = progress_bar(remaining, feed_exp_required(new_level), 10);
                format!(
                    "{}\n✅ 喂养 {} {} 成功！{} 经验+{}\n\n\
                    {} {} 经验 {} {}/{}\n\
                    💡 继续喂养可提升等级",
                    prefix,
                    food.emoji,
                    food.name,
                    name,
                    food.exp_gain,
                    template.emoji,
                    name,
                    feed_bar,
                    remaining,
                    feed_exp_required(new_level),
                )
            }
        }
    }
}

/// 坐骑进化 — 进化当前坐骑到更高品质
pub fn cmd_mount_evolve(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    match get_user_mount(db, user_id) {
        None => format!("{}\n❌ 您还没有坐骑！", prefix),
        Some((name, level, feed_count, ride)) => {
            let template = match find_template(&name) {
                Some(t) => t,
                None => return format!("{}\n❌ 坐骑数据异常", prefix),
            };

            if template.evolves_to.is_empty() {
                return format!("{}\n❌ 「{}」已经是最高品质，无法进化！", prefix, name);
            }
            if level < template.evolve_level {
                return format!(
                    "{}\n❌ 进化条件不足！\n\n\
                    需要等级：{} (当前：{})\n\
                    需要喂养：{}次 (当前：{}次)\n",
                    prefix, template.evolve_level, level, template.evolve_feed, feed_count,
                );
            }
            if feed_count < template.evolve_feed {
                return format!(
                    "{}\n❌ 喂养次数不足！\n\n\
                    需要喂养：{}次 (当前：{}次)\n\
                    还需 {} 次喂养\n",
                    prefix,
                    template.evolve_feed,
                    feed_count,
                    template.evolve_feed - feed_count,
                );
            }

            // 查找进化后的模板
            let new_template = match MOUNT_TEMPLATES.iter().find(|t| t.name == template.evolves_to) {
                Some(t) => t,
                None => return format!("{}\n❌ 进化目标坐骑数据异常", prefix),
            };

            // 执行进化
            save_user_mount(db, user_id, new_template.name, 1, 0, &ride);

            let (hp, ad, def, mdf) = calc_mount_bonus(new_template, 1);
            format!(
                "{}\n═══ 🎊 坐骑进化！═══\n\n\
                {} {} → {} {}\n\n\
                📊 新品质：{} {}\n\
                🏃 新速度：{}\n\
                ⚡ Lv.1属性加成：\n\
                ❤️ HP +{}  ⚔️ AD +{}  🛡️ DEF +{}  🔮 MR +{}\n\n\
                💡 继续喂养提升新坐骑等级！",
                prefix,
                template.emoji,
                name,
                new_template.emoji,
                new_template.name,
                new_template.rarity.emoji(),
                new_template.rarity.name(),
                new_template.speed,
                hp,
                ad,
                def,
                mdf,
            )
        }
    }
}

/// 坐骑排行 — 全服坐骑等级排名
pub fn cmd_mount_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let conn = db.lock_conn();

    // 获取所有坐骑数据
    let mut entries: Vec<(String, String, i32, i32)> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT ID, DATA FROM Global WHERE SECTION = '{}' AND ID LIKE 'mount_%'",
        SECTION
    )) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        }) {
            for row in rows.flatten() {
                let (id, data) = row;
                if data.is_empty() {
                    continue;
                }
                let uid = id.strip_prefix("mount_").unwrap_or(&id).to_string();
                let parts: Vec<&str> = data.split('|').collect();
                if parts.len() >= 3 {
                    let mount_name = parts[0].to_string();
                    let level: i32 = parts[1].parse().unwrap_or(0);
                    let feed: i32 = parts[2].parse().unwrap_or(0);
                    // 评分 = 级别 * 100 + 喂养次数
                    let score = level * 100 + feed;
                    entries.push((uid, mount_name, level, score));
                }
            }
        }
    }
    drop(conn);

    if entries.is_empty() {
        return format!("{}\n📊 暂无坐骑排行数据。", prefix);
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.3));
    entries.truncate(15);

    let mut out = format!("{}\n═══ 🐴 坐骑排行榜 ═══\n\n", prefix);
    let medals = ["🥇", "🥈", "🥉"];

    for (i, (uid, mount_name, level, _score)) in entries.iter().enumerate() {
        let medal = if i < 3 { medals[i] } else { "" };
        let rank_str = if i < 3 {
            medal.to_string()
        } else {
            format!("#{}", i + 1)
        };

        let template = find_template(mount_name);
        let emoji = template.map(|t| t.emoji).unwrap_or("🐴");
        let nickname = db.read_basic(uid, ITEM_NAME);
        let display = if nickname.is_empty() { uid.clone() } else { nickname };

        let marker = if uid == user_id { " ◀ YOU" } else { "" };
        out.push_str(&format!(
            "  {} {} {} Lv.{} ({}){}\n",
            rank_str, emoji, display, level, mount_name, marker,
        ));
    }

    // 当前用户排名
    if let Some(pos) = entries.iter().position(|(uid, _, _, _)| uid == user_id) {
        out.push_str(&format!("\n📍 您的排名：#{}", pos + 1));
    } else {
        out.push_str("\n💡 您还未上榜，快去获取坐骑吧！");
    }

    out
}

/// 获取骑乘中的坐骑属性加成 (供战斗系统集成)
#[allow(dead_code)]
pub fn get_mount_combat_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32) {
    match get_user_mount(db, user_id) {
        Some((name, level, _, ride)) if ride == "on" => {
            if let Some(template) = find_template(&name) {
                return calc_mount_bonus(template, level);
            }
            (0, 0, 0, 0)
        }
        _ => (0, 0, 0, 0),
    }
}

/// 获取坐骑速度加成 (供移动系统集成)
#[allow(dead_code)]
pub fn get_mount_speed(db: &Database, user_id: &str) -> i32 {
    match get_user_mount(db, user_id) {
        Some((name, _, _, ride)) if ride == "on" => {
            if let Some(template) = find_template(&name) {
                return template.speed;
            }
            0
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_template_count() {
        assert_eq!(MOUNT_TEMPLATES.len(), 17);
    }

    #[test]
    fn test_mount_names_unique() {
        let mut names: Vec<&str> = MOUNT_TEMPLATES.iter().map(|t| t.name).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before, "Mount names should be unique");
    }

    #[test]
    fn test_rarity_emoji() {
        assert_eq!(MountRarity::Common.emoji(), "⚪");
        assert_eq!(MountRarity::Uncommon.emoji(), "🟢");
        assert_eq!(MountRarity::Rare.emoji(), "🔵");
        assert_eq!(MountRarity::Epic.emoji(), "🟣");
        assert_eq!(MountRarity::Legendary.emoji(), "🟡");
    }

    #[test]
    fn test_rarity_name() {
        assert_eq!(MountRarity::Common.name(), "普通");
        assert_eq!(MountRarity::Legendary.name(), "传说");
    }

    #[test]
    fn test_calc_mount_bonus() {
        let template = &MOUNT_TEMPLATES[0]; // 小毛驴
        let (hp, ad, def, mdf) = calc_mount_bonus(template, 10);
        assert_eq!(hp, 100); // 10 * 10
        assert_eq!(ad, 20); // 2 * 10
        assert_eq!(def, 10); // 1 * 10
        assert_eq!(mdf, 10); // 1 * 10
    }

    #[test]
    fn test_calc_mount_bonus_zero_level() {
        let template = &MOUNT_TEMPLATES[0];
        let (hp, ad, def, mdf) = calc_mount_bonus(template, 0);
        assert_eq!(hp, 0);
        assert_eq!(ad, 0);
        assert_eq!(def, 0);
        assert_eq!(mdf, 0);
    }

    #[test]
    fn test_feed_exp_required() {
        assert_eq!(feed_exp_required(1), 10);
        assert_eq!(feed_exp_required(5), 50);
        assert_eq!(feed_exp_required(10), 100);
    }

    #[test]
    fn test_find_template_exact() {
        let t = find_template("小毛驴").unwrap();
        assert_eq!(t.name, "小毛驴");
    }

    #[test]
    fn test_find_template_fuzzy() {
        let t = find_template("巨龙");
        assert!(t.is_some());
        assert_eq!(t.unwrap().name, "远古巨龙");
    }

    #[test]
    fn test_find_template_not_found() {
        assert!(find_template("不存在的坐骑").is_none());
    }

    #[test]
    fn test_progress_bar() {
        let bar = progress_bar(5, 10, 10);
        assert_eq!(bar, "█████░░░░░");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(10, 10, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 10, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn test_progress_bar_zero_max() {
        let bar = progress_bar(5, 0, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn test_evolution_chain() {
        // 验证进化链完整性
        for t in MOUNT_TEMPLATES {
            if !t.evolves_to.is_empty() {
                let target = MOUNT_TEMPLATES.iter().find(|x| x.name == t.evolves_to);
                assert!(
                    target.is_some(),
                    "Evolution target '{}' for '{}' not found",
                    t.evolves_to,
                    t.name
                );
            }
        }
    }

    #[test]
    fn test_speed_range() {
        for t in MOUNT_TEMPLATES {
            assert!(t.speed >= 1 && t.speed <= 10, "Speed out of range for {}", t.name);
        }
    }

    #[test]
    fn test_max_level_range() {
        for t in MOUNT_TEMPLATES {
            assert!(
                t.max_level >= 20 && t.max_level <= 60,
                "Max level out of range for {}",
                t.name
            );
        }
    }

    #[test]
    fn test_feed_items_non_empty() {
        assert!(!FEED_ITEMS.is_empty());
        for f in FEED_ITEMS {
            assert!(f.exp_gain > 0, "Feed item {} has non-positive exp", f.name);
        }
    }

    #[test]
    fn test_feed_items_names_unique() {
        let mut names: Vec<&str> = FEED_ITEMS.iter().map(|f| f.name).collect();
        let len_before = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), len_before);
    }

    #[test]
    fn test_rarity_coverage() {
        let mut rarities = std::collections::HashSet::new();
        for t in MOUNT_TEMPLATES {
            rarities.insert(t.rarity.name());
        }
        // Should have all 5 rarities
        assert_eq!(rarities.len(), 5, "Not all rarities covered");
    }
}

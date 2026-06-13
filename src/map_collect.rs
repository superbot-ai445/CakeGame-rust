/// CakeGame 地图资源采集扩展系统
/// 基于 MessageTemplate com.shdic.cg_collection
/// 显示当前地图可采集的草药/矿石，支持分页和直接采集
use crate::core::*;
use crate::db::Database;

/// 地图资源定义
#[allow(dead_code)]
struct MapResource {
    name: &'static str,
    /// 资源类型: herb(草药) / ore(矿石)
    res_type: &'static str,
    /// 采集消耗金币
    cost: i32,
    /// 基础概率 (百分比)
    prob: i32,
    /// 所需采集技能等级
    min_level: i32,
    /// 稀有度标签
    rarity: &'static str,
}

/// 每个地图的资源列表
struct MapResourceSet {
    map_name: &'static str,
    herbs: &'static [MapResource],
    ores: &'static [MapResource],
}

const MAP_RESOURCES: &[MapResourceSet] = &[
    MapResourceSet {
        map_name: "格兰森林",
        herbs: &[
            MapResource {
                name: "草药",
                res_type: "herb",
                cost: 5,
                prob: 80,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "灵芝",
                res_type: "herb",
                cost: 15,
                prob: 50,
                min_level: 1,
                rarity: "精良",
            },
            MapResource {
                name: "仙草",
                res_type: "herb",
                cost: 30,
                prob: 25,
                min_level: 3,
                rarity: "稀有",
            },
            MapResource {
                name: "森林蘑菇",
                res_type: "herb",
                cost: 10,
                prob: 60,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "青苔",
                res_type: "herb",
                cost: 8,
                prob: 70,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "格兰花蜜",
                res_type: "herb",
                cost: 20,
                prob: 35,
                min_level: 2,
                rarity: "精良",
            },
        ],
        ores: &[
            MapResource {
                name: "铜矿石",
                res_type: "ore",
                cost: 10,
                prob: 75,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "铁矿石",
                res_type: "ore",
                cost: 20,
                prob: 45,
                min_level: 1,
                rarity: "精良",
            },
            MapResource {
                name: "锡矿石",
                res_type: "ore",
                cost: 15,
                prob: 55,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "翡翠石",
                res_type: "ore",
                cost: 35,
                prob: 20,
                min_level: 3,
                rarity: "稀有",
            },
        ],
    },
    MapResourceSet {
        map_name: "磐石之地",
        herbs: &[
            MapResource {
                name: "岩石花",
                res_type: "herb",
                cost: 12,
                prob: 65,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "石中草",
                res_type: "herb",
                cost: 25,
                prob: 40,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "磐石灵芝",
                res_type: "herb",
                cost: 40,
                prob: 20,
                min_level: 4,
                rarity: "稀有",
            },
            MapResource {
                name: "地衣",
                res_type: "herb",
                cost: 8,
                prob: 70,
                min_level: 0,
                rarity: "普通",
            },
        ],
        ores: &[
            MapResource {
                name: "铁矿石",
                res_type: "ore",
                cost: 15,
                prob: 60,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "银矿石",
                res_type: "ore",
                cost: 30,
                prob: 35,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "玄铁矿",
                res_type: "ore",
                cost: 50,
                prob: 15,
                min_level: 4,
                rarity: "稀有",
            },
            MapResource {
                name: "花岗岩",
                res_type: "ore",
                cost: 10,
                prob: 70,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "金刚石",
                res_type: "ore",
                cost: 80,
                prob: 8,
                min_level: 6,
                rarity: "史诗",
            },
        ],
    },
    MapResourceSet {
        map_name: "沼泽之地",
        herbs: &[
            MapResource {
                name: "沼泽草",
                res_type: "herb",
                cost: 8,
                prob: 75,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "毒蘑菇",
                res_type: "herb",
                cost: 15,
                prob: 50,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "水仙花",
                res_type: "herb",
                cost: 20,
                prob: 40,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "沼泽灵芝",
                res_type: "herb",
                cost: 45,
                prob: 18,
                min_level: 4,
                rarity: "稀有",
            },
            MapResource {
                name: "腐朽之根",
                res_type: "herb",
                cost: 12,
                prob: 60,
                min_level: 1,
                rarity: "普通",
            },
        ],
        ores: &[
            MapResource {
                name: "泥铁矿",
                res_type: "ore",
                cost: 12,
                prob: 60,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "沼泽水晶",
                res_type: "ore",
                cost: 35,
                prob: 25,
                min_level: 3,
                rarity: "精良",
            },
            MapResource {
                name: "暗影石",
                res_type: "ore",
                cost: 55,
                prob: 12,
                min_level: 5,
                rarity: "稀有",
            },
        ],
    },
    MapResourceSet {
        map_name: "艾尔村庄",
        herbs: &[
            MapResource {
                name: "野花",
                res_type: "herb",
                cost: 3,
                prob: 85,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "薰衣草",
                res_type: "herb",
                cost: 10,
                prob: 60,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "治愈草",
                res_type: "herb",
                cost: 18,
                prob: 45,
                min_level: 1,
                rarity: "精良",
            },
            MapResource {
                name: "月光花",
                res_type: "herb",
                cost: 30,
                prob: 25,
                min_level: 3,
                rarity: "稀有",
            },
        ],
        ores: &[
            MapResource {
                name: "铜矿石",
                res_type: "ore",
                cost: 8,
                prob: 70,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "锡矿石",
                res_type: "ore",
                cost: 12,
                prob: 55,
                min_level: 0,
                rarity: "普通",
            },
        ],
    },
    MapResourceSet {
        map_name: "菲尔沙漠",
        herbs: &[
            MapResource {
                name: "沙漠仙人掌",
                res_type: "herb",
                cost: 15,
                prob: 55,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "沙棘果",
                res_type: "herb",
                cost: 20,
                prob: 40,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "沙漠玫瑰",
                res_type: "herb",
                cost: 40,
                prob: 18,
                min_level: 4,
                rarity: "稀有",
            },
            MapResource {
                name: "龙血树汁",
                res_type: "herb",
                cost: 80,
                prob: 5,
                min_level: 7,
                rarity: "史诗",
            },
        ],
        ores: &[
            MapResource {
                name: "沙金矿",
                res_type: "ore",
                cost: 25,
                prob: 35,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "红宝石",
                res_type: "ore",
                cost: 60,
                prob: 12,
                min_level: 5,
                rarity: "稀有",
            },
            MapResource {
                name: "沙漠琉璃",
                res_type: "ore",
                cost: 45,
                prob: 20,
                min_level: 3,
                rarity: "精良",
            },
        ],
    },
    MapResourceSet {
        map_name: "荒野之地",
        herbs: &[
            MapResource {
                name: "野草",
                res_type: "herb",
                cost: 5,
                prob: 80,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "荒野薄荷",
                res_type: "herb",
                cost: 15,
                prob: 50,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "狼牙草",
                res_type: "herb",
                cost: 25,
                prob: 30,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "荒野之花",
                res_type: "herb",
                cost: 40,
                prob: 15,
                min_level: 4,
                rarity: "稀有",
            },
        ],
        ores: &[
            MapResource {
                name: "铁矿石",
                res_type: "ore",
                cost: 15,
                prob: 55,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "黑曜石",
                res_type: "ore",
                cost: 40,
                prob: 20,
                min_level: 3,
                rarity: "精良",
            },
            MapResource {
                name: "陨铁",
                res_type: "ore",
                cost: 70,
                prob: 8,
                min_level: 6,
                rarity: "史诗",
            },
        ],
    },
    MapResourceSet {
        map_name: "莱茵高原",
        herbs: &[
            MapResource {
                name: "高原雪莲",
                res_type: "herb",
                cost: 25,
                prob: 35,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "风铃草",
                res_type: "herb",
                cost: 12,
                prob: 60,
                min_level: 0,
                rarity: "普通",
            },
            MapResource {
                name: "灵芝",
                res_type: "herb",
                cost: 30,
                prob: 30,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "千年参",
                res_type: "herb",
                cost: 80,
                prob: 5,
                min_level: 7,
                rarity: "史诗",
            },
        ],
        ores: &[
            MapResource {
                name: "高原铁矿",
                res_type: "ore",
                cost: 20,
                prob: 50,
                min_level: 1,
                rarity: "普通",
            },
            MapResource {
                name: "白玉石",
                res_type: "ore",
                cost: 45,
                prob: 20,
                min_level: 3,
                rarity: "精良",
            },
            MapResource {
                name: "寒冰矿",
                res_type: "ore",
                cost: 55,
                prob: 12,
                min_level: 5,
                rarity: "稀有",
            },
        ],
    },
    MapResourceSet {
        map_name: "暗黑城堡",
        herbs: &[
            MapResource {
                name: "暗影草",
                res_type: "herb",
                cost: 30,
                prob: 35,
                min_level: 3,
                rarity: "精良",
            },
            MapResource {
                name: "幽灵花",
                res_type: "herb",
                cost: 50,
                prob: 18,
                min_level: 5,
                rarity: "稀有",
            },
            MapResource {
                name: "亡者之泪",
                res_type: "herb",
                cost: 100,
                prob: 5,
                min_level: 8,
                rarity: "传说",
            },
        ],
        ores: &[
            MapResource {
                name: "暗铁矿",
                res_type: "ore",
                cost: 35,
                prob: 30,
                min_level: 3,
                rarity: "精良",
            },
            MapResource {
                name: "灵魂石",
                res_type: "ore",
                cost: 70,
                prob: 10,
                min_level: 6,
                rarity: "稀有",
            },
            MapResource {
                name: "地狱火石",
                res_type: "ore",
                cost: 100,
                prob: 5,
                min_level: 8,
                rarity: "传说",
            },
        ],
    },
    MapResourceSet {
        map_name: "维度深渊",
        herbs: &[
            MapResource {
                name: "虚空之花",
                res_type: "herb",
                cost: 60,
                prob: 15,
                min_level: 6,
                rarity: "稀有",
            },
            MapResource {
                name: "深渊灵芝",
                res_type: "herb",
                cost: 80,
                prob: 8,
                min_level: 7,
                rarity: "史诗",
            },
            MapResource {
                name: "永恒之种",
                res_type: "herb",
                cost: 150,
                prob: 3,
                min_level: 9,
                rarity: "传说",
            },
        ],
        ores: &[
            MapResource {
                name: "虚空水晶",
                res_type: "ore",
                cost: 70,
                prob: 12,
                min_level: 6,
                rarity: "稀有",
            },
            MapResource {
                name: "维度矿石",
                res_type: "ore",
                cost: 100,
                prob: 5,
                min_level: 8,
                rarity: "史诗",
            },
            MapResource {
                name: "混沌之核",
                res_type: "ore",
                cost: 200,
                prob: 2,
                min_level: 10,
                rarity: "传说",
            },
        ],
    },
    MapResourceSet {
        map_name: "圣光城",
        herbs: &[
            MapResource {
                name: "圣光百合",
                res_type: "herb",
                cost: 20,
                prob: 45,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "天使之泪",
                res_type: "herb",
                cost: 50,
                prob: 15,
                min_level: 5,
                rarity: "稀有",
            },
            MapResource {
                name: "神圣草",
                res_type: "herb",
                cost: 10,
                prob: 65,
                min_level: 0,
                rarity: "普通",
            },
        ],
        ores: &[
            MapResource {
                name: "圣银矿",
                res_type: "ore",
                cost: 25,
                prob: 40,
                min_level: 2,
                rarity: "精良",
            },
            MapResource {
                name: "神圣水晶",
                res_type: "ore",
                cost: 55,
                prob: 15,
                min_level: 5,
                rarity: "稀有",
            },
            MapResource {
                name: "天使之羽矿",
                res_type: "ore",
                cost: 90,
                prob: 5,
                min_level: 7,
                rarity: "史诗",
            },
        ],
    },
];

/// 通用默认资源（不在以上列表中的地图）
const DEFAULT_HERBS: &[MapResource] = &[
    MapResource {
        name: "草药",
        res_type: "herb",
        cost: 5,
        prob: 80,
        min_level: 0,
        rarity: "普通",
    },
    MapResource {
        name: "野花",
        res_type: "herb",
        cost: 8,
        prob: 65,
        min_level: 0,
        rarity: "普通",
    },
    MapResource {
        name: "灵芝",
        res_type: "herb",
        cost: 20,
        prob: 35,
        min_level: 2,
        rarity: "精良",
    },
];

const DEFAULT_ORES: &[MapResource] = &[
    MapResource {
        name: "铜矿石",
        res_type: "ore",
        cost: 10,
        prob: 70,
        min_level: 0,
        rarity: "普通",
    },
    MapResource {
        name: "铁矿石",
        res_type: "ore",
        cost: 15,
        prob: 50,
        min_level: 1,
        rarity: "普通",
    },
];

/// 获取地图资源集
fn get_map_resources(map_name: &str) -> (&'static [MapResource], &'static [MapResource]) {
    for rs in MAP_RESOURCES {
        if rs.map_name == map_name {
            return (rs.herbs, rs.ores);
        }
    }
    (DEFAULT_HERBS, DEFAULT_ORES)
}

/// 获取稀有度图标
fn rarity_icon(rarity: &str) -> &'static str {
    match rarity {
        "普通" => "⬜",
        "精良" => "🟩",
        "稀有" => "🟦",
        "史诗" => "🟪",
        "传说" => "🟧",
        _ => "⬜",
    }
}

/// 显示当前地图草药（分页）
pub fn cmd_map_herbs(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let location = db.read_basic(user_id, ITEM_LOCATION);
    if location.is_empty() {
        return "❌ 请先注册或进入地图".to_string();
    }
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let (herbs, _) = get_map_resources(&location);

    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    let per_page = 5;
    let total_pages = herbs.len().div_ceil(per_page);
    let page = page.min(total_pages.max(1));
    let start = (page - 1) * per_page;

    let gather_count_key = "gather_采药_count".to_string();
    let gather_count: i32 = db.read_user_data(user_id, &gather_count_key).parse().unwrap_or(0);
    let level = crate::gather::calc_level(gather_count);

    let mut r = format!("{}\n═══ {} · 可采集草药 ═══", prefix, location);

    for (i, herb) in herbs.iter().enumerate().skip(start).take(per_page) {
        let icon = rarity_icon(herb.rarity);
        let lock = if level < herb.min_level { "🔒" } else { "" };
        r.push_str(&format!(
            "\n{}. {} {} [{}] ({}金, {}%){}",
            i + 1,
            icon,
            herb.name,
            herb.rarity,
            herb.cost,
            herb.prob,
            lock
        ));
    }

    if herbs.is_empty() {
        r.push_str("\n  （当前地图暂无可采集草药）");
    }

    r.push_str(&format!("\n当前页：{}/{}", page, total_pages.max(1)));
    r.push_str(&format!(
        "\n🌿 采药等级：Lv.{} ({})",
        level,
        crate::gather::get_title(level)
    ));
    r.push_str("\n💡 发送「采集草药+名称」即可尝试采集");

    r
}

/// 显示当前地图矿石（分页）
pub fn cmd_map_ores(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let location = db.read_basic(user_id, ITEM_LOCATION);
    if location.is_empty() {
        return "❌ 请先注册或进入地图".to_string();
    }
    let prefix = crate::user::get_msg_prefix(db, user_id);
    let (_, ores) = get_map_resources(&location);

    let page: usize = args.trim().parse().unwrap_or(1).max(1);
    let per_page = 5;
    let total_pages = ores.len().div_ceil(per_page);
    let page = page.min(total_pages.max(1));
    let start = (page - 1) * per_page;

    let gather_count_key = "gather_挖矿_count".to_string();
    let gather_count: i32 = db.read_user_data(user_id, &gather_count_key).parse().unwrap_or(0);
    let level = crate::gather::calc_level(gather_count);

    let mut r = format!("{}\n═══ {} · 可采集矿石 ═══", prefix, location);

    for (i, ore) in ores.iter().enumerate().skip(start).take(per_page) {
        let icon = rarity_icon(ore.rarity);
        let lock = if level < ore.min_level { "🔒" } else { "" };
        r.push_str(&format!(
            "\n{}. {} {} [{}] ({}金, {}%){}",
            i + 1,
            icon,
            ore.name,
            ore.rarity,
            ore.cost,
            ore.prob,
            lock
        ));
    }

    if ores.is_empty() {
        r.push_str("\n  （当前地图暂无可采集矿石）");
    }

    r.push_str(&format!("\n当前页：{}/{}", page, total_pages.max(1)));
    r.push_str(&format!(
        "\n⛏️ 挖矿等级：Lv.{} ({})",
        level,
        crate::gather::get_title(level)
    ));
    r.push_str("\n💡 发送「采集矿石+名称」即可尝试采集");

    r
}

/// 采集地图资源（草药或矿石）
pub fn cmd_map_collect(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return "❌ 请指定要采集的资源名称\n💡 发送「地图草药」查看可采集草药\n💡 发送「地图矿石」查看可采集矿石"
            .to_string();
    }

    // 检查生存状态
    let hp_str = db.read_basic(user_id, ITEM_HP_CURRENT);
    let hp: i32 = hp_str.parse().unwrap_or(0);
    if hp <= 0 {
        return "❌ 您已阵亡，无法采集！请先恢复生命值".to_string();
    }

    let location = db.read_basic(user_id, ITEM_LOCATION);
    if location.is_empty() {
        return "❌ 请先注册或进入地图".to_string();
    }

    let prefix = crate::user::get_msg_prefix(db, user_id);
    let (herbs, ores) = get_map_resources(&location);

    // 在草药和矿石中查找匹配的资源
    let mut found_resource: Option<&MapResource> = None;
    let mut found_type = "";

    // 精确匹配
    for h in herbs {
        if h.name == args {
            found_resource = Some(h);
            found_type = "采药";
            break;
        }
    }
    if found_resource.is_none() {
        for o in ores {
            if o.name == args {
                found_resource = Some(o);
                found_type = "挖矿";
                break;
            }
        }
    }
    // 模糊匹配
    if found_resource.is_none() {
        for h in herbs {
            if h.name.contains(args) || args.contains(h.name) {
                found_resource = Some(h);
                found_type = "采药";
                break;
            }
        }
    }
    if found_resource.is_none() {
        for o in ores {
            if o.name.contains(args) || args.contains(o.name) {
                found_resource = Some(o);
                found_type = "挖矿";
                break;
            }
        }
    }

    let resource = match found_resource {
        Some(r) => r,
        None => {
            return format!(
                "❌ 在「{}」未找到名为「{}」的可采集资源\n💡 发送「地图草药」或「地图矿石」查看当前地图资源",
                location, args
            );
        }
    };

    // 检查采集技能等级
    let gather_count_key = format!("gather_{}_count", found_type);
    let gather_count: i32 = db.read_user_data(user_id, &gather_count_key).parse().unwrap_or(0);
    let level = crate::gather::calc_level(gather_count);

    if level < resource.min_level {
        return format!(
            "🔒 采集「{}」需要{}等级 Lv.{}\n当前等级：Lv.{} ({})\n💡 继续练习采集提升等级吧！",
            resource.name,
            if found_type == "采药" { "采药" } else { "挖矿" },
            resource.min_level,
            level,
            crate::gather::get_title(level)
        );
    }

    // 计算实际消耗（等级加成减少消耗）
    let cost_reduction = std::cmp::min(level * 2, 20);
    let actual_cost = resource.cost * (100 - cost_reduction) / 100;

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < actual_cost as i64 {
        return format!("❌ 金币不足！采集「{}」需要{}金币", resource.name, actual_cost);
    }

    // 扣除金币
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, actual_cost as i64);

    // 计算成功率（等级加成提升概率）
    let prob_bonus = std::cmp::min(level, 10);
    let actual_prob = std::cmp::min(resource.prob + prob_bonus, 95);

    let roll = rand::random::<u32>() % 100;

    let mut r = format!("{}\n🌿 采集「{}」...", prefix, resource.name);

    if roll < actual_prob as u32 {
        // 采集成功
        db.add_item(user_id, resource.name, 1);
        r.push_str(&format!("\n✅ 采集成功！获得 [{}] ×1", resource.name));

        // 更新采集次数
        let new_count = gather_count + 1;
        db.write_user_data(user_id, &gather_count_key, &new_count.to_string());

        // 检查升级
        let new_level = crate::gather::calc_level(new_count);
        if new_level > level {
            r.push_str(&format!(
                "\n🎉 {}技能升级！Lv.{} → Lv.{} [{}]",
                if found_type == "采药" { "采药" } else { "挖矿" },
                level,
                new_level,
                crate::gather::get_title(new_level)
            ));
        }

        r.push_str(&format!("\n💰 消耗{}金币", actual_cost));
        r.push_str(&format!("\n📊 成功率：{}%", actual_prob));

        // 更新每日任务进度
        crate::daily_quest::on_gathered(db, user_id);
        crate::weekly_quest::on_gathered(db, user_id);
    } else {
        r.push_str(&format!("\n❌ 采集失败！「{}」从指缝间溜走了...", resource.name));
        r.push_str(&format!("\n💰 消耗{}金币", actual_cost));
        r.push_str(&format!("\n📊 成功率：{}%", actual_prob));

        // 失败也计数但不升级
        let new_count = gather_count + 1;
        db.write_user_data(user_id, &gather_count_key, &new_count.to_string());
    }

    // 显示资源剩余（背包中该物品数量）
    let qty = db.get_item_count(user_id, resource.name);
    r.push_str(&format!("\n🎒 {}持有：{}个", resource.name, qty));

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rarity_icon_all_levels() {
        assert_eq!(rarity_icon("普通"), "⬜");
        assert_eq!(rarity_icon("精良"), "🟩");
        assert_eq!(rarity_icon("稀有"), "🟦");
        assert_eq!(rarity_icon("史诗"), "🟪");
        assert_eq!(rarity_icon("传说"), "🟧");
        assert_eq!(rarity_icon("未知"), "⬜"); // default
    }

    #[test]
    fn test_map_resources_not_empty() {
        assert!(MAP_RESOURCES.len() >= 3, "Should have at least 3 map resource sets");
    }

    #[test]
    fn test_map_resources_each_has_herbs_and_ores() {
        for rs in MAP_RESOURCES {
            assert!(!rs.herbs.is_empty(), "{} should have herbs", rs.map_name);
            assert!(!rs.ores.is_empty(), "{} should have ores", rs.map_name);
        }
    }

    #[test]
    fn test_get_map_resources_known_map() {
        let (herbs, ores) = get_map_resources("格兰森林");
        assert!(herbs.len() > 0);
        assert!(ores.len() > 0);
    }

    #[test]
    fn test_get_map_resources_unknown_map_fallback() {
        let (herbs, ores) = get_map_resources("不存在的地图");
        // Should fall back to defaults
        assert_eq!(herbs.len(), DEFAULT_HERBS.len());
        assert_eq!(ores.len(), DEFAULT_ORES.len());
    }

    #[test]
    fn test_default_resources() {
        assert!(DEFAULT_HERBS.len() >= 2, "Default herbs should have at least 2 entries");
        assert!(DEFAULT_ORES.len() >= 1, "Default ores should have at least 1 entry");
    }

    #[test]
    fn test_resource_positive_values() {
        for rs in MAP_RESOURCES {
            for herb in rs.herbs {
                assert!(herb.prob > 0 && herb.prob <= 100, "{} prob out of range", herb.name);
                assert!(herb.cost >= 0, "{} cost should be >= 0", herb.name);
            }
            for ore in rs.ores {
                assert!(ore.prob > 0 && ore.prob <= 100, "{} prob out of range", ore.name);
                assert!(ore.cost >= 0, "{} cost should be >= 0", ore.name);
            }
        }
    }
}

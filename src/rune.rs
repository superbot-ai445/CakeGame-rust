/// CakeGame 符文镶嵌系统 (Rune Inscription System)
///
/// 符文是一种独立于宝石和附魔的装备强化层。
/// 玩家通过战斗、采集和合成获得符文碎片，合成完整符文后镶嵌到装备符文槽中。
///
/// 符文等级: 碎片(Ⅰ) → 初级(Ⅱ) → 中级(Ⅲ) → 高级(Ⅳ) → 传说(Ⅴ) → 神话(Ⅵ)
/// 符文类型: 7种元素符文 (火🔥/水💧/风🌪️/地🪨/雷⚡/光✨/暗🌑)
///
/// 指令: 符文列表/符文合成/镶嵌符文/卸下符文/符文详情/符文排行/符文帮助
/// 数据存储: Global 表 SECTION='rune'
use crate::db::Database;

/// 符文等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuneGrade {
    Fragment,  // 碎片
    Basic,     // 初级
    Advanced,  // 中级
    Superior,  // 高级
    Legendary, // 传说
    Mythic,    // 神话
}

impl RuneGrade {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Fragment => "碎片",
            Self::Basic => "初级",
            Self::Advanced => "中级",
            Self::Superior => "高级",
            Self::Legendary => "传说",
            Self::Mythic => "神话",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Fragment => "🔘",
            Self::Basic => "⚪",
            Self::Advanced => "🟢",
            Self::Superior => "🔵",
            Self::Legendary => "🟣",
            Self::Mythic => "🟡",
        }
    }

    #[allow(dead_code)]
    pub fn level(&self) -> i32 {
        match self {
            Self::Fragment => 0,
            Self::Basic => 1,
            Self::Advanced => 2,
            Self::Superior => 3,
            Self::Legendary => 4,
            Self::Mythic => 5,
        }
    }

    pub fn from_level(level: i32) -> Self {
        match level {
            0 => Self::Fragment,
            1 => Self::Basic,
            2 => Self::Advanced,
            3 => Self::Superior,
            4 => Self::Legendary,
            _ => Self::Mythic,
        }
    }

    /// 合成到下一级需要的碎片数量
    pub fn combine_cost(&self) -> i32 {
        match self {
            Self::Fragment => 3,  // 3碎片 → 1初级
            Self::Basic => 5,     // 5初级 → 1中级
            Self::Advanced => 5,  // 5中级 → 1高级
            Self::Superior => 3,  // 3高级 → 1传说
            Self::Legendary => 3, // 3传说 → 1神话
            Self::Mythic => 0,    // 满级
        }
    }
}

/// 符文类型
#[derive(Debug, Clone, Copy)]
pub struct RuneType {
    pub name: &'static str,
    pub element: &'static str,
    pub emoji: &'static str,
    pub hp_bonus: i32,
    pub ad_bonus: i32,
    pub ap_bonus: i32,
    pub def_bonus: i32,
    pub mdef_bonus: i32,
    pub crit_bonus: i32,
    pub special: &'static str,
}

/// 7种元素符文
pub const RUNE_TYPES: &[RuneType] = &[
    RuneType {
        name: "烈焰符文",
        element: "火",
        emoji: "🔥",
        hp_bonus: 20,
        ad_bonus: 15,
        ap_bonus: 5,
        def_bonus: 0,
        mdef_bonus: 0,
        crit_bonus: 2,
        special: "攻击时5%概率附加灼烧(2回合)",
    },
    RuneType {
        name: "寒潮符文",
        element: "水",
        emoji: "💧",
        hp_bonus: 30,
        ad_bonus: 5,
        ap_bonus: 10,
        def_bonus: 5,
        mdef_bonus: 5,
        crit_bonus: 0,
        special: "受到攻击时3%概率冰冻反击者1回合",
    },
    RuneType {
        name: "疾风符文",
        element: "风",
        emoji: "🌪️",
        hp_bonus: 10,
        ad_bonus: 10,
        ap_bonus: 10,
        def_bonus: 0,
        mdef_bonus: 0,
        crit_bonus: 5,
        special: "闪避率+3%",
    },
    RuneType {
        name: "磐石符文",
        element: "地",
        emoji: "🪨",
        hp_bonus: 50,
        ad_bonus: 0,
        ap_bonus: 0,
        def_bonus: 15,
        mdef_bonus: 10,
        crit_bonus: 0,
        special: "受到致命攻击时5%概率保留1HP",
    },
    RuneType {
        name: "雷霆符文",
        element: "雷",
        emoji: "⚡",
        hp_bonus: 15,
        ad_bonus: 20,
        ap_bonus: 15,
        def_bonus: 0,
        mdef_bonus: 0,
        crit_bonus: 3,
        special: "暴击伤害+15%",
    },
    RuneType {
        name: "圣光符文",
        element: "光",
        emoji: "✨",
        hp_bonus: 40,
        ad_bonus: 5,
        ap_bonus: 5,
        def_bonus: 8,
        mdef_bonus: 8,
        crit_bonus: 0,
        special: "每回合自动回复2%最大HP",
    },
    RuneType {
        name: "暗影符文",
        element: "暗",
        emoji: "🌑",
        hp_bonus: 10,
        ad_bonus: 12,
        ap_bonus: 12,
        def_bonus: 0,
        mdef_bonus: 0,
        crit_bonus: 4,
        special: "击杀敌人时回复10%最大HP",
    },
];

/// 符文套装定义 (收集同元素多个等级触发)
struct RuneSetBonus {
    element: &'static str,
    pieces_2: &'static str, // 2件套效果
    pieces_4: &'static str, // 4件套效果
    pieces_6: &'static str, // 6件套效果
}

const RUNE_SET_BONUSES: &[RuneSetBonus] = &[
    RuneSetBonus {
        element: "火",
        pieces_2: "物攻+5%",
        pieces_4: "灼烧概率提升至10%",
        pieces_6: "攻击时15%概率造成双倍灼烧伤害",
    },
    RuneSetBonus {
        element: "水",
        pieces_2: "HP+8%",
        pieces_4: "冰冻概率提升至8%",
        pieces_6: "被冰冻目标受到的伤害+25%",
    },
    RuneSetBonus {
        element: "风",
        pieces_2: "暴击+3%",
        pieces_4: "闪避率提升至8%",
        pieces_6: "闪避成功后下次攻击必暴击",
    },
    RuneSetBonus {
        element: "地",
        pieces_2: "防御+10%",
        pieces_4: "保命概率提升至10%",
        pieces_6: "保命后获得3回合20%减伤护盾",
    },
    RuneSetBonus {
        element: "雷",
        pieces_2: "魔攻+5%",
        pieces_4: "暴击伤害提升至25%",
        pieces_6: "暴击时20%概率连锁闪电攻击相邻目标",
    },
    RuneSetBonus {
        element: "光",
        pieces_2: "防御+5%",
        pieces_4: "每回合回复提升至4%",
        pieces_6: "复活1次(每场战斗1次)，回复30%HP",
    },
    RuneSetBonus {
        element: "暗",
        pieces_2: "暴击+2%",
        pieces_4: "击杀回复提升至20%",
        pieces_6: "对HP低于30%的目标造成50%额外伤害",
    },
];

/// 符文碎片掉落表 (怪物等级 → 可能掉落的元素)
#[allow(dead_code)]
fn get_fragment_drops(map_level: i32) -> Vec<&'static str> {
    let mut drops = vec!["火", "水", "风", "地"];
    if map_level >= 15 {
        drops.push("雷");
    }
    if map_level >= 30 {
        drops.push("光");
    }
    if map_level >= 50 {
        drops.push("暗");
    }
    drops
}

/// 计算符文等级带来的属性加成倍率
fn grade_multiplier(grade: &RuneGrade) -> f64 {
    match grade {
        RuneGrade::Fragment => 0.0,
        RuneGrade::Basic => 1.0,
        RuneGrade::Advanced => 1.8,
        RuneGrade::Superior => 3.0,
        RuneGrade::Legendary => 5.0,
        RuneGrade::Mythic => 8.0,
    }
}

/// 数据存储键
const SECTION: &str = "rune";
const INVENTORY_KEY: &str = "inventory"; // 符文背包
const INSCRIBED_KEY: &str = "inscribed"; // 已镶嵌的符文

/// 符文背包条目: (元素, 等级, 数量)
#[derive(Debug, Clone)]
struct RuneEntry {
    element: String,
    grade: i32,
    count: i32,
}

/// 解析符文背包 JSON
fn parse_inventory(data: &str) -> Vec<RuneEntry> {
    if data.is_empty() {
        return Vec::new();
    }
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap_or(serde_json::Value::Null);
    let mut result = Vec::new();
    if let Some(arr) = parsed.as_array() {
        for item in arr {
            let element = item.get("e").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let grade = item.get("g").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let count = item.get("c").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if !element.is_empty() && count > 0 {
                result.push(RuneEntry { element, grade, count });
            }
        }
    }
    result
}

/// 序列化符文背包 JSON
fn serialize_inventory(entries: &[RuneEntry]) -> String {
    let arr: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "e": e.element,
                "g": e.grade,
                "c": e.count,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// 已镶嵌条目: 设备槽位 → (元素, 等级)
#[derive(Debug, Clone)]
struct InscribedEntry {
    slot: String,
    element: String,
    grade: i32,
}

/// 解析已镶嵌 JSON
fn parse_inscribed(data: &str) -> Vec<InscribedEntry> {
    if data.is_empty() {
        return Vec::new();
    }
    let parsed: serde_json::Value = serde_json::from_str(data).unwrap_or(serde_json::Value::Null);
    let mut result = Vec::new();
    if let Some(arr) = parsed.as_array() {
        for item in arr {
            let slot = item.get("s").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let element = item.get("e").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let grade = item.get("g").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            if !slot.is_empty() && !element.is_empty() {
                result.push(InscribedEntry { slot, element, grade });
            }
        }
    }
    result
}

/// 序列化已镶嵌 JSON
fn serialize_inscribed(entries: &[InscribedEntry]) -> String {
    let arr: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "s": e.slot,
                "e": e.element,
                "g": e.grade,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// 获取符文类型信息
fn get_rune_type(element: &str) -> Option<&'static RuneType> {
    RUNE_TYPES.iter().find(|r| r.element == element)
}

/// 获取符文套装信息
fn get_set_bonus(element: &str) -> Option<&'static RuneSetBonus> {
    RUNE_SET_BONUSES.iter().find(|s| s.element == element)
}

/// 计算已镶嵌符文的总属性加成
#[allow(dead_code)]
pub fn get_rune_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32, i32) {
    let inscribed_data = db.global_get(SECTION, &format!("{}_{}", user_id, INSCRIBED_KEY));
    let entries = parse_inscribed(&inscribed_data);

    let mut total_hp = 0i32;
    let mut total_ad = 0i32;
    let mut total_ap = 0i32;
    let mut total_def = 0i32;
    let mut total_mdef = 0i32;
    let mut total_crit = 0i32;

    for entry in &entries {
        if let Some(rt) = get_rune_type(&entry.element) {
            let mult = grade_multiplier(&RuneGrade::from_level(entry.grade));
            total_hp += (rt.hp_bonus as f64 * mult) as i32;
            total_ad += (rt.ad_bonus as f64 * mult) as i32;
            total_ap += (rt.ap_bonus as f64 * mult) as i32;
            total_def += (rt.def_bonus as f64 * mult) as i32;
            total_mdef += (rt.mdef_bonus as f64 * mult) as i32;
            total_crit += (rt.crit_bonus as f64 * mult) as i32;
        }
    }

    (total_hp, total_ad, total_ap, total_def, total_mdef, total_crit)
}

/// 计算套装效果数量
fn count_set_pieces(entries: &[InscribedEntry]) -> std::collections::HashMap<String, i32> {
    let mut counts = std::collections::HashMap::new();
    for entry in entries {
        *counts.entry(entry.element.clone()).or_insert(0) += 1;
    }
    counts
}

// ==================== 指令处理 ====================

/// 符文列表 — 查看拥有的所有符文
pub fn cmd_rune_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let inv_data = db.global_get(SECTION, &format!("{}_{}", user_id, INVENTORY_KEY));
    let entries = parse_inventory(&inv_data);

    if entries.is_empty() {
        return "📭 符文背包为空\n\n💡 击杀怪物可获得符文碎片，使用「符文合成」将碎片合成为完整符文".to_string();
    }

    let mut out = String::from("🔮 ═══ 符文列表 ═══\n");

    // 按元素分组
    let mut by_element: std::collections::HashMap<String, Vec<&RuneEntry>> = std::collections::HashMap::new();
    for entry in &entries {
        by_element.entry(entry.element.clone()).or_default().push(entry);
    }

    for (element, items) in &by_element {
        let rune_type = get_rune_type(element);
        let emoji = rune_type.map(|r| r.emoji).unwrap_or("❓");
        out.push_str(&format!("\n{} {}系:\n", emoji, element));
        for item in items {
            let grade = RuneGrade::from_level(item.grade);
            out.push_str(&format!(
                "  {} {}×{} (等级{})\n",
                grade.emoji(),
                grade.name(),
                item.count,
                item.grade
            ));
        }
    }

    // 显示已镶嵌
    let inscribed_data = db.global_get(SECTION, &format!("{}_{}", user_id, INSCRIBED_KEY));
    let inscribed = parse_inscribed(&inscribed_data);
    if !inscribed.is_empty() {
        out.push_str(&format!("\n📌 已镶嵌 ({}/6槽位):\n", inscribed.len()));
        for entry in &inscribed {
            let grade = RuneGrade::from_level(entry.grade);
            if let Some(rt) = get_rune_type(&entry.element) {
                out.push_str(&format!(
                    "  [{}] {}{} {} Lv{}\n",
                    entry.slot,
                    rt.emoji,
                    grade.emoji(),
                    rt.name,
                    entry.grade
                ));
            }
        }

        // 套装效果
        let set_counts = count_set_pieces(&inscribed);
        let mut has_set = false;
        for (element, count) in &set_counts {
            if *count >= 2 {
                if !has_set {
                    out.push_str("\n🎯 套装效果:\n");
                    has_set = true;
                }
                if let Some(sb) = get_set_bonus(element) {
                    out.push_str(&format!(
                        "  {} {}×{}: ",
                        get_rune_type(element).map(|r| r.emoji).unwrap_or(""),
                        element,
                        count
                    ));
                    if *count >= 6 {
                        out.push_str(&format!("{}\n", sb.pieces_6));
                    } else if *count >= 4 {
                        out.push_str(&format!("{}\n", sb.pieces_4));
                    } else {
                        out.push_str(&format!("{}\n", sb.pieces_2));
                    }
                }
            }
        }
    }

    // 属性加成
    let (hp, ad, ap, def, mdef, crit) = get_rune_bonus(db, user_id);
    if hp + ad + ap + def + mdef + crit > 0 {
        out.push_str("\n📊 符文加成总计:");
        if hp > 0 {
            out.push_str(&format!(" HP+{}", hp));
        }
        if ad > 0 {
            out.push_str(&format!(" 物攻+{}", ad));
        }
        if ap > 0 {
            out.push_str(&format!(" 魔攻+{}", ap));
        }
        if def > 0 {
            out.push_str(&format!(" 防御+{}", def));
        }
        if mdef > 0 {
            out.push_str(&format!(" 魔抗+{}", mdef));
        }
        if crit > 0 {
            out.push_str(&format!(" 暴击+{}", crit));
        }
        out.push('\n');
    }

    out
}

/// 符文合成 — 将低级符文合成为高级符文
pub fn cmd_rune_combine(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 参数: 元素 等级 (如 "火 0" 表示将3个火碎片合成1个初级火符文)
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 {
        return "📝 用法: 符文合成 [元素] [等级]\n\n示例: 符文合成 火 0 (将3个火碎片合成1个初级火符文)\n等级: 0=碎片 1=初级 2=中级 3=高级 4=传说".to_string();
    }

    let element = parts[0];
    let grade_level: i32 = parts[1].parse().unwrap_or(-1);

    // 验证元素
    if get_rune_type(element).is_none() {
        return format!("❌ 未知元素「{}」，有效元素: 火/水/风/地/雷/光/暗", element);
    }

    // 验证等级
    if !(0..=4).contains(&grade_level) {
        return "❌ 有效等级: 0=碎片 1=初级 2=中级 3=高级 4=传说".to_string();
    }

    let grade = RuneGrade::from_level(grade_level);
    let cost = grade.combine_cost();
    if cost == 0 {
        return "❌ 神话级符文为最高级，无法继续合成".to_string();
    }

    let next_grade = RuneGrade::from_level(grade_level + 1);

    // 读取背包
    let inv_key = format!("{}_{}", user_id, INVENTORY_KEY);
    let inv_data = db.global_get(SECTION, &inv_key);
    let mut entries = parse_inventory(&inv_data);

    // 找到对应条目
    let idx = entries
        .iter()
        .position(|e| e.element == element && e.grade == grade_level);
    match idx {
        Some(i) => {
            if entries[i].count < cost {
                return format!(
                    "❌ {}{}{}不足，需要{}个，当前{}个",
                    grade.emoji(),
                    element,
                    grade.name(),
                    cost,
                    entries[i].count
                );
            }
            entries[i].count -= cost;
            if entries[i].count == 0 {
                entries.remove(i);
            }
        }
        None => {
            return format!("❌ 没有{}{}{}，无法合成", grade.emoji(), element, grade.name());
        }
    }

    // 添加合成后的高级符文
    let next_level = grade_level + 1;
    if let Some(existing) = entries
        .iter_mut()
        .find(|e| e.element == element && e.grade == next_level)
    {
        existing.count += 1;
    } else {
        entries.push(RuneEntry {
            element: element.to_string(),
            grade: next_level,
            count: 1,
        });
    }

    // 消耗金币
    let gold_cost = (next_level as i64) * 500;
    let current_gold: i64 = db.read_basic(user_id, "金币").parse().unwrap_or(0);
    if current_gold < gold_cost {
        // 回滚
        if let Some(existing) = entries
            .iter_mut()
            .find(|e| e.element == element && e.grade == grade_level)
        {
            existing.count += cost;
        } else {
            entries.push(RuneEntry {
                element: element.to_string(),
                grade: grade_level,
                count: cost,
            });
        }
        if let Some(existing) = entries
            .iter_mut()
            .find(|e| e.element == element && e.grade == next_level)
        {
            existing.count -= 1;
            if existing.count == 0 {
                entries.retain(|e| !(e.element == element && e.grade == next_level));
            }
        }
        return format!(
            "❌ 金币不足，合成{}需要{}金币，当前{}",
            next_grade.name(),
            gold_cost,
            current_gold
        );
    }
    db.write_basic(user_id, "金币", &(current_gold - gold_cost).to_string());

    db.global_set(SECTION, &inv_key, &serialize_inventory(&entries));

    format!(
        "✨ 符文合成成功！\n\n{} {}{}×{} → {} {}{}×1\n💰 消耗: {}金币\n\n💡 使用「镶嵌符文 {} {}」将其装备到装备上",
        grade.emoji(),
        element,
        grade.name(),
        cost,
        next_grade.emoji(),
        element,
        next_grade.name(),
        gold_cost,
        element,
        next_level
    )
}

/// 镶嵌符文 — 将符文镶嵌到装备槽位
pub fn cmd_rune_inscribe(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    // 参数: 元素 等级 槽位 (如 "火 3 武器")
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 3 {
        return "📝 用法: 镶嵌符文 [元素] [等级] [槽位]\n\n槽位: 武器/头盔/胸甲/手套/鞋子/饰品\n示例: 镶嵌符文 火 3 武器".to_string();
    }

    let element = parts[0];
    let grade_level: i32 = parts[1].parse().unwrap_or(-1);
    let slot = parts[2];

    // 验证元素
    if get_rune_type(element).is_none() {
        return format!("❌ 未知元素「{}」，有效元素: 火/水/风/地/雷/光/暗", element);
    }

    // 验证等级 (必须 ≥ 1 才能镶嵌)
    if !(1..=5).contains(&grade_level) {
        return "❌ 镶嵌需要等级≥1的符文 (1=初级 2=中级 3=高级 4=传说 5=神话)".to_string();
    }

    // 验证槽位
    let valid_slots = ["武器", "头盔", "胸甲", "手套", "鞋子", "饰品"];
    if !valid_slots.contains(&slot) {
        return format!("❌ 无效槽位「{}」，有效槽位: {}", slot, valid_slots.join("/"));
    }

    // 读取背包
    let inv_key = format!("{}_{}", user_id, INVENTORY_KEY);
    let inv_data = db.global_get(SECTION, &inv_key);
    let mut entries = parse_inventory(&inv_data);

    // 找到并扣减符文
    let idx = entries
        .iter()
        .position(|e| e.element == element && e.grade == grade_level);
    match idx {
        Some(i) => {
            if entries[i].count < 1 {
                return format!(
                    "❌ 没有{}{} Lv{}",
                    get_rune_type(element).unwrap().emoji,
                    element,
                    grade_level
                );
            }
            entries[i].count -= 1;
            if entries[i].count == 0 {
                entries.remove(i);
            }
        }
        None => {
            return format!(
                "❌ 没有{}{} Lv{}",
                get_rune_type(element).unwrap().emoji,
                element,
                grade_level
            );
        }
    }

    // 读取已镶嵌
    let ins_key = format!("{}_{}", user_id, INSCRIBED_KEY);
    let ins_data = db.global_get(SECTION, &ins_key);
    let mut inscribed = parse_inscribed(&ins_data);

    // 检查该槽位是否已有符文
    if let Some(existing) = inscribed.iter().find(|e| e.slot == slot) {
        // 返还旧符文到背包
        let old_element = existing.element.clone();
        let old_grade = existing.grade;
        if let Some(idx) = entries
            .iter_mut()
            .position(|e| e.element == old_element && e.grade == old_grade)
        {
            entries[idx].count += 1;
        } else {
            entries.push(RuneEntry {
                element: old_element,
                grade: old_grade,
                count: 1,
            });
        }
        // 移除旧镶嵌
        inscribed.retain(|e| e.slot != slot);
    }

    // 镶嵌新符文
    inscribed.push(InscribedEntry {
        slot: slot.to_string(),
        element: element.to_string(),
        grade: grade_level,
    });

    // 保存
    db.global_set(SECTION, &inv_key, &serialize_inventory(&entries));
    db.global_set(SECTION, &ins_key, &serialize_inscribed(&inscribed));

    let rt = get_rune_type(element).unwrap();
    let grade = RuneGrade::from_level(grade_level);
    let mult = grade_multiplier(&grade);

    format!(
        "🔮 镶嵌成功！\n\n{}{} {} 已镶嵌到[{}]\n属性加成: HP+{} 物攻+{} 魔攻+{} 防御+{} 魔抗+{} 暴击+{}",
        rt.emoji,
        grade.emoji(),
        rt.name,
        slot,
        (rt.hp_bonus as f64 * mult) as i32,
        (rt.ad_bonus as f64 * mult) as i32,
        (rt.ap_bonus as f64 * mult) as i32,
        (rt.def_bonus as f64 * mult) as i32,
        (rt.mdef_bonus as f64 * mult) as i32,
        (rt.crit_bonus as f64 * mult) as i32
    )
}

/// 卸下符文 — 从装备槽位取下符文
pub fn cmd_rune_remove(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let slot = args.trim();
    if slot.is_empty() {
        return "📝 用法: 卸下符文 [槽位]\n\n槽位: 武器/头盔/胸甲/手套/鞋子/饰品".to_string();
    }

    let valid_slots = ["武器", "头盔", "胸甲", "手套", "鞋子", "饰品"];
    if !valid_slots.contains(&slot) {
        return format!("❌ 无效槽位「{}」，有效槽位: {}", slot, valid_slots.join("/"));
    }

    // 读取已镶嵌
    let ins_key = format!("{}_{}", user_id, INSCRIBED_KEY);
    let ins_data = db.global_get(SECTION, &ins_key);
    let mut inscribed = parse_inscribed(&ins_data);

    let idx = inscribed.iter().position(|e| e.slot == slot);
    match idx {
        Some(i) => {
            let removed = inscribed.remove(i);

            // 返还到背包
            let inv_key = format!("{}_{}", user_id, INVENTORY_KEY);
            let inv_data = db.global_get(SECTION, &inv_key);
            let mut entries = parse_inventory(&inv_data);

            if let Some(existing) = entries
                .iter_mut()
                .find(|e| e.element == removed.element && e.grade == removed.grade)
            {
                existing.count += 1;
            } else {
                entries.push(RuneEntry {
                    element: removed.element.clone(),
                    grade: removed.grade,
                    count: 1,
                });
            }

            db.global_set(SECTION, &inv_key, &serialize_inventory(&entries));
            db.global_set(SECTION, &ins_key, &serialize_inscribed(&inscribed));

            let rt = get_rune_type(&removed.element).unwrap();
            let grade = RuneGrade::from_level(removed.grade);
            format!(
                "✅ 已从[{}]卸下 {}{} {}\n符文已返还到符文背包",
                slot,
                rt.emoji,
                grade.emoji(),
                rt.name
            )
        }
        None => format!("❌ [{}]没有镶嵌符文", slot),
    }
}

/// 符文详情 — 查看某个元素符文的详细信息
pub fn cmd_rune_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let element = args.trim();
    if element.is_empty() {
        return "📝 用法: 符文详情 [元素]\n\n有效元素: 火/水/风/地/雷/光/暗".to_string();
    }

    let rt = match get_rune_type(element) {
        Some(rt) => rt,
        None => return format!("❌ 未知元素「{}」，有效元素: 火/水/风/地/雷/光/暗", element),
    };

    let mut out = format!("{} ═══ {}详情 ═══\n", rt.emoji, rt.name);
    out.push_str(&format!("元素: {} {}\n\n", element, rt.emoji));

    // 各等级属性
    out.push_str("📊 各等级属性:\n");
    for level in 1..=5 {
        let grade = RuneGrade::from_level(level);
        let mult = grade_multiplier(&grade);
        out.push_str(&format!(
            "{} Lv{}: HP+{} 物攻+{} 魔攻+{} 防御+{} 魔抗+{} 暴击+{}\n",
            grade.emoji(),
            level,
            (rt.hp_bonus as f64 * mult) as i32,
            (rt.ad_bonus as f64 * mult) as i32,
            (rt.ap_bonus as f64 * mult) as i32,
            (rt.def_bonus as f64 * mult) as i32,
            (rt.mdef_bonus as f64 * mult) as i32,
            (rt.crit_bonus as f64 * mult) as i32
        ));
    }

    // 特殊效果
    out.push_str(&format!("\n⚡ 特殊效果: {}\n", rt.special));

    // 套装效果
    if let Some(sb) = get_set_bonus(element) {
        out.push_str(&format!(
            "\n🎯 套装效果:\n  2件: {}\n  4件: {}\n  6件: {}\n",
            sb.pieces_2, sb.pieces_4, sb.pieces_6
        ));
    }

    // 当前拥有
    let inv_data = db.global_get(SECTION, &format!("{}_{}", user_id, INVENTORY_KEY));
    let entries = parse_inventory(&inv_data);
    let total: i32 = entries.iter().filter(|e| e.element == element).map(|e| e.count).sum();
    out.push_str(&format!("\n📦 当前拥有: {}个\n", total));

    out
}

/// 符文排行 — 全服符文战力排名
pub fn cmd_rune_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    // 从 Global 表获取所有用户符文数据，计算战力
    let all_users = db.all_users();

    let mut rankings: Vec<(String, i32)> = Vec::new();

    for key in &all_users {
        if let Some(uid_part) = key.strip_suffix(&format!("_{}", INSCRIBED_KEY)) {
            let data = db.global_get(SECTION, key);
            let entries = parse_inscribed(&data);
            let mut power = 0i32;
            for entry in &entries {
                if let Some(rt) = get_rune_type(&entry.element) {
                    let mult = grade_multiplier(&RuneGrade::from_level(entry.grade));
                    power += ((rt.hp_bonus + rt.ad_bonus + rt.ap_bonus + rt.def_bonus + rt.mdef_bonus + rt.crit_bonus)
                        as f64
                        * mult) as i32;
                }
            }
            if power > 0 {
                rankings.push((uid_part.to_string(), power));
            }
        }
    }

    if rankings.is_empty() {
        return "📭 暂无符文排行数据\n\n💡 镶嵌符文后自动参与排行".to_string();
    }

    rankings.sort_by_key(|b| std::cmp::Reverse(b.1));

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = String::from("🔮 ═══ 符文排行 ═══\n");
    for (i, (uid, power)) in rankings.iter().enumerate().take(15) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, "名称");
        let display = if name.is_empty() { uid.clone() } else { name };
        let stars = if *power > 500 {
            "⭐⭐⭐⭐⭐"
        } else if *power > 300 {
            "⭐⭐⭐⭐"
        } else if *power > 150 {
            "⭐⭐⭐"
        } else if *power > 50 {
            "⭐⭐"
        } else {
            "⭐"
        };
        out.push_str(&format!("{}{}. {} — {}战力 {}\n", medal, i + 1, display, power, stars));
    }

    out
}

/// 符文帮助
pub fn cmd_rune_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = String::from("🔮 ═══ 符文系统帮助 ═══\n\n");
    out.push_str("📌 基本概念:\n");
    out.push_str("  符文是独立于宝石和附魔的装备强化系统\n");
    out.push_str("  7种元素: 🔥火 💧水 🌪️风 🪨地 ⚡雷 ✨光 🌑暗\n");
    out.push_str("  6个等级: 碎片→初级→中级→高级→传说→神话\n\n");

    out.push_str("📋 指令列表:\n");
    out.push_str("  符文列表 — 查看拥有的所有符文\n");
    out.push_str("  符文合成 [元素] [等级] — 将低级符文合成高级\n");
    out.push_str("  镶嵌符文 [元素] [等级] [槽位] — 镶嵌到装备\n");
    out.push_str("  卸下符文 [槽位] — 取下已镶嵌的符文\n");
    out.push_str("  符文详情 [元素] — 查看元素符文详细信息\n");
    out.push_str("  符文排行 — 全服符文战力排名\n");
    out.push_str("  符文帮助 — 显示本帮助\n\n");

    out.push_str("💡 获取方式:\n");
    out.push_str("  • 击杀怪物掉落符文碎片\n");
    out.push_str("  • 碎片合成: 3碎片→1初级\n");
    out.push_str("  • 继续合成: 5初级→1中级, 5中级→1高级, 3高级→1传说, 3传说→1神话\n\n");

    out.push_str("🎯 套装效果:\n");
    out.push_str("  同元素镶嵌2/4/6个触发套装加成\n");
    out.push_str("  混搭不同元素可获得多种属性加成\n\n");

    out.push_str("⚡ 槽位: 武器/头盔/胸甲/手套/鞋子/饰品 (共6个)\n");

    out
}

/// 掉落符文碎片 (战斗系统调用)
#[allow(dead_code)]
pub fn drop_rune_fragment(db: &Database, user_id: &str, monster_level: i32) -> Option<String> {
    let mut rng = rand::thread_rng();
    use rand::Rng;

    // 30%概率掉落
    if rng.gen_range(0..100) >= 30 {
        return None;
    }

    let drops = get_fragment_drops(monster_level);
    let element = drops[rng.gen_range(0..drops.len())];

    // 添加到背包
    let inv_key = format!("{}_{}", user_id, INVENTORY_KEY);
    let inv_data = db.global_get(SECTION, &inv_key);
    let mut entries = parse_inventory(&inv_data);

    if let Some(existing) = entries.iter_mut().find(|e| e.element == element && e.grade == 0) {
        existing.count += 1;
    } else {
        entries.push(RuneEntry {
            element: element.to_string(),
            grade: 0,
            count: 1,
        });
    }

    db.global_set(SECTION, &inv_key, &serialize_inventory(&entries));

    let rt = get_rune_type(element).unwrap();
    Some(format!("🔮 获得{} {}碎片×1", rt.emoji, element))
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rune_type_count() {
        assert_eq!(RUNE_TYPES.len(), 7, "Should have 7 rune types");
    }

    #[test]
    fn test_rune_type_elements() {
        let elements: Vec<&str> = RUNE_TYPES.iter().map(|r| r.element).collect();
        assert!(elements.contains(&"火"));
        assert!(elements.contains(&"水"));
        assert!(elements.contains(&"风"));
        assert!(elements.contains(&"地"));
        assert!(elements.contains(&"雷"));
        assert!(elements.contains(&"光"));
        assert!(elements.contains(&"暗"));
    }

    #[test]
    fn test_grade_levels() {
        assert_eq!(RuneGrade::Fragment.level(), 0);
        assert_eq!(RuneGrade::Basic.level(), 1);
        assert_eq!(RuneGrade::Advanced.level(), 2);
        assert_eq!(RuneGrade::Superior.level(), 3);
        assert_eq!(RuneGrade::Legendary.level(), 4);
        assert_eq!(RuneGrade::Mythic.level(), 5);
    }

    #[test]
    fn test_grade_from_level() {
        assert_eq!(RuneGrade::from_level(0), RuneGrade::Fragment);
        assert_eq!(RuneGrade::from_level(1), RuneGrade::Basic);
        assert_eq!(RuneGrade::from_level(3), RuneGrade::Superior);
        assert_eq!(RuneGrade::from_level(5), RuneGrade::Mythic);
        assert_eq!(RuneGrade::from_level(99), RuneGrade::Mythic); // overflow → mythic
    }

    #[test]
    fn test_combine_cost_escalation() {
        assert_eq!(RuneGrade::Fragment.combine_cost(), 3);
        assert_eq!(RuneGrade::Basic.combine_cost(), 5);
        assert_eq!(RuneGrade::Advanced.combine_cost(), 5);
        assert_eq!(RuneGrade::Superior.combine_cost(), 3);
        assert_eq!(RuneGrade::Legendary.combine_cost(), 3);
        assert_eq!(RuneGrade::Mythic.combine_cost(), 0);
    }

    #[test]
    fn test_grade_multiplier_escalation() {
        let m0 = grade_multiplier(&RuneGrade::Fragment);
        let m1 = grade_multiplier(&RuneGrade::Basic);
        let m2 = grade_multiplier(&RuneGrade::Advanced);
        let m3 = grade_multiplier(&RuneGrade::Superior);
        let m4 = grade_multiplier(&RuneGrade::Legendary);
        let m5 = grade_multiplier(&RuneGrade::Mythic);
        assert!(m0 < m1);
        assert!(m1 < m2);
        assert!(m2 < m3);
        assert!(m3 < m4);
        assert!(m4 < m5);
    }

    #[test]
    fn test_fragment_drops_basic() {
        let drops = get_fragment_drops(1);
        assert!(drops.contains(&"火"));
        assert!(drops.contains(&"水"));
        assert!(!drops.contains(&"雷")); // not at level 1
    }

    #[test]
    fn test_fragment_drops_high_level() {
        let drops = get_fragment_drops(50);
        assert!(drops.len() >= 6);
        assert!(drops.contains(&"光"));
        assert!(drops.contains(&"暗"));
    }

    #[test]
    fn test_inventory_serialize_roundtrip() {
        let entries = vec![
            RuneEntry {
                element: "火".to_string(),
                grade: 0,
                count: 5,
            },
            RuneEntry {
                element: "水".to_string(),
                grade: 1,
                count: 2,
            },
        ];
        let json = serialize_inventory(&entries);
        let parsed = parse_inventory(&json);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].element, "火");
        assert_eq!(parsed[0].count, 5);
        assert_eq!(parsed[1].element, "水");
        assert_eq!(parsed[1].grade, 1);
    }

    #[test]
    fn test_inscribed_serialize_roundtrip() {
        let entries = vec![
            InscribedEntry {
                slot: "武器".to_string(),
                element: "火".to_string(),
                grade: 3,
            },
            InscribedEntry {
                slot: "头盔".to_string(),
                element: "水".to_string(),
                grade: 2,
            },
        ];
        let json = serialize_inscribed(&entries);
        let parsed = parse_inscribed(&json);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].slot, "武器");
        assert_eq!(parsed[1].element, "水");
    }

    #[test]
    fn test_parse_empty_inventory() {
        let entries = parse_inventory("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_empty_inscribed() {
        let entries = parse_inscribed("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_set_bonus_count() {
        assert_eq!(RUNE_SET_BONUSES.len(), 7);
        let elements: Vec<&str> = RUNE_SET_BONUSES.iter().map(|s| s.element).collect();
        assert!(elements.contains(&"火"));
        assert!(elements.contains(&"暗"));
    }

    #[test]
    fn test_count_set_pieces() {
        let inscribed = vec![
            InscribedEntry {
                slot: "武器".to_string(),
                element: "火".to_string(),
                grade: 3,
            },
            InscribedEntry {
                slot: "头盔".to_string(),
                element: "火".to_string(),
                grade: 2,
            },
            InscribedEntry {
                slot: "胸甲".to_string(),
                element: "水".to_string(),
                grade: 1,
            },
        ];
        let counts = count_set_pieces(&inscribed);
        assert_eq!(counts.get("火"), Some(&2));
        assert_eq!(counts.get("水"), Some(&1));
    }

    #[test]
    fn test_rune_type_special_not_empty() {
        for rt in RUNE_TYPES {
            assert!(!rt.special.is_empty(), "Rune {} should have special effect", rt.name);
        }
    }

    #[test]
    fn test_rune_type_bonuses_non_negative() {
        for rt in RUNE_TYPES {
            assert!(rt.hp_bonus >= 0);
            assert!(rt.ad_bonus >= 0);
            assert!(rt.ap_bonus >= 0);
            assert!(rt.def_bonus >= 0);
            assert!(rt.mdef_bonus >= 0);
            assert!(rt.crit_bonus >= 0);
        }
    }
}

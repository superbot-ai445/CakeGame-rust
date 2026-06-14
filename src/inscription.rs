/// CakeGame 铭文系统
///
/// 玩家可在装备上铭刻神秘铭文，获得额外属性加成和特殊效果
/// 7种铭文类型 × 5级品质，铭文可叠加、合成、拆卸
/// 每件装备最多3个铭文槽位，铭文间可能产生共鸣效果
/// 数据存储: Global 表 SECTION='inscription'
use crate::core::*;
use crate::db::Database;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 铭文类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InscriptionType {
    /// 力量铭文 — 物攻加成
    Power,
    /// 智慧铭文 — 魔攻加成
    Wisdom,
    /// 守护铭文 — 防御加成
    Guardian,
    /// 生命铭文 — HP加成
    Vitality,
    /// 迅捷铭文 — 速度/暴击加成
    Swiftness,
    /// 幸运铭文 — 掉落/金币加成
    Fortune,
    /// 暗影铭文 — 特殊效果(吸血/穿透)
    Shadow,
}

impl InscriptionType {
    pub fn from_idx(idx: i32) -> Option<Self> {
        match idx {
            0 => Some(Self::Power),
            1 => Some(Self::Wisdom),
            2 => Some(Self::Guardian),
            3 => Some(Self::Vitality),
            4 => Some(Self::Swiftness),
            5 => Some(Self::Fortune),
            6 => Some(Self::Shadow),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Power => "力量铭文",
            Self::Wisdom => "智慧铭文",
            Self::Guardian => "守护铭文",
            Self::Vitality => "生命铭文",
            Self::Swiftness => "迅捷铭文",
            Self::Fortune => "幸运铭文",
            Self::Shadow => "暗影铭文",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Power => "⚔️",
            Self::Wisdom => "📖",
            Self::Guardian => "🛡️",
            Self::Vitality => "❤️",
            Self::Swiftness => "💨",
            Self::Fortune => "🍀",
            Self::Shadow => "🌑",
        }
    }

    pub fn stat_desc(&self) -> &'static str {
        match self {
            Self::Power => "物攻",
            Self::Wisdom => "魔攻",
            Self::Guardian => "防御",
            Self::Vitality => "HP",
            Self::Swiftness => "暴击率",
            Self::Fortune => "金币加成",
            Self::Shadow => "吸血率",
        }
    }
}

/// 铭文品质等级
pub const INSCRIPTION_GRADES: &[(&str, &str, i32)] = &[
    ("普通", "⬜", 1),
    ("精良", "🟢", 3),
    ("稀有", "🔵", 6),
    ("史诗", "🟣", 12),
    ("传说", "🟠", 20),
];

/// 装备槽位名称
pub const EQUIP_SLOTS: &[&str] = &["武器", "头盔", "胸甲", "手套", "鞋子", "饰品"];

/// 铭文合成配方: (等级) -> 需要同类型低级铭文数量
pub fn merge_cost(target_grade: usize) -> i32 {
    match target_grade {
        0 => 0,
        1 => 3,
        2 => 3,
        3 => 3,
        4 => 3,
        _ => 99,
    }
}

/// 计算铭文属性加成值
pub fn calc_inscription_bonus(insc_type: InscriptionType, grade: usize) -> i32 {
    let base = INSCRIPTION_GRADES.get(grade).map(|g| g.2).unwrap_or(1);
    match insc_type {
        InscriptionType::Power => base * 15,
        InscriptionType::Wisdom => base * 15,
        InscriptionType::Guardian => base * 10,
        InscriptionType::Vitality => base * 50,
        InscriptionType::Swiftness => base,
        InscriptionType::Fortune => base * 2,
        InscriptionType::Shadow => base,
    }
}

/// 铭文共鸣: 2个同类型铭文在同一装备上触发共鸣
pub fn resonance_bonus(insc_type: InscriptionType, count: usize) -> i32 {
    if count >= 3 {
        match insc_type {
            InscriptionType::Power => 50,
            InscriptionType::Wisdom => 50,
            InscriptionType::Guardian => 40,
            InscriptionType::Vitality => 200,
            InscriptionType::Swiftness => 5,
            InscriptionType::Fortune => 10,
            InscriptionType::Shadow => 5,
        }
    } else if count >= 2 {
        match insc_type {
            InscriptionType::Power => 20,
            InscriptionType::Wisdom => 20,
            InscriptionType::Guardian => 15,
            InscriptionType::Vitality => 80,
            InscriptionType::Swiftness => 2,
            InscriptionType::Fortune => 4,
            InscriptionType::Shadow => 2,
        }
    } else {
        0
    }
}

/// 铭文数据结构 (存储在 Global 表)
#[derive(Debug, Clone)]
pub struct InscriptionData {
    /// 用户铭文背包: "type_grade:count,..."
    pub inventory: String,
    /// 装备铭文: "slot:type_grade,type_grade,type_grade;..."
    pub equipped: String,
    /// 铭文碎片(用于合成)
    pub fragments: i32,
    /// 已铭刻总次数
    pub total_inscriptions: i32,
}

impl InscriptionData {
    pub fn from_global(data: &str) -> Self {
        let mut inv = String::new();
        let mut equipped = String::new();
        let mut fragments = 0i32;
        let mut total = 0i32;
        for part in data.split('|') {
            if let Some(v) = part.strip_prefix("inv=") {
                inv = v.to_string();
            } else if let Some(v) = part.strip_prefix("eq=") {
                equipped = v.to_string();
            } else if let Some(v) = part.strip_prefix("frag=") {
                fragments = v.parse().unwrap_or(0);
            } else if let Some(v) = part.strip_prefix("total=") {
                total = v.parse().unwrap_or(0);
            }
        }
        Self {
            inventory: inv,
            equipped,
            fragments,
            total_inscriptions: total,
        }
    }

    pub fn to_global(&self) -> String {
        format!(
            "inv={}|eq={}|frag={}|total={}",
            self.inventory, self.equipped, self.fragments, self.total_inscriptions
        )
    }

    /// 获取背包中某类型某品质铭文数量
    pub fn get_count(&self, insc_type: &InscriptionType, grade: usize) -> i32 {
        let key = format!("{}_{}", *insc_type as i32, grade);
        for entry in self.inventory.split(',') {
            if let Some((k, v)) = entry.split_once(':') {
                if k == key {
                    return v.parse().unwrap_or(0);
                }
            }
        }
        0
    }

    /// 设置背包中某类型某品质铭文数量
    pub fn set_count(&mut self, insc_type: &InscriptionType, grade: usize, count: i32) {
        let key = format!("{}_{}", *insc_type as i32, grade);
        let mut entries: Vec<(String, i32)> = Vec::new();
        let mut found = false;
        for entry in self.inventory.split(',') {
            if let Some((k, v)) = entry.split_once(':') {
                let val = v.parse().unwrap_or(0);
                if k == key {
                    entries.push((key.clone(), count));
                    found = true;
                } else if !k.is_empty() {
                    entries.push((k.to_string(), val));
                }
            }
        }
        if !found && count > 0 {
            entries.push((key, count));
        }
        self.inventory = entries
            .iter()
            .filter(|(_, v)| *v > 0)
            .map(|(k, v)| format!("{}:{}", k, v))
            .collect::<Vec<_>>()
            .join(",");
    }

    /// 获取某装备槽位已铭刻的铭文列表
    pub fn get_slot_inscriptions(&self, slot: usize) -> Vec<(InscriptionType, usize)> {
        let slots: Vec<&str> = self.equipped.split(';').collect();
        if let Some(slot_data) = slots.get(slot) {
            slot_data
                .split(',')
                .filter_map(|s| {
                    if let Some((t, g)) = s.split_once('_') {
                        let t_idx: i32 = t.parse().ok()?;
                        let g_idx: usize = g.parse().ok()?;
                        Some((InscriptionType::from_idx(t_idx)?, g_idx))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 设置某装备槽位的铭文
    pub fn set_slot_inscriptions(&mut self, slot: usize, inscs: &[(InscriptionType, usize)]) {
        let mut slots: Vec<String> = self.equipped.split(';').map(|s| s.to_string()).collect();
        // 确保槽位数组足够大
        while slots.len() <= slot {
            slots.push(String::new());
        }
        slots[slot] = inscs
            .iter()
            .map(|(t, g)| format!("{}_{}", *t as i32, g))
            .collect::<Vec<_>>()
            .join(",");
        self.equipped = slots.join(";");
    }
}

/// 生成确定性哈希种子
fn hash_seed(user_id: &str, extra: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{}{}", user_id, extra).hash(&mut hasher);
    hasher.finish()
}

/// 根据等级获取免费铭文品质
#[allow(dead_code)]
pub fn get_free_grade(user_level: i32) -> usize {
    if user_level >= 80 {
        2
    } else if user_level >= 50 {
        1
    } else {
        0
    }
}

/// 铭文消耗金币
pub fn inscribe_cost(grade: usize) -> i64 {
    match grade {
        0 => 500,
        1 => 2000,
        2 => 8000,
        3 => 30000,
        4 => 100000,
        _ => 500000,
    }
}

/// ==================== 指令实现 ====================
/// 查看铭文背包
pub fn cmd_view_inscription(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let uid = user_id.trim();
    let data_str = db.global_get("inscription", uid);
    if data_str.is_empty() {
        return "📝 你还没有任何铭文材料。\n使用「铭刻铭文」为装备添加铭文。".to_string();
    }
    let data = InscriptionData::from_global(&data_str);

    let mut result = String::from("══════ 铭文背包 ══════\n");
    result.push_str(&format!("💎 铭文碎片: {}\n", data.fragments));
    result.push_str(&format!("📊 已铭刻次数: {}\n\n", data.total_inscriptions));

    // 显示背包铭文
    result.push_str("📦 铭文材料:\n");
    let mut has_any = false;
    for i in 0..7 {
        if let Some(insc_type) = InscriptionType::from_idx(i) {
            for (g, &(grade_name, _grade_icon, _)) in INSCRIPTION_GRADES.iter().enumerate() {
                let count = data.get_count(&insc_type, g);
                if count > 0 {
                    result.push_str(&format!(
                        "  {} {}({}) × {}\n",
                        insc_type.icon(),
                        insc_type.name(),
                        grade_name,
                        count
                    ));
                    has_any = true;
                }
            }
        }
    }
    if !has_any {
        result.push_str("  (空)\n");
    }

    // 显示已装备铭文
    result.push_str("\n🛡️ 已铭刻装备:\n");
    for (slot_idx, slot_name) in EQUIP_SLOTS.iter().enumerate() {
        let inscs = data.get_slot_inscriptions(slot_idx);
        if !inscs.is_empty() {
            result.push_str(&format!("  [{}] ", slot_name));
            let descs: Vec<String> = inscs
                .iter()
                .map(|(t, g)| format!("{}{}{}", t.icon(), INSCRIPTION_GRADES[*g].1, t.name()))
                .collect();
            result.push_str(&descs.join(" + "));
            result.push('\n');
        }
    }

    result
}

/// 铭刻铭文 — 向装备添加铭文
pub fn cmd_inscribe(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let uid = user_id.trim();
    let parts: Vec<&str> = args.trim().split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return "📝 格式: 铭刻铭文+装备槽位+铭文类型\n示例: 铭刻铭文+武器+力量\n槽位: 武器/头盔/胸甲/手套/鞋子/饰品\n类型: 力量/智慧/守护/生命/迅捷/幸运/暗影".to_string();
    }

    // 解析装备槽位
    let slot_name = parts[0];
    let slot = EQUIP_SLOTS.iter().position(|&s| s == slot_name);
    let slot_idx = match slot {
        Some(s) => s,
        None => return format!("❌ 无效装备槽位「{}」\n可选: 武器/头盔/胸甲/手套/鞋子/饰品", slot_name),
    };

    // 解析铭文类型
    let type_name = parts[1];
    let insc_type = match type_name {
        "力量" => InscriptionType::Power,
        "智慧" => InscriptionType::Wisdom,
        "守护" => InscriptionType::Guardian,
        "生命" => InscriptionType::Vitality,
        "迅捷" => InscriptionType::Swiftness,
        "幸运" => InscriptionType::Fortune,
        "暗影" => InscriptionType::Shadow,
        _ => return "❌ 无效铭文类型\n可选: 力量/智慧/守护/生命/迅捷/幸运/暗影".to_string(),
    };

    // 读取铭文数据
    let data_str = db.global_get("inscription", uid);
    let mut data = if data_str.is_empty() {
        InscriptionData::from_global("")
    } else {
        InscriptionData::from_global(&data_str)
    };

    // 检查槽位是否已满(最多3个铭文)
    let slot_inscs = data.get_slot_inscriptions(slot_idx);
    if slot_inscs.len() >= 3 {
        return format!(
            "❌ {} 已铭刻3个铭文，槽位已满！\n使用「拆卸铭文+{}」移除已有铭文。",
            slot_name, slot_name
        );
    }

    // 获取用户等级确定最低品质

    // 查找背包中可用铭文(从最低品质开始)
    let mut used_grade: Option<usize> = None;
    for g in 0..5 {
        if data.get_count(&insc_type, g) > 0 {
            used_grade = Some(g);
            break;
        }
    }
    let grade = match used_grade {
        Some(g) => g,
        None => {
            return format!(
                "❌ 你没有 {}{}，请先通过采集、BOSS掉落或铭文商店获取。",
                insc_type.icon(),
                insc_type.name()
            )
        }
    };

    // 扣除铭文
    let cur = data.get_count(&insc_type, grade);
    data.set_count(&insc_type, grade, cur - 1);

    // 扣除金币
    let cost = inscribe_cost(grade);
    let gold = db.read_currency(uid, CURRENCY_GOLD);
    if gold < cost {
        // 返还铭文
        data.set_count(&insc_type, grade, cur);
        return format!(
            "❌ 金币不足！铭刻{}{}需要{}金币，当前仅有{}金币。",
            INSCRIPTION_GRADES[grade].1,
            insc_type.name(),
            cost,
            gold
        );
    }
    db.modify_currency(uid, CURRENCY_GOLD, OP_SUB, cost);

    // 铭刻到装备
    let mut new_slot = slot_inscs;
    new_slot.push((insc_type, grade));
    data.set_slot_inscriptions(slot_idx, &new_slot);
    data.total_inscriptions += 1;

    // 检查共鸣
    let type_count = new_slot.iter().filter(|(t, _)| *t == insc_type).count();
    let resonance = resonance_bonus(insc_type, type_count);

    // 保存
    db.global_set("inscription", uid, &data.to_global());

    let bonus = calc_inscription_bonus(insc_type, grade);
    let mut result = format!(
        "✅ 铭刻成功！\n\n{} {}{} → {}\n属性加成: +{} {}\n消耗: {}金币",
        insc_type.icon(),
        INSCRIPTION_GRADES[grade].1,
        insc_type.name(),
        slot_name,
        bonus,
        insc_type.stat_desc(),
        cost
    );

    if resonance > 0 {
        result.push_str(&format!(
            "\n\n🔔 {} {} 共鸣触发！额外 +{} {}",
            insc_type.icon(),
            insc_type.name(),
            resonance,
            insc_type.stat_desc()
        ));
    }

    result.push_str(&format!("\n\n💡 {} 上已有 {}/3 个铭文", slot_name, new_slot.len()));

    result
}

/// 拆卸铭文 — 从装备移除铭文
pub fn cmd_remove_inscription(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let uid = user_id.trim();
    let slot_name = args.trim();

    let slot = EQUIP_SLOTS.iter().position(|&s| s == slot_name);
    let slot_idx = match slot {
        Some(s) => s,
        None => return "❌ 格式: 拆卸铭文+装备槽位\n可选: 武器/头盔/胸甲/手套/鞋子/饰品".to_string(),
    };

    let data_str = db.global_get("inscription", uid);
    if data_str.is_empty() {
        return format!("❌ {} 上没有铭文可拆卸。", slot_name);
    }
    let mut data = InscriptionData::from_global(&data_str);

    let slot_inscs = data.get_slot_inscriptions(slot_idx);
    if slot_inscs.is_empty() {
        return format!("❌ {} 上没有铭文可拆卸。", slot_name);
    }

    // 拆卸所有铭文，返还背包(品质-1，最低0)
    let mut returned = Vec::new();
    for (insc_type, grade) in &slot_inscs {
        let return_grade = if *grade > 0 { grade - 1 } else { 0 };
        let cur = data.get_count(insc_type, return_grade);
        data.set_count(insc_type, return_grade, cur + 1);
        returned.push(format!(
            "{}{}{}",
            insc_type.icon(),
            INSCRIPTION_GRADES[*grade].1,
            insc_type.name()
        ));
    }

    data.set_slot_inscriptions(slot_idx, &[]);
    db.global_set("inscription", uid, &data.to_global());

    format!(
        "✅ 拆卸成功！\n\n从 {} 移除了 {} 个铭文:\n{}\n\n⚠️ 拆卸后铭文品质降低1级",
        slot_name,
        returned.len(),
        returned.join(" / ")
    )
}

/// 铭文合成 — 将低级铭文合成高级铭文
pub fn cmd_merge_inscription(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let uid = user_id.trim();
    let parts: Vec<&str> = args.trim().split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return "📝 格式: 铭文合成+铭文类型\n示例: 铭文合成+力量\n自动将背包中同品质铭文3合1为更高品质。".to_string();
    }

    let type_name = parts[0];
    let insc_type = match type_name {
        "力量" => InscriptionType::Power,
        "智慧" => InscriptionType::Wisdom,
        "守护" => InscriptionType::Guardian,
        "生命" => InscriptionType::Vitality,
        "迅捷" => InscriptionType::Swiftness,
        "幸运" => InscriptionType::Fortune,
        "暗影" => InscriptionType::Shadow,
        _ => return "❌ 无效铭文类型\n可选: 力量/智慧/守护/生命/迅捷/幸运/暗影".to_string(),
    };

    let data_str = db.global_get("inscription", uid);
    if data_str.is_empty() {
        return format!("❌ 你没有任何 {} 可合成。", insc_type.name());
    }
    let mut data = InscriptionData::from_global(&data_str);

    // 逐级合成
    let mut merged = Vec::new();
    for g in 0..4 {
        // 最高只能合成到传说(4)
        let count = data.get_count(&insc_type, g);
        let need = merge_cost(g + 1);
        if count >= need {
            let num_merges = count / need;
            let used = num_merges * need;
            data.set_count(&insc_type, g, count - used);
            let cur_higher = data.get_count(&insc_type, g + 1);
            data.set_count(&insc_type, g + 1, cur_higher + num_merges);
            merged.push(format!(
                "{} {}({}) × {} → {}({}) × {}",
                insc_type.icon(),
                insc_type.name(),
                INSCRIPTION_GRADES[g].0,
                used,
                INSCRIPTION_GRADES[g + 1].1,
                INSCRIPTION_GRADES[g + 1].0,
                num_merges
            ));
        }
    }

    if merged.is_empty() {
        return format!("❌ {} 各品质铭文不足3个，无法合成。", insc_type.name());
    }

    db.global_set("inscription", uid, &data.to_global());

    format!("✅ 铭文合成成功！\n\n{}", merged.join("\n"))
}

/// 铭文商店 — 购买铭文
pub fn cmd_inscription_shop(_db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let _uid = user_id.trim();

    let mut result = String::from("══════ 铭文商店 ══════\n\n");

    for i in 0..7 {
        if let Some(insc_type) = InscriptionType::from_idx(i) {
            let price = match insc_type {
                InscriptionType::Power => 3000,
                InscriptionType::Wisdom => 3000,
                InscriptionType::Guardian => 2500,
                InscriptionType::Vitality => 2000,
                InscriptionType::Swiftness => 4000,
                InscriptionType::Fortune => 3500,
                InscriptionType::Shadow => 5000,
            };
            result.push_str(&format!(
                "{} {} — {}金币 (物品+{})\n",
                insc_type.icon(),
                insc_type.name(),
                price,
                insc_type.stat_desc()
            ));
        }
    }

    result.push_str("\n💡 格式: 购买铭文+铭文类型+数量\n示例: 购买铭文+力量+5\n");
    result.push_str("💡 也可通过击败BOSS、采集、深渊等获得铭文碎片。");

    result
}

/// 购买铭文
pub fn cmd_buy_inscription(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let uid = user_id.trim();
    let parts: Vec<&str> = args.trim().split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return "📝 格式: 购买铭文+铭文类型+数量\n示例: 购买铭文+力量+5".to_string();
    }

    let type_name = parts[0];
    let insc_type = match type_name {
        "力量" => InscriptionType::Power,
        "智慧" => InscriptionType::Wisdom,
        "守护" => InscriptionType::Guardian,
        "生命" => InscriptionType::Vitality,
        "迅捷" => InscriptionType::Swiftness,
        "幸运" => InscriptionType::Fortune,
        "暗影" => InscriptionType::Shadow,
        _ => return "❌ 无效铭文类型".to_string(),
    };

    let qty: i32 = if parts.len() > 1 {
        parts[1].parse().unwrap_or(1).clamp(1, 99)
    } else {
        1
    };

    let price_each: i64 = match insc_type {
        InscriptionType::Power => 3000,
        InscriptionType::Wisdom => 3000,
        InscriptionType::Guardian => 2500,
        InscriptionType::Vitality => 2000,
        InscriptionType::Swiftness => 4000,
        InscriptionType::Fortune => 3500,
        InscriptionType::Shadow => 5000,
    };
    let total_cost = price_each * qty as i64;

    let gold = db.read_currency(uid, CURRENCY_GOLD);
    if gold < total_cost {
        return format!("❌ 金币不足！需要{}金币，当前仅有{}金币。", total_cost, gold);
    }

    db.modify_currency(uid, CURRENCY_GOLD, OP_SUB, total_cost);

    let data_str = db.global_get("inscription", uid);
    let mut data = if data_str.is_empty() {
        InscriptionData::from_global("")
    } else {
        InscriptionData::from_global(&data_str)
    };
    let cur = data.get_count(&insc_type, 0);
    data.set_count(&insc_type, 0, cur + qty);
    db.global_set("inscription", uid, &data.to_global());

    format!(
        "✅ 购买成功！\n\n{} {} × {}\n消耗: {}金币\n剩余: {}金币",
        insc_type.icon(),
        insc_type.name(),
        qty,
        total_cost,
        gold - total_cost
    )
}

/// 铭文详情 — 查看装备上的铭文属性
pub fn cmd_inscription_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let uid = user_id.trim();
    let slot_name = args.trim();

    let slot = EQUIP_SLOTS.iter().position(|&s| s == slot_name);
    let slot_idx = match slot {
        Some(s) => s,
        None => return "📝 格式: 铭文详情+装备槽位\n可选: 武器/头盔/胸甲/手套/鞋子/饰品".to_string(),
    };

    let data_str = db.global_get("inscription", uid);
    if data_str.is_empty() {
        return format!("❌ {} 上没有铭文。", slot_name);
    }
    let data = InscriptionData::from_global(&data_str);
    let slot_inscs = data.get_slot_inscriptions(slot_idx);

    if slot_inscs.is_empty() {
        return format!("❌ {} 上没有铭文。", slot_name);
    }

    let mut result = format!("══════ {} 铭文详情 ══════\n\n", slot_name);
    let mut total_power = 0i32;
    let mut total_wisdom = 0i32;
    let mut total_guardian = 0i32;
    let mut total_vitality = 0i32;
    let mut total_swiftness = 0i32;
    let mut total_fortune = 0i32;
    let mut total_shadow = 0i32;

    // 统计各类型数量用于共鸣计算
    let mut type_counts = [0i32; 7];

    for (idx, (insc_type, grade)) in slot_inscs.iter().enumerate() {
        let bonus = calc_inscription_bonus(*insc_type, *grade);
        result.push_str(&format!(
            "  {}. {} {}{} — +{} {}\n",
            idx + 1,
            insc_type.icon(),
            INSCRIPTION_GRADES[*grade].1,
            insc_type.name(),
            bonus,
            insc_type.stat_desc()
        ));
        type_counts[*insc_type as usize] += 1;

        match insc_type {
            InscriptionType::Power => total_power += bonus,
            InscriptionType::Wisdom => total_wisdom += bonus,
            InscriptionType::Guardian => total_guardian += bonus,
            InscriptionType::Vitality => total_vitality += bonus,
            InscriptionType::Swiftness => total_swiftness += bonus,
            InscriptionType::Fortune => total_fortune += bonus,
            InscriptionType::Shadow => total_shadow += bonus,
        }
    }

    // 计算共鸣加成
    for (i, &tc) in type_counts.iter().enumerate() {
        if let Some(insc_type) = InscriptionType::from_idx(i as i32) {
            let res = resonance_bonus(insc_type, tc as usize);
            if res > 0 {
                result.push_str(&format!(
                    "\n🔔 {} {}共鸣: +{} {}",
                    insc_type.icon(),
                    insc_type.name(),
                    res,
                    insc_type.stat_desc()
                ));
                match insc_type {
                    InscriptionType::Power => total_power += res,
                    InscriptionType::Wisdom => total_wisdom += res,
                    InscriptionType::Guardian => total_guardian += res,
                    InscriptionType::Vitality => total_vitality += res,
                    InscriptionType::Swiftness => total_swiftness += res,
                    InscriptionType::Fortune => total_fortune += res,
                    InscriptionType::Shadow => total_shadow += res,
                }
            }
        }
    }

    result.push_str("\n\n📊 属性加成汇总:\n");
    if total_power > 0 {
        result.push_str(&format!("  ⚔️ 物攻 +{}\n", total_power));
    }
    if total_wisdom > 0 {
        result.push_str(&format!("  📖 魔攻 +{}\n", total_wisdom));
    }
    if total_guardian > 0 {
        result.push_str(&format!("  🛡️ 防御 +{}\n", total_guardian));
    }
    if total_vitality > 0 {
        result.push_str(&format!("  ❤️ HP +{}\n", total_vitality));
    }
    if total_swiftness > 0 {
        result.push_str(&format!("  💨 暴击率 +{}%\n", total_swiftness));
    }
    if total_fortune > 0 {
        result.push_str(&format!("  🍀 金币加成 +{}%\n", total_fortune));
    }
    if total_shadow > 0 {
        result.push_str(&format!("  🌑 吸血率 +{}%\n", total_shadow));
    }

    result
}

/// 铭文排行 — 全服铭文战力排行
pub fn cmd_inscription_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let users = db.all_users();
    let mut scores: Vec<(String, i32, i32)> = Vec::new(); // (uid, total_score, inscription_count)

    for uid in &users {
        let data_str = db.global_get("inscription", uid);
        if data_str.is_empty() {
            continue;
        }
        let data = InscriptionData::from_global(&data_str);
        let mut total_score = 0i32;
        let mut count = 0i32;

        for slot_idx in 0..6 {
            let inscs = data.get_slot_inscriptions(slot_idx);
            for (insc_type, grade) in &inscs {
                total_score += calc_inscription_bonus(*insc_type, *grade);
                count += 1;
            }
            // 共鸣加分
            let mut type_counts = [0i32; 7];
            for (t, _) in &inscs {
                type_counts[*t as usize] += 1;
            }
            for (i, &tc) in type_counts.iter().enumerate() {
                if let Some(it) = InscriptionType::from_idx(i as i32) {
                    total_score += resonance_bonus(it, tc as usize);
                }
            }
        }

        if count > 0 {
            scores.push((uid.clone(), total_score, count));
        }
    }

    scores.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut result = String::from("══════ 铭文排行 ══════\n\n");
    let medals = ["🥇", "🥈", "🥉"];

    for (idx, (uid, score, count)) in scores.iter().take(15).enumerate() {
        let medal = if idx < 3 { medals[idx] } else { "  " };
        let name = db.read_basic(uid, "Nickname");
        let display_name = if name == "[NULL]" || name.is_empty() {
            uid.clone()
        } else {
            name
        };
        result.push_str(&format!(
            "{} {}. {} — 铭文战力: {} (铭文数: {})\n",
            medal,
            idx + 1,
            display_name,
            score,
            count
        ));
    }

    if scores.is_empty() {
        result.push_str("暂无铭文排行数据。\n");
    }

    result
}

/// 铭文帮助
pub fn cmd_inscription_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut result = String::from("══════ 铭文系统帮助 ══════\n\n");
    result.push_str("📝 铭文系统让你在装备上铭刻神秘文字，获得额外属性加成！\n\n");
    result.push_str("📌 基础指令:\n");
    result.push_str("  • 查看铭文 — 查看铭文背包和已装备铭文\n");
    result.push_str("  • 铭刻铭文+槽位+类型 — 在装备上铭刻铭文\n");
    result.push_str("  • 拆卸铭文+槽位 — 移除装备上的铭文\n");
    result.push_str("  • 铭文合成+类型 — 3个同品质合成更高品质\n");
    result.push_str("  • 铭文商店 — 查看可购买的铭文\n");
    result.push_str("  • 购买铭文+类型+数量 — 购买铭文\n");
    result.push_str("  • 铭文详情+槽位 — 查看装备铭文详细属性\n");
    result.push_str("  • 铭文排行 — 全服铭文战力排行\n\n");
    result.push_str("📌 铭文类型:\n");
    for i in 0..7 {
        if let Some(insc_type) = InscriptionType::from_idx(i) {
            result.push_str(&format!(
                "  {} {} — +{}\n",
                insc_type.icon(),
                insc_type.name(),
                insc_type.stat_desc()
            ));
        }
    }
    result.push_str("\n📌 品质等级:\n");
    for (name, icon, _) in INSCRIPTION_GRADES {
        result.push_str(&format!("  {} {}\n", icon, name));
    }
    result.push_str("\n📌 核心机制:\n");
    result.push_str("  • 每件装备最多3个铭文槽位\n");
    result.push_str("  • 同装备同类型2个铭文触发共鸣，3个触发高级共鸣\n");
    result.push_str("  • 合成需要3个同品质铭文\n");
    result.push_str("  • 拆卸会降低铭文品质1级\n");
    result.push_str("  • 铭文碎片可通过BOSS/采集/深渊获得\n");
    result
}

/// 获取铭文属性加成(供战斗系统调用)
#[allow(dead_code)]
pub fn get_inscription_bonus(db: &Database, user_id: &str) -> (i32, i32, i32, i32, i32, i32, i32) {
    // (power, wisdom, guardian, vitality, swiftness, fortune, shadow)
    let data_str = db.global_get("inscription", user_id);
    if data_str.is_empty() {
        return (0, 0, 0, 0, 0, 0, 0);
    }
    let data = InscriptionData::from_global(&data_str);
    let mut totals = [0i32; 7];

    for slot_idx in 0..6 {
        let inscs = data.get_slot_inscriptions(slot_idx);
        let mut type_counts = [0i32; 7];

        for (insc_type, grade) in &inscs {
            let bonus = calc_inscription_bonus(*insc_type, *grade);
            totals[*insc_type as usize] += bonus;
            type_counts[*insc_type as usize] += 1;
        }

        // 共鸣加成
        for i in 0..7 {
            if let Some(it) = InscriptionType::from_idx(i as i32) {
                totals[i] += resonance_bonus(it, type_counts[i] as usize);
            }
        }
    }

    (
        totals[0], totals[1], totals[2], totals[3], totals[4], totals[5], totals[6],
    )
}

/// 掉落铭文碎片(供怪物掉落调用)
#[allow(dead_code)]
pub fn drop_inscription_fragment(db: &Database, user_id: &str, monster_level: i32) -> Option<String> {
    // 20%概率掉落铭文碎片
    let hash = hash_seed(user_id, &format!("frag_{}", monster_level));
    if hash % 100 >= 20 {
        return None;
    }

    let fragment_count = 1 + (monster_level / 20).min(5);
    let data_str = db.global_get("inscription", user_id);
    let mut data = if data_str.is_empty() {
        InscriptionData::from_global("")
    } else {
        InscriptionData::from_global(&data_str)
    };
    data.fragments += fragment_count;
    db.global_set("inscription", user_id, &data.to_global());

    Some(format!("💎 获得 {} 个铭文碎片！", fragment_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inscription_type_from_idx() {
        assert!(InscriptionType::from_idx(0).is_some());
        assert!(InscriptionType::from_idx(6).is_some());
        assert!(InscriptionType::from_idx(7).is_none());
        assert!(InscriptionType::from_idx(-1).is_none());
    }

    #[test]
    fn test_inscription_type_names() {
        assert_eq!(InscriptionType::Power.name(), "力量铭文");
        assert_eq!(InscriptionType::Shadow.name(), "暗影铭文");
        assert_eq!(InscriptionType::Fortune.name(), "幸运铭文");
    }

    #[test]
    fn test_inscription_type_icons() {
        assert_eq!(InscriptionType::Power.icon(), "⚔️");
        assert_eq!(InscriptionType::Wisdom.icon(), "📖");
        assert_eq!(InscriptionType::Vitality.icon(), "❤️");
    }

    #[test]
    fn test_grade_definitions() {
        assert_eq!(INSCRIPTION_GRADES.len(), 5);
        assert_eq!(INSCRIPTION_GRADES[0].0, "普通");
        assert_eq!(INSCRIPTION_GRADES[4].0, "传说");
        assert!(INSCRIPTION_GRADES[4].2 > INSCRIPTION_GRADES[0].2);
    }

    #[test]
    fn test_calc_inscription_bonus_escalates() {
        for g in 0..4 {
            let low = calc_inscription_bonus(InscriptionType::Power, g);
            let high = calc_inscription_bonus(InscriptionType::Power, g + 1);
            assert!(
                high > low,
                "Grade {} bonus {} should be > grade {} bonus {}",
                g + 1,
                high,
                g,
                low
            );
        }
    }

    #[test]
    fn test_calc_inscription_bonus_all_types() {
        for i in 0..7 {
            let t = InscriptionType::from_idx(i).unwrap();
            let bonus = calc_inscription_bonus(t, 2);
            assert!(bonus > 0, "Type {} should have positive bonus", t.name());
        }
    }

    #[test]
    fn test_resonance_bonus_none() {
        assert_eq!(resonance_bonus(InscriptionType::Power, 0), 0);
        assert_eq!(resonance_bonus(InscriptionType::Power, 1), 0);
    }

    #[test]
    fn test_resonance_bonus_pair() {
        assert!(resonance_bonus(InscriptionType::Power, 2) > 0);
        assert!(resonance_bonus(InscriptionType::Vitality, 2) > 0);
    }

    #[test]
    fn test_resonance_bonus_triple() {
        let pair = resonance_bonus(InscriptionType::Power, 2);
        let triple = resonance_bonus(InscriptionType::Power, 3);
        assert!(triple > pair, "Triple resonance {} should be > pair {}", triple, pair);
    }

    #[test]
    fn test_merge_cost() {
        assert_eq!(merge_cost(0), 0);
        assert_eq!(merge_cost(1), 3);
        assert_eq!(merge_cost(2), 3);
        assert_eq!(merge_cost(3), 3);
        assert_eq!(merge_cost(4), 3);
    }

    #[test]
    fn test_inscribe_cost_escalates() {
        for g in 0..4 {
            assert!(
                inscribe_cost(g + 1) > inscribe_cost(g),
                "Cost at grade {} should exceed grade {}",
                g + 1,
                g
            );
        }
    }

    #[test]
    fn test_get_free_grade() {
        assert_eq!(get_free_grade(1), 0);
        assert_eq!(get_free_grade(49), 0);
        assert_eq!(get_free_grade(50), 1);
        assert_eq!(get_free_grade(79), 1);
        assert_eq!(get_free_grade(80), 2);
        assert_eq!(get_free_grade(100), 2);
    }

    #[test]
    fn test_inscription_data_roundtrip() {
        let data = InscriptionData {
            inventory: "0_0:3,1_1:2".to_string(),
            equipped: "0_1,0_2;;".to_string(),
            fragments: 50,
            total_inscriptions: 10,
        };
        let serialized = data.to_global();
        let restored = InscriptionData::from_global(&serialized);
        assert_eq!(restored.inventory, "0_0:3,1_1:2");
        assert_eq!(restored.equipped, "0_1,0_2;;");
        assert_eq!(restored.fragments, 50);
        assert_eq!(restored.total_inscriptions, 10);
    }

    #[test]
    fn test_inscription_data_get_set_count() {
        let mut data = InscriptionData::from_global("");
        assert_eq!(data.get_count(&InscriptionType::Power, 0), 0);
        data.set_count(&InscriptionType::Power, 0, 5);
        assert_eq!(data.get_count(&InscriptionType::Power, 0), 5);
        data.set_count(&InscriptionType::Power, 0, 3);
        assert_eq!(data.get_count(&InscriptionType::Power, 0), 3);
        data.set_count(&InscriptionType::Power, 0, 0);
        assert_eq!(data.get_count(&InscriptionType::Power, 0), 0);
    }

    #[test]
    fn test_slot_inscriptions_roundtrip() {
        let mut data = InscriptionData::from_global("");
        let inscs = vec![(InscriptionType::Power, 0usize), (InscriptionType::Guardian, 1usize)];
        data.set_slot_inscriptions(0, &inscs);
        let restored = data.get_slot_inscriptions(0);
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].0, InscriptionType::Power);
        assert_eq!(restored[1].1, 1usize);
    }

    #[test]
    fn test_empty_slot_inscriptions() {
        let data = InscriptionData::from_global("");
        assert!(data.get_slot_inscriptions(0).is_empty());
        assert!(data.get_slot_inscriptions(5).is_empty());
    }

    #[test]
    fn test_equip_slots_count() {
        assert_eq!(EQUIP_SLOTS.len(), 6);
    }

    #[test]
    fn test_hash_seed_deterministic() {
        let a = hash_seed("user1", "frag_10");
        let b = hash_seed("user1", "frag_10");
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_seed_varies() {
        let a = hash_seed("user1", "frag_10");
        let b = hash_seed("user2", "frag_10");
        // Very unlikely to be equal with different inputs
        // but we can at least verify they produce values
        assert_ne!(a, b);
    }
}

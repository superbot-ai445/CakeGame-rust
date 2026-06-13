/// CakeGame 怪物图鉴系统
/// 来自 Config_Monster 表 — 所有怪物信息
use crate::db::Database;
use crate::user;

/// 怪物简要信息
struct MonsterInfo {
    name: String,
    monster_type: String,
    ad: String,
    ap: String,
    hp: String,
    defense: String,
    magic_resistance: String,
    reward_exp: String,
    reward_gold: String,
    introduce: String,
    skills: String,
    reward_goods: String,
    absorb_hp: String,
    hit: String,
    dodge: String,
    adptv: String,
    adptr: String,
    apptv: String,
    apptr: String,
    immune_damage: String,
    ignore_shield: String,
}

/// 查询所有怪物
fn get_all_monsters(db: &Database) -> Vec<MonsterInfo> {
    db.query_rows(
        "SELECT Monster_Name, Monster_Type, Monster_AD, Monster_AP, Monster_HP, \
         Monster_Defense, MagicResistance, Reward_Exp, Reward_Gold, Introduce, \
         Skills, Reward_Goods, Monster_AbsorbHP, Hit, Dodge, \
         Monster_ADPTV, Monster_ADPTR, Monster_APPTV, Monster_APPTR, \
         Monster_ImmuneDamage, IgnoreShield \
         FROM Config_Monster ORDER BY Monster_Name",
        &[],
        |row| {
            Ok(MonsterInfo {
                name: row.get::<_, String>(0).unwrap_or_default(),
                monster_type: row.get::<_, String>(1).unwrap_or_default(),
                ad: row.get::<_, String>(2).unwrap_or_default(),
                ap: row.get::<_, String>(3).unwrap_or_default(),
                hp: row.get::<_, String>(4).unwrap_or_default(),
                defense: row.get::<_, String>(5).unwrap_or_default(),
                magic_resistance: row.get::<_, String>(6).unwrap_or_default(),
                reward_exp: row.get::<_, String>(7).unwrap_or_default(),
                reward_gold: row.get::<_, String>(8).unwrap_or_default(),
                introduce: row.get::<_, String>(9).unwrap_or_default(),
                skills: row.get::<_, String>(10).unwrap_or_default(),
                reward_goods: row.get::<_, String>(11).unwrap_or_default(),
                absorb_hp: row.get::<_, String>(12).unwrap_or_default(),
                hit: row.get::<_, String>(13).unwrap_or_default(),
                dodge: row.get::<_, String>(14).unwrap_or_default(),
                adptv: row.get::<_, String>(15).unwrap_or_default(),
                adptr: row.get::<_, String>(16).unwrap_or_default(),
                apptv: row.get::<_, String>(17).unwrap_or_default(),
                apptr: row.get::<_, String>(18).unwrap_or_default(),
                immune_damage: row.get::<_, String>(19).unwrap_or_default(),
                ignore_shield: row.get::<_, String>(20).unwrap_or_default(),
            })
        },
    )
}

/// 怪物图鉴 — 查看怪物列表或详情
pub fn cmd_bestiary(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let monsters = get_all_monsters(db);

    if monsters.is_empty() {
        return format!("{}\n暂无怪物信息。", prefix);
    }

    let args = args.trim();

    // 如果指定了怪物名，显示详情
    if !args.is_empty() {
        for m in &monsters {
            if m.name == args || m.name.contains(args) {
                return format_monster_detail(&prefix, m);
            }
        }
        return format!(
            "{}\n未找到名为「{}」的怪物。\n发送「怪物图鉴」查看所有怪物列表。",
            prefix, args
        );
    }

    // 列表模式 — 按类型分组
    let mut r = format!("{}\n═══ 怪物图鉴 ═══\n", prefix);

    let mut ordinary = Vec::new();
    let mut boss = Vec::new();
    let mut other = Vec::new();

    for m in &monsters {
        match m.monster_type.as_str() {
            "Ordinary" => ordinary.push(m),
            "Boss" => boss.push(m),
            _ => other.push(m),
        }
    }

    if !ordinary.is_empty() {
        r.push_str("\n📋 普通怪物:\n");
        for (i, m) in ordinary.iter().enumerate() {
            r.push_str(&format!(
                "{}. {} | HP:{} ATK:{} DEF:{}\n",
                i + 1,
                m.name,
                parse_attr(&m.hp),
                parse_attr(&m.ad),
                parse_attr(&m.defense)
            ));
        }
    }

    if !boss.is_empty() {
        r.push_str("\n👹 BOSS怪物:\n");
        for (i, m) in boss.iter().enumerate() {
            r.push_str(&format!(
                "{}. {} | HP:{} ATK:{} DEF:{}\n",
                i + 1,
                m.name,
                parse_attr(&m.hp),
                parse_attr(&m.ad),
                parse_attr(&m.defense)
            ));
        }
    }

    if !other.is_empty() {
        r.push_str("\n❓ 其他类型:\n");
        for (i, m) in other.iter().enumerate() {
            r.push_str(&format!(
                "{}. {} [{}] | HP:{}\n",
                i + 1,
                m.name,
                m.monster_type,
                parse_attr(&m.hp)
            ));
        }
    }

    r.push_str(&format!(
        "\n共 {} 种怪物\n发送「怪物图鉴+怪物名」查看详细信息",
        monsters.len()
    ));

    r
}

/// 格式化怪物详情
fn format_monster_detail(prefix: &str, m: &MonsterInfo) -> String {
    let mut r = format!("{}\n═══ 怪物图鉴: {} ═══", prefix, m.name);

    r.push_str(&format!("\n📖 {}", m.introduce));
    r.push_str(&format!("\n类型: {}", type_label(&m.monster_type)));
    r.push_str("\n\n── 基础属性 ──");
    r.push_str(&format!("\n❤️ 生命: {}", parse_attr(&m.hp)));
    r.push_str(&format!("\n⚔️ 物攻: {}", parse_attr(&m.ad)));
    r.push_str(&format!("\n🔮 魔攻: {}", parse_attr(&m.ap)));
    r.push_str(&format!("\n🛡️ 物防: {}", parse_attr(&m.defense)));
    r.push_str(&format!("\n🌀 魔抗: {}", parse_attr(&m.magic_resistance)));
    r.push_str(&format!("\n🎯 命中: {}", parse_attr(&m.hit)));
    r.push_str(&format!("\n💨 闪避: {}", parse_attr(&m.dodge)));

    // 特殊属性（非零才显示）
    let absorb = parse_attr(&m.absorb_hp);
    if absorb != "0" {
        r.push_str(&format!("\n🧛 吸血: {}", absorb));
    }
    let adptv = parse_attr(&m.adptv);
    if adptv != "0" {
        r.push_str(&format!("\n🗡️ 物穿值: {}", adptv));
    }
    let adptr = parse_attr(&m.adptr);
    if adptr != "0" {
        r.push_str(&format!("\n🗡️ 物穿比: {}%", adptr));
    }
    let apptv = parse_attr(&m.apptv);
    if apptv != "0" {
        r.push_str(&format!("\n✨ 法穿值: {}", apptv));
    }
    let apptr = parse_attr(&m.apptr);
    if apptr != "0" {
        r.push_str(&format!("\n✨ 法穿比: {}%", apptr));
    }
    let immune = parse_attr(&m.immune_damage);
    if immune != "0" {
        r.push_str(&format!("\n🔰 免伤: {}%", immune));
    }
    if m.ignore_shield == "TRUE" {
        r.push_str("\n💢 无视护盾");
    }

    // 技能
    let skills_str = m.skills.trim();
    if !skills_str.is_empty() && skills_str != "[NULL]" {
        r.push_str("\n\n── 技能 ──");
        // Skills format: {"技能名":"概率","技能名":"概率"}
        let cleaned = skills_str.trim_matches('{').trim_matches('}');
        for pair in cleaned.split(',') {
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().trim_matches('"');
                let prob = parts[1].trim().trim_matches('"');
                r.push_str(&format!("\n  ⚡ {} — {}%触发", name, prob));
            }
        }
    }

    // 掉落
    let goods_str = m.reward_goods.trim();
    if !goods_str.is_empty() && goods_str != "[NULL]" {
        r.push_str("\n\n── 掉落物品 ──");
        for entry in goods_str.split(',') {
            let parts: Vec<&str> = entry.trim().rsplitn(2, '*').collect();
            if parts.len() == 2 {
                let prob: f64 = parts[0].trim().parse().unwrap_or(0.0);
                r.push_str(&format!("\n  📦 {} — {:.1}%", parts[1].trim(), prob * 100.0));
            } else {
                r.push_str(&format!("\n  📦 {}", entry.trim()));
            }
        }
    }

    // 奖励
    r.push_str("\n\n── 击杀奖励 ──");
    let exp = parse_attr(&m.reward_exp);
    let gold = parse_attr(&m.reward_gold);
    r.push_str(&format!("\n✨ 经验: {}  💰 金币: {}", exp, gold));

    r.push_str("\n\n发送「怪物图鉴」返回列表");
    r
}

/// 解析属性值（可能是 "30" 或公式）
fn parse_attr(val: &str) -> String {
    let val = val.trim();
    if val.is_empty() || val == "[NULL]" {
        return "0".to_string();
    }
    // 尝试解析为数字
    if let Ok(n) = val.parse::<f64>() {
        if n == n.floor() {
            return format!("{}", n as i64);
        }
        return format!("{:.1}", n);
    }
    // 如果是公式，返回原始值
    val.to_string()
}

/// 类型标签
fn type_label(t: &str) -> &str {
    match t {
        "Ordinary" => "普通怪物",
        "Boss" => "BOSS",
        "Elite" => "精英",
        _ => t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attr_integer() {
        assert_eq!(parse_attr("30"), "30");
        assert_eq!(parse_attr("0"), "0");
        assert_eq!(parse_attr("999"), "999");
    }

    #[test]
    fn test_parse_attr_float() {
        assert_eq!(parse_attr("3.5"), "3.5");
        assert_eq!(parse_attr("100.0"), "100");
    }

    #[test]
    fn test_parse_attr_empty_or_null() {
        assert_eq!(parse_attr(""), "0");
        assert_eq!(parse_attr("  "), "0");
        assert_eq!(parse_attr("[NULL]"), "0");
    }

    #[test]
    fn test_parse_attr_formula() {
        // Non-numeric values returned as-is
        assert_eq!(parse_attr("等级*10"), "等级*10");
        assert_eq!(parse_attr("HP*2+50"), "HP*2+50");
    }

    #[test]
    fn test_type_label() {
        assert_eq!(type_label("Ordinary"), "普通怪物");
        assert_eq!(type_label("Boss"), "BOSS");
        assert_eq!(type_label("Elite"), "精英");
        assert_eq!(type_label("Unknown"), "Unknown");
    }
}

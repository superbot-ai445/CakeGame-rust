/// CakeGame 幻域锻造系统
///
/// 玩家通过战斗收集幻域精华，锻造幻域武器和装备
/// 8种幻域元素 × 5级锻造等级，幻域武器拥有独特技能
/// 幻域副本：解锁幻域区域进行专属挑战
/// 数据存储: Global 表 SECTION='phantom_forge'
use crate::combat_power;
use crate::core::*;
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 幻域元素类型
const ELEMENTS: &[(&str, &str, i32, f64, &str, f64, f64)] = &[
    ("炎狱", "🔥", 1, 0.25, "焚天烈焰", 0.15, 1.5),
    ("冰渊", "❄️", 10, 0.22, "极寒领域", 0.12, 1.2),
    ("雷鸣", "⚡", 20, 0.18, "万雷天罚", 0.18, 2.0),
    ("岩崩", "🪨", 30, 0.15, "山崩地裂", 0.10, 1.8),
    ("风刃", "🌪️", 40, 0.12, "千风之刃", 0.20, 1.3),
    ("暗蚀", "🌑", 50, 0.10, "暗影吞噬", 0.12, 1.6),
    ("圣光", "✨", 60, 0.08, "圣光审判", 0.10, 1.4),
    ("混沌", "🌀", 80, 0.05, "混沌风暴", 0.08, 2.5),
];

/// 锻造等级
const FORGE_LEVELS: &[(&str, &str, i32, i64, f64)] = &[
    ("凡铁", "⬜", 10, 5000, 1.0),
    ("秘银", "🟢", 30, 20000, 1.8),
    ("星陨", "🔵", 80, 80000, 3.0),
    ("龙魂", "🟣", 200, 300000, 5.0),
    ("幻域", "🟠", 500, 1000000, 8.0),
];

/// 幻域副本
const ZONES: &[(&str, i32, i64, i32, i32, i64, i64)] = &[
    ("炎狱试炼场", 15, 500, 3, 5, 3000, 10),
    ("冰渊深渊", 25, 1500, 5, 12, 8000, 25),
    ("雷鸣之巅", 35, 3000, 7, 25, 20000, 50),
    ("岩崩矿脉", 45, 5000, 8, 40, 40000, 80),
    ("风刃峡谷", 55, 8000, 10, 60, 70000, 120),
    ("暗蚀遗迹", 65, 12000, 12, 85, 120000, 180),
    ("圣光神殿", 75, 18000, 15, 120, 200000, 300),
    ("混沌之门", 90, 30000, 20, 200, 500000, 500),
];

struct PhantomData {
    essences: [i32; 8],
    forge_levels: [i32; 8],
    zone_clears: [i32; 8],
    daily_attempts: i32,
}

impl PhantomData {
    fn from_global(db: &Database, uid: &str) -> Self {
        let mut d = PhantomData {
            essences: [0; 8],
            forge_levels: [0; 8],
            zone_clears: [0; 8],
            daily_attempts: 0,
        };
        let data_str = db.global_get("phantom_forge", uid);
        if data_str.is_empty() {
            return d;
        }
        for part in data_str.split(';') {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() != 2 {
                continue;
            }
            let key = kv[0];
            let val: i32 = kv[1].parse().unwrap_or(0);
            if let Some(idx) = key.strip_prefix("ess_") {
                if let Ok(i) = idx.parse::<usize>() {
                    if i < 8 {
                        d.essences[i] = val;
                    }
                }
            } else if let Some(idx) = key.strip_prefix("forge_") {
                if let Ok(i) = idx.parse::<usize>() {
                    if i < 8 {
                        d.forge_levels[i] = val;
                    }
                }
            } else if let Some(idx) = key.strip_prefix("zc_") {
                if let Ok(i) = idx.parse::<usize>() {
                    if i < 8 {
                        d.zone_clears[i] = val;
                    }
                }
            } else if key == "daily" {
                d.daily_attempts = val;
            }
        }
        d
    }

    fn to_global(&self) -> String {
        let mut parts = Vec::new();
        for i in 0..8 {
            if self.essences[i] > 0 {
                parts.push(format!("ess_{}={}", i, self.essences[i]));
            }
            if self.forge_levels[i] > 0 {
                parts.push(format!("forge_{}={}", i, self.forge_levels[i]));
            }
            if self.zone_clears[i] > 0 {
                parts.push(format!("zc_{}={}", i, self.zone_clears[i]));
            }
        }
        if self.daily_attempts > 0 {
            parts.push(format!("daily={}", self.daily_attempts));
        }
        parts.join(";")
    }

    fn total_essence(&self) -> i32 {
        self.essences.iter().sum()
    }

    fn calc_bonus(&self) -> (i64, i64, i64, i64, i64, f64) {
        let mut ad = 0i64;
        let mut ap = 0i64;
        let mut def = 0i64;
        let mut mres = 0i64;
        let mut hp = 0i64;
        let mut crit = 0.0f64;
        for i in 0..8 {
            let level = self.forge_levels[i].min(4) as usize;
            let mult = FORGE_LEVELS[level].4;
            let base = 20 + (i as i64) * 5;
            match i {
                0 => ad += (base as f64 * mult) as i64,
                1 => def += (base as f64 * mult) as i64,
                2 => crit += mult * 0.5,
                3 => mres += (base as f64 * mult) as i64,
                4 => crit += mult * 0.3,
                5 => ap += (base as f64 * mult) as i64,
                6 => hp += (base as f64 * mult * 2.0) as i64,
                7 => {
                    let bonus = (base as f64 * mult * 0.5) as i64;
                    ad += bonus;
                    ap += bonus;
                    def += bonus / 2;
                    mres += bonus / 2;
                }
                _ => {}
            }
        }
        (ad, ap, def, mres, hp, crit)
    }
}

/// 查看幻域锻造
pub fn cmd_phantom_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }
    let d = PhantomData::from_global(db, user_id);
    let mut out = format!("{}\n═══ 🌀 幻域锻造 🌀 ═══\n", prefix);
    out.push_str("\n── 元素精华 ──\n");
    for (i, &(name, icon, _, _, _, _, _)) in ELEMENTS.iter().enumerate() {
        let level_idx = d.forge_levels[i].min(4) as usize;
        let (lname, licon, _, _, _) = FORGE_LEVELS[level_idx];
        out.push_str(&format!(
            "{} {} — {}级({}): 精华×{}\n",
            icon, name, lname, licon, d.essences[i]
        ));
    }
    out.push_str(&format!("\n📊 精华总数: {}\n", d.total_essence()));
    let (ad, ap, def, mres, hp, crit) = d.calc_bonus();
    out.push_str("\n── 幻域属性加成 ──\n");
    if hp > 0 {
        out.push_str(&format!("  ❤️ HP+{}\n", hp));
    }
    if ad > 0 {
        out.push_str(&format!("  ⚔️ 物攻+{}\n", ad));
    }
    if ap > 0 {
        out.push_str(&format!("  📖 魔攻+{}\n", ap));
    }
    if def > 0 {
        out.push_str(&format!("  🛡️ 防御+{}\n", def));
    }
    if mres > 0 {
        out.push_str(&format!("  🔮 魔抗+{}\n", mres));
    }
    if crit > 0.0 {
        out.push_str(&format!("  💨 暴击+{:.1}%\n", crit));
    }
    out.push_str(&format!("\n📅 今日挑战: {}/5次\n", d.daily_attempts));
    out
}

/// 幻域元素图鉴
pub fn cmd_phantom_elements(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut out = "═══ 🌀 幻域元素图鉴 🌀 ═══\n\n".to_string();
    for &(name, icon, lreq, drate, skill, strig, sdmg) in ELEMENTS.iter() {
        out.push_str(&format!(
            "{} {} — Lv{}+ | 掉落率{:.0}%\n",
            icon,
            name,
            lreq,
            drate * 100.0
        ));
        out.push_str(&format!(
            "  武器技能: {} | 触发率{:.0}% | 伤害{:.1}x\n",
            skill,
            strig * 100.0,
            sdmg
        ));
    }
    out.push_str("\n── 锻造等级 ──\n");
    for &(name, icon, ess, cost, mult) in FORGE_LEVELS.iter() {
        out.push_str(&format!(
            "{} {} — 精华{}个 + {}金 | 属性{:.1}x\n",
            icon,
            name,
            ess,
            utils::format_gold(cost),
            mult
        ));
    }
    out
}

/// 幻域锻造 — 提升锻造等级
pub fn cmd_phantom_forge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }
    let idx: usize = match args.trim().parse() {
        Ok(v) if v < 8 => v,
        _ => {
            return format!(
                "{}\n❌ 格式: 幻域锻造 <元素0-7>\n元素: 0炎狱 1冰渊 2雷鸣 3岩崩 4风刃 5暗蚀 6圣光 7混沌",
                prefix
            )
        }
    };
    let mut d = PhantomData::from_global(db, user_id);
    let cur_level = d.forge_levels[idx].min(4) as usize;
    if cur_level >= 4 {
        return format!("{}\n❌ {} 已达到最高锻造等级(幻域级)!", prefix, ELEMENTS[idx].0);
    }
    let next = cur_level + 1;
    let (_, _, ess_needed, gold_needed, _) = FORGE_LEVELS[next];
    let cur_ess = d.essences[idx];
    if cur_ess < ess_needed {
        return format!(
            "{}\n❌ {}精华不足! 需要{}个，当前{}个\n💡 通过幻域副本或击败怪物获得精华",
            prefix, ELEMENTS[idx].0, ess_needed, cur_ess
        );
    }
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < gold_needed {
        return format!("{}\n❌ 金币不足! 需要{}金币", prefix, utils::format_gold(gold_needed));
    }
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, gold_needed);
    d.essences[idx] -= ess_needed;
    d.forge_levels[idx] = next as i32;
    db.global_set("phantom_forge", user_id, &d.to_global());
    let (ename, eicon, _, _, eskill, etrig, edmg) = ELEMENTS[idx];
    let (lname, _, _, _, _) = FORGE_LEVELS[cur_level];
    let (nname, _, _, _, _) = FORGE_LEVELS[next];
    format!(
        "{}\n✅ 幻域锻造成功!\n{} {} {}级 → {}级\n💰 消耗: {}精华 + {}金币\n⚔️ 武器技能: {} (触发率{:.0}%, 伤害{:.1}x)",
        prefix,
        eicon,
        ename,
        lname,
        nname,
        ess_needed,
        utils::format_gold(gold_needed),
        eskill,
        etrig * 100.0,
        edmg
    )
}

/// 幻域副本列表
pub fn cmd_phantom_zones(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }
    let d = PhantomData::from_global(db, user_id);
    let mut out = format!("{}\n═══ 🌀 幻域副本 🌀 ═══\n", prefix);
    for (i, &(name, lreq, pwreq, waves, _, _, _)) in ZONES.iter().enumerate() {
        let (_, eicon, _, _, _, _, _) = ELEMENTS[i];
        let clears = d.zone_clears[i];
        out.push_str(&format!(
            "{}. {} {} — Lv{}/战力{} | {}波 | 通关{}次\n",
            i, eicon, name, lreq, pwreq, waves, clears
        ));
    }
    out.push_str(&format!("\n📅 今日挑战: {}/5次\n", d.daily_attempts));
    out
}

/// 挑战幻域副本
pub fn cmd_phantom_challenge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！", prefix);
    }
    let idx: usize = match args.trim().parse() {
        Ok(v) if v < 8 => v,
        _ => return format!("{}\n❌ 格式: 幻域挑战 <区域0-7>\n使用 幻域副本 查看区域列表", prefix),
    };
    let mut d = PhantomData::from_global(db, user_id);
    if d.daily_attempts >= 5 {
        return format!("{}\n❌ 今日挑战次数已用完(5/5)，明天再来!", prefix);
    }
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let (_, lreq, pwreq, waves, ess_rw, gold_rw, diam_rw) = ZONES[idx];
    if level < lreq {
        return format!("{}\n❌ 等级不足! 需要Lv{}，当前Lv{}", prefix, lreq, level);
    }
    let info = user::calc_total_attrs(db, user_id);
    let power = combat_power::calc_combat_power(&info) as i64;
    if power < pwreq {
        return format!("{}\n❌ 战力不足! 需要{}战力，当前{}", prefix, pwreq, power);
    }
    let mut rng = rand::thread_rng();
    let hp_str = db.read_basic(user_id, ITEM_HP);
    let mut player_hp: i64 = hp_str.parse().unwrap_or(100);
    let mut waves_cleared = 0i32;
    let mut total_damage = 0i64;
    for wave in 1..=waves {
        let wave_power = pwreq + (wave as i64 - 1) * (pwreq / 5);
        let monster_atk = wave_power / 3;
        for _ in 0..5 {
            let player_dmg = (power as f64 * rng.gen_range(0.8..1.2)) as i64;
            total_damage += player_dmg.max(1);
            let monster_dmg = (monster_atk as f64 * rng.gen_range(0.5..1.5)) as i64;
            player_hp -= monster_dmg.max(1);
            if player_hp <= 0 {
                break;
            }
        }
        if player_hp <= 0 {
            break;
        }
        waves_cleared += 1;
    }
    d.daily_attempts += 1;
    let (ename, eicon, _, _, _, _, _) = ELEMENTS[idx];
    let mut out = format!("{}\n═══ 🌀 {} 战斗报告 🌀 ═══\n\n", prefix, ZONES[idx].0);
    if waves_cleared >= waves {
        d.zone_clears[idx] += 1;
        d.essences[idx] += ess_rw;
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_rw);
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, diam_rw);
        out.push_str(&format!("🎉 通关成功! {}/{}波全清!\n", waves_cleared, waves));
        out.push_str(&format!("⚔️ 总伤害: {}\n", total_damage));
        out.push_str(&format!("❤️ 剩余HP: {}\n\n", player_hp.max(0)));
        out.push_str("── 奖励 ──\n");
        out.push_str(&format!("  {} {}精华 +{}\n", eicon, ename, ess_rw));
        out.push_str(&format!("  💰 {}金币\n", utils::format_gold(gold_rw)));
        out.push_str(&format!("  💎 {}钻石\n", diam_rw));
        let drate = ELEMENTS[idx].3;
        if rng.gen_bool(drate) {
            let bonus = rng.gen_range(1..=3i32);
            d.essences[idx] += bonus;
            out.push_str(&format!("  ✨ 额外掉落: {}精华×{}!\n", ename, bonus));
        }
    } else {
        out.push_str(&format!("💀 挑战失败! 通过 {}/{}波\n", waves_cleared, waves));
        out.push_str(&format!("⚔️ 总伤害: {}\n", total_damage));
        out.push_str(&format!("  💡 提升战力后再来挑战! (需要{}战力)\n", pwreq));
    }
    db.global_set("phantom_forge", user_id, &d.to_global());
    out.push_str(&format!("\n📅 今日挑战: {}/5次\n", d.daily_attempts));
    out
}

/// 幻域排行
pub fn cmd_phantom_ranking(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let conn = db.lock_conn();
    let mut stmt =
        match conn.prepare("SELECT User, Value FROM Global WHERE Section='phantom_forge' AND KeyName LIKE 'forge_%'") {
            Ok(s) => s,
            Err(_) => return "❌ 查询失败".to_string(),
        };
    use std::collections::BTreeMap;
    let mut user_scores: BTreeMap<String, i64> = BTreeMap::new();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .unwrap();
    for row in rows.flatten() {
        let uid = row.0;
        let key = &row.1;
        let val = &row.2;
        if let Some(idx_str) = key.strip_prefix("forge_") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                if idx < 5 {
                    let level: i32 = val.parse().unwrap_or(0);
                    let level_idx = level.min(4) as usize;
                    *user_scores.entry(uid.clone()).or_insert(0) += (FORGE_LEVELS[level_idx].4 * 100.0) as i64;
                }
            }
        }
    }
    let mut stmt2 =
        match conn.prepare("SELECT User, Value FROM Global WHERE Section='phantom_forge' AND KeyName LIKE 'zc_%'") {
            Ok(s) => s,
            Err(_) => return "❌ 查询失败".to_string(),
        };
    let rows2 = stmt2
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(2)?)))
        .unwrap();
    for row in rows2.flatten() {
        let uid = row.0;
        let val: i64 = row.1.parse().unwrap_or(0);
        *user_scores.entry(uid).or_insert(0) += val * 10;
    }
    let mut sorted: Vec<(String, i64)> = user_scores.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
    sorted.truncate(15);
    let mut out = "═══ 🌀 幻域锻造排行 🌀 ═══\n\n".to_string();
    if sorted.is_empty() {
        out.push_str("  (暂无数据)\n");
    } else {
        let medals = ["🥇", "🥈", "🥉"];
        for (i, (uid, score)) in sorted.iter().enumerate() {
            let medal = if i < 3 { medals[i] } else { "  " };
            let name = db.read_basic(uid, ITEM_NAME);
            let display = if name.is_empty() { uid.clone() } else { name };
            out.push_str(&format!("{} {}. {} — 幻域评分: {}\n", medal, i + 1, display, score));
        }
    }
    out
}

/// 幻域帮助
pub fn cmd_phantom_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    "═══ 🌀 幻域锻造系统帮助 🌀 ═══

📌 幻域元素 (8种):
  🔥 炎狱 — 灼烧(物攻) | Lv1+ | 25%掉落
  ❄️ 冰渊 — 冰冻(防御) | Lv10+ | 22%掉落
  ⚡ 雷鸣 — 雷击(暴击) | Lv20+ | 18%掉落
  🪨 岩崩 — 穿透(魔抗) | Lv30+ | 15%掉落
  🌪️ 风刃 — 加速(暴击) | Lv40+ | 12%掉落
  🌑 暗蚀 — 吸血(魔攻) | Lv50+ | 10%掉落
  ✨ 圣光 — 治愈(HP) | Lv60+ | 8%掉落
  🌀 混沌 — 全属性增幅 | Lv80+ | 5%掉落

📌 锻造等级 (5级):
  ⬜ 凡铁 → 🟢 秘银 → 🔵 星陨 → 🟣 龙魂 → 🟠 幻域
  属性倍率: 1.0x → 1.8x → 3.0x → 5.0x → 8.0x

📌 指令列表:
  幻域锻造 — 查看幻域锻造状态
  幻域元素 — 查看所有元素信息
  幻域升级 <元素0-7> — 提升锻造等级
  幻域副本 — 查看幻域副本列表
  幻域挑战 <区域0-7> — 挑战幻域副本
  幻域排行 — 全服幻域排行
  幻域帮助 — 查看此帮助

📌 幻域副本 (8个区域):
  每个区域对应一种元素，通关获得对应精华
  每日限5次挑战，波次递增难度

📌 精华获取:
  1. 幻域副本通关奖励
  2. 击败怪物随机掉落
  3. 掉落数量随怪物等级增加"
        .to_string()
}

/// 掉落幻域精华 — 战斗系统调用
#[allow(dead_code)]
pub fn drop_phantom_essence(db: &Database, user_id: &str, monster_level: i32) -> Option<(usize, i32)> {
    let mut rng = rand::thread_rng();
    let eligible: Vec<usize> = (0..8).filter(|&i| monster_level >= ELEMENTS[i].2).collect();
    if eligible.is_empty() {
        return None;
    }
    let idx = eligible[rng.gen_range(0..eligible.len())];
    let drop_rate = ELEMENTS[idx].3;
    if rng.gen_bool(drop_rate) {
        let amount = 1 + (monster_level / 20).min(4);
        let mut d = PhantomData::from_global(db, user_id);
        d.essences[idx] += amount;
        db.global_set("phantom_forge", user_id, &d.to_global());
        Some((idx, amount))
    } else {
        None
    }
}

/// 获取幻域加成 — 战斗系统集成
#[allow(dead_code)]
pub fn get_phantom_bonus(db: &Database, user_id: &str) -> (i64, i64, i64, i64, i64, f64) {
    let d = PhantomData::from_global(db, user_id);
    d.calc_bonus()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elements_count() {
        assert_eq!(ELEMENTS.len(), 8);
    }

    #[test]
    fn test_forge_levels_count() {
        assert_eq!(FORGE_LEVELS.len(), 5);
    }

    #[test]
    fn test_zones_count() {
        assert_eq!(ZONES.len(), 8);
    }

    #[test]
    fn test_element_names_unique() {
        let names: Vec<&str> = ELEMENTS.iter().map(|e| e.0).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_element_level_req_escalation() {
        for i in 1..ELEMENTS.len() {
            assert!(ELEMENTS[i].2 >= ELEMENTS[i - 1].2);
        }
    }

    #[test]
    fn test_element_drop_rate_decreasing() {
        for i in 1..ELEMENTS.len() {
            assert!(ELEMENTS[i].3 <= ELEMENTS[i - 1].3);
        }
    }

    #[test]
    fn test_forge_multiplier_escalation() {
        for i in 1..FORGE_LEVELS.len() {
            assert!(FORGE_LEVELS[i].4 > FORGE_LEVELS[i - 1].4);
        }
    }

    #[test]
    fn test_forge_essence_escalation() {
        for i in 1..FORGE_LEVELS.len() {
            assert!(FORGE_LEVELS[i].2 > FORGE_LEVELS[i - 1].2);
        }
    }

    #[test]
    fn test_forge_cost_escalation() {
        for i in 1..FORGE_LEVELS.len() {
            assert!(FORGE_LEVELS[i].3 > FORGE_LEVELS[i - 1].3);
        }
    }

    #[test]
    fn test_zone_level_req_escalation() {
        for i in 1..ZONES.len() {
            assert!(ZONES[i].1 >= ZONES[i - 1].1);
        }
    }

    #[test]
    fn test_zone_power_req_escalation() {
        for i in 1..ZONES.len() {
            assert!(ZONES[i].2 > ZONES[i - 1].2);
        }
    }

    #[test]
    fn test_zone_waves_escalation() {
        for i in 1..ZONES.len() {
            assert!(ZONES[i].3 >= ZONES[i - 1].3);
        }
    }

    #[test]
    fn test_zone_reward_escalation() {
        for i in 1..ZONES.len() {
            assert!(ZONES[i].4 > ZONES[i - 1].4);
        }
    }

    #[test]
    fn test_skill_names_unique() {
        let names: Vec<&str> = ELEMENTS.iter().map(|e| e.4).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_skill_trigger_rates_valid() {
        for e in ELEMENTS.iter() {
            assert!(e.5 > 0.0 && e.5 <= 1.0);
        }
    }

    #[test]
    fn test_skill_damage_mults_valid() {
        for e in ELEMENTS.iter() {
            assert!(e.6 > 1.0);
        }
    }

    #[test]
    fn test_phantom_data_default() {
        let d = PhantomData {
            essences: [0; 8],
            forge_levels: [0; 8],
            zone_clears: [0; 8],
            daily_attempts: 0,
        };
        assert_eq!(d.total_essence(), 0);
        let (ad, ap, _def, _mres, hp, _crit) = d.calc_bonus();
        assert!(ad > 0);
        assert!(ap > 0);
        assert!(hp > 0);
    }

    #[test]
    fn test_phantom_data_maxed() {
        let d = PhantomData {
            essences: [0; 8],
            forge_levels: [4; 8],
            zone_clears: [0; 8],
            daily_attempts: 0,
        };
        let (ad, ap, def, mres, hp, crit) = d.calc_bonus();
        assert!(ad > 0);
        assert!(ap > 0);
        assert!(def > 0);
        assert!(mres > 0);
        assert!(hp > 0);
        assert!(crit > 0.0);
    }

    #[test]
    fn test_phantom_data_serialization() {
        let d = PhantomData {
            essences: [10, 20, 0, 0, 0, 0, 0, 0],
            forge_levels: [1, 0, 0, 0, 0, 0, 0, 0],
            zone_clears: [3, 0, 0, 0, 0, 0, 0, 0],
            daily_attempts: 2,
        };
        let s = d.to_global();
        assert!(s.contains("ess_0=10"));
        assert!(s.contains("ess_1=20"));
        assert!(s.contains("forge_0=1"));
        assert!(s.contains("zc_0=3"));
        assert!(s.contains("daily=2"));
    }

    #[test]
    fn test_zone_chaos_gate_highest_req() {
        let (_, lreq, pwreq, waves, ess_rw, gold_rw, diam_rw) = ZONES[7];
        assert_eq!(lreq, 90);
        assert_eq!(pwreq, 30000);
        assert_eq!(waves, 20);
        assert_eq!(ess_rw, 200);
        assert_eq!(gold_rw, 500000);
        assert_eq!(diam_rw, 500);
    }
}

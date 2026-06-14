/// CakeGame 公会圣物系统
/// 公会协作发现和升级圣物，为全公会成员提供被动属性加成
/// 通过公会挑战、捐献、集体活动获取圣物碎片
/// 数据存储：Global 表 SECTION='guild_relics'
use crate::core::*;
use crate::db::Database;

/// 圣物等级: (名称, 碎片阈值, 描述, (HP%, 物攻%, 魔攻%, 防御%, 魔抗%))
type RelicLevel = (&'static str, u64, &'static str, (f64, f64, f64, f64, f64));
const RELIC_LEVELS: &[RelicLevel] = &[
    ("沉睡圣物⚪", 0, "尚未觉醒的力量", (0.0, 0.0, 0.0, 0.0, 0.0)),
    ("初醒圣物🟢", 200, "微微散发光芒", (2.0, 2.0, 2.0, 1.0, 1.0)),
    ("觉醒圣物🔵", 800, "力量开始涌动", (5.0, 5.0, 5.0, 3.0, 3.0)),
    ("辉煌圣物🟣", 2000, "神圣之光弥漫", (10.0, 10.0, 10.0, 6.0, 6.0)),
    ("传说圣物🟠", 5000, "传说之力降临", (18.0, 18.0, 18.0, 10.0, 10.0)),
    ("神话圣物🔴", 12000, "神话之力觉醒", (28.0, 28.0, 28.0, 15.0, 15.0)),
    ("创世圣物🟡", 30000, "创世之力降临人间", (40.0, 40.0, 40.0, 22.0, 22.0)),
];

/// 圣物类型: (名称, 分类, 描述, 加成类型)
const RELIC_TYPES: &[(&str, &str, &str, &str)] = &[
    ("龙魂圣剑🗡️", "物攻型", "传说中屠龙者留下的圣剑", "物攻%加成"),
    ("凤凰圣冠👑", "魔攻型", "浴火重生的凤凰遗物", "魔攻%加成"),
    ("玄武圣盾🛡️", "防御型", "远古玄武甲壳锻造", "防御%加成"),
    ("麒麟圣环💫", "生命型", "祥瑞麒麟留下的圣环", "HP%加成"),
    ("朱雀圣翼🪽", "魔抗型", "朱雀展翅时遗落的圣羽", "魔抗%加成"),
    ("混沌圣珠🔮", "全能型", "蕴含混沌之力的圣珠", "全属性%加成"),
];

/// 碎片来源: (名称, 数量, 描述)
const FRAGMENT_SOURCES: &[(&str, u64, &str)] = &[
    ("公会挑战", 30, "完成公会试炼获得"),
    ("公会捐献", 5, "金币/钻石捐献获得"),
    ("公会战", 20, "参与公会战获得"),
    ("世界BOSS", 15, "击败世界BOSS获得"),
    ("每日签到", 3, "公会每日签到获得"),
    ("怪物狩猎", 2, "击杀怪物概率获得"),
];

/// 圣物成就里程碑: (名称, 碎片阈值, 金币奖励, 钻石奖励)
const RELIC_MILESTONES: &[(&str, u64, u64, u64)] = &[
    ("圣物初识", 100, 5000, 30),
    ("圣物守护者", 500, 20000, 100),
    ("圣物大师", 2000, 80000, 300),
    ("圣物之魂", 5000, 200000, 800),
    ("圣物创世者", 15000, 500000, 2000),
];

/// 获取圣物等级信息
fn get_relic_level(fragments: u64) -> (usize, &'static str, &'static str, (f64, f64, f64, f64, f64)) {
    let mut idx = 0;
    for (i, &(_, threshold, _, _)) in RELIC_LEVELS.iter().enumerate() {
        if fragments >= threshold {
            idx = i;
        }
    }
    let (name, _, desc, bonus) = RELIC_LEVELS[idx];
    (idx, name, desc, bonus)
}

/// 生成进度条
fn progress_bar(current: u64, target: u64, width: usize) -> String {
    let pct = if target == 0 {
        1.0
    } else {
        (current as f64 / target as f64).min(1.0)
    };
    let filled = (pct * width as f64) as usize;
    let empty = width - filled;
    format!("{}{} {:.0}%", "█".repeat(filled), "░".repeat(empty), pct * 100.0)
}

/// 获取用户公会ID
fn get_user_guild_id(db: &Database, user_id: &str) -> i32 {
    db.read_basic(user_id, "Guild_ID").parse().unwrap_or(0)
}

/// 获取公会碎片值
fn get_guild_value(db: &Database, section: &str, key: &str) -> u64 {
    db.global_get(section, key).parse().unwrap_or(0)
}

/// 设置公会碎片值
fn set_guild_value(db: &Database, section: &str, key: &str, value: u64) {
    db.global_set(section, key, &value.to_string());
}

/// 查看公会圣物
pub fn cmd_view_guild_relics(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入任何公会，无法查看公会圣物。".to_string();
    }

    let section = format!("guild_relics_{}", guild_id);
    let mut output = String::from("⚔️ ═══════【公会圣物殿堂】═══════ ⚔️\n\n");

    // 公会总碎片
    let total_fragments = get_guild_value(db, &section, "total_fragments");
    let (level_idx, level_name, _, total_bonus) = get_relic_level(total_fragments);
    output.push_str(&format!(
        "🏛️ 公会圣物等级: {} [{}]\n",
        level_name, RELIC_LEVELS[level_idx].0
    ));
    output.push_str(&format!("💎 圣物碎片总量: {}\n", total_fragments));

    // 进度条
    if level_idx + 1 < RELIC_LEVELS.len() {
        let next_threshold = RELIC_LEVELS[level_idx + 1].1;
        output.push_str(&format!(
            "📈 升级进度: {}\n\n",
            progress_bar(total_fragments, next_threshold, 20)
        ));
    } else {
        output.push_str("📈 已达最高等级 ✨\n\n");
    }

    // 全属性加成总览
    output.push_str("📊 全公会成员属性加成:\n");
    if total_bonus.0 > 0.0 {
        output.push_str(&format!(
            "  ❤️ HP: +{:.0}%  ⚔️ 物攻: +{:.0}%  🔮 魔攻: +{:.0}%\n",
            total_bonus.0, total_bonus.1, total_bonus.2
        ));
        output.push_str(&format!(
            "  🛡️ 防御: +{:.0}%  🔮 魔抗: +{:.0}%\n",
            total_bonus.3, total_bonus.4
        ));
    } else {
        output.push_str("  暂无加成 — 捐献碎片以解锁圣物之力！\n");
    }

    output.push('\n');

    // 各圣物状态
    output.push_str("═══ 圣物清单 ═══\n");
    for (i, (name, category, desc, _)) in RELIC_TYPES.iter().enumerate() {
        let relic_key = format!("relic_{}", i);
        let relic_fragments = get_guild_value(db, &section, &relic_key);
        let (r_idx, r_name, _, _) = get_relic_level(relic_fragments);
        output.push_str(&format!("  {} {} [{}] — {}\n", name, category, r_name, desc));
        output.push_str(&format!(
            "    碎片: {} | {}\n",
            relic_fragments,
            progress_bar(
                relic_fragments,
                RELIC_LEVELS.get(r_idx + 1).map_or(u64::MAX, |x| x.1),
                15
            )
        ));
    }

    output.push_str(
        "\n💡 捐献方式: 公会挑战(+30) | 公会捐献(+5) | 公会战(+20) | 世界BOSS(+15) | 每日签到(+3) | 怪物狩猎(+2)",
    );
    output.push_str("\n📖 指令: 捐献圣物 <数量> | 圣物排行 | 圣物详情 <编号> | 圣物帮助");

    output
}

/// 捐献圣物碎片
pub fn cmd_donate_relic(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入任何公会。".to_string();
    }

    let amount: u64 = match args.trim().parse() {
        Ok(n) if n > 0 => n,
        _ => {
            return "❌ 请输入有效的捐献数量。用法: 捐献圣物 <数量>".to_string();
        }
    };

    if amount > 5000 {
        return "❌ 单次最多捐献5000碎片，请分次捐献。".to_string();
    }

    let section = format!("guild_relics_{}", guild_id);

    // 扣除玩家金币作为捐献代价 (1碎片 = 100金币)
    let cost = amount as i64 * 100;
    let user_gold = db.read_currency(user_id, CURRENCY_GOLD);
    if user_gold < cost {
        return format!(
            "❌ 金币不足！捐献{}碎片需要{}金币，你只有{}金币。",
            amount, cost, user_gold
        );
    }
    db.write_currency(user_id, CURRENCY_GOLD, user_gold - cost);

    // 增加公会碎片
    let old_total = get_guild_value(db, &section, "total_fragments");
    let new_total = old_total + amount;
    set_guild_value(db, &section, "total_fragments", new_total);

    // 增加个人贡献
    let contrib_key = format!("contrib_{}", user_id);
    let old_contrib = get_guild_value(db, &section, &contrib_key);
    set_guild_value(db, &section, &contrib_key, old_contrib + amount);

    // 分配到各圣物 (平均分配 + 余数给第一个)
    let relic_count = RELIC_TYPES.len() as u64;
    let base_each = amount / relic_count;
    for i in 0..RELIC_TYPES.len() {
        let relic_key = format!("relic_{}", i);
        let old_val = get_guild_value(db, &section, &relic_key);
        let mut add = base_each;
        if i == 0 {
            add += amount - base_each * relic_count;
        }
        set_guild_value(db, &section, &relic_key, old_val + add);
    }

    // 等级检查
    let (old_level, _, _, _) = get_relic_level(old_total);
    let (new_level, new_name, _, new_bonus) = get_relic_level(new_total);

    let mut output = format!("✅ 捐献成功！消耗{}金币，捐献{}圣物碎片。\n", cost, amount);
    output.push_str(&format!("💎 公会碎片总量: {} → {}\n", old_total, new_total));
    output.push_str(&format!("📊 你的累计贡献: {}\n", old_contrib + amount));

    if new_level > old_level {
        output.push_str(&format!(
            "\n🎉🎉🎉 圣物升级！{} → {}\n",
            RELIC_LEVELS[old_level].0, new_name
        ));
        output.push_str(&format!(
            "  全公会成员获得加成: HP+{:.0}% 物攻+{:.0}% 魔攻+{:.0}% 防御+{:.0}% 魔抗+{:.0}%\n",
            new_bonus.0, new_bonus.1, new_bonus.2, new_bonus.3, new_bonus.4
        ));
    }

    // 检查里程碑
    if let Some(milestone) = check_milestone(db, &section, user_id, old_contrib + amount) {
        output.push_str(&format!(
            "\n🏆 达成里程碑: {}！奖励{}金币+{}钻石\n",
            milestone.0, milestone.1, milestone.2
        ));
        let cur_gold = db.read_currency(user_id, CURRENCY_GOLD);
        db.write_currency(user_id, CURRENCY_GOLD, cur_gold + milestone.1 as i64);
        let cur_diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
        db.write_currency(user_id, CURRENCY_DIAMOND, cur_diamond + milestone.2 as i64);
    }

    output
}

/// 圣物详情
pub fn cmd_relic_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入任何公会。".to_string();
    }

    let idx: usize = match args.trim().parse::<usize>() {
        Ok(n) if n > 0 && n <= RELIC_TYPES.len() => n - 1,
        _ => {
            return format!("❌ 请输入有效的圣物编号(1-{})。", RELIC_TYPES.len());
        }
    };

    let section = format!("guild_relics_{}", guild_id);
    let (name, category, desc, bonus_type) = RELIC_TYPES[idx];
    let relic_key = format!("relic_{}", idx);
    let fragments = get_guild_value(db, &section, &relic_key);
    let (level_idx, level_name, level_desc, bonus) = get_relic_level(fragments);

    let mut output = String::from("⚔️ ═══════【圣物详情】═══════ ⚔️\n\n");
    output.push_str(&format!("{} {} [{}]\n", name, category, level_name));
    output.push_str(&format!("📖 {}\n", desc));
    output.push_str(&format!("🔮 加成类型: {}\n\n", bonus_type));

    output.push_str(&format!("💎 当前碎片: {}\n", fragments));
    output.push_str(&format!("✨ 当前阶段: {} — {}\n", level_name, level_desc));

    if level_idx + 1 < RELIC_LEVELS.len() {
        let next = RELIC_LEVELS[level_idx + 1];
        output.push_str(&format!("📈 下一阶段: {} (需要{}碎片)\n", next.0, next.1));
        output.push_str(&format!("📊 升级进度: {}\n\n", progress_bar(fragments, next.1, 20)));
    } else {
        output.push_str("📈 已达最高等级 ✨\n\n");
    }

    output.push_str("📊 当前属性加成:\n");
    output.push_str(&format!(
        "  ❤️ HP: +{:.0}%  ⚔️ 物攻: +{:.0}%  🔮 魔攻: +{:.0}%\n",
        bonus.0, bonus.1, bonus.2
    ));
    output.push_str(&format!("  🛡️ 防御: +{:.0}%  🔮 魔抗: +{:.0}%\n", bonus.3, bonus.4));

    // 显示所有等级
    output.push_str("\n═══ 等级体系 ═══\n");
    for (i, &(lname, threshold, ldesc, _)) in RELIC_LEVELS.iter().enumerate() {
        let marker = if i == level_idx { " ◀ 当前" } else { "" };
        output.push_str(&format!(
            "  Lv.{} {} — {}碎片 — {}{}\n",
            i + 1,
            lname,
            threshold,
            ldesc,
            marker
        ));
    }

    output
}

/// 圣物贡献排行
pub fn cmd_relic_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return "❌ 你还没有加入任何公会。".to_string();
    }

    let section = format!("guild_relics_{}", guild_id);

    // 收集所有贡献者 — 遍历所有可能的 user_id
    let all_users = db.all_users();
    let mut contributors: Vec<(String, u64)> = Vec::new();
    for uid in &all_users {
        let contrib_key = format!("contrib_{}", uid);
        let contrib = get_guild_value(db, &section, &contrib_key);
        if contrib > 0 {
            // 检查是否同公会
            let u_guild = get_user_guild_id(db, uid);
            if u_guild == guild_id {
                contributors.push((uid.clone(), contrib));
            }
        }
    }
    contributors.sort_by_key(|x| std::cmp::Reverse(x.1));

    let total_fragments = get_guild_value(db, &section, "total_fragments");
    let (_, level_name, _, _) = get_relic_level(total_fragments);

    let mut output = String::from("⚔️ ═══════【圣物贡献排行】═══════ ⚔️\n\n");
    output.push_str(&format!(
        "🏛️ 公会圣物等级: {} | 💎 碎片总量: {}\n\n",
        level_name, total_fragments
    ));

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, contrib)) in contributors.iter().take(15).enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let pct = contrib
            .checked_mul(100)
            .and_then(|v| v.checked_div(total_fragments))
            .unwrap_or(0);
        let marker = if uid == user_id { " ◀ 你" } else { "" };
        output.push_str(&format!(
            "{} #{:2}. UID:{} — {}碎片 ({}%){}\n",
            medal,
            i + 1,
            uid,
            contrib,
            pct,
            marker
        ));
    }

    if contributors.is_empty() {
        output.push_str("暂无贡献记录。快来捐献圣物碎片吧！\n");
    }

    // 个人排名
    if let Some(pos) = contributors.iter().position(|(uid, _)| uid == user_id) {
        let my_contrib = contributors[pos].1;
        output.push_str(&format!(
            "\n📊 你的排名: #{} — {}碎片 (占比{:.1}%)\n",
            pos + 1,
            my_contrib,
            if total_fragments > 0 {
                my_contrib as f64 * 100.0 / total_fragments as f64
            } else {
                0.0
            }
        ));
    } else {
        output.push_str("\n📊 你尚未贡献过圣物碎片。\n");
    }

    output.push_str("\n💡 捐献方式: 捐献圣物 <数量> (100金币/碎片)");
    output
}

/// 圣物帮助
pub fn cmd_relic_help(_db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut output = String::from("⚔️ ═══════【公会圣物帮助】═══════ ⚔️\n\n");
    output.push_str("📖 公会圣物是公会全体成员共享的被动加成系统。\n\n");

    output.push_str("═══ 基本规则 ═══\n");
    output.push_str("• 公会成员捐献金币获取圣物碎片\n");
    output.push_str("• 碎片自动分配到6种圣物上\n");
    output.push_str("• 圣物等级提升后，全公会成员获得属性加成\n");
    output.push_str("• 加成自动生效，无需手动装备\n\n");

    output.push_str("═══ 圣物等级 (7级) ═══\n");
    for (i, &(name, threshold, desc, _)) in RELIC_LEVELS.iter().enumerate() {
        output.push_str(&format!("  Lv.{} {} — {}碎片 — {}\n", i + 1, name, threshold, desc));
    }

    output.push_str("\n═══ 6种圣物 ═══\n");
    for (name, category, desc, bonus_type) in RELIC_TYPES {
        output.push_str(&format!("  {} {} — {} ({})\n", name, category, desc, bonus_type));
    }

    output.push_str("\n═══ 碎片来源 ═══\n");
    for (source, amount, desc) in FRAGMENT_SOURCES {
        output.push_str(&format!("  {} (+{}) — {}\n", source, amount, desc));
    }

    output.push_str("\n═══ 里程碑奖励 ═══\n");
    for (name, threshold, gold, diamond) in RELIC_MILESTONES {
        output.push_str(&format!(
            "  🏆 {} — {}碎片 — {}金+{}💎\n",
            name, threshold, gold, diamond
        ));
    }

    output.push_str("\n═══ 指令列表 ═══\n");
    output.push_str("  查看圣物 — 查看公会圣物总览\n");
    output.push_str("  捐献圣物 <数量> — 捐献金币获取碎片(100金/碎片)\n");
    output.push_str("  圣物详情 <编号> — 查看指定圣物详情(1-6)\n");
    output.push_str("  圣物排行 — 查看贡献排行榜\n");
    output.push_str("  圣物帮助 — 显示本帮助\n");

    output.push_str("\n💡 小贴士: 所有公会成员共享圣物加成，多多捐献让公会更强！");

    output
}

/// 圣物被动加成计算 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_relic_bonus(db: &Database, user_id: &str) -> (f64, f64, f64, f64, f64) {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let section = format!("guild_relics_{}", guild_id);
    let total_fragments = get_guild_value(db, &section, "total_fragments");
    let (_, _, _, bonus) = get_relic_level(total_fragments);
    bonus
}

/// 检查里程碑奖励
fn check_milestone(
    db: &Database,
    section: &str,
    user_id: &str,
    total_contrib: u64,
) -> Option<(&'static str, u64, u64)> {
    let bitmask_key = format!("milestone_{}", user_id);
    let bitmask = get_guild_value(db, section, &bitmask_key);

    for (i, &(name, threshold, gold, diamond)) in RELIC_MILESTONES.iter().enumerate() {
        let bit = 1u64 << i;
        if total_contrib >= threshold && (bitmask & bit) == 0 {
            set_guild_value(db, section, &bitmask_key, bitmask | bit);
            return Some((name, gold, diamond));
        }
    }
    None
}

/// 记录圣物碎片来源 (供其他系统调用)
#[allow(dead_code)]
pub fn record_relic_fragments(db: &Database, user_id: &str, _source: &str, amount: u64) {
    let guild_id = get_user_guild_id(db, user_id);
    if guild_id == 0 {
        return;
    }

    let section = format!("guild_relics_{}", guild_id);
    let old = get_guild_value(db, &section, "total_fragments");
    set_guild_value(db, &section, "total_fragments", old + amount);

    let contrib_key = format!("contrib_{}", user_id);
    let old_contrib = get_guild_value(db, &section, &contrib_key);
    set_guild_value(db, &section, &contrib_key, old_contrib + amount);
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relic_levels_count() {
        assert_eq!(RELIC_LEVELS.len(), 7);
    }

    #[test]
    fn test_relic_levels_sorted() {
        for i in 1..RELIC_LEVELS.len() {
            assert!(RELIC_LEVELS[i].1 > RELIC_LEVELS[i - 1].1);
        }
    }

    #[test]
    fn test_relic_types_count() {
        assert_eq!(RELIC_TYPES.len(), 6);
    }

    #[test]
    fn test_fragment_sources_count() {
        assert_eq!(FRAGMENT_SOURCES.len(), 6);
    }

    #[test]
    fn test_milestones_sorted() {
        for i in 1..RELIC_MILESTONES.len() {
            assert!(RELIC_MILESTONES[i].1 > RELIC_MILESTONES[i - 1].1);
        }
    }

    #[test]
    fn test_get_relic_level_zero() {
        let (idx, name, _, bonus) = get_relic_level(0);
        assert_eq!(idx, 0);
        assert_eq!(name, "沉睡圣物⚪");
        assert_eq!(bonus, (0.0, 0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_get_relic_level_max() {
        let (idx, name, _, bonus) = get_relic_level(999999);
        assert_eq!(idx, 6);
        assert_eq!(name, "创世圣物🟡");
        assert!(bonus.0 > 0.0);
    }

    #[test]
    fn test_get_relic_level_boundary() {
        let (idx, _, _, _) = get_relic_level(200);
        assert_eq!(idx, 1); // 初醒圣物

        let (idx, _, _, _) = get_relic_level(199);
        assert_eq!(idx, 0); // 沉睡圣物
    }

    #[test]
    fn test_get_relic_level_progression() {
        let mut prev_hp = 0.0;
        for threshold in [0, 200, 800, 2000, 5000, 12000, 30000] {
            let (_, _, _, bonus) = get_relic_level(threshold);
            assert!(
                bonus.0 >= prev_hp,
                "HP bonus should increase at threshold {}",
                threshold
            );
            prev_hp = bonus.0;
        }
    }

    #[test]
    fn test_progress_bar_empty() {
        let bar = progress_bar(0, 100, 10);
        assert!(bar.contains("0%"));
        assert!(bar.contains("░"));
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = progress_bar(100, 100, 10);
        assert!(bar.contains("100%"));
        assert!(bar.contains("█"));
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = progress_bar(50, 100, 10);
        assert!(bar.contains("50%"));
    }

    #[test]
    fn test_progress_bar_overflow() {
        let bar = progress_bar(200, 100, 10);
        assert!(bar.contains("100%"));
    }

    #[test]
    fn test_milestone_names_unique() {
        let mut names: Vec<&str> = RELIC_MILESTONES.iter().map(|x| x.0).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len);
    }

    #[test]
    fn test_milestone_rewards_positive() {
        for &(_, _, gold, diamond) in RELIC_MILESTONES {
            assert!(gold > 0);
            assert!(diamond > 0);
        }
    }

    #[test]
    fn test_milestone_rewards_escalate() {
        for i in 1..RELIC_MILESTONES.len() {
            assert!(
                RELIC_MILESTONES[i].2 > RELIC_MILESTONES[i - 1].2,
                "Gold should escalate at milestone {}",
                i
            );
        }
    }

    #[test]
    fn test_relic_bonus_symmetric() {
        let (_, _, _, bonus1) = get_relic_level(5000);
        let (_, _, _, bonus2) = get_relic_level(5000);
        assert_eq!(bonus1, bonus2);
    }
}

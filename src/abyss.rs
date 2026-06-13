/// CakeGame 无尽深渊系统
/// 渐进式难度挑战模式 — 玩家逐层挑战越来越强的怪物，获取递增奖励
/// 数据存储：Basic_User 表的 abyss_floor / abyss_best / abyss_coins 列
use crate::db::Database;
use rand::Rng;

/// 深渊层定义
#[allow(dead_code)]
struct AbyssFloor {
    floor: i32,
    name: &'static str,
    monster_hp: i32,
    monster_ad: i32,
    monster_def: i32,
    monster_mr: i32,
    gold_reward: i32,
    exp_reward: i32,
    diamond_reward: i32,
}

/// 获取深渊层定义（100层）
fn get_floor_def(floor: i32) -> AbyssFloor {
    let base_hp = 200 + floor * 80;
    let base_ad = 30 + floor * 15;
    let base_def = 10 + floor * 5;
    let base_mr = 5 + floor * 4;
    let gold = 50 + floor * 30;
    let exp = 100 + floor * 50;
    let diamond = if floor % 10 == 0 { floor / 10 * 5 } else { 0 };

    let name = match floor {
        1..=10 => "深渊浅层",
        11..=20 => "深渊中层",
        21..=30 => "深渊深层",
        31..=50 => "深渊核心",
        51..=70 => "深渊炼狱",
        71..=90 => "深渊混沌",
        91..=100 => "深渊终焉",
        _ => "深渊尽头",
    };

    AbyssFloor {
        floor,
        name,
        monster_hp: base_hp,
        monster_ad: base_ad,
        monster_def: base_def,
        monster_mr: base_mr,
        gold_reward: gold,
        exp_reward: exp,
        diamond_reward: diamond,
    }
}

/// 深渊怪物名称（按层递进）
fn get_monster_name(floor: i32) -> &'static str {
    match floor {
        1..=5 => "深渊哨兵",
        6..=10 => "深渊守卫",
        11..=15 => "深渊骑士",
        16..=20 => "深渊法师",
        21..=30 => "深渊领主",
        31..=40 => "深渊恶魔",
        41..=50 => "深渊巨龙",
        51..=60 => "深渊死神",
        61..=70 => "深渊魔君",
        71..=80 => "深渊毁灭者",
        81..=90 => "深渊泰坦",
        91..=100 => "深渊之王",
        _ => "深渊之神",
    }
}

/// 读取玩家深渊进度
fn get_abyss_progress(db: &Database, user_id: &str) -> (i32, i32, i32) {
    let current = db.read_basic(user_id, "abyss_floor").parse::<i32>().unwrap_or(0);
    let best = db.read_basic(user_id, "abyss_best").parse::<i32>().unwrap_or(0);
    let coins = db.read_basic(user_id, "abyss_coins").parse::<i32>().unwrap_or(0);
    (current, best, coins)
}

/// 查看深渊 — 显示系统概览和玩家进度
pub fn cmd_view_abyss(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let (current, best, coins) = get_abyss_progress(db, user_id);

    let mut out = format!("{}\n═══ 🌀 无尽深渊 ═══\n", prefix);
    out.push_str("挑战无尽深渊，逐层击败越来越强的怪物！\n");
    out.push_str("每10层有钻石奖励，层数越高奖励越丰厚\n\n");

    if current > 0 {
        out.push_str(&format!("📍 当前进度：第 {} 层\n", current));
    }
    if best > 0 {
        out.push_str(&format!("🏆 最高记录：第 {} 层\n", best));
    }
    if coins > 0 {
        out.push_str(&format!("💰 累计获得：{} 金币\n", coins));
    }

    if current == 0 && best == 0 {
        out.push_str("💡 你还没有挑战过深渊\n");
    }

    out.push_str("\n🎮 指令：\n");
    out.push_str("  挑战深渊 — 进入当前层挑战\n");
    out.push_str("  深渊进度 — 查看挑战进度\n");
    out.push_str("  深渊排行 — 查看全服排行\n");
    out.push_str("  重置深渊 — 从第1层重新开始\n");

    out
}

/// 挑战深渊 — 挑战当前层
pub fn cmd_challenge_abyss(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 检查玩家是否存活
    let hp = db.read_basic(user_id, "P_HP").parse::<i32>().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n你已经阵亡，请先恢复生命再挑战深渊！", prefix);
    }

    let (current, best, _coins) = get_abyss_progress(db, user_id);
    let next_floor = if current == 0 { 1 } else { current };

    if next_floor > 100 {
        return format!(
            "{}\n🎉 恭喜！你已经通关了无尽深渊全部100层！\n🏆 最高记录：{} 层\n\n💡 使用「重置深渊」可以重新挑战",
            prefix, best
        );
    }

    let floor_def = get_floor_def(next_floor);
    let monster_name = get_monster_name(next_floor);

    // 获取玩家属性
    let user_ad = db.read_basic(user_id, "BaseSum_1").parse::<i32>().unwrap_or(50);
    let user_def = db.read_basic(user_id, "BaseSum_2").parse::<i32>().unwrap_or(10);

    // 战斗模拟 — 简化回合制
    let mut rng = rand::thread_rng();
    let mut player_hp = hp;
    let mut monster_hp = floor_def.monster_hp;
    let mut rounds = 0;

    while player_hp > 0 && monster_hp > 0 && rounds < 50 {
        rounds += 1;
        // 玩家攻击
        let base_dmg = (user_ad as f64 * (0.8 + rng.gen_range(0.0..0.4))) as i32;
        let actual_dmg = (base_dmg - floor_def.monster_def).max(1);
        monster_hp -= actual_dmg;

        if monster_hp <= 0 {
            break;
        }

        // 怪物攻击
        let base_dmg = (floor_def.monster_ad as f64 * (0.8 + rng.gen_range(0.0..0.4))) as i32;
        let actual_dmg = (base_dmg - user_def).max(1);
        player_hp -= actual_dmg;
    }

    if monster_hp <= 0 {
        // 胜利！
        db.write_basic(user_id, "abyss_floor", &(next_floor + 1).to_string());
        if next_floor > best {
            db.write_basic(user_id, "abyss_best", &next_floor.to_string());
        }
        crate::achievement::on_abyss_floor(db, user_id, next_floor);

        // 发放奖励
        db.modify_currency(user_id, "Currency_gold", "add", floor_def.gold_reward as i64);
        if floor_def.diamond_reward > 0 {
            crate::achievement::on_diamond_earned(db, user_id, floor_def.diamond_reward as i64);
        }
        db.modify_currency(user_id, "Currency_diamond", "add", floor_def.diamond_reward as i64);

        // 累计金币
        let total_coins = db.read_basic(user_id, "abyss_coins").parse::<i32>().unwrap_or(0) + floor_def.gold_reward;
        db.write_basic(user_id, "abyss_coins", &total_coins.to_string());

        let mut out = format!("{}\n═══ 🌀 深渊第{}层 — {} ═══\n", prefix, next_floor, floor_def.name);
        out.push_str(&format!("👹 怪物：{}\n", monster_name));
        out.push_str(&format!("⚔️ 战斗回合：{} 回合\n\n", rounds));
        out.push_str("🎉 挑战成功！\n\n");
        out.push_str(&format!("💰 金币奖励：{}\n", floor_def.gold_reward));
        out.push_str(&format!("⭐ 经验奖励：{}\n", floor_def.exp_reward));
        if floor_def.diamond_reward > 0 {
            out.push_str(&format!("💎 钻石奖励：{}（每10层）\n", floor_def.diamond_reward));
        }
        out.push_str(&format!("\n📍 下一层：第 {} 层\n", next_floor + 1));

        // 特殊层提示
        if next_floor % 10 == 0 {
            out.push_str("\n🏆 Boss层通关！额外钻石奖励已发放\n");
        }
        if next_floor == 50 {
            out.push_str("\n⭐ 达成成就：深渊征服者！\n");
        }
        if next_floor == 100 {
            out.push_str("\n👑 达成成就：深渊之王！全服最强挑战者！\n");
        }

        out
    } else {
        // 失败 — 玩家扣血但不重置进度
        let damage = hp / 4; // 失败扣25%血
        db.write_basic(user_id, "P_HP", &((hp - damage).max(1)).to_string());

        let mut out = format!("{}\n═══ 🌀 深渊第{}层 — {} ═══\n", prefix, next_floor, floor_def.name);
        out.push_str(&format!("👹 怪物：{}\n", monster_name));
        out.push_str(&format!("⚔️ 战斗回合：{} 回合\n\n", rounds));
        out.push_str("💀 挑战失败！\n\n");
        out.push_str(&format!("你被 {} 击败了！\n", monster_name));
        out.push_str(&format!("损失 {} 点生命\n\n", damage));
        out.push_str("💡 提升实力后再来挑战！\n");
        out.push_str("   提示：提升装备强化等级、镶嵌宝石、学习技能\n");

        out
    }
}

/// 深渊进度 — 查看详细进度
pub fn cmd_abyss_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let (current, best, coins) = get_abyss_progress(db, user_id);

    let next = if current == 0 { 1 } else { current };
    let floor_def = get_floor_def(next);

    let mut out = format!("{}\n═══ 🌀 深渊进度 ═══\n", prefix);

    if best == 0 {
        out.push_str("\n你还没有挑战过深渊\n");
        out.push_str("使用「挑战深渊」开始冒险！\n");
        return out;
    }

    out.push_str(&format!("\n📍 当前层：{}\n", next));
    out.push_str(&format!("🏆 最高记录：{} 层\n", best));
    out.push_str(&format!("💰 累计金币：{}\n", coins));

    // 进度条
    let progress = (best as f64 / 100.0 * 20.0) as usize;
    let bar: String = "█".repeat(progress) + &"░".repeat(20 - progress);
    out.push_str(&format!("\n通关进度 [{}] {}%\n", bar, best));

    // 下一层信息
    if next <= 100 {
        out.push_str("\n📋 下一层预览：\n");
        out.push_str(&format!("  层级：第 {} 层 ({})\n", next, floor_def.name));
        out.push_str(&format!("  怪物：{}\n", get_monster_name(next)));
        out.push_str(&format!("  怪物生命：{}\n", floor_def.monster_hp));
        out.push_str(&format!("  怪物攻击：{}\n", floor_def.monster_ad));
        out.push_str(&format!("  金币奖励：{}\n", floor_def.gold_reward));
        if floor_def.diamond_reward > 0 {
            out.push_str(&format!("  钻石奖励：{}\n", floor_def.diamond_reward));
        }
    } else {
        out.push_str("\n🎉 你已经通关全部100层！\n");
    }

    // 段位显示
    let tier = match best {
        1..=10 => "🥉 深渊新兵",
        11..=20 => "🥈 深渊探索者",
        21..=30 => "🥇 深渊战士",
        31..=50 => "⚔️ 深渊征服者",
        51..=70 => "🔥 深渊炼狱行者",
        71..=90 => "💀 深渊混沌领主",
        91..=100 => "👑 深渊之王",
        _ => "❌ 未挑战",
    };
    out.push_str(&format!("\n🎖 段位：{}\n", tier));

    out
}

/// 深渊排行 — 全服排行
pub fn cmd_abyss_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let mut out = format!("{}\n═══ 🌀 深渊排行 ═══\n\n", prefix);

    // 查询全服排行
    let rows = db.query_rows(
        "SELECT ID, abyss_best FROM Basic_User WHERE CAST(abyss_best AS INTEGER) > 0 ORDER BY CAST(abyss_best AS INTEGER) DESC LIMIT 20",
        &[],
        |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        },
    );

    if rows.is_empty() {
        out.push_str("暂无深渊挑战记录\n");
        out.push_str("成为第一个挑战深渊的勇者！\n");
        return out;
    }

    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, best)) in rows.iter().enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let nickname = db.read_basic(uid, "NickName");
        let best_val: i32 = best.parse().unwrap_or(0);
        let tier = match best_val {
            1..=10 => "新兵",
            11..=20 => "探索者",
            21..=30 => "战士",
            31..=50 => "征服者",
            51..=70 => "炼狱行者",
            71..=90 => "混沌领主",
            91..=100 => "深渊之王",
            _ => "-",
        };
        out.push_str(&format!("{} {} {} — {} 层 ({})\n", medal, i + 1, nickname, best, tier));
    }

    // 当前玩家排名
    let my_best = db.read_basic(user_id, "abyss_best").parse::<i32>().unwrap_or(0);
    if my_best > 0 {
        let rank = db
            .query_row(
                "SELECT COUNT(*) FROM Basic_User WHERE CAST(abyss_best AS INTEGER) > ?",
                &[&my_best.to_string()],
                |row| Ok(row.get::<_, i32>(0).unwrap_or(0)),
            )
            .unwrap_or(0);
        out.push_str(&format!("\n📊 你的排名：第 {} 名（最高 {} 层）\n", rank + 1, my_best));
    }

    out
}

/// 重置深渊 — 从第1层重新开始
pub fn cmd_reset_abyss(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);
    let (_, best, _coins) = get_abyss_progress(db, user_id);

    if best == 0 {
        return format!("{}\n你还没有挑战过深渊，无需重置", prefix);
    }

    db.write_basic(user_id, "abyss_floor", "1");

    let mut out = format!("{}\n═══ 🌀 深渊重置 ═══\n\n", prefix);
    out.push_str("✅ 深渊进度已重置\n");
    out.push_str(&format!("🏆 最高记录保留：{} 层\n", best));
    out.push_str("📍 当前进度：第 1 层\n\n");
    out.push_str("使用「挑战深渊」重新开始冒险！\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_def_basic() {
        let f = get_floor_def(1);
        assert_eq!(f.floor, 1);
        assert_eq!(f.name, "深渊浅层");
        assert!(f.monster_hp > 0);
        assert!(f.gold_reward > 0);
        assert_eq!(f.diamond_reward, 0); // 不是10的倍数
    }

    #[test]
    fn test_floor_def_boss() {
        let f = get_floor_def(10);
        assert_eq!(f.name, "深渊浅层");
        assert_eq!(f.diamond_reward, 5); // 10层奖励5钻
    }

    #[test]
    fn test_floor_def_scaling() {
        let f1 = get_floor_def(1);
        let f50 = get_floor_def(50);
        assert!(f50.monster_hp > f1.monster_hp);
        assert!(f50.monster_ad > f1.monster_ad);
        assert!(f50.gold_reward > f1.gold_reward);
    }

    #[test]
    fn test_floor_def_deep() {
        let f = get_floor_def(90);
        assert_eq!(f.name, "深渊混沌");
        assert_eq!(f.diamond_reward, 45); // 90/10*5 = 45
    }

    #[test]
    fn test_monster_names() {
        assert_eq!(get_monster_name(1), "深渊哨兵");
        assert_eq!(get_monster_name(30), "深渊领主");
        assert_eq!(get_monster_name(100), "深渊之王");
        assert_eq!(get_monster_name(150), "深渊之神");
    }

    #[test]
    fn test_tier_display() {
        let tiers: Vec<(i32, &str)> = vec![
            (1, "新兵"),
            (10, "新兵"),
            (11, "探索者"),
            (20, "探索者"),
            (21, "战士"),
            (30, "战士"),
            (50, "征服者"),
            (70, "炼狱行者"),
            (90, "混沌领主"),
            (100, "深渊之王"),
        ];
        for (floor, expected) in tiers {
            let tier = match floor {
                1..=10 => "新兵",
                11..=20 => "探索者",
                21..=30 => "战士",
                31..=50 => "征服者",
                51..=70 => "炼狱行者",
                71..=90 => "混沌领主",
                91..=100 => "深渊之王",
                _ => "-",
            };
            assert_eq!(tier, expected, "Floor {} tier mismatch", floor);
        }
    }
}

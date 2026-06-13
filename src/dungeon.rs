/// CakeGame 地宫猎敌录系统
/// 来自 ext_dgldl_info 表 — 12 个地宫 BOSS + 掉落表
use crate::core::*;
use crate::db::Database;
use crate::user;
use crate::vip;
use rand::Rng;

/// 地宫 BOSS 信息
struct DungeonBoss {
    name: String,
    drops: Vec<(String, f64)>, // (物品名, 掉落概率 0~1)
}

/// 解析地宫 BOSS 掉落表
/// 格式: "物品名*0.10|物品名*0.03|..."
fn parse_drops(goods: &str) -> Vec<(String, f64)> {
    if goods.trim().is_empty() {
        return Vec::new();
    }
    goods
        .split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.trim().rsplitn(2, '*').collect();
            if parts.len() == 2 {
                let prob_str = parts[0].trim();
                let item_name = parts[1].trim();
                if let Ok(prob) = prob_str.parse::<f64>() {
                    return Some((item_name.to_string(), prob));
                }
            }
            None
        })
        .collect()
}

/// 获取所有地宫 BOSS
fn get_all_bosses(db: &Database) -> Vec<DungeonBoss> {
    db.query_rows("SELECT name, goods FROM ext_dgldl_info", &[], |row| {
        let name: String = row.get(0)?;
        let goods: String = row.get(1).unwrap_or_default();
        Ok(DungeonBoss {
            name,
            drops: parse_drops(&goods),
        })
    })
}

/// 查看地宫 BOSS 列表
pub fn cmd_view_dungeon(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let bosses = get_all_bosses(db);

    if bosses.is_empty() {
        return format!("{}\n地宫暂无 BOSS 信息。", prefix);
    }

    let args = args.trim();

    // 查看特定 BOSS 详情
    if !args.is_empty() {
        for boss in &bosses {
            if boss.name == args || boss.name.contains(args) {
                let mut r = format!("{}\n═══ 地宫BOSS: {} ═══\n掉落物品:", prefix, boss.name);
                if boss.drops.is_empty() {
                    r.push_str("\n  (无掉落)");
                } else {
                    for (item, prob) in &boss.drops {
                        let pct = prob * 100.0;
                        r.push_str(&format!("\n  {} — {:.1}%", item, pct));
                    }
                }
                r.push_str("\n\n发送 '挑战地宫+BOSS名' 进行挑战");
                return r;
            }
        }
        return format!("{}\n找不到地宫 BOSS [{}]。", prefix, args);
    }

    // 列出所有 BOSS
    let mut r = format!("{}\n═══ 地宫猎敌录 ═══", prefix);
    for (i, boss) in bosses.iter().enumerate() {
        let drop_count = boss.drops.len();
        r.push_str(&format!("\n{}. {} ({}种掉落)", i + 1, boss.name, drop_count));
    }
    r.push_str("\n\n发送 '查看地宫+BOSS名' 查看掉落详情");
    r.push_str("\n发送 '挑战地宫+BOSS名' 进行挑战");
    r
}

/// 挑战地宫 BOSS
pub fn cmd_challenge_dungeon(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let boss_name = args.trim();

    if boss_name.is_empty() {
        return format!("{}\n请指定要挑战的 BOSS 名称。\n用法: 挑战地宫+BOSS名", prefix);
    }

    // 查找 BOSS
    let bosses = get_all_bosses(db);
    let boss = bosses
        .iter()
        .find(|b| b.name == boss_name || b.name.contains(boss_name));
    if boss.is_none() {
        return format!("{}\n找不到地宫 BOSS [{}]。", prefix, boss_name);
    }
    let boss = boss.unwrap();

    // 检查用户生命
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再挑战。", prefix);
    }

    // 计算用户战斗力
    let info = user::calc_total_attrs(db, user_id);
    let mut rng = rand::thread_rng();

    // BOSS 属性（基于索引难度递增）
    let boss_idx = bosses.iter().position(|b| b.name == boss.name).unwrap_or(0) as i32;
    let boss_hp = 200 + boss_idx * 150;
    let boss_atk = 30 + boss_idx * 20;
    let boss_def = 10 + boss_idx * 8;

    let mut log = String::new();
    log.push_str(&format!(
        "\n═══ 挑战地宫BOSS: {} ═══\nBOSS 生命: {} | 攻击: {} | 防御: {}",
        boss.name, boss_hp, boss_atk, boss_def
    ));
    log.push_str(&format!(
        "\n你的 攻击:{} | 防御:{} | 生命:{}",
        info.ad + info.ap,
        info.defense + info.magic_res,
        hp_current
    ));

    // 回合制战斗
    let mut user_hp = hp_current;
    let mut boss_hp_left = boss_hp;
    let mut rounds = 0;

    while user_hp > 0 && boss_hp_left > 0 && rounds < 20 {
        rounds += 1;

        // 玩家攻击
        let raw_dmg = (info.ad + info.ap) - boss_def;
        let raw_dmg = if raw_dmg < 1 { 1 } else { raw_dmg };
        let variance = rng.gen_range(-raw_dmg.abs().max(1)..=raw_dmg.abs().max(1));
        let dmg = (raw_dmg + variance).max(1);

        // 暴击判定
        let crit_roll = rng.gen_range(0..100);
        let is_crit = crit_roll < info.crit;
        let final_dmg = if is_crit { dmg * 2 } else { dmg };

        boss_hp_left -= final_dmg;
        if is_crit {
            log.push_str(&format!(
                "\n[{}] 暴击！你对 {} 造成 {} 伤害",
                rounds, boss.name, final_dmg
            ));
        } else {
            log.push_str(&format!("\n[{}] 你对 {} 造成 {} 伤害", rounds, boss.name, final_dmg));
        }

        if boss_hp_left <= 0 {
            break;
        }

        // BOSS 反击
        let boss_raw = boss_atk - (info.defense + info.magic_res);
        let boss_raw = if boss_raw < 1 { 1 } else { boss_raw };
        let boss_var = rng.gen_range(-boss_raw.abs().max(1)..=boss_raw.abs().max(1));
        let boss_dmg = (boss_raw + boss_var).max(1);

        // 闪避判定
        let dodge_roll = rng.gen_range(0..100);
        if dodge_roll < info.dodge {
            log.push_str(&format!("\n[{}] {} 的攻击被你闪避了！", rounds, boss.name));
        } else {
            user_hp -= boss_dmg;
            log.push_str(&format!("\n[{}] {} 对你造成 {} 伤害", rounds, boss.name, boss_dmg));
        }
    }

    // 战斗结果
    if boss_hp_left <= 0 {
        // 胜利 — 掉落物品
        log.push_str(&format!("\n\n✦ 你击败了 {}！✦", boss.name));

        // 掉落判定
        let mut drops_got: Vec<String> = Vec::new();
        for (item_name, prob) in &boss.drops {
            let roll: f64 = rng.gen();
            if roll < *prob {
                // 给予物品
                db.knapsack_add(user_id, item_name, 1);
                drops_got.push(item_name.clone());
            }
        }

        if drops_got.is_empty() {
            log.push_str("\n本次挑战未获得掉落物品，再接再厉！");
        } else {
            log.push_str("\n\n═══ 掉落物品 ═══");
            for item in &drops_got {
                log.push_str(&format!("\n  获得: {}", item));
            }
        }

        // 经验奖励（含VIP加成）
        let exp_reward = 10 + boss_idx * 5;
        let vip_bonus_pct = vip::get_vip_exp_bonus(db, user_id);
        let bonus_exp = if vip_bonus_pct > 0 {
            exp_reward * vip_bonus_pct / 100
        } else {
            0
        };
        let total_exp = exp_reward + bonus_exp;
        let (_, leveled_up) = user::add_experience(db, user_id, total_exp);
        if bonus_exp > 0 {
            log.push_str(&format!("\n经验 +{} (+{}VIP加成)", total_exp, bonus_exp));
        } else {
            log.push_str(&format!("\n经验 +{}", exp_reward));
        }
        if leveled_up {
            log.push_str(" ★ 升级了！");
        }

        // 扣除部分生命（模拟战斗消耗）
        let hp_loss = hp_current - user_hp.max(1);
        if hp_loss > 0 {
            db.write_basic_int(user_id, ITEM_HP_CURRENT, user_hp.max(1));
        }
    } else if user_hp <= 0 {
        // 失败
        log.push_str(&format!("\n\n✗ 你被 {} 击败了...", boss.name));
        db.write_basic_int(user_id, ITEM_HP_CURRENT, 1); // 保留1点生命
        log.push_str("\n生命值降至1，请恢复后再战。");
    } else {
        // 超时
        log.push_str(&format!("\n\n战斗超时！{} 逃走了。", boss.name));
    }

    format!("{}{}", prefix, log)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_drops_basic() {
        let drops = parse_drops("强化石*0.10|生命药水*0.03");
        assert_eq!(drops.len(), 2);
        assert_eq!(drops[0].0, "强化石");
        assert!((drops[0].1 - 0.10).abs() < 1e-10);
        assert_eq!(drops[1].0, "生命药水");
        assert!((drops[1].1 - 0.03).abs() < 1e-10);
    }

    #[test]
    fn test_parse_drops_empty() {
        assert!(parse_drops("").is_empty());
        assert!(parse_drops("  ").is_empty());
    }

    #[test]
    fn test_parse_drops_single() {
        let drops = parse_drops("虚空宝石*0.05");
        assert_eq!(drops.len(), 1);
        assert_eq!(drops[0].0, "虚空宝石");
    }

    #[test]
    fn test_parse_drops_invalid() {
        // No star separator
        let drops = parse_drops("强化石");
        assert!(drops.is_empty());
    }

    #[test]
    fn test_parse_drops_invalid_prob() {
        let drops = parse_drops("强化石*abc");
        assert!(drops.is_empty());
    }

    #[test]
    fn test_parse_drops_zero_prob() {
        let drops = parse_drops("强化石*0.00");
        assert_eq!(drops.len(), 1);
        assert!((drops[0].1 - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_drops_full_prob() {
        let drops = parse_drops("强化石*1.00");
        assert_eq!(drops.len(), 1);
        assert!((drops[0].1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parse_drops_prob_range() {
        let drops = parse_drops("A*0.01|B*0.50|C*0.99");
        assert_eq!(drops.len(), 3);
        for (_, prob) in &drops {
            assert!(*prob >= 0.0 && *prob <= 1.0);
        }
    }

    #[test]
    fn test_parse_drops_with_spaces() {
        let drops = parse_drops(" 强化石 * 0.10 | 生命药水 * 0.03 ");
        assert_eq!(drops.len(), 2);
    }

    #[test]
    fn test_parse_drops_trailing_pipe() {
        let drops = parse_drops("强化石*0.10|");
        assert_eq!(drops.len(), 1);
    }
}

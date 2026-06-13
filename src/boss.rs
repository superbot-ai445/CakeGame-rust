/// CakeGame BOSS系统
/// 来自 ext_sgmonster_info 表 — 野外BOSS
use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;
use crate::vip;
use rand::Rng;

/// 查询所有野外BOSS
fn get_all_bosses(db: &Database) -> Vec<(String, String)> {
    db.query_rows("SELECT Name, goods FROM ext_sgmonster_info", &[], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
        ))
    })
}

/// 查看野外BOSS列表
pub fn cmd_view_boss(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let bosses = get_all_bosses(db);

    if bosses.is_empty() {
        return format!("{}\n暂无野外BOSS信息。", prefix);
    }

    let args = args.trim();

    if !args.is_empty() {
        for (name, goods) in &bosses {
            if name == args || name.contains(args) {
                let mut r = format!("{}\n═══ 野外BOSS: {} ═══", prefix, name);
                if !goods.is_empty() {
                    r.push_str("\n掉落物品:");
                    for entry in goods.split('|') {
                        let parts: Vec<&str> = entry.trim().rsplitn(2, '*').collect();
                        if parts.len() == 2 {
                            let prob: f64 = parts[0].trim().parse().unwrap_or(0.0);
                            r.push_str(&format!("\n  {} — {:.1}%", parts[1].trim(), prob * 100.0));
                        } else {
                            r.push_str(&format!("\n  {}", entry.trim()));
                        }
                    }
                } else {
                    r.push_str("\n(无掉落信息)");
                }
                r.push_str("\n\n发送 '挑战BOSS+BOSS名' 进行挑战");
                return r;
            }
        }
        return format!("{}\n找不到BOSS [{}]。", prefix, args);
    }

    let mut r = format!("{}\n═══ 野外BOSS列表 ═══", prefix);
    for (i, (name, _)) in bosses.iter().enumerate() {
        r.push_str(&format!("\n{}. {}", i + 1, name));
    }
    r.push_str("\n\n发送 '查看BOSS+BOSS名' 查看详情");
    r.push_str("\n发送 '挑战BOSS+BOSS名' 进行挑战");
    r
}

/// 挑战野外BOSS
pub fn cmd_challenge_boss(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let boss_name = args.trim();

    if boss_name.is_empty() {
        return format!("{}\n请指定要挑战的BOSS名称。\n用法: 挑战BOSS+BOSS名", prefix);
    }

    let bosses = get_all_bosses(db);
    let boss = bosses.iter().find(|(n, _)| n == boss_name || n.contains(boss_name));
    if boss.is_none() {
        return format!("{}\n找不到BOSS [{}]。", prefix, boss_name);
    }
    let (name, goods) = boss.unwrap();

    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再挑战。", prefix);
    }

    // 体力检查 (BOSS消耗15体力)
    if let Err(e) = stamina::consume_stamina(user_id, "BOSS", db) {
        return format!("{}\n{}", prefix, e);
    }

    let info = user::calc_total_attrs(db, user_id);
    let mut rng = rand::thread_rng();

    let boss_hp = 500;
    let boss_atk = 80;
    let boss_def = 30;

    let mut log = format!(
        "\n═══ 挑战野外BOSS: {} ═══\nBOSS 生命: {} | 攻击: {} | 防御: {}",
        name, boss_hp, boss_atk, boss_def
    );
    log.push_str(&format!(
        "\n你的 攻击:{} | 防御:{} | 生命:{}",
        info.ad + info.ap,
        info.defense + info.magic_res,
        hp_current
    ));

    let mut user_hp = hp_current;
    let mut boss_hp_left = boss_hp;
    let mut rounds = 0;

    while user_hp > 0 && boss_hp_left > 0 && rounds < 20 {
        rounds += 1;

        let raw_dmg = (info.ad + info.ap - boss_def).max(1);
        let variance = rng.gen_range(-raw_dmg.abs().max(1)..=raw_dmg.abs().max(1));
        let dmg = (raw_dmg + variance).max(1);
        let is_crit = rng.gen_range(0..100) < info.crit;
        let final_dmg = if is_crit { dmg * 2 } else { dmg };

        boss_hp_left -= final_dmg;
        if is_crit {
            log.push_str(&format!("\n[{}] 暴击！你对 {} 造成 {} 伤害", rounds, name, final_dmg));
        } else {
            log.push_str(&format!("\n[{}] 你对 {} 造成 {} 伤害", rounds, name, final_dmg));
        }

        if boss_hp_left <= 0 {
            break;
        }

        let boss_raw = (boss_atk - info.defense - info.magic_res).max(1);
        let boss_var = rng.gen_range(-boss_raw.abs().max(1)..=boss_raw.abs().max(1));
        let boss_dmg = (boss_raw + boss_var).max(1);

        if rng.gen_range(0..100) < info.dodge {
            log.push_str(&format!("\n[{}] {} 的攻击被你闪避了！", rounds, name));
        } else {
            user_hp -= boss_dmg;
            log.push_str(&format!("\n[{}] {} 对你造成 {} 伤害", rounds, name, boss_dmg));
        }
    }

    if boss_hp_left <= 0 {
        log.push_str(&format!("\n\n✦ 你击败了 {}！✦", name));
        crate::achievement::on_boss_kill(db, user_id);

        let mut drops_got: Vec<String> = Vec::new();
        for entry in goods.split('|') {
            let parts: Vec<&str> = entry.trim().rsplitn(2, '*').collect();
            if parts.len() == 2 {
                let prob: f64 = parts[0].trim().parse().unwrap_or(0.0);
                let item_name = parts[1].trim();
                if rng.gen::<f64>() < prob {
                    db.knapsack_add(user_id, item_name, 1);
                    drops_got.push(item_name.to_string());
                }
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

        let exp_reward = 30;
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

        db.write_basic_int(user_id, ITEM_HP_CURRENT, user_hp.max(1));
    } else if user_hp <= 0 {
        log.push_str(&format!("\n\n✗ 你被 {} 击败了...", name));
        db.write_basic_int(user_id, ITEM_HP_CURRENT, 1);
        log.push_str("\n生命值降至1，请恢复后再战。");
    } else {
        log.push_str(&format!("\n\n战斗超时！{} 逃走了。", name));
    }

    format!("{}{}", prefix, log)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_boss_hp_constants() {
        // Boss HP, ATK, DEF should be reasonable
        let boss_hp = 500i32;
        let boss_atk = 80i32;
        let boss_def = 30i32;
        assert!(boss_hp > 0);
        assert!(boss_atk > 0);
        assert!(boss_def > 0);
        assert!(boss_atk > boss_def, "Boss ATK should exceed DEF");
    }

    #[test]
    fn test_damage_calc_floor() {
        // Minimum damage should be at least 1
        let raw_dmg = (100i32 - 200i32).max(1);
        assert_eq!(raw_dmg, 1);
    }

    #[test]
    fn test_drop_parse() {
        let goods = "强化石*0.5|生命药水*0.3";
        let mut items = Vec::new();
        for entry in goods.split('|') {
            let parts: Vec<&str> = entry.trim().rsplitn(2, '*').collect();
            if parts.len() == 2 {
                let prob: f64 = parts[0].trim().parse().unwrap_or(0.0);
                let item_name = parts[1].trim();
                items.push((item_name.to_string(), prob));
            }
        }
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].0, "强化石");
        assert!((items[0].1 - 0.5).abs() < 0.01);
        assert_eq!(items[1].0, "生命药水");
        assert!((items[1].1 - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_drop_parse_empty() {
        let goods = "";
        let items: Vec<_> = goods.split('|').filter(|s| !s.trim().is_empty()).collect();
        assert!(items.is_empty());
    }

    #[test]
    fn test_vip_bonus_calc() {
        let exp_reward = 30i32;
        let vip_bonus_pct = 20i32;
        let bonus = exp_reward * vip_bonus_pct / 100;
        assert_eq!(bonus, 6);
        let total = exp_reward + bonus;
        assert_eq!(total, 36);
    }

    #[test]
    fn test_vip_bonus_zero() {
        let exp_reward = 30i32;
        let vip_bonus_pct = 0i32;
        let bonus = if vip_bonus_pct > 0 {
            exp_reward * vip_bonus_pct / 100
        } else {
            0
        };
        assert_eq!(bonus, 0);
    }
}

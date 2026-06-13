/// CakeGame 战斗统计 & 怪物进化系统
///
/// 数据来源: Ext_Monster_hzxyx 表 (2002条，154个用户，每用户13属性)
/// 功能:
///   1. 战斗统计: 追踪玩家的击杀数、总伤害、死亡次数、PvP胜负
///   2. 怪物进化: 基于玩家击杀数，怪物属性动态增长 (per-user scaling)
///   3. 战斗日志: 最近N场战斗的摘要记录
///
/// 新增指令: 战斗统计, 怪物进化, 战斗日志
use crate::core::ITEM_LOCATION;
use crate::db::Database;
use crate::user;

/// 怪物进化属性 (来自 Ext_Monster_hzxyx)
#[derive(Debug, Clone)]
pub struct MonsterEvolution {
    #[allow(dead_code)]
    pub user_id: String,
    pub hp_bonus: i32,
    pub def_bonus: i32,
    pub mdf_bonus: i32,
    pub ad_bonus: i32,
    pub ap_bonus: i32,
    pub dod_bonus: i32,
    pub hit_bonus: i32,
    pub abhp_bonus: i32,
    pub imd_bonus: i32,
    pub adcb_bonus: i32,
    pub apcb_bonus: i32,
    pub adc_bonus: i32,
    pub apc_bonus: i32,
}

/// 从 Ext_Monster_hzxyx 读取用户的怪物进化数据
pub fn get_monster_evolution(db: &Database, user_id: &str) -> Option<MonsterEvolution> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT MonAttName, MonAttValue, MonDynValue \
             FROM Ext_Monster_hzxyx WHERE Id = ?1",
        )
        .ok()?;
    let rows = stmt
        .query_map(rusqlite::params![user_id], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        })
        .ok()?;

    let mut evo = MonsterEvolution {
        user_id: user_id.to_string(),
        hp_bonus: 0,
        def_bonus: 0,
        mdf_bonus: 0,
        ad_bonus: 0,
        ap_bonus: 0,
        dod_bonus: 0,
        hit_bonus: 0,
        abhp_bonus: 0,
        imd_bonus: 0,
        adcb_bonus: 0,
        apcb_bonus: 0,
        adc_bonus: 0,
        apc_bonus: 0,
    };

    let mut found = false;
    for (att_name, att_value, dyn_value) in rows.flatten() {
        let base: i32 = att_value.parse().unwrap_or(0);
        let dyn_val: i32 = dyn_value.parse().unwrap_or(0);
        let total = base + dyn_val;
        match att_name.as_str() {
            "HP" => evo.hp_bonus = total,
            "DEF" => evo.def_bonus = total,
            "MDF" => evo.mdf_bonus = total,
            "AD" => evo.ad_bonus = total,
            "AP" => evo.ap_bonus = total,
            "DOD" => evo.dod_bonus = total,
            "HIT" => evo.hit_bonus = total,
            "ABHP" => evo.abhp_bonus = total,
            "IMD" => evo.imd_bonus = total,
            "ADCB" => evo.adcb_bonus = total,
            "APCB" => evo.apcb_bonus = total,
            "ADC" => evo.adc_bonus = total,
            "APC" => evo.apc_bonus = total,
            _ => {}
        }
        found = true;
    }

    if found {
        Some(evo)
    } else {
        None
    }
}

/// 计算怪物进化后的属性加成 (基于击杀数缩放)
/// scale_factor = 1.0 + (kill_count / 100) * 0.05, max 2.0x
pub fn calc_evolved_stats(
    base_hp: i32,
    base_ad: i32,
    base_def: i32,
    base_mdf: i32,
    evo: &MonsterEvolution,
    kill_count: i32,
) -> (i32, i32, i32, i32) {
    let scale = (1.0 + (kill_count as f64 / 100.0) * 0.05).min(2.0);
    let evolved_hp = base_hp + (evo.hp_bonus as f64 * scale) as i32;
    let evolved_ad = base_ad + (evo.ad_bonus as f64 * scale) as i32;
    let evolved_def = base_def + (evo.def_bonus as f64 * scale) as i32;
    let evolved_mdf = base_mdf + (evo.mdf_bonus as f64 * scale) as i32;
    (evolved_hp, evolved_ad, evolved_def, evolved_mdf)
}

// ==================== 战斗统计存储 (使用 Global 表) ====================

/// 统计 section 前缀
const STATS_SECTION: &str = "combat_stats";

/// 读取用户的战斗统计值
fn read_stat(db: &Database, user_id: &str, stat: &str) -> i32 {
    let id = format!("{}.{}", user_id, stat);
    db.global_get(STATS_SECTION, &id).parse().unwrap_or(0)
}

/// 写入用户的战斗统计值
fn write_stat(db: &Database, user_id: &str, stat: &str, value: i32) {
    let id = format!("{}.{}", user_id, stat);
    db.global_set(STATS_SECTION, &id, &value.to_string());
}

/// 累加统计值
pub fn inc_stat(db: &Database, user_id: &str, stat: &str, delta: i32) {
    let current = read_stat(db, user_id, stat);
    write_stat(db, user_id, stat, current + delta);
}

/// 记录一次击杀
pub fn record_kill(db: &Database, user_id: &str, monster_name: &str, damage: i32) {
    inc_stat(db, user_id, "kills", 1);
    inc_stat(db, user_id, "total_damage", damage);
    // 更新最高单次伤害
    let max_dmg = read_stat(db, user_id, "max_damage");
    if damage > max_dmg {
        write_stat(db, user_id, "max_damage", damage);
    }
    // 记录击杀的怪物种类
    let kill_id = format!("{}.kill.{}", user_id, monster_name);
    let count: i32 = db.global_get(STATS_SECTION, &kill_id).parse().unwrap_or(0);
    db.global_set(STATS_SECTION, &kill_id, &(count + 1).to_string());
    // 写入最后战斗日志
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let log_entry = format!("{}|{}|{}|击杀", now, monster_name, damage);
    append_combat_log(db, user_id, &log_entry);
}

/// 记录一次死亡
pub fn record_death(db: &Database, user_id: &str, killer: &str) {
    inc_stat(db, user_id, "deaths", 1);
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let log_entry = format!("{}|{}|0|阵亡", now, killer);
    append_combat_log(db, user_id, &log_entry);
}

/// 记录PvP结果
pub fn record_pvp(db: &Database, winner_id: &str, loser_id: &str, damage: i32) {
    inc_stat(db, winner_id, "pvp_wins", 1);
    inc_stat(db, winner_id, "total_damage", damage);
    inc_stat(db, loser_id, "pvp_losses", 1);
    let now = chrono::Local::now().format("%m-%d %H:%M").to_string();
    let log_win = format!("{}|PvP|{}|胜利", now, damage);
    let log_lose = format!("{}|PvP|{}|失败", now, damage);
    append_combat_log(db, winner_id, &log_win);
    append_combat_log(db, loser_id, &log_lose);
}

/// 追加战斗日志 (保留最近20条)
fn append_combat_log(db: &Database, user_id: &str, entry: &str) {
    let log_id = format!("{}.log", user_id);
    let existing = db.global_get(STATS_SECTION, &log_id);
    let mut logs: Vec<String> = if existing.is_empty() {
        Vec::new()
    } else {
        existing.split('\n').map(|s| s.to_string()).collect()
    };
    logs.push(entry.to_string());
    // 保留最近20条
    if logs.len() > 20 {
        logs = logs[logs.len() - 20..].to_vec();
    }
    db.global_set(STATS_SECTION, &log_id, &logs.join("\n"));
}

/// 获取怪物击杀计数
fn get_monster_kill_count(db: &Database, user_id: &str, monster_name: &str) -> i32 {
    let kill_id = format!("{}.kill.{}", user_id, monster_name);
    db.global_get(STATS_SECTION, &kill_id).parse().unwrap_or(0)
}

// ==================== 指令实现 ====================

/// 战斗统计 — 查看个人战斗数据总览
pub fn cmd_combat_stats(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let kills = read_stat(db, user_id, "kills");
    let deaths = read_stat(db, user_id, "deaths");
    let total_damage = read_stat(db, user_id, "total_damage");
    let max_damage = read_stat(db, user_id, "max_damage");
    let pvp_wins = read_stat(db, user_id, "pvp_wins");
    let pvp_losses = read_stat(db, user_id, "pvp_losses");
    let kd_ratio = if deaths > 0 {
        format!("{:.1}", kills as f64 / deaths as f64)
    } else {
        "∞".to_string()
    };

    let mut r = format!(
        "{}\n╔═══════════════════════╗\n║  ⚔️ 战斗统计  ║\n╚═══════════════════════╝\n\n",
        prefix
    );

    r.push_str(&format!(
        "  📊 PVE 战绩\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         🗡️ 击杀总数: {}\n\
         💀 死亡次数: {}\n\
         📈 K/D 比率: {}\n\
         💥 总伤害量: {}\n\
         🔥 最高单伤: {}\n\n",
        kills, deaths, kd_ratio, total_damage, max_damage
    ));

    let pvp_rate = if pvp_wins + pvp_losses > 0 {
        format!("{:.1}%", pvp_wins as f64 / (pvp_wins + pvp_losses) as f64 * 100.0)
    } else {
        "暂无".to_string()
    };

    r.push_str(&format!(
        "  🏆 PVP 战绩\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         ✅ 胜利: {}  ❌ 失败: {}\n\
         📊 胜率: {}\n\n",
        pvp_wins, pvp_losses, pvp_rate
    ));

    r.push_str("💡 发送「战斗日志」查看最近战斗记录\n");
    r.push_str("💡 发送「怪物进化」查看怪物进化状态");

    r
}

/// 战斗日志 — 查看最近的战斗记录
pub fn cmd_combat_log(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let log_id = format!("{}.log", user_id);
    let log_data = db.global_get(STATS_SECTION, &log_id);

    let mut r = format!(
        "{}\n╔═══════════════════════╗\n║  📜 战斗日志  ║\n╚═══════════════════════╝\n\n",
        prefix
    );

    if log_data.is_empty() {
        r.push_str("  暂无战斗记录\n\n");
        r.push_str("💡 去战斗吧！发送「搜索怪物+名称」锁定目标，然后「攻击」开始战斗");
    } else {
        let entries: Vec<&str> = log_data.split('\n').collect();
        r.push_str(&format!("  最近 {} 场战斗:\n\n", entries.len()));

        for (i, entry) in entries.iter().rev().enumerate() {
            let parts: Vec<&str> = entry.split('|').collect();
            if parts.len() >= 4 {
                let time = parts[0];
                let target = parts[1];
                let dmg = parts[2];
                let result = parts[3];
                let icon = if result == "击杀" {
                    "⚔️"
                } else if result == "阵亡" {
                    "💀"
                } else if result == "胜利" {
                    "🏆"
                } else {
                    "❌"
                };
                r.push_str(&format!("  {} #{} [{}] {} 伤害:{}\n", icon, i + 1, time, target, dmg));
            }
        }
    }

    r
}

/// 怪物进化 — 查看当前地图怪物的进化状态
pub fn cmd_monster_evolution(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n请先注册后再查看！", prefix);
    }

    let location = db.read_basic(user_id, ITEM_LOCATION);

    let mut r = format!(
        "{}\n╔═══════════════════════╗\n║  🧬 怪物进化  ║\n╚═══════════════════════╝\n\n",
        prefix
    );

    // 读取用户的怪物进化数据
    let evo = get_monster_evolution(db, user_id);

    if let Some(evo) = &evo {
        r.push_str(&format!(
            "  📍 当前地图: {}\n\n\
             🧬 怪物进化加成 (基于您的战斗历史):\n\
             ━━━━━━━━━━━━━━━━━━━━\n",
            location
        ));

        let attrs = [
            ("❤️ HP", evo.hp_bonus),
            ("⚔️ 物攻", evo.ad_bonus),
            ("🔮 魔攻", evo.ap_bonus),
            ("🛡️ 防御", evo.def_bonus),
            ("🔰 魔抗", evo.mdf_bonus),
            ("🎯 命中", evo.hit_bonus),
            ("💨 闪避", evo.dod_bonus),
            ("💉 吸血", evo.abhp_bonus),
            ("🛡️ 免伤", evo.imd_bonus),
        ];

        for (name, val) in &attrs {
            if *val != 0 {
                r.push_str(&format!("    {} +{}\n", name, val));
            }
        }

        // 显示当前地图怪物的进化后属性
        if let Some(map) = db.map_get(&location) {
            r.push_str(&format!("\n  📍 {} 怪物进化状态:\n", location));
            r.push_str("  ━━━━━━━━━━━━━━━━━━━━\n");

            for monster_name in &map.monsters {
                let kill_count = get_monster_kill_count(db, user_id, monster_name);
                let scale = (1.0 + (kill_count as f64 / 100.0) * 0.05).min(2.0);

                if let Some(monster) = db.monster_get(monster_name) {
                    let (evo_hp, evo_ad, evo_def, evo_mdf) = calc_evolved_stats(
                        monster.hp,
                        monster.ad,
                        monster.defense,
                        monster.magic_resistance,
                        evo,
                        kill_count,
                    );

                    let level = if scale >= 2.0 {
                        "🔴 极进化"
                    } else if scale >= 1.5 {
                        "🟠 高进化"
                    } else if scale >= 1.2 {
                        "🟡 中进化"
                    } else if scale > 1.0 {
                        "🟢 低进化"
                    } else {
                        "⚪ 未进化"
                    };

                    r.push_str(&format!("\n    {} {} (击杀:{}次)\n", level, monster_name, kill_count));
                    r.push_str(&format!(
                        "      HP:{}→{} AD:{}→{} DEF:{}→{} MDF:{}→{}\n",
                        monster.hp,
                        evo_hp,
                        monster.ad,
                        evo_ad,
                        monster.defense,
                        evo_def,
                        monster.magic_resistance,
                        evo_mdf
                    ));
                }
            }
        }

        r.push_str("\n💡 怪物会随着您的击杀次数逐渐进化变强！\n");
        r.push_str("💡 每击杀100次，怪物进化幅度+5%，最高2倍\n");
    } else {
        r.push_str(&format!("  📍 当前地图: {}\n\n", location));
        r.push_str("  您的战斗历史中暂无怪物进化数据\n\n");
        r.push_str("💡 击杀怪物后，怪物会根据您的战斗历史逐渐进化\n");
        r.push_str("💡 进化后的怪物更强大，但击败后获得更多奖励！\n");
    }

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_evolved_stats_no_bonus() {
        let evo = MonsterEvolution {
            user_id: "test".to_string(),
            hp_bonus: 0,
            def_bonus: 0,
            mdf_bonus: 0,
            ad_bonus: 0,
            ap_bonus: 0,
            dod_bonus: 0,
            hit_bonus: 0,
            abhp_bonus: 0,
            imd_bonus: 0,
            adcb_bonus: 0,
            apcb_bonus: 0,
            adc_bonus: 0,
            apc_bonus: 0,
        };
        let (hp, ad, def, mdf) = calc_evolved_stats(100, 20, 10, 5, &evo, 50);
        assert_eq!(hp, 100);
        assert_eq!(ad, 20);
        assert_eq!(def, 10);
        assert_eq!(mdf, 5);
    }

    #[test]
    fn test_calc_evolved_stats_with_bonus() {
        let evo = MonsterEvolution {
            user_id: "test".to_string(),
            hp_bonus: 100,
            def_bonus: 20,
            mdf_bonus: 10,
            ad_bonus: 30,
            ap_bonus: 0,
            dod_bonus: 0,
            hit_bonus: 0,
            abhp_bonus: 0,
            imd_bonus: 0,
            adcb_bonus: 0,
            apcb_bonus: 0,
            adc_bonus: 0,
            apc_bonus: 0,
        };
        // kill_count=100 -> scale = 1.0 + (100/100)*0.05 = 1.05
        let (hp, ad, def, mdf) = calc_evolved_stats(100, 20, 10, 5, &evo, 100);
        assert_eq!(hp, 100 + (100.0 * 1.05) as i32); // 205
        assert_eq!(ad, 20 + (30.0 * 1.05) as i32); // 51
        assert_eq!(def, 10 + (20.0 * 1.05) as i32); // 31
        assert_eq!(mdf, 5 + (10.0 * 1.05) as i32); // 15
    }

    #[test]
    fn test_calc_evolved_stats_max_scale() {
        let evo = MonsterEvolution {
            user_id: "test".to_string(),
            hp_bonus: 100,
            def_bonus: 0,
            mdf_bonus: 0,
            ad_bonus: 0,
            ap_bonus: 0,
            dod_bonus: 0,
            hit_bonus: 0,
            abhp_bonus: 0,
            imd_bonus: 0,
            adcb_bonus: 0,
            apcb_bonus: 0,
            adc_bonus: 0,
            apc_bonus: 0,
        };
        // kill_count=5000 -> scale = min(1.0 + 250*0.05, 2.0) = 2.0
        let (hp, _, _, _) = calc_evolved_stats(100, 20, 10, 5, &evo, 5000);
        assert_eq!(hp, 100 + (100.0 * 2.0) as i32); // 300
    }
}

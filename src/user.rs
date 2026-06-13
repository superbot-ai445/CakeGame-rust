/// CakeGame 用户系统
/// 处理用户注册、属性读写、职业核对等
use crate::core::*;
use crate::db::Database;

/// 核对用户职业属性（读取职业配置，写入用户基础属性）
pub fn verify_occupation_attrs(db: &Database, user_id: &str) {
    let occupation = db.read_basic(user_id, ITEM_OCCUPATION);
    if occupation.is_empty() {
        return;
    }
    if let Some(occ) = db.occupation_get(&occupation) {
        db.write_basic_int(user_id, ITEM_HP, occ.hp);
        db.write_basic_int(user_id, ITEM_MP, occ.mp);
        db.write_basic_int(user_id, ITEM_AD, occ.ad);
        db.write_basic_int(user_id, ITEM_AP, occ.ap);
        db.write_basic_int(user_id, ITEM_DEFENSE, occ.defense);
        db.write_basic_int(user_id, ITEM_HIT, occ.hit);
        db.write_basic_int(user_id, ITEM_DODGE, occ.dodge);
        db.write_basic_int(user_id, ITEM_CRIT, occ.crit);
        db.write_basic_int(user_id, ITEM_ABSORB_HP, occ.absorb_hp);
        db.write_basic_int(user_id, ITEM_AD_PTV, occ.adptv);
        db.write_basic_int(user_id, ITEM_AD_PTR, occ.adptr);
        db.write_basic_int(user_id, ITEM_AP_PTV, occ.apptv);
        db.write_basic_int(user_id, ITEM_AP_PTR, occ.apptv);
        db.write_basic_int(user_id, ITEM_IMMUNE, occ.immune_damage);
    }
}

/// 套装加成结果
pub struct SuitBonus {
    pub hp: i32,
    pub mp: i32,
    pub defense: i32,
    pub magic: i32,
    pub ad: i32,
    pub ap: i32,
    pub hit: i32,
    pub dodge: i32,
    pub crit: i32,
    pub absorb_hp: i32,
    pub adptv: i32,
    pub adptr: i32,
    pub apptr: i32,
    pub apptv: i32,
    pub immune: i32,
}

/// 计算套装加成（检查 Equip_Combined 表，穿齐一套自动加成）
pub fn calc_suit_bonuses(db: &Database, user_id: &str) -> SuitBonus {
    let mut bonus = SuitBonus {
        hp: 0,
        mp: 0,
        defense: 0,
        magic: 0,
        ad: 0,
        ap: 0,
        hit: 0,
        dodge: 0,
        crit: 0,
        absorb_hp: 0,
        adptv: 0,
        adptr: 0,
        apptr: 0,
        apptv: 0,
        immune: 0,
    };

    // 获取已装备的物品名集合
    let equips = db.equip_all(user_id);
    let equipped_names: std::collections::HashSet<String> = equips.iter().map(|e| e.name.clone()).collect();

    if equipped_names.is_empty() {
        return bonus;
    }

    // 从 Equip_Combined 获取所有套装→装备映射
    let rows: Vec<(String, String)> = db.query_rows("SELECT SuitName, EquipName FROM Equip_Combined", &[], |row| {
        let suit: String = row.get(0).unwrap_or_default();
        let equip: String = row.get(1).unwrap_or_default();
        Ok((suit, equip))
    });

    // 按套装名分组
    let mut suit_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (suit_name, equip_name) in rows {
        suit_map.entry(suit_name).or_default().push(equip_name);
    }

    // 检查每个套装是否全部穿齐
    for (suit_name, required_equips) in &suit_map {
        let all_equipped = required_equips.iter().all(|e| equipped_names.contains(e));
        if !all_equipped || required_equips.is_empty() {
            continue;
        }

        // 从 Config_Suit 读取加成
        if let Ok(()) = db.query_row(
            "SELECT Add_HP, Add_MP, Add_Defense, Add_Magic, Add_AD, Add_AP, \
             Add_Hit, Add_Dodge, Add_Crit, Add_AbsorbHP, \
             Add_ADPTV, Add_ADPTR, Add_APPTR, Add_APPTV, Add_ImmuneDamage \
             FROM Config_Suit WHERE Name = ?1",
            &[suit_name.as_str()],
            |row| {
                let hp: i32 = row.get::<_, String>(0).unwrap_or_default().parse().unwrap_or(0);
                let mp: i32 = row.get::<_, String>(1).unwrap_or_default().parse().unwrap_or(0);
                let def: i32 = row.get::<_, String>(2).unwrap_or_default().parse().unwrap_or(0);
                let mag: i32 = row.get::<_, String>(3).unwrap_or_default().parse().unwrap_or(0);
                let ad: i32 = row.get::<_, String>(4).unwrap_or_default().parse().unwrap_or(0);
                let ap: i32 = row.get::<_, String>(5).unwrap_or_default().parse().unwrap_or(0);
                let hit: i32 = row.get::<_, String>(6).unwrap_or_default().parse().unwrap_or(0);
                let dodge: i32 = row.get::<_, String>(7).unwrap_or_default().parse().unwrap_or(0);
                let crit: i32 = row.get::<_, String>(8).unwrap_or_default().parse().unwrap_or(0);
                let absorb: i32 = row.get::<_, String>(9).unwrap_or_default().parse().unwrap_or(0);
                let adptv: i32 = row.get::<_, String>(10).unwrap_or_default().parse().unwrap_or(0);
                let adptr: i32 = row.get::<_, String>(11).unwrap_or_default().parse().unwrap_or(0);
                let apptr: i32 = row.get::<_, String>(12).unwrap_or_default().parse().unwrap_or(0);
                let apptv: i32 = row.get::<_, String>(13).unwrap_or_default().parse().unwrap_or(0);
                let immune: i32 = row.get::<_, String>(14).unwrap_or_default().parse().unwrap_or(0);
                bonus.hp += hp;
                bonus.mp += mp;
                bonus.defense += def;
                bonus.magic += mag;
                bonus.ad += ad;
                bonus.ap += ap;
                bonus.hit += hit;
                bonus.dodge += dodge;
                bonus.crit += crit;
                bonus.absorb_hp += absorb;
                bonus.adptv += adptv;
                bonus.adptr += adptr;
                bonus.apptr += apptr;
                bonus.apptv += apptv;
                bonus.immune += immune;
                Ok(())
            },
        ) {}
    }

    bonus
}

/// 计算用户生命上限（基础 + 装备加成）
pub fn calc_hp_max(db: &Database, user_id: &str) -> i32 {
    let base: i32 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(0);
    let sys = db.get_system_base_attrs();
    let equips = db.equip_all(user_id);
    let equip_bonus: i32 = equips.iter().map(|e| e.add_hp).sum();
    base + sys.0 + equip_bonus
}

/// 计算用户魔法上限
pub fn calc_mp_max(db: &Database, user_id: &str) -> i32 {
    let base: i32 = db.read_basic(user_id, ITEM_MP).parse().unwrap_or(0);
    let sys = db.get_system_base_attrs();
    let equips = db.equip_all(user_id);
    let equip_bonus: i32 = equips.iter().map(|e| e.add_mp).sum();
    base + sys.1 + equip_bonus
}

/// 计算用户总属性（基础 + 装备）
pub fn calc_total_attrs(db: &Database, user_id: &str) -> UserInfo {
    let mut info = UserInfo {
        id: user_id.to_string(),
        name: db.read_basic(user_id, ITEM_NAME),
        level: db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1),
        occupation: db.read_basic(user_id, ITEM_OCCUPATION),
        location: db.read_basic(user_id, ITEM_LOCATION),
        target: db.read_basic(user_id, ITEM_TARGET),
        task: db.read_basic(user_id, ITEM_TASK),
        guild: db.read_basic(user_id, ITEM_GUILD),
        hp: db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0),
        mp: db.read_basic(user_id, ITEM_MP_CURRENT).parse().unwrap_or(0),
        exp: db.read_basic(user_id, ITEM_EXP).parse().unwrap_or(0),
        exp_need: db.read_basic(user_id, ITEM_EXP_NEED).parse().unwrap_or(100),
        gold: db.read_currency(user_id, CURRENCY_GOLD),
        diamond: db.read_currency(user_id, CURRENCY_DIAMOND),
        ..Default::default()
    };

    // 基础属性（职业基础 + 系统基础）
    let sys = db.get_system_base_attrs();
    let base_hp: i32 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(0) + sys.0;
    let base_mp: i32 = db.read_basic(user_id, ITEM_MP).parse().unwrap_or(0) + sys.1;
    let base_ad: i32 = db.read_basic(user_id, ITEM_AD).parse().unwrap_or(0) + sys.2;
    let base_ap: i32 = db.read_basic(user_id, ITEM_AP).parse().unwrap_or(0) + sys.3;
    let base_def: i32 = db.read_basic(user_id, ITEM_DEFENSE).parse().unwrap_or(0) + sys.4;
    let base_mres: i32 = db.read_basic(user_id, ITEM_MAGIC_RES).parse().unwrap_or(0) + sys.8;
    let base_hit: i32 = db.read_basic(user_id, ITEM_HIT).parse().unwrap_or(0) + sys.5;
    let base_dodge: i32 = db.read_basic(user_id, ITEM_DODGE).parse().unwrap_or(0) + sys.6;
    let base_crit: i32 = db.read_basic(user_id, ITEM_CRIT).parse().unwrap_or(0) + sys.7;
    let base_absorb: i32 = db.read_basic(user_id, ITEM_ABSORB_HP).parse().unwrap_or(0) + sys.9;
    let base_adptv: i32 = db.read_basic(user_id, ITEM_AD_PTV).parse().unwrap_or(0) + sys.11;
    let base_adptr: i32 = db.read_basic(user_id, ITEM_AD_PTR).parse().unwrap_or(0) + sys.12;
    let base_apptv: i32 = db.read_basic(user_id, ITEM_AP_PTV).parse().unwrap_or(0) + sys.13;
    let base_apptr: i32 = db.read_basic(user_id, ITEM_AP_PTR).parse().unwrap_or(0) + sys.14;
    let base_immune: i32 = db.read_basic(user_id, ITEM_IMMUNE).parse().unwrap_or(0) + sys.10;

    // 装备加成
    let equips = db.equip_all(user_id);
    let eq_hp: i32 = equips.iter().map(|e| e.add_hp).sum();
    let eq_mp: i32 = equips.iter().map(|e| e.add_mp).sum();
    let eq_ad: i32 = equips.iter().map(|e| e.add_ad).sum();
    let eq_ap: i32 = equips.iter().map(|e| e.add_ap).sum();
    let eq_def: i32 = equips.iter().map(|e| e.add_defense).sum();
    let eq_mres: i32 = equips.iter().map(|e| e.add_magic).sum();
    let eq_hit: i32 = equips.iter().map(|e| e.add_hit).sum();
    let eq_dodge: i32 = equips.iter().map(|e| e.add_dodge).sum();
    let eq_crit: i32 = equips.iter().map(|e| e.add_crit).sum();
    let eq_absorb: i32 = equips.iter().map(|e| e.add_absorb_hp).sum();
    let eq_adptv: i32 = equips.iter().map(|e| e.add_adptv).sum();
    let eq_adptr: i32 = equips.iter().map(|e| e.add_adptr).sum();
    let eq_apptv: i32 = equips.iter().map(|e| e.add_apptr).sum();
    let eq_apptr: i32 = equips.iter().map(|e| e.add_apptv).sum();
    let eq_immune: i32 = equips.iter().map(|e| e.add_immune_damage).sum();

    // 锻造/训练加成
    let train_hp: i32 = db.read_user_data(user_id, "training_hp_bonus").parse().unwrap_or(0);
    let train_mp: i32 = db.read_user_data(user_id, "training_mp_bonus").parse().unwrap_or(0);
    let train_ad: i32 = db.read_user_data(user_id, "training_ad_bonus").parse().unwrap_or(0);
    let train_ap: i32 = db.read_user_data(user_id, "training_ap_bonus").parse().unwrap_or(0);

    // 套装加成（穿齐一套自动激活）
    let suit = calc_suit_bonuses(db, user_id);

    // 增益/减益加成（DynamicAttributes_Register 中的活跃buff）
    let (buff_hp, buff_mp, buff_ad, buff_ap, buff_def, buff_mres, buff_hit, buff_absorb) =
        crate::buff::calc_buff_bonuses(db, user_id);

    // 称号加成
    let (title_hp, title_mp, title_ad, title_ap, title_def, title_mres) = crate::title::get_title_bonuses(db, user_id);

    // 装备类型加成（重甲/皮甲/板甲/布甲）
    let armor_bonus = crate::armor_type::calc_armor_type_bonuses(db, user_id);

    // GM属性调整加成
    let (
        gm_hp,
        gm_mp,
        gm_ad,
        gm_ap,
        gm_def,
        gm_mres,
        gm_hit,
        gm_dodge,
        gm_crit,
        gm_absorb,
        gm_adptv,
        gm_apptv,
        gm_immune,
    ) = crate::gm_adjust::calc_gm_adjustments(db, user_id);

    // 灵兽出战加成
    let (beast_hp, beast_ad, beast_def, beast_mdf) = if let Some((_beast_name, b_hp, b_ad, b_def, b_mdf, _skill)) =
        crate::beast::get_active_beast_bonus(db, user_id)
    {
        (b_hp, b_ad, b_def, b_mdf)
    } else {
        (0, 0, 0, 0)
    };

    // 公会科技加成
    let guild_tech = crate::guild_skill::get_guild_tech_bonus(db, user_id);
    let gt_hp = guild_tech.get("hp").copied().unwrap_or(0.0) as i32;
    let gt_mp = guild_tech.get("mp").copied().unwrap_or(0.0) as i32;
    let gt_ad = guild_tech.get("ad").copied().unwrap_or(0.0) as i32;
    let gt_ap = guild_tech.get("ap").copied().unwrap_or(0.0) as i32;
    let gt_def = guild_tech.get("defense").copied().unwrap_or(0.0) as i32;
    let gt_mdf = guild_tech.get("magic_res").copied().unwrap_or(0.0) as i32;
    let gt_hit = guild_tech.get("hit").copied().unwrap_or(0.0) as i32;
    let gt_dodge = guild_tech.get("dodge").copied().unwrap_or(0.0) as i32;
    let gt_crit = guild_tech.get("crit").copied().unwrap_or(0.0) as i32;
    let gt_absorb = guild_tech.get("absorb_hp").copied().unwrap_or(0.0) as i32;

    info.hp_max = base_hp + eq_hp + train_hp + suit.hp + buff_hp + title_hp + gm_hp + armor_bonus.hp + beast_hp + gt_hp;
    info.mp_max = base_mp + eq_mp + train_mp + suit.mp + buff_mp + title_mp + gm_mp + armor_bonus.mp + gt_mp;
    info.ad = base_ad + eq_ad + train_ad + suit.ad + buff_ad + title_ad + gm_ad + armor_bonus.ad + beast_ad + gt_ad;
    info.ap = base_ap + eq_ap + train_ap + suit.ap + buff_ap + title_ap + gm_ap + armor_bonus.ap + gt_ap;
    info.defense =
        base_def + eq_def + suit.defense + buff_def + title_def + gm_def + armor_bonus.defense + beast_def + gt_def;
    info.magic_res = base_mres
        + eq_mres
        + suit.magic
        + buff_mres
        + title_mres
        + gm_mres
        + armor_bonus.magic_res
        + beast_mdf
        + gt_mdf;
    info.hit = base_hit + eq_hit + suit.hit + buff_hit + gm_hit + armor_bonus.hit + gt_hit;
    info.dodge = base_dodge + eq_dodge + suit.dodge + gm_dodge + armor_bonus.dodge + gt_dodge;
    info.crit = base_crit + eq_crit + suit.crit + gm_crit + armor_bonus.crit + gt_crit;
    info.absorb_hp = base_absorb + eq_absorb + suit.absorb_hp + buff_absorb + gm_absorb + gt_absorb;
    info.ad_ptv = base_adptv + eq_adptv + suit.adptv + gm_adptv;
    info.ad_ptr = base_adptr + eq_adptr + suit.adptr;
    info.ap_ptv = base_apptv + eq_apptv + suit.apptr + gm_apptv;
    info.ap_ptr = base_apptr + eq_apptr + suit.apptv;
    info.immune = base_immune + eq_immune + suit.immune + gm_immune;

    // 战斗风格加成（百分比加成，基于已有的总属性）
    let style_bonuses = crate::combat_style::get_style_bonuses(db, user_id);
    for &(attr, pct) in &style_bonuses {
        let bonus = match attr {
            "AD" => {
                let b = info.ad * pct / 100;
                info.ad += b;
                b
            }
            "AP" => {
                let b = info.ap * pct / 100;
                info.ap += b;
                b
            }
            "Defense" => {
                let b = info.defense * pct / 100;
                info.defense += b;
                b
            }
            "MagicResistance" => {
                let b = info.magic_res * pct / 100;
                info.magic_res += b;
                b
            }
            "Hit" => {
                let b = info.hit * pct / 100;
                info.hit += b;
                b
            }
            "Dodge" => {
                let b = info.dodge * pct / 100;
                info.dodge += b;
                b
            }
            "Crit" => {
                let b = info.crit * pct / 100;
                info.crit += b;
                b
            }
            "AbsorbHP" => {
                let b = info.absorb_hp * pct / 100;
                info.absorb_hp += b;
                b
            }
            _ => 0,
        };
        let _ = bonus; // suppress unused warning
    }

    // 护盾值（从 Shield_Register 读取）
    if let Some((shield_val, _, _, _)) = db.get_user_shield(user_id) {
        info.shield = shield_val;
    }

    // 确保当前生命/魔法不超过上限
    if info.hp > info.hp_max {
        info.hp = info.hp_max;
    }
    if info.mp > info.mp_max {
        info.mp = info.mp_max;
    }

    info
}

/// 虚弱持续时间（秒）: 死亡后5分钟内处于虚弱状态
pub const WEAKNESS_DURATION_SECS: i64 = 300;

/// 检查用户是否虚弱（死亡后有冷却时间）
/// 返回剩余虚弱秒数，0表示不虚弱
pub fn check_weakness(db: &Database, user_id: &str) -> i64 {
    let weak_time = db.read_user_data(user_id, "state.weakness_time");
    if weak_time.is_empty() {
        return 0;
    }
    let death_time = chrono::NaiveDateTime::parse_from_str(&weak_time, "%Y-%m-%d %H:%M:%S")
        .unwrap_or_else(|_| chrono::Local::now().naive_local());
    let now = chrono::Local::now().naive_local();
    let elapsed = (now - death_time).num_seconds();
    let remaining = WEAKNESS_DURATION_SECS - elapsed;
    if remaining > 0 {
        remaining
    } else {
        0
    }
}

/// 虚弱伤害倍率（虚弱时攻击力降低30%）
pub const WEAKNESS_DAMAGE_MULT: f64 = 0.7;

/// 查看虚弱状态 — 显示死亡惩罚倒计时和影响
pub fn cmd_weakness_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = get_msg_prefix(db, user_id);
    let remaining = check_weakness(db, user_id);
    if remaining == 0 {
        return format!("{}\n💪 状态: 正常\n您目前没有虚弱效果，可以全力战斗！", prefix);
    }
    let minutes = remaining / 60;
    let seconds = remaining % 60;
    format!(
        "{}\n💀 === 虚弱状态 ===\n\
         ⚠️ 您处于虚弱中（最近一次战斗死亡）\n\
         ⏱️ 剩余时间: {}分{}秒\n\
         📉 战斗惩罚: 攻击力降低30%\n\
         💡 虚弱期间攻击怪物和玩家都会受到减益\n\
         🕐 等待虚弱结束后再挑战更稳妥",
        prefix, minutes, seconds
    )
}

/// 经验增加（自动升级）
pub fn add_experience(db: &Database, user_id: &str, amount: i32) -> (i32, bool) {
    let mut exp: i64 = db.read_basic(user_id, ITEM_EXP).parse().unwrap_or(0);
    let mut exp_need: i64 = db.read_basic(user_id, ITEM_EXP_NEED).parse().unwrap_or(100);
    let mut level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let mut leveled = false;

    exp += amount as i64;

    while exp >= exp_need {
        exp -= exp_need;
        level += 1;
        leveled = true;
        // 经验需求递增（简单公式：每级 * 1.5）
        exp_need = (exp_need as f64 * 1.5) as i64;
    }

    db.write_basic(user_id, ITEM_EXP, &exp.to_string());
    db.write_basic(user_id, ITEM_EXP_NEED, &exp_need.to_string());
    db.write_basic_int(user_id, ITEM_LEVEL, level);

    if leveled {
        crate::achievement::on_level_up(db, user_id, level);
    }

    (level, leveled)
}

/// 获取消息前缀（通常是昵称 + 换行）
pub fn get_msg_prefix(db: &Database, user_id: &str) -> String {
    let name = db.read_basic(user_id, ITEM_NAME);
    if name.is_empty() {
        user_id.to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weakness_constants() {
        assert_eq!(WEAKNESS_DURATION_SECS, 300);
        assert_eq!(WEAKNESS_DAMAGE_MULT, 0.7);
    }

    #[test]
    fn test_suit_bonus_struct() {
        let bonus = SuitBonus {
            hp: 100,
            mp: 50,
            defense: 30,
            magic: 20,
            ad: 40,
            ap: 25,
            hit: 10,
            dodge: 5,
            crit: 15,
            absorb_hp: 8,
            adptv: 12,
            adptr: 6,
            apptr: 7,
            apptv: 9,
            immune: 3,
        };
        assert_eq!(bonus.hp, 100);
        assert_eq!(bonus.mp, 50);
        assert_eq!(bonus.defense, 30);
        assert_eq!(bonus.magic, 20);
        assert_eq!(bonus.ad, 40);
        assert_eq!(bonus.ap, 25);
        assert_eq!(bonus.hit, 10);
        assert_eq!(bonus.dodge, 5);
        assert_eq!(bonus.crit, 15);
        assert_eq!(bonus.absorb_hp, 8);
        assert_eq!(bonus.adptv, 12);
        assert_eq!(bonus.adptr, 6);
        assert_eq!(bonus.apptr, 7);
        assert_eq!(bonus.apptv, 9);
        assert_eq!(bonus.immune, 3);
    }

    #[test]
    fn test_suit_bonus_all_zero() {
        let bonus = SuitBonus {
            hp: 0,
            mp: 0,
            defense: 0,
            magic: 0,
            ad: 0,
            ap: 0,
            hit: 0,
            dodge: 0,
            crit: 0,
            absorb_hp: 0,
            adptv: 0,
            adptr: 0,
            apptr: 0,
            apptv: 0,
            immune: 0,
        };
        assert_eq!(bonus.hp, 0);
        assert_eq!(bonus.immune, 0);
    }
}

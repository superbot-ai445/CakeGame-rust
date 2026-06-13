use crate::achievement;
use crate::battle_archive;
/// CakeGame 战斗系统
/// PVE 战斗：攻击怪物、伤害计算、怪物反击、奖励发放
use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;
use crate::vip;
use crate::world_event;
use rand::Rng;

/// 战斗日志条目
#[derive(Debug, Clone)]
pub struct CombatLog {
    pub lines: Vec<String>,
}

impl CombatLog {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn add(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
    }
}

impl std::fmt::Display for CombatLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

/// PVE 攻击结果
pub struct AttackResult {
    pub text: String,
    pub error_code: i32,
}

/// 普通攻击怪物
pub fn attack_monster(db: &Database, user_id: &str, skill_name: &str) -> AttackResult {
    let mut log = CombatLog::new();
    let prefix = user::get_msg_prefix(db, user_id);

    // 1. 检查功能可用性
    // 虚弱检查
    let weakness_secs = user::check_weakness(db, user_id);
    let weakness_active = weakness_secs > 0;

    // 体力检查 (每次攻击消耗2体力)
    if let Err(e) = stamina::consume_stamina(user_id, "攻击", db) {
        return AttackResult {
            text: format!("{}\n{}", prefix, e),
            error_code: 1020,
        };
    }

    // 2. 获取目标
    let target = db.read_basic(user_id, ITEM_TARGET);
    if target.is_empty() {
        return AttackResult {
            text: format!(
                "{}\n未锁定攻击目标，无法进行攻击。\n发送'搜索怪物+目标名'即可锁定目标。",
                prefix
            ),
            error_code: 1001,
        };
    }

    // 3. 检查怪物是否存在
    let monster = db.monster_get(&target);
    if monster.is_none() {
        db.write_basic(user_id, ITEM_TARGET, EMPTY);
        return AttackResult {
            text: format!("{}\n不存在此目标。", prefix),
            error_code: 1002,
        };
    }
    let monster = monster.unwrap();

    // 4. 检查怪物是否在当前地图
    let location = db.read_basic(user_id, ITEM_LOCATION);
    let map = db.map_get(&location);
    if let Some(map) = &map {
        if !map.monsters.contains(&target) {
            db.write_basic(user_id, ITEM_TARGET, EMPTY);
            return AttackResult {
                text: format!("{}\n{}内没有{}，请重新搜索！", prefix, location, target),
                error_code: 1003,
            };
        }
    }

    // 5. 获取用户属性
    let info = user::calc_total_attrs(db, user_id);
    let mut user_hp = info.hp;
    let user_mp = info.mp;
    let user_hp_max = info.hp_max;

    // 6. 计算攻击力
    let (user_attack, is_magic) = if !skill_name.is_empty() {
        // 技能攻击
        let skill = db.skill_get(skill_name);
        if skill.is_none() {
            return AttackResult {
                text: format!("{}\n不存在技能[{}]。", prefix, skill_name),
                error_code: 1008,
            };
        }
        let skill = skill.unwrap();

        // 检查是否学会
        if !db.skill_has(user_id, skill_name) {
            return AttackResult {
                text: format!("{}\n您还未学会[{}]。", prefix, skill_name),
                error_code: 1009,
            };
        }

        // 检查冷却
        let last_use = db.read_user_data(user_id, &format!("skill_cd.{}", skill_name));
        if !last_use.is_empty() {
            // 简化：跳过冷却检查
        }

        // 检查消耗
        let consume = skill.consume;
        let consume_type = &skill.consume_type;
        if consume > 0 {
            let current = if consume_type == "魔法" { user_mp } else { user_hp };
            if current < consume {
                let type_name = if consume_type == "魔法" { "魔法" } else { "生命" };
                return AttackResult {
                    text: format!("{}\n无法释放[{}]，剩余{}不足。", prefix, skill_name, type_name),
                    error_code: 1011,
                };
            }
            // 扣除消耗
            if consume_type == "魔法" {
                db.write_basic_int(user_id, ITEM_MP_CURRENT, user_mp - consume);
            } else {
                db.write_basic_int(user_id, ITEM_HP_CURRENT, user_hp - consume);
            }
        }

        // 记录冷却
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        db.write_user_data(user_id, &format!("skill_cd.{}", skill_name), &now);

        // 连招系统：检查前置条件 & 记录技能使用
        let (can_combo, combo_reason) = crate::skill_combo::check_skill_prereq(db, user_id, skill_name);
        crate::skill_combo::record_skill_use(db, user_id, skill_name);
        if !can_combo {
            log.add(format!("⚠ 连招条件: {}", combo_reason));
        }

        // 计算技能伤害
        let effect = skill.effect;
        let is_magic = skill.skill_type.contains("魔") || skill.skill_type.contains("法");

        // 治疗技能
        if skill.skill_type == "疗伤" {
            let heal = effect;
            let new_hp = (user_hp + heal).min(user_hp_max).max(1);
            db.write_basic_int(user_id, ITEM_HP_CURRENT, new_hp);

            log.add(format!("{}使用了[{}]！", prefix, skill_name));
            log.add(format!("恢复生命 {}", heal));
            log.add(format!("当前生命：{}/{}", new_hp, user_hp_max));

            return AttackResult {
                text: log.to_string(),
                error_code: 0,
            };
        }

        (effect, is_magic)
    } else {
        // 普通攻击
        let occupation = db.read_basic(user_id, ITEM_OCCUPATION);
        let occ = db.occupation_get(&occupation);
        let base_damage = occ.map(|o| o.ad.max(o.ap)).unwrap_or(1);
        (base_damage, false)
    };

    // 7. 计算怪物防御
    let monster_def = if is_magic {
        monster.magic_resistance
    } else {
        monster.defense
    };
    let user_penetration = if is_magic { info.ap_ptv } else { info.ad_ptv };
    let user_pen_ratio = if is_magic { info.ap_ptr } else { info.ad_ptr };

    let effective_def = (monster_def - user_penetration).max(0);
    let effective_def = effective_def * (100 - user_pen_ratio.min(100)) / 100;

    // 8. 计算基础伤害
    let mut damage = (user_attack - effective_def).max(0);

    // 8.4 虚弱惩罚（死亡后5分钟内攻击力降低30%）
    if weakness_active {
        let mins = weakness_secs / 60;
        damage = (damage as f64 * user::WEAKNESS_DAMAGE_MULT) as i32;
        log.add(format!("💀 虚弱状态(剩余{}分)：攻击力降低30%！", mins));
    }

    // 8.5 属性克制系统 (Ext_Attribut_Avxav_hzxyx)
    let type_effect = crate::type_effect::calc_type_effectiveness(db, user_id, &target);
    if type_effect.multiplier != 100 {
        damage = damage * type_effect.multiplier / 100;
        let hint = crate::type_effect::format_type_hint(&type_effect);
        if !hint.is_empty() {
            log.add(hint);
        }
    }

    // 8.6 技能熟练度加成
    if !skill_name.is_empty() {
        let prof_bonus = crate::proficiency::get_damage_bonus_pct(db, user_id, skill_name);
        if prof_bonus > 0.0 {
            let old_damage = damage;
            damage = (damage as f64 * (1.0 + prof_bonus / 100.0)) as i32;
            let prof = crate::proficiency::get_skill_proficiency(db, user_id, skill_name);
            let tier = crate::proficiency::get_tier(prof);
            log.add(format!(
                "📜 熟练度加成[{}] Lv.{}: 伤害 +{:.0}% ({}→{})",
                tier.name, prof, prof_bonus, old_damage, damage
            ));
            // 自动增加熟练度
            let occupation = db.read_basic(user_id, "Occupation");
            crate::proficiency::increase_proficiency(db, user_id, skill_name, &occupation);
        } else {
            // 即使没有加成也增加熟练度
            let occupation = db.read_basic(user_id, "Occupation");
            crate::proficiency::increase_proficiency(db, user_id, skill_name, &occupation);
        }
    }

    // 8.7 连招伤害加成
    if !skill_name.is_empty() {
        let (combo_mult, combo_msg) = crate::skill_combo::calc_combo_multiplier(db, user_id);
        if combo_mult > 1.0 {
            let old_damage = damage;
            damage = (damage as f64 * combo_mult) as i32;
            log.add(combo_msg.replace("伤害", &format!("伤害({}→{})", old_damage, damage)));
        }
    }

    // 9. 命中判定
    let user_hit = info.hit;
    let monster_dodge = monster.dodge;
    let hit_rate = user_hit as f64 / (user_hit + monster_dodge) as f64;
    let hit_roll: f64 = rand::thread_rng().gen_range(0.0..1.0);
    let invasion_active = world_event::is_monster_invasion_active(db);

    if hit_roll > hit_rate {
        log.add(format!("{}发动了攻击！", prefix));
        log.add(format!("但是{}闪避了攻击！", target));

        // 怪物反击
        let counter_damage = {
            let base = calc_monster_counter(&info, &monster);
            if invasion_active {
                (base as f64 * 1.5) as i32
            } else {
                base
            }
        };
        if counter_damage > 0 {
            // 护盾吸收
            let mut actual_counter = counter_damage;
            if info.shield > 0 && !monster.ignore_shield {
                let absorbed = db.consume_shield(user_id, counter_damage);
                if absorbed > 0 {
                    actual_counter = (counter_damage - absorbed).max(0);
                    log.add(format!("🛡 护盾吸收了{}点伤害！", absorbed));
                }
            }
            user_hp = (user_hp - actual_counter).max(0);
            db.write_basic_int(user_id, ITEM_HP_CURRENT, user_hp);
            if actual_counter > 0 {
                log.add(format!("{}反击了{}点伤害！", target, actual_counter));
            }
        }

        if user_hp <= 0 {
            handle_user_death(db, user_id, &mut log, &prefix);
        }

        // 被动回复（闪避后也触发）
        if user_hp > 0 {
            let regen_msg = crate::regen::apply_regen(db, user_id);
            if !regen_msg.is_empty() {
                log.add(&regen_msg);
            }
            // 饥饿衰减 + 惩罚
            let (_, _, hunger_tip) = crate::food::apply_hunger_decay(db, user_id);
            if hunger_tip {
                let cfg = crate::food::load_hunger_config(db);
                log.add(&cfg.hunger_tip);
            }
            let hunger_penalty = crate::food::get_hunger_hp_penalty(db, user_id);
            if hunger_penalty > 0 {
                log.add(format!("⚠️ 饥饿导致HP下降 -{}", hunger_penalty));
            }
        }

        return AttackResult {
            text: log.to_string(),
            error_code: 0,
        };
    }

    // 10. 暴击判定
    let crit_rate = info.crit;
    let crit_roll: i32 = rand::thread_rng().gen_range(1..=100);
    let is_crit = crit_roll <= crit_rate;
    if is_crit {
        let crit_multi: f64 = db.global_get("set", "Crit_damage").parse().unwrap_or(1.5);
        damage = (damage as f64 * crit_multi) as i32;
    }

    // 11. 浮动伤害（±10%）
    if damage > 10 {
        let float_damage = db.global_get("set", "Open_floating_damage");
        if float_damage == "TRUE" || float_damage.is_empty() {
            let min_dmg = (damage as f64 * 0.9) as i32;
            damage = rand::thread_rng().gen_range(min_dmg..=damage);
        }
    }

    // 12. 免伤
    let immune = monster.immune_damage;
    if immune > 0 {
        damage = damage * (100 - immune.min(100)) / 100;
    }

    // 13. 吸血
    let absorb_hp = info.absorb_hp;
    let mut healed = 0;
    if absorb_hp > 0 && damage > 0 {
        healed = damage * absorb_hp / 100;
        if healed > 0 {
            user_hp = (user_hp + healed).min(user_hp_max);
            db.write_basic_int(user_id, ITEM_HP_CURRENT, user_hp);
        }
    }

    // 14. 扣除怪物生命
    let monster_hp_remaining = (monster.hp - damage).max(0);
    let monster_dead = monster_hp_remaining <= 0;

    // 15. 生成攻击提示
    // 格式: [攻击描述]|[命中描述]|[未命中描述]|[击杀描述]
    let occupation = db.read_basic(user_id, ITEM_OCCUPATION);
    let weapon_name = db.read_equip_name(user_id, "武器");
    let tips_raw = if !skill_name.is_empty() {
        db.skill_get(skill_name).map(|s| s.attack_tips).unwrap_or_default()
    } else {
        db.occupation_get(&occupation)
            .map(|o| o.attack_tips)
            .unwrap_or_default()
    };

    let parts: Vec<&str> = tips_raw.split('|').collect();
    let crit_text = if is_crit { "暴击" } else { "" };

    // 攻击描述 (dodge 已在前面处理，这里只处理命中情况)
    let atk_tip = if !parts.is_empty() {
        parts[0]
    } else {
        "您发动了攻击！"
    };
    log.add(
        atk_tip
            .replace("[对象名称]", &target)
            .replace("[发起对象]", "您")
            .replace("[技能名称]", skill_name)
            .replace("[装备显示=武器]", &weapon_name),
    );

    // 命中/伤害描述
    let hit_tip = if parts.len() > 1 {
        parts[1].to_string()
    } else {
        "造成了伤害！".to_string()
    };
    let hit_tip = hit_tip
        .replace("[对象名称]", &target)
        .replace("[造成伤害]", &damage.to_string())
        .replace("[是否暴击]", crit_text)
        .replace("[暴击]", crit_text);
    log.add(hit_tip);

    if healed > 0 {
        log.add(format!("吸血恢复了{}点生命！", healed));
    }

    // 16. 怪物反击（怪物入侵时反击伤害+50%）
    let counter_damage = {
        let base = calc_monster_counter(&info, &monster);
        if invasion_active {
            (base as f64 * 1.5) as i32
        } else {
            base
        }
    };
    if counter_damage > 0 && !monster_dead {
        // 护盾吸收
        let mut actual_counter = counter_damage;
        if info.shield > 0 && !monster.ignore_shield {
            let absorbed = db.consume_shield(user_id, counter_damage);
            if absorbed > 0 {
                actual_counter = (counter_damage - absorbed).max(0);
                log.add(format!("🛡 护盾吸收了{}点伤害！", absorbed));
            }
        }
        user_hp = (user_hp - actual_counter).max(0);
        db.write_basic_int(user_id, ITEM_HP_CURRENT, user_hp);
        if actual_counter > 0 {
            log.add(format!("{}反击了！造成了{}点伤害！", target, actual_counter));
        }
    }

    // 17. 处理怪物死亡
    if monster_dead {
        log.add(format!("═══ 击败了{}！═══", target));
        // 更新成就击杀计数
        achievement::on_monster_killed(db, user_id);

        // 记录战斗统计
        crate::combat_stats::record_kill(db, user_id, &target, damage);

        // 奖励经验（含VIP加成 + 全服活动加成）
        let exp_reward = monster.reward_exp;
        if exp_reward > 0 {
            let vip_bonus_pct = vip::get_vip_exp_bonus(db, user_id);
            let bonus_exp = if vip_bonus_pct > 0 {
                exp_reward * vip_bonus_pct / 100
            } else {
                0
            };
            // 全服活动经验加成
            let event_mult = world_event::get_exp_multiplier(db);
            let base_with_vip = exp_reward + bonus_exp;
            let total_exp = if event_mult > 1.0 {
                let event_bonus = (base_with_vip as f64 * (event_mult - 1.0)) as i32;
                log.add(format!("🎉 全服活动经验加成 x{:.1} (+{})", event_mult, event_bonus));
                base_with_vip + event_bonus
            } else {
                base_with_vip
            };
            let (new_level, leveled) = user::add_experience(db, user_id, total_exp);
            if bonus_exp > 0 {
                log.add(format!("获得经验：{} (+{}VIP加成)", total_exp, bonus_exp));
            } else {
                log.add(format!("获得经验：{}", total_exp));
            }
            if leveled {
                log.add(format!("🎉 恭喜升级！当前等级：{}", new_level));
            }
        }

        // 奖励金币（含全服活动加成）
        let gold_reward = monster.reward_gold as i64;
        if gold_reward > 0 {
            let event_mult = world_event::get_gold_multiplier(db);
            let total_gold = if event_mult > 1.0 {
                let event_bonus = (gold_reward as f64 * (event_mult - 1.0)) as i64;
                log.add(format!("💰 全服活动金币加成 x{:.1} (+{})", event_mult, event_bonus));
                gold_reward + event_bonus
            } else {
                gold_reward
            };
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold);
            log.add(format!("获得金币：{}", total_gold));
            achievement::on_gold_earned(db, user_id, total_gold);
        }

        // 掉落物品（含全服活动掉落加成）
        let drop_mult = world_event::get_drop_multiplier(db);
        for item in &monster.reward_goods {
            let adjusted_rate = item.rate * drop_mult;
            let roll: f64 = rand::thread_rng().gen_range(0.0..100.0);
            if roll <= adjusted_rate.min(100.0) {
                db.knapsack_add(user_id, &item.name, item.count);
                if drop_mult > 1.0 {
                    log.add(format!(
                        "掉落物品：[{}]×{} (掉率加成x{:.1})",
                        item.name, item.count, drop_mult
                    ));
                } else {
                    log.add(format!("掉落物品：[{}]×{}", item.name, item.count));
                }
            }
        }

        // 赏金任务进度追踪
        if let Some(bounty_msg) = crate::bounty::on_monster_killed(db, user_id, &target) {
            log.add(bounty_msg);
        }

        // 每日任务进度追踪
        crate::daily_quest::on_monster_killed(db, user_id);

        // 周常任务进度追踪
        crate::weekly_quest::on_monster_killed(db, user_id);

        // 队伍讨伐目标进度追踪
        let current_map = db.read_basic(user_id, crate::core::ITEM_LOCATION);
        crate::team_goals::record_monster_kill(db, user_id, &target, &current_map);

        // 收集图鉴: 记录怪物发现
        crate::collection::record_monster_discovery(db, user_id, &target);

        // 收集图鉴: 记录掉落物品
        for item in &monster.reward_goods {
            let roll: f64 = rand::thread_rng().gen_range(0.0..100.0);
            if roll <= item.rate {
                crate::collection::record_item_collection(db, user_id, &item.name);
            }
        }

        // 记录战斗战报
        let total_exp_reward = monster.reward_exp as i64;
        let total_gold_reward = monster.reward_gold as i64;
        battle_archive::record_battle(
            db,
            user_id,
            "PVE",
            &target,
            true,
            damage as i64,
            counter_damage as i64,
            1,
            total_exp_reward,
            total_gold_reward,
            "",
        );

        // 清除目标
        db.write_basic(user_id, ITEM_TARGET, EMPTY);

        // 用户死亡检查
        if user_hp <= 0 {
            handle_user_death(db, user_id, &mut log, &prefix);
        }
    } else {
        // 显示怪物剩余生命
        log.add(format!("{}剩余生命：{}", target, monster_hp_remaining));
    }

    // 18. 检查装备技能触发
    if !monster_dead {
        if let Some((eq_skill_name, eq_effect)) = crate::equip_skill::try_equip_skill_trigger(db, user_id) {
            let eq_extra_damage = eq_effect / 2;
            if eq_extra_damage > 0 {
                let eq_monster_hp = (monster_hp_remaining - eq_extra_damage).max(0);
                log.add(format!(
                    "⚡ 装备技能[{}]触发！额外造成{}点伤害！",
                    eq_skill_name, eq_extra_damage
                ));

                // 检查持续技能效果
                if let Some(dot_msg) =
                    crate::equip_skill::process_continuous_effect(db, user_id, &eq_skill_name, &target)
                {
                    log.add(dot_msg);
                }

                if eq_monster_hp <= 0 {
                    log.add(format!("═══ 击败了{}！═══", target));
                    let exp_reward = monster.reward_exp;
                    if exp_reward > 0 {
                        let vip_bonus_pct = vip::get_vip_exp_bonus(db, user_id);
                        let bonus_exp = if vip_bonus_pct > 0 {
                            exp_reward * vip_bonus_pct / 100
                        } else {
                            0
                        };
                        let event_mult = world_event::get_exp_multiplier(db);
                        let base_with_vip = exp_reward + bonus_exp;
                        let total_exp = if event_mult > 1.0 {
                            let event_bonus = (base_with_vip as f64 * (event_mult - 1.0)) as i32;
                            log.add(format!("🎉 全服活动经验加成 x{:.1} (+{})", event_mult, event_bonus));
                            base_with_vip + event_bonus
                        } else {
                            base_with_vip
                        };
                        let (new_level, leveled) = user::add_experience(db, user_id, total_exp);
                        if bonus_exp > 0 {
                            log.add(format!("获得经验：{} (+{}VIP加成)", total_exp, bonus_exp));
                        } else {
                            log.add(format!("获得经验：{}", total_exp));
                        }
                        if leveled {
                            log.add(format!("🎉 恭喜升级！当前等级：{}", new_level));
                        }
                    }
                    let gold_reward = monster.reward_gold as i64;
                    if gold_reward > 0 {
                        let event_mult = world_event::get_gold_multiplier(db);
                        let total_gold = if event_mult > 1.0 {
                            let event_bonus = (gold_reward as f64 * (event_mult - 1.0)) as i64;
                            log.add(format!("💰 全服活动金币加成 x{:.1} (+{})", event_mult, event_bonus));
                            gold_reward + event_bonus
                        } else {
                            gold_reward
                        };
                        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, total_gold);
                        log.add(format!("获得金币：{}", total_gold));
                    }
                    let drop_mult = world_event::get_drop_multiplier(db);
                    for item in &monster.reward_goods {
                        let adjusted_rate = item.rate * drop_mult;
                        let roll: f64 = rand::thread_rng().gen_range(0.0..100.0);
                        if roll <= adjusted_rate.min(100.0) {
                            db.knapsack_add(user_id, &item.name, item.count);
                            log.add(format!("掉落物品：[{}]×{}", item.name, item.count));
                        }
                    }
                    db.write_basic(user_id, ITEM_TARGET, EMPTY);
                } else {
                    log.add(format!("{}剩余生命：{}", target, eq_monster_hp));
                }
            }
        }

        // 处理已有的DOT效果
        let dot_msgs = crate::equip_skill::process_active_dots(db, user_id);
        for msg in dot_msgs {
            log.add(msg);
        }

        // 处理已有的Buff效果
        let buff_msgs = crate::equip_skill::process_active_buffs(db, user_id);
        for msg in buff_msgs {
            log.add(msg);
        }

        // 处理 SkillContinued_Register 中的持续效果（每回合递减）
        let registry_effects = crate::equip_skill::get_user_active_effects(db, user_id);
        if !registry_effects.is_empty() {
            for ae in &registry_effects {
                if ae.round > 0 && ae.effect_type == "#2" {
                    log.add(format!("💀 [{}]持续效果生效 (剩余{}回合)", ae.skill, ae.round));
                }
            }
            crate::equip_skill::tick_all_effects(db);
            let cleaned = crate::equip_skill::cleanup_expired_effects(db);
            if cleaned > 0 {
                log.add(format!("✨ {}个过期持续效果已清除", cleaned));
            }
        }
    }

    // 19. 被动回复（战斗结束后自动回血回蓝）
    if user_hp > 0 {
        let regen_msg = crate::regen::apply_regen(db, user_id);
        if !regen_msg.is_empty() {
            log.add(&regen_msg);
        }
        // 饥饿衰减 + 惩罚（战斗结束时）
        let (_, _, hunger_tip) = crate::food::apply_hunger_decay(db, user_id);
        if hunger_tip {
            let cfg = crate::food::load_hunger_config(db);
            log.add(&cfg.hunger_tip);
        }
        let hunger_penalty = crate::food::get_hunger_hp_penalty(db, user_id);
        if hunger_penalty > 0 {
            log.add(format!("⚠️ 饥饿导致HP下降 -{}", hunger_penalty));
        }
    }

    // 20. 更新状态
    db.write_user_data(user_id, "state.IsGameover", if monster_dead { "TRUE" } else { "FALSE" });

    AttackResult {
        text: log.to_string(),
        error_code: 0,
    }
}

/// 计算怪物反击伤害
fn calc_monster_counter(info: &UserInfo, monster: &MonsterDef) -> i32 {
    let monster_ad = monster.ad;
    let monster_ap = monster.ap;
    let user_def = info.defense;
    let user_mres = info.magic_res;

    // 取较高伤害（物攻 vs 魔攻）
    let phys_dmg = (monster_ad - user_def).max(0);
    let mag_dmg = (monster_ap - user_mres).max(0);
    let damage = phys_dmg.max(mag_dmg);

    // 免伤
    let immune = info.immune;
    if immune > 0 {
        damage * (100 - immune.min(100)) / 100
    } else {
        damage
    }
}

/// 处理用户死亡
fn handle_user_death(db: &Database, user_id: &str, log: &mut CombatLog, prefix: &str) {
    log.add(format!("{}被击败了！", prefix));
    log.add("═══ 战斗失败 ═══");
    log.add("您已死亡，正在复活...");

    // 记录战斗统计
    let target = db.read_basic(user_id, ITEM_TARGET);
    crate::combat_stats::record_death(db, user_id, &target);

    // 记录战斗战报（失败）
    battle_archive::record_battle(db, user_id, "PVE", &target, false, 0, 0, 0, 0, 0, "死亡");

    // 中断自动修炼
    let evo_msg = crate::training::interrupt_auto_evo(db, user_id);
    if !evo_msg.is_empty() {
        log.add(&evo_msg);
    }

    // 复活：恢复一半生命
    let hp_max = user::calc_hp_max(db, user_id);
    let revive_hp = hp_max / 2;
    db.write_basic_int(user_id, ITEM_HP_CURRENT, revive_hp);
    log.add(format!("复活成功！当前生命：{}/{}", revive_hp, hp_max));

    // 设置虚弱状态
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.write_user_data(user_id, "state.weakness_time", &now);
}

/// 获取怪物信息文本
#[allow(dead_code)]
pub fn get_monster_info(db: &Database, monster_name: &str) -> String {
    let monster = db.monster_get(monster_name);
    if monster.is_none() {
        return "怪物不存在！".to_string();
    }
    let m = monster.unwrap();
    format!(
        "═══ {} ═══\n类型：{}\n生命：{}\n物攻：{} 魔攻：{}\n防御：{} 魔抗：{}\n命中：{} 闪避：{}\n吸血：{} 免伤：{}%\n介绍：{}",
        m.name, m.monster_type, m.hp, m.ad, m.ap, m.defense, m.magic_resistance,
        m.hit, m.dodge, m.absorb_hp, m.immune_damage, m.introduce
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_log_new() {
        let log = CombatLog::new();
        assert!(log.lines.is_empty());
    }

    #[test]
    fn test_combat_log_add() {
        let mut log = CombatLog::new();
        log.add("第一行");
        log.add("第二行");
        assert_eq!(log.lines.len(), 2);
        assert_eq!(log.lines[0], "第一行");
        assert_eq!(log.lines[1], "第二行");
    }

    #[test]
    fn test_combat_log_display() {
        let mut log = CombatLog::new();
        log.add("攻击了怪物");
        log.add("造成了50点伤害");
        let display = format!("{}", log);
        assert_eq!(display, "攻击了怪物\n造成了50点伤害");
    }

    #[test]
    fn test_combat_log_display_empty() {
        let log = CombatLog::new();
        let display = format!("{}", log);
        assert_eq!(display, "");
    }

    #[test]
    fn test_combat_log_add_string() {
        let mut log = CombatLog::new();
        let s = String::from("owned string");
        log.add(s);
        assert_eq!(log.lines.len(), 1);
        assert_eq!(log.lines[0], "owned string");
    }

    #[test]
    fn test_attack_result_struct() {
        let result = AttackResult {
            text: "攻击成功".to_string(),
            error_code: 0,
        };
        assert_eq!(result.text, "攻击成功");
        assert_eq!(result.error_code, 0);
    }

    #[test]
    fn test_attack_result_error() {
        let result = AttackResult {
            text: "未锁定目标".to_string(),
            error_code: 1001,
        };
        assert_eq!(result.error_code, 1001);
        assert!(result.text.contains("未锁定"));
    }
}

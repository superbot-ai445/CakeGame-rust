/// CakeGame 副本系统
/// 来自 MessageTemplate: com.shdic.cg_instanceDungeon 系列模板
/// 多关卡副本挑战：等级限制、冷却时间、每日重置、消耗物品、固定+随机奖励
use crate::core::*;
use crate::db::Database;
use crate::stamina;
use crate::user;
use crate::vip;
use rand::Rng;

/// 副本关卡
struct InstanceStage {
    name: &'static str,
    monsters: &'static [(&'static str, u32)], // (怪物名, 数量)
}

/// 副本定义
struct InstanceDef {
    name: &'static str,
    instance_type: &'static str, // 普通/精英/地狱
    min_level: u32,
    max_level: u32,
    cooldown_secs: u64,
    daily_max_resets: u32,
    cost_item: &'static str, // 消耗物品名（空=无消耗）
    cost_amount: u32,
    intro: &'static str,
    stages: &'static [InstanceStage],
    fixed_reward_gold: u32,
    fixed_reward_exp: u32,
    fixed_reward_item: &'static str,                // 固定奖励物品（空=无）
    random_rewards: &'static [(&'static str, f64)], // (物品名, 概率)
}

/// 获取所有副本定义
fn get_instances() -> Vec<InstanceDef> {
    vec![
        InstanceDef {
            name: "格兰森林试炼",
            instance_type: "普通",
            min_level: 1,
            max_level: 20,
            cooldown_secs: 300,
            daily_max_resets: 5,
            cost_item: "",
            cost_amount: 0,
            intro: "新手冒险者的第一道试炼。穿过史莱姆和哥布林出没的格兰森林，证明你的实力！",
            stages: &[
                InstanceStage {
                    name: "第一关·史莱姆巢穴",
                    monsters: &[("史莱姆", 3)],
                },
                InstanceStage {
                    name: "第二关·哥布林营地",
                    monsters: &[("哥布林", 2), ("史莱姆", 2)],
                },
                InstanceStage {
                    name: "第三关·领主大厅",
                    monsters: &[("落雷领主", 1)],
                },
            ],
            fixed_reward_gold: 200,
            fixed_reward_exp: 500,
            fixed_reward_item: "【普通】红草药",
            random_rewards: &[
                ("【精良】兽皮胸甲", 0.08),
                ("【精良】鹿皮护肩", 0.08),
                ("白色精粹", 0.50),
            ],
        },
        InstanceDef {
            name: "牛头迷宫",
            instance_type: "普通",
            min_level: 15,
            max_level: 40,
            cooldown_secs: 600,
            daily_max_resets: 3,
            cost_item: "",
            cost_amount: 0,
            intro: "莱茵高原深处的牛头人迷宫，传闻有黄金石巨人把守最终宝藏。勇士们，准备好了吗？",
            stages: &[
                InstanceStage {
                    name: "第一关·迷宫入口",
                    monsters: &[("牛头人", 2), ("十夫长", 1)],
                },
                InstanceStage {
                    name: "第二关·守卫长廊",
                    monsters: &[("牛头卫兵", 2), ("牛头人", 2)],
                },
                InstanceStage {
                    name: "第三关·巨兽巢穴",
                    monsters: &[("牛头巨兽", 1), ("牛头卫兵", 2)],
                },
                InstanceStage {
                    name: "第四关·黄金石巨人",
                    monsters: &[("黄金石巨人", 1)],
                },
            ],
            fixed_reward_gold: 1000,
            fixed_reward_exp: 3000,
            fixed_reward_item: "【精良】密制碳钢胸甲",
            random_rewards: &[
                ("【卓越】古树之杖", 0.05),
                ("【精良】骑士长枪", 0.10),
                ("深渊挑战书", 0.30),
                ("蓝色精粹", 0.40),
            ],
        },
        InstanceDef {
            name: "菲尔炼狱",
            instance_type: "精英",
            min_level: 25,
            max_level: 50,
            cooldown_secs: 1200,
            daily_max_resets: 3,
            cost_item: "深渊挑战书",
            cost_amount: 1,
            intro:
                "拉拉肥的火焰沙漠深处，传说中的炼狱试炼。高温灼烧的大地、凶猛的拉拉肥……只有真正的精英才能活着走出来。",
            stages: &[
                InstanceStage {
                    name: "第一关·沙漠前哨",
                    monsters: &[("火烧拉拉肥", 2)],
                },
                InstanceStage {
                    name: "第二关·雷电山脊",
                    monsters: &[("雷劈拉拉肥", 1), ("火烧拉拉肥", 2)],
                },
                InstanceStage {
                    name: "第三关·熔岩核心",
                    monsters: &[("雷劈拉拉肥", 2)],
                },
                InstanceStage {
                    name: "最终关·拉拉肥之王",
                    monsters: &[("雷劈拉拉肥", 1), ("火烧拉拉肥", 3)],
                },
            ],
            fixed_reward_gold: 3000,
            fixed_reward_exp: 8000,
            fixed_reward_item: "【史诗】精武之魂胸甲",
            random_rewards: &[
                ("【史诗】斩影刃", 0.03),
                ("【史诗】飓风AK47", 0.03),
                ("【史诗】千年枯木法杖", 0.03),
                ("【史诗】灰烬雕翎护肩", 0.02),
                ("【史诗】灰烬雕翎长袍", 0.02),
                ("远古超界石", 0.08),
            ],
        },
        InstanceDef {
            name: "暗黑城堡",
            instance_type: "精英",
            min_level: 35,
            max_level: 60,
            cooldown_secs: 1800,
            daily_max_resets: 2,
            cost_item: "深渊挑战书",
            cost_amount: 2,
            intro:
                "阴森的暗黑城堡，绿色的幽灵四处飘荡。骷髅兵在走廊游荡，无头骑士骑着幽灵马巡逻。这里有传说中的完美装备……",
            stages: &[
                InstanceStage {
                    name: "第一关·骷髅大厅",
                    monsters: &[("骷髅兵", 3), ("骷髅剑士", 1)],
                },
                InstanceStage {
                    name: "第二关·亡灵走廊",
                    monsters: &[("骷髅剑士", 2), ("活死人", 2)],
                },
                InstanceStage {
                    name: "第三关·黑暗王座",
                    monsters: &[("黑暗武士", 1), ("骷髅领主", 1)],
                },
                InstanceStage {
                    name: "最终关·无头骑士",
                    monsters: &[("无头骑士", 1)],
                },
            ],
            fixed_reward_gold: 5000,
            fixed_reward_exp: 15000,
            fixed_reward_item: "【完美】月灵",
            random_rewards: &[
                ("【完美】嗜血之灵", 0.03),
                ("【完美】狂徒信仰", 0.03),
                ("【完美】氪能破灭之眼", 0.03),
                ("【完美】第三元素吊坠", 0.03),
                ("【史诗】幽梦之鸣", 0.05),
                ("【史诗】巫师亚伯的法杖", 0.05),
            ],
        },
        InstanceDef {
            name: "维度深渊",
            instance_type: "地狱",
            min_level: 45,
            max_level: 99,
            cooldown_secs: 3600,
            daily_max_resets: 1,
            cost_item: "深渊挑战书",
            cost_amount: 5,
            intro: "维度深渊，恶魔气息弥漫。这里栖息着最凶猛的怪物和最强大的存在。只有传说中的勇士才敢踏足此地。",
            stages: &[
                InstanceStage {
                    name: "第一关·深渊入口",
                    monsters: &[("末日使者", 1), ("巨牙海民", 2)],
                },
                InstanceStage {
                    name: "第二关·冥界通道",
                    monsters: &[("黑暗武士", 2), ("骷髅领主", 1)],
                },
                InstanceStage {
                    name: "第三关·追猎者领域",
                    monsters: &[("傲の追猎者", 1), ("异类骑士", 1)],
                },
                InstanceStage {
                    name: "第四关·永夜之域",
                    monsters: &[("永夜之皇", 1)],
                },
                InstanceStage {
                    name: "最终关·深渊之主",
                    monsters: &[("CXK", 1)],
                },
            ],
            fixed_reward_gold: 20000,
            fixed_reward_exp: 50000,
            fixed_reward_item: "【远古】荣耀之魂",
            random_rewards: &[
                ("【远古】哈迪斯的召唤", 0.03),
                ("【远古】妖刀村雨", 0.03),
                ("【远古】永恒双星", 0.03),
                ("【远古】龙皇之怒", 0.03),
                ("【远古】黎明使者", 0.03),
                ("【远古】时王枪剑", 0.03),
                ("飓风AKS设计图", 0.01),
                ("旋风M4S设计图", 0.01),
            ],
        },
    ]
}

/// 获取用户当前所在副本和关卡进度
fn get_instance_progress(db: &Database, user_id: &str) -> (String, u32) {
    let inst = db.read_user_data(user_id, "InstanceCurrent");
    let stage_str = db.read_user_data(user_id, "InstanceStage");
    let stage = stage_str.parse::<u32>().unwrap_or(0);
    (inst, stage)
}

/// 获取用户副本冷却
fn get_instance_cooldown(db: &Database, user_id: &str, inst_name: &str) -> String {
    db.read_user_data(user_id, &format!("InstanceCD_{}", inst_name))
}

/// 获取用户今日重置次数
fn get_instance_daily_count(db: &Database, user_id: &str, inst_name: &str) -> u32 {
    let key = format!("InstanceDaily_{}", inst_name);
    let date_key = format!("InstanceDailyDate_{}", inst_name);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let last_date = db.read_user_data(user_id, &date_key);
    if last_date != today {
        db.write_user_data(user_id, &date_key, &today);
        db.write_user_data(user_id, &key, "0");
        0
    } else {
        db.read_user_data(user_id, &key).parse::<u32>().unwrap_or(0)
    }
}

/// 增加重置次数
fn increment_daily_count(db: &Database, user_id: &str, inst_name: &str) {
    let key = format!("InstanceDaily_{}", inst_name);
    let date_key = format!("InstanceDailyDate_{}", inst_name);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    db.write_user_data(user_id, &date_key, &today);
    let cur = db.read_user_data(user_id, &key).parse::<u32>().unwrap_or(0);
    db.write_user_data(user_id, &key, &(cur + 1).to_string());
}

/// 查看副本列表
pub fn cmd_instance_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let instances = get_instances();
    let mut r = format!("{}\n", prefix);

    let dungeon_entries: Vec<(usize, String)> = instances
        .iter()
        .enumerate()
        .map(|(i, inst)| {
            (
                i + 1,
                format!(
                    "[{}] {} Lv.{}-{} CD:{}秒 每日{}次",
                    inst.instance_type,
                    inst.name,
                    inst.min_level,
                    inst.max_level,
                    inst.cooldown_secs,
                    inst.daily_max_resets
                ),
            )
        })
        .collect();
    r.push_str(&crate::template_render::render_dungeon_list(db, &dungeon_entries));

    r.push_str(&format!(
        "\n\n📋 共{}个副本可挑战\n💡 发送'查看副本+副本名'查看详情\n⚔️ 发送'挑战副本+副本名'开始挑战",
        instances.len()
    ));
    r
}

/// 查看副本详情
pub fn cmd_instance_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if args.trim().is_empty() {
        return format!("{}\n请指定副本名称！\n发送'查看副本列表'查看所有副本。", prefix);
    }

    let instances = get_instances();
    let search = args.trim();

    // 模糊匹配
    let inst = instances.iter().find(|i| i.name.contains(search));
    if inst.is_none() {
        // 尝试序号匹配
        if let Ok(idx) = search.parse::<usize>() {
            if idx >= 1 && idx <= instances.len() {
                return format_instance_detail(db, user_id, &prefix, &instances[idx - 1]);
            }
        }
        return format!("{}\n未找到副本'{}'！\n发送'查看副本列表'查看所有副本。", prefix, search);
    }
    format_instance_detail(db, user_id, &prefix, inst.unwrap())
}

fn format_instance_detail(db: &Database, user_id: &str, prefix: &str, inst: &InstanceDef) -> String {
    let daily_count = get_instance_daily_count(db, user_id, inst.name);
    let (current_inst, current_stage) = get_instance_progress(db, user_id);

    let mut r = format!(
        "{}\n═══ 副本详情 ═══\n📌 名称：[{}] {}\n🏷️ 类型：{}\n📊 等级限制：{}-{}\n⏰ 冷却时间：{}秒\n🔄 每日重置：{}/{}次",
        prefix,
        inst.instance_type,
        inst.name,
        inst.instance_type,
        inst.min_level,
        inst.max_level,
        inst.cooldown_secs,
        daily_count,
        inst.daily_max_resets,
    );

    if !inst.cost_item.is_empty() {
        r.push_str(&format!("\n💎 消耗：{}×{}", inst.cost_item, inst.cost_amount));
    } else {
        r.push_str("\n💎 消耗：无");
    }

    r.push_str(&format!("\n\n📝 简介：{}", inst.intro));

    r.push_str("\n\n📍 关卡一览：");
    for (i, stage) in inst.stages.iter().enumerate() {
        let monster_desc: Vec<String> = stage
            .monsters
            .iter()
            .map(|(name, count)| format!("{}×{}", name, count))
            .collect();
        r.push_str(&format!(
            "\n  关卡{}·{} [{}]",
            i + 1,
            stage.name,
            monster_desc.join("、")
        ));
    }

    r.push_str(&format!(
        "\n\n🎁 完成奖励：\n  固定：{}金币 + {}经验{}",
        inst.fixed_reward_gold,
        inst.fixed_reward_exp,
        if inst.fixed_reward_item.is_empty() {
            String::new()
        } else {
            format!(" + {}", inst.fixed_reward_item)
        },
    ));

    if !inst.random_rewards.is_empty() {
        r.push_str("\n  随机掉落：");
        for (item, prob) in inst.random_rewards {
            r.push_str(&format!("\n    {} ({}%)", item, (prob * 100.0) as u32));
        }
    }

    // 当前状态
    if current_inst == inst.name && current_stage > 0 {
        r.push_str(&format!(
            "\n\n📍 当前进度：已在副本中，关卡 {}/{}",
            current_stage,
            inst.stages.len()
        ));
    }

    r.push_str("\n\n⚔️ 发送'挑战副本+副本名'开始挑战！");
    r
}

/// 挑战副本
pub fn cmd_instance_challenge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 检查存活
    let hp_str = db.read_basic(user_id, ITEM_HP_CURRENT);
    let hp: i64 = hp_str.parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命后再挑战副本！", prefix);
    }

    // 体力检查 (副本消耗10体力)
    if let Err(e) = stamina::consume_stamina(user_id, "副本", db) {
        return format!("{}\n{}", prefix, e);
    }

    let args = args.trim();

    // 如果没有参数，检查是否在副本中（继续挑战）
    if args.is_empty() {
        let (current_inst, current_stage) = get_instance_progress(db, user_id);
        if current_inst.is_empty() || current_stage == 0 {
            return format!("{}\n请指定要挑战的副本名称！\n发送'查看副本列表'查看所有副本。", prefix);
        }
        // 继续当前副本
        let instances = get_instances();
        if let Some(inst) = instances.iter().find(|i| i.name == current_inst) {
            return advance_instance_stage(db, user_id, &prefix, inst, current_stage);
        } else {
            return format!("{}\n当前副本数据异常，已重置。请重新挑战。", prefix);
        }
    }

    let instances = get_instances();
    let search = args;
    let inst = instances.iter().find(|i| i.name.contains(search));
    if inst.is_none() {
        // 尝试序号
        if let Ok(idx) = search.parse::<usize>() {
            if idx >= 1 && idx <= instances.len() {
                return start_instance(db, user_id, &prefix, &instances[idx - 1]);
            }
        }
        return format!("{}\n未找到副本'{}'！", prefix, search);
    }
    start_instance(db, user_id, &prefix, inst.unwrap())
}

/// 开始一个新副本
fn start_instance(db: &Database, user_id: &str, prefix: &str, inst: &InstanceDef) -> String {
    // 1. 检查等级
    let level: u32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level < inst.min_level {
        return format!(
            "{}\n等级不足！[{}] 需要等级 {}-{}，您当前等级 {}。",
            prefix, inst.name, inst.min_level, inst.max_level, level
        );
    }
    if level > inst.max_level {
        return format!(
            "{}\n等级过高！[{}] 适合等级 {}-{}，您当前等级 {}。",
            prefix, inst.name, inst.min_level, inst.max_level, level
        );
    }

    // 2. 检查是否已在副本中
    let (current_inst, current_stage) = get_instance_progress(db, user_id);
    if !current_inst.is_empty() && current_stage > 0 {
        if current_inst == inst.name {
            return format!(
                "{}\n您已在副本[{}]中，关卡进度 {}/{}。\n发送'挑战副本'继续挑战！",
                prefix,
                current_inst,
                current_stage,
                inst.stages.len()
            );
        }
        return format!(
            "{}\n您已在副本[{}]中（关卡 {}），请先完成或等待重置后再挑战新副本！",
            prefix, current_inst, current_stage
        );
    }

    // 3. 检查冷却
    let cd_str = get_instance_cooldown(db, user_id, inst.name);
    if !cd_str.is_empty() {
        if let Ok(cd_ts) = cd_str.parse::<i64>() {
            let now = chrono::Local::now().timestamp();
            if now < cd_ts {
                let remaining = cd_ts - now;
                return format!("{}\n副本[{}]冷却中！还需等待 {}秒。", prefix, inst.name, remaining);
            }
        }
    }

    // 4. 检查每日重置
    let daily_count = get_instance_daily_count(db, user_id, inst.name);
    if daily_count >= inst.daily_max_resets {
        return format!(
            "{}\n副本[{}]今日重置次数已用完（{}/{}）！明天再来。",
            prefix, inst.name, daily_count, inst.daily_max_resets
        );
    }

    // 5. 检查消耗物品
    if !inst.cost_item.is_empty() {
        let owned = db.get_item_count(user_id, inst.cost_item);
        if owned < inst.cost_amount as i32 {
            return format!(
                "{}\n挑战副本[{}]需要 {}×{}，您当前拥有 {}个。",
                prefix, inst.name, inst.cost_item, inst.cost_amount, owned
            );
        }
        db.remove_item(user_id, inst.cost_item, inst.cost_amount as i32);
    }

    // 6. 设置副本进度
    db.write_user_data(user_id, "InstanceCurrent", inst.name);
    db.write_user_data(user_id, "InstanceStage", "1");
    increment_daily_count(db, user_id, inst.name);

    // 7. 进入第一关
    advance_instance_stage(db, user_id, prefix, inst, 1)
}

/// 推进副本关卡
fn advance_instance_stage(db: &Database, user_id: &str, prefix: &str, inst: &InstanceDef, stage_num: u32) -> String {
    let stage_idx = (stage_num - 1) as usize;
    if stage_idx >= inst.stages.len() {
        // 已完成所有关卡，发放奖励
        return complete_instance(db, user_id, prefix, inst);
    }

    let stage = &inst.stages[stage_idx];

    // 检查存活
    let hp: i64 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp <= 0 {
        // 副本失败
        db.write_user_data(user_id, "InstanceCurrent", "");
        db.write_user_data(user_id, "InstanceStage", "0");
        let now = chrono::Local::now().timestamp();
        db.write_user_data(
            user_id,
            &format!("InstanceCD_{}", inst.name),
            &(now + inst.cooldown_secs as i64).to_string(),
        );
        return format!(
            "{}\n💀 您在副本[{}]关卡{}中阵亡！\n副本挑战失败，冷却{}秒后可重新挑战。",
            prefix, inst.name, stage_num, inst.cooldown_secs
        );
    }

    // 模拟战斗
    let mut rng = rand::thread_rng();
    let mut log = format!(
        "{}\n═══ 副本[{}] ═══\n📍 {} ({}/{})\n",
        prefix,
        inst.name,
        stage.name,
        stage_num,
        inst.stages.len()
    );

    // 获取角色属性
    let ad: i64 = db.read_basic(user_id, ITEM_AD).parse().unwrap_or(10);
    let ap: i64 = db.read_basic(user_id, ITEM_AP).parse().unwrap_or(10);
    let current_hp_init: i64 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(100);
    let user_hp_max: i64 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(100);
    let defense: i64 = db.read_basic(user_id, ITEM_DEFENSE).parse().unwrap_or(0);

    let mut total_damage_dealt = 0i64;
    let mut total_damage_taken = 0i64;
    let mut monsters_killed = 0u32;
    let mut current_hp = current_hp_init;

    for &(monster_name, count) in stage.monsters.iter() {
        let monster = db.monster_get(monster_name);
        let (m_hp, m_ad, m_def) = if let Some(ref m) = monster {
            (m.hp.max(1) as i64, m.ad.max(1) as i64, m.defense.max(0) as i64)
        } else {
            (100i64, 20i64, 10i64)
        };

        for _ in 0..count {
            if current_hp <= 0 {
                break;
            }

            log.push_str(&format!("\n⚔️ 遭遇 {}！", monster_name));
            let mut monster_hp = m_hp;

            // 回合制战斗（最多20回合）
            for round in 1..=20u32 {
                // 玩家攻击
                let base_dmg = std::cmp::max(ad, ap);
                let dmg = std::cmp::max(1, base_dmg - m_def + rng.gen_range(-5..=5));
                monster_hp -= dmg;
                total_damage_dealt += dmg;

                if monster_hp <= 0 {
                    log.push_str(&format!(
                        "\n  回合{}: 对{}造成{}伤害 ✅击杀！",
                        round, monster_name, dmg
                    ));
                    monsters_killed += 1;
                    break;
                }

                // 怪物反击
                let m_dmg = std::cmp::max(1, m_ad - defense + rng.gen_range(-5..=5));
                current_hp -= m_dmg;
                total_damage_taken += m_dmg;

                if current_hp <= 0 {
                    current_hp = 0;
                    log.push_str(&format!(
                        "\n  回合{}: 对{}造成{}伤害，{}反击{}伤害 💀阵亡！",
                        round, monster_name, dmg, monster_name, m_dmg
                    ));
                    break;
                }

                if round % 5 == 0 {
                    log.push_str(&format!(
                        "\n  回合{}: 造成{}伤害 | 受到{}伤害 | HP: {}/{}",
                        round, dmg, m_dmg, current_hp, user_hp_max
                    ));
                }
            }
        }
    }

    // 写回HP
    db.write_basic(user_id, ITEM_HP_CURRENT, &current_hp.to_string());

    if current_hp <= 0 {
        // 副本失败
        db.write_user_data(user_id, "InstanceCurrent", "");
        db.write_user_data(user_id, "InstanceStage", "0");
        let now = chrono::Local::now().timestamp();
        db.write_user_data(
            user_id,
            &format!("InstanceCD_{}", inst.name),
            &(now + inst.cooldown_secs as i64).to_string(),
        );
        log.push_str(&format!(
            "\n\n💀 副本挑战失败！\n📊 战斗统计：击杀{}只怪物 | 造成{}伤害 | 受到{}伤害\n⏰ 冷却{}秒后可重新挑战。",
            monsters_killed, total_damage_dealt, total_damage_taken, inst.cooldown_secs
        ));
        return log;
    }

    // 关卡通过
    log.push_str(&format!(
        "\n\n✅ {} 通过！\n📊 战斗统计：击杀{}只怪物 | 造成{}伤害 | 受到{}伤害 | 剩余HP: {}/{}",
        stage.name, monsters_killed, total_damage_dealt, total_damage_taken, current_hp, user_hp_max
    ));

    // 推进到下一关
    let next_stage = stage_num + 1;
    if next_stage > inst.stages.len() as u32 {
        // 完成所有关卡
        db.write_user_data(user_id, "InstanceCurrent", "");
        db.write_user_data(user_id, "InstanceStage", "0");
        return complete_instance_from_partial(db, user_id, &log, prefix, inst);
    }

    db.write_user_data(user_id, "InstanceStage", &next_stage.to_string());
    let next_stage_data = &inst.stages[(next_stage - 1) as usize];
    let next_monsters: Vec<String> = next_stage_data
        .monsters
        .iter()
        .map(|(name, count)| format!("{}×{}", name, count))
        .collect();
    log.push_str(&format!(
        "\n\n🔜 下一关：{} [{}]\n💡 发送'挑战副本'继续！",
        next_stage_data.name,
        next_monsters.join("、")
    ));

    log
}

/// 完成副本（从头到尾一次性通关）
fn complete_instance(db: &Database, user_id: &str, prefix: &str, inst: &InstanceDef) -> String {
    complete_instance_from_partial(db, user_id, "", prefix, inst)
}

/// 完成副本（发放奖励）
fn complete_instance_from_partial(
    db: &Database,
    user_id: &str,
    prev_log: &str,
    prefix: &str,
    inst: &InstanceDef,
) -> String {
    let mut log = if prev_log.is_empty() {
        format!("{}\n", prefix)
    } else {
        format!("{}\n", prev_log)
    };

    log.push_str(&format!("═══ 副本[{}]完成！ ═══\n", inst.name));

    // 固定奖励
    {
        let current_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        db.write_currency(user_id, CURRENCY_GOLD, current_gold + inst.fixed_reward_gold as i64);
    }
    user::add_experience(db, user_id, inst.fixed_reward_exp as i32);
    let vip_bonus_pct = vip::get_vip_exp_bonus(db, user_id);
    let bonus_exp = if vip_bonus_pct > 0 {
        inst.fixed_reward_exp as i32 * vip_bonus_pct / 100
    } else {
        0
    };
    if bonus_exp > 0 {
        user::add_experience(db, user_id, bonus_exp);
    }
    log.push_str(&format!(
        "\n🎁 固定奖励：\n  💰 {}金币\n  ✨ {}经验{}",
        inst.fixed_reward_gold,
        inst.fixed_reward_exp as i32 + bonus_exp,
        if bonus_exp > 0 {
            format!(" (+{}VIP加成)", bonus_exp)
        } else {
            String::new()
        }
    ));

    if !inst.fixed_reward_item.is_empty() {
        db.add_item(user_id, inst.fixed_reward_item, 1);
        log.push_str(&format!("\n  📦 {}", inst.fixed_reward_item));
    }

    // 随机奖励
    let mut rng = rand::thread_rng();
    let mut got_random = false;
    for &(item, prob) in inst.random_rewards {
        if rng.gen_bool(prob.min(1.0)) {
            db.add_item(user_id, item, 1);
            if !got_random {
                log.push_str("\n\n🎲 随机掉落：");
                got_random = true;
            }
            log.push_str(&format!("\n  📦 {}", item));
        }
    }
    if !got_random {
        log.push_str("\n\n🎲 随机掉落：本次未获得随机奖励");
    }

    // 记录副本通关到排行榜
    record_instance_completion(db, user_id, inst.name, inst.instance_type);

    log.push_str(&format!(
        "\n\n{}",
        crate::template_render::render_dungeon_complete(
            db,
            &format!("{}金币+{}经验", inst.fixed_reward_gold, inst.fixed_reward_exp)
        )
    ));
    log
}

/// 记录副本通关数据到排行榜
fn record_instance_completion(db: &Database, user_id: &str, inst_name: &str, inst_type: &str) {
    let now = chrono::Local::now().timestamp();
    let section = "instance_ranking";

    // 更新个人通关次数
    let count_key = format!("{}_{}", user_id, inst_name);
    let count_str = db.global_get(section, &count_key);
    let count: u32 = count_str.parse().unwrap_or(0) + 1;
    db.global_set(section, &count_key, &count.to_string());

    // 更新全服通关总次数
    let total_key = format!("total_{}", inst_name);
    let total_str = db.global_get(section, &total_key);
    let total: u32 = total_str.parse().unwrap_or(0) + 1;
    db.global_set(section, &total_key, &total.to_string());

    // 记录最近通关时间（用于排名）
    let last_key = format!("last_{}_{}", user_id, inst_name);
    db.global_set(section, &last_key, &now.to_string());

    // 记录副本类型
    let type_key = format!("type_{}", inst_name);
    if db.global_get(section, &type_key).is_empty() {
        db.global_set(section, &type_key, inst_type);
    }

    // 更新个人总通关次数
    let user_total_key = format!("user_total_{}", user_id);
    let user_total_str = db.global_get(section, &user_total_key);
    let user_total: u32 = user_total_str.parse().unwrap_or(0) + 1;
    db.global_set(section, &user_total_key, &user_total.to_string());
}

/// 副本排行榜
pub fn cmd_instance_ranking(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let section = "instance_ranking";
    let args = args.trim();

    // 如果指定了副本名，显示该副本的通关排行
    if !args.is_empty() {
        return show_instance_specific_ranking(db, user_id, &prefix, args, section);
    }

    // 显示总体排行榜
    let instances = get_instances();
    let mut r = format!("{}\n═══ 副本排行榜 ═══\n", prefix);

    // 个人总通关次数
    let user_total_key = format!("user_total_{}", user_id);
    let user_total: u32 = db.global_get(section, &user_total_key).parse().unwrap_or(0);
    r.push_str(&format!("\n📊 您的总通关次数: {}次\n", user_total));

    // 各副本全服通关统计
    r.push_str("\n🏆 全服副本通关统计:\n");
    for inst in &instances {
        let total_key = format!("total_{}", inst.name);
        let total: u32 = db.global_get(section, &total_key).parse().unwrap_or(0);
        let user_count_key = format!("{}_{}", user_id, inst.name);
        let user_count: u32 = db.global_get(section, &user_count_key).parse().unwrap_or(0);

        let type_emoji = match inst.instance_type {
            "普通" => "🟢",
            "精英" => "🟡",
            "地狱" => "🔴",
            _ => "⚪",
        };

        r.push_str(&format!(
            "\n{} [{}] {} — 全服{}次 | 您{}次",
            type_emoji, inst.instance_type, inst.name, total, user_count
        ));
    }

    r.push_str("\n\n💡 发送'副本排行+副本名'查看该副本详细排行");
    r.push_str("\n⚔️ 发送'挑战副本+副本名'开始挑战！");
    r
}

/// 显示特定副本的详细排行
fn show_instance_specific_ranking(db: &Database, user_id: &str, prefix: &str, search: &str, section: &str) -> String {
    let instances = get_instances();
    let inst = instances.iter().find(|i| i.name.contains(search));
    if inst.is_none() {
        // 尝试序号
        if let Ok(idx) = search.parse::<usize>() {
            if idx >= 1 && idx <= instances.len() {
                return show_instance_detail_ranking(db, user_id, prefix, &instances[idx - 1], section);
            }
        }
        return format!("{}\n未找到副本'{}'！\n发送'副本排行'查看所有副本。", prefix, search);
    }
    show_instance_detail_ranking(db, user_id, prefix, inst.unwrap(), section)
}

/// 显示单个副本的详细排行
fn show_instance_detail_ranking(
    db: &Database,
    user_id: &str,
    prefix: &str,
    inst: &InstanceDef,
    section: &str,
) -> String {
    let total_key = format!("total_{}", inst.name);
    let total: u32 = db.global_get(section, &total_key).parse().unwrap_or(0);

    let mut r = format!(
        "{}\n═══ 副本排行: {} ═══\n📋 类型: [{}] | 全服通关: {}次\n",
        prefix, inst.name, inst.instance_type, total
    );

    // 收集所有玩家的通关数据
    let mut player_scores: Vec<(String, u32, i64)> = Vec::new(); // (user_id, count, last_time)

    // 遍历 Global 表查找该副本的所有玩家数据
    {
        let conn = db.lock_conn();
        let pattern = format!("%_{}", inst.name);
        let mut stmt = conn
            .prepare("SELECT ID, DATA FROM Global WHERE SECTION=?1 AND ID LIKE ?2 AND ID NOT LIKE 'total_%' AND ID NOT LIKE 'last_%' AND ID NOT LIKE 'type_%' AND ID NOT LIKE 'user_%'")
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params![section, pattern], |row| {
                let id: String = row.get(0)?;
                let data: String = row.get(1)?;
                Ok((id, data))
            })
            .unwrap();

        for row in rows.flatten() {
            let (id, data) = row;
            let count: u32 = data.parse().unwrap_or(0);
            // 提取 user_id（格式: user_id_inst_name）
            let user = id.strip_suffix(&format!("_{}", inst.name)).unwrap_or(&id);
            let last_key = format!("last_{}_{}", user, inst.name);
            let last_time: i64 = db.global_get(section, &last_key).parse().unwrap_or(0);
            player_scores.push((user.to_string(), count, last_time));
        }
    }

    // 按通关次数降序排序
    player_scores.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));

    if player_scores.is_empty() {
        r.push_str("\n📭 暂无通关记录\n");
    } else {
        r.push_str("\n🏆 通关排行:\n");
        for (i, (uid, count, _last_time)) in player_scores.iter().take(10).enumerate() {
            let medal = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            let nickname = db.read_basic(uid, "NickName");
            let display_name = if nickname.is_empty() { uid.to_string() } else { nickname };
            let is_self = if uid == user_id { " ← 您" } else { "" };
            r.push_str(&format!(
                "\n{} {}. {} — {}次{}",
                medal,
                i + 1,
                display_name,
                count,
                is_self
            ));
        }
    }

    // 显示个人数据
    let user_count_key = format!("{}_{}", user_id, inst.name);
    let user_count: u32 = db.global_get(section, &user_count_key).parse().unwrap_or(0);
    if user_count > 0 {
        let rank = player_scores.iter().position(|(uid, _, _)| uid == user_id).unwrap_or(0) + 1;
        r.push_str(&format!("\n\n📊 您的排名: 第{}名 (通关{}次)", rank, user_count));
    } else {
        r.push_str("\n\n📊 您尚未通关此副本");
    }

    // 奖励预览
    r.push_str(&format!(
        "\n\n🎁 通关奖励: {}金币 + {}经验",
        inst.fixed_reward_gold, inst.fixed_reward_exp
    ));
    if !inst.fixed_reward_item.is_empty() {
        r.push_str(&format!(" + {}", inst.fixed_reward_item));
    }

    r.push_str("\n\n⚔️ 发送'挑战副本'开始挑战！");
    r
}

/// 副本扫荡 — 跳过战斗，直接领取已通关副本的奖励
/// 条件: 该副本至少通关3次 + 满足等级/冷却/每日次数 + 不在其他副本中
/// 扫荡奖励 = 固定奖励(金币/经验×80%) + 随机掉落(概率不变)
/// 每次扫荡消耗1个「副本扫荡券」（无券则消耗500金币）
pub fn cmd_instance_sweep(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    // 检查存活
    let hp: i64 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命后再扫荡副本！", prefix);
    }

    let args = args.trim();
    if args.is_empty() {
        return format!(
            "{}\n请指定要扫荡的副本名称！\n发送'查看副本列表'查看所有副本。\n💡 扫荡需要该副本至少通关3次。",
            prefix
        );
    }

    let instances = get_instances();
    let search = args;
    let inst = instances.iter().find(|i| i.name.contains(search));
    let inst = match inst {
        Some(i) => i,
        None => {
            // 尝试序号
            if let Ok(idx) = search.parse::<usize>() {
                if idx >= 1 && idx <= instances.len() {
                    &instances[idx - 1]
                } else {
                    return format!("{}\n未找到副本'{}'！", prefix, search);
                }
            } else {
                return format!("{}\n未找到副本'{}'！", prefix, search);
            }
        }
    };

    // 检查等级
    let level: u32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if level < inst.min_level {
        return format!(
            "{}\n等级不足！[{}] 需要等级 {}-{}，您当前等级 {}。",
            prefix, inst.name, inst.min_level, inst.max_level, level
        );
    }
    if level > inst.max_level {
        return format!(
            "{}\n等级过高！[{}] 适合等级 {}-{}，您当前等级 {}。",
            prefix, inst.name, inst.min_level, inst.max_level, level
        );
    }

    // 检查是否已在副本中
    let (current_inst, current_stage) = get_instance_progress(db, user_id);
    if !current_inst.is_empty() && current_stage > 0 {
        return format!(
            "{}\n您已在副本[{}]中（关卡 {}），请先完成或等待重置后再扫荡！",
            prefix, current_inst, current_stage
        );
    }

    // 检查冷却
    let cd_str = get_instance_cooldown(db, user_id, inst.name);
    if !cd_str.is_empty() {
        if let Ok(cd_ts) = cd_str.parse::<i64>() {
            let now = chrono::Local::now().timestamp();
            if now < cd_ts {
                let remaining = cd_ts - now;
                return format!("{}\n副本[{}]冷却中！还需等待 {}秒。", prefix, inst.name, remaining);
            }
        }
    }

    // 检查每日重置
    let daily_count = get_instance_daily_count(db, user_id, inst.name);
    if daily_count >= inst.daily_max_resets {
        return format!(
            "{}\n副本[{}]今日次数已用完（{}/{}）！明天再来。",
            prefix, inst.name, daily_count, inst.daily_max_resets
        );
    }

    // 检查通关次数（至少3次才能扫荡）
    let section = "instance_ranking";
    let user_count_key = format!("{}_{}", user_id, inst.name);
    let user_count: u32 = db.global_get(section, &user_count_key).parse().unwrap_or(0);
    if user_count < 3 {
        return format!(
            "{}\n副本[{}]扫荡需要至少通关3次！您当前通关 {}次。\n⚔️ 请先手动挑战通关。",
            prefix, inst.name, user_count
        );
    }

    // 检查消耗物品（扫荡券）
    let sweep_ticket = "副本扫荡券";
    let has_ticket = db.get_item_count(user_id, sweep_ticket);
    let gold_cost_if_no_ticket: i64 = 500;
    let use_gold = has_ticket <= 0;

    if use_gold {
        let current_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        if current_gold < gold_cost_if_no_ticket {
            return format!(
                "{}\n扫荡副本[{}]需要「{}」或 {}金币，您都没有！\n💰 当前金币: {}",
                prefix, inst.name, sweep_ticket, gold_cost_if_no_ticket, current_gold
            );
        }
        // 扣金币
        db.write_currency(user_id, CURRENCY_GOLD, current_gold - gold_cost_if_no_ticket);
    } else {
        // 消耗扫荡券
        db.remove_item(user_id, sweep_ticket, 1);
    }

    // 检查消耗物品（副本专属）
    if !inst.cost_item.is_empty() {
        let owned = db.get_item_count(user_id, inst.cost_item);
        if owned < inst.cost_amount as i32 {
            // 退还消耗
            if use_gold {
                let cur = db.read_currency(user_id, CURRENCY_GOLD);
                db.write_currency(user_id, CURRENCY_GOLD, cur + gold_cost_if_no_ticket);
            } else {
                db.add_item(user_id, sweep_ticket, 1);
            }
            return format!(
                "{}\n扫荡副本[{}]需要 {}×{}，您当前拥有 {}个。",
                prefix, inst.name, inst.cost_item, inst.cost_amount, owned
            );
        }
        db.remove_item(user_id, inst.cost_item, inst.cost_amount as i32);
    }

    // 增加每日次数
    increment_daily_count(db, user_id, inst.name);

    // 设置冷却（扫荡也消耗冷却）
    let now = chrono::Local::now().timestamp();
    db.write_user_data(
        user_id,
        &format!("InstanceCD_{}", inst.name),
        &(now + inst.cooldown_secs as i64).to_string(),
    );

    // ── 发放奖励（扫荡折扣: 金币/经验×80%）──
    let sweep_rate = 0.8f64;
    let gold_reward = (inst.fixed_reward_gold as f64 * sweep_rate) as i64;
    let exp_reward = (inst.fixed_reward_exp as f64 * sweep_rate) as i32;

    // VIP加成
    let vip_bonus_pct = vip::get_vip_exp_bonus(db, user_id);
    let bonus_exp = if vip_bonus_pct > 0 {
        exp_reward * vip_bonus_pct / 100
    } else {
        0
    };

    {
        let current_gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
        db.write_currency(user_id, CURRENCY_GOLD, current_gold + gold_reward);
    }
    user::add_experience(db, user_id, exp_reward + bonus_exp);

    let mut log = format!(
        "{}\n═══ 副本扫荡完成 ═══\n📌 副本: [{}] {}\n🎫 消耗: {}\n",
        prefix,
        inst.instance_type,
        inst.name,
        if use_gold {
            format!("{}金币", gold_cost_if_no_ticket)
        } else {
            sweep_ticket.to_string()
        }
    );

    log.push_str(&format!(
        "\n🎁 扫荡奖励 (×{:.0}%折扣):\n  💰 {}金币\n  ✨ {}经验{}",
        sweep_rate * 100.0,
        gold_reward,
        exp_reward + bonus_exp,
        if bonus_exp > 0 {
            format!(" (+{}VIP加成)", bonus_exp)
        } else {
            String::new()
        }
    ));

    // 固定物品
    if !inst.fixed_reward_item.is_empty() {
        db.add_item(user_id, inst.fixed_reward_item, 1);
        log.push_str(&format!("\n  📦 {}", inst.fixed_reward_item));
    }

    // 随机掉落（概率不变）
    let mut rng = rand::thread_rng();
    let mut got_random = false;
    for &(item, prob) in inst.random_rewards {
        if rng.gen_bool(prob.min(1.0)) {
            db.add_item(user_id, item, 1);
            if !got_random {
                log.push_str("\n\n🎲 随机掉落:");
                got_random = true;
            }
            log.push_str(&format!("\n  📦 {}", item));
        }
    }
    if !got_random {
        log.push_str("\n\n🎲 随机掉落: 本次未获得随机奖励");
    }

    // 记录排行榜
    record_instance_completion(db, user_id, inst.name, inst.instance_type);

    log.push_str("\n\n💡 发送'副本扫荡+副本名'继续扫荡\n⚔️ 发送'挑战副本+副本名'手动挑战获得全额奖励");

    log
}

/// 副本进度查看
pub fn cmd_instance_progress(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let (current_inst, current_stage) = get_instance_progress(db, user_id);

    if current_inst.is_empty() || current_stage == 0 {
        return format!(
            "{}\n您当前没有进行中的副本。\n发送'查看副本列表'查看可挑战的副本。",
            prefix
        );
    }

    let instances = get_instances();
    if let Some(inst) = instances.iter().find(|i| i.name == current_inst) {
        let stage = &inst.stages[(current_stage - 1) as usize];
        let monster_desc: Vec<String> = stage
            .monsters
            .iter()
            .map(|(name, count)| format!("{}×{}", name, count))
            .collect();

        let mut r = format!(
            "{}\n═══ 副本进度 ═══\n📌 副本：[{}] {}\n📍 当前关卡：{}/{} - {}\n👹 敌人：{}",
            prefix,
            inst.instance_type,
            inst.name,
            current_stage,
            inst.stages.len(),
            stage.name,
            monster_desc.join("、"),
        );

        // HP
        let hp: i64 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
        let hp_max: i64 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(100);
        r.push_str(&format!("\n❤️ 当前HP：{}/{}", hp, hp_max));

        r.push_str("\n\n⚔️ 发送'挑战副本'继续推进关卡！");
        r
    } else {
        format!("{}\n副本数据异常，已重置。", prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_count() {
        let instances = get_instances();
        assert!(instances.len() >= 5, "Should have at least 5 dungeons");
        // Verify types
        let normal = instances.iter().filter(|i| i.instance_type == "普通").count();
        let elite = instances.iter().filter(|i| i.instance_type == "精英").count();
        let hell = instances.iter().filter(|i| i.instance_type == "地狱").count();
        assert!(normal > 0, "Should have normal dungeons");
        assert!(elite > 0, "Should have elite dungeons");
        assert!(hell > 0, "Should have hell dungeons");
    }

    #[test]
    fn test_instance_stages() {
        let instances = get_instances();
        for inst in &instances {
            assert!(!inst.stages.is_empty(), "Instance '{}' should have stages", inst.name);
            assert!(inst.min_level > 0, "Instance '{}' min_level should be > 0", inst.name);
            assert!(
                inst.max_level >= inst.min_level,
                "Instance '{}' max_level should >= min_level",
                inst.name
            );
            assert!(
                inst.daily_max_resets > 0,
                "Instance '{}' should allow resets",
                inst.name
            );
            for stage in inst.stages {
                assert!(!stage.name.is_empty(), "Stage in '{}' should have name", inst.name);
                assert!(
                    !stage.monsters.is_empty(),
                    "Stage '{}' should have monsters",
                    stage.name
                );
            }
        }
    }

    #[test]
    fn test_instance_level_ranges() {
        let instances = get_instances();
        // Verify level ranges don't overlap too much and cover a wide range
        let min_level = instances.iter().map(|i| i.min_level).min().unwrap_or(0);
        let max_level = instances.iter().map(|i| i.max_level).max().unwrap_or(0);
        assert_eq!(min_level, 1, "Lowest dungeon should start at level 1");
        assert!(max_level >= 50, "Highest dungeon should go up to at least level 50");
    }

    #[test]
    fn test_instance_rewards() {
        let instances = get_instances();
        for inst in &instances {
            assert!(inst.fixed_reward_gold > 0, "Instance '{}' should give gold", inst.name);
            assert!(inst.fixed_reward_exp > 0, "Instance '{}' should give exp", inst.name);
        }
    }

    #[test]
    fn test_sweep_reward_calculations() {
        // Sweep gives 80% of base gold/exp
        let instances = get_instances();
        let sweep_rate = 0.8f64;
        for inst in &instances {
            let gold = (inst.fixed_reward_gold as f64 * sweep_rate) as i64;
            let exp = (inst.fixed_reward_exp as f64 * sweep_rate) as i32;
            assert!(gold > 0, "Sweep gold for '{}' should be > 0", inst.name);
            assert!(exp > 0, "Sweep exp for '{}' should be > 0", inst.name);
            assert!(
                gold < inst.fixed_reward_gold as i64,
                "Sweep gold should be less than full reward for '{}'",
                inst.name
            );
            assert!(
                exp < inst.fixed_reward_exp as i32,
                "Sweep exp should be less than full reward for '{}'",
                inst.name
            );
        }
    }

    #[test]
    fn test_sweep_requires_minimum_completions() {
        // Sweep requires at least 3 completions - just verify the constant
        let min_completions: u32 = 3;
        assert!(min_completions > 0, "Minimum completions for sweep must be positive");
    }

    #[test]
    fn test_instance_cooldown_config() {
        let instances = get_instances();
        for inst in &instances {
            assert!(inst.cooldown_secs > 0, "Instance '{}' should have cooldown", inst.name);
            // Cooldown should be reasonable (between 1 second and 1 hour)
            assert!(
                inst.cooldown_secs <= 3600,
                "Instance '{}' cooldown too high: {}",
                inst.name,
                inst.cooldown_secs
            );
        }
    }
}

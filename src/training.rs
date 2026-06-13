/// CakeGame 锻造/训练系统
/// 来自 ext_duanlian_info 表 — 吐纳/冥想/练武/习法 四种训练方式
/// 每种训练消耗资源，永久提升对应属性
use crate::core::*;
use crate::db::Database;
use crate::user;
use rand::Rng;

/// 训练类型配置
struct TrainingType {
    name: &'static str,
    attr_name: &'static str,
    attr_key: &'static str,
    base_gain: i32,
    gold_cost: i32,
    diamond_cost: i32,
    daily_limit: i32,
}

const TRAINING_TYPES: &[TrainingType] = &[
    TrainingType {
        name: "吐纳",
        attr_name: "生命",
        attr_key: "training_hp",
        base_gain: 10,
        gold_cost: 500,
        diamond_cost: 0,
        daily_limit: 10,
    },
    TrainingType {
        name: "冥想",
        attr_name: "魔法",
        attr_key: "training_mp",
        base_gain: 8,
        gold_cost: 500,
        diamond_cost: 0,
        daily_limit: 10,
    },
    TrainingType {
        name: "练武",
        attr_name: "物攻",
        attr_key: "training_ad",
        base_gain: 5,
        gold_cost: 800,
        diamond_cost: 0,
        daily_limit: 5,
    },
    TrainingType {
        name: "习法",
        attr_name: "魔攻",
        attr_key: "training_ap",
        base_gain: 5,
        gold_cost: 800,
        diamond_cost: 0,
        daily_limit: 5,
    },
];

/// 查看锻造系统（锻造列表）
pub fn cmd_training_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut r = format!("{}\n═══ 锻造/训练系统 ═══", prefix);
    r.push_str("\n通过修炼可以永久提升属性！");

    for tt in TRAINING_TYPES {
        let done_today: i32 = db
            .read_user_data(user_id, &format!("{}_date_{}", tt.attr_key, today))
            .parse()
            .unwrap_or(0);
        let total_done: i32 = db.read_user_data(user_id, tt.attr_key).parse().unwrap_or(0);

        r.push_str(&format!("\n\n▸ {} — {} +{}/次", tt.name, tt.attr_name, tt.base_gain));
        r.push_str(&format!(
            "\n  消耗: {}金币{}",
            tt.gold_cost,
            if tt.diamond_cost > 0 {
                format!(" +{}钻石", tt.diamond_cost)
            } else {
                String::new()
            }
        ));
        r.push_str(&format!(
            "\n  今日: {}/{}次 | 累计: {}次",
            done_today, tt.daily_limit, total_done
        ));
    }

    r.push_str("\n\n使用方法: 发送 '吐纳' '冥想' '练武' '习法'");
    r
}

/// 通用训练处理
fn do_training(db: &Database, user_id: &str, training: &TrainingType) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // 检查每日次数限制
    let date_key = format!("{}_date_{}", training.attr_key, today);
    let done_today: i32 = db.read_user_data(user_id, &date_key).parse().unwrap_or(0);

    if done_today >= training.daily_limit {
        return format!(
            "{}\n今日{}次数已达上限({}/{})！\n明日再来吧。",
            prefix, training.name, done_today, training.daily_limit
        );
    }

    // 检查生命值
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再修炼。", prefix);
    }

    // 检查金币
    let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
    if user_gold < training.gold_cost {
        return format!(
            "{}\n{}需要 {}金币，你只有 {}金币。",
            prefix, training.name, training.gold_cost, user_gold
        );
    }

    // 检查钻石
    if training.diamond_cost > 0 {
        let user_diamond: i32 = db.read_currency(user_id, CURRENCY_DIAMOND) as i32;
        if user_diamond < training.diamond_cost {
            return format!(
                "{}\n{}需要 {}钻石，你只有 {}钻石。",
                prefix, training.name, training.diamond_cost, user_diamond
            );
        }
        db.write_currency(user_id, CURRENCY_DIAMOND, (user_diamond - training.diamond_cost) as i64);
    }

    // 扣除金币
    db.write_currency(user_id, CURRENCY_GOLD, (user_gold - training.gold_cost) as i64);

    // 计算收益（小幅随机浮动）
    let variance = rand::thread_rng().gen_range(-1..=2);
    let gain = (training.base_gain + variance).max(1);

    // 写入属性加成
    let bonus_key = format!("{}_bonus", training.attr_key);
    let current_bonus: i32 = db.read_user_data(user_id, &bonus_key).parse().unwrap_or(0);
    db.write_user_data(user_id, &bonus_key, &(current_bonus + gain).to_string());

    // 更新今日次数
    db.write_user_data(user_id, &date_key, &(done_today + 1).to_string());

    // 更新累计次数
    let total_done: i32 = db.read_user_data(user_id, training.attr_key).parse().unwrap_or(0);
    db.write_user_data(user_id, training.attr_key, &(total_done + 1).to_string());

    format!(
        "{}\n═══ {}修炼 ═══\n{}成功！\n\n{}永久 +{}\n消耗 {}金币\n今日进度: {}/{}",
        prefix,
        training.name,
        training.name,
        training.attr_name,
        gain,
        training.gold_cost,
        done_today + 1,
        training.daily_limit
    )
}

/// 吐纳 — 提升生命
pub fn cmd_training_hp(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    do_training(db, user_id, &TRAINING_TYPES[0])
}

/// 冥想 — 提升魔法
pub fn cmd_training_mp(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    do_training(db, user_id, &TRAINING_TYPES[1])
}

/// 练武 — 提升物攻
pub fn cmd_training_ad(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    do_training(db, user_id, &TRAINING_TYPES[2])
}

/// 习法 — 提升魔攻
pub fn cmd_training_ap(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    do_training(db, user_id, &TRAINING_TYPES[3])
}

// ==================== 自动修炼系统 (Auto-Evolution) ====================
// 来自 MessageTemplate: com.shdic.cg_auto_evolution
// 挂机修炼：开始后每60秒自动尝试突破，成功永久提升属性
// 战斗死亡/手动停止 会中断修炼

/// 自动修炼配置
struct AutoEvoConfig {
    target: &'static str,
    attr_name: &'static str,
    bonus_key: &'static str,
    cycle_secs: i64,   // 每周期秒数
    success_rate: f64, // 突破成功率
    min_gain: i32,     // 最小增幅
    max_gain: i32,     // 最大增幅
    max_session: i32,  // 单次修炼最大增幅
    gold_cost: i32,    // 每次突破消耗金币
}

const AUTO_EVO_CONFIGS: &[AutoEvoConfig] = &[
    AutoEvoConfig {
        target: "吐纳",
        attr_name: "生命",
        bonus_key: "training_hp_bonus",
        cycle_secs: 60,
        success_rate: 0.30,
        min_gain: 1,
        max_gain: 3,
        max_session: 50,
        gold_cost: 200,
    },
    AutoEvoConfig {
        target: "冥想",
        attr_name: "魔法",
        bonus_key: "training_mp_bonus",
        cycle_secs: 60,
        success_rate: 0.30,
        min_gain: 1,
        max_gain: 3,
        max_session: 50,
        gold_cost: 200,
    },
    AutoEvoConfig {
        target: "练武",
        attr_name: "物攻",
        bonus_key: "training_ad_bonus",
        cycle_secs: 90,
        success_rate: 0.25,
        min_gain: 1,
        max_gain: 2,
        max_session: 30,
        gold_cost: 300,
    },
    AutoEvoConfig {
        target: "习法",
        attr_name: "魔攻",
        bonus_key: "training_ap_bonus",
        cycle_secs: 90,
        success_rate: 0.25,
        min_gain: 1,
        max_gain: 2,
        max_session: 30,
        gold_cost: 300,
    },
];

fn find_auto_evo_config(target: &str) -> Option<&'static AutoEvoConfig> {
    AUTO_EVO_CONFIGS.iter().find(|c| c.target == target)
}

/// 开始修炼 — 启动挂机修炼
pub fn cmd_auto_evo_start(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 检查是否已在修炼中
    let current_target = db.read_user_data(user_id, "auto_evo_target");
    if !current_target.is_empty() {
        let start_time = db.read_user_data(user_id, "auto_evo_start");
        return format!(
            "{}\n你已经在修炼中啦！（开始修炼时间：{}）\n发送「停止修炼」可停止，或发送「修炼状态」查看进度。",
            prefix, start_time
        );
    }

    let target = args.trim();
    if target.is_empty() {
        return format!(
            "{}\n请指定修炼目标！\n可选：吐纳(生命) / 冥想(魔法) / 练武(物攻) / 习法(魔攻)\n\n用法: 开始修炼+吐纳",
            prefix
        );
    }

    let config = match find_auto_evo_config(target) {
        Some(c) => c,
        None => {
            return format!(
                "{}\n未找到修炼类型「{}」！\n可选：吐纳(生命) / 冥想(魔法) / 练武(物攻) / 习法(魔攻)",
                prefix, target
            );
        }
    };

    // 检查生命值
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        return format!("{}\n您已阵亡，请先恢复生命再修炼。", prefix);
    }

    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 启动修炼
    db.write_user_data(user_id, "auto_evo_target", config.target);
    db.write_user_data(user_id, "auto_evo_start", &now);
    db.write_user_data(user_id, "auto_evo_gains", "0");
    db.write_user_data(user_id, "auto_evo_last_cycle", &now);
    db.write_user_data(user_id, "auto_evo_cycles", "0");

    let success_pct = (config.success_rate * 100.0) as i32;

    format!(
        "{}\n你开始修炼[{}]！\n大约每修炼{}秒能尝试一次突破（概率约{}%）！\n如果成功有望增加{}(+{}/次)，直到修炼被打断。\n或者达到这一次的修炼上限（{}）。\n\nTip:发送'修炼状态'可查询修炼情况，或者发送'停止修炼'停止修炼",
        prefix, config.attr_name, config.cycle_secs, success_pct,
        config.attr_name, config.max_gain, config.max_session
    )
}

/// 停止修炼 — 终止挂机修炼
pub fn cmd_auto_evo_stop(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let current_target = db.read_user_data(user_id, "auto_evo_target");
    if current_target.is_empty() {
        return format!("{}\n你当前没有在修炼中。", prefix);
    }

    // 先结算当前进度
    let settle_msg = settle_auto_evo(db, user_id);

    // 清除修炼状态
    clear_auto_evo(db, user_id);

    format!("{}\n修炼已经被主动停止！\n{}", prefix, settle_msg)
}

/// 修炼状态 — 查看当前挂机修炼进度
pub fn cmd_auto_evo_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let current_target = db.read_user_data(user_id, "auto_evo_target");
    if current_target.is_empty() {
        return format!("{}\n你当前没有在修炼中。\n发送「开始修炼+吐纳」开始挂机修炼！", prefix);
    }

    let config = match find_auto_evo_config(&current_target) {
        Some(c) => c,
        None => {
            clear_auto_evo(db, user_id);
            return format!("{}\n修炼数据异常，已自动停止。", prefix);
        }
    };

    // 结算收益
    let settle_msg = settle_auto_evo(db, user_id);

    let gains: i32 = db.read_user_data(user_id, "auto_evo_gains").parse().unwrap_or(0);
    let cycles: i32 = db.read_user_data(user_id, "auto_evo_cycles").parse().unwrap_or(0);
    let start_time = db.read_user_data(user_id, "auto_evo_start");

    // 检查是否达到上限
    if gains >= config.max_session {
        clear_auto_evo(db, user_id);
        return format!(
            "{}\n修炼已经结束！（已达上限 {}）\n{}\n累计突破 {} 次，{} 共 +{}",
            prefix, config.max_session, settle_msg, cycles, config.attr_name, gains
        );
    }

    // 检查生命
    let hp_current: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
    if hp_current <= 0 {
        clear_auto_evo(db, user_id);
        return format!("{}\n你上次的修炼被迫中断，中断原因[战斗失败]\n{}", prefix, settle_msg);
    }

    // 计算下次突破时间
    let last_cycle_str = db.read_user_data(user_id, "auto_evo_last_cycle");
    let next_breakthrough =
        if let Ok(last) = chrono::NaiveDateTime::parse_from_str(&last_cycle_str, "%Y-%m-%d %H:%M:%S") {
            let next = last + chrono::Duration::seconds(config.cycle_secs);
            let now = chrono::Local::now().naive_local();
            if next <= now {
                "即将到来...".to_string()
            } else {
                let diff = (next - now).num_seconds();
                format!("{}秒后", diff)
            }
        } else {
            "计算中...".to_string()
        };

    let mut result = format!(
        "{}\n═══ 修炼状态 ═══\n修炼目标: {}\n开始时间: {}\n已突破: {}次 | {} +{}",
        prefix, config.attr_name, start_time, cycles, config.attr_name, gains
    );

    if !settle_msg.is_empty() {
        result.push_str(&format!("\n{}", settle_msg));
    }

    result.push_str(&format!(
        "\n修炼上限: {}/{}\n下次突破: {}\n\nTip:发送'停止修炼'停止",
        gains, config.max_session, next_breakthrough
    ));

    result
}

/// 结算自动修炼收益 — 计算经过的周期，尝试突破
fn settle_auto_evo(db: &Database, user_id: &str) -> String {
    let current_target = db.read_user_data(user_id, "auto_evo_target");
    if current_target.is_empty() {
        return String::new();
    }

    let config = match find_auto_evo_config(&current_target) {
        Some(c) => c,
        None => return String::new(),
    };

    let gains: i32 = db.read_user_data(user_id, "auto_evo_gains").parse().unwrap_or(0);
    if gains >= config.max_session {
        return String::new();
    }

    let last_cycle_str = db.read_user_data(user_id, "auto_evo_last_cycle");
    let last_cycle = match chrono::NaiveDateTime::parse_from_str(&last_cycle_str, "%Y-%m-%d %H:%M:%S") {
        Ok(t) => t,
        Err(_) => return String::new(),
    };

    let now = chrono::Local::now().naive_local();
    let elapsed = (now - last_cycle).num_seconds();
    let cycles_to_process = elapsed / config.cycle_secs;

    if cycles_to_process <= 0 {
        return String::new();
    }

    let mut total_gained = 0i32;
    let mut total_cycles: i32 = db.read_user_data(user_id, "auto_evo_cycles").parse().unwrap_or(0);
    let mut current_gains = gains;
    let mut rng = rand::thread_rng();

    for _ in 0..cycles_to_process {
        if current_gains >= config.max_session {
            break;
        }

        total_cycles += 1;

        // 检查金币是否足够
        let user_gold: i32 = db.read_currency(user_id, CURRENCY_GOLD) as i32;
        if user_gold < config.gold_cost {
            // 金币不足，暂停修炼（不中断，只是不结算）
            break;
        }

        // 尝试突破
        if rng.gen_bool(config.success_rate) {
            let gain = rng.gen_range(config.min_gain..=config.max_gain);
            let gain = gain.min(config.max_session - current_gains);
            if gain > 0 {
                total_gained += gain;
                current_gains += gain;
                // 扣除金币
                db.write_currency(user_id, CURRENCY_GOLD, (user_gold - config.gold_cost) as i64);
            }
        }
    }

    // 更新用户数据
    if total_gained > 0 {
        let current_bonus: i32 = db.read_user_data(user_id, config.bonus_key).parse().unwrap_or(0);
        db.write_user_data(user_id, config.bonus_key, &(current_bonus + total_gained).to_string());
        // 成就追踪
        crate::achievement::on_training_breakthrough(db, user_id);
    }

    db.write_user_data(user_id, "auto_evo_gains", &current_gains.to_string());
    db.write_user_data(user_id, "auto_evo_cycles", &total_cycles.to_string());
    db.write_user_data(
        user_id,
        "auto_evo_last_cycle",
        &now.format("%Y-%m-%d %H:%M:%S").to_string(),
    );

    if total_gained > 0 {
        format!("{} 已累计增加了{}！", config.attr_name, total_gained)
    } else {
        String::new()
    }
}

/// 清除自动修炼状态
fn clear_auto_evo(db: &Database, user_id: &str) {
    db.write_user_data(user_id, "auto_evo_target", "");
    db.write_user_data(user_id, "auto_evo_start", "");
    db.write_user_data(user_id, "auto_evo_gains", "");
    db.write_user_data(user_id, "auto_evo_last_cycle", "");
    db.write_user_data(user_id, "auto_evo_cycles", "");
}

/// 中断修炼（战斗死亡调用）
pub fn interrupt_auto_evo(db: &Database, user_id: &str) -> String {
    let current_target = db.read_user_data(user_id, "auto_evo_target");
    if current_target.is_empty() {
        return String::new();
    }

    let settle_msg = settle_auto_evo(db, user_id);
    clear_auto_evo(db, user_id);

    if settle_msg.is_empty() {
        "你的修炼被迫中断！".to_string()
    } else {
        format!("你上次的修炼被迫中断，中断原因[战斗失败]\n{}", settle_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_types_count() {
        assert_eq!(TRAINING_TYPES.len(), 4, "Should have 4 training types");
    }

    #[test]
    fn test_training_types_valid() {
        for tt in TRAINING_TYPES {
            assert!(!tt.name.is_empty(), "Training name must not be empty");
            assert!(!tt.attr_name.is_empty(), "Attr name must not be empty");
            assert!(!tt.attr_key.is_empty(), "Attr key must not be empty");
            assert!(tt.base_gain > 0, "Base gain must be positive");
            assert!(tt.gold_cost > 0, "Gold cost must be positive");
            assert!(tt.daily_limit > 0, "Daily limit must be positive");
        }
    }

    #[test]
    fn test_training_types_unique_names() {
        let mut names: Vec<&str> = TRAINING_TYPES.iter().map(|t| t.name).collect();
        let original_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), original_len, "Training names must be unique");
    }

    #[test]
    fn test_find_auto_evo_config_valid() {
        assert!(find_auto_evo_config("吐纳").is_some());
        assert!(find_auto_evo_config("冥想").is_some());
        assert!(find_auto_evo_config("练武").is_some());
        assert!(find_auto_evo_config("习法").is_some());
    }

    #[test]
    fn test_find_auto_evo_config_invalid() {
        assert!(find_auto_evo_config("不存在").is_none());
        assert!(find_auto_evo_config("").is_none());
        assert!(find_auto_evo_config("修炼").is_none());
    }

    #[test]
    fn test_auto_evo_configs_count() {
        assert_eq!(AUTO_EVO_CONFIGS.len(), 4, "Should have 4 auto evolution configs");
    }

    #[test]
    fn test_auto_evo_configs_valid() {
        for cfg in AUTO_EVO_CONFIGS {
            assert!(!cfg.target.is_empty(), "Target must not be empty");
            assert!(!cfg.attr_name.is_empty(), "Attr name must not be empty");
            assert!(!cfg.bonus_key.is_empty(), "Bonus key must not be empty");
            assert!(cfg.cycle_secs > 0, "Cycle seconds must be positive");
            assert!(
                cfg.success_rate > 0.0 && cfg.success_rate <= 1.0,
                "Success rate must be in (0, 1]"
            );
            assert!(cfg.min_gain > 0, "Min gain must be positive");
            assert!(cfg.max_gain >= cfg.min_gain, "Max gain must be >= min gain");
            assert!(cfg.max_session > 0, "Max session must be positive");
            assert!(cfg.gold_cost >= 0, "Gold cost must be non-negative");
        }
    }

    #[test]
    fn test_auto_evo_configs_match_training_types() {
        // Each auto evo config should correspond to a training type
        for cfg in AUTO_EVO_CONFIGS {
            assert!(
                TRAINING_TYPES.iter().any(|t| t.name == cfg.target),
                "Auto evo target '{}' should match a training type",
                cfg.target
            );
        }
    }
}

/// 挂机修炼系统（自动进化）
/// 来自 MessageTemplate com.shdic.cg_auto_evolution 系列 + Global 配置
/// 玩家启动后自动挂机修炼，周期性尝试突破，提升目标属性
use crate::core::*;
use crate::db::Database;
use crate::template_render;
use crate::user;
use rand::Rng;

/// 修炼配置（从 Global 表 com.shdic.cg_auto_evolutionapp 读取）
const DEFAULT_SUCCESS_RATE: i32 = 15; // 15% 成功率
const DEFAULT_MIN_GAIN: i32 = 2; // 每次成功最少+2
const DEFAULT_MAX_GAIN: i32 = 5; // 每次成功最多+5
const DEFAULT_MAX_INCREASE: i32 = 25; // 单次修炼上限
const DEFAULT_CYCLE_SECS: i64 = 90; // 最小周期90秒
const DEFAULT_GOLD_COST: i32 = 10000; // 每次突破消耗金币
const DEFAULT_DIAMOND_COST: i32 = 1; // 每次突破消耗钻石

/// 修炼目标类型
#[derive(Clone, Debug)]
enum CultivationTarget {
    Exp,     // 经验
    Gold,    // 金币
    Hp,      // 生命
    Mp,      // 魔法
    Ad,      // 物攻
    Ap,      // 魔攻
    Defense, // 防御
}

impl CultivationTarget {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "经验" => Some(Self::Exp),
            "金币" => Some(Self::Gold),
            "生命" => Some(Self::Hp),
            "魔法" => Some(Self::Mp),
            "物攻" => Some(Self::Ad),
            "魔攻" => Some(Self::Ap),
            "防御" => Some(Self::Defense),
            _ => None,
        }
    }

    fn display_name(&self) -> &str {
        match self {
            Self::Exp => "经验",
            Self::Gold => "金币",
            Self::Hp => "生命",
            Self::Mp => "魔法",
            Self::Ad => "物攻",
            Self::Ap => "魔攻",
            Self::Defense => "防御",
        }
    }

    fn emoji(&self) -> &str {
        match self {
            Self::Exp => "✨",
            Self::Gold => "💰",
            Self::Hp => "❤️",
            Self::Mp => "💙",
            Self::Ad => "⚔️",
            Self::Ap => "🔮",
            Self::Defense => "🛡️",
        }
    }

    fn bonus_suffix(&self) -> &str {
        match self {
            Self::Exp => "exp",
            Self::Gold => "gold",
            Self::Hp => "hp",
            Self::Mp => "mp",
            Self::Ad => "ad",
            Self::Ap => "ap",
            Self::Defense => "defense",
        }
    }

    /// 读取配置的修炼目标
    fn load_from_db(db: &Database) -> Self {
        let target_name = db.global_get("com.shdic.cg_auto_evolutionapp", "目标");
        Self::from_str(&target_name).unwrap_or(Self::Exp)
    }

    fn load_success_rate(db: &Database) -> i32 {
        let v: String = db.global_get("com.shdic.cg_auto_evolutionapp", "成功率");
        v.parse().unwrap_or(DEFAULT_SUCCESS_RATE)
    }

    fn load_max_increase(db: &Database) -> i32 {
        let v: String = db.global_get("com.shdic.cg_auto_evolutionapp", "最大增幅");
        v.parse().unwrap_or(DEFAULT_MAX_INCREASE)
    }

    fn load_cycle_secs(db: &Database) -> i64 {
        let v: String = db.global_get("com.shdic.cg_auto_evolutionapp", "最小周期");
        v.parse().unwrap_or(DEFAULT_CYCLE_SECS)
    }

    fn load_gain_range(db: &Database) -> (i32, i32) {
        let v: String = db.global_get("com.shdic.cg_auto_evolutionapp", "增幅");
        let parts: Vec<&str> = v.split('-').collect();
        if parts.len() == 2 {
            let lo = parts[0].parse().unwrap_or(DEFAULT_MIN_GAIN);
            let hi = parts[1].parse().unwrap_or(DEFAULT_MAX_GAIN);
            (lo, hi)
        } else {
            (DEFAULT_MIN_GAIN, DEFAULT_MAX_GAIN)
        }
    }
}

/// 读取用户修炼状态
fn get_cultivation_state(db: &Database, user_id: &str) -> Option<(i64, i32)> {
    let start_str = db.read_user_data(user_id, "auto_evo_start");
    let added_str = db.read_user_data(user_id, "auto_evo_added");
    if start_str.is_empty() {
        return None;
    }
    let start_ts: i64 = start_str.parse().unwrap_or(0);
    let added: i32 = added_str.parse().unwrap_or(0);
    Some((start_ts, added))
}

/// 存储修炼状态
fn set_cultivation_state(db: &Database, user_id: &str, start_ts: i64, added: i32) {
    db.write_user_data(user_id, "auto_evo_start", &start_ts.to_string());
    db.write_user_data(user_id, "auto_evo_added", &added.to_string());
}

/// 清除修炼状态
fn clear_cultivation_state(db: &Database, user_id: &str) {
    db.delete_user_data(user_id, "auto_evo_start");
    db.delete_user_data(user_id, "auto_evo_added");
}

/// 计算从 start_ts 到 now 已经过了多少个周期
fn calc_cycles_elapsed(start_ts: i64, cycle_secs: i64) -> i32 {
    let now = chrono::Utc::now().timestamp();
    let elapsed = now - start_ts;
    if elapsed <= 0 {
        return 0;
    }
    (elapsed / cycle_secs) as i32
}

/// 格式化剩余时间
fn format_remaining(next_cycle_at: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = next_cycle_at - now;
    if diff <= 0 {
        return "即将突破".to_string();
    }
    let mins = diff / 60;
    let secs = diff % 60;
    if mins > 0 {
        format!("{}分{}秒", mins, secs)
    } else {
        format!("{}秒", secs)
    }
}

/// 开始修炼
pub fn cmd_start_cultivation(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 检查是否已在修炼中
    if let Some((start_ts, added)) = get_cultivation_state(db, user_id) {
        let cycle_secs = CultivationTarget::load_cycle_secs(db);
        let max_increase = CultivationTarget::load_max_increase(db);
        let cycles = calc_cycles_elapsed(start_ts, cycle_secs);
        let target = CultivationTarget::load_from_db(db);
        let (lo, hi) = CultivationTarget::load_gain_range(db);
        let success_rate = CultivationTarget::load_success_rate(db);

        // 自动处理已经过的周期
        let result = process_cycles(
            db,
            user_id,
            start_ts,
            added,
            cycles,
            &target,
            cycle_secs,
            max_increase,
            success_rate,
            lo,
            hi,
        );

        let cycle_end = start_ts + ((result.new_cycles_done as i64 + 1) * cycle_secs);
        let remaining = format_remaining(cycle_end);

        let mut out = format!(
            "{}\n{}",
            prefix,
            template_render::render_cultivation_in_progress(db, &format_timestamp(start_ts))
        );
        out.push_str(&format!(
            "\n{}",
            template_render::render_cultivation_cumulative(db, target.display_name(), result.new_added)
        ));
        if result.new_added >= max_increase {
            out.push_str(&format!("\n{}", template_render::render_cultivation_exp_full(db)));
        } else {
            out.push_str(&format!(
                "\n{}",
                template_render::render_cultivation_next_breakthrough(db, &remaining)
            ));
        }
        if !result.events.is_empty() {
            out.push_str("\n\n📜 修炼日志:");
            for e in &result.events {
                out.push_str(&format!("\n  {}", e));
            }
        }
        return out;
    }

    // 解析目标参数（可选：默认为配置中的目标）
    let target = if !args.trim().is_empty() {
        let t = CultivationTarget::from_str(args.trim());
        if t.is_none() {
            return format!(
                "{}\n⚠️ 未知修炼目标 [{}]\n可选: 经验/金币/生命/魔法/物攻/魔攻/防御",
                prefix,
                args.trim()
            );
        }
        t.unwrap()
    } else {
        CultivationTarget::load_from_db(db)
    };

    let now = chrono::Utc::now().timestamp();
    set_cultivation_state(db, user_id, now, 0);

    let cycle_secs = CultivationTarget::load_cycle_secs(db);
    let success_rate = CultivationTarget::load_success_rate(db);
    let (lo, hi) = CultivationTarget::load_gain_range(db);
    let max_increase = CultivationTarget::load_max_increase(db);

    // 模板消息
    let mut out = format!("{}\n🧘 修炼开始！", prefix);
    out.push_str(&format!("\n🎯 修炼目标: {}{}", target.emoji(), target.display_name()));
    out.push_str(&format!("\n⏱️ 修炼周期: {}秒", cycle_secs));
    out.push_str(&format!("\n🎲 突破概率: {}%", success_rate));
    out.push_str(&format!("\n📈 每次成功: +{}~{}", lo, hi));
    out.push_str(&format!("\n🎯 本次上限: +{}", max_increase));
    out.push_str("\n\n💡 发送「开始修炼」可查询修炼进度");
    out.push_str("\n💡 发送「停止修炼」可结束修炼");
    out
}

/// 停止修炼
pub fn cmd_stop_cultivation(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if let Some((start_ts, added)) = get_cultivation_state(db, user_id) {
        // 先处理完当前周期
        let cycle_secs = CultivationTarget::load_cycle_secs(db);
        let max_increase = CultivationTarget::load_max_increase(db);
        let target = CultivationTarget::load_from_db(db);
        let (lo, hi) = CultivationTarget::load_gain_range(db);
        let success_rate = CultivationTarget::load_success_rate(db);
        let cycles = calc_cycles_elapsed(start_ts, cycle_secs);
        let result = process_cycles(
            db,
            user_id,
            start_ts,
            added,
            cycles,
            &target,
            cycle_secs,
            max_increase,
            success_rate,
            lo,
            hi,
        );

        clear_cultivation_state(db, user_id);

        let mut out = format!("{}\n{}", prefix, template_render::render_cultivation_stopped(db));
        out.push_str("\n📊 本次修炼总结:");
        out.push_str(&format!(
            "\n{} {} 累计增加: +{}",
            target.emoji(),
            target.display_name(),
            result.new_added
        ));
        out.push_str(&format!("\n🔄 总突破次数: {}", result.breakthroughs));
        out.push_str(&format!(
            "\n⏱️ 修炼时长: {}",
            format_duration(cycles * cycle_secs as i32)
        ));
        if !result.events.is_empty() {
            out.push_str("\n\n📜 最近修炼日志:");
            for e in result.events.iter().rev().take(5) {
                out.push_str(&format!("\n  {}", e));
            }
        }
        return out;
    }

    format!("{}\n⚠️ 你当前没有在修炼中。发送「开始修炼」开始修炼！", prefix)
}

/// 查看修炼进度
pub fn cmd_view_cultivation(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let target = CultivationTarget::load_from_db(db);
    let cycle_secs = CultivationTarget::load_cycle_secs(db);
    let success_rate = CultivationTarget::load_success_rate(db);
    let max_increase = CultivationTarget::load_max_increase(db);
    let (lo, hi) = CultivationTarget::load_gain_range(db);

    if let Some((start_ts, added)) = get_cultivation_state(db, user_id) {
        let cycles = calc_cycles_elapsed(start_ts, cycle_secs);
        // 处理已过的周期
        let result = process_cycles(
            db,
            user_id,
            start_ts,
            added,
            cycles,
            &target,
            cycle_secs,
            max_increase,
            success_rate,
            lo,
            hi,
        );

        let cycle_end = start_ts + ((result.new_cycles_done as i64 + 1) * cycle_secs);
        let remaining = format_remaining(cycle_end);

        let mut out = format!("{}\n═══ 🧘 修炼进度 ═══", prefix);
        out.push_str(&format!("\n🎯 修炼目标: {}{}", target.emoji(), target.display_name()));
        out.push_str(&format!("\n📈 已增加: +{}/{}", result.new_added, max_increase));

        // 进度条
        let pct = if max_increase > 0 {
            (result.new_added as f64 / max_increase as f64 * 10.0) as i32
        } else {
            0
        };
        let filled = pct.min(10);
        let bar: String = "█".repeat(filled as usize) + &"░".repeat(10 - filled as usize);
        out.push_str(&format!("\n[{}] {}%", bar, pct * 10));

        out.push_str(&format!("\n🎲 突破概率: {}%/周期", success_rate));
        out.push_str(&format!("\n⏱️ 修炼周期: {}秒", cycle_secs));
        out.push_str(&format!("\n🔄 已完成周期: {}", cycles));
        out.push_str(&format!("\n🏆 突破次数: {}", result.breakthroughs));

        if result.new_added >= max_increase {
            out.push_str(&format!("\n\n{}", template_render::render_cultivation_exp_full(db)));
        } else {
            out.push_str(&format!(
                "\n{}",
                template_render::render_cultivation_next_breakthrough(db, &remaining)
            ));
        }

        if !result.events.is_empty() {
            out.push_str("\n\n📜 修炼日志:");
            for e in result.events.iter().rev().take(5) {
                out.push_str(&format!("\n  {}", e));
            }
        }
        return out;
    }

    // 未在修炼 - 显示系统说明
    let mut out = format!("{}\n═══ 🧘 挂机修炼系统 ═══", prefix);
    out.push_str("\n挂机修炼可以在离线时自动提升属性！");
    out.push_str(&format!(
        "\n\n🎯 当前修炼目标: {}{}",
        target.emoji(),
        target.display_name()
    ));
    out.push_str(&format!("\n⏱️ 修炼周期: {}秒", cycle_secs));
    out.push_str(&format!("\n🎲 突破概率: {}%", success_rate));
    out.push_str(&format!("\n📈 每次成功: +{}~{}", lo, hi));
    out.push_str(&format!("\n🎯 单次上限: +{}", max_increase));
    out.push_str(&format!(
        "\n💰 每次突破消耗: {}金币, {}钻石",
        DEFAULT_GOLD_COST, DEFAULT_DIAMOND_COST
    ));
    out.push_str("\n\n💡 发送「开始修炼」开始修炼");
    out.push_str("\n💡 发送「开始修炼+目标」指定修炼目标(经验/金币/生命/魔法/物攻/魔攻/防御)");
    out
}

/// 修炼状态查询（内部，被其他指令调用时使用）
pub fn cmd_cultivation_status(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    cmd_view_cultivation(db, user_id, _args, _msg_type, _group)
}

/// 处理修炼周期结果
struct CycleResult {
    new_added: i32,
    new_cycles_done: i32,
    breakthroughs: i32,
    events: Vec<String>,
}

/// 处理从 start_ts 到现在已经过去的所有周期
#[allow(clippy::too_many_arguments)]
fn process_cycles(
    db: &Database,
    user_id: &str,
    start_ts: i64,
    old_added: i32,
    total_cycles: i32,
    target: &CultivationTarget,
    cycle_secs: i64,
    max_increase: i32,
    success_rate: i32,
    lo: i32,
    hi: i32,
) -> CycleResult {
    let mut added = old_added;
    let mut breakthroughs = 0;
    let mut events = Vec::new();

    // 之前已经处理过的周期数
    let old_cycles: i32 = db.read_user_data(user_id, "auto_evo_cycles").parse().unwrap_or(0);
    let new_cycles = total_cycles - old_cycles;

    if new_cycles <= 0 {
        return CycleResult {
            new_added: added,
            new_cycles_done: total_cycles,
            breakthroughs: 0,
            events,
        };
    }

    let mut rng = rand::thread_rng();

    for i in 0..new_cycles {
        if added >= max_increase {
            events.push(format!("已达到修炼上限(+{})，停止突破", max_increase));
            break;
        }

        // 尝试突破
        let roll = rng.gen_range(0..100);
        if roll < success_rate {
            // 成功突破
            let gain = rng.gen_range(lo..=hi);
            let actual_gain = gain.min(max_increase - added);
            added += actual_gain;
            breakthroughs += 1;

            // 应用收益
            apply_gain(db, user_id, target, actual_gain);

            // 消耗资源
            let current_gold = db.read_currency(user_id, CURRENCY_GOLD);
            if current_gold >= DEFAULT_GOLD_COST as i64 {
                db.write_currency(user_id, CURRENCY_GOLD, current_gold - DEFAULT_GOLD_COST as i64);
            }
            let current_diamond = db.read_currency(user_id, CURRENCY_DIAMOND);
            if current_diamond >= DEFAULT_DIAMOND_COST as i64 {
                db.write_currency(user_id, CURRENCY_DIAMOND, current_diamond - DEFAULT_DIAMOND_COST as i64);
            }

            let cycle_ts = start_ts + (((old_cycles + i + 1) as i64) * cycle_secs);
            events.push(format!(
                "{} 突破成功！{} +{}",
                format_timestamp_short(cycle_ts),
                target.emoji(),
                actual_gain
            ));
        }
    }

    // 更新状态
    db.write_user_data(user_id, "auto_evo_added", &added.to_string());
    db.write_user_data(user_id, "auto_evo_cycles", &total_cycles.to_string());

    CycleResult {
        new_added: added,
        new_cycles_done: total_cycles,
        breakthroughs,
        events,
    }
}

/// 应用修炼收益
fn apply_gain(db: &Database, user_id: &str, target: &CultivationTarget, gain: i32) {
    match target {
        CultivationTarget::Exp => {
            crate::user::add_experience(db, user_id, gain);
        }
        CultivationTarget::Gold => {
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gain as i64);
        }
        CultivationTarget::Hp
        | CultivationTarget::Mp
        | CultivationTarget::Ad
        | CultivationTarget::Ap
        | CultivationTarget::Defense => {
            // 属性加成存储在 user_data 中，与训练系统一致
            let bonus_key = format!("cultivation_{}_bonus", target.bonus_suffix());
            let current: i32 = db.read_user_data(user_id, &bonus_key).parse().unwrap_or(0);
            db.write_user_data(user_id, &bonus_key, &(current + gain).to_string());
        }
    }
}

/// 格式化时间戳为可读时间
fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| "未知".to_string())
}

fn format_timestamp_short(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "??:??".to_string())
}

/// 格式化持续时间
fn format_duration(secs: i32) -> String {
    if secs < 60 {
        format!("{}秒", secs)
    } else if secs < 3600 {
        format!("{}分{}秒", secs / 60, secs % 60)
    } else {
        format!("{}小时{}分", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cultivation_target_from_str() {
        assert!(matches!(
            CultivationTarget::from_str("经验"),
            Some(CultivationTarget::Exp)
        ));
        assert!(matches!(
            CultivationTarget::from_str("金币"),
            Some(CultivationTarget::Gold)
        ));
        assert!(matches!(
            CultivationTarget::from_str("生命"),
            Some(CultivationTarget::Hp)
        ));
        assert!(matches!(
            CultivationTarget::from_str("魔法"),
            Some(CultivationTarget::Mp)
        ));
        assert!(matches!(
            CultivationTarget::from_str("物攻"),
            Some(CultivationTarget::Ad)
        ));
        assert!(matches!(
            CultivationTarget::from_str("魔攻"),
            Some(CultivationTarget::Ap)
        ));
        assert!(matches!(
            CultivationTarget::from_str("防御"),
            Some(CultivationTarget::Defense)
        ));
        assert!(CultivationTarget::from_str("不存在").is_none());
    }

    #[test]
    fn test_cultivation_target_display() {
        assert_eq!(CultivationTarget::Exp.display_name(), "经验");
        assert_eq!(CultivationTarget::Exp.emoji(), "✨");
        assert_eq!(CultivationTarget::Ad.display_name(), "物攻");
        assert_eq!(CultivationTarget::Ad.emoji(), "⚔️");
    }

    #[test]
    fn test_calc_cycles_elapsed() {
        let now = chrono::Utc::now().timestamp();
        // 0 seconds elapsed = 0 cycles
        assert_eq!(calc_cycles_elapsed(now, 90), 0);
        // 89 seconds = 0 cycles
        assert_eq!(calc_cycles_elapsed(now - 89, 90), 0);
        // 90 seconds = 1 cycle
        assert_eq!(calc_cycles_elapsed(now - 90, 90), 1);
        // 270 seconds = 3 cycles
        assert_eq!(calc_cycles_elapsed(now - 270, 90), 3);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30秒");
        assert_eq!(format_duration(90), "1分30秒");
        assert_eq!(format_duration(3661), "1小时1分");
    }

    #[test]
    fn test_format_remaining() {
        let now = chrono::Utc::now().timestamp();
        // Already passed
        let result = format_remaining(now - 10);
        assert_eq!(result, "即将突破");
        // 45 seconds from now
        let result = format_remaining(now + 45);
        assert_eq!(result, "45秒");
        // 2 minutes from now
        let result = format_remaining(now + 125);
        assert_eq!(result, "2分5秒");
    }

    #[test]
    fn test_gain_range_defaults() {
        assert_eq!(DEFAULT_MIN_GAIN, 2);
        assert_eq!(DEFAULT_MAX_GAIN, 5);
        assert_eq!(DEFAULT_MAX_INCREASE, 25);
        assert_eq!(DEFAULT_CYCLE_SECS, 90);
        assert_eq!(DEFAULT_SUCCESS_RATE, 15);
    }
}

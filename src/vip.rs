//! VIP会员系统
//!
//! 来源: MessageTemplate 中的6个VIP模板
//! 指令: VIP信息, VIP签到, VIP充值

use crate::core::*;
use crate::db::Database;
use crate::template_render;

/// VIP等级配置
struct VipLevel {
    name: &'static str,
    min_points: i32,
    exp_bonus: i32,           // 经验加成百分比
    daily_gift: &'static str, // 每日礼包物品名
    level_gift: &'static str, // 升级礼包物品名
    sign_points: i32,         // 每日签到积分
}

/// VIP等级表 (0=非VIP, 1-5=VIP等级)
const VIP_LEVELS: &[VipLevel] = &[
    VipLevel {
        name: "非VIP",
        min_points: 0,
        exp_bonus: 0,
        daily_gift: "",
        level_gift: "",
        sign_points: 0,
    },
    VipLevel {
        name: "VIP1",
        min_points: 100,
        exp_bonus: 5,
        daily_gift: "生命药水",
        level_gift: "初级礼包",
        sign_points: 10,
    },
    VipLevel {
        name: "VIP2",
        min_points: 300,
        exp_bonus: 10,
        daily_gift: "大生命药水",
        level_gift: "中级礼包",
        sign_points: 15,
    },
    VipLevel {
        name: "VIP3",
        min_points: 800,
        exp_bonus: 15,
        daily_gift: "超生命药水",
        level_gift: "高级礼包",
        sign_points: 20,
    },
    VipLevel {
        name: "VIP4",
        min_points: 2000,
        exp_bonus: 20,
        daily_gift: "北姬的红药水",
        level_gift: "至尊礼包",
        sign_points: 30,
    },
    VipLevel {
        name: "VIP5",
        min_points: 5000,
        exp_bonus: 30,
        daily_gift: "护肝药剂",
        level_gift: "传说礼包",
        sign_points: 50,
    },
];

/// 充值档位
struct RechargeTier {
    name: &'static str,
    cost_gold: i64,
    give_points: i32,
    give_days: i32,
}

const RECHARGE_TIERS: &[RechargeTier] = &[
    RechargeTier {
        name: "月卡",
        cost_gold: 10_000,
        give_points: 100,
        give_days: 30,
    },
    RechargeTier {
        name: "季卡",
        cost_gold: 25_000,
        give_points: 300,
        give_days: 90,
    },
    RechargeTier {
        name: "半年卡",
        cost_gold: 45_000,
        give_points: 500,
        give_days: 180,
    },
    RechargeTier {
        name: "年卡",
        cost_gold: 80_000,
        give_points: 1000,
        give_days: 365,
    },
];

/// 获取VIP数据 (level, points, expiry_timestamp, last_sign_date)
fn get_vip_data(db: &Database, user_id: &str) -> (i32, i32, i64, String) {
    let points: i32 = db.read_user_data(user_id, "vip_points").parse().unwrap_or(0);
    let expiry: i64 = db.read_user_data(user_id, "vip_expiry").parse().unwrap_or(0);
    let sign_date = db.read_user_data(user_id, "vip_sign_date");

    let level = calc_vip_level(points);
    (level, points, expiry, sign_date)
}

/// 根据积分计算VIP等级
fn calc_vip_level(points: i32) -> i32 {
    for i in (1..VIP_LEVELS.len()).rev() {
        if points >= VIP_LEVELS[i].min_points {
            return i as i32;
        }
    }
    0
}

/// 获取当前时间戳 (秒)
fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 获取今天日期字符串 YYYY-MM-DD
fn today_string() -> String {
    let ts = now_timestamp();
    let days = ts / 86400;
    // 简单日期计算 (从1970-01-01开始)
    let mut y = 1970;
    let mut d = days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    let month_days = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0;
    for (i, &days_in_month) in month_days.iter().enumerate() {
        if d < days_in_month {
            m = i;
            break;
        }
        d -= days_in_month;
    }
    format!("{:04}-{:02}-{:02}", y, m + 1, d + 1)
}

/// 格式化到期时间
fn format_expiry(expiry_ts: i64) -> String {
    if expiry_ts <= 0 {
        return "未开通".to_string();
    }
    let now = now_timestamp();
    if expiry_ts < now {
        return "已过期".to_string();
    }
    let remaining = expiry_ts - now;
    let days = remaining / 86400;
    let hours = (remaining % 86400) / 3600;
    format!("{}天{}小时后到期", days, hours)
}

/// 计算VIP经验加成 (供战斗系统调用)
#[allow(dead_code)]
pub fn get_vip_exp_bonus(db: &Database, user_id: &str) -> i32 {
    let (level, _points, expiry, _sign_date) = get_vip_data(db, user_id);
    if level == 0 || expiry < now_timestamp() {
        return 0;
    }
    VIP_LEVELS[level as usize].exp_bonus
}

// ==================== 命令实现 ====================

/// VIP信息 - 查看VIP状态
use crate::template::{render_vip_info, TemplateContext};

pub fn cmd_vip_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看VIP信息！\n发送【注册+昵称】进行注册".to_string();
    }

    let (level, points, expiry, _sign_date) = get_vip_data(db, user_id);
    let now = now_timestamp();

    // 尝试使用模板渲染
    let mut ctx = TemplateContext::new();
    ctx.set("VIP等级", &level.to_string());
    ctx.set("VIP积分", &points.to_string());
    let tmpl_result = render_vip_info(db, &ctx);
    if !tmpl_result.is_empty() {
        return tmpl_result;
    }

    if level == 0 && points == 0 && expiry == 0 {
        return template_render::render_not_vip(db);
    }

    let is_active = expiry > now;
    let effective_level = if is_active { level } else { 0 };

    let level_info = &VIP_LEVELS[effective_level as usize];
    let expiry_str = format_expiry(expiry);

    let gift_str = if level_info.daily_gift.is_empty() {
        "无".to_string()
    } else {
        level_info.daily_gift.to_string()
    };

    let tips = if !is_active && points > 0 {
        "⚠️ VIP已过期，发送【VIP充值】续费可恢复等级权益！"
    } else {
        ""
    };
    let mut result = template_render::render_vip_info_tpl(
        db,
        level_info.name,
        points,
        &gift_str,
        level_info.exp_bonus,
        &expiry_str,
        tips,
    );

    if is_active {
        // 显示签到状态
        let today = today_string();
        let (_, _, _, sign_date) = get_vip_data(db, user_id);
        if sign_date == today {
            result.push_str("\n✅ 今日已签到\n");
        } else {
            result.push_str(&format!(
                "\n📋 今日未签到，发送【VIP签到】可获得{}积分+每日礼包\n",
                level_info.sign_points
            ));
        }
        // 显示下一级信息
        if (effective_level as usize) < VIP_LEVELS.len() - 1 {
            let next = &VIP_LEVELS[effective_level as usize + 1];
            result.push_str(&format!(
                "\n📈 升级到{}还需 {} 积分\n",
                next.name,
                next.min_points - points
            ));
        }
    }

    // 充值选项
    result.push_str("\n━━━━ 充值方案 ━━━━\n");
    for tier in RECHARGE_TIERS {
        result.push_str(&format!(
            "  {}：{}金币 → {}积分 + {}天VIP\n",
            tier.name, tier.cost_gold, tier.give_points, tier.give_days
        ));
    }
    result.push_str("\n发送【VIP充值+月卡/季卡/半年卡/年卡】进行充值\n");

    result
}

/// VIP签到 - 每日签到获得积分和礼包
pub fn cmd_vip_sign_in(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再进行VIP签到！\n发送【注册+昵称】进行注册".to_string();
    }

    let (level, points, expiry, sign_date) = get_vip_data(db, user_id);
    let now = now_timestamp();
    let today = today_string();

    // 检查VIP是否有效
    if level == 0 || expiry < now {
        return template_render::render_not_vip(db);
    }

    // 检查今天是否已签到
    if sign_date == today {
        return format!(
            "您今天已经签到过了~\n当前VIP等级：{} [{}积分]\n明天再来吧！",
            VIP_LEVELS[level as usize].name, points
        );
    }

    // 执行签到
    let level_info = &VIP_LEVELS[level as usize];
    let sign_points = level_info.sign_points;

    // 写入签到日期
    db.write_user_data(user_id, "vip_sign_date", &today);

    // 增加积分
    let new_points = points + sign_points;
    db.write_user_data(user_id, "vip_points", &new_points.to_string());

    // 检查是否升级
    let new_level = calc_vip_level(new_points);
    let mut result = template_render::render_vip_sign_success(
        db,
        sign_points,
        VIP_LEVELS[level as usize].name,
        level_info.daily_gift,
    )
    .to_string();

    // 给每日礼包
    if !level_info.daily_gift.is_empty() {
        db.add_item(user_id, level_info.daily_gift, 1);
        result.push_str(&format!(
            "我们为您特意准备了今天的礼物，还请您笑纳~\n\
             [{}]*1已放入您的背包~\n",
            level_info.daily_gift
        ));
    }

    // 检查升级
    if new_level > level {
        let new_level_info = &VIP_LEVELS[new_level as usize];
        // 给升级礼包
        if !new_level_info.level_gift.is_empty() {
            db.add_item(user_id, new_level_info.level_gift, 1);
        }
        result.push_str(&format!(
            "\n{}",
            template_render::render_vip_level_up(db, new_level_info.name, new_level_info.level_gift)
        ));
    }

    result.push_str(&format!(
        "\n当前积分：{} | 等级：{}",
        new_points, VIP_LEVELS[new_level as usize].name
    ));

    result
}

/// VIP充值 - 使用金币充值VIP
pub fn cmd_vip_recharge(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再进行VIP充值！\n发送【注册+昵称】进行注册".to_string();
    }

    let tier_name = args.trim();
    if tier_name.is_empty() {
        return "请选择充值方案：\n\
               \n\
               💰 月卡：10,000金币 → 100积分 + 30天VIP\n\
               💰 季卡：25,000金币 → 300积分 + 90天VIP\n\
               💰 半年卡：45,000金币 → 500积分 + 180天VIP\n\
               💰 年卡：80,000金币 → 1000积分 + 365天VIP\n\
               \n\
               发送【VIP充值+方案名】进行充值\n\
               例如：VIP充值+月卡"
            .to_string();
    }

    // 匹配充值档位
    let tier = RECHARGE_TIERS.iter().find(|t| t.name == tier_name);
    let tier = match tier {
        Some(t) => t,
        None => {
            return format!("未找到充值方案「{}」\n可选：月卡/季卡/半年卡/年卡", tier_name);
        }
    };

    // 检查金币
    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < tier.cost_gold {
        return template_render::render_vip_recharge_fail(db, gold);
    }

    // 扣除金币
    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, tier.cost_gold);

    // 增加积分
    let (_, old_points, expiry, _) = get_vip_data(db, user_id);
    let new_points = old_points + tier.give_points;
    db.write_user_data(user_id, "vip_points", &new_points.to_string());

    // 计算到期时间（叠加或新开）
    let now = now_timestamp();
    let base = if expiry > now { expiry } else { now };
    let new_expiry = base + (tier.give_days as i64) * 86400;
    db.write_user_data(user_id, "vip_expiry", &new_expiry.to_string());

    // 检查升级
    let old_level = calc_vip_level(old_points);
    let new_level = calc_vip_level(new_points);

    let new_gold = db.read_currency(user_id, CURRENCY_GOLD);
    let expiry_str = format_expiry(new_expiry);

    let mut result = template_render::render_vip_recharge_success(db, new_gold, &expiry_str);

    // 升级提示
    if new_level > old_level {
        let new_level_info = &VIP_LEVELS[new_level as usize];
        if !new_level_info.level_gift.is_empty() {
            db.add_item(user_id, new_level_info.level_gift, 1);
        }
        result.push_str(&format!(
            "\n\n{}",
            template_render::render_vip_level_up(db, new_level_info.name, new_level_info.level_gift)
        ));
    }

    // 首冲奖励
    if let Some(first_reward) = check_and_award_first_recharge(db, user_id) {
        result.push_str(&format!("\n🎁 {}\n", first_reward));
    }

    // 累计充值里程碑检查
    if let Some(milestone_msg) = on_vip_recharge(db, user_id, tier.cost_gold) {
        result.push_str(&milestone_msg);
    }

    result
}

/// 获取用户VIP等级 (供其他模块调用)
pub fn get_vip_level(db: &Database, user_id: &str) -> i32 {
    let (level, _points, _expiry, _sign_date) = get_vip_data(db, user_id);
    level
}

// ==================== 首冲奖励系统 ====================

/// 首冲奖励配置
struct FirstRechargeReward {
    gold: i32,
    diamonds: i32,
    item_name: &'static str,
    item_count: i32,
}

const FIRST_RECHARGE_REWARD: FirstRechargeReward = FirstRechargeReward {
    gold: 10000,
    diamonds: 500,
    item_name: "药剂礼包",
    item_count: 1,
};

/// 检查并发放首冲奖励（在 VIP 充值成功后调用）
/// 返回 Some(奖励描述) 如果是首次充值，None 如果已领过
pub fn check_and_award_first_recharge(db: &Database, user_id: &str) -> Option<String> {
    let already_done = db.read_user_data(user_id, "first_recharge_done");
    if already_done == "1" {
        return None;
    }

    // 标记已领取
    db.write_user_data(user_id, "first_recharge_done", "1");

    // 发放奖励
    let r = &FIRST_RECHARGE_REWARD;
    if r.gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, r.gold as i64);
    }
    if r.diamonds > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, r.diamonds as i64);
    }
    if !r.item_name.is_empty() {
        db.add_item(user_id, r.item_name, r.item_count);
    }

    Some(format!(
        "首冲奖励：{}金币 + {}钻石 + [{}]*{}已放入您的背包！~",
        r.gold, r.diamonds, r.item_name, r.item_count
    ))
}

/// 查看首冲奖励 - 显示首冲奖励状态和内容
pub fn cmd_first_recharge(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看首冲奖励！\n发送【注册+昵称】进行注册".to_string();
    }

    let already_done = db.read_user_data(user_id, "first_recharge_done");
    let r = &FIRST_RECHARGE_REWARD;

    let mut result = String::from("🎁 === 首冲奖励 ===\n\n");
    result.push_str("首次VIP充值即可领取超值大礼包！\n\n");
    result.push_str("📦 奖励内容：\n");
    result.push_str(&format!("  💰 {}金币\n", r.gold));
    result.push_str(&format!("  💎 {}钻石\n", r.diamonds));
    result.push_str(&format!("  🎁 [{}]*{}\n\n", r.item_name, r.item_count));

    if already_done == "1" {
        result.push_str("✅ 您已领取首冲奖励！\n");
    } else {
        result.push_str("⏳ 尚未领取 — 发送【VIP充值+月卡】进行首次充值即可获得！\n");
    }

    result
}

// ==================== 累计充值奖励系统 ====================

/// 累计充值里程碑配置
struct CumulativeRechargeMilestone {
    threshold: i64,            // 累计充值金币数
    reward_gold: i64,          // 奖励金币
    reward_diamond: i32,       // 奖励钻石
    reward_item: &'static str, // 奖励物品
    reward_qty: i32,           // 物品数量
    title: &'static str,       // 称号
}

const CUMULATIVE_MILESTONES: &[CumulativeRechargeMilestone] = &[
    CumulativeRechargeMilestone {
        threshold: 10_000,
        reward_gold: 2_000,
        reward_diamond: 50,
        reward_item: "生命药水",
        reward_qty: 5,
        title: "初露锋芒",
    },
    CumulativeRechargeMilestone {
        threshold: 50_000,
        reward_gold: 10_000,
        reward_diamond: 200,
        reward_item: "大生命药水",
        reward_qty: 3,
        title: "一掷千金",
    },
    CumulativeRechargeMilestone {
        threshold: 100_000,
        reward_gold: 25_000,
        reward_diamond: 500,
        reward_item: "超生命药水",
        reward_qty: 3,
        title: "挥金如土",
    },
    CumulativeRechargeMilestone {
        threshold: 300_000,
        reward_gold: 80_000,
        reward_diamond: 1_000,
        reward_item: "护肝药剂",
        reward_qty: 2,
        title: "富甲一方",
    },
    CumulativeRechargeMilestone {
        threshold: 500_000,
        reward_gold: 150_000,
        reward_diamond: 2_000,
        reward_item: "药剂礼包",
        reward_qty: 2,
        title: "财源广进",
    },
    CumulativeRechargeMilestone {
        threshold: 1_000_000,
        reward_gold: 300_000,
        reward_diamond: 5_000,
        reward_item: "远古超界石",
        reward_qty: 1,
        title: "至尊豪客",
    },
];

/// 在VIP充值成功后调用，累加充值总额并检查里程碑
pub fn on_vip_recharge(db: &Database, user_id: &str, gold_spent: i64) -> Option<String> {
    // 读取当前累计充值
    let current_total: i64 = db.read_user_data(user_id, "cumulative_recharge").parse().unwrap_or(0);
    let new_total = current_total + gold_spent;
    db.write_user_data(user_id, "cumulative_recharge", &new_total.to_string());

    // 检查是否有新的里程碑达成
    // 读取已领取的里程碑索引
    let claimed_str = db.read_user_data(user_id, "cumulative_claimed");
    let claimed_idx: i32 = claimed_str.parse().unwrap_or(-1);

    let mut result = String::new();
    for (i, milestone) in CUMULATIVE_MILESTONES.iter().enumerate() {
        let i = i as i32;
        if new_total >= milestone.threshold && i > claimed_idx {
            // 新达成的里程碑
            db.write_user_data(user_id, "cumulative_claimed", &i.to_string());

            // 发放奖励
            if milestone.reward_gold > 0 {
                db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, milestone.reward_gold);
            }
            if milestone.reward_diamond > 0 {
                db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, milestone.reward_diamond as i64);
            }
            if !milestone.reward_item.is_empty() {
                db.add_item(user_id, milestone.reward_item, milestone.reward_qty);
            }

            result.push_str(&format!(
                "\n🎊 累计充值达成：超过{}金币！\n\
                 称号：「{}」\n\
                 奖励：{}金币 + {}钻石 + [{}]*{}\n\
                 已放入您的背包~！\n",
                format_gold(milestone.threshold),
                milestone.title,
                format_gold(milestone.reward_gold),
                milestone.reward_diamond,
                milestone.reward_item,
                milestone.reward_qty
            ));
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// 格式化金币数（带千位分隔符）
pub fn format_gold(amount: i64) -> String {
    let s = amount.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// 查看累计充值 - 显示累计充值进度和里程碑
pub fn cmd_cumulative_recharge(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    if !db.user_exists(user_id) {
        return "请先注册后再查看累计充值！\n发送【注册+昵称】进行注册".to_string();
    }

    let cumulative: i64 = db.read_user_data(user_id, "cumulative_recharge").parse().unwrap_or(0);
    let claimed_idx: i32 = db.read_user_data(user_id, "cumulative_claimed").parse().unwrap_or(-1);

    let mut result = String::from("💎 === 累计充值奖励 ===\n\n");
    result.push_str(&format!("📊 累计充值：{}金币\n\n", format_gold(cumulative)));

    result.push_str("🏅 里程碑奖励：\n");
    for (i, milestone) in CUMULATIVE_MILESTONES.iter().enumerate() {
        let i = i as i32;
        let status = if i <= claimed_idx {
            "✅ 已领取"
        } else if cumulative >= milestone.threshold {
            "🔔 可领取（下次充值自动发放）"
        } else {
            "⏳ 未达成"
        };

        let progress = if cumulative >= milestone.threshold {
            "100%".to_string()
        } else {
            format!("{}%", (cumulative * 100 / milestone.threshold).min(99))
        };

        result.push_str(&format!(
            "  {} {} — 累计{}金币\n\
             \t称号：「{}」| 奖励：{}金+{}钻+[{}]*{}\n\
             \t进度：{} {}\n",
            if i <= claimed_idx {
                "✅"
            } else if cumulative >= milestone.threshold {
                "🔔"
            } else {
                "⬜"
            },
            format_gold(milestone.threshold),
            format_gold(milestone.threshold),
            milestone.title,
            format_gold(milestone.reward_gold),
            milestone.reward_diamond,
            milestone.reward_item,
            milestone.reward_qty,
            progress,
            status
        ));
    }

    // 显示下一个里程碑
    let next = CUMULATIVE_MILESTONES.iter().find(|m| cumulative < m.threshold);
    if let Some(next_milestone) = next {
        let remaining = next_milestone.threshold - cumulative;
        result.push_str(&format!(
            "\n💡 再充值{}金币即可达成「{}」里程碑~\n",
            format_gold(remaining),
            next_milestone.title
        ));
    } else {
        result.push_str("\n🎉 恭喜！您已达成所有累计充值里程碑！\n");
    }

    result.push_str("\nTip: 每次VIP充值自动累计，里程碑奖励自动发放~");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_gold_basic() {
        assert_eq!(format_gold(0), "0");
        assert_eq!(format_gold(123), "123");
        assert_eq!(format_gold(1234), "1,234");
        assert_eq!(format_gold(1234567), "1,234,567");
        assert_eq!(format_gold(1000000), "1,000,000");
    }

    #[test]
    fn test_format_gold_negative() {
        // 负数也应该格式化
        assert_eq!(format_gold(-1234), "-1,234");
    }

    #[test]
    fn test_vip_levels_count() {
        assert_eq!(VIP_LEVELS.len(), 6); // 非VIP + VIP1-5
    }

    #[test]
    fn test_vip_levels_monotonic() {
        // min_points 必须单调递增
        for i in 1..VIP_LEVELS.len() {
            assert!(
                VIP_LEVELS[i].min_points > VIP_LEVELS[i - 1].min_points,
                "VIP等级{}的min_points({})应大于等级{}({})",
                i,
                VIP_LEVELS[i].min_points,
                i - 1,
                VIP_LEVELS[i - 1].min_points
            );
        }
    }

    #[test]
    fn test_vip_exp_bonus_monotonic() {
        // 经验加成应单调递增
        for i in 1..VIP_LEVELS.len() {
            assert!(
                VIP_LEVELS[i].exp_bonus >= VIP_LEVELS[i - 1].exp_bonus,
                "VIP等级{}的exp_bonus({})应大于等于等级{}({})",
                i,
                VIP_LEVELS[i].exp_bonus,
                i - 1,
                VIP_LEVELS[i - 1].exp_bonus
            );
        }
    }

    #[test]
    fn test_vip_level0_is_non_vip() {
        assert_eq!(VIP_LEVELS[0].name, "非VIP");
        assert_eq!(VIP_LEVELS[0].min_points, 0);
        assert_eq!(VIP_LEVELS[0].exp_bonus, 0);
    }

    #[test]
    fn test_calc_vip_level() {
        assert_eq!(calc_vip_level(0), 0);
        assert_eq!(calc_vip_level(50), 0);
        assert_eq!(calc_vip_level(100), 1);
        assert_eq!(calc_vip_level(200), 1);
        assert_eq!(calc_vip_level(300), 2);
        assert_eq!(calc_vip_level(800), 3);
        assert_eq!(calc_vip_level(2000), 4);
        assert_eq!(calc_vip_level(5000), 5);
        assert_eq!(calc_vip_level(99999), 5);
    }

    #[test]
    fn test_recharge_tiers_monotonic() {
        // 充值档位应按价格递增
        for i in 1..RECHARGE_TIERS.len() {
            assert!(
                RECHARGE_TIERS[i].cost_gold > RECHARGE_TIERS[i - 1].cost_gold,
                "充值档{}的价格应大于档{}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_cumulative_milestones_monotonic() {
        // 里程碑阈值应单调递增
        for i in 1..CUMULATIVE_MILESTONES.len() {
            assert!(
                CUMULATIVE_MILESTONES[i].threshold > CUMULATIVE_MILESTONES[i - 1].threshold,
                "里程碑{}的阈值应大于里程碑{}",
                i,
                i - 1
            );
        }
    }

    #[test]
    fn test_cumulative_milestones_rewards_positive() {
        for (i, m) in CUMULATIVE_MILESTONES.iter().enumerate() {
            assert!(
                m.reward_gold > 0 || m.reward_diamond > 0,
                "里程碑{}应有金币或钻石奖励",
                i
            );
            assert!(!m.title.is_empty(), "里程碑{}应有称号", i);
        }
    }

    #[test]
    fn test_vip_sign_points_monotonic() {
        // 签到积分应递增
        for i in 1..VIP_LEVELS.len() {
            assert!(
                VIP_LEVELS[i].sign_points >= VIP_LEVELS[i - 1].sign_points,
                "VIP等级{}的签到积分应大于等于等级{}",
                i,
                i - 1
            );
        }
    }
}

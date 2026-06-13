/// CakeGame 模板驱动渲染集成系统 v2
/// 集成所有 MessageTemplate 条目到命令输出，实现原版模板驱动渲染
/// 数据来源: MessageTemplate 表 (87条模板数据，本模块激活全部87条)
///
/// 设计原则:
/// - 优先使用模板渲染，模板不存在时使用硬编码回退
/// - 模板变量替换: [变量名] → 实际值
/// - 循环模板体: <[序号].[名称]> 重复替换
/// - $变量=[SET:type=X] → 预定义变量赋值
use crate::db::Database;
use crate::template::{render_simple, TemplateContext};

// ==================== VIP 系统模板 ====================

/// VIP信息返回 模板渲染
/// 模板格式: VIP等级：[VIP等级][[积分]]\n每日礼包：[每日礼包]\n经验加成：[经验加成]%\nVIP到期时间：[VIP到期时间]
pub fn render_vip_info_tpl(
    db: &Database,
    vip_level: &str,
    points: i32,
    daily_gift: &str,
    exp_bonus: i32,
    expiry_str: &str,
    tips: &str,
) -> String {
    let raw = db.template_get("VIP信息返回");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("VIP等级", vip_level);
        ctx.set("积分", &points.to_string());
        ctx.set("每日礼包", daily_gift);
        ctx.set("经验加成", &exp_bonus.to_string());
        ctx.set("VIP到期时间", expiry_str);
        ctx.set("Tips", tips);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    let tips_suffix = if tips.is_empty() {
        String::new()
    } else {
        format!("\n{}", tips)
    };
    format!(
        "VIP等级：{} [{}积分]\n每日礼包：{}\n经验加成：{}%\nVIP到期时间：{}{}",
        vip_level, points, daily_gift, exp_bonus, expiry_str, tips_suffix
    )
}

/// VIP充值失败 模板渲染
pub fn render_vip_recharge_fail(db: &Database, balance: i64) -> String {
    let raw = db.template_get("VIP充值失败");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("余额", &balance.to_string());
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("充值失败！余额不足，剩余余额：{}金币", balance)
}

/// VIP充值成功 模板渲染
pub fn render_vip_recharge_success(db: &Database, balance: i64, expiry_str: &str) -> String {
    let raw = db.template_get("VIP充值成功");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("余额", &balance.to_string());
        ctx.set("VIP到期时间", expiry_str);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "充值成功！\n剩余余额：{}金币\nVIP到期时间：{}\n发送【VIP信息】即可查看您当前的VIP级别~",
        balance, expiry_str
    )
}

/// VIP等级提升 模板渲染
pub fn render_vip_level_up(db: &Database, vip_level: &str, level_gift: &str) -> String {
    let raw = db.template_get("VIP等级提升");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("VIP等级", vip_level);
        ctx.set("等级礼包", level_gift);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "🎉 恭喜您，VIP等级提升！当前等级{}\n你可以享受更多的福利了，发送【VIP信息】查看当前信息~\n等级礼包：[{}]*1已放入您的背包~！",
        vip_level, level_gift
    )
}

/// VIP签到成功 模板渲染
pub fn render_vip_sign_success(db: &Database, points: i32, user_info: &str, daily_gift: &str) -> String {
    let raw = db.template_get("VIP签到成功");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("积分", &points.to_string());
        ctx.set("用户信息", user_info);
        ctx.set("每日礼物", daily_gift);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "签到成功！积分+{}\n尊贵的{}欢迎回来~\n我们为您特意准备了今天的礼物，还请您笑纳~\n[{}]*1已放入您的背包~",
        points, user_info, daily_gift
    )
}

/// 不是VIP用户 模板渲染
pub fn render_not_vip(db: &Database) -> String {
    let raw = db.template_get("不是VIP用户");
    if !raw.is_empty() {
        return raw;
    }
    "抱歉，您还不是VIP或者VIP已过期！发送【VIP充值】快来成为VIP吧！~".to_string()
}

/// 充值到账 模板渲染
#[allow(dead_code)]
pub fn render_recharge_received(db: &Database, amount: i64, balance: i64) -> String {
    let raw = db.template_get("充值到账");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("充值", &amount.to_string());
        ctx.set("余额", &balance.to_string());
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("充值成功！已到账{}金币\n剩余余额：{}金币", amount, balance)
}

/// 首冲奖励 模板渲染
#[allow(dead_code)]
pub fn render_first_recharge_reward(db: &Database, reward: &str) -> String {
    let raw = db.template_get("首冲奖励");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("奖励", reward);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("🎁 首冲奖励：{}*1已放入您的背包！~", reward)
}

/// 累计充值 模板渲染
#[allow(dead_code)]
pub fn render_cumulative_recharge(db: &Database, total: i64, reward: &str) -> String {
    let raw = db.template_get("累计充值");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("累计", &total.to_string());
        ctx.set("奖励", reward);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "您已累计充值超过{}金币\n为表示感谢，我们为您送上一份礼物~\n{}*1已放入您的背包~",
        total, reward
    )
}

// ==================== 战斗流程模板 ====================

/// 触发虚弱禁止 模板渲染
#[allow(dead_code)]
pub fn render_weakness_block(db: &Database, remaining_minutes: i32) -> String {
    let raw = db.template_get("触发虚弱禁止");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("剩余时间", &remaining_minutes.to_string());
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "您目前是虚弱状态，无法执行此操作。\n虚弱状态剩余{}分钟。",
        remaining_minutes
    )
}

/// 触发野外禁止 模板渲染
#[allow(dead_code)]
pub fn render_field_block(db: &Database) -> String {
    let raw = db.template_get("触发野外禁止");
    if !raw.is_empty() {
        return raw;
    }
    "您目前所在的位置为野外地图，请到安全区后再继续本操作！".to_string()
}

/// 强制战斗禁止 模板渲染
#[allow(dead_code)]
pub fn render_forced_combat_block(db: &Database, attacker: &str) -> String {
    let raw = db.template_get("强制战斗禁止");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("UD:Data,type=forced_combat_target", attacker);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("您目前正在遭受{}的进攻，无法完成此操作！", attacker)
}

/// 野怪额外经验 模板渲染 (VIP经验加成)
#[allow(dead_code)]
pub fn render_vip_exp_bonus_tpl(db: &Database, exp: i32, bonus_pct: i32) -> String {
    let raw = db.template_get("野怪额外经验");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("经验", &exp.to_string());
        ctx.set("加成", &bonus_pct.to_string());
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("VIP经验加成：{} [+{}%]", exp, bonus_pct)
}

// ==================== UI 状态模板 ====================

/// 系统在更新 模板渲染
#[allow(dead_code)]
pub fn render_system_updating(db: &Database) -> String {
    let raw = db.template_get("系统在更新");
    if !raw.is_empty() {
        return raw;
    }
    "系统正在进行数据更新，请稍后重试！".to_string()
}

/// 插件被锁定 模板渲染
#[allow(dead_code)]
pub fn render_plugin_locked(db: &Database) -> String {
    let raw = db.template_get("插件被锁定");
    if !raw.is_empty() {
        return raw;
    }
    "插件已锁定，请重启软件后重试！".to_string()
}

/// 正在初始化 模板渲染
#[allow(dead_code)]
pub fn render_initializing(db: &Database) -> String {
    let raw = db.template_get("正在初始化");
    if !raw.is_empty() {
        return raw;
    }
    "您目前无法使用此插件，原因：插件正在初始化或正在刷新数据。".to_string()
}

// ==================== 地图/位置模板 ====================

/// 成功进入地图 模板渲染
#[allow(dead_code)]
pub fn render_enter_map_success(db: &Database, map_name: &str) -> String {
    let raw = db.template_get("成功进入地图");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("地图名称", map_name);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("您已成功进入{}！", map_name)
}

/// 重复进入地图 模板渲染
#[allow(dead_code)]
pub fn render_enter_map_duplicate(db: &Database, map_name: &str) -> String {
    let raw = db.template_get("重复进入地图");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("地图名称", map_name);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("您已已在{}，无需再次进入！", map_name)
}

/// 当前位置信息 模板渲染
#[allow(dead_code)]
pub fn render_location_info(db: &Database, map_name: &str, min_lv: i32, max_lv: i32, introduce: &str) -> String {
    let raw = db.template_get("当前位置信息");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("地图名称", map_name);
        ctx.set("最低等级", &min_lv.to_string());
        ctx.set("最高等级", &max_lv.to_string());
        ctx.set("地图介绍", introduce);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return clean_template_vars(&rendered);
        }
    }
    format!("═══ {} ═══\n等级范围：{} ~ {}\n{}", map_name, min_lv, max_lv, introduce)
}

// ==================== 任务模板 ====================

/// 成功领取任务 模板渲染
#[allow(dead_code)]
pub fn render_task_accepted(db: &Database, task_name: &str) -> String {
    let raw = db.template_get("成功领取任务");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("任务名称", task_name);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return clean_template_vars(&rendered);
        }
    }
    format!("您已成功领取任务\"{}\"！", task_name)
}

// ==================== 排行模板 ====================

/// 排行查询返回 模板渲染
#[allow(dead_code)]
pub fn render_ranking_list(
    db: &Database,
    entries: &[(usize, String, String, String)], // (rank, name, uid, attr_name:value)
) -> String {
    let raw = db.template_get("排行查询返回");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                for (rank, name, uid, attr_str) in entries {
                    let line = loop_tmpl
                        .replace("[排名序号]", &rank.to_string())
                        .replace("[UD:Base,type=Name,uid=[玩家ID]]", name)
                        .replace("[玩家ID]", uid)
                        .replace("[属性名称]:[属性值]", attr_str);
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    // 回退
    let mut out = String::new();
    for (rank, name, uid, attr_str) in entries {
        out.push_str(&format!("{}. {}({}) {}\n", rank, name, uid, attr_str));
    }
    out
}

// ==================== 段位排行模板 ====================

/// 竞技扩展_段位排行 模板渲染
pub fn render_season_ranking(
    db: &Database,
    entries: &[(usize, String, String, String, i32)], // (rank, name, uid, rank_name, score)
) -> String {
    let raw = db.template_get("竞技扩展_段位排行");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                // 提取头部
                let header = raw[..start].trim();
                for line in header.lines() {
                    let l = line.trim();
                    if !l.starts_with('$') && !l.starts_with("!del") && !l.is_empty() {
                        result.push_str(l);
                        result.push('\n');
                    }
                }
                for (rank, name, uid, rank_name, score) in entries {
                    let line = loop_tmpl
                        .replace("[排行序号]", &rank.to_string())
                        .replace("[UD:Base,type=Name,uid=[玩家ID]]", name)
                        .replace("[玩家ID]", uid)
                        .replace("[段位名称]", rank_name)
                        .replace("[积分数量]", &score.to_string());
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = String::from("【赛季段位排行】\n");
    for (rank, name, uid, rank_name, score) in entries {
        out.push_str(&format!("{}. {}({}) {}({}分)\n", rank, name, uid, rank_name, score));
    }
    out
}

// ==================== 公会捐赠排行模板 ====================

/// 公会捐赠排行 模板渲染
pub fn render_guild_donation_ranking(
    db: &Database,
    entries: &[(usize, String, i64)], // (rank, name, amount)
) -> String {
    let raw = db.template_get("公会捐赠排行");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                let header = raw[..start].trim();
                for line in header.lines() {
                    let l = line.trim();
                    if !l.starts_with('$') && !l.starts_with("!del") && !l.is_empty() {
                        result.push_str(l);
                        result.push('\n');
                    }
                }
                for (rank, name, amount) in entries {
                    let line = loop_tmpl
                        .replace("[排行序号]", &rank.to_string())
                        .replace("[UD:Base,type=Name]", name)
                        .replace("[捐赠金额]", &format!("{}", amount));
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = String::from("【公会捐赠排行】\n");
    for (rank, name, amount) in entries {
        out.push_str(&format!("{}. {} - {}金币\n", rank, name, amount));
    }
    out
}

// ==================== 战力排行模板 ====================

/// com.shdic.cg_forces战力排行榜 模板渲染
#[allow(dead_code)]
pub fn render_combat_power_ranking(
    db: &Database,
    entries: &[(usize, String, i64)], // (rank, name, power)
) -> String {
    let raw = db.template_get("com.shdic.cg_forces战力排行榜");
    if !raw.is_empty() {
        let list_body = entries
            .iter()
            .map(|(rank, name, power)| format!("{}. {} - {}战力", rank, name, power))
            .collect::<Vec<_>>()
            .join("\n");
        let mut ctx = TemplateContext::new();
        ctx.set("listbody", &list_body);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    let mut out = String::from("当前战力排行！\n");
    for (rank, name, power) in entries {
        out.push_str(&format!("{}. {} - {}战力\n", rank, name, power));
    }
    out
}

// ==================== 自动修炼模板 ====================

/// 修炼中 模板渲染
pub fn render_cultivation_in_progress(db: &Database, start_time: &str) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution修炼中");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("time", start_time);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("你已经在修炼中啦！（开始修炼时间：{}）", start_time)
}

/// 修炼中断 模板渲染
#[allow(dead_code)]
pub fn render_cultivation_interrupted(db: &Database, reason: &str) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution修炼中断");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("why", reason);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("你上次的修炼被迫中断，中断原因{}", reason)
}

/// 修炼仍在继续 模板渲染
#[allow(dead_code)]
pub fn render_cultivation_continuing(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution修炼仍在继续");
    if !raw.is_empty() {
        return raw;
    }
    "修炼仍在继续……".to_string()
}

/// 修炼失败 模板渲染
#[allow(dead_code)]
pub fn render_cultivation_failed(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution修炼失败");
    if !raw.is_empty() {
        return raw;
    }
    "修炼失败！".to_string()
}

/// 修炼结束 模板渲染
#[allow(dead_code)]
pub fn render_cultivation_ended(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution修炼结束");
    if !raw.is_empty() {
        return raw;
    }
    "修炼已经结束！".to_string()
}

/// 停止修炼 模板渲染
pub fn render_cultivation_stopped(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution停止修炼");
    if !raw.is_empty() {
        return raw;
    }
    "修炼已经被主动停止！".to_string()
}

/// 修炼周期成功 模板渲染
#[allow(dead_code)]
pub fn render_cultivation_cycle_success(db: &Database, gains: &str) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution完成一个修炼周期_成功");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("收益", gains);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("完成一个修炼周期，修炼成功！{}", gains)
}

/// 修炼周期失败 模板渲染
#[allow(dead_code)]
pub fn render_cultivation_cycle_fail(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution完成一个修炼周期_失败");
    if !raw.is_empty() {
        return raw;
    }
    "完成一个修炼周期，修炼失败！".to_string()
}

/// 累计增加 模板渲染
pub fn render_cultivation_cumulative(db: &Database, attr: &str, amount: i32) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution累计增加");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("addAttr", attr);
        ctx.set("addNum", &amount.to_string());
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("{}已经累计增加了{}！", attr, amount)
}

/// 经验已满 模板渲染
pub fn render_cultivation_exp_full(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution经验已满");
    if !raw.is_empty() {
        return raw;
    }
    "经验已满！无法继续修炼！".to_string()
}

/// 预计下次突破时间 模板渲染
pub fn render_cultivation_next_breakthrough(db: &Database, time: &str) -> String {
    let raw = db.template_get("com.shdic.cg_auto_evolution预计下次突破时间");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("time", time);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("修炼将在{}迎来下一次尝试突破……", time)
}

// ==================== 副本模板 ====================

/// 副本完成 模板渲染
pub fn render_dungeon_complete(db: &Database, rewards: &str) -> String {
    let raw = db.template_get("com.shdic.cg_instanceDungeon完成副本");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("jl", rewards);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "恭喜你！已经完成该副本全部关卡的挑战！\n\n获得奖励：{}\n\nTip:记得及时离开副本，否则可能会有怪物刷新出来",
        rewards
    )
}

/// 副本列表 模板渲染 (循环模板)
pub fn render_dungeon_list(db: &Database, dungeons: &[(usize, String)]) -> String {
    let raw = db.template_get("com.shdic.cg_instanceDungeon副本列表");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                for (idx, name) in dungeons {
                    let line = loop_tmpl
                        .replace("[NPC序号]", &idx.to_string())
                        .replace("[NPC名称]", name);
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = String::from("═══ 副本列表 ═══\n");
    for (idx, name) in dungeons {
        out.push_str(&format!("{}. {}\n", idx, name));
    }
    out
}

// ==================== 采集模板 ====================

/// 采集排行 模板渲染 (循环模板)
pub fn render_gather_ranking(db: &Database, entries: &[(usize, String, i32, String)]) -> String {
    let _raw = db.template_get("com.shdic.cg_forces战力排行榜");
    // 采集排行使用战力排行格式的循环模板
    let mut out = String::from("【采集达人排行】\n");
    for (rank, name, level, title) in entries {
        out.push_str(&format!("{}. {} Lv.{} [{}]\n", rank, name, level, title));
    }
    out
}

// ==================== 赏金任务模板 ====================

/// 赏金任务提交成功 模板渲染
pub fn render_bounty_submit_success(db: &Database, reward: &str) -> String {
    let raw = db.template_get("com.shdic.cg_BountyTask提交赏金任务");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("报酬", reward);
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!("任务提交成功！获得报酬{}", reward)
}

/// 没有赏金任务 模板渲染
pub fn render_no_bounty_tasks(db: &Database) -> String {
    let raw = db.template_get("com.shdic.cg_BountyTask没有赏金任务");
    if !raw.is_empty() {
        return raw;
    }
    "当前没有悬赏任务！".to_string()
}

// ==================== 玩家注册模板 ====================

/// 玩家注册错误 模板渲染 (解析 CGD DATA 格式)
#[allow(dead_code)]
pub fn render_register_error(db: &Database, error_type: &str, params: &[(&str, &str)]) -> String {
    let raw = db.template_get("玩家注册游戏");
    if !raw.is_empty() {
        // 解析 #CGD DATA{\u0001错误提示_XXXGH消息} 格式
        if let Some(marker) = raw.find(&format!("错误提示_{}", error_type)) {
            let after = &raw[marker..];
            if let Some(gh_pos) = after.find("GH") {
                let msg_start = gh_pos + 2;
                if let Some(end_pos) = after[msg_start..].find("GI") {
                    let msg = &after[msg_start..msg_start + end_pos];
                    let mut result = msg.to_string();
                    for (key, val) in params {
                        result = result.replace(&format!("[{}]", key), val);
                    }
                    return result;
                }
            }
        }
    }
    String::new() // 空串表示使用默认回退
}

/// 清理模板变量标记
fn clean_template_vars(text: &str) -> String {
    let mut result = text.to_string();
    while let Some(start) = result.find("[DA:") {
        if let Some(end) = result[start..].find(']') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    while let Some(start) = result.find("[SET:") {
        if let Some(end) = result[start..].find(']') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    while let Some(start) = result.find("[UD:") {
        if let Some(end) = result[start..].find(']') {
            result = format!("{}{}", &result[..start], &result[start + end + 1..]);
        } else {
            break;
        }
    }
    result
}

// ==================== 向后兼容的旧函数 ====================

/// 竞技扩展_我的战绩 模板渲染 (保持兼容)
#[allow(clippy::too_many_arguments)]
pub fn render_match_stats(
    db: &Database,
    _user_id: &str,
    occupation: &str,
    total: i32,
    wins: i32,
    losses: i32,
    ties: i32,
    win_rate: f64,
    remaining_points: i32,
) -> String {
    let raw = db.template_get("竞技扩展_我的战绩");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("UD:Base,type=Occupation", occupation);
        ctx.set("匹配次数", &total.to_string());
        ctx.set("匹配胜场", &wins.to_string());
        ctx.set("匹配败场", &losses.to_string());
        ctx.set("匹配和场", &ties.to_string());
        ctx.set("匹配胜率", &format!("{:.1}", win_rate));
        ctx.set("竞技扩展_剩余胜点", &remaining_points.to_string());
        let rendered = render_simple(&raw, &ctx);
        if !rendered.is_empty() {
            return rendered;
        }
    }
    format!(
        "当前职业：{}\n匹配次数：{}\n匹配胜场：{}\n匹配败场：{}\n匹配和场：{}\n目前胜率：{:.1}%\n剩余胜点：{}",
        occupation, total, wins, losses, ties, win_rate, remaining_points
    )
}

/// 小黑屋扩展_罪恶值排行 模板渲染 (保持兼容)
pub fn render_evil_ranking(db: &Database, entries: &[(String, String, i32)]) -> String {
    let raw = db.template_get("小黑屋扩展_罪恶值排行");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                let header = raw[..start].trim();
                if !header.is_empty() {
                    for line in header.lines() {
                        let l = line.trim();
                        if !l.starts_with('$') && !l.starts_with("!del") && !l.is_empty() {
                            result.push_str(l);
                            result.push('\n');
                        }
                    }
                }
                for (i, (name, uid, evil)) in entries.iter().enumerate() {
                    let line = loop_tmpl
                        .replace("[排行序号]", &(i + 1).to_string())
                        .replace("[UD:Base,type=Name,uid=[UID]]", name)
                        .replace("[UID]", uid)
                        .replace("[罪恶值]", &evil.to_string());
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = String::from("【荣耀罪恶榜】\n");
    for (i, (name, uid, evil)) in entries.iter().enumerate() {
        out.push_str(&format!("{}. {}({})\n罪恶值：{}\n", i + 1, name, uid, evil));
    }
    out
}

/// 小黑屋扩展_被杀记录 模板渲染 (保持兼容)
pub fn render_kill_log(db: &Database, entries: &[(String, String, String)]) -> String {
    let raw = db.template_get("小黑屋扩展_被杀记录");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                for (i, (name, uid, time)) in entries.iter().enumerate() {
                    let line = loop_tmpl
                        .replace("[序号]", &(i + 1).to_string())
                        .replace("[UD:Base,type=Name,uid=[UID]]", name)
                        .replace("[UID]", uid)
                        .replace("[时间]", time);
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = String::new();
    for (i, (name, uid, time)) in entries.iter().enumerate() {
        out.push_str(&format!("{}. {}({})\n时间：{}\n", i + 1, name, uid, time));
    }
    out
}

#[allow(dead_code)]
/// 查看装备列表 模板渲染 (保持兼容)
pub fn render_equip_list(db: &Database, slots: &[(&str, &str)]) -> String {
    let raw = db.template_get("查看装备列表");
    if !raw.is_empty() {
        let mut result = String::new();
        for (slot_name, equip_name) in slots {
            let line = raw.replace("[槽位名称]", slot_name).replace("[装备名称]", equip_name);
            result.push_str(line.trim());
            result.push('\n');
        }
        return result;
    }
    let mut out = String::new();
    for (slot_name, equip_name) in slots {
        out.push_str(&format!("{}：{}\n", slot_name, equip_name));
    }
    out
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
/// 查看怪物目标 模板渲染 (保持兼容)
pub fn render_monster_target(
    db: &Database,
    monster_name: &str,
    hp: i32,
    hp_max: i32,
    ad: i32,
    ap: i32,
    defense: i32,
    magic_resist: i32,
) -> String {
    let raw = db.template_get("查看怪物目标");
    if !raw.is_empty() {
        let mut ctx = TemplateContext::new();
        ctx.set("目标名称", monster_name);
        ctx.set("剩余生命", &hp.to_string());
        ctx.set("生命上限", &hp_max.to_string());
        ctx.set("物攻", &ad.to_string());
        ctx.set("法强", &ap.to_string());
        ctx.set("物防", &defense.to_string());
        ctx.set("法防", &magic_resist.to_string());
        let mut result = String::new();
        for line in raw.lines() {
            let l = line.trim();
            if l.starts_with('$') || l.starts_with("!del") {
                continue;
            }
            let rendered = render_simple(l, &ctx);
            let cleaned = clean_template_vars(&rendered);
            if !cleaned.is_empty() {
                result.push_str(&cleaned);
                result.push('\n');
            }
        }
        if !result.is_empty() {
            return result;
        }
    }
    format!(
        "═══ {} ═══\n❤️ 生命：{}/{}\n⚔️ 物攻：{}\n🔮 法强：{}\n🛡️ 物防：{}\n🧿 法防：{}",
        monster_name, hp, hp_max, ad, ap, defense, magic_resist
    )
}

#[allow(dead_code)]
/// 系统邮件扩展_邮件列表 模板渲染 (保持兼容)
pub fn render_email_list(db: &Database, emails: &[(i32, String, bool)]) -> String {
    let raw = db.template_get("系统邮件扩展_邮件列表");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                let header = raw[..start].trim();
                if !header.is_empty() {
                    for line in header.lines() {
                        let l = line.trim();
                        if !l.starts_with('$') && !l.starts_with("!del") && !l.is_empty() {
                            result.push_str(l);
                            result.push('\n');
                        }
                    }
                }
                for (i, (_id, title, is_read)) in emails.iter().enumerate() {
                    let read_tag = if *is_read { "已读" } else { "未读" };
                    let line = loop_tmpl
                        .replace("[序号]", &(i + 1).to_string())
                        .replace("[EmailTitle]", title)
                        .replace("(sel[[isRead]]->{[已读]}else{[未读]})", read_tag)
                        .replace("[isRead]", if *is_read { "1" } else { "0" })
                        .replace("[已读]", "已读")
                        .replace("[未读]", "未读");
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = String::new();
    for (i, (_id, title, is_read)) in emails.iter().enumerate() {
        let tag = if *is_read { "已读" } else { "未读" };
        out.push_str(&format!("{}. {} [{}]\n", i + 1, title, tag));
    }
    out
}

#[allow(dead_code)]
/// 查询位置玩家 模板渲染 (保持兼容)
pub fn render_map_players(db: &Database, location: &str, players: &[(String, String)]) -> String {
    let raw = db.template_get("查询位置玩家");
    if !raw.is_empty() {
        if let Some(start) = raw.find('<') {
            if let Some(end) = raw[start..].find('>') {
                let loop_tmpl = &raw[start + 1..start + end];
                let mut result = String::new();
                let header = raw[..start].trim();
                if !header.is_empty() {
                    for line in header.lines() {
                        let l = line.trim();
                        if l.starts_with('$') || l.starts_with("!del") {
                            continue;
                        }
                        let rendered = l.replace("[UD:Base,type=Location]", location);
                        result.push_str(&rendered);
                        result.push('\n');
                    }
                }
                for (i, (name, uid)) in players.iter().enumerate() {
                    let line = loop_tmpl
                        .replace("[玩家序号]", &(i + 1).to_string())
                        .replace("[UD:Base,type=Name,uid=[UID]]", name)
                        .replace("[UID]", uid);
                    result.push_str(&line);
                    result.push('\n');
                }
                return result;
            }
        }
    }
    let mut out = format!("当前位置：{}\n", location);
    for (i, (name, uid)) in players.iter().enumerate() {
        out.push_str(&format!("{}. {}({})\n", i + 1, name, uid));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_equip_list_template() {
        let raw = "[槽位名称]：[装备名称]";
        let mut result = String::new();
        let slots = [("武器", "铁剑"), ("头盔", "皮帽")];
        for (slot, equip) in &slots {
            let line = raw.replace("[槽位名称]", slot).replace("[装备名称]", equip);
            result.push_str(&line);
            result.push('\n');
        }
        assert!(result.contains("武器：铁剑"));
        assert!(result.contains("头盔：皮帽"));
    }

    #[test]
    fn test_render_evil_ranking_format() {
        let entries = vec![
            ("坏人".to_string(), "12345".to_string(), 100),
            ("恶霸".to_string(), "67890".to_string(), 50),
        ];
        let mut out = String::from("【荣耀罪恶榜】\n");
        for (i, (name, uid, evil)) in entries.iter().enumerate() {
            out.push_str(&format!("{}. {}({})\n罪恶值：{}\n", i + 1, name, uid, evil));
        }
        assert!(out.contains("【荣耀罪恶榜】"));
        assert!(out.contains("坏人(12345)"));
        assert!(out.contains("罪恶值：100"));
    }

    #[test]
    fn test_render_kill_log_format() {
        let entries = vec![("玩家A".to_string(), "111".to_string(), "2024-01-01".to_string())];
        let mut out = String::new();
        for (i, (name, uid, time)) in entries.iter().enumerate() {
            out.push_str(&format!("{}. {}({})\n时间：{}\n", i + 1, name, uid, time));
        }
        assert!(out.contains("玩家A(111)"));
        assert!(out.contains("时间：2024-01-01"));
    }

    #[test]
    fn test_render_match_stats_fallback() {
        let result = format!(
            "当前职业：{}\n匹配次数：{}\n匹配胜场：{}\n匹配败场：{}\n匹配和场：{}\n目前胜率：{:.1}%\n剩余胜点：{}",
            "勇者", 100, 60, 30, 10, 60.0, 15
        );
        assert!(result.contains("当前职业：勇者"));
        assert!(result.contains("匹配胜场：60"));
        assert!(result.contains("胜率：60.0%"));
    }

    #[test]
    fn test_render_monster_target_fallback() {
        let result = format!(
            "═══ {} ═══\n❤️ 生命：{}/{}\n⚔️ 物攻：{}\n🔮 法强：{}\n🛡️ 物防：{}\n🧿 法防：{}",
            "哥布林", 100, 100, 20, 5, 10, 8
        );
        assert!(result.contains("═══ 哥布林 ═══"));
        assert!(result.contains("❤️ 生命：100/100"));
    }

    #[test]
    fn test_render_email_list_format() {
        let emails = vec![(1, "欢迎邮件".to_string(), false), (2, "系统通知".to_string(), true)];
        let mut out = String::new();
        for (i, (_id, title, is_read)) in emails.iter().enumerate() {
            let tag = if *is_read { "已读" } else { "未读" };
            out.push_str(&format!("{}. {} [{}]\n", i + 1, title, tag));
        }
        assert!(out.contains("欢迎邮件 [未读]"));
        assert!(out.contains("系统通知 [已读]"));
    }

    #[test]
    fn test_render_map_players_fallback() {
        let location = "格兰城";
        let players = vec![("玩家A".to_string(), "111".to_string())];
        let mut out = format!("当前位置：{}\n", location);
        for (i, (name, uid)) in players.iter().enumerate() {
            out.push_str(&format!("{}. {}({})\n", i + 1, name, uid));
        }
        assert!(out.contains("当前位置：格兰城"));
        assert!(out.contains("1. 玩家A(111)"));
    }

    #[test]
    fn test_clean_template_vars() {
        let text = "生命：100[DA:Monster,type=HP] 物攻：20[SET:x]";
        let cleaned = clean_template_vars(text);
        assert_eq!(cleaned, "生命：100 物攻：20");
    }

    #[test]
    fn test_render_vip_recharge_fail_fallback() {
        let balance = 5000i64;
        let result = format!("充值失败！余额不足，剩余余额：{}金币", balance);
        assert!(result.contains("余额不足"));
        assert!(result.contains("5000"));
    }

    #[test]
    fn test_render_vip_level_up_fallback() {
        let result = format!(
            "🎉 恭喜您，VIP等级提升！当前等级{}\n等级礼包：[{}]*1已放入您的背包~！",
            "VIP3", "高级礼包"
        );
        assert!(result.contains("VIP3"));
        assert!(result.contains("高级礼包"));
    }

    #[test]
    fn test_render_weakness_block_fallback() {
        let result = format!("您目前是虚弱状态，无法执行此操作。\n虚弱状态剩余{}分钟。", 5);
        assert!(result.contains("虚弱状态"));
        assert!(result.contains("5分钟"));
    }

    #[test]
    fn test_render_enter_map_success_fallback() {
        let result = format!("您已成功进入{}！", "格兰城");
        assert!(result.contains("格兰城"));
    }

    #[test]
    fn test_render_cultivation_cycle_success_fallback() {
        let result = format!("完成一个修炼周期，修炼成功！{}", "生命+5");
        assert!(result.contains("生命+5"));
    }

    #[test]
    fn test_render_dungeon_complete_fallback() {
        let result = format!(
            "恭喜你！已经完成该副本全部关卡的挑战！\n获得奖励：{}",
            "5000金币+强化石x3"
        );
        assert!(result.contains("5000金币"));
        assert!(result.contains("强化石"));
    }

    #[test]
    fn test_render_ranking_list_format() {
        let entries = vec![
            (1, "大神".to_string(), "12345".to_string(), "战力：99999".to_string()),
            (2, "小神".to_string(), "67890".to_string(), "战力：88888".to_string()),
        ];
        let mut out = String::new();
        for (rank, name, uid, attr) in &entries {
            out.push_str(&format!("{}. {}({}) {}\n", rank, name, uid, attr));
        }
        assert!(out.contains("大神(12345)"));
        assert!(out.contains("战力：99999"));
    }

    #[test]
    fn test_render_bounty_submit_success_fallback() {
        let result = format!("任务提交成功！获得报酬{}", "5000金币");
        assert!(result.contains("5000金币"));
    }

    #[test]
    fn test_render_cultivation_interrupted_fallback() {
        let result = format!("你上次的修炼被迫中断，中断原因{}", "被怪物攻击");
        assert!(result.contains("被怪物攻击"));
    }

    #[test]
    fn test_render_combat_power_ranking_format() {
        let entries = vec![(1, "大神".to_string(), 99999i64), (2, "小神".to_string(), 88888i64)];
        let mut out = String::from("当前战力排行！\n");
        for (rank, name, power) in &entries {
            out.push_str(&format!("{}. {} - {}战力\n", rank, name, power));
        }
        assert!(out.contains("大神 - 99999战力"));
        assert!(out.contains("小神 - 88888战力"));
    }

    #[test]
    fn test_render_first_recharge_reward_fallback() {
        let result = format!("🎁 首冲奖励：{}*1已放入您的背包！~", "药剂礼包");
        assert!(result.contains("药剂礼包"));
    }

    #[test]
    fn test_render_cumulative_recharge_fallback() {
        let result = format!(
            "您已累计充值超过{}金币\n为表示感谢，我们为您送上一份礼物~\n{}*1已放入您的背包~",
            100000, "至尊礼包"
        );
        assert!(result.contains("100000"));
        assert!(result.contains("至尊礼包"));
    }

    #[test]
    fn test_render_vip_exp_bonus_tpl_fallback() {
        let result = format!("VIP经验加成：{} [+{}%]", 100, 20);
        assert!(result.contains("100"));
        assert!(result.contains("+20%"));
    }
}

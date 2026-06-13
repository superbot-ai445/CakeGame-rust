/// CakeGame 指令路由系统
/// 匹配用户输入 → 调用对应处理函数
use crate::core::*;
use crate::db::Database;
use crate::online_activity;
use crate::user;
use std::collections::HashMap;

/// 指令处理器类型
pub type CommandHandler = fn(db: &Database, user_id: &str, args: &str, msg_type: &str, group: &str) -> String;

/// 指令路由器
pub struct Router {
    prefix: String,
    commands: Vec<CommandDef>,
    handlers: HashMap<String, CommandHandler>,
}

impl Router {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            commands: Vec::new(),
            handlers: HashMap::new(),
        }
    }

    /// 注册指令
    pub fn register(&mut self, name: &str, trigger: &str, handler: CommandHandler) {
        self.commands.push(CommandDef {
            name: name.to_string(),
            trigger: trigger.to_string(),
            enabled: true,
            handler_name: name.to_string(),
        });
        self.handlers.insert(name.to_string(), handler);
    }

    /// 处理消息
    pub fn handle(&self, db: &Database, msg: &str, msg_type: &str, group: &str, user_id: &str) -> String {
        let msg = msg.trim();

        // 按指令长度倒序匹配（长指令优先）
        let mut sorted_cmds: Vec<&CommandDef> = self.commands.iter().filter(|c| c.enabled).collect();
        sorted_cmds.sort_by_key(|b| std::cmp::Reverse(b.trigger.len()));

        for cmd in &sorted_cmds {
            let full_trigger = format!("{}{}", self.prefix, cmd.trigger);
            if msg.starts_with(&full_trigger) {
                let args = msg[full_trigger.len()..].trim();
                let args = args.strip_prefix('+').unwrap_or(args).trim();

                // 注册检查：未注册用户只能使用"注册"和"帮助"
                if !db.user_exists(user_id) && cmd.name != "注册" && cmd.name != "帮助" {
                    return format!("ID:{}\n您还未注册！请先发送 注册+昵称 进行注册。", user_id);
                }

                // 检查用户属性是否已初始化（兼容老用户）
                if db.user_exists(user_id) {
                    let hp_base: i32 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(0);
                    if hp_base == 0 {
                        user::verify_occupation_attrs(db, user_id);
                        let hp_max = user::calc_hp_max(db, user_id);
                        let mp_max = user::calc_mp_max(db, user_id);
                        if hp_max > 0 {
                            db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_max);
                            db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_max);
                        }
                    }
                }

                if let Some(handler) = self.handlers.get(&cmd.handler_name) {
                    let result = handler(db, user_id, args, msg_type, group);
                    // 每次指令执行累积1分钟在线时长
                    online_activity::update_user_active_minutes(db, user_id, 1);
                    return result;
                }
            }
        }

        String::new() // 无匹配指令
    }

    /// 获取所有有效指令列表
    #[allow(dead_code)]
    pub fn list_commands(&self) -> Vec<&CommandDef> {
        self.commands.iter().filter(|c| c.enabled).collect()
    }
}

// ==================== 指令处理函数 ====================

/// 注册用户
pub fn cmd_register(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = format!("ID：{}\n", user_id);

    // 检查用户是否已存在
    if db.user_exists(user_id) {
        return format!("{}{}", prefix, "当前帐号已存在！");
    }

    let nickname = args.trim();
    let name_limit: i32 = db.global_get("setup", "name_limit_up").parse().unwrap_or(12);
    if name_limit == 0 {
        // 默认12
    }

    if nickname.len() as i32 > name_limit {
        return format!(
            "{}注册失败。\n昵称长度不得超过{}({}个汉字或{}个英文字符)",
            prefix,
            name_limit,
            name_limit / 2,
            name_limit
        );
    }

    let final_name = if nickname.is_empty() {
        user_id.to_string()
    } else {
        nickname.to_string()
    };

    // 写入基础信息
    db.write_basic(user_id, ITEM_NAME, &final_name);
    let default_occ = db.global_get("OccupationSet", "Default");
    let default_occ = if default_occ.is_empty() { "勇者" } else { &default_occ };
    db.write_basic(user_id, ITEM_OCCUPATION, default_occ);

    let default_city = db.global_get("set", "MajorCities");
    let default_city = if default_city.is_empty() {
        "格兰森林"
    } else {
        &default_city
    };
    db.write_basic(user_id, ITEM_LOCATION, default_city);

    db.write_basic(user_id, ITEM_TARGET, EMPTY);
    db.write_basic(user_id, ITEM_TASK, EMPTY);
    db.write_basic(user_id, ITEM_GUILD, EMPTY);
    db.write_currency(user_id, CURRENCY_GOLD, 0);
    db.write_currency(user_id, CURRENCY_DIAMOND, 0);

    let exp_need = db.global_get("Basis_info", ITEM_EXP_NEED);
    let exp_need = if exp_need.is_empty() { "100" } else { &exp_need };
    db.write_basic(user_id, ITEM_EXP_NEED, exp_need);
    db.write_basic(user_id, ITEM_EXP, "0");
    db.write_basic_int(user_id, ITEM_LEVEL, 1);

    // 置入职业技能
    let occ_skills = db
        .occupation_get(default_occ)
        .map(|o| o.exclusive_skills)
        .unwrap_or_default();
    for skill in &occ_skills {
        db.skill_learn(user_id, skill, 1, default_occ);
    }

    // 新手礼包
    let reward_str = db.global_get("set", "NovicesReward");
    let reward_str = if reward_str.is_empty() {
        "新手药剂礼包*1,初级疗伤技能*1,初级重击技能*1"
    } else {
        &reward_str
    };
    let rewards: Vec<&str> = reward_str.split(',').collect();
    let mut reward_list = String::new();
    for (i, reward) in rewards.iter().enumerate() {
        let parts: Vec<&str> = reward.split('*').collect();
        if parts.len() == 2 {
            let item_name = parts[0];
            let qty: i32 = parts[1].parse().unwrap_or(1);
            db.knapsack_add(user_id, item_name, qty);
            crate::collection::record_item_collection(db, user_id, item_name);
            reward_list.push_str(&format!("\n{}. [{}]×{}", i + 1, item_name, qty));
        }
    }

    // 核对职业属性
    user::verify_occupation_attrs(db, user_id);
    let hp_max = user::calc_hp_max(db, user_id);
    let mp_max = user::calc_mp_max(db, user_id);
    db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_max);
    db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_max);

    format!(
        "{}注册成功！\n昵称：{}\n职业：{}\n位置：{}\n\n获得新手礼包：{}",
        prefix, final_name, default_occ, default_city, reward_list
    )
}

/// 每日签到
pub fn cmd_sign_in(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let sign_date = db.read_user_data(user_id, "sign_in_date");
    let sign_sustain: i32 = db.read_user_data(user_id, "sign_in_sustain").parse().unwrap_or(0);

    if sign_date == today {
        return format!("{}\n今天您已经签到过了！", prefix);
    }

    let consecutive = sign_sustain + 1;
    db.write_user_data(user_id, "sign_in_date", &today);
    db.write_user_data(user_id, "sign_in_sustain", &consecutive.to_string());

    // 累计签到天数
    let total: i32 = db.read_user_data(user_id, "sign_in_total").parse().unwrap_or(0);
    db.write_user_data(user_id, "sign_in_total", &(total + 1).to_string());

    // 记录签到日期到日历
    let cal_key = format!("sign_cal_{}", &today[..7]); // sign_cal_2026-06
    let mut cal = db.read_user_data(user_id, &cal_key);
    let day = &today[8..10];
    if !cal.contains(day) {
        if cal.is_empty() {
            cal = day.to_string();
        } else {
            cal = format!("{},{}", cal, day);
        }
        db.write_user_data(user_id, &cal_key, &cal);
    }

    let mut result = format!(
        "{}\n恭喜您，签到成功！您已经连续签到{}天（累计{}天）",
        prefix,
        consecutive,
        total + 1
    );

    // 活跃度追踪
    let _ = crate::activity::add_activity(db, user_id, "sign_in");

    // 基础签到奖励：金币随连续天数递增
    let gold_reward: i64 = 100 * consecutive as i64;
    db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, gold_reward);
    result.push_str(&format!("\n💰 获得金币：{}", gold_reward));

    // 连续签到里程碑奖励
    let milestone = match consecutive {
        3 => Some((5, "🎁 连续3天奖励！")),
        7 => Some((15, "🎉 连续7天奖励！")),
        14 => Some((30, "🏆 连续14天奖励！")),
        30 => Some((80, "👑 连续30天奖励！")),
        60 => Some((150, "💎 连续60天奖励！")),
        100 => Some((300, "🌟 连续100天至尊奖励！")),
        _ => None,
    };
    if let Some((diamond, msg)) = milestone {
        // 检查是否已领取该里程碑
        let claimed_key = format!("sign_milestone_{}", consecutive);
        let claimed = db.read_user_data(user_id, &claimed_key);
        if claimed.is_empty() {
            db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, diamond);
            db.write_user_data(user_id, &claimed_key, "1");
            result.push_str(&format!("\n{}", msg));
            result.push_str(&format!("\n💎 获得钻石：{}", diamond));
            // 额外奖励道具（高等级里程碑给稀有道具）
            if consecutive >= 30 {
                db.add_item(user_id, "强化石", 3);
                result.push_str("\n🔨 获得强化石×3");
            }
            if consecutive >= 60 {
                db.add_item(user_id, "高级生命药水", 10);
                result.push_str("\n🧪 获得高级生命药水×10");
            }
            if consecutive >= 100 {
                db.add_item(user_id, "复活卷轴", 5);
                result.push_str("\n📜 获得复活卷轴×5");
            }
        }
    }

    // 每日任务进度追踪
    crate::daily_quest::on_signed_in(db, user_id);

    // 周常任务进度追踪
    crate::weekly_quest::on_signed_in(db, user_id);

    // 成就追踪
    crate::achievement::on_sign_in(db, user_id);

    result
}

/// 查看角色
pub fn cmd_view_character(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let info = user::calc_total_attrs(db, user_id);
    let prefix = user::get_msg_prefix(db, user_id);

    let template = db.template_get("玩家角色查询");
    // 只使用简单模板（不包含 $ 或 # 变量语法）
    if !template.is_empty() && !template.contains('$') && !template.contains('#') {
        let mut result = template;
        result = result.replace("[UID]", &info.id);
        result = result.replace("[昵称]", &info.name);
        result = result.replace("[等级]", &info.level.to_string());
        result = result.replace("[职业]", &info.occupation);
        result = result.replace("[生命]", &format!("{}/{}", info.hp, info.hp_max));
        result = result.replace("[魔法]", &format!("{}/{}", info.mp, info.mp_max));
        result = result.replace("[物攻]", &info.ad.to_string());
        result = result.replace("[魔攻]", &info.ap.to_string());
        result = result.replace("[防御]", &info.defense.to_string());
        result = result.replace("[魔抗]", &info.magic_res.to_string());
        result = result.replace("[命中]", &info.hit.to_string());
        result = result.replace("[闪避]", &info.dodge.to_string());
        result = result.replace("[暴击]", &info.crit.to_string());
        result = result.replace("[金币]", &info.gold.to_string());
        result = result.replace("[钻石]", &info.diamond.to_string());
        result = result.replace("[经验]", &format!("{}/{}", info.exp, info.exp_need));
        result = result.replace("[位置]", &info.location);
        return format!("{}\n{}", prefix, result);
    }

    // 无模板时的默认输出
    format!(
        "{}\n\
         ═══ 角色信息 ═══\n\
         昵称：{}\n\
         等级：{}\n\
         职业：{}\n\
         生命：{}/{}\n\
         魔法：{}/{}\n\
         物攻：{}  魔攻：{}\n\
         防御：{}  魔抗：{}\n\
         命中：{}  闪避：{}\n\
         暴击：{}\n\
         经验：{}/{}\n\
         金币：{}  钻石：{}\n\
         位置：{}",
        prefix,
        info.name,
        info.level,
        info.occupation,
        info.hp,
        info.hp_max,
        info.mp,
        info.mp_max,
        info.ad,
        info.ap,
        info.defense,
        info.magic_res,
        info.hit,
        info.dodge,
        info.crit,
        info.exp,
        info.exp_need,
        info.gold,
        info.diamond,
        info.location,
    )
}

/// 查看背包
pub fn cmd_view_knapsack(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);

    let items = db.knapsack_all(user_id);
    if items.is_empty() {
        return format!("{}\n您的背包暂无物品！", prefix);
    }

    let page_size = 10;
    let total_pages = (items.len() as i32 + page_size - 1) / page_size;
    if page > total_pages {
        return format!("{}\n请输入正确的页码！", prefix);
    }

    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(items.len());

    let mut result = format!("{}\n═══ 背包 ({}/{}) ═══", prefix, page, total_pages);
    for (i, item) in items[start..end].iter().enumerate() {
        result.push_str(&format!("\n{}. [{}]×{}", start + i + 1, item.name, item.quantity));
    }
    result.push_str("\n\n输入'背包+页码'翻页");
    result
}

/// 使用物品
#[allow(unused_assignments)]
pub fn cmd_use_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return String::new();
    }

    // 解析 "物品名*数量"
    let parts: Vec<&str> = item_name.split('*').collect();
    let name = parts[0];
    let use_qty: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);

    let qty = db.knapsack_quantity(user_id, name);
    if qty < use_qty {
        return format!("{}\n您没有足够的 [{}]！", prefix, name);
    }

    let item = db.item_get(name);
    if item.is_none() {
        return format!("{}\n物品 [{}] 不存在！", prefix, name);
    }
    let item = item.unwrap();

    let mut result = String::new();

    match item.item_type.as_str() {
        "potion" | "药剂" => {
            // 使用药剂
            let effect = item.data.effect;
            let s_type = &item.data.s_type;
            let role = &item.data.role;
            match (s_type.as_str(), role.as_str()) {
                ("生命", _) | (_, "HP") => {
                    let hp_max = user::calc_hp_max(db, user_id);
                    let mut hp: i32 = db.read_basic(user_id, ITEM_HP_CURRENT).parse().unwrap_or(0);
                    hp = (hp + effect * use_qty).min(hp_max);
                    db.write_basic_int(user_id, ITEM_HP_CURRENT, hp);
                    result = format!(
                        "{}\n使用 [{}]×{}\n恢复生命 {}\n当前生命：{}/{}",
                        prefix,
                        name,
                        use_qty,
                        effect * use_qty,
                        hp,
                        hp_max
                    );
                }
                ("魔法", _) | (_, "MP") => {
                    let mp_max = user::calc_mp_max(db, user_id);
                    let mut mp: i32 = db.read_basic(user_id, ITEM_MP_CURRENT).parse().unwrap_or(0);
                    mp = (mp + effect * use_qty).min(mp_max);
                    db.write_basic_int(user_id, ITEM_MP_CURRENT, mp);
                    result = format!(
                        "{}\n使用 [{}]×{}\n恢复魔法 {}\n当前魔法：{}/{}",
                        prefix,
                        name,
                        use_qty,
                        effect * use_qty,
                        mp,
                        mp_max
                    );
                }
                _ => {
                    result = format!("{}\n使用 [{}]×{}", prefix, name, use_qty);
                }
            }
            db.knapsack_remove(user_id, name, use_qty);
        }
        "装备" => {
            // 装备物品
            let slot = &item.data.slot_name;
            if slot.is_empty() {
                return format!("{}\n[{}] 无法装备！", prefix, name);
            }
            // 检查等级限制
            let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
            if item.data.lv_limit > 0 && level < item.data.lv_limit {
                return format!("{}\n等级不足，装备 [{}] 需要等级 {}", prefix, name, item.data.lv_limit);
            }
            // 卸下旧装备
            let old = db.equip_remove(user_id, slot);
            // 装备新物品
            db.equip_set(user_id, slot, &item);
            db.knapsack_remove(user_id, name, 1);
            // 把旧装备放回背包
            if let Some(old_name) = old {
                db.knapsack_add(user_id, &old_name, 1);
                result = format!("{}\n装备 [{}]\n卸下 [{}]", prefix, name, old_name);
            } else {
                result = format!("{}\n装备 [{}]", prefix, name);
            }
        }
        "礼包" => {
            // 打开礼包 - 先查找 ext_libao_info 获取真实奖励
            let libao_rewards = db.query_row(
                "SELECT GetGoods, NeedLV FROM ext_libao_info WHERE Goods = ?",
                &[name],
                |row| {
                    Ok((
                        row.get::<_, String>(0).unwrap_or_default(),
                        row.get::<_, String>(1).unwrap_or_default(),
                    ))
                },
            );

            // 检查等级限制
            if let Ok((_, ref need_lv_str)) = libao_rewards {
                let need_lv: i32 = need_lv_str.parse().unwrap_or(0);
                if need_lv > 0 {
                    let user_lv: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
                    if user_lv < need_lv {
                        return format!(
                            "{}\n打开 [{}] 需要等级 {}，您当前等级 {}。",
                            prefix, name, need_lv, user_lv
                        );
                    }
                }
            }

            match libao_rewards {
                Ok((get_goods, _)) if !get_goods.is_empty() => {
                    // 解析奖励 (格式: \x01物品名GH数量GI物品名GH数量)
                    db.knapsack_remove(user_id, name, use_qty);
                    result = format!("{}\n打开 [{}]×{}", prefix, name, use_qty);

                    // 去掉 \x01 前缀，按 GI 分割
                    let clean = get_goods.trim_start_matches('\x01');
                    let items: Vec<&str> = clean.split("GI").collect();
                    for item_str in items {
                        let item_str = item_str.trim();
                        if item_str.is_empty() {
                            continue;
                        }
                        // 按 GH 分割物品名和数量
                        if let Some(gh_pos) = item_str.find("GH") {
                            let reward_name = &item_str[..gh_pos];
                            let qty_str = &item_str[gh_pos + 2..];
                            let qty: i32 = qty_str.trim().parse().unwrap_or(1);
                            if !reward_name.is_empty() {
                                db.add_item(user_id, reward_name, qty * use_qty);
                                result.push_str(&format!("\n获得：[{}]×{}", reward_name, qty * use_qty));
                            }
                        }
                    }
                }
                _ => {
                    // 回退到默认奖励
                    result = format!("{}\n打开 [{}]×{}", prefix, name, use_qty);
                    db.knapsack_remove(user_id, name, use_qty);
                    db.knapsack_add(user_id, "初级疗伤药剂", 3 * use_qty);
                    result.push_str(&format!("\n获得 [初级疗伤药剂]×{}", 3 * use_qty));
                }
            }
        }
        "技能书" => {
            // 学习技能
            let skill_name = &item.data.role;
            if skill_name.is_empty() {
                return format!("{}\n[{}] 无法使用！", prefix, name);
            }
            if db.skill_has(user_id, skill_name) {
                return format!("{}\n您已经学会了 [{}]！", prefix, skill_name);
            }
            let occupation = db.read_basic(user_id, ITEM_OCCUPATION);
            db.skill_learn(user_id, skill_name, 1, &occupation);
            db.knapsack_remove(user_id, name, 1);
            crate::achievement::on_skill_learned(db, user_id);
            result = format!("{}\n使用 [{}]\n学会了技能 [{}]", prefix, name, skill_name);
        }
        "货币袋" => {
            // 开出货币
            let effect = item.data.effect as i64;
            db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, effect * use_qty as i64);
            db.knapsack_remove(user_id, name, use_qty);
            result = format!(
                "{}\n打开 [{}]×{}\n获得金币 {}",
                prefix,
                name,
                use_qty,
                effect * use_qty as i64
            );
        }
        _ => {
            result = format!("{}\n使用 [{}]×{}", prefix, name, use_qty);
            db.knapsack_remove(user_id, name, use_qty);
        }
    }

    result
}

/// 丢弃物品
pub fn cmd_drop_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();
    if item_name.is_empty() {
        return String::new();
    }

    let parts: Vec<&str> = item_name.split('*').collect();
    let name = parts[0];
    let drop_qty: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1).max(1);

    let qty = db.knapsack_quantity(user_id, name);
    if qty < drop_qty {
        return format!("{}\n您没有足够的 [{}]！", prefix, name);
    }

    db.knapsack_remove(user_id, name, drop_qty);
    format!("{}\n丢弃 [{}]×{}", prefix, name, drop_qty)
}

/// 查看装备
pub fn cmd_view_equips(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let equips = db.equip_all(user_id);

    if equips.is_empty() {
        return format!("{}\n您还没有装备任何物品！", prefix);
    }

    let mut result = format!("{}\n═══ 装备栏 ═══", prefix);
    let slot_names = vec![
        ("武器", SLOT_WEAPON),
        ("头盔", SLOT_HELMET),
        ("铠甲", SLOT_ARMOR),
        ("护腿", SLOT_LEG),
        ("靴子", SLOT_BOOTS),
        ("项链", SLOT_NECKLACE),
        ("戒指", SLOT_RING),
        ("翅膀", SLOT_WING),
        ("时装", SLOT_FASHION),
        ("称号", SLOT_TITLE),
    ];

    for (display, slot) in &slot_names {
        if let Some(eq) = equips.iter().find(|e| e.slot == *slot) {
            result.push_str(&format!("\n{}：[{}]", display, eq.name));
            let mut attrs = Vec::new();
            if eq.add_hp > 0 {
                attrs.push(format!("生命+{}", eq.add_hp));
            }
            if eq.add_mp > 0 {
                attrs.push(format!("魔法+{}", eq.add_mp));
            }
            if eq.add_ad > 0 {
                attrs.push(format!("物攻+{}", eq.add_ad));
            }
            if eq.add_ap > 0 {
                attrs.push(format!("魔攻+{}", eq.add_ap));
            }
            if eq.add_defense > 0 {
                attrs.push(format!("防御+{}", eq.add_defense));
            }
            if !attrs.is_empty() {
                result.push_str(&format!("  ({})", attrs.join(" ")));
            }
        } else {
            result.push_str(&format!("\n{}：空", display));
        }
    }

    result
}

/// 卸下装备
pub fn cmd_unequip(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let slot = args.trim();

    let slot_map: HashMap<&str, &str> = [
        ("武器", SLOT_WEAPON),
        ("头盔", SLOT_HELMET),
        ("铠甲", SLOT_ARMOR),
        ("护腿", SLOT_LEG),
        ("靴子", SLOT_BOOTS),
        ("项链", SLOT_NECKLACE),
        ("戒指", SLOT_RING),
        ("翅膀", SLOT_WING),
        ("时装", SLOT_FASHION),
        ("称号", SLOT_TITLE),
    ]
    .iter()
    .cloned()
    .collect();

    let actual_slot = slot_map.get(slot).unwrap_or(&slot);

    match db.equip_remove(user_id, actual_slot) {
        Some(name) => {
            db.knapsack_add(user_id, &name, 1);
            format!("{}\n卸下装备 [{}]", prefix, name)
        }
        None => format!("{}\n该槽位没有装备！", prefix),
    }
}

/// 查看地图列表
pub fn cmd_view_maps(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let _page: i32 = args.trim().parse().unwrap_or(1).max(1);
    let location = db.read_basic(user_id, ITEM_LOCATION);

    let mut result = format!("{}\n═══ 当前位置：{} ═══", prefix, location);

    if let Some(map) = db.map_get(&location) {
        result.push_str(&format!("\n{}", map.introduce));
        result.push_str(&format!("\n安全区：{}", if map.security { "是" } else { "否" }));
        result.push_str("\n\n可前往：");
        if !map.up.is_empty() {
            result.push_str(&format!("\n  ↑ {}", map.up));
        }
        if !map.down.is_empty() {
            result.push_str(&format!("\n  ↓ {}", map.down));
        }
        if !map.left.is_empty() {
            result.push_str(&format!("\n  ← {}", map.left));
        }
        if !map.right.is_empty() {
            result.push_str(&format!("\n  → {}", map.right));
        }
    }

    result
}

/// 进入地图
pub fn cmd_enter_map(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let direction = args.trim();
    let current = db.read_basic(user_id, ITEM_LOCATION);

    let map = db.map_get(&current);
    if map.is_none() {
        return format!("{}\n当前位置数据异常！", prefix);
    }
    let map = map.unwrap();

    let target = match direction {
        "上" | "北" => &map.up,
        "下" | "南" => &map.down,
        "左" | "西" => &map.left,
        "右" | "东" => &map.right,
        _ => {
            // 直接输入地图名
            let target_map = db.map_get(direction);
            if target_map.is_some() {
                // 检查是否相邻
                if map.up == direction || map.down == direction || map.left == direction || map.right == direction {
                    direction
                } else {
                    return format!("{}\n[{}] 不在当前位置的相邻地图中！", prefix, direction);
                }
            } else {
                return format!("{}\n请输入方向（上下左右）或地图名称！", prefix);
            }
        }
    };

    if target.is_empty() {
        return format!("{}\n这个方向没有通往任何地方！", prefix);
    }

    db.write_basic(user_id, ITEM_LOCATION, target);
    // 进入新地图后清除目标
    db.write_basic(user_id, ITEM_TARGET, EMPTY);

    // 成就追踪
    crate::achievement::on_map_visit(db, user_id);

    format!("{}\n您已前往 [{}]", prefix, target)
}

/// 查看当前地图信息
pub fn cmd_location_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let location = db.read_basic(user_id, ITEM_LOCATION);
    let target = db.read_basic(user_id, ITEM_TARGET);

    let mut result = format!("{}\n═══ 位置信息 ═══\n当前位置：{}", prefix, location);

    if let Some(map) = db.map_get(&location) {
        result.push_str(&format!("\n{}", map.introduce));
        result.push_str(&format!(
            "\n等级要求：{}",
            if map.level > 0 {
                map.level.to_string()
            } else {
                "无".to_string()
            }
        ));
        result.push_str(&format!("\n安全区：{}", if map.security { "是" } else { "否" }));

        if !map.monsters.is_empty() {
            result.push_str(&format!("\n出没怪物：{}", map.monsters.join("、")));
        }
    }

    if !target.is_empty() {
        result.push_str(&format!("\n当前目标：{}", target));
    }

    result
}

/// 搜索怪物
pub fn cmd_search_monster(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let location = db.read_basic(user_id, ITEM_LOCATION);

    let map = db.map_get(&location);
    if map.is_none() {
        return format!("{}\n当前位置数据异常！", prefix);
    }
    let map = map.unwrap();

    if map.monsters.is_empty() {
        return format!("{}\n当前位置没有怪物！", prefix);
    }

    let monster_name = if args.trim().is_empty() {
        // 随机选择一个怪物
        let idx = rand::random::<usize>() % map.monsters.len();
        map.monsters[idx].clone()
    } else {
        args.trim().to_string()
    };

    // 检查怪物是否在当前地图
    if !map.monsters.contains(&monster_name) {
        return format!("{}\n当前位置没有 [{}] 这种怪物！", prefix, monster_name);
    }

    let monster = db.monster_get(&monster_name);
    if monster.is_none() {
        return format!("{}\n怪物数据异常！", prefix);
    }
    let monster = monster.unwrap();

    db.write_basic(user_id, ITEM_TARGET, &monster_name);

    format!(
        "{}\n搜索到怪物 [{}]\n等级：{}\n生命：{}\n物攻：{} 魔攻：{}\n防御：{} 魔抗：{}\n已锁定为目标！",
        prefix,
        monster.name,
        monster.monster_type,
        monster.hp,
        monster.ad,
        monster.ap,
        monster.defense,
        monster.magic_resistance
    )
}

/// 锁定目标
pub fn cmd_lock_target(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target = args.trim();
    if target.is_empty() {
        return String::new();
    }

    db.write_basic(user_id, ITEM_TARGET, target);
    format!("{}\n已锁定目标 [{}]", prefix, target)
}

/// 解锁目标
pub fn cmd_unlock_target(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    db.write_basic(user_id, ITEM_TARGET, EMPTY);
    format!("{}\n已解锁目标", prefix)
}

/// 查看技能
pub fn cmd_view_skills(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let skills = db.skill_all(user_id);

    if skills.is_empty() {
        return format!("{}\n您还没有学会任何技能！", prefix);
    }

    let mut result = format!("{}\n═══ 技能列表 ═══", prefix);
    for (i, (name, prof)) in skills.iter().enumerate() {
        let skill_info = db.skill_get(name);
        let desc = skill_info.map(|s| s.introduce).unwrap_or_default();
        result.push_str(&format!("\n{}. [{}] 熟练度：{}", i + 1, name, prof));
        if !desc.is_empty() {
            result.push_str(&format!(" - {}", desc));
        }
    }

    result
}

/// 修改昵称
pub fn cmd_rename(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let new_name = args.trim();
    if new_name.is_empty() {
        return format!("{}\n请输入新昵称！", prefix);
    }

    let name_limit: i32 = 12;
    if new_name.len() as i32 > name_limit {
        return format!("{}\n昵称长度不得超过{}！", prefix, name_limit);
    }

    db.write_basic(user_id, ITEM_NAME, new_name);
    format!("{}\n昵称已修改为 [{}]", prefix, new_name)
}

/// 帮助列表
pub fn cmd_help(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let template = db.template_get("帮助列表");
    if !template.is_empty() {
        return format!("{}\n{}", prefix, template);
    }

    format!(
        "{}\n\
         ═══ 帮助列表 ═══\n\
         注册+昵称 - 注册游戏\n\
         签到 - 每日签到\n\
         查看角色 - 查看角色信息\n\
         查看背包 - 查看背包物品\n\
         使用+物品名 - 使用物品\n\
         丢弃+物品名 - 丢弃物品\n\
         查看装备 - 查看装备栏\n\
         卸下+槽位 - 卸下装备\n\
         查看技能 - 查看技能列表\n\
         搜索怪物 - 搜索并锁定怪物\n\
         攻击 - 攻击当前目标\n\
         释放技能+技能名 - 使用技能\n\
         查看地图 - 查看当前位置\n\
         进入+方向 - 移动到相邻地图\n\
         查看商店 - 查看系统商店\n\
         购买+商品名 - 购买商品\n\
         我的任务 - 查看任务列表\n\
         等级排行 - 查看等级排行\n\
         修改昵称+新昵称 - 修改昵称\n\
         帮助 - 显示此帮助",
        prefix
    )
}

/// GM 发放奖励
pub fn cmd_gm_reward(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power: i32 = db.global_get("Permissions", user_id).parse().unwrap_or(0);
    if power < 100 {
        return format!("{}\n你无权操作", prefix);
    }

    let parts: Vec<&str> = args.splitn(3, '+').collect();
    if parts.len() < 3 {
        return format!("{}\n格式：发放奖励+QQ+类型+数量", prefix);
    }

    let target = parts[0];
    let reward_type = parts[1];
    let amount: i64 = parts[2].parse().unwrap_or(0);

    match reward_type {
        "金币" => {
            db.modify_currency(target, CURRENCY_GOLD, OP_ADD, amount);
            format!("{}\n已向 [{}] 发放金币 {}", prefix, target, amount)
        }
        "钻石" => {
            db.modify_currency(target, CURRENCY_DIAMOND, OP_ADD, amount);
            format!("{}\n已向 [{}] 发放钻石 {}", prefix, target, amount)
        }
        "经验" => {
            user::add_experience(db, target, amount as i32);
            format!("{}\n已向 [{}] 发放经验 {}", prefix, target, amount)
        }
        _ => {
            // 物品
            db.knapsack_add(target, reward_type, amount as i32);
            crate::collection::record_item_collection(db, target, reward_type);
            format!("{}\n已向 [{}] 发放 [{}]×{}", prefix, target, reward_type, amount)
        }
    }
}

/// 查看夜市
pub fn cmd_view_yesi(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.conn.lock().unwrap();
    let mut stmt = match conn.prepare("SELECT Name, PR FROM Ext_yesi") {
        Ok(s) => s,
        Err(_) => return format!("{}\n夜市暂未开放", prefix),
    };
    let items: Vec<(String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
            ))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    drop(stmt);
    drop(conn);

    if items.is_empty() {
        return format!("{}\n🌙 夜市暂无商品", prefix);
    }

    let mut out = format!("{}\n🌙 【夜市】\n━━━━━━━━━━━━━━━━━━━━\n", prefix);
    for (i, (name, price)) in items.iter().enumerate() {
        let name_decoded = crate::encoding::smart_decode(name);
        let price_v: i32 = price.parse().unwrap_or(0);
        out.push_str(&format!("{}. {} - {}金币\n", i + 1, name_decoded, price_v));
    }
    out.push_str("━━━━━━━━━━━━━━━━━━━━\n");
    out.push_str("💡 使用「购买商品+物品名」购买\n");
    out
}

/// 查看排行
pub fn cmd_view_rank(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let rank_type = args.trim();

    let (node, item, display_name) = match rank_type {
        "等级" => (NODE_BASIC, ITEM_LEVEL, "等级"),
        "生命" => (NODE_BASIC, ITEM_HP, "生命"),
        "魔法" => (NODE_BASIC, ITEM_MP, "魔法"),
        "物攻" => (NODE_BASIC, ITEM_AD, "物攻"),
        "魔攻" => (NODE_BASIC, ITEM_AP, "魔攻"),
        "防御" => (NODE_BASIC, ITEM_DEFENSE, "防御"),
        "魔抗" => (NODE_BASIC, ITEM_MAGIC_RES, "魔抗"),
        "金币" => (NODE_CURRENCY, CURRENCY_GOLD, "金币"),
        "钻石" => (NODE_CURRENCY, CURRENCY_DIAMOND, "钻石"),
        _ => (NODE_BASIC, ITEM_LEVEL, "等级"),
    };

    let ranks = db.rank_by_attribute(node, item, 10);
    if ranks.is_empty() {
        return format!("{}\n暂无排行数据！", prefix);
    }

    let mut result = format!("{}\n═══ {}排行 ═══", prefix, display_name);
    for entry in &ranks {
        result.push_str(&format!(
            "\n{}. {} - {}：{}",
            entry.rank, entry.name, display_name, entry.value
        ));
    }

    result
}

/// 攻击当前目标
pub fn cmd_attack(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let result = crate::combat::attack_monster(db, user_id, "");
    result.text
}

/// 自动攻击
pub fn cmd_auto_attack(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let max_times: i32 = db.global_get("automatic_combat", "number.max").parse().unwrap_or(6);
    let skill_plan: Vec<&str> = args.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    let mut output = String::new();
    let mut times = 0;
    let mut plan_idx = 0;

    loop {
        times += 1;
        if times > max_times {
            break;
        }

        let skill = if !skill_plan.is_empty() {
            let s = skill_plan[plan_idx % skill_plan.len()];
            plan_idx += 1;
            if s == "0" || s == "普通攻击" {
                ""
            } else {
                s
            }
        } else {
            ""
        };

        let result = crate::combat::attack_monster(db, user_id, skill);
        output.push_str(&format!("\n[自动战斗第{}次]\n{}", times, result.text));

        if result.error_code != 0
            && result.error_code != 1008
            && result.error_code != 1009
            && result.error_code != 1010
            && result.error_code != 1011
            && result.error_code != 1012
        {
            output.push_str("\n执行过程中出现错误，已跳出自动战斗");
            break;
        }

        // 检查怪物是否已死（目标为空表示已击败）
        let target = db.read_basic(user_id, ITEM_TARGET);
        if target.is_empty() {
            break;
        }
    }

    format!("{}{}", prefix, output)
}

/// 释放技能
pub fn cmd_use_skill(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let skill_name = args.trim();
    if skill_name.is_empty() {
        let prefix = user::get_msg_prefix(db, user_id);
        return format!("{}\n请输入技能名称！", prefix);
    }
    let result = crate::combat::attack_monster(db, user_id, skill_name);
    result.text
}

/// 查看商店
pub fn cmd_view_shop(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);
    crate::shop::view_shop(db, user_id, page)
}

/// 购买商品
pub fn cmd_buy_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::shop::buy_item(db, user_id, args.trim())
}

/// 查看合成配方
pub fn cmd_view_composite(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::shop::view_composite(db, user_id, args.trim())
}

/// 合成物品
pub fn cmd_composite(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::shop::composite_item(db, user_id, args.trim())
}

/// 合成列表
pub fn cmd_composite_list(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);
    crate::shop::composite_list(db, user_id, page)
}

/// 创建公会
pub fn cmd_create_guild(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::create_guild(db, user_id, args.trim())
}

/// 我的公会
pub fn cmd_my_guild(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::view_guild(db, user_id)
}

/// 退出公会
pub fn cmd_leave_guild(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::leave_guild(db, user_id)
}

/// 解散公会
pub fn cmd_disband_guild(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::disband_guild(db, user_id)
}

/// 转让公会
pub fn cmd_transfer_guild(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::transfer_guild(db, user_id, args.trim())
}

/// 公会列表
pub fn cmd_guild_list(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let page: i32 = args.trim().parse().unwrap_or(1).max(1);
    crate::social::guild_list(db, user_id, page)
}

/// 公会捐献
pub fn cmd_guild_donate(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::guild_donate(db, user_id, args.trim())
}

/// 公会成员
pub fn cmd_guild_members(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::guild_members(db, user_id)
}

/// 踢出成员
pub fn cmd_kick_guild_member(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::kick_guild_member(db, user_id, args.trim())
}

/// 申请加入公会
pub fn cmd_apply_guild(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::apply_guild(db, user_id, args.trim())
}

/// 批准加入公会
pub fn cmd_approve_guild(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::approve_guild(db, user_id, args.trim())
}

/// 拒绝加入公会
pub fn cmd_reject_guild(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::reject_guild(db, user_id, args.trim())
}

/// 查看目标
pub fn cmd_view_target(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::view_target(db, user_id)
}

/// 位置玩家
pub fn cmd_view_map_players(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::view_map_players(db, user_id)
}

/// 喊话
pub fn cmd_shout(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::shout(db, user_id, args.trim())
}

/// 创建队伍
pub fn cmd_create_team(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::create_team(db, user_id, args.trim())
}

/// 加入队伍
pub fn cmd_join_team(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::join_team(db, user_id, args.trim())
}

/// 退出队伍
pub fn cmd_leave_team(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::leave_team(db, user_id)
}

/// 队伍成员
pub fn cmd_team_members(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::team_members(db, user_id)
}

/// 请出队伍成员
pub fn cmd_kick_team_member(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::kick_team_member(db, user_id, args.trim())
}

/// 匹配竞技
pub fn cmd_match_arena(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::match_arena(db, user_id)
}

/// 匹配信息
pub fn cmd_match_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::match_info(db, user_id)
}

/// 匹配排行
pub fn cmd_match_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::match_ranking(db, user_id)
}

/// 加入匹配队列
pub fn cmd_match_queue_join(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::match_queue_join(db, user_id)
}

/// 退出匹配队列
pub fn cmd_match_queue_leave(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::match_queue_leave(db, user_id)
}

/// 匹配队列
pub fn cmd_match_queue_view(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    crate::social::match_queue_view(db, user_id)
}

/// 查看职业
pub fn cmd_view_occupation(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let occ_name = args.trim();

    // 支持序号或名称
    let actual_name = if let Ok(idx) = occ_name.parse::<usize>() {
        if idx > 0 {
            let all_occ = db.occupation_list();
            if let Some(occ) = all_occ.get(idx - 1) {
                occ.clone()
            } else {
                return format!("{}\n未找到此职业！", prefix);
            }
        } else {
            return format!("{}\n未找到此职业！", prefix);
        }
    } else if occ_name.is_empty() {
        // 显示当前职业
        db.read_basic(user_id, ITEM_OCCUPATION)
    } else {
        occ_name.to_string()
    };

    match db.occupation_get(&actual_name) {
        Some(occ) => {
            let mut result = format!("{}\n═══ {} ═══", prefix, occ.name);
            result.push_str(&format!("\n{}", occ.intro.replace("\r\n", "\n")));
            result.push_str("\n\n基础属性：");
            result.push_str(&format!("\n  生命：{} 魔法：{}", occ.hp, occ.mp));
            result.push_str(&format!("\n  物攻：{} 魔攻：{}", occ.ad, occ.ap));
            result.push_str(&format!("\n  防御：{} 魔抗：{}", occ.defense, occ.magic_resistance));
            result.push_str(&format!("\n  命中：{} 闪避：{}", occ.hit, occ.dodge));
            result.push_str(&format!("\n  暴击：{} 吸血：{}", occ.crit, occ.absorb_hp));
            if !occ.exclusive_skills.is_empty() {
                result.push_str(&format!("\n\n专属技能：{}", occ.exclusive_skills.join("、")));
            }
            if !occ.transfer_demand.is_empty() && occ.transfer_demand != "[NULL]" {
                result.push_str(&format!("\n转职需求：{}", occ.transfer_demand));
            }
            if occ.transfer_level > 0 {
                result.push_str(&format!("\n转职等级：{}", occ.transfer_level));
            }
            result
        }
        None => format!("{}\n查询[{}]失败，未找到此职业！", prefix, actual_name),
    }
}

/// 职业列表
pub fn cmd_occupation_list(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let all_occ = db.occupation_list();

    if all_occ.is_empty() {
        return format!("{}\n暂无职业数据！", prefix);
    }

    let mut result = format!("{}\n═══ 职业列表 ═══", prefix);
    for (i, name) in all_occ.iter().enumerate() {
        result.push_str(&format!("\n{}. {}", i + 1, name));
    }
    result.push_str("\n\n查看职业详情：发送'查看职业+职业名'");
    result
}

/// 转换职业
pub fn cmd_change_occupation(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let target_name = args.trim();

    if target_name.is_empty() {
        return format!("{}\n请指定要转换的职业！格式：转换职业+职业名", prefix);
    }

    // 获取当前职业
    let current_occ = db.read_basic(user_id, ITEM_OCCUPATION);
    if current_occ.is_empty() {
        return format!("{}\n您还未注册职业！", prefix);
    }

    // 不能转职到当前职业
    if current_occ == target_name {
        return format!("{}\n您已经是 [{}] 职业了！", prefix, target_name);
    }

    // 获取目标职业配置
    let target_occ = match db.occupation_get(target_name) {
        Some(occ) => occ,
        None => return format!("{}\n职业 [{}] 不存在！", prefix, target_name),
    };

    // 检查等级要求
    let user_level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(0);
    if target_occ.transfer_level > 0 && user_level < target_occ.transfer_level {
        return format!(
            "{}\n转职失败！需要等级 {}，当前等级 {}",
            prefix, target_occ.transfer_level, user_level
        );
    }

    // 检查前置职业要求
    if !target_occ.former_occupation.is_empty()
        && target_occ.former_occupation != "[NULL]"
        && current_occ != target_occ.former_occupation
    {
        return format!(
            "{}\n转职失败！[{}] 需要前置职业 [{}]，当前职业 [{}]",
            prefix, target_name, target_occ.former_occupation, current_occ
        );
    }

    // 检查转职物品需求
    if !target_occ.transfer_demand.is_empty() && target_occ.transfer_demand != "[NULL]" {
        let demands: Vec<&str> = target_occ.transfer_demand.split(',').collect();
        for demand in &demands {
            let demand = demand.trim();
            if let Some((item_name, qty_str)) = demand.rsplit_once('*') {
                let item_name = item_name.trim();
                let required_qty: i32 = qty_str.trim().parse().unwrap_or(0);
                let owned_qty = db.get_item_count(user_id, item_name);
                if owned_qty < required_qty {
                    return format!(
                        "{}\n转职失败！缺少物品 [{}]×{}（拥有{}）",
                        prefix, item_name, required_qty, owned_qty
                    );
                }
            }
        }

        // 所有物品检查通过，扣除物品
        for demand in &demands {
            let demand = demand.trim();
            if let Some((item_name, qty_str)) = demand.rsplit_once('*') {
                let item_name = item_name.trim();
                let required_qty: i32 = qty_str.trim().parse().unwrap_or(0);
                db.remove_item(user_id, item_name, required_qty);
            }
        }
    }

    // 所有检查通过，执行转职
    db.write_basic(user_id, ITEM_OCCUPATION, target_name);

    // 重新计算职业属性
    crate::user::verify_occupation_attrs(db, user_id);

    // 恢复满血满蓝
    let hp_max = crate::user::calc_hp_max(db, user_id);
    let mp_max = crate::user::calc_mp_max(db, user_id);
    db.write_basic_int(user_id, ITEM_HP_CURRENT, hp_max);
    db.write_basic_int(user_id, ITEM_MP_CURRENT, mp_max);

    let mut result = format!("{}\n═══ 转职成功！ ═══", prefix);
    result.push_str(&format!("\n{} → {}", current_occ, target_name));
    result.push_str(&format!("\n生命：{} 魔法：{}", target_occ.hp, target_occ.mp));
    result.push_str(&format!("\n物攻：{} 魔攻：{}", target_occ.ad, target_occ.ap));
    result.push_str(&format!(
        "\n防御：{} 魔抗：{}",
        target_occ.defense, target_occ.magic_resistance
    ));
    if !target_occ.exclusive_skills.is_empty() {
        result.push_str(&format!("\n专属技能：{}", target_occ.exclusive_skills.join("、")));
    }
    result.push_str("\n\n属性已重置，生命和魔法已恢复满值！");
    result
}

/// 背包筛选
pub fn cmd_filter_knapsack(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let filter_type = args.trim();

    if filter_type.is_empty() {
        return format!("{}\n请指定筛选类型：装备、药剂、材料、礼包、技能书、货币袋", prefix);
    }

    let items = db.knapsack_all(user_id);
    let filtered: Vec<_> = items
        .iter()
        .filter(|item| {
            if let Some(item_def) = db.item_get(&item.name) {
                match filter_type {
                    "装备" => item_def.item_type == "Equip",
                    "药剂" => item_def.item_type == "potion",
                    "材料" => item_def.item_type == "material",
                    "礼包" => item_def.item_type == "GiftBag",
                    "技能书" => item_def.item_type == "skillbook",
                    "货币袋" => item_def.item_type == "currencybag",
                    _ => true,
                }
            } else {
                false
            }
        })
        .collect();

    if filtered.is_empty() {
        return format!("{}\n背包中没有{}类型的物品！", prefix, filter_type);
    }

    let mut result = format!("{}\n═══ 背包筛选：{} ({}) ═══", prefix, filter_type, filtered.len());
    for (i, item) in filtered.iter().enumerate() {
        result.push_str(&format!("\n{}. [{}]×{}", i + 1, item.name, item.quantity));
    }
    result
}

/// 查看物品详情
pub fn cmd_view_item(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let item_name = args.trim();

    if item_name.is_empty() {
        return format!("{}\n请输入物品名称！", prefix);
    }

    match db.item_get(item_name) {
        Some(item) => {
            let mut result = format!("{}\n═══ {} ═══", prefix, item.name);
            result.push_str(&format!("\n类型：{}", item.item_type));
            if !item.introduce.is_empty() && item.introduce != "[NULL]" {
                result.push_str(&format!("\n介绍：{}", item.introduce));
            }
            if !item.data.slot_name.is_empty() && item.data.slot_name != "[NULL]" {
                result.push_str(&format!("\n槽位：{}", item.data.slot_name));
            }
            if item.data.use_lv > 0 {
                result.push_str(&format!("\n需求等级：{}", item.data.use_lv));
            }
            if item.data.add_hp > 0 {
                result.push_str(&format!("\n生命+{}", item.data.add_hp));
            }
            if item.data.add_mp > 0 {
                result.push_str(&format!("\n魔法+{}", item.data.add_mp));
            }
            if item.data.add_ad > 0 {
                result.push_str(&format!("\n物攻+{}", item.data.add_ad));
            }
            if item.data.add_ap > 0 {
                result.push_str(&format!("\n魔攻+{}", item.data.add_ap));
            }
            if item.data.add_defense > 0 {
                result.push_str(&format!("\n防御+{}", item.data.add_defense));
            }
            if item.data.add_magic > 0 {
                result.push_str(&format!("\n魔抗+{}", item.data.add_magic));
            }
            if item.data.effect > 0 {
                result.push_str(&format!("\n效果：{}", item.data.effect));
            }
            result
        }
        None => format!("{}\n物品[{}]不存在！", prefix, item_name),
    }
}

/// 玩家赠送
pub fn cmd_gift(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let args = args.trim();

    // 格式: 物品+目标QQ|物品名*数量 或 金币+目标QQ+数量
    if let Some(data) = args.strip_prefix("物品") {
        // 去掉"物品"前缀
        let parts: Vec<&str> = data.splitn(2, '|').collect();
        if parts.len() < 2 {
            return format!("{}\n格式：赠送物品+目标QQ|物品名*数量", prefix);
        }
        let target = parts[0];
        let item_parts: Vec<&str> = parts[1].split('*').collect();
        let item_name = item_parts[0];
        let qty: i32 = item_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

        if !db.user_exists(target) {
            return format!("{}\n赠送对象不存在！", prefix);
        }
        if target == user_id {
            return format!("{}\n不能给自己赠送！", prefix);
        }
        if db.knapsack_quantity(user_id, item_name) < qty {
            return format!("{}\n物品不足！", prefix);
        }

        db.knapsack_remove(user_id, item_name, qty);
        db.knapsack_add(target, item_name, qty);
        crate::collection::record_item_collection(db, target, item_name);
        crate::achievement::on_gift(db, user_id);
        format!("{}\n成功给{}赠送了[{}]×{}", prefix, target, item_name, qty)
    } else if let Some(data) = args.strip_prefix("金币") {
        let parts: Vec<&str> = data.splitn(2, '+').collect();
        if parts.len() < 2 {
            return format!("{}\n格式：赠送金币+目标QQ+数量", prefix);
        }
        let target = parts[0];
        let amount: i64 = parts[1].parse().unwrap_or(0);

        if !db.user_exists(target) {
            return format!("{}\n赠送对象不存在！", prefix);
        }
        if target == user_id {
            return format!("{}\n不能给自己赠送！", prefix);
        }
        if db.read_currency(user_id, CURRENCY_GOLD) < amount {
            return format!("{}\n金币不足！", prefix);
        }

        db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, amount);
        db.modify_currency(target, CURRENCY_GOLD, OP_ADD, amount);
        crate::achievement::on_gift(db, user_id);
        format!("{}\n成功给{}赠送了{}金币", prefix, target, amount)
    } else if let Some(data) = args.strip_prefix("钻石") {
        let parts: Vec<&str> = data.splitn(2, '+').collect();
        if parts.len() < 2 {
            return format!("{}\n格式：赠送钻石+目标QQ+数量", prefix);
        }
        let target = parts[0];
        let amount: i64 = parts[1].parse().unwrap_or(0);

        if !db.user_exists(target) {
            return format!("{}\n赠送对象不存在！", prefix);
        }
        if target == user_id {
            return format!("{}\n不能给自己赠送！", prefix);
        }
        if db.read_currency(user_id, CURRENCY_DIAMOND) < amount {
            return format!("{}\n钻石不足！", prefix);
        }

        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_SUB, amount);
        db.modify_currency(target, CURRENCY_DIAMOND, OP_ADD, amount);
        crate::achievement::on_gift(db, user_id);
        format!("{}\n成功给{}赠送了{}钻石", prefix, target, amount)
    } else {
        format!(
            "{}\n格式：\n赠送物品+目标QQ|物品名*数量\n赠送金币+目标QQ+数量\n赠送钻石+目标QQ+数量",
            prefix
        )
    }
}

/// 帮助中心
pub fn cmd_help_center(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let category = args.trim();

    if category.is_empty() {
        return format!(
            "{}\n\
             ═══ 帮助中心 ═══\n\
             共计257个指令，28个分类\\n\\
             1. 基础系统 - 注册/签到/查看角色/帮助\\n\\
             2. 战斗系统 - 搜索怪物/攻击/技能/自动战斗\\n\\
             3. 地图系统 - 查看地图/进入地图/位置信息/地图资源\\n\\
             4. 商店系统 - 查看商店/购买商品/合成\\n\\
             5. 物品系统 - 背包/使用物品/丢弃/筛选\\n\\
             6. 公会系统 - 创建/退出/捐献/成员管理\\n\\
             7. 任务系统 - 领取/查看/提交/放弃任务\\n\\
             8. PVP系统 - 锁定/攻击玩家/战绩/罪恶值\\n\\
             9. 锻炼系统 - 吐纳/冥想/练武/习法/修炼\\n\\
             10. 生活系统 - 种植/采集/烹饪/制药/熔炼/炼制\\n\\
             11. 社交系统 - 队伍/匹配/喊话/公会邀请\\n\\
             12. 经济系统 - 钱庄/赠送/私人商店/出售\\n\\
             13. 排行系统 - 等级/战力/签到/匹配排行\\n\\
             14. 增益系统 - 查看增益/增益信息/BUFF\\n\\
             15. 称号系统 - 查看称号/装备称号/卸下称号\\n\\
             16. GM系统 - 发放奖励/设置管理员/全服邮件/发布公告/删除公告\\\n\\\
             17. 系统功能 - 自动查看/详细信息/验证信息/护盾/装备图鉴\\n\\
             18. VIP系统 - VIP信息/VIP签到/VIP充值/首冲奖励/累计充值\\n\\
             19. 灵兽系统 - 查看灵兽/捕获灵兽/我的灵兽/灵兽出战/喂食/进化\\n\\
             20. 活跃系统 - 查看活跃/领取活跃/活跃兑换\\\n\\
             21. 兑换码系统 - 生成兑换码/兑换码/兑换码列表
             22. 宝箱系统 - 查看宝箱/开启宝箱/宝箱钥匙\\\\\\\n\\\\\\\\
             23. 收集系统 - 收集图鉴/收集统计\\\\\\\n\\\\
             24. 在线活跃 - 在线排行/在线统计/活跃信息\\\\\\\\\\\\\\\n\\\\\\\\\\\
             25. 探索系统 - 探索进度/探索详情/探索奖励/探索排行\n\\
             26. 坐骑系统 - 查看坐骑/坐骑列表/骑乘/下骑/喂养坐骑/坐骑进化/坐骑排行\\\\\\\\\\\\\\\n\\\\\\\\\\\
                        27. 时装系统 - 查看时装/购买时装/穿戴时装/卸下时装/时装图鉴/时装排行
\
            28. 回收系统 - 查看回收/回收物品/批量回收/回收商店/回收兑换/回收排行/回收统计\n\\
            发送「帮助中心+类别名」查看该类别指令",
            prefix
        );
    }

    let result = match category {
        "基础" | "基础系统" => "═══ 基础系统 ═══\n\
             注册+昵称 - 注册游戏\n\
             签到 - 每日签到（连续奖励+里程碑钻石+道具）\n\
             签到日历 - 查看本月签到记录和统计\n\
             补签列表 - 查看可补签日期和费用\n\
             补签+日期 - 钻石补签漏签日期\n\
             查看角色 - 查看角色信息\n\
             修改昵称+新昵称 - 修改昵称\n\
             帮助 - 显示帮助列表\n\
             帮助中心 - 分类帮助索引\n\
             VIP信息 - 查看VIP状态\n\
             VIP签到 - VIP每日签到\n\
             VIP充值 - VIP充值升级\n\
             首冲奖励 - 查看首冲奖励\n\
             每日任务 - 查看每日任务\n\
             领取每日+任务名 - 领取每日奖励\n\
             周常任务 - 查看周常任务\n\
             领取周常+任务名 - 领取周常奖励\n\
             查看祈福 - 查看今日祈福状态\n\\
             祈福 - 每日祈福获取奖励\n\\
             在线奖励 - 查看在线奖励进度\n\\
             领取在线+礼包名 - 领取在线时间奖励\n\\
             离线收益 - 查看离线期间累积的收益\n\\
             领取离线收益 - 领取金币和经验离线奖励\\n\\
             查看回归奖励 - 查看回归奖励系统\\n\\
             领取回归奖励 - 领取离线回归奖励\\n\\
             回归排行 - 回归勇士排行榜\\n\\
             回归统计 - 全服回归系统统计\\\
             查看熟练度 - 查看技能熟练度总览\n\
             技能熟练度+技能名 - 查看特定技能熟练度\n\
             熟练度排行 - 全服技能熟练度排行\n\
             熟练度加成 - 查看技能伤害加成总览"
            .to_string(),
        "战斗" | "战斗系统" => "═══ 战斗系统 ═══\n\
             搜索怪物 - 搜索并锁定怪物\n\
             锁定目标+怪物名 - 锁定指定怪物\n\
             解锁目标 - 清除当前目标\n\
             攻击 - 攻击当前目标\n\
             自动攻击+技能1,技能2 - 自动战斗\n\
             释放技能+技能名 - 使用技能攻击\n\
             查看战力 - 综合战力评分\n\\
             虚弱状态 - 查看死亡惩罚状态\\n\\
             被动回复 - 查看战斗后自动回血回蓝信息\\n\\
             查看深渊 - 无尽深渊挑战系统\\n\\
             挑战深渊 - 逐层挑战深渊怪物\\n\\
             深渊进度 - 查看深渊挑战进度\\n\\
             深渊排行 - 全服深渊排行榜"
            .to_string(),
        "地图" | "地图系统" => "═══ 地图系统 ═══\n\
             查看地图 - 查看当前位置及可前往方向\n\
             进入+方向/地图名 - 移动到相邻地图\n\
             地图传送 - 查看可传送目的地\n\
             地图传送+地图名 - 传送到指定地图（消耗金币）\n\
             位置信息 - 查看当前位置详细信息\n\
             地图资源 - 查看当前地图资源总览\n\
             地图资源+地图名 - 查看指定地图资源\n\
             查看NPC - 查看当前地图NPC\n\
             对话+NPC名 - 与NPC对话\n\
             使用功能+功能名 - 使用NPC功能"
            .to_string(),
        "商店" | "商店系统" => "═══ 商店系统 ═══\n\
             查看商店+页码 - 查看系统商店\n\
             购买+商品名*数量 - 购买商品\n\
             查看合成+配方名 - 查看合成配方\n\
             合成+物品名 - 合成物品\n\
             合成列表+页码 - 查看合成列表\n\
             商品筛选+类型 - 按类型筛选商品\n\
             查看当前商店 - 查看所有可用商店\n\
             搜索商品+商品名 - 搜索跨商店商品"
            .to_string(),
        "物品" | "物品系统" => "═══ 物品系统 ═══\n\
             查看背包+页码 - 查看背包物品\n\
             使用+物品名*数量 - 使用物品\n\
             丢弃+物品名*数量 - 丢弃物品\n\
             查看物品+物品名 - 查看物品详情\n\
             背包筛选+类型 - 按类型筛选（装备/药剂/材料等）\n\
             查看装备 - 查看装备栏\n\
             卸下+槽位 - 卸下装备\n\
             查看技能 - 查看技能列表\n\
             查看职业 - 查看职业信息\n\
             职业列表 - 查看所有职业\n\
             转换职业+职业名 - 转换职业\n\
             强化装备 - 强化装备\n\
             选择强化+装备名 - 预览强化\n\
             确认强化 - 执行强化\n\
             查看强化信息 - 查看各级强化成功率\n\
             查看保底 - 查看强化保底进度\n\
             选择分解+物品名 - 预览分解\n\
             确认分解 - 执行分解\n\
             鉴定装备+装备名 - 鉴定装备获取变量属性\n\
             查看鉴定 - 查看已鉴定装备\n\
             装备合成配方 - 查看所有合成配方\n\
             合成详情+套装名 - 查看合成所需材料\n\
             装备合成+套装名 - 合成套装装备\n\
             我的合成 - 查看已获得的合成加成\n\
             装备类型 - 查看装备类型加成系统\n\
             装备类型+类型名 - 查看特定类型详情"
            .to_string(),
        "公会" | "公会系统" => "═══ 公会系统 ═══\n\
             创建公会+公会名 - 创建公会\n\
             我的公会 - 查看公会信息\n\
             退出公会 - 退出当前公会\n\
             解散公会 - 解散公会\n\
             转让公会+目标QQ - 转让公会\n\
             公会列表+页码 - 查看公会列表\n\
             公会捐献+数量 - 捐献金币\n\
             公会成员 - 查看成员列表\n\
             踢出成员+目标QQ - 踢出成员\n\
             申请加入公会+公会名 - 申请加入\n\
             批准加入+目标QQ - 批准申请\n\
             拒绝加入+目标QQ - 拒绝申请\n\
             邀请加入+目标QQ - 邀请加入公会\n\
             公会捐赠排行 - 查看捐赠排行\n\
             公会试炼 - 查看公会试炼\n\
             挑战试炼 - 挑战试炼BOSS\n\
             试炼进度 - 查看试炼伤害排名"
            .to_string(),
        "任务" | "任务系统" => "═══ 任务系统 ═══\n\
             我的任务 - 查看任务列表\n\
             任务信息+任务名 - 查看任务详情\n\
             领取任务+任务名 - 领取任务\n\
             提交任务+任务名 - 提交完成的任务\n\
             放弃任务+任务名 - 放弃任务"
            .to_string(),
        "PVP" | "pvp" | "PVP系统" => "═══ PVP系统 ═══\n\
             锁定玩家+目标QQ - 锁定玩家\n\
             攻击玩家 - 攻击锁定的玩家\n\
             查看目标 - 查看当前目标\n\
             邪恶值 - 查看邪恶值\n\
             小黑屋 - 查看小黑屋信息\n\
             我的战绩 - 查看PVP战绩\n\
             罪恶值排行 - 查看罪恶值排行\n\
             被杀记录 - 查看被击杀记录\n\
             段位排行 - 查看赛季段位排行\n\
             查看通缉 - 查看全服通缉榜\n\
             发布通缉+目标+金币+原因 - 发布悬赏通缉\n\
             接受通缉+目标名 - 接受通缉任务\n\
             领取赏金+目标名 - 击败目标后领取赏金\n\\
             通缉详情+目标名 - 查看通缉详细信息\n\\
             通缉排行 - 查看猎人排行\n\\
             我的通缉 - 查看我的通缉相关"
            .to_string(),
        "锻炼" | "锻炼系统" => "═══ 锻炼系统 ═══\n\\
             锻造列表 - 查看可锻炼项目\n\
             吐纳 - 锻炼生命\n\
             冥想 - 锻炼魔法\n\
             练武 - 锻炼物攻\n\
             习法 - 锻炼魔攻\n\
             开始修炼 - 开始AFK挂机修炼\n\
             停止修炼 - 停止AFK挂机修炼\n\
             修炼状态 - 查看修炼进度\n\
             属性重置 - 查看重置属性信息\n\
             确认重置 - 确认重置训练属性\n\
             重置记录 - 查看重置历史记录"
            .to_string(),
        "生活" | "生活系统" => "═══ 生活系统 ═══\n\
             查看种子 - 查看可购买种子\n\
             购买种子+种子名 - 购买种子\n\
             种植+种子名 - 种植到花园\n\
             我的花园 - 查看花园状态\n\
             收获 - 收获药材\n\
             出售药材 - 出售药材\n\
             药园仓库 - 药园仓库\n\
             铲除作物+地块号 - 移除地块上的作物\n\
             转换灵气 - 金币/钻石兑换灵力\n\
             采集信息 - 查看采集信息\n\
             采集+类型 - 执行采集\n\
             采集统计 - 查看采集统计\n\
             副职 - 查看采集技能等级\n\
             查看烹饪 - 查看烹饪配方\n\
             烹饪+配方名 - 烹饪食物\n\
             可烹饪 - 查看可烹饪配方\n\
             查看制药 - 查看制药配方\n\
             制药+药剂名 - 制作药剂\n\
             可制药 - 查看可制药配方\n\
             查看熔炼 - 查看熔炼配方\n\
             熔炼+物品名 - 熔炼物品\n\
             可熔炼 - 查看可熔炼物品
\
             查看炼制 - 查看炼制配方
\
             炼制+配方名 - 炼制物品
\
             可炼制 - 查看可炼制配方\n\
             查看附魔 - 查看附魔配方\n\
             附魔+附魔名+装备名 - 为装备附魔\n\
             可附魔 - 查看可附魔装备\n\\
             探索进度 - 查看地图探索进度\n\\
             探索详情+地图名 - 查看单张地图探索详情\n\\
             探索奖励+地图名 - 领取探索里程碑奖励\n\\
             探索排行 - 全服探索进度排行"
            .to_string(),
        "坐骑" | "坐骑系统" => "═══ 坐骑系统 ═══\n\\
             查看坐骑 - 查看当前坐骑状态\n\\
             坐骑列表 - 查看所有坐骑图鉴\n\\
             骑乘 - 装备当前坐骑获得属性加成\n\\
             下骑 - 取消骑乘状态\n\\
             喂养坐骑+食物名 - 喂养坐骑提升经验\n\\
             坐骑进化 - 进化坐骑到更高品质\n\\
             坐骑排行 - 全服坐骑等级排行"
            .to_string(),
        "社交" | "社交系统" => "═══ 社交系统 ═══\n\
             创建队伍+队伍名 - 创建队伍\n\
             加入队伍+队伍名 - 加入队伍\n\
             退出队伍 - 退出当前队伍\n\
             队伍成员 - 查看队伍成员\n\
             请出成员+玩家ID - 请出队伍成员（队长专用）\n\
             匹配 - 匹配竞技\\n\\
             匹配信息 - 查看匹配信息\\n\\
             匹配排行 - 查看匹配排行\\n\\
             加入匹配队列 - 加入PVP匹配队列\\n\\
             退出匹配队列 - 退出匹配队列\\n\\
             匹配队列 - 查看当前匹配队列\\n\\
             喊话+内容 - 全服喊话\\n\\
             位置玩家 - 查看当前地图玩家\n\
             添加好友+玩家ID - 添加好友\n\
             删除好友+玩家ID - 删除好友\n\
             好友列表 - 查看好友列表\n\
             查看好友+玩家ID - 查看好友详情\n\
             对比+玩家昵称 - 与玩家对比属性\n\
             赠送物品+目标QQ|物品名*数量\n\
             赠送金币+目标QQ+数量\n\
             赠送钻石+目标QQ+数量\n\
             祈福排行 - 查看祈福排行榜"
            .to_string(),
        "经济" | "经济系统" => "═══ 经济系统 ═══\n\
             查看钱庄 - 查看钱庄信息\n\
             存款+数量/全部 - 存入金币\n\
             取款+数量/全部 - 取出金币\n\
             我的存款 - 查看存款详情\n\
             商店列表 - 查看私人商店\n\
             进入商店+店名 - 进入私人商店\n\
             我的商店 - 管理我的商店\n\
             上架商品+物品名*数量+价格\n\
             下架商品+物品名\n\
             出售+物品名*数量 - 出售给NPC\n\
             出售价格+物品名 - 查看出售价格\n\
             可出售 - 查看可出售物品\n\
             收购列表 - 查看NPC收购列表\n\
             夜市 - 查看夜市商品\n\
             查看抽奖 - 查看抽奖系统\n\
             抽奖+池名 - 单次抽奖\n\
             十连抽+池名 - 十连抽\n\
             保底信息 - 查看抽奖保底进度\\n\\
             查看折扣 - 查看每日限时折扣\\n\\
             购买折扣+商品名 - 购买折扣商品\\n\\
             折扣统计 - 查看折扣购物统计"
            .to_string(),
        "排行" | "排行系统" => "═══ 排行系统 ═══\n\
             等级排行 - 等级排行榜\n\
             生命排行 - 生命排行榜\n\
             魔法排行 - 魔法排行榜\n\
             物攻排行 - 物攻排行榜\n\
             魔攻排行 - 魔攻排行榜\n\
             防御排行 - 防御排行榜\n\
             魔抗排行 - 魔抗排行榜\n\
             金币排行 - 金币排行榜\n\
             钻石排行 - 钻石排行榜\n\
             战力排行 - 战力排行榜\n\
             签到排行 - 签到排行榜\n\
             匹配排行 - 匹配竞技排行\n\
             段位排行 - 赛季段位排行\n\
             罪恶值排行 - 罪恶值排行\n\
             公会捐赠排行 - 公会捐赠排行\n\
             成就排行 - 成就完成排行"
            .to_string(),
        "增益" | "增益系统" | "BUFF" | "buff" => "═══ 增益系统 ═══\n\
             查看增益 - 查看所有活跃增益/减益\n\
             增益信息 - 查看增益详细属性影响"
            .to_string(),
        "GM" | "gm" | "GM系统" => "═══ GM系统 ═══\n\
             发放奖励+QQ+类型+数量 - 发放奖励\n\
             设置管理员 - 设置GM权限\n\
             设置权限+目标+等级 - 设置权限等级(99/98/31)\n\
             查看权限 - 查看权限等级\n\
             权限列表 - 查看所有管理员\n\
             全服邮件+标题+内容 - 发送全服邮件\n\
             系统邮件+标题+内容 - 发送系统邮件\n\
             生成兑换码+类型+数量+使用次数 - 生成兑换码\n\
             兑换码列表 - 查看所有兑换码\n\
             属性调整+玩家ID+属性+数值 - 调整玩家属性\n\
             系统属性 - 查看系统全局属性\n\
             查看变量 - 查看系统变量面板\n\
             变量详情+变量名 - 查看变量详情\n\
             设置变量+变量名+值 - 修改系统变量(GM)"
            .to_string(),
        "系统" | "系统功能" => "═══ 系统功能 ═══\n\
             自动查看 - 快捷角色概览\n\
             查看详细信息 - 完整属性面板(含穿透/吸血/免伤)\n\
             获取验证信息 - 查看系统状态\n\
             查看护盾 - 查看护盾状态\n\
             获取护盾 - 购买护盾\n\
             装备图鉴 - 浏览装备百科\n\
             查看超界 - 查看超界强化\n\
             超界强化 - 执行超界强化\n\
             超界图鉴 - 查看超界装备隐藏名称\n\
             查看装备技能 - 查看装备附带技能\n\
             查看持续技能 - 查看DOT/增益技能\n\
             查看食物 - 查看食物列表\n\
             购买食物+食物名 - 购买食物\n\
             使用食物+食物名 - 使用食物\n\
             我的食物 - 查看食物背包\n\
             怪物图鉴 - 浏览怪物图鉴\n\
             查看属性克制 - 查看属性克制关系\n\
             查看增益 - 查看增益效果\n\
             成就列表 - 查看成就\n\
             我的成就 - 查看我的成就\n\
             领取成就+成就名 - 领取成就奖励\n\
             成就排行 - 成就完成排名\n\
             全服统计 - 全服综合统计\n\
             系统基础属性 - 查看系统基础属性\\n\\\
             查看征途 - 查看赛季征途进度\\n\\\
             征途详情+章节号 - 查看章节具体目标\\n\\\
             领取征途奖励+章节号 - 领取章节完成奖励\\n\\\
             征途排行 - 全服征途进度排行\\n\\\
             征途统计 - 个人征途数据概览"
            .to_string(),
        "宝石" | "宝石系统" => "═══ 宝石系统 ═══\n\
             查看宝石 - 查看宝石类型和属性\n\
             宝石背包 - 查看拥有的宝石\n\
             镶嵌宝石+宝石名+槽位 - 镶嵌宝石到装备\n\
             卸下宝石+槽位 - 卸下宝石\n\
             合成宝石+宝石名 - 3个同级合成1个高级\n\
             宝石属性 - 查看镶嵌宝石总加成"
            .to_string(),
        "VIP" | "vip" | "VIP系统" => "═══ VIP系统 ═══\n\
             VIP信息 - 查看VIP等级/积分/到期时间\n\
             VIP签到 - 每日VIP签到获取积分\n\
             VIP充值 - VIP充值升级\n\
             首冲奖励 - 查看首次充值奖励状态"
            .to_string(),
        "灵兽" | "灵兽系统" => "═══ 灵兽系统 ═══\n\
             查看灵兽 - 查看当前地图可捕获的灵兽\n\
             捕获灵兽+怪物名 - 捕获灵兽\n\
             我的灵兽 - 查看已拥有的灵兽\n\
             灵兽出战+灵兽名 - 设置出战灵兽\n\
             灵兽喂食+灵兽名 - 喂养灵兽增加忠诚度\n\
             灵兽进化+灵兽名 - 进化灵兽\n\
             灵兽图鉴 - 查看灵兽图鉴\n\
             放生灵兽+灵兽名 - 放生灵兽"
            .to_string(),
        "活跃" | "活跃系统" => "═══ 活跃系统 ═══\n\
             查看活跃 - 查看今日活跃度和奖励进度\n\
             领取活跃+分数 - 领取活跃度里程碑奖励\n\
             活跃兑换 - 使用活跃积分兑换道具\n\
             活跃度 - 查看每日活跃度积分详情\n\
             领取活跃奖励 - 领取活跃度里程碑奖励\n\
             活跃排行 - 今日活跃度排行榜\n\
             完成各种日常活动(签到/战斗/任务/采集等)累积积分\n\
             5个里程碑: 30/60/100/150/200点，奖励递增"
            .to_string(),
        "兑换码" | "兑换码系统" => "═══ 兑换码系统 ═══\n\
             兑换码+码 - 使用兑换码兑换奖励\n\
             生成兑换码+类型+数量+次数 - (GM)生成兑换码\n\
             兑换码列表 - (GM)查看所有兑换码\n\
             类型: 金币/钻石/物品名"
            .to_string(),
        "宝箱" | "宝箱系统" => "═══ 宝箱系统 ═══\n\
             查看宝箱 - 查看宝箱商店和奖励预览\n\
             开启宝箱+宝箱名 - 使用钥匙开启宝箱\n\
             宝箱钥匙 - 购买钥匙\n\
             宝箱: 铜/银/金/至尊"
            .to_string(),
        "收集" | "收集系统" => "═══ 收集系统 ═══\n\
             收集图鉴 - 查看收集进度和里程碑\n\
             收集图鉴+物品 - 查看已收集物品\n\
             收集图鉴+怪物 - 查看已发现怪物\n\
             收集图鉴+领取 - 领取里程碑奖励\n\
             收集统计 - 快速查看收集进度\n\
             收集度越高奖励越丰厚！"
            .to_string(),
        "在线活跃" | "在线统计" | "活跃统计" => "═══ 在线活跃系统 ═══\n\
             在线排行 - 查看在线时长排行榜\n\
             活跃排行 - 在线时长排行榜（别名）\n\
             在线统计 - 查看全局在线数据+我的排名\n\
             活跃信息 - 查看个人活跃详情\n\
             活跃信息+玩家ID - 查看指定玩家活跃\n\n\
             💡 每次使用游戏指令自动累积1分钟在线时长\n\
             活跃等级：挂机萌新→休闲玩家→活跃冒险者→资深勇者→肝帝→传说肝王→永不停歇"
            .to_string(),
        "时装" | "时装系统" => "═══ 时装收集系统 ═══\n\
             查看时装 - 查看时装商店\n\
             购买时装+编号 - 购买时装\n\
             穿戴时装+编号 - 装备时装\n\
             卸下时装 - 取下时装\n\
             时装图鉴 - 查看收集进度和套装奖励\n\
             时装排行 - 全服时装收集排行\n\n\
             💡 集齐同品质时装可激活套装加成(2%%~20%%)"
            .to_string(),
        "回收" | "回收系统" => "═══ 资源回收系统 ═══\n\
             查看回收 - 查看回收系统概览和可回收物品\n\
             回收物品+物品名 - 回收指定物品获得积分\n\
             批量回收+品质 - 一键回收指定品质物品\n\
             回收商店 - 浏览积分兑换商店\n\
             回收兑换+物品名 - 用积分兑换道具\n\
             回收排行 - 全服回收积分排行\n\
             回收统计 - 个人回收历史统计\n\n\
             ♻️ 回收物品获得积分，积分兑换珍贵道具！"
            .to_string(),
        _ => {
            format!(
"未找到分类 [{}]\n可用分类：基础/战斗/地图/商店/物品/公会/任务/PVP/锻炼/生活/社交/经济/排行/增益/GM/系统/VIP/灵兽/活跃/兑换码/宝箱/收集/在线活跃/探索/坐骑/时装/回收",
                category
            )
        }
    };

    format!("{}\n{}", prefix, result)
}

/// 战力评分
pub fn cmd_combat_power(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);

    // 11属性加权公式
    let power = info.hp_max as f64 * 0.5
        + info.mp_max as f64 * 0.3
        + info.ad as f64 * 2.0
        + info.ap as f64 * 2.0
        + info.defense as f64 * 1.5
        + info.magic_res as f64 * 1.5
        + info.hit as f64 * 1.0
        + info.dodge as f64 * 1.0
        + info.crit as f64 * 1.0
        + info.absorb_hp as f64 * 0.5
        + info.shield as f64 * 0.3;

    let power_val = power as i64;

    // 8段段位
    let (rank_name, next_threshold) = if power_val >= 100000 {
        ("🏆 传说", 0)
    } else if power_val >= 50000 {
        ("💎 神话", 100000)
    } else if power_val >= 30000 {
        ("🥇 史诗", 50000)
    } else if power_val >= 15000 {
        ("🥈 精英", 30000)
    } else if power_val >= 8000 {
        ("🥉 高级", 15000)
    } else if power_val >= 4000 {
        ("⚔️ 中级", 8000)
    } else if power_val >= 1500 {
        ("🛡️ 初级", 4000)
    } else {
        ("🌱 新手", 1500)
    };

    let mut result = format!(
        "{}\n\
         ═══ 战力评分 ═══\n\
         综合战力：{}\n\
         段位：{}",
        prefix, power_val, rank_name
    );

    if next_threshold > 0 {
        let progress = ((power_val as f64 / next_threshold as f64) * 100.0).min(100.0);
        result.push_str(&format!("\n升级进度：{:.1}%", progress));
    }

    result.push_str("\n\n属性构成：");
    result.push_str(&format!("\n  生命+{:.0}(50%)", info.hp_max as f64 * 0.5));
    result.push_str(&format!("\n  魔法+{:.0}(30%)", info.mp_max as f64 * 0.3));
    result.push_str(&format!("\n  物攻+{:.0}(200%)", info.ad as f64 * 2.0));
    result.push_str(&format!("\n  魔攻+{:.0}(200%)", info.ap as f64 * 2.0));
    result.push_str(&format!("\n  防御+{:.0}(150%)", info.defense as f64 * 1.5));
    result.push_str(&format!("\n  魔抗+{:.0}(150%)", info.magic_res as f64 * 1.5));
    result.push_str(&format!("\n  命中+{:.0}(100%)", info.hit as f64 * 1.0));
    result.push_str(&format!("\n  闪避+{:.0}(100%)", info.dodge as f64 * 1.0));
    result.push_str(&format!("\n  暴击+{:.0}(100%)", info.crit as f64 * 1.0));

    result
}

/// 战力排行榜
pub fn cmd_power_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 获取所有用户ID
    let user_ids: Vec<String> = {
        let conn = db.lock_conn();
        let mut stmt = match conn.prepare("SELECT DISTINCT ID FROM Basic_User WHERE Node=?1") {
            Ok(s) => s,
            Err(e) => return format!("{}\n查询失败：{}", prefix, e),
        };
        let mut result = Vec::new();
        if let Ok(rows) = stmt.query_map(rusqlite::params![NODE_BASIC], |row| row.get::<_, String>(0)) {
            for r in rows.flatten() {
                result.push(r);
            }
        }
        result
    };

    let mut powers: Vec<(String, String, i64)> = Vec::new();
    for uid in &user_ids {
        if !db.user_exists(uid) {
            continue;
        }
        let info = user::calc_total_attrs(db, uid);
        let power = (info.hp_max as f64 * 0.5
            + info.mp_max as f64 * 0.3
            + info.ad as f64 * 2.0
            + info.ap as f64 * 2.0
            + info.defense as f64 * 1.5
            + info.magic_res as f64 * 1.5
            + info.hit as f64 * 1.0
            + info.dodge as f64 * 1.0
            + info.crit as f64 * 1.0
            + info.absorb_hp as f64 * 0.5
            + info.shield as f64 * 0.3) as i64;
        powers.push((uid.clone(), info.name, power));
    }

    powers.sort_by_key(|b| std::cmp::Reverse(b.2));
    powers.truncate(10);

    if powers.is_empty() {
        return format!("{}\n暂无排行数据！", prefix);
    }

    let mut result = format!("{}\n═══ 战力排行榜 ═══", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, name, power)) in powers.iter().enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let display_name = if name.is_empty() { uid.as_str() } else { name.as_str() };
        result.push_str(&format!("\n{}. {} {} - 战力：{}", i + 1, medal, display_name, power));
    }

    result
}

/// 查看护盾
pub fn cmd_view_shield(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    match db.get_user_shield(user_id) {
        Some((value, get_time, duration, pattern)) => {
            let total_count = db.get_shield_count();
            format!(
                "{}\n\
                 ═══ 护盾信息 ═══\n\
                 护盾值：{}\n\
                 获取时间：{}\n\
                 持续天数：{}\n\
                 模式：{}\n\
                 全服护盾记录：{}",
                prefix, value, get_time, duration, pattern, total_count
            )
        }
        None => {
            format!(
                "{}\n\
                 您当前没有护盾！\n\
                 发送「获取护盾」购买护盾",
                prefix
            )
        }
    }
}

/// 签到排行榜
pub fn cmd_sign_in_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 获取所有有签到数据的用户
    let records: Vec<(String, String, i32)> = {
        let conn = db.lock_conn();
        let mut stmt =
            match conn.prepare("SELECT ID, Item, Data FROM Basic_User WHERE Node=?1 AND (Item=?2 OR Item=?3)") {
                Ok(s) => s,
                Err(e) => return format!("{}\n查询失败：{}", prefix, e),
            };
        let mut result = Vec::new();
        if let Ok(rows) = stmt.query_map(
            rusqlite::params![NODE_USER_DATA, "sign_in_sustain", "sign_in_total"],
            |row| {
                Ok((
                    row.get::<_, String>(0).unwrap_or_default(),
                    row.get::<_, String>(1).unwrap_or_default(),
                    row.get::<_, String>(2).unwrap_or_default(),
                ))
            },
        ) {
            for r in rows.flatten() {
                let uid = r.0.trim_end_matches('\x00').to_string();
                let val: i32 = r.2.parse().unwrap_or(0);
                result.push((uid, r.1, val));
            }
        }
        result
    };

    // 按用户聚合：取连续天数和累计天数
    let mut user_map: std::collections::HashMap<String, (i32, i32)> = std::collections::HashMap::new();
    for (uid, item, val) in &records {
        let entry = user_map.entry(uid.clone()).or_insert((0, 0));
        if item == "sign_in_sustain" {
            entry.0 = *val;
        }
        if item == "sign_in_total" {
            entry.1 = *val;
        }
    }

    let mut ranking: Vec<(String, String, i32, i32)> = Vec::new();
    for (uid, (sustain, total)) in &user_map {
        let name = db.read_basic(uid, ITEM_NAME);
        ranking.push((uid.clone(), name, *sustain, *total));
    }

    // 按累计天数降序排序
    ranking.sort_by(|a, b| b.3.cmp(&a.3).then(b.2.cmp(&a.2)));
    ranking.truncate(10);

    if ranking.is_empty() {
        return format!("{}\n暂无签到数据！", prefix);
    }

    let mut result = format!("{}\n═══ 签到排行榜 ═══", prefix);
    let medals = ["🥇", "🥈", "🥉"];
    for (i, (uid, name, sustain, total)) in ranking.iter().enumerate() {
        let medal = if i < 3 { medals[i] } else { "  " };
        let display_name = if name.is_empty() { uid.as_str() } else { name.as_str() };
        result.push_str(&format!(
            "\n{}. {} {} - 累计：{}天 连续：{}天",
            i + 1,
            medal,
            display_name,
            total,
            sustain
        ));
    }

    result
}

/// 签到日历 - 显示本月签到记录和统计
pub fn cmd_sign_calendar(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 支持查看指定月份: 签到日历+2026-06 或默认当前月
    let now = chrono::Local::now();
    let target_month = if !args.is_empty() && args.len() >= 7 {
        args[..7].to_string()
    } else {
        now.format("%Y-%m").to_string()
    };

    // 读取签到数据
    let cal_key = format!("sign_cal_{}", target_month);
    let cal_data = db.read_user_data(user_id, &cal_key);
    let signed_days: std::collections::HashSet<i32> = if cal_data.is_empty() {
        std::collections::HashSet::new()
    } else {
        cal_data
            .split(',')
            .filter_map(|d| d.trim().parse::<i32>().ok())
            .collect()
    };

    let sign_sustain: i32 = db.read_user_data(user_id, "sign_in_sustain").parse().unwrap_or(0);
    let sign_total: i32 = db.read_user_data(user_id, "sign_in_total").parse().unwrap_or(0);

    // 计算月份天数
    let year: i32 = target_month[..4].parse().unwrap_or(2026);
    let month: i32 = target_month[5..7].parse().unwrap_or(1);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    };

    let mut result = format!("{}\n═══ 签到日历 {}年{}月 ═══", prefix, year, month);
    result.push_str(&format!("\n连续签到: {}天 | 累计签到: {}天", sign_sustain, sign_total));
    result.push_str("\n\n日 一 二 三 四 五 六");
    result.push_str("\n─────────────────────");

    // 计算月份第一天是星期几（蔡勒公式简化版）
    let y = if month <= 2 { year - 1 } else { year };
    let m = if month <= 2 { month + 12 } else { month };
    let weekday = (1 + (13 * (m + 1)) / 5 + y + y / 4 - y / 100 + y / 400) % 7;
    // 蔡勒公式: 0=Saturday, convert to 0=Sunday
    let first_day_col: usize = ((weekday + 6) % 7) as usize; // 0=Sunday

    let mut line = String::new();
    for _ in 0..first_day_col {
        line.push_str("   ");
    }
    for day in 1..=days_in_month {
        let day_str = if signed_days.contains(&day) {
            "✅".to_string()
        } else if day < 10 {
            format!("{:2} ", day)
        } else {
            format!("{} ", day)
        };
        line.push_str(&day_str);

        let col = (first_day_col + day as usize - 1) % 7;
        if col == 6 || day == days_in_month {
            result.push_str(&format!("\n{}", line));
            line.clear();
        }
    }

    // 月度统计
    let signed_count = signed_days.len();
    let rate = if days_in_month > 0 {
        signed_count as f64 / days_in_month as f64 * 100.0
    } else {
        0.0
    };
    result.push_str(&format!(
        "\n\n📊 本月签到: {}/{}天 ({:.0}%)",
        signed_count, days_in_month, rate
    ));

    // 里程碑提示
    if sign_sustain >= 30 {
        result.push_str("\n🏆 达成30天连续签到！至尊签到者！");
    } else if sign_sustain >= 14 {
        result.push_str("\n⭐ 达成14天连续签到！坚持签到！");
    } else if sign_sustain >= 7 {
        result.push_str("\n✨ 达成7天连续签到！继续保持！");
    }

    result.push_str("\n\n发送'签到'进行每日签到 | 发送'补签列表'查看补签");
    result
}

/// 自动查看（快捷角色概览）
pub fn cmd_auto_view_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);

    format!(
        "{}\n\
         ═══ 角色概览 ═══\n\
         {} | Lv.{} {}\n\
         ❤️ {}/{}\n\
         💧 {}/{}\n\
         ⚔️ 物攻：{}  魔攻：{}\n\
         🛡️ 防御：{}  魔抗：{}\n\
         🎯 命中：{}  闪避：{}\n\
         💥 暴击：{}\n\
         💰 金币：{}  💎 钻石：{}\n\
         ✨ 经验：{}/{}\n\
         📍 {}",
        prefix,
        info.name,
        info.level,
        info.occupation,
        info.hp,
        info.hp_max,
        info.mp,
        info.mp_max,
        info.ad,
        info.ap,
        info.defense,
        info.magic_res,
        info.hit,
        info.dodge,
        info.crit,
        info.gold,
        info.diamond,
        info.exp,
        info.exp_need,
        info.location,
    )
}

/// 查看详细信息（基于 CustomVariable 模板的完整属性面板）
pub fn cmd_view_detailed_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let info = user::calc_total_attrs(db, user_id);

    // 战力计算（与 cmd_combat_power 一致）
    let power = (info.ad as i64 * 5000
        + info.defense as i64 * 30
        + info.magic_res as i64 * 30
        + info.crit as i64 * 1000
        + info.hit as i64 * 2
        + info.dodge as i64 * 2
        + info.hp_max as i64 * 50
        + info.mp_max as i64 * 10
        + info.ap as i64 * 5000
        + info.absorb_hp as i64 * 100
        + info.immune as i64 * 200
        + info.ad_ptv as i64 * 50
        + info.ap_ptv as i64 * 50
        + info.ad_ptr as i64 * 100000
        + info.ap_ptr as i64 * 100000)
        / 1000;

    let mut lines = vec![
        format!("{}\n", prefix),
        "═══ 详细信息 ═══\n".to_string(),
        format!("职业：{}  Lv.{}\n", info.occupation, info.level),
        format!("经验：{}/{}\n", info.exp, info.exp_need),
        format!("战力：{}\n", power),
        format!("❤️ 生命：{}/{}\n", info.hp, info.hp_max),
        format!("💧 魔法：{}/{}\n", info.mp, info.mp_max),
    ];

    // 物攻/魔攻（仅显示非零）
    if info.ad > 0 {
        lines.push(format!("⚔️ 物攻：{}\n", info.ad));
    }
    if info.ap > 0 {
        lines.push(format!("🔮 魔攻：{}\n", info.ap));
    }

    lines.push(format!("🛡️ 防御：{}  魔抗：{}\n", info.defense, info.magic_res));

    // 穿透
    if info.ad_ptv > 0 || info.ad_ptr > 0 {
        lines.push(format!("🗡️ 物穿：{}|{}%\n", info.ad_ptv, info.ad_ptr));
    }
    if info.ap_ptv > 0 || info.ap_ptr > 0 {
        lines.push(format!("🌀 法穿：{}|{}%\n", info.ap_ptv, info.ap_ptr));
    }

    // 命中/闪避/暴击（仅显示非零）
    if info.hit > 0 {
        lines.push(format!("🎯 命中：{}\n", info.hit));
    }
    if info.dodge > 0 {
        lines.push(format!("💨 闪避：{}\n", info.dodge));
    }
    if info.crit > 0 {
        lines.push(format!("💥 暴击：{}\n", info.crit));
    }

    // 吸血/免伤（仅显示非零）
    if info.absorb_hp > 0 {
        lines.push(format!("🧛 吸血：{}%\n", info.absorb_hp));
    }
    if info.immune > 0 {
        lines.push(format!("🔰 免伤：{}%\n", info.immune));
    }

    // 护盾
    if info.shield > 0 {
        lines.push(format!("🛡️ 护盾：{}\n", info.shield));
    }

    lines.push(format!("💰 金币：{}  💎 钻石：{}\n", info.gold, info.diamond));
    lines.push(format!("📍 位置：{}", info.location));

    lines.join("")
}

/// 获取护盾（购买护盾）
pub fn cmd_get_shield(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let choice = args.trim();

    if choice.is_empty() {
        return format!(
            "{}\n\
             ═══ 获取护盾 ═══\n\
             1. 初级护盾 - 100金币（护盾值500，持续3天）\n\
             2. 中级护盾 - 300金币（护盾值1500，持续5天）\n\
             3. 高级护盾 - 800金币（护盾值5000，持续7天）\n\
             4. 至尊护盾 - 2000金币（护盾值15000，持续15天）\n\n\
             发送「获取护盾+序号」购买",
            prefix
        );
    }

    let (cost, shield_val, duration, name) = match choice {
        "1" | "初级护盾" => (100i64, 500, 3, "初级护盾"),
        "2" | "中级护盾" => (300i64, 1500, 5, "中级护盾"),
        "3" | "高级护盾" => (800i64, 5000, 7, "高级护盾"),
        "4" | "至尊护盾" => (2000i64, 15000, 15, "至尊护盾"),
        _ => return format!("{}\n请输入正确的护盾等级（1-4）！", prefix),
    };

    let gold = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < cost {
        return format!("{}\n金币不足！需要{}金币，当前{}金币", prefix, cost, gold);
    }

    db.modify_currency(user_id, CURRENCY_GOLD, OP_SUB, cost);
    db.set_user_shield(user_id, shield_val, duration, name);

    format!(
        "{}\n\
         购买成功！\n\
         获得 [{}]\n\
         护盾值：{}\n\
         持续时间：{}天\n\
         消耗金币：{}",
        prefix, name, shield_val, duration, cost
    )
}

/// 获取验证信息（系统状态）
pub fn cmd_get_verify_info(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    // 统计注册玩家数
    let user_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row(
            "SELECT COUNT(DISTINCT ID) FROM Basic_User WHERE Node=?1",
            rusqlite::params![NODE_BASIC],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };

    // 统计物品种类数
    let item_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Goods", [], |row| row.get(0))
            .unwrap_or(0)
    };

    // 统计地图数
    let map_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Map", [], |row| row.get(0))
            .unwrap_or(0)
    };

    // 统计怪物数
    let monster_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Monster", [], |row| row.get(0))
            .unwrap_or(0)
    };

    // 统计技能数
    let skill_count: i32 = {
        let conn = db.lock_conn();
        conn.query_row("SELECT COUNT(*) FROM Config_Skills", [], |row| row.get(0))
            .unwrap_or(0)
    };

    format!(
        "{}\n\
         ═══ 系统验证信息 ═══\n\
         注册玩家：{}\n\
         物品种类：{}\n\
         地图数量：{}\n\
         怪物种类：{}\n\
         技能数量：{}\n\
         \n\
         引擎版本：CakeGame-Rust v2\n\
         指令总数：192\n\
         数据状态：正常",
        prefix, user_count, item_count, map_count, monster_count, skill_count
    )
}

/// 装备图鉴
pub fn cmd_equipment_codex(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let query = args.trim();

    if query.is_empty() {
        // 概览模式：按品质/部位统计
        let equip_count: i32 = {
            let conn = db.lock_conn();
            conn.query_row("SELECT COUNT(*) FROM Config_Goods WHERE Type='Equip'", [], |row| {
                row.get(0)
            })
            .unwrap_or(0)
        };

        let slots = [
            "武器", "头盔", "铠甲", "护腿", "靴子", "项链", "戒指", "翅膀", "时装", "称号",
        ];
        let mut result = format!(
            "{}\n\
             ═══ 装备图鉴 ═══\n\
             装备总数：{}件\n\
             支持3种查询模式：\n\
             1. 装备图鉴 - 概览统计\n\
             2. 装备图鉴+部位名 - 筛选（如：武器/头盔/铠甲）\n\
             3. 装备图鉴+装备名 - 搜索详情\n\n\
             按部位统计：",
            prefix, equip_count
        );

        for slot in &slots {
            let count: i32 = {
                let conn = db.lock_conn();
                conn.query_row(
                    "SELECT COUNT(*) FROM Config_Goods WHERE Type='Equip' AND LtemData LIKE ?1",
                    [format!("%{}%", slot).as_str()],
                    |row| row.get(0),
                )
                .unwrap_or(0)
            };
            if count > 0 {
                result.push_str(&format!("\n  {}：{}件", slot, count));
            }
        }

        return result;
    }

    // 检查是否是品质名筛选
    let quality_names = ["普通", "劣质", "精良", "优秀", "稀有", "史诗", "传说"];
    if quality_names.contains(&query) {
        let equips: Vec<String> = {
            let conn = db.lock_conn();
            let mut stmt = match conn.prepare("SELECT Name FROM Config_Goods WHERE Type='Equip' AND Name LIKE ?1") {
                Ok(s) => s,
                Err(e) => return format!("{}\n查询失败：{}", prefix, e),
            };
            let mut result = Vec::new();
            if let Ok(rows) = stmt.query_map(rusqlite::params![format!("%【{}】%", query)], |row| {
                row.get::<_, String>(0)
            }) {
                for r in rows.flatten() {
                    result.push(r);
                }
            }
            result
        };

        if equips.is_empty() {
            return format!("{}\n未找到 [{}] 品质的装备", prefix, query);
        }

        let mut result = format!("{}\n═══ {}品质装备 ({}) ═══", prefix, query, equips.len());
        for (i, name) in equips.iter().enumerate() {
            result.push_str(&format!("\n{}. {}", i + 1, name));
        }
        result.push_str("\n\n发送「装备图鉴+装备名」查看详情");
        return result;
    }

    // 检查是否是部位名筛选
    let slot_names = [
        "武器", "头盔", "铠甲", "护腿", "靴子", "项链", "戒指", "翅膀", "时装", "称号",
    ];
    if slot_names.contains(&query) {
        let equips: Vec<String> = {
            let conn = db.lock_conn();
            let mut stmt = match conn.prepare("SELECT Name FROM Config_Goods WHERE Type='Equip' AND LtemData LIKE ?1") {
                Ok(s) => s,
                Err(e) => return format!("{}\n查询失败：{}", prefix, e),
            };
            let mut result = Vec::new();
            if let Ok(rows) = stmt.query_map(rusqlite::params![format!("%{}%", query)], |row| row.get::<_, String>(0)) {
                for r in rows.flatten() {
                    result.push(r);
                }
            }
            result
        };

        if equips.is_empty() {
            return format!("{}\n未找到 [{}] 类型的装备", prefix, query);
        }

        let mut result = format!("{}\n═══ {}类装备 ({}) ═══", prefix, query, equips.len());
        for (i, name) in equips.iter().enumerate() {
            result.push_str(&format!("\n{}. {}", i + 1, name));
        }
        result.push_str("\n\n发送「装备图鉴+装备名」查看详情");
        return result;
    }

    // 搜索模式：按装备名查看详情
    match db.item_get(query) {
        Some(item) => {
            let mut result = format!("{}\n═══ {} ═══", prefix, item.name);
            result.push_str(&format!("\n类型：{}", item.item_type));
            if !item.data.slot_name.is_empty() {
                result.push_str(&format!("\n部位：{}", item.data.slot_name));
            }
            if !item.introduce.is_empty() && item.introduce != "[NULL]" {
                result.push_str(&format!("\n介绍：{}", item.introduce));
            }
            if item.data.use_lv > 0 {
                result.push_str(&format!("\n需求等级：{}", item.data.use_lv));
            }
            if item.data.add_hp > 0 {
                result.push_str(&format!("\n生命+{}", item.data.add_hp));
            }
            if item.data.add_mp > 0 {
                result.push_str(&format!("\n魔法+{}", item.data.add_mp));
            }
            if item.data.add_ad > 0 {
                result.push_str(&format!("\n物攻+{}", item.data.add_ad));
            }
            if item.data.add_ap > 0 {
                result.push_str(&format!("\n魔攻+{}", item.data.add_ap));
            }
            if item.data.add_defense > 0 {
                result.push_str(&format!("\n防御+{}", item.data.add_defense));
            }
            if item.data.add_magic > 0 {
                result.push_str(&format!("\n魔抗+{}", item.data.add_magic));
            }
            if item.data.add_hit > 0 {
                result.push_str(&format!("\n命中+{}", item.data.add_hit));
            }
            if item.data.add_dodge > 0 {
                result.push_str(&format!("\n闪避+{}", item.data.add_dodge));
            }
            if item.data.add_crit > 0 {
                result.push_str(&format!("\n暴击+{}", item.data.add_crit));
            }

            // 查询掉落来源
            let monster_drops: Vec<String> = {
                let conn = db.lock_conn();
                let mut stmt = match conn.prepare("SELECT Name FROM Config_Monster WHERE RewardGoods LIKE ?1") {
                    Ok(s) => s,
                    Err(_) => return result,
                };
                let mut drops = Vec::new();
                if let Ok(rows) = stmt.query_map(rusqlite::params![format!("%{}%", item.name)], |row| {
                    row.get::<_, String>(0)
                }) {
                    for r in rows.flatten() {
                        drops.push(r);
                    }
                }
                drops
            };

            if !monster_drops.is_empty() {
                result.push_str(&format!("\n\n掉落来源：{}", monster_drops.join("、")));
            }

            result
        }
        None => format!("{}\n未找到装备 [{}]", prefix, query),
    }
}

/// 全服公告系统
/// 发布公告 (GM)
pub fn cmd_post_announcement(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power: i32 = db.global_get("Permissions", user_id).parse().unwrap_or(0);
    if power < 100 {
        return format!("{}\n你无权操作，需要管理员权限。", prefix);
    }

    let content = args.trim();
    if content.is_empty() {
        return format!("{}\n格式：发布公告+公告内容", prefix);
    }

    // 获取当前公告计数
    let count_str = db.global_get("Announcement", "count");
    let count: i32 = count_str.parse().unwrap_or(0);
    let new_id = count + 1;

    // 存储公告
    let key = format!("msg_{}", new_id);
    let name = db.read_basic(user_id, "Name");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let data = format!("{}|{}|{}|{}", new_id, name, timestamp, content);
    db.global_set("Announcement", &key, &data);
    db.global_set("Announcement", "count", &new_id.to_string());

    format!(
        "{}\n📢 公告发布成功！\n公告编号：{}\n内容：{}\n\n使用「删除公告+编号」可删除此公告",
        prefix, new_id, content
    )
}

/// 查看公告
pub fn cmd_view_announcements(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let count_str = db.global_get("Announcement", "count");
    let count: i32 = count_str.parse().unwrap_or(0);

    if count == 0 {
        return format!(
            "{}\n═══ 全服公告 ═══\n\n暂无公告\n\n💡 管理员发送「发布公告+内容」发布新公告",
            prefix
        );
    }

    let mut result = format!("{}\n═══ 全服公告 ═══", prefix);

    // 显示最近10条公告
    let start = std::cmp::max(1, count - 9);
    let mut shown = 0;
    for i in (start..=count).rev() {
        let key = format!("msg_{}", i);
        let data = db.global_get("Announcement", &key);
        if data.is_empty() {
            continue;
        }
        let parts: Vec<&str> = data.splitn(4, '|').collect();
        if parts.len() >= 4 {
            let msg_id = parts[0];
            let author = parts[1];
            let ts: u64 = parts[2].parse().unwrap_or(0);
            let content = parts[3];

            // 格式化时间
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let diff = now.saturating_sub(ts);
            let time_str = if diff < 60 {
                "刚刚".to_string()
            } else if diff < 3600 {
                format!("{}分钟前", diff / 60)
            } else if diff < 86400 {
                format!("{}小时前", diff / 3600)
            } else {
                format!("{}天前", diff / 86400)
            };

            result.push_str(&format!(
                "\n📢 #{} {} ({})\n   {}\n   发布者：{}",
                msg_id,
                time_str,
                ts_to_date(ts),
                content,
                author
            ));
            shown += 1;
        }
    }

    if shown == 0 {
        result.push_str("\n\n暂无有效公告");
    } else {
        result.push_str(&format!("\n\n共 {} 条公告", count));
    }

    result
}

/// 删除公告 (GM)
pub fn cmd_delete_announcement(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let power: i32 = db.global_get("Permissions", user_id).parse().unwrap_or(0);
    if power < 100 {
        return format!("{}\n你无权操作，需要管理员权限。", prefix);
    }

    let msg_id: i32 = match args.trim().parse() {
        Ok(id) => id,
        Err(_) => return format!("{}\n格式：删除公告+公告编号", prefix),
    };

    let key = format!("msg_{}", msg_id);
    let data = db.global_get("Announcement", &key);
    if data.is_empty() {
        return format!("{}\n公告 #{} 不存在。", prefix, msg_id);
    }

    // 清空公告内容（保留结构，只清除内容）
    db.global_set("Announcement", &key, "");

    format!("{}\n🗑️ 公告 #{} 已删除。", prefix, msg_id)
}

/// 辅助函数：Unix时间戳转日期
fn ts_to_date(ts: u64) -> String {
    // 简化版日期转换
    let _secs = ts % 60;
    let mins = (ts / 60) % 60;
    let hours = (ts / 3600) % 24;
    // 从2024-01-01开始计算天数
    let days = ts / 86400;
    let year = 2024 + days / 365;
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, month, day, hours, mins)
}

/// 设置管理员（bootstrap GM权限）
pub fn cmd_set_admin(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    let current_power: i32 = db.global_get("Permissions", user_id).parse().unwrap_or(0);
    if current_power >= 100 {
        return format!("{}\n您已经是管理员了！", prefix);
    }

    db.global_set("Permissions", user_id, "100");

    format!("{}\n设置成功！您已成为管理员（GM权限已授予）", prefix)
}

/// 商品筛选
pub fn cmd_filter_shop_goods(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    crate::shop::filter_shop_goods(db, user_id, args.trim())
}

/// 查看属性克制
pub fn cmd_view_type_effect(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let monster_name = args.trim();

    if monster_name.is_empty() {
        return format!("{}\n请输入怪物名称！格式：查看属性克制+怪物名", prefix);
    }

    let te = crate::type_effect::calc_type_effectiveness(db, user_id, monster_name);
    let hint = crate::type_effect::format_type_hint(&te);
    format!("{}\n{}", prefix, hint)
}

/// 地图传送 - 消耗金币直接传送到任意地图
pub fn cmd_map_teleport(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return "请先注册后再传送！\n发送【注册+昵称】进行注册".to_string();
    }

    // 检查是否阵亡
    let hp: i64 = db.read_basic(user_id, ITEM_HP).parse().unwrap_or(0);
    if hp <= 0 {
        return format!("{}\n您已阵亡，请先复活后再传送！", prefix);
    }

    let target = args.trim();
    if target.is_empty() {
        // 列出所有可传送的地图及费用
        let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
        let current = db.read_basic(user_id, ITEM_LOCATION);

        let mut result = format!("{}\n═══ 地图传送 ═══\n当前位置：{}", prefix, current);
        result.push_str("\n\n可传送目的地：\n");

        let all_maps = db.map_get_all();
        let mut map_list: Vec<(String, i32, i64)> = Vec::new();
        for (name, map) in &all_maps {
            if *name == current {
                continue;
            }
            let cost = calc_teleport_cost(map.level, level);
            map_list.push((name.clone(), map.level, cost));
        }
        map_list.sort_by_key(|m| m.1);

        for (i, (name, lvl, cost)) in map_list.iter().enumerate() {
            let lvl_str = if *lvl > 0 {
                format!("(Lv{})", lvl)
            } else {
                String::new()
            };
            result.push_str(&format!("  {}. {} {} - {}金币\n", i + 1, name, lvl_str, cost));
        }

        result.push_str("\nTip:发送 地图传送+地图名 即可传送到指定地图");
        return result;
    }

    // 检查目标地图是否存在
    let map = db.map_get(target);
    if map.is_none() {
        return format!("{}\n地图 [{}] 不存在！", prefix, target);
    }
    let map = map.unwrap();

    let current = db.read_basic(user_id, ITEM_LOCATION);
    if current == target {
        return format!("{}\n您已经在 [{}] 了！", prefix, target);
    }

    // 等级检查
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if map.level > 0 && level < map.level {
        return format!(
            "{}\n传送失败！[{}] 需要等级{}，您当前等级{}",
            prefix, target, map.level, level
        );
    }

    // 计算传送费用
    let mut cost = calc_teleport_cost(map.level, level);

    // VIP折扣
    let vip_level: i32 = db.read_user_data(user_id, "vip_level").parse().unwrap_or(0);
    let discount_text = if vip_level >= 5 {
        cost = cost * 50 / 100;
        "（VIP5半价优惠）"
    } else if vip_level >= 3 {
        cost = cost * 80 / 100;
        "（VIP3八折优惠）"
    } else {
        ""
    };

    let gold: i64 = db.read_currency(user_id, CURRENCY_GOLD);
    if gold < cost {
        return format!(
            "{}\n金币不足！传送到 [{}] 需要{}金币，您只有{}金币",
            prefix, target, cost, gold
        );
    }

    // 扣除金币并传送
    db.write_currency(user_id, CURRENCY_GOLD, gold - cost);
    db.write_basic(user_id, ITEM_LOCATION, target);
    // 传送后清除战斗目标
    db.write_basic(user_id, ITEM_TARGET, EMPTY);

    // 记录已访问地图
    let visited = db.read_user_data(user_id, "visited_maps");
    if !visited.contains(target) {
        let new_visited = if visited.is_empty() {
            target.to_string()
        } else {
            format!("{},{}", visited, target)
        };
        db.write_user_data(user_id, "visited_maps", &new_visited);
    }

    format!(
        "{}\n✨ 传送成功！您已到达 [{}]\n消耗{}金币{}",
        prefix, target, cost, discount_text
    )
}

/// 全服统计 - 显示服务器综合统计数据
pub fn cmd_server_stats(db: &Database, _user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("📊 ═══ 全服统计 ═══".to_string());

    // 1. 总注册玩家数
    let all_users = db.all_users();
    let total_players = all_users.len();
    lines.push(format!("👥 注册玩家: {} 人", total_players));

    // 2. 等级统计
    let mut max_level = 0i32;
    let mut max_level_name = String::new();
    let mut total_levels: i64 = 0;
    for uid in &all_users {
        let lvl_str = db.read_basic(uid, ITEM_LEVEL);
        let lvl: i32 = lvl_str.parse().unwrap_or(1);
        total_levels += lvl as i64;
        if lvl > max_level {
            max_level = lvl;
            max_level_name = db.read_basic(uid, ITEM_NAME);
            if max_level_name.is_empty() {
                max_level_name = uid.clone();
            }
        }
    }
    let avg_level = if total_players > 0 {
        total_levels / total_players as i64
    } else {
        0
    };
    lines.push(format!("⭐ 最高等级: Lv.{} ({})", max_level, max_level_name));
    lines.push(format!("📈 平均等级: Lv.{}", avg_level));

    // 3. 货币统计
    let mut total_gold: i64 = 0;
    let mut total_diamond: i64 = 0;
    let mut richest_name = String::new();
    let mut richest_gold: i64 = 0;
    for uid in &all_users {
        let g = db.read_currency(uid, CURRENCY_GOLD);
        let d = db.read_currency(uid, CURRENCY_DIAMOND);
        total_gold += g;
        total_diamond += d;
        if g > richest_gold {
            richest_gold = g;
            richest_name = db.read_basic(uid, ITEM_NAME);
            if richest_name.is_empty() {
                richest_name = uid.clone();
            }
        }
    }
    lines.push(format!("💰 全服金币: {}", crate::vip::format_gold(total_gold)));
    lines.push(format!("💎 全服钻石: {}", crate::vip::format_gold(total_diamond)));
    lines.push(format!(
        "🏆 最富玩家: {} ({}金币)",
        richest_name,
        crate::vip::format_gold(richest_gold)
    ));

    // 4. 公会统计
    let conn = db.lock_conn();
    let guild_count: i32 = {
        let mut stmt = match conn.prepare("SELECT COUNT(*) FROM Config_Union") {
            Ok(s) => s,
            Err(_) => return "❌ 数据库查询失败".to_string(),
        };
        stmt.query_row([], |row| row.get(0)).unwrap_or(0)
    };
    drop(conn);
    lines.push(format!("🏰 公会总数: {}", guild_count));

    // 5. 背包物品统计
    let mut total_items: i64 = 0;
    for uid in &all_users {
        let items = db.knapsack_all(uid);
        for item in &items {
            total_items += item.quantity as i64;
        }
    }
    lines.push(format!("🎒 全服物品: {} 件", total_items));

    // 6. 当前用户排名
    let user_level: i32 = db.read_basic(_user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let mut rank = 1;
    for uid in &all_users {
        let lvl: i32 = db.read_basic(uid, ITEM_LEVEL).parse().unwrap_or(1);
        if lvl > user_level {
            rank += 1;
        }
    }
    lines.push(format!("\n📊 您的等级排名: {}/{}", rank, total_players));

    // 7. 持续效果统计
    let (total_effects, dot_count, buff_count) = crate::equip_skill::count_active_effects(db);
    if total_effects > 0 {
        lines.push(format!(
            "\n🔄 活跃持续效果: {} (DOT:{}, 增益:{})",
            total_effects, dot_count, buff_count
        ));
    }

    lines.join("\n")
}

/// 计算传送费用: 基础费用100 + 目标地图等级要求×30
fn calc_teleport_cost(map_level: i32, _player_level: i32) -> i64 {
    let base: i64 = 100;
    let level_cost: i64 = (map_level as i64).max(1) * 30;
    (base + level_cost).max(100)
}

/// 查看系统基础属性（system_uAttributes 表）
/// 显示所有角色共享的系统级基础属性值
pub fn cmd_view_system_attrs(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let sys = db.get_system_base_attrs();
    // (HP, MP, AD, AP, Defense, Hit, Dodge, Crit, MagicResistance, AbsorbHP, ImmuneDamage, ADPTV, ADPTR, APPTV, APPTR)
    let attrs = [
        ("生命", sys.0),
        ("魔法", sys.1),
        ("物攻", sys.2),
        ("魔攻", sys.3),
        ("防御", sys.4),
        ("命中", sys.5),
        ("闪避", sys.6),
        ("暴击", sys.7),
        ("魔抗", sys.8),
        ("吸血", sys.9),
        ("免疫伤害", sys.10),
        ("物攻穿透", sys.11),
        ("物攻减免", sys.12),
        ("魔攻穿透", sys.13),
        ("魔攻减免", sys.14),
    ];

    let mut result = format!("{}\n═══ 系统基础属性 ═══\n所有角色共享的系统级基础属性：\n", prefix);
    for (name, val) in &attrs {
        if *val > 0 {
            result.push_str(&format!("  {}: +{}\n", name, val));
        }
    }
    result.push_str("\n💡 系统基础属性自动叠加到角色属性上\n无需手动操作，注册即生效");
    result
}

// ═══════════════════════════════════════════════════════
// 以下10个函数修复编译错误 (2026-06-11)
// ═══════════════════════════════════════════════════════

/// 数据库驱动帮助系统 — 查询 Config_Help 表
/// 精确匹配 → 模糊匹配 → 列表全部
pub fn cmd_db_help(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let query = args.trim();
    let conn = db.lock_conn();

    if query.is_empty() {
        // List all help topics
        let mut stmt = match conn.prepare("SELECT Help FROM Config_Help") {
            Ok(s) => s,
            Err(_) => return format!("{}\n❌ 帮助数据加载失败", prefix),
        };
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        if names.is_empty() {
            return format!("{}\n暂无帮助数据。", prefix);
        }
        let mut out = format!("{}\n═══ 帮助主题 ═══\n", prefix);
        for (i, name) in names.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, name));
        }
        out.push_str("\n发送「帮助+主题名」查看详情");
        return out;
    }

    // Try exact match first
    let mut stmt = match conn.prepare("SELECT HelpData FROM Config_Help WHERE Help=?1") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 帮助数据加载失败", prefix),
    };
    if let Ok(mut rows) = stmt.query_map([query], |row| row.get::<_, String>(0)) {
        if let Some(Ok(data)) = rows.next() {
            return format!("{}\n═══ {} ═══\n{}", prefix, query, data);
        }
    }

    // Fuzzy match
    let pattern = format!("%{}%", query);
    let mut stmt = match conn.prepare("SELECT Help, HelpData FROM Config_Help WHERE Help LIKE ?1") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 帮助查询失败", prefix),
    };
    let matches: Vec<(String, String)> = stmt
        .query_map([pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    if matches.len() == 1 {
        format!("{}\n═══ {} ═══\n{}", prefix, matches[0].0, matches[0].1)
    } else if matches.is_empty() {
        format!("{}\n未找到与 [{}] 相关的帮助主题。", prefix, query)
    } else {
        let mut out = format!("{}\n找到 {} 个匹配主题：\n", prefix, matches.len());
        for (name, _) in &matches {
            out.push_str(&format!("  • {}\n", name));
        }
        out.push_str("\n请输入更精确的主题名查看详情。");
        out
    }
}

/// 背包整理 — 按类型优先级排序物品
pub fn cmd_sort_knapsack(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let items = db.knapsack_all(user_id);
    if items.is_empty() {
        return format!("{}\n📦 背包为空，无需整理。", prefix);
    }

    // Sort by type priority then name
    let mut sorted = items;
    sorted.sort_by(|a, b| {
        let pa = type_priority(&a.name, db);
        let pb = type_priority(&b.name, db);
        pa.cmp(&pb).then_with(|| a.name.cmp(&b.name))
    });

    // Rewrite knapsack with sorted order
    let conn = db.lock_conn();
    let _ = conn.execute("DELETE FROM Basic_knapsack WHERE User=?1", [user_id]);
    for item in &sorted {
        let _ = conn.execute(
            "INSERT INTO Basic_knapsack (User, Name, Count) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, item.name, item.quantity],
        );
    }

    let mut category_counts: std::collections::BTreeMap<&str, (i32, i32)> = std::collections::BTreeMap::new();
    for item in &sorted {
        let cat = type_category(&item.name, db);
        let entry = category_counts.entry(cat).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += item.quantity;
    }

    let mut out = format!("{}\n📦 背包整理完成！\n", prefix);
    for (cat, (count, qty)) in &category_counts {
        out.push_str(&format!("  {} {}种 × {}\n", cat, count, qty));
    }
    out.push_str(&format!(
        "\n共 {} 件物品",
        sorted.iter().map(|i| i.quantity).sum::<i32>()
    ));
    out
}

fn type_priority(name: &str, db: &Database) -> i32 {
    if let Some(item) = db.item_get(name) {
        return match item.item_type.as_str() {
            "药剂" | "药品" | "消耗品" => 0,
            "材料" | "素材" => 1,
            "礼包" | "宝箱" => 2,
            "货币袋" => 3,
            "武器" | "防具" | "饰品" | "装备" => 4,
            _ => 5,
        };
    }
    5
}

fn type_category(name: &str, db: &Database) -> &'static str {
    if let Some(item) = db.item_get(name) {
        return match item.item_type.as_str() {
            "药剂" | "药品" | "消耗品" => "🧪药剂",
            "材料" | "素材" => "🪨材料",
            "礼包" | "宝箱" => "🎁礼包",
            "货币袋" => "💰货币袋",
            "武器" | "防具" | "饰品" | "装备" => "⚔️装备",
            _ => "📦其他",
        };
    }
    "📦其他"
}

/// 推荐打怪 — 基于玩家等级推荐合适的怪物
pub fn cmd_recommend_monster(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let user_hp: i32 = db.read_basic(user_id, "生命").parse().unwrap_or(100);
    let user_def: i32 = db.read_basic(user_id, "防御").parse().unwrap_or(10);

    let mut maps = db.map_get_all();
    maps.sort_by(|a, b| a.0.cmp(&b.0));

    let mut recommendations: Vec<(String, String, i32, String)> = Vec::new();

    for (map_name, map_def) in &maps {
        if map_def.level > level {
            continue;
        }
        for monster_name in &map_def.monsters {
            if let Some(monster) = db.monster_get(monster_name) {
                let threat = if monster.defense > 0 {
                    (monster.hp as f64 / user_hp.max(1) as f64) * (monster.ad as f64 / user_def.max(1) as f64)
                } else {
                    monster.hp as f64 / user_hp.max(1) as f64
                };

                let (icon, diff) = if threat < 1.0 {
                    ("🟢", "轻松击杀")
                } else if threat < 3.0 {
                    ("🟡", "势均力敌")
                } else {
                    ("🔴", "极度危险")
                };

                recommendations.push((
                    monster_name.clone(),
                    map_name.clone(),
                    monster.hp,
                    format!("{}{}", icon, diff),
                ));
            }
        }
    }

    recommendations.sort_by_key(|a| a.2);

    let mut out = format!("{}\n═══ 推荐打怪 (等级{}) ═══\n", prefix, level);
    for (i, (monster, map, hp, diff)) in recommendations.iter().take(15).enumerate() {
        out.push_str(&format!(
            "{}. {} {} (HP:{}) 📍{} — {}\n",
            i + 1,
            diff,
            monster,
            hp,
            map,
            diff
        ));
    }
    if recommendations.is_empty() {
        out.push_str("暂无推荐怪物。");
    }
    out
}

/// 怪物弱点分析
pub fn cmd_monster_weakness(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let query = args.trim();
    if query.is_empty() {
        return format!("{}\n请输入怪物名称，如: 怪物弱点 哥布林", prefix);
    }

    let monster = match db.monster_get(query) {
        Some(m) => m,
        None => return format!("{}\n未找到怪物 [{}]", prefix, query),
    };

    let weaknesses: Vec<(&str, i32)> = vec![
        ("物防", monster.defense),
        ("魔抗", monster.magic_resistance),
        ("命中", monster.hit),
        ("闪避", monster.dodge),
        ("穿透", monster.adptv),
    ];

    let weakest = weaknesses.iter().min_by_key(|(_, v)| *v).unwrap();

    let mut out = format!("{}\n═══ 怪物弱点: {} ═══\n", prefix, monster.name);
    out.push_str(&format!(
        "HP: {} | AD: {} | AP: {}\n",
        monster.hp, monster.ad, monster.ap
    ));
    out.push_str(&format!(
        "防御: {} | 魔抗: {} | 命中: {} | 闪避: {} | 暴击: {}\n",
        monster.defense, monster.magic_resistance, monster.hit, monster.dodge, monster.adptv
    ));
    out.push_str(&format!("\n🎯 最弱属性: {} ({})\n", weakest.0, weakest.1));

    if let Some(te) = Some(crate::type_effect::calc_type_effectiveness(db, user_id, query)) {
        out.push_str(&format!("{}\n", crate::type_effect::format_type_hint(&te)));
    }
    out
}

/// 查询掉落 — 查询怪物掉落物品
pub fn cmd_query_drops(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let query = args.trim();
    if query.is_empty() {
        return format!("{}\n请输入物品名或怪物名，如: 查询掉落 哥布林", prefix);
    }

    // Search by monster name
    if let Some(monster) = db.monster_get(query) {
        let mut out = format!("{}\n═══ {} 掉落物 ═══\n", prefix, monster.name);
        if monster.reward_goods.is_empty() {
            out.push_str("该怪物无掉落。");
        } else {
            for (i, reward) in monster.reward_goods.iter().enumerate() {
                out.push_str(&format!(
                    "{}. {} ×{} (概率: {:.1}%)\n",
                    i + 1,
                    reward.name,
                    reward.count,
                    reward.rate * 100.0
                ));
            }
        }
        return out;
    }

    // Search by item name across all monsters
    let conn = db.lock_conn();
    let mut stmt = match conn.prepare("SELECT Name, Reward_Goods FROM Config_Monster WHERE Reward_Goods LIKE ?1") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };
    let pattern = format!("%{}%", query);
    let results: Vec<(String, String)> = stmt
        .query_map([pattern], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    if results.is_empty() {
        return format!("{}\n未找到包含 [{}] 的掉落数据。", prefix, query);
    }

    let mut out = format!("{}\n═══ {} 掉落来源 ═══\n", prefix, query);
    for (monster_name, _) in &results {
        out.push_str(&format!("  • {}\n", monster_name));
    }
    out
}

/// 类型图鉴 — 委托给 type_effect 模块
pub fn cmd_type_chart(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let body = crate::type_effect::format_type_chart_overview(db, user_id);
    format!("{}\n{}", prefix, body)
}

/// 职业类型 — 委托给 type_effect 模块
pub fn cmd_occupation_types(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let body = crate::type_effect::format_occupation_types(db);
    format!("{}\n{}", prefix, body)
}

/// 查看模板 — 查看 MessageTemplate 表 (GM功能)
pub fn cmd_view_templates(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let query = args.trim();
    let conn = db.lock_conn();

    if query.is_empty() {
        // List all templates
        let mut stmt = match conn.prepare("SELECT Name FROM MessageTemplate ORDER BY Name") {
            Ok(s) => s,
            Err(_) => return format!("{}\n❌ 模板数据加载失败", prefix),
        };
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        let mut out = format!("{}\n═══ 消息模板 ({}) ═══\n", prefix, names.len());
        for (i, name) in names.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, name));
        }
        out.push_str("\n发送「查看模板+模板名」查看详情");
        return out;
    }

    // Try exact match
    let mut stmt = match conn.prepare("SELECT Name, Data FROM MessageTemplate WHERE Name=?1") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };
    if let Ok(mut rows) = stmt.query_map([query], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
        if let Some(Ok((name, data))) = rows.next() {
            return format!("{}\n═══ 模板: {} ═══\n{}", prefix, name, data);
        }
    }

    // Fuzzy match
    let pattern = format!("%{}%", query);
    let mut stmt = match conn.prepare("SELECT Name FROM MessageTemplate WHERE Name LIKE ?1") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };
    let matches: Vec<String> = stmt
        .query_map([pattern], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    if matches.is_empty() {
        format!("{}\n未找到模板 [{}]", prefix, query)
    } else {
        let mut out = format!("{}\n找到 {} 个匹配模板：\n", prefix, matches.len());
        for name in &matches {
            out.push_str(&format!("  • {}\n", name));
        }
        out.push_str("\n请输入完整模板名查看详情。");
        out
    }
}

/// 强化排行 — 从 Equip_Register 计算属性加成分数排名
pub fn cmd_enhance_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    let mut stmt = match conn.prepare(
        "SELECT User, EquipName, COALESCE(Add_HP,0)+COALESCE(Add_MP,0)+COALESCE(Add_Defense,0)+COALESCE(Add_Magic,0)+COALESCE(Add_AD,0)+COALESCE(Add_AP,0) as total FROM Equip_Register ORDER BY total DESC LIMIT 20"
    ) {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };

    #[derive(Debug)]
    struct EnhanceEntry {
        user: String,
        equip: String,
        score: i64,
    }

    let entries: Vec<EnhanceEntry> = stmt
        .query_map([], |row| {
            Ok(EnhanceEntry {
                user: row.get(0)?,
                equip: row.get(1)?,
                score: row.get(2)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    if entries.is_empty() {
        return format!("{}\n暂无强化数据。", prefix);
    }

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = format!("{}\n═══ 强化排行榜 ═══\n", prefix);
    for (i, e) in entries.iter().enumerate().take(10) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(&e.user, ITEM_NAME);
        let display_name = if name.is_empty() { e.user.clone() } else { name };
        out.push_str(&format!(
            "{}{}. {} — {} (加成: {})\n",
            medal,
            i + 1,
            display_name,
            e.equip,
            e.score
        ));
    }
    out
}

/// 装备评分排行 — 从背包装备计算评分排名
pub fn cmd_equip_score_ranking(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let conn = db.lock_conn();

    // Get all users' equipped items and score them
    let mut stmt = match conn.prepare("SELECT User, EquipName FROM Equip_Register") {
        Ok(s) => s,
        Err(_) => return format!("{}\n❌ 查询失败", prefix),
    };

    let all_equips: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    // Score each user's total equipment
    use std::collections::BTreeMap;
    let mut user_scores: BTreeMap<String, (f64, i32)> = BTreeMap::new();

    for (uid, equip_name) in &all_equips {
        if let Some(item) = db.item_get(equip_name) {
            let score = crate::equip_score::calc_equip_score(&item.data);
            let entry = user_scores.entry(uid.clone()).or_insert((0.0, 0));
            entry.0 += score;
            entry.1 += 1;
        }
    }

    let mut ranked: Vec<(String, f64, i32)> = user_scores
        .into_iter()
        .map(|(uid, (score, count))| (uid, score, count))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if ranked.is_empty() {
        return format!("{}\n暂无装备评分数据。", prefix);
    }

    let medals = ["🥇", "🥈", "🥉"];
    let mut out = format!("{}\n═══ 装备评分排行榜 ═══\n", prefix);
    for (i, (uid, score, count)) in ranked.iter().enumerate().take(10) {
        let medal = if i < 3 { medals[i] } else { "  " };
        let name = db.read_basic(uid, ITEM_NAME);
        let display_name = if name.is_empty() { uid.clone() } else { name };
        let stars = if *score > 500.0 {
            "⭐⭐⭐⭐⭐"
        } else if *score > 300.0 {
            "⭐⭐⭐⭐"
        } else if *score > 150.0 {
            "⭐⭐⭐"
        } else if *score > 50.0 {
            "⭐⭐"
        } else {
            "⭐"
        };
        out.push_str(&format!(
            "{}{}. {} — {:.0}分 {} ({}件)\n",
            medal,
            i + 1,
            display_name,
            score,
            stars,
            count
        ));
    }
    out
}

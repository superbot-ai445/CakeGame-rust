/// CakeGame 任务系统
/// 实现领取/查看/提交/放弃任务功能
use crate::core::*;
use crate::db::Database;
use crate::encoding;
use crate::user;
use chrono::Local;

/// 解析任务数据中的目标配置
/// 格式: [Target]\n怪物名=数量 或 [Goods]\n物品名=数量
fn parse_task_targets(data: &str) -> Vec<(String, i32)> {
    let mut targets = Vec::new();
    let lines: Vec<&str> = data.lines().collect();
    let mut in_section = false;

    for line in lines {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_section = line == "[Target]" || line == "[Goods]";
            continue;
        }
        if in_section {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let name = parts[0].trim().to_string();
                let count: i32 = parts[1].trim().parse().unwrap_or(0);
                if !name.is_empty() && count > 0 {
                    targets.push((name, count));
                }
            }
        }
    }
    targets
}

/// 解析任务奖励物品
/// 格式: 物品名*数量,物品名*数量
#[allow(dead_code)]
fn parse_reward_items(s: &str) -> Vec<RewardItem> {
    if s.is_empty() {
        return vec![];
    }
    s.split(',')
        .filter_map(|part| {
            let parts: Vec<&str> = part.trim().split('*').collect();
            if parts.len() >= 2 {
                Some(RewardItem {
                    name: parts[0].to_string(),
                    count: parts[1].parse().unwrap_or(1),
                    rate: 100.0,
                })
            } else {
                None
            }
        })
        .collect()
}

/// 全部任务 - 查看游戏内所有任务（不受等级/职业/前置限制）
pub fn cmd_all_tasks(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let page: i32 = args.trim().parse().unwrap_or(1).max(1);

    // 获取所有已完成的任务
    let mut completed_tasks: Vec<String> = Vec::new();
    {
        let conn = db.lock_conn();
        let mut stmt = conn
            .prepare("SELECT TaskName FROM Task_Register WHERE User=?1 AND Complete=?2")
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params![user_id, "TRUE"], |row| row.get(0))
            .unwrap();
        for name in rows.flatten() {
            completed_tasks.push(name);
        }
    }

    // 获取当前进行中的任务
    let current_task = db.read_basic(user_id, ITEM_TASK);

    // 获取所有任务
    let all_task_titles = db.task_get_all_titles();

    if all_task_titles.is_empty() {
        return format!("{}\n暂无任务数据。", prefix);
    }

    // 任务类型图标
    fn task_type_icon(t: &str) -> &'static str {
        match t {
            "主线" => "📜",
            "支线" => "📋",
            "每日" => "🔄",
            "活动" => "🎉",
            _ => "📝",
        }
    }

    // 任务状态
    fn task_status_label(completed: bool, is_current: bool, available: bool) -> &'static str {
        if completed {
            "✅已完成"
        } else if is_current {
            "⚔️进行中"
        } else if available {
            "🟢可接取"
        } else {
            "🔒未解锁"
        }
    }

    // 构建任务列表
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    let occupation = db.read_basic(user_id, ITEM_OCCUPATION);
    let mut entries: Vec<(String, String)> = Vec::new();

    for title in &all_task_titles {
        if let Some(task) = db.task_get(title) {
            let is_completed = completed_tasks.contains(title);
            let is_current = current_task == *title;

            let occ_ok = task.occupation.is_empty() || task.occupation == "[NULL]" || task.occupation == occupation;
            let level_ok = task.level_min <= 0 || level >= task.level_min;
            let max_ok = task.level_max <= 0 || level <= task.level_max;
            let prereq_ok = task.complete_task.is_empty()
                || task.complete_task == "[NULL]"
                || completed_tasks.contains(&task.complete_task);
            let available = !is_completed && !is_current && occ_ok && level_ok && max_ok && prereq_ok;

            let icon = task_type_icon(&task.task_type);
            let status = task_status_label(is_completed, is_current, available);
            let decoded_title = encoding::smart_decode(title);

            let mut rewards = Vec::new();
            if task.reward_gold > 0 {
                rewards.push(format!("{}金", task.reward_gold));
            }
            if task.reward_diamond > 0 {
                rewards.push(format!("{}钻", task.reward_diamond));
            }
            if task.reward_exp > 0 {
                rewards.push(format!("{}经验", task.reward_exp));
            }
            let reward_str = if rewards.is_empty() {
                String::new()
            } else {
                format!(" 💰{}", rewards.join("/"))
            };

            let lv_str = if task.level_max > 0 {
                format!("Lv.{}-{}", task.level_min, task.level_max)
            } else if task.level_min > 0 {
                format!("Lv.{}+", task.level_min)
            } else {
                String::new()
            };

            let prereq_str = if !task.complete_task.is_empty() && task.complete_task != "[NULL]" {
                format!(" →{}", encoding::smart_decode(&task.complete_task))
            } else {
                String::new()
            };

            let display = format!(
                "{}[{}] {} {}{}{}{}",
                icon,
                encoding::smart_decode(&task.task_type),
                decoded_title,
                status,
                if lv_str.is_empty() {
                    String::new()
                } else {
                    format!(" {}", lv_str)
                },
                reward_str,
                prereq_str
            );

            let type_order = match task.task_type.as_str() {
                "主线" => 0,
                "支线" => 1,
                "每日" => 2,
                "活动" => 3,
                _ => 4,
            };
            let sort_key = format!("{}_{:04}", type_order, task.level_min);
            entries.push((sort_key, display));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let total = entries.len();
    let done_count = completed_tasks.len();

    let page_size: usize = 8;
    let total_pages = ((total as i32) + page_size as i32 - 1) / page_size as i32;
    let page = page.min(total_pages.max(1));
    let start = ((page - 1) * page_size as i32) as usize;
    let end = (start + page_size).min(total);

    let mut result = format!(
        "{}\n📋【全部任务】{}/{}页\n完成进度：{}/{} ({}%)",
        prefix,
        page,
        total_pages,
        done_count,
        total,
        done_count.saturating_mul(100).checked_div(total).unwrap_or(0)
    );

    for (i, (_, display)) in entries[start..end].iter().enumerate() {
        result.push_str(&format!("\n{}. {}", start + i + 1, display));
    }

    result.push_str("\n\n说明：✅已完成 ⚔️进行中 🟢可接取 🔒未解锁");
    result.push_str("\n领取任务：发送'领取任务+任务名称'");
    if page < total_pages {
        result.push_str(&format!("\n下一页：全部任务+{}", page + 1));
    }

    result
}

/// 我的任务 - 查看可接任务列表
pub fn cmd_my_tasks(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let page: i32 = args.trim().parse().unwrap_or(1).max(1);

    // 获取用户当前进行中的任务
    let _current_task = db.read_basic(user_id, ITEM_TASK);
    let mut task_list: Vec<String> = Vec::new();

    // 获取所有可接任务
    let available = db.task_available(user_id);

    // 过滤掉已接但未完成的任务
    for title in &available {
        if let Some(record) = db.task_get_record(user_id, title) {
            if !record.complete {
                // 进行中，仍然显示
                task_list.push(format!("[进行中] {}", encoding::smart_decode(title)));
            }
        } else {
            task_list.push(encoding::smart_decode(title));
        }
    }

    if task_list.is_empty() {
        return format!("{}\n您已完成全部任务！", prefix);
    }

    // 分页
    let page_size = 5;
    let total_pages = ((task_list.len() as i32) + page_size - 1) / page_size;
    let page = page.min(total_pages);
    let start = ((page - 1) * page_size) as usize;
    let end = (start + page_size as usize).min(task_list.len());

    let mut result = format!("{}\n【我的任务】第{}/{}页", prefix, page, total_pages);
    for (i, task) in task_list[start..end].iter().enumerate() {
        result.push_str(&format!("\n{}. {}", start + i + 1, task));
    }
    result.push_str("\n\n查看任务详情：发送'查看任务+任务名称'即可");
    if page < total_pages {
        result.push_str(&format!("\n下一页：我的任务+{}", page + 1));
    }

    // 记录当前查看的列表
    db.write_user_data(user_id, "system.see", "我的任务");

    result
}

/// 任务信息 - 查看任务详情
pub fn cmd_task_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let task_name = args.trim();
    let current_task = if task_name.is_empty() {
        // 查看当前任务
        db.read_basic(user_id, ITEM_TASK)
    } else {
        // 支持通过序号查看
        if let Ok(idx) = task_name.parse::<usize>() {
            if idx > 0 {
                let available = db.task_available(user_id);
                if let Some(t) = available.get(idx - 1) {
                    t.clone()
                } else {
                    return format!("{}\n指定任务不存在。", prefix);
                }
            } else {
                return format!("{}\n指定任务不存在。", prefix);
            }
        } else {
            task_name.to_string()
        }
    };

    if current_task.is_empty() {
        return format!(
            "{}\n您目前还未领取任务。\nTip:发送'领取任务+任务名称'即可领取任务。",
            prefix
        );
    }

    let task = match db.task_get(&current_task) {
        Some(t) => t,
        None => return format!("{}\n指定任务不存在。", prefix),
    };

    let mut result = format!(
        "{}\n[{}]\n类型：{}\n简介：{}",
        prefix,
        encoding::smart_decode(&task.title),
        encoding::smart_decode(&task.task_type),
        encoding::smart_decode(&task.info)
    );

    // 显示奖励
    if task.reward_gold > 0 {
        result.push_str(&format!("\n奖励金币：{}", task.reward_gold));
    }
    if task.reward_diamond > 0 {
        result.push_str(&format!("\n奖励钻石：{}", task.reward_diamond));
    }
    if task.reward_exp > 0 {
        result.push_str(&format!("\n获得经验：{}", task.reward_exp));
    }

    // 如果是当前任务，显示进度
    let current = db.read_basic(user_id, ITEM_TASK);
    if current == current_task {
        if let Some(record) = db.task_get_record(user_id, &current_task) {
            let targets = parse_task_targets(&record.data);
            if !targets.is_empty() {
                result.push_str("\n\n任务进度");
                for (i, (name, remaining)) in targets.iter().enumerate() {
                    match task.target_type.as_str() {
                        "Goods" => {
                            // 物品任务：需要收集的数量
                            let have = db.knapsack_quantity(user_id, name);
                            let total = remaining;
                            result.push_str(&format!(
                                "\n{}. [{}][{}/{}]",
                                i + 1,
                                encoding::smart_decode(name),
                                have.min(*total),
                                total
                            ));
                        }
                        "Monster" => {
                            // 怪物任务：剩余击杀数
                            let original = parse_task_targets(&task.data);
                            let original_count = original
                                .iter()
                                .find(|(n, _)| n == name)
                                .map(|(_, c)| *c)
                                .unwrap_or(*remaining);
                            let killed = original_count - remaining;
                            result.push_str(&format!(
                                "\n{}. [{}][{}/{}]",
                                i + 1,
                                encoding::smart_decode(name),
                                killed,
                                original_count
                            ));
                        }
                        _ => {
                            result.push_str(&format!(
                                "\n{}. [{}][{}]",
                                i + 1,
                                encoding::smart_decode(name),
                                remaining
                            ));
                        }
                    }
                }
            }
        }
    }

    // 显示奖励物品
    if !task.reward_goods.is_empty() {
        result.push_str("\n\n任务奖励物品");
        for (i, item) in task.reward_goods.iter().enumerate() {
            result.push_str(&format!(
                "\n{}. [{}]×{}",
                i + 1,
                encoding::smart_decode(&item.name),
                item.count
            ));
        }
    }

    result.push_str("\n\nTip:发送'提交任务'即可提交任务。");
    result
}

/// 领取任务
pub fn cmd_accept_task(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let task_name = args.trim();
    if task_name.is_empty() {
        return prefix; // 忽略空消息
    }

    // 支持通过序号领取
    let actual_task = if let Ok(idx) = task_name.parse::<usize>() {
        if idx > 0 {
            let available = db.task_available(user_id);
            if let Some(t) = available.get(idx - 1) {
                t.clone()
            } else {
                return format!("{}\n不存在指定任务！", prefix);
            }
        } else {
            return format!("{}\n不存在指定任务！", prefix);
        }
    } else {
        task_name.to_string()
    };

    // 检查任务是否存在
    if !db.task_exists(&actual_task) {
        return format!("{}\n不存在指定任务！", prefix);
    }

    // 检查是否有进行中的任务
    let current_task = db.read_basic(user_id, ITEM_TASK);
    if !current_task.is_empty() {
        return format!("{}\n请先完成您当前的任务！", prefix);
    }

    // 获取任务定义
    let task = match db.task_get(&actual_task) {
        Some(t) => t,
        None => return format!("{}\n不存在指定任务！", prefix),
    };

    // 检查等级限制
    let level: i32 = db.read_basic(user_id, ITEM_LEVEL).parse().unwrap_or(1);
    if task.level_min > 0 && level < task.level_min {
        return format!(
            "{}\n任务[{}]需要您的等级达到{}后才可领取！",
            prefix,
            encoding::smart_decode(&task.title),
            task.level_min
        );
    }
    if task.level_max > 0 && level > task.level_max {
        return format!("{}\n您的等级高于此任务的最高等级限制，无法领取此任务！", prefix);
    }

    // 检查职业限制
    if !task.occupation.is_empty() && task.occupation != "[NULL]" {
        let occupation = db.read_basic(user_id, ITEM_OCCUPATION);
        if task.occupation != occupation {
            return format!(
                "{}\n任务[{}]为{}任务，只有{}可领取！",
                prefix,
                encoding::smart_decode(&task.title),
                encoding::smart_decode(&task.occupation),
                encoding::smart_decode(&task.occupation)
            );
        }
    }

    // 检查前置任务
    if !task.complete_task.is_empty() && task.complete_task != "[NULL]" {
        if let Some(record) = db.task_get_record(user_id, &task.complete_task) {
            if !record.complete {
                return format!("{}\n您目前暂未开放此任务！", prefix);
            }
        } else {
            return format!("{}\n您目前暂未开放此任务！", prefix);
        }
    }

    // 检查重置时间（可重复任务）
    if task.reset_time != -1 {
        if let Some(record) = db.task_get_record(user_id, &actual_task) {
            if record.complete {
                // 检查是否过了重置时间
                if !record.date.is_empty() {
                    if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(&record.date, "%Y-%m-%d %H:%M:%S") {
                        let now = Local::now().naive_local();
                        let duration = now.signed_duration_since(last_time);
                        let hours = duration.num_hours();
                        if hours < task.reset_time as i64 {
                            return format!("{}\n您所指定的任务已完成！", prefix);
                        }
                    }
                }
            } else {
                return format!("{}\n您正在进行任务[{}]！", prefix, encoding::smart_decode(&task.title));
            }
        }
    } else {
        // 一次性任务，检查是否已完成
        if let Some(record) = db.task_get_record(user_id, &actual_task) {
            if record.complete {
                return format!("{}\n您所指定的任务已完成！", prefix);
            }
        }
    }

    // 领取任务
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let task_data = encoding::smart_decode(&task.data);

    if db.task_register(user_id, &actual_task, &now, &task.target_type, &task_data, false) {
        db.write_basic(user_id, ITEM_TASK, &actual_task);
        format!("{}\n成功领取任务[{}]！", prefix, encoding::smart_decode(&task.title))
    } else {
        format!(
            "{}\n任务[{}]领取失败，写入数据失败！",
            prefix,
            encoding::smart_decode(&task.title)
        )
    }
}

/// 提交任务
pub fn cmd_submit_task(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current_task = db.read_basic(user_id, ITEM_TASK);
    if current_task.is_empty() {
        return format!(
            "{}\n您目前还未领取任务。\nTip:发送'领取任务+任务名称'即可领取任务。",
            prefix
        );
    }

    let task = match db.task_get(&current_task) {
        Some(t) => t,
        None => {
            db.write_basic(user_id, ITEM_TASK, EMPTY);
            return format!("{}\n任务不存在。", prefix);
        }
    };

    let record = match db.task_get_record(user_id, &current_task) {
        Some(r) => r,
        None => return format!("{}\n任务记录不存在。", prefix),
    };

    // 检查任务完成条件
    let mut completed = true;
    let mut conditions = String::new();
    let mut condition_count = 0;

    match task.target_type.as_str() {
        "Goods" => {
            // 物品任务：检查背包中是否有足够物品
            let targets = parse_task_targets(&record.data);
            let mut consume_items: Vec<(String, i32, bool)> = Vec::new();

            for (name, need_qty) in &targets {
                let have = db.knapsack_quantity(user_id, name);
                if have < *need_qty {
                    completed = false;
                    condition_count += 1;
                    conditions.push_str(&format!(
                        "\n{}. [{}]×{}",
                        condition_count,
                        encoding::smart_decode(name),
                        need_qty - have
                    ));
                }
                consume_items.push((name.clone(), *need_qty, true));
            }

            if !completed {
                return format!("{}\n您还未完成任务！任务进度{}", prefix, conditions);
            }

            // 消耗物品
            for (name, qty, _) in &consume_items {
                db.knapsack_remove(user_id, name, *qty);
            }
        }
        "Monster" => {
            // 怪物任务：检查是否还有未击杀的目标
            let current_targets = parse_task_targets(&record.data);
            let _original_targets = parse_task_targets(&task.data);

            for (name, remaining) in &current_targets {
                if *remaining > 0 {
                    completed = false;
                    condition_count += 1;
                    conditions.push_str(&format!(
                        "\n{}. [{}]×{}",
                        condition_count,
                        encoding::smart_decode(name),
                        remaining
                    ));
                }
            }

            if !completed {
                return format!("{}\n您还未完成任务！任务进度{}", prefix, conditions);
            }
        }
        _ => {
            return format!("{}\n任务对象异常。", prefix);
        }
    }

    // 标记任务完成
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.task_register(user_id, &current_task, &now, "", "", true);
    db.write_basic(user_id, ITEM_TASK, EMPTY);
    // 成就追踪
    crate::achievement::on_quest_completed(db, user_id);

    let mut result = format!(
        "{}\n恭喜您！您已完成任务{}",
        prefix,
        encoding::smart_decode(&task.title)
    );

    // 发放奖励
    if task.reward_gold > 0 {
        db.modify_currency(user_id, CURRENCY_GOLD, OP_ADD, task.reward_gold);
        result.push_str(&format!("\n获得金币：{}", task.reward_gold));
    }
    if task.reward_diamond > 0 {
        db.modify_currency(user_id, CURRENCY_DIAMOND, OP_ADD, task.reward_diamond);
        result.push_str(&format!("\n获得钻石：{}", task.reward_diamond));
    }
    if task.reward_exp > 0 {
        user::add_experience(db, user_id, task.reward_exp as i32);
        result.push_str(&format!("\n获得经验：{}", task.reward_exp));
    }

    // 奖励物品
    if !task.reward_goods.is_empty() {
        result.push_str("\n获得的物品>>");
        for (i, item) in task.reward_goods.iter().enumerate() {
            if db.knapsack_add(user_id, &item.name, item.count) {
                result.push_str(&format!(
                    "\n{}. [{}]×{}",
                    i + 1,
                    encoding::smart_decode(&item.name),
                    item.count
                ));
            }
        }
    }

    result.push_str("\n\nTip:发送'我的任务+页码'即可查看任务。");
    result
}

/// 放弃任务
pub fn cmd_abandon_task(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);

    if !db.user_exists(user_id) {
        return format!("{}\n您还未注册！请先注册。", prefix);
    }

    let current_task = db.read_basic(user_id, ITEM_TASK);
    if current_task.is_empty() {
        return format!(
            "{}\n您目前还未领取任务。\nTip:发送'我的任务+页码'即可查看任务。",
            prefix
        );
    }

    // 检查任务是否存在
    if !db.task_exists(&current_task) {
        db.write_basic(user_id, ITEM_TASK, EMPTY);
        return format!("{}\n任务不存在。", prefix);
    }

    // 删除任务记录
    if db.task_delete_record(user_id, &current_task) {
        db.write_basic(user_id, ITEM_TASK, EMPTY);
        format!("{}\n您已放弃任务[{}]。", prefix, encoding::smart_decode(&current_task))
    } else {
        format!(
            "{}\n放弃任务[{}]失败，无法删除任务记录。",
            prefix,
            encoding::smart_decode(&current_task)
        )
    }
}

/// 更新任务进度（战斗击杀怪物后调用）
#[allow(dead_code)]
pub fn update_task_monster_kill(db: &Database, user_id: &str, monster_name: &str) -> Option<String> {
    let current_task = db.read_basic(user_id, ITEM_TASK);
    if current_task.is_empty() {
        return None;
    }

    let record = db.task_get_record(user_id, &current_task)?;
    if record.complete {
        return None;
    }

    let task = db.task_get(&current_task)?;
    if task.target_type != "Monster" {
        return None;
    }

    // 解析当前任务数据
    let mut targets = parse_task_targets(&record.data);
    let mut updated = false;

    for (name, remaining) in &mut targets {
        if name == monster_name && *remaining > 0 {
            *remaining -= 1;
            updated = true;
        }
    }

    if updated {
        // 重新构建任务数据
        let new_data: String = targets
            .iter()
            .map(|(name, count)| format!("{}={}", name, count))
            .collect::<Vec<_>>()
            .join("\n");
        let full_data = format!("[Target]\n{}", new_data);

        db.task_register(user_id, &current_task, "", "", &full_data, false);

        // 检查是否全部完成
        let all_done = targets.iter().all(|(_, count)| *count == 0);
        if all_done {
            return Some(format!(
                "任务[{}]的目标已全部完成！请发送'提交任务'提交任务。",
                encoding::smart_decode(&current_task)
            ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_task_targets_monster() {
        let data = "[Target]\r\n史莱姆=5";
        let targets = parse_task_targets(data);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].0, "史莱姆");
        assert_eq!(targets[0].1, 5);
    }

    #[test]
    fn test_parse_task_targets_multiple() {
        let data = "[Target]\n史莱姆=3\n哥布林=2";
        let targets = parse_task_targets(data);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].1, 3);
        assert_eq!(targets[1].1, 2);
    }

    #[test]
    fn test_parse_task_targets_empty() {
        let targets = parse_task_targets("");
        assert!(targets.is_empty());
    }

    #[test]
    fn test_parse_task_targets_goods() {
        let data = "[Goods]\n生命药水=10";
        let targets = parse_task_targets(data);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].0, "生命药水");
        assert_eq!(targets[0].1, 10);
    }

    #[test]
    fn test_parse_reward_items_single() {
        let items = parse_reward_items("强化石*3");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "强化石");
        assert_eq!(items[0].count, 3);
    }

    #[test]
    fn test_parse_reward_items_multiple() {
        let items = parse_reward_items("强化石*3,高级药水*1");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "强化石");
        assert_eq!(items[1].count, 1);
    }

    #[test]
    fn test_parse_reward_items_empty() {
        let items = parse_reward_items("");
        assert!(items.is_empty());
    }

    #[test]
    fn test_parse_reward_items_no_count() {
        let items = parse_reward_items("强化石");
        assert!(items.is_empty());
    }

    #[test]
    fn test_task_type_icon_main() {
        // Verify the function exists and has correct signature
        let _f: fn(&crate::db::Database, &str, &str, &str, &str) -> String = super::cmd_all_tasks;
    }

    #[test]
    fn test_task_target_zero_filtered() {
        let data = "[Target]\n史莱姆=0\n哥布林=5";
        let targets = parse_task_targets(data);
        // Zero-count targets should be filtered out
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].0, "哥布林");
    }
}

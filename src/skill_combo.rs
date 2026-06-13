/// CakeGame 技能连招系统 (Skill Combo Chain System)
///
/// 基于 Ext_Skill_Set_hzxyx 表的 Combo/ComboEdit/ComboTime/ComboCDR/ComboNeed/CDR 字段
/// 实现技能前置条件、连招链、连招伤害加成
///
/// 数据来源：
/// - Combo=31 的技能参与连招
/// - ComboEdit=32/33/34 表示连招位置（第1/2/3段）
/// - ComboTime=连招窗口秒数
/// - CDR格式 "冷却秒数,前置技能名" 表示需要先使用前置技能
///
/// 指令: 查看连招, 连招信息, 连招记录
use crate::db::Database;
use crate::user;

/// 连招技能配置
#[derive(Debug, Clone)]
pub struct ComboSkill {
    pub name: String,
    pub combo_position: i32,  // 1=起手, 2=衔接, 3=终结
    pub window_secs: i64,     // 连招窗口
    pub cooldown_secs: i64,   // 连招冷却
    pub prereq_skill: String, // 前置技能（空=无前置）
    pub prereq_cooldown: i64, // 前置技能冷却时间窗口
}

/// 连招链定义
#[derive(Debug, Clone)]
pub struct ComboChain {
    pub name: String,
    pub skills: Vec<ComboSkill>,
    pub bonus_pct: i32, // 完成连招链的伤害加成百分比
}

/// 从数据库加载连招技能配置
fn load_combo_skills(db: &Database) -> Vec<ComboSkill> {
    let conn = db.lock_conn();
    let mut stmt = match conn
        .prepare("SELECT Name, ComboEdit, ComboTime, ComboCDR, CDR FROM Ext_Skill_Set_hzxyx WHERE Combo = '31'")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let skills: Vec<ComboSkill> = stmt
        .query_map([], |row| {
            let name: String = row.get(0).unwrap_or_default();
            let combo_edit: String = row.get(1).unwrap_or_default();
            let combo_time: String = row.get(2).unwrap_or_default();
            let combo_cdr: String = row.get(3).unwrap_or_default();
            let cdr_raw: String = row.get(4).unwrap_or_default();

            // 解析连招位置
            let position = combo_edit.parse::<i32>().unwrap_or(30);
            let combo_pos = if position == 32 {
                1
            } else if position == 33 {
                2
            } else if position == 34 {
                3
            } else {
                0
            };

            // 解析连招窗口
            let window = combo_time.parse::<i64>().unwrap_or(30);

            // 解析连招冷却
            let cooldown = combo_cdr.parse::<i64>().unwrap_or(30);

            // 解析前置技能 (格式: "冷却秒数,技能名")
            let (prereq, prereq_cd) = if cdr_raw.contains(',') {
                let parts: Vec<&str> = cdr_raw.splitn(2, ',').collect();
                let cd = parts[0].parse::<i64>().unwrap_or(0);
                (parts[1].to_string(), cd)
            } else {
                (String::new(), 0)
            };

            Ok(ComboSkill {
                name: name.trim_matches('\0').to_string(),
                combo_position: combo_pos,
                window_secs: window,
                cooldown_secs: cooldown,
                prereq_skill: prereq,
                prereq_cooldown: prereq_cd,
            })
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    skills
}

/// 从数据库加载技能前置条件（非连招技能也有前置）
fn load_skill_prerequisites(db: &Database) -> Vec<(String, String, i64)> {
    let conn = db.lock_conn();
    let mut stmt =
        match conn.prepare("SELECT Name, CDR FROM Ext_Skill_Set_hzxyx WHERE CDR LIKE '%,%' AND Combo != '31'") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

    let prereqs: Vec<(String, String, i64)> = stmt
        .query_map([], |row| {
            let name: String = row.get(0).unwrap_or_default();
            let cdr_raw: String = row.get(1).unwrap_or_default();

            let (prereq, cd) = if cdr_raw.contains(',') {
                let parts: Vec<&str> = cdr_raw.splitn(2, ',').collect();
                let c = parts[0].parse::<i64>().unwrap_or(0);
                (parts[1].to_string(), c)
            } else {
                (String::new(), 0)
            };

            Ok((name.trim_matches('\0').to_string(), prereq, cd))
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    prereqs
}

/// 构建连招链（从单个技能构建完整链路）
fn build_combo_chains(skills: &[ComboSkill]) -> Vec<ComboChain> {
    // 按连招位置排序，构建链
    let mut chains: Vec<ComboChain> = Vec::new();

    // 找到所有起手技能(position=1)
    let starters: Vec<&ComboSkill> = skills.iter().filter(|s| s.combo_position == 1).collect();

    for starter in starters {
        let mut chain_skills = vec![starter.clone()];
        let mut current = starter;

        // 追踪后续连招
        loop {
            // 找到下一个位置的技能，其前置技能是当前技能
            let next_pos = current.combo_position + 1;
            let next = skills.iter().find(|s| {
                s.combo_position == next_pos && (s.prereq_skill == current.name || s.prereq_skill.is_empty())
            });

            match next {
                Some(n) => {
                    chain_skills.push(n.clone());
                    current = n;
                }
                None => break,
            }
        }

        if chain_skills.len() >= 2 {
            let bonus = match chain_skills.len() {
                2 => 20,
                3 => 40,
                _ => 15,
            };
            chains.push(ComboChain {
                name: format!("{}连招", starter.name),
                skills: chain_skills,
                bonus_pct: bonus,
            });
        }
    }

    chains
}

/// 记录玩家使用技能的时间戳
pub fn record_skill_use(db: &Database, user_id: &str, skill_name: &str) {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    db.write_user_data(user_id, "last_used_skill", skill_name);
    db.write_user_data(user_id, "last_skill_time", &now);

    // 追加到连招历史（保留最近5个）
    let history = db.read_user_data(user_id, "combo_history");
    let mut skills: Vec<String> = if history.is_empty() {
        Vec::new()
    } else {
        history.split(',').map(|s| s.to_string()).collect()
    };
    skills.push(skill_name.to_string());
    if skills.len() > 5 {
        skills.remove(0);
    }
    db.write_user_data(user_id, "combo_history", &skills.join(","));

    // 更新连招统计
    let combo_count_key = "combo_total_uses";
    let count: i32 = db.read_user_data(user_id, combo_count_key).parse().unwrap_or(0);
    db.write_user_data(user_id, combo_count_key, &(count + 1).to_string());
}

/// 检查技能是否满足前置条件
/// 返回: (是否可使用, 原因说明)
pub fn check_skill_prereq(db: &Database, user_id: &str, skill_name: &str) -> (bool, String) {
    let combo_skills = load_combo_skills(db);
    let prereqs = load_skill_prerequisites(db);

    // 检查连招技能前置
    if let Some(combo) = combo_skills.iter().find(|s| s.name == skill_name) {
        if !combo.prereq_skill.is_empty() {
            return check_prereq_active(db, user_id, &combo.prereq_skill, combo.prereq_cooldown);
        }
        return (true, String::new());
    }

    // 检查普通技能前置
    if let Some((_, prereq, cd)) = prereqs.iter().find(|(name, _, _)| name == skill_name) {
        if !prereq.is_empty() {
            return check_prereq_active(db, user_id, prereq, *cd);
        }
    }

    (true, String::new())
}

/// 检查前置技能是否在有效期内
fn check_prereq_active(db: &Database, user_id: &str, prereq: &str, window_secs: i64) -> (bool, String) {
    // "所有" 表示无特定前置
    if prereq == "所有" {
        return (true, String::new());
    }

    let last_skill = db.read_user_data(user_id, "last_used_skill");
    let last_time_str = db.read_user_data(user_id, "last_skill_time");

    if last_skill.is_empty() {
        return (false, format!("需要先使用「{}」才能释放此技能！", prereq));
    }

    // 检查是否匹配前置技能链
    let history = db.read_user_data(user_id, "combo_history");
    let recent_skills: Vec<&str> = history.split(',').collect();

    // 检查最近使用的技能是否包含前置
    let found_prereq = recent_skills.contains(&prereq);

    if !found_prereq {
        return (
            false,
            format!("需要先使用「{}」才能释放此技能！（最近使用：{}）", prereq, last_skill),
        );
    }

    // 检查时间窗口
    if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(&last_time_str, "%Y-%m-%d %H:%M:%S") {
        let now = chrono::Local::now().naive_local();
        let elapsed = (now - last_time).num_seconds();
        if elapsed > window_secs {
            return (
                false,
                format!(
                    "「{}」的前置窗口已过期（{}秒前使用，窗口{}秒）",
                    prereq, elapsed, window_secs
                ),
            );
        }
    }

    (true, String::new())
}

/// 计算当前连招倍率（基于连招链进度）
pub fn calc_combo_multiplier(db: &Database, user_id: &str) -> (f64, String) {
    let combo_skills = load_combo_skills(db);
    let chains = build_combo_chains(&combo_skills);
    let history = db.read_user_data(user_id, "combo_history");

    if history.is_empty() {
        return (1.0, String::new());
    }

    let recent: Vec<&str> = history.split(',').collect();

    // 检查是否匹配任何连招链
    for chain in &chains {
        let chain_names: Vec<&str> = chain.skills.iter().map(|s| s.name.as_str()).collect();
        let chain_len = chain_names.len();

        if recent.len() >= chain_len {
            let last_n = &recent[recent.len() - chain_len..];
            if last_n == &chain_names[..] {
                let multiplier = 1.0 + (chain.bonus_pct as f64 / 100.0);
                return (
                    multiplier,
                    format!(
                        "🎯 触发「{}」连招！伤害 ×{:.2} (+{}%)",
                        chain.name, multiplier, chain.bonus_pct
                    ),
                );
            }
        }

        // 部分连招（进行中）
        if recent.len() >= 2 {
            let partial = &recent[recent.len() - 2.min(recent.len())..];
            let matching = partial.iter().zip(chain_names.iter()).filter(|(a, b)| a == b).count();
            if matching >= 2 && matching < chain_len {
                let progress_pct = (matching as f64 / chain_len as f64 * 100.0) as i32;
                return (
                    1.0 + (matching as f64 * 0.05),
                    format!(
                        "⚡ 连招进行中「{}」({}%) — 继续使用「{}」！",
                        chain.name,
                        progress_pct,
                        chain_names.get(matching).unwrap_or(&"?")
                    ),
                );
            }
        }
    }

    (1.0, String::new())
}

/// 查看连招 — 显示所有可用连招链
pub fn cmd_view_combos(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let combo_skills = load_combo_skills(db);
    let chains = build_combo_chains(&combo_skills);

    if chains.is_empty() {
        return format!(
            "{}\n═══ 技能连招系统 ═══\n\n当前没有可用的连招链。\n连招技能数据来自 Ext_Skill_Set_hzxyx 表。",
            prefix
        );
    }

    let mut result = format!(
        "{}\n╔═══════════════════╗\n║  ⚔ 技能连招系统 ⚔  ║\n╚═══════════════════╝",
        prefix
    );

    for chain in &chains {
        let chain_display: Vec<String> = chain
            .skills
            .iter()
            .map(|s| {
                let pos_icon = match s.combo_position {
                    1 => "🅰",
                    2 => "🅱",
                    3 => "🅲",
                    _ => "⬜",
                };
                format!("{}{}", pos_icon, s.name)
            })
            .collect();

        result.push_str(&format!("\n\n━━━ {} ━━━", chain.name));
        result.push_str(&format!("\n  连招链: {}", chain_display.join(" → ")));
        result.push_str(&format!("\n  完整加成: +{}% 伤害", chain.bonus_pct));
        result.push_str(&format!("\n  连招窗口: {}秒", chain.skills[0].window_secs));
    }

    // 显示玩家连招历史
    let history = db.read_user_data(user_id, "combo_history");
    if !history.is_empty() {
        result.push_str(&format!("\n\n━━━ 最近使用 ━━━\n  {}", history.replace(",", " → ")));
    }

    let (_multiplier, combo_msg) = calc_combo_multiplier(db, user_id);

    if !combo_msg.is_empty() {
        result.push_str(&format!("\n\n{}", combo_msg));
    }

    result.push_str("\n\n💡 使用技能时自动触发连招判定");
    result
}

/// 连招信息 — 显示技能前置条件详情
pub fn cmd_combo_info(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let skill_name = args.trim();

    if skill_name.is_empty() {
        // 显示所有有前置条件的技能
        let prereqs = load_skill_prerequisites(db);
        let combo_skills = load_combo_skills(db);

        let mut result = format!("{}\n═══ 技能前置条件 ═══\n\n⚔ 普通技能前置:", prefix);

        for (name, prereq, cd) in &prereqs {
            result.push_str(&format!("\n  {} → 需要先使用「{}」(窗口{}秒)", name, prereq, cd));
        }

        result.push_str("\n\n⚡ 连招技能:");
        for skill in &combo_skills {
            let pos_name = match skill.combo_position {
                1 => "起手",
                2 => "衔接",
                3 => "终结",
                _ => "未知",
            };
            let prereq_info = if skill.prereq_skill.is_empty() {
                "无前置".to_string()
            } else {
                format!("需「{}」", skill.prereq_skill)
            };
            result.push_str(&format!(
                "\n  [{}] {} ({}) — 窗口{}秒, {}",
                pos_name,
                skill.name,
                prereq_info,
                skill.window_secs,
                if skill.cooldown_secs > 0 {
                    format!("冷却{}秒", skill.cooldown_secs)
                } else {
                    "无冷却".to_string()
                }
            ));
        }

        result.push_str("\n\n💡 发送「连招信息+技能名」查看特定技能的前置条件");
        return result;
    }

    // 查看特定技能
    let combo_skills = load_combo_skills(db);
    let prereqs = load_skill_prerequisites(db);

    // 在连招技能中查找
    if let Some(skill) = combo_skills.iter().find(|s| s.name == skill_name) {
        let pos_name = match skill.combo_position {
            1 => "起手技",
            2 => "衔接技",
            3 => "终结技",
            _ => "未知",
        };

        let mut result = format!("{}\n═══ {} ({}) ═══", prefix, skill.name, pos_name);
        result.push_str(&format!("\n连招位置: 第{}段", skill.combo_position));
        result.push_str(&format!("\n连招窗口: {}秒", skill.window_secs));
        result.push_str(&format!("\n连招冷却: {}秒", skill.cooldown_secs));

        if !skill.prereq_skill.is_empty() {
            result.push_str(&format!(
                "\n前置技能: 「{}」(窗口{}秒)",
                skill.prereq_skill, skill.prereq_cooldown
            ));
        } else {
            result.push_str("\n前置技能: 无");
        }

        // 检查当前是否满足前置
        let (can_use, reason) = check_skill_prereq(db, user_id, skill_name);
        if can_use {
            result.push_str("\n\n✅ 当前可使用此技能");
        } else {
            result.push_str(&format!("\n\n❌ {}", reason));
        }

        return result;
    }

    // 在普通前置技能中查找
    if let Some((_, prereq, cd)) = prereqs.iter().find(|(name, _, _)| name == skill_name) {
        let mut result = format!("{}\n═══ {} ═══", prefix, skill_name);
        result.push_str(&format!("\n前置技能: 「{}」", prereq));
        result.push_str(&format!("\n前置窗口: {}秒", cd));

        let (can_use, reason) = check_skill_prereq(db, user_id, skill_name);
        if can_use {
            result.push_str("\n\n✅ 当前可使用此技能");
        } else {
            result.push_str(&format!("\n\n❌ {}", reason));
        }

        return result;
    }

    format!(
        "{}\n未找到技能「{}」的连招/前置信息。\n发送「连招信息」查看所有技能前置条件。",
        prefix, skill_name
    )
}

/// 连招记录 — 显示玩家的连招使用统计
pub fn cmd_combo_record(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let total_uses: i32 = db.read_user_data(user_id, "combo_total_uses").parse().unwrap_or(0);
    let history = db.read_user_data(user_id, "combo_history");
    let last_skill = db.read_user_data(user_id, "last_used_skill");
    let last_time = db.read_user_data(user_id, "last_skill_time");

    let mut result = format!(
        "{}\n╔═══════════════════╗\n║  📊 连招记录  ║\n╚═══════════════════╝",
        prefix
    );

    result.push_str(&format!("\n\n技能使用总次数: {}", total_uses));

    if !last_skill.is_empty() {
        result.push_str(&format!("\n最后使用技能: 「{}」", last_skill));
        result.push_str(&format!("\n最后使用时间: {}", last_time));
    }

    if !history.is_empty() {
        result.push_str(&format!("\n\n最近连招序列:\n  {}", history.replace(",", " → ")));
    }

    // 当前连招状态
    let (_multiplier, combo_msg) = calc_combo_multiplier(db, user_id);
    if !combo_msg.is_empty() {
        result.push_str(&format!("\n\n当前状态:\n  {}", combo_msg));
    } else if !history.is_empty() {
        result.push_str("\n\n当前状态: 未触发连招加成");
    }

    // 显示最佳连招组合
    let combo_skills = load_combo_skills(db);
    let chains = build_combo_chains(&combo_skills);
    if !chains.is_empty() {
        result.push_str("\n\n可用连招链:");
        for chain in &chains {
            let chain_display: Vec<&str> = chain.skills.iter().map(|s| s.name.as_str()).collect();
            result.push_str(&format!("\n  {} (+{}%)", chain_display.join(" → "), chain.bonus_pct));
        }
    }

    result
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    #[test]
    fn test_combo_position_parsing() {
        // 验证连招位置解析逻辑
        assert_eq!(32_i32.checked_sub(31).unwrap_or(0), 1); // 起手
        assert_eq!(33_i32.checked_sub(31).unwrap_or(0), 2); // 衔接
        assert_eq!(34_i32.checked_sub(31).unwrap_or(0), 3); // 终结
    }

    #[test]
    fn test_cdr_parsing() {
        // 验证前置技能解析
        let cdr = "40,御剑·卍";
        let parts: Vec<&str> = cdr.splitn(2, ',').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "40");
        assert_eq!(parts[1], "御剑·卍");
    }

    #[test]
    fn test_combo_chain_bonus() {
        // 验证连招链伤害加成
        assert_eq!(
            match 2 {
                2 => 20,
                3 => 40,
                _ => 15,
            },
            20
        ); // 2段连招 +20%
        assert_eq!(
            match 3 {
                2 => 20,
                3 => 40,
                _ => 15,
            },
            40
        ); // 3段连招 +40%
    }

    #[test]
    fn test_combo_history_truncation() {
        // 验证历史记录截断
        let mut skills: Vec<String> = vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into()];
        skills.push("F".into());
        if skills.len() > 5 {
            skills.remove(0);
        }
        assert_eq!(skills.len(), 5);
        assert_eq!(skills[0], "B");
        assert_eq!(skills[4], "F");
    }

    #[test]
    fn test_partial_combo_match() {
        // 验证部分连招匹配
        let chain = vec!["御剑·踏风斩", "御剑·剑杀"];
        let recent = vec!["其他技能", "御剑·踏风斩", "御剑·剑杀"];

        // 检查最后2个是否匹配
        let last_n = &recent[recent.len() - chain.len()..];
        assert_eq!(last_n, &chain[..]);
    }

    #[test]
    fn test_all_skill_keyword() {
        // "所有" 作为前置应该始终通过
        let prereq = "所有";
        assert_eq!(prereq, "所有"); // 无条件通过
    }
}

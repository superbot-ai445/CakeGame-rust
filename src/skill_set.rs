use crate::db::Database;
use crate::user;

/// Extended skill set entry from Ext_Skill_Set_hzxyx
pub struct ExtSkillSet {
    pub name: String,
    pub ad: i32,
    pub ap: i32,
    pub def: i32,
    pub mdf: i32,
    pub hit: i32,
    pub gl: i32,
    pub wz: String,
    pub combo: bool,
    pub combo_edit: i32,
    pub combo_time: i32,
    #[allow(dead_code)]
    pub combo_cdr: bool,
    #[allow(dead_code)]
    pub combo_need: String,
    pub cdr_target: String,
    pub cx: i32,
    pub zyx: bool,
    pub hp: i32,
    pub mp: i32,
}

fn parse_val(s: &str) -> i32 {
    s.trim().parse().unwrap_or(30)
}

fn load_ext_skill_set(db: &Database, name: &str) -> Option<ExtSkillSet> {
    let conn = db.lock_conn();
    let mut stmt = conn
        .prepare("SELECT Name, HP, MP, AD, AP, DEF, MDF, HIT, WZ, GL, Combo, ComboEdit, ComboTime, ComboCDR, ComboNeed, CXSJ, ZYX FROM Ext_Skill_Set_hzxyx WHERE Name = ?1")
        .ok()?;
    let row = stmt
        .query_row(rusqlite::params![name], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, String>(11)?,
                r.get::<_, String>(12)?,
                r.get::<_, String>(13)?,
                r.get::<_, String>(14)?,
                r.get::<_, String>(15)?,
                r.get::<_, String>(16)?,
            ))
        })
        .ok()?;

    let cdr_raw = &row.13;
    let cdr_target = if cdr_raw == "30" {
        String::new()
    } else {
        // Format: "value,skill_name" e.g. "100,圣光复仇"
        if let Some(pos) = cdr_raw.find(',') {
            cdr_raw[pos + 1..].trim().to_string()
        } else {
            String::new()
        }
    };

    Some(ExtSkillSet {
        name: row.0,
        hp: parse_val(&row.1),
        mp: parse_val(&row.2),
        ad: parse_val(&row.3),
        ap: parse_val(&row.4),
        def: parse_val(&row.5),
        mdf: parse_val(&row.6),
        hit: parse_val(&row.7),
        wz: row.8.clone(),
        gl: parse_val(&row.9),
        combo: row.10 == "31",
        combo_edit: parse_val(&row.11),
        combo_time: parse_val(&row.12),
        combo_cdr: row.13 != "30",
        combo_need: row.14.clone(),
        cdr_target,
        cx: parse_val(&row.15),
        zyx: row.16 == "31",
    })
}

fn format_ext_skill(ext: &ExtSkillSet) -> String {
    let mut result = format!("【{}】", ext.name);

    // Location requirement
    if ext.wz != "30" && !ext.wz.is_empty() {
        result.push_str(&format!(" 📍{}", ext.wz));
    }

    // Success rate
    if ext.gl != 100 && ext.gl != 30 {
        result.push_str(&format!(" 呑中{}%", ext.gl));
    }

    result.push('\n');

    // Stat modifiers (only non-default)
    let mut mods = Vec::new();
    if ext.ad != 30 {
        mods.push(if ext.ad > 30 {
            format!("⚔️攻击+{}", ext.ad - 30)
        } else {
            format!("⚔️攻击{}", ext.ad - 30)
        });
    }
    if ext.ap != 30 {
        mods.push(if ext.ap > 30 {
            format!("🔮魔攻+{}", ext.ap - 30)
        } else {
            format!("🔮魔攻{}", ext.ap - 30)
        });
    }
    if ext.def != 30 {
        mods.push(if ext.def > 30 {
            format!("🛡️防御+{}", ext.def - 30)
        } else {
            format!("🛡️防御{}", ext.def - 30)
        });
    }
    if ext.mdf != 30 {
        mods.push(if ext.mdf > 30 {
            format!("🧿魔抗+{}", ext.mdf - 30)
        } else {
            format!("🧿魔抗{}", ext.mdf - 30)
        });
    }
    if ext.hit != 30 {
        mods.push(if ext.hit > 30 {
            format!("🎯命中+{}", ext.hit - 30)
        } else {
            format!("🎯命中{}", ext.hit - 30)
        });
    }
    if ext.hp != 30 {
        mods.push(format!("❤️HP调整{}", ext.hp - 30));
    }
    if ext.mp != 30 {
        mods.push(format!("💙MP调整{}", ext.mp - 30));
    }

    if !mods.is_empty() {
        result.push_str(&format!("  属性: {}\n", mods.join(" ")));
    }

    // Combo system
    if ext.combo {
        let combo_type = match ext.combo_edit {
            32 => "连击A",
            33 => "连击B",
            34 => "连击C",
            _ => "连击",
        };
        result.push_str(&format!("  🔗{}可触发 (窗口{}秒)\n", combo_type, ext.combo_time));
    }

    // Cooldown reduction
    if !ext.cdr_target.is_empty() {
        result.push_str(&format!("  ⏱️使用后重置[{}]\n", ext.cdr_target));
    }

    // Cooldown
    if ext.cx != 30 && ext.cx > 0 {
        result.push_str(&format!("  ⏳冷却{}回合\n", ext.cx));
    }

    // Special flag
    if ext.zyx {
        result.push_str("  ⚡特殊技能\n");
    }

    result
}

/// 查看扩展技能 - Show extended skill details for a learned skill
pub fn cmd_view_ext_skill(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let skill_name = args.trim();

    if skill_name.is_empty() {
        // Show user's learned skills with ext info
        let skills = db.skill_all(user_id);
        if skills.is_empty() {
            return format!(
                "{}\n您还没有学会任何技能！使用「查看技能详情+技能名」查看扩展信息。",
                prefix
            );
        }

        let mut result = format!("{}\n═══ 扩展技能概览 ═══", prefix);
        let mut has_ext = false;
        for (name, prof) in &skills {
            if let Some(ext) = load_ext_skill_set(db, name) {
                has_ext = true;
                result.push_str(&format!("\n  [{}] 熟练度{}", name, prof));
                let mut tags = Vec::new();
                if ext.combo {
                    tags.push("🔗连击");
                }
                if !ext.cdr_target.is_empty() {
                    tags.push("⏱️CD重置");
                }
                if ext.ad != 30 || ext.ap != 30 || ext.def != 30 || ext.mdf != 30 {
                    tags.push("📊属性变化");
                }
                if ext.zyx {
                    tags.push("⚡特殊");
                }
                if !tags.is_empty() {
                    result.push_str(&format!(" {}", tags.join(" ")));
                }
            }
        }
        if !has_ext {
            result.push_str("\n暂无扩展技能数据。");
        }
        result.push_str("\n\n使用「查看技能详情+技能名」查看具体信息。");
        return result;
    }

    // Show specific skill detail
    match load_ext_skill_set(db, skill_name) {
        Some(ext) => {
            let mut result = format!("{}\n═══ 技能详情 ═══\n", prefix);
            result.push_str(&format_ext_skill(&ext));

            // Check if user has this skill
            let skills = db.skill_all(user_id);
            let user_prof = skills.iter().find(|(n, _)| n == skill_name).map(|(_, p)| *p);
            if let Some(prof) = user_prof {
                result.push_str(&format!("\n  📖 您的熟练度: {}", prof));
            } else {
                result.push_str("\n  ❌ 您尚未学会此技能");
            }
            result
        }
        None => {
            // Try Config_Skills
            if let Some(skill_info) = db.skill_get(skill_name) {
                format!(
                    "{}\n【{}】\n类型: {}\n消耗: {}MP\n{}\n\n(此技能无扩展数据)",
                    prefix, skill_name, skill_info.skill_type, skill_info.consume, skill_info.introduce
                )
            } else {
                format!("{}\n未找到技能 [{}]", prefix, skill_name)
            }
        }
    }
}

/// 查看所有扩展技能列表
/// Helper: load all ext skill set rows
fn load_all_ext_skills(db: &Database) -> Vec<(String, String, String, String, String, String, String)> {
    let conn = db.lock_conn();
    let mut stmt =
        match conn.prepare("SELECT Name, Combo, AD, AP, DEF, MDF, ZYX FROM Ext_Skill_Set_hzxyx ORDER BY Name") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
    let x = match stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
        ))
    }) {
        Ok(r) => r.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    };
    x
}

/// 查看所有扩展技能列表
pub fn cmd_list_ext_skills(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let prefix = user::get_msg_prefix(db, user_id);
    let rows = load_all_ext_skills(db);
    if rows.is_empty() {
        return format!("{}\n无法读取扩展技能数据", prefix);
    }

    let user_skills = db.skill_all(user_id);
    let user_skill_names: std::collections::HashSet<String> = user_skills.iter().map(|(n, _)| n.clone()).collect();

    let mut result = format!("{}\n═══ 扩展技能表 ═══ ({}个)", prefix, rows.len());

    let mut combo_skills = Vec::new();
    let mut normal_skills = Vec::new();

    for (name, combo, ad, ap, def, mdf, zyx) in &rows {
        let has_modifier = ad != "30" || ap != "30" || def != "30" || mdf != "30";
        let is_combo = combo == "31";
        let is_special = zyx == "31";
        let learned = user_skill_names.contains(name);

        let entry = (name, has_modifier, is_special, learned);

        if is_combo {
            combo_skills.push(entry);
        } else {
            normal_skills.push(entry);
        }
    }

    if !combo_skills.is_empty() {
        result.push_str("\n\n🔗 连击技能:");
        for (name, has_mod, special, learned) in &combo_skills {
            let mark = if *learned { "✅" } else { "⬜" };
            let mut tags = Vec::new();
            if *has_mod {
                tags.push("📊");
            }
            if *special {
                tags.push("⚡");
            }
            result.push_str(&format!("\n  {} {}{}", mark, name, tags.join("")));
        }
    }

    if !normal_skills.is_empty() {
        result.push_str(&format!("\n\n⚔️ 普通技能 ({}个):", normal_skills.len()));
        for (name, has_mod, special, learned) in &normal_skills {
            let mark = if *learned { "✅" } else { "⬜" };
            let mut tags = Vec::new();
            if *has_mod {
                tags.push("📊");
            }
            if *special {
                tags.push("⚡");
            }
            result.push_str(&format!("\n  {} {}{}", mark, name, tags.join("")));
        }
    }

    result.push_str("\n\n使用「查看技能详情+技能名」查看具体信息");
    result.push_str("\n✅=已学会 ⬜=未学 📊=有属性变化 ⚡=特殊技能");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_val_number() {
        assert_eq!(parse_val("42"), 42);
        assert_eq!(parse_val("0"), 0);
        assert_eq!(parse_val("-5"), -5);
    }

    #[test]
    fn test_parse_val_default() {
        assert_eq!(parse_val(""), 30);
        assert_eq!(parse_val("abc"), 30);
        assert_eq!(parse_val(" "), 30);
    }

    #[test]
    fn test_parse_val_whitespace() {
        assert_eq!(parse_val(" 100 "), 100);
        assert_eq!(parse_val("\t50"), 50);
    }

    #[test]
    fn test_parse_val_boundary() {
        assert_eq!(parse_val("999999"), 999999);
        assert_eq!(parse_val("-999999"), -999999);
    }
}

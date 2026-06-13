/// CakeGame 系统变量管理系统
///
/// 激活 Ext_CustomVariable_Info 表(16条自定义变量模板)，
/// 提供 GM 可配置的动态游戏参数，支持运行时修改游戏行为。
///
/// 功能:
/// - 查看变量: 列出所有自定义变量及其当前值
/// - 设置变量: GM 修改自定义变量值
/// - 变量详情: 查看指定变量的详细信息
///
/// 数据来源: Ext_CustomVariable_Info 表 + Global 表 SECTION='custom_variables'
const PERMISSION_LEVEL_ADMIN: i32 = 100;

use crate::db::Database;
use crate::permissions;
use crate::user;

/// 变量类型定义
const VARIABLE_DEFS: &[VarDef] = &[
    VarDef {
        slot: 1,
        name: "经验倍率",
        var_type: "战斗",
        default_value: "1.0",
        description: "全服经验获取倍率 (0.1~10.0)",
        unit: "倍",
    },
    VarDef {
        slot: 2,
        name: "金币倍率",
        var_type: "经济",
        default_value: "1.0",
        description: "全服金币获取倍率 (0.1~10.0)",
        unit: "倍",
    },
    VarDef {
        slot: 3,
        name: "掉落倍率",
        var_type: "战斗",
        default_value: "1.0",
        description: "全服物品掉落倍率 (0.1~10.0)",
        unit: "倍",
    },
    VarDef {
        slot: 4,
        name: "强化成功率加成",
        var_type: "装备",
        default_value: "0",
        description: "全服强化成功率额外加成百分比 (-50~50)",
        unit: "%",
    },
    VarDef {
        slot: 5,
        name: "采集冷却减免",
        var_type: "生活",
        default_value: "0",
        description: "采集冷却时间减免百分比 (0~100)",
        unit: "%",
    },
    VarDef {
        slot: 6,
        name: "竞技积分加成",
        var_type: "PVP",
        default_value: "0",
        description: "竞技场获得积分额外加成百分比 (0~200)",
        unit: "%",
    },
    VarDef {
        slot: 7,
        name: "公会贡献倍率",
        var_type: "社交",
        default_value: "1.0",
        description: "公会贡献获取倍率 (0.5~5.0)",
        unit: "倍",
    },
    VarDef {
        slot: 8,
        name: "修炼速度倍率",
        var_type: "成长",
        default_value: "1.0",
        description: "自动修炼经验获取倍率 (0.5~5.0)",
        unit: "倍",
    },
    VarDef {
        slot: 9,
        name: "签到奖励倍率",
        var_type: "日常",
        default_value: "1.0",
        description: "每日签到奖励倍率 (1.0~5.0)",
        unit: "倍",
    },
    VarDef {
        slot: 10,
        name: "最大在线人数",
        var_type: "服务器",
        default_value: "500",
        description: "服务器最大同时在线人数限制",
        unit: "人",
    },
    VarDef {
        slot: 11,
        name: "新手保护等级",
        var_type: "PVP",
        default_value: "10",
        description: "新手玩家低于此等级不受PVP攻击",
        unit: "级",
    },
    VarDef {
        slot: 12,
        name: "世界BOSS血量倍率",
        var_type: "战斗",
        default_value: "1.0",
        description: "世界BOSS血量倍率 (0.5~5.0)",
        unit: "倍",
    },
    VarDef {
        slot: 13,
        name: "副本扫荡折扣",
        var_type: "副本",
        default_value: "80",
        description: "副本扫荡奖励百分比 (50~100)",
        unit: "%",
    },
    VarDef {
        slot: 14,
        name: "交易税",
        var_type: "经济",
        default_value: "5",
        description: "玩家交易手续费百分比 (0~20)",
        unit: "%",
    },
    VarDef {
        slot: 15,
        name: "暴击伤害加成",
        var_type: "战斗",
        default_value: "0",
        description: "全服暴击伤害额外加成百分比 (0~100)",
        unit: "%",
    },
    VarDef {
        slot: 16,
        name: "离线收益倍率",
        var_type: "成长",
        default_value: "1.0",
        description: "离线挂机收益倍率 (0.5~3.0)",
        unit: "倍",
    },
];

/// 变量定义
struct VarDef {
    slot: usize,
    name: &'static str,
    var_type: &'static str,
    default_value: &'static str,
    description: &'static str,
    unit: &'static str,
}

/// 变量类型 emoji
fn var_type_emoji(var_type: &str) -> &'static str {
    match var_type {
        "战斗" => "⚔️",
        "经济" => "💰",
        "装备" => "🛡️",
        "生活" => "🌿",
        "PVP" => "🏟️",
        "社交" => "🤝",
        "成长" => "📈",
        "日常" => "📅",
        "服务器" => "🖥️",
        "副本" => "🗝️",
        _ => "⚙️",
    }
}

/// 获取变量当前值 (从 Global 表读取，无则返回默认值)
pub fn get_variable_value(db: &Database, slot: usize) -> String {
    let def = match VARIABLE_DEFS.iter().find(|d| d.slot == slot) {
        Some(d) => d,
        None => return String::new(),
    };
    let section = "custom_variables";
    let key = format!("slot_{}", slot);
    let stored = db.global_get(section, &key);
    if stored.is_empty() {
        def.default_value.to_string()
    } else {
        stored
    }
}

/// 获取变量浮点值
#[allow(dead_code)]
pub fn get_variable_f64(db: &Database, slot: usize) -> f64 {
    get_variable_value(db, slot).parse::<f64>().unwrap_or(1.0)
}

/// 获取变量整数值
#[allow(dead_code)]
pub fn get_variable_i32(db: &Database, slot: usize) -> i32 {
    get_variable_value(db, slot).parse::<i32>().unwrap_or(0)
}

/// 按名称查找变量定义
fn find_var_by_name(name: &str) -> Option<&'static VarDef> {
    if let Some(def) = VARIABLE_DEFS.iter().find(|d| d.name == name) {
        return Some(def);
    }
    VARIABLE_DEFS
        .iter()
        .find(|d| d.name.contains(name) || name.contains(d.name))
}

/// 查看变量 - 列出所有自定义变量
pub fn cmd_view_variables(db: &Database, user_id: &str, _args: &str, _msg_type: &str, _group: &str) -> String {
    let _ = user::get_msg_prefix(db, user_id);
    let mut out = String::new();

    out.push_str("╔══════════════════════════════╗\n");
    out.push_str("║    ⚙️ 系统变量管理面板        ║\n");
    out.push_str("╚══════════════════════════════╝\n\n");

    let mut last_type = "";
    for def in VARIABLE_DEFS {
        if def.var_type != last_type {
            let emoji = var_type_emoji(def.var_type);
            out.push_str(&format!("━━ {} {} ━━\n", emoji, def.var_type));
            last_type = def.var_type;
        }
        let value = get_variable_value(db, def.slot);
        let is_default = value == def.default_value;
        let marker = if is_default { "⬜" } else { "🟡" };
        out.push_str(&format!(
            "  {} [{}] {} = {}{}\n",
            marker, def.slot, def.name, value, def.unit
        ));
    }

    out.push_str(&format!(
        "\n💡 共 {} 个变量 | 🟡=已修改 ⬜=默认值\n",
        VARIABLE_DEFS.len()
    ));
    out.push_str("📖 用法: 变量详情+变量名 | 设置变量+变量名+值\n");
    out
}

/// 变量详情 - 查看指定变量的详细信息
pub fn cmd_variable_detail(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let _ = user::get_msg_prefix(db, user_id);
    let name = args.trim();
    if name.is_empty() {
        return "📖 请指定变量名\n\n用法: 变量详情+经验倍率\n\n可用变量:\n".to_string()
            + &VARIABLE_DEFS
                .iter()
                .map(|d| format!("  • {}", d.name))
                .collect::<Vec<_>>()
                .join("\n");
    }

    let def = match find_var_by_name(name) {
        Some(d) => d,
        None => {
            let similar: Vec<&str> = VARIABLE_DEFS
                .iter()
                .filter(|d| d.name.contains(name) || name.contains(d.name) || d.var_type.contains(name))
                .map(|d| d.name)
                .collect();
            if similar.is_empty() {
                return format!("❌ 未找到变量 \"{}\"\n\n用法: 变量详情+变量名", name);
            }
            return format!(
                "❌ 未精确匹配 \"{}\"，你是否要查看:\n{}",
                name,
                similar
                    .iter()
                    .map(|s| format!("  • {}", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
    };

    let value = get_variable_value(db, def.slot);
    let is_default = value == def.default_value;
    let emoji = var_type_emoji(def.var_type);
    let admin_note = if permissions::get_permission(db, user_id) >= PERMISSION_LEVEL_ADMIN {
        "\n🔧 GM操作: 设置变量+变量名+新值"
    } else {
        ""
    };

    format!(
        "╔══════════════════════════════╗\n\
         ║  ⚙️ 变量详情                  ║\n\
         ╚══════════════════════════════╝\n\n\
         📌 名称: {} {}\n\
         🏷️ 类型: {} {}\n\
         📍 槽位: #{}\n\
         📝 说明: {}\n\n\
         📊 当前值: {}{}\n\
         📊 默认值: {}{}\n\
         📊 状态: {}\n{}",
        def.name,
        emoji,
        emoji,
        def.var_type,
        def.slot,
        def.description,
        value,
        def.unit,
        def.default_value,
        def.unit,
        if is_default {
            "⬜ 使用默认值"
        } else {
            "🟡 已自定义修改"
        },
        admin_note,
    )
}

/// 设置变量 - GM 修改自定义变量值
pub fn cmd_set_variable(db: &Database, user_id: &str, args: &str, _msg_type: &str, _group: &str) -> String {
    let _ = user::get_msg_prefix(db, user_id);

    if permissions::get_permission(db, user_id) < PERMISSION_LEVEL_ADMIN {
        return "❌ 权限不足: 仅管理员可修改系统变量".to_string();
    }

    let parts: Vec<&str> = args.split('+').map(|s| s.trim()).collect();
    if parts.len() < 2 {
        return "📖 用法: 设置变量+变量名+新值\n\n示例: 设置变量+经验倍率+2.0".to_string();
    }

    let name = parts[0];
    let new_value = parts[1];

    let def = match find_var_by_name(name) {
        Some(d) => d,
        None => return format!("❌ 未找到变量 \"{}\"", name),
    };

    if let Err(e) = validate_variable(def, new_value) {
        return format!("❌ 值验证失败: {}", e);
    }

    let section = "custom_variables";
    let key = format!("slot_{}", def.slot);
    let old_value = get_variable_value(db, def.slot);
    db.global_set(section, &key, new_value);

    // 记录修改日志
    let log_section = "custom_variable_log";
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_count: i32 = db.global_get(log_section, "count").parse().unwrap_or(0);
    let log_key = format!("log_{}", log_count + 1);
    db.global_set(
        log_section,
        &log_key,
        &format!(
            "{}|{}|{}|{}→{}|{}",
            timestamp, user_id, def.name, old_value, new_value, def.unit
        ),
    );
    db.global_set(log_section, "count", &(log_count + 1).to_string());

    let emoji = var_type_emoji(def.var_type);
    format!(
        "✅ 系统变量已更新\n\n\
         {} {} [{}]\n\
         📊 旧值: {}{}\n\
         📊 新值: {}{}\n\
         🕐 时间: {}\n\n\
         ⚠️ 变更将影响全服所有玩家",
        emoji, def.name, def.slot, old_value, def.unit, new_value, def.unit, timestamp
    )
}

/// 变量值验证
fn validate_variable(def: &VarDef, value: &str) -> Result<(), String> {
    let num: f64 = value.parse().map_err(|_| format!("\"{}\" 不是有效数字", value))?;

    let (min, max) = match def.slot {
        1..=3 => (0.1, 10.0), // 经验/金币/掉落倍率
        4 => (-50.0, 50.0),   // 强化成功率加成
        5 => (0.0, 100.0),    // 采集冷却减免
        6 => (0.0, 200.0),    // 竞技积分加成
        7 => (0.5, 5.0),      // 公会贡献倍率
        8 => (0.5, 5.0),      // 修炼速度倍率
        9 => (1.0, 5.0),      // 签到奖励倍率
        10 => (1.0, 10000.0), // 最大在线人数
        11 => (1.0, 100.0),   // 新手保护等级
        12 => (0.5, 5.0),     // 世界BOSS血量倍率
        13 => (50.0, 100.0),  // 副本扫荡折扣
        14 => (0.0, 20.0),    // 交易税
        15 => (0.0, 100.0),   // 暴击伤害加成
        16 => (0.5, 3.0),     // 离线收益倍率
        _ => return Err("未知变量槽位".to_string()),
    };

    if num < min || num > max {
        return Err(format!("值 {} 超出范围 [{}, {}]", value, min, max));
    }

    if def.slot == 10 && num.fract() != 0.0 {
        return Err("最大在线人数必须是整数".to_string());
    }

    Ok(())
}

/// 获取变量修改日志 (最近 N 条)
#[allow(dead_code)]
pub fn get_variable_log(db: &Database, limit: usize) -> Vec<String> {
    let log_section = "custom_variable_log";
    let count: i32 = db.global_get(log_section, "count").parse().unwrap_or(0);

    let mut logs = Vec::new();
    let start = std::cmp::max(1, count - limit as i32 + 1);
    for i in start..=count {
        let key = format!("log_{}", i);
        let entry = db.global_get(log_section, &key);
        if !entry.is_empty() {
            logs.push(entry);
        }
    }
    logs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_defs_count() {
        assert_eq!(VARIABLE_DEFS.len(), 16);
    }

    #[test]
    fn test_variable_slots_unique() {
        let mut slots: Vec<usize> = VARIABLE_DEFS.iter().map(|d| d.slot).collect();
        slots.sort();
        slots.dedup();
        assert_eq!(slots.len(), VARIABLE_DEFS.len());
    }

    #[test]
    fn test_variable_names_unique() {
        let mut names: Vec<&str> = VARIABLE_DEFS.iter().map(|d| d.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), VARIABLE_DEFS.len());
    }

    #[test]
    fn test_var_type_emoji_coverage() {
        let types: Vec<&str> = VARIABLE_DEFS.iter().map(|d| d.var_type).collect();
        for t in types {
            let emoji = var_type_emoji(t);
            assert_ne!(emoji, "⚙️", "Type {} should have specific emoji", t);
        }
    }

    #[test]
    fn test_find_var_by_name_exact() {
        let def = find_var_by_name("经验倍率").unwrap();
        assert_eq!(def.slot, 1);
    }

    #[test]
    fn test_find_var_by_name_fuzzy() {
        let def = find_var_by_name("经验").unwrap();
        assert_eq!(def.slot, 1);
    }

    #[test]
    fn test_find_var_by_name_not_found() {
        assert!(find_var_by_name("不存在的变量").is_none());
    }

    #[test]
    fn test_find_var_by_name_type_search() {
        let defs: Vec<&VarDef> = VARIABLE_DEFS.iter().filter(|d| d.var_type.contains("PVP")).collect();
        assert_eq!(defs.len(), 2);
    }

    #[test]
    fn test_validate_variable_valid() {
        let def = &VARIABLE_DEFS[0]; // 经验倍率
        assert!(validate_variable(def, "2.0").is_ok());
        assert!(validate_variable(def, "0.5").is_ok());
        assert!(validate_variable(def, "10.0").is_ok());
    }

    #[test]
    fn test_validate_variable_out_of_range() {
        let def = &VARIABLE_DEFS[0]; // 经验倍率 0.1~10.0
        assert!(validate_variable(def, "0.05").is_err());
        assert!(validate_variable(def, "15.0").is_err());
    }

    #[test]
    fn test_validate_variable_not_number() {
        let def = &VARIABLE_DEFS[0];
        assert!(validate_variable(def, "abc").is_err());
    }

    #[test]
    fn test_validate_variable_online_limit_integer() {
        let def = &VARIABLE_DEFS[9]; // 最大在线人数
        assert!(validate_variable(def, "500").is_ok());
        assert!(validate_variable(def, "500.5").is_err());
    }

    #[test]
    fn test_validate_variable_negative_enhance() {
        let def = &VARIABLE_DEFS[3]; // 强化成功率加成 -50~50
        assert!(validate_variable(def, "-30").is_ok());
        assert!(validate_variable(def, "30").is_ok());
    }

    #[test]
    fn test_default_values_valid() {
        for def in VARIABLE_DEFS {
            assert!(
                validate_variable(def, def.default_value).is_ok(),
                "Default value '{}' for '{}' should be valid",
                def.default_value,
                def.name
            );
        }
    }
}
